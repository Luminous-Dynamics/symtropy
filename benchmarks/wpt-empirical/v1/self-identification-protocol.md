# Online self-identification benchmark v1 (research draft)

**Protocol ID:** WPT-SELF-ID-01  
**Schema:** `wpt-self-identification-run-v1`  
**Tracking:** [WPT-SELF-IDENTIFICATION-001 #3512](https://github.com/Luminous-Dynamics/symtropy/issues/3512)  
**Status:** research benchmark specification; no new physical experiment is claimed.

## 1. Research premise

The 1 October 2026 Zakerian et al. paper demonstrates an experimental multi-sine phase-perturbation method that estimates primary/secondary self-inductance, mutual inductance and equivalent load resistance from primary-side input impedance without a primary-secondary data link. A separate 50 W 2026 reflected-impedance CC/CV prototype reports low-power estimation/regulation results. These are two distinct methods and power scales; do not merge their results.

The benchmark tests a **self-sensing power link**: the power stage acts as both actuator and sensing channel. It does not remove receiver authentication, session authorization, anti-replay, protection interlocks or the safety communications plane. It asks whether the fast coupling/load estimation and frequency-control loop can operate without a live secondary telemetry dependency.

## 2. Experimental unit

One JSON/JSONL record represents one frozen operating point, reference measurement window and estimator outcome. A batch must include:
- system/hardware/controller/configuration identity;
- operating point (power direction, frequency, air gap, X/Y/Z/yaw, load, ambient and coil temperature);
- calibrated reference instrument and uncertainty;
- four estimates: (L_p), (L_s), (M), and (R_{eq});
- estimator method, secondary data-link dependency, p99 latency, declared deadline and perturbation-energy overhead;
- ZVS observation/success counts and hard switching/protection faults;
- safe action when state is unidentifiable or outside the learned/validated envelope;
- raw waveform, reference measurement, estimator output and configuration artifacts with SHA-256 digests.

## 3. Comparison methods

Run the same operating grid and reference instrumentation for:
1. no online estimation (fixed nominal tuning);
2. offline FEM/analytic lookup table;
3. secondary telemetry-based parameter feed;
4. multi-sine phase-perturbation system identification;
5. reflected-impedance decoupling / CC-CV estimation;
6. any proposed adaptive or learned estimator.

A method must not see the HOLDOUT operating points during training or hyperparameter selection. Group splits by physical coil pair, material lot, receiver design and test day; random per-row splitting is insufficient because near-identical geometry samples leak across splits.

## 4. Parameter-wise accuracy and calibration

Do not compress the four parameter errors into one aggregate score. For each parameter calculate absolute relative error:
[
e_i = 100\frac{|\hat{x}_i-x_i|}{|x_i|}.
]

Report median, P95, P99, signed bias and uncertainty coverage separately for each of (L_p,L_s,M,R_{eq}), split and operating-condition stratum. Keep errors in SI units at the reference and estimator interfaces; percent error is derived, not directly authored.

Proposed research targets (not achieved):
- initial accuracy gate: median absolute relative error <=5% and P95 <=10% for each parameter on held-out operating conditions;
- stretch target: median <=2% and P95 <=5% for each parameter;
- at least 30 independent holdout operating points per published geometry/load/thermal stratum before publishing a stratum-level P95 claim;
- report P99 and maximum even when the primary target is met; rare catastrophic estimator failures must not disappear inside a percentile.

Reference-instrument calibration must cover the full run window. Report instrument uncertainty, estimator uncertainty and their combination separately; do not count repeated samples from one stationary run as independent operating points.

## 5. Excitation overhead and power quality

Compute perturbation energy overhead:
[
r_{exc}=100\frac{E_{perturbation}}{E_{useful-transfer}}.
]

The reported percentage must be recomputed from the two measured energy quantities. Proposed target: <=0.5% of useful transfer energy. Also report injected voltage/current ripple, peak current/voltage delta, spectral sidebands, acoustic/noise impact where relevant, EMC, switching loss and the thermal effect. A low estimator error is not a win if identification excitation disrupts useful transfer or protection margins.

## 6. ZVS and hard constraints

Compute ZVS coverage from raw counts, never from a self-reported percentage:
[
C_{ZVS}=100\frac{N_{ZVS}}{N_{observations}}.
]

Proposed target: >=99% over declared normal-operation observations. Report switching-limit violations and protection trips as hard outcomes; any violation of device SOA or safety/protection limits rejects the physical run, regardless of mean efficiency or estimation accuracy.

## 7. Latency and dynamics

Report p50/p95/p99 estimator latency, convergence time after a step in gap/load/alignment, and stale-estimate age. Compare p99 latency to a predeclared control deadline and to the actual dynamics of the test fixture. Do not use an arbitrary fixed millisecond target across 20 kHz and 85 kHz hardware before the system's sample/control rates are known.

## 8. Observability, identifiability and OOD behavior

Build held-out cases that vary gap, lateral offset, angular offset, load, coil temperature, ferrite/material lot and receiver design independently and jointly. Measure whether the estimator can distinguish these causes or merely fit a correlated proxy. Include:
- same mutual inductance reached through different gap/offset combinations;
- same input impedance reached through different load/temperature combinations;
- parameter drift from component heating and aging;
- asymmetric/partial receiver faults and measurement noise;
- unseen geometry, material and load combinations.

When parameter uncertainty or identifiability leaves the qualified envelope, the estimator must not be used as an unchecked control input. It must trigger the declared REDUCE_POWER/DEENERGIZE/HOLD_LAST_SAFE response.

## 9. Security and safety separation

Fast self-sensing is not permissionless energization. The estimator is never the receiver-authority decision maker. A qualified architecture keeps:
- authenticated session and receiver identity;
- direction/power-class authorization and anti-replay freshness;
- independent current/voltage/temperature protection interlocks;
- configuration/firmware integrity evidence;
- fail-safe response to sensor disagreement or unidentifiable state.

A "communication-free control" claim applies only to the secondary telemetry dependency explicitly measured in the run manifest. It must not be rewritten as "no communications or authentication needed."

## 10. Claim ceiling

The Zakerian paper is an experimental laboratory control method, but the indexed record does not specify its transfer power, operating frequency, exact gap, numeric parameter-error distribution, latency, perturbation energy or absolute efficiency. No extrapolation to 6/11/30 kW EV ratings is valid until same-platform scaling experiments produce matched evidence. The 50 W reflected-impedance paper remains a separate low-power comparator.
