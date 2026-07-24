# Ledger: City-Scale Plugin

**Status:** archived placeholder — feature-gated behind `experimental-village`  
**Archived:** 2026-06-10  
**Archived by:** automated stabilisation pass

---

## Original Intent

A large-scale governance and economy simulation layer built on top of the
Mycelix consensus protocol, intended to run as both a standalone binary demo
and a phase of the Symtropy launcher.

`CityScalePlugin` was to be the entry point for a self-governing city simulation
showing:

- **Governance:** Mycelix ZKP-based quadratic voting across 50 simulated agents
- **Economy:** entropy-backed token flows from `mycelix-economy`
- **Physics:** crowd dynamics via Symtropy's N-dimensional rigid-body solver
- **Consciousness:** aggregate Φ tracking across the city as a collective metric

This corresponds to milestone **M4** in `ROADMAP.md`:
> *"Mycelix Village: 50 NPCs, governance + economy loop at 60fps"*

---

## What Was Removed From Active Compile Path

| File | Change |
|------|--------|
| `src/plugin.rs` | `.add_plugins(symtropy_mycelix_village::city_scale_logic::CityScalePlugin { ... })` moved to `#[cfg(feature = "experimental-village")]` block |
| `crates/symtropy-mycelix-village/Cargo.toml` | `[[bin]]` entry for `city-scale` (pointing to `src/city_scale.rs`) removed — intent preserved here |

---

## Intended Binary

```toml
[[bin]]
name    = "city-scale"
path    = "src/city_scale.rs"
```

Would have produced: `cargo run -p symtropy-mycelix-village --bin city-scale`

---

## Intended Crate Layout (when reactivated)

`symtropy-mycelix-village` needs to grow a `[lib]` target for the launcher
plugin to import from it:

```toml
[lib]
name = "symtropy_mycelix_village"
path = "src/lib.rs"
```

Then `city_scale_logic` would be a module inside that lib:
```
crates/symtropy-mycelix-village/src/
├── lib.rs                   ← re-exports city_scale_logic
├── city_scale_logic/
│   ├── mod.rs               ← CityScalePlugin definition
│   ├── governance.rs
│   ├── economy.rs
│   └── crowd_physics.rs
├── main.rs                  ← existing headless simulation
└── headless.rs              ← existing headless entry point
```

---

## Reactivation Conditions

1. `symtropy-mycelix-village` gains a `[lib]` target
2. `src/city_scale_logic/mod.rs` is written with a compiling `CityScalePlugin`
3. Feature flag `experimental-village` exists in root `Cargo.toml`
4. `cargo check --features experimental-village` passes
5. The binary demo runs headlessly without panic for at least 100 ticks

---

## Related Assets

- `crates/symtropy-mycelix-village/src/main.rs` — existing NPC loop
- `crates/symtropy-mycelix-village/src/headless.rs` — existing headless runner
- `src/resources.rs:GamePhase::CityScale` — phase variant retained in enum
- `ROADMAP.md` — M4 milestone
- `docs/GAME_ENGINE_COMPONENTS.md` — "Mycelix governance/economy layer" row
