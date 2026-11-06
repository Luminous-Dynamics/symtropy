# Symtropy Cutter Health Bench

Standalone Rust semantic validator for cutter-health evaluation manifests.

## Scope

The crate validates:
- dataset and model identity, immutable digests and declared measurement references;
- independent inspection truth and calibration/uncertainty provenance;
- train, calibration and test sample-count accounting;
- disjoint test groups for declared parent-block, specimen, site, rig, cutter-geometry, serial and time holdouts;
- external-site and prospective-field split requirements;
- model-to-dataset/split binding;
- test-only metric partitions and task-specific metric semantics;
- independent inspection truth for wear, anomaly and RUL labels;
- separate calibration data for RUL intervals;
- event-count, coverage and finite/non-negative metric consistency.

The report describes manifest conformance and evaluation scope. It does not rank models or recompute metrics without predictions and raw labels.

## Run

    cargo test --manifest-path crates/domains/symtropy-cutter-health-bench/Cargo.toml

## Disposition

- DiagnosticOnly: random-row partition; no transfer claim.
- GroupedHoldout: valid grouped split manifest, without an external-site claim.
- ExternalSite: held-out site appears only in the test groups for declared dimensions.
- ProspectiveField: held-out site and future time window are declared.

These dispositions describe the test design, not the predictor's scientific validity or physical qualification.

The bundled fixture is fabricated conformance data. No publication dataset or physical cutter data is included.
