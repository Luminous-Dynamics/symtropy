# World Continuation Manifest Contract v0.1

**Status:** design freeze for Q2 implementation; no runtime qualification claim  
**Scope:** world/system/body/region suspend-resume identity and content-addressed continuation  
**Depends on:** Authority Identity Layers v0.1, Q0/Q1 Universal Matter replay, issues #76/#79/#81  
**Tracks:** #83, #85, #86, #87, #88, #89, #90

## 1. Purpose

A Symtropy world may contain many independent authorities, many spatial scales, and many representations. A save, unload, body transition, interplanetary journey, server migration, or long inactive interval must not collapse those authorities into one new owner merely to make persistence convenient.

`WorldContinuationManifest` is therefore a **content-addressed proof root**, not a world-state database.

It answers:

> What exact domain-owned state, continuation state, lineage, forcing configuration, time policy, and restorable artifacts are required to resume this same causal world?

It does not answer those questions by copying the mutable state into `symtropy-world`.

## 2. Core invariant

For same-world continuation:

> Equal qualified continuation manifests plus equal required external inputs must be sufficient to restore equal domain continuation identities and produce equal deterministic future transitions under the same policies.

A manifest may refer to present physical-state digests, but a physical-state digest that omits continuation-significant scheduler/frontier/counter/residual state is not sufficient for a continuation claim.

## 3. Identity layers

The manifest preserves five distinct concepts.

### 3.1 World lineage identity

`WorldInstanceId` identifies one semantic world lineage.

It is not:

- a file name;
- a save slot;
- a manifest digest;
- a Git commit;
- a random storage path.

Same-world continuation preserves `WorldInstanceId`.

A fork receives a distinct `WorldInstanceId` even when its initial physical state is byte-for-byte or digest-for-digest identical to its parent.

### 3.2 Manifest identity

`WorldContinuationManifestDigest` identifies one canonical continuation root.

Changing any continuation-significant field changes this digest.

A new same-world checkpoint normally receives a new manifest digest while preserving its `WorldInstanceId`.

### 3.3 Domain semantic identity

A domain entry may bind independently:

- physical/state digest;
- continuation digest;
- lineage/replay digest.

The claim type must be explicit.

### 3.4 Snapshot artifact identity

A restorable snapshot has its own exact content digest and codec/schema identity.

Snapshot artifact identity is not semantic state identity.

Two differently encoded artifacts may decode to equal semantic state under an explicit migration/equivalence contract. Conversely, a content checksum passing does not prove that the decoded state matches the semantic digest declared by the manifest.

### 3.5 Derived/runtime identity

Renderer caches, entity handles, GPU objects, derived indexes, dirty-region queues, material bindings, and similar runtime state are not automatically part of semantic identity.

If omitted runtime state can change future authoritative evolution, it is not merely a cache and must be reclassified as continuation state.

## 4. Hierarchical structure

The world continuation root is hierarchical.

Conceptually:

```text
World/System Manifest
├── Stellar/System Child Manifest(s)
├── Body Manifest(s)
│   ├── Region Manifest(s)
│   │   ├── Domain snapshot/continuation refs
│   │   └── Derived rebuild contracts
│   └── Body-global domain refs
└── Cross-body/global domain refs
```

A parent hashes child manifest identities rather than duplicating child mutable state.

This enables:

- content-addressed deduplication;
- loading one body without loading the universe;
- local verification of a region subtree;
- unchanged subtree reuse across checkpoints;
- scalable interplanetary and stellar persistence.

## 5. Canonical conceptual schema

The exact Rust types may evolve during implementation, but v0.1 freezes the semantic fields.

### 5.1 Root manifest

A root manifest binds at minimum:

```text
schema_version
world_instance_id
continuation_sequence
lifecycle_mode
parent_manifest_digest?
source_simulation_instant
fixed_timebase_identity
reference_frame
inactive_time_policy_digest
forcing_context_digest?
causal_journal_head?
domain_entries[]
child_manifests[]
rebuild_requirements[]
```

