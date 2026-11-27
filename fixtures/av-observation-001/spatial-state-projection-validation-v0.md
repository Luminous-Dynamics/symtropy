# Spatial State Projection v0 — independent validation protocol

**Status:** adapter/evaluator contract and test plan. Not implemented as a Rust validator yet.

This protocol validates the candidate-visible spatial-state-projection-v0 object without turning its claims into truth or replacing the owning Symthaea spatial types.

## 1. Canonical contracts that the adapter must preserve

The projection composes with Symthaea spatial owners, not a newly invented coordinate system:

- **#498:** ReferenceFrameId = namespace + local ID + generation; ClockDomainId uses the same generation-bound identity discipline. Pose3 = T_reference_from_local, right-handed metric Cartesian, translation is the reference-expressed vector from the reference origin to the local origin, and quaternion order is **[w,x,y,z]**.
- **#498 PoseUncertainty V1:** three independent translation marginal sigmas in metres and three independent left-multiplicative SO(3) tangent sigmas in radians. This is not a full correlated SE(3) covariance.
- **#611:** correlated pose uncertainty and transform-constraint dependency semantics are a distinct later contract. Do not reinterpret V1 marginals as a full covariance.
- **#621:** cross-domain clock arithmetic is invalid without a qualified offset/drift transform, its exact clock generations and bounded validity interval. Inversion/composition are derived views, not independent measurements.
- **#632:** sensor extrinsics are versioned uncertain calibration state, not timeless configured matrices. Calibration state must be current/applicable and have source lineage.
- **#633:** modalities are not generic confidence votes; same sensor ancestry, pose/map prior or calibration root is shared evidence, not independent corroboration.
- **#625:** pose/map updates that share observations must preserve joint/shared ancestry. A pose marginal and map marginal from the same sensor observations must not be fused as independent evidence.

Primary Symtropy world-frame contract: PR #1589's CoordinateFrameV1 / FrameTransformV1 defines a static child-to-parent mapping
p_parent = R_parent_child * p_child + t_parent_child and stores its quaternion as **[x,y,z,w]**. This has the same directional meaning as T_parent_from_child, but a different quaternion wire order.

### Explicit adapter rule

The Symthaea projection remains canonical **WXYZ**. When adapting a transform to Symtropy FrameTransformV1::Static, convert:

    Symthaea: [w, x, y, z]
    Symtropy: [x, y, z, w]

Do not change translation, endpoints or transform direction during this reorder. The runtime adapter must test non-identity rotations; identity quaternion alone cannot detect an ordering bug.

Symtropy's hidden WorldManifestV1.frames, world_generation and evaluator truth are not copied into the candidate projection. Candidate frame IDs and evaluator frame IDs are distinct namespaces. Alignment between a candidate map and truth is an evaluator operation and must not leak into candidate-visible inputs.

## 2. JSON/schema validation

Validate the checked-in schema and example with a full Draft 2020-12 implementation in development tooling. The schema closes unknown object fields and checks basic shape, enums, digest syntax, lengths and required fields. The current local structural checks were useful smoke tests only; they were **not** a full JSON Schema implementation.

Schema-only checks are insufficient for:
- start_tick < end_tick for interval support and start_tick == end_tick for instant support;
- uniqueness and resolution of frame/clock/observation/hypothesis/claim IDs;
- reference namespace/generation equality;
- transform endpoint existence and units;
- unit quaternion normalization / finite values / signed-zero canonicalization;
- calibration and clock transform freshness/validity;
- transform cycles, pose graph residuals and correlated uncertainty;
- parent projection cycles and evidence-root dependence;
- map topology and admissibility of direct adjacency;
- candidate/evaluator process separation.

The production validator should be Rust-first and live at the existing adapter/contract boundary. Do not add a parallel Python production service or a second evidence/clock/calibration registry.

## 3. Required runtime validation order

### V0 — closed-world envelope

