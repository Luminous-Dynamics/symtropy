# Cluster-aware false-safe risk intervals v0.1

**Issue:** [NAV-CLUSTER-RISK-001 #6691](https://github.com/Luminous-Dynamics/symtropy/issues/6691)  
**Parent:** [NAV-RISK-COVERAGE-001 #6687](https://github.com/Luminous-Dynamics/symtropy/issues/6687)

## Why

The base risk–coverage evaluator's Wilson interval assumes independent Bernoulli records. Navigation streams violate this assumption when many records share the same physical event, device, site, clock, anchor layout or environmental disturbance. A thousand overlapping windows from a single event are not a thousand independent trials.

## Method

Choose and freeze one cluster unit before evaluating test data (for example, a complete event/session). The bootstrap samples the same number of clusters with replacement and carries every record from each sampled cluster together.

For each replicate, it computes:
- false-safe records / covered records;
- false-safe exposure seconds / covered exposure seconds.

It returns two-sided 95% nearest-rank percentile intervals, the requested/valid/zero-coverage replicate counts, cluster counts, threshold, seed and algorithm version. The implementation uses deterministic SplitMix64 and stable sorted cluster/record identifiers.

At least 80% of replicates must have non-zero coverage for intervals to be emitted. Zero-coverage replicates are counted, not assigned a zero risk value. At least two distinct clusters are required.

## Usage

```sh
cargo run --manifest-path research/nav-observation-toy-fim/Cargo.toml \
  --bin clustered_risk_demo
```

## Limits

A single declared cluster axis does not capture arbitrary multi-way or nested dependence. Event, site and device may all matter simultaneously. Use a scientifically justified cluster unit; consider hierarchical or multiway bootstrap for nested structures. This method cannot discover hidden shared causes and is not an integrity certificate.
