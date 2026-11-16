# Power Fabric Physical-Model Contracts v0.1

**Status:** schema and synthetic fixture proposal; no physical model is validated by this change.

Issue: [POWER-FABRIC-DYNAMIC-MODEL-001 #5607](https://github.com/Luminous-Dynamics/symtropy/issues/5607)  
Parent specification: [draft PR #5609](https://github.com/Luminous-Dynamics/symtropy/pull/5609)  
Related network assurance: [Sol-Atlas #908](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/908)

## Purpose

These strict Draft 2020-12 schemas make the join between an engineering model and the equipment it describes explicit:

`asset + as-built configuration + firmware + model revision + operating envelope + output quantity + validation evidence`

They are data contracts, not a solver, model admission service, certification authority or physical truth oracle.

## Contracts

- `physical-model-profile-v0.1.schema.json` describes one model's declared applicability and exact target configuration. Every output has its own validity axes, uncertainty declaration and claim ceiling.
- `model-validation-receipt-v0.1.schema.json` records one validation attempt. Each output is separately `pass`, `fail`, `unknown` or `not_evaluable`; the receipt preserves independent reference methods, negative controls and claim ceiling.
- `physical-model-profile-synthetic-gfm-v0.1.json` is a deliberately incomplete grid-forming inverter/weak-grid profile. Asset/module IDs are synthetic; hardware, firmware, calibration, model source, build and topology evidence remain unknown.
- `model-validation-receipt-synthetic-not-evaluable-v0.1.json` records that no solver, test protocol, reference waveform, laboratory test, HIL run or field observation exists.

The synthetic receipt must remain **not_evaluable**. It exists to prove that missing physical evidence can be represented without turning it into a passing validation claim.

## Semantic rules enforced in CI

The dedicated validator first checks both schemas against Draft 2020-12 and validates both fixtures. It also enforces cross-record invariants that JSON Schema alone cannot express:

1. Output IDs, port IDs and operating-axis IDs are unique.
2. Every output validity axis refers to a declared operating-envelope axis.
3. A profile cannot be `admitted` while required asset/configuration/model identities or operating limits remain unknown.
4. A validation receipt must refer to exactly the profile's declared output set.
5. A passing numerical output cannot exceed its declared tolerance; a failing one must exceed it.
6. A negative control cannot pass with the wrong observed disposition.
7. A `complete_pass` receipt requires an admitted profile, resolved source/asset/run/protocol identities, at least one independent reference method, all output checks passing and all negative controls executed successfully.
8. A `not_evaluable` receipt cannot contain only passing outputs.

The validator is CI-only Python, pinned to `jsonschema==4.23.0`; it is not a production model-admission dependency. The trusted engineering and assurance path remains Rust-first.

## Evidence classes are not a ladder

`ManufacturerDeclared`, `Modelled`, `AnalyticallyVerified`, `ReferenceDifferential`, `LaboratoryMeasured`, `HardwareInLoop`, `FieldObserved` and `IndependentlyReviewed` are distinct evidence classes. They are not interchangeable and do not imply one another. A signed review does not replace a measurement; an independent solver does not establish physical truth if both models share incorrect inputs.

## Explicit non-claims

This change does not establish:
- a validated GFM inverter model, RMS/EMT implementation or regime selector;
- an as-built asset, firmware image, calibration, weak-grid topology or measurement chain;
- numerical solver parity, HIL or laboratory correlation;
- NTCSA/grid-code acceptance, SANS compliance, interconnection approval or professional engineering sign-off;
- permission for live control or protection setting changes.

The next promotion step is to bind an actual qualified model source and asset configuration, define the operating envelope and test protocol, then add analytical and independent reference cases before lab/HIL evidence is attempted.
