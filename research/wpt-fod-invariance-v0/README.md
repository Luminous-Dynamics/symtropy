# WPT foreign-object detection nuisance-invariance benchmark v0

**Status:** research protocol + executable synthetic metamorphic oracle  
**Parent:** [WPT-FO-RESILIENCE-001 #3496](https://github.com/Luminous-Dynamics/symtropy/issues/3496)  
**Implementation:** `tools/wpt-fod-invariance-oracle`

## 1. New research baseline

Yan, Xing, Liu & Sun (2026), “Power-Decoupled Metal Object Detection Using Four-Quadrant Flux-Coupling-Ratio Sensing for Wireless Power Transfer,” *IEEE Transactions on Instrumentation and Measurement*, DOI [10.1109/TIM.2026.3716472](https://doi.org/10.1109/TIM.2026.3716472).

The paper describes:
- four spatially arranged sensing quadrants;
- receiver-side contribution compensation;
- primary-side common-scaling suppression via four-channel normalization;
- an adaptive ordered-statistic CFAR threshold;
- an 85 kHz LCC-S experimental WPT prototype;
- object-free residual returning to its baseline band under tested primary input-voltage and secondary-load variation;
- stable residual behaviour under the tested fixed Tx–Rx misalignment conditions;
- sustained object-induced residual separation under the tested experimental conditions.

The abstract/source summary does not provide a fully auditable raw per-channel dataset, class-wise false-negative distribution, p99 latency, complete geometry matrix, temperature/ageing sweep or cross-generation coil test. Do not infer those results. The paper is a promising experimental method reference, not a deployment or safety qualification.

## 2. Paradigm shift: qualify invariance and sensitivity together

A sensing method must satisfy two competing requirements:

1. **Nuisance invariance:** changes that do not represent an object—such as a common change in primary voltage or receiver load—must not move the normalized object-free signature into the alarm region.
2. **Object sensitivity:** a localized object-induced field perturbation must remain distinguishable after the same normalization.

A detector that suppresses every signal change has perfect nuisance invariance but detects nothing. A detector that reacts to every signal change has high sensitivity but may false-alarm whenever the charging operating point changes.

The benchmark therefore evaluates both sides jointly, by operating cell, object class and held-out session.

## 3. Executable mathematical oracle and its exact limit

The Rust crate implements a generic metamorphic control for four non-negative synthetic channel magnitudes:

`N(x)_i = x_i / sum_j(x_j)`

The implementation first divides by the maximum channel to avoid overflow. The mathematical control tests:

`N(alpha * x) = N(x)` for every positive common scale `alpha`.

It then uses total-variation distance from a frozen object-free signature to verify that a deliberately localized synthetic quadrant perturbation remains distinguishable under the same common scaling.

**Important:** this is not the paper's exact 4Q-FCR equation, receiver compensation, sensor physics or OS-CFAR implementation. It is a generic property test for a declared normalization pipeline. The fixture threshold is synthetic; it is not a hardware detection threshold or safety limit.

Any real 4Q-FCR reproduction must implement the source-defined signal model, calibration and thresholding from the full paper and independently acquired raw data. The generic oracle cannot be used as a substitute.

## 4. Matched comparison lanes

Compare on identical object sets, coil/hardware, geometry, power, frequency and nuisance conditions:

| Lane | Method | Required distinction |
|---|---|---|
| A | Conventional reflected-impedance / threshold FOD | Baseline power/load sensitivity |
| B | Planar magnetic-inductance tomography (MIT) | Spatial reconstruction and material/localization limits |
| C | Four-quadrant flux-coupling-ratio sensing | Common-mode rejection and retained object sensitivity |
| D | Thermal sensing | Detection delay relative to object thermal time-to-limit |
| E | Hybrid sensing | Fusion calibration, added sensors, failure dependencies |
| F | Active demagnetization after qualified detection | Thermal suppression, auxiliary energy, coil losses and residual risk |

Do not compare different power classes or meter boundaries as if they were matched. The active DeMag result from the 2026 literature reports a 25 mm coin temperature-rise reduction from 56.6 °C to 13.7 °C after 600 seconds, but its reported efficiency belongs to its own declared test system and power points. It is a mitigation comparator, not evidence that a detector can safely identify every object.

## 5. Nuisance-variable matrix

Freeze an operating envelope for each hardware class. Include:
- primary input voltage and source impedance;
- receiver load, battery-emulator voltage and charge-state corners;
- Tx–Rx gap and X/Y/yaw misalignment;
- receiver geometry/class and coil generation;
- temperature, thermal state and sensor drift;
- frequency / compensation topology only within supported, separately qualified regimes;
- motion and handover for dynamic-capable hardware.

For every nuisance cell, collect both object-free and object-present conditions. Preserve raw quadrant waveforms, synchronized reference data, calibration, controller version, and object identity/position. Do not train and test on neighbouring frames from the same physical session.

The paper's fixed misalignment experiment is a baseline. Dynamic motion, held-out geometries, new receiver revisions, temperature corners and ageing are separate challenges, not results that can be inferred from the paper.

## 6. Metamorphic properties and negative controls

The synthetic oracle covers:
- common positive scaling: normalized object-free signature must remain invariant;
- common scaling with localized perturbation: object signature must remain detectable;
- object-free no-trigger;
- quadrant permutation: ordering must not be silently canonicalized;
- unknown/faulted quadrant: decision must become NOT_EVALUABLE;
- invalid negative, non-finite or all-zero sensor magnitudes: reject;
- extreme finite magnitudes: normalize without overflow.

Physical benchmark negative controls should add:
- voltage/load changes with no object;
- fixed and moving misalignment with no object;
- object present at held-out material, position, size and orientation;
- one quadrant stuck, saturated, missing or gain-drifted;
- clock/phase skew and sensor cross-talk;
- object movement during transfer;
- multiple objects and ferrite/shield-edge placement;
- unseen material or geometry requiring OOD abstention.

## 7. Metrics: never hide a weak operating cell

Report:
- object-free residual drift by voltage/load/misalignment/temperature cell;
- object-present residual separation and its lower confidence bound by object class and location;
- class-wise FNR, FPR, precision and recall;
- p50/p95/p99 detection latency;
- localization error p50/p95 and confidence calibration;
- OOD and sensor-fault abstention;
- sensitivity to quadrant gain, timing, phase and calibration drift;
- detector power/energy overhead and auxiliary sensing power;
- thermal ground-truth response and time-to-limit;
- active-mitigation auxiliary energy and total system efficiency at matched meter planes.

An aggregate confusion matrix is not enough: publish cell-level denominators and worst-cell results. A model cannot compensate for a dangerous material/position corner with high performance elsewhere.

## 8. Statistical qualification

Use independent object identities and complete sessions as held-out groups. Random-frame splitting is invalid for safety qualification. Report the independent sampling unit and cluster/block confidence interval for FNR/FPR.

For zero observed false negatives, the one-sided 95% upper bound is approximately (3/n) for (n) independent positive trials. Around 30,000 independent zero-miss trials are needed for an upper bound near (10^{-4}); 10,000 zero-miss trials correspond to an upper bound around (3\times10^{-4}). Correlated frames do not count as independent trials. Use dependence-aware confidence intervals as specified in #3692.

The proposed #3496 target of 10,000 cases is a useful initial challenge size, but its confidence bound must be reported rather than described as proof of zero risk.

## 9. Proposed gates

Programme targets, not regulatory thresholds or demonstrated results:
- zero false negatives in the frozen known-truth corpus;
- 100% NOT_EVALUABLE on declared sensor-fault / unsupported regimes;
- object-free residual distribution remains in its declared no-object band across the full qualified nuisance envelope;
- object-present separation maintained at every admitted object/position cell;
- active mitigation requires calibrated thermal ground truth and auxiliary energy accounting;
- no active transfer with unknown object identity, unresolved localization or unqualified thermal prediction;
- all metrics tied to object/session-held-out evidence and raw artifact digests.

Use the existing #3496 qualification contract and its safety-policy validator. This crate only supplies an independent mathematical metamorphic control; it does not replace the FOD validator or a physical FOD campaign.

## 10. Safety and claim ceiling

Synthetic software fixtures, published-source comparison and protocol design only. No powered metal-object experiment, active demagnetization build, high-power coil modification or live-road test is authorized. Any physical campaign must be performed in an independently approved isolated facility with appropriate protection and thermal ground truth.

A passing oracle demonstrates only its stated mathematical property. It does not establish paper reproduction, object detection performance, FOD safety, thermal mitigation efficacy, standards compliance or superiority to industry.
