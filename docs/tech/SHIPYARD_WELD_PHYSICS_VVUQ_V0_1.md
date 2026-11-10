# Shipyard Weld Physics VVUQ Benchmark v0.1

Tracking: [SHIPYARD-WELD-PHYSICS-VVUQ-001 #4614](https://github.com/Luminous-Dynamics/symtropy/issues/4614)

## Scope

This benchmark qualifies physical predictions for multipass welding separately from process-evidence semantics. It asks whether a declared model, on a declared specimen/configuration and under admitted input/measurement uncertainty, predicts temperature history, fusion-zone geometry, residual stress and distortion.

It does not implement a welding controller or grant procedure, inspection, structural, class, flag or production acceptance.

## Why this is separate

- [#4433](https://github.com/Luminous-Dynamics/symtropy/issues/4433) verifies workpiece/equipment/procedure/pass identity and exact synthetic process traces.
- [#4443](https://github.com/Luminous-Dynamics/symtropy/issues/4443) owns defect disposition, repair/reinspection and feedback lineage.
- [#4500](https://github.com/Luminous-Dynamics/symtropy/issues/4500) owns NDT provider/method/personnel/coverage/report evidence.
- [#3331](https://github.com/Luminous-Dynamics/symtropy/issues/3331) owns component manufacturing, inspection and as-built qualification.
- [#2048](https://github.com/Luminous-Dynamics/symtropy/issues/2048) owns shipyard design-for-manufacture, block assembly and production planning.
- [#3121](https://github.com/Luminous-Dynamics/symtropy/issues/3121) and [#3164](https://github.com/Luminous-Dynamics/symtropy/issues/3164) own general manufacturing twin VVUQ and lifecycle requalification.

This benchmark owns physical model verification/validation, source admission, measurement uncertainty and scale-transfer evaluation. It consumes process and component identity from the above owners; it does not duplicate them.

## Research finding

The open-access review published 7 November 2026, *A review of prediction and control methods on welding-induced distortions in large metallic structures* (DOI 10.1007/s40194-026-02695-6), highlights a scale-transfer problem: detailed thermo-elasto-plastic FEM is computationally expensive; inherent-strain methods are efficient but calibration-dependent; local/global and surrogate approaches trade accuracy, assumptions and applicability. Its review also identifies limited literature coverage for large aluminium and high-strength-steel structures.

This gives us a stronger objective than building a single high-fidelity solver: qualify a hierarchy of models and quantify where each one stops being trustworthy.

## Model hierarchy to compare

1. Analytical thermal checks: Rosenthal-type far-field behaviour, heat balance, dimensional consistency and manufactured solutions. These are verification references, not fusion-zone truth.
2. Transient moving-heat-source thermal models: declared source form and normalization; compare against independently measured temperature histories and macrographs.
3. Sequentially coupled thermo-elasto-plastic models: temperature-dependent properties, pass activation/deposition, boundary conditions, restraints, cooling and release; compare measured displacement and residual-stress profiles.
4. Inherent-strain / local-global methods: calibrate on a declared local specimen set; validate on held-out structural cases.
5. Reduced-order / surrogate models: report training support, calibration lineage, uncertainty, out-of-domain detection and abstention.
6. Hybrid physics-plus-data correction: a separate comparison arm; it cannot tune against its own held-out specimen or conceal systematic model bias.

No model family is presumed best. Compare predictive error, uncertainty coverage, compute cost, calibration burden, transfer performance and worst-stratum behaviour together.

## Reference-source inventory

The v0.1 inventory deliberately distinguishes discoverable literature from admissible numerical datasets.

| Candidate | Known evidence | Initial state | Admission requirement |
|---|---|---|---|
| NAFEMS MANWG Weld Validation Data Review | Literature index of weld experiments and available temperature, residual-stress, distortion and procedure metadata | METADATA_ONLY | Source bytes, member access, usage rights and selected experimental publication |
| NeT Task Group 4 | Three-pass AISI 316L(N) slot-weld residual-stress measurement / analysis round robin | RIGHTS_PENDING | Exact report/data artifacts, method uncertainty and reuse rights |
| NeT Task Group 6 | Three-pass Alloy 600 / Alloy 82 repair-weld round robin, multiple neutron-diffraction instruments | RIGHTS_PENDING | Exact report/data artifacts and reuse rights |
| Q235 55 mm K-groove, 12-pass study (2026) | Blind-hole residual-stress measurements and sequentially coupled FE; article identifies CC BY 4.0 | METADATA_ONLY | Extracted values with attribution, uncertainty, coordinates and source-to-table audit |
| DH36 stiffened panel, 6 weld lines / 18 passes (2026) | Experimental thermal/mechanical response; free and constrained cases | METADATA_ONLY | Full source, raw/derived measurements, geometry and license |
| 13-pass thick-plate slanted-groove study (2026) | Temperature history, residual stress, deformation and hardness | METADATA_ONLY | Supplementary files, exact measurement definition and rights |
| TWI 32 mm-wall / 910 mm-diameter instrumented pipe weld | Temperature, strain, displacement and post-weld residual-stress measurements | METADATA_ONLY | Underlying measurements, model inputs and reuse permission |
| Curved ship/offshore welded plate study (2026) | Residual-stress model and geometry application | METADATA_ONLY | Exact measured profiles, specimen geometry and model inputs |

A metadata-only source may guide research and define a candidate case, but it cannot contribute numerical scores. A paper's reported result is not automatically a raw measurement file. Do not manufacture missing inputs from figures or abstracts.

## Verification, calibration, validation and UQ

Keep these evidence classes distinct:

- Code verification: analytic limits, manufactured solutions, dimensional checks and independent numerical implementations.
- Solution verification: mesh/time refinement, convergence, solver tolerances, heat/energy balance and activation/contact sensitivity.
- Calibration: fit heat-source, material or boundary parameters only on an explicitly named calibration partition.
- Validation: freeze the model, then compare with independent measurements never used for tuning.
- Measurement uncertainty: instrument, calibration, position, timing, resolution and reconstruction uncertainty.
- Input uncertainty: material properties, heat-source parameters, clamping, contact and boundary-condition uncertainty.
- Model-form uncertainty: residual discrepancy between model family and physical observations.
- Transfer validation: hold out entire specimens, welds, campaigns, geometries, materials or facilities. Never split points or image patches from the same weld across train and test.

## Benchmark stages

- Stage 0 — analytic / manufactured thermal verification and energy-balance checks.
- Stage 1 — transient temperature histories, cooling curves and fusion-zone geometry.
- Stage 2 — local thermo-mechanical displacement and residual-stress profiles.
- Stage 3 — multipass sequencing, multi-joint interaction, cooling and restraint release.
- Stage 4 — local-to-global transfer into stiffened panels, curved plates and blocks.
- Stage 5 — held-out transfer to new thickness, material, geometry and process strata, with calibrated uncertainty and explicit abstention.

A Stage 0–1 pass cannot be represented as Stage 2–5 qualification.

## Metrics

**Thermal:** time-history MAE/RMSE/bias, peak-temperature error, heating/cooling-rate error, spatial isotherm error, energy-balance residual and predictive-interval coverage.

**Fusion zone / geometry:** width, penetration and cross-section area error; HAZ dimensions only where independently measured; segmentation/registration uncertainty.

**Mechanical:** angular/transverse/longitudinal distortion after full cooling and restraint release; full-field geometry error; residual-stress profile error and peak bias; measurement-method uncertainty and equilibrium consistency.

**Efficiency and transfer:** wall-clock/CPU/memory under matched hardware and solver tolerances; speedup versus detailed pass-by-pass FE; number of calibration observations; held-out specimen/campaign/material/geometry error; interval coverage and proper scoring; OOD detection and abstention quality.

Do not set a universal temperature or distortion threshold before a reference's measurement uncertainty and engineering context are admitted.

## Falsification suite

The 26-case v0.1 contract corpus includes positive source-inventory controls and adversarial cases for rights, raw-vs-tabulated-vs-digitized evidence, missing inputs, specimen identity, geometry/material mismatch, restraints, units/coordinates/timing, uncertainty, calibration leakage, specimen-level splits, source conflicts, unsupported scale transfer, surrogate OOD behaviour, interval undercoverage, energy conservation and claim promotion.

The contract corpus is not a numerical model validation. Numerical validation begins only after at least one source has admitted measurements and source-to-data traceability.

## Initial targets — research aspirations, not achievements

- 100% independent oracle agreement on analytic/manufactured verification controls.
- Zero false admission of source-rights gaps, incomplete required inputs, specimen leakage or invalid calibration/validation separation.
- Report mesh/time convergence and energy-balance residuals for every numerical arm.
- Target nominal 90% predictive-interval coverage on adequately powered held-out specimen sets, with confidence intervals and per-stratum reporting.
- Set thermal and geometric error thresholds only after source measurement uncertainty and context of use are documented.
- Report error, uncertainty, compute, calibration burden and applicability envelope together.
- Target at least 10x speedup for an inherent-strain/local-global/surrogate arm only when a matched held-out large-structure comparison demonstrates the improvement without violating pre-registered error and coverage limits.
- No transfer claim outside the admitted material, joint, thickness, process and restraint domain.

## Implementation order

1. Freeze the source inventory and admission states.
2. Validate the schema and 26-case corpus independently of any model solver.
3. Add a dependency-light Rust validator for source identity, evidence class, rights, units, uncertainty and split integrity.
4. Reproduce one analytic thermal reference and one genuinely admitted measurement case.
5. Admit NeT and shipyard thick-plate/panel cases only after exact source bytes, data and rights are established.
6. Add detailed FE, inherent-strain, local/global and surrogate arms only under matched input and compute budgets.
7. Connect run identity and process trace references from #4433, NDT/repair evidence from #4443/#4500, and configuration/as-built lineage from #3331.

## Claim ceiling

A v0 manifest PASS establishes only source/contract/split integrity. A physical validation result applies only to the admitted specimen set, measurement methods, model revision, calibration partition, uncertainty and declared domain. It does not qualify a WPS/PQR, accept a physical weld, qualify NDT, certify structural integrity, grant class/flag approval or authorize production/release.
