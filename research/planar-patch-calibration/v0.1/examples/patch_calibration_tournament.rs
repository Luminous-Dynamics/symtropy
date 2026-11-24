// SPDX-License-Identifier: AGPL-3.0-or-later
//! Synthetic known-answer example for planar patch ellipsoid calibration.
//! This is a model smoke test, not physical calibration evidence.

use planar_patch_calibration::{
    BoundaryObservation, CalibrationConfig, MaximumDissipationObservation,
    MaximumDissipationValidationConfig, MaximumDissipationValidationInput, PatchCalibrationInput,
    calibrate_planar_patch, validate_maximum_dissipation,
};

fn observation(id: &str, group: &str, wrench: [f64; 3]) -> BoundaryObservation {
    BoundaryObservation {
        observation_id: id.to_owned(),
        trial_group_id: group.to_owned(),
        measured_wrench: planar_patch_calibration::MeasuredPatchWrench {
            tangent_u_force: wrench[0],
            tangent_v_force: wrench[1],
            normal_torsional_moment: wrench[2],
        },
        capacity_scale: 1.0,
        weight: 1.0,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = PatchCalibrationInput {
        specimen_digest: "synthetic-specimen-001".to_owned(),
        patch_geometry_digest: "synthetic-patch-geometry-001".to_owned(),
        coordinate_frame_digest: "synthetic-patch-frame-001".to_owned(),
        rig_calibration_digest: "synthetic-rig-001".to_owned(),
        observation_profile_digest: "synthetic-three-axis-boundary-profile-001".to_owned(),
        characteristic_length_m: 0.5,
        observations: vec![
            observation("pure-u", "u-trial", [2.0, 0.0, 0.0]),
            observation("pure-v", "v-trial", [0.0, 0.5, 0.0]),
            observation("pure-spin", "spin-trial", [0.0, 0.0, 0.25]),
            observation(
                "mixed",
                "mixed-trial",
                [1.1547005383792515, 0.2886751345948129, 0.14433756729740646],
            ),
        ],
    };

    let report = calibrate_planar_patch(&input, CalibrationConfig::default())?;
    let scales = report.ellipsoid_shape_scales;

    println!(
        "schema,specimen,rank,observations,trial_groups,status,e_u,e_v,e_n_m,max_boundary_residual,weighted_rms,physical_model_validated"
    );
    println!(
        "{},{},{},{},{},{:?},{},{},{},{},{},{}",
        report.schema_version,
        report.specimen_digest,
        report.design_rank,
        report.observation_count,
        report.unique_trial_group_count,
        report.status,
        scales
            .map(|v| format!("{:.9}", v[0]))
            .unwrap_or_else(|| "NA".to_owned()),
        scales
            .map(|v| format!("{:.9}", v[1]))
            .unwrap_or_else(|| "NA".to_owned()),
        scales
            .map(|v| format!("{:.9}", v[2]))
            .unwrap_or_else(|| "NA".to_owned()),
        report
            .maximum_absolute_boundary_residual
            .map(|value| format!("{value:.9}"))
            .unwrap_or_else(|| "NA".to_owned()),
        report
            .weighted_rms_boundary_residual
            .map(|value| format!("{value:.9}"))
            .unwrap_or_else(|| "NA".to_owned()),
        report.physical_model_validated,
    );
    println!("axis,weighted_column_norm,final_projection_residual,relative_projection_residual");
    for axis in report.axis_excitation {
        println!(
            "{:?},{:.9},{:.9},{:.9}",
            axis.axis,
            axis.weighted_column_norm,
            axis.final_projection_residual,
            axis.relative_projection_residual,
        );
    }

    let calibrated_scales = report
        .ellipsoid_shape_scales
        .ok_or_else(|| std::io::Error::other("synthetic boundary data was not identifiable"))?;
    let inverse_root_six = 1.0 / 6.0_f64.sqrt();
    let direction_input = MaximumDissipationValidationInput {
        specimen_digest: input.specimen_digest.clone(),
        patch_geometry_digest: input.patch_geometry_digest.clone(),
        coordinate_frame_digest: input.coordinate_frame_digest.clone(),
        rig_calibration_digest: input.rig_calibration_digest.clone(),
        observation_profile_digest: "synthetic-held-out-sliding-profile-001".to_owned(),
        calibration_parameter_digest: "synthetic-calibration-result-001".to_owned(),
        held_out_split_digest: "synthetic-held-out-trials-001".to_owned(),
        characteristic_length_m: input.characteristic_length_m,
        ellipsoid_shape_scales: calibrated_scales,
        observations: vec![
            MaximumDissipationObservation {
                observation_id: "heldout-mixed".to_owned(),
                trial_group_id: "heldout-mixed-trial".to_owned(),
                slip_twist: [1.0, 2.0, 4.0],
                measured_wrench: planar_patch_calibration::MeasuredPatchWrench {
                    tangent_u_force: -4.0 * inverse_root_six,
                    tangent_v_force: -0.5 * inverse_root_six,
                    normal_torsional_moment: -0.25 * inverse_root_six,
                },
                capacity_scale: 1.0,
                weight: 1.0,
            },
            MaximumDissipationObservation {
                observation_id: "heldout-fast-mixed".to_owned(),
                trial_group_id: "heldout-fast-mixed-trial".to_owned(),
                slip_twist: [5.0, 10.0, 20.0],
                measured_wrench: planar_patch_calibration::MeasuredPatchWrench {
                    tangent_u_force: -4.0 * inverse_root_six,
                    tangent_v_force: -0.5 * inverse_root_six,
                    normal_torsional_moment: -0.25 * inverse_root_six,
                },
                capacity_scale: 1.0,
                weight: 1.0,
            },
            MaximumDissipationObservation {
                observation_id: "heldout-sticking".to_owned(),
                trial_group_id: "heldout-sticking-trial".to_owned(),
                slip_twist: [0.0; 3],
                measured_wrench: planar_patch_calibration::MeasuredPatchWrench {
                    tangent_u_force: 0.1,
                    tangent_v_force: 0.1,
                    normal_torsional_moment: 0.01,
                },
                capacity_scale: 1.0,
                weight: 1.0,
            },
        ],
    };
    let direction_report = validate_maximum_dissipation(
        &direction_input,
        MaximumDissipationValidationConfig::default(),
    )?;
    println!(
        "direction_schema,heldout_split,status,sliding_count,zero_slip_count,max_wrench_error,weighted_rms_error,non_dissipative_count"
    );
    println!(
        "{},{},{:?},{},{},{},{},{}",
        direction_report.schema_version,
        direction_report.held_out_split_digest,
        direction_report.status,
        direction_report.sliding_observation_count,
        direction_report.zero_slip_observation_count,
        direction_report
            .maximum_normalized_wrench_error
            .map(|value| format!("{value:.9}"))
            .unwrap_or_else(|| "NA".to_owned()),
        direction_report
            .weighted_rms_normalized_wrench_error
            .map(|value| format!("{value:.9}"))
            .unwrap_or_else(|| "NA".to_owned()),
        direction_report.non_dissipative_observation_count,
    );
    Ok(())
}
