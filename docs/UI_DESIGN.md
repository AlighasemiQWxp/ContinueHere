# Client UI design

The ContinueHere client uses a quiet, icon-led workspace that keeps the next
useful action easy to find. A compact navigation rail switches between Send,
Receive, History, and Settings. The Send page gives priority to the destination
device and a grid of icon-only content actions; link options appear when that
action is selected. Device discovery and trusted-device management stay in a
separate panel so they do not crowd the send flow.

Shared UI components own the visual rules for surfaces, icon actions, and
tooltips. Tooltips use the action's localized label, appear after a short delay
on hover or keyboard focus, and support right-to-left text. Icon actions expose
those labels to assistive technology. Important form labels, device names,
connection states, and pairing decisions remain visible. Compact layouts keep
touch targets usable.

Receive, activity, history, settings, and dialogs follow the same spacing,
typography, color, and icon rules. Their feature controllers continue to own
actions and application state; presentation components only show that state and
forward user input.

Visual acceptance covers desktop and compact widths, English and Persian,
keyboard navigation, hover and focus tooltips, touch operation, selected and
unread navigation states, and the Send, Receive, History, and Settings flows.
