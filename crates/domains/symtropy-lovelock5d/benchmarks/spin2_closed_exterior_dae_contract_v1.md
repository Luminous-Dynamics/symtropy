# Closed-exterior algebraic reconstruction contract for the 5D EGB spin-squared DAE

**Status:** solver-interface contract derived from the exact action-derived second-order system and its horizon-regular scaled E_f row. It is not a BVP solve and does not claim full coupled Frobenius coefficients.

## 1. Reduce the state, not the evidence

The four even equations are

\[
E_b^{(2)}=E_f^{(2)}=E_g^{(2)}=E_h^{(2)}=0.
\]

The second-order constraint analysis proves that E_f contains F_2 algebraically and contains no F_2 derivatives. The complete row analysis proves that after multiplication by \(\bar\Delta^2\), its F_2 coefficient is finite and strictly positive at the horizon:

\[
\mathcal G_f =
\mathfrak c(\rho,\lambda)\widehat F_2+
\mathcal R_f^{\rm reg}
(\widehat B_2,\widehat B_2',\widehat H_2,\widehat H_2';\rho,\lambda)=0,
\]

\[
\mathfrak c(\rho,\lambda)>0,
\qquad
\mathfrak c(1,\lambda)=3(\lambda+2)^2.
\]

Here \(\mathcal R_f^{\rm reg}\) is the remainder of the *complete scaled row*, not the singular unscaled remainder.

Therefore define

\[
\boxed{
\widehat F_2(\rho)=
-\frac{\mathcal R_f^{\rm reg}
(\widehat B_2,\widehat B_2',\widehat H_2,\widehat H_2';\rho,\lambda)}
{\mathfrak c(\rho,\lambda)}.
}
\]

For analytic \(\widehat B_2,\widehat H_2\), the right-hand side is analytic through \(\rho=1\). This is an algebraic reconstruction on the closed exterior, provided the differential pair is analytic and satisfies the remaining field equations.

**Implementation consequence:** the nonlinear/linear BVP state vector should contain only the differential pair \((\widehat B_2,\widehat H_2)\). \(\widehat F_2\) is reconstructed at every residual evaluation from the scaled algebraic row. This is consistent with the existing DAE principal-part reduction; it should not be interpreted as deleting E_f from qualification.

## 2. Horizon boundary data are compatibility data

The fixed-horizon conditions are

\[
\widehat B_2(1)=0,\qquad \widehat F_2(1)=0.
\]

The zeroth-order scaled E_f row becomes proportional to
\(\widehat F_2(1)-\widehat B_2(1)\) and is automatically satisfied by these values. Since the row scaling factor vanishes at the horizon, the first nonzero Taylor coefficient must also be imposed:

