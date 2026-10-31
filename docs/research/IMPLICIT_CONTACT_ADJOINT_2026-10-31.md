# Implicit differentiation of converged contact solves

**Status:** experimental fixed-active-set derivative primitive.
**Date:** 2026-10-31
**Implementation:** branch research/implicit-contact-adjoint-2026-10-31.

## Why this is a separate research axis

A contact solver that is accurate in the forward direction is not automatically useful for gradient-based control. Conversely, smoothing hard contact to obtain gradients can change the forward mechanics. The next research direction should keep those objectives separate:

- hard-contact forward solve with explicit residual and physical acceptance criteria;
- derivative of the converged residual, not a tape through every solver iteration;
- explicit rejection when the active set is changing or the derivative is ill-conditioned;
- independent gradient checks against finite differences and, later, real robot trajectories.

The September 2026 Ostrich preprint reports a GPU-accelerated hard-contact simulator using non-smooth Newton, backward Euler and an implicit-function-theorem adjoint that reuses the forward Schur complement. It reports O(1) adjoint memory per timestep, a roughly 50x larger usable timestep than MJX on its evaluated pallet trajectory, and 8,192 parallel worlds on one 24 GB GPU. These are author-reported, scene-specific measurements. Ostrich is a strong research comparator, not a generic performance claim.

DiffMJX (ICLR 2026) addresses gradient accuracy in penalty-based contacts through adaptive time integration and backward-only contacts-from-distance. Bundled Contact Gradients (September 2026) uses contact-local randomized bundles to reduce gradient variance for stiff contact and reports transfer to a Unitree G1. These are distinct approaches with different forward/gradient tradeoffs; Symtropy should benchmark all three objectives rather than conflate them.

## Implemented primitive

The current prototype differentiates the small-island frictionless reference LCP under a locally fixed active set.

For active rows A:

\[
W_{AA}\lambda_A+q_A=0.
\]

For a scalar parameter theta:

\[
\frac{d\lambda_A}{d\theta}
=-W_{AA}^{-1}\left(
\frac{dW_{AA}}{d\theta}\lambda_A+
\frac{dq_A}{d\theta}
\right).
\]

For scalar objective L(lambda,theta), solve the adjoint:

\[
W_{AA}^{T}\psi_A=
\frac{\partial L}{\partial\lambda_A}
\]

and compute:

\[
\frac{dL}{d\theta}
=\frac{\partial L}{\partial\theta}
-\psi_A^T\left(
\frac{dW_{AA}}{d\theta}\lambda_A+
\frac{dq_A}{d\theta}
\right).
\]

The public API accepts W, q, a reference LCP solution, dW/dtheta, dq/dtheta, the objective impulse gradient, the direct objective gradient, an absolute impulse margin, an absolute velocity-slack margin, and a relative numerical tolerance.

The separate absolute margins intentionally have different units. A single dimensionless threshold must not be applied indiscriminately to impulses and velocity residuals.

## Failure semantics

The derivative returns an explicit error when:
- dimensions or values are invalid/non-finite;
- W or its parameter derivative is not symmetric within tolerance;
- W is not positive semidefinite;
- the supplied LCP residual does not match W*lambda+q or violates primal/complementarity conditions;
- an active impulse is too close to zero;
- an inactive row's slack is too close to zero;
- the active block is singular or too ill-conditioned for a locally unique derivative.

The implementation does not regularize a singular active block to manufacture a gradient. At a contact creation, separation, stick-slip or mode transition, the classical derivative may not exist or may be branch-dependent; the correct response is an explicit boundary result and a separately specified generalized-derivative or smoothing policy.

## Tests and benchmark

The source tests cover:
- a one-row analytic derivative with a closed-form sensitivity;
- central finite-difference agreement for the same parameterized LCP;
- 24 deterministic randomized SPD-block checks across 1–4 active rows, comparing both impulse sensitivities and objective gradients against central finite differences;
- a strictly inactive-contact case that returns the direct objective gradient and zero contact sensitivity;
- parameter sensitivity through q (contact bias/free velocity);
- rejection near an active-set transition;
- rejection of a rank-deficient active block;
- rejection of a non-monotone contact matrix.

The Criterion benchmark target now measures fixed-active-set adjoint cost for 1, 2, 4 and 8 rows alongside assembly and LCP reference solve. The adjoint benchmark uses constructed, well-conditioned SPD active blocks to isolate derivative-solve cost; geometric asymmetric-inertia assembly remains measured in separate benchmark groups. It has not been executed; no runtime or memory result is claimed.

## Promotion targets

For well-conditioned fixed-active-set cases:
- adjoint equation residual <= 1e-10 relative;
- implicit sensitivity vs central finite difference <= 1e-6 relative over a documented epsilon sweep;
- gradient directional cosine >= 0.99999 on randomized stable-active-set problems;
- zero false-valid gradients on deliberately generated active-set boundary and rank-deficient cases;
- record condition estimate and refuse gradients above a pinned condition threshold;
- show the adjoint solve reuses the same factorization/Schur complement as the forward solve in the later production implementation.

For horizon-level differentiable simulation:
- adjoint memory per step should not scale with inner contact-iteration count;
- compare memory and throughput vs tape-based AD at equal physical forward error;
- report gradient variance, directional accuracy, trajectory optimization success and sim-to-real transfer separately;
- vary timestep, contact stiffness, friction, mass ratio, mesh resolution and number of parallel worlds.

The thresholds are campaign acceptance targets, not measured results.

## Next implementation stages

1. Validate this small-LCP derivative under CI and randomized fixed-active-set finite differences.
2. Extend the reference derivative to the planar friction-cone KKT/active-face equations already present in Symtropy. Track stick, positive slide, negative slide and transition boundaries as distinct modes.
3. Derive the coupled 3D contact-wrench cone derivative using the exact constitutive model selected for that experiment. Do not differentiate a feasibility approximation and call it maximum-dissipation friction.
4. Move from dense small matrices to matrix-free Jacobian-vector and transpose-Jacobian-vector products through the production contact residual.
5. Reuse the production Newton/Schur factorization in the adjoint pass; avoid storing the entire internal iteration tape.
6. Add checkpointed long-horizon trajectory benchmarks and real measured hardware trajectories.

## Industry comparison

- **Ostrich:** hard NCP/Coulomb contact, non-smooth Newton, backward Euler, IFT adjoint using the forward Schur operator; GPU meshes and many parallel worlds. This is the most direct comparator for residual-based hard-contact gradients.
- **DiffMJX:** adaptive integration for penalty-contact gradient accuracy, with contacts-from-distance used only in the backward pass.
- **Bundled Contact Gradients:** stiff forward dynamics, local randomized contact bundles in the backward pass to reduce gradient variance.
- **ComFree-Sim:** complementarity-free dual-cone analytical contact for highly parallel dense contact. It is a forward-solver alternative; any gradient method would need to be specified and benchmarked separately.

The differentiation campaign must compare equal forward physical fidelity first. Gradient accuracy on a physically wrong trajectory is not a win, and fast gradients that cross an active-set discontinuity without an explicit policy are not qualified.
