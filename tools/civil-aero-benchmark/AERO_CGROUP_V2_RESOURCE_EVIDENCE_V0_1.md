# Linux cgroup v2 resource evidence v0.1

## Purpose and boundary

The initial resource-telemetry tranche parses two receipt-bound Linux cgroup v2 snapshots and derives an operation-scoped resource vector for:
- elapsed monotonic wall time;
- CPU usage, user and system microseconds;
- exact rational CPU core-hours;
- peak memory bytes;
- OOM and OOM-kill event deltas.

The evidence is scoped to one cgroup process tree, not to a single PID. The caller must create a dedicated cgroup for each solver operation and place the complete solver process tree within it. A parent-only process timing fallback is a different scope and cannot be silently promoted to cgroup process-tree evidence.

The current Rust module is a parser/deriver for already-captured cgroup files. It does not yet create cgroups, move processes, own file descriptors, sample live counters or attach the resource report automatically to the external-solver receipt. The adapter/runtime integration remains required before it can produce measured production receipts.

## Snapshot inputs

A snapshot parses:
- `cpu.stat`: `usage_usec`, `user_usec`, `system_usec`;
- `memory.peak`;
- `memory.events`: `oom`, `oom_kill`, and optional `oom_group_kill`;
- `cgroup.events`: `populated`, `frozen`;
- stable operation cgroup identity and monotonic timestamp;
- stable memory.peak reader identity;
- the exact cgroup2 mount-options string.

The parser hashes each raw counter file and binds those digests into a canonical snapshot digest. Unknown future key/value fields may be retained in the raw-file digest; duplicate keys and malformed required fields reject.

## Process-tree and hierarchy semantics

Linux cgroup v2 CPU statistics report accounting for the cgroup and its descendants. The cgroup must be operation-exclusive: unrelated workload must not share it. The adapter must ensure descendant processes are included and that the cgroup is not finalized until `cgroup.events populated=0`.

The kernel's `memory.events` is hierarchical by default. The `memory_localevents` mount option changes it to local-only accounting; the v0.1 parser rejects that option because it would weaken descendant attribution. The mount-options digest is bound into both snapshots.

Relevant kernel documentation:
- cgroup v2: https://docs.kernel.org/admin-guide/cgroup-v2.html

## memory.peak file-descriptor semantics

The kernel documents that `memory.peak` is scoped to a file descriptor and can be reset by writing to that file descriptor. Therefore:
- open the file read-only before the baseline observation;
- retain the same FD for the final observation;
- bind the same reader identity in both snapshots;
- reject snapshots whose reader identities differ;
- never use a write-capable FD or reset the peak counter during an operation.

This v0.1 parser rejects a lower final peak than baseline as a possible reset/regression. Same-FD, read-only capture is a required adapter property and still needs live integration tests.

## Derivation and fail-closed rules

The report is rejected when:
- cgroup identities differ;
- the baseline or final cgroup is populated;
- the cgroup is frozen at either boundary;
- monotonic timestamps are equal or regress;
- CPU or OOM counters regress/reset;
- memory.peak regresses/reset;
- memory.peak reader identity changes;
- cgroup2 mount-option scope changes;
- the oom_group_kill capability appears/disappears between observations.

CPU core-hours are preserved as an exact rational quantity:

`cpu_core_hours = cpu_usage_usec / 3,600,000,000`

No float rounding is required in the evidence. The numerator and denominator are both exported.

OOM and OOM-kill deltas are observations. They do not replace the process exit status or classify the solver's scientific result.

## What remains Unknown

The module does not infer:
- per-process or per-operation CPU package energy;
- host or device energy attribution;
- GPU hours, GPU utilization or GPU memory;
- power from utilization percentage;
- a resource cost when the cgroup/controller is unavailable;
- cost for an unmeasured child process that escaped the cgroup.

Host/device counters are not per-process energy. On shared hardware, energy remains Unknown unless an exclusive measurement domain or separately qualified attribution method exists.

GPU capture requires a separate capability-gated NVML adapter. Unsupported accounting mode, MIG/vGPU limitations, PID reuse, overwritten accounting buffers and driver reloads must remain visible and may result in Unknown.

## Independent replay

The module exports normalized CSV rows for start/end snapshots and the derived report. The Python standard-library oracle independently checks:
- counter-file and snapshot digests;
- cgroup identity and scope consistency;
- counter monotonicity and final emptiness;
- CPU, memory and OOM deltas;
- exact rational core-hour arithmetic;
- canonical report digest.

Rust tests invoke the Python oracle on synthetic known-answer fixtures. These tests establish deterministic parser and arithmetic behavior, not live kernel measurement accuracy.

## Remaining qualification work

1. Add a Linux-only runtime adapter that creates a unique delegated cgroup, records cgroup identity, moves the process tree, opens read-only memory.peak once, and captures raw source artifacts.
2. Test normal completion, process forks, grandchildren, timeout/SIGTERM, partial output, process escape, cgroup deletion failure and OOM kill.
3. Reconcile CPU deltas against independent OS tooling on controlled single-process and forked-child fixtures.
4. Validate memory.peak with a controlled allocation workload and exact FD semantics.
5. Test controller/mount capability discovery, including memory_localevents and missing memory.peak.
6. Integrate the resource receipt with the external solver receipt and matched-budget operation ledger.
7. Keep energy and GPU telemetry Unknown until their own source/attribution adapters pass independent tests.

## Claim ceiling

A future passing runtime qualification will support measured cgroup process-tree CPU time and peak memory for the declared Linux kernel, cgroup configuration and adapter version. It will not establish per-process energy, GPU consumption, solver correctness, aerodynamic validity or optimizer superiority.
