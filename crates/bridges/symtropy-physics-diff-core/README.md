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
