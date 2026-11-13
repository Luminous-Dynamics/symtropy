# Rank-aware holonomic constraint quotient v0.1

First executable slice of [PHYS-HOLONOMIC-RANK-001 #4970](https://github.com/Luminous-Dynamics/symtropy/issues/4970).

## Why this exists

The full-row-rank constraint path represents the Delassus matrix

    W = J M^-1 J^T

as an SPD metric. That is correct only when J has full row rank. With redundant linear holonomic equations, W is positive semidefinite and singular. Rejecting such a case as "bad physics" is incorrect; pretending W is invertible is also incorrect.

For a compatible right-hand side b, the equation W lambda = b has an affine multiplier fibre lambda_0 + ker(W). Because M is SPD,

    ker(W) = ker(J^T).

Therefore all representatives produce the same generalized constraint force J^T lambda and the same kinetic correction M^-1 J^T lambda. This reference represents the physical response and the multiplier fibre separately.

## Coordinate-covariant rank analysis

A constraint-row basis change uses

    b_new = T b_old
    lambda_new = T^-T lambda_old
    W_new = T W_old T^T.

An explicit SPD residual-space metric G must transform as

    G_new = T^-T G_old T^-1.

With G = L L^T, analyze the normalized PSD operator

    W_hat = L^T W L
    b_hat = L^T b
    lambda_hat = L^-1 lambda.

Under a coherent row re-basing, W_hat is orthogonally similar to its old representation. Its normalized spectrum and rank classification are therefore basis invariant up to the declared floating-point profile.

The implementation never adds epsilon I and never silently drops a row.

## Canonical representative is a gauge, not a physical uniqueness claim

When the right-hand side is compatible, v0.1 returns the minimum dual-metric-norm representative. This is a declared gauge convention, not a claim that the physical multipliers are unique. The returned nullspace basis describes the full multiplier fibre.

The generalized force and velocity correction are the response invariants. Contact/friction is expressly excluded: per-contact Coulomb disks may distinguish different allocations in a multiplier nullspace.

## Fixtures and metrics

The exact rational fixture uses M = diag(1,2,4) and

    J = [[1,0,0],
         [0,1,0],
         [1,1,0]].

Its W is

    [[1, 0,   1],
     [0, 1/2, 1/2],
     [1, 1/2, 3/2]]

with exact left-null vector [-1,-1,1]. Rows 0 and 1 are independent and row 2 is their sum, proving rank(J)=rank(W)=2 and multiplier nullity one.

The tests cover an SPD baseline, the exact PSD witness, an incompatible right-hand side, equivalent multiplier representatives, generalized force/velocity invariance, coherent non-orthogonal row re-basing, a near-rank threshold band, and invalid metrics.

## Reproduction

    cargo fmt --manifest-path research/holonomic-rank-quotient-001/Cargo.toml --all -- --check
    cargo test --locked --manifest-path research/holonomic-rank-quotient-001/Cargo.toml
    cargo clippy --locked --manifest-path research/holonomic-rank-quotient-001/Cargo.toml --all-targets -- -D warnings

## Claim boundary

This is a small dense f64 reference for linear bilateral/holonomic constraints. The rank and spectrum are numerical estimates except where the separate exact rational fixture argument applies. It does not qualify unilateral/frictional contact, nonlinear manifolds, reaction-dependent constitutive laws, sparse articulated solvers, or physical validation. Near-threshold cases abstain rather than being promoted.
