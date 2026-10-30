# PWR-TX-TEMP-001 — Synchronized temporal operating scenarios for transformer digital twins

**Status:** executable input-integrity validator. Synthetic leap-year fixture only. This is not an electrical/thermal/ageing solver or a validated South African network profile.

## Objective

A transformer operating twin needs a synchronized time axis across the drivers that jointly determine operation and economics. Independent yearly averages are insufficient when transient peaks, heat waves, EV charging, PV output, tariffs and carbon intensity co-vary.

The `temporal-profile` binary validates complete UTC calendar-year scenarios with a common interval grid. It preserves:

- measured mean current and mean-square current (separately);
- ambient temperature;
- EV load and PV generation as separate contextual drivers;
- electricity tariff, currency, price year and REAL/NOMINAL basis;
- carbon intensity;
- named scenario and weight, source timezone label and source-trace ID.

The validator requires each candidate scenario to use the exact same UTC boundaries. Thus a stress trajectory cannot silently have different sampling or missing periods from the baseline trajectory.

## Run

```sh
cargo run --locked --manifest-path research/transformer-bench/Cargo.toml --bin temporal-profile -- \
  research/transformer-bench/fixtures/temporal-profile-leap-year-synthetic.csv \
  /tmp/temporal-profile-summary.csv
```

The checked-in fixture contains two synthetic scenarios over the 2024 leap year, each with four aligned, unequal-duration intervals. This exercises leap-year and shared-grid semantics, not realistic sub-hourly operational dynamics.

## Input contract

Exact header:

`scenario_id,scenario_weight,profile_id,trace_year,source_timezone,interval_id,start_utc,end_utc,current_pu_mean,mean_square_current_pu2,ambient_temp_c,ev_load_pu,pv_generation_pu,price_per_kwh,currency_code,price_year,price_basis,carbon_intensity_kgco2e_per_kwh,source_trace_id`

Rules:
- At least two scenarios; positive weights must sum to 1.0.
- All scenarios share identical profile, trace year, source-timezone label, tariff currency/year/basis, and exact UTC interval grid.
- Every scenario covers Jan 1 00:00 UTC to next Jan 1 00:00 UTC with gapless, non-overlapping intervals: 8,760 hours or 8,784 in leap years.
- Timestamps must be strict `YYYY-MM-DDTHH:MM:SSZ`; offsets and local wall-clock timestamps are rejected.
- Mean-square current must be at least mean current squared, within a declared numerical tolerance.
- Current, EV/PV, tariff and carbon values must be non-negative and finite; ambient temperature may be negative.
- Source trace IDs and timezone labels must be present, but this program does not authenticate them or resolve the timezone against the IANA database.

The EV load and PV generation fields are contextual explanatory inputs. The validator does **not** add EV load to the measured current, subtract PV generation or calculate a net-load series. That mapping must be explicit in a separately reviewed network/load-flow model; otherwise double-counting is possible. Carbon intensity is carried as a distinct input and is not silently multiplied by load.

## Output

One summary row per scenario with time-weighted mean current, mean-square current, ambient temperature, maximum ambient temperature, EV load, PV generation, tariff and carbon intensity, plus annual coverage and shared profile metadata.

The summaries are convenience descriptors only. A thermal model should consume the ordered time-step rows (or an artifact-bound copy of the source trace), not these annual means, because thermal inertia and short-duration overloads depend on chronology.

## Next digital-twin stages

1. **Source adapters and trust:** ingest meter, SCADA/AMI, weather, EV/PV, tariff and carbon data with immutable source identifiers, calibration/measurement metadata, missing-data flags, units and evidence receipts.
2. **Scenario construction:** generate aligned historical, forecast, stress and compound scenarios. Freeze the training/validation/test split by location, feeder, season and forecast vintage. Estimate weights from defensible data or policy; do not treat expert-assigned stress weights as event probabilities without justification.
3. **Network/load state:** make the mapping from EV/PV to net transformer current explicit with feeder power-flow constraints, phase imbalance, voltage and power factor; preserve harmonics where relevant.
4. **Thermal state:** implement a pinned reference model for the declared transformer class, cooling mode and applicable standard edition; record initial state, parameters, time step, ambient/load, top-oil and winding/hotspot states. Validate against independent temperature-rise tests or traceable sensor data.
5. **Ageing and decisions:** integrate the applicable insulation ageing relation over chronological hotspot state, compare loading/derating/replacement actions, and keep uncertain failure/RUL predictions distinct from calculated loss-of-life proxies.
6. **Evidence and governance:** bind scenario source, transformation, model parameters, software revision, calibration, evaluation splits, output and hard-gate results with `evidence-receipt`. Require an independent reviewer for any operational recommendation.

## Benchmark programme

- Compare temporal ingestion with independently generated UTC ledgers; require zero gaps, overlaps, duplicate IDs, missing scenario intervals and scenario-grid mismatches.
- Compare 15-minute, hourly and coarser aggregations on the same high-resolution trace; measure peak-current, peak-hotspot and cumulative-ageing distortion as sampling is coarsened.
- Challenge the model with high EV charging, reverse power/PV, load ramps, phase imbalance, heat-wave, cold-start, overload-duration and compound stress trajectories within the qualified scope.
- Report hotspot MAE/P95/max, peak timing, transient response, energy/thermal balance, cumulative ageing-integral error, uncertainty coverage/width and false-safe rate by held-out feeder/climate/season.
- Compare a pinned analytical reference, thermal network, high-fidelity thermal model and any learned surrogate at matched parameter/data/compute budgets.
- Keep results split by transformer design, cooling type, feeder/location, frequency, climate and season. A model that performs well on one South African municipal profile has not thereby been validated globally.

Initial aspirational targets from PWR-TX-HST-001 remain MAE ≤2 °C, P95 absolute hotspot error ≤5 °C and cumulative ageing-integral error ≤5% on a declared held-out domain, adjusted for reference measurement uncertainty and application risk. These are proposed research gates, not standards or achieved results.

## Limitations

This binary validates temporal data structure and summarizes input traces. It does not compute electrical net load, voltage, harmonics, losses, thermal state, insulation ageing, carbon emissions or optimal control. It does not perform local-time/DST conversion, authenticate data provenance, establish scenario probabilities or validate tariffs. A trace-integrity pass is not a claim that the source data are physically correct or representative.
