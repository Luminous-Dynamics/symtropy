# Robotics Benchmark Validity Auditor

A standalone Rust tool that audits the *evidence structure* of robotics evaluations before interpreting their scores.

## Run

```sh
cargo test --manifest-path tools/robotics-benchmark-audit/Cargo.toml
cargo run --quiet --manifest-path tools/robotics-benchmark-audit/Cargo.toml -- \
  path/to/evaluation.json > robotics-benchmark-audit-receipt.json
```

The evaluator manifest schema is `schemas/robotics/benchmark_evaluation_v0_1.schema.json`. The focused contract and proposed metrics are documented in `docs/research/ROBOTICS_BENCHMARK_VALIDITY_AND_GENERALIZATION_V0_1.md`.

## Required controls

- One conservative baseline, reactive baseline, metadata-only baseline, and candidate policy.
- Separate train, validation, in-distribution test, OOD test, and adversarial test scenario splits.
- Test-source IDs must be held out from all declared policy training sources; tuning on any test split is rejected.
- Every scenario/seed/evidence-class group must contain all four policy classes exactly once.
- Run actions for non-run dispositions, invalid safe-degradation labels, task success on a non-run scenario, and any declared safety violation fail the audit.
- Report metrics by split, evidence class, and policy class; never mix simulation and field observations.
- Candidate-versus-conservative-baseline success differences use a deterministic, family-cluster bootstrap. Fewer than five independent scenario families is reported as insufficient for an inferential claim.
- The receipt includes the scenario-manifest SHA-256 and reproducible per-group and paired results.

## Claim ceiling

The smoke tests exercise synthetic records, not measured robot performance. A passing evaluation audit does not establish physical safety, simulator fidelity, regulatory compliance, or general robot capability.
