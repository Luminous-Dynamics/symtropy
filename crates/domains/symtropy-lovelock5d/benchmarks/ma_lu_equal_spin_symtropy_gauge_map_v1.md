# Ma–Lü to Symtropy: verified radial/time-sector comparison

**Status:** exact first-coupling, slow-rotation comparison for the radial, temporal and frame-dragging sectors; all four original Symtropy even-sector action rows vanish exactly. Full angular-diagonal H5/H6 component matching remains open.

Source: Ma and Lü, *Quadratic curvature correction to 5D Myers–Perry metric*, JHEP 05 (2026) 081, arXiv:2512.23797, Eqs. (9)–(13). Equation (10) defines chi=a/mu; the compact factor in the spin-squared terms is mu*chi^2=a^2/mu. The H1 spin-squared bracket includes the -1076*mu^3/r^8 term.

## Radial/time-sector map

Let A=r^2+a^2, Delta=A^2/r^2-2*mu, and G=A*(1+alpha_R*H7). Transform to Symtropy's radial gauge by
\[
R^2=G(r),\qquad
f(R(r))=\frac{\Delta}{4G^2}\left(\frac{dG}{dr}\right)^2.
\]
Through first order,
\[
f(R(r))=\frac{\Delta r^2}{A^2}
\left(1+\alpha_R\frac{A}{r}H_7'(r)\right)+O(\alpha_R^2).
\]
Using Ma–Lü's H1,H2,H4,H7 functions, the corresponding Symtropy-sector functions in the old r coordinate are
\[
g=G,\qquad h=G+\frac{2\mu a^2}{A}(1+\alpha_RH_4),
\]
\[
h\,w=\frac{2\mu a}{A}(1+\alpha_RH_2),\qquad
b=1-\frac{2\mu}{A}-\alpha_RH_1+h w^2.
\]
The physical component comparison composes these with r(R) and the fixed-boundary parameter map.

## Fixed (R_H, Omega_H) profiles

Use epsilon_R=alpha_R/R_H^2, nu_S=Omega_H*R_H, and Symtropy's lambda=alpha_GB/R_H^2=4*epsilon_R. The converted profiles on the MP-connected branch are
\[
B_{21}(R)=\frac{(R^2-1)(11R^6+11R^4+22R^2+9)}{9R^{10}},
\]
\[
F_{21}(R)=\frac{(R^2-1)(22R^4-11R^2-35)}{18R^8},
\]
\[
H_{21}(R)=-\frac{(R^2+1)(11R^2+3)}{6R^6}.
\]
The linear-spin W1(R) profile also agrees with Symtropy's exact finite-alpha first-order reference at the order needed for O(a^2) equations; the difference in W1=w/a_S begins at O(nu_S^2), which contributes to w at O(a^3) and w^2 at O(a^4).

## Four original equation rows: exact check

The original action-row fixture is spin2_boundary_map_lambda1_action_rows_v1.json. Substitution of the fixed-boundary profiles into the complete original Eb, Ef, Eg, Eh coefficient rows gives exact rational zero in each row:
\[
E_b=E_f=E_g=E_h=0.
\]
The positive algebraic Ef pivot independently reconstructs the same F21. The local verification uses cB=cH=0, because these are zeroth-order homogeneous-mode offsets for the MP-connected branch. The numbers
\[
B_{21}'(1)=106/9,\qquad H_{21}(1)=-14/3
\]
are the resulting first-coupling horizon sensitivities; they are **not** values to substitute for cB,cH. An earlier exploratory check incorrectly did so and produced spurious nonzero residuals. Negative controls reject that substitution and a mutated H1 profile.

## Important open boundary

This increment does not yet certify the full angular-diagonal dphi^2,dpsi^2 sectors H5,H6. Those are independent components in the published two-angle ansatz and must be checked directly against the reduced Hopf-frame metric before the complete five-dimensional metric can be declared matched. The corrected gauge script intentionally reports this boundary rather than claiming a full angular-block proof.

The converted mu/R_H^2 is the background MP parameter, not the corrected ADM mass. Extract the ADM mass from the full transformed asymptotic metric, including H1, with the selected ensemble explicit.

## Claim ceiling

This is a formal first-coupling, equal-spin, slow-rotation metric-sector comparison and exact original-row validation. It is not a finite-coupling nonlinear EGB solution, interval existence theorem, global uniqueness proof, full hyperbolicity result, or stability calculation.
