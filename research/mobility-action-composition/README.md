# Verified action composition v0.1

**Tracking:** Symtropy [MOBILITY-VERIFIED-ACTION-COMPOSITION-001 #6698](https://github.com/Luminous-Dynamics/symtropy/issues/6698)  
**Parent:** [MOBILITY-MUTATION-BOUNDARY-001 #6669](https://github.com/Luminous-Dynamics/symtropy/issues/6669)  
**Status:** research-only typed batch-admission reference; Rust tests authored, no execution pass claimed.

This crate checks whether one exact set of autonomous action attempts is structurally eligible for concurrent execution under a declared profile. It does not schedule or dispatch actions.

The default M1 single-unresolved-mutation rule remains unchanged. This crate does not relax it globally and does not create a new controller, reservation calendar, occupancy engine, authority service or sensor/evidence registry.

## Reference behavior

The crate checks:
- exact action ID, attempt and unique ticket digest;
- one shared authority/configuration/time binding across the proposed batch;
- qualified non-empty footprint for every action;
- state read/write conflicts against exact object generations;
- exclusive resource/time overlap using same-basis half-open intervals;
- aggregate capacity over every shared resource, including multi-action/hyperedge overload;
- one exact pairwise owner check per action pair and every required owner family;
- owner result must be Clear; Conflict, Unknown, missing, extraneous or duplicate checks block admission.

The aggregate resource event sweep applies interval ends before starts at an equal time, matching half-open interval semantics.

## Existing owners to reuse

- Symtropy #3644 / #4992 / #5009: resource and temporal reservation authority.
- Symtropy #5233: space-time occupancy and uncertainty-expanded temporal intervals.
- Symtropy #5401: motion-envelope projection into occupancy.
- Symtropy #5235: active observation planning over resource/occupancy owners.
- Symtropy #6669: per-action authority-linearized application and one-shot native mutation.
- Mycelix #803: durable effect attempt / persistence classification.
- Mycelix #830: independent offline reconstruction of bounded action authority and effect evidence.

The required owner-check profile must list exact owner families. The caller cannot silently omit one and infer independence from a lack of conflicts. Owner receipts must be independently validated by their owning subsystem; this crate only checks their structural references and declared dispositions.

## Important limitations

- No spatial intersection or swept-volume algorithm is implemented here. Space-time geometry must be supplied by the existing occupancy/motion-envelope owners.
- No reservation is created. Capacity/occupancy claims must reference exact current owner state before dispatch.
- No signatures, digests, calibration or owner truth are authenticated.
- No low-level safety controller, actuator driver, emergency stop or native application path is implemented.
- A QualifiedBatch is eligibility evidence, not an actuator capability.
- Every action still needs its own exact #6669 ticket check at the native application boundary.
- Every action requires its own outcome receipt and reconciliation. An unknown outcome must quarantine its dependency closure; if that closure cannot be reconstructed, use global serial hold.
- The first reference checks one fully independent concurrent batch. It does not yet optimize a multi-batch schedule or calculate a maximum independent set.

## Run

```sh
cargo fmt --manifest-path research/mobility-action-composition/Cargo.toml -- --check
cargo test --manifest-path research/mobility-action-composition/Cargo.toml --locked
cargo clippy --manifest-path research/mobility-action-composition/Cargo.toml --all-targets --locked -- -D warnings
```

Sixteen unit tests are authored in the initial reference. This is source state, not a reported test pass.
