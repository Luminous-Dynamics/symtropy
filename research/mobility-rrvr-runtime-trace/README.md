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

## v0.1 scope boundary

This version models one original physical-effect attempt (`attempt-1`) and one recovery-authority admission (`attempt-2`). It verifies the evidence required to admit that recovery command, including the captured admission facts. It does **not yet** project the subsequent physical outcome of `attempt-2`, a second nested recovery cycle, or repeated recovery attempts. Those require a new evidence-scope/version design so the recovery attempt's own physical outcome, persistence and downstream graph cannot be confused with the original attempt. The current CLI/fixture must not be used to claim multi-cycle recovery verification.

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

The separate attempt-lineage validator now checks a linear chain of up to 64 attempts for one stable logical effect. Every retry edge must join the preceding attempt's confirmed NoEffect physical outcome, confirmed NoEffect mission resolution, definitely-not-committed persistence result, controller-queue fence, and downstream-quiescence receipt to the exact new boot/controller epoch.

The validator does not issue permits or replace owner authority. It is intentionally a separate contract because the pinned v0.1 TLA projection remains limited to one original attempt and one recovery admission. See [ATTEMPT_LINEAGE_V0.1.md](ATTEMPT_LINEAGE_V0.1.md) for the exact semantics, claim ceiling, negative controls, and next formal-refinement step.

The synthetic three-attempt fixture and JSON Schema are:
- fixtures/attempt_lineage_three_attempts_v0_1.json
- schemas/attempt_lineage_v0_1.schema.json

The Rust tests validate two NoEffect recovery edges followed by a terminal Applied attempt, and reject evidence replay, cross-attempt scope changes, incomplete persistence, epoch regression, wrong fence targets, and unsafe intermediate outcomes. This is a reference-contract test, not yet a Rust/TLA+ multi-cycle refinement proof or physical robotics result.
