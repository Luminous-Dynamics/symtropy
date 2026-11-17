use holofrac_spatial_retrieval::partial::PartialSpatialQuery;
use holofrac_spatial_retrieval::{
    reference_architecture_graph, run_benchmark, EncoderSpec, HoloFracEncoder, RelationKind,
    SpatialIndex,
};

fn metric(value: Option<f64>) -> String {
    value
        .map(|number| format!("{number:.3}"))
        .unwrap_or_else(|| "NA".to_owned())
}

fn run_partial_queries(
    graph: &holofrac_spatial_retrieval::SpatialGraph,
    dimension: usize,
    seed: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let encoder = HoloFracEncoder::new(EncoderSpec {
        schema_version: 2,
        dimension,
        seed,
        scalar_levels: 257,
    })?;
    let (index, index_build_micros) = SpatialIndex::build(graph, &encoder)?;
    println!("# partial_index_build_us={index_build_micros:.3}");

    let queries = [
        (
            "subject_relation",
            PartialSpatialQuery::new(
                Some("water-main".into()),
                Some(RelationKind::Supplies),
                None,
            ),
        ),
        (
            "relation_object",
            PartialSpatialQuery::new(
                None,
                Some(RelationKind::Supplies),
                Some("kitchen".into()),
            ),
        ),
        (
            "subject_object",
            PartialSpatialQuery::new(
                Some("water-main".into()),
                None,
                Some("bathroom".into()),
            ),
        ),
        (
            "no_answer_control",
            PartialSpatialQuery::new(
                Some("water-main".into()),
                Some(RelationKind::DrainsTo),
                None,
            ),
        ),
    ];

    println!("# partial_query,labelled_fields,exact_matches,abstained,slot_precision_at_5,slot_recall_at_5,slot_mrr_at_5,slot_false_suggestions,holistic_precision_at_5,holistic_recall_at_5,holistic_mrr_at_5,holistic_false_suggestions,encoding_us,slot_rank_us,holistic_rank_us,exact_oracle_us");
    for (label, query) in queries {
        let report = index.rank_partial_query(graph, &encoder, &query, 5)?;
        println!(
            "# partial_query={},{},{},{},{:.3},{},{},{},{:.3},{},{},{},{:.3},{:.3},{:.3},{:.3}",
            label,
            report.query_slot_count,
            report.exact_match_count,
            report.abstained,
            report.slot_metrics.precision_at_k,
            metric(report.slot_metrics.recall_at_k),
            metric(report.slot_metrics.reciprocal_rank),
            report.slot_metrics.false_suggestions_at_k,
            report.holistic_metrics.precision_at_k,
            metric(report.holistic_metrics.recall_at_k),
            metric(report.holistic_metrics.reciprocal_rank),
            report.holistic_metrics.false_suggestions_at_k,
            report.query_encoding_micros,
            report.slot_ranking_micros,
            report.holistic_ranking_micros,
            report.exact_oracle_micros
        );
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let graph = reference_architecture_graph()?;
    let source_sha = std::env::var("HOLOFRAC_SOURCE_SHA").unwrap_or_else(|_| "UNSET".to_owned());

    println!("# HoloFrac spatial retrieval benchmark v0");
    println!("# source_sha={source_sha}");
    println!("# target_arch={}", std::env::consts::ARCH);
    println!("# target_os={}", std::env::consts::OS);
    println!("# facts={}", graph.facts().len());
    println!(
        "# graph_digest_non_security={}",
        graph.digest_non_security()?
    );
    println!(
        "dimension,facts,positive_queries,bipolar_top1_hits,f32_top1_hits,reversed_queries,bipolar_reversed_false_accepts,f32_reversed_false_accepts,index_build_us,query_encode_us,bipolar_search_us,f32_search_us,bipolar_end_to_end_us,f32_end_to_end_us,exact_oracle_us,metadata_lookup_us,bipolar_only_payload_bytes,f32_only_payload_bytes,dual_view_payload_bytes,encoder_fingerprint_non_security"
    );

    for dimension in [2_048, 4_096, 8_192, 16_384] {
        let report = run_benchmark(&graph, dimension, 0x484f_4c4f_4652_4143)?;
        println!(
            "{},{},{},{},{},{},{},{},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{},{},{},{}",
            report.dimension,
            report.fact_count,
            report.positive_queries,
            report.positive_top1_hits,
            report.f32_positive_top1_hits,
            report.reversed_queries,
            report.reversed_false_accepts,
            report.f32_reversed_false_accepts,
            report.index_build_micros,
            report.mean_query_encode_micros,
            report.mean_bipolar_search_micros,
            report.mean_f32_search_micros,
            report.mean_bipolar_end_to_end_micros,
            report.mean_f32_end_to_end_micros,
            report.mean_exact_oracle_micros,
            report.mean_metadata_lookup_micros,
            report.bipolar_index_bytes_estimate,
            report.f32_index_bytes_estimate,
            report.dual_view_index_bytes_estimate,
            report.encoder_fingerprint_non_security
        );

        for row in &report.outcomes {
            let bipolar_top1 = row
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
                bipolar_top1,
                row.top1_similarity,
                row.hdc_exact_accept,
                f32_top1,
                row.f32_top1_similarity,
                row.f32_exact_accept
            );
        }

        run_partial_queries(&graph, dimension, 0x5041_5254_4941_4c)?;
    }
    Ok(())
}
