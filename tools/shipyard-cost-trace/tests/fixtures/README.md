# Frozen synthetic shipyard ledger fixture: golden-ledger-v0

Fixture implementation: `tests/reference_oracle.rs`. All identifiers, rates and transaction amounts are synthetic test data and are not representative of a real shipyard.

## Purpose

This fixture cross-checks the production ledger against a separately implemented test-only evaluator using linear scans, then pins independent hand-calculated outputs. It covers mixed stages, multiple work packages, labor rates, overtime, material quantities, scrap, rework, a partial/missing plan, credits, and a different build/configuration revision.

The independent evaluator deliberately does not call `shipyard_cost_trace::audit` or import production aggregation helpers. It is a valid-input oracle; malformed input remains covered by dedicated negative-control tests. Do not interpret this as a full independently implemented validator for every malformed ledger.

## Frozen inputs and manual expected results

Money is expressed in minor units under `ZAR-cent-2026-SA` with exponent 2. For this synthetic basis, 100 minor units = 1 major ZAR unit. Rate-card estimates are also expressed in minor units and remain estimates.

| Work package / axis | Planned | Simulated | Observed | Observed − planned | Rate-card outputs |
|---|---:|---:|---:|---:|---|
| hull-001 / welding hours | 10 h | 11 h | 7 h + 5 h = 12 h | +2 h | 1,000 / 1,100 / 1,200 minor units |
| hull-001 / welder overtime | Unknown | Unknown | 2 h | Unknown | 2 × 150 = 300 observed estimate |
| hull-001 / aluminium issued | 120 kg | Unknown | 125 kg | +5 kg | 30,000 planned; 31,250 observed estimate |
| hull-001 / aluminium scrap | Unknown | Unknown | 3 kg | Unknown | No rate admitted |
| hull-001 / rework hours | Unknown | Unknown | 3 h | Unknown | 3 × 100 = 300 observed estimate |
| hull-001 / painting hours | 10 h | Unknown | 8 h | −2 h | 800 planned; 640 observed estimate |
| hull-001 / CNC hours | Unknown | 5.5 h | Unknown | Unknown | No rate admitted |

The missing CNC plan and observation must remain absent; the simulated 5.5 hours must not be recast as observed work.

Direct financial actuals are separate from the rate-card estimates:

| Build | Work package | Synthetic transactions | Expected exact total |
|---|---|---|---:|
| hull-001 | block-01-weld | invoice 150,000 + credit −5,000 | 145,000 minor units |
| hull-001 | block-02-paint | invoice 80,000 | 80,000 minor units |
| hull-002 / configuration B | block-01-weld | invoice 2,345 | 2,345 minor units |

No estimate-plus-invoice grand total is defined or emitted. The hull-002 transaction must not be merged into hull-001, even though it uses the same currency basis and task name.

## Invariants exercised

- Planned, simulated and observed quantities remain distinct.
- Two additive welding contributions (7 h and 5 h) sum to 12 h; this fixture does not authorize cumulative-snapshot ingestion.
- Rate-card costs are partitioned by build, configuration, task, quantity axis, unit, stage, currency basis and rate-card revision.
- Missing values remain unknown.
- Credit notes use signed integer minor units.
- Different build/configuration revisions remain separate.
- The output must be invariant to reordering this valid event set.

## Corpus identity

The raw UTF-8 TSV (LF line endings, including its final newline) has SHA-256 `73fad1b9b94955ba018aa627787651a4e1a570bf754e02b161b1414c4f9ab295` at the commit that introduced this corpus. Recompute this digest whenever the TSV changes; do not substitute the Git blob SHA-1 for this digest.

## Reproduction

```sh
cargo test --locked --offline --manifest-path tools/shipyard-cost-trace/Cargo.toml --test reference_oracle
```

The exact commit, toolchain, runner and command must be recorded with any performance measurements. This fixture is correctness evidence for a synthetic accounting ledger, not evidence of productivity, cost savings, vessel fitness or certification.
