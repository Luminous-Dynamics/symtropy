use std::time::Instant;

use holofrac_spatial_retrieval::fractal::{
    GeometryFamily, estimate_box_counting_dimension, generate_geometry_fixture,
    standard_resolutions,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source_sha = std::env::var("HOLOFRAC_SOURCE_SHA").unwrap_or_else(|_| "UNSET".to_owned());
    println!("# HoloFrac geometric scale qualification v0");
    println!("# source_sha={source_sha}");
    println!("# target_arch={}", std::env::consts::ARCH);
    println!("# target_os={}", std::env::consts::OS);
    println!("family,points,expected_dimension,estimated_dimension,absolute_error,fit_r2,log_count_rmse,fit_start_resolution,fit_end_resolution,fit_scales,min_window_dimension,max_window_dimension,window_dimension_spread,mean_translation_count_spread,generator_us,scale_profile");

    let resolutions = standard_resolutions();
    for family in GeometryFamily::ALL {
        let generation_started = Instant::now();
        let points = generate_geometry_fixture(family, 65_536, 0x484f_4c4f_4652_4143)?;
        let generator_micros = generation_started.elapsed().as_secs_f64() * 1_000_000.0;
        let report = estimate_box_counting_dimension(&points, &resolutions, 2, 5)?;
        let profile = report
            .scale_counts
            .iter()
            .map(|count| {
                format!(
                    "{}:{:.1}:{}:{}",
                    count.resolution,
                    count.mean_occupied_boxes,
                    count.min_occupied_boxes,
                    count.max_occupied_boxes
                )
            })
            .collect::<Vec<_>>()
            .join(";");

        println!(
            "{},{},{:.6},{:.6},{:.6},{:.6},{:.6},{},{},{},{:.6},{:.6},{:.6},{:.6},{:.3},{}",
            family.as_str(),
            report.point_count,
            family.expected_dimension(),
            report.dimension_estimate,
            (report.dimension_estimate - family.expected_dimension()).abs(),
            report.fit_r_squared,
            report.log_count_rmse,
            report.fit_start_resolution,
            report.fit_end_resolution,
            report.fit_scale_count,
            report.min_window_dimension,
            report.max_window_dimension,
            report.window_dimension_spread,
            report.mean_translation_count_spread,
            generator_micros,
            profile
        );
    }
    Ok(())
}
