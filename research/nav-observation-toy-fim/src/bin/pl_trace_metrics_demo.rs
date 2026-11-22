use nav_observation_toy_fim::pl_trace_metrics::{
    evaluate_protection_level_trace, ProtectionLevelTraceRecord,
};

fn item(id:&str,cluster:&str,timestamp_ms:u64,duration_ms:u64,long:f64,cross:f64)->ProtectionLevelTraceRecord{
    ProtectionLevelTraceRecord{
        record_id:id.into(),cluster_id:cluster.into(),
        timestamp_ns:timestamp_ms*1_000_000,support_duration_ns:duration_ms*1_000_000,
        truth_valid:true,error_longitudinal_m:Some(long),error_cross_track_m:Some(cross),
        protection_level_longitudinal_m:1.0,protection_level_cross_track_m:2.0
    }
}
fn main()->Result<(),Box<dyn std::error::Error>>{
    let records=vec![
        item("loop-a-001","loop-a",0,50,1.2,0.5),
        item("loop-a-002","loop-a",50,50,1.4,0.4),
        item("loop-a-003","loop-a",100,50,0.9,0.4),
        item("loop-b-001","loop-b",150,50,0.8,2.5),
        item("loop-b-002","loop-b",200,50,0.7,2.6)
    ];
    let metrics=evaluate_protection_level_trace(&records)?;
    println!("records={} valid_truth={} clusters={}",metrics.record_count,metrics.valid_truth_samples,metrics.distinct_cluster_count);
    println!("pointwise_exceedance_long_ppm={:?} cross_ppm={:?} either_ppm={:?}",metrics.longitudinal_exceedance_ppm,metrics.cross_track_exceedance_ppm,metrics.either_axis_exceedance_ppm);
    println!("breach_events_long={} cross={} either={}",metrics.longitudinal_breaches.event_count,metrics.cross_track_breaches.event_count,metrics.either_axis_breaches.event_count);
    println!("exposure_above_pl_ns_long={} cross={} either={}",metrics.longitudinal_exposure_above_pl_ns,metrics.cross_track_exposure_above_pl_ns,metrics.either_axis_exposure_above_pl_ns);
    println!("max_contiguous_breach_ns_long={} cross={} either={}",metrics.longitudinal_breaches.maximum_contiguous_duration_ns,metrics.cross_track_breaches.maximum_contiguous_duration_ns,metrics.either_axis_breaches.maximum_contiguous_duration_ns);
    println!("scope=declared_sample_support_intervals_not_continuous_interpolation_or_integrity_probability");
    Ok(())
}
