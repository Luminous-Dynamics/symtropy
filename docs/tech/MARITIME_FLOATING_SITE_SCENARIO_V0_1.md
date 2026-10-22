# Maritime / Floating-Site Synthetic Scenario v0.1

**Status:** specification-only seed; not an executable or qualified simulator scenario.  
**Parent issue:** [MARITIME-SIM-001 #1612](https://github.com/Luminous-Dynamics/symtropy/issues/1612)  
**Related engineering contract:** [ENG-SYNTH #1172](https://github.com/Luminous-Dynamics/symtropy/issues/1172), [ENG-SYNTH-CRED #1173](https://github.com/Luminous-Dynamics/symtropy/issues/1173), [ENG-SYNTH-TWIN #1467](https://github.com/Luminous-Dynamics/symtropy/issues/1467)
**Independent oracle contract:** [MARITIME_FLOATING_SITE_ORACLE_V0_1.md](./MARITIME_FLOATING_SITE_ORACLE_V0_1.md)
**Independent Rust verifier:** [`maritime-fixture-verifier`](../../crates/tools/maritime-fixture-verifier/README.md), which validates this frozen data contract only.

## Purpose

Freeze one small, entirely synthetic shared-site case before implementation. It is intentionally limited to unit-bearing envelope checks, transparent energy arithmetic, abstract service availability, and categorical evidence/authority states.

It is **not** a ship design, port design, nuclear plant model, security plan, naval-architecture model, grid simulator, evacuation simulation, CFD, FEM, or authorization to operate physical equipment.

## System boundary

The scenario contains:
- one synthetic cargo-vessel envelope;
- one synthetic berth envelope;
- one shore-power / battery resource;
- one abstract floating-site essential-load group;
- a small set of disturbances and configuration/evidence-state substitutions.

Symtropy is intended to own deterministic synthetic execution, event ordering, replay and run receipts. Symthaea owns engineering intent and model applicability/verification interpretation. Luminous Platform can display source-bound operational/incident evidence and coordinate authorized workflows. Mycelix can coordinate scoped reservations, maintenance and provenance records. These layers do not become physical safety controllers, nuclear operators, regulators or sources of physical truth.

## Frozen illustrative assumptions

All numbers in the companion JSON are invented solely for arithmetic/contract testing. They are not sourced from a vessel, port, utility, floating city, or nuclear supplier.

### Envelope check

The abstract vessel is 80 m long, 12 m wide, and has 2.5 m declared draft. The abstract berth envelope is 100 m long, 18 m wide, with a 3.0 m declared draft limit. The first check is only component-wise envelope comparison.

A pass means only that the explicitly listed dimensions fit this synthetic rectangular envelope. It says nothing about under-keel clearance, tides, maneuvering, mooring, fenders, hydrodynamics, structure, load cases, local rules, or real berth compatibility.

### Transparent battery arithmetic

- Required route energy: 4.0 MWh.
- Declared protected reserve: 1.5 MWh.
- Minimum stored energy at departure: 5.5 MWh.
- Baseline available departure energy: 6.0 MWh, so the simple arithmetic check has 0.5 MWh headroom.

In a separate charging-shortfall fixture, starting stored energy is 3.0 MWh; charging is limited to 2.0 MW for one hour at a declared 0.9 efficiency. Added stored energy is:

`2.0 MW × 1.0 h × 0.9 = 1.8 MWh`

The resulting 4.8 MWh is below the declared 5.5 MWh departure threshold, so the expected result is **infeasible for this simplified energy contract**. This does not predict real vessel energy demand or battery performance.

### Abstract essential-load check

The floating-site fixture declares a synthetic essential load of 1.2 MW and an independent backup supply of 0.5 MW during a modeled supply interruption. The instantaneous 0.7 MW deficit must be reported; this small fixture does not model energy duration, transient response, power quality, islanding protection, or real critical-load engineering.

## Required cases

The companion machine-readable file includes:
1. baseline values fit the declared envelope and pass the simple departure-energy arithmetic;
2. vessel exceeds the berth length limit and must be rejected;
3. charging interruption leaves departure energy below the reserve-inclusive minimum;
4. stale configuration/evidence must return indeterminate rather than inherit the baseline result;
5. a request for nuclear thermal-hydraulic or other unsupported physics must return unsupported, not be approximated by rigid-body mechanics;
6. floating-site supply interruption reveals the stated abstract essential-load deficit.

## Independent data verifier

A standalone Rust verifier now checks the frozen v0.1.0 artifact, independently derives the six expected dispositions, rejects duplicate JSON object keys and schema drift, and emits a report bound to the exact raw-input SHA-256 digest and supplied source commit. It has hostile/reference unit tests and a dedicated CI workflow.

This verifier does not instantiate Symtropy's physics engine. A verifier success means contract/arithmetic consistency for this synthetic document only; it does not admit `EngineeringDeterministic`, qualify a physics model, or provide physical evidence.

## Execution and provenance gate

Before connecting this fixture to production scenario code:
- re-read the exact current ENG-SYNTH / credibility / twin parent issues and inspect the exact engine capability census;
- freeze schema, units, canonical encoding, output dispositions and independent oracle before using the fixture for claim-bearing execution;
- bind any eventual run to exact source commit, engine/model profile, scenario digest, configuration identity, time-step/seed policy, declared outputs, anomalies, unsupported semantics and replay evidence;
- use an independent verifier that does not import the production calculation/reducer;
- verify that malformed/non-finite inputs, unknown units, stale generation and unsupported model paths fail closed;
- obtain qualified external model/reference data before making physical claims.

The required execution profile is `EngineeringDeterministic`, but this file does not establish that profile is currently admitted or qualified for this scenario. If the exact runtime cannot satisfy the profile contract, keep the case specification-only and record the blocker.

## Claim ceiling

A future test pass can establish only that the frozen, declared arithmetic/contracts behave as expected on the exact inputs. It cannot establish seaworthiness, dock safety, grid stability, nuclear safety/security, cyber resilience, emergency egress, commercial performance, regulatory approval, or physical-operation authority.

## References

- [Symtropy engineering scenario/replay contract #1172](https://github.com/Luminous-Dynamics/symtropy/issues/1172)
- [Model suitability and numerical-integrity gate #1173](https://github.com/Luminous-Dynamics/symtropy/issues/1173)
- [Digital-twin rehearsal boundary #1467](https://github.com/Luminous-Dynamics/symtropy/issues/1467)
- [MARITIME-SIM-001 #1612](https://github.com/Luminous-Dynamics/symtropy/issues/1612)
- [Symthaea MARITIME-SEC-001 #7446](https://github.com/Luminous-Dynamics/symthaea/issues/7446)
