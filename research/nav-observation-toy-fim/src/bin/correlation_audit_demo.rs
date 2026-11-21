use std::collections::BTreeMap;

use nav_observation_toy_fim::correlated::CorrelatedRangeAction;
use nav_observation_toy_fim::correlation_audit::{
    audit_correlation_envelope, CorrelationAuditDisposition, CorrelationResidualBlock,
};
use nav_observation_toy_fim::correlation_envelope::{
    CorrelationScenario, PairwiseCorrelation,
};

fn scenarios(low: f64, high: f64) -> Vec<CorrelationScenario> {
    vec![
        CorrelationScenario {
            scenario_id: "low".into(),
            pairs: vec![PairwiseCorrelation {
                action_a: "sensor-a".into(),
                action_b: "sensor-b".into(),
                rho: low,
            }],
        },
        CorrelationScenario {
            scenario_id: "high".into(),
            pairs: vec![PairwiseCorrelation {
                action_a: "sensor-a".into(),
                action_b: "sensor-b".into(),
                rho: high,
            }],
        },
    ]
}

fn blocks(count: usize, hidden_correlation: bool) -> Vec<CorrelationResidualBlock> {
    (0..count)
        .map(|index| {
            let angle = 2.0 * std::f64::consts::PI * index as f64 / count as f64;
            let x = angle.sin();
            let y = if hidden_correlation {
                0.95 * x + 0.05 * (3.0 * angle).sin()
            } else {
                angle.cos()
            };
            CorrelationResidualBlock {
                block_id: format!("holdout-{index:03}"),
                residuals: BTreeMap::from([
                    ("sensor-a".into(), x),
                    ("sensor-b".into(), y),
                ]),
            }
        })
        .collect()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let actions = vec![
        CorrelatedRangeAction::independent("sensor-a", 0.0, 1.0, 1),
        CorrelatedRangeAction::independent("sensor-b", std::f64::consts::FRAC_PI_2, 1.0, 1),
    ];

    let misspecified = audit_correlation_envelope(
        &actions,
        &scenarios(-0.2, 0.2),
        &blocks(100, true),
        0.05,
    )?;
    let consistent = audit_correlation_envelope(
        &actions,
        &scenarios(-0.25, 0.25),
        &blocks(100, false),
        0.05,
    )?;
    let insufficient = audit_correlation_envelope(
        &actions,
        &scenarios(-0.999, 0.999),
        &blocks(3, false),
        0.05,
    )?;

    println!("HELD_OUT_CORRELATION_MODEL_AUDIT");
    assert_eq!(misspecified.disposition, CorrelationAuditDisposition::OutOfModel);
    for (name, audit) in [
        ("hidden_shared_error", &misspecified),
        ("inside_declared_envelope", &consistent),
        ("too_few_independent_blocks", &insufficient),
    ] {
        println!(
            "  {name}: disposition={:?}, pair_rho={:?}, confidence_interval={:?}",
            audit.disposition,
            audit.pair_audits[0].empirical_correlation,
            audit.pair_audits[0].confidence_interval
        );
    }
    Ok(())
}
