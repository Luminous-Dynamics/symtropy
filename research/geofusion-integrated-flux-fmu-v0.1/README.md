# GeoFusion integrated-flux FMI pair v0.1

**Tracking:** [#6407 — GEOFUSION-INTEGRATED-FLUX-001](https://github.com/Luminous-Dynamics/symtropy/issues/6407).  
**Dependency:** stacked on [#6341 — executable analytic FMI 3.0.2 FMU pair](https://github.com/Luminous-Dynamics/symtropy/pull/6341).  
**Status:** synthetic FMI 3.0 Co-Simulation producer/receiver prototype; awaiting CI and independent importer execution.

## Why this variant exists

The W-based baseline exchanges a point-sampled power value. That is useful for comparing sample-and-hold and prediction policies, but a point value is not the integral of energy over a completed communication interval.

This variant tests an alternate application-level FMI port contract:

- **Producer outputs cumulative energy (J)** at each committed communication boundary and a direct intervalThermalEnergy (J) output for the most recently completed interval.
- The master derives the energy increment from two successful cumulative readings, checks the Float64 endpoint-resolution bound, and cross-checks against the direct interval output.
- **Receiver inputs interval energy (J)** exactly once per committed interval. It reports useful-service energy, losses, storage delta and cumulative stored-energy state, all in J.
- The directed producer-to-receiver ordering is explicit: the producer step completes before its energy output is accepted by the receiver. A feedback system would require a separately qualified rollback/implicit algorithm.

These are ordinary FMI 3.0 Float64 variables plus SSP unit declarations. This is not a new FMI API and does not imply universal support for a new FMI feature.

## Synthetic model

The producer emits a sinusoidal thermal power signal:

Q(t)=500000+100000 sin(2πt/400) W

It analytically integrates the signal over each completed interval. The receiver partitions the accepted interval energy as 96% service, 2% loss and 2% storage. Its local first-law residual is recomputed with a Float64 tolerance based on machine precision.

The prototype deliberately does not claim:
- that an arbitrary producer's integral is physically correct;
- that a cumulative Float64 counter has unlimited precision;
- that receiver trajectory inside the interval is exact;
- serialized FMU state, rollback, restart or strong-coupling capability;
- physical geothermal performance or equipment qualification.

A scalar cumulative-energy output is not a serialized FMU checkpoint.

## FMI ports

| FMU | Variable | VR | Causality | Unit | Meaning |
|---|---|---:|---|---|---|
| IntegratedFluxSource | cumulativeThermalEnergy | 11 | output | J | Cumulative integrated source energy at committed time |
| IntegratedFluxSource | intervalThermalEnergy | 12 | output | J | Integral over the most recently completed interval |
| IntegratedFluxReceiver | intervalThermalEnergy | 21 | input | J | One interval-energy transfer to consume |
| IntegratedFluxReceiver | serviceEnergy | 31 | output | J | Service allocated in the interval |
| IntegratedFluxReceiver | lossEnergy | 32 | output | J | Loss allocation in the interval |
| IntegratedFluxReceiver | storageDeltaEnergy | 33 | output | J | Storage allocation in the interval |
| IntegratedFluxReceiver | storedEnergy | 34 | output | J | Cumulative stored-energy state sampled at interval end |

The receiver rejects missing inputs, negative interval energy, duplicate input set before a commit, and stale/repeated currentCommunicationPoint. A successful doStep clears the pending input, so the same energy cannot be committed twice without a new accepted input.

## Qualification

Build/package:

~~~sh
make clean package
~~~

Run the complete local qualification suite with pinned FMPy 0.3.32 and its dependencies installed:

~~~sh
make test
~~~

This runs the per-library FMI symbol/capability audit, deterministic SSP build/verify tests and the FMPy runtime handshake test.

The smoke validates both FMUs, executes two different communication intervals (0–30 s, 30–120 s), compares cumulative and per-interval source energy against the analytic integral, checks the producer/receiver lastSuccessfulTime chain, verifies exact interval energy allocation, and injects negative-J, duplicate-consumption, stale-time, reinitialization and stop-time-overrun controls. Each rejected API call must return FMI Error and leave the last committed participant outputs unchanged.

The workflow pins the FMPy wheel digest and retains the FMUs and artifact digests. A pass qualifies only this synthetic FMI interface and the tested tool/build/host combination.

## Next qualification steps

1. Bind the two FMUs into an SSP 2.0.1 package with the complete W/J connector map and per-port physical boundary metadata.
2. Derive a runtime API ledger from the observed FMPy calls and bind each interval-energy read/write to one source interval ID and commit barrier.
3. Compare the integrated-energy interface against the W/ZOH, causal predictor and exact-average oracle on identical time grids.
4. Add large-counter, reset-epoch, dropped-interval, duplicate-transfer, mismatch, and receiver retry tests.
5. Only then compare with the matched OMSimulator and adjacent preCICE / SUNDIALS references.

## Claim ceiling

This prototype is synthetic interface evidence, not a physical geothermal model, not a universal FMI conformance claim, not a master-algorithm win, and not a plant safety or performance qualification.
