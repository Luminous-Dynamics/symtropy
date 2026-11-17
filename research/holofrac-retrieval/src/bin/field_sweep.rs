use holofrac_spatial_retrieval::field::{FieldFamily, run_field_family_benchmark};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source_sha = std::env::var("HOLOFRAC_SOURCE_SHA").unwrap_or_else(|_| "UNSET".to_owned());
    println!("# HoloFrac continuous spatial-field benchmark v0");
    println!("# source_sha={source_sha}");
    println!("# target_arch={}", std::env::consts::ARCH);
    println!("# target_os={}", std::env::consts::OS);
    println!(
        "field_family,dimension,grid_size,samples,held_out_queries,hdc_mae,hdc_rmse,hdc_r2,nearest_mae,nearest_rmse,nearest_r2,bilinear_mae,bilinear_rmse,bilinear_r2,coordinate_encode_us,positional_unbind_us,cleanup_us,query_p50_us,query_p95_us,build_us,retained_vector_payload_bytes,encoder_index_bytes"
    );

    for family in FieldFamily::ALL {
        let dimensions: &[usize] = if family == FieldFamily::Smooth {
            &[2_048, 4_096]
        } else {
            &[2_048]
        };
        let grid_sizes: &[usize] = if family == FieldFamily::Smooth {
            &[4, 8, 16]
        } else {
            &[4, 8]
        };

        for &dimension in dimensions {
            for &grid_size in grid_sizes {
                let report = run_field_family_benchmark(
                    grid_size,
                    dimension,
                    0x484f_4c4f_4649_454c,
                    family,
                )?;
                println!(
                    "{},{},{},{},{},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{},{}",
                    report.field_family.as_str(),
                    report.dimension,
                    report.grid_size,
                    report.sample_count,
                    report.query_count,
                    report.hdc_mae,
                    report.hdc_rmse,
                    report.hdc_r2,
                    report.nearest_mae,
                    report.nearest_rmse,
                    report.nearest_r2,
                    report.bilinear_mae,
                    report.bilinear_rmse,
                    report.bilinear_r2,
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
    }
    Ok(())
}
