use std::collections::BTreeMap;

use adaptive_evidence_acquisition::{
    AcquisitionBudget, AcquisitionCandidate, CandidateQuality, EvidenceTarget,
    QualifiedExistingPass, SelectionPolicy, plan_acquisition_with_policy,
};

const TARGET_COUNT: usize = 12;
const CANDIDATE_COUNT: usize = 72;
const DEVELOPMENT_EPISODES: usize = 48;
const HOLDOUT_EPISODES: usize = 16;

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
                    .min(progress.required_independent_passes)
                    as f64
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
    layout_family: usize,
) -> (
    Vec<EvidenceTarget>,
    Vec<QualifiedExistingPass>,
    Vec<AcquisitionCandidate>,
    AcquisitionBudget,
) {
    let mut rng = DeterministicRng(seed);
    let layout_scale = match layout_family {
        0 => 1.0,  // regular rectangular array
        1 => 1.15, // wider array
        2 => 0.92, // compact array
        _ => 1.08, // elongated, staggered rows
    };
    let target_columns = 4;
    let row_spacing = if layout_family == 3 { 5.0 } else { 6.0 };
    let column_spacing = if layout_family == 3 { 6.5 } else { 6.0 };

    let target_geometry: Vec<(String, f64, f64)> = (0..TARGET_COUNT)
        .map(|index| {
            let row = index / target_columns;
            let column = index % target_columns;
            let stagger = if layout_family == 3 && row % 2 == 1 {
                column_spacing * 0.35
            } else {
                0.0
            };
            let u = 1.0
                + column as f64 * column_spacing * layout_scale
                + stagger * layout_scale
                + rng.range(-0.35, 0.35) * layout_scale;
            let v = 1.0
                + row as f64 * row_spacing * layout_scale
                + rng.range(-0.30, 0.30) * layout_scale;
            (format!("target-{index:02}"), u, v)
        })
        .collect();

    let targets: Vec<EvidenceTarget> = target_geometry
        .iter()
        .map(|(target_id, _, _)| EvidenceTarget {
            target_id: target_id.clone(),
            required_independent_passes: 2,
            minimum_view_angle_separation_rad: 0.45,
            weight: rng.range(0.5, 2.5),
        })
        .collect();

    // Six spatial acquisition passes cross the work area. Candidate order is
    // serpentine so InputOrder reproduces a fixed-raster comparator. All
    // candidates on one lane share a pass identity; a target cannot accrue
    // repeated independent observations from consecutive frames on that lane.
    let pass_angles: [f64; 6] = [-1.40, -0.85, -0.30, 0.30, 0.85, 1.40];
    let footprint_half_width_m = 4.7 * layout_scale;
    let footprint_half_height_m = 3.8 * layout_scale;
    let mut candidates = Vec::with_capacity(CANDIDATE_COUNT);

    for index in 0..CANDIDATE_COUNT {
        let lane = index / 12;
        let lane_column = index % 12;
        let column = if lane % 2 == 0 {
            lane_column
        } else {
            11 - lane_column
        };
        let u_m = column as f64 * 2.0 * layout_scale + rng.range(-0.12, 0.12);
        let v_m = lane as f64 * 2.0 * layout_scale + rng.range(-0.10, 0.10);
        let view_angle_rad = pass_angles[lane];

        // Compute candidate coverage from an oriented rectangular footprint.
        // This is an analytic synthetic geometry oracle, not a sensor/vision
        // pipeline. Each target is credited only when fully inside the modelled
        // footprint in the candidate's local frame.
        let cosine = view_angle_rad.cos();
        let sine = view_angle_rad.sin();
        let mut covered_target_ids = Vec::new();
        for (target_id, target_u_m, target_v_m) in &target_geometry {
            let delta_u = target_u_m - u_m;
            let delta_v = target_v_m - v_m;
            let local_u = delta_u * cosine + delta_v * sine;
            let local_v = -delta_u * sine + delta_v * cosine;
            if local_u.abs() <= footprint_half_width_m && local_v.abs() <= footprint_half_height_m {
                covered_target_ids.push(target_id.clone());
            }
        }

        // Upstream sensor/evidence preflight is represented by a deterministic
        // fail-closed disposition distribution for benchmark stress, not by a
        // simulated controller or a learned detector.
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
            pass_id: format!("pass-lane-{lane:02}"),
            u_m,
            v_m,
            view_angle_rad,
            covered_target_ids,
            quality,
            acquisition_time_s: rng.range(1.5, 4.5),
            energy_wh: rng.range(0.35, 1.4),
            risk_penalty_s: rng.range(0.05, 1.5),
        });
    }

    let maximum_extent_u = target_geometry
        .iter()
        .map(|(_, u, _)| *u)
        .fold(0.0, f64::max);
    let maximum_extent_v = target_geometry
        .iter()
        .map(|(_, _, v)| *v)
        .fold(0.0, f64::max);
    let budget = AcquisitionBudget {
        start_u_m: 0.0,
        start_v_m: 0.0,
        travel_speed_m_per_s: 1.8,
        maximum_elapsed_time_s: 180.0 + maximum_extent_u * 1.5 + maximum_extent_v,
        maximum_energy_wh: 28.0,
        energy_penalty_s_per_wh: 2.0,
    };

    (targets, Vec::new(), candidates, budget)
}

