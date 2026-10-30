# Contact solver portfolio and industry benchmark programme

**Date:** 2026-10-30
**Status:** research contract and proposed promotion gates; no comparative benchmark results claimed.

## Executive direction

Do not choose one contact solver and optimize it blindly. Build a **solver portfolio** whose selection is driven by measurable island structure, stiffness, residuals, friction model and hardware:

1. **Small-island reference:** exact/least-norm frictionless LCP oracle for 1–8 normal rows; correctness oracle, not production.
2. **Realtime CPU path:** sequential/colored projected iterative solver, warm-started and bounded.
3. **Coupled accuracy path:** island-level Delassus/effective-metric solve for difficult, stiff or highly coupled contacts.
4. **Friction path:** a maximum-dissipation Coulomb-cone reference, with a scalable cone/dual-cone implementation only after agreement tests.
5. **Accelerated path:** batched GPU contact path for many independent islands/contacts, with deterministic challenge cases against a CPU reference.
6. **Recovery path:** adaptive substepping, higher iteration budget, or a better-conditioned solver when monitored residuals fail. A retry/recovery choice must be recorded in the step receipt.

Selection must be auditable: report why a path was chosen, its conditioning estimate, stopping residuals, time budget and fallback.

## Industry landscape as of 2026-10-30

This is a comparison map, not a single leaderboard. The systems solve overlapping but non-identical problems.

| System | Relevant current capability | Implication for Symtropy |
|---|---|---|
| MuJoCo 3.16.0 | The current release is dated 2026-10-14. Its discrete integrator combines constraint solve and time integration in an effective metric \(\widehat M=M+hD+h^2K\), so the solve and velocity update share the same inertia. | Benchmark a shared effective-metric formulation; do not claim novelty merely for assembling a Delassus matrix. |
| Rapier 0.35.3 | SIMD-batched contact processing is always enabled. The October 2026 Dimforge report describes a reworked parallelism model with intra-island parallelism, persistent graph coloring and parallel broad phase. | Rapier is the mandatory Rust CPU baseline. Comparing only independent-island parallelism would be outdated. |
| NVIDIA PhysX 5.8 | Provides PGS and TGS, with TGS generally recommended; NVIDIA documents improvements in convergence, high-mass-ratio handling, drive accuracy and high-frequency effects. | Include dense stacks, high mass ratios and stiff drives; compare both time-to-residual and runtime. |
| NVIDIA Newton | Open, GPU-accelerated, differentiable simulator built on Warp/OpenUSD; developed with Google DeepMind and Disney Research and managed by the Linux Foundation. | Use as an accelerated/differentiable robotics baseline; separate GPU and CPU comparisons. |
| ComFree-Sim (2026 preprint) | Proposes analytical complementarity-free contact using a dual-cone impedance-style prediction/correction; includes a unified 6D friction model and reports near-linear scaling and 2–3× throughput vs MuJoCo Warp in its dense-contact experiments. | This is a serious alternative to iterative complementarity for dense contact. Reproduce matched cases; treat its reported speedup as an author result, not independently established fact. |
| Project Chrono | Engineering/multiphysics-focused system with implicit integration strategies and broader physical coupling. | Use for selected engineering cases rather than claiming apples-to-apples rigid-body game-engine equivalence. |

Primary reading:
- MuJoCo release history: https://github.com/google-deepmind/mujoco/releases
- MuJoCo computation and discrete integrator: https://mujoco.readthedocs.io/en/latest/computation/
- Rapier 0.35.3 package/docs: https://docs.rs/rapier3d/0.35.3
- Dimforge Q3 2026 report: https://dimforge.com/blog/2026/10/25/dimforge-Q3-technical-report/
- PhysX 5.8 simulation docs: https://nvidia-omniverse.github.io/PhysX/physx/5.8.0/docs/Simulation.html
- NVIDIA Newton: https://developer.nvidia.com/newton-physics
- ComFree-Sim paper: https://arxiv.org/abs/2603.12185

## What would actually differentiate Symtropy

### A. One effective metric shared by dynamics and constraints

