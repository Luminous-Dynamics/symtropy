# MAG-AM-PROC-001 — LPBF particle packing and magnetic alignment process twin

**Revision:** 0.1.0  
**Prepared:** 2026-11-24  
**Status:** process-twin contract and literature baseline; no solver or physical experiment is claimed  
**Symtropy issue:** [MAG-AM-PROC-001 #7296](https://github.com/Luminous-Dynamics/symtropy/issues/7296)  
**Symthaea science owner:** [MAG-AM-001 #13506](https://github.com/Luminous-Dynamics/symthaea/issues/13506)

## Purpose

Build a reproducible process-twin benchmark for recycled anisotropic Nd–Fe–B / PA12 bonded magnets fabricated by laser powder-bed fusion (LPBF).

The process twin must connect measured feedstock distributions and process histories to predicted powder packing, particle orientation, porosity and realized texture. It must then compare those predictions with independent microscopy, density/porosity, magnetization and field-map evidence.

The ambition is not to add an unverified magnetic simulator to Symtropy. It is to create a trusted orchestration and qualification boundary around the appropriate physical models.

## Source-pinned baseline

Schäfer et al. (5 January 2026), DOI [10.1002/aesr.202500339](https://doi.org/10.1002/aesr.202500339), report LPBF-printed recycled EOL hot-deformed Nd–Fe–B powder in low-CO2 PA12.

| Printed route | Porosity | Remanence | Coercivity | Alignment degree |
|---|---:|---:|---:|---:|
| HD-derived feedstock | 32.6% | 0.34 T | 1,238 kA/m | 0.75 |
| Milled feedstock | 28.3% | 0.32 T | 1,226 kA/m | 0.65 |

Published settings include 177.5 °C powder-surface temperature, 140 °C chamber temperature, 0.4 mm laser spot, 0.4 mm hatch and 150 μm layer height. The article reports a 5 mm cube characterization coupon and nominal 90 wt% magnetic filler. These are source-specific settings, not a transferable process recipe.

The article's table reports HD powder D90 = 276 μm while its discussion says 279 μm. The source data register retains both values and marks the difference unresolved. Particle-scale simulation must not silently select one as an exact distribution datum.

The authors identify PSD blending, layer-height optimization and homogeneous magnetic-field assistance as future research directions. They did not demonstrate that field assistance improved this specific LPBF process; field-assisted cases are prospective hypotheses.

## Role separation

Symthaea owns:
- material and sample identity;
- intrinsic vs extrinsic magnetic evidence;
- target visibility, contamination control and source custody;
- material-property prediction and experiment selection;
- process→structure→property→performance evidence interpretation.

Symtropy owns:
- physical process-scenario composition;
- deterministic scenario replay and solver-run identity;
- external model adapters and capability declarations;
- parameter sensitivity, process-window analysis and discrepancy reporting;
- build genealogy and evidence references composed with the manufacturing digital thread.

External numerical solvers own the physics they are qualified to solve. Keep solver outputs typed as predictions. Field maps, porosity, texture and hysteresis measured on specimens remain independent observations.

## Physical model hierarchy

### P0 — process and evidence identity

Before numerical prediction, represent:
- powder lot and EOL article/rotor provenance;
- powderization route and process history;
- full PSD and morphology distributions, with instrument/method identity;
- PA12 grade/lot and mixing process;
- machine identity, calibration state and build configuration;
- recipe and exact process parameter set;
- coupon geometry, build orientation and build position;
- observation instruments, calibration, coordinate frame and uncertainty.

A recipe document is not an executed build. A model input is not a process observation.

### P1 — particle packing and contact mechanics

Use a qualified discrete-element or particle-contact model to evaluate:
- measured particle-size and aspect-ratio distributions;
- shape-sensitive packing and orientation;
- contact/friction/cohesion assumptions;
- particle/binder interaction where supported;
- build-layer and deposition boundary conditions;
- packing fraction and porosity spatial distribution.

A spherical-particle approximation may be useful as a deliberately weaker baseline, but it cannot stand in for measured elongated-particle morphology when the research question is anisotropic orientation. State every approximation.

### P2 — magnetic orientation and field coupling

Where physically appropriate, include particle magnetic moment/easy-axis evidence, torque, inter-particle interactions and the spatial/time-dependent applied field.

For a verified fixed-dipole approximation, torque can be written as τ = m × B. That relation is not by itself a complete constitutive model for anisotropic, interacting, multi-domain or temperature-dependent particles. Use the correct material/magnetic model and its measured applicability.

The source paper's LPBF baseline did not use an externally applied magnetic alignment field. Field-assisted deposition is a separate prospective case with its own apparatus identity, calibrated field map, coordinate frame, timing, uncertainty and sham/field-off control. No Symtropy output may command or energize a coil.

### P3 — thermal and binder/process coupling

Represent temperature history, binder state, layer time, local process exposure and machine conditions only to the level supported by calibrated process data and a qualified model. Do not assume a powder-surface temperature is the full particle temperature history. Do not infer polymer rheology, degradation or bonding quality from one setpoint.

### P4 — microstructure and magnetic-property projection

Predicted packing, particle orientation and thermal histories may be supplied to validated microstructure or magnetic-property backends. The projection to porosity, texture, Br, Hc or (BH)max must state:
- constitutive/material model and property evidence;
- exact process and specimen state;
- temperature, field, orientation and geometry;
- calibration/validation envelope;
- uncertainty and model discrepancy;
- omitted physics and OOD disposition.

A predicted alignment statistic must not be represented as measured Kerr/SEM/XRD evidence. A predicted Br is not a measured hysteresis curve.

## Benchmark ladder

### P0 — source contract

Pin the article, table values, figure/transcription sources, machine settings, measurement method and data availability. Keep the unresolved D90 discrepancy explicit.

### P1 — deterministic analytical/synthetic controls

Before external solver execution, test:
- unit and dimension validation;
- geometric bounds and conservation checks;
- known orientation/torque limiting cases;
- PSD normalization and mass/volume-fraction conversion;
- deterministic replay for fixed source, configuration and seed;
- malformed, missing and out-of-domain inputs;
- predicted-vs-measured type separation;
- source-table D90 discrepancy retained as unresolved, not coerced into a false exact value.

Synthetic fixtures test software and mathematics only; they are not experimental powder-bed truth.

### P2 — literature baseline reproduction

Try to reproduce the source HD-versus-milled ordering under declared input assumptions. If the complete particle distributions, contact parameters or raw field/microstructure data cannot be accessed, record an explicit incomplete-input disposition. Do not invent distributions from D10/D50/D90 or use the paper's output values as both model inputs and validation targets.

### P3 — held-out predictive validation

Calibrate against one set of builds and evaluate on held-out builds, powder lots and process windows. Report:
- porosity and density error;
- orientation/texture error against the same measurement definition;
- distributional rather than only mean error;
- uncertainty coverage and calibration;
- OOD detection/abstention;
- sensitivity to PSD, particle shape, contact parameters, layer height and field history;
- runtime and resource use.

The unit of generalization is a genuinely held-out build/lot/process condition, not a random row from the same coupon.

### P4 — experiment-selection tournament

Compare the proposed Symthaea/Symtropy loop against:
- random selection;
- space-filling/LHS;
- factorial or fractional-factorial DOE;
- one-factor-at-a-time;
- physics/descriptor-guided selection;
- conventional Bayesian optimization when its assumptions fit the data.

Freeze candidate pool, training cutoff, initial data, allowed observations, acquisition budget, compute budget and stopping rule. Compare all methods at equal information and budget.

### P5 — component transfer

Only after coupon process-model validation, evaluate whether measured/predicted properties transfer to a named component geometry and use case. Component field maps and application-level force/torque, thermal, mechanical and lifetime claims require their own test boundaries.

## Metrics and baselines

Keep independent axes:

| Domain | Metrics |
|---|---|
| Particle input | PSD coverage, D10/D50/D90, distribution distance, morphology/aspect-ratio coverage |
| Packing | packing fraction, porosity, spatial porosity map, density |
| Orientation | particle-axis distribution, texture statistic, alignment degree with exact definition |
| Magnetic coupling | field-map error, orientation response, uncertainty vs field/time conditions |
| Process | dimensional error, build yield, energy, cycle time, scrap/rework |
| Prediction | held-out MAE/RMSE where appropriate, calibration/coverage, OOD recall and abstention |
| Optimization | Pareto quality and target attainment per experiment, wall time and compute |
| Reproducibility | exact input closure, deterministic replay, build/lot transfer |

Mandatory baseline families:
1. simple geometric/analytical packing model;
2. spherical-particle DEM baseline;
3. morphology-aware DEM;
4. morphology-aware DEM plus magnetic-orientation model;
5. coupled process/thermal model only when its input parameters are measured or calibrated;
6. conventional DOE and experiment-selection baselines.

Do not compare one model with more information or a larger experiment budget against a weaker baseline.

## Implementation boundaries

- Do not add a general DEM solver to Symtropy merely for this benchmark.
- Do not treat Symtropy's rigid-body dynamics as automatically equivalent to a validated granular-material DEM.
- Do not add an EM solver or micromagnetic solver to this workstream.
- Use provider-agnostic adapters and record solver capability, exact version, input closure, runtime, convergence, and output receipt.
- Do not promote predictions to measurements or issue physical machine/coil commands.
- Reuse the existing manufacturing digital thread and process-quality work rather than duplicating lot genealogy and inspection identity.

## Acceptance

The first executable tranche is acceptable when:
1. source facts and their unresolved uncertainties are machine-readable;
2. deterministic synthetic controls test input/identity/units/replay and evidence separation;
3. the process scenario contract explicitly separates measured, assumed, simulated and missing values;
4. solver adapters refuse unclosed inputs and unsupported model families;
5. baseline and held-out evaluation protocols are frozen before target access;
6. every report shows limitations, missing inputs, uncertainty and no-run dispositions.

## Safety and claim ceiling

This is modelling and research infrastructure only. No LPBF machine operation, powder handling, hydrogen processing, energized magnetic coil, process recipe execution or product-release authority is authorized. Physical testing requires appropriately qualified facilities, process-hazard review, interlocks and independent authorization.

A successful synthetic test is not powder-process validation. A converged solver is not a measured microstructure. A model fitted to a source article is not a successful blind prediction. No SOTA superiority is claimed.

## References

1. Schäfer et al. (2026), DOI [10.1002/aesr.202500339](https://doi.org/10.1002/aesr.202500339).
2. [MAG-AM-PROC-001 #7296](https://github.com/Luminous-Dynamics/symtropy/issues/7296).
3. Symtropy [PWR-TX-MFG-001 #2933](https://github.com/Luminous-Dynamics/symtropy/issues/2933).
4. Symtropy [WPT-MAGNETOTHERMAL-CO-DESIGN-001 #3490](https://github.com/Luminous-Dynamics/symtropy/issues/3490).
5. Symthaea [MAG-AM-001 #13506](https://github.com/Luminous-Dynamics/symthaea/issues/13506).
