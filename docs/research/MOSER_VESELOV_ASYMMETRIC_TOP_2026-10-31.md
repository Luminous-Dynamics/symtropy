# Three-dimensional asymmetric-top reference: discrete Moser-Veselov

**Status:** standalone research reference and falsifiable benchmark proposal; not a production integrator or an SOTA claim.  
**Date:** 2026-10-31  
**Qualification corpus:** [PHYS-VALID-06, issue #460](https://github.com/Luminous-Dynamics/symtropy/issues/460)  
**Related implementation:** [#3012 mass-property/contact qualification](https://github.com/Luminous-Dynamics/symtropy/pull/3012)  
**Related research:** [#3119 momentum-map preservation](https://github.com/Luminous-Dynamics/symtropy/issues/3119), [#3120 constrained dumbbell](https://github.com/Luminous-Dynamics/symtropy/pull/3120)

## Why this is the next experiment

Symtropy has developed asymmetric rotational inertia, mass-property consistency checks, rollback on invalid inertia, a Delassus/LCP reference, and a planar maximum-dissipation friction oracle. The current pHDAE symmetry line has qualified a planar constrained dumbbell and names a 3D rigid tetrahedron or asymmetric top as the next stage.

The missing bridge is an independent structure-preserving reference for the actual three-dimensional free asymmetric rigid body. A body can conserve energy while corrupting its momentum map, or preserve a rotation matrix while drifting in energy. Those observables must be tested separately.

This reference implements the discrete Moser-Veselov (DMV) map for the free asymmetric top. Moser-Veselov is an established integrable discretization of free rigid-body dynamics. Hairer and Vilmart describe a symplectic, time-reversible algorithm that preserves the first integrals; their preprocessed extension raises its order. This implementation uses the base DMV map only, not higher-order preprocessing.

Reference: E. Hairer and G. Vilmart, “Preprocessed discrete Moser-Veselov algorithm for the full dynamics of a rigid body,” Journal of Physics A: Mathematical and General 39 (2006), 13225–13235. DOI: https://doi.org/10.1088/0305-4470/39/42/003.

The paper's body-frame equations use the conjugation update for body momentum, the transpose group update for attitude, and the implicit orthogonal increment. The contribution here is not a new numerical method attributed to Symtropy; it is an independent Rust oracle, explicit failure semantics, metamorphic tests, and benchmark fixtures tied to the asymmetric-inertia programme.

## Mathematical contract

Let \(I=\operatorname{diag}(I_1,I_2,I_3)\) be the physical principal inertia tensor, \(\Pi\) body angular momentum, and \(Q\in SO(3)\) the body-to-world attitude.

The continuous equations, under this implementation's conventions, are:

\[
\dot \Pi = \Pi \times I^{-1}\Pi,\qquad
\dot Q = Q\widehat{I^{-1}\Pi}.
\]

The invariants are:

\[
H(\Pi)=\frac12\Pi^TI^{-1}\Pi,\quad
C(\Pi)=\frac12\Pi^T\Pi,\quad
\ell_{\mathrm{world}}=Q\Pi.
\]

Define the Moser-Veselov diagonal matrix:

\[
D=\operatorname{diag}\left(
\frac{I_2+I_3-I_1}{2},
\frac{I_3+I_1-I_2}{2},
\frac{I_1+I_2-I_3}{2}
\right).
\]

Its pairwise sums recover the physical principal moments. All three entries must be strictly positive for a non-degenerate physical principal inertia tensor.

Each discrete step solves for \(\Omega\in SO(3)\):

\[
\Omega^TD-D\Omega=h\widehat{\Pi}_n,
\]

then updates:

\[
\widehat{\Pi}_{n+1}=\Omega\widehat{\Pi}_n\Omega^T,\qquad
Q_{n+1}=Q_n\Omega^T.
\]

The Rust implementation parameterizes \(\Omega=\exp(\widehat\theta)\) and uses Newton with the analytic right-Jacobian of the exponential map and residual-decreasing backtracking. It fails closed on invalid inertia, non-finite inputs, invalid controls, singular Newton systems, failed line search, or non-convergence. It does not project a failed candidate onto SO(3).

## Discrete invariants to audit

1. Energy: \(H(\Pi_{n+1})-H(\Pi_n)\).
2. Casimir: \(C(\Pi_{n+1})-C(\Pi_n)\).
3. Spatial momentum map: \(Q_{n+1}\Pi_{n+1}-Q_n\Pi_n\).
4. Group membership: \(\|Q^TQ-I\|_F\) and \(|\det(Q)-1|\).
5. Implicit equation: normalized residual of \(\Omega^TD-D\Omega-h\widehat\Pi_n=0\).
6. Reversibility: apply \(+h\), then \(-h\), and compare the complete state.
7. Order: compare at \(h,h/2,h/4,h/8\) against an independently implemented high-accuracy RK4 reference.

Energy preservation, Casimir preservation and momentum-map preservation are separate observations. They must not be compressed into a single conservation score.

## Executed implementation

The standalone source is tools/moser-veselov-top/asymmetric_top.rs. It has no third-party dependencies and includes:

- Principal-inertia and physical triangle-inequality validation.
- An SO(3) exponential map with a small-angle series.
- The analytic right-Jacobian needed for Newton's discrete-equation solve.
- A pivoted 3x3 Newton linear solve and residual-decreasing backtracking.
- Explicit typed failure outcomes; no zero-inertia or non-finite fallbacks.
- An independent classical RK4 continuous-time reference.
- Three frozen regimes: asymmetric, near-axisymmetric and intermediate-axis.
- A deterministic CSV campaign at four step sizes, with 1e-4 and 5e-5 RK4 reference-refinement comparison.
- Tests for invariant preservation, reversibility, SO(3) membership, second-order convergence, world-rotation equivariance, dimensional similarity, invalid inertia, invalid input, Newton Jacobian consistency and regime coverage.

The reference has no dependency on the production RigidBody type. That is intentional: it must remain an independent oracle. A subsequent adapter campaign should compare the production BivectorInertia<3> and production torque-free trajectory against this oracle without making the oracle call production integration code.

## Frozen benchmark targets

These are proposed acceptance thresholds, not reported results.

| Observable | Target |
|---|---:|
| Relative energy drift, 2,000-step free-top run | ≤ 2e-11 |
| Relative Casimir drift | ≤ 2e-11 |
| Relative spatial angular-momentum drift | ≤ 2e-11 |
| SO(3) orthogonality and determinant defects | ≤ 2e-11 |
| Discrete equation residual | ≤ 1e-11 |
| Forward/backward state discrepancy | ≤ 2e-11 |
| Observed temporal order | 1.8–2.2 |
| Rotational-equivariance defect | ≤ 2e-11 |
| Similarity-transform defect | ≤ 2e-12 |
| Valid physical cases rejected | 0 |
| Invalid principal inertia silently accepted | 0 |

Thresholds are intended for a pinned double-precision CPU reference. They must be scaled before applying them to reduced precision or GPU implementations.

## Industry comparison and the actual opportunity

Moser-Veselov and Lie-group variational integration are established methods; Symtropy must not claim the algorithm as a new numerical invention. The opportunity is to use an independent structure-preserving reference to qualify the same mass-property model used by the real-time contact solver.

The first matched comparisons should be:
- Symtropy's production torque-free asymmetric-top integration.
- This Moser-Veselov reference.
- Fixed-step RK4 as an accuracy baseline.
- Implicit midpoint or another Lie-group variational baseline where implemented.

Use identical physical inertia, initial momentum, attitude, horizon, double precision, and step-size ladder. Report energy, Casimir, spatial angular momentum, attitude error, body-momentum error, equation residual, Newton iterations, and measured wall time separately. A method that preserves energy but damages the spatial momentum map is not the winner. A method that is more accurate but several times more expensive must be represented on the work-precision frontier.

Only compare against MuJoCo, Rapier, PhysX or Chrono when the same free-top mechanics, inertia, attitude convention and diagnostics can be represented. Engine benchmarks are not automatically a ground truth for this special case.

## Limits and promotion gates

This prototype covers a smooth, torque-free, 3D rigid body with principal-axis inertia. It does not implement:
- External torque, attitude-dependent potential or gyroscopic actuator work.
- Joints, impacts, friction, contact complementarity or CCD.
- Arbitrary non-principal inertia input; the production adapter must diagonalize with provenance.
- An N-dimensional Moser-Veselov implementation.
- Production PhysicsWorld integration or adaptive error control.

Promotion requires:
1. Exact-head Rust workflow passes formatting, warning-denied tests, optimized build and deterministic output replay.
2. Invariant and metamorphic tests pass at the exact submitted source.
3. Production BivectorInertia<3> is compared to this oracle through an independently specified coordinate/frame adapter.
4. The reference is registered in the single engine-wide corpus #460 with source, input, toolchain and configuration identity.
5. A matched work-precision report exists before any performance or superiority claim.

No local Cargo result, external engine comparison, or SOTA advantage is claimed by this research reference.
