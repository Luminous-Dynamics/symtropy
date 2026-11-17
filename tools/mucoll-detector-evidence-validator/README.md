# MuonColliderSoft Detector Evidence Validator

Rust CLI/library for validating the integrity and provenance of MuonColliderSoft aggregate metric reports, metric fragments, RECO inputs, production records and benchmark producer sources.

## Scope

This first version verifies the existing v1 public sidecar/report contract without recalculating physics quantities:

- RECO input SHA-256, byte length and file identity;
- generation input SHA-256/byte length against the MuonColliderSoft `submission.json`;
- production record digest and consistency with the aggregate report;
- common event count and selected-event count semantics;
- each metric fragment digest, study identity, input identity, producer, configuration and metrics against the aggregate;
- producer source digests and aggregate script digest;
- stack version, benchmark/spack revisions, geometry, container image identity and GitHub run metadata;
- finite metric numbers while preserving `null` as unknown.

The result deliberately separates evidence integrity from physics comparison:

`evidence_package_status = valid` does **not** mean the detector physics passed. Since the public MuonColliderSoft baseline currently has no admitted pass/fail physics tolerances, this validator emits:

- `physics_reference_status = not_evaluated`
- `qualification_decision = abstain`

It does not infer reference tolerances, recalculate reconstruction metrics, run ROOT/EDM4hep, or execute the detector simulation stack.

## Run

```sh
cargo run --manifest-path tools/mucoll-detector-evidence-validator/Cargo.toml -- \
  --report metrics.json \
  --reco-input reco.edm4hep.root \
  --submission submission.json \
  --generation-input gen.edm4hep.root \
  --fragments-dir metrics-fragments \
  --producer-dir mucoll-benchmarks/analysis/python/edm4hep \
  --aggregator-source mucoll-benchmarks/analysis/benchmarks/aggregate_metrics.py \
  --sample-label pion
```

The CLI exits 0 when the evidence package is internally consistent, and nonzero when the evidence package is rejected. A zero exit is not a detector-physics pass.

## Tests

```sh
cargo test --manifest-path tools/mucoll-detector-evidence-validator/Cargo.toml
cargo fmt --manifest-path tools/mucoll-detector-evidence-validator/Cargo.toml -- --check
cargo clippy --manifest-path tools/mucoll-detector-evidence-validator/Cargo.toml --all-targets -- -D warnings
```

Tests use synthetic metric fragments and small byte fixtures. They do not need ROOT, Geant4, DD4hep, ACTS or the MuonColliderSoft Spack stack.
