use std::collections::BTreeMap;
use std::hint::black_box;
use std::time::{Duration, Instant};

use construction_function_first_optioneering_core::{
    DeliveryOption, FunctionalBrief, GateResult, MetricKey, OptionTier, ScenarioMetrics,
    evaluate_options,
};

fn brief() -> FunctionalBrief {
    FunctionalBrief {
        brief_id: "scale-benchmark-brief".to_owned(),
        revision_digest: "brief-revision-scale".to_owned(),
        required_gate_ids: vec![
            "functional-service-adequacy".to_owned(),
            "structural-adequacy".to_owned(),
            "fire-safety".to_owned(),
            "accessibility".to_owned(),
        ],
        scenario_ids: vec![
            "normal".to_owned(),
            "supply-delay".to_owned(),
            "maintenance-stress".to_owned(),
        ],
        analysis_horizon_months: 600,
        discount_rate_basis_points: 350,
        currency_code: "ZAR".to_owned(),
        benchmark_profile_id: "OPTIONING-SCALE-001".to_owned(),
        benchmark_profile_digest: "git-blob-sha1:scale-profile".to_owned(),
        fixture_id: "OPTIONING-SCALE-FIXTURE-001".to_owned(),
        fixture_digest: "git-blob-sha1:scale-fixture".to_owned(),
        claim_ceiling: "synthetic_pareto_scale_only".to_owned(),
    }
}

fn gate_results() -> BTreeMap<String, GateResult> {
    [
        "functional-service-adequacy",
        "structural-adequacy",
        "fire-safety",
        "accessibility",
    ]
    .into_iter()
    .map(|id| {
        (
            id.to_owned(),
            GateResult::QualifiedPass {
                evidence_ref: format!("synthetic-evidence:{id}"),
            },
        )
    })
    .collect()
}

fn make_option(index: u64, count: u64, tradeoff: bool) -> DeliveryOption {
    let position = index + 1;
    let reverse = count.saturating_sub(index);
    let (cost, carbon, crew, duration, disruption, repairability) = if tradeoff {
        (
            reverse,
            position,
            (index * 37 + 11) % count + 1,
            (index * 53 + 7) % count + 1,
            (index * 71 + 19) % count + 1,
            position * 10,
        )
    } else {
        (
            reverse,
            reverse * 2,
            reverse * 3,
            reverse * 4,
            reverse * 5,
            position * 10,
        )
    };

    let values_for = |scale: u64| {
        BTreeMap::from([
            (MetricKey::WholeLifeCostMinorUnits, cost * scale),
            (MetricKey::EmbodiedCarbonGramsCo2e, carbon * scale),
            (MetricKey::CrewMinutes, crew * scale),
            (MetricKey::DeliveryDurationMinutes, duration * scale),
            (MetricKey::ServiceDisruptionMinutes, disruption * scale),
            (MetricKey::RepairabilityBasisPoints, repairability),
        ])
    };

    let scenario_metrics = BTreeMap::from([
        (
            "normal".to_owned(),
            ScenarioMetrics {
                scenario_id: "normal".to_owned(),
                values: values_for(1),
            },
        ),
        (
            "supply-delay".to_owned(),
            ScenarioMetrics {
                scenario_id: "supply-delay".to_owned(),
                values: values_for(2),
            },
        ),
        (
            "maintenance-stress".to_owned(),
            ScenarioMetrics {
                scenario_id: "maintenance-stress".to_owned(),
                values: values_for(3),
            },
        ),
    ]);

    DeliveryOption {
        option_id: format!("option-{index:05}"),
        functional_brief_id: "scale-benchmark-brief".to_owned(),
        brief_revision_digest: "brief-revision-scale".to_owned(),
        tier: match index % 3 {
            0 => OptionTier::Avoid,
            1 => OptionTier::Switch,
            _ => OptionTier::Improve,
        },
        delivery_method: format!("synthetic-method-{index}"),
        design_revision_digest: format!("synthetic-design-{index}"),
        gate_results: gate_results(),
        scenario_metrics,
        assumptions_ref: format!("synthetic-assumptions-{index}"),
    }
}

fn percentile(sorted: &[Duration], percentile: usize) -> Duration {
    let index = ((sorted.len() * percentile).div_ceil(100))
        .saturating_sub(1)
        .min(sorted.len() - 1);
    sorted[index]
}

fn run_distribution(count: u64, tradeoff: bool) {
    let brief = brief();
    let options: Vec<DeliveryOption> = (0..count)
        .map(|index| make_option(index, count, tradeoff))
        .collect();

    let mut samples = Vec::new();
    let mut pareto_count = 0_usize;
    for _ in 0..7 {
        let start = Instant::now();
        let report = evaluate_options(black_box(&brief), black_box(&options))
            .expect("generated option catalogue");
        samples.push(start.elapsed());
        pareto_count = report.pareto_option_ids.len();
        black_box(report);
    }
    samples.sort();

    println!(
        "distribution={} options={} repetitions={} pareto={} p50_ms={:.3} p95_ms={:.3} p99_ms={:.3}",
        if tradeoff { "tradeoff-heavy" } else { "dominated-heavy" },
        count,
        samples.len(),
        pareto_count,
        percentile(&samples, 50).as_secs_f64() * 1_000.0,
        percentile(&samples, 95).as_secs_f64() * 1_000.0,
        percentile(&samples, 99).as_secs_f64() * 1_000.0
    );
}

fn main() {
    for count in [10_u64, 100, 1_000, 3_000] {
        run_distribution(count, false);
        run_distribution(count, true);
    }
}
