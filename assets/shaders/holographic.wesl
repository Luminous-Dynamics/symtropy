import bevy_pbr::render::pbr_fragment::pbr_input_from_standard_material;
import bevy_pbr::render::pbr_functions::{
    alpha_discard,
    apply_pbr_lighting,
    main_pass_post_lighting_processing,
};

@if(PREPASS_PIPELINE) {
    import bevy_pbr::prepass::io::{VertexOutput, FragmentOutput};
    import bevy_pbr::deferred::functions::deferred_output;
}
@else {
    import bevy_pbr::render::forward_io::{VertexOutput, FragmentOutput};
    import bevy_pbr::render::mesh_view_bindings::view;
}

struct HolographicSettings {
    fresnel_color: vec4<f32>,
    fresnel_power: f32,
    scanline_speed: f32,
    scanline_density: f32,
    hologram_alpha: f32,
    time: f32,
    enable_holographic: f32,
    _padding: f32,
    _padding2: f32,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(100)
var<uniform> holographic: HolographicSettings;

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    let world_pos = pbr_input.world_position.xyz;
    let view_pos = view.world_position.xyz;
    let view_dir = normalize(world_pos - view_pos);
    let normal = pbr_input.world_normal;
    let ndotv = abs(dot(normalize(normal), -view_dir));
    let fresnel = pow(1.0 - ndotv, holographic.fresnel_power);

    if (holographic.enable_holographic > 0.5) {
        let scan_pos = world_pos.y * holographic.scanline_density
            + holographic.time * holographic.scanline_speed;
        let scanline = smoothstep(0.4, 0.6, fract(scan_pos));
        let noise = fract(sin(dot(world_pos.xz, vec2<f32>(12.9898, 78.233))) * 43758.5453);
        let scan_mod = mix(1.0, 0.85, scanline * 0.2);
        let noise_mod = mix(0.85, 1.0, noise);

        pbr_input.material.base_color = vec4<f32>(
            pbr_input.material.base_color.rgb * scan_mod * noise_mod,
            pbr_input.material.base_color.a * holographic.hologram_alpha
                * (0.15 + 0.85 * pow(fresnel, 1.5)),
        );
        pbr_input.material.emissive = pbr_input.material.emissive
            + vec4<f32>(holographic.fresnel_color.rgb * fresnel * 1.5, 0.0);
    } else {
        let rim = pow(1.0 - ndotv, 2.0) * 0.3;
        pbr_input.material.emissive = pbr_input.material.emissive
            + vec4<f32>(0.3 * rim, 0.5 * rim, 0.8 * rim, 0.0);
    }

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
