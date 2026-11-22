# Temporal-alignment misregistration envelope v0.1

## Purpose

Sensor synchronization is usually treated as a preprocessing step. In real systems, camera, LiDAR, GNSS, IMU, UWB and reference clocks can have offset, scale drift, age and latency. Treating a single estimated time offset as exact hides one source of pose uncertainty.

This kernel propagates explicit **bounds** on relative clock offset, clock scale drift and observation age through a bounded motion envelope and sensor lever arm.

## Model

Clock skew since the last qualified synchronization is bounded by:

`delta_t_clock = abs_offset_bound + drift_bound_ppm * 1e-6 * elapsed_since_sync`

Total temporal misalignment includes observation age:

`delta_t_total = delta_t_clock + observation_age_bound`

With declared maximum motion bounds:

`delta_p_linear = v_max * delta_t_total + 0.5 * a_max * delta_t_total^2`

`delta_theta = omega_max * delta_t_total + 0.5 * angular_accel_max * delta_t_total^2`

A sensor lever arm at distance r contributes a rotational chord bound:

`delta_p_lever = 2 * r * sin(min(delta_theta, pi) / 2)`

The resulting conservative position envelope is:

`delta_p_total = delta_p_linear + delta_p_lever`

This uses triangle-inequality addition rather than assuming independent Gaussian errors. It is deterministic **conditional on all supplied maxima being valid bounds**.

## What it does not establish

- It is not a covariance, standard deviation, probability or protection level.
- It does not estimate clock offset or drift from raw signals.
- It does not validate the motion envelope, lever arm or synchronization evidence.
- It does not include all sensor-specific interpolation, rolling-shutter, exposure, timestamp-quantization or clock-reset effects.
- It does not establish independence from other estimator/fusion errors.

To use it in a PL calculation, the caller must provide auditable bounds and define how this term composes with estimator, calibration and common-cause terms. Do not RSS-combine it with unrelated terms without a justified dependence model.

## Relevance to the benchmark portfolio

The Hilti–Trimble–Oxford dataset uses independently running camera and LiDAR-inertial systems. The reference LiDAR/IMU system is PPS synchronized, while the Insta360 camera/IMU clock is aligned using gyro-norm correlation and nonlinear time-offset/clock-scale refinement. A constant timestamp shift is also applied. UrbanNav combines multiple sensor streams with different rates and ROS/GNSS time bases. These are reasons to make residual time uncertainty an explicit model input, not to assume the dataset's actual residual skew is known.

Dataset notes:

- Hilti: https://github.com/Hilti-Research/hilti-trimble-slam-challenge-2026/blob/655df637120786621be8ffb14c4ee367abfee85a/docs/ground-truth.md
- UrbanNav: https://github.com/IPNL-POLYU/UrbanNavDataset

## Validation

Unit tests cover zero error, offset and observation-age propagation, clock drift accumulation, rotational lever-arm displacement, acceleration terms, monotonicity with stale synchronization, invalid inputs and arithmetic overflow. The demo uses synthetic timing and motion bounds; it is not fitted to Hilti, UrbanNav or a physical vehicle.
## Conservative composition with a Student-t protection level

The module also provides `inflate_student_t_protection_levels_by_temporal_bound`. Given an existing two-dimensional Student-t PL result and a qualified deterministic temporal position-bias bound `delta_p`, it returns:

`PL_overall,total = PL_overall,stat + delta_p`

`PL_long,total = PL_long,stat + delta_p`

`PL_cross,total = PL_cross,stat + delta_p`

For any directional unit vector, the absolute projection of the bounded position-bias vector is no larger than its Euclidean norm. The additive form therefore follows the triangle inequality and does not require independence between the statistical estimator error and time-alignment bias. It is conservative and must not be RSS-combined with other sources without a justified dependence model.

Do not add the temporal term twice if the base covariance/PL calculation already includes that same timing uncertainty. All clock and motion bounds must be independently evidenced before the inflated PL can be interpreted as a valid protection level.
## Proactive synchronization holdover scheduling

The module also provides `maximum_sync_holdover_under_position_budget`. Given current clock bounds, a motion envelope, an explicit temporal position-bias budget and a finite search horizon, it finds the largest additional time before the computed bias envelope exceeds that budget.

- If the budget is already exceeded, the permitted additional holdover is zero.
- If the budget remains satisfied at the search horizon, the result is capped at that horizon and reports that the budget is still satisfied there.
- Otherwise it uses bounded bisection over the monotonic envelope to find the threshold.

This is a deterministic scheduling aid: it can tell a service when to request resynchronization, enter DEGRADED, or abstain before the timing-derived bias budget is exhausted. It does not guarantee that a synchronization attempt will succeed. It also does not make unknown clock bounds valid; missing or unqualified clock bounds must remain unknown/conservative.

Example sensitivity test: with 1 ms initial offset, 100 ppm drift, 5 m/s speed, zero acceleration/rotation, and a 1 cm temporal position-bias budget, the maximum additional holdover is approximately 10 seconds. These are synthetic inputs, not Hilti or UrbanNav measurements.