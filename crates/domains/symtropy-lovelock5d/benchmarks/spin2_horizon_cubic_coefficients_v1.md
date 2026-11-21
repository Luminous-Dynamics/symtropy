# Cubic horizon coefficients of the 5D EGB spin-squared radial and angular rows

**Status:** exact cubic Taylor coefficients have been derived for the scaled \(E_g^{(2)}\) and \(E_h^{(2)}\) rows. The calculation uses the action-derived equation strings and checks their published text and symbolic fingerprints. It does not yet close the complete coupled Frobenius recurrence.

## 1. Variables and scaling

Use
\[
\rho=r/r_H,\qquad \lambda=\alpha/r_H^2,\qquad y=\rho-1,
\]
\[
\bar Q=\sqrt{\lambda^2+2\lambda+\rho^4},\qquad
\bar\Delta=\rho^2+\bar Q-\lambda-2,\qquad
F_0=\frac{\bar\Delta}{\bar Q+\rho^2}.
\]

The two rows are
\[
\mathcal R_g=F_0^2\bar\Delta E_g^{(2)},\qquad
\mathcal R_h=F_0^2\bar\Delta E_h^{(2)}.
\]

Their common denominators after exact collection are, respectively,
\[
D_g=\bar Q^4\rho^5(\bar Q+\rho^2)^8,
\qquad
D_h=2\bar Q^3\rho^5(\bar Q+\rho^2)^7.
\]

Both denominators are strictly positive for \(\rho\geq1,\lambda\geq0\).

## 2. Fixed-horizon Taylor data

Set
\[
\widehat B_2(1)=\widehat F_2(1)=0
\]
and abbreviate
\[
b_1=\widehat B_2'(1),\quad b_2=\widehat B_2''(1),\quad
f_1=\widehat F_2'(1),\quad f_2=\widehat F_2''(1),
\]
\[
h_0=\widehat H_2(1),\qquad h_1=\widehat H_2'(1).
\]

For both complete scaled rows, the order-\(y\) and order-\(y^2\) coefficients vanish after the fixed-horizon values and field-jet derivative relations are imposed. Their first nonzero generic coefficient is cubic.

Define the short polynomials
\[
P_b=3\lambda^4+6\lambda^3-6\lambda^2-18\lambda-9,
\]
\[
P_f=3\lambda^4+6\lambda^3+10\lambda^2+14\lambda+7.
\]

## 3. Exact \(E_g\) cubic coefficient

\[
\boxed{
\begin{aligned}
[\mathcal R_g]_{y^3}
={}&\frac{2(\lambda+2)}{(\lambda+1)^6}
\big[
P_b b_1-3(\lambda+1)^4b_2\\
&-P_f f_1-(\lambda+1)^4f_2\\
&+8(3\lambda^2+8\lambda+3)h_0\\
&-8(\lambda+1)h_1\\
&+8(3\lambda^4+18\lambda^3+37\lambda^2
+28\lambda+4)
\big].
\end{aligned}
}
\]

This closes the previously open cubic valuation of the radial-constraint row.

## 4. Exact \(E_h\) cubic coefficient

\[
\boxed{
\begin{aligned}
[\mathcal R_h]_{y^3}
={}&\frac{\lambda+2}{(\lambda+1)^6}
\big[
P_b b_1-3(\lambda+1)^4b_2\\
&-P_f f_1-(\lambda+1)^4f_2\\
&-24(3\lambda^2+6\lambda+1)h_0\\
&-24(\lambda^4+4\lambda^3+3\lambda^2-4\lambda-4)
\big].
\end{aligned}
}
\]

The coefficient multiplying \(h_0\) is
\[
-\frac{24(\lambda+2)(3\lambda^2+6\lambda+1)}
{(\lambda+1)^6},
\]
which is nonzero for every \(\lambda\geq0\). Thus the cubic order is genuinely the first generic nonzero horizon order of the complete \(E_h\) row under the fixed-horizon expansion.

## 5. Cross-equation cancellation

A particularly useful combination is
\[
[\mathcal R_g]_{y^3}-2[\mathcal R_h]_{y^3}.
\]

All \(b_1,b_2,f_1,f_2\) terms cancel exactly:

\[
\boxed{
\begin{aligned}
[\mathcal R_g]_{y^3}-2[\mathcal R_h]_{y^3}
={}&\frac{16(\lambda+2)}{(\lambda+1)^6}
\big[
12\lambda^2h_0+26\lambda h_0+6h_0\\
&-(\lambda+1)h_1\\
&+6\lambda^4+30\lambda^3+46\lambda^2
+16\lambda-8
\big].
\end{aligned}
}
\]

This yields a horizon compatibility relation involving only the horizon value and first derivative of \(\widehat H_2\), together with the coupling.

It is **not yet classified as an independent boundary condition**. The action-level radial Noether identity in spin2_radial_noether_identity_v1.md shows that \(E_g\) is a propagated constraint once the remaining Euler-Lagrange equations hold. The cubic combination must therefore be placed in the differentiated Noether/constraint hierarchy before counting independent horizon data.

## 6. Exact controls

At \(\lambda=0\), the Myers–Perry horizon data are
\[
b_1=-4,\quad b_2=36,\quad f_1=-2,\quad f_2=14,\quad
h_0=1,\quad h_1=-2.
\]
Both cubic coefficients vanish exactly, as does their difference.

Changing \(h_1\) from \(-2\) to \(-1\) makes the \(E_g\) cubic coefficient nonzero. This is an exact wrong-sign horizon-derivative negative control.

## 7. Reproduction and provenance

Generate the full equation input using the public action-level derivation:

~~~bash
python3 derive_slow_rotation_second_order.py \
  --emit-equations --output /tmp/spin2_equations.json
~~~

Then run:

~~~bash
python3 derive_spin2_horizon_cubics.py \
  --equations /tmp/spin2_equations.json \
  --output spin2_horizon_cubic_coefficients_v1.json
~~~

The local action-level reconstruction used for this derivation produced all four equation lengths, symbolic srepr SHA-256 digests, and emitted-text SHA-256 digests matching the public generator metadata exactly. The standalone cubic verifier is committed as a reproducible artifact; its clean-checkout end-to-end replay remains an outstanding gate.

## 8. Next mathematical tasks

1. Combine the \(E_f\) triangular algebraic reconstruction with the \(E_b,E_h\) differential rows.
2. Differentiate the radial Noether identity and identify exactly which cubic horizon relations are propagated constraints.
3. Derive the reduced two-field principal matrix and its determinant before asserting Fuchsian form.
4. Prove whether its singularities are regular; if necessary, find a minimal weighted/sheared state with a simple-pole first-order operator.
5. Compute horizon and infinity indicial roots, logarithmic resonances, and the boundary-map rank only after the reduced operator is established.

**Claim ceiling:** this artifact establishes exact cubic horizon coefficients and controls for \(E_g,E_h\). It does not establish the complete recurrence, a finite-coupling BVP solution, the EGB boundary index, validated existence, or uniqueness.
