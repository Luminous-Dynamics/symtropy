# Generator Benchmark Protocol — Fuel-Flexible Power Systems

**Version:** 0.1  
**Status:** Protocol proposal; no physical test results  
**Date frozen:** 2026-11-02  
**Primary application:** stationary / industrial AC generator sets  
**Initial sizing hypothesis:** 50–250 kWe, pending a customer duty cycle and equipment availability

## 1. Purpose

Establish a reproducible comparison of generator systems that can use different validated fuel configurations without hiding efficiency, emissions, reliability, fuel-quality or lifecycle penalties behind a single "fuel-agnostic" label.

The protocol separates:
- engine brake thermal efficiency;
- gross generator electrical efficiency;
- net AC efficiency after auxiliaries;
- fuel consumption and fuel batch properties;
- transient electrical performance;
- emissions;
- lifecycle GHG;
- reliability, maintainability and economics.

A software-model PASS is not a physical engine or generator qualification. Synthetic profiles are test fixtures, not field measurements or standardized certification cycles.

## 2. Standards register (snapshot 2026-11-02)

The standards below are candidate references. Before a real campaign, retrieve the licensed/current editions, confirm national adoption and application scope, and record the test laboratory's controlled copies.

| Standard / framework | Relevant purpose | Snapshot / caution |
|---|---|---|
| ISO 8528-1:2018 | Application, ratings and performance classifications for RIC-engine AC generating sets | ISO page says reviewed/confirmed 2023 and current; ISO/DIS 8528-1 is in development and expected to revise it |
| ISO 8528-5:2025 | Generator-set design and performance criteria for the engine + AC generator operating as a unit | Current 2025 edition; use the applicable controlled copy |
| ISO 8528-6:2023 | Test methods for characterizing the complete generating set | Includes simulation methods for assessing specified set capability; physical test and simulation evidence must remain distinct |
| ISO 8178-1:2020 | Test-bed measurement systems for gaseous and particulate exhaust emissions | Current published edition is under revision; applies to non-road/stationary applications including generator sets, not road-primary vehicles |
| ISO 8178-2:2021 | Field measurement/evaluation of gaseous and particulate emissions, including PEMS use | Use for applicable in-service field work, not as a substitute for certification type testing |
| IMO MARPOL Annex VI Reg. 13 / NOx Technical Code 2008 | Marine diesel engine survey, certification and in-service compliance | Applies to installed marine diesel engines above 130 kW subject to stated scope/exceptions; applicability depends on ship, engine, construction date and operating area |
| ISO/IEC 17025 | Competence of testing and calibration laboratories | Require traceable calibration and competent lab scope for critical measurements; exact instrument traceability belongs in the test record |

Road-going heavy-duty engine certification and WHTC/WHSC procedures are separate from this stationary generator protocol. Marine certification is separate again. Never infer a road or marine approval from an ISO 8528 generator test.

## 3. System boundary and reference configuration

Define a generating set as a complete system:
- engine and fuel-conditioning equipment;
- generator / alternator;
- governor and excitation / voltage regulation;
- controlgear and switchgear;
- fuel, lubrication and cooling auxiliaries;
- exhaust aftertreatment and its electrical/thermal parasitics;
- any battery, inverter or power-conditioning equipment included in the product boundary.

Report at least two boundaries:
1. **Engine boundary:** brake output and chemical fuel input (LHV basis).
2. **Net-set boundary:** net AC electrical output at the declared terminal after all included auxiliaries.

For hybridized systems, also report a third boundary:
3. **Delivered-load boundary:** net useful energy delivered to the specified customer interface after all included conversion, storage and distribution losses.

The delivered-load interface must be typed:
- **AC terminal:** report net AC power/energy, voltage, frequency and power factor.
- **DC bus:** report regulated DC power/energy, voltage window, current, ripple, protection and included DC/DC conversion losses.
- **Mechanical output:** for vehicle or vessel comparisons, report shaft / wheel / propulsor work at the declared measurement point rather than relabeling it as electrical efficiency.

