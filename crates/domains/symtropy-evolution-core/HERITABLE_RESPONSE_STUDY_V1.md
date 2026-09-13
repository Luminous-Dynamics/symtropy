# SEL-08B1 — Complete Multi-Generation Heritable-Response Study v0.1

## Status

This document freezes the V1 source contract for a **single complete multi-generation heritable-response study**.

It does not establish adaptation.

The governing separation is:

`model-specific selection quantity != realized heritable response != replicated adaptation`

SEL-08A states one explicitly qualified model-specific viability-selection quantity. SEL-08B1 asks a different question: across a preregistered generation interval, did the declared hereditary class and declared trait/consequence response move consistently with that directional hypothesis under the frozen evidence rules?

SEL-08B2 is responsible for replication-qualified adaptation evidence.

## Current-authority prerequisite

A B1 design may be declared only from a current `ValidatedModelSpecificSelectionEstimate<'_>`.

The design binds the exact:

- SEL-08A estimate digest;
- SEL-08A translation-model digest;
- population identity;
- SEL-08A evolutionary-context digest.

A raw digest, C2 effect, D2 probability, frequency delta, phenotype trend, or serialized 08A estimate cannot substitute for the current capability.

## Preregistration before generation outcomes

`HeritableResponseStudyDesign` is separate from the realized generation ledger.

The design freezes:

- `HeritableResponseStudyId`;
- start and end `PopulationGeneration`;
- expected directional response;
- context policy;
- generation-axis authority;
- hereditary-frequency evidence authority;
- trait-response evidence authority;
- hereditary-transmission authority;
- demography-accounting authority;
- response-rule authority.

The interval must contain at least two generation coordinates: `end > start`.

The expected direction is **not caller-selected after viewing the trajectory**. It is derived from the current SEL-08A model-specific quantity:

- finite negative quantity -> expected Comparison-class frequency decrease;
- finite positive quantity -> expected Comparison-class frequency increase;
- positive infinity -> expected Comparison-class frequency increase;
- exact zero -> V1 refuses a directional study;
- undefined both-zero-survival quantity -> V1 refuses a directional study.

This expected direction is a preregistered study hypothesis under the qualified model. It is not a promise that evolution must follow that direction.

## Current design authority

`HeritableResponseStudyDesign` is serializable evidence representation.

`ValidatedHeritableResponseStudyDesign<'a>` is non-Serde and is created only when the persisted design exactly replays against:

- the current SEL-08A selection capability;
- the current context policy;
- fresh generation-axis authority;
- fresh hereditary-frequency authority;
- fresh trait-response authority;
- fresh transmission authority;
- fresh demography-accounting authority;
- fresh response-rule authority.

Changing any of those authorities changes or invalidates current design authority.

## Complete generation ledger

`HeritableResponseStudy::capture(...)` requires exactly one `GenerationResponseEvidenceInput` for every generation in the preregistered inclusive interval.

For `[g_start, g_end]`, the required record count is:

`g_end - g_start + 1`.

Inputs may arrive in any order. Capture canonicalizes them by `PopulationGeneration`.

The following fail closed:

- duplicate generation;
- omitted generation;
- extra/out-of-range generation;
- non-contiguous persisted order;
- a generation record bound to another population.

No intermediate generation may silently disappear because an endpoint result looks favorable.

## Per-generation evidence

Every `GenerationResponseRecord` binds:

- exact generation coordinate;
- exact population identity;
- `PopulationTrajectoryPointDigest`;
- `ExplicitLinkedPopulationCensusDigest`;
- exact evolutionary-context digest;
- focal hereditary-class count;
- non-zero census size;
- declared trait-response direction;
- hereditary-frequency evidence authority;
- trait-response evidence authority;
- transmission evidence status;
- demography-accounting evidence authority;
- explicit inclusion/exclusion/unavailable disposition.

The focal-class count must satisfy:

`0 <= focal_class_count <= census_size`.

The hereditary-frequency, trait-response, and demography evidence authorities must exactly equal the corresponding preregistered design authorities. A study cannot change its measurement or accounting method generation-by-generation after outcomes become visible.

## Transmission semantics

The study-start generation must use:

`NotApplicableAtStudyStart`.

Every later generation must use either:

- `Present { authority }`, with authority exactly equal to the preregistered transmission authority; or
- `Unavailable { reason }`.

Unavailable transmission evidence remains in the ledger and forces `InsufficientEvidence`.

Phenotypic persistence is therefore not silently treated as hereditary continuity.

## Context policy

V1 supports:

### `ExactSelectionContext`

Every generation must bind the exact SEL-08A selection-context digest. Any change fails with `UndeclaredContextDrift`.

### `DeclaredContextTrajectory { authority }`

Generation context may vary, but the trajectory authority is frozen into the study-design identity. Context changes remain represented by each record's exact context digest.

The V1 core treats the declared context-trajectory authority as an external trust binding; it does not independently infer environmental equivalence.

