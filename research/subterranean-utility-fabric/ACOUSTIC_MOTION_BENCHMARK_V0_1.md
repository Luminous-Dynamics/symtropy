# Motion-Compatible Acoustic Sensing — Benchmark Protocol v0.1

**Status:** research protocol and synthetic campaign contract only  
**Workstream:** Symtropy #4953  
**Parent localization benchmark:** #4925  
**Cable-duct profile owner:** #4913

## 1. Research hypothesis

Recent buried-pipe localization work suggests that continuous-motion acoustic arrays may reduce stationary acoustic acquisition overhead. This is a hypothesis to test, not a conclusion.

The comparison asks whether event-triggered, moving-array acoustic acquisition can deliver more independently locatable maintenance information per second and joule while retaining bounded localization error, uncertainty calibration, and safe robot recovery.

The benchmark distinguishes four tasks:
1. localizing the robot along a route (chainage);
2. localizing a feature/defect relative to the sensing array;
3. classifying a pipe/duct condition;
4. independently relocating a reported maintenance point.

These metrics must never be substituted for one another.

## 2. Literature baselines

- **Stationary direct acoustic + visual odometry:** Worley et al. (2026), DOI 10.1002/rob.70368. Use as a stop-and-go comparison family. Preserve its exact paper metrics, including 1D relative chainage and 0.5 m threshold; data access is via corresponding author on reasonable request.
- **Moving circular array + phase compensation:** Yu et al. (2025), DOI 10.1016/j.apacoust.2025.110545. Uses frequency-domain sparse representation, moving-robot phase compensation, direction/distance estimates with overlapping echoes, and evaluates excitation duration. Faithful reproduction requires the actual signal, excitation and motion setup.
- **Linear array + sparse beamforming:** Yu et al. (2024), DOI 10.1016/j.measurement.2024.115361. Uses a linear microphone array and mixed plane/non-axisymmetric modes. The paper reports localization within 3% of sensing distance in its test, including a 150 mm test pipe. Do not relabel this as P95 or an absolute error.
- **Dual-resolution acoustic inspection:** Sun et al. (2026), DOI 10.1016/j.tust.2025.107133. Low-frequency coarse navigation plus high-frequency local inspection; 8 cm average localization variation over 18 m and 76% feature classification in its own experiment. "Variation" must retain the paper's definition.
- **Active-SLAM uncertainty planning:** Sansoni et al. (2025), DOI 10.1016/j.robot.2025.105059. Use as a planning-method reference for uncertainty maps/frontiers, not as buried-pipe acoustic performance evidence.

## 3. Experimental treatment matrix

| Family | Acquisition | Motion | Purpose |
|---|---|---|---|
| M0 | conventional manual proving / inspection | manual | relevant workflow baseline |
| M1 | fixed-cadence direct acoustic pulse | stop–measure–go | stationary acquisition baseline |
| M2 | moving circular array, phase compensated | continuous | published moving-array family |
| M3 | moving linear array / sparse beamformer | continuous or controlled sweep | published array/beamforming family |
| M4 | dual-resolution low/high acoustic | continuous with coarse/fine phases | acoustic-only navigation/inspection family |
| M5 | event-triggered sensing schedule | adaptive stop/slow/go | Luminous research policy |
| M6 | acoustic + visual + IMU/tether factor graph | adaptive | heterogeneous fusion |
| M7 | M6 + surveyed route anchors/topology | adaptive | global-placement ablation |

A method must be marked not reproduced when its raw data/code or precise method details are unavailable. A simplified Luminous prototype must not be called a reproduction.

## 4. Active acquisition decision

At decision step t, the policy can choose from a bounded set of actions:

- CONTINUE_MOVING: keep current speed and sensor schedule;
- SLOW_AND_SAMPLE: reduce speed and acquire an array window;
- PAUSE_AND_CHIRP: stop and acquire a longer/higher-SNR sample;
- CHANGE_EXCITATION: alter declared waveform/frequency/duration;
- REVERSE_VIEW: move to an independently safe viewpoint for disambiguation;
- DEFER_OR_ABSTAIN: do not issue a precise location claim.

The research policy should consider expected reduction in task loss or localization uncertainty, measurement latency and stationary dwell, energy and processing budget, sensor health and common-cause degradation, risk of false high-confidence locations, profile compatibility and route observability, operator intervention and recovery burden.

The policy is evaluated offline against a frozen independent reference table. It does not directly command a physical robot. In a later physical experiment, any action proposal requires an authorized supervisor and separate safety controller.

## 5. Required metrics

Report separately, per population/profile:

**Localization:** chainage P50/P95/max, endpoint error, drift per metre, wrong segment/branch/anchor rate, outage duration, reacquisition time.

**Feature localization:** distance and direction error using the exact paper-defined metric where it can be reproduced; state the reference frame.

**Classification:** per-class precision/recall/F1, macro-F1, critical obstruction false-negative rate, abstention and calibration.

**Efficiency:** route speed, stationary dwell, total mission time, signal excitation duration, processing latency, energy per accepted metre, operator minutes, accepted maintenance points/hour.

**Integrity and recovery:** high-confidence wrong location, uncertainty interval coverage, sensor dropout behavior, failed/aborted/recovered runs, safe return/retrieval, duct and dummy-cable damage, independent point relocation.

Do not publish a composite score that can conceal false-clearance or critical false-negative events.

## 6. Synthetic corpus contract

The v0.1 manifest contains 14 deterministic scenarios with analytically specified route truth and action outcomes. The scenarios test policy-level correctness, not physical acoustics. The manifest deliberately does not contain fabricated waveforms or a simulated propagation model.

Scenario families cover clean moving pass; fixed stop/go baseline; repeated echo aliases; motion/phase compensation out-of-envelope; excitation duration too short/too long; speed/clock timebase mismatch; failed microphone channels; motor noise and vibration; cable/draw-rope occupancy; water/debris domain shift; false branch association; common-cause sensor degradation; no positive-value sensing action; and required abstention due to missing ground truth.

An independent oracle defines expected dispositions. The estimator/policy cannot provide its own truth labels.

## 7. Proposed goals (not achieved)

- Reduce stationary dwell by at least 25% relative to matched fixed stop/go acquisition at non-inferior independently measured maintenance-point localization.
- Keep P95 localization degradation within 5% of the matched reference, if the data and tolerance make that comparison statistically meaningful.
- Zero high-confidence location claims for explicitly ambiguous or unobservable synthetic cases.
- 100% route coverage accounting.
- 100% inclusion of aborted, failed and manually recovered runs in efficiency denominators.
- Evaluate generalization on held-out pipe material, diameter, joint geometry and occupancy; no adjacent-frame leakage between train/test.
- No physical, cable-integrity, electrical or safety qualification follows from an L0/L1 pass.

## 8. Data and rights

- Worley et al. raw data: request from corresponding author; do not redistribute without permission.
- Yu et al. 2025 accepted manuscript: University of Sheffield record indicates CC BY 4.0 for the accepted manuscript; confirm dataset/code and any third-party asset rights separately.
- Yu et al. 2024 published version: open access CC BY.
- Sun et al. accepted manuscript: University of Bristol record indicates CC BY for the accepted manuscript; publisher version and third-party material may have different terms.
- Any vendor manuals, photos, datasets or firmware documentation must be recorded with exact version and use rights.

## 9. Claim ceiling

The JSON manifest is a deterministic policy-level campaign plan. It is not acoustic waveform physics, a published-paper reproduction, a localization implementation, an acoustic array qualification, or a cable-duct safety case. All numeric research targets are hypotheses until matched experiments and independent verification exist.
