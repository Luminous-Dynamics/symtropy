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

