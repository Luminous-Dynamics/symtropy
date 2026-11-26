# Tree-Backed Swept Pair Qualification v0.1

A reproducible cross-check between the balanced/unbalanced incremental AABB tree reference and the swept-pair envelope oracle.

## What it does

- Consumes generic or semi-implicit-Euler motion receipts through the swept-pair oracle's public proxy type.
- Validates all motion receipts through the O(n²) reference before allowing tree construction.
- Inserts supported conservative envelope AABBs into the incremental tree with **zero additional fat margin**.
- Uses a monotonic, collision-free proxy-ID registry keyed by the complete collider identity (slot, generation, collider ID and subshape path). Retired identities remain allocated; IDs are not recycled.
- Queries tree pairs, applies same-body and symmetric group/mask filters, and adds all unsupported-motion fallback pairs.
- Requires the final tree candidate population to exactly match the O(n²) envelope-pair oracle.
- Captures tree revision, balance policy, registry snapshot, tree statistics, query statistics, fallback pairs and the independent reference receipt.
- Replays the tree build/query and rejects any mismatch.

## Two qualification paths

### One-shot rebuild reference

`build_tree_swept_pair_set` reconstructs a deterministic tree per call from the current supported proxy population. It is useful for checking tree policy parity and replaying a single frame from the complete proxy population and registry snapshot.

### Stateful incremental reference

`TreeBackedBroadphase` preserves the tree and monotonic identity registry across calls. Each `step` applies only the required insert/update/remove operations:

- New supported identities are inserted.
- Changed envelopes are updated; tree-contained changes avoid reinsertion when the prior conservative proxy still contains the new envelope.
- Removed or newly unsupported identities are removed from the tree.
- Unsupported eligible pairs are restored as explicit fail-closed fallback candidates.
- Registry IDs are tombstoned and reused only for the same complete collider identity; a new generation receives a new ID.
- Per-frame maintenance deltas distinguish inserts, removals, updates, fat-contained updates and proxy reinsertions.

Each frame receipt retains the full canonical frame history so a verifier can reconstruct the exact tree operation sequence and compare candidate sets, registry state, tree/query counters and maintenance deltas. A SHA-256 commitment covers the ordered frame count, dimensionality, stable identities, collision filters, motion-evidence variant, source context/digests, exact floating-point bit patterns, bounds, displacement intervals, integrator state and guard values. The verifier checks the commitment before replay, so changing history while retaining the receipt's commitment is rejected.

The commitment is tamper-evident, not a digital signature. A party able to rewrite both the frame history and commitment can produce a new internally consistent receipt; authenticity requires an external signature or independently trusted root. Full history storage grows with replay length, so this is a research assurance receipt rather than a production persistence format.

## Important scope

Both paths retain the O(n²) swept-pair oracle as an independent candidate reference. The stateful API therefore qualifies incremental lifecycle behavior and tree/oracle parity, but it is **not yet a performance path**. Full-history receipts are not a production storage design. The next step is to separate qualification recording from the measured runtime path, then run held-out temporal benchmark campaigns with identical candidate/contact denominators.

## Tree implementation

The branch includes the balanced-tree reference source snapshot from exact PR #7524 commit `fed9ba265485d495d9f6191f280ff826d255e648` at its canonical repository path `research/incremental-aabb-tree-reference/v0.1`. This allows the adapter to use an ordinary local Cargo path dependency. When #7524 lands, the same path and content should be shared rather than duplicated.

- Source revision: `fed9ba265485d495d9f6191f280ff826d255e648`
- Unbalanced policy: `BalancePolicy::Disabled`
- Balanced policy: `BalancePolicy::HeightRotations`

## Run

    cargo fmt --manifest-path research/tree-backed-swept-pairs/v0.1/Cargo.toml -- --check
    cargo test --manifest-path research/tree-backed-swept-pairs/v0.1/Cargo.toml --locked
    cargo clippy --manifest-path research/tree-backed-swept-pairs/v0.1/Cargo.toml --locked --all-targets -- -D warnings

## Qualification targets

- Zero tree candidate false negatives relative to the O(n²) envelope-pair oracle.
- Stable identity-to-proxy mapping across calls when the caller retains the registry.
- Exact replay of candidate sets, tree/query counters and registry state.
- Identical candidate populations under balanced and unbalanced tree policies.
- Unsupported motion remains a fail-closed fallback.
- Generated 2D/3D/4D scene parity.

No benchmark or speedup claim is made. The next performance study must isolate the O(n²) oracle from the measured tree path and compare end-to-end broadphase time, query work, rebuild/update cost, memory, candidate inflation and fallback rate on held-out temporal sequences.
