# Ledger: symtropy-oceanic Crate

**Status:** concept archived — workspace member reference removed  
**Archived:** 2026-06-10  
**Archived by:** automated stabilisation pass

---

## Original Intent

`symtropy-oceanic` was a proposed crate introduced by the large "DeSci Mesh"
commit (`7efeac951`) as part of the planetary-scale simulation track. It was
to extend Symtropy's physics solver with:

- **Oceanographic simulation:** wave dynamics, current modelling, coastal
  erosion — all as a Track A (research hero) N-dimensional physical system
- **Ecosystem coupling:** linking ocean temperature and salinity fields to
  `ConsciousnessField` via a new coupling channel (environment → Φ)
- **Climate-physics bridge:** atmospheric pressure coupling using the same
  `PhysicsCallback<D>` trait extension point, so the ocean could be a pure
  Symtropy physics extension with zero engine modification

---

## What Changed

The `Cargo.toml` workspace `[members]` list referenced `crates/symtropy-oceanic`
but the directory was never created. This caused `cargo build` and `cargo check`
to fail immediately.

The reference was removed from `workspace.members`. The crate name, intent, and
design rationale are preserved here.

---

## Intended Crate Location

```
crates/symtropy-oceanic/
├── Cargo.toml
├── README.md
└── src/
    ├── lib.rs
    ├── ocean_field.rs       ← N-dimensional ocean state tensor
    ├── wave_dynamics.rs     ← wave equations as PhysicsCallback<D>
    ├── current_solver.rs    ← advection + Coriolis
    ├── ecosystem.rs         ← salinity/temp → Φ channel
    └── climate_bridge.rs    ← atmospheric coupling
```

---

## Reactivation Conditions

1. `crates/symtropy-oceanic/` directory created with a `Cargo.toml`
2. `crates/symtropy-oceanic/src/lib.rs` compiles (even as skeleton)
3. Feature flag `experimental-oceanic` added to root `Cargo.toml`
4. Crate added back to `workspace.members`
5. At minimum one test exercises the `PhysicsCallback<D>` interface

---

## Related Assets

- `crates/symtropy-consciousness-physics/src/coupling.rs` — reference implementation for coupling channels
- `crates/symtropy-physics/src/world.rs` — `PhysicsCallback<D>` trait definition
- `ROADMAP.md` — DeSci mesh / planetary-scale simulation track
- `docs/GAME_ENGINE_COMPONENTS.md` — ecological / planetary physics row
