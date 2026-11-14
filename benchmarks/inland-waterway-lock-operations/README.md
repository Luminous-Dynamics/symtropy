# Inland waterway lock operations v0.1

Dependency-free Rust reference evaluator for Symtropy #3347.

## Run

~~~sh
cargo test --locked --manifest-path benchmarks/inland-waterway-lock-operations/Cargo.toml
cargo clippy --locked --manifest-path benchmarks/inland-waterway-lock-operations/Cargo.toml --all-targets -- -D warnings
~~~

## V0 scope

- Vessel/chamber dimensional fit using declared clearance margins and a current water-depth observation.
- Freshness, calibration, datum, operator approval and lock-asset status gates.
- Lock-cycle water control-volume reconciliation, including upper/lower reach exchanges, total chamber+b saving-basin storage and declared losses.
- Optional pump-energy observation with an explicit meter boundary.
- Exact duplicate cycle replay is idempotent; changed payload for the same cycle ID is inconsistent.

The balance checker consumes declared or measured cycle inputs; it does not simulate chamber filling, gate hydraulics, basin equalization, pumping curves, sediment transport or vessel navigation. Water-saving basins are part of the combined storage boundary: internal basin/chamber transfers cancel and must not be counted as external inflow/outflow.

No function opens gates, operates pumps, dispatches vessels or issues a real passage authorization. All tests are synthetic.
