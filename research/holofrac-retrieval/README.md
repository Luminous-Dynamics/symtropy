# HoloFrac Spatial Retrieval Lab

Standalone Rust benchmark for the first executable HoloFrac slice: typed, directed architectural-graph retrieval using the existing `symtropy-hdc-core` crate.

## Run

From the repository root:

```bash
HOLOFRAC_SOURCE_SHA="$(git rev-parse HEAD)" \
  cargo test --manifest-path research/holofrac-retrieval/Cargo.toml

cargo clippy --manifest-path research/holofrac-retrieval/Cargo.toml \
  --all-targets -- -D warnings

HOLOFRAC_SOURCE_SHA="$(git rev-parse HEAD)" \
  cargo run --release --manifest-path research/holofrac-retrieval/Cargo.toml

HOLOFRAC_SOURCE_SHA="$(git rev-parse HEAD)" \
  cargo run --release --manifest-path research/holofrac-retrieval/Cargo.toml \
  --bin factorization_sweep

HOLOFRAC_SOURCE_SHA="$(git rev-parse HEAD)" \
  cargo run --release --manifest-path research/holofrac-retrieval/Cargo.toml \
  --bin field_sweep

HOLOFRAC_SOURCE_SHA="$(git rev-parse HEAD)" \
  cargo run --release --manifest-path research/holofrac-retrieval/Cargo.toml \
  --bin impact_sweep

HOLOFRAC_SOURCE_SHA="$(git rev-parse HEAD)" \
  cargo run --release --manifest-path research/holofrac-retrieval/Cargo.toml \
  --bin multichannel_sweep

HOLOFRAC_SOURCE_SHA="$(git rev-parse HEAD)" \
  cargo run --release --manifest-path research/holofrac-retrieval/Cargo.toml \
  --bin fractal_sweep

HOLOFRAC_SOURCE_SHA="$(git rev-parse HEAD)" \
  cargo run --release --manifest-path research/holofrac-retrieval/Cargo.toml \
  --bin temporal_fixture_sweep

HOLOFRAC_SOURCE_SHA="$(git rev-parse HEAD)" \
  cargo run --release --manifest-path research/holofrac-retrieval/Cargo.toml \
  --bin sensor_placement_sweep

HOLOFRAC_SOURCE_SHA="$(git rev-parse HEAD)" \
  cargo run --release --manifest-path research/holofrac-retrieval/Cargo.toml \
  --bin lifecycle_sweep

HOLOFRAC_SOURCE_SHA="$(git rev-parse HEAD)" \
  cargo run --release --manifest-path research/holofrac-retrieval/Cargo.toml \
  --bin lifecycle_impact_sweep

HOLOFRAC_SOURCE_SHA="$(git rev-parse HEAD)" \
  cargo run --release --manifest-path research/holofrac-retrieval/Cargo.toml \
  --bin pluralistic_decision_sweep
```

