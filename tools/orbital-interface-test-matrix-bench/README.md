# Orbital Interface Qualification Test-Matrix Benchmark v0.1

**Status:** synthetic reference implementation; exact-head validation pending.  
**Owner:** [ORBITAL-QUALIFICATION-TEST-MATRIX-001 #6540](https://github.com/Luminous-Dynamics/symtropy/issues/6540).  
**Physical test-campaign owner:** [#6256](https://github.com/Luminous-Dynamics/symtropy/issues/6256).  
**Claim ceiling:** synthetic test-matrix coverage semantics only.

## Why this exists

A high number of nominal mating cycles does not demonstrate that a campaign exercised required low/high energy, environmental extremes, lifecycle stages or critical interactions. This benchmark audits whether pre-registered obligations have valid test evidence.

NASA-STD-5017B is an active mechanism design/development/qualification standard. Its qualification guidance includes the service-life operating cycle/environment envelope, nominal/low/high-energy function testing and attention to interactions between relevant extremes. It leaves the actual test sequence to project-specific engineering judgment. This crate does not replace NASA or project tailoring, and does not prescribe numerical levels or a universal DOE.

References:
- NASA-STD-5017B: https://standards.nasa.gov/standard/NASA/NASA-STD-5017
- NASA-approved public PDF: https://standards.nasa.gov/sites/default/files/standards/NASA/B//2022-12-06-NASA-STD-5017B-Approved.pdf

## Evaluator semantics

An obligation binds its exact configuration, procedure, service kind, lifecycle phase, factor-level conditions and required evidence key. A completed run can cover an obligation only when those fields match and the measurement evidence declares valid calibration, instrument range, uncertainty, clock alignment, raw-artifact digest shape and an independent-run group.

- A **completed failed** test covers the tested condition but remains a visible failure; coverage is not a pass.
- Planned, aborted, invalid or evidence-incomplete runs do not cover an obligation.
- Two samples from the same independence group do not count as independent replicates.
- Conflicting records under one run ID are Disputed; byte-identical repeated records are DuplicateNoOp.
- Configuration, procedure, function, lifecycle phase and evidence key remain distinct.
- A campaign that exceeds its cost budget can still have complete coverage; cost and adequacy are reported separately.
- Informational obligations do not mask missing mandatory or critical obligations.
- A pre-registration reference must be present before execution; the v0.1 fixture only checks that the reference is supplied and does not authenticate its issuer or prove the record existed before testing. Post-hoc obligation changes are Rejected. A NotApplicable obligation needs both a reason and a review-evidence reference; the reference is not cryptographically or institutionally verified by this crate.

The fixture's pre-registration and NotApplicable review references are declared synthetic fields; this crate checks presence, not authority. The raw-artifact digest is also a declared synthetic field whose format is checked. This crate does not verify original artifact bytes, producer signatures, sensor calibration or physical truth; those remain with #6515, Mycelix evidence/attestation and metrology owners.

## Shortcuts measured on the same obligations

- **Cycle-count baseline:** number of tests labelled Passed reaches a threshold. It can pass even when no low/high-energy test exists.
- **OFAT baseline:** each factor's required level appears somewhere in a valid run. It does not require the factors to occur together.
- **Pairwise baseline:** every pair of required factor levels appears somewhere. It can miss an important three-way interaction.
- **Strict coverage:** all conditions for an obligation occur together in the same eligible run(s), with enough distinct independence groups.

The metrics report false OFAT and pairwise coverage counts when those shortcuts claim a required obligation is covered but the strict evaluator does not.

## Frozen corpus

The v0.1 corpus contains 35 synthetic campaigns. It covers missing low/high energy, OFAT/pairwise interaction traps, non-frozen criteria, unsupported NotApplicable, planned/aborted/failed runs, duplicate/conflicting run IDs, stale configurations/procedures, calibration/range/uncertainty/clock invalidity, post-environment and post-maintenance checks, service/function separation, independent-run grouping, budget overruns and permutation positive controls.

Run locally:

```bash
cargo fmt --manifest-path tools/orbital-interface-test-matrix-bench/Cargo.toml -- --check
cargo test --manifest-path tools/orbital-interface-test-matrix-bench/Cargo.toml --all-targets
cargo clippy --manifest-path tools/orbital-interface-test-matrix-bench/Cargo.toml --all-targets -- -D warnings
cargo run --manifest-path tools/orbital-interface-test-matrix-bench/Cargo.toml --bin orbital-interface-test-matrix -- \
  tools/orbital-interface-test-matrix-bench/fixtures/v0.1/campaigns.json \
  --source-commit <40-lowercase-hex-commit>
node tools/orbital-interface-test-matrix-bench/tests/reference_oracle.mjs \
  tools/orbital-interface-test-matrix-bench/fixtures/v0.1/campaigns.json
```

## Boundaries

- This is a read-only coverage auditor; it does not choose physical limits or approve a test plan.
- It does not infer statistical confidence, failure probability, life distributions or reliability.
- It does not infer pairwise coverage for higher-order critical interactions.
- It does not authenticate evidence producers or verify original measurement bytes.
- It does not grant actuation, service, docking or mission authority.
- A campaign with complete synthetic coverage is not thereby physically qualified, safe or flight-ready.

Composes with #6256/#6303, #6484/#6485 and #6515/#6516; consumes their evidence/applicability boundaries without replacing their owners.