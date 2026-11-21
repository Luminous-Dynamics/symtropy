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

## Industrial intelligence scope (TIDAL stratification)

Every profile layer now declares one of:
- `asset` — machine, instrument, sensor or individual equipment context;
- `site_area` — line, cell, work area, facility or site-level context;
- `enterprise` — cross-site/organizational context;
- `cross_layer` — an API/transport layer that can expose or connect multiple scopes.

This lets the benchmark stratify identity, freshness, grounding and decision tasks by industrial context level, aligned conceptually with NIST's TIDAL asset/site-area/enterprise layering. It is a benchmark taxonomy only; it does not declare ISA-95 conformance or a TIDAL implementation. Actual scope is determined by admitted object relationships and source evidence, not by a profile label alone.

## OPC UA NodeSet provenance and dependency admission

The Machinery and Robotics reference profiles now pin the actual upstream normative NodeSet artifacts from OPCFoundation/UA-Nodeset commit `ed0b821f6a6f1f0c440c442ed95a9146ae8b7f8d`, including file path, Git blob SHA-1, raw-byte SHA-256, byte length, NamespaceUris and all RequiredModel records.

Crucially, the standard's NamespaceMetadata date and the UANodeSet Model's PublicationDate are separate fields:
- Machinery NamespaceMetadata: NamespaceVersion 1.04.2, publication date 2026-11-01. Its pinned NodeSet Model: Version 1.04.2, ModelVersion 1.4.2, PublicationDate 2026-11-02.
- Robotics NamespaceMetadata and NodeSet Model publication date: 2025-09-08; NodeSet ModelVersion 1.2.

The OPC UA NodeSet Admission CLI parses the complete XML artifact offline, rejects DOCTYPE, verifies the raw digest and byte length, checks the target Model and RequiredModels, confirms the namespace URI table, and checks namespace indexes used by NodeId/ParentNodeId/DataType/BrowseName attributes. The parser never fetches a URL. The CI workflow downloads only two hard-coded raw files at the exact pinned commit, checks their SHA-256, then invokes the offline parser.

### A discovered bundle-resolution blocker

Machinery 1.04.2 declares RequiredModel OPC UA core 1.05.07 (publication 2026-07-30); Robotics 1.02 declares OPC UA core 1.05.03 (publication 2023-12-15). Both declare DI 1.04.0 and IA 1.01.4. The bundle analyzer reports `requires_explicit_dependency_resolution` for the core model URI. It does not guess that newer means compatible, nor silently select one dependency. This does not block testing each model independently, but it blocks claiming that a combined Machinery+Robotics NodeSet bundle is dependency-resolved.

The server snapshot CLI separately compares expected NamespaceMetadata (NamespaceUri, NamespaceVersion and NamespacePublicationDate) against an observed read-only snapshot. If `IsNamespaceSubset=true`, the result is `matching_declared_subset`. If false, the result is still `matching_full_namespace_claim_node_coverage_unverified`: metadata is not proof that all required Nodes are present or semantically correct.

From the validator directory:

```sh
cargo run --locked --bin opcua-nodeset-admission -- verify \
  ../nodesets/opcua-machinery-1.04.2.nodeset.json /path/to/Opc.Ua.Machinery.NodeSet2.xml

cargo run --locked --bin opcua-nodeset-admission -- bundle \
  ../nodesets/opcua-machinery-1.04.2.nodeset.json \
  ../nodesets/opcua-robotics-1.02.nodeset.json
```

These checks prove exact artifact identity and declared metadata only. They do not import a model into a server, prove server NodeId coverage, perform UA server/client conformance, authenticate a server, validate cybersecurity, call methods, or establish physical-machine behavior.

## OPC UA NodeSet admission and server snapshot boundary

The Machinery 1.04.2 and Robotics 1.02 profiles now pin both the normative model declaration and the actual UANodeSet XML artifact:
- repository `OPCFoundation/UA-Nodeset`;
- exact source commit, source path, Git blob SHA-1, raw-byte SHA-256 and byte length;
- exact NamespaceUris table;
- UANodeSet Model URI, Version, ModelVersion and Model PublicationDate;
- complete RequiredModel URI/version/publication-date set.

The offline Rust parser uses `quick-xml 0.42.0`, disables optional features, prohibits DTD/DOCTYPE, and has no network capability. It verifies both raw SHA-256 and Git's canonical `blob <length>\\0` SHA-1. It parses Model / RequiredModel metadata and the UANodeSet Alias table; rejects duplicate/empty aliases; resolves aliases used by NodeId, ParentNodeId, DataType, ReferenceType and Reference targets; and validates namespace indexes in node attributes, BrowseNames, reference types, reference targets and alias values. Namespace index 0 denotes the OPC UA core namespace; nonzero NodeSet indexes are checked against the artifact NamespaceUris table. The alias-aware parser has negative controls for duplicate aliases, unknown alias references and out-of-range alias/reference indexes.

