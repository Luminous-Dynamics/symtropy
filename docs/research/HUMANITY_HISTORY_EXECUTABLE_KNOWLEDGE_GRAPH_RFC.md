# RFC: Humanity History as an Executable, Evidence-Aware Knowledge Graph

- **Status:** Draft for review
- **Scope:** Symtropy / Bevy / Sol-Atlas integration, with Symthaea as an evidence-constrained research assistant
- **Proposed first artifact:** `humanity-history-core` (renderer-independent Rust domain model and validator)
- **Non-goal:** claiming that one model, one civilization narrative, or one physics engine can explain all human history
- **Decision requested:** review the domain model, evidence contract, first vertical slice, and benchmark gates before implementation expands

> **Implementation snapshot (2026-10-18):** This draft branch now includes an initial `humanity-history-core` Rust package, typed source/claim/evidence/relation/asset records, numerical estimates with uncertainty intervals, reconstruction models and model-comparison groups, a JSON validation CLI, a synthetic fixture, an Antikythera research seed (7 sources, 9 claims, 30 quantitative estimates, 3 models, 1 comparison group, 4 point-interval probes, 13 paired deltas, 1 planned run with preregistered comparison rules, no bundled media), and 43 unit tests, including exact-decimal arithmetic checks, complete posterior-table coverage, and run-manifest negative controls. The comparison records also distinguish source-reported findings from independently reproduced results and explicitly unassessed axes. The tests have been authored but **have not been executed** in this environment because a Rust toolchain is unavailable. This is a prototype snapshot, not a passing build claim. The validator checks structural invariants; it does not establish historical truth or evaluate whether cited text semantically entails a claim.

## 1. Executive thesis

Build a versioned, queryable, spatially and temporally grounded model of human knowledge—not a static timeline, a list of great inventors, or a set of photorealistic scenes.

A historical item should connect:
- claims and counterclaims to sources;
- people, communities, institutions, works, artifacts, events, places, and time intervals;
- prior knowledge, independent invention, transmission, adaptation, and later consequences;
- a reconstruction's geometry, asset rights, material parameters, simulation recipe, and validation data;
- editorial and model-generated interpretations to the people, data, tools, and reasoning that produced them.

The primary output is not just a rendered scene. It is a navigable argument whose evidence, assumptions, uncertainty, methods, and provenance remain inspectable. Renderers are replaceable clients of this record.

The London Charter explicitly asks cultural-heritage visualizations to document sources, methods, interpretive decisions (paradata), dependency relationships, and the uncertainty of what is represented. The FAIR principles extend the same philosophy to machine-actionable discovery, interoperability, reuse, licensing, and provenance. This RFC makes those principles operational in a Rust-first simulation ecosystem.

References:
- London Charter: https://www.london-charter.org/
- London Charter, documentation and knowledge claims: https://www.london-charter.org/documentation.html
- FAIR principles: https://www.nature.com/articles/sdata201618
- CIDOC CRM: https://cidoc-crm.org/
- W3C PROV-O: https://www.w3.org/TR/prov-o/

## 2. Paradigm shifts

### 2.1 From timeline entries to evidence-bearing claims

A timeline entry tends to compress a complicated research question into one date and one sentence. Store claim records independently. A display card is a view over those records, not the canonical truth.

Each claim should include:
- stable ID and concise proposition;
- claim type (existence, date, attribution, mechanism, influence, interpretation, estimate, or counterfactual);
- temporal and spatial scope, including uncertainty;
- one or more evidence links with source passages, page/figure/record identifiers when available;
- support/challenge/qualify relations, with the source's own limitations;
- review state and decision history;
- author, revision, and provenance.

Do not collapse evidence quality into a single unexamined "confidence score." Keep source quality, independence, directness, dating uncertainty, scholarly disagreement, and review status separately inspectable. A numerical summary may be offered only with a documented method and limitations.

### 2.2 From one progress tree to a branching, plural knowledge graph

Many important advances were independently discovered, transmitted through several routes, adapted locally, forgotten, rediscovered, or produced collectively. Represent those relationships directly. An edge such as `influenced` must not mean the same thing as `preceded`, `shared a technique with`, `independently developed`, or `is a hypothesis about`.

Avoid a single civilizational ladder. Let users explore overlapping regional, linguistic, material, mathematical, and institutional networks without implying all societies followed the same path or that outcomes were inevitable.

