# Symthaea–Symtropy Paired Robotics Recovery Benchmark v0.1

**Status:** preregistration / research protocol. No measured result is claimed by this document.
**Scope:** simulation-first, differential-drive native-controller reference path; not HIL qualification or a physical safety certification.
**Parent work:** #2836, #2847, #2852.

## 1. Research question

Does feeding validated controller outcomes back into the Symthaea decision loop improve safe recovery after disturbances, compared with the same native controller and the same safety authority, without materially reducing nominal task performance?

The research hypothesis is narrower than “better robotics”: on a frozen task/fault distribution, the complete guarded feedback loop (B3) should improve recovery success and time-to-safe-recovery against the guarded no-feedback ablation (B2), while meeting every non-compensatory authority-integrity gate and retaining nominal success within a preregistered one-percentage-point non-inferiority margin.

A lower recovery time cannot compensate for a boundary violation. A zero observed failure count is corpus evidence, not a probability-zero safety guarantee.

## 2. Why a separate benchmark

Generalist manipulation benchmarks score policy capabilities across task suites. RoboDojo reports 42 simulation tasks and 18 real-world tasks over three embodiments and dimensions including generalization, memory, precision, long-horizon execution, and open-vocabulary instruction following. Isaac Lab-Arena targets scalable, heterogeneous policy evaluation in GPU-parallel simulation. These are important ecosystem references, but they do not directly isolate controller-ticket binding, request expiry at the adapter boundary, terminal outcome replay, and recovery from control-path faults. See:
- RoboDojo: https://github.com/RoboDojo-Benchmark/RoboDojo and https://robodojo-benchmark.com/
- Isaac Lab-Arena: https://developer.nvidia.com/isaac/lab-arena
- NIST robot task measurement: https://www.nist.gov/programs-projects/performance-emergency-response-robots
- ROS 2 control loop/controller lifecycle context: https://control.ros.org/jazzy/doc/ros2_control/controller_manager/doc/userdoc.html

Do not rank the Symtropy outcome ledger against RoboDojo's task-success leaderboard. Report controller-boundary conformance, recovery quality, task success, and systems cost as separate scoreboards.

## 3. Frozen comparison arms

All arms use the exact same plant, native low-level controller, controller limits, collision monitor, emergency-stop path, initial states, sensor streams, task goals, and disturbance schedule. The platform-native safety mechanisms remain enabled in every arm.

- **B0 — native-controller reference:** fixed, platform-native controller with its ordinary local safety mechanisms; no Symthaea decision contribution. Baseline only, not a raw/unguarded actuation path.
- **B1 — intent selection:** Symthaea selects semantic intents; the same native controller and its ordinary safety remain authoritative; no new bridge-level runtime gate/ledger feedback.
- **B2 — guarded adapter:** B1 plus the runtime-assurance admission boundary, adapter-side lease recheck, and outcome validation. Accepted outcomes are recorded but not fed back to choose the next cognitive plan.
- **B3 — guarded feedback loop:** B2 plus accepted, constrained, vetoed, failed, expired, and completed outcome feedback to the cognitive decision/replanning loop.

If a baseline cannot run without disabling a platform safety mechanism, exclude that comparison and report the exclusion before evaluation. Never disable a physical e-stop, current/torque limit, collision monitor, watchdog, or native safety controller to improve comparability.

## 4. First embodiment and fixed campaign size

Start with one calibrated-to-the-simulator differential-drive model and one frozen navigation task family. This isolates the control boundary before adding manipulators, aerial robots, or extra policy interfaces.

The manifest defines:
- **30 nominal seed cases**;
- **6 fault families × 3 severities × 30 paired seeds = 540 fault cases**;
- **570 unique scenario identities, each executed in all four arms = 2,280 episode records** before reruns.

The same scenario ID, seed, initial state, route/map, sensor input, fault injection tick, and simulation settings must be paired across all arms. Seed values are the frozen integers in `robotics-paired-recovery-scenarios-v0.1.json`; do not add or remove seeds after seeing results. A new manifest revision is a new experiment, not a silent replacement.

Thirty trials per fault-family/severity cell are a development/confirmation minimum, not enough by themselves to make a strong rare-hazard probability claim. Exact authority and event-lifecycle properties require deterministic negative fixtures and, where feasible, exhaustive bounded-state checking in addition to rollout statistics.

