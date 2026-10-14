# PHYS-EVID-02A CANONICAL PHYSICS EVIDENCE FRAME v0.1

Status: proposed representation theorem; exact-head executable qualification required before PASS is claimed.

Issue: PHYS-EVID-02A / #1101. Parent architecture: PHYS-EVID-02 / #1064.

## Purpose

Freeze one small, language-neutral, bounded binary frame for future persistent physics evidence payloads without pretending that arbitrary framed bytes are themselves qualified semantic evidence.

This theorem establishes only:

```text
typed framing kind
+ bounded payload bytes
        ↓
exact v1 frame bytes
```

and the canonical inverse framing parser.

It does not define the future session-binding/endpoint/consecutive/dwell/arrival semantic payload schemas.

## Why framing is split from semantic payload codecs

Cross-process semantic evidence requires PHYS-EVID-05 / #1074 evidence-session qualification. Encoding durable endpoint/consecutive/etc claims before that session identity exists would omit a required lineage axis or force a later incompatible reinterpretation.

The byte framing primitive is independently stable now, so v0.1 freezes it separately.

## Exact application namespace

The exact v1 magic bytes are ASCII:

```text
symtropy-physics-evidence\0
```

Hex:

```text
73796d74726f70792d706879736963732d65766964656e636500
```

The terminal NUL is part of the magic.

This is intentionally distinct from the Nix-oriented commitment namespaces previously audited in `Luminous-Dynamics/luminous-dynamics`. We reuse the framing pattern, not another application's magic or domains.

## Exact frame

The v1 bytes are:

```text
MAGIC
|| KIND_DOMAIN
|| 0x00
|| u32be(format_version)
|| u64be(payload_len)
|| payload
```

with:

```text
format_version = 1
payload_len     = exact payload byte count
```

All integers are unsigned fixed-width big-endian. There is no text rendering, platform layout, padding, locale, JSON field-order, Rust `Debug`, or Rust `Display` authority in the frame.

## Closed v1 kind registry

The public framing encoder accepts a typed enum, not an arbitrary caller-supplied domain string.

The frozen domains are:

```text
EvidenceSessionBinding
    evidence-session-binding

EndpointSample
    endpoint-sample

ConsecutiveSampledPresence
    consecutive-presence

StepExecutionReceipt
    step-execution-receipt

FixedCadenceStepReceipt
    fixed-cadence-step-receipt

TimedSampledPresenceTransition
    timed-sampled-transition
```

`EvidenceSessionBinding` is included before v1 qualification because PHYS-EVID-05 / #1074 is a prerequisite of persistent semantic claim identity. Without a dedicated domain, the qualified session binding would either be misframed as another evidence kind or force an avoidable immediate v2 solely to persist the lineage namespace.

Each domain is ASCII and is followed by exactly one NUL terminator before the fixed-width version field.

Existing domain bytes must never be reassigned to a different semantic family. New domains/profile interpretations require a later explicitly versioned theorem.

## Resource bounds

V1 freezes:

```text
maximum payload length     = 1_048_576 bytes
maximum kind-domain length = 32 bytes, excluding terminator
```

Encoding rejects a payload larger than 1 MiB.

Decoding parses the declared `u64` length, compares it against the 1 MiB bound before converting/using it as an allocation size, and returns a borrowed payload slice. The decoder performs no allocation based on attacker-declared payload length.

## Canonical decoder

The v1 parser rejects:

- an input shorter than the complete magic;
- wrong magic;
- no kind terminator within the frozen kind-domain bound;
- unknown kind domain;
- truncated version/length header;
- any version other than 1;
- declared payload length above 1 MiB;
- payload shorter than its declared length;
- any byte after the declared payload.

Therefore admitted frames have no trailing-byte aliases.

For every admitted frame:

```text
decode(frame) = (kind, payload)
encode(kind, payload) = exactly frame
```

## Framing is not evidence promotion

`encode_physics_evidence_frame_v1(kind, payload)` intentionally accepts byte payloads because it is a low-level framing primitive.

