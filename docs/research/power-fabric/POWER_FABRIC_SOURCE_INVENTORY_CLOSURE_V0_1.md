# Power Fabric scoped source inventory closure v0.1

**Status:** research contract and synthetic qualification corpus; no provider adapter or independent source verifier claimed  
**Date:** 2026-11-15  
**Owner:** #5322  
**Parent chain:** #5243 → #5245 → #5262 → #5303  
**Composes:** #5215, #5218, #5282, #5283, #5171 and #5241

## 1. Purpose

A mapping ledger can be internally complete while the source extraction from which it was built is incomplete.

The existing standards adapter can require one mapping record for each supplied source semantic reference, compare declared counts and expose unsupported/ambiguous mappings. The build receipt can bind the adapter's inputs and output bytes. Neither mechanism proves that the input inventory contains every record in the extraction scope.

Examples include:
- a paginated API traversal that stops one page early;
- an API whose records change between page requests;
- a filter or permission boundary that was not recorded;
- a terminal continuation token that was never observed;
- a source count that disagrees with the enumerated page records;
- a repeated cursor returning different bytes;
- a source semantic record that has no mapping disposition.

Power Fabric therefore needs an independently inspectable **source inventory closure receipt**. The receipt qualifies a named source inventory within an explicit scope and traversal contract. It must not claim universal completeness or physical truth.

## 2. Two independent questions

Keep these axes separate:

1. **Inventory closure:** did the extraction enumerate and account for every source record within the declared source, snapshot, filter, namespace, permission and time scope?
2. **Semantic mapping:** was every enumerated in-scope source semantic item preserved, explicitly transformed/lost, unsupported or ambiguous?

A source inventory can be complete within scope while containing an unsupported semantic item. In that case the inventory closure is complete, but adapter semantic coverage is incomplete. Conversely, a flawless mapping ledger over six received records cannot prove that six is the full source inventory.

## 3. Scope contract

An InventoryScope binds:
- source-system identity and identity namespace;
- source adapter/profile ID and version;
- exact query/filter scope digest and declared filter descriptors;
- included record classes/namespaces;
- declared time window where relevant;
- explicit exclusions, permission-limited partitions and inaccessible classes;
- source ownership and applicable snapshot policy.

Scope is not an unbounded claim. An exclusion narrows what the receipt claims to cover, and the exclusion itself is part of the committed scope. Unknown permissions or unknown filters make completeness unknown; they are never interpreted as empty datasets.

The source query itself may contain sensitive operational information. Public records should bind a digest and non-sensitive description rather than expose credentials, customer identifiers or private query contents.

## 4. Snapshot and traversal contract

Supported traversal kinds:

- **ImmutableManifest:** an immutable export with an exact file/record manifest and terminal boundary.
- **PointInTimeSnapshot:** source provider supplies a stable snapshot token/revision.
- **CursorPaginatedSnapshot:** page requests are bound to one provider snapshot and an explicit cursor chain.
- **OffsetPaginatedSnapshot:** pages are bound to one provider snapshot and the declared offset/count contract.
- **BestEffortMutable:** traversal is not guaranteed to observe one stable source snapshot.

BestEffortMutable cannot produce complete_within_declared_scope. It is unknown unless an external reconciliation step supplies a stronger snapshot contract.

Each page binds:
- zero-based page index;
- request cursor digest (null only for the first page);
- next cursor digest (null only for a terminal page);
- terminal marker;
- exact source revision/snapshot ID;
- raw page artifact digest;
- record count and namespaced source-record identity list;
- received-at time and evidence/source-receipt reference.

The verifier checks contiguous page indices, first-page origin, exact cursor continuity, one terminal page at the end, no terminal marker on earlier pages, exact source snapshot continuity, unique page IDs, record/page membership and count reconciliation. The page's cursor relationship is evidence about the traversal; the caller cannot self-authorize it.

## 5. Record inventory and semantic accounting

