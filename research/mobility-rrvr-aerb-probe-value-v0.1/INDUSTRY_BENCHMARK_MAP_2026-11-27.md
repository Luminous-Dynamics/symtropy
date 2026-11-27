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


## 2.7 ActiveArena — active perception, memory and multi-round acquisition

**Added to the comparison after the 2026-11-27 literature refresh.**

ActiveArena (arXiv:2609.24124, 2026-09-21) is a direct active-perception comparator, not something AERB can ignore. It reports 35 tasks across five fine-grained categories, multi-round evidence acquisition and memory-based reasoning, with disjoint-scene / unseen-distractor / novel-background OOD protocols. It also reports eight real-world manipulation tasks and 640 trajectories; the paper's evaluated memory/planning configuration achieves a 55.6% average success rate on those eight tasks.

**What AERB must not claim:** first active sensing, first multi-round evidence acquisition, or first active-perception/memory benchmark.

**What AERB can add:** command/effect finality ambiguity, source-qualified evidence, open-world command-holder residuals, false Applied/NoEffect/closure scoring, and evaluator-recomputed non-actuating response plans. An ActiveArena observation can improve scene knowledge without proving whether a previously issued motion/dispense/release command has already produced its physical effect.

Sources:
- https://arxiv.org/abs/2609.24124
- https://leeibo.github.io/ActiveArena/

## 2.8 SaPaVe / ActiveManip-Bench — active-view execution

SaPaVe is a CVPR 2026 Highlight. It jointly learns semantic camera movement and manipulation, introduces ActiveViewPose-200K (200,000 image-language-camera movement pairs) and ActiveManip-Bench, and reports 74.83% simulation success and 85.00% real-world success on its project page.

This is a relevant comparator for view-dependent evidence acquisition and its coupling to manipulation. AERB should reuse or adapt active-view episodes when licensing and task mapping permit, but should score a separate question: did the observation qualify the next physical-effect decision, and did it preserve unresolved execution histories?

Source: https://lmzpai.github.io/SaPaVe/

## 2.9 Uncertainty-calibrated runtime gating under domain shift

Ghaleb et al., *Uncertainty-Calibrated Safety Gating for Vision-Language-Action Manipulation Under Domain Shift* (Sensors 2026, 26(10), 3140; published 2026-05-15), evaluates a model-agnostic gate that chooses between learned policy execution, pause-and-reobserve, and a fallback planner. Its two Isaac Sim tasks cover lighting, texture, occlusion, sensor and combined shifts. The paper reports mean shifted success increasing from 57.5% to 77.2% and expected calibration error decreasing from 0.303 to 0.100 against an ungated VLA baseline. It also explicitly reports residual contacts during pause/fallback and notes that its contribution is an empirical reliability/intervention trade-off, not a universal terminal controller.

**Required AERB baseline:** uncertainty-calibrated gating with pause/reobserve and fallback, compared on matched scenarios and equal evidence/resource access.

**Potential AERB distinction:** the calibrated gate estimates the risk of future failure under distribution shift; AERB additionally models that the previous command may already have caused an effect, that acknowledgement may be absent, and that an incomplete command-holder scope may allow a late second effect. A risk score or fallback switch does not by itself establish NoEffect or close an effect-producing system boundary.

Source:
- https://www.mdpi.com/1424-8220/26/10/3140
- DOI: https://doi.org/10.3390/s26103140

## 2.10 Runtime Assurance / Simplex — architectural prior art, not novelty

Runtime assurance (RTA), including Simplex-style architectures, is established safety-engineering prior art. NASA's public framework describes an advanced controller monitored by a trusted runtime monitor, with control handed to a trusted reversionary controller when a property is violated. The framework also makes monitor sampling rate and switch timing explicit verification concerns.

AERB/RRVR should be described as **complementary to runtime assurance**, not as inventing it. The integration question is how the monitor obtains a trustworthy current execution/effect state and whether the reversionary/fallback controller itself is protected against duplicate, stale or late-arriving commands.

Required baseline/ablation:
- ordinary Simplex/RTA policy monitor and trusted fallback;
- calibrated VLA runtime gate;
- the same system with RRVR's durable intent, endpoint effect fencing, scope closure and independent evidence checks enabled.

The RTA monitor and fallback remain responsible for safety properties and control invariants. AERB supplies a benchmark for ambiguity, evidence acquisition and execution-finality around the handover and subsequent action.

Source: https://ntrs.nasa.gov/citations/20240007986

## 2.11 Refined novelty/claim boundary

The public landscape now clearly contains work on:
- task-aware safety monitoring (SafeVLA-Bench);
- active perception, viewpoint movement and memory (ActiveArena, SaPaVe);
- calibrated runtime intervention under domain shift (uncertainty-calibrated safety gating);
- trusted fallback/runtime assurance (Simplex/RTA);
- fault diagnosis/recovery (VLA-FixBench, LIBERO-RECOVER, RoboRecover);
- action-conditioned world-model fidelity (WorldOracle).

