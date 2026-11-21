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


## EV-CARRIER-LIFECYCLE-001 — module lifecycle and serviceability screen

The `ev_carrier_lifecycle_screen` binary consumes `ev_carrier_module_factory_screen` output and adds module build cost, scheduled inspection burden, fault intervention repair-versus-replacement comparison, parts wait, technician hours and per-module maintenance time fraction.

Run after the module factory overlay:

```bash
cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml --bin ev_carrier_lifecycle_screen -- \
  ev-carrier-module-factory-report.json \
  tools/ev-passenger-cost-frontier/fixtures/ev_carrier_lifecycle_v0_1.json
```

The unscheduled event rate is calculated only from an explicit event count divided by an explicit module-year exposure denominator. The tool does not infer failures from synthetic task examples. A stale/unknown post-maintenance test cannot qualify a service path; the evaluator never grants return-to-service authority or production release. The single-module maintenance-time fraction is not fleet availability.

Dossier: [EV Carrier Lifecycle and Serviceability Screen v0.1](../../docs/research/EV_CARRIER_LIFECYCLE_AND_SERVICEABILITY_V0_1.md). Compose generic task semantics from [AERO-SERVICE-LOOP-001 #4056](https://github.com/Luminous-Dynamics/symtropy/issues/4056) and lifecycle comparisons from [EV-BENCH-001 #2164](https://github.com/Luminous-Dynamics/symtropy/issues/2164).


## BATTERY-SERVICE-NETWORK-001 — exact one-epoch pack allocation v0.1

The `battery_service_network_screen` binary is the first executable slice for [BATTERY-SERVICE-NETWORK-001 #6674](https://github.com/Luminous-Dynamics/symtropy/issues/6674).

Research dossier: [Battery Service Network Benchmark v0.1](../../docs/research/BATTERY_SERVICE_NETWORK_BENCHMARK_V0_1.md).

Run:

```bash
cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml --bin battery_service_network_screen -- \
  tools/ev-passenger-cost-frontier/fixtures/battery_service_network_v0_1.json
```

The v0.1 exact evaluator assigns at most one qualified pack to each mission in one simultaneous dispatch epoch. It enumerates assignments for at most six packs and six missions, maximizing service count, then minimizing energy surplus, then applying a deterministic identifier tie-break.

For each candidate pair it independently checks:
- exact pack/mission configuration identifier and configuration digest;
- pack identity, custody, and post-test evidence bound to the exact pack ID and configuration digest;
- source/producer ID, method ID, evidence reference, observation timestamp and explicit validity interval for every evidence envelope;
- evidence observed no later than the declared dispatch epoch, with the epoch inside the evidence validity interval (future observations are blocked; expired or incomplete validity windows are indeterminate);
- available state and fresh passed post-test evidence;
- usable energy against mission energy **including reserve**;
- peak-power evidence against the mission peak-power requirement;
- pack temperature against both mission and qualified pack limits.

Unknown/stale/incomplete evidence is indeterminate; known identity/configuration mismatch, future observation, non-available/quarantined state, failed post-test or exceeded energy/power/thermal envelope is blocked. Capacity cannot compensate for insufficient peak power. Each pack can be assigned only once in the common epoch. Evidence references and producer/method identifiers are declarative strings in this synthetic benchmark; cryptographic authenticity and authoritative identity/custody resolution remain external owner responsibilities.

The machine-readable input contract is `schemas/battery_service_network_v0_1.schema.json`; the frozen known-answer is `fixtures/battery_service_network_v0_1.json`. The focused workflow runs Rust formatting, tests, CLI evaluation and JSON assertions, and retains a report/hash artifact.

**Deliberate scope limit:** this is a static exact assignment and evidence gate, not a multi-period charging/rack queue optimizer, mobile battery-service routing model, degradation model, thermal battery model or aircraft dispatch authorizer. Those are the next layers in #6674. Synthetic fixture passes validate accounting/algorithm semantics only and do not establish real battery capability, flight safety or operational release.