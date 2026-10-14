# Robotics World Manifest Commitment V1

**Status:** Implementation/reference contract; Rust execution remains pending  
**Owner:** symtropy-robotics-bridge-core::world_manifest_commitment  
**Related:** PR #1589 (coordinate-frame and solver-profile admission), issue #1064 (canonical evidence encoding), issue #1477 (differential numerical verification)

## Purpose and claim ceiling

A solver request must identify the exact world configuration it was prepared against. The pair scenario_id + world_generation is useful as a lifecycle fence, but it is a caller-maintained counter and does not prove that all physics-relevant manifest fields stayed unchanged.

V1 computes a content digest from the full validated WorldManifestV1. Admission recomputes the digest from the actual manifest and compares it with the digest carried in the solver request.

This proves content-address equality under the SHA-256 collision-resistance assumption. It does not prove that the manifest is authorized, that a solver ran, that any solver is physically accurate, or that a claimed build digest corresponds to an executed binary.

## Domain and digest

Canonical bytes begin with these exact ASCII bytes:

    symtropy.robotics.world-manifest.v1 NUL

This is the UTF-8/ASCII string symtropy.robotics.world-manifest.v1 followed by one zero byte, then the canonical-format version as unsigned 16-bit big-endian integer 1.

The digest is standard SHA-256 over the complete canonical byte string, including domain and format version. The resulting 32 bytes are represented in Rust by WorldManifestDigestV1; lowercase hexadecimal is a display format only, not the hashed input.

The manifest does not contain its own digest. The digest is carried by SolverRequestIdentityV1, avoiding a self-referential commitment.

## Primitive encodings

- Unsigned integers use fixed widths shown below and big-endian byte order.
- Collection counts and UTF-8 byte-string lengths use unsigned 64-bit big-endian integers.
- Strings are encoded as byte length followed by their exact UTF-8 bytes. No Unicode normalization or locale processing is performed.
- Optional strings and optional floats use one tag byte: 0 for absent, 1 followed by the value for present.
- All floating-point fields are encoded as their exact IEEE-754 binary64 bit patterns in big-endian order. Non-finite values are rejected by manifest validation. Positive and negative zero therefore have different v1 digests by design.
- SHA-256 fields represented in the Rust manifest as 64 hexadecimal characters are validated and then encoded as 32 raw digest bytes. Hexadecimal letter case does not change canonical identity.
- Enum tags are explicit numeric values below; Rust declaration order is not implicitly hashed.
- The encoder rejects canonical output above 64 MiB.
- Serde/JSON, Rust Debug/Display, memory layout, Hash, DefaultHasher, and map iteration order are not used to form canonical bytes.

## World field order

After the domain and canonical-format version, fields are encoded in this exact order:

1. Manifest schema version (u16).
2. Scenario ID (length-prefixed UTF-8).
3. World generation (u64).
4. Seed (u64).
5. Fixed step in nanoseconds (u64).
6. Three gravity components as binary64 bit patterns, in XYZ order.
7. Frame count and frames sorted by exact UTF-8 frame ID.
8. World fidelity tag (u8).
9. World-asset count and assets sorted by unique artifact ID. Duplicate world-asset IDs are rejected during manifest validation.
10. Solver count and solvers sorted by solver ID.
11. Sensor count and sensors sorted by sensor ID.
12. Platform count and platforms sorted by platform ID.

Sorting applies only to collections whose meaning is set membership rather than sequence order. Fixed numeric arrays retain their defined order. Per-platform actuators are sorted by actuator ID; a platform's sensor-ID membership list is sorted lexicographically. Duplicate identities, including duplicate world-asset artifact IDs, are rejected by manifest validation before encoding. World asset artifact IDs must be unique within the world_assets table; reuse the same identity through platform geometry/kinematics references rather than declaring duplicate rows. Across every ArtifactRef location (world_assets, platform geometry/kinematics, and sensor calibration), one nonblank artifact_id may not claim conflicting SHA-256 content digests; hexadecimal letter case is ignored. URI aliases are allowed when they claim the same content digest, and each URI remains included in the world-manifest commitment.

## Nested field grammars

### Artifact

1. Artifact ID string.
2. URI string.
3. 32 raw SHA-256 bytes.

### Coordinate frame

