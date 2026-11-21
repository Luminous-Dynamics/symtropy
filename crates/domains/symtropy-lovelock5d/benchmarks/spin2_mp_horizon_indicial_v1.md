# Einstein-limit horizon residue and indicial control for the 5D EGB spin-squared system

**Status:** exact \(\lambda=0\) horizon residue and indicial polynomial, derived by Taylor-jet elimination in the analytically scaled equations. This is the first Fuchsian endpoint result for the programme; it is not yet a finite-coupling result.

## 1. Reduced system and Taylor-jet method

At \(\lambda=0\),
\[
\bar Q=\rho^2,\qquad \bar\Delta=2(\rho^2-1),\qquad F_0=1-\rho^{-2}.
\]

Use the algebraic row
\[
\bar\Delta^2 E_f^{(2)}=0
\]
to reconstruct \(\widehat F_2\) in terms of \(\widehat B_2,\widehat B_2',\widehat H_2,\widehat H_2'\), and differentiate the reconstruction once to replace \(\widehat F_2'\) in \(E_b^{(2)},E_h^{(2)}\).

The analytic row scalings have horizon valuations
\[
F_0^2\bar\Delta^2 E_b^{(2)}
=\frac{\bar\Delta^4}{(\bar Q+\rho^2)^2}E_b^{(2)}
=64y^4 E_b^{(2)}+O(y^5),
\]
\[
F_0^2\bar\Delta E_h^{(2)}
=\frac{\bar\Delta^3}{(\bar Q+\rho^2)^2}E_h^{(2)}
=16y^3 E_h^{(2)}+O(y^4),
\]
where \(y=\rho-1\). The scaled \(E_f\) algebraic row has valuation zero.

Rather than expand the full reduced rational expressions, the verifier extracts affine coefficient numerators and their common denominators, computes exact Taylor jets, performs the algebraic elimination by series division, and checks that every reduced coefficient of the scaled \(E_b\) row vanishes through order three and every reduced coefficient of the scaled \(E_h\) row vanishes through order two. Dividing by the respective nonzero row-multiplier leading coefficients therefore gives finite horizon limits for every coefficient in the reduced second-order system at \(\lambda=0\).

This establishes a regular-singular first-order reduction at the Einstein limit: the reduced principal determinant has a simple zero, while all reduced lower-order coefficients have finite limits, so the first-order operator has at most a simple pole.

## 2. Horizon residue

Order the first-order state as
\[
U=(\widehat B_2,\widehat B_2',
\widehat H_2,\widehat H_2')^T.
\]
The reduced horizon principal and lower-order coefficient matrices are
\[
M_0=
\begin{pmatrix}
3/4&-1/4\\
-1/4&1/12
\end{pmatrix},
\]
\[
C_0=
\begin{pmatrix}
0&9/4&-2&5/4\\
0&-3/4&-10/3&1/4
\end{pmatrix},
\qquad
s_0=\begin{pmatrix}-12\\28/3\end{pmatrix}.
\]

Since
\[
\det M(1+y,0)=\frac12y+O(y^2),
\]
the leading inverse is \(M^{-1}=2y^{-1}\operatorname{adj}(M_0)+O(1)\). The first-order system consequently has
\[
\boxed{
y\frac{dU}{dy}=A_0U+g_0+O(y),
}
\]
with
\[
\boxed{
A_0=
\begin{pmatrix}
0&0&0&0\\
0&0&2&-1/3\\
0&0&0&0\\
0&0&6&-1
\end{pmatrix},
\qquad
g_0=
\begin{pmatrix}
0\\-8/3\\0\\-8
\end{pmatrix}.
}
\]

The residue has rank one, trace \(-1\), minimal polynomial \(s(s+1)\), and is diagonalizable.

## 3. Indicial spectrum

The indicial polynomial is
\[
\boxed{
p(s)=\det(sI-A_0)=s^3(s+1).
}
\]

Thus the local homogeneous exponents are:

- \(s=0\), multiplicity three and geometric multiplicity three;
- \(s=-1\), multiplicity one.

The \(y^{-1}\) mode is excluded by analytic horizon regularity. There are three regular homogeneous directions before imposing the fixed-horizon normalization. The exponent gap is one, so the singular branch is resonant with the regular sector; whether that singular branch requires a logarithm must be decided by the next Frobenius recurrence. No logarithmic-mode conclusion is claimed yet.

## 4. Inhomogeneous regularity condition and cross-equation result

An analytic solution has \(U(y)=U_0+O(y)\), so the order-zero equation is
\[
A_0U_0+g_0=0.
\]

Writing
\[
U_0=(b_0,b_1,h_0,h_1)^T
\]
gives two identical compatibility rows:
\[
2h_0-\frac13h_1-\frac83=0,
\qquad
6h_0-h_1-8=0.
\]
Therefore
\[
\boxed{
\widehat H_2'(1)=6\widehat H_2(1)-8.
}
\]

This is exactly the \(\lambda=0\) specialization of the derivative-free cubic combination
\[
[\mathcal R_g]_{y^3}-2[\mathcal R_h]_{y^3}=0
\]
derived in the companion horizon-coefficient calculation. The new interpretation is that the compatibility relation is the solvability condition of the inhomogeneous Fuchsian residue equation, rather than an additional independent boundary condition to be imposed on top of the reduced system.

For the exact Myers–Perry values \(h_0=1,h_1=-2\), the relation is satisfied. With the fixed-horizon value \(\widehat B_2(1)=0\), the local analytic horizon family has two remaining free data at this order, which can be taken as \(\widehat B_2'(1)\) and \(\widehat H_2(1)\).

## 5. Scope and remaining work

**Established at \(\lambda=0\):**
- exact reduced lower-order coefficient limits from scaled-row Taylor jets;
- the horizon residue \(A_0\) and inhomogeneous vector \(g_0\);
- a regular-singular first-order endpoint form;
- indicial roots \(0,0,0,-1\);
- the inhomogeneous horizon compatibility condition;
- agreement with the independent \(E_g-2E_h\) cubic relation.

**Still open:**
- general-\(\lambda\) residue and indicial roots;
- the next recurrence and logarithmic-resonance test;
- infinity indicial modes;
- the global boundary-data map, rank and Fredholm index;
- finite-coupling BVP solutions and validated existence;
- full spacetime principal-symbol and strong-hyperbolicity qualification.

The general-\(\lambda\) principal matrix is now known through its first horizon jet, but the symbolic lower-order Taylor-jet calculation remains significantly more expensive. The \(\lambda=0\) result is therefore an exact anchor and implementation control, not a substitute for finite-coupling analysis.

## 6. Reproduction

~~~bash
python3 derive_slow_rotation_second_order.py --emit-equations --output /tmp/spin2_equations.json
python3 derive_spin2_mp_horizon_indicial.py --equations /tmp/spin2_equations.json --output spin2_mp_horizon_indicial_v1.json
~~~

The verifier checks all four equation lengths, symbolic fingerprints and emitted text digests against the public generator metadata. The local input was a reconstructed equation JSON matching those metadata values, not a byte-for-byte clean-checkout replay. The committed verifier's exact GitHub blob replay remains a separate provenance receipt.

No finite-\(\lambda\) indicial result, CI pass, Rust/Lean result, interval certificate, finite-coupling BVP solution, or strong-hyperbolicity result is claimed.
