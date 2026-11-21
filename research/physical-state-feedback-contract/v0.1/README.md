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
- Validation: finite numeric values, normalized quaternions, symmetric positive-semidefinite 3×3 covariance and inertia matrices, referentially valid entity/contact IDs, monotonic snapshot sequences and immutable query IDs.
- Read-only query trait: methods borrow the provider immutably and requests name the exact snapshot digest. Results must include requested entities; related parent/contact context entities may also be returned.
- Action-bound step contract: a step binds the pre-snapshot, model/configuration, world/episode/trajectory, action-stream digest, duration, clock/frame identity, and exact next snapshot time/index.
- Replay receipts: exact digest bindings plus deterministic, bounded-stochastic or explicitly non-replayable classes. A receipt can be validated against the exact StepRequest and returned snapshot.

## Build and test

Run cargo test and cargo clippy against this manifest with the locked dependency set.

## Limitations

- No engine implements the read-only query or action-conditioned step traits yet.
- No serialization format is committed in v0.1; wire compatibility and canonical encoding must be designed with the first adapter.
- The caller supplies snapshot digests; this crate does not hash a complete canonical byte representation.
- NotQuantified is an explicit absence of an uncertainty estimate, not calibrated uncertainty.
- A structurally valid state snapshot does not demonstrate physical fidelity, causal correctness or downstream decision safety.
- Covariance and inertia-matrix validation is limited to 3×3 matrices; full 6×6 pose covariance and articulated/deformable state are future extensions.
- Adapter conformance must be tested against independent physical measurements and the GAUGE programme (#6670).

This is a research contract prototype, not a claim that Symtropy already exposes a complete simulator API.
