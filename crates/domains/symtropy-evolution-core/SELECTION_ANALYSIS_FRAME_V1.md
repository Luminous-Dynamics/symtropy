# Selection Analysis Frame Contract v1

## Purpose

SEL-07B1 inserts an explicit executable-data boundary between a frozen SEL-07A comparison design and any statistical estimator.

The governing distinction is:

```text
preregistered design
    != executable analysis frame
    != descriptive association estimate
    != causal selection effect
    != model-specific selection coefficient
    != adaptation
```

SEL-07B1 produces no estimate.

Its job is to prove that one exact set of per-individual predictor materializations and one exact set of consequence-derived outcomes are bound to the same currently validated design, selection-evidence ledger, consequence ledger, population, census, context, and persistent individual identities.

## Current-authority chain

Frame capture consumes all three current non-Serde capabilities:

- `ValidatedSelectionComparisonDesign<'_>`;
- `ValidatedSelectionEvidenceLedger<'_>`;
- `ValidatedConsequenceLedger<'_>`.

The implementation rejects any disagreement in:

- exact selection-evidence ledger digest;
- exact consequence-ledger digest;
- population identity;
- census digest;
- context/window digest.

The SEL-07A design is replayed against the supplied current SEL-06C evidence before materialization continues.

This prevents a current design capability from being combined with a convenient stale evidence or consequence subject.

## Full design snapshot

`ExplicitSelectionAnalysisFrame` stores the complete frozen `SelectionComparisonDesign`, not only its digest.

That intentionally carries forward:

- predictor definition;
- typed estimand;
- design class;
- original denominator and exclusions;
- estimand population;
- support/missingness policy;
- protocol-comparability policy;
- confounding declaration;
- death/competing-risk policy;
- uncertainty plan.

A later descriptive estimator therefore does not need to reopen raw evidence merely to reconstruct what analysis was intended.

The frame digest includes the canonical SEL-07A design digest.

## Exact denominator preservation

SEL-07B1 materializes exactly one row for every SEL-07A included individual.

Rows are canonicalized in persistent-individual order inherited from the frozen design.

The frame rejects:

- an omitted included individual;
- duplicate predictor materialization;
- a predictor input for an excluded or unknown individual;
- a row set that differs from the frozen included population;
- any denominator mismatch.

The frame preserves both:

- the original complete evidence denominator; and
- the estimand denominator.

The original exclusions remain present through the stored design snapshot.

An estimator may not later present the estimand population as though it were the original population.

## Predictor representation

V1 deliberately supports a small, auditable predictor representation vocabulary:

- binary reference/comparison group;
- categorical level represented by an exact analysis-authority reference;
- signed fixed-point value with an explicit nonzero scale and encoding authority.

One frame declares one representation.

Observed per-individual values must match that representation exactly.

Missing predictor values remain a distinct typed state carrying an explicit reason authority.

No missing predictor is silently converted into numeric zero, a reference group, or an empty category.

## Predictor-definition binding

Every row carries a deterministic digest of the frozen `PredictorDefinitionRef`.

The digest binds:

- predictor semantic identity;
- source kind;
- revision;
- content digest.

Changing the predictor definition changes every valid row binding and the frame identity.

## Exact source provenance

Phenotype predictor rows bind the exact SEL-06C phenotype evidence source:

- source ID;
- revision;
- evidence content digest;
- evidence protocol ID;
- evidence protocol content digest;
- `CompleteWindow` versus `PartialWindow`;
- support digest for partial evidence.

Exposure predictors receive the analogous binding.

If the SEL-06C source is unavailable, the frame records an explicit unavailable source state. An observed predictor value cannot be materialized from that state.

Genotype/lineage predictors bind the exact current linked-individual manifest digest already replayed by SEL-06C. This makes the value provenance depend on exact hereditary/ancestry/lineage authority rather than a human-readable allele name.

Externally defined predictors require an explicit opaque analysis-authority binding. Evolution-core does not claim that external authority is scientifically correct.

## Frozen support-policy enforcement

Frame capture replays the SEL-07A design against current SEL-06C evidence before accepting predictor values.

Therefore the frozen support policies remain active:

- `CompleteOnly` cannot be satisfied by partial/unavailable evidence;
- `PartialAllowed` requires the exact declared method authority from SEL-07A;
- `UnavailableAllowed` preserves unavailable status;
- `NotRequired` cannot later be used as the phenotype/exposure channel that supplies the predictor.

That last rule prevents a design from declaring a channel irrelevant and then quietly using it to populate the predictor during execution.

## Protocol comparability and calibration

Under SEL-07A exact protocol comparability, the frame binds the exact protocol identity already validated by the design.

Under calibrated comparability, every observed phenotype/exposure predictor must additionally carry `PredictorCalibrationProvenance` matching the exact SEL-07A calibration authority.

Calibration provenance binds:

- exact calibration authority ID/revision/content digest;
- explicit transformed-value provenance digest;
- the exact pre-calibration source binding;
- the exact resulting materialized predictor value through the row digest.

Supplying calibration provenance to an exact-comparability design is rejected.

A missing predictor value does not pretend to have undergone a successful calibration.

