# PHYS-EVID-02B CANONICAL EVIDENCE SESSION BINDING v0.1

Status: proposed semantic representation theorem; exact-head executable qualification required before PASS is claimed.

Issue: PHYS-EVID-02B / #1111.

Exact predecessors:

```text
PHYS-EVID-05 / #1109 persistence-safe qualified session binding
PHYS-EVID-02A / #1107 canonical typed framing
```

joined by the integration-only two-parent convergence root #1112.

## Purpose

Define the first qualified semantic payload carried by the canonical physics evidence frame: one exact PHYS-EVID-05 session binding.

The authority flow is deliberately one-way:

```text
QualifiedEvidenceSessionBinding
        ↓
canonical 80-byte binding payload
        ↓
PhysicsEvidenceFrameKindV1::EvidenceSessionBinding
        ↓
canonical frame bytes
```

The reverse direction is intentionally weaker:

```text
canonical frame bytes
        ↓
DecodedEvidenceSessionBindingV1
        ↛
QualifiedEvidenceSessionBinding
```

Canonical decoding therefore never recreates bootstrap authority.

## Public encoder authority

The sole public semantic encoder accepts only:

```text
&QualifiedEvidenceSessionBinding
```

It accepts no detached session bytes, authority/generation IDs, incarnation number, or profile code.

A raw-field serializer exists only as a private implementation helper used by the production encoder and local golden-vector tests. It is not exported and is not an evidence-minting path.

## Exact payload v1

The canonical semantic payload is exactly 80 bytes:

```text
offset  size  field
0       4     u32be(binding_payload_version = 1)
4       32    EvidenceSessionId exact bytes
36      16    u128be(PhysicalAuthorityId)
52      16    u128be(WorldGenerationId)
68      8     u64be(LocalTemporalIncarnationId)
76      4     u32be(EvidenceSessionProfileV1 code)
80      -     end
```

No field is variable-length. No native-endian representation, Rust memory layout, padding, decimal/hex text, UUID string, JSON field order, `Debug`, or `Display` output participates.

## Version separation

Two independent version authorities are frozen:

```text
PHYS-EVID-02A outer frame version = 1
PHYS-EVID-02B binding payload version = 1
```

The outer version controls framing syntax. The inner version controls the semantic field schema for this evidence kind. A future change to one must not silently reinterpret the other.

## Exact session profile

V1 binds:

```text
EvidenceSessionProfileV1::OsCsprngBoundLiveIncarnation
code = 1
```

so the bytes preserve the exact PHYS-EVID-05 bootstrap profile, not merely its 256-bit session namespace and live-lineage fields.

## Canonical semantic admission

After PHYS-EVID-02A admits the outer frame, PHYS-EVID-02B rejects:

- any outer frame kind other than `EvidenceSessionBinding`;
- payload length other than exactly 80 bytes;
- binding payload version other than 1;
- an all-zero 32-byte session ID;
- zero physical authority ID;
- zero world generation ID;
- zero temporal incarnation ID;
- any session profile code other than 1.

The zero-incarnation rejection is required because PHYS-OBS-04A1/#1086 mints `LocalTemporalIncarnationId` from a checked allocator beginning at 1. Canonical decoded semantics must not admit a binding that the qualified live producer cannot issue.

## Decoder authority boundary

`decode_evidence_session_binding_v1` returns a distinct plain record:

```text
DecodedEvidenceSessionBindingV1 {
    session_id_bytes: [u8; 32],
    physical_authority_raw: u128,
    world_generation_raw: u128,
    temporal_incarnation_raw: u64,
    session_profile_code: u32,
}
```

Its fields are private and exposed only through read-only accessors.

It deliberately does not return or construct:

```text
EvidenceSessionId
QualifiedEvidenceSessionBinding
PersistentEvidenceSession
LocalTemporalIncarnationId
PhysicalAuthorityId
WorldGenerationId
```

as authority-bearing values. The decoded record means only that the bytes obey the canonical v1 semantic syntax. Authentication and proof that an accepted bootstrap actually issued the binding remain later theorems.

## No attacker-sized semantic allocation

The payload is fixed at 80 bytes. The outer PHYS-EVID-02A parser already bounds and borrows the payload before semantic decoding.

