# Manufacturing composition verifier (v0.1)

This is a deliberately narrow, offline, standard-library-only Rust verifier for a **single product / lot / operation** evidence chain. It checks structural consistency and explicit source references across a manufacturing digital-twin composition. It is not a general graph engine.

## Build and test

No Cargo workspace changes or third-party crates are required.

```sh
rustc --edition=2021 -D warnings --test tools/manufacturing-assurance/composition.rs -o /tmp/mfg-composition-tests
/tmp/mfg-composition-tests
rustc --edition=2021 -D warnings tools/manufacturing-assurance/composition.rs -o /tmp/mfg-composition
/tmp/mfg-composition tools/manufacturing-assurance/fixtures/valid-integrated.tsv
/tmp/mfg-composition tools/manufacturing-assurance/fixtures/valid-unified.tsv
/tmp/mfg-composition tools/manufacturing-assurance/fixtures/valid-federated.tsv
```

Or run `bash tools/manufacturing-assurance/run-tests.sh`.

## TSV format

The first verifier uses a strict pipe-delimited format so it can be compiled with only the Rust standard library. Fields cannot contain a pipe, newline or escape sequence. Blank fields are rejected. Lines beginning with `#` are comments.

- `META|schema_version|composition_id|revision|mode|context_id|valid_from_utc|valid_until_utc`
- `COMP|component_id|role|artifact_sha256|schema_id|schema_version|context_id|vvuq_evidence_id|peer_id`
- `NODE|kind|node_id|scope_key|component_id|evidence_id|event_time_utc`
- `EVIDENCE|evidence_id|class|subject_kind|subject_id|source_locator|authority|artifact_sha256|receipt_sha256|valid_from_utc|valid_until_utc|status`
- `EDGE|from_kind|from_id|relation|to_kind|to_id|evidence_id`
- `IFACE|interface_id|from_component|from_port|to_component|to_port|mapping_id|unit_from|unit_to|unit_transform|clock_from|clock_to|clock_transform|join_policy|evidence_id`

The manifest is order-independent. Exactly one META record is required. Component, node, evidence and interface IDs must be unique.

## Composition modes

- `integrated`: every component declaration uses the common `mfg.canonical/1` schema.
- `unified`: component schemas remain independently declared; each interface carries a typed, versioned mapping.
- `federated`: component schemas remain independently declared and every component has a unique peer identity.

These are structural test profiles, not assertions that this small verifier implements every normative requirement of ISO 23247-6:2026.

## Required component roles

`digital_thread`, `machine`, `inspection`, `physics`, `metrology`, `quality`, `intervention`, `authority`.

Each component references a `COMPONENT_VVUQ` evidence record whose subject is that exact component. VVUQ evidence must cover the full composition validity interval. A `SYNTHETIC_ONLY` receipt is accepted only to let the synthetic fixtures exercise the graph; it is not production qualification.

## Required lifecycle nodes

Exactly one each: PRODUCT, LOT, OPERATION, MACHINE_CYCLE, MODEL_RUN, SPECIFICATION, INSPECTION_RESULT, INSTRUMENT, MEASUREMENT, CALIBRATION, MSA_STUDY, UNCERTAINTY_BUDGET, CONFORMITY, ACTION_PROPOSAL, AUTHORIZATION, EXECUTION and POSTCONDITION.

All nodes use the exact `scope_key` format `PRODUCT_ID/LOT_ID/OPERATION_ID`; the verifier checks that it matches the unique PRODUCT, LOT and OPERATION node IDs. This v0.1 scenario supports one product / lot / operation. Every node names its producer component and an evidence record whose subject key is `KIND:id`.

The MEASUREMENT node must include a UTC `event_time_utc`. The CALIBRATION and MSA_STUDY evidence records must have validity intervals containing that measurement timestamp. The MSA_STUDY evidence must have `QUALIFIED` or `SYNTHETIC_ONLY` status. This explicitly links an observation to its instrument, applicable calibration, MSA study and uncertainty budget while leaving measurement-system qualification itself with the existing metrology owner.

## Required edges

- PRODUCT --HAS_LOT--> LOT
- LOT --PROCESSED_IN--> OPERATION
- OPERATION --OBSERVED_BY--> MACHINE_CYCLE
- OPERATION --SIMULATED_BY--> MODEL_RUN
- MACHINE_CYCLE --EXPLICITLY_LINKED_TO--> INSPECTION_RESULT
- INSPECTION_RESULT --HAS_MEASUREMENT--> MEASUREMENT
- MEASUREMENT --MEASURED_WITH--> INSTRUMENT
- INSTRUMENT --CALIBRATED_BY--> CALIBRATION
- MEASUREMENT --QUALIFIED_BY_MSA--> MSA_STUDY
- CALIBRATION --CONTRIBUTES_TO--> UNCERTAINTY_BUDGET
- MSA_STUDY --CONTRIBUTES_TO--> UNCERTAINTY_BUDGET
- MEASUREMENT --HAS_UNCERTAINTY--> UNCERTAINTY_BUDGET
- MEASUREMENT --COMPARED_TO--> SPECIFICATION
- MEASUREMENT --DISPOSED_BY--> CONFORMITY
- CONFORMITY --JUSTIFIES--> ACTION_PROPOSAL
- ACTION_PROPOSAL --AUTHORIZED_BY--> AUTHORIZATION
- AUTHORIZATION --AUTHORIZES--> EXECUTION
- EXECUTION --VERIFIED_BY--> POSTCONDITION

