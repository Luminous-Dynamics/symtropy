# Ledger: Rapier3D Bridge — Manipulator Module

**Status:** archived placeholder — feature-gated behind `experimental-manipulator`  
**Archived:** 2026-06-10  
**Archived by:** automated stabilisation pass

---

## Original Intent

`manipulator.rs` was intended to be the high-level API layer inside
`symtropy-rapier3d-bridge` for:

- **Kinematic robot arm control** — IK solver wrapping Rapier's kinematic body
  interface, driven by target end-effector poses from `symtropy-robotics-bridge`
- **Constraint-based manipulation** — direct force/torque injection into Rapier
  joints, with Symtropy's `ConsciousnessField` modulating joint stiffness
- **Sensor abstraction** — abstract `GraspSensor<D>` that returns N-dimensional
  contact data, usable by both the robotics and AI subsystems

The intent was to make `symtropy-rapier3d-bridge` the production-grade 3D path
for robotics simulations, with Symtropy's own ND solver as the research path.

---

## What Was Removed From Active Compile Path

| File | Change |
|------|--------|
| `crates/symtropy-rapier3d-bridge/src/lib.rs` | `pub mod manipulator;` → `#[cfg(feature = "experimental-manipulator")] pub mod manipulator;` |

---

## Note on PhysicsPipeline::step()

The main `PhysicsPipeline::step()` in `crates/symtropy-rapier3d-bridge/src/pipeline.rs`
is currently a **stub** (calls `rapier3d::pipeline::PhysicsPipeline::step()` but
returns immediately without wiring Symtropy coupling). This is a separate issue
from the missing `manipulator.rs` module.

See: ROADMAP.md P1 item "Fix symtropy-rapier3d-bridge".

---

## Reactivation Conditions

1. `crates/symtropy-rapier3d-bridge/src/manipulator.rs` is written with at
   minimum a `RapierManipulator` struct
2. Feature flag `experimental-manipulator` exists in the crate's `Cargo.toml`
3. `cargo check -p symtropy-rapier3d-bridge --features experimental-manipulator` passes
4. At least one test exercises IK or joint force injection through Rapier

---

## Related Assets

- `crates/symtropy-rapier3d-bridge/src/pipeline.rs` — main bridge pipeline (stubbed)
- `crates/symtropy-rapier3d-bridge/src/lib.rs` — existing pub re-exports
- `crates/symtropy-robotics-bridge/` — consumer of this API
- `ROADMAP.md` — "Fix rapier3d bridge" (P1)