Accordingly, AERB's defensible research claim must be narrower and experimentally testable: **a benchmark for independently evidenced physical-effect finality and adaptive, decision-valued reconciliation under open-world execution ambiguity, evaluated before the next potentially irreversible effect.** Its novelty must be established by a systematic comparison and blind matched experiments, not asserted from architecture diagrams alone.


## 2.12 RRVR temporal closure — command-path quiescence is not a sensing outcome

RRVR's own temporal-closure track (#7433 / draft PR #7435) provides the relevant assurance boundary. It tests whether old commands can still arrive after a system claims its command path is quiescent. It models admission closure, in-flight closure, downstream enqueue closure, terminal endpoint fencing and evidence freshness across the exact command-holding graph.

This is distinct from AERB's physical-effect question:
- temporal closure asks whether a prior command can still execute in the future through the declared path;
- effect observation asks whether the command already produced its physical effect;
- neither a closed command path nor an empty queue proves NoEffect.

AERB's candidate should consume a qualified, exact-scope temporal-closure receipt from the existing owner, retaining Unknown/Hold and `open_world_residual` whenever the graph is incomplete, an edge is unbounded/unqueryable, a clock is invalid or a receipt is stale. AERB must not implement a second command-holder graph or late-arrival proof engine.

The first #7435 focused workflow reached its expected hash-check step and failed because two recorded Git blob pins were stale; JSON/schema checks had passed, while TLC, rustfmt, Rust tests and Clippy were skipped. The two fixture pins were corrected on the latest branch head; a new run is queued, not yet qualified.

Sources:
- https://github.com/Luminous-Dynamics/symtropy/issues/7433
- https://github.com/Luminous-Dynamics/symtropy/pull/7435
- https://github.com/Luminous-Dynamics/symtropy/issues/6817
- https://github.com/Luminous-Dynamics/symtropy/pull/6866

## 2.13 Evidence capsules — candidate observations must contain decision-relevant content

