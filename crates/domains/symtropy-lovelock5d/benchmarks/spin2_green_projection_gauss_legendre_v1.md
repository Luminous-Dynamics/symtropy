# Independent composite Gauss–Legendre oracle for the spin-2 Green projection

**Status:** executed locally on 22 November 2026 using Python 3.13.5 and mpmath 1.3.0.

This is a second numerical route, distinct from the tanh–sinh quadrature fixture. It does not import SymPy, the action-row fixture, or the symbolic Green–Lagrange verifier.

## Endpoint treatment

After \(s=1-t^2\), the transformed integrand contains an endpoint logarithm at \(t=0\). Its exact logarithmic coefficient is
\[
L(t)=\frac{4t(1+t^2)(3t^4+6t^2-10)}{(1-t^2)^3}
=\sum_{k\ge0}\ell_k t^{2k+1}.
\]
The first thirteen coefficients are
\[
(\ell_0,\ldots,\ell_{12})=(40,136,252,376,508,648,796,952,1116,1288,1468,1656,1852).
\]
We subtract \(\log(t)\sum_{k=0}^{12}\ell_k t^{2k+1}\), integrate the remaining regularized function using a composite 4-panel, 24-point-per-panel Gauss–Legendre rule, and restore the removed contribution analytically using
\[
\int_0^1 t^{2k+1}\log(t)\,dt=-\frac1{(2k+2)^2}.
\]

## Numerical results

| Working precision | \(C_H\) discrepancy | \(C_B\) discrepancy |
|---:|---:|---:|
| 40 digits | \(1.86\times10^{-35}\) | \(6.21\times10^{-36}\) |
| 60 digits | \(1.05\times10^{-49}\) | \(3.49\times10^{-50}\) |
| 80 digits | \(1.05\times10^{-49}\) | \(3.49\times10^{-50}\) |

The result agrees with
\[
C_H=67-6\pi^2,\qquad C_B=19-2\pi^2.
\]

The numerical discrepancies are not rigorous interval enclosures. This cross-check validates the Green endpoint integral and the \(B\)-amplitude reconstruction only; it does not validate the action-row derivation, solve a finite-\(\lambda\) BVP, or prove branch existence.

## Reproduction

~~~bash
python3 verify_spin2_green_projection_gauss_legendre.py \\
  --dps 40 60 80 --panels 4 --order 24 \\
  --output spin2_green_projection_gauss_legendre_v1.json
~~~
