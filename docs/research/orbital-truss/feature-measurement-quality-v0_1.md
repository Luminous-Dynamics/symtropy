# Feature Measurement Quality — Research Protocol v0.1

**Issue:** [#7043 ORBITAL-FMQ-001](https://github.com/Luminous-Dynamics/symtropy/issues/7043)  
**Current code:** synthetic analytic 2D circle-feature identifiability benchmark  
**Claim ceiling:** no measurement covariance, scanner accuracy, NDE/POD or physical acceptance

## 1. Why surface coverage is not enough

The October 2026 GSD set-multicover benchmark and our #6925 planner answer: how many views geometrically observe each surface region? That is useful, but it is not the same as determining whether a particular engineering characteristic can be measured.

A new November 2026 paper, Zhang et al., *Feature-oriented automatic viewpoint planning for structured-light 3D measurement*, proposes a Feature Measurement Quality (FMQ) model for dimensional inspection. It treats effective depth information along a feature normal and tangential constraint quality across the measurement plane as distinct contributors to feature fitting. Its abstract reports 0.0936 mm maximum mean absolute error for circular-hole measurements (0.0108 mm standard deviation) and a planning time of approximately 41 seconds for about 26 features. Those results are specific to its binocular structured-light sensor, industrial robot, part geometry, calibration, method, tolerances and experimental setup. They are not targets for this synthetic benchmark.

Paper: https://doi.org/10.1016/j.optlastec.2026.116282

## 2. Our initial, disclosed model

For a 2D circular feature with parameters \(\theta=(c_x,c_y,r)\), a point on the ideal circle at contour angle \(\phi_i\) has a radial residual whose local Jacobian is:

\`J_i = [-cos(phi_i), -sin(phi_i), -1]\`.

This v0.1 research prototype forms a normalized geometric information matrix:

\`F = (1/N) J^T J\`

over **unique contour support angles**, not raw observation row count. Repeating the same contour support from multiple view IDs does not create new geometric rank. A future measurement model may treat repeated acquisitions as independent or correlated precision evidence, but only when the observation covariance and dependence assumptions are declared.

We report:
- unique geometric support count;
- observation row count and distinct declared failure-domain count, separately;
- information matrix and eigenvalue spectrum;
- numerical rank;
- condition number against a case-declared limit;
- nominal and finite-perturbation robust statuses.

The information matrix is dimensionless and normalized. It is a geometric identifiability/conditioning diagnostic, not a Fisher-information claim under a calibrated noise distribution, and it is not a measurement covariance. The eigenvalues/condition number depend on the declared circle residual model and the sampled contour support.

## 3. The paradigm shift

A feature can have:
- high surface visibility but poor feature-parameter identifiability;
- low global coverage but good local geometric constraints for a specific measurand;
- many repeated views but little additional independent geometric support;
- a nominally sufficient view set that becomes underconstrained under finite pose perturbations.

The design question becomes:

**What set of feasible observations constrains the actual engineering measurand, with enough geometric conditioning and declared uncertainty, to support the next decision?**

That is a stronger objective than maximizing covered area. It should eventually compose, not collapse:
1. #6925 surface/view coverage;
2. feature-level identifiability/conditioning (#7043);
3. covariance-aware metrology and guard-banding (#6846);
4. sensor and calibration common-cause analysis (#4258);
5. decision-aware new measurement selection (#4997);
6. route/robot feasibility (#5920);
7. non-anticipative policy and evidence adjudication (#6956 / Mycelix).

## 4. v0.1 analytic corpus

The frozen corpus has five cases:
- four cardinal contour supports: analytically full-rank, condition number 2;
- narrow clustered arc: full rank but ill-conditioned;
- repeated same-direction observations from multiple view IDs/domains: one unique support direction and rank 1;
- nominal full circle but only two robust directions: robustly underconstrained and indeterminate;
- rigidly rotated cardinal supports: identical eigenvalue spectrum.

The benchmark uses fixed-point serialized diagnostics so an independent standard-library Python implementation can compare complete receipts across platforms. It uses a small symmetric 3x3 eigenvalue solver, relative rank threshold, and explicit conditioning bound.

## 5. What the first benchmark does not do

It does not:
- fit raw point clouds;
- model a 3D cylindrical bore or slot;
- implement the FMQ formula from the paper;
- model stereo triangulation, depth of field, illumination, reflectivity or surface material;
- model sensor calibration, point covariance or correlated errors;
- estimate GUM uncertainty or conformity to a tolerance;
- detect defects or estimate POD;
- generate reachable robot trajectories;
- qualify physical equipment or authorize a decision.

A future exact reproduction needs the full article formulas, sensor specifications, calibration and part/ground-truth data. Without those, this is our independently specified analytic surrogate, not a paper reproduction.

## 6. Next benchmark ladder

**F0 — analytic identifiability.** Exact known-answer circle arcs; rank, spectrum, condition number, duplicate-support invariance and rigid-rotation metamorphisms.

**F1 — feature family.** Add line/plane/cylinder/slot/bolt-pattern models and derive their Jacobians. Each model gets independent analytic fixtures and rank/degeneracy controls.

**F2 — point-cloud measurement.** Add actual points, feature fitting algorithms, robust outlier handling, voxel/point density, partial occlusion, sampling anisotropy and registration-frame perturbations.

**F3 — uncertainty.** Introduce Type A/B observation covariance, full cross-covariance and calibration/pose common causes. Propagate by Jacobian and independently check with Monte Carlo. Do not create covariance when the sensor/noise model is missing.

**F4 — decision.** Bind measured characteristics to #6846 tolerance/guard-band profiles. Report false-accept, false-reject and abstention under known truth; never lower a threshold to make a case pass.

**F5 — acquisition and route.** Generate additional candidate observations from measurability deficits; #4997 selects actions under cost and decision impact, while #5920 handles route/robot feasibility. Predicted measurements are not observed results.

**F6 — physical reproduction.** Reproduce the November 2026 FMQ paper with matched sensor, robot, geometry, calibration, ground truth, tolerances and feature set. Report MAE, bias, dispersion, repeatability, failure/abstention, path cost and computation separately. Transfer to orbital truss only after matching the actual physical modality and measurand.

## 7. Metrics and target policy

Keep these distinct:
- surface k-coverage;
- unique geometric support;
- information-matrix rank;
- minimum eigenvalue and condition number;
- nominal-versus-robust degradation;
- feature parameter error against traceable ground truth;
- uncertainty coverage/calibration;
- false acceptance, false rejection and abstention;
- acquisition time, path length and resource use.

A high feature score must not compensate for a hard constraint failure. No composite score is defined in v0.1. The condition threshold is a declared benchmark parameter, not a universal industrial limit.

## 8. Initial gates

- 100% independent agreement on frozen analytic cases.
- Zero rank inflation from duplicate contour support.
- Exact rigid-rotation invariance of the diagnostic spectrum within fixed-point tolerance.
- Robust incompleteness must never be hidden by nominal sufficiency.
- No covariance, measurement accuracy or POD claim without a separately declared sensor/error model and validation evidence.

## 9. Source and benchmark references

- Zhang et al. (November 2026), FMQ-guided structured-light dimensional feature inspection: https://doi.org/10.1016/j.optlastec.2026.116282
- Bin Khalid & Yeum (27 October 2026), GSD-constrained k-coverage viewpoint planning: https://arxiv.org/abs/2610.36450
- NASA Design for Inspection (2024): https://ntrs.nasa.gov/citations/20240003953
- NDE 4.0 review (26 June 2026): https://doi.org/10.3390/ndt4030017

The NDE 4.0 review specifically cautions that statistically qualified POD is not interchangeable with classifier accuracy/F1 and that domain shift, sensor variability, weak interoperability and lack of qualified decision rules remain adoption barriers. v0.1 therefore stops at geometric identifiability and does not emit POD-like outputs.
