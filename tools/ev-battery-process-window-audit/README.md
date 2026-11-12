# ev-battery-process-window-audit

Dependency-free Rust 1.96 reference audit for battery process-window and cell-level conformance records.

Consumes exact recipe/cell configuration identities and declared process-parameter windows. It checks process measurements, quality metric units/boundaries/specification limits, canonical evidence IDs and evidence admission, then reports first-pass conformance separately by production lot and line and by evidence tier.

Out-of-window runs are not included in in-window first-pass-yield denominators. Incomplete and invalid inspections are not passes. Synthetic, simulation, vendor, literature, surrogate and non-admitted evidence cannot be displayed in the physical-evidence tier.

The reported rate is descriptive finite-sample accounting. It is not Cp/Cpk, a population-yield guarantee, an uncertainty interval, process-causality result, equipment controller, safety qualification or product release. Unit tests use explicitly synthetic values and IDs.