# Maritime physical-event association v0.1

A standalone, synthetic-first Rust benchmark for establishing a **unique correspondence** between an already-admitted source event and an independently captured physical observation.

## Why this is different from cargo identity or custody

- #6631 resolves source-local aliases to canonical physical cargo identity.
- #6971 qualifies whether source/witness channels can be treated as complete within their declared envelopes.
- #6783 and #6662 provide qualified time enclosures and clock provenance.
- #6945 provides versioned source event semantics.
- #6662 also correlates paired custody RELEASE/ACCEPT handoffs.

This crate does not duplicate any of those. It asks a narrower question: given two independently identified records for a candidate physical operation, can the exact available evidence establish one unique one-to-one correspondence?

## Contract

Each side declares a typed time role in the frozen profile. The v0.1 test profile is fixed to a source business-event time and an independent sensor-observation time; unsupported profile role pairings and per-record mismatches are NOT_EVALUABLE. Each record must carry the role expected by its channel. For this role pair, the independent sensor-observation interval must be no later than the business-event interval: the witness interval's upper bound must be less than or equal to the source interval's lower bound. Equality is allowed. A definitely later sensor interval is rejected; overlapping intervals are preserved as a distinct `TIME_ORDER_UNRESOLVED` result, not mislabeled as an unmatched event. This prevents silently treating repository record time, source publication time, sensor observation time and business-step event time as interchangeable.

For EPCIS, GS1 distinguishes the time a sensor observation occurred (`sensorElementList` SensorReport `time`) from the business-step `eventTime`; repository `recordTime` is a repository bookkeeping timestamp, not the time of the physical event. The adapter must preserve those roles and map them to qualified time enclosures upstream. See the GS1 EPCIS 2.0.1 Standard and SensorReport model: https://ref.gs1.org/standards/epcis/2.0.1/ and https://ref.gs1.org/epcis/SensorReport.

Candidate edges require exact agreement on:
- canonical box identity supplied upstream;
- normalized physical event kind;
- facility/read point;
- operation/transport-leg context;
- qualified event-time enclosures in the same normalized time basis, with interval separation no greater than the declared profile tolerance. The profile digest binds the declared role mapping and tolerance.

Source-local event IDs are never compared across channels. Their only role is to identify each side's records inside the resulting receipt.

Both channel-qualification flags must already be true under the upstream capture owner. This evaluator does not authenticate independence or discover shared failure domains.

Each record also retains its source-local revision token and upstream-supplied payload SHA-256 string; the correlation profile carries its own SHA-256 string. The evaluator checks that these digest strings have 64 hexadecimal characters and preserves them in every correspondence receipt. It does **not** fetch bytes, recompute hashes, verify signatures or authenticate issuers; the upstream byte-verification and rights owners remain authoritative.

Disposition:
- UNIQUELY_ASSOCIATED — exactly one complete perfect matching exists; receipts are emitted.
- AMBIGUOUS — more than one complete perfect matching exists; no receipts are emitted.
- UNMATCHED — no complete perfect matching exists.
- TIME_ORDER_UNRESOLVED — a unique candidate pairing exists, but the qualified sensor/business time enclosures overlap and their required order cannot be proved; no receipt is emitted.
- NOT_EVALUABLE — invalid profile, duplicate local key, invalid/unqualified interval, incomparable time basis or impossible publication/event-time relation.
- UNKNOWN — the profile itself is absent.

The evaluator uses a maximum bipartite matching, then constructs the alternating directed graph. A directed cycle proves that an alternative perfect matching exists. Only an acyclic alternating graph permits UNIQUELY_ASSOCIATED. The tests compare this algorithm with an exhaustive enumerator for every 3×3 bipartite graph (512 graphs).

This is intentionally a one-to-one v0.1 cardinality profile. Event classes whose observation channel emits multiple records for one physical operation must be consolidated by their source/capture owner or handled under a separately qualified future profile. The matcher does not silently deduplicate them. A zero-by-zero snapshot has a vacuously unique empty matching only when both upstream channels are already qualified; that result does **not** establish that no physical event occurred or that either channel is complete. Capture completeness remains #6971's proposition.

## Frozen corpus and tests

The file at fixtures/association-v0.1.tsv contains 31 independently declared expected dispositions. Unit tests cover:
- exact and tolerance-bounded associations;
- physical context mismatches;
- globally unique matching despite local candidate ambiguity;
- multiple perfect matchings and refusal to tie-break;
- unmatched/cardinality mismatch;
- duplicate local keys and missing revision tokens;
- malformed profile/source/witness digest strings;
- unqualified and incomparable time evidence, source/witness temporal-role mismatch, sensor observations definitely after the claimed business event, and overlapping time intervals that remain unresolved;
- historical as-of publication cutoff;
- input permutation invariance;
- interval widening that exposes a second perfect matching;
- exhaustive oracle comparison over all 512 3×3 graphs;
- a **test-only Hopcroft–Karp challenger** differentially checked against the production DFS matcher and exhaustive oracle on all 512 3×3 graphs;
- a deterministic, seeded 4,096-case 6×6 graph panel comparing Hopcroft–Karp and DFS classifications;
- a test-only two-order interval index challenger that reports exact candidate windows from both interval endpoints, differentially checked against the full exact-context scan on boundary and seeded mixed-context inputs;
- a 128×128 dense-context synthetic control where the interval challenger preserves the exact same edge graph while reducing temporal predicate evaluations from 16,384 to 128.

