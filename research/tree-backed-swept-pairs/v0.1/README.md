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

## Important scope

This is a **rebuild-based qualification adapter**, not a stateful incremental broadphase. It rebuilds a tree per call so insertion order and topology can be replayed from the complete current proxy population and stable registry snapshot. The registry itself can be retained across frames. A later increment should keep the tree alive across frames and qualify insert/update/remove lifecycle operations before claiming incremental runtime benefits.

The O(n²) reference is intentionally retained as an independent candidate oracle. Consequently, this crate is not yet a performance path. It qualifies that the tree's supported-supported query output is complete relative to the oracle and that unsupported pairs are never silently lost.

## Tree implementation

The tree is pinned to the exact balanced-reference PR #7524 commit:

- Revision: `fed9ba265485d495d9f6191f280ff826d255e648`
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
