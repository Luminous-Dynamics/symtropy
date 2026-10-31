# N-dimensional contact-wrench geometry

**Date:** 2026-10-31
**Status:** research proposal; no 3D/4D torsional or rolling solver implementation claimed.
**Context:** Symtropy's generalized velocity is translational velocity plus a bivector angular state.

## The opportunity

The robotics literature is moving beyond point-contact friction toward 6D contact: tangential friction, torsional friction and rolling friction. ComFree-Sim (2026) explicitly models those three channels using a dual-cone impedance formulation. Symtropy can take a complementary route: express the contact wrench space in exterior algebra so the decomposition is not hard-coded to the 3D cross product.

For a unit contact normal (n\in\mathbb{R}^D), define the tangent space:

\[
T=n^\perp=\{t\in\mathbb{R}^D:n\cdot t=0\}.
\]

The bivector space has an orthogonal decomposition:

\[
\Lambda^2(\mathbb{R}^D)
=
(n\wedge T)\oplus\Lambda^2T.
\]

The two summands have direct contact interpretations:
- (n\wedge T): rolling rotations, i.e. angular motion about tangent directions;
- (\Lambda^2T): torsional/twisting rotations within the tangent plane.

This is the exterior-algebra analogue of the 3D decomposition into two rolling axes and one torsional axis.

## Dimension counts

| Spatial dimension D | Tangential linear friction | Rolling bivectors dim(n∧T) | Torsional bivectors dim(Λ²T) | Total angular contact coordinates |
|---:|---:|---:|---:|---:|
| 2 | 1 | 1 | 0 | 1 |
| 3 | 2 | 2 | 1 | 3 |
| 4 | 3 | 3 | 3 | 6 |
| 5 | 4 | 4 | 6 | 10 |

In 3D, the contact impulse/wrench coordinates comprise one normal, two tangential, two rolling and one torsional components: six coordinates. In 4D the same construction yields ten contact coordinates, with no need for special cross-product code.

## Contact frame construction

Given an orthonormal tangent basis (\{t_i\}_{i=1}^{D-1}):
- linear tangent directions are (t_i);
- rolling bivector directions are (n\wedge t_i);
- torsional bivector directions are (t_i\wedge t_j) for (i<j).

The basis is orthonormal in the induced exterior-product metric. Its size is:

\[
(D-1)+(D-1)+\frac{(D-1)(D-2)}{2}
=
\frac{(D-1)(D+2)}{2}
\]

friction coordinates, plus one normal coordinate. The contact-frame builder must be rotation-covariant and robust for normals aligned with coordinate axes; no arbitrary global axis may be treated as the tangent frame.

## Wrench coordinates and dimensional consistency

Let the generalized body velocity be (u=(v,B)\in\mathbb{R}^D\oplus\Lambda^2(\mathbb{R}^D)). The contact velocity vector is assembled by a Jacobian:

\[
\nu_c=J_c u
=
(v_n,v_t,\omega_{roll},\omega_{tors}).
\]

The generalized contact impulse is the dual vector:

\[
p_c=(\lambda_n,p_t,\tau_{roll},\tau_{tors}).
\]

The friction limits need explicit length scales because angular impulse has units of linear impulse multiplied by length:

\[
\lambda_n\ge0,
\qquad
\|p_t\|\le\mu_t\lambda_n,
\]

\[
\|\tau_{roll}\|\le\mu_r R_{roll}\lambda_n,
\qquad
\|\tau_{tors}\|\le\mu_\tau R_{tors}\lambda_n.
\]

The rolling/torsional characteristic radii (R_{roll}) and (R_{tors}) must be explicit model inputs or derived from declared local curvature/contact-patch geometry. They must not be hidden dimensionless fudge factors. For anisotropic contact patches, use a positive-definite metric or per-principal-direction length scales rather than forcing one scalar radius.

## Shared effective contact metric

With generalized mass/inertia matrix (M), contact Jacobian (J_c), and timestep (h), the friction channels should use the same effective metric as normal contact and integration:

