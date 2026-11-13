# Membrane Observation Semantics Contract v0.1

**Tracking issue:** [#5039](https://github.com/Luminous-Dynamics/symtropy/issues/5039)
**Parent benchmark:** [#4977](https://github.com/Luminous-Dynamics/symtropy/issues/4977)
**Dataset crosswalk:** [#4996](https://github.com/Luminous-Dynamics/symtropy/issues/4996)

## Purpose

Standardize what an observation actually means before comparing or training across plants and studies. A 5-minute rolling average is not an instantaneous sample. A normalized permeability is not raw flux/TMP. A laboratory result is not a 5-minute sensor stream. The schema makes these distinctions explicit and machine-checkable.

## Observation evidence classes

1. RAW_INSTANT_MEASUREMENT: sensor reading at one measurement instant.
2. ROLLING_AGGREGATE: a mean/min/max/median or other aggregate over a defined time window.
3. LAB_SAMPLE: laboratory result attached to a physical sample collection time, stage and analyte.
4. DERIVED_METRIC: arithmetic/physics result with explicit formula/version and input observation lineage.
5. NORMALIZED_METRIC: value corrected to a declared reference condition with explicit normalization method/version and all correction inputs.

These classes are not interchangeable training targets. Preserve evidence class through ingestion, feature engineering, evaluation and reporting.

## Required metadata

- Process: UF/NF/RO/MBR, site, plant/train, stage and module identity where available.
- Variable: canonical identifier, value, unit and measurement location.
- Time: timestamp, timezone, time semantics, window start/end/duration and aggregation method.
- Source: sensor/instrument or laboratory identity, calibration record/status and uncertainty.
- Quality: VALID/MISSING/STALE/INVALID/SUSPECT/BELOW_LOD/BELOW_LOQ; missing is null, never zero.
- Derived: formula/version, complete input observation IDs, unit conversion and synchronization tolerance.
- Normalized: normalization method/version, reference state, correction inputs and reference units.
- Lab: sample ID, collection time, stage/location, analyte, method, qualifier, LOD/LOQ and QC status.

## Conditional invariants

- Instant readings have no aggregation window; timestamp semantics must be INSTANT.
- Rolling aggregates require a positive window, start/end timestamps and explicit aggregation method; the recorded timestamp has declared window semantics.
- Laboratory samples use SAMPLE_COLLECTION semantics and preserve the exact physical sample identity.
- Derived metrics require non-empty lineage and a versioned formula.
- Normalized metrics require a versioned reference state and explicit correction-input lineage. If temperature, osmotic pressure, recovery, flow or other required inputs are absent, the normalized result is not admissible.
- MISSING/STALE/INVALID values are null, not numeric zero. Censored results preserve BELOW_LOD/BELOW_LOQ rather than becoming zero.
- Unit strings are mandatory. Unit compatibility and conversion are separate, auditable transformations.

## Dataset admission use

Apply the schema to the acquired Bristol files after preserving the original archives. Do not infer missing columns from a paper description. In particular, the 34-month Bristol long-run release is described as 5-minute rolling averages; its metadata does not confirm TMP, temperature or calibration history. The 12-week trial is a distinct release with UF TMP and sparse laboratory samples.

## Metadata completeness scorecard

Report coverage separately for: variable/unit, timestamp/aggregation, process/location identity, sensor/calibration, quality/missingness, normalization reference inputs, derived lineage, lab sample identity and maintenance event linkage. Do not compress these into one score that hides a missing critical field.

## Nonclaims

This is a serialization/admission contract, not a metrology standard or proof that a sensor is accurate. Passing schema validation does not prove calibration, process correctness, water quality, membrane integrity, fouling mechanism or safe control.