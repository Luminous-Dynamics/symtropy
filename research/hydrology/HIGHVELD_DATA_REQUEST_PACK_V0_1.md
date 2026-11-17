# Highveld hydrology data request and admission pack v0.1

**Status:** request template only; no data request sent, no data acquired, no catchment selected.  
**Owner:** HIGHVELD-HYDRO-DATA-ADMISSION-001 [#5350](https://github.com/Luminous-Dynamics/symtropy/issues/5350).  
**Consumer:** catchment-conditioned benchmark #5338 and probabilistic evaluator PR #5455.

## 1. Purpose

Obtain the metadata and permission conditions needed to determine whether candidate South African hydrometeorological records can support a reproducible hydrology benchmark.

The first request is metadata-first. Do not request or publish bulk observations until the custodian confirms access conditions, fees, permitted use, redistribution and derived-data rights. Public visibility is not a license.

## 2. Scope of the first data package

Keep each source population separate.

| Population | Candidate custodian/source | Required data | Evidence class |
|---|---|---|---|
| O1 rainfall/station | South African Weather Service (SAWS) | Station metadata, daily/hourly precipitation, quality flags, station moves and missingness | Station observation, only if rights and QA admitted |
| O2 surface-water flow/stage | DWS Hydrological Information System | Gauge metadata, audited daily/monthly flow and stage, rating curves, flags, revisions | Audited or provisional observation, explicitly distinguished |
| O3 groundwater | DWS NGA/HYDSTRA | Well metadata, construction/screen interval, water level, datum, method and sampling history | Point vs time-series groundwater observation, separate populations |
| O4 raw water quality | DWS RQIS/WMS or named lab | Sample site/date, analyte, method, units, LOD/LOQ, qualifiers, lab, QA and chain-of-custody | Laboratory observation; not a drinking-water assessment |
| F0 gridded forcing | CHIRPS, ERA5-Land, GPM IMERG | Exact product/version, grid, time interval, units, bias/quality flags, license | Gridded forcing, not station observation |
| O5 utility notices | Johannesburg Water archive via Mycelix #5431 | Original notice, publication time, service area, stated cause and claimed start/end | Reported assertion, not measured flow/pressure or household outcome |
| O6 utility operations | Utility data custodian under agreement | SCADA/pressure/flow, sensor metadata, incident and repair linkage | Operational telemetry, only under explicit agreement |

Do not pool these populations or call them all "ground truth". A modelled/reanalysis field is not a gauge observation; a notice is not telemetry; a raw chemistry result is not a drinking-water compliance assessment.

## 3. Metadata-only discovery before acquisition

For every candidate station, gauge, sample site or grid product, record:

- source/custodian and stable station/site/product ID;
- coordinates and coordinate reference system, vertical datum where relevant;
- catchment and upstream/downstream relationships, contributing area and known transfers;
- parameter/measurand, unit, aggregation and time zone;
- sampling interval, observation window and missingness;
- sensor/method, calibration/maintenance history, rating curve and revision lineage;
- provisional/audited/estimated status and correction/supersession policy;
- quality flags, uncertainty and known operational interruptions;
- access method, terms, fees, license, retention and redistribution permission;
- whether derived values, model outputs, maps or publications may be shared;
- exact query/export time and custodian response reference.

Unknown fields remain unknown. A dashboard chart, search result or station count does not prove a continuous time series or its quality.

## 4. Request template — SAWS climate observations

**Subject:** Data access and metadata enquiry — candidate rainfall station observations for reproducible hydrology research

Dear SAWS Climate Services,

I am preparing a reproducible, non-commercial hydrology research benchmark focused on catchment-conditioned rainfall–runoff and event/low-flow forecast verification in South Africa.

Before requesting any observation files, could you please advise:

1. Which rainfall stations are available for candidate Gauteng/Highveld catchments, and whether station metadata can be supplied first?
2. Which temporal resolutions and periods are available (daily, hourly or finer), and what are the exact units, time zone, aggregation rules and observation-day conventions?
3. Are records quality-controlled, provisional or final? What quality flags, missing-data codes, station moves, instrument changes and correction/revision histories accompany them?
4. What are the data access terms, fees, license, permitted research uses, retention requirements and redistribution restrictions?
5. May derived catchment aggregates, benchmark scores, plots and research publications be shared publicly? May the raw data or a limited reproducibility subset be redistributed?
6. What citation/acknowledgement and station confidentiality requirements apply?
7. Is there a preferred formal application, agreement or data-use process?

The intended first step is metadata-only candidate screening. No station is selected and no bulk data will be published or redistributed unless the applicable terms expressly permit it.

Kind regards,  
[Researcher / organisation]  
[Institutional contact details]

## 5. Request template — DWS Hydrological Information System

**Subject:** Metadata and research-use enquiry — audited surface-water gauge records for Highveld catchment benchmarking

Dear Department of Water and Sanitation, Hydrological Services,

I am preparing a reproducible hydrology benchmark and would like to identify suitable candidate surface-water gauges before any bulk data request.

Please advise:

1. Which gauge/station IDs in candidate Gauteng/Highveld catchments have audited historical stage and/or flow records?
2. For each candidate, can you provide station coordinates, catchment area, datum, parameter, units, sampling interval, observation period and record completeness?
3. Are flow values derived from rating curves? If so, can the applicable rating-curve versions, valid periods, uncertainty and revisions be provided?
4. How are provisional, unaudited, audited, estimated and corrected values flagged?
5. Are upstream/downstream transfers, dam releases, abstractions, return flows or gauge relocations documented?
6. What access conditions, fees, license and redistribution/derived-data permissions apply?
7. Can research plots, derived aggregates and model evaluation results be published? May any raw data be shared in a reproducibility package?
8. What is the formal process and expected response time for a data request?

The initial request is metadata-only. We will preserve the distinction between audited observations, operational displays and forecast products and will not infer missing values or calibration status from a dashboard.

Kind regards,  
[Researcher / organisation]  
[Institutional contact details]

## 6. Request template — raw water-quality observations

**Subject:** Data dictionary and access enquiry — raw surface-water quality observations for catchment research

Dear DWS Resource Quality Information Services,

Please advise whether metadata and raw, qualified laboratory observations are available for candidate river/reservoir sites in Gauteng/Highveld catchments.

For each record, we need (where held): sample-site ID and coordinates, collection date/time, analyte, result, units, analytical method, laboratory, detection/quantification limit, qualifiers/censoring, QA/QC status, sample type, preservation/holding conditions, and any revised or superseded result.

Please also clarify access terms, fees, citation, derived-data publication and redistribution permissions. The intended use is catchment research and model context only; raw-water observations will not be represented as drinking-water compliance or treatment suitability.

Kind regards,  
[Researcher / organisation]  
[Institutional contact details]

## 7. Admission decision template

A candidate is admitted only when all applicable fields are resolved or explicitly bounded:

| Gate | Required evidence | Failure disposition |
|---|---|---|
| Identity | Stable source/station/site/product ID and custodian | Reject candidate identity |
| Rights | Written/public license and permitted use | Hold; no acquisition/publication |
| Semantics | Measurand, units, interval, time zone, datum | Hold or exclude from incompatible metric |
| Quality | Flags, methods, uncertainty, revisions | Downgrade/stratify; never silently impute |
| Spatial relevance | Catchment topology, contributing area, transfer context | Exclude from claimed catchment |
| Temporal coverage | Record period, missingness, event coverage | Restrict eligible split/claim |
| Leakage | Station/event/catchment relationships and revisions | Redesign split or reject |
| Reproducibility | Stable export, source hash, query timestamp and manifest | No exact replay claim |
| Claim ceiling | Observation vs forcing vs forecast vs assertion | Keep population and label separate |

Admission is per dataset version and intended use, not permanent for a source name.

## 8. First benchmark selection sequence

1. Build a candidate list without naming a winner.
2. Compare record length, resolution, completeness, gauge stability, rating-curve quality, catchment area, dam/abstraction context, rainfall coverage, event/low-flow coverage and rights.
3. Prefer one small catchment with a well-documented, auditable gauge and overlapping precipitation observations over a large catchment with ambiguous transfers or poor records.
4. Freeze the catchment/station set and spatial topology before any model calibration.
5. Freeze whole-event temporal splits and upstream/downstream dependency controls before evaluation.
6. Admit gridded forcing products separately; compare them against station observations without relabelling either.
7. Run B0/B1/B2/B3 references and probabilistic scores only after the data contract is complete.

No catchment is selected by this document. It defines the selection method, not the selection result.

## 9. References

- DWS Hydrological Services: https://www.dws.gov.za/hydrology/
- DWS Groundwater Data Management: https://www.dws.gov.za/Groundwater/data.aspx
- DWS Resource Quality Information Services: https://www.dws.gov.za/iwqs/Report.aspx
- SAWS Climate Services: https://testing.weathersa.co.za/home/aboutclimateatsaws
- CHIRPS data catalogue: https://www.chc.ucsb.edu/data
- Copernicus ERA5 / ERA5-Land: https://climate.copernicus.eu/climate-reanalysis
- NASA GPM IMERG: https://gpm.nasa.gov/data/directory
- Johannesburg Water daily notice archive: https://www.johannesburgwater.co.za/media/daily-water-notices-archives/

**Current disposition:** request templates drafted; no message sent, no data acquired, no source admitted and no catchment selected.