That fact creates no implication:

```text
caller can frame bytes
=> caller can mint qualified physics evidence
```

Future semantic codecs must separately accept the appropriate already-qualified semantic token/session inputs and produce canonical payload bytes.

Likewise `decode_physics_evidence_frame_v1` returns only a borrowed framing-level view containing:

```text
PhysicsEvidenceFrameKindV1
&[u8] payload
```

It never returns, reconstructs, or promotes into a live authority-issued physics evidence token or qualified evidence-session binding.

This theorem therefore adds representation, not authority.

## No semantic payload authority yet

PHYS-EVID-02A does not define the canonical fields for:

- evidence-session binding;
- physical authority identity;
- world generation;
- temporal incarnation;
- step stamp;
- subject identity;
- endpoint region;
- membership;
- predecessor claim identity;
- fixed cadence policy;
- timed interval;
- dwell/arrival/delivery policy.

The dedicated `EvidenceSessionBinding` frame domain reserves type separation for the upcoming PHYS-EVID-05 canonical payload, but the frame layer does not define or validate that payload's fields.

Those remain successor semantic-codec work after the required evidence-session theorem exists.

## No cryptographic authority

The frame is suitable as an exact transcript for a later domain-separated commitment/authentication layer.

PHYS-EVID-02A itself does not select a hash, signature, key, issuer, timestamp, consensus protocol, or persistence verifier.

Canonical framing proves deterministic representation only. It does not prove authenticity.

## Golden-vector corpus

The checked-in JSON corpus freezes representative vectors for all six v1 kinds and includes:

- the dedicated evidence-session-binding domain;
- empty payload;
- zero bytes;
- high-bit/non-UTF8 bytes;
- representative multi-byte payloads;
- distinct domains for identical framing rules.

Rust tests reconstruct each vector using the production encoder, compare exact expected hex, decode it, and re-encode it byte-identically.

The Rust tests also bind their expected vector hex to the checked-in JSON file.

An independent Python standard-library oracle parses the JSON, independently implements the frozen framing formula, and compares its output to every expected hex string. It does not invoke Rust and does not parse Rust source to obtain framing constants.

## Adversarial decoder corpus

Qualification requires at minimum:

- wrong magic rejects;
- truncated magic rejects;
- missing kind terminator rejects;
- unknown kind rejects;
- unsupported version rejects;
- oversized declared payload rejects before payload allocation;
- truncated fixed header rejects;
- truncated payload rejects;
- trailing bytes reject;
- non-UTF8 payload round-trips unchanged;
- maximum admitted payload succeeds;
- payload above the limit is rejected by the encoder;
- same payload under two kinds yields different bytes;
- all v1 domains are distinct;
- decode/re-encode yields byte identity.

## Relationship to PHYS-EVID-02 / #1064

This is only the framing subtheorem.

The later full PHYS-EVID-02 semantic codec should look conceptually like:

```text
qualified semantic evidence
+ qualified persistent evidence session
        ↓
canonical semantic payload(s)
        ↓
PHYS-EVID-02A typed frame(s)
```

The framing module must remain unable to decide whether a supplied payload corresponds to a valid session binding, endpoint, timing, dwell, arrival, or settlement semantic value.

## Relationship to PHYS-EVID-05 / #1074

Persistent semantic claim payloads must bind the qualified evidence-session identity or an inseparable canonical reference to it.

V1 therefore reserves the exact `evidence-session-binding` frame domain before qualification. A successor codec can fill that domain only from qualified PHYS-EVID-05 semantics.

A process-local temporal incarnation alone must never be treated as globally unique durable evidence identity.

## Non-claims

No canonical semantic evidence payload schema, no EvidenceSessionId qualification by this framing module, no semantic claim identity, no live-token/session-binding reconstruction, no evidence promotion, no hash collision resistance, no signature authenticity, no issuer/key identity, no timestamp authority, no persistence trust, no consensus, no continuous occupancy, no dwell, no arrival, no custody, no delivery, and no settlement eligibility are established here.
