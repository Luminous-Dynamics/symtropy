# Ledger: Muse System

**Status:** archived placeholder — feature-gated behind `experimental-muse`  
**Archived:** 2026-06-10  
**Archived by:** automated stabilisation pass

---

## Original Intent

An adaptive, consciousness-responsive audio layer for the Symtropy launcher.
The Muse system was intended to sit between `symthaea-muse` (which provides
`MuseConfig`, `MusicalState`, `ReverbConfig`) and the Bevy plugin system,
translating live Φ values and biometric stress signals into real-time
procedural music.

It would have been the primary demonstration of the `live-audio` feature flag.

Key design elements present in the codebase but blocked by the missing module:

- `symthaea_muse::{MuseConfig, MusicalState, ReverbConfig}` — already imported
  in `src/systems/audio.rs`
- `stress_to_musical_state` — defined in `symthaea_biometrics::muse_bridge`;
  currently commented out in `audio.rs` pending this module
- `GamePhase::Muse` — variant added to the launcher's `GamePhase` enum in
  `src/resources.rs` to support a dedicated Muse visualiser phase

---

## What Was Removed From Active Compile Path

| File | Change |
|------|--------|
| `src/systems/mod.rs` | `pub mod muse;` → `#[cfg(feature = "experimental-muse")] pub mod muse;` |
| `src/plugin.rs` | `.add_plugins(systems::muse::MusePlugin)` moved into `#[cfg(feature = "experimental-muse")]` block |

---

## Reactivation Conditions

1. `src/systems/muse.rs` is written and compiles
2. `MusePlugin` is defined and registers at minimum one Bevy system
3. Feature flag `experimental-muse` is added to root `Cargo.toml`
4. `cargo check --features experimental-muse` passes
5. A basic smoke test or demo exists

---

## Related Assets

- `src/systems/audio.rs` — contains the commented-out `stress_to_musical_state` call
- `src/resources.rs:GamePhase::Muse` — the dedicated game phase variant
- `ROADMAP.md` Phase 0 — `live-audio` feature flag (requires ALSA headers)
- `docs/GAME_ENGINE_COMPONENTS.md` — "State-coupled audio" row (Symthaea / Planned/P3)
