// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Evidence-qualified evaluation of pipe localization under physical
//! observability limits.
//!
//! This module consumes an evaluator-side ambiguity set supplied by the
//! hydraulic/model owner (Sol-Atlas). It does not compute hydraulic
//! sensitivity, model discrepancy, sensor placement, or the ambiguity set.
//! It checks that the supplied oracle evidence is well-formed and separates
//! algorithmic ranking skill from the fraction of cases where the sensing
//! geometry can guarantee a Top-K pipe answer.
//!
//! An oracle candidate set is a set of pipes that remain consistent with the
//! declared physical model, sensor layout, observations, and tolerance. When
//! that set contains more than K pipes, no Top-K policy can guarantee capture
//! of the true pipe for every member of the set. This is a conservative
//! *guaranteed feasibility* ceiling, not an expected accuracy bound.

use std::collections::BTreeSet;
use std::fmt::Write as _;

/// Physical inference resolution supplied by the observability owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OracleResolution {
    /// The evidence uniquely identifies one pipe.
    UniquePipe,
    /// Multiple pipes remain hydraulically observationally equivalent.
    AmbiguousPipeSet,
    /// The evidence resolves a zone, but not a unique pipe within it.
    ZoneOnly,
    /// An event may be detected, but no spatial candidate set is supported.
    EventOnly,
    /// The physical/model evidence is too uncertain to establish a ceiling.
    ModelUncertain,
    /// Required observations were not measured.
    NotMeasured,
    /// The requested inference is unsupported by the evidence contract.
    Unsupported,
}

impl OracleResolution {
    /// Stable machine-readable resolution label.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UniquePipe => "unique_pipe",
            Self::AmbiguousPipeSet => "ambiguous_pipe_set",
            Self::ZoneOnly => "zone_only",
            Self::EventOnly => "event_only",
            Self::ModelUncertain => "model_uncertain",
            Self::NotMeasured => "not_measured",
            Self::Unsupported => "unsupported",
        }
    }

    /// Whether the oracle evidence is sufficiently grounded to contribute to
    /// the physical Top-K feasibility denominator.
    pub const fn has_trusted_pipe_set(self) -> bool {
        matches!(
            self,
            Self::UniquePipe | Self::AmbiguousPipeSet | Self::ZoneOnly
        )
    }
}

/// Owner-supplied physical ambiguity evidence for one event and sensor layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservabilityEvidence {
    /// Resolution class computed by the hydraulic/observability owner.
    pub resolution: OracleResolution,
    /// Pipe IDs consistent with the frozen measurements/model/tolerance.
    ///
    /// This is an oracle candidate set, not the model's prediction.
    pub candidate_pipe_ids: Vec<String>,
    /// Zone identity when the oracle resolves only to a zone.
    pub zone_id: Option<String>,
    /// SHA-256 digest of the source observability receipt.
    ///
    /// Syntax is checked, but this module does not authenticate or recompute it.
    pub evidence_digest: String,
}

/// One ground-truthed case with oracle evidence and a model pipe ranking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalizationCase {
    /// Stable case identifier.
    pub case_id: String,
    /// Hydraulic network identity.
    pub network_id: String,
    /// Frozen sensor-layout identity.
    pub sensor_layout_id: String,
    /// Complete pipe universe for this network/version.
    pub network_pipe_ids: Vec<String>,
    /// Independently established true pipe.
    pub truth_pipe_id: String,
    /// Sol-Atlas or other owner-supplied observability evidence.
    pub observability: ObservabilityEvidence,
    /// Model-ranked pipe IDs, best first; empty only on explicit abstention.
    pub ranked_pipe_ids: Vec<String>,
    /// Whether the model explicitly withheld a pipe answer.
    pub abstained: bool,
    /// Model-estimated probability (parts per million) that its Top-K list
    /// contains the true pipe. Required when a prediction is issued.
    pub top_k_confidence_ppm: Option<u32>,
}

/// Frozen evaluation configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalizationEvaluationConfig {
    /// Ranking depth to evaluate.
    pub top_k: usize,
    /// Confidence thresholds for the risk–coverage curve, ascending and unique.
    pub confidence_thresholds_ppm: Vec<u32>,
}

/// One point in the selective risk–coverage curve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RiskCoveragePoint {
    /// Minimum confidence threshold for retaining a model prediction.
    pub confidence_threshold_ppm: u32,
    /// Predictions retained at this threshold on trusted-oracle cases.
    pub selected_count: usize,
    /// Retained predictions / all trusted-oracle cases, in parts per million.
    pub coverage_ppm: Option<u32>,
    /// Top-K hits / retained predictions, in parts per million.
    pub top_k_accuracy_ppm: Option<u32>,
    /// 1 - Top-K accuracy, in parts per million.
    pub top_k_risk_ppm: Option<u32>,
}

