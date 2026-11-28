# Paired virgin/recovered feedstock cohort evaluator

**Programme:** MAG-PAIRED-FEEDSTOCK-COHORTS-001  
**Issue:** #7917  
**Revision:** 0.1-draft

## Purpose

Compare virgin and recovered feedstock through the same specified manufacturing route using randomized, source-lot-blocked build pairs. This workstream connects route qualification (#7911) to manufacturing cohort accounting (#7776), without duplicating either evaluator.

A route can pass its own material acceptance policy and still be inferior to its virgin control. Conversely, a lower best-coupon Br value does not prove that a recovered route is inferior if the component-level function and manufacturing yield are non-inferior. The correct comparison is a matched cohort.

## Paired design contract

The frozen scope binds material family, feedstock form, route, recipe, process revision, target geometry, orientation, magnetization, measurement setup and acceptance policy. Virgin and recovered feedstock use different source lots and eligibility receipts, but the same process contract.

The independent unit is the source-lot pair and randomized build block. Repeated scans or coupons from one build are not independent lot/build replication. Every block records:
- unique virgin and recovered build IDs;
- lot-pair identity and source lot IDs;
- randomized run order;
- randomization receipt and seed commitment;
- blinded sample IDs;
- process/build receipts and method/calibration/uncertainty lineage.

PEM, MEX and LPBF remain distinct route strata. Results do not transfer between routes or material families.

## Denominators, failures and rework

The initiated-build manifest is the denominator source. Every initiated build must have an outcome record, including Fail, MissingInspection, NotEvaluable and Scrapped. Missing or extra build records cannot be silently dropped.

First-pass and final dispositions are separate. A build that fails first pass and is later recovered by rework may count in final accepted output, but it never becomes a first-pass qualified build. Rework requires its own receipt. First-pass yield and final accepted fraction are reported separately.

## Paired decision semantics

Each completed pair is evaluated on the same preregistered physical metric definitions and units. Hard limits are non-compensatory. Recovered material must be robustly non-inferior on every mandatory metric in every required pair. A robust benefit must be uncertainty-separated and persist across all preregistered pairs; one exceptional specimen cannot establish route advantage.

The evaluator reports:
- first-pass and final accepted counts by condition;
- finite-cohort first-pass yield;
- per-block paired metric intervals and non-inferiority decisions;
- robustly improved metrics that remain improved across every pair;
- blockers and claim ceiling.

This v0.1 evaluator does not fit a hierarchical statistical population model, estimate process capability, or infer generalization beyond the observed finite cohort. A confirmatory sample size must be powered from pilot variance and a preregistered practical margin. The proposed feasibility floor of three independent source-lot pairs and three build pairs per lot is not universal statistical sufficiency.

## Integration

- #7911 supplies route eligibility and feedstock assays.
- #7776 supplies manufacturing denominator, rework and cohort semantics.
- #7798 supplies matched experimental-policy and holdout principles.
- #7792 supplies evidence-derived M1-M5 maturity.
- #7773 supplies specimen-level as-built acceptance.
- #7791 supplies recovery passport and origin genealogy.
- #7887 supplies full duty-cycle acceptance.
- #7802 supplies system architecture comparison.
- #7912 supplies element-specific virgin g/MWh after accepted service.

## Claim ceiling

The synthetic fixture is SyntheticControl throughout and must remain Indeterminate. Passing tests validates software semantics only. No recovered-route superiority, physical yield, M4/M5 qualification, production capability, LCA or commercial advantage is claimed.
