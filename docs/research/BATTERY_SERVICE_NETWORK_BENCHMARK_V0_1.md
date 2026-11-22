# BATTERY-SERVICE-NETWORK-001 — qualified pack service network v0.1

**Owner:** Symtropy #6674  
**Implementation slice:** `tools/ev-passenger-cost-frontier/src/battery_service_network.rs`  
**CLI:** `battery_service_network_screen`  
**Frozen fixture:** `tools/ev-passenger-cost-frontier/fixtures/battery_service_network_v0_1.json`  
**Input schema:** `tools/ev-passenger-cost-frontier/schemas/battery_service_network_v0_1.schema.json`  
**Claim ceiling:** exact synthetic one-epoch pack allocation only

## 1. Research thesis

A battery is not just a scalar state-of-charge value attached to an aircraft. It is a serialized, condition-dependent, reconfigurable resource in a network of aircraft, carrier routes, swap systems, chargers, racks, service crews, depots and mobile replenishment vehicles.

The decision unit should be **qualified pack capability at the required place and time**, not nominal pack count or swap count.

A mission requires a complete capability vector:
- conservative lower bound on usable energy after operational and contingency reserve;
- conservative lower bound for peak power under the expected current/load profile;
- conservative upper bound for thermal state and qualified maximum temperature;
- exact pack/airframe/interface configuration;
- pack identity, custody and post-maintenance test evidence;
- observation time and validity interval;
- evidence producer, method, reference and configuration digest.

Capacity cannot compensate for insufficient peak power. A nominally charged pack is not available if it is quarantined, in maintenance, cooling, charging, in transit, custody-unresolved or configured for another aircraft.

The system can recommend a candidate allocation; it cannot authorize electrical energization, flight, maintenance release or return-to-service.

## 2. Ownership boundaries

| Owner | Responsibility | This benchmark consumes / produces |
|---|---|---|
| Symtropy #6674 | Battery inventory/service-network benchmark | One-epoch pack allocation now; network inventory/charging/mobile service later |
| Symtropy #6083 | Carrier/UAV sortie and co-availability | Future pack capability/availability intervals; no flight authority from this module |
| Symtropy #6177 | Shared-aircraft allocation/repositioning | Shared aircraft mission set; pack is another separately time-occupied resource |
| Symtropy #6321 / PR #6328 | Module service tasks, repair/replace, lifecycle cost | Maintenance and post-test task evidence; no duplicate maintenance ledger |
| Symtropy #6153 | Strategic activation/deactivation of logistics modes | Mode choice remains outside the battery inventory problem |
| Symtropy #6247 | Matched algorithm/SOTA tournament | Paper-faithful fixed/mobile swapping and policy comparisons |
| Symtropy #5832 / #5241 | Energy-service primitives and site/grid accounting | Charging energy, site meter and grid constraints |
| Symtropy #6308 | Physical carrier/pod interface control | Exact interface/configuration effectivity |
| Symthaea #11271 / #11252 | Pack capability descriptors and leakage-resistant health inference | Read-only health estimates with explicit domain/uncertainty; no duplicated SOH model |
| Mycelix #3709 / #6062 / #6076 | Serialized identity, custody, battery lineage and passport projection | Identity/authority verification inputs; no duplicate battery registry or passport compiler |
| Sol-Atlas | Regional site, access, grid and source-qualified context | Location constraints only; not permits or approvals |

## 3. v0.1 executable slice

The first slice is an exhaustive exact assignment for one simultaneous dispatch epoch and admits no more than six packs and six missions.

It does not schedule time-varying charging, cooling, rack queues, swap bays, mobile pack-service vehicles, cross-epoch reuse or degradation. Its purpose is to establish an exact, auditable allocation contract before expanding into a network optimizer.

### 3.1 Evidence envelope

Every capability, state and boolean assertion is carried with an evidence envelope:

- `asset_id`: must match the serialized pack ID;
- `configuration_digest`: must match the pack configuration digest;
- `producer_id` and `method_id`;
- `evidence_reference`;
- `evidence_class`: synthetic, measured, independent test or unknown;
- `observed_at_unix_ms`;
- `valid_from_unix_ms` and `valid_until_unix_ms`.

