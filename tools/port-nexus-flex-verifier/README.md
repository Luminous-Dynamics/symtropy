# Port Nexus Flexibility Admission Verifier

A small, standalone Rust verifier for the first port-compute workload-shifting experiment.

## Why this exists

A workload schedule can improve price or carbon metrics while creating a new aggregate peak, a local feeder overload, a point-of-common-coupling (PCC) envelope breach, a ramp violation, or a rebound peak after the nominal dispatch window.

This tool does not create a schedule. It independently replays a fixed job manifest against a reference placement and a candidate placement, then admits or rejects the candidate under declared PCC and feeder constraints.

## Run locally

From the Symtropy repository root:

```bash
cargo test --manifest-path tools/port-nexus-flex-verifier/Cargo.toml
cargo run --manifest-path tools/port-nexus-flex-verifier/Cargo.toml
```

The tool is std-only and has its own nested Cargo workspace so it does not alter the main Symtropy workspace membership or dependency graph.

## v0.1 checks

- Exact job identity set: each job appears exactly once.
- Fixed job power, duration, release/deadline window and useful-work obligations.
- Complete chronological PCC net-import reconstruction.
- Full-horizon no-new-peak check against the baseline schedule.
- Per-slot PCC import envelopes.
- Per-slot feeder import envelopes.
- Adjacent-slot absolute PCC ramp envelopes.
- Baseline feasibility before evaluating the candidate.
- Finite/range checks and fail-closed malformed schedule handling.
- Energy, useful-work and violation receipts.

## Synthetic demonstration

The binary exercises five schedules:
- unconstrained TOU placement (rejected for a new 22 kW peak);
- no-new-peak TOU placement (admitted at 18 kW);
- unconstrained carbon placement (rejected for a new 23 kW peak);
- no-new-peak carbon placement (admitted at 18 kW);
- deferred work recovered together after the nominal dispatch window (rejected at 21 kW).

These are synthetic known-answer cases from the companion research fixture. They are not measured port loads, a global optimum, a South African operating profile, a reproduction of the cited Alibaba study, or a grid interconnection study.

## Claim ceiling and non-goals

A passing result establishes only arithmetic admission under caller-supplied one-zone PCC/feeder profiles and fixed job contracts. It does not prove that those profiles or limits are authentic or representative. It is not AC/DC power flow, N-1, dynamic voltage/frequency, protection coordination, storage/UPS/thermal state, workload queueing, migration, market accreditation, a field trial, regulatory conformity or dispatch authorization.

The verifier must remain independent of any optimizer. Optimizers may propose; this tool checks the declared constraints. A separate metrology pipeline must qualify PCC observations and uncertainty before measured field traces are admitted.
