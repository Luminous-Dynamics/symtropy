# Physical Transformation Reference Kernel v0.1

**Tracking:** [PHYS-TRANSFORM-001 #2209](https://github.com/Luminous-Dynamics/symtropy/issues/2209)  
**Status:** initial dependency-free software reference with three analytical cases; not a general mechanics solver.

## What is implemented

- Transactional, revisioned events for component installation/removal, connection creation/removal, and support activation/release.
- Stable component/connection IDs, explicit material labels, and declared integer-milligram accounting fields.
- Deterministic connected-component analysis after structural-graph changes.
- A conserved mass ledger partitioned into reused, recycled, contaminated, waste, and unaccounted outputs.
- A state-bound `RecoverComponent` event that only recovers an existing component once it has no active support or connections; all output categories must sum exactly to the component's declared mass before the component leaves the active scene.
- A deterministic deconstruction preflight plan that rejects active supports and connections to non-target components, orders target-internal disconnections by ID, then recovery by component ID, and binds execution to the source revision and canonical source-state snapshot, preventing a plan from being replayed against a different world with the same numeric revision.
- Atomic plan execution: the canonical plan is recomputed, stale or tampered plans are rejected, and staged events are committed only if the entire operation succeeds.
- Three closed-form mechanics references: a uniform linear-elastic axial bar, a small-deflection Euler-Bernoulli cantilever under a tip point load, and a simply supported Euler-Bernoulli beam under a full-span uniform load.
- Explicit units, positive-domain checks, non-finite/overflow rejection, signed load behavior, and frozen known-answer outputs for stress, strain, extension, cantilever deflection, simply supported midspan deflection and bending stress.
- Frozen synthetic fixture: three topology components and two connections; disconnecting one edge changes graph connectivity from one connected component to two, and reconnecting it returns the graph to one. A separate component is then recovered through the state transition and its declared 1,000 mg reconciles exactly to recovery outputs.
- Negative controls for duplicate events, failed transactional mutation, dangling endpoints, self-connections, support state, connected-component recovery, unbalanced component recovery, standalone ledger imbalance, numeric overflow and malformed identifiers.
- The current focused crate suite contains 30 library unit tests, 3 truss-receipt CLI unit tests, and 4 integration oracle tests, plus compile-fail/doctest coverage where present. The counts are verified per exact-head CI run; mutation-harness runs are reported separately.
- Deterministic JSON receipt bound to the 40-character source commit supplied by the caller, including state-bound recovered-component count and mass.
- Planar truss reference: dependency-free 2D pin-jointed linear-elastic stiffness assembly, deterministic dense solve, closed-form symmetric V-truss oracle, explicit support reactions and normalized force/moment/free-DOF residuals, fail-closed singular mechanism detection, and scale-invariant pivot tolerance. Scope is capped at 128 nodes / 512 bars and excludes bending, buckling, nonlinear response, contact, fracture, dynamics, and physical validation.
- The planar-truss result now reports strain energy reconstructed from member force × extension and external work from applied nodal loads × displacements, each with joule units, an absolute residual, and a scale-normalized residual. The frozen V-truss oracle and all 128 sweep cases check that strain energy matches external work independently of force/moment equilibrium; the request-bound CLI refuses to publish PASS if energy balance is outside tolerance.
- The symmetric V-truss oracle also runs a fixed-seed 128-case parameter sweep across half-span, cross-sectional area, Young's modulus, and downward load. It checks apex symmetry/displacement, each member's extension/strain/force/stress, all support reactions, and normalized force/moment/free-DOF residuals; alternating cases reverse input storage order. The seed and positive numeric ranges are frozen in the test. This is repeatable numerical coverage, not a statistical sample or physical validation.
- Planar-truss mutation audit tests six deliberately defective variants (omitted area in stiffness, omitted area in member force, wrong stiffness span exponent, wrong strain-energy half-factor, reversed support-reaction sign, and an absolute pivot-tolerance floor that breaks scale invariance). Each intended test must fail in its isolated temporary copy. CI then re-runs the full unmodified suite, formatting and Clippy and publishes a source-bound mutation receipt.
- CI publishes a separate 128-case sweep receipt with the pinned seed, generator constants, exact source commit/hash, tested parameter ranges, 17 scalar assertions per case, and residual gates. It is emitted only after the full test suite, mutation gate, and post-mutation integrity rerun have passed; the receipt is evidence of regression-test execution, not physical validation.
- Frozen planar-truss request bytes live at `fixtures/planar_truss_request_v0_1.txt`; the canonical serializer is order-independent and tested against those exact bytes. The `planar-truss-reference` binary emits a deterministic truss receipt containing node displacements, member forces/stresses/strains, support reactions, normalized equilibrium residuals, source commit and the canonical request. CI computes SHA-256 over the canonical UTF-8 request bytes (without a trailing newline), verifies the pinned digest/oracle gates, and uploads the receipt alongside the topology/mass and mutation receipts. The physical-validation field remains `not_performed`.

## Run locally

```sh
cargo fmt --manifest-path research/physical-transformation-reference/Cargo.toml --all -- --check
cargo test --locked --manifest-path research/physical-transformation-reference/Cargo.toml
cargo clippy --locked --manifest-path research/physical-transformation-reference/Cargo.toml --all-targets -- -D warnings
cargo run --locked --manifest-path research/physical-transformation-reference/Cargo.toml --bin physical-transformation-reference -- \
  --source-commit "$(git rev-parse HEAD)"
cargo run --locked --manifest-path research/physical-transformation-reference/Cargo.toml --bin planar-truss-reference -- \
  --source-commit "$(git rev-parse HEAD)"
```

The default CLI emits the topology/material/analytical-reference receipt. The `planar-truss-reference` binary emits the frozen V-truss receipt with request canonicalization and equilibrium diagnostics; CI adds the SHA-256 request digest and pins the expected digest. Neither receipt is a claim of verified real-world evidence.

## Paradigm shift

Treat destruction, construction and repair as the same state-transition problem. A broken connection modifies topology, a removed component requires valid preconditions, and a rejected event cannot partially mutate the world. Material accounting follows the same principle: all outputs must reconcile to declared input, with unknown mass kept visible rather than disappearing into an aggregate “recycling rate.” A recovery event must refer to an actual component and satisfies topology preconditions before its mass is booked.

This is intentionally a small slice. Graph connectivity does not prove whether a building is stable, whether an element will actually fall, or how stress redistributes. The current mass fields are declared accounting values, not sensor measurements or verified material properties. The deconstruction planner is only a graph/state/accounting preflight: it cannot establish safe removal order, structural stability, load-path preservation, or demolition authorization. The three mechanics fixtures verify only their closed-form idealizations; they do not validate a mesh, nonlinear constitutive law, real connection behavior, dynamic collapse, or solver integration.

For the wider industry matrix and proposed numerical acceptance targets, see [PHYSICAL_TRANSFORMATION_SCORECARD_V0_1.md](../../docs/research/PHYSICAL_TRANSFORMATION_SCORECARD_V0_1.md).

## Comparison with the field

- **Computational solid mechanics:** ASME V&V 10 describes a framework for verification, validation and uncertainty quantification. Symtropy should adopt that separation rather than treating a passing test as physical validation: https://www.asme.org/codes-standards/find-codes-standards/standard-for-verification-and-validation-in-computational-solid-mechanics/2019/pdf
- **Fracture and multiphysics:** MOOSE provides phase-field fracture and coupled mechanics capabilities. A future Symtropy adapter must name its constitutive assumptions, mesh, convergence policy and benchmark rather than claiming equivalent fidelity from visuals: https://mooseframework.inl.gov/modules/phase_field/
- **Large granular/contact simulations:** Project Chrono documents GPU-accelerated DEM for granular dynamics, including workloads at scales far beyond this kernel. This first Rust crate is not a competing particle engine; it is a state/evidence/control layer that can later sit above or beside specialized solvers: https://api.projectchrono.org/module_dem_installation.html
- **Interoperability:** IFC 4.3.2.0 is the official buildingSMART schema release associated with ISO 16739-1:2024. A later adapter can preserve element identities and report import/export losses, without claiming IFC semantics are physical validation: https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/index.html
- **Measurement culture:** NIST AM-Bench is a useful model for publishing high-quality measured challenges and reproducible simulation comparisons; its domain is additive manufacturing, so it is a benchmark-design inspiration, not a demolition reference dataset: https://www.nist.gov/ambench

## Benchmark ladder

1. **Software invariants (implemented slice):** deterministic replay, transactional rejection, graph connectivity and exact integer mass balance.
2. **Analytical mechanics (initial slice implemented):** unit-aware axial bar, cantilever tip load, and simply supported uniform-load cases with independently pinned expected values and declared tolerances.
3. **Solver adapter:** pin solver/version, boundary conditions, mesh and convergence information; report error and residuals.
4. **Physical fracture/contact:** use a rights-cleared measured dataset or controlled instrumented experiment, including uncertainty and held-out cases.
5. **Construction planning:** temporary supports, connection prerequisites, assembly sequence and tool accessibility.
6. **Material circularity:** measured or provenance-bound material lots, recovery yields and chain of custody.
7. **Sim-to-real:** compare forecast distributions and failure modes against observation, and abstain outside qualified domains.

## Scorecard (report dimensions independently)

- Topology and transition correctness.
- Analytical force/displacement error and conservation residuals.
- Rejected invalid-input rate and false-accept rate.
- Deterministic replay and receipt stability.
- Runtime/memory versus number of components and constraints.
- Fragment distribution/contact error only where measured ground truth exists.
- Construction sequence feasibility and unresolved prerequisite count.
- Verified recovered mass, with unmeasured/unknown mass stated separately.
- Uncertainty coverage and abstention rate.

Do not reduce all of these to a single “realism” number. Do not claim state-of-the-art superiority until an openly described, matched evaluation against strong eligible baselines demonstrates it.

## Planar truss reference v0.1

The new `planar_truss` module adds a dependency-free, dense-stiffness reference for 2D pin-jointed axial bars under small-displacement linear elasticity. It accepts node coordinates, per-DOF restraints and loads, and bar area/Young's modulus. Results include nodal displacement, signed member axial force/strain/stress, support reactions, normalized force/moment equilibrium residuals, and the maximum free-DOF residual.

A frozen symmetric two-bar V-truss has an independent closed-form oracle. Further tests cover input-order invariance, load/stiffness scaling, a mechanism/singular reduced stiffness system, duplicated IDs, isolated nodes, coincident endpoints, invalid IDs, non-finite loads and stiffness overflow. A scale-invariance control guards against incorrectly rejecting a valid matrix simply because its stiffness units are numerically small.

The implementation intentionally caps the dense reference at 128 nodes / 512 bars. It is a small benchmark/oracle model, not a large-scale FEM solver; dense assembly and Gaussian elimination are not suitable as the production-scale route. It excludes member bending, buckling, geometric/material nonlinearity, contact, fracture and dynamics. Passing this benchmark qualifies only the named idealized mathematical model—not a real structure or the external SOFiSTiK BE1 benchmark.

## Modal dynamics: separate verification contract

The static planar-truss solver does **not** implement free-vibration or modal analysis. The axial-bar reference model now has a fixed-seed 128-case sweep against an independent closed-form discrete spectrum, area/inverse-length scaling checks, sine-mode shape MAC, eigenpair residuals and mass normalization. [PR #2556](https://github.com/Luminous-Dynamics/symtropy/pull/2556) reproduces the published NAFEMS NBC05 CS1/CS2 *single-element-per-member axial-bar frequency table* within a declared (10^{-4}) relative tolerance and has an exact-head passing receipt/mutation run; that is a narrow published-table reproduction, not full structural dynamics.

The released-joint beam path is intentionally staged:
- [PR #2559](https://github.com/Luminous-Dynamics/symtropy/pull/2559) passed its exact-head element-matrix tests for Euler-Bernoulli axial/bending stiffness, consistent axial/transverse mass, separately inspectable rotary inertia, and cantilever compliance.
- [PR #2560](https://github.com/Luminous-Dynamics/symtropy/pull/2560) adds local-to-global transformations with rigid-body and strain-energy invariance checks; its current exact-head qualification remains separate from the local element pass.
- [PR #2561](https://github.com/Luminous-Dynamics/symtropy/pull/2561) adds a deterministic release-aware assembly in which joint translations are shared but original member-end rotations are private. [PR #2638](https://github.com/Luminous-Dynamics/symtropy/pull/2638) subdivides continuous members without introducing artificial pins at interior mesh stations.
- [PR #2636](https://github.com/Luminous-Dynamics/symtropy/pull/2636) proposes a generalized modal spectrum with residual/orthogonality diagnostics and fail-closed mechanism detection.
- [PR #2639](https://github.com/Luminous-Dynamics/symtropy/pull/2639) records a 1/2/4/8-elements-per-member frequency trend. This is not yet mode-shape correspondence or a 200-elements/member reproduction.
- [PR #2641](https://github.com/Luminous-Dynamics/symtropy/pull/2641) adds cross-mesh MAC on exact shared stations, separate translation/rotation MAC, pinned reference-length scaling for their combined score, and Hungarian maximum-weight mode assignment. [PR #2644](https://github.com/Luminous-Dynamics/symtropy/pull/2644) adds principal-angle subspace MAC so near-repeated modes can be compared by their span even when individual eigenvectors rotate or swap. Unresolved individual MAC stays null; rank-deficient subspace samples stay unresolved rather than being assigned a fake score.

The [beam-fidelity contract](benchmarks/NAFEMS_NBC05_BEAM_FIDELITY_V0_1.md) records the next acceptance target: shape-interpolated/subspace MAC for closely spaced or repeated modes, sparse generalized eigensolving at higher mesh density, convergence against the source's beam model, and eventually an independent solver or eligible experiment. The official NAFEMS solution shows why this matters: axial-only modes omit bending behaviour, and its reported 200-elements-per-member beam model recovered 22 modes below 1600 Hz versus only two axial-bar modes. No full NAFEMS reproduction, experimental validation, structural-safety assessment or code-compliance claim is implied.

A separate [beam-theory applicability screen](benchmarks/BEAM_THEORY_APPLICABILITY_V0_1.md) now encodes two dimensionless gates: cross-section size relative to member / shortest relevant modal wavelength, and the reference shear-slenderness indicator G*A*L_eff^2/(E*I). It has explicit candidate / compare / outside-screen / missing-wavelength statuses, five frozen request cases, deterministic receipt generation, and mutation gates for wavelength selection and the thresholds. The result only recommends an analysis model; it does not quantify actual solver error. The 3/S cantilever metric is labelled an uncorrected proxy that omits section shear-correction factors.

A separate static [Timoshenko cantilever comparison](benchmarks/TIMOSHENKO_STATIC_COMPARISON_V0_1.md) now provides a local stiffness element with explicit shear correction, exact cantilever bending-plus-shear response checks, and a three-case shear-slenderness ladder (25/50/100) targeting deflection ratios 1.12/1.06/1.03. It remains a static element reference only: no Timoshenko mass matrix or modal solve has been qualified.

## Safety and claim ceiling

Offline research only. This crate cannot authorize or control demolition/construction equipment, certify a structure, interpret building codes, or predict collapse. Real-site decisions require validated physics, qualified engineering review, applicable approvals, and independently assessed hazard controls.