/// Deterministic summary that keeps oracle feasibility separate from model skill.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalizationEvaluationReport {
    /// Version of this evaluation contract.
    pub contract_version: &'static str,
    /// Ranking depth evaluated.
    pub top_k: usize,
    /// Number of validated cases.
    pub total_cases: usize,
    /// Cases with a trusted oracle pipe candidate set.
    pub trusted_oracle_cases: usize,
    /// Trusted cases whose oracle candidate set fits entirely within Top-K.
    pub oracle_top_k_feasible_cases: usize,
    /// Guaranteed Top-K feasibility / trusted-oracle cases, in parts per million.
    pub oracle_top_k_feasibility_ppm: Option<u32>,
    /// Cases whose oracle resolves to one unique pipe.
    pub unique_pipe_cases: usize,
    /// Cases with multiple observationally equivalent pipes.
    pub ambiguous_pipe_set_cases: usize,
    /// Cases resolved to a zone, not a unique pipe.
    pub zone_only_cases: usize,
    /// Cases with event-only spatial resolution.
    pub event_only_cases: usize,
    /// Cases with unresolved model/physical uncertainty.
    pub model_uncertain_cases: usize,
    /// Cases whose required observations were not measured.
    pub not_measured_cases: usize,
    /// Cases unsupported by the evidence contract.
    pub unsupported_cases: usize,
    /// Cases where a pipe ranking was issued.
    pub predictions_issued: usize,
    /// Predictions issued on trusted-oracle cases.
    pub scorable_predictions_issued: usize,
    /// Pipe rankings issued without a trusted pipe-level oracle.
    pub unqualified_pipe_claims: usize,
    /// Abstentions where pipe inference is unscorable or cannot be guaranteed
    /// at the configured Top-K depth.
    pub safe_abstentions: usize,
    /// Abstentions despite a trusted oracle set that fits within Top-K.
    pub unnecessary_abstentions: usize,
    /// Top-K hits among issued predictions on trusted-oracle cases.
    pub top_k_hits: usize,
    /// Top-K hits / issued scorable predictions, in parts per million.
    pub conditional_top_k_accuracy_ppm: Option<u32>,
    /// Top-K hits / all trusted-oracle cases, in parts per million.
    /// Abstentions on otherwise scorable cases count as misses.
    pub system_top_k_accuracy_ppm: Option<u32>,
    /// Issued scorable predictions / trusted-oracle cases, in parts per million.
    pub scorable_prediction_coverage_ppm: Option<u32>,
    /// Top-K risk–coverage points at configured confidence thresholds.
    pub risk_coverage: Vec<RiskCoveragePoint>,
}

impl LocalizationEvaluationReport {
    /// Stable line-oriented summary. Source pipe IDs are intentionally omitted.
    pub fn canonical_text(&self) -> String {
        let mut output = String::new();
        let _ = writeln!(&mut output, "CIVBENCH_LOCALIZATION_OBSERVABILITY_V0.1");
        let _ = writeln!(&mut output, "top_k={}", self.top_k);
        let _ = writeln!(&mut output, "total_cases={}", self.total_cases);
        let _ = writeln!(
            &mut output,
            "trusted_oracle_cases={}",
            self.trusted_oracle_cases
        );
        let _ = writeln!(
            &mut output,
            "oracle_top_k_feasible_cases={}",
            self.oracle_top_k_feasible_cases
        );
        let _ = writeln!(
            &mut output,
            "oracle_top_k_feasibility_ppm={}",
            option_ppm(self.oracle_top_k_feasibility_ppm)
        );
        let _ = writeln!(&mut output, "unique_pipe_cases={}", self.unique_pipe_cases);
        let _ = writeln!(
            &mut output,
            "ambiguous_pipe_set_cases={}",
            self.ambiguous_pipe_set_cases
        );
        let _ = writeln!(&mut output, "zone_only_cases={}", self.zone_only_cases);
        let _ = writeln!(&mut output, "event_only_cases={}", self.event_only_cases);
        let _ = writeln!(
            &mut output,
            "model_uncertain_cases={}",
            self.model_uncertain_cases
        );
        let _ = writeln!(
            &mut output,
            "not_measured_cases={}",
            self.not_measured_cases
        );
        let _ = writeln!(
            &mut output,
            "unsupported_cases={}",
            self.unsupported_cases
        );
        let _ = writeln!(&mut output, "predictions_issued={}", self.predictions_issued);
        let _ = writeln!(
            &mut output,
            "scorable_predictions_issued={}",
            self.scorable_predictions_issued
        );
        let _ = writeln!(
            &mut output,
            "unqualified_pipe_claims={}",
            self.unqualified_pipe_claims
        );
        let _ = writeln!(&mut output, "safe_abstentions={}", self.safe_abstentions);
        let _ = writeln!(
            &mut output,
            "unnecessary_abstentions={}",
            self.unnecessary_abstentions
        );
        let _ = writeln!(&mut output, "top_k_hits={}", self.top_k_hits);
        let _ = writeln!(
            &mut output,
            "conditional_top_k_accuracy_ppm={}",
            option_ppm(self.conditional_top_k_accuracy_ppm)
        );
        let _ = writeln!(
            &mut output,
            "system_top_k_accuracy_ppm={}",
            option_ppm(self.system_top_k_accuracy_ppm)
        );
        let _ = writeln!(
            &mut output,
            "scorable_prediction_coverage_ppm={}",
            option_ppm(self.scorable_prediction_coverage_ppm)
        );
        for point in &self.risk_coverage {
            let _ = writeln!(
                &mut output,
                "risk_coverage={},{},{},{},{}",
                point.confidence_threshold_ppm,
                point.selected_count,
                option_ppm(point.coverage_ppm),
                option_ppm(point.top_k_accuracy_ppm),
                option_ppm(point.top_k_risk_ppm)
            );
        }
        output
    }
}

