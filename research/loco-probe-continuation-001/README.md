# LOCO-PROBE-CONTINUATION-001 — Viable continuation after active contact probes

**Status:** research protocol and finite synthetic fixtures. No physical safety result or locomotion superiority claim.

Issue: [#4287](https://github.com/Luminous-Dynamics/symtropy/issues/4287).

## 1. Research question

Can an active contact probe improve the next locomotion decision **without sacrificing robustly viable continuation across every physically possible probe trajectory and observation outcome**?

A probe is a physical action, not a free measurement. It changes support, momentum, body configuration, actuator load, energy and timing before its observation becomes available. Evaluating its information value at a frozen pre-probe state can overvalue a probe that leaves the robot with fewer or no recoverable continuations.

This workstream is the composition layer between the physical consequence of a probe and decision selection after the probe. It does not create another contact estimator, hybrid physics solver, robust-viability fixed-point engine, chance-risk ledger or general POMDP.

## 2. Existing ownership

| Capability | Existing owner | Consumed contract |
|---|---|---|
| Contact evidence and action-conditioned likelihoods | #4004 / #4008 | Sensor evidence, action and likelihood identity |
| Joint correlated contact fusion | #4017 / #4230 | Calibrated joint update; no independence shortcut |
| Belief-space semantics | #4100 | Epistemic, aleatoric and sensor-health separation |
| Whole-system energy boundary | #4044 | Battery/source, DC bus, actuator electrical and mechanical work ledgers |
| Actuator loss maps and sim-to-real transfer | #4179 | Calibrated actuator electrical/mechanical loss model; PACE comparator |
| Command-to-motion actuator dynamics | #4206 | Delivered motion, delay, saturation, stiction and transient friction |
| Finite robust viability/recovery | #4058 / #4061 | Upstream viable/recoverable state sets |
| Sound hybrid transition abstraction | #4083 | Conservative finite physical branch set |
| Certificate lifecycle | #4117 / #4119 | Model, actuator, clock and deadline validity |
| One-step chance gate | #4010 / #4231 | Optional calibrated probabilistic admission |
| Non-recycling horizon risk | #4232 / #4245 | Current horizon allocation and execution status |
| Fixed-state decision value | #4268 / #4269 / #4272 | Loss matrix and expected VOI comparator |

The viability computation and transition abstraction remain owned by #4061/#4083. This package consumes their exact graph/receipt semantics and checks the probe-specific composition. It must not replace their global viability or soundness obligations.

## 3. Mathematical contract

Let:
- (s_0) be the current abstract locomotion state;
- (u) be the candidate probe;
- (B(u,s_0)) be the complete finite set of physically possible probe trajectories;
- (bin B) be one branch, including its intermediate states during probe execution and observation latency;
- (z_b) be its terminal state when the observation becomes available;
- (O(b)) be the observation outcomes physically possible on branch (b);
- (K_h) be the upstream robust viable-or-recoverable state set for remaining horizon (h);
- (Post(z,a)) be the nondeterministic successor set for continuation action (a);
- (R) be the separately declared recovery/terminal set.

### 3.1 Pre-observation probe gate

For a horizon of (H) decision steps and probe/observation duration (d), every branch prefix must retain its required viable fallback:

[
s_{b,j}in K_{H-j},quad j=1,ldots,d.
]

The terminal successor must also be in (K_{H-d}) (or a specifically declared recovery set under the same horizon semantics).

Any physically possible branch is included, regardless of whether its calibrated probability is small or zero. A probe cannot be admitted because a dangerous branch was assigned a negligible likelihood.

### 3.2 Observation-conditioned physical possibility

For each outcome (y), define:

[
B_y={bin B:yin O(b)},
qquad
S_y={z_b:bin B_y}.
]

This is a possibility relation, not a posterior probability. An informative observation may partition the state set, but it may not collapse physically distinct states unless the supplied observation/transition relation justifies that distinction.

### 3.3 Robust continuation action set

For each possible outcome (y), compute:

[
A_R(y)=
left{a:
orall zin S_y,;
Post(z,a)subseteq K_{H-d-1}cup R
ight}.
]

The successor set for each state/action must be non-empty, complete and source-qualified. If an action is missing for even one state in (S_y), it is not robustly admissible for that observation.

For unknown, missing, contradictory or out-of-model evidence, require a declared fallback action robustly admitted over the entire corresponding state set. If no fallback exists, reject the probe or return Unknown.

### 3.4 Probability is a separate layer

A calibrated joint probability distribution over physical branch and observation may rank task utility or expected regret **only after** robust admission has been completed.

Keep separate:
- physical possibility: which branches must be survived;
- calibrated probability: how expected outcomes are weighted;
- decision loss: what consequences matter to the declared task;
- resource cost: energy, latency, thermal and actuator budgets.

Do not use low posterior mass to prune a possible physical branch. Do not promote an action into (A_R(y)) through Bayesian conditioning alone.

## 4. Receipt and identity contract

Bind the probe result to:
- initial state-set digest;
- transition abstraction and physical-model digest;
- probe, command, delivered-action and measured-actuation identities;
- every branch ID, intermediate state and terminal successor;
- observation vocabulary and physical possibility relation;
- observation likelihood/calibration identity and explicit unknown outcome;
- observation sample/acquisition/arrival time and latency;
- upstream viability/recovery set identity and remaining horizon;
- assurance/certificate-lifecycle receipt;
- actuator model and current thermal/voltage/derating state;
- risk-budget receipt and reconciliation state when chance-risk mode is used;
- continuation-action transition map and result digest;
- energy, force, impulse, support, timing and compute bounds.

The composition evaluator validates declared consistency, freshness, completeness and identity. It does not cryptographically authenticate upstream receipts or prove that a supplied abstraction bounds a physical robot.

## 5. Finite fixture vocabulary

The frozen JSON fixtures are in [fixtures/known_answers_v0_1.json](fixtures/known_answers_v0_1.json). They use deliberately tiny transition systems so expected results can be checked by an independent brute-force enumerator.

### Fixture A — safe probe with branch-specific continuation

Two physically possible branches end at distinct viable states. The observation identifies which branch occurred. Advancing is admitted for the good-support state but not for the degraded-support state; retreat remains admitted for both. The correct observation-conditioned action sets are:
- `grip-confirmed`: `advance`, `retreat`;
- `grip-degraded`: `retreat`.

The probe is admitted because both branches remain viable through the probe and observation delay and every possible observation has a robust continuation.

### Fixture B — high-EIG trap with a zero-probability physical branch

The probe's observation would perfectly identify the state under its probabilistic model. One declared physical branch has zero calibrated probability but ends outside the upstream viable/recovery set. The probe must be rejected before EIG/VOI ranking. This is the negative control for using probability to erase physical possibility.

### Fixture C — observation aliases two physical successor states

Two distinct viable terminal states can both produce the same observation. An aggressive action is safe from one state and unsafe from the other; retreat is safe from both. The correct robust continuation set contains retreat only. A concentrated terrain-hypothesis posterior does not justify pretending the body state is known exactly.

### Fixture D — observation latency destroys the intermediate fallback

The final post-probe state is viable, but an intermediate state during probe/measurement latency falls outside the required viable set. The probe is rejected. Checking only the terminal state would miss the dangerous interval.

## 6. Required baselines

1. Passive/no-probe locomotion.
2. Fixed-state VOI from #4272, without post-probe continuation accounting.
3. EIG-only selection after the same initial admission filter (negative control if it ignores the post-probe state set).
4. Random selection among physically admitted probes.
5. Robust viability-only selection without an information objective.
6. STEP-CBF+-style reduced-order foothold/posture screening.
7. Impact-aware robust MPC under the robot/model scope it supports.
8. Transition-aware robust continuation plus decision-value selection.

All policies receive matched morphology, physical abstraction, candidate actions, observation models, sensor data, horizon risk budget and compute/task budgets. Reduced-order, whole-body, simulation and hardware results must not be pooled as equivalent evidence.

## 7. Scorecard

Report by terrain, gait, individual, contact mode and sensor-health stratum:
- probe-admission false-safe count and Unknown count;
- branch-abstraction containment and omitted-branch count;
- intermediate physical violations during probe/observation latency;
- post-observation viable continuation action count by outcome;
- worst-case continuation set and recovery horizon;
- state-set/option reserve before and after probing;
- fallback availability, fallback use and deadline violations;
- expected decision loss, held-out regret and action-switch probability;
- EIG and realized information gain;
- probe energy, impact, support margin and actuator/thermal utilization;
- consumed, pending, skipped and unknown horizon risk allocations;
- stable-step fraction, slip distance, fall rate, task completion and distance before failure;
- worst-case latency, memory and deadline misses.

Action count is not a universal measure of safety. Report the actual successor and recovery witnesses. Never collapse robustness, calibration, utility and energy into one weighted score.

## 8. External SOTA comparison anchors

### STEP-CBF+ — Djeffal et al., Scientific Reports, 22 September 2026

The paper studies a 13.5 kg quadruped with 3-DOF legs in a crawl gait, screening 49 foothold candidates per decision across six hazardous-terrain conditions and 25 randomized terrain/payload realizations. It reports a 4.0% mean fall rate, 77.3% stable-step fraction and approximately 13.6 ms mean candidate-selection time, versus 30.5 ms for its reduced-order convex-MPC comparator. Importantly, its framework predicts next-support-phase effects and actuator/friction constraints. These results are reduced-order randomized simulation, not a whole-body nonlinear invariance proof or hardware validation.

Source: https://www.nature.com/articles/s41598-026-72507-w

### SlipSense — Liu, Cheah & Chuah, arXiv:2606.24350, 23 June 2026

A sensorized foot and multimodal online slip detector evaluated on Unitree Go1. The preprint reports 85.9% accuracy and 24.1 ± 6.4 mm average slip-detection displacement. Use as a reproduction target within its stated experiment, not an unqualified field-wide SOTA bound.

Source: https://arxiv.org/abs/2606.24350

### Zhao et al. — Sensors, 23 September 2026

Continuous contact reliability plus neural residual Kalman filtering; physical tests cover indoor walking and turning, not complex uneven terrain.

Source: https://doi.org/10.3390/s26196041

### ISO 18646-5:2026

Standardized whole-machine locomotion-performance methods; explicitly not intended for safety verification/validation.

Source: https://committee.iso.org/cms/live/live/en/sites/isoorg/contents/data/standard/08/68/86850.html

### PACE — Bjelonic, Tischhauser & Hutter, IJRR, 31 July 2026

PACE combines bottom-up actuator identification with a physics-grounded permanent-magnet synchronous motor energy model and sim-to-real RL. The paper reports 32% reduction in full Cost of Transport for ANYmal (reported value 1.27) and transfer to ten additional robots. Its published dataset and code enable a reproducible energy/model-transfer comparator. These results are platform and energy-boundary specific; they are not directly comparable with RAIBO2 or Symtropy's FlyGym/SPINDER tasks.

Sources:
- https://doi.org/10.1177/02783649261459628
- https://doi.org/10.3929/ethz-c-000803792
- https://github.com/leggedrobotics/pace-sim2real

### RAIBO2 — Lee et al., Nature, 23 September 2026

RAIBO2 completed a 42.195 km marathon in 4 h 19 min 52 s on one battery charge. The study reports total Cost of Transport 0.25 and more than three times the travel range of existing quadrupeds, attributing the result to a system-level combination of lightweight force-transparent mechanics, low-loss motor driving and locomotion policy. The Nature Portfolio event summary reports 1,280 Wh consumed and 2.64 m/s average speed. This is an essential hardware/software co-design reference, not a controller-only baseline or directly comparable FlyGym/biological result.

Sources:
- https://doi.org/10.1038/s41586-026-11102-5
- https://www.natureasia.com/en/info/press-releases/detail/9447

### Impact-aware robust convex MPC

IEEE RA-L 2026 comparator; reproduce within the source paper's exact robot, model and operating conditions.

Source: https://doi.org/10.1109/LRA.2026.3692086

## 9. Proposed targets (not achieved)

- 100% agreement with independent brute-force fixtures.
- Zero false-safe probe or continuation classifications on the frozen corpus.
- Zero omitted physical branches in the declared finite fixtures.
- Adding an admissible adversarial branch cannot expand the robustly admitted probe or continuation set.
- No high-EIG or high-VOI probe bypasses post-probe viability and fallback gates.
- No unknown observation silently removes physical states or probability mass.
- Every admitted probe has a robust continuation/fallback for every physically possible observation.
- At matched physical admission and resource budgets, non-inferior held-out decision regret to the best simple baseline.
- Stretch target: at least 10% lower held-out regret or 10% fewer probes at equal regret, without worsening any hard physical constraint.
- Report viable-set/option coverage versus compute cost and Unknown frequency; do not impose an arbitrary universal action-count threshold.

## 10. Execution sequence

1. Freeze the finite state/branch/observation schema and exact fixtures.
2. Consume #4083/#4061 viability and transition outputs; do not reimplement their authority.
3. Implement a dependency-free Rust finite composition oracle for path-prefix viability, observation-conditioned successor sets and robust continuation action sets.
4. Independently brute-force all tiny fixtures and test zero-probability physical branches, latency, unknown outcomes, missing transitions and added-branch monotonicity.
5. Feed the continuation result into #4272 decision-value ranking.
6. Compare fixed-state VOI, EIG, random feasible probing, robust viability-only, STEP-CBF+-style screening and transition-aware VOI on matched FlyGym/SPINDER tasks.
7. Advance to hardware only after simulator models, receipts, sensor calibration and deadlines have independent evidence.

## 11. Claim ceiling

A finite result qualifies only the supplied finite transition/observation system. It does not prove physical abstraction soundness, continuous-time reachability, calibrated transition probabilities, robot safety, hardware superiority or biological validity. Those require measured and independently verified contact, sensor, actuator, timing, mass and terrain bounds, together with matched physical experiments.
