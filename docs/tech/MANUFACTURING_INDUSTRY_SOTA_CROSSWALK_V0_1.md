# Distributed Manufacturing Industry SOTA Crosswalk and LD-Print Benchmark Plan v0.1

**Checked:** 22 November 2026.
**Scope:** desktop / neighbourhood polymer material-extrusion manufacturing, productive-machine engineering and the cross-repository digital thread.
**Status:** literature/standards crosswalk and proposed benchmark contract. No competitive trial or physical qualification has been performed.

## 1. Executive finding

The differentiating opportunity is not to invent another CoreXY printer, another slicer, or another product passport. Commercial printers already offer excellent integration, speed and support; open-hardware communities already offer configurable and repairable machine designs; industrial standards already cover important parts of additive manufacturing qualification, data packaging, emissions and supply-chain traceability.

The Luminous opportunity is to connect these capabilities into a verifiable regional production service:

`resident/product demand → exact part and rights → qualified process → machine/resource offer → time-compatible supply route → authorized execution → independent measurement → accepted article → repair/lifecycle → spatial service projection`

The competitive unit should be **accepted, useful output and restored service**, not the printer, raw motion speed, local-content slogan or number of devices distributed.

The public owner map is in [MANUFACTURING_CROSS_REPOSITORY_OWNER_CONTRACT_CROSSWALK_V0_1.md](MANUFACTURING_CROSS_REPOSITORY_OWNER_CONTRACT_CROSSWALK_V0_1.md). Symthaea #12533 remains the canonical LD-Print v0 machine design and physical qualification owner; Mycelix #4689 owns Node 0; Sol-Atlas #606 owns owner-native evidence projection.

## 2. Industry and standards landscape

### 2.1 Commercial and open machine baselines

Use at least three deliberately different references:

| Reference | Demonstrated strength to benchmark | What Luminous must not assume |
|---|---|---|
| Bambu Lab H2D (single-nozzle configuration) | Integrated consumer/prosumer workflow, automation, enclosure, high advertised motion/flow envelope and multi-material ecosystem | Advertised maximum motion speed/flow is not accepted parts per hour or full-life capability |
| Prusa CORE One L | Enclosed CoreXY product, service ecosystem, spare/support documentation and a 300 mm-class build envelope | Brand-level “open source” does not mean every design artifact has an unrestricted commercial hardware licence |
| Voron 2.4 | Community-built reference, modular sourcing, documented BOM/build process and static-bed/Z-moving-gantry architecture | Community openness does not mean a random build has controlled calibration, safety or production qualification |
| Local bureau / job shop | A real alternative to ownership and maintenance, with quoted delivery and potentially better equipment/inspection for particular jobs | Website capability claims and a quotation are not proof of accepted sample output or stable process capability |

Exact model, edition, hardware revision, firmware, material and source date must be frozen in the comparison manifest. Vendor specifications are useful screening data; physical metrology is the comparison evidence.

Current preliminary volume screen:
- Luminous initial envelope: 300 × 300 × 300 mm = 27.0 L.
- Prusa CORE One L published: 300 × 300 × 330 mm = 29.7 L.
- Bambu H2D single-nozzle published: 325 × 320 × 325 mm = 33.8 L.

The 300 mm cube is therefore not an advantage in nominal build volume. The 520 × 520 mm Luminous concept is a bare frame envelope, not a finished external dimension, so its apparent footprint comparison is warning-level only. Do not freeze volume until the actual part basket shows a need.

### 2.2 Standards and external reference methods

