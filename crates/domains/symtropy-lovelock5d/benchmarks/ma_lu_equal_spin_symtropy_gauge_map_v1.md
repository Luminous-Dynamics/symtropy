# Ma–Lü to Symtropy: version-of-record metric comparison

**Status:** first-coupling, equal-spin, \(O(\chi^2)\) comparison. The JHEP version of record is the canonical source—not the earlier arXiv-v1 text extraction.

Source: Ma and Lü, *Quadratic curvature correction to 5D Myers–Perry metric*, JHEP 05 (2026) 081, DOI 10.1007/JHEP05(2026)081, published 8 May 2026. Use Eqs. (2.8), (2.9), (2.12), (3.3), and (3.4).

## Convention and equal-spin reduction

The version-of-record Eq. (2.9) defines
\[
\chi_a=\frac{a}{\sqrt{\mu}},\qquad \chi_b=\frac{b}{\sqrt{\mu}}.
\]
For equal spins, \(\chi^2=a^2/\mu\). The version-of-record Eq. (2.8) includes both angular-axis factors: \(x^2d\psi^2\) and \((1-x^2)d\phi^2\). Both matter for regularity and for a valid metric comparison.

Let
\[
A=r^2+a^2,\quad \Delta=A^2/r^2-2\mu,\quad
D=\frac{2\mu a^2}{A},\quad G=A(1+\alpha_RH_7),
\]
and \(\eta=(1-x^2)d\phi+x^2d\psi\). The equal-spin specialization of Eq. (2.12) obeys
\[
H_2=H_3,\quad
H_5=H_7-\frac{4a^2x^2}{45}S(r),\quad
H_6=H_7-\frac{4a^2(1-x^2)}{45}S(r),
\]
where
\[
S(r)=\frac{165}{r^4}+\frac{330\mu}{r^6}+\frac{150\mu^2}{r^8}.
\]
Substitution into the complete Eq. (2.8) angular components gives
\[
ds^2_{\mathrm{ang}}=G[d\theta^2+(1-x^2)d\phi^2+x^2d\psi^2]
+D(1+\alpha_RH_4)\eta^2+O(\alpha_R^2,a^4).
\]
The \(d\psi^2\), \(d\phi^2\), and \(d\phi\,d\psi\) components, both axis degeneracies, and wrong-sign/missing-axis-factor negative controls have all been checked by independent exact SymPy reduction.

## Radial/time/fibre map

In Symtropy radial gauge,
\[
R^2=G(r),\qquad f(R(r))=\frac{\Delta}{4G^2}\left(\frac{dG}{dr}\right)^2.
\]
The radial/time/fibre functions are
\[
g=G,\quad h=G+D(1+\alpha_RH_4),\quad
hw=\frac{2\mu a}{A}(1+\alpha_RH_2),
\]
\[
b=1-\frac{2\mu}{A}-\alpha_RH_1+hw^2.
\]
The published \(H_1\) includes the equal-spin \(-1076\mu^3\chi^2/r^8\) term. The fixed-boundary comparison composes with \(r(R)\) and the simultaneous \((r_h,\mu,a)\to(R_H,\Omega_H)\) parameter map.

## Fixed \((R_H,\Omega_H)\) and coupling normalization

The Ricci-flat first-coupling source identity gives \(\alpha_R=\alpha_{GB}/4\). Define
\[
\nu_S=\Omega_HR_H,\qquad \epsilon_S=\alpha_R/R_H^2=\lambda_{GB}/4.
\]
The parameter conversion distinguishes \(\nu_h=\Omega_Hr_h\) from \(\nu_S\). Its exact-symbolic series is stored in the fixed-boundary map fixture. The paper's spin parameter maps as
\[
\chi=\sqrt2\left(\nu_S-\frac{\nu_S^3}{2}
+7\epsilon_S\nu_S-\frac{5509}{90}\epsilon_S\nu_S^3\right)
+O(\epsilon_S^2,\nu_S^5).
\]
The converted \(\mu/R_H^2\) remains the background MP parameter, not the corrected ADM mass.

## Independent qualification results

The MP-connected fixed-boundary \(B_{21},F_{21},H_{21}\) profiles give exact zero in all four original Symtropy even-sector action rows \(E_b,E_f,E_g,E_h\) when \(c_B=c_H=0\). The algebraic \(E_f\) pivot independently recovers \(F_{21}\). The horizon derivatives \(B'_{21}(1)=106/9\) and \(H_{21}(1)=-14/3\) are sensitivity outputs, not zeroth-order horizon-mode offsets.

The transformed asymptotic \(g_{tt}\) coefficient gives
\[
H_{1,-2}=\frac{16}{3}-\frac{188}{15}\chi^2,\qquad
\Delta M_{\mathrm{metric}}=\frac{3\pi}{8}H_{1,-2}
=2\pi-\frac{47\pi}{10}\chi^2+O(\chi^4).
\]
This exactly matches the equal-spin specialization of Ma–Lü Eq. (3.4). The \(H_7\) radial redefinition preserves the leading inverse-square asymptotic coefficient.

## Claim ceiling

These checks establish a strong first-coupling, equal-spin, slow-spin overlap through \(O(\chi^2)\): the complete angular metric block, radial/time/fibre sectors, fixed-boundary map, four original action rows, and asymptotic mass coefficient. They do not establish a finite-coupling nonlinear solution, interval existence/uniqueness, complete nonlinear uniqueness, or strong hyperbolicity.
