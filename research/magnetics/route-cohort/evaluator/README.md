# Magnetic route cohort evaluator

**Programme:** MAG-AM-ROUTE-COHORT-001  
**Issue:** #7774  
**Parents:** #7368 process robustness, #7770 route evaluator, #7745 as-built specimen acceptance  
**Status:** standalone Rust 2024 reference evaluator; synthetic accounting controls only

## Purpose

Aggregate specimen-level qualification into an auditable manufacturing-cohort summary. It prevents best-specimen cherry-picking, missing-outcome deletion, rework inflation and false claims of independent replication.

## Core accounting rules

- Build records declare every initiated specimen ID; the denominator comes from those declarations, not just the specimens for which a convenient inspection record exists.
- An initiated specimen with no linked record remains in the denominator as MISSING.
- Rework recovery is reported separately and never changes first-pass disposition.
- A build is a replication unit distinct from its specimens; a lot is distinct from a build.
- Descriptive observed fractions are not production-yield inference.
- Inferential yield is never emitted in v0.1. The replication policy can only say whether a preregistered minimum build/lot coverage is met.
- Campaign cost is consumed from a digest-bound cost receipt. Cost per accepted part is represented as exact numerator / denominator, not an averaged successful-part cost.

## Outputs

- initiated builds and specimens;
- completed builds/specimens and pre-inspection build failures;
- first-pass QUALIFIED / FAIL / INDETERMINATE / NOT_EVALUABLE / MISSING counts;
- inspection coverage;
- observed first-pass qualification fraction;
- unresolved fraction;
- reworked specimens and rework recovery fraction;
- final non-scrapped qualified count / observed final accepted fraction;
- cost receipt numerator, currency and accepted-part denominator;
- distinct build and lot counts plus replication-coverage status.

## Limitations

This crate validates identity, denominators, event semantics and descriptive arithmetic. It does not estimate a population yield distribution, build a hierarchical model, derive process capability, compute process uncertainty, qualify a physical process, authenticate receipt producers, or issue product release.

Synthetic fixtures are test oracles only.


## Cohort semantics

The initiated specimen denominator is the union of specimen IDs declared by every initiated build manifest. Missing specimen records do not shrink that denominator. Duplicate, orphaned, mismatched, or receipt-invalid records invalidate cohort structure and cannot increase observed acceptance.

Observed fractions are descriptive. Replication coverage reports the number of distinct valid builds and feedstock lots against a preregistered minimum; meeting that coverage gate is not inferential yield qualification. The v0.1 evaluator explicitly returns `inferential_yield_claim_available = false`.

Campaign cost is consumed as one digest-bound cost receipt. Cost per accepted part remains represented by its total-campaign-cost numerator and final non-scrapped accepted-part denominator; no successful-part-only mean is invented.


The replication gate uses `replication_eligible_builds` and the number of distinct valid feedstock lots actually referenced by valid builds. Unused lot records cannot satisfy the lot floor. Failed-before-inspection builds still count as attempted/eligible build replications when their identity and genealogy are valid. Duplicate receipt IDs invalidate the cohort.
