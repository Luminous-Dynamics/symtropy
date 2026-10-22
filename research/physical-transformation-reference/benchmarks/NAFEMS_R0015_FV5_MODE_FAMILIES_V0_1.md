# NAFEMS R0015 FV5 — Deep Simply-Supported Beam Mode Manifest v0.1

**Tracking issue:** [#2620](https://github.com/Luminous-Dynamics/symtropy/issues/2620)  
**Benchmark ID:** `PHYS-NAFEMS-R0015-FV5-MODES-001`  
**Status:** reference-family manifest; **not an executed or passed FV5 solve**.  
**Model class required:** 3D beam dynamics with shear flexibility, torsion, axial extension, rotary inertia, repeated modes and explicit support freedoms.

## Why this benchmark follows NBC05

NBC05's two-degree-of-freedom axial-bar table is a useful oracle for one explicitly named abstraction. It cannot exercise shear-flexible bending, torsion, extensional modes, repeated eigenvalues, or 3D mode completeness. FV5 is designed to expose those omissions.

The official [Altair OS-V:0415 page](https://2024.help.altair.com/2024.1/hwsolvers/os/topics/solvers/os/nafems_test_problem_fv5_r.htm) describes FV5 as testing repeated eigenvalues, shear deformation, rotary inertia, and possible missing extensional modes. The [NAFEMS R0015 record](https://www.nafems.org/publications/resource_center/r0015/) says its collection contains roughly 30 free-vibration benchmarks, with geometry, boundary conditions, theoretical/numerical target frequencies and mode-shape schematics. The linked public Altair `fv5.fem` ZIP returned 404 during this research pass; the raw solver input has therefore **not** been ingested or independently inspected.

## Source ledger: keep model reference and solver outputs distinct

| Source | Values/evidence | How this benchmark uses it |
|---|---|---|
| NAFEMS FV5 theory table, reproduced in DIANA's [deep simply-supported beam verification page](https://manuals.dianafea.com/d1010/en/1353544-1353584-deep-simply-supported-beam.html) | Target frequency families below | Primary theoretical mode-family targets; actual model inputs still require source reconciliation |
| DIANA FV5 page | Separate five-element FE results: 42.626, 70.768, 125.00, 148.58, 212.52, 288.75 Hz for the six frequency families | Cross-solver comparator only. Do not treat these results as the theory targets |
| Altair [OS-V:0415 page](https://2024.help.altair.com/2024.1/hwsolvers/os/topics/solvers/os/nafems_test_problem_fv5_r.htm) | Five 2-node beam elements; support freedom description; E=200 GPa, nu=0.3, density=8000 kg/m³; target frequencies and rendered normalized values | Independent source attribution. The HTML extraction does not unambiguously associate every normalized number with a specific row; retain but do not use those numbers as a pass/fail oracle until the source table/raw input is reviewed |
| FERS [public FV5 reproduction](https://ferscloud.com/nafems) | Secondary description reports L=10 m, circular section diameter about 2.28 m, shear coefficient kappa=5/6, and translational plus rotary-inertia model details | Discovery/triangulation only. This is close to a 2×2 m square section in area/inertias, but the shape/model identity must remain explicit |
| Ansys [LS-DYNA VM-LSDYNA-SOLVE-064](https://ansyshelp.ansys.com/public/views/secured/corp/v261/en/lsdyna_vm/vm-lsdyna-solve-064.html) | 10 m beam, 2×2 m section, E=200 GPa, nu=0.3, density=8000 kg/m³, explicit supports, six target mode families and six LS-DYNA output values | A separately frozen vendor-published target variant. Its torsional targets (71.200 and 213.610 Hz) differ materially from the DIANA-reproduced R0015 theoretical set (77.542 and 233.100 Hz); do not merge the two into one score |

The Rust reference crate represents the R0015/DIANA-theory and Ansys LS-DYNA target lists as distinct `Fv5ReferenceVariant` values in `src/nafems_fv5_manifest.rs`. Its comparison function pairs modes by stable family ID and checks family kind/multiplicity before reporting relative target discrepancies; it does not treat a sorted frequency index as a family identity. Tests explicitly preserve both target sets and require the torsional target differences to remain above 8%, while flexural mode 1 and extensional targets remain close. This is a literature-interpretation guard, not a numerical solve.

The six theoretical frequency families reproduced by DIANA's FV5 page are:

| Family | Multiplicity | Theoretical target (Hz) | DIANA five-element result (Hz) |
|---|---:|---:|---:|
| Flexural 1 | 2 | 42.649 | 42.626 |
| Torsional 1 | 1 | 77.542 | 70.768 |
| Extensional 1 | 1 | 125.000 | 125.000 |
| Flexural 2 | 2 | 148.310 | 148.580 |
| Torsional 2 | 1 | 233.100 | 212.520 |
| Flexural 3 | 2 | 284.550 | 288.750 |

### Important: FV5 is a benchmark family with source variants

The [Ansys LS-DYNA verification page](https://ansyshelp.ansys.com/public/views/secured/corp/v261/en/lsdyna_vm/vm-lsdyna-solve-064.html) explicitly gives a 10 m beam and a 2×2 m section, with E=200 GPa, nu=0.3, density=8000 kg/m³, A restraints (u_x=u_y=u_z=R_x=0), and B restraints (u_y=u_z=0). Its published NAFEMS-target/LS-DYNA pairs are:

| Family | Multiplicity | LS-DYNA page target (Hz) | LS-DYNA result (Hz) | Result error published by Ansys |
|---|---:|---:|---:|---:|
| Flexural 1 | 2 | 42.650 | 42.642 | 0.019% |
| Torsional 1 | 1 | 71.200 | 70.735 | 0.653% |
| Extensional 1 | 1 | 125.000 | 124.968 | 0.026% |
| Flexural 2 | 2 | 148.150 | 148.000 | 0.101% |
| Torsional 2 | 1 | 213.610 | 211.769 | 0.862% |
| Flexural 3 | 2 | 283.470 | 282.405 | 0.376% |

These are *not identical* to the theoretical reference list reproduced in the DIANA FV5 page. Most notably, the first torsional target differs by roughly 8.2% and the second by roughly 8.4%. The sources may differ in variant, section/mass interpretation, beam theory or target provenance; the available public pages do not justify picking one explanation. Accordingly, the fixtures are separate:
- `nafems_r0015_fv5_mode_manifest_v0_1.txt`: the R0015/DIANA-reproduced theoretical family;
- `nafems_fv5_lsdyna_variant_v0_1.txt`: the Ansys LS-DYNA VM-LSDYNA-SOLVE-064 variant.

The section is specified as 2×2 m in the Ansys variant, while FERS reports an approximately 2.28 m circular section, close in area and second moments but not the same geometry. A reproducible solver comparison must pin the actual (A), (I_y), (I_z), (J), shear areas and mass terms rather than substituting one shape for another. Keep solver outputs, theoretical targets, and geometric/model interpretations in separate evidence fields. Never average variant targets to create an invented “gold” answer.

The 9 modes are not just nine ascending scalar frequencies. The first and second flexural families are repeated pairs; benchmark comparison must preserve pair identity and compare the associated invariant subspaces. A sorted-frequency-only score can silently misclassify mode crossing, mode swapping, or mixing within a repeated eigenspace.

## Inputs that can be frozen now

Source-derived values documented in Altair's page:

- Young's modulus: 200 GPa.
- Poisson ratio: 0.3.
- Density: 8000 kg/m³.
- Discretization: five 2-node beam elements for the published model.
- End A: restraints (u_x=u_y=u_z=	heta_x=0).
- End B: restraints (u_y=u_z=0); axial displacement and the rotation about the beam axis remain unrestrained under the page's description.
- No external load; undamped free-vibration/eigenvalue analysis.

Do **not** close the input manifest until these unresolved fields have a source-quality answer:

- For the R0015/DIANA theoretical variant, the original source-authoritative cross-section and exact raw input still need reconciliation. The Ansys LS-DYNA variant documents a 2×2 m section; the FERS circular-section geometry is a third-party alternative rather than an interchangeable input.
- For the selected model variant, record area (A), second moments (I_y,I_z), torsional constant (J), shear areas, shear coefficient and radius of gyration explicitly.
- Element theory (Euler-Bernoulli vs Timoshenko), shear correction model, and translational/rotary terms in the mass matrix.
- Precise interpretation of Altair's rendered normalized factors.
- The exact raw `fv5.fem` input remains unavailable from the linked Altair archive (404); Ansys exposes a separate LS-DYNA input-download link, which has not yet been retrieved or ingested.
- Exact raw `fv5.fem` input, redistribution/licensing conditions, solver version and eigensolver controls.
- Whether the comparison is theory-to-model, Altair-to-model or DIANA-to-model. These are separate evidence rows, not interchangeable labels.

The approximate circular geometry and kappa=5/6 in the FERS reproduction are helpful leads, but they are not adequate authority for a strict benchmark acceptance gate. If the exact geometry or mass formulation is unknown, report `not-qualified:missing-reference-input` rather than pass.

## Verification ladder

### F0 — frozen reference manifest

Pin the mode-family table, source URLs, source retrieval date, provenance class, known solver outputs, unresolved input fields and claim ceiling. Hash the canonical fixture bytes. No numerical comparison is claimed at F0.

### F1 — physical mode-family completeness

For a future 3D model, verify:
- all nine target modes appear in the appropriate frequency interval;
- repeated flexural pairs are preserved and compared as mass-weighted subspaces/principal angles or cluster MAC;
- torsional and extensional families are explicitly identified, not merely assigned by index;
- mass and stiffness matrices are symmetric on the stated DOF space;
- each generalized eigenpair has a normalized residual, and modes have documented mass normalization/orthogonality;
- missing rigid/mechanism modes, if any, are detected and classified instead of silently filtered out.

### F2 — accuracy and mode pairing

Do not set a universal one-percent tolerance before resolving the model. The source family includes commercial/finite-element results that differ by mode family. Pin one of three distinct comparators per claim: theoretical target, named solver result, or independently derived reference. Report normalized errors per family and multiplicity, subspace angle/MAC, residual and uncertainty separately.

### F3 — convergence and reproducibility

Use the published five-element discretization as the first comparison only after the raw input and element formulation agree. Then refine element counts and show frequency convergence plus mode-family tracking. Receipts bind exact source commit, canonical request digest, solver backend/version, element formulation, supports, mass model, full returned mode set, mode pairing, residuals, and claim ceiling.

### F4 — external and physical validation

A cross-solver agreement is numerical corroboration, not experimental validation. Physical validation requires an identified experiment, known material/geometry/support uncertainty, uncertainty bands for the observed frequencies and mode shapes, and a predeclared validation metric. ASME [V&V 10-2019 (R2025)](https://webstore.ansi.org/standards/asme/ASME102019R2025) is the current cited framework for verification and validation in computational solid mechanics; keep verification, validation and uncertainty assessment as separate claims.

## Explicit claim ceiling

This manifest is a research/benchmarking artifact. It does not mean Symtropy has reproduced FV5. The current element/assembly development and the NBC05 axial-bar table result do not by themselves qualify a 3D Timoshenko beam, torsional modes, extensional modes or the full FV5 spectrum. Do not claim state-of-the-art superiority, physical validation, structural safety or code compliance based on this manifest.
