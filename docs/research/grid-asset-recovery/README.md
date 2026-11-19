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
- zero or more simultaneous renewable resource requirements, each identifying a resource class and required units.

Every resource also declares a repeating calendar period and one or more availability windows within that period. The scheduler only starts a task when **all** required resource classes have enough capacity and the complete non-preemptive task duration fits inside an available window for every resource. The fixture uses a synthetic 24-hour period with a 08:00–22:00 daily window for the qualified crew, crane, test rig and heavy-haul vehicle.

A task requiring a crane, qualified crew and test rig reserves all three for the same interval. A task too long to fit in every shift window is rejected rather than silently split across shifts. The first calendar model is deliberately simple: periodic windows on a shared elapsed-hour axis, without public holidays, shift exceptions, time zones, daylight-saving transitions, travel-time calendars or preemptive work.

The scheduler performs non-preemptive, deterministic list scheduling across **all events in the case**, subject to precedence and resource capacity. Independent activities can overlap. When a crew or tool is unavailable, the next task is moved to the earliest feasible time slot. The reported event restoration time is the last scheduled task completion minus the event start; it is not the sum of all task durations.

This is a transparent greedy baseline. The semantic validator also enforces stage-specific engineering precedence: diagnosis follows detection/isolation; parts sourcing and crew mobilisation follow diagnosis; transport follows parts sourcing; repair waits for the required part, transport (for depot stock) and crew; commissioning follows repair.

The CLI emits a machine-readable JSON schedule report with task intervals, event restoration spans, firm shortfall and calculated unserved energy. It labels itself as a heuristic and does not claim optimality. The report's top-level `optimalityStatus` always describes the primary schedule as heuristic and `primaryScheduleIsProvenOptimal` is false. Optional exact-reference objects carry their own objective label, task limit and exact schedule; their presence never changes the claim attached to the primary schedule. The current objective is deterministic schedule feasibility and per-event restoration time, not weighted load-restoration priority. Grid switching, islanding, black-start and critical-load restoration policy remain with [#3418 POWER-SYSTEM-RESILIENCE-001](https://github.com/Luminous-Dynamics/symtropy/issues/3418).

1. Find tasks whose predecessors have completed.
2. Calculate each task's earliest precedence-ready time.
3. Find the earliest interval where every required resource class has its requested capacity available for the whole task.
4. Select the candidate with the earliest feasible start, with stable event/task-ID tie-breaking.
5. Repeat until all tasks are scheduled or a dependency cycle / invalid resource is found.

**This algorithm is not an optimal RCPSP solver.** It is intended to provide a deterministic, auditable baseline before introducing CP-SAT, MILP or specialised metaheuristics. A future optimiser must be compared against this baseline on identical frozen inputs and must prove every returned schedule feasible.

### Synthetic fixture schedule

The three-event fixture now represents parallel parts sourcing/logistics and crew mobilisation plus resource shift calendars. With the current synthetic durations and 14-hour daily windows, it declares restoration spans of 46 h, 1,222 h and 36 h. These are calculated from the task dependency graphs, simultaneous resource requirements and repeating calendar windows; they are not observed transformer restoration times.

The second event's long OEM procurement duration is deliberately a stress case. It is a synthetic value used to exercise the schedule and must not be interpreted as a current supplier quote. The v0.2 unserved-energy calculation assumes the declared net firm shortfall remains constant until restoration. It is a transparent screening approximation—not a chronological production-cost, load-flow, stability or customer interruption model. Later versions must consume a time series and declared alternative-service availability before making operational or reliability claims.

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


### Bounded exact reference oracle

