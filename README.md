# ContinueHere

[![CI](https://github.com/AlighasemiQWxp/ContinueHere/actions/workflows/ci.yml/badge.svg)](https://github.com/AlighasemiQWxp/ContinueHere/actions/workflows/ci.yml)

ContinueHere is an in-development cross-platform application for continuing an
activity on another trusted device. Its reusable core and new native client are
written in Rust, with Slint used for declarative presentation. Small, focused
modules keep ownership boundaries clear.

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
- Native Rust/Slint Windows client with Send, Receive, History, and Settings, direct core ownership,
  pairing, handoffs, transfers, media previews, settings, and activity history
  (Windows interaction acceptance pending)
- Task-first Slint interface with a compact Navigation Drawer/Bar, standard-sized
  actions, unobstructed vertical scrolling, top-aligned navigation, responsive
  content choices, save confirmation SnackBar, theme/language radio buttons, and
  app brightness
- Persisted Purple, Red, and Green appearance styles and a native Save Directory picker
- File, Folder, Media, Image, and Video selection with native category filters
- Bounded folder transfer preserving nested files and empty directories
- LAN address display, automatic pairing readiness, verified connection-endpoint
  exchange, automatic post-pairing connection, and trusted reconnect hints
- Device-grouped activity history and manual retry (desktop acceptance pending)
- Automated tests for the public API, module lifecycle, and handle behavior

The protocol and security design, Discovery, Pairing, and authenticated
Transport are complete. Phase 11 provides the validated URL handoff technical
MVP, Phase 12 adds validated YouTube handoffs with playback position, and Phase
13 provides validated streaming file transfer. Phase 14 provides validated
local-video handoff with playback position. The native Rust/Slint Windows
interface provides contextual file opening, image and video previews, responsive
English/Persian layouts, and direct ownership of the reusable Rust core.
Additional platforms and new features follow separately, one at a time.

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
The client does not add playback during download, transcoding, or partial transfer
resume. Phase 16 adds persistent history and explicit retries from the beginning.

## Activity history

History opens with one card per other device. Opening a card shows that device's
files, URL and video handoffs, and connection sessions. Entries retain locally
observed timestamps for starting, completing files, ending activities, and
disconnecting. Dates and exact times include milliseconds and the local UTC offset.
An unexpected app exit leaves an interrupted entry with an unknown end time.

Opening History clears its navigation indicator. Device indicators remain until
their individual timelines are opened. New activity for a device whose timeline
is already visible is treated as read. Both levels use the same golden badge.

History is stored locally in `history.bin`. It contains device names, URLs, file
paths, and operation metadata; it does not copy transferred file contents.
Clear history removes finished entries and keeps received files and active work.
Retry creates a linked new attempt, rechecks the trusted connection and source
file metadata, and uses the existing send flows. The receiver accepts a new file
offer again. See [the architecture guide](docs/ARCHITECTURE.md#activity-history-and-retry)
for persistence and retry boundaries.

The connection workflow and simplified desktop presentation passed the complete
local validation suite on September 11, 2026. Pairing also passed manual
two-computer acceptance, and the simplified interface passed manual visual
acceptance on September 12, 2026. Full two-computer transfer acceptance remains
pending.
Run the complete local validation from the project root:

```powershell
.\scripts\validate.ps1
```

This requires Rust, the Windows desktop build tools, and the documented native
media dependencies. The script stops at the first failure.

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
│   └── client/           Native Rust/Slint application
├── crates/
│   └── continuehere/     Core Rust library
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

The generated root `settings.bin`, `device_identity.bin`, `history.bin`,
`endpoints.bin`, and `trusted_devices.bin` files are ignored by Git. Private cryptographic identity
material is stored through the operating system's credential store instead of
the project directory.

## Development

### Requirements

- Rust 1.88 or newer on the stable release channel
- `rustfmt`
- Clippy
- Slint 1.17.1, resolved by Cargo
- GStreamer MSVC x64 runtime and development packages for the Windows Slint client;
  see [Windows client setup](apps/client/README.md)
- `pkg-config` and `libfontconfig1-dev` when building on Debian or Ubuntu Linux
- Visual Studio 2022 with the Desktop development with C++ workload on Windows

The repository includes `rust-toolchain.toml`, so Rustup can install the required
components automatically.

Prepare the Windows media dependencies once without administrator access:

```powershell
.\scripts\install-windows-media.ps1
```

Run the Rust/Slint desktop client from the repository root with:

```powershell
.\scripts\run.ps1
```

The client stores application data in the platform's application-data directory.
On Windows it uses `AlighasemiQWxp/ContinueHere`. Focused Rust controllers own
the Windows flows, with English/Persian presentation and native file/media
support. See the [Windows acceptance checklist](docs/WINDOWS_ACCEPTANCE.md) for
the outstanding interaction checks.

### Validation

Run `./scripts/validate.ps1` manually from the repository root. It formats and
checks the Rust workspace, runs warnings-denied Clippy and tests, and builds the
Windows release client. Equivalent manual commands require the Windows media
environment first:

```powershell
. ./scripts/use-windows-media.ps1
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo build -p continuehere_client --release
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
