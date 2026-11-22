use nav_observation_toy_fim::student_t_pl::StudentTProtectionLevels;
use nav_observation_toy_fim::temporal_alignment_envelope::{
    inflate_student_t_protection_levels_by_temporal_bound, maximum_sync_holdover_under_position_budget, temporal_misalignment_envelope,
    MotionEnvelope, TemporalAlignmentBounds,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let motion = MotionEnvelope {
        max_linear_speed_mps: 3.0,
        max_linear_acceleration_mps2: 1.0,
        max_angular_rate_radps: 0.5,
        max_angular_acceleration_radps2: 1.0,
        sensor_lever_arm_m: 0.2,
    };
    for elapsed in [1.0, 10.0, 100.0] {
        let clock = TemporalAlignmentBounds {
            max_abs_offset_s: 0.002,
            max_abs_clock_drift_ppm: 25.0,
            elapsed_since_sync_s: elapsed,
            max_observation_age_s: 0.03,
        };
        let bound = temporal_misalignment_envelope(clock, motion)?;
        let base_pl = StudentTProtectionLevels {
            degrees_of_freedom: 5.0,
            k_tir_2d: 2.5,
            maximum_eigenvalue: 1.0,
            overall_pl_m: 5.0,
            longitudinal_pl_m: 4.0,
            lateral_pl_m: 3.0,
        };
        let inflated = inflate_student_t_protection_levels_by_temporal_bound(base_pl, bound)?;
        println!(
            "elapsed_since_sync_s={elapsed} clock_skew_s={:.9} total_time_bound_s={:.9} position_bias_bound_m={:.9} orientation_bound_rad={:.9} inflated_hpl_m={:.9}",
            bound.clock_offset_component_s + bound.clock_drift_component_s,
            bound.total_time_misalignment_bound_s,
            bound.total_position_misalignment_bound_m,
            bound.angular_misalignment_bound_rad,
            inflated.inflated_overall_pl_m
        );
    }
    let holdover = maximum_sync_holdover_under_position_budget(
        TemporalAlignmentBounds {
            max_abs_offset_s: 0.001,
            max_abs_clock_drift_ppm: 100.0,
            elapsed_since_sync_s: 0.0,
            max_observation_age_s: 0.0,
        },
        MotionEnvelope {
            max_linear_speed_mps: 5.0,
            max_linear_acceleration_mps2: 0.0,
            max_angular_rate_radps: 0.0,
            max_angular_acceleration_radps2: 0.0,
            sensor_lever_arm_m: 0.0,
        },
        0.01,
        100.0,
    )?;
    println!(
        "sync_holdover_budget_m=0.01 maximum_additional_holdover_s={:.9} budget_satisfied_now={} budget_satisfied_at_search_limit={}",
        holdover.maximum_additional_holdover_s,
        holdover.budget_satisfied_now,
        holdover.budget_satisfied_at_search_limit
    );
    println!("scope=synthetic_bounded_timing_sensitivity_not_dataset_calibration_or_integrity_probability");
    Ok(())
}
