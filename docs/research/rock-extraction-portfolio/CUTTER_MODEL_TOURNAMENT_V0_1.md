# Rock Cutter Model Tournament v0.1

**Parent research issue:** #3840  
**Related evidence:** #3836, #3823/#3824, #3830/#3834, #3812/#3814  
**Registry:** method-evidence registry v0.1.3  
**Claim ceiling:** benchmark protocol and evaluation-manifest conformance only. A valid manifest does not certify a numerical model, physical solver, sensor, machine or excavation method.

## 1. Purpose

Compare physics-based, empirical and learned models of cutter/rock interaction on the same frozen physical evidence. The evaluation target must match the intended use:

- cutter force and torque;
- force-time waveform and impact/impulse response;
- fracture initiation/path and damage zone;
- removed-solid volume and fragment distribution;
- measured tool-interface work and cutting specific energy;
- uncertainty, out-of-domain detection and computational cost.

A model that predicts force accurately can still predict the wrong fracture path, removed volume, fragment distribution or energy. A model that fits the source studies can still fail on an unseen rig or rock family.

## 2. Current reference baselines

### 2.1 Modified CSM and tabular deep learning

Rouhani & Rostami (2026), *Tunnelling and Underground Space Technology* 171:107448, DOI 10.1016/j.tust.2026.107448, compile more than 863 literature-derived linear-cutting records. They compare modified rock-type-specific CSM equations with SAINT, TabNet and TabM. Reported metrics include GMO-SAINT test R² 0.96 for normal force and RSO-TabNet test R² 0.94 for rolling force; modified CSM improves on original CSM for sedimentary, metamorphic and igneous groups (reported p < 0.001).

The available article summary does not establish whether test partitions keep parent blocks, source studies, rigs or rock types wholly out of training. These figures are reference targets to reproduce, not proof of transfer.

### 2.2 SPH–FEM for ODC

Zhang et al. (2026), *Machines* 14(7):829, DOI 10.3390/machines14070829, use SPH–FEM/JHC for an ODC. They calibrate sandstone at UCS 26.45 MPa; their 100 mm cutter / 200 × 200 × 50 mm domain uses a rigid cutter and homogeneous rock. The LCM validation uses 5 kHz acquisition, three repeats per condition and reports ±1.87% combined measurement uncertainty; mean-force simulation errors are reported as 15.14% at 16 Hz and 12.46% at 20 Hz. Their 78.08% minimum cutting-SE reduction and 70.9% / 59.4% / 67.9% rolling/side/normal force attenuation are study-specific comparisons against a non-eccentric reference, not full-cycle, high-strength granite or field results.

### 2.3 Peridynamic disc cutting

Hajivand Dastgerdi et al. (2025), *Simulation Modelling Practice and Theory* 143:103146, DOI 10.1016/j.simpat.2025.103146, compare bond-based peridynamic simulations with Colorado Red Granite LCM data at 3.2, 5.1 and 6.4 mm penetration. They report normal-force errors 0.5–4%, rolling-force errors 8–14% and runtime about 1 h 33 min on an engineering workstation. Reproduce geometry, force error denominator, boundary conditions, material calibration and resolution before ranking.

## 3. Frozen dataset construction

A record is not independent merely because it has a different row ID. The dataset manifest must preserve and group by:

- parent physical block and specimen;
- lithology / rock family / petrographic cohort;
- source paper and original experimental campaign;
- site, tunnel or mine;
- linear-cutting rig and machine;
- cutter geometry, cutter serial and wear state;
- operator, run order and time window where these can confound results.

Store original source identifiers and artifact digests. Do not split adjacent cuts, repeated tracks or sister specimens from one parent block/campaign randomly across training and test. A source-study holdout is especially important when a machine-learning dataset is assembled from published papers: repeated records from the same paper may share rig, measurement and preprocessing characteristics.

### Required split classes

1. Random-row split: diagnostic only; never the transfer score.
2. Parent-block/specimen holdout.
3. Rock-family or lithology holdout.
4. Source-study holdout.
5. Rig/machine holdout.
6. Site/tunnel holdout.
7. Cutter geometry/tool serial holdout.
8. Forward temporal field holdout, where future rings/sections remain inaccessible to training.

