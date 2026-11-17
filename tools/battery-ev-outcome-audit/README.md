# ev-battery-outcome-audit

Dependency-free Rust 1.96 consumer-side integrity audit for EV battery cell, pack and mission references.

Consumes opaque versioned references to canonical Symthaea MAT-013 sample lineage, MAT-PSPP process/structure/property projections, MAT-INTERFACE evidence and MAT-UQ profiles. It then checks cell test configuration, pack membership and mission-outcome reference closure.

It deliberately does not recreate generic sample/process lineage, materials ontology, interface ontology, uncertainty/calibration, physical models or test-metrology authority.

The audit is read-only. It validates declarations and cross-reference consistency, not source authenticity, physical accuracy, safety or battery performance. Unit tests are authored and require exact-head CI execution.