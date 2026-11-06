# Symtropy Conveyance State Bench

Standalone, read-only benchmark-manifest validator for event-level pneumatic/vacuum conveying state estimation.

Implements the evaluation contract in [issue #3877](https://github.com/Luminous-Dynamics/symtropy/issues/3877). This crate admits event labels and leakage-resistant splits; it does not train or score a blockage detector, estimate physical capacity, issue equipment settings, or certify a transport line.

## Run

    cargo test --manifest-path crates/domains/symtropy-conveyance-state-bench/Cargo.toml

## v0.1 contract

- Exact site, campaign, installed path, machine and material group identities.
- Train, calibration and untouched test partitions with group-disjoint membership.
- Declared holdout dimensions (site, campaign, installed path, machine configuration, material cohort, temporal block, parent rock block) must not overlap across partitions.
- Synchronized observation windows; overlaps within a group are rejected to prevent near-duplicate window leakage.
- A complete channel registry with instrument, calibration, location, units, sample rate and source references.
- Every window explicitly records every registered channel as OBSERVED, DEGRADED or MISSING. Missing data is not zero-filled.
- Channel observations bind to the window's clock-synchronization reference and carry time uncertainty.
- Event labels require independent physical or sensor-diagnostic evidence, source provenance and an adjudicator reference.
- The held-out test partition must contain at least one critical seeded restriction/blockage event to support the critical-event recall challenge.
- Sensor faults remain distinct from physical conveying faults; multi-label events are permitted.
- Startup/shutdown transients and unknown/out-of-domain states are explicit.
- Critical-event deadlines and event/clear time boundaries are checked.

## State taxonomy

NOMINAL_STABLE, STARTUP_SHUTDOWN_TRANSIENT, HIGH_SOLIDS_LOADING, STRATIFIED_FLOW, DUNE_DEPOSITION, PARTIAL_RESTRICTION, DEVELOPING_BLOCKAGE, COMPLETE_BLOCKAGE, WATER_ACCUMULATION, FILTER_CYCLONE_LOADING, AIR_LEAK_DISCONNECTION, BLOWER_VFD_FAULT, PRESSURE_SENSOR_FAULT, AIRFLOW_SENSOR_FAULT, UNKNOWN_OUT_OF_DOMAIN.

## Non-claims

Event labels are admitted only with declared independent evidence. Operator-only reports, model predictions and simulations cannot establish physical event truth. The fixture is synthetic conformance data. This manifest does not validate a classifier or prove field transfer; a later frozen prediction artifact and independently labelled physical corpus are required.
