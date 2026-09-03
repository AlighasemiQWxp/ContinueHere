use minicbor::{Decoder, Encoder};

use crate::models::{Capability, DeviceId, Platform};

use super::{
    ApplicationHello, ProtocolEnvelope, ProtocolLimits, ProtocolMessage, TransportError,
    UrlHandoffRejection,
};

const HELLO_KIND: u16 = 1;
const PING_KIND: u16 = 2;
const PONG_KIND: u16 = 3;
const CLOSE_KIND: u16 = 4;
const URL_HANDOFF_KIND: u16 = 5;
const URL_HANDOFF_ACCEPTED_KIND: u16 = 6;
const URL_HANDOFF_REJECTED_KIND: u16 = 7;
const MAX_IDENTIFIER_SIZE: usize = 64;
const MAX_DISPLAY_NAME_SIZE: usize = 128;
const MAX_CAPABILITY_COUNT: usize = 16;
pub(crate) const MAX_URL_SIZE: usize = 4096;

pub(crate) fn encode(envelope: &ProtocolEnvelope) -> Result<Vec<u8>, TransportError> {
    let mut encoder = Encoder::new(Vec::new());
    encoder
        .array(3)
        .map_err(|_| TransportError::InvalidMessage)?;
    encoder
        .u16(message_kind(envelope.message()))
        .map_err(|_| TransportError::InvalidMessage)?;
    encoder
        .u64(envelope.request_id())
        .map_err(|_| TransportError::InvalidMessage)?;
    encode_payload(&mut encoder, envelope.message())?;
    Ok(encoder.into_writer())
}

pub(crate) fn decode(bytes: &[u8]) -> Result<ProtocolEnvelope, TransportError> {
    let mut decoder = Decoder::new(bytes);
    require_array(&mut decoder, 3)?;
    let kind = decoder.u16().map_err(|_| TransportError::InvalidMessage)?;
    let request_id = decoder.u64().map_err(|_| TransportError::InvalidMessage)?;
    let message = decode_payload(&mut decoder, kind)?;
    if decoder.position() != bytes.len() {
        return Err(TransportError::InvalidMessage);
    }
    validate_request_id(request_id, &message)?;
    let envelope = ProtocolEnvelope {
        request_id,
        message,
    };
    if encode(&envelope)? != bytes {
        return Err(TransportError::ProtocolViolation);
    }
    Ok(envelope)
}

fn encode_payload(
    encoder: &mut Encoder<Vec<u8>>,
    message: &ProtocolMessage,
) -> Result<(), TransportError> {
    match message {
        ProtocolMessage::Hello(hello) => {
            encoder
                .array(11)
                .map_err(|_| TransportError::InvalidMessage)?;
            encoder
                .str(hello.device_id.as_str())
                .map_err(|_| TransportError::InvalidMessage)?;
            encoder
                .str(&hello.display_name)
                .map_err(|_| TransportError::InvalidMessage)?;
            encoder
                .u8(encode_platform(hello.platform))
                .map_err(|_| TransportError::InvalidMessage)?;
            encoder
                .u16(hello.protocol_major)
                .map_err(|_| TransportError::InvalidMessage)?;
            encoder
                .u16(hello.minimum_minor)
                .map_err(|_| TransportError::InvalidMessage)?;
            encoder
                .u16(hello.maximum_minor)
                .map_err(|_| TransportError::InvalidMessage)?;
            encoder
                .array(hello.capabilities.len() as u64)
                .map_err(|_| TransportError::InvalidMessage)?;
            for capability in &hello.capabilities {
                encoder
                    .u8(encode_capability(*capability))
                    .map_err(|_| TransportError::InvalidMessage)?;
            }
            encoder
                .bytes(&hello.nonce)
                .map_err(|_| TransportError::InvalidMessage)?;
            encoder
                .u32(hello.limits.maximum_frame_size)
                .map_err(|_| TransportError::InvalidMessage)?;
            encoder
                .u16(hello.limits.maximum_in_flight_requests)
                .map_err(|_| TransportError::InvalidMessage)?;
            encoder
                .u16(hello.limits.idle_timeout_seconds)
                .map_err(|_| TransportError::InvalidMessage)?;
        }
        ProtocolMessage::Ping(nonce) | ProtocolMessage::Pong(nonce) => {
            encoder
                .array(1)
                .map_err(|_| TransportError::InvalidMessage)?;
            encoder
                .u64(*nonce)
                .map_err(|_| TransportError::InvalidMessage)?;
        }
        ProtocolMessage::UrlHandoff { handoff_id, url } => {
            if url.is_empty() {
                return Err(TransportError::InvalidMessage);
            }
            if url.len() > MAX_URL_SIZE {
                return Err(TransportError::MessageTooLarge);
            }
            encoder
                .array(2)
                .map_err(|_| TransportError::InvalidMessage)?;
            encoder
                .bytes(handoff_id)
                .map_err(|_| TransportError::InvalidMessage)?;
            encoder
                .str(url)
                .map_err(|_| TransportError::InvalidMessage)?;
        }
        ProtocolMessage::UrlHandoffAccepted(handoff_id) => {
            encoder
                .array(1)
                .map_err(|_| TransportError::InvalidMessage)?;
            encoder
                .bytes(handoff_id)
                .map_err(|_| TransportError::InvalidMessage)?;
        }
        ProtocolMessage::UrlHandoffRejected { handoff_id, reason } => {
            encoder
                .array(2)
                .map_err(|_| TransportError::InvalidMessage)?;
            encoder
                .bytes(handoff_id)
                .map_err(|_| TransportError::InvalidMessage)?;
            encoder
                .u8(encode_handoff_rejection(*reason))
                .map_err(|_| TransportError::InvalidMessage)?;
        }
        ProtocolMessage::Close => {
            encoder
                .array(0)
                .map_err(|_| TransportError::InvalidMessage)?;
        }
    }
    Ok(())
}

