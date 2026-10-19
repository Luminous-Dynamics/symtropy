# EV Depot Duty Audit v0.1

Standalone Rust validation and descriptive characterization of operational-day and shift-segment CSVs from the public NLR/NREL heavy-duty truck depot-charging study.

## Research source

The archived public source repository is https://github.com/NatLabRockies/hdev-depot-charging-2021. Its source revision checked while preparing this adapter is `61530df51f6cd7d64f014ebbf8714cef24c8ca70`. For each fleet type, use:

- `data/fleet-schedules/<fleet>/veh_op_days.csv`
- `data/fleet-schedules/<fleet>/veh_schedules.csv`

The operating-day table contains vehicle miles traveled (VMT) and on/off-shift seconds. The schedule table contains per-day shift segments. The repository documents the original Borlaug et al. study: *Heavy-Duty Truck Electrification and the Impacts of Depot Charging on Electricity Distribution Systems*, Nature Energy (2021), DOI https://doi.org/10.1038/s41560-021-00855-0. The NLR data catalogue provides 81 *modeled* 15-minute charging profiles—three fleets × three fleet sizes × three strategies × average/minimum/peak-day types—not 81 collections of metered vehicle sessions: https://data.nlr.gov/submissions/162.

This tool does **not** copy the source data into this repository. Retrieve files from the pinned revision, review applicable license/terms, and keep hashes with the run output.

## Run

```sh
cargo test --manifest-path tools/ev-depot-duty-audit/Cargo.toml --all-targets

cargo run --quiet --manifest-path tools/ev-depot-duty-audit/Cargo.toml -- \
  tools/ev-depot-duty-audit/fixtures/veh_op_days_synthetic_v0_1.csv \
  tools/ev-depot-duty-audit/fixtures/veh_schedules_synthetic_v0_1.csv \
  --fleet-id synthetic-example \
  --dataset-id synthetic-fixture-v0.1 \
  --source-uri https://github.com/NatLabRockies/hdev-depot-charging-2021 \
  --source-revision fixture-v0.1 \
  --evidence-class synthetic \
  --kwh-per-mile 1.8
```

For an actual fleet, replace both fixture paths with downloaded source files, use a pinned upstream commit SHA as `--source-revision`, and set `--evidence-class published_operational_summary`. Record the exact retrieval and license conditions. The evidence class is caller-declared; a hash proves byte identity, not source authenticity.

## Checks and outputs

The tool validates required columns, row parsing, unique positive operation-day IDs, finite non-negative VMT, nonnegative shift durations, the source day length (86,399 seconds from 00:00:00 through 23:59:59), valid clock ranges, declared segment lengths, day partition continuity, on/off-shift reconciliation, and VMT reconciliation. Each input row is classified as parsed-valid or invalid; day-level reconciliation issues are separately listed rather than silently discarded.

It reports per-day first-on-shift start and last-on-shift end seconds for every reconciled operation day, alongside source-file SHA-256 digests; VMT and off-shift-hour distributions across reconciled operation-day records; and an idealized minimum average recharge power of `VMT × kWh-per-mile / off-shift-hours`. This last metric is an **assumption-bound lower bound**, not a measured charging rate: it presumes all off-shift time can be used for depot charging. When no off-shift window exists, the power statistic remains unavailable rather than dividing by zero.

## Limits

- This is a source-data audit and workload-characterization tool, not a fleet simulator, scheduler, controller comparison, AC load-flow/voltage model, battery model or charger-capability model.
- Operational schedule records do not by themselves prove a vehicle is continuously at the depot, physically connected, or available to charge through all off-shift seconds.
- The `kwh-per-mile` parameter is an explicit assumption. The NLR profile-generation code used 1.8 kWh/mile in its documented defaults, but this is not a universal vehicle consumption factor.
- The source study's U.S. duty patterns and charging strategy outputs must not be treated as evidence of South African fleet performance without a separate local validation study.

## Reproduce the pinned NLR input locally

The CI integration check fetches only these two small CSV tables from the pinned public repository revision; it does not commit or publish the raw tables:

```sh
REV=61530df51f6cd7d64f014ebbf8714cef24c8ca70
BASE="https://raw.githubusercontent.com/NatLabRockies/hdev-depot-charging-2021/$REV/data/fleet-schedules/fleet1-beverage-delivery"
curl --fail --location "$BASE/veh_op_days.csv" -o /tmp/nlr-veh_op_days.csv
curl --fail --location "$BASE/veh_schedules.csv" -o /tmp/nlr-veh_schedules.csv

cargo run --quiet --manifest-path tools/ev-depot-duty-audit/Cargo.toml -- \
  /tmp/nlr-veh_op_days.csv /tmp/nlr-veh_schedules.csv \
  --fleet-id fleet1-beverage-delivery \
  --dataset-id nlr-hdev-depot-charging-2021 \
  --source-uri https://github.com/NatLabRockies/hdev-depot-charging-2021 \
  --source-revision "$REV" \
  --evidence-class published_operational_summary \
  --kwh-per-mile 1.8
```

The current public source snapshot is expected to contain 76 operation-day rows and 234 schedule segments for beverage delivery, with all 76 days reconciling. These counts are a file-contract regression target for that exact pinned revision, not universal assumptions about other fleet tables or future datasets.
