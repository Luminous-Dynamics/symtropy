// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Observability-qualified hierarchical localization evaluation.
//!
//! The hydraulic owner supplies one independently versioned observability
//! assessment per event. This evaluator does not calculate hydraulic
//! sensitivity, observability, sensor placement, or pipe equivalence classes.
//! It checks that a prediction does not claim a finer localization level than
//! the supplied physical assessment supports, and scores zone and pipe
//! predictions separately.
//!
//! Observed, derived, and modelled truth labels are scored in separate slices.
//! Not-measured, unsupported, withheld, and not-applicable truth remain
//! explicitly unscorable. Confidence is expressed in integer parts-per-million
//! to keep receipts deterministic and to avoid floating-point ambiguity.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

/// Finest localization granularity supported by the physical observability
/// assessment supplied by the hydraulic owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LocalizationLevel {
    /// Event detection is possible, but no spatial localization is supported.
    DetectionOnly,
    /// Localization to a hydraulic zone is supported.
    Zone,
    /// Localization to a pipe/asset is supported.
    Pipe,
}

impl LocalizationLevel {
    /// Stable machine-readable name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DetectionOnly => "detection_only",
            Self::Zone => "zone",
            Self::Pipe => "pipe",
        }
    }

    fn supports(self, target: Self) -> bool {
        self >= target && target != Self::DetectionOnly
    }
}

/// Evidence class attached to a known ground-truth location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TruthEvidenceClass {
    /// Independently observed and confirmed location.
    Observed,
    /// Reproducibly derived from qualified source evidence.
    Derived,
    /// Generated or inferred by a declared model/simulator.
    Modelled,
}

impl TruthEvidenceClass {
    /// Stable machine-readable name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Observed => "observed",
            Self::Derived => "derived",
            Self::Modelled => "modelled",
        }
    }
}

/// Ground-truth label with explicit epistemic status. Unknown states never
/// acquire a placeholder identifier or a numeric zero.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TruthLabel {
    /// A location identity supported by the stated evidence class.
    Known {
        /// Zone or pipe/asset identifier.
        value: String,
        /// Evidence class for this identity.
        evidence: TruthEvidenceClass,
    },
    /// No qualified location measurement exists.
    NotMeasured,
    /// The source cannot support this localization claim.
    Unsupported,
    /// Evidence exists but is withheld from this evaluation.
    Withheld,
    /// The label does not apply to this event.
    NotApplicable,
}

impl TruthLabel {
    fn known_value(&self) -> Option<&str> {
        match self {
            Self::Known { value, .. } => Some(value),
            Self::NotMeasured | Self::Unsupported | Self::Withheld | Self::NotApplicable => None,
        }
    }

    fn evidence_class(&self) -> Option<TruthEvidenceClass> {
        match self {
            Self::Known { evidence, .. } => Some(*evidence),
            Self::NotMeasured | Self::Unsupported | Self::Withheld | Self::NotApplicable => None,
        }
    }
}

/// Independently sourced localization truth for one event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalizationTruth {
    /// Event identity used by the event evaluator.
    pub event_id: String,
    /// True hydraulic zone, when known.
    pub zone: TruthLabel,
    /// True pipe/asset, when known.
    pub pipe: TruthLabel,
}

/// Physical observability evidence supplied by the hydraulic/model owner.
///
/// Candidate universes are the identities that the supplied physical
/// assessment considers distinguishable enough for this protocol. The
/// evaluator validates membership and scores predictions against ground truth;
/// it does not derive these candidate universes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservabilityAssessment {
    /// Event identity shared with truth and prediction records.
    pub event_id: String,
    /// Finest level the physical assessment supports.
    pub maximum_level: LocalizationLevel,
    /// Candidate zones supported by the physical assessment.
    pub zone_candidate_ids: Vec<String>,
    /// Candidate pipes/assets supported by the physical assessment.
    pub pipe_candidate_ids: Vec<String>,
    /// Sensor layout/version reference.
    pub sensor_layout_ref: String,
    /// Hydraulic model/calibration version reference.
    pub hydraulic_model_ref: String,
    /// Caller-computed SHA-256 digest of the canonical assessment payload.
    ///
    /// Syntax is validated, but bytes are not available here for recomputation.
    pub assessment_digest: String,
}

/// Why a policy abstained from spatial localization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbstentionReason {
    /// Physical observability did not support a finer claim.
    NotIdentifiable,
    /// Required sensor channels or labels were unavailable.
    MissingEvidence,
    /// Confidence did not meet the declared policy threshold.
    LowConfidence,
    /// The input was outside the registered model domain.
    OutOfDistribution,
    /// The policy deferred to a human reviewer.
    HumanReview,
}

