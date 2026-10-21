# Furnishing dimension policy v0.1

**Status:** implementation prototype. This policy evaluates whether a structurally coherent dimension claim is admissible for a caller-declared use case. It is not a geometry, accessibility, legal-compliance, source-authenticity, or safety certificate.

## Why this is the next step

The interval classifier and the dimension-provenance contract now exist as separate primitives. The missing boundary is an explicit decision policy between a raw evidence record and a layout decision. A measured room dimension, manufacturer specification, seller estimate, nominal value, unknown value, missing record, and competing claims must not become interchangeable values.

This slice adds a Rust policy evaluator which accepts:
- the versioned dimension-evidence document;
- required subject/axis pairs;
- the evidence states allowed by each requirement;
- whether each requirement is mandatory or only useful for a provisional preview.

The accepted-state list belongs to the specific use case. It is not a universal evidence-quality ranking. An AcceptedByPolicy outcome means only that one structurally coherent record is allowed by the caller's declared policy. It does not verify the external source, validate a content digest against bytes, determine freshness, or establish measurement truth.

## Fail-closed outcomes

| Outcome | Meaning |
|---|---|
| ReadyByPolicy | All requested dimensions are admissible under their explicit state policies. This is only an evidence-policy result. |
| PreviewOnly | No mandatory requirement failed, but one or more dimensions are missing or provisional. |
| Indeterminate | A mandatory dimension is absent, explicitly unknown, or has multiple matching claims. |
| Blocked | The document is structurally invalid, policy configuration is invalid, or a mandatory claim uses a disallowed state. |

For a given subject and axis, multiple claims remain unresolved in v0.1 even when their bounds happen to match. The policy does not choose by timestamp, seller preference, narrower interval, or record order. Future evidence reconciliation requires an explicit method and tests; a conflict must not be resolved by an accidental implementation detail.

Numeric bounds may be retained for explanation when a disallowed known claim exists, but accepted_bounds_m() returns numeric bounds only for AcceptedByPolicy. Downstream hard-fit composition should use that method or preserve the disposition explicitly; it must not use a provisional bound to manufacture a pass.

## Responsibilities and limits

- Symtropy owns deterministic dimension and geometry policy evaluation.
- Symthaea may propose layouts or explain trade-offs but must not certify its own proposals.
- Mycelix remains authoritative for listing revision state, seller claims, and purchasing authority. Product prose must never be used as geometry.
- A successful policy result does not mean source authenticity or freshness was checked. Freshness should be deterministic against a caller-supplied evaluation time, use an explicitly scoped policy, and never depend on the machine wall clock.
- This module does not yet assemble the full four-axis IntervalRect from separate room, product, and placement evidence records. That is a follow-on integration; until then the geometry classifier and evidence policy remain explicit separate gates.
- Functional envelopes (drawer swing, service access, transfer, approach, door swing, and maintenance) remain separate from furniture footprints. Unknown mandatory task envelopes must stay unresolved.
- Accessibility criteria require a versioned rule pack appropriate to jurisdiction and task. ISO 21542:2021 is a relevant built-environment reference, but this prototype does not encode or certify its clauses.
- buildingSMART IDS is useful for alphanumeric IFC information requirements; geometry and measurement uncertainty remain separate evidence contracts.

## Proposed qualification gates

- 0 mandatory missing/unknown/ambiguous dimensions promoted to ReadyByPolicy.
- 100% of structurally invalid evidence documents block promotion.
- 100% of disallowed evidence states block mandatory requirements while remaining explicitly visible in diagnostics.
- 100% deterministic reports for the same document and policy inputs.
- 100% preservation of all matching evidence IDs when claims conflict.
- No source-authenticity, freshness, accessibility, field-accuracy, or user-outcome claims from structural-policy tests alone.

The tests use only the existing synthetic fixture. They do not establish that any real room, catalogue, seller, or product claim is correct.
