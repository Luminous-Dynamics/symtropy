# EV Depot Workload Generator v0.1

Converts the JSON report emitted by `ev-depot-duty-audit` into a simulator-compatible fleet charging corpus plus a separate provenance/assumption manifest. It uses Rust and has no runtime Python or JavaScript dependency.

## Why this is a separate layer

The duty audit validates the source operating schedule and VMT; it does not determine vehicle battery size, real end-of-route SOC, depot connection availability, charge acceptance or grid capability. This generator makes those assumptions explicit and tags every generated scenario as `estimate`, never `measured`.

The charging window is deliberately **last on-shift segment end -> first on-shift start on the next daily cycle**. Intermediate gaps are not treated as charging opportunities. Duty-day profiles are sampled with replacement using a specified seed, and the manifest records each sampled source operation-day ID.

## Build and test

```sh
cargo fmt --manifest-path tools/ev-depot-workload/Cargo.toml --all -- --check
cargo test --manifest-path tools/ev-depot-workload/Cargo.toml --all-targets
```

## Generate an input corpus

First run `ev-depot-duty-audit` on the two source CSVs to produce an audit JSON report. Then provide every modeling assumption explicitly:

```sh
cargo run --quiet --manifest-path tools/ev-depot-workload/Cargo.toml -- \
  --input-report nlr-duty-audit-report.json \
  --output-corpus nlr-estimate-fleet.json \
  --output-manifest nlr-estimate-fleet.manifest.json \
  --fleet-size 6 --seed 12345 \
  --kwh-per-mile 1.8 \
  --pack-capacity-kwh 400 --initial-soc-fraction 0.1 \
  --state-of-health-fraction 0.9 --temperature-capacity-fraction 0.98 \
  --temperature-acceptance-fraction 0.8 --charging-efficiency-fraction 0.92 \
  --maximum-port-input-power-kw 150 --maximum-battery-acceptance-kw 140 \
  --charger-ports 6 --site-power-budget-kw 600
```

The `--kwh-per-mile` value is not inferred from the source file. The report's earlier energy-per-mile assumption is stored separately so a difference is auditable. Choose a battery size with sufficient SOC headroom for all sampled duty-day energy requests; the program fails closed if any sampled row requires more energy than the chosen pack can accept between the declared initial SOC and 100%.

The generated corpus matches the input contract of `tools/ev-fleet-resilience`. Run that evaluator using the manifest-listed modeling inputs and preserve both the corpus and manifest. The corpus and scenario carry evidence class `estimate`, while the manifest preserves the source report digest and source metadata.

## Validation and interpretation

- Rejects an audit report containing invalid source rows, unresolved operation-day reconciliations, malformed source SHA-256 fields or incomplete source identity.
- Uses deterministic sampling with replacement, explicitly not a cryptographic random source.
- Derives a target SOC delta from daily energy demand divided by effective capacity (nominal pack capacity × SOH × temperature capacity fraction).
- Applies static acceptance and efficiency assumptions; it does not model battery discharge during the drive, transient thermal dynamics, charging equipment communication, AC feeder voltages, or stochastic outages.
- Uses a repeat-duty abstraction: input operation-day rows are independent samples, not verified consecutive days for the same vehicle.
- An idealized source-derived duty profile is not proof the vehicle is present, plugged in or permitted to charge throughout the modeled window.
- The output cannot establish the performance of a real NLR fleet, commercial vehicle or South African deployment. It is a transparent workload generator for controlled simulation and should be followed by measured/local validation.

The embedded source revision and report SHA-256 help reproduce the inputs; declared metadata is not cryptographically authenticated by this utility.

## Industry comparison plan: what constitutes a real improvement?

Use the following as separate, scope-labelled benchmark anchors:

