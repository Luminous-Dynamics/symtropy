# SEL-10B — Strict Biological-Species Model V1

Status: source contract for `symtropy-evolution-core`.

This tranche defines and qualifies one explicit species model before any lineage classification is attempted.

The governing boundary is:

`same observations + different species model may legitimately produce different classification evidence`.

SEL-10B therefore creates model authority only. It consumes no target lineage, no SEL-09B result, and no SEL-10A result.

## 1. V1 scope

V1 is a deliberately strict complete-isolation operationalization of a biological-species model.

It is **not** claimed to be the universal definition of species and is not intended to force all taxonomic groups into one ontology.

The built-in model content requires, for a future current-status classifier:

- current complete reproductive-isolation support;
- persistent lineage-divergence history;
- current fertile hybridization or realized hereditary gene flow to contradict this strict complete-isolation lane;
- lineage fusion or explicit loss of lineage persistence to contradict distinct current-species status;
- geographic isolation alone to be insufficient;
- incomplete lineage sorting alone to be non-decisive;
- ecology to be optional/not required in this narrow V1 lane;
- missing or conflicting required evidence to yield insufficient evidence rather than a favorable default.

Historical recontact or introgression may remain compatible when current complete-barrier evidence is supported and the lineage history does not report fusion or loss of persistence.

## 2. Outcome-free theorem

`BiologicalSpeciesModel` must be qualified before target-lineage classification.

The persisted model contains no:

- lineage A/B identity;
- reproductive-isolation status;
- lineage-divergence status;
- current species-status result;
- speciation-event result;
- transition interval.

This prevents a species model from being selected or rewritten after seeing a favorable target outcome without that model choice changing evidence identity.

## 3. Built-in model content identity

`strict_biological_species_model_content_digest_v1()` is computed from a versioned domain separator plus the exact V1 model specification.

The persisted model stores that exact content digest and local validation requires equality with the built-in V1 digest.

The policy surface is also persisted explicitly and local validation requires every policy enum to match the built-in V1 values.

Therefore a serialized object cannot retain the V1 method identity while silently changing one model rule.

## 4. Validity-domain authority

Every qualified model binds one `SpeciesModelValidityDomainRef` consisting of:

- a semantic validity-domain ID; and
- an explicit external authority qualifying that domain.

The validity-domain digest is part of model authority identity.

A different domain is a different model authority even when the built-in V1 content digest is identical.

### Domain versus classification

`outside model validity domain` is not the same statement as `not a species under the model`.

Future SEL-10C classification must preserve that distinction with an explicit typed status such as `OutsideModelValidityDomain` rather than collapsing it into a negative classification.

The validity-domain authority is an auditable trust binding. SEL-10B does not independently prove that a taxon/population system belongs inside that domain from raw biological observations.

## 5. Independent qualification authority

Every model also binds a `SpeciesModelQualificationRef` containing:

- the qualification authority;
- exact built-in model-content digest;
- exact validity-domain digest.

Changing only the qualification authority changes the canonical model authority identity while leaving the built-in model-content digest unchanged.

Changing only the validity domain likewise changes authority identity while leaving the built-in content unchanged.

This keeps three distinct questions separate:

1. what model semantics are being used?
2. where is that model considered valid?
3. who/what qualifies that model for that domain?

## 6. Strict V1 reproductive-isolation requirement

`ReproductiveIsolationModelRequirement::CurrentCompleteBarrierSupported` means future SEL-10C must require current SEL-09B complete-barrier support for this strict lane.

SEL-10B itself does not consume SEL-09B evidence and therefore does not classify anything.

This is intentionally stricter than many real taxonomic practices in which recognized species can hybridize or introgress.

Such systems may:

- fall outside this V1 validity domain; or
- require a future different species model with explicitly different hybridization/gene-flow semantics.

They must not be forced into `NotSupportedUnderModel` merely because this narrow strict lane is inapplicable.

## 7. Lineage-divergence requirement

`LineageDivergenceModelRequirement::PersistentOrRecontactWithoutFusion` states the future interpretation expected by V1:

- a clean persistent SEL-10A divergence history may satisfy the lineage-history side;
- divergence with historical recontact/gene flow may remain compatible when current complete isolation is independently supported;
- explicit lineage fusion contradicts distinct current-species status in this V1 lane;
- explicit loss of lineage persistence likewise contradicts distinct current-species status;
- insufficient lineage history remains insufficient.

