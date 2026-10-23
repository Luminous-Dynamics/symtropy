# Mobility Interval Robustness Audit v0.1

**Status:** synthetic-only decision-support contract and Rust reference implementation proposal. It is not a product ranking, market-access study, safety validation, purchase recommendation, dispatch controller, or autonomous-control input.

## Paradigm shift: from a crisp frontier to an evidence-aware frontier

A conventional Pareto report can be correct for its input numbers and still imply more certainty than those numbers justify. Mobility costs, access time, repair downtime, trip coverage and lifecycle resource estimates vary across service conditions and evidence quality. A single midpoint conceals that uncertainty; a weighted score hides value choices.

This audit takes intervals as explicit bounds and answers three different questions:

- **Robust frontier:** an eligible bundle is not guaranteed to be dominated across the full declared ranges. It survives conservative elimination, but this does not mean it is best.
- **Stable frontier:** no other eligible bundle can possibly dominate it under any admissible values inside the declared intervals. This is a strong, deterministic statement under this input model.
- **Measurement backlog:** for each metric, count unordered candidate pairs whose interval ordering could reverse strictly. Sort by count to expose which metrics leave the most pairwise comparisons unresolved.

The measurement backlog is a transparent first-pass **count**, not a probability, confidence level, expected value of information (EVPI), or optimized survey design. Intervals must not be assigned probabilities unless a separate supported model and data justify them.

## Why this matters for global mobility

The IEA's *Global EV Outlook 2026* reports divergent regional adoption patterns, and its charging chapter discusses home-charging access, public charging, grid constraints and the importance of matching infrastructure to deployments. That evidence motivates modeling charging and service access as scenario-dependent inputs, not assumptions that hold for every household. See the [2026 executive summary](https://www.iea.org/reports/global-ev-outlook-2026/executive-summary) and [charging chapter](https://www.iea.org/reports/global-ev-outlook-2026/electric-vehicle-charging-chap-6-and-10).

The European Commission's [31 July 2026 right-to-repair announcement](https://commission.europa.eu/news-and-media/news/right-repair-new-consumer-rights-easy-and-attractive-repairs-2026-07-31_en) describes repair obligations and spare-part/service access for covered products under EU rules. Treating repairability as an observed, jurisdiction-neutral fact would be unsound; gates and their supporting evidence need explicit scope.

## Method

### 1. Hard gates before metrics

Each candidate must give every declared gate exactly once. A failed gate produces `blocked`; an unknown gate without a failure produces `needs_evidence`; only candidates whose gates all pass can enter the comparison set. A cheap or high-coverage candidate can never compensate for failed scope or safety/legal evidence.

### 2. Interval contracts

Every metric record has a declared metric ID, exact unit, status, lower bound and upper bound. `synthetic_interval` requires finite numeric endpoints with lower less than or equal to upper. `unknown` requires both endpoints to be null. Missing, duplicate or undeclared IDs, unit mismatches, unknown fields and reversed intervals fail closed. An unknown interval is never converted to zero.

### 3. Two dominance relations

For minimizing metrics, bundle A robustly beats B on a metric only when A's upper bound is no greater than B's lower bound. For maximizing metrics, A's lower bound must be no less than B's upper bound. The same direction applies across every declared metric, with at least one strict improvement. If interval bounds overlap, robust dominance may not be proven.

Possible dominance is weaker: for each minimizing metric A's lower bound must be no greater than B's upper bound; for each maximizing metric A's upper bound must be no less than B's lower bound. At least one strict improvement must be possible. If a candidate can be possibly dominated, it is not in the stable frontier.

**Important modeling assumption:** these are rectangular bounds. The calculation considers metric values inside their intervals without encoding cross-metric or cross-candidate correlation. This can be conservative and may describe combinations that cannot occur together. For correlated uncertainties, use separately defined joint scenarios and retain scenario identity; do not add probabilities or narrow bounds by intuition.

### 4. Measurement priority

For a metric and eligible candidate pair, ordering is ambiguous when each candidate can be strictly better than the other under some in-range choices. Count those pairs per metric, then sort descending by count and ascending by metric ID for ties. This helps plan evidence collection; it does not say how expensive collection is, how likely a bound is, or how much a measurement would improve a decision.

## Synthetic fixture and negative controls

The fixture has eight invented bundles and five metrics. Five candidates have complete pass gates and bounded intervals; one fails a hard gate despite attractive fabricated metrics; one has unknown energy/service and repair gates; one has an unknown budget interval. All numbers and gate states are constructed only to exercise the implementation. They are not observations, vendor quotes, regulatory determinations, physical tests or environmental assessments. An independent readback calculation of this fixture retains all five fully evidenced candidates on the conservative robust frontier, but yields an empty stable frontier: the bounds are deliberately too broad to establish that any one candidate cannot be dominated under every admissible realization. The pairwise order-ambiguity counts are 10 for essential-trip coverage, 10 for P90 access time, 9 for monthly burden, 8 for repair downtime and 6 for the lifecycle index. These counts prioritize follow-up evidence only; they do not express likelihood or value of information.

## Benchmarks and acceptance targets

- **Oracle accuracy:** 100% match to a separately written exact Pareto oracle for every 4-point subset of the 3D binary cube (70 subsets), represented as degenerate point intervals. Robust and stable frontiers must match classical Pareto output on those cases.
- **Hard-gate integrity:** 0 failed or unknown-gate candidates in either frontier.
- **Missingness integrity:** 0 candidates with unknown intervals in either frontier; 0 unknown-to-zero imputations.
- **Contract integrity:** 0 duplicate IDs, unit mismatches, reversed intervals, non-finite endpoints or unknown fields accepted.
- **Reproducibility:** canonical result byte-identical when candidate, gate, metric and definition order is permuted.
- **No false certainty:** zero weighted utility score, probability model or EVPI value emitted by this audit.
- **Boundary semantics:** exact point intervals must reproduce the established Pareto result; interval fixtures must distinguish robust survival from stable frontier membership.

These are authored tests/acceptance targets. They are not performance or correctness results until the exact source head runs successfully under the committed workflow.

## Commands

```sh
cargo fmt --manifest-path tools/mobility-interval-audit/Cargo.toml -- --check
cargo test --manifest-path tools/mobility-interval-audit/Cargo.toml --all-targets
cargo run --manifest-path tools/mobility-interval-audit/Cargo.toml -- fixtures/mobility_robustness/mobility_interval_scenarios_v0_1.json
```

## Claim ceiling and next research

Determinism and synthetic oracle agreement establish properties of this implementation on its declared input contract; they do not validate actual transport choices. Before real-world application, each metric interval needs source, timestamp, geography, population, units, sampling or modeling method, uncertainty rationale, validity window and responsible owner. Price should be separated into upfront cost, financing, energy/fuel, service, insurance, downtime and end-of-life costs where data allows. Mobility service bundles should be assessed against user-defined essential trips and actual service windows, not just a vehicle specification sheet.

Next, add correlated joint stress scenarios (service network unavailable, energy price shock, spare-part delay, adverse weather or accessibility constraint) and empirical calibration from rights-cleared, geographically scoped observations. Keep it decision-support only; no physical or autonomous action is authorized by this audit.
