# Recoverable Driving Experience Benchmark v0.1

**Status:** research protocol and machine-readable manifest; not an implemented simulator, training sampler, or driving result  
**Owner:** [#4550 — RECOVERABLE-DRIVING-EXPERIENCE-001](https://github.com/Luminous-Dynamics/symtropy/issues/4550)  
**Environment:** [#4536 — camera-first virtual driving environment](https://github.com/Luminous-Dynamics/symtropy/issues/4536)  
**Curriculum:** [#4538 — Symthaea driver apprenticeship curriculum](https://github.com/Luminous-Dynamics/symtropy/pull/4538)  
**Learner:** [Symthaea #11414](https://github.com/Luminous-Dynamics/symthaea/issues/11414)

## 1. Research question

Does choosing driving experiences by **decision-sensitive value of information and recoverability** produce more durable driving competence per simulator transition, human intervention minute, compute budget and energy budget than uniform sampling, rarity sampling, predictive uncertainty, temporal prediction error, or decision-boundary sampling?

This is deliberately narrower than asking whether Symthaea is a better driver. It isolates the learning mechanism that could make an embodied learner improve faster, transfer better, and preserve recoverable choices when its current model is wrong.

The hypothesis is falsifiable. A null result or a result favouring a simple sampler is useful.

## 2. Paradigm shift: learn by choosing experiments

The ordinary loop is:

`observe -> act -> receive outcome -> update policy`

The proposed loop adds an explicit experiment-selection layer:

`observe -> maintain hypotheses -> identify decision sensitivity -> choose an informative intervention -> branch and re-simulate causal consequences -> learn -> measure changed competence and recoverability`

The learner should not merely seek unfamiliar scenes, high entropy or large prediction errors. It should seek experiences where the information can change an important future decision before that decision's deadline.

Examples:
- A pedestrian is partially occluded. A second view may distinguish a safe yield from a blocked path.
- A lead vehicle brakes. The useful query may be whether available stopping distance survives a delayed actuator, not whether the frame encoder is uncertain.
- A merge has two plausible actor-response modes. An intervention that changes the ego gap may reveal whether another driver yields or accelerates.
- A stale camera frame and a delayed control command occur together. The useful experience tests whether the controller retains a feasible recovery path, not merely whether it predicts the next image.

SAM supplies independent decision/recoverability evidence. The learner's own confidence is not its safety certificate.

## 3. Experimental arms

All arms begin from the same declared checkpoint, initial corpus and curriculum state. They receive matched budgets.

| ID | Selection mechanism | What it tests |
|---|---|---|
| `uniform` | Uniform over eligible episodes/scenario families | Data-volume reference |
| `rarity` | Frequency / inverse-frequency scenario sampling | Long-tail exposure |
| `entropy` | Predictive entropy or ensemble disagreement | Generic uncertainty sampling |
| `prediction_error` | Recent temporal/world-model prediction error | Surprise-driven practice |
| `decision_margin` | Near action-ranking boundaries or small feasible-set margins | Decision-boundary practice |
| `sam_value_of_information` | Estimated value of information for a decision before its deadline | Decision-sensitive teaching |
| `sam_recoverability` | Counterexamples selected by option fragility, recovery depth or residual decision time | Recoverability-directed learning |
| `fixed_human_intervention` | Fixed-rate instructor intervention/correction | Human-effort reference |

A human demonstration arm is a reference distribution, not an autonomous policy. Add hybrid combinations only after the primary arms have been reported separately.

The sampler must be tested with the same policy family and model architecture first. Model-architecture ablations are a separate axis:
- current engineered-state HDC-LTC reference;
- matched visual GRU/LSTM;
- matched visual compact transformer;
- visual encoder + structured HDC + LTC/CfC;
- the same hybrid plus action-conditioned latent world model;
- the same world-model arm plus independent SAM-guided experience selection.

Do not attribute sampler gains to an architecture change or architecture gains to a sampler change.

## 4. Counterfactual branch protocol

### 4.1 Branch point

A branch point is an immutable, receipted decision boundary. It binds:
- episode, branch and parent-branch identity;
- policy-visible observation/history digest and acquisition/delivery times;
- learner checkpoint, recurrent-state policy and inference configuration;
- simulator, renderer, vehicle, map, scenario and actor-policy revisions;
- declared traffic-rule profile and action-space version;
- candidate intervention identity and action sequence;
- source-qualified exogenous event manifest and random keys;
- evaluator-only world-state reference, stored outside policy-visible inputs.

A branch is not identified only by a random seed. The seed, scenario revision, actor policy, model revision and complete initial-state manifest are part of its identity.

### 4.2 Causal replay

When the ego action changes, other agents may respond differently. Therefore:
- Do **not** replay an actor's old action as though it were its response to a new ego action.
- Recompute actor-policy descendants affected by the intervention.
- Reuse random keys only for exogenous variables demonstrated to be intervention-invariant.
- Recompute endogenous/counterfactual descendants; record their new lineage.
- Preserve a branch manifest showing which exogenous inputs were shared, which descendants were recomputed and which evidence is missing.

If causal classification is unknown, do not claim paired counterfactual equivalence. Keep the branch as unpaired/unknown.

### 4.3 Privilege separation

The learner and sampler may use only declared policy-visible information available at the query time. Hidden simulator state, teacher actions, future frames, branch outcomes, evaluator labels and oracle action rankings are evaluator-only unless a training arm explicitly declares that privilege.

Frozen evaluation disables:
- teacher-action blending and future-label access;
- online updates and exploration noise;
- query-selection access to future branch outcomes;
- train/holdout scenario-family overlap;
- recurrent state or checkpoint mutation.

A query decision must be timestamped before its answer/outcome becomes available.

## 5. Metrics: keep the evidence multidimensional

Do not collapse the benchmark into one scalar. Publish every family below, with per-scenario-family distributions and confidence intervals across independent seeds.

### Learning efficiency
- New simulator transitions to a preregistered skill threshold.
- Instructor intervention minutes and correction count to the same threshold.
- Area under competence-versus-transition and competence-versus-human-time curves.
- Regression/relearning cost after the environment or actor-policy distribution changes.

For an active arm, relative transition reduction is:

`1 - transitions_active / transitions_uniform`

Use the same frozen competence threshold and evaluation set. The initial 20% improvement target is a research target, not a safety threshold.

### Decision and query value
- Query precision: fraction of requested demonstrations/measurements that change a feasible action set, action ranking, recovery classification or uncertainty disposition.
- Query utility: held-out outcome difference after the answer/correction, against a matched no-query continuation.
- Query regret: realized loss from asking too late, asking the wrong question or not asking when a decision would have changed.
- Policy-change rate: fraction of answers that change the action or its admissibility; report alongside outcome benefit so a changed action is not automatically counted as useful.

### Action-conditioned model quality
- Predictive error for ego and actor trajectories, road-relative state and sensor outcomes.
- Proper probabilistic scores (for example NLL or CRPS when their distributional assumptions are met).
- Coverage and sharpness of predeclared prediction sets/intervals.
- Action-ranking regret evaluated using the simulator oracle only after the policy decision.
- Counterfactual sensitivity: whether distinct interventions produce the correct direction and magnitude of downstream changes.

Visual fidelity is reported separately. A photorealistic future that violates vehicle dynamics or actor response is not a good driving world model.

### Recoverability and constraint performance
Consume SAM's independent, source-qualified outputs:
- viable alternative-action count/set and option width;
- policy-tree or state-space recoverability depth;
- option fragility under declared faults;
- residual decision time before fallback becomes infeasible;
- fallback admissibility and evidence completeness;
- other-agent option externalities, reported separately from ego options.

Report contact/collision, traffic-rule, hard-constraint, route-progress and comfort/jerk outcomes under explicit simulator semantics. Do not let average progress hide unsafe or unrecoverable episodes.

### Resource performance
- p50/p95/p99 inference, sensing, planning, action-latch and recovery latency;
- model parameters, resident memory, peak memory and compute;
- measured energy per environment transition and per unit competence gain, only when raw, source-grade telemetry exists;
- simulator wall time separately from simulated time.

Synthetic estimates must be labelled synthetic. Missing telemetry stays missing.

### Evidence completeness
- expected versus observed episodes, branches, sensors and action receipts;
- failed, incomplete, censored, timed-out and excluded episodes;
- exact checkpoint, manifest, code, simulator and dataset digests;
- fresh-process replay agreement;
- denominator and exclusion reconciliation.

No missing or failed episode may silently disappear from a reported score.

## 6. Matching and evaluation design

Run two distinct resource comparisons.

**Exposure-matched:** same number of environment transitions, scenario exposures, update opportunities and human corrections. This isolates what the learner learned from the same experience budget.

**Cost-matched:** same wall-clock, accelerator/CPU, memory and human-intervention budget. This evaluates practical learning efficiency. Do not conflate the two.

Keep the following fixed or explicitly match them:
- starting checkpoint and recurrent state;
- sensor modality, observation history, action space and control timing;
- scenario population and split manifests;
- model parameter count and compute budget where possible;
- optimizer, training steps and update rules;
- training data, intervention opportunities and simulator transition budget.

When exact matching is impossible, report the mismatch and avoid a causal claim.

Split by map, route geometry, scenario family, actor-policy family, weather/lighting, vehicle profile, traffic convention, sensor fault family and deployment perturbation. Hold out combinations, not merely random frames from the same episode.

Report mean, median, worst-family, 5th percentile and CVaR for declared tail levels. Confidence intervals must account for independent scenario seeds; frames from one episode are not independent samples.

## 7. Required negative controls

1. Simulator truth accidentally enters a camera-only policy or sampler.
2. Future observations, future actions or branch outcomes leak into a query at time t.
3. Teacher blending remains active during frozen evaluation.
4. A stale/duplicated frame is admitted as fresh independent evidence.
5. A changed ego action is paired with the old actor actions rather than re-simulated actor response.
6. A purportedly exogenous event changes with ego action without being reclassified.
7. The sampler selects only easy/high-reward scenes and neglects critical rare families.
8. An always-stop policy appears successful because progress/mission completion is not measured.
9. Reward hacking improves aggregate reward while violating hard constraints.
10. Training and holdout share maps, actor policies, randomization seeds or scenario templates.
11. Failed, censored or incomplete episodes disappear from denominators.
12. Model checkpoint, recurrent state or policy configuration changes during evaluation.
13. The sampler uses outcome labels that would not exist at its decision time.
14. The world model predicts plausible images but physically impossible trajectories.
15. SAM outputs are copied from the learner instead of independently recomputed/qualified.

The benchmark should prove these controls are rejected before positive results are interpreted.

## 8. Promotion gates and prospective target

A candidate sampler may advance only after:
1. protocol and fixture conformance;
2. negative-control rejection;
3. matched exposure study;
4. cost-matched study;
5. held-out map/actor/fault-family evaluation;
6. independent SAM/recoverability assessment;
7. exact replay and complete denominator reconciliation.

**Initial research target:** at least one active sampler reaches the same preregistered competence threshold using at least 20% fewer simulator transitions or instructor-intervention minutes than uniform sampling, with no regression in predeclared worst-family constraints, calibration, fallback integrity, held-out transfer or deadline compliance.

The exact competence threshold, non-inferiority margin, confidence level, tail levels and family coverage must be frozen before evaluation begins. The 20% target is not a universal guarantee and is not an achieved result.

## 9. External benchmark positioning

Use each external benchmark for the question it actually measures:

| Benchmark | Proper role | Limitation / control |
|---|---|---|
| CARLA local closed-loop scenarios | Closed-loop interaction and controlled scenario execution | The original public Leaderboard 2.1 submission service is closed; pin local simulator, scenario runner and route revisions |
| Bench2Drive | Closed-loop route and scenario performance | Pin version and score semantics; leaderboard results vary by data/compute/teacher/privilege |
| Bench2Drive-Robust | Deployment perturbations: dropped/partial frames, state noise and inference delay | Preserve exact perturbation protocol; do not mix with clean Driving Score |
| NAVSIM v2 | Planning metric (EPDMS) and follow-up-scene evaluation | It approximates closed-loop effects with selected precomputed follow-up scenes; it is not a reciprocal online simulator |
| DrivingGen | Video/trajectory generation, consistency and controllability | World-model evaluation, not a substitute for driving competence |
| Waymo Open Dataset | Large-scale perception, motion and end-to-end driving data/evaluation | Confirm data license and challenge task; dataset results are not physical vehicle validation |

External scores are not pooled. Report native score names and evaluation protocols.

### Bench2Drive v0.0.4 reference snapshot

The Bench2Drive repository marks v0.0.4 (released 6 August 2026) as the version currently in use. It adds a strictly uniform 1,100-route training set (44 scenarios × 25 routes) and a separate 220-route evaluation set (44 scenarios × 5 routes), plus 3D occupancy and dataset/sensor bug fixes. This makes it a particularly useful uniform-sampling reference for the apprenticeship programme. The repository's v0.0.4 results table lists:
- LEAD: 95.59 Driving Score, 87.28% Success Rate, 201.55 Efficiency, 20.56 Comfort.
- Simlingo: 86.55 Driving Score, 70.45% Success Rate, 251.72 Efficiency, 35.47 Comfort.

These are upstream-reported results for the named benchmark revision, not independent Symtropy reproductions. The earlier 89.07 AlignDrive snapshot belongs to a different hosted-board snapshot/version context and must not be carried forward as the current v0.0.4 reference. Pin the exact evaluator, route XML, code tag, model/checkpoint, sensor profile, training data and compute/privilege budget before attempting reproduction. See the [v0.0.4 release results](https://github.com/Thinklab-SJTU/Bench2Drive/blob/0.0.4/docs/v004_update.md) and [tagged repository README](https://github.com/Thinklab-SJTU/Bench2Drive/blob/0.0.4/README.md).

### Deployed-industry comparison is a separate evidence class

A simulation benchmark and an operating driverless service do not measure the same thing. For calibration, Waymo's public safety-impact dashboard reports 271.3 million rider-only miles through June 2026 and compares crash rates against location-adjusted human benchmarks. Waymo reports 95% fewer serious-injury-or-worse crashes and 82% fewer injury-causing crashes in the operating locations covered by that analysis. The same materials distinguish retrospective safety-impact analysis from pre-deployment release readiness, which Waymo evaluates through its Safety Framework and Safety Case. These are company-published operational results, not results Symthaea can meaningfully claim to match through simulation alone. See [Waymo Safety Impact and methodology](https://waymo.com/safety/impact/).

Tesla's public FSD (Supervised) safety report is another, different form of fleet telemetry: it describes collision event definitions and compares FSD (Supervised) driving against manually driven Tesla vehicles with and without active safety features. Tesla counts a collision as FSD-engaged when FSD was active at any point in the five seconds before the event, and does not assign fault in its reported collision data. Its event definitions, road mix, control mode and comparison population differ from Waymo's and from simulator benchmarks. Do not compare raw cross-company rates without reconstructing the methodology and denominators. See [Tesla's FSD (Supervised) Vehicle Safety Report and methodology](https://www.tesla.com/fsd/safety).

### NVIDIA Alpamayo as a current platform comparator

NVIDIA's Alpamayo Open Platform (page updated August 2026) is not just a driving model. It combines 10B-parameter Alpamayo 1/1.5 Nano and 34B Alpamayo 2 Super VLA models, multi-camera RGB plus ego-motion inputs, 64-waypoint trajectory outputs, Chain-of-Causation explanation text, AlpaSim closed-loop simulation, AlpaGym reinforcement learning, auto-labeling and open driving datasets. NVIDIA describes the models as cloud-side teacher models that can be distilled into onboard DRIVE AGX Thor student policies; they are not described as direct in-vehicle runtimes.

This creates a useful competitive reference:
- **Do not compete on parameter count.** Compare against Alpamayo as a high-capacity teacher / reasoning-VLA baseline where its published protocols, weights, data and licences permit.
- **Compete on the student capability frontier.** Test whether a substantially smaller HDC/LTC student can preserve task competence while improving adaptation cost, irregular-time sensor robustness, memory/energy use and recovery quality.
- **Make explanations evidence-grounded.** A generated Chain-of-Causation text is useful for review, but Symthaea should attach each decision claim to policy-visible observation identities, predicted consequences, uncertainty and independently qualified SAM evidence. Text alone is not an assurance artifact.
- **Use the ecosystem comparison honestly.** NVIDIA already offers a simulator, RL stack, teacher models, data and reasoning outputs. Symtropy must demonstrate equivalent closed-loop experiment discipline and benchmark reproduction before architectural differentiation is persuasive.

Source: [NVIDIA Alpamayo platform description and August 2026 update](https://www.nvidia.com/en-us/solutions/autonomous-vehicles/alpamayo/llm-info/).

**Strategic implication:** Symthaea is currently a research architecture and specification programme, not an industry-level autonomous-driving product. The credible path is: reproduce public research baselines; show a predeclared architectural or learning-efficiency advantage under matched budgets; validate robustness and recovery independently; then build a separately governed, staged physical evidence case. Fleet mileage and safety-case evidence are a long-term validation destination, not a near-term leaderboard target.

The 10 November 2026 literature snapshot includes:
- DriveWorld-VLA (ICML 2026), which reports NAVSIMv1 PDMS 91.3 and NAVSIMv2 EPDMS 86.8 for its own protocol.
- DriveVLA-W0 (ICLR 2026), which combines future-image world modeling with VLA planning and reports experiments on NAVSIM plus a substantially larger in-house dataset.
- World-In-World (ICLR 2026), which argues that visual fidelity alone is insufficient and emphasizes closed-loop controllability/action-observation training.
- Bench2Drive-Robust (2026), which explicitly studies system-side perturbations including camera failures, localization noise and inference delay.
- DrivingGen (ICLR 2026), which separates generated-video quality, trajectory plausibility, temporal consistency and controllability.

These results establish strong comparators and evaluation dimensions, not a direct ranking of Symthaea.

## 10. Integration and ownership

This benchmark consumes, rather than reimplements:
- Symtropy #4536 observation/action/transition and event receipts;
- Symtropy deterministic scenario execution and replay owners;
- Symtropy reciprocal actor execution owners;
- Symthaea #11414 model and apprenticeship ablation programme;
- SAM option preservation, counterexample discovery, decision-sensitive measurement, fragility and independent runtime assurance.

No second safety authority or duplicate option evaluator is introduced.

## 11. Current implementation boundary

This PR adds the protocol document, machine-readable manifest and JSON Schema. It does not add a simulator, renderer, camera driver, trained model, training run, human demonstration dataset or driving result.

Two independent executable research PRs now cover the first finite-record layer:
- [Symtropy #4556 — recoverable driving branch evidence auditor](https://github.com/Luminous-Dynamics/symtropy/pull/4556): validates branch lineage, actor response provenance, explicit exogenous-randomness pairing, query-time privilege and episode/branch denominators. It cannot prove the truth of upstream causal declarations.
- [Symtropy #4575 — deadline-aware decision-value experience selector](https://github.com/Luminous-Dynamics/symtropy/pull/4575): deterministically compares seven sampling strategies and rejects SAM-guided queries that lack qualified evidence, positive expected decision value or sufficient time before the decision deadline.

Both are draft PRs. Their exact-head Rust checks have not yet completed; neither is represented as compiled/tested or scientifically validated.

The next environment deliverable remains a minimal offscreen camera-to-control episode with the #4536 observation/action/transition receipts, fresh-process replay and a documented vehicle/renderer fidelity boundary. The selector and branch auditor must be integrated only after their schemas and tests are reviewed.

## References

- [DriveWorld-VLA, ICML 2026](https://proceedings.mlr.press/v306/jia26f.html)
- [DriveVLA-W0, ICLR 2026](https://proceedings.iclr.cc/paper_files/paper/2026/hash/0d70423f59c5fdd24f0dd3fa52e34623-Abstract-Conference.html)
- [World-In-World, ICLR 2026](https://proceedings.iclr.cc/paper_files/paper/2026/hash/5b4263be85820683d78675cc18d2efc7-Abstract-Conference.html)
- [DrivingGen, ICLR 2026](https://drivinggen-bench.github.io/)
- [Bench2Drive v0.0.4 release results and data split](https://github.com/Thinklab-SJTU/Bench2Drive/blob/0.0.4/docs/v004_update.md)
- [Bench2Drive-Robust paper](https://arxiv.org/abs/2605.18059) and [code](https://github.com/Thinklab-SJTU/Bench2Drive-Robust)
- [NVIDIA Alpamayo Open Platform](https://www.nvidia.com/en-us/solutions/autonomous-vehicles/alpamayo/llm-info/)
- [Waymo Safety Impact dashboard](https://waymo.com/safety/impact/)
- [Tesla FSD (Supervised) Safety Report](https://www.tesla.com/fsd/safety)
- [NAVSIM v2 metric definition](https://github.com/autonomousvision/navsim/blob/main/docs/metrics.md)
- [Bench2Drive leaderboard snapshot](https://carlaleaderboard-bench2drive.hf.space/)
- [Waymo Open Dataset challenges](https://waymo.com/open/challenges/)
- [CARLA Leaderboard 2.1 status](https://leaderboard.carla.org/get_started_v2_1/)
