# Selection Comparison Design Contract v1

## Purpose

SEL-07A freezes a pre-analysis comparison design over already validated SEL-06C selection evidence.

It exists to prevent observed consequences, phenotype/exposure observations, or favorable frequency changes from being promoted directly into causal-selection or adaptation claims.

The governing distinction is:

```text
observed consequence association
    != causal selection effect
    != model-specific selection coefficient
    != adaptation
```

A design declaration is not a result.

## Current-authority prerequisite

`SelectionComparisonDesign::declare(...)` consumes `ValidatedSelectionEvidenceLedger<'_>` rather than a raw selection-evidence digest.

Therefore declaration begins from a SEL-06C ledger that has already regained current authority through exact replay of:

- persistent individual identities;
- exact linked-individual manifests;
- exact population and census;
- exact context/window;
- exact consequence observations;
- explicit phenotype/exposure support states;
- exact external evidence and protocol identities.

A persisted design can later have deterministic representation identity, but downstream analysis should consume `ValidatedSelectionComparisonDesign<'_>`, reconstructed only after replay against a current validated SEL-06C capability.

## Typed estimands

V1 keeps consequence endpoints decomposed:

- viability-window risk contrast;
- reproductive-event opportunity-conditioned contrast;
- descendant-production contrast;
- descendant-recruitment contrast.

There is no generic lifetime-fitness scalar and no context-free `s` field.

Changing the estimand changes design identity.

## Design class

V1 records one declared design class:

- descriptive association;
- randomized/interventional;
- controlled simulation intervention;
- matched/stratified observational;
- within-family or lineage-controlled;
- common-environment;
- reciprocal-context;
- declared neutral/null comparison.

Any non-descriptive design requires an explicit comparison authority reference.

This reference is an opaque evidence binding, not a claim that the comparison is scientifically sufficient for causal identification. In particular, a queued or failed neutral-baseline qualification run cannot be converted into a qualified neutral baseline merely by referencing its intended subject.

A design class is also not a causal-result capability. SEL-07A exposes no causal-effect estimator.

## Predictor declaration

`PredictorDefinitionRef` binds:

- a validated semantic predictor identity;
- source-kind declaration;
- revision;
- opaque content digest.

Source kinds include phenotype, exposure, genotype/lineage, and externally defined predictors.

The predictor definition is a preregistered analysis subject. V1 does **not** materialize predictor values or prove that an external predictor definition is scientifically valid. SEL-07B must bind the exact executable predictor inputs before estimating association.

## Exact denominator accounting

SEL-07A preserves the complete SEL-06C denominator even when the estimand population is smaller.

Every individual in the validated selection-evidence ledger must be exactly one of:

- included in the estimand population; or
- explicitly excluded with a validated exclusion-reason identity.

The design rejects:

- an empty estimand population;
- unknown individuals;
- duplicate inclusions;
- duplicate exclusions;
- include/exclude overlap;
- any unaccounted original individual.

Included and excluded identities are canonicalized independently by persistent individual ID. Input ordering is non-semantic.

The design exposes both:

- `original_denominator()`; and
- `estimand_denominator()`.

A convenient complete-case subset therefore cannot silently replace the original denominator.

## Evidence-support policy

Phenotype and exposure channels each declare one support policy:

- `NotRequired`;
- `CompleteOnly`;
- `PartialAllowed { method }`;
- `UnavailableAllowed { method }`.

For every included organism, required support is checked against the exact SEL-06C status.

Thus:

```text
Unavailable != zero
PartialWindow != CompleteWindow
```

Allowing partial or unavailable evidence requires an explicit analysis/missingness authority reference that becomes part of design identity.

The authority reference binds a method identity/revision/content digest. SEL-07A does not independently prove that the method is statistically appropriate; that claim belongs to qualification/execution layers.

## Protocol comparability

For each required phenotype/exposure channel, V1 declares one comparability policy:

- exact protocol identity/content-digest equality; or
- explicit calibration authority.

Under exact comparability, included observed evidence must share the exact `EvidenceProtocolId` and protocol-content digest.

Human-readable trait names do not establish comparability.

Under calibrated comparability, protocol heterogeneity may be declared, but the calibration authority is only an opaque binding in SEL-07A. SEL-07B/SEL-07C must qualify the actual translation/calibration behavior before relying on it.

## Inclusion rule

The exact realized include/exclude partition is additionally bound to an opaque `eligibility_rule` authority reference.

This preserves the declared rule identity separately from its realized population partition.

SEL-07A does not claim that the rule was chosen without outcome peeking; workflow/evidence provenance is needed for stronger preregistration claims.

## Confounding declaration

V1 separates:

- descriptive/no-causal-claim;
- design-based no-adjustment declaration;
- explicit adjustment authority.

A `DescriptiveAssociation` design must use `DescriptiveNoCausalClaim`.

Other design classes may record stronger design/adjustment intent, but SEL-07A does not itself certify causal identification.

Only a later SEL-07C layer may construct a causal-selection effect type, and only after satisfying the design's required identification evidence.

## Death before later endpoints and competing risk

For viability-window risk itself, death-before-endpoint is `NotApplicable`.

Every later reproductive/descendant endpoint must explicitly declare how death before that endpoint is treated:

- endpoint failure;
- censoring; or
- competing risk.

Declaring competing-risk handling requires an explicit competing-risk method authority. Supplying a competing-risk authority when competing-risk semantics are not selected is rejected.

This prevents death before reproduction from being silently dropped or automatically converted into reproductive failure.

## Uncertainty declaration

`UncertaintyPlan` binds:

- exact method authority;
- optional replicate count;
- optional seed-lineage digest;
- optional multiplicity authority;
- minimum-information authority;
- insufficient-support behavior.

A declared replicate count of zero is invalid.

Changing uncertainty configuration changes design identity even when all observed data are unchanged.

## Representation identity vs current authority

`SelectionComparisonDesignDigest` is deterministic representation identity.

It is not sufficient proof that the bound SEL-06C evidence is still current.

`ValidatedSelectionComparisonDesign<'_>` is deliberately non-Serde and can only be created by replaying the design against a current `ValidatedSelectionEvidenceLedger<'_>`.

This continues the authority discipline introduced in SEL-06B and SEL-06C:

```text
serialized representation
    != current evidence authority
```

## Wire-shape boundary

SEL-07A contains no:

- fitness estimate;
- selection coefficient estimate;
- beneficial/deleterious classification;
- adaptation classification;
- causal-effect estimate;
- p-value or significance result.

It is a design authority only.

## Explicit non-claims

SEL-07A does not establish:

- a statistically significant association;
- predictor measurement correctness;
- genotype -> phenotype causation;
- exposure -> consequence causation;
- absence of unmeasured confounding;
- causal selection;
- scalar or relative fitness;
- a model-specific selection coefficient;
- beneficial/deleterious mutation status;
- adaptation;
- reproductive isolation;
- speciation.

## Successor boundary

SEL-07B may execute **descriptive association** only after consuming `ValidatedSelectionComparisonDesign<'_>` and binding exact executable predictor/outcome materialization.

SEL-07C may later introduce a distinct causal-selection effect capability only for design classes whose identification requirements have been independently satisfied and qualified.

Any model-specific selection coefficient must remain a still-later conversion bound to an explicit population-genetic model and estimand.

## Evidence status

Source implementation, static review, and this contract are not executable qualification.

The predecessor POPGEN-05E3, INDIV-06A, SEL-06A, SEL-06B, and SEL-06C qualification lanes were last observed queued and unexecuted. A separate SEL-07A helper must freeze and execute the exact product head before any executable PASS is claimed.
