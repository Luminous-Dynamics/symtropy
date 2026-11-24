# RRVR Runtime-to-TLA Projection v0.1

A fail-closed projection boundary for owner-evidenced robot execution records. It validates the full 43-field `s` record from the pinned `RestartSafeEffectContinuity.tla`, requires ten distinct evidence domains, and emits:

1. a Reflow-compatible JSONL trace (`state`, then optional `event` and `args`);
2. an RRVR provenance sidecar retaining owner revisions, scopes, raw artifact digests, source clocks and observer-quorum evidence;
3. a stdout SHA-256 manifest for the exact input, trace and sidecar bytes.

## Run

```sh
cargo fmt --manifest-path research/mobility-rrvr-runtime-trace/Cargo.toml -- --check
cargo test --manifest-path research/mobility-rrvr-runtime-trace/Cargo.toml --locked
cargo clippy --manifest-path research/mobility-rrvr-runtime-trace/Cargo.toml --all-targets --locked -- -D warnings

# Run the projection CLI, then validate its emitted trace with the pinned TLC jar:
# copy RestartSafeEffectContinuity.tla, RestartSafeEffectContinuityTrace.tla,
# RestartSafeEffectContinuityTrace.cfg and rrvr-trace.ndjson into one directory.
java -XX:+UseParallelGC -cp /path/to/pinned/tla2tools.jar tlc2.TLC \
  -workers 1 -config RestartSafeEffectContinuityTrace.cfg \
  RestartSafeEffectContinuityTrace.tla

cargo run --locked \
  --manifest-path research/mobility-rrvr-runtime-trace/Cargo.toml \
  --bin rrvr_runtime_project -- \
  research/mobility-rrvr-runtime-trace/fixtures/runtime_projection_reservation_acceptance_v0_1.json \
  /tmp/rrvr-runtime-trace.ndjson \
  /tmp/rrvr-runtime-provenance.json
```

The CLI reports `projection_validation=passed` and `tla_conformance=not_run`. The workflow then runs the emitted NDJSON through `RestartSafeEffectContinuityTrace.tla` with the pinned TLC jar. A separate `formal-conformance-manifest.json` is emitted only after both valid traces pass and the wrong-action negative control fails `TraceNotStuck`. The manifest binds the exact model, trace harness, config, jar and three TLC logs by SHA-256. The CLI's projection result and TLC's conformance result remain separate.

## Formal trace validation

The JSONL rows use the same complete `state`, `event`, and `args` shape as Reflow's documented `tla.trace` input:

- line 1 is an initial state: `{"state":{"s":{...}}}`;
- later lines contain the resulting state and the optional TLA action name/arguments;
- each snapshot contains the complete value of the model variable `s`.

This repository's `RestartSafeEffectContinuityTrace.tla` is an explicit action-and-state constrained TLC harness. It deserializes each JSONL row, selects the named TLA action, constrains the successor `s` to the recorded `state.s`, and advances a trace cursor. `TraceNotStuck` requires every row to be consumed. A wrong action name or impossible successor blocks cursor advancement and fails the invariant. The CI negative control mutates the first action label while preserving its state and must be rejected.

The harness deliberately does not infer an inner action from an outer `Next` label. The Rust projection rejects `Next` as an ambiguous specific action. Reflow's `tla.trace` is acknowledged as prior art; the explicit harness keeps our exact action/state contract and negative control reproducible in the pinned TLC workflow. See [Reflow TLA+ plugin documentation](https://reflow.io/docs/plugins/tla-plus/).

## Fail-closed evidence requirements

Every snapshot requires exactly one record from each of these ten evidence domains:

1. process epoch;
2. persistence;
3. physical outcome;
4. mission resolution;
5. controller queue;
6. downstream execution graph;
7. physical-outcome observer quorum;
8. downstream graph-quiescence observer quorum;
9. action applicability;
10. recovery authority.

Each evidence reference binds to a logical-effect ID, attempt ID and subject boot/controller epochs. The physical evidence, persistence, mission, queue, graph and observer records must refer to the same original attempt scope. Action applicability and recovery authority bind to the current recovery attempt/boot/controller epoch, not to stale authority from the original command.

The two observer claims are separate. When either is qualified, the claim is bound to a domain-specific digest. A qualified quorum requires:
- a versioned fault-model revision;
- at least two unique observers;
- a raw observation artifact and digest per observer;
- declared common-cause domains for each observer;
- pairwise disjoint declared common-cause domains.

