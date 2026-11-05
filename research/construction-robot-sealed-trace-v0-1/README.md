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

A production verifier must:
1. validate the closed schema;
2. strictly decode base64 and recompute payload SHA-256;
3. enforce capture sequence and connection/session rules;
4. recompute event digests and the root digest using RFC 8785;
5. compare the root with an independently stored/signature-bound anchor when one exists;
6. leave completeness unknown unless separately supported by recorder and broker boundary evidence;
7. return typed outcomes (valid_integrity_with_unknown_completeness, valid_integrity_with_known_incompleteness, anchored_integrity_valid, rejected) rather than a single misleading PASS.

The GitHub workflow currently validates both schemas, the fixture's exact payload/event/root digest chain, and 17 executable positive/adversarial mutations with a small pinned CI-only Python check. One negative control deliberately recomputes every digest after changing a payload: the internally consistent rewrite remains valid but is explicitly unanchored and unauthenticated. This is not production code; a dependency-policy-compliant Rust verifier and independent implementation comparison are follow-on work under #3707.

## Non-duplication

- #3644: canonical construction task contract.
- #3704: VDA 5050 semantic crosswalk.
- #3706: independent multi-implementation plugfest design.
- #3707: sealed trace format and verifier.

## Claim ceiling

The included fixture is synthetic. It demonstrates a reproducible data format and digest construction, not an actual broker capture, protocol conformance, authenticated sender, complete packet capture, synchronized clocks, construction work performance, safety, or acceptance.
