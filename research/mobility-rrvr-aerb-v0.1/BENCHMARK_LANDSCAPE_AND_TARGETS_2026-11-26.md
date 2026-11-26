# RRVR / AERB benchmark landscape and competitive targets

**Snapshot date:** 2026-11-26  
**Status:** literature and public-leaderboard comparison for protocol design. Numbers below are the sources' reported numbers, not Symtropy or Symthaea results.

## Executive assessment

The field is converging on three adjacent but distinct questions:

1. **Can the policy complete the task?** Generalist policy suites such as RoboCasa365 compare atomic and multi-step manipulation success across seen and unseen splits.
2. **Can the policy avoid unsafe execution and recover from faults?** SafeVLA-Bench, LIBERO-RECOVER and VLA-FixBench measure safety, fault diagnosis and recovery.
3. **Can a world model predict action-conditioned physical outcomes?** MiraBench and RoboWM-Bench test action fidelity, physical plausibility and execution of imagined actions.

RRVR/AERB should not compete by reproducing one aggregate success-rate number. The proposed contribution is an assurance/evidence layer across these axes: preserve all plausible execution histories, prove source and replay provenance, choose observations by worst-case decision value, and independently reconcile whether a physical effect occurred before permitting another attempt.

The reviewed benchmark descriptions do not present this complete set-valued evidence and decision-risk protocol as a primary scoreboard. That is a research gap to test, not a claim that no other unpublished or proprietary system has similar functionality.

## Public landscape

| Benchmark / standard | What it measures | Public reference point | What RRVR/AERB should add |
|---|---|---|---|
| RoboCasa365 | Generalist manipulation across atomic/composite and seen/unseen splits | Current public leaderboard snapshot: 50 target tasks, 20 evaluated models, 3 splits; top overall HexaShell 85.6 (91.1 atomic-seen, 85.0 composite-seen, 80.0 composite-unseen). The top overall entry is labelled Agent, so do not mislabel it as a VLA score. The next entries include VLA models with lower overall scores. | Evaluate recovery and integrity conditional on controlled failure injection; retain the 50-task score only as a task-utility axis. |
| SafeVLA-Bench | Success/safety gap under explicit safety specifications and violation severity | 24 VLA policies / 27 policy-benchmark entries. The authors report that all 15 LIBERO policies above 90% mean success still leave 18–28% of rollouts unsafe; for RoboCasa-365, 38–56% of native successes violate an active safety clause across the nine policy results. | Add pre-retry truth reconciliation, temporal authority/fencing correctness, unknown/stale/disputed evidence and set-valued uncertainty. Do not collapse success and safety into one weighted score. |
| LIBERO-RECOVER | Recovery after manipulation failure, four levels: retry, adaptation, object-state recovery, environmental recovery | More than 1,000 scenarios, built from real execution failures collected from embodied models. | Add uncertain execution outcomes, independent effect evidence, command/authority ambiguity, residual open-world risk and observation-selection value. |
| RoboRecover | Reconstruct deviation states by replaying action prefixes and continue the original task | 2,000 scenarios across RoboTwin and LIBERO; 1,000 per platform with an 800/200 train/test split on each. | Keep the three-attempt, five-frame-per-attempt fidelity gate distinct from the exact RoboTwin state/action-step/all-camera verifier; add source artifact and per-attempt receipt authentication. |
| VLA-FixBench / FaultEval | Fault diagnosis and repair across perception, planning and control; static, dynamic and real-robot evaluations | 6,034 task-execution episodes across 40 simulated settings and two real-robot tasks (tea preparation and charging-plug insertion); 20 VLMs evaluated. The paper's human-intervention upper-bound analysis reports up to +13 percentage points on LIBERO and +35 points on its real-robot tasks. Those are expert upper bounds, not ordinary model gains. | Score whether a diagnosis is supported by admitted observations, whether rollback/retry is authorized by independently reconciled effect evidence, and whether the selected next observation improves decision quality. |
| PAW-GEN-10 / Poke & Wiggle Reality Check | Real-robot rollouts, spatial generalization, execution quality, smoothness, safe failure and contact force | The authors report 3,600 rollouts per policy for the initial four-model benchmark, with 30 rollouts per condition and reproducible scenes; the public board exposes nominal/interpolation/extrapolation tracks and per-environment metrics. | Match physical repeatability, then add fault-injection/recovery traces and receipt-bound effect truth. Physical evidence is a separate tier, not inferred from simulation. |
| MiraBench | Action-conditioned world-model reliability: physics adherence, action following and optimism-bias detection | More than 16,000 human judgments over tasks/failure categories and 12 model configurations; reports visual fidelity is not a reliable proxy for action fidelity and optimism bias is common. | Extend from model prediction to whether the model's uncertainty changes a safe operational decision and whether post-observation decisions remain calibrated. |
| RoboWM-Bench | Whether generated manipulation futures can be converted into executable embodied actions | Converts generated behavior into action sequences and validates through robotic execution. | Require source/receipt provenance, independent physical effect reconciliation and failure-containment outcomes in addition to executability. |
| Manipulation benchmark audit | Benchmark validity: shortcut solvability, statistical significance, overfitting and data-source dependence | A 2026 audit reports these failure modes can invalidate manipulation leaderboard interpretations and provides diagnostics for LIBERO, CALVIN, SimplerEnv, RoboCasa and RoboTwin 2.0. | Make anti-shortcut, preregistration, cluster holdout, data-lineage and significance tests mandatory gates before an AERB result can be presented as a model advance. |
| ISO 9283:1998 | Industrial-manipulator performance test methods: accuracy, repeatability, stabilization, overshoot, drift, path accuracy and related characteristics | Current ISO publication; last reviewed and confirmed in 2021. Its comparison guidance requires matching test cube, load, velocity, path, cycle and environmental conditions. | Use its matched-condition discipline for physical motion comparisons, and separately measure recovery integrity, evidence quality and time-to-safe-reconciliation. |
| ISO 10218-1:2025 and ISO 10218-2:2025 | Robot-level safety requirements and industrial robot application/cell integration safety | Published 2025 editions. Part 2 includes integration, commissioning, operation, maintenance and decommissioning; the series is safety requirements, not a VLA leaderboard. | Position RRVR/AERB as an evidence/assurance and benchmark layer that complements, but does not replace, risk assessment, certified safety functions or standards conformity. |