No field above is implied by wall-clock time or storage location.

### 5.2 Domain continuation entry

A domain continuation entry binds:

```text
authority_or_domain_id
scope
reference_frame
observed_at / checkpoint_instant
schema_identity
physical_state_digest?
continuation_digest?
lineage_digest?
snapshot_content_digest
snapshot_codec_identity
representation_identity?
continuation_requirement
```

`continuation_requirement` is one of the semantic classes below.

### 5.3 Continuation requirement classes

#### RequiredExact

The state must restore exactly before continuation.

Examples include:

- Hydrology active frontier after #76/#79;
- Thermal active frontier;
- ActiveLava next-step counter;
- canonical landscape environment integration residuals after #81.

#### RebuildableWithProof

The state may be absent from the snapshot only when a declared deterministic rebuild contract proves it can be reconstructed from canonical inputs without changing authoritative behavior.

Examples may include:

- structural lookup indexes rebuilt from canonical node records;
- dirty-region/render/collision caches when a conservative deterministic rebuild is proven;
- some representation residency data if #86 proves exact reconstruction semantics.

#### PresentationOnly

The state is explicitly outside semantic/continuation identity.

Examples:

- GPU handles;
- renderer resources;
- transient ECS entity IDs when canonical objects have stable domain identities;
- local UI state.

Presentation-only records must never shadow or replace authority/domain entries.

## 6. Canonical ordering and hashing

Manifest digests must be serializer-independent.

JSON, TOML, CBOR, bincode, serde field order, or a file container may be used as a transport/fixture format, but none defines canonical digest bytes by itself.

The canonical digest contract must use:

1. an explicit domain separator;
2. explicit schema version;
3. explicit presence markers for options;
4. explicit element counts;
5. fixed integer byte order;
6. length-prefixed validated identifiers or their already-canonical identity bytes;
7. typed digest domain/schema/value tuples;
8. canonical collection order.

### 6.1 Domain-entry sort key

Domain entries are canonicalized by a stable tuple equivalent to:

```text
(scope, authority_or_domain_id, identity_class/schema)
```

Arrival order and storage enumeration order must not affect the manifest digest.

Duplicate/conflicting authoritative bindings for the same semantic key fail closed.

### 6.2 Child-manifest sort key

Child references are canonicalized by child scope, then child manifest digest.

Sibling authoritative scopes must be disjoint unless a separately versioned overlay/composition rule explicitly permits overlap.

Parent data cannot silently override a child-owned authoritative scope.

## 7. Same-world continuation vs fork

### 7.1 Continue same world

A same-world continuation must:

- preserve `WorldInstanceId`;
- bind a valid parent/ancestor manifest where policy requires it;
- advance the continuation sequence monotonically;
- restore every `RequiredExact` entry;
- satisfy every `RebuildableWithProof` contract;
- preserve the required simulation-time and forcing context;
- recompute the same manifest identity for a pure suspend/load checkpoint before further simulation.

Re-encoding, compressing, moving, or deduplicating snapshot bytes does not create a new world.

### 7.2 Fork new world

A fork must:

- mint a distinct `WorldInstanceId`;
- record source/parent manifest ancestry;
- bind a fork policy/reason identity when required;
- permit shared content-addressed snapshot artifacts at fork genesis;
- treat subsequent continuation independently.

Equal physical state does not make a fork the same world.

A fork may share bytes without sharing semantic lineage identity.

## 8. Simulation-time semantics

Wall-clock time is never authoritative gameplay time.

A manifest binds an inactive-time policy.

v0.1 recognizes the following semantic classes.

### 8.1 Paused

The scope does not advance simulation time while inactive.

Arbitrary real-world absence must not alter authority state.

### 8.2 DeterministicStepCatchUp

The scope advances from a recorded source simulation instant to a declared target instant using deterministic bounded stepping.

