# Power Fabric Electrical Service Graph v0.1

**Status:** executable topology metadata kernel; not a load-flow solver, installation verification or electrical safety approval  
**Date:** 2026-11-14  
**Owner issue:** [POWER-FABRIC-SERVICE-GRAPH-001 #5183](https://github.com/Luminous-Dynamics/symtropy/issues/5183)  
**Human-facing consumer:** [#5158](https://github.com/Luminous-Dynamics/symtropy/issues/5158)  
**Module compatibility parent:** [#5180](https://github.com/Luminous-Dynamics/symtropy/issues/5180)

## 1. Purpose

Represent the declared electrical topology as a deterministic, typed graph from which Power Fabric can derive source maps, circuit directories and role-specific views.

The graph answers:

- Which sources are declared in this design?
- Which declared directed power paths connect those sources to a listed load?
- Which protective devices, meters, controllers, thermal modules and isolation points are associated with which declared assets?
- What is the graph revision, declared scope and evidence maturity?
- Which path evidence supports each possible source-to-load relationship?

It does **not** answer whether a conductor is energized, whether a source has been physically discovered, whether a protective device is correctly selected, whether the network is stable, or whether any equipment is safe to touch.

## 2. Node types

| Node kind | Purpose | Potential source for path analysis |
|---|---|---:|
| UtilitySource | Declared grid/PCC supply | Yes |
| RenewableSource | PV or another renewable source | Yes |
| GeneratorSource | Engine, turbine, fuel cell or other dispatchable source | Yes |
| Storage | Battery or other store capable of discharge | Yes |
| Converter | Inverter, rectifier, DC/DC, transformer/SST | No |
| AcBus / DcBus | Declared distribution bus | No |
| DistributionBoard | Board/panel node | No |
| Feeder / Circuit | Declared downstream distribution segments | No |
| Load | Appliance, room, process or critical service | No |
| ProtectiveDevice | Breaker, fuse, relay or other protective element association | No |
| IsolationPoint | Declared disconnect/isolation location | No |
| Meter | Measurement point | No |
| ThermalModule | Thermal sink/source/cooling association | No |
| Controller | Supervisory or local control association | No |

Storage is treated as a possible source because it may discharge. Charging is represented separately with the direction of its PowerPath edges. A source-to-load path is directional: use the direction in which the declared energy service is intended to flow.

## 3. Edge types

Only **PowerPath** edges participate in source reachability.

| Edge kind | Meaning | Counts as energy path? |
|---|---|---:|
| PowerPath | Directed possible energy-delivery relationship | Yes |
| ProtectionAssociation | Protective device associated with a circuit/section | No |
| MeasurementAssociation | Meter/sensor observes an asset or service | No |
| ControlAssociation | Controller is associated with an asset or service | No |
| ThermalAssociation | Thermal module/path is associated with an asset | No |
| PhysicalAssociation | Mounting, containment or physical relationship | No |
| AuthorityAssociation | Authorization/governance relationship | No |

A meter reporting a circuit's power does not create a physical power path. A controller's command relationship does not create a physical power path. This explicit typing prevents graph semantics from turning monitoring/control metadata into electrical connectivity.

### Module identity binding

A ServiceNode may retain the exact ModuleIdentity from the module contract: module ID, hardware revision, configuration digest and current/stale state. Source paths carry the corresponding module identities in node order. The graph-level assessment separately reports every stale module configuration so a resident/operator/practitioner view can mark affected diagrams and compatibility claims stale. The graph does not automatically refresh a module identity when an asset changes. The source-map assessment reports stale module identities separately from the graph revision's current/stale flag.

## 4. Evidence and revision

Each node and edge carries an evidence state:

- Unknown
- DesignDeclared
- FieldObserved
- IndependentlyReviewed

Known evidence states require a non-empty evidence ID. Unknown carries no evidence ID.

Each graph revision binds:

- graph ID;
- revision ID;
- declared content digest;
- written scope statement;
- graph maturity: DesignOnly, FieldReconciled or IndependentlyReviewed;
- a maturity-evidence reference;
- current/stale flag.

The constructor rejects a DesignOnly maturity with Unknown maturity evidence, FieldReconciled without at least FieldObserved evidence, and IndependentlyReviewed without IndependentlyReviewed evidence. This is a fail-closed consistency check, not source authentication. The evidence ID must resolve to an actual review/reconciliation record in the wider evidence system.

The digest is supplied by the caller and is not computed by this kernel. Evidence maturity is descriptive and does not equal a Certificate of Compliance, utility approval, proof of absence of voltage or safety certification. Field reconciliation is only within the written scope. It cannot prove that every concealed or undocumented physical source has been discovered.

## 5. Source path assessment

For every declared Load node, the assessment performs deterministic breadth-first search from each potential source over PowerPath edges only. It returns **one deterministic shortest-hop witness path per source/load pair**, with ties resolved by sorted node/edge identity. It does not enumerate every possible route, edge-disjoint backup path or common-cause cut set. Those require a separate bounded path/min-cut analysis.

Each reported path retains:

- source node ID;
- ordered node IDs;
- ordered edge IDs;
- the evidence state/reference for every node;
- the evidence state/reference for every edge.

Paths and output collections are ordered deterministically. The machine result declares its path-witness policy as OneShortestHopPerSourceLoadPair. Cycles are visited once per source. The assessment reports:

- DeclaredSourcePathFound when one or more declared sources have a path to the load;
- NoDeclaredSourcePath when no path exists in this graph.

**NoDeclaredSourcePath is not equivalent to unpowered, disconnected, isolated or safe.** It means only that the current graph revision contains no directed PowerPath from the declared source set to that load.

Likewise, DeclaredSourcePathFound is not proof of live flow. It is a topology assertion, not a meter reading or power-flow calculation.

A stale graph may still be inspected for its historical declared paths, but the assessment must retain revision_current=false so consumers cannot represent it as the current as-built state.

## 6. Determinism and integrity rules

The graph constructor:

- rejects empty IDs/labels;
- rejects duplicate node and edge IDs;
- rejects dangling edge endpoints;
- rejects self-referential edges;
- rejects Meter, ThermalModule and Controller nodes on PowerPath edges;
- rejects Load nodes as the origin of a PowerPath;
- sorts nodes, edges and path results deterministically.

PowerPath edges may pass through source/storage, converter, bus, board, feeder, circuit, protective device, isolation point and load nodes. This is graph topology only; it does not validate the protective device's rating, interrupting capability, coordination, grounding or isolation function.

## 7. Initial synthetic reference system

The associated fixture describes a hypothetical grid + PV + battery system:

- utility source to AC bus;
- PV source through a declared inverter path to the bus;
- battery storage through a declared inverter path to the bus;
- bus to distribution board, circuit and lighting load;
- separate protection and measurement associations.

The fixture is a synthetic topology contract. Values and graph evidence IDs are invented for test purposes and are not physical equipment, installation data or a design recommendation.

## 8. Hostile cases

- A load is connected only by a MeasurementAssociation: no power path is reported.
- A load is connected only by a ProtectionAssociation: no power path is reported.
- A source is omitted from the graph: the kernel cannot discover it; the source map remains a declaration bounded by the graph's scope/evidence.
- A load has no path: result is NoDeclaredSourcePath, never "de-energized".
- A graph revision is stale: paths remain tagged as historical/stale.
- Two sources can reach one load: both paths are listed.
- A cyclic network: BFS terminates and does not duplicate a source path.
- A meter or controller is inserted into a PowerPath: graph construction fails.
- A load is made the origin of a PowerPath: graph construction fails.
- Unknown evidence: remains explicitly unknown and is preserved in path evidence.

## 9. Boundaries and next extensions

This first increment does not model:

- AC/DC voltage levels, impedance, current, power or phase on graph edges;
- load-flow, frequency/voltage stability or grid-forming capability;
- short-circuit, arc-flash, relay coordination or grounding;
- conductor/cable selection, protection settings or switchgear approval;
- physical isolation or absence-of-voltage proof;
- telemetry freshness reconciliation or live as-built discovery;
- standards or CoC issuance.

Those belong to their existing qualified owners or separate typed contracts. The next useful extension is to bind graph nodes to the module contract's exact module/configuration identity and add role-specific projections for resident, operator, practitioner and emergency views.