### Source register

- RoboCasa365 official leaderboard: https://robocasa.ai/leaderboard.html (updated 2026-11-25).
- SafeVLA-Bench: https://safevla.org/ and https://arxiv.org/abs/2606.00773.
- LIBERO-RECOVER: https://arxiv.org/abs/2609.05178.
- RoboRecover: https://arxiv.org/abs/2609.28952.
- VLA-FixBench / FaultEval, ICML 2026: https://proceedings.mlr.press/v306/yan26r.html.
- Poke & Wiggle Reality Check: https://pokeandwiggle.com/leaderboard and https://pokeandwiggle.com/blog/introducing-paw-10.
- MiraBench: https://arxiv.org/abs/2605.29360.
- RoboWM-Bench: https://arxiv.org/abs/2604.19092.
- Manipulation benchmark audit: https://arxiv.org/abs/2606.04233.
- ISO 9283: https://www.iso.org/standard/22244.html.
- ISO 10218-1:2025: https://www.iso.org/standard/73933.html.
- ISO 10218-2:2025: https://www.iso.org/standard/73934.html.

## Paradigm shifts to pursue

### 1. From point estimate to possible-history envelope

After a lost acknowledgement, retain the possible histories—not one guessed state. The public candidate snapshot contains only evidence available at its decision cut. Hidden physical truth, world supports and evaluator loss remain evaluator-only.

Required known states include not dispatched, dispatched/no effect, applied once with acknowledgement lost, partial effect, duplicate effect, stale-authority effect, unresolved holder, and an open-world residual. The residual is important: a finite fault taxonomy must not be treated as exhaustive merely because the schema lists every currently known state.

### 2. From prediction to evidence-qualified effect reconciliation

A model forecast is not a sensor observation. A controller acknowledgement is not proof of physical effect. A journal read is not proof of complete command-holder closure. RRVR should require independently sourced, scope-bound evidence before adjudicating Applied/NoEffect or enabling a new attempt.

### 3. From entropy seeking to decision-risk-reducing observation

Use prior-free minimax when priors/joint likelihoods are not justified. Rank probes by guaranteed worst-case loss reduction under frozen supports. Keep probe cost, delay, permissions and deadlines as separate constraints. A high-information probe that does not alter the safe decision is a negative control; a less informative observation that rules out an unsafe retry can be the positive control.

### 4. From closed-world assurance to explicit residual-risk boundary

The candidate binds an execution scope and system-boundary manifest. The open-world residual may be narrowed only by an independently verified closure receipt covering effect-producing components, command holders and forwarding edges. A self-asserted completeness field is not authentication.

### 5. From benchmark episodes to replayable evidence transactions

Bind the selected attempt to its source episode, policy roster, simulator/robot/controller/render profiles, reset receipt, action-prefix artifact, state artifacts, image frame pairs and outcome report. The experiment should be replayable as a chain of typed, digest-bound receipts—not a table of aggregate scores.

### 6. From one aggregate score to a multi-axis assurance frontier

Report task utility, hard-safety events, hidden-truth support coverage, false finality, recovery time, observation cost, physical repeatability and compute separately. Do not trade safety failures against success, smoothness or latency via a weighted scalar.

## Proposed AERB target ladder

Targets below are aspirations for a preregistered benchmark campaign, not achieved results or certification thresholds.

