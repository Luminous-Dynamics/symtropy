# Symtropy Physics VVUQ Reference Seed v0.1

**Status:** focused regression seed. Not full-engine validation, physical qualification, or external certification.

**Tracking issue:** [#7621](https://github.com/Luminous-Dynamics/symtropy/issues/7621).

## Purpose

Turn two requirements already stated in `RESEARCH_VALIDATION.md` into exact-head, CI-runnable references:

1. Numerical solution verification by comparing free fall to a closed-form solution and the exact discrete update of the integrator.
2. Geometry verification by comparing transformed 2D and 3D OBB GJK classification with independent 4-axis and 15-axis separating-axis references, including randomized, metamorphic and boundary-focused controls.

The executable tests are in `crates/core/symtropy-physics/tests/vvuq_reference.rs`. The current suite contains a 2D and a 3D randomized corpus, common-rigid-motion metamorphic tests, and signed-margin contact-boundary sweeps.

## Track A — Semi-implicit Euler convergence

For constant acceleration (g), semi-implicit Euler uses:

`v_{n+1}=v_n+g\,\Delta t`, `x_{n+1}=x_n+v_{n+1}\,\Delta t`.

At fixed duration (T=N\Delta t), the velocity matches the continuous constant-acceleration solution in exact arithmetic. Its position difference from the closed form is:

`x_N-x(T)=\tfrac12 gT\Delta t`.

This gives an exact scheme-specific reference error, not a fitted threshold. The seed tests (\Delta t\in\{1/30,1/60,1/120,1/240\}), checks the discrete error to (10^{-9}), velocity error to (10^{-10}), and first-order convergence ratios to (10^{-8}).

The test verifies the current semi-implicit Euler translational path only. It does not validate contacts, friction, rotational dynamics, long-term energy behavior, or other integrators.

## Track B — Independent transformed 2D OBB oracle

The reference is a standalone 2D separating-axis test over the four local face axes (two axes per box). For a rectangle with half-extents ((h_x,h_y)), rotated local unit axes ((u,v)), and test axis (a), its projection radius is:

`r(a)=h_x|u\cdot a|+h_y|v\cdot a|`.

The pair overlaps exactly when centre separation on every candidate axis is no greater than the sum of the two projection radii, within the declared (10^{-12}) reference tolerance.

The Rust test generates 4,096 deterministic cases from seed `0x4551435656553031`, with fixed half-extents and varied rotations/translations. It compares this reference to the public transformed-GJK API and records false-positive/false-negative counts as hard test failures. A translation-only GJK path is evaluated as a negative control; the corpus must produce more than 10% disagreements, proving that the test can detect ignored rotations.

The test deliberately avoids calling Symtropy support maps or GJK from the reference implementation. It is still a synthetic mathematical reference, not a comparison against a physical experiment or an external engine.

## Reproduction

```bash
cargo test --locked -p symtropy-physics --test vvuq_reference -- --nocapture
```

The focused workflow pins the repository Rust toolchain, runs formatting and this integration test on the exact triggering revision, and preserves the test log plus source revision as a CI artifact.

## Track C — Contact-boundary and argument-order controls

The two randomized OBB corpora are complemented by deliberately constructed contact cases. For each fixed rotation, the second box is translated along the shared local x-axis by the sum of the boxes' x half-extents plus a signed margin:

- `-1e-6`: shallow overlap;
- `0`: exact face contact, treated as intersecting by the SAT reference;
- `+1e-6`: shallow separation.

The 2D sweep covers eight orientations (24 cases); the 3D sweep covers six composed rotations (18 cases). Each case is checked against its independent SAT oracle in both argument orders, so the test covers boundary classification and collision symmetry. The margin is intentionally much larger than the SAT numerical epsilon and GJK's negative-support cutoff; this tests near-boundary classification without pretending to measure sub-machine-precision contact robustness.

## Industry comparison and next gates

- Rapier, Jolt or MuJoCo comparisons should be added only after matching shape definitions, transforms, tolerance rules, solver/contact semantics and versions.
- Next geometry tiers: 3D 15-axis OBB SAT, convex polygon/hull pairs, degeneracy and grazing corpus, EPA depth and witness error, then public analytic TOI/CCD datasets.
- Next VVUQ tiers: oscillator/pendulum reference, energy/momentum budgets for closed systems, constrained-system residuals, independent inertia/asymmetric-top solutions, and finally blind experimental points with measurement uncertainty.

ASME V&V 20 requires simulation accuracy to be assessed at specified validation points while accounting for both numerical and experimental uncertainty; VVUQ 20.1-2024 offers a multivariate metric for a global discrepancy assessment across observables and operating points. These two synthetic seed tests cover only analytical/code verification for two narrow behaviors; they cannot establish experimental physical validation or a global model-validity claim.

## Evidence to publish

For each exact-head run, preserve the source commit, toolchain, seed, case count, reference algorithm version, tolerances, false-positive/negative counts, translation-only negative-control count, and raw test log. If the corpus changes, increment its version and retain the prior seed for regression comparison.

## Additional 3D narrowphase verification (v0.2 extension)

The test suite now includes a separate 3D OBB corpus, not just the 2D seed:

- 4,096 deterministic pairs using fixed half-extents and arbitrary composed 3D rotations in XY, YZ and XZ planes. Pair centres are sampled inside 90% of the unrotated AABB-overlap band, so the translation-only negative control predicts overlap for every case and the corpus stresses orientation-sensitive separations.
- Independent 15-axis separating-axis theorem: three face axes from each box plus nine pairwise edge cross products.
- The oracle uses only rotated basis vectors, projection radii and scalar dot/cross products; it does not call the engine's support-map or GJK implementation.
- The transformed public GJK API is compared to the SAT oracle, with false-positive and false-negative counts accumulated separately.
- A translation-only implementation is retained as a negative control and must disagree on more than 10% of the corpus.

This extends the narrowphase reference coverage from 2D to 3D. It still does not qualify 4D simplex logic, EPA penetration depth, continuous collision detection, contact manifolds, or physical experiments. The next N-dimensional stage should have its own independent oracle (for example, bounded convex polytopes and linear-program feasibility), rather than extrapolating 3D SAT.

## Industry benchmark snapshot and fair-comparison rules (2026-11-27)

The current public Rapier/Godot performance page reports **p50 physics-step time on identical scenes, release builds, Godot 4.7, an M1 machine, and rendering excluded**. Its published rows are a useful scene-level comparator:

| Scene | Rapier2D | Box2D v3 | Rapier3D | Jolt |
|---|---:|---:|---:|---:|
| 10k-body 2D box pyramid | 15.6 ms | 17.0 ms | 20.6 ms | 25.1 ms |
| Mixed-shape pile | 6.9 ms | 5.5 ms | 10.8 ms | 12.4 ms |
| Joint grid | 6.4 ms | 3.9 ms | 11.5 ms | Timeout |
| Smash (3k bodies) | 7.8 ms | 8.9 ms | 13.3 ms | 12.6 ms |
| Query storm | 5.7 ms | 4.0 ms | 5.7 ms | 22.3 ms |

The Jolt joint-grid timeout is a failed/unsupported result and must stay visible rather than being omitted from an aggregate. These are Godot-integrated scene results, not isolated engine microbenchmarks and not directly comparable with Symtropy's current GJK nanosecond measurements. Reproduce the same scenes and hardware before making a relative-performance claim. See the [published benchmark table](https://godot.rapier.rs/docs/documentation/performance/).

Jolt 5.5 documents an optional cross-platform deterministic build mode with an approximately 8% performance cost. Its documentation explicitly excludes broadphase queries from the guarantee because the broadphase is mutated from multiple threads; narrowphase result order can also vary even when the returned narrowphase results are consistent. It also documents world-scale precision limitations caused by float-based broadphase bounds. This is a valuable comparator for Symtropy's determinism and large-world work: publish determinism coverage by API, not as one global Boolean. See [Jolt 5.5 architecture and determinism notes](https://jrouwe.github.io/JoltPhysicsDocs/5.5.0/).

For scientific VVUQ, ASME V&V 20-2009 is the pointwise CFD/heat-transfer validation reference; **ASME VVUQ 20.1-2024** adds a multivariate metric for global discrepancies across several variables, positions and times, accounting for shared or independent uncertainties. Symtropy should progress from isolated analytical point checks to multivariate validation envelopes over several observables and operating points. See [ASME V&V 20](https://www.asme.org/codes-standards/find-codes-standards/standard-for-verification-and-validation-in-computational-fluid-dynamics-and-heat-transfer) and [ASME VVUQ 20.1-2024](https://www.asme.org/codes-standards/find-codes-standards/vvuq201-vvuq-20-1-multivariate-metric-for-validation).

Dimforge's Q3 2026 report describes recent Rapier/Nexus performance work and discusses scaling limits reported for MuJoCo, MJX, Newton and Genesis in the workloads it measured. This is a vendor engineering report, not an independent cross-engine adjudication; treat it as hypothesis-generating evidence and rerun matched workloads before using it for comparative claims. See [Dimforge Q3 2026 technical report](https://dimforge.com/blog/2026/10/25/dimforge-Q3-technical-report/).

### Symtropy comparison ladder

1. **Core microbenchmarks:** keep Criterion GJK/EPA/raycast/step microbenchmarks, but publish CPU, compiler, features, body distribution, build mode, warm-up, sample count, p50/p95/p99 and allocation metrics.
2. **Matched engine scenes:** port the same frozen scene definitions to Symtropy, Rapier, Jolt and Box2D where the dimension and semantics overlap. Match shapes, masses, friction/restitution, timestep, solver iterations, sleeping, contact filters, CCD mode, thread count, precision and termination criteria. Include both engines' wrapper/integration overhead separately.
3. **Correctness and stability gates:** compare contact sets and collision classifications against independent analytical/brute-force oracles; report tunneling, solver divergence, residuals, non-finite states, stack collapse, joint explosion and timeouts. A faster unstable run is not a win.
4. **Scale/latency frontier:** report p50/p95/p99 full-tick latency, throughput, maximum stable population under a declared error budget, memory, allocations, energy and warm/cold startup.
5. **Determinism frontier:** compare exact state hashes and replay agreement across repeated runs, compilers, build profiles, OS and CPU architectures; report APIs excluded from deterministic guarantees.

Symtropy already has dedicated internal AABB broadphase tournaments (#7588 / #7585) and a separate public analytic TOI/CCD qualification proposal (#7593). This VVUQ seed complements those lanes by checking numerical integration and transformed narrowphase geometry; it must not duplicate broadphase or imply that current-frame AABB candidate correctness establishes swept-collision correctness.

### Proposed comparative targets (research targets, not achieved)

- On the shared 2D/3D rigid-body subset, first match Rapier/Jolt correctness and stability contracts. Only then target within 10% of the median p95 tick time on at least two matched workloads.
- On the N-dimensional differentiator (especially 4D), target a distinct capability frontier: validated reference error and deterministic replay on a supported subset, rather than claiming superiority in 3D game-physics throughput.
- For every promotion, require zero false negatives against the benchmark's independent reference corpus and no regression beyond a preregistered tolerance on held-out scenes.
- Publish the whole Pareto frontier: accuracy/robustness, p95/p99 latency, throughput, memory, allocations, energy and deterministic coverage. Do not rank an implementation on mean FPS alone.

## References

- [Symtropy research validation envelope](../../crates/core/symtropy-physics/RESEARCH_VALIDATION.md).
- [Orientation-aware collision validation limits](../../crates/core/symtropy-physics/ORIENTED_COLLISION_VALIDATION.md).
- [ASME V&V 20: Verification and Validation in CFD and Heat Transfer](https://www.asme.org/codes-standards/find-codes-standards/standard-for-verification-and-validation-in-computational-fluid-dynamics-and-heat-transfer).

- [ASME VVUQ 20.1-2024: Multivariate Metric for Validation](https://www.asme.org/codes-standards/find-codes-standards/vvuq201-vvuq-20-1-multivariate-metric-for-validation).
- [Jolt Physics 5.5 determinism and limitations](https://jrouwe.github.io/JoltPhysicsDocs/5.5.0/).
- [Rapier/Godot scene benchmark](https://godot.rapier.rs/docs/documentation/performance/).
- [NASA CFD verification assessment](https://www.grc.nasa.gov/www/wind/valid/tutorial/verassess.html).
