# Broadphase Architecture Tournament v0.1

A reproducible, CPU-only temporal AABB tournament for Symtropy's broadphase portfolio.

## Compared backends

- Brute-force pair enumeration.
- Sweep-and-prune with a per-frame axis selected from current-frame geometry only.
- Per-leaf incremental AABB tree query (the original one-query-per-proxy reference), balancing disabled.
- Self-pair incremental AABB tree query, balancing disabled.
- Per-leaf incremental AABB tree query, height-rotation balanced.
- Self-pair incremental AABB tree query, height-rotation balanced.
- Persistent pair-delta cache: keep the symmetric active-pair relation and emit additions, removals, and revalidation events for changed proxies.
- Indexed dirty-frontier pair-delta cache: preserve old adjacency for pair deaths, query the persistent incremental AABB tree for current neighbours, refine fat-tree hits against current AABBs, and emit additions/removals/revalidations.
- Symtropy Morton/LBVH AABB adapter: production Morton quantization, longest-common-prefix split construction, per-leaf traversal and the live 50-body brute-force fallback threshold.

The four tree variants use the same pinned `incremental-aabb-tree-reference` implementation from PR #7524. Each per-leaf/self-pair comparison uses the same balancing policy, tree mutation sequence, proxy identities, AABB population and temporal frames. This makes traversal strategy comparable without confounding it with a different topology.

## Frozen scene families

The deterministic generator includes sparse coherent motion, dense clusters, static-heavy scenes, high-churn populations, anisotropic/high-aspect bounds and teleport sequences. Each can run in 2D, 3D and 4D.

The seed, declared proxy count and frame count are explicit and included in every CSV row, so a result can be tied back to its exact deterministic sequence. Scene generation is outside the timed backend region. The same scene sequence is supplied to all nine backends. High-churn fixtures allocate a fresh monotonic ProxyId after a removal/reinsertion boundary; an identity remains stable only for one continuous residency.

The added sleeping-wakeup family keeps most proxies stationary and moves only two of 128 proxies per frame after initialization. This models a common engine regime where most bodies are asleep and only a sparse wake frontier changes.

## Measurement contract

1. The independent brute-force AABB pair oracle is computed before timed backend execution.
2. Full-set backends time their own frame work: brute-force candidate production; SAP axis selection, sorting and sweep; or persistent-tree insert/update/remove, query and current-envelope refinement.
3. Pair-delta backends time dirty-frontier discovery, affected-pair reevaluation and pair birth/death/revalidation event production. Their output contract is explicitly labelled `pair_delta_events`; they do not claim to regenerate the full set each frame.
4. The indexed backend uses the persistent incremental AABB tree to discover current neighbours. It reports tree node visits, leaf tests, returned frontier candidates, raw candidate enumeration, unique affected pairs, index mutations and reinsertion counts. The scan cache separately reports full-population proxy visits.
5. Pair-cache full-set materialization is timed and reported separately. Production consumers should normally keep their own persistent contact/pair state and apply the delta stream; a consumer that needs a full list must include the separately reported materialization cost.
6. The timer stops before independent candidate and event comparisons with the oracle. For pair-delta backends, emitted additions/removals must equal the exact set differences between adjacent brute-force oracle frames. Revalidated pairs must equal the intersection of adjacent oracle relations that touch a changed proxy. The active pair cache must also equal the complete current oracle set.
7. Every false positive, false negative, candidate mismatch, event mismatch and duplicate event remains in the result. The tests require exact candidate parity for all declared fixtures.
8. Tree leaves may remain conservative and larger after a fat-contained update. The benchmark filters raw tree hits against current-frame AABBs and reports stale hits removed.
9. p50/p95/p99 frame latency, pair-cache materialization latency, summed frame time, campaign wall time, pair tests, dirty-frontier size, additions/removals/revalidations, snapshot dirty-discovery entries, full-population scan visits, index traversal, frontier candidates, index maintenance, tree maintenance and node counts are reported.

