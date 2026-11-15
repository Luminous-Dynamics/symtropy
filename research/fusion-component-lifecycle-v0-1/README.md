# Fusion Component Lifecycle Contract v0.1

**Status:** research seed; synthetic contract tests only  
**Parent:** [Symtropy #5208](https://github.com/Luminous-Dynamics/symtropy/issues/5208)  
**Branch:** research/fusion-component-lifecycle-v0-1

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
- Explicit gate states: Pass, Fail, Unknown.
- Validation of required identity fields, digest syntax, duplicate/dangling evidence IDs and gate evidence support.
- Fail-closed disposition:
  - any failed hard gate -> Blocked;
  - otherwise any unknown hard gate -> Indeterminate;
  - all gates pass -> EligibleForFurtherEngineeringReview.

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

## Frozen synthetic fixture corpus

File: research/fusion-component-lifecycle-v0-1/fixtures/lifecycle-cases-v1.tsv

The corpus contains 12 versioned cases: six truth-table cases covering all-pass, unknown, failure precedence and all-unknown behavior; and six adversarial cases covering unsupported PASS, missing reason, dangling evidence, cross-gate scope mismatch, duplicate evidence identity and malformed digest. All records are synthetic; none describes a real material, manufacturing process, component, reactor or observation.

Two separate checks consume the frozen corpus:

- An integration test constructs the declared candidate packages and checks the actual assess implementation against each expected disposition/rejection.
- A standalone binary recomputes the gate-state truth table without calling assess, and checks corpus shape, unique case IDs, allowed adversarial mutations and expected labels. It is an independent oracle for the disposition truth table, not an independent physics or manufacturing validator.

Run:

```bash
cargo test --manifest-path research/fusion-component-lifecycle-v0-1/Cargo.toml
cargo run --manifest-path research/fusion-component-lifecycle-v0-1/Cargo.toml --bin verify_fixture_corpus
cargo clippy --manifest-path research/fusion-component-lifecycle-v0-1/Cargo.toml --all-targets -- -D warnings
```

No test/build/Clippy pass is claimed until those commands execute against the exact proposed head.

## Benchmark interpretation and claim ceiling

The fixtures exercise software contract behavior only. A positive fixture establishes only that a candidate package is internally complete under the declared synthetic assumptions. It does not validate those assumptions, calculate manufacturability, or authorize physical manufacture.

The contract's strongest result remains EligibleForFurtherEngineeringReview, with claim ceiling ContractAndSyntheticFixtureOnly. It cannot issue manufacturing release, safety approval, physical qualification, reactor validation, commissioning or operating authority.

The next increment should connect read-only references to canonical Symthaea MFG-PROC/MAT-UQ records and Symtropy component/metrology receipts without duplicating their schemas. Geometry, tolerance, reachability and physical-property claims still require their own qualified solvers and independent evidence.
