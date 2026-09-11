use super::*;

pub(super) fn run_session(
    controller: Arc<PairingController>,
    session_id: PairingSessionId,
    mode: PairingMode,
    commands: mpsc::Receiver<SessionCommand>,
) {
    if let Err(failure) = perform_session(&controller, &session_id, mode, commands) {
        controller.fail(&session_id, failure);
    }
}

fn perform_session(
    controller: &PairingController,
    session_id: &PairingSessionId,
    mode: PairingMode,
    commands: mpsc::Receiver<SessionCommand>,
) -> Result<(), PairingFailure> {
    let session_deadline = Instant::now() + SESSION_TIMEOUT;
    let role = match &mode {
        PairingMode::Initiate(_) => PairingRole::Initiator,
        PairingMode::Receive => PairingRole::Receiver,
    };
    let local = controller.local_identity()?;
    let cryptographic_identity = controller
        .security
        .identity(local.id())
        .map_err(|_| PairingFailure::SecurityUnavailable)?;
    let mut channel = match mode {
        PairingMode::Initiate(endpoint) => controller
            .transport
            .connect(&endpoint, &cryptographic_identity)
            .map_err(map_transport_failure)?,
        PairingMode::Receive => loop {
            if Instant::now() >= session_deadline {
                controller.update_state(session_id, PairingState::Expired);
                return Ok(());
            }
            if matches!(
                commands.try_recv(),
                Ok(SessionCommand::Cancel | SessionCommand::Reject)
            ) {
                controller.update_state(session_id, PairingState::Cancelled);
                return Ok(());
            }
            match controller
                .transport
                .accept(&cryptographic_identity, ACCEPT_POLL_INTERVAL)
                .map_err(map_transport_failure)?
            {
                Some(channel) => break channel,
                None => continue,
            }
        },
    };

    controller.update_state(session_id, PairingState::ExchangingIdentity);
    let mut local_nonce = [0_u8; 32];
    getrandom::fill(&mut local_nonce).map_err(|_| PairingFailure::Internal)?;
    let connection_port = controller
        .connection
        .listening_port()
        .map_err(map_transport_failure)?;
    let local_hello = PairingHello::new(
        local.id().clone(),
        local.display_name().to_owned(),
        local.platform(),
        ProtocolVersion::CURRENT,
        connection_port,
        local_nonce,
    )
    .map_err(|_| PairingFailure::Internal)?;
    channel
        .send(&PairingMessage::Hello(local_hello.clone()))
        .map_err(map_transport_failure)?;
    let peer_hello = match channel.receive().map_err(map_transport_failure)? {
        PairingMessage::Hello(hello) => hello,
        _ => return Err(PairingFailure::InvalidPeer),
    };
    if peer_hello.protocol_version().major() != ProtocolVersion::CURRENT.major()
        || peer_hello.device_id() == local.id()
    {
        return Err(PairingFailure::ProtocolMismatch);
    }
    let peer_endpoint = channel
        .peer_endpoint(peer_hello.connection_port())
        .map_err(map_transport_failure)?;
    let peer = VerifiedPeer {
        hello: peer_hello,
        fingerprint: channel.peer_fingerprint().map_err(map_transport_failure)?,
        connection_endpoint: peer_endpoint,
    };
    let context = authentication_context(
        &local_hello,
        cryptographic_identity.fingerprint(),
        &peer.hello,
        peer.fingerprint,
    );
    let verification = channel
        .export_authentication(&context)
        .map_err(map_transport_failure)?;
    controller.update_peer(session_id, &peer, verification);
    channel
        .set_timeout(APPROVAL_POLL_INTERVAL)
        .map_err(map_transport_failure)?;

    let deadline = Instant::now() + APPROVAL_TIMEOUT;
    let mut local_approved = false;
    let mut peer_approved = false;
    while !local_approved || !peer_approved {
        if Instant::now() >= deadline {
            controller.update_state(session_id, PairingState::Expired);
            return Ok(());
        }
        match commands.recv_timeout(APPROVAL_POLL_INTERVAL) {
            Ok(SessionCommand::Approve) => {
                channel
                    .send(&PairingMessage::Approved)
                    .map_err(map_transport_failure)?;
                local_approved = true;
            }
            Ok(SessionCommand::Reject) => {
                let _send_result = channel.send(&PairingMessage::Rejected);
                controller.update_state(session_id, PairingState::Rejected);
                return Ok(());
            }
            Ok(SessionCommand::Cancel) => {
                let _send_result = channel.send(&PairingMessage::Rejected);
                controller.update_state(session_id, PairingState::Cancelled);
                return Ok(());
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                controller.update_state(session_id, PairingState::Cancelled);
                return Ok(());
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        if local_approved && !peer_approved {
            match channel.receive() {
                Ok(PairingMessage::Approved) => peer_approved = true,
                Ok(PairingMessage::Rejected) => {
                    controller.update_state(session_id, PairingState::Rejected);
                    return Ok(());
                }
                Ok(_) => return Err(PairingFailure::InvalidPeer),
                Err(TransportError::TimedOut) => {}
                Err(error) => return Err(map_transport_failure(error)),
            }
        }
    }

    if !controller.has_session(session_id) {
        let _send_result = channel.send(&PairingMessage::Rejected);
        return Ok(());
    }

    controller.update_state(session_id, PairingState::PersistingTrust);
    channel
        .set_timeout(Duration::from_secs(15))
        .map_err(map_transport_failure)?;
    match role {
        PairingRole::Initiator => {
            let persisted = persist_peer(controller, &peer)?;
            if let Err(error) = channel.send(&PairingMessage::Committed) {
                controller.rollback_unconfirmed_trust(&persisted);
                return Err(map_transport_failure(error));
            }
            match channel.receive() {
                Ok(PairingMessage::Committed) => {
                    finish_trust(controller, session_id, persisted, &peer);
                    Ok(())
                }
                Ok(PairingMessage::Rejected) => {
                    controller.rollback_unconfirmed_trust(&persisted);
                    controller.update_state(session_id, PairingState::Rejected);
                    Ok(())
                }
                Ok(_) => {
                    controller.rollback_unconfirmed_trust(&persisted);
                    Err(PairingFailure::InvalidPeer)
                }
                Err(error) => {
                    controller.rollback_unconfirmed_trust(&persisted);
                    Err(map_transport_failure(error))
                }
            }
        }
        PairingRole::Receiver => match channel.receive() {
            Ok(PairingMessage::Committed) => {
                let persisted = persist_peer(controller, &peer)?;
                if let Err(error) = channel.send(&PairingMessage::Committed) {
                    controller.rollback_unconfirmed_trust(&persisted);
                    return Err(map_transport_failure(error));
                }
                finish_trust(controller, session_id, persisted, &peer);
                Ok(())
            }
            Ok(PairingMessage::Rejected) => {
                controller.update_state(session_id, PairingState::Rejected);
                Ok(())
            }
            Ok(_) => Err(PairingFailure::InvalidPeer),
            Err(error) => Err(map_transport_failure(error)),
        },
    }
}

fn persist_peer(
    controller: &PairingController,
    peer: &VerifiedPeer,
) -> Result<TrustMutation, PairingFailure> {
    controller.persist_trust(peer).map_err(|error| {
        if matches!(error, PairingError::IdentityConflict) {
            return PairingFailure::InvalidPeer;
        }
        PairingFailure::PersistenceFailed
    })
}

fn finish_trust(
    controller: &PairingController,
    session_id: &PairingSessionId,
    persisted: TrustMutation,
    peer: &VerifiedPeer,
) {
    controller.remember_peer_endpoint(peer);
    if persisted.newly_added() {
        controller
            .trusted_changed
            .publish(TrustedDeviceChange::Added(persisted.into_device()));
    }
    controller.update_state(session_id, PairingState::Trusted);
}

fn authentication_context(
    local: &PairingHello,
    local_fingerprint: [u8; 32],
    peer: &PairingHello,
    peer_fingerprint: [u8; 32],
) -> Vec<u8> {
    let mut identities = [
        (
            local.device_id().as_str(),
            local_fingerprint,
            local.nonce(),
            local.connection_port(),
        ),
        (
            peer.device_id().as_str(),
            peer_fingerprint,
            peer.nonce(),
            peer.connection_port(),
        ),
    ];
    identities.sort_by(|left, right| left.0.cmp(right.0).then_with(|| left.1.cmp(&right.1)));
    let mut context = Vec::new();
    for (device_id, fingerprint, nonce, connection_port) in identities {
        let length = u16::try_from(device_id.len()).unwrap_or(0);
        context.extend_from_slice(&length.to_be_bytes());
        context.extend_from_slice(device_id.as_bytes());
        context.extend_from_slice(&fingerprint);
        context.extend_from_slice(&nonce);
        context.extend_from_slice(&connection_port.to_be_bytes());
    }
    context.extend_from_slice(&ProtocolVersion::CURRENT.major().to_be_bytes());
    context.extend_from_slice(&ProtocolVersion::CURRENT.minor().to_be_bytes());
    context
}

fn map_transport_failure(error: TransportError) -> PairingFailure {
    match error {
        TransportError::TimedOut => PairingFailure::TimedOut,
        TransportError::InvalidMessage | TransportError::MessageTooLarge => {
            PairingFailure::InvalidPeer
        }
        TransportError::ConnectionFailed
        | TransportError::TlsHandshakeFailed
        | TransportError::MissingPeerIdentity => PairingFailure::ConnectionFailed,
        _ => PairingFailure::Internal,
    }
}
