# DPW-8 / AePW-4 external reference intake v0.1

**Parent:** [AER-01 reference admission #4111](https://github.com/Luminous-Dynamics/symtropy/issues/4111)  
**Related structural bridge:** [#4093](https://github.com/Luminous-Dynamics/symtropy/issues/4093)  
**Related implementation:** [structural analytical oracle PR #4110](https://github.com/Luminous-Dynamics/symtropy/pull/4110)

## What this is

A versioned discovery catalog for public NASA/AIAA DPW-8 and AePW-4 reference resources, plus a fail-closed Python standard-library validator. It records the exact source URLs, listing facts, inspected text-preview facts, benchmark conditions, comparison outputs and unresolved terms/units/hash fields.

It is deliberately **not** an admitted benchmark bundle. No source file in v0.1 has been acquired byte-for-byte, hashed, licensed for redistribution or independently replayed. The catalog must not be described as a validated benchmark dataset.

## Important discoveries

### 1. OAT15A verification comes before large CRM FSI runs

The DPW-8 test matrix defines required Verification 1a for the ONERA OAT15A airfoil: steady CFD/RANS grid convergence and polar at Mach 0.73, chord Reynolds number 3 million, static temperature 271 K and angles of attack 1.36, 1.50, 2.50, 3.00 and 3.10 degrees. A six-member grid family is specified, with four members required and six desirable.

The NASA-hosted OAT15A experimental directory lists a 207 KB archive. Its archive contents have not yet been inspected. AIAA states ONERA retains intellectual property in the geometry and specifies attribution conditions; the supporting paper is separately restricted to personal use and may not be reposted or redistributed. Public download does not imply a redistribution licence.

### 2. The CRM Test Case 2a(E) static-FSI case is a concrete coupling benchmark

The AIAA static-deformation case sheet specifies:

- CRM wing/body, initialized from jig/unloaded (wind-off) geometry/grid;
- Mach 0.85, Reynolds number 5 million;
- dynamic pressure 1,384 psf;
- target lift coefficient 0.5000 ± 0.0001 (approximately 2.75 degrees angle of attack);
- static temperature 322.04 K;
- six grid levels, L1 Tiny through L6 Ultra-fine;
- forces/moments, sectional twist/deformation, sectional pressure coefficient, and separate flow/structural residuals;
- six normalized span stations: 0, 0.4286, 0.5546, 0.6773, 0.7954, 0.9150.

Test Case 2b(E) defines a static-FSI polar at the same Mach/Reynolds/dynamic-pressure reference, with angles -1.50, 0, 1.50, 2.50, 3.00, 3.50, 4.00 and 4.50 degrees.

The workshop data-submission page defines Tecplot-readable ASCII templates, includes bending/twist deformation output, notes that template version 3 normalizes deflection by reference semi-span, and specifies -999 for optional data not submitted. The actual template must be fetched and parsed before an importer is implemented; do not infer columns from this summary.

### 3. A small reference FEM model exists

The NASA directory lists an 8 KB `equivalent_beam_january_2025.bdf` under the CRM half-span FEM material. Its browser text preview shows SOL 101 and GRID/CBEAM/PBEAM/MAT1/CONM2/RBE2/SPC/FORCE cards, with 16 CBEAM elements and a 17-grid beam chain.

This is an attractive first parser/interop target because it is small. However, only its text preview has been inspected; exact bytes have not been acquired. The material/unit interpretation is unresolved, including the observed MAT1 E/G values and WTMASS field. No analysis result or model-validity claim is made.

## Files

- `dpw8-reference-catalog-v0.1.json`: known sources and case matrix.
- `verify_reference_catalog.py`: independent standard-library consistency checks and hostile mutations.
- `AER01_INDUSTRY_BENCHMARK_MATRIX_V0_1.md`: industry comparison, benchmark ladder, matched-budget protocol and internal target gates.

Run:

~~~sh
python3 tools/civil-aero-benchmark/dpw8-aepw4-reference-intake/verify_reference_catalog.py
~~~

The validator checks unique source/case IDs, HTTPS provenance, explicit acquisition status, SHA-256 requirements for exact bytes, non-admission of unresolved terms, grid-level ordering, normalized span-station ordering, cross-references and a few source-derived case invariants. It deliberately does not download, unpack, solve or certify anything.

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
