# SEL-10A — Concept-Agnostic Lineage-Divergence History V1

Status: source contract for `symtropy-evolution-core`.

This tranche materializes complete, auditable lineage-divergence history without applying a species concept.

The governing boundary is:

`reproductive isolation != persistent lineage-divergence history != species status != historical speciation event`.

SEL-10A therefore emits no species label, no species-model result, and no speciation-event claim.

## 1. Outcome-free history design

`LineageDivergenceHistoryDesign` is declared before outcome-bearing generation records are consumed.

It freezes:

- one semantic history ID;
- an ordered pair of distinct lineage authorities;
- an exact inclusive start/end generation interval;
- the derived exact generation count;
- frozen evidence protocols for lineage membership, lineage persistence, ancestry relation, context, population structure, demographic episode census, demographic episode evidence, recontact, gene flow, and lineage fusion;
- context policy;
- missing-data policy;
- a completeness authority;
- the built-in content-digested V1 classification rule.

The interval must contain at least two generation coordinates.

The built-in V1 rule is code-owned. Callers cannot provide a post-hoc classification rule.

## 2. Exact inclusive generation coverage

A materialized history requires exactly one generation record for every coordinate in the frozen inclusive interval.

Caller order is non-semantic. Records are canonicalized by generation.

The following fail:

- omitted generation;
- duplicate generation;
- undeclared generation;
- noncontiguous restored history;
- generation count inconsistent with the frozen interval.

A generation is represented as either:

- `Observed(...)`; or
- `Unavailable { generation, reason }` under the frozen missing-data policy.

`FailClosed` rejects unavailable required generations/evidence.

`ReportInsufficientEvidence` preserves unavailable evidence explicitly and prevents promotion to a clean persistent-divergence status.

## 3. Current-revalidated population trajectory points

Every observed generation carries exact A and B `PopulationTrajectoryPoint` values together with the fresh current schema and population state used during capture/replay.

SEL-10A calls `PopulationTrajectoryPoint::validate_current(...)` for both lineages.

Therefore a serialized trajectory-point digest is not current authority. A stale population state fails replay.

The point generation must equal the lineage-history generation.

SEL-10A does not claim that a `PopulationGeneration` is universal world time. It remains the explicit generation coordinate supplied by the underlying evolution/population evidence lineage.

## 4. Exact lineage-subject binding

Field position alone is not lineage identity.

For each observed generation:

- lineage-membership evidence binds the exact lineage authority and exact trajectory-point digest;
- lineage-persistence evidence binds the exact lineage authority;
- ancestry-relation evidence binds the ordered lineage A/B authority pair;
- recontact evidence binds the ordered lineage pair;
- gene-flow evidence binds the ordered lineage pair;
- fusion evidence binds the ordered lineage pair.

A restored record that transplants otherwise-valid evidence onto another lineage or lineage pair fails local validation before a canonical history digest may be obtained.

The ordered A/B pair is intentional representation identity. Swapping lineage orientation is a different history subject unless a future adapter explicitly proves an orientation equivalence.

## 5. Frozen evidence protocols

Each persisted evidence edge carries both:

- the frozen protocol authority; and
- the evidence authority produced/qualified under that protocol.

Restored objects re-check protocol identity.

The protocol/evidence authority is an auditable trust binding. Unless a specific upstream typed capability is consumed and replayed, SEL-10A does not pretend that a hash independently proves the biological proposition encoded by that authority.

This is especially important for lineage membership, lineage persistence, ancestry interpretation, context interpretation, population-structure interpretation, recontact, gene flow, fusion, and episode completeness.

## 6. Context policy

V1 supports:

- `ExactAcrossInterval`; and
- `DeclaredTrajectory { authority }`.

Under `ExactAcrossInterval`, the exact context evidence authority must be identical across every observed generation.

Under `DeclaredTrajectory`, context evidence may vary under the explicitly frozen trajectory authority.

The declared trajectory authority is an auditable interpretation/qualification binding. SEL-10A does not independently infer context continuity from an opaque digest.

## 7. Complete demographic episode accounting

Every observed generation carries a `demographic_episode_census` evidence binding and a canonical semantic-ID-sorted vector of zero or more `LineageHistoryEpisodeObservation` values.

Episode IDs cannot repeat within a generation.

V1 episode kinds are:

- population split;
- founder/recolonization;
- admixture/introgression;
- recontact;
- lineage fusion/remerger;
- extinction;
- other explicit episode.

Each episode binds its generation and the frozen demographic-episode protocol.

An episode may additionally carry a `DemographicInterventionProofBundleDigest` when an upstream typed demographic replay transcript is available.

### Proof-bundle trust boundary

SEL-10A V1 stores that proof-bundle digest as typed identity but does **not** independently replay the referenced `DemographicInterventionProofBundle` during lineage-history capture.

Accordingly:

