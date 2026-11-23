# EHD OED nuisance-adjusted portfolio reference

A bounded, deterministic two-physical-parameter / two-nuisance-parameter experiment-portfolio evaluator. It is an abstract linear-Gaussian benchmark kernel, not an EHD model, parameter estimator or experiment authorization.

## Run

    cargo fmt --manifest-path tools/ehd-oed-portfolio/Cargo.toml -- --check
    cargo test --manifest-path tools/ehd-oed-portfolio/Cargo.toml
    cargo clippy --manifest-path tools/ehd-oed-portfolio/Cargo.toml --all-targets -- -D warnings

The crate is a standalone Cargo workspace with no third-party dependencies.

## What it adds

- A full candidate-level measurement covariance matrix per robustness scenario, including cross-experiment common-mode covariance.
- Dimensionless correlation-matrix normalization before weighted calculations, so independent changes of observation units do not change acceptance.
- Physical Fisher information profiled against two declared nuisance modes using a Schur complement.
- Pairwise model-prediction separation profiled against the same nuisance subspace.
- Worst-case reduction over all declared sensitivity/prediction/covariance scenarios.
- Hard constraints for observation budget, required control tags and distinct condition-family coverage.
- Bounded exhaustive subset enumeration with a 16-candidate ceiling.
- A Pareto frontier across worst-case E-like information, D-like log determinant, model separation and cost.

## Interpretation rules

- No ridge regularization or pseudoinverse is used to rescue a singular nuisance block.
- A physical-only full-rank design can still be rejected after nuisance profiling.
- A model pair whose predictions differ only in the nuisance subspace is reported as non-discriminable.
- The engine returns all non-dominated portfolios; it does not invent a single composite score.
- An empty frontier carries an explicit abstention reason and rejection counters.

## Limits

- Exactly two physical parameters, two nuisance parameters and three model hypotheses.
- One scalar observation per candidate experiment.
- Candidate measurement covariance must be symmetric positive definite; this v0.1 implementation rejects singular covariance instead of using a pseudoinverse.
- Local linear sensitivities and Gaussian errors only. Nonlinear profile likelihood, posterior expected information gain, adaptive optimal control and physical EHD equations remain out of scope.
- The model owner is responsible for declaring nuisance directions and shared causes. The engine cannot discover omitted artifacts.
- Synthetic unit tests are manufactured software controls, not measured EHD operating points.
