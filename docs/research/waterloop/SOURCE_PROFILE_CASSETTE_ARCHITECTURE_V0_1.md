# WaterLoop Source-Profile Treatment Cassettes — Architecture v0.1

**Date:** 16 November 2026  
**Parent design:** Symtropy #5335 / PR #5336  
**Work package:** Symtropy #5671 — WATERLOOP-SOURCE-PROFILE-MODULES-001  
**Status:** Research architecture / test-cell design input; not a released construction package  
**Evidence class:** Literature-backed design hypotheses + unqualified candidate processes  
**Claim ceiling:** Source segregation, interface definition and test protocol only

## 1. The architecture decision

WaterLoop should not be a single tank with a universal greywater filter bolted onto it. The recommended product direction is a **source-separated, profile-gated, replaceable treatment platform**.

The system should first determine what water is entering, which contaminants and loads are present, what the intended use is, and what the treatment module has actually demonstrated. Only then may it credit output as fit for that use.

The functional chain is:

> source identity → feed characterisation → source acceptance → equalisation / pretreatment → exchangeable treatment cassette → independently assessed final barrier → use-specific quality admission → metered useful service

The physical demonstrator remains the storage-first, three-circuit architecture already defined in #5335 / PR #5336. This document adds a treatment architecture to the existing **separate non-potable recovery circuit**; it does not redraw the backup-water circuit or create a new system P&ID.

### Design principles

1. Keep bathroom, laundry, kitchen and blackwater streams separate until their own evidence justifies a controlled combination.
2. Design around feed variability and load shocks, not a single synthetic recipe or one mean concentration.
3. Use replaceable process modules with stable mechanical, hydraulic, electrical, sampling and residual interfaces.
4. Make every module's operating envelope explicit: flow, pressure, temperature, feed quality, duty cycle, maintenance and fault conditions.
5. Measure useful, quality-admitted water at the end-use boundary. Treatment throughput is not useful service.
6. Treat reject, backwash, spent media, sludge, purge and cleaning chemicals as real outputs with a defined destination.
7. Make loss of required evidence or process envelope produce a fail-closed product outlet, not a lower-confidence delivery.
8. Do not automate chemical dosing, public-network switching or potable connections in v0.1.

## 2. Why source-aware design is the right paradigm shift

A systematic review published 23 September 2026 synthesised 131 empirical studies of decentralised greywater treatment (2016–2026). It reports COD ranging from 100–2,000 mg/L, TSS from 30–1,000 mg/L, anionic surfactants often above 100 mg/L during laundry discharge and frequent microbial indicators above recommended reuse thresholds. It identifies temporal variability, household behaviour, source mixing and storage-induced transformations as major drivers of fouling, treatment efficiency, energy demand, maintenance and health risk. The authors' methodological appraisal rated 41% of included studies high robustness, 38% moderate and 21% limited.

These ranges are **literature context**, not WaterLoop feed specifications or local household acceptance limits. The first job is to measure local source populations and their tails.

Ninsiima et al. (2026), *A systematic review of greywater properties and variability that govern decentralised off-grid treatment feasibility*, Discover Applied Sciences. DOI: https://doi.org/10.1007/s42452-026-09559-0

A second study, first published 29 October 2026 in JAWRA, compared two bench-scale residential light-greywater trains on a synthetic contaminant mixture: (A) sedimentation → slow sand → ultrafiltration → UV, and (B) sedimentation → ozonation → granular activated carbon (GAC) → UV. It reported higher turbidity and COD reduction for Train B on that test feed. These numbers do not establish microbiological safety, full-scale robustness, long-term media life or superiority on real household feeds. The two trains are useful **matched-test comparators**, not a selected WaterLoop process.

García-Agudelo et al. (2026), DOI: https://doi.org/10.1111/1752-1688.70162

## 3. Source classes and first build boundary

Keep source identity at the inlet; do not use a catch-all label called greywater.

| Source profile | V0 status | Required interpretation |
|---|---|---|
| Bathroom light greywater: shower, bath and hand basin within a frozen boundary | Primary characterisation candidate | Still variable and potentially microbiologically contaminated; measure each batch and source boundary |
| Laundry | Separate challenge stream | Lint, surfactants, detergent residues, sodium and boron can alter process and irrigation suitability; do not mix into bathroom baseline |
| Kitchen sink / dishwasher | Reject from V0 | Grease, food solids and higher organic loading require a different pretreatment and risk basis |
| Toilet / urinal / blackwater | Reject from V0 | Sanitation feedstock, outside the greywater circuit; use #4902 |
| Nappy/urine/faeces-soiled wash; paint/vehicle/machinery wash; hair dye; chemical cleaning; unknown mixed feeds | Reject / contained quarantine | No feed acceptance without a separately reviewed profile and process envelope |

