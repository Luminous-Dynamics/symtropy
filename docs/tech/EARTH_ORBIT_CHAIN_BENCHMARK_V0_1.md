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
- Hu et al., [A state-of-the-art review of underground logistics systems](https://doi.org/10.1016/j.tust.2025.106641), 2025, reviewing 243 publications.
- IEEE, [A Reality Check on Maglev Technology for the Hyperloop Transportation System](https://doi.org/10.1109/ACCESS.2024.3491904), 2024.
- DP World / Deendayal Port Authority / Nevomo, [750 m MagRail pilot exploration](https://www.dpworld.com/en/news/dp-world-deendayal-port-authority-and-nevomo-sign-mou-to-explore-autonomous-magnetic-rail-freight-movement-in-india), 2025. This is port-scale exploration, not proof of a long-haul underground freight network.

## 12. Ownership and claim ceiling

- #1601 and #3201 own surface network/mode calculations.
- #3202 owns orbital-access architecture physics.
- Sol-Atlas #393 owns evidence-bounded spaceport geography.
- #1972 owns capability history and evidence tiers.
- This benchmark owns end-to-end stage contracts, campaign scheduling, mass/time/cost reconciliation, bottleneck analysis and cross-stage resilience.

Synthetic results remain synthetic. No output establishes a site, maglev corridor, launch vehicle, tether, orbital ring or complete supply chain as safe, licensed, commercially viable or buildable.
