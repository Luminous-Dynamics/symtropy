# HoloFrac Architecture: Multiscale Spatial Intelligence

**Status:** research programme and benchmark contract; no performance or engineering-superiority claims  
**Date:** 2026-11-17  
**Parent work:** [Free-Flow Modular Fractal Structures](../FREEFLOW_MODULAR_FRACTAL_STRUCTURES.md) and [FFMFE benchmark protocol v1](../ffmfe-benchmark/PROTOCOL_V1.md)

## Purpose

HoloFrac Architecture is a proposed bridge between Symthaea's hyperdimensional/vector-symbolic reasoning and Symtropy's physical evaluation substrate. Its first application is a multiscale architectural model that can represent, retrieve, generate, simulate, compare and preserve evidence about spaces and the systems inside them.

"Holographic" here means a compositional, distributed vector-symbolic representation with associative retrieval. It does **not** assert a physical holographic principle, lossless storage of an entire building in one vector, or recovery of exact geometry from a hypervector. "Fractal" is a measurable geometric/dynamical property only when a scaling regime is demonstrated; recursive construction or visual resemblance alone is insufficient.

The programme therefore has three non-negotiable separations:

1. **Authoritative design state:** typed entities, exact geometry, units, constraints, interfaces, versions and provenance.
2. **Associative intelligence:** HDC/VSA encodings used as indexes for similarity, analogy, partial-context retrieval and candidate recall.
3. **Physical evidence:** Symtropy solvers, external reference tools, measured tests and human evaluation. A hypervector similarity score is never a structural, energy or safety certificate.

## Existing foundations to reuse

Do not create a second HDC substrate or another generic fractal generator.

- Symtropy's `symtropy-hdc-core` already provides deterministic dense continuous and bipolar hypervectors, item memory, binding, bundling, permutation, cosine similarity, scalar-level encodings and versioned encoder fingerprints. Its default dimension is 16,384. It deliberately has no cognition or physics policy.
- `symtropy-hdc-physics` already treats HDC as a deterministic semantic shadow of exact simulation state. Exact physics remains authoritative; emitted vectors carry encoder identity and source-state linkage.
- Symthaea's public research programme already calls for controlled HRR/FHRR/MAP/BSC/TPR/GHRR comparisons. Its topology code includes fractal-inspired constructions, but recursive topology is not by itself evidence of a geometric fractal.
- The FFMFS research branch already contains a bounded geometry grammar, port and tolerance checks, assembly dependency planning, a separate axial truss reference solver, beam-element work, damage screens and a frozen matched-mass benchmark protocol.

Useful starting points:

