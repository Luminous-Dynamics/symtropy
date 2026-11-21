# Wald, Lovelock Noether charge and Section 4 density law: exact consistency audit

**Owner:** [#6545 BH-5D-EGB-NHEG-CHARGE-CONSISTENCY-001](https://github.com/Luminous-Dynamics/symtropy/issues/6545)

**Status:** the independent Einstein-calibrated Lovelock Noether route and the full local Wald/Jacobson–Myers density variation agree exactly. The density law in Section 4, Eqs. (24)–(25) of Çelik–Hajian–Kunz is different by an exact rational residual. A coordinate primitive of that residual is derived and verified. The origin of the residual remains unresolved.

> Equation numbering matters: **Section 4 Eqs. (24)–(25)** are the published density relation and polynomial. **Appendix Eqs. (31)–(33)** are the expanded \(k_{R^2}\), \(k_{\mathrm{Ricci}^2}\), and \(k_{\mathrm{Riemann}^2}\) tensors. The latter have not yet been replayed term-by-term in Symtropy.

Primary source: Çelik, Hajian & Kunz, arXiv:2605.15005v2, Sec. 4 and Appendix, https://arxiv.org/html/2605.15005v2.

## 1. Independent geometric inputs and conventions

The exact Cartan/Lanczos replay [#6390](https://github.com/Luminous-Dynamics/symtropy/pull/6390) verifies the source metric's local EGB field equations and Kretschmann invariant. The conformal horizon end [#6458](https://github.com/Luminous-Dynamics/symtropy/pull/6458) and local Wald density [#6464](https://github.com/Luminous-Dynamics/symtropy/pull/6464) have separate clean-checkout replays.

Use
\[
\alpha_{sym}=4\alpha_{NH},\quad
\beta=\frac{2a^2}{\alpha_{sym}},\quad
\epsilon=\cos\theta,\quad
T=\beta\epsilon^4,\quad 0<\beta<1.
\]
All parametric derivatives below are at fixed \(\alpha_{sym}\), so \(\beta\) varies with \(a\).

## 2. Local Wald/Jacobson–Myers variation

The induced horizon data give
\[
R[h]=\frac{4(T^2+10T-3)}
{\alpha_{sym}(1-T)^3},\qquad
\sqrt h=\frac{a\alpha_{sym}}{2\epsilon^3}(1-T).
\]
For the source action \(R+\alpha_{NH}\mathcal L_{GB}\), the local five-dimensional Lovelock entropy density is
\[
s_W=\frac{\sqrt h}{4G}
\left(1+2\alpha_{NH}R[h]\right)
=\frac{\sqrt h}{4G}
\left(1+\frac{\alpha_{sym}}2R[h]\right).
\]
The multiplier is
\[
1+\frac{\alpha_{sym}}2R[h]
=\frac{-5+17T+5T^2-T^3}{(1-T)^3}.
\]
Crucially, the parametric variation must differentiate the full density, including its explicit factor \(a\) and the \(a\)-dependence of \(T\). The exact result is
\[
\boxed{
\frac{1}{2\pi}\partial_a s_W
=\frac{\alpha_{sym}}{16\pi G\epsilon^3}
\frac{-5+36T+42T^2-12T^3+3T^4}{(1-T)^3}.
}
\]
Its normalized endpoint coefficient is \(-5\).

## 3. Coordinate Lovelock Noether route

For
\[
L=R+\alpha_{NH}L_{GB},
\]
the curvature momentum has an Einstein part and a Gauss–Bonnet part. With the pinned convention,
\[
P_{GB}^{abcd}
=2\left[
R^{abcd}-2\left(g^{a[c}R^{d]b}-g^{b[c}R^{d]a}\right)
+R\,g^{a[c}g^{d]b}
\right].
\]
The Lovelock Noether potential is normalized as
\[
Q^{ab}=-2E_R^{abcd}\nabla_c\xi_d,
\]
where \(E_R=\partial L/\partial R\) includes the action prefactor. The EH and GB momentum tensors carry different internal factors, so their antisymmetric contractions must be normalized separately. The EH normalization is fixed by the independently reproduced source \(k_E^{tr}\) in [#6469](https://github.com/Luminous-Dynamics/symtropy/pull/6469).

For \(\xi=-\partial_\psi\), the pullback of \(\xi\cdot\Theta\) to the constant-\(t,r\) three-surface vanishes by antisymmetry because \(\xi\) is tangent to the integration surface. The remaining surface-charge variation is obtained from the variation of the Noether potential three-form, including the volume-form variation.

The exact EH Noether variation is
\[
\mathcal D_E=
\frac{\alpha_{sym}}{16\pi G\epsilon^3}(1-3T),
\]
with endpoint coefficient \(+1\). The exact GB contribution is
\[
\mathcal D_{GB}
=\frac{\alpha_{sym}}{16\pi G\epsilon^3}
\frac{2(-T^3+15T^2+21T-3)}{(1-T)^3},
\]
with endpoint coefficient \(-6\). The sum is
\[
\boxed{
\mathcal D_{\rm axial}
=\frac{\alpha_{sym}}{16\pi G\epsilon^3}
\frac{-5+36T+42T^2-12T^3+3T^4}{(1-T)^3}.
}
\]
This equals the full local Wald density variation exactly:
\[
\boxed{\mathcal D_{\rm axial}=\frac{1}{2\pi}\partial_a s_W.}
\]
This is a pointwise identity in the parameterized local geometry, not just a leading-order coincidence.

## 4. Published Section 4 density law

Section 4, Eq. (24), states
\[
\frac{\delta\mathcal S}{2\pi}
=\delta\mathcal J_2
=2n_{\mathcal H}^{a}\delta\mathcal Q_a
=\frac{\sin\theta\,\mathcal P(\Delta_+,\Delta_-)}
{512\pi G\cos^3\theta\,\Delta_-^3\Delta_+^2}\delta a,
\]
with the polynomial in Section 4, Eq. (25):
\[
\begin{aligned}
\mathcal P={}&419\Delta_-^6+10730\Delta_-^5\Delta_+
-40043\Delta_-^4\Delta_+^2\\
&+46828\Delta_-^3\Delta_+^3
-14627\Delta_-^2\Delta_+^4
-6262\Delta_-\Delta_+^5+3147\Delta_+^6.
\end{aligned}
\]
Using
\[
\Delta_+=\frac{\alpha_{sym}}2(1+T),\qquad
\Delta_-=-\frac{\alpha_{sym}}2(1-T),
\]
and changing to the \(\epsilon\)-coordinate measure, this becomes
\[
\boxed{
\mathcal D_{\rm Sec4}
=\frac{\alpha_{sym}}{16\pi G\epsilon^3}
\frac{1600-2112T-3536T^2+736T^3+156T^4+12T^5-3T^6}
{(1-T)^3(1+T)^2}.
}
\]
Its normalized endpoint coefficient is \(+1600\).

The exact residual from the compact Lovelock/Wald route is
\[
\boxed{
\mathcal D_{\rm Sec4}-\mathcal D_{\rm axial}
=\frac{\alpha_{sym}}{16\pi G\epsilon^3}
\frac{1605-2138T-3645T^2+628T^3+135T^4+18T^5-6T^6}
{(1-T)^3(1+T)^2}.
}
\]
The normalized endpoint coefficient of this residual is \(+1605\). The clean-checkout replay checks the exact rational identity, not only the leading coefficient.

## 5. Exact coordinate primitive of the residual

Let
\[
z=\sqrt\beta\,\epsilon^2.
\]
The residual above is an exact derivative in the chosen coordinate:
\[
\mathcal D_{\rm Sec4}-\mathcal D_{\rm axial}
=\frac{dB}{d\epsilon},
\]
where
\[
B(\epsilon)=\frac{\alpha_{sym}\sqrt\beta}{64\pi G}
\left[I(\sqrt\beta\,\epsilon^2)-I(\sqrt\beta)\right]
\]
and
\[
\begin{aligned}
I(z)={}&12z+
\frac{-40889z^6+42878z^4+35759z^2-51360}
{16z(1-z^2)^2(1+z^2)}\\
&+\frac{19813}{32}\log\frac{1+z}{1-z}
-\frac{5365}{4}\arctan z.
\end{aligned}
\]
The primitive vanishes at the regular axis \(\epsilon=1\). Near the conformal end,
\[
I(z)=-\frac{3210}{z}-1066z-\frac{1936}{3}z^3
-\frac{9232}{5}z^5-\frac{13912}{7}z^7+O(z^9),
\]
so
\[
B(\epsilon)\sim-\frac{1605\alpha_{sym}}
{32\pi G\epsilon^2}.
\]

This is a coordinate primitive of the scalar density residual. **It is not yet proven to be the covariant phase-space boundary current or the symplectic flux.** The next mathematical step is to derive the relevant \((D-2)\)-form/Noether-current decomposition and compare its conformal-end pullback with \(dB/d\epsilon\).

## 6. Required independent reconciliation

The current branch does **not** yet implement Appendix Eqs. (31)–(33) term-by-term. That is the next decisive test:
1. Transcribe the source's explicit \(k_{R^2}\), \(k_{\mathrm{Ricci}^2}\), and \(k_{\mathrm{Riemann}^2}\) with the paper's exact index and Riemann conventions.
2. Evaluate their antisymmetrized \(tr\) component for \(h_{\mu\nu}=\partial_a g_{\mu\nu}\) at fixed \(\alpha_{sym}\), and \(\xi=-\partial_\psi\).
3. Compare that result against the compact Lovelock Noether potential and the published Section 4 density.
4. Independently compute the three \(SL(2,\mathbb R)\) charge-density components \(n_{\mathcal H}^{a}\delta\mathcal Q_a\) for a fixed bifurcation point.
5. Compute the presymplectic current and flux at \(\epsilon\to0\). Determine whether the exact coordinate primitive \(B\) is generated by a covariant boundary term, or whether the local-to-integrated relation requires a correction.
6. Test the result under fixed geodesic defining-function gauge, conformal representative changes, and alternative regular cutoff coordinates.

No source-paper error is claimed until those checks are complete.

## 7. Reproduction and claim ceiling

Run:
~~~sh
python3 derive_nheg_charge_consistency.py --output nheg_charge_consistency.json
~~~
The clean-checkout workflow is pinned to Python 3.13 and SymPy 1.14.0. It successfully executes exact coordinate/Lovelock contractions, checks the EH calibration and GB Noether contribution, verifies exact equality between Wald variation and axial Lovelock variation, verifies the Section 4 source residual and the angular primitive, and preserves a 95-point, 60-digit reference receipt.

**Claim ceiling:** the local Wald/axial identity and exact discrepancy with the source Section 4 density law are reproduced. The Appendix \(k_{GB}\) term-by-term replay, \(SL(2,\mathbb R)\) density decomposition, symplectic flux, integrability and any finite renormalized charges remain unresolved.
