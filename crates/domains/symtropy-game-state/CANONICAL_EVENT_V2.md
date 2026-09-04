# Canonical Event Identity v2

Status: implementation contract for `symtropy-game-state` canonical causal identity v2.

This contract is additive. Historical `EventChain<T>` v1 identifiers and JSON-backed event hashes remain unchanged and are not reinterpreted as canonical v2 identities.

## Purpose

V1 hashes `serde_json` bytes for a generic serializable payload. That remains useful for historical Rust-local integrity, but it is not a portable cross-version/cross-language causal identity contract.

V2 instead binds an explicit binary grammar and a domain-owned typed payload digest.

## Primitive grammar

All canonical v2 digests use SHA-256.

Every digest starts with:

```text
ASCII domain separator || 0x00
```

Domain separators must be non-empty ASCII and may not contain NUL.

Primitive values are encoded as:

```text
u8         = one byte
u32        = 4-byte unsigned big-endian
u64        = 8-byte unsigned big-endian
bytes      = u64_be(length) || exact bytes
string     = bytes(UTF-8)
count      = u64_be(number of elements)
option     = 0x00 for None; 0x01 || value for Some
sha256     = exact 32 digest bytes, no length prefix
```

No `usize`, native-endian integer, JSON number spelling, map iteration order, wall-clock value, or serializer output participates implicitly.

## Stable deterministic event IDs

Canonical-v2 deterministic IDs use a validated `StableIdNamespace`.

A namespace:

- is non-empty;
- uses only ASCII alphanumeric, `.`, `-`, `_`, `:`;
- is at most 63 bytes so `namespace:<32 hex chars>` remains within the existing 96-byte `StableId` limit.

ID derivation is:

```text
SHA256(
  "symtropy/stable-id/v2" || 0x00 ||
  u64_be(namespace_utf8_length) || namespace_utf8 ||
  u64_be(seed) ||
  u64_be(ordinal)
)
```

The stable text ID is:

```text
namespace || ":" || lowercase_hex(first_16_digest_bytes)
```

This path does not modify historical `StableId::derive` behavior.

## Domain-owned payload identity

A canonical payload implements `CanonicalEventPayload`:

```rust
const PAYLOAD_SCHEMA: &'static str;
fn canonical_payload_digest(&self) -> PayloadDigest;
```

The owning domain defines the semantic payload digest. The event layer never treats serialized payload bytes as canonical identity.

The event binds both:

```text
payload_schema
payload_digest
```

so equal digest bytes under different semantic schemas remain distinguishable.

Any payload field that can change authoritative interpretation, replay, or continuation semantics **must** participate in the domain-owned payload digest. Omitting such a field is a domain contract violation even if the outer event hash remains structurally valid.

## Canonical v2 event digest

Domain separator:

```text
symtropy/game-state/event/v2
```

The event digest hashes, in this exact order:

```text
u32_be(schema_version)
string(event_id)
u64_be(simulation_tick)
string(kind)
option(string(actor_id))
option(string(observer_id))
count(causal_parents)
  repeated string(parent_id) in ascending bytewise order of portable StableId text
string(payload_schema)
sha256(payload_digest)
option(sha256(previous_event_digest))
```

The typed payload object itself is not hashed here.

Genesis uses `previous_event_digest = None`; there is no magic `"GENESIS"` digest string in v2.

## Causal-parent semantics

V2 declares direct causal parents to have set semantics for event identity.

Therefore:

- parent order does not change event identity;
- canonical parent ordering is ascending bytewise order of the validated ASCII `StableId` text;
- duplicate parent IDs are invalid;
- every parent must exist in the same verified chain;
- every parent must occur strictly before the child;
- self-parent and future-parent references fail closed.

If a future event needs ordered causal roles, those roles must be represented explicitly in the event/payload schema rather than smuggled through vector insertion order.

## Verification

A reconstructed `EventChainV2` verifies at least:

1. recognized event schema;
2. validated namespace and portable IDs;
3. deterministic event ID for each ordinal;
4. exact previous canonical digest link;
5. monotonic simulation ticks;
6. valid and strictly earlier causal parents;
7. stable payload schema agreement with the domain type;
8. recomputed domain-owned payload digest;
9. recomputed canonical event digest.

### Verified-chain authority boundary

`EventChainV2` is an authoritative verified-chain type, not a container for "possibly verified" data.

Therefore:

- serde deserialization of `EventChainV2<T>` runs the full verifier before returning a chain;
- `EventChainV2::from_events(...)` runs the full verifier before returning a chain;
- a syntactically valid but semantically inconsistent event ID, previous link, payload digest, causal parent, tick ordering, or schema cannot yield a usable `EventChainV2` value through those reconstruction paths;
- v2 envelope deserialization validates the portable grammar of event IDs, actor/observer IDs, parent IDs, event kinds, and payload-schema identifiers at the serde boundary;
- historical v1 `StableId` deserialization remains unchanged.

