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


## South African tariff and grid-emissions reference — 2026/27

This section uses South African market data as a **local operating-cost and annual emissions-accounting comparator**, not as a universal site profile. Every run must bind to its actual electricity distributor, approved tariff, supply capacity, voltage, contract and reporting boundary.

### City Power industrial three-part time-of-use tariff

Primary source: City of Johannesburg, *Medium-Term Budget 2026/27–2028/29*, May 2026, printed pp. 96–97 (PDF pages containing sections 5.1–5.4):
https://joburg.org.za/documents_/Documents/2026-27-Final-Medium-Budget-Reports/2026-27-FINAL-BUDGET-BOOK.pdf

Eligibility and approval:
- This is a City Power tariff, not an Eskom or generic South Africa tariff.
- It is available only to qualifying City Power customers and is described as suitable for large consumers able to shift load away from peak and standard periods.
- Section 5.4.3 states that it applies to business consumers with supply capacity **exceeding 100 kVA**, and is available on application to qualifying consumers. A tariff change requires request and approval.
- Legacy demand-tariff agreements concluded before 1 July 1999 with demand below 100 kVA may continue under the stated grandfather clause. Do not infer that exception for a new site.
- Record the actual approved tariff on the supply account. Do not assume a CEA facility is eligible merely because its modeled load exceeds 100 kVA.

The published three-part TOU tariff rates are:

| Voltage class | Fixed service charge (R/month) | Network charge (R/month) | Demand charge (R/kVA) |
|---|---:|---:|---:|
| Low voltage (≤1 kV) | 2,444.32 | 2,185.32 | 461.28 |
| Medium voltage (>1–33 kV) | 3,360.96 | 9,347.51 | 431.11 |
| High voltage (>33 kV) | 42,915.09 | 43,848.71 | 400.93 |

The energy charges (c/kWh) are the same across the three voltage classes in this schedule:

| Season | Peak | Standard | Off-peak |
|---|---:|---:|---:|
| Summer (September–May) | 322.00 | 242.42 | 186.35 |
| Winter (June–August) | 766.24 | 292.56 | 200.44 |

Time bands (City budget section 5.4.7):
- Weekdays: peak 07:00–10:00 and 18:00–20:00; standard 06:00–07:00, 10:00–18:00 and 20:00–22:00; off-peak 22:00–06:00.
- Saturdays and public holidays: no peak; standard 07:00–12:00 and 18:00–20:00; off-peak 12:00–18:00 and 20:00–07:00.
- Sundays: standard 17:00–19:00; off-peak 19:00–17:00.

The tariff has material billing features that a scheduler cannot ignore:
- The monthly minimum demand basis is the greater of measured demand, 70 kVA, or 80% of the average of the three highest recorded demands over the preceding 12 months.
- Consumers with power factor below 0.96 are billed for reactive energy supplied above the stated 30% threshold relative to billing-period kWh; the published rate is 46.25 c/kVArh.
- Fixed service and network charges, voltage-specific demand charges, energy charges, the minimum-demand ratchet and reactive-energy charges are separate terms.

A schedule that shifts kWh away from expensive intervals can still fail to reduce the bill if its peak kVA ratchet, fixed/network charges or reactive-energy charges dominate. The benchmark must report both physical kWh/kW and the billed Rand components. Do not describe tariff arbitrage as energy efficiency.

The fixture contains arithmetic checks:
- winter peak/off-peak energy-charge ratio = 766.24 / 200.44 ≈ 3.82;
- summer peak/off-peak energy-charge ratio = 322.00 / 186.35 ≈ 1.73.

Those are tariff-rate ratios, not predicted customer bill savings.

### South African 2024 Grid Emission Factors

The Department of Forestry, Fisheries and the Environment published the *Report on South Africa's 2024 Grid Emission Factors* as Government Notice 7891 in Government Gazette 55353 on **11 September 2026**. Official notice:
https://www.gov.za/documents/notices/national-environmental-management-air-quality-act-report-south-africa%E2%80%99s-2024-grid

The report's 2024 values as transcribed by a secondary report of the gazetted release are:
- DGGEF (domestic generation): **0.994 tCO₂e/MWh**;
- NGGEF (national generation / consumption): **0.960 tCO₂e/MWh**;
- TLGEF (transmission losses): **0.021 tCO₂e/MWh**;
- DLGEF (distribution losses): **0.071 tCO₂e/MWh**.

