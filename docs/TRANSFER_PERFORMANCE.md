# Transfer performance

This document records the source-reviewed transfer baseline and the behavior
that performance work must preserve. Source inspection identifies likely costs;
it does not establish end-to-end throughput. Measure representative transfers
before and after each transport change on the same devices and network.

## Current path

- Files are read in 32 KiB chunks. The protocol rejects larger chunks.
- The sender waits for the receiver to accept each chunk before sending the
  next one. Response polling also waits up to 25 ms before checking whether the
  response arrived, adding avoidable delay to every chunk.
- Each chunk is framed and encoded, copied into a message, written through TLS,
  decoded and copied on receipt, then written and hashed by the receiver.
- The receiver publishes a progress event for each accepted chunk. Each event
  schedules a complete transfer-list refresh in the client.
- File data is hashed as it streams. The receiver compares the final SHA-256
  digest, flushes and syncs the temporary file, then publishes it without
  replacing an existing destination.
- Cancellation is sent through the outgoing operation's command channel.
  Transport failures and timeouts resolve pending chunk requests.
- Folder contents are staged into an archive before network transfer and
  unpacked into a temporary directory after verification.
- TCP_NODELAY is enabled on authenticated streams. Activity history already
  avoids rewriting its store for progress-only transfer events.

At the current chunk limit, a 25 ms response-poll delay alone corresponds to
about 1.25 MiB/s (1.31 MB/s) in the one-chunk-at-a-time path. This is a derived
upper-bound example, not a benchmark; network round trips, hashing, disk
activity, framing, and scheduling add their own costs.

## First improvements

Wait directly for either a chunk response or an operation cancellation signal,
without polling. Coalesce progress notifications to at most ten updates per
second per transfer, while retaining the latest byte count in core state and
publishing state transitions and final progress immediately. The interval is
chosen to keep visible progress responsive while avoiding a UI refresh for
every small chunk.

Do not change chunk size or add concurrent chunks based only on this audit.
The receiver currently dispatches incoming chunks to blocking tasks, so adding
in-flight chunks first requires ordered per-transfer writes, bounded memory,
backpressure, and protocol compatibility. Assess that separately with measured
results.

## Integrity and cancellation requirements

Any optimization must retain accepted offsets and size bounds, incremental
SHA-256 verification, temporary-file publication only after verification,
no-overwrite destination handling, and cancellation on the sender and receiver.
It must also leave interrupted temporary files and retry sources in their
current recoverable states.

## Manual measurement plan

Record file size and elapsed time for a small file, a multi-gigabyte file, a
folder with many small files, and concurrent transfers. Repeat on a local
network and a higher-latency connection when available. Compare bytes per second,
CPU use, transfer-event count, UI responsiveness, cancellation latency, and
whether the received file matches its source. Report platform, device, and
network details with results; do not treat CI as a throughput benchmark.
