# EV evidence-to-decision benchmark matrix v0.1

**Programme:** Luminous Dynamics EV engineering (Symthaea, Symtropy, Mycelix, Sol Atlas)  
**Status:** benchmark protocol and integration plan; not a claim of measured SOTA superiority  
**Date:** 27 November 2026

## 1. Paradigm shift: from prediction leaderboard to evidence-qualified operating twin

The field often scores separate components: energy prediction, state-of-health estimation, charging demand forecast, route choice, depot scheduling or charger availability. A low prediction error does not establish that a route is safely feasible, that a charger is available, that a site has sufficient import capacity, or that a fleet will meet tomorrow's duty schedule.

The proposed architecture composes:

    Intent / duty schedule
       -> identity and configuration
       -> physical state and capability
       -> road, charger and grid evidence
       -> uncertainty-aware prediction
       -> constrained route/depot optimization
       -> independent qualification
       -> signed decision receipt
       -> observed outcome and drift feedback

Each claim has a subject, issuer, evidence class, measurement boundary, timestamp/freshness window, uncertainty and digest. A cryptographic signature authenticates an issuer-bound statement; it is not proof of sensor truth. A feasible plan requires relevant evidence to be authenticated and physically/operationally adequate. Unknown, stale or out-of-domain inputs produce INDETERMINATE, not a fabricated feasible result.

The differentiator against a model-only product is a decision receipt: not only what is recommended, but which evidence makes the recommendation valid, what would invalidate it, and which outcome would falsify it.

## 2. Keep benchmark domains separate

| Domain | Strong external anchor | What it tests | What it does not test |
|---|---|---|---|
| Passenger-EV route energy | CHARGE N3 public trip summaries | direction, terrain, speed, published SOC/energy-summary consistency | raw battery truth, calibrated route-energy model, charger metering |
| Heavy-duty depot load | NLR 81-profile archive | vocation × fleet size × strategy × average/minimum/peak aggregate load | measured charging sessions or local South African grid limits |
| Fleet demand/load scenarios | NLR EVI-Pro Lite API | 15-minute daily fleet-load scenarios and load breakdown | class-8 depot transfer unless scenario dimensions match |
| Battery SOH in field vehicles | Nature Communications 2025, 300 EVs, 3 years | multimodal field features and cross-vehicle robustness | automatic transfer to different chemistries/models |
| Same-model early-data SOH | Energy 2026, 100 vehicles | per-vehicle early-life learning and data sufficiency within one model family | cross-brand generalization |
| Depot operation/grid capacity | 2025 vehicle-valet depot study | fleet duty service, grid capacity, queueing, charging policy and cost | valet benefit without matched repositioning assumptions |
| Heavy-duty field operations | 2026 preprint, >400 trucks, ~40k tours, 9 depots | measured site/building load with tour and depot constraints | peer-reviewed baseline until methods/data review |
| MCS interoperability | IEC 61851-23-3:2026; SAE J3271_202503 | coupler, communications, cooling/cable, grid and interoperability lanes | simulation-only conformance |

Do not mix a modeled 15-minute profile with a measured route-energy label, nor treat an operator-published trip card as a CAN/BMS observation.

## 3. External sources and comparison conditions

### N3 passenger-EV corridor

CHARGE's public N3 page currently lists 95 South African BEV models, 32 shown as tested. The current local Sol Atlas corpus contains 26 explicitly transcribed trip cards. It is a curated public-summary subset, not the full test catalogue or a representative fleet sample.

The public page's aggregate metrics may share an upstream calculation. The local audit preserves:
- 24 rows with both nominal capacity and published efficiency;
- 23 within the current rounding screen;
- 1 discrepancy (iCAUR V23 uphill);
- 2 unavailable comparisons;
- 26/26 published range-proxy checks within the rounding screen.

The mismatch is intentionally not corrected. It could reflect nominal versus usable capacity, SOC calibration, calculation basis, transcription or publication error. A public-summary consistency screen is not independent physical validation.

Source: https://charge.co.za/n3-real-world-ev-tests/

### NLR heavy-duty depot profile archive

NLR Catalog submission 162 publishes an 88 KB ZIP containing 81 modeled 24-hour profiles with 15-minute average demand:
- fleets: beverage delivery, warehouse delivery, food delivery;
- sizes: 10, 50, 100 EVs;
- strategies: immediate 100 kW, delayed 100 kW, minimum power;
- profile types: average, minimum-peak, maximum-peak.

Dataset DOI: 10.7799/1787031  
Catalog: https://data.nlr.gov/submissions/162  
Upstream generator: https://github.com/NatLabRockies/hdev-depot-charging-2021  
Paper DOI: https://doi.org/10.1038/s41560-021-00855-0