The 2023 comparison values in the DFFE 2023 report were DGGEF 0.942, NGGEF 0.906, TLGEF 0.020 and DLGEF 0.062 tCO₂e/MWh. The 2024 NGGEF is therefore approximately 5.96% higher than the 2023 NGGEF on this annual location-based metric.

**Source-admission note:** the official DFFE 2024 gazette notice and PDF link are recorded, but the PDF endpoint returned an access error during this audit. The 2024 numeric transcription is cross-checked against a secondary report; it must retain that source limitation until the primary PDF/table is independently ingested. The 2023 comparison is source-pinned to the previous DFFE government report.

DFFE's prior grid-factor methodology describes NGGEF as the location-based factor to use, for instance, in consumer Scope 2 reporting; TLGEF and DLGEF serve distinct loss-accounting use cases. See the official 2022 DFFE report's Appendix A:
https://www.gov.za/sites/default/files/gcis_document/202411/51495gon5498.pdf

These are **annual** factors based on calendar-year generation / consumption information. They are appropriate to the annual location-based reporting method described by DFFE; they are not an hourly, 15-minute or marginal emissions signal. Do not use 0.960 tCO₂e/MWh to select which quarter-hour is cleaner. If a controller requires interval emissions optimization, it needs a separate time-aligned, geographically applicable, method-qualified hourly/marginal signal and uncertainty metadata. If such a signal is absent, the controller may optimize tariff or power demand but must not claim interval decarbonization.

### Frozen South African market-signal challenges

Include at least:
1. approved City Power industrial TOU vs ineligible customer;
2. non-City Power distributor; City tariff application rejected;
3. supply capacity exactly 100 kVA (not eligible under the >100 kVA rule) vs >100 kVA;
4. pre-July-1999 grandfather exception;
5. LV/MV/HV fixed/network/demand charges;
6. summer/winter boundary;
7. weekday / Saturday / public-holiday / Sunday band boundary;
8. minimum demand 70 kVA floor;
9. 80%-of-three-highest-demand ratchet retained after a one-off peak;
10. power factor 0.95 vs 0.96 and reactive-energy charges;
11. load shifting lowers energy charge but not billed demand;
12. low-price schedule increases total kWh;
13. annual NGGEF used for annual location-based Scope 2 reporting;
14. DGGEF mistakenly substituted for NGGEF;
15. TLGEF/DLGEF incorrectly added to NGGEF without a separate boundary;
16. annual NGGEF incorrectly used as intraday marginal signal;
17. 2024 GEF source PDF unavailable / value not independently admitted -> emissions claim marked SOURCE_LIMITED;
18. stale or wrong-geography carbon signal -> no interval carbon optimization claim;
19. tariff changes without effective-date/version update;
20. currency, VAT or tariff component omitted from total billed-cost report.

All tariff/emissions source values, dates, boundaries and verification limitations are captured in `za_energy_market_signal_reference_2026_27.v0_1.json`.

### Additional City budget billing layers and source-text defect

Annexure B of the same City budget (printed pages 78–81) describes the FY2026/27 proposed tariff schedule as **exclusive of**:
- the **6 c/kWh Network Surcharge**;
- the **2% surcharge on Business and Large Power Users**;
- VAT.

The budget's tariff-setting text says the 6 c/kWh Network Surcharge remains unchanged and applies to all customer categories, with residential customers exempt for the first 500 kWh/month. It separately states that a 2% surcharge is levied on business and large-power-user customers. The budget says the electricity tariff changes take effect from 1 July 2026. For a business/LPU CEA account, these additional charges need to be included according to the actual account/billing rules, but the 2% surcharge base and current VAT treatment must be confirmed from the account / billing documentation rather than guessed.

**Do not confuse three different line items:**
1. monthly network charge (R/month) in the three-part TOU tariff table;
2. the separate per-kWh Network Surcharge (6 c/kWh) excluded from Annexure B;
3. the energy charge (c/kWh), to which peak/standard/off-peak periods apply.

The Saturday/public-holiday standard band is printed as `18h00-20h0` in the budget PDF. The reference fixture normalizes the likely intended endpoint to 20:00 but flags it as `INFERRED_FROM_TYPO_REQUIRES_CONFIRMATION`. Do not promote this interval into a production tariff billing engine without City Power confirmation or the customer's approved tariff statement.

The machine-readable profile now contains the surcharge treatment, voltage classes, billing ratchet, source-text typo, account-eligibility caveat and an independent DFFE source-admission status. The latest DFFE 2024 report PDF returned an HTTP 502 during this audit; its values are retained as an official-notice-linked secondary transcription until the primary table is independently admitted.