For a DC-native installation, compare candidates at the same DC load interface; do not force a DC output through a hypothetical AC conversion stage unless the actual customer system requires that conversion. For an AC installation, compare at the same net AC terminal. If a candidate can deliver both, report both boundaries separately. A common service boundary does not mean the internal conversion chain is identical.

Declare the measurement point, power factor where applicable, voltage/frequency or DC voltage window, grid-parallel or islanded mode, ambient conditions, fuel temperature/state, and every included/excluded auxiliary.

Do not compare an engine BTE result against a net AC generator-set efficiency result, or a fuel-cell stack DC efficiency against whole-plant net AC.

## 4. Baseline and candidate families

The benchmark must compare **conversion topologies**, not only different fuels inside one reciprocating engine.

Minimum comparison set:
1. Current production diesel reciprocating engine-generator of similar net-AC rating and application class.
2. Same-set HVO/renewable-diesel compatibility configuration where the manufacturer and fuel specification permit it.
3. A separate natural-gas/biomethane dual-fuel engine-generator only after its gas train, pilot-fuel fraction, ventilation, leak detection, shutdown and safety case are established.
4. **Mainspring linear generator** as the closest same-scale multi-fuel comparator. Its product materials report 250 kW modules, 46% net-AC LHV electrical efficiency on natural gas/biogas, <1.5 ppm NOx, 0–100% output range and a standard 2.5 MW block made from ten 250 kW units. Preserve the vendor's measurement footnotes and uncertainty; do not treat them as independent head-to-head results.
5. **Capstone Energy+ C200 / C250 microturbines** as same-scale comparators. Its FY2026 SEC filing confirms the C200 200 kW platform and describes natural gas, renewable biogas, landfill gas, wastewater methane, sour/low-BTU gas and up to 30% hydrogen blends in product literature. The C250 250 kW platform has completed test runs but is still advancing through commercialization. The filing reports approximately 33% simple-cycle efficiency, but the cited passage does not specify net/gross boundary, LHV/HHV basis or correction conditions; do not admit it as a matched net-AC ranking number without the controlled datasheet/test report. The same filing describes an 800 VDC-native product platform and an up-to-5% improvement claim versus legacy distribution; this is a manufacturer claim, not a verified microturbine net-efficiency improvement.
6. Fuel-cell generator where fuel supply, storage, reformer/stack/inverter parasitics, load response, lifetime, replacement and cost data are credible.
7. Battery-electric storage/inverter supply for the same load trace where a credible grid/charging source is available.

Biodiesel, methanol, hydrogen and ammonia are separate fuel/configuration studies. No implicit drop-in or instant fuel-switch assumption is permitted. The Air Force's March 2026 Mainspring pilot includes ammonia and syngas in its planned test set; this is a test programme, not evidence that the standard commercial product has already qualified those fuels.

### 4.1 Mainspring linear-generator evidence classes

Keep at least three distinct evidence records:

- **Manufacturer current product specification:** 250 kW module / 2.5 MW standard block; 46% net AC LHV on natural gas and biogas; 0–100% output; <1.5 ppm NOx. The datasheet says its efficiency was measured according to ASME PTC 50 at 15°C and 1 atm, with 1% assumed inverter and line losses included. The same datasheet states "unit weight 22 tons"; its unit/package basis must be clarified before power-density calculations.
- **CEC-sponsored early pilot report:** 230 kW beta unit operated at a grocery site for nine months; 41.5% net AC efficiency and >40% across the reported building load range; a third-party source test reported 1.6 ppm NOx, 0.8 ppm CO and 8.4 ppm VOC. The report was prepared by Mainspring and includes a CEC disclaimer; third-party status is specifically stated for the emissions test. Do not label the whole report an independent comparative certification.
- **U.S. Air Force 2026 prototype pilot:** 250 kW test programme to gather fuel efficiency, power, consumption and emissions data on natural gas, biomethane, syngas, ammonia, propane and hydrogen. Do not treat planned test fuels as completed qualifications or publish results until the programme releases them.

Mainspring also reports >1.5 million core run hours and an AEP agreement for up to 1.6 GW with a first 190 MW tranche announced for delivery beginning in 2027. Core-hours are not automatically single-unit elapsed hours, MTBF or system availability; a sales agreement is not installed or independently measured fleet capacity.

