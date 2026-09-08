# ContinueHere Slint Client

This is the native Rust/Slint replacement for the existing Flutter client. It
currently provides the first migration slice: core startup and shutdown, local
and manual discovery, device snapshots, trusted-device connection state,
device-name and destination-directory settings, and live English/Persian
presentation.

The client owns `ContinueHere` directly. Focused Rust controllers retain core
handles and delegate subscriptions. Core delegates schedule refresh requests on
Slint's event loop, where the controllers rebuild immutable presentation
snapshots. The reusable core does not depend on Slint.

On Windows, persisted data stays in the existing Flutter client's roaming
application-data directory at `AlighasemiQWxp/ContinueHere`.

Run the desktop client from the repository root:

```powershell
cargo run -p continuehere_client_slint
```

The Flutter client remains in `apps/client` as the migration reference until the
Slint client reaches feature parity and passes platform acceptance. Pairing
actions, connection commands, handoffs, transfers, history, file dialogs, and
media preview have not yet been ported.
