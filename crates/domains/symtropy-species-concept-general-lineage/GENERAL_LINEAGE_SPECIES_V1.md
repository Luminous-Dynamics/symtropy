# General-Lineage Species Authority V1

## Scope

SEL-10E1B V1 implements one explicit current-status species-concept family whose conceptual target is a separately evolving metapopulation lineage segment.

It is a distinct authority family, not a relaxed configuration of the strict biological-species model.

V1 does not establish historical speciation timing, nomenclature, a universal delimitation threshold, or universal taxonomy truth.

## Governing theorem

```text
separately evolving lineage concept
    != one operational criterion
    != complete reproductive isolation
    != one genetic cluster
    != majority vote across measurements
    != universal species truth
```

And:

```text
complete-isolation contradiction
    != automatic contradiction of lineage separation
```

## Model authority

`GeneralLineageSpeciesModel` binds:

- stable family/model identity;
- exact V1 semantic-content digest;
- exact qualified validity domain;
- exact model qualification authority;
- built-in V1 model rule.

The conceptual content states that:

- separately evolving lineage is the conceptual target;
- reproductive isolation is optional corroboration rather than a universal defining requirement;
- monophyly is not universally required;
- population subdivision alone is insufficient;
- no universal divergence-distance threshold exists;
- sexual and asexual targets may be in-domain only under an explicit qualified validity domain;
- evidence channels are preregistered before outcomes;
- no scalar species score exists.

Persisted model bytes are representation only. Current model authority is non-Serde and requires exact replay from the current validity-domain and qualification authorities.

## Open descriptor projection

The family projects through SEL-10E1A2 into the open species-concept waist with conceptual family ID:

`general-lineage-species`

Its conceptual identity is independent of qualification/domain drift. Its concrete authority identity changes when the exact model qualification or validity domain changes.

The descriptor exposes current species-status capability only. It does not expose historical transition or nomenclature authority.

## Outcome-free classification design

`GeneralLineageClassificationDesign` is frozen before evidence outcomes are evaluated.

It binds:

- exact current SEL-10A lineage-history design;
- exact current general-lineage model;
- target/model applicability protocol;
- canonical ordered evidence-channel declarations;
- minimum qualifying support-group threshold;
- missing-evidence policy;
- built-in dependency-grouping rule;
- built-in classification rule.

Exactly one channel must be `CoreRequired + LongitudinalLineageSeparation`.

The positive threshold is at least two distinct dependency groups in V1.

## Evidence channels

V1 channel kinds include:

- longitudinal lineage separation;
- demographic independence;
- population connectivity;
- genetic diagnosability;
- phenotypic diagnosability;
- ecological differentiation;
- reproductive isolation;
- genealogical concordance.

Roles are:

- `CoreRequired`;
- `Required`;
- `Corroborating`;
- `Optional`.

Every channel binds:

- stable channel ID;
- channel kind and role;
- evidence-dependency group ID;
- dependency-group qualification authority;
- evidence protocol authority;
- channel applicability authority.

Channel order is canonical and non-semantic.

## Dependency-group qualification

Raw channel count is never a corroboration count.

V1 counts distinct preregistered dependency groups. Each group is evidence-bearing:

- all channels sharing a group must bind the same exact grouping qualification authority;
- one grouping qualification authority cannot be relabeled as multiple group IDs;
- qualification drift changes design identity and stales replay;
- the built-in grouping rule is content-bound and cannot be replaced in restored bytes.

This is an anti-pseudoreplication theorem. It prevents obvious inflation such as counting multiple metrics from one sequence dataset as independent corroborations merely because they have different channel names.

It is **not** a claim of statistical independence, causal independence, organizational fault-domain independence, or cross-model independence. SEL-10E2 must establish those stronger properties separately when model families are compared.

## Native SEL-10A core channel

The core longitudinal channel is mechanically derived from current SEL-10A lineage-history authority.

Mapping:

- `PersistentDivergenceObserved` -> supports separation;
- `DivergenceWithRecontact` -> supports separation;
- `LineageFusionObserved` -> contradicts separation;
- `NotPersistent` -> contradicts separation;
- `InsufficientEvidence` -> unavailable.

Callers cannot supply an alternate external value for the core channel.

Thus recontact or realized historical gene flow does not automatically erase lineage separation when the lineages remain persistently distinct.

## Native SEL-09B reproductive-isolation channel

