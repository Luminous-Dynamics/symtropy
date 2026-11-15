# AeroWeft scope-join oracle

This dependency-free Rust crate checks whether existing PNT, reachability, intent and surveillance claims are joined to the exact operation scope and as-known-at cutoff requested by one evaluation.

It checks:
- operation, flight-instance, aircraft-instance, intent and authorization identity including exact authorization revision;
- profile revision, time scale, horizontal frame and vertical datum;
- owner evaluation time, event time, first-received time and the frozen decision cutoff;
- owner applicability windows;
- exact surveillance evaluation/query ID, airspace, cell, traffic class and requested interval;
- coverage validity, receipt lineage and dependency-snapshot presence.

It does **not** authenticate receipt bytes, qualify upstream sources, recalculate PNT freshness, convert clocks, calculate aircraft reachability, decide conformance, establish real sensor coverage, perform conflict geometry or issue operational authority. Those remain with the existing owners (#5270, #5275, #5163, #5284, #4705 and #5171).

The positive output is only `SCOPE_MATCHED_FOR_COMPOSITION`. It must be composed with Track O; it is not a conflict-free result, route clearance, dispatch approval or safety determination.

Run locally:

```sh
cargo fmt --manifest-path Cargo.toml -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
```

All data used by the tests is synthetic. Passing demonstrates only the declared scope-join contract over the finite test set.
