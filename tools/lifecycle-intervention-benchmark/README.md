# Lifecycle intervention and Pareto benchmark

A standalone Rust reference tool for issue #1856. It compares complete lifecycle intervention packages for one fictional modular environmental-sensor unit, with an identical functional unit across each scenario.

## Commands

```sh
cargo fmt --manifest-path tools/lifecycle-intervention-benchmark/Cargo.toml -- --check
cargo test --manifest-path tools/lifecycle-intervention-benchmark/Cargo.toml
SOURCE_REVISION="$(git rev-parse HEAD)" cargo run --manifest-path tools/lifecycle-intervention-benchmark/Cargo.toml
```

## Model

All fixture values are synthetic, integer-valued illustration inputs. The model compares cost, virgin material input, energy, greenhouse-gas impact, water, residual waste, and a original-asset service hours retained. Six metrics are minimized; original-asset service hours retained is maximized. The engine exposes non-dominated options, not one composite sustainability score.

Before an option can be ranked, its evidence digest/status/claim/scope/coverage, safety gate, functional-unit identity, inspection gate, required metrics, and exact material-fate balance must pass. Missing metrics remain unknown, never zero. The mutually exclusive final fates of the old asset are kept in service, reused component, recycled material, energy recovery, and residual; the sums must equal the declared input mass, and residual must agree with the residual-waste metric.

The evaluator uses a direct objective-wise dominance comparator; a separate normalized-vector full-pair oracle computes the expected finite Pareto set. Frozen expectations are checked independently. Reports bind exact manifest bytes to SHA-256, and per-evidence payload digests are recomputed.

## Boundaries

The benchmark does not calculate a real LCA, certify materials, infer material composition, authorize procurement/actuation, or establish safety or environmental outcomes. “Energy recovery” is not counted as material recycling. Composite replace-and-recover options deliver the same declared service unit via the replacement; pure end-of-life outputs would not be comparable without a functional-service baseline.

No production claim should be drawn from synthetic fixture values. Real evaluation needs declared system boundaries, geography/time, licensed/qualified source data, uncertainty, allocation rules for co-products, and function-matched measured baselines.

## Research anchors

- [ISO 59020:2024 — Circular economy: measuring and assessing circularity performance](https://www.iso.org/standard/80650.html). Supports explicit system boundaries, indicators, and reproducible assessment; does not prescribe this fixture's specific metrics or trade-offs.
- [EU Ecodesign for Sustainable Products Regulation (EU) 2024/1781](https://eur-lex.europa.eu/eli/reg/2024/1781/eng). Establishes a framework for product-specific information requirements, including potential digital product passport, repairability, durability, reuse/remanufacturing, and end-of-life information; product-specific applicability depends on the relevant delegated acts.
- [Costa, Hoolahan & Charef (2025), materials passports and material reuse](https://doi.org/10.1038/s44296-025-00079-3). Recommends retaining whole buildings where feasible, pre-redevelopment audits, deconstruction planning, interoperable passports, and lifecycle updates.
- [KC et al. (2025), review of material passports in industrialised construction](https://doi.org/10.3390/su17125661). Discusses lifecycle information and interoperability challenges.

A synthetic PASS establishes only finite contract conformance, never real-world circularity or product qualification.