# Pöschl–Teller QNM perturbation audit

This crate implements the analytic frequency-derivative known-answer controls
for the width perturbation in Lestingi, Sberna & Green, *Schrödinger
perturbation theory for black hole quasinormal modes*, arXiv:2607.19492.

## Frozen convention

The paper uses

    V(x, zeta) = sech²(zeta x),  zeta_0 = 1

and

    omega_n^±(zeta) = ±sqrt(1 - zeta²/4) - i zeta (n + 1/2).

For the plus branch at zeta=1:

    d omega_n / d zeta = -1/(2 sqrt(3)) - i(n + 1/2)
    d² omega_n / d zeta² = -2/(3 sqrt(3)).

The first derivative is independent of overtone in its real part and has a
linear overtone-dependent imaginary part. The exact second derivative is
independent of overtone for this analytic spectrum.

Connection to the general convention in issue #4240:
- V0 = 1;
- b = 1/zeta;
- lambda = V0 b² = 1/zeta².

This conversion is mandatory when comparing formulae across the two crates.

## Included

- Exact frequency, first derivative and second derivative for both branches.
- Principal complex continuation for zeta > 2.
- Explicit rejection of zeta=2 for derivative operations because the branch
  derivative is singular there.
- Finite-difference step-size sweeps for first and second derivatives.
- Published first-order comparisons for overtones n=0,1,2,5,10,15,30,40,50.
- Published second-order fundamental-mode comparison.

## Scope and claim boundary

This is P0 analytic frequency-derivative control for issue #4253 only. It does
not evaluate the bilinear form, regulated contour integrals, mode-shape shifts,
QNM projection sums, continuous-spectrum contributions, special functions,
Green functions or time-domain evolution.

In particular, it makes no claim that a QNM-only expansion reconstructs the
perturbed mode. The 2026 paper explicitly finds that its QNM partial sums for
the Pöschl–Teller mode shift diverge; the continuum contribution must be
considered separately.

Run:

    cargo fmt --manifest-path tools/bh-qnm-perturbation-audit/Cargo.toml -- --check
    cargo clippy --manifest-path tools/bh-qnm-perturbation-audit/Cargo.toml --all-targets -- -D warnings
    cargo test --manifest-path tools/bh-qnm-perturbation-audit/Cargo.toml --all-targets
