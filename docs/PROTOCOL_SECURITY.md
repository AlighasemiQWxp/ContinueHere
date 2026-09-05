# ContinueHere Protocol and Security Design

## Purpose

This document defines the protocol and security contracts that discovery,
pairing, transport, and handoff features must follow. It is a design contract,
not a claim that secure networking is already implemented.

The design assumes that the network can be hostile. A peer must not become
trusted merely because it is nearby, knows a device identifier, or can reach a
ContinueHere endpoint.

## Security goals

ContinueHere must provide:

- explicit user approval before a new device becomes trusted;
- mutual authentication between previously paired devices;
- confidentiality and integrity for all application messages and transferred
  content;
- forward secrecy for established sessions;
- protection against protocol downgrade, replay, message tampering, peer
  impersonation, and malformed input;
- revocable trust without changing the local `DeviceId`;
- bounded parsing and resource use before allocating or processing payloads;
- separation between public device identity, cryptographic identity, trust
  records, and active connections.

The design does not attempt to protect an endpoint after its operating system,
process, or secure credential store has been compromised. Traffic analysis,
radio jamming, network denial of service, and a user approving the wrong
pairing code are also outside the guarantees of the protocol.

## Identity and trust

The existing `DeviceId` identifies one ContinueHere installation. It is not a
secret, a credential, or proof that a peer owns that identity.

Each installation will have a separate cryptographic identity. Its private key
must be generated from the operating system's secure random source and stored
through a platform secure-storage backend. There must be no plaintext fallback
to `settings.bin`, `device_identity.bin`, logs, source files, or ordinary
configuration. Failure to access secure storage must fail closed.

The initial protocol profile uses an Ed25519 signing key as the long-term
identity. TLS carries that key in a locally issued certificate, but trust is
pinned to the SHA-256 fingerprint of its SubjectPublicKeyInfo rather than to a
hostname, public certificate authority, or changeable certificate wrapper. A
lost or changed long-term key requires explicit re-pairing.

A trusted-peer record binds a `DeviceId` to the peer's cryptographic public-key
identity and user-visible metadata. Discovery data is only an untrusted hint.
The authenticated identity presented by the secure channel is authoritative.
If a known `DeviceId` presents a different key, the connection must be rejected
and the device must be paired again through an explicit user action.

Removing a trusted device deletes its trust record and prevents future
authenticated connections. It does not delete or regenerate either device's
local `DeviceId`.

## Discovery privacy

Unknown devices must receive only the information needed to locate a candidate
endpoint and determine basic protocol compatibility. Local discovery must use
a short-lived discovery instance identifier rather than broadcasting the
stable `DeviceId` as the routing identity.

The stable device identifier, display name, detailed capabilities, and trusted
status must be treated as unverified until they are received inside an
authenticated secure channel. Discovery implementations must not persist an
unknown peer as trusted state.

## Pairing contract

Pairing is the only normal path from unknown peer to trusted peer:

1. One device starts a time-limited pairing session.
2. The devices establish an encrypted TLS 1.3 connection using fresh ephemeral
   key exchange. The presented long-term identities are not trusted yet.
3. Both devices derive the same authentication value from the completed
   handshake through a domain-separated TLS exporter. The exporter context
   includes both device identifiers, both public-key fingerprints, both
   connection nonces, and the negotiated protocol version in a fixed order.
4. The user verifies that value through a QR code or matching human-readable
   code and explicitly approves both devices.
5. Each device atomically stores the other device's identifier and public-key
   identity as one trusted-peer record.
6. Trust-established events are published only after persistence succeeds on
   the local device.

Pairing sessions must expire, accept only one successful confirmation, limit
failed attempts, and erase temporary secrets when they finish or are
cancelled. A failed, mismatched, expired, or rejected pairing must create no
trust record. A short manual code is an authentication value, not a password,
and must never be used directly as an encryption key.

The QR representation carries the complete 256-bit authentication value. The
manual representation uses a 10-digit value, displayed in groups, and permits
only one confirmation attempt per handshake. Either method detects a
man-in-the-middle substitution before trust is stored.

Every temporary pairing attempt is owned by a `PairingHandle`. Releasing or
dropping its final reference cancels the session, closes its pairing channel,
prevents later approval, and erases temporary authentication material.
Persistent trusted-device records are not handle-owned and require explicit
revocation.

