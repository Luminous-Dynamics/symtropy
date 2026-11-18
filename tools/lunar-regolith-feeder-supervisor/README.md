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

## State and acknowledgement boundary

The CLI is a single-step recommender. A real integration must:
- persist the supervisor state between calls;
- execute a recommendation only through a separately authorized actuator adapter;
- update the supervisor's stored command setpoints only after receiving a positive actuator acknowledgement;
- feed back measured accelerometer/encoder values, not requested values;
- treat command timeouts or ambiguous acknowledgements as unknown and stop new material withdrawal pending reconciliation.

The crate does not perform actuator I/O or implement a safety-rated state machine. Its last acknowledged setpoint is an input contract for the integrating system, not proof that a physical actuator moved. An Off or Stop recommendation during a quality/sensor hold should remain latched until an operator or higher-level authorized controller explicitly clears and reinitializes the state after inspection.

## Claim ceiling

This is a supervisory reference implementation and regression-test target. No hardware-in-loop test, actuator integration, closed-loop physical test, vibration safety qualification or lunar qualification is claimed.

## Actuator command acknowledgement ledger

The sibling module `command_ledger` is a pure Rust state machine for a future actuator adapter. It is not called by the advisory supervisor and does not send hardware commands.

It models:
- commissioning profile ID/revision and explicit Gamma/feeder-speed limits;
- monotonic command sequence numbers and command IDs;
- the previous acknowledged setpoints and sequence carried with every proposal;
- at most one outstanding mutation;
- explicit pending, unknown, applied, rejected-with-no-effect and partial response states;
- applied acknowledgements accepted only with fresh, independent, current-calibration readback matching the requested setpoints;
- rejected-with-no-effect accepted only when independent readback confirms the previous acknowledged state;
- unknown/partial outcomes that block new proposals until a fresh independently calibrated readback reconciles the actual state;
- stale/uncalibrated/invalid readbacks rejected, and observed state recorded as reconciled rather than falsely labelled as an applied command.

The ledger has regression tests for pending blocking, unknown-outcome blocking, duplicate/mismatched identities, partial application, stale readback, no-effect rejection, repeated rejection sequence gaps, and independent reconciliation.

**Still not implemented:** cryptographic command-payload digest, append-only durable journal, validated crash/restart recovery, actual network/fieldbus transport, device-side idempotency, heartbeat/watchdog and HIL testing. Command IDs are currently sequence-derived identifiers, not cryptographic digests. Do not connect this prototype to actuators or treat it as a safety interlock.

The requirements and acceptance plan are tracked in [#6103](https://github.com/Luminous-Dynamics/symtropy/issues/6103).
