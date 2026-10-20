# EV NLR Reference Profile Baseline v0.1

A standalone Rust reimplementation of the public NLR heavy-duty depot-charging strategy semantics, designed to emit auditable 15-minute load-profile outputs from the reconciled duty-cycle tables consumed by `ev-depot-duty-audit`.

## Source and reference

- Pinned upstream operating schedules: https://github.com/NatLabRockies/hdev-depot-charging-2021/tree/61530df51f6cd7d64f014ebbf8714cef24c8ca70
- NLR public 81-profile catalog: https://data.nlr.gov/submissions/162
- Source paper: Borlaug et al., *Heavy-duty truck electrification and the impacts of depot charging on electricity distribution systems*, Nature Energy (2021), DOI https://doi.org/10.1038/s41560-021-00855-0.

The published NLR bundle has 81 24-hour, 15-minute-average load curves: 3 vocations × fleet sizes 10/50/100 × strategies immediate/delayed/minimum power × profile types average/minimum-peak/maximum-peak. This utility outputs those same **81 curve types** via 27 source-vocation/size/strategy cases, each with 50 deterministically sampled fleet-days.

## Model semantics

- **Immediate:** fixed 100 kW charging at the start of an off-shift interval when enough energy is pending; if one off-shift interval is too short, charge for the whole interval and carry unserved energy across the next shifts.
- **Delayed:** same energy/capacity treatment as immediate, but when an off-shift interval can finish the accumulated request, place the charging interval at the *end* of that dwell.
- **Minimum power:** compute daily energy demand as VMT × explicit kWh/mile; divide by the day's total declared off-shift duration; apply that constant power across each off-shift interval. This is the published reference algorithm's simplified strategy, not a charge-controller recommendation.
- **Carry-over state:** immediate/delayed simulations repeat the selected duty-day segment sequence three times and report the middle day. Adjacent segments with the same on/off-shift state are merged before charging, including the day-wrap dwell. This carries pending energy into the next shifts and avoids treating a short segment as an independent full recharge.
- **Profile generation:** sum vehicle import power, compute the exact maximum one-second load, integrate into 96 × 15-minute bins, and preserve the NLR CSV convention of appending the day's first power value as the final second so the exported profile covers 24 hours.

The tool uses a stable, documented Rust LCG to sample operation-day profiles with replacement. It does **not** reproduce Python's random number generator and Pandas `DataFrame.sample` algorithm; therefore, its profiles should not be expected to match the published ZIP byte-for-byte even when strategy rules and parameters match. The source CSV input hashes, pinned source revision, random seed, and sample IDs are preserved in each report so results are reproducible within this implementation.

## Run

First produce the source audit JSON for the chosen vocation using `ev-depot-duty-audit`. Then:

```sh
cargo fmt --manifest-path tools/ev-nlr-profile-baseline/Cargo.toml --all -- --check
cargo test --manifest-path tools/ev-nlr-profile-baseline/Cargo.toml --all-targets

cargo run --quiet --manifest-path tools/ev-nlr-profile-baseline/Cargo.toml -- \
  --input-report nlr-fleet1-beverage-delivery-audit.json \
  --fleet-size 50 --strategy immediate --samples 50 --seed 0 \
  --kwh-per-mile 1.8 --charger-power-kw 100
```

The command emits JSON to stdout with p50/p95 daily energy and instantaneous peak distributions; average, minimum-peak and maximum-peak 15-minute curves; and per-sample operation-day IDs, energy demand, import energy, exact peak, and the 15-minute profile integral residual.

## Reproducible workflow

`.github/workflows/ev-nlr-profile-baseline.yml` audits all three pinned source vocations and generates 27 strategy/size/vocation cases, yielding 81 load curves in aggregate. It checks:
- all operation-day/schedule rows reconcile and preserve their segment inventories;
- all curves contain 96 nonnegative 15-minute average-power bins;
- per-sample 15-minute energy integral matches the 86,399-second charge energy within the one-second day-wrap bound;
- exact source/tool/output digests are emitted in an artifact; and
- source CSVs are downloaded at a pinned revision, not stored in this repository.

## Direct comparison against the published 81-profile bundle

The companion focused workflow now downloads the public 81-profile ZIP at run time, hashes the archive, and compares each matching average / minimum-peak / maximum-peak curve against the generated output. It records:

- 15-minute profile MAE, RMSE and maximum absolute error in kW;
- daily energy-integral difference (kWh) and relative energy error;
- 15-minute peak-power difference and peak-time difference in minutes.

The exact bundle SHA-256 is embedded in each comparison report and included in the checksum artifact; the ZIP and raw source tables are not committed to this repository. The reference CSV reader fails closed unless columns contain the expected time/power labels, there are exactly 96 quarter-hour records, and timestamps match 00:00, 00:15, ... 23:45.

This is a direct comparison layer, not yet proof of bit-for-bit reproduction. The Rust LCG samples with replacement rather than reproducing NLR's Python random/Pandas sampling; sample selection variance can therefore contribute to profile error even with matching algorithm rules. Review error distributions across all 81 curves, compare matched source assumptions, and isolate sampling error from algorithmic error before claiming superiority.

## Claim limits

These are **modeled reference-style profiles**, not measurements. The duty patterns are published operational summaries; charging windows, kWh/mile, 100 kW fixed power, and the Rust random sample stream are model parameters. Minimum-power does not enforce a vehicle/EVSE maximum input rating and remains an idealized reference baseline.
