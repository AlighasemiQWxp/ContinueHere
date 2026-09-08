# ContinueHere Client

The native ContinueHere interface is shared by the planned Windows, Android,
Linux, macOS, and iOS applications. Phase 15 currently provides the Windows
runner.

Completed incoming transfers can be opened directly from their transfer card.
Images and videos use a native in-app preview; other supported files open with
their operating system's associated application.

The client uses `UiManager` as its public presentation API and communicates
with the Rust core through `continuehere_bridge`. See the project
[architecture guide](../../docs/ARCHITECTURE.md) for ownership and lifecycle
details.

History is organized by device. Each card opens that device's activity timeline,
including file completion, disconnections, and session end times. Opening History
clears its navigation badge; opening a device clears that device's badge.
Failed outgoing activities can be retried after reconnecting. Completed incoming
content uses the existing opening and preview flow.

Phase 16 passed binding generation, Rust formatting, workspace checks,
warnings-denied Clippy, Rust tests, Flutter formatting, analysis and tests, and
the Windows release build. Desktop interaction acceptance and GitHub CI remain
pending. Run `.\scripts\validate.ps1` manually from the repository root to repeat
the complete validation.

## Development

Resolve Flutter dependencies from this directory:

```powershell
flutter pub get
```

Generate bindings after changing the bridge API:

```powershell
flutter_rust_bridge_codegen generate
```

Run the Windows client with `flutter run -d windows`.
