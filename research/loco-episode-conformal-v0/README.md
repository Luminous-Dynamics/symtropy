# LOCO Episode Conformal V0

Dependency-free Rust reference for exact split-conformal rank calculation over independent episode/session blocks.

Research parent: [LOCO-EPISODE-CONFORMAL-001 / #4146](https://github.com/Luminous-Dynamics/symtropy/issues/4146).

## The unit of exchangeability

Legged state-estimation data are serially correlated within a rollout, across feet, and often across multiple episodes from the same session or terrain realization. V0 therefore does not count control ticks as calibration samples.

The input is one calibration block per declared independent exchangeability unit (normally an independently collected session or trial block). Each block retains all episode IDs and Completed/Failed/Aborted outcomes, and supplies one maximum nonconformity score computed over the complete frozen target for every included episode, foot, dimension and timestep.

The Rust utility cannot inspect the upstream trajectory tensor and prove that the score is truly the maximum. The score-definition digest and protocol receipt must establish that separately.

## Exact split-conformal rank

For n independent exchangeability units and miscoverage alpha:

    k = ceil((n + 1) * (1 - alpha))

V0 stores alpha in integer parts per million and performs the rank arithmetic without floating-point rounding.

If k <= n, the threshold is the k-th smallest calibration score (one-based). If k > n, the only valid distribution-free finite-sample answer is an unbounded threshold. The implementation never clamps k to n and never reports an invented finite guarantee.

Example: with alpha = 5%, n = 19 gives k = 19 and uses the maximum calibration score. n = 18 gives k = 19 > n and returns Unbounded.

## Identity and leakage checks

- Model, target, domain, score definition and calibration split digests must match across every block and the dataset manifest.
- Exchangeability unit IDs must be unique.
- Episode IDs must be unique across all blocks.
- Calibration episode IDs must not overlap the supplied fit/test episode ID manifest.
- A calibration receipt applies only to the exact same model, target, domain and score-definition identity.
- Applying a receipt to a new/OOD terrain domain returns NotApplicable; the old coverage statement is not reused.

Failed and aborted episodes remain in the input and count toward the block. The calibration pipeline must define a score for their observed prediction horizon and must not silently filter them.

## Implemented tests

Fourteen deterministic tests cover:
- n=19 / alpha=5% finite rank;
- n=18 / alpha=5% unbounded threshold;
- multiple episodes per one independent session block;
- failed and aborted episode retention;
- duplicate independent unit and episode rejection;
- fit/test leakage;
- model, target, domain, score-definition and calibration-split mismatch;
- invalid alpha and empty identity;
- tied scores and input permutation;
- exact rank ceiling arithmetic;
- zero and maximum integer scores;
- refusal to apply calibration to another domain or model.

## Current research baselines

- Cheng et al. (2026 ICRA), simultaneous noise-covariance and kinematics calibration for legged state estimation: https://www.leggedai.com/publication/2026icra_calibration/
- IUCA, model-agnostic in-situ calibration of correlated state uncertainty, IEEE Journal of Selected Areas in Sensors (7 September 2026): https://doi.org/10.1109/JSAS.2026.3731931
- Learnable Conformal Prediction, ICRA 2026: https://doi.org/10.1109/ICRA57385.2026.11697249
- Online Conformal Prediction via Online Optimization, ICML 2025: https://proceedings.mlr.press/v267/areces25a.html

These are comparator methods; V0 does not reproduce them or claim a better state estimator. Its narrow purpose is to qualify the exact episode-level split-conformal rank, identity boundary and leakage checks.

## Commands

    cargo fmt --manifest-path research/loco-episode-conformal-v0/Cargo.toml -- --check
    cargo test --manifest-path research/loco-episode-conformal-v0/Cargo.toml --locked
    cargo clippy --manifest-path research/loco-episode-conformal-v0/Cargo.toml --locked --all-targets -- -D warnings

Dedicated exact-head workflow: .github/workflows/loco-episode-conformal.yml.

## Claim ceiling

The conformal statement is finite-sample marginal whole-unit coverage under exchangeability of the declared independent units in the exact fixed domain. It is not pointwise conditional coverage, arbitrary domain-shift coverage, a safety probability, a physical viability certificate or proof that the estimator is correct. The upstream max-score and exchangeability assumptions require separate validation.