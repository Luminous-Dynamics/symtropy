# Fleet Charging Queue Screen v0.1

Programme: [EV-CHARGE-QUEUE-001 #2115](https://github.com/Luminous-Dynamics/symtropy/issues/2115). The standalone Rust executable compares first-come-first-served (FCFS) and earliest-departure-deadline-first (EDF) on exactly the same jobs and infrastructure.

## Why this model matters

A single vehicle's charge time does not determine whether a fleet can meet its work schedule. Multiple vehicles can return at the same time, a finite number of charging ports may be occupied, vehicles have different maximum charge acceptance, and the site has a shared input-power cap. The IEA's 2026 charging outlook highlights charging/site-capacity constraints; research on fleet battery swapping also studies queue synchronization and scheduling as bottlenecks. These findings motivate queue-aware planning but do not imply one policy is best in every operation.

Research:
- IEA Global EV Outlook 2026 charging: https://www.iea.org/reports/global-ev-outlook-2026/electric-vehicle-charging-chap-6-and-10
- IEA Vehicle-to-grid technology (managed/bidirectional charging and barriers): https://www.iea.org/reports/vehicle-to-grid-technology
- 2026 cooperative scheduling for multi-fleet battery swapping: https://www.sciencedirect.com/science/article/pii/S2590116826000202
- 2025 mine battery-swapping queue optimization with case-specific outcomes: https://www.frontiersin.org/journals/computer-science/articles/10.3389/fcomp.2025.1729185/full

## Run

From the repository root:

    cargo test --manifest-path tools/rural-industrial-electrification-evaluator/Cargo.toml --all-targets
    cargo run --quiet --manifest-path tools/rural-industrial-electrification-evaluator/Cargo.toml --bin charging_queue_screen -- tools/rural-industrial-electrification-evaluator/fixtures/charging_queue_v0_1.json

The output is deterministic JSON, sorted by scenario, policy and job identity. The fixture contains five synthetic site patterns and 29 charging jobs; each scenario is run twice (FCFS and EDF).

## Energy and charging boundary

The input field `battery_energy_required_kwh` is the energy that must arrive at the battery. `charging_efficiency_fraction` maps site input to battery-side energy. Therefore:

    required_site_input_kwh = required_battery_energy_kwh / efficiency
    max_site_input_power_kw = min(port_power_kw, vehicle_max_acceptance_kw / efficiency)

Port power and site power are both expressed at the **site input boundary**. Vehicle acceptance is battery-side. The simulator uses constant efficiency through a session and no charging taper.

All simultaneously active sessions share the aggregate site cap using deterministic max-min water filling, while respecting each session's port and vehicle acceptance cap. When a low-cap session saturates, the remaining site power can be allocated to sessions with higher caps. It does not intentionally exceed the site input budget.

## Scheduling model

- A charging job becomes available at `arrival_minute`.
- It must have enough energy by `departure_deadline_minute`.
- A port handles one job at a time. Sessions are non-preemptive, so a policy only chooses which waiting job receives a newly available port.
- FCFS orders available jobs by arrival time, then stable job ID.
- EDF orders available jobs by earliest departure deadline, then arrival time, then stable job ID.
- At the departure deadline, incomplete jobs depart. Any still-requested battery energy becomes unserved energy at deadline; expired jobs do not keep consuming charger power after their departure.
- Event time is advanced to the next arrival, deadline or session completion—rather than rounding to a fixed time step. Completion at the exact deadline counts as on-time.

## Outputs

For each job:
- charge start and completion, queue wait and deadline slack;
- requested/delivered battery energy, site input and conversion loss;
- whether charging started and completed by deadline;
- missed deadline and unserved battery energy at departure.

For the fleet:
- completed-by-deadline and missed-deadline counts;
- requested/delivered energy, site input, conversion losses and energy-balance residual;
- median, p95, mean and maximum queue wait;
- peak site input, site-limit check, elapsed horizon and port utilization.

Queue wait for a vehicle that never receives a port is recorded as its entire available wait from arrival until its departure deadline; the `started` flag makes that case explicit. This prevents a never-served job disappearing from wait-time statistics.

## Current synthetic fixture panel

1. Overnight light fleet: eight jobs share two 11 kW ports and a 22 kW site budget.
2. Synchronized bus wave: six high-energy returns share three 150 kW ports and a 240 kW site cap.
3. Policy oracle: a 10 kWh flexible job and a 3 kWh urgent job demonstrate a case where EDF avoids an FCFS missed deadline.
4. Weak-grid rural cohort: heterogeneous charge acceptance under a 25 kW site limit.
5. Mine pack-charge rack: six pack-charge jobs with staggered arrivals and next-use deadlines. Pack handling, swap logistics and pack temperature are intentionally not modeled.

The fixtures are synthetic. Any policy performance differences are outputs for these declared assumptions only, not real fleet measurements or infrastructure recommendations.

## Software acceptance targets

- Site input power never exceeds the declared limit.
- Each session's site power never exceeds the port or vehicle-acceptance limit.
- Delivered site input energy reconciles with battery-side energy plus conversion losses within numerical tolerance.
- Identical input job orderings produce identical canonical reports.
- Each job appears in output, including deadline-missed or unserved jobs.
- Completion exactly at deadline is treated consistently as success.
- A hand-computable FCFS/EDF fixture detects a real scheduling-policy difference.
- Invalid capacity, arrival/deadline order, non-finite data or charging efficiency is rejected.

## Known limitations

No SOC-dependent charge taper, battery temperature/aging dynamics, stochastic arrival patterns, charger outage/repair, connectors shared across incompatible vehicles, staff/labor constraint, swap handling, reserve-energy requirement, grid voltage/reactive-power study, generation forecast, flexible load shifting, or price/carbon optimization is included. Port power and site cap are hard constants. v0.2 should add charger outages, charge curves, temperature, forecast uncertainty and paired tariff/carbon-aware policies only after v0.1 passes and boundary tests are stable.

This queue simulator is a scheduling screen, not a battery warranty model, grid study, safety case or deployment authorization.