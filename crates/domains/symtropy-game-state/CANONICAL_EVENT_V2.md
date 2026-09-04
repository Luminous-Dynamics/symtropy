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
  repeated string(parent_id) in sorted canonical parent order
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

## Compatibility boundary

V1 and v2 make deliberately different claims:

```text
v1 = historical Rust/JSON integrity chain
v2 = serializer-independent canonical causal identity
```

A future continuation manifest or cross-version replay contract must identify which claim class a journal head represents. A v1 head must never be silently promoted into a v2 canonical-causal claim.
