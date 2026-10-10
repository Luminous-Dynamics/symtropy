# Design-to-Reality Authority Contract v0.1

Status: normative architecture proposal only. This document creates no new runtime authority and makes no claim that the described invariants are already enforced.

## Purpose

Symtropy already has strong lower-layer fabrication semantics: persistent workpiece identity, typed interfaces and joints, process specifications, multidimensional capability envelopes, workmanship evidence, three-valued functional engineering constraints, substitution, deterministic fabrication plans, diagnostics, and technical commissioning.

The missing layer is not another fabrication engine. It is a small systems-design and analysis contract that lets humans and Symthaea express what they are trying to build, bind exact design revisions, request analyses, retain evidence, and hand only qualified information to the existing Fabrication authority stack.

The core rule is:

> Design describes intent and exact proposed realizations. Analysis produces evidence. Fabrication evaluates engineering closure and plans physical work. Physical and commissioning authorities determine what actually exists and what is technically ready. No layer silently promotes its own output into a stronger authority class.

## Non-goals

The design layer must not:

- become a physics oracle;
- duplicate `symtropy-fabrication::FunctionalDesign` constraint evaluation;
- create or mutate physical workpieces;
- decide fabrication capability admission;
- synthesize an `ExecutableFabricationPlan` without the Fabrication planning boundary;
- treat a simulation result as physical evidence;
- treat technical commissioning as civic, legal, regulatory, ownership, or operational authorization;
- decide legal inventorship, patent ownership, royalty percentages, or payment finality;
- require public disclosure of confidential design artifacts merely to establish immutable provenance;
- invent proprietary replacements for mature systems-engineering, co-simulation, product-data, or metrology interchange standards where adapters are sufficient.

## Authority matrix

| Question | Owning authority |
| --- | --- |
| What are we trying to make? | Design authority |
| Which exact revision is under discussion? | Design authority |
| Which requirements and verification obligations belong to that revision? | Design authority |
| Which artifact bytes/configuration were referenced? | Content-addressed artifact manifest |
| What did one solver/model/test execution report? | Analysis or measurement evidence producer |
| Is a solver/model qualified for this use? | Qualification authority/profile |
| Does supplied engineering evidence close a checkable constraint? | Fabrication functional evaluator |
| Is a proposed substitute functionally acceptable? | Fabrication substitution authority |
| Which process/capability plan can realize the selected design? | Fabrication planning authority |
| Was a physical process performed? | Process/construction evidence authority |
| What physical object/material state exists? | Physical matter authority |
| Is the exact fabricated subject technically commissioned? | Technical commissioning authority |
| Is it lawful/permitted/certified to operate? | External civic/regulatory authority |
| Who submitted which derivation/contribution? | Provenance/attribution authority |
| Which contribution implies ownership or compensation? | Explicit contract/legal process, not provenance alone |
| Did economic settlement reach finality? | Settlement/finance authority |

## D0 invariants

### D0-001: exact design identity is content-bound

`design_id + revision` is not sufficient authority. Every authority-facing reference to a design revision must bind a canonical content digest.

A valid exact reference has the conceptual shape:

```text
DesignRevisionRef = {
  design_id,
  revision,
  content_digest
}
```

Changing requirements, artifact identities, parent lineage, system structure, or other canonical design semantics must change the digest even if a caller attempts to reuse the same `design_id` and revision number.

### D0-002: design revisions are immutable

A revision is not edited in place after publication into an authority-bearing workflow. Corrections and new alternatives create a new revision with explicit ancestry/supersession metadata.

Mutable user-facing aliases such as `latest`, branch names, titles, or workspace paths are convenience locators only and must not appear as exact authority references.

### D0-003: artifacts are references, not embedded authority

CAD, SysML, meshes, source code, datasets, process models, solver decks, drawings, and other large artifacts may live in different stores. The design graph binds their immutable content identity and semantic role.

Moving an artifact between stores must not change its identity. Changing its bytes must.

Possession of an artifact reference does not imply disclosure/dereference authority.

### D0-004: requirement traceability does not duplicate engineering evaluation

The design layer may represent:

- stakeholder/system requirements;
- requirement derivation/decomposition;
- system roles/components/interfaces;
- references to checkable Fabrication functional constraints;
- verification obligations and cases;
- analysis observables required to produce evidence.

It must not add an independent `passed`, `safe`, `verified`, or scalar confidence field that competes with the Fabrication evaluator.

### D0-005: analysis is evidence, never truth by declaration

An analysis execution records what an exact model/tool/configuration reported for an exact design revision under exact inputs and assumptions.

It must never directly emit caller-authoritative statements such as:

```text
design_is_safe = true
requirement_passed = true
```

Instead, qualified analysis output may be transformed through an explicit, auditable evidence bridge into engineering facts consumed by the Fabrication evaluator.

### D0-006: non-convergence cannot become affirmative evidence

