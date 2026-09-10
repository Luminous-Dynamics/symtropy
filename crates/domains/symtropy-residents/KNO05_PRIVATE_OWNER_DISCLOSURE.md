# KNO-05 — private observer-memory disclosure

`KnowledgeBase` is observer-owned. `KnowledgeClaim::source_id` is provenance and is not a substitute for that owner identity.

Before KNO-05, `ClaimPrivacy::Private` was evaluated only at the claim level as `requester == source_id`. That works when a person is both source and memory owner, but fails for sensor-derived knowledge: a resident could own a private memory sourced by `sensor:...` yet be unable to retrieve it through `KnowledgeBase::disclose`.

## Rule

For a `Private` claim:

- the owning `KnowledgeBase::owner_id` may inspect the remembered claim;
- the original `KnowledgeClaim::source_id` remains permitted by the claim-local rule;
- unrelated third parties are denied.

Other privacy classes keep their existing semantics: public, household, claim-specific consent, and life-safety restricted.

## Authority separation

- `owner_id` answers **whose memory/knowledge base this is**;
- `source_id` answers **where the claim came from**;
- `subject_id` answers **what the claim is about**.

Those identities must not be silently collapsed.

The owner override lives in `KnowledgeBase::disclose`, because the claim intentionally does not duplicate its enclosing knowledge-base owner. `KnowledgeClaim::may_disclose` remains a claim-local policy check.

## Tests

KNO-05 verifies that a private sensor-sourced claim is visible to its observer-owner, remains visible to its recorded source under the existing source rule, and remains hidden from an unrelated requester.

## Qualification

Implemented/static only until exact-head format/check/test/Clippy executes successfully.
