# Independent 5D equal-spin EGB radial-action derivation

**Status:** reduced action transcribed from the primary source; five Euler–Lagrange equations symbolically derived; angular Noether identity checked. The committed research script has not yet been replayed from a clean checkout, and no Rust equation solver is claimed.

**Primary source:** Brihaye, Kleihaus, Kunz and Radu, *Rotating black holes with equal-magnitude angular momenta in d=5 Einstein–Gauss–Bonnet theory*, JHEP 11 (2010) 089, arXiv:1010.0860, Eqs. (2.8)–(2.12).

## 1. Keep the full metric and the unfixed radial gauge

The source's coordinate ansatz is
\[
\begin{aligned}
ds^2={}&-b(r)dt^2+\frac{dr^2}{f(r)}+g(r)d\theta^2\\
&+h(r)\sin^2\theta(d\phi_1-w(r)dt)^2
+h(r)\cos^2\theta(d\phi_2-w(r)dt)^2\\
&+(g(r)-h(r))\sin^2\theta\cos^2\theta
(d\phi_1-d\phi_2)^2.
\end{aligned}
\]
The (g-h) angular cross-term is essential. The equivalent Hopf-frame metric is
\[
ds^2=-b\,dt^2+\frac{dr^2}{f}
+\frac g4(\sigma_1^2+\sigma_2^2)
+\frac h4(\sigma_3+2w\,dt)^2,
\]
with
\[
\begin{aligned}
\sigma_1&=\cos\psi\,d\bar\theta+\sin\psi\sin\bar\theta\,d\phi,\\
\sigma_2&=-\sin\psi\,d\bar\theta+\cos\psi\sin\bar\theta\,d\phi,\\
\sigma_3&=d\psi+\cos\bar\theta\,d\phi,
\end{aligned}
\qquad
\bar\theta=2\theta,\quad \phi=\phi_1-\phi_2,\quad
\psi=\phi_1+\phi_2.
\]

The action is
\[
I=\frac1{16\pi G}\int d^5x\sqrt{-g}
\left(R+\frac\alpha4 L_{GB}\right).
\]
After the angular integration and removal of irrelevant overall constants, the source obtains
\[
A_{\mathrm{eff}}=\int dr\,dt\,L_{\mathrm{eff}},
\qquad
L_{\mathrm{eff}}=L_E+\frac\alpha4 L_{GB}.
\]

## 2. The published radial Lagrangian

