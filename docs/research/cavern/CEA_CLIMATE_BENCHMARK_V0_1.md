# CEA climate / heat / moisture / airflow benchmark v0.1

**Parent:** [CEA-CLIMATE-001 #4065](https://github.com/Luminous-Dynamics/symtropy/issues/4065)  
**Composes:** [CEA-FULL-BOUNDARY-001 #3957](https://github.com/Luminous-Dynamics/symtropy/issues/3957), [PHOTON-DELIVERY-001 #3985](https://github.com/Luminous-Dynamics/symtropy/issues/3985), [CAVERN-ANALOG-001 #4002](https://github.com/Luminous-Dynamics/symtropy/issues/4002), [CEA-CIRCADIAN-001 #4060](https://github.com/Luminous-Dynamics/symtropy/issues/4060).

**Status:** research protocol and literature-comparator register. It does not implement a climate solver, validate a model, size HVAC equipment or authorize physical operation.

## 1. Research question

Can a reduced-order or higher-fidelity climate model predict and control crop-zone service under changing light, transpiration, outside conditions and facility faults while keeping the *complete* electrical, thermal, moisture, water and CO2 flows inside a declared boundary?

The benchmark must be spatially resolved. A room-average temperature or RH setpoint is not evidence that every canopy layer has a safe microclimate.

## 2. Evidence architecture

Use four independent layers:

1. **Analytical reference layer** — psychrometric identities, sensible/latent heat balances, water-vapour and CO2 conservation, heat conduction and bounded thermal-storage references. These should be simple enough to hand-check and independently implement.
2. **Reduced-order process layer** — coupled multi-zone temperature, humidity ratio, CO2, airflow, crop transpiration, lighting loads, envelope and HVAC states. This layer is for rapid scenario and control evaluation.
3. **Spatial high-fidelity layer** — CFD / heat-transfer / porous-canopy models on selected geometries, with mesh and time-step studies and independent physical boundary data. Do not pretend every task needs CFD.
4. **Physical evidence layer** — calibrated multi-zone sensors, metered equipment, measured airflow, surface/rock heat flux, condensate flow and crop-stage records; reserve blind periods / compartments for validation.

Each model output must name its layer, evidence class, domain, validity envelope and claim ceiling. A simulation may challenge a measurement; it cannot overwrite it.

## 3. External state-of-the-art comparator register

The machine-readable registry is `cea_climate_comparators_2026.v0_1.json`. It includes six sources:

- 2026 dynamic control-oriented crop/climate model evaluated against seven independent datasets. It reports rRMSE below 10%, temperature RMSE 0.4 °C, RH RMSE 4.5 percentage points and CO2 RMSE 45 ppm. Crucially, it assumes a perfectly mixed chamber and constant AC COP; these are explicit gaps for our spatial, variable-load cavern case.
- 2026 CFD airflow comparison across 27 m² / 3-layer container and 162 m² / 6-layer building farm configurations. It reports temperature NRMSE 3.57%, velocity NRMSE 34.33%; a sidewall-fan case is simulated at 55% lower fan energy with uniformity 8% lower. The latter is not a metered facility saving.
- 2026 radiant cooling-panel mock-up with >96% horizontal and >95% vertical temperature uniformity. The study assumes cooling capacity is sufficient and did not operate its dehumidifier; it does not qualify latent control or condensation.
- 2025 five-minute transient crop/energy model with dynamic LAI, variable-COP and nine scenario cases. Its abstract and institutional record describe improved energy-estimation accuracy; publisher highlight wording also says cooling energy reduction. The register preserves this ambiguity and does not treat it as a universal saving.
- 2026 Canadian facility TRNSYS model compared to monthly bills, with MAPE 9.1%, CV(RMSE) 14.30% and NMBE -3.05%; its cost-optimised ventilation/heat-exchanger choice depends on electricity and CO2 price.
- 2025 mixed-use urban-farm EnergyPlus building simulation. The register retains whole-building and zoned energy shares separately and flags multiple aggregate EUI scopes; it is not a crop-zone baseline.

**Metric discipline:** RMSE, rRMSE, NRMSE, CV(RMSE), MAPE and NMBE are not interchangeable. A metric can only be compared when the target variable, sampling basis, denominator, aggregation interval and validation split match.

## 4. Canonical model contract

Every scenario declares:
- geometry, zone boundaries, rack/layer/canopy position and orientation;
- time basis, timezone / clock source, interval, epoch and synchronization error;
- weather and exterior boundary trace with source / station / file hash;
- crop/cultivar/stage, leaf-area index or justified range, density and transpiration method;
- spectral PPFD / photoperiod / schedule from #3985 / #4060, plus wall-plug lighting electrical metering;
- measured sensible and latent loads from lights, pumps, controls, people (if applicable), crop transpiration and equipment;
- HVAC equipment identity, rated capacity, measured performance map / COP, part-load and operating mode; unknown COP remains unknown;
- air supply / return / exhaust geometry, measured flow, fan curves, filtration pressure drop, leakage and infiltration;
- moisture sources / sinks: transpiration, humidification, dehumidification, ventilation, condensate, leaks and storage changes;
- CO2 sources / sinks: dosing, crop uptake proxy / model, ventilation and measured concentration;
- walls/envelope and rock boundary: conductivity, heat capacity, diffusivity, thickness, moisture, groundwater/seepage, surface heat flux and external/seasonal temperature;
- sensor calibration, range, uncertainty, time response, location and dependency / common-mode group;
- independent holdout declaration and permitted model-calibration inputs.

Electrical Wh, thermal Wh_th, water mass, humidity ratio, airflow, CO2 and temperature are distinct types. Do not convert between them using hidden coefficients.


### Measurement and verification alignment

Use recognized measurement methods as the instrument / M&V layer, while retaining CEA-specific environmental reporting:

- ANSI/ASHRAE Standard 41.1-2024: temperature measurement under laboratory and field conditions.
- ANSI/ASHRAE Standard 41.2-2026: air velocity and airflow measurement, including density effects and airflow-mixing considerations.
- ANSI/ASHRAE Standard 41.3-2025: pressure measurement.
- ANSI/ASHRAE Standard 41.6-2021: humidity measurement.
- ANSI/ASHRAE Standard 41.7-2025: gas-flow measurement, where the measured stream falls within its stated scope.
- ANSI/ASHRAE Standard 41.11-2023: power measurement for HVAC/R systems and components.
- ASHRAE Guideline 14-2023, including Addendum a: measurement and verification of energy, demand and water savings using pre/post billing or meter data. It does not prescribe instrument metering standards.
- ICCEG / NCERA-101: crop-growth-room, greenhouse and controlled-environment plant-experiment reporting guidance: https://www.controlledenvironments.org/reporting-guidelines/

Official ASHRAE titles/purposes/scopes: https://www.ashrae.org/technical-resources/standards-and-guidelines/titles-purposes-and-scopes  
Guideline 14 addenda: https://www.ashrae.org/technical-resources/standards-and-guidelines/standards-addenda

These are method-alignment references, not a claim that the prototype is ASHRAE-certified or that one standard covers every CEA measurement. For every sensor / meter, record the applicable method, calibration status, range, uncertainty, response time, placement, time synchronization and traceability. Apply the method only within its stated scope, and document any adapted method or deviation. CEA biological variables (cultivar, crop stage, canopy location, PPFD/DLI, nutrient solution and harvest state) still need their own crop-experiment reporting contract.

## 5. Required conservation checks

For every time interval and zone, independently recompute:
- sensible energy storage change = inputs - outputs within declared numerical/measurement tolerance;
- water-vapour storage change = vapour sources - sinks - exhaust / leakage, separately from liquid condensate;
- condensate production = collected + uncollected / drain loss + storage change;
- CO2 storage change = injected + incoming - outgoing - declared biological uptake / other sinks;
- fan electric power, airflow and pressure rise consistency where curves / measurements are available;
- envelope and rock heat flux sign convention and integrated energy;
- facility meters = the sum of declared submeter streams plus explicit residual / unmetered loads.

Missing streams must cause INCOMPLETE / NEEDS_EVIDENCE; they are not zero. Do not add electrical Wh to thermal Wh_th. Do not count latent energy in both the moisture and sensible ledger.

## 6. V0.1 frozen cases

Create at least 30 deterministic fixtures, including:
1. Dry-air psychrometric state conversion and dew-point reference.
2. Humidity-ratio conservation across a mixing junction.
3. Single-zone steady sensible-heat balance.
4. Single-zone transient thermal-capacitance response.
5. Multi-zone air mixing with known exchange rates.
6. LED electrical input and independently measured heat-source partition.
7. Crop transpiration pulse as vapour/latent source.
8. Dehumidifier water removal, heat rejection and electrical load.
9. Condensate storage / drain / reuse closure.
10. CO2 injection and exhaust balance.
11. Ventilation cooling that saves compressor load but loses CO2.
12. Heat exchanger sensible/latent recovery with explicit bypass/leakage.
13. Measured COP vs rated COP vs assumed constant COP.
14. Part-load HVAC performance and mode switching.
15. Rack top/middle/bottom temperature stratification.
16. Rack-level canopy velocity and spatial stagnation.
17. Fan curve and duct pressure-loss sensitivity.
18. Crop-canopy drag / LAI sensitivity in CFD.
19. Radiant panel temperature distribution under LED-on transient.
20. Cold-surface condensation negative control.
21. Thermal bridge / local wall moisture excursion.
22. Wet-rock vs dry-rock thermal properties.
23. Rock heat conduction with known boundary pulse and finite geometry.
24. Groundwater / seepage moisture source with explicit drain capacity.
25. Seasonal external ground-temperature cycle and lag.
26. Infiltration / access-door opening transient.
27. Dynamic crop clock / 27 h treatment increases elapsed facility base-load time.
28. Dynamic spectrum schedule changes photon input and heat, while transpiration response stays measured/unknown rather than assumed.
29. Sensor bias, response lag and time synchronization offset.
30. Sensor dropout and common-mode calibration / power failure.
31. Perfectly mixed model vs spatial observations revealing layer gradients.
32. Reduced-order model vs CFD disagreement with OOD / abstention response.
33. Blind period after lighting / HVAC controller change.
34. Energy meter mismatch and unmetered subsystem load.
35. Missing rock moisture / groundwater data makes thermal-benefit conclusion indeterminate.

Each case contains known truth or source-pinned measurements and negative controls. The candidate model does not grade itself.

## 7. Experimental protocol for the terrestrial analogue

### Stage 0 — instrumentation and site characterization
Before any crop trials:
- independent site/geotechnical and electrical safety review;
- temperature / RH / CO2 sensors at reference points, rack levels and candidate rock surfaces;
- calibrated airflow / differential-pressure measurement with known uncertainty;
- separate meters for lighting, fans, HVAC, dehumidifier, pumps, CO2 and controls;
- wall/rock surface thermistors and heat-flux sensors at representative locations;
- condensate mass / flow collection and water-quality sampling;
- external/weather and ground boundary trace;
- sensor clock synchronization and raw-data retention.

Do not assume a mine/cave is dry, stable, safe, insulated or a useful heat sink. No occupied space or hazardous fault injection is authorized.

### Stage 1 — unoccupied climate cell
Use controlled load steps and passive observations within the reviewed equipment envelope:
- lighting on/off and ramp transients;
- fan speed / operating point changes within approved settings;
- HVAC/dehumidifier state transitions;
- measured humidification or benign moisture challenge only under reviewed procedures;
- condensate recovery, drain and storage checks;
- ventilation / CO2 exchange characterization;
- cold-surface and condensation monitoring.

### Stage 2 — crop compartment
Add crop transpiration and canopy resistance only after environmental and water instrumentation is calibrated. Record crop-stage and cultivar changes. Measure canopy microclimate, leaf/canopy temperature and water-vapour loads; do not infer transpiration from RH alone.

### Stage 3 — blind validation
Fit parameters on one declared time block / subset of compartments. Freeze the model and validate against an independent block, different crop cycle or compartment. Keep calibration and test raw files content-addressed. Do not tune after seeing blind results without creating a new model version and a new holdout.

### Stage 4 — rock / ground coupling
Only after site review and actual thermal / moisture characterization, estimate the rock boundary response. Include seasonal data or a validated transient boundary model. A low, stable wall temperature does not alone prove a net heat sink; integrated heat flux and storage recovery must close.

## 8. Metrics

**State prediction**
- zone/canopy temperature MAE, RMSE, bias, peak error and interval coverage;
- RH percentage-point error, humidity-ratio error and dew-point error;
- CO2 ppm error / bias;
- airflow velocity error by measured region and stated sensor method;
- wall/rock temperature and heat-flux error.

**Spatial climate service**
- time within crop-specific temperature / RH / VPD / CO2 envelope;
- layer-wise minimum/maximum and 5th/95th percentile, not only spatial mean;
- uniformity metric with formula and sampling geometry;
- condensation-margin false-safe / false-alarm rate;
- local excursion duration and cumulative time outside envelope.

**Energy and water**
- lighting, fans, HVAC, dehumidification, pumping, CO2 and control electrical Wh separately;
- useful/recovered/rejected thermal Wh_th separately;
- kWh per accepted edible kg and per nutrition service through #3957;
- fan kWh per airflow / pressure service with boundary stated;
- condensate kg recovered / purged / lost and water quality;
- peak kW and reserve requirement.

**Model quality / robustness**
- exact conservation residuals;
- parameter identifiability / sensitivity;
- blind temporal and spatial transfer error;
- uncertainty calibration / interval coverage;
- OOD detection and abstention;
- runtime / computational cost for control-relevant model;
- failure recovery and sensor-loss behavior.

No universal scalar score: preserve accuracy, service, energy, water, reliability and safety as distinct axes.

## 9. Baselines and competition

Compare on matched geometry, sensors, boundaries, calibration/holdout split and compute / measurement budgets:
- constant-COP and simple setpoint control;
- a well-mixed reduced-order model;
- multi-zone reduced-order model;
- validated CFD specialist baseline for spatial cases;
- direct AC discharge / duct / rack fan / aisle fan alternatives;
- convective vs radiant cooling where condensation and latent load are actively measured;
- fixed-COP vs equipment performance-map / measured COP;
- ventilation vs air-to-air heat exchange under identical CO2 and weather boundaries;
- incumbent human/operator procedure.

The leading 2026 comparators show why a single "SOTA number" is misleading: the control model has good mixed-chamber state accuracy but constant COP; the CFD model has lower temperature but substantially larger velocity error; the radiant-panel study demonstrates spatial temperature uniformity but did not test dehumidification; the Canadian model matches monthly energy bills but not sub-hour canopy microclimate. Our advantage must come from joint validation and boundary closure, not cherry-picking a number.

## 10. Advancement targets

After the instrument register, pilot data, and evaluation split are frozen, aim to:
- pass all analytical conservation and psychrometric golden cases;
- achieve temperature MAE ≤0.5 K and RH error ≤3 percentage points on blind representative zones, provided reference sensors support that uncertainty;
- beat the 2026 control-model comparator on matched zone/operating cases, not by using an easier aggregation or different metric;
- reduce velocity NRMSE below 20% on a declared geometry/sensor protocol as a stretch target, with measured uncertainty and spatial holdout;
- attain preregistered empirical coverage for predictive intervals on blind data;
- achieve zero false-safe condensation results on frozen cold-surface / high-dew-point negative controls;
- target ≥10% lower combined fan + HVAC + dehumidifier electricity at equivalent crop-zone climate service on a measured matched trial, without shifting load outside the boundary or worsening CO2, water quality, crop quality or safety;
- report Pareto alternatives where no architecture dominates.

These are proposed development targets, not demonstrated performance. If the measured reference system performs better, preserve that result and focus on the gap.

## 11. Claim ceiling

A passing reference, simulator or controller test qualifies only its declared analytical / synthetic / measured boundary. It does not certify equipment, establish that a cavern is a heat sink, guarantee crop yield, prove condensation safety, approve human occupancy or qualify a lunar/Martian habitat.
