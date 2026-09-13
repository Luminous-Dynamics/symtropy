# SEL-08B2 — Replication-Qualified Adaptation Evidence v0.1

## Status

This document freezes the V1 source contract for **replication-qualified adaptation evidence** above SEL-08B1.

The governing separation is:

`single-study heritable response != replicated adaptation != reproductive isolation != speciation`.

SEL-08B1 establishes, at most, one complete preregistered multi-generation hereditary/trait-response study. SEL-08B2 asks whether a preregistered set of independent B1 study plans, with a frozen decision rule, produces enough current directional study results to support a scoped adaptation-evidence claim.

## Two-stage authority

B2 is deliberately split into:

1. `AdaptationReplicationDesign` — outcome-independent preregistration;
2. `AdaptationEvidence` — complete outcome-bearing replication ledger.

The replication design is declared from **current `ValidatedHeritableResponseStudyDesign<'_>` capabilities**, not from B1 outcome statuses.

Outcome-bearing `ValidatedHeritableResponseStudy<'_>` capabilities enter only when adaptation evidence is captured.

This prevents the replication set, threshold, or independence rule from being selected after favorable B1 outcomes are known.

## Minimum replication theorem

V1 requires at least two declared replication units.

`minimum_supported_replicates` must satisfy:

`2 <= minimum_supported_replicates <= declared_replication_count`.

A single B1 study can never preregister or support V1 adaptation evidence.

## Exact replication-unit preregistration

Every replication unit binds:

- semantic `ReplicationUnitId`;
- exact B1 `HeritableResponseStudyDesignDigest`;
- exact SEL-08A `ViabilitySelectionTranslationModelDigest` carried by that B1 design;
- exact population identity;
- exact SEL-08A selection-context digest;
- exact preregistered generation count;
- explicit independence-evidence authority.

Input order is non-semantic. Units are canonicalized by `ReplicationUnitId`.

Duplicate unit IDs fail closed.

Two units may **not** reuse the same B1 study-design digest. Different labels around the same B1 plan are not independent replication.

## Model-family compatibility

A major V1 design correction is that B2 does **not** require identical SEL-08A model digests across replicates.

An exact 08A model digest intentionally binds population, context, predictor mapping and qualification identity. Requiring exact equality would accidentally force independent replication back into one exact model lineage.

Instead B2 binds:

- the built-in `viability_selection_model_content_digest_v1()` model-family specification;
- every unit's exact 08A model digest;
- a separate `model_compatibility_authority`.

The compatibility authority is an external trust binding stating that the exact unit-specific models are sufficiently comparable for the preregistered replication claim.

It is responsible for compatibility questions such as:

- whether the focal hereditary classes have comparable biological meaning;
- whether generation/lifecycle mappings are commensurate;
- whether qualification assumptions are compatible;
- whether context differences preserve the intended replication interpretation.

`same mathematical model family != automatically compatible biological replicate`.

Evolution-core binds the authority identity; it does not infer compatibility from an opaque hash.

## Shared expected direction

All preregistered B1 designs must have the same SEL-08A-derived expected response direction.

A cohort mixing expected Comparison-frequency increase and expected Comparison-frequency decrease fails preregistration.

The expected direction remains inherited from SEL-08A through B1. B2 does not choose the favorable sign after viewing replicate outcomes.

## Context compatibility

V1 supports:

### `RequireSharedSelectionContext`

All units must bind the exact same SEL-08A selection-context digest.

### `AllowDeclaredVariation { authority }`

Different selection contexts are allowed only under an explicit cross-context compatibility authority whose identity is part of replication-design identity.

The latter does not prove environmental equivalence. It makes the trust edge auditable.

## Generation-span rule

The replication design freezes `minimum_generation_count` before B1 outcomes are aggregated.

The minimum must itself preserve a multi-generation study (`>= 2`).

Every preregistered B1 design must meet or exceed it.

A short favorable study cannot be admitted after results are visible when the frozen replication rule required a longer span.

## Independence boundary

Every unit carries explicit independence evidence, and the design also binds an `independence_rule_authority` governing the full preregistered cohort.

The intended semantics are cohort-relative: independence evidence must qualify the unit under the exact full set represented by the design.

Examples that are **not automatically independent** include:

- duplicated B1 studies under different labels;
- multiple offspring from one family;
- multiple samples from one underlying population trajectory;
- multiple deterministic seeds sharing hidden state or intervention lineage;
- nominally distinct populations linked by unaccounted migration or shared ancestry.

V1 binds these authority identities but does not independently prove statistical or biological independence from their hashes.

## Complete replication ledger

`AdaptationEvidence::capture(...)` requires exactly one current B1 study capability for every preregistered unit.

The following fail closed:

- omitted declared unit;
- duplicate supplied unit ID;
- unexpected unit;
- study whose B1 design digest differs from the preregistered unit;
- noncanonical persisted replication order;
- duplicate persisted B1 study result.

A current B1 study from unit A cannot be substituted for unit B.

