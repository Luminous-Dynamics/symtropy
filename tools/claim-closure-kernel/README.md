# Engineering Claim Closure Kernel v0.1

A small Rust benchmark kernel for one narrowly defined question:

**Does a declared engineering claim have a structurally closed, time-consistent, digest-linked evidence dependency graph that can be independently replayed?**

It is deliberately not a universal engineering-certification engine. A closed graph means only that the submitted references and declared metadata satisfy this kernel's structural rules. It does not mean that the evidence is true, that a producer is cryptographically authenticated, that an uncertainty model is physically adequate, or that a design is safe.

## Why this exists

Symtropy now has domain-specific work covering temporal evidence replay, model-report attestation, as-built magnetic acceptance, spacecraft sensor bundles, coherent capacity scenarios, beneficiary floors and flow certificates. They each need related structural guarantees:

- exact identity and configuration bindings;
- digest-linked inputs and outputs;
- recursive dependency closure;
- valid-time and knowledge-time separation;
- producer-authority and trust snapshots;
- explicit uncertainty-budget references;
- deterministic handling of ordering, duplicates and cycles;
- independently recomputable reports.

The kernel isolates that shared substrate without replacing domain semantics. Domain evaluators remain responsible for determining which evidence is required and what evidence means.

## Input and result

The input contains one claim, its required evidence bindings, and an evidence graph. Every evidence record declares:
- an immutable record digest;
- role, subject, configuration and scope;
- recorded time and a half-open effective-time interval;
- producer-authority and trust snapshots;
- an uncertainty-budget identity and resolution state;
- zero or more digest-bound dependency edges.

The evaluator traverses only evidence reachable from the claim's required evidence. It checks identity, time cuts, digests, upstream closure, cycles, duplicate references and uncertainty references. Input ordering is canonicalized before the report commitment is computed.

The only positive disposition is `CLOSED_FOR_INDEPENDENT_REVIEW`. Every other condition is `BLOCKED` with typed findings. The evaluator does not rank or average findings: a positive item cannot compensate for a missing or invalid ancestor.

A second function independently recomputes the report from the input and compares it with the supplied report. This detects report tampering and stale reports. It is not a substitute for signature verification.

## Critical trust boundary

The fields `producer_authority.status = verified` and `trust_snapshot.status = verified` are declarations supplied in the input. Version 0.1 checks their identifiers, digests and temporal placement, but does not verify signatures, certificate chains, revocation proofs or transparency receipts. A future adapter may verify SCITT or other receipts before passing an authenticated status into this kernel.

Likewise, `record_sha256` and dependency digests are checked for format and exact linkage, not recomputed from unavailable source bytes. The kernel must never be presented as cryptographic integrity verification.

## Research foundations and complementary standards

- [NASA-STD-7009B](https://standards.nasa.gov/node/263) establishes model-and-simulation credibility practices across the life cycle. NASA's [2026 implementation handbook](https://standards.nasa.gov/standard/NASA/NASA-HDBK-7009) adds examples and guidance; it is not a replacement for the standard.
- [ASME VVUQ 20.1-2024](https://www.asme.org/codes-standards/find-codes-standards/vvuq201-vvuq-20-1-multivariate-metric-for-validation) addresses multivariate comparison of experimental and simulated results, including independent or shared uncertainty across validation points.
- [IETF RFC 9943 (SCITT)](https://www.rfc-editor.org/rfc/rfc9943) defines signed statements, transparency receipts and auditable statement history for digital supply chains.
- [SLSA provenance](https://slsa.dev/spec/build-provenance) and [in-toto attestations](https://in-toto.io/docs/specs/) provide adjacent software-supply-chain provenance mechanisms.
- [ISO 23247-5:2026](https://www.iso.org/standard/87425.html) addresses digital thread connectivity and management across manufacturing life-cycle stages; [ISO 23247-6:2026](https://www.iso.org/standard/87426.html) addresses interoperable digital-twin composition.

These standards are complementary comparators, not direct performance competitors. The benchmark evaluates a narrower executable property: structural claim closure under adversarial mutations.

## Benchmark design

The initial suite should test positive controls and mutations for:
1. missing required evidence and missing transitive ancestors;
2. record-digest substitution and wrong dependency digests;
3. subject, configuration, scope and role substitution;
4. future or stale knowledge and trust snapshots;
5. evidence intervals that do not cover the acquisition interval;
6. revoked, disputed or unverified producer/trust states;
7. unresolved or substituted uncertainty budgets;
8. duplicate identifiers, duplicate edges and dependency cycles;
9. input permutation and deterministic replay;
10. report tampering and resource-limit exhaustion.

The plan records target metrics, not achieved results. The first acceptance target is zero false-closures on the frozen adversarial suite and zero false-blocks on valid positive controls. Scale cases cover 100, 1,000 and 10,000 evidence records with varying graph fan-out and depth. Report p50/p95/p99 latency, peak memory and throughput; compare only matched input graphs.

## Running

```bash
cargo test --manifest-path tools/claim-closure-kernel/Cargo.toml
cargo clippy --manifest-path tools/claim-closure-kernel/Cargo.toml --all-targets -- -D warnings
cargo run --manifest-path tools/claim-closure-kernel/Cargo.toml -- evaluate tools/claim-closure-kernel/fixtures/positive-v0.1.json
```

A successful structural result is not a domain release decision. Domain-specific physical qualification and independent review remain mandatory.
