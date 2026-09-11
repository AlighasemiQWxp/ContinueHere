# ContinueHere Roadmap

ContinueHere is developed in ordered phases. A phase is marked complete only
after its implementation and validation requirements are satisfied.

## A. Foundation

- [x] 01. Core architecture
  - Rust workspace and application root
  - Independent main-system ownership under `CoreModules`
  - Private same-system child-module ownership
  - Typed module access without runtime service lookup
  - Narrow typed capabilities for cross-system communication
  - Module lifecycle and registry
  - Reusable handle foundation
- [x] 02. Project quality
  - Git repository and ignore rules
  - License and development conventions
  - Formatting and linting policy
  - Architecture and GitHub documentation
  - Continuous integration after local validation
- [x] 03. Shared models
  - Stable identifiers and protocol version
  - Platform, capability, and device state models

## B. Core application

- [x] 04. Settings
  - Versioned binary `settings.bin` in the supplied project directory
  - Atomic persistence with bounded and validated section payloads
  - Typed per-system settings capabilities and delegate-based change events
  - Persistent default directory for received files
  - Temporary per-transfer destination overrides
  - Missing-file defaults and explicit malformed-file errors
- [x] 05. Localization
  - Typed English and Persian languages with stable codes
  - Code-owned translation keys and English fallback
  - Persisted language selection with delegate-based change events
  - Left-to-right and right-to-left direction support
- [x] 06. Device identity
  - Stable random identifier generated once per installation
  - Independently versioned and atomically persisted identity file
  - Typed local identity with display name and runtime platform
  - Persisted display-name changes with delegate-based events
  - Explicit separation from settings, discovery, pairing, and security
- [x] 07. Protocol and security design
  - Hostile-network threat model and explicit security boundaries
  - Separate device identity, cryptographic identity, and trust records
  - TLS 1.3 pairing and mutually authenticated transport contracts
  - Versioned deterministic CBOR messaging with bounded parsing
  - Modular ownership for discovery, pairing, security, and transport

## C. Device connection

- [x] 08. Local and manual discovery
  - Handle-owned local browsing, manual endpoints, and advertisements
  - Shared private mDNS worker with first-acquire and last-release lifecycle
  - Bounded immutable candidates and delegate-based change events
  - Temporary discovery identifiers with no stable identity or trust metadata
- [x] 09. Pairing and trusted-device management
  - Handle-owned initiating and receiving sessions
  - Private controller coordinating security, TLS, approval, and persistence
  - QR and 10-digit verification with explicit approval on both devices
  - Versioned atomic trusted-device persistence and explicit revocation
  - Immutable pairing-session and trusted-device delegate events
  - Complete local validation suite and continuous integration passed
- [x] 10. Authenticated and encrypted transport
  - Manager-owned application listener and reusable trusted-peer connections
  - Private supervisor with isolated asynchronous tasks per authenticated connection
  - Mutual TLS 1.3 authentication against pairing-owned trust records
  - Protected hello, version negotiation, capabilities, nonces, and limits
  - Deterministic bounded CBOR framing and correlated control requests
  - Immutable connection snapshots and post-commit delegate events
  - Immediate connection shutdown after trust revocation

## D. First working product

- [x] 11. URL handoff technical MVP
  - Handle-owned outgoing operations with immutable status snapshots
  - Private controller coordinating typed authenticated delivery
  - Strict HTTP and HTTPS validation with bounded protocol messages
  - Capability negotiation, acknowledgement, and duplicate protection
  - Bounded in-memory incoming records and post-commit delegate events
  - Complete local validation suite passed
- [x] 12. YouTube handoff with playback position
  - Shared Manager, Controller, Handle, lifecycle, state, limits, and events
  - Typed URL and YouTube payloads without parallel handoff systems
  - Strict official-host and video-identifier validation
  - Fixed-width millisecond playback position and canonical resume URL
  - Negotiated capability with deterministic bounded protocol messages
  - Complete local validation suite passed
