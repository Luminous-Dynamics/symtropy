# Magnetic route cohort qualification

**Programme:** MAG-AM-ROUTE-COHORT-001  
**Issue:** [#7774](https://github.com/Luminous-Dynamics/symtropy/issues/7774)  
**Status:** standalone Rust 2024 evaluator; synthetic accounting controls only

## Purpose

Aggregate specimen-level acceptance records into a build/lot-aware manufacturing cohort. The denominator is defined by initiated build manifests, not by the subset of specimens with convenient inspection records.

## Compose with existing owners

- #7368 — spatial process variability, field-function uncertainty and accepted-part yield;
- #7770 — specimen evidence and route Pareto evaluator;
- #7745 — as-built magnetic specimen acceptance;
- #2933 — manufacturing genealogy and process receipts;
- #3972 — correlated uncertainty propagation.

This crate owns cohort accounting and claim gating only. It does not reproduce specimen qualification, physical process modelling, statistics/uncertainty engines or product-cost accounting.

## Decision and reporting

- Preserve every initiated build and its declared specimen IDs.
- Missing, duplicate, orphaned, failed, scrapped and unresolved outcomes remain visible; missing specimen records remain in the denominator.
- First-pass qualification, confirmed failure, unresolved outcomes, inspection coverage, build completion, rework recovery and final accepted fraction are separate exact numerator/denominator rates.
- Rework never changes first-pass acceptance.
- Multiple specimens from a single build do not count as independent build replications.
- A preregistered minimum build/lot coverage may be reported as met, but **v0.1 never emits an inferential production-yield claim**.
- Accepted-part cost consumes a digest-bound campaign-cost receipt and preserves the total-cost numerator and accepted-parts denominator. Missing cost evidence remains unknown rather than invented.

## Files

- [Evaluator source](evaluator/src/lib.rs)
- [Synthetic fixture schema](evaluator/cohort-fixture-v0.1.schema.json)
- [Synthetic known-answer cohort](evaluator/fixtures/synthetic-route-cohort-v0.1.json)
- [Benchmark plan](benchmark-plan.toml)

## Claim boundary

The fixture tests denominator, identity, missingness, rework, scrap and replication semantics only. It is not physical route-yield evidence, process capability, a production-quality estimate, certification or product-release authority.
