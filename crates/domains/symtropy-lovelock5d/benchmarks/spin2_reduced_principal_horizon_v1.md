# General-coupling horizon principal block of the reduced 5D EGB spin-squared system

**Status:** exact symbolic result in \(\lambda\) for the horizon value and first radial jet of the reduced highest-derivative matrix. This is not yet a complete Fuchsian reduction, global determinant sign proof, BVP solution, or existence theorem.

## 1. Reduction and source equations

Use
\[
\rho=r/r_H,\qquad \lambda=\alpha/r_H^2,\qquad y=\rho-1,
\]
\[
\bar Q=\sqrt{\lambda^2+2\lambda+\rho^4},\qquad
\bar\Delta=\rho^2+\bar Q-\lambda-2.
\]

The algebraic row is the complete scaled equation
\[
\bar\Delta^2 E_f^{(2)}=0,
\]
which reconstructs \(\widehat F_2\) from
\(\widehat B_2,\widehat B_2',\widehat H_2,\widehat H_2'\).
Differentiate that reconstruction once to eliminate \(\widehat F_2'\) in the original, unscaled differential rows
\[
E_b^{(2)}=0,\qquad E_h^{(2)}=0.
\]

The reduced equations can then be written
\[
M(\rho,\lambda)
\begin{pmatrix}\widehat B_2''\\\widehat H_2''\end{pmatrix}
+\text{lower-order terms}=0.
\]
The coefficient matrix \(M\) is extracted directly from the action-derived equation strings. No horizon-scaled \(E_b\) or \(E_h\) row is used in this definition; the scaled \(E_f\) equation is used only for the regular algebraic pivot.

## 2. Exact horizon matrix

The exact horizon value is
\[
\boxed{
M_0(\lambda)=M(1,\lambda)=
\frac1{12}
\begin{pmatrix}3(\lambda+1)\\-1\end{pmatrix}
\begin{pmatrix}3(\lambda+1)&-1\end{pmatrix}.
}
\]
Equivalently,
\[
M_0=
\begin{pmatrix}
\frac34(\lambda+1)^2&-\frac14(\lambda+1)\\
-\frac14(\lambda+1)&\frac1{12}
\end{pmatrix}.
\]

This is rank one for every \(\lambda\ge0\). Its right null vector is
\[
\boxed{v_0=(1,\,3(\lambda+1))^T.}
\]
The nonzero eigenvalue is
\[
\tau_0=\operatorname{tr}M_0
=\frac{9(\lambda+1)^2+1}{12}>0.
\]

## 3. First radial jet and determinant valuation

The exact total derivative along \(\bar Q(\rho)\), using
\(\bar Q'(1)=2/(\lambda+1)\), gives
\[
\boxed{
M_1(\lambda)=\left.\frac{dM}{d\rho}\right|_{\rho=1}
=
\begin{pmatrix}
-\frac34(\lambda^2-3)&\frac14(\lambda+1)\\
\frac14(\lambda+1)&-\frac{\lambda^2+4\lambda-3}{12(\lambda+1)^2}
\end{pmatrix}.
}
\]

Using the 2-by-2 determinant expansion,
\[
\left.\frac{d}{dy}\det M(1+y,\lambda)\right|_{y=0}
=\operatorname{tr}(\operatorname{adj}(M_0)M_1)=\frac12.
\]
Therefore
\[
\boxed{\det M(1+y,\lambda)=\frac12 y+O(y^2).}
\]

This proves a **simple rank-one degeneracy** of the reduced principal block at the horizon for all \(\lambda\ge0\), rather than a coupling-specific rank drop.

Because the two eigenvalues at \(y=0\) are \(0\) and \(\tau_0\), the small eigenvalue has the expansion
\[
\mu_-(y)=\frac{6}{9(\lambda+1)^2+1}y+O(y^2),
\]
while \(\mu_+(y)=\tau_0+O(y)\). This is an eigenvalue-conditioning statement near the horizon, not a global condition-number bound.

The active and null derivative combinations suggested by the matrix are
\[
\chi=3(\lambda+1)\widehat B_2'-\widehat H_2',
\qquad
\psi=\widehat B_2'+3(\lambda+1)\widehat H_2'.
\]
Here \(\chi'\) is aligned with the range direction of \(M_0\), while \(\psi'\) is aligned with its null direction. This gives a principled candidate basis for the next horizon-weighted reduction. It does not yet establish that the transformed full system is Fuchsian.

## 4. Algebraic pivot and reduced-principal denominators

The unscaled coefficient of \(\widehat F_2\) in \(E_f^{(2)}\) can be written
\[
A_F=\frac{3P}{2\bar Q\rho(\bar Q+\rho^2)\bar\Delta^2},
\]
where
\[
\begin{aligned}
P=2\rho^2\big[&\bar Q(\lambda^3+\lambda^2\rho^2+4\lambda^2
+2\lambda\rho^2+4\lambda+4\rho^6)\\
&+\lambda^3\rho^2+3\lambda^2\rho^4+4\lambda^2\rho^2
+6\lambda\rho^4+4\lambda\rho^2+4\rho^8\big].
\end{aligned}
\]
Every term in the bracket is nonnegative on \(\rho\ge1,\lambda\ge0\), and \(4\rho^8>0\). Thus \(P>0\), and the algebraic pivot cannot vanish on the open exterior. The scaled pivot has the finite positive horizon value
\[
\left.\bar\Delta^2 A_F\right|_{\rho=1}=3(\lambda+2)^2.
\]

The four denominator entries of \(M\), reduced modulo
\(\bar Q^2=\lambda^2+2\lambda+\rho^4\), are positive products proportional to
\[
P,\quad \rho(\bar Q+\rho^2)P,\quad
\rho(\bar Q+\rho^2)P,\quad
\bar Q\rho^3(\bar Q+\rho^2)^3P.
\]
Hence the Schur-reduced highest-derivative matrix has no additional algebraic denominator poles in the open exterior. This says nothing yet about zeros of its full determinant away from the horizon.

## 5. What this proves—and what it does not

**Established by the exact symbolic extraction:**

- \(M_0(\lambda)\) and \(M_1(\lambda)\) for all symbolic \(\lambda\ge0\);
- rank one at the horizon and null direction \(v_0\);
- universal first determinant coefficient \(1/2\);
- positive algebraic \(E_f\) pivot on the open exterior;
- positive representatives for the reduced principal denominators;
- no residual fractional powers in the reduced principal entries.

**Still open:**

- whether \(\det M\) has additional zeros for \(\rho>1\);
- whether all reduced lower-order coefficients are bounded/analytic at the horizon;
- whether the full first-order system is regular-singular rather than irregular;
- the residue matrix \(A_0(\lambda)\), indicial roots, Jordan structure and log resonances;
- horizon/infinity mode counts, boundary-map rank and Fredholm index;
- a finite-coupling BVP solution and validated existence/uniqueness;
- physical PDE hyperbolicity of the rotating background.

The next Fuchsian gate is to form the lower-order Schur coefficients in the analytically scaled rows, divide out their known horizon row valuations, and establish that the resulting unscaled coefficients have finite limits. Only then should the indicial operator be computed.

## 6. Reproduction and provenance

Generate the equations using the action-level generator:

~~~bash
python3 derive_slow_rotation_second_order.py --emit-equations --output /tmp/spin2_equations.json
python3 derive_spin2_reduced_principal_horizon.py --equations /tmp/spin2_equations.json --output spin2_reduced_principal_horizon_v1.json
~~~

The verifier pins the character lengths, symbolic fingerprints, and emitted equation text SHA-256 values for all four rows. The input fixture used for the local derivation was a local reconstruction whose four equation metadata values match those public digests. This is not yet a byte-for-byte clean-checkout replay of the public equation generator. The derivation script and fixture have been committed publicly; a replay from exact fetched GitHub script bytes remains an outstanding receipt.

No CI, Rust, Lean, Arb, finite-coupling BVP, Fuchsian indicial spectrum, or physical hyperbolicity result is claimed.
