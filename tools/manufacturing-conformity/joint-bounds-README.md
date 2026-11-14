# Dependence-robust part-level conformity bounds

This standalone Rust reference aggregates qualified marginal conformance-probability enclosures for all required characteristics of one part/lot/operation. It uses Frechet bounds and makes **no independence assumption**.

## Mathematics

For required characteristics with conformity events A_i and exact marginal probabilities p_i:

    max(0, sum_i p_i - (n-1)) <= P(intersection_i A_i) <= min_i p_i

If only marginal probability enclosures [l_i,u_i] are available, a conservative joint enclosure is:

    joint_lower = max(0, sum_i l_i - (n-1))
    joint_upper = min_i u_i

Decision:
- ACCEPT only when joint_lower >= the approved part-level acceptance threshold.
- REJECT_NONCONFORMING only when joint_upper <= the approved part-level rejection threshold.
- INDETERMINATE otherwise.
- EVIDENCE_INCOMPLETE for missing/stale/mismatched/unqualified evidence or an incomplete required-characteristics manifest.
- INVALID_INPUT for malformed identities, probability enclosures, digests, or policy declarations.

The rejection threshold must be strictly below the acceptance threshold. The policy is externally approved and scoped to the product family/context.

## Why this matters

For five characteristics each with P(conform)=0.9:
- Frechet bounds are [0.50, 0.90].
- Multiplying marginals assumes independence and gives 0.59049.
- With a 0.60 reject threshold, the independence product would imply rejection while the valid dependence-robust bounds require INDETERMINATE.

For twenty characteristics each at 0.99:
- Frechet bounds are [0.80, 0.99].
The lower bound degrades with characteristic count because dependence is unknown. That is not a defect in the calculation; it reveals how much information is missing to make a reliable part-level claim.

## Evidence contract

The request binds an exact required-characteristics manifest, part/lot/operation, decision context and approved policy. Every marginal must match its manifest ID and carry:
- exact characteristic, measurand/unit, specification revision;
- posterior ID and posterior model revision;
- marginal probability enclosure and validity interval;
- posterior, model-calibration, measurement, MSA, calibration, uncertainty and specification evidence states;
- source SHA-256 references for every evidence family.

Instrument calibration, MSA and statistical posterior calibration are separate evidence families. A normalized histogram or model score is not enough to establish a calibrated posterior. No characteristic can be dropped or averaged away. Rework and remeasurement must be represented by new/superseding upstream receipts, not by rewriting this manifest's history.

## Validation

Tests cover:
- exact two-, five-, and twenty-characteristic known answers;
- a concrete negative control where independence multiplication would incorrectly reject;
- unknown/missing required characteristics;
- stale model-calibration evidence, unapproved policy and wrong scope;
- permutation invariance;
- monotonicity when adding required characteristics or widening marginal intervals;
- binary-event atom constructions that attain both Frechet bounds for small n;
- invalid probability intervals and threshold policies;
- deterministic replay.

The synthetic fixtures verify the aggregation mathematics. They do not establish that real posterior intervals are calibrated or that process dependence has been measured.

## Build and run

    rustc --edition=2024 -D warnings --test tools/manufacturing-conformity/joint_bounds.rs -o /tmp/joint-bounds-tests
    /tmp/joint-bounds-tests

    rustc --edition=2024 -D warnings tools/manufacturing-conformity/joint_bounds.rs -o /tmp/joint-bounds
    /tmp/joint-bounds --verify
    /tmp/joint-bounds --csv

## Research ladder

1. Baseline with Frechet bounds when dependence is unknown.
2. Independently validate the scalar posterior calibration receipt on whole parts/lots/campaigns.
3. Add a qualified multivariate posterior or copula only with held-out tail-dependence and calibration evidence.
4. Preserve the Frechet enclosure beside every model-conditioned narrower estimate.
5. Evaluate part-level consumer/producer risk, decision coverage and indeterminate rate on physical part/lot groups.

## Claim and authority ceiling

This is a dependence-robust mathematical bound over supplied marginal probability enclosures. It does not validate the upstream posterior model, infer real dependence, certify a part or production lot, or authorize shipment/release. Existing authorized quality authority remains responsible for final disposition.


Every marginal posterior must explicitly carry the same exact:
- part ID;
- lot ID;
- operation ID;
- decision-context ID;
- decision timestamp.

It is not sufficient for the aggregate manifest alone to name the part while an unscoped marginal score is attached. A posterior fitted to a different operation or taken at a different decision time returns EVIDENCE_INCOMPLETE. These fields are emitted in every marginal receipt row.
