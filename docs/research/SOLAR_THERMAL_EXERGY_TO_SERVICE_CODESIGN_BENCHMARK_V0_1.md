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

### 3.6 Treat high-temperature heat pumps and storage as a coupled machine

Add an explicit experiment family comparing direct high-temperature heat-pump operation with heat-pump-plus-thermal-storage architectures. The hypothesis is that time-shifting and thermally buffering operation can change feasible operating hours, temperature lift, and service continuity; this must be derived from a validated performance map and storage model, not assumed from a fixed COP.

Use identical service temperatures, load profiles, electricity-price traces, equipment bounds, and measurement planes. Report COP and net electricity separately from delivered steam/heat, storage charge/discharge losses, cycling and startup effects, delivered-temperature stability, useful service, installed cost, and lifecycle cost. Compare fixed dispatch, a constrained optimizer, and Symthaea-informed dispatch under equal observation and search budgets.

A 2026 Journal of Energy Storage study evaluates TES integration with high-temperature heat pumps and reports large changes under particular extreme conditions. Treat that as a research lead to reproduce, not a portable guarantee or a target the Symthaea result is allowed to assume. The benchmark should reproduce the paper's relevant scenario first, inspect sensitivity and boundary assumptions, then test held-out duties. If the effect disappears under realistic cost, temperature, or cycling constraints, report that null result.

### 3.7 Make the system repairable by design

Treat cleaning access, replaceable absorber and insulation modules, pumps/valves, seals, sensors, corrosion inspection, fluid replacement, spare parts, and safe isolation as design variables. Report replacement frequency, maintenance labor, material replacement, downtime, embodied burden where data exist, and time to restore service. Avoid assuming a nominally efficient technology is practical when its servicing needs are prohibitive.

### 3.7B Separate the storage medium from its heat-transfer architecture

Treat the storage medium and the mechanisms that move heat into/out of it as separate design choices. Concrete/rock/sensible storage with embedded thermosiphons or heat pipes, and latent stores with heat-pipe-assisted charging/discharging, should be compared only when the component models are supported by data. These architectures can change power density, temperature uniformity, cycling rate and usable discharge temperature even when nominal storage capacity is unchanged.

The Lehigh Thermal Battery provides a concrete reference case: a 150 kWh-thermal concrete-based store with embedded thermosiphons was tested in a relevant environment. The university reports a media-level charge/discharge efficiency above 95% for specified tests and an end-to-end thermal-battery efficiency around 70% for another stated system boundary. Do not merge those figures: the measurement boundary changes the result. Reproduce the source case, identify the exact input/output measurement planes, and report both media-level and whole-system performance when the required measurements exist.

The open-access 2026 dynamic-model study for a heat-pipe-integrated latent store reports a Modelica model compared with experiments, with prediction errors varying by measured output and simulation horizon. It is a good example of how the benchmark should preserve time-series residuals and validation envelopes rather than treat a model as uniformly accurate.

### 3.7C Accept multiple heat sources and independently scheduled outputs

Test a source-agnostic store that can accept heat from direct solar thermal, PV-powered resistance heating, a heat pump, recovered process/data-center heat, or compatible waste heat—only within a declared temperature and material envelope. The store's charging inputs and useful heat/electricity outputs must be separately metered in the model; do not count the same joules once as source generation and again as stored heat.

The scheduling question becomes: which source should charge which store, at what temperature, and when should each output be used? Account for source opportunity cost, source-temperature compatibility, heat-transfer limits, storage degradation, and the value of supplying a particular load at a particular time. Compare this multi-source design against one-source baselines and a simple rule-based dispatch under matched weather, tariffs, and demand.

The goal is not to assume that “hybrid” always wins. Test whether independent charge/discharge paths and source flexibility improve delivered service or lifecycle cost after full parasitic and conversion losses.

### 3.7D Target process integration before storage and collector sizing

Add pinch-style source/sink integration as a first-class stage, rather than estimating storage size from gross solar surplus alone. Represent each hot and cold stream with supply/target temperatures, interval-integrated duty, availability window, and an explicit minimum approach. Compute minimum external hot/cold utility targets and pinch temperatures before comparing storage or heat-pump options.

