# Conveyance state fusion benchmark v0.1

**Status:** event-dataset admission and leakage-validation protocol. No detector is trained or scored in this crate; no field event corpus is presently qualified.

## 1. Research evidence and scope

### Electrical-current monitoring (2026)

Zhang et al., *Measurement* 272 (5 May 2026), article 121117, DOI 10.1016/j.measurement.2026.121117, propose electrical measurement for conveying pipeline pressure characterization and operating-condition identification. Their abstract reports more than 96.4% overall recognition using drive-current signals for no-load, light-load, blockage and water-accumulation states. They report a current-pressure dynamic mapping, improved variational mode decomposition and wavelet processing.

**Critical transfer limit:** the authors specifically scope their model to negative-pressure very dilute-phase pneumatic conveying, with solid-to-air mass flow ratio significantly below unity. Their setup does not establish performance on coarse, abrasive hard-rock fragments, dense plugs, long underground hoses or changing mine geometry. The result is a strong current-only baseline to reproduce, not a target we can claim to exceed without matched data.

Source: https://doi.org/10.1016/j.measurement.2026.121117

### Pressure-fluctuation early warning (2025)

Mittal & Mallick, *Powder Technology* 464 (October 2025), article 121243, DOI 10.1016/j.powtec.2025.121243, study pressure fluctuations near blockage in dense-phase powder conveying. Their work reduces conveying air flow toward blockage at high solids loading ratios (>70), using fly ash and a fine white powder with median particle sizes 30 and 55 micrometres. Near the blockage boundary, their computed signal features vary less along the line than under more stable conditions.

**Critical transfer limit:** this is a dense-phase fine-powder regime, not mine-scale angular hard-rock fragments.

Source: https://doi.org/10.1016/j.powtec.2025.121243

### Physical fragmentation/transport context (2026)

Raju & Esterhuyse's seven-month South African undercutting/vacuum extraction trial reports material removal limiting sustainable cutting during evaluated productive periods. Hose friction, distance and rock density contributed to residual fines that affected gripper stability and navigation. This motivates integrating generation context, transport signals and spatial residual evidence, while recognizing that the paper does not publish a synchronized event-level sensor dataset.

Source: https://doi.org/10.1007/s42461-026-01722-w

## 2. Paradigm shift

Move from single-sensor thresholding to **physics-constrained, multi-signal state estimation with sensor-health and out-of-domain hypotheses represented explicitly**.

A useful detector must distinguish different causes that can produce similar pressure signatures:
- developing rock restriction or complete blockage;
- high solids loading, stratification and dune/sediment formation;
- filter/cyclone loading;
- water accumulation;
- air leakage or hose disconnection;
- blower/VFD degradation;
- pressure tap fouling, sensor drift or disconnection;
- airflow sensor failure;
- commanded startup/shutdown transients;
- unknown material, path or regime outside the validated envelope.

Current is a valuable non-intrusive signal. It is not a universal replacement for pressure, airflow or mass measurements. A current-only model may confuse blower/VFD degradation with process resistance unless it observes sufficient electrical and process context.

## 3. Executable v0.1 contract

The standalone Rust crate `symtropy-conveyance-state-bench` admits:
- sensor-channel identity, type, units, physical location, instrument, calibration, sample rate, source and clock domain;
- complete train/calibration/test group declarations;
- held-out site/campaign/path/machine/material/time/parent-rock dimensions;
- synchronized observation windows with no overlapping windows inside one group;
- an explicit OBSERVED / DEGRADED / MISSING state for every registered channel in every window;
- source/quality references for observed and degraded samples; an explicit reason for missing or degraded data;
- independent event labels, time boundaries, detection deadlines, event-clear state, evidence method, source and adjudicator;
- distinct process-event and sensor-fault labels, with compatible independent evidence methods;
- at least one critical seeded restriction/blockage event in the held-out test partition.

It fails closed on cross-partition group leakage, held-out-dimension overlap, missing channel states, clock mismatch, overlapping windows, invalid event timing and model-derived labels.

## 4. Frozen model tournament

- **B0 — static pressure threshold:** simple pressure alarm.
- **B1 — pressure trend/statistics:** differential pressure, rate-of-change, spectral/entropy features.
- **B2 — current-only:** reproduce Zhang et al.'s current-based baseline as closely as the regime permits.
- **B3 — pressure + airflow:** two process signals.
- **B4 — current + pressure + airflow:** electrical/process fusion.
- **B5 — B4 + material-generation context:** PSD, particle morphology, material cohort, feed/recovered mass and moisture.
- **B6 — physics-informed regime model:** calibrated flow/pressure response features or CFD–DEM-derived features, with simulation and calibration data kept separate from test truth.
- **B7 — sensor-health/OOD/abstention model:** B5/B6 with missing-sensor diagnosis, out-of-domain detection and explicit UNKNOWN.
- **B8 — Symtropy/Symthaea:** same frozen records, prediction horizons, labels and splits as all baselines.

