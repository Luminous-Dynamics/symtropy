# Ma–Lü equal-spin Myers–Perry parameter map

**Status:** exact kinematic relations and a SymPy series-inversion check, locally executed 23 November 2026.

This note pins the literal equations in Ma and Lü, *Quadratic curvature correction to 5D Myers–Perry metric*, JHEP 05 (2026) 081, arXiv:2512.23797.

## Exact relations from the paper

Their horizon function and angular velocity are
\[
\Delta_r=\frac{(r^2+a^2)(r^2+b^2)}{r^2}-2\mu,
\qquad
\Omega_a=\frac{a}{r_h^2+a^2},
\quad \Omega_b=\frac{b}{r_h^2+b^2}.
\]
For equal spins \(a=b\), define \(q=a/r_h\) and \(\nu=\Omega_Hr_h\). Then
\[
\mu=\frac{r_h^2}{2}(1+q^2)^2,
\qquad
\nu=\frac{q}{1+q^2}.
\]
The paper prints \(\chi_a=a/\mu\), \(\chi_b=b/\mu\) in Eq. (16). Taking this literally gives
\[
\chi r_h=\frac{2q}{(1+q^2)^2}.
\]

There is a dimensional subtlety: Eq. (16) is described as defining dimensionless rotation parameters, while \(\mu\) in the horizon function carries dimensions of length squared. The coefficient expansion and Eq. (50) must be interpreted with the paper's length rescaling made explicit. Do not silently replace its printed \(\chi=a/\mu\) by \(a/\sqrt{\mu}\). The equal-spin Eq. (50) expansion is a useful cross-check:
\[
\mu=\frac{r_h^2}{2}+\frac{r_h^4\chi^2}{4}
+\frac{9r_h^6\chi^4}{32}+O(\chi^6).
\]

## Map to Symtropy's horizon spin

Symtropy uses \(\nu=\Omega_Hr_H=a_S/r_H\). Inverting the MP angular-velocity relation on the subextremal branch gives
\[
q=\frac{1-\sqrt{1-4\nu^2}}{2\nu}
=\nu+\nu^3+2\nu^5+5\nu^7+14\nu^9+42\nu^{11}+O(\nu^{13}).
\]
Thus \(q=\nu+O(\nu^3)\), so the physical MP rotation \(a/r_h\) and Symtropy's \(\Omega_Hr_H\) agree through the quadratic slow-spin order. They are not equal at finite spin.

The paper's printed parameter has a different leading normalization:
\[
\chi r_h=2\nu-2\nu^3-2\nu^5-4\nu^7-10\nu^9-28\nu^{11}+O(\nu^{13}).
\]
Consequently, the coefficient of the paper's \(\chi^2\) series cannot be compared directly to a coefficient in \(\nu^2\) without carrying this factor and the higher-order reparameterization.

## Candidate action normalization

Symtropy's reduced action fixture uses \(R+\alpha_{\rm GB}\mathcal L_{\rm GB}/4\), whereas Ma–Lü use \(R+\alpha_R R_{\mu\nu\rho\sigma}R^{\mu\nu\rho\sigma}\).

On a Ricci-flat MP background,
\[
\bar R_{\mu\nu}=0,\qquad \bar R=0,\qquad
\bar\nabla^\alpha\bar R_{\mu\alpha\nu\beta}=0.
\]
The Gauss–Bonnet Euler tensor
\[
H^{\rm GB}_{\mu\nu}
=2\big(RR_{\mu\nu}-2R_{\mu\alpha}R_\nu{}^\alpha
-2R^{\alpha\beta}R_{\mu\alpha\nu\beta}
+R_\mu{}^{\alpha\beta\gamma}R_{\nu\alpha\beta\gamma}\big)
-\frac12 g_{\mu\nu}\mathcal L_{\rm GB}
\]
therefore reduces to
\[
\bar H^{\rm GB}_{\mu\nu}
=2\bar R_\mu{}^{\alpha\beta\gamma}\bar R_{\nu\alpha\beta\gamma}
-\frac12\bar g_{\mu\nu}\bar R_{\alpha\beta\gamma\delta}^2.
\]
The Riemann-squared Euler tensor has the same algebraic terms and a double-divergence term proportional to \(\nabla^\alpha\nabla^\beta R_{\mu\alpha\nu\beta}\), which vanishes on the Ricci-flat background by the contracted Bianchi identity. Hence the two source tensors agree on MP. Matching their coefficients gives
\[
\alpha_R=\frac{\alpha_{\rm GB}}{4}.
\]

This is an equality of the **first-order coupling field-equation sources about a Ricci-flat background**. It is not an equivalence of the finite-coupling theories on general metrics. It does not by itself transform Ma–Lü's metric ansatz into Symtropy's radial gauge.

## Reproduction

`derive_ma_lu_equal_spin_parameter_map.py` checks the horizon identity, angular-velocity relation, literal printed \(\chi=a/\mu\) conversion, inverse series through \(\nu^{11}\), and the action coefficient relation with exact symbolic arithmetic. Its JSON output includes the source digest and SymPy/Python versions.

Claim ceiling: kinematic conversion and action-source normalization only. The metric/gauge transformation, coefficient comparison, original-row checks and thermodynamic comparison remain open.
