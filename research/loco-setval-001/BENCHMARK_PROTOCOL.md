# LOCO-SETVAL-001 — Matched six-leg benchmark protocol

Status: proposed protocol; no Symtropy robot or controller results are claimed.

Research parent: https://github.com/Luminous-Dynamics/symtropy/issues/4083

## 1. Why this comparator

Maoudj et al., “SPINDER: an open-source 18-DoF hexapod robot with hierarchical central pattern generator control and analytic inverse kinematics,” Scientific Reports 16, 26147 (2026), DOI https://doi.org/10.1038/s41598-026-56856-0.

SPINDER is a highly relevant reproducible six-leg comparator:
- 18-DoF, 0.8 kg open hexapod;
- coupled-oscillator CPG with local trajectory generation and analytic IK;
- per-foot binary contact switches, six-axis IMU and ToF depth camera;
- tripod, ripple and wave gaits with online transitions settling in about two gait cycles;
- specific 2 m structured uneven-foam course with up to 18 mm height variation, reported at 100% completion and mean body-pitch deviation below 5 degrees.

Those figures describe the authors' reported platform and test protocol, not a universal reliability bound and not a Symtropy result. The reported 100% course completion is not a zero-risk guarantee; the denominator for that specific course result should not be inferred from the separate flat-floor trial counts.

The paper's steady-state flat-terrain operating points are useful matched baselines (ten 2 m trials per gait): tripod 0.14 ± 0.01 m/s at 1.7 ± 0.1 Hz with duty factor 0.52 ± 0.02 and static margin 8.3 ± 1.2 mm; ripple 0.10 ± 0.01 m/s at 1.3 ± 0.1 Hz with duty factor 0.62 ± 0.03 and margin 14.6 ± 1.8 mm; wave 0.06 ± 0.01 m/s at 0.9 ± 0.1 Hz with duty factor 0.78 ± 0.02 and margin 22.1 ± 2.3 mm. Reproduce or explicitly normalize these conditions before comparing transition speed or energy across different morphologies.

**Important assurance distinction:** the paper reports a mean tripod support-polygon margin of 20.8 mm across a gait cycle, but also brief negative static margins near tripod exchange. The authors interpret these brief intervals as dynamically stabilized rather than sustained loss of balance. This is an unusually useful demonstration that static support-polygon inclusion is not a complete dynamic locomotion certificate. Our benchmark must report static margin and phase-conditioned dynamic recoverability separately; it must not declare every negative instantaneous static margin a physical fall, nor use a positive margin as proof of future viability.

For timing comparisons, SPINDER reports a 50 Hz CPG+IK control loop, 200 Hz master-to-servo position updates, and an approximately 3.6 ms full 18-servo request/response bus cycle at 1 Mbps (about 250 Hz practical upper update rate in the reported arrangement). These are platform-specific measured constraints. Use them as a reference timing envelope, not as universal legged-robot deadlines.

The paper also supplies valuable falsification cases:
- foot switches were reliable on hard/semi-rigid terrain but could report false negatives on soft/compliant surfaces;
- the ToF depth stream is logged/synchronized but was not yet integrated into online foothold planning or gait adaptation;
- MuJoCo ground friction was tuned to μ = 0.8 for the lab vinyl floor; default μ = 1.0 caused noticeable real-floor slip because the model assumed too much traction;
- real servos exhibit approximately ±0.5° position deadband versus the idealized PD actuator model, and the authors increased swing height by 2 mm to compensate;
- compliant printed links deflected approximately 0.3–0.5 mm at the femur tip, with 1–2 mm resulting foot-position mismatch; higher infill reduced deflection;
- servo backlash and compliant links caused stance-exchange mismatch between model and hardware;
- sustained gait produced thermal loading and torque derating after approximately 15–20 minutes in their setup (hip-yaw servos around 55°C);
- closing the ToF-camera loop into terrain-aware foot placement and contact-loss/fault-tolerant gait reorganization were identified as future work.

These are unusually direct opportunities for testing belief health, abstraction soundness, fault recovery and viability preservation.

