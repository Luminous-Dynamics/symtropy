# PHYSICAL AUTHORITY PAIR OBSERVATION v0.1

Status: proposed product theorem; executable qualification required before PASS is claimed.

## Purpose

Observe the current physical relation between two distinct, explicitly named `PhysicsBodySubject`s without persisting runtime `BodyHandle`s and without claiming movement history, route, arrival, or delivery.

## Governing law

```text
subject A + subject B
    -> reject A == B
    -> validate A in one PhysicsAuthorityWorld
    -> validate B in the same PhysicsAuthorityWorld
    -> hold both lifetime-bound validated proofs under one immutable world borrow
    -> derive finite current displacement B - A
    -> reject non-finite displacement or squared-separation overflow
    -> capture both handle-free body snapshots
    -> AuthorityPairSnapshot
```

The two participant validations are attributed independently so a caller can distinguish `SubjectA(error)` from `SubjectB(error)`.

## Snapshot contents

```text
AuthorityPairSnapshot<D> {
    body_a: AuthorityBodySnapshot<D>,
    body_b: AuthorityBodySnapshot<D>,
    relative_translation: [u64; D],
    separation_squared: u64,
}
```

The pair snapshot contains no runtime `BodyHandle` field. Each nested body snapshot names its participant with a `PhysicsBodySubject`.

`relative_translation` is the bitwise current vector:

```text
position(B) - position(A)
```

and `separation_squared` is the bitwise current squared Euclidean norm of that vector.

Both derived spatial values are emitted only when finite.

## Finite-geometry failure semantics

Identity-valid subjects are not enough to make a spatial relation meaningful. Pair capture fails closed when:

```text
any component of position(B) - position(A) is NaN or infinite
```

or when all displacement components are finite but:

```text
norm_squared(position(B) - position(A))
```

overflows or otherwise becomes non-finite.

The typed failures are:

```text
NonFiniteRelativeTranslation { axis }
NonFiniteSeparationSquared
```

The function does not publish a NaN/∞ distance and leave downstream policy to interpret it.

This does not retroactively make every field of `AuthorityBodySnapshot` finite. PHYS-OBS-00 is a raw bitwise selected-body state observation. PHYS-OBS-01 adds stricter finite semantics only for the derived spatial relation it claims.

## Coherence boundary

Capture holds an immutable borrow of the same `PhysicsAuthorityWorld` while both subjects are validated and observed. In ordinary safe Rust, a mutable borrow of that authority world cannot coexist with this capture.

This is a **current-state coherence theorem**, not a historical atomicity or distributed-consensus theorem. It makes no claim about external unsafe mutation, another process, persisted history, or observations before/after the function call.

## Identity failure semantics

Before a pair snapshot is returned:

- identical A/B subjects are rejected;
- subject A must pass the complete existing `validate_subject` path;
- subject B must pass the complete existing `validate_subject` path;
- participant-specific identity failures remain attributed to A or B;
- the derived spatial relation must be finite.

This inherits fail-closed rejection for physical-authority mismatch, world-generation mismatch, unknown NetId, ambiguous live NetId, missing identity index, identity index mismatch, and reverse identity mismatch.

There is no fallback to runtime handle equality.

## Handle-allocation independence

Equivalent pair observations may compare equal even when both target bodies have different runtime `BodyHandle` allocations in equivalent worlds, provided the two `PhysicsBodySubject`s and all captured physical values are equal.

This is not cross-generation continuity. The subject tuple includes `WorldGenerationId`; equal `NetId` under a different generation is a different subject.

## Spatial interpretation

This tranche establishes only exact finite current displacement/separation for two validated live subjects.

A downstream system may later define an explicit predicate such as:

```text
separation_squared <= threshold_squared
```

but the threshold, frame semantics, endpoint geometry, hysteresis, dwell time, and policy meaning belong to that later theorem.

In particular, this pair observation alone does **not** prove:

- that either subject traveled to its current location;
- a route;
- continuous presence;
- arrival;
- delivery;
- custody transfer.

## Relationship to collision observations

Do not derive authoritative collision evidence merely by wrapping the current public mutable `PhysicsWorld::collision_events` vector.

PHYS-OBS-02 / #1026 freezes the prerequisite that collision/sensor/contact event sources must first become world-owned/non-injectable through the public safe API.

## Complete-world boundary

Like PHYS-OBS-00 / #1023, this is selected-subject observation only.

Until PHYS-ID-01C / #1005 and PHYS-ID-01D / #1019 establish complete structural integrity and checked raw-world import, successful pair capture certifies only the two selected subjects through the existing fail-closed validation path. It does not certify unrelated bodies, constraints, caches, or maps elsewhere in the wrapped world.

## Adversarial corpus

The checked-in tests must establish at least:

1. equivalent pair snapshots despite different runtime handle allocation for both participants;
2. exact displacement and squared separation;
3. same-subject rejection;
4. participant-B generation mismatch attributed to B;
5. unknown participant-A NetId attributed to A;
6. NaN/non-finite displacement rejection with exact axis attribution;
7. finite displacement whose squared norm overflows is rejected.

Future identity-integrity work should add pair regressions for newly classified structural failure modes.

## Qualification target

Exact-head qualification should freeze the source/test/contract hashes and run:

```text
cargo fmt --all -- --check
cargo check -p symtropy-physics --locked --all-targets
cargo test -p symtropy-physics --locked --test authority_pair_observation_v01
cargo test -p symtropy-physics --locked --test authority_observation_v01
cargo test -p symtropy-physics --locked --test world_identity_authority_v01
cargo test -p symtropy-physics --locked --test replay_harness
cargo clippy -p symtropy-physics --locked --all-targets -- -D warnings
```

plus static checks that:

- `AuthorityPairSnapshot` contains no public `BodyHandle` field;
- both subjects are validated through `validate_subject`;
- same-subject input is rejected;
- non-finite derived spatial values fail closed;
- the contract contains explicit no-arrival/no-delivery/no-route claims.

## Non-claims

This theorem does not establish:

- complete-world structural integrity;
- checked raw-world import;
- persistence authenticity;
- cross-generation continuity;
- append-only history;
- historical ordering;
- continuous trajectory;
- route compliance;
- arrival;
- delivery;
- cargo custody continuity;
- collision-event provenance;
- cryptographic provenance;
- distributed consensus.
