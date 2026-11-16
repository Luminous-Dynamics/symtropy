# PHRI-TAC-TRANSFER-001 synthetic oracle

A standalone Rust reference for the Symtropy #4776 cross-sensor tactile calibration-transfer contract.

Run from the repository root:

    cargo fetch --locked --manifest-path research/phri-tac-transfer-001/Cargo.toml
    cargo fmt --manifest-path research/phri-tac-transfer-001/Cargo.toml --all -- --check
    cargo test --locked --manifest-path research/phri-tac-transfer-001/Cargo.toml
    cargo clippy --locked --manifest-path research/phri-tac-transfer-001/Cargo.toml --all-targets -- -D warnings
    cargo run --locked --manifest-path research/phri-tac-transfer-001/Cargo.toml

The corpus and JSON Schema are language-neutral. The Rust deny_unknown_fields structs and semantic evaluator are the executable reference. The test suite includes mutations for sample/sequence/lot leakage, source-certificate reuse, target calibration mismatch, stale firmware, false metrology promotion, invalid reference independence and unqualified uncertainty.

Claim ceiling: synthetic reference only. A passing run does not qualify a real sensor, calibration, uncertainty interval, transfer model, robot task, human interaction or safety claim.
