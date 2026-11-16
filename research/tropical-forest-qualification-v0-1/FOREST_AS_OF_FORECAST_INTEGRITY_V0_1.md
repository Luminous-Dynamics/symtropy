# Forest as-of forecast integrity and temporal leakage gate v0.1

**Issue:** [#5487 — FOREST-AS-OF-FORECAST-INTEGRITY-001](https://github.com/Luminous-Dynamics/symtropy/issues/5487)  
**Composes with:** [#5485 forecast usefulness limits](https://github.com/Luminous-Dynamics/symtropy/issues/5485), [#5277 probabilistic forecasts](https://github.com/Luminous-Dynamics/symtropy/issues/5277), [#5398 temporal evidence](https://github.com/Luminous-Dynamics/symtropy/issues/5398), [#5470 AFEX identity](https://github.com/Luminous-Dynamics/symtropy/issues/5470)  
**Date:** 16 November 2026  
**Status:** synthetic as-of integrity policy; no AFEX historical forecast replay

## 1. Paradigm shift: model only what was knowable then

An ecological observation has more than one time:
- **valid/event time:** when the ecological process or sample refers to;
- **collection time:** when the sample or sensor observation was acquired;
- **source release time:** when that exact version became accessible;
- **QA/QC release time:** when the value passed the quality process needed for this model use;
- **rights-effective time:** when intended use was permitted;
- **ingestion time:** when the forecasting system actually received it.

The earliest operationally usable instant is not the collection time. For this benchmark it is the maximum of source release, QA/QC release, rights-effective and ingestion times. Missing any required component is not evidence of early availability.

A forecast fold also has:
- reference time (state origin);
- information cutoff (latest allowed information);
- issue/publication time;
- target interval / horizon.

A true forecast must have its target interval after its issue time. Every input version and fitted transform must have been available by the information cutoff. A later corrected dataset must not silently rewrite the input state used by an earlier forecast.

## 2. External evidence and operational comparator

Dietze et al. define data latency as the interval between collection and availability, and describe lab processing, QA/QC and data release as material constraints on iterative ecological forecasting: https://pmc.ncbi.nlm.nih.gov/articles/PMC5816139/.

The NEON Ecological Forecasting Challenge documents different real target latencies by theme: terrestrial targets are roughly five days old when published, phenology roughly one day, aquatics about two days, and some biotic targets much older. It also records forecast submissions and their publication times: https://projects.ecoforecast.org/neon4cast-docs/CronTiming.html.

The ecological forecasting data convention distinguishes `reference_datetime` from `datetime` and records forecast publication time, which helps distinguish a genuine prospective forecast from a retrospective reforecast: https://doi.org/10.1002/ecs2.4686. NEON additionally separates target availability, driver availability and the time a forecast is submitted: https://projects.ecoforecast.org/neon4cast-ci/instructions.html.

This matters directly to forests. Root traits and phosphatase require lab measurement; litterfall may be sorted, weighed and chemically analysed after collection; satellite composites can be published retrospectively; plot databases can later correct identifications, allometry or QC flags.

## 3. The executable as-of contract

Each fold freezes:
`forecast_reference_time <= information_cutoff_time <= forecast_issued_at < target_start_time`

For each selected input version:
`available_at = max(source_release, qa_release, rights_effective, ingestion)`

and:
`available_at <= information_cutoff_time`

Every selected version binds a logical observation ID and version number. For each logical observation, the selected version must be the latest version whose effective availability is at or before the cutoff. Later versions remain available for later forecast origins, not earlier ones.

Historical observations must have valid time no later than the forecast reference. Realized future observations cannot act as future drivers. A future meteorological driver can be used only as a *forecasted driver* whose own issue time is before the current information cutoff and whose predicted interval covers the target interval.

Fitted transforms carry both their data cutoff and completion time. Their source version dependencies must be among the admitted inputs and have been available by the fold cutoff.

Rights and QC are independent gates. A public URL or a backdated field campaign is not enough to authorize use.

## 4. Synthetic known-truth benchmark

The machine-readable corpus contains 11 cases:
- valid pre-cutoff sensor data;
- pre-issue collection with post-cutoff laboratory QA;
- retrospectively released satellite composite;
- correct selection of the latest version available as-of the cutoff;
- illicit selection of a later correction;
- future realized meteorology used as a driver;
- valid forecasted meteorology issued before the cutoff;
- feature transform fitted with future data;
- missing QA release time;
- uncleared rights;
- evaluation target used as an input;
- post-cutoff ingestion.

The validator reports a case disposition, rejected input IDs/reasons and per-input latency; it binds exact fixture bytes with SHA-256. All timestamps and identities are synthetic.

## 5. Empirical AFEX admission sequence

For each AFEX archive/version, preserve native campaign/event time and separately admit:
1. acquisition/collection timestamp or period;
2. source publication and archive release time;
3. sample processing/assay date and QA/QC release;
4. licence/rights-effective status for the intended use;
5. download/ingestion time and object hash;
6. later archive revisions, corrected values and changed codebooks.

A historic collection date does not imply the data were then available. If source release or QA timestamps cannot be established, the historical as-of replay is NOT_ESTIMABLE. Do not invent an availability lag.

## 6. Proposed empirical acceptance targets (not achieved)

- 100% refusal of inputs released, QA-approved, rights-cleared or ingested after cutoff.
- 100% correct as-of version selection under bitemporal revisions.
- 100% refusal of evaluation targets or realized future drivers used as inputs.
- Forecasted drivers accepted only with their own pre-cutoff issue time and compatible target support.
- 100% of admitted inputs have source/version identity, rights state and availability evidence.
- Empirical reporting of collection-to-release, release-to-QA, QA-to-ingestion and total availability latency by data stream.
- Forecast skill evaluated only on frozen rolling-origin inputs; no random split may let later corrections leak into training.

## 7. Claim ceiling

Synthetic PASS validates the as-of policy only. It does not establish actual AFEX data latency, a historical forecast reconstruction, rights clearance, forecasting accuracy or ecological process validity. A real hindcast requires versioned source files and evidence of when each exact value was released, quality-approved, permitted and ingested.


## 8. Cadence design must use admitted availability times

The [#5492 observation-cadence benchmark](https://github.com/Luminous-Dynamics/symtropy/issues/5492) uses the same separation between sample/event time and availability time. A nominal daily observation cadence is not a daily information cadence when data take seven or ten days to be released and ingested. Cadence optimization must therefore operate only after the source-specific availability contract here has qualified the input stream.

The synthetic cadence adapter is [FOREST_OBSERVATION_CADENCE_V0_1.md](FOREST_OBSERVATION_CADENCE_V0_1.md); its latency/cost profiles are invented, not AFEX timing estimates.
