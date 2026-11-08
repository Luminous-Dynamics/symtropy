# ZA Energy Billing Reference

Standalone, dependency-free Rust evaluator for deterministic City Power 2026/27 Industrial TOU billing-component conformance.

## Scope

- Requires City Power account identity, an Industrial TOU approval reference and supply capacity strictly greater than 100 kVA.
- Uses source-supplied local date, day class and local interval boundaries. Gregorian weekday/weekend classes are checked against the date; a weekday treated as a public holiday requires a date-specific evidence reference. The tool does not independently determine whether a date is an official South African public holiday or perform timezone conversion.
- Scans every minute inside each interval, rejecting any interval that crosses a tariff-band boundary even when its first and last minutes happen to share the same band.
- Preserves the Saturday 18:00–20:00 source-text ambiguity until an approved account/tariff confirmation reference is supplied.
- Calculates energy, service, monthly network, billed demand, reactive-energy and optional per-kWh network surcharge components.
- Requires exactly 12 contiguous prior monthly peak-demand records for the rolling demand ratchet.
- Requires full-month contiguous interval coverage: day 1 starts at 00:00, each interval begins at the prior interval's end, every date is present, and the last interval ends at 24:00 on the billing month's last day. Zero-kWh intervals are valid and must still be represented.
- Requires one day-class and one date-specific public-holiday evidence reference across every interval on the same local date. It does not infer South African public-holiday status or convert timezones.
- Accepts the Business/LPU account surcharge and VAT only as externally assessed invoice lines with evidence references. It does not guess their bases or hardcode VAT.
- Returns INCOMPLETE_INVOICE_RECONCILIATION and a known-component subtotal whenever a bill line is unknown.
- Includes a full synthetic July 2026 counterexample: moving 230 kWh of weekday load from winter peak to off-peak lowers the energy-charge component by R1,301.34 while total kWh and billed kVA remain unchanged; unknown Business/LPU and VAT lines keep the invoice total absent.

The 70 kVA minimum demand floor and the 70–1,000 kVA LV LPU service-connection range are not TOU eligibility rules. The budget's Industrial TOU clause 5.4.3 states a strict >100 kVA supply-capacity gate plus application/approval. A current secondary engineering-provider schedule describes a 70–1,000 kVA low-voltage LPU qualification band for TOU, so accounts in the 70–100 kVA range remain a source-conflict/manual-confirmation case; this evaluator does not admit them without the actual approved account tariff.

## Run

~~~
cargo test --manifest-path tools/za-energy-billing-bench/Cargo.toml
cargo run --manifest-path tools/za-energy-billing-bench/Cargo.toml
~~~

The CLI emits a machine-readable receipt from a synthetic, complete 31-day July 2026 trace containing 2,976 fifteen-minute intervals, including zero-kWh periods. The library also accepts nonuniform interval durations when they are contiguous and remain within a tariff band. Its FNV-1a-64 value is a reproducible non-cryptographic input fingerprint, not an authenticity signature. The reference is not a production customer billing system and makes no real-facility saving claim.


## Rolling demand-memory counterexample (#4194)

A separate integration test in `tests/demand_memory.rs` exercises the 12-month City Power ratchet through the public `calculate_bill` reference. It does not duplicate tariff parsing or introduce a battery/dispatch model.

The synthetic two-month case begins with prior monthly peaks of 250, 240 and 140 kVA (the other nine months are 140 kVA), so July 2026's 80%-of-top-three floor is 168 kVA. A 10 kVA shave of July's measured peak from 160 to 150 kVA does not change July's billed demand: the 168 kVA historical ratchet still binds. However, the July measurement then enters the August history while July 2025's 250 kVA peak rolls out. August's ratchet is 144 kVA without the shave and 141.333... kVA with it.

At the published LV demand rate of R461.28/kVA/month, that changes August demand charge by R1,230.08. With a deliberately synthetic R1,000 action cost, a current-month-only policy rejects the action, while an exact two-month enumeration selects it and reduces the two-month demand-cost-plus-action-cost objective by R230.08. This is a minimal counterexample to horizon-myopic control—not a measured facility saving, battery-performance claim or actual customer bill. The monthly fixtures deliberately leave assessed LPU/VAT lines unknown, so invoice totals remain incomplete.
