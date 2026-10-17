# Multi-cycle lifecycle horizon benchmark

Standalone Rust reference benchmark for [Symtropy #1878](https://github.com/Luminous-Dynamics/symtropy/issues/1878). The model uses a fully synthetic modular environmental sensor/logger and a three-block required service horizon.

## Run

```sh
cargo fmt --manifest-path tools/lifecycle-horizon-benchmark/Cargo.toml -- --check
cargo test --manifest-path tools/lifecycle-horizon-benchmark/Cargo.toml
SOURCE_REVISION="$(git rev-parse HEAD)" cargo run --manifest-path tools/lifecycle-horizon-benchmark/Cargo.toml
```

## Why a horizon benchmark

A circularity choice cannot be judged only when an asset is discarded. Wear, repair limits, remanufacture, replacement, remaining service demand, and original-asset lineage interact over several cycles. Each plan must attempt the same three 10,000-hour service blocks and must pass condition, evidence, safety/function/inspection, and mass-fate gates at every cycle.

Condition is a bounded synthetic state in basis points. It degrades after each delivered service block; repair restores a declared amount; remanufacture resets condition and caps, but cannot increase, original-asset lineage; replacement resets condition and assigns zero lineage share to the initial asset. Lineage-weighted service hours are a synthetic accounting proxy, not a measured physical circularity value.

## Reproducibility and assurance

- Evidence payload digests are recomputed from exact UTF-8 bytes.
- Action profiles bind evidence status, coverage, claim/scope, safety, function, and inspection.
- Retired component/asset mass must be partitioned into reuse, material recycling, energy recovery, or residual. The four fates must sum to retired mass; replacement must retire exactly the declared asset mass.
- Unknown evidence never becomes approval. Partial, revoked, stale, invalid-digest, failed-gate, below-condition, exceeded-repair-limit, unknown-action, and invalid-plan-length cases are distinct.
- The production state transition is compared with a separately structured reference transition evaluator. Pareto output is compared with an independent exhaustive pairwise objective-vector oracle.
- Reports bind exact manifest bytes, source revision, and tool version.

## Research grounding

ISO 59020:2024 calls for defined system boundaries, selected indicators, and reproducible circularity assessment. ISO's second edition is now a working draft under development; pin the edition/profile instead of silently changing measurement semantics:
- https://www.iso.org/standard/80650.html
- https://www.iso.org/standard/94166.html

The EU Ecodesign for Sustainable Products Regulation creates a framework for product-specific information and performance requirements relevant to durability, repairability, reusability, remanufacturing, material recovery, and end-of-life handling. Product-specific duties depend on applicable delegated acts:
- https://eur-lex.europa.eu/eli/reg/2024/1781/oj

A 2025 materials-passport paper recommends deconstruction audits, preserving existing assets where appropriate, and interoperable lifecycle records:
- https://doi.org/10.1038/s44296-025-00079-3

## Claim ceiling

Synthetic contract conformance only. It does not establish actual degradation, useful remaining life, real repair yield, supplier claims, safety qualification, life-cycle assessment, or environmental benefit. A passing benchmark is not authorization to operate, repair, disassemble, procure, or dispose of a physical asset.