impl AbstentionReason {
    /// Stable machine-readable name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotIdentifiable => "not_identifiable",
            Self::MissingEvidence => "missing_evidence",
            Self::LowConfidence => "low_confidence",
            Self::OutOfDistribution => "out_of_distribution",
            Self::HumanReview => "human_review",
        }
    }
}

/// One model-produced localization decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalizationDecision {
    /// Explicit abstention; this is not scored as a false pipe/zone label.
    Abstain {
        /// Declared reason for abstention.
        reason: AbstentionReason,
    },
    /// Ranked candidates at one declared spatial level.
    Ranked {
        /// Granularity claimed by the prediction.
        level: LocalizationLevel,
        /// Candidate IDs in descending model rank; no duplicates allowed.
        candidate_ids: Vec<String>,
        /// Confidence that the first-ranked candidate is correct, in ppm.
        confidence_ppm: u32,
    },
}

/// One prediction bound to a single event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalizationPrediction {
    /// Event identity shared with truth and observability evidence.
    pub event_id: String,
    /// Decision emitted by the localization policy.
    pub decision: LocalizationDecision,
}

/// One risk-coverage point. Candidates are admitted in descending confidence
/// order; equal-confidence predictions enter as one indivisible threshold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RiskCoveragePoint {
    /// Minimum top-1 confidence among predictions included in this point.
    pub minimum_confidence_ppm: u32,
    /// Number of supported predictions included.
    pub covered_events: usize,
    /// Covered predictions divided by all eligible events; None if undefined.
    pub coverage_ppm: Option<u32>,
    /// Number of incorrect top-1 predictions among covered events.
    pub top1_errors: usize,
    /// Top-1 errors divided by covered events; None if no events are covered.
    pub risk_ppm: Option<u32>,
}

/// Results for a single truth-evidence class at one localization level.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LocalizationEvidenceMetrics {
    /// Events with a known label and a sufficient observability ceiling.
    pub eligible_events: usize,
    /// Supported predictions at exactly this granularity.
    pub localized_events: usize,
    /// Explicit abstentions among eligible events.
    pub abstained_events: usize,
    /// Supported but coarser predictions (relevant to pipe-level scoring).
    pub coarse_predictions: usize,
    /// Supported but finer predictions (relevant to zone-level scoring).
    pub more_specific_predictions: usize,
    /// Predictions beyond the supplied physical observability ceiling.
    pub unsupported_granularity_predictions: usize,
    /// Correct top-1 predictions.
    pub top1_hits: usize,
    /// Correct top-5 predictions.
    pub top5_hits: usize,
    /// Valid localized predictions divided by eligible events.
    pub coverage_ppm: Option<u32>,
    /// Top-1 hits divided by valid localized predictions.
    pub conditional_top1_accuracy_ppm: Option<u32>,
    /// Top-5 hits divided by valid localized predictions.
    pub conditional_top5_accuracy_ppm: Option<u32>,
    /// Top-1 hits divided by all eligible events.
    pub system_top1_accuracy_ppm: Option<u32>,
    /// Top-5 hits divided by all eligible events.
    pub system_top5_accuracy_ppm: Option<u32>,
    /// Confidence-threshold risk/coverage curve.
    pub risk_coverage: Vec<RiskCoveragePoint>,
}

/// Separate evidence-class metrics and explicit unscorable counts for one
/// spatial level.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LocalizationLevelReport {
    /// Results using directly observed truth.
    pub observed: LocalizationEvidenceMetrics,
    /// Results using derived truth; never pooled into observed metrics.
    pub derived: LocalizationEvidenceMetrics,
    /// Results using modelled truth; never pooled into observed metrics.
    pub modelled: LocalizationEvidenceMetrics,
    /// Truth labels explicitly not measured.
    pub not_measured_events: usize,
    /// Truth labels explicitly unsupported.
    pub unsupported_truth_events: usize,
    /// Truth labels withheld from this evaluation.
    pub withheld_truth_events: usize,
    /// Truth labels for which this level does not apply.
    pub not_applicable_events: usize,
    /// Known truth labels for which physical observability is insufficient.
    pub observability_limited_events: usize,
}

impl LocalizationLevelReport {
    fn evidence_mut(&mut self, evidence: TruthEvidenceClass) -> &mut LocalizationEvidenceMetrics {
        match evidence {
            TruthEvidenceClass::Observed => &mut self.observed,
            TruthEvidenceClass::Derived => &mut self.derived,
            TruthEvidenceClass::Modelled => &mut self.modelled,
        }
    }