The CLI has three independent operations:
- `verify`: admit the local XML bytes against one pinned reference manifest;
- `bundle`: report conflicting RequiredModel versions across multiple NodeSet manifests;
- `server`: validate a strict read-only NamespaceArray/NamespaceMetadata snapshot and report whether the server declares a namespace subset.

For this exact Machinery/Robotics pair, the RequiredModel sets reveal a core model version skew: Machinery requires OPC UA core 1.05.07 (2026-07-30); Robotics requires core 1.05.03 (2023-12-15). Both require DI 1.04.0 and IA 1.01.4. The bundle analyzer reports `requires_explicit_dependency_resolution`; it does not silently select or upgrade a dependency.

Important date distinction: Machinery normative NamespaceMetadata publication date is 2026-11-01, but its pinned UANodeSet Model PublicationDate is 2026-11-02. Those fields are not collapsed into one date. Machinery ModelVersion is 1.4.2 even though its companion specification Version is 1.04.2.

The synthetic server snapshot declares only the two read services `Read NamespaceArray` and `Read NamespaceMetadata`, zero mutation service calls and `physical_qualification=false`. A matching `IsNamespaceSubset=true` is reported as a subset, not complete support. `IsNamespaceSubset=false` is only a full-namespace declaration; required NodeId coverage and semantic correctness still require an address-space comparison.

### Industry benchmark comparator: OPC Foundation UACTT Lite

The OPC Foundation announced UACTT Lite in June 2026 as a free information-model gap-analysis tool. It compares an expected companion-spec NodeSet against a connected server's address space, checking expected node presence, references, attributes, data types and type definitions, including mandatory and optional nodes. It does **not** test the full functional behavior of a companion specification; full OPC UA CTT covers broader services, security, core functionality, client and PubSub interfaces.

Foundry must not duplicate or claim to replace UACTT Lite. The direct comparison is:
- UACTT Lite: connected-server NodeSet/address-space gap analysis.
- Foundry M0/M0.1: source artifact provenance, offline NodeSet admission, RequiredModel dependency conflict reporting, portable NamespaceArray/NodeId identity and explicit subset-state reporting.
- Future Foundry endpoint tournament: run UACTT Lite on the same isolated read-only server snapshots where possible, then add cross-backend manufacturing identity, QIF join, calibration/uncertainty, time/units, evidence provenance and task-grounding metrics that API/model conformance alone does not establish.

Any live OPC UA endpoint experiment remains read-only and isolated. No methods, writes, PLC operations or product release calls are permitted.

## FoundryBench protocol

The fixed-client benchmark design is specified in [`benchmark-protocol.md`](benchmark-protocol.md). It fixes the evidence/client/task/hardware envelope before comparison, separates protocol, model, semantic and task/operational scoreboards, and defines the TEP complete-scenario and no-leakage rules. It identifies UACTT Lite/full OPC UA CTT as external comparators rather than claiming to replace them. This protocol is a proposal; no full cross-backend tournament has yet run.

## TEP SOTA extensions: feature identity stability and onset-safe decisions

The FoundryBench protocol has been strengthened using two 2026 TEP research directions:

1. Evolutionary WFIF optimizes discriminative feature power, dimensionality and feature-set stability. Its paper reports 95.79% average F1 across 21 faults, improvement on difficult d03/d09/d15, and low variability in selected subset size. FoundryBench deliberately goes further: it measures pairwise feature-identity overlap (Jaccard) and, for equal-cardinality sets, Kuncheva stability. It also checks whether selected variables remain stable under held-out run and operating-mode shifts. A stable but causally spurious proxy is not considered valid.
2. Five-Stage Cascade uses onset-based early diagnosis, exposing only the first post-fault observations at fixed horizons. Its paper reports reductions in confident wrong labels and false alarms against nine same-budget baselines, using safe deferral for uncertain labels. The dataset is available on request, so it is not yet a reproducible public baseline for Foundry.

The protocol now evaluates the earliest defensible decision rather than the earliest forced label. At fixed horizons, report detection, diagnosis, false alarms, confident-wrong diagnoses, safe deferrals, time-to-detection, time-to-correct-diagnosis, coverage and decision loss. Thresholds and feature selection must be trained/selected without the held-out test runs.

