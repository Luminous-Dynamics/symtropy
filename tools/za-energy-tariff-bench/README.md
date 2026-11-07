# ZA energy tariff billing reference

Standalone deterministic Rust reference evaluator for the City Power Johannesburg budget-published FY2026/27 Industrial Three-Part Time-of-Use schedule.

## Scope

The tool calculates reference-schedule components independently of any optimizer:
- seasonal energy charges by local tariff band;
- 6 c/kWh network surcharge as a separate line item;
- monthly service and fixed network charges;
- demand floor and 12-month top-three ratchet;
- reactive-energy charge under the source-stated PF / 30% conditions;
- optional 2% business/LPU surcharge only when a basis is supplied; it remains an assumption until explicitly confirmed against the actual account/invoice;
- VAT only when both an explicit rate and the taxable basis are confirmed.

It rejects:
- time intervals that cross tariff bands (split them before evaluation);
- intervals that cross local midnight;
- Saturday/public-holiday 18:00–20:00, because the budget source prints `18h00-20h0`. This source ambiguity must be clarified before billing-grade use.

The evaluator distinguishes a reference scenario from an invoice-equivalent claim. It withholds the latter unless:
- the distributor is City Power Johannesburg;
- supply capacity exceeds 100 kVA;
- industrial TOU application / approval is confirmed;
- the actual account tariff/contract is confirmed;
- VAT rate and taxable basis are confirmed, and all pre-VAT charge bases are calculable;
- any applicable 2% surcharge basis is explicitly resolved and marked confirmed.

The budget-published energy charges exclude the 6 c/kWh network surcharge, 2% business/LPU surcharge and VAT. Fixed network charge and 6 c/kWh surcharge are separate charges.

## Run

```sh
cargo test --manifest-path tools/za-energy-tariff-bench/Cargo.toml
cargo run --manifest-path tools/za-energy-tariff-bench/Cargo.toml -- tools/za-energy-tariff-bench/fixtures/synthetic-citypower-winter-demand-ratchet.json
```

## Fixture interpretation

The two cases use the same total energy (200 kWh):
- before: 100 kWh winter peak + 100 kWh off-peak, measured maximum 95 kVA;
- after: 200 kWh off-peak, measured maximum 80 kVA.

With prior top-three demand values [125,125,125] kVA, the 80% ratchet floor is 100 kVA. Billed demand remains 100 kVA in both cases. The 100 kWh shift saves R565.80 on the winter energy-charge component only, before fixed charges, surcharges and VAT. It does not reduce this case's demand charge.

These are synthetic calculations; they do not describe a real site or reproduce an invoice.

An assumed surcharge basis may be used for sensitivity analysis, but it does not create an invoice-equivalent total. VAT is withheld when the surcharge itself is not calculable from admitted inputs or its taxable basis is not confirmed.

## Evidence boundary

Source profile and limitations: `../../docs/research/cavern/ZA_ENERGY_MARKET_SIGNAL_REFERENCE_V0_1.md`.
Parent issue: [#4081](https://github.com/Luminous-Dynamics/symtropy/issues/4081).

This tool is not a NERSA/municipal tariff engine, invoice adjudicator, public-holiday calendar, tax adviser, real-time price feed or carbon-intensity estimator. An actual account contract and current invoice override the budget reference.