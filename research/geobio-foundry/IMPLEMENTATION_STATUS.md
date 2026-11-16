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
| `SUBTERRA_PROGRAMME_v0.1.md` | Planetary subterranean research programme and cross-body taxonomy |
| `SUBTERRA_SOTA_GAP_ANALYSIS_2026-11.md` | Current planetary cave science, robotics, communication and morphology baselines |
| `SUBTERRA_BENCHMARK_PROTOCOL_v0.1.md` | Evidence-separated subterranean exploration, mapping, communications and qualification benchmarks |
| `SUBTERRA_SITE_EVIDENCE_SCHEMA_v0.1.json` | Separate morphology, origin, connectivity, accessibility, engineering and preservation claims |
| `SUBTERRA_PROFILE_SCHEMA_v0.1.json` | Non-uniform profile intervals with explicit unobserved geometry |
| `fixtures/subterra-site-evidence-synthetic.json` | Synthetic Mars candidate record preserving unknown origin, connectivity, accessibility and use qualification |
| `fixtures/subterra-nonuniform-profile-synthetic.json` | Synthetic profile with a deliberately unobserved interval |
| `BIOASTEROID_REPLICATION_PROTOCOL_v0.1.md` | Reproduction-first biological/abiotic/hybrid protocol with process accounting, sample sizes and statistical significance controls |
| `PRESSUREFORGE_REFERENCE_PROTOCOL_v0.1.md` | Ideal-gas analytical reference assumptions, equations, exclusions, S0/S1 tests and physical qualification boundary |
| `RESOURCE_GRAPH_SCHEMA_v0.2.json` | JSON Schema for evidence-aware resource-process records |
| `fixtures/water-electrolysis-reference.json` | Synthetic stoichiometric/schema fixture; energy values are illustrative |
| `kernel/Cargo.toml` | Standalone, dependency-free Rust package with its own workspace |
| `kernel/src/lib.rs` | Elemental balance, uncertainty handling, evidence validation and uncertainty-propagated resource intensities |
| `kernel/src/pressure.rs` | Single-species ideal-gas compartment reference, prescribed mass transfer and observed-pressure comparison |
| `kernel/src/subterra.rs` | Non-uniform profile morphometry with observation coverage and uncertainty-aware weighted dimensions |
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
- Feedstock-origin and gross/recovered-energy intensity per kilogram of a nominated product, with first-order uncertainty propagation and separate standard-uncertainty fields.
- Shared correlation-group terms are carried through ratio derivatives; incomplete measurement uncertainty returns `None` rather than zero.
- Numerical overflow and non-finite input rejection; complete-composition reconciliation and resource-intensity ratio overflow guards.
- Bounded single-species ideal-gas model for rigid, isothermal compartments; prescribed gas-mass transfer; observed-pressure residual classification.
- Non-uniform subterranean profile summary with explicit unobserved intervals, coverage, length-weighted geometry, measurement uncertainty and variability.

## Validation evidence

| Check | Result | Boundary |
|---|---|---|
| GeoBio JSON Schema draft-2020-12 meta-validation | **PASS** | Python jsonschema `Draft202012Validator.check_schema` |
| Subterra site/profile schema meta-validation | **PASS** | Python jsonschema `Draft202012Validator.check_schema` |
| Subterra site/profile synthetic fixtures | **PASS** | Both fixture records validate against their respective schemas with format checking |
| Synthetic fixture against schema | **PASS** | Format checker enabled; zero validation errors |
| Negative control: missing uncertainty property | **REJECTED** | Schema correctly rejects |
| Negative control: malformed input digest | **REJECTED** | Schema correctly rejects |
| Negative control: unknown evidence state | **REJECTED** | Schema correctly rejects |
| Rust source and 32 unit tests (16 resource + 5 pressure + 11 Subterra) | **AUTHORED, NOT EXECUTED** | No `rustc` or `cargo` in the available execution environment |
| GitHub Actions / CI | **NOT CHECKED** | No runner wait or pass claim |
| Cross-field semantic validation for all site-state combinations | **NOT IMPLEMENTED** | JSON Schema validates fields and types; a semantic validator is still required to enforce state-transition dependencies |
| Physical, laboratory, field, orbital or flight test | **NOT PERFORMED** | No physical qualification claim |

## Known limitations

1. Correlation groups model perfect correlation within a group and independence between groups. A general covariance matrix, partial correlations and shared calibration covariance are not implemented.
2. Resource-intensity ratio uncertainty uses first-order derivatives and the simple correlation-group model. It is not a full covariance estimator and does not cover non-linear uncertainty, censored data, or model discrepancy.
3. Detection limits, censored values, elemental ontology/chemical speciation, isotopes, heat and exergy balances, water-specific accounting, product purity, downstream recovery, lifecycle cost and JSON deserialization remain future work.
4. The pressure module assumes a single ideal-gas species, fixed volume and uniform fixed temperature; it applies prescribed mass transfers and does not calculate leak conductance, compressible flow, heat transfer, structural stress or rock-mass interaction. It is an analytical bookkeeping reference only.
5. Subterra morphometry is a one-dimensional piecewise-constant profile summary, not a 3D interior reconstruction, cave-connectivity inference, geomechanics model or stability/safety assessment.
6. The Subterra site-evidence schema does not yet enforce all cross-field semantic rules; for example, a `directly_observed_connected` state must eventually require direct in-situ evidence and a consistent interior survey record.
5. The kernel is not a bioprocess model, geomechanics solver, pressure-vessel solver, ECLSS controller, or actuator.
6. Named correlation groups are a conservative simple model only for the stated assumption of perfect positive correlation; they are not a substitute for the covariance structure of a real analytical instrument or shared calibration chain.
7. The synthetic electrolysis fixture is not an empirical performance datum.

## Immediate next increments

1. Add schema-to-Rust deserialization only after choosing a minimal, reviewed serialization dependency policy.
2. Add profile-graph and branch/chamber topology representation, preserving unknown and unobserved geometry.
3. Replace the perfect-correlation group approximation with an explicit covariance/correlation representation and validate ratio uncertainty against Monte Carlo reference cases.
4. Add a separate, validated leak-flow reference model for a declared continuum or rarefied-flow regime, plus independent isolation-state tests.
5. Add blind multi-site synthetic cave geometry and sensor fixtures, then implement SubterraBench scoring.
6. Add ResourceForge test cases comparing an abiotic route, a biological route and a hybrid route on matched product and process boundaries.
7. Only after these reference cases are reproducible, propose production adapters for `symtropy-terrain`, `symtropy-basin`, Sol-Atlas site evidence and robotics interfaces.

No production Symtropy crate, root Cargo workspace membership, or runtime dependency was changed in this increment.
