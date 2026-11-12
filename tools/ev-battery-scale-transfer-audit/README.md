# EV battery scale-transfer audit v0.1

Dependency-free Rust reference audit for exact staged lineage across battery material, electrode, coin-cell, pouch-cell, large-format-cell, module and pack evidence.

The audit requires every scale transition to declare which variables are invariant, rescaled, re-optimized or unknown, and binds predictions to physical transfer comparisons without allowing predicted evidence to impersonate measurement.

## Run from the repository root

    cargo metadata --manifest-path tools/ev-battery-scale-transfer-audit/Cargo.toml --locked --no-deps
    cargo fmt --manifest-path tools/ev-battery-scale-transfer-audit/Cargo.toml -- --check
    cargo test --manifest-path tools/ev-battery-scale-transfer-audit/Cargo.toml --locked
    cargo clippy --manifest-path tools/ev-battery-scale-transfer-audit/Cargo.toml --all-targets --locked -- -D warnings

This is a read-only evidence contract, not a scale-up physics model, test authority, cell/pack simulator, manufacturing controller or product-release system.
