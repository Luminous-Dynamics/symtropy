# Revision-bound contact freshness protocol

Date: 2026-11-26

Status: research prototype; not yet connected to the live RigidBody contact manager.

## 1. Paradigm shift: three clocks, not one dirty bit

A collision pipeline has at least three independently changing state domains:

1. **Pair membership** — whether two colliders are still eligible broadphase candidates.
2. **Contact geometry** — whether the cached manifold still describes the current shapes and transforms.
3. **Solver activation and warm start** — whether cached impulses remain useful and whether the associated island must run again.

Treating all three as one dirty bit causes unnecessary work and loses useful state. The protocol makes three separate decisions:

- reuse or refresh manifold geometry;
- reuse or refresh warm-start state;
- reactivate the solver island.

The most important hypothesis is that a **wake-only event must not automatically invalidate geometric contact or warm-start data**. A sleeping pair can wake while its endpoint transforms, shapes, filter eligibility and narrowphase configuration remain identical. In that case, reactivate the island but retain the manifold and its warm-start history. This must be tested against real engine wake/island semantics; it is not yet a measured performance result.

## 2. Prototype API

The first Rust slice is exported from `src/contact_freshness.rs`:

- `EndpointRevisions`: proxy ID and generation; transform, shape, eligibility, motion-class, material and wake-epoch revisions.
- `GlobalContactRevisions`: world and collision-policy revisions; SHA-256 digests for narrowphase, solver, and contact-response/callback policy.
- `ContactFreshnessStamp`: canonical pair plus the exact endpoint/global tuple under which a manifold was last produced.
- `ContactReuseDecision`: independent `geometry`, `warm_start`, `pair_eligibility_recheck_required` and `solver_activation_required` outputs.
- `plan_contact_lifecycle`: translates canonical, disjoint pair-event streams into deterministic `Added`, `Removed` and `Revalidated` contact actions. New and revalidated pairs require current endpoint revision records; a revalidated pair without a stamp requires a full contact build; an added pair with an orphaned prior stamp is rejected; a stamped revalidation with no typed revision reason is rejected as unexplained. New pairs also request a collision-eligibility recheck because this AABB-only layer does not prove the engine's filter policy.
- `ContactFreshnessError` / `ContactLifecycleError`: identity mismatch, noncanonical or conflicting events, missing endpoint data and monotonic-revision violations. Regressions fail closed.
- `ContactFreshnessStamp::commit_transition`: refuses to advance the stored stamp until all assessment-required geometry refresh, warm-start reset, pair-eligibility recheck and solver activation actions are reported complete. It prevents both premature freshness claims and repeated invalidation after completed warm-start-only work.

The stamp must be written by the contact-producing side only after a successful narrowphase/contact update. The broadphase may request revalidation; it must not certify that its own contact is fresh.

### Revision-to-action matrix

| Changed input | Recompute manifold geometry | Reuse warm start | Recheck pair eligibility | Reactivate solver |
|---|---:|---:|---:|---:|
| transform revision | yes | no | no, if pair delta already qualified current bounds | only if wake epoch also advanced |
| shape/scale revision | yes | no | broadphase AABB/pair delta must also be current | only if wake epoch also advanced |
| proxy generation | yes; new identity | no | yes, reconcile the new residency | new body/contact lifecycle |
| endpoint eligibility revision | yes / drop if disallowed | no | yes | if admitted and awake |
| motion-class revision | yes | no | yes | according to body/island state |
| global collision-policy revision | yes / drop if disallowed | no | yes, global safe reconciliation unless a complete scoped journal exists | according to body/island state |
| narrowphase config digest | yes | no | no, unless config changes eligibility | no |
| material revision | no geometric rebuild required | reset affected warm-start state | no | no |
| per-endpoint response-state revision | no geometric rebuild required | reset affected warm-start state | no | no |
| global response-policy digest | no geometric rebuild required | reset affected warm-start state | no | no |
| solver config digest | no geometric rebuild required | reset affected warm-start state | no | no |
| wake epoch only | no | **yes** | no | **yes** |

