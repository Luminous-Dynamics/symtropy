use holofrac_spatial_retrieval::multichannel::run_multi_channel_benchmark;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source_sha = std::env::var("HOLOFRAC_SOURCE_SHA").unwrap_or_else(|_| "UNSET".to_owned());
    println!("# HoloFrac shared multi-channel field benchmark v0");
    println!("# source_sha={source_sha}");
    println!("# target_arch={}", std::env::consts::ARCH);
    println!("# target_os={}", std::env::consts::OS);
    println!("channels,dimension,grid_size,samples_per_channel,held_out_queries,shared_mae,shared_rmse,independent_mae,independent_rmse,bilinear_mae,bilinear_rmse,shared_build_us,independent_build_us,shared_query_p95_us,independent_query_p95_us,shared_vector_bytes,shared_encoder_index_bytes,shared_channel_name_bytes,independent_vector_bytes,independent_encoder_index_bytes,scalar_grid_value_bytes,shared_total_payload_bytes,independent_total_payload_bytes,shared_to_grid_ratio,shared_to_independent_ratio");

    for dimension in [2_048_usize, 4_096] {
        for grid_size in [4_usize, 8] {
            for channels in [2_usize, 4, 6] {
                let report =
                    run_multi_channel_benchmark(grid_size, dimension, channels, 0x484f_4c4f_4d55_4c54)?;
                println!(
                    "{},{},{},{},{},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.3},{:.3},{:.3},{:.3},{},{},{},{},{},{},{},{},{:.6},{:.6}",
                    report.channels,
                    report.dimension,
                    report.grid_size,
                    report.samples_per_channel,
                    report.held_out_queries,
                    report.shared_mae,
                    report.shared_rmse,
                    report.independent_mae,
                    report.independent_rmse,
                    report.bilinear_mae,
                    report.bilinear_rmse,
                    report.shared_build_micros,
                    report.independent_build_micros,
                    report.shared_query_p95_micros,
                    report.independent_query_p95_micros,
                    report.shared_vector_payload_bytes,
                    report.shared_encoder_index_bytes,
                    report.shared_channel_name_bytes,
                    report.independent_vector_payload_bytes,
                    report.independent_encoder_index_bytes,
                    report.scalar_grid_value_bytes,
                    report.shared_total_payload_bytes,
                    report.independent_total_payload_bytes,
                    report.shared_to_grid_payload_ratio,
                    report.shared_to_independent_payload_ratio
                );
            }
        }
    }
    Ok(())
}