Every edge needs a distinct `EXPLICIT_ASSOCIATION` evidence record whose subject key is `from_kind:from_id:relation:to_kind:to_id`. No fuzzy or nearest-timestamp join is accepted. Extra edges are allowed only when both endpoints exist and their evidence references are valid.

## Typed interfaces

The fixture requires these component-role paths: machine→inspection, inspection→metrology, physics→quality, metrology→quality, quality→intervention and intervention→authority.

Equal unit or clock domains require `IDENTITY`. Different domains require a named `MAP:<versioned-id>` transform. Every interface must use `EXPLICIT_ID` as its join policy and must cite its own `INTERFACE_MAPPING` evidence receipt. The evidence subject must be the exact interface ID. This verifier checks declarations and compatibility; it does not execute unit or clock transforms or prove their numerical correctness.

## Evidence record rules

Evidence records must have a non-empty source locator, declared authority, lowercase 64-character hexadecimal digest references, a valid UTC validity interval, a matching subject and expected evidence class, and status one of `ADMITTED`, `VERIFIED`, `QUALIFIED` or `SYNTHETIC_ONLY`. Each evidence record must be referenced exactly once; orphaned or reused evidence IDs are rejected.

**Critical limitation:** this first slice validates digest shape and reference consistency only. It does not read or hash referenced artifacts, verify signatures, authenticate an authority, validate calibration truth, or establish the correctness of an association. The fixtures intentionally use deterministic sentinel digests and source locators under `fixture://`, and are labelled synthetic-only.

## Output and claim ceiling

A successful result is `MANIFEST_STRUCTURALLY_VALID`, not "VVUQ passed", "product conforming", "part released", or "manufacturing process qualified". The CLI prints explicit claim-ceiling flags. The verifier does not independently validate physical models, measurements, posterior distributions, decisions, authorization signatures or execution telemetry.

The synthetic offline authorization and execution nodes test lifecycle completeness only. They do not connect to a PLC, controller, machine, release system or live factory.

## Initial negative controls

Unit tests cover all three composition modes, missing machine-cycle/inspection association, cross-lot scope mismatch, unqualified VVUQ and MSA evidence, fuzzy joins, unit conversion without a declared mapping, wrong integrated schema, federated peer collisions, missing/wrong interface evidence, absent source locator, VVUQ validity expiry, calibration/MSA expiry, malformed calendar timestamps and digests, orphan evidence and deterministic replay.

## Next qualification work

1. Replace sentinel fixture digests and `fixture://` locators with exact source and receipt hashes for rights-cleared, independently admitted artifacts.
2. Add an independent artifact-byte and signature/authority verification adapter through existing evidence owners; do not silently fold these responsibilities into this structural verifier.
3. Add clock-transform residual bounds, executable unit transforms and an independently generated association oracle.
4. Expand from one-article v0.1 to multi-part/assembly and multi-operation scopes only after genealogy and route completeness semantics are independently qualified.
5. Add schema/format admissions for actual MTConnect and QIF instances; the mapper and schema-admission owner remain authoritative.


## Causal falsification known-truth benchmark

`causal.rs` is a separate offline, standard-library-only benchmark for [MFG-CAUSAL-001 #3152](https://github.com/Luminous-Dynamics/symtropy/issues/3152). It is not part of the structural manifest verifier.

The runner compiles and tests both programs:

```sh
rustc --edition=2021 -D warnings --test tools/manufacturing-assurance/causal.rs -o /tmp/mfg-causal-tests
/tmp/mfg-causal-tests
rustc --edition=2021 -D warnings tools/manufacturing-assurance/causal.rs -o /tmp/mfg-causal
/tmp/mfg-causal
```

It includes:
- a matched-pair cluster-randomized panel with heterogeneous known effects;
- exhaustive enumeration of all treatment assignments to check that the estimator's randomization expectation matches the finite-population ATE;
- a zero-effect time-trend fixture where before/after falsely reports improvement;
- an observational hidden-confounder fixture where treated/control outcome changes appear different and the placebo pre-period contrast exposes the same bias.

It is a synthetic method test. It does not estimate a real factory effect, validate a twin's causal model, handle arbitrary production interference, or authorize intervention.
