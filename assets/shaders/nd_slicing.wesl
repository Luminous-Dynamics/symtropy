import bevy_pbr::render::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
};

@if(PREPASS_PIPELINE)
import bevy_pbr::{
    prepass::io::{VertexOutput, FragmentOutput},
    deferred::functions::deferred_output,
};
@else
import bevy_pbr::render::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
};

struct NdSlicingSettings {
    w_pos: f32,
    w_slice: f32,
    slice_thickness: f32,
    edge_fade: f32,
    time: f32,
    _padding: f32,
    _padding2: f32,
    _padding3: f32,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(100)
var<uniform> settings: NdSlicingSettings;

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    let dist = abs(settings.w_pos - settings.w_slice);

    if (dist > settings.slice_thickness) {
        discard;
    }

    // A zero-thickness slice is valid as a geometric plane. Avoid 0/0
    // when the slice is exactly coincident with the body coordinate.
    let alpha_mult = if (
        settings.edge_fade > 0.001 && settings.slice_thickness > 0.000001
    ) {
        clamp(1.0 - (dist / settings.slice_thickness), 0.0, 1.0)
    } else {
        1.0
    };

    var pbr_input = pbr_input_from_standard_material(in, is_front);
    pbr_input.material.base_color.a = pbr_input.material.base_color.a * alpha_mult;
    pbr_input.material.base_color = alpha_discard(
        pbr_input.material,
        pbr_input.material.base_color,
    );

    @if(PREPASS_PIPELINE) {
        let out = deferred_output(in, pbr_input);
        return out;
    } @else {
        var out: FragmentOutput;
        out.color = apply_pbr_lighting(pbr_input);
        out.color = main_pass_post_lighting_processing(pbr_input, out.color);
        return out;
    }
}
