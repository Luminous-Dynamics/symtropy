# SYM-EVAL-CORE-009A — Generic benchmark-validity reference v1

## Status

**Reference only.** This tranche freezes generic admission semantics for review and adversarial testing. It does not export a production API, and **production authority is not established**.

The executable reference intentionally lives in an integration test. It does not import or inherit authority from the still-unqualified CORE-001 product implementation.

Production CORE-009 remains gated on a genuine **CORE-001 r2 exact execution PASS** from the immutable exact-subject qualifier.

## Purpose

CORE-009A answers one narrow question:

> Given a precommitted validity profile and a set of gate-evidence bindings, what exact canonical receipt says whether performance may be interpreted as confirmatory?

It does **not** decide whether a domain-specific gate is scientifically well-designed. Domain adapters own their own gate vocabulary and map those results into this generic algebra.

The reference deliberately contains no visual names such as `A3`, `Q2`, `FreshHidden`, object permanence, renderer shielding, or tracker semantics.

## Architecture

```text
domain-specific gate vocabulary
        |
        v
generic ValidityProfileV1
        |
        +-- claim schema
        +-- admission policy version
        +-- exact compatibility bindings
        +-- required gates
        +-- required subject/verifier identities
        +-- bounded N/A policy
        |
        v
canonical profile bytes
        |
        v
ValidityProfileDigestV1
        |
        +----------------------------+
        |                            |
        v                            v
exact gate evidence            admission evaluator
bindings                            |
        |                            v
        +----------------------> full blocker vector
                                     |
                                     v
                              deterministic primary state
                                     |
                                     v
                             canonical validity receipt
```

The display label is deliberately excluded from canonical profile identity. A caller cannot obtain a new semantic identity merely by renaming a profile, and cannot preserve identity while changing the canonical requirements.

## Canonical identity

V1 uses the same domain-hash shape intended by CORE-001:

```text
u32_be(domain_len)
domain_utf8
u64_be(payload_len)
payload
```

Domains are:

```text
sym-eval.validity-profile.v1
sym-eval.validity-gate-evidence.v1
sym-eval.validity-receipt.v1
```

The reference uses SHA-256 because the shared evidence substrate already uses SHA-256. The production implementation should reuse the qualified CORE digest type and encoder rather than duplicate this test-local implementation.

### Profile identity

Canonical profile bytes bind:

- claim schema ID and version;
- admission-policy version;
- sorted compatibility requirements;
- sorted gate requirements;
- each gate's exact admission policy;
- optional exact subject identity;
- optional exact verifier identity.

They do not bind the display label.

Reference vector:

```text
profile_digest =
e9ed0534345c5ab153232a85dab89409f55d2cc343cdc28f74e33395443e4a4d
```

### Gate-evidence identity

Each gate-evidence digest binds:

- gate key;
- exact status;
- **opaque evidence reference**;
- optional exact subject identity;
- optional exact verifier identity;
- sorted compatibility bindings.

The evidence reference in CORE-009A is only an opaque nonzero binding. A nonzero 32-byte value is not proof that committed evidence exists.

Resolving that reference against CORE-001/CORE-002 artifact/event authority belongs to CORE-009B.

### Receipt identity

Canonical receipt bytes bind:

- content-derived profile digest;
- deterministic primary admission state;
- every canonical gate-evidence digest;
- the complete sorted blocker vector.

Reference vector for the all-PASS fixture:

```text
receipt_digest =
d853d852cfc2d9aa83f9682432adaedac671d9b8e286129e27a8ff80b11a2e9d
```

Changing any committed evidence byte changes the gate-evidence digest and therefore the receipt digest.

## Generic status vocabulary

The reference distinguishes:

```text
Pass
Fail
Invalid
NotApplicable(reason)
NotExecuted
InfrastructureQueued
InfrastructureFailure
Superseded
EvidenceStale
Underpowered
ProfileMismatch
```

This is intentionally richer than `valid: bool`.

A domain adapter may map stronger domain-specific outcomes into these generic classes. For example, a deterministic leak or manipulation result can map to generic `Invalid` while retaining its domain-specific receipt separately.

