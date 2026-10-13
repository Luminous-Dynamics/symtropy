# Qualified Robotics World and Frame Convention Registry V1/V2

**Status:** Draft contract; Rust execution and CI qualification pending  
**Implementation:** `symtropy-robotics-bridge-core::world_frame_conventions`  
**Related:** PR #1606 (base world commitment), ROS REP-103, ROS REP-105

## Why this is separate from the v1 world commitment

The existing `WorldManifestV1` and its canonical commitment remain byte-for-byte unchanged. A frame ID, parent link, translation, and XYZW quaternion do not by themselves tell a consumer whether components mean forward/left/up, east/north/up, north/east/down, or camera right/down/forward.

The new `QualifiedWorldManifestV2` wraps a validated `WorldManifestV1` and a separate explicit `FrameConventionRegistryV1`. Every frame declared by the world must have exactly one convention binding. Unknown frame IDs, missing bindings, duplicate bindings, unsupported registry versions, and invalid external-spec references are rejected.

## Frozen convention tags

The registry uses explicit numeric tags; Rust enum declaration order is not hashed implicitly.

| Tag | Convention | Positive axes |
|---:|---|---|
| 0 | `BodyForwardLeftUp` | +X forward, +Y left, +Z up |
| 1 | `EnuEastNorthUp` | +X east, +Y north, +Z up |
| 2 | `NedNorthEastDown` | +X north, +Y east, +Z down |
| 3 | `CameraOpticalRightDownForward` | +X right, +Y down, +Z forward |
| 4 | `FrdForwardRightDown` | +X forward, +Y right, +Z down |
| 255 | `ExternalSpec` | Named external convention, pinned by URI and 32-byte SHA-256 |

Tags 0–4 are based on conventions described in [ROS REP-103](https://reps.openrobotics.org/rep-0103/). In particular, REP-103 describes right-handed frames, body x-forward/y-left/z-up, ENU, NED secondary frames, and camera optical axes. REP-105 defines common mobile-platform frame semantics including `earth`, `map`, `odom`, and `base_link`; the convention registry does not replace that frame-topology specification.

The metadata is a declaration, not an automatic coordinate conversion. Consumers must understand the declared convention before interpreting or transforming vectors. Translations remain metres and existing static rotations remain XYZW, as specified by the base world contract. The convention label must not be inferred from a frame's name.

For `ExternalSpec`, both a non-empty URI and a 64-character SHA-256 string are required. Hex case is normalized to 32 raw bytes. The library pins the referenced specification's identity but does not fetch the URI or independently verify external document bytes.

## Registry canonical bytes V1

The registry commitment uses SHA-256 over these bytes:

1. ASCII `symtropy.robotics.frame-conventions.v1` followed by one NUL byte.
2. Canonical-format version as unsigned 16-bit big-endian integer `1`.
3. Registry schema version as unsigned 16-bit big-endian integer `1`.
4. Binding count as unsigned 64-bit big-endian integer.
5. Bindings sorted by exact UTF-8 frame ID; each binding has the frame ID as unsigned-64-bit-length-prefixed UTF-8, followed by its one-byte convention tag. Tag 255 additionally encodes the specification URI using the same string grammar and then the raw 32-byte specification digest.

UTF-8 strings are not Unicode-normalized. Duplicate frame IDs are invalid rather than resolved by ordering. The canonical registry output is capped at 4 MiB.

## Qualified world commitment V2

The qualified commitment deliberately composes the two existing subcommitments rather than rewriting v1 bytes:

```text
SHA256(
  "symtropy.robotics.qualified-world.v2" || NUL
  || u16be(2)
  || WorldManifestDigestV1[32]
  || FrameConventionDigestV1[32]
)
```

The resulting digest is `QualifiedWorldDigestV2`. The qualified envelope therefore binds both the full base manifest and the convention registry. A change to a convention changes the qualified-world digest even when scenario identity and generation stay constant.

`QualifiedSolverRequestV2::bind` associates the base request with the qualified-world digest. `QualifiedSolverResultEnvelopeV2::bind` carries the same digest on the result. `QualifiedWorldManifestV2::validate_solver_result` recomputes the qualified digest, requires request and result digests to match the active qualified world, then performs the existing v1 solver admission checks. A base-manifest-only admission call does not claim convention-qualified admission; use the V2 path for frame-dependent quantities.

## Golden vectors

For the canonical `commitment-smoke-v1` fixture inherited from the v1 commitment test:

- World v1 digest: `136ddd6d2d42cd9d496c53c1141f5bafc576b6a8c1e5517dea80770171fe8f26`.
- Convention registry canonical byte length: `107`.
- Convention registry digest: `2ff85b064397f5e636b8b7510c7f7322581300d9d0d35c8a348dce6d50c7c0d2`.
- Qualified world v2 digest: `962c4a535296a60b34905d73393e820c193d1eac7c87b1f53d2702b2e1f9f38b`.

The Rust unit tests and `scripts/verify_frame_convention_registry_v1.py` target these vectors. The Python checker uses only the standard library and independently spells out the byte grammar; a local successful Python run is not evidence that Rust compiles or that GitHub CI passed.

## Non-claims and next qualification gates

This contract does not prove a solver actually interpreted axes correctly, that external specifications were fetched and verified, that units are physically appropriate, that a solver ran, or that results are numerically or physically accurate. There is no automatic matrix conversion routine yet. Future conversions should be explicit, tested, and convention-aware, not an implicit side effect of hashing labels.

Before promotion, run Rust formatting, focused crate tests, Clippy, and exact-head CI. Missing, queued, or unexecuted checks are not PASS.
