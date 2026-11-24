# NAV-TEMPORAL-ROBUST-VOI v0.1 — Benchmark and qualification contract

**Status:** research protocol and synthetic scenario manifest. This is not an executed benchmark, dataset admission, or physical qualification.

## Objective

Evaluate active navigation policies when both sensor observation models and capture-to-use state-transition models are uncertain. A single observable policy must work across a finite set of plausible mission models while the identity of each model and the common-cause regime remains fixed for the mission.

The core research object is not an individual sensor score. It is the observable contingent policy and its terminal decision loss at the epoch when evidence is actually used.

## Why this benchmark is needed

The current time-aware reference in PR #7284 improves on static-state planning by conditioning at capture and propagating the posterior to the use epoch. It takes each transition kernel as supplied. It does not yet bound uncertainty about the kernel.

The current persistent joint model/common-cause reference in PR #7275 models a persistent observation-model identity and shared-fault regime, but its navigation state does not evolve during evidence age.

The current mission-global minimax and minimax-regret references in PRs #7277 and #7278 correctly distinguish a model that persists for the mission from an adversary that switches models branch by branch. This benchmark composes that distinction with state evolution.

A branchwise worst-case planner may be useful as a conservative diagnostic, but it is not interchangeable with mission-persistent uncertainty. The adversary must not change the motion model after seeing an observation unless that is an explicit and separately justified threat model.

## Reference mathematical contract

Let m be a fixed mission model, c a mission-persistent common-cause regime, and x the navigation service state. For v0.1, each is binary, giving eight joint hidden states across model × common-cause × navigation state.

Each action contains:
- state-conditioned likelihoods for no-observation, observed-zero, and observed-one;
- an action-specific capture-to-use transition kernel indexed by persistent model and common-cause regime;
- a declared evidence age and resource vector;
- separate observation-model and transition-model commitments.

The observable controller sees outcomes, not m or c. It therefore selects one policy tree pi that is common to every candidate model. Model-wise risk is evaluated by holding the same m fixed across every sensing stage.

The primary objective is mission-global minimax risk:

    min_pi max_m E_m[L(pi)]

The secondary objective is mission-global minimax regret:

    min_pi max_m (E_m[L(pi]) - L*_m)

where L*_m is the model-aware exact oracle risk for that same model. Report both. Absolute risk and regret answer different questions.

The rectangular/branch-switching objective is reported only as a separate adversary diagnostic. It must never be silently substituted for the mission-persistent objective.

## v0.1 finite exact scope

- Two persistent joint models.
- Two persistent common-cause states: nominal and shared fault.
- Two navigation service states: inside and outside the declared envelope.
- Horizon two; action two is selected from the observable first outcome.
- The second action's capture epoch is the first action's use epoch.
- State evolution during each action's evidence age.
- Explicit no-observation outcome and state-dependent dropout.
- Asymmetric false-safe and false-degraded terminal losses.
- Hard latency, energy, and bandwidth constraints.
- One shared observable action policy and terminal decisions across models.
- No hidden model identity access, action reuse, implicit independence, or authority transition.
- Exact integer fixed-point arithmetic, checked operations, deterministic tie-breaking, explicit abstention.
- At most four actions in the exact reference to bound policy enumeration.

This is a deliberately small oracle, not a continuous-time POMDP and not the production navigation controller.

## Mandatory comparisons

