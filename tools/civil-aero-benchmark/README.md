# Civil Aero Benchmark 0.1

A dependency-free Rust tool for transparent first-order mission-energy screening of non-weaponized civil research aircraft. It is a starting engineering artifact, not a flight design, CFD solver, flight controller or airworthiness assessment.

## Run

From this directory, run cargo fmt --check, cargo test --locked, cargo clippy --locked --all-targets -- -D warnings, then cargo run --locked -- data/example_cases.csv.

The example input is synthetic. It tests dimensional relationships and controlled counterfactuals; it is not evidence of achievable aircraft performance.

## Model boundary

For steady, level, still-air cruise:

- CL = m g / (q S)
- CD = CD0 + CL squared / (pi e AR)
- D = q S CD
- E_route[Wh] = D times distance[m] divided by (eta times 3600)
- E_usable[Wh] = battery_energy[Wh] times (1 - reserve_fraction)

Here q = 0.5 rho V squared; m is airframe plus payload mass; S is wing planform area; AR is aspect ratio; e is Oswald efficiency; CD0 is zero-lift drag coefficient; eta is a caller-supplied combined propulsive/electrical efficiency. Distance is converted to metres; N*m is joules and 3600 J = 1 Wh.

The screen flags only whether the estimated cruise lift coefficient is below a caller-supplied assumed limit and whether estimated route energy fits within caller-specified energy after reserve. Both can be wrong. A combined pass is not a safe-to-fly or airworthiness determination.

The model omits climb, takeoff, landing, turns, gusts, wind, control authority, stability, dynamic response, propeller operating maps, motor/controller thermal limits, battery sag and aging, aeroelasticity, structural sizing, fabrication tolerances, manufacturing yield, uncertainty propagation and regulations. Do not use it to authorize flight or manufacture a flight-critical part.


## Measured section-polar discrepancy diagnostic

The `section_polar_audit` binary performs a separate two-dimensional diagnostic using the pinned NASA TMR Ladson NACA 0012 force observations. It fits the empirical relation `Cd_section = a + b * Cl^2` independently within each of the 80, 120 and 180 grit zones using only `|alpha| <= 8.5 deg`. It reports MAE, RMSE, maximum absolute error and mean signed error for the calibration rows, an intermediate held-out band (`8.5 < |alpha| <= 16.5 deg`) and a high-angle extrapolation band (`|alpha| > 16.5 deg`). These boundaries are benchmark splits only, not safe operating limits or stall declarations.

The initial pinned-data run is expected to expose a sharp validity gap: calibration RMSE is approximately 0.00022–0.00026, intermediate held-out RMSE approximately 0.00319–0.00413, and high-angle extrapolation RMSE approximately 0.182–0.284 in the three zones. These values are data-specific diagnostics, not universal error guarantees. The NASA TMR source warns that high-angle experimental measurements may no longer be effectively two-dimensional near stall and that transition/tripping materially affects drag.

Important distinctions:
- The fitted coefficient `b` is only an empirical **2D section** coefficient. It is not the aircraft's induced-drag factor `1/(pi e AR)`, and it must not be used to infer Oswald efficiency.
- Roughness zones are kept separate; they are not pooled as IID samples.
- Every report carries the expected source digest and upstream revision, but the workflow independently verifies the exact raw bytes with `sha256sum -c` before running the diagnostic. The CLI does not itself compute SHA-256.
- A successful report is a model-discrepancy diagnostic, not solver validation, aerodynamic certification, or a physical flight envelope.

Run it from the repository root with:

```bash
cargo run --locked --manifest-path tools/civil-aero-benchmark/Cargo.toml --bin section_polar_audit
```


A test-only, stdlib-only independent Python oracle (`scripts/verify-section-polar-independent.py`) separately parses the raw file, checks its SHA-256 and byte length, refits each zone, recomputes every error metric and verifies that perturbing held-out observations cannot affect calibration coefficients. Production remains Rust; the Python script is only a second implementation used by CI to reduce common-mode test errors.

The strategic next gate is to require a calibrated model's uncertainty and demonstrated domain to travel with every candidate. An optimizer that leaves the verified domain must request a higher-fidelity evaluation or return an abstention—not quietly extrapolate a nominal score.

## Input and provenance

The first CSV line must match the documented header exactly. This deliberately small parser rejects quoted fields and comma-containing values. parameter_basis is synthetic, estimate, or measured. Non-synthetic rows must provide a 64-character hexadecimal source_sha256 and non-empty source_ref.

Source fields are declared metadata: the tool does not download evidence, recompute hashes, authenticate suppliers or verify measurements. A measured label is not proof of calibration. Every output includes a model version and evidence_level=screening_only.

The tool emits unweighted engineering metrics and separate gate flags so a single score cannot hide an energy or lift-margin failure.

## Verification and validation sequence