These are design-screening categories, not claims that a given source is safe. Source diversion and rejection must be physical and inspectable, not only a software label.

### V0 use boundary

The first treatment trial is performed in an **unoccupied, isolated test cell**. Its product water goes only to a contained, instrumented test load. No drinking, cooking, personal hygiene, bathing, aerosolised irrigation, food-crop irrigation, occupied toilet flushing, environmental discharge or public-network connection is included.

A later use profile—such as restricted subsurface irrigation or toilet flushing—must have its own risk assessment, quality criteria, monitoring and approval path. A laboratory pass for one use cannot be promoted to another.

## 4. Reference architecture

    SOURCE COLLECTION — kept separate
      ├── Bathroom batch source ── sample + inlet meter ── source acceptance
      ├── Laundry batch source  ── sample + inlet meter ── separate challenge route
      └── Rejected/unknown feed ───────────────────────────> contained quarantine

    ACCEPTED BATHROOM FEED
      │
      ├── isolation + coarse hair/solids screen
      ├── measured equalisation / settling module
      │      └── drain / settled solids collection
      │
      ├── MODULE BAY A: membrane candidate
      │      slow-sand / validated prefilter → UF → terminal UV
      │      └── backwash / concentrate / cleaning residuals
      │
      ├── MODULE BAY B: oxidation / adsorption candidate
      │      contained ozone contact → off-gas destruction
      │      → GAC cassette → terminal UV
      │      └── sludge / spent carbon / purge / cleaning residuals
      │
      ├── independent sample + product flow/volume boundary
      ├── quality and process evidence admission
      │
      ├── PASS: isolated, metered test-use outlet
      └── FAIL / UNKNOWN / POWER LOSS: outlet isolated,
                                      flow to contained quarantine

    ALL FLOWS:
      source + product + bypass + overflow + backwash + purge
      + drain + residual + final inventory are measured or bounded.

The drawing is functional, not a plumbing P&ID. Fitment, materials, pipe sizes, working pressures, vessel volumes, UV dose, membrane rating, ozone dose/contact time, gas controls and electrical protection must be selected only after the test duty and qualified review.

## 5. Replaceable cassette interfaces

Each cassette must expose the same *classes* of interface; exact geometry and pressure/flow envelope remain open until the component duty is known.

| Interface | Required contract |
|---|---|
| Mechanical | Unique keyed mounting; orientation; support/load; wetted-material and seal compatibility; removal clearance; lifting/handling |
| Hydraulic | Inlet/outlet; isolation; declared min/max flow and pressure; pressure drop; drain-down; no hidden bypass |
| Sampling | Feed, interstage and product sample ports; stable sample identity and safe access |
| Instrumentation | Flow, inlet/outlet pressure, differential pressure and stage-specific measurands; channel IDs bind physical tap locations |
| Electrical | Voltage, maximum power, standby/auxiliary loads, protective isolation and emergency stop where relevant |
| Residuals | Dedicated backwash, reject, purge, sludge, spent-media and cleaning-waste outlet; no uncontrolled cross-drain |
| Service | Media/membrane lot and serial identity; operating-hours counter; inspection, clean and replacement procedure |
| Evidence | Cassette type/revision, process envelope, commissioning record, maintenance record and return-to-service verification |

Do not standardise a quick-connect or cartridge dimension before compatibility of pressure rating, chemical exposure, backflow protection, sealing and maintainability is reviewed. “Modular” must not become “any cartridge fits any housing.”

## 6. Candidate trains

### Train A — membrane-based comparison

> screen → settling/equalisation → slow-sand or validated prefilter → UF → terminal UV

Candidate strengths: compact downstream barrier, strong solids reduction potential and instrumentable membrane performance.

Required measurements and failure controls:
- flow and differential pressure across each stage;
- membrane transmembrane pressure (TMP), flux and permeability trend;
- integrity verification on a declared method and schedule;
- clean/backwash frequency, recovery and reject volume;
- UV transmittance (UVT), reactor flow and validated delivered dose—not merely a lamp-on signal;
- UV intensity/lamps/ballast fault state;
- downstream sample at the actual test-use boundary.

A membrane integrity indication or turbidity reading alone does not establish pathogen safety.

### Train B — oxidation / adsorption comparison

> screen → settling/equalisation → contained ozone contact → off-gas destruction → GAC → terminal UV

