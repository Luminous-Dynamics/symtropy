# Executable acceptance evaluator (v0.1)

This isolated crate is a **semantic reference evaluator** for the as-built magnetic acceptance contract.

## What it does

- checks specimen identity and evidence-role closure;
- binds manufacturing lot/build, inspected as-built geometry, material-state and post-magnetization records;
- requires the solver prediction to consume the exact as-built geometry and upstream process/material/magnetization record IDs;
- requires the physical observation, solver, exact-grid comparison and continuous-domain certificate to share one target/frame/grid/operating-condition scope;
- rejects calibration/holdout overlap and incomplete common-cause uncertainty evidence;
- emits non-compensatory QUALIFIED / FAIL / INDETERMINATE / NOT_EVALUABLE outcomes with typed blockers.

## What it does not do

It does not parse raw laboratory data, calculate electromagnetic fields, propagate covariance, establish interval containment, authenticate the identity of evidence producers, verify calibration certificates, validate a solver, or issue product release. Upstream records are treated as claims whose competence must be established by their owners.

## Runner-independent review

The crate uses Rust 2024 and the standard library only. Unit tests are authored for positive closure, wrong-specimen and nominal/as-built mismatches, missing magnetization, definite versus unresolved failures, spatial coverage, holdout leakage, common-cause evidence, unsupported regimes, comparison binding and uncertainty monotonicity.

The current execution environment has no Rust toolchain, so compilation and test execution have **not** been performed locally. CI / exact-head workflow evidence remains separate.
