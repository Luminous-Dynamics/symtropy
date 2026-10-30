# Research campaign: a unified discrete contact/effective-mass solve

**Status:** research specification and benchmark contract; no solver superiority claim.
**Date:** 2026-10-30
**Related work:** [Physics numerical credibility PR #2927](https://github.com/Luminous-Dynamics/symtropy/pull/2927).

## 1. Research hypothesis

The next meaningful step beyond a more accurate rotational inertia tensor is to remove the split between:

- free rigid-body integration,
- normal contact impulses,
- tangential friction impulses,
- joint constraints,
- damping/stiffness response, and
- post-step energy accounting.

These subsystems currently use related but separately assembled approximations. The hypothesis is that an island-level **discrete effective-mass metric** can make them share one physically consistent step operator, reducing disagreement between the dynamics update and the constraint solver.

This is a hypothesis to test. MuJoCo already has a discrete integrator that combines the constraint solve and implicit velocity update in an effective metric. Symtropy must demonstrate a concrete advantage—dimension-generic mechanics, reproducible numerical evidence, better error/cost, or more transparent failure handling—not claim the idea as novel.

## 2. Mathematical contract

For one dynamic body in spatial dimension D, use generalized velocity

[
u = (v, B) in mathbb R^D oplus Lambda^2(mathbb R^D),
qquad
n_u = D + rac{D(D-1)}{2}.
]

Here B is Symtropy's stored rotor bivector. Physical point velocity at world offset r is

[
v_p = v - B r.
]

The generalized inverse-mass action must use the same translational mass and world-rotated bivector inertia operator used by free integration. Bodies with no validated operator may use an explicitly identified legacy mode, but benchmark receipts must never mix the two silently.

For an island of bodies, stack all generalized velocities into u, define the generalized mass operator M, and form the contact/joint Jacobian J from the same kinematic convention. Let v_free be the velocity predicted by forces and the selected smooth-force integration policy.

A research discrete step can be written as

[
u^+ = argmin_u ; rac12 (u-u_{free})^T M_{eff}(u-u_{free})
]

subject to unilateral contact, joint and other admissible constraints, where

[
M_{eff}=M+hD+h^2K
]

is a candidate positive-definite effective metric incorporating implicit damping D and stiffness K where those terms are valid. For a frictionless unilateral-contact prototype, the equivalent reduced system is

[
Wlambda + b ge 0,quad lambda ge 0,quad
lambda^T(Wlambda+b)=0,
qquad W=J M_{eff}^{-1}J^T.
]

Friction requires a maximum-dissipation formulation or a documented cone approximation. The prototype must not pretend a normal-only LCP is a complete Coulomb-friction solver.

The critical consistency requirement is that the same M_eff governs both constraint response and the committed velocity update. Do not solve constraints with one effective inertia and then advance the state with a materially different operator without measuring and reporting that splitting error.

## 3. Architecture experiment

Keep the existing sequential impulse/TGS path as the baseline. Add a separately feature-gated research path in stages:

1. **Contact row contract.** Define a typed row with body handles, world contact offsets, unit normal/tangent direction, right-hand side/bias, unilateral/bilateral kind, and provenance. Validate finite values and normalization.
2. **Generalized response.** Implement a single operation that maps an impulse on a row to the resulting linear and stored-bivector velocity changes using the same inverse-mass and `BivectorInertia` operators as free integration.
3. **Delassus assembly.** Build W_ij by applying the generalized response for row j and evaluating row i's relative point velocity. Verify symmetry and positive semidefiniteness for valid unconstrained mass operators.
4. **Reference solve.** Start with tiny islands (1–8 rows) and a dense reference solver. Compare to a direct KKT/LCP oracle or exhaustive active-set enumeration for small cases. No custom sparse solver until the dense oracle validates the formulation.
5. **Friction.** Add a maximum-dissipation or explicit cone approximation only after normal-contact results are stable. Measure cone residual and frictional work.
6. **Stiff terms.** Add damping/stiffness contributions to M_eff only when a positive-definiteness check and energy/work accounting validate them.
7. **Promotion decision.** Keep this path behind a research feature until matched-work benchmarks justify any runtime default change.

## 4. P0 benchmark contract

Inputs, reference outputs, tolerances, and promotion limits must be committed before performance runs.

| Campaign | Scenario | Required measurements | Initial promotion gate |
|---|---|---|---|
| Algebra | 1–8 row islands, random well-conditioned SPD body operators | W symmetry, eigenvalue floor, solve residual, complementarity residual | symmetry <= 1e-12 relative; residuals <= 1e-10 |
| Analytic contact | sphere-plane, asymmetric ellipsoid-plane, two-contact block | impulse, post-impact velocity, restitution, angular response | effective-mass/impulse identity <= 1e-10 |
| Stacking | 1/10/100 box stacks, equal and extreme mass ratios | penetration max/p95, jitter RMS, energy drift, collapse rate | zero unexplained energy creation; no stack collapse in canonical set |
| Friction | sliding block, oblique impact, multi-point patch | friction cone violation, slip speed, dissipated work | non-positive frictional work within numerical tolerance |
| Gyroscopic | torque-free asymmetric top, forced top, rolling ellipsoid | world angular momentum, kinetic energy, orientation error | error converges under dt refinement; report long-horizon drift |
| CCD | high-spin thin rod, fast projectile, two moving bodies | tunneling rate, TOI error, extra substeps, wall time | zero misses in the frozen adversarial suite |
| Performance | small, medium and large islands | p50/p95 step time, allocations, matrix assembly/solve cost | compare only at matched physical error and scene settings |

Run a step-size and island-size sweep. Publish work-precision curves, not a single frames-per-second result. Include failed and ill-conditioned cases in the result files.

## 5. Industry comparison protocol

Pin engine version, scene assets, timestep, collision margins, solver settings, friction/restitution, CCD flags, and machine/OS/compiler in each receipt.

- **MuJoCo:** use a pinned 3.16.x build. Compare the current recommended `implicitfast` baseline and the newer `discrete` integrator separately; the latter is documented as new and under active development. Its 2026 discrete path combines constraints and the implicit velocity update in a shared effective metric. Do not compare it only against Symtropy's current split sequential-impulse path and then claim a general solver victory.
- **Rapier:** use a pinned 0.35.x build. Record solver iteration counts, internal PGS/stabilization iterations, warm-start settings, contact softness, CCD substeps, and length-unit normalization. The default solver iteration count and demanding-scene recommendations are not equivalent to the same numerical tolerance as Symtropy.
- **Independent mathematical oracle:** for small islands, use active-set enumeration or a trusted complementarity solver; for free rigid bodies, use high-accuracy ODE integration and conservation invariants. Engines are comparison baselines, not ground truth.

Primary score: error and invariant drift versus total compute. Secondary scores: contact stability, robustness/failure rate, determinism/replay fidelity, and engineering cost. A faster result that violates physical constraints is not a win.

## 6. Paradigm shifts worth testing

### 6.1 Adaptive solver effort from physical defect, not fixed iteration count

Estimate the post-solve complementarity residual, energy/work residual, penetration defect and local integration error. Spend extra iterations or substeps only on islands whose defects exceed a declared budget. This could outperform fixed global iteration counts on heterogeneous scenes, but must be tested against worst-case tail latency and starvation.

### 6.2 Conservative-by-construction exchange ledger

Every external force, actuator, damping term, contact impulse, friction impulse and stabilization correction records its expected work/energy exchange. The step report reconciles measured state-energy change against the ledger. Numerical drift becomes an observable residual with an attributable source, not just a scalar total-energy trend.

### 6.3 Multi-fidelity island solve

Use an inexpensive diagonal/block approximation to precondition a more accurate island-level solve. Promote an island to the expensive solve when its condition estimate, residual, mass ratio or contact coupling demands it. Validate both the classifier and the fallback; never let a learned/heuristic choice certify its own correctness.

### 6.4 Dimension-generic mechanics as a real differentiator

Keep D-dimensional translational and bivector rotational mechanics in the same algebraic API. Verify D=2, 3, 4, then test randomized higher dimensions for algebraic consistency. Do not confuse generic compilation with evidence of correct or efficient high-D simulation.

## 7. Required receipts

Each run must persist:

- source commit SHA and dirty-tree status;
- canonical scene/input SHA-256 and scenario schema version;
- engine/backend versions and complete solver settings;
- compiler, target triple, CPU/GPU, OS and feature flags;
- wall-clock distribution (warmups, samples, p50/p95), memory and allocations;
- residuals, invariant drift, constraint violations, failed solves and CCD misses;
- reference fixture version, reference precision, and uncertainty envelope.

A missing required field invalidates the comparison. A queued or skipped workflow is not a passing result.

## 8. Exit decision

Do not replace the existing solver based on a clean demo. Promotion requires:

1. small-island algebra and oracle comparisons pass;
2. cross-path impulse/effective-mass identity passes;
3. contact and friction campaigns meet frozen physical tolerances;
4. high-spin and CCD adversarial suite reports no misses;
5. matched-work performance is competitive with the pinned MuJoCo/Rapier baselines;
6. the new path does not regress legacy scenes or determinism/replay contracts.

Until then this remains an isolated research implementation and benchmark campaign.