The catch-up may be chunked for work budgeting, but chunk scheduling alone must not change the final authoritative result.

### 8.3 DomainApprovedCoarseEvolution

A domain may advance using a coarser or analytical representation only through a declared policy and domain-approved representation-transfer/equivalence contract.

The common world layer does not assume coarse evolution is equivalent.

### 8.4 EventDrivenEvolution

A future domain may advance across sparse deterministic events only under an explicit versioned transition contract.

This mode is not implied by the existence of an event log.

## 9. Deterministic forcing context

Stateless deterministic forcing such as weather fields, orbital ephemerides, stellar irradiance, tides, or boundary-condition generators may affect future evolution without being persistent authority state.

The manifest therefore binds the forcing context required for continuation, for example:

```text
forcing_model_identity
forcing_config_digest
seed/input identity
source cursor / simulation instant
policy identity
```

If a forcing source itself has continuation-significant mutable state, it must become a domain continuation entry rather than being disguised as stateless forcing.

Missing required forcing context fails closed.

## 10. Representation residency

Existing `symtropy-world` residency semantics already distinguish active representation, minimum residency lease, and a domain release permit bound to exact state.

Suspend/resume must preserve their semantics.

A manifest must either:

- bind continuation-critical residency state exactly; or
- bind a deterministic rebuild policy/config and proof classification showing equivalent residency decisions are reconstructed from canonical inputs.

A lease cannot disappear on reload if that would permit an earlier release.

A stale release permit cannot become fresh by losing intermediate state.

Renderer entities and resources remain presentation state.

## 11. Snapshot and artifact semantics

For each required snapshot:

1. verify exact content digest before decode or mutation;
2. verify recognized codec/schema identity;
3. decode transactionally;
4. recompute the semantic state/continuation/lineage identities claimed by the manifest;
5. reject on mismatch;
6. adopt the restored authority only after all applicable verification succeeds.

A file name, directory path, object-store key, or archive member name is never identity.

### 11.1 Migration

Snapshot migration must produce:

- a new artifact content identity;
- explicit source and destination codec/schema identities;
- a migration/equivalence receipt;
- recomputed destination semantic identities.

A migration may preserve `WorldInstanceId` without preserving snapshot bytes.

## 12. Causal journal head

The root may optionally bind a causal-journal head.

The existing `symtropy-game-state` v1 chain uses serializer-produced JSON bytes for a generic payload and therefore must be labeled according to its actual semantics.

A cross-version canonical causal journal should use the future serializer-independent event identity tracked in #82 before it becomes mandatory Q2 world-continuation evidence.

The absence of a canonical journal does not prevent domain snapshot/continuation identity from being correct.

## 13. Restore algorithm invariant

A same-world restore proceeds conceptually as:

```text
verify manifest canonical structure
        ↓
verify ancestry / world identity / sequence
        ↓
verify child manifests recursively
        ↓
verify snapshot content digests
        ↓
decode domain snapshots transactionally
        ↓
recompute domain state/continuation/lineage identities
        ↓
rebuild declared rebuildable state
        ↓
verify rebuild contracts
        ↓
restore forcing/time/residency context
        ↓
recompute continuation manifest
        ↓
require equality with suspended manifest
        ↓
commit restored world for simulation
```

No partial live-world mutation is allowed before verification closes.

## 14. Merkle-style scaling and deduplication

The hierarchy should behave like a content-addressed Merkle DAG in semantics even if the first storage implementation is simpler.

Consequences:

- unchanged child manifests can be reused;
- unchanged domain snapshot artifacts can be deduplicated;
- a local region change propagates through region -> body -> system/root digests;
- unrelated unchanged bodies need not rewrite all snapshot bytes;
- a client/server/tool can verify one subtree against a trusted root.

The design must not require hashing the entire universe's raw mutable bytes into one flat buffer.

## 15. Required validation failures

Construction/restore must fail closed for at least:

