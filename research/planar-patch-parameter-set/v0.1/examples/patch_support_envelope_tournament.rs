// SPDX-License-Identifier: AGPL-3.0-or-later
use planar_patch_parameter_set::{
    BoundedBoundaryObservation, ParameterSetConfig, ParameterSetInput,
    PatchSupportEnvelopeInput, SupportEnvelopeConfig, compute_parameter_set,
    compute_patch_support_envelope,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let parameter_input = ParameterSetInput {
        specimen_digest: "synthetic-specimen-support-001".to_owned(),
        patch_geometry_digest: "synthetic-planar-patch-001".to_owned(),
        coordinate_frame_digest: "synthetic-patch-frame-001".to_owned(),
        rig_calibration_digest: "synthetic-rig-001".to_owned(),
        observation_profile_digest: "synthetic-three-axis-boundary-profile".to_owned(),
        capacity_convention_digest: "mu-normal-load-characteristic-length-v1".to_owned(),
        characteristic_length_m: 0.5,
        observations: vec![
            BoundedBoundaryObservation {
                observation_id: "u-boundary".to_owned(),
                trial_group_id: "u-trial".to_owned(),
                normalized_boundary_components: [2.0, 0.0, 0.0],
                normalized_residual_bound: 0.0,
            },
            BoundedBoundaryObservation {
                observation_id: "v-boundary".to_owned(),
                trial_group_id: "v-trial".to_owned(),
                normalized_boundary_components: [0.0, 0.5, 0.0],
                normalized_residual_bound: 0.0,
            },
            BoundedBoundaryObservation {
                observation_id: "spin-boundary".to_owned(),
                trial_group_id: "spin-trial".to_owned(),
                normalized_boundary_components: [0.0, 0.0, 0.25],
                normalized_residual_bound: 0.0,
            },
        ],
    };

    let parameter_set = compute_parameter_set(&parameter_input, ParameterSetConfig::default())?;
    let envelope = compute_patch_support_envelope(
        &PatchSupportEnvelopeInput {
            parameter_set: &parameter_set,
            capacity_scale: 20.0,
            slip_twist: [0.1, 0.0, 0.0],
        },
        SupportEnvelopeConfig::default(),
    )?;

    println!("schema,status,vertices,active_axes,lower_bound_W,upper_bound_W,upper_exact");
    println!(
        "{},{:?},{},{},{:.9},{},{}",
        envelope.schema_version,
        envelope.status,
        envelope.feasible_vertex_count,
        envelope.active_axis_count,
        envelope.support_lower_bound.unwrap_or(f64::NAN),
        envelope
            .support_upper_bound
            .map_or_else(|| "unbounded".to_owned(), |value| format!("{value:.9}")),
        envelope.upper_bound_is_exact_over_feasible_set,
    );
    Ok(())
}
