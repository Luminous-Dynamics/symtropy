# Factory Accepted-Output Economics v0.1

See [FACTORY-ACCEPTED-OUTPUT-ECONOMICS-001 #5966](https://github.com/Luminous-Dynamics/symtropy/issues/5966) for the full benchmark contract.

This prototype is a synthetic accounting oracle for pump/valve routes. It is not a cost model for real factories, a supplier quote, ROI study, quality authority or production release system.

## Accounting contract

1. Route eligibility is evaluated before economics. INELIGIBLE and UNKNOWN routes cannot be ranked by cost.
2. One output identity contributes at most once. Rework changes the final disposition of the same output; it never adds a second unit.
3. Each case declares its positive started-unit count; each unit has exactly one final disposition bound to the exact batch. The declared count must equal the complete final-disposition set, so silently omitted WIP/scrap units cannot inflate yield. Only RELEASED_FIRST_PASS and RELEASED_AFTER_REWORK count in released output. WIP, SCRAP, AWAITING_INSPECTION and INDETERMINATE do not.
4. Unknown in-scope costs make the economic result incomplete. Unknown out-of-scope costs are excluded only with their explicit OUT_OF_SCOPE classification.
5. All cost items must match the case's exact batch reference, currency, price date and amount unit. No implicit FX, unit or date normalization occurs.
6. Cost intervals are accumulated as integer minor-unit bounds. Cost per released unit is emitted as an exact numerator/denominator pair, not a floating-point estimate.
7. Zero released output has a defined ZERO_RELEASED disposition and no finite cost-per-released-unit ratio.
8. Output and cost-item identities are unique within each evaluated case, not globally across counterfactual cases. A batch-level release evidence reference may be shared by distinct output records only when its externally verified scope covers those units; the synthetic adapter does not authenticate coverage.
9. Stage 0 source refs and evidence classes are synthetic metadata. No issuer/signature or source truth is authenticated.

## Run

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo run --bin factory-accepted-output-economics
```

The frozen corpus contains 20 cases, 50 cost-item rows and 46 final output-disposition rows, plus 35 Rust unit tests. The CLI emits one report row per case plus a deterministic summary. The test suite also mutates identities, currencies, dates, units, missing-cost state, release references, negative amounts and overflow conditions.

## Evidence class

All fixture rows are synthetic. The prototype does not convert synthetic relative route cost units from #5434 into money and does not contain real South African market prices, supplier quotes, labour rates, tariffs or measured factory costs.