The first Rust pinch implementation is intentionally bounded to static, sensible, non-isothermal streams with constant integrated heat capacity. Its hand-checkable fixture is:
- Hot stream: 423.15 K to 323.15 K, 100 kJ available.
- Cold stream: 393.15 K to 493.15 K, 120 kJ required.
- Minimum approach: 20 K.
- Expected minimum hot utility: 110 kJ; minimum cold utility: 90 kJ; maximum internal process heat recovery: 10 kJ.
- Expected shifted pinch: 403.15 K; hot-side pinch 413.15 K; cold-side pinch 393.15 K.

These targets are thermodynamic bounds, not a synthesized exchanger network or guaranteed realized savings. Negative controls must reject reversed hot/cold streams, zero or negative duties, invalid temperature shifts, temperature-approach violations, and manipulated target receipts.

Next, extend from static targeting to a time-expanded heat network where each period carries storage state, heat availability, process demand, equipment rate limits, source identity, and temperature grade. Heat may be carried from one time step to another only through a modeled store with capacity, standing-loss, charge/discharge-power and admissible-temperature constraints. Optimizing a time-average alone is insufficient when solar and process schedules are mismatched.

A useful external reference is Falch, Espen, and Anantharaman (June 2026), who describe pinch-informed storage integration in multiperiod heat-exchanger-network synthesis and report 90% solar-thermal utilization at 250 kWh of storage versus 70% without storage in one lubricant-plant case. Those values are specific to that plant and paper boundary; capture and reproduce its actual data and assumptions before comparison: https://doi.org/10.69997/sct.176470. OpenPinch (March 2026) provides a broader open implementation of static advanced pinch analysis and total-site integration, and identifies dynamic/multi-period analysis as an extension direction: https://doi.org/10.1007/s41660-026-00729-6.

The reference-access boundary matters: the public five-page article reports the operating pattern (16 hours/day, five days/week), a month-long measurement campaign, three solar-production periods per day, no weekend charging, and 18.9 MWh modeled production. It states an ideal, lossless, constant-temperature, cyclic store. The public article text does not include the full interval trace and stream-level input table needed to reconstruct the exact case. Until an author-published supplement or repository is pinned, classify this as **aggregate-anchor / exact-reproduction-blocked**, not reproduced. First implement a reference-compatibility experiment for the ideal model; then separately study losses, finite temperature range, transfer power, and economics. Never grade the realistic model against the ideal paper results as though the boundaries match.

### 3.7E Start with auditable time-expanded state, not a prematurely clever controller

