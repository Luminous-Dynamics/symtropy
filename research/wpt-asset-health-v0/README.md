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
- An August 2026 experiment compared PG 64–22 and PG 76–22 asphalt under a 10 °C controlled thermal-surrogate rise and MMLS3 loading. It reports 88% versus 50% mean tensile-strain amplification, with distributed fibre-optic sensing; the heating pads were passive thermal surrogates rather than energized WPT coils. [DOI 10.1016/j.applthermaleng.2026.132099](https://doi.org/10.1016/j.applthermaleng.2026.132099).
- A 1 November 2026 sensor study describes distributed fibre-optic measurement of strain, temperature and curvature in asphalt, with decoupling and rut-profile reconstruction. It is a sensing-method reference, not a WPT-roadway field validation. [DOI 10.1016/j.sna.2026.118304](https://doi.org/10.1016/j.sna.2026.118304).
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

The machine-readable contract in `asset-observation.schema.json` is illustrated by `asset-observation.synthetic-example.json`, which is explicitly marked SYNTHETIC_KNOWN_TRUTH and SYNTHETIC_ONLY; none of its numeric values are measurements. The revised schema preserves binder/mixture identity, wheel-wander offset and spatial strain, temperature and curvature observations, rather than collapsing a localized hot zone into one segment mean. The contract:
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

## Extended v1 measurement contract

The v1 record and evaluator preserve binder/mixture identity, wheel-wander offset, and spatially registered strain, temperature, curvature and heat-flux observations. Every spatial observation binds to the asset coordinate frame, sensor/method identity, location and depth.

Two recent studies motivate the extension:
- An August 2026 experiment compared PG 64–22 and PG 76–22 asphalt under up to a 10 °C controlled thermal-surrogate rise and MMLS3 loading. It reports 88% versus 50% mean tensile-strain amplification and about 34% lower tensile strain in the polymer-modified mixture. Its embedded heating pads were electrically passive thermal surrogates, not energized WPT transmitters. [DOI 10.1016/j.applthermaleng.2026.132099](https://doi.org/10.1016/j.applthermaleng.2026.132099).
- A 1 November 2026 asphalt-sensor study describes distributed measurement of strain, temperature and curvature with decoupling and 2D rut-profile reconstruction. It is a sensing-method reference, not a long-duration WPT roadway field validation. [DOI 10.1016/j.sna.2026.118304](https://doi.org/10.1016/j.sna.2026.118304).

### Thermal-surrogate equivalence is a separate qualification

A passive heating pad and an energized WPT transmitter may produce the same maximum surface temperature while delivering different spatial heat flux, depth gradients, internal material heating, duty cycles and cooldown histories. A peak-temperature match alone is insufficient to transfer a thermo-mechanical result from surrogate heating to an active WPT system.

Before claiming equivalence for a declared pavement response, compare spatial heat-flux distribution, multi-depth temperature histories, heating rate, dwell, duty cycle, cooldown, thermal-cycle count, ambient background and measurement uncertainty. Equivalence must be limited to a named response quantity and material regime. Otherwise retain the result as a thermal-surrogate experiment, not active-WPT validation.

The expanded schema requires spatial strain, curvature, temperature and heat-flux profiles to use one declared asset coordinate frame. Its synthetic example is explicitly tagged SYNTHETIC_ONLY.