- unknown required manifest schema;
- duplicate authoritative domain key;
- overlapping child ownership without an explicit composition rule;
- child scope/frame mismatch;
- missing required snapshot artifact;
- snapshot content checksum mismatch;
- decoded semantic identity mismatch;
- missing required continuation digest;
- physical-state digest offered where the entry requires continuation identity;
- missing required forcing context;
- invalid backward same-world sequence/time transition;
- fork presented as same-world continuation;
- unsatisfied rebuild contract;
- stale/incompatible residency state;
- unknown required codec.

## 16. Required Q2 tests

### Canonical identity

- entry arrival order does not change digest;
- child arrival order does not change digest;
- one-field perturbation changes digest as specified;
- fixed golden vectors remain stable;
- duplicate/conflicting entries reject.

### Hidden continuation state

At manifest level prove sensitivity to:

- Hydrology active frontier;
- Thermal active frontier;
- ActiveLava next-step counter;
- nested eruption continuation;
- landscape integration residual continuation, if in scope.

The narrower physical digests may remain equal in these fixtures.

### Snapshot restore

- suspend -> restore -> recompute yields the same manifest;
- corrupted bytes fail before live mutation;
- valid bytes with wrong semantic result fail;
- missing derived cache can rebuild when and only when a proof contract exists.

### Behavioral continuation

For a bounded deterministic fixture:

```text
run N steps
checkpoint
run M more steps  -> reference

vs

run N steps
checkpoint
restore
run M more steps  -> resumed
```

Require canonical authority reports/state identities at selected checkpoints to match.

### Same-world/fork

- resume preserves world identity;
- fork changes world identity;
- fork ancestry is verifiable;
- identical genesis artifacts can be shared without collapsing lineage identity.

### Inactive time

- paused scopes remain unchanged across arbitrary host time;
- deterministic catch-up is independent of host scheduling/chunking under the declared policy;
- interrupted catch-up resumes exactly;
- coarse evolution requires its domain proof/receipt.

## 17. Q2 evidence relationship

Passing ordinary unit tests is not sufficient to call a world continuation root Q2-qualified.

The Q2 evidence capsule tracked in #84 must bind:

- exact Git tree;
- Q1 regression result;
- continuation-identity repairs;
- manifest golden vectors;
- hidden-state sensitivity results;
- suspend/resume behavioral checkpoint results;
- rebuild/cache proofs;
- toolchain/lockfile/environment;
- complete logs and checksum manifest.

Q3 native CUF integration must inherit from an exact Q2-qualified parent.

## 18. Non-goals

v0.1 does not:

- make `symtropy-world` owner of terrain/water/ecology/etc.;
- define a universal snapshot codec;
- claim all domains are scientifically complete;
- require every cache to be persisted;
- use wall-clock time as simulation truth;
- declare representation changes automatically equivalent;
- require the whole universe to be loaded for a local resume;
- turn physical-history chronicles into total continuation identities;
- rewrite Universal Matter v4.8 historical artifacts.

## 19. Implementation sequence

The preferred sequence is:

1. preserve Q0/Q1 result for exact Universal Matter v4.8 replay;
2. repair Q2 continuation identities/residual state (#76/#79/#81);
3. implement manifest canonical core (#83/#87/#89/#90);
4. integrate time policy (#85);
5. integrate residency lifecycle semantics (#86);
6. add canonical golden vectors (#88);
7. add machine-checkable Q2 gate/capsule (#84);
8. qualify/promote an exact Q2 tree;
9. start CUF v0.11 native adapters (#72) from that tree;
10. earn Q4 with a real Living Watershed vertical slice.

## 20. Design outcome

The intended result is not merely a save file.

It is a proof that:

> this is the same world, at this simulation instant, with these domain-owned physical states, these hidden continuation states, these forcing/time policies, this ancestry, and these exact restorable artifacts — and that restoring it does not silently change what happens next.
