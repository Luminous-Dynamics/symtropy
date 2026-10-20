# Physical solver benchmark manifest v0.1

Tracking: [PHYS-BENCHMARK-MANIFEST-001 #2358](https://github.com/Luminous-Dynamics/symtropy/issues/2358)

This is a provenance-aware catalog of candidate verification cases, not a record that Symtropy has passed those cases. The manifest deliberately separates cataloguing a benchmark from equation verification, numerical verification and experimental validation.

## Why these candidates

The first candidates span different physics families rather than only more examples of the same beam formula: plane truss response (SOFiSTiK BE1), beam stress/deflection (BE3), bar buckling (BE13), undamped SDOF vibration (BE23), truss snap-through (BE31), elliptic membrane plane stress (NAFEMS P18.LE1), Z-section torsion (P18.LE5), skew-plate bending (P18.LE6), and thermal continuum response (P18.LE11). See the upstream catalog URLs in the machine-readable manifest; exact benchmark files and redistribution rights must be checked before importing artifacts.

The first three Symtropy closed-form functions have topic-level relationships to some beam cases, but they are not declared benchmark-equivalent unless geometry, section, loading, boundary conditions, output definitions and tolerances are matched exactly.

## Status semantics

- catalogued: upstream candidate found; no Symtropy verification claim.
- inputs_unavailable / fixture_incomplete: evidence needed for faithful reproduction is absent.
- implemented_unverified: implementation exists but the benchmark has not passed its independent oracle.
- equation_verified: a pinned formula case passed a known analytical oracle and tolerances.
- numerically_verified: a solver/mesh case passed convergence, equilibrium and independent reference checks.
- experimentally_validated: predictions have been compared against traceable independent measurements within a declared test envelope.

The workflow forbids treating a literal PASS or a catalog entry as evidence. Moving a case to any verified status requires a pinned fixture, source digest, independent oracle, acceptance tolerance and exact-head test reference.

## Public reference context

- NAFEMS describes its standard benchmarks as independent tests intended to be more searching than vendor demonstration cases: https://www.nafems.org/publications/resource_center/p18/
- SOFiSTiK's 2026 catalog publishes a much wider range of static, nonlinear, buckling and dynamics reference cases, with input/reference artifacts listed on the catalog pages: https://docs.sofistik.com/2026/en/verification/index.html
- NIST recommends component-by-component validation using well-defined boundary conditions and separated validation/calibration data: https://tsapps.nist.gov/publication/get_pdf.cfm?pub_id=915449
- ASME lists V&V 10 (2019) and VVUQ 10.2 (2021) for computational solid mechanics and uncertainty quantification: https://www.asme.org/codes-standards/vvuq-standards

No proprietary benchmark bundle is copied into this repository by this metadata-only first slice.
