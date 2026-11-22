# Independent numerical oracle for the spin-2 Green projection

**Status:** locally executed independent arbitrary-precision numerical quadrature, 22 November 2026.

This control does not import SymPy, the action-row fixture, or the symbolic Green--Lagrange verifier. It evaluates the transformed scalar Green integral directly and compares the two endpoint amplitudes with their closed forms.

## Integral and endpoint regularization

The exact scalar projection is
\[
C_H=-2\int_0^1
\left(\frac2s+\frac{(2-s)\log(1-s)}{s^2}\right)
\left(\frac1s+12-3s\right)\,ds.
\]
The constant \(B\)-amplitude follows independently from the first reduced row and the horizon slope:
\[
C_B=(C_H-10)/3.
\]

The numerical implementation maps \(s=1-t^2\), so the logarithmic endpoint at \(s=1\) becomes an integrable \(t\log t\) endpoint. Near \(s=0\), it evaluates the Taylor series of the transformed integrand using exact rational coefficients, avoiding catastrophic cancellation in \(2/s+(2-s)\log(1-s)/s^2\).

## Results

| Working precision | \(C_H\) absolute discrepancy | \(C_B\) absolute discrepancy |
|---:|---:|---:|
| 40 decimal digits | \(3.67\times10^{-40}\) | \(1.26\times10^{-40}\) |
| 60 decimal digits | \(1.44\times10^{-46}\) | \(4.80\times10^{-47}\) |
| 80 decimal digits | \(7.75\times10^{-47}\) | \(2.58\times10^{-47}\) |

The independently integrated values agree with
\[
C_H=67-6\pi^2,\qquad C_B=19-2\pi^2.
\]

The integration uses mpmath 1.3.0 / Python 3.13.5. The reported error is a numerical discrepancy against the closed form, **not** a rigorous interval enclosure. This does not validate the action-row extraction, solve a finite-positive-coupling BVP, or prove a branch-existence theorem.

## Reproduction

~~~bash
python3 verify_spin2_green_projection_quadrature.py \\
  --dps 40 60 80 \\
  --output spin2_green_projection_quadrature_v1.json
~~~

The script source SHA-256 and numerical output are recorded in the adjacent JSON receipt.
