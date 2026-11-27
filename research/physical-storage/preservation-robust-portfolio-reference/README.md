# Preservation Robust Portfolio Reference

Dependency-free Rust reference for a static robust preservation intervention portfolio.

## Contract

- A frozen scenario catalog is bound by an ID and SHA-256 digest.
- Every catalog scenario is mandatory.
- Each eligible action must contain one externally assessed upper-bound vector per scenario.
- Missing or unverified scenario bounds exclude a candidate; invalid bounds remain distinct.
- One selected action set must satisfy all resource caps in every mandatory scenario.
- The exact Pareto frontier minimizes worst-case cost, energy, elapsed duration, downtime and action count.
- Every objective records all scenarios tied for its maximum.
- Plans are proposals only: auto-execution is false and original sources remain retained.

No scenario probabilities, expected values, chance constraints or CVaR are inferred. The selected action set is fixed before the scenario is known; this is not adaptive recourse.

## Evidence and claim limits

This crate checks structure, evidence status, scenario coverage, dependencies, approvals, source-retention flags and arithmetic. It does not authenticate receipts, qualify the estimator, prove that bounds are conservative, or validate a physical measurement boundary.

A synthetic robust pass is conditional on the frozen catalog and supplied bounds. It is not a guarantee against unlisted events and is not physical reliability evidence.

## Local checks

Run cargo fmt --check, cargo clippy --all-targets -- -D warnings, and cargo test.

## Scale boundary

The exact reference is capped at 12 actions (4,095 non-empty subsets). Scenario evaluation adds work per subset; this is a correctness oracle, not a scalable optimizer. Larger instances belong in the PAUGMECON, SAUGMECON, MOBAB-CP and CP-SAT tournament.

Protocol: research/physical-storage/PRESERVATION_ROBUST_PORTFOLIO_001.md  
Frozen corpus: research/physical-storage/fixtures/preservation_robust_portfolio_cases_v0_1.json  
Tracking: https://github.com/Luminous-Dynamics/symtropy/issues/7703
