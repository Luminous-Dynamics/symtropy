# GeoFusion analytic FMI 3.0.2 FMU pair v0.1

**Tracking:** [GEOFUSION-ANALYTIC-FMU-PAIR-001 / #6337](https://github.com/Luminous-Dynamics/symtropy/issues/6337), parent [#6156](https://github.com/Luminous-Dynamics/symtropy/issues/6156).  
**Status:** first analytic executable-model prototype; physical model is synthetic.

## Purpose

This package builds two separate FMI 3.0.2 Co-Simulation FMUs:
- `GeothermalSource`: Float64 thermal-power output, value reference 11, W.
- `HeatExchangerBuffer`: Float64 thermal-power input, VR 21, W; service VR 31, loss VR 32 and storage power VR 33 in W; cumulative stored energy VR 34 in J.

The FMI 3.0.2 C-API headers are vendored verbatim from the official Modelica Association repository at tag `v3.0.2`. Exact upstream blob hashes, release commit and license provenance are recorded in `third_party/FMI-3.0.2/UPSTREAM.md`.

## Model and analytic known answer

The first scenario is deliberately simple:

`Q_geo = 500,000 W`  
`Q_service = 0.96 Q_in`  
`Q_loss = 0.02 Q_in`  
`dE_storage/dt = 0.02 Q_in`

For a constant 2-second interval:
- input energy = 1,000,000 J;
- service energy = 960,000 J;
- heat loss = 20,000 J;
- storage increase = 20,000 J;
- residual = 0 J.

The heat-exchanger FMU accumulates stored energy in joules. Power (W) and energy (J) are kept distinct.

## Build and execute

Requires a C11 compiler, `make`, `zip`, Python 3.12 and the pinned FMPy test dependency.

```sh
make clean all
python3 -m pip install --requirement tests/requirements.txt
python3 tests/fmpy_coupled_smoke.py
```

The Makefile compiles against the vendored official FMI headers with warnings-as-errors, packages the correct `binaries/x86_64-linux/` paths, includes model descriptions, source and upstream licenses, fixes archive timestamps using `SOURCE_DATE_EPOCH`, runs a direct FMI C-API harness and recomputes an independent arithmetic oracle.

FMPy 0.3.32 is pinned as an independent importer. Its model-description validator checks schema validity and model structure. The importer test loads both FMUs, instantiates them, performs typed Float64 exchanges and calls `fmi3DoStep`, then independently recomputes the energy balance.

## Ownership composition

- #6304 owns API-ledger validation and runtime evidence.
- #6198 owns normalized time/step/retry/barrier traces.
- #6154 owns physical carrier/boundary/source-interval and first-law contracts.
- #6224 owns matched external master comparisons.
- #5871 owns generic multirate co-simulation algorithms.

This package does not implement another master algorithm or replace the specialist physics owners.

## Current limits and claim ceiling

- The source FMU is a constant-power analytic source.
- The exchanger uses fixed allocation ratios and exact integration of constant storage power.
- Neither FMU advertises FMU-state get/set or serialization; rollback and serialized-state replay are not qualified.
- Event Mode, Intermediate Update, Scheduled Execution and asynchronous execution are unsupported.
- No geothermal resource, heat-exchanger hardware, plant, equipment safety, economic, physical calibration or SOTA claim is made.
- This first constant-power case is a packaging/interoperability smoke, not a sufficiently rich dynamic test for a numerical master-algorithm superiority claim. Add transients, event-aligned cases, held-out communication grids and a separately authored analytic reference before ranking masters.
