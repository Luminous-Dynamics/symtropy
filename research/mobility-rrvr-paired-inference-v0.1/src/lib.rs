// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: &str = "0.1";
pub const BENCHMARK: &str = "RRVR-PAIRED-INFERENCE";
pub const BOOTSTRAP_PROFILE: &str = "RRVR-SPLITMIX64-CLUSTER-BOOTSTRAP-V0_1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeState {
    Event,
    NoEvent,
    Unknown,
    Disputed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndpointDirection {
    MinimizeEvent,
    MaximizeSuccess,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EndpointSpec {
    pub endpoint_id: String,
    pub direction: EndpointDirection,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preregistration {
    pub control_arm_id: String,
    pub treatment_arm_id: String,
    pub control_implementation_sha256: String,
    pub treatment_implementation_sha256: String,
    pub cluster_unit_id: String,
    pub cluster_unit_profile_sha256: String,
    pub bootstrap_draws: u32,
    pub bootstrap_seed: u64,
    pub alpha: f64,
    pub endpoint_specs: Vec<EndpointSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PairEndpointOutcome {
    pub endpoint_id: String,
    pub control: OutcomeState,
    pub treatment: OutcomeState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PairRecord {
    pub pair_id: String,
    pub control_attempt_id: String,
    pub treatment_attempt_id: String,
    pub cluster_id: String,
    pub control_cluster_id: String,
    pub treatment_cluster_id: String,
    pub frozen_input_sha256: String,
    pub outcomes: Vec<PairEndpointOutcome>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InferenceManifest {
    pub schema_version: String,
    pub benchmark: String,
    pub analysis_id: String,
    pub preregistration: Preregistration,
    pub scheduled_pair_ids: Vec<String>,
    pub excluded_pair_ids: Vec<String>,
    pub pairs: Vec<PairRecord>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BootstrapInterval {
    pub lower: f64,
    pub upper: f64,
    pub draws: u32,
    pub seed: u64,
    pub alpha: f64,
    pub cluster_n: usize,
    pub profile_id: String,
    pub interpretation: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EndpointReport {
    pub endpoint_id: String,
    pub direction: EndpointDirection,
    pub pair_n: usize,
    pub cluster_n: usize,
    /// Count of the literal Event enum state. For maximize_success endpoints this is a success count.
    pub control_event_state_n: usize,
    pub treatment_event_state_n: usize,
    pub control_unknown_n: usize,
    pub treatment_unknown_n: usize,
    pub control_disputed_n: usize,
    pub treatment_disputed_n: usize,
    pub control_adverse_cluster_n: usize,
    pub treatment_adverse_cluster_n: usize,
    pub episode_weighted_control_adverse_rate: f64,
    pub episode_weighted_treatment_adverse_rate: f64,
    pub episode_weighted_risk_difference: f64,
    pub cluster_weighted_control_adverse_rate: f64,
    pub cluster_weighted_treatment_adverse_rate: f64,
    pub cluster_weighted_risk_difference: f64,
    pub control_only_adverse_pair_n: usize,
    pub treatment_only_adverse_pair_n: usize,
    pub bootstrap_interval: Option<BootstrapInterval>,
    pub mcnemar: Option<serde_json::Value>,
    pub treatment_zero_adverse_cluster_upper_bound: Option<f64>,
    pub bootstrap_interval_interpretation: String,
    pub cluster_independence_status: String,
    pub zero_event_bound_assumption: String,
    pub status: String,
    pub reason_codes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InferenceReport {
    pub schema_version: String,
    pub benchmark: String,
    pub analysis_id: String,
    pub control_arm_id: String,
    pub treatment_arm_id: String,
    pub cluster_unit_id: String,
    pub bootstrap_profile_id: String,
    pub scheduled_pair_n: usize,
    pub analyzed_pair_n: usize,
    pub excluded_pair_n: usize,
    pub cluster_n: usize,
    pub endpoint_reports: Vec<EndpointReport>,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D049BB133111EB);
        value ^ (value >> 31)
    }

    fn index(&mut self, upper: usize) -> usize {
        ((u128::from(self.next_u64()) * upper as u128) >> 64) as usize
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn adverse(state: OutcomeState, direction: EndpointDirection) -> bool {
    match direction {
        EndpointDirection::MinimizeEvent => state != OutcomeState::NoEvent,
        EndpointDirection::MaximizeSuccess => state != OutcomeState::Event,
    }
}

fn endpoint_seed(base_seed: u64, endpoint_id: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in endpoint_id.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    base_seed ^ hash
}

fn percentile(sorted: &[f64], probability: f64) -> Option<f64> {
    if sorted.is_empty() {
        return None;
    }
    let position = probability * (sorted.len() - 1) as f64;
    let lower_index = position.floor() as usize;
    let upper_index = position.ceil() as usize;
    let fraction = position - lower_index as f64;
    Some(sorted[lower_index] * (1.0 - fraction) + sorted[upper_index] * fraction)
}

fn cluster_bootstrap(
    cluster_effects: &[f64],
    draws: u32,
    seed: u64,
    alpha: f64,
) -> Option<BootstrapInterval> {
    if cluster_effects.len() < 2 || draws < 1000 {
        return None;
    }
    let mut rng = SplitMix64::new(seed);
    let cluster_n = cluster_effects.len();
    let mut samples = Vec::with_capacity(draws as usize);
    for _ in 0..draws {
        let mut total = 0.0;
        for _ in 0..cluster_n {
            total += cluster_effects[rng.index(cluster_n)];
        }
        samples.push(total / cluster_n as f64);
    }
    samples.sort_by(f64::total_cmp);
    Some(BootstrapInterval {
        lower: percentile(&samples, alpha / 2.0)?,
        upper: percentile(&samples, 1.0 - alpha / 2.0)?,
        draws,
        seed,
        alpha,
        cluster_n,
        profile_id: BOOTSTRAP_PROFILE.to_owned(),
        interpretation: "exploratory_descriptive_only".to_owned(),
    })
}

fn validate_manifest(manifest: &InferenceManifest) -> Result<(), String> {
    if manifest.schema_version != SCHEMA_VERSION {
        return Err("SCHEMA_VERSION_UNSUPPORTED".to_owned());
    }
    if manifest.benchmark != BENCHMARK {
        return Err("BENCHMARK_ID_MISMATCH".to_owned());
    }
    if manifest.analysis_id.trim().is_empty() {
        return Err("ANALYSIS_ID_MISSING".to_owned());
    }
    let prereg = &manifest.preregistration;
    if prereg.control_arm_id.trim().is_empty()
        || prereg.treatment_arm_id.trim().is_empty()
        || prereg.control_arm_id == prereg.treatment_arm_id
    {
        return Err("ARM_DECLARATION_INVALID".to_owned());
    }
    if !is_sha256(&prereg.control_implementation_sha256)
        || !is_sha256(&prereg.treatment_implementation_sha256)
        || !is_sha256(&prereg.cluster_unit_profile_sha256)
    {
        return Err("PREREGISTRATION_DIGEST_INVALID".to_owned());
    }
    if prereg.cluster_unit_id.trim().is_empty() {
        return Err("CLUSTER_UNIT_ID_MISSING".to_owned());
    }
    if !(1000..=100_000).contains(&prereg.bootstrap_draws) {
        return Err("BOOTSTRAP_DRAW_COUNT_OUT_OF_RANGE".to_owned());
    }
    if !prereg.alpha.is_finite() || prereg.alpha <= 0.0 || prereg.alpha >= 0.5 {
        return Err("ALPHA_OUT_OF_RANGE".to_owned());
    }
    if prereg.endpoint_specs.is_empty() {
        return Err("NO_PREREGISTERED_ENDPOINTS".to_owned());
    }
    let mut endpoint_ids = BTreeSet::new();
    for endpoint in &prereg.endpoint_specs {
        if endpoint.endpoint_id.trim().is_empty()
            || endpoint.description.trim().is_empty()
            || !endpoint_ids.insert(endpoint.endpoint_id.as_str())
        {
            return Err("ENDPOINT_SPEC_INVALID_OR_DUPLICATE".to_owned());
        }
    }
    if manifest.scheduled_pair_ids.is_empty()
        || manifest
            .scheduled_pair_ids
            .iter()
            .any(|pair_id| pair_id.trim().is_empty())
        || manifest.scheduled_pair_ids.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err("SCHEDULED_PAIR_IDS_NOT_STRICTLY_SORTED".to_owned());
    }
    if !manifest.excluded_pair_ids.is_empty() {
        return Err("POST_HOC_PAIR_EXCLUSION_FORBIDDEN".to_owned());
    }
    if manifest.pairs.len() != manifest.scheduled_pair_ids.len() {
        return Err("SCHEDULED_PAIR_COUNT_MISMATCH".to_owned());
    }

    let endpoint_spec_ids: BTreeSet<&str> = prereg
        .endpoint_specs
        .iter()
        .map(|endpoint| endpoint.endpoint_id.as_str())
        .collect();
    let mut pair_ids = BTreeSet::new();
    let mut attempt_ids = BTreeSet::new();
    for pair in &manifest.pairs {
        if !pair_ids.insert(pair.pair_id.as_str()) {
            return Err(format!("DUPLICATE_PAIR_ID: {}", pair.pair_id));
        }
        if pair.control_attempt_id.trim().is_empty()
            || pair.treatment_attempt_id.trim().is_empty()
            || pair.cluster_id.trim().is_empty()
            || pair.control_attempt_id == pair.treatment_attempt_id
        {
            return Err(format!("PAIR_IDENTITY_INCOMPLETE: {}", pair.pair_id));
        }
        if !attempt_ids.insert(pair.control_attempt_id.as_str())
            || !attempt_ids.insert(pair.treatment_attempt_id.as_str())
        {
            return Err(format!("DUPLICATE_ATTEMPT_ID: {}", pair.pair_id));
        }
        if pair.control_cluster_id != pair.cluster_id
            || pair.treatment_cluster_id != pair.cluster_id
        {
            return Err(format!("PAIR_CLUSTER_ID_MISMATCH: {}", pair.pair_id));
        }
        if !is_sha256(&pair.frozen_input_sha256) {
            return Err(format!("FROZEN_INPUT_DIGEST_INVALID: {}", pair.pair_id));
        }
        let observed_endpoint_ids: BTreeSet<&str> = pair
            .outcomes
            .iter()
            .map(|outcome| outcome.endpoint_id.as_str())
            .collect();
        if observed_endpoint_ids != endpoint_spec_ids
            || observed_endpoint_ids.len() != pair.outcomes.len()
        {
            return Err(format!("PAIR_ENDPOINT_SET_MISMATCH: {}", pair.pair_id));
        }
    }
    if pair_ids
        != manifest
            .scheduled_pair_ids
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>()
    {
        return Err("SCHEDULED_PAIR_LEDGER_MISMATCH".to_owned());
    }
    Ok(())
}

fn endpoint_report(
    spec: &EndpointSpec,
    manifest: &InferenceManifest,
) -> EndpointReport {
    let pair_n = manifest.pairs.len();
    let cluster_unit_n = manifest
        .pairs
        .iter()
        .map(|pair| pair.cluster_id.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    let outcome_by_pair: BTreeMap<&str, &PairEndpointOutcome> = manifest
        .pairs
        .iter()
        .flat_map(|pair| {
            pair.outcomes
                .iter()
                .filter(|outcome| outcome.endpoint_id == spec.endpoint_id)
                .map(move |outcome| (pair.pair_id.as_str(), outcome))
        })
        .collect();

    let mut control_event_state_n = 0;
    let mut treatment_event_state_n = 0;
    let mut control_unknown_n = 0;
    let mut treatment_unknown_n = 0;
    let mut control_disputed_n = 0;
    let mut treatment_disputed_n = 0;
    let mut control_only_adverse_pair_n = 0;
    let mut treatment_only_adverse_pair_n = 0;

    let mut by_cluster: BTreeMap<&str, Vec<(bool, bool)>> = BTreeMap::new();
    for pair in &manifest.pairs {
        let outcome = outcome_by_pair[pair.pair_id.as_str()];
        if outcome.control == OutcomeState::Event {
            control_event_state_n += 1;
        }
        if outcome.treatment == OutcomeState::Event {
            treatment_event_state_n += 1;
        }
        if outcome.control == OutcomeState::Unknown {
            control_unknown_n += 1;
        }
        if outcome.treatment == OutcomeState::Unknown {
            treatment_unknown_n += 1;
        }
        if outcome.control == OutcomeState::Disputed {
            control_disputed_n += 1;
        }
        if outcome.treatment == OutcomeState::Disputed {
            treatment_disputed_n += 1;
        }
        let control_adverse = adverse(outcome.control, spec.direction);
        let treatment_adverse = adverse(outcome.treatment, spec.direction);
        if control_adverse && !treatment_adverse {
            control_only_adverse_pair_n += 1;
        }
        if treatment_adverse && !control_adverse {
            treatment_only_adverse_pair_n += 1;
        }
        by_cluster
            .entry(pair.cluster_id.as_str())
            .or_default()
            .push((control_adverse, treatment_adverse));
    }

    let control_adverse_n = manifest
        .pairs
        .iter()
        .filter(|pair| adverse(outcome_by_pair[pair.pair_id.as_str()].control, spec.direction))
        .count();
    let treatment_adverse_n = manifest
        .pairs
        .iter()
        .filter(|pair| adverse(outcome_by_pair[pair.pair_id.as_str()].treatment, spec.direction))
        .count();

    let mut control_adverse_cluster_n = 0;
    let mut treatment_adverse_cluster_n = 0;
    let mut cluster_control_means = Vec::with_capacity(cluster_unit_n);
    let mut cluster_treatment_means = Vec::with_capacity(cluster_unit_n);
    let mut cluster_effects = Vec::with_capacity(cluster_unit_n);
    for outcomes in by_cluster.values() {
        let control_mean = outcomes
            .iter()
            .map(|(control, _)| u8::from(*control) as f64)
            .sum::<f64>()
            / outcomes.len() as f64;
        let treatment_mean = outcomes
            .iter()
            .map(|(_, treatment)| u8::from(*treatment) as f64)
            .sum::<f64>()
            / outcomes.len() as f64;
        control_adverse_cluster_n += if control_mean > 0.0 { 1 } else { 0 };
        treatment_adverse_cluster_n += if treatment_mean > 0.0 { 1 } else { 0 };
        cluster_control_means.push(control_mean);
        cluster_treatment_means.push(treatment_mean);
        cluster_effects.push(treatment_mean - control_mean);
    }

    let episode_weighted_control_adverse_rate = control_adverse_n as f64 / pair_n as f64;
    let episode_weighted_treatment_adverse_rate = treatment_adverse_n as f64 / pair_n as f64;
    let cluster_weighted_control_adverse_rate =
        cluster_control_means.iter().sum::<f64>() / cluster_unit_n as f64;
    let cluster_weighted_treatment_adverse_rate =
        cluster_treatment_means.iter().sum::<f64>() / cluster_unit_n as f64;
    let endpoint_seed = endpoint_seed(
        manifest.preregistration.bootstrap_seed,
        &spec.endpoint_id,
    );
    let bootstrap_interval = cluster_bootstrap(
        &cluster_effects,
        manifest.preregistration.bootstrap_draws,
        endpoint_seed,
        manifest.preregistration.alpha,
    );
    // Unique declared cluster IDs do not establish statistical independence.
    // v0.1 deliberately withholds McNemar until an independent campaign
    // attestation and its assumptions are validated upstream.
    let mcnemar = None;
    let treatment_zero_adverse_cluster_upper_bound = (treatment_adverse_cluster_n == 0)
        .then(|| 1.0 - manifest.preregistration.alpha.powf(1.0 / cluster_unit_n as f64));

    let mut reason_codes = vec![
        "CLUSTER_INDEPENDENCE_NOT_INDEPENDENTLY_VERIFIED".to_owned(),
        "MCNEMAR_WITHHELD_NO_INDEPENDENCE_ATTESTATION".to_owned(),
        "BOOTSTRAP_COVERAGE_NOT_VALIDATED_FOR_CONFIRMATORY_INFERENCE".to_owned(),
    ];
    if bootstrap_interval.is_none() {
        reason_codes.push("INSUFFICIENT_CLUSTERS_FOR_BOOTSTRAP_INTERVAL".to_owned());
    }
    if treatment_zero_adverse_cluster_upper_bound.is_some() {
        reason_codes.push(
            "ZERO_EVENT_BOUND_CONDITIONAL_ON_IID_INDEPENDENT_CLUSTERS".to_owned(),
        );
    }
    // Percentile cluster-bootstrap output remains visible as an exploratory
    // description. The current software has no simulation-validated minimum
    // cluster count or coverage guarantee, so it never labels this inferential.
    let status = "descriptive_only";

    EndpointReport {
        endpoint_id: spec.endpoint_id.clone(),
        direction: spec.direction,
        pair_n,
        cluster_n: cluster_unit_n,
        control_event_state_n,
        treatment_event_state_n,
        control_unknown_n,
        treatment_unknown_n,
        control_disputed_n,
        treatment_disputed_n,
        control_adverse_cluster_n,
        treatment_adverse_cluster_n,
        episode_weighted_control_adverse_rate,
        episode_weighted_treatment_adverse_rate,
        episode_weighted_risk_difference: episode_weighted_treatment_adverse_rate
            - episode_weighted_control_adverse_rate,
        cluster_weighted_control_adverse_rate,
        cluster_weighted_treatment_adverse_rate,
        cluster_weighted_risk_difference: cluster_weighted_treatment_adverse_rate
            - cluster_weighted_control_adverse_rate,
        control_only_adverse_pair_n,
        treatment_only_adverse_pair_n,
        bootstrap_interval,
        mcnemar,
        treatment_zero_adverse_cluster_upper_bound,
        bootstrap_interval_interpretation: "exploratory_descriptive_only".to_owned(),
        cluster_independence_status: "not_independently_verified".to_owned(),
        zero_event_bound_assumption: "conditional_on_iid_independent_clusters_not_verified".to_owned(),
        status: status.to_owned(),
        reason_codes,
    }
}

pub fn evaluate_manifest(manifest: &InferenceManifest) -> Result<InferenceReport, String> {
    validate_manifest(manifest)?;
    let cluster_n = manifest
        .pairs
        .iter()
        .map(|pair| pair.cluster_id.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    let endpoint_reports = manifest
        .preregistration
        .endpoint_specs
        .iter()
        .map(|spec| endpoint_report(spec, manifest))
        .collect();
    Ok(InferenceReport {
        schema_version: SCHEMA_VERSION.to_owned(),
        benchmark: BENCHMARK.to_owned(),
        analysis_id: manifest.analysis_id.clone(),
        control_arm_id: manifest.preregistration.control_arm_id.clone(),
        treatment_arm_id: manifest.preregistration.treatment_arm_id.clone(),
        cluster_unit_id: manifest.preregistration.cluster_unit_id.clone(),
        bootstrap_profile_id: BOOTSTRAP_PROFILE.to_owned(),
        scheduled_pair_n: manifest.scheduled_pair_ids.len(),
        analyzed_pair_n: manifest.pairs.len(),
        excluded_pair_n: manifest.excluded_pair_ids.len(),
        cluster_n,
        endpoint_reports,
        claim_ceiling: "Synthetic descriptive paired estimates only. Bootstrap intervals are exploratory; cluster independence and zero-event-bound assumptions are not independently verified. No robot campaign, causal or safety claim.".to_owned(),
    })
}

pub fn evaluate_json(input: &str) -> Result<InferenceReport, String> {
    let manifest: InferenceManifest =
        serde_json::from_str(input).map_err(|error| format!("MANIFEST_JSON_INVALID: {error}"))?;
    evaluate_manifest(&manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest_with(pairs: Vec<PairRecord>) -> InferenceManifest {
        let mut pair_ids: Vec<String> = pairs.iter().map(|pair| pair.pair_id.clone()).collect();
        pair_ids.sort();
        InferenceManifest {
            schema_version: SCHEMA_VERSION.to_owned(),
            benchmark: BENCHMARK.to_owned(),
            analysis_id: "synthetic-known-answer-analysis".to_owned(),
            preregistration: Preregistration {
                control_arm_id: "native".to_owned(),
                treatment_arm_id: "rrvr".to_owned(),
                control_implementation_sha256: "a".repeat(64),
                treatment_implementation_sha256: "b".repeat(64),
                cluster_unit_id: "trajectory_cluster".to_owned(),
                cluster_unit_profile_sha256: "c".repeat(64),
                bootstrap_draws: 2000,
                bootstrap_seed: 42,
                alpha: 0.05,
                endpoint_specs: vec![
                    EndpointSpec {
                        endpoint_id: "false_safe".to_owned(),
                        direction: EndpointDirection::MinimizeEvent,
                        description: "False-safe adjudication event or unresolved verdict".to_owned(),
                    },
                    EndpointSpec {
                        endpoint_id: "recovery_success".to_owned(),
                        direction: EndpointDirection::MaximizeSuccess,
                        description: "Task recovery successful under independent adjudication".to_owned(),
                    },
                ],
            },
            scheduled_pair_ids: pair_ids,
            excluded_pair_ids: Vec::new(),
            pairs,
        }
    }

    fn pair(
        id: &str,
        cluster: &str,
        false_safe_control: OutcomeState,
        false_safe_treatment: OutcomeState,
        success_control: OutcomeState,
        success_treatment: OutcomeState,
    ) -> PairRecord {
        PairRecord {
            pair_id: id.to_owned(),
            control_attempt_id: format!("{id}-native"),
            treatment_attempt_id: format!("{id}-rrvr"),
            cluster_id: cluster.to_owned(),
            control_cluster_id: cluster.to_owned(),
            treatment_cluster_id: cluster.to_owned(),
            frozen_input_sha256: "d".repeat(64),
            outcomes: vec![
                PairEndpointOutcome {
                    endpoint_id: "false_safe".to_owned(),
                    control: false_safe_control,
                    treatment: false_safe_treatment,
                },
                PairEndpointOutcome {
                    endpoint_id: "recovery_success".to_owned(),
                    control: success_control,
                    treatment: success_treatment,
                },
            ],
        }
    }

    #[test]
    fn known_answer_emits_separate_endpoints_without_composite_score() {
        let manifest = manifest_with(vec![
            pair("p1", "c1", OutcomeState::Event, OutcomeState::NoEvent, OutcomeState::NoEvent, OutcomeState::Event),
            pair("p2", "c2", OutcomeState::NoEvent, OutcomeState::Event, OutcomeState::Event, OutcomeState::Event),
            pair("p3", "c3", OutcomeState::NoEvent, OutcomeState::NoEvent, OutcomeState::Event, OutcomeState::NoEvent),
            pair("p4", "c4", OutcomeState::NoEvent, OutcomeState::NoEvent, OutcomeState::Event, OutcomeState::Event),
        ]);
        let report = evaluate_manifest(&manifest).expect("known answer");
        assert_eq!(report.endpoint_reports.len(), 2);
        let false_safe = &report.endpoint_reports[0];
        assert_eq!(false_safe.control_only_adverse_pair_n, 1);
        assert_eq!(false_safe.treatment_only_adverse_pair_n, 1);
        assert_eq!(false_safe.episode_weighted_risk_difference, 0.0);
        assert!(false_safe.mcnemar.is_none());
        assert_eq!(false_safe.status, "descriptive_only");
        assert_eq!(false_safe.cluster_independence_status, "not_independently_verified");
        assert_eq!(false_safe.bootstrap_interval_interpretation, "exploratory_descriptive_only");
        assert!(false_safe.reason_codes.iter().any(|code| code == "CLUSTER_INDEPENDENCE_NOT_INDEPENDENTLY_VERIFIED"));
    }

    #[test]
    fn unknown_and_disputed_count_conservatively_for_safety_but_remain_visible() {
        let manifest = manifest_with(vec![
            pair("p1", "c1", OutcomeState::NoEvent, OutcomeState::Unknown, OutcomeState::Event, OutcomeState::Unknown),
            pair("p2", "c2", OutcomeState::NoEvent, OutcomeState::Disputed, OutcomeState::Event, OutcomeState::Disputed),
        ]);
        let report = evaluate_manifest(&manifest).expect("unknown states are valid data");
        let safety = &report.endpoint_reports[0];
        assert_eq!(safety.treatment_unknown_n, 1);
        assert_eq!(safety.treatment_disputed_n, 1);
        assert_eq!(safety.treatment_adverse_cluster_n, 2);
    }

    #[test]
    fn repeated_pairs_in_same_cluster_disable_independent_pair_mcnemar() {
        let manifest = manifest_with(vec![
            pair("p1", "shared", OutcomeState::NoEvent, OutcomeState::Event, OutcomeState::Event, OutcomeState::Event),
            pair("p2", "shared", OutcomeState::NoEvent, OutcomeState::Event, OutcomeState::Event, OutcomeState::Event),
            pair("p3", "other", OutcomeState::NoEvent, OutcomeState::NoEvent, OutcomeState::Event, OutcomeState::Event),
        ]);
        let report = evaluate_manifest(&manifest).expect("clustered data");
        let safety = &report.endpoint_reports[0];
        assert_eq!(safety.cluster_n, 2);
        assert!(safety.mcnemar.is_none());
        assert!(safety.bootstrap_interval.is_some());
        assert_eq!(safety.status, "descriptive_only");
        assert_eq!(
            safety.bootstrap_interval.as_ref().unwrap().interpretation,
            "exploratory_descriptive_only"
        );
        assert_eq!(safety.cluster_weighted_risk_difference, 0.5);
        assert!((safety.episode_weighted_risk_difference - (2.0 / 3.0)).abs() < 1e-12);
    }

    #[test]
    fn cluster_bootstrap_is_deterministic_for_frozen_inputs() {
        let manifest = manifest_with(vec![
            pair("p1", "c1", OutcomeState::NoEvent, OutcomeState::Event, OutcomeState::Event, OutcomeState::Event),
            pair("p2", "c2", OutcomeState::NoEvent, OutcomeState::NoEvent, OutcomeState::Event, OutcomeState::Event),
            pair("p3", "c3", OutcomeState::NoEvent, OutcomeState::NoEvent, OutcomeState::Event, OutcomeState::Event),
        ]);
        let first = evaluate_manifest(&manifest).expect("first run");
        let second = evaluate_manifest(&manifest).expect("second run");
        assert_eq!(first, second);
    }

    #[test]
    fn zero_adverse_clusters_report_exact_one_sided_upper_bounds() {
        let for_ten = 1.0 - 0.05_f64.powf(1.0 / 10.0);
        let for_hundred = 1.0 - 0.05_f64.powf(1.0 / 100.0);
        assert!((for_ten - 0.2588655508930523).abs() < 1e-12);
        assert!((for_hundred - 0.029513049607039932).abs() < 1e-12);
    }

    #[test]
    fn zero_adverse_treatment_clusters_emit_bound_using_cluster_denominator() {
        let manifest = manifest_with(vec![
            pair("p1", "c1", OutcomeState::NoEvent, OutcomeState::NoEvent, OutcomeState::Event, OutcomeState::Event),
            pair("p2", "c2", OutcomeState::NoEvent, OutcomeState::NoEvent, OutcomeState::Event, OutcomeState::Event),
        ]);
        let report = evaluate_manifest(&manifest).expect("zero adverse clusters");
        let safety = &report.endpoint_reports[0];
        assert_eq!(safety.treatment_adverse_cluster_n, 0);
        assert_eq!(
            safety.zero_event_bound_assumption,
            "conditional_on_iid_independent_clusters_not_verified"
        );
        assert!(safety.reason_codes.iter().any(|code| {
            code == "ZERO_EVENT_BOUND_CONDITIONAL_ON_IID_INDEPENDENT_CLUSTERS"
        }));
        assert_eq!(
            safety.treatment_zero_adverse_cluster_upper_bound,
            Some(1.0 - 0.05_f64.powf(1.0 / 2.0))
        );
    }

    #[test]
    fn pair_cluster_mismatch_rejects() {
        let mut manifest = manifest_with(vec![pair(
            "p1",
            "cluster",
            OutcomeState::NoEvent,
            OutcomeState::NoEvent,
            OutcomeState::Event,
            OutcomeState::Event,
        )]);
        manifest.pairs[0].treatment_cluster_id = "other".to_owned();
        let error = evaluate_manifest(&manifest).expect_err("cluster mismatch");
        assert!(error.starts_with("PAIR_CLUSTER_ID_MISMATCH"));
    }

    #[test]
    fn attempt_ids_cannot_be_reused_across_pairs() {
        let mut manifest = manifest_with(vec![
            pair("p1", "c1", OutcomeState::NoEvent, OutcomeState::NoEvent, OutcomeState::Event, OutcomeState::Event),
            pair("p2", "c2", OutcomeState::NoEvent, OutcomeState::NoEvent, OutcomeState::Event, OutcomeState::Event),
        ]);
        manifest.pairs[1].control_attempt_id = manifest.pairs[0].control_attempt_id.clone();
        let error = evaluate_manifest(&manifest).expect_err("duplicate attempt ID");
        assert!(error.starts_with("DUPLICATE_ATTEMPT_ID"));
    }

    #[test]
    fn post_hoc_exclusions_reject() {
        let mut manifest = manifest_with(vec![pair(
            "p1",
            "cluster",
            OutcomeState::NoEvent,
            OutcomeState::NoEvent,
            OutcomeState::Event,
            OutcomeState::Event,
        )]);
        manifest.excluded_pair_ids.push("p2".to_owned());
        let error = evaluate_manifest(&manifest).expect_err("post-hoc exclusion");
        assert_eq!(error, "POST_HOC_PAIR_EXCLUSION_FORBIDDEN");
    }

    #[test]
    fn different_realized_outcomes_are_allowed_under_one_frozen_pair_input() {
        let manifest = manifest_with(vec![pair(
            "p1",
            "cluster",
            OutcomeState::NoEvent,
            OutcomeState::Event,
            OutcomeState::Event,
            OutcomeState::Event,
        )]);
        assert_eq!(manifest.pairs[0].frozen_input_sha256, "d".repeat(64));
        assert_ne!(
            manifest.pairs[0].outcomes[0].control,
            manifest.pairs[0].outcomes[0].treatment
        );
        assert!(evaluate_manifest(&manifest).is_ok());
    }
}
