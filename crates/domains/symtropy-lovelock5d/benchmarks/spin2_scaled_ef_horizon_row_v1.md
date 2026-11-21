# Full horizon-regular extension of the 5D EGB spin-squared algebraic row

**Status:** the complete scaled E_f^(2) equation—not only its F2 pivot—has an exact real-analytic extension to the non-extremal horizon for lambda >= 0. The first nontrivial horizon compatibility condition is derived. The coupled E_b,E_h recurrence, independent E_g horizon check, and finite-coupling BVP remain open.

**Owner:** [BH-5D-EGB-SPIN2-001 #6169](https://github.com/Luminous-Dynamics/symtropy/issues/6169). This extends the pivot-only result in [PR #6599](https://github.com/Luminous-Dynamics/symtropy/pull/6599).

## 1. Reproducible source equation

The input is the full second-order equation JSON emitted from the committed action-derived generator, not a hand-transcribed expression:

~~~sh
python3 derive_slow_rotation_second_order.py \
  --emit-equations --output /tmp/slow_rotation_second_order_equations.json

python3 derive_spin2_scaled_ef_horizon_row.py \
  --equations /tmp/slow_rotation_second_order_equations.json \
  --output spin2_scaled_ef_horizon_row_v1.json
~~~

The verifier pins the E_f^(2) equation text length and SHA-256, and checks the original generator's symbolic equation fingerprint. The source generator is the action variation in derive_slow_rotation_second_order.py; it varies all five radial functions before imposing g=r^2.

## 2. Exact row scaling

Use dimensionless variables

\[
\rho=r/r_H,\qquad \lambda=\alpha/r_H^2,
\]

and dimensionless fields

\[
\widehat B_2=r_H^2B_2,\qquad
\widehat F_2=r_H^2F_2,\qquad
\widehat H_2=H_2.
\]

Set

\[
\bar Q=\sqrt{\lambda^2+2\lambda+\rho^4},\qquad
\bar\Delta=\rho^2+\bar Q-\lambda-2.
\]

Because rho > 0, the positive background radical satisfies

\[
\sqrt{1+\lambda(\lambda+2)/\rho^4}=\bar Q/\rho^2.
\]

The new equation is the full action-derived row multiplied by bar-Delta squared:

\[
\mathcal G_f(\rho)=\bar\Delta^2 E_f^{(2)}(\rho)=0.
\]

The expanded row is affine in exactly five field jets:

\[
\widehat F_2,\quad \widehat B_2,\quad
\widehat B_2',\quad \widehat H_2,\quad \widehat H_2'.
\]

There are no F2-prime, B2-double-prime, or H2-double-prime terms in this algebraic equation. The verifier replaces the five jets with formal polynomial indeterminates and checks that the complete scaled numerator has total degree one and exactly the five expected linear monomials plus its source term.

## 3. Why the complete scaled row is analytic

After the exact square-root rewrite and row scaling, the coefficient denominators reduce to:

| Term | Exact denominator |
|---|---|
| F-hat-2 | \(2\bar Q\rho(\bar Q+\rho^2)\) |
| B-hat-2 | \(2\bar Q\rho(\bar Q+\rho^2)\) |
| B-hat-2-prime | \(2\) |
| H-hat-2 | \(2\bar Q\rho^3(\bar Q+\rho^2)^2\) |
| H-hat-2-prime | \(2\bar Q\rho^2(\bar Q+\rho^2)^2\) |
| Source | \(\bar Q^2\rho^3(\bar Q+\rho^2)^4\) |

For lambda >= 0 and rho >= 1, we have Q-bar > 0, rho > 0, and Q-bar + rho squared > 0. No denominator contains Delta-bar. The background Q-bar is real-analytic because its radicand is strictly positive. Therefore every coefficient and the source in the complete scaled E_f^(2) row has a real-analytic extension to rho = 1, provided the field profiles are analytic.

This is stronger than the earlier result, which established only that the coefficient multiplying F2 was regular after scaling.

## 4. Horizon values and first derivatives

Let y = rho - 1. The exact coefficient values are:

\[
\begin{aligned}
[\widehat F_2]\mathcal G_f\big|_H&=3(\lambda+2)^2,\\
[\widehat B_2]\mathcal G_f\big|_H&=-3(\lambda+2)^2,\\
[\widehat B_2']\mathcal G_f\big|_H&=0,\\
[\widehat H_2]\mathcal G_f\big|_H&=0,\\
[\widehat H_2']\mathcal G_f\big|_H&=0,\\
\mathcal S_f\big|_H&=0.
\end{aligned}
\]

The first derivatives of those coefficients along Q-bar(rho) are:

\[
\begin{aligned}
\partial_\rho[\widehat F_2]\mathcal G_f\big|_H
 &=3(\lambda+2)^2\frac{\lambda^2+4\lambda+5}{(\lambda+1)^2},\\
\partial_\rho[\widehat B_2]\mathcal G_f\big|_H
 &=-3(\lambda+2)^2\frac{\lambda+3}{\lambda+1},\\
\partial_\rho[\widehat B_2']\mathcal G_f\big|_H
 &=3(\lambda+2)^2,\\
\partial_\rho[\widehat H_2]\mathcal G_f\big|_H
 &=-2\lambda\frac{(\lambda+2)^2}{(\lambda+1)^2},\\
\partial_\rho[\widehat H_2']\mathcal G_f\big|_H
 &=\frac{(\lambda+2)^2}{\lambda+1},\\
\partial_\rho\mathcal S_f\big|_H
 &=2\frac{(\lambda+2)^4}{(\lambda+1)^2}.
\end{aligned}
\]

The zeroth-order horizon row is

\[
3(\lambda+2)^2(\widehat F_2(1)-\widehat B_2(1))=0.
\]

For the fixed-horizon boundary data F-hat-2(1) = B-hat-2(1) = 0, this is automatically satisfied. The first nonzero coefficient in the horizon expansion is then the term of order y.

## 5. First nontrivial horizon compatibility condition

Using the exact coefficient slopes above, the O(y) term contains a cancellation of B-hat-2-prime(1). The resulting condition is

\[
\boxed{
\widehat F_2'(1)
-\frac{2\lambda}{3(\lambda+1)^2}\widehat H_2(1)
+\frac{1}{3(\lambda+1)}\widehat H_2'(1)
+\frac{2(\lambda+2)^2}{3(\lambda+1)^2}=0.
}
\]

This is a genuine derived horizon compatibility relation from the complete E_f^(2) row, rather than an inference from the pivot alone.

**Important:** multiplying by Delta-bar squared, which vanishes at the horizon, preserves the zero set on the open exterior but does not license imposing only the scaled row's value at the single endpoint. The first nontrivial Taylor coefficient must be enforced, and the original unscaled equation must still be checked for rho > 1.

## 6. Remaining work and claim ceiling

The next derivation must combine:

1. The regularized E_f^(2) recurrence above.
2. Horizon-regular expansions of E_b^(2) and E_h^(2).
3. The independent radial constraint E_g^(2) and the radial Noether identity.
4. The asymptotic homogeneous-mode basis and fixed-r_H versus fixed-ADM-energy parameter directions.

The complete scaled algebraic row is now analytically regular at the horizon, but the **coupled Frobenius recurrence remains incomplete**, no finite-alpha BVP is solved, and there is no interval solution enclosure or existence/uniqueness theorem yet.


## Provenance boundary

The local equation-emission replay used for this derivation was an independent SymPy reconstruction of the public action-variation generator. Its 8,174-character E_f expression matches the existing public generator fixture's recorded symbolic fingerprint; the verifier also pins the emitted expression text SHA-256. The byte-for-byte public generator script has **not** yet been executed from a clean checkout in this environment. That clean-checkout generator replay receipt remains outstanding and must be completed before treating the input-generation provenance as fully closed.