fn decode_payload(decoder: &mut Decoder<'_>, kind: u16) -> Result<ProtocolMessage, TransportError> {
    match kind {
        HELLO_KIND => decode_hello(decoder).map(ProtocolMessage::Hello),
        PING_KIND => {
            require_array(decoder, 1)?;
            decoder
                .u64()
                .map(ProtocolMessage::Ping)
                .map_err(|_| TransportError::InvalidMessage)
        }
        PONG_KIND => {
            require_array(decoder, 1)?;
            decoder
                .u64()
                .map(ProtocolMessage::Pong)
                .map_err(|_| TransportError::InvalidMessage)
        }
        CLOSE_KIND => {
            require_array(decoder, 0)?;
            Ok(ProtocolMessage::Close)
        }
        URL_HANDOFF_KIND => {
            require_array(decoder, 2)?;
            let handoff_id = decode_handoff_id(decoder)?;
            let url = decoder.str().map_err(|_| TransportError::InvalidMessage)?;
            if url.is_empty() || url.len() > MAX_URL_SIZE {
                return Err(TransportError::InvalidMessage);
            }
            Ok(ProtocolMessage::UrlHandoff {
                handoff_id,
                url: url.to_owned(),
            })
        }
        URL_HANDOFF_ACCEPTED_KIND => {
            require_array(decoder, 1)?;
            decode_handoff_id(decoder).map(ProtocolMessage::UrlHandoffAccepted)
        }
        URL_HANDOFF_REJECTED_KIND => {
            require_array(decoder, 2)?;
            let handoff_id = decode_handoff_id(decoder)?;
            let reason = decode_handoff_rejection(
                decoder.u8().map_err(|_| TransportError::InvalidMessage)?,
            )?;
            Ok(ProtocolMessage::UrlHandoffRejected { handoff_id, reason })
        }
        _ => Err(TransportError::ProtocolViolation),
    }
}

