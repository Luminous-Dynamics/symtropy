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

cargo run --locked \
  --manifest-path research/mobility-rrvr-runtime-trace/Cargo.toml \
  --bin rrvr_runtime_project -- \
  research/mobility-rrvr-runtime-trace/fixtures/runtime_projection_reservation_acceptance_v0_1.json \
  /tmp/rrvr-runtime-trace.ndjson \
  /tmp/rrvr-runtime-provenance.json
```

The CLI reports `projection_validation=passed` and `tla_conformance=not_run`. The output is designed to be checked against the pinned specification by a separate formal trace checker.

## Formal trace validation

The JSONL shape is compatible with Reflow's documented `tla.trace` input:

- line 1 is an initial state: `{"state":{"s":{...}}}`;
- later lines contain the resulting state and the optional TLA action name/arguments;
- each snapshot contains the complete value of the model variable `s`.

Use the actual pinned model and an appropriate TLC/Reflow trace configuration in the formal owner workflow. A projection validator PASS is not a Reflow/TLC trace-conformance PASS. Reflow documents this distinction: model checking checks the specification; `tla.trace` checks recorded implementation events against it. See [Reflow TLA+ plugin documentation](https://reflow.io/docs/plugins/tla-plus/).

## Fail-closed evidence requirements

Every snapshot requires exactly one record from each of these ten evidence domains:

1. process epoch;
2. persistence;
3. physical outcome;
4. mission resolution;
5. controller queue;
6. downstream execution graph;
7. physical no-effect observer quorum;
8. downstream graph-quiescence observer quorum;
9. action applicability;
10. recovery authority.

Each evidence reference binds to a logical-effect ID, attempt ID and subject boot/controller epochs. The physical evidence, persistence, mission, queue, graph and observer records must refer to the same original attempt scope. Recovery authority has its own attempt scope.

The two observer claims are separate. When either is qualified, the claim is bound to a domain-specific digest. A qualified quorum requires:
- a versioned fault-model revision;
- at least two unique observers;
- a raw observation artifact and digest per observer;
- declared common-cause domains for each observer;
- pairwise disjoint declared common-cause domains.

When both physical no-effect and downstream-quiescence quorums are qualified, their observer IDs, artifacts and declared dependencies must also be disjoint under the same fault-model revision. This is a deliberately conservative *declared fault model* criterion, not a universal proof of statistical independence.

At initialization, both observer claims are `not_applicable`. The TLA variable `observerIndependent=TRUE` is an initial abstraction setting; it does not fabricate an already-observed physical quorum. Once an observed physical outcome is projected, the separate claim-specific observer proofs become mandatory. A disputed observation must project `observerIndependent=FALSE` and a disputed observer status.

## Critical semantics

- `appliedEffectCount <= 1` is enforced even though the TLA TypeOK type domain permits 0..2; the 2 state is an explicit invariant counterexample.
- Indeterminate persistence cannot coexist with an issued/consumed recovery permit.
- An issued permit requires observed NoEffect, mission resolution NoEffect, a resolved persistence outcome, current M1/M2 and both current fences.
- A fresh ticket is bound to the current boot/controller epoch and a new attempt.
- After `NativeAcceptRecovery`, the model deliberately clears `downstreamFenceBootEpoch` to allow the recovery command to execute. The projection validates the captured `admittedDownstreamFence` fact, not a stale current-fence value.
- Current M1/M2 values may be revoked after recovery admission; the captured admission fields remain the evidence of what was true at acceptance time.
- Source clock values are retained with clock identity, boot epoch, unit, resolution and uncertainty. Incomparable clocks are never ordered by timestamp alone.

## Fixture

- `fixtures/runtime_projection_init_v0_1.json`: exact initial model state with ten not-yet-observed owner evidence domains.
- `fixtures/runtime_projection_reservation_acceptance_v0_1.json`: four-state `Init → ReserveAttempt → MarkDispatchStarted → NativeAcceptOld` trace, with all 43 model fields and ten owner evidence domains at each step.

These fixtures are synthetic conformance inputs, not observations of a real robot.

## Claim ceiling

A projection PASS establishes only that a supplied recording satisfies this crate's typed shape, scope, provenance and selected invariant checks. It does **not** authenticate owner artifacts, prove that an event inventory is complete, establish model transition conformance, prove physical causality, qualify hardware, or establish a safety case. Use independent owner authentication, a formal trace checker, ROS/simulator/HIL/physical campaigns and applicable safety standards for those claims.
