use std::error::Error;

use tesla_patent_claim_audit::{
    EARTH_MEAN_RADIUS_M, TESLA_ESTIMATED_RETURN_INTERVAL_S,
    TESLA_STATED_MEAN_SPEED_KM_PER_S, TESLA_WAVELENGTH_MAX_M,
    TESLA_WAVELENGTH_MIN_M, audit,
};

fn main() -> Result<(), Box<dyn Error>> {
    let result = audit();
    println!("quantity,value,unit,classification");
    println!("patent_stated_mean_speed,{:.8},km/s,patent_assertion", TESLA_STATED_MEAN_SPEED_KM_PER_S);
    println!("patent_estimated_return_interval,{:.8},s,patent_estimate", TESLA_ESTIMATED_RETURN_INTERVAL_S);
    println!("implied_speed_ratio_to_c,{:.10},1,derived_arithmetic", result.speed_ratio_to_c);
    println!("implied_round_trip_distance,{:.3},m,derived_arithmetic", result.implied_round_trip_distance_m);
    println!("mean_earth_circumference,{:.3},m,reference_constant", result.earth_circumference_m);
    println!("path_to_circumference_ratio,{:.10},1,derived_arithmetic", result.path_to_circumference_ratio);
    println!("frequency_at_min_wavelength_claimed_speed,{:.3},Hz,derived_arithmetic", result.frequency_at_min_wavelength_using_claimed_speed_hz);
    println!("frequency_at_max_wavelength_claimed_speed,{:.3},Hz,derived_arithmetic", result.frequency_at_max_wavelength_using_claimed_speed_hz);
    println!("frequency_at_min_wavelength_c,{:.3},Hz,comparison_reference", result.frequency_at_min_wavelength_using_c_hz);
    println!("frequency_at_max_wavelength_c,{:.3},Hz,comparison_reference", result.frequency_at_max_wavelength_using_c_hz);
    println!("quarter_wave_factor_min,{:.6},1,derived_arithmetic", result.quarter_wave_odd_multiple_min);
    println!("quarter_wave_factor_max,{:.6},1,derived_arithmetic", result.quarter_wave_odd_multiple_max);
    println!("earth_mean_radius,{:.3},m,reference_constant", EARTH_MEAN_RADIUS_M);
    println!("wavelength_min,{:.3},m,patent_reported_value", TESLA_WAVELENGTH_MIN_M);
    println!("wavelength_max,{:.3},m,patent_reported_value", TESLA_WAVELENGTH_MAX_M);
    Ok(())
}