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

The aim is not to prove that an HDC model is smaller than a transformer. The aim is to find the best measured competence/constraint frontier and determine which Symthaea components actually contribute.

## What the field is doing (snapshot: 11 November 2026)

| Comparator | Public reference | How to use it |
|---|---|---|
| Bench2Drive v0.0.4 | LEAD: 95.59 Driving Score; 87.28% Success Rate; separate efficiency and comfort measures | Closed-loop route benchmark; pin exact tag, routes, sensor profile and evaluator |
| DriveWorld-VLA, ICML 2026 | NAVSIMv1 PDMS 91.3; NAVSIMv2 EPDMS 86.8; nuScenes 3-second average collision rate 0.16 | Action-conditioned latent world-model/VLA reference; preserve each native task metric |
| World-In-World, ICLR 2026 | Closed-loop platform emphasizing task success, controllability, action-observation post-training and inference-time compute | Evaluate whether predicted worlds improve agent success, not just video fidelity |
| NVIDIA Alpamayo 2 Edge, released 10 Nov 2026 | Model card: 2.45B backbone + 0.2B action expert; 15.15 ± 0.04 km per at-fault incident on 875 AlpaSim scenarios / three rollout sets; minADE6 0.52m at 5s on 1,392 samples; Lingo-Judge 75.2; 115,000h video and about 3.7M CoC traces | Strong edge-VLA comparator. Keep closed-loop, open-loop, reasoning, training data and latency as distinct metric families |
| Alpamayo 2 Edge deployment recipe | NVIDIA's named DRIVE AGX Thor-X recipe reports 160–190ms pixel-to-trajectory latency on DRIVE OS 7.2.7.0, CUDA 13.4 and TensorRT 11.3.1.9 | Latency comparator only on a matching hardware/software measurement configuration |
| Alpamayo 2 Super | 34B reasoning/teacher model with trajectory, CoC reasoning and auto-label outputs | High-capacity teacher; account for teacher privilege and generated supervision |
| Waymo WOD-E2E, CVPR 2026 | 4,021 segments / approximately 12h curated for long-tail E2E scenarios with reported occurrence below 0.03% | Rare-event dataset reference; offline dataset, not a closed-loop recovery metric |
| Waymo Open Dataset challenges | Formal challenges are not hosted in 2026, but public leaderboards remain active | Public evaluator/data route; pin the exact active leaderboard and track |
| CARLA Leaderboard 2.0 | Separate SENSORS and MAP tracks with route completion and infraction penalties; official challenge is permanently closed | Historical methodology reference only, not an active submission path |
| Waymo Driver | Company dashboard: 271.3M rider-only miles through June 2026; company-reported 95% fewer serious-injury-or-worse crashes and 82% fewer injury-causing crashes against its location-adjusted human benchmark | Operational safety evidence class and eventual safety-case reference, not a simulator leaderboard |

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
- **Human reference:** demonstrations/corrections; not treated as an autonomous policy.

All arms must expose the same policy-visible observation and action interface for matched comparisons. If a comparator requires a different sensor history, navigation input or teacher privilege, record that mismatch rather than hide it.

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

## Files

- `benchmark_manifest_v0_1.json`: comparator register, model arms, metrics, budget dimensions, benchmark ladder, negative controls and prospective targets.
- `result_card.schema.json`: schema for per-run normalized evidence records.
- `result_card.template.json`: an explicit not-run template; null values are not measurements.

## Executable result-card evaluator

The standalone Rust evaluator is in `evaluator/`. It is pinned to Rust 1.96.0 and only uses Serde/Serde JSON for typed result-card parsing.

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

### Statistical comparison contract

Every complete result card must carry one record per expected evaluation episode:
`episode_id + scenario_family + scenario_cluster_id + disposition + explicit native metric values`.

The comparison pairs **the exact same episode IDs** and rejects differences in scenario family, cluster identity or disposition. It does not silently intersect two different denominators or discard a null metric. Repeated rollouts/seeds from one scenario are averaged inside the scenario cluster before resampling; scenario clusters are then sampled with replacement inside each scenario-family stratum. This avoids treating adjacent frames or repeated rollouts of the same scene as independent evidence.

The bootstrap seed is derived from the first 64 bits of the frozen seed-manifest SHA-256, and the output includes paired-episode count, scenario-cluster count, family count, exact seed, replicate count, candidate-minus-baseline point estimate, 95% interval and whether the interval lies wholly in the candidate-favorable direction. The interval is an unadjusted percentile interval; it is not a multiplicity-adjusted certification of superiority. Pre-register one primary metric and report all secondary/family metrics separately.

To claim non-inferiority on a higher-is-better metric with margin (m), require the lower confidence bound on candidate-minus-baseline to be at least (-m). For a lower-is-better metric, require the upper bound to be no greater than (m). The existing point-estimate margin helper is only a screening calculation, not the final inferential test.

The evaluator reports native metric differences, uncertainty and resource ratios; it does not combine driving score, energy, latency, memory or safety into a fabricated single score. The Pareto frontier is an efficiency/capability analysis, not a safety certificate.

The current suite contains 21 Rust unit tests covering denominator accounting, episode pairing, cluster mismatch rejection, deterministic bootstrap, missing metrics, matched-budget classification, privilege mismatch, benchmark mismatch, Pareto trade-offs, incomplete measurements, protocol separation, margin direction and schema/template parsing.

## Claim ceiling

Passing a manifest/schema audit establishes only structural protocol conformance. A positive simulator result applies only to the exact model, code, dataset, benchmark version, evaluator, hardware, privileges, scenario population, random seeds and sealed split measured. It does not establish roadworthiness, real-road transfer, collision probability, certification or SAE Level 4/5 capability.
