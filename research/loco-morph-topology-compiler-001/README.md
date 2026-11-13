# LOCO-MORPH-TOPOLOGY-COMPILER-001 — Validity-preserving graph compiler for modular embodiment search

**Status:** protocol and synthetic graph fixtures. No physical design, structural-strength or safety result is claimed.

Issue: [#4367](https://github.com/Luminous-Dynamics/symtropy/issues/4367).

## Objective

Compile generated modular embodiments into exact, buildable and traceable module/connection graphs before CAD, geometry, FEA, simulation or hardware evaluation.

Yu et al., *Agile legged locomotion in reconfigurable modular robots*, PNAS 123(10), e2519129123 (10 March 2026; https://pmc.ncbi.nlm.nih.gov/articles/PMC12974426/), explore a huge modular-body design space using an eight-dimensional VAE latent and Bayesian optimization. The authors report that decoded candidates do not always reconstruct a hand-designed morphology exactly. A topology mismatch changes load paths, module capabilities, power, sensing, control authority and the meaning of downstream evidence.

The paradigm shift is **proof-carrying topology proposals**:
- latent/generative search may propose;
- the compiler validates exact discrete graph semantics;
- CAD/geometry/structural analysis validates physical realizability;
- actuator/sensor/viability authorities validate operational capability;
- only an exact identity chain may carry the candidate forward.

A latent vector, natural-language description, approximate module count or filename is never an authoritative topology.

## Ownership and composition

| Concern | Owner | Compiler boundary |
|---|---|---|
| Candidate embodiment variables and nominal/degraded Pareto comparison | #4360 / #4361 / #4362 | Typed candidate identity and design objective context |
| Physical structural-damage campaign | #4364 / #4365 | Module-loss and dock/link fault sets |
| Actuator, drive and sensor faults | #4358 / #4359 | Component and common-cause fault sets |
| Complete conservative fault possibilities | #4342 / #4345 | Exact component/module connectivity signatures |
| Mode failover | #4332 / #4334 | Validated module/evidence graph identity |
| Robust viability and recovery | #4061 / #4083 | Physical reachable-successor authority |
| Qualified operating envelope | #4338 / #4340 | Exact topology digest in qualification cell |

The compiler owns the discrete graph schema, compatibility/cardinality rules, graph invariants, canonical encoding, static graph analysis and compile receipt. It does **not** claim geometry validity, collision freedom, material strength, FEA validity, actuator sizing, dynamic controllability, continuous viability or safety.

## Why one connectivity graph is insufficient

A module can be mechanically attached but electrically unpowered. A sensor may be physically mounted but lack a valid acquisition/clock path. A local controller can exist but lack command authority. Every embodiment therefore carries separate typed graphs:

1. **Structural/load graph:** modules, docks, joints, links and load-bearing paths.
2. **Energy graph:** sources, converters, batteries, drives, isolators and buses.
3. **Sensing graph:** sensors, acquisition nodes, clocks, calibration owners and consumers.
4. **Compute/communication/command graph:** processors, communication links, command authorities and module endpoints.
5. **Actuation graph:** motor/drive/gear/transmission endpoints and controlled joints or effectors.

An edge in one graph never implies an edge in another. Structural connectivity is not power, observation or command connectivity.

## Typed module and port contract

Each module declares stable module ID, class, revision and configuration digest; roles; local energy/compute/sensing/actuation/command capabilities; typed ports with stable IDs, interface class, allowed orientations, cardinality, connector revision and rating-source references; geometry/CAD identity and explicit units; and service/replacement operations.

Each edge declares stable edge ID, source module/port and target module/port; exactly one relation type; connector/interface revision and allowed orientation; directionality and graph-specific cycle policy; optional direction-specific capacity/rating digest; and exact configuration/mating-transform identity.

A physical dock may create several distinct typed edges, such as structural, power and data. Each relation is represented separately. A generic connected flag is insufficient.

## Compile algorithm

1. Validate manifest schema version and non-empty campaign, mission, generator and topology IDs.
2. Validate unique module IDs, module classes/revisions and capability declarations.
3. Validate each module's unique port IDs, finite cardinality and non-empty interface class.
4. Validate edge IDs, endpoints, port membership, interface compatibility, revision and allowed orientation.
5. Count port use against declared cardinality; reject over-subscription and forbidden self-connection.
6. Construct separate adjacency structures for each relation type.
7. Validate per-graph connectivity/required-component rules declared by mission and module roles. Do not force all subsystems into one component when independent autonomous modules are intentional.
8. Validate autonomous-module declarations: local power, compute, sensing, actuation and command capabilities must be explicitly supplied locally or by valid graph paths. No silent inheritance from a disconnected torso or remote host.
9. Validate graph-specific cycle policy; cycles may be allowed in power or structural graphs where schema declares. No universal tree assumption.
10. Compute deterministic module-removal and edge-removal component reports for declared fault sets. Report articulation modules/edges and isolated required services.
11. Produce canonical sorted topology data and digest input; return typed compile errors without a partial success receipt.

All enumeration and analysis are bounded. Exceeding a declared module/edge/fault-removal bound returns ResourceExhausted, not a partial topology marked Valid.

## Required compile invariants

- IDs are non-empty and unique within their declared namespace.
- Every edge endpoint and port exists.
- Port compatibility, revision and orientation rules pass.
- Port cardinalities are respected.
- Self-edges and duplicate edges are rejected unless explicitly supported by a future typed relation.
- Required graph connectivity follows explicit mission rules.
- Independent autonomous modules have all required local capabilities or valid declared paths.
- Power, sensor, compute, command, actuation and structural reachability are checked independently.
- No remote compute dependency is silently replaced by a local autonomy claim.
- Canonical topology identity is invariant under input-order permutation.
- Any graph-changing edit changes the topology digest.
- CAD/URDF/USD/BOM mappings bind module, port, edge and revision identities exactly.
- Static graph analysis is not represented as mechanical strength, dynamic reachability or safety certification.

## Graph-level fault analysis

For every declared single-module, single-edge and supported compound removal set, report connected components in each graph before/after removal; articulation modules and edges; modules disconnected from required energy, sensing, command, compute or actuation paths; remaining autonomous subgraphs and declared capabilities; mission functions no longer supported; and separate statuses for structural connectivity, functional autonomy and mission capability.

This is graph-theoretic screening only. It cannot determine whether remaining limbs physically support the body, whether forces/torques are sufficient or whether a safe gait exists. Those require geometry, structural and physical viability authorities.

## Search integration

Permitted proposal generators include VAE/latent morphology models (PNAS 2026 baseline), Bayesian optimization or evolutionary search over a typed graph grammar, grammar-guided module/port generation, human-authored candidates, and active experiment selection over previously compiled candidates.

All proposal outputs pass through this compiler. Invalid proposals receive stable typed rejection codes, but are never sent to physical simulation as valid candidates.

A continuous latent may guide a discrete graph proposal, but the discrete graph is the output identity. A decoder round-trip must reproduce the same canonical topology digest or emit a new candidate ID and undergo full revalidation.

## Frozen known-answer and adversarial corpus

Machine-readable fixtures: [fixtures/topology_graphs_v0_1.json](fixtures/topology_graphs_v0_1.json). Cases cover:
- valid conventional quadruped graph;
- valid autonomous modular assembly with separate structural/power/sensing/command graphs;
- compatible multi-edge dock represented by separate typed relations;
- wrong connector class/revision/orientation;
- duplicate module, port or edge IDs;
- over-subscribed port and impossible mating;
- structural graph connected while energy/command graph is disconnected;
- autonomous module missing local power, compute or sensing;
- intentionally disconnected autonomous components;
- cycles allowed by one relation type but forbidden by another;
- single module and dock-edge removals with exact component counts;
- latent decode mismatch and CAD/URDF identity mismatch;
- input permutation preserving canonical digest and report;
- resource-bound exhaustion with no partial Valid receipt.

## Proposed targets (not achieved)

- 100% exact agreement with an independent graph oracle on the finite fixture corpus.
- Zero incompatible, duplicate or over-subscribed connections compiled.
- Zero autonomous-module status without explicit local or reachable power/compute/sensing/actuation/command capabilities.
- Zero cross-graph inference (e.g. structural edge implies power edge).
- 100% canonical permutation invariance and graph-change digest sensitivity.
- 100% detection of invalid decoder round-trips and artifact identity mismatches.
- Complete static single-module and edge-removal connectivity report for the declared finite fault set.
- Zero physical-safety, geometry or strength claims emitted by the compiler.

## Claim ceiling

A compile receipt establishes only that a discrete topology obeys declared schema and graph constraints. It does not establish physical buildability, tolerance stack, collision freedom, structural strength, electrical safety, control stability, robust viability or locomotion capability. Those remain downstream independent qualification gates.
