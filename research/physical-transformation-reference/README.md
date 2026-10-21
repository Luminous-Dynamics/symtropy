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
- The current focused crate suite contains 37 library unit tests, 3 truss-receipt CLI unit tests, 3 modal-receipt CLI unit tests, and 4 integration oracle tests, plus compile-fail/doctest coverage where present. The counts are verified per exact-head CI run; mutation-harness runs are reported separately.
- Deterministic JSON receipt bound to the 40-character source commit supplied by the caller, including state-bound recovered-component count and mass.
- Planar truss reference: dependency-free 2D pin-jointed linear-elastic stiffness assembly, deterministic dense solve, closed-form symmetric V-truss oracle, explicit support reactions and normalized force/moment/free-DOF residuals, fail-closed singular mechanism detection, and scale-invariant pivot tolerance. Scope is capped at 128 nodes / 512 bars and excludes bending, buckling, nonlinear response, contact, fracture, dynamics, and physical validation.
- A second independent known-answer case uses asymmetric bar geometry, unequal axial stiffness, and mixed horizontal/vertical load. It pins both apex displacement components, signed member responses, support reactions, equilibrium residuals, and energy/work balance. It specifically exercises the stiffness cross-coupling terms that cancel under the symmetric vertical-load fixture; a zeroed-{cs} mutation is required to fail this oracle.
- The planar-truss result now reports strain energy reconstructed from member force × extension and external work from applied nodal loads × displacements, each with joule units, an absolute residual, and a scale-normalized residual. The frozen V-truss oracle and all 128 sweep cases check that strain energy matches external work independently of force/moment equilibrium; the request-bound CLI refuses to publish PASS if energy balance is outside tolerance.
- The symmetric V-truss oracle also runs a fixed-seed 128-case parameter sweep across half-span, cross-sectional area, Young's modulus, and downward load. It checks apex symmetry/displacement, each member's extension/strain/force/stress, all support reactions, and normalized force/moment/free-DOF residuals; alternating cases reverse input storage order. The seed and positive numeric ranges are frozen in the test. This is repeatable numerical coverage, not a statistical sample or physical validation.
- Planar-truss mutation audit tests seven deliberately defective variants: omitted area in stiffness, omitted area in member force, wrong stiffness span exponent, wrong strain-energy half-factor, zeroed off-diagonal direction-cosine coupling, reversed support-reaction sign, and an absolute pivot-tolerance floor that breaks scale invariance. Each intended test must fail in its isolated temporary copy. CI then re-runs the full unmodified suite, formatting and Clippy and publishes a source-bound mutation receipt.
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
cargo run --locked --manifest-path research/physical-transformation-reference/Cargo.toml --bin modal-bar-reference -- \
  --source-commit "$(git rev-parse HEAD)"
