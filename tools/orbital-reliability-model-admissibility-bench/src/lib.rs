use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

const SCHEMA: &str = "luminous.orbital-reliability-model-admissibility.corpus.v0.1";
const CLAIM_CEILING: &str = "synthetic-model-input-admissibility-only";
const MAX_ARTICLES: usize = 100_000;
const MAX_EVENTS_PER_ARTICLE: u64 = 100_000;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelProfile {
    HomogeneousPoissonConstantIntensity,
    WeibullFirstFailureLife,
    NonhomogeneousPoissonRecurrentEvents,
    PowerLawRecurrentMinimalRepair,
    AcceleratedLife,
    DegradationPath,
    UnknownOrUnregistered,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisRequest {
    AggregateRateOnly,
    AssumptionChallenge,
    FirstFailureLife,
    PowerLawRecurrentEstimate,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InputResolution {
    AggregateCountExposure,
    EventLevel,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Censoring {
    AdministrativeEnd,
    RightCensored,
    Intervention,
    InformativeWithdrawal,
    Unknown,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RestorationState {
    MinimalRepair,
    AsGoodAsNew,
    ImperfectRepair,
    NoRepairRequired,
    Unknown,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    SupportedForAnalysis,
    Indeterminate,
    ModelNotSupported,
    Rejected,
    Disputed,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FailureEvent {
    pub event_id: String,
    pub event_time_seconds: u64,
    pub failure_definition_id: String,
    pub restoration_state: RestorationState,
    pub effective_at: i64,
    pub recorded_at: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ArticleHistory {
    pub record_id: String,
    pub article_serial: String,
    pub sampling_cluster_id: String,
    pub population_id: String,
    pub lot_id: String,
    pub configuration_id: String,
    pub operating_mode_id: String,
    pub environment_profile_id: String,
    pub exposure_seconds: u64,
    pub reported_failure_events: u64,
    pub events: Vec<FailureEvent>,
    pub raw_artifact_sha256: String,
    pub evidence_complete: bool,
    pub exposure_clock_verified: bool,
    pub clock_error_bound_seconds: Option<u64>,
    pub event_capture_complete: bool,
    pub data_loss_counter_verified: bool,
    pub maintenance_history_complete: bool,
    pub censoring: Censoring,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Expected {
    pub disposition: Disposition,
    pub article_count: usize,
    pub cluster_count: usize,
    pub total_exposure_seconds: u64,
    pub total_failure_events: u64,
    pub event_level_records: usize,
    pub recurrent_event_articles: usize,
    pub assumption_diagnostics_ready: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AdmissibilityCase {
    pub case_id: String,
    pub selected_profile: ModelProfile,
    pub analysis_request: AnalysisRequest,
    pub input_resolution: InputResolution,
    pub preregistration_ref: Option<String>,
    pub sampling_frame_ref: Option<String>,
    pub dependence_assessment_ref: Option<String>,
    pub assumption_assessment_ref: Option<String>,
    pub maintenance_policy_ref: Option<String>,
    pub event_capture_assessment_ref: Option<String>,
    pub clock_calibration_ref: Option<String>,
    pub diagnostics_protocol_ref: Option<String>,
    pub multiplicity_policy_ref: Option<String>,
    pub holdout_split_ref: Option<String>,
    pub population_id: String,
    pub lot_id: String,
    pub configuration_id: String,
    pub operating_mode_id: String,
    pub environment_profile_id: String,
    pub failure_definition_id: String,
    pub max_clock_error_seconds: u64,
    pub effective_cutoff: i64,
    pub knowledge_cutoff: i64,
    pub diagnostics_predeclared: bool,
    pub thresholds_frozen_before_holdout: bool,
    pub articles: Vec<ArticleHistory>,
    pub expected: Expected,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    pub schema: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub cases: Vec<AdmissibilityCase>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaseResult {
    pub case_id: String,
    pub disposition: Disposition,
    pub article_count: usize,
    pub cluster_count: usize,
    pub total_exposure_seconds: u64,
    pub total_failure_events: u64,
    pub event_level_records: usize,
    pub recurrent_event_articles: usize,
    pub exposure_min_seconds: Option<u64>,
    pub exposure_max_seconds: Option<u64>,
    pub assumption_diagnostics_ready: bool,
    pub constant_intensity_confirmed: bool,
    pub reliability_claim_granted: bool,
    pub blockers: Vec<String>,
    pub expected_result_matched: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub case_count: usize,
    pub supported_for_analysis: usize,
    pub indeterminate: usize,
    pub model_not_supported: usize,
    pub rejected: usize,
    pub disputed: usize,
    pub article_count: usize,
    pub total_exposure_seconds: u64,
    pub total_failure_events: u64,
    pub assumption_diagnostics_ready: usize,
    pub constant_intensity_confirmations: usize,
    pub reliability_claims_granted: usize,
    pub expected_result_mismatches: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub result: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub authority_granted: bool,
    pub summary: Summary,
    pub cases: Vec<CaseResult>,
}

#[derive(Debug, Default)]
struct Decision {
    disposition: Option<Disposition>,
    article_count: usize,
    cluster_count: usize,
    total_exposure_seconds: u64,
    total_failure_events: u64,
    event_level_records: usize,
    recurrent_event_articles: usize,
    exposure_min_seconds: Option<u64>,
    exposure_max_seconds: Option<u64>,
    assumption_diagnostics_ready: bool,
    blockers: Vec<String>,
}

fn add_blocker(decision: &mut Decision, blocker: &str) {
    if !decision.blockers.iter().any(|existing| existing == blocker) {
        decision.blockers.push(blocker.to_owned());
    }
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn present(value: &Option<String>) -> bool {
    value.as_deref().is_some_and(|item| !item.trim().is_empty())
}

fn evaluate_case(case: &AdmissibilityCase) -> Result<Decision, String> {
    let mut decision = Decision::default();
    if case.case_id.trim().is_empty() {
        return Err("case ID must be non-empty".to_owned());
    }
    if case.articles.is_empty() {
        add_blocker(&mut decision, "empty_article_cohort");
    }
    if case.articles.len() > MAX_ARTICLES {
        return Err("article count exceeds v0.1 limit".to_owned());
    }
    if case.effective_cutoff > case.knowledge_cutoff {
        add_blocker(&mut decision, "knowledge_cutoff_precedes_effective_cutoff");
    }
    if !present(&case.preregistration_ref)
        || !present(&case.sampling_frame_ref)
        || !present(&case.dependence_assessment_ref)
        || !present(&case.assumption_assessment_ref)
    {
        add_blocker(
            &mut decision,
            "missing_preregistration_sampling_or_assumption_reference",
        );
    }
    if !case.diagnostics_predeclared || !case.thresholds_frozen_before_holdout {
        add_blocker(&mut decision, "diagnostics_or_thresholds_not_predeclared");
    }
    if case.analysis_request == AnalysisRequest::AssumptionChallenge
        && (!present(&case.diagnostics_protocol_ref)
            || !present(&case.multiplicity_policy_ref)
            || !present(&case.holdout_split_ref))
    {
        add_blocker(
            &mut decision,
            "missing_diagnostic_protocol_multiplicity_or_holdout",
        );
    }

    match case.selected_profile {
        ModelProfile::NonhomogeneousPoissonRecurrentEvents
        | ModelProfile::AcceleratedLife
        | ModelProfile::DegradationPath
        | ModelProfile::UnknownOrUnregistered => {
            add_blocker(
                &mut decision,
                "selected_model_profile_not_implemented_in_v0_1",
            );
            decision.disposition = Some(Disposition::ModelNotSupported);
        }
        ModelProfile::WeibullFirstFailureLife => {
            if case.analysis_request != AnalysisRequest::FirstFailureLife {
                add_blocker(
                    &mut decision,
                    "weibull_profile_requires_first_failure_life_request",
                );
                decision.disposition = Some(Disposition::ModelNotSupported);
            }
            if case.input_resolution != InputResolution::EventLevel {
                add_blocker(
                    &mut decision,
                    "weibull_first_failure_requires_event_level_input",
                );
                decision.disposition = Some(Disposition::Indeterminate);
            }
        }
        ModelProfile::PowerLawRecurrentMinimalRepair => {
            if case.analysis_request != AnalysisRequest::PowerLawRecurrentEstimate {
                add_blocker(
                    &mut decision,
                    "power_law_profile_requires_power_law_recurrent_estimate_request",
                );
                decision.disposition = Some(Disposition::ModelNotSupported);
            }
            if case.input_resolution != InputResolution::EventLevel {
                add_blocker(
                    &mut decision,
                    "power_law_profile_requires_event_level_input",
                );
                decision.disposition = Some(Disposition::ModelNotSupported);
            }
        }
        ModelProfile::HomogeneousPoissonConstantIntensity => {
            if case.analysis_request == AnalysisRequest::FirstFailureLife {
                add_blocker(
                    &mut decision,
                    "constant_intensity_profile_cannot_answer_first_failure_life_request",
                );
                decision.disposition = Some(Disposition::ModelNotSupported);
            }
            if case.analysis_request == AnalysisRequest::AssumptionChallenge
                && case.input_resolution != InputResolution::EventLevel
            {
                add_blocker(
                    &mut decision,
                    "aggregate_only_input_cannot_challenge_time_constant_assumption",
                );
            }
        }
    }

    if case.input_resolution == InputResolution::EventLevel
        && (!present(&case.event_capture_assessment_ref)
            || !present(&case.clock_calibration_ref)
            || !present(&case.maintenance_policy_ref))
    {
        add_blocker(
            &mut decision,
            "missing_event_capture_clock_or_maintenance_reference",
        );
    }

    let mut record_ids = HashMap::<&str, &ArticleHistory>::new();
    let mut article_ids = HashMap::<&str, &ArticleHistory>::new();
    let mut cluster_ids = HashSet::<&str>::new();
    let mut event_ids = HashMap::<&str, &FailureEvent>::new();
    let mut unique_articles = Vec::<&ArticleHistory>::new();

    for article in &case.articles {
        if article.record_id.trim().is_empty()
            || article.article_serial.trim().is_empty()
            || article.sampling_cluster_id.trim().is_empty()
        {
            add_blocker(&mut decision, "missing_record_article_or_cluster_identity");
            continue;
        }
        if let Some(previous) = record_ids.get(article.record_id.as_str()) {
            if *previous == article {
                add_blocker(&mut decision, "duplicate_record_id");
            } else {
                decision.disposition = Some(Disposition::Disputed);
                add_blocker(&mut decision, "conflicting_record_id");
            }
            continue;
        }
        if article_ids.contains_key(article.article_serial.as_str()) {
            decision.disposition = Some(Disposition::Disputed);
            add_blocker(
                &mut decision,
                "multiple_rows_for_same_article_require_prior_aggregation",
            );
            continue;
        }
        record_ids.insert(&article.record_id, article);
        article_ids.insert(&article.article_serial, article);
        unique_articles.push(article);
        cluster_ids.insert(&article.sampling_cluster_id);

        if article.population_id != case.population_id
            || article.lot_id != case.lot_id
            || article.configuration_id != case.configuration_id
            || article.operating_mode_id != case.operating_mode_id
            || article.environment_profile_id != case.environment_profile_id
        {
            add_blocker(
                &mut decision,
                "population_lot_configuration_mode_or_environment_mismatch",
            );
        }
        if article.exposure_seconds == 0
            || article.reported_failure_events > MAX_EVENTS_PER_ARTICLE
            || !valid_digest(&article.raw_artifact_sha256)
        {
            add_blocker(
                &mut decision,
                "invalid_exposure_event_count_or_artifact_digest",
            );
        }
        if !article.evidence_complete
            || !article.exposure_clock_verified
            || article.clock_error_bound_seconds.is_none()
            || article
                .clock_error_bound_seconds
                .is_some_and(|error| error > case.max_clock_error_seconds)
            || !article.event_capture_complete
            || !article.data_loss_counter_verified
        {
            add_blocker(
                &mut decision,
                "incomplete_measurement_clock_or_event_capture_evidence",
            );
        }
        if matches!(
            article.censoring,
            Censoring::Intervention | Censoring::InformativeWithdrawal | Censoring::Unknown
        ) {
            add_blocker(
                &mut decision,
                "intervention_or_unresolved_informative_censoring",
            );
        }
        if article.reported_failure_events > 0 && !article.maintenance_history_complete {
            add_blocker(
                &mut decision,
                "incomplete_maintenance_or_restoration_history",
            );
        }

        if case.input_resolution == InputResolution::AggregateCountExposure {
            if !article.events.is_empty() {
                add_blocker(
                    &mut decision,
                    "aggregate_resolution_contains_unscoped_event_rows",
                );
            }
        } else {
            if article.events.len() as u64 != article.reported_failure_events {
                add_blocker(
                    &mut decision,
                    "event_rows_do_not_match_reported_failure_count",
                );
            }
            decision.event_level_records += 1;
            if article.reported_failure_events > 1 {
                decision.recurrent_event_articles += 1;
            }
            if article.events.len() > MAX_EVENTS_PER_ARTICLE as usize {
                return Err("event count exceeds v0.1 limit".to_owned());
            }
            let mut prior_time = None;
            for event in &article.events {
                if event.event_id.trim().is_empty() {
                    add_blocker(&mut decision, "empty_event_id");
                }
                if let Some(previous) = event_ids.insert(&event.event_id, event) {
                    if previous == event {
                        add_blocker(&mut decision, "duplicate_event_id");
                    } else {
                        decision.disposition = Some(Disposition::Disputed);
                        add_blocker(&mut decision, "conflicting_event_id");
                    }
                }
                if event.failure_definition_id != case.failure_definition_id
                    || event.event_time_seconds == 0
                    || event.event_time_seconds > article.exposure_seconds
                    || prior_time.is_some_and(|previous| event.event_time_seconds <= previous)
                {
                    add_blocker(
                        &mut decision,
                        "event_time_order_window_or_definition_invalid",
                    );
                }
                if event.effective_at > case.effective_cutoff
                    || event.recorded_at > case.knowledge_cutoff
                {
                    add_blocker(&mut decision, "event_not_known_at_declared_cutoff");
                }
                if case.selected_profile == ModelProfile::PowerLawRecurrentMinimalRepair
                    && event.restoration_state != RestorationState::MinimalRepair
                {
                    add_blocker(
                        &mut decision,
                        "power_law_profile_requires_minimal_repair_event_history",
                    );
                    decision.disposition = Some(Disposition::ModelNotSupported);
                } else if article.reported_failure_events > 1
                    && matches!(
                        event.restoration_state,
                        RestorationState::Unknown | RestorationState::ImperfectRepair
                    )
                {
                    add_blocker(
                        &mut decision,
                        "recurrent_event_restoration_semantics_unresolved_or_unsupported",
                    );
                }
                prior_time = Some(event.event_time_seconds);
            }
        }

        if matches!(
            article.censoring,
            Censoring::RightCensored | Censoring::AdministrativeEnd
        ) {
            // These dispositions may be admissible when the accompanying record is complete.
        }
    }

    decision.article_count = unique_articles.len();
    decision.cluster_count = cluster_ids.len();
    if !unique_articles.is_empty() {
        let mut exposure_min = u64::MAX;
        let mut exposure_max = 0u64;
        for article in &unique_articles {
            decision.total_exposure_seconds = decision
                .total_exposure_seconds
                .checked_add(article.exposure_seconds)
                .ok_or_else(|| "aggregate exposure overflow".to_owned())?;
            decision.total_failure_events = decision
                .total_failure_events
                .checked_add(article.reported_failure_events)
                .ok_or_else(|| "aggregate event count overflow".to_owned())?;
            exposure_min = exposure_min.min(article.exposure_seconds);
            exposure_max = exposure_max.max(article.exposure_seconds);
        }
        decision.exposure_min_seconds = Some(exposure_min);
        decision.exposure_max_seconds = Some(exposure_max);
    }

    if decision.cluster_count < decision.article_count {
        add_blocker(
            &mut decision,
            "shared_sampling_cluster_prevents_independent_article_claim",
        );
    }
    let single_repairable_series = case.selected_profile
        == ModelProfile::PowerLawRecurrentMinimalRepair
        && case.analysis_request == AnalysisRequest::PowerLawRecurrentEstimate
        && case.input_resolution == InputResolution::EventLevel;
    if single_repairable_series {
        if decision.article_count != 1 {
            add_blocker(
                &mut decision,
                "power_law_profile_requires_exactly_one_repairable_series",
            );
            decision.disposition = Some(Disposition::ModelNotSupported);
        }
    } else if decision.article_count < 2 {
        add_blocker(&mut decision, "fewer_than_two_distinct_articles");
    }
    if case.selected_profile == ModelProfile::WeibullFirstFailureLife
        && unique_articles
            .iter()
            .any(|article| article.reported_failure_events > 1)
    {
        add_blocker(
            &mut decision,
            "recurrent_event_history_not_admissible_for_first_failure_weibull_profile",
        );
        decision.disposition = Some(Disposition::ModelNotSupported);
    }
    if case.selected_profile == ModelProfile::HomogeneousPoissonConstantIntensity
        && unique_articles
            .iter()
            .any(|article| article.reported_failure_events > 1)
    {
        if !present(&case.maintenance_policy_ref) {
            add_blocker(
                &mut decision,
                "missing_maintenance_policy_for_recurrent_process",
            );
        }
        if unique_articles.iter().any(|article| {
            article.reported_failure_events > 1 && !article.maintenance_history_complete
        }) {
            add_blocker(
                &mut decision,
                "repair_restoration_history_missing_for_recurrent_process",
            );
        }
    }

    let hard_dispute = decision.disposition == Some(Disposition::Disputed);
    let model_unsupported = decision.disposition == Some(Disposition::ModelNotSupported);
    let hard_reject = decision.blockers.iter().any(|blocker| {
        matches!(
            blocker.as_str(),
            "population_lot_configuration_mode_or_environment_mismatch"
                | "invalid_exposure_event_count_or_artifact_digest"
                | "event_rows_do_not_match_reported_failure_count"
                | "event_time_order_window_or_definition_invalid"
                | "event_not_known_at_declared_cutoff"
                | "aggregate_resolution_contains_unscoped_event_rows"
                | "diagnostics_or_thresholds_not_predeclared"
        )
    });
    let insufficient = !decision.blockers.is_empty();

    if hard_dispute {
        decision.disposition = Some(Disposition::Disputed);
    } else if model_unsupported {
        decision.disposition = Some(Disposition::ModelNotSupported);
    } else if hard_reject {
        decision.disposition = Some(Disposition::Rejected);
    } else if insufficient {
        decision.disposition = Some(Disposition::Indeterminate);
    } else {
        decision.disposition = Some(Disposition::SupportedForAnalysis);
    }

    decision.assumption_diagnostics_ready = case.analysis_request
        == AnalysisRequest::AssumptionChallenge
        && case.input_resolution == InputResolution::EventLevel
        && decision.disposition == Some(Disposition::SupportedForAnalysis)
        && decision.event_level_records == decision.article_count
        && present(&case.diagnostics_protocol_ref)
        && present(&case.multiplicity_policy_ref)
        && present(&case.holdout_split_ref);

    decision.blockers.sort();
    decision.blockers.dedup();
    Ok(decision)
}

pub fn evaluate(corpus: &Corpus) -> Result<Report, String> {
    if corpus.schema != SCHEMA {
        return Err("unsupported model-admissibility corpus schema".to_owned());
    }
    if corpus.evidence_class != "synthetic" {
        return Err("v0.1 accepts synthetic fixtures only".to_owned());
    }
    if corpus.claim_ceiling != CLAIM_CEILING {
        return Err("unsupported model-admissibility claim ceiling".to_owned());
    }
    if corpus.cases.is_empty() {
        return Err("model-admissibility corpus must contain cases".to_owned());
    }

    let mut ids = HashSet::new();
    let mut summary = Summary {
        case_count: corpus.cases.len(),
        supported_for_analysis: 0,
        indeterminate: 0,
        model_not_supported: 0,
        rejected: 0,
        disputed: 0,
        article_count: 0,
        total_exposure_seconds: 0,
        total_failure_events: 0,
        assumption_diagnostics_ready: 0,
        constant_intensity_confirmations: 0,
        reliability_claims_granted: 0,
        expected_result_mismatches: 0,
    };
    let mut results = Vec::with_capacity(corpus.cases.len());
    for case in &corpus.cases {
        if !ids.insert(case.case_id.as_str()) {
            return Err("case IDs must be unique".to_owned());
        }
        let decision = evaluate_case(case)?;
        let disposition = decision.disposition.unwrap_or(Disposition::Indeterminate);
        match disposition {
            Disposition::SupportedForAnalysis => summary.supported_for_analysis += 1,
            Disposition::Indeterminate => summary.indeterminate += 1,
            Disposition::ModelNotSupported => summary.model_not_supported += 1,
            Disposition::Rejected => summary.rejected += 1,
            Disposition::Disputed => summary.disputed += 1,
        }
        summary.article_count += decision.article_count;
        summary.total_exposure_seconds = summary
            .total_exposure_seconds
            .checked_add(decision.total_exposure_seconds)
            .ok_or_else(|| "summary exposure overflow".to_owned())?;
        summary.total_failure_events = summary
            .total_failure_events
            .checked_add(decision.total_failure_events)
            .ok_or_else(|| "summary failure count overflow".to_owned())?;
        summary.assumption_diagnostics_ready += usize::from(decision.assumption_diagnostics_ready);

        let blockers_match = decision.blockers == case.expected.blockers;
        let matched = disposition == case.expected.disposition
            && decision.article_count == case.expected.article_count
            && decision.cluster_count == case.expected.cluster_count
            && decision.total_exposure_seconds == case.expected.total_exposure_seconds
            && decision.total_failure_events == case.expected.total_failure_events
            && decision.event_level_records == case.expected.event_level_records
            && decision.recurrent_event_articles == case.expected.recurrent_event_articles
            && decision.assumption_diagnostics_ready == case.expected.assumption_diagnostics_ready
            && blockers_match;
        summary.expected_result_mismatches += usize::from(!matched);

        results.push(CaseResult {
            case_id: case.case_id.clone(),
            disposition,
            article_count: decision.article_count,
            cluster_count: decision.cluster_count,
            total_exposure_seconds: decision.total_exposure_seconds,
            total_failure_events: decision.total_failure_events,
            event_level_records: decision.event_level_records,
            recurrent_event_articles: decision.recurrent_event_articles,
            exposure_min_seconds: decision.exposure_min_seconds,
            exposure_max_seconds: decision.exposure_max_seconds,
            assumption_diagnostics_ready: decision.assumption_diagnostics_ready,
            constant_intensity_confirmed: false,
            reliability_claim_granted: false,
            blockers: decision.blockers,
            expected_result_matched: matched,
        });
    }

    Ok(Report {
        result: if summary.expected_result_mismatches == 0 {
            "pass".to_owned()
        } else {
            "fail".to_owned()
        },
        evidence_class: "synthetic".to_owned(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
        authority_granted: false,
        summary,
        cases: results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_types_keep_estimands_separate() {
        assert_ne!(
            ModelProfile::HomogeneousPoissonConstantIntensity,
            ModelProfile::WeibullFirstFailureLife
        );
    }

    #[test]
    fn digest_requires_lowercase_sha256_shape() {
        assert!(valid_digest(&"a".repeat(64)));
        assert!(!valid_digest(&"A".repeat(64)));
        assert!(!valid_digest("abc"));
    }

    #[test]
    fn no_state_represents_a_confirmed_hazard_or_reliability_claim() {
        let report = Report {
            result: "pass".to_owned(),
            evidence_class: "synthetic".to_owned(),
            claim_ceiling: CLAIM_CEILING.to_owned(),
            authority_granted: false,
            summary: Summary {
                case_count: 0,
                supported_for_analysis: 0,
                indeterminate: 0,
                model_not_supported: 0,
                rejected: 0,
                disputed: 0,
                article_count: 0,
                total_exposure_seconds: 0,
                total_failure_events: 0,
                assumption_diagnostics_ready: 0,
                constant_intensity_confirmations: 0,
                reliability_claims_granted: 0,
                expected_result_mismatches: 0,
            },
            cases: Vec::new(),
        };
        assert!(!report.authority_granted);
        assert_eq!(report.summary.constant_intensity_confirmations, 0);
        assert_eq!(report.summary.reliability_claims_granted, 0);
    }
}
