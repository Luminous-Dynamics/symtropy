# SIM-REL-037 — independent model-family ledger transition oracle

Status: research implementation on a draft branch. Exact-head compilation and hosted qualification are pending. This module is a logical state-machine auditor, not an independent physics engine or a source of physical-model adequacy claims.

## 1. Why this exists

SIM-REL-034 provides an append-only admission ledger for counterexample-guided model-family refinement. SIM-REL-035 exports and reconstructs the ledger transcript, but its offline reconstruction calls the same `ModelFamilyLedger` constructor and transition functions that produce the live transcript.

That separation is useful, but it does not eliminate common-mode transition defects. If both the producer and replay verifier share an incorrect transition rule, exact replay can reproduce the same defect.

SIM-REL-037 adds a second implementation of the *logical lifecycle*. It reads the immutable baseline and event records, derives the state using a separate transition representation, and compares its result with the exported snapshot.

## 2. Explicit independence boundary

The oracle deliberately does **not** call:

- `ModelFamilyLedger::new` or any live ledger transition method;
- the counterexample perturbation constructor or physical-world mutation helper;
- `replay_shared_policy_across_worlds` or a finite-horizon optimizer;
- the SIM-REL-035 snapshot verifier.

It independently checks:

- event sequence and revision arithmetic;
- baseline and candidate world identities and append order;
- active-policy invalidation and reactivation;
- policy identity/tree/revision binding;
- development/calibration/sealed split separation;
- unique event, validation and evaluation-artifact identities;
- exact development-world coverage in replan receipts;
- metric vector shape, probability ranges and declared risk budgets;
- final-policy sealing and immutable policy/revision identity;
- registered holdout group, manifest and world-ID binding;
- one-time holdout consumption, including failed evaluation;
- holdout aggregate arithmetic and acceptance-flag consistency;
- final snapshot equality for all logical state derived from the event stream.

The oracle does **not** prove:

- that a declared perturbation physically produces its candidate model world;
- that recorded replan or holdout metrics are the true result of executing the policy;
- that a validator or split registry is honest;
- that two differently named groups do not share a real-world source;
- that the finite model family contains the actual physical process;
- that a simulated trajectory is safe in a physical apparatus.

Those remain independent evidence obligations. A passing transition audit is one layer in a composed qualification, never a substitute for physical replay, instrument calibration, provenance authentication or validated abstraction.

## 3. State model

Let the ledger state be

`S = (r, W, P, I, F, C, H, V, E, A)`

where:

- `r`: current model-family revision;
- `W`: ordered development-world snapshots and their unique IDs;
- `P`: active policy identity/tree and its per-world metric receipt, or no active policy;
- `I`: identity of the policy invalidated by the most recent admitted counterexample;
- `F`: frozen final policy identity/tree/metrics/revision, or absent;
- `C`: pre-registered calibration groups;
- `H`: pre-registered sealed-holdout groups;
- `V`: consumed validation-receipt IDs;
- `E`: consumed event-receipt IDs;
- `A`: consumed holdout evaluation-artifact IDs and group IDs.

The exported snapshot now also retains `initial_policy_metrics`, the exact baseline replay metric receipt for the initial policy. This allows the logical oracle to require that an initial-policy seal agrees with the baseline receipt. The receipt is still part of the supplied evidence envelope; this module does not cryptographically authenticate it or independently reproduce the underlying policy replay.

### 3.1 Admission

A counterexample admission is structurally valid only when:

1. its sequence is the next contiguous event sequence;
2. `revision_before == r` and `revision_after == r + 1`;
3. its source world exists and its stored source snapshot equals the current development snapshot;
4. its candidate ID is new, differs from the source ID, and equals the candidate snapshot ID;
5. its provenance split is Development and its group does not alias a registered calibration or holdout group;
6. its provenance binds to the active policy identity, exact policy tree and current revision;
7. its validation receipt is complete and unused;
8. its claimed and independently replayed outcome classes are non-NoViolation and agree.

