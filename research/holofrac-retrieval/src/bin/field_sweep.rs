use holofrac_spatial_retrieval::field::run_regular_grid_field_benchmark;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source_sha = std::env::var("HOLOFRAC_SOURCE_SHA").unwrap_or_else(|_| "UNSET".to_owned());
    println!("# HoloFrac continuous spatial-field benchmark v0");
    println!("# source_sha={source_sha}");
    println!("# target_arch={}", std::env::consts::ARCH);
    println!("# target_os={}", std::env::consts::OS);
    println!("dimension,grid_size,samples,held_out_queries,hdc_mae,hdc_rmse,nearest_mae,nearest_rmse,bilinear_mae,bilinear_rmse,coordinate_encode_us,positional_unbind_us,cleanup_us,query_p50_us,query_p95_us,build_us,retained_vector_payload_bytes,encoder_index_bytes");

    for dimension in [2_048_usize, 4_096] {
        for grid_size in [4_usize, 8, 16] {
            let report = run_regular_grid_field_benchmark(
                grid_size,
                dimension,
                0x484f_4c4f_4649_454c,
            )?;
            println!(
                "{},{},{},{},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{},{}",
                report.dimension,
                report.grid_size,
                report.sample_count,
                report.query_count,
                report.hdc_mae,
                report.hdc_rmse,
                report.nearest_mae,
                report.nearest_rmse,
                report.bilinear_mae,
                report.bilinear_rmse,
                report.mean_coordinate_encode_micros,
                report.mean_positional_unbind_micros,
                report.mean_cleanup_micros,
                report.query_p50_micros,
                report.query_p95_micros,
                report.build_micros,
                report.retained_vector_payload_bytes_estimate,
                report.encoder_index_bytes_estimate
            );
        }
    }
    Ok(())
}
