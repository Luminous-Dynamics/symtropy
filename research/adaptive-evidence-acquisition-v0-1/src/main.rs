use std::collections::BTreeMap;

use adaptive_evidence_acquisition::{
    AcquisitionBudget, AcquisitionCandidate, CandidateQuality, EvidenceTarget,
    QualifiedExistingPass, SelectionPolicy, plan_acquisition_with_policy,
};

const TARGET_COUNT: usize = 12;
const CANDIDATE_COUNT: usize = 72;
const EPISODE_COUNT: usize = 64;

#[derive(Clone, Copy)]
struct DeterministicRng(u64);

impl DeterministicRng {
    fn next_unit(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 11) as f64 / ((1_u64 << 53) as f64)
    }

    fn range(&mut self, minimum: f64, maximum: f64) -> f64 {
        minimum + (maximum - minimum) * self.next_unit()
    }
}

#[derive(Default)]
struct PolicySummary {
    weighted_completion: Vec<f64>,
    evidence_units_per_minute: Vec<f64>,
    evidence_units_per_wh: Vec<f64>,
    elapsed_seconds: Vec<f64>,
    energy_wh: Vec<f64>,
    selected_steps: Vec<f64>,
    gate_violations: usize,
}

impl PolicySummary {
    fn record(
        &mut self,
        plan: &adaptive_evidence_acquisition::AcquisitionPlan,
        targets: &[EvidenceTarget],
        candidates: &[AcquisitionCandidate],
        budget: &AcquisitionBudget,
    ) {
        let weights: BTreeMap<&str, f64> = targets
            .iter()
            .map(|target| (target.target_id.as_str(), target.weight))
            .collect();
        let total_weight: f64 = targets.iter().map(|target| target.weight).sum();
        let completed_weight: f64 = plan
            .targets
            .iter()
            .map(|progress| {
                let weight = weights[progress.target_id.as_str()];
                let fraction = progress
                    .qualifying_passes
                    .min(progress.required_independent_passes) as f64
                    / progress.required_independent_passes as f64;
                weight * fraction
            })
            .sum();
        let elapsed = plan.cumulative_elapsed_time_s;
        let energy = plan.cumulative_energy_wh;
        let units_per_minute = if elapsed > 0.0 {
            completed_weight / (elapsed / 60.0)
        } else {
            0.0
        };
        let units_per_wh = if energy > 0.0 {
            completed_weight / energy
        } else {
            0.0
        };

        self.weighted_completion
            .push(completed_weight / total_weight);
        self.evidence_units_per_minute.push(units_per_minute);
        self.evidence_units_per_wh.push(units_per_wh);
        self.elapsed_seconds.push(elapsed);
        self.energy_wh.push(energy);
        self.selected_steps.push(plan.steps.len() as f64);

        if elapsed > budget.maximum_elapsed_time_s + f64::EPSILON
            || energy > budget.maximum_energy_wh + f64::EPSILON
            || plan.steps.iter().any(|step| {
                candidates
                    .iter()
                    .find(|candidate| candidate.candidate_id == step.candidate_id)
                    .is_none_or(|candidate| candidate.quality != CandidateQuality::Qualified)
            })
        {
            self.gate_violations += 1;
        }
    }

    fn mean(values: &[f64]) -> f64 {
        if values.is_empty() {
            0.0
        } else {
            values.iter().sum::<f64>() / values.len() as f64
        }
    }

    fn p10(values: &[f64]) -> f64 {
        if values.is_empty() {
            return 0.0;
        }
        let mut sorted = values.to_vec();
        sorted.sort_by(f64::total_cmp);
        sorted[((sorted.len() - 1) as f64 * 0.10).floor() as usize]
    }

    fn minimum(values: &[f64]) -> f64 {
        values.iter().copied().reduce(f64::min).unwrap_or(0.0)
    }

    fn print(&self, name: &str) {
        println!(
            "benchmark_policy={} episodes={} mean_completion={:.4} p10_completion={:.4} worst_completion={:.4} mean_weighted_units_per_min={:.4} mean_weighted_units_per_wh={:.4} mean_elapsed_s={:.3} mean_energy_wh={:.3} mean_selected_steps={:.2} gate_violations={}",
            name,
            self.weighted_completion.len(),
            Self::mean(&self.weighted_completion),
            Self::p10(&self.weighted_completion),
            Self::minimum(&self.weighted_completion),
            Self::mean(&self.evidence_units_per_minute),
            Self::mean(&self.evidence_units_per_wh),
            Self::mean(&self.elapsed_seconds),
            Self::mean(&self.energy_wh),
            Self::mean(&self.selected_steps),
            self.gate_violations,
        );
    }
}