PHYS-EVID-02B performs only fixed-size stack copies and fixed-width integer reconstruction. It does not allocate vectors, strings, maps, or buffers based on attacker-declared semantic lengths.

## Golden vector

The v1 language-neutral fixture freezes:

```text
binding_payload_version = 1
session_id_bytes        = 00 01 02 ... 1f
physical_authority      = 0x0102030405060708090a0b0c0d0e0f10
world_generation        = 0x1112131415161718191a1b1c1d1e1f20
temporal_incarnation    = 0x2122232425262728
session_profile         = 1
```

Expected 80-byte payload hex:

```text
00000001000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20212223242526272800000001
```

Expected fully framed hex:

```text
73796d74726f70792d706879736963732d65766964656e63650065766964656e63652d73657373696f6e2d62696e64696e670000000001000000000000005000000001000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20212223242526272800000001
```

A checked-in Python standard-library oracle independently reconstructs the payload and outer frame from fixture fields, then parses both back without invoking Rust or reading Rust source.

## Production-path corpus

Golden bytes alone do not qualify the authority projection. Rust must also:

1. mint a real `LocalQualifiedPhysicalAuthority` and generation;
2. seal through PHYS-OBS-04A1/#1086;
3. bootstrap a real PHYS-EVID-05 `PersistentEvidenceSession` through the OS CSPRNG path;
4. call the public qualified-binding encoder;
5. decode the frame into the plain record;
6. prove each decoded field equals the live qualified binding accessor exactly.

This links canonical bytes to the real qualified producer without exposing a raw public field encoder.

## Persistent-session separation

Changing only `EvidenceSessionId` while authority, generation, incarnation, and profile remain equal changes the canonical payload and frame.

Thus restart/export sessions do not alias merely because their local A/G/incarnation values happen to repeat. This represents PHYS-EVID-05's persistent namespace; it does not introduce a new randomness theorem.

## Re-attestation law

No signer key, signature bytes, signature algorithm identifier, or authentication-envelope identity appears in this semantic payload.

A later authorized re-attestation may therefore preserve the same canonical session semantics while changing the authentication envelope. PHYS-EVID-03/#1065 owns that theorem.

## Git convergence theorem

PHYS-EVID-02B is implemented as one semantic child of #1112. Qualification must separately prove that #1112 has exactly these ordered parents:

```text
first  = #1109 head 120932a027bb4b0dc638515a9618bd283be2a6fa
second = #1107 head 4c464ac0fdb794a8d1c055f8da23135727332bf3
```

and preserves the parent-owned framing/session blobs byte-for-byte. Composition does not upgrade either predecessor's qualification status.

## Required executable corpus

At minimum:

- exact 80-byte payload length and field offsets;
- private field codec equals the checked-in golden payload;
- exact payload frames under `EvidenceSessionBinding` to the checked-in golden frame;
- Python oracle independently agrees with fixture payload and frame;
- real OS-CSPRNG PHYS-EVID-05 binding encodes through the public qualified path and decodes field-for-field;
- same A/G/incarnation/profile with different session IDs changes bytes;
- wrong frame kind rejects before semantic interpretation;
- 79-byte and 81-byte payloads reject;
- payload version != 1 rejects;
- all-zero session ID rejects;
- zero authority rejects;
- zero generation rejects;
- zero temporal incarnation rejects;
- unknown profile rejects;
- decoded type fields remain private;
- no public raw-field encoder exists;
- no conversion/constructor from decoded record to qualified session types exists;
- predecessor #1107 and #1109 semantic blobs remain unchanged.

## Evidence boundary

A qualified PASS requires executable evidence for the exact predecessor subjects and this exact semantic child. Source composition alone does not turn queued predecessor lanes into PASS.

## Successor path

Once this canonical session binding is qualified, durable endpoint/timing claims can bind their own semantic fields to this persistence namespace without treating process-local incarnation as globally unique:

```text
canonical qualified session binding
        +
qualified live endpoint/timing token
        ↓
canonical persistent claim payload
        ↓
PHYS-EVID-03 authenticated envelope
```

## Non-claims

No cryptographic authentication, issuer authorization, signature/key identity, trusted timestamp, mathematical collision impossibility, restart continuity restoration, live authority reconstruction, endpoint membership, sampled presence, elapsed duration, continuous occupancy, dwell, arrival, custody, delivery, consensus, or settlement authority is established here.
