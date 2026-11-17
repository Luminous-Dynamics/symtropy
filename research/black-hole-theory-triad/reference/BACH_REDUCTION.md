# Reduced Bach-equation check for the static spherical branch

**Status:** analytic reduction encoded in the research reference kernel; Rust tests authored but not executed here. This is not a general tensor implementation.

## Source and convention

Use the static spherical line element

\[
ds^2=-B(r)dt^2+B(r)^{-1}dr^2+r^2d\Omega^2,
\]

with signature \((-+++ )\) and a static orthonormal frame on a domain where \(B(r)>0\).

The reduced expressions below are from Mantica & Molinari (2023), Proposition 6.5, Eqs. (40), (45), and (46), [DOI 10.1007/s10714-023-03149-w](https://doi.org/10.1007/s10714-023-03149-w). Their static spherical Bach tensor has two independent scalar combinations, \(B_1+B_2\) and \(B_1-B_2\).

Define

\[
A=\frac{1-B}{r^2}+\frac{B''}{2},\qquad
E=\frac12\left(\frac{1-B}{r^2}+\frac{B'}r-\frac{B''}{2}\right),
\qquad
X=A'+\frac Ar.
\]

Then the two vacuum residual combinations are

\[
\begin{aligned}
B_1+B_2&=-\frac{2B}{r}X-\frac{2B}{3}X',\\
B_1-B_2&=-\frac83 AE-\frac43X\left(B'+\frac Br\right)-\frac{4B}{3}X'.
\end{aligned}
\]

Vacuum requires both to vanish. This is the exact reduced expression being evaluated by the Rust reference file; it does not numerically differentiate a metric or reconstruct a full four-index curvature tensor.

## Four-term ansatz reduction

Take

\[
B(r)=\frac{c_{-1}}r+c_0+c_1r+c_2r^2.
\]

Direct differentiation gives

\[
A=\frac{1-c_0}{r^2}-\frac{c_1}{r},\qquad
E=\frac{1-c_0}{2r^2}-\frac{3c_{-1}}{2r^3},
\qquad
X=-\frac{1-c_0}{r^3},\qquad
X'=\frac{3(1-c_0)}{r^4}.
\]

Substitution yields

\[
B_1+B_2=0,\qquad
B_1-B_2=\frac{4(c_0^2-3c_1c_{-1}-1)}{3r^4}.
\]

Thus the four-term metric is a vacuum solution exactly when

\[
c_0^2-3c_1c_{-1}-1=0.
\]

The quadratic coefficient \(c_2\) remains free in this vacuum condition.

## Mannheim–Kazanas substitution

For

\[
\begin{aligned}
c_{-1}&=-\beta(2-3\beta\gamma),\\
c_0&=1-3\beta\gamma,\\
c_1&=\gamma,\\
c_2&=-\kappa,
\end{aligned}
\]

the constraint becomes

\[
(1-3\beta\gamma)^2
+3\beta\gamma(2-3\beta\gamma)-1=0,
\]

which vanishes identically. This checks that the stated Mannheim–Kazanas parameterization satisfies the reduced vacuum constraint.

## Positive and negative controls in the Rust source

- Positive control: evaluate both residual combinations for a Mannheim–Kazanas parameter set at multiple radii in the static region; both should converge to floating-point roundoff.
- Negative control: perturb \(c_0\) while holding other coefficients fixed. The residual must be nonzero.
- Scaling control: the off-shell residual must scale as \(r^{-4}\).
- Domain control: refuse evaluation where \(B(r)\le0\), because the cited static-frame decomposition is not valid there.

Independent arithmetic in the development session checked the reduced expressions against the closed-form constraint on representative on-shell and off-shell coefficient sets. This is not execution of the Rust tests.

## Scientific limitations

1. The equations are a published symmetry-reduced Bach calculation for this static spherical ansatz, not an independent derivation of the full Bach tensor from arbitrary metric components.
2. A second independent implementation (for example, a general tensor/symbolic implementation) remains necessary before treating this as a qualified field-equation oracle.
3. The code does not establish global regularity, horizon structure, boundary conditions, perturbative stability, the rotating solution, or observational viability.
4. At a horizon the static orthonormal frame degenerates. Extend the check across horizons only using a regular frame or an explicitly justified analytic continuation.