A second scheduler path now exhaustively enumerates precedence-feasible serial schedules for cases of at most ten tasks. The CLI exposes it as `--exact-small`. It is an exact reference only for this bounded model: fixed non-preemptive durations, finish-to-start dependencies, fixed renewable resource capacities and release times. It is not a large-instance optimiser, and cases above ten tasks are rejected. The current task model supports multiple simultaneous renewable resource classes and repeating availability windows, but uses fixed non-preemptive durations and a shared elapsed-hour axis; it does not yet cover time zones, holidays, calendar exceptions, alternate execution modes, setup times, preemption or stochastic duration. Invoke it with `cargo run --manifest-path tools/grid-asset-recovery-validator/Cargo.toml -- --exact-small <case.json>`.

A deliberately small counterexample is included in the Rust tests:
- the deterministic earliest-feasible-start / stable-ID order has makespan **120 h**;
- exhaustive enumeration finds a feasible schedule with makespan **111 h**;
- the greedy gap is **9 h / 8.11%** relative to the exact makespan.

This is a four-task synthetic scheduling microcase, not a power-grid result. Its value is methodological: even a deterministic feasible heuristic can be suboptimal, so Power Fabric must publish its optimality gap where an exact reference is affordable.

For larger recovery plans, the intended comparison remains CP-SAT / MILP / specialised RCPSP methods, with the same task graph, resource model and objective. The exact-small oracle must never be used to imply optimality for the 20-task or larger synthetic/field cases.


## 13. Latest SOTA comparison and calendar-aware benchmark direction

The scheduling literature has moved beyond a simple PSPLIB-only comparison:

- A February 2026 SAT-based RCPSP study tested its methods over **13,413 instances** with diverse network/resource structures and reports 393 new lower bounds, 62 new upper bounds and 290 newly solved optimal instances. The paper is a useful algorithmic benchmark reference; it does not represent grid asset restoration.
- An August 2026 calendar-aware RCPSP dataset provides 12 reproducible instances from tiny (5–8 activities) through large (500–1,000 activities), with DAGs, heterogeneous resources, shifts, lunch breaks, statutory holidays and stochastic disruptions. It reports baseline implementations using Max-Plus, OR-Tools CP-SAT and Gurobi MIP. This is much closer to our next scheduling model than classic PSPLIB, but remains industrial production scheduling rather than grid-repair data.
- The 2026 RCPSP review classifies multi-skilled resources, uncertainty and multi-project scheduling among the major extensions. The benchmark plan should therefore evolve toward shared crews and equipment across events, shift calendars, stochastic task durations, correlated disruptions and portfolio-level objectives.

References:
- SAT-based RCPSP comparison (Computers & Operations Research, February 2026): https://doi.org/10.1016/j.cor.2025.107308
- Calendar-aware RCPSP dataset (Data in Brief, August 2026): https://doi.org/10.1016/j.dib.2026.113256
- 2026 RCPSP review (published online 10 June 2026): https://doi.org/10.1080/00207543.2026.2683116
- PSPLIB and RCPSP resources: https://www.projectmanagement.ugent.be/research/project_scheduling/rcpsp

### Transmission equipment / restoration reference points

- ENTSO-E's 2026 transmission grid investment and supply-chain assessment reports typical lead time of at least 36 months for 245–420 kV power transformers, with TSOs reporting 113–243 planned additions per year in this category. It highlights critical subcomponents such as tap changers, GOES, copper, aluminium, bushings and oil as supply-chain exposures. This is a European TSO planning/supply-chain reference, not a universal delivery promise.
- NERC's 2026 State of Reliability reports 2025 unavailability of 0.22% for AC circuits above 200 kV and 0.35% for transformers in North America. It notes 2025 had the highest transformer unavailability in its five-year period and that supply-chain constraints prolonged two transformer operational outages. These system unavailability measures complement—but do not replace—our proposed stage-level restoration and inventory metrics.
- ACER's 2026 Monitoring Report says permitting remains the leading cause of delay for electricity transmission projects in its reporting scope; average permitting duration increased to almost 4.5 years in 2026. It also reports 20% of electricity transmission Projects of Common Interest had not progressed status for more than six years. These figures motivate the separate route-legitimacy workstream; they should not be treated as a universal jurisdictional timeline.