The primary CLI emits a CSV summary for 2K/4K/8K/16K dimensions, per-query exact/reversed controls and four partial-context workloads per dimension. The factorization sweep varies graph size (100/1,000/5,000 facts) and entity reuse (high/medium/low) at 1K dimension, reporting unique codebook cardinality, factorized versus duplicated role-vector bytes, compact-reference bytes and build time. The continuous-field sweep tests five analytic families—smooth, high-frequency, discontinuous, anisotropic and localized. Smooth uses 2K/4K dimensions and 4×4/8×8/16×16 training grids; the other families use a bounded 2K, 4×4/8×8 sweep. Queries are held-out cell centres. It reports HDC, nearest-neighbour and bilinear MAE/RMSE/R², plus coordinate encoding, unbinding, cleanup, p50/p95 latency and payload estimates. It also compares HDC vectors plus encoder index arrays against scalar-grid value storage and full point records, exposing the memory cost of the 257-level cleanup codebook. Samples whose distinct coordinates collapse to the same encoded x/y level are rejected. The exact evidence-impact sweep measures DAG closure on 100/1,000/10,000/100,000-artifact independent-chain graphs, plus shared-site-input DAGs with 20/200/2,000/20,000 projects under either a common-cause mutation or a local project mutation. The multi-channel sweep uses 2/4/6 channels and 4×4/8×8 grids, comparing shared position+channel+value memory against independent per-channel memories and scalar-grid storage. The geometric scale sweep compares 65,536-point Sierpiński, regular-grid, uniform-random and line controls across resolutions 4–256, with a predeclared fit window of 16–128 (coarser scales 4 and 8 are retained in the full curve but excluded from the slope fit) and explicit fit/window/translation diagnostics. The component lifecycle sweep publishes bitemporal states for a synthetic component across install, inspection, repair, removal, recovery, reuse and recycling, with mass-balance accounting and a deliberately late-recorded final recycling event. It is a schema/mass-accounting fixture, not a real material passport, EPD, deconstruction or circularity result. The lifecycle evidence-impact sweep mutates component condition, interface, material passport and functional use, then checks exact stale/retained evidence sets against a frozen oracle; incomplete inventory forces full conservative invalidation. The pluralistic decision sweep uses a deterministic synthetic multi-stakeholder corpus, revisioned bitemporal preference/consent events and explicit hard vetoes. It reports the Pareto frontier, mean, maximin and leximin rankings separately; masks anonymous identities; excludes local-only, unconsented and withdrawn profiles; and tests future-effective preferences and late-recorded withdrawals. The temporal fixture sweep publishes deterministic six-channel bitemporal event streams with irregular sample times, missing events, delayed arrivals, corrections and historical decision snapshots, keeping evaluator-only truth separate. The sensor-placement sweep compares regular-grid and greedy-maximin layouts with ten predeclared random-placement seeds at matched sensor counts, using a simple nearest-sensor virtual-sensing baseline over the same analytic fields. Raw rows preserve each random seed so variability is visible. These last two are synthetic fixtures, not measured-building results. CI now uploads twelve raw CSVs as a 14-day workflow artifact, alongside runner/toolchain details and the exact checked-out source SHA.

## What is implemented

- Typed spatial scales and directed relation kinds.
- Validation for duplicate IDs/facts, empty identifiers, self-relations, unknown endpoints and broader-to-narrower `contains` direction.
- Stable canonical fact keys and exact graph lookup oracle.
- Deterministic role-separated subject, subject-scale, predicate, object and object-scale encoding through `ItemMemory` and bipolar binding/bundling.
- Explicit directionality negative controls using reversed edges.
- Prebuilt semantic index: candidate encodings and canonical keys are computed once; the benchmark verifies the source digest before timing the frozen query batch.
- Separate timings for index construction, query encoding, bipolar search, f32-view search, exact graph oracle and canonical metadata lookup.
- Encoder-contract fingerprint, full encoder-spec compatibility check, exact canonical source-snapshot check and separate non-security source-graph digest.
- An f32 materialization of the same bipolar code to compare storage and similarity cost, not a second encoder family.
- Positive and reversed-edge retrieval metrics at four dimensions.
- Four partial-context CLI queries per dimension: subject+relation, relation+object, subject+object and a no-answer control.
- Separate role-aligned and holistic partial-vector ranking metrics, with exact-oracle verified results and abstention.
- Partial-context query module with role-aligned slot similarity and holistic partial-bundle ranking.
- Factorized role/value codebook: one bipolar vector per unique typed role/value pair, shared across fact records. Each fact stores five compact codebook indices, and partial retrieval resolves slot vectors through those indices without per-candidate string-key lookups.
- Exact source-graph oracle plus a codebook-keyed exact role-postings index; posting intersection must agree exactly with the full graph scan or fail closed.
- Exact-oracle verification, false-suggestion accounting and abstention when no exact facts match.
- Precision@k, recall@k, reciprocal rank and separate timing measures for exact postings, role-aligned HDC and holistic partial-bundle HDC.
- Stale-index rejection when the exact canonical source graph changes after index construction; insertion-order changes remain equivalent.
- Rejection of an index queried with a different encoder schema, seed or dimension.
- Separate bipolar-only, f32-only and dual-view vector-payload estimates, plus unique codebook-entry count, factorized role-slot payload, duplicated-slot baseline, byte savings, compact index-reference bytes and total retained vector payload.
- Controlled directed-circulant graph fixture generator with one-pass validation for large workloads.
- Factorization capacity sweep across 100/1,000/5,000 facts and high/medium/low node reuse, avoiding the exact-retrieval O(E²) query loop.
- Continuous 2D scalar-field prototype: bipolar positional/value encoding, spatial-memory bundling, coordinate unbinding and scalar-codebook cleanup.
- Five continuous field families and held-out comparison against nearest-neighbour and bilinear interpolation, reporting MAE/RMSE/R² and stage-level timing.
- Matched storage accounting for HDC retained vectors plus scalar encoder indexes versus raw scalar-grid values and full point records.
- Shared multi-channel VSA memory with explicit channel-role binding, compared against independent per-channel memories and scalar-grid payloads.
- Cross-channel interference tests for multiple scalar fields at the same coordinates, with exact channel-key rejection and storage/latency measurements.
- Geometric box-counting estimator with known-dimension line, Sierpiński, grid and uniform-random controls.
- Full box-count curves at seven resolutions, four grid translations, predeclared fit window, regression diagnostics and scale-window sensitivity; no automatic fractal classification.
- Fail-closed detection when distinct source coordinates quantize to the same encoded spatial position.
- Exact engineering-evidence dependency DAG, reverse transitive impact closure and conservative full invalidation when dependency completeness is unknown.
- Independent brute-force closure oracle tests and cycle/unknown-ID/unknown-endpoint fail-closed controls.
- Evidence-impact capacity sweep over 100/1,000/10,000/100,000 artifacts, independent project chains, shared common-cause inputs and project-local changes.
- CI raw benchmark artifacts containing discrete retrieval, factorization capacity, continuous-field, multi-channel-field, evidence-impact and geometric-scale CSV outputs.

