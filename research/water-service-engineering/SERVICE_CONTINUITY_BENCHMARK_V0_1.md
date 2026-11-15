# WaterLoop Service Continuity Benchmark v0.1

**Date:** 15 November 2026  
**Parent:** [WATER-SERVICE-CONTINUITY-001 #5364](https://github.com/Luminous-Dynamics/symtropy/issues/5364)  
**Evaluator:** `waterloop-physical-t0`  
**Evidence class:** synthetic-only  
**Claim ceiling:** time-stepped inventory/service arithmetic and contract semantics; no physical service qualification

## 1. Paradigm: water service, not water inventory

A household does not receive service because it owns a tank, a pump, a filter or an automation controller. It receives service when the required quantity reaches the declared point of use, at the declared pressure/flow envelope, for the declared intended use, at the required time.

The benchmark unit is therefore:

`verified use-qualified service delivered at the point-of-use boundary over time`

It is not nominal tank capacity, treatment throughput, pump nameplate power or predicted service.

Use classes must remain distinct: drinking water, food preparation/personal hygiene, toilet flushing, laundry, irrigation and process water cannot be pooled as interchangeable litres. This T0 evaluator models **one declared use class per scenario** so that inventory and service credit cannot silently cross quality/use boundaries. A complete multi-use product will require separate physical circuits or separately qualified reservoirs and their own evidence bindings.

## 2. South African regulatory reference scenario

The Revised Compulsory National Water and Sanitation Services Standards (GN 6292, Government Gazette 52814, 6 June 2025) establish useful context:

- Regulation 4(1) addresses interruptions longer than 48 hours and requires Water Services Authorities, in agreement with relevant Water Services Institutions, to take reasonable measures to ensure access to alternative drinking-water supply of at least 15 L/person/day at strategically selected, relatively convenient and safe delivery points.
- Regulation 4(2) requires alternative drinking water to meet applicable SANS 241 prescripts.
- Regulation 4(3) adds monitoring/source/tanker records when alternative drinking-water supply continues for more than a week.
- Regulation 4(5) addresses sufficient reservoir capacity and review of operational levels.
- Regulation 3's interim-service standard includes 6 kL/household/month, a minimum 10 L/min flow rate, availability for at least 358 days/year and no interruption longer than 48 consecutive hours, together with SANS 241 water quality.
- Regulation 12 addresses greywater risks when a Water Services Institution plans to collect/use greywater beyond a household boundary.

Primary source: https://www.gov.za/sites/default/files/gcis_document/202506/52814gon6292.pdf

The corpus includes a 72-hour example for four people at 15 L/person/day (180 L total). This is **a context-specific regulation reference scenario**, not a universal household design standard, not a claim that WaterLoop is a Water Services Authority, not a substitute for an authority's delivery obligations, and not evidence of drinking-water quality. The scenario only matches the quantity arithmetic; source, sampling, quality, delivery-point convenience/safety and legal duties remain separate.

## 3. Frozen T0 semantics

Each scenario defines:
- initial measured storage volume;
- maximum usable storage capacity;
- a protected reserve unavailable to discretionary demand;
- an intended-use profile and corresponding stored-water quality profile;
- whether stored-water quality is admitted for that use;
- a series of fixed-duration time steps.

Each time step includes:
- source inflow volume and whether its quality is admitted for the same intended use;
- essential and discretionary demand;
- measured fixture-boundary service capacity;
- whether the service measurement is admitted;
- whether the declared pressure/flow service floor is met.

The evaluator intentionally does not derive outlet capacity from pump curves or calculate pressure. Those values must be provided by a future physical T1 observation, with Mycelix's instrument and metrology contracts.

### Inventory and allocation

For each interval:

`inventory_before_service = min(maximum_storage, prior_inventory + admitted_source_inflow)`

`overflow = max(0, prior_inventory + admitted_source_inflow - maximum_storage)`

If the measured service evidence is not admitted, the entire run is **Indeterminate** and no aggregate service credit is reported. If the declared pressure/flow floor is not met, the service capacity credited for that interval is zero.

Critical/essential demand is allocated first:

`essential_delivered = min(essential_demand, measured_service_capacity, inventory_before_service)`

Discretionary demand is then limited to both remaining capacity and inventory above the protected reserve:

`discretionary_delivered = min(discretionary_demand, remaining_capacity, max(0, inventory_after_essential - protected_reserve))`

Essential demand may draw into the protected reserve when necessary; that shortfall is reported. Discretionary demand cannot consume the protected reserve. Every flow is counted once; source inflow that is not quality-admitted for the intended use is excluded from inventory and recorded as rejected inflow.

### Quality and evidence admission

- Stored-water quality must be admitted for the scenario's intended-use profile.
- The stored-water quality profile ID must equal the intended-use profile ID.
- Each service interval must have admitted measurement evidence.
- An unqualified new source inflow is rejected, not blended into credited inventory.
- A failed pressure/flow service floor does not earn declared-service credit even when water remains in the tank.

If required stored-water quality or service metrology is missing, stale or unadmitted, the output is Indeterminate with null service metrics. Do not misrepresent unknown as zero service or successful delivery.

## 4. Metrics

The evaluator reports:
- essential demand, delivered and unmet volume;
- essential-demand volume coverage;
- full-critical-demand interval availability;
- first interval with a critical-demand shortfall and elapsed hours to that interval;
- discretionary demand, delivered and unmet volume;
- source inflow admitted and rejected for the use profile;
- overflow and end storage.

These are deterministic arithmetic outputs from synthetic profiles. They do not represent a measured household, field population or forecast distribution.

## 5. Frozen synthetic corpus

The WaterLoop physical T0 corpus now contains 25 cases:
- 15 prior physical-bench cases for water balance, usable storage, source substitution, pump energy, service floors and leak metrics;
- 10 continuity cases for normal service, the South African Regulation 4 reference profile, reserve insufficiency, pump outage with gravity fallback, compound demand surge, source restoration, outlet service-floor failure, missing metrology, unqualified stored-water quality and rejected unqualified inflow.

Inputs, manifest, schema and independent oracle are separate files:
- `waterloop-physical-t0-v0.1.json`
- `waterloop-physical-manifest-v0.1.json`
- `waterloop-physical-oracle-v0.1.json`
- `waterloop-physical-t0.schema.json`

## 6. Industry comparison and benchmarks

Compare at matched household/use profiles and evidence level:
1. direct-supply-only baseline;
2. storage-only gravity delivery;
3. storage plus booster pump;
4. storage + pump + local backup power;
5. source-separated, use-qualified reuse;
6. WaterLoop hybrid arrangement;
7. community alternative-delivery-point/no-build baseline where relevant.

The JoJo-style automatic tank-fill and pressure-pump architecture is an established baseline; tank volume and controller features alone are not a WaterLoop novelty claim. Greywater systems such as Hydraloop establish a market baseline for recovery, but advertised potential throughput must not be compared to measured delivered service or to a different third-party rated capacity. Leak-monitoring systems such as Moen Flo establish a baseline for learned demand, alarms and automatic shutoff; WaterLoop must compete on false-alarm burden, avoided loss, safe failover and verified recovery, not on presence of AI.

A 2026 review of 86 AI water-leak studies found operational readiness constrained by isolated algorithms that ignore physical/operational constraints and recommends reporting false alarms, compute feasibility, detection time and integration with SCADA/GIS. A 2026 cross-architecture leak benchmark reports >95% binary detection but identifies localization as the harder problem. That is why this suite includes delivered service and service continuity rather than only event classification:
- https://doi.org/10.2166/hydro.2026.069
- https://doi.org/10.1016/j.jwpe.2026.110114

### Stretch targets — future physical trials only

These are research objectives, not achieved results or universal design standards:
- at least 99% full-critical-service interval availability within a declared design envelope;
- at least 95% of declared essential demand volume delivered in predeclared 72-hour outage profiles;
- zero discretionary draw below the declared protected reserve;
- zero unsupported cross-use credit;
- 100% abstention on seeded missing/stale/conflicted critical metrology and unqualified stored-water quality;
- at least 25% lower lifecycle cost per verified useful m³ than a matched storage/pump baseline, without worsening critical service;
- at least 20% lower energy per verified useful m³ than a matched pump baseline at the same pressure/flow service floor;
- recovery to declared service within a predeclared time after source/power restoration.

Report by use profile, household population, outage duration, source reliability and failure mode. Never average away a critical service failure or treat discretionary-use shortfall as equivalent to drinking-water deprivation.

## 7. What this does not model

T0 is not:
- a hydraulic network solver, water-hammer model or pump controller;
- a water-quality model, microbial-risk model or SANS 241 compliance assessment;
- an optimal scheduling, stochastic reliability or behavioural model;
- a field validation or household-service trial;
- an authorization to connect, treat, pump, valve, isolate, procure or install.

The existing solver and evidence owners remain authoritative. Symtropy #4959 owns pump service/physical references; #5170 owns transient solver qualification; #5145 owns intrusion experiments; Mycelix #6034/#6029/#6048/#6054/#6057 owns instrument, uncertainty, timebase and condition semantics; Mycelix #6100 owns use-specific water-quality assessment; Symtropy #5335/#4531 own the physical test cell and integration.

## 8. T1 physical validation gate

Before translating T0 into physical or household claims, freeze one intended-use profile and compare actual systems under matched demand/time conditions in the unoccupied test cell. Independently admit:
- tank geometry and level/volume calibration;
- source, outlet, overflow, drain and reject measurement;
- outlet pressure/flow and pressure/flow-floor validity;
- pump and auxiliary electricity;
- source/batch/outlet sample identity, method, calibration and quality assessment;
- configuration, firmware, timebase and fault-event identity;
- local plumbing/electrical review and applicable South African professional requirements.

No potable, health, compliance, installation or manufacturing readiness claim follows from the synthetic evaluator.