# Fourth/fifth-order Taylor jets for Bach-tensor qualification

**Status:** implementation design and acceptance contract; no Taylor-jet or Bach evaluator is implemented by this document.  
**Work items:** [BH-METRIC-JET-001 #5965](https://github.com/Luminous-Dynamics/symtropy/issues/5965), [BH-BACH-VERIFY-001 #5937](https://github.com/Luminous-Dynamics/symtropy/issues/5937).  
**Curvature substrate:** [black-hole physics kernel PR #4053](https://github.com/Luminous-Dynamics/symtropy/pull/4053).  
**Static independent oracle:** [theory triad PR #5814](https://github.com/Luminous-Dynamics/symtropy/pull/5814).

## 1. Why the derivative order must be explicit

The current relativity substrate has explicit first/second metric derivatives and second-order forward AD. That supports the Riemann tensor, but not a general Bach tensor.

For four-dimensional pure Weyl-squared gravity, Mantica and Molinari define the Bach tensor (with their stated conventions) by

\[
\mathscr B_{kl}=2\nabla^j\nabla^m C_{jklm}+R^{jm}C_{jklm}.
\]

The Weyl tensor already contains second derivatives of the metric. The double covariant divergence therefore requires fourth derivatives of the metric. A numerical check of the contracted identity \(\nabla^k\mathscr B_{kl}=0\) additionally needs fifth-order metric derivatives.

**Consequence:** reusing second-order AD without extending the derivative substrate is not enough. Finite-differencing a computed Bach tensor is not an acceptable silent substitute.

## 2. Taylor-jet representation

Use a scalar Taylor jet over four coordinates, truncated at total degree five:

\[
J(x+\delta x)=\sum_{|\alpha|\leq 5} c_\alpha (\delta x)^\alpha,
\qquad
\alpha=(\alpha_0,\alpha_1,\alpha_2,\alpha_3).
\]

There are \(\binom{4+5}{5}=126\) multi-indices. Coefficients are normalized Taylor coefficients, so

\[
c_\alpha=\frac{\partial^\alpha J(x)}{\alpha_0!\alpha_1!\alpha_2!\alpha_3!}.
\]

This convention makes the core operations simple:

**Product (convolution)**

\[
(ab)_\alpha=\sum_{\beta\leq\alpha}a_\beta b_{\alpha-\beta}.
\]

**Coordinate derivative**

\[
(\partial_i a)_\alpha=(\alpha_i+1)a_{\alpha+e_i}.
\]

**Reciprocal**

For \(b=a^{-1}\), order coefficients by total degree:

\[
b_0=a_0^{-1},\qquad
b_\alpha=-a_0^{-1}
\sum_{0<\beta\leq\alpha}a_\beta b_{\alpha-\beta}.
\]

Reject zero/non-finite base values and any non-finite intermediate. Never silently promote absent coefficients to an assertion that the underlying function has zero higher derivatives: a jet has an explicit truncation order and every derivative request must state whether it is supported.

Minimum scalar operations: constant/variable construction, add/subtract, multiply, scale, reciprocal/divide, coordinate partials, explicit truncation, value/derivative extraction, and analytic sin/cos composition for spherical and Kerr charts. Additional functions should be added only with derivative tests.

## 3. Required order flow

The evaluator should preserve the following maximum total Taylor degree at every stage:

| Object | Maximum degree retained | Reason |
|---|---:|---|
| Metric g_ab, inverse g^ab | 5 | Input and differentiated inverse |
| Levi-Civita connection Gamma^a_bc | 4 | One metric derivative |
| Riemann, Ricci, scalar curvature, Weyl | 3 | One derivative of the connection |
| First contracted Weyl divergence D_jkl = nabla^m C_jklm | 2 | One derivative of Weyl |
| Bach tensor B_kl | 1 | One further derivative |
| Contracted Bach divergence nabla^k B_kl | 0 | One derivative of Bach |

The implementation must encode these reductions explicitly. A lower-order intermediate must not be differentiated as though the missing Taylor coefficients were available.

## 4. Tensor evaluation contract

Pin all of the following in a machine-readable convention descriptor before returning a qualified result:

- metric signature and coordinate ordering;
- chart identity and units;
- Riemann sign and index order;
- Ricci contraction and Weyl decomposition;
- Bach definition, derivative ordering and index placement;
- action normalization / variation convention;
- coordinate-point identity and metric-jet order.

For the current PR #4053 convention:

\[
R^\rho{}_{\sigma\mu\nu}
=\partial_\mu\Gamma^\rho{}_{\nu\sigma}
-\partial_\nu\Gamma^\rho{}_{\mu\sigma}
+\Gamma^\rho{}_{\mu\lambda}\Gamma^\lambda{}_{\nu\sigma}
-\Gamma^\rho{}_{\nu\lambda}\Gamma^\lambda{}_{\mu\sigma},
\qquad
R_{\sigma\nu}=R^\rho{}_{\sigma\rho\nu}.
\]

A candidate implementation can express the all-lower Weyl tensor as

\[
C_{abcd}=R_{abcd}
-\frac12(g_{ac}R_{bd}-g_{ad}R_{bc}-g_{bc}R_{ad}+g_{bd}R_{ac})
+\frac{R}{6}(g_{ac}g_{bd}-g_{ad}g_{bc}).
\]

Then form

\[
D_{jkl}=g^{mn}\nabla_n C_{jklm},
\qquad
\mathscr B_{kl}=2g^{jn}\nabla_nD_{jkl}
+g^{ja}g^{mb}R_{ab}C_{jklm}.
\]

These expressions are an implementation route, not permission to skip an independent symbolic-variation check. Review the Riemann sign and the order of the two covariant derivatives against the cited convention before freezing test values.

## 5. Independent static cross-check against the published reduction

The static branch in PR #5814 is the external oracle, not a dependency of the general tensor evaluator.

For

\[
ds^2=-B(r)dt^2+B(r)^{-1}dr^2+r^2d\Omega^2,
\]

Mantica and Molinari give

\[
\begin{aligned}
B_1+B_2&=-\frac{2B}{r}X-\frac{2B}{3}X',\\
B_1-B_2&=-\frac83AE-\frac43X\left(B'+\frac Br\right)-\frac{4B}{3}X',
\end{aligned}
\]

where

\[
A=\frac{1-B}{r^2}+\frac{B''}{2},\quad
E=\frac12\left(\frac{1-B}{r^2}+\frac{B'}r-\frac{B''}{2}\right),\quad
X=A'+\frac Ar.
\]

For a generic static point with B>0 and nonzero static acceleration, the full tensor decomposition in the cited paper implies

\[
B_1+B_2=\frac{\mathscr B_{tt}}{B}+B\mathscr B_{rr},
\qquad
B_1-B_2=\frac{4\mathscr B_{\theta\theta}}{r^2}.
\]

This gives a component-level cross-check, not merely agreement that both implementations vanish on shell. The extraction is not to be used where the static frame or acceleration direction degenerates; use a regular chart or a separate analytic fixture there.

The four-term ansatz

\[
B(r)=\frac{c_{-1}}r+c_0+c_1r+c_2r^2
\]

must recover

\[
B_1+B_2=0,\qquad
B_1-B_2=\frac{4(c_0^2-3c_1c_{-1}-1)}{3r^4}.
\]

Test both the Mannheim–Kazanas identity and deliberately off-shell coefficients. Include multiple radii, parameter combinations, held-out points, and the predicted inverse-fourth-power scaling. The quadratic coefficient c_2 must remain free in the constraint.

## 6. Qualification matrix

| Family | Positive control | Negative / adversarial control |
|---|---|---|
| Flat geometry | Cartesian Minkowski: Riemann, Weyl, Bach vanish | Perturb one metric-jet coefficient; detect nonzero curvature |
| Conformal geometry | Conformally flat metric: Weyl and Bach vanish | Nonconstant conformal factor with independently derived expected transformation weight |
| Einstein geometry | Schwarzschild and Kerr: Bach vanishes in regular chart domains | Deliberately perturb metric away from the Einstein family |
| Static Weyl | Mannheim–Kazanas component-level agreement with reduced B1+B2 and B1-B2 | Off-shell coefficient perturbation and r^-4 scaling |
| Coordinates | Same tensor transformation / invariant values across charts | Deliberately mismatched chart or coordinate ordering must fail closed |
| Differential identities | Bach symmetry, trace and contracted divergence | Index-order, sign, signature and derivative-order mutations |
| Conditioning | Report inverse-metric norm and pivot/conditioning diagnostics | Near-horizon, polar-axis and ill-conditioned chart points must not be silently accepted |

For conformal tests, derive and pin the precise expected weight for the chosen index placement; do not casually label every component representation “invariant”. In four dimensions the Bach equation is conformally covariant, but the component weight depends on index placement.

The static orthonormal-frame decomposition is valid only in its stated static domain. It must not be extrapolated across a Killing horizon simply because an algebraic expression remains finite. Use ingoing/horizon-regular coordinates for tensor checks across horizons.

## 7. Independent implementation requirement

The general evaluator must not call the static symmetry-reduced formula in PR #5814. Independence is structural:

1. General route: metric jets → connection → Riemann → Weyl → covariant derivatives → Bach.
2. Static route: published symmetry-reduced scalar expressions in PR #5814.
3. Symbolic route: independently vary the Weyl-squared action, or use a separate tensor-algebra implementation, to produce at least one fixture not generated by the general evaluator.

Compare components after aligning conventions. A zero residual alone is not enough; a sign/normalization error can preserve a zero set. Publish component-level discrepancies, trace, symmetry, divergence, condition estimate, derivative-order coverage, precision, and exact source digests.

## 8. Benchmark protocol

Report a Pareto vector rather than one opaque score:

- maximum absolute and normalized tensor residual;
- component-level agreement with the reduced and symbolic references;
- trace, symmetry and contracted-divergence residuals;
- convergence under increased precision / independent arithmetic;
- condition estimate and rejected-domain count;
- jet operations / arithmetic count, wall time, peak memory and energy where measurable;
- exact compiler, command, source revision and run receipt.

Initial acceptance is correctness-led. Do not set a universal f64 tolerance for singular or badly conditioned charts. Calibrate tolerances against the declared arithmetic, local tensor scale and independent reference error.

A “better than SOTA” claim requires a matched equation, background family, domain, boundary conditions, numerical precision, hardware, accuracy target and full compute ledger. Akribeia's demonstrated higher-dimensional Lovelock sector is a discovery-method comparator, not a direct 4D pure-Weyl Bach benchmark.

## 9. Claim ceiling

This document is a design contract only. It does not implement Taylor jets or a Bach evaluator and has no compilation or numerical-test result. A successful future implementation would qualify the evaluator on its tested metrics/charts and controls; it would not by itself establish rotating Weyl black-hole existence, global regularity, stability, observational viability, or superiority over other gravity theories.

## References

- C. A. Mantica and L. G. Molinari, “The covariant approach to static spacetimes in Einstein and extended gravity theories”, *General Relativity and Gravitation* 55, 100 (2023), [DOI: 10.1007/s10714-023-03149-w](https://doi.org/10.1007/s10714-023-03149-w).
- Luminous Dynamics Symtropy [#5937](https://github.com/Luminous-Dynamics/symtropy/issues/5937), [#5965](https://github.com/Luminous-Dynamics/symtropy/issues/5965), [#3981](https://github.com/Luminous-Dynamics/symtropy/issues/3981), and [PR #5814](https://github.com/Luminous-Dynamics/symtropy/pull/5814).
