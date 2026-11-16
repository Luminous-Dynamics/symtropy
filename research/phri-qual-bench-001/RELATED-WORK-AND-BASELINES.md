# PHRI related work, comparator map, and advancement criteria

**Review date:** 2026-11-12  
**Scope:** physical human–robot interaction (pHRI), variable impedance/admittance, tactile coupling, contact-aware benchmarking, shared autonomy, and evidence qualification.

This review is a protocol aid, not a systematic review and not a claim that the listed works exhaust the field. Comparator arms must be pinned to exact paper revisions, code, model assumptions, and reproducible configurations before numerical comparison.

## 1. What the field already does well

| Area | Relevant public baseline | Demonstrated scope | Consequence for this programme |
|---|---|---|---|
| Passivity-aware variable impedance | Dinc et al., arXiv:2611.10778 (2026) | Diagonal inertia/stiffness-rate QP; configuration-dependent compliance extension using Cholesky/eigendecomposition; Kinova Gen3 experiments with human perturbations; comparison to energy-tank and non-passivity-based methods | The diagonal VIC QP and generalized-eigenbasis compliance path are mandatory comparators. Do not claim that adaptive inertia compensation or passivity-constrained rate optimization is new. |
| Matrix-valued stiffness and shared control | Mühlbauer et al., arXiv:2604.20557 (2026) | Time-varying matrix-valued stiffness and matrix-valued wrench arbitration, including off-diagonal terms; passivation methods; simulation and real-robot experiments | Full-matrix stiffness and passive shared control are already established. Our narrower basis-free rate optimizer must be evaluated for numerical behavior, expressivity, conditioning, latency, and matched-subspace regret—not marketed as the first full-matrix passive controller. |
| Passivity-aware MPC admittance | ICRA 2026 MPVA, DOI 10.1109/ICRA57385.2026.11695860 | Online MPC variable admittance integrating tracking, force directionality, and passivity constraints; 7-DoF Kinova Jaco-2; assistive/resistive tasks with multiple subjects; comparison to fixed-gain and fuzzy adaptive baselines | Include a predictive-controller arm. Compare both control quality and executed-port energy, under the same plant, periods, force limits, and task conditions. |
| Sensorless force estimation and variable admittance | Wan et al., *Robotics* 15(5), 84 (2026), DOI 10.3390/robotics15050084 | Uncertainty-aware external-force estimation and energy-based variable admittance; experiments on a redundant robot; reported force/torque estimation errors below 3 N and 3 N·m in the study setup | Include a disturbance/force-estimation baseline where the sensor architecture is applicable. Its reported errors are study-specific and not directly transferable to other robots or tasks. |
| Continuous tactile coupling | *Tactile physical human–robot interaction: a continuous coupling paradigm for walking assistance*, National Science Review (published 2026-07-01), DOI 10.1093/nsr/nwag402 | Large-area torso tactile coupling for walking assistance; integrates whole-body state estimation, predictive contact planning, tactile modulation and constrained execution at multiple rates | Continuous tactile coupling is an existing research direction. Our differentiation must be independent qualification of contact, energy, authority, and user agency—not the claim that continuous tactile interaction itself is novel. |
| Tactile sensor transfer | TacVerse (2026), DOI 10.1002/adrr.70169 | 106,800 images from seven vision-based tactile sensors; shape, grating and force tasks; within-sensor, zero-shot cross-sensor and few-shot adaptation protocols; direct transfer degrades, with few-shot adaptation only partly closing the gap | Sensor identity, manufacturing lot, calibration revision, temperature and aging must be held out explicitly. TacVerse is directly applicable to VBTS; its results should not be conflated with resistive or capacitive pressure arrays. |
| Matrix-valued passivity indices | Ru et al., arXiv:2601.04796 (2026) | Matrix-valued passivity indices and Loewner-order analysis for MIMO LTI systems | Directional passivity indices are not a new general concept. Keep our comparison strictly within the stated LTI assumptions unless a separate nonlinear sampled-data theory is established. |
| Contact-aware assistive benchmark | He et al., arXiv:2609.02402 (2026) | Deformable, passively responding simulated human calibrated against force–indentation measurements on a care manikin; frozen policy-visible/scorer-only protocol; 140 runs per method; task success separated from physical-contact screening | This is a crucial benchmark design comparator. Adopt calibrated physical response, scorer isolation, contact-quality metrics, and both raw task success and physically screened success. |
| Agency-aware shared autonomy | Zhang et al., ACSA, first published 2026-09-08, DOI 10.1177/10711813261485183 | Offline replay of 3,814 prosthetic grasp episodes; an Agency Loss Index (ALI); set-based autonomy envelope; explicit performance–agency frontier | Measure agency as a first-class outcome. Explicit authorization is necessary but not a substitute for measuring override, disagreement, predictability, workload, and user experience. |
| HRI measurement science | NIST Performance of Human-Robot Interaction programme | Datasets, benchmarking tools, test methods, protocols, metrics, standards, traceable human localization/tracking, trust and safety measurement | Align the evidence model and reporting protocol with metrology practice. Do not claim that reproducible HRI test methods or safety measurement are absent in the industry. |

