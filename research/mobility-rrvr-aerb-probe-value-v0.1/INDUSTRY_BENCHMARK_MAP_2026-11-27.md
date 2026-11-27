# RRVR / AERB industry benchmark map and competitive plan

**Snapshot date:** 2026-11-27  
**Purpose:** current-source benchmark comparison and a reproducible plan for demonstrating anticipatory execution assurance.  
**Claim status:** literature/leaderboard mapping and proposed targets only; no comparative AERB result is claimed.

## 1. Executive positioning

The robotics benchmark landscape now measures several adjacent dimensions:

1. **Task capability:** can a policy complete manipulation tasks from prescribed initial states?
2. **Safety compliance:** did the trajectory satisfy the benchmark's safety specifications, including cases that succeeded but violated a safety clause?
3. **Failure diagnosis and recovery:** can a policy interpret an execution deviation and resume the task?
4. **Action-conditioned prediction:** does a predicted rollout agree with a paired real execution?
5. **Active sensing:** does another observation improve the downstream decision enough to justify its resource and latency cost?
6. **Execution integrity / finality:** what physical effect may already have occurred, which command paths can still produce an effect, and is the evidence sufficient to permit another effect?

RRVR/AERB should be an overlay at the intersection of (2)–(6), while preserving the host benchmark's native task-success and recovery results. The defensible differentiator is not another general-purpose manipulation policy or a claim to better task success. It is a measurable assurance question:

> Before the next potentially irreversible effect, did the system preserve all plausible execution histories, choose a decision-relevant observation, and maintain a truthful boundary between predicted futures and the one factual history?

This is a research gap suggested by the reviewed public benchmarks, not a claim that no industrial or proprietary system implements similar mechanisms.

## 2. Current external benchmark reference points

### 2.1 RoboCasa365 — generalist manipulation capability

Official leaderboard snapshot inspected **2026-11-27**; leaderboard page reports updated **2026-11-25**:
- 50 target tasks, 20 evaluated entries, three splits.
- Top overall entry: HexaShell (classified as an Agent, not a VLA), 85.6 overall; 91.1 atomic-seen; 85.0 composite-seen; 80.0 composite-unseen.
- Top listed VLA entries include minionx at 62.6 overall and Axiom-0 at 61.6.

Use it as a task-capability anchor only. Its overall score is an aggregate task-success measure, not an execution-integrity or safety score.

Source: https://robocasa.ai/leaderboard.html

### 2.2 SafeVLA-Bench — safety separate from success

Official leaderboard currently describes:
- 22,500 scored episodes;
- 24 policies evaluated;
- five evaluation suites;
- eight safety specifications;
- last updated 2026-09-26.

Examples from the official board:
- LIBERO VLA-Adapter: 82.0% safety versus 97.6% native task success, 17.0% successful-but-unsafe (SBU), VSI 0.104.
- RoboCasa-365 Atomic-Seen Xiaomi-Robotics-1: 50.1% safety versus 81.9% task success, 37.7% SBU, VSI 0.233.

The leaderboard defines safety as the share of episodes satisfying every applicable safety specification, not success. It also ranks by safety rather than task success. AERB must keep its execution/evidence endpoints distinct from these STL-style trajectory safety measures.

Source: https://safevla.org/leaderboard

### 2.3 RoboRecover — recovery from execution deviations

RoboRecover reports 2,000 scenarios across RoboTwin and LIBERO, with 1,000 scenarios per platform and fixed 800/200 train/test splits. It reconstructs intermediate deviation states by replaying action prefixes, then evaluates recovery on the original task.

This is the best host integration target for preserving a recovery benchmark's protocol while adding an independent RRVR evidence overlay. Keep host replay fidelity, policy recovery success and AERB assurance as separate gates.

Source: https://arxiv.org/abs/2609.28952

### 2.4 LIBERO-RECOVER — staged manipulation recovery

The current project page inspected 2026-11-27 advertises:
- 2,178 recovery scenarios;
- 130 subtasks across four LIBERO suites;
- four recovery levels;
- 16 evaluation dimensions;
- 3,184 human recovery demonstrations;
- 625,731 frames at 20 Hz.

