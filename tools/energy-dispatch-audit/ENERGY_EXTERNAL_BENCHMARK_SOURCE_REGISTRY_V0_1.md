# ENERGY-DATA-ADAPTER-001: External source registry and promotion gate v0.1

## Purpose

This artifact prevents a common benchmark failure: “publicly visible” data being treated as “reproducibly ingested, legally reusable, temporally understood, and qualified.” It inventories source candidates without downloading their contents or implying they are already usable. All three entries intentionally remain \`reference_only_not_imported\`.

## Candidate registry

### 1. South African power-system reliability series

The paper *A South African Power Supply Reliability Dataset, Structured for Count Time Series and Machine Learning Applications* (Data, published 18 June 2026) describes 43,824 hourly observations from 1 July 2018 through 30 June 2023 and 13 variables around contracted demand, generation, capability loss and load-shedding. DOI: https://doi.org/10.3390/data11060149.

**Useful role:** South African national-system forecasting and regime-robustness evaluation, with a stress period different from many European/US building-control benchmarks.

**Hard boundary:** this is not site-metered building demand or a battery dispatch trace. Do not use it to claim local PV/battery feasibility or feeder-level service reliability. The article is open access, but the paper reports source data drawn from Eskom; the exact underlying data reuse terms still need review. The registry pins neither a source file nor a hash yet.

### 2. CityLearn official v3.0.2

The official release notes (released 9 September 2026) describe 32 datasets in the public registry and integration of the v3 control environment, including BESS/PV/EV and electrical-service capabilities. The same release explicitly removed five \`rec_2023_*\` datasets, \`EC_Ermesinde\`, and ALADI datasets. Documentation:
- https://www.citylearn.net/guides/releases.html
- https://www.citylearn.net/overview/dataset.html
- https://github.com/citylearn-project/CityLearn

**Selected candidate:** `citylearn_challenge_2022_phase_all_plus_evs`, pinned to upstream commit `834575c1a0194c8ae9d648ae858376a94dfceb78`. Its schema blob is `5d58d31fd75e4845d73d319c69df34d64223f533`; the pinned schema declares 17 buildings and 8,760 hourly simulator steps from 2024-08-01. The 30 referenced CSV paths, per-file Git blob SHA-1s, byte sizes and row counts are inventoried in `tools/energy-dispatch-audit/fixtures/citylearn-challenge-2022-plus-evs-v3.0.2-source-manifest-v0.1.json`.

**Hard boundary:** the committed manifest pins upstream Git blob identities but does not vendor or commit the external CSV bytes. The dedicated Actions workflow now sparse-checks out the exact source commit, independently verifies the schema and 30 CSV Git blob IDs, checks byte sizes and logical CSV row counts, and generates raw-content SHA-256 receipts for the CI artifact. That workflow has not yet completed, so no generated receipt bundle is claimed here. Dataset-specific data terms have not been independently cleared. The schema contains columns named `predicted_*`, but does not prove their forecast issue/publication times; those fields remain disqualified as deployed causal forecasts until timing semantics are audited. Retired annual REC families remain excluded. Simulator performance is not a field outcome; electrical service constraints do not automatically prove full AC feeder power-flow feasibility.

### 3. Eskom Data Portal rolling dashboard

The portal describes hourly actual and forecast demand for the recent past week, plus demand and available-capacity information:
- https://www.eskom.co.za/dataportal/demand-side/
- https://www.eskom.co.za/dataportal/demand-side/system-hourly-actual-and-forecasted-demand/

**Useful role:** ingestion smoke tests and quick schema checks.

**Hard boundary:** a rolling, mutable feed is not a stable canonical benchmark. Preserve retrieval time, exact original bytes, and digest before using a snapshot in an experiment. National values cannot be relabelled as a site load.

## Data ingress contract

Every future source adapter must preserve:

- Canonical source URL/DOI, version/tag, retrieval time, publication/revision metadata and data-level license review.
- Exact raw bytes in an immutable external snapshot plus SHA-256, size, source checksum if provided, row/column counts and column inventory.
- Source timestamp strings, source timezone, UTC-normalized timestamps, daylight-saving transitions, cadence, interval conventions, duplicates, missingness, and any interpolation/imputation policy.
- Units and conversions with explicit source and output units; no silent kW/kWh or MW/MWh reinterpretation.
- Observed values versus forecasts, forecast issue time, publication time, target time, and decision deadline. No value may enter a policy before it would actually be available.
- Frozen chronological rolling-origin windows, separate train/validation/test periods, horizon purge, and regime labels. Fit scaling, imputation and calibration only on training data.
- Rights states that distinguish pending, cleared and dataset-specific attribution requirements. Article license does not automatically establish third-party source-data reuse rights.

## Promotion states

| State | Meaning |
|---|---|
| \`reference_only_not_imported\` | Metadata only; no raw source bytes captured. |
| \`license_review_required\` | Source bytes may be under investigation, but reuse/redistribution is not cleared. |
| \`ingested_digest_pinned\` | Exact source snapshot exists with hash, row count and source/schema metadata; not yet benchmark-qualified. |
| \`qualified_for_benchmark\` | Exact dataset identity and bytes, cleared data rights, verified units/time semantics, audited adapter, chronological splits, and documented known limitations are all present. |

Promotion is deliberately fail-closed: the validator refuses premature qualified status, source IDs that drift, false row-count/time-window metadata, reused retired CityLearn dataset IDs, and claims that national reliability data prove site-level dispatch.

## Benchmark strategy after ingress

Keep two tracks separate:
1. **South Africa reliability forecast track:** seasonal naïve and persistence baselines; count-aware baselines for load-shedding frequency; calibrated probabilistic outputs where appropriate; rolling-origin evaluation; false-negative/recall for stress events, calibration, MAE/MASE, and worst-regime error. Report the dataset's aggregation limits.
2. **Closed-loop control track:** pin a present CityLearn dataset/schema and simulator tag; compare rule-based control, matched MPC/robust MPC and selected learning baselines with identical initial conditions, seeds and exogenous inputs. Measure unserved essential service, flexible curtailment, operating cost, renewable curtailment, degradation proxy, constraint violations, p50/p95/p99 runtime, memory, timeouts and fallback counts.

Do not combine the scoreboards or infer one domain from the other. For all controller results, keep physical service constraints as hard gates and pass the proposed dispatch through Symtropy's independent auditor. On simulation-only data, call results simulation results.

## Frozen status as of 2026-10-28

- Three source candidates catalogued.
- CityLearn candidate selected and pinned to upstream commit `834575c1a0194c8ae9d648ae858376a94dfceb78`; 30 schema-referenced CSV Git blob IDs are inventoried.
- Zero external CSV bytes are committed into Symtropy; the planned CI audit hashes upstream bytes in the runner and emits receipts only.
- The CI-time raw SHA-256 receipt bundle is implemented but has not yet been observed from a completed exact-head run.
- Zero candidates rights-cleared.
- Zero candidates benchmark-qualified.
- The South African series' reported 43,824-row window remains a paper claim awaiting direct archive inventory, not a verified import receipt.
- CityLearn `predicted_*` issue/publication-time semantics remain unverified; no live causal-forecast assumption is allowed.

## Promotion targets (not yet achieved)

- Every source file has an immutable raw hash and documented license/attribution.
- Exact source rows, columns, time span, timezone, cadence and units are verified.
- 100% rejection of feature-time leakage, bad split ordering, missing/duplicate timestamps and untracked conversion/imputation.
- Same policy code and frozen split logic run across all approved benchmark sources.
- Any SOTA comparison uses matched data, baselines, compute budgets, seeds and uncertainty intervals; no claim follows merely from beating a weak rule baseline.