SEL-07B1 still does not independently prove that the calibration method is scientifically valid.

## Outcome authority

Callers do **not** supply outcomes to SEL-07B1.

Outcomes are derived directly from the current validated consequence ledger according to the exact frozen `SelectionEstimand`.

The mapping is mechanical:

- viability-window risk -> viability consequence;
- reproductive-event opportunity-conditioned contrast -> reproductive-event consequence;
- descendant-production contrast -> descendant-production consequence;
- descendant-recruitment contrast -> descendant-recruitment consequence.

Every frame row binds the exact current `IndividualConsequenceObservationDigest` already referenced by the SEL-06C evidence record.

If that cross-layer observation binding differs, capture fails.

This means a caller cannot materialize one predictor set and pair it with invented per-row outcomes.

## Missing outcome is not observed zero

For an absent raw consequence channel, SEL-07B1 preserves explicit absence.

For later reproductive/descendant endpoints, if the same consequence observation records death during the window, the exact SEL-07A death policy is materialized as one of:

- `EndpointFailureByObservedDeath`;
- `CensoredByObservedDeath`;
- `CompetingRiskDeath`.

These states are distinct from:

- `Unavailable`; and
- an observed numeric zero.

Therefore:

```text
observed zero descendants
    != unavailable descendant observation
    != censored-by-death descendant observation
    != endpoint-failure-by-death transformation
    != competing-risk death
```

SEL-07B1 never silently rewrites an absent raw outcome to zero.

## Materialization authority

Each frame binds an explicit `AnalysisAuthorityRef` for the predictor-materialization procedure.

This is an opaque method/revision/content binding. Evolution-core does not claim that the referenced materializer is scientifically correct merely because it is named.

The representation configuration and materialization authority are part of frame identity.

Changing either changes frame identity.

## Independent replay requirement

A serialized frame is representation, not current authority.

Critically, the frame may **not re-authorize its own persisted predictor values**.

`ExplicitSelectionAnalysisFrame::validate_current(...)` and `ValidatedSelectionAnalysisFrame::validate_current(...)` require the caller to supply again:

- current validated design capability;
- current validated selection-evidence capability;
- current validated consequence capability;
- current materialization-authority reference;
- current predictor-representation declaration;
- freshly materialized per-individual predictor inputs.

Replay reconstructs a new frame from those independent current inputs and requires exact equality with the persisted frame.

Thus this is forbidden:

```text
stored predictor value
    -> read stored predictor value
    -> call that current replay
```

A stored value changed after serialization can still have a deterministic representation digest, but it cannot regain `ValidatedSelectionAnalysisFrame<'_>` authority unless the independently supplied current materialization reproduces that value exactly.

This closes the same representation-versus-authority seam addressed by SEL-06B, SEL-06C, and SEL-07A.

## `ValidatedSelectionAnalysisFrame<'a>`

The validated capability is deliberately non-Serde.

It binds:

- exact frame digest;
- exact design digest;
- exact selection-evidence ledger digest;
- exact consequence-ledger digest;
- exact population/census/context.

For SEL-07B2 it also exposes the already frozen:

- full design snapshot;
- estimand;
- design class;
- uncertainty plan.

A future estimator therefore has no reason to reopen raw consequence/evidence ledgers and choose a different subset or method.

## Representation identity

`ExplicitSelectionAnalysisFrameDigest` is deterministic representation identity only.

A locally valid deserialized frame may compute a digest even if its authorities are stale.

Only fresh replay can mint current validated frame authority.

## Wire-shape boundary

SEL-07B1 contains no:

- point estimate;
- p-value;
- significance result;
- fitness value;
- selection coefficient;
- global beneficial/deleterious classification;
- causal-selection certification;
- adaptation classification;
- reproductive-isolation claim;
- speciation claim.

The frame is executable evidence preparation, not statistical inference.

## Explicit non-claims

SEL-07B1 does not establish:

- predictor scientific correctness;
- external evidence scientific correctness;
- calibration validity;
- statistical association;
- causal selection;
- absence of confounding;
- genotype -> phenotype causation;
- exposure -> consequence causation;
- scalar or relative fitness;
- a population-genetic selection coefficient;
- adaptation;
- reproductive isolation;
- speciation.

It establishes only that a frozen analysis design has been materialized into a complete, explicit, replayable per-individual frame without silently changing denominator, source identity, outcome semantics, missingness, or calibration provenance.

## Successor boundary

SEL-07B2 may consume `ValidatedSelectionAnalysisFrame<'_>` to execute **descriptive association only** under the exact frozen SEL-07A uncertainty/method authority.

SEL-07B2 must not reopen raw evidence to:

- change the estimand population;
- choose a different estimator after outcome inspection;
- redefine missing as zero;
- replace calibration provenance;
- switch predictor encoding;
- promote association to causation.

A later SEL-07C must use a distinct causal-selection capability with independently satisfied identification requirements.

Any model-specific selection coefficient remains a still-later conversion requiring explicit population-genetic model authority.

## Evidence status

Source implementation, static review, tests, and this contract are not executable qualification.

A separate frozen exact-head helper is required before any executable PASS may be claimed.

Queued/unassigned CI is neither PASS nor product-code FAIL.
