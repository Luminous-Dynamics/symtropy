# Home Design Brief V1

- **Status:** draft interface contract; not a building model or an engineering result.
- **Version:** 0.1.0
- **Purpose:** capture what residents want before the system generates or evaluates architectural candidates.
- **Companion:** [Housing Engineering RFC](housing-engineering-rfc.md) and [housing evidence verifier](../../crates/apps/symtropy-housing-evidence/README.md).

## 1. Product intent

People should be able to describe the life they want to live and review meaningful design alternatives without having to learn a CAD or BIM application first. The brief is the resident-owned input to that workflow. It is not the canonical building geometry, a cost estimate, a code interpretation, or construction permission. The next contract, [Home Layout Candidate V1](home-layout-candidate-v1.md), is the explicit candidate output linked to the exact brief bytes.

The proposed hand-off is:

```text
resident language and choices
  -> reviewed Home Design Brief V1
  -> explicit ambiguity / conflict report
  -> bounded candidate layouts
  -> site + geometry + cost + performance evaluations
  -> open BIM exchange and evidence manifest
  -> professional review and authorized delivery
```

This first slice freezes the intent vocabulary and its safety boundaries. It does not claim that a generator, optimizer, IFC importer/exporter, or building-energy adapter is implemented.

## 2. Cross-project responsibilities

| Component | Proposed responsibility |
|---|---|
| **Symthaea** | Translate dialogue into draft brief fields, expose ambiguity, propose alternatives and explain trade-offs. The resident confirms the brief. Model confidence is not evidence. |
| **Symtropy** | Provide deterministic constraint evaluation / scenario simulation within its demonstrated scope; call qualified domain solvers through pinned adapters. Rigid-body simulation is not building-energy or structural analysis. |
| **Sol-Atlas** | Supply permissioned, provenance-bearing site context as a read-only input. A map layer or coarse elevation source cannot substitute for a survey, drainage study or geotechnical investigation. |
| **Mycelix** | Coordinate consented revisions, approvals, quotations, material / supplier lineage, inspection records and supersession history. A signature does not establish physical truth. |
| **BIM / solver adapters** | Exchange structured building data and return domain-specific evaluations with explicit units, assumptions, versions, inputs, outputs, and validity limits. |
| **Resident and qualified professionals** | Retain design choice, site investigation, discipline-specific review, statutory approvals, and legal accountability. |

Keep this contract renderer-neutral and repository-neutral. Do not introduce a direct linked dependency on Sol-Atlas core without an explicit licensing and redistribution review; exchange versioned artifacts or contracts first.

## 3. Data contract

The normative shape for this proposal is [`home-design-brief.schema.json`](home-design-brief.schema.json). The [synthetic example](fixtures/example-home-design-brief.json) is test data only. The Rust CLI validates schema and implemented cross-field semantics with `cargo run -p symtropy-housing-evidence -- --validate-brief docs/built-environment/fixtures/example-home-design-brief.json`. This command validates the brief contract only; it does not generate a design or evaluate engineering safety.

A brief contains:
- project kind and an explicitly scoped target-budget range;
- site-readiness and jurisdiction resolution state, without embedding precise location data;
- anticipated household size, use patterns and functional accessibility goals, without names or diagnoses;
- requested spaces, quantities, area targets and adjacency preferences;
- a vector of named priorities with an ordinal resident-selected importance from 1 to 5;
- requirements whose origins and status are explicit;
- aesthetic / material preferences and data-sharing scope;
- immutable conflict-policy fields that require preserving contradictions rather than silently relaxing them.

SI units are used in this version. Monetary totals state their ISO-style three-letter currency code. Budget totals are user-provided planning bounds, not quotes or price predictions. Room areas are exploration targets, not automatically applicable code minima.

### Meaning of requirement classes

- `resident_non_negotiable`: retain as a resident's stated constraint and report when candidates cannot satisfy it. It does **not** authorize unsafe construction or override legislation.
- `resident_preference`: optimize or compare as a preference, preserving the resident's chosen importance.
- `exploration_goal`: compare feasibility and alternatives without implying a commitment.
- `external_requirement_reference`: identify a claimed outside requirement with a source pointer. Its presence does not make it applicable or verified.

Origin and verification state are separate fields. `resident_confirmed` means the resident confirmed their own intent; it is not professional or authority approval. Claims of `professionally_reviewed` or `authority_confirmed` must carry one or more `verification_evidence_refs` pointing to separate review/decision records. These IDs are references, not proof: the evidence pipeline must resolve them and verify provenance, scope, supersession and authority. A source pointer may itself be stale, inapplicable, or rights-restricted.

### Conflict policy

