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

Missing optional metrics remain absent. A successful process exit without a valid result is UNRESOLVED. Missing backend, unsupported geometry or incompatible semantics is NOT_RUN, not PASS.

## MoveIt adapter

At minimum capture:
- ROS distribution and exact MoveIt commit/version;
- FCL or Bullet detector identity;
- continuous robot-world query mode and both endpoint RobotState digests;
- AllowedCollisionMatrix digest;
- robot-world versus self-collision scope;
- padding/scaling and transform state;
- model, tool, payload and obstacle digests.

MoveIt's continuous robot-world checking API documents checking between two robot states, ignoring allowed collisions and not performing self-collision checking in that call. Self-collision requires a separately declared query. Do not report one as the other.

## Tesseract adapter

At minimum capture:
- exact Tesseract commit/version and contact-manager plugin;
- evaluator type: DISCRETE, LVS_DISCRETE, LVS_CONTINUOUS or CONTINUOUS;
- longest-valid-segment limit for LVS modes;
- cast/interpolation profile and link geometry identities;
- environment revision and active collision-object set.

Do not compare LVS_CONTINUOUS with one-shot CONTINUOUS without recording their different segment subdivision.

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
