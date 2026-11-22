# RRVR runtime projection: research positioning and benchmark plan v0.1

**Implementation:** `research/mobility-rrvr-runtime-trace/`  
**Tracking:** [MOBILITY-RRVR-RUNTIME-TRACE-001 #6919](https://github.com/Luminous-Dynamics/symtropy/issues/6919)  
**Formal model:** [RestartSafeEffectContinuity.tla](../mobility-restart-reconciliation/model/RestartSafeEffectContinuity.tla)

## 1. What already exists in the field

RRVR must not claim novelty for formal specification, model checking, recorded-trace validation, ROS event tracing or conventional robot performance measurement.

| Baseline | Established capability | Gap RRVR targets |
|---|---|---|
| TLC / TLA+ | Exhaustive bounded state exploration, invariant checks and counterexample traces | Map physical owner evidence and effect identity into the model |
| Reflow `tla.trace` | Validate each recorded state/action step as behavior of a TLA+ specification | Separate attempt/effect identity, independent no-effect and graph-quiescence claims, owner receipts |
| TLA+ trace validation research | Map runtime implementation states to specification states; can expose implementation/specification divergence | Domain-specific evidence qualification and common-cause-aware observer proofs |
| `ros2_tracing` / LTTng | Low-overhead ROS 2 runtime event tracepoints and trace analysis | Semantically bind middleware events to physical outcomes, exact attempt IDs, downstream graph fences and authority |
| NIST robotic performance assessment | Component/system test methods for perception, mobility, dexterity and safety | Recovery correctness, evidence quality and irreversible-effect handling |
| Long-horizon manipulation benchmarks | Task progress, partial completion, error propagation, recovery cost and horizon sensitivity | Evidence-qualified physical-effect continuity across restarts |

Sources and prior art:
- [Reflow TLA+ plugin](https://reflow.io/docs/plugins/tla-plus/): `tla.trace` requires the initial state and validates each later transition; the plugin distinguishes specification checking from checking implementation traces.
- [TLA+ trace validation](https://docs.tlapl.us/using:tlc:trace_validation): recorded implementation states can be compared with a specification; this checks the captured subset of executions, not every possible execution.
- [ros2_tracing](https://github.com/ros2/ros2_tracing): tracepoints for ROS 2 and supported middleware, with Trace Compass analysis.
- [NIST Performance Assessment Framework for Robotic Systems](https://www.nist.gov/programs-projects/performance-assessment-framework-robotic-systems): perception, mobility, dexterity and safety component measurements composed into system performance models.
- Zheng et al., [Long-horizon embodied manipulation: a survey of methods, trends, and challenges](https://doi.org/10.1007/s44336-026-00048-x), 16 November 2026: identifies partial completion, error propagation, recovery cost and horizon-dependent degradation as shortcomings of binary success reporting.
- Al-Batati et al., [ROS 2 in a Nutshell: A Survey](https://doi.org/10.1145/3815113), 21 August 2026: analyzes ROS 2 middleware, real-time scheduling, security/safety and distributed/multi-robot systems.

## 2. The contribution we should try to establish

The research hypothesis is that a runtime/model bridge becomes materially more useful for physical robotics when it carries *evidence-qualified physical-effect identity* rather than only event names and state snapshots.

RRVR's candidate differentiators:
- stable logical-effect ID that survives retries;
- exact attempt ID and original boot/controller epoch;
- separate durable-persistence verdict and physical outcome;
- downstream execution-graph fence evidence, not just controller queue acknowledgement;
- two non-interchangeable observer claims: physical physical-outcome and downstream graph quiescence;
- an explicit fault-model revision and observer dependency graph for common-cause qualification;
- one-shot recovery authority, live-binding and M1/M2 freshness captured at admission;
- exact raw owner receipt/artifact commitments and source clock identity.

The projection deliberately refuses to turn `unknown`, `partial`, `disputed`, stale, wrong-attempt or missing owner evidence into a safe state. It also does not infer that a state/action sequence conforms to the TLA model; that requires a separate formal trace checker.

## 3. Benchmark design

Use one frozen corpus of runtime schedules and run each schedule through the following arms:

| Arm | Projection/evaluation | Expected limitation |
|---|---|---|
| B0: event log | Raw ROS 2 / application event trace | No explicit physical-effect semantics or owner completeness |
| B1: naive model mapping | Map event names to TLA states/actions | Can accept stale/wrong-attempt evidence or aggregate unrelated claims |
| B2: typed evidence projection | Exact owner domains, effect/attempt scope, persistence and graph receipts | No explicit common-cause-qualified observer proof |
| B3: full RRVR projection | B2 plus two separate quorum proofs, common-cause dependencies, permit/admission lineage | More evidence, validation and storage overhead; strongest trust semantics |

All arms must use the same model revision, action schedule, reset conditions, fault profile, raw input events and reference oracle. Do not compare a synthetic model trace against a physical trace as if they were equivalent.

### Mandatory metrics

1. **False-safe projection rate:** unsafe/unknown/partial/disputed/wrong-attempt records accepted as safe. Stretch target: 0.
2. **Evidence completeness:** required owner domains present, current, correctly scoped and artifact-backed.
3. **Attempt/effect scope leakage:** evidence accepted for another logical operation or attempt. Target: 0.
4. **Observer false-independence rate:** quorum marked qualified despite shared declared dependencies or reused observers/artifacts. Target: 0.
5. **Model trace conformance:** accepted state/action traces that pass the independent TLA trace checker; report both false acceptance and false rejection.
6. **Invariant fault detection:** duplicate effects, unknown persistence outcome, old command after downstream fence, revoked M1/M2, replayed permit and binding mutation.
7. **Diagnosis utility:** first-divergence localization accuracy and time for an independent engineer.
8. **Operational overhead:** projection CPU, allocations, memory, storage bytes per event, capture latency, p50/p95/p99 and dropped-event rate.
9. **Clock correctness:** cross-clock ordering assertions attempted, invalid clock epochs, uncertainty propagation and missing clock metadata.
10. **Reproducibility:** byte-identical normalized trace/sidecar under fixed inputs and stable replay context.

Do not collapse these into a weighted score. A higher success or throughput score cannot compensate for one false-safe acceptance.

## 4. Fault campaign

Build a reproducible factorial fault matrix around the actual RRVR lifecycle:
- process crashes before/after reservation, dispatch, native acceptance and physical application;
- persistence commit success, explicit no-commit, timeout/indeterminate commit and delayed acknowledgement;
- controller queue fenced while downstream driver/firmware remains pending;
- old command executes after a claimed fence;
- physical outcome Applied, NoEffect, Partial, Unknown and Disputed;
- shared power, clock, host, network, firmware, calibration and sensor dependencies among purportedly independent observers;
- M1/M2 revocation and whole-live-binding mutation after ticket preparation;
- recovery permit replay, double use, cross-attempt and cross-boot substitution;
- capture loss, duplicate event IDs, clock reset, clock drift and out-of-order arrival.

For each fault family, publish the exact injected schedule, frozen state/config, owner artifacts, expected invariant, observed verdict and independent reproduction result.

## 5. Targets (proposed, not achieved)

### Contract correctness
- 100% of the pinned 43 TLA state fields accepted with exact names/types.
- 100% of the ten owner domains required at every snapshot.
- 0 false-safe projections in the frozen negative-control corpus.
- 0 acceptance of evidence whose logical-effect/attempt/epoch scope does not match.
- 0 observer quorum qualifications with overlapping declared common-cause domains or reused observer/artifact identities.
- 100% deterministic output for identical recording bytes and context.

### Formal conformance
- 100% of the frozen valid action schedules pass the independent `tla.trace`/TLC replay gate.
- 100% of malformed or invariant-violating schedules are rejected by the formal checker.
- 0 claim that projection validation alone constitutes a model-conformance pass.

### Operational performance
- Stretch target: projection-only validation p99 below 1 ms per snapshot on a named reference host.
- Stretch target: no dropped events at the benchmark's declared peak event rate.
- Report real instrumentation overhead against a tracing-disabled baseline; do not use the ROS tracing project's low-overhead characterization as a substitute for our measurement.
- Report CPU, memory, storage and latency distributions by event rate and number of owner/quorum records.

### Real-stack progression
1. synthetic known-answer fixtures;
2. TLC/Reflow trace conformance;
3. simulator instrumented trace with exact revision and reset conditions;
4. ROS 2 trace adapter using `ros2_tracing` metadata and event sources;
5. controller/driver/firmware receipts and downstream graph fence evidence;
6. HIL with calibrated independent observation;
7. physical trials with predeclared safety case, hazards, abort conditions and independent review.

## 6. Standards alignment and limits

- [ISO 10218-1:2025](https://www.iso.org/standard/73933.html) addresses industrial robots as machines.
- [ISO 10218-2:2025](https://www.iso.org/standard/73934.html) addresses integration of industrial robot applications and cells.
- [ISO 21423:2026](https://www.iso.org/standard/86749.html) specifies communication/interoperability for industrial mobile robots and related systems; the ISO scope explicitly says it is not safety-related.
- [ISO 18646-5:2026](https://www.iso.org/standard/86850.html) is a legged-robot locomotion performance standard, not a safety-validation substitute.

RRVR software evidence is not ISO conformity assessment, a risk assessment, a safety-rated controller or a substitute for required protective measures. It can contribute traceability and verification evidence only within an independently reviewed safety lifecycle.

## 7. Claim ceiling

A passing Rust fixture establishes only the projection contract over supplied synthetic inputs. A Reflow/TLC trace pass establishes only that the recorded state/action sequence is a behavior of the pinned abstraction. Neither proves event capture is complete, owner artifacts are authentic, observer independence holds outside the declared fault model, or the TLA abstraction matches physical hardware. Those require independent owner authentication, fault campaigns, simulator/HIL/physical evidence and safety review.
