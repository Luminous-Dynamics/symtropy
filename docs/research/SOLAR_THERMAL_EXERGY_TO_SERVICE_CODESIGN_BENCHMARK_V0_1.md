---
title: Solar–Thermal Exergy-to-Service Co-Design Benchmark
version: 0.1
status: proposed-experiment-contract
scope: solar capture, thermal/electrical storage, heat transfer, conversion, dispatch, degradation, and repair
owner: research/energy-systems
claim_boundary: no benchmark run, engineered optimum, cost claim, or hardware validation is asserted
---

# Solar–Thermal Exergy-to-Service Co-Design Benchmark v0.1

## 1. Research question

Can Symthaea, using Symtropy as a deterministic and physically bounded engineering environment, discover **better whole-system solar-energy architectures and operating policies** than strong conventional design and control baselines?

The system boundary begins at incident solar resource and includes conversion, storage, transfer, useful end-use delivery, parasitic demand, maintenance, degradation, and end-of-life assumptions. The output is not merely generated electricity or stored heat: it is the amount of required service delivered at the required temperature and time, with a defensible lifecycle cost and evidence trail.

This is a falsifiable engineering question. No result from this protocol alone would establish that Symthaea is better than state-of-the-art engineering, or that a simulated design is physically safe or commercially viable.

## 2. Why the field needs a system-level benchmark

Solar thermal and thermal storage research spans distinct subsystems: solar field or collector, receiver, heat-transfer fluid, storage medium and vessel, heat exchangers, conversion cycles, and dispatch. NREL's System Advisor Model maintains CSP models for multiple configurations and references published CSP modeling and validation work. DOE also describes industrial solar heat pathways using concentrating solar-thermal systems and photovoltaics paired with resistive heaters or heat pumps.

The integration problem is bigger than optimizing any one component. A high-efficiency collector can be a poor choice when its temperature profile, storage medium, transfer losses, parasitic loads, maintenance burden, or final demand are mismatched. Thermal-storage economics also depend on installed capital cost, energy prices, operating schedules, and the use case; recent techno-economic work on industrial thermal storage found that profitability depends materially on cost reductions and assumptions about electricity-price arbitrage.

The benchmark should therefore compare complete feasible paths from resource to service rather than select a technology by nominal component efficiency.

## 3. Paradigm shifts to test

### 3.1 Useful service first, not gross kWh first

For every demand, specify the service, required delivery temperature and pressure where relevant, power/heat profile, allowable interruption, and quality limits. Rank designs by whether they reliably satisfy the service profile and by the cost and material burden of doing so.

A kWh of heat at 55 °C and a kWh of heat at 500 °C are not equivalent products. Report delivered heat by temperature band and end use. Report electricity, thermal energy, and exergy separately; never add them into one dimensionally ambiguous total.

### 3.2 Co-design the entire chain

Optimize capture, aperture/collector geometry, receiver, working fluid, transfer network, storage, conversion equipment, heat-pump lift, controller, and maintenance policy together where the model permits. Compare co-design with a component-by-component workflow to test whether system interactions create measurable value.

### 3.3 Match temperature grade and cascade heat

Test whether higher-grade heat can serve the hottest compatible load first, with downstream heat reused at lower temperatures. Explicitly model heat-exchanger approach/pinch constraints and thermal losses. Don't assume a cascade exists merely because two components have different nominal temperatures.

Compare direct heat delivery, heat-pump upgrading, electricity generation followed by electric heating, and hybrid layouts at identical service boundaries. The best pathway depends on the temperature level, ambient conditions, equipment performance curves, and load timing.

### 3.4 Storage is a service-delivery system, not just a capacity number

Compare storage technologies on deliverable temperature, charge/discharge power, thermal stratification or mixing where relevant, standing loss, usable state-of-charge range, cycle stability, charging/discharging limits, containment and balance-of-plant cost, repairability, and how often storage produces useful service.

Candidate classes include:
- sensible heat in water, rock, concrete, or packed beds where material and operating conditions permit;
- latent heat storage using phase-change materials;
- thermochemical storage when reaction kinetics, cycling stability, containment and system complexity are supported by evidence;
- molten-salt or particle storage for compatible higher-temperature applications;
- electrically charged thermal stores using resistance heaters or heat pumps where tariffs, grid constraints and required temperatures make them relevant;
- batteries for electricity needs that thermal stores cannot satisfy.

These are alternatives to evaluate, not an endorsement of any one technology.

### 3.5 Co-optimize design and dispatch under uncertainty

