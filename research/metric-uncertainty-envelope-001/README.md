# Coordinate-covariant metric uncertainty envelope v0.1

Starts [PHYS-METRIC-UNCERTAINTY-001 #5071](https://github.com/Luminous-Dynamics/symtropy/issues/5071).

## Problem

For nominal `M a0 = Q`, the true system is `(M + ΔM) a = Q + ΔQ`, where `M` is SPD and `ΔM` is symmetric.

Let `M = L Lᵀ`, `H = L⁻¹ ΔM L⁻ᵀ`, and `g = L⁻¹ ΔQ`. In whitened coordinates:

`(I + H) x = x0 + g` and `x − x0 = (I + H)⁻¹(g − H x0)`.

If `ρ >= ||H||₂` and `ρ < 1`, then:

`||a − a0||_M <= (||ΔQ||_(M⁻¹) + ρ ||a0||_M) / (1 − ρ)`.

V0 uses the Frobenius norm of the whitened perturbation as an upper estimate for the spectral norm. The implementation inflates the computed value by a declared roundoff margin. This remains a **numerical estimate**, not a formally verified interval bound.

## Implementation

- Frame-typed tangent and covector values.
- SPD mass metric with Cholesky factorization.
- Symmetric mass-perturbation validation.
- Cholesky whitening of `ΔM` as `L⁻¹ ΔM L⁻ᵀ` without forming `M⁻¹`.
- Stable Frobenius accumulation and an explicit `ρ < 1` gate.
- Dual-metric force uncertainty norm and nominal mass norm.
- Typed informative, weak, outside-envelope and vacuous statuses.
- Dense re-basing of metric, mass perturbation, force covector and acceleration.

## Reproduction

```sh
cargo fmt --manifest-path research/metric-uncertainty-envelope-001/Cargo.toml --all -- --check
cargo test --locked --manifest-path research/metric-uncertainty-envelope-001/Cargo.toml
cargo clippy --locked --manifest-path research/metric-uncertainty-envelope-001/Cargo.toml --all-targets -- -D warnings
```

## Limitations

This is a bounded dense SPD reference for a single mass solve. It does not yet cover uncertain constraints, nonlinear trajectories, contact/friction, probabilistic parameter distributions, sparse articulated dynamics, or formally certified floating-point bounds. The Frobenius estimate may be conservative; its conservatism must be measured against spectral and high-precision references.
