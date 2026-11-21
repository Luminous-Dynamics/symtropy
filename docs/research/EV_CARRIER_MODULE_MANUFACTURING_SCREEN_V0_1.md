# EV-CARRIER-MODULE-MFG-001 — Carrier module manufacturing screen v0.1

**Implementation:** `tools/ev-passenger-cost-frontier/src/carrier_module_factory.rs`  
**CLI:** `ev_carrier_module_factory_screen`  
**Input:** `fixtures/ev_carrier_module_processes_v0_1.json`  
**Base:** `EV-FACTORY-DESIGN-001` report from the existing factory frontier  
**Claim ceiling:** synthetic module-process capacity screen only

## Purpose

The module screen connects an electric carrier product architecture to the manufacturing system that must produce it. It consumes the existing 36-scenario EV factory report and adds only the incremental process route for a detachable UAV pod, AMR handoff module or integrated drone bay.

It does **not** create another factory cost model. Existing EV factory scenarios continue to own site, shift, availability, performance, base vehicle yield, factory architecture and general factory cost. The module overlay owns product-specific process times, process yield, maximum available cells, cell footprint, equipment, operators and utility use.

The current v0.1 screen is intentionally a *capacity and resource overlay*, not a complete module cost or production-release decision.

## Architecture / process comparison

The fixture contains:
- detachable UAV pod — fully populated **synthetic** process route for exercising station sizing;
- AMR handoff module — fully populated **synthetic** two-module-per-carrier route;
- integrated enclosed drone bay — production-intent process data deliberately left unknown.

The fixture does not assert that a drone pod or AMR can currently be manufactured at these rates. The synthetic process values are illustrative test inputs, not time studies, equipment quotations or industrial engineering estimates.

## Method

For target good carriers (Q_v), modules per good carrier (m), and module saleable yield (y_m):

[
Q_m = Q_v 	imes m
]

[
A_m = leftlceil rac{Q_m}{y_m} ightceil
]

where (Q_m) is required good modules/year and (A_m) is module manufacturing attempts/year.

For module process station (i), with cycle time (t_i) and productive minutes per cell-year (P) inherited from the base factory scenario:

[
C_{i,mathrm{required}} =
leftlceil rac{t_i A_m}{P} ightceil
]

The number of cells that can actually be sized is constrained by the declared maximum available cells (C_{i,mathrm{available}}):

[
C_{i,mathrm{sized}} =
min(C_{i,mathrm{required}}, C_{i,mathrm{available}})
]

[
K_i =
leftlfloor rac{P C_{i,mathrm{sized}}}{t_i} ightfloor
]

The route bottleneck is:

[
K_{mathrm{route}} = min_i K_i
]

[
Q_{mathrm{module,good}} =
leftlfloor K_{mathrm{route}} y_m ightfloor
]

[
Q_{mathrm{carrier,equivalent}} =
leftlfloor rac{Q_{mathrm{module,good}}}{m} ightfloor
]

The screen compares equivalent carrier capacity with the base factory scenario's target good carriers/year. A known shortfall blocks that configuration. Unknown cycle time, yield or available cell count withholds a complete capacity claim.

## Reported incremental resources

Per site, factory template, target volume and module configuration, the report emits:
- module attempts and good-module target per year;
- required, available and sized parallel cells per process;
- per-station attempt/good-module capacity and route bottleneck;
- equivalent carrier capacity and annual margin against the base factory output;
- additional operators per shift and active positions across declared shifts;
- additional floor footprint and equipment capex;
- module electrical and useful thermal energy per good module;
- inherited base factory production-release state and evidence gaps.

The base factory shift count is now explicit in its scenario report, allowing operator positions to be translated into active positions across shifts without guessing.

The overlay deliberately does not add base factory rent, shared overhead, vehicle assembly labor or general utility costs a second time. The existing factory model owns those costs. A later economics increment can price these incremental module resources using sourced local rates and measured work content.

## Run

From the repository root, first generate the base factory report:

```bash
cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml \
  --bin ev_factory_frontier -- \
  tools/ev-passenger-cost-frontier/fixtures/ev_factory_design_v0_1.json \
  > ev-factory-design-report.json
```

Then run the carrier module overlay:

```bash
cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml \
  --bin ev_carrier_module_factory_screen -- \
  ev-factory-design-report.json \
  tools/ev-passenger-cost-frontier/fixtures/ev_carrier_module_processes_v0_1.json
```

The workflow runs both binaries, checks the 36 base factory scenarios × 3 module configurations (108 overlay results), and retains the base and module reports, inputs, schemas and source hashes in one evidence artifact.

## Status semantics

- `passes_synthetic_screen`: every required module process input is present, station capacity covers base vehicle output, and the values are explicitly synthetic. This is arithmetic only.
- `passes_evidence_backed_screen`: process data is labelled measured or independently tested and capacity covers base output. This still does not grant production release.
- `blocked`: known station-cell capacity cannot support the declared good carrier throughput.
- `indeterminate`: any process yield, cycle time, maximum available cell count, staffing, footprint or utility field needed for the requested result is unknown.

**`module_production_release_ready` is always false in v0.1.** Module EOL acceptance, structural retention, vehicle integration, fire/thermal, HV isolation, environmental, EMC, cybersecurity, operator qualification and local regulatory release are separate gates. A capacity pass is not a production instruction.

