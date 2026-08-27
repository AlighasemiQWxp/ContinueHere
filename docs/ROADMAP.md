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
- [ ] 07. Protocol and security design

## C. Device connection

- [ ] 08. Local and manual discovery
- [ ] 09. Pairing and trusted-device management
- [ ] 10. Authenticated and encrypted transport

## D. First working product

- [ ] 11. URL handoff technical MVP
- [ ] 12. YouTube handoff with playback position
- [ ] 13. Streaming file transfer
- [ ] 14. Local video handoff
- [ ] 15. Desktop user interface

## E. Expansion

- [ ] 16. Activity history and retry
- [ ] 17. Android application
- [ ] 18. Linux, macOS, and iOS support
- [ ] 19. Advanced application and operating-system integrations

## F. Production

- [ ] 20. Security, interruption, performance, memory, and battery hardening
- [ ] 21. Branding, installers, signing, updates, privacy documentation, and version 1.0
