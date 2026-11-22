# UrbanNav matched baseline reproduction protocol v0.1

**Issue:** [NAV-URBANNAV-BASELINES-001 #6733](https://github.com/Luminous-Dynamics/symtropy/issues/6733)  
**Dataset candidate:** [NAV-URBANNAV-001 #6727](https://github.com/Luminous-Dynamics/symtropy/issues/6727)  
**Status:** reproduction protocol only; none of these baselines has been reproduced by Symtropy.

## 1. Objective

Run an evidence-carrying bake-off on the same UrbanNav route segments. The first goal is to reproduce established methods, not to compare our new service policy against an intentionally weak or mismatched estimator.

The bake-off must separate three layers:

1. **Pose estimation:** produce the best estimate under the same sensor inputs.
2. **Integrity/protection:** bound the error and decide when a pose is safe for a declared task.
3. **Observation/service policy:** decide which observation to request next, whether to abstain, and when a degraded service may recover.

A method may be excellent at one layer and weak at another. Scores must not conflate them.

## 2. Dataset selection and frozen route plan

The candidate is UrbanNav-HK-Medium-Urban-1:
- publisher-reported 33.7 GB, 785 seconds and 3.64 km;
- GNSS, IMU, LiDAR, stereo camera and SPAN-CPT + Inertial Explorer reference;
- RINEX 3.02 and ROS TimeReference topic documented upstream.

The upstream MIT dataset-payload terms are explicit, but the dataset remains not admitted until the chosen downloaded objects are frozen and hashed and the time/frame/calibration/reference lineage has passed audit. No data payload is implied to be downloaded by this document.

### Route partitions

Do not randomly split epochs or sliding windows. Keep complete loops/route sessions grouped. For the first sequence, create immutable segment manifest entries for:
- loop A;
- loop B;
- known u-blox F9P malfunction interval;
- sensor outage or missing-reference intervals;
- any segment where reference trajectory or calibration is invalid.

The exact loop boundaries and F9P fault interval must be determined from the pinned bag and upstream issue #11 before scores are computed. Until then, keep all segments unqualified. The malfunction interval must either be excluded from nominal performance or explicitly labeled as a fault episode in a separate stress test.

Initial development can use one loop for parser and estimator debugging, but final parameter selection must not use the held-out loop. Better still, use the Medium-Urban route for initial reproduction, then freeze Deep-Urban as an environment-held-out route when its exact reference coverage and calibrations are qualified.

## 3. Primary baseline A: smooth adaptive protection levels as a control problem

**Maharmeh, Resende and Nashashibi**, “Integrity as a Control Problem: Smooth and Adaptive Protection Levels for Multi-Modal Localization”, Sensors 26(16), 5140 (14 August 2026), DOI 10.3390/s26165140.

Paper:
- https://www.mdpi.com/1424-8220/26/16/5140
- https://doi.org/10.3390/s26165140

The authors reformulate protection-level generation as a closed-loop controller:
- estimate longitudinal and lateral drift by comparing ES-EKF displacement with IMU dead-reckoning;
- combine baseline inertial drift, directional drift and a LiDAR geometric-residual term weighted by a sensitivity measure derived from the minimum eigenvalue of the local information/Hessian matrix;
- propagate a bounded ideal setpoint through a saturation dynamic;
- track that setpoint using an adaptive PID controller;
- adapt gains from normalized Shannon entropy of the LiDAR ICP residual histogram;
- expand PL at full controller command rate, but contract it more slowly;
- clamp the PL to a noise-derived lower floor and application ceiling, with anti-windup.

### Paper-reported configuration to reproduce

The published configuration lists:
- base IMU drift rate 0.05 m/s;
- drift weight 1.5;
- geometric weight 0.5;
- target integrity risk 1e-3;
- maximum PL Smax = 10 m for the primary result;
- saturation exponent n = 2;
- nominal PID gains Kp=0.2, Ki=0.05, Kd=0.001;
- asymmetric decay modifier 0.15;
- derivative cutoff 5 Hz;
- integral saturation threshold 0.3;
- entropy gain coefficients alpha_p=2.0, alpha_i=0.8, alpha_d=0.5.

The paper also reports a sensitivity sweep at Smax 10, 12 and 14 m. Reproduce these exact values and the paper's full/non-integral variants before tuning anything for Luminous.

### Paper-reported outcomes and caveats

Across three UrbanNav routes (medium-urban 3.64 km, low-urban 1.21 km, deep-urban 4.51 km), the authors report 0.0% pointwise PL exceedance for medium/low and non-zero deep-urban exceedances. For the deep route they report 1.7% longitudinal / 2.3% lateral for the non-integral form and 1.6% / 1.0% for the full PID with integral; a larger Smax=14 m sweep reduces reported exceedance further (1.4% longitudinal, 0.7% lateral).

These are finite-route empirical Stanford-diagram point fractions, as defined in the paper. They must **not** automatically be reinterpreted as a calibrated per-hour, per-mission or probabilistic integrity guarantee. The authors also state that an error-gradient episode briefly exceeds the deployed PL. Their own deep-urban result is therefore a useful failure case, not a perfect safety result.

### Exact reproduction gates

- Reconstruct the paper's ES-EKF / IMU dead-reckoning and LiDAR registration signals, not a substitute covariance trace.
- Use the same route intervals, local vehicle frame and along/cross-track projection.
- Reproduce the published PL curves and pointwise Stanford-diagram exceedance exactly.
- Reproduce full PID and non-integral forms, and Smax 10/12/14 sweeps separately.
- Check entropy histogram bin count, LiDAR residual normalization, Hessian eigenvalue floor, time base, initialization and PL lower floor from the full paper / code.
- Run negative controls with GNSS outage, repeated geometry, moving objects, degraded LiDAR registration, IMU drift and long outage.
- Add event/session cluster confidence intervals and exposure-time metrics as separate metrics; do not change the paper's own metric during reproduction.

**Do not treat 0.0% exceedance on one route as proof that the true integrity risk is zero.** Report the sample count, duration, distinct degradation events, cluster interval and maximum contiguous breach.

## 4. Primary baseline B: adaptive temporal Kalman filter plus solution separation

**Luo, Sun and Fu**, “ATM: Adaptive Temporal Kalman Filter-Based Solution Separation Method for Positioning and Integrity Monitoring”, IEEE Transactions on Instrumentation and Measurement 75 (2026), DOI 10.1109/TIM.2026.3670522.

References:
- https://doi.org/10.1109/TIM.2026.3670522
- https://www.researchgate.net/publication/401578449_ATM_Adaptive_Temporal_Kalman_Filter-Based_Solution_Separation_Method_for_Positioning_and_Integrity_Monitoring

The accessible publisher-independent record describes:
- measurement-noise modelling;
- an adaptive first-order Markov model for temporally colored noise;
- a combined state/measurement-vector formulation;
- weighted-least-squares reformulation of the Kalman update;
- positioning-domain solution separation for fault detection and exclusion.

The accessible abstract/index reports horizontal accuracy improvements of 4.80% and 4.57% on the reported medium/deep UrbanNav experiments, HPL decreases of 13.32% and 18.32%, and VPL decreases of 18.51% and 26.74%.

**Full-paper gate:** the complete method details, baseline configuration, route interval, parameter fitting procedure and uncertainty definitions have not yet been recovered from the accessible full-text sources. These headline figures are not a reproduction recipe. Obtain the full paper and exact configuration from the authors or publisher before implementing the ATM baseline. Do not infer the missing algorithm from percentages.

### Reproduction requirements

- reproduce the colored-noise first-order Markov model and the state/measurement combination exactly;
- reproduce the solution-separation fault hypotheses and FDE thresholds;
- use identical satellite observations, receiver, ephemeris products, antenna and time alignment;
- reproduce the same baseline and UrbanNav route intervals;
- report positioning accuracy, HPL/VPL, detection/exclusion probability, false alarm, outage and compute overhead;
- validate HPL/VPL against independently reviewed truth, not the algorithm's own state/covariance.

## 5. Primary baseline C: resilient GNSS–IMU–LiDAR–camera–loop fusion

**Liu et al.**, “Robust urban SLAM via resilient GNSS–IMU–LiDAR–camera–loop fusion”, ISPRS Journal of Photogrammetry and Remote Sensing 238 (August 2026), pages 794–815, DOI 10.1016/j.isprsjprs.2026.05.035.

Paper:
- https://doi.org/10.1016/j.isprsjprs.2026.05.035

Publisher abstract describes:
- GNSS, LVIO and loop-closure factors;
- per-factor uncertainty models;
- frame-by-frame covariance propagation of LVIO on Lie groups, constrained by absolute GNSS and loop closures;
- Hessian-eigenvalue LIO degeneracy detection;
- per-degree-of-freedom down-weighting according to degeneracy level;
- LiDAR-assisted, depth-enriched visual features;
- 3D object detection / dynamic point removal;
- evaluation including UrbanNav, KITTI, GREAT-WHU and two self-collected datasets.

The authors report pose performance comparable to their SOTA SLAM comparisons and better overall map-construction quality. Those are paper claims, not independently reproduced Luminous results.

### Code and licensing caveat

The linked candidate code repository is https://github.com/TK72/FAST-LIVOSAM, pinned for this audit at commit 9aa51993cdf30bebd6d24d538bdba75f11b9ab3e. Its README advertises FAST-LIVO2, global pose graph, CenterPoint dynamic-object removal and multi-sensor fusion, but says current experiments are mainly GREAT-WHU and KITTI. It does not establish that the exact paper UrbanNav experiments/configuration are in the repository.

The README has a “MIT License” heading, but the repository root snapshot has no LICENSE file and GitHub metadata reports no recognised licence. Do not assume source reuse or redistribution is authorized until a complete licence file / author confirmation is obtained. Reproduction may still be designed independently from the paper and existing permissively licensed upstream dependencies, subject to each component's licence.

### Reproduction gates

- identify exact paper source revision and the precise UrbanNav config used;
- obtain all factor weights, covariance models, keyframe and loop-closure thresholds, map settings and CenterPoint version;
- separate estimator-pose results from map-quality/dynamic-object removal results;
- reproduce at least the UrbanNav route on which calibration and truth coverage are qualified;
- compare ablations: no GNSS factor, no loop closure, no Hessian degeneracy weighting, no dynamic filtering, no camera depth enrichment;
- report position/orientation error, map completeness/precision, dynamic-object leakage, runtime, GPU dependence and sensor dropout;
- do not call a repository demo a paper reproduction without exact version/config and route match.

## 6. Primary baseline D: receiver-only Doppler-rate spoofing detector

**Fang and Xu**, “A Doppler frequency metric for GNSS spoofing detection in urban environments”, Aerospace Systems (6 July 2026), DOI 10.1007/s42401-026-00509-7.

Paper: https://link.springer.com/article/10.1007/s42401-026-00509-7

Method:
- standard receiver Doppler frequency tracking outputs only;
- compare measured Doppler frequency-shift rate against predicted rate derived from receiver navigation outputs;
- adaptive threshold and Neyman–Pearson decision rule;
- authors analyse 16 user-dynamics / spoofer-antenna scenarios and report theoretical effectiveness in 11;
- TEXBAT evaluation reports detection delay under 5 seconds and false alarm around 2%.

This is a valuable lightweight receiver-only baseline. Its known ineffective scenarios must remain in the benchmark; do not report only the 11 successful cases. Also measure multipath and unintentional interference false alarms in the UrbanNav environment.

Use it on HK-UrbanSpoof only after the raw IF payload, rights and reference-label ambiguity are resolved. TEXBAT may be used independently under its own licence/terms.

## 7. Common matched-run contract

Every baseline must receive:
- identical exact input file manifest and payload hashes;
- exact same route segment, start/stop and excluded intervals;
- same coordinate frame, datum, transform, antenna/IMU lever arm and time conversion;
- same allowed sensor streams and sensor dropout masks;
- same ground-truth version and truth-exclusion policy;
- same calibration/training data and frozen validation/test split;
- same hardware/compute envelope for runtime comparison;
- fixed, logged parameter-search budget with no test-route tuning.

Do not randomly split epochs. Keep full loops, route sessions, spoofing tests and physical events indivisible. Use one loop for development, lock all settings, then evaluate on the untouched loop and a distinct route/environment where qualified.

## 8. Required metrics

### Estimator
- horizontal, vertical and 3D position error: median, p95, p99, maximum;
- velocity, attitude and along-track/cross-track error;
- trajectory drift per second, distance and turn;
- outage duration and trajectory-valid fraction.

### Integrity/protection
- the paper's exact pointwise Stanford-diagram exceedance metric;
- number of distinct breach events, not only the number of breach samples;
- longest contiguous PL breach and total false-safe exposure seconds;
- exceedance by hour and route-kilometre;
- time-to-alert, detection probability at fixed false-alert budget;
- HPL/VPL calibration, p95/p99/max protection levels;
- availability, continuity, abstention and requalification delay.

### Service/active sensing
- false-safe and false-degraded exposure;
- risk–coverage curve;
- energy per acquisition, observations per hour and duty cycle;
- value of information / decision value at decision time, not only sample time;
- sensitivity to measurement latency and evidence age.

### Runtime
- end-to-end latency p50/p95/p99;
- CPU/RAM/GPU, energy and startup/calibration time;
- resource-budget violations (target: zero).

### Statistics
- number of independent route loops, physical fault events and sessions;
- cluster-aware intervals by route/session/event, not per-epoch independence;
- exact ground-truth coverage and gaps;
- report the one-sided or two-sided interval convention consistently.

## 9. Claims and reproduction status

| Method | Literature result | Luminous reproduction status |
|---|---|---|
| Adaptive PID PL control | 0% medium/low route pointwise exceedance, deep-urban nonzero breach in paper | not reproduced |
| Adaptive temporal KF / solution separation | reported UrbanNav accuracy/HPL/VPL improvements in abstract | full method not yet recovered; not reproduced |
| Resilient GLVIO + loop fusion | paper reports SOTA-comparable pose/map quality on UrbanNav/KITTI/GREAT and own routes | not reproduced; code/config/licence verification incomplete |
| Doppler-rate spoofing | 11/16 representative scenarios; TEXBAT delay <5s, false alarm ~2% in paper | not reproduced |

A metric reported by an article is not a Luminous result. A matching software name is not a matched reproduction. A matched reproduction is not an independent field validation or safety certification.

## 10. Next implementation increments after baseline reproduction

Only after a faithful baseline is in hand:
1. replace a static health heuristic with multi-cause dependency-aware degradation signals;
2. add temporal freshness propagation from sample time to decision time;
3. compare smooth/asymmetric PL control with the existing risk–coverage and cluster-risk evaluators;
4. measure whether active sensing reduces energy/observation count without increasing false-safe exposure, missed-event probability or time-to-alert;
5. compare common-cause and temporal-VoI decisions against periodic, paper-faithful VoI, entropy/FIM greedy and exact small-instance oracle;
6. publish raw receipts, parameter provenance, input digests, exclusions and cluster-aware statistics.

The target is a more trustworthy **navigation service**, not a paper score that cannot be reconstructed.
