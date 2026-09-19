#!/usr/bin/env python3
from pathlib import Path

path = Path("crates/domains/symtropy-terrain/src/lib.rs")
text = path.read_text()

module_anchor = "use symtropy_physics_gpu::HybridFluidPlugin;\n\npub const CHUNK_SIZE: usize = 16;"
module_replacement = "use symtropy_physics_gpu::HybridFluidPlugin;\n\nmod geometry_authority;\npub use geometry_authority::*;\n\npub const CHUNK_SIZE: usize = 16;"
if text.count(module_anchor) != 1:
    raise SystemExit("expected exactly one Terrain geometry module anchor")
text = text.replace(module_anchor, module_replacement, 1)

register_anchor = ".register_type::<EarthChunk>()\n            .add_message::<ExcavationEvent>()"
register_replacement = ".register_type::<EarthChunk>()\n            .register_type::<EarthChunkLatticeCoord>()\n            .add_message::<ExcavationEvent>()"
if text.count(register_anchor) != 1:
    raise SystemExit("expected exactly one EarthChunk registration anchor")
text = text.replace(register_anchor, register_replacement, 1)

path.write_text(text)
print("Terrain exact geometry authority integration transform applied")