- **NLR 2021 profile benchmark:** 81 modeled 24-hour, 15-minute-resolution profiles across beverage-delivery, warehouse-delivery and food-delivery fleets; fleet sizes 10, 50 and 100; immediate 100 kW, delayed 100 kW and minimum-power strategies; average, minimum and peak days. This is the natural first external profile-comparison target, but it is modeled aggregate demand derived from operating schedules, not measured individual charge sessions. Dataset: https://data.nlr.gov/submissions/162 ; paper: https://doi.org/10.1038/s41560-021-00855-0.
- **NLR 2024 national operating summaries:** broader medium/heavy-duty duty patterns, dwell, and normalized charging curves spanning depot-centric vocational fleets as well as transit and school buses. Import this next to broaden the source population beyond one beverage-delivery fleet: https://data.nlr.gov/submissions/231 ; report: https://doi.org/10.2172/2319200.
- **Applied Energy 2025 grid-capacity-reduction study:** compares low-capital-cost depot management strategies and reports that vehicle valets can reduce required grid capacity by up to 64% in its large 30-vehicle depot case. Treat this as a case-specific reference, not a universal target. A valid reproduction needs the same operating assumptions, cost boundary, service constraints and infrastructure model: https://doi.org/10.1016/j.apenergy.2025.126563.
- **2026 real-world-operation preprint:** reports an analysis using data from more than 400 heavy trucks and nearly 40,000 tours across nine depots, with measured building loads and site-specific grid capacities over more than three months. As of October 2026 this is a recent SSRN preprint, not a peer-reviewed benchmark release; first inspect methods, accessible data, inclusion criteria and uncertainty before using it as a challenge set: https://papers.ssrn.com/sol3/papers.cfm?abstract_id=7543981.
- **Interoperability boundary:** SAE J3271:2025 covers MCS couplers, communication/control, cable handling/cooling, grid interconnection and interoperability testing; IEC 61851-23-3:2026 defines requirements for MCS DC supply equipment within its scope. Simulated power feasibility does not establish standards compliance or field interoperability: https://saemobilus.sae.org/standards/j3271_202503-sae-megawatt-charging-system-electric-vehicles ; https://webstore.iec.ch/en/publication/69016.

## Proposed metric contract for matched comparisons

For each matched workload and controller, preserve:
- grid capacity required to meet a declared mission/service target (kW and kW/vehicle);
- on-time mission/departure fraction and unserved battery kWh;
- 15-minute site-load trace error against an external profile when a genuine comparable profile exists (MAE/RMSE in kW, energy integral residual and peak-demand error);
- p50/p95 waiting and charge-window slack;
- total energy losses, explicit cost boundary and curtailment; and
- wall-clock/solver status, feasibility, hard-constraint violations and all failed scenarios.

Compare FCFS/greedy, deadline-aware charging and a constrained optimization reference on identical jobs. Add immediate/delayed/minimum-power profile references before claiming an improvement over NLR; a valet/repositioning strategy is not implemented in this model and must not be credited to it. Run fleet sizes 10, 50 and 100, at least three vocations, average/peak-day workloads, and feeder/charger contingencies. For this current adapter, the six-vehicle result is a pipeline smoke/feasibility test—not a head-to-head industry result.

**Improvement threshold:** do not choose a universal headline percentage in advance. Reproduce the reference first, measure variance, define a materially useful improvement for each metric, and then preregister the comparison. The 2025 paper's 64% case-specific grid-capacity reduction is a useful external validation target only after its assumptions can be matched; it is not evidence that the current six-vehicle run has achieved 64%.

## Food-delivery sensitivity: vehicle energy use versus ports

The workflow `.github/workflows/ev-depot-food-sensitivity.yml` now adds a paired factorial sensitivity across three declared battery-energy intensities (1.2, 1.5 and 1.8 kWh/mile), fleet sizes 10/50/100, 30 identical seed numbers per stratum, and two physical port-count assumptions (one port per vehicle versus two vehicles per port). That is 540 scenario replications; every replication emits an FCFS/earliest-departure-first capacity frontier and empirical service rates up to 120 kW/vehicle.

