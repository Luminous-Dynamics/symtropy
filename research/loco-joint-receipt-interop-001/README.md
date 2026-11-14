# LOCO-JOINT-RECEIPT-INTEROP-001 — normalized single-joint evidence packet

Status: **v0.1 contract proposal / synthetic golden packet**. This artifact specifies an offline interchange boundary between the Symtropy single-joint metrology normalizer and Symthaea's causal mission replay join. It is not a runtime protocol, controller, safety authority, physical qualification, or claim of hardware performance.

Parent: [Symtropy #4889](https://github.com/Luminous-Dynamics/symtropy/issues/4889). Producer reference: [LOCO-JOINT-METROLOGY-001 #4783](https://github.com/Luminous-Dynamics/symtropy/issues/4783), PR [#4786](https://github.com/Luminous-Dynamics/symtropy/pull/4786). Consumer reference: Symthaea [ROB-CAUSAL-MISSION-REPLAY-001 PR #11664](https://github.com/Luminous-Dynamics/symthaea/pull/11664).

## 1. Ownership and non-goals

- **Symtropy producer** owns normalization of instrument, timestamp, sampling, thermal, energy-boundary and command-response receipts. It does not establish truth of sensors or physical models merely by serializing a receipt.
- **Symthaea consumer** joins the packet to its authoritative `MissionTrace`, validates exact event/configuration/calibration/prefix/state bindings, and runs retrospective causal replay.
- Existing actuator, clock, thermal, energy, viability, projection, RTA and safe-stop authorities remain authoritative. No owner is duplicated here.
- This packet never authorizes actuation. `actuation_authorized` is a required literal `false`.
- Raw DAQ traces, calibration certificates and instrument-native files remain external artifacts, content-addressed by digests and retained under their own access/retention controls.

## 2. Version 0.1 packet

The strict schema is `fixtures/joint_receipt_v0_1.schema.json`. The synthetic complete reference is `fixtures/joint_receipt_v0_1_complete.json`.

The envelope binds:

1. protocol/schema identity and producer/consumer reference revisions;
2. mission, robot, boot epoch, clock, actuator and source trace identities;
3. the exact observation → decision → proposal → assurance → command → actuator application → physical transition → terminal event chain;
4. policy/model/action-domain/configuration and sensor/torque/power/thermal/sampling/energy-calibration digests;
5. source/application/motion timestamp intervals and an explicit same-clock, bounded-mapping or Unknown relation;
6. decision regret, epsilon, calibration receipt and valid-through/application ticks;
7. per-node thermal coverage and predecessor/result digests;
8. the upstream first-law disposition (Balanced, Unknown, Inconclusive or Invalid), never a recomputed value;
9. sampling qualification, AoI and command-to-motion as separate intervals;
10. reason-coded evidence dispositions, terminal closure and an explicit no-actuation-authority field;
11. an optional future-options evidence slot which is explicitly `NotEvaluated` until the LOCO-FUTURE-OPTIONS owner supplies a qualified receipt.

### Integer and time representation

Every `u64`-width value (nanoseconds, event sequence, boot epoch, ticks, fixed-point task-loss units) is encoded as a base-10 JSON string matching `^(0|[1-9][0-9]*)$`. Signed clock offsets use `^(0|-?[1-9][0-9]*)$`. This avoids loss of precision in JSON implementations whose number type is IEEE-754 binary64. Rates and bounded cardinalities that are known to fit the JSON safe-integer range are JSON integers. This v0.1 profile contains no floating-point values. Both producer and consumer must perform checked decimal-to-integer conversion and reject values outside the declared Rust width; the schema regex checks lexical form, not numeric range.

Timestamp intervals are closed bounds in one named monotonic clock and boot epoch. A cross-clock relation is usable only with a qualified mapping whose source/target clock, boot epoch, bounded offset and digest are all present. `Unknown` is represented explicitly; it is never converted to a numeric age.

### Evidence-state semantics

- `Qualified`: the named evidence owner supplied a qualified result for the exact stated scope.
- `Unknown`: required evidence or a valid clock/state relation is unavailable, stale, incomplete or not established.
- `Inconclusive`: the declared measurement boundary or upstream analysis cannot resolve the claim.
- `Rejected`: a supplied fact contradicts an invariant or a hard declared constraint.
- `NotEvaluated`: an optional subsystem has not yet been evaluated; it is not a successful empty result.

These states are not interchangeable. The consumer may further downgrade a producer result after joining it to the mission trace; it must not upgrade Unknown, Inconclusive or Rejected based only on the producer's label.

## 3. Canonical integrity digest

`integrity.payload_sha256` is SHA-256 of the canonical UTF-8 JSON serialization of the complete top-level object **after removing the entire `integrity` member**. Use RFC 8785 JSON Canonicalization Scheme (JCS) property ordering and string escaping. Because all potentially wide integers are decimal strings and v0.1 forbids floating-point values, no implementation may parse a wide integer through a binary floating-point number before canonicalization. A producer must reject duplicate JSON object keys before canonicalization.

The digest is lowercase hexadecimal. The golden packet's digest is a known-answer value. The integrity block is excluded from its own preimage; no other field is excluded. The schema does not prove the digest is authentic: transport signatures, trust roots and provenance policy remain separate concerns.

## 4. Consumer binding rules

The consumer loads the `MissionTrace` independently and requires exact equality for the shared identities, including mission/robot/boot/scenario, source-observation, decision, proposal, assurance, command, application, physical-transition and terminal IDs/sequences. It also requires exact equality of actuator/controller/policy/model/action-domain and calibration/configuration digests, expected/observed executed-prefix digests, thermal snapshot/predecessor/result, actuator application receipt and terminal reserve identity.

A mismatch is reason-coded `Rejected`; absent required evidence is `Unknown`; incomplete energy boundary is `Inconclusive` or `Unknown` according to the upstream owner's disposition. A missing option receipt is `NotEvaluated`, not zero options. Cross-clock subtraction without a qualified mapping is forbidden. Source-to-actuator AoI and command-to-motion response are separate measurements.

## 5. Golden packet and required mutations

The complete golden packet is synthetic and binds a single application/physical transition. It provides a positive structural case only. The producer/consumer implementation must add mutation tests for:

- changed event ID or sequence;
- changed calibration, configuration, application or scenario digest;
- changed timestamp interval, clock ID or boot epoch;
- changed expected/observed executed prefix;
- changed thermal predecessor/result or hidden reserve reset;
- incomplete/invalid energy boundary;
- incomplete thermal node manifest;
- decision expired or regret above epsilon;
- terminal safe-stop / reserve digest mismatch;
- unsupported schema or producer version;
- tampered payload digest and duplicate JSON keys.

Each mutation must fail closed with a stable reason code. Keep Unknown, Inconclusive and Rejected cases distinct. The current golden packet does not claim these mutations are already implemented in either adapter.

## 6. Future-option evidence extension

The `future_options` object is deliberately `NotEvaluated` in the v0.1 golden packet. When populated from Symtropy [LOCO-FUTURE-OPTIONS-001 #4588](https://github.com/Luminous-Dynamics/symtropy/issues/4588), it must bind the complete physical-branch manifest, delivered-observation partition, continuation-action domain, common-continuation sets per observation class, separate evaluator-only clairvoyant upper bound, recovery-topology receipt, physical-state predecessor/result, validity horizon and reason-coded disposition.

The operational continuation set is the intersection of admitted continuations across all physically possible branches that are indistinguishable under observations actually delivered to the robot. A union or per-hidden-state choice is evaluator-only clairvoyance, not deployable recoverability. Do not collapse option counts, recovery paths, thermal headroom, torque authority, energy, deadlines and compute into one universal score.

## 7. Qualification ladder

1. Schema validation and canonical-digest known-answer agreement.
2. Producer serialization round-trip preserving all identities and dispositions.
3. Consumer join against the exact matching `MissionTrace`.
4. Adversarial mutations fail closed; no status upgrades.
5. Exact-head Rust format, locked tests and strict Clippy on both repositories.
6. One newly acquired calibrated single-joint trace, with raw measurements and calibration artifacts retained and digest-bound.
7. Sequential disturbance experiments without resetting temperature, wear, actuator authority, battery or recovery state.

Steps 1–5 qualify only the software interchange and retrospective structural replay. Steps 6–7 are required before making any physical claim. A passing packet does not establish sensor truth, calibration validity, physical model accuracy, first-law closure, safety or robot certification.

## 8. Research references

- [Symtropy single-joint protocol #4783](https://github.com/Luminous-Dynamics/symtropy/issues/4783)
- [Symthaea causal mission replay #11664](https://github.com/Luminous-Dynamics/symthaea/pull/11664)
- [Observation-constrained future mobility #4588](https://github.com/Luminous-Dynamics/symtropy/issues/4588)
- [RFC 8785 — JSON Canonicalization Scheme](https://www.rfc-editor.org/rfc/rfc8785)

All v0.1 measurements and identities in the companion packet are synthetic known answers, not measurements from a physical robot.

### Machine-readable mutation oracles

`fixtures/mutations_v0_1.json` contains 13 synthetic mutation cases and `fixtures/mutations_v0_1.schema.json` is its strict Draft 2020-12 schema. They cover identity and calibration drift, timestamp inversion, executed-prefix mismatch, thermal-history mismatch, incomplete energy, missing thermal coverage, decision expiry, terminal reserve mismatch, unsupported version, payload tampering, unknown clock relation and duplicate JSON object keys. The expected reason codes are proposed stable contract codes; they are **not yet emitted by the Rust producer/consumer**. The mutation corpus is authored test data, not executed Rust evidence.


## 6. Executable Rust wire-integrity reference

The isolated dependency-pinned Rust crate is at \`reference/\`. It adds the missing executable integrity layer alongside the existing Draft 2020-12 schema and semantic preflight:

- strict recursive JSON parsing rejects duplicate object keys before canonicalization;
- v0.1 numeric profile rejects all floating-point numbers and numeric integers outside the IEEE-754 safe-integer range; wide values stay decimal strings;
- JCS object-member ordering uses UTF-16 code units, not Rust UTF-8 byte ordering;
- string escaping, nested arrays/objects and compact UTF-8 serialization are deterministic;
- SHA-256 is recomputed over the entire top-level packet with only the complete \`integrity\` member removed;
- protocol ID, schema version, canonicalization profile, digest scope and \`actuation_authorized: false\` are checked.

This Rust reference does **not** duplicate the full JSON Schema validator or the producer's physical normalization. The existing Python 3.12 workflow continues to perform the closed Draft 2020-12 schema and golden-packet semantic checks; the Rust job independently exercises the JCS/SHA-256 and parser contract.

Run:

    cargo fmt --manifest-path research/loco-joint-receipt-interop-001/reference/Cargo.toml -- --check
    cargo test --locked --manifest-path research/loco-joint-receipt-interop-001/reference/Cargo.toml
    cargo clippy --locked --manifest-path research/loco-joint-receipt-interop-001/reference/Cargo.toml --all-targets -- -D warnings

The parent golden fixture must produce SHA-256 \`cb19e0327c2f47c895eae4e49a89c9f8f9c9918e080d52b201d98b1a794823b8\`. The test corpus also verifies duplicate-key rejection, float / unsafe-integer rejection, UTF-16 property ordering, digest tampering, integrity-member exclusion and no-actuation-authority behavior.

This remains synthetic wire-integrity qualification; it does not serialize the live \`AdapterReport\`, consume the receipt in Symthaea, authenticate the producer, validate measurements, or establish a physical joint result.


## 7. Versioned measurement-uncertainty sidecar (v0.1)

The original joint receipt packet remains **unchanged**. A separate sidecar is introduced at:

- \`fixtures/joint_uq_sidecar_v0_1.schema.json\`
- \`fixtures/joint_uq_sidecar_v0_1_complete.json\`
- \`fixtures/joint_uq_mutations_v0_1.json\`

Protocol: \`LOCO-JOINT-UQ-SIDECAR-001\`, schema version \`0.1.0\`.

The sidecar binds itself to the exact parent joint receipt payload SHA-256. It binds common mission, robot, actuator, controller, scenario, clock/boot epoch, physical boundary and physical-state predecessor/result identities. Every per-channel uncertainty receipt binds the common scope digest and the exact measurement artifact, instrument configuration, calibration, units and scale.

The synthetic packet contains:
- shaft torque and bus-voltage measurements with separate uncertainty receipts;
- a shared calibration-source artifact represented by the same dependence group and the same joint-model receipt across both channels;
- channel-local repeatability components with separately declared independence evidence;
- a derived electrical-energy result with the exact torque/voltage input measurement IDs and input uncertainty receipt IDs;
- an explicit Monte Carlo coverage receipt and convergence receipt for the derived energy result;
- literal \`actuation_authorized: false\` and \`conformity_certified: false\`.

The Rust reference exposes `validate_uq_sidecar(sidecar, parent)`. Alongside duplicate-key-free parsing and JCS/SHA-256, it checks exact parent packet and physical-scope bindings, predeclared measurement-channel completeness, calibration and UQ validity over both the full measurement window and audit tick, per-measurement uncertainty identity, source-component/dependence-group coverage, consistent cross-channel shared-source group and joint-model receipts, derived input/output uncertainty-receipt lineage, coverage unit/scale and interval ordering, and upstream disposition preservation. A Monte Carlo interval without a convergence receipt downgrades to Inconclusive; upstream Unknown/Inconclusive/Rejected is never upgraded.

The complete uncertainty-receipt policy remains owned by [LOCO-MEASUREMENT-UQ-RECEIPT-001 #5062 / PR #5067](https://github.com/Luminous-Dynamics/symtropy/pull/5067). That independently tested Rust auditor remains the authoritative typed-profile completeness implementation; this wire adapter checks that the serialized sidecar remains bound to the exact causal packet and that no cross-packet identity/dependence contradictions are introduced. The Python Draft 2020-12 preflight independently checks the closed sidecar schema and golden fixture semantics.

The Rust reference currently has 30 authored deterministic tests; they are not passing-test claims until the exact-head Rust 1.96 workflow passes. The 10-case mutation inventory includes parent digest drift, actuator scope drift, source dependence group split, shared-model receipt mismatch, derived input-UQ omission, calibration expiry, missing Monte Carlo convergence, actuation authority escalation, duplicate JSON keys, and payload digest tampering.

### Claim boundary

This is synthetic sidecar interoperability and evidence-lineage qualification. It does not authenticate instruments, calculate covariance or nonlinear uncertainty, validate calibration truth, prove the physical state, or demonstrate a hardware result. The UQ auditor (#5062 / PR #5067) remains the owner of receipt-completeness semantics; this sidecar transports those receipts and binds them to the parent causal trace.


### Predeclared channel completeness

The UQ sidecar carries the full predeclared `measurement_requirements` set and the explicit `evaluation_tick`, not only the channels that happened to arrive. The synthetic fixture requires shaft torque, bus voltage, both parent-required thermal nodes (motor and drive), and derived electrical energy. A separate gearbox-temperature channel is explicitly NotApplicable with a reason. The parent and sidecar required thermal-node sets must match exactly. The consumer rejects a packet that omits a Required measurement, supplies a NotApplicable channel, or changes the measurand/node identity. Calibration and UQ validity must cover both the entire acquired window and the evaluation tick; a stale evaluation tick returns an incomplete-evidence result rather than being upgraded.
