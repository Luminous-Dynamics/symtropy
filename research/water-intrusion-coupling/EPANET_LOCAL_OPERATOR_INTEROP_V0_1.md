# EPANET Local Junction Operator Interoperability Contract v0.1

**Programme:** WATER-MIXING-ENGINE-INTEROP-001 / [#5290](https://github.com/Luminous-Dynamics/symtropy/issues/5290)  
**Dynamic operator:** [#5274](https://github.com/Luminous-Dynamics/symtropy/issues/5274), draft [#5288](https://github.com/Luminous-Dynamics/symtropy/pull/5288)  
**Evidence class:** API/source inspection and proposed synthetic hook contract  
**Evidence ceiling:** no EPANET fork, plugin or physical network qualification implemented

## 1. API inspection: what the Toolkit can and cannot observe

The OWA-EPANET Toolkit 2.3 provides a time-step quality loop:
- EN_openQ, EN_initQ, EN_runQ, EN_nextQ step between hydraulic events;
- EN_stepQ advances one water-quality time step at a time.

The Toolkit documents the following computed read-only result channels:
- node EN_QUALITY, EN_DEMAND, EN_DEMANDFLOW, EN_DEMANDDEFICIT, EN_SOURCEMASS, and tank values;
- link EN_FLOW, EN_LINKQUAL, EN_STATUS, EN_LINK_LEAKAGE and other hydraulic/energy values.

EN_LINKQUAL is useful as a per-link, time-series result. It is not the internal leading-segment list (parcel volume/concentration) consumed at a junction during the current quality step. The public Toolkit does not expose the exact parcel-consumption records or a setter for the computed junction EN_QUALITY. EN_QUALITY and EN_LINKQUAL are read-only computed results.

Sources:
- Toolkit quality stepping: https://wateranalytics.org/EPANET/toolkit-usage.html
- Toolkit computed result properties: https://wateranalytics.org/EPANET/toolkit-usage.html
- Read-only EN_LINKQUAL and node properties: https://wateranalytics.org/EPANET/group___enumerations.html
- Junction leading-segment consumption, weighted node quality and pipe-segment creation: https://usepa.github.io/EPANET2.2/12_analysis_algorithms.html

**Consequence:** a public-Toolkit-only sidecar can observe node and link results over time, but cannot truthfully reconstruct the exact inlet parcel mass that EPANET consumed at an internal junction step. Do not label reconstructed node/link data as leading-segment truth.

## 2. Supported integration lanes

### Lane A — public Toolkit observer

This lane is read-only and does not claim to inject a replacement local operator. It records:
- simulation time and quality/hydraulic step sizes;
- node quality, source mass, demand delivered/requested, leakage and tank state;
- per-link signed flow, computed link quality, status and leakage;
- source type/strength/pattern, model digest, engine version, unit system and solver options.

It supports:
- exact input/version provenance;
- time-series parity against unchanged EPANET;
- locating candidate junctions and operating regimes;
- comparing network-level outputs with external physical measurements.

It does not support:
- direct leading-segment mass/volume extraction;
- writing a computed junction concentration;
- assigning a different outgoing concentration to each link within the solver's internal parcel creation step.

The Lane A receipt must mark internal_segment_observation = unavailable and operator_writeback = unsupported.

### Lane B — engine-level hook / maintained fork

The local operator must be called where the engine has already collected actual incoming segment contributions for a quality time step and is about to create outgoing link segments. The base engine algorithm:
1. updates/reactions existing segments;
2. consumes leading segments from incoming links to account mass and volume into a node;
3. computes node quality (for a junction, total mass inflow divided by total inflow volume);
4. applies the external quality source semantics;
5. creates new segments in outflow links.

The fork should add an opt-in hook at the junction-specific part of this sequence. It should not rewrite hydraulics, parcel advancement, segment reactions, topological ordering, reversal handling or tank mixing.

The hook input must carry the exact per-link parcel contributions after the engine has determined how much leading segment volume is consumed this quality step. A reconstructed link average or link result quality is not an acceptable substitute.

## 3. Hook input contract

A synthetic/engine hook snapshot should include:

- exact engine name, version, source commit and build features;
- node identity and geometry revision;
- quality timestamp, quality time step and time to next hydraulic event;
- incoming link identity, signed flow direction, segment IDs, consumed segment volumes and each segment concentration;
- explicit external source records and their exact engine semantics;
- outgoing link identities and their hydraulic volumes for the same quality step;
- demand, emitter, leakage or other terminal withdrawal volumes;
- supported local operator identity, state digest and qualified domain;
- time/units/provenance digests and current flow-reversal state.

The snapshot must distinguish a segment consumed amount from the segment's entire volume and distinguish link-wide computed quality from the leading parcel quality.

## 4. Source, demand, leakage and storage semantics

The local operator's control volume must account for:
- mass from consumed incoming pipe segments;
- explicit external source mass/volume;
- outgoing pipe-parcel mass on each outgoing link;
- demand/terminal withdrawal mass;
- leakage/other sinks;
- known local inventory change;
- unresolved residual.

EPANET's source types have different semantics:
- CONCEN: concentration of external inflow;
- MASS: adds prescribed mass flow;
- FLOWPACED: adds concentration relative to mixed inflow;
- SETPOINT: sets outgoing concentration subject to its documented rule.

The initial hook should only enable source modes whose exact mass/volume contribution is represented in its contract. Unsupported FLOWPACED/SETPOINT or complex source interactions must disable local dynamic mode at that node until separately implemented and tested. Do not double-count a booster that the base engine applies after blending.

Positive demands, delivered demand under pressure-dependent demand, emitters and leakage must each be represented exactly once as terminal mass sinks. If the local operator cannot assign/derive an outlet mass for a terminal withdrawal, dynamic mode must refuse or the feature scope must explicitly disable at that node.

Any local storage/memory term is signed: positive inventory delta means mass retained; negative means release of previously retained mass. If it is not independently modelled/identified, keep it in unresolved residual; do not infer a physical storage mechanism from a delayed curve alone.

## 5. Hook output contract

The hook returns a mass flux/concentration for every outgoing pipe port and every supported terminal withdrawal port, plus a signed local inventory change or an explicitly unresolved term.

For each outgoing pipe:
- its flow/volume must be the engine's exact same-step hydraulic output;
- mass must be finite and nonnegative for a passive conservative tracer;
- concentration is outlet_mass / outlet_volume only for positive volume;
- zero-flow port must have zero assigned outgoing mass unless a separately modeled storage term explains it.

The fork then creates the usual parcel segment for each outgoing link using that link's own concentration. Preserve the existing segment merging tolerance and ordering semantics unless there is a separately tested reason to change them.

A node receipt is emitted per quality step with the signed mass balance:

incoming_segment_mass + external_source_mass - outgoing_pipe_mass - terminal_withdrawal_mass - inventory_delta = unresolved_residual

Never normalize outgoing port concentrations to repair a nonzero residual.

## 6. Flow reversal and state continuity

At hydraulic events, EPANET may reorder pipe parcels when flow reverses and re-sort nodes topologically. The local hook must:
- derive current inlet/outlet port roles from signed engine flow at every step;
- bind segment IDs to the link identity and direction for that time step;
- define whether the local kernel state is invariant under reversal;
- reset, remap or abstain when reversal changes the operator's physical orientation or leaves its qualified envelope;
- never carry an old inlet/outlet role map across reversal without an explicit transition.

## 7. T0 hook arithmetic prototype (next sub-increment)

Before touching the EPANET engine, implement a small independent Rust validator for a supplied hook snapshot. It should verify:
- consumed-segment mass calculation;
- external-source support/unsupported semantics;
- port identity completeness and no duplicate IDs;
- every pipe/terminal outlet assigned exactly once;
- per-port mass-to-concentration conversion and zero-flow refusal;
- node water-volume and tracer-mass residuals;
- signed residual and abstention instead of normalization;
- zero-delay complete-mix parity;
- stale/mismatched engine, step, geometry and operator provenance rejection.

This is a synthetic contract test. It is not a simulation and does not prove the Toolkit can supply the snapshot fields. Exact-segment snapshot availability requires the Lane B fork hook.

## 8. Differential test matrix

| Test | Required invariant |
|---|---|
| Hook disabled | Unmodified EPANET outputs remain parity-compatible |
| Zero-delay operator equal to complete mixing | Outgoing link concentrations match baseline node concentration |
| Static outlet-resolved split | Each outgoing segment receives its declared port concentration; total mass reconciles |
| Dynamic kernel | Port response matches the discrete kernel oracle on the same quality time grid |
| Multiple leading segments | All consumed portions, not entire segments, contribute exact mass |
| Hydraulic event step | Correct quality time step and current hydraulic state are bound |
| Flow reversal | Segment order and port-role transitions remain correct |
| External sources | Exact documented source semantics preserved or dynamic mode refuses |
| Demand/leakage | Every terminal sink accounted once |
| Tank adjacency | Existing tank model unchanged unless separately qualified |
| Reactions | Existing segment-reaction path unchanged |
| Missing port/snapshot field | Hook rejects; no guessed link mapping |
| Residual outside uncertainty | Open/indeterminate result; no normalized closure |
| Operator out of domain | Refusal or governed baseline selection, never falsely qualified fallback |

## 9. Industry parity and competitive strategy

Use EPA EPANET 2.2 as the public reference. Use OWA Toolkit 2.3 as the observable API implementation, with exact version and build pinned. Test Bentley WaterGEMS/WaterCAD only with exact product build, engine compatibility mode, solver options and licensed executable access; Bentley's own pages have had inconsistent EPANET compatibility descriptions, so no blanket product claim is justified.

The competitive proposition is a local qualification layer that preserves network-engine parity when disabled, is compatible with the engine's parcel transport, and adds outlet-resolved predictions only for nodes and regimes with sufficient independent physical evidence.

## 10. Evidence ceiling

This contract and any synthetic validator do not demonstrate an EPANET fork, an operational plugin, a physically correct mixing operator, a real tracer measurement, a qualified sensor, potability, contamination, toxicology, SANS 241 compliance or water-release authority.
