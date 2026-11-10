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
}

// This order and layout intentionally mirrors NodeTelemetryGpu in
// symtropy-render-bridge/src/telemetry_ssbo.rs (64 bytes, 16 x 4-byte slots).
struct NodeTelemetryGpu {
    position: vec3<f32>,
    variational_free_energy: f32,
    bandwidth_bps: f32,
    latency_ms: f32,
    tunnel_state: u32,
    dht_holding_completeness: f32,
    gossip_frequency_hz: f32,
    validation_failure_count: u32,
    wasm_memory_fraction: f32,
    last_hot_reload_time: f32,
    holographic_coherence: f32,
    thermal_gradient: f32,
    circuit_load: f32,
    padding: f32,
};

struct TelemetrySettings {
    node_index: u32,
    time: f32,
    deformation_scale: f32,
    noise_frequency: f32,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(100)
var<uniform> telemetry: TelemetrySettings;

@group(constants::MATERIAL_BIND_GROUP) @binding(101)
var<storage, read> nodes: array<NodeTelemetryGpu>;

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    var activity = 0.0;
    var coherence = 0.0;
    var integrity = 0.0;

    let node_count = arrayLength(&nodes);
    if (node_count > 0u && telemetry.node_index < node_count) {
        let node = nodes[telemetry.node_index];
        let noise_gain = max(telemetry.noise_frequency, 0.0);
        activity = clamp(
            abs(node.variational_free_energy) * noise_gain
                + clamp(node.circuit_load, 0.0, 1.0) * 0.35
                + clamp(node.thermal_gradient / 100.0, 0.0, 1.0) * 0.15,
            0.0,
            1.0,
        );
        coherence = clamp(node.holographic_coherence, 0.0, 1.0);
        integrity = clamp(node.dht_holding_completeness, 0.0, 1.0);
    }

    let phase = 0.5 + 0.5 * sin(telemetry.time * 2.0 + f32(telemetry.node_index) * 0.17);
    let strength = clamp(telemetry.deformation_scale, 0.0, 4.0);
    let stress_tint = vec3<f32>(1.0, 0.18, 0.06);
    let coherence_tint = vec3<f32>(0.08, 0.82, 0.96);
    let integrity_tint = vec3<f32>(0.22, 0.88, 0.42);

    let target_tint = mix(
        mix(stress_tint, coherence_tint, coherence),
        integrity_tint,
        integrity * 0.25,
    );
    let modulation = clamp(activity * (0.65 + 0.35 * phase) * strength, 0.0, 1.0);

    pbr_input.material.base_color = vec4<f32>(
        mix(pbr_input.material.base_color.rgb, target_tint, modulation * 0.55),
        pbr_input.material.base_color.a,
    );
    pbr_input.material.emissive = pbr_input.material.emissive
        + vec4<f32>(target_tint * modulation * 0.8, 0.0);

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
