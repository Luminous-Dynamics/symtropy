# AER-01 Mission Chronology and Phase-State Contract v0.1

**Parent:** [AERO-MISSION-CHRONOLOGY-001 #4029](https://github.com/Luminous-Dynamics/symtropy/issues/4029)  
**AER-01:** #3921  
**Whole-aircraft closure:** #3945  
**Mass evidence/loading state:** #4000  
**Takeoff-state cruise adapter:** #4025  
**Status:** synthetic contract/oracle pilot; no phase-complete aircraft model.

## 1. Scope

This increment defines an initial machine-readable MissionTimeline, AircraftPhaseState and PhaseEnergyBalance contract, plus a small independent oracle. It does not add a production flight trajectory or aircraft-performance model.

The critical separation is:

- a state snapshot is a configuration- and source-bound set of installed masses, geometry locations and computed mass properties;
- a phase transition is an event that consumes a prior state and emits a new state;
- an energy/power balance reports source, bus, conversion, shaft and propulsive planes separately;
- a mission closure result cannot be feasible when required phases are not assessed.

The existing evaluator remains a steady-level cruise screen. This pilot therefore marks taxi, takeoff, climb, descent and landing as NOT_ASSESSED and declares the whole synthetic mission INDETERMINATE.

## 2. Contracts

- schemas/mission-timeline.schema.json — ordered phase sequence, start/end state identities, reserve policy, primary metric and evidence ceiling.
- schemas/aircraft-phase-state.schema.json — itemized mass records, exact configuration/source identities, predecessor digest, state digest, reference frame and expected mass properties.
- schemas/phase-energy-balance.schema.json — bus, motor, shaft, energy-store, peak power and continuous power-limit fields.

All values are synthetic SI test data. Each phase-state SHA-256 is recomputed from the canonical state payload: remove only state_sha256; serialize UTF-8 JSON with lexicographically sorted object keys, compact separators, preserved array order and no ASCII escaping; hash with SHA-256. previous_state_sha256 is included in the digest. This binds each digest to the supplied state content, but does not authenticate real aircraft source bytes.

## 3. Known-answer fixture

data/mission/mission_chronology_fixture_v0_1.json includes four synthetic states:

1. Dispatch: 720 kg, 100 kg fuel.
2. Takeoff: same loading; new state identity.
3. Cruise end: 25 kg fuel consumed, 695 kg total.
4. Landing: a further 5 kg fuel consumed, 690 kg total.

Fuel is located away from the aircraft CG so consuming it changes mass, CG and inertia. The independent oracle recomputes all three from the raw item list for every state.

The synthetic cruise phase also provides distinct energy planes:

- Bus input: 100 MJ.
- Motor bus input: 80 MJ.
- Auxiliary bus load: 5 MJ.
- Thermal bus load: 5 MJ.
- Other conversion loss: 10 MJ.
- Motor shaft output: 72 MJ after 8 MJ motor conversion loss.
- Propulsive useful energy: 60 MJ after 12 MJ propulsor loss.
- Remaining energy after the assessed cruise segment: 150 MJ.
- Declared reserve requirement: 50 MJ.

Bus, motor, shaft and storage arithmetic are independently replayed. These values are not mapped to a real engine, propeller, motor, battery or aircraft.

## 4. Independent oracle

scripts/verify-mission-chronology.py uses only the Python standard library. It independently:
- recomputes and verifies the canonical SHA-256 for each state payload;
- computes total mass, CG and inertia for each state;
- verifies ordered predecessor/state identity and phase continuity;
- checks that only the explicitly declared fuel item changes across the two burn transitions;
- checks bus, motor and shaft conservation separately;
- checks energy-store transition, reserve margin and peak-vs-continuous source power;
- refuses a feasible mission label while any required phase remains NOT_ASSESSED;
- runs hostile negative controls for stale predecessor, modified state content with unchanged digest, bus imbalance, over-limit peak power, fuel mass gain with a rebuilt digest chain, reserve shortfall, false-feasible mission status and duplicate mass items.

The dedicated workflow parses the three JSON schemas and fixture, then runs the independent oracle. JSON parsing is not a substitute for a full JSON Schema validator; the field-level semantics are independently checked by the fixture oracle and require further extension before a general external schema-validation claim.

## 5. What is deliberately not modelled

- Takeoff ground roll and obstacle-clear distance.
- Climb trajectory, climb gradient or obstacle clearance.
- Stall, spin, handling qualities, stability or flight envelope.
- Descent/approach/landing performance.
- Time-resolved aerodynamics or changing aircraft configuration beyond the simple fuel-mass transition.
- Hybrid generator start/ramp and dispatch; use #3900.
- Installed battery/capacitor limits and ageing; use #3903.
- Thermal dynamics and cooling hardware sizing.
- Structural loads, aeroelasticity, flutter or control-system failure behaviour.

These phases are NOT_ASSESSED, not assumed safe or successful.

## 6. Next sequence

1. Extend the state transition contract to typed fuel-tank, payload and module events with explicit evidence sources.
2. Compose #3900 and #3903 through exact mission/phase/configuration digests.
3. Build and independently qualify separate low-order ground/takeoff, climb, descent/approach/landing and go-around/diversion models.
4. Integrate those models into a chronological mission evaluator with explicit reserve branches and per-phase applicability.
5. Only then evaluate full mission feasibility on source-frozen Wren 2 and Wayfarer 4 cards.

## Claim ceiling

The current pilot verifies synthetic arithmetic and state/phase contracts only. It does not establish real aircraft mass properties, energy performance, reserve adequacy, takeoff/climb/landing capability, aircraft safety, airworthiness, certification or permission to fly.
