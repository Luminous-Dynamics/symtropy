# symtropy-lovelock5d

A research crate for five-dimensional, equal-angular-momentum Einstein–Gauss–Bonnet (EGB) black-hole reference profiles, action-derived cohomogeneity-one equations, and structure-aware verification controls.

This crate is intentionally separate from the 4D symtropy-relativity metric kernel. It is not a generic five-dimensional tensor-curvature package and it does not yet contain a production nonlinear rotating EGB boundary-value solver.

## Scientific scope

The initial target is the asymptotically flat, equal-spin, cohomogeneity-one branch in the convention of Brihaye, Kleihaus, Kunz and Radu, JHEP 11 (2010) 089, arXiv:1010.0860:

\[
I=\frac{1}{16\pi G}\int d^5x\sqrt{-g}
\left(R+\frac{\alpha}{4}\mathcal L_{GB}\right).
\]

The five radial functions are \(b,f,g,h,w\). The gauge \(g(r)=r^2\) is imposed only after varying the unfixed radial action. The \(E_g\) equation is retained as a radial constraint. The full angular cross-term and Hopf-frame normalization belong to the source ansatz; omitting them changes the theory being solved.

## Current contents

- Exact equal-spin five-dimensional Myers–Perry reference profiles and derivatives at \(\alpha=0\).
- Static Einstein-branch Boulware–Deser profiles and curvature controls at finite \(\alpha\).
- Published first-order slow-rotation reference around Boulware–Deser, including \(w,w',w''\), horizon angular velocity, asymptotic angular coefficient and exact angular Noether flux.
- A dependency-free second-order jet evaluator for the unfixed reduced action and all five Euler–Lagrange residuals.
- A binary64 outward-rounded angular-quadrature pre-screen that fails closed when the metric or denominator is unresolved. This is a fast screening layer, not a proof-grade interval solver.
- Exact MP and BD fixtures, the \(O(a^2)\) spin-response equation derivation, and solver-comparison protocols.
- An executed high-precision reference for the areal-to-isotropic map of the static BD exterior and its linear-spin transformation.

The finite-\(\alpha\) first-order angular mode obeys
\[
C=2r_H^2+\alpha,\quad
s=\sqrt{1+\frac{\alpha C}{r^4}},\quad
F=1-\frac{C}{r^2(1+s)},
\]
\[
w=\frac{aC}{r^4(1+s)}+O(a^3),\quad
P_w=-2aC=-4\mathcal W.
\]
For this exact static/linear-spin reference,
\[
K=r^2-\alpha(F-1)=r^2s\ge r^2
\quad(\alpha\ge0).
\]
This positivity bound is not a theorem for arbitrary nonlinear candidates.

The time-sector expression from the 2010 source is a **sourced radial balance** at finite coupling:
\[
P_t'=\frac{\alpha}{6}r^2\sqrt{\frac{bh}{f}}\mathcal L_{GB}.
\]
It is not a second conserved Noether flux. Do not replace it with a constant at finite \(\alpha\).

## Evidence and implementation status

| Area | Status | Claim ceiling |
|---|---|---|
| MP and BD reference profiles | Implemented; exact symbolic controls recorded in development | Reference implementation; Rust suite still needs execution |
| Five action-derived residuals | Implemented with second-order radial jets; exact MP/BD controls authored | Cohomogeneity-one action oracle, not a full 5D tensor evaluator |
| First-order finite-\(\alpha\) slow spin | Exact published reference implemented | \(O(a)\) solution only |
| Spin-squared equations | Symbolically extracted; exact \(\alpha=0\) MP coefficients pass in the development SymPy session | Finite-\(\alpha\) \(B_2,F_2,H_2\) BVP is not solved |
| Areal/isotropic BD map | 90-digit mpmath reference replayed for \(\lambda=0,0.7,3\) | Numerical coordinate-map consistency; not Arb-certified and not an isotropic field-equation solve |
| Angular interval pre-screen | Rust implementation and tests authored | Binary64 screening, not IEEE-1788/Arb certification |
| Nonlinear rotating EGB branch | Not implemented | No finite-spin nonzero-\(\alpha\) solution claimed |
| Full-domain residual / existence proof | Not implemented | No interval solution enclosure or radii-polynomial proof claimed |
| QNM/stability pipeline | Not implemented | No stability or ringdown result claimed |

**Rust execution boundary:** the current development environment used for this branch did not provide rustc, cargo or nix. The Rust tests are authored but not compiled or executed here. Do not read the presence of tests or green parent-commit CI as a pass for this branch.

## Build and test

From the repository root, once a Rust toolchain is available:

    cargo test -p symtropy-lovelock5d

The SymPy derivation scripts are research-only. The isotropic reference uses mpmath:

    python3 crates/domains/symtropy-lovelock5d/benchmarks/isotropic_gauge_reference.py \
      --dps 90 \
      --output crates/domains/symtropy-lovelock5d/benchmarks/isotropic_gauge_reference_v1.json

A successful research script is not a substitute for compiling the Rust implementation or qualifying a physical solution.

## Benchmark ladder

1. **L0 exact controls:** all five action residuals on equal-spin MP and static BD; first-integral sign and normalization; all negative controls rejected.
2. **L1 linear spin:** compare \(w,w'\), \(P_w\), \(K\) and the known \(O(a^3)\) MP remainder.
3. **L2 spin squared:** solve the finite-coupling linear BVP for \(B_2,F_2,H_2\), retain \(E_g\), verify horizon/infinity conditions and exact MP limit.
4. **L3 nonlinear rotating branch:** reproduce the published 5D equal-spin EGB branch with matched \((r_H,\alpha,\Omega_H)\); compare both continuation directions and both radial gauges.
5. **L4 physical qualification:** thermodynamic first law, independent full-domain residual bounds, and only then perturbation/QNM calculations.

The common scorecard includes held-out residuals, full-domain enclosures, angular flux drift, radial-constraint defect, asymptotic charges, horizon observables, first-law defect, gauge mismatch, spectral tails, conditioning, convergence failures, runtime, memory and energy per qualified solution.

See benchmarks/bh_solver_comparison_matrix_v1.md and benchmarks/isotropic_gauge_reference_v1.md for the evolving protocol and gauge derivation.