## 2. Standards and comparator ladder

Industry performance reporting now has a current standard baseline: **ISO 18646-5:2026** (July 2026). Its public preview identifies rated speed, stopping distance, maximum climbing height, stair climbing, maximum slope, dragging force, crossing capacity, minimum pass-through height and minimum travel width as performance classes. The standard explicitly does not validate safety requirements; use it for matched locomotion-performance measurement, then publish Symtropy's viability and runtime-assurance evidence separately. The detailed crosswalk and conditions are in [`ISO18646-5-2026-CROSSWALK.md`](ISO18646-5-2026-CROSSWALK.md). Do not claim conformance from this summary alone.

## 2. Comparator ladder

Run every controller on the same six-leg plant, actuator model, sensor availability, terrain realization and disturbance seed.

### Additional current baselines: dynamic margins and learning-frontier methods

**HORIZON — recoverability-governed training curriculum.** Bai et al., arXiv:2606.05143v1 (3 June 2026), already use a checkpointed and rollback-capable curriculum frontier. It groups domain randomization into actuation, mass, disturbance, contact, inertia, centre-of-mass and initial-state axes. Its empirical phase gate uses rollout length, tracking error and checkpoint-policy preservation. The preprint reports diagnostics on 1,509 phases across 65 runs; the composite gate's correlation with later frontier gain is 0.52 versus 0.35 for episode fraction alone. In its five-seed fixed-OOD table, its chosen four-domain core reports 36.8% success on a deliberately stacked seven-domain stress suite versus 28.1% for full-domain training; it also reports Go2 hardware interventions. Crucially, the authors explicitly describe the gate as a *finite-horizon training signal, not an exact viability kernel*. This is a baseline for the empirical learning frontier, not a physical safety certificate and not a concept we can claim to have invented. Reproduce its length/tracking/checkpoint gates under matched compute, seeds, robot and OOD suites; then add a separately sourced physical viability gate. https://arxiv.org/abs/2606.05143

**3D force-tilt and moment-tilt stability margins.** Ashtekar and Dutta (Mechanism and Machine Theory, 15 October 2026) develop dynamic-geometric margins that include external force/moment disturbances and changing centroidal angular momentum. These should be compared with planar support-polygon/ZMP margins, not treated as interchangeable with a robust hybrid viability calculation. https://doi.org/10.1016/j.mechmachtheory.2026.106551

**STEP-CBF+.** Djeffal et al. (Scientific Reports, September 2026) combine predicted support, friction/torque feasibility and discrete barrier screening. Their reduced-order randomized simulation reports a 4.0% mean fall rate, 77.3% stable-step fraction and approximately 13.6 ms candidate selection across six scenarios. The paper explicitly does not establish a whole-body nonlinear invariance proof or hardware validation. Reproduce under matched reduced-order assumptions, and preserve this evidence boundary. https://www.nature.com/articles/s41598-026-72507-w

**Contact-reliability-aware state estimation.** Zhao et al. (Sensors 26(19), 6041; published 23 September 2026) provide a direct modern baseline beyond binary foot switches. Their framework retains a physics-based Kalman recursion, estimates gait-phase-constrained continuous contact reliability, uses it to regulate contact-related covariance, and separately adds dynamic and observation residual corrections. Their paper reports 61.0% / 56.2% base-position RMSE reductions, 32.2% / 31.2% base-velocity reductions and 59.1% / 54.7% average foot-position reductions versus Binary Contact KF on its flat / mildly uneven simulation cases. Its reported mean full-pipeline latency is 26.46 ms, maximum 36.24 ms, with approximately 50 ms update period; physical results cover indoor walking and turning, not complex terrain. These are paper-specific results, not general performance guarantees. Compare binary KF, continuous-contact KF, the full neural residual KF and D1/D2/D3 ablations under matched data and update periods. Add calibration, out-of-domain, inter-source dependence and deadline tests; do not equate lower RMSE with contact-safety assurance. https://doi.org/10.3390/s26196041

