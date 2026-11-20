# Strong-coupling asymptotics of the 5D BD isotropic horizon coordinate

**Status:** matched-asymptotic derivation plus a 90-decimal numerical comparison over a frozen strong-coupling grid. This is a statement about a radial coordinate chart, not a physical change in the areal horizon radius.

Related gauge work: [#6181](https://github.com/Luminous-Dynamics/symtropy/issues/6181). Independent isotropic action work: [#6202](https://github.com/Luminous-Dynamics/symtropy/issues/6202).

## 1. The coordinate-conditioning problem

For the static Einstein-branch 5D Boulware–Deser solution in units \(r_H=1\), define
\[
\lambda=\alpha/r_H^2,\quad \rho=R/r_H,\quad \xi=x/r_H.
\]
The isotropic coordinate is normalized by \(\xi/\rho\to1\) at infinity and obeys
\[
\frac{d\log\xi}{d\rho}=\frac{1}{\rho\sqrt{F(\rho)}}.
\]
The horizon is at \(\rho_H=1\), while its isotropic coordinate \(\xi_H\) is determined by an integral over the complete exterior.

As \(\lambda\) grows, \(\xi_H\) becomes exponentially small. This is not a shrinking physical horizon: \(R_H/r_H=1\) by construction. It is compression of the isotropic coordinate chart, with direct consequences for uniform-\(x\) mesh spacing, interpolation scales, and floating-point representation.

## 2. Matched-asymptotic derivation

Write
\[
I(\lambda)=-\log\xi_H
=\int_0^1\frac{F(y,\lambda)^{-1/2}-1}{y}\,dy,
\]
where
\[
F(y,\lambda)=\frac{2(1-y^2)}
{1+\lambda y^2+\sqrt{1+\lambda(2+\lambda)y^4}}.
\]

There are two overlapping regions.

**Outer region.** For fixed \(y>0\) and large \(\lambda\),
\[
F(y,\lambda)^{-1/2}
\sim \frac{\sqrt\lambda\,y}{\sqrt{1-y^2}},
\]
so the leading outer contribution grows as
\[
\sqrt\lambda\int_0^1\frac{dy}{\sqrt{1-y^2}}
=\frac{\pi}{2}\sqrt\lambda.
\]
This approximation is nonuniform near \(y=0\), so it cannot determine the logarithmic term or the constant by itself.

**Inner region.** Set \(t=\sqrt\lambda\,y\). The leading inner lapse and its inverse square root are
\[
F_{\rm in}(t)=\frac{2}{1+t^2+\sqrt{1+t^4}},\qquad
A(t)=F_{\rm in}(t)^{-1/2}
=\sqrt{\frac{1+t^2+\sqrt{1+t^4}}{2}}.
\]
The finite matching constant is
\[
C_{\rm in}=\lim_{T\to\infty}
\left[
\int_0^T\frac{A(t)-1}{t}\,dt-T+\log T
\right].
\]
Using \(u=A(t)\), with
\[
t^2=\frac{2u^2(u^2-1)}{2u^2-1},
\]
the integral reduces to the closed form
\[
C_{\rm in}=
-1-\frac{2+\sqrt2}{4}\log\left(1+\frac1{\sqrt2}\right)
-\frac{2-\sqrt2}{4}\log\left(1-\frac1{\sqrt2}\right)
+\log2.
\]
Numerically,
\[
C_{\rm in}\approx-0.5835044693003125492681718980633031505374,
\]
and \(A_{\rm in}=e^{-C_{\rm in}}\approx1.79230852786415858056766956132731624856\).

Matching the two regions gives
\[
-\log\xi_H=
\frac{\pi}{2}\sqrt\lambda-\frac12\log\lambda+C_{\rm in}+o(1),
\]
hence
\[
\boxed{
\xi_H\sim A_{\rm in}\sqrt\lambda
e^{-\pi\sqrt\lambda/2},
\qquad
A_{\rm in}\approx1.7923085278641586.
}
\]

## 3. Numerical asymptotic check

The companion script horizon_coordinate_asymptotic.py imports the static BD gauge-map reference, evaluates the horizon integral at 90 decimal working precision, and compares the exact numerical \(\xi_H\) against the leading asymptotic form.

| \(\lambda\) | Exact/asymptotic ratio | Relative asymptotic error |
|---:|---:|---:|
| 100 | 0.9642221215580462 | \(3.5778\times10^{-2}\) |
| 1,000 | 0.9879237356752801 | \(1.2076\times10^{-2}\) |
| 10,000 | 0.9961069367933008 | \(3.8931\times10^{-3}\) |
| 100,000 | 0.9987615603164910 | \(1.2384\times10^{-3}\) |
| 1,000,000 | 0.9996076390176518 | \(3.9236\times10^{-4}\) |

The ratio approaches one monotonically on this frozen grid. The script fails closed if the horizon coordinate is nonpositive, the ratio leaves its declared sanity range, the relative error is not decreasing on the grid, or the error exceeds 5%.

The machine-readable values and execution metadata are in horizon_coordinate_asymptotic_v1.json. Reproduce with:

    python3 horizon_coordinate_asymptotic.py --dps 90 --output horizon_coordinate_asymptotic_v1.json

These are high-precision numerical checks of the asymptotic formula, not a rigorous bound on the \(o(1)\) remainder.

## 4. Solver-design consequence: use a horizon-relative logarithmic chart

The natural chart variable is
\[
\eta=\log(\xi/\xi_H).
\]
It places the horizon at \(\eta=0\), regardless of how small \(\xi_H\) becomes, and
\[
\frac{d\eta}{d\rho}=\frac{1}{\rho\sqrt{F(\rho)}}.
\]
For numerical work one may compactify this coordinate, for example
\[
z=\frac{\eta}{1+\eta}\in[0,1),
\]
after checking the map and its endpoint asymptotics. The logarithmic chart should coexist with the areal chart; it must not replace it or be silently treated as an invariant radius.

Recommended atlas:
- areal \(\rho\) chart for geometric horizon radius and standard asymptotic matching;
- isotropic \(\xi\) chart for reproducing the 2010 isotropic formulation at moderate coupling;
- horizon-relative \(\eta\) chart for large-\(\lambda\) coordinate conditioning;
- horizon-regular charts for rotating/near-extremal calculations where static isotropic coordinates may become unsuitable.

## 5. Qualification gates still required

1. Derive the exact Jacobian and endpoint transformations in the logarithmic chart for generic profiles, not only static BD.
2. Add outward-rounded/Arb enclosures for the map derivative, inverse map and compactification Jacobian.
3. Derive the isotropic EGB equations independently with a radial lapse varied before setting it to unity; see #6202.
4. Compare independent residuals and invariant observables across areal, isotropic and horizon-relative charts on the same candidates.
5. Test the chart transition at matched \((r_H,\alpha,\Omega_H)\), including the nonlinear branch when it exists.

## Claim ceiling

The asymptotic coefficient and its numerical convergence are a research result for the static BD isotropic horizon coordinate. They do not establish a new EGB black-hole branch, improve the field-equation residual, prove existence, or imply a physical horizon singularity.