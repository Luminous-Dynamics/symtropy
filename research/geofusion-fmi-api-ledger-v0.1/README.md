# GeoFusion FMI API Exchange Ledger v0.1

**Tracking:** [GEOFUSION-FMI-API-LEDGER-001 / #6304](https://github.com/Luminous-Dynamics/symtropy/issues/6304), under [GEOFUSION-BENCH-001 / #6156](https://github.com/Luminous-Dynamics/symtropy/issues/6156).  
**Status:** schema and synthetic fixture audit slice; no FMI runtime execution.

## Why this layer exists

The existing [FMI trace verifier](../geofusion-fmi-trace-verifier/README.md) validates normalized clocks, step outcomes, retries, state-digest continuity and commit barriers. Its state digests are opaque caller-supplied strings. It does not prove that an importer captured those digests from actual serialized FMU state, that exchanged values crossed the FMI API, or that the package and selected platform binary match the declared model revision.

This package defines an adapter-neutral API-call ledger between an instrumented FMI importer/master and the existing validators. It composes existing owners rather than creating another master algorithm or energy accounting implementation:

- [#6198](https://github.com/Luminous-Dynamics/symtropy/pull/6198): normalized time/step/retry/barrier trace.
- [#6154](https://github.com/Luminous-Dynamics/symtropy/pull/6154): typed physical ports, carrier/boundary/source allocation and first-law arithmetic.
- [#6224](https://github.com/Luminous-Dynamics/symtropy/pull/6224): matched OMSimulator/FMPy and adjacent coupling/integrator comparisons.
- [#5871](https://github.com/Luminous-Dynamics/symtropy/issues/5871): generic multirate coupling-algorithm tournament.
- [#1467](https://github.com/Luminous-Dynamics/symtropy/issues/1467): broader FMI/SSP federation boundary.

## v0.1 evidence profile

The ledger records immutable package, modelDescription, and selected platform-binary identities; FMI/model/interface versions; a frozen value-reference map; the actual order and status of FMI calls; typed Float64 set/get values with exact IEEE-754 bit patterns; exact fmi3DoStep time arguments; and successful lastSuccessfulTime outputs only when defined.

Every exchanged value is bound to a named physical port, unit, carrier, boundary, source interval and (for coupling transfers) transfer identity. Each doStep is linked one-to-one to a normalized trace attempt and commit barrier. A physical balance is not recomputed here: the audit reads the already-owned energy contract fixture and checks the cross-reference.

The schema is deliberately strict (additionalProperties=false). v0.1 is a synchronous FMI 3.0.2 Co-Simulation profile with scalar Float64 physical ports and a declared seconds-to-master-nanoseconds mapping. It is not a generic schema for every FMI interface or data type.

### State and rollback semantics

FMI state capture is optional and conditional. Only an FMU that advertises both canGetAndSetFMUState and canSerializeFMUState can qualify serialized state evidence in this profile. The collector must hash the exact serialized bytes and record their byte length; the independent checker must recompute both. When these capabilities are absent, the ledger reports STATE_SNAPSHOT_UNAVAILABLE. It must not claim rollback, restore, deterministic replay or state-continuity qualification from caller-supplied text digests alone.

A matching SHA-256 proves only that the bytes examined by the checker match the recorded digest. It does not prove semantic correctness of an opaque FMU state or that a compromised, uninstrumented importer reported the right bytes. Runtime evidence requires an instrumented collector at the actual FMI C-API boundary and an independently controlled artifact root.

## Synthetic positive fixture

fixtures/positive-two-fmu-ledger.json demonstrates a source-to-heat-exchanger exchange over one synthetic one-second interval:

- geothermal thermal output / heat-exchanger input: 500,000 W;
- delivered thermal service: 480,000 W;
- declared heat loss: 10,000 W;
- net storage charge: 10,000 W;
- synthetic first-law residual: 0 W.

The fixture contains two federates and one normalized barrier. **All referenced FMU packages, model descriptions, platform binaries, master binary, trace and energy-contract artifacts are explicitly non-executable text/JSON stubs. They are test data, not valid FMUs, binaries or external solver results.** No FMI API was called and no model was simulated.

## Run the fixture audit

The production contract is intended to be implemented in Rust. The small Python script is test-only fixture tooling, not a production parser, runtime, adapter or dependency. It validates the JSON Schema, root-confined artifact byte length/digests, ordered call shape, exact Float64 encodings, one-to-one trace links, barrier membership and synthetic energy cross-reference.

```sh
python3 -m pip install --requirement research/geofusion-fmi-api-ledger-v0.1/tests/requirements.txt
python3 research/geofusion-fmi-api-ledger-v0.1/tests/audit_fixture.py
```

CI pins jsonschema and runs only this fixture audit. It does not compile Rust, load FMUs, execute FMI functions, run OMSimulator/FMPy, certify FMI conformance, or establish thermofluid validity.

## Frozen negative controls

fixtures/frozen-cases.tsv records 25 planned adversarial controls with expected REJECT or NOT_EVALUABLE dispositions. The current fixture audit checks corpus shape and unique case IDs; it does not yet execute these mutations against a production Rust validator. Their presence is a frozen test plan, not a 25/25 executed pass.

## Unsupported in v0.1

- FMI Event Mode / fmi3UpdateDiscreteStates and same-time event rounds;
- Intermediate Update callbacks and Scheduled Execution;
- asynchronous execution and fmi3Pending;
- non-Float64 and array-valued physical port exchange;
- hidden interpolation/hold policies and inferred clock mappings;
- uninstrumented runtime logs;
- state restore without exact serialized-state evidence.

Unsupported behavior must be reported as NOT_EVALUABLE rather than silently coerced or counted as a pass.

## Industry comparison boundary

FMI defines the component interface, not a universal co-simulation master algorithm. OMSimulator's oms_ma, oms_mav and oms_mav2 remain direct master candidates in #6224. FMPy is an importer and FMI 3.0 interoperability/smoke-test harness, not a like-for-like multi-FMU master. preCICE and SUNDIALS/ARKODE are adjacent coupling and component-integrator references; compare them on equivalent equations and participant adapters rather than putting them on the same undifferentiated leaderboard.

Keep mature FMI 2.0.5 runs distinct from exploratory FMI 3.0.2 runs while OpenModelica documents FMI 3.0.2 support as experimental. SSP 2.0.1 describes portable system topology and parameterization; this ledger covers runtime API evidence and does not invent another system-description format. The FMI Cross-Check repository is archived; its published compliance/reference-result methodology is historical context, not a live benchmark service.

## Qualification targets (not achieved results)

- 100% acceptance of frozen positive runtime ledgers;
- 100% rejection or explicit not-evaluable disposition for registered negative controls;
- zero accepted physical values without exact variable/unit/port/carrier/boundary/interval identity;
- zero false state-continuity claims when serialized state bytes are absent or inconsistent;
- one-to-one trace-attempt and API-call correspondence;
- no silent unsupported-feature fallback;
- all comparative results pinned to exact importer/master, FMU package, model description, platform binary, runtime, hardware, configuration and fixture revisions.

The intended performance objective is **time-to-qualified-result**, with accuracy, conservation, event-time correctness, coverage and evidence completeness as hard constraints. Raw speed or one weighted score cannot compensate for false qualification.

## Claim ceiling

A passing fixture audit validates only this synthetic schema/fixture relationship. A future passing Rust runtime validator may qualify the declared ledger contract. Neither result alone proves FMI-wide conformance, physical geothermal validity, plant performance, equipment safety, or superiority over industry tools.
