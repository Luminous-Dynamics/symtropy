# Lunar regolith feeder supervisor

A Rust, fail-closed **advisory supervisory policy** for a hopper + rotary pocket feeder with vibration assistance. It does not execute actuator commands and is not certified control or flight software.

## Motivation

Stepanova, Dreyer & Garvey (2026; DOI 10.1007/s44461-026-00021-y) report 480 parabolic-flight experiments at 0.16 g and 27–60 Pa. For ICN-LHT-1 in their apparatus, vibration-on tests sustained hopper supply to a rotary pocket feeder with measured Γ from 3.4 to 23.8; 80/84 vibration-off experiments arrested within seconds. Crucially:
- This is hopper replenishment of a rotary pocket feeder, not free gravity discharge.
- The exact critical value was not identified; the data only bound Γ_cr ≤ 3.4 because there were no tests between Γ=0 and Γ=3.4.
- The feeder setpoint primarily determined delivered rate; vibration influenced continuity and pocket fill.
- Measured vibration differed from command due to isolator/resonance effects; close the loop on accelerometer-measured Gamma.
- Supplementary agglutinate work identified vibration-induced size segregation and fragmentation during repeated handling.

## Policy

The supervisor separates two control questions:
1. **Keep hopper supply continuous:** use measured flow state, pocket fill and actual Gamma. No-flow/intermittent flow pauses metered withdrawal and increases vibration only within a material/apparatus-specific commissioned envelope.
2. **Meter delivered mass:** when pocket fill is adequate, adjust feeder speed to match delivered mass rate. Do not use vibration as a substitute for feeder metering.

It also:
- stops feed if critical sensors are stale/failed;
- stops and requests material review if configured segregation or fragmentation limits are crossed;
- abstains if a quality metric required by the configured acceptance profile is missing;
- never commands Gamma above the commissioned maximum or trims below the validated minimum;
- trims vibration down one step after repeated stable windows to reduce unnecessary vibration energy;
- resumes a paused feeder only after flow is re-observed as continuous.

## Commissioning boundary

The config must carry a named material and apparatus profile plus measured `gamma_min_validated` and `gamma_max_validated`. The 3.4 lower bound from the cited ICN-LHT-1 study is **not** a default for another simulant, hard vacuum, different wall/surface, fill level, hopper geometry or flight hardware. Each application needs its own characterization and risk review.

## Run

```sh
cargo fmt --manifest-path tools/lunar-regolith-feeder-supervisor/Cargo.toml -- --check
cargo test --manifest-path tools/lunar-regolith-feeder-supervisor/Cargo.toml
cargo clippy --manifest-path tools/lunar-regolith-feeder-supervisor/Cargo.toml --all-targets -- -D warnings
cargo run --manifest-path tools/lunar-regolith-feeder-supervisor/Cargo.toml -- examples/synthetic-supervisor-input.json
```

The CLI accepts a JSON path or stdin and emits a recommended decision. The example is synthetic only.

## Claim ceiling

This is a supervisory reference implementation and regression-test target. No hardware-in-loop test, actuator integration, closed-loop physical test, vibration safety qualification or lunar qualification is claimed.
