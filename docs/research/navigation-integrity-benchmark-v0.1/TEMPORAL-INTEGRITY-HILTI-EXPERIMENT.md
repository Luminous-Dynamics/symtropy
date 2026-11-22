# Temporal integrity and Hilti matched-baseline experiment plan v0.1

**Snapshot:** 22 November 2026
**Engineering references:** [PR #6785](https://github.com/Luminous-Dynamics/symtropy/pull/6785) temporal misalignment envelope; [PR #6781](https://github.com/Luminous-Dynamics/symtropy/pull/6781) trajectory support evaluator; [PR #6779](https://github.com/Luminous-Dynamics/symtropy/pull/6779) Hilti dataset candidate.

## 1. Hypothesis

A localization system should not treat sensor timestamps or observation age as exact. A bounded timing error under motion creates a bounded position/orientation misregistration; if unmodelled, it can make a nominal covariance or protection level overconfident. Explicit timing envelopes should improve the calibration and conservatism of output support decisions, at a measurable cost in protection level, availability or sensing frequency.

This is a testable engineering hypothesis, not a result. The Rust kernel in #6785 computes a deterministic envelope from caller-supplied clock and motion bounds; it does not estimate or validate those bounds.

## 2. Matched official Hilti baseline

The official challenge is still open for submissions. It ranks two tasks:

- SLAM in any reference frame: 25 evaluated runs.
- Floor-plan localization: 24 evaluated runs.
- Each run requires at least 99% matched reference-pose coverage; otherwise its score is zero.
- The per-pose position score is exponential, and the final score sums the per-run scores.

Current published leaderboard anchors:

| Result | Published score | Additional reported detail |
|---|---:|---|
| Best SLAM — ACDC-K / KAIST | 2,410.0 / 2,500 | ACDC-VSLAM: equal-area spherical features, point/line constraints, reliability/depth-weighted BA and adaptive loop corrections |
| Best floor-plan localization — ETH–THU–Stuttgart | 2,196.3 / 2,400 | OKVIS2-X with online extrinsic calibration and floor-plan global anchoring |
| Second SLAM — SPARO / Inha | 2,393.9 / 2,500 | 0.109 m mean RMSE, 0.095 m mean position error, 99.85% mean coverage |

Official references:

- https://hilti-challenge.com/
- https://hilti-challenge.com/leaderboard-2026
- https://github.com/Hilti-Research/hilti-trimble-slam-challenge-2026
- https://sparolab.github.io/research/hilti_challenge/

Reproduce the official metric exactly first. Use OpenVINS and Stella-VSLAM as repository-provided starter baselines; seek to reproduce OKVIS2-X and the winner pipelines only after exact configuration and code/data rights are reviewed. Do not imply that the winning team result has been reproduced by Luminous.

## 3. Freeze the operating conditions

Every contender receives the same ROS bag payload bytes, camera/IMU calibration, image preprocessing, rolling-shutter configuration, initialization, map/floorplan priors, allowed sensor privileges, timestamp/frame conversion, evaluation interval, hardware and compute budget.

Keep the five early-release sequences for development/calibration only. Use the official held-out leaderboard set for final scoring, then add leave-one-floor-out and leave-one-date/condition-out cross-domain folds. Entire runs and repeated traversals are indivisible groups. The 30 sequences are not 30 independent missions merely because they have separate run IDs.

The official dataset has two 30 Hz fisheye camera streams and 1 kHz IMU. Its reference uses a rigidly attached Hesai XT32 LiDAR-inertial system; raw LiDAR is not distributed. The reference and camera systems have separate clocks, with time mapping estimated by gyro-norm correlation and nonlinear offset/clock-scale refinement. Preserve the stated 10,000 s timestamp shift and five-second initial exclusion. The truth is useful for scoring, but it is not independently reproducible from the released camera/IMU payload alone.

The repository and challenge site specify CC BY-NC-SA 3.0: non-commercial research with attribution/share-alike only. This candidate is not commercially usable without separate permission. Do not upload data to commercial infrastructure or distribute derivatives beyond the licence terms.

## 4. Synthetic temporal stress matrix

First test the mathematical envelope against a controlled simulator where the ground-truth time shift is known. These are deliberately synthetic stress settings, not claims about the actual Hilti clock error:

| Dimension | Proposed stress values |
|---|---|
| Relative clock offset | 0, 1, 5, 10, 20, 50, 100 ms |
| Relative clock scale drift | 0, 1, 10, 25, 50, 100 ppm |
| Observation age | 0, 10, 20, 50, 100, 250 ms |
| Linear speed | 0.1, 1, 3, 5 m/s |
| Linear acceleration | 0, 0.5, 1, 3 m/s² |
| Angular rate | 0, 0.1, 0.5, 1.5 rad/s |
| Sensor lever arm | measured physical lever arm; sensitivity sweep 0, 0.1, 0.25, 0.5 m |

Use a full-factorial sweep only for small subsets; otherwise use a predeclared covering array and corner-case suite. Include clock resets, discontinuous time jumps, dropped synchronization pulses, rolling-shutter row time, exposure midpoint/center semantics and delayed delivery as separate fault families. Do not fold these distinct phenomena into one generic scalar without tracing the assumptions.

## 5. Containment and calibration metrics

For each imposed timing fault, compare the true pose mismatch due to time misalignment with the computed deterministic envelope:

- containment rate: fraction of reference pose errors no greater than the envelope;
- maximum violation magnitude and duration;
- envelope tightness ratio: envelope size divided by the realized temporal pose error (report only where the error is nonzero);
- false-safe exposure when a system remains NORMAL despite the truth error exceeding its total PL;
- p95 time to detect timing degradation and time to enter a conservative service state;
- recovery time after synchronization is requalified;
- accuracy/availability cost of the added temporal term;
- runtime and energy overhead.

Do not tune maximum clock/motion bounds on the same held-out sequence where containment is reported. Estimate/qualify bounds on separate synchronization-calibration data, freeze them, then test on untouched runs.

## 6. Physical bound qualification

To promote the temporal envelope from synthetic sensitivity to physical evidence, obtain an independent timing reference or synchronization instrumentation. A timing residual inferred from the same reference trajectory used for scoring cannot independently validate itself.

Record:

- clock identities, source of timestamp, hardware/software timestamp semantics, time scale and epoch;
- offset and scale-drift estimate plus a conservative uncertainty/maximum bound;
- age from acquisition to consumer, including processing, queueing and transport latency;
- resynchronization interval, reset events, PPS/PTP/gPTP state, holdover and oscillator temperature/rating;
- exposure interval, rolling-shutter row timing, timestamp reference point and readout direction;
- frame/body lever arms and their calibration uncertainty;
- vehicle speed, acceleration, turn rate and angular acceleration limits from independent measurement or a declared ODD envelope.

On Hilti, the documented gyro-correlation time synchronization is part of the reference-generation lineage. It is not independent proof of the remaining time-mapping error. On UrbanNav, qualify ROS TimeReference/GNSS time, RINEX, IMU and ground-truth clocks separately.

## 7. Combining temporal bound with protection levels

The #6785 helper adds a deterministic temporal position-bias bound to the base Student-t PL using a triangle-inequality composition:

`PL_total = PL_statistical + delta_p_temporal`

This is conservative under arbitrary dependence only if `delta_p_temporal` really bounds the timing-induced position bias and the base statistical PL covers the remaining error contribution. Do not include timing uncertainty again if already propagated through the base covariance/PL. Do not replace addition by root-sum-square without a justified stochastic and dependence model.

The current helper treats each directional PL plus a Euclidean position bound conservatively. A future refinement can compute a directional support bound using the full bounded velocity/acceleration and lever-arm geometry, but only after a verified frame transform and interval arithmetic implementation.

## 8. Success criteria

Research targets (not achieved results):

1. Mathematical containment in all deterministic unit/corner tests.
2. On physical held-out runs, a declared high containment target with confidence bounds based on independent runs/events—not high-rate epochs.
3. A smaller gap between promised bound and realized temporal error than a crude global worst-case bound, without losing containment.
4. Lower false-safe exposure than the same estimator with timing uncertainty ignored.
5. No increase in false NORMAL restoration during clock reset, holdover or loss of sync.
6. Report position score/coverage, evidence-backed coverage, integrity risk and energy/latency together.

Any failed containment, missing clock evidence or unbounded time age should force the output into a conservative support state, not silently return a normal position.

## 9. Promotion boundaries

Passing the current Rust tests validates formula/software behavior under supplied bounds. Passing synthetic containment validates the simulator and envelope implementation under the selected stress model. Neither establishes a physical clock bound. Physical held-out containment requires independently measured timing and motion limits. Safety or certification requires a separate use-specific safety case.