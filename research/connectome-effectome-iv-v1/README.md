# Connectome Effectome IV V1

A dependency-free Rust implementation of a narrow first step toward an experimentally estimated locomotion effectome: scalar endogenous source activity, scalar downstream outcome, and one or more randomized instruments.

**Parent issue:** [#3540 — LOCO-EFFECTOME-001](https://github.com/Luminous-Dynamics/symtropy/issues/3540)  
**Related work:** #3375 (intervention design), #3450 (identifiability), #3454 (blind prediction), #3534 (source evidence contract), #3538 (connectome uncertainty).

## What is estimated

For observations (i=1,ldots,n), V1 estimates:

`y_i = intercept + beta * x_i + error_i`

where the source activity `x` may be endogenous (correlated with omitted/common causes), and the instrument vector `z_i` is declared by the experimental design. A first-stage projection predicts `x` from the instruments. The second stage estimates the scalar effective response coefficient using the projected source activity.

The implementation:
- accepts one or more instrument columns;
- includes an intercept;
- checks observation count, finite values, instrument schema, and full column rank;
- reports first-stage R² and F statistic;
- reports an HC0 heteroskedasticity-robust standard error for the scalar 2SLS coefficient;
- includes OLS as an explicitly labelled observational comparator;
- returns the first-stage coefficients, fitted values, residual diagnostics and sample count;
- keeps instrument assumptions in a separate caller-supplied audit record.

## Assumptions are not magically testable

Random assignment can be audited from the experimental procedure. Relevance has an empirical first-stage diagnostic. The exclusion restriction (assignment affects the outcome only through the declared endogenous source) is generally not proven by the same regression. It requires design justification, off-target pathway assessment, negative controls, and sensitivity analysis.

`InstrumentAssumptionAudit` stores the declared status and evidence reference for randomization, exclusion, timing/carryover and interference. A status of `SupportedByDesign` means the caller has documented its justification; it is not a statistical proof. An estimator result cannot be called causally qualified unless every required assumption is declared supported with a non-empty evidence reference. Unknown or violated assumptions remain visible. The crate never silently downgrades to OLS.

## Connectome prior boundary

V1 does **not** yet implement a Bayesian connectome prior. It provides the scalar IV baseline required to test causal estimation under confounding. A subsequent phase can compare:
- no-connectome IV;
- anatomy-only point predictions;
- source-uncertainty-aware connectome-informed priors.

Any future prior must permit a zero effective effect on an anatomical edge and must not create a direct monosynaptic claim from an indirect total effect.

## Synthetic verification

Known-answer fixtures cover:
- confounding that biases OLS while valid IV recovers the structural coefficient;
- an invalid direct instrument-to-outcome path, which biases the IV point estimate but is not automatically "detected" by the regression;
- caller-declared invalid/unknown assumption status that blocks causal qualification;
- weak first stage and rank-deficient instrument design;
- duplicate/blank instrument IDs, inconsistent feature width, and non-finite observations;
- row-order invariance and exact known-answer recovery.

All fixtures are synthetic. Passing tests validate code behavior only. They do not establish instrument validity in a biological experiment or estimate a real locomotion effectome.

## Run

```sh
cargo fmt --manifest-path research/connectome-effectome-iv-v1/Cargo.toml -- --check
cargo test --manifest-path research/connectome-effectome-iv-v1/Cargo.toml --locked
cargo clippy --manifest-path research/connectome-effectome-iv-v1/Cargo.toml --locked --all-targets -- -D warnings
```