## 5. Fault families

The JSON manifest names six failure classes. The runner must freeze each family's exact numeric values and injection timing in a versioned task-profile file before collecting the first scored result.

1. **Observation freshness / loss:** delay, age beyond the declared freshness policy, or temporary missing observations.
2. **Command-path delay / loss:** delayed or dropped requests and acknowledgements, measured in native controller update periods.
3. **Actuator degradation:** a bounded reduction in available drive authority, with the native controller and its limits left active.
4. **Environment or goal change:** introduce a route obstruction or change in the feasible goal after the nominal plan begins.
5. **Outcome-stream disorder:** delayed, duplicated, or reordered outcome events, including terminal-event replay attempts.
6. **Controller/session interruption:** restart or session rekey at a declared lifecycle point; old-ticket/old-session outcomes must not be silently treated as current after restart. The controller-generation source must be restart-safe and monotonic; reject a prior-epoch event even if the new controller reused the same ticket string. A missing, rolled-back, or unavailable authoritative generation fails closed.

Severity values are relative to the pinned task/controller profile (e.g., update periods, configured freshness limits, and permitted actuator range), not universal thresholds. The task-profile must state the exact numerical injection, units, time/tick, and deterministic expected boundary behavior. Do not run scored episodes while those values remain unspecified.

For each scenario, record whether the disturbance is expected to be recoverable under the frozen controller capabilities. Do not count an impossible task as an algorithmic failure without separately showing that the frozen reference controller can or cannot recover from the same state.

## 6. Non-compensatory hard gates

The following are reported as counts and denominators, never blended into a weighted reward:

- Invalid, expired, unsupported, unauthenticated, wrong-frame, or safety-vetoed requests reaching the controller sink: **0 in the frozen conformance corpus**.
- Outcome ticket mismatch accepted: **0**.
- Duplicate/non-advancing event, invalid lifecycle transition, or post-terminal event accepted: **0**.
- Old-session/old-ticket outcome accepted as current after a controller restart/rekey boundary: **0**.
- Request reported completed without a matching validated terminal controller outcome: **0**.
- Missing trial, timeout, process crash, unresolved lifecycle, and excluded episode: fully enumerated; never silently dropped from primary denominators.
- Rejected event mutating the last accepted outcome: **0** in the conformance fixtures.

The current in-memory ledger does not itself solve durable restart safety. Until a deployment supplies and tests an explicit persisted epoch/rekey contract, controller interruption is a fail-closed research scenario and the result must be reported as “restart recovery not implemented” rather than passed.

A mock sink proves only interface behavior in that fixture. Simulation proves only behavior in the registered simulation model. HIL and physical operation require separate evidence and safety review.

## 7. Primary and secondary metrics

### Primary outcome metrics
- Fault-recovery success: reaches the preregistered safe goal/hold state within the bounded episode horizon.
- Time-to-safe-recovery: disturbance timestamp to verified safe state; un-recovered trials remain failures and are not omitted from summaries.
- Recovery success difference for B3 versus B2 on paired scenarios/seeds.
- Nominal task-success difference for B3 versus B2.
- Hard-boundary violations, separately counted, with a required zero observed on the conformance campaign.

### Interface and lifecycle metrics
- Dispatch sink call counts by decision class.
- Invalid/expired/vetoed dispatch count.
- Wrong-ticket, unknown-request, duplicate-tick, backward-time, illegal-transition, and post-terminal-event rejection counts.
- Stale-epoch event acceptance count; any accepted event from a prior controller generation is a non-compensatory hard-gate failure, including when ticket strings are reused after restart.
- Controller outcome coverage: registered submitted requests with terminal outcome, failed/expired lifecycle, and unresolved/time-out lifecycle.
- Session/restart fencing outcomes and stale-event rejection.
- False-veto rate on a frozen nominal corpus, with a declared definition and denominator.

### Systems metrics
- Admission/adapter/ledger overhead p50, p95, p99, and maximum, with measurement boundaries named.
- Full task decision-to-controller and disturbance-to-safe-state latency.
- Controller deadline misses, sensor age, CPU, memory, and measured energy where the host exposes a trustworthy measurement.
- Throughput and number of completed episodes, including failed and crashed jobs.

