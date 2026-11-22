# DPW-8 Force/Moment V5 parser and reconciliation protocol v0.1

## Scope and evidence ceiling

This protocol defines a clean-room parser for the AIAA DPW-8/AePW-4 Force/Moment submission form, Version 5. It is an ingestion and reconciliation component, not a CFD solver, source authenticator, aerodynamic validation method, or optimizer.

The source review motivating this adapter found that one public participant file presents the same grid/angle points twice: once in zones grouped by grid level and once in zones grouped by angle. The parser must not count the two presentations as 40 independent simulations. It canonicalizes the shared (GRID_LEVEL, ALPHA) cells only after exact field-by-field agreement, retaining both original line numbers. It never overwrites an original result.

The official V5 form describes 31 named variables, required vehicle force/moment coefficients, optional component fields, and -999 as the missing-value sentinel. Therefore an optional -999 becomes a typed missing value; it is never substituted with zero. Dataset and zone auxiliary metadata are retained verbatim.

The pinned participant result that motivated this adapter is referenced in issue #2936. Its repository revision and Git blob are recorded there. The upstream repository root did not expose a license at that revision; no official submission bytes are vendored here, and no redistribution permission is inferred. The committed fixture is explicitly synthetic and must never be represented as participant or experimental data.

## Parser invariants

1. Require the exact 31-column V5 schema and one TITLE.
2. Preserve dataset-level and per-zone auxiliary metadata.
3. Preserve physical source line numbers for every numeric record.
4. Require each row to agree with its enclosing GRID LEVEL n or ALPHA x zone.
5. Require finite numeric values, positive grid identifiers/sizes, positive GRID_FAC, Mach and Reynolds number, and all nine required fields.
6. Convert optional -999 fields to None; reject -999 in the nine required columns.
7. Reconcile the grid-level and angle-level views by (GRID_LEVEL, ALPHA).
8. Reject duplicate keys in either view, missing counterpart cells, a different key set, or any field mismatch.
9. Emit one canonical row per pair with both source line references.
10. Require one stable mesh identity (GRID_SIZE and GRID_FAC) per GRID_LEVEL and reject reuse of a GRID_SIZE under multiple levels.\n11. Flag suspicious coefficient patterns as warnings only. Never average, correct, remove, or silently exclude flagged rows.

## Synthetic known-answer and negative controls

The committed data/dpw8/synthetic_two_view_fixture_v0.1.dat contains four grid levels and five angles. Each view contains 20 records, for 40 raw rows and 20 canonical cells. Optional component values are present only where explicitly given; every -999 is missing. One deliberately suspicious cell copies another angle's total lift/drag coefficients and makes a large adjacent-grid discontinuity. The audit must flag it while preserving its exact CL/CD/CM values and both source lines.

Rust unit tests and the stdlib Python oracle independently verify:
- 40 -> 20 cardinality, full two-view equality and deterministic canonical order;
- exact preservation of both line IDs and dataset/zone metadata;\n- stable mesh identity across all angles belonging to one grid level;
- missing-sentinel semantics;
- suspicious-result detection without mutation;
- one-field paired-view mutation rejection;
- duplicate-cell rejection;
- zone-label/row disagreement;
- malformed field-count rejection;
- non-finite required-value rejection.

The fixture is a parser correctness test only. Its coefficient values are synthetic and are not an aerodynamic benchmark result.

## Next validation ladder

### A. Source and file-format validation
Obtain explicit source-owner/workshop permission or a documented license before mirroring participant bytes. Preserve the original file, exact commit/blob, byte length, digest, source path, source authority, and terms. Add independent parser agreement on lawfully accessible official examples. Unknown syntax, unknown columns, changed field counts, unknown sentinel rules, and conflicting table views fail closed.

### B. Solver verification
Before comparing solver predictions, establish iterative convergence, conservation/consistency, spatial convergence, and temporal convergence where relevant. For systematically refined grids, report the refinement ratio, observed order, extrapolated value, uncertainty band, and Grid Convergence Index (GCI), with limitations when the grid family is not systematically similar. A result without solver history, grid lineage, and convergence status is not an admitted solver comparison.

### C. Physics validation
Compare CL, CD and CM separately against the relevant experimental reference and its measurement uncertainty. Also compare surface-pressure distributions, spanwise loads, deformation and unsteady spectra where that working group requires them. Keep steady RANS, buffet, high-angle, high-speed, static-deformation and dynamic aeroelastic tasks distinct. A close integrated coefficient alone does not validate all flow physics.

### D. Robust optimization and decision value
Compare adjoint, gradient-based, Bayesian, evolutionary, random-search and Symthaea/Symtropy policies on the same frozen tasks, held-out geometry families and matched budgets. Count every expensive solve, failed/non-converged solve, surrogate call, tuning run, orchestration cost and independent audit. Report best validated objective against budget, hypervolume against budget, feasible-design yield, regret to the best known result, convergence rate, wall time, and uncertainty/fidelity gap. Do not rank optimizers from one geometry or one task.

### E. Physical and manufacturing evidence
Only after a separately reviewed test plan, compare the predicted candidate to measured non-flight coupons or a qualified test article. Record dimensional metrology, surface finish, material/build lot, test rig calibration, repeatability, expanded uncertainty, failure modes and exact artifact digests. Never infer flightworthiness, structural adequacy or manufacturing release from this parser or a coupon geometry audit.

## Benchmark targets

The parser acceptance target is exact: zero lost canonical cells, zero double-counted cells, 100% paired-field equality, 100% source-line coverage, zero optional-sentinel-to-zero coercions, deterministic output, and rejection of every one-field mismatch in the negative-control suite.

Solver targets should be declared per case before looking at candidate results. Report convergence and uncertainty rather than applying one universal drag-error threshold across different regimes. The minimum numerical study uses at least three suitable mesh levels; the workshop's prescribed case-specific grid family takes precedence.

Optimizer targets are decision quality per total cost, not raw candidate iterations: validated objective/hypervolume curves under common cost caps, feasible yield, held-out performance, OOD abstention, calibrated uncertainty, and cost of failed runs. The confirmation set must be sealed before optimizer tuning.

## Reference material

- AIAA DPW-8/AePW-4 workshop: https://www.aiaa-dpw.org/
- Official V5 submission/postprocessing instructions and -999 semantics: https://www.aiaa-dpw.org/postprocessing.html
- NASA CFD verification assessment: https://www.grc.nasa.gov/www/wind/valid/tutorial/verassess.html
- NASA DPW grid archive: https://dpw.larc.nasa.gov/DPW8/
- ShapeBench benchmark paper: https://arxiv.org/abs/2605.20763
- SU2 source and release lineage: https://github.com/su2code/SU2


Every invocation reports the exact input byte length and SHA-256 digest. The digest identifies the bytes processed; it does not authenticate who supplied them or make an unverified participant file official. Passing a path is always caller-supplied and unverified, even if its name matches the repository fixture.
