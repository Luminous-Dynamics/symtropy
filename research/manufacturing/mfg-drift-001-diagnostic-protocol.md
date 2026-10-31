# MFG-DRIFT-001 — Manufacturing twin drift diagnosis protocol

**Work item:** [MFG-DRIFT-001 #3166](https://github.com/Luminous-Dynamics/symtropy/issues/3166).  
**Implementation branch:** feat/manufacturing-drift-diagnosis-20261031.  
**Related programmes:** #3121 (composition VVUQ), #3164 (declared change impact), #2995 (measurement uncertainty and calibration), #2963 (time-dependent process capability), #3025 (MTConnect/QIF interoperability), #2976 (bounded intervention).

## 1. Research hypothesis

Observed disagreement between a manufacturing twin and a physical system has multiple possible causes. A residual alone does not identify which one is responsible.

The programme will test whether a transparent diagnostic can distinguish, within a declared observability contract:
- primary sensor bias/drift;
- real process/context movement outside the model's validity envelope;
- in-domain model discrepancy;
- interface unit/semantic mapping defects;
- clock and sequencing defects;
- multiple simultaneous faults;
- insufficient evidence / unqualified measurement systems.

The system must preserve uncertainty and abstain where the signatures are not identifiable.

## 2. Why diagnose before adapt

A naive online calibration loop can make a twin fit a biased sensor. A process shift can be hidden by continuously adjusting model parameters. A clock or unit conversion defect can look like a physical residual. Therefore, this research keeps diagnosis separate from recalibration and adaptation.

Relevant current research:
- NISTIR 8620 (2026) identifies interoperability, VVUQ and cybersecurity as persistent manufacturing twin challenges: https://www.nist.gov/publications/digital-twins-workshops-summary-report
- Lugaresi, Dhore & Matta (Procedia CIRP, 2026) explore online validation and parameter calibration by comparing physical and simulated event sequences/performance indicators: https://doi.org/10.1016/j.procir.2026.03.284
- A 2026 semantic-drift evaluation compares digital-twin framework capabilities for managing semantic drift: https://doi.org/10.1016/j.future.2025.108240
- ISO/AWI 23247-8, approved and under development, covers manufacturing digital-twin VVUQ throughout lifecycle stages: https://committee.iso.org/standard/94858.html

Our first reference differs from online calibration work: it does not update model parameters. It tests whether the evidence can distinguish fault families before any owner considers recalibration or requalification.

## 3. Observability contract

A single residual series is insufficient to distinguish sensor bias from model discrepancy. The minimum synthetic contract therefore contains:
- a primary sensor;
- an independently qualified reference sensor;
- the twin prediction with exact model/configuration identity;
- the process/context input and its validity envelope;
- the interface source value, declared mapping and received value;
- source time, expected transport latency and received time;
- calibration status and stale/replay flags.

If the independent reference channel is missing or unqualified, the sensor-vs-model distinction must abstain.

For real data, every channel must retain its raw source value, unit, source timestamp, sequence/clock context and calibration/measurement-system receipt. The diagnostic must never replace raw source observations with corrected values.

## 4. Diagnostic signatures

| Fault family | Required signature | Output |
|---|---|---|
| No-fault control | Primary and reference agree; model residual below threshold; context in envelope; mapping/clock valid | IN_DOMAIN_NO_DETECTED_DRIFT |
| Primary sensor bias | Primary disagrees with independent reference; reference agrees with twin; input in-domain | PRIMARY_SENSOR_BIAS |
| Process/context shift | Measured process input leaves declared model envelope | OUT_OF_VALIDITY_DOMAIN |
| Model discrepancy | Reference disagrees with twin while primary/reference agree and input remains in-domain | MODEL_DISCREPANCY |
| Interface mapping | Received mapped value disagrees with source value transformed under declared scale/offset | INTERFACE_MAPPING_FAULT |
| Clock skew | Received time differs from source time plus declared transport latency beyond tolerance | CLOCK_SKEW |
| Multiple signatures | Two or more distinct fault signatures occur together | UNRESOLVED_MULTIPLE_FAULTS |
| Missing/invalid observations | Required independent channels absent, non-finite or below minimum window size | INSUFFICIENT_EVIDENCE / INSUFFICIENT_SAMPLE_COUNT |
| Invalid calibration | Measurement system is not qualified for the window | MEASUREMENT_SYSTEM_UNQUALIFIED |
| Stale/replayed input | Observation sequence is stale or replayed | DATA_INTEGRITY_FAULT |

These are deliberately simple known-truth signatures, not a general root-cause inference engine. A MODEL_DISCREPANCY label means the model disagrees with trusted references under in-domain inputs; it does not prove why the model is wrong.

## 5. Synthetic benchmark corpus

The first executable Rust slice has:
- no-fault baseline;
- primary sensor bias;
- out-of-domain process input;
- in-domain model discrepancy;
- mapping conversion fault;
- clock skew;
- multiple simultaneous faults;
- missing reference channel;
- unqualified calibration;
- stale/replayed data;
- insufficient window length;
- invalid thresholds;
- deterministic report replay.

Add progressive difficulty:
1. independent Gaussian noise at multiple SNR levels;
2. heavy-tailed outliers and isolated spikes;
3. gradual sensor drift and abrupt bias steps;
4. slow process drift versus abrupt context regime shift;
5. correlated primary/reference sensor errors;
6. calibration uncertainty and expired calibration;
7. asynchronous sampling, clock drift and event reordering;
8. nonlinear unit/frame transformations;
9. missingness that depends on machine state;
10. two or more simultaneous fault families.

Synthetic truth must be frozen and hidden evaluation seeds must not be used for threshold tuning.

## 6. Metrics and target gates

Report by fault family:
- detection recall;
- false attribution rate and confusion matrix;
- false-positive rate on no-fault controls;
- correct abstention rate on ambiguous/multiple-fault cases;
- detection delay for gradual/abrupt drift;
- sensitivity to reference-channel uncertainty and calibration scope;
- performance under sample-count/missingness/noise changes;
- mapping and clock residual detection precision/recall;
- in-domain vs OOD discrimination;
- deterministic replay and independent-oracle agreement.

First gates:
- 100% classification on frozen noiseless single-fault controls;
- 100% abstention on deliberately non-identifiable/multiple-fault fixtures;
- zero primary-sensor bias mislabeled as model discrepancy when an independent qualified reference identifies the sensor disagreement;
- zero model-discrepancy attribution when process inputs are explicitly outside the model validity envelope;
- zero model adaptation or recalibration performed by this diagnostic reference.

Never collapse fault-family recall, false attribution and abstention into one accuracy score.

## 7. Baseline comparisons

Compare:
1. raw residual threshold only (deliberately weak);
2. primary sensor vs prediction only (cannot distinguish sensor bias from model discrepancy);
3. primary sensor vs independent reference (measurement-health baseline);
4. process-envelope-only OOD detection;
5. mapping/clock invariant checks;
6. the composed diagnostic signature engine;
7. where data and licensing permit, online validation/calibration methods such as the 2026 discrete-event simulation approach.

Compare only with matched observability channels, validity envelopes and data splits. An online calibration method is not an equivalent baseline if it consumes additional channels or automatically adapts model parameters.

## 8. Integration and authority boundary

The diagnostic produces evidence and a disposition:
- IN_DOMAIN_NO_DETECTED_DRIFT;
- a specific fault signature;
- OUT_OF_VALIDITY_DOMAIN;
- MEASUREMENT_SYSTEM_UNQUALIFIED;
- DATA_INTEGRITY_FAULT;
- INSUFFICIENT_EVIDENCE / UNRESOLVED_MULTIPLE_FAULTS.

It does not modify calibration, update a twin, change an operating envelope, authorize an intervention, release product or actuate a machine. A diagnosis can trigger a request to the existing owner for calibration review, model VVUQ, mapping correction or change-impact analysis. Requalification and authority remain separate.

## 9. Claim ceiling

The first reference validates transparent diagnostic signatures on synthetic known-truth data. It does not detect actual drift in a production plant, establish physical root cause or prove a twin valid. Real validation requires independently qualified reference sensors, rights-cleared holdout campaigns and controlled fault-injection or independently adjudicated ground truth.
