# Asymptotically hyperbolic end of the singly rotating 5D EGB NHEG horizon

**Owner:** [#6457 BH-5D-EGB-NHEG-CONFORMAL-END-001](https://github.com/Luminous-Dynamics/symtropy/issues/6457)

**Status:** exact intrinsic horizon metric, sectional curvatures, scalar curvature, cutoff-torus extrinsic curvature and regulated volume have been derived symbolically. No renormalized physical entropy/charge or global parent spacetime is claimed.

## 1. The correct asymptotic object is a noncompact conformal end

The source's finite-curvature domain is explicitly **noncompact**. The polar coordinate is restricted to \(\theta\in[0,\pi/2)\); the endpoint \(\theta=\pi/2\) is a conformal end, not a smooth point that can be added to a compact \(S^3\). The norm \(g_{\psi\psi}\) diverges there, and the standard NHEG area, entropy and \(J_2\) integrals diverge. The source recommends studying charge densities rather than applying the usual compact-surface NHEG thermodynamics.

Source: Çelik, Hajian & Kunz, arXiv:2605.15005v2 (6 Sep 2026), Secs. 3.1 and 4, especially Eqs. (19)–(23).

## 2. Exact induced horizon metric

Use Symtropy's coupling convention
\[
\alpha_{sym}=4\alpha_{NH},\qquad
\beta=\frac{2a^2}{\alpha_{sym}},\qquad 0<\beta<1.
\]
Set
\[
\epsilon=\cos\theta,\quad p=1+\beta\epsilon^4,\quad
q=1-\beta\epsilon^4,\quad L^2=\frac{\alpha_{sym}}2.
\]
On a constant-\(t,r\) horizon section, the induced metric is diagonal:
\[
h_{\epsilon\epsilon}=
\frac{\alpha_{sym}q^2}
{2p\epsilon^2(1-\epsilon^2)},\quad
h_{\phi\phi}=\frac{a^2(1-\epsilon^2)}{\epsilon^2},\quad
h_{\psi\psi}=\frac{\alpha_{sym}p}{2\epsilon^2}.
\]
Its exact volume density, in the \(\epsilon\) coordinate, is
\[
\boxed{\sqrt{\det h}=
\frac{a\alpha_{sym}}{2\epsilon^3}(1-\beta\epsilon^4)}.
\]
Integrating from the regular axis \(\epsilon=1\) to a cutoff \(\epsilon\) gives
\[
\boxed{
A_H(\epsilon)=\Delta\phi\,\Delta\psi\,\frac{a\alpha_{sym}}4
\left(\epsilon^{-2}+\beta\epsilon^2-1-\beta\right).
}
\]
The leading divergence coefficient is exact. This is the same \(\epsilon^{-2}\) non-integrability found in the previous local measure artifact, now derived directly from the intrinsic horizon metric.

## 3. The horizon end is asymptotically hyperbolic

The proper radial distance satisfies
\[
d\sigma=\sqrt{h_{\epsilon\epsilon}}\,d\epsilon
=\frac{Lq}{\epsilon\sqrt{p(1-\epsilon^2)}}\,d\epsilon
\sim L\frac{d\epsilon}{\epsilon}.
\]
Thus \(\sigma\sim L\log(1/\epsilon)\), up to orientation and an additive constant. The tangential metric has conformal boundary
\[
\boxed{\gamma_{(0)}=\lim_{\epsilon\to0}\epsilon^2h_{\parallel}
=a^2d\phi^2+L^2d\psi^2.}
\]
If the globally unspecified \(\psi\) direction is assigned a finite period, the conformal boundary is a flat torus; without that quotient it is a cylinder/noncompact circle factor. The local metric does not choose this global identification.

The three sectional curvatures of the 3D horizon metric are exactly
\[
K_{\sigma\phi}=
-\frac{2(1-6\beta\epsilon^4-3\beta^2\epsilon^8)}
{\alpha_{sym}(1-\beta\epsilon^4)^3},
\]
\[
K_{\sigma\psi}=K_{\phi\psi}=
-\frac{2}{\alpha_{sym}(1-\beta\epsilon^4)}.
\]
Consequently,
\[
K_{\sigma\phi},K_{\sigma\psi},K_{\phi\psi}
\longrightarrow-\frac2{\alpha_{sym}}=-\frac1{L^2}.
\]
The intrinsic scalar curvature is
\[
\boxed{
R[h]=\frac{4(\beta^2\epsilon^8+10\beta\epsilon^4-3)}
{\alpha_{sym}(1-\beta\epsilon^4)^3}
=-\frac{12}{\alpha_{sym}}
+\frac{4\beta}{\alpha_{sym}}\epsilon^4
+O(\epsilon^8).
}
\]
This is a precise geometric characterization of the infinite-area horizon: its end approaches a three-dimensional hyperbolic geometry of radius \(L=\sqrt{\alpha_{sym}/2}\).

## 4. Cutoff-torus extrinsic curvature

With unit normal oriented toward \(\epsilon\to0\), the two mixed principal extrinsic-curvature eigenvalues are
\[
K^\phi{}_\phi=
\frac{\sqrt p}{Lq\sqrt{1-\epsilon^2}},\qquad
K^\psi{}_\psi=
\frac{\sqrt{1-\epsilon^2}}{L\sqrt p}.
\]
Their trace approaches
\[
K^\phi{}_\phi+K^\psi{}_\psi\longrightarrow\frac2L.
\]
Each fixed-\(\epsilon\) angular two-surface has constant-coefficient metric and zero intrinsic scalar curvature. These are the basic local data one would need for a cutoff-boundary or asymptotically-hyperbolic analysis.

## 5. Why this does not yet give a physical renormalized entropy

The regulated area has a finite Hadamard part in the chosen defining function:
\[
\operatorname{FP}A_H=
-\Delta\phi\,\Delta\psi\,\frac{a\alpha_{sym}}4(1+\beta).
\]
That number is **not** automatically an entropy. A change of boundary defining function can change the finite part, and a covariant phase-space charge has additional terms beyond the area measure in higher-curvature gravity. A legitimate thermodynamic prescription must derive:
- admissible boundary conditions at the conformal end;
- finite, integrable Iyer–Wald/covariant phase-space charge variations;
- vanishing or controlled symplectic flux;
- any counterterms from the variational principle or from a global parent completion;
- independence of physical charges from admissible cutoff choices.

Until those are supplied, the standard integrated entropy and second angular momentum remain divergent, while local charge densities can be investigated separately.

## 6. Reproduction and reference data

Run:
~~~sh
python3 derive_nheg_conformal_end.py --output nheg_conformal_end_v1.json
~~~
The script checks the induced metric, positive radial lapse, exact volume density and cutoff primitive, all three sectional curvatures, scalar-curvature series, cutoff-torus extrinsic curvature and conformal boundary metric. The machine fixture contains 95 reference points over \(\beta=0.05,\ldots,0.95\) and \(\epsilon=0.2,0.1,0.05,0.02,0.01\).

## Claim ceiling

This establishes an asymptotically hyperbolic **local horizon end** and its regulated volume. It does not establish a unique renormalized entropy/charge, a finite global phase space, hyperbolicity, stability, or a global asymptotically-flat parent black hole.
