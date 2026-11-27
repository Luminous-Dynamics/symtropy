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
- residual-Jacobian aggregate record ID and nine nonnegative finite component bounds;
- three component-level interval-AD record IDs, expression IDs and exact input digests for residual Bx, By and Bz;
- the interval-AD source revision, implementation ID, artifact digest and environment digest that produced the three derivative enclosures;
- probe calibration, registration, background and temperature evidence IDs;
- solver run and model-set IDs;
- geometry/material digests, coordinate frame, operating condition and field units.

The crate checks identity consistency and interval arithmetic. It cannot authenticate source records or prove an upstream Jacobian supremum is physically correct.

## Interval-AD-backed residual Jacobian derivation

The crate now provides `bind_interval_ad_residual_jacobian`. It accepts three `ValidatedDerivativeEnclosure<3>` records—one each for the exact scalar residual components Bx, By and Bz—and maps their XYZ derivative absolute suprema into the 3×3 Jacobian matrix.

The adapter requires:
- exact expected expression IDs for Bx/By/Bz and three distinct component record IDs;
- the same cell ID, exact XYZ box, axis identities and metre parameter units;
- field output units matching the physical field units supplied to this crate;
- matching frame, operating condition, model set, solver run, geometry/material, requirement and residual-expression provenance;
- identical source revision, implementation ID, artifact digest and environment digest across the three component derivatives.

The three interval-AD records' own input SHA-256 digests are retained individually in the resulting physical-discrepancy record. The residual Jacobian aggregate ID does not replace those component identities.

This removes the need to manually transcribe the nine derivative suprema. It still does **not** authenticate that the supplied expressions really represent the physical model-minus-measurement residual, or that their input parameter boxes are physically correct. Expression IDs are checked against caller-supplied expected IDs; physical semantics remain a separate model-review obligation.

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
- derives the 3×3 residual Jacobian from three scope-matched interval-AD certificates (Bx, By, Bz), retaining each component record ID, expression ID and input digest;
- rejects component expression, cell-box, unit or source-scope mismatch;
- preserves evidence role, including blind-holdout identity, without fitting or modifying the supplied model.

The crate currently has 16 authored tests, including an affine residual fixture checked at all eight cell corners, zero-width cells, measurement/Jacobian monotonicity, centre/cell identity, inconsistent model enclosures, missing calibration/registration/requirement identities, invalid Jacobian values, component-wise interval-AD matrix construction, expression mismatch and cell-box mismatch. These tests are authored; do not treat them as a pass until the exact PR-head matrix completes.

## Benchmark plan

Compare a zero-discrepancy assumption, constant centre-bias model, affine residual model, Lipschitz/Jacobian supremum envelope, interval-Taylor residual model and probabilistic GP residual as a separate non-deterministic comparator.

Use frozen physical holdouts. Report component-wise enclosure width, fraction of cells with finite bounds, physical holdout misses, false-safe/false-fail decisions, observation count, calibration/registration overhead and interval-evaluation cost.

Primary deterministic target: zero physical holdout samples outside the claimed residual envelope on the preregistered challenge suite. Sampled challenge passes are necessary falsification controls, not a proof of the Jacobian supremum.

## Claim ceiling

This is an arithmetic and evidence-contract reference. It does not create probe calibration, certify the measurement uncertainty, derive the residual-Jacobian supremum, validate a constitutive law, authenticate a solver, infer manufacturing yield or replace physical metrology. No component-release or actuation authority is added.
