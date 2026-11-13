# Artwork Conservation Reference Model
**Status:** research/design specification v0.1  
**System owner:** Symtropy  
**Integration:** consumes artwork identity and evidence from Mycelix; consumes geometry/scene frames from Sol-Atlas  
**Claim ceiling:** decision support and measured exposure history first; no unvalidated deterioration or authenticity claims

## 1. Objective

Create a deterministic, evidence-carrying reference model for the environmental and handling conditions affecting physical artworks. The first release should answer narrow questions such as:
- What environmental exposure was actually recorded?
- Which conservation thresholds supplied by an institution or conservator were exceeded, for how long, and with what data gaps?
- What handling or transport loads were measured or simulated under declared assumptions?
- Which condition changes merit expert inspection?

Do not begin by claiming to predict an artwork's remaining lifetime. Material-specific deterioration is chemically, optically, structurally, and contextually complex. A generic physics engine cannot supply missing material parameters or replace conservation judgement.

## 2. Paradigm shift: from static condition report to auditable lifecycle state

A conventional condition report is a dated observation. The Symtropy model should connect successive observations to:
- environmental sensor traces and calibration records;
- transport and installation events;
- physical geometry and object orientation;
- material/technique descriptions with explicit source and confidence;
- conservation interventions;
- observed changes and expert assessments;
- model versions, parameters, uncertainty, and limitations.

Every simulated result must link to its inputs and assumptions. A simulation must never be presented as a physical observation.

## 3. Staged model

### Stage A — exposure ledger (first implementable slice)

Ingest timestamped, unit-bearing observations:
- temperature;
- relative humidity;
- illuminance and UV where available;
- vibration/shock/tilt during transport where instrumented;
- sensor identity, calibration, placement, missingness, and time synchronization.

Compute transparent summaries: duration in declared bands, cumulative light exposure where measurement units support it, rate/range of change, excursions, missing-data windows, and trace quality. Thresholds must be institution- or conservator-supplied, versioned, and scoped to a particular collection/object/material. Do not ship invented universal thresholds.

### Stage B — risk triage with uncertainty

Compare observations against the declared policy and emit explainable findings:
- threshold exceeded / not exceeded / indeterminate;
- time exposed and affected interval;
- source sensor and calibration status;
- data quality and completeness;
- uncertainty bounds;
- recommended human review category.

The model may rank items for inspection. It must not diagnose damage or automatically prescribe restoration.

### Stage C — material-specific response models

Only after a conservator and materials specialist select a well-defined material/condition class:
- state the deterioration mechanism and governing assumptions;
- obtain traceable parameters from peer-reviewed sources or controlled measurement;
- fit on calibration data and freeze parameter manifests;
- reserve independent objects/sites/time periods for validation;
- model competing mechanisms and interactions where justified;
- report prediction intervals and out-of-domain abstention.

### Stage D — transport and installation mechanics

Use Sol-Atlas geometry and a declared coordinate frame. Symtropy can model a bounded mechanical scenario (orientation, support contacts, acceleration/shock envelope, load paths) where constitutive properties and boundary conditions are known. Unknown material properties or support conditions must block numerical confidence claims.

## 4. Data and event contract

Each input/output event should carry:
- artwork ID and passport version;
- event ID, event type, UTC time interval, and source clock quality;
- sensor/device ID, firmware, calibration record, placement, units, and digest;
- original observation digest and transformation digest;
- coordinate frame and transform lineage where spatial;
- model, parameter, and policy digests;
- missing data, imputation policy (default: none for evidentiary metrics), and uncertainty;
- result classification and explicit human-review requirement.

Never overwrite the original observation when cleaning, resampling, or converting units. Preserve raw and derived records with lineage.

## 5. Baselines and benchmark programme

### Determinism and integrity
- Exact replay of a frozen trace and model build yields identical discrete decisions and digest-bound outputs.
- Unit conversions and timestamp transformations are tested with positive and negative fixtures.
- Missing, stale, uncalibrated, out-of-range, or contradictory sensor values produce an explicit indeterminate state.
- No imputation may be silently counted as a measured observation.

### Exposure calculations
Compare the implementation with an independently written reference calculation for cumulative duration and dose under synthetic piecewise-constant and irregularly sampled traces. Report numerical error, boundary behavior, and sensitivity to time resolution.

### Risk triage
Build a conservator-adjudicated set of historical monitoring traces and condition observations. Use object-level and institution-level holdouts. Report precision/recall for actionable alerts, false alerts per object-month, detection delay, calibration, and abstention coverage. Compare against:
1. current institution rules;
2. simple threshold/exceedance summaries;
3. a calibrated statistical baseline.

Do not use a random split of adjacent sensor samples as train/test.

### Material-specific prediction
Only after a sufficiently sized, rights-cleared, expert-labelled dataset exists, report:
- interval coverage and interval width;
- MAE/RMSE only where the target supports them;
- calibration by material, mechanism, and environment;
- transfer to held-out objects and sites;
- worst-group error and out-of-domain abstention;
- improvement over conservative simple baselines.

### Mechanical scenarios
Where physical measurements exist, compare predicted load/deflection or contact response with instrumented tests and independent finite-element or analytical baselines. Report parameter uncertainty and sensitivity. Simulation-only fixtures do not establish real transport safety.

## 6. Integration boundaries

- **Mycelix:** signed identity, custody, event and rights assertions; it does not certify material condition.
- **Sol-Atlas:** geometry, scene, coordinate frame, capture lineage and spatial uncertainty; it does not infer legal ownership.
- **Symtropy:** deterministic physical/exposure models and replay receipts; it does not create an artwork's identity or override a conservator.
- **Symthaea:** explain evidence, retrieve relevant knowledge, and help formulate hypotheses; it does not autonomously diagnose, authenticate, or authorize interventions.

## 7. Initial acceptance criteria

- A deterministic exposure-ledger slice handles temperature/RH/light traces with raw-to-derived lineage.
- All units, time bases, sensor calibrations, gaps, and parameter/policy digests are explicit.
- Reference calculations cover regular, irregular, missing, stale, and contradictory traces.
- No universal material threshold is hard-coded without an authoritative, scoped source.
- Outputs distinguish observation, calculation, simulation, expert opinion, and recommendation.
- At least one independent conservator reviews the schema and report semantics before a pilot.

## 8. What this work must not claim

It is not an authenticity detector, a universal conservation oracle, a certified insurance valuation, or a substitute for professional examination. A passing deterministic replay test proves reproducibility of the software calculation only, not that the model describes the material world accurately.
