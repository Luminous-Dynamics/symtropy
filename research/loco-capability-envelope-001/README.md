# LOCO-CAPABILITY-ENVELOPE-001 — Cross-fitted held-out terrain capability and tail-risk benchmark

**Status:** protocol and synthetic fixtures. No robot performance result or safety claim.

Issue: [#4327](https://github.com/Luminous-Dynamics/symtropy/issues/4327).

## Objective

Measure empirical locomotion capability and tail-risk across matched multidimensional terrain/disturbance families, including held-out structured terrain, using cross-fitted lower-tail evaluation and per-feature success envelopes.

This is a performance/generalization benchmark. It complements:
- #3357 matched locomotion baselines;
- #3475 degraded-loop and perturbation recovery;
- #4304/#4308 robust viability frontiers;
- #4324/#4326 multi-outcome safety-vector audit;
- #4044/#4179/#4206 energy/actuator evidence;
- #4310/#4313 viability-conditioned probing/adaptation.

**Not interchangeable:** an empirical 50%-success capability envelope is not a robust viability certificate. Sampled CVaR is not a worst-case proof. Curriculum performance is not evidence that a continuous uncertainty abstraction is sound.

## Why now: 2026 UED comparison

Gilewicz, Galus & Wojdas, *Is It the Curriculum or Just the Easy Start? Benchmarking Unsupervised Environment Design on Quadrupeds* (arXiv:2610.40364, 29 Oct 2026) evaluate 14 UED/curriculum methods across five morphologies with matched PPO environment-step budgets and five training seeds. Their terrain space has eight dimensions. Evaluation includes random levels, seven held-out structured terrain families, cross-fitted CVaR25 (difficulty ranking and tail scoring on separate evaluation passes), per-feature 50%-success capability envelopes and pairwise level-wise domination. Their in-distribution random-level leader is not the held-out leader on Go1, Go2 and Barkour.

Paper: https://arxiv.org/abs/2610.40364
Public code/level space/evaluation suite: https://github.com/Solvro/ml-wojtek-ued

The benchmark lesson: **training-distribution mean success, held-out transfer and tail robustness are different properties**. Our scorecard must keep them separate.

## Frozen level and experiment identity

Each environment level binds:
- stable level ID, version and replay/content digest;
- morphology and robot configuration digest;
- terrain/disturbance feature vector with units and bounded support;
- friction/contact uncertainty and actuator-fault family;
- observation delay, occlusion and sensor-health profile;
- task, route, command and payload profile;
- environment seed and simulator/physics digest;
- train/validation/held-out split membership and split manifest digest.

Start with an eight-axis-compatible terrain vector:
1. step height;
2. gap width;
3. slope/tilt;
4. terrain roughness/height variation;
5. friction lower bound or interval width;
6. obstacle density/spacing;
7. payload or centre-of-mass offset;
8. external impulse/force severity or a separately stratified actuator-derating regime.

Do not presume axes are independent. Each coordinate must declare units, constructor, range, direction and whether it induces nested uncertainty sets. Non-nested fault or terrain families are categorical strata rather than monotone axes.

## Evaluation split and cross-fitting

Freeze levels before training. Bind train, validation, difficulty-ranking and final tail-scoring pools to distinct manifests.

- A curriculum may see only the levels and feedback explicitly permitted for its method.
- Held-out structured families cannot be used for curriculum, hyperparameter or checkpoint selection.
- Partition test episodes into a *difficulty-ranking* split and a *tail-scoring* split. Rank/identify the lower-tail subset using the first split; measure CVaR on the second. Swap the two roles and combine the estimates. Do not select and score the worst episodes on the same observations.
- Preserve the training seed as the top-level cluster. Episodes, control timesteps and repeated rollouts of one trained policy are not independent training seeds.
- Report seed-level values and uncertainty intervals. Pooled episode count does not replace independent training runs.

## Standardized complete-machine locomotion profile

In addition to the multidimensional UED/capability evaluation, add a separate **ISO 18646-5:2026** profile for reproducible machine-performance comparisons. The standard (published July 2026) covers legged-robot locomotion performance and is intended for complete-machine tests; its scope does not verify or validate safety requirements.

Map the standard's applicable test procedures into source-pinned trial manifests for:
- speed and stopping distance;
- climbing and stair traversal;
- slope traversal;
- obstacle crossing and clearance.

For every test preserve the full standard edition/profile, robot configuration, surface/obstacle geometry, payload, speed/command, environmental conditions, sensor/controller versions, exposure, repeated trials and raw outcomes. Do not reproduce ISO procedures from memory: the exact controlled conditions and acceptance methods must be taken from the purchased/authorized standard text and version-pinned in the run manifest.

ISO performance results are reported separately from:
- robust viability/recovery proofs (#4308);
- calibrated risk and state-dependent action admission (#4119/#4231);
- multi-outcome adverse-event vector (#4326);
- held-out capability envelopes and CVaR (this issue).

A robot can pass a performance test while lacking a validated safety case. A higher ISO locomotion score does not override a false-safe result or an adverse-event regression.

Reference: https://www.iso.org/standard/86850.html

## Capability envelope

For a feature axis (j), a declared fixed slice of all other dimensions, and success criterion (q), estimate the largest tested severity level at which the empirical success rate remains at least 50%. Report:
- discrete tested levels and exact successes/trials;
- interval for success probability at each level;
- largest tested passing level and smallest tested failing level;
- censoring if all tested levels pass or all fail;
- non-monotonicity diagnostic;
- slice identity, morphology, policy and train/test manifests.

Do not interpolate a physical critical threshold by default. If success is non-monotone in severity, report the sampled response surface and suppress a single-threshold envelope. Do not compare per-axis thresholds across different slices as if they described one joint safe region. Interactions are evaluated by joint grids / factorial slices and cross-referenced with #4308.

## Cross-fitted tail metric

For episode outcome loss (L) and (alpha=0.25), compute empirical lower-tail performance / CVaR25 under one preregistered sign convention. Preserve the complete ranked loss vector, tie rule, split IDs, selected tail size and the split used to select the tail. Report both passes and the combined estimate. Do not silently call this a robust worst-case guarantee.

Where rewards are used instead of losses, define the lower-tail reward convention explicitly. Keep completion, fall, recovery and adverse-contact distributions alongside the chosen tail metric.

## Metric vector — no universal scalar score

Report independently:
- task completion probability and route completion;
- time-to-completion, steps/distance-to-failure and throughput;
- per-family and held-out success distributions;
- cross-fitted CVaR25 and mean/median;
- per-feature 50%-success envelope and confidence interval;
- recovery success, recovery time, cost and remaining horizon;
- multi-outcome safety vector from #4326 (falls, slip, foot scuff, shank/body collisions, impact, emergency recovery, Unknown/missingness);
- energy at each declared boundary and CoT only within comparable definitions;
- actuator torque/slew/thermal utilization;
- observation age, inference latency, deadline misses, CPU/GPU and memory.

No aggregate weighted score may hide an adverse-category regression or held-out family failure.

## Matched baseline matrix

All formal comparisons use the same robot/morphology, task/action alphabet, training steps/interaction budget, compute budget, frozen train/validation/test manifests and test exposure:
- PPO domain randomization without adaptive level generation;
- random curriculum;
- PLR / learning-progress;
- PAIRED / ACCEL and other UED methods where reproducibly executable;
- adversarial / minimax terrain curriculum;
- PSC predictive safety curriculum;
- CPG/rule-based and heuristic foothold baselines;
- nominal MPC, CBF/MPC and impact-aware robust MPC;
- SafeMind probabilistic CBF / differentiable-QP baseline (IEEE Access 2026): 200 Hz Unitree A1 and ANYmal C deployment; paper-reported results are protocol-specific and must be remeasured under matched strata;
- STEP-CBF+ reduced-order foothold/barrier screening (Scientific Reports 2026): 13.5 kg quadruped simulation, 49 foothold candidates, 13.6 ms mean selection; not hardware or whole-body invariance evidence;
- impact-aware robust convex MPC (IEEE RA-L 2026): include the Go2 hardware uncertain-impact tests separately from simulation;
- ARER-MetaRL-style short-context fault adaptation for locked-joint strata;
- reference-free sampling-based MPC with no handcrafted gait/contact schedule;
- Symtropy full stack and ablations (no probing, no context adaptation, no robust viability gate, no recovery objective, no safety-vector audit, no actuator-model transfer).

A method that cannot be run at matched budgets is labelled *unmatched* and excluded from formal superiority ranking.

## Statistical and evidence contract

- Pair methods by training-seed blocks, morphology and frozen level IDs.
- Use cluster-aware intervals / hierarchical models with training seed as top-level unit. No timestep-level pseudo-replication.
- State the estimator, interval method, monotonicity test, tail split procedure and censoring for each envelope/CVaR value.
- Report held-out families separately; any pooled weighting is preregistered.
- A five-seed paper baseline is a comparator, not an adequacy target; size the final experiment from interval width, power and between-seed variance.
- Simulator, biological and physical hardware evidence remain separate tiers.
- A safety-improvement claim additionally requires the full #4326 taxonomy and physical gates; a capability-envelope improvement cannot override them.

## Frozen known-answer / adversarial cases

Machine-readable fixtures: [fixtures/known_answers_v0_1.json](fixtures/known_answers_v0_1.json).

1. In-distribution easy-start curriculum wins random levels but loses held-out structured terrain.
2. Mean reward improves while cross-fitted CVaR25 worsens.
3. Monotone success profile with a known discrete 50%-success bracket.
4. Non-monotone success profile; threshold is suppressed and surface is returned.
5. Joint high-high severity fails although each one-axis max passes.
6. Train/test level digest overlap invalidates held-out claims.
7. Same policy tested under two morphologies; pooling is refused.
8. Ranking and tail-scoring split accidentally overlap; cross-fit rejected.
9. Exposure/episode counts differ; no implicit reweighting.
10. Safety taxonomy/detector digest differs; safety results not pooled.
11. Unknown/unclassified measurement increases with severity; not counted as success.
12. Probe consumes time/horizon; post-probe success and recovery use remaining horizon.
13. Hardware results and simulator results are distinct evidence tiers.
14. A wider 50%-success envelope coexists with more foot scuffs/falls; performance and safety are reported separately.

## Proposed qualification targets (not achieved)

- 100% manifest, split and seed-block validation on frozen fixtures.
- Independent cross-fit CVaR implementation reproduces the exact fixture result.
- Exact envelope brackets on monotone fixtures; correct abstention on non-monotone cases.
- Zero train/test or difficulty-rank/tail-score leakage.
- Zero pooling across unmatched morphology, physics, exposure or detector identities.
- Per-family uncertainty intervals and raw trial counts are always preserved.
- No overall improvement claim if a required adverse category is unmeasured or regresses.
- Stretch: improve held-out CVaR25 and/or capability envelope at equal interaction/compute budget without worsening a hard physical constraint or adverse-outcome category.

## Claim ceiling

This benchmark measures empirical sampled success, transfer and lower-tail outcomes over frozen distributions. It does not prove worst-case safety or continuous viability and does not replace #4308. Physical safety claims require independent event classification, sound physical abstraction and qualified hardware trials.
