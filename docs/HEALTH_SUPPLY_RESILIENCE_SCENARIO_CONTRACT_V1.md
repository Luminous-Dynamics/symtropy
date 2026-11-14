# Essential-Health-Product Supply Resilience — Scenario Contract v1

**Status:** synthetic, non-clinical scenario design  
**Tracking:** [HEALTH-SUPPLY-SIM-001 #1617](https://github.com/Luminous-Dynamics/symtropy/issues/1617)  
**Cross-project parent:** [Mycelix MED-OPEN-001 #5027](https://github.com/Luminous-Dynamics/mycelix/issues/5027)  
**Research checked:** 14 October 2026

## 1. Scope and non-claims

This profile uses Symtropy's existing deterministic engineering scenario/replay substrate to test whether an *abstract supply network* can continue delivering items when suppliers, transport, storage evidence, quality disposition, power or communications fail.

It does **not** simulate physiology, dosage, therapeutic equivalence, clinical outcomes, pharmacokinetics, shelf-life chemistry, temperature degradation curves, or real medicine efficacy. It is not a drug or cold-chain physics model. A temperature deviation, missing monitor history or quality problem enters the simulation as an externally supplied status event and may force a hold/quarantine; Symtropy cannot infer a real product's safe temperature window.

Do not create a second simulation runtime. Use existing deterministic event scheduling, state-machine and replay primitives after checking the supported engine/model-path capability census and suitability gate.

## 2. State must not collapse

Every inventory/lot observation distinguishes these states:

- `ReportedAvailable`: a source claims stock exists; the source may not be independently validated.
- `VerifiedAvailable`: an authorized source supplies current stock evidence for the exact site/product/lot scope.
- `Unknown`: required observation is absent, contradictory or too stale to support the claim.
- `PendingQuality`: lot is awaiting the declared disposition from its qualified quality owner.
- `Quarantined`: the exact lot is held from simulated dispatch/use.
- `Recalled`: a scoped recall notice applies.
- `Expired`: the source's product/lot state records expiry; the simulator does not calculate a date from product chemistry.
- `NotReported`: no source report exists in the current feed window; this is not equal to zero stock.

No unverified inventory can be silently used to satisfy simulated demand. A quarantined or recalled lot cannot be dispatched. A stale snapshot must not be rendered as current availability. An alternate product cannot be considered substitutable unless a qualified external relationship explicitly supplies that scoped equivalence; the simulator itself cannot decide it.

## 3. Versioned synthetic model

The first reference profile should be deterministic and small enough to inspect by hand.

### Entities

- `ProductIdentity`: synthetic, stable product/configuration ID; intended-use and authorization references are external and not synthesized by this model.
- `SupplierNode`: sourcing node with explicit capacity and lead-time envelope; capability qualification state is externally evidenced.
- `LotNode`: exact lot ID and product ID; incoming quality, recall and storage-condition status.
- `InventoryObservation`: site/product/lot, quantity bound, state, source snapshot reference, observation time, expiry of the observation.
- `FacilityNode`: warehouse/distribution/qualified manufacturing node with declared capacities, power and communications dependencies.
- `TransportEdge`: lead time, capacity, availability state and replayable disruption events.
- `DemandEnvelope`: aggregate synthetic demand by product/site/time window; no patient/person records or treatment assumptions.
- `ServiceAccessNode`: aggregate access point at coarse geography, not a residence or individual.
- `AuthorityRef`: external quality disposition, license/authorization, source-rights decision or recall reference; a simulation cannot generate these facts.

### Event identity

Each event should bind:
- scenario ID and schema version;
- unique event ID;
- deterministic tick/time and stable tie-break sequence;
- subject IDs (product, lot, site, supplier, edge);
- event type and typed payload;
- source class: `SyntheticAssumption`, `ExternalObservationRef`, `QualifiedDispositionRef`, or `SimulationDerived`;
- source snapshot/review references when not synthetic;
- explicit units and bounds for numerical fields;
- correction/supersession pointer, where applicable.

Reject duplicate IDs, references to absent subjects, untyped units, invalid ranges and nondeterministic ordering. Sort same-tick events using a fixed stable key such as `(tick, declared_sequence, event_id)`. Frozen inputs plus the same engine/profile version must yield the same event/replay identity and final state.

## 4. Example manifest (all fictional)

```json
{
  "schema": "health_supply_resilience_scenario_v1",
  "scenario_id": "SYNTH-HSR-SUPPLIER-OUTAGE-001",
  "mode": "synthetic_only",
  "tick_unit": "day",
  "horizon_ticks": 30,
  "assumptions": {
    "demand_source": "synthetic_fixture",
    "clinical_interchangeability": "not_modelled",
    "quality_and_authorization": "external_evidence_only"
  },
  "products": [
    {
      "product_id": "SYNTH-PRODUCT-001",
      "authorization_state": "external_reference_required",
      "substitution_policy": "deny_unless_scoped_external_evidence"
    }
  ],
  "suppliers": [
    {
      "supplier_id": "SYNTH-SUPPLIER-A",
      "product_id": "SYNTH-PRODUCT-001",
      "qualification": "fixture_assumption_only",
      "lead_time_ticks": 3,
      "daily_capacity_units": 40
    }
  ],
  "inventory": [
    {
      "site_id": "SYNTH-HUB-01",
      "product_id": "SYNTH-PRODUCT-001",
      "lot_id": "SYNTH-LOT-A-001",
      "quantity_units": 80,
      "state": "VerifiedAvailable",
      "authority_ref": "SYNTH-OBS-001"
    }
  ],
  "demand": [
    {
      "site_id": "SYNTH-ACCESS-01",
      "product_id": "SYNTH-PRODUCT-001",
      "start_tick": 0,
      "end_tick_exclusive": 30,
      "units_per_tick": 10
    }
  ],
  "events": [
    {
      "event_id": "SYNTH-EVENT-001",
      "tick": 2,
      "sequence": 1,
      "type": "SupplierUnavailable",
      "subject_id": "SYNTH-SUPPLIER-A",
      "source_class": "SyntheticAssumption"
    }
  ]
}
```

The fixture is a parser/state-transition illustration only. Its capacities and quantities are invented and must not be used as a forecast for any real supply chain. The `fixture_assumption_only` supplier flag intentionally prevents this single fixture from proving real supplier qualification.

## 5. Required scenario corpus

Each case must freeze input identity, assumptions, expected invariants and an independently implemented checker.

1. **Supplier interruption:** unavailability reduces replenishment only after its event tick; already-held eligible stock is not erased.
2. **Transport delay/capacity loss:** affected routes change arrival times/capacity, not product efficacy.
3. **Unknown inventory snapshot:** `Unknown` or `NotReported` never becomes zero inventory or verified stock without new evidence.
4. **Lot recall:** only exact affected lot IDs and downstream edges are held; unaffected lots remain traceable.
5. **Missing storage history:** a temperature-sensitive lot becomes hold/quarantine when its externally defined storage evidence is absent/stale; no inferred stability claim is allowed.
6. **Pending or failed quality disposition:** lot cannot be dispatched while pending/quarantined/failed.
7. **Demand surge:** aggregate synthetic demand exceeds capacity; expose shortage and recovery-time ranges without patient-level outcomes.
8. **Second-source ramp-up:** new capacity becomes available only after externally scoped qualification, lead time and capacity assumptions; an intention or a successful simulation is not qualification.
9. **Power/communications loss:** nodes depending on the unavailable service cannot perform the modeled operation; no implicit failover.
10. **Conflicting source feeds:** retain contradiction/unknown state and report source lineage; don't add duplicates to inventory.
11. **Expiry/waste:** use explicit expiry events or dated external lot status; do not compute product stability from generic physical variables.
12. **Stale product authorization/license:** prevent the affected operation pending a current external authority reference; this is a simulated process gate, not legal interpretation.
13. **Derivative-source duplicate:** two feeds repeating the same observation cannot be counted as independent stock confirmations.
14. **Simulation optimism negative control:** even if the synthetic scenario satisfies all demand, output must not emit clinical efficacy, regulatory authorization, manufacturing qualification or real-world service sufficiency.

## 6. Metrics, with assumptions exposed

- Fraction of aggregate simulated demand met by tick and over the horizon.
- Stockout duration and time-to-recovery (ticks), bounded by the stated demand/capacity assumptions.
- Units and duration held in quality quarantine or affected by a scoped recall.
- Time/steps needed to trace the affected lot through every known downstream edge.
- Supplier concentration and single-source dependency count.
- Expired/wasted units only where expiry state is explicitly represented.
- Capacity/transport/power/communications shortfall.
- Impact of missing, stale, contradictory or duplicated feeds.
- Replay determinism, invariant violations and independent-checker agreement.

Every metric must state units, time range, denominator, observation coverage and whether it derives from a synthetic assumption or externally referenced observation. Do not present one resilience score that hides missing data or safety holds.

## 7. Authority and evidence boundaries

| Question | Owning project/system | What Symtropy may do |
|---|---|---|
| Does a source record exist, and may its contents be reused? | Mycelix rights/provenance contract, with source terms review | Consume an explicit rights state and block unavailable/restricted inputs |
| What does the scientific evidence support? | Accountable scientific/clinical owners; Symthaea assists synthesis and abstention | Compare scenarios only under explicit assumptions; never convert hypothesis into clinical truth |
| Is a product/facility authorized or a lot released? | Relevant regulator, qualified manufacturer and accountable quality authority | Consume exact external state references and simulate holds/failures |
| Where are aggregated suppliers/access points and how fresh is the data? | Sol Atlas licensed evidence snapshots/projection | Render aggregate geography, coverage and staleness with source lineage |
| What happens under supply-network disruption? | Symtropy scenario/replay substrate | Execute deterministic counterfactuals and report system-level consequence ranges |

The chain of authority is deliberately one-way: source rights, clinical/product authorization and quality dispositions are external inputs; simulation outputs never write back or mint those states.

## 8. Qualification and acceptance criteria

- Exact source head, scenario manifest, engine/profile version, ordered events, dependency/toolchain identity and test corpus are bound in a replay receipt.
- Independent checker reconstructs inventory/lot eligibility and demand met from frozen scenario data; it does not import the producer's verdict function.
- Negative controls cover each hard boundary above; golden outputs have a frozen identity/version.
- Numerical values include units and explicit bounds; reject NaN, infinity, negative capacities and invalid event ranges.
- No external dataset is copied without source-rights review. Start with fabricated synthetic values only.
- Full CI and focused scenario tests must complete at the exact subject head; queued and skipped runs are not passes.

## 9. Next integration points

- Mycelix [MED-OPEN-001 #5027](https://github.com/Luminous-Dynamics/mycelix/issues/5027) and [source-rights contract PR #5030](https://github.com/Luminous-Dynamics/mycelix/pull/5030) provide source rights/identity inputs.
- Symthaea [HEALTH-MFG-000 #5969](https://github.com/Luminous-Dynamics/symthaea/issues/5969) separates manufacturing infrastructure evidence from clinical and regulatory authority; its [public-health reasoning ladder #135](https://github.com/Luminous-Dynamics/symthaea/issues/135) owns source-backed reasoning and abstention.
- Sol Atlas [ATLAS-DSCI-001 #99](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/99) owns licensed scientific dataset snapshots and geographic evidence projections.

**The goal is dependable access under disruption, not automated treatment decisions.** A simulation is useful when it exposes hidden dependencies and uncertainties—especially when it refuses to call unknown stock, unqualified substitutes or unreleased lots “available.”
