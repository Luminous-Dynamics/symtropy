# DPW-8 ONERA OAT15A protocol manifest v0.1

## Scope and evidence ceiling

This manifest freezes the public DPW-8 Verification 1 protocol facts for three separate lanes:
- V1a: steady grid-convergence/polar;
- V1b: unsteady extended polar/buffet;
- V1c: scatter-reduction reprise.

These are not interchangeable. A V1a result does not establish unsteady buffet behaviour. A V1c reproduction is not a generic substitute for either V1a or V1b.

The Rust module `src/dpw8_protocol_manifest.rs` encodes these lane definitions and has a Python standard-library oracle at `scripts/verify-dpw8-protocol-independent.py`. The Rust test calls that oracle for all three lanes.

Current source entries are explicitly `not_admitted`: the module starts from official source URLs but does not claim that the live pages, grid bytes, geometry or experimental data have been snapshotted, hashed, licensed for redistribution, or independently admitted. It is a protocol-spec contract, not a solver run or a validation result.

## V1a — steady grid convergence and polar

Official base flow conditions:
- Mach 0.73;
- chord Reynolds number 3,000,000;
- static temperature 271 K.

Official angle-of-attack list:
`[1.36°, 1.50°, 2.50°, 3.00°, 3.10°]`.

The protocol specifies a six-member grid family: four levels are required and six are desirable. Periodic sidewall boundary conditions are specified. Committee-supplied grids are encouraged; user-generated grids are allowed under the workshop guidelines.

V1a is steady CFD (for example RANS). Multiple turbulence models may be submitted. Turbulence model choice must be an explicit benchmark lane attribute rather than silently inferred from a solver default.

## V1b — unsteady extended polar / buffet

V1b is explicitly unsteady and has its own angle-of-attack list:
`[3.25°, 3.40°, 3.50°, 3.60°, 3.90°]`.

The official page lists the same six grid levels. V1b must additionally qualify time-step sensitivity, windowed mean-force stability, sample-length sufficiency, spectral resolution, and any declared buffet metric before the relevant unsteady claim can be admitted.

A steady residual pass or V1a grid-convergence report cannot be reused as V1b temporal/statistical convergence evidence.

## V1c — scatter-reduction reprise

The public V1c subsection prescribes:
- updated grid families with a common farfield 1000 chords away;
- French-Vanilla SA-negative, all terms;
- Sutherland's viscosity law;
- adiabatic (not isothermal) wall;
- residual convergence to approximately 1e-10;
- gamma 1.4, Prandtl 0.72, turbulent Prandtl 0.9;
- farfield chi = nu / nu-tilde = 3.

The currently admitted public V1c subsection does **not** provide a separate angle-of-attack schedule or restate its base Mach/Reynolds/temperature in that subsection. Therefore the v0.1 protocol manifest preserves the AoA schedule and those base flow values as unspecified rather than copying the V1a or V1b list by inference. A later, byte-pinned committee protocol or source file may supply them under a new manifest revision.

V1c's "updated grid family" is deliberately not treated as an ordinary six-member family until the exact V1c grid definition is byte-admitted.

## Geometry attribution and source use

The geometry must be referred to as **ONERA OAT15A**. ONERA retains the geometry intellectual property. ONERA must be acknowledged each time the geometry is shared with a partner, and the sharing conditions must accompany it.

The experimental reference paper is for personal use and must not be reposted or redistributed without permission. Treat geometry bytes, grid bytes, experimental data and papers as distinct source artifacts with distinct admission and redistribution states. A SHA-256 digest does not grant a license.

## Source-admission states

Every required source begins as `not_admitted`. A byte-pinned reference-only state records exact length, SHA-256 and revision while retaining redistribution restrictions. A byte-pinned redistributable state is only valid if the actual permission basis has been separately established. Permission-pending bytes may be hashed for internal audit but cannot be treated as admitted or redistributable.

Source URL presence is not source admission. A mutable web directory is not a pinned grid. Every grid member and experimental data artifact needs its own byte digest, source revision and applicability/permission decision.

## Independent replay

The independent Python oracle recomputes the canonical manifest digest, checks lane-specific values, verifies source admission metadata, and rejects:
- V1a/V1b angle-list substitution;
- V1c inheriting an inferred AoA list;
- V1c scatter settings missing or changed;
- incomplete byte-pinned metadata;
- omitted required source references;
- changed ONERA designation/attribution/IP conditions;
- tampered canonical manifest identity.

The fixtures are synthetic protocol definitions. The oracle agreement is a finite software-contract result, not proof that the public sources are authentic or physically correct.

## Advancement gates

Before any real run:
1. Snapshot and hash the exact official test-case page and geometry policy revision.
2. Admit the exact selected OAT15A geometry/IGES artifact under the ONERA sharing conditions.
3. Admit each chosen committee or user-generated grid separately, with grid family and refinement lineage.
4. Admit experimental files and their uncertainty/data-processing provenance separately from the geometry and grids.
5. Freeze all physical and solver settings and protocol revisions.
6. Bind each production run to the corresponding lane and operating point.
7. Require V1a spatial convergence, V1b temporal/statistical convergence, and V1c scatter protocol conformance as separate claims.
8. Preserve historical participant reproduction, current pinned SU2, and any alternate solver/build as separate lanes.

## Official sources

- DPW-8 test cases: https://www.aiaa-dpw.org/TestCases/testcases.html
- ONERA geometry/IP policy: https://www.aiaa-dpw.org/geometry.html
- DPW-8 ONERA OAT15A grids/data index: https://dpw.larc.nasa.gov/DPW8/ONERA_OAT15A/
- DPW-8 root directory: https://dpw.larc.nasa.gov/DPW8/
- ONERA OAT15A experimental references and paper-use condition: https://www.aiaa-dpw.org/experiment.html
- AePW-4 separate aeroelastic case family: https://nescacademy.nasa.gov/workshops/AePW4/public

## Relationship to related work

- #3500: DPW-8 OAT15A source admission and lane-separated case manifest.
- #3413: protocol-aware SU2 run bundle and claim admission.
- #3495: applicability domains by model, quantity, geometry family and flow regime.
- #3494: source-built SU2 discrete-adjoint baseline.
- #3498: measured resource evidence and matched-budget accounting.
- #2890: decision-value experiment selection.
