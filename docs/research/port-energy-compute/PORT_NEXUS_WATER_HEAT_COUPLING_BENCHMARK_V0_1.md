# Port Nexus Compute–Cooling–Water Coupling Benchmark v0.1

**Protocol:** PORT-NEXUS-WATER-HEAT-001  
**Snapshot:** 19 November 2026  
**Status:** cross-owner integration benchmark and synthetic heat-interface fixture; not a desalination design or product-water qualification.

## Thesis

A coastal compute campus can be more than an electrical load. It can be a controllable source of temperature-graded heat, a participant in port electricity management and a potential thermal partner for a nearby water-production system.

The design question is not whether waste heat can be reused. It is whether the complete joint system can deliver more useful compute, qualified water, heat service and grid value with lower whole-system cost/risk after all pump, heat-pump, cooling, RO, storage, treatment, cleaning, brine, outfall and lifecycle burdens are counted.

The relevant service chain is:

`AI useful work -> liquid cooling -> heat exchanger -> (direct RO feed preheat | heat pump | thermal storage | ORC | seawater rejection)`

The desalination system remains a distinct water-service owner. Heat integration must not upgrade a thermal balance into a potable-water claim.

## New research evidence

### 1. Data-centre / seawater desalination co-optimization

Yang, Konstantinou and Hou, *Enabling grid-interactive data center-desalination coordination through thermo-electric coupling-based waste heat utilization*, Applied Energy 408 (1 April 2026), DOI [10.1016/j.apenergy.2026.127428](https://doi.org/10.1016/j.apenergy.2026.127428).

This study links workload-driven waste heat to RO feedwater temperature and temperature-/salinity-dependent specific energy consumption. Its two-stage stochastic planning/operation approach uses Benders decomposition and McCormick linearization. It reports an 86.5% reduction in total annualized cost in its case, with waste-heat synergy contributing 17% savings.

The 86.5% is an overall case result and must not be attributed to waste heat alone or reused as a target. The 17% synergy is the more relevant hypothesis to isolate. Reproduce the exact study boundary and baseline before comparing.

Sources: [Applied Energy](https://www.sciencedirect.com/science/article/pii/S0306261926000802); [University of Hong Kong repository](https://hub.hku.hk/handle/10722/379334).

### 2. Heat pump upgrading vs ORC power recovery

Hong et al., *Thermal design-based waste heat utilization strategies for liquid-cooled data centers in third-generation district heating networks*, Energy Conversion and Management 368 (15 November 2026), DOI [10.1016/j.enconman.2026.122025](https://doi.org/10.1016/j.enconman.2026.122025).

The modeled 120 MW liquid-cooled facility supplies waste heat at 50–60°C. The study compares high-temperature heat pumps with organic Rankine cycles. It reports average COP 3.46 for its selected heat-pump fluid and ORC thermal efficiency 5.80% for its selected fluid, with electricity-to-heat price ratio crossover regions around 2.7 and 3.4 in that case.

The study explicitly highlights that when liquid cooling already avoids chiller operation, adding a heat pump may become a net parasitic load unless it replaces a real external heating service. Therefore, the model must compare delivered useful heat against heat-pump electricity and the value of the displaced alternative—not count recovered heat as free.

Source: [Energy Conversion and Management](https://www.sciencedirect.com/science/article/pii/S0196890426009945).

### 3. Seawater cooling and freshwater co-production

Chen et al., *A dual-purpose seawater evaporation system for data center cooling and freshwater production: design and performance analysis*, Applied Thermal Engineering 287 (February 2026), article 129458.

The proposed seawater-evaporation architecture couples heat removal with freshwater co-generation. It reports modeled freshwater outputs around 10,004 and 10,744 tonnes/day for two cases and power reductions of 41.5% and 40.0% against its study baselines.

This is an exploratory architecture, not evidence of commercial maturity, potable-water quality, marine discharge acceptability or site-specific performance. It belongs in the comparison set, not as the default system.

Source: [Applied Thermal Engineering](https://www.sciencedirect.com/science/article/abs/pii/S1359431125040505).

## Paradigm shift

Treat waste heat as a **time-, temperature- and sink-qualified co-product**, not as an automatically useful stream.

There are three separate questions:
1. Is heat physically available at a useful temperature and time?
2. Is a real downstream service present, with compatible flow, temperature, quality and delivery window?
3. Does the recovered-heat path improve the complete system after electricity, pumping, heat-pump lift, storage, fouling, treatment, environmental impacts and lifecycle cost?

A thermal energy balance alone answers only part of the first question.

## Existing owner composition — no duplicate engines

- Symtropy #5355 DESAL-BRINE-CO-DESIGN-001 owns desalination/brine architecture comparisons, species mass, product quality and water recovery.
- Symtropy #2152 WATER-PROC-CLOSURE-001 owns process water/material/utility/residual closure.
- Symtropy #4958 POTABLE-WATER-TRAIN-BENCH-001 owns treatment-train and intended-use product-water service.
- Symtropy #2683 ENERGY-THERMAL-STATE-001 and #1916 THERMAL-DYNAMIC-001 own storage state, losses, heat grade and multi-period thermal integration.
- Symtropy #6228 OCEAN-COMPUTE-THERMAL-CABLE-001 owns seawater cooling scenarios and 0D cooling heat accounting.
- Symtropy #6211 owns compute workload/service obligations and rebound.
- Symtropy #6255 owns read-only multi-domain re-plan triggers.
- Symtropy #4582 owns canonical port W/Wh/readiness and shared physical capacity.
- Symthaea SEP-001 #5199 owns generic separator/species semantics; #11572/#11473 own maritime shore-power service and forecast-aware EMS.
- Sol-Atlas #654/#942 supplies source-qualified water source, intake/outfall, site, ecology and rights-aware spatial context.
- Mycelix #5974/#4882 records approvals, metrology and physical delivery/reconciliation evidence.

This issue owns only the **co-design interface and matched benchmark**. It does not create a second RO model, water ledger, membrane engine, thermal-store solver, cooling model or product-water certification engine.

## V0.1 matched architectures

Compare under identical compute useful-work, water-demand, source-water, grid, marine and weather scenarios:

1. **No-reuse reference:** liquid-cooled data centre rejects heat at its qualified cooling boundary; independent conventional SWRO.
2. **Direct low-grade heat use:** an exchanger preheats RO feed only when the actual membrane/process model admits the temperature, salinity and quality envelope.
3. **Heat-pump upgraded service:** lift heat to a named process/port sink; include compressor, source/sink pumps, losses and displaced heat alternative.
4. **Thermal storage:** decouple continuous compute heat from time-varying water/heat demand using the existing #2683/#1916 store model.
5. **ORC alternative:** compare net electricity generation after pumps, cooling, controls, low-temperature efficiency and availability.
6. **Three-mode seawater cooling:** use direct seawater / partial economiser + chiller / chiller-only fallback; integrate heat recovery only when it does not violate the cooling boundary.
7. **Dual-purpose seawater evaporation:** include as a lower-maturity research comparator with its own freshwater quality, concentration, residuals and environmental burden.
8. **Hybrid cascade:** combine multiple stages only when each stage provides independently measured useful service and no heat or water stream is double-counted.

## Synthetic known-answer heat interface

The companion fixture contains one deliberately simplified 0D energy balance:
- IT heat generation: 10 MW;
- feedwater preheat demand: 300 kg/s water, 4.18 kJ/(kg·K), 20°C to 25°C;
- feedwater useful heat demand: 6.27 MW;
- recovery exchanger efficiency: 80%;
- IT-loop heat withdrawn for reuse: 7.8375 MW;
- useful heat delivered: 6.27 MW;
- transfer loss: 1.5675 MW;
- remaining IT heat rejected to seawater: 2.1625 MW;
- total energy partitions: 6.27 MW useful preheat + 1.5675 MW exchanger loss + 2.1625 MW residual rejection = 10 MW.

This fixture demonstrates accounting only. It does **not** model membrane performance, RO SEC temperature response, feed salinity/speciation, scaling, fouling, potable-water quality, heat-exchanger sizing, transient controls, or environmental discharge. No desalination benefit is claimed from this arithmetic case.

The synthetic no-sink case must reject heat-reuse credit and send the available heat through the declared cooling/rejection path. A sink that is absent, out-of-window or incompatible with temperature grade cannot be valued.

## Required coupled measurements and evidence

### Compute and grid
- useful work, job release/deadline, service latency and queue state;
- IT and facility power on explicit meter boundaries;
- PCC/feeder, tariff and carbon signal vintages;
- renewables, storage, cooling auxiliaries and recovery/rebound trajectories;
- service-equivalent comparisons against fixed compute and port schedules.

### Heat and cooling
- IT loop supply/return temperature, flow and heat rate;
- recovered heat source, delivered heat, transfer loss and rejected heat separately;
- heat-pump/ORC electricity and net useful output;
- source/sink approach temperature, pinch and operating envelope;
- pump and auxiliary electricity;
- thermal-store state/losses and cooling fallback;
- seawater intake/discharge and site-specific marine limits.

### Water
- exact feed source and water chemistry / salinity / species / temperature;
- feed mass flow and pressure;
- admitted RO or desalination model revision and validity envelope;
- temperature-dependent SEC only from the owner-validated process model;
- product volume, intended use, quality, post-treatment and off-spec diversion;
- concentrate/brine, cleaning, pretreatment and all residuals;
- source rights, lab analysis, method, uncertainty and meter boundaries.

No feedwater quality or water-service evidence means no qualified product-water claim.

## Falsification cases

1. Heat source exists but no time-matched downstream sink: zero recovered-service credit.
2. Heat-pump output is counted but compressor electricity omitted: reject.
3. The same source heat is counted both as RO preheat and seawater heat rejection: reject.
4. Exchanger delivery exceeds source heat minus declared losses: reject.
5. RO temperature response is extrapolated beyond owner-validated range: not evaluable.
6. Feedwater is preheated but product quality fails: no qualified water credit.
7. Water flow is reduced while heat-recovery schedule assumes the old demand: trigger/re-plan.
8. Thermal-store losses and reserve are omitted: reject.
9. A heat-reuse configuration increases cooling energy, but the cost/energy boundary omits it: reject.
10. Modelled freshwater output is reported as delivered potable water: reject.
11. Brine, chemicals, membrane replacement or outfall impacts are omitted: no lifecycle/environmental benefit claim.
12. Heat source or RO feed telemetry is stale/future-available: incomplete, not a pass.

## Metrics

Report separate, non-compensatory values:
- useful compute completion and on-time fraction;
- qualified freshwater volume and intended-use quality;
- RO SEC (kWh/m³ qualified water) using the full owner-declared boundary;
- thermal source, useful heat delivered, transfer losses and residual heat;
- heat-pump COP / ORC net electricity and parasitics;
- cooling electricity, pump power, thermal-store loss/reserve;
- seawater discharge thermal load and environmental evidence;
- PCC peak/ramp, feeder constraints, cost, carbon and water burden;
- lifecycle capex/opex, membrane/heat exchanger replacement, downtime and maintenance.

## Claim ceiling

This work establishes a coupled benchmark and integration contract only. It does not prove a specific port needs desalination, that a real feedwater source is available, that feed preheat improves RO performance, that recovered heat is saleable, that freshwater is potable, that discharge is environmentally acceptable, or that any architecture is commercially superior. Those claims require owner-qualified process models, metrology, real demand/sink evidence, site permits and independent field validation.