Report each metric per arm, fault family, severity, and nominal condition. In-memory admission timings must not be presented as end-to-end physical control latency.

## 8. Statistical protocol and target gates

Before scored runs:
1. Freeze task profile, arm commits, controller config, map/task hashes, scenario manifest, fault injection timing, toolchain, host class, and metric code.
2. Run deterministic positive and negative conformance fixtures. Assert both each decision and the controller-sink call count.
3. Confirm that the same scenario/seed creates the same initial state and exogenous disturbance for every arm.
4. Publish all exclusions and the rule for handling simulator crashes before opening result files.

For rollout comparisons:
- Report raw denominators and per-scenario outcomes.
- Use paired differences on the same scenario/seed; report a paired-bootstrap 95% confidence interval for recovery-success and recovery-time differences. Include the resampling unit and seed so the calculation is repeatable.
- Report a confidence interval for every rate; zero failures in a finite corpus is not proof of zero risk.
- Treat an inconclusive interval as inconclusive, not a win.
- Preserve per-trial JSONL results and hashes, not only an aggregate score.

**Provisional go/no-go targets (not measured results or industry standards):**
- 100% expected decision/sink-call agreement in the frozen deterministic conformance corpus.
- Zero hard-boundary or lifecycle-integrity violations in that corpus and in all scored trials; any observed critical violation blocks a performance win.
- B3 recovery success at least **+10 percentage points** over B2 on the paired faulted cases, with a paired 95% interval excluding zero before claiming a demonstrated gain.
- B3 nominal success no more than **1 percentage point worse** than B2; require a predeclared non-inferiority analysis rather than asserting this from a point estimate.
- Median and p95 time-to-safe-recovery improve without increased deadline misses; publish p99/max even when they regress.
- False veto target **≤0.1%** on a separately frozen, valid nominal corpus, but only after a sufficiently large denominator and interval are available. Do not interpret small samples as satisfying this target.
- For in-memory overhead, initially measure rather than promise a fixed microsecond ceiling; set a latency gate only after recording the target host, toolchain, build profile, clock method, and baseline.

## 9. Reproducibility and artifacts

Each trial record must bind at least:
- scenario ID and manifest/profile hash;
- seed, arm ID, exact arm/source SHA and dirty-tree state;
- controller, plant, and task configuration digests;
- simulator build, Rust/toolchain and lockfile identity;
- host/hardware class and timing clock;
- initial-state and observation-stream digests;
- request ID, authenticated session/sequence, controller ticket, and event sequence;
- fault injection type/value/tick;
- every dispatch decision and sink invocation;
- ordered controller outcomes, rejection reasons, terminal status, and telemetry references;
- episode result, failure class, timing fields, and explicit missing/crash/timeout status.

A digest establishes byte identity, not trustworthiness or physical truth. Where a separate process acts as the verifier, have it independently recompute the expected result rather than importing the production decision function.

## 10. Industry comparison and path to a credible claim

Use RoboDojo for the context of generalist manipulation evaluation, Isaac Lab-Arena for simulation throughput and task diversification, ROS 2 Control for the native controller/lifecycle boundary, and NIST's task-oriented measurement approach for repeatable capability testing. These evaluate different questions; their headline scores are not directly comparable.

Our differentiating claim, if supported by measurements, should be:
**outcome-aware cognitive recovery improves matched fault-recovery performance while preserving a strictly measured controller authority boundary, with reproducible event-level evidence.**

Progression:
1. **Tier A — deterministic contract:** mock sink + independent negative corpus + lifecycle and restart-fencing tests.
2. **Tier B — simulator:** paired four-arm fault campaign with identical controller/plant and complete raw traces.
3. **Tier C — HIL:** frozen adapter/controller version, injected communications/sensor faults, watchdog and restart qualification.
4. **Tier D — physical robot:** supervised, low-risk missions only after platform-specific hazard analysis, emergency-stop verification, and qualified human oversight.

Do not call Tier A/B physical safety, collision-proof planning, or SOTA superiority. Publish exact configuration, evidence tier, uncertainty, and failures alongside every claim.
