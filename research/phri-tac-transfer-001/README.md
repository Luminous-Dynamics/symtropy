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


## Active calibration policy comparison

The crate also exposes a deterministic synthetic nonlinear response-surface comparison in `src/active_benchmark.rs`.

The runner compares:
- full target grid reference;
- uniform stratified;
- seeded random stratified;
- maximin stratified;
- D-optimal stratified;
- source-model transfer without target labels;
- source-model prior plus D-optimal target adaptation.

The target candidate pool has 72 contacts across four temperature × loading-phase strata. The held-out target domain has 60 interleaved force/position/condition cases that are not passed to any selector. Sparse target policies receive 16 target reference contacts (four per mandatory stratum). Source reference contacts and target reference contacts are reported separately: the transfer-only arm uses source contacts only (zero target calibration contacts), while transfer-plus-adaptation reports both source and target contact budgets.

Results report synthetic regression RMSE, p95/worst absolute error, a regression prediction-interval diagnostic, and a transparent synthetic contact-cost proxy. The interval is not a GUM uncertainty budget, metrological uncertainty statement or calibrated physical coverage result. The cost proxy is not measured laboratory time. These are synthetic model estimates only.

The target response profile is deliberately shifted from the source response profile. The harness is intended to reveal whether an active policy or transfer prior helps under sensor shift; no policy is presumed superior. Use multiple preregistered seeds and keep the full held-out set out of policy selection.
