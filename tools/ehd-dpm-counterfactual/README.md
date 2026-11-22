# EHD diagnostic counterfactual qualification

A standalone Rust reference for randomized, drift-balanced qualification of diagnostic interventions such as PIV seeding, probes, illumination changes, and collector-surface treatments.

## Design

Each independent acquisition cluster is assigned one of two four-period schedules:

- ABBA: control, diagnostic, diagnostic, control
- BAAB: diagnostic, control, control, diagnostic

Period times must be symmetric. This cancels first-order linear drift within a block. Balanced ABBA/BAAB allocation also counterbalances a common quadratic drift component across the campaign, subject to the declared assumptions.

Every period requires a unique outcome-evidence ID, independently verifiable surface-reset receipt, and purge receipt. Physical ledgers and validation-envelope gates remain independent and take precedence over a small effect estimate.

Repeated blocks inside the same acquisition/common-cause cluster are averaged before campaign inference. They do not inflate the count of independent clusters.

## Decision rule

The evaluator calculates the diagnostic-minus-control effect within each block, averages blocks within each independent cluster, then computes the equal-weight campaign mean. A cluster-level t-based sampling margin is combined with conservative absolute measurement bounds.

A diagnostic is classed as within tolerance only when the complete resulting interval lies inside the predeclared effect tolerance. A non-significant or imprecise estimate is unresolved, not passive.

## Run

```sh
cargo test --manifest-path Cargo.toml
cargo clippy --manifest-path Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path Cargo.toml -- --check
```

These instructions do not mean that tests have already run. This environment does not currently expose cargo, rustc or rustfmt.

## Limitations

This crate evaluates supplied scalar measurements and external receipts. It does not perform or cryptographically verify randomization, verify a physical reset/purge, measure current/flow, estimate tracer charge, model electrode fouling, process PIV images, or measure thrust. ABBA/BAAB does not remove arbitrary nonlinear drift, hidden environmental changes, unmeasured carryover or shared common causes. The fixed t critical value assumes approximately normal independent cluster means.