| Comparator | Purpose | Claim boundary |
|---|---|---|
| Static-state mission-global minimax | Removes age propagation | Diagnostic baseline |
| Fixed-kernel time-aware exact policy (#7284) | Isolates benefit of temporal propagation when a kernel is supplied | Not robust to kernel uncertainty |
| Persistent Bayesian joint-model mixture (#7275) | Tests prior-weighted model adaptation | Not a worst-case guarantee |
| Mission-global minimax (#7277) | Tests one shared policy under a fixed model set | Primary robust-risk comparator |
| Mission-global minimax regret (#7278) | Measures loss relative to model-aware oracles | Distinct from absolute risk |
| Rectangular / branch-switching adversary | Quantifies conservatism from per-branch model switching | Diagnostic only |
| Time-validity gate only (#7139) | Rejects inapplicable evidence without revaluing sensing | Gate-only comparator |
| One-step greedy / mutual information / expected information gain | Conventional information-seeking baselines | Must share action/resource set |
| Periodic and random sensing | Operational simple baselines | Same duty-cycle / resource envelope |
| Constant-velocity or kinematic prediction | Delay-compensation baseline | Same target epoch and input data |
| Deep trajectory prediction | Current learned delay-compensation comparator | Reproduce exact dataset and split |
| Clairvoyant future-state oracle | Upper bound for evaluator analysis | Never a deployable policy |

Do not compare against a baseline with different timestamps, site splits, sensor availability, or resource budgets and call the difference a policy improvement.

## Benchmark scoreboards

Never collapse these dimensions into one weighted score.

### 1. Localization and tracking

Report position error P50/P95/P99, mean absolute and root-mean-square error, trajectory error accumulation, velocity/distance/orientation error, validity of trajectory, availability, and latency. Align reporting with ISO/IEC 21134:2026 where its dead-reckoning integrated localization/tracking scope applies.

### 2. Decision integrity

Report false-safe and false-degraded exposure separately, risk-coverage curves, abstention, use-epoch posterior calibration, stale-evidence overconfidence, and the number of invalid or future-dated decisions. A lower average position error is not a safety result.

### 3. Active sensing and robust policy quality

Report each model's expected terminal loss, worst-case mission loss, minimax regret, gap to the exact finite oracle, gap to the clairvoyant evaluator, acquisition yield, redundant sensing, and stop/abstention timing.

### 4. Operational cost

Report worst-case and distributional latency, energy per valid decision, bandwidth/airtime, compute/memory, unavailable time, and recovery time. Maintain a Pareto view rather than assigning arbitrary exchange rates among risk, joules, bytes, and time.

## Scenario protocol

The manifest defines fourteen synthetic challenge cases. Each case specifies its expected invariant or fail-closed disposition. The executable fixture definitions and generated traces remain a later implementation step; the manifest itself is not evidence that any score has been achieved.

Required metamorphic checks include:
- candidate action permutation invariance;
- persistent model label permutation invariance;
- adding a plausible model cannot improve the worst-case risk of a fixed policy;
- zero-age transitions are identity;
- missing transition evidence never defaults to identity;
- no-observation updates the posterior before temporal propagation;
- no future information is available to the policy;
- an unknown outcome blocks unqualified retries;
- resource ceilings hold on every observable branch;
- every action is used at most once.

## Dataset and industry protocol

### Dynamic trajectory track: MILUV

MILUV is the best initial candidate for dynamic replay because it provides 217 minutes across 36 experiments and three quadcopters, with UWB timestamps and CIR, stereo and downward vision, IMU, height, magnetometer, and motion-capture truth. Dataset payload rights are CC BY 4.0 according to the paper; the data payload and any separate tooling must be checked under their own licenses. Admission still requires a payload manifest, timestamp/frame audit, synchronization and ground-truth audit, data digest, episode/site/device grouping, and leakage checks.

### Spatial generalization track: HYMN

HYMN is useful for multi-technology spatial generalization: UWB, BLE, Wi-Fi, 5G, and GNSS in an indoor/indoor-outdoor facility. The published v1 collection has 48 fixed reference points (36 indoor, 12 transition), with stationary fixed-orientation samples and no logged measurements while moving between points. It must not be used as evidence of continuous-motion delay compensation.

### Standards and commercial interoperability

ISO/IEC 21134:2026 is the current international benchmark framework for integrated indoor localization/tracking using dead reckoning. Use its relevant position, trajectory, orientation, relative-pose, integrated-performance, practicality, and data indicators as a reporting baseline. It is not a safety or integrity certificate.

FiRa Core 4.1 adds UWB subsystem features including DL-TDoA support for untracked navigation and extended DS-TWR security testing. FiRa core certification evaluates the UWB subsystem's PHY, MAC, and Link layers; it does not qualify the full navigation estimator, decision policy, or autonomous safety authority.

## Proposed performance gates (targets, not achieved results)

1. Exact tiny-model oracle: 100% agreement with an independent exhaustive evaluator.
2. Software negative controls: 100% rejection of seeded malformed model, chronology, identity, and resource violations.
3. Medium synthetic policy suite: target ≤1% relative regret to an exact oracle where the oracle is tractable; report tail and worst-case regret separately.
4. Dynamic data reproduction: first reproduce the published delay-compensation baseline on its exact data, target epoch, and split before claiming superiority.
5. Stretch tracking target after reproduction: reduce mean error by at least 10% and raise the within-0.5-m success rate above the published 85% comparator, without worsening P95/P99, valid coverage, or false-safe exposure. This is a research target, not a result.
6. Decision-quality target: lower worst-case use-epoch loss or improve the risk/coverage Pareto frontier versus static, greedy, and fixed-kernel policies under the same model set and resources.
7. Operational target: at equal or lower worst-case use-epoch risk, reduce energy or airtime by at least 20% against periodic acquisition. Keep this distinct from accuracy.
8. Dataset generalization: report episode-, device-, site-, and motion-regime holdouts; random packet splits are not sufficient.

A benchmark win requires confidence intervals or paired episode-level uncertainty, complete split disclosure, frozen inputs, and independently generated ground truth. A result that improves P50 while worsening P99 or false-safe exposure is not a win.

## Execution sequence

1. Qualify #7284 at its exact corrected head.
2. Implement the joint persistent-model/common-cause/time-evolution finite oracle for this issue.
3. Add independent exhaustive cross-checks for tiny action sets and model permutations.
4. Instantiate the fourteen fixtures and emit machine-readable result vectors.
5. Admit MILUV only after its payload and time/frame gates pass; keep HYMN on the stationary spatial-transfer track.
6. Reproduce the published dynamic baseline under matched conditions.
7. Run controlled physical campaigns only after independent reference instrumentation and safety review exist.

## References

- Modaberi & Far (2026), Real-time delay-compensated UWB localization for dynamic agents via deep trajectory prediction: https://doi.org/10.1016/j.eswa.2026.132248
- HYMN dataset descriptor and methodology: https://arxiv.org/abs/2604.20349
- HYMN methods comparison (IPIN 2026): https://arxiv.org/abs/2609.25835
- MILUV dataset paper: https://doi.org/10.1177/02783649251405898
- UWB CIR/NLOS survey (November 2026): https://doi.org/10.1016/j.compeleceng.2026.111418
- ISO/IEC 21134:2026: https://www.iso.org/standard/86651.html
- FiRa Core 4.1 announcement: https://www.firaconsortium.org/news/press-releases/2026/10/fira-consortium-launches-core-4-1-specifications-and-enhanced
- FiRa core certification scope: https://www.firaconsortium.org/certifications/certification-program

## Claim ceiling

No physical motion model is calibrated here. No dataset payload has been admitted by this benchmark package. No result establishes empirical navigation accuracy, integrity risk, PHMI, safety certification, or autonomous authority.