/// Stable validation or evaluation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalizationEvaluationError {
    /// Stable machine-readable error code.
    pub code: &'static str,
    /// Case, field, or offending identity.
    pub subject: String,
}

impl LocalizationEvaluationError {
    fn new(code: &'static str, subject: impl Into<String>) -> Self {
        Self {
            code,
            subject: subject.into(),
        }
    }
}

impl std::fmt::Display for LocalizationEvaluationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.subject)
    }
}

impl std::error::Error for LocalizationEvaluationError {}

fn valid_identifier(value: &str) -> bool {
    if value.is_empty() || value.len() > 256 || value.trim() != value {
        return false;
    }
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    first.is_ascii_alphanumeric()
        && bytes.all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
        })
}

fn normalized_sha256(value: &str) -> Option<String> {
    let hex = value.strip_prefix("sha256:")?;
    if hex.len() != 64
        || !hex.bytes().all(|byte| byte.is_ascii_hexdigit())
        || hex.bytes().all(|byte| byte == b'0')
    {
        return None;
    }
    Some(hex.to_ascii_lowercase())
}

fn ratio_ppm(numerator: usize, denominator: usize) -> Option<u32> {
    if denominator == 0 {
        return None;
    }
    let scaled = (numerator as u128)
        .saturating_mul(1_000_000)
        .checked_div(denominator as u128)
        .unwrap_or(0);
    Some(scaled.min(1_000_000) as u32)
}

fn option_ppm(value: Option<u32>) -> String {
    value.map_or_else(|| "none".to_owned(), |value| value.to_string())
}

fn insert_unique(
    values: &mut BTreeSet<String>,
    value: &str,
    code: &'static str,
    subject: &str,
) -> Result<(), LocalizationEvaluationError> {
    if !valid_identifier(value) {
        return Err(LocalizationEvaluationError::new(code, subject));
    }
    if !values.insert(value.to_owned()) {
        return Err(LocalizationEvaluationError::new(code, value));
    }
    Ok(())
}