Use weather forecast errors, cloud transients, soiling, varying demand, component outages, price uncertainty, and storage degradation. Compare a fixed rule, a transparent optimizer or model-predictive controller, and a Symthaea-informed policy under equal information, actuator limits and decision budgets.

Test robust and distribution-shift performance rather than optimize only a typical meteorological year. Preserve common weather traces across paired policy comparisons.

### 3.6 Make the system repairable by design

Treat cleaning access, replaceable absorber and insulation modules, pumps/valves, seals, sensors, corrosion inspection, fluid replacement, spare parts, and safe isolation as design variables. Report replacement frequency, maintenance labor, material replacement, downtime, embodied burden where data exist, and time to restore service. Avoid assuming a nominally efficient technology is practical when its servicing needs are prohibitive.

### 3.7 Turn the model into an active scientist

Symthaea can propose the next simulation or measurement that most reduces decision-relevant uncertainty, not merely search a fixed design space. Test expected value of information per unit time and cost using a held-out evaluator.

This requires separating epistemic uncertainty (poorly known model parameters) from real variability (weather and demand). The agent must not get evaluator-only ground truth. A simulation recommendation must say which measurement could disconfirm the preferred design and what decision changes if its result differs.

## 4. System alternatives

At minimum, compare the following at a common site, weather, service and reliability boundary. Unsupported options should be marked out of domain, not assigned optimistic guessed performance.

1. Reference PV plus inverter and electrical storage, where the load needs electricity.
2. PV plus resistance heating plus thermal storage.
3. PV plus heat pump plus thermal storage, with measured or cited COP/performance maps across source temperature, sink temperature and operating envelope.
4. Non-concentrating solar thermal for temperature ranges it can credibly serve.
5. Concentrating solar thermal (for example, trough, linear Fresnel or tower where geometry, direct normal irradiance, scale and temperature fit the use case).
6. Solar thermal plus layered sensible/latent/thermochemical storage when relevant data exist.
7. Hybrid architectures that assign electricity and multiple heat grades to different compatible loads.
8. Conventional reference supply and control appropriate to the use case, used as a cost/service comparator—not a misleadingly weak straw baseline.

For each architecture, include a no-storage comparator unless physically inapplicable. The design search may discover other options, but new model classes must pass their own verification cases before entering a confirmatory contest.

## 5. Mathematical contract and accounting

All model inputs and reports must carry units and a declared boundary. Convert temperature to kelvin for thermodynamic calculations. State whether each energy value is instantaneous power, interval-integrated energy, stored energy, or annual cumulative energy.

### 5.1 Solar resource and capture

Keep global horizontal irradiance (GHI) and direct normal irradiance (DNI) distinct. Use DNI for concentration models where appropriate; do not substitute GHI into a concentrator model. Include incidence-angle / optical effects, shading or cosine effects where appropriate, soiling, collector/receiver thermal loss, start-up, tracking and pumping parasitics, and temperature-dependent operating limits according to the chosen reference model.

Use versioned weather files and immutable source hashes. Distinguish measured weather, typical meteorological year inputs, forecasts provided to the controller, and hidden realized weather used by the evaluator.

### 5.2 Useful thermal delivery

For each interval, enforce:

`Q_collected = Q_to_storage + Q_direct_use + Q_dumped + Q_collector_losses`

This simplified boundary is valid only when the report defines the measurement planes and storage charge/discharge heat is not counted twice. If a transfer network lies between these planes, include its inventory and losses explicitly.

For every heat exchanger, enforce energy balance and a positive, configured minimum approach temperature; verify flow/temperature bounds and the available heat-capacity rates. Calculate delivery at the load-side measurement plane, not at the collector outlet.

### 5.3 Storage

For a lumped sensible store, a minimal model may use:

`E_store = m * integral(c_p(T) dT)`

and interval balance:

`E_next = E_now + Q_charge_in - Q_discharge_out - Q_standing_loss`

Use temperature-dependent material properties if the reference data support them. A single lump is not appropriate when stratification, thermocline movement or spatial gradients materially change the output temperature; select and verify a layered or distributed representation then.

For PCM or thermochemical systems, use declared enthalpy/reaction-state and cycle-stability models supported by data. Do not reuse a sensible-heat equation as if it captured phase change, kinetics or degradation.

### 5.4 Electricity and conversion

Account separately for electrical bus balance, storage conversion losses, inverter/motor/pump parasitics and end-use conversion. A heat pump must use an explicit performance map or qualified approximation across relevant source/sink conditions; do not model COP as a constant if it decides the ranking.

