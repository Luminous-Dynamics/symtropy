# Magnetic physical-discrepancy envelopes (v0.1)

A zero-dependency Rust research reference (apart from the sibling interval-AD crate) for enclosing the difference between a magnetic field solver and a traceable physical observation over an entire spatial cell.

## Why this layer is needed

Interval AD can certify that a *mathematical expression* is enclosed over a parameter box. It cannot prove that the expression matches a real magnet, its constitutive law, the installed sensor geometry, or the calibrated probe observation.

This crate separates:
- the solver's predicted field interval over the complete cell;
- the measured physical field interval at the exact cell centre;
- the model-predicted field interval at that same centre;
- a separately sourced, component-wise residual-Jacobian absolute supremum over the complete cell.

It then forms a deterministic residual envelope and combines it with the cell-wide solver prediction. The result is conditional on the validity of the measurement interval and residual-Jacobian suprema.

## Mathematical contract

For field component i, the centre residual interval is:

    R_i(c) = B_measured_i(c) - B_model_i(c)

For cell half-widths h_j and a source-bound residual-Jacobian supremum:

    J_abs[i,j] >= sup_{x in cell} |∂R_i(x)/∂x_j|

the cell-wide residual is enclosed by:

    R_i(cell) ⊆ R_i(c) + Σ_j [-J_abs[i,j] h_j, +J_abs[i,j] h_j]

The physical field over the cell is enclosed by:

    B_physical_i(cell) ⊆ B_model_i(cell) + R_i(cell)

All arithmetic uses the outward-expanded interval operations from the sibling magnetic-interval-ad crate. The cell centre is required to be exactly the representable midpoint of the declared box. The measured and centre-model records must use that same coordinate frame, condition, component convention and exact centre coordinate.

## Evidence requirements

Each input binds:
- exact cell ID and 3D bounds in metres;
- exact centre coordinate and measurement record ID/digest;
- measured field intervals at the centre, already including declared probe/calibration/registration/background/temperature uncertainty;
- model field intervals at the centre and over the complete cell;
- residual-Jacobian supremum record ID and nine nonnegative finite component bounds;
- probe calibration, registration, background and temperature evidence IDs;
- solver run and model-set IDs;
- geometry/material digests, coordinate frame, operating condition and field units.

The crate checks identity consistency and interval arithmetic. It cannot authenticate source records or prove an upstream Jacobian supremum is physically correct.

## Fail-closed rules

- Missing measurement, solver, calibration, registration, environment or residual-bound identities are rejected.
- Cell centre mismatch is rejected.
- Non-finite measured/model intervals or residual-Jacobian bounds are rejected.
- Negative or non-finite Jacobian absolute bounds are rejected.
- An unbounded input cannot produce a finite physical-field certificate.
- A measured standard deviation, RMSE or finite sample maximum is not accepted as a deterministic supremum by conversion. The caller must provide a separate absolute interval/bound and its evidence identity.
- The result remains a predicted physical-field enclosure, not a measurement map or a centre observation. It cannot be substituted for the separate centre margin used by spatial functional certification.

## Initial implementation

The public Rust reference is under `research/magnetics/physical-discrepancy/` on branch `research/magnetic-physical-discrepancy-v0.1`. It:
- computes the centre residual from interval-valued measurement and model centre observations;
- computes outward-expanded half-widths from the exact cell box;
- applies a 3×3 residual-Jacobian absolute-supremum matrix to form component-wise cell residual intervals;
- adds each residual interval to the solver's full-cell field interval;
- checks model-centre inclusion in the model's full-cell enclosure;
- retains solver/model and Jacobian source, implementation, artifact and environment identities, along with calibration/registration/background/temperature IDs;
- preserves evidence role, including blind-holdout identity, without fitting or modifying the supplied model.

The authored tests include an affine residual fixture checked at all eight cell corners, zero-width cells, measurement/Jacobian monotonicity, centre/cell identity, inconsistent model enclosures, missing calibration/registration identities, invalid Jacobian values and source-digest controls. Authored tests are not a test pass.

## Benchmark plan

Compare a zero-discrepancy assumption, constant centre-bias model, affine residual model, Lipschitz/Jacobian supremum envelope, interval-Taylor residual model and probabilistic GP residual as a separate non-deterministic comparator.

Use frozen physical holdouts. Report component-wise enclosure width, fraction of cells with finite bounds, physical holdout misses, false-safe/false-fail decisions, observation count, calibration/registration overhead and interval-evaluation cost.

Primary deterministic target: zero physical holdout samples outside the claimed residual envelope on the preregistered challenge suite. Sampled challenge passes are necessary falsification controls, not a proof of the Jacobian supremum.

## Claim ceiling

This is an arithmetic and evidence-contract reference. It does not create probe calibration, certify the measurement uncertainty, derive the residual-Jacobian supremum, validate a constitutive law, authenticate a solver, infer manufacturing yield or replace physical metrology. No component-release or actuation authority is added.