**Test-method caution:** Mainspring states that product efficiency was measured under ASME PTC 50. ASME's public PTC catalogue identifies PTC 50 as Fuel Cell Power Systems Performance. Obtain the actual test report and a documented scope/applicability explanation before treating this as a directly comparable generator-set test. Do not silently substitute PTC 50 for ISO 8528 or another test code.

### 4.2 Fair comparison rules across topologies

- Compare at the same net electrical terminal, with the same included auxiliaries, voltage/frequency, power factor, reserve and load profile.
- Compare net AC efficiency on a common LHV basis; preserve HHV conversions and fuel properties.
- Report output range, step-load response, start/stop, ramp rates, low-load performance and parasitic loads separately.
- Include mass, volume, skid footprint, foundation, ventilation, fuel conditioning, noise, heat rejection, cooling-water use and site requirements.
- Distinguish vendor specifications, manufacturer-authored demonstration reports, third-party measurement, independent validation, regulatory certification and field-operation data.
- A linear generator does not have a conventional crankshaft / alternator chain; a fuel cell has a DC stack and inverter chain. Model each topology explicitly rather than forcing all outputs through an engine-brake-power field.

#### Direct-DC delivery cases

For data-centre, EV-charging and other DC-native loads, add a separate delivered-DC-bus test case:
- define the customer DC-bus voltage range, ripple, transient load profile, protection and isolation requirements;
- meter net DC energy at the customer interface and separately meter fuel input, auxiliary loads, conversion stages and thermal rejection;
- compare an AC-output system plus rectifier / distribution against a DC-native source at the **same delivered DC load**, including cable, transformer, rectifier, DC/DC, inverter and UPS losses that are inside the declared boundary;
- do not add an imaginary inverter loss to a DC-native architecture, and do not omit real power-conditioning stages from the AC or DC alternative;
- report DC-bus energy efficiency and AC-terminal efficiency as separate metrics; do not relabel one as the other.

Capstone's FY2026 filing describes an 800 VDC platform and claims up to 5% improvement versus legacy distribution. Record the claim as manufacturer-reported and require matched load, voltage, conversion boundary and independent measurement before ranking. This is a valuable topology hypothesis, not an assumed 5% system gain.

### 4.3 Primary same-scale published reference: Mainspring MSE3 linear generator

Do not describe this programme as the first 250 kW multi-fuel generator.

Mainspring's current product materials publish a 250 kW linear-generator module and a standard 2.5 MW package built from ten 250 kW units, 46% net-AC LHV efficiency on natural gas and biogas, <1.5 ppm NOx, 0–100% output range, and product fuels including biogas, natural gas, hydrogen and propane. The 2.5 MW datasheet states that 46% was measured under ASME PTC 50 at 15°C / 1 atm and includes a 1% assumed inverter/line-loss allowance. It lists "unit weight 22 tons"; clarify the unit/package meaning before using this in a kg/kW comparison.

The 2024 California Energy Commission-sponsored project report gives an earlier 230 kW beta-unit result: 41.5% net-AC efficiency, above 40% over its reported 130–370 kW building-load range, with a nine-month grocery-site demonstration. Its third-party source test reported 1.6 ppm NOx, 0.8 ppm CO and 8.4 ppm VOC. The report was prepared by Mainspring under a CEC-sponsored project and contains a disclaimer that it does not necessarily represent CEC views and is not a CEC approval of accuracy. The report describes third-party testing specifically for source-test emissions; do not call the efficiency result an independent comparative certification.

The Department of the Air Force announced a 250 kW pilot in March 2026 to gather fuel efficiency, power, consumption and emissions data across natural gas, biomethane, syngas, ammonia, propane and hydrogen. These are planned pilot tests, not public proof that every listed fuel has passed qualification.

Mainspring's manufacturer page reports >1.5 million core run hours, and its October 2026 press release announces an agreement with AEP for up to 1.6 GW through 2034, with an initial 190 MW tranche scheduled to start delivery in 2027. Keep core run hours distinct from single-unit hours, MTBF and availability; keep a procurement agreement distinct from installed, independently measured capacity.

