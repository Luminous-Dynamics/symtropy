# Nonlinear constraint-chart reference v0.1

This standalone reference starts [PHYS-NONLINEAR-CONSTRAINT-CHART-001 #5127](https://github.com/Luminous-Dynamics/symtropy/issues/5127), extending the constant constraint-row basis work in #4876.

## The distinction this tests

Let g(q)=0 define a regular constraint manifold and let h(q)=phi(g(q)) use another smooth, locally invertible defining function with the same zero set.

At a fixed state, write T=Dphi(g). Then:

- h = phi(g): residual values transform nonlinearly;
- J_h = T J_g: Jacobians transform by the derivative;
- lambda_h = T^-T lambda_g: multipliers transform as covectors to preserve generalized force;
- W_h = T W_g T^T: the Delassus operator transforms by congruence;
- h_ddot = T g_ddot + D²phi(g)[g_dot,g_dot]: acceleration constraints have a curvature term.

Do not assert lambda_h^T h = lambda_g^T g for nonlinear phi. The invariant object is the virtual-work pairing with differentials, not a finite residual-value pairing.

## Fixture

The reference uses the unit-circle constraint

    g(q) = 0.5 * (qᵀ q - 1)

and monotone nonlinear chart

    phi_beta(g) = g + beta * g³, beta >= 0.

This chart has one real zero (g=0) and derivative 1 + 3 beta g² > 0.

## What the tests cover

- same physical zero set under nonlinear reparameterization;
- Jacobian, multiplier, force and Delassus transformation laws;
- full second-derivative chain rule including D²phi[g_dot,g_dot];
- an explicit negative control for the naive linear residual-pairing assumption;
- one-step Gauss-Newton correction dependence on the chosen defining function;
- convergence of both chart choices to the same metric-nearest projection on the circle;
- covariance of the correction under a dense generalized-coordinate re-basing.

## Reproduction

    cargo fmt --manifest-path research/nonlinear-constraint-chart-001/Cargo.toml --all -- --check
    cargo test --locked --manifest-path research/nonlinear-constraint-chart-001/Cargo.toml
    cargo clippy --locked --manifest-path research/nonlinear-constraint-chart-001/Cargo.toml --all-targets -- -D warnings

## Scope

v0.1 is a small dense, smooth, scalar holonomic reference. It does not qualify multi-constraint nonlinear mixing, redundant constraints, nonsmooth contact, friction, singular chart transitions, sparse articulated solvers or production solver superiority. The nearest-point oracle is exact for the unit circle under the identity metric only.
