#!/usr/bin/env python3
"""Apply the exact Terrain authority dependency-boundary hardening.

This transformer is intentionally fail-closed: every predecessor marker must
exist exactly once. It removes two currently unnecessary authority couplings
and repairs the mixed-WGPU Naga 27 diagnostic feature mismatch at the bridge
that actually combines the two graphics stacks.
"""

from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    file_path = Path(path)
    text = file_path.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(
            f"{path}: expected exactly one predecessor marker, found {count}: {old!r}"
        )
    file_path.write_text(text.replace(old, new, 1))


replace_once(
    "crates/domains/symtropy-terrain/Cargo.toml",
    'symtropy-physics-gpu = { path = "../../domains/symtropy-physics-gpu" }\n',
    "",
)
replace_once(
    "crates/domains/symtropy-terrain/src/lib.rs",
    "use symtropy_physics_gpu::HybridFluidPlugin;\n\n",
    "",
)
replace_once(
    "crates/domains/symtropy-terrain/src/lib.rs",
    "        app.add_plugins(HybridFluidPlugin)\n            .register_type::<EarthChunk>()\n",
    "        app.register_type::<EarthChunk>()\n",
)
replace_once(
    "crates/bridges/symtropy-rapier3d-bridge/Cargo.toml",
    'symthaea-bevy-brain = { path = "../../bridges/symthaea-bevy-brain" }\n',
    "",
)

brain_manifest = Path("crates/bridges/symthaea-bevy-brain/Cargo.toml")
brain_text = brain_manifest.read_text()
brain_marker = (
    'symthaea = { path = "../../../stubs/symthaea", default-features = false, '
    'features = ["vision-manifold", "muse", "reasoning_engine"] }\n'
)
if brain_text.count(brain_marker) != 1:
    raise SystemExit("symthaea-bevy-brain: unexpected exact manifest predecessor")
brain_insertion = brain_marker + (
    "# Cargo feature-coherence shim: this bridge intentionally combines Bevy/WGPU 29\n"
    "# with Symthaea-core/WGPU 27. Bevy's naga_oil activates codespan-reporting\n"
    "# 0.12 termcolor, so Naga 27 must select its matching diagnostic writer mode.\n"
    'naga27-coherence = { package = "naga", version = "27", default-features = false, features = ["termcolor"] }\n'
)
brain_manifest.write_text(brain_text.replace(brain_marker, brain_insertion, 1))

print("Terrain authority dependency-boundary transform applied")