**Test-method clarification required:** Mainspring states that the 46% result was measured according to ASME PTC 50. The ASME public PTC catalogue identifies PTC 50 as Fuel Cell Power Systems Performance. Obtain the detailed test report and a documented applicability explanation before treating the result as directly comparable with ISO 8528 generator-set test results. Do not silently infer that one code supersedes the other.

Sources: Mainspring product page and 2.5 MW datasheet; CEC-500-2024-037 final report; Air Force / AFMC pilot announcement dated 4 March 2026; Mainspring / AEP announcement dated 13 October 2026; ASME Performance Test Codes catalogue (URLs listed in Section 12).

A reference engine map must be measured or sourced with traceable evidence. If no valid map or topology-specific performance dataset exists, the model must mark the operating point unknown and refuse quantitative ranking.

Use the versioned competitor evidence register and schema: [COMPETITOR_BENCHMARKS_2026-11-02.json](data/COMPETITOR_BENCHMARKS_2026-11-02.json) and [COMPETITOR_BENCHMARK_SCHEMA_2026-11-02.json](data/COMPETITOR_BENCHMARK_SCHEMA_2026-11-02.json). The dataset records source, evidence class, boundary and comparison limitations per claim; it is not a substitute for a controlled product test report.

## 5. Test matrix

### 5.1 Steady-state engine / set map

At a minimum, characterize approved load points across the declared operating range, including low load, intermediate load, rated load and any declared overload/peak-load capability. Exact points, dwell times, correction factors and test conditions must follow the applicable controlled standard and manufacturer rating.

At every point capture:
- fuel mass flow and fuel batch identity;
- LHV (measured or certificate-supported with evidence class);
- engine speed and torque / brake power;
- generator gross electrical output;
- net output and auxiliary power;
- voltage, current, frequency and power factor;
- coolant, oil, intake, exhaust and ambient temperatures;
- boost / intake pressure and relevant air-flow state;
- regulated pollutants and particulate metrics in the applicable test method;
- uncertainty, calibration IDs and stable-period selection rule.

### 5.2 Synthetic engineering profiles (not standards cycles)

Use these deterministic fixtures for software testing and early design exploration only:

| Profile | Segment sequence | Purpose |
|---|---|---|
| GEN-BASE-80 | 80% rated load for 8 h | Base-load energy and fuel accounting |
| GEN-VARIABLE-LOAD | 20%, 40%, 60%, 80%, 100%, 60%, 40% load; equal 30 min segments | Weighted energy efficiency across a variable operating map |
| GEN-STEP-LOAD | 50% -> 75% -> 100% -> 50%; 10 min nominal dwell per state | Identify test points for step-load response; static model does not predict transient response |
| GEN-LOW-LOAD | 10%, 20%, 30% load; 60 min per point | Expose low-load efficiency and emissions penalties |
| GEN-MODULAR-DISPATCH | Two or more sets with staggered commitment and load sharing | Compare modular set dispatch against one oversized set; requires minimum-load, start, stop and ramp data |

The profiles use normalized load fractions and synthetic durations. They do not contain engine maps, vendor fuel consumption or measured performance. No numerical efficiency ranking may be produced until real map points are supplied.

### 5.3 Transient and power-quality tests

Use ISO 8528-5 / ISO 8528-6 controlled requirements to select and perform the applicable set-level tests. Independently capture:
- load-step magnitude and direction;
- voltage and frequency deviations;
- recovery time and settling criteria;
- engine speed, torque/fuel-flow response;
- smoke / combustion stability where relevant;
- protection events, trips and derating;
- load power factor and non-linear load conditions.

The current 0D model cannot predict these transients. It can only store and compare later measured traces through a separate time-series/evidence interface.

## 6. Energy arithmetic and acceptance checks

For each steady-state segment:

`P_{fuel} = dot m_{fuel} \times LHV`

`P_{brake} = P_{fuel} \times \eta_{brake}`

`P_{gross,electric} = P_{brake} \times \eta_{generator}`

`P_{net,AC} = P_{gross,electric} - P_{auxiliary}`

