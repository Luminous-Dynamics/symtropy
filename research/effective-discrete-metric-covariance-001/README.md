# Effective discrete metric covariance reference v0.1

This dependency-free Rust reference qualifies the dense-coordinate covariance of a two-coordinate implicit linear step with mass, damping and stiffness. It is the first executable slice of [PHYS-EFFECTIVE-DISCRETE-METRIC-001 #5189](https://github.com/Luminous-Dynamics/symtropy/issues/5189).

## Discrete equation

For `M a = f - D v_next - K q_next`, `v_next = v + h a`, and `q_next = q + h v_next`, eliminate acceleration and next position to obtain:

`M_hat v_next = M v - h K q + h f`,
`M_hat = M + h D + h^2 K`,
`q_next = q + h v_next`,
`a_step = (v_next - v) / h`.

The reference computes this operator and transition directly. It does not use a continuous-time acceleration surrogate.

## Coordinate covariance

For `q = S y`, tangent state maps by `S`, the force covector by `S^T`, and each matrix by congruence `S^T (.) S`. The test path directly solves the transformed effective system and then lifts the result. It never reconstructs canonical inputs and calls the original solver.

Integer unimodular 2x2 shears provide exact basis maps. The tests span 17 signed shear values and five physical timesteps with non-commuting M, D and K matrices.

## Discrete energy/work ledger

The solver emits a separate `DiscreteEnergyLedgerV1`. For the implicit update, with
`Δv = v_next - v` and `Δq = q_next - q = h v_next`, the exact algebraic balance is:

```
ΔT + ΔV
  + h * v_nextᵀ D v_next
  + 1/2 * Δvᵀ M Δv
  + 1/2 * Δqᵀ K Δq
  = h * fᵀ v_next
```

The ledger separates kinetic energy before/after, quadratic potential energy before/after, external force work, physical damping dissipation, the mass-weighted velocity-increment term, the stiffness-weighted position-increment term, and absolute/relative balance residuals.

The two increment terms are properties of this implicit discrete update; they must not be reported as physical damping. They explain why a backward/implicit step can be strongly stable while substantially altering amplitude and phase. A dedicated stiff-oscillator control uses a step whose scalar amplification radius is below one while its one-step position differs materially from the analytic continuous trajectory. This is an accuracy warning, not a claim that implicit integration is generally inaccurate.

## Metrics

The reference reports the effective-metric condition estimate separately from the mass-only metric, backward solve residual, the discrete energy/work ledger, and lifted physical state errors. Coordinate infinity-norm condition numbers are expected to vary with basis; physical lifted-state, metric-induced differences, and scalar energy/work terms are the invariance criteria.

## Claim boundary

v0.1 is a linear 2D oscillator method reference. It is not a full MuJoCo `discrete` integrator, a contact solver, flex/actuator model, SAP/CENIC implementation, sparse solver, or production benchmark. It does not establish SOTA superiority.

## Reproduction

    cargo fmt --manifest-path research/effective-discrete-metric-covariance-001/Cargo.toml --all -- --check
    cargo test --locked --manifest-path research/effective-discrete-metric-covariance-001/Cargo.toml
    cargo clippy --locked --manifest-path research/effective-discrete-metric-covariance-001/Cargo.toml --all-targets -- -D warnings

## Tests

- correct `M + h D + h^2 K` construction;
- exact dyadic closed-form step;
- discrete kinetic/potential/work/dissipation ledger identity;
- energy-ledger covariance under dense re-basing;
- unforced dissipative energy non-growth;
- stable-but-inaccurate large-timestep oscillator control;
- five timesteps × 17 dense non-orthogonal shears;
- force-covector variance negative control;
- missing timestep exponent negative control;
- indefinite stiffness and non-unimodular frame rejection;
- separately reported effective-metric conditioning;
- mass-only, damping-only and stiffness-only strata.