Extend the reference from \(W=JM^{-1}J^T\) to an explicitly declared step metric:

\[
\widehat M=M+hD+h^2K,\qquad
W_h=J\widehat M^{-1}J^T.
\]

The matrix is admitted only if the selected damping/stiffness terms keep it symmetric positive definite (or a documented semidefinite solve is used). The same \(\widehat M\) must govern both the velocity step and constraint response. This is a known direction in MuJoCo; Symtropy's opportunity is to generalize it consistently across N-dimensional bivector rotation, contact, joints, and machine-checkable work accounting.

### B. A dual view of friction

The current reference oracle is frictionless. The next reference should be maximum-dissipation Coulomb contact, then compare primal cone, pyramidal approximation and dual-cone/impedance formulations on the same fixtures.

For a contact with normal impulse \(\lambda_n\), tangential impulse \(\lambda_t\), and coefficient \(\mu\):

\[
\lambda_n\ge0,\qquad \|\lambda_t\|\le\mu\lambda_n.
\]

The comparison must report cone violation, post-contact tangential slip, normal complementarity, and signed work. Torsional/rolling friction must remain separate model terms until the underlying contact geometry and units are explicit.

### C. Solver choice by predicted difficulty

Collect cheap per-island features:
- row/body count and graph degree;
- estimated condition number or spectral bounds of the effective operator;
- mass ratio, inertia anisotropy and redundancy indicators;
- stiffness/compliance and contact persistence;
- current residual reduction per iteration.

Select and budget the solver from those features. A fallback is triggered by residual stagnation, energy inconsistency, or a conditioning threshold—not solely a fixed iteration count.

### D. Mechanical evidence as part of the solver API

Every accepted island/step should expose:
- exact input/configuration/source digests;
- pre/post generalized velocities and poses;
- normal and tangential impulses;
- primal, dual and complementarity residuals;
- normal restitution work, friction work, stabilization work and actuator/external work as separate signed terms;
- momentum/energy balance residuals with unmodelled terms identified;
- solver selected, iteration count, fallback and termination reason.

A receipt is not a certificate unless the input observation boundary and every work channel are complete. Missing channels should mark the evidence incomplete.

## Benchmark suite

### 1. Analytical and metamorphic kernel suite

- 1–8 row analytical frictionless LCPs, including redundant/rank-deficient systems.
- 2D/3D/4D asymmetric inertia and all independent rotation planes.
- Contact row permutation, global rigid-rotation covariance, object relabelling, unit conversion, and supported parallel scheduling.
- Singular and nearly singular inertia/effective-mass negative controls.
- Compare residuals and physical outputs, not necessarily unique impulses where the solution is non-unique.

### 2. Contact mechanics suite

| Fixture | Sweep | Measures |
|---|---|---|
| Single asymmetric body impact | restitution 0–1; all principal and coupled planes | impulse error, angular response, momentum and energy |
| Redundant block/plane patch | 1–16 points; symmetric and offset COM | complementarity, impulse distribution stability, penetration |
| Stacked boxes | 10/100/1,000 bodies; mass ratios 1:1 to 1:10,000 | drift, residual, maximum penetration, time-to-stability |
| Dense granular pile | 1k/10k/100k contacts | throughput at fixed penetration/residual, memory, determinism |
| Sliding/rolling/torsion contact | friction 0–1.5; anisotropic mass moments | cone violation, slip, rolling/torsional response, dissipated work |
| Stiff contact/actuator | logarithmic stiffness/damping sweep | stability envelope, energy behavior, time-to-residual |
| High-speed/thin obstacle | dt and velocity sweep with CCD enabled/disabled | tunneling rate, false positives, time cost |
| Articulated manipulation | hand/object/robot contact | grasp retention, contact wrench error, closed-loop task success |

Industry comparisons should begin with 3D equivalents and the same material/solver settings wherever mappable. The 4D results are a distinct Symtropy research claim, not a fake cross-engine victory.

### 3. Performance and accuracy reporting

