# CivDyn Configuration Sensitivity v0.1

A dependency-free Rust reference for auditing decision instability across complete AI decision configurations.

## Audit identity

A configuration includes:
- model ID and model version;
- persona;
- interview protocol;
- facilitator/chair;
- aggregation rule;
- option/order profile;
- prompt template;
- tool policy;
- evidence snapshot;
- decoding policy.

The caller supplies a canonical `sha256:<64 hex>` digest over the complete decision-relevant configuration and task contract. This crate validates its syntax and uniqueness; it does not compute or authenticate the digest. A changed digest is a new audit identity and must be re-evaluated.

## Input boundary

Every configuration must be evaluated on the exact same case IDs and declared cohort/slice labels. Duplicates, missing cases, cohort relabelling, model/version mixtures and digest reuse fail closed. Cohort labels are explicitly provided by the data owner; this crate never infers sensitive attributes.

This v0.1 benchmark compares one model/version across configurations. Model-to-model comparisons belong in separate, protocol-matched benchmark runs.

## Reported metrics

- Overall selected numerator / case denominator and selection rate per configuration.
- Declared-cohort numerators, denominators and selection rates.
- Selection-rate ratio to an explicitly named reference cohort; undefined when the reference rate is zero.
- Signed rate gap to the reference cohort.
- Paired per-case flip count and rate against a declared baseline configuration.
- Exact configuration-surface minimum, maximum and selection-rate range.
- Matched single-facet contrasts only when exactly one of nine declared facets differs, with overall and cohort-specific rate changes.

A configuration pair changing multiple facets is never labelled a single-factor effect. These are finite-sample descriptive measurements; the kernel does not create confidence intervals, causal attribution or a universal fairness score.

## Synthetic control

The executable uses twelve synthetic cases across two declared cohorts and three configurations:
1. Natural-order baseline.
2. Semantic option permutation that preserves case decisions.
3. Position-biased order variant that flips four cases and reverses the cohort selection-rate ratio.

The fixture is an analytical software control, not human-subject data and not a reproduction of EMNLP 2026 results.

## Run

```sh
cargo fmt --manifest-path research/civdyn-config-sensitivity/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path research/civdyn-config-sensitivity/Cargo.toml --all-targets -- -D warnings
cargo check --locked --manifest-path research/civdyn-config-sensitivity/Cargo.toml --all-targets
cargo test --locked --manifest-path research/civdyn-config-sensitivity/Cargo.toml --all-targets
cargo run --locked --manifest-path research/civdyn-config-sensitivity/Cargo.toml --bin configuration-sensitivity-sweep
```

## Research context

Loghmani et al., *Hidden Configuration Effects on Fairness in LLM-Based Multi-Agent Decision Systems*, EMNLP 2026, report that persona, chair and speaking order materially affect fairness outcomes in their evaluated systems:
https://aclanthology.org/2026.emnlp-main.2531/

The objective is not to force all configurations to yield identical decisions. It is to make configuration-dependent changes visible, attributable to controlled single-facet comparisons where possible, and subject to re-audit. A measured disparity is not, by itself, proof that the allocation rule is unjust; interpretation depends on the task, valid criteria, data quality and declared normative framework.
