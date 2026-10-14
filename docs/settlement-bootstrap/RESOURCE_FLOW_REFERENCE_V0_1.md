# CRASH-BOOT resource-flow reference corpus v0.1

**Status:** synthetic reference semantics; not an engineering model, life-support design, operational checklist, or survivability claim.

This tranche makes one part of [CRASH-BOOT-00](https://github.com/Luminous-Dynamics/symtropy/issues/1618) independently inspectable: resource accounting under bounded inputs, service-capacity limits, disconnected groups, shared dependencies, conflicting evidence, and unqualified resources.

## Why this is the next slice

A settlement planner for 1 to 30,000+ people must not get credit for multiplying one camp's average demand by a population count. It must separately track (a) resource inventory, (b) per-time demand and inflow, (c) installed versus functioning service capacity, (d) whether services can reach each group, and (e) whether the evidence and model are qualified for the question being asked.

The public corpus is deliberately small and hand-inspectable. Its 30,000-person case is a topology test, not an extrapolation that assumes all services scale linearly:

- machine-readable vectors: `docs/settlement-bootstrap/fixtures/resource_flow_reference_v0.1.json`;
- independent standard-library oracle: `scripts/verify_crash_boot_resource_flow_v0_1.py`;
- focused CI: `.github/workflows/crash-boot-resource-flow-reference.yml`.

The checker validates the fixture and arithmetic contract only. It does not run Symtropy's physics engine, estimate human life-support needs, validate an environmental model, or authorise an action.

## Frozen accounting semantics

For a single resource over one declared time interval, the fixture checks:

```text
demand =
    population × qualified per-person rate
  + fixed system load
  + declared growth/production load
  + declared losses
```

Every quantity is an interval with compatible dimensions. Lower and upper demand bounds are calculated from the corresponding bounds. Qualified inflow is handled separately from demand; neither gross stock nor installed capacity is automatically usable.

The 12-person interval example is **synthetic only**:

- demand: 24–43 abstract resource units/day;
- usable inventory: 120–180 abstract units;
- qualified inflow: 0–8 abstract units/day;
- net depletion envelope: 16–43 units/day;
- arithmetic runway envelope: 120/43 to 180/16 days (about 2.79–11.25).

This is rectangular interval arithmetic, not a probability distribution, percentile, independent-random-variable assumption, or real survival estimate. It is only valid for the bounds stated in the fixture. Correlations, discrete events, changing populations, failures, and time-varying demand require a richer model and must not be hidden inside this example. A separate scale probe checks only the synthetic per-person contribution at 1, 12, 100, 1,000, 10,000 and 30,000 people; it explicitly does not claim fixed loads, service capacity or network topology scale linearly.

The reference capacity gate has 30 units/day available. The minimum supported demand is 24; the maximum is 43. Because the maximum exceeds the capacity, the result is **not certified for the full bound**—not an average-demand PASS.

## Failure modes the oracle freezes

1. **Population and topology:** scale probes are 1, 12, 100, 1,000, 10,000 and 30,000. The scale arithmetic covers only the synthetic per-person contribution, not total system demand, rated capacity, distribution or measured performance.
2. **Local distribution at 1,000:** synthetic total demand and nominal capacity are both 2,000 units/day, yet one disconnected 400-person habitat receives zero. Aggregate capacity must not mask local service failure.
3. **Local distribution at 30,000:** three habitats have total demand of 60,000 units/day and delivered capacity of 60,000. The central 10,000-person habitat is 2,000 units/day short while the south habitat has 2,000 units/day of surplus; its transfer path to the central habitat is unavailable. Therefore total delivery equality still does not establish subgroup coverage.
4. **Unknown environment:** unknown atmosphere and pressure remain UNKNOWN; Earth values are never silently inserted.
5. **Qualification of stock:** quarantined, treatment-unqualified inventory remains visible as gross stock but contributes zero to currently allocatable stock until the necessary release evidence exists. This is an allocation rule, not a statement that the physical material has vanished.
6. **Dimensions:** mass flow and volume flow are incompatible without explicit, qualified conversion evidence.
7. **Common-mode failure:** three pumps sharing one failed power bus are three unavailable components, not three independent backups.
8. **Inventory reconciliation:** a lost transfer acknowledgement cannot cause the same lot to be counted at both source and destination.
9. **Evidence conflict:** stale and contradictory observations stay unresolved without separate adjudication evidence; last-write-wins is forbidden.

## Dependency graph and recovery gates

The JSON fixture also contains a separate `dependency_graph_reference` contract. Edges mean **required dependencies**: every declared prerequisite must be available and qualified before a dependent node is counted as available. Alternative groups are explicit: a service is supported only if at least one all-of group has every member in `VERIFIED_AVAILABLE` state.

The six graph cases freeze the following behaviors:

1. A failed power bus propagates through the water pump, water loop, and clinic-hygiene service.
2. Two pumps on the same failed source do not count as independent redundancy.
3. A separately qualified manual fallback can remain available while the powered pump is lost.
4. A repair report does not restore service while the repaired controller remains `REPORTED_REPAIRED_UNVERIFIED`; dependent services remain unavailable pending requalification.
5. A cyclic dependency graph is rejected instead of guessing a fixed point.
6. A reference to an undeclared dependency is rejected, with the missing identifier retained for diagnostics.

This is a deliberately small graph-logic oracle, not a complete reliability model, fault-tree analysis, probabilistic risk assessment or diagnosis engine. It assumes the graph and qualification states have already been supplied by an authoritative evidence process; this code does not authenticate physical sensors, prove that a manual fallback exists, estimate failure rates, validate redundancy independence, or direct repairs. A future implementation needs typed node/edge identities, evidence lineage and freshness, units/capacity, time-dependent transitions, cycle detection, common-cause groups, and explicit repair/requalification receipts.

## Research implications

The contract follows the structure of established human-spaceflight systems engineering without pretending their values transfer to an unknown planet:

- NASA's [ECLSS overview](https://www.nasa.gov/reference/environmental-control-and-life-support-systems-eclss/) treats air revitalization, oxygen generation, water recovery, waste, ventilation, and fire response as distinct but coupled subsystems.
- NASA's [ISRU overview](https://www.nasa.gov/overview-in-situ-resource-utilization/) supports keeping deposit detection, accessibility, extraction, processing and usable output distinct.
- NASA's active [Standard for Models and Simulations, NASA-STD-7009B](https://standards.nasa.gov/standard/NASA/NASA-STD-7009) and [2026 implementation handbook](https://standards.nasa.gov/standard/NASA/NASA-HDBK-7009) emphasize model credibility for a declared use, including evidence and uncertainty. Reproducible arithmetic by itself is not model validation.
- NASA's [Spaceflight Human System Standard, Volume 2](https://www.nasa.gov/reference/nasa-std-3001v2/) (July 15, 2026 edition) is a relevant reference for crew interfaces, habitability and environmental health in designed spaceflight systems—not a complete standard for an arbitrary extraterrestrial environment.
- NASA's [planetary-protection overview](https://sma.nasa.gov/sma-disciplines/planetary-protection/) motivates retaining unknown biological status and contamination pathways as explicit constraints.

These references establish subsystem boundaries and verification expectations, not extraterrestrial deployment readiness, acceptable exposure limits for an unknown world, or numerical inputs for this corpus.

## Admission and non-claims

This file set does **not** relax the gates tracked by [WORLD-START product PR #1422](https://github.com/Luminous-Dynamics/symtropy/pull/1422), [its exact-subject qualifier PR #1423](https://github.com/Luminous-Dynamics/symtropy/pull/1423), or [model-suitability issue #1173](https://github.com/Luminous-Dynamics/symtropy/issues/1173). Their exact-subject qualification must be checked independently; an unfinished or queued run is not a pass.

No new production planner, actuation, medical triage, habitat control, resource recommendation, alien-chemistry processing, or command authority is introduced. The next step after this reference corpus is independent review of the input contract and negative controls, then an independently implemented verifier and broader scale/distribution vectors. Domain experts and physical testing remain prerequisites for any eventual real-world use.
