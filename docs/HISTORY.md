# Activity history and retry

`ActivityManager` is an independent Rust system under `CoreModules`. Its public
API is `activities()`, `retry(id)`, `remove(id)`, `clear()`, and
`on_changed(delegate)`. Private `ActivityController`, `ActivityStore`, and
`RetryController` children own recording, persistence, and retry Handle lifetimes.
The core remains independent of Flutter.

The composition root injects narrow Handoff, FileTransfer, Connection, and
trusted-peer capabilities. Typed delegates record outgoing and incoming handoffs,
file transfers, and authenticated connection changes. Source file paths remain
private core metadata on outgoing transfer snapshots and are never added to wire
messages. History observes events before UI consumers release finished Handles.
A Removed event does not erase a completed history record. Progress-only events
are ignored, so receiving a chunk does not write the history file.

An activity keeps the stable peer ID and a display-name/platform snapshot.
Connection sessions receive their own IDs; operations reference the current
session when available. A local-video handoff absorbs its related file record
and preserves the separate file-completion timestamp and playback position.
A verified incoming file can briefly appear on its own before the associated
local-video handoff arrives and groups it.

Timestamps are this installation's observed UTC Unix milliseconds. Flutter
renders their full local date, time, milliseconds, and UTC offset. A connection
end means the local Transport observed removal, whether caused by peer closure,
a local disconnect, revocation, or shutdown. It is not proof of the exact remote
socket-close time. File completion and activity completion are distinct: a
local-video file may finish before its handoff acknowledgement. Delivered means
the remote Handoff accepted the content, not that someone opened or played it.
An unfinished record restored after an unexpected exit is Interrupted, with no
invented end or disconnect time.

`history.bin` is separate from settings and trust storage. Version 1 uses the
`CHHIST01` header, bounded length-prefixed UTF-8 fields, and little-endian
integers. Reads reject unsupported headers, truncated or trailing data, duplicate
IDs, oversized payloads, invalid enum values, unreasonable timestamps, and
relative file paths. Writes use atomic replacement. Retention keeps at most
1,100 records, evicting the oldest finished entries while preserving active work;
files are bounded to 16 MiB and individual strings to 16 KiB. Corrupt startup
history produces an explicit startup error rather than being silently replaced.

Runtime write failures leave the latest bounded history in memory and expose a
storage error through the snapshot. The UI warns that recent changes may not
survive closing the application. Subsequent meaningful updates attempt persistence
again; deletion returns an error and retains records if saving fails. Delegates
run after state mutation and the save attempt, outside the history mutex.

History starts before Transport can publish connections and remains running until
Handoff, FileTransfer, and Transport have stopped. The retry controller uses a
single worker that sleeps on a bounded event channel to release finished Handles.
It does not poll or perform network work. Releasing on this worker avoids
reentering a Handle's mutex from its own synchronous event callback. Shutdown
joins the worker, releases any remaining owned Handles, and flushes history.

Retry is explicit and limited to unsuccessful outgoing content. It creates new
operation identifiers and links the new record to the original attempt, retaining
the original outcome. The controller prevents simultaneous retries of the same
record. Trust and the current connection are checked again, and existing send
owners perform capability and content validation. File retries require an
absolute regular source file with matching stored size and modification time;
this is a metadata check, not proof that the original bytes are unchanged.
The existing transfer path performs its normal integrity verification.

Files restart at byte zero and require a new receiver acceptance. Automatic
retries, offline queues, and partial-transfer resume are not included. If an
acknowledgement was lost, a manual retry may deliver content again; history does
not promise exactly-once delivery across attempts or application restarts.

The Flutter History destination first presents device cards ordered by their most
recent activity. Selecting a card opens a lazy per-device timeline with all,
files, and connection filters. Existing platform opening and file-preview
capabilities handle completed incoming content. Removing or clearing history
never deletes received files; active entries remain.

`UiNotifications` owns two independent unread sets: navigation destinations and
history device IDs. Activity outside History marks both levels. Opening History
clears its navigation indicator only. Opening a device clears only that device's
indicator. Events for the currently visible device are read immediately; events
for another device while History is open mark that device only. These unread
markers are session-local presentation state, and loading existing history at
startup creates no notifications. Clearing history also removes stale markers.

The shared Flutter Badge renders a yellow-to-golden circular gradient and a soft
glow, with a brief appearance transition respecting reduced-motion preferences.
It does not pulse continuously. English and Persian labels, directional spacing,
and explicit left-to-right timestamp runs support the existing RTL interface.

### Phase 16 validation

Binding generation, Rust formatting, workspace checks, warnings-denied Clippy,
Rust tests, Flutter formatting, analysis and tests, and the Windows release build
passed locally. Desktop interaction acceptance and continuous integration remain
pending. Run `.\scripts\validate.ps1` from the repository root to repeat the
complete local suite.

Desktop acceptance should cover two different installations: create traffic,
disconnect while viewing Transfers/Receive, observe the History navigation badge,
open History and confirm the navigation badge clears while the correct device
remains marked, then open that device and confirm its badge clears. Repeat with
two peers to verify their unread state stays independent. Check restart history,
file completion and end timestamps, missing source files, unsuccessful retry,
receiver reacceptance, clearing history, received-file opening, Persian RTL,
narrow windows, enlarged text, and reduced motion. Fresh first-pairing UI and the
remaining Phase 15 acceptance requirements continue to be tracked separately.
