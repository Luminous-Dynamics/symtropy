# Fusion Component Lifecycle Contract v0.1

**Status:** research seed; synthetic contract tests only  
**Parent:** [Symtropy #5208](https://github.com/Luminous-Dynamics/symtropy/issues/5208)  
**Branch:** `research/fusion-component-lifecycle-v0-1`

## Purpose

This isolated, dependency-free Rust crate tests a narrow handoff contract for reactor-component candidate evidence:

```text
design identity
  + material record identity
  + process-plan identity
  + manufacturing / tolerance / inspection / assembly / replacement gates
  -> bounded engineering-review disposition
```

It is intended to connect Symthaea's candidate/material/process references to Symtropy's deterministic engineering-replay and lifecycle evidence.

## What is implemented

- Stable candidate identity and geometry digest fields.
- Evidence references with subject scope, revision, SHA-256 digest and evidence class.
- Explicit gate states: `Pass`, `Fail`, `Unknown`.
- Validation of required identity fields, digest syntax, duplicate/dangling evidence IDs and gate evidence support.
- Fail-closed disposition:
  - any failed hard gate -> `Blocked`;
  - otherwise any unknown hard gate -> `Indeterminate`;
  - all gates pass -> `EligibleForFurtherEngineeringReview`.

The strongest possible result is deliberately **not** a manufacturing release, safety case, physical qualification, reactor qualification or operating authorization.

## Current hard gates

1. Material applicability under the declared environment / condition profile.
2. Process capability.
3. Tolerance-stack assessment.
4. Inspection access.
5. Assembly access.
6. Replacement access.

The crate does not calculate these physical/engineering properties. It validates only the evidence contract and synthetic gate logic supplied by a caller.

## What is not implemented

- CAD parsing, geometric interference or tolerance-stack computation.
- Material-property, joining, structural, thermal-hydraulic, neutronics or plasma solvers.
- Robotics reachability, remote-handling simulation or radiation-environment modelling.
- Manufacturing process execution, machine programs, inspection execution or physical acceptance.
- Net-energy, safety, licensing, production-readiness or component-life claims.
- Serialization or network APIs.

These are explicit non-goals for this first contract seed. Specialist owners and solvers must provide bounded, qualified evidence through versioned adapters before those claims can be considered.

## Verification

Run the isolated crate:

```bash
cargo test --manifest-path research/fusion-component-lifecycle-v0-1/Cargo.toml
cargo clippy --manifest-path research/fusion-component-lifecycle-v0-1/Cargo.toml --all-targets -- -D warnings
```

No test/build/Clippy pass is claimed until those commands execute against the exact proposed head.

## Benchmark interpretation

The unit tests are synthetic contract fixtures, not fusion-component test data. A positive fixture establishes only that the candidate package is internally complete under the declared synthetic assumptions. It does not validate the assumptions or authorize physical manufacture.

The next increment should add machine-readable frozen fixtures and an independent evaluator, then connect canonical Symthaea MFG-PROC/MAT-UQ references and Symtropy component/metrology receipts without duplicating their schemas.