\[
\widehat F_2'(1)
-\frac{2\lambda}{3(\lambda+1)^2}\widehat H_2(1)
+\frac{\widehat H_2'(1)}{3(\lambda+1)}
+\frac{2(\lambda+2)^2}{3(\lambda+1)^2}=0.
\]

This condition is derived from the full scaled E_f row. It provides the initial algebraic derivative relation for the horizon patch. It is only one member of the coupled horizon recurrence: the E_b and E_h equations still determine the differential horizon data, and E_g remains an independent constraint check.

## 3. Exact known-answer / negative-control pair

At \(\lambda=0\), the exact equal-spin Myers–Perry second-order profiles are

\[
\widehat B_2=-\rho^{-2}(1-\rho^{-4}),\qquad
\widehat F_2=-\rho^{-2}(1-\rho^{-2}),\qquad
\widehat H_2=\rho^{-2}.
\]

Their horizon data are

\[
\widehat H_2(1)=1,\quad
\widehat H_2'(1)=-2,\quad
\widehat F_2'(1)=-2.
\]

Substitution into the compatibility relation gives exactly zero. The deliberate mutation
\(\widehat H_2'(1)=+2\) gives residual \(4/3\) and is rejected. Both checks are replayed by **derive_spin2_scaled_ef_horizon_row.py** and recorded in **spin2_scaled_ef_horizon_row_v1.json**.

## 4. Three-patch solver contract

### Horizon patch

- Use analytic power-series coefficients for \(\widehat B_2,\widehat H_2\).
- Reconstruct every \(\widehat F_2\) Taylor coefficient algebraically from \(\mathcal G_f=0\).
- Generate the remaining recurrence from the original E_b and E_h rows, while checking E_g through the exact radial Noether identity.
- Do not initialize by dividing the unscaled E_f row by its \(\Delta^{-2}\) pivot.

### Open-exterior patch

- Solve the reduced two-function differential system obtained by differentiating the algebraic reconstruction and substituting it into E_b and E_h.
- Scale dependent variables and the radial coordinate; use the condition estimate to choose precision and collocation order.
- Evaluate all four original equations, E_w^(1), and the Noether defect on independent nodes.
- Retain the unscaled E_f residual away from the endpoint.

### Asymptotic patch

- Derive the homogeneous basis from the finite-\(\lambda\) asymptotic operator, not just from the Einstein-limit modes.
- Separate physical mass/charge parameter directions from gauge and normalization modes.
- Match mass and angular momentum using the declared fixed-\(r_H\) ensemble; rerun separately at fixed ADM energy rather than mixing the ensembles.

The patches must agree on at least one overlap interval. Their matching error is a qualification metric, not merely a plotting aid.

## 5. Conditioning and precision policy

The current finite-grid conditioning probe is binary64 evidence only; it observed a maximum 2-norm condition estimate of approximately \(2.25\times10^8\) on its frozen \(\lambda\le10\) grid. It is not a global bound.

Use a deterministic policy:
- binary64 for initial discovery and residual screening;
- extended precision when the estimated condition and requested observable tolerance demand it;
- ball arithmetic for the final enclosure;
- unresolved if a required inverse, denominator, tail, or boundary bound cannot be enclosed.

Never accept a candidate only because a nonlinear solver converged or because a collocation residual is small.

## 6. Independent acceptance contract

For each candidate, retain:

1. all four original even equations \(E_b,E_f,E_g,E_h\);
2. first-order angular equation and exact angular Noether flux;
3. radial Noether identity defect;
4. horizon compatibility and asymptotic boundary residuals;
5. areal/isotropic invariant-observable comparison;
6. held-out and full-domain residual bounds;
7. spectral-tail and approximate-inverse defects;
8. thermodynamic first-law residual;
9. replay receipt with exact action/equation and coefficient hashes.

A successful reduced solve does not qualify a solution until every independent check passes.

## 7. Claim ceiling

The full scaled E_f row is analytically regular, determines F_2 algebraically, and supplies one exact horizon compatibility relation. The Myers–Perry horizon control passes and a wrong-sign mutation fails.

The coupled E_b/E_h horizon recurrence, E_g horizon qualification, finite-\(\alpha\) BVP, full-domain interval enclosure, radii-polynomial certificate and nonlinear rotating branch are still open. This document defines the solver interface and evidence contract; it is not evidence that a finite-coupling black hole solution has been computed.


## 8. Triangular horizon-series reconstruction of F-hat-2

The full scaled algebraic row can be written

\[
A_F(y)\widehat F_2(y)+A_B(y)\widehat B_2(y)
+A_{B'}(y)\widehat B_2'(y)
+A_H(y)\widehat H_2(y)
+A_{H'}(y)\widehat H_2'(y)+S(y)=0,
\]

where every coefficient is analytic at \(y=0\), and

\[
A_F(0)=3(\lambda+2)^2\ne0.
\]

Expand

\[
A_X(y)=\sum_{j\ge0}a_{X,j}y^j,\quad
S(y)=\sum_{j\ge0}s_jy^j,
\]

\[
\widehat F_2(y)=\sum_{n\ge0}f_ny^n,\quad
\widehat B_2(y)=\sum_{n\ge0}b_ny^n,\quad
\widehat H_2(y)=\sum_{n\ge0}h_ny^n.
\]

Coefficient matching gives the reusable triangular recurrence

\[
\boxed{
f_n=-\frac{1}{a_{F,0}}\left[
\sum_{j=1}^{n}a_{F,j}f_{n-j}
+\sum_{j=0}^{n}\left(
a_{B,j}b_{n-j}
+a_{B',j}(n-j+1)b_{n-j+1}
+a_{H,j}h_{n-j}
+a_{H',j}(n-j+1)h_{n-j+1}
+s_j\right)\right].
}
\]

Thus \(f_n\) is determined algebraically once the differential-pair Taylor data through \(b_{n+1},h_{n+1}\) are known. No differential integration of F-hat-2 is needed. The recursion is triangular because the only occurrence of \(f_n\) is multiplied by the nonzero \(a_{F,0}\).

At order zero, the exact row gives

\[
f_0=b_0,
\]

so fixed-horizon data \(b_0=f_0=0\) are consistent. At order one, the exact coefficient values and slopes give

\[
f_1-\frac{2\lambda}{3(\lambda+1)^2}h_0
+\frac{h_1}{3(\lambda+1)}
+\frac{2(\lambda+2)^2}{3(\lambda+1)^2}=0,
\]

with the \(b_1\) terms cancelling. This is the compatibility relation independently checked against the Myers–Perry alpha-zero horizon data and its wrong-sign negative control.

This recurrence is an exact algebraic consequence of the full scaled E-f row. Higher-order E-b/E-h recurrence coefficients still have to be derived; the formula does not supply them or prove that a global finite-coupling BVP exists.
