# PHYSICAL AUTHORITY SELECTED OBSERVATION v0.1

Status: proposed product theorem; executable qualification required before PASS is claimed.

## Purpose

Provide a handle-free bitwise observation of one explicitly named live physics subject without claiming a complete-world snapshot or historical replay.

The observed subject is:

```text
PhysicsBodySubject {
    PhysicalAuthorityId,
    WorldGenerationId,
    NetId,
}
```

`BodyHandle` is used only transiently inside subject validation and is not stored in the observation.

## Governing law

```text
PhysicsBodySubject
    -> PhysicsAuthorityWorld::validate_subject
    -> exact live body proof
    -> bitwise body-state capture
    -> AuthorityBodySnapshot
```

Never:

```text
claimed subject
    -> coincidentally equal BodyHandle
    -> durable-looking observation
```

## Snapshot contents

`AuthorityBodySnapshot<D>` captures:

```text
subject
body_type
translation bits
rotation matrix bits
linear velocity bits
angular velocity matrix bits
sleeping
sleep_counter
```

It intentionally does not contain a runtime `BodyHandle`.

## Failure semantics

Capture fails before returning a snapshot when subject validation fails, including:

- physical authority mismatch;
- world generation mismatch;
- unknown NetId;
- ambiguous live NetId;
- missing identity index;
- identity index pointing to the wrong handle;
- reverse identity mismatch.

There is no fallback to a runtime handle.

## Handle-allocation independence theorem

For two authority worlds that represent the same exact authority/generation-bound subject in equal captured physical state, the selected-subject snapshots may compare equal even when the target body has a different runtime `BodyHandle` in each world.

This theorem is intentionally narrower than cross-generation continuity. It requires the same `PhysicsBodySubject`, which includes the same `WorldGenerationId`.

## Relationship to runtime replay

`replay::BodySnapshot` remains valid for runtime-local deterministic replay and contains `BodyHandle`.

`AuthorityBodySnapshot` serves a different purpose: an explicitly selected authority-bound observation whose persisted-looking identity is the `PhysicsBodySubject`.

Neither type replaces the other.

## Complete-world boundary

This tranche is **selected-subject only**.

It does not prove that every live body in the wrapped raw world is structurally valid. `PhysicsAuthorityWorld::new` is still an infallible wrapper until PHYS-ID-01C / #1005 and PHYS-ID-01D / #1019 establish complete structural integrity and checked import.

Therefore:

```text
successful AuthorityBodySnapshot::capture(subject)
```

means only that the exact selected subject passed the existing fail-closed validation path and its current state was captured.

It does not certify unrelated bodies, constraints, caches, or indices elsewhere in the world.

## Historical boundary

This snapshot is a state observation, not an append-only historical record.

It carries no sequence number, simulation tick, predecessor commitment, persistence proof, or cross-generation continuity proof.

A later historical observation layer must define those semantics explicitly.

## Adversarial corpus

The checked-in test tranche must establish at least:

1. equal selected-subject snapshots despite different runtime handle allocation;
2. world-generation mismatch rejection;
3. physical-authority mismatch rejection;
4. ambiguous duplicate live NetId rejection.

Future structural/import work should add selected-subject regressions for each newly classified identity-integrity failure mode.

## Qualification target

Exact-head qualification should freeze the source/test/contract hashes and run:

```text
cargo test -p symtropy-physics --test authority_observation_v01
cargo test -p symtropy-physics --test physics_identity_authority_v01
cargo test -p symtropy-physics --test net_identity_mutation_v01
cargo test -p symtropy-physics --test replay_harness
cargo clippy -p symtropy-physics --all-targets -- -D warnings
```

plus static checks that:

- `AuthorityBodySnapshot` contains `PhysicsBodySubject`;
- it contains no `BodyHandle` field;
- capture calls `validate_subject`;
- no new complete-world, trajectory, route, arrival, or delivery claim appears in the contract.

## Non-claims

This theorem does not establish:

- complete `PhysicsWorld` structural integrity;
- checked raw-world import;
- persistence authenticity;
- stable cross-generation identity;
- append-only history;
- continuous trajectory;
- route compliance;
- arrival;
- delivery;
- cargo custody continuity;
- cryptographic provenance;
- distributed consensus.