The current arXiv v4 abstract dated 2026-11-03 still describes 1,000+ scenarios; the project page labels the work a 2027 submission / under review. Keep those release-surface counts attributed to their respective sources until they are reconciled.

This benchmark is useful for scenario and recovery-level coverage. AERB should add command/effect/authority ambiguity and evidence-qualified decisions rather than relabel its recovery levels.

Sources:
- https://liulin815.github.io/LIBERO-Recovery/
- https://arxiv.org/abs/2609.05178

### 2.5 VLA-FixBench / FaultEval — diagnosis is not grounded repair

VLA-FixBench is an ICML 2026 benchmark with 6,034 task-execution episodes across 40 simulated task settings and two real-robot tasks (tea preparation and charging-plug insertion). Its framework spans static diagnosis, dynamic repair and real-robot evaluation. The PMLR paper reports that an idealized human-intervention upper bound can improve task success by 13 percentage points on LIBERO and 35 percentage points on real robots.

AERB must test more than whether the observer gives a plausible explanation. It should score whether the requested observation can change a decision, and whether every resulting branch has a defensible non-actuating response.

Sources:
- https://proceedings.mlr.press/v306/yan26r.html
- https://bowen-yan-research.github.io/papers/vla-fault-recovery/
- https://kakigo.github.io/VLA-FixBench/

### 2.6 WorldOracle — action-conditioned prediction fidelity

WorldOracle (arXiv:2610.18826, 2026-10-13) evaluates action-conditioned world models against paired real-world executions. It evaluates physical state correctness, interaction spatial alignment, key interaction-area fidelity and multi-view action coherence across seven representative models.

This is a complementary predictive-validity anchor. AERB should use prediction outputs only as candidate hypotheses; it then asks whether their uncertainty and selected observations support a safe decision. A plausible imagined rollout is not evidence that the corresponding physical effect occurred.

Source: https://arxiv.org/abs/2610.18826

## 3. Existing Luminous components: compose rather than duplicate

| Existing work | Owned capability | AERB integration |
|---|---|---|
| Symthaea decision-predictability `robust.rs` | Prior-free one-probe minimax value over set-valued outcome supports | Reuse for guaranteed worst-case decision-loss reduction; do not create a second minimax kernel |
| Symthaea PR #13814 | AERB eight-world lost-ack robust conformance example | Reuse the known-answer loss table and scope-closure / weak-probe controls; its first exact-head run failed only at rustfmt, and a formatting/duplicate-workflow correction is now committed for requalification |
| Symthaea PR #13571 | Prior-free finite-horizon sequential policy oracle plus separate exhaustive enumerator | Reuse for conditional observation trees and branch-dependent resource receipts; still draft and not exact-head qualified |
| Symthaea PR #13911 | AERB integration example stacked on #13571 | Exercise journal-first then conditional scope-reconciliation only where it improves the worst-case terminal loss; qualification pending |
| Symthaea PR #9821 / issue #9817 | Decision-focused active sensing with expected task-loss reduction and six resource dimensions | Reuse for calibrated probabilistic sensing/resource Pareto frontiers; keep distinct from prior-free robust minimax |
| Symtropy #7555 / #7577 | Decision-sufficient stopping and evaluator-side omitted-alternative checks | Reuse for false-exclusion, false-unique and unnecessary-observation scoring |
| Symtropy #7515 / #7563 and evidence-adapter work | Evidence capsules, provenance, signatures and cross-language digest hardening | Reuse for raw observation and receipt provenance; digest equality is not authentication or physical truth |

The current v0.1 AERB contract remains unchanged. The v0.2 extension must explicitly distinguish:
- pre-observation probe selection;
- a no-new-actuation guard;
- candidate-predicted observation supports;
- evaluator-held actual supports and loss tables;
- the one executed observation branch;
- the candidate's conditional response plan;
- evaluator-only sequential oracle policies.