## Claim boundaries

This is an initial discrete graph retrieval and analytic continuous-field research fixture, not a floor-plan generator, validated building model, GHRR implementation, or proof of fractal geometry. Current bipolar binding is commutative; direction is represented through distinct subject/object role vectors. This is **not** non-commutative matrix binding.

The f32 view is a conversion of the same bipolar code. It does not test learned continuous-HV representations. The current HoloFrac encoders use bipolar element-wise multiplicative binding and majority bundling (MAP-like), not HRR/FHRR convolution, BSC XOR, or matrix GHRR. The continuous-field experiment is a separate bipolar positional/value-memory pipeline; its analytic-field accuracy and memory profile do not establish performance on measured building fields or validate a physical PDE solver. HDC memory is compared explicitly with scalar-grid storage; a field-family result only matters when its accuracy/memory/latency trade-off is competitive for a declared workload. The evidence-impact module is exact over the declared dependency DAG; it cannot prove that the source dependency inventory is complete. Unless graph completeness is explicitly declared, all evidence is conservatively marked stale. The current fixtures are small and synthetic; their results cannot establish generalization to real buildings or superiority over commercial/academic graph retrieval systems. The geometric estimator is a box-counting diagnostic, not proof of scale invariance outside the declared finite scale range. The Sierpiński/grid/random/line controls validate the estimator on known point sets; they do not make a building design fractal by association. The multi-channel experiment tests whether channel-role binding reduces duplicated VSA storage, but it is still compared with much smaller scalar-grid payloads and may not be memory-competitive with conventional field storage.

The reported memory values are vector-payload estimates, not allocator-inclusive process RSS. The prototype retains one bipolar and one f32 holistic vector per fact, plus a shared bipolar role/value codebook. The role-slot payload therefore scales with the number of unique typed role/value pairs rather than five vectors per fact. Estimates include vector payload, per-fact codebook indices, fact IDs stored in exact postings and one vector header per postings list. They still exclude HashMap key/table allocations, fact/graph object overhead, canonical-key strings, vector allocator overhead and allocator fragmentation. The factorization sweep reports gross vector savings and net savings after compact index/postings metadata; neither is allocator-inclusive RSS. Larger qualification workloads must also measure process RSS and update costs. Timings are local microbenchmarks on a small synthetic graph and must not be treated as SOTA results. The index build is separated from query scoring; source freshness is verified before the timed batch.

The source-graph digest and encoder fingerprint are non-cryptographic regression identifiers. They are not security hashes, signatures, or provenance attestations.

HDC provides candidate retrieval only. Exact typed-graph lookup remains authoritative. On fully specified exact queries, exact graph/metadata lookup is the correctness and efficiency baseline. Partial queries with known source identifiers now also compare an exact postings intersection against the two HDC ranking treatments. HDC is being investigated for noisy-cue, alias, semantic, associative and cross-scale retrieval, not as a presumed replacement for indexed graph databases. Partial-query abstention in this prototype is determined by the exact source-graph filter; no learned confidence threshold is used. Issue #5867 tracks held-out project/topology splits, threshold calibration and risk-coverage curves, update costs, and interference/capacity sweeps: https://github.com/Luminous-Dynamics/symtropy/issues/5867.
