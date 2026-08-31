# ContinueHere

[![CI](https://github.com/AlighasemiQWxp/ContinueHere/actions/workflows/ci.yml/badge.svg)](https://github.com/AlighasemiQWxp/ContinueHere/actions/workflows/ci.yml)

ContinueHere is an in-development cross-platform application for continuing an
activity on another trusted device. The project is written in Rust and is being
built as a collection of small, focused modules with clear ownership boundaries.

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
- Automated tests for the public API, module lifecycle, and handle behavior

The protocol and security design, Discovery, and Phase 9 Pairing are complete.
Pairing passes the complete local validation suite and continuous integration.
General authenticated transport, handoff features, and desktop/mobile
interfaces remain planned work.
Discovery candidates are only untrusted connection hints. No sensitive
information should be transferred yet. See the
[protocol and security design](docs/PROTOCOL_SECURITY.md) and
[project roadmap](docs/ROADMAP.md) for the intended development order.

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
│   ├── desktop/          Planned desktop application
│   └── mobile/           Planned mobile application
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

The generated root `settings.bin`, `device_identity.bin`, and
`trusted_devices.bin` files are ignored by Git. Private cryptographic identity
material is stored through the operating system's credential store instead of
the project directory.

## Development

### Requirements

- The stable Rust toolchain
- `rustfmt`
- Clippy

The repository includes `rust-toolchain.toml`, so Rustup can install the required
components automatically.

### Validation

Run the complete local validation suite from the workspace root:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
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
