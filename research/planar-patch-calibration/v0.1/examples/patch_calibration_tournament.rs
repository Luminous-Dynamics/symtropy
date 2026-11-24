// SPDX-License-Identifier: AGPL-3.0-or-later
//! Synthetic known-answer example for planar patch ellipsoid calibration.
//! This is a model smoke test, not physical calibration evidence.

use planar_patch_calibration::{
    BoundaryObservation, CalibrationConfig, PatchCalibrationInput, calibrate_planar_patch,
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
    Ok(())
}