fn policies(seed: u64) -> [(&'static str, SelectionPolicy); 5] {
    [
        (
            "evidence_gain_per_cost",
            SelectionPolicy::EvidenceGainPerCost,
        ),
        ("nearest_feasible", SelectionPolicy::NearestFeasible),
        (
            "maximum_marginal_evidence",
            SelectionPolicy::MaximumMarginalEvidence,
        ),
        ("input_order_fixed_route", SelectionPolicy::InputOrder),
        (
            "seeded_hash_order",
            SelectionPolicy::SeededHashOrder { seed },
        ),
    ]
}

fn generate_episode(
    seed: u64,
) -> (
    Vec<EvidenceTarget>,
    Vec<QualifiedExistingPass>,
    Vec<AcquisitionCandidate>,
    AcquisitionBudget,
) {
    let mut rng = DeterministicRng(seed);
    let targets: Vec<EvidenceTarget> = (0..TARGET_COUNT)
        .map(|index| EvidenceTarget {
            target_id: format!("target-{index:02}"),
            required_independent_passes: 2,
            minimum_view_angle_separation_rad: 0.45,
            weight: rng.range(0.5, 2.5),
        })
        .collect();

    let mut candidates = Vec::with_capacity(CANDIDATE_COUNT);
    for index in 0..CANDIDATE_COUNT {
        // The supplied order is a serpentine/raster-style baseline route.
        let lane = index / 12;
        let lane_column = index % 12;
        let column = if lane % 2 == 0 {
            lane_column
        } else {
            11 - lane_column
        };
        let primary_target = (index * 7 + seed as usize) % TARGET_COUNT;
        let mut covered_target_ids = vec![format!("target-{primary_target:02}")];
        if index % 4 == 0 {
            covered_target_ids.push(format!("target-{:02}", (primary_target + 3) % TARGET_COUNT));
        }

        let quality_roll = rng.next_unit();
        let quality = if quality_roll < 0.82 {
            CandidateQuality::Qualified
        } else if quality_roll < 0.90 {
            CandidateQuality::NotQualified
        } else if quality_roll < 0.95 {
            CandidateQuality::Unknown
        } else {
            CandidateQuality::InvalidEvidence
        };

        candidates.push(AcquisitionCandidate {
            candidate_id: format!("candidate-{index:03}"),
            pass_id: format!("pass-{index:03}"),
            u_m: column as f64 * 1.8 + rng.range(0.0, 0.4),
            v_m: lane as f64 * 2.2 + rng.range(0.0, 0.4),
            view_angle_rad: rng.range(-std::f64::consts::PI, std::f64::consts::PI),
            covered_target_ids,
            quality,
            acquisition_time_s: rng.range(1.5, 4.5),
            energy_wh: rng.range(0.35, 1.4),
            risk_penalty_s: rng.range(0.05, 1.5),
        });
    }

    let budget = AcquisitionBudget {
        start_u_m: 0.0,
        start_v_m: 0.0,
        travel_speed_m_per_s: 1.8,
        maximum_elapsed_time_s: 160.0,
        maximum_energy_wh: 28.0,
        energy_penalty_s_per_wh: 2.0,
    };

    (targets, Vec::new(), candidates, budget)
}

fn run_matched_seeded_benchmark() {
    let mut summaries: BTreeMap<&'static str, PolicySummary> = BTreeMap::new();

    for episode in 0..EPISODE_COUNT {
        let seed = 20_261_117_u64 + episode as u64;
        let (targets, existing, candidates, budget) = generate_episode(seed);

        for (name, policy) in policies(seed ^ 0x5eed) {
            let plan = match plan_acquisition_with_policy(
                &targets,
                &existing,
                &candidates,
                &budget,
                policy,
            ) {
                Ok(plan) => plan,
                Err(error) => {
                    eprintln!("benchmark_episode={episode} policy={name} error={error}");
                    std::process::exit(2);
                }
            };
            summaries
                .entry(name)
                .or_default()
                .record(&plan, &targets, &candidates, &budget);
        }
    }

    println!(
        "suite=ADAPTIVE-EVIDENCE-ACQUISITION-BENCHMARK-001 episodes={EPISODE_COUNT} targets_per_episode={TARGET_COUNT} candidates_per_episode={CANDIDATE_COUNT} seed_start=20261117"
    );
    for (name, summary) in &summaries {
        summary.print(name);
    }
    println!(
        "note=synthetic matched candidate/budget benchmark; all policies share identical fail-closed qualification and independence gates; not a physical performance claim"
    );
}

fn main() {
    let (targets, existing, candidates, budget) = generate_episode(7);
    println!("suite=ADAPTIVE-EVIDENCE-ACQUISITION-EXAMPLE-001");
    println!("candidate_pool_size={}", candidates.len());
    for (name, policy) in policies(7) {
        match plan_acquisition_with_policy(&targets, &existing, &candidates, &budget, policy) {
            Ok(plan) => {
                let selected = plan
                    .steps
                    .iter()
                    .map(|step| step.candidate_id.as_str())
                    .collect::<Vec<_>>()
                    .join(",");
                println!(
                    "policy={} disposition={:?} selected={} completed_targets={}/{} elapsed_s={:.3} energy_wh={:.3} unknown_candidates={} invalid_candidates={}",
                    name,
                    plan.disposition,
                    selected,
                    plan.completed_targets,
                    plan.total_targets,
                    plan.cumulative_elapsed_time_s,
                    plan.cumulative_energy_wh,
                    plan.blockers.unknown,
                    plan.blockers.invalid_evidence,
                );
            }
            Err(error) => {
                eprintln!("example_policy={} planning_error={error}", name);
                std::process::exit(2);
            }
        }
    }
    run_matched_seeded_benchmark();
}
