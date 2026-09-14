# PHYSICAL AUTHORITY QUALIFIED LIVE SUBJECT v0.1

Status: proposed product theorem; exact-head executable qualification required before PASS is claimed.

Issue lineage: PHYS-ID-04C / #1082.

## Purpose

Preserve namespace-capability provenance when a plain `PhysicsBodySubject` locator is validated.

The existing types intentionally carry weaker meanings:

```text
PhysicsBodySubject
    = caller-constructible locator/reference data

ValidatedNetBody
    = live identity/index consistency under one PhysicsAuthorityWorld
```

Because legacy naked `(A,G)` wrappers can also mint `ValidatedNetBody`, that token must not be interpreted as proof that validation occurred through the stronger PHYS-ID-04/03 namespace-capability path.

This tranche adds:

```text
LocalNamespacePhysicsAuthorityWorld::validate_subject(...)
        ↓
LocalQualifiedValidatedNetBody<'a, D>
```

so the missing predicate is represented explicitly in the Rust type system.

## Exact theorem

`LocalQualifiedValidatedNetBody` proves only:

> this exact plain subject locator passed the existing `PhysicsAuthorityWorld::validate_subject` checks when validation was invoked through one live `LocalNamespacePhysicsAuthorityWorld`.

The production safe API provides no constructor from:

```text
PhysicsBodySubject
ValidatedNetBody
(A,G,N)
```

and no unchecked `From`/`Into` promotion.

## Construction

The qualified wrapper accepts a plain subject only as a requested locator:

```text
qualified.validate_subject(subject)
```

It then delegates to the existing live identity/index validator and wraps the successful lifetime-bound result privately.

This reuses rather than duplicates the existing checks for:

```text
PhysicalAuthorityId equality
WorldGenerationId equality
exactly one live matching NetId
NetId -> BodyHandle index agreement
BodyHandle -> NetId reverse agreement
```

Validation errors remain `PhysicsIdentityError` from the existing authority theorem.

## No laundering

The following promotion is intentionally unavailable:

```text
legacy.validate_subject(subject)
    -> ValidatedNetBody
    -> LocalQualifiedValidatedNetBody
```

The strong token appears only after validation through the qualified wrapper.

This is an identity-layer instance of PHYS-EVID-00/#1062:

```text
weaker evidence/token
    !-> stronger token
```

unless the theorem adding the missing predicate actually executes.

## Lifetime law

The qualified token owns the inner `ValidatedNetBody<'a,D>`.

Therefore it retains the existing immutable borrow of the authority world while live. Ordinary safe code cannot obtain mutable access to that same authority world until the token is dropped.

No serde/deserialization path recreates the live token.

## Read-only projection

The qualified token may expose:

```text
subject()
net_id()
body()
runtime_handle()
```

with exactly the same semantics as the inner validated token.

`runtime_handle()` remains explicitly ephemeral and is not promoted into durable identity.

No accessor returns ownership of the inner `ValidatedNetBody`; there is no generic upgrade/downgrade conversion API in this tranche.

## Equal locator values are not equal provenance

A legacy world and a namespace-qualified world may contain bodies for equal plain locator values:

```text
PhysicsBodySubject(A,G,N)
```

That does not make their validation tokens interchangeable.

The locator is data. The token type records which validation theorem executed.

Future evidence APIs that require PHYS-ID-04/03 provenance must therefore accept:

```text
&LocalNamespacePhysicsAuthorityWorld
```

and validate internally, or accept `LocalQualifiedValidatedNetBody` directly.

They must not accept a generic `ValidatedNetBody` and infer namespace qualification from matching numeric IDs.

## Relationship to #1080

#1080 provides the namespace capability chain:

```text
LocalQualifiedPhysicalAuthority
    -> LocalQualifiedWorldGeneration
    -> LocalNamespacePhysicsAuthorityWorld
```

This tranche preserves that provenance one step further into selected live-body validation.

It does not change #1080's process-local scope.

## Relationship to #1059

#1059 separates caller-constructible observation records from authority-issued evidence tokens.

This tranche applies the same no-laundering principle before observation capture, at the subject-validation boundary. Both are required for a clean evidence lattice.

## Required adversarial corpus

At minimum:

- qualified wrapper validates one bound body and returns exact subject/net/body/handle data;
- wrong physical authority fails with the existing mismatch error;
- wrong generation fails with the existing mismatch error;
- unknown NetId fails with the existing error;
- legacy wrapper can validate an equal plain subject but obtains only `ValidatedNetBody`;
- source audit finds no public constructor for `LocalQualifiedValidatedNetBody`;
- source audit finds no `From<ValidatedNetBody>` or `Into<LocalQualifiedValidatedNetBody>`;
- token fields remain private;
- token has no Serialize/Deserialize authority;
- token preserves the immutable validation lifetime;
- runtime handle remains documented as ephemeral.

## Non-claims

No complete-world structural integrity, no generation-scoped NetId tombstone theorem, no cross-process identity, no cryptographic provenance, no temporal ordering, no observation correctness, no persistence authenticity, no dwell/arrival/custody/delivery semantics, and no settlement authority.
