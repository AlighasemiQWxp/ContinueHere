# ContinueHere Client

Frontend ownership and migration checkpoints are documented in
[Frontend architecture](../../docs/FRONTEND.md). Rust implementation files are
grouped by feature under `src/ui`; Slint pages and dialogs live under `ui/pages`
and `ui/dialogs`. Both structural checkpoints have passed user validation. The
current checkpoint gives each feature restricted core access and uses typed
commands for UI actions.

This is the native Rust/Slint application. Windows is the accepted reference
platform. The Android port reuses the same Rust core, Slint interface, focused
controllers, Handles, and application behavior; only operating-system services
are implemented behind the Android platform boundary.

The implementation includes startup/shutdown, discovery, pairing verification,
trusted-device connection commands, URL/YouTube/local-video handoffs, file
transfers, native dialogs, contextual image/video previews, settings, and
device-grouped history with retry and unread indicators. The current redesign
uses Send, Receive, History, and Settings. Slint's official Material library
supplies Navigation Drawer, Navigation Bar, buttons, SnackBar, Radio Button,
Slider, TextField, and DropDownMenu components.

The drawer is used at widths of at least 900 logical pixels; smaller windows use
bottom navigation. Content and preview controls scroll at a minimum window size
of 360 by 360 logical pixels. The primary content surface scrolls vertically
without placing a horizontal scrollbar over compact navigation. Drawer actions
remain top-aligned. The Send screen follows a task-first sequence: choose a
connected destination, choose content from a compact responsive grid, use link
tools only when needed, then manage nearby or trusted devices. Manual endpoint
entry and listener details remain available as secondary troubleshooting tools.
Purple, Red, and Green themes provide restrained gradients, highlights, cards,
and controls; theme selection and application brightness (50–100 percent) persist
through SettingsManager. Brightness affects application rendering, not the
physical monitor or Windows file dialogs.

`UiManager` owns a UI-only transition controller. Preview, reconnect, and error
surfaces acquire transition Handles and render through one modal host above the
page and navigation layers. A transition stays active until its Back, Escape, or
Close action releases the Handle; application shutdown remains the final cleanup
safeguard.

Document preparation uses a small internal `PhaseController<P>` composed into
Handoff's preparation owner. Its phases cover choosing a document and editing
the page or slide; the core remains responsible for the submitted handoff and
transfer. Cancelling preparation clears its draft and invalidates late picker
results. Invalid input or a failed submission leaves the draft available for
correction. The helper is not applied to Pairing or Transfers in this pilot.

After running the validation script, manually check the document workflow:

- Choose PDF and PowerPoint files, enter a valid page or slide, and send. The
  dialog should close while the existing handoff continues normally.
- Enter zero, nonnumeric text, and an overflowing number. Correct the input and
  retry; the document and dialog should remain available after each error.
- Cancel the picker, cancel the document dialog with Cancel or Back/Escape, and
  choose again. No old title, position, or dialog should reappear.
- Disconnect the destination before submitting. The draft should remain editable
  after the error. Also check shutdown with a picker or document dialog open,
  including asynchronous picker completion on Android.

Automated tests cover phase notifications, subscription removal, preparation
cancellation, stale selection identities, and continuation validation. The pilot
passed manual Windows formatting, compilation checks, Clippy, tests, and release
build validation. Android-to-computer connectivity is currently unresolved, so
manual document workflow acceptance remains pending.

Save Directory displays the saved path and opens a native folder dialog. File,
Folder, Media, Image, and Video selection uses native dialogs with category
filters. Android selection uses the system document picker and imports granted
content into session-scoped app-owned staging paths before the existing handoff
and transfer APIs receive it. Stale imports are cleared at startup and clean
shutdown. Folder transfers preserve nested content and empty directories.
Verified incoming files use a no-overwrite commit with a portable copy fallback
when the destination filesystem does not support hard links. The Receive screen
shows IPv4 interface addresses and both listener ports. History can reuse an
editable, previously authenticated connection endpoint.

While the client is open, it automatically keeps one pairing receive operation
ready and advertises the operation's current listener endpoint. Terminal pairing
results re-arm receiving. Re-entering the same normalized manual endpoint reuses
the existing candidate, and nearby presentation coalesces identical endpoints.
Pairing also exchanges each application's listener port inside the verified pairing
channel. Once both devices commit trust, the initiator connects automatically; later
Connect actions reuse the verified endpoint without a second manual address field.

The supplied ContinueHere artwork is embedded as the Slint window icon and a
multi-resolution Windows executable icon. The bundled Material source and small
accessibility/slider adaptations are documented in [vendor notes](vendor/README.md).

The connection workflow and simplified presentation passed manual formatting,
checks, linting, tests, and release compilation on September 11, 2026. Pairing
also passed manual two-computer acceptance, and the simplified interface passed
manual visual acceptance on September 12, 2026. Full two-computer transfer
acceptance remains pending.

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

## Android setup

The Android client uses the same Rust/Slint application crate and builds as an
ARM64 native activity. Install Android SDK Platform 36, Build Tools 36.0.0,
NDK, platform tools, and Android Studio's bundled JDK. The Android scripts honor
`ANDROID_HOME`, `ANDROID_NDK_ROOT`, and `JAVA_HOME` when they are set. Otherwise,
they detect Android Studio's standard Windows SDK and JDK locations and select
the newest installed NDK for the current process. Android helper sources are
compiled as Java 17 bytecode so newer bundled JDKs remain compatible with D8.
Install the Rust target and Cargo APK tooling once:

```powershell
rustup target add aarch64-linux-android
cargo install cargo-apk
```

Build from the repository root with:

```powershell
.\scripts\build-android.ps1
```

With an Android device connected through ADB, build, install, and run with:

```powershell
.\scripts\run-android.ps1
```

The first Android checkpoint covers native startup and app-private persistence.
The Android platform boundary also keeps Wi-Fi multicast reception enabled for
the shared mDNS discovery runtime. Android file and media selection is
asynchronous: system-granted files are staged as ordinary local paths so the
reusable Rust core remains independent of Android content URIs. Android folder
tree selection and external destination folders require a separate Storage
Access Framework adapter. Android opening, media, lifecycle, and full
device-to-Windows parity remain part of Phase 17 and must pass physical-device
acceptance before the phase can be marked complete.
