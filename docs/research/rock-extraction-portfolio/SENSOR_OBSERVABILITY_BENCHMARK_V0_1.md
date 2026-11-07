# Sensor observability and fault-pair diagnosability benchmark v0.1

**Status:** declared-layout structural diagnosability evaluator with synthetic response envelopes. No physical fault signatures, sensor installation or rock-conveying diagnosability have been qualified.

## 1. Why detection accuracy is insufficient

A closed-set classifier can score well on a fixed rig while failing when:
- operating point, material or particle-size distribution changes;
- a filter gradually loads;
- blower degradation mimics a transport restriction;
- pressure impulse lines foul;
- one transducer drifts or becomes disconnected;
- sensor timing or calibration changes;
- two physical conditions create overlapping signal distributions.

The question is not only whether a model predicts a label. It is whether the installed measurement layout contains enough independent information to distinguish the fault hypotheses at all.

## 2. Current research basis

1. **Borg, Refalo & Francalanza (2026), Measurement 258A, 118992, DOI 10.1016/j.measurement.2025.118992.** Their measurement-based pneumatic fault-monitoring study tests three system-complexity levels and seven fault-profile levels. Pressure-drop statistics and cycle time performed usefully across tested compressed-air configurations. The study supports cross-system evaluation; it is not coarse hard-rock conveying.
   https://doi.org/10.1016/j.measurement.2025.118992

2. **Zhang et al. (5 May 2026), Measurement 272, 121117, DOI 10.1016/j.measurement.2026.121117.** Electrical-current monitoring reports >96.4% recognition across several states, including blockage and water accumulation, in very dilute-phase conveying (solids-to-air ratio significantly below one). This is a current-only baseline worth reproducing in its own domain, not a hard-rock/dense-flow performance claim.
   https://doi.org/10.1016/j.measurement.2026.121117

3. **Zhang et al. (online 28 September 2026), Digital Chemical Engineering 21, 100345, DOI 10.1016/j.dche.2026.100345.** The review separates adaptive process-monitoring strategies from robust cross-domain modelling, emphasizing that IID assumptions fail under aging and changing operating conditions. It supports separate measurement of known-fault generalization and unknown-fault rejection.
   https://doi.org/10.1016/j.dche.2026.100345

4. **Kościelny & Sztyber-Betley (2025), Mechanical Systems and Signal Processing 236, 112979.** Explicit pairwise fault isolability is a recognized diagnostic-system property. It should be measured directly rather than hidden in one confusion matrix.
   https://doi.org/10.1016/j.ymssp.2025.112979

5. **Achouch et al. (24 October 2026), Sensors 26(21), 6741.** A review of 131 conveyor-belt fault studies reports limited remaining-life prediction, degradation forecasting and principled multisensor fusion in that adjacent mechanical-conveyor literature. This is not pneumatic conveying, but it highlights why diagnosis, prognosis and deployment transfer must be evaluated separately.
   https://doi.org/10.3390/s26216741

## 3. Paradigm shift: structural diagnosability before model complexity

For each pair of states, ask whether at least one (or a declared number of) independent measurement principles produce non-overlapping response envelopes within the declared calibration domain.

A single pressure transducer may be highly sensitive to two different physical faults and still not distinguish them. Two pressure transducers on common impulse plumbing, common power or a shared calibration chain may fail together and cannot automatically count as two independent principles.

The layout evaluator reports:
- required physical/sensor fault pair;
- selected channels whose response intervals separate that pair;
- independent-principle groups represented;
- minimum required independent groups;
- IDENTIFIABLE, NOT_IDENTIFIABLE or INSUFFICIENT_EVIDENCE.

A synthetic structurally identifiable pair is not a physically qualified diagnosis. It only demonstrates the arithmetic and declared grouping contract.

## 4. State taxonomy

The v0.1 vocabulary includes:
- normal stable operation;
- startup/shutdown transient;
- high solids loading;
- stratified flow;
- dune deposition;
- partial restriction;
- developing blockage;
- complete blockage;
- water accumulation;
- filter/cyclone loading;
- air leak/disconnection;
- blower/VFD fault;
- pressure sensor fault;
- airflow sensor fault;
- current sensor fault;
- unknown/out-of-domain.

