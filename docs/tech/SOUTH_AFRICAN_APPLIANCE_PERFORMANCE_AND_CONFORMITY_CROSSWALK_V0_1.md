# South African Appliance Performance and Conformity Crosswalk v0.1

**Status:** desk-research baseline and proposed benchmark programme. This is not legal advice, a certification, a laboratory report, a product design freeze, or evidence that any Luminous Dynamics product is compliant or outperforms industry.  
**Research cut-off:** 21 November 2026  
**Parent programme:** [APPLIANCE-PORTFOLIO-001 #1825](https://github.com/Luminous-Dynamics/symtropy/issues/1825), [APPLIANCE-QUAL-001 #1805](https://github.com/Luminous-Dynamics/symtropy/issues/1805), [APPLIANCE-WASHER-001 #1828](https://github.com/Luminous-Dynamics/symtropy/issues/1828), [cross-family morphology #1893](https://github.com/Luminous-Dynamics/symtropy/issues/1893).  
**Industrial owners to compose:** [pump system energy/service #5103](https://github.com/Luminous-Dynamics/symtropy/issues/5103), [pump service benchmark #4959](https://github.com/Luminous-Dynamics/symtropy/issues/4959), [thermal pinch #1899](https://github.com/Luminous-Dynamics/symtropy/issues/1899), [thermal dynamics #1916](https://github.com/Luminous-Dynamics/symtropy/issues/1916), [manufacturing twin VVUQ #3121](https://github.com/Luminous-Dynamics/symtropy/issues/3121).  
**Evidence owner:** [Mycelix APPLIANCE-EVID-001 #5156](https://github.com/Luminous-Dynamics/mycelix/issues/5156).

## 1. Executive decision

Build one evidence and engineering platform, not one universal physical appliance and not one undifferentiated compliance score.

The programme should share:
- product, module, unit, design-revision and supplier identity;
- test-protocol and instrument/calibration identity;
- functional-capability and service-task descriptions;
- diagnostics, safe-state and evidence semantics;
- repair/restoration, parts-compatibility and lifecycle-cost accounting;
- manufacturing process, inspection and nonconformance records.

Keep physical modules and safety cases product-specific. A pump, washer drain assembly, refrigerator compressor and HVAC fan do not become interchangeable because they share a connector, dimension, model, or evidence schema.

The most valuable South African starting wedge is a **locally serviceable appliance and industrial-utility platform with credible independent metrology**. It responds to a real policy and capability gap while giving Luminous a path from household products into industrial pumps, motors, compressed air and process heat.

## 2. South African policy and market context

### 2.1 Energy-efficiency policy

The Department of Electricity and Energy published a **draft National Energy Efficiency Strategy 2025–2035** in Government Gazette Notice 4221 on 30 October 2026. It is a draft policy proposal, not an enacted product requirement. The draft identifies outdated minimum energy performance standards and labels, insufficient accredited testing capacity, and opportunities in household appliances and industrial energy use. It states that thermal processes account for approximately 77% of industrial energy demand. Separately, it estimates that the full set of industrial efficiency interventions could reduce industrial energy consumption by 40.7 PJ by 2035; that 40.7 PJ figure is the industry-sector total, not a thermal-process-only saving. These are draft-strategy estimates, not verified savings available to a particular plant.

Sources:
- Draft NEES notice and Gazette PDF (Notice 4221, 30 October 2026): https://www.gov.za/sites/default/files/gcis_document/202610/55638gen4221.pdf
- Department appliance labelling / MEPS programme: https://www.energy.gov.za/EEE/Appliances.html

**Engineering implication:** do not stop at domestic product labels. Create a measurement capability that spans household appliances and industrial utility systems. For factories, measure useful thermal service, hydraulic service and delivered compressed air at the process boundary—not just motor nameplate efficiency.

### 2.2 Current South African compulsory-specification context

The South African compulsory-specification and Letter of Authority (LOA) pathway is product-category and model specific. NRCS guidance identifies household electrical appliances in the VC 8055 scope as regulated products, with an LOA required before import where applicable. An overseas test report, an IEC/CB report, an SABS mark, an NRCS LOA, a manufacturer declaration and an internal engineering measurement are separate evidence objects; none should be substituted for another.

The 2026 amendment to VC 9008 appears in Government Gazette 55257 (28 August 2026). An Acts Online transcription identifies it as Notice 7859 of 2026 and states that Phase 1 takes effect 12 months after publication and Phase 2 24 months after publication. On the Gazette date, those intervals fall on 28 August 2027 and 28 August 2028 respectively; confirm the numbering, transitional clauses and exact product schedule against the Gazette original before making a market-placement decision. The published schedule must be checked for the specific product class, MEPS threshold, test standard, label and scope.

Sources:
- NRCS compulsory specifications: https://www.nrcs.org.za/compulsory-specifications/
- NRCS LOA information: https://www.nrcs.org.za/letter-of-authority/
- Official Government notice index (Government Gazette 55257, 28 August 2026): https://www.gov.za/documents/notices?%3Bpage=1&page=15
- Searchable text of the 2026 VC 9008 amendment (Acts Online transcription; verify against the Gazette original): https://www.acts.co.za/national_/n7859_notice_no__7859_of_2026

**Gate before any South African market claim:** for the exact manufacturer, brand, model/SKU, hardware revision, rated voltage/frequency, intended use and importer, obtain a competent applicability determination. Capture the applicable VC/SANS/IEC edition, LOA holder and scope where required, test report scope, importer/warranty obligations, plug/cord compatibility, EMC/radio scope where connected, and product-specific energy-labelling status. A component certificate does not certify the assembled appliance.

### 2.3 Product stewardship and circularity

South Africa's national EPR and waste-electrical/electronic-equipment policy context means end-of-life handling, take-back, reuse, repair and material recovery should be designed into the business model, not added after product launch. The product evidence model should preserve material declarations, repair history, refurbishment status and recovery route.

Source: https://www.gov.za/documents/notices/environmental-affairs-national-environmental-management-waste-act-national-policy-management-waste-electrical-and-electronic-equipment-29-aug-2025

## 3. Safety and test-standard map

This is a starting map, not a determination of applicable law. The exact product type, household/commercial/industrial use, installation, refrigerant, voltage, national adoption and amendment status govern the actual standard set.

| Product family | International standards / test references to investigate | Important boundary |
|---|---|---|
| General household appliances | IEC 60335-1:2020 + AMD1:2025 | Apply with the relevant Part 2 and confirm current SANS adoption and NRCS scope. |
| Washing machines | IEC 60335-2-7:2024 for safety; IEC 60456:2024 Edition 6 for household performance; IEC 60704-2-4:2025 Edition 4 for airborne noise | Keep safety, performance, noise, South African SANS adoption and regulatory/LOA scope distinct. IEC 60456:2024 is a major technical revision; do not assume existing local laboratory scope or legacy product-sheet values automatically align. |
| Household dishwashers | IEC 60335-2-5:2025; applicable dishwasher performance procedure | Commercial dishwashers have a different safety part, e.g. IEC 60335-2-58. |
| Refrigerators/freezers | IEC 60335-2-24:2025; applicable energy and temperature-performance methods | Refrigerant handling and sealed-system repairs remain specialist service boundaries. |
| Heat pumps, air conditioners and dehumidifiers | IEC 60335-2-40:2024 + AMD1:2026 | Refrigerant class, charge, flammability and installation/site rules require product-specific treatment. |
| Motors | IEC 60034-30-1:2025 efficiency classes where within scope | IE class is a motor rating; a variable-speed appliance's total system performance requires a full operating map. |
| Pumps | ISO 9906:2012 acceptance tests for rotodynamic pumps | Match flow, head, fluid, speed, impeller and acceptance grade; do not infer whole-system energy performance from a pump certificate alone. |
| Displacement compressors | ISO 1217:2009 + AMD1:2016 (check current revision status before test freeze) | Measure package input power and delivered flow at specified pressure and inlet conditions; system leaks/treatment/controls are separate. |
| Industrial energy management | ISO 50001:2018 | Management-system standard, not product certification or a guaranteed savings claim. |

Reference pages:
- IEC 60335-1:2020+AMD1:2025 consolidated record: https://webstore.iec.ch/en/publication/85018
- IEC 60335-2-7:2024: https://webstore.iec.ch/en/publication/80076
- IEC 60335-2-5:2025: https://webstore.iec.ch/en/publication/101702
- IEC 60335-2-24:2025: https://webstore.iec.ch/en/publication/75938
- IEC 60335-2-40:2024+AMD1:2026 consolidated record: https://webstore.iec.ch/en/publication/104758
- IEC 60034-30-1:2025: https://webstore.iec.ch/en/publication/91195
- ISO 9906: https://www.iso.org/standard/41202.html
- ISO 1217: https://www.iso.org/standard/35807.html
- ISO 50001: https://www.iso.org/standard/69426.html

Before a physical test plan is frozen, record the exact edition/amendments from the standards body catalogue, applicable local adoption and accredited laboratory scope. Do not copy a foreign test procedure into a South African conformity file without confirming its acceptance and relevance.

## 4. International market benchmarks: use as floors and reference points, not automatic SOTA

The following are published voluntary US ENERGY STAR thresholds and programmes. They are **not South African MEPS**, not equivalent to the applicable local test procedure, and not proof of the best product available in 2026. Use them as public comparison anchors, then freeze real products from the target market and measure them under a common protocol.

| Product | Published reference threshold | How to use it responsibly |
|---|---|---|
| Front-load clothes washer | ENERGY STAR current published criteria page: IMEF ≥ 2.76 and IWF ≤ 3.2 for front-loading residential washers over 2.5 cu ft | Convert the rating into a locally relevant test basket; compare actual cleaning/rinsing, kg dry load, water and energy at matched programme and temperature. |
| Standard-size dishwasher | ENERGY STAR: ≤ 240 kWh/year and ≤ 3.2 US gal/cycle for standard-size products | Keep capacity/place settings and cleaning/rinse outcome matched; annual use assumptions and local use patterns differ. |
| Integrated electric heat-pump water heater | ENERGY STAR: UEF ≥ 3.30; First Hour Rating ≥ 45 US gal/h | Report delivered hot-water temperature, draw profile, ambient condition, standby/recovery, noise and backup-element use, not UEF alone. |
| Refrigerator/freezer | ENERGY STAR Most Efficient 2025 recognizes standard-size refrigerator-freezers and freezers at AEC ≤ 637 kWh/year; non-top-freezer refrigerator-freezers must also be at least 27% more efficient than the US federal requirement | Do not apply one kWh/year number across different capacities, configurations, ice makers or freezer arrangements. Normalize by usable volume and thermal service. |

Sources:
- ENERGY STAR clothes washer key product criteria: https://www.energystar.gov/products/clothes_washers/key_product_criteria
- ENERGY STAR dishwashers: https://www.energystar.gov/products/dishwashers
- ENERGY STAR residential water heaters: https://www.energystar.gov/products/water_heaters
- ENERGY STAR Most Efficient 2025 criteria PDF: https://www.energystar.gov/sites/default/files/2024-09/Consumer%20Refrigeration%20Products%20ENERGY%20STAR%20Most%20Efficient%202025%20Criteria.pdf

For any product class not shown above, do not invent a threshold. Identify the current local requirement, best independent product dataset, product-specific test method and a frozen competitor set before setting a numeric comparison claim.

Add two international policy/market comparison layers to the source register:
- CLASP, **World's Best MEPS 2023**, which compares minimum-efficiency policies for major appliances across multiple economies and identifies where national requirements lag model-regulation / best-available-technology levels: https://www.clasp.ngo/worlds-best-meps-2023/
- IEA 4E Product Energy Efficiency Trends (PEET), which tracks standards, labels and product-efficiency trends across member economies: https://www.iea-4e.org/publications/peet/status-of-domestic-refrigerator-regulations/

These are useful for setting an international ambition ladder and checking whether a foreign label is actually a frontier. They are not interchangeable test results: align product class, capacity, climate, duty profile, test edition and metric before transferring a threshold.

## 4A. Washer-specific standards update and South African laboratory-scope gap

The current household washer performance method is **IEC 60456:2024, Edition 6**, published 27 November 2024 and shown as valid with stability date 2028. It replaces IEC 60456:2010 plus Amendment 1:2022. The revision materially changes the performance evidence available: it adds new standard powder/liquid/wool detergents; cleaning rather than the prior washing-performance formulation; wool cleaning; LAS rinsing; gentleness of action; low-power modes; temperature inside the washload; intermittently recurring functions; alternative conditioning/ambient corrections; mixed textile loads; and multi-compartment machines. It does not cover commercial laundries and does not itself define acoustical-noise measurement.

**The benchmark consequence:** energy per cycle is no longer an adequate standalone washer objective. A credible comparison should preserve cleaning, rinse residue/LAS, gentleness/textile care, temperature inside the load, spin residual moisture, low-power energy, cycle time, water and electricity as separate outcomes, with the applicable prescribed test materials and conditioning. A separate long-horizon textile durability study may extend the scope, but must not be represented as a direct IEC 60456 requirement.

For washer noise, **IEC 60704-2-4:2025 Edition 4** is now the current IEC method for washing machines and spin extractors. It is aligned to IEC 60456:2024 test programmes and detergents. For washer safety, **IEC 60335-2-7:2024 Edition 9** is the latest IEC product-specific safety part identified in the catalogue. The exact relevant Part 1 amendments, SANS adoption, applicability and NRCS scope still require competent local determination.

References:
- IEC 60456:2024: https://webstore.iec.ch/en/publication/70049
- IEC 60704-2-4:2025: https://webstore.iec.ch/en/publication/90407
- IEC 60335-2-7:2024: https://webstore.iec.ch/en/publication/80076
- IEC 60335-1:2020+AMD1:2025 consolidated record: https://webstore.iec.ch/en/publication/85018

**South African method transition remains unresolved.** Public sources confirm that SABS has an ISO/IEC 17025-accredited Appliances Laboratory conducting household appliance safety, performance and energy-efficiency testing. They do not, on the public laboratory page alone, prove that its current accredited schedule includes IEC 60456:2024 Edition 6 or IEC 60704-2-4:2025. Legacy public documents reference older and inconsistent washing-machine performance methods (including SANS 60456 / IEC 60456:2003 and SANS 1695). These legacy references are not sufficient to establish the current SANS adoption or laboratory scope.

Required pre-test action: retrieve the live SABS standards catalogue status for the current SANS method; obtain the current SANAS certificate and detailed scope for the exact SABS Appliances Laboratory; confirm whether the laboratory can perform IEC 60456:2024 as an accredited test, a non-accredited customer-specific engineering test, or cannot yet perform it; record the status separately. If external testing is considered, verify the exact method scope of the ILAC-MRA laboratory and separately confirm what NRCS accepts for any regulatory/LOA purpose.

Sources:
- SABS Appliances Laboratory: https://www.sabs.co.za/appliances
- SANAS latest accredited-facilities database: https://www.sanas.co.za/Pages/website/accreditation/load/latest-accredited-facilities.aspx
- Historical DMRE/SABS scope-capacity document (legacy 2003 reference): https://www.dmre.gov.za/Portals/0/Energy_Website/EEE/Projects/EnergyEfficiencyStandardsAndLabellingForHouseholdAppliances/Capacity-to-test-for-newEnergyEfficiencyStandards-SABS-Lab.pdf
- Historic UNDP/SABS appliance-laboratory tender (legacy SANS 1695 reference): https://procurement-notices.undp.org/view_file.cfm?doc_id=143929

Related source-grounded product candidate register: [South African Washer Comparator Discovery Register v0.1](SOUTH_AFRICAN_WASHER_COMPARATOR_DISCOVERY_REGISTER_V0_1.md). It is a procurement discovery register only: Bosch WAN282X1ZA, Miele WWA120 WCS and LG F1222TD5 are documented candidate products, while a Samsung regional page that currently says "Not for sale" is explicitly excluded from local procurement. Their published energy figures use different methods/label regimes and are not ranked.

## 5. Shared benchmark contract: compare useful service, not label numbers

### 5.1 Freeze comparator identity

For each product family:
1. Select at least three comparable market models based on a written selection rule (capacity, topology, price band, intended use, market availability and service ecosystem).
2. For a repeatability baseline, use multiple physical units of the **same model and revision**; three different models are not a three-unit repeatability set.
3. Preserve exact model, SKU, serial/lot, hardware and firmware revision, accessories, type plate, purchase date/price, warranty, manuals, service route, spares and test condition.
4. Acquire South African units where the first market is South Africa. Retail listings are discovery evidence, not proof of delivered unit identity or market approval.
5. Keep refurbished/used donor teardowns separate from functioning control tests.

### 5.2 Normalize by delivered function

| Family | Primary useful-output denominator | Required quality and boundary measures |
|---|---|---|
| Washer | kWh and litres per kg of dry textile treated under a defined programme and load | Standardized soil removal, rinse residue, spin residual moisture, cycle time, textile damage, detergent, water hardness and temperature. |
| Dishwasher | kWh and litres per accepted standardized load / place-setting basket | Cleaning, drying, rinse residue, capacity, cycle time, sound and cleaning-product boundary. |
| Refrigerator/freezer | kWh per usable litre-day at specified compartment temperatures | Temperature stability/uniformity, ambient condition, door openings, frost/defrost, usable volume, noise and food-safety limits. |
| Heat-pump water heater | kWh electricity per kWh of hot water delivered at specified temperature and draw profile | First-hour delivery, recovery, standby, ambient, backup resistance use, noise, Legionella/hygiene controls and safety requirements. |
| Dryer | kWh per kg water removed to a defined textile endpoint | Residual moisture, time, textile damage, condensate capture, filter pressure drop and maintenance. |
| HVAC / ventilation | kWh per unit of conditioned-air service (temperature/humidity/airflow) across a climate/load profile | Comfort envelope, humidity, filtration, ventilation rate, noise, cycling, defrost and part-load performance. |
| Pump system | kWh per m³ delivered at specified flow, head, fluid and duty schedule | Pump + motor + drive + pipe/valve losses, throttling, leakage, control strategy and useful-pressure compliance. |
| Compressed air | kWh per Nm³ delivered at defined pressure, temperature, dryness and quality class | Package specific power, leakage, pressure drop, treatment, storage, part-load unloading and demand profile. |
| Industrial process heat | Primary energy per unit of useful heat delivered at the required temperature and time profile | Heat quality, losses, heat recovery, storage, product quality, throughput, emissions and process safety. |

Where output quality cannot be measured credibly, the result is descriptive and no efficiency-superiority claim is allowed. Do not use command state, motor current, a fault code or a simulation as a proxy for delivered function.

### 5.3 Whole-life cost and environmental boundary

Calculate separately:
- owner purchase, installation and financing cost;
- electricity, water, consumables and maintenance;
- diagnosis, parts, logistics, repair labour, failed attempts, repeat visits and downtime;
- warranty, spare-part availability and refurbishment;
- embodied material/production impact, refrigerant leakage where relevant, and end-of-life reuse/recovery.

Publish the assumed duty profile, tariff/date, water price, discount rate, service geography, lifespan scenario and missing fields. An unknown cost, failure rate or emissions factor is not zero. Report uncertainty and sensitivity. A lower purchase price or BOM is not a lifecycle win by itself.

## 6. Research targets: ambitious hypotheses, not claims

Set product-specific targets after the control baseline and instrument capability are known. The following are *provisional R&D gates* for architecture selection, not achieved results or universal requirements.

| Area | Provisional target to test against frozen controls | Non-compensatory condition |
|---|---|---|
| Washer | ≥ 15% lower energy per kg and/or water per kg at matched load and programme; pursue ≥ 20% combined resource improvement where the Pareto frontier permits | Cleaning, rinsing, textile care and cycle-time acceptance must be non-inferior; keep separate energy and water claims. Existing ≥25% median and P90 restoration-time hypothesis in #1828 remains separate. |
| Dishwasher | ≥ 15% lower kWh and water per accepted load | Cleaning, drying, rinse and capacity outcomes remain non-inferior. |
| Refrigerator/freezer | ≥ 15% lower kWh per usable litre-day | Temperature stability, usable capacity and safety remain non-inferior. |
| Heat-pump water heater | Design target UEF ≥ 3.6 for a compatible integrated product profile | Reproduce equivalent draw profile, ambient condition, first-hour rating and temperature delivery; verify local market applicability and hygiene/safety. |
| Dryer | ≥ 15% lower kWh per kg water removed | Match final moisture and textile care; include cycle time, filter/coil cleaning and condensate. |
| Pump system | ≥ 15% lower kWh/m³ at matched hydraulic service over the real duty cycle | Minimum/maximum pressure and flow, cavitation, reliability and process output pass independently. |
| Compressed air | ≥ 10% lower package/system specific energy at matched pressure and air quality | Delivered flow, dew point/contaminant limits, reliability and production quality are not degraded. |
| Process heat | ≥ 10% lower primary energy per useful thermal unit in the target temperature/time window | Product quality, throughput, heat-source constraints and process safety are maintained. |
| Repairability | Retain the existing ≥25% lower median and P90 verified-restoration-time hypothesis in #1805/#1828 | Failed attempts, safety stops, parts delays, rework and recurrence remain in the denominator/reporting. |
| Manufacturing | Establish baseline first; then target ≥10% improvement in first-pass yield or throughput without increased escape defects or energy/material burden | No production-yield claim without a frozen process boundary, representative lot and traceable inspection data. |

**Do not sum these targets into a single weighted score.** Use a constrained Pareto frontier: safety/conformity, functional quality and essential performance are gates; energy, water, repairability, cost, durability, embodied impact and manufacturability are trade-off objectives.

If baseline uncertainty is larger than the proposed effect, improve metrology or increase samples before changing the target. If no design meets a target, publish the result and retain the real frontier.

## 7. How to outperform the state of the art credibly

Use a four-level comparison ladder:

1. **Regulatory floor:** applicable South African MEPS/VC/SANS requirements and market-entry scope.
2. **Voluntary benchmark:** ENERGY STAR, IEC/ISO test references, sector datasets and recognized independent schemes.
3. **Best available market products:** exact local models and revisions, not generic marketing claims or a hand-picked weak comparator.
4. **Independent matched testing:** common protocols, calibrated instruments, representative samples, blinded output scoring where practicable, raw observations, uncertainty and independent replication.

A design is not “better than SOTA” because it beats one retailer listing or one old appliance. Define SOTA for each product family as a reproducible set of top-performing, comparable units with transparent inclusion rules. Report the median and spread of comparators, not only the weakest control. Preserve unfavorable outcomes, and do not retune the baseline after seeing candidate results.

Required experiments:
- factorial/ablation tests for service spine × repair granularity × diagnostics, using the existing #1910 plan rather than a new experiment;
- measured simulation-to-test residuals for Symtropy physical models;
- design trade-offs with non-modular controls to detect modularity penalties;
- diagnostic top-k recall, calibration, abstention and false reassurance on held-out seeded and physical faults;
- independent rebuilds by people other than the design author;
- lifecycle sensitivity analysis across electricity tariffs, water prices, part availability and use profiles.

### External-tool validation and industrial heat-pump scope (research update: 21 November 2026)

The first industrial demonstrator should use the existing whole-system pump programme, but its comparison with industry tools must be explicit. DOE PSAT / MEASUR is a recognized screening and opportunity-quantification comparator that uses measured or estimated field inputs and achievable pump/motor performance assumptions. It is not a physical test, nor a complete validation oracle for every operating trajectory.

A dedicated comparator task is now tracked in [PUMP-PSAT-DIFF-001 #6561](https://github.com/Luminous-Dynamics/symtropy/issues/6561), under [PUMP-SYSTEM-ENERGY-SERVICE-001 #5103](https://github.com/Luminous-Dynamics/symtropy/issues/5103). The required sequence is:
1. reproduce independently hand-checkable arithmetic;
2. compare the shared input/output domain against a version-pinned PSAT / MEASUR reference;
3. explain differences caused by boundaries, defaults, pump/motor curves, source datasets and rounding;
4. declare unsupported fields and unrepresentable cases;
5. only then evaluate Symtropy's additional trajectory, storage, service-floor, uncertainty, degradation and lifecycle decision capabilities as separately scoped extensions.

The intention is to establish fair parity where the methods overlap and to demonstrate additional value where Symtropy has a valid, testable capability—not to claim that it replaces DOE tools or relevant ISO/ANSI standards.

For industrial heat pumps, the IEA's 2026 Heat Pump Monitor notes that applications below about 120°C have entered commercial operation, systems below about 160°C are beginning to enter commercial use, while higher-temperature ranges remain mainly demonstration, prototype or concept work. It also highlights integration into existing production processes, electricity-to-fuel price ratios and qualified technician availability as major deployment constraints. Therefore, our first process-heat studies should prioritize low- and medium-temperature heat sources/sinks with explicit temperature lift, schedule and cost conditions; higher-temperature concepts must retain a research/demonstration label until the exact system has credible evidence. The IEA also reports that global heat-pump manufacturing capacity and upstream compressor concentration create supply-chain dependencies, strengthening the case for qualified second sourcing and repair capability rather than assumptions of immediate local compressor manufacture.

Reference: IEA, *Heat Pump Monitor 2026 — Key findings*: https://www.iea.org/reports/heat-pump-monitor-2026/key-findings

## 8. Luminous system responsibilities and authority limits

| System | Appropriate role | Hard boundary |
|---|---|---|
| Symtropy | Thermofluid/mechanical/system models, experiment design, multi-objective trade-offs, manufacturing twins and simulation VVUQ. | A model is not physical evidence or a conformity report; report validation domain and residuals. |
| Symthaea | Advisory fault-hypothesis ranking, evidence synthesis, uncertainty and next-test/value-of-information suggestions. | No safety interlocks, hazardous actuation, certification or autonomous repair authority. Preserve abstention. |
| Mycelix | Bind design release, exact BOM, serialised unit, supplier/part provenance, work performed, actual parts, test receipts and bounded recurrence follow-up. | A signed record does not prove a physical measurement happened or that a product is safe. Reuse #5156 and existing manufacturing/repair owners. |
| Sol-Atlas | Product/process knowledge, source and licence lineage, training/service procedures, local capability and skills maps. | A knowledge record does not qualify a supplier, worker, part or process. |
| Physical product and test system | Deterministic protection, local essential function, calibrated sensors, safety procedures and testable physical output. | The cloud, DHT, AI or dashboard is never a required safety function. |

## 9. South African capability strategy

Prioritize capabilities that can be used by multiple product families and local suppliers:
- a traceable energy/power metrology bench with calibration evidence;
- water-flow, pressure, temperature, humidity, airflow, sound and vibration measurement fixtures;
- motor/drive characterization and dynamometer access through an accredited partner;
- pump-loop test capability with a characterized hydraulic duty cycle;
- compressed-air leak and package-specific-power audits with pressure/quality metrology;
- thermal measurement and heat-pump test capability with safe refrigerant handling through qualified partners;
- product teardown, fastener/material identification, repair-time and post-restoration verification fixtures;
- supplier qualification, incoming inspection, traceable substitutions and repair-spares logistics.

Because the draft NEES identifies accredited-laboratory capacity as a constraint, build a **partner-first test network** rather than pretending a small prototype workshop is an accredited certification laboratory. Record each lab's ISO/IEC 17025 scope, calibration traceability, method, uncertainty and product category. Internal rigs are engineering tools until separately validated and accepted for the intended purpose.

## 10. Execution sequence

### Stage A — evidence freeze
- Freeze a market and use profile for one washer family and one industrial utility family (recommend pump system first because matched hydraulic service can be measured without developing a complete new appliance).
- Choose comparator models by published criteria and actual South African availability; preserve exact model/revision identity.
- Download and version applicable standards/catalogue entries and determine South African compulsory-specification applicability with a competent specialist.
- Freeze functional output, instrument requirements, protocols, uncertainty budget, sample-size rationale and non-inferiority margins before collecting confirmatory results.

### Stage B — measurement pilots
- Validate instrument calibration, resolution, boundary placement and repeatability using existing products.
- Complete control-only pilot measurements before candidate design comparisons.
- Capture all failures, interruptions, missing data, instrument faults and protocol deviations.
- Publish a reproducible test bundle and digest-bound receipts; do not label pilot data as a product qualification.

### Stage C — design and build
- Use the existing washer and appliance morphology work for household architecture.
- Use existing pump and process-heat work for industrial platforms.
- Create modularity only where the matched experiment demonstrates service, cost, performance or recovery benefit.
- Begin with low-hazard test fixtures and certified commercial appliances; specialist labs/qualified personnel own mains, pressure, refrigerant, thermal and rotating machinery tests.

### Stage D — replication and market decision
- Replicate selected results on additional units and by an independent builder/tester.
- Complete applicable laboratory and regulatory assessment for exact final product revision.
- Publish the complete performance envelope, limitations, repair process, spares policy, warranty and evidence bundle before any market claim.

## 11. Acceptance criteria for this crosswalk

- Every regulatory claim is tied to a current primary source and exact product scope.
- Every product benchmark names its procedure, edition, use profile, comparator identity and measurement boundary.
- ENERGY STAR and other foreign voluntary labels are explicitly not represented as South African MEPS or certification.
- Proposed targets are stored separately from measured results.
- No product-family efficiency result is accepted without a matched useful-output/quality denominator.
- Safety and conformity are independent gates and are never averaged into an efficiency score.
- Missing, stale, contradictory or uncalibrated evidence stays unknown or is quarantined; it is never silently coerced to zero or pass.
- Industrial and household products share evidence contracts but retain product-specific hardware, hazards and test methods.
- Existing Symtropy / Mycelix owners are reused; no second universal repair ledger, work-order database, provenance system, or safety authority is created.

## 12. Open questions

1. Which exact South African washer models and revisions are available for the first control set at acquisition time?
2. Which local laboratories have an active ISO/IEC 17025 scope for each target product and exact method?
3. What are the amended VC 9008 product-category thresholds and phase dates when mapped to the intended SKUs?
4. Which pump loop / industrial site gives the clearest representative duty profile and an independently metered baseline?
5. Which product family has the highest value of information after including acquisition cost, parts access, test cost and market size?

**Current conclusion:** washer remains the first large household architecture study; pump systems are the recommended first industrial demonstrator. Heat-pump water heating and industrial process heat are high-value second-wave targets. We should not begin with a full custom factory or a complicated connected appliance before two product families demonstrate repeatable, useful, independently measured improvements.
