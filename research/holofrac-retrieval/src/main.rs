use holofrac_spatial_retrieval::{reference_architecture_graph, run_benchmark};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let graph = reference_architecture_graph()?;
    let source_sha = std::env::var("HOLOFRAC_SOURCE_SHA").unwrap_or_else(|_| "UNSET".to_owned());

    println!("# HoloFrac spatial retrieval benchmark v0");
    println!("# source_sha={source_sha}");
    println!("# target_arch={}", std::env::consts::ARCH);
    println!("# target_os={}", std::env::consts::OS);
    println!("# facts={}", graph.facts().len());
    println!("# graph_digest_non_security={}", graph.digest_non_security()?);
    println!("dimension,facts,positive_queries,bipolar_top1_hits,f32_top1_hits,reversed_queries,bipolar_reversed_false_accepts,f32_reversed_false_accepts,index_build_us,mean_query_us,bipolar_index_bytes_estimate,f32_index_bytes_estimate,encoder_fingerprint_non_security");

    for dimension in [2_048, 4_096, 8_192, 16_384] {
        let report = run_benchmark(&graph, dimension, 0x484f_4c4f_4652_4143)?;
        println!(
            "{},{},{},{},{},{},{},{},{:.3},{:.3},{},{},{}",
            report.dimension,
            report.fact_count,
            report.positive_queries,
            report.positive_top1_hits,
            report.f32_positive_top1_hits,
            report.reversed_queries,
            report.reversed_false_accepts,
            report.f32_reversed_false_accepts,
            report.index_build_micros,
            report.mean_query_micros,
            report.bipolar_index_bytes_estimate,
            report.f32_index_bytes_estimate,
            report.encoder_fingerprint_non_security
        );

        for row in &report.outcomes {
            let top1 = row
                .hdc_top1
                .as_ref()
                .map(|fact| fact.canonical_key())
                .unwrap_or_else(|| "NONE".to_owned());
            let f32_top1 = row
                .f32_top1
                .as_ref()
                .map(|fact| fact.canonical_key())
                .unwrap_or_else(|| "NONE".to_owned());
            println!(
                "# query={},oracle={},metadata={},bipolar_top1={},bipolar_similarity={:.6},bipolar_accept={},f32_top1={},f32_similarity={:.6},f32_accept={}",
                row.query.canonical_key(),
                row.oracle_match,
                row.exact_metadata_match,
                top1,
                row.top1_similarity,
                row.hdc_exact_accept,
                f32_top1,
                row.f32_top1_similarity,
                row.f32_exact_accept
            );
        }
    }
    Ok(())
}
