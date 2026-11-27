# Magnetic additive manufacturing route tournament

**Protocol ID:** MAG-AM-ROUTE-TOURNAMENT-001  
**Issue:** [Symtropy #7757](https://github.com/Luminous-Dynamics/symtropy/issues/7757)  
**Revision:** 0.1-draft  
**Date:** 27 November 2026  
**Status:** literature-stratified protocol; no matched physical route result claimed

## 1. Decision question

Which manufacturing route can deliver the best **accepted component** for a defined magnetic field or mechanical function over its operating envelope, after material properties, geometry, texture, process variation, inspection, manufacturing yield, cost and lifecycle requirements are included?

This is not a contest to maximize one material scalar. The target is the realized field map, force/torque, gradient, encoder phase, leakage or other application response of an identified as-built specimen.

## 2. Current literature landscape

The September 2026 review *State-of-the-art additive manufacturing of NdFeB permanent magnets: challenges and pathways toward high performance* reports that published AM NdFeB remanence values generally fall around 0.6–1.0 T. It identifies material extrusion (MEX) as the most competitive published route for bonded NdFeB remanence, while laser-based powder-bed routes offer geometric freedom but struggle to establish magnetic texture; SLS and binder jetting remain constrained by porosity. This is a review-level synthesis across different studies, not a matched route tournament.

The March 2026 review *Bonded Nd-Fe-B magnets: Materials, processing, properties and outlook* highlights the filler-loading/binder trade-off: more binder assists flowability and processing but dilutes magnetic energy product; higher powder loading raises magnetic fraction while making mechanical strength and processing more difficult.

These findings imply that a route's value depends on the target:
- High remanence may dominate a compact motor or generator design.
- Fine local material zoning and complex geometry may dominate a patterned-pole, encoder or field-shaping component.
- Mechanical strength, low eddy-current loss, dimensional repeatability or repairability may dominate other devices.
- A low nominal material cost is irrelevant if first-pass accepted-part yield is poor.

The comparison must remain stratified by feedstock, material form, specimen geometry, post-process and measurement method.

## 3. Route families and comparison strata

### 3.1 Material extrusion (MEX)

Record feedstock filament/pellet route, polymer and additives, NdFeB grade and source lot, filler fraction (mass and volume basis separately), extrusion/deposition path, layer/road dimensions, field-assisted orientation if used, thermal history, voids, post-processing and magnetization history.

Potential strength: published route-level remanence leadership among AM bonded NdFeB. Main trade-offs to measure: filler loading, flowability, bead/layer interfaces, anisotropy, dimensional tolerances, strength and process stability.

### 3.2 Laser powder-bed fusion / selective laser melting (LPBF / SLM)

Record powder source/lot, PSD and morphology, chemistry/phase, powder reuse history, layer height, scan strategy, laser/thermal history, atmosphere, density/porosity, texture, geometry, binder if applicable and post-processing.

Potential strength: geometric flexibility and dense metal components in applicable process/material systems. Main questions: phase retention, oxidation/decomposition, magnetic texture, shrinkage, thermal damage and the exact material regime.

Do not conflate metal SLM with polymer-bonded LPBF recipes or transfer settings between different machines/material lots.

### 3.3 Selective laser sintering (SLS)

Record hard/soft/nonmagnetic material fractions, local powder placement, polymer matrix, field history during deposition, layer geometry, post-print magnetization and probe/background-field conditions.

Potential strength: patterned multi-material pole structures and spatially localized field functions. Distinguish local field in the build region, post-magnetization field and measured probe field.

### 3.4 Binder jetting

Record powder chemistry/PSD, binder and saturation, layer process, depowdering, sintering/infiltration or other declared post-processing, shrinkage compensation and post-magnetization.

Measure porosity, density gradients, feature fidelity, dimensional change, texture, mechanical integrity and local field function. Do not assume that binder-jet permanent magnets share the same material or post-process physics as MEX or LPBF.

### 3.5 Lithography-based composite manufacturing (LCM) and photopolymer routes

Record suspension formulation, NdFeB filler fraction and particle distribution, field-assisted orientation and timing, resin cure, electron-beam or other post-cure dose and sequence, specimen condition and magnetic measurement method.

For Kim et al. (2025), retain the source's field-aligned LCM and electron-beam post-cure as a distinct process route. Reconcile the publisher's dataset DOI against the candidate repository metadata before treating a dataset as verified.

### 3.6 Adjacent near-net-shape and conventional controls

Powder extrusion moulding (PEM), compression/injection moulding, conventionally bonded magnets, sintered magnets and segmented arrays can be useful engineering comparators. They remain separate route strata. Do not relabel PEM or conventional methods as AM to make a single additive leaderboard.

## 4. Paradigm shift: function-normalized route selection

Use this design sequence:

```
application requirement and operating envelope
    -> target vector field / force / torque / gradient / sensor response
    -> admissible component geometry and material topology
    -> route-specific feedstock and process constraints
    -> realized material state, geometry and magnetization history
    -> predicted and measured as-built function
    -> uncertainty-qualified acceptance
    -> application-specific robust Pareto frontier
```

The unit of comparison is the accepted manufactured component, not a bulk material datasheet value.

Do not normalize away the physical differences between routes. Preserve the source measurand and compare only when specimen condition and measurement method are matched or a qualified conversion exists.

## 4B. Evidence maturity: MOEM M1–M5

Adopt the qualification-oriented maturity ladder proposed by Fodor et al. (2026), *Materials-Driven Additive Manufacturing for Electrical Machines: Qualification-Oriented Design Beyond Geometric Freedom*, DOI [10.1002/adem.71259](https://doi.org/10.1002/adem.71259). This is a research reporting framework, **not a formal standard or certification scheme**.

Every route/candidate result must report its highest demonstrated maturity, the exact supporting evidence receipts and the gaps that prevent the next level. Maturity is not a route ranking, and a higher label must not be assigned merely because a component was printed or a simulation converged.

| Level | Evidence class | Magnetic manufacturing interpretation | Minimum evidence boundary |
|---|---|---|---|
| M1 | Conceptual / modelling | Geometry, route or machine concept; analytical, numerical or design-based evidence only | No experimental material validation |
| M2 | Coupon-level material evidence | Printed or deposited specimens characterized for magnetic/material properties | Property data are not yet linked to component geometry or application duty |
| M3 | Controlled PSPP validation | Process parameters, microstructure, defects, interfaces and properties are linked under controlled conditions | Limited component or duty-cycle validation |
| M4 | Functional component / subsystem | Component or subassembly tested under application-relevant electromagnetic, thermal and mechanical conditions | Reproducibility, lifetime and scale-up remain limited |
| M5 | Qualification-relevant evidence | Reproducible manufacturing, variability control, durability and qualification-relevant performance demonstrated | Requires a defensible production/qualification evidence package |

### Maturity gates for permanent-magnet routes

- **M2:** Record the exact specimen, material grade/lot, property method, temperature and material state. For hard magnets, include remanence / polarization, intrinsic coercivity, recoil behavior and method-scope limitations. A datasheet ceiling, starting powder or best coupon does not become component evidence.
- **M3:** Bind process conditions and post-processing to microstructure, texture/orientation, porosity/defects, interfaces and magnetic properties. Separate in-process alignment fields from post-build magnetization. Include measured process-to-property relationships and uncertainty.
- **M4:** Demonstrate the actual component/subassembly's target field or mechanical function under application-relevant gap, temperature, speed, external field, armature-reaction / field-weakening and mechanical-retention conditions, as relevant. Record bondline/airgap tolerances and magnetic bypass paths for hybrid assemblies.
- **M5:** Require repeated valid builds and material lots, controlled variability, durability and qualification-relevant component performance. A single excellent component is not M5; a cohort denominator alone is not M5; simply meeting a minimum build count is not M5.

A route tournament must stratify results by maturity. Do not compare an M2 coupon remanence with an M4 functional component response as equivalent evidence. A lower-maturity route may remain a promising research candidate, but its ranking must be clearly marked as feasibility-only.

### Application-level economics

The 2026 perspective argues that industrial adoption should be judged by **cost-per-performance at the intended production volume**, not printability alone. For permanent-magnet and hybrid assemblies, report cost-per-torque or cost-per-power where applicable alongside magnetic function, remanence/coercivity at temperature, demagnetization margin, mechanical retention, build rate, post-processing, inspection, yield, scrap and process capability. These should remain separate metrics or explicit application-specific ratios; do not hide a failed reliability gate inside a favorable cost metric.

## 4C. November 2026 state-of-the-art envelope and component-level competitive test

The September 2026 review *State-of-the-art additive manufacturing of NdFeB permanent magnets: challenges and pathways toward high performance* (Journal of Science: Advanced Materials and Devices 11(3), 101226, DOI [10.1016/j.jsamd.2026.101226](https://doi.org/10.1016/j.jsamd.2026.101226)) reports AM NdFeB remanence values generally around 0.6–1.0 T across the literature. In that review, MEX is the most competitive printed route for bonded NdFeB and reaches the highest reported remanence among the surveyed AM routes; LPBF can produce dense parts but has not demonstrated the magnetic texture needed for high performance; SLS and binder jet remain constrained by porosity. The same review reports conventional anisotropic sintered NdFeB around 1.4–1.5 T. These are aggregated literature ranges, **not matched specimens, common measurement conditions or universal route limits**.

Fodor et al. (2026), DOI [10.1002/adem.71259](https://doi.org/10.1002/adem.71259), give representative property points for different material forms: isotropic bonded NdFeB feedstock datasheet ceilings around Br 0.73–0.76 T, early LPBF NdFeB around Br 0.59 T / Hcj 695 kA/m / (BH)max 45 kJ/m³, aligned SmFeN in PA12 around Br 0.68 T / Hcj 501 kA/m / (BH)max 78.8 kJ/m³, and a SmCo-Al cold-spray composite example around Br 0.35 T / Hcj 863 kA/m / (BH)max 20.7 kJ/m³. These figures come from different feedstocks and studies. They are **route/material context only**, not directly rankable without matched temperature, specimen geometry, method, density, orientation, uncertainty and magnetic-state conditions.

### A different competitive question

Do not frame the entire programme as “make printed NdFeB reach sintered-NdFeB remanence.” Sintered anisotropic NdFeB remains the high-performance reference. AM can win by changing the component architecture: topology-specific PM placement, segmented flux paths, material-efficient shapes, integrated retention, reduced machining scrap, thermal/mechanical integration, or hybrid PM-assisted reluctance designs. But architecture freedom is not automatically an engineering advantage.

The tournament must compare against the strongest relevant conventional baseline, not an intentionally weak or incomplete design:

1. optimized conventional sintered-NdFeB rotor/assembly at the same duty point;
2. optimized conventional bonded-PM component where the AM feedstock is bonded;
3. optimized PM-assisted synchronous-reluctance baseline where that is the actual system comparator;
4. AM-shaped or functionally integrated PM insert;
5. AM/hybrid architecture with integrated retention, segmentation or other claimed advantage.

Use identical application target, winding/inverter, cooling boundary, operating envelope, duty cycle, air gap and temperature assumptions. State the system boundary and which parameters were co-optimized. Do not treat a torque gain over a reluctance-only rotor as proof of superiority over an optimized conventional PM rotor.

### Non-compensatory function and architecture metrics

Report separate measured/qualified values:
- remanence (B_r(T)), intrinsic coercivity (H_{cj}(T)), recoil/demagnetization curves and irreversible flux loss after representative thermal/current exposure;
- demagnetization margin under worst-case armature reaction, field weakening, fault current and maximum magnet temperature;
- air-gap field-map error and harmonic content, torque, torque ripple, torque density and power density;
- magnet mass, magnet volume, rotor inertia and active-material utilization;
- mechanical retention margin, fatigue/vibration and bondline/interface stability;
- eddy-current and core losses, efficiency over the declared duty cycle and cooling/pump parasitics;
- first-pass yield, accepted-part cost, post-processing, inspection and cost per torque/power at the intended production volume.

An AM route is a competitive win only when its application-specific value is non-dominated against the relevant optimized conventional baseline, while all hard requirements (especially coercivity/demagnetization, thermal stability, retention and durability) remain satisfied. A lower remanence material can still be valuable if architecture delivers the same qualified function with lower PM mass, inertia, scrap or life-cycle cost; it is not a win if the supposed gain merely shifts cost or risk elsewhere.

## 5. Common target and matched experiment design

Choose one application target before selecting numeric improvement thresholds. Candidate targets:
- small multipole encoder magnet with a specified pole-pair count and working gap;
- patterned two-pole / multi-pole surface magnet with a defined map and standoff;
- compact Halbach-like field concentrator with a specified usable field region;
- application-specific motor/actuator segment with torque and thermal limits.

For each target:
1. Freeze the target field/function, coordinate frame, operating envelope and hard constraints.
2. Freeze a common outer envelope and measurement geometry where each route can manufacture it.
3. Record route-specific feedstock and process constraints; do not require physically incompatible inputs to be identical.
4. Prefer splitting common source lots into route-compatible fractions; otherwise separate lot and route effects.
5. Manufacture repeated specimens over more than one build and, where possible, more than one lot.
6. Inspect as-built geometry and material state; bind to each specimen's genealogy.
7. Use a common independent measurement reference and matched sensor plane/standoff/grid where feasible.
8. Keep calibration/fit, adaptive exploration, blind holdout and final confirmation separate.
9. Record all deviations and failed builds. Do not report only the best specimen.

## 6. Measurement semantics that must not be collapsed

Keep distinct:
- mass-specific magnetization (A·m²/kg) and volumetric polarization/remanence (T);
- intrinsic Br / J / Hcj / (BH)max and a measured surface or working-gap field;
- field measured with background/external field present and zero-background remanent field;
- in-process alignment field, post-build magnetization field and field during measurement;
- <cos(theta)> alignment degree, Br/Js ratio, texture distribution and other method-specific metrics;
- sensor-plane field, total stray field, gradient, force and torque;
- nominal CAD geometry and independently inspected as-built geometry.

For hard-magnet properties, IEC 60404-5:2015 addresses flux density, polarization, field strength, demagnetization and recoil measurements. IEC 60404-18:2025 addresses open-circuit measurements using a superconducting magnet (SCM-VSM or extraction) with self-demagnetization correction; its scope presumes material properties homogeneous throughout the tested volume. The test method and applicability must be recorded; neither standard alone qualifies a spatially heterogeneous manufactured component.

## 7. Metrics — keep independent dimensions

### Functional performance
- vector field MAE / RMSE / max / p95 on exact matched grids;
- direction and polarity error, pole-boundary displacement;
- working-gap field, field uniformity and leakage;
- field gradient, force, torque, cogging or encoder phase where relevant;
- thermal/demagnetization margin and application response.

### Material and microstructure
- Br / J / Hcj / (BH)max with units, temperature, specimen and test method;
- orientation/texture distribution and method-specific alignment metric;
- porosity, density, filler loading and spatial heterogeneity;
- phase retention, oxidation/contamination and powder history where relevant;
- mechanical strength, modulus, fatigue, impact and environmental durability.

### Manufacturing
- dimensional accuracy, shrinkage, surface roughness, minimum feature and interface resolution;
- build time, throughput, process stability and repeatability;
- first-pass accepted-part yield, scrap/rework and failure modes;
- energy per build and per accepted part, material utilization and recycled fraction.

### Economics and lifecycle
- accepted-part cost, not feedstock cost alone;
- equipment utilization, post-processing, inspection and calibration time;
- lifecycle energy/material use, recycling/recovery route and end-of-life separability.

### Model and decision quality
- held-out lot/build prediction error and uncertainty coverage;
- OOD detection and abstention;
- false-safe / false-fail acceptance decisions;
- physical observations and compute per accepted component;
- realized decision regret at equal experiment and compute budgets.

Do not publish a hidden weighted composite score. Use hard constraints plus a transparent multiobjective Pareto frontier. Where policy-makers require a scalar, publish it as an application-specific downstream preference, not as physical truth.

## 8. Baseline tournament

For every application and route stratum, compare:
1. conventional sintered / segmented / assembled reference;
2. bonded MEX baseline;
3. LPBF/SLM baseline where material and process are admissible;
4. SLS / binder jet baseline where admissible;
5. LCM + field alignment + post-cure where admissible;
6. route-specific conventional geometry/parameter optimization;
7. route-specific process-aware inverse design;
8. as-built, evidence-gated route selection.

The optimizer leaderboard is separate from the manufacturing-route leaderboard. Compare against DOE/factorial, space-filling, response-surface, Bayesian and evolutionary approaches with matched experiment and compute budgets.

## 9. Active learning and experiment selection

The experiment planner should rank a next specimen or measurement by its expected effect on an explicit engineering decision, subject to:
- process compatibility and facility limits;
- cost, time, compute and specimen budget;
- calibration and blind-holdout protection;
- risk of confounding route, lot and build effects;
- expected reduction in uncertainty over the target application Pareto frontier.

Compare active selection to random, Latin-hypercube/space-filling, factorial and conventional expert-designed tests at equal budgets. Report outcome per build and per measurement. A claim of fewer experiments is credible only at equivalent false-safe rate and held-out predictive performance.

## 10. Staged benchmark ladder

- **R0 — literature/source ledger:** freeze article versions, source excerpts, raw-data availability and exact measurands. Preserve unresolved DOI/table-text discrepancies.
- **R1 — synthetic/analytical controls:** verify unit semantics, evidence-role isolation, exact-grid comparison, and expected metric calculations.
- **R2 — common target model challenge:** route-specific modelling for one target geometry with independently checked analytical or numerical references where applicable.
- **R3 — repeated physical route specimens:** calibrated measurements, as-built geometry/material characterization and build/lot holdouts.
- **R4 — equal-budget route and experiment tournament:** active design against DOE, space-filling and incumbent baselines.
- **R5 — application demonstration:** component function, operating environment, durability, accepted-part yield, cost and lifecycle evidence.

## 11. Current numeric targets and claim discipline

Do not freeze a universal remanence or accuracy target. First select the application and metrology chain, establish its repeatability and uncertainty, and determine which route strata can make comparable specimens.

The first correctness gates are:
- no false-safe acceptance on a preregistered sealed challenge set;
- 100% specimen/material/build/measurement identity closure for QUALIFIED outcomes;
- zero calibration/holdout leakage;
- uncertainty widening cannot improve a pass margin;
- route superiority only after matched, independently measured components;
- report all failed specimens/builds and route-specific applicability limits.

Aspirational efficiency improvements (for example fewer physical measurements) remain hypotheses until measured against equal-budget baselines.

## 12. Source register

1. *State-of-the-art additive manufacturing of NdFeB permanent magnets: challenges and pathways toward high performance*, Journal of Science: Advanced Materials and Devices 11(3), September 2026, DOI [10.1016/j.jsamd.2026.101226](https://doi.org/10.1016/j.jsamd.2026.101226). Review-level cross-study synthesis; reported route ranges are not matched test values.
2. *Bonded Nd-Fe-B magnets: Materials, processing, properties and outlook*, Journal of Rare Earths, online 3 March 2026, DOI [10.1016/j.jre.2026.03.002](https://doi.org/10.1016/j.jre.2026.03.002). Binder/filler/loading/processability trade-off.
3. Schäfer et al. (2026), recycled EOL hot-deformed NdFeB / PA12 LPBF, DOI [10.1002/aesr.202500339](https://doi.org/10.1002/aesr.202500339); see Symthaea #13506 and Symtropy #7296. Raw supporting data available from the authors on request. Keep the HD powder D90 276 μm / 279 μm source discrepancy unresolved.
4. Behera, Lv & Singamneni (2026), patterned multi-material magnetic poles by SLS, DOI [10.1038/s41598-026-43131-5](https://doi.org/10.1038/s41598-026-43131-5). Distinguish build field, post-magnetization field and measured probe field.
5. Kim et al. (2025), field-aligned LCM + EBPC, DOI [10.1080/17452759.2025.2593543](https://doi.org/10.1080/17452759.2025.2593543). Reconcile the publisher dataset DOI and candidate Zenodo record before ingesting.
6. Rathfelder et al. (2025), recycled NdFeB powder extrusion moulding, DOI [10.1038/s41598-025-19111-6](https://doi.org/10.1038/s41598-025-19111-6). Near-net-shape adjacent route, not a matched LPBF/MEX comparator.
7. IEC 60404-5:2015 and IEC 60404-18:2025. Standards define particular hard-magnet property methods; they do not remove the need to report specimen, homogeneity assumptions and uncertainty.

## 13. Safety and authority

This protocol creates no machine-control or physical test authority. Powder handling, laser processing, magnetization, energized-field experiments, destructive exposure, mechanical testing and thermal testing require a suitably qualified facility, risk assessment, calibrated instrumentation, approved procedures and independent authorization. No product-release, standards-conformity or safety claim follows from a software benchmark.
