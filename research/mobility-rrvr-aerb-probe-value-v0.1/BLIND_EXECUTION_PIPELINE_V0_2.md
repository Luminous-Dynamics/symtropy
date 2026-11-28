# AERB blind execution pipeline v0.2

Status: implementation candidate, exact-head CI pending. This is a process-model/reference-policy execution path, not a learned-agent result, simulator result, HIL qualification, physical qualification or certification.

## 1. Three mutually disjoint roots

The runner requires three host paths that remain pairwise disjoint after canonicalization:

| Root | Contents | Candidate mount |
|---|---|---|
| Candidate input | Opaque manifest, projected episodes, READY marker | `/candidate/input`, read-only |
| Candidate output | Initially empty; candidate-created submissions only | `/candidate/output`, writable |
| Evaluator custody | Hidden reveal, source episodes/submissions, world packs, projection receipts/key, leakage rows, custody seal and host-run receipt | Never mounted |

The materializer creates all three roots with mode 0700. Candidate files are created with mode 0600, no-replace semantics and fsync. The input root contains no source submission or precomputed reference answer. Source submissions are retained only in evaluator custody to validate source integrity and replay the projection.

## 2. Execution chain

1. Validate the source campaign and its bijective episode/truth mapping.
2. Generate a blind assignment preserving the source truth-class counts; opaque IDs and order are CSPRNG-generated. The distribution is a benchmark design, not an empirical probability model.
3. HMAC-SHA-256 project source identities, digests and clocks while preserving within-domain event/acquisition intervals and ordering.
4. Write projected input episodes, source archives, evaluator world packs, receipts, reveal, projection key and metadata-audit input to their respective roots.
5. Emit a pre-run whole-manifest commitment and READY markers; fsync each root.
6. Build the candidate execution command from typed allowlisted arguments; the current reference policy receives only `--candidate`.
7. Launch directly through `timeout → prlimit → bubblewrap`, with a cleared environment, user/PID/IPC/network/UTS namespaces, dropped capabilities, read-only input/runtime, writable output, private bounded `/tmp`, and resource limits. There is no unsandboxed fallback.
8. Suppress candidate stdin/stdout/stderr; only candidate-created files in the writable output mount are collected.
9. After process termination, the host runner inventories output names, validates regular files, hashes artifacts, records missing and unexpected entries, binds candidate executable/runtime/host-tool digests and sandbox argv/limits into a run-profile digest, and writes the receipt to evaluator custody.
10. The evaluator independently inventories the output directory, reconciles it against the supervisor receipt, checks byte lengths and hashes, validates each candidate submission against its projected episode snapshot, replays projection from evaluator-only source data and secret, then scores against the evaluator-only world pack.

A process crash, nonzero exit, timeout, missing output, extra output, symlink, non-regular file, oversized file, read error, hash mismatch or replay failure cannot be promoted to a completed valid episode. Missing episodes remain in the scheduled denominator. A host runner that cannot write its receipt is an infrastructure failure and does not produce a qualified report.

## 3. Current reference policy

The first executable is intentionally conservative:
- retain all eight possible histories, including the open-world residual;
- `hold_reconcile` when all observations are admitted;
- `abstain` whenever any observation is stale, unknown or disputed;
- never recommend a fresh physical attempt;
- never request an extra probe.

It exists to validate the isolation, output, custody and evaluator pipeline. It does not demonstrate adaptive recovery, model intelligence, SOTA task success, or superior decision quality. The next benchmark candidates are ordinary retry-after-timeout, stop-and-human-reconcile, endpoint-journal reconciliation, greedy information gain, calibrated expected-value sensing, robust minimax/minimax-regret and a clairvoyant evaluator upper bound.

## 4. Security properties and residual risk

Implemented as source-level controls, pending exact-head qualification:
- path canonicalization and pairwise root separation;
- input/runtime read-only mounts, output-only writable mount, evaluator root omitted;
- direct argv rather than a shell; typed candidate argument allowlist;
- environment clear, null standard streams, namespace isolation, dropped capabilities;
- per-process rlimits and wall timeout;
- pre/post run byte digests and evaluator-only receipts;
- no-replace output creation, fsync and strict output inventory.