If heat is converted to electricity, report the cycle and heat-rejection boundary and all auxiliary loads. Do not compare gross cycle output to net delivered electricity.

### 5.5 Exergy and second-law checks

Report useful thermal energy and exergy separately. For a finite heat transfer at a constant boundary temperature (T) into an environment at (T_0), a reference relation is:

`Ex_heat = Q * (1 - T0 / T)`

with absolute temperatures. For non-isothermal streams, variable heat capacities, phase transitions and chemical storage, compute exergy using the appropriate enthalpy/entropy or chemical-potential model and clearly state the approximation. The constant-temperature expression is not a universal formula for every store.

Check that the system's exergy balance is consistent with the selected boundary, transfers, work, and irreversibilities. Never claim an efficiency above its declared theoretical boundary based on a unit, sign or double-counting error. If an uncertain material property controls the result, surface the uncertainty rather than report unjustified precision.

### 5.6 Lifecycle and practical constraints

Report installed-cost breakdown, replacement and maintenance schedule, financing/price assumptions, fixed/variable operating cost, lifetime, discounting method, residual value, and cost of useful delivered service. Keep embodied emissions and critical-material burdens separate if data quality is too weak for reliable aggregation.

Include land/aperture, water for cleaning/cooling, corrosion and compatibility, pressure limits, thermal cycling, freeze/overtemperature protection, safe shutdown, leak consequences, noise where relevant, and available skills/supply chain as explicit constraints or evidence fields.

## 6. Decision variables

Version the searchable design space. Initial bounded variables may include:

- technology and permitted component combinations;
- aperture/collector area and optical parameters within a validated range;
- receiver and heat-transfer-fluid operating envelope;
- storage capacity, power, temperature band, geometry class, insulation assumptions, and allowable state range;
- heat exchanger conductance / area and declared minimum approach temperature;
- pipe length/diameter and pump flow, with pressure-drop/pumping calculations when implemented;
- heat-pump sizing and temperature-lift envelope;
- thermal/electrical dispatch rules, reserve margins, priority tiers and safe fallback;
- maintenance intervals and replacement policy.

Do not allow unconstrained parameter search to generate physically impossible materials, temperatures, pressures or efficiencies. A proposed geometry or material outside model validity must be blocked for evidence, not extrapolated silently.

## 7. Baselines and causal experiments

Use the same site/time traces, equipment bounds, service profiles and objective definitions for every deployable candidate.

Required baselines:

- fixed transparent dispatch rules;
- greedy solar self-consumption / immediate-use policy;
- conventional constrained optimization or model-predictive control using the same physical model and information;
- a reference technology model where available, such as NREL's System Advisor Model (SAM) for its supported CSP configurations;
- small cases with exhaustive enumeration or an exact optimizer as an upper/lower bound;
- the best non-Symthaea design found under the same engineering budget;
- Symthaea-informed design search and/or dispatch with an independently constrained simulator/validator.

Do not grant Symthaea richer forecasts, extra search evaluations, hidden realized weather, greater actuator authority, or a weaker baseline model. Account for differences in compute and expensive simulator calls.

### Mandatory ablations

- remove Symthaea design search while keeping its chosen starting design and evaluation budget fixed;
- remove future forecasts or replace them with a predeclared forecast baseline;
- remove memory/history while keeping current observations;
- remove temperature-cascade awareness;
- replace a learned state/coupling score with a constant or shuffled negative control;
- replace uncertainty-aware selection with nominal-value selection;
- disable active experiment selection while keeping the number/cost of simulator calls fixed.

If an ablation fails to remove the claimed gain, narrow the explanation. If a simple baseline wins, preserve the result and use it to redirect research.

## 8. Benchmark corpus and splits

Use versioned cases with an explicit validity scope.

**Tier 0 — analytic fixtures**
- one collector with hand-calculable incident, reflected/optical, useful, and loss terms;
- one single-node store with closed-form energy balances and standing loss;
- one two-temperature cascade where energy conservation and the minimum approach constraint can be checked by hand;
- one heat pump using a small declared COP map and a test outside the map that must block;
- negative controls for GHI/DNI swaps, Celsius/Kelvin confusion, double-counted charge/discharge heat, over-unity efficiency, negative inventory, and impossible heat-exchanger approaches.