These profiles are modeled aggregate scenarios, not metered sessions. Strict replication requires matching the pinned operation-day inputs, schedule segments, resampling with replacement, random-seed derivation, number of sampled days, energy assumption, charging strategies and extraction/aggregation procedure. Without those controls, the result is a cross-model comparison, not a replication.

### EVI-Pro Lite

NLR's EVI-Pro Lite API provides 24-hour fleet-level charging-load profiles at 15-minute steps, including breakdown endpoints by vehicle type and class. It offers a second public scenario baseline distinct from the heavy-duty NLR schedule-driven archive.

API docs: https://developer.nlr.gov/docs/transportation/evi-pro-lite-v1/

Use it only after matching scenario dimensions (fleet size, vehicle mix, weekday/weekend, adoption assumptions, charging access and temporal resolution). It is a public fleet-scenario baseline, not a substitute for a measured depot or a passenger trip energy label.

### Battery health field data

Nature Communications (2025) describes an open-source dataset of 300 diverse EVs over three years, with driving and charging records and multiple modalities. This is a stronger field-data anchor for Symthaea battery-health work than synthetic cell traces alone.

Paper and data description: https://www.nature.com/articles/s41467-025-56485-7

An Energy (2026) multi-vehicle study uses roughly three years of operating data from 100 Changan EVs. It reports mean RMSE 0.0564, MAE 0.0442 and MAPE 5.28% across 99 vehicles with the first 30% of samples used for training. This is a within-model study with separate per-vehicle models/hyperparameter choices; it is not a cross-brand or chemistry-agnostic SOTA result.

Paper DOI: https://doi.org/10.1016/j.energy.2026.142412

The benchmark question is not whether Symthaea obtains lower RMSE on a random split. It is whether it preserves calibrated uncertainty and safe capability bounds on held-out vehicles, chemistries, routes, temperature regimes and early-data budgets.

### Grid capacity and valet charging

A 2025 Applied Energy vehicle-valet study reports a 64% average grid-capacity reduction in its 30-vehicle case (5th/95th percentiles 55%/73%) and 13% lower levelized cost of charging (8%/24% bounds). This is a replication target, not our result.

DOI: https://doi.org/10.1016/j.apenergy.2025.126563

A valid comparison must include plug occupancy, vehicle repositioning, deadhead movements, driver/operator time, duty completion guarantees, charger power, site import capacity, charging efficiency and cost boundary. A stationary scheduler cannot claim a valet benefit if it does not model repositioning.

### Heavy-duty field operations

A 2026 SSRN preprint reports more than 400 trucks, nearly 40,000 tours and nine depots, with measured building loads and site-specific grid connection capacities combined with simulated fleet charging. It is a candidate external challenge but remains a preprint; methods, licensing, source data, measured/simulated variables and train/test splits require review.

Source: https://papers.ssrn.com/sol3/papers.cfm?abstract_id=7543981

### MCS interoperability

IEC 61851-23-3:2026 and SAE J3271_202503 define distinct megawatt charging system requirements/test scopes. Treat coupler, communications, cooling, cable handling, grid interface, interoperability and bidirectional operation as separate qualification lanes. A power-flow simulation or digital twin is not conformance evidence.

- https://webstore.iec.ch/en/publication/69016
- https://saemobilus.sae.org/standards/j3271_202503-sae-megawatt-charging-system-electric-vehicles

## 4. Matched baseline suite

Every evaluation release should freeze identical source data, splits, weather/tariff inputs, vehicle configuration, pack capability, charger constraints and compute budget.

### Passenger route-energy baselines
1. Persistence / previous-trip mean by model and direction.
2. Linear or generalized additive regression with speed, elevation, temperature and vehicle mass.
3. Physics-inspired longitudinal energy model with rolling resistance, drag, grade, drivetrain efficiency and regenerative-braking limits.
4. Gradient-boosted trees (LightGBM/CatBoost/XGBoost) with identical features.
5. GRU/LSTM/TCN and Transformer-style sequence models only when raw time series exist.
6. Hybrid residual learner over the physics baseline.

### Battery capability and SOH baselines
1. Coulomb-counting/voltage/temperature rule baseline where signals permit.
2. Incremental-capacity / differential-voltage features with transparent regression.
3. Gradient boosting and random forest.
4. GRU/TCN/Mamba/Transformer models with vehicle-level holdout.
5. Calibrated capability envelope, not just point SOH.

### Depot and charging-policy baselines
1. NLR published scenario outputs (archive replay target).
2. NLR EVI-Pro Lite scenario profile (matched only where dimensions overlap).
3. Immediate charging.
4. Delayed charging.
5. Minimum-power / valley-fill charging.
6. FCFS and earliest-departure-first scheduling.
7. Nearest-charger and shortest-route policies where route choice is part of the task.
8. Constrained deterministic optimization.
9. Robust or distributionally robust optimization with uncertainty sets.
10. Receding-horizon MPC with explicit departure-energy and grid constraints.
11. Valet-enabled scheduling only when vehicle repositioning is represented.

