# Gobabeb BSRN station validation lane

**Status:** source-backed validation protocol; no station file has been ingested or scored in this change.
**Purpose:** provide an observational reference lane that is independent of NASA POWER point reanalysis for the initial Namib environmental replay.

## 1. Why this is a better first experiment

A gridded reanalysis is useful for complete chronological drivers, but cannot establish site-level error by itself. Gobabeb has an instrumented Baseline Surface Radiation Network (BSRN) station and a published PANGAEA dataset series:
- **Series DOI:** https://doi.org/10.1594/PANGAEA.968639
- **Coverage stated by the publisher:** 2012-05-16 through 2026-08-31 for the series snapshot consulted on 2026-10-28.
- **Station:** GOB / Gobabeb, Namibia; latitude -23.561400, longitude 15.041980, elevation 407 m.
- **Reported observations:** short-wave downward/global radiation (SWD), direct radiation (DIR), diffuse radiation (DIF), long-wave radiation, 2 m air temperature, relative humidity and station pressure.
- **Observed temporal resolution:** the published June 2025 monthly record displays one-minute timestamps and 561,278 data points across its 15 fields.
- **License:** Baseline Surface Radiation Network License 1.0 (BSRN-1.0). Review the license and the individual dataset metadata before redistribution.
- **Citation requirement:** PANGAEA requests that the dataset be cited when used.

Sources:
- PANGAEA series page: https://doi.pangaea.de/10.1594/PANGAEA.968639
- Example monthly record with metadata, variables and one-minute observations: https://doi.pangaea.de/10.1594/PANGAEA.983811
- BSRN data release conditions: https://bsrn.awi.de/data/conditions-of-data-release/
- PANGAEA metaheader format documentation: https://wiki.pangaea.de/wiki/Metaheader
- PANGAEA text-export/content-negotiation guidance: https://wiki.pangaea.de/wiki/Data_Access_and_Reuse
- [DNS/network fallback and license boundaries](ENVIRONMENT_DATA_ACQUISITION_FALLBACKS_2026.md)
- NASA POWER Daily API: https://power.larc.nasa.gov/docs/services/api/temporal/daily/
- NASA POWER Hourly API: https://power.larc.nasa.gov/docs/services/api/temporal/hourly/

**Use restriction:** the exact June 2025 record is labelled BSRN-1.0. BSRN's release conditions say the data are supplied for bona fide research, require citation of the specific dataset, and forbid direct commercial sale of the data; direct archive files may also have access restrictions. That wording does not, by itself, decide every use of derived values in a commercial product. Review the exact terms and obtain appropriate permission/legal review before commercial deployment or redistribution. Keep the raw dataset outside this repository unless redistribution terms have been confirmed. The accessible PANGAEA HTML preview is limited to 2,000 rows and is not a complete monthly source file. See the acquisition fallback protocol for a DNS-independent local-registration route.

The station is a measurement reference at its sensor location; it does not make every field a validated environmental driver. In particular, the listed BSRN record does not itself provide dust deposition, salt deposition, fog liquid-water content, wind and precipitation in the same series. Keep those variables missing/unvalidated unless a suitable separate source is added.

## 2. Source acquisition and provenance

The BSRN data are not bundled in this repository. Before ingesting any month:

1. Retrieve the exact monthly PANGAEA dataset or its explicitly linked BSRN archive file. Record the resolved monthly DOI, source URL, retrieval time, license/terms, source bytes and SHA-256. Do not cite the series DOI as if it uniquely identifies every monthly file.
2. Preserve the original downloaded bytes separately from any normalized trace. Never overwrite the raw copy during normalization.
3. Parse the publisher's tabular metadata and column names, then map each field explicitly by published name/unit. Fail closed when a column, unit, time basis, QC field or missing-value marker differs from the reviewed contract.
4. Preserve raw instrument values and measurement-quality information. Do not silently clip night-time small negative SWD values: pyranometer offset/noise can produce them. Publish raw metrics and any separately defined physically screened metrics, with the screen stated.
5. Mark the station manifest `measured_field`, with `status: ingested_unvalidated` until schema, units, timestamps, missingness, QA/QC flags and the declared use have been reviewed. Hashes establish byte identity only; they do not certify measurement quality.

Do not build a parser by assuming that a browser-rendered HTML table is byte-identical to PANGAEA's tab-delimited download. Use a captured example from the actual file format as an adapter fixture before implementation.

### Source acquisition helper

The new [PANGAEA source fetcher](scripts/fetch_pangaea_source_file.sh) retrieves one exact dataset DOI's tab-delimited representation, preserves the raw bytes, and writes a download manifest with resolved URL, HTTP status, content type, retrieval time, byte count and SHA-256. It refuses HTML/error pages or a response with no timestamp-shaped data rows. It deliberately does not parse columns or assert timezone/interval meaning.

Example for the publisher's June 2025 format-inspection record:

```bash
bash research/polar-infrastructure/benchmark/scripts/fetch_pangaea_source_file.sh \
  10.1594/PANGAEA.983811 \
  "Baseline Surface Radiation Network License 1.0 (BSRN-1.0)" \
  measured_field \
  ./artifacts/gobabeb-bsrn-2025-06
```

Before running, verify that the named license applies to that exact monthly record. The June 2025 file is for source-format/adapter work and is **not** part of the planned 2020–2021 locked validation window. The required `EVIDENCE_CLASS` argument is chosen by the operator after reviewing the exact dataset metadata; the fetcher restricts it to the manifest enum but does not independently substantiate it. Use `measured_field` only after confirming that the source represents in-situ measurements. Mock tests use `not_evaluated` because their byte content is synthetic. Network retrieval has not been demonstrated from this authoring environment; no downloaded source file is committed by this PR.

