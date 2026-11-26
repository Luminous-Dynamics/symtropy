# Broadphase Architecture Tournament v0.1

A reproducible, CPU-only temporal AABB tournament for Symtropy's broadphase portfolio.

## Compared backends

- Brute-force pair enumeration.
- Sweep-and-prune with a per-frame axis selected from current-frame geometry only.
- Per-leaf incremental AABB tree query (the original one-query-per-proxy reference), balancing disabled.
- Self-pair incremental AABB tree query, balancing disabled.
- Per-leaf incremental AABB tree query, height-rotation balanced.
- Self-pair incremental AABB tree query, height-rotation balanced.
- Persistent pair-delta cache: keep the symmetric active-pair relation and emit only additions/removals for changed proxies.

The four tree variants use the same pinned `incremental-aabb-tree-reference` implementation from PR #7524. Each per-leaf/self-pair comparison uses the same balancing policy, tree mutation sequence, proxy identities, AABB population and temporal frames. This makes traversal strategy comparable without confounding it with a different topology.

## Frozen scene families

The deterministic generator includes sparse coherent motion, dense clusters, static-heavy scenes, high-churn populations, anisotropic/high-aspect bounds and teleport sequences. Each can run in 2D, 3D and 4D.

The seed, declared proxy count and frame count are explicit and included in every CSV row, so a result can be tied back to its exact deterministic sequence. Scene generation is outside the timed backend region. The same scene sequence is supplied to all seven backends.

The added sleeping-wakeup family keeps most proxies stationary and moves only two of 128 proxies per frame after initialization. This models a common engine regime where most bodies are asleep and only a sparse wake frontier changes.

## Measurement contract

1. The independent brute-force AABB pair oracle is computed before timed backend execution.
2. Full-set backends time their own frame work: brute-force candidate production; SAP axis selection, sorting and sweep; or persistent-tree insert/update/remove, query and current-envelope refinement.
3. The pair-delta backend times only dirty-frontier discovery, affected-pair reevaluation and pair birth/death event production. Its output contract is explicitly labelled `pair_delta_events`; it does not claim to regenerate the full set each frame.
4. Pair-cache full-set materialization is timed and reported separately. Production consumers should normally keep their own persistent contact/pair state and apply the delta stream; a consumer that needs a full list must include the separately reported materialization cost.
5. The timer stops before independent candidate and event comparisons with the oracle. For the pair-delta backend, emitted additions/removals must equal the exact set differences between adjacent brute-force oracle frames. The active pair cache must also equal the complete current oracle set.
6. Every false positive, false negative, candidate mismatch, event mismatch and duplicate event remains in the result. The tests require exact candidate parity for all declared fixtures.
7. Tree leaves may remain conservative and larger after a fat-contained update. The benchmark filters raw tree hits against current-frame AABBs and reports stale hits removed.
8. p50/p95/p99 frame latency, pair-cache materialization latency, summed frame time, campaign wall time, pair tests, dirty-frontier size, pair events, tree traversal, proxy maintenance, tree height and node counts are reported.

The tree backend uses zero extra fat margin because the tournament inputs are already authoritative AABBs. Candidate parity is exact for current-frame AABB overlap; it is not a shape-level or continuous time-of-impact proof.

The pair-delta output is a different API contract from full candidate-set output. Do not compare its event-generation p95 directly to full-set p95 without accounting for the consumer's persistent-pair integration and the separately reported materialization cost.

The cache exposes two input paths:
- `step` safely derives changed identities by diffing the prior/current AABB snapshots. This is the path used by the current tournament row.
- `step_with_dirty_ids` accepts a strictly sorted authoritative change journal from a physics world. The caller is responsible for completeness; `verify_dirty_proxy_ids` independently checks it against the snapshots in the audit path. This fast path is implemented and unit-tested, but it is not yet the measured tournament backend.

A full invalidation (for example, collision filter/layer policy change) forces the safe snapshot-derived path; the dirty-journal fast path rejects it until all affected relationships are revalidated. A production integration must bind the journal to exact world revisions and a trusted source before treating it as authoritative.

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

This is an internal reference tournament, not an external industry comparison. It does not yet include Symtropy's production LBVH/static-cache path, GPU SAP/LBVH, Box2D, Jolt, MuJoCo Warp or Newton. The pair-delta cache is an experimental CPU reference and is not yet integrated into the simulation engine's sleep/contact lifecycle. It does not time motion-bound construction, narrowphase, CCD, contact generation, the solver or full simulation ticks. No SOTA or performance-superiority claim is made.

The p95 promotion target remains prospective: at least 20% lower total broadphase cost on two held-out temporal families with no more than 5% regression on the other declared families, with zero correctness failures and unchanged candidate/contact/solver-row denominators. Candidate-count reductions alone are not a speed win.

A later cross-engine campaign must preserve the exact scenario and AABB population export, map each engine's contact/filter/lifecycle semantics, run on matched hardware and distinguish different physical laws. The public NYU CCD / analytic-TOI corpus belongs in a separate shape-level continuous-collision lane, not in this AABB overlap tournament.