| Reference | Relevant contribution | Correct Luminous use | Boundary / nonclaim |
|---|---|---|---|
| ISO/ASTM 52902:2023 | AM geometric-capability test artefacts, feature measurements and measurement-uncertainty considerations | Adapt a geometric artefact campaign to the exact MEX machine, material and intended tolerance class; bind position/orientation and calibrated measurement | It does not prescribe all measurement methods, machine settings or product-specific limits. Referencing it is not conformity |
| ISO/ASTM 52924:2023 | Polymer AM part-property classification, including thermoplastic MEX and PBF; mechanical, physical and geometrical properties | Use its property-class framing to state what a particular part requires and what evidence is available | Property classification is not a universal part safety approval or a complete process qualification |
| ISO/ASTM 52951:2026 | AM data package from design through acceptance | Borrow the digital-thread structure: design, build/process parameters, manufacturing and acceptance data; create an explicitly MEX-specific Luminous profile | Its detailed technical basis is PBF-LB/M metal. Do not claim FDM/MEX conformity by copying the package idea |
| ISO/ASTM 52966:2026 | Capability-level framework for temporary self-sufficient AM systems, considering people, training, infrastructure, materials and full part cycle | Use as a reference for evidence-bearing H0/H1/N1/R2/Q3 capability levels and support dependencies | It targets temporary self-sufficient contexts such as ships, islands and remote operations. It is not a household-printer certification or statutory safety approval |
| NIST IR 8536 (final, 9 Sep 2026) | Manufacturing supply-chain traceability meta-framework: linked, temporally ordered and cryptographically verifiable provenance; interoperability and selective disclosure | Map Mycelix/owner-native event chains and source references while preserving domain standards and confidentiality | It is a meta-framework, not a product schema or production certification. NIST describes its reference implementation as research/MVP maturity, not a production platform |
| UL 2904 Ed. 2 (2023) | Chamber method to measure coarse/fine/ultrafine particles and VOCs from 3D printers/feedstocks in indoor non-industrial scenarios | Plan emissions measurement for the exact printer/material/use envelope; compare feedstock, object and machine at identical test conditions | Chamber testing is not a universal safe-use guarantee for every room, material or exposure scenario |
| NIOSH 2024-103 | Practical hierarchy of controls for desktop 3D printing, including source control, enclosure/local exhaust and filtration approaches | Treat emissions control as an engineered and measured machine/facility subsystem, not a low-odour filament claim | Generic room air changes, enclosure presence or consumer filter branding do not establish site-specific exposure control |
| NIST AM-Bench 2025 | Carefully documented research benchmarks and challenge data for metal AM, plus a polymer vat-photopolymerization challenge | Borrow preregistration, controlled measurement, public reference data and independent prediction comparison | It is not a desktop FFF/MEX leaderboard; the 2025 polymer challenge is vat photopolymerization, not material extrusion |

Official reference links:
- ISO/ASTM 52902:2023: https://www.iso.org/standard/79683.html
- ISO/ASTM 52924:2023: https://www.iso.org/standard/76909.html
- ISO/ASTM 52951:2026: https://www.iso.org/standard/76828.html
- ISO/ASTM 52966:2026: https://www.iso.org/standard/89300.html
- NIST IR 8536: https://csrc.nist.gov/pubs/ir/8536/final
- UL 2904 Ed. 2: https://www.shopulstandards.com/ProductDetail.aspx?productId=UL2904_2_S_20230526
- NIOSH 2024-103: https://www.cdc.gov/niosh/docs/2024-103/
- NIST AM-Bench: https://www.nist.gov/ambench

### 2.3 Competitive interpretation

The industry already has strong isolated capabilities:
- Bambu is a strong comparison for integrated workflow, motion, materials ecosystem and out-of-box usability.
- Prusa is a strong comparison for long-term service, spare parts, maintenance documentation and customer support.
- Voron is a strong comparison for build-from-components, community repair, modifiability and open mechanical designs.
- Local specialist bureaus and job shops are the counterfactual for users who need parts but do not need to own and operate a machine.
- NIST/ISO/ASTM/UL methods provide relevant measurement/qualification patterns.
- Mycelix, OPC UA/MTConnect/QIF-type industrial interoperability and NIST IR 8536-style traceability address data and operations integration; these should be consumed through owner-native adapters, not replaced with one Luminous universal schema.

Luminous should compete at the intersection: **machine serviceability + regional repair route + capability evidence + supplier/lot traceability + independent acceptance + affordable access**. A stronger CAD model or a single high-speed print is not sufficient differentiation.

## 3. Benchmark architecture: three independent evidence layers

### Layer A — machine capability

Measure a particular machine configuration:
- geometric feature error by X/Y/Z, position, orientation and build height;
- local and global repeatability;
- feature resolution and failure boundaries;
- flatness/warpage and thermal stability;
- dynamic/motion behavior if it affects the selected part function;
- downtime, maintenance and repair/recommissioning time;
- offline behavior and controlled power-loss recovery;
- measured energy and emissions under the declared use envelope.

A machine profile is a versioned claim about an as-configured machine. A measured spatial map is not a universal correction field; interpolation outside tested/validated regions remains Unknown.

### Layer B — process/material capability

Bind the process to:
- exact resin/grade, supplier and spool/lot;
- moisture/storage/conditioning and drying where applicable;
- nozzle/tool, wear state, build plate and surface preparation;
- firmware, slicer, profile and G-code digest;
- temperatures, flow, layer height, cooling, orientation, supports and post-processing;
- environmental conditions and exposure controls;
- witness coupon identity, orientation, conditioning, test method and uncertainty.

Use ISO/ASTM 52924 to declare required/achieved properties. Do not infer material behavior from filament name, datasheet or a synthetic model alone.

