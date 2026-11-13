# Verified interval metric uncertainty envelope v0.1

Starts [PHYS-VERIFIED-INTERVAL-001 #5082](https://github.com/Luminous-Dynamics/symtropy/issues/5082).

## Arithmetic contract

v0.1 treats each finite `f64` input as its exact binary64 value and uses outward interval arithmetic. Each primitive operation widens the rounded result with `f64::next_down` / `next_up`, which are stable in Rust 1.86 and later. The CI toolchain is pinned to Rust 1.96.0 on Ubuntu 24.04 x86_64.

This is a **conditional verified enclosure** for that arithmetic/runtime contract. It is not a proof for arbitrary hardware, compiler flags, non-IEEE targets or unreviewed build configurations.

## Bound

For `M a0 = Q`, perturbed `(M + ΔM) a = Q + ΔQ`, with `M = L Lᵀ`, define `H = L⁻¹ ΔM L⁻ᵀ`, `g = L⁻¹ ΔQ`, and `rho >= ||H||₂`. If `rho < 1`,

`||a - a0||_M <= (||g||₂ + rho ||a0||_M)/(1-rho)`.

An interval Cholesky encloses the exact factor `L`. Interval triangular solves enclose `H`, `g`, and `Lᵀa0`. Frobenius of interval max-absolute endpoints bounds the spectral norm. The final scalar operations are also outward-rounded. The result reports upper endpoints only.

## Fail-closed behavior

- Reject non-finite or non-symmetric exact input matrices.
- Return `INCONCLUSIVE_SPD` if any interval Cholesky pivot cannot be proved strictly positive.
- Return `OUTSIDE_PERTURBATION_ENVELOPE` when `rho_upper >= 1` or exceeds the profile.
- Return no finite bound if division, overflow, or interval arithmetic becomes non-finite.

## Reproduction

```sh
cargo fmt --manifest-path research/verified-interval-metric-envelope-001/Cargo.toml --all -- --check
cargo test --locked --manifest-path research/verified-interval-metric-envelope-001/Cargo.toml
cargo clippy --locked --manifest-path research/verified-interval-metric-envelope-001/Cargo.toml --all-targets -- -D warnings
```

## Limitations

v0.1 covers small dense SPD mass solves. It does not yet certify the Delassus eigensolver, uncertain nonlinear trajectories, contact/friction, sparse articulated dynamics, or arbitrary platforms. A successful enclosure is valid only under the exact declared platform and build assumptions.
