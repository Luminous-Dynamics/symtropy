# Selection Evidence Bridge Contract v1

## Purpose

SEL-06C binds externally produced phenotype and individual-exposure evidence to the exact persistent individuals and exact observed consequences already validated by SEL-06A/SEL-06B.

Evolution-core remains an evidence-binding authority. It does not become a morphology engine, sensor model, ecology engine, phenotype ontology, or causal inference engine.

## Core distinctions

```text
context membership != individual exposure
individual exposure != causal effect
phenotype observation != genotype cause
missing observation != observed zero
partial/censored observation != complete observation
representation identity != current evidence authority
```

These distinctions are structural, not comments.

## External evidence references

`PhenotypeEvidenceRef` and `ExposureEvidenceRef` are opaque bindings. Each binds:

- exact `EvolutionIndividualId`;
- typed external source identity;
- source-local revision;
- opaque evidence-content digest;
- exact measurement/evidence protocol identity;
- opaque protocol-content digest;
- exact SEL-06A context/window digest.

Evolution-core does not interpret the external evidence payload and does not infer units, trait meaning, dose, mechanism, or causal direction from its digest.

A reference is evidence identity, not proof that the external producer is scientifically correct.

## Observation support

Each individual carries explicit phenotype and exposure status.

### `Unavailable`

No evidence is supplied for that channel.

This is not zero phenotype, zero exposure, absence of a trait, survival, failure, or any other numerical observation.

### `PartialWindow`

Evidence exists but the observation is explicitly incomplete or censored for the required window. An opaque `ObservationSupportDigest` binds the external support/censoring record.

Partial evidence is not silently promoted to complete evidence.

### `CompleteWindow`

The external evidence authority declares the supplied evidence complete for the required window/protocol semantics.

Evolution-core binds that declaration. It does not independently prove the scientific correctness of the producer's completeness claim.

## Complete denominator theorem

`ExplicitSelectionEvidenceLedger` contains exactly one evidence-status record for every individual in the validated consequence ledger.

An individual with unavailable phenotype and unavailable exposure remains present as an explicit record.

Therefore missing measurement cannot silently remove an organism from the population denominator.

Capture rejects missing, duplicate, or extra individual inputs rather than constructing a convenient complete-case population.

Input order is non-semantic. Canonical record order follows the already canonical persistent-individual order of the validated consequence ledger.

## Exact individual and consequence binding

Every persisted record binds:

- exact persistent individual identity;
- exact current `LinkedIndividualManifestDigest`;
- exact `IndividualConsequenceObservationDigest`;
- explicit phenotype status;
- explicit exposure status.

Consequently, changing genome/ancestry/mutation-lineage-backed manifest identity or changing the observed consequence stales strict replay even if the same human-readable individual ID remains.

## Protocol identity

Evidence recorded under different protocols is not silently comparable merely because a source, trait name, factor name, or numerical payload appears similar.

Protocol content is part of canonical evidence identity.

Changing protocol identity or protocol content changes bridge identity.

Whether two protocols are scientifically comparable is deferred to a declared SEL-07 comparison design.

## Context and exposure separation

The SEL-06A `EvolutionaryContextRef` identifies the external environment/context/window authority shared by the consequence ledger.

`ExposureEvidenceRef` is per individual.

Two organisms in the same context may therefore have different exposure content, different support, or unavailable exposure evidence.

This prevents the shortcut:

```text
organism was in context X
therefore organism received causal factor F
```

## Current-authority boundary

SEL-06C consumes `ValidatedConsequenceLedger<'_>`, not a raw consequence-ledger digest.

A persisted `ExplicitSelectionEvidenceLedger` can have deterministic representation identity, but downstream analysis should consume `ValidatedSelectionEvidenceLedger<'_>`.

That non-Serde capability can only be reconstructed by replaying the bridge against:

- a current `ValidatedConsequenceLedger<'_>`;
- exact schema and chromosome map;
- exact current explicit census;
- exact current linked-individual subjects.

Thus SEL-07 does not need to rediscover the representation/current-authority seam.

## Non-claims

SEL-06C does not establish:

- phenotype heritability;
- genotype-to-phenotype causation;
- exposure-to-consequence causation;
- competition mechanism;
- expected lifetime reproductive success;
- scalar or relative fitness;
- beneficial/deleterious mutation status;
- selection coefficient;
- adaptation;
- reproductive isolation;
- speciation.

## Successor requirement

SEL-07 must begin with a declared comparison design before estimating an effect.

That design must bind the target endpoint/estimand, comparison class, denominator/inclusion policy, missingness/censoring treatment, confounding/adjustment declaration, uncertainty method, and any population-genetic model used to translate an effect into a model-specific selection coefficient.

A descriptive association must remain a different authority type from a causal selection effect.
