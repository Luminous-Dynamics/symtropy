# Factory Accepted-Output Economics — Stage 0

**Owner:** Symtropy [FACTORY-ACCEPTED-OUTPUT-ECONOMICS-001 #5966](https://github.com/Luminous-Dynamics/symtropy/issues/5966)  
**Parent:** [MATERIAL-ACCOUNTED-ROUTES-001 #5875](https://github.com/Luminous-Dynamics/symtropy/issues/5875)  
**Composition:** matched routes #5432/#5434, receipt conformance #5879/#5912, cost semantics #2397 and VVUQ/quality owners #2996/#3007/#3009/#3000/#3072  
**Status:** dependency-free Rust prototype; synthetic inputs only.

## What this computes

The evaluator composes three distinct inputs:

1. Route admissibility: ELIGIBLE, INELIGIBLE or UNKNOWN.
2. Cost-item account: exact batch reference, bounded integer minor-unit amount, currency, price date, amount unit, evidence class, explicit scope and source reference.
3. Final disposition of each unique output identity: first-pass release, release after rework, scrap, WIP, awaiting inspection or indeterminate.

It reports cost per **unique released output**, never per process completion or rework event. Rework remains a cost and a disposition-history effect; it does not create another output identity. Only RELEASED_FIRST_PASS and RELEASED_AFTER_REWORK count toward the release denominator.

## Fail-closed rules

- Ineligible or unknown routes are not economically ranked.
- Unknown in-scope cost items produce COST_INCOMPLETE; unknown is not zero.
- An explicitly out-of-scope cost is excluded, but remains visible in the input fixture.
- Currency, price date and amount unit must match the case exactly. No FX or unit conversion is guessed.
- Cost intervals use integer minor-unit lower/upper bounds; the evaluator emits exact numerator/denominator pairs, not rounded floating-point prices.
- Zero released outputs produce ZERO_RELEASED and no finite unit-cost ratio.
- Output identities, release references and cost-item identities cannot be reused.
- The component report preserves both cost category and evidence class; estimated, quoted, paid, public, measured and synthetic evidence are not conflated.
- Amounts are already attributed to the exact batch; the prototype does not calculate depreciation, NRE amortization, asset life, utilization or repeated-run allocation. Those must be evidenced and supplied by an upstream accounting policy.\n- The evaluator does not authenticate sources or release products.

## Frozen corpus

- 20 synthetic cases.
- 50 cost-item rows.
- 46 final output-disposition rows.
- 27 Rust unit tests.
- Known-answer dispositions include 15 computed cases, 2 zero-release cases, 1 ineligible route, 1 unknown route and 1 incomplete cost account.

The hand-calculated expected-results file is separate from candidate output. Tests also attack duplicate output/release/cost identities, mixed currency/date/unit, missing release evidence, incomplete or reversed cost intervals, total overflow, negative or omitted cost items, component-total reconciliation and output-order dependence.

## Run

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
cargo run --locked -- --summary
cargo run --locked -- --breakdown
```

The focused workflow pins Rust 1.96.0 and archives exact-run source identity, toolchain versions, frozen fixture hashes, a per-case summary and a category/evidence-class cost breakdown.

## Claim ceiling

All current values and source references are synthetic. The evaluator does not turn #5434 dimensionless route-cost units into ZAR; it is not a supplier quote, invoice, measured tariff, industrial labour rate, physical yield, ROI calculation, market estimate or production-release authority. No conclusion about South African competitiveness or superiority to an incumbent system follows from this corpus.
