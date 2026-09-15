# PHYS-EVID-01A CANONICAL ENDPOINT CLAIM POSITION / VALUE v0.1

Status: proposed semantic-identity theorem; exact-head executable qualification required before PASS is claimed.

Issue: PHYS-EVID-01A / #1119. Parent architecture: PHYS-EVID-01 / #1063.

Exact predecessor:

```text
PHYS-EVID-02C / #1117
e8753451e522ec343dc9669685ddc404f1cdade6
```

## Purpose

Separate one admitted persistent endpoint sample into two language-neutral semantic transcripts:

```text
position
    = which exact proposition at which exact qualified sample coordinate

value
    = what exact endpoint facts/result are asserted at that position
```

Then classify two admitted records as:

```text
DistinctPosition
Duplicate
Contradiction
```

This is the first concrete implementation of PHYS-EVID-01 semantic identity. It is deliberately narrower than replay-safe action execution.

## Why position and value must be separate

The endpoint result is not part of the position key.

If `Inside`/`Outside` were placed in the position, then:

```text
same session / step / subject / endpoint proposition -> Inside
same session / step / subject / endpoint proposition -> Outside
```

would become two unrelated keys and the evidence layer could no longer surface a fork/corruption condition.

The same reasoning applies to observed target/anchor translations and the derived PHYS-OBS-03 center/offset fields. They are facts asserted at one position, not coordinates selecting the position.

Therefore PHYS-EVID-01A freezes:

```text
same position + same value      = Duplicate
same position + different value = Contradiction
different position              = DistinctPosition
```

## Admission boundary

The only public derivation path is:

```text
derive_endpoint_sample_claim_v1(frame_bytes)
```

It first calls:

```text
decode_endpoint_sample_v1(frame_bytes)
```

from PHYS-EVID-02C.

No caller-field constructor exists for the position or value wrappers. Malformed or internally contradictory endpoint bytes therefore reject before PHYS-EVID-01A can derive semantic identity.

This preserves #1117 as the single canonical semantic-admission predicate.

## Exact position semantics

For one admitted endpoint sample, position contains exactly:

```text
PHYS-EVID-01A endpoint position profile/version
EvidenceSessionId exact 32 bytes
PhysicalAuthorityId raw u128
WorldGenerationId raw u128
LocalTemporalIncarnationId raw u64
EvidenceSessionProfileV1 raw u32
dimension D
step_index
target NetId
anchor NetId
center_offset bits[D]
half_extent bits[D]
```

The session fields are taken from the decoded PHYS-EVID-02B semantic record. PHYS-EVID-01A does **not** copy the nested PHYS-EVID-02A frame bytes into the position transcript.

This matters because generic framing syntax, transport envelopes, signer identity, or future re-attestation must not redefine which semantic physical claim is being identified.

## Exact value semantics

The value contains exactly:

```text
PHYS-EVID-01A endpoint value profile/version
dimension D
membership
target_translation bits[D]
anchor_translation bits[D]
region_center bits[D]
offset_from_center bits[D]
```

These fields have already passed PHYS-EVID-02C's finite/canonical checks and its independent geometry/membership recomputation.

The value transcript does not independently reimplement PHYS-OBS-03 geometry. #1117 remains the authority for semantic admission; #1119 only separates the admitted semantics into position and value.

## Position transcript v1

Exact bytes:

```text
ASCII "symtropy-physics-claim-position\0"
ASCII "endpoint-sample-v1\0"
u32be(transcript_version = 1)
u16be(dimension)
32 bytes EvidenceSessionId
u128be(physical_authority)
u128be(world_generation)
u64be(temporal_incarnation)
u32be(session_profile)
u64be(step_index)
u64be(target_net_id)
u64be(anchor_net_id)
D × u64be(center_offset_bits)
D × u64be(half_extent_bits)
```

Length:

```text
157 + 16*D bytes
```

For D=3 the exact position transcript is 205 bytes.

## Value transcript v1

Exact bytes:

```text
ASCII "symtropy-physics-claim-value\0"
ASCII "endpoint-sample-v1\0"
u32be(transcript_version = 1)
u16be(dimension)
u8(membership: 0 Outside, 1 Inside)
D × u64be(target_translation_bits)
D × u64be(anchor_translation_bits)
D × u64be(region_center_bits)
D × u64be(offset_from_center_bits)
```

Length:

```text
55 + 32*D bytes
```

For D=3 the exact value transcript is 151 bytes.

The different magic namespaces provide explicit position/value domain separation before a cryptographic commitment algorithm is introduced.

## Semantic session identity, not nested framing identity

PHYS-EVID-02C embeds an exact canonical session-binding frame so the endpoint decoder can single-source session validation through PHYS-EVID-02B.

PHYS-EVID-01A deliberately projects that admitted nested frame down to its semantic fields:

```text
session_id
A
G
live_incarnation
session_profile
```

and commits those fields to the position transcript.

It does not commit:

