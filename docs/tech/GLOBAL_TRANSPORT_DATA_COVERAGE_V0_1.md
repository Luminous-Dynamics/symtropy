# Global Transport Data Coverage and Benchmark Portfolio v0.1

**Reviewed:** 2026-10-16  
**Status:** source-discovery and qualification plan only; not evidence that listed datasets have been acquired, licensed, parsed, or qualified.  
**Tracking issue:** [TRANSPORT-DATA-GLOBAL-001 #1667](https://github.com/Luminous-Dynamics/symtropy/issues/1667)  
**Optimizer benchmark ladder:** [TRANSPORT_BENCHMARK_LADDER_V0_1.md](TRANSPORT_BENCHMARK_LADDER_V0_1.md)

## 1. Mission: global reach without false uniformity

Expand transport research beyond a single-country pilot by admitting each geography at the highest evidence level its sources support. A worldwide candidate network does not imply worldwide service coverage, OD demand, timetables, route capacity, costs, reliability, or permission to operate.

The paradigm shift is a **global federation of regional evidence-backed cases**: common optimizer contracts and outcome metrics, source-specific adapters and units, locally appropriate operating semantics, and explicit coverage gaps. Add regions when their evidence passes the relevant gate; do not wait for every country, and do not impute live operational facts into data-sparse areas.

## 2. Evidence tier is scoped to each claim

| Tier | Evidence available | Permitted uses | Not established |
|---|---|---|---|
| G0 | Rights-reviewed network/facility geometry and schema/CRS | Topology exploration; candidate paths | Current condition, schedule, capacity, traffic, or safe operation |
| G1 | Network/connectivity indicators and source-defined link relationships | Corridor discovery and connectivity context | Verified schedules, live capacity, OD flow or costs |
| G2 | Official mode totals, port statistics or national/regional aggregate series | Plausibility checks and aggregate comparison | Route-level allocation or operational feasibility |
| G3 | Rights-reviewed OD/commodity/mode data with a documented network crosswalk | Scoped regional OD scenario planning | Missing service times/capacity; any unknown must stay unknown |
| G4 | Corridor-level schedules, resource capacities, dwell, costs and/or observed reliability with clear time coverage | Corridor-calibrated planning and scenario tests | Generalization outside the supported corridor/period |
| G5 | Frozen empirical evaluation, predeclared holdouts, baselines, uncertainty analysis and independent reproduction | A narrowly stated, validated performance claim | Automatic transfer to other modes, jurisdictions or operating regimes |

A source may be G0 for geometry and G2 for annual volumes while remaining unqualified for schedules and physical capacity. There is no single country grade or universal transport-quality score.

## 3. Initial global and regional source portfolio

Status for every entry below is **discovered / access review required**, unless an artifact-specific manifest later records otherwise.

| Geography | Candidate source(s) | Candidate evidence | Important limitations before use |
|---|---|---|---|
| Global topology | [OpenStreetMap](https://www.openstreetmap.org/copyright) | Road, rail, facility and other mapped-feature geometry | ODbL attribution/share-alike obligations require review; topology does not prove service, capacity, condition or permission |
| Global maritime | [UNCTAD LSCI](https://unctadstat.unctad.org/datacentre/reportInfo/US.LSCI), [PLSCI](https://unctadstat.unctad.org/datacentre/reportInfo/US.PLSCI) | Quarterly country/port liner-network connectivity indicators | Connectivity is not a full OD matrix or real-time capacity |
| Global container ports | [World Bank CPPI 2020–2024](https://www.worldbank.org/en/topic/transport/publication/cppi-2024) | Vessel time-in-port based comparison and trends | Container ports only; not entire multimodal network, live berth availability or a substitute for source-level operations data |
| Global country context | [World Bank LPI](https://datacatalog.worldbank.org/infrastructure-data/search/dataset/0038649/logistics-performance-index) | Survey-derived country logistics context (published waves 2007–2023) | Country-level indicator with uncertainty and a limited domain of validity; not actual route travel time or physical capacity |
| United States | [BTS/FHWA Freight Analysis Framework 6](https://www.bts.gov/faf) | 2022 base-year regional/state OD flows, commodity, mode, tonnage and value estimates | Statistical estimates, not a live operational network or route capacity |
| European Union | [Eurostat Transport](https://ec.europa.eu/eurostat/web/transport) | Road, rail, maritime, inland waterways, air and pipeline statistics; passenger/freight activity, modal split and intermodal indicators | Geography and definitions vary by series; aggregates cannot be elevated to route-level capacity |
| Australia | [BITRE freight statistics](https://www.bitre.gov.au/statistics/freight), [2026 freight chapter](https://www.bitre.gov.au/freight-chapter-yearbook-2026) | Multi-mode freight activity and selected corridor/intermodal indicators | Inspect the precise table, period, method, and operational coverage independently |
| Canada | [Transport Canada Key Transportation Indicators](https://tdih-cdit.tc.canada.ca/en/dashboard/key-transportation-indicators) | Downloadable air, marine, road, trade and selected cross-border/rail indicators | Validate each file's geographic and time coverage and whether it is an OD series or aggregate |
| ASEAN | [Quarterly goods trade by transport mode](https://data.aseanstats.org/trade-mot-quarterly) | Trade value by commodity and mode at national frontier; source states CC BY 4.0 | Customs mode is not end-to-end physical movement, shipment tonnage, link capacity or cost; check current record metadata on acquisition |
| Brazil | [Integrated Transport Planning open data / PNL 2050](https://pit.infrasa.gov.br/dados-abertos/) | Candidate geospatial modal axes and OD matrices; release is progressive | Review exact files, license/access, methodology, units, network crosswalk and forecast-vs-observation distinctions |
| Japan | [MLIT transport statistics](https://www.mlit.go.jp/k-toukei/) | Recent rail, aviation and port monthly statistics | Check precise series semantics, translation, geographic granularity and use rights individually |
| India | [India Open Government Data — major-port traffic](https://www.data.gov.in/catalog/traffic-handled-major-ports-india) | Historical throughput aggregates for major ports | Check exact resource vintage, currentness, units and NDSAP reuse/redistribution permissions; not OD, berth availability or live port capacity |
| China | [National Bureau of Statistics annual freight communiqué](https://www.stats.gov.cn/english/PressRelease/202502/t20250228_1958822.html) | National-mode freight totals and tonne-kilometre plausibility checks | Aggregate annual totals are not OD flows, route schedules or operational capacities |
| South Africa | Transnet TRIM network-statement annexures, Stats SA P7162, Western Cape Freight Demand Model | Candidate rail-section capacity/running-time evidence, aggregate mode totals, regional OD scenarios | Existing [#1631 admission gates](https://github.com/Luminous-Dynamics/symtropy/issues/1631) still apply; discovery is not acquisition or parsing |
| Other countries / corridors | Government and operator sources admitted incrementally | Any evidence class supported by the exact admitted artifacts | Do not infer missing source details from neighboring countries, language, a map, or country rankings |

This is a **candidate source portfolio**, not a declaration that every listed source is public for every intended use or that every region has complete data.

## 4. Artifact-admission protocol

For every file/table/API and each intended use, freeze:

- publisher and stable source URL; exact title, release/version, data vintage and retrieval timestamp;
- raw byte count and SHA-256, media/container type, file-level license/access decision, attribution requirements, and redistribution/training/publication permissions;
- spatial CRS and identifier crosswalks; geometry coverage, temporal coverage, observation time, event time, repository/record time and forecast time as distinct concepts;
- units, currencies/base year, commodity/mode taxonomies, aggregation method, revisions, uncertainty and known exclusions;
- parser/transformation commit, schema fingerprint, source row/sheet counts, accepted/rejected counts, duplicates, missingness and reconciliation checks;
- precise claim ceiling: topology / connectivity / aggregate context / OD scenario / calibrated corridor / empirical benchmark.

Unknown rights or semantics block the corresponding use. A software-code license does not grant the license to datasets mentioned in documentation. Source digests establish byte identity, not issuer authenticity or data truth.

## 5. Global roll-out

### Wave A — globally useful foundations

1. Admit OSM-derived topology after ODbL review, retaining provenance, extraction date and geographic coverage.
2. Add UNCTAD country/port liner-connectivity indicators and World Bank CPPI observations as separate maritime evidence planes.
3. Use World Bank LPI only as country-level context with its uncertainty and survey scope.
4. Run mode-agnostic deterministic synthetic tests everywhere; keep them clearly separate from empirical data.

### Wave B — regional statistical and OD cases

Prioritize the first source families above: US FAF6; EU Eurostat; Australia BITRE; Canada Transport Canada; ASEANstats; Brazil PNL/PNL2050; Japan MLIT; India major-port aggregates; China national aggregate series; and South African rail/transport sources after the current admission gates. Each region advances independently according to its available evidence level.

### Wave C — corridor-level qualification

Where available, add operator/service schedules, section/link capacities, equipment/terminal windows, costs, delays/dwell, energy/emissions factors, disruption history and delivered-shipment outcomes. Only call a corridor G4 after its required input planes are source-qualified. Only call an empirical result G5 after leakage-resistant held-out testing and independent reproduction.

New countries and datasets should be selected for marginal evidence: a new OD region, mode, topology class, disruption history, independent evaluator, or underserved population is more valuable than another aggregate indicator that repeats the same proposition.

## 6. Cross-region benchmarking rules

- Compare like with like: define each estimand, unit, population, mode, spatial grain, period, and uncertainty before placing regions on the same chart.
- Preserve different source classifications and use reviewed crosswalks. Do not silently equate a shipment, port call, customs-declaration mode, tonne, tonne-kilometre, TEU, train path, passenger trip, and concurrent equipment slot.
- Use official source margins to validate OD aggregates. Specify permitted rounding/tolerance; surface source-model discrepancies instead of forcing unexplained conservation.
- Set chronological and route/region holdouts before tuning. Keep target geography out of candidate selection when testing transfer.
- Retain negative controls: wrong units, incomplete source coverage, stale/counterfactual capacity, missing intermodal links, conflicting claims, and topology-only paths presented as operational.
- Report served/unserved demand, feasibility, p50/p95 journey time, source-defined planning/buffer indices where valid, terminal dwell, transfer/gate waiting, capacity use, energy, GHG, whole-life cost, recovery time, and geographic service distribution. Report only metrics supported by each dataset.
- Do not combine country rankings, LPI, CPPI, OD totals and live capacity into a universal scalar score.
- Report benchmark portfolio common-mode dependence. A dozen datasets using the same underlying source or simulator are not a dozen independent evidence sources.

## 7. Benchmark acceptance targets

| Gate | Target |
|---|---|
| Geographic evidence inventory | Every admitted source has a scope, evidence tier, rights state, date reviewed, coverage gaps and exact artifact receipts |
| Crosswalk correctness | 100% of accepted records map through explicit, versioned taxonomy/ID/unit conversions; ambiguous records remain rejected/unknown |
| Source integrity | 100% of results bind input manifest, parser/configuration, source code commit and verifier outcome |
| OD consistency | Reconcile source-defined margins within published rounding/uncertainty; unexplained mismatch is a blocker, not auto-corrected |
| Global breadth | At least 6 distinct regional source families across at least 4 continents at G2 or higher before publishing a cross-region coverage summary; do not pool performance metrics unless the measurand, units, time/spatial grain and crosswalks are demonstrably comparable |
| Corridor validity | No minimum country-count requirement for G4/G5; each specific corridor must meet its own evidence, timing, capacity and holdout requirements |
| Operational nonclaims | Zero schedule/capacity/price/dispatch claims derived solely from geometry, LPI, connectivity indices, national aggregates or synthetic tests |
| Decision value | Every newly added source or benchmark states what decision uncertainty it is intended to change; reassess after execution |

The six-region/four-continent target is a **portfolio breadth target**, not proof that data are comparable or that each country is qualified at the same level. A global map or coverage summary may list stratified regional evidence side-by-side; a pooled performance result additionally requires a declared common measurand, compatible units and temporal/spatial grain, crosswalk validation, and region-level outcomes so composition effects remain visible. If these conditions do not hold, do not compute a pooled score.

## 8. Cross-project ownership

- **Symtropy:** route/flow optimization, resource-capacity accounting, deterministic disruption scenarios and outcome metrics.
- **Symthaea:** mode-specific engineering validity, benchmark rights/coverage, evidence-diversity and model-transfer evaluation.
- **Mycelix:** federation, participant permissions, operational provenance and separately authorized reservation/commit semantics.
- **Sol Atlas:** read-only global topology/projection, per-layer uncertainty, geographic evidence gaps and alternative corridors.

Reuse existing transport schemas and evidence owners. Do not create a fifth canonical transport ontology, parallel global route database, duplicate event registry, or one global country score.

## 9. Nonclaims

No source is claimed acquired, parsed, permitted for training/redistribution, or qualified merely because it appears here. The current source discovery does not establish global operational coverage, global shortest/optimal routes, current capacity, live schedules, prices, safety, availability, emissions, or service performance. A G0 map can be globally useful while the operational model remains locally sparse.
