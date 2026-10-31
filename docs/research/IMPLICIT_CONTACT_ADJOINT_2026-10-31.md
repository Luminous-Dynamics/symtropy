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

## Implemented primitives

The prototype now differentiates two reference contact families:

1. The small-island frictionless LCP under a locally fixed active set.
2. The single-contact planar maximum-dissipation Coulomb reference under a locally fixed separating, sticking, positive-sliding or negative-sliding mode.

The planar reference exposes its active mode as explicit solver output so the derivative does not have to infer a mode from rounded impulses.

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

### Planar friction-mode sensitivity

For sticking, both post-contact components are constrained to zero and:

\[
\frac{d\lambda}{d\theta}
=-W^{-1}\left(\frac{dW}{d\theta}\lambda+
\frac{dq}{d\theta}\right).
\]

For sliding with signed tangential velocity s in {+1,-1}, the active Coulomb edge is:

\[
\lambda=\lambda_n c,\qquad
c=\begin{bmatrix}1\\-\mu s\end{bmatrix},
\qquad
e_n^T(Wc\lambda_n+q)=0.
\]

Differentiating the scalar normal equation includes both dW and dmu:

\[
\frac{d\lambda_n}{d\theta}
=-\frac{
e_n^T\left[
\left(\frac{dW}{d\theta}c+
W\frac{dc}{d\theta}\right)\lambda_n+
\frac{dq}{d\theta}
\right]
}{
e_n^T Wc
},\qquad
\frac{d\lambda}{d\theta}
=c\frac{d\lambda_n}{d\theta}
+\frac{dc}{d\theta}\lambda_n.
\]

The separating mode has zero impulse sensitivity while the positive normal separating velocity remains outside its transition margin. Sticking rejects a near-zero Coulomb-cone slack; sliding rejects near-zero tangential post-velocity or a vanishing friction coefficient. These are one-sided mode derivatives, not derivatives across mode changes.

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
- planar sticking and positive-sliding derivatives against central finite differences, including friction-coefficient sensitivity;
- negative-sliding sensitivity on the opposite Coulomb edge;
- strict separating-mode zero sensitivity;
- stick–slip transition rejection;
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

1. Validate the fixed-active-set LCP and planar mode derivatives under exact-head CI and the randomized finite-difference tests.
2. Derive the coupled 3D contact-wrench cone derivative using the exact constitutive model selected for that experiment. Do not differentiate a feasibility approximation and call it maximum-dissipation friction.
3. Move from dense small matrices to matrix-free Jacobian-vector and transpose-Jacobian-vector products through the production contact residual.
4. Reuse the production Newton/Schur factorization in the adjoint pass; avoid storing the entire internal iteration tape.
5. Add checkpointed long-horizon trajectory benchmarks and real measured hardware trajectories.

## Industry comparison

- **Ostrich:** hard NCP/Coulomb contact, non-smooth Newton, backward Euler, IFT adjoint using the forward Schur operator; GPU meshes and many parallel worlds. This is the most direct comparator for residual-based hard-contact gradients.
- **DiffMJX:** adaptive integration for penalty-contact gradient accuracy, with contacts-from-distance used only in the backward pass.
- **Bundled Contact Gradients:** stiff forward dynamics, local randomized contact bundles in the backward pass to reduce gradient variance.
- **ComFree-Sim:** complementarity-free dual-cone analytical contact for highly parallel dense contact. It is a forward-solver alternative; any gradient method would need to be specified and benchmarked separately.

The differentiation campaign must compare equal forward physical fidelity first. Gradient accuracy on a physically wrong trajectory is not a win, and fast gradients that cross an active-set discontinuity without an explicit policy are not qualified.
