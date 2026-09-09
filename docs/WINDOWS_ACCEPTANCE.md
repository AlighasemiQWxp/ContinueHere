# Windows client acceptance

## Status and preparation

The native-client baseline passed local validation and CI on September 9, 2026.
The current Material redesign, folder protocol, endpoint cache, appearance settings,
and supplied application icon passed the complete local validation suite on the same
date. Every interaction and two-computer item below remains pending; automated
validation does not establish interaction acceptance.

Install the Windows prerequisites in [client setup](../apps/client/README.md),
then run manually from the repository root on the development computer:

```powershell
.\scripts\validate.ps1
.\scripts\run.ps1
```

Validation formats, checks, lints, tests, and builds the release executable. Use
the same updated revision on both computers. Each needs Rust/build dependencies
when building from source and the documented GStreamer runtime to run. A standalone
installer or media-runtime bundle is not supplied. Older builds do not understand
the new folder capability and cannot be used as the second peer for this checklist.

Use two computers on the same reachable Wi-Fi/Ethernet network, with distinct
Windows profiles/application identities. Allow ContinueHere through Windows Firewall
for the private network if prompted. Guest-network isolation, VPN routing, or a
blocked inbound listener can prevent connection despite a shared network name.

## Pair and connect

1. On computer B, open Receive. Note its LAN IPv4 address, Pairing listener port,
   and Connection listener port. Multiple adapters may show multiple addresses;
   choose the one on the shared network. Loopback and wildcard addresses are excluded.
2. Choose Receive pairing on B. On A, open Send and choose Pair on B's nearby
   endpoint. If discovery is unavailable, enter B's `IP:pairing-port` under Manual
   endpoint, choose Add, then Pair on that candidate.
3. Compare the verification codes and approve on both computers. Verify reject,
   cancel, and timeout paths in separate attempts. A candidate is not trusted until
   verification completes.
4. On A, enter B's `IP:connection-port` in Destination connection endpoint, then
   Connect on B's trusted-device card. Select B as the destination for sending.
5. Disconnect. In History on A, choose Reconnect for B. Verify the saved endpoint
   is filled in and the authenticated connection can be restored. Restart B and
   update its port in the dialog if it changed. An incoming-only peer may have no
   saved endpoint. Forgetting a peer requires pairing it again before reconnecting.
6. Leave a healthy connection idle while previewing a video for over two minutes;
   it should remain connected. Disconnect intentionally and verify no reconnection
   is attempted automatically.

Addresses refresh every five seconds or through Refresh addresses. The client
advertises pairing endpoints while running; Receive pairing still explicitly
enables an approval session. Discovery identifiers remain temporary untrusted hints.

## Send, receive, and continue playback

| Content | Procedure | Expected result |
| --- | --- | --- |
| File | Choose File on A, select a safe text/PDF file, accept on B | Progress in Send/Receive; open only after verification; bytes match |
| Folder | Select a folder with nested files, a non-ASCII name, and an empty subfolder | One folder offer; all content and empty directories preserved; no archive-opening step |
| Image | Choose Image and select PNG/JPEG/GIF/WebP | Picker filter shows supported image extensions; received images open with zoom/scroll; animated formats animate |
| Video | Choose Video and select MP4/MKV | Picker filters supported video extensions; ordinary transfer opens from the start |
| Media | Choose Media, inspect video/image/audio choices, then send a supported audio file | Media category filter applies; safe content can open through the existing preview/platform route |
| URL | Enter a valid HTTPS URL and send | Receive opens the link explicitly; unsafe schemes and malformed input are refused |
| YouTube | Copy a YouTube URL, enter `1:23` as playback position, choose the YouTube action | Receive opens the canonical video link at approximately 83 seconds; browser/account/player rules may affect playback |
| Local video continuation | Choose Preview local video, play/seek to a known position, select B, choose Continue on device | B approves the file; after complete delivery its incoming handoff opens the video near the captured position |
| Entered local position | Set the playback field, then Send video at entered time | Video picker opens; accepted video opens at the entered position on B |

YouTube sends a URL plus position, not downloaded YouTube bytes. Automatic capture
of another browser's playback position is deferred. Local continuation captures
the position when Continue on device is clicked; playback on A can continue during
the transfer. Check a decodable file with duration longer than the chosen position.
Resume is subject to codec/keyframe seeking, not a guarantee of frame-exact playback.
No content plays while still downloading, and files must fully verify first.

After completion, inspect recent activity and Open actions directly in Send and
Receive, then the peer timeline in History. Reopen received local-video activity
after restarting the app and verify its saved position. Test sending in both
directions and more than one transfer while keeping the owning app running.