Define the common factor
\[
S=\sqrt{\frac{fh}{b}}.
\]
The Einstein–Hilbert part is
\[
\boxed{
L_E=S\left[
b'g'
+\frac{g}{2h}b'h'
+\frac{b}{2g}(g')^2
+\frac{b}{h}g'h'
+\frac12 gh(w')^2
+\frac{2b}{f}\left(4-\frac hg\right)
\right].
}
\]

The Gauss–Bonnet part is
\[
\boxed{
L_{GB}=\frac Sg\left[
\frac{4h}{g}b'g'
+2(4g-3h)\left(\frac{b'h'}h+h(w')^2\right)
-\frac{f}{2h}b'h'(g')^2
-\frac12fh(g')^2(w')^2
\right].
}
\]

The effective Lagrangian is first order in radial derivatives. Importantly, f' does not occur: f is a nondynamical/lapse-like radial variable and its Euler–Lagrange equation is an algebraic constraint in this representation.

## 3. Vary first; impose the gauge second

Use independent radial functions b,f,g,h,w and define
\[
E_q=\frac{\partial L_{\mathrm{eff}}}{\partial q}
-\frac{d}{dr}\left(
\frac{\partial L_{\mathrm{eff}}}{\partial q'}
\right),
\quad q\in\{b,f,g,h,w\}.
\]
Because f' and w are absent from L_eff,
\[
E_f=\frac{\partial L_{\mathrm{eff}}}{\partial f},
\qquad
E_w=-\frac d{dr}\left(\frac{\partial L_{\mathrm{eff}}}{\partial w'}\right).
\]

The research script derives all five equations before making the gauge substitution:
\[
g=r^2,\qquad g'=2r,\qquad g''=2.
\]
It retains the gauge-sliced E_g expression as a radial constraint/consistency equation. The remaining equations E_b,E_f,E_h,E_w are not silently substituted for the full variational system.

The resulting equations are at most second order in radial derivatives, as expected for Lovelock gravity. In the symbolic replay, the second-derivative inventory is:
- E_b: g'',h'';
- E_f: no second derivatives;
- E_g: b'',g'',h'',w'';
- E_h: b'',g'';
- E_w: g'',w''.

The gauge-fixing order is an explicit part of the benchmark contract. A future solver must not erase the radial constraint by varying only the gauge-substituted action and then claiming it has recovered all five equations.

## 4. Angular Noether current emerges exactly from the action

The conjugate momentum to the cyclic variable w is
\[
\begin{aligned}
p_w
&=\frac{\partial L_{\mathrm{eff}}}{\partial w'}\\
&=w'h\sqrt{\frac{fh}{b}}\left[
g-\frac\alpha4\left(
\frac{f(g')^2}{g}-16+\frac{12h}{g}
\right)\right].
\end{aligned}
\]
This is exactly the published Eq. (2.12). Since w does not occur undifferentiated,
\[
\boxed{E_w=-p_w'=0,\qquad p_w=P_w=\mathrm{constant}.}
\]

After the gauge g=r²,
\[
\boxed{
P_w=w'h\sqrt{\frac{fh}{b}}\left[
r^2-\alpha\left(f-4+\frac{3h}{r^2}\right)\right].
}
\]
The symbolic fixture asserts both identities:
\[
\frac{\partial L_{\mathrm{eff}}}{\partial w'}=P_w,
\qquad
E_w+\frac{dP_w}{dr}=0.
\]
This independently links the action-level ODE to the angular flux used in the five-dimensional reference crate.

## 5. Why this is an important implementation boundary

The radial action is a much smaller object than a generic 5D tensor calculus implementation, but it is not merely a reduced ansatz pasted into a solver. It preserves:
- the complete equal-spin angular geometry;
- the radial constraint from varying f;
- the metric equation associated with g before choosing g=r²;
- the exact angular Noether identity;
- second-order Lovelock radial equations.

This gives us a tractable independent equation oracle for the next stage: derive a second Rust implementation of the gauge-sliced E_b,E_f,E_g,E_h,E_w residuals and compare them against the full 5D tensor equations once that verifier exists. Candidate discovery can then use the angular flux, while the action-derived residual remains independent of any PINN or spectral candidate generator.

## 6. Reproducible artifacts

- derive_reduced_egb_system.py — research-only SymPy derivation of L_E,L_GB, five unfixed Euler–Lagrange equations, post-variation gauge substitution, canonical momenta and identity checks.
- reduced_egb_action_fixture_v1.json — equation hashes, expression sizes, derivative inventory and exact Noether identity results.
- noether_flux_reduction_v1.md — the conditional angular-flux reconstruction and its interval abstention gate.
- akribiea_5d_mp_exact_reference.py and akribiea_5d_mp_exact_fixture_v1.json — the independent exact Myers–Perry and source-balance controls.

## 7. Claim ceiling

This establishes a reproducible symbolic formulation of the 5D equal-spin EGB cohomogeneity-one radial action. It does not solve the nonzero-alpha rotating boundary-value problem, establish a rotating branch, give full-domain interval enclosures, or prove existence, uniqueness, stability or astrophysical viability.


## 8. Exact-solution regression catches the action transcription

The first draft of the independent transcription used (b,g'h'/g) for the fourth Einstein–Hilbert term. That version preserved the (w)-canonical momentum but failed the finite-alpha Boulware–Deser equations and the alpha-zero flat/Schwarzschild–Tangherlini angular equations. This is an important example of why checking the Noether identity alone is not enough.

The source's Eq. (2.11) has
[
oxed{rac{b}{h}g'h'}
]
for this term. With the denominator corrected to (h), exact symbolic substitution now gives
[
E_b=E_f=E_g=E_h=E_w=0
]
for both:
1. the five-dimensional static Boulware–Deser Einstein branch at finite (alpha), with (b=f=F, g=h=r^2, w=0);
2. the five-dimensional equal-spin Myers–Perry solution at (alpha=0), with all five radial functions in the (g=r^2) gauge.

The corrected expression also preserves the exact identity (dL_{m eff}/dw'=P_w) and (E_w=-P_w'). The corrected equation digests and expression sizes are pinned in reduced_egb_action_fixture_v1.json.

The symbolic equations were executed independently in the development session with SymPy 1.14.0. The committed script includes both exact-solution substitutions as hard assertions, but has not yet been replayed from a clean checkout.
