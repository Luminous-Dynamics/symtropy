# Luminous Productive Machine Platform v0.1

**Status:** research/design specification. No machine has been fabricated, commissioned, safety-qualified or commercially benchmarked by this document.
**Programme owner:** [PRODUCTIVE-MACHINE-PLATFORM-001 #6762](https://github.com/Luminous-Dynamics/symtropy/issues/6762)
**Composes:** [APPLIANCE-MICROFACTORY-001 #6603](https://github.com/Luminous-Dynamics/symtropy/issues/6603), [DISTRIBUTED-APPLIANCE-FAB-001 #6735](https://github.com/Luminous-Dynamics/symtropy/issues/6735), [CITYKIT-SYNTH-001 #3768](https://github.com/Luminous-Dynamics/symtropy/issues/3768), [ABOVEGROUND-PANELCELL-001 #3773](https://github.com/Luminous-Dynamics/symtropy/issues/3773), [EV-FACTORY-DESIGN-001 #2354](https://github.com/Luminous-Dynamics/symtropy/issues/2354), and Symthaea [MFG-PROC-000 #5686](https://github.com/Luminous-Dynamics/symthaea/issues/5686).

## 1. Product thesis

Luminous should build **productive capability**, not a collection of disconnected machines. A machine is a maintained capability with a defined process envelope, qualified feedstocks and tools, trained operators, safeguarding, calibration, maintenance, measured output quality and a reproducible repair route.

The differentiator is not a new printer motion system by itself. It is a supplier-neutral, locally serviceable machine family coupled to:
- open editable design files and explicit hardware licensing;
- serviceable modules and locally sourceable BOMs;
- machine/process capability profiles and calibration evidence;
- digital part packages linked to exact product revisions and rights;
- independent inspection and accepted-output receipts;
- whole-life economics and matched commercial/open baselines;
- a distributed H0 household → N1 neighbourhood → R2 regional partner → Q3 qualified industrial network.

Households participate without being required to own a printer. A household node is suitable only where the machine, material, ventilation, skill and part-risk envelope all permit it.

Do not make one universal machine. FDM, CNC, polymer recycling, textiles and electronics have different forces, dust, heat, emissions, materials and failure modes. Share identity, data, calibration, service and evidence interfaces; engineer each physical process separately.

## 2. Machine family

| ID | Machine | Initial role | Stage |
|---|---|---|---|
| M0 | Productive workbench / metrology / assembly station | Workholding, repair, torque-controlled assembly, dimensional/electrical checks, calibration register | First enabling asset |
| M1 | LMF-FDM-300 enclosed CoreXY MEX printer | Low-risk fixtures, jigs, accessories and fit-checks | First powered machine design |
| M2 | LMF-CNC-400 enclosed bench CNC router | Bounded wood/plastic machining; no default household operation or metal-cutting claim | Later N1/R2 design |
| M3 | LMF-RECYCLE-01 polymer recovery line | Identified polymer sorting, cleaning/drying, size reduction, extrusion/filtration and batch testing | N1/R2 only |
| M4 | Textile + low-voltage electronics repair/assembly cells | Repair, covers, harnesses and small-batch assembly using established equipment | Adopt/fixture first; automate selectively |

### M0 — productive workbench

Design the station before buying a production machine. Freeze the bench load/stability envelope, workholding interfaces, lighting, ESD area where needed, tool layout, guarded fixtures, torque-tool checks, caliper/micrometer/balance requirements, low-voltage test equipment, safe isolation procedures and calibration register. It becomes the first repeatable assembly and inspection capability, not just furniture.

### M1 — LMF-FDM-300

The initial concept target is a 300 × 300 × 300 mm class build envelope within an approximately 520 × 520 × 620 mm frame envelope. These are editable concept dimensions, not final machine requirements or a build specification.

The first design should prioritize:
- common extrusions/rails/belts/bearings/fasteners and replaceable subassemblies;
- frame and gantry stiffness, vibration modes, thermal stability and tolerance stack;
- safe hot-end/bed operation, thermal runaway and over-temperature protection, guarded motion, service access and appropriate safety-related functions;
- enclosed material-extrusion operation and a documented source-control/ventilation profile, validated for chosen materials;
- controlled firmware/configuration releases, calibration, geometric test artefacts and maintenance history;
- non-critical printed covers/ducts/guides only after review; critical structure and safety-related parts must be qualified metal or bought components.

A visual envelope model is in `cad/productive-machines/lmf_fdm_300_envelope_v0_1.scad`. It depicts only the frame, build volume, bed and gantry placeholders. It does not yet define rail layout, belt path, Z kinematics, motor sizing, heater/electrical system, thermal model, stiffness, tolerances, guarding or a safe operating envelope. It is not fabrication-ready.

Use Voron and Prusa CORE One as strong open/commercial baselines before selecting a novel design. Do not assume their licensing terms are interchangeable. Voron design repositories identify GPL-3.0 licensing; Prusa's CORE One CAD release uses its Open Community License. Record the exact upstream version and satisfy its terms before deriving or redistributing CAD. A Symtropy software repository licence does not automatically license hardware drawings.

### M2 / M3 — separate process machines

CNC requires structural rigidity, spindle/tool envelope, workholding, tool/runout checks, guarding, access interlocks, chip/dust extraction, stop behavior and geometric/performance tests. Begin with suitable wood and plastics; do not silently extend to metal.

Polymer recovery requires known resin identity and lot custody, contamination quarantine, cleaning/drying, process temperature control, size reduction guards, filtration, output traceability and batch property tests. Recycled polymer is not equivalent to virgin polymer without evidence. Shredders and hot extruders belong in competent N1/R2 facilities, not an unqualified home workshop.

## 3. Shared machine capability profile

Each machine configuration must bind:
- machine ID, design/configuration revision, BOM digest, upstream sources and hardware licence;
- intended process, capability tier, operating domain and prohibited uses;
- machine risk assessment, guards, interlocks, safety functions and proof-test procedure;
- operator skill/training, site, utilities, emissions/dust/noise controls;
- qualified materials/feedstock, lot and conditioning;
- tooling, fixtures, sensors and calibration status;
- firmware/controller/profile digests and configuration;
- validated envelope, limits, uncertainty and known failure modes;
- maintenance intervals, spares, repair procedures and fault recovery;
- reference artefacts, inspection/test methods, acceptance criteria and raw data;
- first-pass/final yield, cost per accepted output, energy, scrap, downtime and labour;
- commissioning, independent replication, review and expiry evidence.

A hash proves identity of recorded bytes, not engineering correctness, licensing, feedstock authenticity or output conformity. A machine's self-report is not independent acceptance. A simulator pass is not physical safety evidence. A QR code is not a product qualification.

## 4. Part admission and safety boundary

Consume #6735's P0–P3 part-risk and H0–Q3 capability model. Household printing starts with rights-cleared P0 organisational/cosmetic items only, and only inside a reviewed printer/material/ventilation/training envelope. N1 may consider individually admitted P1 items after operator, material, process and inspection requirements are met. P2 routes to qualified R2/Q3 processes. P3 remains with OEM or competent qualified industrial routes.

Do not home-print or casually substitute mains enclosures/insulation, rotating guards, blades/impellers, pressure or hot-water containment, refrigerant parts, hot-surface barriers, structural/load-bearing items, fire barriers, or food/water-contact parts. The list is a screening boundary, not a complete hazard analysis. Product-specific conformity and legal decisions belong to competent authorities and qualified specialists.

## 5. Industry comparison and benchmark programme

Compare a frozen task and output against:
1. a strong commercial machine/process;
2. a strong open-hardware machine/process where one exists;
3. local conventional job-shop/OEM routing;
4. household or neighbourhood routing only when the part/process is admitted.

Freeze design revision, material/lot, build orientation, tool, environment, operator capability, firmware, measurement system and acceptance criteria. Publish raw results, exclusions and uncertainty.

| Metric | Measurement contract |
|---|---|
| Cost | Total cost per accepted output, including capital, failed builds, material, labour, energy, ventilation, inspection, finishing, calibration, maintenance, downtime, transport and rework |
| Quality | Critical-feature error + measurement uncertainty; strength/stiffness/orientation and exposure tests where relevant |
| Yield | First-pass and final yield; all failures, scrap, rework and rejected attempts in denominator |
| Throughput | Accepted output per elapsed time, including setup/changeover, queue, calibration and inspection |
| Availability | P50/P90 request-to-accepted-part and verified restoration time; uptime, MTTR and spares waiting |
| Reproducibility | At least three independent machine/operator nodes for a low-risk reference artefact |
| Energy/material | kWh and feedstock/scrap mass per accepted output; recycled content verified by source/lot |
| Safety | Safeguard/interlock proof tests, emission/dust controls, operator exposure, near misses and abnormal recovery |
| Capability closure | Locally maintained, repairable and reproducible machine dependencies reported separately; imported capability is not counted as local production closure |

Initial targets are hypotheses, not results:
- 100% of released files have explicit rights, exact compatibility, part-risk, allowed tier and limitations.
- 100% of machine releases have BOM, source, licence, assembly/repair guide, risk file, wiring/control diagrams and commissioning method.
- 100% rejection of seeded wrong configuration, stale calibration, unsupported material/tool, modified build file, missing inspection and bypassed guard/interlock.
- three independently operated nodes reproduce one low-risk part/machine configuration to predeclared tolerances.
- test a stretch hypothesis of ≥25% lower P50/P90 restoration or accepted-output time on a frozen low-volume basket without regression in safety, reliability, fit/function or total cost.
- publish economics at annual volumes 1/3/10/50/100; report all included costs.
- do not claim Cpk until process stability and adequate sample size are established.

Do not claim superiority from advertised maximum print speed, nominal spindle RPM, BOM price or a one-off demonstration. The metric is the accepted, safe, useful output over its whole life.

## 6. Standards and technical basis

Use current editions and scope carefully; re-check status before design freeze:
- ISO 12100:2010, machinery risk assessment and risk reduction (current listing as at 22 Nov 2026; DIS 12100.3 is under development).
- IEC 60204-1:2016+A1:2021, electrical equipment of machinery.
- ISO 230-1:2012, geometric accuracy tests for machine tools under no-load/quasi-static conditions; not a complete dynamic-performance or safety test.
- ISO/ASTM 52902:2023, geometric capability test artefacts for AM systems.
- ISO/ASTM 52920:2023, industrial AM processes and production sites.
- ISO/ASTM 52951:2026, AM data-package model from design through acceptance; detailed basis is metal PBF-LB/M, so borrow digital-thread principles, not FDM conformity.
- ISO/ASTM 52966:2026, AM capability levels for temporary self-sufficient systems; useful by analogy for tiered capacity, not a household certification.
- ISO 27548:2024, desktop material-extrusion particle/chemical emissions chamber test; measured chamber emissions do not necessarily equal every real-room exposure.

NIOSH 2024-103 recommends source control, material/process substitution, enclosures/local exhaust and operating procedures for desktop 3D printing. A low-odour filament, generic enclosure or consumer filter is not a quantified exposure-control qualification. Initial household pilot excludes resin/vat photopolymerisation.

## 7. South African ecosystem: partner, don't duplicate

The CSIR Advanced Design and Manufacturing Innovation Centre describes polymer/powder AM, 3D scanning, design and product-lifecycle capabilities supporting SMMEs. CSIR Future Production and the Centre for Robotics and Future Production describe wider production, automation, inspection and skills capability. These are capability leads, not an access agreement, quotation or confirmed partner. The located 2025/26 CPAM industry-led call closed 18 February 2026; no currently open funding call is assumed.

Precious Plastic is a useful open-hardware precedent for local machine-building and polymer recovery, including designs intended to be fabricated through local metal shops. Its adoption counts are self-reported. Use it as a comparator and learning source, not proof that recycled feedstock is automatically qualified.

## 8. Execution gates

- **D0 — requirements:** freeze the M0 workbench and M1 printer use cases, operating envelope, acceptance outputs and exclusions.
- **D1 — baseline:** select commercial/open baselines and audit hardware licences.
- **D2 — engineering:** frame/gantry/thermal/electrical concept, risk file, BOM, drawings, tolerances and sourcing.
- **D3 — design review:** independent mechanical/electrical/safety review; close critical hazards before build.
- **D4 — prototype:** fabricate with documented supplier/material/tool identities; no production claim.
- **D5 — commissioning:** test safety functions, geometric capability, thermal behavior, emissions controls, repeatability and maintenance.
- **D6 — replication:** three independent builds/nodes and matched output economics.
- Start CNC and polymer recovery only after separate process-specific requirements and risk gates.

## References

- ISO/ASTM 52966:2026: https://www.iso.org/standard/89300.html
- ISO/ASTM 52951:2026: https://www.iso.org/standard/76828.html
- ISO/ASTM 52902:2023: https://www.iso.org/standard/82056.html
- ISO/ASTM 52920:2023: https://www.iso.org/standard/76827.html
- ISO 27548:2024: https://www.iso.org/standard/83712.html
- ISO 12100:2010: https://www.iso.org/standard/51528.html
- IEC 60204-1:2016+A1:2021: https://webstore.iec.ch/en/publication/71256
- ISO 230-1:2012: https://www.iso.org/standard/55295.html
- NIOSH 2024-103: https://www.cdc.gov/niosh/docs/2024-103/
- CSIR ADMIC: https://wwwprod.csir.co.za/advanced-design-and-manufacturing-innovation-centre
- CSIR manufacturing: https://www.csir.co.za/what-we-do/manufacturing
- CSIR Robotics and Future Production: https://www.csir.co.za/centre-robotics-and-future-production
- Voron design repository: https://github.com/VoronDesign/Voron-2
- Prusa CORE One CAD release: https://blog.prusa3d.com/open-source-core-one-files_110319/
- Precious Plastic machines: https://www.preciousplastic.com/machines

**Claim ceiling:** this profile and concept CAD do not establish a fabricated or safe machine, compliance, qualified process, validated emission controls, local-content percentage, supplier agreement or competitive advantage. Physical tests and independent review are required.
