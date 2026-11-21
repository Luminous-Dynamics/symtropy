# EHD Array Interaction and Force-Coupling Benchmark (2026)

**Benchmark:** EHD-ARRAY-INTERACTION-001  
**Status:** measurement protocol and data contract; no Luminous array measurements  
**As of:** 21 November 2026  
**Machine-readable form:** [ehd_array_interaction_benchmark_v0_1.json](../../data/research/ehd_array_interaction_benchmark_v0_1.json)

## Why this deserves a separate benchmark

The paired-anode experiment by Kitamura et al. (2023) is a direct warning against the usual assumption that a multi-thruster array is the vector sum of individually measured actuators. Their tested orthogonal pair interfered and did not superpose; the resultant direction was approximately 45 degrees. Their tested horizontal/parallel pair did superpose. This is configuration-dependent evidence, not a universal statement about all EHD arrays. [DOI 10.1109/ISIE51358.2023.10228184](https://doi.org/10.1109/ISIE51358.2023.10228184).

A second control direction is switched corona: Yashita et al. (2025) demonstrated thrust control by switching duty ratio, but switching transients and full loss accounting remain necessary before the method can be ranked as a propulsion-system efficiency or flight-control result. [DOI 10.1109/ICM62621.2025.10934861](https://doi.org/10.1109/ICM62621.2025.10934861).

The paradigm shift is to treat the array as a **configuration-conditioned vector actuator network**, not a set of independent scalar thrust sources.

## 1. Experiment ladder

1. **Isolated-cell baselines:** calibrate each cell's vector force, per-channel current/voltage waveform, terminal power and uncertainty under a declared environmental block.
2. **Paired-cell interaction:** measure the combined vector force and compare it against the isolated-cell vector sum measured in the same block.
3. **Small-array transfer:** test three-or-more-cell arrangements to determine whether pairwise interactions explain the result or higher-order terms are material.
4. **Switched-drive interaction:** measure dynamic force, per-channel waveforms, switching loss, overshoot, settling, discharge stability and thermal response.
5. **Holdout transfer:** hold out complete orientation/spacing families and a separate experimental session.

All tests require a qualified laboratory, appropriate high-voltage interlocks and independent metrology review. This document is not an apparatus energisation guide.

## 2. The interaction quantities

For two cells under matched boundary conditions, define the vector additivity residual:

[
mathbf r_{ij} = mathbf F_{ij,mathrm{measured}}
 - (mathbf F_{i,mathrm{isolated}}+mathbf F_{j,mathrm{isolated}}).
]

Keep the residual vector as the primary observation. A norm alone loses direction; a percentage error is misleading when the reference vector is close to the force-balance detection floor. Propagate the covariance of the isolated baselines and pair measurement into the residual uncertainty.

For a multi-channel array, identify a local command-to-force Jacobian:

[
deltamathbf F = mathbf J(mathcal C,mathbf u_0),deltamathbf u,
]

where the configuration (mathcal C) includes geometry/orientation/spacing, gas state, freestream and discharge regime. The matrix is local to an operating point (mathbf u_0); it is not a universal constant. Report its singular values, condition number, scaling convention, uncertainty and qualified envelope. If corona onset/extinction, a discharge-mode transition, actuator saturation or a material flow-topology change occurs, re-identify or abstain.

## 3. Control-design implication

Only after the coupling map is qualified should Symthaea use it for allocation. Compare:
- independent-cell allocation using isolated-cell baselines;
- local coupling-compensated allocation using the measured Jacobian;
- constrained health-aware allocation that can derate or isolate an unstable cell and reallocate remaining control authority.

The comparison uses identical sensor, balance and platform boundaries. Report force-vector direction and magnitude, trajectory/attitude error, settling, actuator saturation, cell failures, switching loss, total wall-plug input and complete system mass. Do not treat one-hour tethered hover as self-powered endurance.

## 4. Data and uncertainty contract

The JSON requires geometry and cell identity, per-cell drive waveform, voltage/current waveform, power, gas/environment, freestream, balance calibration, instrumentation bandwidth, session/run order, vector force/covariance, flow and stability observations, thermal and emissions channels where available.

Pair baselines must be measured in the same session and environmental block; run order is randomized; controls include no-discharge and matched dummy loads; raw timestamps are retained; and holdout/exclusion rules are frozen before analysis.

## 5. Prospective performance gates

These are targets, not achieved performance:
- held-out force-direction error no greater than 5 degrees;
- held-out force-magnitude relative error no greater than 10%, only when the reference force is resolved above its declared detection threshold;
- at least one complete orientation family, one spacing family and one independent session held out;
- no use of a local linearization outside its validated envelope;
- safe bounded response after disabling/derating one cell, within the separate platform qualification;
- switching and converter losses included in wall-plug power.

The benchmark is passed only when the full declared holdout criteria and uncertainty coverage are met. A single fitted configuration matrix is not evidence of general array control.

## 6. Claim ceiling

This is a data contract and experiment protocol. It does not claim that Luminous has measured an array coupling matrix, implemented a switching model or controller, built a prototype, performed a physical test, or exceeded SOTA.
