# ContinueHere Client

This is the native Rust/Slint application. The current platform is Windows;
additional platforms and new features follow in separate milestones.

The implementation includes startup/shutdown, discovery, pairing verification,
trusted-device connection commands, URL/YouTube/local-video handoffs, file
transfers, native dialogs, contextual image/video previews, settings, and
device-grouped history with retry and unread indicators. The current redesign
uses Send, Receive, History, and Settings. Slint's official Material library
supplies Navigation Drawer, Navigation Bar, buttons, SnackBar, Floating Action
Button, Radio Button, Slider, TextField, and DropDownMenu components.

The drawer is used at widths of at least 960 logical pixels; smaller windows use
bottom navigation. Content and preview controls scroll at a minimum window size
of 360 by 360 logical pixels. The primary content surface scrolls vertically
without placing a horizontal scrollbar over compact navigation. Drawer actions
remain top-aligned, and content-category actions use a responsive two- or
three-column grid. Purple, Red, and Green themes provide coordinated gradients,
highlights, cards, and controls; theme selection and application brightness
(50–100 percent) persist through SettingsManager. Brightness affects application
rendering, not the physical monitor or Windows file dialogs.

Save Directory displays the saved path and opens a native folder dialog. File,
Folder, Media, Image, and Video selection uses native dialogs with category
filters. Folder transfers preserve nested content and empty directories. The
Receive screen shows IPv4 interface addresses and both listener ports. History
can reuse an editable, previously authenticated connection endpoint.

While the client is open, it automatically keeps one pairing receive operation
ready and advertises the operation's current listener endpoint. Terminal pairing
results re-arm receiving. Re-entering the same normalized manual endpoint reuses
the existing candidate, and nearby presentation coalesces identical endpoints.

The supplied ContinueHere artwork is embedded as the Slint window icon and a
multi-resolution Windows executable icon. The bundled Material source and small
accessibility/slider adaptations are documented in [vendor notes](vendor/README.md).

The changes described above passed manual formatting, checks, linting, tests,
release compilation, and GitHub CI on September 9, 2026. Windows interaction and
two-computer transfer acceptance remain pending.

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
