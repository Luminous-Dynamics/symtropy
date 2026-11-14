---
title: Space Capability Ledger v0.1
status: research-ledger / coverage inventory
date: 2026-10-18
source_of_truth: docs/tech/space-capability-ledger-v0.1.json
claim_boundary: historical references establish that a capability was demonstrated somewhere; they do not establish that Symtropy reproduces the mission or that a new design is qualified
---

# Space Capability Ledger v0.1

## Purpose

Turn major achieved spaceflight capabilities into a traceable engineering program. The purpose is not to retell space history or claim a digital twin can do everything a space agency has done. Each capability becomes a bounded acceptance contract with an authoritative reference, an independent model/test target, required input conventions and an explicit claim ceiling.

The machine-readable corpus lives in `docs/tech/space-capability-ledger-v0.1.json`. Its initial 14 capability entries span trajectory/reentry, crewed lunar activity, rendezvous and docking, reusable transport, habitation/ECLSS, orbital servicing, deep-space operations, planetary rovers and rotorcraft, small-body sample return, kinetic-impact planetary defense, lunar ISRU, autonomous construction and integrated Moon Base architecture.

## Evidence states must not be collapsed

The ledger records separate evidence dimensions:
- **Historical reference state:** a named mission or system demonstrated some version of the capability.
- **Symtropy implementation state:** an inventory-level statement about substrate or benchmark coverage in this repository.
- **External data replay state:** whether versioned external numerical data have actually been replayed against the model.
- **Hardware/flight qualification state:** whether the proposed implementation has the evidence and authority required for physical operation.

These states are intentionally non-transitive. A successful mission is not a software test; a source document is not a reproduced trajectory; a synthetic test is not a physical demonstration; and a digital-twin pass is not flight qualification.

The first ledger is a **starter inventory, not a claim of completeness**. Its purpose is to expose each next gap and provide a reviewable denominator. Add new capabilities by references and requirements, not by silently expanding existing rows. The source list is a source registry, not a substitute for reading and pinning the exact mission datasets required by a numerical reproduction.

## How each reference becomes engineering work

For every selected capability:

1. Freeze the mission or technology reference, the source document/version, units, coordinate frames, time scales, assumptions and the exact data required.
2. Choose a narrow acceptance case with independently known outputs or bounds. When historical numerical data are not public or complete enough, label the first case `synthetic_benchmark` and preserve the distinction.
3. Implement a minimal deterministic Symtropy model without embedding mission-specific assumptions in a universal physics kernel.
4. Build an independently written reference evaluator and counterexamples. The system under test cannot serve as its own only oracle.
5. Record input/model/test digests, exact source commit, compiler/toolchain, command, exit status and receipt. Missing/queued/skipped CI is not a pass.
6. Only after numerical replay validates should the case become a model fidelity result. Hardware-in-the-loop, environment testing and mission qualification remain separate gates.

## Priority order for the lunar program

### P0 — Independent physics and deterministic replay
Trajectory frames, timing, units, conservation, rigid-body/contact behavior, power/energy and material accounting. Every important quantity must have a testable definition and an independent balance or reference oracle.

### P1 — Uncrewed lunar pathfinder
Sol-Atlas supplies the spatial/site-evidence projection; Symtropy simulates terrain, surface power, robotic mobility and resource-process envelopes; Symthaea provides bounded planning/diagnostics behind deterministic local safety controllers; Mycelix binds material lots, work receipts, models and provenance. Unknown site data remain unknown, not defaulted into favorable values.