A domain adapter may not reinterpret:

```text
InfrastructureQueued
NotExecuted
Superseded
EvidenceStale
```

as `Pass`.

## N/A semantics

`NotApplicable` is not a free-form escape hatch.

A gate may use N/A only when the profile precommits:

```text
PassOrNotApplicable(exact_reason)
```

The exact reason must match. A different reason is a profile mismatch.

A gate declared `Pass` cannot be replaced by N/A.

## Compatibility join

The generic engine does not hardcode particular binding names.

The profile declares exact key/value requirements. For the reference fixture those keys are illustrative:

```text
environment
experiment-plan
population
producer-profile
scorer-profile
sensor-profile
```

Production domain profiles may declare different versioned keys.

For every required gate:

- every required key must be present;
- every required value must match;
- undeclared extra keys fail closed;
- required subject identity must match when declared;
- required verifier identity must match when declared.

This prevents individually plausible receipts from unrelated profiles, populations, environments, models, or verifiers from being silently joined.

## Complete blockers, deterministic primary state

The complete blocker vector is authoritative for diagnosis.

A single primary state exists only as a deterministic summary projection:

```text
ProfileMismatch
    >
StaleEvidence
    >
InvalidBenchmark
    >
Infrastructure
    >
IncompleteValidity
```

This precedence does not discard lower-priority blockers.

For example, if one gate is invalid, another is stale, and a third has the wrong subject identity, the receipt retains all three blockers while the deterministic primary state is `BlockedProfileMismatch`.

## Performance visibility

Only:

```text
ConfirmatoryAdmitted
```

permits:

```text
PerformanceAvailabilityV1::Confirmatory
```

Every blocked state exposes performance only as:

```text
DiagnosticOnly
```

This prevents a benchmark from publishing a strong result first and treating validity as explanatory metadata later.

## Duplicate and ambiguity rejection

V1 rejects before receipt construction:

- duplicate gate requirements;
- duplicate compatibility requirements;
- duplicate gate evidence;
- duplicate compatibility bindings within one gate evidence item.

Unexpected gates and unexpected compatibility keys do not disappear; they become explicit profile-mismatch blockers.

## Terminal score composition

CORE-001 v1 makes the score event terminal.

Production integration should therefore be:

```text
validity evidence complete
        |
        v
CORE-009 validity receipt
        |
        v
performance report references
exact validity-receipt digest
        |
        v
CORE-001 terminal score commits both:
  - benchmark-validity-receipt artifact
  - performance-report artifact
```

Do not append validity after the **terminal score**.

The validity receipt must not reference the final performance-report digest. The performance report points to the validity digest, avoiding a commitment cycle.

CORE-002 finalization should verify the required terminal artifact set for confirmatory profiles.

CORE-005/Xenia attestation may authenticate the finalized receipt identity but can never upgrade a blocked validity result.

## Relationship to visual Q5

SYM-EVAL-001E-Q5 owns visual-specific semantics and hostile fixtures.

CORE-009A owns only generic mechanics.

Later CORE-009C should map the visual profile into the qualified generic substrate and port the Q5 hostile corpus without moving visual semantics into shared CORE.

## Required successor

CORE-009B must replace the test-local opaque evidence reference with exact evidence resolution.

At minimum it must prove that a claimed gate result resolves to committed/verifiable evidence with the required:

- artifact/event role;
- schema identity;
- experiment identity;
- subject identity;
- verifier identity;
- compatibility bindings;
- finalized-receipt state where required.

A struct that merely says `status = Pass` must not be self-authenticating.

## Nonclaims

CORE-009A does not establish:

- production benchmark-validity authority;
- qualification of CORE-001, CORE-002, or Q5;
- existence or authenticity of evidence behind an opaque evidence reference;
- signer or runner trust;
- statistical power;
- absence of unknown shortcuts;
- object permanence or visual reasoning;
- model performance;
- physical safety.

Its claim is narrower: this reference defines deterministic canonical bytes and fail-closed generic admission semantics suitable for review before promotion into the shared qualified evidence substrate.
