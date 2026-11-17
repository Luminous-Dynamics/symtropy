# HoloFrac integration map

**Date:** 2026-11-17  
**Status:** coordination map; references are existing programmes, not claims of completed integration.

HoloFrac is a shared representation/retrieval capability. It is not a second building-design application, a replacement BIM model, a solver, an approval authority, or a competing resident-preference system.

## Existing owners and boundaries

| Owner | Existing work | HoloFrac contribution | Must not duplicate |
|---|---|---|---|
| Symthaea | [#12519 multiscale structural grammars with HDC/VSA](https://github.com/Luminous-Dynamics/symthaea/issues/12519) | Experimentally compare compositional encodings, retrieval, capacity and cross-scale transfer | A second HDC substrate or a second grammar learner |
| Symthaea | [#7790 intent-to-design compiler](https://github.com/Luminous-Dynamics/symthaea/issues/7790) | Index typed requirements, spatial relations, alternatives and provenance for retrieval | Human-intent schema, hard/soft gate semantics, or preference aggregation |
| Symthaea | [#7801 room/spatial counterfactual qualification](https://github.com/Luminous-Dynamics/symthaea/issues/7801) | Retrieve related contexts and dependency paths for adversarial tests | Room-quality certification or human-outcome evidence |
| Symtropy | [#1766 spatial design](https://github.com/Luminous-Dynamics/symtropy/issues/1766) | Attach semantic indexes to exact, versioned spatial entities and geometry | Exact geometry provider, accessibility analysis or code authority |
| Symtropy | [#1848 spatial resilience](https://github.com/Luminous-Dynamics/symtropy/issues/1848) | Index time-varying service/habitability states and event histories | Timeline kernel, resource conservation or resilience verdict |
| Symtropy | [#1790 building simulation adapters](https://github.com/Luminous-Dynamics/symtropy/issues/1790) | Link candidate graphs to solver receipts and affected dependencies | Physics solvers or solver qualification |
| Symtropy | [#5661 FFMFS geometry lab](https://github.com/Luminous-Dynamics/symtropy/pull/5661) and [#5808 matched-mass benchmark](https://github.com/Luminous-Dynamics/symtropy/issues/5808) | Encode module, connector, assembly and evidence relationships for retrieval | Structural analysis, manufacturing qualification or structural safety |
| Sol-Atlas | [#617 AEC industry comparison](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/617) and [#502 engineering assurance comparison](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/502) | Supply a separately measurable graph-memory component in matched workflows | Product comparison methodology or vendor capability claims |

## Canonical ownership model

1. **Intent and rights:** Symthaea #7790 consumes source-bound resident intent and explicit hard/soft constraints. Sol-Atlas owns resident-facing decision receipts and Mycelix owns consent/provenance and authorized decision rights. HoloFrac does not infer preferences from identities or turn a partial preference graph into a total ranking.
2. **Exact spatial state:** Symtropy's qualified geometry/provider contracts remain authoritative for geometry, units, coordinate frames and revision identity. HoloFrac stores only identifiers, typed relationships and source-bound semantic indexes.
3. **Associative retrieval:** the HoloFrac adapter indexes typed graphs. Retrieval returns candidate source IDs, similarity and encoder fingerprint. Exact source lookup verifies every candidate used downstream.
4. **Physical evaluation:** Symtropy #1790 adapters and FFMFS engineering labs own solver execution, model assumptions, convergence, uncertainty and evidence ceilings.
5. **Qualification:** Symthaea #7801 and Symtropy #1766/#1848 own the broader spatial, counterfactual, resilience and human-centred task suites.
6. **Comparative evaluation:** Sol-Atlas #617/#502 own fair product/workflow and assurance comparisons. HoloFrac is one component under matched scope, not the new benchmark owner.

## Shared graph contract (initial)

The retrieval fixture uses stable source entity IDs and explicit relation direction. A future adapter must additionally carry:
- exact source system and source revision;
- geometry GUID / entity identity and source digest;
- relation schema and version;
- units, coordinate frame and validity interval when relevant;
- evidence/authorization references when relevant;
- encoder schema and fingerprint;
- exact graph/source digest separate from the vector and encoder fingerprint.

No HDC value is a cryptographic identity. Use the owning source's exact identifier and an established cryptographic digest for integrity.

## Integration sequence

### I0 — deterministic graph retrieval
Current standalone Rust slice. Validate entity references, exact graph lookup, canonical metadata lookup, directed-role encoding, reversed-edge negative controls, fingerprints and dimension/resource accounting.

### I1 — partial-context retrieval and abstention
Add queries with one or more unknown fields, report recall@k/precision@k/MRR and false completion. Calibrate abstention on held-out graph families, not on the training fixture. Unknown answers must stay unknown.

### I2 — source-bound adapters
Map a frozen subset of the Symthaea intent object, FFMFS module graph and Symtropy spatial graph into the shared relation vocabulary. Preserve original IDs and versioned mappings; never flatten multiple authorities into one mutable record.

### I3 — IFC/IDS exchange experiment
Use IFC 4.3 and IDS as external exchange/requirements formats. Test identity, property, relation, units and geometry round-trip loss. IDS checking is not geometry verification or physical qualification.

### I4 — counterfactual dependency retrieval and evidence invalidation
[Symtropy #5934](https://github.com/Luminous-Dynamics/symtropy/issues/5934) now tracks the executable exact dependency-DAG slice in [impact.rs](../holofrac-retrieval/src/impact.rs). It models consumer -> dependency edges across requirements, parameters, geometry, interfaces, solver inputs/runs and evidence receipts. It validates acyclicity, computes reverse transitive closure, and uses a separate brute-force reachability oracle in tests. An incomplete dependency inventory conservatively invalidates all scoped evidence; it never preserves an unproven current result.

The first capacity executable sweeps 100/1,000/10,000/100,000 artifacts with one or more changed project roots. Next qualification must add shared/common-cause DAGs, randomized topology families, incremental source-revision updates, raw evidence-currentness receipts and allocator-inclusive RSS. HDC may suggest analogous or missing dependency edges, but exact source-bound closure remains the only invalidation authority.

### I5 — architectural and structural applications
Run the existing #1766, #1848, #1790 and FFMFS scenarios with HoloFrac enabled/disabled under identical candidate sets, solver-call budgets, source revisions and hardware. Measure whether retrieval reduces work or improves context selection without reducing correctness, uncertainty calibration or hard-gate preservation.

## Release rule

Do not integrate this crate into production Symthaea/Symtropy APIs until:
- exact-head Rust tests and Clippy pass;
- deterministic replay is confirmed on supported targets;
- directed-edge negative controls pass;
- held-out project/topology splits exist;
- partial retrieval and abstention are calibrated;
- exact source lookup is mandatory at reliance points;
- the owning programmes accept the adapter contracts.

The current crate is an initial research implementation only. It has not yet been locally compiled or executed in this environment.
