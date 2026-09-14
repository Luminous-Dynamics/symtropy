# PHYSICAL AUTHORITY ENDPOINT BOX OBSERVATION v0.1

Status: proposed product theorem; executable qualification required before PASS is claimed.

## Purpose

Establish a fail-closed **instantaneous endpoint-membership predicate** for one explicitly named live `PhysicsBodySubject` relative to one endpoint box anchored to a second validated physics subject.

This theorem is intentionally weaker than arrival, dwell, route compliance, custody transfer, or delivery.

## Governing law

```text
endpoint box spec
    + target PhysicsBodySubject
    + one PhysicsAuthorityWorld
        |
        +-> validate target subject
        +-> validate anchor subject
        +-> reject target == anchor
        +-> compute finite translated world-space box center
        +-> compute finite target-origin offset from center
        +-> evaluate existing HyperBox closed-boundary containment
        +-> capture handle-free target + anchor snapshots
        |
        `-> AuthorityEndpointBoxObservation
```

## Region specification

```text
AuthorityEndpointBoxSpec<D> {
    anchor: PhysicsBodySubject,
    center_offset: [f64; D],
    half_extents: [f64; D],
}
```

The stored implementation uses exact `f64::to_bits()` values after validation so equality is bitwise and deterministic.

`center_offset` is expressed in **world axes** relative to the anchor body's current translation:

```text
region_center = anchor.translation + center_offset
```

The anchor body's rotation is deliberately ignored in PHYS-OBS-03.

`half_extents` are also expressed in world axes and must be finite and strictly positive on every axis.

Signed zero in `center_offset` is canonicalized:

```text
-0.0 -> +0.0
```

so geometrically identical endpoint definitions are not split by signed-zero representation.

## Box authority

Containment reuses the checked-in `symtropy_math::HyperBox::contains_local` semantics rather than introducing a second box predicate.

For the finite target-origin offset:

```text
offset = target.translation - region_center
```

membership is equivalent to the closed-axis test:

```text
for every axis i:
    abs(offset[i]) <= half_extents[i]
```

Therefore a target origin exactly on any face/edge/corner boundary is **Inside**.

## Identity and handle boundary

Both target and anchor must independently pass `PhysicsAuthorityWorld::validate_subject` under the same immutable authority-world borrow.

The observation persists no runtime `BodyHandle`.

A successful observation carries:

```text
region: AuthorityEndpointBoxSpec<D>
subject: AuthorityBodySnapshot<D>
anchor: AuthorityBodySnapshot<D>
region_center: [u64; D]
offset_from_center: [u64; D]
membership: EndpointMembership
```

where body snapshots are authority/generation-bound and the derived coordinates are stored bitwise.

The implementation reuses `AuthorityBodySnapshot::from_validated` at crate-only visibility so the target and anchor are not independently rescanned after the validated proofs are already held.

## Same-subject rule

The target may not use itself as the endpoint anchor.

After both identities validate, exact `target == anchor` returns:

```text
SameSubject { subject }
```

This prevents a subject from manufacturing a trivial self-relative endpoint observation.

## Numeric fail-closed boundary

A valid region spec requires:

- every center-offset component finite;
- every half-extent finite;
- every half-extent strictly greater than zero.

Capture additionally rejects arithmetic that leaves the finite domain.

If:

```text
anchor.translation[i] + center_offset[i]
```

is non-finite, capture returns:

```text
NonFiniteRegionCenter { axis: i }
```

If:

```text
target.translation[i] - region_center[i]
```

is non-finite, capture returns:

```text
NonFiniteOffsetFromCenter { axis: i }
```

No membership value is returned in either case.

## Rotation boundary

PHYS-OBS-03 is **translation-anchored and world-axis-aligned**.

Anchor rotation does not affect membership.

This is deliberate, not an omission hidden in the implementation. `Rotor::from_matrix` currently accepts an arbitrary matrix while `is_proper_rotation` is only diagnostic. Treating `Transform::inverse()` as a trustworthy local-frame conversion would therefore silently assume a proper orthonormal rotor.

PHYS-OBS-03R / #1035 freezes the successor requirement: rotated/local-frame endpoint regions must first fail closed on non-finite, non-orthogonal, and reflection matrices under a qualified tolerance.

## Point-membership boundary

The target is represented by the **body transform origin only**.

This theorem does not establish that:

- the entire collider is contained by the endpoint box;
- any part of the collider overlaps the box;
- the body can physically fit through an opening;
- the body is collision-free inside the region.

A large collider may overlap the endpoint while its origin is Outside, and the checked-in adversarial corpus preserves that distinction.

Whole-shape containment/intersection should be a later theorem using the physics shape/support-map authority rather than silently changing this point predicate.

## Instantaneous-only boundary

`EndpointMembership::Inside` means only:

```text
the validated target origin is inside the exact endpoint box
at this observation
```

It does **not** establish:

- previous membership;
- continuous presence;
- first entry time;
- dwell time;
- hysteresis state;
- low velocity / stopped state;
- route compliance;
- movement history;
- arrival;
- delivery;
- custody transfer.

No later system should rename one instantaneous `Inside` result to `arrived = true` without an explicit temporal theorem.

## Hysteresis boundary

PHYS-OBS-03 intentionally does not add enter/exit hysteresis.

Hysteresis depends on previous classification state and therefore belongs to a temporal/state-machine successor. That successor should define exact enter and retain regions, transition semantics, and reset behavior rather than hiding history inside an instantaneous geometry function.

## Contract-binding boundary

The endpoint spec is an exact value object, not a durable `RegionId` or contract commitment.

A later logistics/economic contract may hash or otherwise bind the exact spec, but PHYS-OBS-03 alone does not establish:

- who authorized the endpoint;
- that the endpoint was agreed before transport began;
- that the endpoint was unchanged during a contract;
- that a marketplace/order/escrow references this exact region.

## Relationship to PHYS-OBS-01

PHYS-OBS-01 establishes finite current displacement/separation between two validated subjects.

PHYS-OBS-03 adds a typed spatial policy around one validated anchor, but remains instantaneous.

The safe progression is:

```text
PHYS-OBS-00  selected subject state
    -> PHYS-OBS-01  finite pair relation
    -> PHYS-OBS-03  instantaneous endpoint membership
    -> temporal ordered observations
    -> dwell/hysteresis theorem
    -> arrival theorem
    -> custody/delivery theorem
    -> ECON settlement eligibility
