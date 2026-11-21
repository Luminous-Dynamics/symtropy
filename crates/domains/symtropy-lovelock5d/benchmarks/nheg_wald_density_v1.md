# Wald/Jacobson–Myers density and cutoff sensitivity at the singly rotating 5D EGB NHEG end

**Owner:** [#6461 BH-5D-EGB-NHEG-WALD-DENSITIES-001](https://github.com/Luminous-Dynamics/symtropy/issues/6461)

**Status:** exact local Wald/Jacobson–Myers density and regulated cutoff integral derived symbolically. Full rotating Iyer–Wald angular-charge variations and an integrable renormalized phase space remain open.

## 1. Exact coupling normalization

The source action is
\[
I=\frac1{16\pi G}\int d^5x\sqrt{-g}\,
(R+\alpha_{NH}\mathcal L_{GB}),
\]
while Symtropy's action is
\[
R+\frac{\alpha_{sym}}4\mathcal L_{GB}.
\]
Therefore \(\alpha_{sym}=4\alpha_{NH}\).

For a smooth compact cross-section of a 5D Lovelock Killing horizon, the local Wald/Jacobson–Myers integrand is
\[
s_W=\frac{\sqrt h}{4G}(1+2\alpha_{NH}\,{}^{(3)}R[h])
=\frac{\sqrt h}{4G}\left(1+\frac{\alpha_{sym}}2R[h]\right).
\]
This is a local formula. Extending its *integral* to the noncompact horizon section still requires a boundary/phase-space prescription.

## 2. Exact entropy multiplier

From the intrinsic horizon metric in [#6457](https://github.com/Luminous-Dynamics/symtropy/issues/6457), set
\[
\epsilon=\cos\theta,\qquad q=1-\beta\epsilon^4,\qquad
0<\beta<1.
\]
The intrinsic scalar is
\[
R[h]=\frac{4(\beta^2\epsilon^8+10\beta\epsilon^4-3)}
{\alpha_{sym}q^3}.
\]
Thus the exact Wald multiplier is
\[
\boxed{
1+\frac{\alpha_{sym}}2R[h]
=\frac{-5+17\beta\epsilon^4+5\beta^2\epsilon^8-\beta^3\epsilon^{12}}
{(1-\beta\epsilon^4)^3}.
}
\]
The area density in the \(\epsilon\) coordinate is
\[
\sqrt h=\frac{a\alpha_{sym}}{2\epsilon^3}(1-\beta\epsilon^4).
\]
The exact local Wald density therefore is
\[
\boxed{
4G\,s_W(\epsilon)=
\frac{a\alpha_{sym}}{2\epsilon^3}
\frac{-5+17\beta\epsilon^4+5\beta^2\epsilon^8-\beta^3\epsilon^{12}}
{(1-\beta\epsilon^4)^2}.
}
\]
At the conformal end the multiplier tends to \(-5\), and
\[
4G\,s_W(\epsilon)
=-\frac{5a\alpha_{sym}}{2\epsilon^3}
+\frac{7a\alpha_{sym}\beta}{2}\epsilon+O(\epsilon^5).
\]
This is a useful warning that higher-curvature entropy density is not simply a positive area density; it does not turn the divergent integral into a finite quantity.

## 3. Exact regulated Wald integral

The density has a closed-form primitive:
\[
F(\epsilon)=
\frac{\beta^2\epsilon^8+2\beta\epsilon^4+5}
{2\epsilon^2(1-\beta\epsilon^4)}.
\]
With angular periods \(\Delta\phi,\Delta\psi\), the integral between the regular axis \(\epsilon=1\) and cutoff \(\epsilon\) is
\[
\boxed{
S_W(\epsilon)=
\frac{\Delta\phi\Delta\psi\,a\alpha_{sym}}{8G}
\left[
\frac{\beta^2+2\beta+5}{2(1-\beta)}
-\frac{\beta^2\epsilon^8+2\beta\epsilon^4+5}
{2\epsilon^2(1-\beta\epsilon^4)}
\right].
}
\]
Hence
\[
\boxed{
S_W(\epsilon)\sim
-\frac{5\Delta\phi\Delta\psi\,a\alpha_{sym}}
{16G\,\epsilon^2}.
}
\]
For comparison, the area-law entropy diverges as
\[
S_A(\epsilon)\sim
+\frac{\Delta\phi\Delta\psi\,a\alpha_{sym}}
{16G\,\epsilon^2}.
\]
The leading local Wald contribution is therefore \(-5\) times the area-law coefficient. Both integrals are divergent. The finite cutoff part of this chosen primitive is not identified as a physical entropy.

## 4. A controlled regulator-sensitivity test

The area primitive from [#6457](https://github.com/Luminous-Dynamics/symtropy/issues/6457) is
\[
A_H(\epsilon)=C_A(\epsilon^{-2}+\beta\epsilon^2-1-\beta),
\qquad C_A=\frac{\Delta\phi\Delta\psi\,a\alpha_{sym}}4.
\]
Consider the smooth subleading change of defining function
\[
\widetilde\epsilon=\epsilon+s\epsilon^3+O(\epsilon^5),
\qquad
\epsilon=\widetilde\epsilon-s\widetilde\epsilon^3
+3s^2\widetilde\epsilon^5+O(\widetilde\epsilon^7).
\]
Then
\[
\frac{A_H}{C_A}
=\widetilde\epsilon^{-2}+2s-1-\beta
+(\beta-3s^2)\widetilde\epsilon^2+O(\widetilde\epsilon^4).
\]
The raw finite cutoff term shifts by \(2sC_A\).

For the Wald primitive,
\[
F(\epsilon(\widetilde\epsilon))
=\frac5{2\widetilde\epsilon^2}+5s+
\left(\frac{7\beta}{2}-\frac{15s^2}{2}\right)
\widetilde\epsilon^2+O(\widetilde\epsilon^4).
\]
The Wald finite cutoff part shifts by
\[
-\frac{5s\,\Delta\phi\Delta\psi\,a\alpha_{sym}}{8G},
\]
which is \(-5\) times the shift in the area-law entropy finite part.

**Important gauge qualification:** this is a smooth non-geodesic defining-function change. It demonstrates regulator sensitivity within the broader class of smooth cutoffs. It does not prove dependence within a fixed geodesic-defining-function gauge and fixed conformal representative. The next phase-space work must explicitly declare that gauge and test the allowed transformations.

## 5. What this does not solve

The source's covariant phase-space variation is
\[
\delta H_\xi=\oint_{\mathcal H}\sqrt{-g}\,k_\xi^{tr},
\]
with separate Einstein and Gauss–Bonnet terms. A local Wald density is only one component of the thermodynamic problem. The full task still requires:
- the rotating EGB presymplectic potential and the horizon-generator/axial \(k_\xi^{tr}\);
- leading and subleading charge-variation densities for parametric and symmetry-preserving variations;
- symplectic flux through the conformal end;
- boundary conditions ensuring integrability;
- a justified geodesic-defining-function/counterterm prescription or a global parent completion.

A recent 2026 covariant-phase-space entropy decomposition paper emphasizes that, for some nonminimal theories and non-smooth horizon fields, the entropy entering the first law can include contributions beyond the simple Wald term. That paper studies static examples, not this rotating NHEG; it is a cautionary method comparison rather than a result transferable without derivation.

## 6. Reproduction and claim ceiling

Run:
~~~sh
python3 derive_nheg_wald_density.py --output nheg_wald_density_v1.json
~~~
The script checks the multiplier, exact density, primitive, leading divergence, area and Wald re-expansions under the subleading defining-function change. The JSON fixture contains 95 numerical reference points.

**Claim ceiling:** this establishes the local Wald/Jacobson–Myers density and its exact regulated integral for the noncompact section, plus a specific regulator-sensitivity example. It does not establish a finite or unique renormalized entropy, integrable Iyer–Wald charges, finite \(J_\psi\), a global parent black hole, hyperbolicity or stability.
