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

### Repository-resolution note: Sol-Atlas and Terra Atlas

A public [`sol-atlas-leptos` repository](https://github.com/Luminous-Dynamics/sol-atlas-leptos) exists and contains a shared `sol-atlas-core` crate plus the Leptos/WebGL globe renderer. Its core provides geodetic display math and provenance-bearing layer types; the globe is a visualization surface, not a cadastral/geotechnical survey engine. The `sol-atlas-core` Cargo metadata declares `AGPL-3.0-or-later`, so Symtropy must not add it as a linked Rust dependency without an explicit licensing/redistribution review. Start with a versioned, read-only data contract and artifact exchange, not a direct crate dependency.

Related current work remains provisional:
- [Sol Atlas PR #104](https://github.com/Luminous-Dynamics/sol-atlas-leptos/pull/104) has green exact-head run [`38509723929`](https://github.com/Luminous-Dynamics/sol-atlas-leptos/actions/runs/38509723929) for `fmt`, `test-native`, and `check-wasm`. Its NDWI/MNDWI estimator still expects already aligned, QA-masked reflectance samples; it adds no scene downloader or raster decoder. The water-observation export requires a canonical evidence-bundle SHA-256; no production scene was downloaded or analyzed.
- [Sol Atlas PR #101](https://github.com/Luminous-Dynamics/sol-atlas-leptos/pull/101) has green exact-head catalog validation [`38510250201`](https://github.com/Luminous-Dynamics/sol-atlas-leptos/actions/runs/38510250201) (9 domains, 23 source records, 15 Python tests) and green Rust `Check` [`38510250259`](https://github.com/Luminous-Dynamics/sol-atlas-leptos/actions/runs/38510250259). It remains a catalog/metadata inventory: CHIRPS/WaPOR raster products have not yet been acquired or processed.
- [Mycelix Earth PR #3776](https://github.com/Luminous-Dynamics/luminous-dynamics/pull/3776) proposes real Sentinel-2/Landsat STAC metadata discovery, but labels itself catalogue metadata discovery—not complete image science—and keeps raster processing/QA behind later gates. Its inspected exact-head validation run on 2026-10-13 concluded failure; the available job-log fetch supplied no diagnostic text, so its cause is unknown and the provider remains unverified.
- The separately named [`terra-atlas` repository](https://github.com/Luminous-Dynamics/terra-atlas) is archived and describes an energy-investment globe. It is not interchangeable with Sol Atlas or a source of verified housing-site conditions.

The interface remains an **unimplemented adapter**, but the canonical renderer/core repository has been located. See [`site-evidence-adapter-contract.md`](site-evidence-adapter-contract.md) for the proposed read-only interface and its acceptance gates. No site evidence may be upgraded merely because it appears in a globe, a map tile, a catalogue, a scenario, or a visualization. Terrain, solar, flood, hydrology and access layers must each carry their source, version/vintage, acquisition and receipt times, coordinate reference system, native resolution, coverage and QA state, uncertainty, processing chain, rights, and suitability notes. A site boundary, subsurface condition, foundation parameter, flood design level or legal setback must remain `REQUIRES_REVIEW` or `INSUFFICIENT_EVIDENCE` until its appropriate authoritative data or investigation exists.

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

The Rust verifier prototype is in `crates/apps/symtropy-housing-evidence` (binary: `housing-evidence-verify`). It embeds the checked-in schema, validates the JSON Schema 2020-12 meta-schema and manifest, and disables remote schema-reference fetching. An independently pinned expected-claims contract constrains the exact embedded schema digest and verifier executable/version, as well as case/jurisdiction/model bytes/rule pack/toolchain and each mandatory claim's scope, criterion, approved tool, and exact input/output artifact digests. Local artifact bytes are hashed against their declarations. The JSON report separates producer-declared status, conservative effective status, and the evidence-integrity verdict. `EVIDENCE_INTEGRITY_VERIFIED` means package integrity only—not independent calculation, building safety, code compliance, professional review, or statutory approval. The external contract pins model bytes by digest; the manifest carries the canonical `evidence_artifacts` registry.

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

- **Structural linear-static pilot:** begin evaluation with CalculiX as a candidate adapter for the analytical cantilever and additional reference cases. The [solver-adapter evaluation memo](solver-adapter-evaluation.md) documents the rationale, GPL-2.0 distribution gate, and mandatory acceptance tests. A public test corpus is useful for designing a campaign, but does not prove suitability for a specific design domain.
- **Nonlinear/dynamic structural and geotechnical studies:** keep OpenSees as a later candidate. Its official documentation targets structural/geotechnical dynamic-response simulation, but its license requires review before incorporating it into a commercial-distribution product. Do not bundle or link it as a dependency absent legal clearance.
- **Energy/thermal:** EnergyPlus/OpenStudio or another suitable engine with ANSI/ASHRAE 140/BESTEST evidence;
- **Geometry/coordination:** IFC validation, IDS validation, clash checks, and explicit unsupported-feature reporting;
- **Site/hydrology/geotechnical:** authoritative geospatial data plus discipline-specific analyses and investigation inputs;
- **Construction robots:** Symtropy scene/replay plus robot-specific kinematics, collision, force/load, reachability, and safety checks.

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

### Jurisdiction evidence registry contract

The controlled regression suite now checks both sides of the readiness gate: a synthetic record with locally hashed source, review, implementation and test artifacts can satisfy the prerequisites, while source-byte mutation and parent-directory traversal must block it. A translated rule without an edition or exact clause/location remains ineligible even when its other test artifacts are present. An `independently_reviewed` translation now needs its own digest-verified review receipt, a distinct reviewer reference from its author reference, and a review date no later than the registry evaluation date. Source snapshot, applicability-review, translation-review, implementation and test-corpus roles cannot reuse the same declared SHA-256 digest. These controls qualify verifier logic only; they do not establish any real legal source's applicability.


The experimental `jurisdiction-evidence-registry.schema.json`, its synthetic fixture, and `housing-evidence-verify --validate-jurisdiction-registry <registry.json>` implement a catalog-only first step. Each source record has a stable ID, instrument type, source locator, lifecycle state, effective window, applicability state, optional local source snapshot, review evidence, and optional versioned rule translation with hashed implementation/test artifacts. Schema validity does not make a record executable.

Eligibility for a later rule evaluator is computed—not accepted from input—and requires a registry resolved to a named municipality; a source reported in force on the explicit `evaluation_date`; a coherent effective-date window; project-specific applicability confirmation; locally hash-verified source, applicability-review and translation-review artifacts; an applicability review decision for evaluation; a translation review receipt approving evaluation whose reviewer reference differs from the author reference; public or appropriately licensed source access; and a test corpus whose artifacts hash correctly. The source snapshot, applicability-review, translation-review, implementation and test-corpus roles must have distinct declared SHA-256 digests. Publication dates, snapshot-capture dates, and both review dates must not postdate the registry evaluation date; because `evaluation_date` is date-only, timestamp checks operate at calendar-day precision. Proposals, unknown lifecycle states, unverified snapshots, and unresolved applicability remain ineligible. Reviewer qualifications, reference identity, signature authenticity, source authority, and legal interpretation still require independent review outside this CLI.

The synthetic Gauteng registry deliberately leaves the municipality unresolved, snapshots absent, applicability unassessed, and both records ineligible. It records the official Act page as an unverified catalog entry and separates the South African Government's 13 October 2026 extension notice for the National Building Regulations and Building Standards Amendment Bill as a proposal with a 12 November 2026 comment deadline. It does not claim the Bill is in force, assert that it knows the current consolidated text, or encode a SANS clause as executable. These official pages provide discovery metadata, not an adopted rule pack ([Act reference](https://www.gov.za/documents/national-building-regulations-and-building-standards-act-16-apr-2015-1302), [comment-period extension notice](https://www.gov.za/documents/notices/national-building-regulations-and-building-standards-act-extension-submission)).

### Rule evaluation receipt contract

`rule-evaluation-receipt.schema.json` and `housing-evidence-verify --validate-receipt <receipt.json> --registry <registry.json>` define the next boundary: a receipt binds the exact registry bytes and revision, evaluation date, confirmed jurisdiction, source record and edition/clause, source snapshot, rule/version, implementation, test corpus, translation-review receipt, project-input manifest, evaluator executable, and output artifact. Referenced artifacts are resolved only under the registry directory and checked against SHA-256 digests; record-level bindings must match the selected registry source record, and a not-ready source record cannot make a receipt eligible to evaluate.

The receipt validator intentionally reports the producer's declared outcome separately from the verifier's result. Even when all receipt bindings and digests verify, its effective status is `NOT_REPRODUCED`: this command does not execute the rule, independently rerun the calculation, authenticate signatures/identities, or prove code compliance. A future evaluator must independently pin its binary, complete requirement set, inputs, and deterministic known-answer corpus before its result can be trusted.

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
- **M0.5 — Resident-owned brief:** capture spaces, priorities, accessibility goals, budget bounds, privacy scope, and non-negotiable conflict behavior before generating candidates; see [Home Design Brief V1](home-design-brief-v1.md), its schema/fixture, and schema regression tests and the `housing-evidence-verify --validate-brief <brief.json>` schema/cross-field validator.
- **M0.75 — Candidate layout contract:** bind rectangular floor-plan candidates to exact brief bytes, check room coverage and basic bounds/overlap, and defer openings, circulation, structural checks, and code compliance; see [Home Layout Candidate V1](home-layout-candidate-v1.md).
- **M0.8 — Jurisdiction source registry:** catalog source pointers separately from proposals, verify local snapshot/review/implementation/test artifact digests, and compute rule-evaluator readiness only when explicit jurisdiction, lifecycle, applicability, review and rights gates are satisfied.
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
- [South African Government — extension of public comment on the 2026 National Building Regulations and Building Standards Amendment Bill (deadline 12 November 2026)](https://www.gov.za/documents/notices/national-building-regulations-and-building-standards-act-extension-submission)
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
