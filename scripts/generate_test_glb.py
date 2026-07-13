import bpy
import os

# Create a simple cube
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.mesh.primitive_cube_add(location=(0, 0, 0))
obj = bpy.context.active_object
obj.name = "TestNode"

# Export as GLB
export_path = "/srv/luminous-dynamics/symtropy/assets/symtropy-foundry-data/raw_vault/seedworks_wetland_v0/mycelial_node.glb"
os.makedirs(os.path.dirname(export_path), exist_ok=True)
bpy.ops.export_scene.gltf(filepath=export_path, export_format='GLB')
print(f"Generated test GLB at {export_path}")
