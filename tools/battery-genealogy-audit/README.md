# battery-genealogy-audit

Dependency-free Rust 1.96 reference audit for material-lot → process-event → specimen → cell-build → test genealogy.

Checks globally unique node identities, exact parent/child lineage, process event chronology and acyclicity, cell assembly/formation linkage, electrode specimen identity, measured-field instrument/calibration references, test configuration matching, and raw-data SHA-256 syntax.

This is a read-only declared-record integrity checker. It does not prove that reported values are physically accurate, calibrations are authentic, a process is capable, a recipe is safe, or a battery meets performance targets.

Unit tests are authored but must be run by the exact-head workflow.