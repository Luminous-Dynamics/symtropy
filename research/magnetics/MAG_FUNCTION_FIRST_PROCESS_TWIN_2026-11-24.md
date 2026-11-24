# MAG-FUNC-PROC-001 — Field-function process twin for spatially programmed magnets

**Revision:** 0.1.0  
**Prepared:** 2026-11-24  
**Status:** analytic reference and orchestration contract; no finite-magnet solver or physical process is claimed  
**Symtropy issue:** [MAG-FUNC-PROC-001 #7310](https://github.com/Luminous-Dynamics/symtropy/issues/7310)  
**Symthaea design owner:** [MAG-FUNC-001 #13521](https://github.com/Luminous-Dynamics/symthaea/issues/13521)

## Purpose

Create a reproducible process and solver-orchestration benchmark for magnetic components whose spatial field function is deliberately programmed through geometry, hard/soft material placement, local easy-axis orientation and magnetization history.

The central object is a target vector field map plus its application function, not a material-only scalar. The process twin must preserve the difference between:
- material zoning and physical component geometry;
- local particle orientation and final magnetization state;
- in-process alignment field and post-print magnetizing field;
- predicted field maps and measured probe observations;
- intrinsic magnet properties and surface fields under external/background field.

## Source-pinned motivation

Behera, Lv & Singamneni (2026), [Patterned magnetic pole configurations in bonded magnets](https://doi.org/10.1038/s41598-026-43131-5), demonstrate multi-material SLS of nylon composites with NdFeB and FeSi/FeCo regions and local in-build field orientation. The proof-of-concept used two magnetic islands, five 50 μm layers, a reported 230 mT process field and 1.5–1.9 T post-print magnetization. The paper reports selected-point surface probe fields and pole contours under stated external-field conditions; those are not intrinsic Br/Hcj.

The authors identify powder placement and external field control as sources of output variability and note patent-pending apparatus/process embodiments. The benchmark must not reproduce protected apparatus details or imply freedom to operate.

Kim et al. (2025), [LCM + field alignment + EBPC](https://doi.org/10.1080/17452759.2025.2593543), adds a separate process family for aligned bonded composites and improved matrix cure. Dataset DOI discrepancy is unresolved (publisher states 10.5281/zenodo.17404587; NLM catalog lists 10.5281/zenodo.17510759); do not ingest a dataset until authoritative identity is verified.

## Process-twin responsibilities

Symtropy owns exact geometry and region identity, process scenario/layer sequence/local material placement, time- and position-dependent in-process field scenarios as modelled inputs, post-print magnetization history as a separate operation, solver identity/input closure/replay and sensitivity/discrepancy reporting.

Symthaea owns target function and requirement semantics, material/source/evidence identity, candidate/topology selection, uncertainty/Pareto comparison, experiment selection and scientific evidence interpretation.

Use existing geometry, catalog, magnetic, FIELD/metrology, manufacturing digital thread, solver and V&V owners. Do not create duplicates.

## Analytic reference — exact for point dipoles only

The companion fixture at research/magnetics/fixtures/magnetic_function_field_reference_cases_v0_1.json defines SI point-dipole cases:

B(r) = (mu0 / 4pi) * (3 * (m dot r) * r / |r|^5 - m / |r|^3)

with mu0 / 4pi = 1e-7 T m / A.

Known-answer cases test axial/equatorial signs and magnitudes, two-source superposition and symmetry, linearity in moment, inverse-cube geometric scaling, global moment reversal, rotation covariance and rejection of a query at the point-dipole singularity.

The formula is exact for ideal point dipoles in free space. It is not a finite magnet solution and does not model nonlinear B-H, demagnetization, hysteresis, eddy currents, temperature, stress or manufacturing. It is a coordinate/unit/sign/superposition oracle and a far-field sanity reference only.

## Finite geometry and material solver boundary

For real components, compose qualified external solvers for magnetostatic finite-element/boundary-element physics, nonlinear B-H, permanent-magnet recoil/demagnetization, hysteresis, temperature dependence, eddy-current/frequency response and relevant thermal/mechanical process effects.

Every solver request must bind exact geometry, material state/property evidence, coordinate frames, boundary conditions, mesh/discretization, solver version, execution context, convergence and output receipts. A converged numerical run is not a measured field or qualified component.

## Process and observation contract

For every manufactured or predicted case, preserve:
- material region geometry, grade, lot and volume fraction;
- local easy-axis/orientation distribution and evidence class;
- layer deposition sequence, recipe and machine identity;
- in-build field map, orientation, timing and uncertainty;
- post-print magnetization history and field;
- external/background field during measurement;
- probe type, calibration, orientation, bandwidth, standoff and reference grid;
- background/control measurement and subtraction rule;
- temperature, geometry, date and uncertainty.

An applied magnetizing field of 1.9 T, an in-process field of 230 mT, and an observed surface field of 10 mT are different quantities and must never share one untyped field.

## Benchmark ladder

- P0 — source and target map contract; exact coordinate frame and measurement conditions.
- P1 — point-dipole analytic references and metamorphic controls.
- P2 — reproduce published two-region hard/soft patterns if raw maps and apparatus metadata are acquired with appropriate permission.
- P3 — compare predicted and measured field maps on held-out geometries and material/process lots.
- P4 — compare integrated multi-material design against a conventional assembled segmented magnet at the same target field function, material budget and application boundary.
- P5 — qualify a named component through force/torque, losses, thermal, mechanical and lifecycle measurements.

## Metrics

- vector field-map RMSE/MAE and uncertainty coverage over a frozen 3D grid;
- target field correlation and polarity-boundary localization;
- peak field and gradient error;
- leakage under declared external/background conditions;
- force/torque and ripple where applicable;
- material-region placement and easy-axis orientation error;
- component mass/volume, interface count, assembly steps and tolerance stack;
- process yield, repeatability, accepted-part cost and energy;
- exact run replay, solver input closure and OOD/abstention.

Keep all metrics independent. Do not collapse them into one scalar score.

## Known source limitations and research hazards

- Behera et al. raw data are available from the corresponding author on request. Published contours cannot be digitized into a purported exact 3D truth map.
- LCM/EBPC dataset DOI is conflicted and must be resolved before ingestion.
- The 2026 multi-material SLS article reports patent-pending apparatus and material-delivery solutions; do independent IP review.
- Surface probe fields under an external field are not remanence or intrinsic coercivity.
- Point-dipole analytic references cannot validate finite-magnet or nonlinear-material predictions.
- Physical testing and magnetization require independent facilities and safety authorization.

## Claim ceiling

This tranche provides analytic known-answer data and process-twin contracts only. No solver execution, finite-element validation, physical prototype, magnetic measurement, patent clearance, SOTA result or deployment qualification is claimed.

## References

1. Behera et al. (2026), [Patterned magnetic pole configurations in bonded magnets](https://doi.org/10.1038/s41598-026-43131-5).
2. Kim et al. (2025), [Magnetic alignment and electron beam post-curing](https://doi.org/10.1080/17452759.2025.2593543).
3. Symthaea [MAG-FUNC-001 #13521](https://github.com/Luminous-Dynamics/symthaea/issues/13521).
4. Symthaea [MAG-AM-001 #13506](https://github.com/Luminous-Dynamics/symthaea/issues/13506).
