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

## 2. Comparator ladder

Run every controller on the same six-leg plant, actuator model, sensor availability, terrain realization and disturbance seed.

1. **B0: SPINDER-style CPG/IK baseline** — coupled oscillators, local trajectory generation, analytic IK, fixed or velocity-conditioned gait schedule.
2. **B1: CPG + rule fallback** — baseline plus handcrafted contact-loss and actuator-fault fallback; no formal viability oracle.
3. **B2: predictive safety baseline** — a matched CBF/QP or robust-MPC implementation with the same state estimate, constraints and compute budget.
4. **B3: learned baseline** — a declared DRL/CPG or residual-policy controller, with its training budget and observation set frozen.
5. **S0: Symtropy finite-oracle-only** — consume a supplied finite transition graph; explicitly synthetic/abstract evidence only.
6. **S1: Symtropy hybrid assurance** — sound interval/set-valued abstraction feeding the finite robust viability oracle.
7. **S2: Symtropy dual-channel** — S1 plus belief/provenance and active contact information, with probing restricted to robustly admitted actions.

Do not compare a simulated S1/S2 result directly against hardware B0/B1 as if they shared evidence maturity. Publish simulator and hardware scorecards separately.

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

### Locomotion quality
- course completion and time;
- gait transition settling cycles and phase error;
- body pitch/roll excursions;
- foot placement/stance errors;
- slip distance and support-loss duration;
- failed, incomplete and aborted episodes.

### Sensing and model quality
- contact event precision/recall and mode confusion matrix;
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
- A zero-failure sample is not a zero-risk guarantee. Report an upper confidence bound and sample size.
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

Do not set a universal millisecond or recovery-success threshold before measuring the actual platform's control period, actuator response and baseline variation.

## 7. Research hypotheses

1. **Viability margin predicts transitions:** phase-conditioned viable-set distance, robust action count and worst-case recovery horizon predict necessary gait changes earlier than speed-only thresholds.
2. **Sensor-aware viability beats point estimates:** representing ambiguous contact outcomes as branches reduces false-safe decisions under soft terrain and stale switches.
3. **Active probing expands useful knowledge, not physical safety bounds:** safe probes can shrink epistemic terrain uncertainty and enlarge the set of useful decisions, while disturbance bounds remain separate.
4. **Individual-aware tubes improve transfer:** per-robot actuator/damping/coupling uncertainty improves held-out recovery calibration compared with population-average dynamics.
5. **Option-preserving recovery improves tail behavior:** controllers that preserve multiple admissible recovery actions may reduce worst-tail failures even when average speed is unchanged.

Each hypothesis needs at least one negative control and a preregistered falsification criterion.

## 8. Publication-quality evidence package

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