**Episode-level conformal calibration of estimator uncertainty.** Compare nominal covariance, bi-level covariance/kinematics calibration, IUCA-style correlated-state calibration and split/learnable conformal approaches. The Symtropy V0 protocol treats a whole independent session/trial block as one exchangeability unit and calibrates a maximum error score across the declared trajectory horizon, all feet and state dimensions. It reports the exact finite-sample order statistic and returns an unbounded threshold when the sample count cannot attain the requested coverage. Calibration is scoped to exact model, target, score-definition and operating-domain digests; a new/OOD domain invalidates reuse. This is a statistical marginal-coverage diagnostic, not a physical safety guarantee. See [#4146](https://github.com/Luminous-Dynamics/symtropy/issues/4146) and [PR #4147](https://github.com/Luminous-Dynamics/symtropy/pull/4147).

Calibration comparators:
- Cheng et al. (ICRA 2026), simultaneous covariance and kinematics calibration: https://www.leggedai.com/publication/2026icra_calibration/
- IUCA (IEEE Journal of Selected Areas in Sensors, 2026): https://doi.org/10.1109/JSAS.2026.3731931
- Learnable Conformal Prediction (ICRA 2026): https://doi.org/10.1109/ICRA57385.2026.11697249
- Online Conformal Prediction via Online Optimization (ICML 2025): https://proceedings.mlr.press/v267/areces25a.html

**Contact-probability calibration metrics.** The separate [#4148 / PR #4159](https://github.com/Luminous-Dynamics/symtropy/pull/4159) reference computes exact fixed-point Brier, ECE and MCE for one declared channel/target/model/domain/split stratum at a time. Report every reliability bin, including empty bins, and distinguish correlated observation count from independent session/trial count. This is a diagnostic utility, not an estimator or safety guarantee.

**Action-conditioned contact information.** [LOCO-CONTACT-INFO-001 / #4004](https://github.com/Luminous-Dynamics/symtropy/issues/4004) composes with the scalar source-qualified contact-information reference [#4008](https://github.com/Luminous-Dynamics/symtropy/pull/4008) and the joint correlated Gaussian fusion reference [#4230](https://github.com/Luminous-Dynamics/symtropy/pull/4230). V0 and V1 distinguish the scalar conditional-independence path from the full cross-channel covariance path. The joint path uses a calibrated R matrix, Cholesky solves and a log-determinant EIG; it rejects asynchronously acquired samples instead of silently fusing them as simultaneous. Neither path validates sensor covariance from raw hardware data or certifies probe safety.

The benchmark must therefore distinguish at least four constructs: static support margin, dynamic geometric margin, robust future physical viability, and empirical learning recoverability. None is a synonym for another.

1. **B0: SPINDER-style CPG/IK baseline** — coupled oscillators, local trajectory generation, analytic IK, fixed or velocity-conditioned gait schedule.
2. **B1: CPG + rule fallback** — baseline plus handcrafted contact-loss and actuator-fault fallback; no formal viability oracle.
3. **B2: predictive safety baseline** — a matched CBF/QP or robust-MPC implementation with the same state estimate, constraints and compute budget.
4. **B3: learned baseline** — a declared DRL/CPG or residual-policy controller, with its training budget and observation set frozen.
5. **S0: Symtropy finite-oracle-only** — consume a supplied finite transition graph; explicitly synthetic/abstract evidence only.
6. **S1: Symtropy hybrid assurance** — sound interval/set-valued abstraction feeding the finite robust viability oracle.
7. **S2: Symtropy dual-channel** — S1 plus belief/provenance and action-conditioned contact information (#4004/#4008/#4230); any probing policy must first pass an independent robust physical-admissibility filter.
8. **E0: Binary Contact KF** — binary contact validity baseline.
9. **E1: Continuous Contact KF** — continuous reliability/covariance-regulation ablation.
10. **E2: Contact-reliability-aware neural residual KF** — Zhao et al. full baseline, including separate dynamic and observation residuals.
11. **E3: Symtropy graded contact evidence** — source-provenance and dependence-safe interval envelope across D1/D2/D3, used only as a qualified evidence product (#4143/#4144).
12. **E4: contact-probability calibration metrics** — exact Brier/ECE/MCE and reliability bins per fixed stratum, with session/trial clustering (#4148/#4159); evaluation utility, not an estimator.
13. **E5: episode-level conformal uncertainty calibration** — finite-sample whole-unit marginal coverage under the declared exchangeability assumption and exact operating domain (#4146/#4147); not a physical safety claim.
14. **C1: HORIZON-style learning curriculum** — reproduce its episode-length, tracking and checkpoint-preservation gate without formal physical viability certification.
15. **S3: Symtropy dual-frontier curriculum** — empirical learning gate plus independently sourced physical viability and certificate-validity gates; hardware progression requires all gates, while out-of-envelope exploration remains simulation-only.
16. **E6: joint correlated contact fusion** — calibrated full cross-channel R, exact channel-order manifest and synchronized multivariate update (#4017/#4230). The diagonal-R path must match the scalar reference; deliberately treating correlated channels as independent is a required overconfidence negative control.
17. **P1: calibrated one-step chance gate** — five marginal constraints with explicit Bonferroni alpha, independent session/episode tail validation and abstention on underpowered or out-of-domain calibration (#4010/#4231). The EIG ranking is applied only after every constraint passes.
18. **P2: finite-horizon risk ledger** — immutable allocation over all 5H events, action admission/consumption receipts, and non-recyclable skipped/unknown allocations (#4232/#4245). Compare against the invalid negative control that reuses the same one-step alpha on every footfall.
19. **S4: composed dual-assurance candidate** — S1 robust set-based viability + E6 correlated evidence + P1 one-step bounds using P2's exact per-step/per-constraint allocation + certificate lifecycle. A probability pass cannot override a robust rejection; Unknown on either authority path remains explicit.
20. **A1: actuator/energy transfer baseline** — compare a constant-efficiency model, calibrated static loss map (#4179/#4182), and PACE-style bottom-up dynamic actuator identification. PACE's reported 32% ANYmal full-CoT reduction is a different platform result; reproduce its methodology and energy boundary rather than comparing that number directly with SPINDER.

Do not compare a simulated S1/S2 result directly against hardware B0/B1 as if they shared evidence maturity. Publish simulator and hardware scorecards separately.

For curriculum comparisons, match policy architecture, training compute, observation interface, seed count and fixed-OOD scenarios when reproducing HORIZON. Report its empirical checkpoint gate separately from the physical viability oracle. No rollout-survival statistic may promote an Unknown physical model to Certified.

### 2.1 Assurance-policy and horizon-risk ablations

Run the following policy/assurance variants under the same plant, observations, action candidates, operating domain and compute limits:

- **R-only:** robust set-valued viability/recovery gate, with no probabilistic risk channel.
- **P-only:** calibrated five-constraint one-step chance gate, with no claim of robust future invariance. This is an analysis arm, not a hardware-authorized fallback.
- **R+P:** robust viability and one-step calibrated chance gate both pass; a pass on one channel cannot compensate for a rejection/Unknown on the other.
- **R+P+H:** R+P with the immutable finite-horizon allocation from #4245. The exact five per-constraint ppb amounts for each step must pass into #4231 unchanged; do not collapse them to a step sum and re-divide. Compare uniform and predeclared nonuniform profiles at matched total alpha.
- **Uniform horizon profile:** equal alpha across all (step, constraint) events, with deterministic ppb remainder placement.
- **Predeclared nonuniform profile:** phase-/constraint-weighted vector from `HorizonRiskPlan::from_preallocated_allocations`, frozen before the episode and with exactly the same total alpha as the uniform arm. Candidate profiles may allocate more to touchdown, support exchange, slip recovery or high-load actuator constraints only when that profile is declared before the relevant outcomes.
- **Post-outcome reallocation negative control:** increase or transfer alpha after observing an adverse event. This must be rejected/labelled invalid because the risk plan is immutable and the horizon budget cannot be replenished after outcomes.
- **Reused-alpha negative control:** reapply the same one-step family-wise alpha at every step. It must be labelled invalid for the declared horizon guarantee and included only to demonstrate cumulative-risk inflation.
- **EIG-only negative control:** rank by expected information gain without an admissibility filter. It must never be included in a deployment-eligible set.
- **Diagonal-covariance negative control:** treat deliberately correlated D1/D2/D3 channels as independent. Measure overconfidence against the joint-R reference.
- **One-step allocation negative control:** recompute an equal five-way split from each step's alpha sum when the horizon ledger has an uneven ppb remainder. It must fail exact allocation-identity checks.

Report robust and probabilistic outputs separately. The horizon union bound is conditional on each event's marginal tail validity under the declared filtration; a good finite allocator cannot rescue a miscalibrated predictive model. A finite-graph viability pass cannot authenticate the physical graph abstraction.

### 2.2 External comparator matching

The comparison must be layered, not a leaderboard mash-up:

- **Locomotion/controller:** SPINDER CPG, SPINDER hybrid/rule feedback, matched CBF/QP, robust MPC and a version-pinned learned policy on the same plant.
- **Contact/state estimator:** Binary Contact KF, continuous reliability KF, Zhao et al.'s full residual KF and regime-specific MoE odometry.
- **Safety/foothold screening:** reproduce STEP-CBF+ under its reduced-order model assumptions; separately test robust MPC and set-valued viability. Do not report a simulation-only STEP-CBF+ result as hardware safety evidence.
- **Sim-to-real/energy:** PACE-style bottom-up actuator identification and the constant-efficiency baseline, followed by actuator-map transfer tests. Report electrical battery energy, actuator electrical input, mechanical positive/negative work and auxiliary loads as separate boundaries.
- **Standards:** use ISO 18646-5:2026 for matched locomotion-performance measures. Its scope explicitly does not verify or validate safety requirements; report the independent viability/runtime-assurance evidence separately. Do not claim conformance from a subset of tests.

The PACE paper reports a 32% full Cost-of-Transport reduction for ANYmal (reported CoT 1.27) and transfer to ten additional robots using a physics-grounded actuator/energy model. This is a strong methodology baseline, not a directly comparable SPINDER or FlyGym score. Reproduce the system boundary and identification protocol before interpreting relative energy. https://doi.org/10.1177/02783649261459628

## 3. Frozen scenario matrix

### A. Nominal and gait transitions
- level floor at matched speed/turning trajectories;
- tripod/ripple/wave switching at multiple oscillator phases;
- speed-command ramps with different ramp rates and dwell times;
- uneven but rigid terrain, including the SPINDER-like 18 mm class.

### B. Contact and terrain faults
- unexpected contact loss during stance;
- touchdown slip and partial support;
- soft/compliant surface with contact-switch false negatives;
- missed, delayed, stuck-high and stuck-low contact switch;
- contact mode ambiguous across two nearly simultaneous footfalls;
- terrain height or friction outside the nominal calibration envelope.

### C. Actuation and timing faults
- servo backlash/deadband and compliance;
- torque saturation and bounded tracking error;
- actuator slowdown/derating after thermal loading;
- delayed command delivery and asynchronous sensor timestamps;
- missed control deadlines and stale state estimates.

### D. Information-seeking tests
- compare passive sensing against candidate safe contact probes;
- informative probe with an admissible recovery option;
- informative probe that is physically inadmissible and must be filtered;
- uninformative or contradictory observation;
- probe that narrows epistemic terrain uncertainty while environmental disturbance bounds remain unchanged.

Every injected condition needs a documented ground-truth label, injection mechanism, severity, random seed and independent outcome evaluator.

## 4. Primary qualification scorecard

Keep the quantities separate:

### Safety and viability
- false robust-safe action count;
- robust viable-state coverage under a declared abstraction;
- fraction of rejected actions with concrete violating-successor witnesses;
- action-availability count and recovery-option count per state;
- contact-loss survival rate and worst-case recovery horizon;
- unknown/incomplete/abstain rate with reason.

### Finite-horizon risk accounting
- declared horizon H and total family-wise alpha;
- exact alpha[t,j] for every step and physical constraint;
- cumulative allocated, committed/reserved, consumed, skipped-unused, unknown-locked and pending alpha;
- horizon-level violations and per-event marginal violations;
- admission, execution, unknown and reconciliation receipt counts;
- zero allocation, replay, mismatch, stale and unresolved execution counts;
- rate of candidate rejection/abstention by reason;
- exact consistency of the one-step gate allocation with the horizon ledger.

### Locomotion quality
- course completion and time;
- gait transition settling cycles and phase error;
- body pitch/roll excursions;
- foot placement/stance errors;
- slip distance and support-loss duration;
- failed, incomplete and aborted episodes.

### Sensing and model quality
- contact event precision/recall and mode confusion matrix;
- continuous contact-reliability calibration: Brier score, log loss, reliability diagrams and coverage-versus-risk;
- whole-episode conformal coverage at declared alpha, empirical coverage confidence intervals, threshold width/volume and independent calibration-unit count;
- fit/calibration/test leakage, exchangeability-unit grouping and OOD-domain abstention;
- per-channel and cross-channel disagreement, timestamp skew and abstention;
- detection latency for stale, missing, stuck and saturated signals;
- prediction calibration by surface and individual;
- event-order branch coverage;
- interval/tube enclosure soundness and inflation;
- out-of-distribution detection and abstention.

### Physical and computational cost
- battery-terminal discharge, returned and net energy;
- mechanical positive and negative work separately;
- compute and sensing energy where measurable;
- temperature and observed actuator derating;
- worst-case end-to-end decision latency;
- deadline misses, peak memory and transition count.

Do not compare mechanical-only simulation work to hardware battery energy. Do not combine safety, energy, speed and task reward into one weighted score.

## 5. Statistical protocol

- Freeze train/calibration/test splits by individual, terrain family and session. Do not randomly split adjacent time windows from the same trial across train and test.
- Freeze scenario definitions and primary metrics before comparing controllers.
- Use paired trial seeds and matched initial states/commands for simulation comparisons.
- Report numerator, denominator, failure definition and confidence interval for every success/failure rate.
- A zero-failure sample is not a zero-risk guarantee. Report an upper confidence bound and sample size. For independent Bernoulli trials, the exact one-sided 95% Clopper–Pearson upper bound after zero observed events is (1 - 0.05^{1/n}): about 1% still requires 299 independent trials, 0.1% requires 2,995, and 0.01% requires 29,956. These counts are illustrative only; repeated ticks and correlated episodes from one session do not count as independent trials. State the exchangeability/independence unit and use clustered or session-level inference when appropriate.
- Keep two budgets distinct: **operational violation risk** (alpha) and **statistical confidence in the estimated marginal tail bounds** (delta). The current #4231 V0 Clopper–Pearson check is pointwise 95% confidence per constraint. It does not establish simultaneous 95% confidence across the five constraints or the full horizon.
- For zero observed tail events, the exact one-sided upper bound is (1 - delta_event^(1/n)). At pointwise 95% confidence (delta_event=0.05), a 1% marginal risk bound needs 299 independent units and a 0.1% bound needs 2,995. These are **pointwise** examples.
- For simultaneous 95% statistical confidence across five one-step constraints, allocate delta=5%/5=1% per constraint. A 1% operational tail bound then needs 459 independent zero-exceedance units per constraint. For a ten-step horizon with 5% operational alpha and 5% confidence delta each allocated over 50 events, each event receives 0.1% operational alpha and 0.1% confidence delta; zero exceedances require 6,905 independent units per declared event stratum. Pooling across strata/events requires a preregistered exchangeability/conditional-validity argument.
- Do not count correlated ticks or adjacent windows from one rollout as independent units. If the sample-size target is not met, report the pointwise interval/abstention honestly; do not claim simultaneous or horizon-wide confidence.
- Report horizon-level operational family-wise bounds separately from statistical confidence in the marginal models. A one-step 5% guarantee reused on each footfall is not a 5% episode guarantee.
- Report median, 5th percentile, 95th percentile and worst observed recovery times, not only means.
- Stratify results by gait, phase at disturbance, contact mode, terrain compliance, individual, battery state and thermal state.
- Preserve raw traces, controller/config digests, actuator/sensor calibrations and trial receipts.

## 6. Proposed gates

These are targets for qualification, not achieved results:

- V0: 100% agreement with independent exhaustive oracle on frozen small transition systems.
- V1: 100% enclosure containment on exact-rational reference fixtures; zero omitted guard branches in the frozen event-order corpus.
- V2: zero false robust-safe outcomes in the frozen adversarial simulator corpus; every incomplete abstraction must return Unknown.
- V2: demonstrate nonzero useful viable-state coverage and publish coverage-versus-inflation curves. A vacuous tiny safe set does not qualify as practical success.
- V3: on hardware, show statistically supported recovery improvement against B0/B1/B2 under matched faults while preserving hard limits; pre-register effect size and sample size after baseline variance is measured.
- V3: no hardware deployment certificate from simulation alone; validate sensor health, timing, actuator limits, contact bounds and physical model discrepancy.
- V2: no one-step alpha reuse across a horizon; the sum of all preallocated marginal ppb values must equal the declared horizon budget exactly, with no zero-alpha event and no post-outcome recycling.
- V2: zero allocation-identity mismatches between horizon ledger and one-step gate; Unknown execution locks its allocation and blocks progression until receipt-backed reconciliation.
- V2: the chance-gate calibration receipt must meet the exact per-constraint sample/coverage threshold; an underpowered, heavy-tailed, miscalibrated or OOD prediction abstains.
- V3: publish the robustness/energy/task-performance Pareto frontier under matched hardware and a preregistered independent-session validation plan; no single weighted reward substitutes for the separate scorecards.

Do not set a universal millisecond or recovery-success threshold before measuring the actual platform's control period, actuator response and baseline variation.

## 7. Research hypotheses

1. **Viability margin predicts transitions:** phase-conditioned viable-set distance, robust action count and worst-case recovery horizon predict necessary gait changes earlier than speed-only thresholds.
2. **Sensor-aware viability beats point estimates:** representing ambiguous contact outcomes as branches reduces false-safe decisions under soft terrain and stale switches.
3. **Active probing expands useful knowledge, not physical safety bounds:** safe probes can shrink epistemic terrain uncertainty and enlarge the set of useful decisions, while disturbance bounds remain separate.
4. **Individual-aware tubes improve transfer:** per-robot actuator/damping/coupling uncertainty improves held-out recovery calibration compared with population-average dynamics.
5. **Option-preserving recovery improves tail behavior:** controllers that preserve multiple admissible recovery actions may reduce worst-tail failures even when average speed is unchanged.

Each hypothesis needs at least one negative control and a preregistered falsification criterion.

## 8. Publication-quality evidence package

For probabilistic/horizon claims also bind the risk-plan identity, total alpha, exact per-step/per-constraint allocation vector, independent validation-unit definition, held-out tail counts, model/calibration/domain digests, filtration/conditioning contract, and every execution/Unknown/reconciliation receipt. The evaluator must be able to recompute the horizon union-bound total from raw allocation records.

For every reported result bind:
- source commit and code/configuration digest;
- model/abstraction identity and coverage statement;
- hardware identity, morphology and mass configuration;
- terrain and disturbance identity;
- sensor/actuator calibration and clock mapping;
- raw trial traces and outcome labels;
- verifier and evaluator version;
- inclusion/exclusion and abort reasons;
- exact statistical procedure and confidence interval.

## 9. References

- Maoudj et al. (2026), SPINDER, Scientific Reports 16, 26147: https://doi.org/10.1038/s41598-026-56856-0
- Jia et al. (2026), contact sensing survey: https://doi.org/10.1007/s10462-026-11644-0
- Impact-Aware Robust Convex MPC (IEEE RA-L, 2026): https://doi.org/10.1109/LRA.2026.3692086
- Madabushi et al. (2026), hybrid invariant tubes: https://arxiv.org/abs/2604.05108
