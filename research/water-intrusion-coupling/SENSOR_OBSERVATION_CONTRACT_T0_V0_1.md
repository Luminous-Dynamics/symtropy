# Sensor Observation Contract T0 v0.1

**Programme:** WATER-MIXING-SENSOR-OBSERVATION-CONTRACT-001 / [#5329](https://github.com/Luminous-Dynamics/symtropy/issues/5329)  
**Fault-aware comparator:** Li et al. (2026), [Toward trustworthy water quality monitoring under sensor unreliability](https://pmc.ncbi.nlm.nih.gov/articles/PMC13630539/)  
**Bayesian design parent:** [#5324](https://github.com/Luminous-Dynamics/symtropy/issues/5324), [draft PR #5325](https://github.com/Luminous-Dynamics/symtropy/pull/5325)  
**Evidence class:** synthetic observation-provenance and fail-closed boundary contract  
**Evidence ceiling:** no physical mass closure or sensor qualification

## 1. Research distinction

Current fault-aware monitoring research jointly detects sensor faults, reconstructs affected signals, and prioritizes possible contamination source regions. That is useful for operational diagnostics, but reconstructed values are not independent observations.

This T0 addresses the physical-evidence boundary:
- raw measurement;
- independently calibrated measurement;
- reconstructed/model-imputed/held-last value;
- missing value or missing channel record;
- faulted/stuck/drifting/spiky measurement;
- calibration state;
- clock synchronization;
- complete temporal window/tail;
- immutable provenance digests.

**A reconstructed value may be useful to a diagnostic model, but it must never silently enter physical mass closure as if it had been measured.**

## 2. Contract

Every experiment declares its required inlet and outlet channel counts. Every supplied channel record carries:
- unique ID and role;
- measurement state and sensor-health state;
- current/expired/missing calibration status;
- clock synchronization state;
- complete/truncated window state;
- positive sample count;
- SHA-256 digests for the raw record, calibration record and clock map.

The contract accepts measured_raw or measured_calibrated only when health is explicitly healthy, calibration is current, clock is synchronized, the window is complete, and all provenance digests are valid.

Reconstructed, model-imputed, synthetic-fill and held-last values are separately classified as inferred. They are not promoted into the measured channel count. Missing required channel records are detected from the declared inlet/outlet counts, not from the number of records that happen to be present.

## 3. Fail-closed disposition

The deterministic disposition priority is:
1. invalid provenance digest;
2. missing required channel record or missing measurement;
3. inferred/reconstructed measurement;
4. unresolved sensor fault;
5. calibration unresolved;
6. clock unresolved;
7. incomplete time window/tail;
8. complete measured boundary.

The ordering is conservative: an invalid provenance record cannot be masked by a lower-priority state. The output mass_closure_evidence_permitted means only that the synthetic record contract is complete; physical_mass_closure_claim_permitted remains false for every T0 case.

## 4. Frozen eleven-case controls

1. complete measured boundary;
2. reconstructed outlet;
3. omitted required outlet record;
4. confirmed fault;
5. suspected drift;
6. expired calibration;
7. uncertain clock;
8. truncated temporal tail;
9. stuck sensor;
10. invalid raw/calibration/clock digest;
11. all channels reconstructed.

Every negative control refuses measured-only boundary evidence. The reconstructed cases explicitly set imputation_counted_as_observed=false.

## 5. Comparison with fault-aware water-quality monitoring

Li et al. (2026) describe a unified pipeline for fault detection, localization, isolation, reconstruction and contamination source-region prioritization. Their experiments include missing, bias, spiky and drift faults, and show the practical value of fault/contamination differentiation under sparse sensing.

This T0 is complementary, not a reproduction or a direct SOTA comparison. It does not detect faults or reconstruct signals. It enforces an evidence invariant so reconstructed values cannot become measurements for physical closure. Future work should compare diagnostic utility and this measured-only guarantee side by side.

## 6. Benchmarks to add

- zero imputed-as-observed records;
- zero complete-boundary dispositions when a required sensor record is omitted;
- zero complete-boundary dispositions for confirmed/suspected sensor fault, invalid provenance, expired calibration, unsynchronized clock or truncated tail;
- permutation-invariant disposition and stable channel identity binding;
- fault-detection/localization/isolation benchmark compared with CAE/CNN-LSTM and simple robust-statistical baselines;
- report missed-fault, false contamination alarm, abstention, and diagnostic ranking separately from mass-closure eligibility.

Targets are not achieved physical results. A COMPLETE_MEASURED_BOUNDARY synthetic disposition is not proof that the calibration is correct or the junction physics is valid.

## 7. Evidence and safety boundary

The T0 checks declared records only; it does not independently verify raw sensor hardware, calibration process, clock hardware or measurement truth. Physical T1 work requires immutable raw traces, calibration traceability, a clock map, all required inlet/outlet channels, and full physical/sensor tail capture.

Use only a facility-approved closed-loop non-potable rig with approved tracer handling. Do not introduce sensor faults or tracers into a live drinking-water distribution system. No output establishes physical mass closure, potability, contamination, toxicology, SANS 241 compliance or release authority.
