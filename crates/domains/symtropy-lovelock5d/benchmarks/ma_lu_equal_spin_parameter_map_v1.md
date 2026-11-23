# Ma–Lü equal-spin Myers–Perry parameter map

**Status:** exact kinematic relations and a SymPy series-inversion check, locally executed 23 November 2026.

The Ma–Lü paper defines the dimensionless rotations in Eq. (2.9) as
\[
\chi_a=\frac{a}{\sqrt{\mu}},\qquad \chi_b=\frac{b}{\sqrt{\mu}}.
\]
The square root is explicit in the published PDF. The earlier transcription \(\chi=a/\mu\) was incorrect and is superseded.

## Exact equal-spin relations

The five-dimensional MP horizon function and angular velocities are
\[
\Delta_r=\frac{(r^2+a^2)(r^2+b^2)}{r^2}-2\mu,\qquad
\Omega_a=\frac{a}{r_h^2+a^2},\quad
\Omega_b=\frac{b}{r_h^2+b^2}.
\]
For \(a=b\), define \(q=a/r_h\) and \(\nu=\Omega_Hr_h\). Then
\[
\mu=\frac{r_h^2}{2}(1+q^2)^2,\qquad
\nu=\frac{q}{1+q^2}.
\]
Using Eq. (2.9),
\[
\chi=\frac{a}{\sqrt{\mu}}
=\frac{\sqrt{2}\,q}{1+q^2}
=\boxed{\sqrt{2}\,\nu}.
\]
This relation is exact on the equal-spin branch. There is no dimensional ambiguity in the definition of \(\chi\).

The inverse relation for the physical MP ratio \(q\) is
\[
q=\frac{1-\sqrt{1-4\nu^2}}{2\nu}
=\nu+\nu^3+2\nu^5+5\nu^7+14\nu^9+42\nu^{11}+O(\nu^{13}).
\]
Thus \(q\) and Symtropy's \(\nu\) agree only at leading slow-spin order, while the paper's expansion parameter is exactly \(\chi=\sqrt2\nu\).

## Mass-series cross-check

Ma–Lü's Eq. (3.9), specialized to equal spins, gives
\[
\frac{\mu}{r_h^2}
=\frac12+\frac{\chi^2}{2}
+\frac{5\chi^4}{8}
+\frac{7\chi^6}{8}
+O(\chi^8).
\]
The exact horizon relation above reproduces this series after eliminating \(q\). The verifier checks this identity through \(\chi^6\).

## Candidate first-coupling action normalization

Symtropy's reduced action uses \(R+\alpha_{\rm GB}\mathcal L_{\rm GB}/4\), whereas Ma–Lü use \(R+\alpha_R R_{\mu\nu\rho\sigma}R^{\mu\nu\rho\sigma}\). On a Ricci-flat MP background, the two curvature-squared Euler tensors reduce to the same tensor, so the first-order source map is
\[
\alpha_R=\frac{\alpha_{\rm GB}}{4},\qquad
\epsilon_R=\frac{\alpha_R}{r_h^2}=\frac{\lambda}{4}
\]
at matched horizon radius. This identity applies to the first-order coupling correction about MP, not to generic finite-coupling metrics.

## Reproduction

\`derive_ma_lu_equal_spin_parameter_map.py\` checks the horizon identity, angular-velocity relation, Eq. (2.9) \(\chi=a/\sqrt{\mu}\), the exact \(\chi=\sqrt2\nu\) relation, the inverse series through \(\nu^{11}\), the Eq. (3.9) mass-series cross-check, and the first-coupling normalization. The script and JSON fixture record source hashes and Python/SymPy versions.

Claim ceiling: exact kinematic conversion and first-coupling coefficient normalization only. The metric/gauge transformation, component comparison, original-row checks and thermodynamic comparison remain open.
