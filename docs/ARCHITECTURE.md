# ContinueHere Architecture

## Overview

ContinueHere uses a modular Rust workspace. The application composition root
owns the main systems, constructs their dependencies, and controls their
lifecycle. Every system has one main module. Each main module privately owns,
constructs, and controls the child modules that belong to that system. Feature
code is added incrementally without coupling unrelated systems together.

## Workspace boundaries

- `crates/continuehere` contains the reusable core application library.
- `apps/desktop` is reserved for the desktop application.
- `apps/mobile` is reserved for future mobile applications.
- `docs` records stable architectural decisions and the development roadmap.

Application crates may depend on the core library. The core library must not
depend on an application or user-interface implementation.

## Application root

`ContinueHereBuilder` constructs and starts a `ContinueHere` instance.
`ContinueHere` owns `CoreModules`, the private project-wide module owner.
`CoreModules` registers every main system as a strongly typed field and owns the
optional dynamic module registry.

Known main systems and child modules use typed fields instead of runtime lookup.
The dynamic registry is reserved for genuinely optional or replaceable modules
and is never used as a general service locator on normal hot paths.

The current hierarchy is:

```text
ContinueHere
└── CoreModules
    ├── SettingsManager
    ├── DirectoryManager
    ├── LocalizationManager
    ├── DeviceManager
    ├── DiscoveryManager
    └── ModuleRegistry
```

Every independent system owns its own main module directly under `CoreModules`.
A main module may privately own child modules that are part of the same system,
but it must not own another independent main system. In particular,
`SettingsManager` does not own systems merely because they have configurable
settings.

## Settings communication

Systems communicate with `SettingsManager` through narrow, typed settings
capabilities. A capability exposes only the read, write, and change-event
operations needed for one system. The system's main module and the future
Settings user interface use the same capability, so settings have one path and
one authoritative owner.

`DirectorySettings` is the first concrete capability. `SettingsManager` owns it
and injects the same underlying capability into `DirectoryManager`. It stores
the default destination directory for received files. A destination selected
for one transfer is a temporary override resolved by `DirectoryManager`; it
does not modify the saved default.

The directory system defines its own `DirectoryChangedDelegate` and exposes the
`on_directory_changed` event through both `DirectorySettings` and
`DirectoryManager`. The event fires only after a changed default directory has
been persisted successfully. A temporary per-transfer override does not change
shared state and therefore does not fire the event. Dropping the returned
`DirectoryChangedSubscription` unregisters that listener.

The Localization system uses its own localization-specific settings capability.
`CoreModules` constructs the main systems and injects that capability without
making either main system own the other.

Settings capabilities are introduced only when a system has real settings to
expose. Empty gateways are not created speculatively.

## Localization

`LocalizationManager` is the independent main module for localized text and
text direction. `SettingsManager` owns the typed `LocalizationSettings`
capability and injects the same underlying capability into
`LocalizationManager`. English is the default language, and selecting Persian
is persisted in the `localization` section of `settings.bin`.

Languages and translation keys are typed values. Translation catalogs are
defined in Rust code and remain private implementation details. A lookup first
uses the active language and falls back to the required English text when that
translation is unavailable. Translation keys are added with the feature that
owns the actual user-facing text instead of being created speculatively.

Text direction is derived from the active language: English is left-to-right
and Persian is right-to-left. It is not stored as a separate preference.

The localization system defines its own `LanguageChangedDelegate` and exposes
the `on_language_changed` event through both `LocalizationSettings` and
`LocalizationManager`. The event fires only after a changed language has been
persisted successfully. Selecting the active language again does nothing.
Dropping the returned `LanguageChangedSubscription` unregisters that listener.

## Settings persistence

`SettingsManager` owns the only persistent settings store. The application
supplies its project directory during construction, and the store uses the
fixed path `<project directory>/settings.bin`.

The file has a versioned binary envelope containing independently versioned
system sections. Section identifiers and payload lengths allow newer unknown
sections to be preserved without exposing a string-based settings API. Known
sections are decoded and validated through their typed settings capabilities.

The version-1 envelope uses the `CHSETBIN` magic bytes, a little-endian root
version and section count, then a sequence of UTF-8 section identifiers,
little-endian section versions, payload lengths, and opaque payload bytes. The
version-1 `directories` payload is the UTF-8 representation of an absolute
default transfer directory. The version-1 `localization` payload is one stable
language byte. The complete file and every section are bounded before
allocation or decoding.

Writes replace the complete small settings document atomically. Runtime state
updates and system-owned delegates run only after the new file has committed
successfully, so a failed write leaves the old file and active setting intact
and does not publish a false change.

The settings file contains preferences only. Device identity, trusted-device
credentials, security keys, transfer history, and temporary per-transfer paths
remain with their owning systems.

Native file-picker integration remains an application-interface responsibility.
The picker supplies an absolute directory to `DirectoryManager`, which resolves
that one-transfer selection or falls back to the persisted default.

## Device identity

`DeviceManager` is the independent main module for the local device system. It
privately owns the current `LocalDeviceIdentity`, its persistence store, and its
change event. The identity contains a stable random `DeviceId`, a user-facing
display name, and the platform detected for the current run.

The permanent identifier is generated once with the operating system's secure
random source and remains unchanged across normal restarts, display-name
changes, and network changes. It is an opaque identifier, not a secret or proof
of trust. Hardware addresses, IP addresses, and hostnames are not used as the
permanent identifier.

