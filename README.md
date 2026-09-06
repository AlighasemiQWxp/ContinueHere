# ContinueHere

[![CI](https://github.com/AlighasemiQWxp/ContinueHere/actions/workflows/ci.yml/badge.svg)](https://github.com/AlighasemiQWxp/ContinueHere/actions/workflows/ci.yml)

ContinueHere is an in-development cross-platform application for continuing an
activity on another trusted device. Its reusable core is written in Rust, and
its native client is being built with Flutter. Both sides use small, focused
modules with clear ownership boundaries.

## Project status

The architectural foundation is complete. It currently provides:

- A Rust workspace with a dedicated `continuehere` library crate
- A `ContinueHere` application composition root
- Independent main systems with private same-system child modules
- Strongly typed access to main systems and their capabilities
- Ordered module startup, reverse-order shutdown, and startup rollback
- Reusable handle ownership and lifecycle primitives
- Shared device identifiers, protocol versions, and device-description models
- Versioned binary settings with atomic persistence
- Typed default and per-transfer destination directory handling
- Directory-change delegates with automatically released subscriptions
- Persisted English and Persian language selection
- Typed translation keys with English fallback
- Left-to-right and right-to-left text direction
- Language-change delegates with automatically released subscriptions
- Stable local device identity with independent atomic persistence
- Persisted device display names with delegate-based change events
- Defined protocol, pairing, trust, and secure-transport contracts
- Hostile-network threat model and bounded message-processing rules
- Handle-owned local discovery, manual endpoints, and temporary advertisements
- Bounded untrusted discovery candidates with delegate-based change events
- Handle-owned TLS 1.3 pairing with explicit two-device verification
- OS-secured Ed25519 identities and atomically persisted trusted-device records
- Pairing-session and trusted-device delegate events
- Manager-owned authenticated TLS 1.3 application connections
- Deterministic bounded application framing and correlated control requests
- Connection lifecycle events and immediate trust-revocation cleanup
- Handle-owned URL handoffs with capability negotiation and acknowledgement
- Strict URL validation, duplicate protection, and bounded incoming handoff state
- Shared handoff Manager, Controller, Handle, state, limits, and event architecture
- Typed YouTube handoffs with validated video identifiers and playback positions
- Negotiated playback-position capability and deterministic bounded messages
- Handoff-operation and incoming-handoff delegate events
- Handle-owned streaming file transfers with explicit receiving-device approval
- Bounded authenticated chunks with sequential offsets and transport backpressure
- SHA-256 verification, destination-local temporary files, and atomic no-overwrite commits
- File-transfer progress, state, cancellation, and cleanup delegate events
- Local-video handoffs that reuse file transfer and preserve millisecond playback positions
- Receiver-owned verified video paths with explicit file acceptance and capability checks
- Native Flutter Windows client with RTL localization and contextual transferred-file previews
- Automated tests for the public API, module lifecycle, and handle behavior

The protocol and security design, Discovery, Pairing, and authenticated
Transport are complete. Phase 11 provides the validated URL handoff technical
MVP, Phase 12 adds validated YouTube handoffs with playback position, and Phase
13 provides validated streaming file transfer. Phase 14 provides validated
local-video handoff with playback position. The Windows interface is now in
development and includes contextual file opening without embedding a browser or
WebView.
Android, Linux, macOS, and iOS will use the same Flutter client in later roadmap
phases.

Discovery candidates are only untrusted connection hints. Application data must
use an authenticated Transport connection. See the
[protocol and security design](docs/PROTOCOL_SECURITY.md) and
[project roadmap](docs/ROADMAP.md) for the intended development order.

## Local video handoff

Configure a handoff handle with the trusted destination device, an absolute
video file path, and the current playback position, then use the handle:

```rust
let handle = app.handoff().get_handle("continue-local-video")?;
handle.configure_local_video(device_id, video_path, playback_position)?;
handle.use_handle()?;
```

Keep the handle alive while the operation runs. The receiver approves the file
offer through `app.file_transfers().accept_incoming(...)`, using its default
folder or a temporary destination. `Handoff::transfer()` exposes an immutable
snapshot of the associated transfer for progress and failure details.

Once the full file is verified and saved, the receiver publishes an incoming
`HandoffPayload::LocalVideo` containing its own file path, the transfer ID, and
the playback position. `Delivered` means the receiving app has that ready-to-open
record; it does not mean a player has started. Player selection, codec support,
opening, and seeking belong to the application interface.

The initial file-name extensions are MP4, M4V, MKV, WebM, MOV, and AVI,
case-insensitively. This checks file eligibility, not media decodability.
Empty files, directories, and symbolic-link sources are rejected. Playback
starts through the application interface only after the complete file arrives.
The client does not add playback during download, transcoding, partial transfer
resume, or persistent history.

## Design goals

- Keep feature modules independent and easy to replace
- Expose small typed APIs instead of implementation details
- Use explicit ownership and lifecycle rules
- Prefer event-driven coordination between systems
- Keep platform-specific code behind narrow boundaries
- Validate performance and security claims with measurements

## Workspace

```text
ContinueHere/
├── apps/
│   └── client/           Shared Flutter application
├── crates/
│   ├── continuehere/     Core Rust library
│   └── continuehere_bridge/  Flutter bridge
└── docs/                 Architecture and roadmap documentation
```

The application root owns a typed core module collection. Every independent
system has one main module in that collection and may construct private child
modules belonging to the same system. Systems communicate through narrow typed
capabilities instead of owning one another. Frequently used capabilities are
accessed through typed methods, while the dynamic registry is reserved for
optional lifecycle-managed modules. Additional details are available in the
[architecture guide](docs/ARCHITECTURE.md).

The application supplies an absolute project directory when constructing the
core. ContinueHere stores local preferences at `settings.bin` and its separate
local identity at `device_identity.bin` inside that directory:

```rust
let app = ContinueHere::builder(project_directory).build().await?;
```

The generated root `settings.bin`, `device_identity.bin`, and
`trusted_devices.bin` files are ignored by Git. Private cryptographic identity
material is stored through the operating system's credential store instead of
the project directory.

## Development

### Requirements

- Rust 1.88 or newer on the stable release channel
- `rustfmt`
- Clippy
- Flutter 3.47.2 on the stable channel
- Visual Studio 2022 with the Desktop development with C++ workload on Windows
- `flutter_rust_bridge_codegen` 2.13.0 when changing the bridge API

The repository includes `rust-toolchain.toml`, so Rustup can install the required
components automatically.

### Validation

Run the complete local validation suite from the workspace root:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cd apps/client
flutter pub get
dart format --output=none --set-exit-if-changed lib test
flutter analyze
flutter test
flutter build windows
```

## Contributing

Development conventions and the validation workflow are documented in
[CONTRIBUTING.md](CONTRIBUTING.md).

## Author

Created and maintained by AlighasemiQWxp.

## License

ContinueHere is source-available under the
[ContinueHere Source-Available License 1.0](LICENSE). You may study, compile,
run, and privately modify the project for personal, non-commercial education
and evaluation. Redistribution and commercial use are not permitted without
prior written permission from the copyright holder.

ContinueHere is not open-source software as defined by the Open Source
Initiative. See the [ownership and usage notice](NOTICE.md) for a concise
summary of the applicable restrictions intended for people and automated
tools.
