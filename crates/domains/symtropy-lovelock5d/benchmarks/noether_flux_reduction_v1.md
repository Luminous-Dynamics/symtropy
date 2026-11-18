# Five-dimensional equal-spin EGB: Noether-flux reduction protocol v1

**Status:** exact first integral transcribed; Myers–Perry alpha=0 control independently symbolically checked; Rust reference code and tests authored but not compiled.
**Work owner:** [BH-5D-EGB-NOETHER-001 #6072](https://github.com/Luminous-Dynamics/symtropy/issues/6072).
**Matched SOTA owner:** [BH-AKRIBEIA-REPRO-001 #6063](https://github.com/Luminous-Dynamics/symtropy/issues/6063).

## 1. Source and theory conventions

Use the five-dimensional Einstein–Gauss–Bonnet action
\[
I=\frac{1}{16\pi G}\int d^5x\sqrt{-g}
\left(R+\frac{\alpha}{4}\mathcal L_{GB}\right),
\quad
\mathcal L_{GB}=R^2-4R_{\mu\nu}R^{\mu\nu}
+R_{\mu\nu\rho\sigma}R^{\mu\nu\rho\sigma}.
\]
The coupling convention is the Akribeia convention \(\alpha=4\alpha_2/\alpha_1\), with \(\alpha_1=(16\pi G)^{-1}\) and \(\alpha_0=0\).

Use the complete equal-spin cohomogeneity-one coordinate ansatz of Brihaye et al., Eq. (2.8):
\[
\begin{aligned}
ds^2={}&-b(r)dt^2+\frac{dr^2}{f(r)}+g(r)d\theta^2\\
&+h(r)\sin^2\theta(d\phi_1-w(r)dt)^2
+h(r)\cos^2\theta(d\phi_2-w(r)dt)^2\\
&+(g(r)-h(r))\sin^2\theta\cos^2\theta
(d\phi_1-d\phi_2)^2.
\end{aligned}
\]
The final angular cross-term is essential whenever \(g\ne h\). Omitting it changes the metric and invalidates a full tensor derivation.

The equivalent left-invariant Hopf-frame form is
\[
ds^2=-b\,dt^2+\frac{dr^2}{f}
+\frac g4(\sigma_1^2+\sigma_2^2)
+\frac h4(\sigma_3+2w\,dt)^2.
\]
The exact one-form normalization and orientation must be mapped explicitly before exchanging coefficients. Do not identify the forms by name alone.

The reduced radial action must be varied before imposing \(g=r^2\), following Brihaye et al. (2010), arXiv:1010.0860. The gauge-fixed flux expression below may only be applied after the variation and after the areal-base gauge is imposed.

## 2. Exact rotational Noether first integral

The reduced action is independent of the undifferentiated rotational function \(w(r)\). Its cyclic Euler–Lagrange equation has first integral
\[
P_w=w'h\sqrt{\frac{fh}{b}}
\left[
g-\frac{\alpha}{4}
\left(\frac{fg'^2}{g}-16+\frac{12h}{g}\right)
\right].
\]
After imposing \(g=r^2\), \(g'=2r\):
\[
\boxed{
P_w=w'h\sqrt{\frac{fh}{b}}
\left[r^2-\alpha\left(f-4+\frac{3h}{r^2}\right)\right]
=\text{constant}.
}
\]
Write
\[
K(r)=r^2-\alpha\left(f(r)-4+\frac{3h(r)}{r^2}\right).
\]
Then \(P_w=w'h\sqrt{fh/b}\,K\).

The asymptotic/horizon relation in Brihaye et al. gives
\[
\mathcal W=\frac14 h_H
\sqrt{\frac{f_1h_H}{b_1}}\,w_1
\left[-r_H^2+\alpha\left(\frac{3h_H}{r_H^2}-4\right)\right].
\]
With the same convention the Noether charge is
\[
P_w=-4\mathcal W,
\qquad
J_{\rm each}=\frac{2\pi^2\mathcal W}{8\pi G}.
\]
The sign depends on the Hopf-fiber orientation and the sign convention in \(d\phi_i-wdt\). Pin and test these together; do not independently flip one.

## 3. Field elimination as a conditional quadrature

Where all of \(b,f,h\) are positive and an independent enclosure proves \(K\) is separated from zero,
\[
w'(r)=\frac{P_w}{h\sqrt{fh/b}\,K(r)}
=\frac{-4\mathcal W}{h\sqrt{fh/b}\,K(r)}.
\]
Using \(w(r_H)=\Omega_H\), \(w(\infty)=0\),
\[
\Omega_H=4\mathcal W
\int_{r_H}^{\infty}
\frac{dr}{h\sqrt{fh/b}\,K(r)}.
\]
This is a candidate-generation reduction: solve for the remaining radial functions and the scalar \(\mathcal W\), then reconstruct \(w\) by quadrature. It is not an independent field equation and does not eliminate the need to check the remaining EGB equations.

### Mandatory abstention rule

Never divide by \(K(r)\) based on point samples or a floating-point threshold alone for a qualified run. Require an outward-rounded interval/ball enclosure over every subdomain with
\[
0\notin K(I_j).
\]
If any subdomain interval contains zero, return the state inconclusive_for_quadrature and revert to the undivided flux equation. Do not discard the subdomain or silently cross the zero. A zero may signal a branch/regularity condition that needs a separate local analysis.

The Rust helper currently uses a conservative floating-point near-zero guard only to prevent obvious division by zero in exploratory calculations. It is explicitly not a rigorous interval certificate.

## 4. Exact Myers–Perry positive control

At \(\alpha=0\), define
\[
q=r_H^2\Omega_H^2,\quad u=(r_H/r)^2,\quad D=1-q+qu^2.
\]
In the \(g=r^2\) gauge:
\[
b=1-u/D,\quad
f=(1-u)(1-q(1+u))/(1-q),\quad
h=r^2+r_H^2qu/(1-q),\quad
w=\Omega_Hu^2/D.
\]
The nonextremal domain is \(0\le q<1/2\).

The asymptotic rotational coefficient is
\[
\mathcal W=\frac{r_H^4\Omega_H}{1-q}.
\]
Direct substitution into the alpha=0 Noether charge gives
\[
P_w=-4\mathcal W
\]
at every exterior radius; the exact squared identity is \(P_w^2=16\mathcal W^2\). For \(\Omega_H>0\), \(w'<0\) and the remaining prefactors are positive, fixing the negative sign.

For the Akribeia reference \(r_H=1,\Omega_H=33/100\):
\[
q=1089/10000,\quad
\mathcal W=3300/8911,\quad
P_w=-13200/8911.
\]
The exact all-function fixture and rational sample at \(r=3\) are pinned in akribiea_5d_mp_exact_fixture_v1.json.

## 5. Separate Boulware–Deser control

The static EGB Einstein branch provides an independent control:
\[
w=0,\quad h=r^2,\quad
b=f=1+\frac{r^2}{\alpha}
\left(1-\sqrt{1+\frac{\alpha(\alpha+2r_H^2)}{r^4}}\right).
\]
Use its rationalized form to avoid cancellation as \(\alpha\to0\):
\[
b=f=1-\frac{\alpha+2r_H^2}
{r^2\left(1+\sqrt{1+\alpha(\alpha+2r_H^2)/r^4}\right)}.
\]
It has \(b(r_H)=0\) on the positive square-root branch and tends to the five-dimensional Schwarzschild–Tangherlini lapse \(1-r_H^2/r^2\). Do not use it as a rotating solution or as a substitute for the Myers–Perry control.

## 6. Three solver architectures to compare

A fair experiment compares identical equations, parameter values, domains, boundary conditions, precision and residual norms:

1. Five-field Akribeia-style: neural candidate for \(b,f,h,w\) (with \(g=r^2\)), followed by Chebyshev Gauss–Newton refinement.
2. Flux-reduced candidate: discover the remaining radial profiles and \(\mathcal W\), reconstruct \(w\) by quadrature only where \(K\) is certified nonzero.
3. Flux-reduced continuation: seed with exact Myers–Perry at \(\alpha=0\), then continue the EGB branch in \(\alpha\), tracking conditioning and branch turns.

Record success/failure rates, continuation coverage, collocation and held-out residuals, full-domain field residual enclosures, \(P_w(r)+4\mathcal W\) drift, \(K\)-enclosures, horizon/asymptotic errors, spectral tails, condition estimates, wall time, memory and energy. Separate candidate-generation cost from qualification cost.

The potential improvement is fewer independent unknown profiles and an exact conserved charge built into candidate generation. This is a falsifiable hypothesis, not a performance claim.

## 7. Frozen negative controls

- Change the EGB coupling normalization by four.
- Reverse the Hopf-fiber orientation without changing the sign of \(w\).
- Remove the alpha-dependent term from \(K(r)\).
- Mutate a candidate coefficient so the computed \(P_w(r)\) drifts with radius.
- Use a radial coordinate with \(g\ne r^2\) but retain the gauge-fixed flux formula.
- Let the interval enclosure of \(K\) contain zero and still perform quadrature.
- Use the wrong \(w_H\) sign.
- Confuse \(\mathcal W\) with the physical angular momentum without \(2\pi^2/(8\pi G)\).
- Treat alpha=0 Myers–Perry verification as evidence for a nonzero-alpha EGB branch.
- Use the 4D relativity tensor API as a 5D verifier.
- Substitute the BD static solution for rotating MP.
- Label the Akribeia approximately \(10^{-160}\) MP relative \(L^2\) solution error as an EGB field-equation residual.

Each mutation must fail or become inconclusive. Zero accepted mutations is the target.

## 8. Evidence levels and claim ceiling

Keep these distinct:
1. Algebraic/source-derived Noether identity.
2. Floating-point flux conservation on sampled candidate data.
3. Outward-rounded full-domain flux enclosure.
4. Full EGB field residual and boundary enclosure.
5. Approximate inverse and nonlinear operator bounds.
6. Lean-checked radii-polynomial local existence/uniqueness in a declared function-space ball.

A flux identity alone verifies only the cyclic equation. It does not prove the other EGB equations, existence of a rotating higher-curvature branch, horizon regularity in another chart, stability or observational viability.


## 9. Time-sector Eq. (2.14): sourced radial balance, not another conserved charge

A full PDF reinspection found that Eq. (2.14) contains alpha-dependent terms inside its current. Define
\[
C_b=f-4+\frac{h}{r^2}+\frac{rfh'}{h},\qquad
C_w=4-\frac{3h}{r^2}-f,
\]
\[
P_t(r)=\sqrt{\frac{fh}{b}}\left[
r^2(hww'-b')+
\alpha\left(C_b b'+C_w hww'+rfh(w')^2\right)
\right].
\]
The Einstein-only expression \( \sqrt{fh/b}\,r^2(hww'-b') \) is incomplete for nonzero alpha.

For the 5D tensor convention in Eq. (2.4), the Lanczos trace is
\[
H^\mu{}_\mu=-\frac12 L_{GB}.
\]
The trace of the vacuum EGB equations therefore gives
\[
R=-\frac{\alpha}{12}L_{GB}.
\]
Combining this with the full Eq. (2.14) gives the on-shell source balance
\[
\boxed{
P_t'(r)=\frac{\alpha}{6}r^2\sqrt{\frac{bh}{f}}L_{GB}.
}
\]
Its asymptotic endpoint is \(P_t(\infty)=2\mathcal U\). It is **not constant at finite alpha**. Only at alpha=0 does the Gauss-Bonnet source vanish and the current reduce to the Einstein constant \(P_t=2\mathcal U\).

For the static Boulware–Deser solution \(b=f=F,\ h=r^2,\ w=0\):
\[
P_t(r)=-r^3F'(r)+3\alpha r(F(r)-1)F'(r),
\]
\[
L_{GB}=\frac{12}{r^3}\left[(F-1)(rF''+2F')+r(F')^2\right],
\]
and this exact branch satisfies \(P_t'=\alpha r^3L_{GB}/6\). This is a decisive negative control against treating the Einstein-only bracket as a finite-alpha conserved charge.

The Rust reference crate now includes:
- the complete alpha-dependent time-sector current;
- a source-prefactor helper for \(P_t'\);
- pointwise \(b'\) reconstruction from the *local* current, with the full coefficient \(-r^2+\alpha C_b\);
- Boulware–Deser \(F,F',F''\) and an independently derived static 5D \(L_{GB}\) scalar;
- a finite-alpha Boulware–Deser source-balance test.

The proposed dual-conserved-flux reduction is withdrawn. The remaining solver architectures are the full four-profile system, the angular-Noether reduced system using the true conserved \(P_w\), and a source-balanced system using the complete Eq. (2.14) identity. The latter remains a benchmark hypothesis and must retain all remaining EGB field equations.
