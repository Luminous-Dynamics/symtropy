use nav_observation_toy_fim::ds_twr_integrity::{correct_positive_nlos_bias_interval, ds_twr_range_interval, DsTwrDurationInput, PositiveNlosBiasBound};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base = DsTwrDurationInput {
        round_trip_term_s: 300.0e-9,
        round_trip_abs_error_bound_s: 0.5e-9,
        clock_scaled_response_term_s: 300.0e-9,
        clock_scaled_response_abs_error_bound_s: 0.5e-9,
        clock_scale_reference_term_s: 300.0e-9,
        clock_scale_reference_abs_error_bound_s: 0.5e-9,
        turnaround_term_s: 100.0e-9,
        turnaround_abs_error_bound_s: 0.5e-9,
    };
    for multiplier in [0.0, 1.0, 2.0, 5.0] {
        let mut input = base;
        input.round_trip_abs_error_bound_s *= multiplier;
        input.clock_scaled_response_abs_error_bound_s *= multiplier;
        input.clock_scale_reference_abs_error_bound_s *= multiplier;
        input.turnaround_abs_error_bound_s *= multiplier;
        let range = ds_twr_range_interval(input)?;
        println!(
            "timestamp_error_multiplier={multiplier} nominal_range_m={:.6} lower_m={:.6} upper_m={:.6} interval_width_m={:.6} clock_ratio=[{:.9},{:.9}]",
            range.nominal_range_m,
            range.lower_bound_m,
            range.upper_bound_m,
            range.upper_bound_m - range.lower_bound_m,
            range.clock_ratio_lower,
            range.clock_ratio_upper
        );
    }
    let observed = ds_twr_range_interval(base)?;
    let nlos = correct_positive_nlos_bias_interval(
        observed,
        PositiveNlosBiasBound {
            minimum_bias_m: 0.0,
            maximum_bias_m: 1.5,
        },
    )?;
    println!(
        "nlos_bias_bound_m=[0.0,1.5] observed_range_m={:.6} possible_true_range_m=[{:.6},{:.6}]",
        observed.nominal_range_m,
        nlos.lower_bound_m,
        nlos.upper_bound_m
    );
    println!("scope=equation_level_interval_arithmetic_not_miluv_timestamp_parsing_or_range_calibration");
    Ok(())
}
