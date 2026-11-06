# WPT embedded asset health benchmark v0

**Status:** research specification, closed schema and synthetic challenge manifest  
**Parent:** [WPT-ASSET-HEALTH-001 #3726](https://github.com/Luminous-Dynamics/symtropy/issues/3726)  
**Related:** #3452 roadway co-design; #3641 lifecycle economics; #3477 metrology; #3560 adaptive matrix atlas; #3692 cluster-aware calibration.

## Research question

Can the condition of an embedded wireless-power asset be inferred and forecast from synchronized structural, thermal, environmental and electromagnetic evidence—and can its remaining service life be estimated without confusing repeated readings, simulated values or right-censored assets for independent failures?

The missing bridge is:

`traffic / environment → pavement and embedment damage → geometry / material drift → L, M, k, Q, ESR and resonance drift → qualified power envelope → useful route energy and maintenance decision`

## Why this matters now

The literature is moving from coil prototypes toward embedded infrastructure:

- Moncada et al. report a **75,000 traffic-pass** full-scale accelerated pavement test of a flexible pavement with an embedded DWPT unit. Their abstract reports progressive viscoelastic strain accumulation and greater surface-layer degradation around the unit, alongside no visible surface distress under that test. [Paper, DOI 10.1080/10298436.2025.2604702](https://doi.org/10.1080/10298436.2025.2604702).
- The 2026 electrified-road review covers 139 papers and identifies coupled thermal, structural and long-term durability challenges, while noting that full-scale field evidence remains limited. [Review, DOI 10.1080/14680629.2026.2701268](https://doi.org/10.1080/14680629.2026.2701268).
- Pavement thermal-material characterization combines laboratory specimen testing and FEM to study embedded WPT heat and asphalt temperature profiles. [DOI 10.1177/03611981251372472](https://doi.org/10.1177/03611981251372472).
- A 2026 rigid-pavement study reports a field-calibrated 3D FE model checked against slab strain and FWD observations; it remains a simulation-based assessment. [DOI 10.1016/j.trgeo.2026.102045](https://doi.org/10.1016/j.trgeo.2026.102045).
- INDOT's pilot explicitly investigates traffic loading, freeze–thaw, snowplowing, resurfacing and maintainability: [official project page](https://www.in.gov/indot/emerging-mobility/dynamic-wireless-power-transfer/).
- Honda announced a **planned** million-wheel-load evaluation using 49 kN per wheel from Japanese FY2027 onward, with later high-power testing up to 150 kW. These are announced plans, not completed tests: [6 October 2026 announcement](https://global.honda/en/topics/2026/c_2026-10-05beng.html).
- Japan's public-road experiment reports approximately 10 kW, 63.3% of drive energy replenished during the reported drive and 18 months of stable buried-transmitter parameters without false detection/transmission. This is an operational endurance reference, not a full pavement fatigue or RUL dataset. [DOI 10.1541/ieejias.146.194](https://doi.org/10.1541/ieejias.146.194).

These are different systems and measurement boundaries. No cross-study leaderboard or lifetime extrapolation is justified from the abstracts alone.

## Paradigm shift: a coupled physical asset, not a road plus a coil

A pavement-embedded WPT segment is one composite asset. Maintain separate, typed state variables for:

1. Structural condition: strain, deflection, rutting, cracking, settlement, interface bond, pad translation and tilt.
2. Thermal/environmental history: coil/pavement temperature, thermal gradients and dose, duty cycle, ambient conditions, moisture, freeze–thaw and chloride exposure.
3. Electromagnetic/electrical condition: L, M, k, Q, ESR, resonant frequency, insulation resistance and leakage-field map.
4. Service condition: qualified operating envelope, boundary-specific energy, useful Wh/km and availability.
5. Provenance: asset lineage, material lot, installation/hardware/controller revision, instrument, calibration and raw evidence digest.

A latent damage estimate must never overwrite a measured observation. A single opaque health score is not an acceptable substitute for the typed condition vector.

## Causal hypotheses to validate

The benchmark is designed to test—not presume—these connections:

- settlement / differential movement → coil position or tilt → coupling and alignment envelope;
- traffic fatigue / cracking / interface debonding → pad motion and changed thermal resistance;
- thermal cycling → material ageing and interface stress → thermal/mechanical drift;
- water ingress / corrosion → resistance and insulation change;
- ferrite / shield damage → Q, loss, field distribution and heat change.

Each relationship must be labelled as literature-supported, hypothesized, simulated, synthetic known-truth, laboratory-measured or field-measured. Correlation alone does not establish causation.

## Benchmark controls and evidence rules

Use matched conventional-pavement, embedded-unpowered and embedded-energized arms where feasible. Preserve pavement type, material lot, installation and hardware identity.

The machine-readable contract in `asset-observation.schema.json` is illustrated by `asset-observation.synthetic-example.json`, which is explicitly marked SYNTHETIC_KNOWN_TRUTH and SYNTHETIC_ONLY; none of its numeric values are measurements. The contract:
- uses explicit units and typed measurement fields;
- permits `null` for unmeasured values rather than silent imputation;
- binds observations to asset/segment/time and measurement-plane provenance;
- records uncertainty and calibration receipts;
- distinguishes measured, published-measured, simulated, synthetic and inferred evidence.

The scenario manifest includes synthetic known-truth progressions and negative controls for sensor drift, missing calibration, asset leakage across splits, censoring and missing raw evidence. The deterministic equations and hand-recomputable test points are frozen separately in `synthetic-reference-oracle.md`. These are evaluator controls, not material or hardware constitutive laws. The repository contains no invented WPT measurement values.

## Statistical discipline

Split by physical asset, segment, material lot, installation, route and environmental campaign. Never split adjacent time rows randomly across train/calibration/test. One thousand readings from one segment are not one thousand independent lifetime trials.

Right-censored assets are not failures. Repeated measurements are clustered. Report condition and RUL interval coverage, width and dependence-aware confidence bounds, reusing the methodological direction of #3692.

## Proposed targets

These are programme targets, not standards or achieved results:

- 100% traceability for every value used in a derived condition or maintenance decision.
- Zero fabricated/imputed measurements.
- Zero false-healthy classifications in the frozen known-truth threshold corpus.
- Nominal 90% and 95% condition/prognostic interval coverage reported by independent asset groups, with initial 88–92% and 93–97% target bands only where sample size supports them.
- Structural and thermal error tolerances frozen from instrument repeatability and independent reference methods before candidate evaluation.
- Electromagnetic drift reported in physical units and compared to instrument uncertainty; no universal percentage threshold is assumed.
- Maintenance optimization must expose failure mode, service loss, downtime and lifecycle cost; safety/protection gates are not weighted objectives.

## Next implementation increments

1. Rust validation of observation, asset lineage, units, timestamps, provenance and null semantics.
2. Deterministic synthetic known-truth progression fixtures.
3. Split-manifest validator enforcing asset/material-lot isolation.
4. Independent evaluator for drift and prognostic scoring, including censoring.
5. Structural, thermal and electromagnetic model adapters with separate validity claims.
6. Coupled state transition that updates the electromagnetic atlas input only when supported by measured geometry/material state.
7. Integration into route energy and lifecycle maintenance accounting.

## Claim ceiling

This is a schema, benchmark and software research specification. It does not establish field lifetime, hardware safety, standards compliance, predictive maintenance readiness or SOTA superiority. No energized high-power build, public-road operation or unapproved accelerated pavement test is authorized.

## Time-to-event evidence is separate from condition observations

A condition observation is not a survival outcome. The new `asset-lifecycle-outcome.schema.json` defines a separate first-event survival dataset with explicit `TARGET_FAILURE`, `RIGHT_CENSORED` and `COMPETING_EVENT` outcomes. This prevents the end of a study from being converted into a failure and lets the evaluator enforce grouped split isolation across asset, segment, material lot, installation, route and environmental campaign.

The synthetic lifecycle fixture contains no field-lifetime observations. Its times and event labels are known-truth controls only. The Rust contract validator checks timestamp-derived durations, event/censor semantics, duplicate asset identity and cross-split leakage.


## Survival outcomes and leakage-resistant evaluation

The contract is now split into two related but distinct record types:

- `asset-observation.schema.json` records a condition measurement at an observation time.
- `asset-lifecycle-outcome.schema.json` records a time-to-event subject with a target failure, right-censoring or a competing event.

The synthetic example `asset-lifecycle-outcomes.synthetic-example.json` contains one case of each type. The validator verifies event/censor semantics, duration against UTC timestamps, one first-event record per asset, and prevents repeated asset, segment, material-lot, installation, route or environmental-campaign groups from crossing train/calibration/test splits. All lifecycle values are synthetic fixtures; there is no field RUL result.


Lifecycle outcome provenance is validated separately from condition-observation provenance. A MEASURED time-to-event outcome requires source-record identity and a raw artifact digest; a PUBLISHED_MEASURED outcome requires a source locator. This does not infer the event from incomplete evidence and does not convert censored observations into failures.

## Exact-head test and claim boundary

The Rust contract package is a parser/semantic validator for versioned condition observations and lifecycle outcome records. Its package gate is:

`cargo test --locked -p wpt-asset-health-contract`

The test suite is intended to prove record semantics (including target failure vs right-censoring vs competing events), UTC duration consistency, provenance requirements, identity uniqueness and split isolation. It does not estimate an RUL model or establish predictive accuracy. A synthetic lifecycle outcome is never a measured asset outcome; a successful validator run cannot create field evidence.

For a measured outcome, preserve the source-record identity and raw artifact digest alongside the time/event record. For a published measurement, preserve a locator to the underlying report. Missing provenance is a validation failure, not an invitation to impute event truth.