`EventEnvelopeV2<T>` remains the structural carrier for tools that need to inspect individual unverified or future-schema records. Such envelopes do not become authoritative chain state until a current semantic verifier accepts the complete chain.

A structurally hash-consistent unknown schema is not interpreted under current v2 semantics.

## Frozen golden vector 001

Input:

```text
namespace       = fold.event
seed            = 91
ordinal         = 0
simulation_tick = 7
kind            = fold.observed
actor           = None
observer        = None
parents         = []
payload_schema  = test.payload.v1
payload value   = u32 5
payload domain  = symtropy/test-payload/v1
previous        = None
```

Expected:

```text
event_id = fold.event:51dcf21565f1ac6e2f0d3c63c36b5f87

payload_digest =
fb6f135dd2a33020e10c8af60da6b22a6e662fa02e523415c49ecc9f02778a83

event_digest =
7c52f0ef452a98cf2d32523d16da2abf1e411d226ece106c0c757d1e89cf4fb2
```

The expected vector was independently derived from this byte contract rather than copied from the Rust implementation's runtime output.

## Frozen golden vector 002 — non-genesis

Vector 002 starts from vector 001 as the exact first event in the same chain, then appends a second event. It exists to exercise all of the canonical option/link fields that vector 001 leaves empty.

Input for the second event:

```text
namespace       = fold.event
seed            = 91
ordinal         = 1
simulation_tick = 8
kind            = fold.rewind.applied
actor           = Some(actor:player)
observer        = Some(observer:alice)
parents         = [fold.event:51dcf21565f1ac6e2f0d3c63c36b5f87]
payload_schema  = test.payload.v1
payload value   = u32 9
payload domain  = symtropy/test-payload/v1
previous        = 7c52f0ef452a98cf2d32523d16da2abf1e411d226ece106c0c757d1e89cf4fb2
```

Expected:

```text
event_id = fold.event:00fb94311f6bf3be801601321504d560

payload_digest =
1d2b7ec51b918b7e3c7b4953f5a2796b2dc58c1e091b0a501896a819957cbd63

previous_digest =
7c52f0ef452a98cf2d32523d16da2abf1e411d226ece106c0c757d1e89cf4fb2

event_digest =
bdb881578c4db99b954d4bbb1907adeaede631f9b8b49db3396c81c08dcc74a7
```

Vector 002 was independently derived from the same frozen byte grammar. Together, vectors 001 and 002 cover genesis/non-genesis linkage, both option states for actor/observer/previous digest, and a non-empty causal-parent set.

## Frozen canonical preimages

The following lowercase hex strings are the exact bytes fed to SHA-256. They are frozen alongside the final digests so cross-language implementations can distinguish framing errors from hashing errors.

Vector 001 stable-ID preimage, 56 bytes:

```text
73796d74726f70792f737461626c652d69642f763200000000000000000a666f6c642e6576656e74000000000000005b0000000000000000
```

Vector 001 payload preimage, 29 bytes:

```text
73796d74726f70792f746573742d7061796c6f61642f76310000000005
```

Vector 001 event preimage, 179 bytes:

```text
73796d74726f70792f67616d652d73746174652f6576656e742f76320000000002000000000000002b666f6c642e6576656e743a35316463663231353635663161633665326630643363363363333662356638370000000000000007000000000000000d666f6c642e6f6273657276656400000000000000000000000000000000000f746573742e7061796c6f61642e7631fb6f135dd2a33020e10c8af60da6b22a6e662fa02e523415c49ecc9f02778a8300
```

Vector 002 stable-ID preimage, 56 bytes:

```text
73796d74726f70792f737461626c652d69642f763200000000000000000a666f6c642e6576656e74000000000000005b0000000000000001
```

Vector 002 payload preimage, 29 bytes:

```text
73796d74726f70792f746573742d7061796c6f61642f76310000000009
```

Vector 002 event preimage, 310 bytes:

```text
73796d74726f70792f67616d652d73746174652f6576656e742f76320000000002000000000000002b666f6c642e6576656e743a303066623934333131663662663362653830313630313332313530346435363000000000000000080000000000000013666f6c642e726577696e642e6170706c69656401000000000000000c6163746f723a706c6179657201000000000000000e6f627365727665723a616c6963650000000000000001000000000000002b666f6c642e6576656e743a3531646366323135363566316163366532663064336336336333366235663837000000000000000f746573742e7061796c6f61642e76311d2b7ec51b918b7e3c7b4953f5a2796b2dc58c1e091b0a501896a819957cbd63017c52f0ef452a98cf2d32523d16da2abf1e411d226ece106c0c757d1e89cf4fb2
```

## Compatibility boundary

V1 and v2 make deliberately different claims:

```text
v1 = historical Rust/JSON integrity chain
v2 = serializer-independent canonical causal identity
```

A future continuation manifest or cross-version replay contract must identify which claim class a journal head represents. A v1 head must never be silently promoted into a v2 canonical-causal claim.
