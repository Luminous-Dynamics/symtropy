# plasma-metriplectic-audit

Finite-dimensional audit of a supplied GENERIC / metriplectic operator contract.

## Run

    cargo fmt --manifest-path crates/tools/plasma-metriplectic-audit/Cargo.toml -- --check
    cargo test --locked --manifest-path crates/tools/plasma-metriplectic-audit/Cargo.toml
    cargo clippy --locked --manifest-path crates/tools/plasma-metriplectic-audit/Cargo.toml --all-targets -- -D warnings
    cargo run --locked --manifest-path crates/tools/plasma-metriplectic-audit/Cargo.toml -- benchmarks/plasma/metriplectic-audit-v0.json

## Algebraic contract

For one supplied state and its energy/entropy gradients:

    du/dt = L grad(E) + M grad(S)

The audit checks:
- L + Lᵀ = 0 (reversible operator skew-symmetry);
- M = Mᵀ and M positive semidefinite (dissipative operator);
- L grad(S) = 0;
- M grad(E) = 0;
- grad(E) · du/dt = 0;
- grad(S) · du/dt >= 0.

The minimum eigenvalue of the scale-normalized symmetric part of M is computed using a symmetric Jacobi iteration. Material asymmetry is reported and rejected; it is not silently repaired. A rank-deficient positive-semidefinite M is allowed.

## Important claim boundary

This verifies **finite-dimensional local algebraic identities only**. It does not prove:
- the Poisson Jacobi identity;
- skew-adjointness of a spatial differential operator including boundary terms;
- the physical validity of E or S;
- collision invariants or a plasma collision operator;
- exact discrete-time preservation;
- function-space or mesh-transfer guarantees;
- that a supplied operator came from trusted code or a real plasma.

The continuous-time GENERIC identities must be paired with the correct discrete time integrator and solver-level physical diagnostics. The separate Vlasov-Poisson-Landau discrete-gradient reference is not reproduced by this crate.

Collisionless Landau damping is not assigned a blanket monotone-entropy gate: phase mixing in the fine-grained collisionless kinetic system is reversible. Collisional Vlasov-Poisson-Landau requires its own pinned collision operator and discrete entropy law.