### 2.3 From plausible visuals to documented reconstruction arguments

Every nontrivial visual element should be traceable to a source or declared as a modern design choice. A scene manifest links geometry, texture, sounds, text, and environmental parameters to source/license records and reconstruction decisions. Use visibly different presentation for documented observation, evidence-based reconstruction, hypothetical reconstruction, and counterfactual experiment.

Photorealism is not a quality metric by itself. A carefully annotated schematic may be more honest and educational than an attractive but overconfident reconstruction.

### 2.4 From time slider to bitemporal, uncertain history

Historical time is not always one known timestamp. Support intervals, alternative chronologies, calendars, date conventions, uncertain location geometry, and the difference between:
1. **valid time** — when the claim says something happened; and
2. **record time** — when the dataset learned, changed, or published the claim.

A time-scrub operation must not silently convert an uncertain date range into exactness. The user should be able to see how a historical map changes under competing date assumptions.

### 2.5 From "AI historian" to accountable research assistant

Symthaea may suggest sources, identify contradictions, propose graph edges, generate study questions, and explain mathematical models. It must not independently promote its generated text to attested fact.

Every machine-generated proposal should include:
- supporting source identifiers and precise spans where available;
- a model/tool version and reproducible run manifest;
- explicit uncertainty and missing-evidence notes;
- an acceptance/rejection trail from a human reviewer;
- a route back to the source independent of the model.

If source support is absent, the result is a research lead, not a published claim. Benchmark factual grounding separately from fluency.

### 2.6 From static depiction to executable experiments—with explicit domain boundaries

Users should manipulate reconstructed tools, machines, mathematical constructions, and environmental conditions where useful. Each simulation is a model with declared assumptions and reference tests, not proof that the historical event unfolded exactly that way.

Symtropy's rigid-body engine is a useful tool for mechanics and spatial interaction, not a universal solver for fluids, combustion, metallurgy, climate, social behavior, or every physical regime. Add domain-specific models behind narrow interfaces; publish each model's validity domain and errors.

Likewise, Symtropy's Phi/consciousness coupling must not be used as a measure of historical truth, cultural significance, or scholarly confidence. Those are separate epistemic questions.

### 2.7 From asset gathering to a provenance-preserving supply chain

Treat data and media ingestion as a reproducible build pipeline:
1. discover source and rights statement;
2. retain source ID, version/date, license/rights, and access conditions;
3. ingest original metadata without erasing it;
4. normalize into the internal schema with a recorded transformation;
5. validate fields, identifiers, time, geometry, and links;
6. emit machine-readable reports and content digests;
7. allow licensed redistribution only when the exact assets permit it.

Do not assume an open metadata license applies to an associated image, 3D scan, manuscript, or recording. Respect community protocols and culturally restricted knowledge in addition to ordinary copyright.

### 2.8 From an always-online product to a durable research instrument

Canonical records and small reconstructions should work offline and be portable. Store source snapshots or durable references when permitted, normalized data, schema version, deterministic test fixtures, and reconstruction manifests. Keep the data model independent of a specific GPU, renderer, hosting provider, or database.

## 3. System boundaries

### 3.1 Proposed crate boundary

Start with a small renderer-independent crate named `humanity-history-core`. This is a proposed name; check the workspace and crates.io before establishing a permanent public name.

Responsibilities:
- typed historical entities, claims, evidence references, relations, intervals, locations, and provenance;
- schema/version validation and invariant checks;
- graph queries for direct prerequisites, contested relations, paths, and time/place filters;
- stable serialization and migration policy;
- deterministic fixtures and report output.

Non-responsibilities:
- rendering, UI state, networking, web scraping, an AI model runtime, asset decoding, and domain-specific physics;
- an all-encompassing ontology that must model every discipline before anything ships.

Avoid pulling Bevy into this core crate. Bevy is a consumer of records and scene manifests. Sol-Atlas provides geographic/time exploration. Symtropy provides suitable executable models and deterministic scenario support. Symthaea operates above the evidence API and produces reviewable proposals. Mycelix can later provide attribution and community review mechanisms; it must not be required for local reads or initial validation.

### 3.2 Interoperability before ontology reinvention

Use a deliberately small internal model with explicit mapping points to:
- CIDOC CRM concepts for cultural heritage events, actors, objects, places, and time;
- W3C PROV-O concepts for entities, activities, agents, and derivation;
- FAIR principles for stable identifiers, discoverability, reuse, licenses, and provenance.

