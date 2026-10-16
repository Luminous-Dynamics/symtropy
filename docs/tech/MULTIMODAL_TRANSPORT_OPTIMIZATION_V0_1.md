# Multimodal Transport Optimization — Research and Model Contract v0.1

**Status:** research specification; no route-level winner or cost-saving claim has been established.  
**Related issues:** [TRANSPORT-OPT-001](https://github.com/Luminous-Dynamics/symtropy/issues/1601), [CARGO-MOD-001](https://github.com/Luminous-Dynamics/symtropy/issues/1604)  
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

### 5.3 Modular cargo units and transfer interfaces

Treat the **cargo module** as a first-class planning object. A reusable module should cross compatible legs while keeping its identity, declared contents, dimensions, mass, sealing/inspection status and custody history intact. Modularity is useful only where the next vehicle/terminal can actually accept and safely handle the unit; a universal digital identity is not a universal physical form factor.

Prefer adapters to established profiles rather than defining a new global box standard:
- ISO Series 1 freight containers with declared ISO 668 envelope/rating and ISO 1161 corner-fitting profile;
- regional rail/road swap bodies;
- pallets, crates, roll cages and reusable inner modules for warehouse and last-mile work;
- tanks, flat racks, and bulk/FIBC units where commodity handling requires them;
- aircraft-approved IATA Unit Load Devices (ULDs) as a separate family with aircraft-position, restraint, serviceability, centre-of-gravity and certified maximum gross mass constraints.

Represent the profile family, standard edition/version and conformity evidence explicitly. A profile that merely resembles a standard must be marked unverified, not certified. A route edge or handling station should declare what profile families, mass/envelope limits, locks/lift interfaces, orientation and environmental controls it supports. Compatibility requires checking the actual equipment and operating envelope, not comparing nominal length alone.

At minimum, each module instance carries:
- stable module ID, profile ID/version and traceable inspection/serviceability state;
- outer dimensions and usable volume with explicit units;
- tare mass, declared payload mass, maximum payload and maximum gross mass;
- handling/locking interface and allowed orientation;
- cargo requirements such as temperature band, humidity, power, tilt/shock, segregation and hazardous-goods controls;
- declared security seal state and custody/contents digest;
- cleanliness/contamination state for food, water, medicines and sensitive cargo;
- module availability, location, ownership/custodian and expected empty return/repositioning.

**Safety contracts:** reject overweight/over-volume loads, unsupported equipment/profile pairs, failed inspection, broken seals where policy requires intact seals, hazardous cargo incompatibility, cold-chain excursion, invalid lift/restraint and unavailable modules. Air cargo must use aircraft-approved ULD envelopes and restraint rules, not a scaled-up maritime container assumption. Packing/strapping and centre-of-gravity safety remain explicit operational checks; packing authority and module identity do not certify the cargo as correctly secured.

Model nested units only where the equipment actually supports them. E.g., standardized inner tote → roll cage/pallet → swap body/container → vehicle is a *composed handling plan*, not an implicit physical nesting guarantee. Each consolidation/deconsolidation event records a custody receipt, handling time, energy/cost, seal change, mass/volume reconciliation and damage/temperature observations. Cargo contents are not reweighed/recounted as a new independent load on every leg.

The economic comparison must include the whole-cycle cost: module purchase/depreciation, cleaning, inspection, repair/loss, return imbalance, empty repositioning, tare mass, usable-volume utilization, labor/robotic handling, compatible terminal investment and repacking avoided. Report utilization by both mass and volume (the binding constraint matters), and compare net door-to-door outcomes against a declared baseline. Do not advertise modularity as savings until handling dwell, damage, empty-return and asset-pooling assumptions are measured.

Research anchors:
- [ISO 668:2020](https://committee.iso.org/standard/76912.html): Series 1 freight-container dimensions/classifications/ratings.
- [ISO 1161](https://committee.iso.org/cms/live/live/en/sites/isoorg/contents/data/standard/08/64/86404.html): corner/intermediate fittings; Edition 6 was listed as under publication in October 2026, so adapters should record the exact edition rather than assume it.
- [ISO 3874:2017](https://www.iso.org/standard/63826.html): handling and securing methods for Series 1 containers built and tested to ISO 1496. Treat these handling methods and the applicable container/equipment manuals as physical constraints; matching corner fittings or envelope dimensions alone is not a lift approval.
- [GS1 EPCIS 2.0 / CBV](https://www.gs1.org/standards/epcis): a standardized cross-organization visibility-event model for what/when/where/why, custody and sensor observations. Prefer an EPCIS adapter for exchange of logistics events rather than inventing a parallel event vocabulary. EPCIS interoperability is not a physical-safety certificate, and local typed evidence requirements still need explicit validation.
- [IATA Unit Load Devices](https://www.iata.org/uld): aircraft ULDs are aircraft components subject to structural, serviceability, weight and operational requirements; the [2026 ULD Regulations, 14th edition](https://www.iata.org/en/publications/manuals/uld-regulations/) added life-cycle assessment guidance.
- [IATA ULD serviceability checks](https://www.iata.org/en/publications/newsletters/iata-knowledge-hub/how-to-conduct-a-uld-serviceability-check/): gross mass, allowable position load and centre-of-gravity constraints remain explicit.
- [IMO/ILO/UNECE CTU Code](https://unece.org/transport/intermodal-transport/imoilounece-code-practice-packing-cargo-transport-units-ctu-code): packing, securing, overloading and contents declaration. The [2025 CTU Code revision working document](https://unece.org/transport/documents/2026/07/working-documents/imoilounece-code-practice-packing-cargo-transport) must be tracked as a revision/work document unless and until its final status is confirmed.
- [Reusable packaging review](https://www.sciencedirect.com/science/article/pii/S0925527320301201): reusable packaging requires joint accounting of economic/environmental impacts, packaging-system design and reverse logistics; reuse does not guarantee a net benefit when return, cleaning and loss costs are ignored.
- [World Development Report 2025 — Standards for Development](https://www.worldbank.org/en/publication/wdr2025): standards and containerization as coordination infrastructure.

## 5.4 Space logistics is a separate physical-feasibility profile

The same logistics concepts can span terrestrial, orbital, cislunar, lunar-surface and later Mars supply chains, but a generic graph edge with hand-entered cost/time is not a flight-feasibility model. Keep the optimization architecture reusable while requiring a domain-specific physical feasibility adapter for space legs.

### Physical movement constraints

A future `SpaceLegProfile` / adapter should declare the exact service and model version and bind at least:

- origin/destination operational domain and reference frame;
- launch/arrival window, schedule confidence, service provider and capacity commitment;
- trajectory/orbit/transfer model and qualified ephemeris/kernel/assumption commitments;
- delta-v budget, propellant and propulsion assumptions, dry/wet mass, payload fraction and staging/refuelling assumptions where relevant;
- allowable payload envelope, mass properties, restraint/handling contract and any crew/cargo safety constraints;
- thermal, radiation, power, environment, lifetime, storage and consumables limits;
- rendezvous/docking/berthing or planetary landing/surface transfer interfaces;
- maintenance, repair, spares, refurbishment, reuse and end-of-life assumptions;
- planetary-surface mobility range, payload, terrain and local energy for surface legs;
- for local resource utilization, feedstock evidence, process throughput/yield, energy/water inputs, product quality, equipment availability and reliability.

Missing constraints must make a leg unsupported or explicitly scenario-only; a plausible duration or cost value must not upgrade it to feasible. Reuse a mission-validated astrodynamics/service source rather than implementing a new orbital propagator inside the general-purpose route enumerator. Keep mission analysis and certification outside this planning crate.

### Distinct networks, linked by explicit handoffs

Do not collapse these graphs:

1. **Physical logistics graph** — movement of cargo/assets through terrestrial legs, launch and rendezvous operations, orbital/cislunar transport, landers and planetary surface mobility.
2. **Communication/contact graph** — direct or relay contact opportunities, time windows, direction, latency, capacity, confidence and security profile. Reuse a Delay-Tolerant Networking/contact-plan standard (for example BPv7/BPSec, CCSDS Schedule-Aware Bundle Routing, and applicable LunaNet profiles); do not reimplement their routing protocols here.
3. **Evidence-reconciliation graph** — delayed observations, custody/event commitments, source provenance, conflict sets and reconciliation outcomes across disconnected operational domains.

A physical vehicle may move when no communications contact exists. A message may arrive without cargo moving. A new synchronization can reveal conflicting evidence, but must not automatically decide which physical-state claim is true.

### Whole-life supply-chain objective

For terrestrial-to-space/lunar/Mars scenarios, optimize more than currency or elapsed time. Keep visible, dimensioned objectives for:

- recurring launch mass and propellant;
- packaging/tare, cargo utilization and transfer/crew/robot handling burden;
- resupply cadence, storage capacity/lifetime, consumables and inventory risk;
- missed launch/contact-window consequences and service reliability;
- power, thermal control, maintenance, spares, repairability, refurbishment and reuse;
- local production / ISRU scenarios and their energy, feedstock, yield and equipment constraints;
- waste closure, recycling, planetary-protection / contamination constraints where applicable;
- redundancy, common-mode failure, spare logistics and recovery from long-duration outages.

Do not presume local-resource production improves resilience: scenario benefits must include process yield, power, maintenance and uptime assumptions, and be tested against launch-supplied alternatives. Do not combine currency, mass, mission risk and environmental constraints into a single score without an explicit policy and sensitivity analysis.

### Boundaries and qualification

- Earth freight standards (for example ISO container profiles and GS1/DCSA/IATA cargo-data models) remain terrestrial/modal adapters; they are not automatically lunar or Martian physical standards.
- Space communication standards qualify their own data/service interfaces; they do not certify cargo movement or physical delivery.
- Time and reference-frame conversion must preserve source time scale/clock, frame, kernel/profile commitment and uncertainty. UTC and terrestrial coordinates are not universal defaults.
- Sol Atlas may visualize these outputs only with explicit observed / curated / derived / scenario statuses; visualization is not a feasibility proof.
- Mycelix may coordinate identity, evidence, scoped authorization and delayed reconciliation; a DHT receipt alone does not establish a physical fact.
- Symthaea predictions remain advisory and must not authorize launch, custody, inventory, dispatch or equipment mutation.

### First useful qualification fixture

After the terrestrial route/module/resource invariants pass, use a small, fully synthetic Earth logistics → launch-site transfer → scheduled Earth-orbit/cislunar service → lunar relay/contact opportunity → lander/surface depot scenario. Include a missed launch window, a missed communication contact, a stale inventory/propellant report, one shared-capacity conflict and delayed contradictory custody evidence. The independent verifier should prove that physical movement, communications delivery and evidence reconciliation remain distinct. This is an architecture/algorithm fixture, not a flight trajectory or mission design.

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

Update the initial historical context with the latest results as soon as this model is used: the South African Yearbook 2024/25 traces freight-rail volumes down from 226 million tonnes in 2017/18 to 149.5 million tonnes in 2022/23; the FY2025/26 Transnet results subsequently report about 167.9 million tonnes, up from 160.1 million tonnes in FY2024/25, but below the 180-million-tonne target reported for FY2025/26. See [Transnet Annual Results](https://www.transnet.net/AnnualResults) and [Reuters' 10 September 2026 report](https://www.reuters.com/world/africa/south-africas-transnet-returns-profit-asset-sale-higher-freight-volumes-2026-09-10/). The older decline, later recovery, operational reliability, asset availability, maintenance, crime/vandalism and investment constraints all matter, but national annual throughput is not a corridor-level capacity or reliability measurement. The government's 250-million-tonne Transnet network target by 2029 is a policy target, not an achieved outcome or a valid route-level forecast. Every pilot must record data period/as-of date and use measured corridor service, tariffs, terminal dwell, shipment mix and reliability rather than extrapolating a national total.

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

A first deterministic Rust reference module now lives in the domain crate `symtropy-transport-optimization`. It enumerates a bounded Pareto frontier for a single demand over validated nodes, movement edges and explicit intermodal transfer edges. It records operator cost, user cash cost, external cost, time, distance, energy, greenhouse-gas emissions and declared safety/disruption risk proxies. It enforces per-edge capacity and quantity-unit compatibility, deadlines, transfer limits and checked arithmetic. Search exhaustion fails closed rather than silently truncating the frontier. The search also rejects repeated origin/state cycles so zero-cost loops cannot become misleading Pareto alternatives through lexicographic tie-breaking.

The crate's toy passenger and cargo cases use **synthetic illustrative numbers only**. They demonstrate that a fast/high-cost/high-emission route can remain on the Pareto frontier beside a slower/lower-cost/lower-emission intermodal route. They do not represent actual Gauteng–Durban prices or a recommendation for a real corridor.

The modular-cargo contract is being strengthened in layers:

- **Safe orientation:** module profiles and handling equipment must explicitly share an approved axis orientation. Dimensions are checked after that declared permutation; geometric fit alone does not certify stability, lashing, cargo distribution or safe restraint. **Center-of-gravity envelopes, local floor/point-load pressure, acceleration load cases and engineering load-distribution validation are not yet implemented** and remain mandatory follow-on checks before real dispatch use.
- **Load-securing evidence:** profiles may require an inspection/securement record; missing or failed evidence rejects dispatch.
- **Aircraft position compatibility:** when equipment requires verified ULDs, the profile must declare position-specific aircraft/ULD eligibility evidence matching the aircraft type, loading position, accepted ULD type code and gross-mass limit. These are evidence fields, not proof of an external certificate by themselves.
- **Revisioned custody handoff:** custody transfers check source custodian/location, module identity, revision and stable contents digest. Stale/replayed receipts fail; a seal-state change requires a nonempty evidence digest; the returned state carries the latest receipt digest. Persistence, cryptographic signatures, issuer authorization and concurrent-writer serialization remain outside this function and must be supplied by the authoritative custody service.
- **Mass and volume utilization:** report both rates and identify the binding capacity; the lower percentage alone should not obscure whether mass or cube is the actual limit.
- **Reusable-module economics:** a scenario calculator includes initial acquisition, cleaning/inspection, repair, empty repositioning, a conservative replacement allowance from the declared loss-rate assumption, avoided baseline handling cost, plus empty-return energy/emissions. It distinguishes reusable lifecycle cost from avoided baseline handling cost and reports net incremental cost; its result is sensitivity arithmetic, not measured savings or a statistical loss prediction.
- **Route search integrity:** only simple paths over the declared node/mode state space are expanded, and a route may not return to its origin. A regression fixture ensures a zero-cost loop cannot displace a direct route because its IDs sort earlier.
- **Terminal transfer equipment and resource binding:** transfer equipment is distinct from a rail-wagon/vehicle position. Its profile declares terminal, supported mode pairs, module family, handling interfaces, orientation/envelope, gross-mass limit, temperature capability and power supply. A separate `CargoTransferResourceBinding` pins a proposed transfer to the exact assignment ID, capacity-scope ID, module ID/profile ID, edge ID, equipment ID and terminal ID, and declares the capacity demand in the edge's unit. `validate_cargo_transfer_capacity_plan` validates a whole proposed batch: assignment IDs must be unique; edges and equipment budgets must be uniquely defined; requests must refer to transfer edges at the correct terminal; capacity units must match; and aggregate requests must fit both each edge's declared capacity and the shared equipment budget across all referenced edges in the same capacity scope. Capacity totals use checked integer arithmetic and produce a deterministically ordered usage report. This numeric gate is not a time-window reservation or proof of live availability. The capacity-scope identifier is caller-supplied planning context and must not be misrepresented as a verified calendar window. `validate_cargo_transfer_plan` couples each binding with its module profile, module instance and transfer equipment, then requires both the aggregate capacity gate and `check_cargo_transfer_compatibility` against canonical graph edges.

The next layer is now represented by `CargoTransferCapacityEvidenceV1`: it binds a capacity budget to a budget identity, issuer/authority claim and profile, source record/schema/digest, revision lineage, explicit time profile, capacity observation time, repository record time and declared validity interval. Freshness is evaluated by `validate_cargo_transfer_capacity_evidence` against a caller-supplied epoch-second evaluation time and explicit `CargoTransferCapacityFreshnessPolicy`; it does not read a wall clock, and each evidence time-profile ID must match the consumer policy's expected time profile. The wrapper `validate_cargo_transfer_plan_with_evidence` applies that gate to evidence-bearing budgets before calling the combined physical/capacity validator. It rejects duplicate budget identities and evidence not referenced by any proposed assignment, then returns a `CargoTransferPlanEvidenceReport` containing both the deterministic numeric capacity report and the exact freshness-qualified source evidence in deterministic order. This retains the source digest, authority claim, revision, and distinct time fields needed to replay or audit the proposed plan. The source business-event time, capacity-observation time and repository record time are separate fields; only the explicit capacity-observation time determines age. Schema-specific adapters must also enforce that source's cross-field rules—for example, EPCIS 2.0.1 requires a sensor observation time to be no later than eventTime, while eventTime describes the business step rather than sensor condition time ([GS1 EPCIS 2.0.1, sensor fields](https://ref.gs1.org/standards/epcis/2.0.1/)). The generic freshness validator intentionally does not invent those source-specific mappings. The declared state rejects superseded, revoked or conflicted records.

This is a **structural/time-window qualification gate only**: source status, authority identity and digest are not cryptographically authenticated or independently recomputed here; the record does not prove the real equipment has that capacity, that a reported observation is true, or that a reservation is held. The caller must supply a consistent time profile/evaluation instant, trustworthy verification receipts, conflict-resolution result and authoritative capacity snapshot. Fresh snapshot is not committed reservation, and concurrent booking still requires an authority-owned compare-and-consume/lease transaction with idempotent receipts and reconciliation. The capacity-scope ID remains a planning label, not a calendar window by itself.

The planned operation-window and overlap-capacity layer is being developed separately in [stacked draft PR #1621](https://github.com/Luminous-Dynamics/symtropy/pull/1621). It must remain unqualified until exact-head CI passes. This base layer intentionally keeps source-snapshot freshness/provenance separate from scheduling and reservation claims.

The contracts fail closed on mass/volume/envelope overruns, unsupported mode/family/orientation/interface, nonserviceable or improperly sealed units, absent load-securing evidence when required, temperature/power failure, arithmetic overflow, stale custody revisions, contents changes and aircraft-ULD/position mismatch. A compatible interface means at least one explicitly compatible certified/declared interface is supported, not that every optional interface on a module must be present.

This remains a *single-demand route-option enumerator plus module/equipment, transfer-compatibility, custody and lifecycle-economics contracts*, not a complete transport optimization system. The new transfer check evaluates one supplied edge/equipment pair; network edges do not yet persistently reference equipment resources, and module profiles are not yet incorporated into route-level cargo-flow assignment or equipment scheduling. The solver does not yet allocate multiple simultaneous demands against shared capacity, optimize timetables/service frequency or vehicle schedules, select infrastructure investments, estimate stochastic reliability from data, or validate cost factors against operators. Custody checks are pure state-transition guards, not a signed/atomic ledger. Standard references and evidence digests must not be described as ISO or IATA certification.

At this tranche, the Rust source has 49 authored test functions, including aggregate-capacity acceptance, shared-equipment overcommit across separate edges, duplicate assignment IDs and edge IDs, missing/duplicate budgets, invalid scope/terminal, capacity-unit mismatch and arithmetic-overflow cases. They still require exact-head CI before being treated as passing.

Next steps:
1. requalify the exact Rust crate after the capacity-evidence tranche; confirm formatting, Clippy, unit/invariant tests and workspace build;
2. add center-of-gravity, floor/point-load, acceleration/restraint and load-distribution evidence contracts from applicable equipment manuals;
3. persist/resolve capacity evidence as graph-owned or versioned source records and independently verify issuer/source receipts; define owner authority and conflict/supersession semantics, then add compare-and-consume reservations, idempotent effect receipts and concurrency/partition reconciliation before claiming confirmed capacity;
4. integrate `validate_cargo_transfer_plan` into multi-demand cargo-flow allocation, preserving explicit unserved demand, capacity-conservation invariants, and independent batch-output verification;
5. qualify future operation windows and overlapping capacity with the schedule-aware layer in PR #1621, then introduce timetable, empty-module repositioning and reusable-asset pooling feasibility;
6. add versioned adapters for real South African route, service, tariff, module inventory, dwell, load, energy and reusable-packaging data;
7. compare Pareto and sensitivity outputs against held-out operator/official measurements before discussing a specific cost-saving winner.

Learned predictions, real-time dispatch and investment MILPs stay out of scope until the base flow/accounting invariants and evidence adapters qualify.