    /// Total number of events whose ground-truth label is known at this level.
    pub fn known_truth_events(&self) -> usize {
        self.observed.eligible_events
            + self.derived.eligible_events
            + self.modelled.eligible_events
            + self.observability_limited_events
    }
}

/// Counts of events by their physical observability ceiling.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ObservabilityCounts {
    /// Events where only event detection is supported.
    pub detection_only: usize,
    /// Events where zone-level localization is supported.
    pub zone: usize,
    /// Events where pipe-level localization is supported.
    pub pipe: usize,
}

/// Full hierarchical localization evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalizationEvaluationReport {
    /// Number of truth events in the evaluation.
    pub event_count: usize,
    /// Event counts by the supplied physical observability ceiling.
    pub observability: ObservabilityCounts,
    /// Zone-level metrics.
    pub zone: LocalizationLevelReport,
    /// Pipe-level metrics.
    pub pipe: LocalizationLevelReport,
    /// Total predictions whose granularity exceeds the physical ceiling.
    pub unsupported_granularity_predictions: usize,
}

impl LocalizationEvaluationReport {
    /// Deterministic line-oriented receipt. This is not a signature or
    /// authentication of the hydraulic owner or its assessment payloads.
    pub fn canonical_text(&self) -> String {
        let mut output = String::new();
        let _ = writeln!(&mut output, "CIVBENCH_LOCALIZATION_EVALUATION_V0.1");
        let _ = writeln!(&mut output, "event_count={}", self.event_count);
        let _ = writeln!(
            &mut output,
            "observability=detection_only,{}",
            self.observability.detection_only
        );
        let _ = writeln!(&mut output, "observability=zone,{}", self.observability.zone);
        let _ = writeln!(&mut output, "observability=pipe,{}", self.observability.pipe);
        let _ = writeln!(
            &mut output,
            "unsupported_granularity_predictions={}",
            self.unsupported_granularity_predictions
        );
        append_level_report(&mut output, "zone", &self.zone);
        append_level_report(&mut output, "pipe", &self.pipe);
        output
    }
}

fn append_level_report(
    output: &mut String,
    level: &str,
    report: &LocalizationLevelReport,
) {
    for (class, metrics) in [
        ("observed", &report.observed),
        ("derived", &report.derived),
        ("modelled", &report.modelled),
    ] {
        let _ = writeln!(
            output,
            "level={level},truth={class},eligible={},localized={},abstained={},coarse={},more_specific={},unsupported={},top1_hits={},top5_hits={},coverage_ppm={},conditional_top1_ppm={},conditional_top5_ppm={},system_top1_ppm={},system_top5_ppm={}",
            metrics.eligible_events,
            metrics.localized_events,
            metrics.abstained_events,
            metrics.coarse_predictions,
            metrics.more_specific_predictions,
            metrics.unsupported_granularity_predictions,
            metrics.top1_hits,
            metrics.top5_hits,
            option_ppm(metrics.coverage_ppm),
            option_ppm(metrics.conditional_top1_accuracy_ppm),
            option_ppm(metrics.conditional_top5_accuracy_ppm),
            option_ppm(metrics.system_top1_accuracy_ppm),
            option_ppm(metrics.system_top5_accuracy_ppm),
        );
        for point in &metrics.risk_coverage {
            let _ = writeln!(
                output,
                "risk_coverage={level},{class},threshold={},covered={},coverage_ppm={},errors={},risk_ppm={}",
                point.minimum_confidence_ppm,
                point.covered_events,
                option_ppm(point.coverage_ppm),
                point.top1_errors,
                option_ppm(point.risk_ppm)
            );
        }
    }
    let _ = writeln!(
        output,
        "level={level},unscorable=not_measured,{}",
        report.not_measured_events
    );
    let _ = writeln!(
        output,
        "level={level},unscorable=unsupported,{}",
        report.unsupported_truth_events
    );
    let _ = writeln!(
        output,
        "level={level},unscorable=withheld,{}",
        report.withheld_truth_events
    );
    let _ = writeln!(
        output,
        "level={level},unscorable=not_applicable,{}",
        report.not_applicable_events
    );
    let _ = writeln!(
        output,
        "level={level},observability_limited={}",
        report.observability_limited_events
    );
}

fn option_ppm(value: Option<u32>) -> String {
    value.map_or_else(|| "none".to_owned(), |number| number.to_string())
}

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

