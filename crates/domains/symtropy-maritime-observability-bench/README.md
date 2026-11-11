# Symtropy Maritime Observability Adapter

A thin maritime asset/sensor adapter over `symtropy-sensor-observability-bench`. Pairwise interval-separation remains in the general crate; this adapter owns maritime identity mapping, sensor dependency admission, scenario replay and report translation.

## Run

```sh
cargo test --manifest-path crates/domains/symtropy-maritime-observability-bench/Cargo.toml
cargo run --manifest-path crates/domains/symtropy-maritime-observability-bench/Cargo.toml -- benchmarks/maritime-observability/maritime-observability-case-v0.1.json --source-commit <40-hex> --case-sha256 <64-hex>
```

## P0.1 semantics

- Binds vessel/engine/configuration identity, operating regime, sensor topology and a versioned fault taxonomy.
- Maps maritime measurement-principle identities to the generic evaluator's independent-principle groups.
- Retains common-cause dependencies separately; shared gateway/clock/power loss is replayed as an explicit outage scenario.
- Stale/unknown quality, expired/unknown calibration, derived/reconstructed values and unsupported source roles cannot contribute independent measured evidence.
- Replays every declared sensor/common-cause loss scenario through the generic evaluator.
- Emits a read-only, source/case-digest-bound report.

The adapter does not compute individual-fault detectability, train a classifier, infer root causes, qualify physical sensor installations or authorize actuation/maintenance/mode transitions. Synthetic fixture metrics are structural only.