Do not directly adopt every external ontology class as a Rust type, and do not claim full CIDOC CRM conformance. The CIDOC CRM versions page lists v7.4 (August 2026) as a **draft** and v7.3.2 (March 2026) as a **draft**; pin the exact vocabulary version for each mapping and test it in fixtures rather than silently following a moving URL. Until that contract is selected and checked, describe our work as concept mapping, not CIDOC CRM conformance. Write mapping documents and fixtures for the first supported subset. Preserve external identifiers and source-native fields so future migrations do not destroy meaning.

### 3.3 Initial entity set

The first version only needs:
- `Source` — publication, artifact record, excavation record, dataset, oral-history record, or other evidentiary object;
- `Claim` — a proposition with scope and uncertainty;
- `EvidenceLink` — source passage/artifact anchor plus its relationship to a claim;
- `Agent` — person, community, institution, workshop, or collective, with attribution caveats;
- `Event` — discovery, manufacture, observation, migration, publication, translation, adoption, conflict, or other occurrence;
- `Place` — point, area, route, or uncertain geometry with coordinate reference system;
- `KnowledgeObject` — method, equation, notation, design, process, theory, language concept, artwork, or artifact;
- `Relation` — typed and scoped edge, optionally contested;
- `Reconstruction` — a versioned interpretive model linking claims and sources to visual/simulation assets;
- `SimulationRecipe` — parameters, units, implementation, domain validity, seed, expected results, measured results, and environment;
- `AssetRecord` — media URI or content digest, rights, source, transformation, and attribution.

All records require stable IDs and schema versions. Optional fields must remain optional; unknown must not be encoded as zero, false, or an invented exact date.

### 3.4 Claim relations

Use typed relations rather than a generic `connected_to` edge. Initial candidates:
- supports / challenges / qualifies;
- occurred_before / overlaps_in_time / chronology_disputed;
- used / adapted / independently_developed / transmitted_via;
- derived_from / translated_from / cites;
- implemented_by / enabled / constrained;
- located_at / associated_with;
- reconstructs / parameterized_from / validated_against.

Edges about influence, transmission, and causality must carry their own evidence or hypothesis status. Temporal succession alone does not prove influence or causation.

### 3.5 Source and asset rights

Every imported record must retain source-native identifiers and rights statements. Model separately:
- metadata reuse license;
- text/source access conditions;
- image/audio/model asset rights;
- attribution obligations;
- community access protocols and restrictions;
- whether redistribution is allowed.

Fail closed for distribution of assets with unknown reuse status. This must not imply that material is historically false or unavailable for private analysis; the gate concerns public packaging and permitted use.

## 4. Proposed minimal record (illustrative, not final API)

The first schema should be machine-readable (Serde JSON plus an explicit schema version is sufficient to begin). Example shape:

```json
{
  "schema_version": "0.1",
  "id": "hh:claim:example-001",
  "kind": "claim",
  "proposition": "A documented technique was used at this site",
  "claim_type": "artifact_use",
  "valid_time": {
    "start": {"value": -500, "calendar": "proleptic_gregorian"},
    "end": {"value": -300, "calendar": "proleptic_gregorian"},
    "precision": "century",
    "basis": "source-dating"
  },
  "scope": {
    "place_ids": ["hh:place:example-site"],
    "agent_ids": []
  },
  "evidence": [
    {
      "source_id": "hh:source:catalogue-record",
      "locator": "object record / figure / paragraph",
      "relation": "supports",
      "independence_group": "excavation-report-1",
      "note": "Direct material evidence; interpretation still under review"
    }
  ],
  "review": {
    "state": "proposed",
    "reviewed_by": [],
    "decision_log": []
  }
}
```

This is a shape proposal, not a historical assertion. The type system should model dates and missingness more rigorously than this JSON sketch; no production code should be generated by blindly copying the example.

Use explicit enums/newtypes where they prevent invalid states. Prefer stable identifiers over copied titles. Avoid an untyped bag of arbitrary JSON as the only domain model.

## 5. First vertical slice

Select one well-documented, bounded case study with a physical or mathematical object and connections across at least two domains. Candidate options include a mathematical method tied to a real artifact, a documented water-management or astronomical instrument, or a machine with enough technical documentation to validate its mechanism. A researcher should select the actual case only after evaluating source accessibility, rights, and historical significance.