Freeze feature transforms, thresholds, calibration, prediction horizon and code digests before the untouched test partition is evaluated.

## 5. Metrics

Headline sample accuracy is insufficient. Report:
- event precision/recall and F1 by event class;
- critical blockage recall and missed critical events;
- false alarms per operating hour and nuisance alarms per shift;
- detection lead time relative to independently witnessed onset;
- onset-time error, event-duration error and clear/recovery-time error;
- per-configuration, per-material, per-path and per-sensor-availability worst subgroup;
- probability calibration, Brier/log loss and event-level interval coverage where probabilistic predictions are produced;
- OOD false acceptance and abstention coverage/risk curves;
- explicit signal-loss degradation for pressure, current, airflow and mass channels;
- compute latency, memory and model maintenance cost.

Compare nuisance alarms only at matched event recall, and show the full precision-recall / false-alarm trade-off rather than selecting one favourable threshold.

## 6. Holdouts and sample integrity

Use complete groups for splits. The v0.1 manifest supports held-out dimensions:
- SITE
- CAMPAIGN
- PATH_CONFIGURATION
- MACHINE_CONFIGURATION
- MATERIAL_COHORT
- TEMPORAL_BLOCK
- PARENT_ROCK_BLOCK

Train, calibration and test group IDs must be disjoint. Values in every declared holdout dimension must also be disjoint between all three partitions. Random adjacent windows are not an external-transfer headline. Overlapping windows within a group are rejected to reduce near-duplicate leakage.

The critical seeded blockage corpus must be present in the held-out test. This is a conformance requirement for the event benchmark, not a safety certification.

## 7. Hypotheses / target gates (not achieved results)

- H1: at matched critical-event recall, multi-signal fusion yields at least 20% fewer nuisance false alarms than the strongest single-signal baseline on held-out configurations.
- H2: adding material-generation context improves transfer across particle-size/morphology and density shifts.
- H3: an explicit sensor-health state reduces false blockage attribution under pressure-tap or airflow-sensor faults.
- H4: OOD abstention improves selective risk under unseen installed paths and material cohorts.
- H5: current-only monitoring retains useful performance when pressure sensing is unavailable, but no universal pressure-sensor replacement claim is made.

The 20% target is a preregistered hypothesis. Determine sample size from pilot event rates, campaign clustering, expected false-alarm exposure and confidence-interval width. Do not claim superiority from a synthetic conformance fixture.

## 8. Adversarial and negative controls

- same pressure signature produced by blockage and blower degradation;
- pressure tap fouling / drift / disconnection;
- airflow sensor drift or loss;
- current-sensor loss and VFD fault;
- filter loading and water accumulation;
- startup/shutdown and commanded transient;
- stratified flow and dune deposition before complete blockage;
- high solids loading without blockage;
- simultaneous sensor fault and physical restriction;
- clock skew, unsynchronized sensor windows and overlapping windows;
- no-event stable operation and long operating periods without alarms;
- domain shift in particle size, morphology, density, moisture, hose length, bends, vertical rise and filter state;
- labels generated by model predictions or operator reports alone;
- adjacent windows, specimens or campaigns leaking across train/calibration/test.

The structural negative-control gate is 100% detection of planted group, timing, provenance, channel-coverage and label-authority violations. It is not a physical detection-rate claim.

## 9. Integration ownership

- #3864/#3876 own measured fragmentation, installed transport capacity and mass-balance windows.
- #3891/#3910 own size-conditioned particle morphology measurement.
- #3877/#3911 (this issue/PR) own event-level sensor fusion dataset admission and evaluation protocol.
- #3888/#3890 own spatial floor residue and support-footprint evidence.
- #3863/#3846 own cutter-health and wear context.
- #3812/#3814 own accepted output and full-cycle metrology.
- #3825 owns energy/resource accounting.
- #3853 owns strategy eligibility and Pareto comparison.

## 10. Non-claims

The crate does not train a blockage detector, compute pressure or flow from sensor inputs, determine an operating threshold, initiate a clearing action, or command equipment. It does not qualify explosion protection, ventilation, dust control, pressure equipment, hose integrity, mine communications or worker safety. Synthetic fixtures are conformance data only. Independent field labels and prospective validation on the intended hard-rock path are required before operational use.
