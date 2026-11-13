# Nonlinear constrained midpoint residual-to-state envelope v0.1

This dependency-free Rust reference begins [PHYS-NONLINEAR-MIDPOINT-ERROR-001 #4948](https://github.com/Luminous-Dynamics/symtropy/issues/4948).

## Scope

The fixture is a dimensionless one-degree-of-freedom holonomic midpoint KKT system. It is not the production dumbbell integrator. Its purpose is to make the nonlinear-residual-to-state theorem executable with a Jacobian, a known positive-branch root, and an analytically derived Jacobian-Lipschitz bound.

Unknowns are \(z=(q_1,v_1,\lambda)\), \(q_m=(q_0+q_1)/2\), and \(g(q_m)=q_m^2-1\). The residual is

\[
F(z)=
\begin{bmatrix}
2(q_1-q_0)-h(v_0+v_1)\\
m(v_1-v_0)-h f-2q_m\lambda\\
q_m^2-1
\end{bmatrix}.
\]

The KKT Jacobian is

\[
J(z)=
\begin{bmatrix}
2 & -h & 0\\
-\lambda & m & -2q_m\\
q_m & 0 & 0
\end{bmatrix}.
\]

For this polynomial fixture the Jacobian variation satisfies

\[
\|J(z)-J(w)\|_F^2
\leq \Delta\lambda^2+\frac54\Delta q_1^2
\leq \frac54\|z-w\|_2^2.
\]

Thus \(L=\sqrt{5/4}\) is an analytic Jacobian-Lipschitz upper bound in the declared dimensionless Euclidean product metric.

## Newton–Kantorovich estimate

At the approximate state \(\hat z\), compute \(J_0=J(\hat z)\). Let

- \(\eta=\|J_0^{-1}F(\hat z)\|_2\), the Newton correction norm;
- \(\beta\), an upper estimate for \(\|J_0^{-1}\|_2\);
- \(\gamma=\beta L\);
- \(h_K=\gamma\eta\).

When \(h_K<1/2\), the classical Kantorovich radius is

\[
\rho=\frac{1-\sqrt{1-2h_K}}{\gamma}.
\]

The fixture also enforces that the proposed ball remains inside the positive constraint branch \(q_m>0\), which separates the intended root from the other algebraic branch.

The implementation uses the inverse Frobenius norm as an upper estimate for the inverse spectral norm, with a small numerical inflation. That is conservative in exact arithmetic, but the floating-point implementation does **not** use outward-rounded interval arithmetic. Therefore v0.1 reports an admissible estimate, not a formally certified root enclosure.

## Fail-closed behavior

The estimator abstains when the KKT Jacobian is singular, the Kantorovich product exceeds its profile, the ball crosses the positive-branch boundary, or the radius exceeds the declared profile. A claimed Jacobian-Lipschitz bound below the analytically derived fixture value is rejected.

## Tests and benchmark ladder

The tests cover the analytic Jacobian against finite differences, the positive-branch root within the reported radius, understated Lipschitz claims, singular KKT admission, the opposite constraint branch, and a high-mass regime that must abstain.

The example sweeps mass ratios and time steps and prints residual, Newton correction, inverse-Jacobian estimate, Kantorovich product, radius, and typed status.

## Reproduction

    cargo fmt --manifest-path research/nonlinear-midpoint-error-001/Cargo.toml --all -- --check
    cargo test --locked --manifest-path research/nonlinear-midpoint-error-001/Cargo.toml
    cargo clippy --locked --manifest-path research/nonlinear-midpoint-error-001/Cargo.toml --all-targets -- -D warnings
    cargo run --release --locked --manifest-path research/nonlinear-midpoint-error-001/Cargo.toml --example kkt_condition_ladder
