# Habitat Evidence Validity and Scoped Invalidation Benchmark v0.1

**Status:** Proposed synthetic assurance benchmark. Code and fixture are submitted for CI/review; no physical or regulatory claim is made.

## Paradigm shift

Move from **a report with a timestamp** to **an evidence artifact with a bounded claim, scope, validity status, provenance digest, dependency graph, and explicit invalidation behavior**. Freshness alone does not establish validity. A report can be recent but outside coverage; a valid report can become inapplicable after a terrain, geometry, method, supplier, or material revision.

The verifier can enforce a reviewed validity policy; it must never generate or approve its own real-world validity policy.

## Frozen synthetic fixture

File: `docs/research/fixtures/HABITAT_EVIDENCE_INVALIDATION_V0_1.json`

- 10 synthetic evidence/decision artifacts from terrain source through survey, site choice, concept, structural screening, procurement, as-built, commissioning and furnishings.
- Each artifact carries claim ID, scope ID, version, status, coverage flag, payload digest, validity-rule ID and explicit dependencies.
- 4 change events: terrain source revision, survey invalidation, material/supplier substitution, furnishing substitution.
- No real measurements, universal time-to-live, field probabilities or permitting rules are invented.

## Executable verifier

Package: `tools/habitat-evidence-invalidation`

The standalone Rust verifier pins exact fixture bytes, verifies each synthetic payload digest, rejects unknown fields and unknown/cyclic dependencies, and requires every `valid_for_claim` artifact to have complete declared coverage. A claim may be supported only when status, claim ID, scope ID, coverage and content digest match.

Each change event is evaluated as an independent counterfactual, not combined into an invented chronology. For each event, it calculates transitive downstream invalidation: the trigger artifact is marked as superseded in that event's impact record, and the listed descendants are invalidated. This per-event reporting prevents one scenario's effects from contaminating another scenario's precision measurement. Duplicate delivery with identical ID and payload is idempotent; conflicting reuse of an event ID is rejected. Output ordering is canonical, so semantically irrelevant event/artifact ordering cannot change the report.

A separate test oracle evaluates each artifact by tracing its own dependency ancestry to the trigger rather than calling the production forward-propagation routine.

## Frozen acceptance values

| Event | Expected affected artifacts | Precision intent |
|---|---:|---|
| Terrain source revision | 10 | All derived downstream decisions and receipts become invalid |
| Flood survey invalidation | 9 | Survey and downstream dependents invalidated; terrain source remains valid |
| Material/supplier substitution | 3 | Procurement superseded; as-built and commissioning receipts invalidated |
| Furnishing substitution | 2 | Furnishing package/install only; terrain/site/structure remain valid |

Counts describe this invented graph, not real-world project rework.

## Acceptance targets

- 100% agreement with an independent dependency-ancestry oracle across the frozen graph and 32 generated DAGs (4–24 nodes each, every node used as a trigger).
- 0 stale, revoked, out-of-scope, digest-mismatched or incomplete-coverage critical evidence promoted as valid.
- 100% invalidation of declared dependents after each seeded change.
- 0 unrelated invalidations in material- and furnishing-substitution precision controls.
- 100% deterministic output under input ordering permutations.
- 100% idempotency for exact duplicate event delivery; conflicting duplicate IDs always rejected.
- Cycles, unknown dependencies, unknown fields and malformed digests fail closed.

## Research anchors

- NIST's [Building Digitization and Semantic Interoperability](https://www.nist.gov/programs-projects/building-digitization-and-semantic-interoperability) project identifies barriers from heterogeneous machine-readable lifecycle data and manual mapping.
- [NIST IR 8356](https://csrc.nist.gov/pubs/ir/8356/final) discusses digital-twin trust and cybersecurity.
- USGS research on [data limitations in landslide susceptibility modelling](https://www.usgs.gov/publications/overcoming-data-limitations-landslide-susceptibility-modelling) notes constraints from difficult-to-collect and nonrepresentative inventories.

These sources motivate the direction but do not define universal engineering validity durations or acceptance criteria.

## Next step

Add separate versioned validity-rule contracts for real source types (for example, survey coverage and re-trigger conditions), reviewed by the relevant domain professionals. Then integrate Sol-Atlas source/scope/freshness receipts and Mycelix design revision, procurement substitution, inspection and commissioning provenance. Use measured project data before making claims about reducing real cost or time.
