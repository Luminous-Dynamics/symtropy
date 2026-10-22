# LAPTOP-TWIN-001-V1 — Evidence-bounded laptop thermal twin baseline

**Status:** deterministic synthetic baseline contract; no physical laptop calibration has been completed  
**Parent:** [LAPTOP-TWIN-001 #2697](https://github.com/Luminous-Dynamics/symtropy/issues/2697)  
**Cross-stack workload and receipt owner:** [Symthaea LAPTOP-ARCH-001 #8446](https://github.com/Luminous-Dynamics/symthaea/issues/8446)  
**Benchmark date:** 2026-10-22 · **Version:** 1.0

## 1. Deliberately narrow first milestone

The initial baseline is a one-node, first-order RC thermal response. It is meant to prove deterministic trace ingestion, units, model identity, golden outputs and negative-control behavior before fitting a higher-order model.

It predicts an abstract effective node temperature from a supplied power trace. It does **not** predict laptop input power, fan curves, CPU junction temperature, performance, or safe operating limits. A measured node can be mapped to a CPU/GPU/surface sensor only after its location, offset, workload and configuration are documented. It is not a replacement for firmware thermal protection or a safety certification.

The synthetic reference trace is only a test oracle. Its parameters are invented for hand-checkability, not identified from a real laptop. It must never be used to fit, rank, or claim the thermal performance of a commercial device.

## 2. Exact analytical model

For interval (k), assume constant effective heat input (P_k), ambient temperature (T_{a,k}), and interval duration (Delta t_k > 0). Parameters are thermal resistance (R_\theta > 0) in K/W and effective thermal capacitance (C_\theta > 0) in J/K. The time constant is (\tau=R_\theta C_\theta).

[
T_{k+1}=T_{a,k}+R_\theta P_k+
(T_k-T_{a,k}-R_\theta P_k)e^{-\Delta t_k/(R_\theta C_\theta)}
]

This is the exact step response for the declared first-order, piecewise-constant assumptions; it is not a discretization of an arbitrary real laptop. The golden fixture uses (R_\theta=0.8\ K/W), (C_\theta=120\ J/K), (T_a=25\ ^\circ C), initial (T_0=25\ ^\circ C), and ten-second intervals. Expected values are rounded to six decimal places for comparison; the implementation should retain floating-point precision internally.

The test trace power values are model inputs. If a future predictor estimates power from workload features, that is a separate model requiring a separately frozen target, train/validation/held-out split and reported error. When externally measured power is available, input energy is a trace integral, not a twin prediction.

## 3. Baselines and sequence

1. **Ambient/static equilibrium baseline:** predicts (T_a) or (T_a+R_\theta P) from the stated baseline variant. It deliberately ignores transient lag.
2. **One-node RC baseline:** uses the equation above with preregistered parameters; do not tune parameters against the golden fixture.
3. **Calibrated Symtropy candidate:** only after real trace ingestion is implemented, fit using training sessions and report held-out sessions separated by physical run/session and workload. Do not randomly split neighboring samples from one thermal run between training and test.

Keep prediction, simulation, and measurement provenance distinct. The initial milestone can demonstrate arithmetic and fail-closed inputs on synthetic traces; it cannot demonstrate accuracy against real laptops.

## 4. Input/trace identity

A trace must bind:
- exact device/configuration digest and physical-unit token when measuring a device;
- BIOS and embedded-controller versions, OS/kernel, driver/runtime versions, and workload/scorer hashes;
- sample interval and timebase, power source and mode, display state, battery state/health where relevant;
- ambient temperature, initial thermal state, sensor identity/location, and calibration provenance;
- source trace hash, model version/parameters, prediction output hash and evidence status.

Use the versioned receipt from Symthaea LAPTOP-ARCH-001 for shared evidence identity. Do not create a second global evidence ontology here. Missing sample timing, units, model parameters, configuration match or provenance must yield invalid/not-qualified, never default-zero values.

## 5. Evaluation metrics and proposed gates

On later real, held-out traces report temperature MAE/RMSE, maximum absolute error, bias by temperature band, 5/30/60-minute sustained throughput and p95 latency error, whole-device energy error only where the power model is independently qualified, and empirical coverage of prediction intervals. Report residuals separately by AC/battery mode, workload, ambient conditions and initial thermal state.

Initial candidate gates from #2697 are proposals, not achieved results: median absolute power error <=10% on calibrated whole-device traces; held-out 30-minute throughput error <=10%; peak temperature absolute error <=3 °C on comparable sensor points; nominal 90% prediction intervals cover 85–95% of held-out observations; beat the simple RC baseline on at least two predeclared metrics without violating a safety/thermal hard constraint. Freeze the split and gates before tuning.

If no physical traces exist, label every result synthetic and make no claim of laptop prediction accuracy or superiority. A candidate differing in SKU, chassis, cooling, BIOS/EC, OS/runtime or sensor topology is out-of-distribution until requalified; the twin must abstain rather than silently transfer parameters.

## 6. Industry comparison

Use the current [MLPerf Client](https://mlcommons.org/benchmarks/client/) standard where the exact model/runtime/hardware path is supported for end-to-end client LLM and agentic workload quality/latency. Pair those task outcomes with laptop-specific energy and thermal traces; do not substitute NPU TOPS for measured task utility. Independent [Framework Laptop 13 Pro reviews](https://www.tomshardware.com/laptops/framework-laptop-13-pro-review) illustrate why chassis cooling, sustained power settings and battery-test methodology affect outcomes. Published reviews help select reference devices and hypotheses, but are not trace-level calibration data for a different OS or workload.

## 7. Golden fixture and required negative controls

See [rc_thermal_step_reference_v1.json](fixtures/rc_thermal_step_reference_v1.json). Its expected outputs are analytically generated, synthetic-only values.

Required fail-closed cases before implementation is considered useful:
- non-positive interval, thermal resistance or capacitance;
- negative / non-finite power, missing units or non-finite temperature;
- missing timestamps or inconsistent sample intervals without an explicit resampling rule;
- a sensor dropout silently replaced with zero or ambient temperature;
- a BIOS/EC/device configuration digest that differs from the calibration receipt;
- an AC-to-battery transition passed to a model that has no qualified transition behavior;
- measured power mislabeled as model output, or synthetic power/temperature mislabeled as physical observations.

## 8. Claim ceiling

Passing golden synthetic arithmetic proves only that the implementation matches the equation and declared inputs. It does not prove that the equation is an adequate laptop model, that parameters are identified, or that it predicts energy, temperature, battery endurance, performance or safety on a physical device. Those require independent measured traces and held-out validation.