A learned policy should be compared against deterministic, physics-based and optimization baselines before its complexity is justified.

## 5. Required metric families

### Prediction accuracy
- MAE, RMSE, signed bias.
- P50/P90/P95 absolute error and worst-case underestimation.
- Load-duration-curve error and 15-minute ramp error for depot power.

### Energy and demand consistency
- Per-trip/per-profile daily energy integral error.
- Signed aggregate energy bias and sum of per-profile absolute energy errors (do not allow cancellation).
- Peak-demand error and P95 peak error by fleet/strategy.
- Arrival/departure SOC error, charge-energy/time error against independent meter boundaries.

### Uncertainty quality
- PICP and interval width at 50%, 80%, 90%, 95%.
- WIS/CRPS where distributions are available.
- Coverage by uphill/downhill, speed, temperature, model, pack age, duty type, depot, fleet size and strategy.
- Risk/coverage curve: how decision reliability changes as the system abstains on uncertain inputs.

### Decision quality
- False-feasible reserve-violation count/rate (hard non-compensatory gate).
- Departure-energy shortfall and missed-duty count.
- Unserved kWh, queue time, charger occupancy and session completion.
- Grid capacity required, peak import, demand charge and PV/storage utilization.
- Battery-degradation proxy only where backed by a qualified, explicit model.
- User/driver time, repositioning distance and compute/solver time.

### Evidence quality
- Authenticated source coverage and exact configuration binding.
- Freshness/staleness rejection rate.
- Calibration/measurement evidence completeness.
- Replay determinism and provenance receipt completeness.
- Missing fields reported as unavailable, never silently imputed.

## 6. Split design and anti-leakage

- Passenger route: split by physical trip campaign, day and vehicle identity; hold out whole trips first, then whole vehicle models and time blocks.
- SOH: train on early chronological data, validate on a later window, test on the remainder; additionally hold out vehicles and model/chemistry domains. A random sample split of cycles/windows from the same vehicle is not adequate evidence of transfer.
- Depot: hold out depot, vocation, fleet size, charging strategy and calendar block. Preserve full days and 15-minute sequence.
- Do not split windows from the same physical trip, charging session or operating day across train/calibration/test.
- Report seed-wise mean and dispersion; freeze seeds and hyperparameter-search budget.
- Keep a source-level benchmark manifest containing dataset DOI/version, raw archive hash, file hashes, schema version, license, split manifest, model/config SHA, code SHA, parameters and machine/compute budget.

## 7. Promotion gates

These are prospective targets, not current results:

- 100% replay of frozen benchmark inputs and reports.
- Zero false-feasible reserve violations in the frozen adversarial suite; any nonzero result blocks safe-feasibility promotion.
- Nominal 90% prediction interval coverage target at least 90% overall and no unreviewed undercoverage in high-risk uphill/high-demand/temperature strata.
- Non-compensatory reporting of worst-case and P95 errors, not only average MAE.
- Improvement over the strongest matched baseline on at least two relevant axes without regression on service completion, reserve safety or evidence rejection.
- No cross-dataset generalization claim without a genuine held-out dataset and evidence of comparable measurement semantics.
- No field-validated statement without raw, permissioned measurements and independent metering.
- For NLR strict replication, archive identity and operating assumptions must be independently re-established before comparing results.

## 8. Execution order

1. Finish the NLR 81-profile archive audit with its committed dependency lock and synthetic known-answer controls (Symtropy PR #7817).
2. Obtain and verify the official 88 KB archive, run the first real profile-contract audit, and preserve the raw ZIP and extracted per-file hashes.
3. Compare Symtropy workload candidates against all 81 profiles; report per-profile and grouped residuals.
4. Add load-duration-curve, ramp and tail metrics, then compare with EVI-Pro Lite on carefully matched fleet scenarios.
5. Extend Symtropy with a matched valet/charging-policy replication using exact assumptions and service guarantees.
6. Build the N3 raw-measurement partnership protocol (vehicle, pack, EVSE, meter, charger session, weather, road and driver consent).
7. Benchmark Symthaea battery capability and uncertainty on the open 300-EV dataset and the 2026 same-model study as two distinct regimes.
8. Qualify each MCS standards lane separately; do not collapse simulation, cybersecurity, interoperability and physical conformance into one score.

## 9. Claim ceiling

This document is a benchmark protocol. It does not establish an industry-leading result. The current Sol Atlas N3 corpus is a curated public-summary consistency audit, and the current Symtropy NLR profile adapter has been tested on deterministic synthetic archives only. The first real external score requires the official archive, exact provenance, matched baselines and independently reviewed data semantics.
