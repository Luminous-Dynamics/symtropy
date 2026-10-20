# Functional-envelope oracle v0.3: spatial broad-phase and scaling protocol

**Status:** proposed implementation slice stacked on functional-envelope oracle v0.2. Benchmarks are diagnostic, not a claim of industry-leading performance, physical safety, accessibility or code compliance.

## Paradigm shift

A useful geometric oracle must scale without weakening independent verification. The v0.2 functional-envelope contract checks physical footprints separately from task envelopes, but a naïve all-pairs loop and per-check full scans grow quadratically on sparse scenes. v0.3 replaces candidate discovery—not the exact collision predicate—with a deterministic, balanced axis-aligned bounding-volume hierarchy (BVH).

- Build one immutable BVH for each alternative's physical footprints.
- Query the BVH for possible collisions and then confirm each candidate with the same exact rectangle-intersection predicate.
- Reuse the index for front/task-envelope clearance queries.
- Index requirements, item placements and envelope definitions by stable IDs, removing repeated full-catalog scans.
- Keep output diagnostics ordered by input index, not tree traversal order, so performance work does not affect reproducibility.

A BVH only prunes regions whose bounding box cannot intersect the query. It cannot assert a pass by itself; exact geometry remains the final predicate. Worst-case overlap-heavy layouts can still require quadratic work because reporting every colliding pair is itself quadratic.

## Diagnostic benchmark

Run:

\`\`\`sh
cargo fmt --manifest-path tools/furnishing-layout-audit/Cargo.toml -- --check
cargo test --locked --manifest-path tools/furnishing-layout-audit/Cargo.toml
cargo clippy --locked --manifest-path tools/furnishing-layout-audit/Cargo.toml --all-targets -- -D warnings
cargo bench --locked --manifest-path tools/furnishing-layout-audit/Cargo.toml --bench functional_envelope_scaling
\`\`\`

The benchmark generates sparse, synthetic grids with 25, 100, 1,000 and 10,000 items plus one mandatory functional-envelope check per item. It records p50/p95 for the complete Rust oracle on the already-constructed in-memory document. It does not measure memory, UI render time, room measurement accuracy, network/DHT work or a production solver.

No timing threshold is used in CI. The logs are evidence for a named exact commit and runner; a performance claim additionally needs repeated measurements on a stable reference machine, peak-memory data, correctness results and a matched baseline.

## Acceptance gates

- Zero differences between BVH collision candidates and brute-force rectangle overlap in deterministic tests.
- All v0.1 and v0.2 synthetic expected outcomes and adversarial mutations continue to pass.
- Unknown or malformed required envelope evidence still yields indeterminate/blocked; it can never become pass through indexing.
- Identical inputs give the same diagnostic ordering and decision results.
- Record the full 25/100/1,000/10,000 p50/p95 series and memory before setting an SLA.
- Zero false-safe outcomes in the frozen known-truth corpus remain a hard correctness gate; speed cannot compensate for a false-safe result.

## Claim boundary

This remains an axis-aligned 2D rectangle model with synthetic evidence. It does not qualify human task envelopes, door-swing arcs, wheelchair turning, reach biomechanics, building-code or accessibility compliance, site measurement, comfort or physical safety. The next model-fidelity work should add separately sourced and independently qualified task-specific models, not infer human dimensions from this benchmark.
