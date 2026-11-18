# Self-sensing tendon calibration reference — v0.1

**Owner:** [TENDON-PROPRIO-QUAL-001 #6078](https://github.com/Luminous-Dynamics/symtropy/issues/6078)  
**Parent:** [SYNTH-JOINT-001 #6037](https://github.com/Luminous-Dynamics/symtropy/issues/6037)  
**Source:** *Worm-inspired proprioceptive continuum robot with retractile motion and sensorized tendons*, Mechatronics 118 (September 2026), 103524, [DOI 10.1016/j.mechatronics.2026.103524](https://doi.org/10.1016/j.mechatronics.2026.103524).  
**Evidence class:** synthetic analytic calibration oracles plus paper-reported reference metrics; no Luminous material, actuator or joint measurement.

## 1. What the source actually demonstrates

The paper constructs a two-section, spring-based continuum manipulator. Each section has three tendons, and each tendon combines four 0.10 mm nylon filaments with one 0.08 mm nichrome wire in an approximately 0.55 mm braid. The nichrome strand provides a resistive measurement of its own tendon arc length while the braid transmits actuation load.

The reported operating interval for the linear resistance-length calibration is 40–80 mm, with approximately 1.19 mm calibration RMSE. The paper uses short voltage excitation pulses on non-overlapping channels to limit self-heating. It reports a temperature-equivalent length drift below 0.1 mm for temperature changes up to 10 °C under its chosen readout regime.

**Crucial boundary:** this is tendon-length sensing, not direct tendon-force or joint-torque sensing. The prototype also uses separate tendon tension transducers, motor rotary encoders, and two IMUs. The system therefore demonstrates integrated length sensing and redundant proprioception; it does not demonstrate that the independent force/encoder/IMU channels can be safely eliminated.

The paper reports nominal Cartesian tracking RMSE of 2.5 / 3.0 / 1.8 mm in X/Y/Z with maximum absolute errors of 7 / 11 / 7 mm. With a 150 g end-effector motor load, it reports RMSE 0.902 / 0.913 / 1.172 mm and maximum absolute errors 7.91 / 4.26 / 4.94 mm. The loaded result is a different operating condition, not evidence that load universally improves accuracy.

Under sinusoidal voltage perturbations, the reported maximum Cartesian deviation reached 47.6 mm in one trial (other trials 30.01 and 33.89 mm). The paper attributes this sensitivity to a kinematic controller without dynamic compensation for gravity, tendon tension, routing friction and backbone elasticity. Its authors identify additional fatigue/wear characterization under loads above 50 g as future work. Do not interpret the nominal tracking results as a load-bearing knee, wrist or humanoid-joint qualification.

## 2. Sensor model

Within the calibrated interval, the paper uses:

```text
R(L) = R0 + kL * (L - L0),     kL > 0

L_hat(R) = L0 + (R - R0) / kL

P_avg = delta * V^2 / R(L)

Delta_L_eq = R0 * alpha * Delta_T / kL
```

where R and R0 are resistance, L and L0 are tendon arc length, kL is the calibrated resistance-length slope, delta is the non-overlapping pulse duty factor, V is excitation amplitude, alpha is the nichrome temperature coefficient under the paper's adopted definition, and Delta_T is temperature change.

The inverse must be rejected outside the calibrated length/resistance interval. Do not extrapolate and label the result measured. The thermal relation is a correction/uncertainty term, not an independent temperature measurement.

## 3. What this reference crate checks

The standalone Cargo crate contains synthetic known-answer tests for:

1. Forward resistance from the linear calibration.
2. Inverse length estimation and strict calibrated-domain rejection.
3. Average Joule excitation power under a declared duty factor.
4. Temperature-equivalent length drift.
5. First-order length uncertainty propagation from resistance and slope uncertainty.
6. Invalid/non-finite calibration, readout and excitation inputs.

All numeric test values are synthetic and deliberately separate from the paper's reported 1.19 mm RMSE. The crate does not estimate tendon tension, joint torque, full robot shape, stiffness or safety margin.

## 4. Run

```bash
cargo fmt --manifest-path benchmarks/synthetic-joints/tendon-proprio/v0.1/Cargo.toml -- --check
cargo test --manifest-path benchmarks/synthetic-joints/tendon-proprio/v0.1/Cargo.toml --locked
cargo clippy --manifest-path benchmarks/synthetic-joints/tendon-proprio/v0.1/Cargo.toml --all-targets --locked -- -D warnings
```

A dedicated GitHub Actions workflow runs these commands on changes to this benchmark package. This is isolated from the root Cargo workspace and has no third-party dependencies.

## 5. Next physical qualification experiment (not authorized here)

Use one sensorized tendon coupon and an otherwise mechanically identical non-sensorized tendon control. Maintain independent inline tension, displacement and temperature references. Freeze the calibration data partition before fitting. Test held-out length, tension, temperature, routing radius, direction reversal, remount, dwell/creep, slack and cycle-age strata. Then add an antagonistic single-axis fixture only after coupon-level repeatability and uncertainty are established.

Keep the channels separate:
- Resistive tendon signal: length estimate.
- Independent tension transducer: force reference and slack/overload protection.
- Encoder: motor/capstan motion readback using a pinned gear-ratio convention.
- IMU/optical metrology: independent geometry reference.

Sensor-removal claims require a causal ablation while the independent referee instruments remain installed. Hiding a reference channel from an estimator is not the same as physically removing it from the system mass, safety or energy boundary.

## 6. Claim ceiling and safety

This is an analytic software reference only. It does not reproduce the paper's material, calibrate a physical tendon, demonstrate load-bearing capacity, validate fatigue life or authorize fabrication, energization, human contact or wearable testing. The braid construction, terminations, routing, temperature, duty cycle and life state each require their own measurements. Any repaired, rerouted or replaced tendon invalidates its previous calibration until requalified.