Every split manifest must list its holdout dimensions and immutable train/test group digests. A semantic validator must verify that test groups are disjoint from training groups for every declared holdout dimension. JSON Schema alone cannot enforce this cross-array condition.

## 4. Model families

The tournament should include:

- original Colorado School of Mines (CSM) semi-theoretical model;
- modified CSM models, including rock-type-specific variants;
- transparent regression baselines;
- SAINT, TabNet, TabM and compact tabular models;
- hybrid physics + learned residual / uncertainty models;
- FEM with JHC/RHT or other calibrated constitutive models;
- SPH–FEM;
- bond-based and state-based peridynamics;
- DEM / bonded-particle methods for block contacts and fragment interactions.

Do not force every model family to predict every target. Each model declares its target set and intended validity domain. Unknown inputs, unsupported rocks and unrepresented joints/confinement must produce an out-of-domain result or an explicitly qualified extrapolation—not a silent confident prediction.

## 5. Metrics and acceptance

### Force and motion

Report per-axis MAE, RMSE, bias, normalized error, R², 95th-percentile absolute error, peak error, impulse error and force-time waveform discrepancy. Use the same units, filtering rules and force definitions across baselines. Preserve signed errors; do not report R² alone.

### Fracture and material

Where physical labels exist, report crack-initiation location, crack-path/topology discrepancy, fracture/damage-zone geometry, removed-solid volume error, fragment-size distribution error, fines/oversize, and mass/volume balance. Simulation damage variables are not automatically observed crack labels; compare against CT, DIC, microscopy, independent scanning or other declared physical measurements.

### Energy

Compare predicted tool-interface work with measured force-displacement/torque-rotation work on the same declared boundary. Keep cutting-only work, machine/facility input and full-cycle energy separate. Downstream Bond/P80 estimates are not measured crusher energy.

### Uncertainty and OOD

Report prediction interval coverage and width, calibration, out-of-domain detection recall, false-accept rate, abstention rate, sensor-noise sensitivity and drift. Report confidence intervals by rock/site/rig group and worst-group tails, not only pooled averages.

### Computational practicality

Record wall-clock, hardware/solver version, memory, mesh/particle resolution, time step, convergence failures, determinism and scaling with domain/particle count. Report accuracy–cost Pareto curves, not only the most accurate run.

### Gates

- Reproduce a published comparator on its stated domain and metric boundary.
- Evaluate all model families on identical frozen test records.
- Freeze grouping and split manifests before model tuning.
- No group overlap across declared holdout dimensions.
- Uncertainty and OOD behaviour are reported.
- A physical solver needs force + fracture/volume/fragment evidence for its intended task.
- A learned model needs leakage audit and prospective validation.
- Measurement uncertainty is reported separately from model error.
- No universal “winner” or weighted composite score in v0.1.

## 6. Proposed paradigm shifts

1. **Model ensembles that know when not to predict.** Use mechanics baselines, learned residuals and OOD checks; abstain when material, machine or geometry leaves the evidence domain.
2. **Inverse and forward consistency.** A model estimating UCS/BTS from forces should be checked by forwarding the inferred properties back to force prediction and comparing with a separate measured result.
3. **Active sensing.** Treat cutter force/torque and vibration as a geological observation stream, but validate estimates against independent cores, face mapping and lab measurements.
4. **Fidelity on demand.** Use cheap CSM/learned surrogates for fast screening, and FEM/SPH/peridynamics only where the task requires fracture-field resolution. Calibration and model selection depend on the physical question.
5. **Prospective utility, not score chasing.** Ultimately test whether better uncertainty-qualified predictions improve method/parameter selection and accepted full-cycle output on unseen ground.

## 7. Integration and boundaries

- #3840 owns the model tournament and evaluation protocol.
- #3836 owns the reciprocal cutter-force / rock-strength benchmark.
- #3305 owns TBM ring-level cutter, machine and geology data.
- #1593 owns measured mechanical specific energy.
- #1588/#1603 own physical material/tool response.
- #1594 owns acoustic emission / propagation models.
- #3823/#3824 own synchronized mechanism response records.
- #3830/#3834 own response-bundle semantic validation.
- #3812/#3814 own full-cycle accepted output and declared energy comparisons.
- #3825 owns multi-carrier energy/resource accounting.

The model benchmark does not authorize safety decisions or machine control. Independent physical measurements and field evidence remain the referee.
