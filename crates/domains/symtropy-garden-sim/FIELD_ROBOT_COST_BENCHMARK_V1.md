# Field-Robot Service Cost Model — Benchmark Contract V1

**Status:** implementation/benchmark contract on a stacked draft PR. All currently authored operating values are synthetic; no local farm costs, ROI, yield change, or purchase recommendation is established.

## Paradigm

Model cost of a successfully delivered farm service, conditioned on field geometry and operations—not robot sticker price or theoretical hectares-per-day. Separate productive machine time, nonproductive within-field machine time, operator labor, external route logistics, mobilization, maintenance, recovery, and annualized capital. Keep the no-new-robot/manual/conventional baseline eligible.

This module computes component-wise costs only. It does not choose a cross-candidate winner or implement a second Pareto algorithm; the existing Symtropy garden portfolio comparator owns that comparison after upstream constraints are assessed.

## Contract and cost boundaries

- All monetary values use non-negative integer minor currency units under one explicit uppercase three-letter currency code (example: ZAR minor units). No implicit FX conversion or mixing currencies.
- Exact integer fields are serialized as decimal strings to avoid JSON consumers rounding large integers.
- Annualized capital = ceiling((acquisition cost − salvage value) / useful life).
- Productive/nonproductive machine-time cost = ceiling(minutes × machine cost per hour / 60) per component.
- Operator cost = ceiling(total operator-labor minutes × labor cost per hour / 60). Supervision minutes are a subset of operator-labor minutes, not added again. Aggregate labor minutes may exceed one machine's annual calendar time when multiple workers contribute; only a single machine's runtime is bounded by the annual machine-time envelope.
- Maintenance = ceiling(total machine minutes × cost per 100 machine-hours / 6000).
- External route logistics = ceiling(annual route metres × cost per kilometre / 1000).
- Mobilization = missions/year × cost/mission.
- Recovery = recovery events/year × cost/event.
- Total annual cost is the checked sum of those separately reported components.
- Cost per completed service unit = ceiling(total annual cost / completed service units).
- Cost per hectare, when an area denominator is supplied = ceiling(total annual cost × 10000 / completed square metres).

**Avoid double-counting:** nonproductive machine minutes describe machine time within the declared work boundary (for example repositioning/turning/setup). Route distance/logistics describes the separate route/mobilization boundary to and between fields/depots. Implementations must document the boundary before adding local values.

Field count and access-point count are returned as context metadata; they are not multiplied into costs as arbitrary coefficients. Their effects must be represented through measured route distances, mobilization, setup, turning/nonproductive time, or calibrated distributions. Missing geometry is not replaced with an assumed average.

## Evidence semantics

Required evidence domains: site geometry, completed service output, capital cost, machine operating cost, travel/logistics, labor cost, and maintenance/recovery.

- Source-referenced inputs require a nonempty source reference and 64-hex SHA-256 syntax. This kernel does **not** fetch or authenticate the reference or independently recompute its digest.
- Estimated and synthetic-fixture inputs permit a calculation but mark the evidence disposition provisional.
- Missing, stale, conflicting, duplicated, malformed, or absent evidence blocks the output.
- A structurally calculated cost is not a validated service outcome or ROI.

## Provider service coverage — bounded extension

The separate `provider_coverage` module calls the existing `field_robot_cost` kernel and adds no duplicate cost equation or cross-candidate optimizer. It compares an explicitly scoped provider-wide annual cost envelope with:
- primary-service receipts calculated from **paid completed units × the declared fee per unit**;
- receipts from other service lines, recorded separately so transport or another activity cannot silently disappear into one number;
- total declared receipts, modeled annual cost, difference, and unpaid completed primary-service units.

Hard gates bind scenario, scope, period, currency and primary paid units to the canonical cost input. Missing, stale, conflicting or malformed revenue evidence blocks the result. Arithmetic uses checked integer operations and exact decimal-string serialization. A separate verifier validates the independent 15-case inventory and test-function names, without calling either evaluator.

**Cost-boundary rule:** the cost input must cover the same provider/asset annual envelope as all receipt streams being counted. Do not compare multiple-service receipts against a cost model that includes only one task; this module does not allocate shared costs or deduce the cost of each service line. Other-service receipts are not profit, subsidy treatment is not implemented, and this is not a liquidity, affordability, price-fairness, ROI or business-viability assessment. Source references and digest shape are not authenticated here. All included examples are synthetic.

## Benchmark inventory

The frozen field-robot cost inventory is `fixtures/field_robot_cost_cases.v1.tsv` (25 cases). The Rust tests include a hand-calculated baseline, route-distance sensitivity, no-robot baseline, exact ceiling arithmetic, currency/schema/time/output blockers, evidence-quality controls, overflow controls, multi-operator labor, and exact-integer JSON round-trip. Its independent verifier does not import or invoke the cost evaluator and emits `STRUCTURE_VALID_ONLY`, not an economic result.

The provider-coverage inventory is `fixtures/provider_coverage_cases.v1.tsv` (15 cases) with a distinct integration-test file and independent structural verifier. Its oracle cases include exact coverage and shortfall arithmetic, unpaid completed units, ancillary-revenue separation, identity/scope/period/currency mismatch, stale/missing/malformed evidence, blocked cost input, and overflow. The manifest check only binds IDs to test names; the test suite must run separately. Both inventories are synthetic test inputs, not field data.

## Target metrics

| Metric | Target / requirement |
|---|---|
| Hand-calculated cost oracle | 100% exact match |
| Hard input/evidence blockers | Zero false accepts on the frozen synthetic corpus |
| Currency/unit conversion | Zero implicit conversions |
| Reproducibility | Identical canonical input and version produce identical component output |
| Decision uncertainty | If input scenarios cross break-even, report sensitivity; do not manufacture one certain ROI |
| Portfolio comparison | Delegate to Symtropy's existing Pareto comparer; no composite score here |
| Farm transfer claims | No claim until local field/service/repair evidence and held-out evaluation exist |

## Research motivation

A 2026 open-access study of 61 maize fields in northwestern Germany reported that field size and route/logistics significantly affected modeled autonomous robot cost. The reported values are not transferred to South Africa. They motivate measuring field sizes, fragmented plot counts, access points, route distance, seasonal utilization, and repair lead time before inferring break-even.

Reference: *Structural cost modeling and sensitivity analysis of autonomous field robot operations*, Smart Agricultural Technology 13 (March 2026), https://doi.org/10.1016/j.atech.2026.101913.

## Current limitations and next steps

1. This first version does not estimate crop yield, crop quality, task-failure probability, seasonal utilization, stochastic repair cost, discount rate, tax, or residual-value uncertainty. Add these only with explicit assumptions/ranges and independent oracle fixtures.
2. Cost-rate evidence is source-referenced, not authenticated; no external quotes, supplier services, labor rates or exchange rates are embedded.
3. The annual model accepts measured/declared minutes, route distance and completed outputs. It does not infer them from a map or robot datasheet.
4. Expand to a bounded distribution/sensitivity layer using Symtropy scenario/value-of-information ownership. Do not clone the Pareto comparator.
5. A source-qualified local profile for Southern Africa requires real field geometry, verified cost/rate sources, service history, and enough seasonal observations to characterize utilization and downtime.

No result in this benchmark establishes cost savings, a profitable purchase, yield improvement, environmental superiority, machine safety, or field readiness.
