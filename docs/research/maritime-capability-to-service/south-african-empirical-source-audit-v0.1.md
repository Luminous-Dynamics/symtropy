# South African empirical source admission audit v0.1

Companion to [MCSC-SA-EMPIRICAL-EVIDENCE-001 #7836](https://github.com/Luminous-Dynamics/symtropy/issues/7836), under [MCSC-CORRIDOR-EVIDENCE-001 #7818](https://github.com/Luminous-Dynamics/symtropy/issues/7818).

Audit date: 27 November 2026.

## Decision summary

The current evidence landscape supports a **three-layer baseline**, not a single merged dataset:

1. Official annual/regulatory aggregates for sector context and reporting definitions.
2. AIS-derived movement observations for vessel/port-call reconstruction, subject to coverage and detector validation.
3. Event-level port/terminal/utility/corridor records with source-observed and first-available timestamps for KITE historical decision replay.

Only the first layer is presently source-qualified by the documents reviewed below. The AIS sources are candidates for movement reconstruction and require additional acquisition, rights, coverage and processing validation. No historical decision-time corpus for South African ports is admitted by this audit.

## Source register

| Source | Source time / coverage | Class | Admissible use now | Not admissible as |
|---|---|---|---|---|
| Ports Regulator of South Africa, 2024/25 Annual Report | 2024/25 financial-year report, listed on the official annual-reports page; eight commercial ports; WEGO performance summary | PUBLISHED_AGGREGATE / REGULATORY_DERIVED | Regulatory KPI vocabulary, reported annual scorecard context | Raw call-level delay, port event chronology, absolute vessel waiting hours |
| Transnet media statement, FY2025/26 | Published 2026-05-12; eight TNPA commercial ports; annual totals | PUBLISHED_AGGREGATE / OWNER_REPORTED | Aggregate annual vessel arrivals and cargo throughput context | Per-port arrivals, per-vessel times, receiver delivery or corridor incidents |
| UNCTAD Review of Maritime Transport 2026 | Published 2026-11-26; global/developing/developed and Africa regional aggregates | PUBLISHED_AGGREGATE / INTERNATIONAL_COMPARATOR | Trade-route, waiting-time and continuity context | South African port-specific event truth or operational baseline |
| Voydar port calls by port/month | 2026-10 monthly file, 681 rows, generated 2026-11-05; feeds BarentsWatch, Digitraffic and Voydar receivers | OBSERVED_MOVEMENT_DERIVED / MONTHLY_AGGREGATE | Potential method comparator after South African port rows are checked in the actual file | KITE event history, individual vessel-call timeline from monthly aggregates |
| Voydar Durban per-port page/API (ZADUR) | Per-port page retrieved 2026-11-26; AIS-detected call sections include events dated 2026-11-25 and repeated-caller history since 2026-08-28 | OBSERVED_MOVEMENT_DERIVED / PER-PORT PAGE/API | Exploratory movement reconstruction; API advertises section-level source/licence metadata | Official port-call/berth/cargo event, first-availability replay, receiver acceptance |
| IMF PortWatch-derived Ngqura daily activity (EED) | Estimate up to 2026-11-20; weekly/daily and 28-day aggregated metrics | OBSERVED_MOVEMENT_DERIVED / AIS_ESTIMATE | Port-level aggregate context with explicit modelled/estimate class | Official port statistic, per-vessel event history, independent corroboration of another PortWatch-derived feed |
| Dues live AIS vessel board | Current snapshot; displays Durban and Cape Town and a two-week replay; explicitly says free feed does not hear Algoa Bay / Ngqura | OBSERVED_MOVEMENT_DERIVED / LIVE_FEED | Coverage and missingness control for the displayed ports | Complete South African AIS coverage, long-horizon historical baseline |
| GuillermoSiaira Open AIS Historical Dataset | README declares daily global AIS positions / port calls / voyages; repository HEAD audited at de1b329075514d5e4c14830e0dd4eeddd1f06895 (2026-05-02) | CANDIDATE_OBSERVED_MOVEMENT | Exploratory source-code and pipeline audit; potential retrospective reconstruction after repairs and byte/rights validation | Current validated South African dataset, signed source provenance, historical first-availability replay |

## 1. Ports Regulator of South Africa (PRSA)

Source: [2024/25 Annual Report](https://portsregulator.org/wp-content/uploads/2025/10/PORTS-REGULATOR-SA-AR-2025-V7-HIGH-RES.pdf), especially pp. 40–41 of the report.

The report describes WEGO (Weighted Efficiency Gains from Operations) as a regulatory incentive/oversight tool under the tariff methodology. It names Vessel Service Delay, Ship Working Hour, Berth Productivity, Ship Productivity Indicator, Ship Turnaround Time and Truck Turnaround Time as the measured KPIs. Its provisional table presents weighted/capped performance measures for Richards Bay, Durban, East London, Ngqura, Port Elizabeth, Mossel Bay, Cape Town and Saldanha.

Important qualification boundaries:

- WEGO values are weighted scorecard values, not raw event-level observations.
- The annual report does not supply the underlying individual vessel-call sample, full KPI denominator, timing boundary, raw source observations or every metric's full normalization/weighting calculation.
- The table includes a cap at 15% and revenue weighting; do not interpret its port score as an absolute wait-time or delay-hour measurement.
- The report says the Regulator will develop a framework to validate WEGO KPIs moving forward. Treat the published table as regulator-reported context, not an independently re-computed call-level gold set.

The MCSC adapter must preserve the original KPI label, reference period, normalization/weight/cap metadata, and all unknown definitions. It must not map WEGO percentage values onto hours, TEU, tonnes or service quantities.

## 2. Transnet official annual aggregate

Source: [Transnet media statement, 12 May 2026](https://www.transnet.net/getFile.ashx?id=15673).

The statement reports 8,630 TNPA vessel arrivals for FY2025/26 versus 7,912 for FY2024/25, described as a 9% year-on-year increase. It also reports cargo throughput of approximately 304 million tonnes across the eight commercial seaports, up 4.2%.

Admissible use:
- system-wide annual activity baseline;
- consistency check against future full-year port-call totals, provided the definitions and fiscal-year cutoffs are aligned.

Not admissible:
- allocation of total arrivals to individual ports without source rows;
- inferred average waiting time or port-call duration;
- claims about which port, vessel, cargo or receiver achieved service;
- event-time or first-available-time observations.

## 3. SAaff / BUSA Cargo Movement Update #306 (1 November 2026)

Source: [BUSA Cargo Movement Update #306, dated 1 November 2026](https://ewc.org.za/wp-content/uploads/2026/11/BUSA-Cargo-Movement-Update-2026-11-01.pdf), authored by SAaff and distributed by BUSA.

This is the strongest identified public South African terminal-level aggregate snapshot in this audit. It contains two different source families that must remain separate:

### Vessel dwell table (Table 4, PDF page 6)

The report provides average anchorage and berth time in days for eight terminal/port combinations:

| Port | Terminal | Anchorage days | Berth days |
|---|---|---:|---:|
| Cape Town | CTCT | 0.2 | 1.8 |
| Cape Town | FPT | 0.1 | 3.2 |
| Durban | MPT | 4.9 | 4.1 |
| Durban | DGT | 10.3 | 4.9 |
| Durban | Pier 1 | 5.4 | 3.3 |
| East London | MPT | 0.1 | 2.0 |
| Ngqura | NCT | 1.8 | 2.6 |
| Port Elizabeth | PECT | 0.7 | 1.5 |

The table attributes these values to MarineTraffic AIS, calculated for the report and marked updated 3 November 2026. The text introduces them as average vessel time at anchorage and average time at berth “as of Thursday morning,” without a precise observation window or named vessel-call denominator. The report's cover date is 1 November 2026, while other narrative references vessel queue observations on 3 and 5 November. These dates must not be collapsed into one assumed report cutoff.

**Admission class:** PUBLISHED_AGGREGATE / AIS_DERIVED.

**Claim ceiling:** terminal-level reported averages only. Not raw vessel-event intervals, official port-limit arrival/departure times, a complete vessel-call sample, independent metrology, cargo completion, or receiver acceptance. The report does not publish the vessel-level MarineTraffic rows, coverage denominator, missing-call treatment, or exact observation window for Table 4.

### Weekly container flow table (Table 2, PDF page 5)

For 26 October–1 November 2026, the report attributes weekly terminal flow to Transnet Port Terminals (TPT), calculated by SAaff:

| Port/terminal | Daily average TEU | Weekly total TEU | Week-on-week |
|---|---:|---:|---:|
| Durban Gateway Terminal (DGT) | 4,997 | 34,976 | +12% |
| Durban New Pier (Pier 1) | 1,931 | 13,517 | -13% |
| Cape Town CTCT | 2,698 | 18,886 | +76% |
| Ngqura NCT | 2,608 | 18,257 | +15% |
| Port Elizabeth PECT | 860 | 6,017 | +204% |
| Other | 1,253 | 8,770 | +7% |
| All terminals | 14,346 | 100,423 | +20% |

**Admission class:** PUBLISHED_AGGREGATE / OPERATOR_DERIVED.

**Claim ceiling:** weekly terminal TEU throughput as reported/calculated from TPT data. Not vessel counts, dwell-time denominators, individual cargo acceptance or a per-receiver delivery record. “Other” must not be allocated to named ports.

### Frozen source projection

The values above are frozen in [busa-cargo-movement-update-306-snapshot-v0.tsv](fixtures/busa-cargo-movement-update-306-vessel-dwell-v0.tsv). The file retains the separate report date and source-update date, uses empty period boundaries where the PDF does not state a precise dwell observation window, and stores the two source families under different record_type and source_basis values.

This is a useful first local benchmark baseline for terminal-level congestion and throughput comparisons. It still does not provide source event timestamps, first-availability timestamps, per-vessel traces or a historical decision-time corpus for KITE.

## 3. UNCTAD 2026 international comparator

Sources:
- [Review of Maritime Transport 2026](https://unctad.org/publication/review-maritime-transport-2026)
- [UNCTAD summary of shifting shipping connections, 26 November 2026](https://unctad.org/news/global-shippings-changing-map-where-trade-connections-are-growing-and-vulnerabilities-are)
- [Official report PDF](https://unctad.org/system/files/official-document/rmt2026_en.pdf)

Selected figures:
- African containership port calls grew 30% and oil-tanker calls grew 85% between 2018 H1 and 2025 H2.
- Average containership waiting time increased from 9.2 to 11.8 hours for developing countries and from 4.0 to 6.0 hours for developed countries between March 2016 and December 2025.

These are international/regional aggregates and a useful external motivation for corridor resilience. They must not be represented as South African port-specific waiting-time estimates. UNCTAD also emphasizes continuity plans spanning whole transport corridors, including pre-agreed routes and customs procedures; this supports MCSC's corridor-level research question, not any specific South African event claim.

## 4. Voydar monthly port-call aggregates

Source: [Voydar Open Datasets](https://www.voydar.com/data).

At the audit date, the port-call-by-port-and-month table lists the 2026-10 snapshot with 681 rows, generated 5 November 2026, under CC BY 4.0. The page identifies the input feeds as BarentsWatch AIS API, Digitraffic marine AIS and Voydar-operated receivers. Its method counts a call when a vessel stays under 2 knots inside a port geofence for 15 minutes; it counts the call in the month it began. It states that coverage-limited counts are lower bounds. Monthly fields include calls, distinct vessels, closed calls and median dwell hours.

Critical limitations:
- The public page lists 681 rows and previews only the first five; the full CSV could not be fetched in this audit session. South African membership of the **monthly CC BY file** therefore remains UNKNOWN. Do not infer either presence or absence from its preview.
- The separate ZADUR per-port page/API is positive evidence that Voydar has a South African per-port view, but it is not proof that ZADUR is present in the 681-row monthly open-data file or that every section is redistributable.
- Monthly aggregates cannot reconstruct individual call chronology or per-vessel decision-time evidence.
- A port-call detector's geofence/speed/minimum-duration semantics are not identical to official arrival, anchorage, berth, cargo-work or departure event definitions.
- Closed-call count is a detector-specific status for observed departure, not proof of completed cargo service or receiver acceptance.
- The page excludes AIS feeds whose redistribution rights are unknown; do not assume that any other public AIS pipeline has equivalent source rights.

This dataset is a useful external detector/coverage comparator only after its actual rows and source coverage have been inspected.

## 5. Voydar per-port movement view: Durban (ZADUR)

Source: [Voydar Port of Durban page](https://www.voydar.com/port/zadur-durban), retrieved 26 November 2026. UN/LOCODE ZADUR.

The per-port page includes a recent-arrivals/departures table with AIS-detected records dated 24–25 November 2026, and a regular-caller section derived from AIS port-call detection (at least 30 minutes in port). Its JSON endpoint is advertised as \`GET /api/v1/ports/ZADUR\`; the page says each section has its own source and licence, and sections whose sources do not permit redistribution are listed as withheld.

Admission class: OBSERVED_MOVEMENT_DERIVED / PER-PORT PAGE/API.

Use:
- candidate for exploratory vessel movement and detected-call reconstruction;
- a way to inspect section-level source lineage and licensing;
- a coverage/missingness comparison against official or terminal-owned arrival/departure records.

Do not:
- treat AIS-detected time as official port-limit arrival, anchorage, berth, cargo-work or receiver-acceptance time;
- assume all page sections share one upstream source or licence;
- redistribute any section marked withheld;
- use page retrieval time as historical first-available time;
- assume ZADUR's existence on the port page proves it appears in the separate 681-row monthly CC BY CSV.

The API JSON endpoint could not be fetched by the available web retrieval path during this audit, so no raw JSON bytes, per-section source metadata, licence records or immutable digest have been frozen. The visible webpage itself is a source lead, not a qualified empirical archive. The page also distinguishes nearby AIS sightings from port calls; these must not be conflated.

## 6. IMF PortWatch-derived Ngqura estimates

Source: [El Estrecho Digital, Port of Ngqura daily activity](https://www.elestrechodigital.com/en/maritime-radar/ports/port2063), using IMF PortWatch AIS estimates; page reports data through 20 November 2026.

At the time of retrieval the page reports a seven-day estimated average of 16 kt/day, a 28-day estimated volume of 24 kt/day, and an average of one port call per day. Its by-vessel-type table gives last-28-day call and estimated cargo figures. The page explicitly says the figures are estimates, may include transhipment, and are not equivalent to official port statistics.

Admission class: OBSERVED_MOVEMENT_DERIVED / MODELLED AIS ESTIMATE.

This is a useful regional time-series comparator for Ngqura, but:
- it is an estimate from an AIS-derived global product, not a TNPA event ledger;
- port calls and cargo volumes are not receiver-accepted service;
- transhipment and model assumptions may affect the estimate;
- the page's weekly update and revision behaviour must be preserved;
- other IMF PortWatch-derived views are the same upstream provenance family, not independent corroboration.

## 7. Dues live AIS vessel board: coverage control

Source: [Dues South African ports vessel finder](https://dues.co.za/vessel-finder), observed 27 November 2026.

The page displays vessels at anchor, at quay and moving around Durban and Cape Town, with a two-week replay. It explicitly states that Algoa Bay (Ngqura/Gqeberha) is not heard by its free feed.

Admission class: OBSERVED_MOVEMENT_DERIVED / LIVE FEED, licensing and raw-history retention to be verified.

This is useful as a **negative coverage control**: no observed Ngqura vessel on this feed cannot be interpreted as no vessel activity at Ngqura. Its current two-week replay is not evidence of a long historical archive or decision-authority first-availability timestamp. Its source provenance and redistribution terms must be obtained before ingestion.

## 5. Candidate: GuillermoSiaira Open AIS Historical Dataset

Public repository: [GuillermoSiaira/marine-traffic](https://github.com/GuillermoSiaira/marine-traffic).

Repository state frozen for this source-code audit:
- default-branch HEAD: de1b329075514d5e4c14830e0dd4eeddd1f06895;
- commit date: 2026-05-02;
- GitHub API reports the commit as unsigned/unverified;
- repository README states data is CC BY 4.0 and describes daily Parquet snapshots and an IPFS/Filecoin publication flow;
- the repository root did not expose a LICENSE file or committed IPFS manifest during this inspection. The remote manifest CIDs and actual Parquet bytes were not fetched or verified.

### Source-code findings

The reviewed paths are linked to the exact inspected commit:
- [collector/main.py](https://github.com/GuillermoSiaira/marine-traffic/blob/de1b329075514d5e4c14830e0dd4eeddd1f06895/collector/main.py)
- [processor/main.py](https://github.com/GuillermoSiaira/marine-traffic/blob/de1b329075514d5e4c14830e0dd4eeddd1f06895/processor/main.py)
- [processor/quality_check.py](https://github.com/GuillermoSiaira/marine-traffic/blob/de1b329075514d5e4c14830e0dd4eeddd1f06895/processor/quality_check.py)

Confirmed from the inspected source:

1. The collector's Parquet schema stores the AIS message timestamp but no per-record received_at / ingested_at / validated_at / first_available_at.
2. The Parquet writer places batches under year/month/day/hour computed from collector wall-clock now, not each record's event timestamp. Late/backfilled records can be partitioned separately from their event day.
3. If parsing MetaData.time_utc fails, parse_message() substitutes pd.Timestamp.now(tz="UTC"). This must be replaced with a parse error / UNKNOWN disposition, never an invented event timestamp.
4. flush() logs write/upload errors but returns a new empty MessageBuffer after the exception. The caller can therefore discard the failed batch rather than retain it for reconciliation. msgs_written is incremented before write/upload succeeds.
5. The quality checker estimates coastal coverage using hard-coded boxes for Europe/Mediterranean, East Asia, eastern North America, west-coast Americas and the Middle East. It omits Africa and subtracts up to 0.3 from the aggregate quality score based on this proxy. This makes that score invalid as a South African coverage metric.
6. The port-call detector selects stopped rows (SOG below 0.5 kn or AIS status anchor/moored), maps positions to nearest ports, and groups by vessel/port. Its shown segmentation logic changes segments when port_id changes but does not split on a maximum timestamp gap. It can bridge sparse/dark periods unless a separate upstream guarantee rules that out.
7. Arrived/departed timestamps are first/last qualifying stopped-observation times. They are reconstructed AIS estimates, not official port-limit, anchorage, berth, cargo-work or receiver-acceptance events.

These source-code findings do not prove any already-published row is wrong. They do mean this source cannot currently be admitted as a historical KITE decision-time corpus. It may be useful for exploratory movement reconstruction only after source rights, exact IPFS bytes, coverage, quality controls, event-time semantics and pipeline loss behavior are independently validated.

### Required source repairs / independent controls

- retain event_time and source_observed_at separately;
- record receiver_received_at / first_available_at and every ingestion/validation/rights transition;
- reject unparsable timestamps rather than substituting current wall-clock time;
- partition by an explicit event-time strategy while retaining ingestion-time partition metadata;
- persist failed batches and reconcile them before creating a new buffer;
- account for retries, duplicates, dropped messages and upload acknowledgements;
- model Africa/South Africa receiver coverage using actual receiver metadata, not a fixed hard-coded continent list;
- split port-call candidate segments on large observation gaps and preserve uncertainty intervals;
- carry state across event-day boundaries or use a validated lookback/lookahead window;
- compare detected arrival/departure boundaries against official source records where available;
- publish an immutable manifest containing exact source CID, raw file digest, schema/parser version, source-day coverage and QA report digest.

No changes are proposed to this external repository by MCSC. These are admission requirements and negative controls for any downstream use.

## Cross-source alignment rules

- WEGO score != hours in port.
- Transnet annual vessel arrivals != count of receiver-accepted cargo deliveries.
- UNCTAD developing-country waiting time != South African port wait time.
- AIS stopped-position geofence event != official port call/berth event.
- AIS port call != cargo movement, inspection pass or receiver acceptance.
- Event time != source-observed time != first available time != ingestion/validation time.
- Missing row/empty feed/coverage gap != zero activity.

## Initial executable corpus plan

Freeze a source manifest with the sources above and test:
- aggregate known-answer reproduction;
- no disaggregation of annual totals;
- unit/metric boundary mismatch rejection;
- port coverage and feed source attribution;
- first-availability absence => historical replay UNKNOWN;
- bad timestamp parse => UNKNOWN, not now;
- failed upload => retained/reconciled batch, not silent loss;
- duplicate feed/source mirror => one provenance family;
- Africa omitted from coverage proxy => test failure;
- long AIS gap => no continuous event claim without corroboration.

## Qualification targets

- 100% of admitted rows bind source identity, class, units, scope, version and claim ceiling.
- Zero generated current timestamps for malformed source time.
- Zero silent batch loss in acquisition/replay tests.
- Zero false historical-availability claims without first-available evidence.
- Exact reproduction of every selected aggregate statistic with denominator and boundary.
- Report source coverage, censoring and missingness per port/time window.
- Do not claim operational superiority until per-call data and matched baselines are independently qualified.

## Claim ceiling

This audit establishes source-document and source-code observations only. It does not establish current port efficiency, present-day port capacity, cause of delay, actual corridor availability, cargo delivery, receiver acceptance, causal benefit, or permission to operate or dispatch. A published aggregate remains aggregate. AIS position evidence remains observed movement, not authoritative service acceptance.