| Tier | Target | Why it matters |
|---|---|---|
| T0 — schema/process conformance | 100% of candidate inputs bind to one snapshot digest and decision cut; zero future leakage; zero stale/disputed/wrong-scope support shrinkage; all probe outcome supports cover their declared world catalog. | Proves the benchmark itself is not leaking truth or silently dropping ambiguity. |
| T1 — deterministic process model | Zero false Applied/NoEffect/closure and zero unsafe fresh-attempt recommendations in known-answer negative controls; deterministic reports for frozen inputs; independent evaluator recomputation. | Establishes that the assurance rules work under controlled fault semantics. |
| T1 — epistemic coverage | Report truth-in-support, support size, unsupported eliminations, selective risk-vs-coverage and abstention by fault family. Target 100% support coverage on generated known-answer states; target at least 95% coverage on a preregistered distributional holdout while minimizing unnecessary set size. | Avoids rewarding a model that is safe only because it returns every world, or confident only because it drops inconvenient worlds. |
| T1 — decision value | At matched probe budget, deadlines and information, target ≥20% lower worst-case decision loss than the strongest fixed-cadence / threshold / entropy baseline on at least one preregistered regime, with no increase in hard-safety endpoints. Publish null results if not reached. | A direct, falsifiable improvement over current observation heuristics. |
| T1 — resource efficiency | Report p50/p95/p99 latency, compute, memory, bytes, observation count and time-to-reconciliation. Target ≥25% fewer probes or ≥25% faster safe reconciliation than a matched fixed-cadence baseline without loss of support coverage. | Makes evidence-aware recovery useful in real production cells. |
| T2 — independent simulator | Cross-platform RoboTwin/LIBERO; hold out fault types, object families, scene layouts, controller profiles and simulator configurations; use separate model/plant parameters and independent effect observations. | Prevents same-simulator prediction loops from masquerading as physical accuracy. |
| T3 — HIL | Independently measure sensor acquisition, controller/actuator output, authority epoch, queue generation and endpoint effect. Preserve stale/delayed/correlated evidence. | Tests the software-to-hardware timing and evidence chain. |
| T4 — physical cell | Matched ISO 9283 conditions where applicable; preregister safe fault injection; independent pose/contact/force observation; compare policy, fixed wait, journal-only, endpoint-fence and robust-observation arms. | Establishes real-world behavior, not simulation extrapolation. |

### Rare-event confidence target

A zero-failure pilot is not evidence of zero risk. Under an IID independent-cluster Bernoulli assumption, the familiar rule-of-three approximation says that about 3,000 independent clusters with zero observed events are needed for a one-sided 95% upper bound near 0.1%. This is only a planning approximation: use an exact binomial bound for an IID design, and a defensible cluster-level method when observations share a scenario, trajectory, robot, operator, or environment. Frames within one episode are not independent trials.

## Required baselines and ablations

Every comparison should match observations, probe menu, safety authority, deadlines, action budget and compute. Include:

- immediate retry (synthetic negative control; not a proxy for every industrial recovery system);
- fixed wait / fixed-cadence observation;
- event/threshold-triggered observation;
- uncertainty-triggered observation;
- journal-only reconciliation;
- independent pose/force/contact observation;
- endpoint deduplication/fencing and command-holder closure;
- entropy / expected information gain;
- Bayesian EVSI only with justified priors and joint likelihoods;
- prior-free robust minimax observation selection;
- persistence/constant-velocity, Kalman/LQG, AR/ESN, GRU/LSTM/TCN/state-space baselines;
- HDC-only, temporal model-only (CfC/LTC), and HDC+CfC/LTC;
- human expert diagnosis/intervention as a non-deployable upper bound.

Keep the same evidence available to all compared systems. No baseline gets hidden truth, future observations or a more generous probe budget.

## Benchmark-validity gates

Apply the four diagnostics highlighted by the 2026 manipulation-benchmark audit:

1. **Shortcut solvability:** include trivial low-capacity and non-language baselines; test whether success can be achieved without the claimed capability.
2. **Statistical significance:** preregister sample size, estimand, clusters, seeds and stopping; report intervals and paired outcomes.
3. **Overfitting:** use task, scene, object, fault-family and source-trajectory holdouts; keep private challenge scenarios and rotate them.
4. **Data-source dependence:** disclose policy training data and source overlap; compare on independent source streams and report lineage.

Also require: all invalid/unknown/disputed cases remain in denominators; model/checkpoint/profile digests; full test harness and raw artifacts; evaluator version; independent rerun.

## Competitive claim discipline

A future SOTA claim must identify the exact benchmark, split, task count, attempt count, robot/simulator, fault regime, safety specification, hardware, model/checkpoint, information and compute budget. Report success and safety separately. AERB should first demonstrate a new assurance metric with an independent evaluator; only later compare end-to-end models on a matched, public, reproducible campaign.

The near-term win is not "beat RoboCasa overall." It is: **match a strong baseline on task completion, materially reduce unsafe/false-finality events, preserve calibrated uncertainty, and achieve faster/cheaper safe reconciliation under the same evidence and action budget.**