The schema fixes these semantics:
1. **Preserve and report** conflicts, infeasible goals, missing data and unsupported requirements.
2. Never delete, weaken or reclassify a resident non-negotiable simply to make a generator succeed.
3. Any proposed relaxation requires an explicit resident decision with a revision record.
4. No preference, importance value, optimizer score or resident decision can override applicable safety constraints or law.
5. If mandatory requirements are infeasible together, return a blocked/unsatisfied result with a minimal conflict explanation and candidate alternatives—not a misleading pass.

Priorities stay multi-dimensional. Do not collapse affordability, comfort, accessibility, resilience, environmental impact, and personal expression into one opaque “best home” score. Show Pareto alternatives and the reason for each trade-off.

## 4. Privacy and consent

The shareable brief intentionally has no coordinate fields, street-address field, title-deed field, occupant names, or health/diagnosis fields. Precise parcel/site material belongs in a separate access-controlled site-evidence record, and should only be attached under an explicit project access policy. Public maps and demonstration fixtures must use synthetic or deliberately coarsened locations.

`sharing_intent` is a declaration of intended sharing scope, not an access-control enforcement mechanism. Mycelix or the hosting application must enforce actual read/write permissions, revocation, retention, and audit. Setting `personal_data_included: false` does not replace a privacy review of free text; the review path must detect or reject accidental personal information before sharing.

## 5. Standards and tool interoperability

- [IFC 4.3.2.0](https://technical.buildingsmart.org/standards/ifc/ifc-schema-specifications/) is the initial open BIM target where supported. Pin the schema/version and record loss reports; do not imply that a design brief is itself IFC.
- [buildingSMART IDS](https://technical.buildingsmart.org/Projects/Information-Delivery-Specification-Ids/) can check required information in an exchange model. It is not a structural solver or a full legal-compliance engine.
- [EnergyPlus](https://energyplus.net/) and [OpenStudio](https://openstudio.net/) are candidate tools for whole-building energy workflows after geometry, weather, schedules, construction assemblies and adapter behavior are qualified.
- [Bonsai](https://bonsaibim.org/studio.html) is an existing IFC-native authoring option for human BIM review and hand-off; the project should integrate instead of rebuilding a complete BIM authoring stack.
- Jurisdiction-specific requirements must be linked to edition/effective date, exact clause, rights status and applicability review. This contract intentionally does not encode building regulations.

## 6. Acceptance gates for the first implementation

- [ ] JSON Schema passes its declared Draft 2020-12 meta-schema validation.
- [ ] The synthetic example validates; invalid field, missing-field, out-of-range priority and policy-weakening mutations fail.
- [ ] Unknown properties fail closed; no remote schema reference is necessary to validate the brief.
- [ ] The Rust CLI reports duplicate space IDs, unknown/self/duplicate adjacency relations, duplicate priority axes, invalid budget range ordering, duplicate requirement IDs, and unsupported survey-availability claims as semantic errors. Arbitrary contradictory free-text requirements still need a separate conflict-analysis step.
- [ ] A generation result points to the exact brief revision/digest and preserves the unresolved requirements and limitations.
- [ ] Candidate comparisons expose separate cost, comfort, accessibility, resilience and environmental measures with assumptions and units.
- [ ] Professional/authority verification states have explicit evidence-reference fields; references are resolved and authenticated only by the separate evidence workflow.
- [ ] IFC export/import and IDS checks are a later integration gate with a declared supported subset and round-trip/loss-report fixtures.
- [ ] No brief-only workflow labels a candidate as structurally safe, code compliant, permitted or construction ready.
- [ ] Exact-head CI results, including these new tests, finish successfully before any pass claim.

## 7. Deliberate non-goals

This contract is not a floor-plan generator, architectural drafting system, price database, building-code engine, finite-element solver, climate-data source, legal approval, or a claim of housing affordability. It does not choose the resident's values, infer a diagnosis, publish a household's precise location, or turn a render into a construction document.

## Dimension constraints and provenance

A space may declare optional dimension constraints against its local-plan X or Y rectangle extent. Each constraint has a stable ID, minimum/maximum bound, numeric value in metres, declared origin, and an explicit enforcement mode. Resident preferences can be advisory; a constraint changes candidate contract validity only when explicitly marked `must_satisfy`. An unmet must-satisfy constraint is surfaced as a semantic issue rather than silently relaxing the brief.

Professional- or authority-origin constraints require a source locator, edition/effective date or version, and clause/page/section location. These fields are declarations only: the verifier does not authenticate the source, establish that it applies to the particular parcel/project, or interpret a standard. Do not populate such a constraint from an unverified assumption; keep it unresolved until a qualified source review establishes the required value and applicability.

The v1 comparator measures axis-aligned rectangle extents, not wall-face clear dimensions, door clearances, net usable area, turning/circulation space, fire egress, or accessibility. X/Y axes depend on the stated candidate coordinate frame and are not automatically equivalent to architectural width/depth. Constraint diagnostics expose each zone instance and its source metadata; they are not a building-code determination.