Pairing-session and trusted-device delegates carry immutable snapshots. They
are notifications rather than commands or authoritative storage. Pairing state
and trust records are committed before their events are published, and event
callbacks run after internal locks are released.

## Secure transport

All application traffic must use TLS 1.3. Older TLS versions, anonymous cipher
suites, plaintext fallback, and application data before authentication are not
allowed. TLS early data, commonly called 0-RTT, must remain disabled because it
can be replayed.

The initial TLS profile uses X25519 ephemeral key exchange, Ed25519 peer
signatures, and the standard TLS 1.3 AES-128-GCM/SHA-256 or
ChaCha20-Poly1305/SHA-256 authenticated cipher suites. Algorithm selection is
performed by the reviewed TLS implementation. ContinueHere code must not
implement cryptographic primitives itself.

Previously paired devices must mutually authenticate their cryptographic
identities against the locally stored trust records. A successful TLS
handshake proves control of the expected keys; matching a `DeviceId` alone is
never sufficient.

Transport errors, authentication failures, and protocol violations terminate
the affected connection. They must not silently retry with weaker security.
Session secrets and temporary plaintext buffers must be released as soon as
their operation ends.

## Application protocol

The application protocol runs only inside the authenticated TLS channel. Its
version is independent from TLS and uses the existing `ProtocolVersion` model.

The first application exchange is a protected hello containing:

- the supported protocol major and minor range;
- the authenticated device identifier;
- platform and supported capabilities;
- a fresh connection nonce;
- protocol limits required by the peer.

The discovery-advertised version is only a connection hint. The protected hello
is the authoritative negotiation. A different major version is incompatible
and closes the connection. For the same major version, both peers select the
highest mutually supported minor version. Required behavior cannot be added in
a minor version, and negotiation results cannot enable capabilities that either
peer omitted.

## Message representation

Control messages use deterministic CBOR with integer field and message
identifiers. Every encoded value uses a definite length. Duplicate map keys,
indefinite-length values, trailing data, invalid UTF-8, unsupported required
fields, and non-deterministic encodings are rejected.

Each transport frame has a fixed-size length prefix followed by exactly one
CBOR message envelope. The envelope contains:

- a message kind;
- a request identifier when the message belongs to an operation;
- a typed payload;
- optional fields that are explicitly defined as safe to ignore.

Message kinds are divided into requests, responses, events, cancellations,
stream control, and protocol errors. Request identifiers correlate replies and
make duplicate requests detectable. They are not authentication credentials.

Large files and videos are never collected into one control message. A stream
starts with bounded metadata, carries bounded sequential chunks, and ends with
an authenticated result or cancellation. Phase 13 defines file-transfer
integrity, non-resumable retry behavior, destination handling, and final atomic
commit rules below.

## Parsing and resource limits

Every implementation must enforce limits before allocation. The protocol
profile for each implemented phase must define at least:

- maximum control-frame and field sizes;
- maximum nesting depth and collection counts;
- maximum concurrent requests, streams, and pending pairings;
- handshake, idle, request, and transfer timeouts;
- per-peer rate limits and buffered-byte limits.

Lengths must be converted with checked arithmetic. Unknown message kinds,
invalid state transitions, duplicate identifiers, out-of-order stream chunks,
and data beyond a declared boundary are protocol errors. Parsers return typed
errors and never expose partially validated messages to feature systems.

## Replay and state rules

TLS record protection supplies integrity and ordering within one connection.
The application protocol additionally uses fresh connection nonces and request
identifiers so a request cannot be confused with an operation from another
connection.

State-changing operations must define whether they are idempotent. A duplicate
non-idempotent request is rejected rather than executed twice. Trust changes,
handoffs, and final transfer commits require an authenticated current session
and cannot be accepted from cached discovery or application data.

## System ownership

Current and future phases preserve the existing main-system architecture:

- `DiscoveryManager` owns candidate endpoint discovery and untrusted
  discovery state.
- `PairingManager` owns pairing sessions, trusted-peer records, approval,
  revocation, and pairing-specific events. Its private `PairingController`
  coordinates each stateful workflow.
- `SecurityManager` owns the local cryptographic identity, secure-storage
  backend, TLS identity construction, and exporter derivation.
