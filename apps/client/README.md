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
