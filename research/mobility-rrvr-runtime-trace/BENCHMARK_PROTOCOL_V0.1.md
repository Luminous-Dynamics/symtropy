# RRVR matched evidence and recovery benchmark v0.1

**Tracking:** [#7106 — matched evidence, fault-delivery, recovery and diagnosis benchmark suite](https://github.com/Luminous-Dynamics/symtropy/issues/7106)  
**Scorer:** src/benchmark.rs  
**CLI:** rrvr_benchmark_score  
**Schema:** schemas/benchmark_corpus_v0_1.schema.json  
**Synthetic fixture:** fixtures/benchmark_matched_arms_v0_1.json

## 1. Benchmark thesis

RRVR is not intended to replace task, manipulation, VLA-safety or recovery benchmarks. It is an evidence/recovery overlay: preserve native task and safety outcomes, then measure whether the system can reliably explain what happened across a fault, restart, physical effect, observer verdict, downstream fence and recovery decision.

The lifecycle under measurement is:

Plan → Dispatch → Injector execution → Independent delivery observation → Physical-effect outcome → Downstream quiescence → Restart/recovery decision → Authority admission.

Every stage keeps its own artifact, scope and verdict. A scheduled event, accepted ROS request, injector log or completed task is never automatically promoted into physical delivery or safe recovery.

## 2. Existing baselines and what RRVR adds

### RoboRecover (2026)

RoboRecover evaluates robot-policy recovery from execution-induced deviation states reconstructed by replaying action prefixes. Its public paper describes 2,000 scenarios spanning RoboTwin and LIBERO, with fixed 800/200 train/test splits on each platform. It establishes recovery from off-nominal intermediate states as a separate policy-evaluation dimension. RRVR should reuse compatible recovery episodes and preserve RoboRecover's native recovery score; the added RRVR dimension is evidence sufficiency, effect/attempt continuity, fault-delivery qualification and no-unsafe-retry behavior.

Sources:
- [RoboRecover paper](https://arxiv.org/abs/2609.28952)
- [RoboRecover public repository](https://github.com/RUCKBReasoning/RoboRecover)

### SafeVLA-Bench (2026)

SafeVLA-Bench intentionally separates native task success (SR) from episode safety, successful-but-unsafe (SBU), and worst-violation severity (VSI). Its leaderboard reports 24 policies, five suites, 22,500 episodes and eight safety specifications, with trajectory/force signals re-evaluated from raw rollouts. That is the correct methodological pattern: keep success and safety separate. RRVR adds a third orthogonal dimension—evidence/recovery integrity—without collapsing it into either metric.

The benchmark must preserve the host policy wrappers, seed protocol, success predicates and safety specifications. Report SR, Safety, SBU and VSI exactly as defined by the host benchmark, then report RRVR overlay metrics beside them.

Sources:
- [SafeVLA-Bench paper and methodology](https://safevla.org/)
- [SafeVLA-Bench current leaderboard](https://safevla.org/leaderboard.html)

### ROS fuzzing and diagnosis

The July 20, 2026 survey analyzed 92 selected works. It concludes that ROS fuzzing is moving beyond message validity toward runtime and task-level failures, but the evidence profile is uneven: input modelling, feedback and oracle construction have advanced more clearly than repeatable execution and diagnosable failure analysis. It also warns that different ROS layers, failure definitions, oracles and execution settings make results difficult to compare.

RRVR targets that gap with immutable plans, run/world epochs, per-phase receipts, replayable traces, causal minimization, exact failure signatures and cross-layer provenance. These are design opportunities motivated by the review, not proof that all current tools lack these features.

Source: [A Survey of Fuzzing for ROS-Based Robotic Systems, Applied Sciences 16(14), 7250 (2026)](https://doi.org/10.3390/app16147250)

### Gazebo and ROS 2 measurement substrate

Gazebo implements the simulator-agnostic ROS 2 Simulation Interfaces. The documented control/query set includes feature discovery, ResetSimulation, StepSimulation service, SimulateSteps action (with feedback and cancellation), GetSimulationState, SetSimulationState, GetEntities, and GetEntityState. SceneBroadcaster is required for synchronized world-state and reset detection. State queries synchronize through SceneBroadcaster updates; its default update cadence differs between running and paused states, while step completion feedback is derived from the world-stats stream published at 10 Hz. A benchmark must therefore distinguish the requested cut-point step, actual execution step, observation step, state-query latency and step-feedback latency.

This is important: a ROS service response or action result is not automatically a physical-effect observation, and the 10 Hz stats feedback must not be mistaken for a hard real-time fault-delivery timestamp.

Source: [Gazebo ROS 2 Simulation Interfaces](https://gazebosim.org/docs/rotary/ros2_sim_interfaces/)

### ROS 2 tracing guidance

REP 2014 gives useful grey-box benchmarking guidance and recommends LTTng/ros2_tracing for benchmarking individual nodes and complete computational graphs. Its current page marks the REP status as Rejected, so RRVR references its methodology as useful guidance—not as an active normative standard.

Source: [REP 2014: Benchmarking performance in ROS 2](https://reps.openrobotics.org/rep-2014/)

## 3. Matched experimental arms

Every matched pair contains exactly five arms:

1. **task_only** — host task/safety protocol, without an RRVR recovery/evidence overlay;
2. **restart_only** — restart recovery without cross-owner physical-effect/observer/graph composition;
3. **logging_only** — ROS/Gazebo instrumentation but no evidence-qualified interpretation;
4. **trace_only** — trace/schema/model checking without complete cross-owner physical-effect and independent observer composition;
5. **full_rrvr** — plan-bound execution receipts, independent observer qualification, effect reconciliation, downstream fencing and native-authority handoff.

All five arms must share identical task, policy/checkpoint, scene, action prefix, seed, platform profile, environment and planned fault-schedule digests. The scorer recomputes a commitment from those inputs and rejects a pair if any arm differs.

A benchmark cell that cannot preserve the same inputs is not a matched comparison and must be reported separately.

## 4. Required metrics

### Native task and safety (preserve host definitions)

- Native task success rate (SR).
- Episode safety rate.
- Successful-but-unsafe count and rate, both over all episodes and over successful episodes.
- Violation severity distribution from the host benchmark's raw signals.

RRVR does not define a replacement for task or safety. It preserves those host metrics and their denominators.

### Evidence integrity

- False Confirmed rate: validator says Confirmed, independent adjudication says NotDelivered.
- False Refuted rate: validator says Refuted, independent adjudication says Delivered.
- Unknown/insufficient-evidence frequency.
- Delivery observation coverage: independently observed delivery receipts / planned fault events.
- Target/revision/clock/world epoch/producer mutation rejection.
- Evidence sufficiency latency and provenance completeness.

Ground-truth adjudication is supplied as an input label and must be independently reviewed. The scorer does not authenticate adjudicators or decide what happened in the physical world.

### Recovery and physical-effect safety

- Recovery attempts and native recovery success.
- Duplicate physical effects.
- Blind redispatch after an unknown outcome.
- Stale downstream execution after accepted quiescence.
- False admission.
- Partial/unknown/disputed effect outcomes.
- Operator interventions and recovery cost.

### Diagnosis and reproducibility

- Root-cause localization top-1 and top-3 accuracy against independently adjudicated component labels.
- Original and minimized trace event counts; median trace-size reduction.
- Exact failure-signature preservation and causal/happens-before preservation (qualified separately by the trace-minimizer verifier).
- Replay-complete rate, failure reproduction rate, time-to-replay and time-to-explanation.
- Reset repeatability and event/cut-point skew.

### Performance

- Reset, pause, step, dispatch, injector receipt, observer and reconciliation p50/p95/p99 latency.
- Time-to-evidence p50/p95/p99.
- Mission makespan p50/p95/p99.
- CPU, memory, trace overhead and artifact storage per event.
- Throughput under safe concurrency, only after all hard safety/evidence gates pass.

## 5. Scoring method v0.1

The Rust scorer uses:
- integer parts-per-million for rates;
- nearest-rank percentiles (no interpolation);
- paired per-pair differences for full RRVR versus each baseline;
- exact matched-input commitments;
- strict rejection of duplicate episode/artifact identities, incomplete pairs, inconsistent safety severity, impossible recovery labels, unsupported delivery verdicts and invalid trace-minimization counts.

The paired comparisons are descriptive: task-success, safety and recovery-success differences are reported with pair wins/ties/losses and mean paired deltas. Version 0.1 does not compute confidence intervals or significance tests.

## 6. Research targets (not achievements)

Hard evidence/safety targets:
- zero false Confirmed verdicts on the frozen negative-control corpus;
- zero stale downstream executions after accepted quiescence;
- zero blind redispatch after unknown dispatch outcomes;
- zero duplicate effects in the frozen restart/replay corpus;
- 100% declared cut-point, command-holding-node and fault-domain coverage;
- 100% rejection of source/plan/schedule/target/epoch/clock/producer mutation controls.

Statistical target:
- At least 2,995 independent representative zero-failure trials for each safety-critical failure class to support a one-sided 95% upper failure-probability bound below 0.1%, assuming representative independent trials.
- Correlated seeds, repeated simulator snapshots and shared fault schedules are clustered observations; they must not be falsely counted as independent trials. Use cluster-aware or hierarchical uncertainty analysis in the inferential scorer version.

Utility/performance targets:
- at least 99.9% valid positive-control admission within a declared platform profile, with confidence intervals and false-hold decomposition;
- at least 25% lower verified mission makespan versus a safe sequential baseline, only after all hard gates pass;
- at least 50% median trace-event reduction while preserving exact property/witness/happens-before identity;
- 100% deterministic plan-to-event replay identity for fixed artifacts/seeds/builds, with simulator nondeterminism exposed and quantified.

These are research targets, not current results.

## 7. Reproducibility protocol

Preregister the scenario corpus, baseline arms, metric definitions, exclusions, stopping conditions and oracle ownership. Retain raw episode/trace artifacts, plan and manifest digests, policy checkpoint, simulator/plugin/firmware builds, world snapshot, reset profile, event schedule, oracle inputs and minimized reproducer. Report all denominators and confidence intervals. Keep simulation, HIL and physical evidence tiers separate.

No benchmark aggregate should be published without enough metadata to reconstruct each matched episode and regenerate the tables from raw artifacts.

## 8. CLI

    cargo run --locked --manifest-path research/mobility-rrvr-runtime-trace/Cargo.toml --bin rrvr_benchmark_score -- benchmark-corpus.json benchmark-report.json

The CLI emits a versioned report and a compact summary sidecar. A mismatch in policy, seed, scene, environment or fault schedule within a matched pair is rejected.

## 9. Claim ceiling

A passing scorer demonstrates corpus structural validity and deterministic metric calculation. It does not authenticate raw artifacts, prove the independent ground-truth labels, establish physical safety, or show RRVR is superior to SOTA. Superiority requires matched public benchmark runs, independently validated labels, enough representative trials, and full disclosure of protocol and denominators.


## 10. Protocol and oracle identity locks

Each episode now carries the host benchmark ID/revision, native policy-wrapper revision, safety-specification revision, independent delivery-adjudicator revision and root-cause-adjudicator revision. These are part of the matched-input commitment: two arms cannot be compared if they quietly use different host protocols or oracle versions. A scenario-cluster ID is also required and reported separately from the number of matched pairs so later inferential analysis can avoid counting repeated episodes from one scenario family as independent clusters.


## 11. Cluster-aware inference mode v0.1

The scorer provides an optional nonparametric percentile bootstrap over scenario clusters:

    cargo run --locked --manifest-path research/mobility-rrvr-runtime-trace/Cargo.toml --bin rrvr_benchmark_score -- benchmark-corpus.json benchmark-report.json --bootstrap-replicates 1000

It requires at least five distinct declared clusters and between 1,000 and 200,000 replicates. A corpus with fewer than five clusters can still be scored descriptively, but inferential mode rejects it.

For each resample, the implementation draws the same number of clusters as observed, with replacement. Every matched pair and its five arms remain together inside the sampled cluster. Pair-level task/safety/recovery deltas are first averaged within each cluster, then clusters are equally weighted, avoiding larger clusters dominating merely because they contribute more episodes. The fixed resampling seed is derived from the benchmark version, exact source-corpus digest and replicate count. The report contains paired full-RRVR-minus-baseline point estimates and nearest-rank 95% percentile intervals for task success, safety and recovery success.

This is a percentile cluster bootstrap, not a hierarchical model or BCa interval. Five clusters is an execution floor, not evidence of adequate power or independent population sampling. The CI corpus has six synthetic clusters solely to exercise the code; those intervals are not empirical results. The researcher must justify that clusters represent meaningful independent sampling units and report sensitivity to cluster definitions, task/platform strata, exclusions and predeclared endpoints. When clusters share a controller, simulator build, task template or fault-generation seed, that dependence must be reflected in the cluster design.


## 12. Delivery-decision error denominators

The arm report distinguishes four related quantities:
- **False-confirm rate:** false Confirmed / all episodes independently adjudicated NotDelivered. This is a false-positive rate with the actual negative class as denominator.
- **False-refute rate:** false Refuted / all episodes independently adjudicated Delivered. This is a false-negative rate with the actual positive class as denominator.
- **Decision coverage:** classified Confirmed-or-Refuted episodes / all episodes with adjudicated Delivered-or-NotDelivered truth. Unknown assessments are abstentions and lower coverage.
- **Classification accuracy:** correct Confirmed/Refuted decisions / all classified decisions. Unknown abstentions are not counted as classifications; coverage must be reported beside accuracy so a system cannot look accurate simply by refusing to decide.

Counts for adjudicated Delivered and NotDelivered, classified episodes, unknown decisions and correct classifications are emitted separately. An always-Unknown system can have undefined classification accuracy, but its decision coverage is zero; it cannot be presented as an accurate classifier.
