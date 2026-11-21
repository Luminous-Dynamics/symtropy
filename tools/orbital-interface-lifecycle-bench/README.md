# Orbital Interface Lifecycle Qualification Benchmark v0.1

**Status:** initial synthetic reference implementation; hosted Rust verification pending.  
**Owner:** [ORBITAL-INTERFACE-LIFECYCLE-001 #6484](https://github.com/Luminous-Dynamics/symtropy/issues/6484).  
**Composes:** [#6336 nominal interface conformance](https://github.com/Luminous-Dynamics/symtropy/pull/6336), [#6372 robust compatibility](https://github.com/Luminous-Dynamics/symtropy/pull/6372), [#6379 service reconfiguration](https://github.com/Luminous-Dynamics/symtropy/pull/6379), [#6384 receipt admission](https://github.com/Luminous-Dynamics/symtropy/issues/6384), and [#6256 ground test cell](https://github.com/Luminous-Dynamics/symtropy/issues/6256).  
**Claim ceiling:** synthetic lifecycle-state semantics only.

## Purpose

A compatibility or qualification result at commissioning is not a lifetime guarantee. This benchmark evaluates whether the same exact provider/client pair remains qualified as service cycles, inspections, wear observations, maintenance, repairs, configuration changes and evidence validity evolve.

The v0.1 evaluator is a deterministic state-contract reference. It does not estimate a fatigue curve, seal lifetime, probability of failure, remaining useful life or orbital reliability.

## Decision model

The evaluator binds qualification to:
- exact provider/client pair;
- exact provider configuration and procedure revision;
- service kind (mechanical capture or prepared storable-fluid interface);
- current cycle count and qualified cycle envelope;
- inspection age and inspection interval;
- qualification validity interval;
- calibration, inspection coverage, wear, environment and uncertainty evidence;
- reconciled lifecycle lineage and exact raw-artifact admission;
- unresolved service-specific defects, maintenance and revocation state.

Disposition is one of:
- `qualified_within_declared_envelope`
- `restricted`
- `requalification_required`
- `withdrawn`
- `rejected`
- `indeterminate`

A critical defect or revoked qualification withdraws the relevant service qualification. Changed configuration, expired qualification, exceeded cycle envelope, expired inspection or post-qualification maintenance requires requalification. Missing, invalid, contradictory or unadmitted evidence is indeterminate. Mechanical and fluid service qualifications are distinct.

## Frozen corpus

The v0.1 corpus contains 42 synthetic known-answer cases. It covers valid service histories, cycle/inspection/qualification expiry, wear limits, measurement uncertainty, calibration, environment, maintenance, repairs, pair/configuration/procedure changes, service-kind separation, missing raw evidence, duplicate/conflicting event lineage, counter resets, revocation and critical-negative precedence.

The integration test contains a separately written reference decision function. It does not call the production evaluator's decision function.

## Run

```bash
cargo fmt --manifest-path tools/orbital-interface-lifecycle-bench/Cargo.toml -- --check
cargo test --manifest-path tools/orbital-interface-lifecycle-bench/Cargo.toml --all-targets
cargo clippy --manifest-path tools/orbital-interface-lifecycle-bench/Cargo.toml --all-targets -- -D warnings
cargo run --manifest-path tools/orbital-interface-lifecycle-bench/Cargo.toml -- \
  tools/orbital-interface-lifecycle-bench/fixtures/v0.1/cases.json \
  --source-commit <40-hex-commit>
```

The CLI hashes the exact fixture bytes it reads and emits a deterministic JSON receipt. A SHA-256 digest binds bytes; it does not authenticate the producer, instrument, calibration certificate or physical truth. The receipt always sets `authority_granted=false`.

## Baselines and metrics

The research issue defines matched comparisons against:
1. static qualification that remains valid indefinitely after an initial pass;
2. cycle-count-only thresholds;
3. latest-inspection-wins;
4. lifecycle-aware, evidence-bounded state qualification.

Do not report a synthetic score as an empirical reliability rate. The intended physical metrics include cycles to first restriction, inspection coverage, uncertainty-to-margin, requalification interval/turnaround, time spent restricted, and accepted service per qualified interface-cycle. Report all rejected, withdrawn, requalification-required and indeterminate outcomes separately.

## Physical correlation

The physical path belongs to #6256 and its staged uncrewed ground test cell. This benchmark does not prescribe universal cycle counts or loads. The responsible hardware engineer must set the test envelope from the exact article, design life, applicable standards, facility, hazard analysis and intended mission profile.

NASA NESC RP-06-108 discusses life-cycle mechanism testing, including a 2× expected-cycle reference in its cited context. That is not a universal test prescription.

The initial physical programme remains guarded, uncrewed and limited to reviewed benign-fluid coupons. No hydrazine, hypergolics, cryogenics, high-pressure propellant or live spacecraft hardware is in v0.1.

## Standards context

- [NASA NESC RP-06-108](https://ntrs.nasa.gov/api/citations/20080019636/downloads/20080019636.pdf): spacecraft DDT&E and mechanism life-cycle testing guidance.
- [ANSI/AIAA S-157-2025](https://webstore.ansi.org/standards/aiaa/ansiaiaa1572025): prepared in-space storable-fluid transfer design, verification and operations.
- [ISO 24330:2022](https://www.iso.org/standard/78463.html): programmatic RPO/OOS principles and practices.
- [NASA 2026 ISAM State of Play](https://www.nasa.gov/isam/): capability/facility/developer inventory, not a lifecycle qualification leaderboard.
- [On-orbit servicing as a future accelerator for small satellites (2025)](https://www.nature.com/articles/s44453-025-00024-7): discusses interface standardisation, inspection, repeatable qualification and serviceability.

## Validation state

The repository-authoring runtime used for this branch has no Cargo, rustc or rustfmt executable. No local Rust formatting, compilation, test or Clippy pass is claimed. The focused exact-head workflow is the required next verification; queued or skipped status is not a pass.

No physical coupon, facility, procurement, calibration or environmental test is claimed.