The vertical slice must prove the full loop:

1. Import a small, explicitly licensed source/metadata fixture.
2. Store at least several distinct claims, source records, typed relations, and one documented disagreement or uncertainty.
3. Run a Rust validator without a renderer or network.
4. Query a timeline and place with interval uncertainty preserved.
5. Open a Bevy scene or schematic that links each meaningful reconstructed feature to its claim/source record.
6. Run one bounded interactive model with units, declared assumptions, a reference test, and reproducible parameters.
7. Ask Symthaea a question and ensure each factual answer links to a supported claim; unsupported responses are marked as unanswered/research leads.
8. Export the record, evidence report, asset manifest, and simulation recipe in portable formats.

A compact vertical slice with strong evidence is preferable to a large but unreviewable showcase.

## 6. Benchmark suite

All numeric targets below are **proposed acceptance targets**, not measured current performance. Establish a baseline, record the machine/toolchain/dataset, then decide whether a target should be tightened or relaxed.

### 6.1 Evidence and provenance correctness

- **Source traceability:** 100% of published nontrivial claims have at least one inspectable evidence link or are explicitly categorized as hypothesis/counterfactual.
- **Rights completeness:** 100% of distributable external assets have a recorded reuse status, provenance, and required attribution. Unknown rights block packaging.
- **Mutation tests:** removing a required source, changing an evidence relation, or replacing a known source digest must cause the expected validator failure or invalidate the dependent review state.
- **Contradiction preservation:** fixtures with conflicting sources must retain both sides in the query result; the validator must never silently discard a conflict to force a single timeline.
- **Citation evaluation:** human-annotated benchmark of Symthaea answers measuring supported-claim precision, claim coverage, and unsupported assertions separately. Start with a target of 0 unsupported high-confidence claims in the release-gating fixture suite, not a claim that general hallucinations can be eliminated.

### 6.2 Quantitative evidence and reconstruction comparison

- Numeric values are stored as decimal strings with units and source anchors; the validator rejects non-finite values and reversed uncertainty bounds. Sensitivity deltas and point-to-interval distances use exact base-10 decimal arithmetic and serialize as strings, avoiding binary floating-point artifacts.
- Declared point-vs-interval sensitivity probes report interval position and arithmetic distance only. They must be labeled descriptive, never as posterior odds or model probabilities unless a separate, explicit statistical model with validated assumptions is implemented.
- Paired point-estimate deltas report the difference between a source's named baseline and variant. They record the dependence/sensitivity basis and must not turn two preprocessing variants of the same underlying observations into independent replications.
- The pilot records all 13 displayed integer-N rows (349–361) from the source table rather than cherry-picking only 354–356. The full displayed column sums to 0.9998 due to four-decimal rounding; preserve the source values rather than silently renormalizing them.
- Analysis-run lifecycle records distinguish `planned`, `inputs_verified`, `executed`, `reproduced`, and `failed`. Executed runs must retain code revision, command, environment fingerprint, manifest/input/output digests, and a limitations statement. A reproduction record must name the source and target estimates. Digest syntax validation is not the same as computing hashes from actual bytes.
- The Antikythera pilot encodes source-reported discrete posterior values for N=354/355/356 under full and trimmed data; the computed deltas match simple subtraction of rounded table values and are not a local posterior reproduction.
- Credible/confidence intervals require explicit coverage and conditioning notes; a sensitivity variant must not be counted as an independent replication merely because it is a separate estimate record.
- Every reconstruction model points to its target claims and quantitative estimates, declares assumptions, and carries source evidence.
- Every model comparison contains at least two distinct models, named evaluation axes, and a declared decision rule. Per-model/per-axis assessments distinguish source-reported findings, independently reproduced results, hypotheses, and not-evaluated axes; independently reproduced results require a versioned validation artifact. The default case-study rule is no aggregate winner when axes or evidence are not comparable.
- Mutation suite: malformed numeric values, reversed intervals, missing coverage, source-less estimates, dangling model/estimate references, and duplicate comparison models must fail with specific validator codes.
- Numerical benchmark outputs must show interval coverage/calibration where ground truth or suitable reference data exist; otherwise report conditional model results without implying a calibrated probability of historical truth.

### 6.2 Graph correctness and scale

