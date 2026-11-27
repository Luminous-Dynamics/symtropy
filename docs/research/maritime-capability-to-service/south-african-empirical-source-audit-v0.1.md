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
| Ports Regulator of South Africa, 2024/25 Annual Report | Published 2025-07-31; eight commercial ports; WEGO performance summary | PUBLISHED_AGGREGATE / REGULATORY_DERIVED | Regulatory KPI vocabulary, reported annual scorecard context | Raw call-level delay, port event chronology, absolute vessel waiting hours |
| Transnet media statement, FY2025/26 | Published 2026-05-12; eight TNPA commercial ports; annual totals | PUBLISHED_AGGREGATE / OWNER_REPORTED | Aggregate annual vessel arrivals and cargo throughput context | Per-port arrivals, per-vessel times, receiver delivery or corridor incidents |
| UNCTAD Review of Maritime Transport 2026 | Published 2026-11-26; global/developing/developed and Africa regional aggregates | PUBLISHED_AGGREGATE / INTERNATIONAL_COMPARATOR | Trade-route, waiting-time and continuity context | South African port-specific event truth or operational baseline |
| Voydar port calls by port/month | 2026-10 monthly file, 681 rows, generated 2026-11-05; feeds BarentsWatch, Digitraffic and Voydar receivers | OBSERVED_MOVEMENT_DERIVED / MONTHLY_AGGREGATE | Potential method comparator after South African port coverage is verified | KITE event history, individual vessel-call timeline from monthly aggregates |
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
- The source-feed registry must be checked for South African coverage before selecting any port rows. No South African row or coverage percentage is asserted by this audit.
- Monthly aggregates cannot reconstruct individual call chronology or per-vessel decision-time evidence.
- A port-call detector's geofence/speed/minimum-duration semantics are not identical to official arrival, anchorage, berth, cargo-work or departure event definitions.
- Closed-call count is a detector-specific status for observed departure, not proof of completed cargo service or receiver acceptance.
- The page excludes AIS feeds whose redistribution rights are unknown; do not assume that any other public AIS pipeline has equivalent source rights.

This dataset is a useful external detector/coverage comparator only after its actual rows and source coverage have been inspected.

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
