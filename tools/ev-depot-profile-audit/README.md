# EV Depot NLR Reference-Profile Audit v0.1

Standalone Rust archive audit for the official NLR 2021 heavy-duty depot charging load-profile dataset.

## Source

- NLR catalog: https://data.nlr.gov/submissions/162
- Public 15-minute profile archive: https://data.nlr.gov/system/files/162/15min-depot-charging-profiles.zip
- Upstream generating code: https://github.com/NatLabRockies/hdev-depot-charging-2021
- Reference study: Borlaug et al., *Nature Energy* (2021), https://doi.org/10.1038/s41560-021-00855-0.

The catalog describes 81 modeled aggregate load profiles: 3 fleet vocations × 3 fleet sizes (10, 50, 100) × 3 charging strategies (immediate, delayed, minimum-power) × 3 profile classes (average, minimum-peak, peak-peak). Each profile should have 96 average-power bins at 15-minute resolution.

These are **modeled aggregate fleet load traces**, not observed charging events or individual vehicle telemetry.

## Run

```sh
cargo fmt --manifest-path tools/ev-depot-profile-audit/Cargo.toml --all -- --check
cargo test --manifest-path tools/ev-depot-profile-audit/Cargo.toml --all-targets

curl --fail --location \
  https://data.nlr.gov/system/files/162/15min-depot-charging-profiles.zip \
  -o /tmp/15min-depot-charging-profiles.zip

cargo run --quiet --manifest-path tools/ev-depot-profile-audit/Cargo.toml -- \
  /tmp/15min-depot-charging-profiles.zip \
  https://data.nlr.gov/submissions/162 \
  nlr-162-resource-version-2-2021-06-08
```

The last argument is a caller-declared resource-version label; the report hashes the exact archive and each internal CSV. The workflow does not publish or commit the source zip.

## Checks and outputs

The tool recognizes the upstream filename convention, validates the complete 81-key cross product, checks timestamps are exactly 00:00 through 23:45 at 15-minute intervals, and rejects missing/duplicate keys, malformed power columns, non-finite/negative power, incorrect interval count or gaps in the timestamp sequence. It explicitly ignores AppleDouble macOS sidecar entries under `__MACOSX/` and `._*` filenames; the archive-level report preserves the number ignored instead of misclassifying resource-fork metadata as profile data.

For each profile it emits the internal CSV SHA-256, fleet/size/strategy/profile class, total daily energy (`Σ kW × 0.25 h`), peak and mean power, nonzero/zero intervals and first/last nonzero interval. The archive-level report carries a ZIP SHA-256 and findings. The CLI exits nonzero if the expected 81-profile contract does not validate; invalid profiles and missing/duplicate keys are represented in the JSON report.

## Limits

This is a profile archive integrity and descriptive-statistics tool. It does not reproduce NLR's underlying model, explain every modeling assumption, validate feeder voltages, prove a real depot's grid capacity, or demonstrate that Symtropy performs better. Time-series error comparisons are only meaningful after aligning duty cycles, charging-window boundaries, service guarantees, vehicle charging behavior and energy assumptions. Do not compare RMSE or peak demand without recording those differences.
