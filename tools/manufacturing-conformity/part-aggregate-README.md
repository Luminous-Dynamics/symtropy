# Part-level multi-characteristic conformity aggregation

Standalone Rust reference that aggregates qualified scalar measurement-conformity receipts into a scoped characteristic disposition. It is a deterministic evidence combiner, not a product-release system.

## Aggregation rule

For a manifest of mandatory characteristics bound to one product/part/lot/revision/operation:

- Any **verified**, exact-scope REJECT_NONCONFORMING child result makes the aggregate REJECT_NONCONFORMING.
- Otherwise, any missing, duplicate, stale, revoked, mismatched, indeterminate or evidence-incomplete mandatory child keeps the aggregate INDETERMINATE.
- CHARACTERISTICS_ACCEPTED is returned only when every mandatory characteristic has exactly one verified, current, scope-matching ACCEPT receipt.

Optional characteristics are preserved in the output but do not gate the mandatory set. Unrecognized extra reports are retained as extra references; they cannot stand in for a required characteristic. Duplicate reports are not resolved by assuming the newest timestamp is current. A valid successor/rework chain must be explicitly bound in a future manifest contract.

The result is CHARACTERISTICS_ACCEPTED, not PRODUCT_RELEASED. An external authorized quality function retains shipment/release authority.

## Dependence boundary

The deterministic conjunction of scalar guard-band results does not require independent measurement errors. A probability-based product risk calculation is intentionally absent. Marginal expanded uncertainties or coverage probabilities must not be multiplied to fabricate a product-level confidence claim. A future JCGM 106 mode requires a qualified joint distribution or conservative dependence model, including common-cause calibration/datum contributions and a declared risk budget.

## Build and run

    rustc --edition=2024 -D warnings --test tools/manufacturing-conformity/part_aggregate.rs -o /tmp/part-aggregate-tests
    /tmp/part-aggregate-tests

    rustc --edition=2024 -D warnings tools/manufacturing-conformity/part_aggregate.rs -o /tmp/part-aggregate
    /tmp/part-aggregate --verify
    /tmp/part-aggregate --csv

## Verification

- Exhaustive 4^3 truth table across ACCEPT, REJECT_NONCONFORMING, INDETERMINATE and EVIDENCE_INCOMPLETE for three required characteristics.
- Required missing/duplicate/scope-mismatched/revoked reports fail closed.
- Optional and unexpected extra reports remain visible without substituting for a required characteristic.
- Any verified rejection is preserved even when another mandatory characteristic is missing.
- Reordering scalar reports preserves canonical output bytes.
- Frozen 12-scenario result table, deterministic replay and source/output SHA-256 in CI.

## Claim and authority ceiling

The aggregator consumes references to upstream scalar reports. It does not authenticate their signatures, redo metrology, validate the parent uncertainty budget, certify a part, or release/shipment-authorize a product. It does not estimate joint consumer risk without a qualified dependence model.
