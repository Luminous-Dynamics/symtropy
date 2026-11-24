# Inertial Contact Rank Oracle v0.1

A dependency-free Rust reference that proposes a low-dimensional basis of contact normal rows in the generalized-inertia metric.

## Why this is different from geometric clustering

For an SPD generalized mass matrix:

    M = L * L^T

and normal contact Jacobian J_n, form:

    S = J_n * L^-T

so that:

    W = J_n * M^-1 * J_n^T = S * S^T

The whitened row S_i measures the contact row's kinematic leverage under the inertia metric. Two contacts with similar normals or nearby positions can have different inertial leverage because their generalized Jacobian rows differ.

The crate computes S using lower-triangular solves; it never materializes M^-1.

## Selection algorithm

1. Validate mass symmetry, dimensions, finiteness and positive-definite Cholesky factorization.
2. Whiten each normal Jacobian row.
3. Run deterministic pivoted modified Gram-Schmidt with two re-orthogonalization passes.
4. Select the largest remaining projection residual; stable contact ID breaks exact ties.
5. Stop when the next pivot is at or below:

       eta = rank_relative * first_pivot_norm

6. Return the pivot order, excluded IDs, per-row projection residuals, threshold, Cholesky diagnostics, every whitened candidate row and the orthonormal selected basis. The full whitened rows let an independent consumer recompute the Delassus Gram matrix and replay selection.

This is an RRQR-style reference based on pivoted, re-orthogonalized MGS. It is not a strong-RRQR or Householder-QR implementation. Pivot residual ratios are not reported as spectral condition numbers.

## Critical claim boundary

**A row-space basis is not a contact-deletion certificate.**

Linear dependence alone does not prove that unilateral contact inequalities or friction-cone complementarity constraints are redundant. Contact bias/free velocity, unilateral signs, friction directions and active-set semantics still matter.

Accordingly:
- physical_omission_certified is always false;
- excluded contacts are labelled BelowInertialRankThreshold;
- this output may seed #7203's full-patch KKT check in the admitted frictionless convex QP lane;
- a failed KKT check must add the violated omitted contact and re-solve;
- for frictional LCP/SOCCP/TGS, rank selection is conditioning evidence only until a model-specific omission certificate exists.

## Coordinate covariance

For an invertible generalized-coordinate rebasing q = T q':

    M' = T^T * M * T
    J_n' = J_n * T

the physical Delassus matrix is unchanged:

    J_n' * (M')^-1 * (J_n')^T = J_n * M^-1 * J_n^T

The whitened row Gram matrix is therefore invariant. Rank, stable-ID pivot selection and pivot residual norms should agree within a declared floating-point envelope, except for threshold-adjacent cases that require an explicit inconclusive band.

## Run

    cargo fmt --manifest-path research/inertial-contact-rank-oracle/v0.1/Cargo.toml -- --check
    cargo test --manifest-path research/inertial-contact-rank-oracle/v0.1/Cargo.toml --locked
    cargo run --manifest-path research/inertial-contact-rank-oracle/v0.1/Cargo.toml --example inertial_rank_tournament --locked
    cargo clippy --manifest-path research/inertial-contact-rank-oracle/v0.1/Cargo.toml --locked --all-targets -- -D warnings

The standalone crate is wired into the existing three-platform repository CI workflow.

## CUSP relationship

Azizkhani & Chen's 2026 CUSP soft-robot framework uses:
1. inertia-whitened RRQR on normal rows;
2. Ruiz equilibration of the coupled LCP;
3. light Tikhonov regularization on normal-force blocks.

Paper: https://arxiv.org/abs/2602.21316, Sections III-D1–III-D3.

This crate implements the first stage as a seed proposer. Ruiz scaling and Tikhonov are deliberately separate follow-on experiments because scaling must preserve variable mappings, while regularization changes the mathematical contact model and must be quantified.

## Proposed targets (not achieved)

- zero false physical-omission claims from row-space rank alone;
- 100% stable-ID and input-order invariance outside the declared rank threshold band;
- dense non-orthogonal coordinate-rebasing drift within a preregistered tolerance;
- exact duplicate-row families reduce to their independent whitened rank;
- every row's projection residual is retained in the evidence record;
- end-to-end work savings are claimed only when #7203's full-patch KKT certificate and solver/task gates pass.

Related programme: #7232, #5076, #4818, #7203, #7199, #7195, #7155, #7130, #7212 and #460.