1. Code verification: units and shape checks, deterministic replay, monotonicity/counterfactual tests, invalid-input rejection.
2. Analytical/solver parity: independently review hand calculations, then compare with pinned outputs from SU2 or another named solver. A solver producing a result is not proof of physical validity.
3. Public-data validation: use NASA's Turbulence Modeling Resource NACA 0012 force and pressure datasets under published Reynolds/Mach conditions. Record grid/refinement evidence, compare lift, drag and pressure separately, and preserve near-stall limitations.
4. Mission-level validation: only after aerodynamic coefficients have an appropriate validation domain, combine them with motor/propeller maps and battery measurements. Hold out complete environmental conditions and physical builds, not random rows from a single run.
5. Manufacturing qualification: compare measured build mass, dimensional tolerances, material/layup records, first-pass yield, scrap, repairability, cost and energy per conforming unit. Keep missing values unresolved; do not turn them into zero.
6. Physical test: independent review of load cases, fail-safe behaviour, operating permissions and site hazards before flight. This screening tool never grants authorization.

## Comparison protocol

Compare candidates on the same stated mission, payload, density profile, route distance, reserve, wind treatment, measurement process and evidence level. Report route energy, Wh per payload-kilometre, estimated L/D, lift and energy margins, uncertainty and model validity. Show per-condition and worst-condition outcomes alongside aggregates. Improvement claims require paired held-out comparisons plus physical confirmation; synthetic examples cannot meet that bar.

## Next work

- Pin and checksum the NASA TMR NACA 0012 reference files and write an independent data-admission manifest.
- Add grid/refinement, convergence and benchmark-report ingestion without implementing a bespoke CFD solver.
- Establish a separate low-Reynolds-number dataset relevant to small electric research airframes; do not pool different Reynolds regimes.
- Add uncertainty propagation and a fail-closed comparator while preserving unweighted source metrics.
- Define a manufacturable, non-flight-critical airfoil coupon and a documented measurement procedure before any full demonstrator.

## References

- NASA TMR, 2D NACA 0012 Airfoil Validation: https://tmbwg.github.io/turbmodels/naca0012_val.html
- NASA CFD Verification and Validation overview: https://www.grc.nasa.gov/www/wind/valid/tutorial/overview.html
- SU2 open-source CFD and PDE-constrained optimization: https://github.com/su2code/SU2
- NASA OpenVSP Ground School: https://www.nasa.gov/software/openvsp-ground-school/
- NIST industrial CFD V&V/UQ review: https://www.nist.gov/publications/summary-industrial-verification-validation-and-uncertainty-quantification-procedures

## Evidence-source manifest

`data/source_manifest.v0.1.json` records the exact NASA TMR force, pressure, geometry and grid URLs discovered for the high-Re validation lineage, alongside a candidate UIUC low-Re data lineage. It is deliberately a **source-discovery manifest**, not a qualified dataset: hashes and byte lengths remain null until the exact raw artifacts are retained and independently hashed. The CI guard rejects a null hash paired with an admitted-looking status.

Do not report a CFD validation result until raw bytes, revision, conditions, units, licensing/applicability review, parser tests, convergence evidence and independent checks have been recorded. Do not pool the NASA Re=6,000,000 case with low-Reynolds-number airfoil measurements as if they were the same regime.

## Matched-mission candidate comparison

A second CLI, the Pareto comparator, compares candidates only when they share an identical mission and parameter evidence class:

`cargo run --locked --bin pareto -- data/candidate_designs.csv`

It re-evaluates each candidate from its stored input fields at the comparison boundary rather than trusting caller-provided derived metrics. It excludes candidates failing either screening gate, rejects mixed missions/evidence classes and duplicate IDs, and reports the nondominated set without inventing a single scalar winner.

The frozen synthetic corpus intentionally includes three trade-off candidates and one dominated candidate. Its expected Pareto set is balanced_reference, lower_mass_higher_drag, and higher_mass_lower_drag; dominated_high_drag should be excluded. This expected set is a property of the chosen synthetic fixture, not evidence that any physical airframe is optimal.

Objectives are estimated route energy (lower), total mass (lower), and estimated lift-to-drag (higher). Total mass is merely a proxy here: without structural sizing and load cases, lower mass says nothing about adequate strength. This comparator is not robust optimization; it samples one nominal point per candidate and does not propagate parameter/model uncertainty.

## Robustness and industry-comparison protocol

Read ROBUST_PARETO_BENCHMARK_PROTOCOL_V0_1.md for the baseline ladder and research plan. It compares the first-order screen (B0), public reference case (B1), pinned SU2 fixed-geometry solution (B2), SU2 adjoint optimization (B3), a conventional fixed-budget multi-objective baseline (B4), and the proposed Symthaea/Symtropy/SU2 pipeline (B5).

