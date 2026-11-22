# Physical State Feedback Contract v0.1

A dependency-free Rust contract for queryable state from physical simulators and action-conditioned world models.

## Why

A model is not a trustworthy simulator merely because it generates plausible images or actions. It must expose named state at immutable snapshots, distinguish measured/estimated/predicted/assumed/unavailable values, preserve units and frames, and declare what uncertainty is known.

The crate defines the type and validation layer only. It deliberately does not ship a physics engine, a world model, a JSON/CBOR serializer, or a hardware adapter.

## Contract contents

- Snapshot identity: model/configuration SHA-256, world/episode/trajectory, monotonic step/time, clock source, world frame, transform convention and SI units.
- Entity state: SE(3) pose with explicit world-from-entity direction and parent/child frame IDs, linear/angular velocity, acceleration, mass, inertia and contact readback. Contact classification carries its own provenance and uncertainty.
- Field semantics: mandatory unit, epistemic status, source/model/fixture/derivation provenance, and quantified or explicitly NotQuantified uncertainty. Measured, estimated, predicted and assumed fields have status-specific provenance requirements. A 3×3 covariance declares its squared SI unit and coordinate frame, which must match the associated field.
- Fail-closed unavailable values: UNAVAILABLE requires value=None and a reason; it cannot silently encode zero.
- Validation: finite numeric values, normalized quaternions, symmetric positive-semidefinite 3×3 covariance and inertia matrices with dimensionless scale-normalized tolerances, rigid-body inertia triangle inequalities, referentially valid entity/contact IDs, monotonic snapshot sequences and immutable query IDs.
- Read-only query trait: methods borrow the provider immutably and requests name the exact snapshot digest. Results must include requested entities and only their transitive parent/contact context; unrelated hidden entities are rejected.
- Action-bound step contract: a step binds the pre-snapshot, model/configuration, world/episode/trajectory, action-stream digest, duration, clock/frame identity, and exact next snapshot time/index.
- Replay receipts: exact digest bindings plus deterministic, bounded-stochastic or explicitly non-replayable classes. A receipt can be validated against the exact StepRequest and returned snapshot.
- Canonical snapshot content bytes: `StateSnapshot::canonical_content_bytes` emits a versioned, deterministic big-endian encoding of every content field except the claimed `snapshot_sha256`. Entity records and contact-partner sets are sorted; empirical sample order is retained; strings use exact UTF-8 bytes without Unicode normalization; finite negative zero is normalized to positive zero. The caller still computes SHA-256 over these bytes and compares it with the identity claim.

## Build and test

Run cargo test and cargo clippy against this manifest with the locked dependency set.

## Limitations

- No engine implements the read-only query or action-conditioned step traits yet.
- The canonical byte encoding is a content-hash preimage format, not a general JSON/CBOR wire protocol. It is versioned by a fixed magic prefix and schema version; changes to field order, tags, float rules or string policy require a new canonical encoding version.
- The caller still computes SHA-256 over canonical bytes and supplies/compares the claimed digest; this crate does not implement the cryptographic hash function. The claimed digest is excluded from its own preimage.
- NotQuantified is an explicit absence of an uncertainty estimate, not calibrated uncertainty.
- A structurally valid state snapshot does not demonstrate physical fidelity, causal correctness or downstream decision safety.
- Covariance and inertia-matrix validation is limited to 3×3 matrices. Scale normalization avoids unit/magnitude-dependent PSD decisions, but the validator still uses floating-point tolerances rather than an interval proof; near-threshold cases are not certified exact. Full 6×6 pose covariance and articulated/deformable state are future extensions.
- Adapter conformance must be tested against independent physical measurements and the GAUGE programme (#6670).

This is a research contract prototype, not a claim that Symtropy already exposes a complete simulator API.