Candidate strengths: adsorption/oxidation may address some dissolved organic loads not removed by a solids barrier alone.

Required measurements and failure controls:
- ozone generation and applied dose under a validated process definition;
- contactor flow/contact conditions;
- ambient/off-gas monitoring and destruction-system state;
- GAC cassette identity, operating hours, target-analyte breakthrough method and replacement;
- spent-carbon handling, purge/sludge and any oxidation by-product assessment;
- UVT, flow and validated final barrier performance.

Ozone is a laboratory/test-skid candidate only. It is not a consumer-ready process in this design. Gas exposure controls and qualified engineering review are prerequisites.


### Train D — electrocoagulation-assisted laundry challenge (optional, not in first build)

A 2026 review of electrocoagulation-based greywater/laundry treatment describes EC as a potentially useful pretreatment or core destabilisation step for colloids, fibres, turbidity and some surfactant-associated pollutants. It also concludes that EC alone is not a stable reuse solution where dissolved organics, pathogens, salts or micropollutants remain; hybrid downstream treatment is needed. Important scale-up burdens include electrical energy, electrode consumption/passivation, sludge and residual-metal control.

A January 2026 continuous-flow **industrial laundry** study reported COD removal above 80%, turbidity removal above 90% and specific energy below 1.5 kWh/m³ for optimised reactor geometries. This is an industrial wastewater result, not residential laundry-greywater performance and not a directly comparable WaterLoop result. It is a reason to preserve an optional challenge test, not to add an EC module to the first build.

Candidate challenge sequence:
- separate laundry-only feed batch;
- screen / equalisation;
- continuous-flow EC test skid with recorded current, voltage, charge loading, electrode material/lot, polarity schedule and residence-time distribution;
- solid/floc separation;
- downstream treatment selection only after the residual dissolved organics, salts and microbiological barriers are measured;
- separate electrode sludge, spent-electrode, purge and cleaning-waste collection.

The EC module remains optional and unselected. It should proceed only if the laundry source-characterisation data show a contaminant profile that justifies its energy, electrode, sludge and maintenance burden.

Sources:
- *Electrocoagulation-based hybrid systems for greywater and laundry wastewater reuse: A critical review* (October 2026), DOI: https://doi.org/10.1016/j.dwt.2026.101869
- *Hydrodynamic and electrode optimization of a continuous-flow electrocoagulation reactor for laundry wastewater* (January 2026), DOI: https://doi.org/10.1016/j.dwt.2026.101687

### Train C — passive / nature-based comparator

A planted/media treatment module may be compared where land, climate, hydraulic loading, mosquito/vector management, maintenance access and residence time are suitable. Land occupation, seasonal variability, pre/post-treatment storage, microbial safety barrier and operator duties must be counted. Passive does not mean maintenance-free or safe by default.

### Storage-only comparator

Storage/no-treatment may be included only as an isolated non-potable test comparator with a declared safe test load and professional review. It is not a product-use route and does not establish suitability for an actual reuse application.

## 7. Critical process and fail-safe contract

The process envelope is multi-dimensional; no single threshold sensor is a universal quality sensor.

Potential process-critical variables include:
- feed source identity and batch;
- flow, temperature, pH, conductivity and turbidity;
- COD/TSS and source-specific surfactant/sodium/boron measures;
- membrane TMP, flux and integrity evidence;
- UVT, UV intensity, validated UV dose/flow relation and lamp fault;
- ozone contact conditions and off-gas destruction state, if Train B is tested;
- cassette identity, media age, operating hours and maintenance state;
- product sample result, analytical method, uncertainty and validity for the named end use.

### Functional safe state

On rejected/unknown feed, operation outside the admitted process envelope, missing/expired sample, failed process verification, unknown cassette identity, critical stale/contradictory telemetry, or power loss where power is required:
1. isolate the product-use outlet;
2. divert or retain process water in a contained quarantine/residual path;
3. preserve the source/batch/process/fault identity;
4. require inspection and authorised manual reset;
5. repeat the relevant process verification and sample admission before return to service.

The first physical system uses local manual controls and read-only telemetry. There is no remote actuation, autonomous dosing or live building/municipal connection. The fail-safe circuit must be demonstrated physically; a software flag alone is insufficient.

## 8. Measurement and sampling strategy

### Feed characterization

Start with independently collected bathroom-only batches. Preserve exact source, collection interval, household/fixture boundary, storage interval, temperature and any source events. Do not pool batches before source variability is quantified.

