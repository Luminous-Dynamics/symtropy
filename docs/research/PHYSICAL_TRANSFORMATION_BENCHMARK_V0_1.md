# Physical Transformation Benchmark v0.1

**Status:** research contract / proposal  
**Tracking:** [PHYS-TRANSFORM-001 #2209](https://github.com/Luminous-Dynamics/symtropy/issues/2209)  
**Scope:** deterministic state transitions, structural graph topology, bounded analytical mechanics, and material accounting.  
**Not in scope:** a production fracture solver, demolition controller, building-code verdict, structural certification, or autonomous machine control.

## 1. Core thesis

Destruction and construction are not separate simulation features. They are operations over one revisioned physical state.

A state contains:
- components with stable IDs, geometry references, material declarations, units, and provenance;
- connections and constraints with explicit endpoints and types;
- loads and boundary conditions with declared frames, units, and validity windows;
- event history with deterministic ordering and before/after state digests;
- material lots whose mass is conserved across recovered, reused, recycled, contaminated, and waste categories.

A support removal must change the graph and invalidate any analysis that depended on the prior graph. A break event must not merely hide a mesh. A construction event must create or reconnect components and can only restore a prior topology when the necessary operations actually reconstruct it. A renderer may visualize the result, but screenshots are not correctness evidence.

## 2. Architecture and responsibility boundaries

| Layer | Responsibility | Must not claim |
|---|---|---|
| Symtropy state kernel | Component/connection graph, event semantics, geometry handles, revisioning, deterministic receipts | Physical validity beyond the implemented model |
| Mechanics adapter | Unit-aware analytical cases and later solver integrations | Solver certification from adapter success |
| Renderer | Display state and uncertainty/failure overlays | Visual plausibility as proof |
| Symthaea | Plan proposals, diagnosis, uncertainty explanation, action alternatives | Replacing mechanics calculations or evidence |
| Sol-Atlas | Site/terrain/utilities context and provenance | Inferring unknown site conditions |
| Mycelix | Material custody, reuse provenance, lifecycle/accounting claims | Treating declared recovery as verified recovery |

Keep the first reference kernel dependency-light and isolated from the main workspace if that avoids pulling rendering or solver dependencies into the test oracle.

## 3. State and event contract

Each event has a stable ID, schema version, input state revision, typed payload, and deterministic outcome. Events are applied transactionally: rejected events leave the state unchanged.

Initial event families:
- `ComponentInstalled`, `ComponentRemoved`
- `ConnectionCreated`, `ConnectionRemoved`
- `SupportActivated`, `SupportReleased`
- `LoadDeclared`, `LoadRemoved`
- `ConnectionDamaged`, `ConnectionFailed`
- `MaterialRecovered`, `MaterialSorted`, `MaterialReused`, `MaterialDiscarded`

Required rejection cases:
- duplicate event/component/connection IDs;
- references to missing components or mismatched connection endpoints;
- unknown units or material definitions;
- non-finite or out-of-domain numeric values;
- invalid transitions (e.g. removing a nonexistent connection);
- a load or boundary condition with an unspecified frame or units;
- mass-ledger imbalance beyond the declared tolerance;
- any requested physics operation not supported by the selected model.

Causal order matters. Event reordering is allowed only for events explicitly proven independent; event permutations must not silently be canonicalized when they change physical meaning.

## 4. Benchmark ladder

### B0 — State-machine and topology invariants
Synthetic graph cases verify component counts, connection endpoints, connected components, support state, revision increments, transactional rejection, deterministic replay, and byte-stable receipts.

### B1 — Analytical mechanics
Use small closed-form cases before any large solver:
- axial bar under a declared load;
- cantilever tip deflection and maximum bending stress under stated assumptions;
- simply supported beam with a known load case;
- invalid units, zero/negative dimensions, singular configurations, and overflow controls.

Every fixture must declare assumptions, units, boundary conditions, formula source, expected value, and tolerance. Keep an independently implemented oracle separate from the production evaluator. A passing analytical fixture validates only that case and convention.

### B2 — Causal damage and fragmentation
Start with graph connectivity, not visually convincing rubble:
- remove a support and verify the topology change;
- break a connection and verify components separate only when graph connectivity requires it;
- track fragments by stable component membership;
- test deterministic split/merge history and conservation of component/material IDs.

Do not equate graph disconnection with a physically predicted collapse. Geometry contact, stress redistribution, crack propagation, and collision response remain unresolved until separately implemented and benchmarked.

### B3 — Construction and repair sequences
Represent assembly as a sequence with prerequisites:
- temporary support installed before a dependent component is released;
- connections created only when both endpoints exist;
- sequence replay reaches the declared target topology;
- reverse/repair plans identify missing operations rather than magically restoring state;
- inaccessible or unsupported operations return unresolved until geometric reachability/tooling models exist.

### B4 — Material recovery
For each material lot, report declared input mass and outputs by category. Require:
`input_mass = reused_mass + recycled_mass + contaminated_mass + waste_mass + unaccounted_mass`
within a declared numeric tolerance and unit basis. Never infer recycling yield from appearance or material name. Keep unknown composition and unverified weighing explicit.

### B5 — Numerical solver comparison
Only after a solver is selected: pin solver/version/build, element type, mesh, units, boundary conditions, nonlinear settings, convergence criteria, and output extraction. Compare displacements, forces, residuals, energy balance, and convergence against analytical or independently qualified cases. Record solver limitations and licensing.

### B6 — Fragmentation, contact, and sim-to-real
Require a named licensed benchmark or a controlled physical experiment with instrumented geometry/material properties, force/time traces, measurement uncertainty, and held-out cases. Compare fragment count/size distribution, trajectory/contact outcomes, energy accounting, and timing. Synthetic scenes alone cannot establish sim-to-real validity.

## 5. Baselines and scorecard

Every experiment compares matched cases against:
1. **No-op baseline:** state remains unchanged when no event is applied.
2. **Graph-only baseline:** topology changes are evaluated without claiming mechanical prediction.
3. **Analytical oracle:** closed-form values on the supported small cases.
4. **Selected solver baseline:** only when its model, configuration, and reference case are pinned.
5. **Ablations:** disable event provenance, material accounting, or uncertainty handling one at a time to show what each layer contributes.

Report metrics independently rather than collapsing them into a single score:
- topology/connectivity error and state-transition rejection rate;
- analytical force/displacement relative error and unit-consistency failures;
- mass/energy balance residuals;
- deterministic replay and receipt stability;
- invalid-input and negative-control detection;
- runtime and peak memory versus component/constraint count;
- fragmentation distribution error and contact error when ground truth exists;
- construction-sequence feasibility and number of unresolved prerequisites;
- material mass by verified category, with unknown/unmeasured mass kept separate;
- uncertainty coverage, abstention rate, and false-accept rate.

Pre-register primary metrics and tolerances before evaluating new methods. Report confidence intervals for measured comparisons. Do not claim “better than SOTA” without an eligible public baseline, matched constraints, reproducible data, and independent evaluation.

## 6. Evidence and status vocabulary

Every result distinguishes:
- **SYNTHETIC:** generated inputs and expected outcomes;
- **ANALYTICAL_REFERENCE:** a known-answer case under declared assumptions;
- **MEASURED:** instrumented observation with calibration and uncertainty;
- **SIMULATED:** output from a named model/solver/configuration;
- **INDEPENDENTLY_REPRODUCED:** separately implemented or executed comparison with recorded provenance;
- **UNRESOLVED:** missing model, unsupported physics, insufficient evidence, or failed qualification.

A digest proves identity of bytes, not truth of a measurement. A validator passing a manifest does not prove that its declared physical properties are accurate.

## 7. Safety and operational gates

The first iterations are offline research and visualization only. They must not command real demolition equipment, cranes, robots, or construction machinery. Real-site use requires qualified structural/geotechnical engineering review, applicable jurisdictional approvals, validated sensors/models, fail-safe controls, and independently assessed hazard analysis. No simulation result by itself authorizes demolition, occupancy, or construction.

## 8. Initial acceptance criteria

- [ ] Versioned component/connection/event schema with explicit units.
- [ ] Transactional state transitions and deterministic event replay.
- [ ] Frozen synthetic topology fixtures and independent expected results.
- [ ] At least two closed-form mechanics fixtures with declared tolerances.
- [ ] Material mass ledger with an intentionally imbalanced negative control.
- [ ] Invalid-unit, duplicate-event, unknown-reference, non-finite-value, and unsupported-physics controls.
- [ ] Receipt binds input bytes, model/config version, event sequence, output state, and findings.
- [ ] Focused Rust formatting, tests, and Clippy workflow pinned to repository toolchain.
- [ ] Exact-head CI result recorded; authored tests never described as passing until actually executed.

## 9. Suggested implementation sequence

1. Implement schema and graph/event kernel with deterministic fixtures.
2. Add independent topology oracle and mutation/negative controls.
3. Add unit-aware analytical mechanics as a separate adapter.
4. Add material ledger and custody boundary.
5. Introduce a solver adapter only after the reference cases and claim boundaries are stable.
6. Integrate renderer overlays for component state, failure hypotheses, uncertainty, and evidence status.
7. Add robotics/tool reachability and site context only when those inputs can be provenance-bound.

The target is not maximal visual destruction. It is a simulation whose physical claims become progressively more useful because each layer has explicit assumptions, independent tests, and a known validity envelope.
