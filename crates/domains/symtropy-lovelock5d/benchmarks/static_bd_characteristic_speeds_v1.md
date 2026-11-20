# Exact static 5D EGB characteristic speeds and hyperbolicity boundary

**Owner:** [#6334 BH-5D-EGB-HYPERBOLICITY-001](https://github.com/Luminous-Dynamics/symtropy/issues/6334)

**Status:** exact symbolic specialization of the published static spherical Lovelock effective metrics to the Symtropy 5D Boulware–Deser convention. This is not a rotating-background result.

## 1. Match the coupling convention

The Symtropy static solution is
\[
F(\rho)=1-\frac{2+\lambda}{\rho^2(1+s)},\qquad
s^2=1+\frac{\lambda(\lambda+2)}{\rho^4},
\]
with \(\rho=r/r_H\), \(\lambda=\alpha/r_H^2\).

Reall, Tanahashi and Way write the asymptotically flat EGB branch as
\[
F=1+\frac{\rho^2(1-q)}{\widetilde\alpha_2},\qquad
q^2=1+\frac{2\widetilde\alpha_2\mu}{\rho^4}.
\]
The exact convention match is
\[
\widetilde\alpha_2=\lambda,\qquad
\mu/r_H^2=(2+\lambda)/2,\qquad q=s.
\]
Rationalization gives exact equality of the two lapse expressions. This pins the factor-of-two convention rather than importing a threshold from an unmatched definition of the Gauss–Bonnet coupling.

## 2. Published effective metrics

For static, maximally symmetric Lovelock black holes, the physical tensor, vector and scalar characteristic sectors have effective inverse metrics
\[
G_A^{IJ}=\eta^{IJ},\qquad G_A^{ij}=c_A(\rho)\delta^{ij},
\]
where \(A\in\{T,V,S\}\). A sector's effective metric is Lorentzian when \(c_A>0\), degenerate when \(c_A=0\), and has non-Lorentzian signature when \(c_A<0\).

Specializing Reall–Tanahashi–Way Eqs. (5.13) and (5.17) to \(d=5\), \(\Lambda=0\), and \(\kappa=1\) gives
\[
A=1/q^2=1/s^2,\qquad B=0,
\]
and therefore
\[
\boxed{c_T=3-\frac{2}{s^2}=\frac{1+3z}{1+z}},
\]
\[
\boxed{c_V=\frac{1}{s^2}=\frac{1}{1+z}},
\]
\[
\boxed{c_S=\frac{2}{s^2}-1=\frac{1-z}{1+z}},
\qquad z=\frac{\lambda(\lambda+2)}{\rho^4}.
\]
All three tend to one at infinity and recover the GR metric null cone at \(\lambda=0\).

## 3. Exact scalar-sector boundary

For \(\lambda\ge0\), \(z\) decreases monotonically with radius, so its maximum on the exterior closure is at the horizon:
\[
z_H=\lambda(\lambda+2).
\]
The tensor and vector speeds are strictly positive. The scalar speed changes sign when \(z=1\).

At the horizon,
\[
c_S(1)=\frac{1-\lambda(\lambda+2)}{1+\lambda(\lambda+2)}
=\frac{2}{(1+\lambda)^2}-1.
\]
The exact critical coupling is
\[
\boxed{\lambda_{\rm crit}=\sqrt2-1
=0.414213562373095\ldots}
\]
and the scalar characteristic degeneracy radius is
\[
\boxed{\rho_c=[\lambda(\lambda+2)]^{1/4}.}
\]

Consequences:
- If \(0\le\lambda<\sqrt2-1\), all three static spherical effective characteristic metrics are Lorentzian throughout the open exterior.
- At \(\lambda=\sqrt2-1\), the scalar speed reaches zero only in the horizon limit; it is positive for every \(\rho>1\).
- If \(\lambda>\sqrt2-1\), then \(\rho_c>1\) and \(c_S<0\) for \(1<\rho<\rho_c\). The scalar effective metric is non-Lorentzian in a finite exterior shell. The static background therefore fails the Lovelock characteristic hyperbolicity test there.

This is not a curvature singularity. The stationary BD metric and its scalar curvature invariants can remain finite while the scalar gravitational characteristic metric changes signature.

## 4. Why this changes the programme

The Akribeia-matched static coupling interval extends to \(\lambda=3\), well beyond \(\lambda_{\rm crit}\). The existence or numerical convergence of a stationary metric in that interval must not be conflated with a well-posed gravitational initial-value problem.

The static threshold is a control and a warning, not a rotating threshold. At nonzero equal spin, the static scalar/vector/tensor decomposition may mix. We must recompute the physical principal symbol on:
1. exact Myers–Perry (\(\lambda=0\));
2. finite-\(\lambda\) static BD;
3. finite-\(\lambda\), linear slow rotation;
4. the qualified spin-squared background;
5. nonlinear equal-spin EGB candidates.

For rotating backgrounds use a horizon-regular invariant frame and resolve degenerate polarizations before assessing characteristic roots. Do not apply the static \(c_T,c_V,c_S\) expressions unchanged.

## 5. Reproduction

~~~sh
python3 derive_static_bd_characteristic_speeds.py \
  --output static_bd_characteristic_speeds_v1.json
~~~

The script symbolically matches the lapse conventions, specializes the published \(A,B,c_T,c_V,c_S\) expressions, verifies the three simplified speeds, checks their GR limit, and derives the exact critical coupling and radius.

## 6. Claim ceiling

This is an exact static spherical characteristic-sector result. It establishes the scalar effective-metric signature change for the static 5D BD branch. It does not establish strong hyperbolicity of a chosen gauge-fixed evolution formulation, rotating-background characteristics, mode stability, EFT validity, or nonlinear branch behavior.

## References

- Reall, Tanahashi & Way, *Causality and hyperbolicity of Lovelock theories*, CQG 31 (2014) 205005, especially Sec. 5, Eqs. (5.13)–(5.17): https://arxiv.org/abs/1406.3379
- Brihaye, Kleihaus, Kunz & Radu, *Rotating black holes in five-dimensional Einstein–Gauss–Bonnet gravity*, arXiv:1010.0860: https://arxiv.org/abs/1010.0860


## Semantics of \(c_A\)

The functions \(c_T,c_V,c_S\) are the angular coefficients in the effective inverse metrics / characteristic quadratic forms. They are not direction-independent coordinate velocities. In the static spherical geometry, the effective covariant metrics can be written (up to an irrelevant positive conformal factor) as
\[
G_{A\mu\nu}dx^\mu dx^\nu=-Fdt^2+F^{-1}dr^2+\frac{r^2}{c_A(r)}d\Omega_3^2.
\]
All physical sectors share the metric radial null directions; their angular characteristic cones differ. Positivity of every \(c_A\) is the static sector signature/hyperbolicity test used here. This still does not prove strong hyperbolicity of a chosen gauge-fixed first-order formulation.
