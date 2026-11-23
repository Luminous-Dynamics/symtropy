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

### Safety baseline: SafeVLA-Bench

SafeVLA-Bench demonstrates the complementary point that task completion and safety are separate dimensions. Its public leaderboard reports 24 policies, five evaluation suites, 22,500 episodes and eight safety specifications (last updated 26 September 2026). It ranks by the share of episodes satisfying every applicable specification, while reporting native task success (SR), successful-but-unsafe episodes (SBU) and violation severity (VSI) separately.

Two examples make the measurement problem concrete:
- On LIBERO, VLA-Adapter reports 97.6% task success alongside 82.0% safety and 17.0% successful-but-unsafe episodes.
- On RoboCasa-365 Atomic-Seen, Xiaomi-Robotics-1 reports 81.9% task success, 50.1% safety and 37.7% successful-but-unsafe episodes.

These are SafeVLA's own leaderboard quantities, not RRVR results. RRVR should preserve SafeVLA-style safety/task-success separation, then add a separate recovery-evidence dimension: did the system know what the previous attempt physically did, did every command-holding path become quiescent, and was the next attempt admitted only after those claims were independently supported? An unsafe-but-successful policy episode and a false-safe recovery admission are different failure classes and should be reported separately.

Source: [SafeVLA-Bench leaderboard](https://safevla.org/leaderboard), including its definitions of SR, safety, SBU and VSI; leaderboard snapshot dated 26 September 2026.

### Emerging evaluation gap: recovery state, safety, and evidence continuity

Three 2026 results make the correct RRVR position clearer:

1. **Policy recovery is now a benchmark dimension.** RoboRecover constructs execution-induced intermediate states by replaying action prefixes, then evaluates whether a policy can continue the original task. Its 2,000-scenario RoboTwin/LIBERO corpus is a strong policy substrate. It does not, by itself, answer whether a previous physical operation was durably reconciled, whether a stale command remained in a driver queue, or whether a new attempt was independently authorized. RRVR should overlay the same recovery points rather than create a competing policy score.
2. **Safety is not task success.** SafeVLA-Bench reports SR, Safety, successful-but-unsafe (SBU) and violation severity separately. Its September 2026 leaderboard includes RoboCasa-365 Xiaomi-Robotics-1 at 81.9% SR, 50.1% Safety and 37.7% SBU; on LIBERO, VLA-Adapter reports 97.6% SR and 82.0% Safety. These are not RRVR comparisons; they demonstrate why a single completion metric cannot stand in for a safety verdict.
3. **Fuzzing needs repeatable execution and diagnosable failures.** The July 2026 ROS-based robotics fuzzing survey screened 92 works and says results remain difficult to compare because tested ROS layers and failure definitions differ. It finds more visible progress in input modeling, feedback and oracle construction than in repeatable execution and diagnosable failure analysis. That is a direct opening for exact replay bundles and causal minimization—but only if RRVR reports the tested ROS/system layer and its evidence scope instead of flattening package crashes, middleware failures, simulator faults and physical-task failures into one category.

The November 16, 2026 long-horizon manipulation survey adds a complementary measurement argument: binary task success can hide partial completion, error propagation, recovery cost and horizon-dependent degradation. RRVR should preserve an attempt-by-attempt physical-effect ledger and report how uncertainty, blocked progress and recovery cost accumulate over the complete physical interaction horizon.

### Proposed RRVR diagnostic evidence package

Every benchmark failure should produce a replayable package containing:

- immutable run/episode ID, logical-effect ID, attempt ancestry and exact boot/controller epochs;
- simulator, ROS distribution, middleware/DDS, controller, driver, firmware, calibration and model/config revisions;
- exact injection schedule, random seed, event clock model and declared observation loss/reordering;
- per-owner receipt IDs, scopes, raw artifact IDs/digests and the applicable fault-model revision;
- the exact safety/recovery invariant, TLA action/state witness, and three-valued verdict;
- minimized trace with the failure signature preserved, retained happens-before ancestors, the number of oracle calls, and a deterministic replay result;
- explicit claim ceiling: simulated, model-checked, HIL, physical, independently reviewed, or not established.

A minimized trace is not enough if it changes the failure. The minimizer must keep the exact invariant/property, violation class and witness event (or an explicitly defined equivalent signature), and the output must replay under the frozen oracle/configuration. Report:
- **Replay-complete rate:** fraction of discovered failures reproduced from the published bundle in a clean environment;
- **Signature-preserving reduction:** original events versus retained events, only for reductions that preserve the exact failure signature;
- **Minimization determinism:** identical minimized bundle over repeated oracle runs;
- **Diagnosis yield:** fraction of bundles that localize the failing layer/owner without manual reconstruction;
- **Oracle budget:** calls, wall time and resource use per minimized failure.

These are diagnostic-quality metrics, not safety scores. A benchmark may not claim "reproducible" merely because an event log exists.

### Instrumentation and comparability

ROS 2 REP 2014 recommends a grey-box, non-functional benchmarking approach that measures both individual nodes and complete computational graphs, using LTTng/ros2_tracing for architecture-neutral runtime instrumentation. RRVR should adopt that measurement discipline and add the semantic boundaries REP 2014 does not itself define: durable effect identity, per-attempt owner receipts, downstream physical command holders and independent physical observation. Publish raw traces and instrumentation overhead against a tracing-disabled baseline.

Use a layered fault taxonomy so each result is comparable only to evidence at the same level:

| Layer | Representative failures | Required evidence |
|---|---|---|
| Package/process | crash, restart, duplicate process | process/boot epoch and durable owner state |
| ROS graph/runtime | executor delay, callback loss, lifecycle transition | node, callback, topic/service/action and clock trace |
| Middleware/network | DDS partition, reconnect, duplicate/reordered delivery | transport identity, sequence and delivery receipts |
| Controller/driver | queue retention, stale accepted command, watchdog reset | per-layer queue/epoch receipt and hardware counter |
| Firmware/actuator | buffered command after host restart, latch/drive state | device-owned state/receipt and injected fault observation |
| Physical task | partial contact, object moved, effect already applied | independent physical observer and mission predicate |

A Confirmed result at one row must not be promoted to a claim about a lower or higher row without the corresponding receipt. In particular, a ROS topic being quiet is not proof that firmware has no buffered motion command, and a controller reporting "stopped" is not proof that a physical effect did not occur.

Sources:
- [RoboRecover, arXiv:2609.28952](https://arxiv.org/abs/2609.28952), 24 September 2026.
- [SafeVLA-Bench leaderboard](https://safevla.org/leaderboard.html), snapshot 26 September 2026.
- [Yang et al., A Survey of Fuzzing for ROS-Based Robotic Systems](https://doi.org/10.3390/app16147250), Applied Sciences 16(14), 7250, published 20 July 2026.
- [Zheng et al., Long-horizon embodied manipulation: a survey of methods, trends, and challenges](https://doi.org/10.1007/s44336-026-00048-x), published 16 November 2026.
- [ROS 2 REP 2014: Methodology for benchmarking performance in ROS 2](https://reps.openrobotics.org/rep-2014/).

### Prior art: receipt-gated effectuation is not a new claim

The October 2026 Internet-Draft [A Permit to Effect Once Is Not a Permit to Effect Permanently](https://datatracker.ietf.org/doc/html/draft-das-safety-first-execution-finality-00) already discusses staged physical effectuation, single-use authority, receipts, anti-replay, independent observation and recovery from indeterminate/partial/conflicting outcomes. It is an active individual Internet-Draft with intended status Experimental, not an IETF-endorsed standard; the draft itself says it has no formal standing in the IETF standards process and expires 4 April 2027.

Therefore RRVR should not claim novelty for receipt-gated or staged effectuation, one-shot permits, anti-replay, or distinguishing Applied/NoEffect/Partial/Unknown/Disputed. The narrower research hypothesis is whether these semantics can be made executable across separately owned robot persistence, mission resolution, controller queues, downstream driver/firmware graphs, independently qualified observers and the existing native authority boundary—with exact attempt lineage, model-trace conformance and reproducible fault-injection evidence. That integration claim also remains to be demonstrated against a real declared stack.

Source: [IETF Datatracker, draft-das-safety-first-execution-finality-00](https://datatracker.ietf.org/doc/html/draft-das-safety-first-execution-finality-00), published 1 October 2026, Experimental individual Internet-Draft.

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

### Paired policy-plus-assurance evaluation

Use RoboRecover-style reconstructed intermediate states as the recovery-policy substrate and SafeVLA-style trajectory specifications for physical-safety scoring. Keep the candidate policy, initial scene, action prefix, simulator revision, random seed and injected deviation identical between paired arms.

| Arm | Recovery policy | Recovery admission | Purpose |
|---|---|---|---|
| P0 | Fixed policy | Original benchmark behavior | Native recovery-success reference |
| P1 | Same policy | Blind/naive retry after process restart | Quantify why a policy-only retry is insufficient |
| P2 | Same policy | Typed owner evidence, without independent observer/fence gates | Isolate the value of evidence structure |
| P3 | Same policy | Full RRVR evidence, observer and graph-fence gate | Measure the incremental assurance effect |
| P4 | Same policy | Full RRVR plus injected observer/common-cause and downstream queue faults | Test adversarial robustness, not just nominal replay |

Do not silently count a blocked recovery as a successful task. Report (a) native policy RSR, (b) trajectory safety, (c) RRVR false-safe admissions, (d) RRVR false holds on valid positive controls, (e) duplicate physical effects, (f) stale commands after accepted quiescence, and (g) time/throughput costs. The desired result is not “RRVR has the highest RSR”; it is “the same recovery policy is prevented from making unsupported or duplicate physical mutations while valid, independently evidenced recovery remains available.”

### Statistical qualification

A one-sided 95% upper confidence bound below 0.1% after zero observed failures requires approximately 2,995 independent representative Bernoulli trials. This is a planning reference under an IID assumption, not a blanket safety demonstration. Robotics campaigns commonly contain clustered observations: repeated rollouts may share a controller build, firmware, sensor, calibration, environment, simulator seed family or power/clock failure domain. Treating every correlated rollout as independent would overstate the evidence.

Therefore:
- publish exact denominators and the unit of independence (episode, hardware unit, controller/firmware build, campaign, or independent fault realization);
- stratify by platform, controller/driver/firmware revision, sensor family, calibration, environment, fault class and common-cause domain;
- report per-stratum results and cluster-aware intervals alongside pooled estimates;
- use model checking, exhaustive declared cut coverage, adversarial fault injection and mutation controls for rare structural failures; do not try to establish their absence by random trials alone;
- preserve zero-tolerance requirements for false-safe admissions and duplicate effects even when performance intervals are reported.

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
- At least 99.9% admission of valid positive controls within a declared platform profile, reported separately from safety failures.
- 100% coverage of declared crash cuts, command-holding graph nodes, observer fault scenarios and recovery transitions in the frozen campaign.
- Zero false-safe admissions and zero duplicate physical effects across the frozen negative-control corpus.
- A stretch target of at least 25% lower verified mission makespan through safe concurrency, measured only after every hard safety and evidence gate passes; no latency gain may compensate for an invalid admission.

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
| L3 | RRVR attempt lineage with per-attempt scoped receipts plus two fault-model-qualified observer quorums for each retryable parent | Higher validation and evidence-retention cost; still depends on owner completeness and authenticity |

All arms receive the same frozen chain, epoch changes, physical outcomes, persistence states, and injected faults. A policy-recovery score must remain separate from this evidence/authorization dimension.

### Mandatory chain faults

- A descendant follows Applied, Partial, Unknown, Disputed, or NotObserved.
- A NoEffect claim is only InsufficientEvidence or is Refuted.
- Persistence is InFlight or IndeterminateCommit (a resolved Committed result is admissible only alongside independently confirmed physical and mission NoEffect).
- The child names the wrong parent, skips an ordinal, or reuses an attempt ID.
- Evidence is replayed across attempts or carries the wrong logical-effect/attempt/epoch scope.
- Physical-outcome observer evidence is absent for a confirmed outcome; or a retryable parent lacks separate downstream-quiescence observer evidence, has scope tampering, a different fault profile, shared declared common-cause dependencies, reuses an operational owner as an observer, or replays owner receipts, claim IDs, observation IDs or raw artifacts. Reusing a sensor across attempts with a new observation and raw artifact is a positive control, not a replay fault.
- Boot/controller epochs regress or do not advance together.
- Controller or downstream fence is missing, non-quiescent, unconfirmed, or targeted at a different child epoch.
- The same logical effect is split across separate identifiers.

### Proposed targets (not achieved)

- Zero accepted descendant attempts after any non-NoEffect or unqualified NoEffect outcome.
- Zero accepted retry edges with InFlight or IndeterminateCommit persistence.
- Zero accepted evidence receipts with cross-attempt or cross-epoch scope leakage.
- Zero retry edges admitted without both claim-specific observer quorums qualified under the same fault-model revision and independent across declared common-cause dependencies.
- 100% rejection of the frozen unsafe-lineage controls and 100% acceptance of the frozen valid reference chains.
- Validation cost grows linearly with attempt count; report p50/p95/p99 runtime and bytes per attempt rather than hiding cost in a composite score.
- Report false holds separately from false-safe acceptance. No aggregate score may trade a false-safe admission for a better recovery-success rate.

### Qualification boundary

The current Rust contract is a bounded reference validator and its three-attempt fixture is synthetic. Every typed state is bound to the same OwnerEvidenceRef that supplies the owner, verdict, asserted value, exact scope and artifact digest; a mutated outcome without a matching owner assertion is rejected. It reuses the existing observer-quorum evaluator for NoEffect and downstream quiescence and verifies exact claim-scope commitments, but it does not yet refine the TLA+ state machine for nested restart/recovery or authenticate raw receipts. The next formal step is a bounded multi-attempt TLA+ model with a second recovery crash cut, followed by differential testing of Rust decisions against all reachable model traces. Only after that should the benchmark be adapted to a declared simulator and then an instrumented ROS/controller/driver stack.


## 9. Paired campaign and evidence-debt extension

The benchmark must not let an assurance arm receive an easier scene, shorter action prefix, friendlier reset, different controller build or different fault schedule. The new manifest validator enforces pair equivalence over policy/checkpoint, task, scene, action-prefix, platform/environment/reset profile, random seed, stable logical-effect ID and ordered fault schedule. Only the arm (last-attempt-only, flat-history, owner-receipts, full-RRVR) may differ within a pair.

The first reference fixture contains:
- one positive pair: full-RRVR must admit a valid fully reconciled control; the last-attempt-only comparison is measured;
- one negative pair: full-RRVR must block an unsafe recovery schedule; the last-attempt-only comparison is measured;
- four injected faults across process, observer, controller and firmware layers and four distinct lifecycle cut points.

### Evidence debt

For each logical effect, define the unresolved interval from the first point where a consequential effect might have occurred until independent evidence resolves it as Applied, NoEffect, Partial, Unknown/Disputed or otherwise terminal under the declared policy.

Publish:
- unresolved-effect count over time;
- oldest unresolved-effect age;
- unresolved-effect seconds per 100 physical actions;
- p50/p95/p99 evidence-clearance time;
- number and duration of safe holds caused by unresolved evidence;
- false-safe admissions and false holds on valid positive controls alongside the debt measures.

This avoids a common metric trap: a system that instantly retries can show low apparent recovery latency by deleting uncertainty from its accounting. Conversely, a correct fail-closed hold can increase unresolved time while preventing duplicate effects. Evidence debt is an availability/reconciliation measure, not a score to minimize by relaxing safety.

### Updated external comparison (23 November 2026)

- The [RoboCasa365 leaderboard](https://robocasa.ai/leaderboard), updated 22 November 2026, lists 50 tasks, 25 models and three evaluation splits; its listed top Overall result is 85.6 (HexaShell). This is task success and generalization, not recovery assurance. A fair RRVR evaluation would inject matched faults into the same 50-task subset and report policy task success, SafeVLA-style safety, and RRVR evidence-gate correctness separately.
- [SafeVLA-Bench](https://safevla.org/leaderboard.html) evaluates 24 policies over five suites and 22,500 episodes, ranking by safety rather than native task success and separately reporting successful-but-unsafe (SBU) and violation severity. For example, Xiaomi-Robotics-1 on RoboCasa-365 Atomic-Seen reports 81.9% SR but 50.1% Safety and 37.7% SBU; VLA-Adapter on LIBERO reports 97.6% SR and 82.0% Safety. These are safety/task-success measurements, not RRVR results.
- The 20 July 2026 ROS fuzzing survey reviews 92 selected works and identifies repeatable execution/reset and diagnosable failure analysis as comparatively weak stages. It specifically recommends reporting target layer, reset/replay policy, instrumentation overhead, artifacts and diagnosis output together. See [Yang et al., Applied Sciences 16(14), 7250](https://doi.org/10.3390/app16147250).
- The 16 November 2026 long-horizon manipulation survey calls out partial completion, error propagation, recovery cost and horizon-dependent degradation as metrics obscured by binary success. See [Zheng et al., Vicinagearth 3, 17](https://doi.org/10.1007/s44336-026-00048-x).
- [ROS 2 REP 2014](https://reps.openrobotics.org/rep-2014/) already recommends reproducible grey-box measurement of nodes and complete computational graphs using LTTng/ros2_tracing. [NIST's robotic performance framework](https://www.nist.gov/programs-projects/performance-assessment-framework-robotic-systems) composes perception, mobility, dexterity and safety metrics into task-level system assessment. RRVR should reuse these measurement disciplines, while adding typed owner/effect/attempt/epoch boundaries and physical command-holding graph evidence.

These sources reinforce that RRVR is an orthogonal assurance overlay. The target is not to outperform RoboCasa on raw task success or SafeVLA on its own safety specifications. The target is to make the same recovery-policy episodes evidence-complete and fail closed under declared faults, without suppressing valid positive-control recovery.



## 10. Compiled plan identity and replay integrity

The manifest is the experiment definition; the compiled plan is the immutable instruction artifact consumed by future platform adapters. The new compiler separates three levels of commitment:
- source-manifest hash over exact input bytes;
- pair commitment over the shared experiment conditions and ordered fault list;
- scenario schedule hash over the exact ordered event array.

A top-level plan hash binds the full body, while the compiled file itself has a separate artifact digest. The verifier requires both the exact source-manifest bytes and an externally pinned expected plan hash. This prevents a consumer from accepting an internally self-consistent but substituted plan merely because it carries its own newly computed digest.

The first compiler campaign is a software known-answer fixture. It must not be confused with execution of any fault injector. A later simulator adapter must emit per-injection receipts bound to plan SHA, pair/scenario/fault IDs and exact injector revision; ROS/controller/driver/HIL adapters must preserve that identity. Report plan validation, injection delivery, event observation, physical effect and safety review as distinct gates.


## 11. Delivery evidence between planned faults and observed failures

A deterministic scenario plan is not an executed experiment. The next qualification boundary is the execution receipt chain: target resolution -> native adapter dispatch -> injector execution -> independent target observation. Each phase has separate producer identity/revision, artifact digest, run/world epoch, cut point and clock coordinate. A delivery is Confirmed only when the pinned injector's execution receipt and a distinct observer artifact both bind to the planned event and the observed signature matches the signature committed in the compiled plan.

This directly operationalizes the 2026 ROS-fuzzing survey's finding that repeatable execution/reset and diagnosable failure analysis are less mature than input modelling, feedback and oracle construction. It also composes with Gazebo's simulator-agnostic ROS 2 Simulation Interfaces for reset, stepping and state queries, and the ROS 2 tracing guidance for graph-level instrumentation. None of these tools alone proves fault delivery; that requires the receipt/observation linkage.


## 12. Simulator control and event delivery

The Gazebo integration must use the standard ROS 2 Simulation Interfaces rather than timing sleeps or ad hoc ROS topic commands. Gazebo documents ResetSimulation, StepSimulation, SimulateSteps (action with feedback/cancellation), Get/SetSimulationState, GetSimulatorFeatures, GetEntities and GetEntityState. It also documents that SceneBroadcaster is required for state synchronization/reset detection; paused/running state updates are throttled and state-dependent calls can time out if its update rate is misconfigured. This supports three strict RRVR rules: pin the API and plugin build, negotiate required capabilities before mutation, and verify the exact simulation iteration/world epoch after every reset and step. See [Gazebo's current ROS 2 Simulation Interfaces](https://gazebosim.org/docs/latest/ros2_sim_interfaces/) and its [dependency/timeout notes](https://gazebosim.org/docs/rotary/ros2_sim_interfaces/).

The first RRVR adapter code is transport-neutral and mock-tested. It does not yet contact Gazebo. The next validation target is a concrete, version-pinned ROS 2 service/action client, then a named simulation world with independent target observation. The results remain simulation-only and must be reported separately from HIL/physical evidence.