The Morton/LBVH adapter rebuilds its hierarchy each frame and times construction plus per-leaf query. At fewer than 50 proxies it follows the production brute-force fallback. It operates on already-computed AABBs and omits RigidBody shape transformation, static/kinematic cache, and body collision filters, so it is an algorithm-level adapter rather than a full production integration.

The tree backend uses zero extra fat margin because the tournament inputs are already authoritative AABBs. Candidate parity is exact for current-frame AABB overlap; it is not a shape-level or continuous time-of-impact proof.

The pair-delta output is a different API contract from full candidate-set output. `added` and `removed` change pair membership. `revalidated` contains pairs that remain active but touch a dirty endpoint; consumers must refresh narrowphase/contact state for these pairs even though their membership did not change. Do not compare its event-generation p95 directly to full-set p95 without accounting for the consumer's persistent-pair integration and the separately reported materialization cost.

The scan and indexed backends currently use the safe snapshot-derived `step` path. This still costs O(N) to discover dirty IDs, even though the indexed path avoids the subsequent O(kN) scan of all current proxies. The strictly sorted authoritative-journal `step_with_dirty_ids` path is implemented and unit-tested for both caches, with independent `verify_dirty_proxy_ids` audits, but is not yet the measured tournament path. Therefore this indexed backend is an intermediate algorithm experiment, not yet proof of O(k log N + C) end-to-end frame work. A follow-on must bind the journal to world revisions and charge or separately measure its producer cost.

A full invalidation (for example, collision filter/layer policy change) forces the safe snapshot-derived path; the dirty-journal fast path rejects it until all affected relationships are revalidated. Empty frames are supported by the stateful cache, so removing the last proxy emits every remaining pair removal and a later insertion can reinitialize the relation. The tournament scene oracle still uses non-empty frames. A production integration must bind the journal to exact world revisions and a trusted source before treating it as authoritative.

## Run

    cargo fmt --manifest-path research/broadphase-tournament/v0.1/Cargo.toml -- --check
    cargo test --manifest-path research/broadphase-tournament/v0.1/Cargo.toml --locked
    cargo clippy --manifest-path research/broadphase-tournament/v0.1/Cargo.toml --locked --all-targets -- -D warnings

Run the CSV benchmark in release mode:

    cargo run --release --manifest-path research/broadphase-tournament/v0.1/Cargo.toml --bin broadphase-tournament -- 128 40 2

Arguments are proxy count, frame count and repetitions. Defaults are 128, 40 and 2. Output is CSV with one row per family, dimension and backend.

For initial local qualification, use a small smoke run before larger campaigns:

    cargo run --release --manifest-path research/broadphase-tournament/v0.1/Cargo.toml --bin broadphase-tournament -- 48 18 1

## Claim boundary

This is an internal reference tournament, not an external industry comparison. It includes an AABB adapter of Symtropy's production Morton/LBVH algorithm and an indexed pair-delta prototype over the benchmark incremental tree, but not the complete production RigidBody and static/kinematic-cache integration. The indexed cache still uses snapshot-derived dirty discovery in this campaign. GPU SAP/LBVH, Box2D, Jolt, MuJoCo Warp and Newton are not yet executed as matched backends. Neither pair-delta cache is integrated into the simulation engine's live sleep/contact lifecycle. The campaign does not time production motion-bound construction, narrowphase, CCD, actual contact generation, solver work or full simulation ticks. No SOTA or performance-superiority claim is made.

The p95 promotion target remains prospective: at least 20% lower total broadphase cost on two held-out temporal families with no more than 5% regression on the other declared families, with zero correctness failures and unchanged candidate/contact/solver-row denominators. Candidate-count reductions alone are not a speed win.

A later cross-engine campaign must preserve the exact scenario and AABB population export, map each engine's contact/filter/lifecycle semantics, run on matched hardware and distinguish different physical laws. The public NYU CCD / analytic-TOI corpus belongs in a separate shape-level continuous-collision lane, not in this AABB overlap tournament.