The matrix is conservative. A production integration may retain more state only when it can prove the relevant invariants. A mismatch never means “unchanged.”

### Monotonicity and identity

- Per-domain revisions are monotonic within one proxy generation.
- A generation change is a new residency identity; never compare its reset counters with the old generation or alias the old contact.
- World revisions may advance while an unaffected pair remains untouched. Reuse depends on unchanged endpoint/global revisions and a complete authoritative mutation journal, not rewriting every pair stamp each frame.
- Any endpoint/global mutation must advance the enclosing world revision. A changed endpoint or policy tuple under an unchanged world revision is rejected.
- A lower world, policy or same-generation endpoint revision is an error and requires reconciliation/quarantine.
- The caller must bind the revision tuple to the authoritative world mutation source. This crate cannot prove journal provenance.

## 3. Transition protocol to implement

For each world tick:

1. The world owns the monotonic world revision and mutation journal.
2. The broadphase resolves current pair membership and emits `added`, `removed` and `revalidated` pair events.
3. The independent verifier checks journal completeness against an authoritative snapshot or replay source, including proxy generations and global policy revisions.
4. For every retained or new pair, the contact manager compares its last committed stamp with the current exact revision tuple.
5. A typed decision selects geometry refresh, warm-start retention, eligibility recheck and island activation independently.
6. The contact manager commits a new stamp only after narrowphase succeeds. Failed narrowphase work leaves the old stamp in place but unusable for a mismatched revision.
7. A deterministic per-frame receipt binds previous/current world revisions, input digests, pair events, freshness decisions, contact lifecycle counters and verifier result.

Unknown journal outcome, missing revision, generation alias, revision regression or verifier disagreement must not be converted into an empty delta. The safe path is a full scoped reconciliation or quarantine, with no contact-state reuse across the uncertainty.

## 4. Evidence receipt contract

The next receipt schema should contain:

- schema version and verifier version;
- exact Symtropy commit, build/toolchain and platform identity;
- previous/current world revisions and world mutation-source identity;
- proxy-generation, transform/AABB, shape/config, material and filter-policy digests;
- journal sequence range, canonical journal digest and independent completeness result;
- before/after pair-relation digests and canonical `added`, `removed`, `revalidated` sets;
- per-pair endpoint revision tuples, typed freshness reasons and selected reuse decisions;
- manifold creates/refreshes/reuses/deletes, warm-start reuses/resets, solver wake activations and island merges/splits;
- narrowphase invocation count, solver-row denominator, CCD candidates and fallback counts;
- raw per-frame timing and retained-memory/allocation measurements.

Receipts are evidence, not authority. The verifier must be logically separate from the producer and must consume immutable inputs. A self-reported `journal_complete=true` is insufficient.

## 5. Qualification and adversarial benchmark programme

Keep the current AABB tournament as the pair-membership lane. Add a contact-lifecycle lane with frozen temporal input and controlled mutation classes:

1. sleeping idle frames with unchanged pair relation;
2. one-body wake with unchanged transforms/AABBs;
3. wake-island cascade and partial island split/merge;
4. transform drift while pair membership remains active;
5. shape/scale/margin change with unchanged body transform;
6. collision filter/layer toggle with unchanged AABBs;
7. motion-class/body-type migration;
8. contact material/friction change;
9. narrowphase and solver configuration changes;
10. removal/reinsertion with recycled slot and incremented generation;
11. journal gap, duplicate, omission, rollback and out-of-order delivery;
12. teleports, CCD/high-speed motion and large heterogeneous colliders.

Run negative controls for every invalidation class: mutate one revision while deliberately holding all other data constant; then mutate the geometry without advancing the declared revision and require the independent snapshot auditor to reject the journal. Include generation-reuse and stale-stamp injection tests.

### Metrics and denominators

