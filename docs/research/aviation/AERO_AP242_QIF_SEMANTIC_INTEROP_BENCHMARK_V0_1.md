# AERO AP242 / QIF Semantic Interoperability Benchmark v0.1

**Tracker:** [AERO-DIGITAL-THREAD-INTEROP-001 #7395](https://github.com/Luminous-Dynamics/symtropy/issues/7395)  
**Status:** synthetic-first benchmark contract; no normative AP242/QIF parser or conformance result  
**Owners:** Symtropy benchmark orchestration and semantic adapters; Mycelix physical-unit/build lineage and artifact integrity; Symthaea BASALT/ASSURE evidence freshness and lifecycle  
**Claim ceiling:** preservation of a declared semantic projection over a bounded synthetic corpus only

## 1. Purpose

A digital thread is useful only when engineering meaning survives the boundaries between product definition, manufacturing, inspection, metrology and qualification systems.

A valid file, parseable XML document, matching SHA-256 digest or resolvable GUID does not prove semantic equivalence. A QIF measurement can be syntactically valid but attached to the wrong physical unit, product revision, datum, characteristic, unit or calibration event. An AP242 model can carry the expected geometry while identifying a different released product configuration. A round trip can preserve bytes and silently change engineering meaning.

This benchmark tests **semantic preservation**, not merely syntax, file existence or artifact identity.

## 2. Architecture boundary

The benchmark uses an internal, versioned semantic projection as a neutral comparison surface:

AP242 source → AP242 adapter → semantic projection ← QIF adapter ← QIF source

The projection is not an alternative AP242 or QIF schema. It is not a replacement PLM/QMS/MES, materials database, metrology system, artifact store, evidence graph, lifecycle or certification authority. It contains only the identity and meaning needed by the bounded benchmark.

Each adapter must retain:
- source artifact reference, source digest, byte length and media type;
- declared normative standard edition and application/profile identifier;
- parser/adapter implementation identity and version;
- source-native identifiers and canonical cross-system references;
- explicit unsupported, unknown, unresolved and lossy transformations.

Do not claim AP242/QIF conformance from a successful projection parse. Normative conformance requires the exact standard edition, schema/application profile, licensing/rights basis, required test suite and independent review.

## 3. Semantic identity model

The benchmark's canonical join chain is:

product definition revision → product configuration → manufactured physical unit → material/process lineage → specimen sampling event → specimen item → inspection/measurement result

These identities are distinct. In particular:

- a product definition is not a serialized physical unit;
- a sampling event may produce multiple specimen items;
- a traveler or work order may cover multiple units/specimens;
- a calibration certificate artifact is not the same identity as the calibration evidence record;
- a result artifact digest is not a physical-unit identity;
- a QIF characteristic/result ID is not a product characteristic identity unless an explicit link says so.

The projection must preserve exact immutable identifiers and cross-file edges. Human-readable names and labels are display metadata, never join keys.

## 4. Required projection semantics

### 4.1 Product definition and configuration

- selected AP242 edition/application profile;
- product-definition identity and exact revision;
- geometry/model artifact identity and digest;
- product-configuration identity and exact effectivity/release state;
- coordinate-system/reference-datum identities;
- units and transformations applied to geometry/PMI values;
- characteristic/PMI/tolerance identities and their explicit attachment targets.

Where an input profile does not support a field, mark it NOT_SUPPORTED or NOT_COVERED; do not synthesize it.

### 4.2 Physical unit and process lineage

- serialized physical-unit ID;
- immutable product/configuration revision binding;
- material system/grade/revision and supplier/source identity;
- material lot, process recipe revision, process batch, machine/tool/site and panel/detail lineage;
- specimen ID, parent sampling-event ID and unique specimen-item ID;
- source-system, source-record, issuer/witness and provenance references;
- identity disposition (verified, unresolved, conflict) with its evidence basis.

Use Mycelix's existing manufactured-unit and packet lineage. Do not invent a second unit registry.

### 4.3 Inspection and metrology result

- QIF edition/profile;
- inspection plan/procedure/method revision;
- exact equipment/instrument identity;
- calibration evidence-record identity and separate certificate artifact identity;
- valid_from_unix_ms, valid_until_unix_ms, and measurement used_at_unix_ms / acquisition time;
- half-open calibration interval semantics [valid_from, valid_until);
- characteristic/result identity and exact attachment to unit/specimen/configuration;
- result value, unit, limits, uncertainty, resolution and method-specific metadata;
- status: observed, censored, excluded, missing, failed or not collected, with reason/limit retained;
- raw observation, derived/corrected observation and processing lineage kept distinct.

The projection must preserve enough information to determine which physical observation, instrument, method, calibration evidence and product configuration a result belongs to. It must not infer measurement truth from schema validity, signatures or file hashes.

### 4.4 Uncertainty and sample design

- measurand and unit;
- uncertainty type and value (standard, expanded, interval or other declared representation);
- coverage factor/confidence level where actually present;
- estimator/processing method and version;
- experimental unit, specimen-item identity and clustering hierarchy;
- sampling and dependence design where represented by the source;
- missingness, censoring, exclusion and invalid-result disposition.

Never silently turn unknown uncertainty into zero. Never count several cuts from one sampling event as independent experiments without an explicit, justified statistical design.

## 5. Round-trip and equivalence rules

For each test packet:

1. Parse the source artifact with adapter A and produce the semantic projection.
2. Export the supported projection subset through an independent adapter/exporter B.
3. Parse the exported artifact with an independently implemented reader.
4. Compare semantic values and typed relationships, not XML byte equality.
5. Recompute artifact digests and lengths independently.
6. Report each unsupported or lossy field and affected downstream claim.
7. Preserve the source and derived artifact identities; never overwrite the source.

Two artifacts are semantically equivalent only over the explicitly declared comparison scope. Equivalent geometry does not imply equivalent configuration, effectivity, PMI, material, unit, inspection or calibration semantics.

Harmless XML element ordering, whitespace, namespace-prefix choices, serialization order and stable-ID ordering may be metamorphic transformations only where the selected standard/profile defines them as semantically irrelevant. Never normalize away ordering that has engineering meaning.

## 6. Frozen benchmark arms

Use the same source documents, packet, projection profile, challenge cases, compute budget and reviewer task for every arm:

1. **Manual reconstruction:** qualified reviewer reconstructs required joins and meanings from source documents.
2. **Byte-only verification:** content digest/byte-length equality and reference existence, without semantic parsing.
3. **Syntax/profile validation:** parser and exact declared profile/schema validation only.
4. **Single semantic adapter:** one implementation produces and validates the projection.
5. **Independent dual-adapter round trip:** two separately implemented adapters/exporters and independent semantic comparison.
6. **Semantic change-impact composition:** dual-adapter projection plus Mycelix unit lineage and BASALT/ASSURE owner-native freshness/lifecycle receipts.

This benchmark evaluates information preservation and engineering-thread joins. It does not evaluate the accuracy of a CAD kernel, NDE algorithm, materials model or structural solver.

## 7. Adversarial and metamorphic challenge design

### Wrong or missing identity

- correct display labels but wrong product-definition revision;
- correct geometry artifact but wrong effectivity/configuration ID;
- correct unit ID linked to a different build packet;
- QIF result linked to the wrong specimen, specimen item or serialized unit;
- duplicated, dangling, missing, aliased or cross-unit GUID/ID;
- calibration evidence ID confused with certificate file identity;
- reused sampling-event ID with duplicate specimen-item ID;
- legitimate multiple specimen items sharing one sampling event and parent traveler.

### Measurement and transformation errors

- millimetres interpreted as inches or vice versa;
- coordinate frame, datum, sign or axis transformation omitted;
- tolerance/PMI attached to a different feature or datum;
- characteristic identity preserved but measured value attached to wrong characteristic;
- uncertainty, coverage factor, resolution or sampling unit silently dropped;
- observed/censored/excluded/missing/failed result omitted or rewritten as zero;
- stale, wrong-instrument, wrong-method or out-of-window calibration;
- calibration revocation/correction after acquisition;
- derived/corrected signal substituted for the original raw observation.

### Version and serialization changes

- AP242 or QIF edition/profile downgrade;
- namespace or identifier rewriting that breaks joins;
- unknown required semantic feature silently ignored;
- non-semantic ordering/whitespace transformations that should preserve the projection;
- serializer/exporter round trip that preserves bytes but changes semantic relationships;
- semantically equivalent source artifacts with different bytes;
- same digest with incompatible declared media type/profile or wrong semantic association.

### Expected disposition

Each challenge states an exact expected result per required field/relationship:
- PRESERVED;
- TRANSFORMED_EQUIVALENT (with declared conversion and tolerance);
- NOT_SUPPORTED;
- UNKNOWN;
- NOT_COVERED;
- CONFLICT;
- LOSSY_BLOCKED.

A whole packet cannot be reported equivalent if a mandatory identity, unit, uncertainty, calibration or physical-unit join is unresolved.

## 8. Metrics and prospective targets

Report a vector; never collapse semantic correctness, safety and cost into one score.

| Dimension | Measurement | Prospective software target |
|---|---|---|
| Identity preservation | Exact required ID and edge preservation | 100% mandatory identity/edge preservation or explicit abstention |
| False joins | Wrong product/unit/specimen/configuration/result association accepted | Zero on bounded challenge corpus |
| Measurement integrity | Value, unit, characteristic, status and reason preservation | Zero silent result/status loss |
| Uncertainty integrity | Uncertainty, estimator and experimental-unit preservation | 100% of declared required uncertainty semantics preserved or NOT_COVERED |
| Calibration integrity | Exact evidence/instrument/method and acquisition interval | Zero false accepts on boundary, mismatch and revocation controls |
| Geometry/PMI | Coordinate/datum/PMI relationship and conversion residual | Within the pinned profile's declared numerical tolerance; never invent a universal tolerance |
| Round-trip | Semantic projection difference before/after round trip | Zero unexplained semantic drift on supported fields |
| Metamorphic behavior | Projection equality under semantically harmless serialization changes | 100% same projection for declared harmless transformations |
| Abstention | Unsupported/ambiguous/unknown fields detected | 100% explicit UNKNOWN/NOT_SUPPORTED/NOT_COVERED where applicable |
| Human effort | Reviewer reconstruction time and error rate | Report against manual baseline; do not predeclare an unsubstantiated reduction |
| Performance | Parse/validation runtime, memory and artifact size | Report p50/p95 on frozen hardware and corpus |

Before any external claim:
- independently review a rights-cleared golden corpus;
- use at least two independent parser/exporter implementations;
- pin the standard edition, schema/application profile, tool versions and source licenses;
- report false joins and missed joins separately with denominators and confidence intervals;
- retain all unsupported/lossy-field records.

These targets are research objectives, not ISO conformance criteria, regulatory criteria or achieved results.

## 9. Connection to existing owners

- **Mycelix:** serialized physical unit, product/build/inspection/test packet lineage and exact artifact byte integrity.
- **Symtropy:** aviation semantics, AP242/QIF adapter, configuration and process joins, benchmark corpus and impact explanations.
- **Symthaea BASALT:** resource digest/generation freshness and generic direct/transitive invalidation.
- **Symthaea ASSURE:** append-only qualification lifecycle and supplied-view currentness.
- **Symthaea CEB:** typed engineering-claim support closure and counter-evidence.
- **Symthaea uncertainty/VOI owners:** observation-level uncertainty/sampling provenance and decision-value experiment selection.

The benchmark does not copy any of these owners' generic state machines or ledgers.

## 10. Non-claims

This v0.1 benchmark specification does not implement AP242 or QIF parsers, prove standards conformance, authenticate publishers, prove physical identity, establish measurement truth, qualify a material/process/part, prove NDE probability of detection, substantiate aircraft structure, approve a configuration, authorize manufacturing/installation/operation, establish airworthiness or certification.
