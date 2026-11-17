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
    println!("dimension,facts,positive_queries,positive_top1_hits,reversed_queries,reversed_false_accepts,mean_query_us,bipolar_index_bytes,f32_view_index_bytes,encoder_fingerprint_non_security");

    for dimension in [2_048, 4_096, 8_192, 16_384] {
        let report = run_benchmark(&graph, dimension, 0x484f_4c4f_4652_4143)?;
        println!(
            "{},{},{},{},{},{},{:.3},{},{},{}",
            report.dimension,
            report.fact_count,
            report.positive_queries,
            report.positive_top1_hits,
            report.reversed_queries,
            report.reversed_false_accepts,
            report.mean_query_micros,
            report.bipolar_index_bytes,
            report.continuous_f32_index_bytes,
            report.encoder_fingerprint_non_security
        );

        for row in &report.outcomes {
            let top1 = row
                .hdc_top1
                .as_ref()
                .map(|fact| fact.canonical_key())
                .unwrap_or_else(|| "NONE".to_owned());
            println!(
                "# query={},oracle={},metadata={},top1={},similarity={:.6},accepted={}",
                row.query.canonical_key(),
                row.oracle_match,
                row.exact_metadata_match,
                top1,
                row.top1_similarity,
                row.hdc_exact_accept
            );
        }
    }
    Ok(())
}
