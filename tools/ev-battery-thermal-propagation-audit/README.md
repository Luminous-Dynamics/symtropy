# EV battery thermal-propagation evidence audit v0.1

Dependency-free Rust 1.96 read-only evidence-transfer auditor.

It checks exact test article and configuration identity, application/test-level scope, physical-vs-risk-analysis evidence classes, trigger and result semantics, source digests, metrology references, six product-change equivalence domains and deterministic reports.

## Commands

    cargo metadata --manifest-path tools/ev-battery-thermal-propagation-audit/Cargo.toml --locked --no-deps
    cargo fmt --manifest-path tools/ev-battery-thermal-propagation-audit/Cargo.toml -- --check
    cargo test --manifest-path tools/ev-battery-thermal-propagation-audit/Cargo.toml --locked
    cargo clippy --manifest-path tools/ev-battery-thermal-propagation-audit/Cargo.toml --all-targets --locked -- -D warnings

All included examples are synthetic. A candidate-for-human-review disposition is not a safety determination, standard conformance or product approval.
