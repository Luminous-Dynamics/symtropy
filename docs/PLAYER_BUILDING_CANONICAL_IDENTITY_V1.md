# PB-01 Canonical Identity v1

Status: **normative candidate for #439 H2; no runtime PASS claim**.

This document freezes the serializer-independent byte grammar that hardened
PB-01 intent and plan identity should use. It is deliberately separate from
JSON/wire serialization. A Rust implementation, migration tool, multiplayer
peer, or CAD adapter that sees the same semantic PB-01 value must be able to
reproduce the same bytes without depending on Serde representation choices.

## Compatibility decision

The hardened proposal model is **proposal schema version 2**. Canonical identity
is independently named **canonical identity profile v1**.

That split is intentional:

- pre-hardening experimental PB-01 values used schema 1 and JSON-derived SHA-256;
- H1 adds exact `target_refs` to the wire/semantic manifest;
- H2 replaces JSON-derived digest preimages with the canonical byte grammar here;
- hardened exact intent/plan references carry `schema_version`, so a bare exact
  reference is self-describing enough to distinguish the schema generation that
  defines its identity rules.

Do not silently reinterpret schema-1 hashes as schema-2/canonical-v1 hashes.

## Design rules

1. Identity is SHA-256 over an explicit domain-separated canonical preimage.
2. All integers are fixed-width little-endian.
3. All text is UTF-8 and encoded as `u32 byte_length || exact_bytes`.
4. Every collection is `u32 item_count || items...` in the canonical order
   already enforced by the PB-01 constructors.
5. Every optional is `u8 tag`; `0` means absent, `1` means present and is
   followed immediately by the encoded value. Other tags are invalid.
6. Enum tags are explicit protocol constants below. Rust/Serde discriminants
   are never identity.
7. No field may be omitted because its current value looks redundant.
8. Wire JSON object ordering, whitespace, field naming, pretty printing and
   Serde version are irrelevant to identity.
9. Changing this grammar requires a new identity domain/profile. Changing the
   semantic/wire shape requires a new proposal schema version. Neither may be
   silently reinterpreted.
10. Length conversion overflow fails closed before hashing.

## Primitive encodings

```text
u8(v)       = one byte
u32(v)      = 4 little-endian bytes
u64(v)      = 8 little-endian bytes
i64(v)      = 8 little-endian two's-complement bytes
text(v)     = u32(len(utf8(v))) || utf8(v)
stable_id   = text(id.as_str())
option(x)   = 00 | 01 || encode(x)
vector(xs)  = u32(count(xs)) || encode(xs[0]) || ...
```

`ProposalDigest`:

```text
stable_id(algorithm)
text(value)
```

`ExactAuthorityRef` remains an external-authority reference and therefore does
not inherit the PB proposal schema number:

```text
stable_id(authority_id)
stable_id(subject_id)
u64(revision)
ProposalDigest(content_digest)
```

`ConstructionIntentRef`:

```text
u32(schema_version)
stable_id(intent_id)
u64(revision)
ProposalDigest(content_digest)
```

`ConstructionPlanRef` uses the same self-describing pattern:

```text
u32(schema_version)
stable_id(plan_id)
u64(revision)
ProposalDigest(content_digest)
```

The digest is part of exact ancestry identity. Two same-intent/same-revision
parents with different digests are distinct explicit forks. Schema is also part
of the reference: cross-schema ancestry must be an explicit migration operation,
not an ordinary parent edge.

## Authoring pose

Canonical identity profile v1 uses **authored parameter identity**.

```text
i64(translation_um[0])
i64(translation_um[1])
i64(translation_um[2])
u32(rotation_turn32[0])
u32(rotation_turn32[1])
u32(rotation_turn32[2])
```

Distinct intrinsic XYZ turn32 tuples remain distinct proposal identities even
if downstream geometry shows physical transform equivalence. Snap/CAD adapters
must deterministically choose and record the authored tuple they emit.

## GeometryIntent tags

```text
00 Cuboid:
   u64(size_um[0]) || u64(size_um[1]) || u64(size_um[2])

01 Cylinder:
   u64(radius_um) || u64(length_um)

02 Panel:
   u64(width_um) || u64(height_um) || u64(thickness_um)

03 Beam:
   u64(length_um) || u64(cross_section_um[0]) || u64(cross_section_um[1])

04 ExactExternal:
   ExactAuthorityRef(geometry_ref)
```

## MaterialIntent