fn decode_hello(decoder: &mut Decoder<'_>) -> Result<ApplicationHello, TransportError> {
    require_array(decoder, 11)?;
    let device_id_text = decoder.str().map_err(|_| TransportError::InvalidMessage)?;
    if device_id_text.is_empty()
        || device_id_text.len() > MAX_IDENTIFIER_SIZE
        || device_id_text.trim() != device_id_text
    {
        return Err(TransportError::InvalidMessage);
    }
    let device_id = DeviceId::new(device_id_text).map_err(|_| TransportError::InvalidMessage)?;
    let display_name = decoder.str().map_err(|_| TransportError::InvalidMessage)?;
    if display_name.is_empty()
        || display_name.len() > MAX_DISPLAY_NAME_SIZE
        || display_name.trim() != display_name
    {
        return Err(TransportError::InvalidMessage);
    }
    let platform = decode_platform(decoder.u8().map_err(|_| TransportError::InvalidMessage)?)?;
    let protocol_major = decoder.u16().map_err(|_| TransportError::InvalidMessage)?;
    let minimum_minor = decoder.u16().map_err(|_| TransportError::InvalidMessage)?;
    let maximum_minor = decoder.u16().map_err(|_| TransportError::InvalidMessage)?;
    let capability_count = require_bounded_array(decoder, MAX_CAPABILITY_COUNT)?;
    let mut capabilities = Vec::with_capacity(capability_count);
    for _ in 0..capability_count {
        let capability =
            decode_capability(decoder.u8().map_err(|_| TransportError::InvalidMessage)?)?;
        if capabilities
            .last()
            .is_some_and(|previous| previous >= &capability)
        {
            return Err(TransportError::ProtocolViolation);
        }
        capabilities.push(capability);
    }
    let nonce_bytes = decoder
        .bytes()
        .map_err(|_| TransportError::InvalidMessage)?;
    if nonce_bytes.len() != 32 {
        return Err(TransportError::InvalidMessage);
    }
    let mut nonce = [0_u8; 32];
    nonce.copy_from_slice(nonce_bytes);
    let maximum_frame_size = decoder.u32().map_err(|_| TransportError::InvalidMessage)?;
    let maximum_in_flight_requests = decoder.u16().map_err(|_| TransportError::InvalidMessage)?;
    let idle_timeout_seconds = decoder.u16().map_err(|_| TransportError::InvalidMessage)?;
    if maximum_frame_size == 0 || maximum_in_flight_requests == 0 || idle_timeout_seconds == 0 {
        return Err(TransportError::InvalidMessage);
    }
    Ok(ApplicationHello {
        device_id,
        display_name: display_name.to_owned(),
        platform,
        protocol_major,
        minimum_minor,
        maximum_minor,
        capabilities,
        nonce,
        limits: ProtocolLimits {
            maximum_frame_size,
            maximum_in_flight_requests,
            idle_timeout_seconds,
        },
    })
}

fn validate_request_id(request_id: u64, message: &ProtocolMessage) -> Result<(), TransportError> {
    let requires_identifier =
        !matches!(message, ProtocolMessage::Hello(_) | ProtocolMessage::Close);
    if (requires_identifier && request_id == 0) || (!requires_identifier && request_id != 0) {
        return Err(TransportError::ProtocolViolation);
    }
    Ok(())
}

fn require_array(decoder: &mut Decoder<'_>, expected: u64) -> Result<(), TransportError> {
    match decoder
        .array()
        .map_err(|_| TransportError::InvalidMessage)?
    {
        Some(length) if length == expected => Ok(()),
        _ => Err(TransportError::InvalidMessage),
    }
}

fn require_bounded_array(
    decoder: &mut Decoder<'_>,
    maximum: usize,
) -> Result<usize, TransportError> {
    let length = decoder
        .array()
        .map_err(|_| TransportError::InvalidMessage)?
        .ok_or(TransportError::InvalidMessage)?;
    let length = usize::try_from(length).map_err(|_| TransportError::InvalidMessage)?;
    if length > maximum {
        return Err(TransportError::InvalidMessage);
    }
    Ok(length)
}

fn message_kind(message: &ProtocolMessage) -> u16 {
    match message {
        ProtocolMessage::Hello(_) => HELLO_KIND,
        ProtocolMessage::Ping(_) => PING_KIND,
        ProtocolMessage::Pong(_) => PONG_KIND,
        ProtocolMessage::UrlHandoff { .. } => URL_HANDOFF_KIND,
        ProtocolMessage::UrlHandoffAccepted(_) => URL_HANDOFF_ACCEPTED_KIND,
        ProtocolMessage::UrlHandoffRejected { .. } => URL_HANDOFF_REJECTED_KIND,
        ProtocolMessage::Close => CLOSE_KIND,
    }
}