## Full-trajectory hereditary classification

A major V1 hardening is that hereditary direction is **not** inferred from the first and last frequencies alone.

Every adjacent generation transition is classified using exact rational frequency comparison:

`focal_count_next / census_next` versus `focal_count_previous / census_previous`.

Each transition is one of:

- expected direction;
- neutral plateau;
- opposite direction.

The complete hereditary trajectory is:

- `Expected` if at least one expected-direction transition exists, no opposite transition exists, and any remaining transitions are neutral plateaus;
- `Opposite` if at least one opposite transition exists, no expected transition exists, and any remaining transitions are neutral plateaus;
- `Neutral` if every transition is neutral;
- `Mixed` if both expected and opposite transitions occur.

A favorable endpoint cannot hide an internal reversal.

For example, under expected decrease:

`3/4 -> 1/4 -> 2/4`

has a favorable final endpoint relative to the start, but contains one expected decrease and one opposite increase. It is `Mixed`, therefore not directional-response evidence.

Plateaus do not erase an otherwise consistent directional trajectory.

## Trait-response classification

The study-start generation establishes the baseline record. Trait concordance is evaluated across all later generation records.

For the preregistered direction, later traits are classified as:

- all expected -> expected;
- all neutral -> neutral;
- all opposite -> opposite;
- any mixture -> mixed.

Mixed trait evidence is conservatively `InsufficientEvidence`.

## Typed single-study result

`HeritableResponseStudyStatus` is one of:

- `DirectionalResponseObserved`;
- `NullResponse`;
- `ReversedResponse`;
- `HereditaryOnlyMismatch`;
- `TraitOnlyMismatch`;
- `InsufficientEvidence`.

`DirectionalResponseObserved` requires both a complete expected hereditary trajectory and complete expected trait direction under the frozen study design.

`NullResponse` requires both hereditary and trait response to be neutral.

`ReversedResponse` requires both to be consistently opposite.

Hereditary-only and trait-only mismatches remain first-class evidence rather than being promoted or dropped.

Any excluded/unavailable generation, unavailable transmission evidence, or mixed trajectory yields `InsufficientEvidence` in V1.

The persisted status is recomputed from the full persisted ledger during local validation. Altering the serialized status without altering the evidence cannot obtain a valid canonical digest.

## Representation identity versus current authority

`HeritableResponseStudy` and its generation records are serializable deterministic evidence representations.

Their canonical digests establish representation identity only.

`ValidatedHeritableResponseStudy<'a>` is non-Serde. Current authority requires exact reconstruction from:

- current validated B1 design;
- current validated SEL-08A selection estimate;
- fresh complete generation evidence inputs.

A serialized study does not self-authorize after restoration.

## External evidence trust boundary

V1 deliberately binds existing trajectory, census, context, transmission, and demography evidence identities instead of building a second population-genetics engine inside B1.

The following distinction is mandatory:

`evidence/digest is bound != evolution-core independently re-derived or scientifically qualified the underlying observation`.

In particular, V1 does not independently reopen every `PopulationTrajectoryPointDigest`, census digest, hereditary-frequency count, trait measurement, or demography record during local validation. Their exact identities and preregistered authorities are carried into the evidence graph and must be supplied again during current replay.

A later strengthening may replace selected opaque materializations with direct typed current capabilities. That would strengthen authority; it must not be backported as though V1 had already proved it.

## Adversarial corpus

The B1 corpus covers at minimum:

- exact complete three-generation directional study;
- non-semantic input reordering with canonical output order;
- Serde restoration plus fresh current replay;
- omitted generation rejection;
- duplicate generation rejection;
- undeclared exact-context drift rejection;
- hereditary change without compatible trait response;
- trait response without hereditary change;
- fully reversed response;
- unavailable transmission retained as `InsufficientEvidence`;
- stale preregistered response-rule authority rejected;
- favorable endpoint with internal hereditary reversal classified as mixed/insufficient;
- plateau tolerance for otherwise consistent directional response;
- wire shape with no adaptation or speciation claim.

## Explicit non-claims

SEL-08B1 does **not** establish:

- replicated adaptation;
- universal beneficial/deleterious status;
- generic lifetime fitness;
- ecological superiority;
- reproductive isolation;
- species identity;
- speciation.

Even `DirectionalResponseObserved` means only that one exact preregistered study contains a complete, directionally concordant hereditary and trait response under its represented authorities and assumptions.

## Successor

SEL-08B2 must consume multiple current `ValidatedHeritableResponseStudy<'_>` capabilities under a preregistered replication and independence rule before it may emit typed adaptation evidence.

Multiple rows, multiple offspring, or multiple seeds are not automatically independent replications.

## Qualification status

Source review, static reasoning, and mergeability are not executable qualification.

A frozen CI-only helper must execute the exact B1 product head with pinned Rust and named prerequisite corpora. Queued/unassigned jobs are neither PASS nor product-code FAIL.
