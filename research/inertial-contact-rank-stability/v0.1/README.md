# Inertial Contact Rank Stability Oracle v0.1

A dependency-free Rust reference for deciding whether the **spectral row-rank** of an inertia-whitened contact-normal Jacobian is stable under declared mass and Jacobian perturbations.

This is a follow-on to #7232/#7233, not a replacement for nominal pivoted row selection.

## Perturbation model

For nominal SPD generalized mass M = L L^T, let:

    S = J_n L^-T
    H = L^-1 ΔM L^-T
    D = ΔJ_n L^-T

with declared spectral-norm bounds:

    ||H||_2 <= rho < 1
    ||D||_2 <= delta

The perturbed mass is M_tilde = L (I+H) L^T and a Gram-equivalent symmetric-root whitening is:

    S_tilde = (S + D) (I+H)^(-1/2)

Given a valid upper bound y_norm >= ||S||_2, the crate computes:

    epsilon_S =
        delta / sqrt(1-rho)
        + y_norm * (1/sqrt(1-rho) - 1)

This bounds ||S_tilde - S||_2 under the declared assumptions. Weyl's inequality then bounds the movement of each singular value by epsilon_S.

## Fixed absolute rank floor

A fixed absolute singular-value floor sigma_floor is used. For each nominal singular value sigma_i, the oracle returns:

    lower_i = max(0, sigma_i - epsilon_S)
    upper_i = sigma_i + epsilon_S

A direction is:
- robustly above the floor if lower_i > sigma_floor;
- robustly below the floor if upper_i < sigma_floor;
- uncertain if its interval touches or crosses the floor.

The robust rank is returned only when every interval is classifiable. Otherwise status is INCONCLUSIVE. The fixed singular-value floor is deliberately not conflated with #7233's relative RRQR-style pivot-residual threshold.

## Numerical reference

The crate uses a deterministic one-sided Jacobi SVD on the smaller column dimension of the rectangular candidate-row matrix. It scales the matrix before rotations, checks finite intermediates, records sweep count and final maximum column correlation, and fails closed if the configured sweep budget does not converge.

This is a small-dense binary64 reference, not an interval-arithmetic proof. For formal numerical guarantees, a higher-precision/interval SVD backend and independent certification are future work.

## Deliberate non-claims

The report distinguishes three different questions:

1. Is the spectral row-rank stable under the declared M/J envelope?
2. Is the greedy stable-ID basis sequence unchanged? The spectral envelope alone does not prove this, so v0.1 always returns NotCertifiedBySpectralEnvelope.
3. Are omitted unilateral/frictional contacts physically safe to remove? Always false here. Use #7203's full-patch KKT check for its restricted convex frictionless QP and separate model-specific certificates for frictional contact.

A robust low rank is only a robust numerical conditioning/seed property, not contact-law redundancy.

## Run

    cargo fmt --manifest-path research/inertial-contact-rank-stability/v0.1/Cargo.toml -- --check
    cargo test --manifest-path research/inertial-contact-rank-stability/v0.1/Cargo.toml --locked
    cargo run --manifest-path research/inertial-contact-rank-stability/v0.1/Cargo.toml --example rank_stability_tournament --locked
    cargo clippy --manifest-path research/inertial-contact-rank-stability/v0.1/Cargo.toml --locked --all-targets -- -D warnings

The standalone crate is explicitly wired into the three-platform repository CI workflow.

## Proposed targets (not achieved)

- 100% singular-value interval coverage against independently sampled mass/Jacobian perturbations;
- zero false robust-rank PASS around rank-loss boundaries;
- 100% INCONCLUSIVE on threshold-crossing intervals;
- rank and singular enclosures remain stable under correctly transformed non-orthogonal generalized coordinates;
- zero greedy basis-ID claims without pivot-margin enclosures;
- physical omission remains false in every report.

## Relationship

- #7232/#7233: nominal inertia-whitened contact seed selection.
- #7234: uncertainty-aware rank stability.
- #5076: robust Delassus operator envelope and rank uncertainty.
- #4818: coordinate-invariant operator spectrum.
- #7203/#7199: full-patch KKT certification and adaptive refinement.
- #7195/#7197: finite solver response and work/termination receipts.
- #7155/#7157: wrench support and task safety.
- #7130/#7212: generator lineage and production Delassus adapter.
- #460: canonical physics qualification corpus.