Correctness:
- stale manifold reuse count (must be zero);
- stale warm-start reuse count (must be zero);
- pair false negatives, false positives and missing/duplicate events;
- generation aliases and unaccounted revision changes (must be zero);
- exact eligibility and revision coverage;
- deterministic replay agreement.

Work:
- pair candidates and contact pairs refreshed;
- manifold geometry refresh/reuse ratio;
- warm-start retention/reset ratio;
- solver-island activation count, merge/split count and wake-to-solve latency;
- journal producer, audit and verifier cost;
- broadphase index maintenance and safe-fallback cost;
- narrowphase invocations, contact points and solver rows.

Performance:
- p50/p95/p99 broadphase + contact maintenance + complete physics tick;
- worst-frame spikes during wake storms, insertion/removal and rebuilds;
- allocations, retained pair/contact bytes and peak RSS;
- solver iteration/convergence and bounded trajectory/energy deviation against a full-refresh reference;
- CPU and GPU lanes separately, with synchronization and transfer cost included.

### Prospective gates

These are goals, not results:

- zero stale contact or warm-start reuse under every injected mutation and replay case;
- exact pair and contact lifecycle parity with an independent verifier;
- at least 20% lower p95 broadphase-plus-contact-maintenance cost on two held-out sleeping/mixed families after journal production, verification, index maintenance and contact work are included;
- no more than 5% p95 end-to-end tick regression on any declared family;
- no worse solver convergence or physically meaningful error than the full-refresh reference under the declared tolerance;
- publish raw receipts and worst-case distributions, not just aggregate means.

Do not optimize only the pair-delta producer. The denominator is the real contact-manager and physics-tick work.

## 6. Industry positioning and genuine differentiation

This proposal must be evaluated against capabilities that already exist:

