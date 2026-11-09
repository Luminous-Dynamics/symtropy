# DPW-8 / AePW-4 external reference intake v0.2

**Parent:** [AER-01 reference admission #4111](https://github.com/Luminous-Dynamics/symtropy/issues/4111)  
**Related structural bridge:** [#4093](https://github.com/Luminous-Dynamics/symtropy/issues/4093)  
**Related implementation:** [structural analytical oracle PR #4110](https://github.com/Luminous-Dynamics/symtropy/pull/4110)

## What this is

A versioned discovery catalog for public NASA/AIAA DPW-8 and AePW-4 reference resources, plus a fail-closed Python standard-library validator. It records the exact source URLs, listing facts, inspected text-preview facts, benchmark conditions, comparison outputs and unresolved terms/units/hash fields.

It is deliberately **not** an admitted benchmark bundle. No source file in v0.2 has been acquired byte-for-byte, hashed, licensed for redistribution or independently replayed. The catalog must not be described as a validated benchmark dataset.

## Important discoveries

### 1. OAT15A verification comes before large CRM FSI runs

The DPW-8 test matrix defines required Verification 1a for the ONERA OAT15A airfoil: steady CFD/RANS grid convergence and polar at Mach 0.73, chord Reynolds number 3 million, static temperature 271 K and angles of attack 1.36, 1.50, 2.50, 3.00 and 3.10 degrees. A six-member grid family is specified, with four members required and six desirable.

The NASA-hosted OAT15A experimental directory lists a 207 KB archive. Its archive contents have not yet been inspected. AIAA states ONERA retains intellectual property in the geometry and specifies attribution conditions; the supporting paper is separately restricted to personal use and may not be reposted or redistributed. Public download does not imply a redistribution licence.

### 2. Keep rigid-CFD and elastic-FSI cases separate

The current DPW-8 Test Cases page defines **rigid Configuration B** at Mach 0.85, chord Reynolds number 5 million and 100°F (310.928 K). Its rigid 2a case is a six-grid convergence study at alpha=2.50 degrees. Its rigid 2b sweep uses alpha=[-1.50, 2.50, 2.75, 3.00, 3.25, 3.50, 3.75, 4.00, 4.25] degrees with grid level 3 or 4.

The separate AIAA Static Deformation Working Group PDF defines the **elastic FSI** cases at 120°F (322.04 K), not 100°F:

- **2a(E), Wing/Body Deformation (cruise):** Mach 0.85, Reynolds number 5 million, dynamic pressure 1,384 psf, CL=0.5000 ± 0.0001 (approximate alpha 2.75°), 120°F, jig/unloaded geometry, grid levels 1–6.
- **2b(E), Wing/Body Deformation (polar):** Mach 0.85, Reynolds number 5 million, dynamic pressure 1,384 psf, alpha=[-1.50, 0.00, 1.50, 2.50, 3.00, 3.50, 4.00, 4.50] degrees, 120°F, jig/unloaded geometry, grid level 3.

The rigid and elastic cases have separate AIAA case-sheet PDFs and must remain separate records. The apparent geometry wording discrepancy from earlier notes came from conflating the rigid 2a case sheet (which uses deformed-wing matching geometry) with elastic 2a(E) (which specifies jig/unloaded geometry). The current test-case page and the dedicated elastic PDF agree that the elastic FSI cases start from jig/as-built or unloaded geometry; record the exact geometry artifact and revision in each run manifest.

The static-FSI PDF lists comparison data from NTF197, NTF215, NTF229, ETW ESWIRP and Ames216, and comparison metrics including forces/moments, sectional Cp, sectional twist/deformation and separate flow/structural residuals. Do not import the rigid-CFD nine-point alpha schedule into the elastic 2b(E) polar.
The workshop data-submission page defines Tecplot-readable ASCII templates, includes bending/twist deformation output, notes that template version 3 normalizes deflection by reference semi-span, and specifies -999 for optional data not submitted. The actual template must be fetched and parsed before an importer is implemented; do not infer columns from this summary.

### 3. A small reference FEM model exists

The NASA directory lists an 8 KB `equivalent_beam_january_2025.bdf` under the CRM half-span FEM material. Its browser text preview shows SOL 101 and GRID/CBEAM/PBEAM/MAT1/CONM2/RBE2/SPC/FORCE cards, with 16 CBEAM elements and a 17-grid beam chain.

This is an attractive first parser/interop target because it is small. Its text preview has been inspected, but exact bytes have not been acquired. A direct HTTPS retrieval attempt on 8 November 2026 returned no bytes; the failed attempt is recorded in the machine-readable catalog, with no fabricated digest and no claim that the NASA resource is unavailable or corrupt. The NASA-hosted URL is also not accessible through the web text-opening interface, and a filename search found only the official directory listing.

The material/unit interpretation is unresolved, including the observed MAT1 E/G values and WTMASS field. No analysis result or model-validity claim is made. The parser PR #4142 uses only a synthetic fixture. It must not be applied to the NASA model as if its card dialect were already verified.

### 4. Optional dynamic CRM FSI Test Case 3: preserve the measurement operators

The DPW-8 Buffet Working Group identifies Configuration A Test Case 3 as an **optional** unsteady CFD + dynamic-wing fluid/structure-interaction case. The public AIAA test matrix specifies Mach 0.85, chord Reynolds number 2.27 million, static temperature 326.15 K, total pressure 12.0 kPa and angles of attack 3.05°, 3.61° and 4.70°. It calls for an unloaded (wind-off) CFD/FEM start, full- and half-span stick FEMs, six grid levels, and resolution of wing structural dynamics/spectra.

JAXA's 2.16%-scale CRM data page exposes several measurement families. They are different observation operators, not interchangeable rows of one response vector:

| Measurement family | Published content | Required handling |
|---|---|---|
| Wind-tunnel balance | Corrected CL/CD/Cm with PSP-simultaneous vs standalone balance (BL) tags | Preserve each row's provenance; PSP-simultaneous CD is N/A and must remain missing, not imputed |
| Wing-root strain gauge | Buffet intensity coefficient C_B | Preserve definition: RMS signal at alpha divided by zero-lift RMS at alpha=-0.8°; it is an onset indicator, not a full structural mode/deformation field |
| ESP | Time-averaged point Cp | Use sensor coordinates/IDs from the paired CSV |
| Kulite | Point Cp RMS and sensor-specific PSD | Link each trace/PSD to sensor position through the ESP file; the page flags a PSD peak near 9 kHz as wind-tunnel noise |
| Unsteady PSP (uPSP) | Time-averaged/RMS surface Cp, surface PSD and snapshots | Retain CRM jig-shape mapping, MAC rescaling, masks and camera-exposure low-pass effects; do not equate uPSP RMS directly with Kulite RMS |
| Structural deformation | Official deformation data pending | The only stated 10–12 mm tip deflection at alpha=4.7° is preliminary qualitative information, not a calibrated validation target |

The JAXA page carries a copyright notice ("All Rights Reserved"). Its links are public, but the v0.2 catalog marks terms and redistribution as review-required; no CSV/ZIP bytes have been acquired or hashed. Do not redistribute the source data or derived datasets until the rights and permitted uses are reviewed.

**Do not inherit Test Case 2 spectral settings into Test Case 3.** The exact time step, sampling rate, record length, window, segment length, overlap, PSD normalization, sensor synchronization and calibration must be read from the Test Case 3 files or controlling procedure. The #3071 unsteady evidence owner already requires those to be explicit; this catalog does not implement another Welch PSD engine.

The dynamic-FSI benchmark must keep separate outputs for force/balance, buffet C_B, ESP means, Kulite RMS/PSD, uPSP mean/RMS/PSD, and structural dynamics. A successful pressure-spectrum reproduction cannot be promoted into measured wing deformation validation while the official deformation data remain pending.

## Files

- `dpw8-reference-catalog-v0.2.json`: source and benchmark-case catalog (17 source records, 7 benchmark cases; all downloaded artifacts remain unadmitted).
- `verify_reference_catalog.py`: independent standard-library consistency checks and hostile mutations, including Test Case 3 measurement-operator and evidence-boundary checks.
- `AER01_INDUSTRY_BENCHMARK_MATRIX_V0_1.md`: industry comparison, benchmark ladder, matched-budget protocol and internal target gates.

Run:

~~~sh
python3 tools/civil-aero-benchmark/dpw8-aepw4-reference-intake/verify_reference_catalog.py
~~~

The validator checks unique source/case IDs, HTTPS provenance, explicit acquisition status, SHA-256 requirements for exact bytes, non-admission of unresolved terms, grid-level ordering, normalized span-station ordering, cross-references, source-derived case invariants, optional Test Case 3 status, modality separation, missing-data semantics and the official-deformation-pending boundary. It deliberately does not download, unpack, solve or certify anything.

## Next engineering sequence

1. Acquire and SHA-256 the 8 KB equivalent-beam BDF and its surrounding README; preserve the raw file and retrieval receipt.
2. Write a fixed-field/continuation-aware NASTRAN bulk-data reader for the admitted subset. Do not split BDF cards on whitespace: compact fixed-width fields and continuation lines are present.
3. Validate syntax/topology only: unique IDs, GRID references, CBEAM endpoints, PBEAM/MAT1 references, RBE2 dependencies, SPC/FORCE references and unit metadata. Preserve raw card bytes and line locations.
4. Acquire the small ONERA OAT15A archive only after terms review; inspect its archive member inventory safely; separate measurements, coordinates, IGES geometry and papers.
5. Start with OAT15A Verification 1a at L1-L4 if terms and mesh/case compatibility permit; retain L5/L6 as desirable sensitivity levels rather than a gate to initial progress.
6. Then use CRM Test Case 2a(E) and the small equivalent-beam / half-span / full-span model hierarchy to qualify state mapping and static-FSI convergence. A workshop reproduction must compare the prescribed metrics and keep grid, structural and coupling residuals separate.
7. Only after that consider the Test Case 4 wing/body/nacelle/pylon grid family. Its six archives are listed at 361 MB, 834 MB, 1 GB, 5 GB, 12 GB and 33 GB; do not download them as a first experiment.

## Claim ceiling

The DPW8 CRM is a transonic transport research model. It can qualify methods, data plumbing, solver reproducibility and coupling for the exact workshop case. It does not validate low-speed Wren 2 or Wayfarer 4 aircraft. OAT15A is an airfoil verification case, not an aircraft structural validation case. The NASTRAN preview is not a reproduced structural solution. No item here establishes structural adequacy, flutter clearance, fatigue life, airworthiness, certification, or permission to manufacture/test/fly.