The production matcher remains augmenting-path DFS and production candidate generation remains the exact-context BTreeMap scan. Both challengers are compiled only for tests (`cfg(test)`) and are not selected for production. The interval challenger result is a comparison-count reduction on a synthetic dense-context fixture, not a wall-clock speedup or field throughput claim. Production adoption requires successful exact-head qualification plus matched benchmarks reporting graph density, candidate generation, p50/p95/p99 runtime, peak memory and output equivalence. The alternating-cycle uniqueness check remains unchanged in both matching implementations.

## Run

    cargo fmt --manifest-path benchmarks/maritime-physical-event-association/Cargo.toml -- --check
    cargo test --manifest-path benchmarks/maritime-physical-event-association/Cargo.toml --locked
    cargo clippy --manifest-path benchmarks/maritime-physical-event-association/Cargo.toml --locked --all-targets -- -D warnings

## Algorithm and scale boundary

The P0 implementation currently:
- blocks witness records by exact, upstream-normalized canonical box, event kind, facility and operation context;
- evaluates interval compatibility only inside each exact-context block, with measured `candidate_pair_evaluations` exposed in the report;
- production maximum matching currently uses augmenting-path DFS, O(V × E) worst-case for the balanced graph;
- a test-only Hopcroft–Karp implementation is a differential challenger, O(E√V) asymptotically, and is not yet used by production;
- a test-only two-order interval index sorts each context's witness intervals by lower and upper endpoints and reports candidates satisfying the exact frozen window bounds in O(log n + min(P,S)) query work plus returned candidates, where P is the lower-endpoint prefix and S is the upper-endpoint suffix; worst-case work can still be linear per source in a dense overlap population;
- checks uniqueness with an alternating directed graph cycle traversal, O(V + E), independently of which maximum-matching implementation is used.

Let V = V_s + V_w, E be admissible graph edges, and K be the number of source/witness pairs sharing an exact context block. Production candidate generation costs O(V log G + K) using the current BTreeMap index over G distinct contexts, rather than testing all V_s × V_w pairs. K can still be quadratic in a single dense context block; the counter makes that residual work visible. The test-only interval challenger adds O(V_w) index memory and per-source O(log n + min(P,S)) broad-window reporting plus returned candidates (P is the lower-endpoint prefix and S the upper-endpoint suffix); worst-case dense overlap remains linear per source. Matching is O(V × E) worst-case for production DFS, while the test-only Hopcroft–Karp challenger is asymptotically O(E√V); uniqueness checking remains O(V + E). The graph and matching state use O(V + E + G) memory after context indexing. These are algorithmic bounds, not measured production throughput. These are algorithmic bounds for the current implementation, not measured production throughput. The P0 graph sizes are tiny and the exhaustive 3×3 oracle is the correctness reference.

The deterministic sparse-context scale control uses 128 source records and 128 witness records with 128 distinct exact contexts. It produced 128 candidate-pair evaluations, 128 admissible edges and 128 receipts; a global 128×128 cross-product would evaluate 16,384 pairs. This is an exact comparison-count result for the synthetic fixture, not a measured wall-clock speedup, field throughput result or dense-block complexity guarantee. The report's visible counts retain the full as-of denominators even when the unresolved-key lists are empty after a unique association.

The next optimization stage should preserve exact outputs while benchmarking both challengers on matched distributions:
1. The test-only interval challenger matches the full context scan on frozen boundary and seeded mixed-context inputs; on the synthetic 128×128 dense-context panel it retains 128 exact edges while reducing time-predicate evaluations from 16,384 to 128. Next measure sparse, dense and clustered interval distributions before any production adoption.
2. The test-only Hopcroft–Karp challenger is differentially checked against DFS on 512 exhaustive 3×3 and 4,096 seeded 6×6 graphs. Benchmark both implementations before considering a production replacement; the alternating-cycle uniqueness proof remains the common correctness gate.
3. Publish candidate-pair evaluations, edge density, matched hardware/input profiles, p50/p95/p99 latency, throughput and peak memory at 10², 10³, 10⁴ and larger events. Do not publish speedup claims until measured.

The 2026 survey/experimental benchmark on heterogeneous entity matching describes representation and semantic heterogeneity as important test dimensions. It is a useful algorithmic baseline, not a freight-specific physical-truth benchmark: https://doi.org/10.1016/j.datak.2026.102575. Symtropy's intended differentiator is not a new general entity-matching algorithm; it is the stricter correspondence contract over canonical cargo identity, typed physical-event semantics, qualified temporal roles, source/witness provenance and explicit refusal when correspondence is non-unique.

## Benchmark targets

Prospective targets, not achieved real-world results:
- 100% agreement with the exhaustive small-graph oracle.
- Zero false-unique outputs on frozen ambiguous/negative cases.
- 100% ambiguity detection for non-unique perfect matchings.
- 100% permutation invariance.
- Zero candidate edges across a mismatched canonical ID, event kind, facility, operation context or time basis.
- P2: zero false-unique outputs over at least 3,000 independently adjudicated negative/ambiguous cases, with cluster dependence and effective sample size reported.

Compare against greedy nearest-time matching, minimum-gap assignment with deterministic tie-breaking, and a maximum-cardinality matcher that returns one arbitrary assignment without checking uniqueness. Report false association, ambiguity and unmatched detection separately, plus edge density, throughput, p50/p95/p99 runtime and peak memory. Do not assert SOTA based on synthetic accuracy or asymptotic complexity alone.

## Claim ceiling

A passing P0 establishes only deterministic association behaviour on the synthetic profile and oracle. It does not establish physical occurrence, channel independence, real-world capture completeness, canonical identity correctness, clock traceability, custody, delivery, legal ownership or operational readiness. No source record is mutated and no booking, inventory, terminal movement or custody transition is authorized.
