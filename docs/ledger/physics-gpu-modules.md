# Ledger: Physics GPU — Render & Fluid Modules

**Status:** archived placeholder — feature-gated behind `experimental-gpu`  
**Archived:** 2026-06-10  
**Archived by:** automated stabilisation pass

---

## Original Intent

`symtropy-physics-gpu` was to be the GPU-accelerated path for two subsystems:

### `render.rs` — GPU Render Bridge

A WGPU / Bevy render-world integration that would:
- Upload `BroadphaseCell` AABBs to a GPU storage buffer per-frame
- Render collider wireframes as a Bevy `RenderPhase` via a custom pipeline
- Overlay the Φ field as a heat-map via a compute shader + texture blit
- Support both 3D (camera-space projection) and 4D (projected-to-3D first)

Intended to replace the current CPU-side `ConsciousnessVisuals` in
`src/systems/postprocess.rs` with a fully GPU-resident pass.

### `fluid.rs` — GPU Fluid Simulation

A compute-shader SPH (Smoothed-Particle Hydrodynamics) layer that would:
- Run a position-based fluid solver entirely on GPU
- Couple fluid pressure to the `ConsciousnessField` via a density→Φ channel
- Export a per-cell temperature buffer that feeds `ThermodynamicLedger`
- Support N-dimensional projection (3D slice of ND fluid)

This would have been the first experimental coupling between classical fluid
dynamics and the IIT-derived Φ field.

---

## What Was Removed From Active Compile Path

| File | Change |
|------|--------|
| `crates/symtropy-physics-gpu/src/lib.rs` | `pub mod render;` → `#[cfg(feature = "experimental-gpu")] pub mod render;` |
| `crates/symtropy-physics-gpu/src/lib.rs` | `pub mod fluid;` → `#[cfg(feature = "experimental-gpu")] pub mod fluid;` |

---

## Reactivation Conditions

1. `crates/symtropy-physics-gpu/src/render.rs` is written with at minimum
   a placeholder `GpuRenderPlugin` struct
2. `crates/symtropy-physics-gpu/src/fluid.rs` is written with at minimum
   a placeholder `GpuFluidPlugin` struct
3. Feature flag `experimental-gpu` exists in `symtropy-physics-gpu/Cargo.toml`
4. `cargo check -p symtropy-physics-gpu --features experimental-gpu` passes

---

## Related Assets

- `crates/symtropy-physics-gpu/src/lib.rs` — existing skeleton (accelerator trait, BroadphaseAccelerator)
- `crates/symtropy-physics-gpu/src/accel.rs` — GPU acceleration trait stubs
- Bevy 0.18 Render World documentation — required reading for implementation
- `ARCHITECTURE.md` — "GPU Acceleration" extension point
