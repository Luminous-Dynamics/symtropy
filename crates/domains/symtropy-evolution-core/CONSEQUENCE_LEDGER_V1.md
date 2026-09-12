# Context-Bound Individual Consequence Ledger Contract v1

## Purpose

This contract records decomposed observed outcomes for every validated individual
in one exact explicit linked census under one externally bound context/window.

It is the first selection-adjacent authority in evolution-core, but it is **not**
a selection model.

## Core distinction

```text
observed consequence
    != causal fitness effect
    != selection coefficient
    != adaptation
```

A frequency increase is not evidence that an allele was beneficial. A larger
realized descendant count is an observed outcome, not proof that genotype caused
that outcome.

## Prerequisite identity

Every observation consumes INDIV-06A authority:

- exact `EvolutionIndividualId`;
- exact current `LinkedIndividualManifestDigest`;
- exact `ExplicitLinkedPopulationCensusDigest`;
- exact expected `PopulationId`.

Consequences are never attached to a canonical homolog-row index, ancestry-copy
ID alone, genome digest alone, or aggregate population state.

## External evolutionary context reference

`EvolutionaryContextRef` binds:

- `EvolutionaryContextId`;
- context-local revision;
- opaque 32-byte external content digest;
- `ConsequenceWindowId`.

Evolution-core does not interpret or authenticate the external ecology/world
state. The content digest is an exact semantic binding, not an authorization
token. A Living World/ecology adapter may later produce this reference from a
stronger authority chain.

Changing context identity, revision, content digest, or observation window
changes the context digest and invalidates an old observation under current
replay.

## Missing data is not zero

Each consequence channel is optional.

`None` means the channel was not supplied/observed. `Some(...0...)` means an
explicit zero was observed under that channel's declared semantics.

The canonical digest preserves this distinction.

## V1 consequence channels

### Viability

- `SurvivedWindow`
- `DiedDuringWindow`

This is descriptive only. The cause of death/survival remains external.

### Reproductive events

Records:

- observed opportunity count;
- realized reproductive-event count.

V1 requires realized events <= observed opportunities for this channel's
semantics. The vocabulary is intentionally generic rather than exclusively
sexual/mating-specific.

### Descendant production

Records an attributed produced-descendant/propagule count under the declared
external observation semantics.

### Descendant recruitment

Records how many attributed descendants reached the externally declared
recruitment boundary. When both production and recruitment are supplied,
recruited <= produced.

At least one channel must be supplied.

## IndividualConsequenceObservation

One observation binds:

- `ConsequenceObservationId`;
- exact individual identity;
- exact current individual-manifest digest;
- exact population identity;
- exact explicit-census digest;
- exact context/window digest;
- decomposed consequence values.

Construction and restore-time validation require the current exact census and
current linked subjects. A stale genome/ancestry/lineage manifest, census change,
population relabel, or context/window drift fails replay.

## ExplicitConsequenceLedger

The ledger represents one **complete explicit-census observation set** under one
context/window.

It requires:

- exactly one observation for every census member;
- no missing member;
- no extra member;
- unique individual observations;
- unique observation IDs;
- common exact population/census/context binding;
- canonical ordering by persistent individual ID.

Input ordering is non-semantic.

A caller therefore cannot silently provide only a favorable subset and have V1
represent it as a complete population-level consequence record.

## Local restored-data hardening

Even before current replay, a restored ledger's local validator requires every
embedded observation to carry the same population, census, and context digests
as the enclosing ledger. A hostile mixed-authority ledger cannot mint a
canonical ledger digest merely because each observation is locally well formed.

Current authority still requires full replay against the exact census, linked
subjects, and external context reference.

## What the ledger intentionally does not contain

The V1 consequence wire shape contains no:

- allele field;
- genotype field;
- phenotype field;
- fitness scalar;
- relative-fitness scalar;
- beneficial/deleterious classification;
- selection coefficient;
- adaptation classification.

Genetic/phenotypic/context attribution must be introduced by later evidence
layers rather than smuggled into consequence collection.

## Consequence identity theorem

Two genetically identical individuals may legitimately have different observed
consequences.

The same numeric consequence values observed under different context revisions,
content digests, or windows have different evidence identity.

Likewise, an unobserved channel and an explicitly observed zero have different
identity.

## Future causal selection boundary

A later selection-attribution layer should require at minimum:

1. qualified neutral/null fate behavior;
2. exact individual+census authority;
3. exact context-bound consequence observations;
4. phenotype/genotype exposure evidence appropriate to the hypothesis;
5. explicit causal/null comparison rather than frequency-change inference.

A possible future chain is:

```text
exact individual + census
        ↓
phenotype/genotype evidence
        +
external environment/ecology context
        ↓
complete consequence ledger
        ↓
null / counterfactual comparison
        ↓
qualified consequence association
        ↓
only then: causal selection hypothesis/evidence
```

V1 stops at the consequence ledger.

## Evidence boundary

Source implementation and static review are not executable qualification. The
neutral POPGEN-05E evidence stack and INDIV-06A helper remain separate frozen
subjects. Queued workflow jobs with no executed steps are not PASS or FAIL
evidence.
