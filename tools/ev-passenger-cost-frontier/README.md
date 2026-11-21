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

## Public component research

- Research dossier: [EV Component Public Benchmarks and Improvement Opportunities](../../docs/research/EV_COMPONENT_PUBLIC_BENCHMARKS_AND_IMPROVEMENT_OPPORTUNITIES_V0_1.md)
- Machine-readable source ledger: `fixtures/ev_component_public_benchmarks_v0_1.json`

This evidence is gathered from public market surveys, online retail listings, manufacturer specifications and official tariff/policy sources. It does not contact vendors or imply firm quotes. The ledger labels the exact evidence class and claim limitations for each source.

## Evidence and claim limit

Evidence classes such as `synthetic`, `estimate`, and `literature` are scenario-building inputs, not production quotes. Only manufacturer quotes, measured costs or audited cost records count as cost-release-grade in v0.1. A candidate is never production-feasible unless its price envelope, engineering gate, safety gate and vehicle-envelope evidence all qualify. Passing this CLI is arithmetic qualification only; it does not demonstrate manufacturability, safety, homologation or achievable MSRP.

The committed fixture deliberately uses synthetic costs and unqualified gates, so every candidate must remain `needs_evidence`. It illustrates the method, not an assertion that any design can be produced for the target price.


## EV-CARRIER-MODULE-MFG-001 — process capacity overlay

The `ev_carrier_module_factory_screen` binary consumes the existing `ev_factory_frontier` report and adds incremental manufacturing routes for carrier modules. It compares a synthetic detachable UAV pod, synthetic AMR handoff module and an intentionally unknown integrated drone-bay route.

Run after `ev_factory_frontier`:

```bash
cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml --bin ev_carrier_module_factory_screen -- \
  ev-factory-design-report.json \
  tools/ev-passenger-cost-frontier/fixtures/ev_carrier_module_processes_v0_1.json
```

The overlay inherits productive minutes/cell-year, shifts, target good-carrier throughput and base factory release gaps from the factory report. It computes module attempts from modules-per-carrier and module yield, sizes required cells against declared available parallel cells, identifies the process bottleneck, and reports additional operators, active shift positions, footprint, equipment capex and module energy per good unit. Unknown process data remain indeterminate; a known cell-capacity shortfall blocks the route.

This is an incremental capacity/resource overlay, not a second factory costing engine. Base building, general overhead, site labor/energy rates and vehicle assembly costs remain with `ev_factory_frontier`. Module production release remains false in v0.1, including synthetic-screen passes.

Research dossier: [EV Carrier Module Manufacturing Screen v0.1](../../docs/research/EV_CARRIER_MODULE_MANUFACTURING_SCREEN_V0_1.md).
