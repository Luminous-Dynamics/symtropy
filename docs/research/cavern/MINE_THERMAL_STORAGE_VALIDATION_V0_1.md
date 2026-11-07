# Mine thermal storage evidence and field-validation protocol v0.1

**Parent:** [MINE-THERMAL-STORE-001 #4070](https://github.com/Luminous-Dynamics/symtropy/issues/4070)  
**Composes:** [THERMAL-DYNAMIC-001 #1916](https://github.com/Luminous-Dynamics/symtropy/issues/1916), [BROWNFIELD-UNDERGROUND-001 #1951](https://github.com/Luminous-Dynamics/symtropy/issues/1951), [CEA-CLIMATE-001 #4065](https://github.com/Luminous-Dynamics/symtropy/issues/4065), [CEA-FULL-BOUNDARY-001 #3957](https://github.com/Luminous-Dynamics/symtropy/issues/3957).

**Status:** source-pinned literature case, machine-readable observations and independent arithmetic checks. Not a thermal-storage solver, mine-site assessment, reinjection permission or physical operating plan.

## 1. Evidence that changes the research target

The 2026 Reiche Zeche study is a rare field-scale mine thermal energy storage (MTES) demonstration rather than only a conceptual storage simulation.

Source: *Underground thermal energy storage in mines: Field scale heating-cooling cycles and performance at the Reiche Zeche underground mine*, Journal of Energy Storage, DOI 10.1016/j.est.2026.122087.

Published summary:
- 3 controlled heat/cool cycles across a total of 504 days;
- 38.0 MWh heat supplied;
- approximately 90% of stored energy resided in the surrounding gneiss;
- rock warmed 10.1 K at 1.8 m depth after the hottest cycle;
- tracer-derived throughflow 79 L/h and residence time about 10.5 days;
- advective heat-loss coefficient 0.092 kW/K;
- Fe(II) oxidation and Fe-oxyhydroxide precipitation dominated exchanger fouling;
- hydrophobic coating limited conductance losses to about 18% in the tested context.

Source URL: https://doi.org/10.1016/j.est.2026.122087

The authors indicate raw data are available on request. The current fixture therefore admits only the published summary and hand-checkable derived arithmetic, not the underlying 504-day time series.

## 2. Important distinction: storage fraction is not recovery efficiency

"Approximately 90% of stored energy resides in the surrounding gneiss" describes the reported location of stored thermal energy. It does **not** mean:
- 90% of supplied heat is stored;
- 90% of stored heat is recovered;
- 90% of heat is delivered at a useful temperature;
- the system has 90% round-trip thermal efficiency;
- 90% of the mine void's nominal capacity is usable.

Keep at least five quantities separate:
1. injected / supplied heat;
2. temporary stored-energy state (rock and water, no double counting);
3. recovered heat at the exchanger;
4. delivered heat after pumping/distribution/heat-pump conversion;
5. useful service within the required temperature and time envelope.

The machine-readable reference case labels the 90% result as a source-reported storage-location fraction.

## 3. Independent arithmetic consistency checks

The source reports 79 L/h tracer-derived throughflow and 0.092 kW/K advective heat-loss coefficient.

Assuming nominal liquid water:
- density = 1,000 kg/m³;
- specific heat = 4.18 kJ/(kg·K).

Then the advective heat-capacity rate is:

`(79 L/h ÷ 1,000 L/m³) × 1,000 kg/m³ × 4.18 kJ/(kg·K) ÷ 3,600 s/h = 0.0916556 kW/K`

This is within approximately 0.38% of the reported 0.092 kW/K. It is a dimensional consistency check, not an independent reconstruction of the authors' fitted thermal/hydraulic model.

A second conditional check uses `V = Q × tau`:
- 79 L/h × 10.5 days × 24 h/day = 19,908 L = 19.908 m³.

This is an implied mobile-water volume only under a single-compartment / well-mixed residence-time interpretation. It is not a measured total mine-water inventory, mine void volume, or the rock fracture-matrix storage domain.

A third scale check:
- 38,000 kWh / (504 × 24 h) = approximately 3.1415 kW_th average over the full elapsed campaign.

This is not installed power, active-cycle average power, recovered power, peak power or delivered service. It divides aggregate heat supplied by total elapsed calendar time, including inactive periods.

## 4. External industry baselines

### Existing mine-water heat and cooling

Oppelt et al. (2025), *Mine Water as an Energy Source: Overview of Technical Basics, Existing Plants, and Monitoring Results*, DOI 10.1007/s10230-025-01057-w, reviews more than 51 existing mine-water heating/cooling sites. It reports approximately 85 MW heating and 20 MW cooling capacity worldwide, with seasonal performance factors up to 10 in a set of four monitored facilities using combined heating and cooling.

This is an industry-maturity comparator, not a matched performance target. System SPF depends on whether pumping, distribution, heat pumps, auxiliaries and both useful heating/cooling services are inside the numerator/denominator. Preserve the source boundary.

Source: https://doi.org/10.1007/s10230-025-01057-w

### Environmental trade-offs

Motamedi et al. (2026), *When heat meets water: Environmental trade-offs of underground thermal energy storage*, Journal of Energy Storage 149, article 120265, DOI 10.1016/j.est.2025.120265, reviews thermal, hydrogeochemical and microbiological groundwater impacts for ATES, BTES, cavern storage and flooded-mine systems. It identifies pH, dissolved oxygen, major ions, trace elements and microbial activity as important monitoring variables.

Source: https://doi.org/10.1016/j.est.2025.120265

### Heat integration and storage dispatch

Use OpenPinch / multiperiod pinch and existing Symtropy #1916 as the heat-integration and dispatch comparator. This issue adds site-specific field physics and monitoring evidence; it does not duplicate the optimizer.

OpenPinch review: https://doi.org/10.1007/s41660-026-00729-6

## 5. Field data and instrumentation contract

A future qualified field campaign should obtain synchronized, calibrated measurements for:

**Thermal**
- distributed rock temperatures at multiple depths and offsets;
- mine-water inlet/outlet/ambient temperatures;
- thermal flow and heat-exchanger inlet/outlet temperatures;
- thermal storage and boundary conditions;
- exchanger approach temperature and conductance over time;
- surface/rock heat flux at representative locations.

**Hydraulic**
- water level and pressure;
- measured throughflow with calibrated method;
- tracer injection/recovery protocol, breakthrough and residence-time distribution;
- recharge/inflow/outflow and hydraulic connectivity;
- pump power, flow, lift and pressure drop.

**Geochemistry / biology**
- pH, conductivity, dissolved oxygen and redox;
- temperature-conditioned Fe(II)/total iron, manganese and relevant site-specific species;
- alkalinity, major ions, trace metals and suspended solids;
- scaling, precipitation, corrosion and exchanger fouling coupons;
- microbial indicators when environmental risk assessment requires them.

**Energy and service**
- heat input / extraction, heat-pump electricity, pump/auxiliary electricity, distribution losses;
- useful heating/cooling delivered by temperature band;
- actual equipment COP / performance map and part-load state;
- operating windows, cycle state, maintenance downtime and service demand.

All sensors need calibration identity, uncertainty, timebase and spatial applicability. Missing streams remain missing; they are not zero.

## 6. Frozen benchmark corpus

The parent issue requires at least 30 cases. The initial frozen portfolio should include:
1. conductive rock-only transient storage;
2. water-only sensible storage;
3. rock/water coupled exchange;
4. low- and high-advection regimes;
5. tracer flow and residence-time distributions;
6. fracture/matrix connectivity;
7. finite rock boundary loss;
8. controlled heat pulse and energy closure;
9. source-reported 504-day MTES summary (raw data unavailable);
10. temperature-grade mismatch;
11. heat-pump upgrade with measured COP map;
12. pump lift and hydraulic loss;
13. distribution loss;
14. combined heating/cooling service and full system SPF;
15. seasonal imbalance and multi-year drift;
16. Fe(II) oxidation / Fe-oxyhydroxide fouling;
17. mineral scaling and corrosion;
18. dissolved oxygen/redox change;
19. coating and exchanger-conductance degradation;
20. hydraulic isolation vs throughflow;
21. contamination / groundwater-receptor connection;
22. reinjection chemistry and temperature limits;
23. missing hydrochemistry;
24. sensor drift / common-mode timebase error;
25. unmetered pump energy;
26. storage rock/water double count;
27. CEA load shift relative to storage availability;
28. seasonal thermal-boundary uncertainty;
29. outage / maintenance delay;
30. unknown rock properties causing NeedsEvidence;
31. apparent high stored energy but low recovered service;
32. non-storage ground-source heat-pump baseline;
33. hot-water tank baseline;
34. direct grid/boiler/chiller reference;
35. no-build / remediation alternative.

## 7. Comparison metrics

Do not collapse the following into one result:
- energy supplied, stored, recovered and delivered (MWh_th);
- storage location by rock/water domain;
- temperature grade and time availability;
- thermal recovery fraction and round-trip efficiency with explicit boundaries;
- advective throughflow, residence-time distribution and heat-loss coefficient;
- pump and heat-pump electricity (kWh_el);
- system seasonal performance factor (SPF);
- temperature-grade / exergy loss where calculation is justified;
- exchanger fouling, scaling, corrosion, pressure drop and conductance decay;
- water use, treatment, reinjection, purge and loss;
- hydrochemical / microbiological changes and environmental compliance;
- maintenance, availability and lifecycle costs.

A large reservoir heat capacity can coexist with low recoverable service if heat leaks away, temperature drops below useful grade, pumping is expensive or exchangers foul.

## 8. Claim ceiling

The source fixture reproduces only published summary facts and arithmetic consistency checks. It does not reconstruct raw observations, prove field-wide storage efficiency, qualify a specific mine, authorize pumping or reinjection, establish environmental safety, or demonstrate CEA/industrial service savings.
