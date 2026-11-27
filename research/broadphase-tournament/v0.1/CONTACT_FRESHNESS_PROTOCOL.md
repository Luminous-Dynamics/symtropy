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
- `GlobalContactRevisions`: world and collision-policy revisions; SHA-256 digests for narrowphase and solver configuration.
- `ContactFreshnessStamp`: canonical pair plus the exact endpoint/global tuple under which a manifold was last produced.
- `ContactReuseDecision`: independent `geometry`, `warm_start`, `pair_eligibility_recheck_required` and `solver_activation_required` outputs.
- `plan_contact_lifecycle`: translates canonical, disjoint pair-event streams into deterministic `Added`, `Removed` and `Revalidated` contact actions. New and revalidated pairs require current endpoint revision records; a revalidated pair without a stamp requires a full contact build; an added pair with an orphaned prior stamp is rejected; a stamped revalidation with no typed revision reason is rejected as unexplained. New pairs also request a collision-eligibility recheck because this AABB-only layer does not prove the engine's filter policy.
- `ContactFreshnessError` / `ContactLifecycleError`: identity mismatch, noncanonical or conflicting events, missing endpoint data and monotonic-revision violations. Regressions fail closed.

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
| material revision | no geometric rebuild required | no | no | no |
| solver config digest | no geometric rebuild required | no | no | no |
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

Implemented in this branch: typed revision structures, generation-aware identity checks, monotonicity errors including world-revision binding, separate geometry/warm-start/activation decisions, deterministic pair-delta-to-contact action planning, fail-closed unexplained-revalidation detection, an independently implemented action-plan verifier, immutable before/after receipt replay, and authored unit tests for mutation and event classes. The verifier is a separate algorithm boundary in the same crate, not yet a separately deployed trust anchor.

Not yet implemented or established: live world mutation journal integration; cryptographic receipt serialization; independent verifier/replay; real contact-manager or solver integration; matched external-engine runs; Rust formatting, tests, Clippy or release timings on this exact branch head.

No speedup, correctness qualification or SOTA superiority claim is made until these gaps close.

## 8. Immutable transition receipt and independent replay

The branch now exports `contact_freshness_verifier` with two layers:

- `verify_contact_lifecycle_plan` independently reconstructs the expected `Added`, `Removed` and `Revalidated` actions from a supplied pair-delta frame, prior contact stamps, and current revision records. It does not call the producer planner or `ContactFreshnessStamp::assess`; the audit logic is separately implemented.
- `verify_contact_lifecycle_receipt` starts from immutable before/after pair-relation snapshots, before/after endpoint-revision maps, prior contact stamps, and the producer's plan. It derives additions/removals by set difference and derives retained-pair revalidation from changed endpoint revisions, missing prior stamps, and global collision/narrowphase/solver configuration changes. It then invokes the separate action audit.

Receipt checks include:
- strictly advancing world-step revision and monotonic collision-policy revision;
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
- reconstructing addition and removal events from before/after relation snapshots.

The benchmark follow-on should measure receipt construction, independent replay CPU, serialization size, verifier memory and the ratio of safe-fallback transitions. These costs must be included in the end-to-end tick and in any speed claims.
