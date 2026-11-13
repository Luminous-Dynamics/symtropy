# Aero replenishment strategy v0

**Status:** exact finite DAG path enumerator and Pareto evaluator over synthetic mission edges. It is not an aircraft dynamics model, battery model, charger physics model, site scheduler, session controller or safety/flight permit.

## Research objective

Compare battery-only continuation, conductive/robotic charging, IPT/CPT docking, battery exchange and qualified airborne replenishment options over the same frozen aircraft mission and battery reserve.

The model consumes upstream-declared edge costs. It does not infer propulsion, battery acceptance, charging losses, contact wear, receiver drag or availability from a bearer label.

For each edge:

```text
E_next = E_current - aircraft_energy_draw + battery_terminal_energy_accepted
```

Every intermediate battery state must remain within the declared capacity and reserve. A complete path must reach the terminal mission node by the declared deadline.

## Paradigm shift

The optimization target is not "best charging technology." It is the best **mission replenishment strategy** across a set of qualified opportunities. A path may combine modes where compatible, but every installed aircraft asset contributes its mass once per path, and every replenishment source-energy input is charged at the case's common source-side real-energy plane.

The evaluator emits a non-dominated set across:
- total elapsed slots (minimize);
- source-side real energy (minimize);
- aircraft battery energy drawn by mission, dwell or loiter (minimize);
- unique installed receiver mass (minimize);
- terminal battery energy (maximize).

No weighted scalar can trade away reserve, capacity or deadline feasibility.

## Input contract

The CLI accepts one exact-header TSV, one row per directed mission edge. Header columns:

1. case_id
2. evidence_class
3. start_node
4. terminal_node
5. deadline_slots
6. capacity_mwh
7. initial_battery_mwh
8. reserve_mwh
9. site_energy_plane_id
10. battery_energy_plane_id
11. edge_id
12. from_node
13. to_node
14. edge_kind (MISSION_LEG or REPLENISHMENT)
15. mode_id
16. duration_slots
17. battery_gain_mwh (accepted energy at the battery-terminal plane)
18. aircraft_draw_mwh (mission propulsion, dwell or loiter energy at the battery-terminal plane)
19. site_energy_mwh (real source energy at the case-level site-energy plane)
20. asset_id (NONE or installed aircraft asset identifier)
21. installed_mass_g (unique mass penalty for that asset)
22. available (true/false)

Every row repeats the same case metadata. All edges are in one finite directed acyclic graph. Duplicate edge IDs, cycles, inconsistent asset masses, invalid edge accounting, inconsistent case metadata, unsupported evidence class and malformed rows are rejected.

A MISSION_LEG has zero battery gain, zero site energy and no installed replenishment asset. A REPLENISHMENT must have positive battery gain and positive source energy. This prevents a battery-exchange edge from being recorded as free energy: the upstream owner must account for the pack's embodied/recharge energy at the common source plane.

For each path, an asset ID's mass is counted once even when that same receiver is used at multiple opportunities. If a strategy uses two physically distinct installed assets, both masses count. The v0 source energy plane is common across every mode in a case; upstream evidence must normalize source energy to that common boundary before admission.

All fixture values are synthetic known-truth numbers. No source-reported hardware efficiency, vendor specification or physical measurement is embedded as a performance claim.

## Frozen synthetic options

The fixture has one mission start, two replenishment decision points and one terminal node. It includes:
- battery-only continuation;
- conductive contact;
- robotic contact;
- battery exchange with pack-energy accounting;
- optical-beam loiter as a synthetic airborne alternative;
- IPT and CPT dock choices;
- a repeated optical receiver asset to test one-time mass accounting;
- a final mission leg.

All options share the same initial battery, reserve, capacity, source meter plane and terminal mission demand.

## Required controls

- exact exhaustive path enumeration on a small DAG;
- intermediate reserve and capacity checks;
- deadline and terminal-reachability checks;
- unavailable edge rejection;
- cycle and duplicate-edge rejection;
- conflicting asset mass rejection;
- source-energy plane and battery-terminal boundary preservation;
- one-time receiver-mass accounting;
- deterministic row-order replay;
- explicit NO_FEASIBLE_STRATEGY.

## Run

```sh
cargo test --locked --manifest-path tools/aero-replenishment-strategy/Cargo.toml
cargo run --locked --manifest-path tools/aero-replenishment-strategy/Cargo.toml -- research/aero-replenishment-strategy-v0/synthetic-mission-edges.tsv
```

## Claim ceiling

A pass establishes exact finite graph arithmetic and Pareto filtering only. It does not validate actual battery electrochemistry, aircraft propulsion, mission reserve policy, charger physics, contact/battery-swap reliability, airborne beaming, exposure/EMC, site power or operational permission. Physical architecture comparison requires qualified upstream mission, metrology, hardware and safety receipts.