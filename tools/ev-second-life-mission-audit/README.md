# Second-life battery mission qualification audit v0.1

Dependency-free Rust 1.96 read-only evaluator for second/third-life mission evidence and hard-gate consistency.

Checks exact pack/configuration identity, complete cell membership, first-life history coverage, incident-history adjudication, competent-authority thermal decision, BMS target-mission verification, admitted evidence for all ten hard gates, lower-bound mission capability plus reserve, rights and deterministic reports.

Synthetic known-answer fixtures can only return `synthetic_fixture_only`; they can never return a real eligible disposition.

## Commands

    cargo metadata --manifest-path tools/ev-second-life-mission-audit/Cargo.toml --locked --no-deps
    cargo fmt --manifest-path tools/ev-second-life-mission-audit/Cargo.toml -- --check
    cargo test --manifest-path tools/ev-second-life-mission-audit/Cargo.toml --locked
    cargo clippy --manifest-path tools/ev-second-life-mission-audit/Cargo.toml --all-targets --locked -- -D warnings

No battery health model, BMS action, physical test, disassembly, transport or commissioning authorization is included.
