# CEA Facility Benchmark Reference v0.1

A dependency-free Rust audit for a narrow but important boundary: facility electricity inputs, gross harvest, accepted edible harvest, rejected/spoiled/in-edible output, water make-up, useful thermal input and rejected heat.

It does not model plant physiology, crop growth, nutrient composition, diet adequacy, food safety, primary energy or lifecycle emissions. It refuses to call a synthetic fixture a measured facility.

## Run

```sh
cargo test
cargo run --release
cargo run --release -- --input fixtures/synthetic-lettuce-facility-v0-1.input
```

## Frozen synthetic positive control

The synthetic fixture contains:
- 2 m² facility area and a 365-day accounting period;
- 1,750,000 Wh total electricity across seven declared facility systems;
- 180,000 g gross harvest, reconciled as 120,000 g accepted edible output, 10,000 g rejected, 5,000 g spoiled and 45,000 g inedible residue;
- 1,200,000 ml make-up water;
- separate useful-heat and heat-rejection observations.

The hand-calculated facility ratios are:
- 14.583333 kWh/kg accepted edible output;
- 875.000000 kWh/m²-year;
- 10.000000 L make-up water/kg accepted edible output;
- 66.666667% of gross harvest accepted as edible output.

These values are intentionally synthetic arithmetic fixtures. They were chosen to exercise familiar lettuce-facility metric scales; they do not reproduce, validate or represent a specific vertical farm or the literature benchmarks.

## Accounting semantics

- Electrical subsystems are summed at the declared facility meter boundary.
- Thermal Wh_th are reported separately and never added to electrical Wh.
- Harvest mass must reconcile exactly.
- Energy per kilogram uses accepted edible mass, never gross biomass.
- Annualized facility energy is normalized to 365 days.
- The output preserves exact integer numerator quantities; derived ratios are rendered to six decimal places.
- Missing inputs, unknown keys, duplicate keys, unsupported evidence classes and inconsistent mass balances reject.

## How to compare with industry

A 2025 vertical-farming review reports current lettuce facility electricity around 10–18 kWh/kg and a projected technical range of 3.1–7.4 kWh/kg (DOI 10.1016/j.tsep.2024.103165). The lower range is a projected technical benchmark, not an achieved industry-wide result.

A 2025 cross-crop meta-analysis reviews 116 studies from 40 countries and 23 crop types and highlights wide differences in crops, facilities and system boundaries (DOI 10.1038/s44264-025-00091-z). A benchmark comparison must therefore match crop, edible basis, operating period and system boundary before ranking.

## Evidence ceiling

- Evidence class: synthetic.
- This is facility accounting, not agronomic validation.
- It is not a nutrient ledger; compose the existing Symtropy exact nutrient contribution path.
- It is not primary-energy or lifecycle accounting; compose the existing Symtropy energy-to-service / well-to-service owners.
- It does not establish that underground farms or extraterrestrial farms are energy-efficient or viable.