Identity data is stored in the independently versioned
`<project directory>/device_identity.bin` file. A missing file creates and
atomically commits a new identity before device-system startup succeeds. An
existing file is bounded and validated before use. Malformed identity data is
preserved and reported instead of being silently replaced with a new identity.

The initial display name uses a valid environment-provided system name when
available and otherwise uses a platform-specific fallback. Display-name changes
commit to disk before runtime state changes or the system-owned
`DeviceIdentityChangedDelegate` event is published. Selecting the current name
does nothing.

Discovery deliberately does not broadcast the local identity. It supplies only
temporary candidate endpoints and a protocol-version hint. Pairing and
security remain responsible for learning and proving a peer's stable identity.

## Discovery

`DiscoveryManager` is the independent main module for finding possible peers.
It owns reusable `DiscoveryHandle` instances, a private worker, the current
candidate store, and discovery-specific events. Callers follow the shared
handle lifecycle:

```text
Get handle -> Configure mode -> Use -> Release
```

A handle can browse the local network, add one manually entered endpoint, or
temporarily advertise a listening endpoint. Starting a local-browse handle
acquires the shared mDNS browser. Additional local-browse handles reuse it, and
the final release stops it and removes local candidates. A manual candidate is
present only for the lifetime of its handle. Advertisements use a fresh random
instance identifier for each operation.

The private worker is the only owner of mDNS activity. It receives handle
commands through a channel, translates backend events into immutable bounded
candidate snapshots, and publishes `DiscoveryChangedDelegate` and
`DiscoveryStatusChangedDelegate` events. Public methods never expose the mDNS
backend or mutable candidate state. The worker commits operation, candidate,
and status state before publishing candidate events, so delegates always
observe a coherent snapshot when they query the manager.

Local discovery uses `_continuehere._tcp.local.` and advertises only the current
protocol version. It does not advertise a stable `DeviceId`, display name,
capabilities, trust state, or credentials. Manual endpoints and discovered
endpoints are syntax-checked and bounded, but remain untrusted hints until a
future authenticated transport verifies the peer.

## Protocol and security

The protocol and security contracts are defined in
[`PROTOCOL_SECURITY.md`](PROTOCOL_SECURITY.md). The network is treated as
hostile: discovery information is an untrusted hint, a `DeviceId` is not a
credential, and all application messages must eventually travel through a
mutually authenticated TLS 1.3 channel.

Cryptographic identity remains separate from the public local-device identity.
Private-key material belongs in a platform secure-storage backend, trusted-peer
records belong to the pairing system, and neither belongs in `settings.bin` or
`device_identity.bin`.

`DiscoveryManager` is an independent main system under `CoreModules`. Future
`PairingManager`, `SecurityManager`, and `TransportManager` modules will follow
the same ownership rule. They will exchange narrow typed capabilities and
system-specific events. Transport will expose validated typed messages rather
than raw sockets or decoded protocol values.

The application protocol uses protected version negotiation, deterministic
CBOR control messages, bounded length-prefixed framing, typed request
identifiers, and separate streaming messages for large content. Discovery,
pairing, transport, and transfer phases must define and test their concrete
limits before accepting network input.

## Module lifecycle

Main systems and dynamic modules implement a shared lifecycle contract:

1. Project main systems start in registration order.
2. A main system starts its child modules in registration order.
3. If startup fails, previously started modules stop in reverse order.
4. Normal shutdown stops every started module in reverse order.
5. A main system stops its own children before an earlier main system stops.
6. Shutdown continues after an error and returns the first failure.

This gives resources a predictable dependency order and keeps partial startup
from leaving active modules behind.

## Handle ownership

The handle foundation manages reusable, stateful operations through an explicit
lifecycle:

```text
Get handle -> Configure -> Use -> Release
```

`BaseHandle` stores shared lifecycle state. `BaseHandleProvider` owns and reuses
handles by identifier. Specialized handles compose these foundations and expose
only the capabilities required by their feature.

Handles are reference-based ownership tokens. Releasing a handle invalidates
its operation and allows the provider to remove it safely.

## Feature modules

Feature folders currently define architectural boundaries for activities,
discovery, pairing, transport, security, transfer, handoff, settings,
localization, managers, controllers, backends, models, and utilities.

Each feature should be implemented independently and should communicate through
small typed APIs or events. Every independent system has one main module under
`CoreModules`. A main module owns only child modules from its own system.
Controllers coordinate multi-step behavior; managers own stable feature APIs;
backends isolate infrastructure and platform code.

## Shared models

Shared models are small, immutable value types used across feature boundaries.
They define stable device identifiers, protocol versions, platforms,
capabilities, and device snapshots without owning feature behavior.

The models do not discover devices, generate identities, persist data,
serialize protocol messages, or publish state-change events. Those
responsibilities remain with their owning roadmap phases. Model fields stay
private and are exposed through narrow constructors and read-only methods.

Shared identifiers use dedicated types instead of plain strings so unrelated
identifiers cannot be mixed accidentally. Capability collections prevent
duplicates while preserving a simple representation suited to the small number
of capabilities expected per device.

## Visibility and dependencies

- Keep types private unless another module or crate must use them.
- Expose capabilities instead of concrete implementation details.
- Inject dependencies through constructors.
- Avoid global mutable state and general-purpose service lookup.
- Add a dependency only when the standard library or an existing dependency is insufficient.
- Keep cryptographic choices behind narrow backends and use reviewed protocol
  implementations instead of custom cryptographic primitives.

## Validation

Architecture changes should include focused tests for public behavior, failure
paths, state transitions, and cleanup ordering. Performance-sensitive designs
must be benchmarked before being described as optimized.