**Tier 1 — reference-model reproduction**
- Reproduce selected published NREL SAM / SolarPILOT cases in their supported domains with pinned versions, inputs and outputs.
- Where public empirical data exist, compare hourly/daily/monthly energy and key operating variables against field measurements, stating the measurement uncertainty and source model validation scope.
- A SAM match is a model-to-model check, not independent validation of an innovation or a new component outside SAM's model class.

**Tier 2 — annual and transient co-design**
- Annual weather/demand traces to examine seasonality, solar fraction, useful heat, storage sizing and service reliability.
- Separate transient windows for cloud passages, start-up/shutdown, thermal gradients, demand steps, defocusing, backup switching and fault recovery.
- Multiple sites and weather regimes; keep site/weather families or years held out from design selection.
- Stress cases for forecast error, soiling, material-property uncertainty, pump/valve failure, sensor bias, stale telemetry and unavailable spare parts.
- Paired runs across the same realized weather/fault histories.

**Tier 3 — hardware or field comparison**
- Calibrated test loop or instrumented pilot with approved safety envelope, calibrated sensors, thermal/electrical energy measurements and component inspection.
- Predeclare the comparison period, uncertainty propagation, shutdown behavior, missing-data rules and maintenance recording.
- No simulated result qualifies a new material, pressure vessel, receiver or high-temperature installation for operation.

A larger seed count inside one weather family does not replace diversity in sites, seasons, component types or physical regimes.

## 9. Outcomes and benchmark targets

Report a Pareto table instead of one opaque score. Primary metrics must be predeclared for each end use.

### Service and thermodynamics
- fraction of required heat demand met, stratified by load and temperature band;
- unmet useful heat in kWh_th, heat-deficit hours, longest interruption and critical-load service;
- net electricity delivered or consumed, reported separately from heat;
- annual useful solar fraction, dump/curtailment fraction, parasitic energy and start-up losses;
- exergy delivered at the load and declared exergy destruction / loss budget;
- storage deliverable energy at each required temperature, round-trip useful-heat efficiency, standing loss and discharge-temperature stability.

### Cost, materials, and maintainability
- levelized cost of useful heat (currency/kWh_th delivered at required condition) with input assumptions and sensitivity ranges;
- cost per unit of useful exergy where the end-use decision requires it;
- installed cost, annual O&M, replacement burden, downtime, lifespan assumptions and forecast error in replacements;
- water/material consumption, component count, critical materials and repair/inspection labor where evidence exists;
- degradation of storage capacity, conductivity, reaction enthalpy, corrosion allowance, optical reflectance or receiver performance where measured/modelled.

### Reliability and robustness
- service shortfall under held-out weather and fault families;
- recovery time after a fault, unsafe-command rejections, operation outside component limits, and fallback success;
- sensitivity to uncertainty in irradiance, demand, efficiency, heat capacity, thermal conductivity, cycling stability and equipment cost;
- p50/p95/p99 run time, memory, simulator calls and policy decision latency on pinned hardware.

### Candidate promotion gates

All following are proposed experimental gates, not standards or demonstrated results:

1. **Accounting:** 100% of accepted traces pass independent unit, energy and capacity checks; every seeded invalid trace is rejected.
2. **Determinism:** 100% replay agreement for deterministic fixtures at a pinned build, including manifest/report digests.
3. **Reference fidelity:** reproduce selected benchmark cases within published measurement/model tolerance; where none is specified, preregister the acceptable error band before testing.
4. **Practical Pareto improvement:** on the chosen primary objective, target at least 10% improvement over the strongest baseline under matched service, safety and cost boundaries; report 95% uncertainty intervals and reject promotion if the distribution of harm or another declared hard constraint worsens unacceptably.
5. **Shift robustness:** retain declared service floors on held-out site/year/weather and fault families; report worst-case and tail results, not only means.
6. **Ablation evidence:** the claimed Symthaea mechanism must contribute measurable incremental value versus the strongest non-Symthaea design/control search at equal budgets.
7. **Engineering cost:** include model-build, simulator-call, compute and human-review effort. A small gain requiring disproportionate engineering or unrealistic sensors is not automatically useful.
8. **External evidence:** hardware or field performance, component durability, safety and real cost remain separate gates requiring their own measurements and review.

A single impossible command, invalid balance, unacknowledged model extrapolation, or hard service-floor breach can block qualification even when averages are attractive.

## 10. Experimental procedure

