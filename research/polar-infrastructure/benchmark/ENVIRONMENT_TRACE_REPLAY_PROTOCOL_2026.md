# Environmental trace replay and fair-comparison protocol

**Status:** proposed protocol; trace manifest is a template, not an ingested dataset.
**Current evidence ceiling:** synthetic boundary-analyzer fixture plus catalog/manifest structure checks. No real environmental trace has been ingested by this change; no controller or physical model has been validated.

## 1. The shift: from scenario count to reproducible stress evidence

A catalog of hostile environments is useful for coverage planning, but catalog size is not model validity. The next unit of progress is a *replayable, source-backed environmental trajectory* with known provenance, spatial/temporal scope, uncertainty, missingness, transformation history, and an explicit engineering use.

This protocol separates four questions that are often accidentally conflated:

1. **Source authenticity:** can an independent reviewer identify the dataset and reproduce the acquired source artifact?
2. **Trace integrity:** can the normalized time series be regenerated from the source artifact using a pinned transformation?
3. **Model suitability:** has the environmental data/model been validated for the declared site, scale, variables and engineering purpose?
4. **System resilience:** did the compared system preserve the declared services and hard constraints under that trajectory?

Passing one layer does not imply passing the next. A valid SHA-256 digest proves byte identity, not physical accuracy. Reanalysis is not an in-situ measurement. A successful simulation replay is not field validation.

The initial manifest template is [environment-trace-manifest-template-2026.json](fixtures/environment-trace-manifest-template-2026.json), governed by [environment-trace-manifest-0.1.0.schema.json](schemas/environment-trace-manifest-0.1.0.schema.json). It uses the Namib coastal desert as a *profile ID only*: no site coordinates, dates, source dataset, or observation values are fabricated.

## 2. Admission contract for a replayable trace

A trace cannot be marked `validated_for_replay` until the manifest carries all of the following:

- **Source provenance:** provider, dataset/version, source URL, attribution/license or access terms, retrieval time, source-artifact SHA-256, and an immutable or durable archive URI.
- **Spatial meaning:** coordinate reference, site/grid-cell identity, latitude/longitude/elevation where applicable, and spatial support/resolution. A nearest grid cell must not be presented as a station observation.
- **Chronology:** UTC timestamps, start/end, sample interval, expected/observed/missing counts, and whether each variable is instantaneous, interval-averaged or accumulated.
- **Variable semantics:** physical quantity, canonical unit, native source field, conversion/transformation, variable-level evidence class, and whether the field is mandatory for that replay.
- **Missingness and uncertainty:** explicit missing-data policy, gap masks, known dataset biases, uncertainty method, and any interpolation or reconstruction marked separately from source observations.
- **Transformation provenance:** code revision, deterministic recipe/configuration digest, derived-artifact SHA-256, format/size, and validation evidence against independent observations or a declared reference where applicable.
- **Intended-use boundary:** site/time/body/mission, model applicability, excluded physics and evidence class. Mark unsupported variables unknown/not-evaluated; never silently substitute zero.

The JSON Schema expresses the core shape and elevated requirements for a replay-validated manifest. The Rust contract gate is implemented in [environment_trace_manifest.rs](src/bin/environment_trace_manifest.rs): it rejects unknown/missing fields, duplicate variable IDs, invalid status/evidence combinations, incomplete replay metadata, bad digest syntax and inconsistent sample counts. Its report explicitly says it does not hash the actual artifact or validate physical-model correctness. The workflow's jq assertions are structural guardrails; neither jq nor the Rust gate is a full Draft 2020-12 JSON Schema engine or a replacement for data-quality review and domain expert sign-off.

## 2.1 Minimal value-shape and physical-sanity screening

The NASA POWER daily normalizer checks that requested fields are numeric or explicitly missing, daily irradiation/wind/precipitation are non-negative, relative humidity is within 0–100%, daily minimum does not exceed daily maximum, and the daily mean lies between them when all three exist. These checks catch malformed records before CSV generation; they are not a site validation, a climatological envelope, or a controller/plant model.

Do not copy those rules wholesale into the station-data adapter. The published Gobabeb BSRN record includes small negative nighttime SWD samples (for example, values around -3 W/m²). Those raw instrument values must be preserved and analyzed with the instrument's QA/offset context; a separate, explicitly declared screened metric can coexist with the raw metric. A universal global-horizontal-radiation `>= 0` check would incorrectly discard legitimate measurement information in that source class.