When both physical-outcome and downstream-quiescence quorums are qualified, their observer IDs, artifacts and declared dependencies must also be disjoint under the same fault-model revision. This is a deliberately conservative *declared fault model* criterion, not a universal proof of statistical independence.

At initialization, both observer claims are `not_applicable`. The TLA variable `observerIndependent=TRUE` is an initial abstraction setting; it does not fabricate an already-observed physical quorum. Once an observed physical outcome is projected, the separate claim-specific observer proofs become mandatory. A disputed observation must project `observerIndependent=FALSE` and a disputed observer status.

## Base projection v0.1 scope boundary

This version models one original physical-effect attempt (`attempt-1`) and one recovery-authority admission (`attempt-2`). It verifies the evidence required to admit that recovery command, including the captured admission facts. It does **not yet** project the subsequent physical outcome of `attempt-2`, a second nested recovery cycle, or repeated recovery attempts. Those require a new evidence-scope/version design so the recovery attempt's own physical outcome, persistence and downstream graph cannot be confused with the original attempt. The original 43-field TLA runtime projection CLI/fixture must not be used to claim multi-cycle recovery verification. The separate attempt-lineage validator checks synthetic multi-attempt evidence structure, but its multi-cycle semantics still require the bounded TLA+ refinement tracked by #6985.

## Critical semantics

- `appliedEffectCount <= 1` is enforced even though the TLA TypeOK type domain permits 0..2; the 2 state is an explicit invariant counterexample.
- Indeterminate persistence cannot coexist with an issued/consumed recovery permit.
- An issued permit requires independently confirmed NoEffect, mission resolution NoEffect, a resolved persistence outcome, current M1/M2 and both current fences.
- A fresh ticket is bound to the current boot/controller epoch and a new attempt.
- After `NativeAcceptRecovery`, the model deliberately clears `downstreamFenceBootEpoch` to allow the recovery command to execute. The projection validates the captured `admittedDownstreamFence` fact, not a stale current-fence value.
- Current M1/M2 values may be revoked after recovery admission; the captured admission fields remain the evidence of what was true at acceptance time.
- Source clock values are retained with clock identity, boot epoch, unit, resolution and uncertainty. Incomparable clocks are never ordered by timestamp alone.

## Fixture

- `fixtures/runtime_projection_init_v0_1.json`: exact initial model state with ten not-yet-observed owner evidence domains.
- `fixtures/runtime_projection_reservation_acceptance_v0_1.json`: four-state `Init → ReserveAttempt → MarkDispatchStarted → NativeAcceptOld` trace, with all 43 model fields and ten owner evidence domains at each step.
- `fixtures/runtime_projection_no_effect_recovery_v0_1.json`: 17-state lifecycle covering old-attempt NoEffect, persistence no-commit reconciliation, crash/restart, controller and downstream fencing, physical observation, mission resolution, permit issuance, fresh ticket and one-shot recovery acceptance. The final state deliberately has `downstreamFenceBootEpoch=0` with `admittedDownstreamFence=true`.

All fixtures pin SHA-256 digests of the exact base TLA source, safe model configuration and TLC JAR. Every owner receipt carries an asserted value and source artifact commitment.

The observer evidence in the full recovery fixture contains two separately qualified proof sets with distinct observers, observation IDs, raw artifacts and artifact digests, and disjoint declared common-cause dependencies under the same fault-model revision. This is a synthetic known-answer fixture, not a physical deployment.

These fixtures are synthetic conformance inputs, not observations of a real robot.

## Claim ceiling

A projection PASS establishes only that a supplied recording satisfies this crate's typed shape, scope, provenance and selected invariant checks. It does **not** authenticate owner artifacts, prove that an event inventory is complete, establish model transition conformance, prove physical causality, qualify hardware, or establish a safety case. Use independent owner authentication, the pinned TLC trace harness, ROS/simulator/HIL/physical campaigns and applicable safety standards for those claims.


## Multi-attempt lineage extension v0.1

