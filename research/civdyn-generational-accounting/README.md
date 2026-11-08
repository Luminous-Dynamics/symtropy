# CIVDYN Generational Accounting v0.1

A cohort-indexed accounting layer that composes the shared `civdyn-resource-ledger`. It records who receives service, who inherits physical and financial liabilities, and what capital state a future cohort receives.

It is not a second physical simulator and does not produce a universal intergenerational-fairness score.

## Paradigm

Separate four ledgers:
1. Physical resource stocks, service and obligations in native units.
2. Financial liabilities in an explicit currency and price/base year.
3. Cohort service received and unmet, reconciled against the physical ledger.
4. Inherited capital measures and shared resource stocks at birth/entry boundaries.

A financial discount rate changes only the reported present value of financial liabilities. It never modifies water, energy, food, maintenance obligations or capital-condition values.

## Synthetic matched experiment

Both policies start with 100 m³ in a basin. Two current cohorts each receive 10 m³ of household water. Both policies therefore have identical current household water service: 20 m³ served and zero unmet.

- `maintain_now`: exports no additional water, so the future cohort inherits 80 m³. Pump condition index is 0.95. No deferred maintenance or financial liability is assigned to the future cohort.
- `defer_maintenance_and_export_water`: exports 30 m³ to industry while preserving the same current household service. The future cohort inherits 50 m³, a pump-condition index of 0.60, a 40 kWh maintenance-energy obligation, and an illustrative ZAR 1,000,000 financial liability due in 2050.

The 40 kWh physical obligation and ZAR debt are separate. Neither can offset the other. The inherited financial obligation list references the existing debt record; it does not add its amount a second time.

All values, population assumptions and asset-condition indices are synthetic known answers.

## Mandatory controls

- Cohort service totals reconcile exactly (within the shared ledger's declared tolerance) to the authoritative `RegionOutcome`.
- Discount-rate sensitivity affects only financial present values.
- Splitting an identical cohort preserves regional service totals.
- Reassigning a liability changes its incidence, not its physical quantity.
- Future cohorts inherit from a declared preceding closing-year state.
- Financial obligations are referenced, not duplicated, in inheritance snapshots.
- Unknown cohort assignment is retained as an explicitly unassigned record with reason.

## Run

```sh
cargo fmt --manifest-path research/civdyn-generational-accounting/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path research/civdyn-generational-accounting/Cargo.toml --all-targets -- -D warnings
cargo check --locked --manifest-path research/civdyn-generational-accounting/Cargo.toml --all-targets
cargo test --locked --manifest-path research/civdyn-generational-accounting/Cargo.toml --all-targets
cargo run --locked --manifest-path research/civdyn-generational-accounting/Cargo.toml --bin cohort-inheritance-sweep
```

## Claim boundary

This is synthetic accounting software. It does not estimate real cohort welfare, determine a correct intergenerational discount rate, monetize ecological condition, or demonstrate that one policy is normatively superior. Empirical work requires a calibrated demographic model, licensed cohort data, domain review and declared welfare assumptions.
