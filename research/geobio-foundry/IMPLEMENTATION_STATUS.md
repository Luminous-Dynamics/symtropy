# GeoBio Foundry v0.2 — Implementation and Validation Status

**Repository:** Luminous-Dynamics/symtropy (public)  
**Branch:** `research/geobio-foundry-v0.2`  
**Date:** 2026-11-16  
**Production integration:** none; research-only package

## Artifacts committed

| Artifact | Purpose |
|---|---|
| `README.md` | Research index and evidence contract |
| `ARCHITECTURE_FIT_v0.2.md` | Maps GeoBio responsibilities onto existing Symtropy terrain, basin, physics and robotics components |
| `RESEARCH_PROGRAMME_v0.2.md` | Seven paradigm shifts, system architecture, work packages and demonstrator |
| `SOTA_GAP_ANALYSIS_2026-11-16.md` | Current public industry/research baselines and differentiation boundaries |
| `BENCHMARK_PROTOCOL_v0.2.md` | Evidence tiers, baselines, proposed gates, experimental design and failure dispositions |
| `RESOURCE_GRAPH_SCHEMA_v0.2.json` | JSON Schema for evidence-aware resource-process records |
| `fixtures/water-electrolysis-reference.json` | Synthetic stoichiometric/schema fixture; energy values are illustrative |
| `kernel/Cargo.toml` | Standalone, dependency-free Rust package with its own workspace |
| `kernel/src/lib.rs` | Elemental balance, uncertainty handling, evidence validation and nominal resource intensities |
| `kernel/README.md` | Kernel scope, command and limitations |

## Implemented in the reference kernel

- Typed evidence states, resource origins, material flows, elemental assays, energy and provenance.
- Fixed field units: kilograms and joules.
- Flow validation and complete-composition mass reconciliation.
- Element-by-element material balance.
- Uncertainty propagation for independent measurements and named fully correlated groups.
- Explicit unquantified status for missing uncertainty or unassayed elements.
- Process-level incomplete-inventory status; per-element passes cannot imply a complete inventory.
- Independent-witness requirement for independently reproduced and qualified claims.
- Gross energy and recovered energy reported separately.
- Nominal feedstock-origin and gross-energy intensity per kilogram of a nominated product.
- Numerical overflow and non-finite input rejection.

## Validation evidence

| Check | Result | Boundary |
|---|---|---|
| JSON Schema draft-2020-12 meta-validation | **PASS** | Python jsonschema `Draft202012Validator.check_schema` |
| Synthetic fixture against schema | **PASS** | Format checker enabled; zero validation errors |
| Negative control: missing uncertainty property | **REJECTED** | Schema correctly rejects |
| Negative control: malformed input digest | **REJECTED** | Schema correctly rejects |
| Negative control: unknown evidence state | **REJECTED** | Schema correctly rejects |
| Rust source and 11 unit tests | **AUTHORED, NOT EXECUTED** | No `rustc` or `cargo` in the available execution environment |
| GitHub Actions / CI | **NOT CHECKED** | No runner wait or pass claim |
| Physical, laboratory, field, orbital or flight test | **NOT PERFORMED** | No physical qualification claim |

## Known limitations

1. Correlation groups model perfect correlation within a group and independence between groups. A general covariance matrix, partial correlations and shared calibration covariance are not implemented.
2. Resource-intensity ratios are nominal point estimates; ratio uncertainty is not propagated. The report explicitly returns `uncertainty_propagated = false`.
3. Detection limits, censored values, elemental ontology/chemical speciation, isotopes, heat and exergy balances, water-specific accounting, product purity, downstream recovery, lifecycle cost and JSON deserialization remain future work.
4. The kernel is not a bioprocess model, geomechanics solver, pressure-vessel solver, ECLSS controller, or actuator.
5. The synthetic electrolysis fixture is not an empirical performance datum.

## Immediate next increments

1. Add schema-to-Rust deserialization only after choosing a minimal, reviewed serialization dependency policy.
2. Add uncertainty propagation for resource-intensity ratios and covariance-aware measurement models.
3. Add independent analytical pressure-compartment mass/leak/isolation reference equations.
4. Add deterministic synthetic geological fixtures with hidden regions and measurement provenance.
5. Add ResourceForge test cases comparing an abiotic route, a biological route and a hybrid route on matched product and process boundaries.
6. Only after these reference cases are reproducible, propose production adapters for `symtropy-terrain`, `symtropy-basin` and robotics interfaces.

No production Symtropy crate, root Cargo workspace membership, or runtime dependency was changed in this increment.