Not established by this implementation:
- cgroup v2 aggregate CPU/memory/PID enforcement; rlimits are per process/resource and are not equivalent to cgroup quotas;
- aggregate writable-output disk quota. Per-file RLIMIT_FSIZE and post-run inventory do not prevent every disk-exhaustion strategy;
- seccomp syscall filtering, kernel exploit resistance, hostile-host resistance, microVM isolation or a separate evaluator service;
- race-free immutable source custody using open file descriptors throughout (the materializer currently canonicalizes/checks paths then opens files);
- cryptographic attestation of the host supervisor or issuer identity. SHA-256 binds bytes; it does not prove who produced them.
- resistance to every possible metadata, timing, adaptive, temporal or stateful inference attack. The grouped categorical attack suite is diagnostic, not a proof of blindness.
- physical sensor truth, actual effect multiplicity, command-holder closure or robot viability.

For production-grade blind evaluation, the preferred next layer is a separate evaluator service plus a microVM (e.g. Firecracker) or hardened gVisor boundary, cgroup v2 limits, seccomp, quota-controlled output storage, immutable content-addressed input bundles, open-file-descriptor-based custody and independent run attestation.

## 5. Benchmark and comparison protocol

AERB is an assurance overlay, not a replacement for host benchmarks:
- **RoboRecover**: preserve its RoboTwin/LIBERO split and source-policy grouping; report recovery separately from initial-state success.
- **DiMuRe / LIBERO-Disturb**: publish matched physical-step/sensing/compute/deadline budgets separately from expanded budgets with extra steps, replanning or handoff.
- **SafeVLA-Bench**: retain native task success, all-spec safety, successful-but-unsafe (SBU) and worst-violation severity (VSI); add AERB effect-ambiguity, authority and evidence metrics alongside them.
- **ActiveArena**: compare multi-round sensing and ID/OOD performance; count probe cost and physical stop/abort qualification.
- **EmbodiedGovBench**: measure runtime drift, recovery, override and audit completeness without re-labelling its native scores.

Primary AERB metrics:
- native task success and recovery success;
- false NoEffect, false finality and false scope closure;
- unsafe fresh-attempt recommendation;
- hidden-truth support coverage and open-world-residual retention;
- stale/unknown/disputed evidence acceptance;
- authority revocation race outcomes at native apply;
- probe loss regret vs. evaluator oracle, joules/time/steps to safe hold;
- candidate-output missing/extra/malformed/unresolved counts;
- metadata-only attack accuracy/balanced accuracy on grouped held-out clusters;
- invalid and unresolved episodes, always reported in the scheduled denominator.

Matched-budget results must equalize physical steps, sensing calls, compute/token budget, wall deadline and allowed skills. Expanded-budget results must disclose each additional resource. Never combine those tracks into one ranking.

## 6. Qualification gates and status labels

| Evidence tier | Required gate before promotion |
|---|---|
| Source/schema | Schema and deterministic fixture conformance |
| Software | Rust 1.96 format, locked tests, strict Clippy, mutation tests |
| Process model | Complete materialize → isolated execute → output seal → evaluator replay; zero invalid/extra/missing outputs in the reference fixture |
| Sandbox | Actual Bubblewrap canary passes on exact head; no claim from builder unit tests alone |
| Simulator | Matched-budget RoboRecover/DiMuRe-compatible and SafeVLA-aligned comparisons; zero hard safety violations in declared envelope |
| HIL | ROS 2 Control/firmware lifecycle with independent safety supervisor |
| Physical | Reduced-speed/nonhazardous tests with interlocks and independent qualification |

No exact-head CI result is represented as passed until Actions reports it. Even a clean reference-policy process run qualifies only the software/custody path; it is not a learned-agent benchmark score or evidence of physical safety.
