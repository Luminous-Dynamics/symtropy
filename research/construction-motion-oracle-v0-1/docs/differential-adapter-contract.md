# Differential Adapter Contract v0.1

## Purpose

Map one frozen construction trajectory/obstacle case into an external collision checker and return a result that can be compared with the independent analytic oracle.

Adapters are observations only. They do not reserve space, issue work permits, command actuators or authorize robot motion.

## Required run identity

Every adapter result must bind:

| Field | Meaning |
|---|---|
| schema_version | Exact adapter-result schema version |
| case_id / case_sha256 | Frozen corpus record identity |
| robot_model_sha256 | Exact body/link geometry and kinematic model |
| tool_sha256 / payload_sha256 | Exact attached geometry |
| obstacle_set_sha256 | Exact obstacle geometry |
| frame_id / transform_sha256 | Exact comparison frame and transforms |
| trajectory_sha256 | Exact path/state sequence |
| interpolation_profile | Joint interpolation, timing and subdivision policy |
| collision_matrix_sha256 | Allowed-collision matrix / contact exclusions |
| backend_name / backend_version / build_sha | Exact backend implementation |
| query_mode | DISCRETE, LVS_DISCRETE, LVS_CONTINUOUS or CONTINUOUS |
| margin_profile_sha256 | Padding, contact tolerance and uncertainty policy |
| result_status | PASS, FAIL, UNRESOLVED or NOT_RUN |
| collision_found | Optional backend observation; never substituted for missing output |
| minimum_distance_mm | Optional backend metric with documented semantics |
| latency_ns | Monotonic measured elapsed duration |
| memory_bytes | Measured peak/allocated memory where supported |
| raw_receipt_sha256 | Digest of raw adapter output and logs |

Missing optional metrics remain absent. A successful process exit without a valid result is UNRESOLVED. Missing backend, unsupported geometry or incompatible semantics is NOT_RUN, not PASS. The machine-readable result contract is schema/adapter-result-v0.1.schema.json.

## MoveIt adapter

At minimum capture:
- ROS distribution and exact MoveIt commit/version;
- FCL or Bullet detector identity;
- continuous robot-world query mode and both endpoint RobotState digests;
- exact runtime capability probe and raw backend result proving the selected overload actually executed.

**Backend-specific caution:** the abstract `CollisionEnv` API documents continuous robot-world checking between two RobotState values, while the currently inspected MoveIt `CollisionEnvFCL` implementation logs `Continuous collision not implemented` / `Not implemented` for those overloads. The MoveIt Bullet collision-checker tutorial separately documents Bullet's continuous collision capabilities. Therefore:
- treat MoveIt FCL continuous robot-world as `NOT_RUN` / unsupported for the inspected implementation unless a later exact commit proves otherwise;
- do not infer FCL CCD from the abstract API;
- test the actual Bullet CCD path separately, pinning exact MoveIt/Bullet commits and interpolation semantics;
- a call that logs an error, returns an untouched result, or cannot prove execution is `UNRESOLVED`, never a clear result.
- AllowedCollisionMatrix digest;
- robot-world versus self-collision scope;
- padding/scaling and transform state;
- model, tool, payload and obstacle digests.

MoveIt's abstract continuous robot-world checking API describes a query between two robot states, with allowed collisions ignored and self-collision excluded from that call. Backend implementation is decisive: the currently inspected FCL implementation leaves the continuous overload unimplemented, while the MoveIt Bullet tutorial documents CCD support. Pin the actual backend commit and prove the selected path executed. Self-collision requires a separately declared query. Do not report one as the other.

## Tesseract adapter

At minimum capture:
- exact Tesseract commit/version and contact-manager plugin;
- evaluator type: DISCRETE, LVS_DISCRETE, LVS_CONTINUOUS or CONTINUOUS;
- longest-valid-segment limit for LVS modes;
- cast/interpolation profile and link geometry identities;
- environment revision and active collision-object set;
- continuous-cast geometry construction, including convex-hull settings and padding.

The documented evaluator distinctions are material:
- DISCRETE checks only supplied states.
- LVS_DISCRETE adds interpolated states no farther apart than the configured longest-valid-segment length, but remains a sequence of discrete checks; a sufficiently thin obstacle can still lie between samples.
- LVS_CONTINUOUS casts each interpolated segment. The swept link is approximated by the convex hull of its endpoint poses; shorter segments can tighten this approximation to a curved link path.
- CONTINUOUS casts a whole trajectory segment in one operation with the same convex-hull approximation over the larger segment.

Do not compare LVS_CONTINUOUS with one-shot CONTINUOUS without recording subdivision and cast geometry. A convex-hull cast can be conservative relative to the true curved sweep; report false-positive burden rather than silently treating every candidate as physical contact.

## Runtime and concurrency cautions

The MoveIt Bullet documentation notes that its collision managers contain mutable state and the current implementation is not thread-safe. A benchmark must serialize access or give each worker an isolated manager; never scale throughput by concurrently sharing a mutable manager. Robot-world continuous checking and self-collision remain separate query scopes.

FCL itself advertises continuous collision detection in its library API. That does not prove a particular MoveIt plugin overload implements it: the exact MoveIt FCL implementation inspected above currently reports its continuous overload as not implemented. Record library capability and integration-plugin capability separately.

## Published performance claims

Tesseract's documentation publishes Bullet-versus-FCL speed ratios for contact, penetration and distance modes (including FIRST/CLOSEST/ALL variants). Treat those numbers as a backend-specific published reference, not a portable baseline: the exact robot/obstacle models, hardware, compiler, query distribution, collision-manager configuration and measurement method must be reproduced before making a comparative claim. Our formula microbenchmarks are not in the same workload class and must never be compared numerically with those ratios.

## Matched comparison rules

1. Same case digest, link/tool/payload geometry, obstacle geometry, frame, transforms and collision matrix.
2. Same trajectory timing and declared interpolation semantics. If a backend cannot consume the same interpolation, mark the case semantically unmatched and do not pool the result.
3. Keep robot-world and self-collision separate.
4. Record exact geometry support. Meshes approximated to convex hulls, spheres or padding must be declared as a changed geometry profile.
5. Report outcomes per case and per backend; preserve false-safe outcomes and unresolved cases.
6. Separate endpoint-only misses, discretization misses, continuous-cast results, envelope broad-phase candidates and analytical truth.
7. Do not count a broad-phase overlap as a confirmed physical collision.
8. Do not use vendor-reported confidence as independent uncertainty qualification.

## Required metrics

- TP / FP / FN / TN against analytical truth for supported analytic cases;
- abstention / NOT_RUN / malformed output counts;
- physical collision outcomes versus coordination-envelope conflicts;
- envelope volume inflation versus analytical swept AABB;
- minimum-distance error, where semantics are matched;
- p50 / p95 / p99 latency, throughput, memory and allocation count;
- corpus case-family and geometry coverage.

No aggregate score may hide a false negative, a missing case family or a high abstention rate.
