# Client UI design

ContinueHere keeps the main flow explicit: choose a device, choose content, send,
then open or continue it on the other device. The client uses Rust controllers
and Slint components, with core state and operation lifetimes unchanged.

## First use and devices

An empty Send screen explains the app and offers Add a device. Setup asks users
to open ContinueHere on both devices on the same local network, select a nearby
endpoint, compare verification codes, and approve on both devices. Discovery has
no friendly-name field; a nearby entry uses a neutral label and address until
pairing supplies its name. Discovery metadata never establishes trust.

Trusted but disconnected devices show Connect and Change connection address.
Manual pairing is under Can't find your device, with directions to the other
app's Receive connection details. Forgetting a device requires confirmation.
No extra onboarding preference or duplicate device state is stored.

## Sending and continuing

The main content choices are File, Folder, Link, and Continue activity. Icons
have visible labels. File can send an ordinary PDF or presentation without a
continuation point; contextual image, media, and video picker filters remain
available. Link sends an ordinary URL without asking for a playback time.

Continue activity groups video preview, entered-time video and YouTube sharing,
and PDF page or PowerPoint slide preparation. The interface states that progress
in other apps is not read automatically. Playback input accepts seconds, MM:SS,
or HH:MM:SS; an empty value starts at the beginning. The video preview shows the
current time and destination, and the document dialog shows the intended device
before submission. Existing validation preserves an invalid document draft.

## Receiving and history

Receive shows the device name, network/listener availability, and default save
folder. Incoming offers show the save folder before acceptance, with a separate
folder override. Connection addresses, IDs, ports, and pairing diagnostics are
expandable. A ready heading describes local state, not proven peer reachability.

Active transfers show progress and cancellation. Finished activity uses one
shared card with device name, status, Open, Retry, and expandable details.
Disconnected retryable activity offers Reconnect. History starts with the latest
50 content items; By device retains full timelines and connection sessions.
Send and Receive show up to five finished items with a View history action;
older activity is retained in the core store, not deleted by this display limit.
Clearing finished history requires confirmation and preserves received files,
paired devices, and active operations.

## Visual rules and acceptance

Desktop navigation and primary content actions retain visible labels. Tooltips
provide labels for secondary icon controls. Shared surfaces, spacing, theme
colors, and focus behavior remain consistent. Settings prioritizes device name,
received-file location, and language before appearance.

Manual acceptance must cover fresh and returning users, desktop and compact
widths, long names, English and Persian, keyboard navigation, touch, pairing,
reconnection, every content type, continuation, progress, cancellation, retries,
and both destructive confirmations. Check that completed transfers appear once
and that save paths and selected destinations match the actual operation.
Source inspection and Cargo checks do not establish visual or two-device acceptance.
