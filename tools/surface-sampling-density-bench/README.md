# Surface Sampling Density Benchmark v0.1

This standalone Rust crate implements a synthetic accounting contract for ray-intersection density over non-uniform surface elements. It is deliberately separate from orbital binary/k-view coverage, support-envelope geometry, feature-measurability conditioning and metrological uncertainty.

## Definition

For surface element i with area A_i and candidate viewpoint set V, let P_ij be the number of admitted ray intersections contributed by view j. For selected subset S:

rho_i(S) = sum(P_ij for j in S) / A_i

The fixture stores area in square micrometres and target density in micro-rays per square millimetre. The exact rational density in micro-rays/mm² is `(hits × 10^12) / area_um2`; the threshold is checked exactly using integer arithmetic:

hits × 10^12 >= target_micro_rays_per_mm2 × area_um2

The receipt exposes the exact rational density numerator/denominator, selected and candidate hit totals, number of contributing views, per-view contributions, density threshold attainment, minimum-contributing-view attainment and input provenance. A high-density small element cannot compensate for an under-dense large or critical element; all elements must independently meet their declared target.

## Outcome semantics

- FEASIBLE: every selected element meets its density and distinct contributing-view targets, the declared view budget is respected, and all ray-count rows are exact synthetic-oracle inputs.
- INFEASIBLE: the full candidate pool cannot supply the target density or number of contributing views for at least one element.
- BUDGET_INFEASIBLE: the candidate pool can supply the targets, but the selected plan or declared view budget does not.
- INDETERMINATE: at least one ray-count row has caller-supplied-unverified provenance.
- INVALID_INPUT: element area/identity/target, view IDs, matrix completeness, units or collection shape is invalid.

Candidate-pool insufficiency, selected-plan insufficiency, budget excess and unverified ray provenance remain separate receipt fields and reason codes.

## Frozen corpus

The thirteen controls include equal-area/equal-hit equality, tenfold area scaling, additive multi-view sampling, unselected-view exclusion, candidate-pool insufficiency, selected-plan insufficiency, unverified provenance, zero-area invalid input, no averaging away a critical under-sampled element, k-view count separated from density, multi-element scaling and row-order invariance, and a requested contributing-view count above the candidate pool (reported as infeasible, not invalid input).

## Reproduction

    cargo test --locked --manifest-path tools/surface-sampling-density-bench/Cargo.toml --all-targets
    cargo clippy --locked --manifest-path tools/surface-sampling-density-bench/Cargo.toml --all-targets -- -D warnings
    cargo run --quiet --locked --manifest-path tools/surface-sampling-density-bench/Cargo.toml --bin sampling-density-bench -- docs/research/orbital-truss/fixtures/sampling-density-v0_1.json <40-hex-source-commit>
    python3 tools/surface-sampling-density-bench/tests/sampling_density_reference_oracle.py docs/research/orbital-truss/fixtures/sampling-density-v0_1.json <40-hex-source-commit>

The workflow compares the full Rust receipt with the independent standard-library Python oracle and retains exact-head artifacts. A standalone lockfile is committed.

## Claim ceiling

The current fixtures provide ray-intersection counts as synthetic inputs. This crate accounts for those counts; it does not generate camera rays, intersect a physical mesh, simulate a sensor, establish a real point-cloud sampling density, infer resolvable defect size, estimate covariance, compute probability of detection or qualify a measurement. EXACT_SYNTHETIC_ORACLE means exact relative to the declared synthetic fixture, not experimentally verified ray physics.

The principal literature comparator is Staderini, Integrated Planning and Optimization Framework for Robotic Visual Inspection, TU Wien doctoral dissertation submitted 1 December 2025, Chapter 4's sampling-density matrix. The current crate is a small contract/oracle benchmark, not a reproduction of the thesis' full geometry, camera-ray generation, surface-sampling or robot planning pipeline.


## Contract correction

v0.1 receipts now use the correct unit-scaled rational numerator (hits × 10^12), consistent with the exact density threshold. A requested minimum contributing-view count greater than the candidate pool is a valid but infeasible requirement, not malformed input. Likewise, a view-budget ceiling above the candidate count is harmless and no longer invalidates the case.


## Geometry-generated ray counts v0.1

The companion \`ray_geometry\` module and \`sampling-ray-generator-bench\` CLI generate a ray-intersection count matrix from explicit synthetic triangle meshes and pinhole pixel-centre rays. Each camera frame must be orthonormal and right-handed; each ray is tested against every non-degenerate triangle using two-sided Möller–Trumbore intersection. The nearest hit owns a pixel. Shared-edge ties within a declared 1 nm distance tolerance are counted once and resolved by stable \`(element_id, triangle_id)\` order.

The four fixtures cover a known 3×3 triangle projection (five hits), front/rear occlusion (the rear surface receives zero hits), two separately identified views, and a square split into two triangles where shared-diagonal pixels are counted once (four hits, two tie pixels).

Element areas remain caller-declared synthetic metadata; this generator does not derive area uncertainty, model finite pixel footprints, lens distortion, radiometry, structured-light projected patterns or sensor noise. Its counts are exact only relative to the stated ideal pinhole/pixel-centre/triangle model. The independent Python oracle implements the projection and triangle-intersection calculations separately; it is a test oracle, not production code.