1. Parse the exact schema version; reject unknown versions and unexpected fields.
2. Enforce byte/count/ID limits before allocating unbounded collections.
3. Verify all lowercase SHA-256 syntax and recompute digests from canonical payload bytes where the artifact is locally available.
4. Verify the evidence-prefix digest matches the exact prefix offered to the producer and producer revision digest matches the admitted executable/model bundle.
5. Verify projection creation time uses its own ClockDomainId; do not compare its integer tick to sensor capture ticks without a #621 transform.

### V1 — identity and evidence references

1. Treat frame and clock identity as the complete triple (namespace, local_id, generation); never join by local ID alone.
2. Verify every observation ref resolves to an admitted sensor artifact and exact immutable artifact digest.
3. Verify capture support and known-at time are distinct fields and their clock domains/generations are explicit.
4. For interval support require start_tick < end_tick; for instant support require equality.
5. For cross-sensor ordering/fusion, resolve the referenced #621 clock transforms and enforce their bounded source interval, target generation, offset/drift uncertainty and currentness. Otherwise mark temporal relation incomparable.
6. A clock-transform digest or its inverse is not an additional independent synchronization observation.

### V2 — coordinate frames and poses

1. Resolve every frame_ref to a unique candidate-local generation. Reject duplicates and stale generations.
2. Resolve optional parent-frame references. The frame-parent registry must have one declared root per candidate map and no parent cycles.
3. Pose endpoints must exist, be distinct, and use metric frames; a pixel frame cannot be composed using a metric Pose3.
4. Interpret reference_frame_ref as reference and local_frame_ref as local, exactly T_reference_from_local.
5. Validate translation and quaternion components are finite.
6. Validate quaternion by the owning #498 scale-safe normalization and canonicalize signed zero; do not normalize malformed values silently without recording a derived correction.
7. Validate the declared #498 independent marginal uncertainty profile: nonnegative finite translation sigmas (metres) and rotation tangent sigmas (radians). Never call this a correlated covariance.
8. Resolve calibration_state_digest through #632 and reject stale, invalidated, out-of-envelope or unknown calibration state for geometry claims that depend on it.
9. For conversion to Symtropy FrameTransformV1, reorder WXYZ to XYZW explicitly, preserve p_parent = R_parent_child p_child + t, and test the result against canonical reference vectors.

A pose graph may legitimately contain cycles. Do not confuse it with the frame-parent registry tree. For each cycle, check the composed SE(3) residual against a preregistered uncertainty-aware bound. An inconsistent edge is retained as a conflict/rejected constraint; it must not be silently optimized away or counted as a valid loop closure.

### V3 — entity and topology claims

1. Every entity hypothesis ID is unique within its declared candidate namespace and all source observation refs resolve.
2. A position_m is a MetricPoint3 in its named frame_ref; never treat it as a displacement or infer an omitted frame.
3. Position sigmas are per-axis marginal diagnostics, not a full correlated point covariance.
4. Relation subject/object IDs resolve to candidate hypotheses. The current v0 pairwise relation types require both endpoints.
5. observed_traversal and direct_adjacency are distinct predicates. Sequential co-occurrence or an observation gap does not prove a doorway/edge.
6. An inferred direct-adjacency claim needs a declared derivation rule and source evidence. Preserve proposed/contested state until its own evidence rule passes.
7. Relation cycles are allowed when the topology warrants them; validate edge semantics, not graph acyclicity.
8. Every unresolved constraint retains at least two distinct candidate alternatives and the exact observations supporting the ambiguity. A residual index is never itself a new observation.

### V4 — lineage and no double counting