The separate attempt-lineage validator now checks a linear chain of up to 64 attempts for one stable logical effect. Persistence, physical outcome, mission resolution, controller-queue fencing and downstream fencing all reuse the parent OwnerEvidenceRef type; the typed value must exactly match the owner-asserted value in that same reference. Every retry edge must join the preceding attempt's confirmed NoEffect physical outcome, confirmed NoEffect mission resolution, resolved Committed or DefinitelyNotCommitted persistence result, controller-queue fence, and downstream-quiescence receipt to the exact new boot/controller epoch. Every confirmed physical outcome requires a claim-specific physical observer quorum. A retryable parent additionally requires a separate downstream-quiescence quorum; both reuse the existing OwnerEvidenceRef type and must qualify under the same fault-profile revision, with disjoint common-cause dependencies and exact claim-scope commitments. Owner evidence IDs, claim IDs, observer proof artifacts and observation IDs are unique across the lineage; a sensor may recur only with a new observation and artifact. The validator consumes the upstream observer-assurance owner's qualified proof; it does not replace its full dependency-graph, calibration, placement, clock or physical fault assessment. It also rejects same-attempt observer self-report where the observer envelope/member reuses an operational evidence-owner identity, and prevents observer artifacts from reusing the raw artifacts attached to operational owner receipts.

The validator does not issue permits or replace owner authority. It is intentionally a separate contract because the pinned v0.1 TLA projection remains limited to one original attempt and one recovery admission. See [ATTEMPT_LINEAGE_V0.1.md](ATTEMPT_LINEAGE_V0.1.md) for the exact semantics, claim ceiling, negative controls, and next formal-refinement step.

The synthetic three-attempt fixture and JSON Schema are:
- fixtures/attempt_lineage_three_attempts_v0_1.json
- schemas/attempt_lineage_v0_1.schema.json

The Rust tests validate two NoEffect recovery edges followed by a terminal Applied attempt, and reject evidence replay, cross-attempt scope changes, incomplete persistence, epoch regression, wrong fence targets, and unsafe intermediate outcomes. This is a reference-contract test, not yet a Rust/TLA+ multi-cycle refinement proof or physical robotics result.


## Paired fault campaign extension v0.1

The separate campaign-manifest validator freezes the test conditions used to compare recovery-assurance arms. It requires exact platform/environment/reset/fault-model digests, policy/scene/action-prefix identities, seeds, pair IDs, ordered fault schedules, expected dispositions, failure signatures, and declared layer/cut coverage.

The four-arm reference fixture includes a positive pair and a negative pair comparing full_rrvr with last_attempt_only; the same seed, task, policy digest, scene, action prefix, logical-effect ID and fault schedule are preserved within each pair. The synthetic negative schedule crosses process, observer, controller and firmware layers.

