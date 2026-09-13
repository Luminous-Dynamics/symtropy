# Current species status V1

## Scope

SEL-10C classifies the **current** status of one ordered lineage pair only under one exact, currently qualified SEL-10B species model.

It does not establish a historical speciation event, a transition time, a universal taxonomic truth, or the correctness of another species concept.

The governing separation is:

`current reproductive isolation != persistent lineage history != model applicability != current species status != historical speciation event`.

## Preregistration

`CurrentSpeciesClassificationDesign` is outcome-free. It is declared from current non-Serde authorities before outcome-bearing SEL-10A/SEL-09B evidence is evaluated.

It freezes:

- the exact ordered lineage A/B pair;
- the exact SEL-10A lineage-history design digest;
- the exact SEL-09B reproductive-isolation design digest;
- the exact SEL-10B model digest;
- the built-in species-model content digest;
- the exact model validity-domain digest;
- the target-domain applicability protocol;
- the built-in content-digested V1 classification rule.

The design carries no realized 10A/09B status and no species-status result.

## Target-domain applicability

Applicability is evidence-bearing and is not inferred from the model name.

`SpeciesModelApplicabilityEvidence` binds:

- the exact ordered target lineage pair;
- the exact SEL-10B model digest;
- the exact model validity-domain digest;
- the preregistered applicability protocol;
- one typed disposition;
- the evidence/reason authority for that disposition.

The dispositions are:

- `InsideValidityDomain`;
- `OutsideValidityDomain`;
- `Unavailable`.

`OutsideModelValidityDomain` is not a negative taxonomic classification. It means the chosen model is not being applied to this target under its qualified domain.

## Required current inputs

Evaluation requires current non-Serde capabilities for:

1. the preregistered SEL-10C design;
2. the exact SEL-10A lineage-divergence history;
3. the exact SEL-09B reproductive-isolation evidence;
4. the exact SEL-10B biological-species model;
5. fresh applicability input.

The current 10A and 09B capabilities must bind the exact preregistered designs and ordered target lineage pair. The current 10B capability must bind the exact preregistered model.

## Persisted evidence

`CurrentSpeciesStatusEvidence` stores complete snapshots of:

- the SEL-10C classification design;
- the SEL-10A lineage history;
- the SEL-09B reproductive-isolation evidence;
- the SEL-10B species model;
- target-domain applicability evidence;
- the derived current status.

It also stores the canonical digests of those exact upstream subjects.

This is intentional: local validation can detect snapshot/digest/status tampering without pretending the persisted bytes are current authority.

## V1 deterministic status rule

The built-in V1 classification precedence is:

1. applicability `OutsideValidityDomain` → `OutsideModelValidityDomain`;
2. applicability `Unavailable` → `InsufficientEvidence`;
3. SEL-10A `LineageFusionObserved` or `NotPersistent`, or SEL-09B `Contradicted` → `ContradictedUnderModel`;
4. SEL-10A or SEL-09B `InsufficientEvidence` → `InsufficientEvidence`;
5. SEL-09B `NotSupported` → `NotSupportedUnderModel`;
6. SEL-09B `Supported` plus SEL-10A `PersistentDivergenceObserved` or `DivergenceWithRecontact` → `SupportedUnderModel`;
7. otherwise → `NotSupportedUnderModel`.

Historical recontact therefore does not automatically erase current species support under this strict SEL-10B lane when the current complete-barrier theorem is independently supported and fusion/persistence loss is absent.

## Current statuses

V1 exposes exactly:

- `SupportedUnderModel`;
- `NotSupportedUnderModel`;
- `ContradictedUnderModel`;
- `InsufficientEvidence`;
- `OutsideModelValidityDomain`.

These are model-bound current evidence states, not universal species truth.

## Local validation

Local canonicalization must recompute and verify:

- classification-design digest;
- embedded lineage-history digest and design binding;
- embedded reproductive-isolation digest and design binding;
- embedded species-model digest, model-content binding, and validity-domain binding;
- ordered target-lineage agreement across upstream snapshots;
- applicability lineage/model/domain/protocol binding;
- the derived V1 status.

Changing only a serialized final status must therefore fail.

Changing only an embedded upstream digest/snapshot or applicability subject must also fail.

## Current replay

Persisted data and canonical digests are representation identity only.

`ValidatedCurrentSpeciesStatus<'_>` is non-Serde and regains current authority only by re-running the complete evaluation from:

- the current validated SEL-10C design;
- current validated SEL-10A history;
- current validated SEL-09B isolation evidence;
- current validated SEL-10B model;
- fresh applicability evidence.

Exact equality with the persisted evidence is required.

## Model identity

A different SEL-10B qualification or validity domain is a different model authority. A status produced under one model authority cannot be replayed under another merely because the built-in mathematical/model-content digest is the same.

Different qualified species models may legitimately classify the same biological observations differently. Such results must remain distinct evidence identities.

## Test requirements

The V1 corpus covers:

- `SupportedUnderModel` from complete isolation + persistent divergence;
- support with qualified historical recontact but no fusion;
- `NotSupportedUnderModel` from adequate but unsupported isolation;
- `ContradictedUnderModel` from current isolation contradiction;
- contradiction from lineage fusion or persistence loss;
- `InsufficientEvidence` from incomplete lineage/isolation evidence;
- unavailable applicability;
- `OutsideModelValidityDomain` as distinct from a negative classification;
- Serde/current replay;
- applicability evidence as evidence identity;
- serialized status tampering;
- applicability subject transplantation;
- embedded species-model digest tampering;
- outcome-free classification-design wire shape;
- absence of historical speciation-transition fields.

## Explicit non-claims

SEL-10C does **not** establish:

- when divergence began;
- when reproductive barriers arose;
- whether one exact historical speciation transition occurred;
- a transition generation or timestamp;
- the mechanism of speciation;
- irreversible isolation;
- permanent absence of gene flow;
- validity under a different species concept;
- universal species identity.

Those temporal/event claims remain reserved for SEL-10D.