### Layer C — product/task acceptance

The output must pass the actual function-specific requirements:
- exact product/revision and manufacturing rights;
- interface dimensions, fit and assembly;
- functional loads/environment, creep, wear, thermal/chemical compatibility as relevant;
- independent inspection and uncertainty;
- installation and post-repair verification;
- intended duty profile and prohibited uses.

A dimensional coupon pass does not qualify a different functional part. One accepted specimen does not establish production yield or a stable process.

## 4. Matched LD-Print reference campaign

Freeze one reference task basket before machine selection. Initial candidates can include low-risk manufacturing aids and a non-functional packaging spacer, but no record enters the campaign until its exact CAD/revision, rights, function and test are available.

### Comparator ladder

1. Commercial desktop FDM machine with exact model and firmware frozen.
2. Open-hardware build (e.g. Voron configuration) with exact BOM/build revision and machine identity recorded.
3. Local FDM bureau route using a matched part/material and inspection protocol.
4. Local CNC/job-shop route where it is technically appropriate.
5. LD-Print v0 after its own staged qualification.

### Frozen controls

- Exact artifact geometry/revision and CAD digest.
- Material grade, supplier and lot; conditioning and measured mass.
- Nozzle/tool and wear state.
- Machine hardware/configuration, firmware and slicer/profile versions.
- Orientation, support, layer height, walls/infill, cooling and process settings.
- Room/chamber conditions and emissions controls.
- Operator method and training.
- Measurement instrument, calibration, method, uncertainty and acceptance criteria.
- Queue, setup, calibration, finishing, inspection and rework accounting.

Randomize or block run order to avoid machine/operator/material/environment confounding. Retain all failed, incomplete, missing and rejected jobs in the denominator.

### Three artifacts

1. **Geometric artefact:** ISO/ASTM 52902-informed feature set with position/orientation coverage and calibrated metrology.
2. **Functional part:** one real, low-risk Luminous jig/accessory with exact fit/function acceptance.
3. **Material witnesses:** coupons in relevant orientations from identified lots, tested for properties needed by that part; no universal material-strength claim.

### Sample design

- Screening: at least five repeats per chosen artifact/material/profile cell to discover variance and protocol defects; screening is not a superiority trial.
- Confirmatory: determine sample size from a preregistered effect size, pilot variance and desired power; freeze the evaluation set and test plan before runs.
- Independent replication: at least three separately built/operated nodes for a feasibility/reproduction demonstration. Three nodes are a replication gate, not enough by themselves to claim a population-wide yield or capability statistic.
- Use blinded dimensional/functional inspection where feasible; publish raw data, exclusions, missingness, uncertainty and analysis scripts.

## 5. Metrics that matter

Do not combine these into a weighted factory score. Safety and required quality are non-compensatory gates; cost, time, energy, waste and serviceability remain separate Pareto objectives.

| Metric | Definition / denominator | Initial benchmark target |
|---|---|---|
| Critical-feature conformance | Exact critical features passing the preregistered part requirement; include uncertainty and all failed runs | Zero false acceptance in frozen negative controls; physical limits part-specific |
| Geometric repeatability | Within- and between-machine distribution by feature, axis, position and orientation | Quantify before attempting a 10% critical-feature-error improvement hypothesis |
| Spatial capability | Error and interval coverage across preselected test positions and orientations | No claimed capability for untested/out-of-domain regions |
| First-pass yield | Parts accepted without reprint/rework ÷ all first attempts | Report baseline and candidate, including failures/censored runs |
| Final yield | Finally accepted parts ÷ all starts | Report rework burden, not yield alone |
| Accepted throughput | Accepted useful parts ÷ elapsed machine + operator time | Compare on identical task basket |
| Full cost per accepted output | Capital allocation, material, failed attempts, energy, labour, setup, calibration, inspection, finishing, ventilation, maintenance, downtime, transport, rework and warranty reserve ÷ accepted outputs | Lower than a frozen alternate route on at least one volume band, with no quality/safety regression |
| Time to restoration | Request → compatible accepted part installed and service verified | Test Symthaea #7608's proposed ≥25% median/P90 improvement hypothesis only after observed OEM/job-shop baseline |
| Calibration burden | Operator time + artefact material + repeats to reach a fixed acceptance threshold | Symthaea FDM protocol's ≥20% lower calibration time/waste is a hypothesis, not a result |
| Energy and material | kWh, virgin/recycled input, support, scrap and failed-print mass per accepted output | Publish measured values and boundaries |
| Maintenance | MTTR, tool count, skills, spare lead-time, requalification and recommission time | Target service modules repairable with documented common tools; quantify actual times |
| Local productive closure | Separately report local manufacture, local service/assembly, imported-stocked, long-lead and unknown critical BOM dependencies | 100% critical BOM items classified; unknown never counts as local |
| Emissions/exposure | Particle/VOC emissions and facility exposure evidence for exact material/process/site | No household operating claim without the relevant assessment and controls |
| Traceability | Share of accepted units with exact design/BOM/process/machine/material lot/instrument/inspection/repair links | 100% required links for released products; missing records block release |

