# CFI-Space EVSI Observation Selection v0.1

Issue: [#7880 — EVSI-driven active observation selection](https://github.com/Luminous-Dynamics/symtropy/issues/7880)

## Purpose

This benchmark tests a different objective from maximum confidence or maximum entropy reduction:

**Choose the next observation that reduces expected mission decision loss the most, after accounting for sensing cost and admissibility.**

The model is a finite synthetic hypothesis set, a mission loss matrix, and a fixed candidate set of observations. Each observation proposal includes its conditional likelihood model, resource cost, latency budget, required capability and preconditions, dependency status, and explicit advisory-only/no-actuation flags.

## Exact integer EVSI

Priors and observation likelihoods use basis points (10,000 = 1.0). For hypothesis h, prior p_h, proposal a, outcome o, and mission loss L(h,d):

- prior best-decision numerator: min_d sum_h p_h * L(h,d)
- joint hypothesis/outcome weight: w(h,o) = p_h * P(o|h,a)
- posterior best-decision numerator for outcome o: min_d sum_h w(h,o) * L(h,d)
- gross EVSI numerator: prior numerator * 10,000 - sum of posterior best-decision numerators
- net EVSI numerator: gross EVSI numerator - cost units * 100,000,000

The common denominator is 100,000,000. The selector uses integer arithmetic for all decision-value calculations. Shannon information gain is computed separately as a baseline diagnostic; it never affects selection.

The selected proposal is the admissible action with the greatest strictly positive net EVSI. When all proposals are inadmissible or non-positive, the output has no recommendation.

## Synthetic known-answer

Hypotheses:
- healthy-lighting-a;
- healthy-lighting-b;
- common-cause-geometry-fault.

The two healthy subtypes have identical operational loss. The illumination-survey proposal distinguishes those healthy subtypes, yielding more Shannon information gain, but does not alter the best mission decision; its gross EVSI is zero. The independent-reference-check proposal has lower information gain but separates healthy evidence from a common-cause fault and reduces expected decision loss. The EVSI selector therefore selects the independent reference check while the entropy baseline selects the illumination survey.

This is the intended paradigm-shift test: **information gain is not decision value**.

## Admissibility

A proposal is rejected when:
- required sensing capability is unavailable;
- preconditions are not met;
- dependencies are unknown;
- latency exceeds its declared bound;
- the proposal is not advisory-only;
- any actuation authority is asserted.

The selector emits no hardware command. Its report asserts observer_only=true, hardware_action_issued=false and actuation_authority=false. Any future sensing request must pass through the existing, independently owned action/sensing pre-effect gate.

## Run

```sh
cargo fmt --manifest-path tools/cfi-space-evsi-bench/Cargo.toml -- --check
cargo test --manifest-path tools/cfi-space-evsi-bench/Cargo.toml
cargo clippy --manifest-path tools/cfi-space-evsi-bench/Cargo.toml --all-targets -- -D warnings

cargo run --quiet --manifest-path tools/cfi-space-evsi-bench/Cargo.toml -- \
  --scenario tools/cfi-space-evsi-bench/fixtures/synthetic-evsi-scenario-v0.1.json \
  --expect-evsi independent-reference-check \
  --expect-information-gain illumination-survey
```

The workflow runs the Rust selector and a separate Python standard-library challenger and diffs their full score receipts. Python is test-only; the selector and future production implementation remain Rust.

## Claim ceiling

This is a finite synthetic selector contract. It does not implement a sensor driver, physical observation model, vehicle controller, HIL test, autonomous maneuver or mission-specific safety case. EVSI is only as good as its priors, outcome likelihoods, loss function and dependency declarations. Real deployment requires independently calibrated observation likelihoods, validated mission losses, robust model-mismatch treatment, and separate action admission.