# Manufacturing architecture portability — M0 profile contract

This folder starts [MFG-ARCH-PORTABILITY-001 / #6579](https://github.com/Luminous-Dynamics/symtropy/issues/6579).

## Scope

M0 defines a versioned, source-reviewable manifest for declaring a *composition* of industrial information layers. It is deliberately not a protocol implementation or a production runtime.

The architecture is layered, not a contest between mutually exclusive protocols:

- event transport / Unified Namespace (for example MQTT);
- machine information models (for example OPC UA Machinery);
- contextual manufacturing APIs (for example CESMII i3X);
- equipment observation documents (MTConnect);
- semantic/context models and relationships;
- digital-thread links between product, process, execution, inspection and twin evidence.

A profile may combine several layers. Each layer pins its own specification revision, implementation identity, read/write scope, semantics preserved, unsupported semantics and evidence/admission state.

## Files

- `profile.schema.json`: JSON Schema Draft 2020-12 for the M0 architecture-profile contract.
- `profiles/uns-mqtt.synthetic.json`: one synthetic-only event-transport profile fixture. It does **not** implement an MQTT broker or claim MQTT conformance.

## Hard boundaries

- The benchmark's initial client contract is read-only: write, command and product-release capabilities are structurally prohibited by the schema.
- Source documents, source-owned records and their digests remain authoritative. A normalized field is a derived view, not a replacement for its source.
- Unknown, stale, contradictory and unsupported semantics must be reported, not silently coerced.
- Standard publication, implementation existence, schema admission, protocol conformance and independent qualification are different states.
- A protocol-valid event does not prove source truth, calibration, process capability, part conformity, safety or authorization.
- The manifest is an integration-test declaration, not a cybersecurity assessment, safety case or standards certificate.

## Composition rules

1. Treat each layer as an independently versioned component.
2. Pin exact specification/format and implementation revisions; do not use the word "latest" as a version pin.
3. Record what the implementation actually supports and what it does not.
4. Preserve the native identity, units, frame, clock, sequence, calibration and source locator at every adapter boundary.
5. Preserve source-owned evidence references and explicitly identify the system that owns each authority.
6. Replaying identical admitted inputs, profile revision and evaluator version must reproduce the same normalized semantic result wherever deterministic replay is claimed.
7. Test protocol conformance, semantic preservation and task-level decision value as separate experiments.

## Next increment

The next step is to add reference-only profiles for i3X 1.0 and OPC UA for Machinery 1.04.2, then connect each profile to a shared frozen workload and independent evaluator. The i3X upstream conformance suite must be reused rather than reimplemented. MTConnect/QIF source admission remains owned by #3025/#3026/#3033; composition VVUQ remains owned by #3121; context and decision-value ablations remain owned by #5622.

No live broker, endpoint, CNC machine, robot, PLC, write method or physical asset is contacted by this M0 fixture.

## Reference profiles added in M0.1

- `profiles/i3x-1.0.reference.json`: source-reviewed i3X 1.0 API profile. The upstream suite is a separate external referee (60 conformance tests). When a concrete read-only endpoint exists, run the upstream CLI with `--no-writes`; do not expose or declare update capabilities. The upstream suite requires Node.js >=18.17 for its own test runner; this does not add a Node runtime to Foundry.
- `profiles/opcua-machinery-1.04.2.reference.json`: separately pins OPC UA for Machinery Part 1 v1.04.2, published 2026-11-01.
- `profiles/opcua-robotics-1.02.reference.json`: independently pins OPC UA for Robotics Part 1 v1.02. Do not conflate this with Machinery or infer robot safety conformance.

Reference profiles are source-reviewed specifications only. Their runtime semantics have not been implemented or tested. The `required_fields_to_preserve` list is a future adapter/evaluator requirement, not a claim that fields are currently preserved. `preserved_fields` remains empty and every profile stays `not_run`.

## Offline Rust profile validator

A standalone Rust validator lives in `validator/`. It pins the Draft 2020-12 implementation to `jsonschema = 0.62.0` and disables the crate's default remote/file reference-resolution features. The Foundry profile schema is self-contained, so profile validation must not retrieve schemas or other data from the network.

Run from the validator directory:

```sh
cargo test
cargo run -- ../profile.schema.json \
  ../profiles/uns-mqtt.synthetic.json \
  ../profiles/i3x-1.0.reference.json \
  ../profiles/opcua-machinery-1.04.2.reference.json \
  ../profiles/opcua-robotics-1.02.reference.json
cargo clippy --all-targets -- -D warnings
```

The validator performs both formal JSON Schema validation (including format assertions) and independent semantic checks for duplicate standard/layer identities, unresolved standard references, layer-kind/reference-role mismatch, invalid access paths, implementation/evidence-state promotion, reference-only source misuse, synthetic-to-conformance promotion and semantic-field contradictions.

The isolated validator has a committed Cargo v4 lockfile generated by the successful exact-head workflow's dependency resolution. The dedicated workflow runs `cargo test --locked`, `cargo run --locked` and `cargo clippy --locked`. The exact lockfile is checked into this PR, so transitive dependency resolution is frozen for this toolchain/target-independent dependency graph. The validator has no endpoint access and cannot establish runtime conformance.

## Portable observation projection and semantic referee

`portable-observation.schema.json` defines the exact candidate projection shape for a source-preserving observation. It requires the adapter-profile identity, original source-document digest, source schema identifier/version, machine and data-item identities, source and reference timestamps, clock mapping and uncertainty, native kind/unit/value, source locator and opaque-extension manifests.

The validator's `semantic_evaluator` compares candidate projections against evaluator-owned truth one required field at a time. It does not normalize or silently coerce values. The current test-only truth is one finite synthetic MTConnect sample record; it is deliberately compiled only in the test module. A passing comparison establishes exact agreement for that fixture only.

The negative controls cover wrong machine identity, missing sequence, altered unit, changed clock mapping, dropped opaque extension, and additional candidate fields. Additional fields are surfaced in the evaluator report rather than silently discarded. The candidate projection schema separately rejects undeclared root fields; unknown source-standard extensions belong in the explicit opaque-extension manifest.

The observation timestamp numbers in this fixture are synthetic and are not admitted as real plant-clock or physical measurement values. Time-unit and clock-domain claims still require source-specific clock and unit qualification. The exact comparator intentionally does not invent tolerances.

## Explicit cycle-to-inspection association referee

`inspection-join.schema.json` and the test-only `inspection_evaluator` extend the projection check to an explicit manufacturing-cycle ↔ QIF-result relationship. The strict record binds:
- exact cycle, QIF result, part, lot, operation and characteristic identities;
- cycle source, MTConnect source and QIF source document digests repeated inside the association evidence;
- association receipt identity, authority label, digest and validity interval;
- QIF result status, source measurement value/unit, derived value/unit and transform revision;
- source/reference timestamps, both clock-map digests, reference-clock identity and measurement window.

The evaluator independently compares the association's bound source digests, cycle and QIF result IDs against the candidate's cycle and QIF records. It checks measurement and decision times against both cycle and association effectivity.

For the finite fixture, combined timestamp uncertainty is recomputed as the **sum of the absolute cycle and QIF clock uncertainty bounds**. This is deliberately conservative when dependence/correlation has not been qualified; it does not assume statistical independence and does not use root-sum-square. The supplied combined value must exactly equal this reconstruction and remain below the declared maximum.

Important claim ceiling: all values and hashes are synthetic. The evaluator verifies structure and internal consistency; it does not cryptographically authenticate the association authority, prove that its clock mappings are correct, qualify the CMM, recompute metrology uncertainty, interpret the QIF spec, assess conformity, or authorize product release. The production receipt/authority owners must independently admit those claims.

## QIF inspection join follow-on

The `inspection-join.schema.json` and `inspection_evaluator` define a strict, synthetic-only cycle-to-QIF relationship. The association must bind exact source document digests, cycle ID, QIF result ID, part, lot, operation and characteristic. The evaluator checks that the measurement falls within the cycle window and association effectivity, and that the association is valid at both measurement and decision time.

The evaluator reconstructs the combined clock uncertainty as the sum of the declared absolute cycle and QIF uncertainty bounds. This is a deliberately conservative rule when correlation is not characterized; it does not assume independence or use root-sum-square. The submitted combined value must match the reconstruction and remain under the declared maximum.

The evaluator still does not authenticate the association issuer, validate the source clocks, qualify metrology, determine conformity or authorize release. All current join values are synthetic.

### Executable inspection-join validator

From `tools/manufacturing-architecture-portability/validator`:

```sh
cargo run --locked --bin validate-inspection-join -- \
  ../inspection-join.schema.json \
  tests/fixtures/qif-cycle-join.candidate.json
```

The command validates the candidate against the strict Draft 2020-12 schema, independently checks exact cycle/QIF/association identity and timing relationships, recomputes the conservative clock-uncertainty sum and emits a machine-readable JSON disposition. A schema or semantic failure exits non-zero. It is read-only and has no live endpoint or mutation capability.

## Standards metadata and the NIST TIDAL benchmark direction

The OPC UA reference profiles pin the standard *and* the model identity:
- Machinery: namespace `http://opcfoundation.org/UA/Machinery/`, model 1.04.2, publication date 2026-11-01, and the exact 1.04.2 NodeSet URI.
- Robotics: namespace `http://opcfoundation.org/UA/Robotics/`, model 1.02, publication date 2025-09-08, and the exact 1.02 NodeSet URI.

The published NodeSets describe the complete namespace. A server may expose a declared subset; any live adapter must report its actual server namespace metadata and subset status and must not assume complete NodeSet implementation from the standard reference alone.

NIST's TIDAL direction layers intelligence across asset, site/area and enterprise ISA-95 levels. Its planned integration bench explicitly compares UNS/MQTT + knowledge graphs/type models, i3X and OPC UA profiles, with the Tennessee Eastman Process as the initial test case and planned extension to NIST digital-twin and additive-manufacturing test beds. It evaluates accuracy, latency, cost and grounding; fault-detection recall is emphasized ahead of precision. The Foundry tournament should preserve that recall-oriented fault-detection stratum while reporting false alarms, decision cost, grounding and latency separately. The NIST programme currently describes these as a developing test bench and future software products; Foundry should align when its public fixtures/harness are released rather than imply that those future datasets are already admitted.

## i3X conformance policy

The i3X 1.0 upstream suite covers 60 MUST and declared MAY behaviors, including transport, `/info`, object/relationship exploration, current/historical queries and subscription lifecycle. Update (write-back) tests run when a server declares update capability. For Foundry read-only profiles, the conformance command must include `--no-writes`; update functionality remains unverified and must not be advertised. The official suite is a protocol/API conformance referee, not a manufacturing semantic/identity or physical-quality oracle.

## Inspection metrology evidence boundary

The QIF join projection also carries:
- measurement evidence class (`synthetic`, `recorded`, `physical_observation`);
- instrument calibration disposition and an optional exact calibration reference ID/digest;
- measurement-uncertainty disposition and an optional exact uncertainty/MSA reference ID/digest.

A `qualified_reference` status requires a non-empty reference ID and 64-character digest. This is only a structurally present reference; the evaluator does not authenticate the issuer, resolve the canonical source, or establish that the calibration/uncertainty evidence is applicable to the exact instrument, range, time, method, lot and characteristic.

The current QIF join fixture deliberately declares both calibration and measurement uncertainty as `unknown`, with no reference. It is still admissible as an internally coherent **synthetic identity/time join**, but the CLI reports `metrology_reference_state=unknown_or_unqualified` and `physical_metrology_qualified=false`. Supplying both qualified-reference fields changes the report to `references_present_not_authenticated`, never to physically qualified.

The first fixture evaluator accepts only `measurement_evidence_class=synthetic`. Recorded/physical evidence requires a distinct owner-admitted profile, source-rights and signature/authority checks, and the canonical calibration/metrology owners; it cannot pass through the synthetic lane.

The QIF inspection join follows existing metrology owners rather than inventing a calibration/uncertainty engine:
- Symtropy #2995 owns traceable measurement uncertainty budgets, Gage R&R, calibration, drift, covariance and verified combined uncertainty.
- Mycelix #4704 owns immutable inspection criteria and the independent qualification-attestation boundary; the measurement record must not supply its own permissive acceptance limits.
- Mycelix #6034 owns numerical instrument capability, calibration and sample-level measurement evidence.
- Symtropy #4947 owns source/master clock transforms and multirate event-time uncertainty.

The portability adapter carries only exact references and explicit evidence states. It does not recompute the MSA budget, authenticate an attestation, certify the instrument or classify the part. The synthetic fixture therefore labels measurement evidence as synthetic and calibration/uncertainty as unknown.

A supplied `qualified_reference` must also declare its subject/applicability:
- calibration attestation: exact instrument/resource ID and validity interval covering both measurement and decision time;
- uncertainty budget: exact characteristic, original measurement unit and validity interval covering both measurement and decision time.

The evaluator rejects wrong instrument, wrong characteristic, wrong unit, or expired/future calibration/uncertainty references. Even a structurally valid, correctly scoped reference pair is reported as `applicability_matched_references_not_authenticated`; `physical_metrology_qualified` remains false. Authentication, accreditation/authority scope, calibration-chain traceability, MSA budget reconstruction, covariance and conformity remain upstream owner responsibilities (#2995, Mycelix #4704/#6034).

## MTConnect stream observations versus top-level Assets

The portable observation projection now admits only MTConnect `Sample`, `Event` and `Condition` stream records. MTConnect `Asset` objects are a separate top-level resource/document class and must use `portable-asset.schema.json` with the distinct Assets 2.8 JSON Schema identifier, source document digest, raw asset digest and source locator.

A test-only synthetic CuttingTool Asset fixture is validated by the asset schema and deliberately rejected by the stream-observation schema. This consumes the existing MTConnect/QIF mapper semantics in #3025/#3026/#3033; it does not create a second asset registry or parser.

For a `qualified_reference` claim, calibration and uncertainty records also bind their validity intervals to an explicit clock identity. The current evaluator requires that clock to equal the QIF measurement's declared reference clock; both the mapped measurement time and decision time must fall inside the validity interval. It does not compare raw source timestamps against a reference-clock interval. Clock mapping correctness remains owned by the clock/metrology evidence path (#4947); #6579 only checks the supplied identities and time envelope.

## External NIST integration anchors

Two NIST assets should be separate benchmark strata, not pooled into one synthetic/physical dataset:

- **Tennessee Eastman Process (TEP):** NIST identifies this as the initial test case for its planned SI-for-Manufacturing integration bench. Use it for continuous-process fault detection/diagnosis and contextualization, with its own process model, fault labels, source/rights manifest and baseline. Do not mix it into discrete CNC/QIF acceptance statistics.
- **NIST Smart Manufacturing Systems (SMS) Test Bed:** its CAx Lab spans CAD/CAM/CAI/PDM/V&V, while its Manufacturing Lab includes CNC milling/turning and inspection equipment, with machine data collected using MTConnect. This is the more directly relevant source candidate for a future physical/equipment information-architecture stratum. External data admission, schema/release pinning, data rights and source digests remain prerequisites.

NIST's TIDAL benchmark plan emphasizes accuracy, latency, cost and grounding, and weights fault-detection recall ahead of precision. Foundry should publish recall and precision together with false-alarm burden, abstention, decision cost and per-fault-family results. NIST's benchmark environment, agent harness and evaluation toolkit are future programme deliverables (FY28 in its current plan), not a released harness that this PR has run.