fn validate_candidate_ids(
    ids: &[String],
    subject: &str,
) -> Result<BTreeSet<String>, LocalizationEvaluationError> {
    let mut unique = BTreeSet::new();
    for id in ids {
        if !valid_identifier(id) {
            return Err(LocalizationEvaluationError::new(
                "invalid_candidate_id",
                format!("{subject}:{id}"),
            ));
        }
        if !unique.insert(id.clone()) {
            return Err(LocalizationEvaluationError::new(
                "duplicate_candidate_id",
                format!("{subject}:{id}"),
            ));
        }
    }
    Ok(unique)
}

fn validate_assessment(
    assessment: &ObservabilityAssessment,
) -> Result<(), LocalizationEvaluationError> {
    if !valid_identifier(&assessment.event_id)
        || !valid_identifier(&assessment.sensor_layout_ref)
        || !valid_identifier(&assessment.hydraulic_model_ref)
    {
        return Err(LocalizationEvaluationError::new(
            "invalid_observability_reference",
            assessment.event_id.as_str(),
        ));
    }
    if normalized_sha256(&assessment.assessment_digest).is_none() {
        return Err(LocalizationEvaluationError::new(
            "invalid_observability_digest",
            assessment.event_id.as_str(),
        ));
    }
    let zones = validate_candidate_ids(
        &assessment.zone_candidate_ids,
        &format!("{}:zone", assessment.event_id),
    )?;
    let pipes = validate_candidate_ids(
        &assessment.pipe_candidate_ids,
        &format!("{}:pipe", assessment.event_id),
    )?;
    let valid_universe = match assessment.maximum_level {
        LocalizationLevel::DetectionOnly => zones.is_empty() && pipes.is_empty(),
        LocalizationLevel::Zone => !zones.is_empty() && pipes.is_empty(),
        LocalizationLevel::Pipe => !zones.is_empty() && !pipes.is_empty(),
    };
    if !valid_universe {
        return Err(LocalizationEvaluationError::new(
            "observability_candidate_universe_mismatch",
            assessment.event_id.as_str(),
        ));
    }
    Ok(())
}

fn validate_truth(truth: &LocalizationTruth) -> Result<(), LocalizationEvaluationError> {
    if !valid_identifier(&truth.event_id) {
        return Err(LocalizationEvaluationError::new(
            "invalid_event_id",
            truth.event_id.as_str(),
        ));
    }
    for (level, label) in [("zone", &truth.zone), ("pipe", &truth.pipe)] {
        if let TruthLabel::Known { value, .. } = label {
            if !valid_identifier(value) {
                return Err(LocalizationEvaluationError::new(
                    "invalid_truth_identifier",
                    format!("{}:{level}", truth.event_id),
                ));
            }
        }
    }
    Ok(())
}

fn validate_prediction(
    prediction: &LocalizationPrediction,
) -> Result<(), LocalizationEvaluationError> {
    if !valid_identifier(&prediction.event_id) {
        return Err(LocalizationEvaluationError::new(
            "invalid_event_id",
            prediction.event_id.as_str(),
        ));
    }
    if let LocalizationDecision::Ranked {
        candidate_ids,
        confidence_ppm,
        ..
    } = &prediction.decision
    {
        if candidate_ids.is_empty() {
            return Err(LocalizationEvaluationError::new(
                "empty_candidate_ranking",
                prediction.event_id.as_str(),
            ));
        }
        validate_candidate_ids(candidate_ids, &prediction.event_id)?;
        if *confidence_ppm > 1_000_000 {
            return Err(LocalizationEvaluationError::new(
                "confidence_out_of_range",
                prediction.event_id.as_str(),
            ));
        }
    }
    Ok(())
}

fn validate_truth_in_candidate_universe(
    truth: &LocalizationTruth,
    assessment: &ObservabilityAssessment,
) -> Result<(), LocalizationEvaluationError> {
    if assessment.maximum_level.supports(LocalizationLevel::Zone) {
        if let Some(zone) = truth.zone.known_value() {
            if !assessment.zone_candidate_ids.iter().any(|candidate| candidate == zone) {
                return Err(LocalizationEvaluationError::new(
                    "truth_zone_outside_observable_universe",
                    truth.event_id.as_str(),
                ));
            }
        }
    }
    if assessment.maximum_level.supports(LocalizationLevel::Pipe) {
        if let Some(pipe) = truth.pipe.known_value() {
            if !assessment.pipe_candidate_ids.iter().any(|candidate| candidate == pipe) {
                return Err(LocalizationEvaluationError::new(
                    "truth_pipe_outside_observable_universe",
                    truth.event_id.as_str(),
                ));
            }
        }
    }
    Ok(())
}

