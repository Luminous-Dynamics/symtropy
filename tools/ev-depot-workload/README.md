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