Every enumerated source record has:
- namespaced source identity (namespace + source ID);
- source record payload digest;
- source record class/type;
- the page that delivered it;
- source revision/snapshot.

The same local identifier in different namespaces is not a collision. The same namespace and identifier appearing twice in one snapshot is a conflict unless the source profile explicitly defines a stable versioned duplicate representation.

Every enumerated in-scope record must have exactly one semantic disposition:
- mapped;
- loss_explicit;
- unsupported;
- ambiguous;
- excluded_with_reason;
- unavailable_with_reason.

Missing dispositions are incomplete. Duplicate dispositions are conflicted. Unsupported/ambiguous records remain in the inventory commitment and make semantic mapping incomplete; they are not erased from the source count.

Explicit exclusions of whole partitions/classes remain part of the scope contract. A record-level excluded_with_reason disposition is not a substitute for silently excluding a whole inaccessible source partition.

## 6. Closure states

The closure state is computed by a verifier; it is not a field a producer may set.

- **complete_within_declared_scope:** scope and snapshot contracts are explicit and admitted; traversal is stable; all pages and cursor links are present; a terminal boundary is observed; source page/record totals reconcile when published; every enumerated source identity is unique; and every record has exactly one semantic disposition.
- **incomplete:** a page or record is missing, the cursor chain is broken, counts mismatch, a required terminal boundary is absent, or a source record lacks one disposition.
- **unknown:** source snapshot/closure capability, permission scope, filter scope or evidence admission cannot be established. Unknown is not complete.
- **conflicted:** revisions change mid-traversal, same cursor yields contradictory bytes, namespaced identities conflict, page content contradicts its snapshot, or multiple source snapshots cannot be reconciled.

Semantic status is returned separately:
- complete;
- complete_with_declared_loss;
- incomplete_unsupported_or_ambiguous;
- unknown.

## 7. Historical cutoffs and evidence admission

All page receipts, source snapshot evidence and exclusions are checked at an explicit as-known-at cutoff. A page received after that cutoff cannot change the earlier inventory projection. A later cutoff may admit it and produce a different manifest/receipt revision.

The generic evidence admission, source-artifact integrity and point-in-time trust policy belongs to #5171. This workstream consumes its typed result; it does not reimplement signatures, source authentication, clocks or evidence dependency independence.

A digest is a content identifier under a declared byte boundary. It does not prove that the source system is truthful or that the bytes represent the physical world.

## 8. Dependency and build integration

Emit stable dependency identities and fingerprints for:
- source system and namespace;
- adapter/profile version;
- query/filter scope digest;
- source snapshot ID/revision/token;
- traversal contract version;
- page identity, cursor chain and page payload digest;
- enumerated record identity/payload digest;
- exclusion/permission manifest digest;
- record-to-semantic-disposition ledger digest;
- closure verifier ID/version and source trust receipt.

