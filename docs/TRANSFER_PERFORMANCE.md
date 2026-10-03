# Transfer performance

This document records the source-reviewed transfer path. Source inspection does
not establish end-to-end throughput. Measure representative transfers before
and after transport changes on the same devices and network.

## Current path

Files are read in 32 KiB chunks, and the protocol rejects larger chunks. The
sender waits for each chunk response before sending the next one. Response and
cancellation signals are selected directly; the old 25 ms polling delay has
been removed.

Each chunk is framed, encoded, copied into a message, written through TLS,
decoded, then written and hashed by the receiver. Progress notifications are
coalesced to roughly ten updates per second per transfer, with state transitions
and final progress published immediately. The latest byte count remains in core
state. The client still rebuilds transfer presentation snapshots on notifications.

SHA-256 verification, flush and sync, and no-overwrite publication remain in the
receive path. Folder contents are packaged before network transfer and extracted
after verification. TCP_NODELAY is enabled, and Activity history avoids rewriting
its store for progress-only events. Send and Receive show active transfer rows;
finished content appears through persistent Activity records.

## Next measurements

Measure transfer throughput, UI refresh cost, memory, and cancellation latency
before changing the protocol or introducing another scheduling abstraction.
Determine whether multiple events queue redundant UI refreshes or unchanged
models are repeatedly replaced. If measurements justify it, coalesce pending
refreshes within the existing feature controller while keeping terminal updates
and operation ownership intact.

Larger or concurrent chunks require ordered per-transfer writes, bounded memory,
backpressure, and compatibility review. The receiver dispatches incoming chunks
to blocking tasks, so concurrency must not be added without those guarantees.

## Integrity and cancellation requirements

Preserve accepted offsets and size bounds, incremental SHA-256 verification,
publication only after verification, destination conflict handling, sender and
receiver cancellation, and current retry-source retention rules.

## Manual measurement plan

Record size and elapsed time for a small file, a multi-gigabyte file, a folder
with many small files, and concurrent transfers. Repeat on the same local network
and a higher-latency connection when available. Compare bytes per second, CPU,
transfer-event count, UI responsiveness, cancellation latency, and received-file
integrity. Record platform, device, and network details. CI is not a throughput
benchmark.
