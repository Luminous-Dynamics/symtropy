# Coupled 3D Signorini-Coulomb point contact via quartic elimination

**Status:** independent single-contact impulse reference; exact-head Rust validation pending.  
**Date:** 2026-10-31  
**Parent:** [#3146 PHYS-FRICTION-3D-001](https://github.com/Luminous-Dynamics/symtropy/issues/3146), extending [#3145 fixed-normal-load tangential disk](https://github.com/Luminous-Dynamics/symtropy/pull/3145).  
**Qualification authority:** all eventual campaign receipts belong in [#460](https://github.com/Luminous-Dynamics/symtropy/issues/460).

## Research shift

A one-contact 3D Coulomb solve is not a two-stage operation in which the normal impulse is frozen and the tangent plane is solved afterward. The normal and tangent impulses are coupled through the full Delassus matrix. This reference solves the three components together and does not drop the normal/tangent blocks.

A generic nonlinear solve can return one locally consistent impulse while hiding other admissible solutions. Instead, the sliding branch is reduced to a quartic in the tangent-plane angle. The solver enumerates all real roots of that polynomial, validates each candidate against the original equations, and reports **Unique**, **MultipleSolutions**, or **Inconclusive**. It never picks one of several valid impulses as though it were uniquely determined.

## Frozen mathematical model

Use contact coordinates (n, t1, t2) and

\[
W =
\begin{bmatrix}
a & d^T\\
d & A
\end{bmatrix}\succ0,\qquad
q=\begin{bmatrix}q_n\\q_t\end{bmatrix},\qquad
p=\begin{bmatrix}\lambda\\j\end{bmatrix}.
\]

The post-impulse relative velocity is

\[
u=q+Wp.
\]

The normal law is

\[
\lambda\ge0,\quad u_n\ge0,\quad \lambda u_n=0.
\]

For a selected nonnegative normal impulse, the tangent law is maximum dissipation:

\[
j(\lambda)=\arg\min_{\|j\|\le\mu\lambda}
\left[\frac12j^TAj+(q_t+d\lambda)^Tj\right].
\]

The crucial coupling is explicit: the tangent free velocity is q_t + d lambda, and the normal velocity contains dᵀ j.

## Sliding elimination

For a sliding solution, write

\[
j=-\mu\lambda s,\qquad
s=(\cos\theta,\sin\theta),\qquad
s_\perp=(-\sin\theta,\cos\theta).
\]

The normal complementarity equation on the active branch gives

\[
0=u_n=q_n+\lambda(a-\mu d^Ts),
\qquad
D(\theta)=a-\mu d^Ts,
\qquad
\lambda=-q_n/D(\theta).
\]

The tangential disk KKT condition is

\[
u_t+\tau j=0,\qquad \tau\ge0.
\]

Therefore u_t is parallel to s; eliminating tau gives the single scalar angular equation

\[
s_\perp^T\left[q_t+\lambda d-\mu\lambda A s\right]=0.
\]

Substituting the normal equation and multiplying by D(theta),

\[
D(\theta)s_\perp^Tq_t
-q_n s_\perp^Td
+\mu q_n s_\perp^TAs=0.
\]

Under the tangent half-angle substitution t = tan(theta/2),

\[
\cos\theta=\frac{1-t^2}{1+t^2},\qquad
\sin\theta=\frac{2t}{1+t^2},
\]

multiplication by (1+t²)² produces a real polynomial of degree at most four. The implementation stores its coefficients in ascending powers of t, isolates its real roots by recursively partitioning at derivative roots, maps the roots back to angles, separately checks the theta = pi projective point, and tests the resulting impulses against the unsimplified KKT and complementarity equations.

A root is not accepted merely because it solves the reduced polynomial. It must also satisfy:
- positive normal impulse;
- the original normal complementarity equation;
- the Coulomb disk boundary;
- nonnegative disk multiplier;
- tangential KKT stationarity;
- non-positive full-contact kinetic-energy change.

The sticking branch is checked independently using the full unconstrained solution p = -W⁻¹q. Separating is retained as a candidate whenever q_n > 0. Thus a positive-approach state can expose multiple mathematical solutions under strong non-associated normal/tangent coupling instead of silently forcing the zero-impulse branch.

## Existence result: no-solution is the wrong claim for this bounded model

For one contact with finite mu and W positive definite, the frozen impulse law above has at least one solution. This is narrower than a general Painlevé statement and does not extend automatically to multiple simultaneous contacts, acceleration-level rigid-body equations, singular Delassus operators, or compliant contact.

For each lambda >= 0, the tangent subproblem has a unique minimizer because A is positive definite and the Coulomb disk is compact. Write j(lambda) = lambda z(lambda). Then z(lambda) minimizes over the fixed compact disk norm(z) <= mu:

\[
\frac12z^TAz+d^Tz+\frac{1}{\lambda}q_t^Tz.
\]

As lambda tends to infinity, strict convexity gives convergence to the unique minimizer z* of

\[
\frac12z^TAz+d^Tz,\qquad\|z\|\le\mu.
\]

Its KKT form is z* = -(A + tau I)⁻¹d, with tau >= 0 (including tau = 0 for an interior minimizer). Consequently,

\[
d^Tz_*=-d^T(A+\tau I)^{-1}d
\ge-d^TA^{-1}d.
\]

Because W is positive definite, its Schur complement obeys

\[
a-d^TA^{-1}d>0.
\]

The reduced normal residual

\[
\phi(\lambda)=q_n+a\lambda+d^Tj(\lambda)
\]

is continuous, starts at phi(0) = q_n, and has the positive asymptotic slope

\[
\lim_{\lambda\to\infty}\frac{\phi(\lambda)}{\lambda}
=a+d^Tz_*
\ge a-d^TA^{-1}d>0.
\]

For an approaching contact q_n < 0, the intermediate value theorem therefore guarantees at least one positive root. For q_n >= 0, zero impulse is already a separating/grazing solution. Thus **a numerically empty candidate set is Inconclusive, not a physical NoSolution classification**, for this SPD single-contact model.

This existence argument does not prove uniqueness. The independent candidate enumeration includes a concrete SPD fixture with at least three distinct admissible impulses (one sticking and two sliding), demonstrating that multiple solutions must be represented as an outcome rather than hidden behind one selected answer.

## What the implementation contains

- New public module coupled_friction and crate-level exports.
- Full symmetric positive-definite 3x3 Delassus validation.
- Coupled normal/tangent elimination retaining both cross blocks.
- Quartic coefficient construction in the caller's contact frame.
- Recursive derivative-partition real-root isolation for degree at most four; unresolved near-degenerate roots fail closed.
- Explicit projective theta = pi check.
- Independent sticking and separating candidates.
- Original-equation checks for Signorini complementarity, tangent KKT, Coulomb disk and full kinetic-energy change.
- Unique, MultipleSolutions and Inconclusive result classes; no arbitrary impulse selection.
- Tests for diagonal stick/slip, off-diagonal coupling, separating contact, a three-solution SPD witness, tangent-frame rotation, grazing, invalid matrices, and a deterministic SPD approach panel.

## Benchmark plan and target gates

These are promotion targets, not achieved results:

| Observable | Initial target |
|---|---:|
| Normal complementarity residual | <= 1e-10 normalized |
| Tangential KKT stationarity | <= 1e-10 normalized |
| Coulomb disk excess | <= 1e-12 relative |
| Positive kinetic-energy change from contact impulse | 0 beyond roundoff envelope |
| Tangent-frame covariance | <= 1e-10 |
| Quartic roots independently cross-checked | 100% on frozen bounded corpus |
| Multiple-solution detection | 100% on known multi-root fixtures |
| Silent arbitrary root selection | 0 |
| Non-finite / invalid-metric acceptance | 0 |

The independent validation ladder should include:
1. diagonal and block-diagonal analytic cases;
2. off-diagonal SPD cases with independently generated high-precision references;
3. stick/slip boundary sweeps and repeated polynomial roots;
4. tangent-plane rotations and coherent unit changes;
5. near-singular SPD spectra, with typed rejection before the tolerance envelope is exceeded;
6. multi-root witnesses and perturbations around their folds;
7. negative controls that flip the normal sign, reverse a tangent basis, violate SPD, inject NaN/Inf, or perturb a candidate outside the Coulomb disk.

The quartic routine itself needs differential testing against a separate high-precision polynomial root isolator. Sharing the same coefficients is acceptable; sharing the same root-isolation algorithm is not.

## Industry comparison

This reference should first be compared with independent mathematical oracles, not with an entire engine on an unmatched scenario. The next software baselines are:
- the fixed-load tangent disk reference in #3145;
- a separately implemented nonlinear complementarity/KKT solver;
- a high-precision scalar-normal-residual continuation oracle;
- later, engine-level matched contact fixtures through the canonical differential runner in #443.

MuJoCo, PhysX, Rapier, Chrono and Newton remain relevant for end-to-end solver behaviour, but their contact stabilization, restitution, warm starting, iteration budgets and friction conventions must be mapped before any numeric comparison. A solver disagreement is evidence to investigate; no engine is automatically ground truth.

### What the current industry modes actually solve

The benchmark must name the solver mode, because the default friction approximation is not uniform across engines:

- **MuJoCo 3.17:** contact cones may be elliptic or pyramidal; its PGS, CG and Newton solvers solve the chosen constraint model. The NoSlip solver is an additional friction-only post-processing pass. The documentation explicitly says exact zero slip is not guaranteed, that convergence depends on the iteration cap, and that the pass adds cost and can occasionally destabilize complex multi-contact systems. Compare elliptic/Newton and PGS or pyramidal variants separately; do not treat all configurations as one baseline. References: [computation / cone model](https://mujoco.readthedocs.io/en/3.17.0/computation.html), [modeling / friction and NoSlip](https://mujoco.readthedocs.io/en/3.17.0/modeling.html), [solver enums](https://mujoco.readthedocs.io/en/3.17.0/APIreference/APItypes.html).
- **Rapier 3D:** the documented default Simplified friction model uses one Coulomb constraint per group of four contacts plus a rotational twist constraint; the alternative Coulomb model solves one constraint per contact point and is described as more mechanically correct but more expensive. A single-point benchmark should use per-point Coulomb mode. A four-point patch benchmark should compare both modes and report the distinct model semantics, plus warm-starting and contact clustering. Reference: [Rapier Rust integration parameters](https://rapier.rs/docs/user_guides/rust/integration_parameters/).
- **PhysX:** the current patch-friction model selects up to two friction anchors per contact patch. TGS uses the combined impulse in the two tangent axes; PGS processes those axes separately, which the docs note can produce asymmetries in some static/dynamic-friction transitions. TGS is slightly more expensive. This maps directly to tangent-frame covariance, anchor-layout and axis-order stress cases. Reference: [PxFrictionType / patch friction](https://nvidia-omniverse.github.io/PhysX/physx/latest/_api_build/structPxFrictionType.html).
- **Project Chrono:** NSC treats contacts as hard/stiff constraints and uses variational-inequality solvers; SMC computes reaction forces using a penalty/compliance model. These are separate benchmark lanes, not interchangeable parameter settings. Reference: [Chrono collision and contact formulations](https://api.projectchrono.org/collisions.html).
- **Newton:** GPU collision, SDF-based mesh contacts, hydroelastic contact sampling and multiple solver backends make it a strong contact-rich robotics comparison. Hydroelastic surface sampling changes the contact representation and force distribution, especially for torsional friction; it is not a like-for-like single-point impulse reference. Begin with its point-contact-compatible solver path, then run a separate surface-contact fidelity campaign. References: [Newton collisions](https://newton-physics.github.io/newton/latest/concepts/collisions.html), [Newton architecture](https://developer.nvidia.com/newton-physics).

### Matched benchmark ladder

1. **Mathematical point-contact oracle:** identical SPD W, q, mu and impulse-level Signorini-Coulomb law. Compare candidate count, normal complementarity, tangent KKT, cone violation, energy change and classification against an independent high-precision KKT/root-count implementation. This is where Symtropy's quartic method must first establish correctness.
2. **Solver-mode bridge:** map a single sphere/plane or body/body contact into each engine with restitution disabled, matching tangent model and material coefficient. Record whether each engine exposes or can reconstruct the same W and contact impulse; mark unobservable fields unavailable, never zero.
3. **Patch and manifold cases:** compare Rapier simplified/per-point, PhysX patch-anchor PGS/TGS, MuJoCo elliptic/pyramidal, Chrono NSC/SMC and Newton point/hydroelastic contact as explicitly named model variants. Include point count, anchor layout, patch extent, tangent rotation and axis permutation.
4. **Contact-rich system:** only after the single-contact law is qualified, extend to stacks, frictional grasping, redundant multi-contact and manifold changes. Use #443 for executable engine differential traces and #460 for the shared evidence record.

For every lane, preregister timestep/substeps, material parameters, restitution, contact softness/stiffness, warm start, solver iterations/tolerances, thread/backend, hardware, model geometry and initial state. Publish work-precision curves rather than one fastest-run number: residual and physical error versus runtime, along with p50/p95/p99 step latency, memory, throughput, slip drift, energy/work balance, penetration and failure classifications. The target is not an arbitrary speedup; it is a better accuracy-cost-reliability frontier at the same physical model and declared error budget.

## Claim ceiling

This reference covers one 3D point contact, fixed Coulomb coefficient, impulse-level Signorini complementarity, and SPD 3x3 Delassus matrices. It does not qualify multiple contacts sharing rigid bodies, rolling/torsional friction, restitution, compliant contact, singular Delassus systems, acceleration-level Painlevé problems, production island integration, or SOTA performance.

All benchmark receipts feed the existing qualification authority #460. No second corpus authority is introduced.
