# Multimodal Mission Chain and Handoff Continuity Benchmark v0.1

**Status:** synthetic-only mission-chain feasibility auditor. It is not a live route planner, carrier recommendation, safety case, medical protocol, permit, route clearance, dispatch controller or deployment authorization.

## Paradigm shift: optimize the end-to-end service, not one vehicle

A mission can fail even when every carrier looks capable in isolation. A cold-chain load can arrive late or warm during transfer. A wheelchair user can board the first platform but have no accessible transfer at the next hub. Individually feasible legs can miss the delivery window when handoff time is counted. An unverified route cannot become acceptable simply because the selected rover passes a capacity check.

The correct unit of evaluation is an ordered chain: **payload requirements → first-mile mode → hub/transfer → next leg → further handoffs → destination and recovery.** At each interface, the benchmark must preserve custody, securement, condition control, transfer responsibility, accessibility/caregiver support, and schedule feasibility.

The best chain may combine an existing boat, specialist carrier, local vehicle, sled or assisted human transfer. A mission may be better served by storage or a fixed sensor at a hub than by sending a robot everywhere. The benchmark should be able to conclude that no available chain is admissible.

## What v0.1 models

- Five synthetic environments: Amazon Basin, Congo Basin, Antarctica, Sahara and Afghanistan.
- Eight payload profiles: bulk supplies, cold-chain medicine, fragile instruments, contained samples, seated passengers, wheelchair passenger, stretcher patient and field crew.
- Fourteen platform profiles and twenty-one transfer/storage hubs.
- Seven mission scenarios and eight chain assessments, including deliberate controls for interrupted condition control, worst-case deadline overrun and unknown Afghanistan route/hazard evidence.

Every leg declares its platform, origin/destination hubs, minimum/maximum travel duration, route authorization, environmental protection, recovery and hazard-evidence statuses. Every handoff declares the adjacent leg pair, hub, minimum/maximum duration, authorization, environmental protection, securement, condition-control continuity, accessibility, patient support, privacy/custody, operator staffing, recovery and hazard-evidence statuses.

The route chain must be continuous: first origin and final destination match the mission; adjacent legs meet at the same hub; each handoff links the correct adjacent legs at that hub. N legs require exactly N−1 handoffs.

## Cargo and human requirements

### Cargo

Each payload has a class, maximum mass, optional allowed temperature interval, securement requirement and privacy sensitivity. Every leg must support the payload classes and combined maximum mass. A platform's controlled-temperature interval must lie inside each relevant payload's allowed interval. Hubs used for staging or transfer must accept the payload class and mass; temperature-sensitive cargo also needs a compatible declared storage zone and condition-control capability. Missing condition evidence is never treated as a pass.

