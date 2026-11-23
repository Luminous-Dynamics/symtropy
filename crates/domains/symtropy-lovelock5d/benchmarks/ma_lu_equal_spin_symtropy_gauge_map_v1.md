# Equal-spin Ma–Lü to Symtropy radial-gauge map

**Status:** exact symbolic angular reduction and first-order radial-coordinate map, executed locally with Python 3.13.5 / SymPy 1.14.0 on 23 November 2026.

Source: Ma and Lü, *Quadratic curvature correction to 5D Myers–Perry metric*, JHEP 05 (2026) 081, arXiv:2512.23797, Eqs. (2.8)–(2.12).

## Equal-spin angular reduction

Set equal spins \(a=b\), \(x=\cos\theta\), and define
\[
A=r^2+a^2,\qquad \Delta_r=A^2/r^2-2\mu,\qquad
\eta=(1-x^2)d\phi+x^2d\psi.
\]
Ma–Lü's Eq. (2.8) has the orbit-space coefficient
\[
G(r)=(1+\alpha_R H_7(r))A.
\]
The equal-spin limit gives \(H_2=H_3\). Using the Eq. (2.12) coefficients through \(O(\chi^2)\), with \(\chi^2=a^2/\mu\), one finds
\[
H_5(A+D x^2)=A H_7+D x^2 H_4+O(\chi^4),
\]
\[
H_6(A+D(1-x^2))=A H_7+D(1-x^2)H_4+O(\chi^4),
\]
where \(D=2\mu a^2/A\). Both identities cancel exactly through \(a^2\). Therefore the full equal-spin angular part becomes
\[
G\,d\Omega_3^2+D(1+\alpha_R H_4)\eta^2
+O(\alpha_R^2,\chi^4),
\]
where \(d\Omega_3^2=d\theta^2+(1-x^2)d\phi^2+x^2d\psi^2\).

This is the key cohomogeneity-one reduction: the two-angle dependence collapses to a round \(S^3\) base plus a single Hopf-fibre deformation.

## Radial gauge transformation

Symtropy's metric ansatz uses \(g(R)=R^2\). The coordinate definition is
\[
R^2=G(r)=A(1+\alpha_R H_7(r)).
\]
The inverse to first order in \(\alpha_R\) is
\[
r=\sqrt{R^2-a^2}
-\frac{\alpha_R R^2 H_7(\sqrt{R^2-a^2})}
       {2\sqrt{R^2-a^2}}
+O(\alpha_R^2).
\]

Ma–Lü's radial metric coefficient is \(G/\Delta_r\). By the chain rule,
\[
f(R)=\frac{\Delta_r}{4G^2}\left(\frac{dG}{dr}\right)^2.
\]
Expanding this gives
\[
f(R(r))=\frac{\Delta_r r^2}{A^2}
\left(1+\alpha_R\frac{A}{r}H_7'(r)\right)
+O(\alpha_R^2).
\]
The Symtropy radial-gauge functions and time shift are
\[
g=G,\qquad h=G+D(1+\alpha_R H_4),
\]
\[
h\,w=\frac{2\mu a}{A}(1+\alpha_R H_2),
\]
\[
b=1-\frac{2\mu}{A}-\alpha_R H_1+h w^2.
\]
These are currently functions of the old Boyer–Lindquist coordinate \(r\); the full component comparison must compose with \(r(R)\).

## Remaining fixed-parameter conversion

Ma–Lü keep the Boyer–Lindquist horizon radius \(r_h\) and parameters \((\mu,a,b)\) fixed in their perturbation scheme. Symtropy instead fixes its areal radial-gauge horizon \(R_H\) and physical \(\Omega_H=w(r_h)\). The conversion cannot be skipped.

At zero coupling,
\[
R_H^2=r_h^2+a^2,\qquad
\nu_h=\Omega_Hr_h=\frac{q}{1+q^2},\qquad q=\frac{a}{r_h},
\]
while Symtropy's spin is
\[
\nu_S=\Omega_HR_H=\frac{q}{\sqrt{1+q^2}}.
\]
Thus \(q=\nu_S/\sqrt{1-\nu_S^2}\), \(r_h=R_H\sqrt{1-\nu_S^2}\), and
\[
\mu=\frac{R_H^2}{2(1-\nu_S^2)},\qquad
\chi=\sqrt{2}\,\nu_h=\sqrt{2}\,\nu_S\sqrt{1-\nu_S^2}.
\]
At nonzero \(\alpha_R\), solve simultaneously
\[
R_H^2=A_h(1+\alpha_R H_7(r_h)),\qquad
\Omega_H=w(r_h),
\]
then re-expand at fixed \((R_H,\Omega_H)\). That corrected parameter renormalization is not yet part of this derivation.

No equality with Symtropy's \(B_{21},F_{21},H_{21}\) is claimed yet. No full original-EOM residual or finite-coupling result is claimed.

## Reproduction and provenance

The derivation was executed locally with Python 3.13.5 / SymPy 1.14.0. The local source SHA-256 is ac07be432a1450e9deeb4368b71a05957122dc85b486238d4ba51757cfbbb4db; its local Git blob SHA-1 is 287e16dadd113350ade8eacd240f2a012f4459ca. The currently published source file is a lightly revised copy and has not yet been replayed byte-for-byte. The published file is indexed under derive_ma_lu_equal_spin_symtropy_gauge.py.

The symbolic controls cover H2=H3, the H5,H6 angular identities, the radial f chain rule, a wrong-chi^2 normalization negative control, and omission of the H7 radial redefinition. Claim ceiling: angular reduction and radial gauge transformation only; no horizon/Omega reparameterization, full coefficient comparison or finite-coupling result.
