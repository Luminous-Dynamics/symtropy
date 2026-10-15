# Garden-to-Facility Multiphysics and Scale Scenario Contract v1

**Status:** synthetic-only scenario contract; not a live facility model or medicine-manufacturing procedure.  
**Parents:** [health supply resilience scenario PR #1620](https://github.com/Luminous-Dynamics/symtropy/pull/1620), [typed robotics digital-twin world PR #1587](https://github.com/Luminous-Dynamics/symtropy/pull/1587), [energy and thermodynamic ledger hardening PR #1622](https://github.com/Luminous-Dynamics/symtropy/pull/1622).

## 1. Purpose and boundary

Define a deterministic scenario family for comparing garden-scale infrastructure and its regional support services: irrigation, pumps, storage, greenhouse/environmental loads, power, monitoring, harvest logistics, transport, waste and a separately modeled authorized-facility capacity envelope.

This is a proposed contract, not a claim that Symtropy already contains a complete coupled agro-hydrological-thermal model. Reuse existing event/replay, fluid/thermal, resource-flow and robotics-world primitives where suitable. Add a subsystem only after a capability census proves the contract is missing.

The model does not infer whether a species is medicinal, whether soil/water is safe, whether material is authentic, whether a preparation treats a condition, whether a medicine is stable, or whether a facility has legal/quality authority. Those facts must arrive as externally assessed states. No formulation, dose, extraction process or drug-manufacturing parameters belong in the scenario.

## 2. Multi-scale meanings

The scenario distinguishes four transformations:

- **Site replication:** another garden is added; locality, water, land permission, hazard and governance evidence are independently bound.
- **Network pooling:** sites share seedlings, tools, testing logistics, storage or transport. Modeled savings count only when capacity, lead time and governance constraints are represented.
- **Facility capacity change:** the same operation targets a different volume/rate range and therefore requires a new capability/qualification profile.
- **Physical scale transfer:** geometry/equipment/process architecture changes. It requires domain-specific similarity and validation; geometric scaling alone proves no equivalence.

A compact modular unit is not automatically more efficient. At small scales, fixed overhead, dead volume, sensor resolution, surface-to-volume effects, minimum stable flow, cleaning, maintainability and operator skill can dominate. A scenario may compare alternatives, but cannot transfer physical process qualification or legal permission.

## 3. Versioned world/scenario envelope

Reuse the current typed scenario/world envelope and record:

- schema and engine/profile version; scenario ID/generation; fixed timestep and horizon;
- seed if stochastic sampling is explicitly enabled (the first reference corpus should be deterministic);
- site/facility/network geometry and coordinate frames by immutable asset reference;
- climate/water/power observations with source, observation window, retrieval time, quality and units;
- declared synthetic assumptions, approximations and excluded mechanisms;
- physical model/solver identities, exact input/output artifacts, parser and validation state;
- resource and mass/energy balances, uncertainty and limitations;
- external authority references for land permission, sampling disposition, botanical identity, buyer specification, facility capability or quality release;
- independent verifier version and replay receipt.

Source state is per field: `Observed`, `ExternallyReviewed`, `SyntheticAssumption`, `Derived`, `Unknown`, `Stale`, `Conflicting` or `Restricted`. A whole-run label must not erase mixed evidence states.

## 4. Typed system entities

- `GardenSite` — coarse geometry, approved land-use state, water-source reference, power/service dependencies, growing-area bound and hazard-review state.
- `WaterSource` — source identity and access review, measured quantity/quality references, storage capacity and freshness. Quality acceptance is external, not inferred from imagery.
- `ResourceReservoir` — water, electrical energy, thermal energy or other declared stores with finite capacities and units.
- `EquipmentNode` — pump, tank, sensor, greenhouse envelope, gateway or generic facility equipment with profile and capability/currentness reference.
- `MaterialLot` — generic horticultural lot/harvest identity and quantity; not a medicine product or quality claim.
- `TransportEdge` — route, mode, time/capacity envelope, availability, service dependencies and disruption events.
- `FacilityNode` — abstract downstream node; missing current capability, permission or quality references block the modeled operation.
- `WasteFlow` — output stream, accumulation, transfer and authorized destination reference; unhandled waste is a constraint violation.
- `EvidenceEvent` — immutable event/source lineage, correction/supersession pointer and deterministic sequence.

No patient-level demand or household-specific health data are required. Demand should remain aggregate and synthetic until a distinct lawful governance decision exists.

## 5. Physical equations and claim ceilings

Models must declare scope and simplifications. Independently recompute at least:

- water inflow + declared additions - withdrawal - declared loss = storage change;
- energy input + regeneration - useful output - declared dissipation = stored-energy change;
- thermal storage/heat transfer where a validated thermal model exists;
- material input/output and waste quantities with explicit boundaries;
- finite capacity, flow/rate constraints, service outages and time-window consistency.

Tolerances must be explicit and domain-appropriate. A conservation mismatch is a violation unless a bounded numerical tolerance explains it. Passing a ledger proves accounting consistency only—not physical validity or model adequacy.

Use an evidence hierarchy: analytically checked baseline → verified numerical solver → comparison against an independent solver/reference → calibration against a named physical article → held-out validation in a declared envelope. Do not claim a maturity level higher than the weakest required model or input.

On scale changes, document which terms remain invariant and which must be reidentified. Reynolds/Peclet/Biot similarity, for example, is mechanism-specific, not a universal badge. Unknown necessary parameters produce `Unresolved` or `OutOfDomain`, not an invented value.

## 6. Mandatory scenarios and negative controls

1. Water interruption with finite storage and explicit replenishment.
2. Missing/conflicting water-quality evidence blocks the dependent horticultural use; no safe-by-default state.
3. Power outage disables dependent pumps/monitors unless a declared backup satisfies its own capacity constraints.
4. Thermal excursion outside a calibrated model domain is reported as out-of-domain, not extrapolated silently.
5. Unknown or failed site-hazard status cannot be outweighed by a favorable climate score.
6. Multiple gardens compete for a shared testing/transport resource; capacity and scheduling are conserved.
7. A harvest lot changes identity/quantity; downstream custody references must update without borrowing old evidence.
8. Downstream facility lacks current permission/capability/quality reference; simulated routing is blocked.
9. Scale-down costs less but violates controllable flow, monitoring limits or waste capacity; candidate fails.
10. All synthetic demand is served, but output still says real-world service sufficiency is unproven.
11. Same-tick events arrive in different source order; replay remains deterministic.
12. Corrupt, stale, duplicated, conflicting or unverified artifacts produce an explicit unknown/block.
13. Stuck valve/pump, sensor drift/loss and communication delay change outputs within the declared model boundary.
14. An efficacy, therapeutic equivalence, shelf-life, manufacturing authorization or batch-release claim appears as a simulation output; schema/admission rejects it as out of scope.

Every fixture needs an expected transition and separate checker over raw event/artifact rows. Keep negative controls in the corpus.

## 7. Reference scenario: one benign water/energy cell

Begin with a Johannesburg community-garden support cell, not a medicine factory:

- one finite tank, pump, irrigation zone, power source and explicitly synthetic backup assumption;
- synthetic, labelled interval inputs;
- modeled storage, pump power, declared irrigation demand and reported sensor health only;
- interruption, leak, stuck-actuator, sensor-loss and over-capacity cases;
- independent recomputation of water and energy totals;
- immutable inputs, run metadata, output rows and verifier receipt.

Compare later with a supervised physical water/energy monitoring experiment before changing model maturity. Extend to a regional network and aggregate transport/testing capacity only afterward. The downstream regulated facility remains an abstract, policy-gated node until a qualified party provides the exact capability and scope.

## 8. Acceptance criteria

- Deterministic replay for frozen inputs and engine/profile versions.
- Strict units/ranges; invalid state rejected before mutation.
- Independent recomputation of balances, capacities and final report fields.
- Exact input/output/source identities and honest synthetic/external distinctions.
- Fail-closed handling of unknown, stale, conflicting and restricted evidence.
- No patient fields, exact residential coordinates or free-text sensitive traditional knowledge in shared scenarios.
- No medicine efficacy, equivalence, stability, authorization or release conclusions from simulation.
- No promotion of physical/manufacturing maturity solely from software execution.

## 9. References

- [NIST Digital Twins for Advanced Manufacturing](https://www.nist.gov/programs-projects/digital-twins-advanced-manufacturing): trustworthy twins need validation and quantified uncertainty.
- [Symtropy health-supply resilience scenario](https://github.com/Luminous-Dynamics/symtropy/pull/1620).
- [Symtropy thermodynamic ledger hardening](https://github.com/Luminous-Dynamics/symtropy/pull/1622).
- [SANBI Medicinal Garden](https://www.sanbi.org/gardens/free-state/tours/medicinal-garden/): SANBI cautions that educational labels do not guarantee efficacy/safety and some species are poisonous.
- [WHO quality assessment of herbal medicines and contaminants](https://www.who.int/publications/i/item/9789241594448).