- `TransportManager` owns listeners, connections, protocol framing, and
  validated message delivery. Phase 9 exposes only its pairing capability;
  Phase 10 adds normal trusted application connections.
- `HandoffManager` owns caller-scoped handoff operations, incoming handoff
  state, URL semantics, duplicate handling, and handoff-specific events. Its
  private controller coordinates the workflow through a typed Transport
  capability.

Normal application connections are manager-owned infrastructure rather than
caller-owned handles. A private supervisor serializes registry changes and
owns isolated asynchronous work per peer connection. Each connection has one
sequential reader and one writer owner so a cancelled command cannot split a
partially read frame. Feature operations may be handle-owned, but their release
does not implicitly close a connection shared with another operation.

Transport receives only a read-only trusted-peer capability from Pairing.
Removing trust remains a Pairing operation; after the removal commits,
Transport closes matching connections and rejects later handshakes. The
temporary pairing channel uses a separate ALPN identifier and
untrusted-certificate verification policy and cannot be reused as an
authenticated application channel.

Each manager is an independent main system under `CoreModules`. They
communicate through constructor-injected typed capabilities and specific
delegate events. Raw sockets, private keys, undecoded CBOR values, and mutable
trust records are not exposed through the public application API.

Feature systems receive only validated typed messages. Transport does not
decide whether a URL should open or where a file should be stored, and feature
systems will not perform encryption or parse network frames.

Phase 11 URL handoffs use authenticated application connections and advertise
the `UrlHandoff` capability during the protected hello. Transport owns bounded
encoding, request correlation, timeouts, and typed delivery. Handoff validates
URL semantics, rejects unsupported schemes, and commits an incoming immutable
record before acknowledging delivery. A handoff identifier provides bounded
duplicate protection independently of the connection-local request identifier.
URL payloads are limited to 4,096 UTF-8 bytes. Runtime state is limited to 32
active outgoing operations, 64 pending incoming handoffs, and 256 remembered
handoff identifiers for duplicate detection.

Delivery acknowledgement means that the receiving Handoff system accepted the
request into runtime state. It does not mean that a browser opened the URL.
Received URLs are not persisted or logged because they may contain private
query or fragment data.

Phase 12 keeps the same Handoff lifecycle and acknowledgement messages while
adding a distinct YouTube payload kind. The protected hello must include
`PlaybackPositionHandoff` before that payload can be sent. The payload contains
one 16-byte handoff identifier, one YouTube video identifier of at most 11
ASCII bytes, and one unsigned 64-bit millisecond playback position. Transport
enforces wire bounds before allocation; Handoff then requires an exact valid
11-character video identifier before committing incoming state.

Only HTTPS `youtube.com`, `www.youtube.com`, and `m.youtube.com` watch URLs and
single-video `youtu.be` URLs are accepted as outgoing YouTube handoffs. Exact
host matching rejects lookalike domains. The receiving interface can construct
a canonical YouTube resume URL, but acknowledgement still means only that the
receiving Handoff system accepted the immutable runtime record. YouTube URLs
and playback positions are not persisted or logged.

Phase 13 advertises `FileTransfer` only while the receiving FileTransfer system
is available. Every transfer uses a random 16-byte transfer identifier and this
authenticated request sequence: offer, accepted or rejected, zero or more
sequential chunks, finish, and final accepted or rejected. Cancellation is a
typed request and is valid while an offer or stream is active. A response is
correlated by the connection-local request identifier and must contain the
same transfer identifier.

An offer contains one UTF-8 file name of at most 255 bytes and one unsigned
64-bit file size. The FileTransfer system rejects empty names, path separators,
`.` and `..`, control characters, non-portable reserved names, non-regular
sources, and files larger than 100 GiB. The sender cannot provide a destination
path. At most four non-terminal transfers and 16
retained incoming offers exist at once. An incoming offer waits at most 60
seconds for an explicit local decision; Transport allows 65 seconds for the
correlated offer response.

Each chunk contains an unsigned 64-bit byte offset and at most 32 KiB of data.
Chunks must be non-empty, sequential, and must not exceed the offered size.
The sender waits for each authenticated chunk acknowledgement before reading
and sending the next chunk, which bounds in-flight file data and supplies
backpressure through the existing connection command and frame limits. Normal
chunk, finish, and cancellation requests retain the 15-second request timeout.

