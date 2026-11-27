# As-built magnetic functional acceptance protocol

**Protocol ID:** MAG-ASBUILT-FUNCTION-QUAL-001  
**Issue:** [Symtropy #7745](https://github.com/Luminous-Dynamics/symtropy/issues/7745)  
**Revision:** 0.1-draft  
**Date:** 27 November 2026  
**Status:** protocol and synthetic semantic fixtures; no solver or physical result claimed

## 1. Decision question

Does this exact manufactured specimen meet this exact target magnetic function over the declared operating envelope?

The object being qualified is not the nominal CAD design, a material datasheet, a simulation, or an isolated field probe reading. It is a specimen identified through its material lot and manufacturing history, with independently observed as-built geometry and material state, a declared post-build magnetization history, a qualified prediction, and a physical functional measurement.

The principal paradigm shift is from **nominal-design qualification** to **evidence-joined as-built functional qualification**.

## 2. Ownership and composition

This programme consumes rather than recreates existing capabilities.

| Concern | Owner / existing work | Adapter responsibility |
|---|---|---|
| Material/process science | Symthaea #13506, #13521, #13533 | Preserve material-state scope and evidence role |
| Process and build scenario | Symtropy #7296, #7310 | Join specimen/build/recipe/process identities |
| Manufacturing genealogy | Symtropy #2933 | Reference existing genealogy and receipts |
| Field-map comparison | Symtropy #7332 | Consume exact-grid comparison output |
| Physical model discrepancy | Symtropy #7667, #7730 | Preserve model-discrepancy and transfer evidence |
| Continuous-domain margin | Symtropy #7639 | Consume a complete, scope-matched spatial disposition |
| Solver correctness | Symtropy #7444 and external solver owners | Require admissible solver qualification; do not solve fields here |

No second magnetic ontology, component catalog, material database, event ledger, field solver, DEM solver, micromagnetic solver, calibration service or actuation authority is introduced.

## 3. Specimen evidence graph

Each accepted specimen must form one closed chain:

1. **Target requirement.** Immutable requirement ID and revision; measurand; units; reference frame; field/gradient/force/torque quantity; spatial domain; operating conditions; hard limits; acceptance-policy digest.
2. **Design intent.** Nominal CAD/geometry digest, material zoning, local easy-axis or magnetization intent, target field map identity and design revision.
3. **Material lineage.** Source material lot, feedstock route and lot, composition/grade, particle-size and morphology evidence where relevant, and the material-property evidence actually used downstream.
4. **Manufacturing genealogy.** Unique specimen ID, build/run, machine identity, executed recipe identity, layer/scan/deposition or moulding history, relevant process-event receipts, and deviations/nonconformances. Reuse the existing manufacturing digital thread.
5. **Post-build magnetization.** Distinct operation identity; magnetizer/fixture identity; field-vector map or declared field envelope; orientation; waveform/time; temperature; sequence; and any demagnetizing or stabilization steps. An in-process alignment field is not the post-build magnetizing field.
6. **As-built geometry.** Independently inspected geometry digest, coordinate frame, dimensional deviations, inspection instrument and calibration chain, uncertainty, and exact specimen binding.
7. **As-built material state.** Measured or explicitly modelled porosity/density, texture/orientation, hysteresis/demagnetization, remanence/coercivity and temperature dependence as applicable. Distinguish specimen measurements from witness coupons and lot-level evidence; a coupon is not automatically the specimen.
8. **Qualified solver prediction.** Solver package/version/build, formulation, material constitutive model, mesh/refinement, boundary conditions, exact as-built geometry/material/magnetization input digests, environment and output digest. A convergence claim is not a physical validation claim.
9. **Physical function observation.** Exact specimen; probe/sensor identity and calibration; independent angular/force/torque reference as applicable; coordinate frame and grid; standoff; background-field treatment; magnetization state; temperature; acquisition timing; operating conditions; raw data digest and reduction-code digest.
10. **Comparison and spatial qualification.** Exact-grid field/function comparison, uncertainty/coverage evidence, continuous-domain margin certificate, and complete domain-partition identity.
11. **Disposition.** Policy-bound per-requirement outcomes and specimen-level status, with every upstream record preserved by immutable reference.

## 4. Identity and scope closure

The adapter checks joins; it does not create upstream evidence.

Required joins include:

- specimen ID is identical across manufacturing, geometry, material-state, magnetization, solver, measurement and acceptance records;
- source lot and build/run IDs match the manufacturing genealogy;
- as-built geometry digest used by the solver is the inspected geometry digest, or an explicit, qualified geometry reconstruction/registration transform is supplied;
- solver material-state input refers to the admitted material evidence or a declared model transformation with its own qualification;
- magnetization history is tied to the specimen and matches the state at prediction and measurement;
- prediction and observation have the same target requirement, operating condition, frame and exact grid, or use an explicit independently qualified transformation;
- the comparison receipt binds the exact prediction and physical observation digests;
- spatial certificates bind the same specimen, target requirement, geometry/material state, model set, operating condition and domain partition;
- all calibration, registration, temperature and common-cause uncertainty references are retained.

A digest is content identity, not proof of authorship, instrument calibration, physical truth or process execution. The adapter must not infer a missing record from a filename, branch, nominal design, simulation output or nearby specimen.

## 5. Evidence roles

Keep these roles distinct:

- **DESIGN_INTENT:** target and nominal design; never an observation.
- **PREDICTED:** simulation, surrogate, reduced-order model or process prediction.
- **CALIBRATION:** data used to fit, tune, register or calibrate a model/instrument.
- **PHYSICAL_OBSERVATION:** raw measurement of the identified specimen under declared conditions.
- **INDEPENDENT_REFERENCE:** independent calibrated reference measurement or chain.
- **BLIND_HOLDOUT:** sealed observation excluded from fitting, calibration, active test selection and threshold selection.
- **PROCESS_RECORD:** executed machine/process event evidence, with machine and recipe identity.

A model output cannot be relabelled as a physical observation. Calibration observations cannot be reused as blind holdout. A witness coupon or source-paper scalar cannot silently become specimen-level ground truth. A published result remains literature evidence unless independently reproduced.

## 6. Measurement semantics

Magnetic quantities that sound similar are not interchangeable:

- intrinsic remanence, polarization, coercivity and demagnetization-curve properties;
- mass-specific magnetization in A·m²/kg and volumetric quantities in T;
- surface probe field and a component's useful working-gap field;
- field measured with an external bias field present and zero-background remanent field;
- in-process alignment field and post-build magnetizing field;
- field-map error, phase/polarity error, gradient, force and torque.

Every physical map records its exact frame, grid, standoff, probe, calibration/reference chain, temperature, background-field correction/subtraction, specimen magnetization state and acquisition interval. No contour image or paper-reported scalar is treated as a pointwise 3D truth map.

For hard-magnet material-property tests, IEC 60404-5:2015 describes measurement of flux density, polarization, field strength and demagnetization/recoil curves in a closed magnetic circuit. IEC 60404-18:2025 describes open-circuit DC measurement using a superconducting magnet (SCM-VSM or extraction) with self-demagnetization correction. These are method references only; applicability depends on the actual specimen and method, and this protocol makes no conformity claim.

## 7. Dispositions

Per-requirement results are retained; the aggregate specimen status is non-compensatory.

- **QUALIFIED:** every required item is present, admissible and scope-matched; every hard requirement is safe; comparison uncertainty is covered by the frozen policy; the continuous-domain certificate covers the complete domain and is safe; no required result is blind-holdout leakage or out-of-domain.
- **FAIL:** at least one in-scope, admissible hard requirement is definitely violated under the frozen acceptance rule. Other incomplete requirements remain reported; they do not erase a demonstrated failure.
- **NOT_EVALUABLE:** the required measurand, physical regime, material model, solver formulation or measurement method is outside declared support. This is not a failure of the physical part and is never a pass.
- **INDETERMINATE:** missing/contradictory identities, unresolved uncertainty, incomplete map/domain coverage, unqualified model discrepancy, insufficient evidence role or unresolved spatial margin prevents a decision.

An unsupported claim cannot upgrade INDETERMINATE or NOT_EVALUABLE to QUALIFIED. A weighted objective cannot trade away a hard limit.

## 8. Uncertainty and common cause

Report separate contributions and the dependence model. Do not assume independent errors merely because they appear in different files.

At minimum consider:
- reference and probe calibration;
- probe position, standoff, orientation and frame registration;
- temperature and thermal history;
- specimen/fixture deformation and shaft/runout where applicable;
- shared lot, build, machine, recipe and post-magnetization effects;
- spatial interpolation, quadrature and unsampled-domain uncertainty;
- solver discretization, material constitutive uncertainty and physical-model discrepancy.

Use a qualified covariance, interval, affine/zonotope or other validated representation only within its admitted assumptions. If dependence is unknown, preserve that uncertainty or return INDETERMINATE. Widening an uncertainty envelope must not improve a margin or upgrade a disposition.

## 9. Benchmark ladder

### A0 — contract and provenance

Freeze protocol, requirements, source register, schemas, measurement definitions, and unknowns. Public scalar results can be transcribed with their exact measurand/source limitations; missing raw data remain missing.

### A1 — synthetic semantic and adversarial controls

Known-answer cases cover:
- complete and scope-matched safe chain;
- missing or duplicated specimen/process identities;
- nominal-versus-as-built geometry mismatch;
- material evidence from an unrelated lot or coupon presented as specimen truth;
- missing or mismatched post-build magnetization history;
- solver output built from nominal rather than as-built state;
- field map with wrong frame/grid/standoff/background conditions;
- calibration/holdout role leakage;
- common-cause uncertainty omitted or widened;
- partial spatial coverage or overlapping/gapped domain partitions;
- definitely violated hard limit;
- unsupported material/measurement regime.

These cases test the acceptance contract only. They are not synthetic evidence of material or solver performance.

### A2 — analytical references

Use exact analytical permanent-magnet field references for uniformly prescribed magnetization (e.g. finite cuboids) and independent implementations. Magpylib is useful for fast analytical magnetostatic calculations of common magnet/current geometries. Its own documentation distinguishes cases where analytical assumptions are exact from material-response regimes; do not use it as a general nonlinear hysteretic FEM replacement.

### A3 — numerical solver verification

Use public benchmark problems such as TEAM Workshop Problem 13 for three-dimensional nonlinear magnetostatics, with stated B-H data and measurement comparison. Add independent solver implementations and ordered mesh/refinement studies. Keep solver verification separate from validation against a physical manufactured specimen.

Commercial comparator capabilities to record include Ansys Maxwell's 2D/3D electromagnetic solvers, adaptive meshing, material models, multiphysics and optimization workflows, and COMSOL AC/DC's static/transient/frequency-domain electromagnetic and multiphysics capabilities. Do not claim the adapter is a better field solver.

### A4 — held-out process/material prediction

Use repeated builds and lots with measured geometry, density/porosity, orientation/texture and magnetic properties. Split by physical build/lot, not random rows from the same build. Predict held-out cases without using their inspection results in design, fitting or active selection. Report calibration and OOD/abstention.

### A5 — physical component qualification

Use repeated physical components with independently calibrated field maps and application response. Blind whole spatial sectors, operating conditions and at least one build/lot. Measure yield, durability, thermal behaviour and demagnetization margins using safe, approved facilities.

## 10. Baseline tournament

For specimen acceptance, compare these methods on identical specimens, data access, measurement budgets and hard limits:

1. nominal-CAD-only prediction;
2. process-blind nominal material-card solver;
3. process-aware model without as-built inspection;
4. process-aware model with measured as-built geometry and material state;
5. uniform-grid physical inspection;
6. decision-directed active metrology;
7. proposed evidence-gated as-built acceptance.

For design optimization, separately compare fixed-direction parameter/shape optimization, magnetization-direction/adjoint methods, commercial FEM topology optimization, DOE/response-surface methods, Bayesian optimization and evolutionary multiobjective methods. The 2023 gradient-based permanent-magnet assembly work and current commercial optimization offerings are relevant incumbents; applying an AI or optimizer alone is not a novelty claim.

The 2026 near-real-time full-wave inverse-design Green-function method and differentiable electromagnetic geometry optimization are adjacent SOTA methods, not direct low-frequency permanent-magnet comparators. Any transfer must demonstrate the relevant governing equations, material regime, boundary conditions and manufacturing constraints.

## 11. Metrics

Keep independent dimensions; do not publish a hidden aggregate winner score.

**Evidence / assurance**
- chain-closure fraction and exact-match rate;
- unsupported, duplicated, stale and scope-mismatched evidence count;
- role/holdout leakage count;
- false-safe and false-fail rate;
- qualified / failed / indeterminate / not-evaluable specimen count.

**Physical / functional**
- vector field MAE, RMSE, max and p95;
- direction error and polarity-boundary displacement;
- field gradient and leakage;
- force/torque and application response where relevant;
- certified-safe / failing / unresolved spatial volume;
- measured uncertainty coverage and repeatability.

**Manufacturing**
- as-built dimensional deviation and porosity/density;
- local material and texture variation;
- lot/build variance and process-window sensitivity;
- first-pass yield, accepted-part cost, scrap/rework, material utilization and process energy.

**Efficiency**
- physical observations and acquisition time per accepted specimen;
- compute time/memory per qualified candidate;
- experiment-selection benefit at equal experiment and compute budgets.

## 12. Initial targets and comparison rules

Correctness gates:
- zero false-safe classifications on a preregistered sealed challenge set;
- 100% evidence identity/scope closure for every QUALIFIED disposition;
- zero calibration-to-holdout leakage;
- no status improvement when relevant uncertainty is widened;
- exact as-built geometry/material/magnetization identity for every qualified solver comparison;
- complete continuous-domain coverage for any claim of spatially universal safety.

Performance hypotheses, not current achievements:
- at least 2× fewer physical observations than uniform-grid inspection at the same false-safe rate;
- improved held-out prediction and first-pass yield versus process-blind baselines;
- improved functional performance per unit cost/material/process energy;
- improved build/lot generalization.

Only freeze numeric performance gates after the measurement floor and repeated-build variance are known. Compare matched targets, common data access, identical compute/experiment budgets and sealed holdouts. Report each metric independently.

## 13. Source register and comparator limitations

- Schäfer et al. (2026), recycled EOL hot-deformed Nd–Fe–B / PA12 LPBF, DOI [10.1002/aesr.202500339](https://doi.org/10.1002/aesr.202500339). The published HD-derived and milled routes report different porosity, remanence and alignment. Supporting data are available from the authors on request; these scalar values are literature evidence, not a local reproduction or a 3D truth map. The source's HD-powder D90 is reported as 276 μm in Table 2 and 279 μm in discussion; preserve the discrepancy.
- Behera, Lv & Singamneni (2026), patterned magnetic poles by SLS, DOI [10.1038/s41598-026-43131-5](https://doi.org/10.1038/s41598-026-43131-5). The reported in-process field, post-magnetization field and probe-field contours have distinct meanings; raw point maps and conditions are needed for a matched reproduction.
- Kim et al. (2025), field-aligned lithography-based composite manufacturing and electron-beam post-cure, DOI [10.1080/17452759.2025.2593543](https://doi.org/10.1080/17452759.2025.2593543). Keep mass-specific and volumetric units distinct; reconcile the publisher data DOI before ingesting any candidate dataset.
- Rathfelder et al. (2025), recycled NdFeB powder extrusion moulding, DOI [10.1038/s41598-025-19111-6](https://doi.org/10.1038/s41598-025-19111-6). Sintered PEM is a separate manufacturing stratum, not a direct LPBF/PA12 comparator.
- Wolz et al. (2025), high-coercivity HRE-free NdFeB by LPBF, DOI [10.1016/j.jmmm.2025.173363](https://doi.org/10.1016/j.jmmm.2025.173363). Distinct feedstock/chemistry/process route; use as an AM capability reference, not a like-for-like recycled powder baseline.
- NIST OOMMF, [Computational Micromagnetics](https://www.nist.gov/programs-projects/computational-micromagnetics), is an open, documented environment for static/dynamic micromagnetic problems and algorithm comparison; it addresses a different scale/regime than bulk bonded-magnet process twins.
- TEAM Workshop Problem 13, [benchmark solution paper](https://aces-society.org/includes/downloadpaper.php?nf=d244321ad5d8d4bf402cdd85ec419b6c&of=1993W12), supplies a 3D nonlinear magnetostatic comparison with measurement data.
- Magpylib, [physics/computation documentation](https://magpylib.readthedocs.io/en/6.1.0/_pages/user_guide/guide_resources_01_physics.html), documents its analytical field families.
- Ansys Maxwell, [official solver overview](https://ansys.synopsys.com/products/electronics/ansys-maxwell), and COMSOL AC/DC, [official capability overview](https://www.comsol.com/acdc-module), are commercial solver/multiphysics comparators.
- Sun et al. (2026), near-real-time full-wave inverse design, DOI [10.1038/s41467-026-69477-y](https://doi.org/10.1038/s41467-026-69477-y); Yang et al. (2026), differentiable electromagnetic geometry optimization, DOI [10.1038/s44172-026-00642-3](https://doi.org/10.1038/s44172-026-00642-3). These are adjacent inverse-design methods, not directly comparable permanent-magnet low-frequency solvers.

## 14. Physical authority and safety

This protocol grants no authority to energize a coil, run a magnetizer, operate an additive-manufacturing machine, handle reactive powder, execute a recipe, or expose a magnet to destructive fields. Physical trials require qualified facilities, hazard review, instrument calibration, safe fixtures, independent authorization and defined emergency procedures.

No product-release, standards-conformity, certification or safety claim follows from a software disposition.