## 6. Paradigm shifts that can create durable advantage

### 6.1 Compile capability backwards from service demand

Do not start with a catalogue of machines. Start with a resident or factory task and exact product revision, then determine the permitted production routes and capability gaps. This makes a printer one resource in a network, not the centre of the economy.

### 6.2 Design-for-evidence, not merely design-for-manufacture

Make critical features inspectable and repairable by design:
- accessible datums and measurement paths;
- calibration/reference features and test ports;
- replaceable wear modules with stable interfaces;
- readable identity after finishing/repair;
- inspection and service clearances;
- disassembly without destroying unaffected evidence;
- built-in witness artifacts only where their correlation to the actual part is validated.

The test is lower measurement uncertainty and repair/requalification effort without harming functional performance, not the number of sensors or metadata fields.

### 6.3 Capability envelope as a versioned machine passport

Bind as-built frame/gantry, controller and firmware, BOM and licences, sensor map, configuration, spatial test results, material/process envelopes, operator/facility requirements, calibration, repairs and expiry/revocation events.

A replacement generation does not inherit its predecessor's capability automatically. Use Symthaea CIV-BOOT #5782 and PROD-EQP #5964 for multi-generation capability renewal rather than a boolean "self-reproducing" flag.

### 6.4 Active calibration, with no automatic physical compensation in v0

Use Symthaea MFG-ACTIVE research (#7638/#7670) to select the next most informative calibration position/feature/material trial under a fixed budget. Keep discovery and held-out confirmation separate. Calibration suggestions remain offline until independently reviewed and qualified. Do not allow the model to adjust live printer parameters in the first physical pilot.

### 6.5 Distributed production as a service

Households may submit tasks, co-design, learn, assemble, inspect or repair without owning machines. A neighbourhood hub supplies shared machine capacity; regional partners supply specialized processes; Node 0 can act as the reproducible reference cell. Admission and access remain rights-, skill-, safety- and process-bounded.

### 6.6 Reproducing machines without degrading capability

Local frame fabrication is not local machine reproduction. Measure the recursive dependencies: bearings, rails, motors, semiconductors, controllers, sensors, metrology, tools, calibration references, firmware, utilities, extraction and skills. Report what can be made, serviced, stocked, repaired and independently reproduced separately.

### 6.7 Evidence-preserving federation

Use NIST IR 8536 principles—linked time-ordered events, integrity checks and selective disclosure—through Mycelix and exact owner-native schemas. Keep confidential supplier and resident data private; publish verifiable claim summaries and necessary references. No single global manufacturing ledger or universal event row is required.

## 7. Go/no-go sequence for LD-Print v0

1. Demand gate: freeze a real task basket with exact CAD, rights, product revision and observed/forecast demand class.
2. Baseline gate: freeze exact commercial, open-hardware and local service comparators.
3. Architecture gate: compare moving-Z-bed and moving-Z-gantry arrangements for stiffness, dynamics, thermal gradients, tolerance stack, service access, volume, mass and cost.
4. BOM/source gate: classify every critical dependency and obtain like-for-like quotes/sample evidence from multiple suppliers; no inferred price/stock.
5. Safety/emissions gate: machine-specific hazard review, electrical/thermal protection, guarding and source-controlled exposure plan.
6. Design gate: drawings, wiring, bill of materials, licences, assembly/service/commissioning documents and measurement plan reviewed.
7. Prototype gate: unpowered assembly inspection before supervised powered commissioning.
8. Physical qualification gate: exact machine/material/tool/firmware/site + calibrated measurements and independent test.
9. Replication gate: three independent nodes reproduce one admitted low-risk part under the frozen protocol.
10. Value gate: matched full-cost/accepted-output and restoration-time comparison against OEM/bureau/job-shop routes.

Failure at a safety, rights or mandatory acceptance gate cannot be offset by faster printing, lower cost or better circularity.

## 8. Claim ceiling

This document provides standards and industry comparisons plus a proposed benchmark. It does not claim SOTA performance, standards conformity, a safe household printer, an admitted part, a qualified supplier, a local closed supply chain, lower repair time or lower cost. Those claims require the exact design, source data, competent review, calibrated physical measurements and matched independent evaluation.