fn validate_case(case: &LocalizationCase) -> Result<(), LocalizationEvaluationError> {
    if !valid_identifier(&case.case_id)
        || !valid_identifier(&case.network_id)
        || !valid_identifier(&case.sensor_layout_id)
    {
        return Err(LocalizationEvaluationError::new(
            "invalid_case_context",
            case.case_id.as_str(),
        ));
    }

    let mut network_pipes = BTreeSet::new();
    for pipe_id in &case.network_pipe_ids {
        insert_unique(
            &mut network_pipes,
            pipe_id,
            "invalid_or_duplicate_network_pipe",
            case.case_id.as_str(),
        )?;
    }
    if network_pipes.is_empty() {
        return Err(LocalizationEvaluationError::new(
            "empty_network_pipe_universe",
            case.case_id.as_str(),
        ));
    }
    if !network_pipes.contains(&case.truth_pipe_id) {
        return Err(LocalizationEvaluationError::new(
            "truth_pipe_outside_network",
            case.case_id.as_str(),
        ));
    }

    let _digest = normalized_sha256(&case.observability.evidence_digest).ok_or_else(|| {
        LocalizationEvaluationError::new(
            "invalid_observability_digest",
            case.case_id.as_str(),
        )
    })?;

    let mut oracle_pipes = BTreeSet::new();
    for pipe_id in &case.observability.candidate_pipe_ids {
        insert_unique(
            &mut oracle_pipes,
            pipe_id,
            "invalid_or_duplicate_oracle_pipe",
            case.case_id.as_str(),
        )?;
        if !network_pipes.contains(pipe_id) {
            return Err(LocalizationEvaluationError::new(
                "oracle_pipe_outside_network",
                format!("{}:{pipe_id}", case.case_id),
            ));
        }
    }

    let zone_present = case.observability.zone_id.is_some();
    if let Some(zone_id) = &case.observability.zone_id {
        if !valid_identifier(zone_id) {
            return Err(LocalizationEvaluationError::new(
                "invalid_zone_id",
                case.case_id.as_str(),
            ));
        }
    }
    match case.observability.resolution {
        OracleResolution::UniquePipe => {
            if oracle_pipes.len() != 1
                || !oracle_pipes.contains(&case.truth_pipe_id)
                || zone_present
            {
                return Err(LocalizationEvaluationError::new(
                    "unique_pipe_oracle_inconsistent",
                    case.case_id.as_str(),
                ));
            }
        }
        OracleResolution::AmbiguousPipeSet => {
            if oracle_pipes.len() < 2
                || !oracle_pipes.contains(&case.truth_pipe_id)
                || zone_present
            {
                return Err(LocalizationEvaluationError::new(
                    "ambiguous_pipe_oracle_inconsistent",
                    case.case_id.as_str(),
                ));
            }
        }
        OracleResolution::ZoneOnly => {
            if oracle_pipes.len() < 2
                || !oracle_pipes.contains(&case.truth_pipe_id)
                || !zone_present
            {
                return Err(LocalizationEvaluationError::new(
                    "zone_oracle_inconsistent",
                    case.case_id.as_str(),
                ));
            }
        }
        OracleResolution::EventOnly | OracleResolution::NotMeasured | OracleResolution::Unsupported => {
            if !oracle_pipes.is_empty() || zone_present {
                return Err(LocalizationEvaluationError::new(
                    "unlocalized_oracle_has_candidates",
                    case.case_id.as_str(),
                ));
            }
        }
        OracleResolution::ModelUncertain => {}
    }

    let mut ranking = BTreeSet::new();
    for pipe_id in &case.ranked_pipe_ids {
        insert_unique(
            &mut ranking,
            pipe_id,
            "invalid_or_duplicate_predicted_pipe",
            case.case_id.as_str(),
        )?;
        if !network_pipes.contains(pipe_id) {
            return Err(LocalizationEvaluationError::new(
                "predicted_pipe_outside_network",
                format!("{}:{pipe_id}", case.case_id),
            ));
        }
    }

    if case.abstained {
        if !case.ranked_pipe_ids.is_empty() || case.top_k_confidence_ppm.is_some() {
            return Err(LocalizationEvaluationError::new(
                "abstention_carries_prediction",
                case.case_id.as_str(),
            ));
        }
    } else {
        if case.ranked_pipe_ids.is_empty() {
            return Err(LocalizationEvaluationError::new(
                "empty_prediction_without_abstention",
                case.case_id.as_str(),
            ));
        }
        match case.top_k_confidence_ppm {
            Some(confidence) if confidence <= 1_000_000 => {}
            Some(_) => {
                return Err(LocalizationEvaluationError::new(
                    "confidence_out_of_range",
                    case.case_id.as_str(),
                ));
            }
            None => {
                return Err(LocalizationEvaluationError::new(
                    "missing_top_k_confidence",
                    case.case_id.as_str(),
                ));
            }
        }
    }
    Ok(())
}