The temporal implementation is available in `research/solar-thermal-kernel/src/temporal.rs` on [PR #1853](https://github.com/Luminous-Dynamics/symtropy/pull/1853). It executes an ordered list of periods, applies the declared standing-loss model before each action, allows one explicit idle, charge, or discharge action per period, and independently recomputes state transitions plus the whole-schedule energy ledger. The analytic two-period day/night fixture is included in the emitted flat-scalar JSON receipt, and the standalone auditor independently recomputes its period temperatures, charge/discharge ledger, losses, grade gate, service and first-law closure. An explicit idle action is now available to general Rust schedules and is covered by a Rust test; the fixed receipt remains the two-period charge/discharge fixture. This remains a fixed fixture-specific audit rather than a general schedule-trace parser, and is not an optimizer.

The canonical analytic day/night fixture has a 10 kg store with constant 1,000 J/(kg K) heat capacity, 300–400 K range, 900 kJ offered daytime charge, 0.1 W/K standing-loss conductance, a 3,600 s interval at 300 K ambient, and a 300 kJ useful nighttime request at 340 K with 10 K approach and 90% exchanger effectiveness. Under these explicit synthetic assumptions, it accepts 900 kJ, loses 32.4 kJ during the next interval, delivers 300 kJ useful heat, reports no unmet service, and closes the store-energy ledger to numerical tolerance. Those figures are fixture expectations only—not field-performance estimates.

Next, extend this state layer without skipping falsification: grow the fixed receipt schema into a versioned schedule-trace format; seed negative controls for duration/power mismatch, capacity overflow, incorrect temperature-grade transfer, tampered or reordered periods, duplicated energy, and inconsistent state; then connect per-period pinch targets to storage state and utilities. The time-expanded graph must preserve energy grade, source provenance, interval duration, rate constraints, and service timing. A controller is not allowed to certify its own result.

### 3.8 Turn the model into an active scientist

Symthaea can propose the next simulation or measurement that most reduces decision-relevant uncertainty, not merely search a fixed design space. Test expected value of information per unit time and cost using a held-out evaluator.

This requires separating epistemic uncertainty (poorly known model parameters) from real variability (weather and demand). The agent must not get evaluator-only ground truth. A simulation recommendation must say which measurement could disconfirm the preferred design and what decision changes if its result differs.

### 3.9 Make fidelity adaptive to the decision

A fixed-fidelity model forces a bad trade: either every candidate is expensive to simulate, or every candidate inherits the limitations of a fast simplified model. Instead use a promotion ladder: analytic/lumped screening, dynamic reduced-order dispatch, a richer stratified or multi-node model when temperature grade and fast draw matter, then a pinned external reference or experiment for finalists. The cheap model must declare its validity envelope; outside that envelope, escalate rather than extrapolate silently.

The goal is not maximal simulation fidelity everywhere. It is **decision quality per unit of evaluation budget**. Pre-register output-specific tolerances and report false-screen rate (high-fidelity winners discarded early), false-promotion rate (cheap winners failing the higher-fidelity check), ranking flips, compute time, memory and evaluator calls. A proposed initial engineering target is at least 10× faster screening on a pinned corpus while every decision-critical output stays inside its declared error budget; this is a test target, not achieved performance. See [THERMAL-FIDELITY-001](https://github.com/Luminous-Dynamics/symtropy/issues/1954).

### 3.10 Optimize for tail service risk, not mean performance alone

Expected cost or average useful heat can hide rare but serious failures. The benchmark must treat critical-service floors as hard constraints and report the upper tail of unmet-service fraction and operating-cost regret separately from the mean. Scenario-based stochastic, robust/worst-case, risk-aware CVaR, deterministic forecast, rule-based and perfect-foresight policies must receive the same realized scenarios and appropriate information at decision time. Perfect foresight is an upper bound only; it must never be presented as an online controller.

Preserve chronology and correlation among irradiance, ambient temperature, load, tariffs and faults. Separate forecast inputs available to the controller from held-out realized conditions used only by the evaluator. Include cloudy/ramp events, seasonal demand shifts, component outages, sensor dropouts, forecast bias and storage degradation. Do not let scenario reduction remove rare service-critical events without a documented coverage audit.

Report expected service shortfall, worst-case shortfall, weighted P95 and CVaR95 of demand-normalized unmet heat, probability of critical-service-floor violation, longest consecutive deficit, reserve-floor excursions, fallback activations and tail regret against a held-out oracle. Define the weighted quantile and CVaR convention explicitly and test it on small hand-calculable distributions, including ties and fractional tail mass. A service-floor breach cannot be compensated by a lower average cost.

This direction is supported by published CST/TES research comparing stochastic dispatch with deterministic and heuristic strategies, and newer risk-aware MPC studies that explicitly measure dispatch deficit and upper-tail scenario risk. Their quantitative gains are plant- and objective-specific; they are hypotheses for a matched-boundary benchmark, not targets to copy blindly. See [THERMAL-RISK-001](https://github.com/Luminous-Dynamics/symtropy/issues/1969).

The current kernel receipt now includes a hand-calculable three-scenario fixture that checks the metric contract independently of controller logic: probability weights 0.80 / 0.17 / 0.03; 100 J requested in each scenario; 100 / 80 / 20 J delivered; 0.80 minimum service fraction. It yields 5.8 J expected unmet heat, 80 J worst unmet heat, 0.058 mean normalized shortfall, P95 shortfall 0.2, CVaR95 0.56 and 0.03 service-floor violation probability. The CVaR definition uses exactly the worst 5% probability mass, fractionally selecting mass at the boundary. This fixture validates arithmetic and receipt tamper controls only; it does not validate scenario likelihoods or controller performance. See the separate kernel implementation in [PR #1853](https://github.com/Luminous-Dynamics/symtropy/pull/1853).


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

The kernel now includes a narrowly scoped, typed direct-beam adapter: declared direct-normal irradiance (DNI), aperture area, incidence cosine, unshaded fraction, and interval duration produce aperture-plane incident power and interval energy, with an independent recomputation check. A hand case (800 W/m² DNI, 10 m² aperture, 0.5 incidence cosine, 0.8 unshaded fraction, 3600 s) yields 3.2 kW and 11.52 MJ. This is a dimensional/accounting bridge only: it does not estimate DNI from GHI, compute solar position or shadows, add diffuse/albedo irradiance, or model optical/receiver losses. DNI input must remain source- and timestamp-bound before it can support a site-level benchmark. The current canonical flat receipt now carries this as a distinct fixture and the independent stdin auditor recomputes both power and energy while rejecting tampered inputs or outputs. It intentionally remains separate from the downstream collector fixture, so the two incident-energy values are not implicitly connected. The receipt still declares `source_binding: not-source-hash-bound`; it proves deterministic arithmetic for pinned synthetic inputs, not weather provenance, solar-position correctness, shading correctness, or measured yield.

A plausible first external resource for the South African pilot is the [CAMS solar radiation time-series](https://ads.atmosphere.copernicus.eu/datasets/cams-solar-radiation-timeseries?tab=overview), dataset DOI https://doi.org/10.24381/5cab0912. The official description covers historical irradiation from 2004 to near-real-time, includes direct-normal irradiation (BNI), and describes all-sky/cloudy products within Meteosat/Himawari coverage that includes Africa. It offers multiple interval resolutions and UT or true-solar-time modes under a CC-BY licence, subject to the portal's current access/attribution terms. Treat this as satellite-derived radiation data, not local ground-truth measurement.

CAMS BNI is interval irradiation in Wh/m², not the W/m² input expected by an instantaneous/interval-mean irradiance adapter. The kernel now has an explicit Wh/m²-to-mean-W/m² conversion with independent verification and energy-preservation tests. It does not reconstruct irradiance or sun geometry within the interval: applying a single incidence/shading factor remains an approximation, so preserve the chosen interval resolution and quantify sensitivity to it. CAMS timestamps refer to the end of the integration interval; retain UTC vs true-solar-time choice explicitly, never interpret true solar time as South African civil time, preserve raw response bytes and metadata, and calculate a source/configuration SHA-256 before calling a field run reproducible. See [ECMWF's product overview](https://www.ecmwf.int/en/forecasts/datasets/cams-solar-radiation-time-series) and the [CAMS data documentation](https://confluence.ecmwf.int/spaces/CKB/pages/266592908/CAMS+solar+radiation+time-series+data+documentation).

**Do not conflate interval irradiation with plane-of-aperture beam energy.** For a fixed or tracking collector, the direct-beam aperture energy is `E_beam,ap = integral(DNI(t) * A_aperture * max(cos(theta_incidence(t)), 0) * f_unshaded(t) dt)`. Converting CAMS BNI to interval-mean DNI preserves the beam-normal energy but cannot recover its within-interval time distribution or covariance with changing incidence angle and shading. Multiplying the mean DNI by one midpoint geometry factor is an approximation whose error should be measured rather than assumed negligible.

For geometry, use a pinned implementation of the [NREL Solar Position Algorithm](https://doi.org/10.2172/15003974) as a reference candidate for sun zenith/azimuth and surface incidence angles. The revised report describes angular-position uncertainty of approximately ±0.0003° over its declared validity range; this is geometric precision, not irradiance accuracy or evidence that the collector/shading model is correct.

Proposed integration test: compare the most-resolved pinned radiation series against 15-minute and hourly aggregations while evaluating geometry at the same declared time quadrature. Include fixed-tilt/tracking cases, cloudy/ramp days, varied solar elevations and intervals crossing sunrise/sunset. Report daily/monthly aperture-plane energy bias, interval-error distribution, maximum error and whether the model error changes the design/controller ranking. For the selected site/configuration, a **provisional target** is under 1% relative error in daily aperture-plane beam energy against the pinned high-resolution reference, plus a separate bound for sunrise/sunset-crossing intervals. This target is untested and site/configuration specific. Preserve CAMS interval-end timestamps, UTC vs true-solar-time modes, missing values and raw source/configuration digests.

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

For the current constant-ambient, lumped sensible-store loss law, compare explicit Euler against an analytic trajectory before tuning a controller. With piecewise-linear c_p(T), constant mass and conductance, the cooling-time integral can be solved segment-wise; a new experimental integrator now implements this as an alternative without silently changing the legacy dispatch fixtures. Its promotion requires agreement with the constant-c_p exponential solution, independent elapsed-time and energy checks, floor-event handling, and variable-c_p tests. The current dispatch schedule still uses the explicit-Euler path pending qualification, so its trajectory may remain timestep-sensitive. The analytic fixture matrix now targets the dimensionless interval ratios UA·Δt/(m·c_p) = 0.01, 0.1, 1, and 10; variable-cp segment crossings; equivalence when one interval is split into consecutive subintervals; and a coarse-Euler stress comparison. For the constant-cp case at ratio 1 (100,000 seconds for the fixture parameters), explicit Euler requests 1 MJ of loss and clamps at Tmin, whereas the analytic solution yields approximately 336.79 K and 632.12 kJ loss. These are deterministic model-derived reference values, not experimental observations. Do not switch the canonical dispatch path until the new exact-head test/Clippy/audit workflow passes and the result has been reviewed. See [THERMAL-LOSS-001](https://github.com/Luminous-Dynamics/symtropy/issues/1947).

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

**Treat data volume as an experimental variable, not a footnote.** Mohammadzadeh et al. (2024), [stochastic-MILP dispatch under uncertainty](https://doi.org/10.1016/j.segan.2024.101458), studied a hypothetical 115 MW CST + 10-hour storage plant. With a limited historical weather corpus, its stochastic optimizer reported about 6–9% more profit than the closest heuristic on novel conditions; with a larger corpus, the stochastic approach and strongest heuristic became nearly indistinguishable, and on some holdout subsets the heuristic slightly led. This result is specific to its power-plant/electricity-profit boundary, but it motivates a general causal experiment: **sweep training-corpus size while holding confirmatory weather/load cases fixed**.

For the dispatch tournament, compare rule-based control, deterministic MPC, stochastic/robust/CVaR control, a strong cross-weather historical-profile heuristic, and Symthaea under the same information and budget. Increase the training scenario corpus across preregistered sizes (e.g. 5, 10, 20, 50, 100 independent day/week trajectories where available); preserve temporal/weather correlation, and keep full weather years/events as holdouts rather than randomly splitting adjacent intervals. Record expected useful service and cost, P95/CVaR95 shortfall, regret on cloudy/ramp events, runtime, memory, simulator evaluations, scenario-generation time and data-preparation effort. Plot quality-versus-data-volume and quality-versus-compute, and report where added model/search complexity stops producing a material, statistically defensible benefit. Perfect foresight stays a separately labeled oracle bound, never a deployable controller. See [THERMAL-OPT-001](https://github.com/Luminous-Dynamics/symtropy/issues/1877).

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
- one static pinch problem with hand-checkable minimum hot/cold utility targets, maximum internal heat recovery, and shifted/hot-side/cold-side pinch temperatures;
- one ordered day/night storage schedule with closed-form standing loss, temperature-grade-gated delivery, an independently recomputed first-law ledger, and a tampered-report negative control;
- one heat pump using a small declared COP map and a test outside the map that must block;
- negative controls for GHI/DNI swaps, Celsius/Kelvin confusion, double-counted charge/discharge heat, over-unity efficiency, negative inventory, impossible heat-exchanger approaches, inconsistent period duration/power, invalid dispatch ordering, and tampered temporal reports.

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

### 8.1 Pinned multi-period process-integration cross-check

Use the public OpenPinch multi-period crude-preheat dataset as an independent reference-model input, not as an assumed numeric result:

- Reference repository: [OpenPinch](https://github.com/waikato-ahuora-smart-energy-systems/OpenPinch), MIT license.
- Reference release commit: `0f38afbf8565c295a619bb0baed1d77ee5884565` (v0.7.0).
- Published source distribution: `openpinch-0.7.0.tar.gz`, SHA-256 `7645ef3a8d1c579eb619d3308ce7f33dfd5598d4a6cff2c55519bf4395a1d50a`; PyPI release provenance records the same publishing commit.
- Fixture: `OpenPinch/tutorials/sample_cases/crude_preheat_train_multiperiod.json`; pinned Git blob object `d9f9b471b2a879a794bf057e6b4b677e2b9ca236` (this is a Git blob ID, not a SHA-256 digest).
- Period IDs and weights: `turndown`, `base`, `peak`; weights `0.2, 0.6, 0.2`. Inputs include period-varying stream target temperatures and heat flows; temperatures are in °C and duties in kW. Convert explicitly before giving values to the Rust Kelvin/J model.
- Reference execution sequence at that commit: `PinchProblem("crude_preheat_train_multiperiod.json")`, `validate()`, `target.all_periods.all_heat_integration()`, then `summary_frame(include_periods=True, include_weighted_average=True)`. Pin exact OpenPinch package/environment/solver versions with the generated baseline receipt.

The acceptance artifact is not just an aggregate weighted utility. Preserve each ordered period's stream inputs and hot/cold utility targets, then independently verify units, all period IDs and weights, per-period first-law consistency and weighted aggregation. Add a Rust cross-check for the subset the Rust static pinch model actually supports. The full OpenPinch case includes multiple streams and other features beyond the current narrow Rust model; **do not claim whole-case parity until the supported subset and its mapping are explicitly scoped**. Numeric OpenPinch outputs are not yet checked into this benchmark contract, so this is a pinned input candidate—not a passing reproduction. Generate its reference outputs in the pinned upstream runtime, review and commit the expected ledger, then implement the independent Rust comparison.

This gives us a public multi-period input corpus while the industrial solar-storage paper's complete plant time series remain unavailable. The two references answer different questions: OpenPinch qualifies period-specific process integration; Falch et al. is a storage-utilization study with its own idealized storage assumptions.

### 8.2 Independent measured PVT/exergy-control dataset (different storage physics)

A complementary empirical candidate is Zenodo record v1.0.0 [“Perfect-Foresight Flow-Rate Control of a Photovoltaic–Thermal Collector for Thermochemical Storage: An Exergy Upper Bound”](https://doi.org/10.5281/zenodo.20713009), published 18 August 2026. The record includes the 19 September 2024 validation holdout, two measured controller-comparison days (10 and 24 September), contrast-day inputs, and released outputs for Cases A–D and a horizon sweep; its Python 3.11 script regenerates outputs and figures. The associated paper was published in *Energies* on 22 August 2026 (19(17), 3949; [DOI 10.3390/en19173949](https://doi.org/10.3390/en19173949)). The code can be treated as a pinned reference oracle while new production implementation remains Rust-first.

**Provenance and rights boundary:** Zenodo lists the archive `Minimal_Dataset_Paper2_v2.zip` with MD5 `93397af1b0c1689843863ee8aab37b24` and `paper2_run_v2.py` with MD5 `44856a97a31788101bbefc1c39a46d6`. These are MD5 identifiers supplied by the record, not SHA-256 checksums. The record's public page does not show a license value. Do not vendor, mirror, or redistribute the dataset or code until reuse permissions are confirmed; pin the exact record version and independently calculate SHA-256 if downloaded for local-only reproduction.

This is **not** the same device model as the current sensible store: the study concerns a PVT collector and SrCl₂/NH₃ thermochemical storage. Its perfect-foresight controller is explicitly an exergy upper bound, not a feasible causal controller. Therefore:
1. Archive the exact upstream metadata and calculate strong content digests in the local benchmark manifest, but keep downloaded files outside the public repository unless rights are explicit.
2. Reproduce the released Cases A–D outputs under a pinned environment and retain 19 September as a true holdout; do not tune model parameters or policy on it.
3. Build a typed Rust adapter for only the measured collector/exergy boundary after confirming README column definitions and units. Validate against released outputs and measured comparator days independently.
4. Compare online policies only to causal baselines using data available at that timestamp; report perfect foresight separately as the upper bound. Measure the gap to that bound; do not train to leaked realized weather.
5. Integrate dispatch only after boundary-level validation passes. Do not claim cross-chemistry parity with industrial sensible storage or the OpenPinch cases.

The industrial paper, OpenPinch input corpus, and PVT dataset are deliberately separate benchmark families: ideal storage sizing, period-specific process integration, and measured collector/exergy control respectively. Combining them into one score before the boundaries and physics align would make the benchmark less scientifically useful, not more.

### 8.3 Solar-plus-storage techno-economic reference: STEP 1

[NREL/Sandia's Solar Thermal Energy Planner (STEP 1)](https://step1.nrel.gov/) is an appropriate system-level comparison when the public tool's site, land, load-temperature, fuel/electricity price and financial assumptions can be aligned. STEP 1 generates a nominal production profile using SAM and uses a tailored REopt optimization to size and dispatch solar-plus-storage systems, reporting cost of heat, net present value and capital cost. It spans several solar-thermal technologies and PV + resistance + TES. See the 2026 report [Gifford et al., DOI 10.2172/3017739](https://doi.org/10.2172/3017739).

Do not treat the tool's output as a universal ground truth or compare with different tariff, land, service, emissions or financing boundaries. Use STEP 1 as a *model/system comparator*: freeze one input case, capture every assumption and output visible/exportable through the supported interface, and compare only common quantities. The pinned analytic kernel first qualifies physical ledgers; this separate benchmark tests whether a proposed end-to-end design improves levelized cost of useful heat or net present value under aligned constraints. The model's own optimum is not evidence of field performance.

## 9. Outcomes and benchmark targets

Report a Pareto table instead of one opaque score. Primary metrics must be predeclared for each end use.

### Service and thermodynamics
- fraction of required heat demand met, stratified by load and temperature band;
- unmet useful heat in kWh_th, heat-deficit hours, longest interruption and critical-load service;
- expected, worst-case, weighted P95 and CVaR95 of demand-normalized unmet heat; probability of critical service-floor violation and held-out regret;
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
7. **Multi-fidelity decision quality:** measure low-to-high-fidelity false screens, false promotions, ranking flips and decision quality per compute budget. Require per-quantity error budgets and escalation on out-of-domain or decision-reversing model disagreement ([THERMAL-FIDELITY-001](https://github.com/Luminous-Dynamics/symtropy/issues/1954)).
8. **Tail-risk service quality:** report weighted P95/CVaR95 shortfall, critical-service violation probability and worst-case deficit; preserve hard critical-service/equipment limits under held-out weather, demand and fault scenarios. Perfect foresight is an upper-bound comparator only ([THERMAL-RISK-001](https://github.com/Luminous-Dynamics/symtropy/issues/1969)).
7. **Engineering cost:** include model-build, simulator-call, compute and human-review effort. A small gain requiring disproportionate engineering or unrealistic sensors is not automatically useful.
8. **External evidence:** hardware or field performance, component durability, safety and real cost remain separate gates requiring their own measurements and review.
9. **Numerical resolution:** compare loss integration with a closed-form constant-cp reference; demonstrate timestep-refinement convergence for variable cp(T); bind integrator/version/tolerance/step settings into receipts, and reject or explicitly flag trajectories outside the error budget ([THERMAL-LOSS-001](https://github.com/Luminous-Dynamics/symtropy/issues/1947)). Energy closure alone does not establish trajectory accuracy.

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
- **Evaluating the potential of thermal energy storage integration in enhancing high-temperature heat pump performance: A comprehensive analysis and optimization strategies**, *Journal of Energy Storage* 147 (20 February 2026), article 120344, DOI: https://doi.org/10.1016/j.est.2026.120344. Use its high-temperature heat-pump-plus-TES cases as a reproducible hypothesis; its reported gains are scenario-specific and not Symthaea evidence.
- IEA Solar Heating and Cooling Task 62 reports, including **Mapping degradation of PCM and TCM** (September 2025): https://www.task62.iea-shc.org/reports
- Lehigh University, **Lehigh Thermal Battery**: component architecture and university-reported test boundaries for concrete sensible storage with embedded thermosiphons: https://engineering.lehigh.edu/research/resolve/volume-2-2023/lehigh-thermal-battery and https://news.lehigh.edu/engineers-unveil-lehigh-thermal-battery-for-decarbonization-applications
- Emil et al. (2025), **Validation of a pit thermal energy storage model: Demonstration of a comprehensive approach**, *Journal of Energy Storage* 116, 115680, DOI https://doi.org/10.1016/j.est.2025.115680. The study reports five years of in-situ storage measurements and separates solution verification, temporal/spatial discretization error and validation uncertainty. Use it to guide the verification protocol, not as a parameter-fit dataset for a different store.
- Conti et al. (2025), **How thermal storage modeling affects integrated energy systems simulation: Lumped vs. stratified models under fast and high-capacity thermal loads**, *Journal of Energy Storage*, DOI https://doi.org/10.1016/j.est.2025.117704. Use the study's 0-D versus validated 1-D comparison to design stratification-sensitive test families; do not assume a lumped store is adequate under fast draw.
- Wen et al. (2025), **Development of a reduced-order dynamic model for large-scale seasonal thermal energy storage applications**, *Energy*, DOI https://doi.org/10.1016/j.energy.2025.137379. The studied reduced-order model reported up to 98% faster runs with under 6% energy deviation against an experimentally validated COMSOL reference. Treat this as evidence that a measured speed/accuracy trade-off is possible for that setup, not as a universal tolerance or a Symtropy result.
- A 2026 open-access study, **Dynamic modeling of heat pipe integrated thermal battery latent heat storage system experiment validation**, *Journal of Energy Storage* 160, article 121875, DOI: https://doi.org/10.1016/j.est.2026.121875. Use its published model/experiment comparison as a temporal validation case; inspect output-specific errors and applicability before reuse.
- Deeper 2026 case study on multi-temperature R718 heat pumps, thermal storage and PV in grid-connected industrial multi-energy systems: DOI https://doi.org/10.1016/j.energy.2026.141996. Reproduce applicable model assumptions first; its reported advantages versus rule-based operation are not transferable without matching loads, hardware envelopes and cost boundaries.
- DOE, **Thermal Storage R&D for CSP Systems**: historical research targets and technology directions; do not silently treat old program targets as current universal acceptance standards: https://www.energy.gov/cmei/systems/thermal-storage-rd-csp-systems

**Recent dynamic-storage reference with an important evidence caveat.** Mikkelson et al. (2026), [dynamic validation of a heat-pipe-integrated latent thermal battery](https://doi.org/10.1016/j.est.2026.121875), report temperature errors ranging from 2.5 °C to 39.7 °C across 10–47 hour runs and identify the model implementation in the HYBRID repository; the article's data availability statement says experimental data are available on request. Treat it as a distinct high-temperature Al–Mg–Zn latent-store validation candidate, not as a public time-series fixture today. Do not set a generic tolerance from its error range, which depends on the output, calibration/holdout split, and operating period.

- Walmsley (2026), **OpenPinch: An Open-Source Python Library for Advanced Pinch Analysis and Total Site Integration**, *Process Integration and Optimization for Sustainability* 10, 3947–3963, DOI: https://doi.org/10.1007/s41660-026-00729-6. Use as a reference for method semantics and test cases; Symtropy’s Rust implementation must be independently compared, not assumed equivalent.
- Falch, Espen, and Anantharaman (2026), **Optimizing Heat Storage Integration for Solar Thermal Systems in Industrial Process Heat Networks**, *Systems and Control Transactions* 5, 419–423, DOI: https://doi.org/10.69997/sct.176470. The lubricant-plant utilization/storage numbers are an external reproduction target only after exact data, assumptions, and measurement boundaries are pinned.

- Mohammadzadeh et al. (2024), **A stochastic-MILP dispatch optimization model for concentrated solar thermal under uncertainty**, *Sustainable Energy, Grids and Networks* 39, 101458, DOI https://doi.org/10.1016/j.segan.2024.101458. Use as a baseline-design reference for stochastic and heuristic dispatch comparisons; preserve the perfect-knowledge upper-bound distinction.
- **Risk-aware stochastic MPC for optimal dispatch in concentrating solar power plants under irradiance uncertainty** (2026), DOI https://doi.org/10.1016/j.segan.2026.101458. The indexed abstract reports evaluations against deterministic MPC, scenario-based stochastic MPC, rule-based and robust-minimax baselines using scenario-tail CVaR and dispatch deficit. Treat its gains as case-specific; first align plant, forecast, storage, cost and service boundaries.
- **Medium-term optimal operation of energy hub considering risk assessment and uncertainties of energy price and renewable energy resources** (2025), DOI https://doi.org/10.1016/j.ecmx.2025.101350, illustrates a CVaR-based cost/risk trade-off for a system that includes thermal storage.

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
