# Engineering Scenario DSL v0.1 — design contract

**Status:** Draft design / authoring contract; no parser, compiler, or execution support is claimed  
**Issue:** [ENG-SCENARIO-DSL-001 #2985](https://github.com/Luminous-Dynamics/symtropy/issues/2985)  
**Parent substrate:** [ENG-SYNTH #1172](https://github.com/Luminous-Dynamics/symtropy/issues/1172)  
**Credibility boundary:** [ENG-SYNTH-CRED #1173](https://github.com/Luminous-Dynamics/symtropy/issues/1173)

## 1. Decision

Define a small declarative **Engineering Scenario Language (ESL)** as an authoring layer for bounded engineering runs.

The language compiles to a versioned typed scenario representation and a deterministic plan. It does not execute a solver, certify a model, authorize an operation, prove a physical claim, or replace a domain solver.

The intended pipeline is:

```text
TOML authoring source
  -> strict TOML parse
  -> structural/schema validation
  -> typed scenario IR
  -> dimensions, ranges, references, and temporal semantics
  -> model/solver registry resolution
  -> ENG-SYNTH capability and context-of-use checks
  -> deterministic plan + diagnostics
  -> separately admitted execution
  -> existing replay/run-health/evidence paths
```

The first authoring frontend is TOML 1.0 because Symtropy's current NACA0012 reference case already uses TOML. The normalized interchange representation is JSON. Do not let TOML's convenience syntax become an implicit semantic model.

## 2. Explicit ownership boundaries

| Concern | Owner |
|---|---|
| Authoring syntax, typed scenario fields, diagnostics, plan compilation | ESL |
| Deterministic physics execution, runtime capabilities, stable runtime identity, replay capture | ENG-SYNTH #1172 |
| Context-of-use, model profile, numerical-integrity policy and suitability semantics | ENG-SYNTH-CRED #1173 and #1468–#1477 |
| Exact evidence status, validation receipts, limitations and claim ceiling | Symthaea LEP / SE-VV |
| Principal/action/resource authorization and policy integrity | Mycelix |
| Solver implementation and domain-specific numerical validity | Registered solver/provider |

The DSL may call the contracts owned by these systems. It must not copy their authority or create parallel versions of their semantics.

## 3. Language design

### 3.1 Declarative, bounded, capability-safe

v0.1 must not support:

- shell commands or arbitrary program text;
- `eval`, embedded expressions, macros, or runtime code generation;
- environment-variable interpolation;
- remote `include` / `import` or network retrieval;
- dynamic plugin loading from a scenario file;
- implicit filesystem access or side effects;
- undeclared defaults that materially alter a physical model.

A manifest can name a registered solver adapter. It cannot provide a command line or executable path that bypasses the registry.

### 3.2 Typed core

Every scenario must identify:

- schema/version and stable scenario ID/revision;
- purpose, decision supported, domain and consequence class;
- a bounded context-of-use and validity envelope;
- exact model and solver/adapter identities, or an explicit unresolved reference;
- typed parameters and input-artifact provenance;
- initial and boundary conditions;
- reference frames and stable entity IDs where applicable;
- disturbances/interventions with explicit time semantics;
- named observables, units, frames, sampling/window rules and tolerances;
- numerical method, timestep, stop conditions, determinism profile and resource budgets;
- limitations, unsupported semantics and the current claim ceiling.

A generic field called `valid`, `verified`, or `safe` is not part of the language.

### 3.3 Quantities and units

A physical quantity is represented in the normalized IR as:

```json
{ "value": 2.13163, "unit": "kg/m^3" }
```

The authoring surface may use readable field names such as `density_kg_m3`, but the compiler must normalize the value and unit separately.

Rules:

- every physical quantity has a finite numeric value and an explicit recognized unit;
- every unit maps to a known dimension;
- incompatible dimensions are compile errors, not conversions guessed from field names;
- conversions preserve the authored value/unit in source provenance and store a normalized typed value for calculation;
- unitless values such as Reynolds number, node counts, and (y^+) are explicitly dimensionless/count types, not silently treated as physical quantities;
- NaN, positive/negative infinity, overflow, and invalid ranges are rejected.

Use an established Rust dimensional-analysis library only after assessing its API, lockfile impact, and compatibility with the domain's exact unit needs. Do not invent a bespoke general-purpose unit algebra in the first tranche.

### 3.4 Exact model and artifact identity

A model name is not an executable identity. Model resolution should include, where available:

- stable model registry ID and version/revision;
- provider/adapter ID and version;
- solver executable/build digest;
- physics formulation and enabled submodels;
- model parameter/configuration digest;
- supported dimensions, variables, units and boundary-condition kinds;
- known approximations and unsupported behaviors.

Reference artifacts must distinguish immutable upstream Git object identity (including its hash algorithm) from a SHA-256 digest of the exact retrieved bytes. A Git blob SHA-1 is not a SHA-256 checksum.

If an exact solver binary/version or required input artifact is absent, report an unresolved/unsupported capability. Do not resolve it from a mutable `master` branch at compile or execution time.

### 3.5 Context of use is not inferred

The compiler validates the presence and internal structure of context-of-use information, then delegates the suitability semantics to ENG-SYNTH-CRED.

At minimum the author must state purpose, decision supported, domain, configuration/environment/range scope, consequence class, and known exclusions. An engine feature list is not a model-credibility assessment.

### 3.6 Plan result is not an execution result

Compilation returns one of three high-level dispositions:

| Disposition | Meaning |
|---|---|
| `Invalid` | Syntax, structure, type, dimension, identity, or semantic contract failed |
| `Blocked` | The input is well-formed but requested capabilities/conditions are unsupported, unresolved, or outside the declared envelope |
| `PlanReady` | The named requirements resolve to available capabilities and a deterministic plan can be emitted |

Per-requirement resolution uses only:

- `SupportedExact`;
- `SupportedApproximation(policy_id)`;
- `Unsupported(reason)`.

A `PlanReady` result is **not** permission to execute. Admission is a separate operation that re-resolves capabilities and authorizes the exact plan. A completed run is a later, separate fact.

## 4. Identity and determinism

Retain two different commitments:

1. **Source digest:** digest of the exact authoring bytes. It preserves what the author supplied, including comments/formatting and any source-level ordering.
2. **Semantic-plan digest:** digest of the normalized typed IR plus the resolved model/solver/configuration identities and all execution-relevant settings.

This distinction means harmless TOML formatting changes can preserve the semantic-plan digest without erasing the exact source identity.

Canonicalization must be versioned and selected once for the cross-project evidence contract. Before using a digest for signatures or attestations, publish cross-language golden vectors and verify them with an implementation independent of the producer. Define exactly which arrays are ordered and which collections are sets; never sort a semantically ordered schedule.

A fixed source + schema/compiler version + registry snapshot + declared seed/time policy must produce byte-identical normalized plans. If registry resolution changes, the plan identity must change or resolution must fail.

## 5. First real input: NACA0012 RANS reference case

The current main-branch file is:

- `benchmarks/aero/naca0012-inc-rans/case.toml`
- source blob at the time of this design: `c11a1af5f2d47bdeaf235237409c5d8a039924d3`
- current status: `reference-inputs-pinned-upstream-blobs-run-not-yet-qualified`

This is the first realistic compiler fixture, not a proof that a suitable solver run has happened.

The existing manifest includes a NACA0012 geometry, a steady 2D incompressible RANS / Spalart–Allmaras description, Reynolds number, density, dynamic viscosity, mesh dimensions, wall/far-field conditions, two angles of attack, requested observables, and limitations.

The compiler should normalize representative fields as follows:

| Existing authoring field | Typed interpretation |
|---|---|
| `geometry.chord_m` | length, metres |
| `flow.reynolds_number_per_chord` | dimensionless |
| `flow.density_kg_m3` | mass density, kg/m³ |
| `flow.dynamic_viscosity_kg_m_s` | dynamic viscosity, kg/(m·s) |
| `flow.mesh_normal_nodes` | positive integer count |
| `flow.wall_y_plus_target` | dimensionless |
| `instances[].angle_of_attack_deg` | plane angle, degrees |
| `instances[].freestream_velocity_m_s` | velocity vector, m/s |
| `flow.wall_condition` / `farfield_condition` | registered boundary-condition kinds |
| `observables.*` | typed, named observable definitions with reference provenance |

The compiler must preserve that the upstream config/mesh identities currently use Git blob SHA-1 fields. It must not relabel those values as SHA-256. The current manifest itself says the executable digest, derived configuration digests and reference-data digests must still be captured, and that grid convergence requires at least three nested TMR numerical-analysis grids. Those are unresolved qualification inputs, not facts the compiler may manufacture.

A v0.1 parse/schema test can pass while capability resolution remains `Blocked`. A future `PlanReady` requires a registered adapter and complete immutable model/input identity, not merely a syntactically valid TOML file.

## 6. Diagnostics and hostile inputs

Every diagnostic has a stable machine code, severity, field path, and concise explanation. Diagnostics are deterministic and bounded.

Minimum negative cases:

- duplicate TOML keys, duplicate IDs and unknown fields;
- unsupported schema/compiler/registry revision;
- missing/unknown unit and dimension mismatch;
- NaN, infinity, overflow, and out-of-range values;
- unknown solver, adapter, model, submodel or boundary condition;
- missing model-required initial/boundary condition;
- stale or mismatched artifact identity;
- unordered/ambiguous event timing or invalid timestep;
- contradictory solver, timestep, tolerance or stop settings;
- missing seed/time policy or unsupported determinism profile;
- undeclared approximation, callback or provider;
- resource budget absent, unbounded, or exceeded;
- semantically meaningful mutation that does not change the semantic-plan digest;
- harmless formatting mutation that unexpectedly changes the semantic-plan digest.

Never discard an unsupported field to salvage a plan.

## 7. Benchmark programme

### Correctness before speed

The first release gates are:

- 100% agreement with independent expected outcomes for the frozen positive/negative corpus;
- zero false `PlanReady` results for invalid units, unresolved models, unsupported conditions, stale artifacts, and missing hard requirements;
- zero silently dropped unsupported semantics;
- same semantic scenario produces identical normalized IR and digest under a pinned registry/compiler environment;
- any semantic mutation invalidates the old plan digest;
- at least 1,000 deterministic property/mutation cases before v0.2, with zero false-ready cases for hard invalidity classes;
- bounded input size, nesting, collection lengths, and diagnostic output.

### Performance

Measure separately:

- strict source parse;
- structural validation;
- typed deserialization;
- dimensional/semantic validation;
- registry/capability resolution;
- canonicalization/digest;
- total plan compilation.

Report cold and warm p50/p95/p99, throughput, peak RSS/allocations where available, and 1/16/64 concurrent compilers for 1 KiB, 10 KiB, 100 KiB and 1 MiB manifests. Record exact compiler/toolchain, registry snapshot, hardware, source and raw result artifacts. Do not compare an in-process warm Rust path against a remote solver's end-to-end wall time.

### Differential and metamorphic tests

- independent unit/dimension oracle for the supported unit subset;
- cross-implementation canonicalization golden vectors;
- TOML key-order and formatting invariance;
- ordered schedule mutation must change identity;
- equivalent unit representation may preserve physical equivalence but must preserve authored source identity;
- changing solver/model revision, input bytes, conditions, reference frame, tolerances or seed policy must change the semantic-plan identity;
- unsupported capability mutations must move `PlanReady` to `Blocked`, never be ignored.

## 8. Industry and interoperability posture

The DSL should compete on the quality of its contract and compiler, not by reimplementing entire standards:

- Use TOML as the authoring surface only; compare the resulting contract and diagnostics with JSON Schema and CUE on equivalent inputs.
- Evaluate FMI 3.0.2 / SSP adapters when actual model exchange/co-simulation is needed. FMI is an interchange boundary, not a physical-validity certificate.
- Keep Symtropy's engine core standards-light; solver protocol adapters live at the boundary.
- Bind evidence to the existing LEP / in-toto / SLSA-compatible provenance path rather than inventing another attestation format.
- Compare toolchain footprint, diagnostics usability, migration burden, deterministic output, portability and conformance—not syntax aesthetics alone.

## 9. Staged implementation

1. Freeze this v0.1 contract and map current manifests; no runtime physics changes.
2. Add schema and Rust types for the normalized IR; strict parser, diagnostic codes and bounded resource limits.
3. Add NACA0012 compile fixture and independent negative corpus. At this stage a valid document may still resolve to `Blocked`.
4. Add the offline registry/capability resolver against the exact ENG-SYNTH capability descriptor.
5. Add plan canonicalization and cross-language golden vectors; select the canonicalization profile in coordination with LEP.
6. Add benchmark harness only after semantic conformance is established.
7. Later, separately integrate executor admission, SYMTAPE, RunHealthReceipt and LEP evidence references. No new replay/evidence authority is introduced here.

## 10. Non-claims

This document and any schema-only/fixture-only pass establish no solver execution, numerical convergence, model verification, validation, uncertainty quantification, physical fidelity, engineering requirement satisfaction, safety, design acceptance, certification, evidence admission, or actuator authority.

**Success means the requested experiment is precisely specified and its support/limitations are visible—not that nature has agreed with the simulation.**
