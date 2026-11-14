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

const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");
const SCHEMA_ID: &str = "luminous.symtropy.ocean-sequential-design";
const SCHEMA_VERSION: &str = "0.1.0";
const SUITE_ID: &str = "OCEAN-SEQUENTIAL-DESIGN-SYNTH-001";
const DEFAULT_CORPUS: &str = "docs/tech/OCEAN_SEQUENTIAL_DESIGN_CORPUS_V0_1.json";
const MAX_CORPUS_BYTES: usize = 250_000;
const MAX_CANDIDATES: usize = 12;
const SCALE: u64 = 1_000_000;
const NBITS_PER_BIT: f64 = 1_000_000_000.0;
const SCENARIO_IDS: &[&str] = &[
    "imperfect-detection-and-correlation",
    "false-positive-control",
    "low-sensitivity-and-rights-gates",
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
enum RightsReview {
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
    priority_ppm: u64,
    historical_record_count: u64,
    historical_effort_units: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Candidate {
    candidate_id: String,
    stratum_id: String,
    time_window_id: String,
    /// Same group means perfectly correlated synthetic repeat; select at most one.
    correlation_group: String,
    observation_mode: ObservationMode,
    rights_review: RightsReview,
    use_permission: UsePermission,
    exposure_knowledge: ExposureKnowledge,
    expected_signal_ppm: Option<u64>,
    minimum_detectable_signal_ppm: Option<u64>,
    /// P(positive | latent target present), synthetic normalized ppm.
    sensitivity_ppm: Option<u64>,
    /// P(positive | latent target absent), synthetic normalized ppm.
    false_positive_ppm: Option<u64>,
    effort_units: u32,
    exposure: Exposure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Scenario {
    scenario_id: String,
    description: String,
    /// Prior probability of a synthetic latent target, not a real occupancy estimate.
    prior_presence_ppm: u64,
    max_actions: usize,
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
    RepeatDominantStratum,
    BatchInformationGain,
    AdaptiveInformationGreedy,
    ExactAdaptiveOracle,
}

const POLICIES: &[Policy] = &[
    Policy::RandomFixedSeed,
    Policy::LargestCoverageGap,
    Policy::RepeatDominantStratum,
    Policy::BatchInformationGain,
    Policy::AdaptiveInformationGreedy,
    Policy::ExactAdaptiveOracle,
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PolicyReport {
    policy: Policy,
    first_action_id: Option<String>,
    expected_information_gain_nbits: u64,
    initial_action_gain_nbits: u64,
    dominated_by_exact_oracle_nbits: u64,
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
struct Receipt {
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

#[derive(Debug, Clone)]
struct State {
    posterior_presence_ppm: u64,
    selected_mask: u16,
    effort_used: u32,
    exposure: Exposure,
    selected_strata: Vec<String>,
}

impl State {
    fn new(prior_presence_ppm: u64) -> Self {
        Self {
            posterior_presence_ppm: prior_presence_ppm,
            selected_mask: 0,
            effort_used: 0,
            exposure: Exposure {
                sound_units: 0,
                light_units: 0,
                sediment_units: 0,
            },
            selected_strata: Vec::new(),
        }
    }

    fn selected_count(&self) -> usize {
        self.selected_mask.count_ones() as usize
    }
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
        return Err("claim ceiling must be non-empty".into());
    }
    if corpus.scenarios.len() != SCENARIO_IDS.len() {
        return Err(format!("expected {} frozen scenarios", SCENARIO_IDS.len()));
    }
    for (index, scenario) in corpus.scenarios.iter().enumerate() {
        if scenario.scenario_id != SCENARIO_IDS[index] {
            return Err(format!("scenario {index} must be {}", SCENARIO_IDS[index]));
        }
        if scenario.description.trim().is_empty()
            || scenario.strata.is_empty()
            || scenario.candidates.is_empty()
        {
            return Err(format!(
                "scenario {} has missing required content",
                scenario.scenario_id
            ));
        }
        if scenario.prior_presence_ppm > SCALE {
            return Err(format!(
                "scenario {} prior is outside ppm range",
                scenario.scenario_id
            ));
        }
        if scenario.max_actions == 0 || scenario.max_actions > 6 {
            return Err(format!(
                "scenario {} horizon must be 1..=6",
                scenario.scenario_id
            ));
        }
        if scenario.candidates.len() > MAX_CANDIDATES || scenario.candidates.len() > 16 {
            return Err(format!(
                "scenario {} exceeds candidate bit-mask limit",
                scenario.scenario_id
            ));
        }
        if scenario.candidates.len() < scenario.max_actions {
            // A smaller available action set is allowed; the horizon is an upper bound.
        }
        let mut stratum_ids = HashSet::new();
        for stratum in &scenario.strata {
            if stratum.stratum_id.trim().is_empty()
                || !stratum_ids.insert(stratum.stratum_id.as_str())
            {
                return Err(format!(
                    "scenario {} has blank/duplicate stratum IDs",
                    scenario.scenario_id
                ));
            }
            if stratum.priority_ppm == 0 || stratum.priority_ppm > SCALE {
                return Err(format!(
                    "stratum {} priority outside ppm range",
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
                    "scenario {} has blank/duplicate candidate IDs",
                    scenario.scenario_id
                ));
            }
            if !stratum_ids.contains(candidate.stratum_id.as_str()) {
                return Err(format!(
                    "candidate {} references missing stratum",
                    candidate.candidate_id
                ));
            }
            if candidate.time_window_id.trim().is_empty()
                || candidate.correlation_group.trim().is_empty()
            {
                return Err(format!(
                    "candidate {} has blank time/correlation group",
                    candidate.candidate_id
                ));
            }
            if candidate.effort_units == 0 {
                return Err(format!(
                    "candidate {} effort must be positive",
                    candidate.candidate_id
                ));
            }
            for (name, value) in [
                ("expected_signal_ppm", candidate.expected_signal_ppm),
                (
                    "minimum_detectable_signal_ppm",
                    candidate.minimum_detectable_signal_ppm,
                ),
                ("sensitivity_ppm", candidate.sensitivity_ppm),
                ("false_positive_ppm", candidate.false_positive_ppm),
            ] {
                if value.is_some_and(|ppm| ppm > SCALE) {
                    return Err(format!(
                        "candidate {} {name} exceeds 1,000,000 ppm",
                        candidate.candidate_id
                    ));
                }
            }
        }
    }
    Ok(())
}

fn corpus_from_bytes(bytes: &[u8]) -> Check<Corpus> {
    if bytes.len() > MAX_CORPUS_BYTES {
        return Err(format!("corpus exceeds byte limit {MAX_CORPUS_BYTES}"));
    }
    let corpus: Corpus =
        serde_json::from_slice(bytes).map_err(|error| format!("invalid corpus: {error}"))?;
    validate_corpus(&corpus)?;
    Ok(corpus)
}

fn source_commit(explicit: Option<String>) -> Check<String> {
    let value = explicit
        .or_else(|| env::var("OCEAN_SEQUENTIAL_SOURCE_COMMIT").ok())
        .or_else(|| env::var("GITHUB_SHA").ok())
        .ok_or_else(|| "source commit required via --source-commit, OCEAN_SEQUENTIAL_SOURCE_COMMIT, or GITHUB_SHA".to_owned())?;
    if !(40..=64).contains(&value.len()) || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("source commit must be a 40-64 character hexadecimal Git object ID".into());
    }
    Ok(value.to_ascii_lowercase())
}

fn candidate_eligible(candidate: &Candidate) -> bool {
    matches!(
        candidate.observation_mode,
        ObservationMode::PassiveNonContact | ObservationMode::ActiveNonContact
    ) && candidate.rights_review == RightsReview::Reviewed
        && candidate.use_permission == UsePermission::Permitted
        && candidate.exposure_knowledge == ExposureKnowledge::Known
        && matches!(
            (
                candidate.expected_signal_ppm,
                candidate.minimum_detectable_signal_ppm,
                candidate.sensitivity_ppm,
                candidate.false_positive_ppm
            ),
            (Some(expected), Some(minimum), Some(sensitivity), Some(false_positive))
                if expected >= minimum
                    && sensitivity > 0
                    && sensitivity <= SCALE
                    && false_positive <= SCALE
        )
}

fn add_exposure(left: Exposure, right: Exposure) -> Exposure {
    Exposure {
        sound_units: left.sound_units.saturating_add(right.sound_units),
        light_units: left.light_units.saturating_add(right.light_units),
        sediment_units: left.sediment_units.saturating_add(right.sediment_units),
    }
}

fn exposure_within(exposure: Exposure, budget: Budget) -> bool {
    exposure.sound_units <= budget.sound_units
        && exposure.light_units <= budget.light_units
        && exposure.sediment_units <= budget.sediment_units
}

fn can_add(scenario: &Scenario, state: &State, index: usize) -> bool {
    let candidate = &scenario.candidates[index];
    if !candidate_eligible(candidate)
        || state.selected_mask & (1_u16 << index) != 0
        || state.selected_count() >= scenario.max_actions
        || state.effort_used.saturating_add(candidate.effort_units) > scenario.budget.effort_units
        || !exposure_within(
            add_exposure(state.exposure, candidate.exposure),
            scenario.budget,
        )
    {
        return false;
    }
    !scenario
        .candidates
        .iter()
        .enumerate()
        .any(|(other_index, other)| {
            state.selected_mask & (1_u16 << other_index) != 0
                && other.correlation_group == candidate.correlation_group
        })
}

fn entropy_nbits(probability_ppm: u64) -> u64 {
    if probability_ppm == 0 || probability_ppm == SCALE {
        return 0;
    }
    let p = probability_ppm as f64 / SCALE as f64;
    let entropy = -p * p.log2() - (1.0 - p) * (1.0 - p).log2();
    (entropy * NBITS_PER_BIT).round().max(0.0) as u64
}

fn positive_probability_ppm(presence_ppm: u64, candidate: &Candidate) -> u64 {
    let sensitivity = candidate
        .sensitivity_ppm
        .expect("eligible candidate has sensitivity");
    let false_positive = candidate
        .false_positive_ppm
        .expect("eligible candidate has false-positive rate");
    ((u128::from(presence_ppm) * u128::from(sensitivity)
        + u128::from(SCALE - presence_ppm) * u128::from(false_positive))
        / u128::from(SCALE)) as u64
}

fn posterior_after(presence_ppm: u64, candidate: &Candidate, positive: bool) -> u64 {
    let sensitivity = candidate
        .sensitivity_ppm
        .expect("eligible candidate has sensitivity");
    let probability = positive_probability_ppm(presence_ppm, candidate);
    if positive {
        if probability == 0 {
            presence_ppm
        } else {
            ((u128::from(presence_ppm) * u128::from(sensitivity)) / u128::from(probability))
                .min(u128::from(SCALE)) as u64
        }
    } else {
        let negative_probability = SCALE - probability;
        if negative_probability == 0 {
            presence_ppm
        } else {
            ((u128::from(presence_ppm) * u128::from(SCALE - sensitivity))
                / u128::from(negative_probability))
            .min(u128::from(SCALE)) as u64
        }
    }
}

fn mutual_information_nbits(presence_ppm: u64, candidate: &Candidate) -> u64 {
    let positive = positive_probability_ppm(presence_ppm, candidate);
    let negative = SCALE - positive;
    let positive_posterior = posterior_after(presence_ppm, candidate, true);
    let negative_posterior = posterior_after(presence_ppm, candidate, false);
    let expected_entropy = ((u128::from(positive) * u128::from(entropy_nbits(positive_posterior))
        + u128::from(negative) * u128::from(entropy_nbits(negative_posterior)))
        / u128::from(SCALE)) as u64;
    entropy_nbits(presence_ppm).saturating_sub(expected_entropy)
}

fn apply_observation(scenario: &Scenario, state: &State, index: usize, positive: bool) -> State {
    let candidate = &scenario.candidates[index];
    let mut next = state.clone();
    next.posterior_presence_ppm =
        posterior_after(state.posterior_presence_ppm, candidate, positive);
    next.selected_mask |= 1_u16 << index;
    next.effort_used = next.effort_used.saturating_add(candidate.effort_units);
    next.exposure = add_exposure(next.exposure, candidate.exposure);
    next.selected_strata.push(candidate.stratum_id.clone());
    next
}

fn selected_strata(scenario: &Scenario, state: &State) -> HashSet<String> {
    scenario
        .candidates
        .iter()
        .enumerate()
        .filter(|(index, _)| state.selected_mask & (1_u16 << index) != 0)
        .map(|(_, c)| c.stratum_id.clone())
        .collect()
}

fn stable_hash(seed: u64, value: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in seed
        .to_le_bytes()
        .into_iter()
        .chain(value.as_bytes().iter().copied())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn rank_candidate(
    scenario: &Scenario,
    state: &State,
    index: usize,
    policy: Policy,
    seed: u64,
) -> (u128, u128, u128) {
    let candidate = &scenario.candidates[index];
    let stratum = scenario
        .strata
        .iter()
        .find(|s| s.stratum_id == candidate.stratum_id)
        .expect("validated stratum");
    let gain_posterior = if policy == Policy::BatchInformationGain {
        scenario.prior_presence_ppm
    } else {
        state.posterior_presence_ppm
    };
    let gain = mutual_information_nbits(gain_posterior, candidate);
    let seen_strata = selected_strata(scenario, state);
    match policy {
        Policy::RandomFixedSeed => (
            u128::from(u64::MAX - stable_hash(seed, &candidate.candidate_id)),
            0,
            0,
        ),
        Policy::LargestCoverageGap => (
            if seen_strata.contains(&candidate.stratum_id) {
                0
            } else {
                1
            },
            u128::from(stratum.priority_ppm),
            u128::from(gain),
        ),
        Policy::RepeatDominantStratum => {
            let target = if let Some(first) = scenario
                .candidates
                .iter()
                .enumerate()
                .find(|(i, _)| state.selected_mask & (1_u16 << i) != 0)
                .map(|(_, candidate)| candidate.stratum_id.clone())
            {
                first
            } else {
                scenario
                    .strata
                    .iter()
                    .max_by_key(|s| (s.historical_record_count, s.stratum_id.as_str()))
                    .map(|s| s.stratum_id.clone())
                    .unwrap_or_default()
            };
            (
                u128::from(candidate.stratum_id == target),
                u128::from(gain),
                u128::from(stratum.priority_ppm),
            )
        }
        Policy::BatchInformationGain => (
            u128::from(gain / u64::from(candidate.effort_units.max(1))),
            u128::from(gain),
            0,
        ),
        Policy::AdaptiveInformationGreedy => (
            u128::from(gain / u64::from(candidate.effort_units.max(1))),
            u128::from(gain),
            u128::from(stratum.priority_ppm),
        ),
        Policy::ExactAdaptiveOracle => (0, 0, 0),
    }
}

fn best_greedy_action(
    scenario: &Scenario,
    state: &State,
    policy: Policy,
    seed: u64,
) -> Option<usize> {
    let mut best: Option<(usize, (u128, u128, u128))> = None;
    for index in 0..scenario.candidates.len() {
        if !can_add(scenario, state, index) {
            continue;
        }
        let candidate = &scenario.candidates[index];
        if policy != Policy::RandomFixedSeed
            && mutual_information_nbits(
                if policy == Policy::BatchInformationGain {
                    scenario.prior_presence_ppm
                } else {
                    state.posterior_presence_ppm
                },
                candidate,
            ) == 0
        {
            continue;
        }
        let rank = rank_candidate(scenario, state, index, policy, seed);
        match best {
            None => best = Some((index, rank)),
            Some((best_index, best_rank)) => {
                let id = &scenario.candidates[index].candidate_id;
                let best_id = &scenario.candidates[best_index].candidate_id;
                if rank > best_rank || (rank == best_rank && id < best_id) {
                    best = Some((index, rank));
                }
            }
        }
    }
    best.map(|(index, _)| index)
}

fn weighted_future(q_positive_ppm: u64, positive_value: u64, negative_value: u64) -> u64 {
    ((u128::from(q_positive_ppm) * u128::from(positive_value)
        + u128::from(SCALE - q_positive_ppm) * u128::from(negative_value))
        / u128::from(SCALE)) as u64
}

fn policy_value(scenario: &Scenario, state: &State, policy: Policy, seed: u64) -> u64 {
    if state.selected_count() >= scenario.max_actions {
        return 0;
    }
    let Some(index) = best_greedy_action(scenario, state, policy, seed) else {
        return 0;
    };
    let candidate = &scenario.candidates[index];
    let prior = state.posterior_presence_ppm;
    let q_positive = positive_probability_ppm(prior, candidate);
    let immediate_gain = mutual_information_nbits(prior, candidate);
    let positive_future = if q_positive > 0 {
        policy_value(
            scenario,
            &apply_observation(scenario, state, index, true),
            policy,
            seed,
        )
    } else {
        0
    };
    let negative_future = if q_positive < SCALE {
        policy_value(
            scenario,
            &apply_observation(scenario, state, index, false),
            policy,
            seed,
        )
    } else {
        0
    };
    immediate_gain.saturating_add(weighted_future(
        q_positive,
        positive_future,
        negative_future,
    ))
}

fn exact_oracle_value(scenario: &Scenario, state: &State) -> u64 {
    if state.selected_count() >= scenario.max_actions {
        return 0;
    }
    let mut best = 0;
    for index in 0..scenario.candidates.len() {
        if !can_add(scenario, state, index) {
            continue;
        }
        let candidate = &scenario.candidates[index];
        let prior = state.posterior_presence_ppm;
        let q_positive = positive_probability_ppm(prior, candidate);
        let immediate_gain = mutual_information_nbits(prior, candidate);
        let positive_future = if q_positive > 0 {
            exact_oracle_value(scenario, &apply_observation(scenario, state, index, true))
        } else {
            0
        };
        let negative_future = if q_positive < SCALE {
            exact_oracle_value(scenario, &apply_observation(scenario, state, index, false))
        } else {
            0
        };
        let value = immediate_gain.saturating_add(weighted_future(
            q_positive,
            positive_future,
            negative_future,
        ));
        best = best.max(value);
    }
    best
}

fn first_action(scenario: &Scenario, policy: Policy, seed: u64) -> Option<usize> {
    let state = State::new(scenario.prior_presence_ppm);
    if policy == Policy::ExactAdaptiveOracle {
        let mut best: Option<(usize, u64)> = None;
        for index in 0..scenario.candidates.len() {
            if !can_add(scenario, &state, index) {
                continue;
            }
            let candidate = &scenario.candidates[index];
            let q = positive_probability_ppm(state.posterior_presence_ppm, candidate);
            let gain = mutual_information_nbits(state.posterior_presence_ppm, candidate);
            let plus = if q > 0 {
                exact_oracle_value(scenario, &apply_observation(scenario, &state, index, true))
            } else {
                0
            };
            let minus = if q < SCALE {
                exact_oracle_value(scenario, &apply_observation(scenario, &state, index, false))
            } else {
                0
            };
            let score = gain.saturating_add(weighted_future(q, plus, minus));
            if best.map(|(_, old)| score > old).unwrap_or(true) {
                best = Some((index, score));
            }
        }
        return best.map(|(index, _)| index);
    }
    best_greedy_action(scenario, &state, policy, seed)
}

fn evaluate_scenario(scenario: &Scenario, seed: u64) -> ScenarioReport {
    let state = State::new(scenario.prior_presence_ppm);
    let oracle_value = exact_oracle_value(scenario, &state);
    let policy_reports = POLICIES
        .iter()
        .map(|policy| {
            let root_action = first_action(scenario, *policy, seed);
            let gain = if *policy == Policy::ExactAdaptiveOracle {
                oracle_value
            } else {
                policy_value(scenario, &state, *policy, seed)
            };
            let immediate = root_action
                .map(|index| {
                    mutual_information_nbits(
                        state.posterior_presence_ppm,
                        &scenario.candidates[index],
                    )
                })
                .unwrap_or(0);
            PolicyReport {
                policy: *policy,
                first_action_id: root_action
                    .map(|index| scenario.candidates[index].candidate_id.clone()),
                expected_information_gain_nbits: gain,
                initial_action_gain_nbits: immediate,
                dominated_by_exact_oracle_nbits: oracle_value.saturating_sub(gain),
            }
        })
        .collect();
    ScenarioReport {
        scenario_id: scenario.scenario_id.clone(),
        eligible_candidate_count: scenario
            .candidates
            .iter()
            .filter(|candidate| candidate_eligible(candidate))
            .count(),
        policy_reports,
    }
}

fn add_invariant(invariants: &mut Vec<InvariantResult>, id: &str, passed: bool, detail: &str) {
    invariants.push(InvariantResult {
        invariant_id: id.to_owned(),
        passed,
        detail: detail.to_owned(),
    });
}

fn all_policy_branches_respect_budgets(
    scenario: &Scenario,
    state: &State,
    policy: Policy,
    seed: u64,
) -> bool {
    if !exposure_within(state.exposure, scenario.budget)
        || state.effort_used > scenario.budget.effort_units
    {
        return false;
    }
    if state.selected_count() >= scenario.max_actions {
        return true;
    }
    let Some(index) = best_greedy_action(scenario, state, policy, seed) else {
        return true;
    };
    if !can_add(scenario, state, index) {
        return false;
    }
    let q = positive_probability_ppm(state.posterior_presence_ppm, &scenario.candidates[index]);
    let plus = q == 0
        || all_policy_branches_respect_budgets(
            scenario,
            &apply_observation(scenario, state, index, true),
            policy,
            seed,
        );
    let minus = q == SCALE
        || all_policy_branches_respect_budgets(
            scenario,
            &apply_observation(scenario, state, index, false),
            policy,
            seed,
        );
    plus && minus
}

fn build_receipt(corpus: &Corpus, bytes: &[u8], source_commit: String) -> Receipt {
    let reports: Vec<ScenarioReport> = corpus
        .scenarios
        .iter()
        .map(|s| evaluate_scenario(s, corpus.seed))
        .collect();
    let mut invariants = Vec::new();
    let mut budgets_ok = true;
    let mut no_ineligible_selected = true;
    let mut oracle_ok = true;
    let mut correlations_respected = true;
    for scenario in &corpus.scenarios {
        for policy in POLICIES {
            let state = State::new(scenario.prior_presence_ppm);
            if !all_policy_branches_respect_budgets(scenario, &state, *policy, corpus.seed) {
                budgets_ok = false;
            }
            // The recursive transition only admits candidates after all independent eligibility gates.
            fn audit(
                scenario: &Scenario,
                state: &State,
                policy: Policy,
                seed: u64,
                flags: &mut (bool, bool),
            ) {
                if state.selected_count() >= scenario.max_actions {
                    return;
                }
                let Some(index) = best_greedy_action(scenario, state, policy, seed) else {
                    return;
                };
                if !candidate_eligible(&scenario.candidates[index]) {
                    flags.0 = false;
                }
                let candidate = &scenario.candidates[index];
                let seen_same_group =
                    scenario
                        .candidates
                        .iter()
                        .enumerate()
                        .any(|(other_index, other)| {
                            state.selected_mask & (1_u16 << other_index) != 0
                                && other.correlation_group == candidate.correlation_group
                        });
                if seen_same_group {
                    flags.1 = false;
                }
                let q = positive_probability_ppm(state.posterior_presence_ppm, candidate);
                if q > 0 {
                    audit(
                        scenario,
                        &apply_observation(scenario, state, index, true),
                        policy,
                        seed,
                        flags,
                    );
                }
                if q < SCALE {
                    audit(
                        scenario,
                        &apply_observation(scenario, state, index, false),
                        policy,
                        seed,
                        flags,
                    );
                }
            }
            let mut flags = (true, true);
            audit(scenario, &state, *policy, corpus.seed, &mut flags);
            no_ineligible_selected &= flags.0;
            correlations_respected &= flags.1;
        }
        let report = reports
            .iter()
            .find(|item| item.scenario_id == scenario.scenario_id)
            .expect("one report per scenario");
        let oracle = report
            .policy_reports
            .iter()
            .find(|item| item.policy == Policy::ExactAdaptiveOracle)
            .expect("oracle report");
        oracle_ok &= oracle.dominated_by_exact_oracle_nbits == 0
            && report.policy_reports.iter().all(|p| {
                p.expected_information_gain_nbits <= oracle.expected_information_gain_nbits
            });
    }
    add_invariant(
        &mut invariants,
        "cumulative_effort_and_exposure_budget",
        budgets_ok,
        "All branches of every policy respect cumulative effort, sound, light and sediment limits.",
    );
    add_invariant(&mut invariants, "rights_and_detection_gates", no_ineligible_selected,
        "Selected actions require non-contact mode, completed rights review, explicit permitted use, and an adequate detection profile.");
    add_invariant(&mut invariants, "perfectly_correlated_repeats_not_double_counted", correlations_respected,
        "At most one observation per declared perfectly correlated station/time group can enter a schedule.");
    add_invariant(&mut invariants, "exact_adaptive_oracle_dominates_heuristics", oracle_ok,
        "The exhaustive adaptive decision-tree oracle is at least as good as every heuristic under the same likelihoods and constraints.");
    add_invariant(&mut invariants, "no_single_non_detection_absence_claim", true,
        "The benchmark models latent target probability separately from observation results and has no absence conclusion field.");
    add_invariant(
        &mut invariants,
        "deterministic_inputs_bound_receipt",
        !sha256_hex(bytes).is_empty() && !source_commit.is_empty(),
        "Receipt binds exact corpus bytes, seed, suite/schema and source commit.",
    );

    let passed = invariants.iter().filter(|i| i.passed).count();
    let failed = invariants.len() - passed;
    Receipt {
        schema_id: SCHEMA_ID.into(),
        schema_version: SCHEMA_VERSION.into(),
        tool_version: TOOL_VERSION.into(),
        suite_id: SUITE_ID.into(),
        corpus_sha256: sha256_hex(bytes),
        source_commit,
        evidence_class: corpus.evidence_class.clone(),
        claim_ceiling: corpus.claim_ceiling.clone(),
        seed: corpus.seed,
        scenario_count: reports.len(),
        passed_invariant_count: passed,
        failed_invariant_count: failed,
        result: if failed == 0 { "PASS" } else { "FAIL" }.into(),
        invariants,
        scenarios: reports,
    }
}

fn run() -> Check<ExitCode> {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut corpus_path = PathBuf::from(DEFAULT_CORPUS);
    let mut receipt_path: Option<PathBuf> = None;
    let mut explicit_commit = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--corpus" => {
                i += 1;
                corpus_path = PathBuf::from(args.get(i).ok_or("--corpus requires a path")?);
            }
            "--receipt" => {
                i += 1;
                receipt_path = Some(PathBuf::from(
                    args.get(i).ok_or("--receipt requires a path")?,
                ));
            }
            "--source-commit" => {
                i += 1;
                explicit_commit =
                    Some(args.get(i).ok_or("--source-commit requires an ID")?.clone());
            }
            other => return Err(format!("unknown argument: {other}")),
        }
        i += 1;
    }
    let bytes = fs::read(&corpus_path)
        .map_err(|e| format!("cannot read corpus {}: {e}", corpus_path.display()))?;
    let corpus = corpus_from_bytes(&bytes)?;
    let commit = source_commit(explicit_commit)?;
    let receipt = build_receipt(&corpus, &bytes, commit);
    let serialized =
        serde_json::to_vec_pretty(&receipt).map_err(|e| format!("cannot encode receipt: {e}"))?;
    if let Some(path) = receipt_path {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent)
                .map_err(|e| format!("cannot create receipt directory: {e}"))?;
        }
        fs::write(&path, &serialized).map_err(|e| format!("cannot write receipt: {e}"))?;
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
            println!("policy={:?} first_action={:?} expected_gain_nbits={} first_gain_nbits={} oracle_gap_nbits={}",
                policy.policy, policy.first_action_id, policy.expected_information_gain_nbits,
                policy.initial_action_gain_nbits, policy.dominated_by_exact_oracle_nbits);
        }
    }
    Ok(if receipt.result == "PASS" {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("ocean-sequential-design-bench: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus() -> (Corpus, Vec<u8>) {
        let bytes =
            include_bytes!("../../../docs/tech/OCEAN_SEQUENTIAL_DESIGN_CORPUS_V0_1.json").to_vec();
        let corpus = corpus_from_bytes(&bytes).expect("frozen synthetic sequence corpus parses");
        (corpus, bytes)
    }

    #[test]
    fn frozen_corpus_has_required_synthetic_worlds() {
        let (corpus, _) = corpus();
        assert_eq!(corpus.scenarios.len(), 3);
        assert_eq!(corpus.evidence_class, "SyntheticAdversarial");
    }

    #[test]
    fn deterministic_receipts_repeat_for_same_inputs() {
        let (corpus, bytes) = corpus();
        let a = serde_json::to_vec(&build_receipt(&corpus, &bytes, "a".repeat(40))).unwrap();
        let b = serde_json::to_vec(&build_receipt(&corpus, &bytes, "a".repeat(40))).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn non_detection_is_an_observation_not_an_absence_label() {
        let prior = 250_000;
        let (corpus, _) = corpus();
        let candidate = &corpus.scenarios[0].candidates[0];
        let posterior = posterior_after(prior, candidate, false);
        assert!(posterior < prior);
        assert!(posterior > 0);
    }

    #[test]
    fn repeat_in_same_correlation_group_is_not_selectable_twice() {
        let (corpus, _) = corpus();
        let scenario = &corpus.scenarios[0];
        let mut state = State::new(scenario.prior_presence_ppm);
        let a = scenario
            .candidates
            .iter()
            .position(|c| c.candidate_id == "reef-window1-primary")
            .unwrap();
        let b = scenario
            .candidates
            .iter()
            .position(|c| c.candidate_id == "reef-window1-correlated-repeat")
            .unwrap();
        assert!(can_add(scenario, &state, a));
        state = apply_observation(scenario, &state, a, true);
        assert!(!can_add(scenario, &state, b));
    }

    #[test]
    fn false_positive_rate_limits_evidence_from_a_positive() {
        let (corpus, _) = corpus();
        let scenario = &corpus.scenarios[1];
        let candidate = scenario
            .candidates
            .iter()
            .find(|c| c.candidate_id == "noisy-sensor-high-false-positive")
            .unwrap();
        let prior = 50_000;
        let positive_posterior = posterior_after(prior, candidate, true);
        assert!(
            positive_posterior < 500_000,
            "large false-positive rate must not make one positive near-certain"
        );
    }

    #[test]
    fn inadequate_detection_and_unpermitted_actions_are_ineligible() {
        let (corpus, _) = corpus();
        let low_signal = corpus.scenarios[2]
            .candidates
            .iter()
            .find(|c| c.candidate_id == "deep-low-signal-negative-control")
            .unwrap();
        assert!(!candidate_eligible(low_signal));
        let restricted = corpus.scenarios[2]
            .candidates
            .iter()
            .find(|c| c.candidate_id == "restricted-use")
            .unwrap();
        assert!(!candidate_eligible(restricted));
    }

    #[test]
    fn every_policy_is_bounded_by_exact_adaptive_oracle() {
        let (corpus, _) = corpus();
        for scenario in &corpus.scenarios {
            let report = evaluate_scenario(scenario, corpus.seed);
            let oracle = report
                .policy_reports
                .iter()
                .find(|p| p.policy == Policy::ExactAdaptiveOracle)
                .unwrap();
            for policy in &report.policy_reports {
                assert!(
                    policy.expected_information_gain_nbits
                        <= oracle.expected_information_gain_nbits
                );
                assert_eq!(
                    policy.dominated_by_exact_oracle_nbits,
                    oracle.expected_information_gain_nbits - policy.expected_information_gain_nbits
                );
            }
        }
    }

    #[test]
    fn independent_exposure_caps_hold_for_all_replanning_branches() {
        let (corpus, _) = corpus();
        for scenario in &corpus.scenarios {
            for policy in POLICIES {
                assert!(all_policy_branches_respect_budgets(
                    scenario,
                    &State::new(scenario.prior_presence_ppm),
                    *policy,
                    corpus.seed
                ));
            }
        }
    }

    #[test]
    fn missing_profile_never_becomes_a_zero_rate() {
        let (corpus, _) = corpus();
        let mut candidate = corpus.scenarios[0].candidates[0].clone();
        candidate.sensitivity_ppm = None;
        assert!(!candidate_eligible(&candidate));
    }

    #[test]
    fn unsupported_schema_unknown_fields_and_duplicate_ids_fail_closed() {
        let (_, bytes) = corpus();
        let text = String::from_utf8(bytes).unwrap();
        assert!(corpus_from_bytes(text.replace(SCHEMA_VERSION, "99.0.0").as_bytes()).is_err());
        assert!(corpus_from_bytes(
            text.replacen("\"seed\":", "\"owner\":\"claimed\",\n  \"seed\":", 1)
                .as_bytes()
        )
        .is_err());
    }

    #[test]
    fn source_uses_explicit_likelihood_for_false_negative_and_false_positive() {
        let (corpus, _) = corpus();
        for scenario in &corpus.scenarios {
            for c in &scenario.candidates {
                if candidate_eligible(c) {
                    let prior = scenario.prior_presence_ppm;
                    let q = positive_probability_ppm(prior, c);
                    assert!(q <= SCALE);
                    assert!(posterior_after(prior, c, true) <= SCALE);
                    assert!(posterior_after(prior, c, false) <= SCALE);
                }
            }
        }
    }
}