```text
stable_id(material_class)
vector(ExactAuthorityRef specification_refs)
```

## IntentElement

```text
stable_id(element_id)
stable_id(role_id)
AuthoringPose(pose)
GeometryIntent(geometry)
MaterialIntent(material)
```

## ConstructionIntent canonical preimage

Domain bytes, including the trailing NUL:

```text
symtropy.player-building.intent.canonical.v1\0
```

Then:

```text
u32(schema_version = 2)
stable_id(intent_id)
u64(revision)
stable_id(proposer_id)
option(stable_id(site_id))
ExactAuthorityRef(authoring_frame_ref)
vector(ConstructionIntentRef parent_refs)
vector(IntentElement elements)
vector(ExactAuthorityRef target_refs)
vector(ExactAuthorityRef source_refs)
```

An intent is semantically valid when it has at least one authored element or
one exact existing target. `target_refs` are therefore identity-bearing intent
scope, not planner-added context.

## ConstructionAction tags

The H1/H3 primary-path cutover is expected to expose the following v1 action
set. Tags are frozen independently of Rust enum declaration order:

```text
00 RealizeElement:
   stable_id(element_id)

01 JoinElements:
   stable_id(first_element_id)
   stable_id(second_element_id)
   stable_id(connection_kind)

02 JoinElementToExactSubject:
   stable_id(element_id)
   ExactAuthorityRef(target)
   stable_id(connection_kind)

03 ModifyExactSubject:
   ExactAuthorityRef(target)
   stable_id(modification_kind)
   option(ExactAuthorityRef(payload_ref))

04 RemoveExactSubject:
   ExactAuthorityRef(target)
   stable_id(removal_kind)

05 AdapterProposal:
   stable_id(profile_id)
   ExactAuthorityRef(payload_ref)
```

## PlannedOperation

```text
stable_id(operation_id)
vector(stable_id depends_on)
ConstructionAction(action)
```

## PlanningContext

```text
stable_id(context_id)
u64(revision)
vector(ExactAuthorityRef exact_inputs)
```

## ConstructionPlan canonical preimage

Domain bytes, including the trailing NUL:

```text
symtropy.player-building.plan.canonical.v1\0
```

Then:

```text
u32(schema_version = 2)
stable_id(plan_id)
u64(revision)
ConstructionIntentRef(intent_ref)
vector(stable_id intent_element_ids)
vector(ExactAuthorityRef intent_target_refs)
ExactAuthorityRef(compiler_ref)
PlanningContext(planning_context)
vector(PlannedOperation operations)
```

The plan snapshot deliberately includes both exact intent identity and its
closed-over element/target subject sets. Restored/replayed plans must resolve
both snapshots against the exact supplied intent before later authority crossing.

## External golden vector A — creation intent

Semantic value:

```text
schema_version = 2
intent_id = "intent:golden"
revision = 1
proposer_id = "actor:golden"
site_id = None
authoring_frame_ref = authority:frame / frame:golden @ 3 / sha256:"frame-digest"
parents = []
elements = [
  element_id = "element:a"
  role_id = "role:beam"
  translation_um = [1, -2, 3]
  rotation_turn32 = [4, 5, 6]
  geometry = Beam { length_um=1000, cross_section_um=[10,20] }
  material_class = "material:timber"
  material specification_refs = []
]
target_refs = []
source_refs = [authority:terrain / cell:golden @ 7 / sha256:"terrain-digest"]
```

Canonical preimage length: **358 bytes**.

Canonical preimage hex:

```text
73796d74726f70792e706c617965722d6275696c64696e672e696e74656e742e63616e6f6e6963616c2e763100020000000d000000696e74656e743a676f6c64656e01000000000000000c0000006163746f723a676f6c64656e000f000000617574686f726974793a6672616d650c0000006672616d653a676f6c64656e0300000000000000060000007368613235360c0000006672616d652d646967657374000000000100000009000000656c656d656e743a6109000000726f6c653a6265616d0100000000000000feffffffffffffff030000000000000004000000050000000600000003e8030000000000000a0000000000000014000000000000000f0000006d6174657269616c3a74696d62657200000000000000000100000011000000617574686f726974793a7465727261696e0b00000063656c6c3a676f6c64656e0700000000000000060000007368613235360e0000007465727261696e2d646967657374
```

SHA-256:

```text
e9c44bc64dd0a41c51924d5cb62e31de98ee9e64ebdbed86b68c6e37a61014ef
```

## External golden vector B — repair-only intent

Semantic value:

```text
schema_version = 2
intent_id = "intent:repair"
revision = 1
proposer_id = "actor:golden"
site_id = None
authoring_frame_ref = authority:frame / frame:golden @ 3 / sha256:"frame-digest"
parents = []
elements = []
target_refs = [authority:construction / structure:shelter @ 7 / sha256:"state-a"]
source_refs = []
```

Canonical preimage length: **252 bytes**.

Canonical preimage hex:

```text
73796d74726f70792e706c617965722d6275696c64696e672e696e74656e742e63616e6f6e6963616c2e763100020000000d000000696e74656e743a72657061697201000000000000000c0000006163746f723a676f6c64656e000f000000617574686f726974793a6672616d650c0000006672616d653a676f6c64656e0300000000000000060000007368613235360c0000006672616d652d64696765737400000000000000000100000016000000617574686f726974793a636f6e737472756374696f6e110000007374727563747572653a7368656c7465720700000000000000060000007368613235360700000073746174652d6100000000
```

SHA-256:

```text
cb57b7458bd85461f2017e9d9419c29b3efcd925282c47768977b96ca0cbf8d7
```

## External golden vector C — one-step plan

This vector references golden intent A by its exact schema-2 SHA-256 above.

Semantic value:

```text
schema_version = 2
plan_id = "plan:golden"
revision = 1
intent_ref = schema 2 / intent:golden @ 1 / sha256:e9c44bc64dd0a41c51924d5cb62e31de98ee9e64ebdbed86b68c6e37a61014ef
intent_element_ids = ["element:a"]
intent_target_refs = []
compiler_ref = authority:planner / planner:golden @ 2 / sha256:"compiler-digest"
planning_context = context:golden @ 4 with exact_inputs = [
  authority:terrain / cell:golden @ 7 / sha256:"terrain-digest"
]
operations = [
  op:realize; depends_on=[]; RealizeElement("element:a")
]
```

Canonical preimage length: **412 bytes**.

Canonical preimage hex:

```text
73796d74726f70792e706c617965722d6275696c64696e672e706c616e2e63616e6f6e6963616c2e763100020000000b000000706c616e3a676f6c64656e0100000000000000020000000d000000696e74656e743a676f6c64656e01000000000000000600000073686132353640000000653963343462633634646430613431633531393234643563623632653331646539386565396536346562646265643836623638633665333761363130313465660100000009000000656c656d656e743a610000000011000000617574686f726974793a706c616e6e65720e000000706c616e6e65723a676f6c64656e0200000000000000060000007368613235360f000000636f6d70696c65722d6469676573740e000000636f6e746578743a676f6c64656e04000000000000000100000011000000617574686f726974793a7465727261696e0b00000063656c6c3a676f6c64656e0700000000000000060000007368613235360e0000007465727261696e2d646967657374010000000a0000006f703a7265616c697a65000000000009000000656c656d656e743a61
```

SHA-256:

```text
28f7b0c0913ac190c67486def4a76cac877d40a9d0eea190dd712ac0e3c04d04
```

## Required implementation tests

The primary H2 cutover must at minimum prove:

- all three external vectors above byte-for-byte and digest-for-digest;
- exact refs carry schema 2 and reject unsupported/cross-schema use unless an
  explicit migration path is invoked;
- caller insertion order cannot change preimage or digest;
- one-micrometre pose drift changes intent bytes/digest;
- authoring-frame drift changes intent bytes/digest;
- target-set drift changes intent bytes/digest;
- concurrent parent digest drift changes intent bytes/digest while same-revision
  forks remain legal ancestry;
- compiler-ref drift changes plan bytes/digest;
- planning-context drift changes plan bytes/digest;
- action variant changes the explicit tag/preimage;
- pretty vs compact JSON has no effect on identity;
- a future Serde field rename/order change cannot affect canonical identity;
- canonical preimage creation fails closed on any impossible length conversion.

## Migration rule

Existing pre-H2 PB-01 hashes derived from `serde_json::to_vec` are experimental
**schema-1 identities**. They are not canonical-v1. If preservation is needed,
a migration reader may recognize schema 1, verify its legacy experimental hash,
and emit a new schema-2 revision/reference. Runtime authority crossing must not
accept schema 1 after the hardened PB-01 cutover.

## Authority statement

Canonical identity proves exact semantic correspondence only. It does not prove
physical safety, available matter, reservation, permission, fabrication success,
construction completion, commissioning, ownership, room semantics, or home.
