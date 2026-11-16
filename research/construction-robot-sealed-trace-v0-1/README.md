# Construction Robot Sealed Trace Envelope v0.1

**Status:** synthetic trace format, closed schema and mutation corpus. No production MQTT recorder or cryptographic-signature service is implemented.

## Purpose

Independent adapter comparison is meaningless unless each observer receives the exact same evidence. This profile preserves the MQTT application payload bytes and the metadata needed to interpret delivery and timing without normalizing away differences.

The trace is a passive evidence container. It is not a task contract, fleet manager, broker, safety system, robot-authentication mechanism, or construction acceptance record.

## Event structure

Each captured event preserves:
- monotonically assigned recorder capture sequence (not source event order);
- exact topic, direction, QoS, RETAIN and DUP flags, connection epoch and optional packet identifier;
- participant role and identity claim, explicitly not authentication;
- distinct timestamp observations, each with its own source, clock ID and uncertainty;
- exact MQTT application payload bytes as base64 plus SHA-256 over the decoded bytes;
- previous event digest and event digest.

The original payload is never parsed and reserialized as its evidence representation. Parsing is a separate derived annotation bound to the exact event digest, parser name/version and rule-set digest.

## Digest construction

Canonicalization is RFC 8785 JSON Canonicalization Scheme (JCS). For v0.1, values in the digest profile use JSON integers (not floating point numbers), ASCII object keys, UTF-8 strings and no NaN/Infinity.

Event digest:

```text
SHA256(
  UTF8("construction-robot-trace-event-v0.1") || 0x00 ||
  UTF8(JCS(event_object_without_event_digest))
)
```

The event object includes previous_event_digest and payload SHA-256, as well as all transport metadata and timestamps. The first event has previous_event_digest = null; later events point to the immediately preceding event digest in recorder capture order. Each derived parser annotation separately uses `interpretation_digest = SHA256(UTF8("construction-robot-trace-annotation-v0.1") || 0x00 || UTF8(JCS(interpretation)))` and must reference an event digest that exists in the sealed trace.

Root digest:

```text
SHA256(
  UTF8("construction-robot-trace-root-v0.1") || 0x00 ||
  UTF8(JCS({
    "manifest": manifest,
    "ordered_event_digests": [event_digest, ...],
    "annotations": annotations
  }))
)
```

The manifest includes capture completeness, loss counters, filter/redaction state, first/last sequence and count. The final_trace_digest field itself is excluded from the root input.

A hash chain is not a digital signature and does not stop an editor from modifying a trace and recomputing all hashes. It provides a deterministic integrity check relative to a separately trusted digest anchor. `complete_with_boundary_evidence` requires an explicit non-empty `capture_boundary_evidence_ref`; setting the status is not itself boundary evidence. A redacted derivative requires both its `parent_trace_digest` and `transformation_digest`, and a filtered trace requires a `filter_profile_digest. Without a detached signature or external receipt, a self-contained file is internally checkable but not externally authenticated. Even with an external anchor, integrity does not prove that the recorder captured every broker message, that the broker timestamp is true, that clocks are synchronized, or that the participant identity claim is authentic.

## Time semantics

- recorder_received: the recorder's own clock at receipt;
- broker_ingress / broker_egress: broker-originated timestamp evidence, if actually made available by the broker;
- robot_reported: a timestamp carried inside a robot message.

Clock ID and uncertainty are mandatory for every timestamp. Capture sequence proves only recorder order. Two events from different clocks cannot be strongly ordered unless the uncertainty intervals support that conclusion.

A recorder reporting zero local drops does not prove that the broker or network lost nothing. v0.1 therefore allows completeness to remain unknown, which is the correct default where broker-side boundary evidence is absent.

## Redaction and annotations

Redaction produces a derivative trace. The derivative must identify the parent trace digest and transformation digest; it never overwrites the original. A transformed trace can be useful for sharing, but must not be confused with the unredacted evidence.

Protocol parsing and interpretation are detached annotations. They reference event digests and identify parser and rule-set versions. Re-running a different adapter does not mutate the source event or its original payload bytes.

## Frozen corpus

The 3-event example is synthetic and deliberately ends in a robot-reported action FINISHED observation. Its integrity can validate while capture completeness remains unknown and its semantics remain executor progress only.

The 16 mutation cases cover byte changes, digest changes, event reordering/removal/insertion, topic/QoS/retain changes, timestamp provenance, uncertainty understatement, connection-epoch changes, broken chain, falsely upgraded capture completeness, redaction without parent, annotation rebinding and valid unknown completeness.

## Verification boundaries

The standalone Rust verifier is in `verifier/`. It uses a pinned Cargo.lock and exposes both a library API and an offline CLI:

```sh
cargo test --manifest-path research/construction-robot-sealed-trace-v0-1/verifier/Cargo.toml --locked
cargo run --manifest-path research/construction-robot-sealed-trace-v0-1/verifier/Cargo.toml --locked -- verify research/construction-robot-sealed-trace-v0-1/fixtures/trace-synthetic-vda-001.json
cargo run --manifest-path research/construction-robot-sealed-trace-v0-1/verifier/Cargo.toml --locked -- verify TRACE.json --expected-root TRUSTED_ROOT_SHA256
```

The expected root must be obtained independently (for example from a detached signature or separate receipt channel); copying the root from the same trace is not an anchor. The verifier rejects duplicate JSON keys before conversion to a JSON value, checks exact object keys, decodes base64 strictly, recomputes payload SHA-256, verifies sequence/session metadata and previous-event links, checks annotation references, recomputes event and root digests, and distinguishes integrity from completeness and external anchoring.

The v0.1 canonicalization profile uses RFC 8785 JSON Canonicalization Scheme rules over an explicitly restricted I-JSON subset: object keys are ordered by UTF-16 code units, strings are serialized without normalization, and only safe integers (absolute value no greater than 2^53-1) are accepted. Floating-point values and larger integers are rejected, not silently rounded. This restriction is reflected in the schema/README and should be versioned rather than relaxed implicitly.

The Rust CLI has 11 focused unit tests, including duplicate-key rejection, digest tampering, externally supplied anchors, Unicode key ordering, unsafe-number rejection, evidence-dependent completeness/redaction checks and the fully recomputed-but-unanchored rewrite negative control. The CI-only Python harness continues to test the frozen 17-case mutation corpus independently. The focused workflow now runs locked Rust metadata, formatting, tests, strict Clippy and fixture verification on Ubuntu, Windows and macOS, in addition to JSON Schema and mutation checks. These remain synthetic/software tests; the independent second implementation and real broker capture are separate plugfest gates.

## Non-duplication

- #3644: canonical construction task contract.
- #3704: VDA 5050 semantic crosswalk.
- #3706: independent multi-implementation plugfest design.
- #3707: sealed trace format and verifier.

## Claim ceiling

The included fixture is synthetic. It demonstrates a reproducible data format and digest construction, not an actual broker capture, protocol conformance, authenticated sender, complete packet capture, synchronized clocks, construction work performance, safety, or acceptance.
