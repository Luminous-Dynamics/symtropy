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


## South African tariff and grid-emissions evidence — 2026/27

### City Power industrial three-part TOU tariff

The City of Johannesburg 2026/27 Medium-Term Budget (May 2026), printed pages 96–97, publishes an industrial three-part time-of-use tariff for qualifying large consumers. The schedule took effect with the City's FY2026/27 tariffs from **1 July 2026**. Official sources:
- Budget / tariff schedule: https://joburg.org.za/documents_/Documents/2026-27-Final-Medium-Budget-Reports/2026-27-FINAL-BUDGET-BOOK.pdf
- City tariff announcement and effective date: https://joburg.org.za/media_/Newsroom/Pages/2026-News-Articles/2026-News-Articles/2026-News-Articles.aspx (see approved 2026/27 tariffs announcement, 10 June 2026)

The tariff is not universally available. City Power states that eligibility criteria apply; it targets large consumers able to reduce or reallocate load in peak/standard periods. The budget states it applies to business supply capacities **exceeding 100 kVA**, with application / approval by City Power. A CEA site must not be assumed eligible: record the actual distribution utility, supply voltage, contracted tariff and approved supply capacity.

Reported tariff schedule values (table units are cents/kWh, rand/kVA and rand/month):

| Voltage class | Fixed service charge (R/month) | Network charge (R/month) | Demand charge (R/kVA/month) |
|---|---:|---:|---:|
| Low voltage (≤1 kV) | 2,444.32 | 2,185.32 | 461.28 |
| Medium voltage (>1–33 kV) | 3,360.96 | 9,347.51 | 431.11 |
| High voltage (>33 kV) | 42,915.09 | 43,848.71 | 400.93 |

Energy charge schedule (common across the three voltage categories in the published table):

| Season | Peak (c/kWh) | Standard (c/kWh) | Off-peak (c/kWh) |
|---|---:|---:|---:|
| Summer (September–May) | 322.00 | 242.42 | 186.35 |
| Winter (June–August) | 766.24 | 292.56 | 200.44 |

Time bands:
- Weekdays: peak 07:00–10:00 and 18:00–20:00; standard 06:00–07:00, 10:00–18:00 and 20:00–22:00; off-peak 22:00–06:00.
- Saturdays and public holidays: no peak; standard 07:00–12:00 and 18:00–20:00; off-peak 12:00–18:00 and 20:00–07:00.
- Sundays: standard 17:00–19:00; off-peak 19:00–17:00.

Further billing effects:
- Minimum billed demand is the greater of measured demand, 70 kVA, or 80% of the average of the three highest recorded demands over the preceding 12 months.
- Customers with power factor below 0.96 may be billed for reactive energy above the stated 30% threshold; the tariff table lists 46.25 c/kVArh.
- Summer and winter rate seasons are separate from the weekday/weekend time bands.
- The City's announcement reports an 8.63% average electricity tariff increase effective 1 July 2026. This average increase is not a replacement for the tariff-specific rates above.

The schedule creates a real co-optimization problem: a controller must consider energy charges, fixed/service/network charges, demand charges and the demand ratchet. Shifting kWh away from peak does not guarantee a lower bill if the billed demand floor, power-factor charges or fixed charges dominate. Tariff-driven load shifting is not an energy-efficiency claim.

### DFFE South Africa 2024 location-based grid factors

The Department of Forestry, Fisheries and the Environment published the *South Africa's 2024 Grid Emission Factors Report* on 11 September 2026, Government Notice 7891, Government Gazette 55353:
https://www.gov.za/sites/default/files/gcis_document/202609/55353gon7891.pdf

The report gives four **annual location-based factors** from 2024 calendar-year data:
- DGGEF (domestic generation): **0.994 tCO₂e/MWh**;
- NGGEF (national generation / consumption): **0.960 tCO₂e/MWh**;
- TLGEF (transmission losses): **0.021 tCO₂e/MWh**;
- DLGEF (distribution losses): **0.071 tCO₂e/MWh**.

The report identifies NGGEF as the factor for consumer Scope 2 reporting; DGGEF is intended for policy / international reporting, while TLGEF and DLGEF are for Scope 3 loss reporting. Do not add TLGEF or DLGEF to NGGEF for Scope 2 without a distinct reporting boundary and method.

**Critical temporal limitation:** DFFE states that the annual GEF publication has an inherent data-availability lag of approximately two years; the 2026 publication reflects 2024 data. It does not provide a current 15-minute marginal or hourly carbon signal. Use NGGEF for the stated annual location-based Scope 2 reporting boundary, not to rank individual intervals for real-time carbon-responsive lighting.

