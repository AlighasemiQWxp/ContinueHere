# Frontend architecture

The native client uses the same ownership principles as the core: explicit
composition, feature-owned implementation details, typed dependencies, and
events for committed changes. The reusable core remains independent of Slint.

## Ownership

| Owner | Responsibility |
| --- | --- |
| Application | Runtime, window startup, and final shutdown |
| UiManager | Construct feature owners and release them before shutting down the core |
| ShellController | Navigation, page unread acknowledgement, language, theme, and brightness presentation |
| Feature controllers | User commands, operation handles, subscriptions, and feature state |
| Feature capabilities | Small typed operations one feature intentionally offers another |
| Snapshot adapters | Convert authoritative core data into display rows |
| Slint pages and dialogs | Layout, input bindings, and user callbacks |
| Platform adapters | Native pickers, storage integration, opening content, and media |

Rust features live in `apps/client/src/ui/<feature>/`. Small features keep their
implementation in `mod.rs`. Devices, Settings, and Handoff separate callback
binding from their owner. Devices, Settings, and History have private snapshot
adapters. Preview privately owns image decoding. Shared UI dispatch and phase
notifications live under `shared`; feature rules do not belong there.

Each core-using feature receives a restricted access object composed by
`UiManager`: `DevicesAccess` exposes devices, discovery, pairing, and transport;
`PairingAccess` exposes pairing and localization; `HandoffAccess` exposes
handoff, activity, and transport; `TransfersAccess` exposes transfers, handoff,
localization, and transport; `HistoryAccess` exposes activity, transport, and
localization; `SettingsAccess` exposes device identity, appearance, directory,
and language settings; and `ShellAccess` exposes application appearance and
localization. The access objects retain the core privately for lifetime safety;
feature logic cannot reach unrelated managers through `ContinueHere`.

The shell applies language and appearance independently of Settings. Settings
edits and presents saved settings; it no longer owns application-wide strings or
refreshes unrelated features. The shell updates global presentation; features
that render localized snapshots subscribe to language changes and refresh their
own view. Core delegates marshal updates through the shared UI event target
before refreshing on the UI thread.

Pairing receives a `PairingDevices` capability for post-pairing connection and
network refresh. History, Handoff, and Transfers receive a `PreviewOpener` for
opening verified content. Transfers also receives `HandoffTransfers` to cancel
a video transfer owned by Handoff. These operations are composed by `UiManager`,
retain weak UI ownership, and keep one feature from invoking another feature
through generated window callbacks. History refreshes from Activity events,
which already cover connection changes; Transfers refreshes from transfer
events.

Slint callback strings are decoded into feature commands at their Rust boundary
for Devices, Pairing, History, Handoff, Transfers, Preview, and document
preparation. The controller logic matches typed variants rather than action
names embedded as strings.

## Views

`ui/app.slint` owns the window, navigation composition, startup overlay, modal
focus shield, and bindings to the generated Rust interface. It composes:

- `ui/pages`: Receive, Send, History, Settings, outgoing transfers, and recent
  activity. Receive retains its single feed of pending transfers and activity.
- `ui/dialogs`: Preview, Reconnect, Error, and document continuation.
- `ui/models.slint`: Presentation row definitions shared by those views.
- `ui/components.slint` and `ui/theme.slint`: Reusable controls and appearance.

Each extracted view declares its own typed inputs and callbacks. It does not
access the containing window or other pages. The shell supplies available width
and window height where responsive layout needs them. The shell retains modal
geometry and Escape routing; feature owners retain modal resource cleanup.

## Migration checkpoints

The first stage established view boundaries, feature folders, shell ownership,
shared UI dispatch, and the Settings dependency boundary. The user completed
the requested validation for that checkpoint. Core APIs, operation lifetimes,
and platform adapters remain unchanged.

This checkpoint removes cross-feature callback paths, gives each core-using
feature a restricted access object, and decodes UI action strings into typed
commands. It preserves UI behavior while making the composition boundaries
explicit. The user completed the requested validation for this checkpoint.
Review any remaining ownership or lifecycle duplication before expanding the
architecture further.

The generated window interface remains the UI boundary for Slint callbacks and
presentation properties. Do not use it as a cross-feature event bus. Keep
Handle release, cancellation, pending connections, media shutdown, and the
unified Receive feed under their existing feature owners.
`PhaseController<P>` remains limited to document preparation; it does not copy
core Pairing, Handoff, or Transfer state.

## Validation

Run `scripts/validate.ps1` manually from the repository root. It formats, checks,
lints, tests, and builds the Windows release client. Then run `scripts/run.ps1`
and check the following without requiring a second device:

- Open every page at compact and wide widths, including the Send content grid.
- Switch English/Persian, theme, and brightness. Check navigation labels and
  page content, restart, and confirm the saved values are displayed.
- Save the device name and directory; verify save feedback and picker cancellation.
- Open and close available dialogs and previews. Verify Escape, modal stacking,
  zoom, scrolling, and media cleanup where supported.
- Navigate between pages and close the application during startup and normal use.

The existing Android-to-computer connection issue still blocks paired-device
document, transfer, and pairing acceptance. Track that separately from layout and
settings checks, and do not claim two-device acceptance from CI results.
