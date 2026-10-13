# Housing Engineering: Evidence-First Digital Twin RFC

- **Status:** Draft architecture proposal; not a building design, compliance determination, or certification.
- **Date:** 2026-10-13
- **Owner:** Luminous Dynamics
- **Initial pilot assumption:** a simple, low-rise affordable dwelling in South Africa, with the jurisdiction adapter kept replaceable. The first pilot should use a conventional, well-characterized construction system.
- **Scope:** shared engineering contract for Symtropy, Symthaea, Sol-Atlas, and Mycelix. This document proposes interfaces and acceptance gates; it does not assert that the capabilities below are already implemented.

## 1. Purpose

Build a reproducible engineering workflow that can help design, analyze, verify, construct, maintain, and improve housing while keeping the evidence behind important claims inspectable.

The core product is not an AI-generated drawing. It is a **versioned building model plus a traceable evidence package**: requirements, assumptions, geometry, materials, calculations, solver inputs/outputs, code-rule evaluations, uncertainty, human reviews, construction records, and change history.

The platform must make it easy to discover what is known, what failed, what is missing, and what still requires a qualified person or authority. It must not convert a plausible rendering, a model's confidence, or a successful software test into a claim of building safety.

## 2. Responsibilities and boundaries

| System | Proposed responsibility | Must not claim |
|---|---|---|
| **Sol-Atlas** | Site context: terrain, slope, climate, solar exposure, flood and drainage context, access, utilities, environmental constraints, and source provenance. | That coarse or stale geospatial data substitutes for a survey, geotechnical investigation, flood study, or site-specific design input. |
| **Symtropy** | Deterministic/replayable scene and rigid-body simulation; construction sequence, access, robot reach/collision, material handling, tolerances, and scenario visualization. Integrate specialist solvers through explicit adapters. | That its current rigid-body engine is a validated finite-element, foundation, fire, moisture, or whole-building energy solver. |
| **Symthaea** | Requirements synthesis, tool orchestration, alternative generation, trade-off explanation, anomaly detection, and evidence-linked recommendations. | That its internal reasoning or confidence is proof, certification, or a substitute for independently checked calculations. |
| **Mycelix** | Provenance and accountability for design revisions, materials, supplier records, inspection evidence, approvals, maintenance, contracts, and shared governance. | That signatures or decentralized records make an untrue measurement true, prove workmanship, or themselves grant regulatory approval. |
| **Dedicated rule/solver services** | Deterministic code checks, geometry/model validation, structural and building-physics analyses, cost calculations, and validation suites. | That one solver or one code check covers every discipline or every local requirement. |
| **Qualified people and authorities** | Site investigations, rational design where required, independent checking, professional sign-off, inspections, and statutory decisions. | That automated output bypasses the legal approval route. |

The projects exchange artifacts through versioned, documented interfaces. Avoid hidden cross-project assumptions and avoid making one repository a monolithic dependency for all the others.

### Repository-resolution note: Sol-Atlas

