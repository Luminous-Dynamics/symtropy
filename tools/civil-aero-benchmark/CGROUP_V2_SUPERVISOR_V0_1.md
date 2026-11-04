# Delegated cgroup v2 process-tree supervision v0.1

**Status:** Rust supervisor prototype and deterministic gate tests; no delegated Linux-host execution result is claimed yet.  
**Issue:** #3570 — AERO-LIVE-CGROUP-SUPERVISOR-001.  
**Dependency:** #3498 — AERO-RESOURCE-TELEMETRY-001 (snapshot parser, resource derivation and independent raw-snapshot oracle).

## Purpose

The existing resource telemetry parser derives CPU, wall, peak-memory and OOM deltas from two cgroup v2 snapshots. This supervisor adds the missing live process boundary: run one command in a fresh operation-owned cgroup, include all descendants, capture start/end snapshots, and retain enough raw evidence for independent replay.

It measures process-tree CPU and peak memory. It does **not** infer GPU time, energy, solver correctness, convergence, CFD validity or experimental agreement.

## Delegation is a hard precondition

The operator supplies an already delegated cgroup v2 subtree. Before creating a child group, the runner verifies:
- the path resolves under a cgroup2 mount from `/proc/self/mountinfo`;
- it is not the cgroup2 mount root itself;
- `cpu` and `memory` are available and already enabled in the delegated root's `cgroup.subtree_control`;
- the delegated root's `cgroup.procs` is writable;
- no attempt is made by this program to write `cgroup.subtree_control`, edit systemd policy, or change manager-owned ancestors.

On systems using systemd, delegation must be configured by the administrator/service manager. Do not point the runner at `/sys/fs/cgroup`, a service-manager-owned ancestor, or an arbitrary user's non-delegated cgroup.

The result directory must be new, have an already-existing parent, and be outside the whole cgroup2 mount filesystem and delegated subtree. The program does not create parents before checking this condition.

## Launch and measurement sequence

1. Validate request, run ID, explicit allow-listed environment and delegated root.
2. Create a fresh child domain cgroup; require CPU and memory controllers and `cgroup.kill`.
3. Open `memory.peak` once read-only. Record descriptor device/inode and `F_GETFL` flags. Retain this FD for both samples.
4. Capture baseline `cpu.stat`, `memory.peak`, `memory.events`, `cgroup.events` and mountinfo identity. Require `populated=0`, `frozen=0`, and `memory.peak=0`.
5. Spawn the child behind a Unix socket pre-exec gate. The child blocks before `exec`; the parent writes its PID to the new cgroup's `cgroup.procs` and only then releases the gate. This ensures the target and subsequent forked descendants begin inside the measured cgroup.
6. Poll the direct child and recursive `cgroup.events populated`. On timeout, write `cgroup.kill`, then wait for the entire hierarchy to become empty.
7. Capture final raw files, using the same still-open read-only `memory.peak` descriptor. Require empty/unfrozen final boundary and monotonic counters.
8. Reuse #3498's `parse_cgroup_v2_snapshot` and `derive_cgroup_v2_resource_report`; do not duplicate counter math.
9. Persist raw start/end files, mountinfo, exact command/environment manifests, stdout/stderr, resource evidence CSV, artifact inventory and canonical run receipt.
10. Close the retained memory FD, then remove the empty child cgroup. If finalization fails while the cgroup is populated, the guard does not remove it.

The resource evidence interval begins at baseline capture and ends after the descendant hierarchy is empty. It includes launch-gate and cleanup overhead within that interval; it excludes the outer Rust supervisor's own CPU use, because that supervisor remains outside the operation cgroup. The child process tree—including nested Python worker processes—is inside.

## CLI

Build the Rust CLI from the benchmark crate:

```sh
cargo build --locked --manifest-path tools/civil-aero-benchmark/Cargo.toml --bin cgroup_v2_run
```

Run with an existing results parent directory and an already delegated cgroup v2 subtree:

```sh
tools/civil-aero-benchmark/target/debug/cgroup_v2_run \
  --delegate-root /sys/fs/cgroup/<delegated-subtree> \
  --result-dir /var/tmp/symtropy-runs/<unique-run-id> \
  --run-id <unique-run-id> \
  --timeout-seconds 3600 \
  --cwd /path/to/working-directory \
  --env OMP_NUM_THREADS=4 \
  -- /absolute/path/to/program arg1 arg2
```

The executable is passed as argv, not interpreted by a shell. The environment is cleared and rebuilt from the fixed allow-list plus reviewed `--env KEY=VALUE` overrides. Duplicate explicit overrides and unreviewed environment keys are rejected.

Exit codes:
- `0`: process exited zero and the cgroup cleanup completed;
- `3`: a terminal receipt was retained for non-success (nonzero exit, signal, timeout or cleanup failure);
- `2`: preflight/supervisor rejection or failure before a terminal receipt could be completed.

The CLI prints a receipt SHA-256 and evidence ceiling. A zero wrapper code does not imply solver convergence or physical validity.

## Retained run bundle

The result directory contains:
- `command-manifest.txt`: hashed program/cwd/argv identity (argv text is not stored in plaintext);
- `environment-manifest.txt`: allow-listed environment keys and value digests;
- `stdout.bin`, `stderr.bin`: raw child output;
- `start-*.txt`, `end-*.txt`: exact raw cgroup counter files;
- `cgroup2-mountinfo.txt`: source mountinfo line;
- `resource-evidence.csv`: start/end snapshots and derived CPU/wall/peak-memory/OOM report;
- `artifacts.csv`: byte length and SHA-256 for every retained artifact;
- `run-receipt.csv`: canonical terminal status, resource values and digests.

Energy and GPU are unknown; no estimate is inferred from CPU usage. The snapshot report is process-tree CPU/peak-memory evidence, not a claim of per-process attribution.

## Independent resource replay

The dependent #3498 resource oracle can replay the resource evidence file without a Rust runtime dependency:

```sh
python3 tools/civil-aero-benchmark/scripts/verify-cgroup-v2-resource-independent.py \
  /var/tmp/symtropy-runs/<unique-run-id>/resource-evidence.csv
```

This verifies the raw snapshot digests, snapshot/report canonical hashes, counter deltas and evidence ceiling. Then verify the complete run bundle:

```sh
python3 tools/civil-aero-benchmark/scripts/verify-cgroup-v2-run-independent.py \
  /var/tmp/symtropy-runs/<unique-run-id>/run-receipt.csv
```

The run verifier re-hashes the artifact inventory, checks exact raw counter files against the snapshot CSV, checks command/environment identity manifests and receipt accounting, and invokes the independent resource oracle. The verifier's synthetic self-test creates a known-answer run bundle and rejects an artifact tamper.

## Known limitations before a live-host qualification

- CI can exercise the pre-exec gate using a temporary file as a fake `cgroup.procs`; it cannot prove cgroup delegation and process-tree behavior on every systemd/kernel configuration.
- A real Linux-host run with an explicitly delegated CPU+memory subtree is required before any live process-tree measurement claim.
- The executable path and argv are bound as command identity, but this prototype does not yet use `execveat`/an already-open executable FD to defeat a hostile executable-path replacement race. Do not treat it as hostile-host attestation.
- If recursive kill fails or the cgroup remains populated past the grace interval, the group is retained for operator intervention; the runner does not delete a live cgroup or claim completed measurement.
- Host energy and device GPU counters are outside this adapter.
- A cgroup receipt does not authenticate the host, operator, executable author, solver model, or scientific input rights.

## Research targets unlocked by this work

Once a live-host qualification exists, benchmark policies on two simultaneous axes:
1. official objective/condition call budget, unchanged from BOCoDe or the CFD protocol;
2. full operation cost: cgroup CPU core-hours, wall time, peak memory, failed/retried calls and (later) qualified GPU/energy.

Compare cost-quality curves, time-to-first-feasible, feasible attainment, validated hypervolume, evidence coverage and worst-family performance. Keep failed attempts in the cost denominator. This is the prerequisite for claims such as “20% fewer high-fidelity condition evaluations at non-inferior validated feasible hypervolume”; it does not establish that claim.