/// Evaluate model localization against an independently supplied physical
/// observability oracle.
///
/// Trusted oracle classes are UniquePipe, AmbiguousPipeSet, and ZoneOnly.
/// EventOnly, ModelUncertain, NotMeasured, and Unsupported remain visible as
/// separate counts but are excluded from Top-K algorithmic accuracy and the
/// physical feasibility denominator. Abstention on an unscorable oracle or
/// when its ambiguity set exceeds K is counted as safe; abstention when the
/// full trusted set fits in K is counted as unnecessary.
///
/// The oracle digest is retained only as a validated reference. The caller is
/// responsible for computing/authenticating the source receipt and for keeping
/// the truth source independent from candidate-visible model input.
pub fn evaluate_localization(
    cases: &[LocalizationCase],
    config: &LocalizationEvaluationConfig,
) -> Result<LocalizationEvaluationReport, LocalizationEvaluationError> {
    if config.top_k == 0 {
        return Err(LocalizationEvaluationError::new(
            "invalid_top_k",
            "must be positive",
        ));
    }
    let mut previous_threshold = None;
    for threshold in &config.confidence_thresholds_ppm {
        if *threshold > 1_000_000
            || previous_threshold.is_some_and(|previous| *threshold <= previous)
        {
            return Err(LocalizationEvaluationError::new(
                "invalid_confidence_thresholds",
                threshold.to_string(),
            ));
        }
        previous_threshold = Some(*threshold);
    }
    if cases.is_empty() {
        return Err(LocalizationEvaluationError::new(
            "empty_localization_corpus",
            "no cases",
        ));
    }

    let mut ordered_cases: Vec<&LocalizationCase> = cases.iter().collect();
    ordered_cases.sort_by(|left, right| left.case_id.cmp(&right.case_id));

    let mut case_ids = BTreeSet::new();
    let mut trusted_oracle_cases = 0;
    let mut oracle_top_k_feasible_cases = 0;
    let mut unique_pipe_cases = 0;
    let mut ambiguous_pipe_set_cases = 0;
    let mut zone_only_cases = 0;
    let mut event_only_cases = 0;
    let mut model_uncertain_cases = 0;
    let mut not_measured_cases = 0;
    let mut unsupported_cases = 0;
    let mut predictions_issued = 0;
    let mut scorable_predictions_issued = 0;
    let mut unqualified_pipe_claims = 0;
    let mut safe_abstentions = 0;
    let mut unnecessary_abstentions = 0;
    let mut top_k_hits = 0;

    for case in &ordered_cases {
        if !case_ids.insert(case.case_id.as_str()) {
            return Err(LocalizationEvaluationError::new(
                "duplicate_case_id",
                case.case_id.as_str(),
            ));
        }
        validate_case(case)?;

        let resolution = case.observability.resolution;
        let trusted = resolution.has_trusted_pipe_set();
        if trusted {
            trusted_oracle_cases += 1;
            if case.observability.candidate_pipe_ids.len() <= config.top_k {
                oracle_top_k_feasible_cases += 1;
            }
        }
        match resolution {
            OracleResolution::UniquePipe => unique_pipe_cases += 1,
            OracleResolution::AmbiguousPipeSet => ambiguous_pipe_set_cases += 1,
            OracleResolution::ZoneOnly => zone_only_cases += 1,
            OracleResolution::EventOnly => event_only_cases += 1,
            OracleResolution::ModelUncertain => model_uncertain_cases += 1,
            OracleResolution::NotMeasured => not_measured_cases += 1,
            OracleResolution::Unsupported => unsupported_cases += 1,
        }

        let physically_guaranteed_at_k =
            trusted && case.observability.candidate_pipe_ids.len() <= config.top_k;
        if case.abstained {
            if physically_guaranteed_at_k {
                unnecessary_abstentions += 1;
            } else {
                safe_abstentions += 1;
            }
            continue;
        }

        predictions_issued += 1;
        if !trusted {
            unqualified_pipe_claims += 1;
            continue;
        }
        scorable_predictions_issued += 1;
        if case
            .ranked_pipe_ids
            .iter()
            .take(config.top_k)
            .any(|pipe_id| pipe_id == &case.truth_pipe_id)
        {
            top_k_hits += 1;
        }
    }

    let mut risk_coverage = Vec::with_capacity(config.confidence_thresholds_ppm.len());
    for threshold in &config.confidence_thresholds_ppm {
        let mut selected_count = 0;
        let mut selected_hits = 0;
        for case in &ordered_cases {
            if !case.observability.resolution.has_trusted_pipe_set() || case.abstained {
                continue;
            }
            if case.top_k_confidence_ppm.is_some_and(|value| value >= *threshold) {
                selected_count += 1;
                if case
                    .ranked_pipe_ids
                    .iter()
                    .take(config.top_k)
                    .any(|pipe_id| pipe_id == &case.truth_pipe_id)
                {
                    selected_hits += 1;
                }
            }
        }
        let accuracy = ratio_ppm(selected_hits, selected_count);
        risk_coverage.push(RiskCoveragePoint {
            confidence_threshold_ppm: *threshold,
            selected_count,
            coverage_ppm: ratio_ppm(selected_count, trusted_oracle_cases),
            top_k_accuracy_ppm: accuracy,
            top_k_risk_ppm: accuracy.map(|value| 1_000_000 - value),
        });
    }

    Ok(LocalizationEvaluationReport {
        contract_version: "0.1.0",
        top_k: config.top_k,
        total_cases: cases.len(),
        trusted_oracle_cases,
        oracle_top_k_feasible_cases,
        oracle_top_k_feasibility_ppm: ratio_ppm(
            oracle_top_k_feasible_cases,
            trusted_oracle_cases,
        ),
        unique_pipe_cases,
        ambiguous_pipe_set_cases,
        zone_only_cases,
        event_only_cases,
        model_uncertain_cases,
        not_measured_cases,
        unsupported_cases,
        predictions_issued,
        scorable_predictions_issued,
        unqualified_pipe_claims,
        safe_abstentions,
        unnecessary_abstentions,
        top_k_hits,
        conditional_top_k_accuracy_ppm: ratio_ppm(top_k_hits, scorable_predictions_issued),
        system_top_k_accuracy_ppm: ratio_ppm(top_k_hits, trusted_oracle_cases),
        scorable_prediction_coverage_ppm: ratio_ppm(
            scorable_predictions_issued,
            trusted_oracle_cases,
        ),
        risk_coverage,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(value: u64) -> String {
        format!("sha256:{value:064x}")
    }

    fn oracle(
        resolution: OracleResolution,
        candidates: &[&str],
        zone_id: Option<&str>,
    ) -> ObservabilityEvidence {
        ObservabilityEvidence {
            resolution,
            candidate_pipe_ids: candidates.iter().map(|value| (*value).to_owned()).collect(),
            zone_id: zone_id.map(str::to_owned),
            evidence_digest: digest(500),
        }
    }

    fn prediction(
        ranked: &[&str],
        confidence: u32,
    ) -> (Vec<String>, bool, Option<u32>) {
        (
            ranked.iter().map(|value| (*value).to_owned()).collect(),
            false,
            Some(confidence),
        )
    }

    fn abstention() -> (Vec<String>, bool, Option<u32>) {
        (Vec::new(), true, None)
    }

    fn case(
        case_id: &str,
        resolution: OracleResolution,
        candidates: &[&str],
        zone_id: Option<&str>,
        truth: &str,
        ranking: (&[&str], bool, Option<u32>),
    ) -> LocalizationCase {
        LocalizationCase {
            case_id: case_id.to_owned(),
            network_id: "network-1".to_owned(),
            sensor_layout_id: "layout-1".to_owned(),
            network_pipe_ids: vec![
                "pipe-a".to_owned(),
                "pipe-b".to_owned(),
                "pipe-c".to_owned(),
                "pipe-d".to_owned(),
                "pipe-e".to_owned(),
            ],
            truth_pipe_id: truth.to_owned(),
            observability: oracle(resolution, candidates, zone_id),
            ranked_pipe_ids: ranking.0.iter().map(|value| (*value).to_owned()).collect(),
            abstained: ranking.1,
            top_k_confidence_ppm: ranking.2,
        }
    }

    fn config() -> LocalizationEvaluationConfig {
        LocalizationEvaluationConfig {
            top_k: 2,
            confidence_thresholds_ppm: vec![500_000, 750_000, 900_000],
        }
    }

    #[test]
    fn unique_pipe_case_has_full_oracle_feasibility() {
        let cases = vec![case(
            "unique",
            OracleResolution::UniquePipe,
            &["pipe-a"],
            None,
            "pipe-a",
            prediction(&["pipe-a", "pipe-b"], 950_000),
        )];
        let report = evaluate_localization(&cases, &config()).expect("valid case");
        assert_eq!(report.trusted_oracle_cases, 1);
        assert_eq!(report.oracle_top_k_feasibility_ppm, Some(1_000_000));
        assert_eq!(report.conditional_top_k_accuracy_ppm, Some(1_000_000));
        assert_eq!(report.system_top_k_accuracy_ppm, Some(1_000_000));
    }

    #[test]
    fn ambiguous_set_separates_physical_feasibility_from_model_skill() {
        let cases = vec![case(
            "ambiguous",
            OracleResolution::AmbiguousPipeSet,
            &["pipe-a", "pipe-b", "pipe-c"],
            None,
            "pipe-c",
            prediction(&["pipe-a", "pipe-d"], 650_000),
        )];
        let report = evaluate_localization(&cases, &config()).expect("valid ambiguous case");
        assert_eq!(report.ambiguous_pipe_set_cases, 1);
        assert_eq!(report.oracle_top_k_feasibility_ppm, Some(0));
        assert_eq!(report.conditional_top_k_accuracy_ppm, Some(0));
    }

    #[test]
    fn zone_only_is_scorable_but_not_misreported_as_unique_pipe() {
        let cases = vec![case(
            "zone-only",
            OracleResolution::ZoneOnly,
            &["pipe-b", "pipe-c"],
            Some("zone-west"),
            "pipe-c",
            prediction(&["pipe-c", "pipe-b"], 800_000),
        )];
        let report = evaluate_localization(&cases, &config()).expect("valid zone case");
        assert_eq!(report.zone_only_cases, 1);
        assert_eq!(report.oracle_top_k_feasibility_ppm, Some(1_000_000));
        assert_eq!(report.top_k_hits, 1);
    }

    #[test]
    fn event_only_cases_are_excluded_from_pipe_accuracy() {
        let cases = vec![case(
            "event-only",
            OracleResolution::EventOnly,
            &[],
            None,
            "pipe-a",
            prediction(&["pipe-a"], 990_000),
        )];
        let report = evaluate_localization(&cases, &config()).expect("event-only case");
        assert_eq!(report.event_only_cases, 1);
        assert_eq!(report.trusted_oracle_cases, 0);
        assert_eq!(report.unqualified_pipe_claims, 1);
        assert_eq!(report.system_top_k_accuracy_ppm, None);
    }

    #[test]
    fn abstention_on_unobservable_case_is_safe_not_a_false_zero() {
        let cases = vec![case(
            "not-measured",
            OracleResolution::NotMeasured,
            &[],
            None,
            "pipe-a",
            abstention(),
        )];
        let report = evaluate_localization(&cases, &config()).expect("not-measured abstention");
        assert_eq!(report.not_measured_cases, 1);
        assert_eq!(report.safe_abstentions, 1);
        assert_eq!(report.system_top_k_accuracy_ppm, None);
    }

    #[test]
    fn abstention_on_top_k_feasible_case_is_counted_as_unnecessary() {
        let cases = vec![case(
            "avoidable-abstention",
            OracleResolution::UniquePipe,
            &["pipe-a"],
            None,
            "pipe-a",
            abstention(),
        )];
        let report = evaluate_localization(&cases, &config()).expect("valid abstention");
        assert_eq!(report.unnecessary_abstentions, 1);
        assert_eq!(report.system_top_k_accuracy_ppm, Some(0));
        assert_eq!(report.conditional_top_k_accuracy_ppm, None);
    }

    #[test]
    fn model_uncertain_is_not_used_as_a_physical_observability_ceiling() {
        let cases = vec![case(
            "uncertain-model",
            OracleResolution::ModelUncertain,
            &["pipe-a", "pipe-b"],
            None,
            "pipe-a",
            abstention(),
        )];
        let report = evaluate_localization(&cases, &config()).expect("model uncertainty");
        assert_eq!(report.model_uncertain_cases, 1);
        assert_eq!(report.trusted_oracle_cases, 0);
        assert_eq!(report.safe_abstentions, 1);
    }

    #[test]
    fn risk_coverage_curve_filters_predictions_by_declared_confidence() {
        let cases = vec![
            case(
                "high-correct",
                OracleResolution::UniquePipe,
                &["pipe-a"],
                None,
                "pipe-a",
                prediction(&["pipe-a", "pipe-b"], 950_000),
            ),
            case(
                "medium-wrong",
                OracleResolution::UniquePipe,
                &["pipe-b"],
                None,
                "pipe-b",
                prediction(&["pipe-a", "pipe-c"], 700_000),
            ),
            case(
                "low-correct",
                OracleResolution::ZoneOnly,
                &["pipe-c", "pipe-d"],
                Some("zone-east"),
                "pipe-c",
                prediction(&["pipe-c", "pipe-d"], 550_000),
            ),
        ];
        let report = evaluate_localization(&cases, &config()).expect("valid curve");
        assert_eq!(report.risk_coverage[0].selected_count, 3);
        assert_eq!(report.risk_coverage[0].coverage_ppm, Some(1_000_000));
        assert_eq!(report.risk_coverage[0].top_k_accuracy_ppm, Some(666_666));
        assert_eq!(report.risk_coverage[1].selected_count, 1);
        assert_eq!(report.risk_coverage[1].coverage_ppm, Some(333_333));
        assert_eq!(report.risk_coverage[1].top_k_accuracy_ppm, Some(1_000_000));
        assert_eq!(report.risk_coverage[2].selected_count, 1);
        assert_eq!(report.risk_coverage[2].top_k_risk_ppm, Some(0));
    }

    #[test]
    fn input_order_does_not_change_summary_or_risk_curve() {
        let cases = vec![
            case(
                "a",
                OracleResolution::UniquePipe,
                &["pipe-a"],
                None,
                "pipe-a",
                prediction(&["pipe-a"], 900_000),
            ),
            case(
                "b",
                OracleResolution::AmbiguousPipeSet,
                &["pipe-b", "pipe-c"],
                None,
                "pipe-c",
                prediction(&["pipe-c", "pipe-b"], 800_000),
            ),
        ];
        let first = evaluate_localization(&cases, &config()).expect("valid corpus");
        let mut reversed = cases.clone();
        reversed.reverse();
        let second = evaluate_localization(&reversed, &config()).expect("permuted corpus");
        assert_eq!(first, second);
        assert_eq!(first.canonical_text(), second.canonical_text());
    }

    #[test]
    fn duplicate_case_ids_fail_closed() {
        let first = case(
            "duplicate",
            OracleResolution::UniquePipe,
            &["pipe-a"],
            None,
            "pipe-a",
            prediction(&["pipe-a"], 800_000),
        );
        let mut second = first.clone();
        second.truth_pipe_id = "pipe-b".to_owned();
        let error = evaluate_localization(&[first, second], &config()).expect_err("duplicate ID");
        assert_eq!(error.code, "duplicate_case_id");
    }

    #[test]
    fn unique_oracle_must_contain_only_the_true_pipe() {
        let cases = vec![case(
            "bad-unique",
            OracleResolution::UniquePipe,
            &["pipe-a", "pipe-b"],
            None,
            "pipe-a",
            prediction(&["pipe-a"], 800_000),
        )];
        let error = evaluate_localization(&cases, &config()).expect_err("invalid unique oracle");
        assert_eq!(error.code, "unique_pipe_oracle_inconsistent");
    }

    #[test]
    fn trusted_oracle_candidate_set_must_contain_truth() {
        let cases = vec![case(
            "bad-ambiguity",
            OracleResolution::AmbiguousPipeSet,
            &["pipe-a", "pipe-b"],
            None,
            "pipe-c",
            prediction(&["pipe-a"], 800_000),
        )];
        let error = evaluate_localization(&cases, &config()).expect_err("truth omitted");
        assert_eq!(error.code, "ambiguous_pipe_oracle_inconsistent");
    }

    #[test]
    fn oracle_and_prediction_ids_must_belong_to_network_universe() {
        let mut cases = vec![case(
            "outside",
            OracleResolution::UniquePipe,
            &["pipe-a"],
            None,
            "pipe-a",
            prediction(&["pipe-x"], 800_000),
        )];
        let error = evaluate_localization(&cases, &config()).expect_err("predicted pipe outside");
        assert_eq!(error.code, "predicted_pipe_outside_network");

        cases[0].ranked_pipe_ids = vec!["pipe-a".to_owned()];
        cases[0].observability.candidate_pipe_ids = vec!["pipe-x".to_owned()];
        let error = evaluate_localization(&cases, &config()).expect_err("oracle pipe outside");
        assert_eq!(error.code, "oracle_pipe_outside_network");
    }

    #[test]
    fn abstention_cannot_carry_a_ranked_list_or_confidence() {
        let mut cases = vec![case(
            "bad-abstention",
            OracleResolution::UniquePipe,
            &["pipe-a"],
            None,
            "pipe-a",
            abstention(),
        )];
        cases[0].ranked_pipe_ids = vec!["pipe-a".to_owned()];
        let error = evaluate_localization(&cases, &config()).expect_err("abstention payload");
        assert_eq!(error.code, "abstention_carries_prediction");
    }

    #[test]
    fn non_abstention_requires_a_ranked_list_and_confidence() {
        let mut cases = vec![case(
            "missing-prediction",
            OracleResolution::UniquePipe,
            &["pipe-a"],
            None,
            "pipe-a",
            prediction(&["pipe-a"], 800_000),
        )];
        cases[0].ranked_pipe_ids.clear();
        let error = evaluate_localization(&cases, &config()).expect_err("empty ranking");
        assert_eq!(error.code, "empty_prediction_without_abstention");

        cases[0].ranked_pipe_ids = vec!["pipe-a".to_owned()];
        cases[0].top_k_confidence_ppm = None;
        let error = evaluate_localization(&cases, &config()).expect_err("missing confidence");
        assert_eq!(error.code, "missing_top_k_confidence");
    }

    #[test]
    fn invalid_digests_confidence_and_thresholds_fail_closed() {
        let mut cases = vec![case(
            "bad-digest",
            OracleResolution::UniquePipe,
            &["pipe-a"],
            None,
            "pipe-a",
            prediction(&["pipe-a"], 800_000),
        )];
        cases[0].observability.evidence_digest = "sha256:00".to_owned();
        assert_eq!(
            evaluate_localization(&cases, &config())
                .expect_err("bad digest")
                .code,
            "invalid_observability_digest"
        );

        cases[0].observability.evidence_digest = digest(500);
        cases[0].top_k_confidence_ppm = Some(1_000_001);
        assert_eq!(
            evaluate_localization(&cases, &config())
                .expect_err("confidence out of range")
                .code,
            "confidence_out_of_range"
        );

        let invalid_config = LocalizationEvaluationConfig {
            top_k: 2,
            confidence_thresholds_ppm: vec![800_000, 800_000],
        };
        cases[0].top_k_confidence_ppm = Some(800_000);
        assert_eq!(
            evaluate_localization(&cases, &invalid_config)
                .expect_err("duplicate threshold")
                .code,
            "invalid_confidence_thresholds"
        );
    }

    #[test]
    fn zero_k_and_empty_corpus_fail_closed() {
        let invalid = LocalizationEvaluationConfig {
            top_k: 0,
            confidence_thresholds_ppm: vec![],
        };
        let cases = vec![case(
            "one",
            OracleResolution::UniquePipe,
            &["pipe-a"],
            None,
            "pipe-a",
            prediction(&["pipe-a"], 900_000),
        )];
        assert_eq!(
            evaluate_localization(&cases, &invalid)
                .expect_err("zero K")
                .code,
            "invalid_top_k"
        );
        assert_eq!(
            evaluate_localization(&[], &config())
                .expect_err("empty corpus")
                .code,
            "empty_localization_corpus"
        );
    }

    #[test]
    fn unsupported_and_not_measured_statuses_remain_separate() {
        let cases = vec![
            case(
                "unsupported",
                OracleResolution::Unsupported,
                &[],
                None,
                "pipe-a",
                abstention(),
            ),
            case(
                "not-measured",
                OracleResolution::NotMeasured,
                &[],
                None,
                "pipe-b",
                abstention(),
            ),
        ];
        let report = evaluate_localization(&cases, &config()).expect("valid unscorable cases");
        assert_eq!(report.unsupported_cases, 1);
        assert_eq!(report.not_measured_cases, 1);
        assert_eq!(report.trusted_oracle_cases, 0);
        assert_eq!(report.safe_abstentions, 2);
    }
}
