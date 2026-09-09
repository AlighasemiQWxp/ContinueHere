# ContinueHere Slint Client

This is the native Rust/Slint replacement for the existing Flutter Windows
client. The current milestone recreates the five existing screens and their
implemented behavior on Windows. Additional platforms and new features follow
after Windows parity is accepted.

The implementation includes startup/shutdown, discovery, pairing verification,
trusted-device connection commands, URL/YouTube/local-video handoffs, file
transfers, native dialogs, contextual image/video previews, settings, and
device-grouped history with retry and unread indicators. Rust formatting,
workspace checks, warnings-denied Clippy, tests, and the Windows release build
passed locally on September 9, 2026. Visual comparison and Windows interaction
acceptance are pending.

The client owns `ContinueHere` directly. Focused Rust controllers retain core
handles and delegate subscriptions. Core delegates schedule refresh requests on
Slint's event loop, where the controllers rebuild immutable presentation
snapshots. The reusable core does not depend on Slint.

On Windows, persisted data stays in the existing Flutter client's roaming
application-data directory at `AlighasemiQWxp/ContinueHere`.

## Windows setup

Prepare the official GStreamer MSVC x64 runtime and development packages once:

```powershell
.\scripts\install-windows-media.ps1
```

The script downloads both packages and their published SHA-256 checksums from
the [official GStreamer Windows downloads](https://gstreamer.freedesktop.org/data/pkg/windows/1.26.10/msvc/),
verifies them, and extracts a private copy under
`%LOCALAPPDATA%\ContinueHere\dependencies`. It does not need administrator
access or register a machine-wide installation. The development package
supplies native link libraries and pkg-config; the runtime supplies media
plugins and DLLs.

The run and validation scripts recognize that private copy,
`GSTREAMER_1_0_ROOT_MSVC_X86_64`, and the standard
`C:\gstreamer\1.0\msvc_x86_64` and
`C:\Program Files\gstreamer\1.0\msvc_x86_64` locations. They prepend its `bin`
directory to PATH for the current process. This follows the
[GStreamer Rust setup instructions](https://github.com/GStreamer/gstreamer-rs#windows).
Rust and Visual Studio's Desktop development with C++ tools are also required.

Run from the repository root:

```powershell
.\scripts\run-slint.ps1
```

Run validation manually from the same root:

```powershell
.\scripts\validate-slint.ps1
```

This formats the Rust workspace, checks all targets, runs warnings-denied
Clippy and tests, and builds the Windows release client. The first run resolves
the new dependencies into `Cargo.lock`. The older Flutter validation does not
validate this migration. `scripts/validate.ps1` additionally validates the
retained Flutter reference.

The Flutter client remains in `apps/client` until Slint reaches feature parity
and passes Windows acceptance. Use the
[Windows migration checklist](../../docs/WINDOWS_SLINT_MIGRATION.md) for the
side-by-side interaction and media checks. Green compilation or CI alone does
not establish visual or interaction parity.

Windows release execution currently requires the installed GStreamer runtime.
A standalone installer bundling dependencies is not part of this migration.
Non-Windows builds have explicit unavailable results for native dialogs and
video playback; they are not supported application platforms yet.
