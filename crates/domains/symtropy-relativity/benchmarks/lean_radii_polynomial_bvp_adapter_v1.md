# Lean radii-polynomial adapter for the static Bach boundary-value problem

**Status:** design and exact theorem-arithmetic fixture only. No Lean BVP theorem, verified operator bound, or black-hole existence result is claimed.
**Owner:** [BH-LEAN-BVP-CERT-001 #6019](https://github.com/Luminous-Dynamics/symtropy/issues/6019).
**Related:** [BH-SPECTRAL-VALIDATION-001 #5992](https://github.com/Luminous-Dynamics/symtropy/issues/5992), [Bach evaluator #5937](https://github.com/Luminous-Dynamics/symtropy/issues/5937), [Taylor jets #5965](https://github.com/Luminous-Dynamics/symtropy/issues/5965), [chart contract #4073](https://github.com/Luminous-Dynamics/symtropy/issues/4073), [implementation PR #5968](https://github.com/Luminous-Dynamics/symtropy/pull/5968).

## 1. Reuse a formal theorem instead of rebuilding one

Wang, *A Lean 4 Framework for the Radii Polynomial Method*, arXiv:2610.02335 (1 October 2026), supplies a reusable theorem for a nonlinear map from a Banach domain X to a normed codomain Y, a bounded approximate inverse A:Y→X, and an approximate derivative A†:X→Y.

Pinned upstream source:
- Repository: https://github.com/IlPreteRosso/RadiiPolynomial
- Source revision: 445773960f43
- Lean toolchain: leanprover/lean4:v4.34.1
- Mathlib: v4.34.1, source revision d13f23b7
- LeanCert: source revision 7f91b6eb3567437f6cfac03ed279706603ee22f4
- Upstream files inspected: Core/Theorem.lean, Applications/IVP/Chebyshev, Certification/IVP/Chebyshev/Face.lean, Applications/Galerkin/Face.lean, lakefile.toml, lean-toolchain, ARCHITECTURE.md.

The upstream general radii polynomial is

\[
p(r)=Z_2(r)r^2-(1-Z_0-Z_1)r+Y_0.
\]

Its four bounds have distinct meanings:

\[
\begin{aligned}
Y_0 &\geq \|A F(\bar x)\|,\\
Z_0 &\geq \|I-A A^\dagger\|,\\
Z_1 &\geq \|A(A^\dagger-DF(\bar x))\|,\\
Z_2(r)r &\geq \sup_{x\in\overline B_r(\bar x)}
\|A(DF(x)-DF(\bar x))\|.
\end{aligned}
\]

With r>0, A injective, and p(r)<0, the theorem supplies a zero in the declared ball and local uniqueness under its stated hypotheses. It does not say anything about a different function space, boundary condition, chart, or equation until those are encoded in F and the realization theorem.

The upstream source already has exact-rational finite certificate faces for Chebyshev IVPs and polynomial/Galerkin maps. It does not provide a black-hole boundary-value problem, its gauge/constraint map, or its fourth-order tail estimates. Those are the new work.

### Proof-kernel boundary

Prefer exact-rational obligations discharged by Lean's kernel (for example decide +kernel) where the certificate is finite. If LeanCert native interval evaluation is used, preserve its axiom report and the exact theorem dependency chain. Every result must include #print axioms output. A Lean proof of rational certificate arithmetic is not, by itself, a proof that the input bounds Y0/Z0/Z1/Z2 are sound.

## 2. First domain: compact static exterior annulus

Begin with the static spherical metric

\[
ds^2=-B(r)dt^2+B(r)^{-1}dr^2+r^2d\Omega^2
\]

on a finite annulus

\[
r\in[r_a,r_b],\qquad r_a>r_h,\qquad B(r)>0.
\]

The first frozen control uses r_a=3, r_b=6 and the Mannheim–Kazanas coefficients

\[
B(r)=-\frac{17}{10r}+\frac7{10}+\frac r{10}-\frac{r^2}{100}.
\]

On this annulus B(3)=103/300 and B(6)=197/300. Its derivative is positive throughout the interval (B' is decreasing and B'(6)=49/1800>0), so the static chart is valid throughout the declared domain. This is a compact exterior-domain test only; it does not establish regularity at the horizon.

The four-term constraint is

\[
Q=c_0^2-3c_1c_{-1}-1.
\]

For the stated coefficients Q=0. The independent full-component static reference in PR #5968 yields

\[
B_{tt}=\frac{QB}{3r^4},\quad
B_{rr}=-\frac{Q}{3r^4B},\quad
B_{\theta\theta}=\frac{Q}{3r^2},\quad
B_{\phi\phi}=\frac{Q\sin^2\theta}{3r^2}.
\]

The off-shell mutation c0=11/10 gives Q=18/25 and must be rejected by a residual enclosure that excludes zero.

## 3. Define the correct BVP before choosing a spectral norm

Let X be a coefficient Banach space for the unknown B(r), and let Y include the field-equation residual plus every boundary and normalization constraint.

The map must be square after gauge/parameter handling:

\[
F:X\longrightarrow Y.
\]

Do not simply feed the fourth-order ODE into a root theorem and assume the derivative is invertible. The exact solution family has free parameters; coordinate/time normalization and physical branch parameters may create kernel directions. First derive the independent static field equation and its differential identities from the full Bach tensor. Then either:
- fix the physical parameters and impose a complete, mathematically justified boundary/gauge set; or
- augment the unknown with continuation parameters and add a phase/normalization condition for each free direction.

Do not impose boundary data copied from the known exact solution and then mistake the identity representation for discovery. The exact solution is the known-answer plumbing control. A meaningful certificate must use a separately generated spectral candidate and a verifier that reconstructs the residual and bounds independently.

### The fourth-derivative norm problem

A naive Chebyshev weighted coefficient norm on B does not automatically control a fourth derivative. Differentiation amplifies high modes; on an unscaled C^0-like coefficient space, the fourth-derivative map is not bounded by the same norm.

Choose one of these routes and prove its mapping properties:
1. A graph norm that includes the coefficient norms of derivatives up to order four.
2. An integrated formulation with explicit integration constants and boundary constraints, so the nonlinear map acts on a space where the residual operator is bounded.
3. A preconditioned formulation whose Green/operator maps and endpoint terms have explicit norm bounds.

The BVP certificate must state X, Y, the norm, boundary traces, gauge conditions, and the operator domains. The exact function being analytic on the annulus is not enough to establish a bounded inverse of the linearized BVP.

## 4. An analytic Chebyshev tail bound for the inverse-radius term

This is the first non-grid truncation control for the known Mannheim–Kazanas branch.

Map [a,b]=[3,6] to [-1,1] by

\[
r=c+dx,\qquad c=\frac92,\quad d=\frac32,\quad x\in[-1,1].
\]

Let

\[
\kappa=\frac cd=3,\qquad
q=\kappa-\sqrt{\kappa^2-1}=3-2\sqrt2.
\]

For the standard one-sided Chebyshev convention

\[
f(x)=a_0+\sum_{n=1}^{\infty}a_nT_n(x),
\]

the reciprocal-radius term has the exact expansion

\[
\frac1r=\frac{1}{d\sqrt{\kappa^2-1}}
\left(1+2\sum_{n=1}^{\infty}(-q)^nT_n(x)\right).
\]

Therefore, for c_{-1}=-17/10,

\[
a_0=-\frac{17}{30\sqrt2},\qquad
a_n=-\frac{17}{15\sqrt2}(-q)^n\quad(n\ge1).
\]

The remaining Mannheim–Kazanas terms c0+c1 r+c2 r² are a polynomial of degree two in x. Thus they contribute no Chebyshev tail once N≥2.

For the one-sided weighted coefficient norm

\[
\|a\|_{\ell^1_\nu}=|a_0|+\sum_{n\ge1}|a_n|\nu^n,
\]

the exact tail bound after degree N is

\[
\boxed{
\sum_{n>N}|a_n|\nu^n
\leq
\frac{17}{15\sqrt2}
\frac{(q\nu)^{N+1}}{1-q\nu}
}
\]

provided qν<1. This is an analytic geometric-series bound, not a sampled-grid estimate.

For ν=2, qν=6-4√2≈0.3431457505, so the condition holds. The reference fixture contains high-precision evaluations at N=4,8,12,16,24,32.

### Exact conversion to the pinned upstream bilateral Chebyshev representation

The pinned upstream sources were inspected directly:
- `Analysis/SequenceSpace/Chebyshev/Scalar.lean` defines the bilateral weight as `nu^|k|`.
- `Analysis/SequenceSpace/Chebyshev/Symmetrize.lean` defines symmetric storage by copying the nonnegative coefficient to both signs.
- `Analysis/SequenceSpace/Chebyshev/OneSided.lean` identifies nonnegative-mode restriction with the one-sided weighted coefficient norm.
- The physical basis is the Joukowski/Laurent identity
  [
  T_n(x)=\frac{z^n+z^{-n}}2,qquad x=\frac{z+z^{-1}}2.
  ]

Therefore a one-sided physical Chebyshev series
[
f(x)=a_0+\sum_{n\ge1}a_nT_n(x)
]
is represented in the symmetric bilateral Laurent carrier by
[
c_0=a_0,\qquad c_n=c_{-n}=\frac{a_n}{2}\quad(n\ge1).
]
The bilateral weighted norm is exactly
[
|c_0|+\sum_{n\ge1}(|c_n|+|c_{-n}|)\nu^n
=|a_0|+\sum_{n\ge1}|a_n|\nu^n.
]
Thus the one-sided tail bound above transfers to the upstream symmetric bilateral norm with **no additional factor**. The factor 1/2 belongs in each of the positive and negative stored coefficients; it is canceled by the two equal weighted contributions.

The conversion and norm equality are an elementary coefficient calculation. They have not yet been formalized in the pinned Lean project or executed there. The future BVP adapter must add those Lean lemmas (or use an existing exact equivalent) before feeding the tail bound into an upstream certificate.

This analytic tail is only one input to a complete BVP proof. It does not by itself bound the nonlinear field residual, the boundary data, the inverse defect, or the spectral error after applying fourth derivatives.

## 5. Radii-polynomial arithmetic plumbing fixture

The separate rational fixture uses

\[
Y_0=\frac1{1000},\quad Z_0=\frac1{100},\quad
Z_1=\frac1{100},\quad Z_2=\frac1{10},\quad r_0=\frac1{100}.
\]

Then

\[
p(r_0)=\frac1{10}\frac1{10000}
-\left(1-\frac1{100}-\frac1{100}\right)\frac1{100}
+\frac1{1000}
=-\frac{879}{100000}<0.
\]

The combined contraction bound is

\[
Z_0+Z_1+Z_2r_0=\frac{21}{1000}<1.
\]

This is deliberately labelled theorem-arithmetic only. These rational values are not derived from a black-hole candidate, a verified inverse, a field residual, or an actual BVP operator. They test that the certificate schema and the Lean inequality adapter preserve exact rational arithmetic; they do not certify a physical or mathematical black-hole solution.

## 6. Implementation sequence

### P0 — source and theorem pin
- Import the pinned RadiiPolynomial revision and Lean toolchain.
- Preserve lake-manifest and source digests.
- Reproduce the standalone rational radii-polynomial fixture with Python Fraction and Lean kernel evaluation.
- Record #print axioms.

### P1 — known-answer static coefficient fixture
- Pin the annulus, Chebyshev convention, metric units and exact MK coefficients.
- Reproduce the inverse-radius series and analytic tail.
- Prove the one-sided-to-bilateral coefficient conversion used by the selected upstream space.
- Compare exact function evaluation and truncated series over the whole annulus using the analytic tail, not a maximum sampled error.

### P2 — bounded BVP operator
- Derive the independent fourth-order ODE from the full static Bach tensor and document why it is equivalent to the independent component equations on the declared domain.
- Construct the graph-norm or integral/preconditioned map.
- Add and prove the boundary/gauge constraints.
- Prove boundedness and derivative variation of F on the selected ball.

### P3 — independent finite-head/tail estimates
- Compute finite-head matrices and candidate inverse in a discovery environment.
- Reconstruct the matrix, residual and tail bounds independently.
- Enclose rounding, coefficient input uncertainty and all omitted modes.
- Prove Y0, Z0, Z1 and Z2 with exact rational / ball evidence.
- Apply the upstream Lean theorem to obtain local existence and uniqueness on the specified ball.

### P4 — continuation and rotating systems
- Continue overlapping local existence balls along a parameterized static branch.
- Only after successful static proof, formulate a two-dimensional stationary axisymmetric PDE with horizon/axis-regular charts, gauge and constraints.
- Do not interpret a finite chain of local balls as global branch coverage.

## 7. Qualification gates and failure controls

- Exact MK function is contained by the Chebyshev tail enclosure.
- The c0=11/10 off-shell perturbation has a Bach residual enclosure excluding zero.
- A boundary-condition mutation invalidates the certificate.
- Missing gauge/phase constraints expose an operator kernel rather than passing invertibility.
- Underestimated high-mode tail or fourth-derivative amplification fails the enclosure.
- A mismatched one-sided/bilateral Chebyshev normalization is detected by exact coefficient fixtures.
- Wrong Bach sign/action normalization fails the component-level static oracle.
- Changing the annulus/chart/unit convention rejects the certificate.
- A bad approximate inverse fails Z0/Z1.
- An interval enclosure too wide or ill-conditioned returns Inconclusive, not success or nonexistence.

## 8. Claim ceiling

The rational fixture validates only exact theorem arithmetic. The analytic Chebyshev tail validates only one known function component on one annulus. A future successful Lean BVP certificate would establish a local solution for the explicitly stated static equation, function space, chart, annulus, boundary conditions and parameter ball. It would not establish horizon regularity, global uniqueness, rotating solutions, dynamical stability or observational viability.

## References
- Wang, *A Lean 4 Framework for the Radii Polynomial Method*, arXiv:2610.02335 (2026): https://arxiv.org/abs/2610.02335
- Pinned Lean source: https://github.com/IlPreteRosso/RadiiPolynomial/tree/445773960f43
- Hungria, Lessard & Mireles James, *Rigorous numerics for analytic solutions of differential equations: the radii polynomial approach*, Math. Comp. 85 (2016), 1427–1459, DOI 10.1090/mcom/3046.
- Symtropy #5937, #5965, #5992, #6019 and PR #5968.
