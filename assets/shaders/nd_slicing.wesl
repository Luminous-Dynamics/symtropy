import bevy_pbr::render::pbr_fragment::pbr_input_from_standard_material;
import bevy_pbr::render::pbr_functions::alpha_discard;

@if(!PREPASS_PIPELINE) {
    import bevy_pbr::render::pbr_functions::{
        apply_pbr_lighting,
        main_pass_post_lighting_processing,
    };
}

@if(PREPASS_PIPELINE) {
    import bevy_pbr::prepass::io::{VertexOutput, FragmentOutput};
    import bevy_pbr::deferred::functions::deferred_output;
}
@else {
    import bevy_pbr::render::forward_io::{VertexOutput, FragmentOutput};
    import bevy_pbr::render::mesh_view_bindings::view;
}

struct NdSlicingSettings {
    w_pos: f32,
    w_slice: f32,
    slice_thickness: f32,
    edge_fade: f32,
    time: f32,
    phi_global: f32,
    surprise_global: f32,
    harmony_global: f32,
    energy_level: f32,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(100)
var<uniform> settings: NdSlicingSettings;

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    let dist = abs(settings.w_pos - settings.w_slice);

    // A zero-width slice is a valid exact section, not a division-by-zero case.
    if (settings.slice_thickness <= 0.000001) {
        if (dist > 0.000001) {
            discard;
        }
    } else if (dist > settings.slice_thickness) {
        discard;
    }

    var alpha_mult = 1.0;
    if (settings.edge_fade > 0.001 && settings.slice_thickness > 0.000001) {
        alpha_mult = clamp(1.0 - (dist / settings.slice_thickness), 0.0, 1.0);
    }

    var pbr_input = pbr_input_from_standard_material(in, is_front);
    pbr_input.material.base_color.a = pbr_input.material.base_color.a * alpha_mult;
    pbr_input.material.base_color = alpha_discard(
        pbr_input.material.flags,
        pbr_input.material.alpha_cutoff,
        pbr_input.material.base_color,
    );

    @if(PREPASS_PIPELINE) {
        return deferred_output(in, pbr_input);
    }
    @else {
        var out: FragmentOutput;
        out.color = apply_pbr_lighting(pbr_input);
        out.color = main_pass_post_lighting_processing(pbr_input, out.color);
        return out;
    }
}
