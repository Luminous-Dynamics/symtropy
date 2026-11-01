# Earth-to-Orbit Payload Supply-Chain Benchmark v0.1

**Status:** research and benchmark contract; synthetic-first; no operational capability or site approval implied  
**Tracking:** [EARTH-ORBIT-CHAIN-001 #3228](https://github.com/Luminous-Dynamics/symtropy/issues/3228)  
**Composes:** [TRANSPORT-OPT-001 #1601](https://github.com/Luminous-Dynamics/symtropy/issues/1601), [FREIGHT-MAGLEV-001 #3201](https://github.com/Luminous-Dynamics/symtropy/issues/3201), [ORBITAL-ACCESS-001 #3202](https://github.com/Luminous-Dynamics/symtropy/issues/3202), [SPACE-CAPABILITY-LEDGER-001 #1972](https://github.com/Luminous-Dynamics/symtropy/issues/1972), Sol-Atlas [SPACEPORT-GEOGRAPHY-001 #393](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/393)  
**Version:** 0.1.0

## 1. Decision problem

For a declared payload demand and destination orbit/service, compare complete supply-chain portfolios that deliver **accepted payload mass to the required destination and time window**. The chain may span:

`supplier / factory → inland freight → port or airport gateway → integration / processing → launch range → launch vehicle → target orbit`

Future alternatives may replace the final launch stage with a separately qualified tether, skyhook, orbital ring, partial ring or launch-loop architecture. They must not be collapsed into one technology class.

The port, industrial campus, launch pad, tracking station and payload integration site may be geographically separate. A seaport does not have to be co-located with a rocket range. Surface routing, terminal transfer, range safety, launch windows and orbit geometry must all be represented.

This benchmark is the orchestration and reconciliation seam. It is not a second transport optimizer, GIS, launch-trajectory solver, orbital dynamics engine, tunnel solver, or authority for permitting and safety.

## 2. Core paradigm

### Optimize useful delivered service

Do not optimize tonnes arriving at a coastal port, maximum vehicle lift, theoretical corridor speed, or advertised launch price in isolation. Optimize useful accepted payload service at the declared destination.

Every cargo unit needs a stable identity and mass state through the chain. Distinguish:
- payload and payload adapter;
- container, packaging and handling equipment;
- launch vehicle dry mass and propellant;
- integration/processing equipment;
- infrastructure construction mass;
- recurring mission payload.

A payload must not be double-counted or silently disappear when transferred between modes. Mass, units, capacity and transfer state are conserved or explicitly transformed.

### Treat each stage as a constrained service

Every stage declares input/output compatibility, mass/volume envelopes, handling duration, dwell/storage capacity, throughput, energy, emissions, cost, availability, dependencies, rights, evidence tier and responsible authority.

An unserved launch window is not zero delay. Unknown capacity is not unlimited capacity. An incompatible orbit is infeasible, not merely expensive. Missing rights or safety evidence remains Unknown / NeedsEvidence.

## 3. System boundary and interfaces

Use an acyclic campaign graph for the first tranche. Each node and edge should be typed and versioned.

A stage contract should preserve:
- stable node / edge / cargo / mission IDs;
- mode and technology family;
- input and output cargo references, mass, dimensions and units;
- location or facility reference and coordinate provenance where applicable;
- route length, profile, capacity, operating calendar and service schedule;
- handling, queue, storage, clean-room or hazardous-material constraints;
- time interval, launch window and required-by deadline;
- capex, fixed opex, variable cost and uncertainty interval;
- energy carrier, energy amount, emissions factor and source;
- failure domain, shared dependencies and recovery option;
- source identity, version, license/permission, acquisition time and claim ceiling;
- time-profile identifier / epoch convention, matching the authoritative transport allocation contract [#1639](https://github.com/Luminous-Dynamics/symtropy/issues/1639);
- specialist-model receipt, exact model version, configuration digest and output digest.

The end-to-end evaluator may join specialist receipts but cannot reinterpret an unqualified specialist result as verified physics.

## 4. Alternative portfolios

At minimum compare:
1. Existing freight and launch chain with operating-policy optimization.
2. Existing port and conventional railway upgrades.
3. Conventional rail, road and intermodal alternatives.
4. MagRail / linear-induction retrofit where the demonstrated configuration supports the required cargo.
5. New conventional rail with tunnelled sections only where a surface alignment is constrained.
6. Dedicated underground freight maglev or automated pods.
7. Alternative port, dry port, inland terminal, integration campus and distributed manufacturing portfolios.
8. Existing orbital launch providers and current vehicle families.
9. Future elevator, tether, skyhook, orbital ring, partial ring and launch-loop alternatives, each with distinct physics, evidence tiers and construction-mass assumptions.
10. No-build / continue existing providers baseline.

A linear motor is not automatically magnetic levitation. A short port-yard pilot is not evidence for a long-distance underground maglev network. A deorbit tether demonstration is not an elevator. A conceptual orbital ring is not a launch service.

## 5. Geography and orbit coupling

For each launch site, derive the accessible orbit envelope from launch latitude, azimuth, target inclination, range safety and downrange constraints. Do not assign a generic equatorial bonus.

Equatorial launch can exploit Earth's rotational velocity for suitable eastward, low-inclination missions. The advantage is target-orbit dependent; high-inclination and polar missions do not receive the same benefit. The candidate site must be tested against actual azimuth corridors, population and territory overflight, air/sea traffic, weather and emergency range operations.

Keep geography architecture-specific:
- **Rocket range:** target-orbit/azimuth and range-safety corridor.
- **Classical space elevator:** the standard architecture uses an equatorial Earth anchor, a long tensile tether, GEO region and counterweight.
- **Slanted geostationary cable variant:** Birch (1982) argued that a geostationary cable could in principle descend to a non-polar surface point, although an equatorial site is more convenient. Treat this as a distinct exploratory variant with explicit off-axis cable geometry, transverse loads, anchor reactions, tension distribution, climber motion and weather exposure—not as a proven way to remove the material problem.
- **Orbital-ring Jacob's Ladder variant:** Birch's preliminary concept describes a low-Earth-orbit ring supporting geostationary skyhooks that hold above specific points on Earth, with cable ladders to the surface. Model the ring/skyhook station-keeping, Earth-relative geometry, precession, dynamic loads, failure containment and terminal access separately; do not reduce the search to equatorial latitude or treat the 1982 proposal as demonstrated engineering.
- **Orbital ring:** orbital plane, moving mass-stream, support/transfer dynamics and terminal geometry.
- **Skyhook / momentum-exchange tether:** orbit and tether motion, capture/release geometry and momentum exchange.
- **Launch loop:** separate ground-supported dynamic architecture and paired endpoint / corridor constraints.

An orbital ring is not a space elevator. Do not infer that all ring surface terminals must lie on the equator.

## 5A. Orbital last-mile delivery is a separate service

A successful primary launch does not necessarily complete the customer's mission. Rideshare customers may be inserted into a primary orbit chosen for another payload; an OTV/OMV can provide a separate maneuvering / last-mile service toward the customer's required orbit, phasing or destination state.

Compare:
- dedicated launch directly to the target orbit;
- rideshare with already-compatible orbit;
- rideshare plus one OTV/OMV;
- rideshare plus a time-expanded sequence of multiple cooperating servicers / depots;
- existing provider / no-build.

NASA's 2026 Small Spacecraft Technology State-of-the-Art Chapter 10 describes the OTV/OMV market as nascent, with many planned systems but few having flight heritage. Distinguish propulsion-oriented OTVs from hosted-function OMVs, and record exact vehicle configuration, flown-vs-proposed status, payload envelope, propellant, power, navigation, communications and capture interface. See https://www.nasa.gov/smallsat-institute/sst-soa/integration-launch-and-deployment/ (section 10.5).

The planning layer models discrete mission assignments, vehicle schedules, depots, inventory, payload compatibility, deadlines and transfer windows. It consumes orbit/trajectory/rendezvous qualification receipts; it does not independently certify trajectory dynamics, docking or capture. Physical object custody during interbody travel remains with Symtropy #103.

Benchmark failure cases where:
- primary injection does not satisfy final target orbit;
- phasing / plane-change / altitude requirements exceed the qualified propulsion budget;
- a missed rendezvous or failed capture leaves cargo in transit;
- a servicing vehicle or depot outage breaks a multi-hop plan;
- nominally independent servicers share navigation, comms, power, supplier or depot dependencies.

## 5B. Resilience must include irrecoverable and no-resupply states

Do not assume every failure is repairable, replaceable or eventually replenished. For each deterministic scenario, state whether repair, reroute, spare, resupply, recycling, local manufacture or ISRU is actually available within the mission horizon and evidence tier.

A closed-loop pathway is not qualified merely because a mass can be collected or processed: record resulting material grade, yield, energy, tools, facility, contamination and qualification for reuse. Unknown recovery remains Unknown / NeedsEvidence. A deterministic fault set is not a calibrated probability distribution.

The OECD's 2026 assessment identifies concentrated launch activity, qualified supplier scarcity, specialized inputs and critical-material concentration as real space-economy vulnerability classes. Map sub-tier suppliers and common processing dependencies; two nominal vendors are not independent if they rely on the same qualified material, semiconductor or component source. See https://www.oecd.org/en/publications/the-space-economy-at-a-glance-2026_cbf9b240-en/full-report/managing-vulnerabilities-dependencies-and-sustainability-risks-of-the-space-economy_4d97ef6d.html.

Research/benchmark owners:
- Orbital last-mile service: Symtropy #3253.
- No-resupply, recovery and qualified closed-loop supply: Symtropy #3254.

## 6. Frozen synthetic corpus

Freeze at least 24 synthetic cases across the following dimensions:
- coastal port adjacent to a range vs separate gateway and launch site;
- inland origin connected by rail, road, MagRail, tunnelled conventional rail or dedicated freight system;
- high, low, seasonal and uncertain demand;
- payload classes with different mass, volume, fragility, cleanliness and hazardous-material constraints;
- low-, medium- and high-inclination target orbits including polar / sun-synchronous;
- alternate launch windows, weather closures and range restrictions;
- bottlenecked integration, clean-room, propellant, storage, port, rail, road, tracking and grid capacity;
- independent versus shared suppliers, gateways, utilities, fibre, borders and access links;
- construction outage, schedule overrun, demand shock, vehicle delay and missed launch window;
- no-build / current-provider baseline that can win;
- future ring / tether construction cargo under explicitly speculative mass, power and material assumptions.

Deterministic fault sets are the starting point. Do not manufacture operational failure probabilities. Stochastic distributions require source-backed calibration and explicit scope.

## 7. Independent baselines

Small cases must have hand-computable mass, unit, timing and capacity oracles and exhaustive enumeration where tractable.

Larger instances should be compared against:
- time-expanded minimum-cost flow / multicommodity flow where assumptions fit;
- MILP / CP-SAT for declared discrete decisions and scheduling;
- deterministic queueing or discrete-event simulation for terminals and launch campaigns;
- greedy and current-practice baselines;
- robust / stochastic / multi-objective methods with matched uncertainty and compute budgets.

Record solver version, hardware, runtime, memory, stopping condition, objective and optimality gap. Do not claim better-than-SOTA from unmatched workloads or a synthetic-only win.

## 8. Metrics

Preserve separate ledgers and report:
- accepted payload kg delivered by destination orbit/service and deadline;
- mission cadence, launch-window service rate and campaign completion time;
- cost per accepted delivered kg and whole-chain generalized cost;
- capex, fixed/variable opex, energy, maintenance, inventory, insurance, renewal and decommissioning;
- energy and lifecycle GHG per accepted delivered kg;
- transfers, handling/dwell, queues, storage and terminal utilization;
- missed-window recovery, time-to-recover and unmet demand;
- N-1 and common-mode service loss across transport, port, grid, tracking and range dependencies;
- Pareto regret / optimality gap vs exact and classical references;
- local economic distribution, land/water/ecological impact and public contingent liabilities.

Keep private financial, public fiscal, economic resource, distributional, environmental and resilience ledgers distinct. Gross payroll, operator revenue and cargo price are not automatically net social benefit.

## 9. Mandatory negative controls

- cheap chain whose launch azimuth cannot serve the target orbit;
- equatorial preference incorrectly applied to a polar/high-inclination mission;
- missing or double-counted mass at any transfer;
- stage capacity below demand reported as feasible;
- unknown capacity treated as unlimited;
- linear-induction propulsion mislabeled as maglev;
- one shared port/road/grid/tracking dependency counted as independent redundancy;
- ignored weather closure, range closure or launch window;
- unknown rights or missing range-safety evidence treated as approved/safe;
- unqualified tether material or ring dynamics promoted to operational capability;
- ring construction mass omitted from the supply-chain ledger;
- no-build / existing-provider baseline omitted.

## 10. Acceptance targets

- 100% fail-closed outcomes on frozen semantic and hard-gate negative controls.
- Exact mass, unit, transfer and capacity reconciliation on every hand-checkable case.
- Exact exhaustive-oracle agreement on bounded small cases.
- Independent analytic orbit/azimuth compatibility agreement.
- Byte-stable replay for fixed manifests and pinned specialist-model receipts.
- Every alternative whose objective intervals overlap remains visible on the Pareto frontier.
- Held-out case family and evaluation protocol frozen before tuning.
- Runtime, memory, hardware, optimality gap and solver baselines reported; no unsupported superiority claim.

## 11. Research and industry references

- NASA, [Chapter 14: Launch](https://science.nasa.gov/learn/basics-of-space-flight/chapter14-1/): equatorial rotational velocity and target-orbit constraints.
- Birch, P., [Orbital Ring Systems and Jacob's Ladders — I](https://nss.org/wp-content/uploads/Orbital-Rings.pdf), *Journal of the British Interplanetary Society*, 35 (1982), pp. 475–497. Primary concept source for off-equatorial geostationary cables, stationary skyhooks and orbital-ring variants; the paper expressly describes the analysis as preliminary and exploratory.
- Kenya PPP Directorate, [proposed spaceport feasibility study](https://pppkenya.go.ke/kenya-advances-plans-for-proposed-spaceport-project/), 1 October 2026; Space in Africa, [transaction-advisory appointment](https://spaceinafrica.com/2026/10/28/kenya-appoints-consortium-to-advise-on-proposed-commercial-spaceport/), 28 October 2026. These are feasibility steps, not a construction decision.
- NASA, [Small Spacecraft Technology State of the Art — In-Space Propulsion](https://www.nasa.gov/smallsat-institute/sst-soa/in-space_propulsion/): documented tether flight demonstrations; not an Earth elevator or orbital-ring demonstration.
- NASA, [Small Spacecraft Technology State of the Art — Integration, Launch, Deployment and Orbital Transport](https://www.nasa.gov/smallsat-institute/sst-soa/integration-launch-and-deployment/), 18 May 2026, §10.5: OTV/OMV last-mile service and the distinction between nascent announced systems and vehicles with flight heritage.
- OECD, [The Space Economy at a Glance 2026 — managing vulnerabilities, dependencies and sustainability risks](https://www.oecd.org/en/publications/the-space-economy-at-a-glance-2026_cbf9b240-en/full-report/managing-vulnerabilities-dependencies-and-sustainability-risks-of-the-space-economy_4d97ef6d.html): launch-provider concentration, specialist supplier dependencies and critical-material concentration.
- Selvamurugan, Lightsey and Ho, [Time-optimal trajectory design of distributed space systems for in-space transportation in Earth Orbit](https://doi.org/10.1016/j.actaastro.2026.05.056), *Acta Astronautica* 248 (November 2026), pp. 17–35. A time-expanded multi-vehicle trajectory/scheduling case study; preliminary case-specific findings, not universal superiority.
- Hu et al., [A state-of-the-art review of underground logistics systems](https://doi.org/10.1016/j.tust.2025.106641), 2025, reviewing 243 publications.
- IEEE, [A Reality Check on Maglev Technology for the Hyperloop Transportation System](https://doi.org/10.1109/ACCESS.2024.3491904), 2024.
- DP World / Deendayal Port Authority / Nevomo, [750 m MagRail pilot exploration](https://www.dpworld.com/en/news/dp-world-deendayal-port-authority-and-nevomo-sign-mou-to-explore-autonomous-magnetic-rail-freight-movement-in-india), 2025. This is port-scale exploration, not proof of a long-haul underground freight network.

## 12. Ownership and claim ceiling

- #1601 and #3201 own surface network/mode calculations.
- #1639 owns time-windowed physical-resource allocation and its explicit time-profile semantics; this chain layer consumes its qualification receipt and verifies profile identity rather than recalculating resource capacity.
- #3202 owns orbital-access architecture physics.
- Sol-Atlas #393 owns evidence-bounded spaceport geography.
- #1972 owns capability history and evidence tiers.
- This benchmark owns end-to-end stage contracts, campaign scheduling, mass/time/cost reconciliation, bottleneck analysis and cross-stage resilience.

Synthetic results remain synthetic. No output establishes a site, maglev corridor, launch vehicle, tether, orbital ring or complete supply chain as safe, licensed, commercially viable or buildable.
