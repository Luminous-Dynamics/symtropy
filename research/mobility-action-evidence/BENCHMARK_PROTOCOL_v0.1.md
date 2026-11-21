# Evidence-Gated Autonomy Benchmark Protocol v0.1

**Programme:** CDAAB — Cross-Domain Autonomy Assurance Benchmark  
**Implementation:** MOBILITY-ACTION-EVIDENCE-001 (#6503), M1 mission supervisor (#6498/#6499), M2 action/effect contract (#6530)  
**Status:** research protocol. The current Rust tests are a contract-level software reference, not a simulation, HIL, physical-safety or regulatory qualification.

## 1. The paradigm shift

The field commonly scores a policy by whether an action sequence succeeds in a particular evaluation setup. That is useful, but it does not answer whether an action was still valid when dispatched, whether its assumptions had expired, whether a remote claim was trustworthy, whether a timeout hid a partial physical effect, or whether the reported completion was independently observed.

CDAAB evaluates **verified autonomy**, not action generation alone. The unit of evaluation is an evidence-bearing transition:

`Intent → Proposal → Configuration-bound ticket → Local assurance check → Dispatch → Transport response → Independent physical observation → Reconciliation → Accepted outcome`

These are separate events and must not be collapsed into a single success flag.

Five principles define the paradigm:

1. **Proof-carrying action, not timeless command.** Every action is bound to the exact platform/configuration, ODD and mode revision, mission generation, authority epoch/scope, evidence watermark and validity window that justified it.
2. **Epistemic uncertainty is an execution state.** Unknown, unqualified, stale and conflicted evidence are typed states with explicit holds; they are not silently converted into low-confidence numbers or a healthy state.
3. **Outcome is causal and independently observed.** Transport acknowledgement, controller status and agent self-report are not proof of physical effect. Completion requires a separate, predicate-bound observer/evaluator receipt.
4. **Authority is local, revocable and partition-resilient.** Federation can coordinate and preserve evidence, but a network partition must not disable local safe-state behaviour or extend expired authority.
5. **Recovery is a first-class capability.** A timeout does not imply no effect. Partial/unknown effects require reconciliation; retries require independently established no-effect or an independently reviewed recovery permit.

This architecture composes existing owners. It is not a second estimator, trust fabric, ODD engine, controller, clock service, fusion engine or runtime-assurance system.

## 2. Evaluation ladder

The levels are non-substitutable. A pass at one level never implies a pass at a higher level.

| Level | Evidence | Minimum result |
|---|---|---|
| L0 — Contract | deterministic unit/integration tests, negative controls, independent oracle | every declared invariant and mutation gate agrees with the oracle |
| L1 — Reproducible replay | frozen traces, exact hashes, deterministic re-execution | same inputs produce same decisions and evidence digest |
| L2 — Adversarial simulation | fault injection, scenario variation, controller/evaluator separation | no unsafe acceptance on declared faults; useful positive-control acceptance |
| L3 — Matched system comparison | planner-only vs M1 vs M1+M2 vs domain RTA+independent evaluator | report safety, mission, cost and recovery dimensions separately |
| L4 — HIL / hardware qualification | target compute, clocks, sensors, actuator I/O in guarded test rigs | real timing, reboot, bus, sensor and fault behaviour measured on named hardware |
| L5 — Physical / operational evaluation | independent test authority, relevant safety case and approvals | only claims explicitly supported by the test programme |

## 3. Frozen M2 contract corpus

The current integration-test oracle compares two separately written implementations over deterministic fault mutations. It covers 23 admission cases and 18 physical-outcome cases across positive controls, expired tickets, configuration changes, mission generation, ODD/mode revision, authority epoch/revocation/scope, clock restart, navigation profile/service/frame/source/sequence, unknown/unqualified/missing navigation time, mismatched comparison clock, missing cross-clock correlation, stale observations, observer identity, event sequence, ticket/configuration/frame/calibration/predicate binding, confidence, and no-effect/partial/unknown dispositions.

The next corpus expansion should keep three files separate:

- **Controller-visible manifest:** actions, allowed inputs, contracts and scenario parameters.
- **Evaluator-only truth:** injected fault identity, actual simulated physical state, ground-truth effect, expected safe/unsafe labels.
- **Run receipt:** exact source commits, binary digests, runtime versions, scenario-set digest, seed commitments, per-episode event logs and evaluator version.

The controller must not receive the evaluator labels, hidden fault identity or expected outcome. Freeze the manifest and evaluator protocol before running candidate policies; disclose hashes before execution and reveal hidden labels after the run.

Required scenario strata:

- **Temporal:** exact expiry boundary, stale source, timestamp from future, clock reset, boot epoch change, drift/correlation loss, delayed and reordered messages.
- **Configuration/authority:** platform swap, firmware/configuration change, ODD/mode transition, authority renewal/revocation, scope reduction, lease expiry, stale mission generation.
- **Evidence integrity:** duplicate event, same event ID/different digest, source replay, evaluator/controller identity collision, missing calibration, conflicting frame, common-mode source failure.
- **Physical response:** successful effect, no effect, partial effect, ambiguous effect, delayed effect, unobserved effect, sensor disagreement.
- **Distributed operation:** network partition, delayed federation, remote advisory contradiction, recovery after reconnection, local safe-state availability while disconnected.
- **Positive capability:** valid ticket, valid cross-clock correlation, legitimate degraded-mode operation where the domain profile allows it, safe recovery, and independently confirmed completion.

## 4. Matched comparisons

Run identical scenarios, seeds, sensors, action budget, compute budget and low-level controller privileges for:

- **A — Baseline:** planner/controller without M1/M2 evidence gating.
- **B — M1:** mission supervisor with explicit completion and reconciliation semantics.
- **C — M1 + M2:** configuration-/authority-/lease-bound action tickets and physical-outcome validation.
- **D — Complete domain stack:** C plus the domain's existing runtime assurance, ODD, control-envelope and local interlock owners, with an independent physical-response evaluator.

Do not compare a planner with richer sensor access or privileged simulator state against a restricted system without declaring and matching those privileges. Report ablations as paired runs; no result should be credited to M2 if the improvement comes from an unreported controller, sensor or compute change.

## 5. Non-compensatory metrics

Publish each dimension, its denominator and its uncertainty. Never average safety failures away with speed or task success.

### Safety / authority

- invalid, expired, revoked, out-of-ODD or stale-evidence admissions;
- false-safe and false-completion rates;
- mutation attempts after unknown/partial effect;
- unqualified-time promotion and clock/boot mismatch acceptance;
- time from invalidation to hold / local safe-state.

**Contract gate:** zero false accepts in every frozen negative control; 100% of invalidation cases must resolve to explicit reject/hold/reconciliation. No unsafe mutation is permitted after an unknown effect.

### Capability / utility

- valid positive-control acceptance;
- mission and subgoal completion conditional on zero safety/authority violations;
- unnecessary hold rate, rejected-valid-action rate and time to recover;
- subgoal count, horizon and task-family performance;
- time-to-complete and operator intervention.

**Initial research target:** at least 95% acceptance on a separately frozen valid positive-control corpus while retaining 100% rejection of the declared negative controls. These are acceptance targets, not achieved results.

### Evidence quality

- observer/evaluator agreement and independently adjudicated attribution accuracy;
- evidence completeness, provenance and lineage;
- duplicate/replay/conflict detection;
- reconciliation completeness and unresolved-outcome fraction;
- calibration and confidence calibration, when an actual calibrated probability is supplied.

### Systems cost / real-time behaviour

- p50/p95/p99 and worst-case measured decision/hold latency;
- CPU, memory, energy and communication bytes per verified outcome;
- local safe-state performance during network partition;
- recovery time and operator minutes;
- cost per verified completion, not merely cost per attempted episode.

## 6. Statistical and benchmark-integrity rules

- Use exact binomial intervals for per-trial categorical failure rates and paired, scenario-cluster-aware intervals for policy comparisons.
- Treat scene, platform, weather, route, sensor configuration and fault family as possible dependence clusters. Do not count correlated repeated episodes as independent trials.
- Publish raw counts, denominators, confidence intervals, excluded episodes and reasons; include positive and negative controls.
- Report calibration (Brier score / ECE) only when outputs are genuinely probabilistic and the calibration set is separate from the final test.
- Include shortcut-solvability probes, held-out scenario families, within-range randomization, training-data/source disclosure and periodic hidden-set refresh.
- Keep test labels and evaluator logic out of the controller. An incomplete exploration or unresolved result is UNKNOWN/INCOMPLETE, never PASS.
- Freeze the precise benchmark version, commit, split, evaluator, scenario assets and aggregation rules. Different benchmark versions, sensor privileges, model wrappers and suites are not interchangeable.

For intuition on statistical resolution: with zero observed failures, roughly 2,995 independent trials are needed for a one-sided 95% upper bound below 0.1%. Independence is often unrealistic in simulation; the actual design must account for scenario clusters and effective sample size. Do not use this illustrative figure as a substitute for a preregistered sampling plan.

## 7. External comparison set (version-pinned, not score-collapsed)

### Long-horizon manipulation

- **LADDER:** 145 tasks, three suites, 1–10 subgoals, two embodiments and 2,024 simulated episodes per system; the research page reports GPT-Policy/GPT-6 Astra at 62.0% task success and $6.49 per successful episode. The live leaderboard currently lists 63.3% for GPT-6 Astra. Treat this as a metric/revision discrepancy to resolve against exact run artifacts, not as a single unambiguous SOTA value. Real WidowX and Franka experiments are also reported.
- **RoboCasa365:** official leaderboard snapshot updated 16 November 2026 lists HexaShell at 85.6 overall, including 80.0 on composite-unseen; Awomo-0.5I is at 64.4 overall and 45.2 composite-unseen. This is a different task suite, training protocol and score; do not compare 85.6 directly with LADDER's 62%.
- Use the 2026 CoRL benchmark-audit diagnostics for shortcut solvability, statistical significance, distribution/sample overfitting and data-source dependence before treating a manipulation score as capability evidence.

### Road autonomy

- Pin **Bench2Drive v0.0.4** explicitly. The official repository identifies v0.0.4 as current, provides a 220-route validation set (44 scenarios × 5), and reports LEAD at 95.59 driving score / 87.28 success rate in its published closed-loop table. Preserve route/scenario strata, vehicle/controller version and the metric components; never compare a headline driving score from another version.
- Use **NAVSIM v2** with the official devkit and a named private-test split for planning comparisons. Its EPDMS is an extended, two-stage pseudo-simulation score, not a full reactive closed-loop road test. The official docs explain the two-stage follow-up-scene weighting; third-party leaderboard aggregators are snapshots and not a replacement for the official server/evaluator.
- Retain Waymo Open Dataset as a separate perception/motion/E2E data and leaderboard comparison. Waymo states formal challenges are not hosted in 2026, while its leaderboards remain active.

### Air autonomy

- Use the existing aircraft-local controller and runtime-assurance stack. Compare against PX4's publicly documented unit/integration CI, sanitizer/SITL, hardware bench testing and HIL flow. A desktop/SITL pass must not stand in for target flight-controller testing: PX4 explicitly documents hardware-only faults such as boot ordering, link lifecycle, storage, timing drift and concurrency.

### Marine autonomy

- Use COLREG encounter families, wind/current feasibility, right-of-way uncertainty, visibility/degradation, sensor loss, local fallback and human handover as separate strata. The 15 November 2026 safe-RL unmanned-sailboat paper combines wind-dependent COLREG reward shaping and safety shielding with hierarchical risk-aware radar; treat it as a current simulation comparator unless and until a matched physical result is available.

## 8. How to beat the state of the art

The strategy is not to claim a larger policy score than a VLA or driving planner on a task suite that does not measure our contribution. It is to create a new, reproducible comparison axis:

1. **Beat the benchmark's invalidity:** every score is accompanied by shortcut, significance, overfitting and provenance diagnostics.
2. **Beat command-centric systems on assurance:** zero false completion and zero unqualified / revoked / expired action admission across frozen fault strata.
3. **Beat timeout-based orchestration on recovery:** no blind retry; measure ambiguity detection, physical-outcome attribution, safe recovery and mission restoration.
4. **Beat monolithic orchestration on cost:** use staged cheap checks, typed claims and escalation to expensive reasoning only when evidence warrants it. Compare cost per independently verified completion at matched safety.
5. **Beat closed-world assumptions on generalization:** hidden scenario families, configuration changes, source common-mode faults and cross-domain adapters that preserve each domain's own safety semantics.
6. **Beat opaque autonomy on auditability:** reproducible event lineage from proposal and authority through action, observation, independent evaluation and final mission outcome.

**Performance target after the safety gate:** at matched verified success and zero additional safety/authority violations, target a ≥10% relative reduction in operator interventions or all-in resource cost, with a preregistered confidence interval excluding no improvement. This is a research target, not a measured gain.

## 9. Claim ceiling and next gates

The current M2 crate and independent oracle qualify only deterministic software-contract behaviour for the fields and assumptions it explicitly checks. The next gates are: exact-head CI; mutation/oracle agreement; M1 adapter; Mycelix claim/evidence mapping; frozen controller/evaluator corpus; simulator integrations per domain; then HIL/physical tests through domain owners.

No part of this protocol authorizes actuator output, vehicle operation, flight, navigation service, or regulatory compliance.