### P2 — Persistent robotic utility service
Demonstrate power and communication uptime, thermal/dust constraints, maintenance, recovery after faults, local resource characterization and a verified product chain. The new [LUNAR-SEED-UTILITY-001 #1970](https://github.com/Luminous-Dynamics/symtropy/issues/1970) and [draft PR #1971](https://github.com/Luminous-Dynamics/symtropy/pull/1971) start at the narrow energy-dispatch/mass-accounting layer. Its input and process values are synthetic.

### P3 — Integrated infrastructure
Connect logistics, redundancy, repairability, habitat/environmental control and qualified construction material. Each utility and capability is independently checked before system-level scenarios may consume it as a qualified input.

### P4 — Human-rated settlement / beyond the Moon
Crewed operations only follow appropriate system-specific validation, hazard analysis, physical testing, qualification and independent safety review. Mars and later environments require new environment models and evidence; they are not obtained by switching a planet-name field.

This staged structure follows the NASA Moon to Mars Architecture's distinct sub-architectures—including autonomy/robotics, communications/PNT, habitation, ISRU, logistics, mobility, power, transportation and infrastructure—and its phased Moon Base framing. It does not imply NASA has qualified this repository's software.

## Benchmarks to demand

There should be no universal “space-readiness score.” Incomparable properties must not be averaged into a single number. At minimum, individual acceptance cases should report:

| Domain | Primary metric | Mandatory negative controls |
|---|---|---|
| Dynamics / guidance | Position, velocity, attitude and event-time error against an independent reference; conservation residuals | Wrong frame, time epoch, units, impulse sign, missing thrust or invalid ephemeris |
| Rendezvous / contact | Relative-state error, collision/keep-out violations, peak contact loads, abort reachability | Sensor dropout, latency, inconsistent state estimates, actuator limits |
| Power / thermal | Service delivered at the required time/temperature, energy/heat residual, reserve margin and recovery time | Dark interval, unexpected generation loss, failed storage element, impossible temperature/state |
| Habitation / ECLSS | Species/material balances, service floor, consumables, quality and outage exposure | Sensor drift, leaking interface, blocked process, stale filter/catalyst and failed recovery |
| Robotics / construction | Accepted work per unit energy, localization error, task completion, wear and recoverability | Dust/terrain perturbation, wheel slip, jammed tool, degraded actuator, incomplete inspection |
| ISRU / industry | Feed-to-product mass balance, energy per accepted product, yield/grade/contaminants and spares dependency | Unknown feedstock, zero recovery, double-counting coproducts, out-of-spec product, missing dependency |
| Logistics / provenance | Delivered accepted cargo, end-to-end mass/identity reconciliation, delay tolerance and chain completeness | Lost relay, duplicate delivery, conflicting ownership, corrupt receipt, stale model/data |
| Fault management | Detection delay, isolation accuracy, false alarms, uncertainty/abstention, invariant violations | Unknown fault, confounder, multi-fault case, stale telemetry, common-mode sensor loss |

Every metric needs a frozen denominator, the operating envelope, and a documented reference oracle. Report per-case results and distributions only when the number and variety of evaluated cases justify them. A mean score cannot hide a critical safety-gate failure.

## Existing work to compose

The ledger deliberately reuses the current Symtropy ownership map:
- [#1542 Lunar Elevator Transport Network](https://github.com/Luminous-Dynamics/symthaea/issues/1542) and [#1736 Space Economy Benchmark](https://github.com/Luminous-Dynamics/symtropy/pull/1736) retain cislunar transport/economic alternatives.
- [#1604 Planetary Industrial Ecology](https://github.com/Luminous-Dynamics/symthaea/issues/1604) owns industrial closure and the equipment/resource dependency graph.
- [#1618 Crash-Survivor Settlement Bootstrap](https://github.com/Luminous-Dynamics/symtropy/issues/1618) owns settlement/capability bootstrapping boundaries.
- [#1803 Resource-to-Product Chains](https://github.com/Luminous-Dynamics/symtropy/issues/1803) owns evidence-bounded material/resource-to-product paths.
- [#1840 Solar-Thermal Exergy-to-Service](https://github.com/Luminous-Dynamics/symtropy/issues/1840) owns its separate hand-calculable energy/thermal model.
- [#1970 Seed-Base Utility](https://github.com/Luminous-Dynamics/symtropy/issues/1970) starts the first integrated synthetic utility-control slice.

Symthaea owns bounded autonomy, planning and fault diagnosis—not physical actuator authority. Mycelix should own provenance/custody/coordination records and capability/governance semantics. Sol-Atlas should own spatial, terrain, map and site/resource evidence projections. Symtropy owns deterministic physics and engineering campaigns. Bridges must bind exact revisions and keep authority boundaries explicit.

## Known gaps in this v0.1 ledger

- It does not contain mission telemetry, ephemerides, command logs, measured component maps, official test fixtures, licensed numerical datasets or verified reference-output digests.
- The corpus is not an exhaustive list of all space achievements. Its 14 entries seed an auditable denominator for review.
- The current Symtropy status fields are preliminary source/issue inventory, not a completed code audit or test result.
- Lunar surface water abundance/accessibility, extraction yield, habitat closure, dust/radiation lifetime, plant sizing, safety, economy and flight qualification remain unproved here.
- No historical mission is said to be numerically reproduced until a pinned independent reference case has actually been executed.

## Research anchors

- NASA [Moon to Mars Architecture Components](https://www.nasa.gov/moontomarsarchitecture-components/): capability/sub-architecture map.
- NASA [Moon Base Systems](https://www.nasa.gov/moonbase-systems/): phased lunar-base reference architecture.
- NASA [Lunar Surface Technology](https://www.nasa.gov/lunar-surface-technology/): power, ISRU, construction, robotics, dust and environmental technology boundaries.
- NASA [History of Lunar Exploration](https://science.nasa.gov/solar-system/moon/history-of-lunar-exploration/): lunar mission history.
- NASA [Environmental Control and Life Support Systems](https://www.nasa.gov/reference/environmental-control-and-life-support-systems-eclss/): subsystem decomposition and resource flows.
- NASA [Hubble servicing history](https://science.nasa.gov/mission/hubble/overview/the-history-of-hubble/): maintainability, replacement and upgrade evidence.
- NASA [Ingenuity](https://science.nasa.gov/mission/mars-2020-perseverance/ingenuity-mars-helicopter/), [OSIRIS-REx](https://science.nasa.gov/mission/osiris-rex/) and [DART](https://science.nasa.gov/planetary-defense-dart/): representative technology/missions for atmospheric flight, sample return, and kinetic impact.

The external references establish what NASA documented, not the completeness of this corpus nor any Symtropy validation result.