The sequential oracle is not an execution authority. The active-sensing planner's Bayesian/expected result is admitted only with justified, calibrated priors and joint outcome models. Keep probabilistic EVSI, prior-free robust minimax and finite-horizon sequential minimax as different estimands.

## 4. Competitive research thesis: a multi-axis assurance result

Do not publish one weighted AERB score. Publish a report with separate axes:

1. **Execution finality:** false Applied, false NoEffect, false closure, duplicate/partial/stale-epoch effect and unaccounted command-holder outcomes.
2. **Epistemic coverage:** whether actual hidden history lies inside candidate support; report false exclusions and support size.
3. **Decision safety:** unsafe fresh-attempt recommendation, unsafe contingent branch, false-unique and decision-regret.
4. **Observation value:** baseline minimax risk, worst-case post-observation risk, guaranteed reduction, selected-probe regret and decision-sufficient stopping.
5. **Recovery utility:** host-native recovery success and time-to-safe-recovery.
6. **Evidence quality:** decision-cut integrity, stale/missing/disputed observations, source/clock scope, raw artifact completeness and independent replay.
7. **Resources:** probe cost, physical sensing energy, wake energy, compute, latency, p95/p99 decision time, memory, bytes and observation failures.
8. **Transfer:** performance by held-out robot, controller, firmware, endpoint, sensor modality, clock domain, task family and source trajectory.

No safety/finality endpoint may be traded off against task success by a weighted average.

## 5. Baseline matrix

Every baseline must receive the same candidate-visible evidence, command profile, permitted probes, budget, deadline and compute envelope.

### Execution and recovery baselines
- Native controller / behavior tree / VLA / WAM with no AERB observer.
- Immediate retry as a deliberately unsafe synthetic negative control, not as a claim about all industrial controllers.
- Fixed wait, endpoint journal only, independent physical sensor only, endpoint dedupe/fencing, and scope-closure reconciliation.
- Native RoboRecover / LIBERO-RECOVER / VLA-FixBench recovery policy with the RRVR overlay disabled and enabled.

### Observation-selection baselines
- No observation / hold.
- Cheapest eligible probe.
- Uniform random eligible probe.
- Fixed-cadence and event-triggered sensing.
- Maximum entropy / EIG when a calibrated joint likelihood model exists.
- Bayesian EVSI / expected task-loss reduction when priors and likelihoods are justified.
- Prior-free one-step robust minimax.
- Prior-free sequential minimax policy oracle.
- Evaluator-only optimal probe/tree as a labelled upper bound, never a candidate system.

### Observer ablations
- Rules-only.
- HDC-only.
- Temporal predictor only (constant velocity / Kalman / AR / GRU-LSTM-TCN / CfC-LTC as matched).
- HDC + temporal predictor.
- HDC + temporal predictor + calibrated selective support.
- Full observer plus the existing independent evidence/authority controls.

Model size, training data, observation history, inference compute and action/probe budget must be matched or explicitly reported.

## 6. Adversarial benchmark matrix

Required controlled fault families:
- lost, delayed, duplicated, reordered and forged acknowledgements;
- command dispatched but no effect; effect occurred once; partial effect; duplicate effect;
- stale authority epoch and stale queue generation;
- endpoint restart with buffered or in-flight commands;
- multiple command holders, forwarding loops and incomplete system-boundary inventories;
- clock drift, cross-domain timestamp skew and decision-cut leakage;
- source corruption, correlated sensors and common-cause observations;
- observation failure, stale receipt, delayed acquisition, deadline miss and budget exhaustion;
- an ordinary probe that appears conclusive but cannot eliminate open-world residual;
- a high-entropy probe with zero decision-loss reduction;
- a low-information probe that prevents an unsafe retry;
- decision-equivalent histories that do not justify further sensing;
- a conditional policy tree that is safe on common outcomes but unsafe on one rare admissible branch;
- candidate-visible leakage of the seed, hidden truth, actual supports or evaluator loss table;
- a simulated/counterfactual branch accidentally recorded as factual actuation evidence.

Freeze scenario families and splits before tuning. Hold out entire controller, endpoint, sensor, firmware, source and task families—not only random seeds.

