# Symtropy Physics Differential Core

A dependency-light shared data contract for recording observations from independent physics backends. The crate intentionally contains no engine integration and no claim that either backend is physical ground truth. Its only runtime dependency is Serde for machine-readable records.

## Contract

An `ObservationFrame` identifies one sample from one execution profile:

- `scenario_id` and `scenario_digest`: the declared scenario and its canonical input digest.
- `backend_id`, `backend_version`, and `solver_profile`: the implementation and explicit numerical policy.
- `configuration_digest`: the canonicalized solver/world settings used for the run.
- `timestep_seconds`, `step_index`, and `simulation_time_seconds`: simulation time, independent of wall clock.

Body IDs are semantic IDs supplied by the scenario mapping. A Rust engine handle, body insertion position, ECS entity number, or backend traversal order must never be used as an implicit cross-engine key.

Body kinematics use SI-labelled fields. Position is metres, linear velocity metres per second, angular velocity radians per second, and orientation is a scalar-first quaternion `[w, x, y, z]). The native adapter converts its rotor matrix and plane-valued angular velocity at the boundary; Rapier exports its own state into the same convention.

## Unavailable measurements

Optional measurements are encoded as absent fields when the backend cannot report them or when their conventions have not been qualified. Do not replace unavailable contact counts, penetration, or energy with zero. Potential energy may be negative because its reference level is scenario-dependent. A metric should only be compared when both adapters document compatible semantics.

## Validation

`ObservationFrame::validate` checks:
- schema version, non-empty provenance fields, and a finite positive timestep;
- finite nonnegative simulation time and finite vectors;
- normalized WXYZ quaternions and unique, non-empty semantic body IDs;
- finite optional metrics, nonnegative penetration depths and kinetic energy.

This is structural validation, not proof of deterministic execution, physical accuracy, or cross-backend equivalence.

## Current boundary

D1 foundation: common serializable records, validation, a native 3D frame adapter, and an explicit Rapier body-state adapter. Scenario identity must still be supplied by the caller, and the runner must collect equivalent frame-level measurements from both worlds.

Not included yet: scenario fixture corpus, deterministic replay qualification, tolerance profiles, time-series divergence metrics, contact-heavy stack/CCD suites, or an automatic comparator. Those remain later PHYS-DIFF-01 stages; observation agreement alone does not establish physical truth.


## Trace envelope (schema v1)

`ObservationTrace` is the canonical sampled-run container for `ObservationFrame` values. It holds one immutable `RunIdentity` for the entire run and validates that every appended frame carries the same identity, has a strictly increasing step index and simulation time, and obeys the fixed-step clock `time = step_index * timestep_seconds` within a scale-aware floating-point envelope. A failed `push` does not mutate the trace; import/serialization boundaries can call `validate` to recheck the complete frame sequence. Empty traces, mixed backend identity, repeated/backwards steps, backwards time and off-clock timestamps are rejected.

The trace envelope does not prescribe engine-specific contact or energy semantics. `None` continues to mean unavailable or not comparable; it is never silently converted to zero.


## Paired-run lineage manifest (schema v1)

The crate also exposes a Symtropy-specific `DifferentialLineageManifest` and
`audit_differential_lineage` adapter. This is a sidecar to the canonical
`ObservationTrace`, not another observation format or a generic
independence-profile implementation.

A manifest binds:
- one shared scenario ID and exact scenario digest for the paired experiment;
- each lane's existing `RunIdentity` and exact serialized-trace SHA-256
  commitment;
- explicit dependency identities for solver implementation, scenario
  generator, expected-answer oracle, observation adapter, result
  canonicalizer, source revision/tree, Cargo.lock, compiler/toolchain,
  runner image, execution provider, host/runtime, hardware, operator,
  acquisition and calibration;
- completeness state and explicit known/unknown/not-applicable evidence for
  each declared dependency;
- optional digest references to the canonical Mycelix QUAL-001E,
  ASSURE-PORTFOLIO-CUT-001 and LEX-NET-030 owners.

Multiple dependencies of the same kind remain separate entries. Partial
overlap is detected by the actual `(dependency kind, identity)` pair, not by
dependency-node labels or a composite per-lane digest. A shared scenario is
expected and is recorded separately; it is not counted as a second
independent observation.

The caller supplies a separately pinned lineage policy specifying required
lane kinds and dependency kinds that must be disjoint. The audit verifies that
each provided trace validates, matches its exact lane `RunIdentity`, and
uses the same scenario subject. Shared solver, expected-answer or adapter
identities remain visible. Unknown/incomplete dependencies block a declared
separation. A disjoint result is only
`DeclaredDisjointUnverified`: it does not authenticate the source references,
establish organizational independence, or prove physical truth.

The Mycelix assurance references remain unbound until their actual profile,
policy and graph digests are supplied by their canonical owners. This adapter
does not create replacement schemas, signatures or trust roots.

### Minimal use

```rust
let report = audit_differential_lineage(
    &manifest,
    &observation_traces_by_lane_id,
    &externally_pinned_policy,
    &expected_policy_sha256,
)?;
```

The caller must calculate and authenticate `observation_trace_sha256` from
the exact serialized trace bytes before treating the manifest as evidence.
This crate validates digest shape and binds the value; it does not calculate
or authenticate those bytes.

### Qualification cases added

- paired native/reference traces bind the exact same scenario subject;
- shared scenario generator, runner image, observation adapter, compiler/toolchain and execution provider remain visible;
- one expected-answer lineage reused by both lanes blocks the mandatory
  expected-answer separation;
- a partial inventory or unknown solver lineage remains indeterminate;
- duplicate dependency identities do not multiply evidence;
- invalid trace, scenario substitution, policy digest and artifact digest
  controls reject;
- canonical assurance digests are shape-checked but never described as
  verified merely because a string is present.

This sidecar is an evidence-binding adapter. It does not make the differential
runner execute, decide which solver is correct, or establish an SOTA result.