## Failure and persistence checks

- Cancel every native picker: no send starts and the previous setting is retained.
- Reject an offer; accept into the default Save Directory and a one-off directory.
  Neither case should unexpectedly change the saved default.
- Cancel during file transfer, folder preparation, and folder extraction. Disconnect
  during an offer and during transfer. Active rows must stop and incomplete owned
  files should be cleaned up; completed content must remain.
- Use an existing destination name. Confirm file/folder contents are never overwritten.
  Test a read-only or full destination, a missing source, and an unsupported folder
  containing a junction/symlink. Report the failure without claiming completion.
- Retry unsuccessful outgoing activity from History after reconnecting. A new offer
  is required. Files restart at zero. Folder retries enumerate the current folder;
  they do not restore its old contents or preserve timestamps/permissions.
- Test ordinary file hash equality on both machines. For folders, compare relative
  paths, empty directories, file sizes, and hashes. Folder limits are 4096 entries,
  32 segments, 1024 UTF-8 path bytes, and 100 GiB including the package headers.
  Keep enough temporary space on A and enough space for package plus extraction on B.
- Force-close during active work using test data. History should restore the activity
  as interrupted without an invented end time. Abrupt process/power loss may leave
  staging or partial folder content; crash recovery is not yet an acceptance claim.
- Clear/remove history: received files, trusted devices, endpoint hints, and active
  work remain. Inspect exact timestamps, device grouping, retry links, and unread
  indicators. Opening History and opening an individual timeline clear their own
  respective indicators.
- Open safe files/URLs, missing files, corrupt media, unsupported codecs, and files
  without a Windows association. Executable/script opening must remain refused.

## Settings, responsive layout, and icon

| Area | Manual acceptance |
| --- | --- |
| Navigation | Exactly Send, Receive, History, Settings. At 960 logical pixels and above use the Material Navigation Drawer with its actions aligned directly below the application header; below that use the Navigation Bar. The bottom bar must remain fully visible and unobstructed while content scrolls. No separate Devices/Transfers destinations. |
| Application header | The supplied application icon and ContinueHere name are vertically centered, evenly spaced, and remain visually aligned in both left-to-right and right-to-left layouts. |
| Buttons | Mouse click, Tab/Shift+Tab, Enter, and Space activate actions once. Actions must not stay selected like navigation items. Verify disabled controls cannot act. |
| Choose Content | File, Folder, Media, Image, and Video appear as a responsive grid: two columns in narrow content and three columns when space permits. Every category opens the correct native picker. |
| Device name | Save a valid name; see “Device name saved as X.” in SnackBar. Try an invalid name: no success notice. Restart to confirm the saved value and stable identity. |
| Save Directory | Title, current path, and Floating Action Button are visible; choose/cancel the native directory dialog and verify persistence. |
| Theme Style | Purple, Red, Green radio choices recolor the Material surfaces and controls immediately. Restart to confirm selection. |
| Language | English/Persian radio choices persist, right-to-left layout and input are usable, and mixed-direction paths remain legible. |
| Brightness | Slider adjusts app rendering between 50 and 100 percent; mouse and keyboard steps work. Restart to confirm. OS brightness/dialogs remain unchanged. |
| Resolution | Resize through 360×360, 600×800, 959×600, 960×600, and 1440×900 logical pixels at Windows scaling 100/150/200 percent. All actions remain reachable by vertical scrolling; the primary pages must not create a horizontal scrollbar, and long paths/names plus preview/reconnect controls remain usable. |
| Visual theme | Purple, Red, and Green each retain readable contrast while applying a cohesive background gradient, subtle color highlights, elevated cards, and consistent control spacing. Decorative styling must never block input or obscure content. |
| Preview | Play/pause, seek, volume, zoom, animated images, close/reopen, and sending current position work. Closing stops sound and releases files. Programmatic playback progress does not repeatedly seek. |
| Icon | Supplied artwork appears in the title bar/window switcher/taskbar and built executable in Explorer. Verify at small/large icon sizes after rebuilding; existing pinned shortcuts may retain cached icons. |
| Startup/accessibility | Test slow/failed startup, close during startup/connection, keyboard focus, screen reader, and Windows reduced-animation preference. |

## Delivery gate

Record the command results and any failing interaction steps. After manual
validation is confirmed, review the full diff/lockfile for secrets, generated
outputs, unrelated changes, and unwanted references; commit on main, push without
rewriting history, verify matching local/remote hashes, and confirm CI. CI and
compilation do not substitute for the two-computer tests above.
