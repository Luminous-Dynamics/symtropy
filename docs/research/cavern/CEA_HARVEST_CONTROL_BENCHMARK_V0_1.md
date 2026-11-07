# CEA harvest-constrained lighting / climate control benchmark v0.1

**Parent:** [CEA-HARVEST-CONTROL-001 #4074](https://github.com/Luminous-Dynamics/symtropy/issues/4074)  
**Composes:** #4060 crop schedule, #3985 photon delivery, #4065 climate dynamics, #3957 whole-facility ledger, #4041/#4057 experiment selection.

**Status:** benchmark protocol and source-pinned simulation comparator. It does not implement a physical controller, establish measured savings or authorize closed-loop control.

## 1. Direct external comparator

Sathyanarayanan, Raecke & Streif, *Beyond Light Scheduling: Harvest-Constrained Co-Optimization of Lighting and Climate in Vertical Farms under Volatile Electricity Prices*, Smart Agricultural Technology 15 (2026), article 102636, DOI 10.1016/j.atech.2026.102636.

Primary source: https://doi.org/10.1016/j.atech.2026.102636  
Publisher article record: https://www.sciencedirect.com/science/article/pii/S2772375526008609

The source reports a three-level hierarchical simulation at a 250 ±1.5 g/head harvest target:
1. strategic crop-cycle plan / harvest trajectory over the full 14–25 day cycle;
2. day-ahead price-responsive re-planning;
3. 15-minute shrinking-horizon MPC updating from crop / climate state, prices and weather.

Across 36 simulated country × season × production-intensity cases, the paper reports 13.7% mean electricity-cost reduction (7.3–19.7%) and 8.4% lower electricity use. About 97% of modeled cost savings is attributed to climate control; lighting-only optimization is about 10% more expensive than joint control.

These are model-scenario results, not field measurements. Data and simulation code are available on request. A secondary technical summary reports additional robustness limitations / sensitivity results; those are retained as challenge hypotheses, not primary truth until verified against the full paper / supplement.

## 2. Research hypothesis

The target is **harvest service under measured uncertainty**, not price arbitrage:
- meet marketable mass and harvest deadline;
- satisfy crop-quality, dry-matter and nutritional constraints;
- maintain temperature / humidity / CO2 / photon / condensation service;
- minimize whole-facility cost / energy / water / emissions across explicit resource boundaries;
- remain safe and predictable when crop model / sensor / actuator / forecast is imperfect.

The crop model's internal biomass state is not an observation. A forecast harvest target does not count as achieved harvest. Only an independent measurement with an admitted uncertainty and marketability protocol can verify the physical harvest.

## 3. Baseline registry

Compare on identical crop, facility, weather, forecast, tariff and resource boundaries:
- static incumbent fixed schedule;
- greedy cheapest-hour lighting only;
- LED-only optimizer;
- climate-only optimizer;
- strategic crop-cycle / harvest-constrained optimizer;
- day-ahead controller;
- 15-minute intraday MPC;
- joint lighting + climate co-optimization;
- joint controller with independent biomass feedback;
- robust / uncertainty-aware policy;
- human/operator baseline where prospectively captured;
- exact exhaustive oracle on small finite schedules.

The source paper's 13.7% cost / 8.4% energy outcomes are the first external source reproduction target. Do not use them as guaranteed thresholds. The original data/code being request-only means the detailed baseline is SOURCE_SUMMARY_ONLY until admitted.

## 4. Critical feedback / model failure experiment

Create a matched suite with crop-model bias and independent biomass sensing:
- baseline calibrated crop model;
- biased growth rate / delayed development;
- doubled reported calibration error as a challenge case;
- cultivar / crop-stage OOD transfer;
- noisy, delayed, missing or drifting biomass proxy;
- sensor occlusion / missing plants / density changes;
- daily independent biomass observation and update;
- no feedback and stale feedback negative controls.

The paper's secondary technical summary reports that crop-model error can cause harvest-marketability failure and that daily biomass correction improves modeled marketability. The exact percentage needs primary-source verification. Benchmark the general mechanism regardless of whether those secondary numbers reproduce.

The biomass feedback channel should have:
- calibrated gravimetric harvest ground truth;
- non-destructive vision / canopy proxy method;
- independent measurement uncertainty / prediction interval;
- cultivar / stage / geometry applicability;
- missingness, occlusion, time delay and sensor drift;
- a fail-closed state when the estimate is OOD or too uncertain.

## 5. Multi-timescale control protocol

Use an explicitly nested plan:
- Strategic: crop-cycle feasible harvest trajectory, crop timing, permitted DLI and quality envelope.
- Day-ahead: revise target against forecast weather / tariff / demand / carbon signal with uncertainty and resource limits.
- Intraday: update from actual crop state, climate, meter / equipment state and latest forecast; obey hard actuator, cycle, power and climate constraints.
- Fallback: static safe schedule / operator escalation when crop state, timebase, forecast, model validity or actuator state is untrusted.

A 15-minute sampling interval is an external comparator, not a universal sampling requirement. Use the shortest interval justified by sensor latency, equipment dynamics and meaningful control response.

## 6. Required challenge families

At minimum reproduce the issue #4074 42-case portfolio:
- source-summary baseline and exact small-instance oracle;
- fixed vs variable DLI and fixed vs adaptive photoperiod;
- LED-only vs climate-only vs joint control;
- model bias, biomass sensor noise / drift / missingness / delay;
- marketability classifier uncertainty and fresh / dry mass basis mismatch;
- weather / tariff forecast error and stale grid-carbon signal;
- equipment minimum on/off time, defrost, COP part-load, actuator saturation and wear;
- peak demand, power caps and demand charges;
- CO2 supply limits, irrigation and nutrient constraints;
- false-feasible harvest target under hidden true crop mass;
- safe fallback and stop / human review cases.

Keep hard constraints as constraints, not scalar penalties that can be traded for a cheaper harvest.

## 7. Metrics

Harvest:
- target weight error and deadline miss;
- marketable mass / fraction / grade;
- fresh mass, dry mass, dry-matter %, nutrition and rejection;
- false-feasible prediction rate;
- confidence interval / uncertainty calibration for harvest forecast.

Energy / climate:
- whole-facility kWh and peak kW;
- LED, HVAC, dehumidification, fans, pumps, CO2, controls and cold-storage separately;
- heating/cooling and useful / rejected thermal streams;
- crop-zone climate service and condensation exposure.

Economics / environment:
- tariff, demand charges, CO2 cost and carbon-cost provenance;
- water make-up, reuse, purge and groundwater penalty if applicable;
- time-matched grid emissions with geography / source / interval evidence.

Control / reliability:
- solve time, infeasible solve frequency, fallback activation;
- device cycles, defrost, wear / service interval;
- forecast/model residual, sensing age / latency and recovery time.

No universal composite score. No mean improvement may conceal a missed harvest or safety / quality breach.

## 8. Proposed advancement goals

After source baseline and test corpus are frozen:
- exact small-case scheduling agreement with an independent oracle;
- zero harvest / climate / power / equipment hard-constraint violations in deterministic conformance fixtures;
- zero false-feasible marketable-harvest promotion on frozen model-error / stale-biomass / OOD controls;
- on independent measured trials, target at least 95% of harvests within a pre-registered marketable target band at equal or lower full-facility cost than the static baseline; report confidence bounds and sample size;
- demonstrate that biomass-feedback sensing improves harvest reliability enough to justify the sensing / maintenance costs;
- stretch: improve the Pareto frontier for whole-facility energy / cost while preserving harvest, quality, climate, water and reliability under held-out forecast and device disturbances.

These are proposed validation targets, not achieved performance or a claim over the paper's simulation.

## 9. Claim ceiling

A passing synthetic policy test is not an operational controller qualification. A model forecast is not a measured harvest. A model-simulated saving is not a field saving. Physical use requires calibrated meters, independent biomass evidence, validated climate / crop applicability, safety controls, human authority and a measured campaign.