Publish for each fixture:
- median and p95 step time; p50/p95 time-to-target residual;
- contacts/second and bodies/second, with exact scene size;
- max/mean penetration and normal complementarity violation;
- friction-cone violation and tangential post-slip;
- normalized momentum and energy/work residual;
- memory high-water mark and allocations/step;
- run-to-run variation and deterministic replay divergence;
- CPU single-thread, CPU multi-thread, and GPU runs in separate tables.

**Do not use raw FPS as the primary quality metric.** The primary plot is time-to-accuracy: runtime required to reach a fixed, independently defined physical-error envelope.

## Proposed numerical gates

These are targets for the reference and carefully bounded fixtures, not guarantees for arbitrary scenes:

- Delassus symmetry: \(\|W-W^T\|_\infty/\max(1,\|W\|_\infty)\le10^{-12}\).
- PSD: \(\lambda_{\min}(W)\ge-10^{-12}\max(1,\|W\|_2)\) for valid mass/constraint rows.
- Analytic reference primal/complementarity residual: normalized <= \(10^{-10}\).
- Production-to-reference physical velocity residual: <= \(10^{-6}\) on the frozen small-island suite initially, tightening as the implementation matures.
- Coulomb cone violation: <= \(10^{-8}\) normalized for cone-reference paths.
- No unexplained positive friction work; friction impulse work must be non-positive within roundoff.
- Rollback: exact equality of physics-owned state and caches after injected failure.
- No NaN/Inf or singular response may be silently converted to a valid zero.

## Competitive targets (stretch goals, not current results)

Only after matching physical error and hardware:
- CPU small/medium rigid-body scenes: target >=1.5× throughput versus the strongest comparable Rust baseline, with equal or lower p95 penetration and no worse residual.
- Dense GPU contact scenes: target >=2× versus a matched GPU baseline at the same error envelope; compare directly with ComFree-Sim and Newton rather than assuming GPU wins.
- Hard scenes: cut time-to-target-residual by >=2× through adaptive solver selection, without reducing the target accuracy.
- 4D mechanics: provide validated D-dimensional mass, inertia and contact behavior over D=2,3,4 with all independent bivector planes tested.

A speedup counts only if the physical error envelope, timestep, contact semantics, precision, hardware, compiler flags and scene definition are matched. If no equivalent exists, call it a capability comparison, not a performance win.

## First friction reference increment

The cumulative branch now includes `solve_planar_friction_cone`, an exhaustive single-contact 2D reference for Signorini normal complementarity plus maximum-dissipation Coulomb friction. It enumerates separation, sticking, positive tangential sliding and negative tangential sliding. The effective metric must be SPD; inconsistent/Painleve-like cases return an explicit no-admissible-solution error. The implementation reports a normalized normal-complementarity/friction-law residual and is benchmarked separately from the Delassus/LCP reference.

This is intentionally **not** a 3D circular cone solver, multi-contact island solve, or 6D torsional/rolling friction implementation. The next acceptance gate is an independent 3D cone reference and production-to-reference contact-patch comparison.

## Immediate implementation sequence

1. Keep the small-island frictionless oracle independent and correct; keep the new planar friction reference as a separate correctness module, not a premature production replacement.
2. Define a typed island problem containing generalized mass/effective metric, Jacobian rows, normal/tangent bases, compliance and targets.
3. Implement one-contact 2D/3D maximum-dissipation reference and compare it with production friction response.
4. Add multi-contact cone reference fixtures with redundant rows and a clearly stated solver tolerance.
5. Instrument production TGS with work, residual and per-island timing receipts.
6. Add adaptive iteration/substep experiments driven by residual contraction and conditioning.
7. Only then implement production solver switching and GPU batching.

## Promotion rule

A solver strategy is promoted only when it:
- passes analytical, metamorphic and adversarial fixtures;
- beats the current baseline on time-to-accuracy, not just FPS;
- produces complete receipts for all declared work and observation channels;
- documents its validity envelope and unsupported regimes;
- reproduces at a pinned source/configuration SHA on supported targets.

No engine is the universal SOTA for all contact densities, friction models and hardware. Symtropy should win by publishing a more complete, reproducible evidence envelope while delivering competitive time-to-accuracy on clearly defined workloads.
