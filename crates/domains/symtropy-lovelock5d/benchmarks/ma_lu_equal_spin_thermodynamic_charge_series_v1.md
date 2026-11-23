# Ma–Lü equal-spin thermodynamic charge benchmark — through spin tenth order

**Source pin:** Ma and Lü, *Quadratic curvature correction to 5D Myers–Perry metric*, JHEP 05 (2026) 081, DOI 10.1007/JHEP05(2026)081, published 8 May 2026. This uses the publisher version of record and Eqs. (2.9), (2.12), (3.2)–(3.4), (3.9). The article states that its rotating solution is linear in the higher-derivative coupling and expanded through tenth order in the rotation parameters.

## Equal-spin horizon parameter series

For chi_a=chi_b=chi, Eq. (3.9) becomes
\[
\frac{\mu}{r_h^2}
=\frac12+\frac{\chi^2}{2}
+\frac{5\chi^4}{8}
+\frac{7\chi^6}{8}
+\frac{21\chi^8}{16}
+\frac{33\chi^{10}}{16}
+O(\chi^{12}).
\]
The verifier derives this from the exact equal-spin MP horizon equation and checks every coefficient through chi^10.

## Equal-spin Komar/ADM mass correction

The version-of-record Eq. (3.4), after chi_a=chi_b=chi, gives
\[
\Delta M
=2\pi-\frac{47\pi}{10}\chi^2
+\frac{1683\pi}{280}\chi^4
-\frac{2047\pi}{420}\chi^6
+\frac{37921\pi}{6160}\chi^8
-\frac{90107\pi}{20020}\chi^{10}
+O(\chi^{12}).
\]
At zero higher-curvature coupling, Symtropy's physical spin is nu_S=Omega_H R_H and
\[
\chi=\sqrt2\,\nu_S\sqrt{1-\nu_S^2}.
\]
Re-expansion yields
\[
\Delta M
=2\pi-\frac{47\pi}{5}\nu_S^2
+\frac{2341\pi}{70}\nu_S^4
-\frac{9143\pi}{105}\nu_S^6
+\frac{184423\pi}{770}\nu_S^8
-\frac{655638\pi}{1001}\nu_S^{10}
+O(\nu_S^{12}).
\]
This is a useful independent high-order charge benchmark: it tests the thermodynamic correction without substituting the differential equation residuals for a charge calculation.

## Independent asymptotic metric check

From the equal-spin H1 asymptotics in Eq. (2.12),
\[
H_1=\left(\frac{16}{3}-\frac{188}{15}\chi^2\right)r^{-2}
+O(r^{-4}).
\]
With the five-dimensional Komar prefactor 3pi/8,
\[
\Delta M_{\rm metric}
=\frac{3\pi}{8}\left(\frac{16}{3}-\frac{188}{15}\chi^2\right)
=2\pi-\frac{47\pi}{10}\chi^2+O(\chi^4),
\]
which agrees exactly with Eq. (3.4) through chi^2. This independent H1 check does not claim that the H1 asymptotic coefficient alone reproduces the higher chi^4–chi^10 terms; those are checked directly from the published Eq. (3.4).

## Charges at fixed Symtropy boundary data

Ma–Lü hold the background parameters (mu,a,b) and r_h fixed in their perturbative construction. Symtropy instead compares at fixed R_H,Omega_H. The current first-coupling map is only known through the displayed low-spin order. Combining it with J_a=pi*mu*a/2, unchanged at fixed Ma–Lü parameters, gives
\[
\frac{M}{R_H^2}
=\frac{3\pi}{8(1-\nu_S^2)}
+\epsilon_S\left(\frac{3\pi}{4}-\frac{11\pi}{6}\nu_S^2\right)
+O(\epsilon_S\nu_S^4,\epsilon_S^2),
\]
\[
\frac{J_a}{R_H^3}=\frac{J_b}{R_H^3}
=\frac{\pi\nu_S}{4(1-\nu_S^2)}
+\epsilon_S\pi\left(\frac{\nu_S}{2}-\frac{46}{9}\nu_S^3\right)
+O(\epsilon_S\nu_S^5,\epsilon_S^2).
\]
**Important scope limit:** the total fixed-boundary ADM mass is only claimed through O(epsilon_S*nu_S^2), and J through O(epsilon_S*nu_S^3), because the current first-order H2/H4/H7 horizon/physical-Omega_H reparameterization is not yet known to higher spin. The direct alpha_R*DeltaM observable correction, re-expressed in the GR spin map, is independently checked through nu_S^10.

## Reproduction and exact-source provenance

Run python3 verify_ma_lu_equal_spin_thermodynamic_charge_series.py.

The exact public source bytes were fetched back, locally hash-checked and executed:
- Git blob SHA-1: e75dcd96941dadbc1f5f0940521d959060dbc776
- SHA-256: 038f21d147e07da45e1e3bb68e73b9bba8b3f5e102b2e05e250a068b09e31242
- Python 3.13.5 / SymPy 1.14.0
- Eq. (3.9) through chi^10, Eq. (3.4) through chi^10, the nu_S^10 re-expansion, the independent H1/Komar chi^2 check, fixed-boundary M,J to their declared orders, and both negative controls pass exactly.