```

## Complete-world boundary

Like PHYS-OBS-00 and PHYS-OBS-01, this is selected-subject observation only.

Until PHYS-ID-01C / #1005 and PHYS-ID-01D / #1019 establish complete structural integrity and checked raw-world import, successful endpoint capture certifies only the selected target and anchor through the existing fail-closed validation path. It does not certify unrelated world structure.

## Collision boundary

Collision/contact/sensor evidence remains deferred to PHYS-OBS-02 / #1026 because those event buffers are currently ordinary downstream-mutable state.

Endpoint membership does not imply collision provenance and collision does not imply endpoint membership.

## Adversarial corpus

The checked-in tests must establish at least:

1. invalid/non-finite region parameters are rejected;
2. signed-zero center offsets canonicalize to positive zero;
3. equivalent endpoint observations compare equal despite different runtime handle allocation;
4. exact region-center and target-offset bits are captured;
5. closed-boundary membership includes exact faces/edges/corners;
6. a point just beyond the boundary is Outside;
7. anchor rotation does not affect membership;
8. target == anchor is rejected;
9. anchor identity failure remains attributed to the anchor;
10. translated region-center overflow fails closed;
11. target-offset overflow fails closed;
12. collider overlap does not get promoted into body-origin membership.

## Qualification target

Exact-head qualification should freeze source/test/contract hashes and run at least:

```text
cargo fmt --all -- --check
cargo check -p symtropy-physics --locked --all-targets
cargo test -p symtropy-physics --locked --test authority_endpoint_box_v01
cargo test -p symtropy-physics --locked --test authority_pair_observation_v01
cargo test -p symtropy-physics --locked --test authority_observation_v01
cargo test -p symtropy-physics --locked --test world_identity_authority_v01
cargo test -p symtropy-physics --locked --test replay_harness
cargo clippy -p symtropy-physics --locked --all-targets -- -D warnings
```

plus static audits that:

- endpoint observation types contain no public `BodyHandle` field;
- target and anchor resolve through `validate_subject`;
- `AuthorityBodySnapshot::from_validated` remains crate-only;
- containment calls `HyperBox::contains_local`;
- anchor rotation is not used for PHYS-OBS-03 membership;
- the contract explicitly denies dwell, arrival, delivery, collider containment, and route claims;
- the frozen product checkout is unchanged after qualification.

## ECON consequence

A qualified `Inside` observation can eventually become one **physical predicate input** to logistics or settlement logic.

It must not by itself release escrow, transfer title, mark cargo delivered, or establish route compliance.

## Non-claims

This theorem does not establish:

- complete-world structural integrity;
- checked raw-world import;
- persistence authenticity;
- cross-generation continuity;
- append-only history;
- historical ordering;
- continuous trajectory;
- dwell;
- hysteresis;
- route compliance;
- arrival;
- delivery;
- cargo custody continuity;
- whole-collider containment or overlap;
- oriented/local-frame endpoint validity;
- collision-event provenance;
- cryptographic provenance;
- distributed consensus.