Initial analytical panel for risk-review and test design:
- flow and volume over time;
- temperature, pH, conductivity and turbidity;
- COD and TSS;
- surfactant indicator/method appropriate to the detergents in use;
- sodium and boron for any laundry challenge;
- microbiological indicators and any additional determinants identified by the qualified risk assessor.

Analytes, methods, preservation, sample frequency and detection limits must be fixed with the analytical laboratory and Mycelix #6100 before a qualification trial. The panel above is a design starting point, not a complete safety panel.

### Paired measurement and mass balance

For each analyte, report:
- paired influent/effluent concentrations;
- influent/effluent mass loads where flow and volume data permit;
- mass in sludge, backwash, spent media, purge and other residuals where measurable;
- change in stored inventory;
- unresolved mass-balance residual and uncertainty.

A decrease in concentration is not necessarily a removal. Dilution, adsorption, transformation, accumulation and unmeasured residual paths must remain distinct.

### Core system metrics

Water recovery = verified product volume / accepted feed volume.

Energy intensity = total electrical energy / verified useful volume.

Also report analyte-specific mass removal, microbiological indicator performance, residuals per useful cubic metre, downtime, maintenance time, consumables, cleaning water, failure/abstention events and lifecycle cost. Count pump, UV, ozone, controls, standby, backwash and auxiliary energy inside the declared system boundary.

## 9. Matched test protocol

1. Freeze source profile, test batch identity, intended test load and train configuration.
2. Verify meter calibration/admission, sampling protocol, timebase, vessel geometry and all flow paths.
3. Run a baseline feed batch through the isolated batch loop with no product-use credit.
4. Randomize or predeclare Train A / Train B run order and match feed batches, flow, loading, temperature and test duration.
5. Repeat normal, dilute, peak-load, high-solids, interrupted-flow, restart and extended-idle conditions.
6. Inject controlled screen blockage, membrane fouling/TMP, UV fault/low UVT, unknown cassette, stale sample and power-loss scenarios under a contained test protocol.
7. Record bypass/quarantine routing and verify no product outlet delivery in every failed/unknown state.
8. Repeat across multiple independent source batches and more than one component/media lot.
9. Use independent laboratory analyses for quality endpoints; process sensors remain process monitoring evidence.
10. Complete maintenance/change-out trials with multiple operators and repeat return-to-service verification.

No occupied use, aerosol generation, potable use or environmental discharge is part of this stage.

## 10. Industry and standards comparison

| Comparator | What it contributes | What WaterLoop must match before comparison |
|---|---|---|
| NSF/ANSI 350 certified or tested systems | Independent intended-use physical, chemical and microbiological performance evaluation | Exact standard edition/class, system scope, test report, feed, duty, end use and listed configuration |
| Residential UF + UV systems | Compact solids/microbial barrier strategy | Actual feed, UVT/dose/flow, membrane integrity, fouling and operating cost |
| Ozone + GAC + UV | Oxidation/adsorption treatment route | Ozone safety, by-products, media breakthrough, residuals, power and matched feed |
| Nature-based treatment | Potentially low-energy process route | Land, residence time, climate, seasonal performance, microbial safeguards and operator work |
| Storage-only systems | Lowest-complexity non-potable service comparator | Actual usable volume, storage age, quality/use boundary, losses and delivery service |

NSF describes Standard 350 as covering minimum material, design, construction and performance requirements for onsite residential/commercial systems treating greywater or combined wastewater, with intended-use physical, chemical and microbiological criteria. Use this as an external benchmark; it is not assumed to be the governing South African approval route.

Source: https://www.nsf.org/knowledge-library/nsf-ansi-standard-350-certification-water-reuse-treatment-systems

Local boundaries must be checked against the current consolidated City of Johannesburg by-laws, the 2025 national standards, WRC TT 746/17, the applicable current SANS editions and professional review. The City of Johannesburg's 2023 by-laws specifically address greywater storage/treatment and route stored greywater overflow to sewer; do not design an overflow to stormwater by default.

## 11. Manufacturing and service package

The design package must eventually contain:
- revision-controlled assembly and module drawings;
- treatment process flow diagram and instrumented P&ID;
- module dimensional and mechanical interface drawings;
- preliminary bill of materials with wetted material/seal compatibility;
- pressure, flow, electrical and ambient design envelopes;
- sample port and metrology map;
- residual/waste handling specification;
- cleaning, inspection, media change-out and return-to-service procedures;
- spare/consumable sourcing and service access assessment;
- configuration-specific test evidence and commissioning receipt.

No supplier SKU, pressure vessel, membrane, UV reactor, ozone generator, pump or fitting is frozen in v0.1. The first manufactured article should be a **contained test cassette and test skid**, not a complete household product.