The input includes a frozen `dispatch_time_unix_ms`. The evaluator checks:
1. the pack state value itself (available/quarantined/etc.) is nested inside its evidence-bearing state record;
2. all evidence bindings match the exact pack and configuration;
3. evidence window is complete and internally ordered;
3. evidence was observed no later than the dispatch decision time;
4. dispatch time falls inside the evidence validity interval;
5. unknown, missing, expired or incomplete evidence is indeterminate;
6. mismatched identity/configuration or future observation is blocked.

This prevents post-decision information leakage and makes freshness a time-window check rather than an unverified `fresh=true` assertion.

**Trust limitation:** producer IDs, method IDs and references are declarative strings in v0.1. The code checks their presence and binding but does not verify digital signatures, trusted issuer credentials, clock provenance or external attestation. Those remain with Mycelix identity/trust and the appropriate independent assurance owners. An evidence class label is not proof that a measurement is real.

### 3.2 Candidate pair hard checks

The operational state value is nested in `PackStateEvidence`; there is no separate mutable state field. Capability inputs are explicitly bounded: usable energy and peak power are conservative lower bounds; temperature is a conservative upper bound. A point estimate cannot be silently interpreted as a qualified limit.

For each mission-pack pair the evaluator checks:
- matching configuration ID and digest;
- evidence-bearing pack state is available (the value cannot be changed independently of its evidence record);
- fresh identity verification and resolved custody;
- fresh passed post-maintenance test;
- usable energy ≥ mission energy including reserve;
- peak power ≥ mission peak-power requirement;
- current pack temperature ≤ mission limit and pack qualified maximum.

A pack can be assigned at most once. A mission can receive at most one pack in the v0.1 abstraction. Non-available states are blocked; unknown state is indeterminate.

### 3.3 Exact objective

For admitted instances:
1. maximize number of missions assigned;
2. minimize total surplus energy above the mission-with-reserve requirement;
3. break ties deterministically by sorted mission and pack IDs.

This is an explicit deterministic objective for the known-answer exercise, not an economic optimum. It does not yet minimize battery degradation, pack life consumption, grid energy, cost or future option value.

### 3.4 Synthetic known answer

The fixture has four packs and four missions:
- three packs are configuration-compatible and available;
- one pack is intentionally quarantined;
- four missions compete for three assignable packs;
- one mission is intentionally left unserved.

A separate enumeration of the fixture's declared objective found 38 search nodes and selected:
- pack-a → mission-1;
- pack-b → mission-2;
- pack-c → mission-3;
- mission-4 unserved.

The expected mapping is pack-a → mission-1 (300 Wh surplus), pack-b → mission-2 (300 Wh surplus), pack-c → mission-3 (200 Wh surplus), with mission-4 unserved. The workflow asserts the exact mapping, energy margins and 38-node search count.

This independent fixture arithmetic is not evidence that the Rust code compiles or passes its CI tests. At the time of this document update, the exact-head GitHub workflow run has not been observed; do not claim compile, Clippy, rustfmt or Rust tests are green. The current test source contains 35 focused test cases, including expired last-state evidence at replay cut-off and sequence-overflow controls. Eleven additional journal controls cover confirmed transition, timeout lock, blocked retry, independent reconciliation, rejection of synthetic reconciliation, duplicate event/idempotency keys, sequence gaps, illegal transitions, replay cut-off and wrong-serial evidence., including an explicit test that the claimed pack state is carried inside its evidence envelope, including future-observation, expired/malformed time-window, wrong-asset/configuration evidence, stale/unknown pack state, energy-reserve, peak-power, thermal, quarantine/post-test, exact-allocation and deterministic replay controls. Their execution is pending exact-head workflow evidence.

## 4. The next paradigm shift: battery-service capacity as a network option

A swap removes one energy asset from a machine and inserts another; it does not make the removed pack instantly usable again. The removed pack must follow an explicit state path such as depleted → return transport → inspect/quarantine/cool → charging → balancing/post-test → available.

The future service network should compose:
- fixed battery stations;
- mobile battery-swapping vehicles / service trailers;
- serialized battery inventory and configuration classes;
- charging racks and parallel channels;
- grid import and site storage limits;
- cooling, balancing, inspection, repair and quarantine;
- technician/crew qualification and availability;
- demand forecasts, weather and mission eligibility;
- vehicle/aircraft routes and rendezvous.