fn decode_handoff_id(decoder: &mut Decoder<'_>) -> Result<[u8; 16], TransportError> {
    let bytes = decoder
        .bytes()
        .map_err(|_| TransportError::InvalidMessage)?;
    if bytes.len() != 16 {
        return Err(TransportError::InvalidMessage);
    }
    let mut identifier = [0_u8; 16];
    identifier.copy_from_slice(bytes);
    Ok(identifier)
}

fn encode_handoff_rejection(reason: UrlHandoffRejection) -> u8 {
    match reason {
        UrlHandoffRejection::Invalid => 1,
        UrlHandoffRejection::Busy => 2,
        UrlHandoffRejection::Unavailable => 3,
    }
}

fn decode_handoff_rejection(value: u8) -> Result<UrlHandoffRejection, TransportError> {
    match value {
        1 => Ok(UrlHandoffRejection::Invalid),
        2 => Ok(UrlHandoffRejection::Busy),
        3 => Ok(UrlHandoffRejection::Unavailable),
        _ => Err(TransportError::InvalidMessage),
    }
}

fn encode_platform(platform: Platform) -> u8 {
    match platform {
        Platform::Windows => 1,
        Platform::Linux => 2,
        Platform::MacOs => 3,
        Platform::Android => 4,
        Platform::Ios => 5,
        Platform::Unknown => 0,
    }
}

fn decode_platform(value: u8) -> Result<Platform, TransportError> {
    match value {
        0 => Ok(Platform::Unknown),
        1 => Ok(Platform::Windows),
        2 => Ok(Platform::Linux),
        3 => Ok(Platform::MacOs),
        4 => Ok(Platform::Android),
        5 => Ok(Platform::Ios),
        _ => Err(TransportError::InvalidMessage),
    }
}

fn encode_capability(capability: Capability) -> u8 {
    match capability {
        Capability::UrlHandoff => 1,
        Capability::PlaybackPositionHandoff => 2,
        Capability::FileTransfer => 3,
    }
}

fn decode_capability(value: u8) -> Result<Capability, TransportError> {
    match value {
        1 => Ok(Capability::UrlHandoff),
        2 => Ok(Capability::PlaybackPositionHandoff),
        3 => Ok(Capability::FileTransfer),
        _ => Err(TransportError::InvalidMessage),
    }
}

#[cfg(test)]
mod tests {
    use crate::models::{DeviceId, LocalDeviceIdentity, Platform};

    use super::{ApplicationHello, ProtocolEnvelope, ProtocolMessage, decode, encode};

    #[test]
    fn hello_round_trip_is_deterministic() {
        let identity = LocalDeviceIdentity::new(
            DeviceId::new("device-1").expect("identifier should be valid"),
            "Desktop",
            Platform::Windows,
        )
        .expect("identity should be valid");
        let envelope = ProtocolEnvelope::hello(
            ApplicationHello::local(&identity, Vec::new()).expect("hello should be created"),
        );
        let encoded = encode(&envelope).expect("hello should encode");
        let decoded = decode(&encoded).expect("hello should decode");

        assert!(matches!(decoded.message(), ProtocolMessage::Hello(_)));
        assert_eq!(encode(&decoded).expect("hello should re-encode"), encoded);
    }

    #[test]
    fn trailing_data_is_rejected() {
        let envelope = ProtocolEnvelope::ping(1, 2).expect("ping should be valid");
        let mut encoded = encode(&envelope).expect("ping should encode");
        encoded.push(0);

        assert!(decode(&encoded).is_err());
    }

    #[test]
    fn non_canonical_encoding_is_rejected() {
        let encoded = [0x83, 0x18, 0x02, 0x01, 0x81, 0x02];

        assert!(decode(&encoded).is_err());
    }

    #[test]
    fn url_handoff_round_trip_is_deterministic() {
        let identifier = [7_u8; 16];
        let envelope = ProtocolEnvelope::url_handoff(
            4,
            identifier,
            "https://example.com/watch?v=1".to_owned(),
        )
        .expect("URL handoff should be valid");
        let encoded = encode(&envelope).expect("URL handoff should encode");
        let decoded = decode(&encoded).expect("URL handoff should decode");

        assert!(matches!(
            decoded.message(),
            ProtocolMessage::UrlHandoff { handoff_id, url }
                if handoff_id == &identifier && url == "https://example.com/watch?v=1"
        ));
        assert_eq!(encode(&decoded).expect("message should re-encode"), encoded);
    }
}