## 7. Target ladder (proposed, not achieved)

| Qualification area | Initial target | Stretch target |
|---|---:|---:|
| Exact finite-world oracle agreement | 100% on exhaustive small fixtures | Mutation/fuzz and independent-language recomputation |
| Hidden truth in candidate support | 100% in the fully specified synthetic model | ≥99.9% in prospectively qualified, calibrated held-out regimes, with selective coverage reported |
| False finality / unsafe fresh-attempt in exhaustive negatives | 0 | 0 under adversarial mutation and independent evaluator |
| Robust observation selection | Positive, reproducible improvement over fixed/simple baselines where available | ≥25% lower worst-case decision loss at equal-or-lower resource envelope |
| Sequential policy regret | Report exact gap to small-world exhaustive oracle | <10% of available oracle gain where the oracle is well-defined |
| Decision-sufficient stopping | No unnecessary probe in exact decision-equivalent fixtures | Maintain quality while reducing sensing cost on held-out tasks |
| Replay / provenance integrity | 100% exact source/profile/action-prefix binding in controlled fixtures | Independent verifier reproduces all scored outcomes |
| Transfer | Report all source/controller/task strata | Independent replication on a second robot/controller/sensor stack |

These are research targets, not safety limits, standard thresholds or observed performance.

## 8. Statistical and benchmark-validity rules

- Pre-register primary endpoints, candidate support rules, loss table, cluster unit, missingness and stopping rules.
- Pair control and treatment on the same frozen scenario inputs; retain independent realized outcomes.
- Resample complete independent scenario/trajectory clusters, not frames or repeated attempts as if they were independent.
- Keep unknown, disputed, invalid and unresolved cases in explicit denominators; never silently drop them.
- Zero observed failures are not zero risk. Under an independently justified Bernoulli-cluster model, use exact one-sided bounds; otherwise report descriptive rates without population-risk claims.
- Publish risk/coverage curves, worst-regime strata, probe-selection regret and component-wise resources.
- Preserve host benchmark protocols and native scores. AERB is an overlay, not a replacement score.
- Use a blind evaluator environment. Separate folders in one process are not a security boundary.

## 9. Evidence tiers

1. Formal/synthetic finite-world known-answer models.
2. Deterministic process-model campaigns.
3. Simulator episodes with independent reset, action, effect and observation receipts.
4. HIL with independently captured actuator/controller/endpoint evidence.
5. Controlled physical trials with independent safety oversight and the applicable robot-cell risk assessment.

A result at one tier cannot be promoted to a higher tier without new evidence. Passing AERB does not establish compliance with ISO 10218, functional safety, safety certification or a complete robot-cell safety case.

## 10. Work order

1. Preserve AERB v0.1 and complete the sequential-oracle exact-head qualification separately.
2. Add v0.2 candidate schema: independent actuation guard, probe request, candidate-predicted supports and bounded outcome-to-disposition plan.
3. Add the narrow AERB adapter to the existing robust one-step and sequential evaluators; do not fork either algorithm.
4. Add an independent exhaustive small-world oracle and mutation/metamorphic tests.
5. Add evaluator scoring for false exclusions, contingent-policy worst-case loss, probe regret, risk reduction, stop correctness and resource-path receipts.
6. Run matched fixed-wait, cheapest, random, EIG-if-calibrated, Bayesian EVSI-if-calibrated, one-step robust and sequential robust baselines.
7. Freeze blind source/task/controller holdouts; only then evaluate Symthaea HDC/CfC/LTC observer treatments.
8. Integrate onto exact RoboRecover / LIBERO-Recover host episodes, then simulator, HIL and controlled physical tiers.

## 11. Publication boundary

AERB may claim a new benchmark axis only after a stable public protocol, independently reproducible evaluator, non-leaking blind split and matched external baseline comparison exist. Until then, describe the work as a proposed and partially implemented assurance benchmark. Do not claim SOTA, production safety, real-world calibration or physical effectiveness from schemas or synthetic examples.