The intended differentiator is decision credibility per engineering cost—not a claimed aerodynamic result: useful experiments selected with a fixed expensive-evaluation budget, held-out-condition generalization, honest uncertainty and model discrepancy, independent feasibility checks, and improvement confirmed on manufactured non-flight coupons. No SOTA superiority claim is supported until the same tasks, compute budget, data splits, physical conditions and objective constraints are measured head-to-head.

## Parametric non-flight coupon geometry

The optional Rust binary named coupon_mesh creates a watertight, extruded, ground-test-only NACA 0012 coupon using the corrected scaled sharp-trailing-edge polynomial published by NASA TMR. It uses cosine chordwise spacing and performs an internal edge-use, directed edge-winding, unreferenced-vertex, and signed-volume audit before writing ASCII STL. The generated JSON manifest explicitly states that its dimensions are millimetres, its SHA-256 still needs to be computed over the exact output bytes, and the geometry is not approved for flight or any specific wind tunnel.

Example command (100 mm chord / 80 mm span are only CI smoke-test parameters, not recommended dimensions for a real facility):

`cargo run --locked --bin coupon_mesh -- --chord-mm 100 --span-mm 80 --chordwise 32 --spanwise 4 --out out/coupon.stl --manifest out/coupon.json`

For a real non-flight ground-test article, choose chord, span, blockage margin, support and measurement envelope only after the facility owner reviews the actual test section, Reynolds-number target and force-balance limits. The generator cannot assess those constraints. It does not add mounting hardware, internal structure, load qualification or finishing allowance. STL does not encode units, so retain the JSON manifest alongside the geometry. After generating a new STL and manifest, bind the exact STL bytes to the manifest with:

`bash tools/civil-aero-benchmark/scripts/finalize-coupon-manifest.sh out/coupon.stl out/coupon.json`

The script refuses mismatched, previously finalized, non-manifold or flight-authorized-looking manifests; it computes SHA-256 over the actual STL bytes, records the digest, and then prints digests for both STL and manifest. Preserve the printed receipt or commit it to the manufacturing evidence record.

The profile convention follows the corrected scaled sharp-TE equation on NASA TMR's validation page, but is generated analytically; it is not copied from the raw reference grid or experimental data. The source bytes and experimental data remain unadmitted until the AERO-DATA-001 gates pass.

## External SOTA comparison target: ShapeBench

Do not make a superiority claim from the NACA 0012 smoke test. The 2026 ShapeBench preprint describes 103 aerodynamic shape-optimization tasks across eight shape categories, with standardized budget accounting and comparison baselines spanning adjoint methods, L-BFGS-B, PSO, CMA-ES, Bayesian optimization, OpenEvolve, ShinkaEvolve and ShapeEvolve where applicable. Its reported optimizer rankings vary by task and budget, which is an explicit reason to test a diverse suite rather than extrapolate from one airfoil. See:

- ShapeBench paper: https://arxiv.org/abs/2605.20763
- Official code linked by the paper: https://github.com/ShapeBench/ShapeBench
- Dataset linked by the paper: https://huggingface.co/datasets/ShapeBench

