# Isolated Bevy 0.20 graphics probe

This package is an **isolated migration spike**, not the production Symtropy workspace upgrade. It is deliberately outside the root Cargo workspace and has no dependencies on Symtropy's Bevy 0.19 crates. The goal is to compile a small real PBR scene against Bevy 0.20 and exercise the new built-in pan-orbit camera without introducing duplicate Bevy ECS/render types into production applications.

## Requirements

- Rust 1.97.1 or newer, matching the Bevy 0.20.0 manifest's declared minimum.
- A desktop graphics environment for running the scene interactively.
- Internet access on the first Cargo build to retrieve Bevy 0.20.0 dependencies.

The separate package has its own Cargo manifest and resolver-3 workspace. It does not modify the root workspace's Bevy 0.19 version constraint or lockfile.

## What the probe exercises

- Bevy 0.20 PBR scene APIs: `Camera3d`, `Mesh3d`, `MeshMaterial3d`, `StandardMaterial`, directional-light shadow configuration.
- Built-in `DefaultPanOrbitCameraPlugins` and `PanOrbitCamera` for interactive view inspection.
- Deterministic, asset-free grid geometry with eight shared material profiles.
- Quality-like workload controls: 1–10,000 objects and explicit shadow enable/disable.
- Minimal unit tests for the fixture's grid-framing and object-count contract.

## Run

From repository root:

```sh
cd research/bevy-020-graphics-probe
cargo +1.97.1 test --bin symtropy-bevy-020-graphics-probe
cargo +1.97.1 run --bin symtropy-bevy-020-graphics-probe
```

Environment overrides:

- `SYMTROPY_BEVY020_PROBE_OBJECTS`: integer in 1–10,000 (default 64).
- `SYMTROPY_BEVY020_PROBE_SHADOWS`: `on` / `off` (default on).

## Migration boundaries

This probe does not prove Bevy 0.20 is integrated into Symtropy. Production migration still requires:
- a compatible Lightyear release or a reviewed port;
- coordinated versions for `bevy_egui`, `bevy-inspector-egui`, and `bevy-tokio-tasks`;
- WESL/custom-shader, `ShaderBuffer`, 2D material, and changed API migrations;
- an updated root lockfile and exact-head workspace checks.

The probe's purpose is to discover source/API and rendering issues in parallel while the main workspace keeps its single Bevy 0.19 version. Only promote code into production after compatibility and regression gates are met.