The (kWh/mile) values are scenario assumptions, **not** estimates extracted from NLR's reported diesel vehicle miles. In a real EV fleet the conversion depends on vehicle mass, route, speed, weather, grade, auxiliary load and energy-recovery; the source CSVs do not provide those EV consumption observations. The 1.2/1.5/1.8 sweep asks whether the apparent food-vocation infeasibility is robust to this single assumption and whether port count changes the frontier with duty windows held fixed. It does not model a charger queue reservation system or depot repositioning/valet.

Use the output to answer two diagnostic questions:
- If lower (kWh/mile) removes missed departures and improves energy service while port mode barely changes the frontier, the current infeasibility is demand/window/vehicle-acceptance-limited for the modelled assumptions.
- If one port per vehicle materially improves deadline service or the feasible-capacity tail, port contention is a relevant bottleneck. If neither mode meets 29/30 scenarios, the correct conclusion is “no tested capacity meets this target within this design space”; do not extrapolate beyond the 120 kW/vehicle search or assert a physical limit.

The benchmark's 29/30 threshold is an **empirical pass-rate screen across 30 deterministic resamples**, not a calibrated 95% real-world service guarantee or a statistical confidence interval. Results should be compared by matched seed, consumption assumption and port mode.

## Robust capacity benchmark across three vocations

The companion workflow `.github/workflows/ev-depot-robustness.yml` now runs 30 deterministic resamples for each of the three NLR source vocations—beverage delivery, warehouse delivery and food delivery—at hypothetical fleet sizes 10, 50 and 100: 270 scenarios total, with FCFS and earliest-departure-first capacity frontiers for every case. Each source schedule table is audited independently at the same pinned upstream commit; raw CSVs are downloaded during the run but not committed into this repository.

The benchmark uses `ceil(fleet_size / 2)` ports (two vehicles per installed port as a first infrastructure assumption) and a 45 kW/vehicle upper grid connection. Pack capacities are explicit model assumptions chosen to cover the maximum daily VMT in each pinned vocation with the remaining SOC headroom: 400 kWh for beverage delivery, 600 kWh for warehouse delivery and 1,200 kWh for food delivery. This prevents the workload generator from silently clipping high-energy days; a later vehicle-specific energy/pack-sizing study should replace these coarse choices. All other thermal, SOH, efficiency, acceptance, port-power, and kWh/mile values remain declared assumptions.

The 45 kW/vehicle upper connection and two trucks per port are directionally comparable to the 2026 SSRN preprint's reported 20–45 kW-peak-per-truck planning range and 1.5–2.5 trucks per charging point, but this is **not** a like-for-like replication: that work uses >400 trucks and nearly 40,000 tours across nine depots with measured site loads, while this benchmark resamples three NLR source duty tables and assumes charging physics and infrastructure. The SSRN work remains a preprint: https://papers.ssrn.com/sol3/papers.cfm?abstract_id=7543981.

For each vocation × fleet size × policy, the workflow reports the median and P95 of per-scenario minimum sampled grid connection in kW/vehicle, counts of cases infeasible even at 45 kW/vehicle, P95 missed-departure fraction at the declared base connection, minimum energy-service fraction, and monotonicity diagnostics. It also computes the minimum sampled capacity where at least 95% of the 30 generated scenarios meet both the deadline and battery-energy service targets (at least 29/30 passes), separately for each vocation, fleet size and policy. Capacity steps are 5% of the base 45 kW/vehicle, so reported values are sampled estimates—not continuous minima. Thirty duty-profile resamples provide exploratory empirical pass rates, not a calibrated reliability guarantee or a confidence interval.

Why this matters: the six-vehicle and single-vocation smoke tests can hide scheduling differences and the diversity of real duty patterns. This multi-vocation, multi-scale, multi-seed suite now tests whether those results survive work-profile heterogeneity and fleet scaling. The next benchmark expansion should compare our outputs directly to the 81 modeled aggregate NLR 15-minute charging profiles and their immediate/delayed/minimum-power strategies: https://data.nlr.gov/submissions/162. The 2025 vehicle-valet paper reports an average 64% grid-capacity reduction for its particular 30-vehicle cases; that is an external replication target only after exact assumptions are matched, not a performance claim for this simulator: https://doi.org/10.1016/j.apenergy.2025.126563.
