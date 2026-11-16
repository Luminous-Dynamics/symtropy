# GeoBio Foundry — Symtropy research integration

GeoBio Foundry is an evidence-aware research layer for geological infrastructure, resource processing, contained biological manufacturing, and autonomous industrial operations in extraterrestrial and terrestrial environments.

This work lives in the public Luminous Dynamics Symtropy repository. It does not use or depend on the archived personal repository.

## Why this repository

Symtropy already contains reusable foundations:
- `symtropy-terrain`: volumetric terrain, excavation/weathering events, substrate classes, and fluid coupling.
- `symtropy-basin`: water, soil, atmosphere, heat, toxin, radiation, trophic, and infrastructure-integrity state.
- `symtropy-physics`: rigid-body dynamics, contact, collision and deterministic replay.
- `symtropy-robotics-bridge-core`: robotics-facing types and traits.

GeoBio should compose these capabilities rather than create a competing terrain or physics engine. The first increment is an isolated, dependency-free reference kernel and research specification. It is not yet wired into the production workspace.

## Research documents

### Subterra — planetary subterranean programme

- [Subterra programme v0.1](SUBTERRA_PROGRAMME_v0.1.md)
- [Subterra SOTA and competitive analysis](SUBTERRA_SOTA_GAP_ANALYSIS_2026-11.md)
- [SubterraBench v0.1](SUBTERRA_BENCHMARK_PROTOCOL_v0.1.md)
- [Subterra site evidence schema](SUBTERRA_SITE_EVIDENCE_SCHEMA_v0.1.json)
- [Subterra non-uniform profile schema](SUBTERRA_PROFILE_SCHEMA_v0.1.json)
- [Synthetic non-uniform profile fixture](fixtures/subterra-nonuniform-profile-synthetic.json)


- [Architecture fit and integration boundaries](ARCHITECTURE_FIT_v0.2.md)
- [Research programme v0.2](RESEARCH_PROGRAMME_v0.2.md)
- [State-of-the-art gap analysis — 16 November 2026](SOTA_GAP_ANALYSIS_2026-11-16.md)
- [GeoBioBench protocol v0.2](BENCHMARK_PROTOCOL_v0.2.md)
- [BioAsteroid replication and process-comparison protocol](BIOASTEROID_REPLICATION_PROTOCOL_v0.1.md)
- [PressureForge analytical reference protocol](PRESSUREFORGE_REFERENCE_PROTOCOL_v0.1.md)
- [Resource graph record schema v0.2](RESOURCE_GRAPH_SCHEMA_v0.2.json)
- [Synthetic water-electrolysis fixture](fixtures/water-electrolysis-reference.json)
- [Reference kernel](kernel/) — elemental/resource accounting, bounded ideal-gas pressure reference and non-uniform subterranean morphometry
- [Implementation and validation status](IMPLEMENTATION_STATUS.md)

## Evidence contract

A geological occurrence is not an accessible reserve. An accessible reserve is not extracted feedstock. A process simulation is not a measured product. A measurement is not an independent reproduction. A test is not automatically a mission qualification.

Every result must retain its source, method, units, uncertainty, model/data digest, and evidence state. Unknown is not zero. Missing information must produce an unquantified or abstaining result, never a silent pass.

## Scope of the first kernel

The initial Rust kernel covers resource-flow validation, elemental balance, uncertainty propagation with explicit correlation groups, process-level inventory completeness, and simple resource-intensity metrics. It is not a bioprocess model, geomechanics solver, pressure-vessel solver, life-support controller, or certification tool.

The kernel is deliberately isolated from Symtropy's production workspace until the reference semantics have been reviewed and tested.