\[
\widehat M=M+hD+h^2K,
\qquad
W_c=J_c\widehat M^{-1}J_c^T.
\]

The matrix must retain all cross-couplings:
- normal ↔ tangential;
- tangential ↔ rolling/torsional;
- rolling ↔ torsional;
- body A ↔ body B through the shared generalized response.

For an asymmetric rigid body, discarding these off-diagonal blocks can change both the impulse and energy response. A first independent reference should assemble the complete small contact matrix directly from generalized impulse response, not hand-code isolated scalar effective masses.

## Competing contact formulations

The same geometric contact frame can support different constitutive/solver models. These should be reported separately:
1. hard Signorini plus maximum-dissipation Coulomb cones;
2. soft convex contact with an explicit compliance/impedance law;
3. polyhedral cone approximation for SIMD/GPU batching;
4. complementarity-free dual-cone impedance for high-throughput contact.

They are not mathematically interchangeable. Every benchmark must declare which model is being tested, its compliance, cone approximation/facet count, and whether strict complementarity or maximum dissipation is expected.

## Implemented geometry/API increment

The branch now contains a first reference implementation in `symtropy-physics::contact_frame`:

- `ContactFrame<D>` builds a deterministic orthonormal tangent basis by reorthogonalized modified Gram-Schmidt with largest-residual pivoting.
- It constructs rolling `n∧T` and torsional `Λ²T` bivector bases, and decomposes/reconstructs arbitrary bivectors.
- `rotated_by` transforms the complete frame. `transported_to` uses the shortest normal-to-normal rotation to preserve local coordinates over time; exact antipodal transitions use a deterministic half-turn axis from the prior tangent basis.
- `ContactWrenchFrictionLimits` records separate tangential, rolling and torsional coefficients with explicit rolling/torsional characteristic lengths. Its fields are private and validated by the constructor and again at use.
- `ContactWrenchImpulse<D>` checks normal non-attraction, tangent orthogonality, dimensions, and each separate friction bound; it can reconstruct the angular impulse bivector from rolling and torsional coordinates.
- `ContactWrenchImpulse<D>::validate_ellipsoidal_friction_budget` exposes a *separate coupled model* with squared normalized tangential, rolling and torsional usage summing to at most one. The original `validate` remains an independent-channel/product-of-bounds model; the API deliberately does not pretend these two constitutive sets are equivalent.
- The coupled budget has explicit zero-bound semantics: a channel with zero coefficient may only carry zero impulse. The regression matrix verifies the independent model accepts three simultaneously saturated channels while the coupled ellipsoid rejects them, and verifies a feasible interior point.
- `ContactWrenchRow<D>` and `assemble_contact_wrench_delassus` now construct a full reference metric with D linear contact coordinates plus D(D-1)/2 angular coordinates. The assembler applies both linear-at-offset and direct angular impulses to each body, measures all normal/tangent/rolling/torsion responses, and preserves cross-channel and cross-body coupling. It is a metric assembler, not an impulse solver.

Source-level tests cover dimensions 2/3/4/5, all independent bivector coordinates, orthogonality and reconstruction, coordinate-axis/near-axis normals, common rotations, transported-frame coordinate continuity, antipodal transport, 3D/4D wrench bounds, negative controls and SI-to-centimetre length-unit conversion. This remains a reference layer, not a production contact solver: the full Delassus matrix is now assembled and can be checked for symmetry/PSD, but neither validation mode computes a maximum-dissipation impulse, and the new matrix is not yet connected to production contact resolution.

Frame transport is important for warm-started impulses. A canonical frame rebuilt independently at every normal may jump when its deterministic pivot changes; transporting the previous frame avoids that arbitrary change except at the unavoidable antipodal ambiguity, where the implementation chooses a documented deterministic half-turn.

## Friction-wrench feasible-set semantics

There are at least two useful but non-equivalent feasible sets:

1. **Independent channel limits (existing `validate`)**:
   \[
   \|p_t\|\le b_t,\quad \|\tau_r\|\le b_r,\quad
   \|\tau_\tau\|\le b_\tau.
   \]
   Each channel can simultaneously reach its individual limit. This is a product set (three separate balls).