Each state is explicitly a PROCESS_STATE, SENSOR_FAULT or OUT_OF_DOMAIN. Sensor-fault states list the exact affected channel IDs. Those channels must be DEGRADED or MISSING in that state. A generic SENSOR_FAULT state that does not say which channel failed is not a useful fault hypothesis.

## 5. v0.1 executable contract

Standalone crate: `symtropy-sensor-observability-bench`.

### Layout records

Every channel identifies:
- channel/measurand/technique/unit;
- physical location;
- calibration reference;
- independence-principle group;
- dependency domain;
- material/path/geometry domain;
- raw source artifact reference.

A channel must bind to the exact parent material cohort and transport configuration. Independence groups represent reviewed common-mode/dependence assumptions; serial-number uniqueness is not enough.

### Response matrix

Every declared state × channel cell has one response record:
- VALID / DEGRADED / MISSING / UNKNOWN;
- bounded numeric interval only when VALID;
- exact unit and calibration domain;
- synchronized state measurement-window identity;
- raw source and adjudication reference;
- evidence role.

All channel responses within one state share a synchronized measurement-window reference. A response cannot use model estimates or operator descriptions as physical truth.

### Pairwise comparison

For response intervals A=[a-,a+] and B=[b-,b+], a channel discriminates only when:

`a+ < b-` or `b+ < a-`.

Intervals that touch or overlap do not discriminate. No midpoint/mean separation shortcut is allowed.

Each discriminating channel contributes its `independence_group_ref` at most once. The pair is IDENTIFIABLE only when the number of unique groups meets the required minimum. If below threshold:
- unexpected missing/degraded/unknown evidence or inadmissible model estimates => INSUFFICIENT_EVIDENCE;
- complete valid evidence with overlapping signatures / too few independent groups => NOT_IDENTIFIABLE.

A known unavailable channel that is explicitly declared faulted in a sensor-fault state is expected missing evidence; it is not itself a telemetry dropout. It still cannot count as a healthy measurement signature.

### Critical pair policy

The versioned layout profile specifies a default minimum independent-principle count for critical pairs. Every critical pair must meet or exceed that threshold. Pair-specific higher thresholds are permitted.

The evaluator does not assume that two is always enough. The decision owner must define the minimum according to consequence, common-mode analysis and available independent modalities.

## 6. Synthetic response matrix

The fixture contains:
- 7 channels;
- 6 independence-principle groups;
- 8 declared states;
- 8 required fault pairs;
- 56 state/channel response records.

Two pressure channels share one pressure-differential principle and count once. The pressure and airflow sensor-fault states mark their own affected channel degraded; an independent instrument-health diagnostic separates the synthetic sensor-fault signature. The OOD state has UNKNOWN response envelopes and must remain INSUFFICIENT_EVIDENCE.

Synthetic replay yields:
- normal vs developing blockage: 5 independent discriminating principles;
- normal vs filter loading: 3;
- developing blockage vs filter loading: 5;
- normal vs blower/VFD fault: 5;
- pressure sensor fault: 1 independent health principle;
- airflow sensor fault: 1 independent health principle;
- high solids vs blockage: 4;
- normal vs OOD: INSUFFICIENT_EVIDENCE.

These are fabricated interval relationships, not measured fault coverage or a recommended sensor layout.

## 7. Benchmark tournament

Run all models on exactly the same physical sensor layouts, event labels, fault pairs, material/path domains and held-out campaigns.

- **B0:** pressure-only baseline.
- **B1:** current-only baseline; reproduce the published 96.4% result only in the very-dilute domain with a matching apparatus and labels.
- **B2:** pressure + airflow.
- **B3:** pressure + current + airflow.
- **B4:** B3 plus filter differential pressure and independent gravimetric mass-flow reference.
- **B5:** B4 plus PSD/morphology/material context.
- **B6:** physics-constrained residuals, sensor-health and OOD hypotheses.
- **B7:** Symtropy/Symthaea fusion with calibrated uncertainty, selective prediction and abstention.

Use leave-one-material-cohort-out, leave-one-installed-path-out and leave-one-campaign-out evaluation. Do not randomly split adjacent windows from the same physical event across training and test.

