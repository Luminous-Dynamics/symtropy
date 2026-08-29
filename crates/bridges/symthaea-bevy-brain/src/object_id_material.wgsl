#import bevy_pbr::forward_io::VertexOutput

struct ObjectIdMaterial {
    encoded_id: vec4<f32>,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> material: ObjectIdMaterial;

@fragment
fn fragment(_mesh: VertexOutput) -> @location(0) vec4<f32> {
    return material.encoded_id;
}