## 3. Chronology and split policy

### Preserve joint history

Keep temperature, irradiance, wind, precipitation, cloud/fog indicators and any available site-specific dust/salt observations aligned on their original chronology. Do not independently shuffle variables or resample each hazard as though they were independent. Doing so can erase the co-occurrence patterns the resilience benchmark is supposed to test.

Store source values separately from derived/controller-visible signals. Every conversion must state interval semantics, timezone, units, time aggregation and interpolation. Never bridge a long outage with silent interpolation. Keep missingness as a mask and publish how results change under defensible gap treatments.

### Prevent leakage

Do not randomly split individual time steps. Split by contiguous whole episodes and preferably whole site-years or time blocks, with a frozen temporal holdout. Add geographic holdouts only when enough comparable sites exist. A site represented in tuning data must not be described as a geographic holdout.

Use three distinct evaluation sets:

- **Development:** openly used to debug the trace adapter, calibrate models and tune policies.
- **Validation:** used to choose a frozen model/policy and to select operating thresholds.
- **Locked test:** run once the configuration and primary metrics are registered; publish a manifest of the split and hashes before evaluation.

If only one site or one year is available, state that limitation and avoid claims of geographic or temporal generalization. Reusing a severe event many times with small perturbations does not create independent field evidence.

### Empirical histories versus injected stress

Maintain separate evidence labels for (a) observed/reanalysis chronology, (b) transformations and model-generated trajectories, and (c) adversarial fault injection. A synthetic injected dust deposition pulse may be a useful engineering stress test, but it must not be assigned an empirical occurrence probability without a defensible occurrence model.

Where credible probabilities exist, publish the source and calibration of those probabilities. Otherwise report named stress cases, coverage and worst observed outcomes; do not turn an equally weighted test list into an expected annual risk.

## 4. Earth-first data acquisition strategy

The most useful first milestone is a small, geographically explicit Earth suite rather than attempting to parameterize all 51 profiles at once.

1. **Broad weather/solar drivers:** inspect NASA POWER's solar/meteorological APIs and ECMWF ERA5. ERA5 has hourly globally complete reanalysis and ensemble-based uncertainty products at a coarser resolution than local instruments. Record the exact product, grid cell, retrieval/version and native temporal semantics. NASA POWER returns analysis-ready solar/meteorological time series; its parameter and temporal product must be chosen explicitly.
2. **Higher-resolution solar data:** inspect NLR's NSRDB availability by location before selecting it. The product is not a universal promise of site coverage; record the particular regional product, native interval and grid support where available.
3. **Site-specific stressors:** obtain field or properly calibrated domain-specific data for Namib fog liquid water/capture yield, marine salt deposition/corrosion, and dust deposition/abrasion. Global weather reanalysis does not by itself validate these quantities or an equipment's response to them.
4. **Asset response:** pin manufacturer curves or independently measured derating, battery temperature/aging constraints, cleaning efficacy, water-system yields, and maintenance-access assumptions. Label unsupported behavior as hypothesized and include it only in synthetic sensitivity studies until validated.
5. **Cross-domain extension:** only after an Earth trace adapter and independent checks exist, add mission-body sources for lunar illumination, Mars dust/thermal timelines or other solar-system profiles. Each requires its own source, coordinate/time system, trajectory/site context and model-credibility assessment.

Useful entry points:
- NASA POWER API docs: https://power.larc.nasa.gov/docs/services/api/temporal/daily/
- ECMWF ERA5 hourly single-level data: https://www.ecmwf.int/en/forecasts/datasets/era5-hourly-data-single-levels-1940-present
- NLR NSRDB API: https://developer.nlr.gov/docs/solar/nsrdb/
- NASA-STD-7009B model and simulation credibility: https://standards.nasa.gov/standard/NASA/NASA-STD-7009
- IEEE 2030.8 microgrid-controller testing scope: https://standards.ieee.org/ieee/2030.8/6169/
- NIST SP 800-160 Vol. 2 Rev. 1 cyber-resiliency framework: https://csrc.nist.gov/pubs/sp/800/160/v2/r1/final

## 4.1 First executable pilot: Gobabeb/Namib daily environment envelope

