# Construction Reference-Class Learning Core — v0.1

**Status:** synthetic claim-scope and promotion validator. Not a statistical causal-inference engine, reference-class forecaster, worker-monitoring system or model-training pipeline.

Parent: [CONSTRUCTION-REFERENCE-CLASS-LEARNING-001 #5264](https://github.com/Luminous-Dynamics/symtropy/issues/5264).

The validator composes source-owned records from:
- accepted output #3605 and the floor economics / joint option stack;
- physical trial receipt #3521/#3522 and assurance chain #3566;
- evidence validity #1846 and survey authority #1607;
- blind challenge custody #1300 (where applicable).

## Evidence distinctions

- Descriptive cases remain local observations.
- Matching measured covariates creates a matched observational reference candidate; it does not establish causality.
- Quasi-experimental eligibility requires preregistration, explicit assumptions and passed pre-trend, placebo, shuffled-treatment, sensitivity and common-cause diagnostics. Its causal interpretation remains conditional on the assumptions.
- Randomized-design eligibility requires an allocation protocol/record, preregistration, intent-to-treat analysis, contamination review and cluster-corrected analysis.
- Independent replication requires cases across at least two projects and sites plus at least two distinct assessor references. A replicated descriptive association remains descriptive.

## What the crate checks

- Same exact functional unit, unit symbol, acceptance contract, currency and horizon across all cases.
- Attempted work reconciles to finally accepted + finally rejected + unresolved dispositions.
- First-pass denominators are consistent; no uninspected or unresolved output becomes accepted by implication.
- Evidence IDs and effect metrics are unique; source artifacts have explicit status/digest/claim references.
- Study-design case roles and group membership match their declared treatment/control/pre/post roles.
- Required diagnostics and negative controls are present and passed before causal eligibility.
- Train/validation/test/blind case partitions cannot share a site or project across splits.
- Future cases outside the declared applicability envelope are returned as out-of-domain or insufficient context, not extrapolated.

The effect-estimate values and intervals are caller-supplied outputs from a separately identified analysis report. This crate validates identity/shape/denominators; it does not recompute statistical effects or verify identification assumptions as true.

## Local commands

```sh
cargo fmt --manifest-path research/construction-reference-class-learning-v0-1/Cargo.toml -- --check
cargo test --manifest-path research/construction-reference-class-learning-v0-1/Cargo.toml --locked
cargo clippy --manifest-path research/construction-reference-class-learning-v0-1/Cargo.toml --locked --all-targets -- -D warnings
```

## Claim ceiling

V0.1 establishes only synthetic comparison-manifest, denominator, partition, applicability and promotion semantics. It does not establish that a construction method causes improvement, that a result transfers to another site, that a future productivity prediction is calibrated, or that a model should update.
