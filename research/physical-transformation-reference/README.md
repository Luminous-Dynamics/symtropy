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
- Twenty-one unit tests and nine integration tests (four independent known-answer/topology oracles and five metamorphic tests) pin topology, accounting, deconstruction planning, analytical outcomes, scaling laws, source-unit normalization equivalence, and a fixed-seed 128-case parameter sweep.
- A CI mutation audit first requires the unmutated metamorphic suite to pass, then creates six disposable source copies with targeted axial-area/extension, cantilever-span/modulus, and simply-supported-beam-span/stress defects; each corresponding metamorphic test must fail. The script writes a source- and toolchain-bound JSON receipt; mutations cannot edit the checked-out production source.
- Deterministic JSON receipt bound to the 40-character source commit supplied by the caller, including state-bound recovered-component count and mass.

## Run locally

```sh
cargo fmt --manifest-path research/physical-transformation-reference/Cargo.toml --all -- --check
cargo test --locked --manifest-path research/physical-transformation-reference/Cargo.toml
cargo clippy --locked --manifest-path research/physical-transformation-reference/Cargo.toml --all-targets -- -D warnings
cargo run --locked --manifest-path research/physical-transformation-reference/Cargo.toml -- \
  --source-commit "$(git rev-parse HEAD)"
```

The CLI emits one JSON receipt. The source commit binds the receipt to versioned code and embedded fixtures; it is not a cryptographic digest of physical inputs or a claim of verified real-world evidence.

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
2. **Analytical mechanics (initial slice implemented):** unit-aware axial-bar and cantilever-tip-load cases with independently pinned expected values and declared tolerances.
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

## Safety and claim ceiling

Offline research only. This crate cannot authorize or control demolition/construction equipment, certify a structure, interpret building codes, or predict collapse. Real-site decisions require validated physics, qualified engineering review, applicable approvals, and independently assessed hazard controls.


## Design-for-disassembly assessment v0.1

The optional `design_for_disassembly` module adds a **vector-valued** connector metadata assessment:

- connection type: reversible fastener, cut-required, bonded/monolithic, or unknown;
- required tool class (tool-free, standard hand tool, powered hand tool, thermal cutting, specialized rig, or unknown);
- declared disassembly effort (person-minutes), retaining `None` if the total is not complete;
- tool access and inspection access: accessible, constrained, blocked, or unknown;
- declared component-damage risk: low, moderate, high, or unknown;
- connector reuse: reusable, not reusable, or unknown;
- metadata completeness counts and a sorted list of connections that remain unresolved.

The assessment binds itself to the world's numeric revision and canonical source-state snapshot. The CLI now emits a frozen DfD known-answer vector (2 connections, 1 reversible fastener, 1 cut-required connector, 17 declared person-minutes, zero incomplete profiles) into the deterministic reference receipt. Duplicate connector profiles and profiles referring to foreign connection IDs are rejected. A missing profile is represented as unknown, not silently ignored; once any effort is missing, the total remains unresolved even if later profiles have effort estimates. Checked arithmetic rejects total-effort overflow.

There is deliberately no scalar "DfD score": the vector makes trade-offs and missing dimensions visible rather than allowing arbitrary weights to mask unknown connection behavior. `Complete` means only that the declared metadata vector is complete for the graph. It is not a structural stability result or a qualified safe-removal sequence.

The [PHYS-REVERSIBILITY-001 issue #2268](https://github.com/Luminous-Dynamics/symtropy/issues/2268) tracks adding an open, measured DfD comparison set and later comparing against published indicators such as DEI, once permitted source data and metric definitions are available.


## Measured DfD validation protocol

A proposed field/bench protocol is in [PHYSICAL_DFD_MEASUREMENT_PROTOCOL_V0_1.md](../../docs/research/PHYSICAL_DFD_MEASUREMENT_PROTOCOL_V0_1.md), tracked by [PHYS-DISASSEMBLY-OBS-001 #2282](https://github.com/Luminous-Dynamics/symtropy/issues/2282). It freezes predictions before task start, uses grouped blind holdouts, preserves all incomplete/safety-stopped tasks, and compares effort/tool/outcome predictions against naive baselines. It starts with non-load-bearing bench coupons; it is not a real demolition trial. No measured corpus or performance claim is included.
