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

## Industry landscape as of 2026-10-31

This is a comparison map, not one universal leaderboard. The systems solve overlapping but non-identical contact models, scenes and hardware targets.

| System | Current relevant capability | Implication for Symtropy |
|---|---|---|
| MuJoCo 3.17.0 | Released 22 October 2026. Adds batched stepping for many copies of one model; its release notes state that batch output is bit-identical to a plain loop across thread counts. Its `discrete` integrator shares the effective metric between constraint solve and velocity update; the documentation still describes this method as new/under active development. | Benchmark both common-metric accuracy and batched determinism/throughput. Do not claim novelty merely for a shared effective metric. |
| Rapier 0.35.3 | Rust CPU/SIMD rigid-body baseline. Dimforge's 25 October 2026 report describes intra-island parallelism, persistent graph coloring and parallel broad-phase, beyond only scheduling independent islands. The report gives up to 2× single-thread and 4× multi-thread improvement versus its earlier baseline on an M4 Max with 8 workers; that is a vendor's before/after result, not an independent cross-engine ranking. | Mandatory Rust CPU baseline. Compare dense islands, not just independent stacks. |
| Nexus 0.7.0 | Rust-authored GPU shaders; Soft-TGS and twist-friction approximation. Dimforge reports a 505,000-body pyramid at about 14 ms/frame on an RTX 5080, without sleeping-island optimization. It also reports early 2D/3D MPM and CUDA backend performance. | Serious GPU scale reference, but reproduce the exact scene, hardware, accuracy, and contact law. The vendor figure is not an apples-to-apples result for Symtropy. |
| Physmark | Open benchmark suite with rigid-body piles/stacks, many-environment robotics and MPM. Its public README lists scenes from 8 to 10,000 bodies, large brick scenes at 110,000 and 505,922 bodies, MPM 512–195,112 particles, and Go1/T1 robots in 1–8,192 environments. The runner chooses compatible backends, probes engines, and records videos; non-compatible engines may be skipped rather than silently assigned another backend. | Adopt its workload families and provenance rules. Extend with asymmetric inertia, physical residuals, N-D contact frames and source/configuration receipts. |
| NVIDIA PhysX 5.8 | PGS and TGS, with TGS documented as the generally recommended solver. NVIDIA cites better convergence, high mass-ratio handling, joint-drive accuracy and high-frequency behavior. | Include TGS, friction patches, position/velocity iteration splits, high-mass-ratio and joint-drive tests. |
| Newton Physics | Open-source GPU-accelerated differentiable robotics engine, based on Warp/OpenUSD, with multiple solver families. | Essential robotics/GPU baseline; not an apples-to-apples CPU Rust comparison. |
| ComFree-Sim (2026 arXiv preprint) | Proposes a GPU-parallelized complementarity-free analytical contact update using impedance-style prediction/correction in the dual Coulomb cone; reports unified 6D tangential/torsional/rolling friction, near-linear contact scaling and 2–3× dense-scene throughput versus MJWarp with comparable physical fidelity. Those are the paper authors' experimental claims, not independently reproduced results. | Serious alternative contact formulation. Reproduce matched cases; compare its impedance/dual-cone law separately from hard Signorini plus maximum-dissipation Coulomb. |
| Project Chrono 10.0.0 | Mature engineering/multiphysics reference with multibody dynamics, FSI, peridynamics, checkpointing and implicit integration/Jacobian strategies. | Use for selected engineering verification cases rather than real-time rigid-body ranking. |

Primary reading:
- [MuJoCo 3.17.0 release](https://github.com/google-deepmind/mujoco/releases/tag/3.17.0)
- [MuJoCo computation/integrator docs](https://mujoco.readthedocs.io/en/stable/computation/index.html)
- [Rapier changelog](https://github.com/dimforge/rapier/blob/master/CHANGELOG.md)
- [Dimforge Q3 2026 technical report](https://dimforge.com/blog/2026/10/25/dimforge-Q3-technical-report/)
- [Physmark benchmark repository and scene matrix](https://github.com/dimforge/physmark)
- [PhysX 5.8 simulation docs](https://nvidia-omniverse.github.io/PhysX/physx/5.8.0/docs/Simulation.html)
- [NVIDIA Newton](https://developer.nvidia.com/newton-physics)
- [ComFree-Sim arXiv preprint 2603.12185](https://arxiv.org/abs/2603.12185)
- [Project Chrono release news](https://projectchrono.org/news/)

## Contact-law semantics: do not mix unlike models

A solver comparison is meaningful only after the contact law is stated. These are distinct model families, not interchangeable implementations of one exact law:

| Contact model | Mathematical contract | Typical trade-off |
|---|---|---|
| Signorini + maximum-dissipation Coulomb | Unilateral normal complementarity; tangential impulse lies in a Coulomb disk/cone and maximally dissipates slip. The new planar oracle covers a single 2D contact. | Strong hard-contact reference, but non-smooth and costly as coupled contact count grows. |
| Convex/compliant soft contact | Soft penetration/impedance objective; strict complementarity may be relaxed. MuJoCo documents that normal force and separating normal velocity can both be positive in this model, and friction need not be maximally dissipative. | Smoothness, optimization and practical soft-contact behavior; not a like-for-like reference to hard Signorini-Coulomb. |
| Complementarity-free dual-cone impedance | Predict smooth velocity, then correct violations of dual-cone contact constraints with stiffness/damping-like impedance; polyhedral facets approximate circular cones. ComFree-Sim extends this to tangential, torsional and rolling friction. | Contact-pair/facet parallelism and GPU throughput; fidelity depends on facet resolution, impedance and compliance settings. |
| Temporal Gauss-Seidel / substepping | Solve constraints over temporal substeps with updated geometry/velocity. | More stable penetration correction and convergence at higher per-iteration cost. |

Therefore every cross-engine campaign needs two tracks:
1. **Matched-law comparisons**, where the engine can be configured to approximate the same hard or compliant law.
2. **Best-native-model comparisons**, where each engine uses its preferred contact formulation and all physical metrics are reported without pretending the models are identical.

The planar reference is not an oracle for MuJoCo's soft convex model or ComFree-Sim's impedance model. Instead, create dedicated expected envelopes for each law and measure the trade-offs explicitly.

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

### 2.5 In-tree contact load fixtures

The Criterion `physics_bench` target now includes two engine-step workloads:
- `contact_plane_1000_independent_3d`: 1,000 dynamic spheres in a grid, all contacting one static half-space; independent dynamic contact islands.
- `contact_stack_100_single_island_3d`: 100 overlapping spheres in one vertical stack; strongly coupled contact island.

Both use the same 1/120 s step, eight solver iterations, and assert the expected contact count and finite post-step state while black-boxing contact count, penetration and mechanical energy. Scene construction is outside the measured iteration. These are in-tree baselines only; results still need to be run on pinned hardware before comparison with other engines. The executable `cargo run --release -p symtropy-physics --example contact_quality_campaign` emits CSV with median/p95 per-step time, final contacts, max penetration, mechanical-energy change, rotation error and health counters. Set `SYMTROPY_SOURCE_REV`, `SYMTROPY_BENCHMARK_HOST`, `SYMTROPY_BENCHMARK_CPU` and `SYMTROPY_BENCHMARK_BUILD_FLAGS` at compile time to bind each record to its source and environment.

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
