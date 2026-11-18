# Grid Asset Recovery — v0.1 research benchmark

**Owner:** [GRID-ASSET-RECOVERY-001, Symtropy #6119](https://github.com/Luminous-Dynamics/symtropy/issues/6119)  
**Status:** Synthetic contract and benchmark protocol. No operational spare policy, OEM interchangeability, physical repair rate, or grid reliability improvement is claimed.  
**Snapshot date:** 2026-11-18

## 1. Research thesis

A transmission asset is not resilient merely because it satisfies a static contingency criterion or has a low expected failure rate. The system must also be able to **restore the lost service** when it fails.

The recovery system is:

`failure → detect/isolate → diagnose → obtain compatible part → transport/access → mobilise qualified resources → repair/replace → test/commission → restore service`

Time-to-restored-service depends on the whole recovery chain. A spare sitting in a depot is not useful if it is electrically or mechanically incompatible, already reserved, unserviceable, inaccessible, too heavy for the available route, or cannot be installed by an available qualified crew.

This benchmark therefore treats inventory, compatibility, logistics, specialist resources and commissioning as coupled constraints.

## 2. Relationship to existing work

- [#6067 transmission portfolio](https://github.com/Luminous-Dynamics/symtropy/issues/6067): chooses carrier, route and architecture; consumes repairability outputs.
- [#6080 firm capacity](https://github.com/Luminous-Dynamics/symtropy/issues/6080): defines secure delivered capacity and bottleneck migration; supplies service and contingency boundaries.
- [#6105 global portability](https://github.com/Luminous-Dynamics/symtropy/issues/6105): supplies regional hazard, jurisdiction and cost context.
- [#6106 transmission industrialisation](https://github.com/Luminous-Dynamics/symtropy/issues/6106): owns design-for-manufacture, supplier/factory development and build throughput; this benchmark consumes qualified part and supplier data.
- [#3418 power-system resilience](https://github.com/Luminous-Dynamics/symtropy/issues/3418): owns grid-forming, islanding, black-start and protection-system qualification.
- [#4043 built-asset robot loop](https://github.com/Luminous-Dynamics/symtropy/issues/4043): may provide inspected/as-built/repair task evidence.

This work owns fleet-level part allocation, recovery scheduling, spare contention and time-to-restored-service benchmarking. It does not create a second dispatch, protection, metrology, supplier-qualification or asset-digital-twin authority.

## 3. Industry baseline

The IEA's *Building the Future Transmission Grid* (25 February 2025) reported, from its 2024 industry survey, procurement lead times of 2–3 years for cables and up to 4 years for large power transformers; some HVDC cable waits exceeded 5 years. The IEA also identifies concentrated component supply and standardisation/procurement coordination as important levers. These figures are industry-level survey findings, not universal current quotes.

The US DOE Office of Electricity page updated 29 October 2026 reports US distribution-transformer lead times increased from 3–6 months in 2019 to 12–30 months in 2023, and identifies more than 80,000 transformer varieties in the US as a specification-fragmentation concern. That is a **distribution-transformer / US** data point; do not transfer it to large power transformers or other jurisdictions.

CIGRE's 10 September 2026 discussion of transformer overload capability highlights that additional utilisation has thermal and life limits. Overload capability is not a substitute for a qualified replacement strategy.

NERC reliability assessment material tracks element availability, outage severity, protection and equipment-related outage indicators. This benchmark complements those indicators with recovery-chain duration and service consequence; it does not replace them.

Primary references:
- IEA: https://www.iea.org/reports/building-the-future-transmission-grid/executive-summary
- US DOE: https://www.energy.gov/oe/supply-chain-and-market-analysis
- CIGRE: https://www.cigre.org/article/GB/news/the_latest_news/a-grid-that-must-do-more-possibilities-limits-and-risks-for-increasing-transformer-overload-capability
- NERC: https://www.nerc.com/programs/reliability-assessment--performance-analysis

## 4. Paradigm shift: from static N-1 to restorative capability

Treat resilience as a **portfolio of failure prevention, service substitution, qualified repair and replenishment**, not a single N-1 checkbox.

The benchmark should compare:
1. no-pooling / procurement-only;
2. local strategic stock;
3. regional shared stock;
4. dual-sourced parts with controlled configuration;
5. modular replacement only where interfaces and full qualification support it;
6. temporary service alternatives (bounded by their own power, energy, duration and availability).

Do not assume part interchangeability from nameplate voltage, MVA, geometry or vendor assertion. A valid compatibility decision may require mechanical envelope, electrical ratings, insulation, protection, cooling, terminals, firmware/control, environmental limits and acceptance-test equivalence.

## 5. Model and claim boundaries

The first version is intentionally a hand-calculable **sequential** recovery model. Each event has ordered tasks. The sum is the restoration time. Later work may add dependency graphs and parallel critical-path scheduling, but must not silently add all task times when they overlap.

A depot-stock plan consumes inventory. An OEM-procurement plan records a new supply path and does not consume depot stock. A part is usable only for an explicitly listed interface profile with matching evidence. Synthetic compatibility is allowed only to test the contract; it is not evidence that real equipment is interchangeable.

The Rust validator checks:
- unique IDs and all references;
- part/interface compatibility and evidence maturity;
- available inventory after reservations and exclusive spare allocation;
- logistics origin, destination, transit duration and cargo capacity;
- resource assignment overlap and unit capacity;
- complete recovery stages;
- claimed restoration time versus task arithmetic;
- net unserved energy arithmetic from lost service, alternate capacity and restoration hours;
- no promotion of synthetic evidence to a field or independent evidence class.

## 6. Benchmark metrics

Publish a Pareto vector, not one blended resilience score:

| Metric | Definition / reporting rule |
|---|---|
| Time to restored service | Event start to completed test/commissioning; report stages separately |
| Tail restoration | p50/p90/p95/p99 only when an evidence-backed distribution exists |
| Firm shortfall | Lost firm MW less explicitly available alternate MW, floored at zero |
| Unserved energy | Integrate shortfall over the restoration profile; v0.1 uses constant shortfall |
| Spare coverage | Serviceable unreserved units divided by declared critical demand |
| Qualified substitute coverage | Demand profiles with evidence-qualified substitutes / total profiles |
| Stockout probability | Scenario or empirical estimate with stated event and replenishment assumptions |
| Crew contention | Resource-hour demand, utilisation and overlapping unmet demand |
| Logistics resilience | Transit-time distribution, route capacity, access/weather/customs constraints |
| Lifecycle resilience cost | Inventory, storage, obsolescence, replenishment, logistics, repair and outage consequence |
| Common-cause exposure | Shared supplier, depot, route, utility, communications, weather or cyber dependencies |

Compare strategies under matched asset criticality, failure severity, service boundary, geography, contingency policy, time horizon and uncertainty. Keep observed events, field measurements, OEM statements, engineering estimates and synthetic fixtures in distinct evidence classes.

## 7. Initial archetypes and adversarial suite

Start with a synthetic two-region transformer fleet and then extend to HVDC converter equipment, underground/submarine cable repair, overhead line components, GIS/circuit breakers and remote mine/island corridors.

Frozen adversarial mutations must reject:
- nominally similar but unqualified substitute part;
- one physical spare allocated to two separate failures;
- reserved or unserviceable inventory;
- impossible transport mass or wrong depot-to-site route;
- two overlapping repairs booked against one specialist crew;
- missing final test/commissioning;
- a claim that a synthetic fixture is field-observed or independently verified.

Add correlated failures and route disruption before making any probabilistic stock optimisation claims.

## 8. Proposed internal targets — not achieved results

After a frozen baseline exists:
- zero false acceptance in the compatibility, inventory, resource, schedule and evidence negative-control suite;
- 100% of critical recovery stages linked to a task, owner, source and duration distribution—or explicitly marked unknown;
- deterministic replay of frozen cases;
- zero double allocation of physical spare units or specialist resource capacity;
- target at least 20% reduction in p95 time-to-restored-service on one independently reviewed archetype versus a matched no-pooling/current-policy baseline, without increasing safety or quality risk;
- improve lifecycle resilience cost or make any necessary resilience premium explicit;
- maintain performance under correlated failures and tail logistics delays, not just average MTTR.

These are internal research objectives. They are not published SOTA values, guarantees, acceptance criteria or Luminous achievements.

## 9. Next increments

1. Add task dependency graphs and resource-constrained critical paths.
2. Add distributions for failure arrival, supplier lead time, transport, repair and commissioning.
3. Use event simulation with reproducible seeds and common-random-number comparisons across strategies.
4. Add shared-spare contention across correlated failures and regional pools.
5. Include storage/obsolescence/inspection and repairable-component loops.
6. Add local cost/FX/price-year and outage-cost sensitivities.
7. Integrate with #6080's firm-capacity output and #6067 route portfolio.
8. Obtain TSO, OEM, cable-repair, logistics and maintenance review before introducing real asset or outage data.

**Safety boundary:** This benchmark is research and decision support only. It does not authorize switching, protection, overload, energisation, substitution, lifting, repair or return to service. Asset owners, qualified engineers, OEMs and competent authorities retain those responsibilities.


## 10. Deterministic dependency and resource scheduler (v0.2 increment)

Each task now has:
- a stable `taskId`;
- a `dependsOn` list of predecessor task IDs within the same outage event;
- duration;
- optional renewable resource ID and required units.

The scheduler performs non-preemptive, deterministic list scheduling across **all events in the case**, subject to precedence and resource capacity. Independent activities can overlap. When a crew or tool is unavailable, the next task is moved to the earliest feasible time slot. The reported event restoration time is the last scheduled task completion minus the event start; it is not the sum of all task durations.

This is a transparent greedy baseline. The semantic validator also enforces stage-specific engineering precedence: diagnosis follows detection/isolation; parts sourcing and crew mobilisation follow diagnosis; transport follows parts sourcing; repair waits for the required part, transport (for depot stock) and crew; commissioning follows repair.

The CLI emits a machine-readable JSON schedule report with task intervals, event restoration spans, firm shortfall and calculated unserved energy. It labels itself as a heuristic and does not claim optimality.

1. Find tasks whose predecessors have completed.
2. Calculate each task's earliest precedence-ready time.
3. Find the earliest interval where its required resource capacity is available.
4. Select the candidate with the earliest feasible start, with stable event/task-ID tie-breaking.
5. Repeat until all tasks are scheduled or a dependency cycle / invalid resource is found.

**This algorithm is not an optimal RCPSP solver.** It is intended to provide a deterministic, auditable baseline before introducing CP-SAT, MILP or specialised metaheuristics. A future optimiser must be compared against this baseline on identical frozen inputs and must prove every returned schedule feasible.

### Synthetic fixture schedule

The three-event fixture now represents parallel parts sourcing/logistics and crew mobilisation. With the current synthetic durations and resource capacities, it declares restoration spans of 32 h, 1,219 h and 23 h. These are calculated from the task dependency graphs and the declared single-crew capacity; they are not observed transformer restoration times.

The second event's long OEM procurement duration is deliberately a stress case. It is a synthetic value used to exercise the schedule and must not be interpreted as a current supplier quote.

### Scheduler benchmark plan

A fair algorithm comparison should include:
- deterministic serial schedule generation schemes (SSGS) with multiple priority rules;
- the current earliest-feasible-start list scheduler;
- CP-SAT with interval and cumulative-resource constraints;
- MILP / branch-and-bound where the instance size supports exact or bounded-gap solutions;
- optionally, metaheuristics after exact and lower-bound baselines are established.

Use standard resource-constrained project scheduling corpora such as PSPLIB and more recent RCPLIB / MMLIB variants to verify generic scheduling correctness and scaling. They do not contain transmission-specific repair semantics, so they are **algorithmic scheduling baselines**, not grid asset benchmarks. Augment them with frozen transmission recovery archetypes covering crew skills, spare compatibility, transport, permits, weather windows, commissioning and correlated events.

Report:
- feasibility rate and zero false acceptance on adversarial cases;
- event/task interval output, resource waiting time and restored-service critical path;
- makespan gap to proven optimum / best-known bound where available;
- lower bound, upper bound and optimality gap separately;
- p50/p95 runtime, memory, scaling, solver status and determinism;
- sensitivity to task priority, tie-breaking, resource availability and delayed release;
- schedule stability and service consequence, not just makespan.

### External scheduling references

- PSPLIB / RCPSP research and benchmark data: https://www.projectmanagement.ugent.be/research/project_scheduling/rcpsp
- RCPSP benchmark dataset overview: https://www.tandfonline.com/doi/full/10.1080/00207543.2026.2683116
- Google OR-Tools scheduling recipes for interval variables and cumulative resource constraints: https://github.com/google/or-tools/blob/stable/ortools/sat/docs/scheduling.md

## 11. Industry comparison: outage indicators versus restoration-chain evidence

NERC's 2026 State of Reliability reports 2025 transformer unavailability of 0.35% for transformers at 200 kV and above in its reporting area. NERC's Transmission Availability Data System records outage frequency, duration and causes. The asset-recovery benchmark is complementary: it adds stage-level repair, inventory, transport, crew and commissioning evidence to a system-level availability picture. It must not claim to replace or directly outperform NERC's reliability indicators.

South African context is especially important. NTCSA's April 2026 winter outlook briefing reports 36–48 month lead times for certain transformer classes, a single supplier for Class 3 transformers that was not then producing, and no local Class 4 supplier. It also reports 23 pre-qualified international factories. These details justify region-specific supplier and spares scenarios; they do not prove every transformer class or project has the same lead time.

For submarine cable restoration, the ICPC's August 2026 repair-commencement comparison offers a method for separating notification-to-departure delays from transit and repair. Its core dataset covers submarine telecommunications cable repairs, not power-cable repairs. Do not transfer its repair-time statistics directly to HVDC export cables; adapt the measurement method and obtain power-cable-specific evidence.

Primary references:
- NERC 2026 State of Reliability: https://prod.nerc.com/globalassets/programs/reliability-assessment--performance-analysis/nerc_sor_2026_technical_assessment.pdf
- NERC TADS: https://prod.nerc.com/programs/reliability-assessment--performance-analysis/transmission-availability-data-system
- NTCSA / Eskom Winter Outlook Briefing April 2026: https://www.eskom.co.za/wp-content/uploads/2026/04/State-of-the-System-%E2%80%93-Winter-Outlook-Briefing-2026.pdf
- ICPC Global Comparison of Repair Commencement Times (updated 28 August 2026): https://www.iscpc.org/publications/icpc-global-comparison-of-repair-commencement-times/
- CIGRE transformer overload capability discussion (10 September 2026): https://www.cigre.org/article/GB/news/the_latest_news/a-grid-that-must-do-more-possibilities-limits-and-risks-for-increasing-transformer-overload-capability

## 12. Next SOTA-facing increments

1. Add calendar-aware task windows, working shifts, site access and travel/permit constraints; continuous elapsed time is only the v0.2 abstraction.
2. Add crew skill matrices, multiple crews, specialist subcontractors and equipment resources as separate renewable resource classes.
3. Model stochastic repair durations and lead times with common random numbers so strategy comparisons use identical event scenarios.
4. Add stock policy alternatives (local stock, regional pool, dual sourcing, repairable exchange pool, qualified modular replacement) under one frozen demand/failure sequence.
5. Add failure dependency / common-cause scenarios across depots, factories, routes, communications and weather.
6. Compare the heuristic to CP-SAT / MILP on small cases with proven optima, then scale through larger PSPLIB/RCPLIB scheduling corpora and transmission-specific cases.
7. Feed scenario restoration and firm-shortfall outputs to #6080 without giving the scheduling engine dispatch or operational authority.