The following implications are invalid:

```text
solver exited => solver converged
solver converged => model qualified
model qualified => model applicable to this subject
applicable simulation => physical observation
physical observation => technical commissioning
```

A failed/non-converged run remains useful diagnostic evidence but cannot silently produce an affirmative engineering fact for constraint closure.

### D0-007: applicability is explicit

Solver/model qualification is scoped to an explicit domain/profile. A tool is not globally `trusted` because it performed well in another regime.

Qualification should conceptually bind:

```text
solver + implementation/version + model/profile + validity domain + benchmark/evidence lineage
```

Evidence outside the declared applicability domain cannot satisfy a consumer requirement merely because it has higher nominal fidelity or a prestigious producer.

### D0-008: uncertainty is preserved

Where an analysis or measurement has uncertainty, the uncertainty must remain visible through the evidence bridge. Do not collapse intervals, resolution, model discrepancy, numerical uncertainty, or calibration uncertainty into an arbitrary quality score.

### D0-009: disagreement is retained

Multiple facts may address the same subject/dimension. Contradiction is represented explicitly and remains available to evaluators. There is no latest-write-wins truth rule for engineering evidence.

### D0-010: qualification is faceted, not scalar

There is no universal `design_validation_score` or single integer level that means an invention is proven.

Consumers may require different facets such as:

- geometry integrity;
- unit consistency;
- material-model applicability;
- numerical convergence;
- discretization/mesh convergence;
- simulation reproduction;
- independent-solver reproduction;
- manufacturability;
- prototype fabrication;
- physical test;
- independent physical replication;
- field operation;
- reliability duration;
- external certification.

A facet can be established, failed, indeterminate, expired, superseded, not established, or not applicable according to its own theorem/profile.

### D0-011: one consequential decision uses one coherent evidence cut

A qualification or contract acceptance decision must bind one coherent immutable evidence cut: exact design revision, exact evidence refs, exact profile/policy generation, and explicit evaluation time where time/freshness matters.

Old and new generations may not be opportunistically mixed to manufacture a stronger conclusion.

Same exact cut + same exact profile + same explicit evaluation time must produce the same deterministic qualification result, absent explicitly external nondeterministic authority.

### D0-012: qualification can expire without rewriting history

A historical qualification record preserves what was established under its exact evidence cut and profile at that time. New fatigue data, revoked calibration, superseded material assumptions, or newer policy may cause future qualification to expire/supersede without deleting or rewriting historical evidence.

### D0-013: planning feasibility is not engineering feasibility

ERP/MRP/MES systems may determine whether resources, inventory, machines, and schedules can satisfy a production plan. That does not establish that the design is physically correct or manufacturable within qualified capability envelopes.

Production coordination should reference exact Symtropy design/fabrication authority records rather than developing a parallel physical-feasibility oracle.

### D0-014: provenance is not ownership

The technical graph may prove that contributor X submitted transformation T from exact parent revision A to exact child revision B, or produced/reproduced/challenged evidence E.

It must not infer patent inventorship, legal ownership, royalty percentages, or payment rights solely from that provenance.

### D0-015: contracts determine compensation

Compensation is defined by an explicit contract that references exact design/evidence/qualification conditions. Useful contract outcomes include fixed bounties, milestone payments, validation prizes, falsification prizes, royalties, procurement terms, or combinations.

Negative results and qualified falsification must be representable as economically valuable outcomes.

### D0-016: settlement remains external to design semantics

A design or innovation record must not contain authoritative `paid = true` state. Settlement finality belongs to a settlement/finance authority and is referenced through an immutable receipt/status contract.

### D0-017: no autonomous authority escalation for Symthaea

Symthaea may propose designs, analyses, experiments, alternatives, optimizations, and challenges. Recommendations remain proposals. They do not grant manufacturing capability, approve commissioning, spend funds, waive requirements, or create regulatory/operational permission without an independently valid authority path.

## Canonical design graph boundary

The initial implementation should be deliberately small. The design graph is a root over exact immutable references, not a universal engineering object model.

Conceptually:

```text
DesignRevision
  identity
  exact digest
  parent revisions
  artifact references
  requirement references
  system/decomposition references
  verification obligations
```

It deliberately does not own:

```text
physical workpiece state
solver execution state
engineering pass/fail state
fabrication progress
commissioning state
payment state
```

## Requirements and verification obligations

A requirement can point to one or more explicit verification obligations. Candidate obligation kinds include:

```text
EngineeringConstraint(constraint_id)
EvidenceKind(evidence_kind)
AnalysisObservable(observable_id, analysis_profile)
ExternalVerificationCase(scheme_id, case_id)
```

The requirement does not store whether those obligations are satisfied. Satisfaction is derived from the relevant owning authority and exact evidence cut.

This permits high-level requirements such as "support 5 kN service load" to compile/reference precise F7 constraints without creating another predicate language.

