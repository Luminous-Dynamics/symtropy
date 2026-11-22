use nav_observation_toy_fim::risk_coverage::{
    risk_coverage_curve, EvaluationCase, ServiceLoss,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cases = vec![
        EvaluationCase {
            record_id: "clear-correct".to_owned(),
            confidence_ppm: 980_000,
            claim_state_one: true,
            truth_state_one: true,
            exposure_seconds: 3600,
        },
        EvaluationCase {
            record_id: "high-confidence-false-safe".to_owned(),
            confidence_ppm: 940_000,
            claim_state_one: true,
            truth_state_one: false,
            exposure_seconds: 600,
        },
        EvaluationCase {
            record_id: "moderate-confidence-false-degraded".to_owned(),
            confidence_ppm: 720_000,
            claim_state_one: false,
            truth_state_one: true,
            exposure_seconds: 900,
        },
        EvaluationCase {
            record_id: "low-confidence-correct".to_owned(),
            confidence_ppm: 250_000,
            claim_state_one: false,
            truth_state_one: false,
            exposure_seconds: 1800,
        },
    ];
    let loss = ServiceLoss {
        false_safe_loss_micros_per_second: 100,
        false_degraded_loss_micros_per_second: 1,
    };

    for point in risk_coverage_curve(&cases, loss)? {
        println!(
            "threshold_ppm={} record_coverage_ppm={} exposure_coverage_ppm={} false_safe_records={} false_degraded_records={} false_safe_rate_ppm={:?} false_safe_wilson_upper_95_ppm={:?} weighted_loss_per_covered_hour_micros={:?}",
            point.threshold_ppm,
            point.record_coverage_ppm,
            point.exposure_coverage_ppm,
            point.false_safe_records,
            point.false_degraded_records,
            point.false_safe_rate_ppm,
            point.false_safe_wilson_upper_95_ppm,
            point.weighted_loss_per_covered_hour_micros
        );
    }
    println!("scope=offline_evaluator_metric_not_integrity_certificate");
    Ok(())
}