This policy is model semantics, not a current lineage result.

## 8. Current gene flow and historical recontact

V1 intentionally distinguishes current evidence from historical counter-history.

`CurrentGeneFlowPolicy::ContradictsCompleteIsolation` means current viable fertile hybridization or realized hereditary gene flow contradicts the strict current complete-isolation lane.

`HistoricalRecontactPolicy::CompatibleWhenCurrentCompleteBarrierSupported` means prior recontact/introgression recorded by SEL-10A does not automatically erase the possibility of distinct current species under this V1 model.

The historical evidence remains visible in 10A identity and must not be deleted to obtain a favorable classification.

## 9. Geographic isolation

`GeographicIsolationPolicy::NeverSufficientAlone` prevents allopatry or lack of observed contact from becoming species status by itself.

This is consistent with the upstream SEL-09 rule that no contact is not evidence of a reproductive barrier.

A later classifier cannot substitute geography for current reproductive-isolation evidence while retaining this V1 model identity.

## 10. Incomplete lineage sorting

`IncompleteLineageSortingPolicy::DoesNotDecideStatusAlone` prevents a genomic lineage-sorting pattern from being treated as a standalone species-status theorem.

V1 does not implement a full incomplete-lineage-sorting inference engine. If a future taxonomic model needs explicit genomic/coalescent evidence, that must be added as a separate model/evidence requirement rather than inferred from this enum label.

## 11. Ecology policy

`EcologySpeciesEvidencePolicy::NotRequiredInStrictV1` means ecology is not a required evidence family for this narrow V1 biological-species lane.

It does **not** mean ecological differentiation is biologically irrelevant.

A future ecological-species model, integrative-taxonomy model, or other concept may require explicit ecological/context evidence and will therefore have a different model-content identity.

## 12. Missing/conflicting evidence

`SpeciesModelMissingConflictPolicy::InsufficientEvidence` requires missing/conflicting required evidence to remain explicit and prevents favorable default classification.

SEL-10B itself performs no classification; this field constrains future SEL-10C behavior under the model.

## 13. Representation identity versus current model authority

`BiologicalSpeciesModel`, its validity-domain reference, qualification reference, and canonical digest are serializable representation identity.

They are not current model authority after deserialization.

Current authority is non-Serde:

`ValidatedBiologicalSpeciesModel<'_>` reconstructs the model from:

- the exact semantic model ID already stored;
- fresh current validity-domain authority;
- fresh current qualification authority;
- the built-in V1 model-content specification.

The reconstructed model must exactly equal the persisted model.

Qualification drift or validity-domain drift therefore fails current replay.

## 14. Canonical identity

The model canonical digest binds:

- version;
- semantic model ID;
- exact built-in model-content digest;
- validity-domain digest;
- qualification authority and its content/domain bindings;
- every explicit V1 policy enum.

The model contains no target-lineage data or target classification result.

## 15. Corpus

The dedicated `species_model_v1` corpus covers:

- exact built-in V1 policy surface;
- target/outcome-free wire shape;
- model-content identity remaining stable across qualification drift;
- qualification drift changing canonical model authority identity;
- validity-domain drift changing canonical model authority identity;
- Serde restoration plus fresh current replay;
- stale qualification rejection;
- serialized qualification-binding tamper rejection.

## 16. Explicit non-claims

SEL-10B does **not** establish:

- that any particular lineage pair is a species pair;
- current species status;
- reproductive isolation for any target;
- persistent divergence for any target;
- that a target lies inside the validity domain;
- historical speciation;
- a speciation time;
- a speciation mechanism;
- universal taxonomic truth;
- equivalence of different species concepts.

SEL-10C must separately combine a current qualified species model with current target evidence before any present species-status capability can exist.

SEL-10D remains a separate temporal/historical transition authority.

## 17. Qualification boundary

Source/static review is not executable qualification.

A frozen CI helper must bind the exact immutable SEL-10B product SHA and run pinned Rust/tooling, formatting, all-target checking, `species_model_v1`, the SEL-10A lineage-history corpus, SEL-09B/09A prerequisites, the adaptation/selection chain, all tests, and strict Clippy.

Queued or unassigned CI is unexecuted evidence: neither PASS nor product-code FAIL.