- Validator: src/fault_campaign.rs
- CLI: src/bin/rrvr_campaign_validate.rs
- Schema: schemas/fault_campaign_v0_1.schema.json
- Fixture: fixtures/fault_campaign_paired_v0_1.json
- Specification: [FAULT_CAMPAIGN_V0.1.md](FAULT_CAMPAIGN_V0.1.md)
- Tracking: [#7025](https://github.com/Luminous-Dynamics/symtropy/issues/7025)

The campaign CLI emits exact input/profile digests, pair/scenario/control counts, declared and covered layers/cut points, and explicit false claim-ceiling fields. A PASS qualifies only the manifest as complete and internally comparable; it does not mean fault injection executed, platform inventory was authenticated, or physical safety was established.


## Deterministic compiled campaign plan v0.1

The campaign manifest can now be compiled into a canonical plan with:
- exact source-manifest SHA-256;
- a pair commitment shared by comparator/full-RRVR arms;
- a per-scenario digest of the exact ordered fault schedule;
- a top-level plan SHA-256 and separate compiled-file artifact digest;
- full platform, injector, native adapter, runtime monitor and reset bindings.

The compiler CLI is rrvr_campaign_compile. The companion rrvr_campaign_verify CLI checks the strict plan structure, per-schedule digests, pair membership/equivalence/commitments, required layer/cut coverage, top-level digest, exact source-manifest bytes, canonical bytes and an externally pinned expected plan digest.

- Implementation: src/campaign_plan.rs
- Compiler: src/bin/rrvr_campaign_compile.rs
- Verifier: src/bin/rrvr_campaign_verify.rs
- Schema: schemas/compiled_campaign_plan_v0_1.schema.json
- Specification: [CAMPAIGN_PLAN_V0.1.md](CAMPAIGN_PLAN_V0.1.md)
- Tracking: [#7027](https://github.com/Luminous-Dynamics/symtropy/issues/7027)

CI compiles the same source twice and compares exact bytes, verifies the compiled plan and rejects a mutated fault event. A verified plan remains an instruction artifact only: it does not run the injector, create native authority, or establish physical safety.


## Plan-bound fault execution evidence v0.1

The compiled plan now has a separate execution-evidence contract. It distinguishes target resolution, adapter dispatch, injector execution and independent delivery observation. A Confirmed delivery requires the exact target revision/profile, accepted adapter dispatch, a pinned injector execution receipt at the planned simulation step and a separate observer artifact/domain reporting the exact expected fault signature.

- Validator: src/execution_evidence.rs
- CLI: src/bin/rrvr_execution_validate.rs
- Schema: schemas/execution_evidence_bundle_v0_1.schema.json
- Fixture: fixtures/execution_evidence_positive_v0_1.json
- Specification: [EXECUTION_EVIDENCE_V0.1.md](EXECUTION_EVIDENCE_V0.1.md)
- Tracking: [#7051](https://github.com/Luminous-Dynamics/symtropy/issues/7051)

Unknown dispatch, execution or delivery states require reconciliation; the report never grants retry permission. Reset and all event receipts share one simulation clock, while evidence IDs, artifact IDs/digests, dispatch IDs and idempotency keys cannot be replayed within a bundle. The CLI validates records only—it does not run Gazebo, ROS, HIL or physical fault injection.


## Gazebo simulator adapter core v0.1

The first transport-neutral adapter core now validates an exact Gazebo capability profile and provides mock-tested orchestration for pause, reset, discrete stepping, target resolution and idempotent dispatch reconciliation.

- Module: src/gazebo_adapter.rs
- Preflight CLI: src/bin/rrvr_gazebo_preflight.rs
- Capability schema: schemas/gazebo_capability_profile_v0_1.schema.json
- Synthetic profile: fixtures/gazebo_capabilities_positive_v0_1.json
- Specification: [GAZEBO_ADAPTER_V0.1.md](GAZEBO_ADAPTER_V0.1.md)
- Tracking: [#7067](https://github.com/Luminous-Dynamics/symtropy/issues/7067)

The campaign/compiled plan pin the simulator build, ROS distribution, Simulation Interfaces package revision, SceneBroadcaster ID/revision/binary digest, environment and reset profile. The capability profile must advertise GetSimulatorFeatures, ResetSimulation, state query/update, entity query and one discrete step interface (StepSimulation or SimulateSteps). Missing SceneBroadcaster or a mismatched plugin/API build fails closed.

The transport trait has no live ROS client implementation yet. The CLI validates a captured capability profile only; it does not contact Gazebo, reset/step a world, or inject a fault. Mock tests exercise the orchestration, and the actual ROS 2 service/action client remains the next integration.


The execution-evidence validator also enforces cross-event chronology: fault event N+1 cannot begin with a receipt earlier than the last available receipt of event N on the same simulation clock. Local correctness is not enough if the global trace is reordered.

### Delivery observer quorum extension

Fault delivery confirmation now requires the campaign-pinned minimum observer quorum (at least two profiles), each pinned by observer ID/revision/binary digest and declared common-cause domains. Confirmed/Refuted observations carry separate receipts per observer. The validator rejects shared declared domains between quorum members or with the injector, replayed observer artifacts and unpinned binaries.

The positive four-event bundle carries eight observer receipts: two per fault. The evidence report separates four delivery verdicts from eight supporting observer receipts. This is still fault-model-relative independence, not a claim of physical independence.


## Plan-derived Gazebo dispatch composition

dispatch_planned_fault_once now composes the verified plan, exact simulation cut point, target resolution, final world/target rechecks and one idempotent injector dispatch. The function rejects a changed world epoch, clock, iteration, simulation time, world-state digest or target binding before dispatch. An Unknown injector result remains a reconciliation requirement. This is covered by mock-transport regression tests; it is not a live Gazebo run.

The final snapshot/re-resolution narrows a time-of-check/time-of-use window but does not provide an atomic world lock. Gazebo's standard interfaces do not enforce exclusive mutation ownership. A live campaign must isolate and control all world-mutating clients, and must account for documented reset/spawn races, top-level-model-only enumeration, SceneBroadcaster synchronization and the separate /stats step-feedback clock. See [GAZEBO_ADAPTER_V0.1.md](GAZEBO_ADAPTER_V0.1.md).

The research positioning has also been updated to acknowledge EAST (October 2026) and existing ROS fault-injection/tracing tools as prior art. RRVR's intended differentiator is cross-restart physical-effect lineage and authority continuity, not generic fault injection or digital-twin safety testing. See [RESEARCH_POSITIONING_AND_BENCHMARKS_V0.1.md](RESEARCH_POSITIONING_AND_BENCHMARKS_V0.1.md).


### Durable injector idempotency and payload conflict

The dispatch request now includes a SHA-256 commitment over its canonical plan/run/fault/target payload. The local registry detects same-process duplicates and same-key/different-payload conflicts; it is not durable and cannot protect a new coordinator process by itself. A concrete injector must persist idempotency-key-to-payload-digest-to-outcome atomically before effect execution, return the stored outcome for an identical retry, and reject a changed payload under the same key. `PayloadConflict` is distinct from `Unknown` and requires investigation, not retry.

A test double exercises the contract across a simulated coordinator restart. A real injector ledger, its persistence semantics and its physical effect boundary remain unqualified.


### Blind observer evaluation

The Gazebo observer trait no longer receives the expected fault-signature digest. The independent assessor retains that oracle and compares it only after the observer returns its measured signature. A structurally qualified quorum now produces Confirmed (unanimous expected signature), Refuted (unanimous different signature), or InsufficientEvidence (observer disagreement). Invalid membership, stale scope and common-cause overlap remain hard errors. These verdicts do not grant authority.


This increment advances the payload-conflict and observer-blinding portions of [#7148](https://github.com/Luminous-Dynamics/symtropy/issues/7148). The durable append-only dispatch journal and real injector-side ledger are not implemented by this mock-transport PR.


### Reset-qualified dispatch session

Plan-derived dispatch now requires a private reset-verification marker created only by reset_and_verify after exact initial-state, epoch, clock and pause checks. Capability preflight alone is not enough to dispatch. The dispatch path compares the session against the private marker and re-queries the live capability profile; plan, reset, session or profile drift blocks the injector call. The marker is process-local only. After coordinator restart, a concrete adapter must reconcile a durable reset receipt from the #7148 journal rather than reconstructing reset qualification from an epoch string.


An ambiguous ResetSimulation response is non-retryable. The session moves to OutcomeUnresolved, a second reset and any fault dispatch are blocked, and reconcile_reset may restore Verified only from a read-only observation that matches the exact planned new epoch and initial state. This protects the live-process case; reset intent and outcome still need durable write-ahead journaling under #7148 to survive coordinator crashes.


The process-local dispatch guard retains the last observed outcome as well as the payload digest: Unknown and PayloadConflict are not collapsed into DuplicateSuppressed on repeat calls. Independent delivery observation is bound to the exact scheduled simulation step in this v0.1 contract; later-step observations are rejected unless a future plan version explicitly commits an observation window.


The dispatch idempotency key uses a versioned hash of a canonical, plan-verified event/run identity rather than delimiter-concatenated identifiers. The scenario/fault must be members of the integrity-verified plan; the payload commitment then binds the generated key to the target and exact fault request.


The reset marker is bound to the exact compiled plan digest, not only to the simulator/world profile. A session qualified under one plan cannot be reused for another valid campaign on the same simulator. The session is non-cloneable and the raw injector dispatch wrapper is private; use the composed plan-bound dispatch API.


The new model/GazeboResetLifecycle.tla is checked with the workflow's pinned TLC tool: the safe lifecycle must satisfy at-most-one reset request and no fault dispatch before verified reset; the unsafe retry config must produce an AtMostOneResetRequest counterexample. This remains a bounded abstract protocol model, not a proof of Gazebo transport behavior or a Rust/TLA refinement result.


Fault-dispatch keys and payload commitments include the verified post-reset world epoch and simulation clock identity. Reusing the same plan/run/fault fields against a different world generation or clock must produce a different key and request fingerprint.


Dispatch reconciliation is bound to the registered request payload and updates the local record with the read-only injector-ledger decision. NoDispatchProven, Unknown and PayloadConflict remain non-dispatching for that key; a genuinely new attempt must have a new run/attempt identity.


The reset model now also pins the campaign identity at BeginReset and checks DispatchUsesResetPlan. The second TLC negative control swaps the active campaign after reset verification and must produce a counterexample if dispatch can reuse the old reset qualification under the new plan.
