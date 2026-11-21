# Exact mismatch between local Wald variation and the published NHEG charge-density law

**Owner:** [#6491 BH-5D-EGB-NHEG-WALD-CHARGE-CONSISTENCY-001](https://github.com/Luminous-Dynamics/symtropy/issues/6491)

**Status:** exact same-measure algebraic discrepancy established. Its covariant-phase-space interpretation is unresolved. This is a consistency result, not an attribution of fault.

## 1. Freeze the coupling and measure conventions

The source action is
\[
I=\frac1{16\pi G}\int\sqrt{-g}\,
(R+\alpha_{NH}\mathcal L_{GB}),
\]
while the Symtropy metric convention is
\[
\alpha_{sym}=4\alpha_{NH},\quad
\beta=\frac{2a^2}{\alpha_{sym}},\quad
\epsilon=\cos\theta,\quad T=\beta\epsilon^4.
\]
Set \(q=1-T\), \(p=1+T\). All comparisons below use the same \(d\epsilon\,d\phi\,d\psi\) measure and the same fixed-theory variation \(\delta\alpha_{sym}=0\).

The induced horizon density and scalar curvature from the exact 3D metric are
\[
\sqrt h=\frac{a\alpha_{sym}q}{2\epsilon^3},
\qquad
{}^{(3)}R[h]=\frac{4(T^2+10T-3)}{\alpha_{sym}q^3}.
\]

## 2. Differentiate the local Wald/Jacobson–Myers density correctly

For 5D EGB,
\[
s_W=\frac{\sqrt h}{4G}
\left(1+2\alpha_{NH}\,{}^{(3)}R[h]\right)
=\frac{\sqrt h}{4G}\left(1+\frac{\alpha_{sym}}2R[h]\right).
\]
The multiplier is
\[
1+\frac{\alpha_{sym}}2R[h]
=\frac{-5+17T+5T^2-T^3}{q^3}.
\]
Therefore
\[
\sqrt h(1+\alpha_{sym}R[h]/2)
=\frac{a\alpha_{sym}}{2\epsilon^3}
\underbrace{\frac{-5+17T+5T^2-T^3}{q^2}}_{f(T)}.
\]
At fixed \(\alpha_{sym}\), \(T=2a^2\epsilon^4/\alpha_{sym}\), so
\[
a\frac{\partial T}{\partial a}=2T.
\]
Differentiating the actual family, rather than holding \(\beta\) fixed, gives
\[
\frac{1}{2\pi}\frac{\partial s_W^{\rm integrated\ density}}{\partial a}
=\frac{\alpha_{sym}}{16\pi G\epsilon^3}
\left(f(T)+2Tf'(T)\right).
\]
After simplification,
\[
\boxed{
\mathcal D_{W,\epsilon}
=\frac{\alpha_{sym}}{16\pi G\epsilon^3}
\frac{3T^4-12T^3+42T^2+36T-5}{q^3}.
}
\]
When placed over the source's common denominator \(q^3p^2\), the Wald numerator is
\[
\boxed{P_W(T)=3T^6-6T^5+21T^4+108T^3+109T^2+26T-5.}
\]

## 3. Compare with the source's published charge density

The paper's Eqs. (24)–(25), converted to the same coupling and coordinate measure, give
\[
\mathcal D_{{\rm src},\epsilon}
=\frac{\alpha_{sym}}{16\pi G\epsilon^3}
\frac{Q(T)}{q^3p^2},
\]
\[
Q(T)=1600-2112T-3536T^2+736T^3
+156T^4+12T^5-3T^6.
\]
Subtracting the Wald variation produces the exact polynomial
\[
P_\Delta(T)=Q(T)-P_W(T)
\]
\[
\boxed{
P_\Delta(T)=1605-2138T-3645T^2+628T^3
+135T^4+18T^5-6T^6.
}
\]
Thus
\[
\boxed{
\mathcal D_{{\rm src},\epsilon}-\mathcal D_{W,\epsilon}
=\frac{\alpha_{sym}}{16\pi G\epsilon^3}
\frac{P_\Delta(T)}{q^3p^2}.
}
\]

At \(T\to0\), in the identical measure:
- published source density: \(+1600\,\alpha_{sym}/(16\pi G\epsilon^3)\);
- fixed-coupling local Wald variation: \(-5\,\alpha_{sym}/(16\pi G\epsilon^3)\);
- difference: \(+1605\,\alpha_{sym}/(16\pi G\epsilon^3)\);
- ratio of source to Wald leading coefficients: \(-320\).

The leading integrated difference is consequently
\[
\frac{1605\alpha_{sym}}{32\pi G\epsilon^2}\,\delta a+O(1).
\]
The discrepancy is not the factor-four action conversion, a sign choice for \(\Delta_-\), or a missing \(d\theta/d\epsilon\) Jacobian: those have been fixed in the exact conversion.

## 4. What is and is not established

**Established:** the local intrinsic metric gives the stated \(R[h]\); the standard Lovelock Wald/Jacobson–Myers integrand differentiates to \(P_W(T)\); the source's displayed Eq. (24)–(25) density converts to \(Q(T)\); these two local density representatives are not equal in the common measure.

**Unresolved:** whether the source charge tensor is a different presymplectic/solution-phase-space representative whose difference is a boundary term, whether a conformal-end flux contributes, whether the horizon generator or entropy normalization differs, or whether an algebraic error exists in either charge calculation.

The compact-surface argument cannot simply be imported. The source itself notes that the angular integration surface is noncompact and that standard NHEG integrated entropy/angular momentum diverge. A locally exact form can leave a nonzero flux at the conformal end. Conversely, one cannot invoke an unspecified boundary ambiguity to explain any mismatch: the candidate \(Y\)-ambiguity, its boundary conditions and symplectic flux must be derived explicitly.

## 5. Next derivation sequence

1. Rebuild \(k_{R^2}\), \(k_{Ricci^2}\), and \(k_{Riemann^2}\) directly from the screenshot-pinned Eqs. (31)–(33), retaining every index placement and derivative order.
2. Compare the resulting \(\sqrt{-g}\,k_{\zeta_H}^{tr}\) with the derivative of the local Wald charge on the same bifurcation surface.
3. Evaluate the difference as a differential form on the three-dimensional horizon section. Test whether it is \(dY\) for a local covariant \(Y\), and whether its integral is a pure conformal-end flux.
4. Compute the symplectic current and flux for the fixed-\(\alpha\) parameter variation and admissible perturbations preserving the NHEG symmetries.
5. Repeat on a regulated compact subregion and take the cutoff to zero; compare boundary terms and check the GR limit.
6. Only then decide whether the source density is a distinct charge prescription or one of the formulae requires correction.

## 6. Reproduction

Run:
~~~sh
python3 derive_nheg_wald_charge_consistency.py --output nheg_wald_charge_consistency_v1.json
~~~
The script checks the factor-four coupling map, the fixed-\(\alpha\) chain rule, both coordinate measures, the source polynomial, the Wald derivative, the exact difference polynomial, and the \(-320\) leading ratio. The fixture records 95 rational reference points.

**Claim ceiling:** the exact density mismatch is established; the covariant phase-space explanation is not. No finite entropy, finite angular momentum, or attribution of error is claimed.