## 8. Required performance metrics

- pairwise fault-pair isolability matrix;
- critical-pair NOT_IDENTIFIABLE and INSUFFICIENT_EVIDENCE rate;
- independent-principle count and common-mode failure sensitivity;
- known fault recall and pairwise mis-isolation cost;
- unknown/OOD false acceptance and abstention coverage;
- nuisance alarms per operating hour at matched critical-event recall;
- event detection lead time and missed-severity distribution;
- sensor dropout/degradation ablation;
- calibration, interval coverage and selective risk;
- cross-cohort, cross-path and cross-campaign transfer;
- installation, recalibration and maintenance burden.

Do not substitute overall accuracy or macro-F1 for a fault-pair isolation matrix.

## 9. Targets and acceptance gates

### Software conformance

- 100% rejection of missing/duplicate state-channel response cells;
- 100% rejection of a declared faulted channel reported as healthy;
- 100% enforcement of the versioned critical-pair minimum;
- 100% of channels sharing one declared independence group count as one principle per pair;
- 100% of unknown/OOD response states remain non-identifiable or insufficient, never forced into a known class;
- 100% rejection of mismatched units/domains or unsynchronized state windows.

These are frozen-case conformance requirements, not physical field accuracy.

### Physical/model tournament targets (hypotheses, not achieved results)

- At matched critical-event recall, at least 20% reduction in nuisance alarms against the best valid single-signal baseline.
- No critical pair accepted as identifiable unless all required independent principles have independently qualified response envelopes.
- Report false acceptance of OOD separately from abstention.
- Report pairwise missed-fault costs and detection lead-time distributions.
- Demonstrate transfer to at least one untouched material cohort and one untouched installed path before making any generalization claim.

Set final numerical thresholds only after the measurement-system study establishes label quality, reference uncertainty, event frequency and consequence.

## 10. Negative controls

- two pressure channels with one dependency group counted twice;
- shared pressure impulse line, supply or calibration chain treated as independent;
- blockage and filter loading with overlapping pressure-only responses;
- blower/VFD fault mimicking a restriction;
- high solids loading overlapping partial restriction;
- pressure/airflow sensor fault but faulty channel reported VALID;
- unexpected telemetry dropout incorrectly marked as known sensor failure;
- missing/unknown channel used as a discriminating signature;
- interval touch/overlap classified as separated;
- model-derived signatures treated as measurements;
- same source artifact reused for all response cells;
- channels within one state using different measurement windows;
- channel domain mismatch across material/path configuration;
- pair-specific threshold below layout critical-pair minimum;
- UNKNOWN/OOD state mapped to the nearest known class.

## 11. Next research beyond v0.1

1. **Coupled faults:** pair and higher-order simultaneous fault combinations; fault interaction may not equal the sum of individual signatures.
2. **Sensor-placement optimization:** minimum-cost placement under diagnosability, common-mode, maintainability and calibration-domain constraints. Pareto front only; no single arbitrary weighted winner.
3. **Active diagnosis:** choose additional observations/tests by sequential EVSI with real cost, risk and stopping criteria. No physical actuation is implied.
4. **Degradation prognosis:** interval-valued health trajectory and remaining-life distribution evaluated with censoring, competing risks and calibration.
5. **Robust transfer:** invariant physical residuals and group-conditional calibration over rock cohorts/path geometries; explicit negative-transfer controls.
6. **Causal signatures:** model fault propagation from physical equations, then validate causal responses against independent fault injections/field events.

## 12. Integration and claim boundary

- #3877/#3913: independently labelled event-level conveying state evidence.
- #3987/#3991: calibration, sampling, timing and uncertainty record authority.
- #3927: stage-resolved dry-solid flow and PSD.
- #3910: morphology.
- #3890: spatial residual/support evidence.
- #3917: experiment value/stopping.
- #3993: layout-level pairwise diagnosability.

This crate evaluates declared response envelopes and grouping assumptions. It does not prove field identifiability outside measured domains, optimize an installation, prescribe safe sensor placement or line operating conditions, or authorize mine operation. Physical claims require independently measured fault signatures, qualified sensors, common-mode analysis and held-out prospective fault campaigns.