### 4.1 State machine

Candidate states for a serialized pack:
`Available, Reserved, Installed, Depleted, InTransit, Cooling, Charging, Balancing, Inspection, Quarantined, Repair, PostTest, Retired`.

A state transition requires a confirmed event and the correct predecessor state. A timeout or lost acknowledgement does not permit the allocator to assume the swap happened. It must reconcile physical custody and actual pack state before any further mutation.

Pack, charger, rack, bay, crew, mobile service vehicle and aircraft occupancy must be time-indexed. Two different assets may not claim one exclusive resource interval.

### 4.2 Dynamic formulation sketch (v0.2+, not implemented)

Let:
- (p in P): serialized packs;
- (m in M): mission requests;
- (s in S): depots, fixed docks and staging sites;
- (r in R_s): charging racks/channels at a site;
- (v in V): mobile service vehicles;
- (t in T): time buckets;
- (omega in Omega): demand/weather/grid/maintenance scenarios.

Candidate binary variables:
- (x_{pmtomega}): pack (p) is assigned to mission (m) at time (t);
- (c_{pr tomega}): pack (p) occupies charging channel (r);
- (z_{pstomega}): pack (p) is staged/available at site (s);
- (q_{ptomega}): pack (p) is quarantined or unavailable;
- (u_{vptomega}): mobile unit (v) carries pack (p);
- (a_{mstomega}): mission (m) is served through a feasible carrier/aircraft at site (s).

Core constraints include:

**Pack exclusivity**
[
sum_m x_{pmtomega} + sum_r c_{prtomega} +
sum_v u_{vptomega} + q_{ptomega}
leq 1
]
after the state representation is refined so mutually exclusive states are counted consistently.

**Mission allocation**
[
sum_p x_{pmtomega} leq 1
]

**Compatibility and health**
[
x_{pmtomega} leq 	ext{Qualified}_{pm tomega}
]
where qualification is supplied by the independent pack/mission capability evaluator and cannot be learned or softened by the optimizer.

**Charging rack and grid capacity**
[
sum_p c_{prtomega} leq 1,qquad
sum_{p,r} P_{prtomega}c_{prtomega}
leq P^{grid}_{stomega}
]

**Inventory balance**
A pack can enter a site, charger, service bay or installed aircraft only after the prior state transition is acknowledged and its resource interval is released. Pack balances are conserved across each site/vehicle/time boundary.

**Nonanticipativity**
Decisions at time (t) may only use observations available at (t); scenario-specific future demand/weather/evidence references are not policy features.

The v0.2 objective should be lexicographic or Pareto-based:
1. satisfy hard safety, permission, health, custody and resource constraints;
2. maximize authorized successful on-time service and minimum coverage;
3. control worst-case stockout/wait risk (e.g. CVaR / distributionally robust objectives);
4. minimize full lifecycle cost and degradation per successful delivery;
5. preserve optionality and resilience under uncertain demand and infrastructure outages.

Safety, legal permission, identity, evidence freshness and post-test release are hard constraints, not weighted penalties.

## 5. Research and competitive baselines

### 5.1 Optimization literature