1. Frame ID string.
2. Optional parent-frame ID.
3. Transform-kind tag and payload:
   - 0: root; no payload.
   - 1: static; three translation binary64 values in metres, then four XYZW quaternion binary64 values.
   - 2: kinematic; provider artifact ID string, then provider frame ID string.

Transform convention is p_parent = R_parent_child * p_child + t_parent_child. This digest does not prove that an external kinematics artifact actually contains the named provider frame.

### Solver declaration

1. Solver ID string.
2. Physics-domain tag.
3. Implementation string.
4. Version string.
5. 32 raw build SHA-256 bytes.
6. 32 raw parameter SHA-256 bytes.
7. Output-contract version (u16).
8. Fidelity tag.

Physics-domain tags:

| Tag | Domain |
|---:|---|
| 0 | RigidBody |
| 1 | Terrain |
| 2 | MechanicalCutting |
| 3 | Acoustic |
| 4 | Thermal |
| 5 | Electromagnetic |
| 6 | ElectricalDischarge |
| 7 | Fluid |
| 8 | ContinuumMechanics |
| 9 | Fracture |
| 10 | SensorModel |

Fidelity tags: 0 Kinematic, 1 DeterministicReference, 2 ReducedOrder, 3 DomainSolver, 4 CoupledRegion.

### Sensor

1. Sensor ID string.
2. Modality tag.
3. Frame ID string.
4. Optional model-solver ID.
5. Optional calibration artifact (tag then Artifact, when present).
6. Sample rate binary64.
7. Latency nanoseconds (u64).
8. Jitter nanoseconds (u64).
9. Noise standard deviation binary64.
10. Noise unit string.
11. Dropout probability binary64.
12. Optional saturation minimum binary64.
13. Optional saturation maximum binary64.

Sensor-modality tags: 0 Camera, 1 Lidar, 2 Imu, 3 Encoder, 4 ForceTorque, 5 Vibration, 6 AcousticEmission, 7 Ultrasound, 8 ThermalImager, 9 Thermocouple, 10 Pressure, 11 Flow, 12 Gas, 13 Electrical.

### Platform

1. Platform ID string.
2. Platform-type tag.
3. Geometry artifact.
4. Kinematics artifact.
5. Base-frame ID string.
6. Mass in kilograms (binary64).
7. Control rate in hertz (binary64).
8. Controller-interface version (u16).
9. Actuator count and actuators sorted by actuator ID; each actuator encodes ID, minimum command binary64, maximum command binary64, and command-unit string.
10. Sensor-ID count and sensor IDs sorted lexicographically.

Platform-type tags: 0 Quadrotor, 1 Vehicle, 2 Humanoid, 3 Auv, 4 Helicopter, 5 Manipulator, 6 Surgical, 7 Orbital, 8 Quadruped, 9 Exoskeleton.

## Solver admission

The world-aware admission path verifies:

- Manifest structural validation.
- Request/result identity equality.
- Scenario ID and world generation equality.
- Recomputed manifest digest equality.
- Exact declared solver-profile equality, including implementation, version, build digest, parameter digest, output-contract version, domain, and fidelity.
- Every output frame exists in the manifest frame registry.

Digest mismatch does not get repaired by updating the generation. A legitimate world change requires the caller to prepare a new solver request against the new manifest digest and generation. The digest carried by a request is still a claim until recomputed and compared by the receiving world.

## Independent golden vector

The Rust unit tests and scripts/verify_world_manifest_v1_golden.py both target a fixed synthetic fixture:

- Canonical byte length: 1221.
- SHA-256: 136ddd6d2d42cd9d496c53c1141f5bafc576b6a8c1e5517dea80770171fe8f26.
- Reordering set-like tables leaves the digest unchanged.
- Changing the X gravity component from 0.0 to 0.25 changes the digest to cd86b0824a1eea6606455fef98c57cdb755a0db66d73137c0c4e130503fb9e3b.

The Python checker uses only hashlib and struct and is intentionally a pinned-fixture oracle rather than a second production encoder. Its execution can catch byte-grammar drift without requiring the Rust implementation to generate the expected digest.

## Versioning and non-claims

Any change to field order, primitive encoding, enum tags, ordering rules, digest normalization, or domain label requires a new canonical-format version. Existing v1 digests must never be reinterpreted under a later grammar.

V1 is content identity only. It does not establish authentic authorship, cryptographic authorization, physical validation, numerical verification, solver execution, run health, context-of-use suitability, or real-robot readiness. Those claims require separate evidence and verification paths.
