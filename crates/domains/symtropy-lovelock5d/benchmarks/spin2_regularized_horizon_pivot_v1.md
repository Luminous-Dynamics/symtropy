# Horizon-regularized algebraic pivot for the 5D EGB spin-squared DAE

**Status:** exact symbolic desingularization of the algebraic coefficient multiplying `F2`; the complete rescaled `E_f` row has not yet been proved to extend smoothly to the horizon. No Frobenius recurrence or finite-coupling BVP is claimed.

**Parent work:** `spin2_constraint_structure_v1.json`, owned by [BH-5D-EGB-SPIN2-001 #6169](https://github.com/Luminous-Dynamics/symtropy/issues/6169).

## 1. The numerical issue

The open-exterior algebraic equation has the form

\[
E_f^{(2)}=c_F(r)F_2+\mathcal R_f=0,
\qquad
c_F=\frac{3rN}{(r^2+Q)Q\Delta^2},
\]

where

\[
Q=\sqrt{\alpha^2+2\alpha r_H^2+r^4},\qquad
\Delta=r^2+Q-\alpha-2r_H^2.
\]

The known Laurent expansion is

\[
c_F/r_H=\frac{3(1+\lambda)^2}{4y^2}+\frac{3}{y}+O(1),
\quad y=\rho-1,
\]

with \(\rho=r/r_H\) and \(\lambda=\alpha/r_H^2\). This is a pole in the chosen algebraic representation, not evidence that a regular physical \(F_2\) diverges.

## 2. Exact row scaling

Define

\[
\bar Q=\sqrt{\lambda^2+2\lambda+\rho^4},\qquad
\bar\Delta=\rho^2+\bar Q-\lambda-2,
\]

and

\[
\begin{aligned}
\bar N={}&\bar Q(\lambda^3+\lambda^2\rho^2+4\lambda^2
+2\lambda\rho^2+4\lambda+4\rho^6)\\
&+\lambda^3\rho^2+3\lambda^2\rho^4+4\lambda^2\rho^2
+6\lambda\rho^4+4\lambda\rho^2+4\rho^8.
\end{aligned}
\]

The dimensionless pivot is

\[
\bar c_F=\frac{c_F}{r_H}
=\frac{3\rho\bar N}{(\rho^2+\bar Q)\bar Q\bar\Delta^2}.
\]

Multiply the algebraic row by \(\bar\Delta^2\). The coefficient of \(F_2\) becomes

\[
\boxed{\mathfrak c(\rho,\lambda)
=\bar\Delta^2\bar c_F
=\frac{3\rho\bar N}{(\rho^2+\bar Q)\bar Q}.}
\]

For \(\lambda\ge0\), \(\rho\ge1\), all terms in \(\bar N\) are nonnegative, with \(4\bar Q\rho^6+4\rho^8\) strictly positive. Both denominator factors are positive. Hence \(\mathfrak c>0\) on the closed exterior.

At the horizon,

\[
\bar Q_H=\lambda+1,\qquad
\bar N_H=(\lambda+1)(\lambda+2)^3,
\]

and therefore

\[
\boxed{\mathfrak c_H=3(\lambda+2)^2>0.}
\]

The first horizon-relative derivative is also exact:

\[
\mathfrak c(1+y,\lambda)
=3(\lambda+2)^2
+3(\lambda+2)^2\frac{\lambda^2+4\lambda+5}{(\lambda+1)^2}y
+O(y^2).
\]

## 3. Consistency with the known Laurent pole

The denominator expansion is

\[
\bar\Delta=
\frac{2(\lambda+2)}{\lambda+1}y
+\left(1+\frac{3}{\lambda+1}-\frac{2}{(\lambda+1)^3}\right)y^2
+O(y^3).
\]

Combining it with the expansion of \(\mathfrak c\) recovers

\[
\bar c_F=\frac{3(\lambda+1)^2}{4y^2}+\frac{3}{y}+O(1),
\]

so both previously reported Laurent coefficients are reproduced exactly. The new representation is therefore cross-checked against, rather than substituted for, the original calculation.

## 4. Solver implication and required next check

On the open exterior, replacing \(E_f^{(2)}=0\) by \(\bar\Delta^2E_f^{(2)}=0\) preserves the zero set. Its \(F_2\) coefficient is now finite and nonzero at the horizon, which is a better-conditioned candidate representation for horizon-adjacent algebraic elimination and series coefficient extraction.

**Do not yet use the scaled row as a proven horizon equation.** First establish that the complete scaled remainder \(\bar\Delta^2\mathcal R_f\), including every term involving \(B_2,H_2\) and their derivatives, has an analytic extension to \(\rho=1\). Then expand the coupled \(E_b,E_h\) rows in the same regular variables and retain \(E_g\) as an independent constraint check.

The original unscaled \(E_f\), the other even equations, the radial Noether defect, and the exact Myers–Perry limit remain acceptance checks. Row scaling is a solver representation choice, not permission to remove a field equation.

## 5. Reproduction and claim ceiling

Run:

```sh
python3 derive_spin2_regularized_horizon_pivot.py \
  --output spin2_regularized_horizon_pivot_v1.json
```

The script checks the rescaled-coefficient identity, its exact horizon value and first derivative, and recovery of the two known Laurent coefficients using SymPy exact algebra. It also checks finite positivity on a deterministic 60-point binary64 grid.

This artifact does **not** prove smoothness of the full rescaled `E_f` row, derive the complete Frobenius recurrence, solve the finite-\(\alpha\) BVP, provide an interval enclosure, or establish existence/uniqueness.
