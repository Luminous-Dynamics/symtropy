# Areal / isotropic gauge map for the 5D EGB laboratory

**Status:** executed high-precision reference calculation for the static Einstein-branch Boulware–Deser (BD) background, including the first-order-in-spin transformation. This is a coordinate-map control, not an independent isotropic-gauge EGB field-equation solver.

**Owner:** [BH-5D-EGB-GAUGE-ATLAS-001 #6181](https://github.com/Luminous-Dynamics/symtropy/issues/6181).

## 1. Source gauge and correct coordinate map

Brihaye et al. use the isotropic-gauge variables
\[
b_I=\bar f,\quad f_I=\bar f/m,\quad
g_I=x^2m/\bar f,\quad h_I=x^2n/\bar f,\quad
w_I=\bar w/x.
\]
In the areal-base gauge \(g_A(R)=R^2\), equating radial and angular-base coefficients gives
\[
R^2=\frac{x^2}{f_I},\qquad
\frac{dR^2}{f_A}=\frac{dx^2}{f_I}.
\]
Therefore
\[
\boxed{\frac{d\log x}{dR}=\frac{1}{R\sqrt{f_A(R)}}},\qquad x/R\to1
\quad(R\to\infty).
\]
The shortcut \(x=R\sqrt{f_A}\) is wrong. It does not satisfy the radial metric identity.

Given the map, reconstruct the source variables by
\[
\bar f=b_A,\quad f_I=x^2/R^2,\quad
m=b_AR^2/x^2,\quad n=b_Ah_A/x^2,\quad
\bar w=xw_A.
\]
Then \(g_I=R^2,\ h_I=h_A,\ w_I=w_A\). This is a radial chart change, not a change of the Hopf-frame normalization.

## 2. Stable finite-coupling BD integral

Set \(\rho=R/r_H,\ \xi=x/r_H,\ \lambda=\alpha/r_H^2,\ C=2+\lambda,\ y=1/\rho\). The Einstein-branch lapse can be written without subtractive cancellation as
\[
F(y)=\frac{2(1-y^2)}
{1+\lambda y^2+\sqrt{1+\lambda(2+\lambda)y^4}}.
\]
Define
\[
J(y)=\frac{F(y)^{-1/2}-1}{y}.
\]
The asymptotic normalization is imposed directly:
\[
\boxed{\xi(\rho)=\rho\exp\left[-\int_0^{1/\rho}J(y)\,dy\right]}.
\]
It follows that
\[
\frac{d\log\xi}{d\rho}=\frac{1}{\rho\sqrt{F(\rho)}}>0
\quad(\rho>1,\lambda\ge0),
\]
so the map is monotone and invertible on the static exterior.

Two useful endpoint expansions follow analytically:
\[
F(\rho)=\frac{2(\rho-1)}{1+\lambda}+O((\rho-1)^2),
\]
\[
\xi-\xi_H=\xi_H\sqrt{2(1+\lambda)(\rho-1)}+O(\rho-1),
\]
and
\[
\xi=\rho-\frac{2+\lambda}{8\rho}+O(\rho^{-3}).
\]
Thus the simple areal-gauge horizon zero becomes quadratic in \(\xi-\xi_H\). Horizon Taylor data must not be copied between charts.

For \(\lambda=0\), the integral has the exact closed form
\[
\xi=\frac{\rho+\sqrt{\rho^2-1}}{2},\qquad
\rho=\xi+\frac{1}{4\xi},\qquad \xi_H=\frac12.
\]

## 3. First-order rotation transformation

At \(O(a)\), the radial geometry remains the static BD geometry:
\[
w_A=a\,\frac{2+\lambda}
{\rho^4\left(1+\sqrt{1+\lambda(2+\lambda)/\rho^4}\right)}
+O(a^3).
\]
The isotropic gauge function is therefore
\[
\bar w(\xi)=\xi\,w_A(\rho(\xi)).
\]
Even-sector metric backreaction first enters at \(O(a^2)\); this fixture does not claim the finite-\(\alpha\), \(O(a^2)\) solution.

## 4. Executed reference fixture

The file isotropic_gauge_reference.py generates isotropic_gauge_reference_v1.json with mpmath at 90 decimal working precision. It evaluates \(\lambda=0,0.7,3\), checks \(\rho=1.1,2,10\), and records the \(\rho=2\) chart values. Checks include:
- the \(\lambda=0\) analytic map;
- high-precision differentiation against \(d\xi/d\rho=\xi/(\rho\sqrt F)\);
- inverse-map round trips;
- positivity of the exterior lapse, isotropic radius, isotropic radial coefficient and coordinate Jacobian;
- the radial line-element identity;
- reconstruction of the isotropic angular-base and Hopf-fiber coefficients;
- the mapped first-order rotational function.

The script now fails closed if any tested absolute/relative defect exceeds \(10^{-80}\) at 90-digit working precision. The generated fixture records this threshold and the maximum isotropic reconstruction defect.

Representative horizon radii:
- \(\lambda=0:\ \xi_H=0.5\);
- \(\lambda=0.7:\ \xi_H\approx0.3802790616618158\);
- \(\lambda=3:\ \xi_H\approx0.1817243859986928\).

At the three validation radii, the largest reported inverse-map error is about \(1.1\times10^{-90}\); the maximum derivative and radial-metric defects are about \(2.5\times10^{-91}\) and \(4.9\times10^{-91}\), respectively. These are high-precision numerical consistency checks, **not rigorous enclosures** and not evidence of an EGB field-equation solve.

Reproduce with:

    python3 isotropic_gauge_reference.py --dps 90 --output isotropic_gauge_reference_v1.json

The script is a research-only reference and uses mpmath; it adds no production Python dependency.

## 5. Next qualification gates

1. Implement the inverse map as an independently tested monotone root solve, not merely by reusing the forward interpolation.
2. Derive isotropic-gauge reduced-action equations independently from the isotropic ansatz and vary before gauge fixing.
3. Check the exact BD, MP, and finite-\(\alpha\) \(O(a)\) controls in both gauges.
4. Transform a full nonlinear candidate and compare \(E,J,T_H,A_H,S_{\rm Wald},r_e\) and curvature invariants.
5. Add outward-rounded/Arb enclosures for the map derivative and Jacobian; split the exterior domain when necessary.
6. Require both gauges to pass held-out field residuals and global thermodynamic checks before accepting a candidate.

## Claim ceiling

This advances #6181 from a coordinate-map specification to an executed static-BD/linear-spin reference. It does not implement an independent isotropic EGB residual, certify the map with interval arithmetic, or demonstrate a nonlinear rotating solution in two gauges.
