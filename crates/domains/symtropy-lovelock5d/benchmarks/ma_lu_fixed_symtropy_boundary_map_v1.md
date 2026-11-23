# Ma–Lü to Symtropy: fixed-horizon / physical-angular-velocity map

**Source pin:** Ma and Lü, JHEP 05 (2026) 081, DOI 10.1007/JHEP05(2026)081, publisher version of record, 8 May 2026. Eq. (2.9) uses \(\chi=a/\sqrt{\mu}\).

Define
\[
q=\frac{a}{r_h},\quad \nu_h=\Omega_Hr_h,\quad
\nu_S=\Omega_HR_H,\quad
\epsilon_R=\frac{\alpha_R}{r_h^2},\quad
\epsilon_S=\frac{\alpha_R}{R_H^2}.
\]
For equal spins,
\[
\frac{\mu}{r_h^2}=\frac{(1+q^2)^2}{2},\quad
\nu_h=\frac{q}{1+q^2},\quad
\chi=\frac{\sqrt2 q}{1+q^2}=\sqrt2\nu_h.
\]
Symtropy fixes the areal-gauge horizon \(R_H\), with
\[
R_H^2=(r_h^2+a^2)(1+\alpha_R H_7(r_h)),\qquad
\nu_S=\Omega_HR_H,\qquad \Omega_H=w(r_h).
\]
The paper's background variables \((r_h,\mu,a)\) and Symtropy's \((R_H,\Omega_H)\) are therefore different parameter ensembles.

At first order in coupling and through the displayed slow-spin order, the exact-symbolic fixed-\((R_H,\Omega_H)\) map is
\[
\frac{R_H}{r_h}=1+\frac53\epsilon_S+\frac12\nu_S^2
+\frac{277}{90}\epsilon_S\nu_S^2+O(\epsilon_S^2,\nu_S^4),
\]
\[
q=\nu_S+\frac12\nu_S^3+\epsilon_S
\left(7\nu_S-\frac{3619}{90}\nu_S^3\right)
+O(\epsilon_S^2,\nu_S^5),
\]
\[
\frac{r_h}{R_H}=1-\frac53\epsilon_S-\frac12\nu_S^2
-\frac{127}{90}\epsilon_S\nu_S^2+O(\epsilon_S^2,\nu_S^4),
\]
\[
\frac{a}{R_H}=\nu_S+\frac{16}{3}\epsilon_S\nu_S
-\frac{2068}{45}\epsilon_S\nu_S^3+O(\epsilon_S^2,\nu_S^5),
\]
\[
\frac{\mu}{R_H^2}=\frac12-\frac53\epsilon_S+\frac12\nu_S^2
+\frac{454}{45}\epsilon_S\nu_S^2+O(\epsilon_S^2,\nu_S^4),
\]
\[
\chi=\sqrt2\left(\nu_S-\frac12\nu_S^3
+7\epsilon_S\nu_S-\frac{5509}{90}\epsilon_S\nu_S^3\right)
+O(\epsilon_S^2,\nu_S^5).
\]
Also \(\epsilon_R=\epsilon_S(1+\nu_S^2)+O(\epsilon_S^2,\nu_S^4)\), and the Symtropy coupling is \(\lambda_{GB}=4\epsilon_S\).

**Ensemble warning:** \(\mu/R_H^2\) above is the reparameterized background MP mass parameter, not the corrected ADM mass. Ma–Lü compute the latter from the asymptotic Komar integral; its equal-spin coefficient is separately checked against \(H_1\) in ma_lu_equal_spin_adm_mass_v1.json. Do not mix fixed background parameters, fixed \((R_H,\Omega_H)\), fixed thermodynamic potentials, and fixed charges.

**Claim ceiling:** exact first-order-coupling, equal-spin, slow-rotation conversion only. No finite-coupling solution or existence theorem follows.
