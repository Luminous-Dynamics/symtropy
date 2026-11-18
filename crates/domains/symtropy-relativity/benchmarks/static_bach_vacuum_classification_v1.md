# Static spherical Bach-vacuum classification v1

**Status:** exact symbolic reduction independently checked; formal Lean classification not yet written or executed.  
**Owner:** BH-STATIC-CLASSIFICATION-001 (#6044).  
**Related:** [Bach reference PR #5968](https://github.com/Luminous-Dynamics/symtropy/pull/5968), [general Bach #5937](https://github.com/Luminous-Dynamics/symtropy/issues/5937), [formal BVP #6019](https://github.com/Luminous-Dynamics/symtropy/issues/6019), [SOTA benchmark #6043](https://github.com/Luminous-Dynamics/symtropy/issues/6043).

## 1. Result

For a static, spherically symmetric metric in areal-radius gauge,
\[
ds^2=-B(r)dt^2+B(r)^{-1}dr^2+r^2d\Omega^2,
\]
on a connected interval \(I\subset(0,\infty)\) where \(B(r)>0\), define
\[
A=\frac{1-B}{r^2}+\frac12B'',\qquad
E=\frac12\left(\frac{1-B}{r^2}+\frac{B'}r-\frac12B''\right),
\qquad X=A'+\frac Ar.
\]

The independent full-tensor calculation gives the two reduced projections
\[
\begin{aligned}
\mathcal B_+&=\frac{B_{tt}}B+B B_{rr}
=-\frac{2B}{3}\left(X'+\frac{3X}{r}\right),\\
\mathcal B_-&=\frac{4B_{\theta\theta}}{r^2}
=-\frac83AE-\frac43X\left(B'+\frac Br\right)-\frac43BX'.
\end{aligned}
\]

The first vacuum equation \(\mathcal B_+=0\), together with \(B>0\), implies
\[
X'+\frac{3X}{r}=0,
\quad
(r^3X)'=0,
\quad
X=\frac C{r^3}.
\]
Since \(X=(rA)'/r\), integration gives
\[
(rA)'=\frac C{r^2},\qquad
A=\frac D r-\frac C{r^2}.
\]
Substituting the definition of A yields an Euler equation
\[
r^2B''-2B=2Dr-2C-2.
\]
Its complete solution on I is
\[
\boxed{B(r)=\frac{c_{-1}}r+c_0+c_1r+c_2r^2}
\]
with \(c_0=C+1\), \(c_1=-D\), and \(c_{-1},c_2\) free integration constants.

Substituting this general solution into the second independent equation gives
\[
\boxed{\mathcal B_-=\frac{4}{3r^4}
\left(c_0^2-3c_1c_{-1}-1\right).}
\]
Therefore the full static spherical vacuum family on the stated domain is exactly
\[
B(r)=c_{-1}/r+c_0+c_1r+c_2r^2,\qquad
\boxed{c_0^2-3c_1c_{-1}=1.}
\]

Conversely, the independent full-component Bach tensor for this ansatz is
\[
B_{tt}=\frac{QB}{3r^4},\quad
B_{rr}=-\frac{Q}{3r^4B},\quad
B_{\theta\theta}=\frac{Q}{3r^2},\quad
B_{\phi\phi}=\frac{Q\sin^2\theta}{3r^2},
\quad Q=c_0^2-3c_1c_{-1}-1.
\]
Thus Q=0 makes every component vanish. This converse is checked symbolically in the independent full-tensor oracle.

## 2. Why this matters

This is a better first formal target than numerically rediscovering the static vacuum family:

- It identifies the exact solution manifold and its free parameters.
- It exposes the algebraic constraint that a one-function residual benchmark must test.
- It explains why an unaugmented static vacuum root is not isolated: there is a continuous family of exact solutions.
- It prevents an invalid radii-polynomial setup whose linearized operator silently has physical/gauge zero modes.
- It gives exact known-answer controls for coefficient, derivative, boundary and interval tests.
- It clarifies where numerical discovery is genuinely needed: source terms, matter couplings, non-static matter perturbations, other higher-curvature actions, and rotating stationary PDEs.

The static vacuum classification is not a claim about rotating solutions or general higher-curvature actions.

## 3. Assumptions and proof obligations

The reduction requires:
1. \(r>0\) on a connected interval.
2. \(B(r)>0\) throughout the interval, so the chosen static chart is valid and division by B in \(\mathcal B_+\) is legitimate.
3. Sufficient regularity to define the fourth-order Bach equations and integrate the resulting ODE (at least \(C^4\) for the classical formulation).
4. The metric is in the stated areal-radius gauge and the action/equation convention matches the pinned Bach tensor convention.
5. The two reduced projections are exactly equivalent to the independent full tensor equations using the four-dimensional Bach trace identity and spherical symmetry. This equivalence must be formalized; it is not inferred solely from an ansatz.

The symbolic calculation proves the reduction identities by direct substitution and simplification. A Lean classification theorem still needs to establish the differential integrations and the equivalence between the reduced projections and all covariant components.

## 4. Exact parameter manifold and branch normalization

The coefficients satisfy one scalar constraint in four real coefficients, so the vacuum family is generically three-dimensional. A local uniqueness theorem must therefore either fix three independent parameters/normalizations or augment the unknowns with those parameters and phase conditions. Do not apply a radii polynomial to the unaugmented map and assume its derivative is invertible.

The conventional Mannheim–Kazanas parameterization
\[
c_0=1-3\beta\gamma,\quad
c_1=\gamma,\quad
c_{-1}=-\beta(2-3\beta\gamma),\quad
c_2=-\kappa
\]
satisfies Q=0 identically. It is a useful physical branch parameterization, but the coefficient form with Q=0 is the more general algebraic statement and should remain the classification theorem's primary representation.

## 5. Exact regression controls

- Schwarzschild: \((c_{-1},c_0,c_1,c_2)=(-2,1,0,0)\), Q=0.
- Non-Einstein MK: \((-17/10,7/10,1/10,-1/100)\), Q=0.
- Off-shell mutation: \((-17/10,11/10,1/10,-1/100)\), Q=18/25.

For the off-shell mutation,
\[
\mathcal B_-(3)=8/675,\quad
\mathcal B_-(6)=1/1350,
\]
so the ratio is exactly 16, as expected from the \(r^{-4}\) scaling.

## 6. New research workflow

### P0 — formalize the reduction
- Prove the algebraic identity for \(\mathcal B_+\).
- Prove \(\mathcal B_+=0\) and \(B>0\) imply \((r^3X)'=0\).
- Integrate the first-order identities to obtain A and then B.
- Prove the Euler ODE's complete solution form on a connected interval.
- Substitute the solution into \(\mathcal B_-\) and prove the exact Q constraint.
- Connect the reduced equations to the full covariant Bach tensor using trace-freeness and the static-spherical diagonal structure.
- Preserve signature, Riemann convention, action normalization, chart and domain in the theorem statement.

### P1 — formalize branch identity and uniqueness
- Define the coefficient manifold Q=0.
- Prove the map from coefficients to B(r) is injective on any nonempty open interval (or state the appropriate nondegeneracy hypothesis).
- Add explicit parameter/gauge normalization for local uniqueness of a point on this three-dimensional solution manifold.
- Prove the known-answer Schwarzschild and MK coefficient fixtures.

### P2 — move numerical certification to genuinely new problems
Use radii-polynomial/interval methods for:
- static Bach with matter/source terms;
- nonlinear boundary data not analytically integrable;
- perturbations about the exact MK family;
- alternative higher-curvature actions;
- rotating stationary axisymmetric PDEs.

For every such problem, identify kernel/phase modes before constructing the approximate inverse.

## 7. Claim ceiling

The symbolic derivation and exact coefficient controls establish the static spherical Bach reduction and its analytic solution family, subject to the assumptions above. They do not prove the Lean formal classification, horizon regularity, asymptotic physical admissibility, dynamical stability, rotating solutions, EFT validity or observational viability.


## 8. Exact static-annulus and coefficient arithmetic in Lean

The branch now includes `benchmarks/lean/static_bach_vacuum_coefficients_v1.lean`. It contains kernel-checkable rational/algebraic controls for:
- Schwarzschild Q=0;
- non-Einstein MK Q=0;
- the off-shell Q=18/25 mutation and exact residuals 8/675 at r=3 and 1/1350 at r=6;
- the factor-16 residual ratio;
- the conventional beta/gamma coefficient parameterization satisfying Q=0 identically;
- B(3)=103/300 and B(6)=197/300;
- a direct factorization proving B(r)>0 for every r in [3,6].

The positivity proof uses
[
100r B(r)=103+(r-3)(-r^2+7r+91),
]
whose factors are positive/nonnegative on the declared annulus. This confirms the entire MK reference annulus lies in the static chart, not merely the two sampled endpoints.

This Lean file is authored and has not been compiled. The full differential integration, equivalence of the reduced projections to all covariant components, and the complete classification theorem remain in #6048.