- **Mobile swap + charging under uncertainty:** Xu et al., *Transportation Research Part D* 158 (Sep 2026), Article 105458, [DOI 10.1016/j.trd.2026.105458](https://doi.org/10.1016/j.trd.2026.105458). Closed-loop battery-swapping vans return depleted packs to charging depots; a three-level stochastic model jointly sizes service network, mobile units, battery inventory and charging racks.
- **Fixed/mobile swapping with risk-aware inventory:** Xu et al., *Transportation Research Part C* (Nov 2026), Article 105855, [DOI 10.1016/j.trc.2026.105855](https://doi.org/10.1016/j.trc.2026.105855). Distributionally robust demand, worst-case mean-CVaR and joint chance constraints with exact MILP reformulation. The paper highlights a non-linear infrastructure cost premium for near-absolute reliability and the risk of asset-light flexible-only networks in extreme demand.
- **Charging / swapping / grid integration:** Ai et al., *Transportation Research Part E* 214 (Oct 2026), Article 104897, [DOI 10.1016/j.tre.2026.104897](https://doi.org/10.1016/j.tre.2026.104897). Joint location, inventory and grid constraints.
- **Integrated distribution and swap technology:** *Computers & Industrial Engineering* 214 (Apr 2026), Article 111827, [DOI 10.1016/j.cie.2026.111827](https://doi.org/10.1016/j.cie.2026.111827), set-cover/MILP infrastructure and inventory design with CPLEX/GRASP comparisons.

These papers are road-EV infrastructure comparators, not direct evidence for UAV pack networks. Reproduce their exact problem definitions first, then transfer algorithms under declared aircraft-specific constraints.

### 5.2 Industry product and service references

**DJI Dock 4 — direct UAV dock benchmark.** DJI's official support page specifies:
- battery swap time **under 25 seconds**, measured from dock cover fully closed to swap completion;
- operational turnaround **under 45 seconds**, measured from aircraft landing after the prior flight to takeoff after swap;
- maximum three batteries stored across its four battery compartments;
- two parallel charging channels;
- 15% to 95% per-battery recharge in 27 minutes at 25°C;
- one aircraft accommodated, up to 2,000 W input and -40°C to +50°C specified operating temperature.

These are controlled-environment manufacturer figures, not independent Luminous measurements. The dock accommodates one aircraft and has a 12 m/s maximum allowable landing wind. Its aircraft battery is specified as 260 Wh, 21.42 V, 6S Li-ion NMC, 925 g, 3C discharge and 400 cycles; the cycle figure has a storage-temperature condition (cumulative storage above 35°C no more than 100 days in a 12-month period). The dock uses compressor-based air conditioning. DJI documents a limited backup mode: under a 25°C test, one aircraft battery can power the dock for approximately 4.5 hours from 90% to 10%, with sequential power supply across three batteries. After grid loss, aircraft charging, air-conditioning and wind-speed-gauge heating are unavailable. This should be modelled as degraded continuity, not continued full service. These are product-specific battery/dock data, not generic lifetime or resilience assumptions. Sources: [DJI Dock 4 specifications](https://enterprise.dji.com/dock-4/specs) and [DJI Dock 4 support](https://www.dji.com/support/product/dock-4).

**Hextronics Atlas — larger automated UAV battery inventory.** The public Atlas spec sheet lists DJI Matrice 300/350 compatibility, <2-minute swap, eight sequentially managed battery positions, charging 10%→100% in 45 minutes and 0%→100% in 70 minutes (charging in pairs), plus a supplier-stated >99.99% battery-swapping reliability. The sheet marks IP55 as pending. These are vendor-published data; the >99.99% claim has no visible test denominator/conditions in the sheet and cannot be used as a statistical failure rate until independently substantiated. Source: [Atlas spec sheet](https://docs.hextronics.cloud/product/spec-sheet-atlas.pdf).

**Skydio Dock for X10 — launch latency, not a swap competitor.** Skydio's current technical spec lists airborne in 20 seconds and publishes environmental, launch/landing-wind, rain/snow and communications limits. It does not state a battery-swap time, so retain it as a launch-latency / operating-envelope reference, not on the battery-exchange leaderboard. Source: [Skydio Dock technical specifications](https://www.skydio.com/dock/technical-specs).

**NIO — mature passenger-car swapping network.** NIO reported on 6 February 2026 that it had reached 100 million cumulative swaps, an average three-minute swap, 3,790 stations and 5.28 billion kWh cumulative energy delivered. These are company-reported road-EV metrics; they are not direct UAV turnaround targets. Source: [NIO announcement](https://www.nio.io/news/20260206001).

**Gogoro — service-network economics.** Its 11 November 2026 Q3 filing reports 684,000 subscribers, US$37.1 million battery-swapping service revenue and 24.4% gross margin. It also reports that total Q3 revenue was down 9.1% year-over-year and battery-service revenue down 4.7%; this is a useful caution that scale, subscription revenue and gross margin must be studied with market demand and cash economics. These are company-reported ePTW metrics, not aircraft results. Source: [Gogoro Q3 2026](https://investor.gogoro.com/news-releases/news-release-details/gogoro-releases-third-quarter-financial-results-gogoro-reports).

Do not create one leaderboard by comparing aircraft swap time, car swap time and ePTW subscribers. Report battery class, service boundary, site density, pack inventory, charging parallelism, utilization, reliability and financial denominator.

## 6. Cross-architecture endurance: ground, mobile and airborne replenishment

A separate simulation/reproduction programme is tracked by [AERIAL-ENDURANCE-ARCHITECTURE-001 #6702](https://github.com/Luminous-Dynamics/symtropy/issues/6702). Its research trigger is the July 2026 IEEE Robotics and Automation Letters paper “A Mid-Air Autonomous Battery Exchange Aerial System” (DOI [10.1109/LRA.2026.3693549](https://doi.org/10.1109/LRA.2026.3693549)). The publisher abstract reports a supplier drone rendezvous/docking with a receiver and a belt/stepper mechanism that transfers a replacement battery and collects the depleted one, with simulations and outdoor flight tests under moderate wind. It is a research prototype, not evidence of commercial readiness, operational reliability, or a general aircraft certification basis. [IEEE paper record](https://xplorestaging.ieee.org/document/11520271).

Compare this architecture against:
- fixed automated ground dock;
- mobile ground exchange via carrier/trailer/service vehicle;
- carrier-based detachable UAV or AMR modules;
- in-flight battery supplier aircraft;
- airborne conductive energy transfer where technically feasible;
- tethered ground-powered endurance for stationary missions;
- wireless power only with measured transfer efficiency and alignment tolerance;
- two-aircraft rotation/leapfrog where a second aircraft is safer/cheaper than mid-air physical contact.

The key net-value question is whether mission-hours gained exceed the supplier aircraft's own flight/hover/return energy, payload penalty, hardware mass, extra crew/maintenance, rendezvous time, weather restrictions, risk and capital. Report receiver and supplier energy separately. Do not count transferred battery energy as free energy.

A mid-air transfer event must be modeled as two coupled aircraft reservations plus the pack's serialized state/custody transition. Neither aircraft may be double-booked, and the donor must retain full return reserve after transfer. Unknown transfer outcome locks both asset states pending reconciliation. The entire architecture remains simulation/reproduction only until separately qualified engineering, aviation permission and controlled physical-test approval exist.

## 7. Benchmark tournament

Required matched baselines:
1. one-pack-per-aircraft, charge-in-place;
2. fixed local pack inventory and charging;
3. fixed exchange station with central charging;
4. mobile swap service only;
5. fixed + mobile hybrid;
6. nearest-ready-pack greedy;
7. earliest-deadline / lowest-reserve greedy;
8. deterministic rolling-horizon inventory scheduler;
9. stochastic / distributionally robust inventory and charging;
10. exact DP/MILP on admitted small cases;
11. Symthaea policy distilled from exact oracle, with an independent feasibility gate, only after the deterministic controls are reproduced.

Required ablations:
- capacity-only vs capacity + peak-power health;
- no temperature derating vs independent thermal gate;
- no rack/charger queues vs queue-aware;
- no return route for depleted packs vs closed-loop mobile replenishment;
- no pack identity/configuration binding vs exact serial/effectivity;
- no maintenance reserve vs explicit inspection/quarantine reserve;
- no grid cap vs measured site/grid cap;
- perfect information vs point-in-time observation only;
- homogeneous packs vs multiple compatible classes.

## 8. Metrics and aspirational gates

Report independent axes, not one score:
- successful authorized on-time deliveries, late and unserved demand;
- cover-closed-to-swap-complete time and landing-to-next-takeoff time;
- full service-ready time including verification, post-test and queue;
- p50/p90/p95/p99 wait, stockouts and response-time tails;
- charged/depleted pack inventory, pack utilization, stranded spares and replenishment trips;
- charge input energy, pack-terminal energy, thermal/cooling overhead and site-meter energy;
- pack cycles, capacity fade, resistance/power fade and degradation per successful delivery/flight hour/km;
- mobile service distance, energy, crew-hours and response-time coverage;
- maintenance/quarantine downtime, failed post-tests and repair/replace incidence;
- capex/opex and lifecycle cost per successful on-time service;
- exact optimality gap, regret, compute, memory, hardware and evidence abstention.

Aspirational v0.2+ targets:
- 100% exact agreement on admitted small inventory/charge/occupancy instances;
- zero duplicate pack/mission/rack/charger/crew/bay allocation;
- zero configuration, custody, expired-evidence or future-observation admission;
- zero energy, peak-power or thermal hard-gate violations;
- deterministic replay under frozen inputs and input-order permutations;
- reduce p95 waiting and stockout probability against the strongest matched fixed-only and fixed/mobile baselines;
- mean regret ≤ 1%, p95 ≤ 5% on held-out larger cases where an oracle/reference exists;
- target ≥ 5 percentage-point service improvement only as a matched held-out research stretch goal, never at the cost of a hard-gate violation.

For a physical UAV-dock pilot, measure both DJI timing boundaries under the same start/end events, aircraft/pack class, environmental conditions, test protocol and acceptance checks. Report confidence intervals and sample counts. Do not claim superiority from a nominal best-case record.

## 9. Battery health and passport boundary

Battery mission health is at least two-dimensional:
- **energy capability:** usable capacity/energy with reserve;
- **power capability:** resistance/voltage-sag-limited peak power;
- plus thermal state, uncertainty, configuration and validity domain.

A 2026 *Scientific Reports* study reconstructed 22 VTC6 cells under aviation duty protocols, with 439 capacity, 388 impedance and 415 paired states. It argues that capacity-only health can hide voltage-sag risk. [DOI 10.1038/s41598-026-70252-8](https://doi.org/10.1038/s41598-026-70252-8).

The battery service evaluator consumes these typed, time-valid descriptors; it does not estimate SOH or model battery electrochemistry.

The European Commission's current Battery Passport FAQ clarifies that applicability follows the finished, safely functioning battery boundary. Multiple packs sharing a BMS/common components may be one finished battery; independently functioning packs may be separate. The 2 kWh threshold is assessed against that boundary, case by case. The responsible economic operator remains legally responsible even if it delegates passport data tasks in writing. Sources: [Commission battery FAQ](https://single-market-economy.ec.europa.eu/single-market/digital-product-passport/eu-digital-product-passport-faq-batteries_en).

Mycelix owns the identity, custody, battery lineage and category-specific passport projection. Symtropy does not infer legal applicability or submit a Registry record on the operator's behalf.

## 10. Standards, interoperability and safety boundary

The current road-EV swapping standards are useful references for the carrier's ground-side battery-service equipment, but they do **not** certify a UAV battery, aircraft dock or aerial operation.

- **IEC 62840-1:2025** ([official IEC listing](https://webstore.iec.ch/en/publication/66398)) supplies general guidance and interoperability principles for road-EV swappable and handheld-swappable battery systems, including functional, interface, data and operational interoperability. Its scope is road EVs and it excludes vehicles primarily designed for off-road use and maintenance/service of swap stations or vehicles.
- **IEC 62840-2:2025** ([official IEC listing](https://webstore.iec.ch/en/publication/66390)) specifies safety requirements for road-EV battery swap systems, including communication security, EMC, marking/instructions, electric-shock/other hazard protection, detailed interface/state-transition protocol requirements, capacitor-discharge limits, automated mechanical handling safeguards aligned with ISO 10218-1/-2, and overcurrent/short-circuit protection testing. It also excludes swap-station maintenance/service and EV maintenance/service.
- **ISO/TS 5474-5:2024** ([official ISO listing](https://www.iso.org/standard/81301.html)) addresses vehicle-side automatic conductive power-transfer connections: electrical/mechanical safety, compatibility, environmental conditions, functionality and test procedures. It is not a battery swap protocol and does not specify aircraft interfaces.

Do not claim these standards apply directly to UAV battery systems. The actual system needs a documented applicability assessment and its own aviation, electrical, battery/transport, machinery and site safety basis with competent authorities and qualified labs.

For future engineering verification, derive a design-specific test matrix covering:
- mechanical keying, insertion/removal geometry, retention/locking, misalignment rejection and module drop;
- electrical de-energization, isolation verification, interlocks, contactor sequencing and residual voltage/capacitor discharge;
- chemistry, voltage, current, BMS, firmware and configuration handshake before electrical contact or charging;
- authenticated safety-critical communication; loss, replay, stale-data and timeout behavior;
- overcurrent, short-circuit, reverse polarity, insulation, earth continuity, emergency disconnect and thermal/fire containment;
- EMC disturbance and safe recovery;
- automated mechanism collision, guarding and pinch/crush hazards;
- custody/state reconciliation after exchange and independent post-test before a pack returns to available inventory.

This is a **preliminary engineering verification matrix**, not a normative compliance checklist. The full standards and actual certification basis must be reviewed by competent specialists; the public abstracts/product pages do not establish clause-level conformity or exact acceptance limits.

## 11. Compatibility graph and statistical reliability

### 10.1 Interchangeability is a qualified graph

The strict v0.1 equality check on pack/mission configuration ID and digest is intentionally conservative. A future shared battery pool should not infer interchangeability from a similar connector, nominal voltage, battery outline, chemistry family or vendor label.

Represent compatibility as explicit, versioned edges joining:
- exact serialized pack and cell/module/BMS configuration;
- aircraft battery bay, latch/retention and power interface;
- dock/carrier exchange mechanism and handling envelope;
- any adapter/harness (a new configuration requiring its own qualification);
- charger, charging profile, firmware/BMS protocol and thermal system;
- software, firmware and safety-interlock versions.

Each edge must bind exact endpoint configuration digests, qualification owner/issuer, evidence references, scope, effective date, expiry/revocation, and relevant operating conditions. The qualification scope includes voltage/current transients, peak power, thermal range, cooling, mechanical loads/vibration, insertion/removal life, contact resistance, communication behavior, firmware, emergency isolation and environmental exposure.

Compatibility is not necessarily symmetric or transitive: A↔B and B↔C do not imply A↔C. An adapter creates a new configuration and must not silently inherit all endpoint approvals. A missing, stale, revoked or out-of-scope edge blocks allocation. An optimizer cannot create or widen compatibility. #6308 remains physical interface owner; Mycelix remains identity/custody/provenance owner; qualified engineering/certification authorities own approval.

### 10.2 Reliability needs declared event denominators

Keep six distinct success measures:
1. swap mechanism attempt → completed/verified exchange;
2. aircraft landing → next takeoff (operational turnaround);
3. post-swap inspection/electrical check passed;
4. next mission launched and completed;
5. requested delivery completed on time;
6. network service request completed on time despite asset/charger/grid/weather/permission constraints.

High mechanical swap success does not prove high fleet service reliability. Stockouts, charger queues, site outages, weather/airspace no-go, maintenance and mission failure must remain visible.

For a simple independent Bernoulli model with zero observed failures, the one-sided 95% upper failure-probability bound is:

`p_{upper}=1-0.05^{1/n}`

It takes 2,995 zero-failure attempts to bound failure probability below 10^-3 (99.9% reliability), and 29,956 zero-failure attempts below 10^-4 (99.99% reliability). These sample sizes rely on independence and a defined trial/failure event. A supplier claim such as >99.99% without attempt count, failure count, confidence interval, test conditions and failure definition cannot be used as a statistical reliability comparison.

Physical validation should stratify by distinct dock/mechanism units, pack serial/age/cycle count, temperature/humidity, dust/salt/moisture/contamination, vibration, insertion misalignment, mechanical wear, connector resistance, partial insertion, communication loss/replay, pack mismatch, charger/grid disturbances, emergency stops and operator interventions. Repeated tests on one dock are correlated; use cluster-aware or hierarchical reliability analysis where needed. Publish exact binomial intervals for independent cohorts and separate mechanism reliability from network service availability.

## 12. Event-sourced state and transaction reconciliation

A first event journal reducer is implemented in the same Rust module and tracked by [PACK-STATE-TRANSACTION-001 #6700](https://github.com/Luminous-Dynamics/symtropy/issues/6700). It consumes an initial evidence-bearing pack state and a time-ordered event stream:
- confirmed events apply only an allowed transition;
- timeout/outcome-unknown leaves the last confirmed state unchanged and locks further normal events;
- reconciliation can clear the lock only with measured or independent-test state evidence;
- duplicate event IDs/idempotency keys, non-contiguous sequences, non-monotonic or post-cutoff events, illegal transitions and wrong-asset/configuration evidence are rejected.

The journal CLI is `battery_pack_journal_screen`, with strict schema `schemas/battery_pack_state_journal_v0_1.schema.json` and a synthetic fixture demonstrating available → reserved, install timeout → reconciliation to installed. The fixture is synthetic and does not attest that an actual physical operation occurred. Event evidence references are metadata in this prototype, not cryptographically authenticated messages. Mycelix remains custody/trust owner.

A companion `validate_exclusive_resource_schedule` function and `battery_resource_occupancy_screen` CLI now validate half-open occupancy intervals for packs, aircraft, carrier slots, swap bays, chargers/channels, racks, technicians and mobile service units. It rejects same-resource overlap, expired/invalid reservations, duplicate idempotency keys and evidence that does not cover the full interval. This is a validator, not yet a time-expanded optimizer: it does not choose the best assignment, model charging curves/queue dynamics or optimize cross-epoch inventory.

## 13. Exact fixed-duration charging queue oracle

A separate bounded exact scheduler is now implemented and tracked under [CHARGER-QUEUE-ORACLE-001 #6705](https://github.com/Luminous-Dynamics/symtropy/issues/6705).

The v0.1 scheduler searches at most four jobs, three charger channels and eight discrete time slots. It requires:
- a pack state evidence record declaring the pack Cooling/charge-ready, with exact serial/configuration and validity at the proposed start;
- exact charger-to-pack configuration digest compatibility and charger availability evidence across the planning horizon;
- release slot, exclusive deadline slot, fixed duration and requested charger input power;
- one job per charger channel at a time;
- per-slot aggregate charging draw no greater than site import cap minus non-charging site load;
- grid evidence covering each scheduled time slot.

Its lexicographic objective maximizes completed jobs, then minimizes aggregate waiting slots, then makespan, then deterministic ID tie-breaks. The declared grid input is reported as integer watt-seconds under the constant-power block abstraction. This is not pack-terminal energy or a measured charging profile.

The synthetic fixture uses four pack jobs, two charger channels and six 15-minute slots. The exact known answer schedules:
- job-a / pack-a on charger-1 in slots [0,2) at 3,000 W;
- job-d / pack-d on charger-2 in slots [0,3) at 2,500 W;
- job-b / pack-b on charger-1 in slots [2,4) at 3,500 W;
- job-c remains unserved because its window cannot contain its three-slot duration.

The 8,000 W import cap less 1,000 W non-charging load gives 7,000 W/slot charging headroom. Expected search count is 54 and total declared grid input is 18,450,000 watt-seconds. The workflow asserts the exact assignment, interval boundaries, power, energy-accounting values and no-authority flags.

The new `battery_charge_queue_screen` CLI, strict JSON schema and fixture are integrated into the EV Passenger Cost Frontier workflow. This exact scheduler does not yet model variable charge curves, efficiency, battery thermal evolution, degradation, charger faults, demand charges or stochastic grid uncertainty. It does not energize a pack or release it to service; the event journal #6700 remains the state transition/reconciliation owner.

## 14. Next implementation phases

**v0.2 — exact multi-period charging and inventory**
- model pack-state transitions, charge curves, cooldown/balancing, parallel charger/rack occupancy and grid caps;
- add site locations, travel time and fixed/mobile battery-service routes;
- require confirmed return/depleted-pack custody before charging;
- compare fixed, mobile and hybrid network architectures on small exact cases.

**v0.3 — degradation, risk and policy**
- consume pack-health descriptors with serial/time/configuration provenance;
- optimize degradation per successful mission while preserving energy/power/thermal constraints;
- add distributionally robust demand, weather and charger outage scenarios;
- reproduce cited fixed/mobile network papers and compare rolling-horizon/ALNS/MILP approaches;
- train Symthaea only from leak-free exact labels, with independent shield and abstention.

**Physical pilot — separately gated**
- compatible replaceable packs and connector/power/thermal interlock design;
- instrumented swap mechanism, landing-to-takeoff and complete dock-to-ready measurements;
- repeated service trials, faults, environmental/thermal tests, battery ageing and maintenance;
- aviation, electrical, battery transport/fire and site permissions from competent authorities.

No synthetic result, paper percentage or manufacturer timing claim substitutes for a physical article, independent qualification or legal authorization.
