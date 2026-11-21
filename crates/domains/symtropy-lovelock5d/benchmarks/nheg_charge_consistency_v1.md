# Wald, Lovelock Noether charge and source density: exact consistency audit

**Owner:** [#6545 BH-5D-EGB-NHEG-CHARGE-CONSISTENCY-001](https://github.com/Luminous-Dynamics/symtropy/issues/6545)

**Status:** a coordinate-basis Lovelock-momentum replay has isolated an exact three-way local density discrepancy. The clean-checkout run is pending. The source's expanded higher-curvature \(k\)-tensor has not yet been replayed term-by-term, and the interpretation is unresolved.

## 1. Why this test matters

The exact Cartan/Lanczos calculation, the noncompact conformal-end geometry and the local Wald density have independent passing replay gates:
- [#6390](https://github.com/Luminous-Dynamics/symtropy/pull/6390): source near-horizon metric satisfies all 25 local EGB field equations exactly;
- [#6458](https://github.com/Luminous-Dynamics/symtropy/pull/6458): the horizon section is asymptotically hyperbolic and its cutoff measure is exact;
- [#6464](https://github.com/Luminous-Dynamics/symtropy/pull/6464): the local Wald/Jacobson–Myers density and its cutoff primitive are exact;
- [#6467](https://github.com/Luminous-Dynamics/symtropy/pull/6467): source Eqs. (24)–(25) have been converted exactly to Symtropy variables;
- [#6469](https://github.com/Luminous-Dynamics/symtropy/pull/6469): the Einstein \(k_E^{tr}\) contribution has been independently derived.

The unresolved question is whether the source's *full* higher-curvature charge density agrees with an independent Lovelock Noether-charge construction and with the local Wald entropy variation.

## 2. Convention and local geometry

Set
\[
\alpha_{sym}=4\alpha_{NH},\quad
\beta=\frac{2a^2}{\alpha_{sym}},\quad
\epsilon=\cos\theta,\quad
T=\beta\epsilon^4,\quad 0<\beta<1.
\]
The induced horizon data are
\[
R[h]=\frac{4(T^2+10T-3)}{\alpha_{sym}(1-T)^3},
\qquad
\sqrt h=\frac{a\alpha_{sym}}{2\epsilon^3}(1-T).
\]
The exact coordinate Riemann calculation used by the replay script uses the same source metric and Riemann convention that reproduces the published Kretschmann scalar. The full Cartan/Lanczos field-equation replay is independently green.

## 3. Local Wald variation — not the Wald density itself

The five-dimensional EGB Wald/Jacobson–Myers density is
\[
s_W=\frac{\sqrt h}{4G}\left(1+\frac{\alpha_{sym}}2 R[h]\right),
\]
with multiplier
\[
1+\frac{\alpha_{sym}}2R[h]
=\frac{-5+17T+5T^2-T^3}{(1-T)^3}.
\]
Varying the *density* at fixed \(\alpha_{sym}\), so \(T\propto a^2\) varies, and dividing by \(2\pi\) gives
\[
\boxed{
\frac{1}{2\pi}\partial_a s_W
=\frac{\alpha_{sym}}{16\pi G\epsilon^3}
\frac{T^4-2T^3+76T^2+26T-5}{(1-T)^4}.
}
\]
Its normalized conformal-end coefficient is \(-5\). This is distinct from the unvaried local density and from any integrated finite-part prescription.

## 4. Lovelock momentum / axial Noether route

For
\[
L=R+\alpha_{NH}L_{GB},
\]
the curvature momentum is
\[
E_R^{abcd}=\frac1{16\pi G}\left[
\frac12(g^{ac}g^{bd}-g^{ad}g^{bc})
+2\alpha_{NH}P_{GB}^{abcd}
\right],
\]
\[
P_{GB}^{abcd}=R^{abcd}
-2(g^{a[c}R^{d]b}-g^{b[c}R^{d]a})
+R g^{a[c}g^{d]b}.
\]
The replay script derives the coordinate metric, inverse, connection, Riemann/Ricci tensors, scalar curvature and relevant \(P_{GB}^{tr\rho\sigma}\) components. It contracts these against \(\nabla_\rho\xi_\sigma\) for \(\xi=-\partial_\psi\).

For the axial generator, \(\xi\) is tangent to the constant-\(t,r\) three-surface. Therefore the pullback of \(\xi\cdot\Theta\), a three-form evaluated on three tangent vectors, vanishes by antisymmetry. The source-normalized Noether-charge density convention is calibrated against the independently reproduced Einstein \(k_E^{tr}\) term; this fixes the overall normalization before adding the GB sector.

The resulting exact axial charge-variation density has normalized polynomial
\[
\boxed{
\mathcal D_{\rm axial}=
\frac{\alpha_{sym}}{16\pi G\epsilon^3}
\frac{3T^4-11T^3+27T^2+15T-2}{(1-T)^3}.
}
\]
Its leading coefficient is \(-2\): \(+1\) from the Einstein sector and \(-3\) from the GB sector.

## 5. Source-published density — Eq. (24)–(25)

The source's density law, converted to the same epsilon measure, is
\[
\boxed{
\mathcal D_{\rm source}=
\frac{\alpha_{sym}}{16\pi G\epsilon^3}
\frac{Q(T)}{(1-T)^3(1+T)^2},
}
\]
\[
Q(T)=1600-2112T-3536T^2+736T^3
+156T^4+12T^5-3T^6.
\]
Its leading coefficient is \(+1600\).

The Einstein term agrees exactly with [#6469](https://github.com/Luminous-Dynamics/symtropy/pull/6469):
\[
\mathcal D_E=
\frac{\alpha_{sym}}{16\pi G\epsilon^3}(1-3T).
\]
The mismatch is isolated in the GB charge contribution: the compact Lovelock momentum route gives a leading coefficient \(-3\), while the source density requires \(+1599\).

## 6. What this does and does not prove

The exact local formulas are different. That is a consistency finding, not yet a diagnosis. Candidate explanations still to test:

1. A mismatch in the factor-of-two / Hodge-dual convention between the source's \(k^{tr}\) and the Noether potential.
2. A missing term in the source's expanded Eqs. (31)–(33), or an implementation error in their transcription.
3. A missing boundary contribution or a nontrivial local-to-integrated step in the NHEG entropy perturbation law.
4. A failure to account for the entropy generator \(\zeta_{\mathcal H}=n_{\mathcal H}^a\xi_a-k^i m_i\) and its \(SL(2,\mathbb R)\) charge-density contribution.
5. A subtlety in the noncompact conformal-end phase space or in the permitted variations.

The next gate is to implement the source's full \(k_{R^2}-4k_{Ricci^2}+k_{Riemann^2}\) formula term-by-term and compare it against \(\delta Q_\xi-\xi\cdot\Theta\). Then compute the entropy-generator charge and the conformal-end symplectic flux. Do not infer that the paper is wrong until these independent routes have been compared under identical conventions.

## 7. Reproduction

Run:
~~~sh
python3 derive_nheg_charge_consistency.py --output nheg_charge_consistency.json
~~~
The clean-checkout workflow uses Python 3.13 and SymPy 1.14.0, runs the exact coordinate contractions, and preserves a 95-point, 60-digit reference receipt.

**Claim ceiling:** an exact local discrepancy between independently reconstructed expressions. No finite entropy, renormalized angular momentum, global parent solution, or source-paper error is claimed.
