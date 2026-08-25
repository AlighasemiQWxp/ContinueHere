# ContinueHere Architecture

## Overview

ContinueHere uses a modular Rust workspace. The application composition root
owns the main systems, constructs their dependencies, and controls their
lifecycle. Feature code is added incrementally without coupling unrelated
systems together.

## Workspace boundaries

- `crates/continuehere` contains the reusable core application library.
- `apps/desktop` is reserved for the desktop application.
- `apps/mobile` is reserved for future mobile applications.
- `docs` records stable architectural decisions and the development roadmap.

Application crates may depend on the core library. The core library must not
depend on an application or user-interface implementation.

## Application root

`ContinueHereBuilder` constructs a `ContinueHere` instance. `ContinueHere` owns:

- `CoreModules`, which stores frequently used, strongly typed managers
- `ModuleRegistry`, which stores optional lifecycle-managed modules

Core managers are accessed through narrow typed methods. The registry is not a
general service locator and is not used for normal hot-path module access.

## Module lifecycle

Optional modules implement a shared lifecycle contract:

1. Modules start in registration order.
2. If startup fails, previously started modules stop in reverse order.
3. Normal shutdown stops every started module in reverse order.
4. Shutdown continues after an error and returns the first failure.

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
small typed APIs or events. Controllers coordinate multi-step behavior; managers
own stable feature APIs; backends isolate infrastructure and platform code.

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
