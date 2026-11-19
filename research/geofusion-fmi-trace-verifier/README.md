# Geothermal–Fusion FMI Co-Simulation Trace Verifier v0.1

**Tracking:** [GEOFUSION-BENCH-001 / Symtropy #6156](https://github.com/Luminous-Dynamics/symtropy/issues/6156)  
**Status:** first executable trace-contract slice; synthetic fixtures only.

## Purpose

This dependency-free Rust package checks the **time and communication-step semantics** of a multi-federate co-simulation trace before that trace is combined with energy-stream arithmetic or used to support an engineering claim.

It verifies:
- stable federate, model-revision and clock-binding identity;
- exact rational mapping from each model's integer clock ticks to a common integer-nanosecond master clock;
- accepted, early-return and normalized-discard communication-step semantics;
- FMI importer permission for early return, actual `lastSuccessfulTime`, and unchanged-state checks for discard;
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

- `Accepted`: the FMI call did not set `earlyReturn`. The actual `lastSuccessfulTime` must advance the model clock and must not exceed the requested communication endpoint. It **need not equal** the requested endpoint: FMI 3.0.2 explicitly permits deviation for internal fixed-/variable-step solver reasons. The actual returned time, not the requested endpoint, anchors the next trace interval.
- `EarlyReturn`: the importer had enabled `earlyReturnAllowed`, and the returned time is strictly between the current and requested endpoint. All required federates must reach the same master-time barrier before the next communication sequence.
- An early return at the current communication point is valid FMI behavior in some event situations, but v0.1 has no same-time event-round record. It is classified as `ZERO_PROGRESS_EARLY_RETURN_UNSUPPORTED`, not as an invalid FMI trace.
- `eventHandlingNeeded` and `terminateSimulation` are explicitly rejected as unsupported trace paths until Event Mode and termination membership transitions are represented. If an event is signalled without Event Mode being enabled, it is rejected as a contract violation.
- `Rejected`: a normalized no-advance/discard disposition. Caller-supplied before/after state digests must match. Its trace end-time field is set to the canonical start time; it is not copied from the undefined output arguments of an FMI `fmi3Discard` call.
- A retry must have a new attempt ID, reference the immediately preceding rejected attempt, preserve its start state and not widen its requested horizon.
- Every commit barrier must contain exactly one terminal accepted/early-return attempt per required federate.

FMI source: [FMI 3.0.2, §4.1.3 and §4.2](https://fmi-standard.org/docs/3.0.2/). This verifier is stricter than the full standard in several deliberately unsupported areas. It is a bounded trace profile, not a complete importer/master implementation. It does not load FMUs, implement Event Mode / `fmi3UpdateDiscreteStates`, Intermediate Update callbacks, successful-step rollback/re-execution, Scheduled Execution, interpolation/hold, or wall-clock scheduling.

## Run

```sh
cargo test --manifest-path research/geofusion-fmi-trace-verifier/Cargo.toml
cargo clippy --manifest-path research/geofusion-fmi-trace-verifier/Cargo.toml --all-targets -- -D warnings
```

## Frozen corpus and claim ceiling

The frozen TSV corpus mutates a valid two-federate trace with wrong model identity, clock-map mismatch, rejected-step advancement or state mutation, early return without importer permission, zero-progress early return, unrepresented event/termination paths, invalid retry linkage/start/horizon, unresolved rejection, sequence gaps and barrier membership/time defects.

All examples are synthetic. Passing tests establish only the declared trace-verification behavior. They do not prove FMI standard conformance, physical model accuracy, energy conservation, geothermal output, fusion performance, grid stability, equipment safety or operational authorization.