For a duty-cycle segment with duration `\Delta t_i`:

`E_i = P_i \times \Delta t_i`

Aggregate energies first, then calculate the duty-cycle efficiency:

`\eta_{net,cycle} = \sum_i E_{net,AC,i} / \sum_i E_{fuel,i}`

Do not arithmetic-average segment efficiency. Duration, fuel flow and input energy differ between segments.

First-law bookkeeping must close:

`E_{fuel} = E_{engine,rejected} + E_{generator,loss} + E_{auxiliary} + E_{net,AC}`

Numerical closure tolerance and measurement uncertainty are different quantities. A zero numerical residual does not imply measurement accuracy.

## 7. Fuel Flexibility Efficiency Retention (FFER)

FFER is an internal comparative metric, not a certification standard.

For a given fuel and a frozen reference duty cycle, compare the **integrated cycle efficiencies** at the same net electrical service boundary:

eta_net,cycle = sum(net AC energy over segments) / sum(fuel LHV energy over segments)

FFER = eta_net,cycle(shared architecture) / eta_net,cycle(fuel-specific reference)

Both candidates must use the same profile identity, segment order, dwell times, net-load demand, ambient correction and auxiliary boundary. Delivered net electrical energy must match within a preregistered tolerance; otherwise the comparison is a service mismatch, not an efficiency win. Report each segment and integrated totals so an aggregate cannot hide a low-load or transient penalty. Uncertainty and missing map points remain visible.

Do not arithmetic-average segment efficiencies or silently normalize missing points. Do not compare unrelated fuels by their heating value alone. FFER answers whether shared-platform flexibility has an efficiency penalty relative to a fuel-specific reference. It does not show lower lifecycle emissions or lower cost.

Proposed staged research gates:
- **Feasibility gate:** FFER >= 0.90 on the frozen weighted map.
- **Strong platform target:** FFER >= 0.95.
- **Stretch target:** FFER >= 0.97.

These are programme targets, not claims about achievable engine performance. A candidate that fails the threshold is still a useful result if it exposes the mass, cost, efficiency or emissions cost of modularity.

## 8. Emissions and lifecycle boundary

Keep the following separate:
- measured tailpipe / stack emissions mass and weighted concentration;
- regulated pollutant compliance under the applicable test cycle;
- direct combustion CO2;
- upstream fuel production and transport emissions;
- methane leakage and slip for gaseous fuels;
- NH3 slip and N2O where relevant to ammonia;
- electricity generation and charging pathway;
- equipment manufacture, battery replacement and end-of-life where included.

Report gCO2e per net kWh for generators, then use actual energy-chain and vehicle/vessel models to compare gCO2e per tonne-km or tonne-nautical-mile.

For each pathway record LHV/HHV basis, GWP time horizon/source, allocation method, electricity mix and year, upstream boundaries, leakage assumptions and uncertainty. A fuel label such as "renewable", "green" or "low-carbon" is not lifecycle evidence.

### 8.1 Marine fuel storage, payload and range

A marine propulsion comparison must include the fuel containment system, not only the fuel's neat LHV or engine efficiency. Use [MARINE_FUEL_STORAGE_REFERENCES_2026-11-02.json](data/MARINE_FUEL_STORAGE_REFERENCES_2026-11-02.json) with [MARINE_FUEL_STORAGE_SCHEMA_2026-11-02.json](data/MARINE_FUEL_STORAGE_SCHEMA_2026-11-02.json) for source-attributed screening values.

The referenced 2026 study reports tank-included volumetric energy density: MGO 10.1 MWh/m³; methanol 4.4; LNG/bio-LNG 3.1; liquid ammonia 2.9; liquid hydrogen 1.3. For equal stored LHV energy these correspond to approximately 2.30×, 3.26×, 3.48× and 7.77× the MGO tank-system volume, respectively. Its tank-included gravimetric values are also retained in the dataset.

