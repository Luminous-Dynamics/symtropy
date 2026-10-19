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