```text
PHYS-EVID-02A magic
frame kind-domain text
outer frame version
payload-length field
```

as semantic claim position identity.

Thus generic encoding/transport syntax does not become the proposition key.

## Duplicate theorem

Repeated receipt/transport/decoding of the same canonical endpoint claim produces byte-identical position and value transcripts.

```text
same frame decoded twice
    -> same position
    -> same value
    -> Duplicate
```

Non-Clone Rust ownership does not participate. Object allocation, pointer identity, receipt filename, workflow run, transport packet, database row ID, and signature envelope do not participate.

## Distinct-position theorem

At minimum, changing any of these creates a different position:

- persistent session ID;
- physical authority;
- world generation;
- temporal incarnation;
- session profile;
- dimension;
- step index;
- target NetId;
- anchor NetId;
- center-offset bits;
- half-extent bits.

This prevents overbroad deduplication. The same subject/region at a later legitimate step is a distinct claim position.

## Contradiction theorem

For one exact position, any difference in the exact admitted value transcript is a contradiction at the semantic-record layer.

Examples include different:

- target translation bits;
- anchor translation bits;
- region-center bits;
- offset bits;
- membership.

PHYS-EVID-02C ensures each individual record is internally self-consistent. Therefore PHYS-EVID-01A contradiction means two separately admitted records assert incompatible facts at one unique semantic sample position.

At this stage the records are not yet authenticated. `Contradiction` therefore means semantic incompatibility of canonical records, not yet proof that a qualified external issuer equivocated.

After PHYS-EVID-03 authentication, a successor theorem may promote such a pair into authenticated fork/corruption evidence.

## No cryptographic commitment yet

The canonical transcripts are durable language-neutral bytes, but PHYS-EVID-01A does not select a hash algorithm.

In particular it does not use or authorize:

```text
std::hash::Hash
DefaultHasher
pointer/address identity
Debug / Display output
JSON formatting/order
UUID generated by storage
file name
workflow run ID
signature bytes
signer-key fingerprint
```

as semantic claim identity.

PHYS-EVID-01D / #1122 is reserved for the later domain-separated cryptographic commitment theorem.

## No trusted replay registry yet

PHYS-EVID-01A intentionally provides pure derivation/comparison only.

It does **not** expose a durable dedup table, compare-and-set store, settlement ledger, or side-effect executor. Canonical bytes remain unauthenticated until PHYS-EVID-03.

PHYS-EVID-01C / #1121 is reserved for ActionKey + durable at-most-once consumer semantics after a verified/authenticated persistent evidence type exists.

This avoids accidentally treating canonical syntax as economic/governance authority.

## Golden vector

The language-neutral vector reuses the exact D=3 PHYS-EVID-02C golden endpoint frame.

Expected position transcript length:

```text
205
```

Expected value transcript length:

```text
151
```

The checked-in fixture freezes exact transcript hex for both. Rust derives those bytes through the production decoder. An independent Python standard-library oracle parses the PHYS-EVID-02C fixture, extracts semantic session/endpoint fields, and independently reconstructs both transcripts without invoking Rust or reading Rust source.

## Required adversarial corpus

At minimum:

- same frame twice -> `Duplicate`;
- independently decoded same canonical bytes -> identical position/value;
- same position with a second valid but different endpoint value -> `Contradiction`;
- adjacent step -> `DistinctPosition`;
- different persistent session -> `DistinctPosition`;
- different live incarnation -> `DistinctPosition`;
- different target -> `DistinctPosition`;
- different anchor -> `DistinctPosition`;
- changed center offset with self-consistent derived geometry -> `DistinctPosition`;
- changed half extent -> `DistinctPosition`;
- malformed endpoint membership rejects before claim derivation;
- membership/translation/result changes do not alter position;
- region proposition changes do not merely alter value;
- claim source contains no nested-session-frame copy into the position transcript;
- no public raw-field constructor exists;
- no std-hash-derived durable ID exists.

## Successor architecture

```text
#1119 / PHYS-EVID-01A
endpoint position/value transcripts + pure comparison
        ↓
#1120 / PHYS-EVID-01B
ordered derived-claim identity
        ↓
#1122 / PHYS-EVID-01D
cryptographic semantic commitments
        +
PHYS-EVID-03 authenticated verified evidence
        ↓
#1124 / PHYS-EVID-01E
authenticated contradiction/fork handling
        ↓
#1121 / PHYS-EVID-01C
ActionKey + durable at-most-once side effects
```

## Evidence status

This theorem is a source successor of source-only #1117. It cannot upgrade #1117 or any predecessor lane merely by compiling in the same tree.

A PASS requires its own exact-head executable qualification plus the appropriate frozen predecessor qualification evidence.

## Non-claims

No new observation correctness, no authentication, no issuer/key identity, no cryptographic collision resistance, no trusted timestamp, no authenticated fork proof, no durable replay database, no side-effect authorization, no continuous occupancy/dwell/arrival, no custody/delivery, no consensus, and no settlement authority.