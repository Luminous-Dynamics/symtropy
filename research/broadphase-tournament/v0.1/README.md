# Broadphase Architecture Tournament v0.1

A reproducible, CPU-only temporal AABB tournament for Symtropy's broadphase portfolio.

## Compared backends

- Brute-force pair enumeration.
- Sweep-and-prune with a per-frame axis selected from current-frame geometry only.
- Per-leaf incremental AABB tree query (the original one-query-per-proxy reference), balancing disabled.
- Self-pair incremental AABB tree query, balancing disabled.
- Per-leaf incremental AABB tree query, height-rotation balanced.
- Self-pair incremental AABB tree query, height-rotation balanced.

The four tree variants use the same pinned `incremental-aabb-tree-reference` implementation from PR #7524. Each per-leaf/self-pair comparison uses the same balancing policy, tree mutation sequence, proxy identities, AABB population and temporal frames. This makes traversal strategy comparable without confounding it with a different topology.

## Frozen scene families

The deterministic generator includes sparse coherent motion, dense clusters, static-heavy scenes, high-churn populations, anisotropic/high-aspect bounds and teleport sequences. Each can run in 2D, 3D and 4D.

The seed, declared proxy count and frame count are explicit and included in every CSV row, so a result can be tied back to its exact deterministic sequence. Scene generation is outside the timed backend region. The same scene sequence is supplied to all six backends.

## Measurement contract

1. The independent brute-force AABB pair oracle is computed before timed backend execution.
2. Each backend times its own frame work: brute-force candidate production; SAP axis selection, sorting and sweep; or persistent-tree insert/update/remove, query and current-envelope refinement.
3. The timer stops before the backend candidate set is compared with the oracle.
4. Every false positive, false negative and mismatching frame remains in the result. The tests require exact candidate parity for all declared fixtures.
5. Tree leaves may remain conservative and larger after a fat-contained update. The benchmark filters raw tree hits against current-frame AABBs and reports stale hits removed.
6. p50/p95/p99 frame latency, summed frame time, campaign wall time, pair tests, tree traversal, proxy maintenance, tree height and node counts are reported.

The tree backend uses zero extra fat margin because the tournament inputs are already authoritative AABBs. Candidate parity is exact for current-frame AABB overlap; it is not a shape-level or continuous time-of-impact proof.

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

This is an internal reference tournament, not an external industry comparison. It does not yet include Symtropy's production LBVH/static-cache path, GPU SAP/LBVH, Box2D, Jolt, MuJoCo Warp or Newton. It does not time motion-bound construction, narrowphase, CCD, contact generation, the solver or full simulation ticks. No SOTA or performance-superiority claim is made.

The p95 promotion target remains prospective: at least 20% lower total broadphase cost on two held-out temporal families with no more than 5% regression on the other declared families, with zero correctness failures and unchanged candidate/contact/solver-row denominators. Candidate-count reductions alone are not a speed win.

A later cross-engine campaign must preserve the exact scenario and AABB population export, map each engine's contact/filter/lifecycle semantics, run on matched hardware and distinguish different physical laws. The public NYU CCD / analytic-TOI corpus belongs in a separate shape-level continuous-collision lane, not in this AABB overlap tournament.