References:
- ENTSO-E 2026 assessment: https://eepublicdownloads.blob.core.windows.net/public-cdn-container/clean-documents/Publications/Position%20papers%20and%20reports/2026/entso-e_report_investment_supply_chain_assessment_2026_261030.pdf
- NERC 2026 State of Reliability: https://prod.nerc.com/globalassets/programs/rapa/pa/nerc_sor_2026_technical_assessment.pdf
- ACER 2026 Monitoring Report: https://www.acer.europa.eu/sites/default/files/documents/Publications/ACER-2026-Monitoring-Report-energy-infrastructure-development.pdf

### What would constitute a real SOTA claim

A future result must identify the exact benchmark corpus and version, include baseline source/commit hashes, publish hardware/runtime limits, report lower/upper bounds and optimality gaps, and replay the same calendar/task/resource data across methods. A lower restoration time on our synthetic fixture is not a SOTA result. A solver beating PSPLIB is not evidence it beats grid-operator restoration practice. Both algorithmic and operational comparisons are required.


## 14. Service-consequence objective: weighted unserved energy

The initial scheduler used makespan as its exact small-case objective. That is useful for project completion, but it can prefer the wrong outage order when simultaneous events have very different service consequences.

A second bounded exact objective now minimises:

`weighted_unserved_energy = \sum_i (firm\_MW\_lost_i - alternate\_firm\_MW_i)^+ \times (restoration\_hour_i)`

Here the per-event weight is net firm-capacity shortfall in MW, and elapsed restoration is measured from the event start through the final required recovery task. This is a simplified weighted unserved-energy proxy; v0.x assumes constant shortfall until restoration and does not represent time-varying demand, customer interruption classes, network power flows, dynamic security, or production-cost dispatch.

The normal scheduler now exposes two list-scheduling policies:
- default earliest-feasible-start with stable task-ID tie-break;
- `--policy-service-weighted-critical-path`, which breaks same-start ties by descending net firm shortfall divided by remaining precedence critical-path duration.

The service-weighted policy is a heuristic, not an optimality proof. It is intended to improve outage ordering where resources are simultaneously available and service consequences differ. The benchmark must compare both policies on frozen cases and record cases where the priority ratio is misleading.

The CLI also provides two distinct bounded exact modes:
- `--exact-small`: minimum global makespan, at most 10 tasks.
- `--exact-small-service`: minimum weighted unserved energy, at most 8 tasks.

Both are exhaustive bounded references, not scalable optimisation claims. The heuristic policy can be invoked with `cargo run --manifest-path tools/grid-asset-recovery-validator/Cargo.toml -- --policy-service-weighted-critical-path <case.json>`. The service objective uses a deterministic greedy schedule as an initial upper bound, then exhaustively checks precedence-feasible serial schedules under the same resource calendars.

### Service-priority counterexample

A two-event synthetic example has one crew:
- Low-consequence event: 10 MW shortfall, 5-hour repair.
- Critical event: 100 MW shortfall, 10-hour repair.

Stable-ID greedy order repairs the low-consequence event first: 1,550 MWh weighted shortfall. Exact service-oriented ordering repairs the critical event first: 1,150 MWh. That is 400 MWh / 25.8% lower under this simplified objective, with the same 15-hour global makespan.

This is a scheduling microcase, not a grid restoration result. It demonstrates why "minimise project duration" and "minimise service harm" are different objectives and why a recovery plan should expose both. The service-weighted critical-path heuristic also selects the critical outage first on this microcase (1,150 MWh), matching the bounded exact result here. That agreement is a test of this small case only; it does not establish heuristic optimality or general superiority.

### Current synthetic fleet service ledger

