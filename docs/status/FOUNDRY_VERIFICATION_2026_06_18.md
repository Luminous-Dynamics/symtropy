# Foundry Verification Report - 2026-06-18

## Overview
This report documents the current state of the Symtropy Asset Foundry pipeline, distinguishing between verified production-ready features and experimental/speculative stubs.

## Verified Infrastructure (Production Ready)
- **Asset Registration**: CLI and `registry_manager.py` correctly register processed assets with `role='optimized'`.
- **COALESCE Export**: `export_pack.py` prioritizes optimized versions over raw sources.
- **Physical Fingerprinting**: Blender integration extracts mass, COM, and inertia tensors; these are stored in `assets.sqlite` and injected into Bevy `RigidBody` components.
- **FoundryPlugin**: Bevy plugin automatically wires `_COLLISION` and `_LODn` mesh tags.
- **Just Integration**: Root `justfile` provides `foundry-status`, `foundry-ingest`, `foundry-convert`, and `foundry-export`.
- **Nix Support**: `flake.nix` includes `jsonschema` for manifest validation.

## Verified Assets
- `symtropy.env.seedworks.wetland.node.0001` (Mycelial Resonant Node)
- `symtropy.test.node.0002` (Test Mycelial Node)

## Experimental / Speculative (Quarantined)
The following modules were implemented as architectural stubs or research prototypes. They are currently excluded from the stable build path to maintain focus on Seedworks v0.1:
- `evolution`: Stochastic blueprint mutation (needs deterministic seed-based replacement).
- `fitness`: Simplified success metrics for world-lines.
- `neural-link`: Placeholder for LLM blueprint generation.
- `cognition`: Prototype for simulation self-optimization.
- `bio-feedback`: Research into biometric coupling.
- `ghost-memory`: Prototype for historical pattern synthesis.
- `mesh-synthesis`: Experimental procedural geometry generation (requires Bevy 0.18.1 path fixes).

## Known Issues & Risks
- **Non-Determinism**: The `evolution` stub used `thread_rng()`, which violates Symtropy's requirement for reproducible world seeds.
- **Workspace Dependencies**: `symthaea` crate has a conflicting `libsqlite3-sys` dependency in the current workspace.
- **Warning Debt**: `symtropy-foundry` crate currently generates warnings on build.

## Roadmap to Seedworks v0.1
1.  **D-Warnings**: Enforce `D-warnings` across the Foundry crate.
2.  **Deterministic Evolution**: Replace stochastic stubs with a seed-based causality engine.
3.  **End-to-End Proof**: Verify a full "Patch Conduit" repair loop as specified in the crafting design docs.
