# Ocean Environmental Measurement Pipeline v0.1

**Status:** software/metrology design and synthetic verification fixture only.  
**Owner issue:** [OCEAN-DC-ENVIRONMENT-001 #6391](https://github.com/Luminous-Dynamics/symtropy/issues/6391)  
**PR:** [#6322](https://github.com/Luminous-Dynamics/symtropy/pull/6322)  
**Date:** 2026-11-20

## 1. Paradigm shift: from sensor telemetry to decision-grade environmental evidence

A conventional telemetry dashboard answers: "What value did a sensor report?"

This pipeline answers a stricter sequence:

1. Which physical quantity and boundary does this value represent?
2. Which station, sampled feature, sensor and calibration produced it?
3. Does its calibration validity cover the exact observation window?
4. Are intake and outfall observations paired to the same water parcel, accounting for hydraulic residence time?
5. Are laboratory results tied to representative sampling, preservation, a chain of custody and a specific batch?
6. What is the uncertainty, including shared/correlated transport-lag effects?
7. Does the record close its narrowly declared measurement boundary?
8. Are there time gaps, below-detection observations or unresolved signals?
9. What decision is allowed—and, just as importantly, what remains UNKNOWN?

The central design rule is **no evidence laundering**: parsing, interpolation, a plausible number, a standards crosswalk or a clean dashboard must not upgrade synthetic, stale, uncalibrated, censored, rights-restricted or otherwise unknown evidence into a measured environmental fact.

## 2. Standards and interoperability boundaries

### 2.1 ISO 5667-10:2026 — representative wastewater/cooling-water sampling

ISO 5667-10:2026 (edition 3, published 18 November 2026) addresses sampling of industrial and urban wastewater and includes cooling-water contexts. The pipeline carries sampling method, collection window, preservation, laboratory batch, detection limit and custody metadata rather than treating a lab concentration as a context-free scalar.

This project does not claim that the synthetic flow-proportional-composite records are an ISO-compliant sampling campaign. A field sampling plan must be specified for the actual water matrix, discharge regime, temporal variability, analyte stability, access and applicable regulator conditions, and independently reviewed by qualified sampling personnel.

### 2.2 JCGM GUM and Monte Carlo propagation

The calculation path has two distinct layers:

- first-order uncertainty propagation from a declared measurement model;
- a deterministic Monte Carlo cross-check using a fixed seed and a common residence-time perturbation across outlet flow, temperature and concentration.

The v0.1 fixture uses 50,000 draws per interval, a two-sided 95% percentile interval and a proposed 5% maximum relative difference between Monte Carlo and first-order standard uncertainty. These are **project test parameters**, not requirements imposed by JCGM.

The Monte Carlo model assumes the explicitly recorded synthetic distributions. Real data require defensible input distributions, covariance, calibration reference effects, instrument drift, sampling variance, environmental variability and model validation. A disagreement above tolerance or a nonphysical draw rejects the receipt for model review; it is not fixed by widening the threshold after seeing results.

### 2.3 OGC SensorThings API 2.0

The bundle's normalized mapping targets OGC SensorThings API 2.0, published 4 November 2026. It uses:
- Thing, Location, Sensor, ObservedProperty, Datastream and Observation;
- Feature for sampled environmental features;
- ProximateFeatureOfInterest and UltimateFeatureOfInterest relationships; and
- SWE-Common resultType/resultEncoding, with units in the result structure.

The bundle is **not** yet an API payload. An adapter must validate against the current API schema and service `$metadata`, preserve phenomenonTime separately from resultTime, and record server capability/extensions. Legacy v1.1 mappings must be explicit versioned conversions, not silent aliases.

### 2.4 GOOS EOV and OBIS interoperability

The environmental catalogue maps relevant GOOS EOVs and provides candidate Darwin Core/OBIS biodiversity terms. These are separate from project-specific intake, outfall, pollutant-flux and thermal-plume measurements. GOOS specification sheets may change; pin the exact EOV version used per campaign. OBIS records require current provider/node guidance, rights, taxonomic and geospatial review, quality flags and sensitive-species location controls.

## 3. Measurement chain and equations

### 3.1 Hydraulic residence-time pairing

The synthetic fixture has a five-minute inlet window and an outlet window shifted by the declared 300-second residence time. The 15-second standard uncertainty of that transport lag is represented as one **common random variable**, not as three unrelated timestamp errors.

For any field system, residence time must be identified from a defensible tracer/step-response experiment or validated hydraulic model across relevant flow and operating states. One fixed nominal lag is inadequate if flow, temperature, mixing, bypass, recirculation or storage changes materially.

### 3.2 Thermal transfer

For the narrowly declared heat-exchanger boundary:

`P_th = rho × cp × Q_out × (T_out - T_in)`

The output expresses kW after unit conversion. The uncertainty model includes fluid density, heat capacity, outlet flow, paired temperatures and a common lag sensitivity:

`u_lag(P_th) = |dP_th / d(tau)| × u(tau)`

The synthetic calculation uses first-order partial derivatives and compares the result against Monte Carlo propagation. It does not infer a plume radius or ecological threshold from the heat-exchanger outlet alone.

### 3.3 Pollutant mass flux

For each named analyte:

`F = C × Q`

With concentration in mg/L and flow in m³/s, the numeric product is g/s. Convert to kg/day by multiplying by 86.4.

The net added load estimate is:

`F_net = C_out × Q_out - C_in × Q_in`

The calculation reports concentration and mass flux separately; dilution can reduce concentration while total mass release remains unchanged or rises. Below-detection samples require a declared censored-data treatment; v0.1 rejects them rather than substituting zero or half the detection limit.

### 3.4 Uncertainty and covariance

The output flow, temperature and concentration measurements share transport-lag error. Their contribution must use one common perturbation and the relevant temporal sensitivities. A common timebase, shared calibration reference, common laboratory batch, common flow standard or shared environmental correction may create other covariance terms. The current fixture declares instrument calibration domains independent within an interval and uses an explicit common-lag covariance term. Real data must establish, not assume, those relationships.

The net pollutant-load resolution ratio is `abs(F_net) / u(F_net)`. A value below one standard uncertainty is marked unresolved. It is not evidence of zero pollution, compliance or no impact. The confidence interval and measurement detection limits remain visible.

### 3.5 Flow closure is not a complete mass balance

The two-meter flow closure screen is deliberately restricted to a synthetic single-pass, once-through boundary with only intake and outfall streams, no side streams and zero storage change during each interval. It is a metrology consistency screen only.

A real system with purge, bleed, bypass, recirculation, cleaning, leakage, evaporation, storage/holdup change, multiple outlets or unmetered transfers must use a signed, multi-stream mass balance. The two-meter screen must fail closed rather than silently ignore a stream. It never establishes a discharge permit, water-quality or ecological result.

## 4. Evidence receipt and observation coverage

Each sample carries:
- unique sample and interval identity;
- exact UTC interval window;
- sensor and calibration identifiers;
- calibration-valid-from/to timestamps;
- signed per-sample sensor/logger clock offset (checked independently for every reading, not inferred from an interval-level health flag);
- calibration independence domain;
- QC and source-rights state;
- station identity and role;
- method, preservation, chain-of-custody and laboratory batch for lab observations;
- value, unit, standard uncertainty, detection limit and temporal sensitivity where applicable.

The verifier checks the absolute signed clock offset for each sample independently against the declared bound. A valid interval-level clock flag cannot mask one skewed sensor. It computes SHA-256 over the exact source-file bytes and emits byte length and hash scope. This proves input-byte identity only; it does not authenticate a sensor, logger, calibration certificate, laboratory or acquisition chain.

Observation coverage is reported independently:
- exact observed input-side durations;
- gaps and their start/end windows;
- overlap/out-of-order rejection;
- explicit statement that energy is summed over observed input-side intervals only.

Gaps are not interpolated and are never counted as zero load. This v0.1 reports coverage of inlet sampling windows; outlet observations are paired to delayed windows. Field programmes should additionally report sensor uptime, telemetry completeness, sample coverage, event exclusions and campaign/season coverage.

## 5. Proposed acceptance ladder

These are project gates for software and future field qualification—not standard-mandated thresholds and not achieved results.

| Gate | Proposed acceptance | Failure behavior |
|---|---|---|
| Schema/provenance | 100% of accepted records have IDs, UTC windows, station, calibration, rights and raw-data digest/custody | Reject or quarantine |
| Calibration | Validity covers the complete sample window | Reject |
| Time synchronisation | Every sensor/logger and laboratory record within the fixture's signed ±250 ms clock-offset test bound; field bound to be set from actual response time, transport lag and sensor dynamics | Reject / resynchronise |
| Parcel pairing | Input and output windows match validated residence time; transport-lag uncertainty declared | Reject until identified |
| Sample handling | Method, preservation, batch and custody complete | Quarantine lab result |
| Detection limit | Below-LOD values use a reviewed censored-data method | Do not set to zero |
| Numerical validity | No non-finite or non-physical draw; equations/unit conversions independently checked | Reject |
| Uncertainty agreement | Monte Carlo vs first-order standard uncertainty within the declared 5% fixture tolerance | Model review; no qualified receipt |
| Time coverage | All gaps and overlaps explicitly reported; no interpolation as default | Keep unknown duration visible |
| Boundary completeness | Two-meter flow screen only for the exact single-pass/no-side-stream condition | Require a full signed multi-stream mass balance |
| Environmental clearance | Requires field baseline, independent review, permit limits and ecological gates | UNKNOWN/BLOCKED until evidenced |

## 6. Synthetic fixture results (software fixture only)

The fixture contains three five-minute input windows; 15 temperature station observations, six flow observations, three intake/outfall analyte pairs and nine plume/reference chemistry observations. It uses one analyte for exercising the chain.

An independent numerical cross-check of 50,000 draws per interval found:
- first-order and Monte Carlo thermal standard uncertainty relative differences below 0.6%;
- net pollutant-load standard uncertainty relative differences below 0.4%;
- the simulated 95% net-load range crossed zero in all three intervals.

Interpretation: the synthetic net-load change is unresolved under the fixture's uncertainty model. It is neither a claim of increased pollution nor a claim of no pollution. These values are not measured ocean data and must never enter a real-site performance claim.

## 7. Reproduction

From the repository root, with a Rust 2024 toolchain and Cargo available:

```bash
cargo test --manifest-path tools/ocean-env-balance-verifier/Cargo.toml
cargo run --manifest-path tools/ocean-env-balance-verifier/Cargo.toml
```

The expected status explicitly remains:
- `SYNTHETIC_MEASUREMENT_BUNDLE_VALID_NO_SITE_OR_COMPLIANCE_CLAIM`;
- environmental compliance `NOT_ASSESSED`;
- ecological impact `NOT_ASSESSED`; and
- site clearance `UNKNOWN_BLOCKED`.

The verifier has authored regression/negative-control tests, but Rust compilation and execution have not yet been performed in this environment because `rustc` and `cargo` are unavailable. The numerical cross-check is independent JavaScript analysis, not a Rust test pass or CI result.

## 8. Next physical qualification increments

1. Define the actual cooling-loop/control volume and meter every inlet, outlet, bypass, purge, cleaning stream and storage change.
2. Perform tracer-based residence-time and mixing-response experiments across load and pump regimes.
3. Obtain metrology-lab calibration certificates, traceability chains, common-reference/covariance details and independent meter verification.
4. Build an ISO 5667-10 sampling plan with representative sampling, preservation, lab accreditation/methods, blanks, duplicates, spikes and analyte-specific detection-limit strategy.
5. Collect pre-operation seasonal baseline at intake, outfall, near-field, far-field and independent reference/control stations under rights-reviewed hydrographic/ecological protocols.
6. Publish OGC SensorThings v2.0 payloads only after schema and server-metadata conformance tests.
7. Run a blind replay: operators provide raw measurements and metadata, a separate verifier produces the receipt, and an independent reviewer reproduces it from pinned bytes and tool versions.

**Claim ceiling:** This is a software and metrology design package backed by a synthetic fixture. It contains no field-measured flow, temperature, pollutant, ecological state, sensor calibration, lab result, permit limit, site approval or environmental clearance.