Fragile instruments will need real shock/vibration limits in a later version. V0.1 models class and condition continuity but does not simulate shock isolation, load geometry, center-of-mass changes or thermal transients. The single-leg payload contract in [PR #1879](https://github.com/Luminous-Dynamics/symtropy/pull/1879) is the companion screen for mass, package geometry and static center-of-mass bounds.

### Human passengers and patients

People are not ordinary cargo. Each leg must provide occupant restraints and emergency egress, plus the required accessible transfer, wheelchair securement, patient support and caregiver access. Hubs and handoffs must preserve relevant accessibility/care support. Privacy-sensitive loads require custody/privacy controls. This audit does not determine clinical suitability, specify medical care or qualify a patient compartment. Any eligible result requires qualified engineering/human-factors and clinical review, route clearance, local approval and instrumented verification before a real trial.

NIST ambulance-compartment research highlights trade-offs between keeping providers safely restrained and preserving access to patients/equipment; modeling and simulation have been used to evaluate patient-compartment design. This is a human-factors precedent, not direct qualification for off-road or robotic patient transport. See [NIST ambulance compartment design](https://www.nist.gov/publications/modeling-and-simulation-improving-ambulance-patient-compartment-design-standards) and [human factors in ambulance/EMS equipment](https://pubmed.ncbi.nlm.nih.gov/30638417/).

## Feasibility rules

- `blocked_hard_gate`: a required route, environmental, recovery, handoff authorization, securement, operator or hazard status fails.
- `blocked_chain_mismatch`: platform/hub/payload incompatibility, a condition/accessibility/capacity problem, broken chain continuity, or failure to guarantee the deadline from upper duration bounds.
- `needs_gate_evidence`: no known failed gate or mismatch, but required route, hazard, authorization, recovery or applicable handoff evidence is unknown.
- `eligible_for_human_review`: all declared leg/handoff gates pass, platform/hub compatibility passes, topology is continuous, and the sum of worst-case leg plus handoff durations fits the deadline.

Status precedence is fail-closed. A delivery deadline is guaranteed only when the sum of all maximum leg and handoff duration bounds fits the deadline; averages do not substitute for bounded worst-case timing.

Afghanistan examples deliberately leave route authorization and explosive-hazard status unknown. They remain `needs_gate_evidence`; this tool does not route into or approach suspected explosive hazards and never authorizes autonomous hazard handling.

## Synthetic baseline

The fixture contains eight assessments across seven missions. The independent structural/reference calculation gives four eligible for human review, one blocked by a failed condition-control handoff, one blocked because worst-case travel and transfer duration exceeds the deadline, and two awaiting Afghanistan route/hazard evidence. In the deadline negative control, good individual legs cannot compensate for an infeasible whole-chain upper bound.

These are synthetic regression outcomes. Durations, capacities, platform/hub capabilities and statuses are invented to exercise the rules; they are not real route times, service availability, product specifications or safety evidence.

## Benchmark targets

- **Chain integrity:** 100% agreement on ordered route endpoints; exactly one handoff between each adjacent leg; zero broken chains admitted.
- **Custody continuity:** every declared payload supported by every leg/hub and accounted for across every handoff.
- **Condition continuity:** zero temperature-constrained loads admitted with missing, failed or unknown relevant condition-control evidence.
- **Passenger continuity:** zero wheelchair/patient/crew chains admitted if any required platform, handoff or hub lacks accessibility, restraint/egress, patient/caregiver support or human operators.
- **Deadline integrity:** zero chain marked eligible when the sum of upper duration bounds exceeds the deadline.
- **Hazard integrity:** zero route with unknown/failed authorization or hazard evidence marked eligible.
- **Oracle accuracy:** 100% agreement with the independent reference disposition for the fixture and all 3^4 = 81 combinations of the four leg gate statuses.
- **Reproducibility:** byte-identical output after reordering independent profile/assessment collections; leg and handoff arrays remain ordered because sequence is semantically meaningful.
- **No false authority:** no weighted universal winner, route authorization, clinical authorization or autonomous hazard-handling permission is emitted.

## Limits and next phase

V0.1 is a compatibility auditor, not an optimizer. It does not calculate globally optimal routes, predict traffic/weather, estimate fuel/emissions/cost, optimize inventory/loading, estimate risk probabilities or choose among multiple route graphs. It also does not model temperature excursions during transfer beyond declared hub zones and condition-control status.

Next: connect the chain contract to the payload-first checks from PR #1879, add versioned module/custody manifests, use sensor receipts (e.g. temperature logger evidence) across handoffs, and add route-graph search for feasible Pareto sets across time, cost, energy, ecological disturbance and recovery—using real evidence rather than invented probabilities.

## Research anchors

- NIST, [Modeling and Simulation for Improving Ambulance Patient Compartment Design Standards](https://www.nist.gov/publications/modeling-and-simulation-improving-ambulance-patient-compartment-design-standards).
- PubMed, [Human Factors and Ergonomics in Ambulances and EMS Equipment](https://pubmed.ncbi.nlm.nih.gov/30638417/).
- ISO, [ISO 668:2020 Series 1 freight containers](https://www.iso.org/standard/76912.html): a scoped intercontinental freight interface, not a universal small-robot module standard.
- World Bank, [transport and cold-chain logistics in the Amazon](https://blogs.worldbank.org/en/latinamerica/how-better-transport-can-unlock-jobs-incomes-amazon), 24 April 2026.
