# PHRI-TAC-TRANSFER-001 synthetic oracle

A standalone Rust reference for the Symtropy #4776 cross-sensor tactile calibration-transfer contract.

Run from the repository root:

    cargo fetch --locked --manifest-path research/phri-tac-transfer-001/Cargo.toml
    cargo fmt --manifest-path research/phri-tac-transfer-001/Cargo.toml --all -- --check
    cargo test --locked --manifest-path research/phri-tac-transfer-001/Cargo.toml
    cargo clippy --locked --manifest-path research/phri-tac-transfer-001/Cargo.toml --all-targets -- -D warnings
    cargo run --locked --manifest-path research/phri-tac-transfer-001/Cargo.toml

## Active calibration planner

The library also exposes a coverage-first planner in `src/active_plan.rs`. It ranks only candidate contacts whose target unit/profile, phantom revision, reference calibration, method and operating conditions match the request. Mandatory operating strata are covered before optional contacts are ranked by a model-supplied information-per-cost score.

That score is a prioritization hint, not a measurement-uncertainty calculation. The planner does not operate a fixture, create force labels, certify calibration or authorize a physical contact. Seven unit tests cover coverage precedence, stale reference/method/condition rejection, exact identity binding, budget shortfall, deterministic tie-breaking, duplicate identities and invalid ranking scores.

The corpus and JSON Schema are language-neutral. The Rust deny_unknown_fields structs and semantic evaluator are the executable reference. The test suite includes mutations for sample/sequence/lot leakage, source-certificate reuse, target calibration mismatch, stale firmware, false metrology promotion, invalid reference independence and unqualified uncertainty.

Claim ceiling: synthetic reference only. A passing run does not qualify a real sensor, calibration, uncertainty interval, transfer model, robot task, human interaction or safety claim.