The live acquisition adapter is [fetch_and_normalize_nasa_power_daily.sh](scripts/fetch_and_normalize_nasa_power_daily.sh), which requests a single point from the NASA POWER Daily API in UTC and passes the response to [normalize_nasa_power_daily.sh](scripts/normalize_nasa_power_daily.sh). The manifest links both the source-response artifact and normalized trace artifact; [verify_environment_trace_artifacts.sh](scripts/verify_environment_trace_artifacts.sh) independently recomputes their SHA-256 hashes and checks trace byte size and row count against the manifest. The latter refuses unexpected response layout, request dates, time standard or parameter units; preserves the raw response copy and its SHA-256; maps source variables to canonical units; keeps fill values blank; and emits a manifest marked `ingested_unvalidated`. Daily solar irradiance is a daily energy total, not instantaneous power, and daily meteorology is not enough to establish inverter stability or control-loop performance.

A published Gobabeb location is 23°33′42.61″S, 15°02′30.15″E (approximately latitude -23.561836, longitude 15.041708). The station is in the Namib Desert at the fog/rain ecotone. See the [EIS Namibia library record for a Gobabeb research-station paper](https://the-eis.com/elibrary/index.php/search/22385) and the [Gobabeb institute overview](https://gobabeb.org/). Do not treat the coordinate alone as proof that a gridded product equals the station record.

From a machine with network access, the first repeatable pilot command is:

```bash
bash research/polar-infrastructure/benchmark/scripts/fetch_and_normalize_nasa_power_daily.sh \
  earth.coastal_desert_namib \
  "Gobabeb station coordinates; NASA POWER point trace, not station observations" \
  -23.561836 15.041708 20240101 20241231 \
  ./artifacts/gobabeb-power-2024
```

This issues one modest daily API request, records the effective URL and exact requested/response geometry coordinates, and writes `source-response.json`, `environment-trace.csv`, `acquisition-config.json`, and `environment-trace-manifest.json`. The current adapter intentionally does **not** mark the manifest `validated_for_replay`; users must archive the artifacts durably, check units/quality, and validate variables against suitable reference observations before treating them as an engineering trace. The verifier establishes byte integrity and sample-row count only, not that the source data are accurate or the physical model is valid. The checked-in fixture is explicitly synthetic and exists only to test transformations and negative controls offline.

For a suitability check, NASA POWER documents its [validation API](https://power.larc.nasa.gov/docs/services/api/validate/), which compares supported POWER variables with nearby qualifying surface sites and reports statistics such as bias and RMSE. The selection is based on nearest qualifying site and parameter-specific availability; verify the selected station, temporal match, QC and representativeness rather than equating a generic validation result with Gobabeb site validation. The [NASA POWER Daily API documentation](https://power.larc.nasa.gov/docs/services/api/temporal/daily/) also notes that daily data can be UTC or local solar time, with LST the default, so the adapter explicitly requests and checks UTC.

A priority follow-on is an **hourly** trace adapter, followed by appropriate site-meter and plant response traces. Daily records can support broad seasonal feasibility screening; they cannot serve as a proxy for fast electrical controller, EMT or protection tests. Those require a separately validated plant model and matched hourly/sub-hour disturbances or suitable laboratory traces.

## 5. Fair benchmark arms

Every comparison must use the same episode boundaries, service contract, initial-state digest, disturbance trace, asset limits, controller-visible observations/forecasts, hardware class, decision deadline and metric extractor. Record controller version/configuration and tuning budget. If an arm receives better forecasts, longer horizon or more compute, show it as a separate information/compute tier.

| Arm | Role | Fairness rule |
|---|---|---|
| Documented incumbent/operator policy | Operational reference | Capture policy version and operator overrides |
| Transparent fixed rule/reserve policy | Simple reproducible baseline | Publish thresholds and fallback behavior |
| LP/MILP plan where a validated formulation exists | Optimization comparator | Match constraints, forecast availability and decision horizon |
| MPC/receding-horizon controller | Replanning comparator | Match forecast inputs, wall-clock budget and deadlines |
| Robust/scenario optimization | Uncertainty-aware comparator | Freeze uncertainty set and scenario budget before test |
| Candidate policy | Proposed method | Freeze before locked evaluation |
| Clairvoyant optimizer | Optional diagnostic upper bound only | Label access to future ground truth; do not treat as a fair operational competitor |
| Negative controls | Test harness integrity | Include known floor violations, stale data, failed actuation and invalid manifests |

IEEE 2030.8 is a benchmark/design reference for controller-function tests, not evidence that Symtropy conforms to that standard. NIST cyber-resilience concepts motivate fault-to-recovery cases, while NASA-STD-7009B motivates model credibility gates. Map individual tests to relevant functions or principles; do not claim certification or standard conformity without the required process and evidence.

## 6. Outcome and boundary metrics

Keep hard safety/service gates separate from optimization objectives. A cost, fuel or average-availability improvement must never compensate for a critical service-floor breach.

**Service outcomes (per service, before any aggregate):**
- physical shortfall with units and its time integral;
- number and duration of floor breaches, including longest consecutive breach;
- time to first breach, treating a trace with no breach as right-censored at its endpoint;
- recovery time after disturbance and time to restore every required service floor;
- fuel/resupply, maintenance interventions, component stress and operating cost;
- uncertainty intervals across independent episodes/sites, preserving temporal dependence.

**Boundary-discovery quality:**
- zero false passes on known negative controls;
- fraction of seeded minimal counterexamples recovered;
- boundary regret against an exhaustive reference on small enumerable cases;
- number of expensive evaluations and wall time to reach a predeclared frontier error;
- coverage by gate/group and count of invalid, missing and unevaluated cases;
- monotonicity-conflict witnesses and retained incomparable points, rather than assuming one scalar hazard severity.

**Trace/model quality:**
- timestamp/unit/schema integrity, duplicate records, missingness fraction and gap-length distribution;
- source and transformed artifact hash verification;
- independent validation error for the declared variables and range;
- uncertainty calibration/coverage where probabilistic predictions are produced;
- explicit out-of-domain detection and abstention when required variables/models are absent.

**Execution and provenance:**
- p50/p95/p99 decision latency, missed deadlines, peak memory and platform energy on named hardware;
- 100% lineage for reported runs: study ID, exact code/config/model revisions, source and derived hashes, scenario, initial state, constraints and result digest;
- recomputation by an independent verifier from raw traces and per-step outputs.

Publish per-site and per-hazard results alongside any summary score. Keep Earth weather-service resilience, electrical EMT dynamics, cyber-physical integrity, space radiation, thermal protection and planetary protection as separate physics/acceptance gates. Shared provenance does not make the physical models interchangeable.

## 7. Targets and evidence gates

The first targets are **integrity targets**, not an unearned claim to beat state of the art:

1. 100% of replay-eligible traces meet the manifest's required metadata and have verified hashes.
2. 100% of required negative controls are detected; zero false passes in the declared suite.
3. Every profile and model declares status, intended use and exclusions; missing evidence is never silently a pass.
4. Small deterministic cases reproduce the same output digest under identical code/config/data, or document and bound expected nondeterminism.
5. Before a performance superiority claim, reproduce a relevant published/open baseline within pre-registered tolerance on a matching plant and protocol.
6. Only after baseline reproduction, preregister primary service metrics, hard gates, tuning budgets, holdout and uncertainty methods. A proposed relative improvement (for example 10%) is a hypothesis until the baseline distribution and practical significance are established; it is not a result or a universal target.
7. Advance from offline replay to software-in-the-loop, hardware-in-the-loop, commissioning and field evaluation only with explicit gate criteria and independent domain review.

The ambition is to outperform the *strongest eligible baseline* on held-out compound events, with no new critical floor failures, and with lower service shortfall or a better cost/resilience Pareto frontier. No superiority claim is warranted until a matched test demonstrates it.

## 8. What would count as a paradigm shift?

The differentiator is a common evidence and failure-boundary layer over specialist physics, not one universal environment simulator or one universal score:

- **From scenario quantity to trace provenance:** replay any admitted stress timeline with verifiable lineage.
- **From average availability to service contracts:** preserve heat, water, sanitation, communication and other critical floors individually.
- **From one severity score to partial-order boundary discovery:** preserve incomparable hazards and surface violations of assumed monotonicity.
- **From independent random faults to chronological compound hazards:** retain correlated environmental histories and then label injected failures separately.
- **From leaderboard-only claims to credible-model envelopes:** expose evidence class, validation domain, uncertainty and unsupported transfer.
- **From best-case optimization to bounded autonomy:** evaluate safe fallback, abstention, recovery and degraded operation when inputs or models are outside their validated domain.

This protocol defines a work queue. It does not claim trace ingestion, physical-model validity, field performance, IEEE conformity, or state-of-the-art superiority.
