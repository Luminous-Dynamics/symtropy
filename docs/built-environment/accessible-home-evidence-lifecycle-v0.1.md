# ACCESS-HOME-EVIDENCE-LIFECYCLE-001 — scope-bound evidence and invalidation v0.1

**Status:** draft synthetic contract. No physical home has been assessed by this proposal. Not a compliance, usability, accessibility or life-safety certificate.

## Why a temporal model is necessary

An evidence result is not a permanent property of a room or product. Clearance evidence may cease to apply when furniture moves or a door is altered; an alert-path claim may become stale when the device, power source or network dependency changes. A confirmed measurement is valid only for the declared scope and conditions a reviewer actually checked.

ISO 21542 covers access, circulation, egress and fire evacuation, as well as residential-unit recommendations. Its official status page continues to show ISO 21542:2021 as published and under review after the 3 September 2026 close-of-review stage. The standard's latest lifecycle status and jurisdictional applicability should be checked before real use. WHO's Housing and health guidelines also identifies accessibility as a housing-health concern and calls for stronger standardized/longitudinal evidence; the accessibility review underpinning the 2018 guideline covered literature through April 2018, so any new study needs a refreshed search.

Sources:
- [ISO 21542:2021 status and scope](https://committee.iso.org/standard/71860.html)
- [WHO, Housing and health guidelines](https://www.who.int/publications/i/item/9789241550376)
- [WHO accessibility systematic review](https://www.ncbi.nlm.nih.gov/books/NBK535292/)

## Paradigm shift

Treat evidence as **a versioned claim with explicit dependencies and an invalidation lifecycle**, not a Boolean badge. Instead of asking "was this checked?", ask "what exactly was checked, for which configuration facets and condition, by whom, when, and what subsequent change could make that claim stale?"

The model separates:
1. **Claim truth/status as declared** — verified-for-scope, known barrier, unknown, proposed, measurement pending or professional review pending.
2. **Currentness** — current for declared facet revisions, needs revalidation, expired/review-due or superseded.
3. **Task/path consequence** — derived from current claims only, with hard constraint handling still separate.
4. **Audit trail** — observed evidence is never silently rewritten or deleted just because configuration changed.

## Data model

### Configuration facets
Use opaque scope IDs and version IDs for facets such as furniture placement, door/clearance geometry, control/device model, power supply, network dependency, and support arrangement. Public fixtures use invented identifiers only. For real sites, use resident-local or restricted IDs; never hash/publish a sensitive floor plan merely to make a scope version.

Each facet has a current revision. A change event lists explicit facet_id, from_revision_id, and to_revision_id. Events must form a causal sequence: a change cannot claim to move from a revision other than the currently known revision for that facet.

### Evidence claims
Each claim has an opaque ID and an assertion_key identifying the proposition being assessed within its scope, plus source/reference, reviewer role, observation time, optional review-due deadline, evidence state, and bindings to the facet revisions on which it depends. Conflict detection compares claims only when both scope_ref and assertion_key match; distinct assertions about the same room or system must not conflict merely because their evidence states differ. The bindings decide whether each claim is still current.

A claim becomes **needs_revalidation** when any bound facet changes. Changes to a separate facet that the claim does not depend on must not invalidate it without an explicit dependency. This avoids both unsafe stickiness and needless global invalidation.

### Supersession
A newer claim may supersede an older claim with the same scope_ref and assertion_key, preserving the old claim and the reason. A supersession chain must be acyclic. If two incompatible claims with the same scope_ref and assertion_key both purport to be current and neither supersedes the other, the conflict remains unresolved; the evaluator must not arbitrarily pick the latest timestamp as truth. Claims with different assertion_key values are distinct propositions and are evaluated separately.

### Path dependencies
A path references required evidence claims. It is evaluated only from current claims whose scope bindings match current facet versions and whose review windows have not lapsed under the declared policy.

- A current required known barrier blocks the path.
- Otherwise, any required claim that is stale, expired, superseded without a current replacement, unknown, proposed or review-pending makes the path not evaluable.
- Only current, sufficiently evidenced required claims can make a path complete-for-fixture.
- A stale known-barrier claim does not automatically block the current path: it is evidence about an old configuration. The current path remains not evaluable until its status is re-established; any other independently current required barrier still blocks.
- Replaying the same valid initial state and event sequence must yield the same final statuses and receipt.

These are synthetic decision rules. They do not prove the underlying claim is true and do not replace site measurement, resident trials or qualified safety review.

## Change and expiry rules

1. Accept only events whose from_revision_id matches the known current facet revision.
2. Advance only the named facet revisions; retain event order and event provenance.
3. Re-evaluate each claim's facet bindings. A mismatched binding becomes needs-revalidation; do not mutate its original evidence to imply that the reviewer evaluated the new configuration.
4. Review-due/expired evidence is non-passing until an explicit policy and a fresh review establish currentness. Dates should not imply validity on their own.
5. Recompute all paths and failure-mode results that depend on affected claims.
6. Record why each finding changed, with event ID and facet IDs—not raw resident notes, exact addresses, or private plan contents in public logs.
7. Resolve supersession conflicts explicitly; never silently discard history.

## Required mutation tests

- A route-clearance claim bound to furniture-r1 becomes stale after furniture-r1 → furniture-r2.
- A notification-device claim bound only to device-r4 remains current after an unrelated furniture event.
- An event with a false from_revision_id is rejected.
- An expired review claim cannot be treated as current without a qualified refresh.
- A path depending on one stale required claim becomes not-evaluable even if the old claim was fixture-complete.
- A currently declared hard barrier still blocks even if another independent required claim is stale.
- A stale old known-barrier claim alone does not determine the current path state.
- Cyclic supersession, duplicate event IDs, unknown facets, unknown claim references, and conflicting current claim lineages fail closed. Distinct assertions within one scope must not conflict solely because their evidence states differ.
- Replaying an identical event sequence is deterministic; changing event order where causality matters produces an error, not a silently different result.

## Benchmark targets

- 100% of claims with a facet revision mismatch are non-current.
- 100% of events with a wrong starting revision fail validation.
- 100% of affected required path assessments are recomputed after change.
- Zero unrelated claims invalidated solely by a change to a facet outside their declared bindings.
- 100% of review-due/expired claims stay non-passing until revalidation.
- 100% of invalidation findings explain the event/facet responsible.
- 100% of supersession lineages are acyclic; incompatible current claims remain explicitly unresolved.
- Identical initial state plus identical event sequence yields an identical report under a pinned build.
- Zero public receipts contain real resident/site payloads, private measurements, raw plan hashes or identifying revision IDs.

## Privacy boundary

This lifecycle contract composes with the resident privacy work in issue/PR #1722. Public tests use synthetic fixtures only. For actual work, revision identifiers, evidence payloads, detailed layouts and identity mappings stay local or in a specifically authorized restricted store. A digest is evidence of byte identity, not evidence truth, safety or anonymity.

## Integration with Symtropy

- Resident brief/planner: issue #1687 and PR #1688.
- Housing evidence/jurisdiction contract: PR #1596.
- v0.1 task-chain triage: issue/PR #1695/#1696.
- Task network: PR #1705.
- v0.2 Rust semantic validator: PR #1711.
- Typed hard constraints/preferences: PR #1715.
- Participatory longitudinal outcome protocol: PR #1717.
- Privacy contract: issue/PR #1722.

The first deliverable here is a schema and synthetic fixture; a Rust event-replay evaluator should follow only after the contract is reviewed. No model output qualifies a real home or its life-safety systems.
