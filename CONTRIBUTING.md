# Contributing to ContinueHere

ContinueHere is developed one focused module at a time. Changes should preserve
clear ownership, small public APIs, and strict boundaries between features.

## External contributions

ContinueHere is source-available but is not an open-source project. External
code contributions are not accepted unless the maintainer invites the
contribution in writing and the ownership and licensing terms are agreed
before submission. Unsolicited pull requests may be closed without review.

Bug reports and focused technical suggestions are welcome when they do not
include third-party proprietary information or code that the reporter is not
authorized to share.

## Development principles

- Keep modules focused on one responsibility.
- Register one main module per system at the project root.
- Construct and own system child modules inside their main module.
- Prefer constructor-injected dependencies over global access.
- Keep implementation details private and expose only required capabilities.
- Use events for communication when one system should react to another.
- Keep platform-specific behavior behind replaceable interfaces.
- Avoid speculative abstractions and dependencies.
- Add tests for lifecycle rules, state transitions, and public behavior.
- Do not claim performance or security properties without evidence.

## Workflow

1. Work on one scoped change.
2. Update documentation when an API, lifecycle rule, or architectural boundary changes.
3. Run formatting, checking, linting, and tests from the workspace root.
4. Use a short imperative commit message that describes the outcome.

## Local validation

On Windows, run the complete Rust workflow from the project root with
`.\scripts\validate.ps1`. It formats the workspace, checks all targets, runs
warnings-denied Clippy and tests, and builds the release client. It stops on the
first failure. The equivalent individual checks are:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Every command must pass before a change is considered ready.

## Commit style

Keep commits small and cohesive. Use an imperative summary such as:

```text
Add ordered module shutdown
```

Explain design decisions in the commit body only when the reason is not clear
from the code and documentation.