- [symtropy-hdc-core](../../crates/core/symtropy-hdc-core/README.md)
- [symtropy-hdc-physics](../../crates/domains/symtropy-hdc-physics/README.md)
- [HDC physics research protocol](../../crates/domains/symtropy-hdc-physics/docs/HDC_PHYSICS_RESEARCH_PROTOCOL.md)
- [FFMFS research programme](../FREEFLOW_MODULAR_FRACTAL_STRUCTURES.md)
- [FFMFE benchmark protocol v1](../ffmfe-benchmark/PROTOCOL_V1.md)
- [Symthaea complexity, chaos and multiscale research programme](https://github.com/Luminous-Dynamics/symthaea/blob/main/docs/COMPLEXITY_CHAOS_KNOWLEDGE_PROGRAM.md)

### Important implementation boundary

The current Symtropy continuous-vector binding operation is element-wise multiplication, which is commutative. Permutation can encode order as a separate operation, but that is not the same implementation as non-commutative GHRR matrix binding. Do not label the existing core "GHRR." A GHRR adapter must be a separately specified and separately tested research implementation, including its memory and compute costs.

## The proposed paradigm shifts

### 1. From geometry-first CAD to intent-and-constraint-first design

Represent user needs, site conditions, functional programme, access, privacy, environmental targets, maintenance and lifecycle constraints before choosing a shape. Generate geometry only through a declared grammar or solver that can preserve and report those requirements.

The system should expose infeasible requirements and trade-offs rather than silently weakening constraints.

### 2. From one building model to a typed multiscale spatial graph

Represent relationships across site, building, storey, zone, room, assembly, component and material scales. A local query must be able to retrieve its relevant parent context and dependencies; a global change must identify its affected descendants and connected systems.

The graph is authoritative. HDC accelerates associative access to it; it does not replace it.

### 3. From fractal appearance to functional scale coupling

A recursive rule is useful only when it earns a measurable function: load transfer, shading, daylight distribution, airflow, acoustic diffusion, drainage, utility routing, repair access or assembly.

Measure geometric scaling only over an explicitly declared interval and compare against non-fractal controls. Use scale-dependent performance curves rather than one universal "fractal quality" score.

### 4. From shape optimization to grammar-and-interface co-design

Optimize the reusable vocabulary, connector/interface families, attachment rules, assembly sequence and global topology—not only a final mesh. Keep primary structure, secondary structure, services, envelope and occupant-authored infill as distinct layers.

This directly extends FFMFS: free-form globally, standardized and inspectable locally.

### 5. From a single optimum to robust, occupant-steerable Pareto families

Return feasible alternatives with explicit trade-offs across structure, energy, daylight, thermal comfort, ventilation, water, materials, cost, maintenance, accessibility, privacy, sensory control and expression. Keep occupant requirements visible and editable; do not collapse diverse preferences into one assumed universal objective.

### 6. From one-shot simulation to evidence-carrying closed-loop design

Every candidate links to its exact source snapshot, constraints, solver versions, weather and boundary conditions, numerical settings, uncertainty, failed runs and outputs. Compare predicted and measured behaviour; calibrate only with versioned evidence. Changes to inputs or models create new receipts.

### 7. From "AI designs buildings" to a role-separated engineering system

- Symthaea proposes, retrieves, explains, decomposes requirements and selects experiments.
- Symtropy evaluates declared physical models and records their limits.
- External tools provide independent comparison where formulations overlap.
- Designers, occupants and qualified professionals retain review and approval authority.
- Mycelix may later provide provenance/authorization; it must not turn a model's claim into engineering authority.

### 8. From a single spatial graph to an evidence-and-change graph

A geometry graph says what is connected. An engineering change graph must also say why each object exists, which requirements it satisfies, which exact source/version created it, which analyses depend on it, and which evidence becomes stale after a change.

Model dependency edges explicitly:
- design parameter -> geometry feature -> physical boundary condition;
- component/interface -> load path or service network;
- requirement -> constraint -> candidate decision;
- source snapshot -> analysis input -> solver run -> evidence receipt;
- maintenance event -> changed component state -> future inspection/analysis.

A change should produce a machine-checkable impact closure. Independent analysis adapters re-evaluate only affected evidence, while unchanged evidence remains current only when its exact dependencies are unchanged. Unknown dependency relationships must invalidate conservatively, not silently remain PASS.

Autodesk Forma's connected AEC work is already moving toward shared project context, drawing search/change analysis and AI-assisted review. HoloFrac must therefore be tested on the stricter claim: exact dependency closure, independently replayable evidence, and zero false-current classifications under seeded cross-tool mutations—not merely the ability to highlight a change.

### 9. From simulation batches to uncertainty-directed experiments

Do not launch every expensive simulation, and do not trust a surrogate point estimate without calibration. Maintain an explicit model of uncertainty and choose the next experiment according to expected information or decision value.

Compare RBF/RBFMOpt, TPE, HypE and adaptive Polynomial Chaos Expansion (PCE) with high-fidelity simulation. The November 2026 BIM-adaptive surrogate study uses sequential sparse PCE and multi-output/multi-timescale sensitivity analysis; the October 2026 design-support study combines RBF surrogates, uncertainty quantification and bipartite clustering. Those are current comparison arms, not details to rediscover.

A new HoloFrac hypothesis is that a typed dependency graph can localize uncertainty and propagate it through the parts of the building that depend on an uncertain parameter. Test that against global surrogate uncertainty and simple sensitivity-based selection. It must improve decision quality or reduce high-fidelity calls at matched feasibility and error—not simply produce attractive uncertainty visualizations.

### 10. From duplicated role vectors to factorized multiscale memory

Repeated room IDs, relation kinds, scales, materials and component families should share typed codebook vectors. Per-fact records hold compact references; exact source identifiers and postings remain available for definitive retrieval.

Use a two-path query contract:
- exact path: source-keyed postings/graph traversal for exact identifiers and relations;
- associative path: HDC similarity for incomplete, noisy, analogical or cross-scale cues.

The exact path is expected to win for exact known keys. HDC earns its place only when it improves recall or transfer under incomplete/noisy semantic context without increasing false acceptance. Exact verification remains mandatory before design decisions.

For non-commutative composition, evaluate a separate GHRR implementation and the November 2026 flattened inference treatment. Match both effective dimension D*m^2 and actual bytes; do not report nominal D as if it were comparable to FHRR's D.


## Canonical spatial model

The initial architecture graph should be a typed, attributed, versioned multigraph/hypergraph. A normal graph is adequate for simple adjacency. Explicit interface objects or hyperedges are needed when a connection is shared by more than two members, rooms or systems.

### Entity families

- Context: site, climate region, terrain, adjacent environment.
- Spatial: building, storey, zone, room, circulation segment, opening, threshold.
- Structural: system, assembly, node, member, panel, support, interface, connector.
- Environmental: facade, shading device, thermal zone, ventilation path, water catchment, drainage path, acoustic surface.
- Services: electrical, data, water, wastewater, HVAC, fire/life-safety and sensing components.
- Human requirements: task, access envelope, occupant-selected setting, privacy boundary, wayfinding cue.
- Evidence: source model, simulation run, observation, inspection, test specimen, maintenance event.

### Relationship vocabulary

The first relation set should include:

- `contains`, `part_of`, `adjacent_to`, `accessible_from`, `opens_to`;
- `supports`, `braces`, `connects_to`, `interfaces_with`;
- `shades`, `illuminates`, `views`, `acoustically_coupled_to`;
- `ventilates_to`, `supplies`, `drains_to`, `exchanges_heat_with`;
- `requires_access_to`, `maintained_by`, `assembled_before`;
- `derived_from`, `evaluated_by`, `observed_by`, `verified_by`.

Relations must specify direction where direction matters, endpoint types, units where applicable, coordinate frame, validity interval and provenance. A reversed relation must be a negative control, not treated as an equivalent fact.

### Interoperability

Use IFC 4.3 as the first exchange contract and IDS to express machine-checkable information-delivery requirements. Preserve source GUIDs and stable project-local IDs. Do not force every Symtropy-specific relation into an ad hoc IFC property; maintain a versioned extension/mapping layer and report unmapped semantics.

IFC is an exchange and information model, not the sole internal solver representation. Exact geometry, topology, units and constraints must survive round-trip tests within declared tolerances.

## HDC/VSA experiment design

The current executable research slices are intentionally separated:
- [Typed graph, exact postings, partial-context and factorized-index retrieval](../holofrac-retrieval/README.md)
- [Continuous spatial-field encoding and reconstruction](../holofrac-retrieval/src/field.rs)
- [Shared multi-channel spatial memory](../holofrac-retrieval/src/multichannel.rs)
- [Geometric box-counting qualification](../holofrac-retrieval/src/fractal.rs)
- [Exact engineering-evidence dependency closure](../holofrac-retrieval/src/impact.rs)
- [Bitemporal six-channel observation fixture](../holofrac-retrieval/src/temporal.rs)
- [Sensor-placement and nearest-sensor virtual-sensing baselines](../holofrac-retrieval/src/sensor_placement.rs)
- [Continuous-field benchmark CLI](../holofrac-retrieval/src/bin/field_sweep.rs)
- [Multi-channel field benchmark CLI](../holofrac-retrieval/src/bin/multichannel_sweep.rs)
- [Geometric scale benchmark CLI](../holofrac-retrieval/src/bin/fractal_sweep.rs)
- [Bitemporal observation benchmark CLI](../holofrac-retrieval/src/bin/temporal_fixture_sweep.rs)
- [Sensor-placement benchmark CLI](../holofrac-retrieval/src/bin/sensor_placement_sweep.rs)
- [Temporal observation protocol](TEMPORAL_FIXTURE_PROTOCOL_V0.md)
- [Sensor-placement protocol](SENSOR_PLACEMENT_PROTOCOL_V0.md)

The graph benchmark measures exact relations and partial-context candidates; the continuous-field tracks measure positional encoding, multi-channel binding, bundling, unbinding, cleanup and held-out reconstruction against classical storage/interpolation. The geometry track tests a declared scale estimator against known-dimension controls. The dependency track computes exact evidence invalidation. The temporal fixture separates evaluator-only truth from observations and models late delivery/corrections with decision-time snapshots. The sensor-placement fixture compares regular-grid, seeded-random and greedy-maximin layouts using a deliberately simple nearest-sensor virtual-sensing baseline. Both temporal and placement fixtures are synthetic and do not represent measured buildings. None substitutes for physical simulation or engineering authority.

Compare representations on identical graph fixtures and query sets:

- exact source-keyed role-postings and adjacency-list retrieval;
- HRR;
- FHRR;
- MAP;
- BSC;
- TPR;
- matrix-native GHRR and flattened GHRR, only when separately specified and dimension/byte matched;
- exact typed-graph traversal and canonical serialized lookup as non-HDC controls.

Report exact identifiers, aliases, noisy/corrupted labels, relation masking, incomplete cues and held-out scale/topology families separately. The exact postings baseline is not an optional strawman; HDC is not expected to win on complete exact keys.

Sweep dimensions including 2,048, 4,096, 8,192, 16,384, 32,768 and 65,536 where the representation supports them. Keep seed, item-memory construction, normalization, query corpus, hardware, precision and memory accounting explicit.

The HDC layer may return candidate matches with similarity, encoder fingerprint and source IDs. Exact graph traversal must verify every result used for design decisions. Unknown or out-of-domain cases remain unknown.

## Architectural proving ground

Start with a small, public, reproducible courtyard dwelling or pavilion fixture rather than a city-scale claim. It should include a site context, at least two spatial levels, a functional room programme, circulation, openings, structural assemblies, facade/shading, water and ventilation paths, and explicit occupant-controlled settings.

The fixture should be deliberately small enough for independent review but rich enough to expose cross-scale dependencies. Its exact dimensions and requirements are frozen in the benchmark protocol before the first ranking run.

The architectural testbed must connect three existing research lines:

1. **FFMFS:** reusable multiscale structural grammar, interfaces, tolerances, assembly and damage cases.
2. **HoloFrac:** spatial graph composition, hierarchy, associative retrieval and scale-transfer tests.
3. **Symtropy building twin:** structural, daylight, thermal, airflow, water and lifecycle evaluations as validated models become available.

A separate floor-plan benchmark should use the Modified Swiss Dwellings (MSD) dataset for building-complex-scale plan generation and completion. Its scale and license make it a useful public benchmark; it is not a substitute for local climate, code or occupant validation.

## Industry comparison: what not to claim

Commercial systems already offer substantial capability:

- Autodesk Forma is connecting site, Revit and Civil 3D workflows around shared project context, and is adding AI drawing search, revision change analysis, multifamily layout exploration and drawing compliance review.
- TestFit targets rapid feasibility and site planning with zoning and unit/parking mix constraints.
- nTop combines implicit geometry, parametric exploration and structural/thermal/CFD simulation; its 5.23 release brought CFD into the iterative implicit-geometry loop.
- Autodesk Fusion Generative Design produces editable alternatives against geometric, performance and manufacturing criteria.
- Established building-performance workflows use EnergyPlus, Radiance, Modelica-based emulators and BOPTEST for building-control benchmarking.

HoloFrac should not claim to beat these products at generic CAD, implicit geometry, CFD, FEA, topology optimization, drawing revision analysis or building control without direct matched tests. Autodesk's connected data and AI change-analysis work means "preserving project context" alone is no longer a sufficient differentiation claim.

The differentiated research hypothesis is narrower and testable: **typed source lineage + exact dependency closure + independent evidence invalidation/replay + uncertainty-directed physical evaluation + occupant-steerable Pareto alternatives across design, assembly and operation.** This must be compared against the complete connected product workflows, not isolated tools or a marketing feature checklist.

The strongest first result would be a reproducible case where this integration reduces expensive rework or evaluation cost without losing feasibility, accuracy, diversity or user control—not a visually novel rendering.

## Initial release gates

- No duplicate HDC substrate.
- No use of HDC vectors as authoritative geometry or safety state.
- No "GHRR" label for commutative element-wise binding.
- No fractal claim without a scale interval, estimator diagnostics, uncertainty and null controls.
- No "better than SOTA" claim without frozen, matched baselines and released raw results.
- No building-performance claim without declared weather, occupancy, schedules, model versions and uncertainty.
- No structural-safety claim from L0-L3 evidence.
- Every result identifies source commit, dirty state, toolchain, configuration, seeds, exact input digests and failed/infeasible candidates.