Every `ReplicationResponseRecord` retains:

- unit ID;
- B1 study-design digest;
- exact B1 study-result digest;
- exact B1 typed study status.

Null, mismatch, reversed and insufficient studies are evidence. They are never silently dropped.

## Typed adaptation status

`AdaptationEvidenceStatus` is one of:

- `Supported`;
- `NotSupported`;
- `Contradicted`;
- `InsufficientEvidence`.

V1 uses a deterministic preregistered decision rule.

### `Contradicted`

If **any** declared replicate has B1 status `ReversedResponse`, the adaptation evidence is `Contradicted`.

This takes precedence over missing/insufficient evidence because an observed complete reversal is explicit contrary evidence.

### `InsufficientEvidence`

If no reversed replicate exists but **any** declared replicate has B1 status `InsufficientEvidence`, the B2 result is `InsufficientEvidence`.

Unavailable transmission or other incomplete B1 evidence therefore cannot be converted into a negative or silently excluded replicate.

### `Supported`

If there is no reversed or insufficient replicate, `Supported` requires:

`count(DirectionalResponseObserved) >= minimum_supported_replicates`.

Null, hereditary-only mismatch and trait-only mismatch studies remain present and count against the preregistered threshold.

V1 therefore permits a preregistered tolerance rule such as two directional studies out of three complete studies, while preserving the third non-directional history in evidence identity.

### `NotSupported`

If the evidence is complete and non-contradictory but directional replication does not reach the frozen threshold, the result is `NotSupported`.

## Negative-history identity theorem

Adaptation identity includes the full ordered replication ledger, not only the successful-count total or final status.

Therefore two cohorts can have:

- the same number of directional replicates;
- the same final `Supported` status;

and still have different evidence identity because one retained a null replicate while another retained a hereditary/trait mismatch.

This prevents a lossy `2/3 supported` summary from replacing the actual evidence history.

## Representation identity versus current authority

`AdaptationReplicationDesign` and `AdaptationEvidence` are serializable evidence representations.

Their canonical digests establish representation identity only.

Current authority is non-Serde:

- `ValidatedAdaptationReplicationDesign<'_>` requires fresh current B1 study-design capabilities and fresh compatibility/independence/decision authorities;
- `ValidatedAdaptationEvidence<'_>` requires the current validated replication design plus all current B1 study capabilities.

Serialized adaptation evidence does not self-authorize after restoration.

The persisted adaptation status is recomputed locally from the complete replication ledger. Altering only the serialized status makes canonicalization fail.

## External trust boundary

Several B2 inputs remain explicit external trust bindings:

- model compatibility;
- cross-context compatibility where used;
- cohort independence rule;
- per-unit independence evidence;
- decision-rule authority.

Binding those authorities makes the trust graph explicit and replayable. It does not mean evolution-core independently established their scientific truth.

Future stronger typed current capabilities may replace selected opaque authorities. Such strengthening must not be retroactively attributed to V1.

## Adversarial corpus

The V1 corpus covers at minimum:

- two current directional B1 studies -> `Supported`;
- canonical replication-unit ordering;
- distinct exact 08A model digests under one explicit compatibility authority;
- Serde restore plus fresh design/evidence replay;
- one declared unit rejected;
- same B1 study-design digest under two unit labels rejected;
- one reversed replicate -> `Contradicted`;
- one insufficient replicate -> `InsufficientEvidence`;
- complete non-directional replication -> `NotSupported`;
- preregistered `2 of 3` threshold with visible null history -> `Supported`;
- same success count/final status but different negative history -> different evidence digest;
- stale model-compatibility authority rejected on current replay;
- omitted declared replicate rejected;
- current B1 study substitution across units rejected;
- serialized adaptation-status tampering rejected locally;
- independence-evidence-only drift changes replication-design identity;
- wire shape contains no reproductive-isolation/speciation claim.

## Explicit scope of `Supported`

`Supported` means only:

> the exact declared hereditary-response hypothesis has replication-qualified adaptation evidence for the exact represented B1 designs, populations/lineages, contexts, generation spans, compatibility authorities, independence authorities and decision threshold.

It does **not** establish:

- universal beneficial or deleterious mutation status;
- generic lifetime fitness superiority;
- ecological dominance;
- permanence under arbitrary environments;
- reproductive isolation;
- species identity;
- speciation.

## Successor boundary

Reproductive isolation must remain a separate SEL-09 authority.

Even strong replicated adaptation evidence cannot mint reproductive isolation or species status.

A future SEL-09 tranche should separately model gene flow, mate compatibility/preference, hybrid viability/fertility, spatial/contact structure and sustained isolation evidence before any speciation claim is representable.

## Qualification status

Source review, static reasoning and GitHub mergeability are not executable qualification.

A frozen CI-only helper must execute the exact B2 product head with pinned Rust and the complete named B2 -> B1 -> 08A -> causal/evidence prerequisite chain.

Queued or unassigned jobs are neither PASS nor product-code FAIL.
