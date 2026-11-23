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

Candidate edges require exact agreement on:
- canonical box identity supplied upstream;
- normalized physical event kind;
- facility/read point;
- operation/transport-leg context;
- qualified event-time enclosures in the same normalized time basis, with interval separation no greater than the declared profile tolerance.

Source-local event IDs are never compared across channels. Their only role is to identify each side's records inside the resulting receipt.

Both channel-qualification flags must already be true under the upstream capture owner. This evaluator does not authenticate independence or discover shared failure domains.

Disposition:
- UNIQUELY_ASSOCIATED — exactly one complete perfect matching exists; receipts are emitted.
- AMBIGUOUS — more than one complete perfect matching exists; no receipts are emitted.
- UNMATCHED — no complete perfect matching exists.
- NOT_EVALUABLE — invalid profile, duplicate local key, invalid/unqualified interval, incomparable time basis or impossible publication/event-time relation.
- UNKNOWN — the profile itself is absent.

The evaluator uses a maximum bipartite matching, then constructs the alternating directed graph. A directed cycle proves that an alternative perfect matching exists. Only an acyclic alternating graph permits UNIQUELY_ASSOCIATED. The tests compare this algorithm with an exhaustive enumerator for every 3×3 bipartite graph (512 graphs).

This is intentionally a one-to-one v0.1 cardinality profile. Event classes whose observation channel emits multiple records for one physical operation must be consolidated by their source/capture owner or handled under a separately qualified future profile. The matcher does not silently deduplicate them.

## Frozen corpus and tests

The file at fixtures/association-v0.1.tsv contains 22 independently declared expected dispositions. Unit tests cover:
- exact and tolerance-bounded associations;
- physical context mismatches;
- globally unique matching despite local candidate ambiguity;
- multiple perfect matchings and refusal to tie-break;
- unmatched/cardinality mismatch;
- duplicate local keys;
- unqualified and incomparable time evidence;
- historical as-of publication cutoff;
- input permutation invariance;
- interval widening that exposes a second perfect matching;
- exhaustive oracle comparison over all 512 3×3 graphs.

## Run

    cargo fmt --manifest-path benchmarks/maritime-physical-event-association/Cargo.toml -- --check
    cargo test --manifest-path benchmarks/maritime-physical-event-association/Cargo.toml --locked
    cargo clippy --manifest-path benchmarks/maritime-physical-event-association/Cargo.toml --locked --all-targets -- -D warnings

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