The existing RRVR Evidence Capsules v0.1 package (#7515) already has typed source-bound evidence payloads for physical effect findings, endpoint fence receipts, forwarding-edge closure receipts and holder inventories. Its validator was exact-head qualified on Rust 1.96 (11 tests, format and strict Clippy) for software semantics only.

AERB v0.1's candidate-visible Observation currently exposes identity, role, quality, event/acquisition times, ingest sequence, artifact digest and an `observation_kind` string, but no typed observation result. A digest is a commitment to bytes, not the measurement or receipt itself. An observer cannot genuinely revise its possible-history support if it cannot inspect the admitted result.

The v0.2 follow-on [#7728](https://github.com/Luminous-Dynamics/symtropy/issues/7728) therefore requires a source-qualified, decision-cut-bounded materialized payload projection that reuses #7515, with safe read-only host mediation for larger raw artifacts. The candidate must see what the qualified source actually returned, while evaluator truth, actual support partitions, priors and loss tables remain hidden.


## 2.14 Blind benchmark generation — public deterministic fixtures are not blind tests

The v0.1 `aerb_generate` tool is useful for public deterministic conformance, but its current algorithm is not a blind challenge generator:
- episode IDs are sequential;
- `scenario_id` exposes `index % 8`;
- hidden truth is assigned from a deterministic, seed-shuffled cycle with each of eight worlds appearing exactly once per block;
- the public CI workflow invokes the generator with seed `42`.

The seed and truth map are not written into candidate files, but the code, workflow seed and candidate-visible IDs/position expose enough information to reconstruct or infer the hidden assignment. A stateful candidate can also exploit the exact block-balance property.

This does not invalidate the declared v0.1 public synthetic smoke test or its broad-support baseline. It does invalidate using the same generator/IDs as a blind learned-agent benchmark.

New issue [#7732](https://github.com/Luminous-Dynamics/symtropy/issues/7732) separates:
- a **public deterministic conformance corpus**, where fixed seeds and visible IDs are acceptable; and
- a **blind challenge corpus**, where hidden assignments are made with evaluator-only cryptographic entropy, candidate IDs/order are opaque and uncorrelated, and the assignment is revealed only after the campaign under an auditable commitment.

The blind-corpus acceptance test includes metadata-only classifiers and stateful sequential inference attacks. Exact quota balancing must not create candidate-visible sequence leakage. The evaluator must keep the true cluster/stratum labels separately while candidate metadata remains non-informative about hidden world identity.


## 2.15 DiMuRe — diagnosis-guided multilevel recovery

**Added in the 2026-11-27 literature refresh.**

DiMuRe (arXiv:2611.04226, 2026-11-03) is a direct recovery comparator. It uses phase-specific evidence and method preconditions to select program reuse or recovery skills, can generate corrective code after program exit, and locally verifies recovery attempts. Its LIBERO-Disturb benchmark targets mid-execution disturbances.

The abstract reports, on 801 scheduled disturbed Encore development cases:
- Local DiMuRe: 77.65% complete-task success.
- Original program under the same physical-step allowance: 58.68%.
- Full DiMuRe: 81.90%, with terminal replanning, additional step allowance and early handoff.

Do not compare the 81.90% result to the 58.68% baseline as though the physical-step and replanning budgets were equal. Preserve the protocol differences and report physical actions/steps separately.

**AERB addition:** measure whether the previous attempt's physical effect is independently known before another recovery attempt, whether a recovery probe can discriminate decision-relevant histories, and whether its acquisition is safe over the full sensing interval. DiMuRe's local verification is complementary; this is not a claim that DiMuRe lacks verification.

Source: https://arxiv.org/abs/2611.04226

## 2.16 EmbodiedGovBench — capability and lifecycle governance

**Added in the 2026-11-27 literature refresh.**

EmbodiedGovBench (arXiv:2604.11174, 2026-04-13) frames embodied-agent governance as a first-class evaluation dimension. It covers seven areas: unauthorized capability invocation, runtime drift robustness, recovery success, policy portability, version-upgrade safety, human override responsiveness and audit completeness.

This expands the comparator set beyond task-level manipulation. AERB/RRVR should align its evidence and commit records with governance requirements rather than presenting recovery as an isolated skill.

**AERB addition:** bind the decision's evidence dependencies, possible-world support and counterfactual policy to the exact current action-ticket at the pre-effect boundary. #7788 implements the first narrow adapter slice; it is not yet qualified.

Source: https://arxiv.org/abs/2604.11174

## 2.17 Standards and industrial assurance boundary — 2026-11-27

Benchmarks and standards answer different questions. AERB scores observable assurance behaviour in bounded experiments; it does not replace a machine risk assessment, a safety-related control system, a validated safety function or conformity assessment.

Relevant published and draft references:

| Standard / draft | Scope and current status | RRVR integration boundary |
|---|---|---|
| ISO 10218-1:2025 | Published 2025-02; industrial robots as partly completed machinery | Bind the robot identity/configuration, safety-function and evidence references; do not claim robot-level compliance from an AERB result |
| ISO 10218-2:2025 | Published 2025-02; industrial robot applications and robot cells, including integration and lifecycle operation | Keep application/cell hazards, end-effectors, workspace and integration evidence in the application safety case |
| ISO 13849-1:2023 | Published methodology for safety-related parts of control systems in high-demand/continuous modes | AERB must not substitute software decision tests for required PLr, architecture, validation or safety-function evidence |
| ISO 12100:2010 | Current risk-assessment and risk-reduction methodology; ISO/DIS 12100.3 is under development | AERB evidence can support hazard-analysis traceability, but cannot make risk acceptance decisions |
| ISO 3691-4:2023 / ISO/DIS 3691-4 (Edition 3) | 2023 version is published; a replacement DIS is in enquiry/development in 2026 for driverless industrial trucks and their systems, including relevant AMR classes | Use only for the in-scope industrial truck operating domain; preserve the exact edition and draft status in every compliance mapping |

Important scope caveats:
- ISO 10218-1:2025 excludes, among other applications, airborne/space robots, service robots with public access, consumer products and robots integrated on mobile platforms. Do not use it as a universal robot standard.
- The 2026 ISO/DIS 3691-4 draft describes driverless industrial-truck hazards and verification, but explicitly excludes hazards related to self-evolving behaviour or logic (e.g. AI). It is a draft, not a published final replacement standard.
- ISO 13849-1:2023 provides a methodology; it does not define the required performance level for a specific application and does not itself provide cybersecurity requirements.
- ISO 12100 establishes risk-assessment/risk-reduction principles; it does not make an empirical benchmark a safety case.

**Strategic consequence:** RRVR should be a versioned evidence and verification layer that helps preserve traceable claims, assumptions, configuration and change history for the applicable safety lifecycle. It must not claim to certify an AI policy, qualify a safety PLC, prove functional-safety performance or satisfy a standard merely because its software tests pass.

Official sources:
- https://www.iso.org/standard/73933.html
- https://www.iso.org/standard/73934.html
- https://www.iso.org/standard/73481.html
- https://www.iso.org/standard/51528.html
- https://www.iso.org/standard/83545.html
- https://www.iso.org/standard/88615.html

## 2.18 Updated research differentiation

The refreshed comparison supports a narrower, stronger proposition:

**AERB is an execution-ambiguity and evidence-to-effect assurance overlay. It does not compete to replace active perception, fault diagnosis, VLA recovery, temporal safety monitoring, runtime assurance, safety-related control systems or industrial safety standards.**

Its research claims should be tested across three additional boundaries:

1. **Recovery decision boundary:** whether independently admitted evidence justifies reusing, adapting, stopping or retrying the original task.
2. **Observation boundary:** whether a proposed information-gathering trajectory is safe and decision-relevant over its entire physical duration.
3. **Effect boundary:** whether the evidence-supported decision remains current at the exact action-ticket/commit cut, including late arrivals, revocation, scope changes and A→B→A revision histories.

The claim ceiling remains synthetic/process-model software conformance until there is independent simulator, HIL and controlled physical evidence. Standards mapping remains an applicability aid, not conformity assessment.