1. Resolve all parent_projection_digests; reject missing parents and cycles.
2. Parent projections are derived memory/state, not fresh independent sensor evidence.
3. Overlapping observation roots, shared calibration, clock, pose, map or model roots remain dependent unless an owning qualified process establishes an independence bound.
4. A pose derived from a map and a map updated from that pose must retain joint/shared ancestry (#625). Do not feed posterior marginals back as independent factors.
5. Multiple renderings, embeddings, HDC encodings, patches, optical-flow tracks or residuals from one raw observation do not multiply the number of independent observations.
6. supersedes_projection_digest is revision history, not proof that the new projection is more correct. Verify the predecessor digest and change set.

## 4. Mandatory negative and metamorphic tests

- [ ] WXYZ versus XYZW non-identity rotation round-trip through the explicit adapter.
- [ ] Inverse pose is a derived view and does not create a second source factor.
- [ ] Wrong transform direction / swapped endpoints is rejected.
- [ ] Same local frame ID with a different namespace or generation does not resolve.
- [ ] Frame-parent cycle is rejected; valid pose-graph cycle is evaluated by residual, not rejected solely for being a cycle.
- [ ] Non-finite quaternion/translation and zero quaternion are rejected.
- [ ] Signed zero is canonicalized according to #498 before content digesting.
- [ ] Clock reset changes generation; equal numeric ticks from different domains are incomparable without #621.
- [ ] Interval endpoints are half-open and correctly ordered; instant and interval supports differ.
- [ ] Stale calibration digest / generation cannot support a current metric geometry claim.
- [ ] Same observation supports both pose and map: no independent corroboration or double-counted confidence.
- [ ] Parent projection dependency is not misreported as a fresh source.
- [ ] Late correction revises current map state while preserving earlier known-at projection state.
- [ ] Observation sequence alone does not fabricate adjacency.
- [ ] False loop closure with similar texture but incompatible route/pose is retained as contested/rejected.
- [ ] Candidate/evaluator witness IDs or world coordinates injected into projection are rejected.
- [ ] Reordering object arrays and renaming candidate-local IDs leaves substantive evaluator scores invariant.
- [ ] Unknown/underconstrained geometry remains Unknown; no fabricated metric scale.

## 5. Independent reference vectors

Keep deterministic test vectors for:
- identity pose and translated pose;
- non-identity WXYZ quaternion with a known transformed point;
- explicit WXYZ→XYZW adapter conversion with the same transformed point;
- inverse rotation and endpoint swap;
- frame generation mismatch;
- known and unknown clock transform validity;
- a consistent triangle pose cycle and one deliberately corrupted edge;
- a calibration revision that invalidates derived geometry;
- a map update sharing one observation root with the pose update;
- same topology with different candidate labels/order.

The evaluator should use an independently implemented transform oracle for the test vectors; do not compare the adapter against itself.

## 6. Promotion gates

The projection is promoted from a JSON interchange draft only after:
1. full Draft 2020-12 schema validation in CI;
2. Rust runtime validation for all V0–V4 checks;
3. exact-head tests of the WXYZ/XYZW adapter and SE(3) vectors;
4. independent evaluator agreement on every deterministic fixture;
5. evaluator truth is process/filesystem isolated from the candidate;
6. the candidate map output beats no-map and frozen-general-backbone baselines on held-out spatial cases at matched evidence/compute budgets.

Until then it is a candidate assertion schema, not a verified world model, SLAM result, or action-authorizing map.


## 7. Checked-in schema refinements and current local checks

The v0 schema now additionally requires:
- a typed sensor frame reference for video, audio, RGB-D, stereo, IMU and LiDAR observations;
- typed projection creation time (ClockDomainId + tick), separate from each source capture support and known-at time;
- parent projection digests and #621 clock-transform digests as lineage references;
- both subject and object candidate hypotheses for pairwise relation claims;
- a frame reference whenever an entity hypothesis emits metric position.

The candidate projection example currently passes local structural checks for the schema keywords used. Seven adversarial mutations were rejected: injected evaluator truth, prefixed/malformed digest, missing candidate-only frame marker, source-free inferred entity, malformed quaternion arity, missing sensor frame, and metric position without a frame reference. This is a targeted structural check, **not full Draft 2020-12 conformance** and not the runtime validation described above.

The checked-in [transform vector file](./spatial-state-projection-v0.transform-vectors.json) provides:
- a 90° Z rotation with non-zero translation;
- expected Symthaea WXYZ and Symtropy XYZW quaternion forms;
- transformed point and inverse pose expectations;
- six negative adapter/lineage cases.

The vector file is a test fixture specification. No Rust adapter test has run yet. The validation work should implement an independent reference transform, not merely call the same conversion function for both expected and actual values.
