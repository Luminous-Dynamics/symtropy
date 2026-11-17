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

Compare representations on identical graph fixtures and query sets:

- HRR;
- FHRR;
- MAP;
- BSC;
- TPR;
- GHRR, only when a separately specified implementation is available;
- exact typed-graph traversal and canonical serialized lookup as non-HDC controls.

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

- Autodesk Fusion Generative Design produces editable alternatives against geometric, performance and manufacturing criteria, including additive, milling, cutting and casting constraints.
- nTop combines implicit geometry, parametric exploration, structural/thermal/CFD simulation and accelerated evaluation of many variants.
- Established building-performance workflows use tools such as EnergyPlus, Radiance and Modelica-based emulators; BOPTEST provides a common environment for benchmarking building-control strategies.

HoloFrac should not claim to beat these products at generic CAD, implicit geometry, CFD, FEA, topology optimization or building control without direct matched tests.

The differentiated research hypothesis is the integrated system: **multiscale typed spatial memory + grammar/interface co-design + counterfactual physical evaluation + occupant-steerable Pareto alternatives + evidence lineage across design, assembly and operation.**

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
