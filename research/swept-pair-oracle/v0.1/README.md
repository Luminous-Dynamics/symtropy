# Swept Pair Oracle v0.1

A dependency-light Rust reference for qualifying swept broadphase pair completeness.

This crate composes with the directional-motion-envelopes crate and intentionally uses an O(n²) pair enumeration. It is a **qualification oracle**, not a production broadphase or performance implementation.

## Contract

A pair can be omitted only when both endpoint motion receipts validate against their exact stable collider identities and source inputs, and the two outward-rounded conservative envelopes are strictly disjoint.

Touching envelope boundaries remain candidates. If either endpoint has unsupported or unknown motion, the pair is retained as a fallback. Collision group/mask filtering follows the symmetric rule used by Symtropy. Same-body collider siblings are excluded.

The replay receipt preserves the complete canonically sorted proxy population, candidate set, fallback set, omission witnesses, and denominator counts. Verification recomputes the whole result. This is replayability, not cryptographic authenticity.

## Independent temporal oracle

For constant-translation AABBs, the reference oracle solves the common-time intersection of the two per-axis overlap inequalities over t in [0, 1]. It intersects the corresponding linear time intervals and does not reuse the envelope-overlap algorithm.

Generated 2D, 3D, and 4D scenes compare the candidate set with this independent oracle. Candidate inflation is expected because independently swept AABB envelopes can overlap even when the two moving boxes never overlap at the same time.

## Run

    cargo fmt --manifest-path research/swept-pair-oracle/v0.1/Cargo.toml -- --check
    cargo test --manifest-path research/swept-pair-oracle/v0.1/Cargo.toml --locked
    cargo clippy --manifest-path research/swept-pair-oracle/v0.1/Cargo.toml --locked --all-targets -- -D warnings

## Claim boundary

The tests qualify the stated linearly translating AABB model and the receipt/candidate logic. They do not prove upstream motion-state producers, rotational or accelerated trajectories, a general CCD solver, broadphase tree performance, GPU scalability, or production superiority. Unsupported trajectories are explicitly retained rather than approximated as stationary.

## Next qualification gates

- Add bounded-acceleration and rotation-aware motion evidence before widening the declared motion domain.
- Connect the reference to the incremental AABB tree and an independent shape-level temporal oracle.
- Compare against brute force, sweep-and-prune, dynamic BVH, and the existing endpoint pipeline on held-out temporal scenes.
- Preserve pair recall, candidate inflation, fallback rate, p50/p95/p99 update/query time, rebuild work, memory, and full-step cost.
