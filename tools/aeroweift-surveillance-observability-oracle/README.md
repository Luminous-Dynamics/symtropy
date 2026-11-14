# AeroWeft Surveillance Observability Oracle — Synthetic Slice 0.1

This dependency-free Rust workspace is a tiny finite reference oracle for issue #5275.

Golden fixture:
fixtures/aeroweift-surveillance-observability/tiny-coverage-gap-v0.json

## What it checks

- A coverage claim is scoped by traffic class, spatial cell, and half-open time interval.
- Coverage, environment, PNT/time and dependency-inventory gates remain separate.
- Independent receiver/sensor families are counted by failure-domain family ID, not by receiver count.
- A report becomes a qualified track only when it matches a qualified source claim for the same class/cell/tick and is current/integrity-qualified.
- Cooperative coverage cannot be re-used as non-cooperative coverage.
- A no-report result never becomes empty airspace, even when class-specific coverage is qualified.
- An invalid, stale or unsupported report leaves possible occupancy unresolved.
- The oracle can accept separately qualified cooperative and non-cooperative positive controls.

## Deliberate limitations

This is not a sensor model, ADS-B receiver, radar, multilateration engine, EO/IR detector, DAA system, conflict-resolution policy or flight authority. It does not produce detection probability, sensor range, threat trajectory or avoidance manoeuvre. It evaluates only whether supplied synthetic evidence claims support a traffic-class/cell/time observability assertion.

Future integration consumes upstream coverage and source claims from appropriately qualified sensor/service owners and supplies unresolved possible occupancy to #2279/#2297/#5163. It must not generate absent aircraft kinematics.

## Exact-head commands

cargo fmt --manifest-path tools/aeroweift-surveillance-observability-oracle/Cargo.toml -- --check

cargo clippy --locked --manifest-path tools/aeroweift-surveillance-observability-oracle/Cargo.toml --all-targets -- -D warnings

cargo test --locked --manifest-path tools/aeroweift-surveillance-observability-oracle/Cargo.toml

Source and tests are authored; no pass is claimed until exact-head CI completes.
