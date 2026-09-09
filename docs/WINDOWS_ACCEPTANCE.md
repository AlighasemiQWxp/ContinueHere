# Windows client acceptance

## Scope and status

Validate the native Rust/Slint Windows client and the behavior implemented through
Phase 16. The reusable Rust core owns protocols, persistence formats, identity,
trust, activity history, and retry.

The implementation passed formatting, workspace checks, warnings-denied Clippy,
tests, and the Windows release build locally on September 9, 2026. GitHub CI also
passed its Linux Rust and Windows client jobs. It is not
accepted as complete until the interaction checklist passes. No new platform is
a prerequisite for finishing the Windows migration. Android, Linux, macOS, iOS,
and remaining product features follow separately.

## Parity checklist

Every acceptance item below remains pending manual Windows verification.

| Area | Implemented migration | Manual acceptance |
| --- | --- | --- |
| Startup | Responsive loading/failure view, background core construction, orderly shutdown even when closed during startup | Start normally; test an invalid data location; close during startup and during connection |
| Shell | Devices, Send, Transfers, History, Settings; dark Material controls, narrow bottom navigation, wide side navigation, page fade, golden unread indicators | Inspect narrow, medium, and wide widths at 100/150/200 percent scaling |
| Localization | Existing English/Persian labels and persisted selection, mirrored navigation and right-aligned Persian content/input | Switch language on every screen; type/select/edit Persian and mixed-direction text; inspect keyboard focus and labels |
| Devices | Local/manual discovery, pairing receiver/initiator, verification code, approve/reject/cancel, trust removal, connect/disconnect | Pair two Windows instances with separate data stores; test both approvals, rejection, cancellation, timeout, wrong peer, disconnect, and forget |
| Send | Connected-device selection, URL, YouTube with position, local video, file selection, incoming open/remove | Send valid and invalid content; cancel every file dialog; disconnect before sending |
| Transfers | Offers, accept/default folder, alternate folder, reject, cancel/remove, progress, verified-file opening | Complete a transfer; reject/cancel; simulate disconnection; test destination conflicts and missing received files |
| Preview | Images with zoom/scroll, animated GIF/WebP, in-window video/audio, pause/play, seek, volume, local-video resume | Exercise supported formats and codecs, corrupt files, replay at end, repeated open/close, closing during load; confirm audio stops and files release |
| External opening | HTTP/HTTPS URLs and associated applications for safe files; retained executable/script refusal | Open PDF/text; verify blocked script/executable extensions; test missing associations |
| History | Per-device cards and timelines, exact timestamps with milliseconds/offset, failure/interruption state, clear/remove/retry, received-content opening | Verify sessions and transfers, restart persistence, unknown end after unexpected exit, reconnect/retry, clear preserving active work and received files |
| Badges | Page indicators and separate history-device unread state | New activity on another page; open History; open one device; new activity while viewing its timeline |
| Settings | Device name, native destination folder picker, English/Persian persistence | Change each setting, cancel folder selection, restart and confirm values and device identity |
| Accessibility | Standard control keyboard semantics; application page transitions follow the Windows animation preference | Tab/Shift+Tab, Enter, Space, screen reader, focus while preview is open, reduced animations |

## Two-device connection details

Pairing and authenticated application connections use different ports. Devices
shows both local listener endpoints. When a listener shows `0.0.0.0`, use that
machine's actual LAN address on the other machine, with the displayed port.

1. On the receiving machine, choose Receive pairing.
2. On the initiating machine, add the receiver's LAN address and **pairing**
   port as a manual endpoint, then choose Pair on that candidate.
3. Compare the code on both machines and approve on both.
4. Enter the other machine's LAN address and **connection** port in Destination
   connection endpoint, then choose Connect on its trusted-device card.
5. Choose that connected device in Send.

Discovery IDs are temporary hints and are not trusted device IDs. The client
therefore uses explicit connection endpoints. Core TLS authentication still
validates the selected trusted peer.
Automatic endpoint-to-trust association and additional discovery behavior remain
separate future work.

## Validation and delivery

Install the Windows prerequisites documented in `apps/client/README.md`,
then run from the repository root:

```powershell
.\scripts\validate.ps1
.\scripts\run.ps1
```

Use separate machines or Windows user profiles for paired-device testing so each
instance has its own identity and application-data directory.

The validation script formats, checks, lints, tests, and builds locally. The
Windows CI also installs the native media dependencies and checks the locked
workspace. Linux CI remains a source/core portability check. Neither counts as
acceptance for a Linux application.

After manual validation is confirmed, review the full diff and updated lockfile,
commit on main, push without rewriting history, verify matching local/remote
hashes, and wait for CI. Packaging media dependencies in a standalone installer
follows later.
