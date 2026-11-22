# Sustainable Mobility Service Capacity Benchmark v0.1

**Programme owners:** [EV-DRONE-OPS-BASELINE-001 #6226](https://github.com/Luminous-Dynamics/symtropy/issues/6226), [MFB-SOTA-REPRO-001 #6247](https://github.com/Luminous-Dynamics/symtropy/issues/6247), [BATTERY-SERVICE-NETWORK-001 #6674](https://github.com/Luminous-Dynamics/symtropy/issues/6674), [MOBILITY-ENERGY-RESILIENCE-001 #6707](https://github.com/Luminous-Dynamics/symtropy/issues/6707)  
**Assurance layers:** [#6803 flexibility](https://github.com/Luminous-Dynamics/symtropy/pull/6803), [#6809 recovery](https://github.com/Luminous-Dynamics/symtropy/pull/6809), [#6829 capability lease](https://github.com/Luminous-Dynamics/symtropy/pull/6829), [#6837 commitment journal](https://github.com/Luminous-Dynamics/symtropy/pull/6837)  
**Research cut-off:** 22 November 2026  
**Status:** benchmark protocol and industry/SOTA synthesis; performance targets are not achievements.

## 1. The problem: swap speed is not service capacity

A battery swap can take seconds while the same aircraft remains unavailable for a long period due to charge restoration, cooling, inspection, inventory shortages, charger queues, site power limits, weather, airspace permissions, maintenance or crew availability.

The field therefore needs a common answer to:

> Given the same demand, payloads, deadlines, weather, permissions, energy supply and capital budget, how many successful on-time missions can each service architecture deliver sustainably over a multi-day horizon—and at what total lifecycle cost, energy use and risk?

The unit of comparison is **successful useful service delivered**, not swap count, launch count, nominal aircraft capacity, battery nameplate energy or charger utilization.

The benchmark deliberately separates:
1. mechanical exchange success;
2. landing-to-launch turnaround;
3. battery charge restoration;
4. qualified aircraft/pack availability;
5. successful on-time mission delivery;
6. independently measured grid service.

A vendor may lead one measure and lag another. Those results must not be collapsed into one leaderboard.

## 2. Commercial dock baseline: DJI Dock 4

DJI's manufacturer specifications report:

| Measure | Published specification | Boundary |
| --- | ---: | --- |
| Battery swap | <25 seconds | Cover fully closed to swap complete |
| Operational turnaround | <45 seconds | Aircraft landing to next takeoff after swap |
| Stored batteries | Maximum 3 | Four compartments exist, but no more than three may be occupied |
| Parallel charge channels | 2 | Charger-channel count |
| Charge restoration | 27 minutes | One battery from 15% to 95% at 25°C |
| Maximum dock input | 2,000 W | AC site input specification |
| Aircraft accommodated | 1 | One aircraft per dock |

Sources: [DJI Dock 4 specifications](https://enterprise.dji.com/dock-4/specs), [DJI product comparison/support](https://www.dji.com/products/comparison-dock). DJI states that the swap and turnaround timings are from controlled test environments and are for reference.

The charging specification creates a useful independent replenishment comparator. If both channels continuously complete a 27-minute 15–95% cycle, their idealized upper bound is:

\[
2 \times \frac{60}{27} \approx 4.44
\]

15–95% battery-charge sessions per hour across the two channels.

That is **not** 4.44 missions per hour. The spec does not provide the actual mission-energy consumption, dispatch mix, battery degradation, charge curve under other temperatures, queue occupancy, failed charge sessions or network-level service rate needed to infer mission throughput. Nor should the 2,000 W maximum AC input be divided into pack-terminal energy without measured conversion losses and load boundaries.

The correct conclusion is narrower: the 25-second swap metric and 27-minute charge-restoration metric describe different stages of the service process. A benchmark that reports only swap time omits a potential replenishment bottleneck.

## 3. Architecture tournament

Compare the following architectures on identical workloads. They are options, not a predetermined ranking.

| Architecture | Primary strength to test | Common bottleneck to measure |
| --- | --- | --- |
| Fixed single-aircraft dock | Repeatable site, autonomous landing/swap/charge cycle | Limited aircraft count, battery inventory, charge channels and fixed-site outage |
| Fixed multi-bay / multi-aircraft hub | Shared inventory, charging and maintenance scale | Capital, land, channel/queue bottlenecks, correlated site failure |
| Truck-drone rendezvous | Aircraft can serve while the truck moves or pauses | Rendezvous timing, route coupling, truck availability and battery logistics |
| Mobile drone-station trailer | Mobile launch/recovery and charging infrastructure | Trailer mass, parking/setup, road restrictions, site power and crew |
| EV carrier with shared UAV/AMR pool | Cross-carrier aircraft reuse and multimodal fallback | Relocation, cargo transfers, compatibility and shared-resource contention |
| Fleet rotation / leapfrog | Continuous service without a single fixed station | Donor aircraft/vehicle capital, deadheading and reserve consumption |
| Mid-air battery supplier | Potential airborne endurance extension | Supplier energy, rendezvous, transfer success, wind, donor payload and recovery risk |
| Tethered / externally powered operation | Long-duration stationary observation or service | Site power, tether envelope, mobility loss and weather |

Mobile station trailers are already a published architecture; novelty cannot be claimed merely because a truck carries a drone station. Shi, Hong and Li's TSP-MDST model studies a truck-drone collaboration system with a mobile station trailer and a tailored routing-structure-guided adaptive large-neighborhood search. Reproduce that native problem before comparing a carrier variant. [Transportation Research Part E, September 2026, DOI 10.1016/j.tre.2026.104975](https://doi.org/10.1016/j.tre.2026.104975).

The July 2026 IEEE RA-L mid-air battery exchange paper is a research prototype with simulation and outdoor demonstrations in moderate wind. It is a replenishment architecture comparator, not a commercially qualified reliability baseline. [DOI 10.1109/LRA.2026.3693549](https://doi.org/10.1109/LRA.2026.3693549).

## 4. Matched workload contract

Every architecture receives the same versioned scenario bundle:

- geospatial demand and service points, with dataset licence and transformation digests;
- request arrival times, service deadlines, payload mass/volume and pickup/delivery/return semantics;
- identical operating area, road network, candidate sites and time horizon;
- weather fields and forecast issue times, including wind, precipitation, visibility and temperature;
- source-qualified aviation/operating permission windows and no-go intervals;
- aircraft configuration, payload/range/power envelopes and maintenance/inspection rules;
- serialized battery inventory, compatible pack/BMS/dock/charger configurations and energy/power/thermal evidence;
- chargers, channels, swap bays, pads, crews, carriers, trailers and service units as separately scheduled resources;
- site load, charger demand, import capacity, renewable/storage traces and outage scenarios;
- cost model covering aircraft, carrier, dock/trailer, battery inventory, charging infrastructure, staffing, energy, maintenance, insurance, failures, depreciation, facilities and financing assumptions.

Do not let one architecture use a denser demand distribution, more favorable weather, larger battery inventory, looser payload, additional aircraft, extra charging channels or a more permissive flight envelope.

The evaluation must be nonanticipative: a policy cannot consume observations that would not have been available at the decision time. Forecast vintages and observation cut-offs are part of the frozen input.

### Dataset rights

- Amazon Last Mile Routing Research Challenge data has a non-commercial licence on the AWS Registry listing. Keep it in a non-commercial reproduction track unless a separate rights review/permission allows commercial use.
- The 2026 Mendeley multi-visit pickup/delivery/resupply dataset is listed CC BY 4.0; use the exact version and provide attribution.
- South African scenarios need separately sourced local demand, road, weather, land/right-of-way, site, grid and aviation permission data. US operational claims or datasets do not establish South African feasibility.

See [#6247](https://github.com/Luminous-Dynamics/symtropy/issues/6247) for the existing dataset rights and source-native reproduction rules.

## 5. Common metric contract and denominators

### Service outcome

| Metric | Numerator / denominator | Required split |
| --- | --- | --- |
| On-time mission service | Successfully completed by-deadline requests / all committed requests | By request class, payload, site and day |
| Useful mission-hours | Productive service time / aircraft-hour and site-hour | Separate paid, standby, reposition, maintenance and blocked time |
| Order-to-door latency | Request receipt to verified completion | p50/p90/p95/p99; report unserved separately |
| Service availability | Time the complete qualified service chain could accept a request / scheduled operating time | Identify resource, site, weather, permission and grid downtime |
| Fallback success | Requests successfully served by truck-only, AMR, locker or human fallback / requests handed to fallback | Report fallback latency and cost |

### Battery and infrastructure

| Metric | Correct measurement boundary |
| --- | --- |
| Swap mechanism success | Successful verified mechanical exchanges / exchange attempts |
| Swap duration | Cover/handling event start to verified swap completion; include p50/p95/p99 |
| Operational turnaround | Landing to takeoff, with the exact definition stated |
| Charge restoration time | Pack-terminal energy at start/end, temperature, charge protocol and pack serial |
| Qualified pack availability | Time pack is in an eligible, tested state and available at the required site |
| Queue wait | Job ready-to-charge to charge start, by channel and charger |
| Charge efficiency | Pack-terminal delivered energy / charger-input energy; never infer from nameplate |
| Grid energy | Site-meter import/export and interval peaks, separately from battery-terminal energy |
| Degradation | Capacity/power/temperature changes over cycles, with serial-level exposure and uncertainty |

### Cost and lifecycle

Report full lifecycle cost per successful on-time mission and per useful mission-hour. Include capital utilization, battery replacement, mobile fleet movement, failed attempts, redelivery, staffing, insurance, downtime, repair, recycling and residual value. Grid-service revenue is separate and may be included only when the exact service was authorized, metered and settled.

### Reliability evidence

Do not treat repeated cycles on one dock as independent fleet units. Report the number of distinct units, pack serials, attempts, failures, exposure and operating strata. For independent Bernoulli trials with zero failures, a rough one-sided 95% upper bound on failure probability is \(1-0.05^{1/n}\); this does not account for common-cause or correlated failures. Publish stratified confidence intervals and common-cause analysis.

A supplier claim such as “>99.99% reliable” is not independently comparable without a disclosed denominator, failure definition, test environment, confidence method and observation boundary.

## 6. Optimization and policy tournament

Use the same frozen scenario, compute and assurance contract for each:

1. fixed inventory / fixed dock;
2. truck-only or ground-robot-only;
3. nearest-feasible and earliest-deadline greedy;
4. fixed one-aircraft-per-carrier;
5. best static regional assignment;
6. myopic shared-fleet reallocation;
7. deterministic rolling-horizon MPC;
8. MILP/MIBLP or exact DP on tractable cases;
9. reproduced ALNS / adaptive tabu search / paper-native heuristics;
10. two-stage robust optimization and column-and-constraint generation;
11. chance-constrained and distributionally robust optimization;
12. DR-SAC / offline RL as proposal policies;
13. Symthaea-guided policy distillation or optimizer search, after exact labels and matched baselines exist.

Current direct comparators include:
- Shared truck-drone fleet sizing/routing and cross-vehicle retrieval, Xie et al., *Drones* 2026, DOI 10.3390/drones10090658.
- Multi-visit pickup/delivery, release dates, lockers and resupply, Liang, Chen and Du, *EJOR* 2026, DOI 10.1016/j.ejor.2026.10.020.
- TSP-MDST mobile drone-station trailer, Shi, Hong and Li, *TR-E* 2026, DOI 10.1016/j.tre.2026.104975.
- DR-SAC for heavy-truck swapping under a KL ambiguity set, Du and Zhu, *Applied Sciences* 2026, DOI 10.3390/app162211346.
- Cluster control for battery swapping and grid regulation, Chen et al., *Electronics* 2026, DOI 10.3390/electronics15112444.

Reproduce the paper-native instances and denominators first. Only then compare all methods on a new common UAV/carrier scenario set. Report solver, hardware, time/memory budgets, seeds, optimality gap, timeouts and the exact baseline configuration. A timeout or infeasible solution cannot be silently excluded.

## 7. Assurance gates applied identically to every policy

The benchmark now has a connected assurance sequence. All candidate policies must pass the same gates:

1. **Offer-window feasibility — #6803 / #6802:** exact per-resource/per-slot allocation, individual energy and power limits, PCC envelope and evidence cut.
2. **Recovery and repeated-shock viability — #6809 / #6808:** chronological per-resource energy state, reserve floors, charger availability, recovery deadline and second disturbance.
3. **Unique capability promise — #6829 / #6827:** one resource cannot be sold to multiple counterparties or overlap mission/recovery/maintenance windows; unresolved offers remain reserved.
4. **Atomic promise journal — #6837 / #6832:** compare-and-swap versioning, append-only event digest chain, consume-once idempotency and evidence-backed terminal reconciliation.

These gates are not performance scores and do not prove physical qualification. They form the same hard feasibility boundary around every optimizer. Measure both:
- unsafe candidate proposals rejected by the gates;
- useful service/revenue lost due to conservative constraints or unresolved capacity.

Do not weaken the gates to improve the headline objective. Improve evidence freshness, reconciliation latency, resource placement, charger capacity or scheduling instead.

## 8. Paradigm shifts and where to aim beyond SOTA

### A. Optimize sustainable throughput, not the fastest local operation

The useful objective is a Pareto frontier over successful on-time missions, useful mission-hours, lifecycle cost, energy per successful delivery and reserve/recovery performance. A sub-25-second swap can coexist with a charge queue, stockout or unavailable aircraft.

### B. Co-design inventory, charging, mobility and recovery

Inventory, charger channels, mobile replenishment, site headroom, battery degradation and mission deadlines must be co-optimized over a multi-day horizon. Treat mobile assets as relocatable resources with travel/setup/crew costs, not as free extra battery capacity.

### C. Optimize resilience as a sequence of disturbances

Test a first demand/grid event, constrained recovery, then a second event before full restoration. Publish reserve-restoration curves and repeated-shock survival by the actual scenario denominator.

### D. Use learning without learned authority

DR-SAC, LSTM and Symthaea can propose forecasts and schedules. The independent per-resource feasibility, recovery, lease and journal checks remain invariant. Publish shield intervention rate, false acceptance and the performance opportunity cost.

### E. Make energy boundaries explicit

Vehicle battery-terminal energy, aircraft battery-terminal energy, charger-input energy, stationary ESS energy and site-meter electricity are separate quantities. Never compare or sum them without measured conversion/efficiency and a declared boundary.

### F. Expand to adaptive multimodal service

The optimal service may be truck-only, fixed dock, mobile trailer, shared drone fleet, AMR, locker or human fallback depending on time, payload, weather, permissions, site energy and demand. The architecture selector should be allowed to choose not to fly.

## 9. Benchmark target ladder

Targets, not achieved results:

| Dimension | Target |
| --- | --- |
| Small exact instances | 100% feasibility agreement with independent enumeration |
| Hard reserve/permission violations | 0 false acceptance |
| Resource double-booking | 0 under all tested policies and competing offers |
| Unknown physical/commitment outcome | No automatic mutation or release; reconcile first |
| Recovery | 100% before next protected mission in admitted scenarios, otherwise explicit abstention |
| Repeated shocks | 100% survival of the frozen hard-case suite, with scenario denominator published |
| Paper reproduction | Reproduce source-native baseline objectives and constraints within declared tolerance before cross-paper comparison |
| Large-case policy regret | Mean ≤1%, p95 ≤5% against exact/MILP references where tractable |
| Runtime | Report p50/p95/p99 and timeout rate; bounded verifier target p99 <10 ms on declared hardware |
| Operational superiority | Improve the matched service-cost-energy Pareto frontier with confidence intervals, not an unsupported single percentage |

## 10. Evidence maturity and comparison admissibility

Every comparator row must be assigned exactly one primary maturity class:
- **Manufacturer specification:** declared operating envelope or controlled product test; useful for configuration comparison, not automatically independent field reliability.
- **Company-reported operation:** public operational or safety claim; preserve the company's stated population, timeframe, event definition and exclusions.
- **Peer-reviewed simulation/optimization:** reported result on a paper-native model and dataset; reproduce before transfer to another fleet, chemistry, geography or mission mix.
- **Independent field measurement:** calibrated, timestamped observations with source, asset identity, denominator, test protocol and uncertainty.
- **Luminous synthetic fixture:** deterministic arithmetic/control case only; never a field-performance claim.

A performance comparison is inadmissible when the numerator, denominator, hardware boundary, service definition, time horizon, battery type or operating environment differs without an explicit normalization model. Report unknown/missing data as unknown and publish the missing-data rate. Do not rank a vendor from a marketing reliability percentage with no exposed trial count.

## 11. Current South African boundary

The existing #6226 and #6330 evidence gates remain authoritative for aviation/operating applicability; the energy programme #6707 retains grid and SSEG boundaries. The base business case should start with permitted mobility service, behind-the-meter self-consumption, peak management and resilience. Do not include grid-export/market revenue merely because a model has residual kWh. IEC 62933-5-4:2026 is a stationary grid-integrated lithium-ion EES safety-test reference, not qualification of an aircraft propulsion battery for V2G. South African utility/grid, aviation, product-safety and site permissions remain separate.

## 12. Claim boundary

This benchmark defines a fair comparison and evidence ledger. It does not claim Luminous outperforms DJI, Zipline, Amazon, Wing, a mobile trailer or a published optimizer. It does not establish a South African commercial operating case, aircraft airworthiness, real battery life, grid-export permission or field reliability. Those claims require matched reproduction and independent measured evidence.


## 13. Executable v0.1: closed-loop service-capacity oracle

The benchmark now has a bounded Rust event simulator:

- `tools/ev-passenger-cost-frontier/src/sustainable_mobility_service_capacity.rs`
- `tools/ev-passenger-cost-frontier/src/bin/sustainable_mobility_service_capacity_screen.rs`
- `tools/ev-passenger-cost-frontier/schemas/sustainable_mobility_service_capacity_v0_1.schema.json`
- `tools/ev-passenger-cost-frontier/fixtures/sustainable_mobility_service_capacity_v0_1.json`

Run:

```bash
cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml --bin sustainable_mobility_service_capacity_screen -- \
  tools/ev-passenger-cost-frontier/fixtures/sustainable_mobility_service_capacity_v0_1.json
```

### v0.1 state machine

A sortie completion changes the resource state through this loop:

`installed pack -> depleted -> FCFS charging queue -> active charge channel -> ready spare -> aircraft turnaround -> next sortie`

An aircraft landing without a ready spare enters an explicit waiting queue. A completed charge is assigned to the oldest waiting aircraft, or becomes ready inventory if no aircraft is waiting. Charging channels are limited by:

[
N_{effective} = minleft(N_{channels},leftlfloor
rac{max(0,P_{site}-P_{fixed})}{P_{charger}}ightflooright)
]

The simulation uses deterministic event ordering: charge completions first, then mission completions, with a stable ID tie-break. Simultaneous events therefore cannot depend on hash/map iteration order.

### Frozen fixture

The fixture is an eight-hour synthetic comparison with four architectures:
1. a direct one-pack recharge baseline;
2. a Dock 4 specification-shaped profile with one aircraft, three ready spares, two charging channels, 45-second turnaround and 27-minute restoration;
3. the same profile with site headroom admitting only one 500 W modeled charger channel;
4. a two-aircraft shared pool with six ready spares, four nominal channels and three channels admitted by the synthetic site headroom.

The short mission durations, 500 W per-channel input proxy, site fixed loads and shared-pool compatibility are explicit synthetic stress assumptions. They are not vendor specifications. The manufacturer's 2 kW maximum Dock 4 AC input is not a per-channel charger rating; the simulation does not infer internal channel power or conversion efficiency from it.

Output separates:
- completed modeled sorties and cycles per hour;
- completed mission-hours;
- effective channels and modeled charging power;
- charge starts/completions and peak/current backlog;
- ready spares, active chargers and waiting aircraft at the horizon;
- aircraft wait seconds/share and maximum single wait;
- charger-input energy proxy and site fixed-load energy.

A sortie in this model is assumed to complete successfully once launched. The metric is therefore **modeled cyclic sortie capacity**, not successful on-time customer service, mission reliability or commercial throughput.

### Independent controls

The module includes 16 Rust test definitions. Among them:
- a single-pack/no-replenishment case is compared with the closed-form completion count (1+lfloor(H-T_m)/(T_m+T_o)floor);
- completed sorties are bounded by initial battery packs plus successfully completed recharges;
- battery-pool conservation is checked at the horizon;
- site power limits the effective channel count;
- no grid headroom cannot fabricate charge completions;
- increased inventory must reduce waiting in the frozen short-cycle stress case;
- an event-count ceiling rejects pathological horizons/event rates;
- architecture input permutation produces the same canonical output.

The exact-head Cargo/format/report workflow is wired in CI. No pass is claimed until the latest exact head completes. This remains a fixed-parameter event model; it is not yet a charge-curve, stochastic demand, weather/permission, lifecycle-cost or mobile replenishment simulator.

## 14. Current research comparison: what is new, and what is not

The current literature reinforces that we should not claim novelty for battery swapping, multi-visit sorties, mobile trailers, fleet sizing or routing in isolation:

- Xie, Zhang and Jia's 2026 *Drones* paper jointly sizes multi-truck/multi-drone fleets and routes; its ablations identify multi-visit delivery as a major fleet/cost mechanism, with cross-vehicle retrieval adding flexibility. [DOI 10.3390/drones10090658](https://doi.org/10.3390/drones10090658).
- Liang, Chen and Du's EJOR paper (online 15 October 2026) combines release dates, time windows, pickup/delivery, lockers and drone resupply using a mixed-integer bilinear model and a tailored metaheuristic. [DOI 10.1016/j.ejor.2026.10.020](https://doi.org/10.1016/j.ejor.2026.10.020).
- Tran et al.'s November 2026 *Transportation Research Part E* article jointly models truck routing and multi-rendezvous drone resupply and reports its adaptive tabu search outperforming CPLEX by 7.39% on its own instances. The figure is paper-specific, not a cross-domain claim. [DOI 10.1016/j.tre.2026.105082](https://doi.org/10.1016/j.tre.2026.105082).
- Liu, Zhu and Zhang's November 2026 *Transportation Research Part E* paper includes heterogeneous drones, multi-visit sorties, en-route launch/recovery, energy dynamics and an RL-guided ALNS. [DOI 10.1016/j.tre.2026.105114](https://doi.org/10.1016/j.tre.2026.105114).

The current v0.1 differentiator is modest and precise: a bounded, reproducible, event-driven capacity composition of finite pack inventory, charge channels, site import headroom and aircraft turnaround, with explicit end-of-horizon backlog and waiting. It does not replace the route optimizers or specialized battery/charge schedulers in #6247/#6674/#6705.

The larger research opportunity is to connect those layers on a matched multi-day workload, then test whether the architecture/policy can deliver more **qualified on-time mission-hours per lifecycle dollar and per metered kWh** without violating reserve, permission, inventory or recovery constraints.

## 15. Paradigm-shift research roadmap

### A. From component-speed optimization to service-flow optimization

Treat the depot as a production network whose output is verified useful service. Flight, turnaround, charge, inspection, cooling, maintenance, repositioning and permission windows are process stages competing for resources. Optimize the bottleneck of the entire loop, not whichever local stage has the most impressive vendor number.

### B. From fixed architecture to adaptive service topology

Allow the exact same demand set to be served by fixed dock, mobile station, truck-only, shared UAV/AMR, locker or human fallback. Choose not to fly when grid, weather, permissions, payload or reserve make aerial service inferior or unavailable. Mobile and mid-air replenishment must pay for donor assets, route/rendezvous time, payload and energy.

### C. From nominal packs to time-indexed, qualified energy inventory

A pack is not simply `available` or `empty`. Its compatibility, location, state-of-charge, peak power, temperature, health, charge protocol, custody and return-to-service evidence all affect whether it can serve the next mission. Compose the event simulator with #6674/#6705 rather than creating a second battery registry or charger scheduler.

### D. From average throughput to tail-risk and resilience

Use empirical demand/weather traces and correlated outage/charger failures. Report p50/p95/p99 queue waits, unmet mission demand, recovery curves and repeated-shock service survival. Mean utilization is not a resilience metric. Do not release unresolved battery or service promises to improve average throughput.

### E. From learned dispatch to learned proposals under a fixed assurance envelope

Compare greedy, MPC, exact MILP/DP, robust/chance-constrained, ALNS/ATS and DR-SAC/Symthaea proposal policies. Every policy must use the same #6803/#6809/#6829/#6837 hard gates. Report rejected unsafe proposals and the economic/service opportunity cost of conservative gating, separately from any policy reward.

## 16. Target ladder (not achieved performance)

| Target | Benchmark rule |
| --- | --- |
| Event model | 100% agreement with an independent discrete-event oracle on bounded small cases |
| Battery accounting | Zero pack double-allocation; pool conservation at every event cut |
| Charger physics abstraction | Zero overlapping jobs per channel; active channels never exceed grid headroom |
| Sustainable throughput | Report cycles/hour, useful mission-hours and per-aircraft-hour denominator |
| Service quality extension | p50/p95/p99 queue and end-to-end request latency; unserved demand reported explicitly |
| Hard assurance | Zero false acceptance of reserve, permission, compatibility or double-booking violations |
| Robustness | Frozen repeated-shock tests; 100% survival of admitted hard cases or explicit abstention |
| Optimization quality | Mean regret ≤1%, p95 ≤5% against exact/MILP references where tractable |
| Runtime | p99 <10 ms for bounded evaluator on declared hardware; end-to-end optimizer latency reported separately |
| Commercial superiority | Pareto improvement on matched service/cost/energy with confidence intervals and independent measured denominators |

These are targets. A passing synthetic evaluator test does not establish any of the service, reliability or economics targets.

## 17. Source anchors

- [DJI Dock 4 official specs](https://enterprise.dji.com/dock-4/specs): swap <25 s; operational turnaround <45 s; three maximum stored batteries; two parallel charging channels; 27-minute 15–95% recharge at 25°C; maximum dock AC input 2 kW; one aircraft per dock.
- [DJI product/support details](https://www.dji.com/support/product/dock-4): controlled-test definitions and reference-only caveats for swap and turnaround.
- [Xie et al. 2026](https://doi.org/10.3390/drones10090658): multi-truck/multi-drone fleet sizing/routing.
- [Liang et al. 2026](https://doi.org/10.1016/j.ejor.2026.10.020): multi-visit, release-date/time-window pickup/delivery and drone resupply.
- [Tran et al. 2026](https://doi.org/10.1016/j.tre.2026.105082): multi-rendezvous truck-drone resupply.
- [Liu et al. 2026](https://doi.org/10.1016/j.tre.2026.105114): heterogeneous en-route drone operations.

## 18. Exact-head v0.1 verification and frozen fixture result

Exact-head Actions run [#45066300699](https://github.com/Luminous-Dynamics/symtropy/actions/runs/45066300699) at `f043d97e68f2ae84ef303f2eb09bbf26d97d2c22` passed:
- strict schema/fixture JSON parsing;
- package workspace `cargo fmt --check`;
- `cargo test`;
- deterministic report assertions;
- report artifact upload.

The test binaries reported 154 + 16 + 8 + 9 = **187 passed, 0 failed**. The service-capacity module itself contains 16 focused Rust tests. Clippy is not part of this workflow and remains unverified.

The eight-hour frozen synthetic fixture produced:

| Architecture profile | Effective channels | Completed modeled sorties | Sorties / wall-clock hour |
| --- | ---: | ---: | ---: |
| Direct single-pack recharge | 1 | 11 | 1.375 |
| Dock 4-shaped, 3 spares / 2 channels | 2 | 38 | 4.750 |
| Same profile, site-limited to 1 channel | 1 | 21 | 2.625 |
| Two-aircraft shared pool, site-limited to 3 channels | 3 | 58 | 7.250 |

These figures are deterministic fixture outputs from the supplied parameters. They are not DJI performance, observed aircraft uptime, delivered customer requests or successful on-time missions. The simulator assumes every sortie completes and has no stochastic demand, weather, permission, maintenance or mission-failure process. The four architecture profiles also differ in aircraft count and mission duration; they are not a normalized leaderboard. A fair comparative claim requires a matched workload and capital/resource budget across all candidates.

