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
