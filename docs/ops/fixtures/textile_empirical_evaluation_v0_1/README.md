# Textile empirical evaluation fixture v0.1

This directory defines a proposed field-study data contract for paired production-policy evaluation.

- **trial.schema.json** is a JSON Schema Draft 2020-12 shape contract.
- **trial.example.json** is synthetic-only, with illustrative counts and resource values.
- The protocol is **docs/ops/TEXTILE_EMPIRICAL_EVALUATION_PROTOCOL_V0_1.md**.

The example is not a measured pilot, not a valid preregistration, not a randomization record, and not evidence of factory savings. The synthetic evidence references deliberately cannot substantiate physical events.

## Invariants not expressible as simple JSON Schema shape checks

A Rust loader/validator must enforce checked arithmetic and fail closed if any of these do not hold:

1. completed new units = accepted + rejected new units;
2. accepted new units = fulfilled from new + unsold at window end + other accepted-new fates;
3. eligible existing inventory = issued + remaining eligible + other inventory fates;
4. total fulfilled = fulfilled from new + fulfilled from existing stock;
5. total fulfilled = on-time fulfilled + late fulfilled;
6. realized uncancelled demand = total fulfilled + stockout units;
7. if existing inventory is issued, a valid physical issue/transfer receipt is required for any non-synthetic study;
8. the pair set covers the baseline and comparison assignments within each preregistered pair, with the same frozen demand inputs and compatible profile/functional unit;
9. actual policy and policy deviations are retained separately from assignment;
10. resource measures marked missing or not applicable are never coerced to zero.

The JSON Schema only validates structure; it does not authenticate evidence, confirm calibration, enforce cross-record pairing, reconcile arithmetic, prove randomization, or estimate treatment effects.

## Required implementation sequence

1. Add a Rust parser/loader that reports path-specific failures and rejects unknown fields.
2. Add cross-field and cross-record invariants with overflow/negative-control tests.
3. Run an independent Draft 2020-12 validator over this fixture and invalid fixtures.
4. Add a report generator with profile-stratified absolute effects, uncertainty intervals, missingness, assignment deviations, and sensitivity analysis.
5. Evaluate against redacted real records only after privacy, data-rights, unit definitions, and the analysis plan are reviewed independently.

Never label the synthetic example as a benchmark result. It exists to make the intended data shape and accounting boundaries inspectable.
