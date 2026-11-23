# RRVR runtime projection: research positioning and benchmark plan v0.1

**Implementation:** `research/mobility-rrvr-runtime-trace/`  
**Tracking:** [MOBILITY-RRVR-RUNTIME-TRACE-001 #6919](https://github.com/Luminous-Dynamics/symtropy/issues/6919)  
**Formal model:** [RestartSafeEffectContinuity.tla](../mobility-restart-reconciliation/model/RestartSafeEffectContinuity.tla)

## 1. What already exists in the field

RRVR must not claim novelty for formal specification, model checking, recorded-trace validation, ROS event tracing, or policy recovery benchmarks.

| Baseline | Established capability | What RRVR should add (not replace) |
|---|---|---|
| TLC / TLA+ | Bounded/exhaustive state exploration, invariant checking and counterexample traces | Owner-evidence semantics for physical effects, persistence, fencing and recovery authority |
| Reflow `tla.trace` and research trace-validation frameworks | Validate recorded implementation states/actions against a TLA+ specification | Exact effect/attempt/epoch lineage; physically grounded owner receipts; evidence completeness and observer fault model |
| `ros2_tracing` / LTTng | Low-overhead ROS 2 runtime tracepoints and causal runtime analysis | Semantically connect message/callback traces to physical outcomes and downstream graph quiescence |
| NIST Performance Assessment Framework for Robotic Systems | Task-driven component test methods for perception, mobility, dexterity and safety, composed into system-level performance models | Add recovery authorization correctness, physical-effect continuity and recovery evidence quality |
| RoboRecover (2026) | Policy recovery from execution-induced intermediate states across RoboTwin and LIBERO | Add a safety/evidence overlay to the same recovery points: determine whether an attempted recovery is actually authorized and whether the physical task effect is known |

### Closest policy-recovery SOTA: RoboRecover

RoboRecover is the closest current benchmark for recovery-policy capability, and RRVR should use it as a complementary substrate rather than claim to beat it with a trace checker.

Its September 24, 2026 preprint contains 2,000 scenarios: 1,000 each on RoboTwin and LIBERO, with fixed 800/200 train/test splits on each platform. It evaluates six policies per platform across four recovery stages and four deviation types, with nine Stage–Type combinations represented in each test set. Its protocol replays the action prefix so candidate policies start from the same reconstructed recovery condition.

Published results:
- LIBERO best fixed-policy overall Recovery Success Rate (RSR): `64.40%` (`π0.5`); UnifoLM has `98.83%` initial-state success but only `48.00%` recovery RSR.
- RoboTwin highest fixed-policy Overall RSR: `59.50%` (LingBot-VA); highest category-balanced Macro-9 RSR: `55.62%` (Fast-WAM).
- A retrospective per-scenario best-of-six oracle reaches `95.63%` on RoboTwin and `91.80%` on LIBERO. This is an oracle upper reference, not a deployable policy.
- LIBERO interventions raise direct `π0.5` recovery RSR from `64.4%` to `77.0%` with Monitor+Reversion; Monitor+Correction reaches `75.5%`. Reversion changes the starting state; Correction introduces a short external controller, so these are not like-for-like fixed-policy comparisons.

The key conclusion is not that RRVR must produce a higher RSR. It is that recovery success and recovery authorization are different questions. A controller can produce a successful trajectory from an execution-deviation state while the system still lacks evidence that an earlier physical effect did not occur or that an old command is quiescent. RRVR should report policy RSR and evidence-gate correctness separately.

**Reproducibility/access qualification (checked 23 November 2026):** the [public RoboRecover repository](https://github.com/RUCKBReasoning/RoboRecover) documents replay adapters, fixed train/test starting-point splits and per-file SHA-256 checksums. Its README also states that the complete scenario archive is still being prepared for Hugging Face and that full evaluation currently requires an archive exported by the authors. The README lists six LIBERO adapters but only four included RoboTwin recovery adapters; FastWAM and LingBot-VA have configuration guidance while their RoboTwin recovery entrypoints are pending. Its dependency manifest lists source revisions but marks historical revision verification incomplete. Therefore the benchmark plan has an explicit access gate: obtain the authorised archive, verify the exact archive and per-scenario hashes, simulator/model revisions and licenses/terms, then smoke-test one LIBERO and one RoboTwin sample in clean pinned environments. No result may be called a RoboRecover reproduction until those steps succeed. If access remains unavailable, use a clearly separate synthetic RRVR fault overlay and report the external-data gap rather than substituting scenarios.

Sources:
- [RoboRecover, arXiv:2609.28952](https://arxiv.org/abs/2609.28952), posted 24 September 2026; see its fixed split and task coverage, matched recovery baselines, stage/type comparisons and intervention results.
- [NIST Performance Assessment Framework for Robotic Systems](https://www.nist.gov/programs-projects/performance-assessment-framework-robotic-systems): composes task-specific perception, mobility, dexterity and safety component measures into integrated assembly performance models.
- [Reflow TLA+ plugin](https://reflow.io/docs/plugins/tla-plus/) and [TLA+ trace validation](https://docs.tlapl.us/using:tlc:trace_validation): prior art for recorded-trace/model compatibility.
- [Cirstea et al., Validating Traces of Distributed Programs Against TLA+ Specifications, SEFM 2024](https://doi.org/10.1007/978-3-031-77382-2_8): reduces compatibility of recorded distributed-program traces with a TLA+ specification to constrained TLC model checking.
- [ros2_tracing, IEEE RA-L 2022](https://doi.org/10.1109/LRA.2022.3174346): the paper reports 0.0033 ms average end-to-end message-latency overhead with its full ROS 2 instrumentation in its tested workload. This is a reference measurement, not a universal guarantee or a directly comparable RRVR projection target.

## 2. The contribution we should try to establish

The research hypothesis is that a runtime/model bridge becomes materially more useful for physical robotics when it carries *evidence-qualified physical-effect identity* rather than only event names and state snapshots.

**Paradigm shift:** treat a robot operation as an externally consequential transaction whose physical effect may outlive the process that initiated it. Recovery is not a policy retry; it is a proof-carrying admission decision. The system may resume mutation only after reconciling (a) the logical effect and attempt identity, (b) persistence outcome, (c) the old controller queue, (d) all downstream driver/firmware work, (e) independent physical outcome evidence, and (f) the authority/binding state for the new attempt. When any of these is unknown, partial, disputed, stale or mismatched, the safe result is a hold plus reconciliation—not another physical command.

This leads to a two-plane architecture:
- **Operational plane:** low-latency runtime guard at the native acceptance boundary. It consumes typed, scoped owner receipts and one-shot recovery authority before admitting a new mutation.
- **Assurance plane:** append-only provenance, exact action/state trace replay through pinned TLC, counterexample preservation, and independent simulator/HIL/physical audit.

Neither plane can replace the other. Fast guards do not establish their own correctness; offline model checking does not authenticate physical observations.

RRVR's candidate differentiators:
- stable logical-effect ID that survives retries;
- exact attempt ID and original boot/controller epoch;
- separate durable-persistence verdict and physical outcome;
- downstream execution-graph fence evidence, not just controller queue acknowledgement;
- two non-interchangeable observer claims: physical physical-outcome and downstream graph quiescence;
- an explicit fault-model revision and observer dependency graph for common-cause qualification;
- one-shot recovery authority, live-binding and M1/M2 freshness captured at admission;
- exact raw owner receipt/artifact commitments and source clock identity;
- an exact owner-asserted value per evidence record, bound into the claim-scope digest;
- a separate exact-action/exact-successor-state TLC trace harness plus a deliberately mutated wrong-action negative control.

The projection deliberately refuses to turn `unknown`, `partial`, `disputed`, stale, wrong-attempt or missing owner evidence into a safe state. It also does not infer that a state/action sequence conforms to the TLA model; that requires a separate formal trace checker.

### Runtime tracing performance baseline

The published `ros2_tracing` evaluation reports an average end-to-end message latency overhead of 0.0033 ms with all ROS 2 instrumentation enabled in its tested message-publish/subscribe workload. This is a useful published reference point, not a transferable guarantee for other ROS distributions, robot workloads, hardware, storage settings or trace configurations. The ROS 2 benchmarking recommendation (REP-2014) advocates grey-box, non-functional benchmarking of individual nodes and complete computation graphs with LTTng/`ros2_tracing`.

RRVR should therefore benchmark its projection and evidence sidecar separately from event collection:
- tracepoint/collector overhead relative to tracing-disabled execution;
- projection CPU and allocations per snapshot;
- provenance/hash work per owner record and observer member;
- durable-writer and upload overhead;
- total p50/p95/p99 event-to-qualified-verdict latency;
- dropped/incomplete event rates at a declared peak rate.

Do not compare a pure Rust JSON projection microbenchmark directly with the 0.0033 ms end-to-end ROS message-latency result. Run a paired ROS 2 workload with the same instrumentation enabled/disabled, then separately report per-stage overhead.

Sources: [Bédard et al., ros2_tracing, IEEE RA-L 2022](https://doi.org/10.1109/LRA.2022.3174346); [ROS 2 REP-2014, performance benchmarking methodology](https://reps.openrobotics.org/rep-2014/).

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

v0.1 covers one original effect attempt plus one recovery-authority admission. It does not yet qualify the physical outcome of the recovery attempt itself or nested/multiple recovery cycles. The next semantic increment must give each successive attempt its own physical-outcome, persistence, mission, controller-queue and downstream-graph evidence domains while preserving the original effect lineage.

A passing Rust fixture establishes only the projection contract over supplied synthetic inputs. A Reflow/TLC trace pass establishes only that the recorded state/action sequence is a behavior of the pinned abstraction. Neither proves event capture is complete, owner artifacts are authentic, observer independence holds outside the declared fault model, or the TLA abstraction matches physical hardware. Those require independent owner authentication, fault campaigns, simulator/HIL/physical evidence and safety review.


## 8. Multi-attempt recovery as a distinct benchmark dimension

The v0.1 runtime projection has a disclosed one-recovery-admission ceiling. A robot that safely reaches a second attempt has not yet demonstrated safe recovery if that second attempt itself crashes or produces an ambiguous effect. The attempt-lineage reference contract adds a separately measurable dimension: preservation of one logical-effect identity across repeated, individually evidenced attempts.

### Benchmark arms

| Arm | Recovery-history representation | What it can miss |
|---|---|---|
| L0 | Last-attempt-only record | Prior effects and stale attempts are discarded |
| L1 | Flat attempt list without parent links | Evidence can be attached to the wrong predecessor |
| L2 | Parent-linked attempts with logical-effect ID | Does not necessarily qualify persistence and graph fences |
| L3 | RRVR attempt lineage with per-attempt scoped receipts | Higher validation and evidence-retention cost; still depends on owner completeness and authenticity |

All arms receive the same frozen chain, epoch changes, physical outcomes, persistence states, and injected faults. A policy-recovery score must remain separate from this evidence/authorization dimension.

### Mandatory chain faults

- A descendant follows Applied, Partial, Unknown, Disputed, or NotObserved.
- A NoEffect claim is only InsufficientEvidence or is Refuted.
- Persistence is Committed, InFlight, or IndeterminateCommit.
- The child names the wrong parent, skips an ordinal, or reuses an attempt ID.
- Evidence is replayed across attempts or carries the wrong logical-effect/attempt/epoch scope.
- Boot/controller epochs regress or do not advance together.
- Controller or downstream fence is missing, non-quiescent, unconfirmed, or targeted at a different child epoch.
- The same logical effect is split across separate identifiers.

### Proposed targets (not achieved)

- Zero accepted descendant attempts after any non-NoEffect or unqualified NoEffect outcome.
- Zero accepted retry edges with InFlight or IndeterminateCommit persistence.
- Zero accepted evidence receipts with cross-attempt or cross-epoch scope leakage.
- 100% rejection of the frozen unsafe-lineage controls and 100% acceptance of the frozen valid reference chains.
- Validation cost grows linearly with attempt count; report p50/p95/p99 runtime and bytes per attempt rather than hiding cost in a composite score.
- Report false holds separately from false-safe acceptance. No aggregate score may trade a false-safe admission for a better recovery-success rate.

### Qualification boundary

The current Rust contract is a bounded reference validator and its three-attempt fixture is synthetic. It does not yet refine the TLA+ state machine for nested restart/recovery, and it does not authenticate raw receipts. The next formal step is a bounded multi-attempt TLA+ model with a second recovery crash cut, followed by differential testing of Rust decisions against all reachable model traces. Only after that should the benchmark be adapted to a declared simulator and then an instrumented ROS/controller/driver stack.
