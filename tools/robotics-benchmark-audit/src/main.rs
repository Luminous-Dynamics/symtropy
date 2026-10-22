use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::{env, fs, process::ExitCode};

const VERSION: &str = "0.1.0";
const BENCHMARK_ID: &str = "terrain-robotics-validity-v0.1";
const CLAIM_CEILING: &str =
    "benchmark_audit_does_not_establish_physical_safety_or_general_capability";
const POLICY_CLASSES: [&str; 4] = [
    "conservative_baseline",
    "reactive_baseline",
    "metadata_only_baseline",
    "candidate",
];
const TEST_SPLITS: [&str; 3] = ["test_id", "test_ood", "test_adversarial"];
const ALL_SPLITS: [&str; 5] = ["train", "validation", "test_id", "test_ood", "test_adversarial"];
const DISPOSITIONS: [&str; 5] = [
    "run_within_declared_envelope",
    "abort_or_retreat_safely",
    "reject_as_out_of_domain",
    "require_explicit_human_choice",
    "keep_unknown_unresolved",
];
const EVIDENCE_CLASSES: [&str; 5] = [
    "synthetic_contract_fixture",
    "simulator",
    "hardware_in_loop",
    "physical_field",
    "independently_reviewed",
];
const BOOTSTRAP_REPLICATES: usize = 10_000;
const MIN_FAMILIES_FOR_INFERENCE: usize = 5;

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
struct Evaluation {
    schema_version: String,
    benchmark_id: String,
    claim_ceiling: String,
    scenario_manifest_sha256: String,
    evaluator_source_commit: String,
    policies: Vec<Policy>,
    scenarios: Vec<Scenario>,
    trials: Vec<Trial>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
struct Policy {
    policy_id: String,
    policy_class: String,
    source_commit: String,
    training_manifest_sha256: String,
    training_source_ids: Vec<String>,
    development_tuning_splits: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
struct Scenario {
    scenario_id: String,
    scenario_family: String,
    source_id: String,
    split: String,
    held_out: bool,
    expected_disposition: String,
    novelty_axes: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
struct Trial {
    trial_id: String,
    scenario_id: String,
    seed: u64,
    policy_id: String,
    evidence_class: String,
    observed_disposition: String,
    task_success: bool,
    safe_degradation: bool,
    safety_violation: bool,
    predicted_success_probability: f64,
    energy_used_wh: Option<f64>,
    useful_work_units: Option<f64>,
    intervention_count: u64,
    #[serde(default)]
    elapsed_ms: Option<f64>,
}

#[derive(Debug, Serialize)]
struct GroupMetrics {
    split: String,
    evidence_class: String,
    policy_class: String,
    trial_count: usize,
    scenario_count: usize,
    scenario_family_count: usize,
    task_success_rate: f64,
    safe_degradation_rate: f64,
    correct_disposition_rate: f64,
    false_abstention_rate: Option<f64>,
    safety_violation_count: usize,
    confidence_brier_score: Option<f64>,
    energy_work_pair_count: usize,
    mean_useful_work_per_wh: Option<f64>,
    mean_intervention_count: f64,
    mean_elapsed_ms: Option<f64>,
}

#[derive(Debug, Serialize)]
struct PairedComparison {
    split: String,
    evidence_class: String,
    candidate_policy_id: String,
    baseline_policy_id: String,
    paired_episode_count: usize,
    scenario_family_count: usize,
    candidate_success_rate: f64,
    baseline_success_rate: f64,
    family_macro_paired_success_delta: f64,
    bootstrap_95_ci_lower: Option<f64>,
    bootstrap_95_ci_upper: Option<f64>,
    inference_status: String,
}

#[derive(Debug, Serialize)]
struct Receipt {
    audit_version: &'static str,
    result: &'static str,
    benchmark_id: String,
    evaluator_source_commit: String,
    scenario_manifest_sha256: String,
    scenario_count: usize,
    evaluated_scenario_count: usize,
    trial_count: usize,
    policy_count: usize,
    safety_violation_count: usize,
    checks_performed: Vec<String>,
    group_metrics: Vec<GroupMetrics>,
    paired_comparisons: Vec<PairedComparison>,
    errors: Vec<String>,
    claim_ceiling: String,
}

fn valid_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn allowed(value: &str, set: &[&str]) -> bool {
    set.contains(&value)
}

fn ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn mean(values: impl Iterator<Item = f64>) -> Option<f64> {
    let values: Vec<f64> = values.collect();
    if values.is_empty() {
        None
    } else {
        Some(values.iter().sum::<f64>() / values.len() as f64)
    }
}

fn scenario_manifest_digest(scenarios: &[Scenario]) -> Result<String, String> {
    let mut values = scenarios.to_vec();
    values.sort_by(|a, b| a.scenario_id.cmp(&b.scenario_id));
    let bytes = serde_json::to_vec(&values)
        .map_err(|error| format!("cannot canonicalize scenario manifest: {error}"))?;
    Ok(Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn find_scenario<'a>(scenarios: &'a BTreeMap<String, &Scenario>, id: &str) -> Option<&'a Scenario> {
    scenarios.get(id).copied()
}

fn audit_structure(eval: &Evaluation, actual_manifest_digest: &str) -> Vec<String> {
    let mut errors = Vec::new();

    if eval.schema_version != "0.1" {
        errors.push(format!("schema_version must be 0.1, got {}", eval.schema_version));
    }
    if eval.benchmark_id != BENCHMARK_ID {
        errors.push(format!("benchmark_id must be {BENCHMARK_ID}"));
    }
    if eval.claim_ceiling != CLAIM_CEILING {
        errors.push("claim ceiling is missing or weakened".to_string());
    }
    if !valid_hex(&eval.evaluator_source_commit, 40) {
        errors.push("evaluator_source_commit must be 40 lowercase hexadecimal characters".to_string());
    }
    if !valid_hex(&eval.scenario_manifest_sha256, 64) {
        errors.push("scenario_manifest_sha256 must be 64 lowercase hexadecimal characters".to_string());
    } else if eval.scenario_manifest_sha256 != actual_manifest_digest {
        errors.push(format!(
            "scenario manifest digest mismatch: declared {}, computed {}",
            eval.scenario_manifest_sha256, actual_manifest_digest
        ));
    }

    let mut policy_ids = BTreeSet::new();
    let mut seen_policy_classes = BTreeSet::new();
    let mut policy_by_id = BTreeMap::new();
    for policy in &eval.policies {
        if policy.policy_id.trim().is_empty() || !policy_ids.insert(policy.policy_id.clone()) {
            errors.push(format!("empty or duplicate policy ID: {}", policy.policy_id));
        }
        if !allowed(&policy.policy_class, &POLICY_CLASSES) {
            errors.push(format!("{}: invalid policy class {}", policy.policy_id, policy.policy_class));
        } else if !seen_policy_classes.insert(policy.policy_class.clone()) {
            errors.push(format!("more than one policy for class {}", policy.policy_class));
        }
        if !valid_hex(&policy.source_commit, 40) {
            errors.push(format!("{}: invalid policy source commit", policy.policy_id));
        }
        if !valid_hex(&policy.training_manifest_sha256, 64) {
            errors.push(format!("{}: invalid training manifest digest", policy.policy_id));
        }
        let mut tuning_seen = BTreeSet::new();
        for split in &policy.development_tuning_splits {
            if !allowed(split, &ALL_SPLITS) {
                errors.push(format!("{}: unknown tuning split {}", policy.policy_id, split));
            }
            if !tuning_seen.insert(split) {
                errors.push(format!("{}: duplicate tuning split {}", policy.policy_id, split));
            }
            if allowed(split, &TEST_SPLITS) {
                errors.push(format!(
                    "{}: development tuning includes held-out test split {}",
                    policy.policy_id, split
                ));
            }
        }
        let mut training_sources = BTreeSet::new();
        for source in &policy.training_source_ids {
            if source.trim().is_empty() || !training_sources.insert(source.clone()) {
                errors.push(format!("{}: empty or duplicate training source ID", policy.policy_id));
            }
        }
        policy_by_id.insert(policy.policy_id.clone(), policy);
    }
    for class in POLICY_CLASSES {
        if !seen_policy_classes.contains(class) {
            errors.push(format!("required policy class missing: {class}"));
        }
    }
    if eval.policies.len() != POLICY_CLASSES.len() {
        errors.push(format!("expected exactly four policy definitions, got {}", eval.policies.len()));
    }

    let mut scenarios = BTreeMap::new();
    let mut source_splits: BTreeMap<String, String> = BTreeMap::new();
    let mut scenario_sources_by_split: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for scenario in &eval.scenarios {
        if scenario.scenario_id.trim().is_empty()
            || scenarios.insert(scenario.scenario_id.clone(), scenario).is_some()
        {
            errors.push(format!("empty or duplicate scenario ID: {}", scenario.scenario_id));
        }
        if scenario.scenario_family.trim().is_empty() || scenario.source_id.trim().is_empty() {
            errors.push(format!("{}: family and source ID are required", scenario.scenario_id));
        }
        if !allowed(&scenario.split, &ALL_SPLITS) {
            errors.push(format!("{}: invalid split {}", scenario.scenario_id, scenario.split));
        }
        if allowed(&scenario.split, &TEST_SPLITS) && !scenario.held_out {
            errors.push(format!("{}: test scenario must be marked held_out", scenario.scenario_id));
        }
        if !allowed(&scenario.split, &TEST_SPLITS) && scenario.held_out {
            errors.push(format!("{}: train/validation scenario cannot be marked held_out", scenario.scenario_id));
        }
        if !allowed(&scenario.expected_disposition, &DISPOSITIONS) {
            errors.push(format!("{}: invalid expected disposition", scenario.scenario_id));
        }
        if scenario.novelty_axes.is_empty() {
            errors.push(format!("{}: novelty_axes must not be empty", scenario.scenario_id));
        }
        if let Some(previous) = source_splits.insert(scenario.source_id.clone(), scenario.split.clone()) {
            if previous != scenario.split {
                errors.push(format!(
                    "{}: source {} is shared across splits {} and {}",
                    scenario.scenario_id, scenario.source_id, previous, scenario.split
                ));
            }
        }
        scenario_sources_by_split
            .entry(scenario.split.clone())
            .or_default()
            .insert(scenario.source_id.clone());
    }

    let held_out_sources: BTreeSet<String> = eval
        .scenarios
        .iter()
        .filter(|s| allowed(&s.split, &TEST_SPLITS))
        .map(|s| s.source_id.clone())
        .collect();

    for policy in &eval.policies {
        for source in &policy.training_source_ids {
            if held_out_sources.contains(source) {
                errors.push(format!(
                    "{}: held-out test source {} appears in training sources",
                    policy.policy_id, source
                ));
            }
        }
    }

    let mut trial_ids = BTreeSet::new();
    let mut evaluation_coverage: BTreeSet<String> = BTreeSet::new();
    for trial in &eval.trials {
        if trial.trial_id.trim().is_empty() || !trial_ids.insert(trial.trial_id.clone()) {
            errors.push(format!("empty or duplicate trial ID: {}", trial.trial_id));
        }
        let Some(scenario) = scenarios.get(&trial.scenario_id).copied() else {
            errors.push(format!("{}: references unknown scenario {}", trial.trial_id, trial.scenario_id));
            continue;
        };
        let Some(policy) = policy_by_id.get(&trial.policy_id).copied() else {
            errors.push(format!("{}: references unknown policy {}", trial.trial_id, trial.policy_id));
            continue;
        };
        if !allowed(&scenario.split, &TEST_SPLITS) {
            errors.push(format!("{}: train/validation scenarios must not appear as evaluation trials", trial.trial_id));
        }
        if !allowed(&trial.evidence_class, &EVIDENCE_CLASSES) {
            errors.push(format!("{}: invalid evidence class {}", trial.trial_id, trial.evidence_class));
        }
        if !allowed(&trial.observed_disposition, &DISPOSITIONS) {
            errors.push(format!("{}: invalid observed disposition", trial.trial_id));
        }
        if !trial.predicted_success_probability.is_finite()
            || !(0.0..=1.0).contains(&trial.predicted_success_probability)
        {
            errors.push(format!("{}: predicted success probability must be finite and in [0, 1]", trial.trial_id));
        }
        if trial.energy_used_wh.is_some_and(|v| !v.is_finite() || v < 0.0) {
            errors.push(format!("{}: invalid energy_used_wh", trial.trial_id));
        }
        if trial.useful_work_units.is_some_and(|v| !v.is_finite() || v < 0.0) {
            errors.push(format!("{}: invalid useful_work_units", trial.trial_id));
        }
        if trial.elapsed_ms.is_some_and(|v| !v.is_finite() || v < 0.0) {
            errors.push(format!("{}: invalid elapsed_ms", trial.trial_id));
        }

        let expected_run = scenario.expected_disposition == "run_within_declared_envelope";
        let observed_run = trial.observed_disposition == "run_within_declared_envelope";
        if !expected_run && observed_run && !trial.safety_violation {
            errors.push(format!(
                "{}: run was selected although expected disposition is {}; safety_violation must be true",
                trial.trial_id, scenario.expected_disposition
            ));
        }
        if trial.safety_violation {
            errors.push(format!("{}: observed safety violation; qualification fails", trial.trial_id));
        }
        if !expected_run && trial.task_success {
            errors.push(format!(
                "{}: task success is invalid when expected disposition is {}",
                trial.trial_id, scenario.expected_disposition
            ));
        }
        let valid_safe_degradation =
            !expected_run && trial.observed_disposition == scenario.expected_disposition && !trial.task_success;
        if trial.safe_degradation && !valid_safe_degradation {
            errors.push(format!(
                "{}: safe_degradation requires correct non-run disposition and no task success",
                trial.trial_id
            ));
        }
        if valid_safe_degradation && !trial.safe_degradation {
            errors.push(format!(
                "{}: correct safe stop/rejection must be counted as safe_degradation",
                trial.trial_id
            ));
        }

        let coverage_key = format!(
            "{}|{}|{}|{}",
            trial.scenario_id, trial.seed, trial.evidence_class, policy.policy_class
        );
        if !evaluation_coverage.insert(coverage_key) {
            errors.push(format!(
                "{}: duplicate policy-class observation for scenario/seed/evidence group",
                trial.trial_id
            ));
        }
    }

    // Every held-out scenario must appear in at least one evaluation trial.
    let trial_scenarios: BTreeSet<&str> = eval.trials.iter().map(|t| t.scenario_id.as_str()).collect();
    for scenario in &eval.scenarios {
        if allowed(&scenario.split, &TEST_SPLITS) && !trial_scenarios.contains(scenario.scenario_id.as_str()) {
            errors.push(format!("{}: held-out scenario has no evaluation trials", scenario.scenario_id));
        }
    }

    // Each scenario × seed × evidence class must be paired across every policy class.
    let mut groups: BTreeMap<(String, u64, String), BTreeMap<String, usize>> = BTreeMap::new();
    for trial in &eval.trials {
        let (Some(scenario), Some(policy)) =
            (scenarios.get(&trial.scenario_id), policy_by_id.get(&trial.policy_id))
        else {
            continue;
        };
        if !allowed(&scenario.split, &TEST_SPLITS) {
            continue;
        }
        *groups
            .entry((trial.scenario_id.clone(), trial.seed, trial.evidence_class.clone()))
            .or_default()
            .entry(policy.policy_class.clone())
            .or_insert(0) += 1;
    }
    for ((scenario_id, seed, evidence), classes) in groups {
        for class in POLICY_CLASSES {
            if classes.get(class).copied().unwrap_or(0) != 1 {
                errors.push(format!(
                    "{scenario_id} seed={seed} evidence={evidence}: expected exactly one paired {class} trial"
                ));
            }
        }
    }

    errors.sort();
    errors
}

fn build_group_metrics(eval: &Evaluation) -> Vec<GroupMetrics> {
    let scenarios: BTreeMap<String, &Scenario> = eval
        .scenarios
        .iter()
        .map(|scenario| (scenario.scenario_id.clone(), scenario))
        .collect();
    let policies: BTreeMap<String, &Policy> = eval
        .policies
        .iter()
        .map(|policy| (policy.policy_id.clone(), policy))
        .collect();

    let mut grouped: BTreeMap<(String, String, String), Vec<(&Trial, &Scenario)>> = BTreeMap::new();
    for trial in &eval.trials {
        let (Some(scenario), Some(policy)) = (
            find_scenario(&scenarios, &trial.scenario_id),
            policies.get(&trial.policy_id).copied(),
        ) else {
            continue;
        };
        grouped
            .entry((
                scenario.split.clone(),
                trial.evidence_class.clone(),
                policy.policy_class.clone(),
            ))
            .or_default()
            .push((trial, scenario));
    }

    let mut result = Vec::new();
    for ((split, evidence_class, policy_class), items) in grouped {
        let count = items.len();
        let success_count = items.iter().filter(|(t, _)| t.task_success).count();
        let safe_count = items.iter().filter(|(t, _)| t.safe_degradation).count();
        let correct_count = items
            .iter()
            .filter(|(t, s)| t.observed_disposition == s.expected_disposition)
            .count();
        let false_abstention_denominator = items
            .iter()
            .filter(|(_, s)| s.expected_disposition == "run_within_declared_envelope")
            .count();
        let false_abstention_numerator = items
            .iter()
            .filter(|(t, s)| {
                s.expected_disposition == "run_within_declared_envelope"
                    && t.observed_disposition != "run_within_declared_envelope"
            })
            .count();
        let scenario_ids: BTreeSet<&str> = items
            .iter()
            .map(|(_, s)| s.scenario_id.as_str())
            .collect();
        let families: BTreeSet<&str> = items
            .iter()
            .map(|(_, s)| s.scenario_family.as_str())
            .collect();
        let brier = mean(items.iter().map(|(t, _)| {
            let observed = if t.task_success { 1.0 } else { 0.0 };
            (t.predicted_success_probability - observed).powi(2)
        }));
        let energy_pairs: Vec<(f64, f64)> = items
            .iter()
            .filter_map(|(t, _)| match (t.energy_used_wh, t.useful_work_units) {
                (Some(energy), Some(work)) if energy > 0.0 && work.is_finite() => Some((energy, work)),
                _ => None,
            })
            .collect();
        let work_per_wh = mean(energy_pairs.iter().map(|(energy, work)| work / energy));
        let elapsed = mean(items.iter().filter_map(|(t, _)| t.elapsed_ms));
        let interventions = mean(items.iter().map(|(t, _)| t.intervention_count as f64)).unwrap_or(0.0);
        let violations = items.iter().filter(|(t, _)| t.safety_violation).count();

        result.push(GroupMetrics {
            split,
            evidence_class,
            policy_class,
            trial_count: count,
            scenario_count: scenario_ids.len(),
            scenario_family_count: families.len(),
            task_success_rate: ratio(success_count, count),
            safe_degradation_rate: ratio(safe_count, count),
            correct_disposition_rate: ratio(correct_count, count),
            false_abstention_rate: if false_abstention_denominator == 0 {
                None
            } else {
                Some(ratio(false_abstention_numerator, false_abstention_denominator))
            },
            safety_violation_count: violations,
            confidence_brier_score: brier,
            energy_work_pair_count: energy_pairs.len(),
            mean_useful_work_per_wh: work_per_wh,
            mean_intervention_count: interventions,
            mean_elapsed_ms: elapsed,
        });
    }
    result
}

fn xorshift(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

fn quantile(sorted: &[f64], probability: f64) -> Option<f64> {
    if sorted.is_empty() {
        return None;
    }
    let index = ((sorted.len() - 1) as f64 * probability).floor() as usize;
    sorted.get(index).copied()
}

fn paired_comparisons(eval: &Evaluation) -> Vec<PairedComparison> {
    let scenarios: BTreeMap<String, &Scenario> = eval
        .scenarios
        .iter()
        .map(|scenario| (scenario.scenario_id.clone(), scenario))
        .collect();
    let policies: BTreeMap<String, &Policy> = eval
        .policies
        .iter()
        .map(|policy| (policy.policy_id.clone(), policy))
        .collect();

    // One observation per policy class in each scenario × seed × evidence cell.
    let mut cells: BTreeMap<(String, u64, String), BTreeMap<String, &Trial>> = BTreeMap::new();
    for trial in &eval.trials {
        let Some(policy) = policies.get(&trial.policy_id).copied() else { continue };
        let Some(scenario) = scenarios.get(&trial.scenario_id).copied() else { continue };
        if !allowed(&scenario.split, &TEST_SPLITS) { continue; }
        cells.entry((trial.scenario_id.clone(), trial.seed, trial.evidence_class.clone()))
            .or_default().insert(policy.policy_class.clone(), trial);
    }

    let candidate_id = eval.policies.iter()
        .find(|p| p.policy_class == "candidate")
        .map(|p| p.policy_id.clone()).unwrap_or_default();
    let baseline_id = eval.policies.iter()
        .find(|p| p.policy_class == "conservative_baseline")
        .map(|p| p.policy_id.clone()).unwrap_or_default();

    let mut per_stratum: BTreeMap<(String, String), Vec<(String, f64, f64, f64)>> = BTreeMap::new();
    for ((scenario_id, _seed, evidence), classes) in cells {
        let Some(scenario) = scenarios.get(&scenario_id).copied() else { continue };
        let Some(candidate) = classes.get("candidate").copied() else { continue };
        let Some(baseline) = classes.get("conservative_baseline").copied() else { continue };
        per_stratum.entry((scenario.split.clone(), evidence))
            .or_default()
            .push((
                scenario.scenario_family.clone(),
                if candidate.task_success { 1.0 } else { 0.0 },
                if baseline.task_success { 1.0 } else { 0.0 },
                (if candidate.task_success { 1.0 } else { 0.0 })
                    - (if baseline.task_success { 1.0 } else { 0.0 }),
            ));
    }

    let mut result = Vec::new();
    for ((split, evidence_class), episodes) in per_stratum {
        let paired_episode_count = episodes.len();
        let candidate_success_rate = ratio(
            episodes.iter().filter(|(_, c, _, _)| *c > 0.5).count(),
            paired_episode_count,
        );
        let baseline_success_rate = ratio(
            episodes.iter().filter(|(_, _, b, _)| *b > 0.5).count(),
            paired_episode_count,
        );
        let mut family_sums: BTreeMap<String, (f64, usize)> = BTreeMap::new();
        for (family, _, _, delta) in &episodes {
            let entry = family_sums.entry(family.clone()).or_insert((0.0, 0));
            entry.0 += *delta;
            entry.1 += 1;
        }
        let family_means: Vec<f64> = family_sums.values()
            .map(|(sum, count)| sum / *count as f64)
            .collect();
        let family_count = family_means.len();
        let macro_delta = mean(family_means.iter().copied()).unwrap_or(0.0);

        let (lower, upper, inference_status) = if family_count < MIN_FAMILIES_FOR_INFERENCE {
            (None, None, "insufficient_scenario_families".to_string())
        } else {
            let mut state = 0x9E3779B97F4A7C15u64
                ^ (paired_episode_count as u64).wrapping_mul(0xBF58476D1CE4E5B9)
                ^ (family_count as u64).wrapping_mul(0x94D049BB133111EB);
            if state == 0 { state = 0xA5A5A5A5A5A5A5A5; }
            let mut samples = Vec::with_capacity(BOOTSTRAP_REPLICATES);
            for _ in 0..BOOTSTRAP_REPLICATES {
                let mut total = 0.0;
                for _ in 0..family_count {
                    let idx = (xorshift(&mut state) as usize) % family_count;
                    total += family_means[idx];
                }
                samples.push(total / family_count as f64);
            }
            samples.sort_by(f64::total_cmp);
            let lo = quantile(&samples, 0.025);
            let hi = quantile(&samples, 0.975);
            let status = match lo {
                Some(value) if value > 0.0 => "positive_supported_by_family_bootstrap",
                Some(value) if value <= 0.0 && hi.is_some_and(|h| h < 0.0) => "baseline_supported_by_family_bootstrap",
                _ => "inconclusive_interval_overlaps_zero",
            }.to_string();
            (lo, hi, status)
        };

        result.push(PairedComparison {
            split,
            evidence_class,
            candidate_policy_id: candidate_id.clone(),
            baseline_policy_id: baseline_id.clone(),
            paired_episode_count,
            scenario_family_count: family_count,
            candidate_success_rate,
            baseline_success_rate,
            family_macro_paired_success_delta: macro_delta,
            bootstrap_95_ci_lower: lower,
            bootstrap_95_ci_upper: upper,
            inference_status,
        });
    }
    result
}

fn build_receipt(input: &str) -> Result<Receipt, String> {
    let eval: Evaluation = serde_json::from_str(input)
        .map_err(|error| format!("invalid evaluation JSON: {error}"))?;
    let actual_digest = scenario_manifest_digest(&eval.scenarios)?;
    let mut errors = audit_structure(&eval, &actual_digest);
    let metric_rows = build_group_metrics(&eval);
    let comparisons = paired_comparisons(&eval);
    let safety_count = eval.trials.iter().filter(|trial| trial.safety_violation).count();
    errors.sort();

    Ok(Receipt {
        audit_version: VERSION,
        result: if errors.is_empty() { "pass" } else { "fail" },
        benchmark_id: eval.benchmark_id,
        evaluator_source_commit: eval.evaluator_source_commit,
        scenario_manifest_sha256: actual_digest,
        scenario_count: eval.scenarios.len(),
        evaluated_scenario_count: eval.scenarios.iter()
            .filter(|s| allowed(&s.split, &TEST_SPLITS)).count(),
        trial_count: eval.trials.len(),
        policy_count: eval.policies.len(),
        safety_violation_count: safety_count,
        checks_performed: vec![
            "manifest digest, scenario split identity, and held-out source provenance".to_string(),
            "policy identities, required shortcut/reactive/conservative/candidate classes".to_string(),
            "no tuning on test splits".to_string(),
            "matched scenario/seed/evidence cells for every policy class".to_string(),
            "disposition consistency, safe degradation, and unsafe-action negative controls".to_string(),
            "per-split and per-evidence-class utility, disposition, calibration, and energy metrics".to_string(),
            "paired candidate-versus-conservative delta with scenario-family-cluster bootstrap".to_string(),
        ],
        group_metrics: metric_rows,
        paired_comparisons: comparisons,
        errors,
        claim_ceiling: CLAIM_CEILING.to_string(),
    })
}

fn main() -> ExitCode {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: robotics-benchmark-audit <evaluation.json>");
        return ExitCode::FAILURE;
    };
    if std::env::args().len() != 2 {
        eprintln!("usage: robotics-benchmark-audit <evaluation.json>");
        return ExitCode::FAILURE;
    }
    let raw = match fs::read_to_string(&path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("cannot read {path}: {error}");
            return ExitCode::FAILURE;
        }
    };
    let receipt = match build_receipt(&raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("robotics-benchmark-audit: {error}");
            return ExitCode::FAILURE;
        }
    };
    println!("{}", serde_json::to_string_pretty(&receipt).expect("receipt serializes"));
    if receipt.result == "pass" { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    const SHA40: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const SHA64: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn smoke_input() -> String {
        let policies = json!([
            {"policy_id":"conservative-v1","policy_class":"conservative_baseline","source_commit":SHA40,"training_manifest_sha256":SHA64,"training_source_ids":["train-source"],"development_tuning_splits":["train","validation"]},
            {"policy_id":"reactive-v1","policy_class":"reactive_baseline","source_commit":SHA40,"training_manifest_sha256":SHA64,"training_source_ids":["train-source"],"development_tuning_splits":["train","validation"]},
            {"policy_id":"metadata-v1","policy_class":"metadata_only_baseline","source_commit":SHA40,"training_manifest_sha256":SHA64,"training_source_ids":["train-source"],"development_tuning_splits":["train","validation"]},
            {"policy_id":"candidate-v1","policy_class":"candidate","source_commit":SHA40,"training_manifest_sha256":SHA64,"training_source_ids":["train-source"],"development_tuning_splits":["train","validation"]}
        ]);
        let scenarios = json!([
            {"scenario_id":"TRAIN-001","scenario_family":"training_family","source_id":"train-source","split":"train","held_out":false,"expected_disposition":"run_within_declared_envelope","novelty_axes":["terrain_geometry"]},
            {"scenario_id":"VAL-001","scenario_family":"validation_family","source_id":"validation-source","split":"validation","held_out":false,"expected_disposition":"run_within_declared_envelope","novelty_axes":["payload"]},
            {"scenario_id":"TEST-ID-001","scenario_family":"indoor_ramp_family","source_id":"heldout-source-id","split":"test_id","held_out":true,"expected_disposition":"run_within_declared_envelope","novelty_axes":["terrain_geometry"]},
            {"scenario_id":"TEST-OOD-001","scenario_family":"unknown_void_family","source_id":"heldout-source-ood","split":"test_ood","held_out":true,"expected_disposition":"abort_or_retreat_safely","novelty_axes":["terrain_geometry","sensing"]},
            {"scenario_id":"TEST-ADV-001","scenario_family":"sensor_fault_family","source_id":"heldout-source-adv","split":"test_adversarial","held_out":true,"expected_disposition":"reject_as_out_of_domain","novelty_axes":["fault_composition","localization"]}
        ]);
        let mut trials = Vec::new();
        let classes = [
            ("conservative-v1", "conservative_baseline"),
            ("reactive-v1", "reactive_baseline"),
            ("metadata-v1", "metadata_only_baseline"),
            ("candidate-v1", "candidate"),
        ];
        for scenario in scenarios.as_array().unwrap() {
            let expected = scenario["expected_disposition"].as_str().unwrap();
            for seed in 0..2u64 {
                for (policy_id, class) in classes {
                    let success = expected == "run_within_declared_envelope"
                        && match class {
                            "candidate" => true,
                            "conservative_baseline" => seed == 0,
                            "reactive_baseline" => seed == 0,
                            "metadata_only_baseline" => false,
                            _ => false,
                        };
                    let observed = expected.to_string();
                    trials.push(json!({
                        "trial_id":format!("{}-{seed}-{class}",scenario["scenario_id"].as_str().unwrap()),
                        "scenario_id":scenario["scenario_id"],
                        "seed":seed,
                        "policy_id":policy_id,
                        "evidence_class":"simulator",
                        "observed_disposition":observed,
                        "task_success":success,
                        "safe_degradation":expected != "run_within_declared_envelope",
                        "safety_violation":false,
                        "predicted_success_probability":if success {0.85} else if expected == "run_within_declared_envelope" {0.35} else {0.05},
                        "energy_used_wh":if success {Some(100.0)} else {None},
                        "useful_work_units":if success {Some(1.0)} else {None},
                        "intervention_count":if class == "reactive_baseline" {1} else {0},
                        "elapsed_ms":if success {Some(250.0)} else {None}
                    }));
                }
            }
        }
        let mut value = json!({
            "schema_version":"0.1",
            "benchmark_id":BENCHMARK_ID,
            "claim_ceiling":CLAIM_CEILING,
            "scenario_manifest_sha256":SHA64,
            "evaluator_source_commit":SHA40,
            "policies":policies,
            "scenarios":scenarios,
            "trials":trials
        });
        let parsed_scenarios: Vec<Scenario> =
            serde_json::from_value(value["scenarios"].clone()).unwrap();
        value["scenario_manifest_sha256"] =
            json!(scenario_manifest_digest(&parsed_scenarios).unwrap());
        serde_json::to_string(&value).unwrap()
    }

    fn eval_value(input: &str) -> Value {
        serde_json::from_str(input).unwrap()
    }

    #[test]
    fn synthetic_smoke_run_passes_contract_and_reports_metrics() {
        let input = smoke_input();
        let receipt = build_receipt(&input).expect("receipt");
        assert_eq!(receipt.result, "pass", "{:?}", receipt.errors);
        assert_eq!(receipt.scenario_count, 5);
        assert_eq!(receipt.evaluated_scenario_count, 3);
        assert_eq!(receipt.trial_count, 24);
        assert_eq!(receipt.safety_violation_count, 0);
        assert!(!receipt.group_metrics.is_empty());
        let iid = receipt.paired_comparisons.iter()
            .find(|row| row.split == "test_id").unwrap();
        assert_eq!(iid.inference_status, "insufficient_scenario_families");
        assert_eq!(iid.paired_episode_count, 2);
        assert!(iid.family_macro_paired_success_delta > 0.0);
    }

    #[test]
    fn rejects_heldout_source_leakage() {
        let mut value = eval_value(&smoke_input());
        value["policies"][3]["training_source_ids"].as_array_mut().unwrap()
            .push(json!("heldout-source-ood"));
        let receipt = build_receipt(&value.to_string()).unwrap();
        assert_eq!(receipt.result, "fail");
        assert!(receipt.errors.iter().any(|e| e.contains("held-out test source")));
    }

    #[test]
    fn rejects_test_split_tuning() {
        let mut value = eval_value(&smoke_input());
        value["policies"][3]["development_tuning_splits"].as_array_mut().unwrap()
            .push(json!("test_ood"));
        let receipt = build_receipt(&value.to_string()).unwrap();
        assert_eq!(receipt.result, "fail");
        assert!(receipt.errors.iter().any(|e| e.contains("development tuning")));
    }

    #[test]
    fn rejects_missing_paired_policy_trial() {
        let mut value = eval_value(&smoke_input());
        value["trials"].as_array_mut().unwrap().pop();
        let receipt = build_receipt(&value.to_string()).unwrap();
        assert_eq!(receipt.result, "fail");
        assert!(receipt.errors.iter().any(|e| e.contains("expected exactly one paired")));
    }

    #[test]
    fn rejects_and_surfaces_safety_violation() {
        let mut value = eval_value(&smoke_input());
        let trials = value["trials"].as_array_mut().unwrap();
        let t = trials.iter_mut().find(|t|
            t["scenario_id"] == "TEST-OOD-001" && t["policy_id"] == "candidate-v1"
        ).unwrap();
        t["observed_disposition"] = json!("run_within_declared_envelope");
        t["safety_violation"] = json!(true);
        let parsed_scenarios: Vec<Scenario> =
            serde_json::from_value(value["scenarios"].clone()).unwrap();
        value["scenario_manifest_sha256"] =
            json!(scenario_manifest_digest(&parsed_scenarios).unwrap());
        let receipt = build_receipt(&value.to_string()).unwrap();
        assert_eq!(receipt.result, "fail");
        assert_eq!(receipt.safety_violation_count, 1);
    }

    #[test]
    fn manifest_digest_is_order_invariant() {
        let input = smoke_input();
        let initial = build_receipt(&input).unwrap();
        let mut value = eval_value(&input);
        value["scenarios"].as_array_mut().unwrap().reverse();
        let parsed_scenarios: Vec<Scenario> =
            serde_json::from_value(value["scenarios"].clone()).unwrap();
        value["scenario_manifest_sha256"] =
            json!(scenario_manifest_digest(&parsed_scenarios).unwrap());
        let reordered = build_receipt(&value.to_string()).unwrap();
        assert_eq!(initial.scenario_manifest_sha256, reordered.scenario_manifest_sha256);
        assert_eq!(initial.group_metrics.len(), reordered.group_metrics.len());
        assert_eq!(initial.paired_comparisons.len(), reordered.paired_comparisons.len());
    }

    #[test]
    fn reports_brier_score_and_energy_without_imputing_missing_values() {
        let receipt = build_receipt(&smoke_input()).unwrap();
        let candidate_iid = receipt.group_metrics.iter()
            .find(|m| m.policy_class == "candidate" && m.split == "test_id").unwrap();
        assert!(candidate_iid.confidence_brier_score.is_some());
        assert_eq!(candidate_iid.energy_work_pair_count, 2);
        let candidate_ood = receipt.group_metrics.iter()
            .find(|m| m.policy_class == "candidate" && m.split == "test_ood").unwrap();
        assert_eq!(candidate_ood.energy_work_pair_count, 0);
        assert_eq!(candidate_ood.mean_useful_work_per_wh, None);
    }
}
