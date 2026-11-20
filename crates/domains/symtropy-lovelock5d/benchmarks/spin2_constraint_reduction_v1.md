# 5D EGB spin-squared system: differential-algebraic reduction

**Status:** action-derived equation structure established symbolically; finite-\(\alpha\) numerical conditioning probed on a frozen grid. No BVP has been solved.

## 1. A hidden differential-algebraic structure

The second-order system contains four equations for \(B_2,F_2,H_2\). Inspection of the complete action-derived operators shows:

- \(E_f^{(2)}\) contains **no derivative of \(F_2\)**.
- Its coefficient multiplying \(F_2\) is
  \[
  c_F=\frac{\partial E_f^{(2)}}{\partial F_2}
  =\frac{3rN}{(r^2+Q)Q\Delta^2},
  \]
  where
  \[
  Q=\sqrt{\alpha^2+2\alpha r_H^2+r^4},\qquad
  \Delta=r^2+Q-\alpha-2r_H^2.
  \]
  Here \(N\) is the positive polynomial recorded in the machine-readable fixture.
  For \(\alpha\ge0\), \(r>r_H>0\), \(Q>0\), \(\Delta(r_H)=0\), and
  \(\Delta'=2r+2r^3/Q>0\). Every term in \(N\) is nonnegative, with strictly positive \(4Qr^6+4r^8\). Therefore \(c_F>0\) on the open exterior.

Thus \(E_f^{(2)}=0\) determines \(F_2\) algebraically as a function of \(B_2,H_2,B_2',H_2'\). This is a differential-algebraic reduction, not a guessed ansatz.

The coefficient scales as \(\Delta^{-2}\) toward the horizon. Do **not** evaluate the algebraic quotient naively at \(r=r_H\); derive a regular horizon Frobenius expansion and match it to the open-exterior formulation.

## 2. The reduced principal operator

After eliminating \(F_2\), the remaining differential equations are \(E_b^{(2)}=0\) and \(E_h^{(2)}=0\) for \(B_2,H_2\). The chain rule matters because \(F_2'\) depends on \(B_2'',H_2''\).

In the exact Einstein limit \(\alpha=0\), the reduced principal matrix is
\[
\mathsf P_0(r)=
\begin{pmatrix}
3r^3/4 & -r_H^2/(4r)\\
-r_H^2/(4r) & (2r^2-r_H^2)^2/(12r^5)
\end{pmatrix},
\]
with
\[
\boxed{\det\mathsf P_0(r)=\frac{r^2-r_H^2}{4}>0
\quad (r>r_H).}
\]
This proves nonsingularity of the reduced Einstein-limit principal block throughout the open exterior.

## 3. Exact radial constraint identity

The companion radial Noether artifact verifies the exact radial-diffeomorphism identity. At \(O(a^2)\), when \(E_w^{(1)}=0\), it determines \(E_g^{(2)}\) from the three equations used in the DAE reduction. Keep \(E_g\) as a separate residual in every candidate qualification bundle.

## 4. Finite-coupling conditioning probes

The research script samples \(\lambda=\alpha/r_H^2\in\{0,0.1,0.7,3,10,100\}\) and
\(\rho=r/r_H\in\{1.001,1.01,1.1,1.5,2,5,10,100\}\), with \(r_H=1\).

The binary64 probes found a positive sampled reduced principal determinant, but the largest estimated 2-norm condition number is approximately \(2.50\times10^{11}\). This is a conditioning warning, **not** a global positivity result. Large coupling and near-horizon points need better scaling, higher precision and a horizon-adapted chart. The recorded values are not interval enclosures.

## 5. Recommended solver formulation

1. Generate the full \(O(a^2)\) equations from the unfixed action.
2. Derive a horizon-regular Frobenius expansion for \(B_2,H_2,F_2\); avoid the singular exterior algebraic quotient exactly at the horizon.
3. On the open exterior, solve \(E_f^{(2)}=0\) algebraically for \(F_2\).
4. Integrate the coupled second-order pair \(E_b^{(2)},E_h^{(2)}\) for \(B_2,H_2\).
5. Match to the four Einstein-limit asymptotic modes and derive their finite-\(\alpha\) counterparts.
6. Evaluate all four even equations, the first-order angular equation, horizon regularity, asymptotic charges and the Noether identity defect. \(E_g\) remains a fail-closed acceptance gate.
7. Compare areal and independent isotropic formulations before claiming a physical solution.

Reproduce with:

~~~sh
python3 derive_slow_rotation_second_order.py --emit-equations --output /tmp/spin2_equations.json
python3 derive_spin2_constraint_structure.py --equations /tmp/spin2_equations.json --output spin2_constraint_structure_v1.json
~~~

## Claim ceiling

This is an equation-structure result. The finite-\(\alpha\) BVP, full-domain interval determinant, nonlinear rotating branch and radii-polynomial existence theorem remain open.