## 3. Matched comparison contract

The purpose of the first comparison is to quantify the error of the point reanalysis against the instrument record—not to show controller superiority.

### Variables to compare

Start with clearly aligned, supported pairs:
- BSRN SWD (global short-wave downward radiation) versus a comparable NASA POWER global horizontal/all-sky short-wave quantity.
- BSRN T2 (air temperature at 2 m) versus a matched NASA POWER 2 m temperature quantity.
- BSRN RH (relative humidity) versus a matched POWER RH field, only after documenting temporal aggregation and unit semantics.
- Add long-wave, direct/diffuse or pressure only after confirming that the exact POWER parameter and station variable measure the same physical quantity.

Do not equate direct-normal and direct-horizontal radiation without the required geometry transformation. Do not compare a one-minute instantaneous sample directly to a daily mean or daily energy total.

### Time and aggregation

- Pin the time standard for both products. NASA POWER defaults to Local Solar Time (LST) unless UTC is requested; request UTC explicitly and verify the response metadata.
- Inspect BSRN's monthly metadata/archive convention to confirm the timestamp's interval meaning before joining; do not infer start/end-of-interval semantics from the column label alone.
- Aggregate BSRN into hourly or daily windows using the declared interval semantics and valid-observation count. Integrate irradiance to energy using the actual sampling intervals; do not treat W/m² as Wh/m² without integration.
- Report time-window coverage, missing/invalid observations, longest gaps, duplicates and alignment errors before calculating error metrics.
- If a minute sample sequence has holes, do not silently fill gaps or pretend the interval is complete. A day/hour with too little valid source data is excluded from the primary paired metric and still counted as excluded coverage.

### Calibration / validation leakage

Use disjoint contiguous time blocks:
1. **Calibration block:** may be used to estimate any bias correction, local mapping or uncertainty.
2. **Validation block:** choose transformations/hyperparameters and freeze the procedure.
3. **Locked test block:** evaluate once the mapping and primary metrics are preregistered.

Keep entire months or seasons together rather than randomly splitting one-minute rows. Autocorrelated minute samples are not independent replicates; compute uncertainty using block/day-level resampling or other justified time-series methods.

NASA POWER's own validation API describes comparisons against the nearest qualifying station with site-selection and QC criteria. Do not treat that service as an independent confirmation of a custom Gobabeb comparison unless the exact selected station, source, overlap and data reuse are checked. Prefer to report the direct, locally reproducible comparison from the pinned station file and the pinned POWER request.

The machine-readable [Gobabeb-versus-POWER validation plan](fixtures/gobabeb-bsrn-validation-plan-2026.json) pins the planned temporal split: calibration 2016–2018, validation 2019, and locked test 2020–2021. Those are **planned windows**, not evidence that each monthly file has passed QA/QC; preserve any missing months and do not shift the locked split to improve scores. The file's admission gates are all false until source data are actually acquired and reviewed. The 2025 monthly record was inspected only to establish the publisher's variable inventory and sampling display, not to compute the planned locked-test metrics.

## 4. Report metrics before using the trace in resilience simulations

For every matched variable and temporal aggregation, report:
- Number of expected windows, matched windows, excluded windows and coverage fraction.
- Mean bias error (MBE), MAE, RMSE, correlation and regression slope/intercept; define sign convention and units.
- Metrics by month/season, daytime versus night-time for irradiance, and fog/stratus event windows when publisher annotations and timestamps can be reconciled.
- For irradiance: daytime errors and all-time errors separately; retain instrument negative values in the raw track and label any screened track.
- A block-level confidence interval, with temporal block definition and random seed.
- Exact source DOIs, retrieval URLs, source/normalized/config SHA-256, variable mappings, temporal aggregation recipe and code revision.

A lower RMSE alone does not prove better engineering suitability. Compare systematic bias, tail errors, event-time error, completeness and the variables that actually drive the target model. Do not use station calibration and then score on the same month as if it were held out.

## 5. Decision gates and benchmark ambitions

These are proposed pre-registered gates, not achieved results.

1. **Integrity:** 100% of station source bytes, transformation config and normalized trace can be re-hashed and matched to the manifest.
2. **Temporal integrity:** no unexplained duplicate timestamps; explicit missingness; all matched rows use compatible physical units, timestamp semantics and aggregation windows.
3. **Coverage:** publish valid-pair fraction and excluded intervals. Set a predeclared minimum usable coverage for each metric instead of silently discarding bad months.
4. **Reference credibility:** variable mapping and QA/QC rules are reviewed against the publisher's metadata; independent source/measurement uncertainty is stated.
5. **Generalization:** calibrate on one set of contiguous months, validate on another and report an untouched time block. Never claim site-independent performance from one station.
6. **Reanalysis improvement:** any correction to NASA POWER must outperform uncorrected POWER on locked blocks for predeclared metrics and must not introduce a material regression in critical weather/irradiance periods.
7. **Control evaluation:** only after the forcing data are admitted should the same trace drive incumbent, rule-based and candidate policies under identical service contracts, initial states, forecasts and compute budgets.

## 6. Why this is a field-level improvement

The benchmark should decompose error sources instead of blending them into one score:

`observed station data → source/QA validation → time-aligned model comparison → uncertainty-aware environmental trace → system replay → controller comparison`

That decomposition separates measurement error, reanalysis error, transformation error, physical-model discrepancy and control-policy performance. It makes failures diagnosable: the controller should not be blamed for a bad forcing trace, and a good-looking control score should not conceal a biased or incomplete environmental input.

A successful station comparison would validate only the declared variables, time window, site and use. It would not validate fog-water yield, salt/dust degradation, all Namib sites, daily-to-intrahour transfer or solar-system environments.
