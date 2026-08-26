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
    ├── LocalizationManager
    ├── DeviceManager
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

For example, the Localization system will use a localization-specific settings
capability when its persisted preferences are implemented. `CoreModules`
constructs the main systems and injects that capability without making either
main system own the other.

Settings capabilities are introduced only when a system has real settings to
expose. Empty gateways are not created speculatively.

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

## Validation

Architecture changes should include focused tests for public behavior, failure
paths, state transitions, and cleanup ordering. Performance-sensitive designs
must be benchmarked before being described as optimized.