- **MuJoCo Warp 3.17** already has an incremental sleeping pass that restricts emitted pairs to newly awakened bodies; its SAP variants include tile and segmented sorts. This makes wake-aware pair emission a baseline comparator, not a novelty claim. See the [MuJoCo Warp API](https://mujoco.readthedocs.io/en/3.17.0/mjwarp/api.html).
- **Bullet DBVT** has separate dynamic and fixed trees plus a persistent overlapping-pair cache. Compare contact/manifold lifetime and warm-start behavior, not just its pair-cache header. See [btDbvtBroadphase](https://github.com/bulletphysics/bullet3/blob/master/src/BulletCollision/BroadphaseCollision/btDbvtBroadphase.h).
- **Jolt 5.3** uses broadphase layers and a quadtree; moving bodies expand ancestors concurrently and a tighter tree is rebuilt in the background. Compare layered populations, rebuild debt and concurrent update consistency. See [Jolt 5.3](https://jrouwe.github.io/JoltPhysicsDocs/5.3.0/).
- **PhysX 5.5** provides SAP, MBP, ABP, PABP and GPU options, with different behavior across sleeping, all-moving and insertion/removal-heavy workloads. Compare a workload portfolio, not one global winner. See [PhysX 5.5 collision documentation](https://nvidia-omniverse.github.io/PhysX/physx/5.5.0/docs/RigidBodyCollision.html).
- **Newton** exposes NxN, SAP and explicit compatible-pair modes; explicit pairs have topology and memory constraints, while SAP is the large/coherent-scene alternative. See [Newton collision documentation](https://newton-physics.github.io/newton/latest/concepts/collisions.html).
- **Box2D** publishes useful dynamic-tree query and structural statistics; use node/leaf visits and tree-quality metrics in addition to time. See [Box2D dynamic tree](https://box2d.org/documentation/group__tree.html).

The intended contribution is not a new generic AABB tree or a claim that pair caches are new. It is a **revision-qualified contact-state protocol** with explicit geometry, warm-start and activation validity; exact invalidation reasons; generation-safe identity; independent replay receipts; and full downstream cost accounting. That is the hypothesis to falsify.

## 7. Current evidence boundary

Implemented in this branch: typed revision structures, generation-aware identity checks, monotonicity errors including world-revision binding, separate geometry/warm-start/activation decisions, manifold and warm-start payload revisions/digests, a post-action stamp commit barrier, deterministic candidate-pair-to-contact-work action planning, fail-closed unexplained-revalidation detection, an independently implemented action-plan verifier, immutable before/after receipt replay with post-stamp payload checks, and authored unit tests for mutation and event classes. The verifier is a separate algorithm boundary in the same crate, not yet a separately deployed trust anchor.

Not yet implemented or established: authoritative live world mutation journal integration; canonical serialization and independent re-hashing of actual contact/manifold/impulse payload bytes; separate verifier process/trust anchor; live contact-manager or solver integration; matched external-engine runs; exact-head Rust formatting, tests, Clippy or release timings.

No speedup, correctness qualification or SOTA superiority claim is made until these gaps close.

## 8. Immutable transition receipt and independent replay

### 8.1 Contact payload identity and commit discipline

A revision vector alone says which inputs were current; it does not identify the actual cached contact data. The stamp now also carries `ContactPayloadRevisions`:

- manifold revision + SHA-256 digest of the canonical manifold payload;
- warm-start revision + SHA-256 digest of the canonical solver warm-start payload.

Within an unchanged proxy generation:
- If geometry refresh is required, the committed manifold revision must advance.
- If geometry remains reusable, the manifold revision and digest must remain identical.
- If warm-start reset is required, its revision must advance.
- If warm-start is reusable, its revision and digest must remain identical.
- A new proxy generation starts a new counter lifetime, but both digests must still be present.

The independent receipt verifier checks these revision/digest relations between prior and post stamps. The hash values are supplied by the contact-state snapshot adapter: this crate does not serialize engine-private manifolds or impulses and does not recompute the hashes from payload bytes. Therefore a production verifier must independently read/hash the actual contact and warm-start buffers; it must not trust digest strings reported by the broadphase/contact producer.

The caller-supplied completion flags in `commit_transition` are not an independent attestation. They are a local commit barrier. Production assurance still requires a world/contact-manager-owned action receipt and an independent observer that verifies the resulting payload bytes and solver activation.


The branch now exports `contact_freshness_verifier` with two layers:

- `verify_contact_lifecycle_plan` independently reconstructs the expected `Added`, `Removed` and `Revalidated` actions from a supplied pair-delta frame, prior contact stamps, and current revision records. It does not call the producer planner or `ContactFreshnessStamp::assess`; the audit logic is separately implemented.
- `verify_contact_lifecycle_receipt` starts from immutable before/after pair-relation snapshots, before/after endpoint-revision maps, prior contact stamps, and the producer's plan. It derives additions/removals by set difference and derives retained-pair revalidation from changed endpoint revisions, missing prior stamps, and global collision/narrowphase/solver configuration changes. It then invokes the separate action audit.

Receipt checks include:
- strictly advancing world-step revision and monotonic collision-policy revision;
- post-transition contact stamps must match current endpoint/global revisions and refer only to current broadphase pairs;
- an existing contact whose geometry and eligibility remain reusable must have a committed post-transition stamp; a wake-only or material/solver-only transition cannot silently drop it;
- canonical, sorted, unique pair relations;
- endpoint-map key/identity consistency and endpoint presence for every pair;
- prior contact stamps must belong to the prior pair relation and exactly match the prior endpoint snapshot;
- prior stamps must not carry stale collision-policy or narrowphase/solver configuration digests;
- endpoint revision monotonicity, with counter reset allowed only after proxy generation advances;
- exact additions, removals and retained-pair revalidation;
- exact contact action order and content, including geometry reuse, warm-start reuse, eligibility recheck and solver activation.

A receipt replay is not yet a cryptographic proof and is not yet a separately deployed trust anchor. The verifier is a distinct implementation boundary in the same crate. Production assurance should build/run it separately against immutable world snapshots and a mutation log, and should sign/digest canonical serialized receipts with an established cryptographic implementation.

The pair-relation snapshots are inputs to this contact-lifecycle verifier; they must already be independently qualified by the geometric broadphase oracle. This verifier does not derive collision geometry from shapes or AABBs, and it must not be described as proving geometric overlap.

### Receipt qualification tests

The verifier includes tests for:
- accepting a valid wake-only revalidation while preserving geometry and warm starts;
- rejecting an omitted wake revalidation;
- rejecting a non-advancing world revision;
- rejecting a prior stamp inconsistent with its authoritative endpoint snapshot;
- rejecting missing pair endpoints;
- reconstructing addition and removal events from before/after relation snapshots;
- rejecting omitted post-stamps after wake-only transitions and stale post-stamps;
- rejecting a missing global solver-config revalidation even when a replacement post-stamp is supplied.
- rejecting payload digest mutation when geometry or warm-start state should have been retained;
- rejecting missing payload digests;
- rejecting shape refresh without manifold revision advancement;
- rejecting material refresh without warm-start revision advancement.

The benchmark follow-on should measure receipt construction, independent replay CPU, serialization size, verifier memory and the ratio of safe-fallback transitions. These costs must be included in the end-to-end tick and in any speed claims.

### Contact-response and callback semantics are not geometry

`GlobalContactRevisions` now has `contact_response_policy_sha256`, a separate revision for material/contact-response and callback policy. A change to this digest keeps manifold geometry reusable but invalidates warm-start state, with a typed `ContactResponsePolicyChanged` reason. The independent action/receipt verifier reconstructs the same result; receipt tests cover both accepted warm-start reset and omitted revalidation.

This revision is a version/digest of the response policy, not a license to skip current-step callback semantics. Geometry reuse must not suppress any per-step contact-response work required by the engine, including current material/friction modulation, impulse modulation, collision notifications, trauma/application hooks, or work recording. Event callbacks need their own event identity and delivery/commit receipts where exactly-once or replay semantics matter; a geometry lease is not a callback lease.

The contact-response digest is currently caller-supplied metadata just like the other config digests. EndpointRevisions.response_state separately tracks per-body dynamic values consumed by that policy—for example, the state used by consciousness-dependent friction or impulse modulation. A changed response-state revision preserves geometry but invalidates warm starts; a regression is rejected. Production must bind both the global policy digest and endpoint response-state revision to authoritative snapshots.

In the live Symtropy PhysicsCallback interface, modulate_impulse and friction_multiplier are evaluated inside contact resolution, while on_collision and apply_trauma are emitted after a nontrivial normal impulse. record_dissipation is emitted for frictional dissipation. Geometry reuse must not bypass these current-step semantic paths. A per-contact callback/response revision is not proof that a callback has been delivered; event delivery needs separate identity/commit evidence when replay or exactly-once semantics are required. The #7699 observer must independently qualify the resulting contact payload bytes.

### Contact callback and event delivery receipts

The current model tracks global response-policy and per-endpoint response-state revisions, but those do not prove that callbacks were evaluated or events delivered. Symtropy's live PhysicsCallback includes per-contact friction/impulse modulation and collision/trauma/dissipation outputs. Geometry reuse must leave those semantics current.

The next event-identity and replayable-delivery increment is tracked in [#7718](https://github.com/Luminous-Dynamics/symtropy/issues/7718). Event identity must be per world step and bind endpoint generations, contact/manifold identity, event kind and a deterministic ordinal. Continuous contacts on different steps must remain distinct legitimate events. For delivery across process failure, prefer an idempotent sink/outbox; exactly-once cannot be claimed without a receiver-side transaction. Callback evaluation counts and emitted event counts must be recorded separately.

### 8.3 Stable callback event identities and receipt replay

The research crate now exports `callback_event_receipt`. `CallbackEventId` binds each emitted event to world revision, event subject, event kind and deterministic per-subject/per-step ordinal. Collision, trauma and dissipation use a canonical pair subject with endpoint generations and manifold revision; work uses a body subject with body generation and actuator revision. `CallbackEventSnapshot` additionally binds the global response-policy digest, endpoint response-state revisions, and a payload SHA-256 field.

`verify_callback_event_receipts` compares a supplied ordered delivery receipt stream against an independently supplied expected event set. It rejects noncanonical/duplicate/omitted/unexpected event identities, wrong world revisions, missing digest fields, and payload/response-state mismatches. It reports delivered, pending and failed outcomes separately; failed or pending is never reported as delivered.

The module has 12 authored tests. It does not invoke PhysicsCallback, derive the expected event set from solver impulses, hash actual callback payload bytes, or provide a durable outbox. Exactly-once delivery across process failure is not claimed. A production adapter should use an idempotent receiver/outbox; without a receiver-side transaction the honest contract is at-least-once delivery with stable deduplication identity. The remaining integration is tracked in [#7718](https://github.com/Luminous-Dynamics/symtropy/issues/7718).

### 8.2 Candidate topology is not exact contact topology

The present AABB receipt prototype uses the broadphase candidate relation as its pair relation. That is suitable for candidate-to-contact work obligations, but it does not distinguish an exact narrowphase contact from a conservative false-positive candidate. Therefore the absence of a post-contact stamp after a geometry refresh is not by itself proof that narrowphase ran and found no contact.

The next receipt revision is tracked in [#7702 — separate candidate and exact contact relations](https://github.com/Luminous-Dynamics/symtropy/issues/7702). It will maintain both:
- candidate relation: conservative broadphase pairs;
- exact contact relation: pairs with an actual narrowphase/manifold result, constrained to be a subset of candidates.

The receipt must explicitly derive contact-created, contact-retained and contact-ended outcomes separately from candidate added/removed/revalidated events. Every current exact contact needs one current payload stamp; every ended contact needs an explicit reason; a new candidate needs an explicit narrowphase result even when no contact is found. Until this is implemented, the current planner should be described as translating candidate-pair events into contact work obligations, not as proving the exact contact topology.



## 9. Relative-motion geometric lease prototype

The research crate now exports `contact_reuse_lease`, an initial executable admission predicate for #6865. It consumes a canonical relative-transform translation delta, an upper bound on maximum point-rotation angle, maximum contact-geometry lever arm, generation and shape revisions, stable feature identity and feature-margin lower bound, normal-variation upper bound, and updated signed-gap interval.

The conservative displacement bound is:

`norm_upper(delta_t_rel) + R_rel * theta_max + rounding_allowance`

using (2sin(	heta/2) le 	heta), conservative for the admitted angle interval ([0,pi]). Arithmetic operations are rounded upward with `next_up`; overflow or non-finite input fails closed.

Reuse is admitted only when the pair identity, proxy generations, shape revisions and feature identity remain valid; the feature-margin lower bound strictly exceeds the relative-motion upper bound; the normal-variation upper bound stays within tolerance; and the updated gap interval remains inside the declared contact/penetration envelope. The result is `ReuseCertified`, `ContactEnded`, `RefreshRequired(reason)` or `UnsupportedGeometry`. Equality at the feature-margin or contact-slop boundary refreshes rather than reuses.



The current feature identity must be compared against the cached feature identity on every lease evaluation. A missing cached/current feature witness returns `UnsupportedGeometry`; a mismatch returns `RefreshRequired(FeatureIdentityChanged)`. The predicate also rejects unsupported dimensions outside 2D–4D. Tests cover support-feature change, missing current identity, 2D/4D input, and cumulative motion measured from the last fresh manifold rather than rebased per recycled frame.

### Evidence boundary

This is a predicate over externally supplied witnesses. It does not yet derive the relative transform, feature margin, normal-variation interval, or updated gap interval from the real shape/manifold types. A production adapter must conservatively compute and independently qualify those values, or fall back to fresh narrowphase. The prototype does not modify production contacts and does not establish a speedup.

### Current production baseline observed

In the current `symtropy-physics` source, `PhysicsWorld::step_internal` clears `self.contacts` and reconstructs manifolds from broadphase candidates every step. The separate `ContactCache` stores normal/tangent impulses and contact points for warm-start matching by body pair plus proximity; its `begin_frame()` clears the frame cache. Therefore the current code has impulse warm-starting but not persistent geometric-manifold recycling. Any integration must first preserve the old manifold separately from the impulse cache and handle identity/revision/semantic invalidation.

### New external prior art: Box2D 3.2 contact recycling

Box2D 3.2 shipped contact recycling enabled by default. The October 2026 design write-up describes caching the relative transform of a contact pair, computing a relative-movement measure that combines translation and rotation, and reusing contact points while adjusting separation. It warns not to rebase the cached relative transform on each recycled step because accumulated drift results. Release notes describe a default movement threshold of roughly 5 cm or 11 degrees, configurable per world/body. Box2D 3.2 reports roughly 3x total performance over 3.1 and attributes a portion to contact recycling; these are vendor-reported results, not an independent matched comparison.

Therefore **relative-motion manifold recycling is established prior art**. Symtropy's research opportunity must go further: feature-stability lower bounds and normal-variation bounds, uncertainty intervals and deterministic refresh on boundary/unsupported cases, separation of manifold validity from material/callback and solver warm-start validity, generation/shape-revision-bound identity, and independently replayable contact lifecycle receipts.

References:
- [Box2D Contact Recycling, October 2026](https://box2d.org/posts/2026/10/contact-recycling/)
- [Box2D 3.2 release notes](https://box2d.org/documentation/md_release__notes__v320.html)
- [Box2D simulation/contact documentation](https://box2d.org/documentation/md_simulation.html)

### Contact lease predicate microbenchmark

The tournament package now has a separate `contact-lease-benchmark` binary. Run:

`cargo run --release --manifest-path research/broadphase-tournament/v0.1/Cargo.toml --bin contact-lease-benchmark -- 100000`

It emits 27 rows (nine witness/outcome classes × 2D/3D/4D) with requested checks, reuse/ended/refresh/unsupported/error counts, elapsed nanoseconds and nanoseconds per check. CI runs 100,000 checks per row and retains `contact-lease.csv` with the other exact-head benchmark artifacts. The harness times synthetic witness assembly plus the lease admission predicate; it does not call GJK/EPA/SAT, derive real feature margins or measure saved narrowphase work. Treat it as an admission-overhead baseline only.

### Qualification tests and targets

The lease prototype has 23 authored tests. The next acceptance corpus must add common rigid motion, rotation-dominant and long-lever-arm cases, thin/short feature switches, smooth/high-curvature shapes, normal drift, high-speed/teleport/CCD, feature-ID instability, deformation, and unsupported non-convex cases. Benchmark fresh narrowphase versus persistent-manifold lease by family; report lease checks/reuse/refresh/contact-end/unsupported counts, feature-margin and normal-bound distributions, GJK/EPA/SAT calls avoided, updated-separation work, manifold memory, and p50/p95/p99 including all checks. Require zero missed contact ends and zero false manifold reuse against an always-fresh narrowphase reference, plus bounded solver/trajectory error and stacking stability. Performance targets remain prospective until these correctness gates and matched engine runs pass.

## 10. Current authored test inventory

At the current research branch state, the authored test counts are:
- contact freshness producer and commit barrier: 33 tests;
- independent lifecycle/receipt verifier: 31 tests;
- relative-motion geometric lease predicate: 23 tests;
- callback event identity and delivery receipt verifier: 12 tests.

These counts describe source annotations only. They are not evidence that the tests have compiled or run. Exact-head Rust formatting, tests, strict Clippy and scale-campaign results must be read from the current GitHub Actions run before any pass claim.