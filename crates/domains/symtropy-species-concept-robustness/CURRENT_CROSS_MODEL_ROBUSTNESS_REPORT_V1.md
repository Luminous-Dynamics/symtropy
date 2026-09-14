# Current Cross-Model Species Robustness Report V1

SEL-10E2B executes current-species robustness only after SEL-10E2A has frozen the comparison design and regained current authority.

The governing theorem is:

> agreement across preregistered models is not majority truth; disagreement across models is not evidence failure; pairwise independence is not transitive independence.

## 1. Authority boundary

V1 execution requires a non-Serde `CurrentCrossModelRobustnessAuthority` from SEL-10E2A.

A persisted E2A design or report digest is representation identity only. It does not regain current scientific authority by deserialization.

Every contributing model row must be materialized from a live family-specific current capability:

- strict biological species concept: `ValidatedCurrentSpeciesStatus`;
- general-lineage species concept: `ValidatedGeneralLineageSpeciesEvidence`;
- or an explicit qualified reason that the declared current capability is unavailable.

The final persisted report can regain current authority only by exact replay from the current E2A authority and a fresh complete set of current model rows.

## 2. Complete declared-model matrix

The E2A model set is immutable for one E2B report lineage.

E2B retains exactly one canonical row per preregistered conceptual family. It rejects:

- undeclared model insertion;
- duplicate conceptual-family rows;
- omission of any declared row;
- deletion of an inconvenient contradictory row;
- source-family substitution;
- classification-design substitution;
- evidence-surface substitution.

Input order is irrelevant. Persisted row order is canonical conceptual-identity order.

An E2A `FailClosed` missing-model policy rejects an explicit missing-current-capability row. `ReportInsufficientCoverage` retains the row and prevents it from being treated as a negative vote.

## 3. Preserve family-specific semantics

E2B does not replace family results with a generic boolean.

Each row persists its exact source-family result and digest.

The normalized cross-model disposition is only a comparison projection:

- `Supports`;
- `DoesNotSupport`;
- `Contradicts`;
- `InsufficientEvidence`;
- `OutsideValidityDomain`;
- `MissingCurrentCapability`.

The following distinctions are mandatory:

> non-support != contradiction

> outside validity domain != negative evidence

> missing current capability != disagreement

> insufficient evidence != disagreement

## 4. Pair context remains visible

For every preregistered model pair, E2B persists the exact E2A pair context:

- semantic-relation evidence digest;
- semantic dependency class;
- both exact fault-profile digests;
- exact independence-policy digest;
- pairwise fault-domain disposition;
- pair qualification authority.

Nested, criterion-dependent, overlapping, unknown, or unqualified pairs remain visible in the report. They cannot be relabeled as independent by the aggregate layer.

## 5. Independent coverage is a clique, not a count

Pairwise independence is not transitive.

Therefore V1 never interprets these facts:

- A is eligible with B;
- B is eligible with C;

as proof that A, B, and C jointly form independent coverage.

A qualifying independent-coverage witness is a canonical set of conceptual identities of size `minimum_independent_model_coverage` for which **every pair** passes the frozen E2A `pair_is_eligible_for_independent_coverage` theorem.

The report persists separate canonical witnesses for:

- support;
- non-support;
- contradiction.

A witness is not a vote count. It is an auditable proof that the corresponding same-disposition rows contain a pairwise-qualified independent subset meeting the preregistered threshold.

## 6. Aggregate precedence

V1 derives exactly one typed aggregate state in this order:

1. If every declared row is outside its model validity domain: `AllModelsOutsideValidityDomain`.
2. If at least one resolved applicable model supports and at least one resolved applicable model contradicts: `MixedSupportAndContradiction`.
3. If more than one resolved conclusion kind occurs among support, non-support, and contradiction: `ModelDependentConclusion`.
4. If support is the only resolved conclusion kind and the canonical support clique reaches the frozen threshold: `RobustSupportAcrossQualifiedIndependentCoverage`.
5. If contradiction is the only resolved conclusion kind and the canonical contradiction clique reaches the frozen threshold: `RobustContradictionAcrossQualifiedIndependentCoverage`.
6. If non-support is the only resolved conclusion kind and the canonical non-support clique reaches the frozen threshold: `ConcordantNonSupportAcrossQualifiedIndependentCoverage`.
7. Otherwise: `InsufficientIndependentModelCoverage`.

A favorable independent subset can never hide a resolved contrary model conclusion.

The dedicated non-support state is intentionally not named robust contradiction:

> independent absence of support != independent evidence of contradiction.

## 7. No voting theorem

V1 exposes no:

- majority count;
- vote count;
- scalar taxonomy score;
- weighted family score;
- winner model;
- tie breaker;
- threshold based on fraction of models agreeing.

Adding any such quantity requires a new scientific theorem and a new report lineage. It cannot be smuggled into V1 as an implementation detail.

## 8. Bounded exact execution

V1 supports at most 32 preregistered models.

The exact clique search is capped at 1,000,000 candidate steps. If exact witness search would exceed that bound, execution fails closed.

There is no approximate clique fallback and no heuristic promotion to robust status.

The bound is an execution-safety limit, not a scientific claim that 32 models are sufficient or optimal.

## 9. Replay and tamper resistance

Local validation recomputes:

- the E2A design digest;
- exact row/model bindings;
- complete model coverage;
- pairwise context;
- all three independence witnesses;
- aggregate status;
- the built-in V1 report-rule authority.

Current validation then re-executes the entire report from current E2A authority plus fresh model-bound current capabilities and requires exact equality.

Tampering with a stored status, witness, pair context, row binding, or design snapshot therefore cannot self-authorize.

## 10. V1 non-claims

SEL-10E2B does not establish:

- majority truth;
- a universally correct species concept;
- philosophical superiority of one model family;
- historical-transition robustness;
- a speciation event or speciation time;
- nomenclatural authority;
- universal taxonomy truth.

A robust current-species conclusion remains explicitly conditional on the preregistered model families, their validity domains, the shared subject, the qualified semantic/fault-domain independence theorem, and the evidence available under those models.