## Analysis boundary

The analysis layer should describe requests and evidence independently of any one solver.

A request binds at minimum:

```text
exact design revision
analysis profile
input/model artifacts
requested observables
assumptions
initial/boundary conditions
resource/fidelity policy where relevant
```

An execution receipt/evidence record binds at minimum:

```text
exact request
exact design revision
solver/tool identity and version
model/profile identity
execution environment identity
result artifacts
extracted observations
convergence/numerical evidence
uncertainty
applicability domain
content digest
```

External solver execution must remain sandboxable and must not gain host authority merely because its output is useful evidence.

## Interoperability policy

Symtropy should own internal authority semantics while mapping to mature interchange standards where useful.

Initial targets:

- SysML v2 for systems requirements, decomposition, interfaces, verification cases, quantities/units, and model exchange;
- FMI 3.x for model exchange, co-simulation, and scheduled execution adapters;
- SSP 2.x for hierarchical multi-model system structure, parameterization, packaging, and variants;
- STEP/AP242-class product data for mechanical/product-definition interchange;
- QIF for manufacturing quality/inspection evidence interchange;
- OpenUSD for collaborative scene/assembly/world visualization where appropriate, without treating it as canonical engineering truth.

Adapters must preserve unsupported semantics or fail visibly rather than silently dropping authority-relevant information.

## Geometry policy

Meshes such as STL are derived artifacts, not canonical mechanical design authority when richer product geometry exists.

The design layer should remain backend-neutral. A future `GeometryBackend` may bridge the existing Symthaea fabrication geometry, Rust B-rep/NURBS kernels, robust manifold mesh operations, STEP readers/writers, or other qualified implementations without changing the design authority contract.

## Mycelix composition policy

The intended cross-system separation is:

```text
Symtropy Design
  exact technical proposal and traceability
        |
        v
Symtropy Analysis
  model/simulation/test evidence
        |
        v
Symtropy Fabrication
  engineering constraint closure + realization/planning + commissioning
        |
        v
Mycelix epistemics/provenance
  durable claims, challenges, derivations, replications, contributors
        |
        v
Mycelix Business/Marketplace
  contractual obligations and acceptance workflow
        |
        v
Mycelix Finance / external rail
  settlement finality
```

Mycelix social confidence/reputation may help discovery/admission policy but must not numerically alter physical estimates or turn missing engineering evidence into satisfied constraints.

## First vertical acceptance proof

The first end-to-end demo should be a small load-bearing bracket with explicit requirements for mass, deflection under a known load envelope, material/process constraints, and manufacturability on an approved capability profile.

The demo should include at least three candidates:

1. a conservative baseline;
2. an optimized design that survives qualification;
3. a pathological candidate that appears superior under a coarse/unstable analysis but fails convergence/refinement qualification.

The full positive path should demonstrate:

```text
requirement
-> exact design revision
-> analysis request
-> qualified analysis evidence
-> engineering facts
-> F7 constraint evaluation
-> F8 realization/substitution as needed
-> F10 executable fabrication plan
-> physical fabrication/process evidence
-> physical measurement
-> technical commissioning
-> provenance/attribution record
-> contract acceptance evidence
-> external settlement receipt
```

The negative/hostile path must prove at least:

- altered design content under reused id/revision is rejected by digest mismatch;
- evidence for a prior design revision cannot satisfy the new revision;
- non-converged analysis cannot produce affirmative engineering facts;
- missing uncertainty/calibration needed by a profile yields indeterminate/unknown rather than pass;
- a physical test for the wrong specimen/subject cannot satisfy the current subject;
- changing a fabrication plan invalidates exact bindings that depended on the prior plan;
- caller-authored `Satisfied`/`commissioned`/`paid` flags have no authority;
- contradictory evidence remains visible;
- payout/settlement replay is idempotent or rejected by the owning economic authority.

## Implementation sequence

The intended first implementation tranches are:

1. D1: dependency-light content-bound design revision graph;
2. D2: requirement traceability and verification obligations without duplicating F7 predicates;
3. A0: immutable analysis request/evidence contracts;
4. A1: solver/model applicability and qualification semantics;
5. B0: explicit analysis-evidence to Fabrication engineering-fact bridge;
6. B1: verified design-to-F8/F10 realization bridge;
7. interoperability adapters (SysML/FMI/SSP/STEP/QIF) as independent optional crates/features;
8. Mycelix provenance/contract/settlement projections;
9. hostile bracket acceptance proof.

D1 and A0 should remain independent of active Fabrication hardening branches. Integration into Fabrication should wait until the relevant exact-authority contracts underneath it have qualified.

## Claim boundary

This document freezes architectural intent only.

It does not establish solver correctness, CAD kernel correctness, physical safety, manufacturability, certification, legal inventorship, payment enforceability, or production readiness. Those claims require their own implementation and evidence.
