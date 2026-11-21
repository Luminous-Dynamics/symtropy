# Appliance Microfactory: Low-Capital Manufacturing Cell v0.1

**Status:** research/design brief; not a factory plan, supplier commitment, capex approval or product qualification.  
**Primary work package:** [APPLIANCE-MICROFACTORY-001 #6603](https://github.com/Luminous-Dynamics/symtropy/issues/6603)  
**Parent product contract:** [APPLIANCE-QUAL-001 #1805](https://github.com/Luminous-Dynamics/symtropy/issues/1805)

## Executive decision

Build a **process-and-test cell before building a factory**.

The first physical product should remain the #1805 guarded 24 V DC fan. It is a bounded learning vehicle for product design, local fabrication, airflow, power, acoustics, vibration, assembly, metrology, repair and small-batch repeatability. It is not a claim that the product is safe, compliant or superior.

The first cell should own the intellectual and quality core:

- product architecture, editable CAD/ECAD and revision control;
- assembly fixtures, inspection gauges and test equipment;
- firmware, local diagnostics and software/configuration release;
- incoming inspection, controlled assembly, in-process checks and end-of-line (EOL) testing;
- repair/service procedures and as-built evidence.

Initially partner for specialist manufacturing processes and purchase traceable commodity components. Bring processes in-house only when demand and evidence justify the extra capital, skills, maintenance, space, energy and safety burden.

## 1. The manufacturing paradigm

Traditional make-or-buy comparisons often focus on piece price. This programme compares **cost per conforming, serviceable unit delivered on time**, with quality, repairability and supply risk visible.

The full route is:

`requirements → design revision → BOM → process route → material/part lots → operations → inspections → EOL test → as-built unit → repair/field feedback`

A digital thread is not a dashboard. It must preserve exact links and reject missing, stale, conflicting or unqualified evidence. Existing Mycelix product identity, manufacturing, custody and repair owners remain authoritative; this project must not create a second MES, inventory, work-order or provenance ledger.

The manufacturing cell is itself a product: modular stations, defined utility boundaries, standard fixtures, published work instructions, calibration requirements, safe work zones and replaceable tooling.

## 2. First-product boundary

Start from the #1805 fan architecture:

- guarded rotating assembly;
- extra-low-voltage DC supply, initially from an appropriate external power supply;
- no custom mains power electronics in the first demonstrator;
- local controls and essential operation without cloud or account dependence;
- replaceable modules only where they reduce measured restoration time, cost or material loss;
- physical guard and protective functions independent of Symthaea or network services.

A low-voltage product still requires a product-specific hazard review. The external power supply does not confer conformity on the complete appliance. Powered testing requires a restrained article, intact guards, a physical stop and a defined exclusion zone. Do not test exposed impellers or defeat interlocks.

### Standards starting points

| Purpose | Reference | Scope/use |
|---|---|---|
| Comfort-fan performance | [IEC 60879:2019+AMD1:2025](https://webstore.iec.ch/en/publication/106286) | Current consolidated performance-measurement reference for household/similar comfort fans, including acoustic noise and standby-power methods. Confirm product category and exact test setup before claiming a standards-method result. |
| Fan safety | [IEC 60335-2-80:2024](https://webstore.iec.ch/en/publication/70375) | Particular safety requirements for household and similar fans; used with applicable Part 1. |
| General appliance safety | [IEC 60335-1:2020+AMD1:2025](https://webstore.iec.ch/en/publication/85018) | General safety reference, including DC-supplied appliances in its scope. |
| South African market route | [NRCS VC 8055 notice](https://www.gov.za/documents/notices/national-regulator-compulsory-specifications-act-compulsory-specifications) | Exact classification, current VC, LOA applicability and product-family scope require competent review. |

Do not infer South African SANS adoption of the latest IEC editions, NRCS exemption, or accredited laboratory coverage from an international standard listing. Record exact edition, national adoption, applicable VC, test-report scope, accreditation schedule and reviewer before any market-placement claim.

## 3. Make / buy / partner

| Work / component | First-stage decision | Why / admission condition |
|---|---|---|
| Product design, CAD/ECAD, interface control | Own | Core IP and revision authority. Publish editable source and licences. |
| Assembly fixtures, test adapters, gauges | Own | Enables repeatability and evidence quality. Fixture drawings and calibration requirements are controlled artifacts. |
| Firmware, configuration, local diagnostics | Own | Differentiation and offline service; no safety authority assigned to learned software. |
| Assembly, inspection, EOL test, repair | Own | Keeps the critical learning and product-quality loop in-house. |
| Commodity motors, bearings, connectors, cable, fasteners | Buy initially | Use identified parts with traceable specifications and qualified second sources where practical. |
| External power supply | Buy from qualified supplier | Do not design custom mains electronics for the first build; still assess the complete product. |
| Sheet cutting/bending, laser cutting, CNC | Local job shop | Avoid premature machine capital; freeze drawings, material, tolerances and inspection criteria. |
| Industrial additive and injection moulding | Specialist partner | Use for suitable prototypes or production lots after material/process admission. |
| Balancing, specialist acoustics, advanced metrology | Specialist partner/lab | Use when internal screening equipment cannot support the claim. |
| Product safety/performance tests for conformity | Competent/accredited lab | Verify exact scope, edition and accreditation schedule; internal tests are not certification. |

For every sourcing decision, record dated evidence for landed cost, tooling/NRE, minimum order, yield, lead-time distribution, inspection burden, rework, supplier risk, FX/import exposure, IP/licence, process capability and repair consequence. A local assembly step is not a local-content calculation.

## 4. Staged cell architecture

### Stage 0 — design and manual assembly

- Safe organized bench and ESD controls where appropriate.
- Hand tools, controlled torque drivers, crimp tooling and connector gauges.
- Calipers, micrometers, balance, tachometer, temperature measurement and suitable electrical measurement.
- Controlled work instructions, revision labels, fixtures and a calibration register.
- Contract out sheet metal, CNC and specialist polymer parts.

### Stage 1 — repeatable prototype cell

- Enclosed FDM printer for fit-checks, fixtures and selected non-safety-critical prototype parts.
- Suitable low-voltage DC supply and electrical/power measurement.
- Guarded airflow/performance screening fixture with calibrated flow/pressure measurement.
- Vibration and acoustic screening instrumentation.
- Simple go/no-go gauges, assembly poka-yoke and a serialised build traveller.
- Process-specific extraction, ventilation, housekeeping and safe material storage.

A home-built screening rig is useful for development and repeatability. It is not automatically an IEC 60879-compliant test facility or an accredited laboratory.

### Stage 2 — small-batch assembly

- Fixture-based assembly and controlled crimp/torque checks.
- Programmable EOL sequence with polarity, current, speed, vibration/noise screen, airflow screen and thermal checks as applicable.
- Serial/QR unit identity and an auditable traveler.
- Add semi-automation only after measured cycle-time, error and ergonomic data identify a real bottleneck.

### Stage 3 — selective vertical integration

Consider CNC, sheet-metal, moulding, balancing or specialist additive equipment only after an evidenced comparison across utilization, capex, maintenance, skills, process capability, floor space, utilities, safety and total cost per conforming unit. Do not buy metal powder-bed fusion, laser systems, injection presses or industrial robot cells for the first fan programme.

## 5. Quality and manufacturing data

For each unit, bind:

- exact product/design revision and BOM digest;
- actual component/material identities and lots;
- substitution approval and compatibility evidence;
- process-route revision and operation completion evidence;
- station, tool/fixture and operator identities where relevant;
- calibration status and inspection results;
- firmware/configuration identity;
- EOL protocol, raw-data digest, limits and result;
- nonconformance, rework and retest history;
- final disposition and as-built identity.

A completed work order is not proof of an operation; a test command is not a measurement; a proposed BOM part is not an installed part; a hash is not proof of truth or authority. Missing lot identity, expired calibration, unresolved nonconformance, unapproved substitution or incomplete mandatory test blocks the relevant release state.

## 6. Benchmark system

Publish protocol and analysis plan before measuring candidate units. Freeze product revision, BOM, source parts, test configuration, operating point, environment, instruments, calibration and analysis. Retain failures, missing data, scrap, rework and censored follow-up.

### Product-level scorecard

- airflow at declared power and acoustic boundaries;
- input power and energy per delivered airflow/service unit;
- acoustic outcome under declared method and environment;
- vibration, temperature rise, stability and guard integrity;
- unit-to-unit repeatability and measurement uncertainty;
- median/P90 verified restoration time, repair task success, parts cost/lead time and recurrence.

### Process-level scorecard

- first-pass yield and final yield, reported separately;
- rolled throughput yield across process steps;
- scrap mass/material yield and rework;
- labour time and elapsed time per conforming unit;
- bottleneck cycle time, takt adherence and changeover;
- nonconformance escape rate and defect Pareto;
- energy/water per conforming unit;
- cost per conforming unit including tooling, inspection, scrap, rework, calibration, maintenance, partner operations and warranty reserve;
- supplier on-time/in-full performance and lead-time variability;
- traceability completeness and invalid/stale evidence rejection;
- design-change and repair turnaround time.

Do not use OEE as a vanity metric for an irregular prototype cell. Once there is a stable schedule, report availability, performance and quality separately, with small-batch changeover and demand losses visible. No throughput gain may come from bypassing inspection, maintenance or safe work.

### Initial pilot hypotheses — not achieved results

- At least three independently traceable units of one frozen revision; seek at least two builders other than the design author when feasible.
- 100% of units bind released revision, BOM, actual parts and EOL evidence.
- 100% of mandatory EOL checks are completed; reject all frozen wrong-revision, wrong-part, stale-calibration, reversed-polarity, missing-guard and incomplete-test negative controls.
- After process stabilization, target at least 90% first-pass yield for the defined pilot route; publish every defect/rework and retain all attempted units in the denominator.
- Target Cpk ≥1.33 only for selected critical-to-function characteristics after process stability and adequate sample size are demonstrated. Otherwise report raw distributions and uncertainty; do not call a small sample a capability study.
- Preserve #1805's proposed goal of at least 25% lower median and P90 verified restoration time against a frozen commercial control.
- Test the #1805 hypothesis of at least 10% lower energy per delivered airflow unit only at matched airflow and declared acoustic constraints.
- Cost and lead-time scenarios at batch sizes 1, 3, 10, 50 and 100 must label measured costs, dated quotations and synthetic assumptions separately.

A candidate is not “better than industry” because of lower BOM cost or a fast assembly demo. It must meet the same functional boundary, safety requirements and method, then improve one or more Pareto dimensions without unacceptable regression elsewhere.

## 7. South African capability to leverage

Do not duplicate expensive facilities that already exist:

- [CSIR Advanced Design and Manufacturing Innovation Centre](https://www.csir.co.za/advanced-design-and-manufacturing-innovation-centre): public description includes SMME support, polymer/powder additive manufacturing, 3D scanning and lifecycle-aware product development.
- [CSIR Future Production: Manufacturing](https://www.csir.co.za/what-we-do/manufacturing): public capability areas include materials/manufacturing, additive processes, powder metallurgy, casting, smart manufacturing and product localisation.
- [Collaborative Programme in Additive Manufacturing](https://www.csir.co.za/sites/default/files/2026-01/TOR%20-%20CFP%20No%20003-08-27-2024%20CPAM%20%20-%20Aug%202024%20__revised.pdf): programme material describes SMME and industrial development support across design, process development, monitoring/control, post-processing, testing and analysis.
- [dtic Manufacturing Support Programme](https://www.thedtic.gov.za/financial-and-non-financial-support/incentives/manufacturing-support-programme-msp/): the public page describes support for eligible registered manufacturers, localisation, production capacity and competitiveness. Confirm current guideline, eligible SIC/activity, application status, ownership criteria, reimbursable cost rules and cash-flow implications before relying on it.

These are discovery leads, not confirmed access, supplier quotes, grant eligibility, funding or partnerships. Record the date, contact, facility process scope, cost, lead time, data/IP terms and acceptance criteria for every proposed partner.

## 8. Gate sequence

| Gate | Evidence required | Decision |
|---|---|---|
| M0 — process definition | Product/hazard scope, requirements, baseline method, make/buy/partner map, controlled process flow | Is this the right first product and process? |
| M1 — partner map | Named facilities, scope, costs/quotes, lead time, IP/data terms, qualification evidence | What should be outsourced? |
| M2 — fixture/metrology | Open fixture CAD, instruments, calibration/uncertainty plan, repeatability study | Can we measure the claimed functions? |
| M3 — first article | Exact revision, lots, deviations, hazard review, screening results | Is the design manufacturable and testable? |
| M4 — pilot batch | ≥3 traceable units, yield/rework, EOL data, cost per conforming unit | Is the route repeatable? |
| M5 — independent replication | Independent builder reproduces unit and selected measurements; repair trial | Does the design transfer beyond its author? |
| M6 — scale decision | Manual cell vs partner production vs selective capex at 1/3/10/50/100 units | Is bringing a process in-house justified? |

## 9. Competitive strategy

Do not compete with mass manufacturers on installed factory scale. Compete on dimensions that their volume model may underserve:

- repair-first product architecture and long-lived service modules;
- rapid adaptation to local climate, grid instability, water availability and user requirements;
- open and supplier-neutral technical packages;
- exact evidence from design through as-built unit and verified repair;
- low-volume, high-mix production without loss of revision discipline;
- local parts and qualified second sources;
- measured useful service and lifecycle cost, not feature count or nameplate claims.

To claim a genuine advantage, compare against at least one properly selected and tuned commercial baseline plus the incumbent unit, using identical methods and output floors. Publish the complete metric vector, uncertainty, Pareto frontier and failures. No single weighted score can hide a safety, quality or evidence failure.

## 10. Claim ceiling

This brief does not establish a workshop, machine inventory, supplier agreement, prototype, yield, unit cost, local-content share, accredited laboratory scope, product safety, conformity, funding eligibility or production capacity. Numerical thresholds are pilot hypotheses. Every advancement requires evidence appropriate to that claim.

## References

- IEC 60879:2019+AMD1:2025 — https://webstore.iec.ch/en/publication/106286
- IEC 60335-2-80:2024 — https://webstore.iec.ch/en/publication/70375
- IEC 60335-1:2020+AMD1:2025 — https://webstore.iec.ch/en/publication/85018
- CSIR ADMIC — https://www.csir.co.za/advanced-design-and-manufacturing-innovation-centre
- CSIR manufacturing — https://www.csir.co.za/what-we-do/manufacturing
- CSIR CPAM SMME programme terms — https://www.csir.co.za/sites/default/files/2026-01/TOR%20-%20CFP%20No%20003-08-27-2024%20CPAM%20%20-%20Aug%202024%20__revised.pdf
- dtic Manufacturing Support Programme — https://www.thedtic.gov.za/financial-and-non-financial-support/incentives/manufacturing-support-programme-msp/
- NRCS VC 8055 notice — https://www.gov.za/documents/notices/national-regulator-compulsory-specifications-act-compulsory-specifications