The inventory manifest and closure receipt become upstream artifacts pinned by the Power Fabric build lock (#5258/#5262). The standards adapter consumes the enumerated record set and disposition ledger (#5215). Source page, filter, permission, snapshot or verifier changes flow through the existing invalidation graph (#5218/#5283), selectively staling dependent outputs. Do not build another dependency engine.

A historical closure receipt is immutable. Regeneration creates a new manifest/receipt revision, and every current projection states whether it is current, stale, incomplete, conflicted or unknown.

## 9. Threat model

The contract assumes ordinary errors or adversaries may:
- omit the first, middle or final page;
- replay a prior page or cursor;
- fork or skip continuation tokens;
- alter a filter after the source query is issued;
- use offset pagination while source records are inserted or deleted;
- cross a source revision boundary during traversal;
- reuse a cursor with changed content;
- hide a permission-limited partition;
- falsify an expected source count;
- duplicate or relabel source identities;
- map only the records that are convenient;
- submit late evidence into an earlier historical replay;
- mark the inventory complete because a schema contains a boolean.

Controls:
- use source-provider snapshot consistency evidence where available;
- record every page/cursor edge and raw page digest;
- verify snapshot continuity and terminal traversal independently;
- compare source-declared counts when available, without treating counts as sufficient proof;
- namespace identifiers and reject duplicate identity within a namespace;
- require one and only one semantic disposition per enumerated record;
- disclose filters, permissions and exclusions;
- fail closed on unknown snapshot or access scope;
- preserve historical receipts and as-known-at replay.

## 10. Frozen P0 qualification cases

The machine-readable fixture includes positive controls for immutable exports, stable paginated API snapshots, explicit scope exclusions, cross-namespace identifier reuse, unsupported-but-accounted records and page-order normalization. Negative controls include missing first/middle/terminal pages, cursor repeats/skips, count mismatch, source revision drift, contradictory page bytes, duplicate namespaced IDs, missing filter/permission scope, missing/duplicate semantic dispositions, late pages and best-effort mutable pagination.

Targets:
- 100% expected closure state and diagnostic agreement on the frozen corpus;
- zero false-complete results for omitted pages, unstable snapshots, hidden filters, unknown permissions or absent terminal boundaries;
- 100% page/cursor/count reconciliation when source contracts publish those values;
- 100% unique namespaced record identity and one-to-one disposition accounting;
- zero silent semantic drops;
- 100% point-in-time replay correctness under late page arrival;
- byte-identical output for equivalent page manifests under the declared serialization profile;
- independent verifier reproduces closure state from exact pinned page artifacts and source snapshot evidence.

Reject-all and accept-all controls must both fail. Report the worst failure family, not only a pooled percentage.

## 11. Implementation sequence

**P0 — contract and oracle:** this specification, schema, fixture corpus, explicit states, mutation matrix, threat model and independent known-answer oracle.

**P1 — Rust closure kernel:** typed scope/snapshot/traversal/page/record/disposition contracts, deterministic cursor-chain and page-membership validation, bitemporal cutoff projection, duplicate detection, closure/semantic status separation and source/evidence verifier interfaces.

**P2 — adapter/build integration:** require a closure receipt before an adapter can claim complete source inventory; pin the inventory manifest and verifier version in the build lock; feed source dependencies to #5218/#5283.

**P3 — real provider adapters:** first use rights-cleared static IEC SCL/CIM/IFC example packages and documented snapshot APIs; then add real source connectors only where permissions and stable snapshot semantics can be demonstrated.

**P4 — independent reproduction:** a second implementation verifies the manifest, cursor chain, source record inventory, semantic ledger and closure result from frozen source artifacts.

## 12. Claim ceiling

A complete_within_declared_scope receipt establishes only that the enumerated source inventory is closed under the named scope, source snapshot contract, traversal evidence and verifier. It does not prove the scope captures every real-world fact, that excluded data are irrelevant, that source claims are truthful, that the physical installation matches the records, or that the electrical design is safe or compliant.

## References

- SLSA Build Provenance v1.2 defines software build inputs and resolved dependencies, while its specification explicitly treats dependency completeness as best effort; this is a useful interoperability pattern, not an electrical source-inventory closure oracle: https://slsa.dev/spec/v1.2/build-provenance
- SLSA Build Requirements distinguish provenance completeness, authenticity and accuracy; those are separate properties and should not be collapsed into a digest match: https://slsa.dev/spec/v1.2/build-requirements
- in-toto Attestation Framework provides a stable statement/attestation envelope for binding verified predicates to subjects; Power Fabric should reuse the envelope and publish its own scoped inventory predicate: https://in-toto.io/docs/specs/
- buildingSMART IDS defines machine-readable, use-case-specific requirements for IFC objects, classifications, properties, values and units. It demonstrates the value of explicit exchange scope and validation, but does not prove the completeness of an external extraction: https://technical.buildingsmart.org/Projects/Information-Delivery-Specification-Ids/