The follow-on [AERO-SHAPEBENCH-001 issue #2896](https://github.com/Luminous-Dynamics/symtropy/issues/2896) defines the adapter. Before integration, review the official API, task/data terms, exact release revisions, dependencies, and budget rules. First reproduce a small, diverse smoke subset, then freeze a representative confirmatory subset and held-out geometry families. Compare the proposed Symthaea experiment-selection policy to official and conventional baselines under identical task definitions and matched total cost, including surrogate and high-fidelity evaluations, failed solves, tuning, orchestration and audit work. Only use official high-fidelity replay to confirm candidates where available.

A ShapeBench optimization win would support a claim about that measured optimization method/task suite—not by itself the physical validity of new Reynolds/Mach conditions, a manufactured article, structural adequacy or flight readiness. Keep NASA TMR solver validation, small-aircraft low-Re data, and the non-flight coupon measurements as distinct evidence classes.

## Deterministic stress-scenario screening

A third CLI evaluates each candidate against an explicit scenario CSV:

`cargo run --locked --bin robust -- data/example_cases.csv data/illustrative_stress_scenarios.csv`

It emits per-candidate/per-scenario lift and drag estimates, energy after reserve, energy margin, lift margin and separate screening flags. The summary reports the scenario producing the highest estimated energy, the lowest energy margin, the lowest lift margin, and counts of screening failures.

The committed stress set includes nominal, overweight, drag-model bias, lower density, lower propulsion efficiency, reduced battery energy, longer route and a combined adverse case. Every factor is visible in the scenario data file. The set is labelled synthetic_stress_not_probability. The multipliers are software stress fixtures, **not probabilities, not measured uncertainty bounds and not a real operating envelope**. Do not use them as aircraft engineering assumptions. Before evaluating a real candidate, replace or supplement them with bounds justified by actual material/build distributions, calibrated measurements, validated model discrepancy and the intended mission environment.

The negative control data file has a nominal estimate that fits its declared usable energy, but the explicit combined stress and lower-efficiency/energy cases expose failures. The CLI reports those failures; it does not convert a failed scenario into a fatal program error, because a stress report is useful evidence. The workflow asserts that the negative control is reported as not robust.

This is a deterministic scenario-sensitivity screen. It is not a Monte Carlo analysis, calibrated reliability estimate, probabilistic safety assessment, structural analysis or flight authorization.

## Pinned NASA TMR experimental bytes and parser contract

Three reference text artifacts are now retained in the repository under data/reference/nasa-tmr/ at the immutable upstream repository commit 20a39f549dbff988a6accc421ac84dafc4487695 (TMBWG/turbmodels). The upstream LICENSE at that commit states CC0 1.0 Universal. Their exact SHA-256 values and byte lengths are recorded in data/source_manifest.v0.1.json and data/reference/nasa-tmr/SHA256SUMS. The copied files retain the upstream Git blob IDs; no rows were regenerated or rounded during mirroring.

- CLCD_Ladson_expdata.dat — 1,343 bytes, SHA-256 78cd2f6aa4968e80f44cbf6c96f699bd9c6e45681d958ec528a10cf72ed23357. The source header says Ladson/NASA TM 4074 (1988), Re=6 million, M=0.15, transition tripped. The three 80/120/180 grit zones have 17/18/18 force observations respectively.
- CP_Ladson.dat — 6,331 bytes, SHA-256 547e022634a360cd7697813543348cd447ee0038cdbc8f00a5296c7822ec7af8. The file header says M=0.3 and contains nine zones spanning Re=3, 6 and 9 million with different transition descriptions; do not silently treat this as the M=0.15 tripped-force experiment.
- CP_Gregory_expdata.dat — 1,608 bytes, SHA-256 38d8474bc67661c3cef5e4cd9eeddb95d4a436d06abc1f58a8518066f510647c. The header says Re=2.88 million, digitized approximately from a photocopy, and upper surface only.

The strict Rust parser code is in src/reference_data.rs. It preserves each source zone and original line number, checks required source/variable headers, exact numeric column counts, non-finite values, invalid chord-normalized coordinates, duplicate zone labels and empty zones. Known-answer tests freeze the extracted row and zone counts. The reference_audit binary prints counts without promoting the content to qualified aerodynamics evidence. Run it from this package directory with cargo run --locked --bin reference_audit.

The CI workflow runs sha256sum -c against the pinned raw files before the parser-count check. This demonstrates retained-byte integrity and parser-structure agreement, not independent reproduction of the original experiment, source-authority authentication, CFD verification/validation or flight readiness. The three sources remain marked parser-review-pending in the evidence manifest until the exact-head Rust workflow passes and a separate review is recorded.

The corrected TMR profile-point file and grid archive remain separate source-admission items; do not conflate the geometry polynomial or these experimental datasets with a grid-convergence result.

## Deterministic normalized reference export

The reference_export binary turns the strict parser output into one consistent CSV schema, preserving dataset ID, exact source SHA-256, upstream Git revision, condition zone, and original source line. It leaves non-applicable measurement columns blank and emits source_parser_output_unqualified to make the evidence limit visible. The all-dataset mode produces one header followed by 531 observations; per-dataset modes preserve their own row counts.

Run from this package directory:

`cargo run --locked --bin reference_export -- all > reference-observations.csv`

The workflow checks the all-dataset output is deterministic across two invocations, verifies source-hash/revision binding in known rows, and checks per-dataset row counts. The normalized CSV is derived from the original source; the exact unmodified files remain authoritative and the export is not independent experimental data or an aerodynamic validation result.

## Matched-budget experiment cost accounting

The Rust budget auditor is in src/budget.rs with CLI src/bin/budget_audit.rs. Run it from this package directory:

`cargo run --locked --bin budget_audit -- data/decision_value/matched_budget_ledger.csv`

The ledger requires a fixed budget cap for every policy on the same task/replicate, records per-operation cost/wall/CPU time, binds successful outputs to SHA-256, and requires failure reason codes when an operation fails. Failed or aborted high-fidelity runs still count against the policy's total cost. The auditor rejects a policy overrun and rejects a comparison with only one policy. Actual spend may differ under a common cap; report remaining budget and quality-by-cost curves rather than forcing unused budget to count as consumed.

The committed ledger is synthetic. It charges a nonconverged high-fidelity run to the random baseline and demonstrates bookkeeping only. The CSV's cost units need a documented mapping to actual solver calls, compute, tuning, orchestration and independent-audit effort before a real ShapeBench or CFD comparison. The output label is ledger_audit_only, not aerodynamic validation or an optimizer win.

