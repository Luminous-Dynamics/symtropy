# Furnishing dimension evidence v0.1

**Status:** synthetic contract prototype. This establishes a traceable shape for dimension assertions; it does not authenticate a source, establish source freshness, or automatically promote a layout constraint to pass.

## Paradigm shift: dimensions are claims with evidence, not naked scalars

A renderer may need a single width for drawing, but a planning decision needs to know what that width means. A tape/laser observation, a manufacturer specification, a seller estimate from a listing, a nominal catalogue dimension and an unknown value are not interchangeable.

This contract represents a dimension as:

- A stable evidence ID and subject identity/revision reference.
- An axis (`x`, `y`, `width`, `depth`, `height`) and a bounded interval in metres.
- A state (`measured`, `manufacturer_specified`, `seller_estimated`, `nominal`, or `unknown`).
- A source kind, stable source reference and capture time where a source exists.
- A method that is compatible with the claimed state.
- A recording timestamp and optional note; content SHA-256 can bind a source snapshot when the caller has the exact bytes.

An `unknown` record must not supply numeric bounds. In the fixture it has a stable ID and recording timestamp, but no evidence source and no guessed value. A future policy layer should convert unknown required dimensions into indeterminate/blocked task decisions.

## Files added

- `fixtures/interiors/furnishing_dimension_evidence_v0_1.schema.json` — JSON Schema Draft 2020-12 contract.
- `fixtures/interiors/furnishing_dimension_evidence_v0_1.json` — entirely synthetic examples of manufacturer-specified, measured, seller-estimated and unknown dimensions.
- `tools/furnishing-layout-audit/src/dimension_evidence.rs` — dependency-free Rust structural validator and mutation tests.
- `tools/furnishing-layout-audit/src/lib.rs` — public module registration.

The runtime validator checks schema version, document units/status, unique evidence IDs, subject/axis/state vocabulary, bound ordering and finiteness, positive extent minima, compatible source kinds/methods, required source metadata, and optional SHA-256 shape. JSON Schema additionally describes the required conditional fields and date-time format. The runtime does not currently include a general JSON Schema engine, so this is not a claim that every JSON Schema keyword is executed by Rust.

## Research and industry comparison

[Planner 5D's current business planner](https://planner5d.com/business/furniture-room-planner) describes a 2D/3D catalogue-linked planner with to-scale products, configurable SKUs, live pricing, cart/quote handoff and AI floor-plan recognition. Its [AI placement feature](https://planner5d.com/use/ai-furniture-placement) describes generating and comparing layouts and editing the results. These are strong incumbent capabilities to respect; this work does not attempt to beat their rendering, catalogue breadth, generation experience or sales integrations.

[SceneEval (WACV 2026)](https://openaccess.thecvf.com/content/WACV2026/html/Tam_SceneEval_Evaluating_Semantic_Coherence_in_Text-Conditioned_3D_Indoor_Scene_Synthesis_WACV_2026_paper.html) evaluates explicit requirements (object counts, attributes and spatial relationships) separately from implicit plausibility (support, collisions and navigability). A mature evaluation should preserve these independent dimensions rather than collapse everything into a style or realism score.

[buildingSMART IDS 1.0](https://www.buildingsmart.org/Standards/Bsi-Standards/Information-Delivery-Specification-Ids/) is a standard for computer-interpretable IFC information requirements, including types, properties, classifications and quantities. buildingSMART states that IDS is limited to alphanumeric information and does not cover geometry; therefore use standard information vocabularies where appropriate while keeping measured geometry, uncertainty and source provenance explicit.

[ISO 21542:2021](https://www.iso.org/standard/71860.html) covers built-environment accessibility and usability, including access, circulation, egress and evacuation. This evidence schema does not implement the standard. Accessibility rule packs need exact applicable clauses, jurisdiction/version, task scope, measurement assumptions and professional review.

The differentiator to pursue is an auditable chain from room/product evidence through bounded geometry and task feasibility to the decision the resident sees. It is not enough to draw a plausible room or show a retailer's dimensions beside it.

## Required next integration

1. Attach evidence records to the exact subject revision and axis used by an interval geometry input.
2. Add an explicit evaluation policy for acceptable states per use case. For example, a hard fit gate may require a measured room boundary and a manufacturer-specified product dimension with a documented tolerance; a stylistic preview may allow a seller estimate but must label the result provisional.
3. Evaluate source freshness deterministically against a caller-supplied evaluation time. Never use machine wall-clock time inside a replayable evidence receipt.
4. Preserve all conflicting observations. Do not select the latest, most precise-looking or seller-favorable dimension as truth without policy and provenance.
5. Bind source snapshots by content digest where available; validate that the digest matches the bytes outside this structural validator.
6. Compose validated evidence with the interval classifier, then keep `DefinitelyClear`, `DefinitelyOverlapping` and `Indeterminate` distinct in every downstream API/UI.

## Proposed acceptance gates

| Measure | Target |
|---|---:|
| Duplicate evidence IDs | 0 |
| Invalid known-state/source combinations | 0 |
| Unknown records with numeric bounds | 0 |
| Missing or malformed bounds on known states | 0 |
| Malformed optional SHA-256 values | 0 |
| False source-authenticity claims | 0 — structural validation must never be described as authentication |
| Stale source promoted without caller policy | 0 after freshness integration |
| Evidence loss through projection or client adapter | 0 after end-to-end integration |

The contract is a foundation for deterministic provenance, not yet an end-to-end proof system. A passing record only means it is structurally coherent. Source authenticity, dimension accuracy, freshness, interval-classifier correctness, accessibility compliance and user outcomes remain separate claims requiring their own evidence.