bash research/physical-transformation-reference/scripts/verify_modal_bar_mutations.sh
```

The default CLI emits the topology/material/analytical-reference receipt. The `planar-truss-reference` binary emits the frozen V-truss receipt with request canonicalization and equilibrium diagnostics. The `modal-bar-reference` binary parses its seven-field frozen request, derives area from the declared radius, evaluates both mass formulations, and emits the 1/2/4/8/16-element convergence ladder. CI hashes the canonical request and source tree, requires repeated receipts to be byte-identical, and uploads the receipt. A separate isolated-mutant harness targets five modal formula/numerical defects; it must show the original modal test suite passing and each mutant detected. These receipts are software verification evidence, not physical validation or a full NAFEMS Challenge 5 reproduction.

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
2. **Analytical mechanics (initial slice implemented):** unit-aware axial bar, cantilever tip load, and simply supported uniform-load cases with independently pinned expected values and declared tolerances. A separate modal axial-bar subproblem now checks consistent/lumped mass, exact frequency limits, and two-mode mesh convergence; the complete 2D NAFEMS Challenge 5 model is still future work.
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

## Safety and claim ceiling

Offline research only. This crate cannot authorize or control demolition/construction equipment, certify a structure, interpret building codes, or predict collapse. Real-site decisions require validated physics, qualified engineering review, applicable approvals, and independently assessed hazard controls.

## Modal mechanics slice: fixed-free axial bar

The `modal_bar` module adds a separate, deliberately small generalized-eigenvalue reference for a uniform 1D axial bar fixed at one end and free at the other. It assembles the reduced axial stiffness matrix and offers both consistent and row-sum-lumped mass formulations, then solves the small symmetric generalized eigenproblem through a Cholesky reduction and Jacobi eigensolver. The model is capped at 32 elements.

For the continuum model, the mode-n frequency is

`f_n = (2n - 1) / (4 L) * sqrt(E / rho)`.

For the NAFEMS Challenge 5 bar parameters (`L=1 m`, `r=0.01 m`, `E=210 GPa`, `rho=7800 kg/m^3`), the one-element test has independently derived frequency ratios `2*sqrt(3)/pi` for consistent mass and `2*sqrt(2)/pi` for row-sum-lumped mass. These bracket the continuum frequency from opposite sides. The tests sweep 1, 2, 4, 8 and 16 elements, check first- and second-mode convergence, and exercise area invariance, common E/rho scaling, inverse-length scaling, and unsupported inputs.

**External benchmark boundary:** this is the analytical 1D axial-bar subproblem discussed in [NAFEMS Benchmark Challenge 5](https://www.nafems.org/downloads/nbr5v3.pdf), not a reproduction of the full planar modal truss in that challenge. NAFEMS explicitly highlights that consistent and lumped mass matrices can differ significantly on coarse meshes and converge with refinement. The full challenge also raises mode-shape fidelity and the limitations of single bar elements for real truss dynamics; the present module does not resolve those limitations or calculate a general 2D truss's natural modes.

Do not confuse this modal benchmark with [SOFiSTiK BE1](https://docs.sofistik.com/2026/en/verification/_static/verification/pdf/be1.pdf), which is a separate static 13-member plane-truss deflection case. The external reference reports a vertical deflection of about 69.09 mm at free node 8; no BE1 reproduction is claimed by the current two-bar static oracle or this modal bar module.

The initial modal acceptance contract is software verification only: exact analytical subproblem, expected consistent/lumped bracketing, monotone convergence on the frozen mesh ladder, deterministic results, and fail-closed invalid domains. No hardware timing, commercial solver comparison, experimental validation or engineering certification is claimed. The next external step is to pin the complete NAFEMS Challenge 5 geometry, restraints, member data, mass-matrix convention and target frequencies in a rights-aware manifest before comparing modal results.

## External modal fixture: NAFEMS Challenge 5 original two-member truss

The public `modal_truss::nafems_challenge5_reference_fixture()` freezes the original single-element-per-member planar truss shown in NAFEMS Benchmark Challenge 5, Figure 1. The two left nodes are fixed, the free node is at `(1 m, 1 m)`, and the model uses one horizontal member plus one diagonal member, both circular with radius `0.01 m`, `E=210 GPa`, and density `7800 kg/m^3`. The reported published frequency pairs are:

| Mass convention | First mode | Second mode |
|---|---:|---:|
| Consistent mass (NAFEMS CS1) | 452.09 Hz | 1114.6 Hz |
| Lumped mass (NAFEMS CS2) | 369.13 Hz | 910.05 Hz |

The new external-reference test gates all four rounded published values within 0.02%, and also requires two elastic modes, mass orthogonality, and normalized generalized-eigenvalue residual below `1e-10`. The tighter numerical result is derived from the stated geometry/material inputs; the rounded published frequencies are the external comparison targets.

**Scope distinction:** this tests the original two-member model shown in Challenge 5 and the mass-formulation effect for that discretization. It does not complete the full challenge investigation of member refinement, intermediate-node constraint equations, higher-mode convergence, or beam-member sensitivity. The NAFEMS authors warn that bar-only modes from one element per member may not represent the structure's real modes; the full benchmark follow-up must therefore include mode-shape interpretation and refinement/constraint variants, not only frequency agreement. No measured physical validation is claimed.

Source: [NAFEMS Benchmark Challenge 5 — Dynamic Characteristics of a Truss Structure](https://www.nafems.org/downloads/nbr5v3.pdf), Figure 1 and pages 0–3. The separate [SOFiSTiK BE1 benchmark](https://docs.sofistik.com/2026/en/verification/_static/verification/pdf/be1.pdf) remains a static truss-deflection case, not a modal benchmark.
