# EV Passenger Cost Frontier v0.1

A small Rust tool for turning a retail-price target into an explicit allowable vehicle cost envelope. It complements, and does not replace, the Symtropy EV mission/energy/charging screens.

## Run

```sh
cargo fmt --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml -- --check
cargo test --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml
cargo run --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml -- \
  tools/ev-passenger-cost-frontier/fixtures/ev_passenger_cost_v0_1.json
```

## Cost boundary

All monetary values are in the catalog currency. Every BOM line carries a quantity unit and unit-cost basis (for example 42 kWh of cells at ZAR/kWh plus a vehicle-set cost for enclosure and BMS). Each cost range must be **landed at the assembly facility** and include the applicable supplier price, currency conversion, inbound freight, duties and non-recoverable charges. Factory conversion and warranty/service reserve are per good vehicle. Fixed program investment is amortized over the declared number of vehicles. VAT and the dealer/distributor allowance are applied only in the retail waterfall.

- Retail ceiling = observed like-for-like baseline retail price × (1 − target undercut).
- Ex-VAT retail = retail ceiling / (1 + VAT), if the baseline includes VAT.
- OEM net revenue = ex-VAT retail × (1 − dealer/distributor margin).
- Maximum affordable COGS = OEM net revenue × (1 − target OEM gross margin).
- Good-vehicle BOM = sum(quantity × unit-cost range) / manufacturing yield.
- COGS = good-vehicle BOM + per-good-unit conversion + warranty/service reserve + fixed program investment / amortization units.
- Implied retail price inverts the same margin/tax waterfall for each COGS bound.

The model uses cost *ranges*. A low-case result cannot conceal a high-case shortfall.

## Evidence and claim limit

Evidence classes such as `synthetic`, `estimate`, and `literature` are scenario-building inputs, not production quotes. Only manufacturer quotes, measured costs or audited cost records count as cost-release-grade in v0.1. A candidate is never production-feasible unless its price envelope, engineering gate, safety gate and vehicle-envelope evidence all qualify. Passing this CLI is arithmetic qualification only; it does not demonstrate manufacturability, safety, homologation or achievable MSRP.

The committed fixture deliberately uses synthetic costs and unqualified gates, so every candidate must remain `needs_evidence`. It illustrates the method, not an assertion that any design can be produced for the target price.
