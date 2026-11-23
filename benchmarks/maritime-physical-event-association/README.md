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

Each side declares a typed time role in the frozen profile. The v0.1 test profile compares a source business-event time with an independent sensor-observation time; each record must carry the role expected by its channel. For this role pair, the independent sensor-observation interval must be no later than the business-event interval: the witness interval's upper bound must be less than or equal to the source interval's lower bound. Equality is allowed; overlap without a proven ordering is refused. This prevents silently treating repository record time, source publication time, sensor observation time and business-step event time as interchangeable.

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
- NOT_EVALUABLE — invalid profile, duplicate local key, invalid/unqualified interval, incomparable time basis or impossible publication/event-time relation.
- UNKNOWN — the profile itself is absent.

The evaluator uses a maximum bipartite matching, then constructs the alternating directed graph. A directed cycle proves that an alternative perfect matching exists. Only an acyclic alternating graph permits UNIQUELY_ASSOCIATED. The tests compare this algorithm with an exhaustive enumerator for every 3×3 bipartite graph (512 graphs).

This is intentionally a one-to-one v0.1 cardinality profile. Event classes whose observation channel emits multiple records for one physical operation must be consolidated by their source/capture owner or handled under a separately qualified future profile. The matcher does not silently deduplicate them. A zero-by-zero snapshot has a vacuously unique empty matching only when both upstream channels are already qualified; that result does **not** establish that no physical event occurred or that either channel is complete. Capture completeness remains #6971's proposition.

## Frozen corpus and tests

The file at fixtures/association-v0.1.tsv contains 29 independently declared expected dispositions. Unit tests cover:
- exact and tolerance-bounded associations;
- physical context mismatches;
- globally unique matching despite local candidate ambiguity;
- multiple perfect matchings and refusal to tie-break;
- unmatched/cardinality mismatch;
- duplicate local keys and missing revision tokens;
- malformed profile/source/witness digest strings;
- unqualified and incomparable time evidence, source/witness temporal-role mismatch, and sensor observations that occur after the claimed business event;
- historical as-of publication cutoff;
- input permutation invariance;
- interval widening that exposes a second perfect matching;
- exhaustive oracle comparison over all 512 3×3 graphs.

## Run

    cargo fmt --manifest-path benchmarks/maritime-physical-event-association/Cargo.toml -- --check
    cargo test --manifest-path benchmarks/maritime-physical-event-association/Cargo.toml --locked
    cargo clippy --manifest-path benchmarks/maritime-physical-event-association/Cargo.toml --locked --all-targets -- -D warnings

## Algorithm and scale boundary

The P0 implementation currently:
- constructs the candidate graph with a straightforward pairwise scan, O(V_s × V_w) candidate checks;
- finds a maximum matching with augmenting-path DFS, O(V × E) worst-case for the balanced graph;
- checks uniqueness with an alternating directed graph cycle traversal, O(V + E).

The graph and matching state use O(V + E) memory after edge construction. These are algorithmic bounds for the current implementation, not measured production throughput. The P0 graph sizes are tiny and the exhaustive 3×3 oracle is the correctness reference.

For larger shadow datasets, the next optimization should preserve the exact edge predicate while reducing unnecessary comparisons:
1. Block only on exact, upstream-normalized canonical box, event kind, facility and operation context. Never use lossy bucketing that can remove a valid interval edge.
2. Use interval indexes / sweep windows to generate only time-compatible edges.
3. Evaluate Hopcroft–Karp maximum matching (O(E√V)) against the current DFS matcher, retaining the same alternating-cycle uniqueness test and exhaustive oracle on small graphs.
4. Publish matched hardware/input profiles, edge density, p50/p95/p99 latency, throughput and peak memory at 10², 10³, 10⁴ and larger events. Do not publish speedup claims until measured.

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