Sources:
- [Evolutionary WFIF feature selection for TEP (Scientific Reports, 2026)](https://www.nature.com/articles/s41598-026-49874-5)
- [Five-Stage Cascade early diagnosis (2026)](https://www.sciencedirect.com/science/article/pii/S0959152426002015)

These are comparator designs and proposed Foundry metrics, not reproduced results or claims of superiority.

## Executable FoundryBench metrics

The Rust validator exposes `bench_metrics`:
- `measure_feature_subset_stability` reports pairwise feature-set Jaccard across selector runs, mean Kuncheva for equal-cardinality pairs, mean selected-feature count and its population standard deviation. It rejects duplicate run IDs, duplicate feature IDs, unknown/non-canonical feature IDs and empty feature sets.
- `inspect_run_partition_leakage` verifies the train/validation/test simulation-run ID sets do not overlap. Apply it before generating sample windows so the same trajectory cannot leak across splits.

The adversarial unit test `equal_subset_size_can_hide_identity_instability` proves that zero selected-feature-count variance does not imply stable feature identities. These are metric primitives only; the current test fixtures are synthetic and no TEP data is included.

## Next OPC UA milestone

The follow-on workstream [MFG-ARCH-OPCUA-ADDRESS-SPACE-001 #6615](https://github.com/Luminous-Dynamics/symtropy/issues/6615) addresses a remaining boundary: exact NodeSet provenance and matching NamespaceMetadata do not establish that the actual server address space contains the expected Nodes, attributes and references. It will use OPC Foundation UACTT Lite/CTT as external comparators rather than duplicating them, then add canonical NodeId and manufacturing semantic checks under a read-only snapshot contract.

The same module exposes `evaluate_tep_onset_horizon`. It accepts one record per fault or normal run at a fixed post-onset horizon, requires all 21 standard fault scenarios plus normal-operation exposure, and validates detection/diagnosis consistency. It reports recall, confident wrong labels, safe deferrals, diagnosis coverage/selective accuracy, detection delay and false alarms per normal-operation hour. Diagnosis correctness is an independent input label; the evaluator does not infer it from model output. Current tests use synthetic known-answer records only.

### FoundryBench CLI

From `tools/manufacturing-architecture-portability/validator`, run the offline metrics over strict JSON manifests:

```sh
cargo run --locked --bin foundrybench-metrics -- feature-stability \
  ../benchmarks/feature-stability-input.schema.json \
  ../benchmarks/feature-stability.synthetic.json

cargo run --locked --bin foundrybench-metrics -- tep-onset \
  ../benchmarks/tep-onset-input.schema.json \
  ../benchmarks/tep-onset.synthetic.json

cargo run --locked --bin foundrybench-metrics -- run-partitions \
  ../benchmarks/run-partition-input.schema.json \
  ../benchmarks/run-partitions.synthetic.json
```

The feature-stability command reports Jaccard/Kuncheva overlap and subset-size variance; the supplied synthetic fixture intentionally has zero subset-size variance with non-identical feature subsets. The TEP onset command requires all 21 fault scenario IDs plus normal-operation exposure, and reports the separate detection/diagnosis/deferral metrics. The partition command returns non-zero when any complete run ID appears in more than one partition.

All three committed manifests are synthetic known-answer fixtures; they contain no real TEP measurements, fitted feature selector or model-performance result.

### Binary probability-calibration CLI

`foundrybench-metrics calibration` consumes an independent binary outcome/probability manifest and reports Brier score, clipped negative log-likelihood, fixed-width ECE/MCE and reliability-bin counts, mean probabilities and observed positive rates. Probabilities are validated in [0,1]; duplicate/non-canonical sample IDs and invalid bin counts fail closed.

The benchmark includes a synthetic known-answer manifest. The CI workflow runs this CLI and checks that the Brier, NLL and ECE outputs are present. It is not the 2026 CRNN result and does not imply any trained classifier has been evaluated.

## Physics-guided residual reference-model stress

A 2026 TEP binary-detection baseline uses residuals from a nominal simulator, a temporal attention CRNN and post-hoc probability calibration. It reports run-level 70/20/10 splitting, macro-F1 about 0.93±0.02 and ECE about 0.03, with a nuisance-alarm budget around 0.1/hour. The paper's "twin" is the nominal simulator, not a plant-specific physical twin; its task is binary normal/fault detection, not multiclass fault diagnosis.

The protocol now includes a reference-model mismatch sweep `lambda ∈ {0, 0.05, 0.10, 0.20}` with detector/calibrator/threshold fixed, plus a separate recalibration arm. Compare raw-data, physics-guided residual and hybrid methods on identical run-level partitions. Report recall, false alarms/hour, Brier, NLL, ECE, detection delay, residual drift and calibration drift at each mismatch. Sensor noise/dropout and operating-mode shifts remain separate controlled factors.

Source: [Scientific Reports 2026 physics-guided residual/CRNN TEP study](https://www.nature.com/articles/s41598-026-49874-5). The paper's result is a comparator, not reproduced by Foundry.
