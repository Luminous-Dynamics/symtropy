# Adaptive PID protection-level baseline harness v0.1

**Direct comparator:** Maharmeh, Resende & Nashashibi, Sensors 26(16), 5140 (2026), DOI 10.3390/s26165140.  
**Baseline plan:** [NAV-URBANNAV-BASELINES-001 #6733](https://github.com/Luminous-Dynamics/symtropy/issues/6733).

## Implemented law

The Rust module implements the published one-axis controller from explicit precomputed inputs:
1. instantaneous drift rate: IMU base drift + weighted directional filter/IMU displacement discrepancy per second + weighted LiDAR residual × inverse-square-root Hessian sensitivity;
2. bounded setpoint evolution via the paper's saturation function;
3. optional GNSS bound intersection using projected innovation plus caller-supplied TIR inflation factor times projected GNSS sigma;
4. normalized LiDAR residual entropy and linear gain scheduling;
5. conditional-integration anti-windup and first-order low-pass derivative;
6. PID velocity command;
7. full-speed expansion and damped contraction, clamped to a noise-derived floor and absolute PL ceiling.

The paper's numerical configuration is provided by `AdaptivePlConfig::paper_2026(k_tir)`. The caller can obtain the 2D radial Student-t multiplier with `student_t_k_tir_2d(dof, tir)` from the adjacent covariance baseline module, but should do so only after confirming the paper's exact TIR coverage convention. The controller harness keeps K_TIR explicit to avoid silently imposing a distributional interpretation.

## Explicit boundary

This does **not** implement:
- ES-EKF state estimation / GNSS fusion;
- raw IMU propagation or gravity compensation;
- LiDAR scan-to-map registration or point-residual extraction;
- Hessian/Jacobian construction and eigenvalue estimation;
- entropy histogram generation from raw residuals;
- UrbanNav parsing, timestamps, frame transforms or calibration;
- paper's full baseline comparator / exact route preprocessing.

Those inputs are required for a paper-faithful route reproduction. This module is an isolated equation-level baseline harness that allows its numeric controller law to be tested now without claiming the route experiment has been reproduced.

## Conditional-integration fidelity

The implementation exposes two modes rather than silently altering the paper's discrete rule:

- `IntegralAntiWindup::PaperConditional` is the default in `paper_2026()` and follows the paper's Equation (33): integrate when the previous integral state is inside the threshold or when the error unwinds it. Because it accepts one full discrete increment, it may cross the threshold by one increment before subsequent integration is blocked. This finite-step detail is retained for faithful reproduction.
- `IntegralAntiWindup::HardClamped` is an explicit conservative variant that additionally clamps the newly computed integral state to the threshold. It is a robustness modification, not the default paper implementation.

The tests cover both behaviours. Do not mix their results in a baseline comparison: report the selected policy in the configuration receipt.

## LiDAR residual equation caveat

The paper describes the geometric residual as an RMS point-to-map distance. However, its displayed formula is printed as the square root of the mean of Euclidean norms, without an explicit square on each norm. The latter is dimensionally inconsistent with a distance. This module accepts a precomputed geometric residual and therefore does not silently choose between the prose and printed formula. The raw LiDAR frontend must reproduce the author's exact preprocessing or resolve the discrepancy before a route-level result is labelled a paper reproduction.

## Tests

The tests cover:
- exact published parameter values except caller-supplied K_TIR;
- entropy-to-gain schedule;
- along/cross-track drift projection;
- geometric degeneracy sensitivity;
- three-term instantaneous error rate;
- GNSS candidate-bound intersection;
- asymmetric rise/fall rates;
- setpoint and PL ceilings;
- integral anti-windup;
- fail-closed invalid inputs and no state mutation.

## Known interpretive caution

The paper's text describes the LiDAR residual term as an RMS residual but the displayed equation is typeset without an explicit square on each norm before averaging. This implementation consumes the precomputed residual scalar rather than silently choosing a preprocessing interpretation. The data-pipeline reproduction must resolve this with the authors' exact source/code or an explicit equation reading.

Likewise, the paper reports finite-route Stanford-diagram exceedance fractions. Reproduce that metric exactly, but also report unique breach events, exposure duration, maximum contiguous breach and cluster-aware intervals. A 0% observed sample fraction is not a proof of zero true integrity risk.
