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


### Selector/evaluator separation — hardened

The active-policy API receives only `&[Candidate]`. A candidate contains a commanded stimulus setpoint and a sensor-derived feature vector; it does not contain a reference-force label. Reference-force observations live in a separate `SyntheticDomain` used by model fitting after selection. Held-out target truth lives in `HeldoutDomain` and is passed only to the evaluator after every policy selection is frozen. The synthetic reference channel also has a separate deterministic noise term. This is a code-level data boundary, not merely a statement that the selector promises not to read labels.

The candidate setpoint is a declared stimulus, not an independently measured force. The held-out evaluator compares predictions with its separate synthetic truth. The regression prediction interval still does not constitute a metrological uncertainty budget.


## Single-seed policy comparison — preliminary only

The first hosted execution on 16 November 2026 completed the Rust test suite, strict Clippy and tracked-corpus CLI. For the one frozen seed in v0.1, the 16-contact D-optimal arm produced a 0.004986 N synthetic RMSE versus 0.004234 N for the 72-contact target-grid reference. Uniform and maximin were substantially worse on this response surface; the source-only arm was poor under the declared source/target shift; the transfer-prior arm traded somewhat better worst-case error/interval diagnostic coverage against greater total source+target contact cost. These are deterministic synthetic results, not physical calibration measurements or a stable policy ranking.

This result is deliberately not the campaign conclusion. [PHRI-TAC-ROBUST-001 #5591](https://github.com/Luminous-Dynamics/symtropy/issues/5591) now owns paired multi-seed statistics, model-misspecification families, reference-channel common-mode/noise injection and per-stratum failure analysis. No policy should be selected from this single seed.


## Paired multi-seed robustness campaign

The follow-on executable exposes `run_active_calibration_campaign(first_seed, replicate_count)`. It requires at least 50 paired seeds and evaluates every policy on the same source, target and held-out domains for each seed. Each policy reports mean, median, p95 and maximum values for synthetic RMSE, p95/worst absolute error, prediction-interval coverage/width and generated fixture-cost proxy. It also reports the per-seed paired RMSE difference against the full target-grid arm.

The first implementation is a stability screen, not a complete statistical analysis: bootstrap confidence intervals, cluster-aware inference, model-family misspecification, common-mode reference faults and multiple physical target articles remain follow-on requirements under #5591. These repeated runs remain synthetic and do not establish physical uncertainty coverage or calibration savings.
