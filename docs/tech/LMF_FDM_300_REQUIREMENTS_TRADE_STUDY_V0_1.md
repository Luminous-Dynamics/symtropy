# LMF-FDM-300 Requirements and Architecture Trade Study v0.1

**Status:** requirements/trade-study proposal; no machine architecture is frozen.
**Owner:** [LMF-FDM-300-ARCH-001 #6787](https://github.com/Luminous-Dynamics/symtropy/issues/6787)
**Programme:** [PRODUCTIVE-MACHINE-PLATFORM-001 #6762](https://github.com/Luminous-Dynamics/symtropy/issues/6762)

## 1. Decision before detailed design

The original 300 × 300 × 300 mm envelope is only a concept. It is not yet justified by a frozen part-demand corpus. The next design stage should choose the smallest, stiffest, most serviceable machine that meets the actual Luminous work basket—not maximize build volume for its own sake.

The first machine's reason to exist is not a new motion system. It must make low-risk fixtures, jigs, packaging spacers, prototypes and repair accessories at competitive cost with local repairability, offline operation, reproducible calibration and independent acceptance.

**Do not freeze the frame or order long-lead components until the workload, architecture, BOM and risk review are complete.**

## 2. Current reference machines

These are manufacturer/project-declared specifications, not independent test measurements. Capture exact model/SKU, revision, date, test conditions and source before comparing physical units.

| Reference | Build volume | Published external dimensions | Mass | Useful lesson |
|---|---:|---:|---:|---|
| Bambu Lab H2D, single nozzle | 325 × 320 × 325 mm = 33.8 L | 492 × 514 × 626 mm | 31 kg | High integration, automation and advertised high flow/speed; manufacturer's hotend flow figure has its own specific test conditions |
| Prusa CORE One L | 300 × 300 × 330 mm = 29.7 L | 469 × 521 × 635 mm | 21.9 kg | Larger CoreXY format, chamber management, service ecosystem and spare-part support |
| Voron 2.4 | Officially configured 250/300/350 mm XY classes; Z varies with configuration | Build-specific | Build-specific | Open community BOM/sourcing/build documentation; fixed bed with Z-moving gantry, but more complex mechanics and gantry tramming |

Sources:
- Bambu Lab H2D technical specification: https://cdn1.bambulab.com/documentation/h2d/en/H2D_Laser_Full_Combo_20250305.pdf
- Prusa CORE One L product specifications: https://www.prusa3d.com/p/core-one-l/
- Voron 2.4 architecture and documented size/configuration: https://docs.vorondesign.com/hardware.html
- Voron 2.4 repository and licence metadata: https://github.com/VoronDesign/Voron-2
- Voron BOM/sourcing guide: https://docs.vorondesign.com/sourcing.html

The H2D specification reports a maximum toolhead speed of 1000 mm/s, acceleration of 20,000 mm/s² and 40 mm³/s standard-flow hotend under a stated test model/material/temperature. These are manufacturer test-condition figures, not sustained accepted-part throughput. We must measure usable output with real jobs and rejected attempts in the denominator.

Voron documents the V2 as a static bed with a Z-moving gantry and says that its four-point gantry tramming and additional motors make it more complex. The documented stock configurations span 250 × 250 × 220 mm through 350 × 350 × 330 mm. This is an important counterexample to assuming that fixed-bed mechanics are automatically simpler.

## 3. What the current concept tells us

The current Luminous concept is 300 × 300 × 300 mm = **27.0 L**, inside an approximately 520 × 520 mm frame envelope.

Arithmetic screening only:
- 27.0 L is about 9.1% below the CORE One L's 29.7 L.
- 27.0 L is about 20.1% below the H2D single-nozzle 33.8 L.
- 520 × 520 mm gives a 0.2704 m² concept-frame footprint. This is about 10.7% larger in plan area than the CORE One L's published 469 × 521 mm footprint and 6.9% larger than the H2D's 492 × 514 mm external plan dimensions.

The frame envelope is not the finished machine's external footprint and is not a like-for-like product measurement. It excludes final panels, protrusions, cable access, doors, ventilation connections and service clearance. Still, the concept currently shows no build-volume or footprint advantage. That is a useful design warning, not an independent product comparison.

### Build-volume trade study

Compare at least:
- compact class (e.g. 250 mm XY) for rigidity, low capital and common parts;
- 300 × 300 × 330 mm class (29.7 L) to reach CORE One L parity;
- 325 × 320 × 325 mm class (33.8 L) to reach H2D nominal single-nozzle volume;
- an alternate size dictated by measured fixture/product demand.

For each candidate calculate: share of reference jobs fitting whole, annual print count, support/part-splitting penalty, build time, dimensional deviation over the usable volume, frame footprint, mass, structural stiffness, thermal uniformity, energy, maintenance access, BOM cost and sourcing risk. Use real CAD bounds and a frozen job basket. Do not infer a demand distribution from illustrative examples.

A possible stretch hypothesis is to meet the 29.7–33.8 L reference class without exceeding the relevant commercial footprint, while improving documented repair time, locally replaceable modules, offline usability and independent replication. This is not yet a requirement or result. A smaller printer can still win for a workload that does not need the extra volume.

## 4. Architecture alternatives

### A — CoreXY XY gantry with Z-moving bed

The XY gantry remains fixed in Z while the build platform moves vertically.

Advantages to test:
- fewer moving Z-gantry components and a straightforward mechanical arrangement;
- close commercial precedents, including the CORE One family;
- simpler access and inspection of the upper XY gantry.

Risks to test:
- increasing bed/part mass over Z travel;
- bed support, tilt, lead-screw/belt synchronisation and repeatability;
- thermal expansion and chamber gradients;
- vibration/coupling if the bed support is under-designed.

### B — CoreXY XY gantry with fixed bed and Z-moving gantry

The bed remains stationary; the print gantry moves in Z.

Advantages to test:
- no large moving build-plate mass during operation;
- potentially stable reference platform for some measurement and thermal conditions;
- strong open-hardware precedent in Voron 2.4.

Risks to test:
- more Z actuators and synchronisation;
- gantry racking, tramming and calibration complexity;
- increased moving assembly mass and maintenance burden;
- more components and alignment operations.

### C — Other Cartesian/gantry alternative

Keep an alternative open if a different arrangement is materially easier to source, build, inspect, maintain or qualify for the chosen use case. Do not eliminate a candidate because it is less fashionable.

### Architecture decision rule

Do not pick by weighted aesthetic score. First apply hard gates: part-demand coverage, safety architecture, maintainability, measured stiffness/geometry, sourcing, cost ceiling and testability. Then compare the remaining Pareto options. Preserve rejected alternatives and the evidence that caused rejection.

## 5. Preliminary requirements-to-verification matrix

| ID | Requirement | Verification method | Freeze status |
|---|---|---|---|
| LMF-R01 | The build envelope covers the declared reference part/job basket with minimal splitting | CAD bounds + actual print jobs; report count and annual demand coverage | Open — part corpus absent |
| LMF-R02 | Machine footprint and service clearances are explicitly measured | External envelope drawing and access trial | Open |
| LMF-R03 | Machine meets feature-specific geometric acceptance for admitted parts | ISO/ASTM 52902-inspired artefact, calibrated metrology, uncertainty and repeat builds | Method selected; numerical limits part-specific |
| LMF-R04 | Useful output beats a matched alternative on a declared dimension | Same part, material lot, environment, acceptance, operator method and full cost | Baseline not measured |
| LMF-R05 | Output can be reproduced across 3 independent nodes | Three independent machine/operator nodes; raw feature results and variance | Hypothesis |
| LMF-R06 | Maintenance is possible with documented tools and modular replacement | Timed replacement/repair exercises, tools, skills, downtime and successful recommissioning | Open |
| LMF-R07 | Local operation does not require cloud, account or network access | Disconnect external network; verify local control, queue and safe stop | Proposed requirement |
| LMF-R08 | Power failure enters a safe, non-surprising state | Controlled power interruption, heater/motion stop, no uncontrolled restart, state integrity test | Requires safety design/review |
| LMF-R09 | Any job resume after outage is separately qualified | Controlled recovery tests by material, bed/chamber state and process; otherwise require reprint | Open |
| LMF-R10 | Emissions/extraction arrangement is specific to machine + material + room | Source-control design and appropriate emissions/exposure assessment | No household envelope admitted |
| LMF-R11 | Thermal and electrical hazards have engineered protection independent of network/AI | Risk assessment, circuit/electrical review, fault-injection and proof tests | Open |
| LMF-R12 | Machine firmware/configuration changes are controlled and recoverable | Versioned firmware/config, update verification, rollback and local recovery test | Open |
| LMF-R13 | Every critical component has traceable source status and alternate source or explicit risk | BOM/dependency audit and supplier evidence | No BOM frozen |
| LMF-R14 | Hardware CAD and documentation have an explicit licence and provenance | File-by-file licence review, upstream attribution and legal review | Open |
| LMF-R15 | Cost/energy/material are measured per accepted output | Full cost ledger, kWh, input/scrap/rework mass and accepted output denominator | No physical measurements |

Numeric tolerances must come from the part's function and the measurement system's uncertainty, not from a generic claim that the machine is “0.1 mm accurate”.

## 6. Power, offline operation and South African resilience

A productive machine should remain locally operable without cloud access or vendor-account availability. External connectivity must not be a safety prerequisite. Remote monitoring is optional and non-authoritative.

Power interruption behavior must be designed conservatively:
- machine enters a defined safe state;
- heaters and motion do not restart unexpectedly;
- stored job state is integrity-checked;
- the operator is told whether the job is known-good, recoverable or must be discarded;
- resume is not assumed to preserve part properties or quality.

A UPS or battery does not replace correct protective design. Do not add battery packs or automatic resume features before their electrical, thermal and fault behavior are reviewed.

## 7. Safety and emissions work

Use ISO 12100:2010 machinery risk assessment and risk reduction as the published basis until its successor is formally published. ISO lists ISO/DIS 12100.3 as the project under development; do not cite it as a published standard. IEC 60204-1 is a relevant electrical-equipment-of-machinery reference. The exact applicable editions, national adoption, product standards, statutory rules and competent review must be confirmed at design freeze.

Desktop material-extrusion printing emissions are not solved by a low-odour filament or generic enclosure. Select materials and process profiles, define source capture/ventilation, consider room occupancy, maintenance and filter replacement, and obtain the appropriate exposure/emissions assessment. Household use remains out of scope until that exact envelope is reviewed.

No one-page design can replace a machine-specific hazard analysis. Physical guarding, protective functions and safe stop behavior must remain independent of Symthaea, Mycelix, cloud access and model-generated plans.

## 8. Hardware licensing and design provenance

Audit exact files, not brand names:
- Voron-2 repository identifies GPL-3.0. The exact release and all third-party assets still need review. Using it as a benchmark is distinct from making a derivative.
- Prusa CORE One CAD uses OCL. Prusa describes internal production use as permitted, but commercial sale of machines or remixes based on those files requires separate agreement. This is source-available community licensing, not an unrestricted commercial hardware licence.
- CERN Open Hardware Licence v2 provides strongly reciprocal (S), weakly reciprocal (W) and permissive (P) variants. For Luminous-owned hardware, CERN-OHL-S-2.0 is a strong candidate where the objective is to sell hardware while requiring distributed design modifications to remain shared under the licence. Review exact terms, patent grants, third-party materials, attribution and liability with counsel before selecting it.

References:
- CERN OHL current variants: https://cern-ohl.web.cern.ch/
- Prusa OCL explanation (including commercial sale restriction): https://blog.prusa3d.com/core-one-cad-files-release-under-the-new-open-community-license-ocl_127290/
- Voron-2 declared licence: https://github.com/VoronDesign/Voron-2

The Symtropy software repository licence does not automatically determine the licence for CAD, PCB files, drawings, BOMs or machine documentation.

## 9. Benchmark discipline

Use ISO/ASTM 52902:2023 geometric-capability test artefacts and NIST's open AM-Bench experimental discipline as references. NIST AM-Bench includes carefully controlled measurements and public data; its 2025 polymer challenge includes vat-photopolymerisation, not a direct desktop material-extrusion leaderboard. Luminous should therefore construct a matched MEX test corpus and avoid claiming direct AM-Bench ranking.

Benchmark:
- geometric error and measurement uncertainty;
- functional fit and post-installation results;
- orientation-dependent mechanical behavior where relevant;
- first-pass and final yield;
- accepted pieces per elapsed hour;
- full cost per accepted piece;
- energy, material, support waste and failed builds;
- uptime, MTTR, service access and spare lead time;
- power-loss recovery, offline behavior and safe stop;
- independent replication and calibrated measurement.

Do not claim superiority from maximum advertised speed, acceleration or flow. Compare the same design, material lot, geometry, process settings, environment, measurement method and acceptance predicate. All failed prints and rework remain in the denominator.

## 10. Next design gate

Before detailed CAD:
1. Build the first 20–50 item demand corpus from actual appliance jigs, fixtures, packaging and accessory candidates, with exact part IDs and rights.
2. Freeze the required part envelope and print-volume distribution.
3. Gather exact commercial/open baseline configurations and confirm their specifications and hardware licences.
4. Build a parametric trade study for A/B/C architectures and at least three volume classes.
5. Define preliminary critical-to-function tolerances and an instrument/uncertainty plan.
6. Expand the machine dependency BOM into local-manufacture, local-service, imported-stocked, imported-long-lead and unknown categories.
7. Complete a competent preliminary machine risk assessment before electrical/thermal detailed design.
8. Freeze hardware licence policy.
9. Only then update the OpenSCAD concept into assembly-level CAD and detailed drawings.

**Claim ceiling:** this study does not select a final architecture, authorize procurement, establish machine safety, verify material/process capability, confirm local sourcing or demonstrate SOTA performance. It is a requirements and decision gate.
