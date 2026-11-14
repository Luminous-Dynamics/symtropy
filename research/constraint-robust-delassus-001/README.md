# Robust Delassus uncertainty envelope v0.1

Starts [PHYS-CONSTRAINT-ROBUST-001 #5076](https://github.com/Luminous-Dynamics/symtropy/issues/5076).

## Bound

For SPD `M = L Lᵀ` and full-row-rank `J`, define `Y = J L⁻ᵀ`, so `W = J M⁻¹ Jᵀ = Y Yᵀ`. For symmetric `ΔM` and arbitrary `ΔJ`, define `H = L⁻¹ ΔM L⁻ᵀ`, `D = ΔJ L⁻ᵀ`.

Let `rho >= ||H||₂`, `delta >= ||D||₂`, and `y_norm >= ||Y||₂`. If `rho < 1`, then

`W_tilde = (Y + D) (I + H)⁻¹ (Y + D)ᵀ`

and

`||W_tilde − W||₂ <= (rho*y_norm² + 2*delta*y_norm + delta²)/(1-rho)`.

Using Weyl's inequality, the robust lower eigenvalue estimate is `lambda_min(W) - epsilon_W`. v0 uses Frobenius estimates for `rho`, `delta` and `y_norm`, so all resulting envelopes are numerical estimates, not formally verified interval bounds.

## Implementation

- Cholesky whitening without forming `M⁻¹`.
- Small symmetric Jacobi eigensolver for nominal `lambda_min(W)`.
- Frobenius upper estimates for whitened mass/Jacobian perturbations and whitened constraint rows.
- Robust full-row-rank admission only when the lower eigenvalue estimate exceeds a declared floor.
- Inverse-Delassus sensitivity bound only when `epsilon_W < lambda_min(W)`.
- Frame-typed tangent/covector values and dense-coordinate re-basing of `M`, `ΔM`, `J`, and `ΔJ`.
- Explicit abstention for `rho >= 1`, rank-loss/near-rank-loss, non-finite inputs and non-SPD metrics.

## Reproduction

```sh
cargo fmt --manifest-path research/constraint-robust-delassus-001/Cargo.toml --all -- --check
cargo test --locked --manifest-path research/constraint-robust-delassus-001/Cargo.toml
cargo clippy --locked --manifest-path research/constraint-robust-delassus-001/Cargo.toml --all-targets -- -D warnings
```

## Limitations

v0.1 covers small dense SPD mass matrices and full-row-rank linear constraints. Frobenius upper bounds can be conservative. It does not yet cover unilateral contact, friction cones, complementarity, nonlinear manifolds, redundant constraints, sparse articulated dynamics, or verified interval arithmetic. Do not infer hard-contact or trajectory robustness from this reference.
