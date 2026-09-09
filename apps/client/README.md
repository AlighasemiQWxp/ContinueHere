# ContinueHere Client

This is the native Rust/Slint application. The current platform is Windows;
additional platforms and new features follow in separate milestones.

The implementation includes startup/shutdown, discovery, pairing verification,
trusted-device connection commands, URL/YouTube/local-video handoffs, file
transfers, native dialogs, contextual image/video previews, settings, and
device-grouped history with retry and unread indicators. Rust formatting,
workspace checks, warnings-denied Clippy, tests, and the Windows release build
passed locally on September 9, 2026. Windows interaction acceptance is tracked
separately.

The client owns `ContinueHere` directly. Focused Rust controllers retain core
handles and delegate subscriptions. Core delegates schedule refresh requests on
Slint's event loop, where the controllers rebuild immutable presentation
snapshots. The reusable core does not depend on Slint.

On Windows, persisted data is stored in the roaming application-data directory
at `AlighasemiQWxp/ContinueHere`.

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
.\scripts\run.ps1
```

Run validation manually from the same root:

```powershell
.\scripts\validate.ps1
```

This formats the Rust workspace, checks all targets, runs warnings-denied
Clippy and tests, and builds the Windows release client. Use the
[Windows acceptance checklist](../../docs/WINDOWS_ACCEPTANCE.md) for interaction
and media checks. Green compilation or CI alone does not establish interaction
acceptance.

Windows release execution currently requires the installed GStreamer runtime.
A standalone installer bundling dependencies is not part of this migration.
Non-Windows builds have explicit unavailable results for native dialogs and
video playback; they are not supported application platforms yet.
