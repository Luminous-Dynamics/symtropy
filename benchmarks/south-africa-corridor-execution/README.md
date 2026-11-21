# South African Corridor Execution Benchmark

A dependency-free, read-only Rust reference validator for the FY2025/26 corridor scorecard published by Transnet Rail Infrastructure Manager (TRIM).

This is the first bounded implementation under [SOUTH-AFRICA-CORRIDOR-EXECUTION-001](https://github.com/Luminous-Dynamics/symtropy/issues/6473), composing the broader [Southern Africa freight baseline](https://github.com/Luminous-Dynamics/symtropy/issues/3243) and [South African transport data qualification](https://github.com/Luminous-Dynamics/symtropy/issues/1631).

## What it does

- Freezes the six corridor rows from the 2026 TRIM Operational Performance Review.
- Keeps each target and actual measure separate.
- Validates the exact v0.1 column contract, corridor set, uniqueness and percentage ranges.
- Derives target gaps in percentage points.
- Preserves the two OTD series under the exact source labels: **including 80s** and **excluding 80s**.
- Rejects unknown or incomplete inputs rather than filling missing values with zero.

## Run

From this directory:

```bash
cargo test --offline --locked
cargo run --offline --locked
cargo clippy --offline --locked --all-targets -- -D warnings
```

The package has no third-party dependencies and is isolated by its own Cargo workspace. A machine without Rust/Cargo can still review the CSV and manifest; the Rust tests must be executed before claiming a Rust pass.

## Industry comparators and target ladder

See [industry-comparator-register.md](industry-comparator-register.md) for current external references covering the CPPI, DCSA Port Call 2.0, 2026 integrated berth/quay-crane and robust yard optimization, railway disruption recovery, public ship-block scheduling data, and ITTC manoeuvring validation. It also defines matched-baseline rules and proposed advancement targets. Those targets are research goals, not achieved results.

## Source and scope

- Publisher: Transnet Rail Infrastructure Manager.
- Report: *2026 Rail Infrastructure Manager Operational Performance Review*.
- Observation period: FY2025/26.
- Source: https://www.transnet.net/getFile.ashx?id=27812
- The fixture contains aggregate corridor percentages, not train-level observations.

The report labels OTD as "including 80s" and "excluding 80s". The report narrative discusses exceptions, but the exact meaning and denominator rules of code 80 are not established by this fixture. The labels are retained verbatim; the two measures are never merged.

No route-level train capacity, allocated paths, spare paths, cargo movements, port-call events, customer delivery times, or private operator outcomes are inferred.

## Non-claims

This package is not:
- a railway timetable, dispatch, path-allocation or safety system;
- a Transnet integration or real-time feed;
- a port-call or terminal performance model;
- a route-capacity parser for Network Statement Annexures 33/33a/33b;
- evidence that Symtropy or any operator has improved real corridor performance;
- a state-of-the-art result or a physical/operational qualification.
