// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const SCHEMA_ID: &str = "luminous.symtropy.ocean-observation-allocation";
const SCHEMA_VERSION: &str = "0.1.0";
const SUITE_ID: &str = "OCEAN-OBSERVATION-ALLOCATION-SYNTH-001";
const DEFAULT_CORPUS: &str = "docs/tech/OCEAN_OBSERVATION_ALLOCATION_CORPUS_V0_1.json";
const MAX_CORPUS_BYTES: usize = 250_000;
const MAX_CANDIDATES_PER_SCENARIO: usize = 16;
const SCALE_PPM: u64 = 1_000_000;
const SCENARIO_IDS: &[&str] = &[
    "access-biased-sampling",
    "depth-gap-and-detection-limit",
    "independent-exposure-caps",
    "repeat-versus-coverage",
];

type Check<T> = Result<T, String>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ObservationMode {
    PassiveNonContact,
    ActiveNonContact,
    Contact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum RightsReviewStatus {
    Reviewed,
    ReviewRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum UsePermission {
    Permitted,
    Restricted,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ExposureKnowledge {
    Known,
    Unknown,
}

/// Synthetic detectability profile. Values are normalized ppm units, not physical
/// sensor calibration. Missing values mean the detection profile is incomplete.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DetectionProfile {
    minimum_detectable_signal_ppm: Option<u64>,
    expected_signal_ppm: Option<u64>,
    detection_probability_ppm: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Exposure {
    sound_units: u32,
    light_units: u32,
    sediment_units: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Budget {
    effort_units: u32,
    sound_units: u32,
    light_units: u32,
    sediment_units: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Stratum {
    stratum_id: String,
    /// Declared decision importance, normalized to 1..=1,000,000 ppm.
    priority_ppm: u64,
    /// Synthetic uncertainty remaining before this allocation, in ppm.
    uncertainty_ppm: u64,
    /// Historical observation count; not abundance, occupancy, or absence evidence.
    existing_record_count: u64,
    /// Historical sampling effort used to expose access bias; not a quality score.
    historical_effort_units: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Candidate {
    candidate_id: String,
    stratum_id: String,
    observation_mode: ObservationMode,
    rights_review_status: RightsReviewStatus,
    use_permission: UsePermission,
    /// Absent or incomplete profile is an explicit non-eligibility signal.
    detection_profile: Option<DetectionProfile>,
    exposure_knowledge: ExposureKnowledge,
    effort_units: u32,
    /// Nominal synthetic information gain at perfect detection, before probability discount and the uncertainty cap.
    information_gain_ppm: u64,
    /// Exposure in separate declared channels. Unknown channels never become zero.
    exposure: Exposure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Scenario {
    scenario_id: String,
    description: String,
    budget: Budget,
    strata: Vec<Stratum>,
    candidates: Vec<Candidate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    schema_id: String,
    schema_version: String,
    suite_id: String,
    evidence_class: String,
    claim_ceiling: String,
    seed: u64,
    scenarios: Vec<Scenario>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Policy {
    RandomFixedSeed,
    LargestCoverageGap,
    RawRecordCountBaseline,
    EffortAwareStratified,
    UncertaintyReductionGreedy,
    ExactSmallCaseOracle,
}

const POLICIES: &[Policy] = &[
    Policy::RandomFixedSeed,
    Policy::LargestCoverageGap,
    Policy::RawRecordCountBaseline,
    Policy::EffortAwareStratified,
    Policy::UncertaintyReductionGreedy,
    Policy::ExactSmallCaseOracle,
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PolicyReport {
    policy: Policy,
    selected_candidate_ids: Vec<String>,
    weighted_coverage_ppm: u64,
    information_score_units: u64,
    raw_uncertainty_reduction_ppm: u64,
    effort_used: u32,
    exposure_used: Exposure,
    uncovered_stratum_ids: Vec<String>,
    oracle_regret_units: u64,
    /// True when no other policy is at least as good on every declared metric
    /// and strictly better on at least one metric.
    pareto_nondominated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScenarioReport {
    scenario_id: String,
    eligible_candidate_count: usize,
    policy_reports: Vec<PolicyReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct InvariantResult {
    invariant_id: String,
    passed: bool,
    detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct VerificationReceipt {
    schema_id: String,
    schema_version: String,
    tool_version: String,
    suite_id: String,
    corpus_sha256: String,
    source_commit: String,
    evidence_class: String,
    claim_ceiling: String,
    seed: u64,
    scenario_count: usize,
    passed_invariant_count: usize,
    failed_invariant_count: usize,
    result: String,
    invariants: Vec<InvariantResult>,
    scenarios: Vec<ScenarioReport>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct ExposureUsed {
    sound_units: u32,
    light_units: u32,
    sediment_units: u32,
}

impl ExposureUsed {
    fn add(self, exposure: Exposure) -> Self {
        Self {
            sound_units: self.sound_units.saturating_add(exposure.sound_units),
            light_units: self.light_units.saturating_add(exposure.light_units),
            sediment_units: self.sediment_units.saturating_add(exposure.sediment_units),
        }
    }

    fn within(self, budget: Budget) -> bool {
        self.sound_units <= budget.sound_units
            && self.light_units <= budget.light_units
            && self.sediment_units <= budget.sediment_units
    }
}

#[derive(Debug, Clone)]
struct SelectionState {
    selected: Vec<usize>,
    selected_flags: Vec<bool>,
    remaining_uncertainty: Vec<u64>,
    covered: Vec<bool>,
    effort_used: u32,
    exposure_used: ExposureUsed,
    information_score_units: u64,
    raw_uncertainty_reduction_ppm: u64,
}

impl SelectionState {
    fn new(scenario: &Scenario) -> Self {
        Self {
            selected: Vec::new(),
            selected_flags: vec![false; scenario.candidates.len()],
            remaining_uncertainty: scenario.strata.iter().map(|s| s.uncertainty_ppm).collect(),
            covered: vec![false; scenario.strata.len()],
            effort_used: 0,
            exposure_used: ExposureUsed::default(),
            information_score_units: 0,
            raw_uncertainty_reduction_ppm: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
struct Rank {
    primary: u128,
    secondary: u128,
    tertiary: u128,
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn validate_corpus(corpus: &Corpus) -> Check<()> {
    if corpus.schema_id != SCHEMA_ID || corpus.schema_version != SCHEMA_VERSION {
        return Err("unsupported schema ID or version".into());
    }
    if corpus.suite_id != SUITE_ID {
        return Err("unexpected suite ID".into());
    }
    if corpus.evidence_class != "SyntheticAdversarial" {
        return Err("corpus must be explicitly SyntheticAdversarial".into());
    }
    if corpus.claim_ceiling.trim().is_empty() {
        return Err("claim ceiling must be present".into());
    }
    if corpus.scenarios.len() != SCENARIO_IDS.len() {
        return Err(format!(
            "expected {} frozen scenarios, got {}",
            SCENARIO_IDS.len(),
            corpus.scenarios.len()
        ));
    }
    for (scenario_index, scenario) in corpus.scenarios.iter().enumerate() {
        if scenario.scenario_id != SCENARIO_IDS[scenario_index] {
            return Err(format!(
                "scenario {scenario_index} must be {}, got {}",
                SCENARIO_IDS[scenario_index], scenario.scenario_id
            ));
        }
        if scenario.description.trim().is_empty() {
            return Err(format!(
                "scenario {} has an empty description",
                scenario.scenario_id
            ));
        }
        if scenario.strata.is_empty() || scenario.candidates.is_empty() {
            return Err(format!(
                "scenario {} must have strata and candidates",
                scenario.scenario_id
            ));
        }
        if scenario.candidates.len() > MAX_CANDIDATES_PER_SCENARIO {
            return Err(format!(
                "scenario {} has {} candidates; exact oracle limit is {}",
                scenario.scenario_id,
                scenario.candidates.len(),
                MAX_CANDIDATES_PER_SCENARIO
            ));
        }

        let mut stratum_ids = HashSet::new();
        for stratum in &scenario.strata {
            if stratum.stratum_id.trim().is_empty()
                || !stratum_ids.insert(stratum.stratum_id.as_str())
            {
                return Err(format!(
                    "scenario {} has a blank/duplicate stratum ID",
                    scenario.scenario_id
                ));
            }
            if stratum.priority_ppm == 0 || stratum.priority_ppm > SCALE_PPM {
                return Err(format!(
                    "stratum {} priority must be in 1..=1,000,000 ppm",
                    stratum.stratum_id
                ));
            }
            if stratum.uncertainty_ppm > SCALE_PPM {
                return Err(format!(
                    "stratum {} uncertainty exceeds 1,000,000 ppm",
                    stratum.stratum_id
                ));
            }
        }

        let mut candidate_ids = HashSet::new();
        for candidate in &scenario.candidates {
            if candidate.candidate_id.trim().is_empty()
                || !candidate_ids.insert(candidate.candidate_id.as_str())
            {
                return Err(format!(
                    "scenario {} has a blank/duplicate candidate ID",
                    scenario.scenario_id
                ));
            }
            if !stratum_ids.contains(candidate.stratum_id.as_str()) {
                return Err(format!(
                    "candidate {} references missing stratum {}",
                    candidate.candidate_id, candidate.stratum_id
                ));
            }
            if candidate.effort_units == 0 {
                return Err(format!(
                    "candidate {} must have positive effort",
                    candidate.candidate_id
                ));
            }
            if candidate.information_gain_ppm > SCALE_PPM {
                return Err(format!(
                    "candidate {} information gain exceeds 1,000,000 ppm",
                    candidate.candidate_id
                ));
            }
            if let Some(profile) = candidate.detection_profile {
                for (name, value) in [
                    (
                        "minimum_detectable_signal_ppm",
                        profile.minimum_detectable_signal_ppm,
                    ),
                    ("expected_signal_ppm", profile.expected_signal_ppm),
                    (
                        "detection_probability_ppm",
                        profile.detection_probability_ppm,
                    ),
                ] {
                    if value.is_some_and(|ppm| ppm > SCALE_PPM) {
                        return Err(format!(
                            "candidate {} {name} exceeds 1,000,000 ppm",
                            candidate.candidate_id
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

fn corpus_from_bytes(bytes: &[u8]) -> Check<Corpus> {
    if bytes.len() > MAX_CORPUS_BYTES {
        return Err(format!("corpus exceeds {MAX_CORPUS_BYTES} byte limit"));
    }
    let corpus: Corpus =
        serde_json::from_slice(bytes).map_err(|error| format!("invalid corpus JSON: {error}"))?;
    validate_corpus(&corpus)?;
    Ok(corpus)
}

fn source_commit(explicit: Option<String>) -> Check<String> {
    let candidate = explicit
        .or_else(|| env::var("OCEAN_ALLOCATION_SOURCE_COMMIT").ok())
        .or_else(|| env::var("GITHUB_SHA").ok())
        .ok_or_else(|| {
            "source commit is required (use --source-commit, OCEAN_ALLOCATION_SOURCE_COMMIT, or GITHUB_SHA)".to_owned()
        })?;
    if !(40..=64).contains(&candidate.len())
        || !candidate.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("source commit must be a 40-64 character hexadecimal Git object ID".into());
    }
    Ok(candidate.to_ascii_lowercase())
}

fn stratum_index(scenario: &Scenario, id: &str) -> usize {
    scenario
        .strata
        .iter()
        .position(|stratum| stratum.stratum_id == id)
        .expect("corpus validation ensures stratum references exist")
}

fn detection_profile_is_adequate(candidate: &Candidate) -> bool {
    let Some(profile) = candidate.detection_profile else {
        return false;
    };
    match (
        profile.minimum_detectable_signal_ppm,
        profile.expected_signal_ppm,
        profile.detection_probability_ppm,
    ) {
        (Some(minimum), Some(expected), Some(probability)) => {
            minimum <= SCALE_PPM
                && expected <= SCALE_PPM
                && probability > 0
                && probability <= SCALE_PPM
                && expected >= minimum
        }
        _ => false,
    }
}

fn expected_information_gain_ppm(candidate: &Candidate) -> u64 {
    let probability = candidate
        .detection_profile
        .and_then(|profile| profile.detection_probability_ppm)
        .expect("only candidates with a complete profile are eligible");
    ((u128::from(candidate.information_gain_ppm) * u128::from(probability)) / u128::from(SCALE_PPM))
        as u64
}

fn is_eligible(candidate: &Candidate) -> bool {
    matches!(
        candidate.observation_mode,
        ObservationMode::PassiveNonContact | ObservationMode::ActiveNonContact
    ) && candidate.rights_review_status == RightsReviewStatus::Reviewed
        && candidate.use_permission == UsePermission::Permitted
        && detection_profile_is_adequate(candidate)
        && candidate.exposure_knowledge == ExposureKnowledge::Known
}

fn can_add(scenario: &Scenario, state: &SelectionState, candidate_index: usize) -> bool {
    let candidate = &scenario.candidates[candidate_index];
    if state.selected_flags[candidate_index] || !is_eligible(candidate) {
        return false;
    }
    if state.effort_used.saturating_add(candidate.effort_units) > scenario.budget.effort_units {
        return false;
    }
    state
        .exposure_used
        .add(candidate.exposure)
        .within(scenario.budget)
}

fn add_candidate(scenario: &Scenario, state: &mut SelectionState, index: usize) {
    let candidate = &scenario.candidates[index];
    let stratum_index = stratum_index(scenario, &candidate.stratum_id);
    let reduction =
        expected_information_gain_ppm(candidate).min(state.remaining_uncertainty[stratum_index]);
    let priority = scenario.strata[stratum_index].priority_ppm;
    let weighted_score = (u128::from(reduction) * u128::from(priority) / u128::from(SCALE_PPM))
        .min(u128::from(u64::MAX)) as u64;

    state.selected.push(index);
    state.selected_flags[index] = true;
    state.remaining_uncertainty[stratum_index] -= reduction;
    state.covered[stratum_index] = true;
    state.effort_used = state.effort_used.saturating_add(candidate.effort_units);
    state.exposure_used = state.exposure_used.add(candidate.exposure);
    state.information_score_units = state.information_score_units.saturating_add(weighted_score);
    state.raw_uncertainty_reduction_ppm = state
        .raw_uncertainty_reduction_ppm
        .saturating_add(reduction);
}

fn stable_hash(seed: u64, id: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in seed
        .to_le_bytes()
        .into_iter()
        .chain(id.as_bytes().iter().copied())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn rank_candidate(
    scenario: &Scenario,
    state: &SelectionState,
    candidate_index: usize,
    policy: Policy,
    seed: u64,
) -> Rank {
    let candidate = &scenario.candidates[candidate_index];
    let s_idx = stratum_index(scenario, &candidate.stratum_id);
    let stratum = &scenario.strata[s_idx];
    let remaining = state.remaining_uncertainty[s_idx];
    let marginal_gain = expected_information_gain_ppm(candidate).min(remaining);
    let marginal_score =
        u128::from(marginal_gain) * u128::from(stratum.priority_ppm) / u128::from(SCALE_PPM);
    let uncovered_first = if state.covered[s_idx] { 0 } else { 1 };
    let effort = u128::from(candidate.effort_units.max(1));

    match policy {
        Policy::RandomFixedSeed => Rank {
            primary: u128::from(u64::MAX - stable_hash(seed, &candidate.candidate_id)),
            secondary: 0,
            tertiary: 0,
        },
        Policy::LargestCoverageGap => Rank {
            primary: uncovered_first,
            secondary: u128::from(remaining) * u128::from(stratum.priority_ppm),
            tertiary: marginal_score,
        },
        Policy::RawRecordCountBaseline => Rank {
            primary: u128::from(stratum.existing_record_count),
            secondary: marginal_score,
            tertiary: u128::from(u64::MAX - u64::from(candidate.effort_units)),
        },
        Policy::EffortAwareStratified => {
            // Prefer an uncovered stratum; within it, discount already heavily sampled strata.
            let denominator = effort
                .saturating_mul(u128::from(stratum.historical_effort_units).saturating_add(1));
            Rank {
                primary: uncovered_first,
                secondary: (marginal_score.saturating_mul(u128::from(SCALE_PPM)))
                    / denominator.max(1),
                tertiary: u128::from(remaining),
            }
        }
        Policy::UncertaintyReductionGreedy => Rank {
            primary: marginal_score.saturating_mul(u128::from(SCALE_PPM)) / effort,
            secondary: uncovered_first,
            tertiary: u128::from(remaining),
        },
        Policy::ExactSmallCaseOracle => Rank::default(),
    }
}

fn select_greedy(scenario: &Scenario, policy: Policy, seed: u64) -> SelectionState {
    let mut state = SelectionState::new(scenario);
    loop {
        let mut best: Option<(usize, Rank)> = None;
        for candidate_index in 0..scenario.candidates.len() {
            if !can_add(scenario, &state, candidate_index) {
                continue;
            }
            let rank = rank_candidate(scenario, &state, candidate_index, policy, seed);
            match best {
                None => best = Some((candidate_index, rank)),
                Some((best_index, best_rank)) => {
                    let id = &scenario.candidates[candidate_index].candidate_id;
                    let best_id = &scenario.candidates[best_index].candidate_id;
                    if rank > best_rank || (rank == best_rank && id < best_id) {
                        best = Some((candidate_index, rank));
                    }
                }
            }
        }
        let Some((candidate_index, _)) = best else {
            break;
        };
        add_candidate(scenario, &mut state, candidate_index);
    }
    state
}

fn selection_for_mask(
    scenario: &Scenario,
    mask: u32,
    eligible: &[usize],
) -> Option<SelectionState> {
    let mut state = SelectionState::new(scenario);
    for (bit, candidate_index) in eligible.iter().enumerate() {
        if mask & (1_u32 << bit) != 0 {
            if !can_add(scenario, &state, *candidate_index) {
                return None;
            }
            add_candidate(scenario, &mut state, *candidate_index);
        }
    }
    Some(state)
}

fn exact_oracle(scenario: &Scenario) -> SelectionState {
    let eligible: Vec<usize> = (0..scenario.candidates.len())
        .filter(|index| is_eligible(&scenario.candidates[*index]))
        .collect();
    assert!(eligible.len() <= MAX_CANDIDATES_PER_SCENARIO);
    let limit = 1_u32 << eligible.len();
    let mut best: Option<SelectionState> = None;
    for mask in 0..limit {
        let Some(candidate) = selection_for_mask(scenario, mask, &eligible) else {
            continue;
        };
        let replace = match &best {
            None => true,
            Some(current) => {
                let candidate_coverage = weighted_coverage_ppm(scenario, &candidate);
                let current_coverage = weighted_coverage_ppm(scenario, current);
                candidate.information_score_units > current.information_score_units
                    || (candidate.information_score_units == current.information_score_units
                        && candidate_coverage > current_coverage)
                    || (candidate.information_score_units == current.information_score_units
                        && candidate_coverage == current_coverage
                        && candidate.effort_used < current.effort_used)
                    || (candidate.information_score_units == current.information_score_units
                        && candidate_coverage == current_coverage
                        && candidate.effort_used == current.effort_used
                        && selected_ids(scenario, &candidate) < selected_ids(scenario, current))
            }
        };
        if replace {
            best = Some(candidate);
        }
    }
    best.expect("empty candidate subset always satisfies non-negative budgets")
}

fn selected_ids(scenario: &Scenario, state: &SelectionState) -> Vec<String> {
    state
        .selected
        .iter()
        .map(|index| scenario.candidates[*index].candidate_id.clone())
        .collect()
}

fn weighted_coverage_ppm(scenario: &Scenario, state: &SelectionState) -> u64 {
    let denominator: u128 = scenario
        .strata
        .iter()
        .map(|stratum| u128::from(stratum.priority_ppm))
        .sum();
    if denominator == 0 {
        return 0;
    }
    let covered: u128 = scenario
        .strata
        .iter()
        .enumerate()
        .filter(|(index, _)| state.covered[*index])
        .map(|(_, stratum)| u128::from(stratum.priority_ppm))
        .sum();
    (covered.saturating_mul(u128::from(SCALE_PPM)) / denominator).min(u128::from(SCALE_PPM)) as u64
}

fn dominates(left: &PolicyReport, right: &PolicyReport) -> bool {
    let no_worse = left.weighted_coverage_ppm >= right.weighted_coverage_ppm
        && left.information_score_units >= right.information_score_units
        && left.raw_uncertainty_reduction_ppm >= right.raw_uncertainty_reduction_ppm
        && left.effort_used <= right.effort_used
        && left.exposure_used.sound_units <= right.exposure_used.sound_units
        && left.exposure_used.light_units <= right.exposure_used.light_units
        && left.exposure_used.sediment_units <= right.exposure_used.sediment_units;

    let strictly_better = left.weighted_coverage_ppm > right.weighted_coverage_ppm
        || left.information_score_units > right.information_score_units
        || left.raw_uncertainty_reduction_ppm > right.raw_uncertainty_reduction_ppm
        || left.effort_used < right.effort_used
        || left.exposure_used.sound_units < right.exposure_used.sound_units
        || left.exposure_used.light_units < right.exposure_used.light_units
        || left.exposure_used.sediment_units < right.exposure_used.sediment_units;

    no_worse && strictly_better
}

fn report_for(
    scenario: &Scenario,
    policy: Policy,
    state: SelectionState,
    oracle_score: u64,
) -> PolicyReport {
    let uncovered_stratum_ids = scenario
        .strata
        .iter()
        .enumerate()
        .filter(|(index, _)| !state.covered[*index])
        .map(|(_, stratum)| stratum.stratum_id.clone())
        .collect();
    PolicyReport {
        policy,
        selected_candidate_ids: selected_ids(scenario, &state),
        weighted_coverage_ppm: weighted_coverage_ppm(scenario, &state),
        information_score_units: state.information_score_units,
        raw_uncertainty_reduction_ppm: state.raw_uncertainty_reduction_ppm,
        effort_used: state.effort_used,
        exposure_used: Exposure {
            sound_units: state.exposure_used.sound_units,
            light_units: state.exposure_used.light_units,
            sediment_units: state.exposure_used.sediment_units,
        },
        uncovered_stratum_ids,
        oracle_regret_units: oracle_score.saturating_sub(state.information_score_units),
        pareto_nondominated: false,
    }
}

fn evaluate_scenario(scenario: &Scenario, seed: u64) -> ScenarioReport {
    let oracle = exact_oracle(scenario);
    let oracle_score = oracle.information_score_units;
    let mut policy_reports = Vec::with_capacity(POLICIES.len());
    for policy in POLICIES {
        let state = match policy {
            Policy::ExactSmallCaseOracle => oracle.clone(),
            _ => select_greedy(scenario, *policy, seed),
        };
        policy_reports.push(report_for(scenario, *policy, state, oracle_score));
    }
    for index in 0..policy_reports.len() {
        let is_dominated = (0..policy_reports.len()).any(|other_index| {
            other_index != index && dominates(&policy_reports[other_index], &policy_reports[index])
        });
        policy_reports[index].pareto_nondominated = !is_dominated;
    }
    ScenarioReport {
        scenario_id: scenario.scenario_id.clone(),
        eligible_candidate_count: scenario
            .candidates
            .iter()
            .filter(|candidate| is_eligible(candidate))
            .count(),
        policy_reports,
    }
}

fn add_invariant(
    invariants: &mut Vec<InvariantResult>,
    id: impl Into<String>,
    passed: bool,
    detail: impl Into<String>,
) {
    invariants.push(InvariantResult {
        invariant_id: id.into(),
        passed,
        detail: detail.into(),
    });
}

fn build_receipt(corpus: &Corpus, bytes: &[u8], commit: String) -> VerificationReceipt {
    let scenario_reports: Vec<ScenarioReport> = corpus
        .scenarios
        .iter()
        .map(|scenario| evaluate_scenario(scenario, corpus.seed))
        .collect();
    let mut invariants = Vec::new();

    let mut all_budgets_respected = true;
    let mut no_ineligible_selected = true;
    let mut oracle_dominates_score = true;
    for (scenario, report) in corpus.scenarios.iter().zip(&scenario_reports) {
        let candidate_by_id: std::collections::BTreeMap<&str, &Candidate> = scenario
            .candidates
            .iter()
            .map(|candidate| (candidate.candidate_id.as_str(), candidate))
            .collect();
        for policy_report in &report.policy_reports {
            let mut effort = 0_u32;
            let mut exposure = ExposureUsed::default();
            for candidate_id in &policy_report.selected_candidate_ids {
                match candidate_by_id.get(candidate_id.as_str()) {
                    Some(candidate) if is_eligible(candidate) => {
                        effort = effort.saturating_add(candidate.effort_units);
                        exposure = exposure.add(candidate.exposure);
                    }
                    _ => no_ineligible_selected = false,
                }
            }
            if effort > scenario.budget.effort_units || !exposure.within(scenario.budget) {
                all_budgets_respected = false;
            }
        }
        let oracle_score = report
            .policy_reports
            .iter()
            .find(|item| item.policy == Policy::ExactSmallCaseOracle)
            .map(|item| item.information_score_units)
            .unwrap_or(0);
        if report.policy_reports.iter().any(|item| {
            item.information_score_units > oracle_score
                || (item.policy == Policy::ExactSmallCaseOracle && item.oracle_regret_units != 0)
        }) {
            oracle_dominates_score = false;
        }
    }
    add_invariant(
        &mut invariants,
        "separate_effort_and_exposure_budgets",
        all_budgets_respected,
        "Every selected set stays within effort and each exposure-channel cap independently.",
    );
    add_invariant(
        &mut invariants,
        "no_contact_or_unpermitted_or_inadequately_detectable_action_selected",
        no_ineligible_selected,
        "Only reviewed-and-permitted non-contact candidates with complete, adequate detection profiles and known exposure can be selected.",
    );
    add_invariant(
        &mut invariants,
        "exact_oracle_zero_regret",
        oracle_dominates_score,
        "The exhaustive oracle reports zero regret against its own declared objective.",
    );

    if let Some(biased) = scenario_reports
        .iter()
        .find(|report| report.scenario_id == "access-biased-sampling")
    {
        let aware = biased
            .policy_reports
            .iter()
            .find(|report| report.policy == Policy::EffortAwareStratified);
        let raw = biased
            .policy_reports
            .iter()
            .find(|report| report.policy == Policy::RawRecordCountBaseline);
        let passed = match (aware, raw) {
            (Some(aware), Some(raw)) => aware.weighted_coverage_ppm > raw.weighted_coverage_ppm,
            _ => false,
        };
        add_invariant(
            &mut invariants,
            "effort_aware_coverage_beats_record_count_in_frozen_access_bias_case",
            passed,
            "In the preregistered synthetic access-bias scenario, the effort-aware policy must cover more declared-priority weight than the raw-count baseline.",
        );
    }

    let pareto_frontier_consistent = scenario_reports.iter().all(|scenario| {
        scenario
            .policy_reports
            .iter()
            .enumerate()
            .all(|(index, report)| {
                let expected_frontier = !(0..scenario.policy_reports.len()).any(|other_index| {
                    other_index != index && dominates(&scenario.policy_reports[other_index], report)
                });
                report.pareto_nondominated == expected_frontier
            })
    });
    add_invariant(
        &mut invariants,
        "pareto_frontier_is_consistent_with_separate_metrics",
        pareto_frontier_consistent,
        "Frontier membership is based on coverage, declared information score, uncertainty reduction, effort and each exposure channel separately; no weighted safety score is used.",
    );

    // No biological presence/absence labels exist in this schema: record counts
    // remain sampling metadata and cannot be converted into inferred ecology.
    add_invariant(
        &mut invariants,
        "record_counts_never_emit_biological_absence",
        true,
        "The benchmark schema contains no species-presence or absence conclusion field.",
    );

    let passed_invariant_count = invariants.iter().filter(|result| result.passed).count();
    let failed_invariant_count = invariants.len() - passed_invariant_count;
    VerificationReceipt {
        schema_id: SCHEMA_ID.to_owned(),
        schema_version: SCHEMA_VERSION.to_owned(),
        tool_version: VERSION.to_owned(),
        suite_id: SUITE_ID.to_owned(),
        corpus_sha256: sha256_hex(bytes),
        source_commit: commit,
        evidence_class: corpus.evidence_class.clone(),
        claim_ceiling: corpus.claim_ceiling.clone(),
        seed: corpus.seed,
        scenario_count: scenario_reports.len(),
        passed_invariant_count,
        failed_invariant_count,
        result: if failed_invariant_count == 0 {
            "PASS"
        } else {
            "FAIL"
        }
        .to_owned(),
        invariants,
        scenarios: scenario_reports,
    }
}

fn run() -> Check<ExitCode> {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut corpus_path = PathBuf::from(DEFAULT_CORPUS);
    let mut receipt_path: Option<PathBuf> = None;
    let mut explicit_commit = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--corpus" => {
                index += 1;
                corpus_path = PathBuf::from(args.get(index).ok_or("--corpus requires a path")?);
            }
            "--receipt" => {
                index += 1;
                receipt_path = Some(PathBuf::from(
                    args.get(index).ok_or("--receipt requires a path")?,
                ));
            }
            "--source-commit" => {
                index += 1;
                explicit_commit = Some(
                    args.get(index)
                        .ok_or("--source-commit requires a Git object ID")?
                        .clone(),
                );
            }
            other => return Err(format!("unknown argument: {other}")),
        }
        index += 1;
    }

    let bytes = fs::read(&corpus_path)
        .map_err(|error| format!("cannot read corpus {}: {error}", corpus_path.display()))?;
    let corpus = corpus_from_bytes(&bytes)?;
    let commit = source_commit(explicit_commit)?;
    let receipt = build_receipt(&corpus, &bytes, commit);
    let serialized = serde_json::to_vec_pretty(&receipt)
        .map_err(|error| format!("cannot serialize receipt: {error}"))?;
    if let Some(path) = receipt_path {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent).map_err(|error| {
                format!(
                    "cannot create receipt directory {}: {error}",
                    parent.display()
                )
            })?;
        }
        fs::write(&path, &serialized)
            .map_err(|error| format!("cannot write receipt {}: {error}", path.display()))?;
    }
    println!(
        "suite={} corpus_sha256={} scenarios={} invariants={}/{} result={}",
        receipt.suite_id,
        receipt.corpus_sha256,
        receipt.scenario_count,
        receipt.passed_invariant_count,
        receipt.passed_invariant_count + receipt.failed_invariant_count,
        receipt.result
    );
    for invariant in &receipt.invariants {
        println!(
            "invariant={} result={} detail={}",
            invariant.invariant_id,
            if invariant.passed { "PASS" } else { "FAIL" },
            invariant.detail
        );
    }
    for scenario in &receipt.scenarios {
        println!(
            "scenario={} eligible_candidates={}",
            scenario.scenario_id, scenario.eligible_candidate_count
        );
        for policy in &scenario.policy_reports {
            println!(
                "policy={:?} pareto_nondominated={} weighted_coverage_ppm={} information_score_units={} raw_uncertainty_reduction_ppm={} effort={} sound={} light={} sediment={} oracle_regret_units={} selected={}",
                policy.policy,
                policy.pareto_nondominated,
                policy.weighted_coverage_ppm,
                policy.information_score_units,
                policy.raw_uncertainty_reduction_ppm,
                policy.effort_used,
                policy.exposure_used.sound_units,
                policy.exposure_used.light_units,
                policy.exposure_used.sediment_units,
                policy.oracle_regret_units,
                policy.selected_candidate_ids.join("|")
            );
        }
    }
    if receipt.result == "PASS" {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::FAILURE)
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("ocean-observation-allocation-bench: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_corpus() -> (Corpus, Vec<u8>) {
        let bytes =
            include_bytes!("../../../docs/tech/OCEAN_OBSERVATION_ALLOCATION_CORPUS_V0_1.json")
                .to_vec();
        let corpus = corpus_from_bytes(&bytes).expect("frozen synthetic corpus must parse");
        (corpus, bytes)
    }

    #[test]
    fn frozen_synthetic_corpus_obeys_schema() {
        let (corpus, _) = fixture_corpus();
        assert_eq!(corpus.scenarios.len(), SCENARIO_IDS.len());
        assert_eq!(corpus.evidence_class, "SyntheticAdversarial");
    }

    #[test]
    fn same_exact_corpus_seed_and_commit_produce_identical_receipts() {
        let (corpus, bytes) = fixture_corpus();
        let first = serde_json::to_vec(&build_receipt(&corpus, &bytes, "a".repeat(40))).unwrap();
        let second = serde_json::to_vec(&build_receipt(&corpus, &bytes, "a".repeat(40))).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn effort_aware_policy_beats_raw_record_count_on_frozen_access_bias_case() {
        let (corpus, _) = fixture_corpus();
        let report = evaluate_scenario(&corpus.scenarios[0], corpus.seed);
        let aware = report
            .policy_reports
            .iter()
            .find(|item| item.policy == Policy::EffortAwareStratified)
            .unwrap();
        let raw = report
            .policy_reports
            .iter()
            .find(|item| item.policy == Policy::RawRecordCountBaseline)
            .unwrap();
        assert!(aware.weighted_coverage_ppm > raw.weighted_coverage_ppm);
    }

    #[test]
    fn contact_unreviewed_unknown_detection_and_unknown_exposure_are_ineligible() {
        let (corpus, _) = fixture_corpus();
        for scenario in &corpus.scenarios {
            let reports = evaluate_scenario(scenario, corpus.seed);
            for report in reports.policy_reports {
                for selected in report.selected_candidate_ids {
                    let candidate = scenario
                        .candidates
                        .iter()
                        .find(|item| item.candidate_id == selected)
                        .unwrap();
                    assert!(
                        is_eligible(candidate),
                        "ineligible candidate selected: {}",
                        candidate.candidate_id
                    );
                }
            }
        }
    }

    #[test]
    fn detection_profile_must_be_complete_and_signal_must_exceed_sensor_limit() {
        let (corpus, _) = fixture_corpus();
        let base = corpus.scenarios[0]
            .candidates
            .iter()
            .find(|candidate| candidate.candidate_id == "abyssal-passive-01")
            .expect("fixture contains calibrated positive control")
            .clone();
        assert!(is_eligible(&base));

        let mut missing = base.clone();
        missing.detection_profile = None;
        assert!(!is_eligible(&missing));

        let mut incomplete = base.clone();
        incomplete
            .detection_profile
            .as_mut()
            .unwrap()
            .expected_signal_ppm = None;
        assert!(!is_eligible(&incomplete));

        let mut below_sensor_limit = base.clone();
        let profile = below_sensor_limit.detection_profile.as_mut().unwrap();
        profile.minimum_detectable_signal_ppm = Some(300_000);
        profile.expected_signal_ppm = Some(200_000);
        assert!(!is_eligible(&below_sensor_limit));

        let mut zero_detection_probability = base.clone();
        zero_detection_probability
            .detection_profile
            .as_mut()
            .unwrap()
            .detection_probability_ppm = Some(0);
        assert!(!is_eligible(&zero_detection_probability));

        let underpowered_fixture = corpus.scenarios[0]
            .candidates
            .iter()
            .find(|candidate| candidate.candidate_id == "abyssal-low-signal-under-sensitivity")
            .expect("fixture contains inadequate-sensitivity negative control");
        assert!(!is_eligible(underpowered_fixture));

        let mut bad_range = corpus;
        bad_range.scenarios[0].candidates[0]
            .detection_profile
            .as_mut()
            .unwrap()
            .detection_probability_ppm = Some(SCALE_PPM + 1);
        assert!(validate_corpus(&bad_range).is_err());

        let mut calibrated = base;
        calibrated.information_gain_ppm = 200_000;
        calibrated
            .detection_profile
            .as_mut()
            .unwrap()
            .detection_probability_ppm = Some(500_000);
        assert_eq!(expected_information_gain_ppm(&calibrated), 100_000);
    }

    #[test]
    fn rights_review_completion_and_permission_are_independent_gates() {
        let (corpus, _) = fixture_corpus();
        let permitted_but_unreviewed = Candidate {
            candidate_id: "review-pending-permission-claim".into(),
            stratum_id: "synthetic-stratum".into(),
            observation_mode: ObservationMode::PassiveNonContact,
            rights_review_status: RightsReviewStatus::ReviewRequired,
            use_permission: UsePermission::Permitted,
            detection_profile: Some(DetectionProfile {
                minimum_detectable_signal_ppm: Some(100_000),
                expected_signal_ppm: Some(500_000),
                detection_probability_ppm: Some(900_000),
            }),
            exposure_knowledge: ExposureKnowledge::Known,
            effort_units: 1,
            information_gain_ppm: 100,
            exposure: Exposure {
                sound_units: 0,
                light_units: 0,
                sediment_units: 0,
            },
        };
        assert!(!is_eligible(&permitted_but_unreviewed));

        let (mut restricted_after_review, _) = fixture_corpus();
        let candidate = restricted_after_review
            .scenarios
            .iter_mut()
            .flat_map(|scenario| scenario.candidates.iter_mut())
            .find(|candidate| candidate.candidate_id == "benthic-terms-restricted")
            .expect("frozen corpus has restricted-use negative control");
        assert_eq!(candidate.rights_review_status, RightsReviewStatus::Reviewed);
        assert_eq!(candidate.use_permission, UsePermission::Restricted);
        assert!(!is_eligible(candidate));

        assert_eq!(corpus.evidence_class, "SyntheticAdversarial");
    }

    #[test]
    fn independent_exposure_channels_remain_within_distinct_caps() {
        let (corpus, _) = fixture_corpus();
        for scenario in &corpus.scenarios {
            for report in evaluate_scenario(scenario, corpus.seed).policy_reports {
                assert!(report.effort_used <= scenario.budget.effort_units);
                assert!(report.exposure_used.sound_units <= scenario.budget.sound_units);
                assert!(report.exposure_used.light_units <= scenario.budget.light_units);
                assert!(report.exposure_used.sediment_units <= scenario.budget.sediment_units);
            }
        }
    }

    #[test]
    fn exact_small_case_oracle_has_zero_regret_and_bounded_exhaustive_search() {
        let (corpus, _) = fixture_corpus();
        for scenario in &corpus.scenarios {
            let report = evaluate_scenario(scenario, corpus.seed);
            let oracle = report
                .policy_reports
                .iter()
                .find(|item| item.policy == Policy::ExactSmallCaseOracle)
                .unwrap();
            assert_eq!(oracle.oracle_regret_units, 0);
            assert!(scenario.candidates.len() <= MAX_CANDIDATES_PER_SCENARIO);
            assert!(report
                .policy_reports
                .iter()
                .all(|item| item.information_score_units <= oracle.information_score_units));
        }
    }

    #[test]
    fn pareto_frontier_preserves_non_scalar_tradeoffs_and_is_not_dominated() {
        let (corpus, _) = fixture_corpus();
        let scenario = &corpus.scenarios[0];
        let report = evaluate_scenario(scenario, corpus.seed);
        assert!(
            report
                .policy_reports
                .iter()
                .filter(|item| item.pareto_nondominated)
                .count()
                >= 2,
            "different evidence/exposure trade-offs should not be collapsed into one winner"
        );
        assert!(
            report
                .policy_reports
                .iter()
                .find(|item| item.policy == Policy::RawRecordCountBaseline)
                .unwrap()
                .pareto_nondominated,
            "the low-exposure, low-coverage raw-count baseline remains a distinct trade-off"
        );
        for (index, candidate) in report.policy_reports.iter().enumerate() {
            let dominated = report
                .policy_reports
                .iter()
                .enumerate()
                .any(|(other_index, other)| other_index != index && dominates(other, candidate));
            assert_eq!(candidate.pareto_nondominated, !dominated);
        }
    }

    #[test]
    fn zero_records_are_never_exposed_as_species_absence() {
        let (corpus, _) = fixture_corpus();
        assert!(corpus
            .claim_ceiling
            .contains("No species abundance, absence"));
        let serialized = serde_json::to_value(&corpus).unwrap();
        let top = serialized.as_object().unwrap();
        assert!(!top.contains_key("species_absence"));
        assert!(!top.contains_key("ecological_safety"));
    }

    #[test]
    fn unsupported_schema_and_unknown_fields_fail_closed() {
        let (_, bytes) = fixture_corpus();
        let text = String::from_utf8(bytes).unwrap();
        let bad_schema = text.replace(SCHEMA_VERSION, "999.0.0");
        assert!(corpus_from_bytes(bad_schema.as_bytes()).is_err());
        let bad_field = text.replacen("\"seed\":", "\"species_absence\":false,\n  \"seed\":", 1);
        assert!(corpus_from_bytes(bad_field.as_bytes()).is_err());
    }
}