Under the declared 14-hour daily resource calendar, the fixture has:
- OUTAGE-001: 80 MW net shortfall × 46 h = 3,680 MWh;
- OUTAGE-002: 50 MW net shortfall × 1,222 h = 61,100 MWh;
- OUTAGE-003: 25 MW net shortfall × 36 h = 900 MWh.

Total calculated synthetic weighted unserved energy is **65,680 MWh**. It is a schema/scheduler arithmetic fixture—not measured loss of load or a physical reliability prediction. The CLI includes per-event values and the total in its JSON report.

The next SOTA-facing objective should become a multiobjective Pareto set:
- weighted unserved energy;
- worst-event restoration time;
- p95/p99 restoration under stochastic disruption;
- lifecycle cost of spares, crew, equipment and logistics;
- schedule stability and robustness to common-cause outages.

Do not bury these in one opaque weighted score. Publish each objective, the trade-offs and the constraint/evidence boundary.


## 15. Second asset-family archetype: submarine HVDC cable repair

The benchmark corpus now has a separate synthetic cable-repair case, not just transformer replacement. It exercises:
- port inventory for a cable repair kit;
- port-to-subsea-fault heavy logistics with explicit cargo capacity and 18-hour transit;
- marine mobilisation;
- a heavy-haul vehicle;
- jointing team, cable-lay vessel and crane reserved simultaneously;
- two explicit 12-hour repair work blocks across recurring daily work windows;
- HVDC test equipment and return-to-service commissioning.

The cable case declares:
- 350 MW lost firm capacity and 50 MW bounded alternate capacity;
- 300 MW net shortfall;
- 110 hours from outage start to completion under the synthetic daily calendars;
- 33,000 MWh constant-shortfall arithmetic.

These figures are deliberately synthetic. The 12-hour repair blocks are task segmentation across windows, not a field-validated cable repair method or vessel operating plan. The model does not yet include weather forecasts, sea-state limits, vessel mobilisation permits, fault-localisation uncertainty, cable route engineering, burial depth, joint bay construction or site-specific commissioning requirements.

This second archetype is important because the model should not assume transformer-style recovery applies to cable systems. Later corpus families should add overhead-line tower/conductor failures, HVDC converter valves/transformers, GIS/circuit breakers and remote island/mine corridors.

The validator CI now schema-checks both transformer-fleet and submarine-cable fixtures, runs the same Rust semantic tests over both and invokes the CLI on both cases.


## 16. Integration boundaries: service cascades and deep uncertainty

The recovery engine owns **asset-level restoration execution**: equipment compatibility, spares, task dependencies, renewable resources, work calendars, logistics and return-to-service timing.

