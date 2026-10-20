# EV Capacity Robustness Aggregator v0.1

Standalone Rust aggregator for repeated EV depot simulations. It converts a set of scenario-specific capacity frontiers into **empirical service reliability frontiers**, grouped by fleet vocation, fleet size, connector-release assumption and scheduling policy.

## Connector occupancy is a first-class uncertainty

A vehicle that finishes charging may remain plugged in until the driver returns. Assuming every connector becomes instantly reusable at charge completion can materially understate the port count and grid-capacity interaction. The matrix therefore separates two boundary policies: `release_on_charge_completion` assumes immediate connector turnover, while `hold_until_departure` reserves the port until the vehicle departs. Neither is a universal truth: the first is optimistic without verified unplug/repositioning logistics; the second may be conservative if a valet or automatic cable system can move vehicles. The current workload model does not implement a valet, connector relocation delay, or labor cost, so results are reported separately rather than blended into one average.

## Why this is a paradigm shift

A capacity that succeeds for one seed may fail for another combination of duty days. Reporting only the smallest feasible capacity in one scenario rewards a lucky sample. This tool instead requires a complete fixed-seed matrix and asks: "At what sampled grid budget did at least 95% of these resampled workloads meet both the service and energy targets?"

It preserves every failed seed and reports the Wilson 95% binomial interval beside the observed success fraction. The interval is an uncertainty descriptor, not a claim that 20 seeds guarantee population-level reliability. It assumes independent Bernoulli sampling for its interpretation, and cannot correct for model misspecification or nonrepresentative source data.

## Inputs

A tab-separated run manifest with five columns:

```text
fleet_size    seed    vocation    port_release_policy    report_path
10             0       fleet1-beverage-delivery    release_on_charge_completion    results/fleet1-beverage-delivery-fleet-10-seed-0-release_on_charge_completion.json
```

The actual columns are tab-separated, not space-separated. Each report must be a simulation JSON output from `tools/ev-fleet-resilience`, with an estimate-only scenario, an FCFS result and an earliest-departure-first result, plus matching 20-point frontiers. Each pair of policy results must use identical job IDs and requested energies.

## Run

```sh
cargo fmt --manifest-path tools/ev-capacity-robustness/Cargo.toml --all -- --check
cargo test --manifest-path tools/ev-capacity-robustness/Cargo.toml --all-targets

cargo run --quiet --manifest-path tools/ev-capacity-robustness/Cargo.toml -- \
  --runs-manifest runs.tsv \
  --output-json robust-capacity-report.json \
  --output-csv robust-capacity-frontier.csv \
  --expected-seeds 20 \
  --minimum-fleet-sizes 3 \
  --minimum-vocations 3 \
  --coverage-target 0.95
```

Report paths in the run manifest are interpreted relative to the process working directory. The input file must contain every seed from 0 through `expected-seeds - 1` for every vocation/fleet-size/connector-release combination. The matrix must contain both `release_on_charge_completion` and `hold_until_departure`. Duplicate coordinates, missing seeds, missing policies, mismatched job denominators, invalid energy accounting, missing/untrue power-limit flags, incomplete frontiers, or wrong evidence classes cause a hard failure.

## Outputs and metrics

For every vocation × fleet size × connector-release policy × scheduler:
- robust minimum *sampled* site power (kW) and kW per vehicle;
- 20 capacity points at 5% intervals of port-nameplate capacity;
- the number/proportion of workload resamples meeting 95% deadline completion and 95% battery-energy service at each point;
- all failed seed IDs, P05/P50/P95 service values, delivered/unserved energy distributions, observed peak import distributions and Wilson 95% binomial intervals;
- repeated-run controller metrics: deadline completion, energy service, misses, unserved energy, queue waits and maximum site import;
- exact SHA-256 digest of every simulation report and a digest of the run-manifest bytes.

The robust minimum is the first sampled capacity where the empirical feasibility fraction reaches `--coverage-target`. The value is **grid-resolution dependent**: with 20 evenly spaced points it is sampled at 5% increments, not an exact continuous optimum. A null value means no sampled capacity reached the target.

## Current matrix

The companion focused workflow runs 360 assumption-driven estimates (180 duty samples × two connector-release assumptions):
- three pinned NLR source vocations (beverage delivery, warehouse delivery and food delivery);
- fleet sizes 10, 50 and 100;
- 20 deterministic seeds per vocation/fleet-size/connector-release pair;
- one charging port per two trucks, rounded up;
- two connector-release boundary conditions: `release_on_charge_completion` (optimistic immediate connector turnover) and `hold_until_departure` (vehicle occupies its port for the entire parking interval);
- both FCFS and earliest-departure-first scheduling under each connector assumption.
- 150 kW declared port nameplate, scaled site nameplate, and fixed declared energy/SOH/temperature/efficiency/acceptance assumptions;
- FCFS and earliest-departure-first on identical jobs within each connector assumption.

The source data are the NLR/NREL duty-cycle tables pinned at `61530df51f6cd7d64f014ebbf8714cef24c8ca70`; they are published operational summaries, not metered charge sessions. Charge jobs are generated as `estimate` evidence from VMT, shift boundaries and explicit assumptions. Raw source CSV files are downloaded transiently for the run but are not included in the uploaded artifact.

## Industry anchors

- NLR's 81 modeled 15-minute depot-load profiles span three heavy-duty vocations, fleet sizes 10/50/100, immediate/delayed/minimum-power strategies, and average/minimum/peak-day profiles: https://data.nlr.gov/submissions/162 ; paper https://doi.org/10.1038/s41560-021-00855-0.
- NLR 2024 national operating summaries broaden duty/dwell patterns beyond the three 2021 tractor fleets: https://data.nlr.gov/submissions/231 ; https://doi.org/10.2172/2319200.
- Passow & Rajagopal (Applied Energy, 2025) found 64% mean grid-capacity reduction with a vehicle valet in their 30-vehicle-depot scenarios (5th/95th percentiles 55%/73%) and 13% lower levelized charging cost. This is a case-specific external replication target, not a result from this model: https://doi.org/10.1016/j.apenergy.2025.126563.
- Link et al. (SSRN preprint, 30 September 2026) use >400 trucks, nearly 40,000 tours and nine depots; they report 20–45 kWpeak per truck and 1.5–2.5 trucks per charging point as generally sufficient in the study. It remains a preprint and its assumptions are not universal: https://papers.ssrn.com/sol3/papers.cfm?abstract_id=7543981.

## Claim ceiling

This is an empirical robustness summary across deterministic resamples of assumption-driven simulations. It is not measured charger performance, a probability guarantee over all real fleets, a validated electrochemical/thermal model, AC feeder power flow or voltage validation, causal proof of policy superiority, a reproduction of the valet paper's result, or proof of South African deployment feasibility.
