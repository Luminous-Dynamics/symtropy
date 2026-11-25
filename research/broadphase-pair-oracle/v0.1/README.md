# Broadphase Pair Oracle v0.1

Backend-neutral scorecard for broadphase pair-set completeness and workload performance.

## Contract

The broadphase candidate set is evaluated against an independent brute-force oracle over the same frozen collider identities, conservative AABB snapshot, and collision filter. The receipt must explicitly declare complete oracle coverage; partial or unknown oracle coverage fails closed. The workload collider count must exactly equal the frozen collider identity list. This crate scores pair sets; it does not compute AABB overlap or prove that the supplied oracle is complete.

Pair identity includes body slot, body generation, collider ID and compound subshape path. Endpoint order is canonicalized. Slot reuse with a new generation is a distinct collider identity. Duplicate colliders, duplicate pair rows, self-pairs and unknown endpoints fail closed.

## Denominators

- Possible unfiltered pairs = N * (N - 1) / 2.
- Eligible pairs after filtering are supplied by the frozen collision-filter pass and may be smaller than the unfiltered population.
- Oracle overlapping pairs are the brute-force AABB-overlap subset of eligible pairs.
- Candidate precision, recall and false-positive rate use these pair identities.
- An empty oracle-overlap population yields unavailable recall, not a fabricated 100%.

This population is separate from NYU edge-edge/vertex-face analytic queries and from actual narrowphase contacts.

## Per-frame evidence

Each scorecard binds world revision/step, SHA-256 of the frozen AABB snapshot, SHA-256 of collision-filter policy, brute-force oracle ID/version, collider population, and disjoint static/awake/sleeping object classes.

WorkloadFrame records moved, added and removed colliders; update, rebuild and pair-generation time; full-step time; node/leaf visits; reinserts; pair-cache changes; and peak memory. These allow comparison of DBVT, SAP, LBVH, layered trees and brute force on the same workload trace.

Receipt metadata is declarative: the harness must actually freeze/hash the AABBs and filter and execute the oracle independently. This crate does not authenticate caller honesty or prove the oracle implementation.

## Industry comparison targets

- Box2D: dynamic binary AABB tree and query node/leaf visits.
- Jolt: static/non-moving and moving layers; concurrent ancestor expansion and background tight-tree rebuild.
- Bullet: dynamic/fixed DBVT trees and persistent pair cache; include SAP.
- MuJoCo: covariance-principal-axis SAP plus per-body static AABB BVH midphase.
- PhysX: SAP, MBP, ABP, PABP and GPU broadphase.
- NYU Scalable CCD: coordinate-axis swept broadphase on linearized motion.

Compare pair recall first (must be 100%), then candidate precision, update/rebuild time, pair-generation time, total p50/p95/p99, node visits, reinserts, cache churn, memory and concurrency scaling.

## Run

cargo fmt --manifest-path research/broadphase-pair-oracle/v0.1/Cargo.toml -- --check
cargo test --manifest-path research/broadphase-pair-oracle/v0.1/Cargo.toml --locked
cargo clippy --manifest-path research/broadphase-pair-oracle/v0.1/Cargo.toml --locked --all-targets -- -D warnings

## Promotion targets (aspirational)

- 100% recall against the independently generated brute-force AABB-overlap oracle.
- Zero duplicate, orphan or stale pair identities across frozen frames.
- At least 20% p95 end-to-end improvement on two held-out workload families with no more than 5% p95 regression on every declared family, at equal 100% recall.
- Report all per-frame costs and static/dynamic/churn strata; no speed claims from tree height alone.