The receiver writes chunks to a randomly named temporary file in the selected
destination directory while both peers compute SHA-256 incrementally. Finish
contains the sender's 32-byte digest. The receiver accepts finish only when the
received byte count exactly matches the offer and the digest matches. It then
flushes the temporary file and creates the final name through an atomic
no-overwrite filesystem operation before acknowledging completion. Existing
destination files are never replaced. Rejection, cancellation, shutdown,
integrity failure, or filesystem failure removes incomplete temporary data.
Phase 13 does not resume partial data; a later retry starts at byte zero.

## Local video handoff

Phase 14 adds `LocalVideoHandoff` capability code 4 and handoff message kind 15.
The deterministic CBOR payload is an array of exactly three values: a 16-byte
handoff ID, a 16-byte transfer ID, and an unsigned 64-bit millisecond playback
position. It uses the existing handoff accepted/rejected responses.
The message contains no source path, receiver destination path, shell command,
or player arguments.

Local-video support is advertised when both Handoff and Transfer handlers are
available. Before creating a transfer, the sender checks that the connected
peer advertised both local-video and file-transfer support. An unsupported
connection cannot receive a silently downgraded ordinary file operation.
The existing strict hello decoder rejects unknown capability codes; older
builds that do not recognize code 4 may reject the connection itself.
Mixed-version interoperability with those builds is not claimed.

The complete video first follows the Phase 13 offer, explicit acceptance,
bounded streaming, digest verification, and atomic commit contract.
Only then does the sender send the playback metadata. The receiver binds the
transfer ID to the authenticated peer and requires a completed incoming transfer
with a locally resolved regular video file. An unverified or other-peer transfer
cannot be turned into a ready video by supplying its ID. No file is opened or
executed automatically.

The existing bounded handoff inbox and duplicate window apply. Remembered
duplicates must match both sender and exact typed payload; changing a transfer
ID or playback position under a remembered handoff ID is rejected. An exact
duplicate remains acknowledged if its transfer record was subsequently removed,
without recreating an incoming record or opening a file.

Cancellation cleans up an incomplete owned transfer through Transfer. A file
already committed on the receiver is preserved even if handoff acknowledgement
fails or cancellation arrives afterwards. File completion and player execution
are separate facts: acknowledgement certifies only acceptance of the ready
handoff record. Playback positions and handoff records remain in memory.

## Persistence boundaries

Security-sensitive state remains separate from user preferences and the public
device identity:

- `settings.bin` contains preferences only;
- `device_identity.bin` contains the stable non-secret device identity only;
- platform secure storage contains local private-key material;
- the pairing system owns versioned, bounded, atomically replaced trusted-peer
  records;
- temporary pairing, handshake, and session secrets are never persisted.

Malformed trust data must be preserved and reported instead of being silently
replaced. Trust and key changes publish events only after their authoritative
state has committed successfully.

## Phase boundaries

Phase 7 fixes the contracts in this document. It does not create keys, open
sockets, discover peers, pair devices, or claim that transfers are secure.

- Phase 8 implements handle-owned local and manual candidate discovery plus
  temporary endpoint advertisement. Its candidates remain untrusted hints.
- Phase 9 implements pairing and trusted-device management.
- Phase 10 implements mutually authenticated TLS transport and protocol
  framing.
- Phases 11 and 12 implement typed URL and playback-position handoffs.
- Phase 13 implements explicitly accepted, bounded streaming file transfer.
- Phase 14 links verified file delivery to a local-video playback position.
- Later phases add typed messages and limits without weakening these contracts.

Any later change that weakens authentication, confidentiality, integrity,
version checks, parsing limits, secure storage, or explicit user approval
requires a new security review before implementation.

## Standards basis

- TLS 1.3: [RFC 8446](https://www.rfc-editor.org/rfc/rfc8446)
- Secure TLS recommendations: [RFC 9325](https://www.rfc-editor.org/rfc/rfc9325)
- Ed25519 signatures: [RFC 8032](https://www.rfc-editor.org/rfc/rfc8032)
- Deterministic CBOR: [RFC 8949](https://www.rfc-editor.org/rfc/rfc8949)
