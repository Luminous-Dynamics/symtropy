// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Reproducible, dependency-light microbenchmark for the exact Pareto comparer.
//!
//! Run with:
//! cargo bench --manifest-path crates/domains/symtropy-garden-sim/Cargo.toml --bench portfolio_frontier
//!
//! Synthetic data only. The printed fingerprint is an FNV-1a repeatability aid,
//! not a cryptographic digest or evidence of source authenticity.

use std::hint::black_box;
use std::time::Instant;
use symtropy_garden_sim::portfolio::{
    compare_portfolios_v1, PortfolioCandidateFeasibilityV1, PortfolioCandidateV1,
    PortfolioComparisonRequestV1, PortfolioComparisonStatusV1,
    PortfolioObjectiveDefinitionV1, PortfolioObjectiveDirectionV1,
    PortfolioObjectiveValueV1, GARDEN_PORTFOLIO_SCHEMA_VERSION,
};

fn next_random(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *state
}

fn make_request(
    candidate_count: usize,
    objective_count: usize,
    seed: u64,
    comparison_id: String,
) -> PortfolioComparisonRequestV1 {
    let objectives: Vec<_> = (0..objective_count)
        .map(|index| PortfolioObjectiveDefinitionV1 {
            objective_id: format!("objective-{index:02}"),
            direction: if index.is_multiple_of(2) {
                PortfolioObjectiveDirectionV1::Minimize
            } else {
                PortfolioObjectiveDirectionV1::Maximize
            },
            unit: format!("unit-{index:02}"),
            value_scale: 1,
        })
        .collect();

    let mut state = seed;
    let candidates: Vec<_> = (0..candidate_count)
        .map(|index| {
            let candidate_id = format!("CANDIDATE-{index:05}");
            let feasibility = if index.is_multiple_of(17) {
                PortfolioCandidateFeasibilityV1::Unassessed
            } else if index.is_multiple_of(11) {
                PortfolioCandidateFeasibilityV1::Infeasible
            } else {
                PortfolioCandidateFeasibilityV1::Feasible
            };
            let blocker_refs = match feasibility {
                PortfolioCandidateFeasibilityV1::Feasible => Vec::new(),
                PortfolioCandidateFeasibilityV1::Infeasible => {
                    vec![format!("synthetic:blocker:{candidate_id}")]
                }
                PortfolioCandidateFeasibilityV1::Unassessed => {
                    vec![format!("synthetic:missing-evidence:{candidate_id}")]
                }
            };
            let evidence_refs = if feasibility == PortfolioCandidateFeasibilityV1::Feasible {
                vec![format!("synthetic:evidence:{candidate_id}")]
            } else {
                Vec::new()
            };
            let objective_values = if feasibility == PortfolioCandidateFeasibilityV1::Feasible {
                objectives
                    .iter()
                    .map(|objective| PortfolioObjectiveValueV1 {
                        objective_id: objective.objective_id.clone(),
                        unit: objective.unit.clone(),
                        value_scale: objective.value_scale,
                        value_scaled: i128::from(next_random(&mut state) % 1_000_001),
                    })
                    .collect()
            } else {
                Vec::new()
            };
            PortfolioCandidateV1 {
                candidate_id,
                scope_ref: "scope:synthetic-community".to_owned(),
                period_ref: "period:synthetic-001".to_owned(),
                feasibility,
                blocker_refs,
                evidence_refs,
                objective_values,
            }
        })
        .collect();

    PortfolioComparisonRequestV1 {
        schema_version: GARDEN_PORTFOLIO_SCHEMA_VERSION,
        comparison_id,
        scope_ref: "scope:synthetic-community".to_owned(),
        period_ref: "period:synthetic-001".to_owned(),
        objectives,
        candidates,
    }
}

