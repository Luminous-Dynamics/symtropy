# Validated Consequence Authority Contract v1

## Purpose

`ValidatedConsequenceLedger<'a>` is a non-serializable, point-in-time capability proving that one `ExplicitConsequenceLedger` successfully replayed against the exact current authorities supplied to its constructor.

It exists to prevent a deterministic representation digest from being mistaken for current evidence authority.

## Core distinction

```text
representation identity
    !=
current evidence authority
```

`ExplicitConsequenceLedger::canonical_digest()` identifies one locally valid representation. It does not by itself prove exact current census completeness, current individual manifests, current population identity, or current external context.

`ValidatedConsequenceLedger::validate_current(...)` performs that stronger replay before the capability can exist.

## Construction authority

Construction requires:

- the candidate `ExplicitConsequenceLedger`;
- expected `PopulationId`;
- exact `HereditarySchema`;
- exact `ChromosomeMap`;
- exact `ExplicitLinkedPopulationCensus`;
- exact current `LinkedIndividualSubject` set;
- exact `EvolutionaryContextRef`.

The constructor delegates to `ExplicitConsequenceLedger::validate_current(...)`, which reconstructs the complete ledger from those authorities and requires exact equality.

A raw `ExplicitConsequenceLedgerDigest` cannot construct this capability.

## Persistence boundary

The capability deliberately does not implement Serde.

Persisted bytes may preserve a ledger representation, but they do not preserve point-in-time authority. After restore, callers must supply the exact current authorities again and construct a fresh capability.

This prevents an old successful validation from becoming a transferable authorization token.

## Borrow boundary

The capability immutably borrows the validated ledger for its lifetime.

While the capability exists through ordinary safe Rust borrowing, the same ledger cannot be mutably changed behind it.

The other supplied authorities are validated at construction time and represented in the capability by their exact semantic identities/digests. The capability does not claim that an external world or ecology can never change after validation.

If downstream work requires a later external state, it must validate against that later authority rather than reusing an old capability as proof of a new state.

## Exposed evidence

The capability exposes only read-only evidence:

- the validated ledger reference;
- exact ledger representation digest;
- expected population identity;
- exact explicit-census digest;
- exact context/window digest.

Downstream selection-adjacent APIs should require the capability itself, not merely one of these digests.

## Partial-ledger theorem

A hostile or accidental Serde-restored ledger may be internally well formed, canonically ordered, and share one population/census/context identity while containing only a subset of the census.

Such a value may have a deterministic representation digest.

It cannot construct `ValidatedConsequenceLedger` against the exact current census because full replay rejects incomplete coverage.

Therefore:

```text
has canonical digest
    !=
proved complete current evidence
```

## Non-claims

This capability does not establish:

- authenticity or scientific correctness of the external ecology/context provider;
- phenotype truth;
- individual exposure truth;
- genotype or phenotype causation;
- scalar or relative fitness;
- selection coefficient;
- beneficial/deleterious classification;
- adaptation;
- speciation.

It proves only that one consequence ledger successfully replayed against the exact authority objects supplied at construction time.

## Successor use

SEL-06C phenotype/exposure evidence should consume `ValidatedConsequenceLedger<'_>` rather than `ExplicitConsequenceLedgerDigest` directly.

Later association and causal-selection layers should preserve the same rule: representation identifiers may bind evidence, but only validated capabilities or equivalently explicit replay proofs may confer current authority.