### Primary references

- Dinc et al., *Passivity-Guaranteed Variable Impedance and Compliance Control via Online Parameter Optimization*: https://arxiv.org/abs/2611.10778
- Mühlbauer et al., *Passive Variable Impedance For Shared Control*: https://arxiv.org/abs/2604.20557
- ICRA 2026, *Adaptive Physical Human–Robot Interaction via a Passivity-Aware Model Predictive Variable Admittance Control*: https://doi.org/10.1109/ICRA57385.2026.11695860
- Wan et al., *Human–Robot Interaction: External Force Estimation and Variable Admittance Control Incorporating Passivity*: https://doi.org/10.3390/robotics15050084
- *Tactile physical human–robot interaction: a continuous coupling paradigm for walking assistance*: https://doi.org/10.1093/nsr/nwag402
- Wei et al., *TacVerse: A Multisensor Dataset and Benchmark for Cross-Sensor Vision-Based Tactile Perception*: https://doi.org/10.1002/adrr.70169
- Ru et al., *Matrix-Valued Passivity Indices: Foundations, Properties, and Stability Implications*: https://arxiv.org/abs/2601.04796
- He et al., *A Physics-Consistent Benchmark for Contact-Rich Human-Robot Interaction in Assistive Care*: https://arxiv.org/abs/2609.02402
- Zhang et al., *Agency-Constrained Shared Autonomy for Prosthetic Grasp Selection*: https://doi.org/10.1177/10711813261485183
- NIST, *Performance of Human-Robot Interaction*: https://www.nist.gov/programs-projects/performance-human-robot-interaction

## 2. Honest position of the current Luminous work

The current portfolio is a set of software and protocol references, not a validated pHRI controller or product.

