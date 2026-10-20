# Physical Transformation Reference Kernel v0.1

**Tracking:** [PHYS-TRANSFORM-001 #2209](https://github.com/Luminous-Dynamics/symtropy/issues/2209)  
**Status:** initial dependency-free software reference; not a mechanics solver.

## What is implemented

- Transactional, revisioned events for component installation/removal, connection creation/removal, and support activation/release.
- Stable component/connection IDs, explicit material labels, and declared integer-milligram accounting fields.
- Deterministic connected-component analysis after structural-graph changes.
- A conserved mass ledger partitioned into reused, recycled, contaminated, waste, and unaccounted outputs.
- Frozen synthetic fixture: three components and two connections; disconnecting one edge changes graph connectivity from one connected component to two, and reconnecting it returns the graph to one.
- Negative controls for duplicate events, failed transactional mutation, dangling endpoints, self-connections, support state, unbalanced mass, numeric overflow and malformed identifiers.
- Independent integration tests pin the expected graph and ledger outcomes.
- Deterministic JSON receipt bound to the 40-character source commit supplied by the caller.

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

Treat destruction, construction and repair as the same state-transition problem. A broken connection modifies topology, a removed component requires valid preconditions, and a rejected event cannot partially mutate the world. Material accounting follows the same principle: all outputs must reconcile to declared input, with unknown mass kept visible rather than disappearing into an aggregate “recycling rate.”

This is intentionally a small slice. Graph connectivity does not prove whether a building is stable, whether an element will actually fall, or how stress redistributes. The current mass fields are declared accounting values, not sensor measurements or verified material properties.

## Comparison with the field

- **Computational solid mechanics:** ASME V&V 10 describes a framework for verification, validation and uncertainty quantification. Symtropy should adopt that separation rather than treating a passing test as physical validation: https://www.asme.org/codes-standards/find-codes-standards/standard-for-verification-and-validation-in-computational-solid-mechanics/2019/pdf
- **Fracture and multiphysics:** MOOSE provides phase-field fracture and coupled mechanics capabilities. A future Symtropy adapter must name its constitutive assumptions, mesh, convergence policy and benchmark rather than claiming equivalent fidelity from visuals: https://mooseframework.inl.gov/modules/phase_field/
- **Large granular/contact simulations:** Project Chrono documents GPU-accelerated DEM for granular dynamics, including workloads at scales far beyond this kernel. This first Rust crate is not a competing particle engine; it is a state/evidence/control layer that can later sit above or beside specialized solvers: https://api.projectchrono.org/module_dem_installation.html
- **Interoperability:** IFC 4.3.2.0 is the official buildingSMART schema release associated with ISO 16739-1:2024. A later adapter can preserve element identities and report import/export losses, without claiming IFC semantics are physical validation: https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/index.html
- **Measurement culture:** NIST AM-Bench is a useful model for publishing high-quality measured challenges and reproducible simulation comparisons; its domain is additive manufacturing, so it is a benchmark-design inspiration, not a demolition reference dataset: https://www.nist.gov/ambench

## Benchmark ladder

1. **Software invariants (now):** deterministic replay, transactional rejection, graph connectivity and exact integer mass balance.
2. **Analytical mechanics:** unit-aware bar/beam/cantilever cases with independently computed answers and tolerances.
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