2. **Coupled ellipsoidal budget (new `validate_ellipsoidal_friction_budget`)**:
   \[
   \left(\frac{\|p_t\|}{b_t}\right)^2+
   \left(\frac{\|\tau_r\|}{b_r}\right)^2+
   \left(\frac{\|\tau_\tau\|}{b_\tau}\right)^2\le1.
   \]
   This is a joint budget: spending more of the tangential budget leaves less for rolling/torsion. A channel with zero bound permits only zero impulse in that channel.

These are explicit alternatives, not an assertion that either one is the unique physical friction law. Real anisotropic contact patches may need a full positive-definite metric with cross-channel coupling. The current methods only validate feasible inputs; selecting a maximum-dissipation solution requires the coupled effective metric and a separate solver.

## Research sequence

1. Implement a (D)-generic orthonormal tangent frame with deterministic pivoting and rotation-covariance tests.
2. Implement projectors for (T), (n\wedge T), and (\Lambda^2T); test reconstruction and orthogonality.
3. Assemble a small 3D contact-wrench effective metric for one asymmetric body and one point contact.
4. Add a 3D reference for tangential, torsional and rolling limits, including explicit contact-patch radius.
5. Extend the same basis/metric to D=4; confirm the six angular contact coordinates and all cross-couplings.
6. Compare hard-cone, soft-impedance and dual-cone formulations using the same geometry and external references.
7. Only after reference agreement, integrate the model into production contact solving.

## Acceptance and adversarial tests

- Orthonormal tangent basis: \(\|T^TT-I\|\le10^{-12}\).
- Projection closure: \(P_T+P_{n\wedge T}+P_{\Lambda^2T}=I\) on bivector coordinates within \(10^{-12}\).
- Rotation covariance under random proper rotations, including normals near coordinate axes.
- Unit scaling: converting length units scales angular impulse radii and angular contact coordinates consistently.
- Delassus/effective metric symmetry and PSD within scale-relative roundoff.
- Independent-channel and coupled-ellipsoid feasible sets are tested separately; no silent switch between them.
- Tangential, rolling and torsional friction work separately non-positive for a maximum-dissipation law.
- No friction impulse for an open/separating contact under the declared hard-contact model.
- Increasing the contact-patch discretization must converge in net wrench and work, not merely look visually stable.
- Negative controls for zero/negative radii, non-finite coefficients, rank-deficient contact metrics and impossible constitutive parameters.

## Industry benchmark refresh (2026-10-31)

The immediate 3D contact baselines now include MuJoCo 3.17.0, Rapier 0.35.3, PhysX 5.8, Newton and the 2026 ComFree-Sim preprint. For GPU scale, Dimforge's Nexus 0.7.0 / Physmark workload suite is the most relevant new Rust comparison: it separates rigid-body piles, many parallel robotics environments and MPM; the Q3 report publishes a 505,000-body pyramid at about 14 ms/frame on an RTX 5080 (vendor-reported, scene-specific). These are not direct validations of Symtropy's N-D contact law. The comparison programme adopts the workload families, pins the exact scene/backends/hardware, and adds the additional geometric and work residuals required for N-dimensional mechanics.

Primary links: [MuJoCo 3.17](https://github.com/google-deepmind/mujoco/releases/tag/3.17.0), [Dimforge Q3 report](https://dimforge.com/blog/2026/10/25/dimforge-Q3-technical-report/), [Physmark](https://github.com/dimforge/physmark), [ComFree-Sim preprint](https://arxiv.org/abs/2603.12185).

## Where this can compete

The 3D benchmark must compare with native 6D contact engines and the published ComFree-Sim model, using matched contact-patch geometry and physical-error envelopes. The 4D benchmark is a distinct capability claim: no standard 3D engine is a valid baseline for a 4D bivector contact manifold. The contribution is not claiming that the decomposition itself is new; it is building a dimension-generic, unit-consistent contact-wrench operator with independent verification and measured solver trade-offs.