fn report_fingerprint(request: &PortfolioComparisonRequestV1) -> u64 {
    let report = compare_portfolios_v1(black_box(request));
    assert_eq!(report.status, PortfolioComparisonStatusV1::Compared);
    let mut bytes = Vec::new();
    for id in &report.frontier_candidate_ids {
        bytes.extend_from_slice(id.as_bytes());
        bytes.push(0xff);
    }
    bytes.push(0xfe);
    for witness in &report.dominance_witnesses {
        bytes.extend_from_slice(witness.candidate_id.as_bytes());
        bytes.push(0xfd);
        bytes.extend_from_slice(witness.dominated_by_candidate_id.as_bytes());
        bytes.push(0xfc);
    }
    for excluded in &report.excluded_candidates {
        bytes.extend_from_slice(excluded.candidate_id.as_bytes());
        bytes.push(0xfb);
        bytes.extend_from_slice(format!("{:?}", excluded.feasibility).as_bytes());
        for blocker_ref in &excluded.blocker_refs {
            bytes.extend_from_slice(blocker_ref.as_bytes());
            bytes.push(0xfa);
        }
    }
    // FNV-1a 64-bit is used only to make output changes visible between runs.
    bytes.iter().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

fn run_comparison(request: &PortfolioComparisonRequestV1) {
    let report = compare_portfolios_v1(black_box(request));
    assert_eq!(report.status, PortfolioComparisonStatusV1::Compared);
    black_box((
        report.frontier_candidate_ids.len(),
        report.dominated_candidate_ids.len(),
        report.dominance_witnesses.len(),
        report.excluded_candidates.len(),
    ));
}

fn measure_single(label: &str, request: &PortfolioComparisonRequestV1) {
    let output_fingerprint = report_fingerprint(request);
    for _ in 0..5 {
        run_comparison(request);
    }
    let mut samples = Vec::with_capacity(30);
    for _ in 0..30 {
        let start = Instant::now();
        run_comparison(request);
        samples.push(start.elapsed().as_nanos());
    }
    samples.sort_unstable();
    let p50 = samples[14];
    let p95 = samples[28];
    println!(
        "workload={label} candidates={} objectives={} repetitions=30 p50_us={} p95_us={} output_fingerprint_fnv1a64={output_fingerprint:016x}",
        request.candidates.len(),
        request.objectives.len(),
        p50 / 1_000,
        p95 / 1_000,
    );
    println!("workload={label} sorted_samples_ns={samples:?}");
}

fn measure_batch(label: &str, requests: &[PortfolioComparisonRequestV1]) {
    let output_fingerprint_sum = requests
        .iter()
        .map(report_fingerprint)
        .fold(0_u64, u64::wrapping_add);
    for _ in 0..5 {
        for request in requests {
            run_comparison(request);
        }
    }
    let mut samples = Vec::with_capacity(30);
    for _ in 0..30 {
        let start = Instant::now();
        for request in requests {
            run_comparison(request);
        }
        samples.push(start.elapsed().as_nanos());
    }
    samples.sort_unstable();
    let p50 = samples[14];
    let p95 = samples[28];
    println!(
        "workload={label} scenarios={} candidates_per_scenario={} objectives_per_scenario={} repetitions=30 p50_ms={} p95_ms={} output_fingerprint_sum_fnv1a64={output_fingerprint_sum:016x}",
        requests.len(),
        requests.first().map_or(0, |request| request.candidates.len()),
        requests.first().map_or(0, |request| request.objectives.len()),
        p50 / 1_000_000,
        p95 / 1_000_000,
    );
    println!("workload={label} sorted_samples_ns={samples:?}");
}

fn main() {
    println!(
        "benchmark=portfolio_frontier_v1 os={} arch={} debug_assertions={} seed=24301 note=synthetic_only",
        std::env::consts::OS,
        std::env::consts::ARCH,
        cfg!(debug_assertions),
    );

    let large = make_request(1_000, 12, 24_301, "benchmark-large-1000x12".to_owned());
    measure_single("frontier_1000x12", &large);

    let scenarios: Vec<_> = (0..100)
        .map(|index| {
            let batch_index = u64::try_from(index).expect("batch index fits u64");
            let seed = 24_301_u64.wrapping_add(batch_index * 7_919);
            make_request(100, 8, seed, format!("benchmark-batch-{index:03}"))
        })
        .collect();
    measure_batch("batch_100x100x8", &scenarios);
}
