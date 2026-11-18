# HAPS Fleet Availability Evaluator

A small, dependency-pinned Rust tool for analytic HAPS fleet availability calculations with two separate common-mode failure domains.

## Run

From the repository root:

```sh
cargo fmt --manifest-path tools/haps-fleet-availability/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path tools/haps-fleet-availability/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path tools/haps-fleet-availability/Cargo.toml
cargo run --locked --manifest-path tools/haps-fleet-availability/Cargo.toml
```

Pass a different case JSON path as the first argument to the CLI.

## Model boundary

The evaluator distinguishes:
- independent per-HAPS availability;
- shared HAPS-only weather outage;
- shared external common-mode outage affecting both terrestrial and LEO fallback;
- terrestrial and LEO independent failures conditional on no external shared event.

It validates the stated independence and maintenance accounting assumptions. Planned maintenance must already be included in individual HAPS availability and is not subtracted again.

This is an analytic synthetic model, not a Markov reliability model, event-driven simulator, fault-tree analysis, weather model, maintenance scheduler, measured availability or SLA.