Do not apply these factors as universal ship-design multipliers. The tank type, design pressure, temperature, insulation, containment, fill limit and safety clearances are study-specific. MARAD's December 2024 guide separately estimates ~7.6× MGO tank volume for liquid hydrogen, ~4× for ammonia and ~2.3× for methanol; its ammonia volumetric LHV is 12.7 MJ/L. MARAD's older 2022 guide gives ~4.1× for ammonia, while a 2023 Nordic report gives 7.2 MJ/L for ammonia in a 20-bar tank. Keep these distinct source assumptions rather than averaging them.

For each route and vessel, report:
- usable fuel energy at the declared reserve policy;
- tank internal volume and complete tank-system mass;
- fuel mass, tank mass and tank-system footprint;
- displacement and lost cargo payload;
- range/endurance at the matched route and speed/load profile;
- bunker availability, time, frequency and terminal compatibility;
- fuel conditioning, ventilation, thermal management, leak/toxicity/fire containment and maintenance access;
- lifecycle GHG per delivered tonne-nautical-mile.

A battery is not a fuel: compare usable electrical storage energy, charge source, state-of-charge window, round-trip loss, power limit, degradation and pack mass/volume separately. A vessel comparison must use delivered cargo work; tank-volume ratios alone are not a ship feasibility or safety result.

## 9. Measurement quality and statistical design

- Use a competent testing/calibration laboratory and record the accreditation scope where required.
- Record calibration certificates, due dates, range, resolution, accuracy and traceability for fuel-flow, torque, speed, electrical power and emissions instrumentation.
- Define steady-state detection and averaging windows before reviewing results.
- Include repeated baseline runs before and after fuel/configuration changes to reveal drift.
- Randomize candidate run order where practical; do not test every candidate in an order that confounds ambient or equipment drift.
- Include negative controls: intentionally wrong fuel LHV, omitted auxiliary load, swapped gross/net measurement point, stale engine map, mismatched duty cycle, and missing emissions pathway.
- Report measurement uncertainty separately from model uncertainty and run-to-run variation.
- Preserve raw and derived data digests, configuration SHAs, sensor calibration identifiers and test operator/lab identity.

## 10. South African test-environment extension

Add a separate application profile, not an uncorrected universal derating factor:
- ambient temperature and humidity envelope sourced from the intended site;
- site altitude / barometric pressure;
- intake dust loading and filtration maintenance;
- cooling-system approach temperature and heat rejection;
- specification-bounded local fuel-batch variation verified by laboratory analysis;
- load profile and power quality of the actual industrial consumer;
- remote-service interval, spare-parts lead time and technician availability;
- grid parallel / islanding constraints and load shedding where applicable.

A South African scenario may not be populated with guessed local fuel prices, grid emission factors or environmental averages. Use dated sources and preserve unknowns.

**National listed-activity screening note (2026-11-02):** the consolidated NEMA: Air Quality Act listed-activities text identifies Subcategory 1.5 as liquid- and gas-fuel stationary reciprocating engines used for electricity generation, with an application threshold of at least 10 MW thermal input per unit (LHV basis). This is a heat-input threshold, not electrical output. The provisional 50–250 kWe demonstrator is expected to be materially smaller, but do not infer a blanket exemption from this note. Verify the currently operative Gazette/consolidated notice, any provincial MEC-listed activities, municipal air-quality requirements, site-specific approvals, fuel storage and fire requirements, electrical installation rules, occupational safety and noise obligations with the relevant authorities before procurement or operation. A threshold screen does not confer operating permission.

## 11. Decision outputs

For each candidate publish:
- evidence class and source for every input;
- engine BTE and net generator efficiency separately;
- fuel mass and fuel energy per net kWh;
- segment-by-segment and weighted cycle results;
- auxiliary consumption and conversion losses;
- transient performance measured values or explicit "not modelled";
- pollutant emissions and lifecycle intensity with separate boundaries;
- uncertainty and sensitivity;
- uptime / maintenance evidence or explicit "not measured";
- cost breakdown with missing categories preserved as unknown;
- Pareto comparison rather than one opaque aggregate score;
- independent verifier output and claim ceiling.

No candidate is called "better than SOTA" without a matched rating, duty cycle, fuel batch, measurement boundary, test method and uncertainty comparison against a current reference.

## 12. Sources