- [x] 13. Streaming file transfer
  - Independent FileTransfer Manager, private Controller, Handle, and delegate event
  - Explicit incoming offer acceptance with default or temporary destination selection
  - Authenticated bounded 32 KiB chunks with sequential offset validation
  - Incremental SHA-256 integrity verification and destination-local temporary files
  - Atomic no-overwrite final commit, cancellation cleanup, and immutable progress snapshots
  - Complete local validation suite passed
- [x] 14. Local video handoff
  - Complete local validation suite passed
  - Shared Handoff lifecycle with a typed local-video payload and playback position
  - Narrow Transfer capability with event-driven progress and owned cancellation
  - Explicit file approval followed by complete-file verification and handoff acknowledgement
  - Exact completed-transfer and authenticated-sender binding on the receiver
  - Player opening and seeking remain application-interface responsibilities
- [ ] 15. Windows user interface migration
  - Native Rust/Slint UI for the implemented Windows behavior
  - Direct Rust core ownership in the application client
  - Four screens: Send, Receive, History, and Settings; transfer activity is contextual
  - Pairing, connection commands, handoffs, file dialogs, transfer approval and
    opening, image/video previews, existing history/retry, and unread indicators
  - Preserve Windows application data, English/Persian input and layouts, and
    feature-owned Handles and post-commit event subscriptions
  - Formatting, workspace checks, warnings-denied Clippy, tests, and the Windows
    release build passed locally on September 9, 2026
  - GitHub CI passed on September 9, 2026; Windows interaction acceptance remains
    pending through the Windows migration checklist
  - The subsequent approved Material redesign and transfer extensions below passed
    the complete local validation suite on September 9, 2026

## E. Subsequent work

Finish Windows interaction acceptance first. After Windows acceptance, add the
remaining features and platforms in separate steps. Each platform receives its
own implementation and acceptance cycle. Phases 17 and 18 remain deferred.

- [ ] 16. Activity history and retry
  - Independent Rust ActivityManager with bounded, versioned local history
  - Device cards opening per-device timelines for files, handoffs, and connection sessions
  - Exact locally observed start, completion, disconnection, and end timestamps
  - Interrupted recovery with unknown end times after an unexpected exit
  - Explicit retry through existing Handoff and FileTransfer Handles
  - Separate History navigation and device-card unread indicators
  - Golden circular badges, English/Persian labels, and responsive RTL layouts
  - Native-client baseline local validation, Windows release build, and CI passed
  - Current revision local validation and GitHub CI passed; desktop interaction
    acceptance pending

- [ ] Windows Material redesign and two-computer transfer acceptance
  - Official Navigation Drawer/Bar, clickable buttons, SnackBar, directory action,
    language/theme radio buttons, persisted appearance and brightness slider
  - Responsive Send/Receive activity and previews; supplied application icon
  - LAN address and port display, pairing advertisements, authenticated History reconnect
  - Automatic pairing readiness, terminal-state receive re-arming, and normalized
    manual/visible endpoint deduplication
  - Verified connection-endpoint exchange and automatic authenticated connection
    after pairing, with cached one-click reconnect
  - Task-first desktop layout with standard-sized actions, compact content tiles,
    horizontally grouped secondary controls, and troubleshooting details kept
    visually secondary
  - Category-filtered native pickers and bounded folder transfer through FileTransfer
  - YouTube link plus entered position; local-video handoff from the in-app player
  - The current connection workflow and simplified presentation passed formatting,
    checks, linting, tests, and the release build on September 11, 2026; pairing
    passed manual two-computer acceptance, while visual and transfer acceptance
    remain pending
  - Automatic browser playback capture, streaming while downloading, partial resume,
    offline queues, installers, and additional platforms remain future work
- [ ] 17. Android application
- [ ] 18. Linux, macOS, and iOS support, one platform at a time
- [ ] 19. Advanced application and operating-system integrations

## F. Production

- [ ] 20. Security, interruption, performance, memory, and battery hardening
- [ ] 21. Branding, installers, signing, updates, privacy documentation, and version 1.0
