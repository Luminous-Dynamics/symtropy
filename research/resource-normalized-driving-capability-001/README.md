# RESOURCE-NORMALIZED-DRIVING-CAPABILITY-001

**Status:** protocol and comparator register only; no model benchmark run or performance result  
**Issue:** https://github.com/Luminous-Dynamics/symtropy/issues/4639  
**Related environment:** #4536, #4595, #4616  
**Related learner:** Symthaea #11414, #11476, draft PR #11480  
**Related active learning:** #4550, #4552, #4556, #4575, #4581

## The paradigm shift: compare capability frontiers, not model names

Autonomous-driving comparisons are misleading when they ignore input privileges, model access, data, evaluation variant, inference hardware, training compute, teacher-generated labels and system overhead. This protocol requires two simultaneous views:

1. **Matched capability:** same benchmark implementation, evaluation split, input permissions, data exposure, action space, scenario transitions and inference rig wherever feasible.
2. **Capability frontier:** native task score against measured inference energy, p99 latency, memory, training transitions, GPU-hours, energy and human correction effort.

The aim is not to prove that an HDC model is smaller than a transformer. The aim is to find the best measured competence/constraint frontier and determine which Symthaea components actually contribute. Bench2Drive v0.0.4 is the maintainer-designated current benchmark release; its LEAD result is 95.59 DS / 87.28 SR. The separate community leaderboard snapshot reports methods on v0.0.3, including BridgeDrive (LEAD) at 96.34 / 89.25 and privileged PDM-Lite at 97.02 / 92.27. Do not compare v0.0.3 and v0.0.4 scores directly: v0.0.4 changes the scenario distribution and adds 3D occupancy. The public v0.0.3 leaderboard also warns that methods vary in data acquisition, expert use, compute and privileges.

## What the field is doing (snapshot: 13 November 2026)

| Comparator | Public reference | How to use it |
|---|---|---|
| Bench2Drive v0.0.4 current release | LEAD: 95.59 DS / 87.28 SR; efficiency 201.55; comfort 20.56 | Maintainer README marks v0.0.4 as current; uses a uniform 44-scenario × 25-route training set, 220-route validation set and adds 3D occupancy |
| Bench2Drive v0.0.3 self-reported leaderboard | BridgeDrive (LEAD): 96.34 DS / 89.25 SR; TFv6: 95.28 / 86.81 | Public leaderboard says methods differ in data, experts, compute and privileges; these are not matched head-to-head runs |
| Bench2Drive v0.0.3 non-privileged snapshot leader | TFv6+ScaleDrive: 96.65 DS / 90.45 SR (public table last updated 6 Nov 2026) | Self-reported result; table states methods vary in datasets, experts, training/inference compute and IL/RL. Treat as a reference ceiling, not a controlled model-only comparison |
| Bench2Drive v0.0.3 privileged LEAD | LEAD: 96.78 DS / 96.59 SR (below privileged-method divider) | Ground-truth perception input. Keep separate from the non-privileged visual-input target; never describe this as fair vision-only SOTA |
| Bench2Drive v0.0.3 privileged section | PDM-Lite: 97.02 DS / 92.27 SR | Ground-truth/privileged perception; retain as a separate upper-bound comparator, not a fair vision-only target |
| DriveWorld-VLA, ICML 2026 | NAVSIMv1 PDMS 91.3; NAVSIMv2 EPDMS 86.8; nuScenes 3-second average collision rate 0.16 | Action-conditioned latent world-model/VLA reference; preserve each native task metric |
| NAVSIM v2 leaderboard snapshot (SOTA2, page marked updated 5mo ago) | TPS-Drive 2026.05: NC 98.5 / EPDMS 86.7; DriveVLA-W0 2026.05: NC 98.5 / EPDMS 86.1 | Third-party snapshot. Retain EPDMS and NC separately; do not mix with paper-reported NAVSIM numbers or infer this page is a current official organizer ranking |
| World-In-World, ICLR 2026 | Closed-loop platform emphasizing task success, controllability, action-observation post-training and inference-time compute | Evaluate whether predicted worlds improve agent success, not just video fidelity |
| NVIDIA Alpamayo 2 Edge, released 10 Nov 2026 | Model card: 2.45B backbone + 0.2B action expert; 15.15 ± 0.04 km per at-fault incident on 875 AlpaSim scenarios / three rollout sets; minADE6 0.52m at 5s on 1,392 samples; Lingo-Judge 75.2; 115,000h video and about 3.7M CoC traces | Strong edge-VLA comparator. Keep closed-loop, open-loop, reasoning, training data and latency as distinct metric families |
| Alpamayo 2 Edge deployment recipe | NVIDIA's named DRIVE AGX Thor-X recipe reports 160–190ms pixel-to-trajectory latency on DRIVE OS 7.2.7.0, CUDA 13.4 and TensorRT 11.3.1.9 | Latency comparator only on a matching hardware/software measurement configuration |
| TIER IV AutoE2E (announced 11 Nov 2026) | 79.9M parameters; TIER IV/Autoware report 0.744m three-second ADE on KITScenes-Multimodal validation; reactive component released, world/reasoning components developing | Compact Fast–Slow architecture reference. The ADE is an open-loop trajectory metric on that dataset, not a closed-loop collision/safety score; keep it separate from NAVSIM and Bench2Drive metrics | 
| TIER IV METEOR (announced 11 Nov 2026) | 16M parameters; mapless CNN/BEV; 1–8 cameras with optional LiDAR/radar; NVIDIA and Renesas platform support described | Parameter-count and hardware-portability reference only; the announcement provides no comparable closed-loop score or latency/energy measurement |
| Alpamayo 2 Super | 34B reasoning/teacher model with trajectory, CoC reasoning and auto-label outputs | High-capacity teacher; account for teacher privilege and generated supervision |
| Waymo WOD-E2E, CVPR 2026 | 4,021 segments / approximately 12h curated for long-tail E2E scenarios with reported occurrence below 0.03% | Rare-event dataset reference; offline dataset, not a closed-loop recovery metric |
| Waymo Open Dataset challenges | Formal challenges are not hosted in 2026, but public leaderboards remain active | Public evaluator/data route; pin the exact active leaderboard and track |
| CARLA Leaderboard 2.0 | Separate SENSORS and MAP tracks with route completion and infraction penalties; official challenge is permanently closed | Historical methodology reference only, not an active submission path |
| CARLA Leaderboard 2.1 | Current protocol defines privileged-free policy evaluation, route completion and infraction criteria; official submission service and evaluation servers are closed | Latest protocol is a local evaluator reference, not a live competition/submission target |
| Waymo Driver | Company dashboard: 271.3M rider-only miles through June 2026; company-reported 95% fewer serious-injury-or-worse crashes and 82% fewer injury-causing crashes against its location-adjusted human benchmark | Operational safety evidence class and eventual safety-case reference, not a simulator leaderboard |
| NHTSA Part 573 Recall 26E035 (17 Jun 2026) | Official report describes freeway closure-sign nonrecognition, freeway construction-zone entry and cone-channelization events involving 5th-generation ADS | Failure-mode evidence for temporary traffic-control scenarios; not a rate, controlled benchmark or leaderboard result |