## 12. Proposed targets and gates

### Hard gates

- zero cross-connections in physical inspection and pressure/trace testing;
- zero product-use outlet delivery during seeded failed, unknown or expired evidence states;
- 100% rejected/unknown feed is isolated before product delivery;
- every inlet, outlet, bypass, backwash, purge, overflow, drain and residual path is measured or explicitly bounded by uncertainty;
- no quality-unqualified volume is credited as useful service;
- no physical trial begins until plumbing/electrical/building review and laboratory/safety plans are approved.

### Optimization targets — set numerically after baselines

- maximize quality-admitted useful-water recovery;
- minimize energy per verified useful cubic metre;
- minimize residual volume and hazardous residual burden;
- minimize fouling, change-out frequency, unplanned downtime and operator time;
- minimize lifecycle cost per verified useful cubic metre without worsening quality, continuity or health risk;
- reduce module change-out and return-to-service time across repeat operator trials.

A provisional 20% lifecycle-cost advantage over a matched reference can be used as a research aspiration after a real baseline exists; it is not a current performance claim or a design requirement to be met by relaxing safety.

## 13. Design release stages

| Stage | Output | Exit gate |
|---|---|---|
| A — source characterization | Independent batch dataset and feed variability | Mycelix sample/method admission and source boundary freeze |
| B — cassette definition | Module interface drawing, operating envelope and failure modes | Reviewed P&ID, materials, electrical and residual paths |
| C — matched bench test | Train A/B comparative results | Independent analytical results, mass balance and repeatability |
| D — reliability/service | Fault, restart, maintenance and cassette-change-out results | All fail-safe negative controls pass |
| E — manufacturing review | Revision-controlled first-article build package | Qualified engineering and site review; procurement authorization remains separate |

## 14. Nonclaims

This architecture does not select an approved treatment train, certify pathogen removal, establish a potable-water result, permit occupied reuse, approve municipal plumbing, authorise discharge, or authorise procurement/construction. All candidate trains remain unqualified until the stated evidence gates are completed.

## 15. Longitudinal sustainment benchmark — regional field lesson

A 27 October 2026 study of an integrated decentralized biofilter–constructed wetland at Eswatini Christian University provides a relevant regional comparator. The 2015 field campaign reported COD, total nitrogen and total phosphorus reductions of 87.1%, 73.3% and 79.0%. The 2026 follow-up documented continued use of one containerized biofilter plus the wetland after other units had stopped, but the paper explicitly states that **no analytical water-quality measurements were made after December 2015**. The UV lamps were no longer operating in the 2026 configuration. The authors describe routine inspection of about 10–20 minutes by a locally trained groundskeeper, while the history also included electricity/wiring interruptions and loss of local laboratory capability.

This is a useful sustainability reference, not proof of 2026 treatment quality, and it concerns domestic wastewater at institutional scale rather than isolated household greywater.

Rhee et al. (2026), *Treatment Performance and Long-Term Operation of a Decentralized Biofilter-Constructed Wetland System in Eswatini: Conditions for Sustaining Treatment Service*, Water 18(21), 2679. DOI: https://doi.org/10.3390/w18212679

### Required WaterLoop distinction

A treatment asset can remain physically in service while its claimed quality performance is unverified. Report these as different timelines:
- **operational status:** equipment operating, bypassed, isolated, under repair or decommissioned;
- **quality verification status:** current admitted laboratory evidence for the exact process configuration and intended use;
- **service status:** metered useful volume delivered at the point of use;
- **maintenance support status:** trained local operator, specialist support, spares, consumables and laboratory availability.

No quality claim may bridge a gap in independent quality verification. If a required barrier is offline, the corresponding product-use outlet must remain isolated unless a separately validated alternate treatment configuration and intended use are admitted. “The plant is still running” cannot be reported as “the water is still safe.”

### Additional benchmark metrics

Add longitudinal measures to the comparison:
- fraction of scheduled periods with admitted quality verification;
- quality-verification gap duration and stale-evidence abstention count;
- fraction of service periods in which each required barrier is actually operational;
- time-to-detect and time-to-isolate a barrier failure;
- planned/unplanned downtime and service continuity;
- routine operator minutes per day/week and specialist intervention hours;
- spares/consumables lead time and replacement-task success;
- laboratory access interruptions and time to restore monitoring;
- quality-verified useful volume, not merely total flow through an operating unit.

The design objective is **maintainable, quality-verifiable service over time**, not maximum initial removal efficiency or uninterrupted equipment runtime at any cost.