- ISO 8528-1:2018 and revision status: https://www.iso.org/standard/68539.html
- ISO/DIS 8528-1: https://www.iso.org/obp/ui?_escaped_fragment_=iso%3Astd%3Aiso%3A8528%3A-1%3Adis%3Aed-4%3Av1%3Aen
- ISO 8528-5:2025: https://www.iso.org/standard/85960.html
- ISO 8528-6:2023: https://committee.iso.org/cms/live/live/en/sites/isoorg/contents/data/standard/07/10/71010.html?browse=ics
- ISO 8178-1:2020: https://www.iso.org/standard/79330.html
- ISO 8178-2:2021: https://www.iso.org/standard/75546.html
- IMO NOx Regulation 13 / NOx Technical Code: https://www.imo.org/en/ourwork/environment/pages/nitrogen-oxides-%28nox%29-%E2%80%93-regulation-13.aspx
- DOE SuperTruck 2 results: https://www.energy.gov/cmei/vehicles/articles/fotw-1346-june-10-2024-us-doe-supertruck-3-initiative-seeks-reduce
- Cummins HELM X10/X15: https://www.cummins.com/en-ame/engines/on-highway/fuel-agnostic
- Wärtsilä methanol engines: https://www.wartsila.com/marine/products/engines-and-generating-sets/methanol-engines
- Everllence ammonia engine Type Approval / FAT milestones: https://www.everllence.com/marine/products/two-stroke-engines/ammonia-engine
- PNNL / Nature Energy heavy-duty lifecycle comparison: https://www.pnnl.gov/publications/cradle-grave-lifecycle-analysis-us-medium-and-heavy-duty-vehicle-fuel-pathways
- Mainspring linear generator product page: https://www.mainspringenergy.com/product/
- Mainspring 2.5 MW / 10 × 250 kW datasheet R40322: https://linear-power.files.svdcdn.com/production/Mainspring-Linear-Generator-2.5MW-Datasheet-R40322.pdf
- California Energy Commission / Mainspring, CEC-500-2024-037 report page: https://www.energy.ca.gov/publications/2024/high-efficiency-and-ultra-low-emissions-linear-generator-demonstration-project
- California Energy Commission / Mainspring, CEC-500-2024-037 PDF: https://www.energy.ca.gov/sites/default/files/2024-05/CEC-500-2024-037.pdf
- U.S. Department of the Air Force / AFMC, 250 kW Mainspring multi-fuel pilot (4 March 2026): https://www.afmc.af.mil/News/Article-Display/Article/4421866/air-force-pilots-fuel-flexible-generator-for-travis-air-force-base/
- Mainspring announcement of its U.S. Air Force pilot (4 March 2026): https://www.mainspringenergy.com/news/mainspring-energy-awarded-u-s-air-force-pilot-project-for-multi-fuel-resilient-power-generation-at-travis-air-force-base
- Mainspring / AEP up-to-1.6 GW agreement (13 October 2026): https://www.mainspringenergy.com/news/mainspring-energy-signs-landmark-1-6-gw-agreement-with-aep-for-reliable-affordable-power-capacity
- ASME Performance Test Codes catalogue (PTC 50 listed for Fuel Cell Power Systems Performance): https://www.asme.org/codes-standards/publications-information/performance-test-codes
- Capstone Energy+ FY2026 Form 10-K: https://www.sec.gov/Archives/edgar/data/1009759/000110465926077817/cgeh-20260331x10k.htm
- Capstone FY2025 Form 10-K: https://www.sec.gov/Archives/edgar/data/1009759/000155837025008930/cgeh-20250331x10k.htm
- South African National Environmental Management: Air Quality Act, 2004: https://www.gov.za/documents/national-environment-management-air-quality-act
- South African listed-activities notice and amendments: https://www.gov.za/documents/notices/national-environmental-management-air-quality-act-listed-activities-and
- Consolidated listed-activities text (screening reference; verify operative Gazette): https://www.saflii.org/za/legis/consol_reg/loawriaewhomhasdeoteihscececoch2046.pdf

This protocol establishes a testable comparison process. It does not replace licensed standards, laboratory procedures, a hazard review or regulatory approval.
