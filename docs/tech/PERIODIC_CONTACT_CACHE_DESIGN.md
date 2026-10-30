# Periodic Contact Cache and Image-Aware Lever Arms

**Status:** adapter prototype; exact-head compilation/tests required.  
**Implementation:** draft PR #2973.  
**Integration gate:** PHYS-TOPOLOGY-002 #2893.

## Why periodic contacts need a different key

The Euclidean warm-start cache is keyed by (body_a, body_b). On a periodic domain, one pair of bodies can have different image contacts. A cached impulse from image n = [-1, 0, ...] must not be reused for image n = [0, 0, ...], even if their contact witnesses happen to be spatially close in a particular unwrapped coordinate chart.

The periodic identity is the canonical tuple:

(min(BodyHandle A, BodyHandle B), max(BodyHandle A, BodyHandle B), canonical image offset).

If the caller reverses the body order, it must reverse the relative lattice offset. Negating an i64::MIN component fails closed.

## Canonical contact point

The input image offset translates B relative to A:

x_B_image = x_B + n * L.

When body_a < body_b, the contact witness is already in the canonical lower-ID frame. When the input order is reversed, the same physical pair is represented after translating both bodies by -n * L; therefore the contact point is also shifted by -n * L.

The cache stores and compares contact points only in this canonical frame. This prevents a contact from being spuriously unmatched because the narrow phase selected the opposite body ordering.

## Lever arms must use the selected image

For the actual query ordering:

- r_A = p_contact - x_A
- r_B = p_contact - (x_B + n * L)

Using p_contact - x_B for body B is wrong whenever the selected image offset is nonzero; it adds a one-or-more-period error to the torque lever arm and therefore changes angular effective mass, friction response, and impulse application.

The helper returns the actual-order lever arms and canonical identity/point separately. A later PhysicsWorld adapter must pass these lever arms into effective-mass and impulse calculations rather than rebuilding them from the original body centres.

## Tangential impulse orientation

The cache stores tangential impulse as a world-space vector, expressed as the impulse applied to the canonical higher-ID body. The scalar tangent impulse used by a Euclidean cache is tied to a chosen tangent basis and contact ordering; it is not safe to reuse blindly after swapping A and B.

On storage:
- if input A is the lower ID, the tangent impulse on input B is already the impulse on canonical higher-ID body;
- if input A is the higher ID, input B is the lower ID, so the vector sign is reversed to express the impulse on canonical higher-ID body.

The normal impulse remains a non-negative scalar magnitude and is applied along the current canonical contact normal by the solver. The cache does not infer or reconstruct a tangent basis.

## Frame lifecycle

PeriodicContactCache owns two maps:

- current-frame write set, populated by store;
- previous-frame lookup set, read by lookup.

Call begin_frame() once before processing contacts for a new frame. It promotes current writes to previous lookup state and starts an empty current write set. lookup never reads current-frame writes. If a body pair/image does not appear in one completed frame, the next begin_frame expires its stale warm-start entries.

This follows the two-phase warm-start pattern without changing the existing Euclidean ContactCache.

## Independent tests

The adapter tests check:
- two lattice images for the same body pair retain independent cached impulses;
- storing with reversed body order and offset, then querying in canonical order, returns the same canonical point and tangent-vector orientation;
- lever arms are swapped correctly under equivalent reversed image representation and are computed against the translated B centre;
- non-periodic axes reject nonzero image offsets;
- non-finite points/centres/impulses/distances, negative normal impulses and invalid match thresholds fail closed;
- previous-frame lookup works after a frame rotation, current writes are not visible to same-frame lookup, and missed contacts expire after an empty frame.

These are source-level test assertions, not yet passing results.

## Integration requirements before production use

1. Periodic broadphase and narrow phase provide the exact image offset per contact.
2. Manifold generation runs with B's image transform, and contact anchors/points retain image identity.
3. The TGS/effective-mass calculation consumes the helper's image-aware r_A and r_B.
4. The warm-start store/lookup follows previous/current frame lifecycle and the canonical image key.
5. The solver converts the cached world-space tangential impulse vector into the current contact tangent plane with a bounded projection/residual check.
6. Each physical body-pair/image contact receives at most one impulse per solve pass, with deterministic handling of simultaneous image contacts.
7. The no-periodic-domain path stays on the existing Euclidean cache and has a measured regression baseline.

Do not integrate an image-aware cache while leaving torque/effective-mass calculations on the unshifted centres. That partial integration would retain the most damaging seam error.
