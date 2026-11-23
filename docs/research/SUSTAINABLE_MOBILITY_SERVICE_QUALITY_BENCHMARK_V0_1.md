# Sustainable Mobility Service Quality Benchmark v0.1

**Programme:** [SUSTAINABLE-MOBILITY-SERVICE-CAPACITY-001 #6909](https://github.com/Luminous-Dynamics/symtropy/issues/6909)  
**Service-quality child:** [#6921](https://github.com/Luminous-Dynamics/symtropy/issues/6921)  
**Industry baseline:** [#6226](https://github.com/Luminous-Dynamics/symtropy/issues/6226)  
**Matched SOTA tournament:** [#6247](https://github.com/Luminous-Dynamics/symtropy/issues/6247)  
**Branch:** \`research/sustainable-mobility-service-quality-v0-1\`  
**Status:** deterministic request-to-service-opportunity evaluator, strict schema, frozen request fixture, CLI and 14 Rust test definitions committed; exact-head CI pending.

## 1. Why v0.2 is a separate problem

The v0.1 service-capacity model answers: *how many cyclic mission opportunities can a given aircraft/pack/charger/site configuration provide over a declared horizon?*

That does not answer: *which requests should receive those scarce opportunities, which deadlines will be met, and what should happen when weather, permission or maintenance closes the aerial option?*

A system can have adequate total capacity and still deliver poor service because capacity occurs at the wrong time. It can also show high sortie counts while rejecting high-value urgent requests, missing deadlines, or operating on a stale permission assumption.

The next unit of analysis is therefore the **request outcome**, with explicit states:
- offered;
- rejected at the acceptance cutoff;
- accepted by the service system;
- completed on time;
- completed late;
- unserved;
- fulfilled by a separately represented fallback mode.

This evaluator uses prequalified time-bounded service opportunities. It does not create routes, infer aircraft permissions, or rebuild the battery/charger simulator.

## 2. Paradigm shift: capacity is an option with a deadline, not a throughput number

The architecture composes two layers:

1. **Capacity supply (#6909):** a frozen upstream capacity report sets the horizon and maximum aerial service-opportunity budget. Each opportunity is a time-bounded token tied to an aircraft and unique capacity reference. The source digest and architecture ID travel with the request-quality result.
2. **Request demand (#6921):** requests arrive over time with an acceptance cutoff, service duration, payload, service class, mode eligibility and completion deadline. FCFS or EDF chooses among requests already arrived when an opportunity begins.

An aerial service opportunity is usable only when:
- the request allows aerial service;
- its payload and service duration fit the opportunity;
- the operation starts no later than the request deadline;
- one flight window covers the mission;
- weather is \`go\`, permission is \`approved\`, and the evidence is current at the decision time;
- no maintenance/site-closed/no-go interval overlaps the mission and turnaround occupancy;
- the aircraft and its capacity reference are not already double-booked.

If any of those conditions is unknown, stale or blocked, aerial service is unavailable. A request may use a fallback opportunity only when its declared mode allows it. The evaluator never silently converts an aerial-only request into a ground service.

FCFS and EDF use the exact same request stream, opportunities, constraints and information cut. The only difference is the ordering of currently eligible requests:
- FCFS: earliest arrival, then deadline, then stable request ID;
- EDF: earliest deadline, then arrival, then stable request ID.

A service starts only if the opportunity start is no later than the request deadline. Completion may still be late when mission duration runs past the deadline; late completion is counted separately from unserved work.

## 3. Executable v0.1

Files:
- \`tools/ev-passenger-cost-frontier/src/sustainable_mobility_service_quality.rs\`
- \`tools/ev-passenger-cost-frontier/src/bin/sustainable_mobility_service_quality_screen.rs\`
- \`tools/ev-passenger-cost-frontier/schemas/sustainable_mobility_service_quality_v0_1.schema.json\`
- \`tools/ev-passenger-cost-frontier/fixtures/sustainable_mobility_service_quality_v0_1.json\`

Run:

\`\`\`bash
cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml --bin sustainable_mobility_service_quality_screen -- \\
  tools/ev-passenger-cost-frontier/fixtures/sustainable_mobility_service_quality_v0_1.json
\`\`\`

The frozen eight-hour workload contains 11 requests, six aerial opportunities, six fallback opportunities and five evidence-bound flight windows. It includes:
- early requests with conflicting arrival/deadline priorities;
- an aerial-only request during a weather no-go interval;
- an aerial-only request overlapping maintenance;
- a request that can fall back when its aerial option is unavailable;
- an unknown-permission interval;
- one request arriving after its acceptance cutoff;
- one on-time start that finishes after the deadline and must be counted late;
- one request with no service opportunity before its deadline.

The capacity envelope is explicitly tied to the #6909 Dock-shaped synthetic profile:
- 8-hour horizon;
- 4.444444 cycles/hour steady-state replenishment ceiling;
- 0.5 initial-inventory allowance per hour;
- at most 39 aerial opportunities over the horizon.

The service-quality module checks that the number of supplied aerial opportunities does not exceed the upstream declared envelope. It does not authenticate the report digest or independently re-run the pack/charger simulation. Opportunity schedules are supplied inputs and must be generated from a separately qualified capacity result before any stronger claim is made.

### Reporting

Each policy returns:
- offered, accepted and rejected request counts;
- on-time, late and unserved counts;
- aerial vs fallback fulfilment;
- available, blocked and unused aerial opportunities;
- unused fallback opportunities;
- on-time rate of all offered requests and of accepted requests;
- late and unserved rates of accepted requests;
- p50/p95/p99 latency, mean latency, mean queue wait and maximum queue wait;
- per-request disposition, opportunity, service mode, start/completion, latency, wait and denial reason.

Percentile values use nearest-rank order statistics. No interpolation or statistical confidence interval is claimed from this synthetic fixture.

## 4. Industry anchor: DJI Dock 4

DJI's official Dock 4 specification lists:
- battery swap time below 25 seconds;
- landing-to-next-takeoff operational turnaround below 45 seconds;
- at most three stored batteries across the battery compartments;
- two parallel charging channels;
- 27 minutes to charge a compartment battery from 15% to 95% at 25°C;
- maximum AC input power of 2,000 W.

The <25-second measure is the mechanical swap interval defined by DJI from cover closure to swap completion. The <45-second figure is the landing-to-next-takeoff turnaround. The 27-minute charge interval is a separate replenishment metric. The 2 kW maximum dock input is not a published per-channel charger rating.

Sources:
- [DJI Dock 4 official specifications](https://enterprise.dji.com/dock-4/specs)
- [DJI Dock 4 support specifications and FAQ](https://www.dji.com/support/product/dock-4)

The #6909 v0.1 four-architecture fixture uses a synthetic 500 W/channel input proxy, shared pack assumptions and five-minute missions. It is intentionally described as a **Dock-shaped** profile, not a reproduction of DJI Dock 4. This service-quality fixture then consumes a declared 39-opportunity upstream envelope and adds request arrivals/deadlines; it does not claim to measure DJI's actual customer-service rate.

A strong industry comparison must not rank products by swap time alone. Report six distinct denominators:
1. mechanical exchanges attempted/completed;
2. landing-to-takeoff turnaround completed;
3. charge restoration completed;
4. available qualified aircraft-hours;
5. completed on-time customer requests;
6. network-level request service over the operating day.

## 5. 2026 research comparison

### 5.1 Dynamic request acceptance under wind

Zhao, Zhang and Coelho, *Transportation Research Part E* 214 (October 2026), article 105074, formulate dynamic truck–drone collaboration with stochastic customer arrivals and changing weather. Their method uses a route-based MDP and approximate dynamic programming with value-function approximation; the abstract identifies wind-aware energy and ground-speed feasibility and reports instances approaching 300 requests.

This is the closest request-admission baseline for #6921. It is not a direct UAV battery-depot performance result. Reproduce the paper's original request stream, one-truck/one-drone configuration, route assumptions, wind model and benchmark policies first. Then add the shared battery-service network as a matched extension.

Source: [DOI 10.1016/j.tre.2026.105074](https://doi.org/10.1016/j.tre.2026.105074).

### 5.2 Strategic mode switching under uncertain demand

Lu et al., *Transportation Research Part C* 191 (October 2026), article 105807, formulate real-options decisions for switching between truck-only and drone-assisted delivery. The article preview reports 17.4% deterministic and 31.3% stochastic cost reductions versus immediate cost-based switching on the paper's own instances.

This is a distinct strategic problem. #6153 owns mode activation/withdrawal. The request evaluator here does not acquire or reposition a fleet; it compares FCFS/EDF service admission within already-declared service opportunities.

Source: [DOI 10.1016/j.trc.2026.105807](https://doi.org/10.1016/j.trc.2026.105807).

### 5.3 Multi-visit pickup/delivery and resupply

Liang, Chen and Du, *European Journal of Operational Research* (available online 15 October 2026), DOI 10.1016/j.ejor.2026.10.020, combine release dates, time windows, simultaneous pickup/delivery, lockers and a distinct drone-resupply fleet. Their MIBLP and ALNS results show that resupply and release-time coordination change the feasible service architecture.

This motivates retaining mode eligibility and fallback as explicit request attributes. However, this v0.1 evaluator does not model truck routes, resupply sorties or rendezvous.

Source: [DOI 10.1016/j.ejor.2026.10.020](https://doi.org/10.1016/j.ejor.2026.10.020).

### 5.4 Mobile battery replenishment under uncertainty

Xu et al., *Transportation Research Part D* 158 (September 2026), article 105458, model mobile battery-swapping vans returning to charging stations to reload. Their three-level stochastic framework jointly sizes vans, racks and battery inventory and allocates service under demand/charging uncertainty.

The next network-level extension should connect a service opportunity to the replenishment vehicle's travel/setup/crew and loaded-pack state, while leaving #6674/#6705 responsible for battery inventory and charge queues. This v0.1 fallback is a prequalified time slot, not a mobile-van routing model.

Source: [DOI 10.1016/j.trd.2026.105458](https://doi.org/10.1016/j.trd.2026.105458).

## 6. Benchmark tournament and targets

These are targets, not achieved comparisons.

### Policy baselines

Run every policy on the same frozen streams:
- FCFS;
- EDF;
- shortest processing time;
- minimum-deadline-slack;
- rolling-horizon MPC with only contemporaneously available forecast vintages;
- exact exhaustive/DP oracle on very small instances;
- reproduced MDP/ADP baseline from Zhao et al.;
- robust/chance-constrained admission after matching their uncertainty sets;
- Symthaea forecast/policy proposals behind the same independent constraints.

#6921 owns the service-request tournament. #6153 retains strategic service-mode choice, #6177 shared fleet/repositioning, #6083 sortie/carrier co-availability, #6147 dynamic cargo/release/resupply, #6909 cyclic service capacity and #6674/#6705 battery inventory/charging. This module does not replace those owners.

### Required metrics

Always publish numerator and denominator:
- accepted / offered;
- on-time / offered;
- on-time / accepted;
- late / accepted;
- unserved / accepted;
- aerial / completed;
- fallback / completed;
- p50/p95/p99 arrival-to-completion latency;
- aircraft opportunity utilization and blocked/unused opportunity rate;
- fallback opportunity utilization;
- denial reasons by count;
- energy and cost only when their boundary and source are specified.

Targets:
- zero aerial admission on unknown/denied/stale permission or weather;
- zero use of future observations;
- zero aircraft/opportunity overlap or capacity-reference reuse;
- zero violation of the upstream #6909 opportunity budget;
- 100% deterministic output under permutations of independent input records;
- 100% agreement with an exact assignment oracle on tiny bounded request sets;
- zero request silently changing from aerial-only to fallback;
- report accepted/on-time/late/unserved counts without double-counting;
- target p99 <10 ms for the bounded evaluator on declared hardware; report end-to-end optimization latency separately;
- on held-out streams, target lower p95 latency and/or higher on-time rate than FCFS/EDF without increasing unsafe aerial admissions or consuming additional protected capacity.

Do not report a single blended score that hides late/unserved requests or unsafe aerial proposals.

## 7. Paradigm shifts for the next versions

1. **Capacity contracts rather than average throughput.** #6909 should emit or reference time-bounded capacity opportunities with unique aircraft/pack/channel provenance. A request evaluator should consume those opportunities rather than independently inventing service capacity.
2. **Online recourse rather than static schedules.** Only requests and weather/permission vintages available at the decision time can influence admission. Later observations may trigger a later decision, not rewrite the past.
3. **Fallback as a first-class capability.** Truck, locker, AMR and human service are explicit alternatives with their own capacity, time and eligibility. Fallback is not a hidden rescue assumption.
4. **Reserve-aware demand rejection.** An urgent request must not displace an already committed mission or consume recovery margin; #6803/#6809/#6829/#6837 remain upstream hard gates.
5. **Separate policy quality from physical truth.** A synthetic request marked completed is a deterministic model outcome, not a measured delivery success. Field reliability and cost require independently measured evidence and denominator discipline.

## 8. Claim boundary

The v0.1 request evaluator is a deterministic assignment model over supplied service opportunities and declared weather/permission evidence. It does not authenticate the evidence, model actual wind physics, estimate demand, solve routes, simulate battery charging, or authorize dispatch. The fixture's opportunities, service durations and fallback slots are synthetic. No field performance, regulatory eligibility, commercial savings, or SOTA superiority is claimed.
