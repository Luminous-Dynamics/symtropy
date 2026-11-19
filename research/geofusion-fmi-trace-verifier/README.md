# Geothermal–Fusion FMI Co-Simulation Trace Verifier v0.1

**Tracking:** [GEOFUSION-BENCH-001 / Symtropy #6156](https://github.com/Luminous-Dynamics/symtropy/issues/6156)  
**Status:** first executable trace-contract slice; synthetic fixtures only.

## Purpose

This dependency-free Rust package checks the **time and communication-step semantics** of a multi-federate co-simulation trace before that trace is combined with energy-stream arithmetic or used to support an engineering claim.

It verifies:
- stable federate, model-revision and clock-binding identity;
- exact rational mapping from each model's integer clock ticks to a common integer-nanosecond master clock;
- accepted, early-return and rejected communication-step semantics;
- retry identity and unchanged retry start state;
- contiguous per-federate step sequences;
- common commit barriers containing exactly one terminal step from every required federate;
- barrier monotonicity and common model-time alignment.

## Ownership and non-duplication

This package is deliberately narrower than the whole FMI benchmark.

- [#6156](https://github.com/Luminous-Dynamics/symtropy/issues/6156) owns the full FMI co-simulation benchmark and the future comparison against established master algorithms/importers.
- [PR #6154](https://github.com/Luminous-Dynamics/symtropy/pull/6154) owns energy-stream, carrier, source-allocation and first-law arithmetic contracts. This verifier does not reimplement them.
- [PR #6143](https://github.com/Luminous-Dynamics/symtropy/pull/6143) owns the geothermal–fusion system architecture and analytical energy benchmark.
- [#4487](https://github.com/Luminous-Dynamics/symtropy/issues/4487) remains the model-admission and escalation owner.

## Time model

Each federate retains its own clock identity and versioned clock binding. A binding maps integer local ticks to integer master-clock nanoseconds using an exact rational scale:

`master_ns = master_epoch_ns + (local_tick - local_epoch_tick) × numerator / denominator`

The mapping is accepted only when the result is integral and representable as i64. This deliberately rejects silent rounding, floating-point clock drift and implicit resampling.

## Step semantics

- `Accepted`: returned model time equals the requested endpoint.
- `EarlyReturn`: returned time is strictly between start and requested endpoint. The master barrier is placed at that returned time; every other required federate must also stop at the same master time.
- `Rejected`: returned time equals the start time; no model-time advancement is committed.
- A retry must have a new attempt ID, reference the immediately preceding rejected attempt, preserve its start state and not widen its requested horizon.
- Every commit barrier must contain exactly one terminal accepted/early-return attempt per required federate.

This is a bounded trace contract, not a complete FMI 3 importer/master implementation. It does not implement FMU loading, FMI callbacks, event iteration, intermediate update, rollback of opaque solver state, interpolation, or wall-clock scheduling.

## Run

```sh
cargo test --manifest-path research/geofusion-fmi-trace-verifier/Cargo.toml
cargo clippy --manifest-path research/geofusion-fmi-trace-verifier/Cargo.toml --all-targets -- -D warnings
```

## Frozen corpus and claim ceiling

The frozen TSV corpus mutates a valid two-federate trace with wrong model identity, clock-map mismatch, rejected-step advancement, invalid retry linkage/start/horizon, unresolved rejection, sequence gaps and barrier membership/time defects.

All examples are synthetic. Passing tests establish only the declared trace-verification behavior. They do not prove FMI standard conformance, physical model accuracy, energy conservation, geothermal output, fusion performance, grid stability, equipment safety or operational authorization.