A historical hour-of-day profile derived from published generation data may be used only as a clearly labelled sensitivity scenario. It is not equivalent to date-aligned measured hourly average emissions, marginal emissions, or a verified forecast. Behind-the-meter PV, imports, local procurement and site-specific supply must be separately accounted for.

### Required South African market-signal controls

Add benchmark cases for:
- eligible City Power industrial TOU consumer versus ineligible supply / Eskom distribution customer;
- LV/MV/HV schedule selection and rate-version validity;
- summer/winter season boundary and weekday/Saturday/public-holiday/Sunday transition;
- fixed, network, demand, minimum-demand ratchet and reactive-energy billing;
- peak shifting that lowers energy charge but not billed demand;
- a tariff policy that lowers cost but increases total kWh;
- an annual NGGEF used correctly for Scope 2 reporting;
- invalid substitution of DGGEF, TLGEF or DLGEF for NGGEF;
- annual factor incorrectly used as an hourly / marginal signal;
- stale or missing price / carbon series causing HUMAN_REVIEW / no carbon claim;
- City Power tariff applied to a site supplied by another distributor;
- tariff rates changed without effective-date update.

This is a South African *market-data comparator*, not a claim that every Johannesburg site qualifies for the City Power industrial TOU tariff. The source facts and units are preserved in `za_energy_market_signal_reference_2026_27.v0_1.json`.


## South African tariff and emissions profile — 2026/27

A versioned South African market-signal profile is added at:
- `ZA_ENERGY_MARKET_SIGNAL_REFERENCE_V0_1.md`
- `za_energy_market_signal_reference.schema.json`
- `fixtures/za_energy_market_signal_reference_2026_27.v0_1.json`

The new source owner is [ZA-ENERGY-SIGNAL-001 #4081](https://github.com/Luminous-Dynamics/symtropy/issues/4081).

### City Power Industrial TOU

The May 2026 City of Johannesburg Medium-Term Budget publishes a FY2026/27 industrial three-part TOU schedule. It is conditional: large consumers with supply above 100 kVA, application and City Power approval, and an ability to shift load. The schedule itself is identified as proposed; it must not be assigned to a site without its actual account/contract confirmation.

The published energy rates are ex VAT and exclude a separate 6 c/kWh network surcharge and 2% surcharge on business / large power users. It also has monthly service/network charges, demand charges, a minimum demand floor (maximum of measured kVA, 70 kVA, or 80% of the average of the three highest recorded demands in the preceding 12 months) and reactive-energy billing where PF is below 0.96. Thus the control benchmark must optimize the actual billing function, not just a list of time-of-use kWh rates.

The budget's weekend time-band text contains an apparent typo ("18h00-20h0" on Saturday/public holidays). It is preserved as a source ambiguity and requires City Power / invoice clarification for billing-grade use.

### DFFE 2024 Grid Emission Factors

DFFE Government Notice 7891 (11 Sep 2026) publishes 2024 annual factors: DGGEF 0.994, NGGEF 0.960, TLGEF 0.021 and DLGEF 0.071 tCO2e/MWh. The report identifies NGGEF for consumer Scope 2 and the loss factors for Scope 3; it also states an approximately two-year data availability lag. The NGGEF is an annual location-based factor—not a real-time or marginal emissions signal. A controller may use it for annual emissions accounting, but must not use it to claim interval carbon-intensity optimization.

### South African greenhouse simulation comparator

Ren et al. (2026), DOI 10.1016/j.seta.2026.105320, compares MEC, MECo and MTC on a Pretoria-weather greenhouse simulation. It reports MTC total cost 39.41% below MECo in winter and 34.69% below MECo in summer; winter energy use 13.74% below MECo. The paper's groundwater penalty is a modeled weighting, its grid factor is 0.879 kgCO2e/kWh, and SCC is R866/tCO2e. Those are study-specific assumptions and must remain separate from the later DFFE NGGEF, current tariff invoices, real groundwater prices and statutory carbon-tax claims.

### Evaluation rule

Every controller comparison must report:
- kWh and peak kW/kVA;
- time-of-use energy charges;
- fixed service/network charges;
- demand charge / ratchet;
- reactive-energy charge, surcharges and VAT;
- total invoice-equivalent cost only when the applicable contract is confirmed;
- annual Scope 2 emissions using the correct DFFE NGGEF boundary;
- interval/marginal emissions only when a separate qualified interval dataset exists;
- harvest, quality, whole-facility energy, water and climate service from #3957.

A cheaper schedule is not necessarily more energy-efficient or lower-carbon. A carbon accounting factor is not automatically a dispatch signal.
