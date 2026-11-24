# Planar Patch Calibration v0.1

A dependency-free Rust reference for weighted boundary-equation fitting and identifiability diagnostics of the aggregate planar patch friction ellipsoid in #7216.

## Declared observation model

For each declared sliding/boundary observation:

    (f_u / (c e_u))^2 + (f_v / (c e_v))^2
      + (m_n / (c L (e_n/L)))^2 = 1

where:
- c = mu * N is the independently supplied friction/load capacity scale for that observation;
- L is a declared positive characteristic patch length in metres;
- e_u and e_v are dimensionless tangential shape scales;
- e_n is the torsional shape scale in metres.

Define dimensionless shape vector q = [e_u, e_v, e_n/L], beta_i = 1/q_i^2, and:

    x = [(f_u/c)^2, (f_v/c)^2, (m_n/(c L))^2]

Then the boundary equation becomes x · beta = 1.

Nondimensionalizing torsion before rank analysis is essential: otherwise the fit would compare squared force units with squared moment units and the rank threshold would depend on arbitrary unit choices.

## V0.1 estimator

The reference applies positive observation weights using sqrt(weight) row scaling, then uses deterministic pivoted, twice-reorthogonalized modified Gram-Schmidt on the three design columns. It solves the resulting 3x3 upper-triangular least-squares system without forming normal equations.

The reported shape scales are:

    e_u = 1/sqrt(beta_u)
    e_v = 1/sqrt(beta_v)
    e_n = L/sqrt(beta_n)

The report preserves:
- specimen, patch, coordinate-frame, rig and observation-profile digests;
- characteristic length and observation/trial-group counts;
- design rank, rank threshold and pivot order/norms;
- each axis's original weighted column norm and final residual after projection onto the full fitted basis;
- dimensionless inverse-squared scales and physical shape scales when identified;
- maximum absolute and weighted RMS normalized-boundary residual;
- separate held-out maximum-dissipation directional residuals and observed-power violations;
- explicit Unidentifiable, IdentifiedWithinResidualTolerance or IdentifiedPoorBoundaryFit status.

A rank-deficient experiment returns no shape point estimates. The algorithm does not add Tikhonov regularization to manufacture values for unexcited axes.

## What this does not estimate

## Held-out maximum-dissipation validation

The same crate also evaluates a supplied calibrated shape vector against held-out sliding observations. For axes a = [c e_u, c e_v, c e_n] and slip v = [v_u, v_v, omega_n], it predicts:

    w_i = -(a_i^2 * v_i) / ||diag(a) v||

It reports per-observation predicted wrench, measured/predicted power and dimensionless normalized wrench error, plus weighted RMS and maximum error. Positive measured power beyond its declared tolerance is counted separately and makes the validation inconsistent.

Zero-slip observations are preserved as ZERO_SLIP_STICKING_SET_NOT_DIRECTIONAL and excluded from directional scoring. If every observation has zero slip, the status is NO_INFORMATIVE_SLIDING_OBSERVATIONS, never a zero-error pass. The receipt binds a calibration-parameter digest and held-out split digest; the validation function does not refit the parameters.

This is an independent local-law check, not a joint estimator. The v0.1 fitter still identifies shape parameters from boundary observations only; it does not infer unknown mu separately from normal-load calibration, correlated sensor covariance or evolving pressure/contact dynamics.

## Run

    cargo fmt --manifest-path research/planar-patch-calibration/v0.1/Cargo.toml -- --check
    cargo test --manifest-path research/planar-patch-calibration/v0.1/Cargo.toml --locked
    cargo run --manifest-path research/planar-patch-calibration/v0.1/Cargo.toml --example patch_calibration_tournament --locked
    cargo clippy --manifest-path research/planar-patch-calibration/v0.1/Cargo.toml --locked --all-targets -- -D warnings

The standalone crate is explicitly qualified by the existing three-platform repository CI workflow.

## Proposed targets (not achieved)

- zero point estimates for rank-deficient trials;
- full-rank recovery on independently excited tangential-u, tangential-v and torsional boundary fixtures;
- physical torsional scale invariant to the chosen numerical characteristic length;
- held-out boundary and maximum-dissipation direction residual coverage reported separately from training fit;
- zero-slip sticking samples never enter the directional score;
- no calibration claim without specimen, rig, frame and observation-profile identities;
- robust/Bayesian parameter intervals and maximum-dissipation direction fitting are future extensions.

## Claim ceiling

This is a deterministic weighted least-squares identifiability reference, not a calibrated physical patch model. Synthetic recovery tests do not establish experimental accuracy, point-contact equivalence or production-solver correctness.

Related work: #7236, #7216, #7155, #7157, #7195, #7197, #7234, #7235 and #460.