- a proof-bundle digest is not, by itself, a freshly validated demographic capability;
- the episode evidence authority remains responsible for the claimed episode binding in this tranche;
- a later hardening tranche may consume a fresh replayed demographic capability directly when the cross-generation adapter is mature.

Likewise, `demographic_episode_census` is a completeness authority binding. It does not mean evolution-core independently discovered every historical episode from raw world data.

## 8. Recontact, gene flow, and lineage fusion remain counter-history

SEL-10A never deletes evidence because it weakens a clean divergence narrative.

Recontact, realized gene flow/introgression, and fusion remain first-class persisted history.

A later species model must see that counter-history rather than receiving only a favorable terminal label.

## 9. Built-in V1 history classification

The persisted status is recomputed from the complete ledger.

Precedence is:

1. any qualified lineage-fusion observation → `LineageFusionObserved`;
2. otherwise explicit qualified loss of either lineage's persistence → `NotPersistent`;
3. otherwise unavailable required generation/persistence/recontact/gene-flow/fusion evidence → `InsufficientEvidence`;
4. otherwise any qualified recontact or gene-flow observation → `DivergenceWithRecontact`;
5. otherwise complete qualified persistent lineage tracks → `PersistentDivergenceObserved`.

This precedence is part of `lineage_divergence_history_rule_v1()` content identity.

A caller cannot supply a different classifier while retaining the V1 method identity.

## 10. Persistence is evidence-bearing, not inferred from labels

`PersistentDivergenceObserved` does not mean two strings remained different.

The design binds distinct lineage authorities, while every observed generation carries qualified membership and persistence evidence bound to those lineage authorities and the exact current-revalidated population trajectory points.

If a lineage is explicitly `NotPersistent`, V1 reports `NotPersistent` even when other generations look clean.

If fusion is explicitly observed, fusion has stronger precedence and remains visible as `LineageFusionObserved`.

## 11. Representation identity versus current authority

`LineageDivergenceHistoryDesign`, `LineageDivergenceHistory`, and their canonical digests are serializable representation identity.

They are not current scientific authority after deserialization.

Current authority is non-Serde:

- `ValidatedLineageDivergenceHistoryDesign<'_>` reconstructs the design from fresh exact authorities and requires equality;
- `ValidatedLineageDivergenceHistory<'_>` locally validates the persisted history, then re-captures it from the current validated design and fresh current generation inputs and requires exact equality.

This replay revalidates population trajectory points against current schema/population state.

Serialized status tampering, lineage-subject transplant, protocol drift, context drift, and stale population state therefore cannot regain current authority merely because the bytes deserialize.

## 12. Canonicalization

Canonical identity binds:

- design identity;
- full inclusive generation ledger;
- exact population trajectory-point identities;
- exact lineage membership/persistence bindings;
- exact ordered pair evidence;
- context/structure/census evidence;
- all demographic episodes and optional proof-bundle identities;
- recontact/gene-flow/fusion evidence;
- recomputed typed status.

Generation input order is non-semantic.

Episode input order within one generation is non-semantic and canonicalized by semantic episode ID.

## 13. V1 corpus

The dedicated `lineage_divergence_history_v1` corpus covers at least:

- clean complete history → `PersistentDivergenceObserved`;
- recontact + gene flow retained → `DivergenceWithRecontact`;
- explicit lineage fusion → `LineageFusionObserved`;
- explicit loss of persistence → `NotPersistent`;
- unavailable generation under report policy → `InsufficientEvidence`;
- unavailable generation under fail-closed policy → rejection;
- omitted generation rejection;
- duplicate generation rejection;
- exact-context drift rejection;
- stale current population-state rejection through `PopulationTrajectoryPoint::validate_current`;
- Serde restoration + current replay;
- serialized status tampering rejection;
- serialized lineage-subject transplant rejection;
- no species/speciation fields in the wire shape.

## 14. Explicit non-claims

SEL-10A does **not** establish:

- a species concept;
- current species identity;
- species equivalence or non-equivalence under any taxonomic philosophy;
- reproductive isolation beyond whatever separate current SEL-09B evidence exists;
- an exact historical divergence time;
- an exact historical speciation time;
- a speciation mechanism;
- a historical speciation event;
- permanent zero gene flow;
- irreversibility of divergence;
- universal taxonomic truth.

A future SEL-10B must bind an explicit qualified species model before classification.

A future SEL-10C must combine current lineage-history evidence, current reproductive-isolation evidence, and the explicit species model to produce current model-bound species-status evidence.

A future SEL-10D must separately establish historical speciation-transition evidence and temporal uncertainty.

## 15. Qualification boundary

Source review and static reasoning are not executable qualification.

A frozen CI helper must bind the exact immutable SEL-10A product SHA and run pinned Rust/tooling, formatting, all-target checking, this dedicated corpus, prerequisite reproductive-isolation/contact/adaptation/selection corpora, complete tests, and strict Clippy.

Queued or unassigned CI is unexecuted evidence: neither PASS nor product-code FAIL.
