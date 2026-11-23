# Productive Machine Benchmark Protocol v0.1

**Status:** preregistration template and benchmark design; no comparative experiment has been conducted.
**Owner:** [PRODUCTIVE-MACHINE-PLATFORM-001 #6762](https://github.com/Luminous-Dynamics/symtropy/issues/6762)

## 1. Why a matched benchmark is required

Published machine specifications are capability claims under manufacturer-defined test conditions. Advertised maximum speed, acceleration, build volume or flow rate is not the same thing as accepted part throughput, dimensional accuracy, mechanical performance, repeatability, availability or lifecycle cost.

The benchmark must compare the same frozen output, process envelope, material lot, environment, measurement system and acceptance criteria. It must retain all failed attempts, rework, setup, calibration, downtime and inspection.

NIST's Additive Manufacturing Test Artifact is an established reference method: build the artifact on the machine, document all process settings, then measure geometric characteristics and compare deviations across machines. ISO/ASTM 52902:2023 also provides geometric-capability test artefacts. Luminous should use these references plus a task-specific functional artifact; no single benchmark part captures every machine capability.

## 2. Current reference baselines

The first comparison set should contain one machine from each relevant class, selected and frozen before testing:

| Baseline | Publicly stated capability | What it tests |
|---|---|---|
| Bambu Lab H2D (single nozzle) | 325 × 320 × 325 mm build volume; manufacturer's specification also lists maximum toolhead speed/acceleration and hotend flow under stated test conditions | Strong integrated commercial reference for throughput, automation, multi-material/dual-tool features, enclosure and consumer workflow |
| Prusa CORE One L | 300 × 300 × 330 mm build volume, CoreXY, enclosed chamber and active heating in manufacturer's current product information | Serviceable commercial ecosystem and a same-class build envelope |
| Voron 2.4 | Configurable 250/300/350 mm sizes; community-maintained documentation, CAD, BOM/configurator and sourcing guide | Open community-built reference for repair/build-from-parts and user-maintained machine |
| Local job-shop / bureau | Quote and actual measured service for the identical released part | Real alternative to buying, maintaining and operating a machine |
| LMF-FDM-300 | Concept target 300 × 300 × 300 mm; not yet engineered or built | Candidate under test, not a proven competitor |

Sources:
- Bambu Lab H2D technical specification: https://cdn1.bambulab.com/documentation/h2d/en/H2D_Laser_Full_Combo_20250305.pdf
- Prusa CORE One L current specifications: https://www.prusa3d.com/product/prusa-core-one/
- Voron documentation: https://docs.vorondesign.com/
- Voron configurable sizes: https://vorondesign.com/voron2.4
- Voron BOM/sourcing guide: https://docs.vorondesign.com/sourcing.html
- NIST AM test artifact: https://www.nist.gov/el/intelligent-systems-division-73500/production-systems-group/nist-additive-manufacturing-test

Manufacturer specs must be captured with exact model, variant, firmware, date and source. Do not substitute the H2D Laser Edition / dual-nozzle volume for its single-nozzle volume. Do not combine CORE One and CORE One L specifications. Do not treat Voron community build options as factory-certified performance.

## 3. Frozen test article set

Use three artifacts, each with a different purpose:

1. **Geometric capability artifact:** NIST AM test artifact or an ISO/ASTM 52902-compatible artifact. Measure critical dimensions, holes/pins, thin walls, flatness/warpage, angles, bridges/overhangs and repeatability with a documented measurement uncertainty.
2. **Functional reference part:** a low-risk, rights-cleared geometry taken from the actual appliance/tooling programme. Freeze CAD revision, interface dimensions, load/use constraints, installation and acceptance check. It must not be safety-critical.
3. **Material witness coupons:** print coupons in XY and Z orientations from identified material lots and test only the properties relevant to the admitted use. Record conditioning, process profile and failure mode; do not infer whole-part strength from a single coupon.

No one artifact should be used to claim complete machine qualification. Separate geometric, mechanical, thermal, process, exposure and safety evidence.

## 4. Controlled variables

Freeze and publish:
- exact printer model/configuration, hardware revisions, maintenance state and firmware;
- slicer and profile version, G-code digest and all machine settings;
- filament manufacturer, grade, lot, colour, storage, drying/conditioning and measured mass;
- nozzle type/diameter/wear, build plate and adhesive/cleaning protocol;
- build orientation, supports, infill, perimeters, layer height and cooling;
- ambient temperature/humidity and chamber temperature where applicable;
- operator training and setup protocol;
- instrument identity, calibration status, measurement method and uncertainty;
- inspection/acceptance rules, exclusions and failure definitions.

Run a randomized or blocked sequence to avoid confounding machine condition with operator, material lot or environmental drift. Preserve raw measurements and rejected samples.

## 5. Metrics

| Domain | Required outputs |
|---|---|
| Geometry | Signed and absolute error by feature; flatness/warpage; repeatability; measurement uncertainty |
| Mechanical | Relevant tensile/flexural/impact or part-specific test; orientation dependence; failure mode; test conditions |
| Functional | Fit, assembly effort, installation success, task completion and post-repair verification |
| Process | Build time, setup, queue, calibration, failed attempts, first-pass and final yield |
| Economics | Total cost per accepted part including capital allocation, labour, energy, material, waste, inspection, ventilation, maintenance, transport and rework |
| Availability | Request-to-accepted-part P50/P90, MTTR, downtime, spare-part waiting, maintenance burden |
| Resource use | kWh, material input, support/scrap mass, failed-print mass and verified recycled-content fraction |
| Safety/exposure | Guard/interlock test results, thermal protection, emission controls and facility/operator exposure review |
| Openness/serviceability | Source availability, BOM completeness, hardware licence, replacement part availability, service time and documented repairability |

## 6. Experimental sequence

**Screening**
- One exact machine configuration per baseline.
- At least five repeated builds per artifact/material/profile cell for setup and variance discovery.
- Randomize order where practical.
- Report raw values, mean/median, range, spread and measurement uncertainty.
- This phase only estimates variance and finds protocol failures; it does not establish statistical superiority.

**Confirmatory comparison**
- Determine sample size from a preregistered effect size, observed variance and desired statistical power.
- Include independent operators and, where feasible, multiple physical units of each machine.
- Use a frozen comparison protocol and blinded part measurement where practical.
- Report confidence intervals/effect sizes and every exclusion; use a multiple-comparison plan if many metrics are tested.
- A P50/P90 improvement claim requires enough observations and a defensible sampling frame; a few demonstration prints are not sufficient.

**Replication**
- Reproduce one admitted low-risk part on three independent machine/operator nodes.
- Compare feature distributions and functional acceptance, not only identical file hashes.
- Report node-specific deviations and process adjustments; do not conceal them by averaging.

## 7. LMF-FDM-300 go/no-go criteria

Do not freeze a 300 mm build volume merely because it appeared in the first envelope model. Compare machine volume, frame footprint, build-volume fraction, stiffness, access and serviceability against the reference machines. The present 300 mm cube is within the market class, not an advantage over H2D or CORE One L.

Proceed from concept to detailed mechanical design only when:
- the primary use basket proves the selected build envelope is useful;
- a requirements trade study explains why 300 mm cube is preferable to a smaller, stiffer or larger design;
- kinematic architecture, linear motion, drive system, Z system, frame/joint concept, thermal system and maintenance access are defined;
- a complete preliminary BOM has source alternatives and single-source risks;
- machine-specific hazards and safeguards have a reviewed design path;
- hardware CAD/documentation licensing is explicitly selected and recorded;
- baseline access and the measurement plan are feasible.

A design that is slower than commercial equipment can still win on serviceability, lifecycle cost, openness and local uptime. It must demonstrate that with matched measurements.

## 8. Statistical and claim rules

- Do not use vendor maximum speed as measured production rate.
- Do not use theoretical volume or nominal accuracy as measured capability.
- Do not omit failed jobs from throughput or unit cost.
- Do not claim a process is stable or Cpk-capable until stability and sample-size prerequisites are met.
- Do not claim mechanical properties from CAD, slicer settings or geometry alone.
- Do not claim an emissions-control profile from enclosure presence or filter labels alone.
- Do not claim safety or conformity from an open-source design or an E-stop switch.
- Do not claim the local network is self-sufficient when motors, electronics, rails, fasteners, feedstock or skills remain imported; report each capability dependency and its evidence class separately.

## 9. Target hierarchy

The first objective is **parity with a matched baseline on the selected low-risk task**, with complete traceability and safe operation. The second is to win on at least two Pareto dimensions—such as cost per accepted part, local repair turnaround, independent replication or machine maintainability—without unacceptable regression in quality, safety, reliability or lifecycle cost. The stretch target of 25% lower P50/P90 restoration time should be tested only after a local OEM/job-shop baseline is observed and preregistered.

This is a research target, not a claim of SOTA performance.
