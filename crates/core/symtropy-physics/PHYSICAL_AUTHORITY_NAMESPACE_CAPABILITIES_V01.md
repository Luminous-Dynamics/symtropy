# PHYSICAL AUTHORITY NAMESPACE CAPABILITIES v0.1

Status: proposed product theorem; exact-head compilation/tests/Clippy required before PASS is claimed.

Issue lineage: PHYS-ID-04 / #1079 and PHYS-ID-03 / #1078.

## Purpose

Introduce an additive process-local capability chain so ordinary safe code cannot establish the strongest live namespace theorem merely by repeating naked numeric identifiers.

This tranche deliberately leaves the existing legacy constructor available for compatibility.

It adds a stronger path:

```text
LocalQualifiedPhysicalAuthority(A)
        ↓ mint_generation
LocalQualifiedWorldGeneration(A,G)
        ↓ consume exactly once
LocalNamespacePhysicsAuthorityWorld(A,G)
        ↓ explicit downward projection
PhysicsAuthorityWorld(A,G)
```

The theorem is local capability provenance only.

## Exact claim

Inside one running process and through the production safe API introduced here:

1. fresh local physical-authority roots receive distinct nonzero `PhysicalAuthorityId` values from one checked atomic allocator;
2. one local root owns exactly one mutable generation allocator;
3. generation IDs are monotonic/non-reused within that root;
4. a `LocalQualifiedWorldGeneration` has no public detached constructor and is non-cloneable;
5. constructing a `LocalNamespacePhysicsAuthorityWorld` consumes that generation capability;
6. repeating naked `PhysicalAuthorityId` / `WorldGenerationId` values cannot recreate the local qualified wrapper type;
7. projection back to plain `PhysicsAuthorityWorld` is explicit and loses the stronger namespace-capability theorem.

## Why the legacy IDs remain public

`PhysicalAuthorityId`, `WorldGenerationId`, and `PhysicsBodySubject` remain useful serializable/reference data.

This tranche does not make data identifiers themselves secret or unconstructible. Instead it separates:

```text
identifier value
    !=
authority capability
```

The compatibility constructor:

```text
PhysicsAuthorityWorld::new(A, G, world)
```

therefore remains valid but weaker. It does not produce `LocalNamespacePhysicsAuthorityWorld`.

Future evidence-bearing APIs can migrate to require the qualified wrapper while mechanics/tests continue using the legacy surface during migration.

## Process-local boundary

The physical-authority root allocator is intentionally process-local.

It uses a checked atomic monotonic counter because the local theorem needs uniqueness/no-reuse among concurrent fresh roots in one process, not global synchronization semantics.

A process restart may restart that numeric allocator.

Therefore this tranche does **not** establish:

```text
same numeric A across processes
    => same qualified authority root
```

or:

```text
A minted in process P1
    != every A ever minted in process P2
```

Persistent/exported evidence requires PHYS-ID-04B / PHYS-EVID-03 style authenticated authority-root credentials and PHYS-EVID-05 session identity.

## Generation ownership

`LocalQualifiedPhysicalAuthority` is intentionally non-cloneable.

Its `mint_generation(&mut self)` path serializes mutation of the local generation allocator through Rust borrowing. Generation IDs use checked arithmetic and never wrap.

`LocalQualifiedWorldGeneration` is also non-cloneable and has no public constructor from `(A,G)`.

Thus ordinary safe code cannot:

```text
copy generation capability G
    -> bind world W1
    -> bind unrelated world W2
```

through the qualified path.

Multiple different generations under the same physical authority remain legitimate.

## Wrapper theorem

`LocalNamespacePhysicsAuthorityWorld` owns both:

```text
LocalQualifiedWorldGeneration
PhysicsAuthorityWorld
```

and creates the inner legacy wrapper from the capability's exact A/G values.

The qualified wrapper intentionally does not implement `Deref`; callers must use explicit accessors when they want the weaker legacy authority surface.

`into_unqualified()` consumes the wrapper and discards the stronger capability type. It does not return the generation capability for reuse.

## Structural integrity boundary

This tranche does not validate arbitrary incoming `PhysicsWorld` structural state.

`LocalNamespacePhysicsAuthorityWorld::bind` establishes only:

```text
this world object is bound to this consumed local namespace capability
```

It does not establish:

```text
all private body/index/topology invariants were valid before binding
```

Complete structural integrity and checked raw-world adoption remain #1005 / #1019.

A future strongest constructor should compose:

```text
qualified generation capability
+
checked structural adoption
    ↓
fully namespace-qualified + structurally-qualified PhysicsAuthorityWorld
```

## Temporal boundary

This tranche does not modify `AuthorityStepStamp` and does not establish temporal wrapper-incarnation uniqueness.

PHYS-OBS-04A / #1055 remains required for detached temporal lineage identity.

Generation qualification answers:

```text
who owns the A/G body-subject namespace?
```

Temporal incarnation answers:

```text
which live wrapper incarnation issued this step sequence?
```

Do not conflate them.

## Body identity composition

After PHYS-ID-02 / #1075 is implemented, the intended live identity composition is:

```text
qualified A root
+ qualified G ownership
+ generation-scoped NetId non-reuse
    ↓
PhysicsBodySubject(A,G,N)
```

which prevents one qualified live subject key from silently changing which body incarnation it names inside that architecture.

This tranche alone does not yet implement NetId tombstones/removal semantics.

## Persistence/authentication boundary

A local root capability is not a cryptographic credential.

Persistent verification must still prove which security/Xenia identity is authorized to speak for the physical-authority lineage A.

A signed payload containing numeric A is insufficient unless the signer-to-A authorization relationship is verified under an explicit credential/profile.

## Required executable corpus

At minimum:

- two fresh local roots mint distinct nonzero A values;
- one root mints distinct G values without reuse;
- generations minted under different roots remain distinct namespace pairs even when numeric generation allocation begins from the same local sequence;
- binding a generation capability constructs an inner `PhysicsAuthorityWorld` with exactly matching A/G;
- mutable downward projection preserves the same wrapper/capability ownership;
- `into_unqualified()` preserves inner A/G values but does not return a generation capability;
- legacy naked `(A,G)` construction remains source-compatible but does not produce the qualified wrapper type;
- static audit confirms no public constructor exists for `LocalQualifiedWorldGeneration`;
- static audit confirms neither local root nor generation capability derives/implements `Clone`, `Copy`, `Serialize`, or `Deserialize`;
- local root allocation and generation allocation use checked arithmetic and never wrap.

## Recommended qualification

```text
cargo fmt --all -- --check
cargo check -p symtropy-physics --locked --all-targets
cargo test -p symtropy-physics --locked --test local_authority_namespace_v01
cargo test -p symtropy-physics --locked --lib
cargo clippy -p symtropy-physics --locked --all-targets -- -D warnings
```

plus a source audit for the capability-construction and trait-surface requirements above.

## Successor path

```text
this tranche
    local authority root + generation capabilities
        ↓
#1005/#1019
    compose checked structural adoption
        ↓
#1075
    generation-scoped NetId non-reuse
        ↓
#1055
    temporal incarnation
        ↓
#1054/#1059
    opaque authority-issued observations
```

Persistent path additionally composes #1074/#1064/#1065 and PHYS-ID-04B credentials.

## Non-claims

No cross-process uniqueness, no cryptographic authentication, no global/network consensus, no persistent authority-root credential, no complete-world structural validity, no NetId tombstone theorem, no same-generation restore, no temporal-incarnation identity, no elapsed time, no observation correctness, no dwell/arrival/custody/delivery semantics, and no settlement authority.