It should feed, not duplicate:
- [CIVDYN-CASCADE-001 #3894](https://github.com/Luminous-Dynamics/symtropy/issues/3894): propagate asset outage/restoration trajectories through electricity, water, telecom, transport, healthcare and other interdependent services; preserve dependency logic, degraded service and whole-event consequences.
- [PROTECT-ROBUST-001 #5317](https://github.com/Luminous-Dynamics/symtropy/issues/5317): evaluate nonstationary/compound hazards, uncertainty sets, tail risk, common-cause failures and robust service floors.
- [POWER-SYSTEM-RESILIENCE-001 #3418](https://github.com/Luminous-Dynamics/symtropy/issues/3418): retain grid-forming, islanding, black-start, protection and switching/operational qualification authority.
- [GRID-ENHANCEMENT-FIRM-CAPACITY-001 #6080](https://github.com/Luminous-Dynamics/symtropy/issues/6080): consume firm shortfall and bottleneck/alternative transfer boundaries.
- [GRID-TRANSMISSION-PORTFOLIO-001 #6067](https://github.com/Luminous-Dynamics/symtropy/issues/6067): consume corridor technology and route-specific asset configuration.

The cross-system chain should be:
**hazard/event scenario → asset failure set → recovery schedule and resource contention → asset service-return trajectories → dependent-service cascade → customer/ecosystem consequence → independent replay and risk review.**

Keep physical energy/resource ledgers with their existing owners. This recovery tool emits time and service-availability evidence; it does not claim to simulate load flow, frequency stability, water balances or downstream service dependence.


## 15. Disruption-aware recovery scenario sets

The scheduler now accepts an independent, versioned scenario-set file via:

`cargo run --manifest-path tools/grid-asset-recovery-validator/Cargo.toml -- --scenario-set docs/research/grid-asset-recovery/data/synthetic-recovery-scenarios-v0.1.json docs/research/grid-asset-recovery/data/synthetic-asset-recovery-v0.1.json`

A scenario may overlay:
- task duration realizations;
- task release delays;
- partial or complete resource-capacity outages over absolute elapsed-hour intervals;
- explicit cause IDs and shared correlation-group IDs for common-cause disruptions.

Resource outages consume capacity over the interval while remaining subject to the recurring shift calendar. The fast heuristic and both bounded exact small-instance references use the same outage reservations. A regression test specifically checks that exact makespan and service references cannot schedule through an outage.

### Synthetic scenario portfolio

The initial four-case scenario set contains:
- baseline (0.55);
- regional storm affecting the heavy-haul vehicle and crane under one shared correlation group (0.25);
- OEM supplier delay extending one procurement task (0.15);
- compound storm and supplier delay (0.05).

These are **synthetic illustrative weights**, not probabilities estimated from historical utility, weather, supplier or repair records. A synthetic case is forbidden from claiming expert-elicited or empirical-fit probability basis. The purpose is to verify the scenario engine, common-cause representation, probability aggregation and tail reporting.

The report emits expected weighted unserved energy, weighted p50/p95, worst scenario, scenario-specific makespan, service consequence and task schedule. The p95 is a weighted quantile over this small discrete scenario set; it is not a statistically reliable estimate of a real-world 95th percentile.

### What to do before real-world claims

1. Replace synthetic scenario weights with source-backed or expert-elicited values and document provenance, uncertainty and calibration.
2. Build a larger fixed-seed ensemble with independent and correlated hazards: severe weather, access closure, flood, wildfire, cyber/communications outage, port congestion, OEM production interruption, critical component shortage and multiple simultaneous asset failures.
3. Fit duration distributions by activity type from real anonymised event data. Compare lognormal, gamma, empirical bootstrap and heavy-tail alternatives; do not assume one distribution for all task classes.
4. Use common random numbers across competing spare/crew strategies so the same failure and disruption realizations are replayed for every policy.
5. Report expected value, p50/p90/p95/p99, CVaR95, worst-case bounded stress, recovery probability by time threshold and schedule stability.
6. Validate calibration with held-out years/sites and reliability diagrams / proper scoring rules; compare predicted lead-time and repair-time distributions with observed outcomes.
7. Model common-cause dependence explicitly (shared storm system, port, supplier, region, communications provider, depot or road corridor). Do not multiply marginal probabilities as if correlated events were independent.
8. Preserve the separation between physical event risk, supplier lead-time uncertainty, task duration uncertainty and policy choices.

### Next algorithmic frontier

A credible robust-recovery solver should compare:
- greedy earliest-start and service-weighted critical-path policies;
- multi-start priority-rule serial schedule generation;
- CP-SAT interval/cumulative resource scheduling;
- MILP or SAT/branch-and-bound for bounded instances;
- reactive rescheduling after observations, with frozen/committed tasks protected;
- robust or stochastic optimisation with expected weighted unserved energy, CVaR95, p95 restoration and lifecycle cost shown as separate objectives.

A policy should be evaluated on both **solution quality** and **stability**: how much schedule churn it creates when a supplier estimate or weather forecast changes, how many already-started tasks it invalidates, and whether a small input change causes a large and avoidable resource reassignment.

The scenario engine currently uses deterministic scenario overlays and fixed task durations per scenario. It is not yet a stochastic simulator or a dispatch/operational decision system.