1. Freeze one use case, service temperature profile, site/weather trace, model scope and lifecycle-cost boundary.
2. Verify the analytic fixtures before implementing the optimizer.
3. Reproduce at least one reference-model case within that model's validity domain.
4. Freeze baseline versions, equal budgets, primary metrics, exclusions and held-out cases.
5. Run a development pilot to discover bugs and estimate variance; never recycle pilot results as confirmatory held-out outcomes.
6. Run paired baselines and candidates on identical realizations and maintain a separate evaluator that recomputes accounting and outcomes from traces.
7. Run the preregistered ablations and negative controls; preserve every failed seed and unexpected outcome.
8. Measure the strongest candidate on held-out climates/load profiles and compare Pareto frontiers, not only one favored scalar.
9. Advance to a calibrated test loop only when uncertainty is low enough to justify the prototype and hazards have an approved safety plan.
10. Publish exact source/model/config/weather hashes, receipts, uncertainty and exclusions. Queued, skipped, stale, or missing CI is not validation.

## 11. Public data and reference resources

Use only sources appropriate to the modeled technology and version. Initial reference anchors:

- U.S. Department of Energy, **Solar for Industrial Processes**: explains CSP for direct process heat and PV paired with resistive heaters or heat pumps, plus low- and high-temperature applications: https://www.energy.gov/cmei/systems/solar-industrial-processes
- NREL / National Laboratory of the Rockies, **SAM CSP publications and model references**: supported configurations, CSP modeling references, and thermal-storage / electric-thermal-storage studies: https://sam.nlr.gov/concentrating-solar-power/csp-publications.html
- Wikoff et al. (2025), **Benchmarking Thermal Energy Storage Cost for Industrial Process Heat**, *Applied Energy* 402, article 126873, DOI: https://doi.org/10.1016/j.apenergy.2025.126873
- IEA Solar Heating and Cooling Task 62 reports, including **Mapping degradation of PCM and TCM** (September 2025): https://www.task62.iea-shc.org/reports
- DOE, **Thermal Storage R&D for CSP Systems**: historical research targets and technology directions; do not silently treat old program targets as current universal acceptance standards: https://www.energy.gov/cmei/systems/thermal-storage-rd-csp-systems

Record source version, licence/terms, units, calibration window, missing-data handling and provenance. Do not invent field inputs from climatology or literature summaries. Reference models are not truth: validate against physical data within the proposed system's regime.

## 12. Symthaea and Symtropy architecture

Keep Symtropy responsible for deterministic, bounded state transition and physics/ledger checks. Let Symthaea propose designs, identify uncertain assumptions, select experiments, generate candidate dispatch policies, and explain evidence for the decision. The independent evaluator should be a third path that recomputes service, cost and physics constraints from recorded inputs/actions; do not let the optimizer certify its own result.

An engineering result should contain:
- typed, unit-labelled design manifest and validity envelope;
- versioned weather/load/model/data manifests;
- candidate design, controller, action trace and all relevant simulator budgets;
- energy and exergy ledgers with explicit boundaries and residuals;
- material/degradation assumptions with cited evidence or an explicit unknown state;
- independent benchmark report, comparison receipts, uncertainty and all failed cases;
- the exact claim level: analytical, simulated, reference-model-reproduced, calibrated-hardware, or field-demonstrated.

No internal Symthaea metric—Φ or otherwise—can stand in for heat delivered, energy saved, service reliability, physical safety, or durability. It may be a research variable, but its practical value must be demonstrated by controlled comparison.

## 13. Implementation sequence

1. **Contract review:** choose a narrow first use case (recommendation: 80–150 °C process heat or domestic/commercial hot water, unless available data strongly favor another temperature band).
2. **Rust data model:** typed units and versioned manifests for irradiance, collector/receiver, loads, transfer links, stores and conversion.
3. **Closed-form reference fixtures:** one collector, one sensible store, one heat exchanger, one heat pump and the negative controls.
4. **Independent verifier:** recompute the balances, check units/bounds, and reject report tampering before optimizing anything.
5. **Baseline adapter:** pin an external reference model for relevant configurations or encode a small, independently checked first-principles reference model; don't pretend they are equivalent.
6. **Constrained design search:** start with exhaustive small spaces and transparent optimization, then compare Symthaea-informed search under the same evaluation-call budget.
7. **Temporal control:** add held-out annual weather/load traces, thermal stratification where necessary, degradation and fault/recovery scenarios.
8. **Active science:** evaluate the value of information of proposed measurements and select a laboratory test only if expected decision value justifies its cost.
9. **Calibrated prototype:** proceed only after model reproduction, sensitivity analysis and a domain-appropriate safety review.

This is a research contract. The next implementation must be a small, testable vertical slice—not a claim that a full engineering digital twin already exists.