- Queries return typed paths and evidence for each edge; a path made of one unsupported edge must not be presented as fully attested.
- Seed corpus target: 1,000 claims / 10,000 relations; stress target: 100,000 claims / 1,000,000 relations.
- Proposed local query SLO: p95 <100 ms for common indexed lookups and p95 <500 ms for bounded, depth-limited relationship queries on the stress corpus. Publish hardware, graph shape, index settings, and memory use; revise targets from measured baselines.
- Property tests cover cycles, duplicated IDs, missing references, invalid interval boundaries, relation type compatibility, schema migrations, and deterministic serialization.
- Query results must be stable under different input-file orderings. Determinism claims are scoped to a declared environment and algorithm.

### 6.3 Time, place, and historical uncertainty

- Support interval-valued chronology, alternative calendars, imprecise places, and source-record time separately from asserted historical time.
- Round-trip tests preserve uncertainty, original calendar values, and source identifiers.
- Synthetic cases for conflicting dates and fuzzy geography must produce expected map/timeline membership; no silent midpoint conversion.
- All mapped coordinates declare a coordinate reference system or are explicitly marked unknown.

### 6.4 Simulation validity

- Every released simulation recipe records units, governing assumptions, validity domain, implementation version, seed, reference cases, tolerance, and measured error.
- Numeric tolerances are model-specific; do not impose the same error percentage on ray optics, rigid-body collisions, thermal processes, or archaeological reconstructions.
- Perturbation tests vary parameters within declared uncertainty and report outcome ranges.
- Replays record whether the guarantee is same-build, same-CPU, or cross-platform. Only claim a stronger guarantee after it is demonstrated.
- A historically plausible-looking animation without a validated model is labeled illustrative, never "proof."

### 6.5 Visual and application performance

- First target: a modest scene with 10,000 indexed records and 1,000 visible graph elements, with a smooth filter/time-scrub interaction on the reference machine.
- Rendering target: 1080p at 60 FPS for the first reference scene on declared hardware, with p95 frame time, startup time, peak memory, asset load latency, and GPU/CPU identified.
- Show the source/evidence panel without forcing expensive simulation to run; offline read-only mode remains usable.
- Do not make visual fidelity the only performance or quality target.

### 6.6 Learning efficacy

Compare against a conventional article/timeline control using a pre-registered protocol:
- immediate understanding of mechanism and chronology;
- delayed retention after at least one week;
- transfer to a novel related question;
- ability to distinguish evidence from hypothesis;
- accessibility and completion differences across relevant user groups.

Provisional research goal: a +10 percentage-point improvement in delayed retention on the first study, with confidence intervals and effect sizes reported. Treat this as a hypothesis to test, not a guaranteed product claim. If the result is null or negative, revise the interaction design.

### 6.7 Coverage and representation

Publish a reviewed corpus matrix by region, language, time period, domain, type of evidence, and level of scholarly certainty. Measure uncovered cells, source dependence, and reviewer diversity. Do not use one crude regional percentage as a proxy for fairness: record why areas are sparse (evidence scarcity, licensing restrictions, language access, or curation backlog) and fund concrete gap reduction.

### 6.8 Build and release gates

The first release gate is local and deterministic:
- format, lint, compile, unit and property tests for the isolated core;
- schema round-trip and invalid-fixture tests;
- rights and source-manifest validation;
- benchmark report with machine and fixture digests;
- no reliance on a green external runner for claims not locally demonstrated.

External CI remains important before merge, but queued checks are not passes. Publish exact commit SHA and command output in evidence reports. Fail closed.

## 7. Research and data strategy

Treat source providers as adapters, not as the canonical knowledge model. Initial research lanes:
- OpenHistoricalMap for time-aware historical geography where coverage/license fit;
- Wikidata for discovery and entity linking, followed by source verification;
- Europeana and institution repositories for cultural-heritage metadata, checking each media object's rights separately;
- scholarly/primary-source collections chosen per case study;
- quantitative history datasets for contextual measures with their definitions and caveats intact.

Related references:
- OpenHistoricalMap: https://www.openhistoricalmap.org/
- Wikidata query examples: https://www.wikidata.org/wiki/Wikidata:SPARQL_query_service/queries/examples
- Europeana rights guidance: https://www.europeana.eu/en/rights/usage-guidelines-for-metadata
- Our World in Data: https://ourworldindata.org/

