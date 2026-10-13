# Multimodal Transport Optimization — Research and Model Contract v0.1

**Status:** research specification; no route-level winner or cost-saving claim has been established.  
**Related issue:** [TRANSPORT-OPT-001](https://github.com/Luminous-Dynamics/symtropy/issues/1601)  
**Related existing design:** [Symtropy Vehicle & Mobility Design Bible](./Symtropy%20Vehicle%20%26%20Mobility%20Design.md) and [interbody transit](https://github.com/Luminous-Dynamics/symtropy/issues/103).

## 1. Executive conclusion

The initial investment thesis is **automate and coordinate the transport network before betting on driverless vehicles in one mode**. The model must be able to falsify this thesis for a particular corridor.

There is no universal winner between ground, aerial, and marine transport. The right decision depends on origin/destination geography, distance, demand density, commodity, time criticality, terminal access, available capacity, infrastructure condition, energy supply, safety, reliability, and the value of access to the people being served.

A reasonable hypothesis to test is:

- **Passenger movement:** first improve safe walking/cycling access, frequent affordable buses/BRT, conventional and automated rail where demand supports it, and reliable feeder/first-last-mile connections. Consider autonomous shuttles as feeders to shared public transport—not as an automatic substitute for a high-occupancy service.
- **Cargo:** test rail and maritime/inland water for dense long-distance flows where suitable infrastructure and handling capacity exist; road for collection, distribution, flexible routing, and places the fixed network cannot reach; air for genuinely time-critical, high-value, medical, or remote-service cargo where the added cost and impacts are justified.
- **Automation:** compare dispatch and scheduling, train/ship/terminal coordination, predictive maintenance, fleet utilization, safety assistance, and vehicle autonomy as separate interventions. Automation may increase utilization or reliability, but can add capital, remote-control, connectivity, cybersecurity, certification, energy, and workforce-transition costs.
- **Network design:** compare door-to-door services and their transfer operations, not just a truck against a train or an aircraft against a ship in isolation.

This hierarchy is a **research prior, not the optimizer's answer**. Recommendations must come from local demand and capacity data, fully accounted costs, and sensitivity analysis.

## 2. Why this is worth modeling

Transport affects access to work, education, health care, food, water, trade, and disaster response. Lowest operator expense is not equivalent to lowest total social cost. Nor is lowest carbon intensity per kilometre enough: a route can be infeasible, poorly connected, too slow for a deadline, unreliable, inaccessible, or lack capacity.

Evidence anchors (each has a scope limit):

1. The IEA's *Energy Efficiency 2025 — Transport* reports that transport accounts for around 30% of global energy demand and that road transport accounts for nearly 90% of domestic transport energy demand. It specifically notes that buses transport people much more efficiently than private cars. This argues for testing occupancy and network effects, not only vehicle automation. [IEA, 2025](https://www.iea.org/reports/energy-efficiency-2025/transport).
2. The IEA's *The Future of Rail* reports that rail carried 8% of the world's passengers and 7% of global freight transport while representing about 2% of transport energy demand in its referenced global data. These figures are from the report's historical baseline, not a current per-route forecast. [IEA, *The Future of Rail*](https://www.iea.org/reports/the-future-of-rail).
3. The European Environment Agency finds rail and waterborne transport much more greenhouse-gas-efficient than road freight and aviation in its EU analysis. Its comparison uses EU data for 2014–2018; its authors warn against applying average factors to specific journeys. It is a directional prior for candidate scenarios, not a substitute for South African or route-specific data. [EEA, 2021](https://www.eea.europa.eu/en/analysis/publications/rail-and-waterborne-best-for-low-carbon-motorised-transport).
4. UN Trade and Development estimates that ships carry over 80% of world trade in goods by volume, while its 2025 review highlights volatile costs, route disruptions, rerouting and decarbonization costs. Marine is structurally essential to global freight, but dependence on a port or chokepoint also creates resilience risk. [UNCTAD, *Review of Maritime Transport*](https://unctad.org/topic/transport-and-trade-logistics/review-of-maritime-transport).
5. UITP says automated road vehicles are most beneficial when integrated into inclusive public transport and warns that without such integration, private AVs may simply replace private cars. Automated metro/rail already has a long operational history, unlike some public-road autonomy use cases. [UITP, *Automated Mobility*](https://www.uitp.org/publications/automated-mobility-next-step-public-transport/).
6. As of 2026, the IMO has adopted a non-mandatory Maritime Autonomous Surface Ships (MASS) Code, effective 1 July 2026. It emphasizes risk assessment, connectivity, remote operations, cybersecurity and human oversight; the ship's master retains overall responsibility. Maritime autonomy therefore has a defined regulatory direction, not a free pass around safety or accountability. [IMO, MASS Code](https://www.imo.org/en/ourwork/safety/pages/autonomous-shipping-%28mass%29.aspx).

## 3. Decision table: hypotheses to compare, not fixed prescriptions

| Transport task | Primary candidates to test | Where automation may help | Main reasons a candidate may lose |
|---|---|---|---|
| Dense urban passenger demand | Walking/cycling access, bus/BRT, metro or commuter rail, transfers | Dispatch/headway control, predictive maintenance, safe signaling, accessible feeder shuttles | Low density, low service frequency, unsafe access, poor transfers, high capital cost |
| Intercity passenger trips | Rail, scheduled coach/bus, car/shared service, aviation for long distances | Timetable/fleet coordination, rail signaling, safety assistance | Demand and route distance, station/airport access, reliability, competing infrastructure |
| Dense long-haul freight | Rail, maritime/coastal/inland water where geographically viable, road feeder | Train/yard planning, consist utilization, port/terminal scheduling, predictive maintenance | Unreliable line capacity, slow transfers, lack of sidings, terminal dwell, empty return legs |
| Collection and last-mile delivery | Road, electric vans/trucks, cargo bikes, local consolidation | Routing, load matching, depot charging, safe low-speed autonomy in constrained areas | Low drops per route, congestion, failed deliveries, battery/cargo constraints, curb access |
| Intercontinental general cargo | Maritime plus rail/road legs and terminal transfer | Port calls, berth/crane/yard scheduling, vessel energy management | Chokepoints, weather, border delays, container imbalance, port congestion |
| Urgent small/high-value/medical cargo | Road or air/drone depending on geography and deadline | Dispatch and airspace/route deconfliction within approved operating envelope | High energy/cost per payload, weather, payload/range, certification, landing/handoff availability |
| Rural/peri-urban passenger access | Scheduled bus, demand-responsive transit, shared accessible van; rail where demand fits | Dynamic pooling, guaranteed transfer, low-density service coordination | Low demand, deadheading, affordability, long pickup waits, digital exclusion |
| Disaster or supply disruption | Mixed-mode routes and reserves | Early warning, rerouting, inventory/vehicle staging, resilient dispatch | Single points of failure, power/communications loss, capacity exhaustion |

No row should be converted into an investment recommendation without an evidence record for the actual corridor and service class.

## 4. What is being optimized?

### 4.1 Separate ledgers

Keep at least four ledgers distinct; do not add all of them together without checking transfer payments and double-counting:

1. **Operator resource cost:** fleet and equipment, capital, maintenance, energy, staff/remote operations, terminals, handling, repositioning, insurance, certification, and administration.
2. **User burden:** fares/freight price, door-to-door time, waiting, transfers, unreliability, access/egress, affordability, damage/spoilage, inventory and missed-deadline costs.
3. **Public/fiscal cost:** subsidy, public capital, maintenance of public rights-of-way, emergency response, enforcement and foregone public budget.
4. **External and distributional outcomes:** safety risk, climate and local pollution, noise, congestion, road wear, land/ecosystem impacts, resilience, service coverage, accessibility, and impacts on workers and communities.

A fare or freight charge is a cost to the user and revenue to the operator; it is generally a transfer within the social ledger, not two independent resource costs. Fares remain important for affordability and demand response. Report private, fiscal, and social results separately so the accounting does not hide this distinction.

### 4.2 Whole-life cost

For a policy scenario \(s\), calculate present value or annualized cost on a consistent time horizon:

\[
C_{\text{life}}(s) =
C_{\text{capital, annualized}} +
C_{\text{operations}} +
C_{\text{maintenance}} +
C_{\text{energy}} +
C_{\text{fleet and staff}} +
C_{\text{terminals and transfers}} +
C_{\text{inventory and delay}} +
C_{\text{expected disruption}} +
C_{\text{externalities}} +
C_{\text{transition and decommissioning}}.
\]

Include empty running/repositioning and realistic load/occupancy factors. Include infrastructure construction and maintenance where evidence permits; omitting these may materially change mode comparisons. Record every category's boundary. Do not use a per-vehicle-kilometre number without stating payload/occupancy, utilization, and empty-return assumptions.

For freight, keep measures such as cost per tonne delivered, cost per tonne-kilometre, time distribution, damage/spoilage and late-delivery probability visible. For passengers, keep cost per completed journey and passenger-kilometre, door-to-door travel time, waiting/transfer time, accessibility, affordability and service span visible. They are related, but not interchangeable metrics.

### 4.3 Multi-objective, not one hidden magic score

Default objective is a **Pareto set** over:

- whole-life resource/social cost;
- user burden and affordability;
- door-to-door journey/delivery time and reliability;
- life-cycle energy and greenhouse-gas emissions;
- safety risk and other health/environmental impacts;
- resilience to outages and hazard scenarios;
- accessibility and geographic/equity of service.

Use hard constraints for legal and safety requirements, minimum service, accessibility and capacity. A policy may choose transparent trade-off weights or constraints after seeing the Pareto frontier, but no universal weight vector should be silently embedded in Symtropy. Report how the recommendation changes under plausible weights and disputed assumptions. A cheaper plan that strands rural communities or violates a safety requirement must not win because the affected dimension was underweighted.

## 5. Network and demand model

### 5.1 Time-dependent multimodal graph

Represent candidate infrastructure and scheduled service as a directed graph \(G=(V,E)\).

Nodes can be communities, passenger stops, rail yards, intermodal terminals, depots, ports, airports, warehouses, hospitals and delivery points. Edges represent a feasible movement **or** a transfer/handling operation. Each edge is time-dependent and records:

- mode, service/operator and route identity;
- distance, expected duration and a distribution/interval for duration;
- capacity by relevant unit (seats, tonnes, volume, pallets, TEU, slots, payload/range);
- timetable, operating window, frequency, cut-off, transfer/handoff time and compatibility;
- energy/fuel and emissions model, with declared boundary and source;
- variable and fixed resource costs and who pays them;
- reliability, weather sensitivity, incident/outage profile and recovery options;
- accessibility, legal/operating constraints and service eligibility;
- provenance, geography, validity interval, units, uncertainty and digest of source data.

Transfers are real edges: account for terminal fees, loading, handling equipment, wait, inventory, handling loss/damage, transfer emissions, and missed connection risk. A road–rail–road itinerary must pay its real first/last-mile and terminal costs. If the model cannot represent a constraint, it must mark the case unsupported instead of pretending the route is feasible.

Use a time-expanded graph or equivalent scheduling model where timetable, cut-off, deadline, congestion or transfer windows matter. Avoid a single average edge weight where this permits impossible catchable connections.

### 5.2 Different demands require different contracts

**Passenger demand** is indexed by origin, destination, time/departure window, trip purpose or service class when supported, party size, fare/affordability constraints, accessibility requirements and willingness-to-travel-time ranges. Model completed journeys, not only vehicle movements. Preserve unmet demand explicitly.

**Cargo demand** is indexed by origin, destination, commodity, mass, volume, shipment size, handling/temperature/fragility restrictions, declared value class if needed for inventory/damage, delivery window, perishability and service priority. Mass and volume must both fit; hazardous/restricted cargo rules apply. Do not treat passengers as interchangeable tonnes or use one generalized cost for all commodities.

The system may share corridor capacity (road congestion, port slots, energy supply) but each demand type keeps its own service constraints and output metrics.

## 6. Optimization formulation

Let \(k\) identify a passenger group or cargo class and \(a\) a feasible time-dependent movement or transfer edge.

- \(f_{k,a}\): flow assigned to edge \(a\);
- \(y_j \in \{0,1\}\): build/upgrade investment \(j\) (terminal, siding, charging, electrification, fleet or capacity);
- \(u_m \in \{0,1\}\): select declared automation/operating intervention \(m\);
- \(q_k \geq 0\): explicitly unserved demand (never silently discarded).

The starter model solves a capacity-constrained flow assignment over the currently available network. A later investment model jointly chooses \(y\), services, fleet/schedules and flows. Core constraints:

1. **Flow conservation:** each demand unit begins at its source and ends at its destination or is counted as unserved.
2. **Capacity:** edge, vehicle, fleet, station, port, terminal, charging, storage and transfer capacity cannot be exceeded.
3. **Time feasibility:** connections, cut-offs, operating windows and deadlines obey the same timebase.
4. **Mode/commodity eligibility:** vehicle range/payload, infrastructure gauge/clearance, dangerous-goods rules, accessibility, weather envelope and permits are respected.
5. **Demand and service:** fulfilment, fare/price limits, minimum frequency/coverage, accessibility and reliability targets are explicit.
6. **Budget and deployment:** capital, operations, staffing, power, fleet, terminal and maintenance resources are limited by the declared scenario.
7. **Safety/environment:** legal/safety constraints are non-negotiable; explicit carbon, pollution, noise or habitat limits can be imposed where policy requires them.
8. **Conservation/accounting:** delivered cargo mass/volume, passenger journeys, fleet capacity and energy are reconciled; transfer and empty movements remain visible.

Start with a reference shortest-path solver for small examples, then capacity-constrained min-cost flow. Use MILP/CP-SAT or another declared optimization method when discrete infrastructure, fleet/schedule, or multi-commodity constraints are required. Document optimality gaps, runtime limits and fallback behavior. No heuristic or approximate solution should be reported as proven optimal without a corresponding bound/status.

## 7. Treat automation as an intervention, not a mode

Every automation option is a delta against a matched baseline with the **same demand and service obligation**. Examples:

- dispatch and timetable coordination;
- predictive maintenance and asset-health monitoring;
- terminal crane/yard/berth scheduling;
- train/ship/fleet utilization and load matching;
- driver assistance and collision avoidance;
- autonomous rail or constrained-route vehicle operations;
- remote/automated ship operations;
- approved drones or aircraft for a specific service class.

Model capital and lifecycle costs, telemetry/connectivity, control centres, redundant communications, cyber-risk management, certification, insurance, training/retraining, emergency response, staffing and remote oversight. Potential improvements to utilization, delay, incident rates, staffing, maintenance or energy must be backed by local/operator evidence or represented as explicit sensitivity ranges. An unknown benefit is not a zero-cost guaranteed saving.

Run at least these paired counterfactuals:
1. maintain/repair existing service with no new automation;
2. operational/digital coordination only;
3. electrification or fleet renewal only;
4. terminal/intermodal capacity improvement;
5. mode-shift with required feeder investment;
6. automation plus the above, with interaction terms exposed where evidence supports them.

This separates the value of better coordination from the value of buying an autonomous vehicle.

## 8. Uncertainty, resilience and governance

Evaluate scenarios \(\omega\) for demand growth/decline, peak/off-peak, energy/fuel prices, carbon factors, load factor, labour/maintenance cost, weather, port/rail disruption, road congestion, asset failure, security/communications outage, and delivery-window risk.

Publish base, stress and recovery cases, plus probability-weighted expectations only when the probabilities are defensible. Consider robust or regret-minimizing solutions where uncertainty is substantial. A plan that is slightly cheaper in the mean but catastrophically fragile may be inferior.

Every input requires a record with: name/definition, unit, source, data owner/licence, geography, observed period/as-of date, route/commodity/mode scope, transformations, uncertainty interval, sample size, model version, and evidence class (measured, official statistic, derived, assumed, or illustrative). Missing or expired inputs are \`Unknown\`/\`NotEvaluated\`, not zero.

Symthaea may later forecast demand, incidents or time distributions, but predictions must be calibrated and carry uncertainty. A learned prediction must not override safety, capacity, eligibility, conservation or accessibility constraints. Preserve decision inputs, model/build identity, selected routes, rejected alternatives, constraint slacks, result digests and objective weights for replay and review.

## 9. Pilot order and data plan

### Pilot A — Gauteng to Durban freight

Compare observed road-only and rail-plus-road/intermodal routes for compatible cargo classes. Capture realistic terminal dwell and drayage, service availability, reliability distributions, empty return, handling, cargo value/time sensitivity and actual route capacity. Start with public/aggregate data; do not claim a specific lane winner without operator and terminal data.

### Pilot B — Johannesburg/Pretoria passenger access

Compare existing public transport, planned/incremental capacity, transfer quality, bus/rail/BRT integration, safe walking access, and feeder services. Report access-to-work/health/education, journey time, affordability and service frequency by area and relevant population group. Include a low-demand rural/peri-urban case to avoid optimizing only for the densest corridors.

### Pilot C — domestic ports and bulk/agricultural/mineral freight

Separate commodity classes, handling requirements, train availability, terminal/storage capacity, port queues, road/rail access and disruption dependencies. Marine is a link in a whole chain, not a standalone origin-to-destination answer.

### Pilot D — urgent medical and small high-value cargo

Compare road and approved air/drone options with explicit deadlines, weather/operating envelopes, payload, handoff, failure-recovery and energy/cost per delivered shipment. Avoid generalizing an emergency-medical result to routine freight.

**Source discovery for South Africa:**
- Department of Transport [Freight Databank, network statement and travel-survey links](https://www.transport.gov.za/).
- [Roadmap for the Freight Logistics System in South Africa](https://www.transport.gov.za/wp-content/uploads/2023/02/Roadmap-for-the-Freight-Logistics-System-in-South-Africa-FINAL-FOR-RELEASE.pdf).
- [South Africa Yearbook 2024/25](https://www.gov.za/documents/other/south-africa-yearbook-202425-20-feb-2026).
- Transnet and terminal operator service/capacity, tariff, maintenance and reliability data; Stats SA transport and price statistics; municipal origin-destination/travel survey data; schedules and network data from operators; suitable public geospatial data for network geometry.

The South African Yearbook 2024/25 reports freight-rail volume falling from 226 million tonnes in 2017/18 to 149.5 million tonnes in 2022/23 and describes operational reliability, asset availability, maintenance, crime/vandalism and investment constraints. These are historical context, not a measure of current route capacity. The Minister of Transport's 2025 targets (including 250 million tonnes on the Transnet network by 2029) are policy targets, not achieved outcomes. Model actual measured service and costs instead of using either value as a forecast.

## 10. Qualification plan

Build an auditable and small first, then scale:

**Q0 — Schema and arithmetic**
- explicit units and currencies, FX/date rules, inflation/base year, real/nominal discount rate and infrastructure lifetime;
- exact toy graphs whose minimum-cost routes and flows can be solved by hand;
- no negative or non-finite costs, no missing mode/transfer semantics, no invalid capacities or unproven edges.

**Q1 — Flow integrity**
- every demand unit accounted as delivered, in-transit, or explicitly unserved;
- conservation through transfers; capacities never exceeded; inaccessible destinations do not receive phantom service;
- deterministic replay from canonical inputs and exact model identity.

**Q2 — Sensitivity and Pareto**
- change value of time, transfer delay, load factor, fuel/electricity cost, carbon value and disruption rates independently;
- demonstrate where a recommendation changes, expose Pareto-dominated routes and do not collapse uncertainty into false precision.

**Q3 — Independent benchmark comparison**
- compare route time, charges, dwell, energy/emissions and throughput to held-out operator or official measurements;
- use consistent lifecycle boundaries and independent sources where possible;
- report errors and missing data per case, not only average score.

**Q4 — Planning evidence**
- scenario-based investment/capacity comparisons with capital and operational horizons;
- stress tests for port/rail outages and extreme weather;
- affordability/accessibility and worker-transition outcomes;
- independent review before any public investment or operational deployment recommendation.

A simulation of a route graph is not physical validation of vehicle autonomy or safety. Any automatic dispatch/control integration requires a separate safety case, operating envelope, fallback design and certification path.

## 11. Minimum output for every modeled alternative

Each result should include:
- route and service legs, transfer edges and why rejected alternatives fail;
- delivered/unserved passenger/cargo quantity and any constraints binding at optimum;
- whole-life cost and private/user/fiscal/social breakdown;
- cost per passenger journey, passenger-km, shipment, tonne delivered and tonne-km (where meaningful);
- door-to-door time distributions and missed-deadline probability;
- energy, life-cycle GHG, other declared environmental effects, safety exposure and disruption recovery;
- capacity utilization, empty running, dwell/transfer time and robustness;
- service distribution by area/population group, fare burden and accessibility;
- parameter uncertainty, sensitivity/Pareto outcomes, source provenance and exact replay identity;
- evidence label: \`illustrative\`, \`measured\`, \`validated-for-scope\`, \`not-evaluated\`, or \`out-of-domain\`.

No output should headline “cheapest mode” without stating the service obligation, system boundary, cost basis, time horizon, demand/occupancy assumptions, uncertainty and trade-offs.

## 12. Research references

- International Energy Agency, [Energy Efficiency 2025 — Transport](https://www.iea.org/reports/energy-efficiency-2025/transport).
- International Energy Agency, [The Future of Rail](https://www.iea.org/reports/the-future-of-rail).
- European Environment Agency, [Rail and waterborne — best for low-carbon motorised transport](https://www.eea.europa.eu/en/analysis/publications/rail-and-waterborne-best-for-low-carbon-motorised-transport).
- UN Trade and Development, [Review of Maritime Transport](https://unctad.org/topic/transport-and-trade-logistics/review-of-maritime-transport).
- International Maritime Organization, [Autonomous Shipping (MASS)](https://www.imo.org/en/ourwork/safety/pages/autonomous-shipping-%28mass%29.aspx).
- UITP, [Automated Mobility: The Next Step for Public Transport](https://www.uitp.org/publications/automated-mobility-next-step-public-transport/).
- Archetti, Peirano and Speranza, [Optimization in multimodal freight transportation problems: A survey](https://doi.org/10.1016/j.ejor.2021.07.031), *European Journal of Operational Research* (2022): covers network, intermodal and multi-objective formulation families.

## v0.1 reference implementation and next step

A first deterministic Rust reference module now lives in the domain crate `symtropy-transport-optimization`. It enumerates a bounded Pareto frontier for a single demand over validated nodes, movement edges and explicit intermodal transfer edges. It records operator cost, user cash cost, external cost, time, distance, energy, greenhouse-gas emissions and declared safety/disruption risk proxies. It enforces per-edge capacity and quantity-unit compatibility, deadlines, transfer limits and checked arithmetic. Search exhaustion fails closed rather than silently truncating the frontier.

The crate's toy passenger and cargo cases use **synthetic illustrative numbers only**. They demonstrate that a fast/high-cost/high-emission route can remain on the Pareto frontier beside a slower/lower-cost/lower-emission intermodal route. They do not represent actual Gauteng–Durban prices or a recommendation for a real corridor.

This is a *route-option enumerator*, not a complete transport optimization system. It does not yet allocate multiple simultaneous demands against shared capacity, optimize timetable/service frequency or vehicle schedules, select infrastructure investments, estimate stochastic reliability from data, or validate cost factors against operators. These limitations are explicit.

Next steps:
1. qualify the exact Rust crate and its toy-network tests;
2. add multiple-demand min-cost flow with shared capacity and explicit unserved demand;
3. introduce timetable and transfer-window feasibility;
4. add versioned input adapters for real South African route, service, tariff, dwell, load, and energy data;
5. compare Pareto and sensitivity outputs against held-out operator/official measurements before discussing a specific cost-saving winner.

Learned predictions, real-time dispatch and investment MILPs stay out of scope until the base flow/accounting invariants and evidence adapters qualify.