The oracle then appends the candidate snapshot, increments the revision once, removes the active policy, and records the invalidated identity. It does not recompute the physical perturbation or decide whether the claimed outcome is physically true.

### 3.2 Replan

A replan is valid only after admission has invalidated the active policy. Its revision must equal the current revision; its identity must be new; and both its replayed-world list and its per-world metric list must cover exactly the current development family.

The independent transition oracle validates the metric structure and the declared hazard/abstention budgets. It does not establish that those metrics were generated by a correct policy replay. SIM-REL-035 and the exact policy replay receipt remain required for that claim.

### 3.3 Final seal

A final seal must identify the currently active policy, exact policy tree and current revision. Its per-world metrics must cover the current family and match the metrics already retained for that policy. For the initial policy, that means matching the immutable initial-policy metric receipt; for a replanned policy, it means matching the replan event.

After sealing, neither family expansion nor re-planning is legal.

### 3.4 Sealed holdout

A holdout event must occur after final sealing and must match:

- a registered sealed group and its manifest ID;
- the frozen policy identity and revision;
- the exact registered world-ID set and supplied snapshot IDs;
- a unique evaluation-artifact ID.

For successful replays, the oracle recomputes worst-case total loss and terminal loss from the recorded per-world metrics and verifies the exact acceptance predicate. For failed replays, it requires a non-empty failure reason, no successful metrics/aggregates, and a false acceptance flag.

The group is consumed in either case and cannot be reopened for tuning.

## 4. Qualification design

The focused suite should combine valid transcripts generated by SIM-REL-034 with mutations rejected by both independent auditors.

Required negative controls:

- sequence gap, repeated sequence and stale/non-adjacent revision;
- source snapshot substitution and candidate ID collision;
- candidate admitted from Calibration or SealedHoldout;
- duplicate event, validation, policy or artifact identity;
- stale policy identity/tree and policy resurrection;
- replan without an admission, incomplete world coverage and out-of-budget metric;
- seal of a stale policy/tree/revision or altered metrics;
- holdout before seal, wrong group/manifest/world set, repeated consumption;
- failed holdout whose record claims success, or successful holdout with inconsistent aggregate/acceptance;
- any post-seal admission or replan;
- forged active/frozen policy state, receipt sets, consumed groups or final revision.

The test strategy has three layers:

1. **Producer** — SIM-REL-034 creates real state transitions and records exact physical/policy replay receipts.
2. **Independent logical oracle** — SIM-REL-037 derives transition state without invoking the producer's transition methods or replay engine.
3. **Offline replay verifier** — SIM-REL-035 reconstructs the transcript through its established replay path and verifies exact event and snapshot equality.

The two verifiers must agree on valid transcripts and reject a shared mutation corpus, but agreement is not enough to prove physical correctness because SIM-REL-035 shares the producer's semantic functions. A later independently implemented policy/physics oracle is still required for that claim.

## 5. Performance and correctness reporting

Record:

- valid transcript count, event count and family-revision depth;
- per-event audit latency and total transcript audit latency;
- accepted/rejected mutation counts by failure class;
- false acceptance count (target: zero on the declared mutation corpus);
- false rejection count on valid producer-generated transcripts (target: zero);
- peak memory and serialized snapshot size;
- maximum supported development-world and event counts.

Report the exact compiler/toolchain, source SHA, Cargo lockfile SHA, generated corpus manifest, and workflow checkout SHA. Do not convert a small deterministic unit-test corpus into a general security or model-adequacy guarantee.

## 6. Roadmap after SIM-REL-037

- SIM-REL-038: adaptive recipe selection driven only by Development outcomes, with explicit difficulty, mechanism coverage and benchmark anti-saturation metrics.
- SIM-REL-039: exact finite physical challenge families for actuator, sensor, thermal and mechanical faults.
- SIM-REL-040: causal action-realization evidence from command through actuator readback and physical measurement.
- SIM-REL-041: matched adapters for sequential planning, model-error-aware design, embodied scientific discovery and trajectory safety benchmarks.

No SOTA, open-world adequacy, physical-safety or production-readiness claim is made by this module.
