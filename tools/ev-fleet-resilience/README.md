# EV Fleet Charging Resilience Screen v0.1

**Programme:** EV-CHARGE-RESILIENCE-001 (#2137)  
**Implementation:** standalone Rust reference evaluator; no third-party runtime beyond Serde/JSON  
**Input:** `fixtures/ev_fleet_resilience_v0_1.json`  
**Schema:** `schemas/ev_fleet_resilience_v0_1.schema.json`

## Why this is a paradigm shift

A vehicle range estimate, charger nameplate, or average site load cannot tell a fleet manager which work will be missed when vehicle charge acceptance tapers, arrivals overlap, departure deadlines differ, grid capacity is curtailed, and ports become unavailable at the same time.

This screen evaluates **service delivered per declared disturbance**. It runs FCFS and earliest-departure-first against the same jobs, vehicle templates, site power, and contingency schedule. It reports missed deadlines and unserved battery energy rather than turning distinct trade-offs into one opaque score.

The initial implementation makes a deliberate, reviewable compromise: it is deterministic and scenario-based, not a Monte Carlo reliability estimator or an optimizer. Outage windows and reduced site/port capacities are explicit input events. Time advances to arrivals, departures, charge completion, SOC-acceptance band boundaries, and capacity-event edges; it does not rely on a coarse fixed timestep.

## Model

For each vehicle template:

- Effective pack energy capacity is nominal capacity × state-of-health factor × static temperature-capacity factor.
- Required battery-side energy is effective capacity × (target SOC − initial SOC).
- Instantaneous battery acceptance is the active SOC band power × the static temperature-acceptance factor.
- Charger input is bounded by both the individual port-input rating and the battery acceptance converted across the declared charging-efficiency boundary.
- Active sessions share the site input budget by max-min water filling, each bounded by its instantaneous cap.

For each scenario, site import budget and available-port count can change over non-overlapping, half-open intervals `[start_minute, end_minute)`. At a capacity reduction, sessions with lowest current policy priority are suspended if necessary. They can resume when capacity returns. Sessions are otherwise non-preemptive: arrivals do not interrupt an already-active job. On a newly free port, FCFS sorts by arrival then stable job ID; earliest-departure-first sorts by departure, arrival, then stable job ID.

A job completing exactly at its departure boundary counts as completed. A job missing its deadline remains in the denominator, and its undelivered requested battery energy is reported. Zero-energy jobs are retained and complete at arrival without using a port. The report is sorted by scenario, policy, and job identity to make repeat runs stable.

## First adversarial fixtures

There are seven scenario inputs and fourteen policy results (seven scenarios × FCFS/earliest-departure-first):

1. **Depot pair:** `depot-grid-baseline` and `depot-grid-curtailment` use the same four vehicle jobs, installed ports and base site rating; only the latter includes a 30-minute site-power curtailment.
2. **Rural pair:** `rural-feeder-baseline` and `rural-feeder-interruption` share the same three jobs; only the latter includes a feeder outage and reduced-power/port recovery.
3. **Port/taper pair:** `port-stable-baseline` and `port-loss-and-taper` share the same three jobs and installed equipment; only the latter includes the temporary port and site-capacity derate.
4. **Deadline policy oracle:** a 10 kWh job due at minute 120 and a 3 kWh job due at minute 20 compete for one 10 kW port. This fixture is constructed so FCFS misses the urgent job while earliest-departure-first completes both; it tests the implementation, not the universal superiority of EDF.

Use each baseline/stress pair to calculate service degradation under the *same duty demand*, by policy: changes in completion count, unserved battery kWh, p95 wait and site input. The committed evaluator reports both sides as separate, deterministic rows rather than claiming one universal resilience score. All fixture numbers and battery curves are synthetic.

## Acceptance/benchmark targets

| Invariant | Target |
|---|---:|
| Per-event site input | 0 exceedances of the active declared cap |
| Per-job input | 0 exceedances of port or instantaneous acceptance cap |
| Energy accounting | `site input = battery energy + conversion loss` within (10^{-6}) kWh |
| Deadline boundary | Completion at deadline is consistently accepted |
| Denominator integrity | Every job appears exactly once under both policies |
| Determinism | Identical inputs and permutations yield identical canonical job/fleet results |
| Input integrity | Duplicate IDs, unknown template references, invalid SOCs, curve gaps/overlaps, and overlapping capacity events fail closed |
| Evidence | Every result retains its evidence class; synthetic is never promoted to measured |
| Comparison | FCFS and earliest-departure-first share identical physical inputs; report outcomes without a universal-winner claim |

These are **software-contract targets**, not physical fleet targets. When measured traces exist, add time-aligned baseline datasets and compare per-job delivered kWh, wait/deadline distributions, peak grid import, sensitivity to outage duration, and error against an independent simulator or replay oracle.

## Limitations / claim ceiling

- Piecewise-constant charging efficiency and acceptance within an SOC band.
- Static SOH and temperature derating; no electrochemical state, thermal transients, charge-power history, or degradation cost.
- Site capacity events are stipulated inputs, not derived from feeder AC load-flow or protection coordination.
- A capacity event supplies an available-port count and a site-input cap; it does not model which physical charger fails, switching transients, communications loss, electrical faults, or common-mode failures.
- No probability distribution is assigned to outages, arrivals, weather, or vehicle use; output is not a probability of service availability.
- No stationary battery, PV forecast, tariff or V2G dispatch is optimized here.
- This is not a vehicle/charger safety certification, manufacturer curve, real range estimate, field result, grid-connection recommendation, or authorization for deployment.

## Research anchors

- IEA, *Global EV Outlook 2026 — Electric vehicle charging*: https://www.iea.org/reports/global-ev-outlook-2026/electric-vehicle-charging-chap-6-and-10
- IEA, *Vehicle-to-grid technology* (20 May 2026): https://www.iea.org/reports/vehicle-to-grid-technology
- Mogire, Kilbourn & Luke (2026), *Resilient Electric Vehicle Charging Stations in Urban Areas: A Systematic Literature Review*, *World Electric Vehicle Journal* 17(3), 148: https://doi.org/10.3390/wevj17030148
- Powell & Johnson (2024), *Impact of EV Charging Station Reliability, Resilience, and Location on EV Adoption*, NREL/National Laboratory of the Rockies, NREL/TP-5R00-89896: https://doi.org/10.2172/2432350
- Santos, Andersen, Marinelli & Pastorelli (2026), *Linking fleet operations to grid requirements: an early-stage framework for electric truck charging depot planning to minimize grid connection capacity*, 10th E-Mobility Power System Integration Symposium: https://orbit.dtu.dk/en/publications/linking-fleet-operations-to-grid-requirements-an-early-stage-fram/
- South Africa, Department of Trade, Industry and Competition, *Electric Vehicles White Paper* (2023): https://www.thedtic.gov.za/wp-content/uploads/EV-White-Paper.pdf