fn validate_prediction_in_candidate_universe(
    prediction: &LocalizationPrediction,
    assessment: &ObservabilityAssessment,
) -> Result<(), LocalizationEvaluationError> {
    let LocalizationDecision::Ranked {
        level,
        candidate_ids,
        ..
    } = &prediction.decision
    else {
        return Ok(());
    };
    if *level > assessment.maximum_level {
        return Ok(());
    }
    let universe = match level {
        LocalizationLevel::DetectionOnly => {
            return Err(LocalizationEvaluationError::new(
                "invalid_prediction_level",
                prediction.event_id.as_str(),
            ));
        }
        LocalizationLevel::Zone => &assessment.zone_candidate_ids,
        LocalizationLevel::Pipe => &assessment.pipe_candidate_ids,
    };
    if candidate_ids
        .iter()
        .any(|candidate| !universe.iter().any(|allowed| allowed == candidate))
    {
        return Err(LocalizationEvaluationError::new(
            "prediction_outside_observable_universe",
            prediction.event_id.as_str(),
        ));
    }
    Ok(())
}

/// Input validation or cross-reference failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalizationEvaluationError {
    /// Stable machine-readable failure category.
    pub code: &'static str,
    /// Event, level, or offending reference.
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

fn ratio_ppm(numerator: usize, denominator: usize) -> Option<u32> {
    if denominator == 0 {
        return None;
    }
    Some(
        (numerator as u128 * 1_000_000_u128 / denominator as u128)
            .min(1_000_000) as u32,
    )
}

fn evidence_slice_mut(
    report: &mut LocalizationLevelReport,
    evidence: TruthEvidenceClass,
) -> &mut LocalizationEvidenceMetrics {
    report.evidence_mut(evidence)
}

fn score_label(
    report: &mut LocalizationLevelReport,
    target: LocalizationLevel,
    label: &TruthLabel,
    prediction: &LocalizationDecision,
    maximum_level: LocalizationLevel,
    event_id: &str,
) -> Result<(), LocalizationEvaluationError> {
    let TruthLabel::Known { value, evidence } = label else {
        match label {
            TruthLabel::NotMeasured => report.not_measured_events += 1,
            TruthLabel::Unsupported => report.unsupported_truth_events += 1,
            TruthLabel::Withheld => report.withheld_truth_events += 1,
            TruthLabel::NotApplicable => report.not_applicable_events += 1,
            TruthLabel::Known { .. } => unreachable!(),
        }
        return Ok(());
    };

    if !maximum_level.supports(target) {
        report.observability_limited_events += 1;
        return Ok(());
    }

    let metrics = evidence_slice_mut(report, *evidence);
    metrics.eligible_events += 1;

    match prediction {
        LocalizationDecision::Abstain { .. } => {
            metrics.abstained_events += 1;
        }
        LocalizationDecision::Ranked {
            level,
            candidate_ids,
            confidence_ppm,
        } => {
            if *level > maximum_level {
                metrics.unsupported_granularity_predictions += 1;
                return Ok(());
            }
            if *level < target {
                metrics.coarse_predictions += 1;
                return Ok(());
            }
            if *level > target {
                metrics.more_specific_predictions += 1;
                return Ok(());
            }
            let top1 = candidate_ids.first().is_some_and(|candidate| candidate == value);
            let top5 = candidate_ids
                .iter()
                .take(5)
                .any(|candidate| candidate == value);
            metrics.localized_events += 1;
            metrics.top1_hits += usize::from(top1);
            metrics.top5_hits += usize::from(top5);
            metrics
                .risk_coverage
                .push(RiskCoveragePointSeed::new(*confidence_ppm, top1).into_point_seed());
        }
    }
    let _ = event_id;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RiskCoveragePointSeed {
    confidence_ppm: u32,
    top1_hit: bool,
}

impl RiskCoveragePointSeed {
    fn new(confidence_ppm: u32, top1_hit: bool) -> Self {
        Self {
            confidence_ppm,
            top1_hit,
        }
    }

    fn into_point_seed(self) -> RiskCoveragePoint {
        RiskCoveragePoint {
            minimum_confidence_ppm: self.confidence_ppm,
            covered_events: usize::from(self.top1_hit),
            coverage_ppm: None,
            top1_errors: usize::from(!self.top1_hit),
            risk_ppm: None,
        }
    }
}