fn independent_gate_violations(
    plan: &adaptive_evidence_acquisition::AcquisitionPlan,
    targets: &[EvidenceTarget],
    candidates: &[AcquisitionCandidate],
    budget: &AcquisitionBudget,
) -> usize {
    let mut violations = usize::from(
        plan.cumulative_elapsed_time_s > budget.maximum_elapsed_time_s + f64::EPSILON
            || plan.cumulative_energy_wh > budget.maximum_energy_wh + f64::EPSILON,
    );
    let mut credited: BTreeMap<&str, Vec<(&str, f64)>> = BTreeMap::new();
    let target_by_id: BTreeMap<&str, &EvidenceTarget> = targets
        .iter()
        .map(|target| (target.target_id.as_str(), target))
        .collect();

    for (ordinal, step) in plan.steps.iter().enumerate() {
        if step.ordinal != ordinal {
            violations += 1;
        }
        let Some(candidate) = candidates
            .iter()
            .find(|candidate| candidate.candidate_id == step.candidate_id)
        else {
            violations += 1;
            continue;
        };
        if candidate.quality != CandidateQuality::Qualified || candidate.pass_id != step.pass_id {
            violations += 1;
            continue;
        }
        for target_id in &step.marginal_target_ids {
            let Some(target) = target_by_id.get(target_id.as_str()) else {
                violations += 1;
                continue;
            };
            if !candidate
                .covered_target_ids
                .iter()
                .any(|covered| covered == target_id)
            {
                violations += 1;
                continue;
            }
            let previous = credited.entry(target_id.as_str()).or_default();
            let duplicate_pass = previous
                .iter()
                .any(|(pass_id, _)| *pass_id == candidate.pass_id.as_str());
            let insufficient_separation = previous.iter().any(|(_, angle)| {
                let difference = (angle - candidate.view_angle_rad).abs();
                difference.min(std::f64::consts::TAU - difference)
                    < target.minimum_view_angle_separation_rad
            });
            if duplicate_pass || insufficient_separation {
                violations += 1;
            } else {
                previous.push((candidate.pass_id.as_str(), candidate.view_angle_rad));
            }
        }
    }

    let expected_energy: f64 = plan.steps.iter().map(|step| step.energy_wh).sum();
    if (expected_energy - plan.cumulative_energy_wh).abs() > 1.0e-9 {
        violations += 1;
    }
    violations
}

fn run_policy_split(
    split_name: &str,
    episodes: usize,
    seed_start: u64,
    layout_families: &[usize],
    summaries: &mut BTreeMap<&'static str, PolicySummary>,
) {
    for episode in 0..episodes {
        let seed = seed_start + episode as u64;
        let family = layout_families[episode % layout_families.len()];
        let (targets, existing, candidates, budget) = generate_episode(seed, family);

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
                    eprintln!(
                        "benchmark_split={split_name} episode={episode} policy={name} error={error}"
                    );
                    std::process::exit(2);
                }
            };
            summaries
                .entry(name)
                .or_default()
                .record(&plan, &targets, &candidates, &budget);
            let violations = independent_gate_violations(&plan, &targets, &candidates, &budget);
            if violations > 0 {
                summaries
                    .get_mut(name)
                    .expect("summary was recorded")
                    .gate_violations += violations;
            }
        }
    }
}

fn run_matched_seeded_benchmark() {
    let mut development_summaries: BTreeMap<&'static str, PolicySummary> = BTreeMap::new();
    let mut holdout_summaries: BTreeMap<&'static str, PolicySummary> = BTreeMap::new();

    // The first three layout families are development distributions. The
    // staggered-row family is withheld from policy development and replayed
    // only as a synthetic geometry-family holdout.
    run_policy_split(
        "development",
        DEVELOPMENT_EPISODES,
        20_261_117,
        &[0, 1, 2],
        &mut development_summaries,
    );
    run_policy_split(
        "holdout_staggered",
        HOLDOUT_EPISODES,
        20_270_101,
        &[3],
        &mut holdout_summaries,
    );

    println!(
        "suite=ADAPTIVE-EVIDENCE-ACQUISITION-BENCHMARK-002 development_episodes={DEVELOPMENT_EPISODES} holdout_episodes={HOLDOUT_EPISODES} targets_per_episode={TARGET_COUNT} candidates_per_episode={CANDIDATE_COUNT} development_seeds=20261117+ holdout_seeds=20270101+"
    );
    println!("split=development layout_families=regular,wide,compact");
    for (name, summary) in &development_summaries {
        summary.print(name);
    }
    println!("split=holdout layout_family=staggered_rows");
    for (name, summary) in &holdout_summaries {
        summary.print(name);
    }
    println!(
        "note=synthetic matched candidate/budget benchmark using oriented analytic footprints; sensor quality is a deterministic disposition fixture; no image formation, occlusion physics, obstacles, flight dynamics or physical trial; all policies share fail-closed qualification and independence gates"
    );
}

fn main() {
    let (targets, existing, candidates, budget) = generate_episode(7, 0);
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
