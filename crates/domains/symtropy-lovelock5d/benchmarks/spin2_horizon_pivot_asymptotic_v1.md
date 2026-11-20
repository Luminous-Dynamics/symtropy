# 5D EGB spin-squared horizon pivot: exact Laurent asymptotics

**Status:** exact symbolic expansion of the algebraic coefficient in the open-exterior DAE relation. This is *not* the full horizon Frobenius recurrence.

The DAE analysis found that \(E_f^{(2)}=0\) determines \(F_2\) algebraically in the open exterior:
\[
E_f^{(2)}=c_F(r)F_2+\mathcal R_f[B_2,H_2,B_2',H_2',W_1^2]=0,
\]
where
\[
c_F(r)=\frac{3rN(r)}{(r^2+Q)Q\Delta^2},\quad
Q=\sqrt{\alpha^2+2\alpha r_H^2+r^4},\quad
\Delta=r^2+Q-\alpha-2r_H^2.
\]

Let \(x=r-r_H\). At the horizon,
\[
Q_H=\alpha+r_H^2,\qquad
\Delta'(r_H)=\frac{2r_H(\alpha+2r_H^2)}{\alpha+r_H^2}.
\]
The numerator polynomial has the exact factorization
\[
N(r_H)=(\alpha+r_H^2)(\alpha+2r_H^2)^3.
\]
Using the Taylor expansion of \(Q\) through \(x^2\), sufficient to recover the first two Laurent coefficients, gives
\[
\boxed{
c_F(r)=
\frac{3(\alpha+r_H^2)^2}{4r_H}\frac{1}{x^2}
+\frac{3r_H^2}{x}+O(1).
}
\]

In dimensionless variables \(\lambda=\alpha/r_H^2\) and \(y=x/r_H\),
\[
\boxed{
\frac{c_F}{r_H}=
\frac{3(1+\lambda)^2}{4y^2}+\frac{3}{y}+O(1).
}
\]
Both displayed coefficients are exact for \(\alpha\ge0\) and \(r_H>0\).

## What this does—and does not—mean

The double pole belongs to the *algebraic elimination coefficient*. It does not imply that \(F_2\) diverges, and it is not a physical horizon singularity. A regular solution can only emerge when the other terms in \(E_f^{(2)}\) cancel the singular pivot contribution appropriately.

Therefore:
1. Do not evaluate the quotient \(F_2=-\mathcal R_f/c_F\) directly at \(x=0\).
2. Derive the complete regular Frobenius expansion of \(B_2,F_2,H_2\) from the unfixed action equations.
3. Match that horizon expansion to the open-exterior DAE representation at \(x>0\).
4. Keep \(E_g^{(2)}\) as an independent radial-constraint residual, with the radial Noether identity used as a cross-check.
5. Validate the matching under precision increase, overlap-radius movement and a second radial gauge.

## Reproduction

~~~sh
python3 derive_spin2_horizon_pivot_asymptotic.py \
  --output spin2_horizon_pivot_asymptotic_v1.json
~~~

The script uses only exact SymPy algebra for the Taylor coefficients. It does not contain a numerical horizon-to-infinity solver.

## Claim ceiling

This artifact establishes only the first two Laurent coefficients of the algebraic \(E_f\to F_2\) pivot. It does not derive the complete horizon regularity conditions, solve the finite-\(\alpha\) spin-squared BVP, provide an interval enclosure, or prove existence/uniqueness.
