// SPDX-License-Identifier: AGPL-3.0-or-later
use planar_patch_parameter_set::{
    BoundedBoundaryObservation, ParameterSetConfig, ParameterSetInput, ParameterSetStatus,
    compute_parameter_set,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = ParameterSetInput {
        specimen_digest: "synthetic-specimen-parameter-set-001".to_owned(),
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

    let report = compute_parameter_set(&input, ParameterSetConfig::default())?;
    if report.status != ParameterSetStatus::FeasibleWithFiniteIntervals {
        return Err(format!("unexpected synthetic fixture status: {:?}", report.status).into());
    }

    println!(
        "schema,status,excited_axes,observations,trial_groups,vertices,plane_triples,\
u_beta_low,u_beta_high,v_beta_low,v_beta_high,n_beta_low,n_beta_high"
    );
    println!(
        "{},{:?},{},{},{},{},{},{:.9},{:.9},{:.9},{:.9},{:.9},{:.9}",
        report.schema_version,
        report.status,
        report
            .intervals
            .iter()
            .filter(|interval| interval.axis_was_excited)
            .count(),
        report.observation_count,
        report.unique_trial_group_count,
        report.feasible_vertex_count,
        report.plane_triples_checked,
        report.intervals[0].beta_lower.unwrap_or(f64::NAN),
        report.intervals[0].beta_upper.unwrap_or(f64::NAN),
        report.intervals[1].beta_lower.unwrap_or(f64::NAN),
        report.intervals[1].beta_upper.unwrap_or(f64::NAN),
        report.intervals[2].beta_lower.unwrap_or(f64::NAN),
        report.intervals[2].beta_upper.unwrap_or(f64::NAN),
    );

    println!("axis,shape_lower_m_or_unitless,shape_upper_m_or_unitless");
    for (name, interval) in ["u", "v", "torsion"]
        .into_iter()
        .zip(report.intervals.iter())
    {
        println!(
            "{name},{},{}",
            interval.shape_scale_lower.map_or_else(|| "unknown".to_owned(), |v| format!("{v:.9}")),
            interval.shape_scale_upper.map_or_else(|| "unbounded".to_owned(), |v| format!("{v:.9}")),
        );
    }
    Ok(())
}
