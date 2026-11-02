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
3. **Delivered-bus boundary:** net AC/DC energy delivered after inverter, battery, rectifier and distribution losses, as applicable.

Declare the measurement point, power factor, voltage/frequency, grid-parallel or islanded mode, ambient conditions, fuel temperature/state, and every included/excluded auxiliary.

Do not compare an engine BTE result against a net AC generator-set efficiency result.

## 4. Baseline and candidate families

Minimum comparison set:
1. Current production diesel generator set of similar rating and application class.
2. Same-set HVO/renewable-diesel compatibility configuration where the manufacturer and fuel specification permit it.
3. A separate natural-gas/biomethane dual-fuel candidate only after its gas train, pilot-fuel fraction, ventilation, leak detection, shutdown and safety case are established.
4. Battery-electric storage/inverter supply for the same load trace where a credible grid/charging source is available.
5. Optional fuel-cell generator only when fuel supply, storage, parasitics, operating life and cost data are credible.

Biodiesel, methanol, hydrogen and ammonia are separate fuel/configuration studies. No implicit drop-in or instant fuel-switch assumption is permitted.

A reference engine map must be measured or sourced with traceable evidence. If no valid map exists, the model must mark the operating point unknown and refuse quantitative ranking.

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

This protocol establishes a testable comparison process. It does not replace licensed standards, laboratory procedures, a hazard review or regulatory approval.