No single data source establishes historical truth. Imported database assertions must maintain their source identity and be independently reviewable.

## 8. Work packages

### W0 — RFC and domain invariants
- Confirm crate ownership and name.
- Freeze the minimum entity/claim/evidence model.
- Write 15–25 valid and invalid fixture cases.
- Agree on claim-status language and provenance invariants.

### W1 — `humanity-history-core`
- Rust types and versioned Serde format.
- IDs, interval types, typed relations, validation errors, deterministic report.
- Unit/property tests for references, dates, graph links, rights completeness, and round-trip stability.
- No Bevy/network/AI dependencies.

### W2 — source adapter and evidence report
- One bounded source collection with an exact import manifest and license check.
- Preserve source-native IDs and raw fields.
- Emit before/after counts, rejects, warnings, and content digests.

### W3 — Bevy / Sol-Atlas view
- Timeline and geographic selection.
- Claim/evidence inspection and confidence/uncertainty display that does not rely only on color.
- Scene manifest to claims and asset sources.
- Accessible non-3D route to the same data.

### W4 — validated interactive reconstruction
- One domain-specific model with reference solution/test data.
- Parameter ranges tied to source or explicit hypothetical assumptions.
- Scenario replay and exportable run manifest.

### W5 — Symthaea research interface
- Retrieval from validated records only for evidence-bearing claims.
- Structured proposals for new claims/edges with source spans.
- Factuality benchmark and no-source abstention tests.
- Human review for promotion to published status.

### W6 — learning and external validation
- Accessibility review.
- Historical/technical expert review of the selected slice.
- Small learning study; publish methodology and uncertainty.
- Migration and preservation test for serialized records and manifests.

## 9. Risks and explicit anti-goals

- **Inventor monoculture:** avoid single-person attribution when discovery is collective, cumulative, disputed, or independently developed.
- **Presentism:** don't judge historical actors solely by present-day categories or flatten their own concepts into modern terminology.
- **False causality:** chronology and correlation are not proof of influence.
- **Photorealism bias:** high-resolution assets do not repair weak evidence.
- **AI laundering:** fluent text is not corroboration; no generated claim enters the authoritative graph without provenance and review.
- **Metric fetishism:** Phi, FPS, graph size, and scene count are not measures of truth, significance, or learning by themselves.
- **Ontology lock-in:** keep mappings/versioned schemas explicit; do not require the whole domain to fit one external vocabulary.
- **Rights failure:** metadata licensing does not imply media rights.
- **Unbounded scope:** no promise to recreate "every significant advancement" before there is a reviewable definition of significance, corpus coverage, and resource model.
- **Simulation overclaim:** every model declares where it applies and what it does not establish.

## 10. Definition of done for the first milestone

The first milestone is complete when a clean local checkout can validate a small, rights-cleared historical fixture, reject deliberately broken evidence/provenance/temporal records, query a supported and a disputed relationship, export an understandable audit report, and show a scene whose elements link back to the exact claims and sources used to create it.

The work must be testable without launching Bevy, starting an LLM, accessing a network, or waiting for CI. A demo scene is added only after the underlying records and validation tests exist.

## 11. Open decisions

1. Which well-documented case study offers the best combination of source access, global-context value, technical depth, and an executable model?
2. What small subset of CIDOC CRM and PROV-O should the first adapter map, and which concepts must remain local extensions?
3. Which fields distinguish scholarly review from community access or cultural protocols, and who is authorized to set them?
4. What exact query benchmark corpus and target machine should be the release baseline?
5. Which asset formats and preservation representations should be the portable minimum independent of Bevy?

## 12. Decision summary

Proceed with the evidence model and validator first, then one full end-to-end case study. Treat this as research infrastructure and an open, reviewable method—not as an AI-generated historical canon. The strongest differentiator is the coupling of source-grounded historical claims, uncertain time-space, executable models, versioned reconstructions, and measurable learning outcomes in one reproducible system.

The Antikythera pilot's next-stage raw-data and Bayesian reanalysis plan is documented in [ANTIKYTHERA_POSTERIOR_REPLICATION_PROTOCOL.md](case-studies/ANTIKYTHERA_POSTERIOR_REPLICATION_PROTOCOL.md). It is a protocol, not a result: the source files' exact license, inventory, and hashes remain unverified, and no posterior reproduction has been executed.