The public standalone Symtropy repository still marks its `sol-atlas-core` and `sol-atlas-bevy` path dependencies as stripped, and its `atlas` feature is described as a planetary-globe/game view. I did not find a canonical public `Luminous-Dynamics/sol-atlas` repository in this review. The separate public [`terra-atlas` repository](https://github.com/Luminous-Dynamics/terra-atlas) describes itself as a renewable-energy investment and project-mapping platform; it must not be assumed to be the same system or to provide authoritative housing survey, geotechnical, flood, or code-compliance data.

Therefore, the Sol-Atlas adapter remains an **unresolved interface**, not an implemented dependency. Before coding the adapter, resolve the canonical source/repo and define the data contract. Any site facts used for a real design must carry source, resolution, capture date, coordinate reference system, coverage, uncertainty and suitability notes, and must explicitly state when a survey, geotechnical investigation or specialist study is still required.

## 3. Shared model and evidence contract

### 3.1 Interchange

- Use **IFC 4.3.2.0** as the initial open BIM interchange target where supported. Pin the exact schema/version in each run manifest; do not silently follow a development draft.
- Use **buildingSMART IDS** for machine-readable information-delivery requirements. IDS can check the presence and values of specified model information; it is not a structural solver or a full legal compliance engine.
- Preserve native/source inputs alongside the interoperable export. IFC export/import must have round-trip and loss-report tests for the subset we use.
- Use SI units internally, with explicit units on quantities and conversion tests at every adapter boundary.
- Store a canonical project/building revision and give every element a stable identifier so issues, calculations, inspections, and material records can refer to the same object.
- Do not redistribute protected standards text without the necessary rights. Store clause identifiers, edition/effective dates, licensed source references, and narrowly scoped rule expressions as permissions permit.

### 3.2 Evidence records

Every consequential claim should be represented as a machine-readable record with, at minimum:

- stable claim ID and claim type;
- building-model revision and referenced element IDs;
- jurisdiction, rule-pack identifier, edition, clause identifier, and applicability conditions where relevant;
- assumptions, units, source references, and source hashes where licensing permits;
- tool/solver name, version, executable or container digest, configuration, and exact command or invocation;
- immutable input/output artifact hashes, raw outputs, analysis method, expected tolerance, and measured result;
- validation domain, uncertainty or sensitivity information, and known limitations;
- status, reviewer, review timestamp, and supersession relationship.

A hash proves that an artifact matches a recorded digest; it does not prove that the artifact is correct. Digital signatures prove who signed a record under a particular key policy; they do not prove the physical conditions described by the record.

A draft JSON Schema for this envelope is available at [`housing-evidence-manifest.schema.json`](housing-evidence-manifest.schema.json). The [`example-not-evidence.json`](fixtures/example-not-evidence.json) fixture is intentionally synthetic: its digest is a placeholder, no engineering computation has run, and its status is `NOT_EVALUATED`. It must never be used as a real project record.

The Rust verifier prototype is in `crates/apps/symtropy-housing-evidence` (binary: `housing-evidence-verify`). It embeds the checked-in schema, validates the JSON Schema 2020-12 meta-schema and manifest, and disables remote schema-reference fetching. An independently pinned expected-claims contract constrains schema/case/jurisdiction/model bytes/rule pack/toolchain and each mandatory claim's scope, criterion, approved tool, and exact input/output artifact digests. Local artifact bytes are hashed against their declarations. The JSON report separates producer-declared status, conservative effective status, and the evidence-integrity verdict. `EVIDENCE_INTEGRITY_VERIFIED` means package integrity only—not independent calculation, building safety, code compliance, professional review, or statutory approval. The external contract pins model bytes by digest; the manifest carries the canonical `evidence_artifacts` registry.

Run the structural check on the deliberately unevaluated fixture from the repository root:

```text
cargo run -p symtropy-housing-evidence -- docs/built-environment/fixtures/example-not-evidence.json
```

A construction/evaluation report must not be treated as passed just because this command can parse it. For a candidate `overall_status: PASS`, supply the independent expected-claims contract described in the verifier README, its trusted out-of-band `--expected-claims-sha256` digest, and `--artifact-root`. The contract must pin the intended model, rule pack, toolchain, and full mandatory claim definitions, including exact input/output artifact digests. The verifier emits `evidence_integrity_status: EVIDENCE_INTEGRITY_VERIFIED` only when those pins and local artifact bytes match. This is not a structural/code result; the synthetic fixture remains `NOT_EVALUATED` with placeholder bytes.

The JSON Schema checks document shape and includes conservative aggregate guards: an overall `PASS` requires a confirmed jurisdiction, a rule pack marked `reviewed_for_use`, at least one mandatory `PASS` claim, and `PASS` for every included mandatory claim. It cannot establish that the declared requirement set is complete, that a digest matches the bytes, or that a clause is applicable. A separate verifier must compare the manifest's claim set to the expected mandatory requirement set, verify artifact bytes and tool provenance, and enforce all cross-record invariants. Add fixture-based positive and negative tests—including omitted mandatory claims—before treating the contract as stable.

### 3.3 Fail-closed result states

Use separate status values; do not reduce every state to a green/red badge.

- `PASS`: evaluated against a named, applicable requirement with the declared evidence and acceptance criteria satisfied.
- `FAIL`: evaluated and one or more acceptance criteria were not satisfied.
- `REQUIRES_REVIEW`: the rule or evidence requires a competent reviewer or statutory decision.
- `NOT_EVALUATED`: no evaluation result exists.
- `BLOCKED`: evaluation could not run because of missing inputs, unsupported geometry, unavailable licensed data, tool failure, or another stated blocker.
- `INSUFFICIENT_EVIDENCE`: an analysis ran but does not support the stated conclusion to the required confidence/validity domain.

The aggregate result must never be `PASS` while any mandatory requirement is `FAIL`, `BLOCKED`, `NOT_EVALUATED`, `INSUFFICIENT_EVIDENCE`, or unresolved `REQUIRES_REVIEW`. Report the full vector of statuses and why each item is unresolved.

## 4. Verification architecture

Use a hierarchy of evidence. Passing a lower tier does not imply passing a higher one.

1. **Schema and units:** valid model structure, identifier uniqueness, supported geometry, units, coordinate frames, tolerances, and required fields.
2. **Analytical/property tests:** identities and cases with known answers; dimensions and units; equilibrium checks; load-path bookkeeping; material and quantity calculations.
3. **Independent numerical comparison:** compare a solver's output with an independently implemented/reference solution and published benchmark for the relevant physical domain. Declare tolerances before executing the test.
4. **Empirical validation:** where the claim warrants it, compare against controlled laboratory, field, or monitored-building data and document calibration/validation separation.
5. **Independent engineering and statutory review:** qualified reviewers assess assumptions, suitability, local requirements, and submissions. Authority decisions remain external evidence records, not an AI status.

Every campaign must include a clean-checkout reproduction path, exact source revision, configuration, raw outputs, environment metadata, predeclared expected results/tolerances, and negative controls. Queued, skipped, timed-out, or unavailable verification is not a pass.

### Solver strategy

Do not implement bespoke building-grade solvers merely because Rust is the preferred production language. Prefer adapters to appropriate, maintained specialist tools while building a small Rust-native orchestration and evidence layer. Select solvers per domain only after checking license, maturity, test suite, supported physics, formats, and reproducibility.

Initial evaluation candidates (not yet selected or certified):

- structural analysis: compare suitable open-source finite-element/frame solvers and an independent reference suite before making structural claims;
- energy/thermal: EnergyPlus/OpenStudio or another suitable engine with ANSI/ASHRAE 140/BESTEST evidence;
- geometry/coordination: IFC validation, IDS validation, clash checks, and explicit unsupported-feature reporting;
- site/hydrology/geotechnical: authoritative geospatial data plus discipline-specific analyses and investigation inputs;
- construction robots: Symtropy scene/replay plus robot-specific kinematics, collision, force/load, reachability, and safety checks.

Symtropy's existing rigid-body functionality remains useful for physical interaction, construction choreography, and robotics scenes. The RFC does not treat rigid-body validation as structural FEM validation.

## 5. Jurisdiction adapter and South African pilot

The jurisdiction adapter should be a separately versioned package, not scattered constants embedded in design code. Each rule needs a source, edition/status, effective dates, applicability predicate, evaluation implementation, tests, review trail, and rights/access notes.

For a South African pilot:

- begin from the **National Building Regulations and Building Standards Act 103 of 1977** and the operative regulations and referenced standards applicable to the actual municipality, building use, and construction system;
- use SANS 10400 and other referenced standards only from authoritative, current editions with the appropriate access rights; verify amendments and municipal processes before treating a rule pack as current;
- model the responsible local authority and required professional roles/approvals explicitly;
- for novel or non-standardized construction systems, investigate the applicable **Agrément South Africa** assessment/certification route. A model result does not replace an Agrément certificate where one is needed or a municipality's approval;
- keep the pilot geometry, construction system, intended occupancy, and site assumptions narrow. The first pass should not attempt high-rise, unusual soil, retaining structures, or novel structural materials.

A code clause that cannot be lawfully accessed, interpreted, or converted into a rigorously tested rule must be marked unavailable or requires-review. It must not be paraphrased from memory and passed off as an executable legal rule.

## 6. Evidence-backed code-improvement process

Treat every proposed code change as a falsifiable research hypothesis.

1. **Identify a specific provision or gap.** Explain its original safety purpose, scope, affected building classes, and the observed problem. Avoid claiming a rule is unnecessary merely because one design performs well.
2. **State the proposed change and intended outcome.** Identify the safety/performance functions that must remain invariant and the outcomes expected to improve.
3. **Pre-register the evaluation.** Define baseline designs, alternative designs, hazards, boundary conditions, datasets, metrics, acceptance thresholds, uncertainty analysis, and negative controls before viewing final results.
4. **Test across the applicability envelope.** Include plausible extremes, adverse cases, sensitivity analyses, degradation/maintenance scenarios, and representative populations/sites. Report counterexamples and failed runs.
5. **Compare against independent evidence.** Use independent solvers, qualified review, and laboratory or field evidence where the risk and novelty require it. A model trained on a code is not independent proof that the code is correct.
6. **Quantify trade-offs.** Report safety, uncertainty, accessibility, health, durability, environmental burden, construction complexity, lifecycle cost, and distributional effects. Do not trade away mandatory life-safety performance for a lower price.
7. **Publish a dossier for peer and regulator review.** Include exact clause references, proposed language, evidence manifest, reproducibility steps, scope limitations, dissenting findings, and a rollback/monitoring plan.
8. **Track the official decision and post-deployment evidence.** A research recommendation is not an adopted code amendment. Measure real outcomes after any approved change and revise the hypothesis when evidence conflicts with it.

## 7. First vertical slice

### Bounded scenario

Create a **reference-only, single-storey detached dwelling model** using a well-characterized conventional construction system and documented, synthetic/site-independent initial conditions. This is a software/verification benchmark, not a construction-ready design and not a substitute for a real site survey.

The initial scenario should exercise the entire evidence contract without claiming to calculate every discipline at once:

1. establish the canonical building model and stable element IDs;
2. validate geometry, units, required model information, and quantity extraction;
3. attach a versioned, deliberately limited rule pack whose rules and rights have been reviewed;
4. run one bounded, independently verifiable engineering calculation through a specialist solver adapter;
5. record raw inputs/outputs, exact tool versions, hashes, tolerances, and status;
6. inject negative controls (missing material property, invalid units, missing required datum, solver failure, and an out-of-threshold result) and verify fail-closed behavior;
7. export an evidence manifest that can be re-run from a clean checkout;
8. have an independent practitioner review the assumptions and the report before expanding the domain.

Then add further disciplines one at a time—structural, fire, moisture/thermal, energy, water/sanitation, accessibility, drainage, cost and construction simulation—only when each domain has a suitable reference suite and clear validity boundary.

### First analytical reference case

A deliberately small analytical oracle now lives at [`benchmarks/cantilever-beam-linear-static-v1.json`](benchmarks/cantilever-beam-linear-static-v1.json), with an integration test at `crates/apps/symtropy-housing-evidence/tests/reference_cantilever_beam.rs`. It checks rectangular-section inertia, a linear-static Euler-Bernoulli cantilever's tip deflection, support bending stress, and invalid non-positive inputs. This establishes a known-answer case and exercises units/calculation plumbing; it does not compare an external solver yet, validate FEM, or establish building safety. The first solver-adapter milestone must reproduce this oracle and then pass a broader independent benchmark set.

### Acceptance gates

The first slice is accepted only if:

- all mandatory records have stable IDs, units, provenance, and model revision links;
- every reported status can be traced to a requirement, input, computation, and acceptance criterion;
- a clean rerun reproduces the declared outputs within predeclared tolerances;
- negative controls fail or block exactly as specified and cannot be mistaken for passes;
- missing/unsupported analyses remain visible in the overall report;
- the report names out-of-scope physics, unresolved issues, and review/approval needs;
- the package explicitly states that it is not a permit set, engineering certification, or construction authorization.

No result is to be described as passed until evidence is produced for the exact source revision and scenario.

## 8. Project integration milestones

- **M0 — Contract and vocabulary:** stabilize this evidence schema, IDs, status semantics, artifact hashing, and clean-run manifest.
- **M1 — Model ingestion:** import/export a narrow IFC subset; validate units, IDs, and loss reports; add IDS validation.
- **M2 — Verification harness:** define scenario files, solver-adapter contract, reference cases, negative controls, and exact-head reproducibility.
- **M3 — First engineering domain:** choose one discipline and solver only after a documented comparison of candidates, licenses, and validation evidence.
- **M4 — Mycelix provenance:** record signed attestations, approvals, material batch provenance, inspection evidence, and superseding records; keep critical evidence available for offline review.
- **M5 — Symthaea orchestration:** generate explainable alternatives and run plans, while mandatory acceptance criteria remain deterministic and external to model confidence.
- **M6 — Symtropy construction digital twin:** simulate the validated geometry, robot motion, material handling, and construction sequence; keep simulation validity claims separate from building-physics claims.
- **M7 — Pilot and feedback:** review with qualified local professionals, seek required approvals, observe a properly authorized physical pilot, and compare as-built/operational data against predictions before making public performance claims.

## 9. Research references

These are architectural starting points, not endorsements of a particular solver or declarations of code compliance:

- [South African Government — National Building Regulations and Building Standards Act 103 of 1977](https://www.gov.za/documents/national-building-regulations-and-building-standards-act-16-apr-2015-1302)
- [Agrément South Africa — Certification Process](https://agrement.co.za/certification-process/)
- [buildingSMART — IFC 4.3.2.0 official specification](https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/)
- [buildingSMART — Information Delivery Specification (IDS)](https://technical.buildingsmart.org/Projects/Information-Delivery-Specification-Ids/)
- [ASHRAE — Building energy simulation resources, including Standard 140](https://www.ashrae.org/technical-resources/bookstore/ashrae-building-energy-simulation-and-performance-modeling-resources)
- [NIST — Validation of ASCE 41 procedures in performance-based seismic engineering](https://www.nist.gov/programs-projects/validation-asce-41-procedures-pbse)

## 10. Current limitations and decision log

- This RFC is a proposal. No code, solver adapter, legal rule pack, or housing design is delivered by this document alone.
- The current Symtropy rigid-body engine is not represented as a certified structural, foundation, fire, fluid, moisture, or energy solver.
- The repositories' own maturity declarations should be rechecked before selecting components; passing unit tests is not evidence of field deployment or professional qualification.
- The exact initial municipality, construction system, SANS editions, local amendments, and licensed standards access must be confirmed before producing jurisdiction-specific rules.
- A change to this RFC must preserve the fail-closed status semantics, negative controls, immutable provenance, and distinction between software verification, physical validation, professional review, and legal approval.