## Paradigm shift and next work

The objective is to make the vehicle and its factory co-designable:
- use common datums, fasteners, HV/LV interfaces, diagnostic identities and replaceable modules across carrier families;
- optimize station count and production volume jointly with payload, energy, mission service and maintainability;
- qualify the module for removal, repair, upgrade and redeployment, not just first installation;
- preserve digital product/battery identity across suppliers, manufacturing, vehicle installation, maintenance and end-of-life;
- use actual takt studies, time studies, yield, rework and equipment quotations to replace synthetic values.

A detachable pod is not automatically more modular or cheaper: it must earn that claim through measured removal/installation time, repair-versus-replace cost, fleet downtime, cross-carrier compatibility, retention performance and supplier lead time.

## External research anchors

- The September 2026 mobile drone station trailer work (TSP-MDST) establishes a relevant trailer architecture comparator; the Luminous concept must differentiate on integrated EV energy, cargo/load qualification, shared capacity, manufacturing and lifecycle rather than mobile deployment alone. DOI: https://doi.org/10.1016/j.tre.2026.104975.
- The IEA Global EV Outlook 2026 notes that new battery manufacturing facilities can take more than five years to approach nominal output, reinforcing the need to distinguish nameplate capacity from yield-qualified output and actual process capability: https://www.iea.org/reports/global-ev-outlook-2026/electric-vehicle-batteries.
- The European Commission says the Battery Passport applies from 18 February 2027 to specified EV, light means of transport and industrial batteries; its updated data-point guidance provides a common information basis for battery identity and lifecycle traceability: https://eur-lex.europa.eu/eli/reg/2023/1542/oj and https://single-market-economy.ec.europa.eu/news/guidance-support-preparations-digital-batteries-passport-2026-08-21_en.

## Non-claims

The fixture does not demonstrate actual factory throughput, supplier access, local production cost, plant layout, process capability, module yield, physical product quality, vehicle homologation, airworthiness, battery/fire safety, commercial competitiveness or production readiness. No SOTA performance or cost superiority is claimed.


## Updated external benchmark: mobile drone modules must earn their production economics

A mobile drone-station trailer is not a new category by itself: the September 2026 TSP-MDST paper is already a published architecture comparator (DOI https://doi.org/10.1016/j.tre.2026.104975). A further August 2026 study jointly sizes and routes multi-truck/multi-drone fleets and identifies multi-visit delivery and cross-vehicle retrieval as major flexibility/cost mechanisms; it reports average 15.4–15.5% cost savings against four simplified fleet-sizing strategies on its own medium/large test instances (Xie et al., https://doi.org/10.3390/drones10090658). These figures are not our baseline and must not be transferred to our concept.

A major operational benchmark has also moved: Walmart announced on 5 November 2026 that its Zipline programme had completed more than **2 million deliveries** and would add **300 sites** under a multi-year agreement; the company describes eligible delivery in as fast as 30 minutes where available. This is a fixed retail-network deployment tied to nearby Walmart stores—not evidence that truck-mounted pods, shared aircraft or mobile trailers are already solved. Source: https://corporate.walmart.com/news/2026/11/walmart-expands-drone-delivery-network-with-zipline.

For the carrier module manufacturing benchmark, therefore compare:
- integrated enclosed bay installed in vehicle body;
- detachable pod transferable between vehicles and depot fixtures;
- mobile drone-station trailer with its own axle, coupling, restraint and utility boundary;
- ground-only AMR handoff module;
- no-module fixed-hub/truck-only controls.

Evaluate module removal/install time, cross-carrier compatibility, repair vs replace, good-module yield, line bottleneck, supplier lead time, module-specific EOL test cost, lifecycle downtime and additional site energy. The trailer or pod should not win merely because it transports aircraft.

## Local manufacturing and traceability update

InvestSA reports seven local vehicle OEMs and more than 500 automotive component manufacturers across Gauteng, the Eastern Cape and KwaZulu-Natal. It also describes APDP2, AIS and a NEV production investment allowance. This provides a route to investigate a local supplier and manufacturing ecosystem, not proof that the precise drone, battery, HV or safety-critical module supply chain is ready. Validate supplier process capability and regulatory applicability at part-number level. Source: https://www.investsa.gov.za/key-sectors/automotive-and-components/.

The European Commission's Battery Passport guidance says the passport obligation begins on **18 February 2027** for each EV battery, each LMT battery and each industrial battery over 2 kWh placed on the EU market or put into service. Not every data point is mandatory for every category or from day one. Dynamic data are expected to apply from the start, and a new passport is required on repurposing/remanufacture, with lineage to the previous passport. Do not presume that every UAV battery falls within EV/LMT/industrial categories; classify each pack and intended use against the Regulation and the relevant guidance. Sources: https://single-market-economy.ec.europa.eu/news/guidance-support-preparations-digital-batteries-passport-2026-08-21_en and https://single-market-economy.ec.europa.eu/single-market/digital-product-passport/eu-digital-product-passport-faq-batteries_en.

For Luminous, the product thread should keep the engineering configuration, serialized physical pod, aircraft, traction battery, UAV battery and installed vehicle configuration distinct, then bind each manufacturing/test/service event to the right artifact. Mycelix remains the identity/lineage/passport projection owner; Symtropy owns engineering qualification meaning. The passport operator remains legally responsible.
