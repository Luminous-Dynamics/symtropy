# Synchronization integrity and active resynchronization

**Related implementation:** [PR #6785](https://github.com/Luminous-Dynamics/symtropy/pull/6785)

## Why timestamp uncertainty belongs in the integrity model

A sensor observation is not fully characterized by its value and covariance. Its acquisition epoch, clock domain, timestamp semantics, maximum age, transport/processing latency and synchronization state matter. Under motion, a time error becomes a pose error. Clock offset, scale drift, observation age and sensor lever arm can jointly dominate nominal estimator uncertainty.

The Hilti–Trimble–Oxford reference uses a PPS-synchronized LiDAR/IMU system, while the Insta360 camera/IMU stream uses an independent time base aligned through gyro-norm correlation and nonlinear offset/clock-scale refinement with gyro-bias optimization. UrbanNav combines GNSS, IMU, LiDAR, stereo and ROS time references. These are concrete reasons to expose residual time uncertainty rather than assume preprocessing has made it zero.

References:

- Hilti dataset/synchronization documentation: https://github.com/Hilti-Research/hilti-trimble-slam-challenge-2026/blob/655df637120786621be8ffb14c4ee367abfee85a/docs/ground-truth.md
- Hilti dataset page: https://hilti-challenge.com/dataset-2026
- Sensor-centric 2026 safety/uncertainty review: https://www.mdpi.com/1424-8220/26/12/3801

## Deterministic timing-to-pose envelope

PR #6785 exposes typed clock and motion bounds and computes:

`delta_t = offset_bound + drift_ppm * 1e-6 * elapsed_since_sync + observation_age`

`delta_p_linear = v_max * delta_t + 0.5 * a_max * delta_t^2`

`delta_theta = omega_max * delta_t + 0.5 * angular_accel_max * delta_t^2`

`delta_p_lever = 2 * r * sin(min(delta_theta, pi) / 2)`

`delta_p_total = delta_p_linear + delta_p_lever`

The additive result is a deterministic bound conditional on the supplied maxima. It is not a distribution, covariance, probability or protection level.

## Conservative composition with Student-t PL

The same PR can add `delta_p_total` to an existing Student-t overall, along-track and cross-track protection level. This uses the triangle inequality; it does not assume statistical independence between the timing bias and estimator error. Do not substitute RSS unless the dependence and stochastic model are justified. Do not add the same timing uncertainty twice if it is already propagated through the base covariance.

## Synchronization-aware sensing/refresh policy

The latest implementation also includes a bounded inverse solver that finds the maximum additional holdover time before the temporal position-bias envelope exceeds a declared budget:

- if the budget is already exceeded, permitted holdover is zero;
- if the full declared search horizon remains within budget, the solver returns that cap;
- otherwise it uses bounded bisection to find the threshold.

This creates an explicit active policy: refresh synchronization before the envelope consumes the allowed error budget, enter DEGRADED if a refresh cannot be obtained, or abstain if no valid bound remains. It does not guarantee the sync attempt succeeds, nor does it turn unknown oscillator drift into a valid bound.

## Validation plan

Use two separate phases:

1. **Synthetic containment:** apply known time offsets/drift/latency to a deterministic trajectory generator; compare true pose displacement from timing faults with the computed envelope; include clock resets, sync dropouts, observation reordering, rolling-shutter row timing and timestamp semantics.
2. **Physical qualification:** obtain independent PPS/PTP or other reference timing; measure offset/scale/holdover bounds across temperature and motion; freeze them using a separate calibration set; test containment on untouched sessions.

Stress values such as 1–100 ms offset, 1–100 ppm clock drift and 0–250 ms observation age are experimental grid choices only. They are not claims about the actual Hilti/UrbanNav clock errors.

A reference trajectory synchronized using the same gyro-correlation procedure is not an independent proof of residual synchronization error. Do not calibrate the bound and score it against the same timing lineage.

## Benchmark outputs

Report temporal-envelope coverage/containment, bound tightness, worst violation, false-safe exposure, time to synchronization fault alert, time to resync, service availability cost, PL inflation and compute/energy overhead. Keep the official trajectory-accuracy score separate from integrity and synchronization-assurance metrics.

No physical clock bound, sensor timing calibration, or field containment result is established by the current PR.