If a preregistered channel kind is `ReproductiveIsolation`, V1 requires current SEL-09B authority. An opaque external replacement is rejected.

SEL-09B is interpreted as corroborative evidence under this family:

- `Supported` -> supports separation;
- `NotSupported` -> does not support separation;
- `Contradicted` -> does not support separation;
- `InsufficientEvidence` -> unavailable.

The important boundary is deliberate:

A fertile hybrid or realized gene flow can contradict the strict **complete reproductive-isolation** theorem without, by itself, proving that two persistent metapopulation lineages are not separately evolving.

Therefore SEL-09B `Contradicted` is not mapped to general-lineage `ContradictsSeparation`.

## External channels

Channels without native Symtropy authority may enter through explicit external evidence and qualification authorities.

These authority references are trust bindings, not internally re-derived scientific truth. Current replay requires the exact same fresh external evidence surface.

A future native genetic/ecological/morphological/phylogenetic authority should replace the corresponding opaque external channel rather than fabricating observations from existing lineage history.

## Result surface

V1 exposes only:

- `SupportedUnderGeneralLineageModel`;
- `NotSupportedUnderGeneralLineageModel`;
- `ContradictedUnderGeneralLineageModel`;
- `InsufficientIndependentEvidence`;
- `OutsideModelValidityDomain`.

The persisted result retains the exact design, applicability evidence, full channel matrix, supporting dependency groups, unavailable potential groups, and typed status.

No scalar score exists.

## Decision precedence

1. Outside the qualified model domain -> `OutsideModelValidityDomain`.
2. Unavailable model applicability -> insufficient or fail-closed under the frozen policy.
3. Any applicable channel explicitly qualified as `ContradictsSeparation` -> `ContradictedUnderGeneralLineageModel`.
4. Core lineage-separation channel unavailable/outside -> insufficient.
5. Core channel does not support separation -> `NotSupportedUnderGeneralLineageModel`.
6. Missing required applicable channels -> insufficient.
7. Required channels explicitly not supporting -> not supported.
8. Positive threshold met across distinct qualified dependency groups -> supported.
9. If unavailable independent groups could still change the threshold -> insufficient.
10. Otherwise -> not supported.

`OutsideChannelDomain` is distinct from negative evidence.

## Missing evidence

Missing data is not coerced into negative evidence.

Under `ReportInsufficientIndependentEvidence`, unavailable groups cause an insufficient result only when they are required or when they could still change the preregistered support threshold.

Under `FailClosed`, unavailable required evidence is rejected.

## Replay and currentness

`ValidatedGeneralLineageClassificationDesign<'_>` and `ValidatedGeneralLineageSpeciesEvidence<'_>` are non-Serde capabilities.

Serialized restoration can establish local canonical representation only. Current authority requires fresh replay from:

- current SEL-10A design/history;
- current general-lineage model;
- current model-applicability evidence;
- current native SEL-09B evidence where declared;
- current exact external evidence/qualification authorities for generic channels.

## Adversarial theorems

The V1 corpus covers:

- distinct general-lineage conceptual identity;
- sexual/asexual domain variation without changing conceptual family identity;
- asexual in-domain support without reproductive-isolation evidence;
- multiple metrics in one qualified dependency group do not inflate corroboration;
- conflicting qualifications inside one group are rejected;
- one grouping qualification cannot be renamed into multiple groups;
- grouping qualification drift changes design identity and stales replay;
- grouping-rule tampering fails locally;
- missing evidence is counterfactually classified rather than coerced negative;
- recontact with persistent lineages can remain supported;
- lineage fusion contradicts separate-lineage status;
- native SEL-09B support can corroborate the model;
- native SEL-09B complete-isolation contradiction is not a general-lineage veto;
- independent other corroboration can support despite complete-isolation failure;
- external reproductive-isolation bypass is rejected;
- forged aggregate status/support-group summaries cannot canonicalize;
- qualification drift stales current replay.

## Explicit non-claims

SEL-10E1B V1 does not claim:

- that the general-lineage concept is philosophically final;
- that dependency-group qualification proves statistical independence;
- that any universal evidence threshold is biologically correct across all taxa;
- that every persistent cluster is a species;
- that reproductive isolation is irrelevant;
- that gene flow is always compatible with lineage separation;
- historical speciation timing;
- nomenclatural authority;
- cross-model robustness;
- universal taxonomy truth.
