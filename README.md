# ContinueHere

[![CI](https://github.com/AlighasemiQWxp/ContinueHere/actions/workflows/ci.yml/badge.svg)](https://github.com/AlighasemiQWxp/ContinueHere/actions/workflows/ci.yml)

ContinueHere is an in-development cross-platform application for continuing an
activity on another trusted device. The project is written in Rust and is being
built as a collection of small, focused modules with clear ownership boundaries.

## Project status

The architectural foundation is complete. It currently provides:

- A Rust workspace with a dedicated `continuehere` library crate
- A `ContinueHere` application composition root
- Strongly typed access to core managers
- Ordered module startup, reverse-order shutdown, and startup rollback
- Reusable handle ownership and lifecycle primitives
- Automated tests for the public API, module lifecycle, and handle behavior

Discovery, pairing, secure transport, handoff features, persistent settings, and
desktop/mobile interfaces are planned work. See the [project roadmap](docs/ROADMAP.md)
for the intended development order.

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

The application root owns the main modules. Frequently used core modules are
accessed directly through typed methods, while the dynamic registry is reserved
for optional lifecycle-managed modules. Additional details are available in the
[architecture guide](docs/ARCHITECTURE.md).

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

Created and maintained by Dalton.

## License

ContinueHere is available under the [MIT License](LICENSE).