### Why AutoE2E changes our architecture plan

AutoE2E is a useful small-model comparison, and the announced numbers need careful interpretation: TIER IV/Autoware report 79.9M parameters and 0.744m average displacement error over a three-second horizon on KITScenes-Multimodal validation. That ADE is not a closed-loop driving score. The reactive model is the released component, while world-model and reasoning-model components are still in development. METEOR is an even smaller 16M-parameter mapless CNN/BEV model with one-to-eight camera support and optional LiDAR/radar, but the announcement does not provide a matched task score. Sources: [Autoware AutoE2E announcement](https://autoware.org/autoware-foundation-and-tier-iv-join-forces-to-releases-autoe2e-an-open-source-ai-model-for-end-to-end-autonomy/) and [TIER IV's model-family release](https://tier4.co.jp/en/updates/press-release/20261111-tieriv-unveils-e2e-autonomous-driving).

This motivates an explicit Symthaea **fast/slow cognitive budget** ablation:
- Fast path: low-latency HDC/LTC/CfC state update, action candidates and independently enforced actuation deadline.
- Slow path: invoke action-conditioned imagination, additional camera/sensor acquisition, simulation branches or a human query only when those can plausibly change the selected action or reduce a predeclared decision loss.
- Reserve actuation time before spending any inference budget. If no query can finish with sufficient margin, choose a qualified fallback rather than wait.
- Measure slow-path activation rate, incremental latency/energy, value of information, action-change rate, regret reduction, deadline skips and fallback preservation separately.

The slow path must not self-certify the fast path or displace SAM. Candidate query selection is governed by #4550 and the decision-critical VOI work; runtime allocation evidence is already represented in the `inference_allocation` result-card section.

### Industry safety evidence needs both exposure metrics and failure-mode counterexamples

Waymo's company-reported safety dashboard and NHTSA Recall 26E035 answer different questions. The former is an aggregate, location-adjusted operational comparison through June 2026. The latter is a regulatory event chronology for a specific freeway-construction-zone failure mode: the report describes ramp-closure sign nonrecognition and entries into pre-planned work zones in Phoenix, plus vehicles entering between cones marking a lane closure in the San Francisco Bay Area. The filing describes 3,871 potentially involved 5th-generation ADS vehicles and a software/operational remedy. It does **not** provide a public-road failure-rate denominator for those individual events. <Link url="https://static.nhtsa.gov/odi/rcl/2026/RCLRPT-26E035-7637.pdf" title="NHTSA Recall 26E035"/>.

The benchmark implication is important: aggregate safety performance does not eliminate the need for scenario-specific ODD-boundary tests. Conversely, a recall chronology is not a fair basis for ranking Waymo against a simulator policy. We use it only to motivate independently reproducible temporary-traffic-control scenarios and negative controls.

These results are not pooled into one ranking. Bench2Drive Driving Score, NAVSIM PDMS/EPDMS, AlpaSim kilometres per at-fault incident, minADE, reasoning scores, WOD-E2E dataset coverage and Waymo operational crash rates answer different questions. The Alpamayo model card's 2.45B + 0.2B breakdown differs from NVIDIA's rounded "2B" headline; the manifest preserves the detailed model-card count and labels the source.

Sources are linked in `benchmark_manifest_v0_1.json`. Model-card figures and vendor deployment measurements are reported claims, not independent Symtropy reproductions. NVIDIA's 160–190ms latency is meaningful only with its named Thor-X / DRIVE OS / CUDA / TensorRT configuration; it is not compared to laptop or desktop CPU timing.

## New benchmark tracks: deployment robustness and speed-conditioned control

The Luminous degradation/recovery work is tracked separately in [Symtropy #4657](https://github.com/Luminous-Dynamics/symtropy/issues/4657) and consumes the same normalized result-card principles defined here.

### Bench2Drive-Robust

The standard closed-loop score is not enough for a deployable system. Bench2Drive-Robust is a model-agnostic extension that holds a policy fixed and injects deployment perturbations into the closed-loop feedback loop. Its published settings include:

| Fault family | Published severities | What to record |
|---|---|---|
| Camera stream | 20- or 60-tick burst frame drops | stale-frame exposure, missing-event accounting, recovery time, success |
| Partial observation | 50% or 80% camera occlusion | degradation by mask severity, calibration, fallback behaviour |
| Localization | 5 m or 15 m GPS noise | route and interaction degradation, estimator residuals |
| Speed estimate | multiplicative noise with means 0.5 or 0.2 and standard deviation 0.2 | speed-adherence and control error |
| Compute/control | 100 ms, 200 ms or 500 ms action delay | p95/p99 control age, missed deadlines, fallback and task success |

Source: [Bench2Drive-Robust repository](https://github.com/Thinklab-SJTU/Bench2Drive-Robust) and [paper](https://arxiv.org/abs/2605.18059). These are benchmark perturbation settings, not universal real-world fault distributions.

Our extension requires **paired clean and perturbed episodes**: same checkpoint, initial scenario, route, actor policies, seed manifest and data permissions; only the explicitly declared fault injection changes. Report native task metrics and relative degradation per fault family/severity, plus recovery time, control age, fallback outcomes and worst-family tails. Never combine different fault families into one robustness number without predeclared weights.

### Luminous Degradation-and-Recovery Envelope

The new [#4657 protocol](https://github.com/Luminous-Dynamics/symtropy/issues/4657) adds an event-centric view on top of deployment robustness. Bench2Drive-Robust provides important system perturbations; our extension measures the *full temporal response* to a fault:

`fault onset -> event-specific detection -> containment -> degraded operation -> fault clearance -> healthy-observation quorum -> recovery`

The standalone auditor currently covers camera drop, frame delay, stale replay, pixel corruption, calibration shift, clock drift and compute overrun. It preserves event IDs so one fault cannot claim another fault's detection or containment. It also represents independent, cascaded and declared common-cause events, and rejects premature normal-mode restoration, undeclared fault conditions, accepted stale/corrupt inputs, incomplete tick denominators and unmeasured per-tick latency/energy evidence.

The severity-sweep generator schedules all seven families at five declared control levels (10%, 25%, 50%, 75%, 100%), producing 35 independent event windows. Those values are normalized experiment controls, not a claim that severity has a physically equivalent meaning across fault families. The generator does not inject faults or execute a simulator.

**What to publish per fault:** detection latency, containment latency, stale/corrupt acceptance, degraded-mode duration, safe-stop/fallback result, recovery quorum and latency, false-positive degradation on clean controls, p95/p99 control latency, energy and worst-family tail. Keep those axes separate. A model that never resumes normal operation is not a recovery success; one that resumes too early is also a failure.

The first v0.1 fixture is explicitly synthetic and uses an untrained input bridge, not a trained driver. Its latency and energy values exercise validation logic only and are not hardware measurements. See [the auditor README and fixture](https://github.com/Luminous-Dynamics/symtropy/tree/main/research/degradation-recovery-envelope-001).

### Bench2Drive-Speed

[Bench2Drive-Speed](https://github.com/Thinklab-SJTU/Bench2Drive-Speed) adds desired-speed-conditioned driving, follow/overtake intent and speed adherence over a 2,100-scenario CARLA suite. It is a separate capability track: report speed command adherence, tracking error, comfort/jerk, overtake/follow success and route success independently. It must not be pooled into the ordinary Bench2Drive Driving Score.

### NAVSIM v2 leaderboard snapshot

A third-party SOTA2 page currently lists NTR 2026.05 at 99.4 NC / 90.9 EPDMS for the NAVSIM v2 end-to-end test track. The page was marked updated five months before this 11 November 2026 snapshot. Treat it as a **third-party, stale-at-snapshot leaderboard reference**, not an official organizer result and not a reproducible target until the benchmark owner, evaluator revision, test access and current ranking are independently re-verified. It is deliberately separate from the DriveWorld-VLA paper's NAVSIMv2 EPDMS 86.8 result; different entries and provenance, same broad metric label, not an apples-to-apples comparison without exact protocol alignment.

Source: [SOTA2 NAVSIM v2 E2E test snapshot](https://www.sota2.com/research/sota/end-to-end-driving-on-navsim-v2-test).

## Symthaea's competitive hypothesis

Test a hybrid rather than assume HDC/LTC replaces visual representation learning:

`learned visual features -> structured HDC scene binding -> continuous-time HDC-LTC/CfC memory -> action-conditioned latent dynamics -> policy`

SAM remains an independent evidence/recoverability interface, not a policy head or duplicate evaluator.

The current #11480 bridge is only the deterministic visual/proprioceptive representation and receipt boundary. It uses the existing Vision Manifold and vehicle encoder; it does not establish a trained visual driving policy or semantic perception quality.

## Required model arms

- **A — legacy state policy:** existing 20-channel vehicle-state HDC encoder + HDC-LTC controller + PD-teacher training.
- **B — visual representation:** Vision Manifold + vehicle HDC fusion. This is a representation baseline until a compatible policy is trained.
- **C — compact recurrent:** same learned visual front-end + GRU/LSTM.
- **D — compact transformer:** same learned visual front-end + compact transformer temporal policy.
- **E — hybrid:** learned visual front-end + structured HDC scene binding + HDC-LTC/CfC temporal core.
- **F — hybrid world model:** E + action-conditioned latent future/state prediction.
- **G — decision-sensitive learning:** F + counterfactual-value experience selection and SAM-qualified evidence, using existing #4550/SAM owners.
- **H — industry teacher/student reference:** Alpamayo 2 Edge/Super, only where weights, data, hardware, licences and evaluator access are actually available.
- **I — TIER IV AutoE2E:** compare the released 79.9M-parameter reactive component; keep its developing World Model/Reasoning components out of the measured system until they are available and pinned.
- **J — TIER IV METEOR:** 16M-parameter mapless CNN/BEV baseline; pin the camera count, optional LiDAR/radar inputs, preprocessing, platform and complete energy/latency measurement.
- **K — risk-constrained observability:** paired passive observation versus active sensor request/safe reposition at the same frozen decision boundary; measure proper scores and information/action costs separately.
- **L — selective risk-coverage:** evaluate the matched policy's frozen predictions with abstention and separately evidenced fallback; report risk–coverage curves, confidence intervals and actual autonomy coverage.
- **Human reference:** demonstrations/corrections; not treated as an autonomous policy.

The model arms and the decision-layer arms answer different questions. K and L are policy overlays, not new perception backbones. Run factorial ablations so any improvement can be attributed to the visual encoder, temporal core, action-conditioned model, active-observation policy, abstention gate or SAM evidence interface.

All arms must expose the same policy-visible observation and action interface for matched comparisons. For external compact baselines, first reproduce each model's native sensor configuration and native evaluator, then run a separate matched-sensor comparison where possible. An input-profile mismatch (camera count, LiDAR/radar, BEV preprocessing, navigation/map inputs or history window) must be visible; never silently strip or enrich inputs to manufacture parity.

## Orthogonal learning and inference allocation experiments

Do not treat the sampler or inference allocator as part of the model architecture. They are independent experimental factors.

### Training experience selection

Use [#4550](https://github.com/Luminous-Dynamics/symtropy/issues/4550)'s preregistered arms and [#4575](https://github.com/Luminous-Dynamics/symtropy/pull/4575)'s deterministic selector:
- cyclic/uniform;
- scenario rarity;
- entropy/disagreement;
- temporal prediction error;
- action-decision margin;
- SAM expected decision-loss reduction per acquisition cost;
- SAM recoverability / fragility-directed counterexamples.

The selector's evidence inputs, cost and query deadline must be measured. A high-entropy scenario is not automatically valuable; the outcome must show that the selected experience changes a decision or improves held-out competence.

### Inference-time imagination allocation

Use [#4581](https://github.com/Luminous-Dynamics/symtropy/pull/4581)'s anytime allocator as an independent factor:
- fixed rollout count;
- fixed compute budget;
- deadline-aware marginal expected decision-regret reduction;
- no additional rollouts when marginal value is non-positive;
- no allocation that consumes the actuation or recovery reserve.

The allocator is a deterministic greedy priority heuristic, not a globally optimal knapsack solver. Its marginal regret and latency inputs must be checkpoint-, context-, hardware- and observation-bound, and their calibration must be measured.

### Factorial ablation design

At minimum, cross these factors on the same scenario and evaluation splits:

| Factor | Levels |
|---|---|
| Policy architecture | matched GRU/LSTM, compact transformer, HDC-LTC/CfC hybrid |
| Experience sampler | uniform, entropy, prediction-error, decision-margin, SAM decision-value, SAM recoverability |
| Inference allocation | fixed rollouts, compute-matched anytime, latency-matched anytime |
| World model | absent, action-conditioned |
| Assurance | ordinary fixed fallback baseline, independently SAM-qualified recoverability |

This is a research matrix, not an instruction to run every cross-product immediately. Start with one-factor ablations, then run the smallest predeclared factorial block that can distinguish architecture, sampler and allocator effects without changing data, hardware or test access.

Report two resource views for every comparison:
1. **Equal compute:** same total training experiences, human labels, rollout compute and total measured energy.
2. **Equal deadline:** same hardware, control deadline, p99 latency ceiling and reserved actuation/recovery time.

Never let the anytime allocator receive extra compute in the equal-compute comparison. Never let the fixed baseline be denied inference time or sensor history available to the allocator. Record all extra world-model calls, cached predictions, teacher labels and selected-candidate evaluation costs.

### Required interaction tests

- Does decision-sensitive sampling reduce transitions to a frozen competence threshold with no worst-family regression?
- Does the HDC-LTC/CfC temporal core benefit more from irregular timestamps and partial observations than matched GRU/LSTM and transformer baselines?
- Does action-conditioned imagination improve action ranking and closed-loop outcomes over an equally trained model without imagination?
- Does anytime allocation improve task quality under equal compute, or reduce compute while holding quality non-inferior under equal deadline?
- Does SAM recoverability ranking reduce fragile decisions without collapsing into always-stop behaviour?
- Do the benefits persist under Bench2Drive-Robust and the event-attributed #4657 degradation/recovery protocol?

Primary outcomes remain separate: native task score, transitions/human minutes per skill gate, rollout cost, marginal regret reduction, p99 latency, energy, recovery and worst-family performance. Do not fit post-hoc weights to claim a combined win.

## Jurisdiction and local-rule transfer

The new [#4774 jurisdictional transfer track](https://github.com/Luminous-Dynamics/symtropy/issues/4774) adds an explicit domain shift: left-hand vs right-hand traffic, regional sign/marking semantics and locally sourced rules.

Three evidence classes are kept separate:
- **TrafficSignBench:** closed-loop sign-compliant destination (SCD) task/metric. Its SCD result must not be collapsed into ordinary route Driving Score.
- **GeoDrive-Bench:** open-loop regional driving-reasoning QA. Its six listed countries do not include South Africa; QA accuracy is not closed-loop driving competence.
- **ZA source profile:** a draft source-qualified record that declares left-hand traffic from official South African tourism guidance and separately records the Department of Transport sign manual and National Road Traffic Act source references. All sources are currently marked not reviewed; exact applicable revisions, amendments, commencement/effective dates and content digests remain unresolved. This profile is explicitly not policy eligible.

The first experiment is zero-shot LHT/RHT transfer plus fixed-budget adaptation. It separates traffic geometry, legal semantics and interaction ecology. A coordinate-mirror fixture validates geometry and task-identity plumbing only; it does not transform or certify local laws, signs, right-of-way rules or actor behaviour. Steering-wheel handedness and roundabout circulation are independent profile fields, not inferences from traffic side.

The v0.1 source/profile validator and fixtures are under [`research/jurisdictional-driving-transfer-001/`](https://github.com/Luminous-Dynamics/symtropy/tree/main/research/jurisdictional-driving-transfer-001). A profile can be structurally valid as a draft without being policy eligible. Policy eligibility requires legally reviewed statute and traffic-sign-manual source records, explicit effective dates and digests plus a versioned rule-bundle digest. The validator checks declared structure; it does not fetch/authenticate sources or decide legal interpretation.

## Compact-model competition and the new architecture split

The 11 November 2026 TIER IV releases change the baseline strategy. AutoE2E contributes a 79.9M-parameter Fast–Slow reference and reports 0.744m 3-second ADE on KITScenes-Multimodal validation. METEOR contributes a 16M-parameter mapless CNN/BEV reference with camera/LiDAR/radar configurations. The AutoE2E ADE is an open-loop trajectory-prediction result, not a closed-loop driving score; METEOR's announcement does not provide a comparable task score.

This suggests a four-layer Symthaea experiment:

1. **Fast reactive path:** a compact camera/BEV encoder and low-latency policy proposal. HDC scene binding can retain identities and relations; the policy must still be trained and evaluated.
2. **Persistent temporal state:** HDC-LTC/CfC state update on each accepted observation, with explicit clock and missingness semantics.
3. **Conditional slow path:** action-conditioned imagination, active sensor acquisition and/or additional reasoning only when expected decision-loss reduction can justify the added latency, energy and risk.
4. **Independent assurance and abstention:** SAM checks evidence/recoverability; a selective gate can yield/abstain and use separately measured fallback. The slow path cannot certify the fast path.

The key comparison is not just model size. Measure the Pareto frontier across native driving capability, risk at fixed coverage, actual autonomy coverage, fallback success, p99 deadline compliance, inference joules, peak memory, simulator transitions and human interventions. A fast model that abstains constantly may have low accepted-action risk but poor useful coverage; an always-on reasoning model may improve decisions while missing actuation deadlines. Neither should win by collapsing the tradeoff into one composite.

### New paired decision-layer tracks

- **Observability / value of information (#4983):** use paired passive and active branches from the same decision checkpoint. Keep the hidden actor/outcome labels evaluator-only, preserve common exogenous randomness where valid, and price sensor requests/reposition by duration, energy, maneuver distance and residual risk. Evaluate Brier score, log loss, calibration, entropy reduction, decision changes and safety outcomes independently.
- **Selective risk–coverage (#4942):** evaluate frozen failure probabilities against observed outcomes, with equal-score tie groups, episode-cluster bootstrap intervals, risk at fixed coverage, coverage at fixed risk, abstention timing, fallback success/duration and human interventions. A missing or censored outcome must not be imputed as success.
- **Composition test:** run K and L over the same frozen backbone/checkpoint, then factorially add the action-conditioned world model and SAM. This isolates whether active sensing, abstention or imagination contributes anything beyond the backbone.

## What to measure

### Capability
Native benchmark scores, route completion, traffic-rule compliance, task progress, interaction quality, contact/collision outcomes under the simulator's definitions, comfort/jerk and worst-family performance.

### Predictive competence
Action-conditioned next-state error, calibration/coverage, actor response prediction, action-ranking regret, causal counterfactual sensitivity and controllability under interventions. Open-loop image fidelity is a diagnostic metric only.

### Efficiency
Trainable parameters; training GPU-hours; measured kWh; simulator transitions; frame exposure; human labels and intervention minutes; inference FLOPs/tokens when available; p50/p95/p99 inference and recovery latency; deadline misses; peak memory; energy per frame and energy per successful episode.

### Robustness and assurance
Camera dropout (including 20/60-tick burst faults), partial observation (50%/80% occlusion), GPS noise (5m/15m), multiplicative speed-estimation noise and inference/control delay (100/200/500ms) under paired clean/perturbed closed-loop episodes; also calibration error, lighting/weather, sensor asynchrony, actor-policy change, compute overload, safe fallback, option preservation, recoverability, replay integrity and evidence completeness. These published perturbation levels form a reproducible test matrix, not a claim about real-world fault prevalence.

## Fair-comparison controls

- Use the same benchmark tag, evaluator, routes, sensor definitions, action space, holdout split and evaluation seed manifest.
- Match training data, simulator transitions, human interventions and update opportunities. Report unmatched budgets.
- Report both matched hardware and frontier measurements; do not compare laptop timings with server timings as equivalent evidence.
- Count teacher-generated labels, pretraining datasets, synthetic data and privileged simulator truth as resources/privileges.
- Include full system cost: perception, memory, world model, planning, safety monitoring and host overhead—not just the policy head.
- Freeze checkpoint, recurrent state, normalization, calibration, query policy and privilege settings for final evaluation.
- Split by map, scenario family, actor policy, weather/lighting, road convention, vehicle profile and fault-generation family.
- Preserve failed, incomplete, censored and timed-out runs in the expected denominator.
- Publish per-family tails and confidence intervals across independent seeds.
- Do not optimize on the sealed test split.

## Prospective targets (not measured results)

1. 100% exact agreement on analytic control/rendering fixtures and declared negative controls.
2. 100% expected/observed episode accounting and zero hidden-state or teacher/evaluator leakage in frozen evaluation.
3. At matched Bench2Drive v0.0.4 conditions, reach within 1.5 native Driving Score points of the strongest reproducible compact baseline while using no more than 50% of its measured inference energy. Hardware, input privileges, dataset, precision, power profile and energy measurement rig must be frozen before evaluation.
4. Against uniform sampling, reduce simulator transitions or human correction minutes to a frozen skill threshold by at least 20%, without degrading worst-family constraints, calibration, fallback integrity or held-out competence.
5. Demonstrate at least one statistically supported HDC/LTC advantage in temporal irregularity, memory, missing-sensor robustness, adaptation cost, latency/energy or recoverability.
6. Show that action-conditioned world-model training improves closed-loop outcomes versus the same policy without that model, under matched data and compute.

These thresholds are prospective research targets only. They are not safety requirements or existing results.

## Paired deployment-fault evidence

The paired fault campaign is now tracked separately in [#4681](https://github.com/Luminous-Dynamics/symtropy/issues/4681). It owns the typed clean/perturbed pairing contract, fault-event ledger and per-metric degradation evaluator. This benchmark protocol consumes its result format instead of duplicating pair validation.

The critical comparison is not simply “score under a camera drop.” It is the delta against a **same-checkpoint, same-scenario, same-seed, same-actor, same-hardware clean run**, with only one declared fault changed. Each fault family and severity remains a separate row. A missing camera frame is an event, not a frozen pixel pretending to be a new observation. The report must preserve recovery, control age, deadline misses and fallback outcomes in addition to native route metrics.

## Benchmark ladder

1. Analytic vehicle/control and camera projection fixtures.
2. Deterministic 20+ tick offscreen observations linked to applied actions and immutable world snapshots.
3. Fresh-process replay including pixels, bridge-input digests and branch/action receipts.
4. Internal held-out virtual apprenticeship with all model arms.
5. Bench2Drive v0.0.4 closed loop.
6. Bench2Drive-Robust perturbation suite.
7. Bench2Drive-Speed desired-speed/follow/overtake-conditioned control, with native speed metrics kept separate.
8. NAVSIM v2, keeping each task/leaderboard track separate and independently verifying current evaluator access.
9. World-In-World action-conditioned closed-loop world-model evaluation.
10. Staged HIL and closed-course validation under independent safety authority.
11. Operational mileage and incident-rate evidence only after a separately governed safety case.

## Temporary traffic controls: a separate operating-condition shift

Construction zones and temporary traffic controls should not be treated as ordinary visual occlusion. They change the *rules and geometry of the road while the vehicle is in motion*. A system can have intact sensors and still fail because it gives generic obstacle avoidance precedence over a temporary closure rule, fails to bind the current closure state to the route, or enters a region that its prior plan assumed was open.

The new `temporary_traffic_control` benchmark stage is motivated by NHTSA 26E035, but is not a reproduction of that incident or an estimate of its prevalence. Build independent synthetic scenarios for:

- ramp-closure signs and mandatory exits;
- cone/barrier channelization and lane shifts;
- conflicting obstacle-avoidance and closure constraints;
- a temporary rule that activates or expires while the vehicle approaches;
- closed-lane entry, safe rerouting and a blocked-route fallback;
- false-positive work-zone interpretation on a clean control.

Keep native outcomes distinct: closure-sign compliance, cone/barrier channelization compliance, work-zone boundary entries, reroute completion, false-positive closure responses, detection latency and worst-family performance. Include the exact jurisdiction/profile revision and effective-time window, and make the policy's source-qualified rule bundle available only through the declared operational interface. Evaluator-only event truth must remain hidden.

This is complementary to:
- [#4774 jurisdictional driving transfer](https://github.com/Luminous-Dynamics/symtropy/issues/4774), which owns sourced jurisdiction/rule profiles and left/right traffic transfer;
- [#4657 degradation and recovery](https://github.com/Luminous-Dynamics/symtropy/issues/4657), which owns fault-conditioned recovery envelopes;
- [#4794 camera boundary fault injection](https://github.com/Luminous-Dynamics/symtropy/issues/4794), which tests the camera bridge's integrity boundary.

A rule profile that says a lane is open must not override current policy-visible closure signs and cones. A temporary closure that cannot be reconciled against a source-qualified rule bundle must be treated as unresolved—not silently guessed.

## Files

- `benchmark_manifest_v0_1.json`: 17 comparator records, nine model/learning arms, 16 benchmark stages, metrics, resource dimensions, negative controls and prospective targets.
- `result_card.schema.json`: schema for per-run normalized evidence records.
- `result_card.template.json`: an explicit not-run template; null values are not measurements.

## Executable result-card evaluator

The standalone Rust evaluator is in `evaluator/`. It is pinned to Rust 1.96.0 and only uses Serde/Serde JSON for typed result-card parsing. The strict per-run result-card contract is version 0.2.0.

From the evaluator directory:

```sh
cargo fmt -- --check
cargo test --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
```

Evaluate one baseline and one or more candidate cards on a **single native metric**:

```sh
cargo run --release --locked --bin capability-frontier -- driving_score maximize baseline.json candidate.json efficient.json
```

The tool:
- validates resource fields, latency percentile order, SHA-256 syntax, unique per-episode IDs and exact aggregate/per-episode disposition accounting;
- refuses to compare different benchmark IDs, protocol/evaluator revisions, holdout splits, seed manifests or policy/teacher input privileges;
- reports `matched_capability` only when training data/exposure, human interventions, teacher labels, training GPU-hours, training kWh, inference hardware, precision and measurement method are aligned and reported;
- reports `capability_frontier_only` when evaluation is aligned but training/compute budgets differ or are unreported;
- computes a metric-specific Pareto frontier over the native task metric, trainable parameters, training GPU-hours, training kWh, inference joules/frame, p99 latency and peak memory;
- excludes cards missing any required frontier measurement rather than treating missing values as zero or as a benefit;
- computes a deterministic 10,000-replicate, 95% paired bootstrap interval automatically for baseline/candidate comparisons.

### Result card v0.2.0: experience selection and inference allocation

The strict result-card schema is now **0.2.0**. In addition to architecture, data, compute, native metrics and paired episode records, each run must carry two orthogonal execution records.

**Experience selection record**
- strategy ID: `uniform`, `rarity`, `entropy`, `prediction_error`, `decision_margin`, `sam_decision_value`, or `sam_recoverability`;
- selector revision, candidate-pool digest and selection-receipt manifest digest;
- candidates scored, queries issued/accepted/rejected and query deadline misses;
- selector-attributed simulator transitions, human intervention minutes and GPU-hours.

Selector-attributed costs are a *breakdown of the total training budget*, not extra cost to add a second time. The evaluator verifies that selector transition, intervention and GPU-hour attribution do not exceed the total values in the card. It also verifies `queries_issued = queries_accepted + queries_rejected`, that candidates scored cover the queries issued, and that deadline misses are a subset of rejected queries.

**Inference allocation record**
- allocator identity/revision: `fixed_rollouts` or `deadline_aware_anytime`;
- allocation-receipt and latency-bound manifest digests;
- decision-opportunity count, additional rollouts, declared compute-unit budget and actual compute units used;
- required actuation reserve, reserve violations and deadline-skipped rollout count;
- predicted regret reduction and observed regret reduction in separate fields.

The predicted value is the allocator's supplied estimate; it is not a measured policy improvement. Observed regret reduction is evaluator-derived and may be negative. The evaluator rejects actual rollout units exceeding the declared budget but **does not erase** runs that report an actuation-reserve violation; the violation remains visible as a negative result.

A matched-compute comparison requires the same declared rollout compute budget and actuation-reserve requirement. Actual rollout units used remain an outcome/resource axis. Strategy and allocator IDs are intentionally allowed to differ between baseline and candidate arms.

The metric-specific Pareto frontier now includes native task score, trainable parameters, training GPU-hours/kWh, simulator transitions, human intervention minutes, inference joules/frame, actual rollout compute units, p99 latency and peak memory. The frontier still creates no weighted composite score. Every complete run must include the sampler candidate-pool/receipt digests and allocator/latency-bound digests; the `not_run` template leaves them null and makes no performance claim.

### Statistical comparison contract

Every complete result card must carry one record per expected evaluation episode:
`episode_id + scenario_family + scenario_cluster_id + disposition + explicit native metric values`.

The comparison pairs **the exact same episode IDs** and rejects differences in scenario family, cluster identity or disposition. It does not silently intersect two different denominators or discard a null metric. Repeated rollouts/seeds from one scenario are averaged inside the scenario cluster before resampling; scenario clusters are then sampled with replacement inside each scenario-family stratum. This avoids treating adjacent frames or repeated rollouts of the same scene as independent evidence.

The bootstrap seed is derived from the first 64 bits of the frozen seed-manifest SHA-256, and the output includes paired-episode count, scenario-cluster count, family count, exact seed, replicate count, candidate-minus-baseline point estimate, 95% interval and whether the interval lies wholly in the candidate-favorable direction. The interval is an unadjusted percentile interval; it is not a multiplicity-adjusted certification of superiority. Pre-register one primary metric and report all secondary/family metrics separately.

To claim non-inferiority on a higher-is-better metric with margin (m), require the lower confidence bound on candidate-minus-baseline to be at least (-m). For a lower-is-better metric, require the upper bound to be no greater than (m). The existing point-estimate margin helper is only a screening calculation, not the final inferential test.

The evaluator reports native metric differences, uncertainty and resource ratios; it does not combine driving score, energy, latency, memory or safety into a fabricated single score. The Pareto frontier is an efficiency/capability analysis, not a safety certificate.

The current suite contains 27 authored Rust unit tests covering denominator accounting, episode pairing, cluster mismatch rejection, deterministic bootstrap, missing metrics, matched-budget classification, privilege mismatch, benchmark mismatch, Pareto trade-offs, incomplete measurements, protocol separation, margin direction and schema/template parsing.

## Claim ceiling

Passing a manifest/schema audit establishes only structural protocol conformance. A positive simulator result applies only to the exact model, code, dataset, benchmark version, evaluator, hardware, privileges, scenario population, random seeds and sealed split measured. It does not establish roadworthiness, real-road transfer, collision probability, certification or SAE Level 4/5 capability.
