# Ma–Lü to Symtropy fixed-horizon / physical-angular-velocity map

**Status:** exact symbolic first-order-coupling and slow-rotation series derived locally with Python 3.13.5 / SymPy 1.14.0 on 23 November 2026.

This is the missing parameter layer between Ma–Lü's fixed Boyer–Lindquist horizon parameterization and Symtropy's fixed areal horizon and physical angular velocity. It follows the equal-spin radial-gauge map in the companion note and Ma–Lü Eqs. (2.8)–(2.12).

## Distinct spin parameters

Define
\[
q=\frac{a}{r_h},\qquad
\nu_h=\Omega_Hr_h,\qquad
\nu_S=\Omega_HR_H,\qquad
\epsilon_R=\frac{\alpha_R}{r_h^2},\qquad
\epsilon_S=\frac{\alpha_R}{R_H^2}.
\]
Ma–Lü's equal-spin MP horizon relation gives
\[
\frac{\mu}{r_h^2}=\frac{(1+q^2)^2}{2},\qquad
\nu_h=\frac{q}{1+q^2},\qquad
\chi=\frac{a}{\sqrt{\mu}}=\sqrt2\,\nu_h.
\]
But Symtropy fixes \(\nu_S=\Omega_HR_H\), not \(\nu_h\). Already at zero coupling,
\[
R_H^2=r_h^2+a^2,\qquad
\nu_S=\frac{q}{\sqrt{1+q^2}},\qquad
q=\frac{\nu_S}{\sqrt{1-\nu_S^2}}.
\]
Thus it is incorrect to set \(\nu_h=\nu_S\) when comparing fixed-boundary solutions.

## First-coupling horizon values

Set \(r_h=1\) for the dimensionless expansion and use \(\epsilon_R=\alpha_R/r_h^2\). The Ma–Lü equal-spin horizon values through \(q^2\) are
\[
H_{2h}=-\frac{16}{3}+\frac{838}{45}q^2,\quad
H_{4h}=-\frac{88}{3}+\frac{4141}{45}q^2,\quad
H_{7h}=\frac{10}{3}-\frac{578}{45}q^2.
\]
The orbit-space conformal factor and physical angular velocity are
\[
\frac{R_H}{r_h}=\sqrt{(1+q^2)(1+\epsilon_R H_{7h})},
\]
\[
\nu_S=\frac{R_H}{r_h}\,
\frac{q}{1+q^2}
\frac{1+\epsilon_RH_{2h}}
{1+\epsilon_R(H_{7h}+q^2H_{4h})/(1+q^2)}.
\]
The \(\epsilon_R\) term in the denominator is essential: it comes from the fibre metric and is part of the physical horizon angular velocity.

Expanding:
\[
\nu_S=q-\frac12q^3-7\epsilon_Rq
+\frac{5509}{90}\epsilon_Rq^3
+O(\epsilon_R^2,q^5),
\]
\[
\frac{R_H}{r_h}
=1+\frac12q^2+\frac53\epsilon_R
-\frac{503}{90}\epsilon_Rq^2
+O(\epsilon_R^2,q^4).
\]

## Fixed Symtropy boundary data

After inversion and converting to \(\epsilon_S=\alpha_R/R_H^2\):
\[
\frac{\alpha_R}{r_h^2}
=\epsilon_S(1+\nu_S^2)+O(\epsilon_S^2,\nu_S^4),
\]
\[
q=\nu_S+\frac12\nu_S^3
+\epsilon_S\left(7\nu_S-\frac{3619}{90}\nu_S^3\right)
+O(\epsilon_S^2,\nu_S^5),
\]
\[
\frac{R_H}{r_h}
=1+\frac53\epsilon_S+\frac12\nu_S^2
+\frac{277}{90}\epsilon_S\nu_S^2
+O(\epsilon_S^2,\nu_S^4),
\]
\[
\frac{r_h}{R_H}
=1-\frac53\epsilon_S-\frac12\nu_S^2
-\frac{127}{90}\epsilon_S\nu_S^2
+O(\epsilon_S^2,\nu_S^4),
\]
\[
\frac{a}{R_H}
=\nu_S+\frac{16}{3}\epsilon_S\nu_S
-\frac{2068}{45}\epsilon_S\nu_S^3
+O(\epsilon_S^2,\nu_S^5),
\]
\[
\frac{\mu}{R_H^2}
=\frac12-\frac53\epsilon_S+\frac12\nu_S^2
+\frac{454}{45}\epsilon_S\nu_S^2
+O(\epsilon_S^2,\nu_S^4),
\]
\[
\chi=\sqrt2\left(\nu_S-\frac12\nu_S^3
+7\epsilon_S\nu_S-\frac{5509}{90}\epsilon_S\nu_S^3\right)
+O(\epsilon_S^2,\nu_S^5).
\]

## Verification and limitations

The local exact-symbolic prototype checks the inverse series by substitution, the GR limit, the fixed-horizon and fixed-\(\Omega_H\) expansions, and negative controls for:
- omitting \(H_7\) from the horizon radius;
- omitting \(H_2,H_4\) from the angular velocity;
- confusing \(\nu_h\) with \(\nu_S\);
- dropping the \(\sqrt2\) in \(\chi\).

Environment: Python 3.13.5 / SymPy 1.14.0. Local prototype source SHA-256:
0ec67b7a552c59497d5d9fafa3b0a330cc40b8c16f8cccba1bc89802dad2b651.

This result is a first-order-coupling, equal-spin, slow-rotation parameter conversion. It is not a corrected metric coefficient match, full EOM validation, finite-coupling existence result, or hyperbolicity result. The next step is to compose the radial inverse \(r(R)\) with this parameter map, compare \(B_{21},F_{21},H_{21}\) at fixed \((R_H,\Omega_H)\), and evaluate the four original Symtropy equations.


**Parameter versus charge:** the displayed \(\mu/R_H^2\) is only the converted background Myers–Perry parameter in Ma–Lü's perturbative construction. It is **not** the corrected ADM mass. The physical ADM mass must be extracted separately from the asymptotic \(1/R^2\) coefficient of the fully transformed metric, including the \(H_1\) correction and with the ensemble specified. Likewise, fixed background parameters, fixed \((R_H,\Omega_H)\), and fixed thermodynamic potentials/charges are distinct comparison ensembles.