| Component | Current contribution | Not established yet |
|---|---|---|
| Diagonal VIC passivity reference (#4738) | Reference QP and passivity checks; latest exact-head compile-name correction committed | Current-head compile/test/Clippy result; robot replication; discrete executed-port passivity |
| Continuous tactile coupling (#4743) | Fixed-grid continuous tactile field reference; dedicated workflow was green on its recorded exact head | Cross-sensor transfer, calibration, real tactile hardware, biomechanical validity, human agency outcomes |
| Basis-free optimizer (#4745) | Fixed-size 3×3 symmetric stiffness-rate projection using a whitened metric and pointwise spectral projection; scalar inertia scale | Full matrix inertia, higher-dimensional task-space validation, validated floating-point bounds, physical integration, matched SOTA comparison |
| Discrete energy observer (#4778) | Sampled storage/work residual, historical prefix-deficit accounting and evidence identity | Conservative work-bound derivation, actual actuator-port measurement, multi-port completeness and instrumented validation |
| Multirate proposal gate (#4787) | Expiry, supersession, state/clock/agency binding and pre-application revalidation | Scheduler timing, concurrency, hard real-time behavior, independent hardware stop |
| Interaction envelope (#4808) | Conjunctive energy/contact/agency/freshness/command/port-graph/actuator-authority software admission; 30,000-case deterministic regression test authored | Exact-head execution, authenticated authority evidence, hardware interlocks, calibrated evidence producers, physical and human validation |
| Qualification protocol (#4755) | 34 scenario families, 14 comparator/negative-control arms, receipt schema and analytic fixtures | Executed cases, run receipts, simulator/manikin agreement, HIL, human-subject results |

All status must be reconciled to exact PR head SHAs and completed workflow conclusions before changing these labels.

## 3. The strongest defensible paradigm shift

The objective should not be “one controller with a bigger optimizer.” The strongest research thesis is:

> **Evidence-qualified interaction control:** every adaptation is a time-bounded proposal; every execution candidate is tied to a frozen interaction identity; independent physical, energy, contact, wearer-authority and actuator-authority evidence is reconciled before application; the result is replayable and independently scoreable.

This reframes pHRI qualification around a *closed evidence loop*:

1. **Estimate:** produce task, tactile, state and uncertainty observations.
2. **Propose:** higher-rate planners/optimizers produce a candidate with source state, profile, command, sequence and expiry.
3. **Qualify:** energy, local contact, agency, timing, port-boundary and actuator-authority domains are checked independently.
4. **Admit or abstain:** conjunction only; unknown, stale, contradictory or missing evidence blocks admission.
5. **Apply:** a one-use receipt is revalidated at the caller's application boundary; the independent physical stop remains outside this software reference.
6. **Observe:** measure actual applied outputs and physical responses, not desired commands alone.
7. **Audit:** preserve raw traces, calibration/profile revisions, exact source/binary identities, exclusions and scorer-only labels.

This is a system architecture and benchmark thesis. It is not a theorem that the resulting robot is safe.

## 4. Benchmark targets that would materially advance the field

Targets below are research objectives, not achieved results or universal safety thresholds.

### A. Compositional software conformance

- Zero false admissions in the frozen finite adversarial matrix and the deterministic 30,000-case generated fault campaign.
- 100% of admitted bundles have exact agreement on interaction sequence, proposal sequence, clock/boot, agency epoch, state revision, profile, command and current port-graph digest.
- Zero application permits from stale, expired, superseded, revoked, replayed or digest-substituted receipts.
- Report false rejection and abstention separately. Never improve apparent admission performance by weakening a gate.
- Keep a weighted-score negative control and an identity-unbound negative control; show the specific failure each permits.

The 30,000 generated cases are a regression/conformance campaign, not an estimate of real-world failure probability. The generator is structured, cases are not IID samples from the field, and zero failures in this test must not be converted into a population-risk claim.

### B. Computation and timing

- Measure p50/p95/p99/maximum admission and revalidation latency on every declared target computer, with compiler, clock source, thermal state, CPU load and control period in the receipt.
- Aspirational target: p99 gate latency below 10% of the fast-loop period on the target platform. Treat this as a goal to test, not an assumed real-time guarantee.
- Zero blocking/waiting of the fast loop on optimizer or MPC completion in every declared contention test.
- Report deadline misses, fallback behavior, queue age, sensor age, proposal age and thermal degradation separately.
- Measure physical stop latency with an independent instrumented stop path; do not infer it from software gate latency.

### C. Optimizer superiority under matched assumptions

For the common subspace, compare the Luminous basis-free 3×3 stiffness-rate method directly against the Dinc generalized-eigenbasis compliance extension and a high-accuracy convex reference solution. Hold plant, state, bounds, task, update period, metric, numerical precision and compute budget constant.

Report:
- objective regret relative to the reference optimum;
- maximum primal feasibility violation, stationarity, complementarity and reconstruction residual;
- false-feasible rate and explicit abstention/infeasibility rate;
- sensitivity to repeated/near-repeated eigenvalues, condition number, coordinate rotations and finite precision;
- worst-case and percentile latency, not just mean latency.

In native-expressivity comparisons, disclose parameter dimensions and allowed matrix degrees of freedom. The current optimizer does not support arbitrary full matrix inertia; it cannot claim dominance over a more expressive controller on the basis of speed or fewer variables.

### D. Physics-consistent contact qualification

- Follow the contact-aware benchmark principle: calibrate the deformable/passive response against an independent load-cell and deformation reference, and freeze the policy-visible versus scorer-only observation boundary.
- Report task success and physically screened success separately, including region correctness, peak force/pressure, joint-limit margin, contact stability, contact area, pressure-time, deformation-rate proxy, power and dwell time.
- Target 95% empirical coverage for any declared 95% measurement-uncertainty interval on a held-out non-human phantom calibration/validation set. This is a metrology coverage objective, not a tissue-injury threshold.
- A candidate that succeeds at the task but fails the physical-contact screen is not a valid contact-rich success.

### E. Tactile transfer and calibration

- Split by physical sensor serial and manufacturing lot—not random frames.
- Publish within-unit, cross-unit zero-shot, cross-family zero-shot, few-shot, calibration-revision, aging and temperature-drift results separately.
- Aspirational target: recover at least 90% of the within-unit-to-zero-shot transfer gap using no more than 5% of the target-unit labelled calibration budget, while retaining calibrated uncertainty coverage. This is a challenging research target and must be judged per task and sensor family.
- Report abstention and out-of-distribution detection. A model that cannot support a calibrated force or exposure bound must abstain from producing that bound.

### F. Agency and usefulness

- Report task success, physically screened success, explicit stop/refusal compliance, override rate, intent disagreement, ALI or a justified analogue, workload, predictability, and user-reported comfort independently.
- Compare user-only control, confidence blending, set-based agency-constrained shared autonomy, and the Luminous gate/interaction policy.
- Publish a performance–agency frontier across users and tasks. Do not collapse agency into task success or a single opaque scalar.
- User studies require separate ethics review, informed consent and approved protocols. The software gate does not replace them.

## 5. What would count as “better than SOTA”

A result should only be called better when it beats a matched comparator on a pre-registered metric without violating another independent constraint.

Examples:
- **Optimizer:** lower objective regret or latency at equal/worse-never constraint violation, with repeated-eigenspace and conditioning robustness.
- **Contact benchmark:** higher physically screened task success, not merely higher raw task completion.
- **Tactile transfer:** less cross-unit degradation under held-out serial/lot splits, with uncertainty coverage and lower calibration-data budget.
- **Agency:** a strictly better task-performance/agency frontier, not just higher classification accuracy or more assistance.
- **System assurance:** lower false-admission rate and stronger event/command/port-graph traceability under identical fault injections, with false rejection disclosed.
- **Deployment:** measured repeatability, maintainability, requalification burden and stop-path latency on target hardware.

Do not compare headline metrics from different tasks, robot models, sensors, subjects or safety screens as if they were one leaderboard.

## 6. Immediate priority order

1. Get the current exact heads of #4738, #4745, #4778, #4787 and #4808 through dedicated formatting, compile, test and strict Clippy workflows; diagnose any failure from logs.
2. Freeze the benchmark manifest and add independent scorer / policy-observation separation to every applicable scenario and receipt.
3. Build the high-accuracy optimizer oracle and matched Dinc/Mühlbauer adapter plan.
4. Construct a traceable non-human layered phantom protocol with independent load and deformation references.
5. Add sensor-serial/lot leakage checks and tactile uncertainty-calibration metrics.
6. Only after those steps, consider instrumented hardware and ethically approved user studies.

The current research advantage should be *reproducibility and claim discipline across the full interaction chain*. Do not claim a performance lead until matched experiments demonstrate it.
