# Exact first coupling derivative of the 5D EGB spin-2 global boundary map

**Status:** exact coefficient calculation at \(\lambda=\alpha/r_H^2=0\), \(r_H=1\), through \(O(\lambda a^2)\). The two homogeneous columns are obtained in the fixed Einstein-limit horizon and asymptotic coordinates. This is a formal perturbative derivative, not a finite-\(\lambda\) branch-existence theorem.

## 1. Starting point and endpoint coordinates

The five reduced radial fields \(b,f,g,h,w\) are varied independently in the Einstein plus Gauss–Bonnet reduced action. The radial gauge \(g=r^2\) is imposed only after Euler–Lagrange variation. The two independent Einstein-limit horizon variations are

\[
m_b=\left(\frac{1-r^{-2}}2,0,0\right),\qquad
m_h=\left(0,-2r^2+3-r^{-2},\,2r^4-r^2\right)
\]

in the field order \((B_2,F_2,H_2)\). Their horizon coordinates are \(p=(b_1,h_0)^T\), where \(b_1=B_2'(1)\), \(h_0=H_2(1)\). The forbidden infinity amplitudes are \(C=(C_B,C_H)^T\), where \(C_B=\lim_{r\to\infty}B_2\) and \(C_H=\lim_{r\to\infty}H_2/r^4\).

At zero coupling the map is \(J_0=\operatorname{diag}(1/2,2)\).

## 2. Action-row extraction and algebraic reconstruction

The exact action-row extractor is extract_spin2_boundary_map_lambda1_rows.py; it records all four original even-sector coefficient rows \(E_b,E_f,E_g,E_h\) at \(O(\lambda a^2)\), before reduction. The algebraic \(E_f\) row has the positive exterior pivot

\[
p_f(r)=\frac{3r^5}{(r^2-1)^2}>0,\qquad r>1.
\]

Thus \(F_{21}\) is reconstructed algebraically. Substitution into \(E_b,E_h\) gives a two-field differential system for \(B_{21},H_{21}\). The radial \(E_g\) constraint remains represented in the original-row fixture and is propagated by the exact action-level radial Noether identity; it is not counted as a third independent evolution equation.

With \(x=r^2\), the two reduced rows are

\[
\mathcal E_b=\mathcal L_b[B,H]-\frac{9c_B}{2x}
+\left(6-\frac9x\right)c_H-\frac{33}{x}+\frac{24}{x^2}+\frac{50}{x^3}=0,
\]

\[
\mathcal E_h=\mathcal L_h[B,H]-\frac{9c_B}{2x}
+\left(4x^3+48x^2-12x+6-\frac9x\right)c_H
-8+\frac{31}{x}+\frac{56}{x^2}+\frac{50}{x^3}=0,
\]

where

\[
\mathcal L_b=3x^4B''+6x^3B'-x^2H''+2xH'-2H,
\]

\[
\mathcal L_h=3x^4B''+6x^3B'
+(-4x^4+4x^3-x^2)H''
+(-4x^2+2x)H'+(8x^2+4x-2)H.
\]

Here \(c_B,c_H\) select the two zeroth-order homogeneous horizon modes; the \(B,H\) in these equations are their first coupling sensitivities with those defining horizon coordinates held fixed.

## 3. First column: fixed \(b_1\) horizon mode

For \(c_B=1,c_H=0\), the exact solution is elementary:

\[
B_{21}^{(b)}(x)=\frac12-\frac{3}{4x}+\frac{1}{4x^3},\qquad
H_{21}^{(b)}(x)=0.
\]

It obeys \(B_{21}^{(b)}(1)=0\), \(\partial_xB_{21}^{(b)}(1)=0\), and both reduced equations exactly. Its boundary image is

\[
\left(C_B^{(b)},C_H^{(b)}\right)=\left(\frac12,0\right).
\]

## 4. Second column: scalar self-adjoint equation and Green projection

For \(c_B=0,c_H=1\), subtracting the two reduced rows cancels the \(B\) derivatives and gives

\[
\boxed{x^2(x-1)H''+xH'-(2x+1)H=x^2+12x-3.}
\]

The horizon-regular solution has \(H(1)=0\) and Frobenius slope \(H_x(1)=10\). Its first coefficients are

\[
H(1+t)=10t+\frac{17}{2}t^2-\frac12t^3+\frac38t^4
-\frac{59}{200}t^5+\frac6{25}t^6-\frac{981}{4900}t^7+\cdots.
\]

The scalar equation can be put in self-adjoint form with weight \(P(x)=(x-1)/x\). A growing homogeneous solution and a decaying adjoint solution are

\[
y_1=x(2x-1),\qquad
y_2=2x-x(2x-1)\ln\frac{x}{x-1},
\]

with the exact normalization

\[
P(x)\left(y_1y_2'-y_2y_1'\right)=1.
\]

The Green–Lagrange identity projects the \(x^2\)-growing coefficient \(C_H\) directly:

\[
C_H=-2\int_1^\infty y_2(x)\,
\frac{x^2+12x-3}{x^3}\,dx.
\]

With \(s=1/x\), the integrand has the exact primitive

\[
A(s)=-\frac1s+3-9s+
\left(3s+21-\frac{23}{s}-\frac1{s^2}\right)\ln(1-s)
+18\operatorname{Li}_2(s).
\]

The endpoint values are

\[
A(0)=\frac{53}{2},\qquad A(1)=3\pi^2-7.
\]

Consequently,

\[
\boxed{C_H=67-6\pi^2.}
\]

No large-radius fit is used.

## 5. Recovering the constant \(B_{21}\) amplitude

The first reduced row is

\[
3x^2(x^2B_x)'=x^2H_{xx}-2xH_x+2H-S_b,
\qquad S_b=6-\frac9x.
\]

With the fixed horizon conditions \(B(1)=B_x(1)=0\), Fubini's theorem and integration by parts give

\[
C_B=\frac13\left[
C_H-H_x(1)+H(1)-\int_1^\infty\frac{S_b(x)}{x^3}\,dx
\right].
\]

The source integral is exactly zero, \(H(1)=0\), and \(H_x(1)=10\). Hence

\[
\boxed{C_B=\frac{C_H-10}{3}=19-2\pi^2.}
\]

The algebraic \(E_f\) reconstruction independently gives the growing relation \(C_F=-C_H\), so the \(F_{21}\) growth is not a third independent asymptotic mode.

## 6. Complete first derivative and independent consistency check

Combining the two homogeneous columns gives

\[
\boxed{
J_1=
\begin{pmatrix}
\frac12&19-2\pi^2\\
0&67-6\pi^2
\end{pmatrix}.
}
\]

Therefore

\[
J(\lambda)=
\begin{pmatrix}1/2&0\\0&2\end{pmatrix}
+\lambda J_1+O(\lambda^2).
\]

The previously derived physical-branch tangent \(p_*'(0)=(106/9,-14/3)^T\) fixes the affine offset derivative independently:

\[
\partial_\lambda C(p_0,0)=-J_0p_*'(0)
=\left(-\frac{53}{9},\frac{28}{3}\right)^T.
\]

The verifier checks this cross-consistency, as well as the determinant and singular-value perturbation identities:

\[
\det J(\lambda)=1+
\left(\frac{69}{2}-3\pi^2\right)\lambda+O(\lambda^2),
\]

\[
\sigma_{\min}'(0)=\frac12,\qquad
\sigma_{\max}'(0)=67-6\pi^2,
\]

\[
\kappa_2'(0)=130-12\pi^2.
\]

The singular-value and condition-number slopes are for the fixed published Euclidean coordinates. They are not invariant under arbitrary endpoint basis scaling; use the normalization-aware protocol and frozen endpoint metrics when interpreting conditioning.

## 7. Reproduction

From the repository root:

~~~bash
python3 crates/domains/symtropy-lovelock5d/benchmarks/extract_spin2_boundary_map_lambda1_rows.py \
  --output /tmp/spin2_boundary_map_lambda1_action_rows_v1.json

python3 crates/domains/symtropy-lovelock5d/benchmarks/verify_spin2_boundary_map_lambda1.py \
  --rows /tmp/spin2_boundary_map_lambda1_action_rows_v1.json \
  --output /tmp/spin2_boundary_map_lambda1_v1.json
~~~

The action extractor performs five-field Euler–Lagrange variation before imposing \(g=r^2\). The verifier independently reduces the recorded rows, verifies the two homogeneous sensitivities, the Green–Lagrange projection, horizon recurrence, \(E_f\) growing-mode relation, and the known physical-branch offset derivative. \(E_g\) remains the Noether-propagated radial constraint; its original coefficient row is retained in the row fixture.

The exact source Git blob identifiers, fixture hashes, Python/SymPy versions, and replay results are recorded in spin2_boundary_map_lambda1_replay_v1.json.

## 8. Claim ceiling and next gates

This is the exact first derivative at \(\lambda=0\) of both columns of the forbidden-mode boundary map for the specified five-dimensional \(O(a^2)\) perturbative system, with fixed Einstein-limit endpoint coordinates.

Still open:
- independent finite-difference convergence of both columns against corrected small-positive-\(\lambda\) BVPs;
- finite-\(\lambda\) boundary-map invertibility in frozen, declared endpoint metrics;
- a finite-coupling BVP solution and validated continuation segment;
- an interval-Newton/radii-polynomial existence and local-uniqueness theorem;
- full nonlinear rotating EGB uniqueness and physical PDE hyperbolicity.

No finite-coupling existence or hyperbolicity conclusion is inferred from \(J_1\).
