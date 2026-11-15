//! Non-compensatory megastructure design-review gate.
//!
//! This typed reference checks review completeness and hard-gate precedence.
//! It does not rank architectural taste, infer community consent, adjudicate
//! rights, certify physical safety, or calculate a universal beauty score.
//!
//! The evaluator receives the evaluation date as input. It does not consult
//! wall-clock time, so expiry decisions are deterministic and replayable.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardGate {
    StructuralSafety,
    OperationalSafety,
    Accessibility,
    Maintainability,
    Rights,
    Environment,
    Ecology,
}

impl HardGate {
    pub const ALL: [Self; 7] = [
        Self::StructuralSafety,
        Self::OperationalSafety,
        Self::Accessibility,
        Self::Maintainability,
        Self::Rights,
        Self::Environment,
        Self::Ecology,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::StructuralSafety => "structural_safety",
            Self::OperationalSafety => "operational_safety",
            Self::Accessibility => "accessibility",
            Self::Maintainability => "maintainability",
            Self::Rights => "rights",
            Self::Environment => "environment",
            Self::Ecology => "ecology",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateStatus {
    Pass,
    Fail,
    Unknown,
    NotApplicable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewDimension {
    FormalCoherenceLegibility,
    HumanExperienceDignity,
    MaterialHonestyRepairability,
    EcologicalPlanetaryOrbitalRelationship,
    PluralismPlaceCulturalAuthorship,
}

impl ReviewDimension {
    pub const ALL: [Self; 5] = [
        Self::FormalCoherenceLegibility,
        Self::HumanExperienceDignity,
        Self::MaterialHonestyRepairability,
        Self::EcologicalPlanetaryOrbitalRelationship,
        Self::PluralismPlaceCulturalAuthorship,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FormalCoherenceLegibility => "formal_coherence_legibility",
            Self::HumanExperienceDignity => "human_experience_dignity",
            Self::MaterialHonestyRepairability => "material_honesty_repairability",
            Self::EcologicalPlanetaryOrbitalRelationship => {
                "ecological_planetary_orbital_relationship"
            }
            Self::PluralismPlaceCulturalAuthorship => "pluralism_place_cultural_authorship",
        }
    }

    const fn index(self) -> usize {
        match self {
            Self::FormalCoherenceLegibility => 0,
            Self::HumanExperienceDignity => 1,
            Self::MaterialHonestyRepairability => 2,
            Self::EcologicalPlanetaryOrbitalRelationship => 3,
            Self::PluralismPlaceCulturalAuthorship => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewStatus {
    Complete,
    Incomplete,
    Deferred,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhysicalDeltas {
    pub mass_kg: f64,
    pub power_kw: f64,
    pub thermal_kw: f64,
    pub maintenance_time_pct: f64,
}

impl PhysicalDeltas {
    fn is_finite(self) -> bool {
        self.mass_kg.is_finite()
            && self.power_kw.is_finite()
            && self.thermal_kw.is_finite()
            && self.maintenance_time_pct.is_finite()
    }

    fn has_nonzero_delta(self) -> bool {
        self.mass_kg != 0.0
            || self.power_kw != 0.0
            || self.thermal_kw != 0.0
            || self.maintenance_time_pct != 0.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deferment {
    pub reason: String,
    pub expires_on: String,
    pub improvement_owner: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReviewEnvelope {
    pub candidate_id: String,
    pub design_generation: String,
    pub hard_gates: [GateStatus; 7],
    pub reviewed_dimensions: [bool; 5],
    pub review_status: ReviewStatus,
    pub views_complete: bool,
    pub stakeholder_group_count: usize,
    /// True means the record explicitly preserves dissent or records that
    /// none was expressed; an absent dissent record is not silently accepted.
    pub dissent_handling_recorded: bool,
    pub physical_deltas: PhysicalDeltas,
    /// Required when a non-zero physical/lifecycle delta has been assessed.
    pub trade_study_evidence_id: Option<String>,
    /// ISO-8601 calendar date supplied by the caller; no wall clock is used.
    pub evaluation_date: String,
    pub deferment: Option<Deferment>,
}

impl ReviewEnvelope {
    pub fn dimension_reviewed(&self, dimension: ReviewDimension) -> bool {
        self.reviewed_dimensions[dimension.index()]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    InvalidEnvelope,
    BlockedHardGate,
    DesignReviewIncomplete,
    TradeStudyRequired,
    TimeLimitedDeferment,
    AdmissibleForMulticriteriaReview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReasonCode {
    InvalidCandidateIdentity,
    InvalidDesignGeneration,
    InvalidEvaluationDate,
    NonFinitePhysicalDelta,
    HardGateNotPass,
    UnexpectedDeferment,
    MissingDeferment,
    MissingDefermentReason,
    MissingDefermentOwner,
    InvalidDefermentExpiry,
    ExpiredDeferment,
    MinimumHumanDignityReviewMissing,
    RequiredViewsMissing,
    RequiredReviewDimensionMissing,
    StakeholderCoverageMissing,
    DissentHandlingMissing,
    LifecycleTradeStudyRequired,
}

impl ReasonCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidCandidateIdentity => "INVALID_CANDIDATE_IDENTITY",
            Self::InvalidDesignGeneration => "INVALID_DESIGN_GENERATION",
            Self::InvalidEvaluationDate => "INVALID_EVALUATION_DATE",
            Self::NonFinitePhysicalDelta => "NONFINITE_PHYSICAL_DELTA",
            Self::HardGateNotPass => "HARD_GATE_NOT_PASS",
            Self::UnexpectedDeferment => "UNEXPECTED_DEFERMENT",
            Self::MissingDeferment => "MISSING_DEFERMENT",
            Self::MissingDefermentReason => "MISSING_DEFERMENT_REASON",
            Self::MissingDefermentOwner => "MISSING_DEFERMENT_OWNER",
            Self::InvalidDefermentExpiry => "INVALID_DEFERMENT_EXPIRY",
            Self::ExpiredDeferment => "EXPIRED_DEFERMENT",
            Self::MinimumHumanDignityReviewMissing => "MINIMUM_HUMAN_DIGNITY_REVIEW_MISSING",
            Self::RequiredViewsMissing => "REQUIRED_VIEWS_MISSING",
            Self::RequiredReviewDimensionMissing => "REQUIRED_REVIEW_DIMENSION_MISSING",
            Self::StakeholderCoverageMissing => "STAKEHOLDER_COVERAGE_MISSING",
            Self::DissentHandlingMissing => "DISSENT_HANDLING_MISSING",
            Self::LifecycleTradeStudyRequired => "LIFECYCLE_TRADE_STUDY_REQUIRED",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GateFinding {
    pub gate: HardGate,
    pub status: GateStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewDecision {
    pub disposition: Disposition,
    pub reason_codes: Vec<ReasonCode>,
    pub gate_findings: Vec<GateFinding>,
}

fn valid_token(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'
        })
}

fn parse_date(value: &str) -> Option<(u32, u32, u32)> {
    let mut parts = value.split('-');
    let year = parts.next()?.parse::<u32>().ok()?;
    let month = parts.next()?.parse::<u32>().ok()?;
    let day = parts.next()?.parse::<u32>().ok()?;
    if parts.next().is_some() || year == 0 || !(1..=12).contains(&month) {
        return None;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return None,
    };
    if day == 0 || day > max_day {
        return None;
    }
    Some((year, month, day))
}

fn decision(
    disposition: Disposition,
    reason_codes: Vec<ReasonCode>,
    gate_findings: Vec<GateFinding>,
) -> ReviewDecision {
    ReviewDecision {
        disposition,
        reason_codes,
        gate_findings,
    }
}

fn review_incomplete(
    mut reasons: Vec<ReasonCode>,
    gate_findings: Vec<GateFinding>,
) -> ReviewDecision {
    reasons.sort_by_key(|reason| reason.as_str());
    reasons.dedup();
    decision(
        Disposition::DesignReviewIncomplete,
        reasons,
        gate_findings,
    )
}

/// Evaluate hard constraints first, then review completeness and lifecycle
/// deltas. No weighted sum, aesthetic ranking or preference score exists.
pub fn evaluate(envelope: &ReviewEnvelope) -> ReviewDecision {
    let mut invalid_reasons = Vec::new();
    if !valid_token(&envelope.candidate_id) {
        invalid_reasons.push(ReasonCode::InvalidCandidateIdentity);
    }
    if !valid_token(&envelope.design_generation) {
        invalid_reasons.push(ReasonCode::InvalidDesignGeneration);
    }
    let evaluation_date = parse_date(&envelope.evaluation_date);
    if evaluation_date.is_none() {
        invalid_reasons.push(ReasonCode::InvalidEvaluationDate);
    }
    if !envelope.physical_deltas.is_finite() {
        invalid_reasons.push(ReasonCode::NonFinitePhysicalDelta);
    }
    if !invalid_reasons.is_empty() {
        invalid_reasons.sort_by_key(|reason| reason.as_str());
        return decision(Disposition::InvalidEnvelope, invalid_reasons, Vec::new());
    }
    let evaluation_date = evaluation_date.expect("validated above");

    let mut gate_findings = Vec::new();
    for (gate, status) in HardGate::ALL.into_iter().zip(envelope.hard_gates) {
        if status != GateStatus::Pass {
            gate_findings.push(GateFinding { gate, status });
        }
    }
    if !gate_findings.is_empty() {
        return decision(
            Disposition::BlockedHardGate,
            vec![ReasonCode::HardGateNotPass],
            gate_findings,
        );
    }

    if envelope.review_status != ReviewStatus::Deferred && envelope.deferment.is_some() {
        return decision(
            Disposition::InvalidEnvelope,
            vec![ReasonCode::UnexpectedDeferment],
            gate_findings,
        );
    }

    if envelope.review_status == ReviewStatus::Deferred {
        let Some(deferment) = envelope.deferment.as_ref() else {
            return review_incomplete(vec![ReasonCode::MissingDeferment], gate_findings);
        };

        let mut reasons = Vec::new();
        if deferment.reason.trim().is_empty() {
            reasons.push(ReasonCode::MissingDefermentReason);
        }
        if deferment.improvement_owner.trim().is_empty() {
            reasons.push(ReasonCode::MissingDefermentOwner);
        }
        match parse_date(&deferment.expires_on) {
            None => reasons.push(ReasonCode::InvalidDefermentExpiry),
            Some(expiry) if expiry <= evaluation_date => {
                reasons.push(ReasonCode::ExpiredDeferment);
            }
            Some(_) => {}
        }
        if !envelope.dimension_reviewed(ReviewDimension::HumanExperienceDignity) {
            reasons.push(ReasonCode::MinimumHumanDignityReviewMissing);
        }
        if !reasons.is_empty() {
            return review_incomplete(reasons, gate_findings);
        }
        if envelope.physical_deltas.has_nonzero_delta()
            && envelope
                .trade_study_evidence_id
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
        {
            return decision(
                Disposition::TradeStudyRequired,
                vec![ReasonCode::LifecycleTradeStudyRequired],
                gate_findings,
            );
        }
        return decision(
            Disposition::TimeLimitedDeferment,
            Vec::new(),
            gate_findings,
        );
    }

    let mut reasons = Vec::new();
    if !envelope.views_complete {
        reasons.push(ReasonCode::RequiredViewsMissing);
    }
    if envelope.reviewed_dimensions.iter().any(|reviewed| !reviewed) {
        reasons.push(ReasonCode::RequiredReviewDimensionMissing);
    }
    if envelope.stakeholder_group_count == 0 {
        reasons.push(ReasonCode::StakeholderCoverageMissing);
    }
    if !envelope.dissent_handling_recorded {
        reasons.push(ReasonCode::DissentHandlingMissing);
    }
    if envelope.review_status == ReviewStatus::Incomplete && reasons.is_empty() {
        // A record cannot self-label "incomplete" and still pass all checks.
        reasons.push(ReasonCode::RequiredViewsMissing);
    }
    if !reasons.is_empty() {
        return review_incomplete(reasons, gate_findings);
    }

    if envelope.physical_deltas.has_nonzero_delta()
        && envelope
            .trade_study_evidence_id
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
    {
        return decision(
            Disposition::TradeStudyRequired,
            vec![ReasonCode::LifecycleTradeStudyRequired],
            gate_findings,
        );
    }

    decision(
        Disposition::AdmissibleForMulticriteriaReview,
        Vec::new(),
        gate_findings,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn complete_review() -> ReviewEnvelope {
        ReviewEnvelope {
            candidate_id: "SYNTHETIC-HABITAT-D".to_owned(),
            design_generation: "DESIGN-D1".to_owned(),
            hard_gates: [GateStatus::Pass; 7],
            reviewed_dimensions: [true; 5],
            review_status: ReviewStatus::Complete,
            views_complete: true,
            stakeholder_group_count: 5,
            dissent_handling_recorded: true,
            physical_deltas: PhysicalDeltas {
                mass_kg: 0.0,
                power_kw: 0.0,
                thermal_kw: 0.0,
                maintenance_time_pct: 0.0,
            },
            trade_study_evidence_id: None,
            evaluation_date: "2026-11-15".to_owned(),
            deferment: None,
        }
    }

    fn deferred_review() -> ReviewEnvelope {
        let mut envelope = complete_review();
        envelope.candidate_id = "SYNTHETIC-INTEGRATION-F".to_owned();
        envelope.design_generation = "DESIGN-F1".to_owned();
        envelope.review_status = ReviewStatus::Deferred;
        envelope.views_complete = false;
        envelope.reviewed_dimensions = [false, true, false, false, false];
        envelope.stakeholder_group_count = 3;
        envelope.deferment = Some(Deferment {
            reason: "Temporary emergency deployment".to_owned(),
            expires_on: "2027-03-31".to_owned(),
            improvement_owner: "facility-design-authority".to_owned(),
        });
        envelope
    }

    #[test]
    fn pluralistic_complete_review_is_admissible_without_beauty_score() {
        let result = evaluate(&complete_review());
        assert_eq!(
            result.disposition,
            Disposition::AdmissibleForMulticriteriaReview
        );
        assert!(result.reason_codes.is_empty());
        assert!(result.gate_findings.is_empty());
    }

    #[test]
    fn polished_but_structurally_unsafe_candidate_is_blocked() {
        let mut envelope = complete_review();
        envelope.candidate_id = "SYNTHETIC-ORS-NODE-A".to_owned();
        envelope.hard_gates[0] = GateStatus::Fail;
        envelope.hard_gates[1] = GateStatus::Unknown;
        let result = evaluate(&envelope);
        assert_eq!(result.disposition, Disposition::BlockedHardGate);
        assert_eq!(result.gate_findings.len(), 2);
        assert_eq!(result.reason_codes, vec![ReasonCode::HardGateNotPass]);
    }

    #[test]
    fn majority_preference_cannot_override_unknown_rights_or_erased_dissent() {
        let mut envelope = complete_review();
        envelope.candidate_id = "SYNTHETIC-SPACEPORT-B".to_owned();
        envelope.hard_gates[4] = GateStatus::Unknown;
        envelope.dissent_handling_recorded = false;
        let result = evaluate(&envelope);
        assert_eq!(result.disposition, Disposition::BlockedHardGate);
        assert_eq!(result.gate_findings[0].gate, HardGate::Rights);
    }

    #[test]
    fn technically_admissible_candidate_with_missing_review_is_incomplete() {
        let mut envelope = complete_review();
        envelope.candidate_id = "SYNTHETIC-SHIPYARD-C".to_owned();
        envelope.views_complete = false;
        envelope.reviewed_dimensions[1] = false;
        envelope.dissent_handling_recorded = false;
        let result = evaluate(&envelope);
        assert_eq!(result.disposition, Disposition::DesignReviewIncomplete);
        assert!(result.reason_codes.contains(&ReasonCode::RequiredViewsMissing));
        assert!(result.reason_codes.contains(&ReasonCode::RequiredReviewDimensionMissing));
        assert!(result.reason_codes.contains(&ReasonCode::DissentHandlingMissing));
    }

    #[test]
    fn lifecycle_deltas_require_a_separate_trade_study() {
        let mut envelope = complete_review();
        envelope.candidate_id = "SYNTHETIC-SHIPYARD-E".to_owned();
        envelope.physical_deltas.mass_kg = 12_000.0;
        envelope.physical_deltas.power_kw = 85.0;
        envelope.physical_deltas.thermal_kw = 110.0;
        envelope.physical_deltas.maintenance_time_pct = 18.0;
        let result = evaluate(&envelope);
        assert_eq!(result.disposition, Disposition::TradeStudyRequired);
        assert_eq!(
            result.reason_codes,
            vec![ReasonCode::LifecycleTradeStudyRequired]
        );

        envelope.trade_study_evidence_id = Some("TRADE-STUDY-E1".to_owned());
        let accepted = evaluate(&envelope);
        assert_eq!(
            accepted.disposition,
            Disposition::AdmissibleForMulticriteriaReview
        );
    }

    #[test]
    fn valid_emergency_deferment_is_time_limited_and_accountable() {
        let result = evaluate(&deferred_review());
        assert_eq!(result.disposition, Disposition::TimeLimitedDeferment);
        assert!(result.reason_codes.is_empty());
    }

    #[test]
    fn expired_deferment_is_not_admissible() {
        let mut envelope = deferred_review();
        envelope.deferment.as_mut().expect("deferment").expires_on = "2026-11-15".to_owned();
        let result = evaluate(&envelope);
        assert_eq!(result.disposition, Disposition::DesignReviewIncomplete);
        assert!(result.reason_codes.contains(&ReasonCode::ExpiredDeferment));
    }

    #[test]
    fn deferment_cannot_hide_a_nonzero_physical_delta() {
        let mut envelope = deferred_review();
        envelope.physical_deltas.power_kw = 2.0;
        let result = evaluate(&envelope);
        assert_eq!(result.disposition, Disposition::TradeStudyRequired);
    }

    #[test]
    fn hard_gate_not_applicable_is_unresolved_without_authority() {
        let mut envelope = complete_review();
        envelope.hard_gates[6] = GateStatus::NotApplicable;
        let result = evaluate(&envelope);
        assert_eq!(result.disposition, Disposition::BlockedHardGate);
        assert_eq!(
            result.gate_findings[0],
            GateFinding {
                gate: HardGate::Ecology,
                status: GateStatus::NotApplicable
            }
        );
    }

    #[test]
    fn beautiful_but_unmaintainable_candidate_is_blocked() {
        let mut envelope = complete_review();
        envelope.candidate_id = "SYNTHETIC-SUPPORT-G".to_owned();
        envelope.hard_gates[3] = GateStatus::Fail;
        envelope.hard_gates[1] = GateStatus::Unknown;
        let result = evaluate(&envelope);
        assert_eq!(result.disposition, Disposition::BlockedHardGate);
        assert!(result.gate_findings.iter().any(|finding| {
            finding.gate == HardGate::Maintainability && finding.status == GateStatus::Fail
        }));
    }

    #[test]
    fn unknown_environment_or_ecology_blocks_even_with_a_complete_review() {
        let mut envelope = complete_review();
        envelope.candidate_id = "SYNTHETIC-SPACEPORT-H".to_owned();
        envelope.hard_gates[5] = GateStatus::Unknown;
        envelope.hard_gates[6] = GateStatus::Unknown;
        let result = evaluate(&envelope);
        assert_eq!(result.disposition, Disposition::BlockedHardGate);
        assert_eq!(result.gate_findings.len(), 2);
    }

    #[test]
    fn invalid_dates_and_nonfinite_deltas_fail_closed() {
        let mut date = complete_review();
        date.evaluation_date = "2026-02-30".to_owned();
        assert_eq!(evaluate(&date).disposition, Disposition::InvalidEnvelope);

        let mut delta = complete_review();
        delta.physical_deltas.mass_kg = f64::NAN;
        assert_eq!(evaluate(&delta).disposition, Disposition::InvalidEnvelope);
    }

    #[test]
    fn missing_deferment_owner_and_expiry_do_not_pass() {
        let mut envelope = deferred_review();
        let deferment = envelope.deferment.as_mut().expect("deferment");
        deferment.improvement_owner.clear();
        deferment.expires_on.clear();
        let result = evaluate(&envelope);
        assert_eq!(result.disposition, Disposition::DesignReviewIncomplete);
        assert!(result.reason_codes.contains(&ReasonCode::MissingDefermentOwner));
        assert!(result.reason_codes.contains(&ReasonCode::InvalidDefermentExpiry));
    }
}
