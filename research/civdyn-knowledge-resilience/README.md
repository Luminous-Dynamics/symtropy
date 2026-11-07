# CivDyn Knowledge Resilience v0.1

A source-aware trace auditor for knowledge transmission, durable capability and cultural memory.

## Capability lifecycle

This benchmark does not equate content delivery with learning. It reports:
- transmission delivery;
- acquisition and declared acquisition source;
- comprehension;
- demonstrated performance against an item-specific threshold;
- delayed retention at an explicit interval;
- independent teaching to another learner.

Carrier-state counts are exported as denominators alongside acquisition and comprehension counts. A claim may be acquired while epistemically refuted. That does not upgrade its truth status; refuted-item uptake and onward teaching remain separately measurable.

## Knowledge and evidence lineage

Each item has a semantic family, epistemic status, declared evidence-source families and prerequisite items. Repeated URLs or multiple copies from one source family do not count as independent corroboration.

## Archive resilience

The CSV includes separate scope and event-detail fields; transmission sender/receiver IDs are never written into the semantic-family column. Archive copies are grouped by failure domain. Three copies across two domains count as two independent domains, not three. A copy is recoverable only when its checksum is verified, it remains accessible and its failure domain is not in the declared failed-domain set.

## Synthetic reference

The fixture models three generations:
- demonstrated hydrology capability discovered by generation 0, passed through generation 1, and demonstrated again in generation 2;
- a delivered-but-not-acquired exposure;
- a refuted water claim that is acquired and taught onward;
- three verified hydrology archive copies across two failure domains, plus an unverified copy of the refuted claim.

The fixture uses generated opaque IDs and artificial records only. It does not reproduce any human cultural transmission experiment.

## Run

```sh
cargo fmt --manifest-path research/civdyn-knowledge-resilience/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path research/civdyn-knowledge-resilience/Cargo.toml --all-targets -- -D warnings
cargo check --locked --manifest-path research/civdyn-knowledge-resilience/Cargo.toml --all-targets
cargo test --locked --manifest-path research/civdyn-knowledge-resilience/Cargo.toml --all-targets
cargo run --locked --manifest-path research/civdyn-knowledge-resilience/Cargo.toml --bin knowledge-resilience-sweep
```

## Claim ceiling and next gates

The v0.1 reference validates evidence-aware trace semantics, acquisition/capability separation, delayed retention and archive failure-domain accounting. It does not predict actual human teaching choices or model the full causal dynamics of cultural evolution.

V1 requires source-exact tasks, licensed human data, repeated generational trials, preregistered performance tests, held-out task families, network/teacher-loss shocks, retention curves and comparison of random, prestige/payoff-biased, intentional teaching, structured context, archive-first and evidence-first policies.
