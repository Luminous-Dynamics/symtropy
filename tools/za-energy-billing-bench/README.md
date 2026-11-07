# ZA Energy Billing Benchmark v0.1

Standalone Rust reference evaluator for the City Power 2026/27 Industrial Three-Part TOU budget-published schedule.

## Scope

The evaluator calculates only source-published and explicitly supplied components:
- seasonal peak / standard / off-peak energy charges;
- LV / MV / HV monthly service and network charges;
- measured demand, 70 kVA floor and 80%-of-three-highest 12-month ratchet;
- 6 c/kWh per-kWh Network Surcharge where the account evidence says it applies;
- reactive-energy excess where measured PF and kVArh satisfy the published conditions.

It does not guess the 2% Business/LPU surcharge base or VAT. Those are accepted only as actual assessed invoice line amounts with evidence references. If either is unknown, the report contains a known-component subtotal and no full invoice total.

It refuses use on non-City Power accounts, supply capacity at or below 100 kVA, missing approval/qualification evidence, stale source-profile identity, incomplete meter provenance and unresolved tariff periods. The Saturday/public-holiday 18:00–20:00 band remains blocked until the source-printed 18h00-20h0 endpoint is confirmed.

## Run

`cargo test --locked --manifest-path tools/za-energy-billing-bench/Cargo.toml`

`cargo run --locked --manifest-path tools/za-energy-billing-bench/Cargo.toml`

The CLI uses synthetic data only. Its explicit example Business/LPU and VAT values are illustrative invoice lines, not calculated from the published 2% rule and not a real customer bill.

## Calendar boundary

v0.1 accepts caller-qualified local month, day class, minute-of-day and interval duration. It deliberately does not parse timezone databases, daylight-saving transitions or public-holiday calendars. Those must be supplied by a separately qualified calendar adapter. Intervals crossing local midnight or a tariff boundary must be split before evaluation.

## Source / claim boundary

Profile: `ZA-CITYPOWER-INDUSTRIAL-TOU-2026-27-BUDGET-PROPOSED`

The source rates are the budget-published proposed 2026/27 schedule. Account eligibility and actual billing must be confirmed against the approved City Power account. This tool is a conformance reference, not production invoicing software, a tariff opinion, a dispatch optimizer or a saving claim.
