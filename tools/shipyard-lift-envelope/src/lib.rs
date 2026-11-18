//! Evidence-bounded synthetic preflight checks for a single-crane ship-block hoist.
//!
//! Recomputes only declared fixed-point load arithmetic and checks source
//! identity, validity windows, exact crane-chart-cell matching, and bindings
//! to separately qualified motion/occupancy reports. It does not design rigging.

use std::collections::{BTreeMap, BTreeSet};

pub const CONTRACT_VERSION: &str = "shipyard-lift-envelope-v0.1";
pub const PPM_SCALE: u64 = 1_000_000;
pub const GRAVITY_MILLINEWTONS_PER_KG: u128 = 9_807;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceState {
    Current,
    Revoked,
    Superseded,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct EvidenceRef {
    pub source_record_id: String,
    pub owner_id: String,
    pub revision: String,
    pub subject_id: String,
    pub subject_revision: String,
    pub sha256: String,
    /// Clock domain used by the validity interval ticks.
    pub validity_clock_domain_id: String,
    /// Inclusive start of the evidence-validity interval.
    pub valid_from_tick: u64,
    /// Exclusive end of the evidence-validity interval.
    pub valid_until_tick: u64,
    pub state: EvidenceState,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RiggingLeg {
    pub leg_id: String,
    pub component_revision: String,
    /// Engineer-supplied upper bound on this leg's share of vertical load.
    pub maximum_vertical_share_ppm: u64,
    /// Engineer-supplied lower bound on sin(angle above horizontal).
    pub minimum_sine_angle_ppm: u64,
    /// Axial tensile WLL for this exact component/configuration. Sling-angle
    /// amplification is applied separately by the verifier; other required
    /// deratings must already be reflected in this source-supplied WLL.
    pub working_load_limit_mn: u64,
    pub wll_evidence: EvidenceRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CraneChartCell {
    pub crane_id: String,
    pub configuration_revision: String,
    pub radius_band_id: String,
    pub working_quadrant_id: String,
    /// Net allowable suspended load for this exact chart cell, in milli-newtons.
    /// Hook block/rigging deductions must already be handled by the chart owner.
    pub net_capacity_mn: u64,
    pub record_id: String,
    pub evidence: EvidenceRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeatherEnvelope {
    /// Conservative observed gust upper bound, in millimetres per second.
    pub gust_upper_mm_per_s: u64,
    /// Source-declared maximum permitted gust for this exact operation.
    pub maximum_gust_mm_per_s: u64,
    /// Sensor/observation subject identity (station or metocean source).
    pub observation_subject_id: String,
    pub observation_subject_revision: String,
    /// Exact source-owned limit profile identity.
    pub limit_profile_id: String,
    pub limit_profile_revision: String,
    /// Observation provenance and operating-limit provenance are different claims.
    pub observation_evidence: EvidenceRef,
    pub operating_limit_evidence: EvidenceRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MotionOccupancyBinding {
    pub motion_report_id: String,
    pub motion_report_evidence: EvidenceRef,
    pub occupancy_projection_id: String,
    pub occupancy_evidence: EvidenceRef,
    pub block_id: String,
    pub block_revision: String,
    pub payload_id: String,
    pub site_epoch_id: String,
    pub frame_id: String,
    pub clock_domain_id: String,
    /// The downstream occupancy projection must cover the complete lift interval.
    pub occupancy_start_tick: u64,
    pub occupancy_end_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiftPlan {
    pub lift_id: String,
    pub block_id: String,
    pub block_revision: String,
    pub site_epoch_id: String,
    pub frame_id: String,
    pub clock_domain_id: String,
    /// Half-open operation interval [start_tick, end_tick).
    pub start_tick: u64,
    pub end_tick: u64,
    /// Conservative upper bound, not a nominal or average mass.
    pub mass_upper_kg: u64,
    /// Source-approved dynamic amplification factor in parts per million.
    pub dynamic_factor_ppm: u64,
    pub dynamic_factor_profile_id: String,
    pub dynamic_factor_profile_revision: String,
    pub mass_evidence: EvidenceRef,
    pub centre_of_gravity_envelope_evidence: EvidenceRef,
    pub pick_point_and_load_share_evidence: EvidenceRef,
    pub dynamic_factor_evidence: EvidenceRef,
    pub crane_id: String,
    pub crane_configuration_revision: String,
    pub crane_chart: CraneChartCell,
    pub crane_radius_band_id: String,
    pub crane_working_quadrant_id: String,
    pub rigging_legs: Vec<RiggingLeg>,
    pub weather: WeatherEnvelope,
    pub motion: MotionOccupancyBinding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    /// Inputs are structurally consistent under this bounded arithmetic model.
    /// Human engineering/site review remains mandatory.
    ReadyForHumanReview,
    /// A declared quantitative limit is exceeded.
    Rejected,
    /// Evidence, identity, scope, freshness or numeric completeness is missing.
    Indeterminate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticSeverity {
    Rejected,
    Indeterminate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub severity: DiagnosticSeverity,
    pub subject_id: Option<String>,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LegAssessment {
    pub leg_id: String,
    pub tension_upper_mn: Option<u64>,
    pub working_load_limit_mn: u64,
    pub within_working_load_limit: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreflightReport {
    pub disposition: Disposition,
    pub design_load_upper_mn: Option<u64>,
    pub leg_assessments: Vec<LegAssessment>,
    pub diagnostics: Vec<Diagnostic>,
}

fn diagnostic(
    diagnostics: &mut Vec<Diagnostic>,
    code: &'static str,
    severity: DiagnosticSeverity,
    subject_id: Option<&str>,
    detail: impl Into<String>,
) {
    diagnostics.push(Diagnostic {
        code,
        severity,
        subject_id: subject_id.map(str::to_owned),
        detail: detail.into(),
    });
}

fn nonblank(value: &str) -> bool {
    !value.trim().is_empty()
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_evidence(
    evidence: &EvidenceRef,
    start_tick: u64,
    end_tick: u64,
    expected_clock_domain_id: &str,
    seen_ids: &mut BTreeMap<String, EvidenceRef>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let id = evidence.source_record_id.as_str();
    if !nonblank(id)
        || !nonblank(&evidence.owner_id)
        || !nonblank(&evidence.revision)
        || !nonblank(&evidence.subject_id)
        || !nonblank(&evidence.subject_revision)
        || !nonblank(&evidence.validity_clock_domain_id)
    {
        diagnostic(
            diagnostics,
            "EVIDENCE_IDENTITY_INCOMPLETE",
            DiagnosticSeverity::Indeterminate,
            Some(id),
            "source record, owner, document revision, exact subject binding and validity clock domain must be nonblank",
        );
    }
    if !is_sha256(&evidence.sha256) {
        diagnostic(
            diagnostics,
            "EVIDENCE_DIGEST_INVALID",
            DiagnosticSeverity::Indeterminate,
            Some(id),
            "evidence digest must be exactly 64 hexadecimal characters",
        );
    }
    if evidence.validity_clock_domain_id != expected_clock_domain_id {
        diagnostic(
            diagnostics,
            "EVIDENCE_CLOCK_DOMAIN_MISMATCH",
            DiagnosticSeverity::Indeterminate,
            Some(id),
            "evidence validity ticks must use the exact lift-plan clock domain",
        );
    }
    if evidence.valid_from_tick >= evidence.valid_until_tick
        || evidence.valid_from_tick > start_tick
        || evidence.valid_until_tick < end_tick
    {
        diagnostic(
            diagnostics,
            "EVIDENCE_NOT_VALID_FOR_LIFT_INTERVAL",
            DiagnosticSeverity::Indeterminate,
            Some(id),
            "evidence validity must cover the complete half-open lift interval",
        );
    }
    match evidence.state {
        EvidenceState::Current => {}
        EvidenceState::Revoked => diagnostic(
            diagnostics,
            "EVIDENCE_REVOKED",
            DiagnosticSeverity::Indeterminate,
            Some(id),
            "revoked evidence cannot support this preflight",
        ),
        EvidenceState::Superseded => diagnostic(
            diagnostics,
            "EVIDENCE_SUPERSEDED",
            DiagnosticSeverity::Indeterminate,
            Some(id),
            "superseded evidence cannot support this preflight",
        ),
    }
    if nonblank(id) {
        match seen_ids.get(id) {
            Some(existing) if existing != evidence => diagnostic(
                diagnostics,
                "EVIDENCE_ID_CONFLICT",
                DiagnosticSeverity::Indeterminate,
                Some(id),
                "the same evidence source-record ID was supplied with conflicting metadata",
            ),
            Some(_) => {}
            None => {
                seen_ids.insert(id.to_owned(), evidence.clone());
            }
        }
    }
}

/// Computes ceil(left * right / divisor) with checked integer arithmetic.
fn ceil_mul_div(left: u128, right: u128, divisor: u128) -> Option<u128> {
    if divisor == 0 {
        return None;
    }
    let product = left.checked_mul(right)?;
    let quotient = product / divisor;
    let remainder = product % divisor;
    if remainder == 0 {
        Some(quotient)
    } else {
        quotient.checked_add(1)
    }
}

fn to_u64(value: u128) -> Option<u64> {
    u64::try_from(value).ok()
}

fn validate_subject_binding(
    evidence: &EvidenceRef,
    expected_subject_id: &str,
    expected_subject_revision: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if evidence.subject_id != expected_subject_id
        || evidence.subject_revision != expected_subject_revision
    {
        diagnostic(
            diagnostics,
            "EVIDENCE_SUBJECT_MISMATCH",
            DiagnosticSeverity::Indeterminate,
            Some(&evidence.source_record_id),
            format!(
                "evidence subject {}/{} does not match required {}/{}",
                evidence.subject_id,
                evidence.subject_revision,
                expected_subject_id,
                expected_subject_revision
            ),
        );
    }
}

fn validate_context(plan: &LiftPlan, diagnostics: &mut Vec<Diagnostic>) {
    for (name, value) in [
        ("lift_id", plan.lift_id.as_str()),
        ("block_id", plan.block_id.as_str()),
        ("block_revision", plan.block_revision.as_str()),
        ("site_epoch_id", plan.site_epoch_id.as_str()),
        ("frame_id", plan.frame_id.as_str()),
        ("clock_domain_id", plan.clock_domain_id.as_str()),
        ("crane_id", plan.crane_id.as_str()),
        ("crane_configuration_revision", plan.crane_configuration_revision.as_str()),
        ("crane_radius_band_id", plan.crane_radius_band_id.as_str()),
        ("crane_working_quadrant_id", plan.crane_working_quadrant_id.as_str()),
        ("dynamic_factor_profile_id", plan.dynamic_factor_profile_id.as_str()),
        ("dynamic_factor_profile_revision", plan.dynamic_factor_profile_revision.as_str()),
        ("weather_observation_subject_id", plan.weather.observation_subject_id.as_str()),
        ("weather_observation_subject_revision", plan.weather.observation_subject_revision.as_str()),
        ("weather_limit_profile_id", plan.weather.limit_profile_id.as_str()),
        ("weather_limit_profile_revision", plan.weather.limit_profile_revision.as_str()),
    ] {
        if !nonblank(value) {
            diagnostic(
                diagnostics,
                "LIFT_CONTEXT_FIELD_EMPTY",
                DiagnosticSeverity::Indeterminate,
                Some(&plan.lift_id),
                format!("{name} must be nonblank"),
            );
        }
    }
    if plan.end_tick <= plan.start_tick {
        diagnostic(
            diagnostics,
            "LIFT_INTERVAL_INVALID",
            DiagnosticSeverity::Indeterminate,
            Some(&plan.lift_id),
            "lift interval must be a non-empty half-open interval",
        );
    }
    if plan.mass_upper_kg == 0 {
        diagnostic(
            diagnostics,
            "MASS_UPPER_BOUND_INVALID",
            DiagnosticSeverity::Indeterminate,
            Some(&plan.block_id),
            "mass upper bound must be positive",
        );
    }
    if plan.dynamic_factor_ppm < PPM_SCALE {
        diagnostic(
            diagnostics,
            "DYNAMIC_FACTOR_INVALID",
            DiagnosticSeverity::Indeterminate,
            Some(&plan.lift_id),
            "dynamic amplification factor must be at least 1.0 (1,000,000 ppm)",
        );
    }
}

fn validate_crane_chart(plan: &LiftPlan, diagnostics: &mut Vec<Diagnostic>) {
    let chart = &plan.crane_chart;
    let matches = chart.crane_id == plan.crane_id
        && chart.configuration_revision == plan.crane_configuration_revision
        && chart.radius_band_id == plan.crane_radius_band_id
        && chart.working_quadrant_id == plan.crane_working_quadrant_id;
    if !matches {
        diagnostic(
            diagnostics,
            "CRANE_CHART_CELL_MISMATCH",
            DiagnosticSeverity::Indeterminate,
            Some(&chart.record_id),
            "crane, configuration, exact radius band and working quadrant must match the lift; no interpolation is permitted",
        );
    }
    if !nonblank(&chart.record_id)
        || !nonblank(&chart.crane_id)
        || !nonblank(&chart.configuration_revision)
        || !nonblank(&chart.radius_band_id)
        || !nonblank(&chart.working_quadrant_id)
        || chart.net_capacity_mn == 0
        || chart.evidence.source_record_id != chart.record_id
    {
        diagnostic(
            diagnostics,
            "CRANE_CHART_RECORD_INVALID",
            DiagnosticSeverity::Indeterminate,
            Some(&chart.record_id),
            "crane chart cell identity and positive net capacity must bind to its source record",
        );
    }
}

fn validate_motion_binding(plan: &LiftPlan, diagnostics: &mut Vec<Diagnostic>) {
    let motion = &plan.motion;
    let identities_match = motion.block_id == plan.block_id
        && motion.block_revision == plan.block_revision
        && motion.payload_id == plan.block_id
        && motion.site_epoch_id == plan.site_epoch_id
        && motion.frame_id == plan.frame_id
        && motion.clock_domain_id == plan.clock_domain_id;
    if !identities_match {
        diagnostic(
            diagnostics,
            "MOTION_CONTEXT_MISMATCH",
            DiagnosticSeverity::Indeterminate,
            Some(&motion.motion_report_id),
            "motion report must bind the exact block, revision, payload, site epoch, frame and clock domain",
        );
    }
    if !nonblank(&motion.motion_report_id)
        || !nonblank(&motion.occupancy_projection_id)
        || motion.motion_report_evidence.source_record_id != motion.motion_report_id
        || motion.occupancy_evidence.source_record_id != motion.occupancy_projection_id
    {
        diagnostic(
            diagnostics,
            "MOTION_OCCUPANCY_REFERENCE_INVALID",
            DiagnosticSeverity::Indeterminate,
            Some(&motion.motion_report_id),
            "motion and occupancy references must bind to their exact source-record identities",
        );
    }
    if motion.occupancy_start_tick > plan.start_tick
        || motion.occupancy_end_tick < plan.end_tick
        || motion.occupancy_end_tick <= motion.occupancy_start_tick
    {
        diagnostic(
            diagnostics,
            "OCCUPANCY_INTERVAL_DOES_NOT_COVER_LIFT",
            DiagnosticSeverity::Indeterminate,
            Some(&motion.occupancy_projection_id),
            "existing occupancy projection must cover the complete lift interval",
        );
    }
}

fn json_quote(input: &str) -> String {
    use std::fmt::Write as _;

    let mut output = String::with_capacity(input.len() + 2);
    output.push('"');
    for character in input.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{0008}' => output.push_str("\\b"),
            '\u{000c}' => output.push_str("\\f"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character <= '\u{001f}' => {
                let _ = write!(output, "\\u{:04x}", character as u32);
            }
            character => output.push(character),
        }
    }
    output.push('"');
    output
}

fn evidence_state_name(value: EvidenceState) -> &'static str {
    match value {
        EvidenceState::Current => "current",
        EvidenceState::Revoked => "revoked",
        EvidenceState::Superseded => "superseded",
    }
}

fn evidence_json(evidence: &EvidenceRef) -> String {
    format!(
        "{{\"source_record_id\":{},\"owner_id\":{},\"revision\":{},\"subject_id\":{},\"subject_revision\":{},\"sha256\":{},\"validity_clock_domain_id\":{},\"valid_from_tick\":{},\"valid_until_tick\":{},\"state\":{}}}",
        json_quote(&evidence.source_record_id),
        json_quote(&evidence.owner_id),
        json_quote(&evidence.revision),
        json_quote(&evidence.subject_id),
        json_quote(&evidence.subject_revision),
        json_quote(&evidence.sha256.to_ascii_lowercase()),
        json_quote(&evidence.validity_clock_domain_id),
        evidence.valid_from_tick,
        evidence.valid_until_tick,
        json_quote(evidence_state_name(evidence.state)),
    )
}

impl LiftPlan {
    /// Canonical JSON bytes for the exact declared input and all evidence references.
    /// A caller should hash these bytes separately from the report's canonical JSON.
    /// The serializer does not authenticate evidence or certify physical truth.
    pub fn canonical_json(&self) -> String {
        let mut legs = self.rigging_legs.clone();
        legs.sort();
        let legs_json = legs
            .iter()
            .map(|leg| {
                format!(
                    "{{\"leg_id\":{},\"component_revision\":{},\"maximum_vertical_share_ppm\":{},\"minimum_sine_angle_ppm\":{},\"working_load_limit_mn\":{},\"wll_evidence\":{}}}",
                    json_quote(&leg.leg_id),
                    json_quote(&leg.component_revision),
                    leg.maximum_vertical_share_ppm,
                    leg.minimum_sine_angle_ppm,
                    leg.working_load_limit_mn,
                    evidence_json(&leg.wll_evidence),
                )
            })
            .collect::<Vec<_>>()
            .join(",");

        let chart = &self.crane_chart;
        let crane_json = format!(
            "{{\"crane_id\":{},\"configuration_revision\":{},\"radius_band_id\":{},\"working_quadrant_id\":{},\"net_capacity_mn\":{},\"record_id\":{},\"evidence\":{}}}",
            json_quote(&chart.crane_id),
            json_quote(&chart.configuration_revision),
            json_quote(&chart.radius_band_id),
            json_quote(&chart.working_quadrant_id),
            chart.net_capacity_mn,
            json_quote(&chart.record_id),
            evidence_json(&chart.evidence),
        );

        let weather = &self.weather;
        let weather_json = format!(
            "{{\"gust_upper_mm_per_s\":{},\"maximum_gust_mm_per_s\":{},\"observation_subject_id\":{},\"observation_subject_revision\":{},\"limit_profile_id\":{},\"limit_profile_revision\":{},\"observation_evidence\":{},\"operating_limit_evidence\":{}}}",
            weather.gust_upper_mm_per_s,
            weather.maximum_gust_mm_per_s,
            json_quote(&weather.observation_subject_id),
            json_quote(&weather.observation_subject_revision),
            json_quote(&weather.limit_profile_id),
            json_quote(&weather.limit_profile_revision),
            evidence_json(&weather.observation_evidence),
            evidence_json(&weather.operating_limit_evidence),
        );

        let motion = &self.motion;
        let motion_json = format!(
            "{{\"motion_report_id\":{},\"motion_report_evidence\":{},\"occupancy_projection_id\":{},\"occupancy_evidence\":{},\"block_id\":{},\"block_revision\":{},\"payload_id\":{},\"site_epoch_id\":{},\"frame_id\":{},\"clock_domain_id\":{},\"occupancy_start_tick\":{},\"occupancy_end_tick\":{}}}",
            json_quote(&motion.motion_report_id),
            evidence_json(&motion.motion_report_evidence),
            json_quote(&motion.occupancy_projection_id),
            evidence_json(&motion.occupancy_evidence),
            json_quote(&motion.block_id),
            json_quote(&motion.block_revision),
            json_quote(&motion.payload_id),
            json_quote(&motion.site_epoch_id),
            json_quote(&motion.frame_id),
            json_quote(&motion.clock_domain_id),
            motion.occupancy_start_tick,
            motion.occupancy_end_tick,
        );

        format!(
            "{{\"contract_version\":{},\"lift_id\":{},\"block_id\":{},\"block_revision\":{},\"site_epoch_id\":{},\"frame_id\":{},\"clock_domain_id\":{},\"start_tick\":{},\"end_tick\":{},\"mass_upper_kg\":{},\"dynamic_factor_ppm\":{},\"dynamic_factor_profile_id\":{},\"dynamic_factor_profile_revision\":{},\"mass_evidence\":{},\"centre_of_gravity_envelope_evidence\":{},\"pick_point_and_load_share_evidence\":{},\"dynamic_factor_evidence\":{},\"crane_id\":{},\"crane_configuration_revision\":{},\"crane_radius_band_id\":{},\"crane_working_quadrant_id\":{},\"crane_chart\":{},\"rigging_legs\":[{}],\"weather\":{},\"motion\":{}}}",
            json_quote(CONTRACT_VERSION),
            json_quote(&self.lift_id),
            json_quote(&self.block_id),
            json_quote(&self.block_revision),
            json_quote(&self.site_epoch_id),
            json_quote(&self.frame_id),
            json_quote(&self.clock_domain_id),
            self.start_tick,
            self.end_tick,
            self.mass_upper_kg,
            self.dynamic_factor_ppm,
            json_quote(&self.dynamic_factor_profile_id),
            json_quote(&self.dynamic_factor_profile_revision),
            evidence_json(&self.mass_evidence),
            evidence_json(&self.centre_of_gravity_envelope_evidence),
            evidence_json(&self.pick_point_and_load_share_evidence),
            evidence_json(&self.dynamic_factor_evidence),
            json_quote(&self.crane_id),
            json_quote(&self.crane_configuration_revision),
            json_quote(&self.crane_radius_band_id),
            json_quote(&self.crane_working_quadrant_id),
            crane_json,
            legs_json,
            weather_json,
            motion_json,
        )
    }
}

fn disposition_name(value: Disposition) -> &'static str {
    match value {
        Disposition::ReadyForHumanReview => "ready_for_human_review",
        Disposition::Rejected => "rejected",
        Disposition::Indeterminate => "indeterminate",
    }
}

fn severity_name(value: DiagnosticSeverity) -> &'static str {
    match value {
        DiagnosticSeverity::Rejected => "rejected",
        DiagnosticSeverity::Indeterminate => "indeterminate",
    }
}

impl PreflightReport {
    /// Canonical JSON bytes for content hashing and replay comparison.
    /// This serializes the verifier's result; it does not authenticate inputs.
    pub fn canonical_json(&self) -> String {
        let mut legs = self.leg_assessments.clone();
        legs.sort();
        let mut diagnostics = self.diagnostics.clone();
        diagnostics.sort_by(|left, right| {
            left.code
                .cmp(right.code)
                .then_with(|| left.subject_id.cmp(&right.subject_id))
                .then_with(|| left.severity.cmp(&right.severity))
                .then_with(|| left.detail.cmp(&right.detail))
        });

        let legs_json = legs
            .iter()
            .map(|leg| {
                format!(
                    "{{\"leg_id\":{},\"tension_upper_mn\":{},\"working_load_limit_mn\":{},\"within_working_load_limit\":{}}}",
                    json_quote(&leg.leg_id),
                    leg.tension_upper_mn
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "null".to_owned()),
                    leg.working_load_limit_mn,
                    leg.within_working_load_limit
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "null".to_owned()),
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let diagnostics_json = diagnostics
            .iter()
            .map(|item| {
                format!(
                    "{{\"code\":{},\"severity\":{},\"subject_id\":{},\"detail\":{}}}",
                    json_quote(item.code),
                    json_quote(severity_name(item.severity)),
                    item.subject_id
                        .as_deref()
                        .map(json_quote)
                        .unwrap_or_else(|| "null".to_owned()),
                    json_quote(&item.detail),
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"contract_version\":{},\"disposition\":{},\"design_load_upper_mn\":{},\"leg_assessments\":[{}],\"diagnostics\":[{}]}}",
            json_quote(CONTRACT_VERSION),
            json_quote(disposition_name(self.disposition)),
            self.design_load_upper_mn
                .map(|value| value.to_string())
                .unwrap_or_else(|| "null".to_owned()),
            legs_json,
            diagnostics_json
        )
    }
}

fn disposition_for(diagnostics: &[Diagnostic]) -> Disposition {
    if diagnostics
        .iter()
        .any(|item| item.severity == DiagnosticSeverity::Indeterminate)
    {
        Disposition::Indeterminate
    } else if diagnostics
        .iter()
        .any(|item| item.severity == DiagnosticSeverity::Rejected)
    {
        Disposition::Rejected
    } else {
        Disposition::ReadyForHumanReview
    }
}

/// Recompute bounded load arithmetic and check declared evidence/context.
///
/// External engineering limits are supplied by their responsible source owners.
/// The verifier checks their exact identities/validity and the narrow arithmetic
/// declared by this profile. It does not authenticate source records or prove
/// their underlying measurements/calculations true.
pub fn evaluate(plan: &LiftPlan) -> PreflightReport {
    let mut diagnostics = Vec::new();
    let mut seen_evidence_ids = BTreeMap::new();
    let mut seen_leg_ids = BTreeSet::new();
    let mut leg_assessments = Vec::new();

    validate_context(plan, &mut diagnostics);

    for evidence in [
        &plan.mass_evidence,
        &plan.centre_of_gravity_envelope_evidence,
        &plan.pick_point_and_load_share_evidence,
        &plan.dynamic_factor_evidence,
        &plan.crane_chart.evidence,
        &plan.weather.observation_evidence,
        &plan.weather.operating_limit_evidence,
        &plan.motion.motion_report_evidence,
        &plan.motion.occupancy_evidence,
    ] {
        validate_evidence(
            evidence,
            plan.start_tick,
            plan.end_tick,
            &plan.clock_domain_id,
            &mut seen_evidence_ids,
            &mut diagnostics,
        );
    }

    validate_subject_binding(&plan.mass_evidence, &plan.block_id, &plan.block_revision, &mut diagnostics);
    validate_subject_binding(
        &plan.centre_of_gravity_envelope_evidence,
        &plan.block_id,
        &plan.block_revision,
        &mut diagnostics,
    );
    validate_subject_binding(
        &plan.pick_point_and_load_share_evidence,
        &plan.block_id,
        &plan.block_revision,
        &mut diagnostics,
    );
    validate_subject_binding(
        &plan.dynamic_factor_evidence,
        &plan.dynamic_factor_profile_id,
        &plan.dynamic_factor_profile_revision,
        &mut diagnostics,
    );
    validate_subject_binding(
        &plan.crane_chart.evidence,
        &plan.crane_id,
        &plan.crane_configuration_revision,
        &mut diagnostics,
    );
    validate_subject_binding(
        &plan.weather.observation_evidence,
        &plan.weather.observation_subject_id,
        &plan.weather.observation_subject_revision,
        &mut diagnostics,
    );
    validate_subject_binding(
        &plan.weather.operating_limit_evidence,
        &plan.weather.limit_profile_id,
        &plan.weather.limit_profile_revision,
        &mut diagnostics,
    );
    validate_subject_binding(
        &plan.motion.motion_report_evidence,
        &plan.block_id,
        &plan.block_revision,
        &mut diagnostics,
    );
    validate_subject_binding(
        &plan.motion.occupancy_evidence,
        &plan.block_id,
        &plan.block_revision,
        &mut diagnostics,
    );

    validate_crane_chart(plan, &mut diagnostics);
    validate_motion_binding(plan, &mut diagnostics);

    if plan.weather.gust_upper_mm_per_s > plan.weather.maximum_gust_mm_per_s {
        diagnostic(
            &mut diagnostics,
            "WIND_GUST_LIMIT_EXCEEDED",
            DiagnosticSeverity::Rejected,
            Some(&plan.weather.observation_evidence.source_record_id),
            "conservative observed gust upper bound exceeds the declared operation limit",
        );
    }

    let design_load_upper_mn = if plan.mass_upper_kg > 0
        && plan.dynamic_factor_ppm >= PPM_SCALE
    {
        let base_load = u128::from(plan.mass_upper_kg)
            .checked_mul(GRAVITY_MILLINEWTONS_PER_KG);
        let amplified = base_load.and_then(|base| {
            ceil_mul_div(
                base,
                u128::from(plan.dynamic_factor_ppm),
                u128::from(PPM_SCALE),
            )
        });
        match amplified.and_then(to_u64) {
            Some(value) => Some(value),
            None => {
                diagnostic(
                    &mut diagnostics,
                    "DESIGN_LOAD_ARITHMETIC_OVERFLOW",
                    DiagnosticSeverity::Indeterminate,
                    Some(&plan.block_id),
                    "checked fixed-point design-load arithmetic overflowed its declared range",
                );
                None
            }
        }
    } else {
        None
    };

    if let Some(design_load) = design_load_upper_mn {
        if plan.crane_chart.net_capacity_mn > 0
            && design_load > plan.crane_chart.net_capacity_mn
            && plan.crane_chart.crane_id == plan.crane_id
            && plan.crane_chart.configuration_revision == plan.crane_configuration_revision
            && plan.crane_chart.radius_band_id == plan.crane_radius_band_id
            && plan.crane_chart.working_quadrant_id == plan.crane_working_quadrant_id
        {
            diagnostic(
                &mut diagnostics,
                "CRANE_CHART_CAPACITY_EXCEEDED",
                DiagnosticSeverity::Rejected,
                Some(&plan.crane_chart.record_id),
                "conservative design load exceeds the exact chart-cell net capacity",
            );
        }
    }

    let mut total_share_ppm = 0_u64;
    for leg in &plan.rigging_legs {
        if !nonblank(&leg.leg_id) {
            diagnostic(
                &mut diagnostics,
                "RIGGING_LEG_ID_EMPTY",
                DiagnosticSeverity::Indeterminate,
                None,
                "every rigging leg requires an explicit nonblank identity",
            );
        } else if !seen_leg_ids.insert(leg.leg_id.clone()) {
            diagnostic(
                &mut diagnostics,
                "RIGGING_LEG_ID_DUPLICATE",
                DiagnosticSeverity::Indeterminate,
                Some(&leg.leg_id),
                "rigging leg identities must be unique",
            );
        }

        validate_evidence(
            &leg.wll_evidence,
            plan.start_tick,
            plan.end_tick,
            &plan.clock_domain_id,
            &mut seen_evidence_ids,
            &mut diagnostics,
        );
        match total_share_ppm.checked_add(leg.maximum_vertical_share_ppm) {
            Some(value) => total_share_ppm = value,
            None => diagnostic(
                &mut diagnostics,
                "RIGGING_SHARE_SUM_OVERFLOW",
                DiagnosticSeverity::Indeterminate,
                Some(&leg.leg_id),
                "rigging load-share sum overflowed",
            ),
        }

        if !nonblank(&leg.component_revision) {
            diagnostic(
                &mut diagnostics,
                "RIGGING_COMPONENT_REVISION_EMPTY",
                DiagnosticSeverity::Indeterminate,
                Some(&leg.leg_id),
                "each rigging leg must bind to an exact component revision",
            );
        }
        validate_subject_binding(
            &leg.wll_evidence,
            &leg.leg_id,
            &leg.component_revision,
            &mut diagnostics,
        );

        let valid_leg = nonblank(&leg.component_revision)
            && leg.maximum_vertical_share_ppm > 0
            && leg.maximum_vertical_share_ppm <= PPM_SCALE
            && leg.minimum_sine_angle_ppm > 0
            && leg.minimum_sine_angle_ppm <= PPM_SCALE
            && leg.working_load_limit_mn > 0;

        if !valid_leg {
            diagnostic(
                &mut diagnostics,
                "RIGGING_LEG_BOUNDS_INVALID",
                DiagnosticSeverity::Indeterminate,
                Some(&leg.leg_id),
                "leg share, sine lower bound and WLL must be positive and within their declared ranges",
            );
        }

        let tension_upper_mn = if valid_leg {
            design_load_upper_mn.and_then(|load| {
                ceil_mul_div(
                    u128::from(load),
                    u128::from(leg.maximum_vertical_share_ppm),
                    u128::from(leg.minimum_sine_angle_ppm),
                )
                .and_then(to_u64)
            })
        } else {
            None
        };

        let within = match tension_upper_mn {
            Some(tension) if tension > leg.working_load_limit_mn => {
                diagnostic(
                    &mut diagnostics,
                    "RIGGING_WLL_EXCEEDED",
                    DiagnosticSeverity::Rejected,
                    Some(&leg.leg_id),
                    "conservative leg-tension upper bound exceeds the supplied configuration-specific WLL",
                );
                Some(false)
            }
            Some(_) => Some(true),
            None => {
                if valid_leg && design_load_upper_mn.is_some() {
                    diagnostic(
                        &mut diagnostics,
                        "RIGGING_TENSION_ARITHMETIC_OVERFLOW",
                        DiagnosticSeverity::Indeterminate,
                        Some(&leg.leg_id),
                        "checked sling-leg tension arithmetic overflowed its declared range",
                    );
                }
                None
            }
        };

        leg_assessments.push(LegAssessment {
            leg_id: leg.leg_id.clone(),
            tension_upper_mn,
            working_load_limit_mn: leg.working_load_limit_mn,
            within_working_load_limit: within,
        });
    }

    if plan.rigging_legs.is_empty() {
        diagnostic(
            &mut diagnostics,
            "RIGGING_LEGS_MISSING",
            DiagnosticSeverity::Indeterminate,
            Some(&plan.lift_id),
            "at least one explicitly identified rigging leg is required",
        );
    } else if total_share_ppm < PPM_SCALE {
        diagnostic(
            &mut diagnostics,
            "RIGGING_SHARE_BOUNDS_DO_NOT_COVER_LOAD",
            DiagnosticSeverity::Indeterminate,
            Some(&plan.lift_id),
            "the sum of source-supplied per-leg maximum vertical load-share bounds must be at least 1,000,000 ppm; conservative upper bounds may sum above one, and equal sharing is never inferred",
        );
    }

    leg_assessments.sort();
    diagnostics.sort_by(|left, right| {
        left.code
            .cmp(right.code)
            .then_with(|| left.subject_id.cmp(&right.subject_id))
            .then_with(|| left.severity.cmp(&right.severity))
            .then_with(|| left.detail.cmp(&right.detail))
    });

    PreflightReport {
        disposition: disposition_for(&diagnostics),
        design_load_upper_mn,
        leg_assessments,
        diagnostics,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence_for(id: &str, subject: &str, subject_revision: &str) -> EvidenceRef {
        EvidenceRef {
            source_record_id: id.to_owned(),
            owner_id: format!("owner::{id}"),
            revision: "r1".to_owned(),
            subject_id: subject.to_owned(),
            subject_revision: subject_revision.to_owned(),
            sha256: "a".repeat(64),
            validity_clock_domain_id: "yard-monotonic-ms".to_owned(),
            valid_from_tick: 0,
            valid_until_tick: 1_000,
            state: EvidenceState::Current,
        }
    }

    fn baseline() -> LiftPlan {
        LiftPlan {
            lift_id: "lift-001".to_owned(),
            block_id: "block-001".to_owned(),
            block_revision: "block-r7".to_owned(),
            site_epoch_id: "yard-epoch-3".to_owned(),
            frame_id: "yard-frame-mm".to_owned(),
            clock_domain_id: "yard-monotonic-ms".to_owned(),
            start_tick: 100,
            end_tick: 200,
            mass_upper_kg: 40_000,
            dynamic_factor_ppm: 1_100_000,
            dynamic_factor_profile_id: "dynamic-factor-profile-01".to_owned(),
            dynamic_factor_profile_revision: "r1".to_owned(),
            mass_evidence: evidence_for("mass-record-001", "block-001", "block-r7"),
            centre_of_gravity_envelope_evidence: evidence_for("cog-envelope-001", "block-001", "block-r7"),
            pick_point_and_load_share_evidence: evidence_for("pick-load-share-001", "block-001", "block-r7"),
            dynamic_factor_evidence: evidence_for("dynamic-factor-001", "dynamic-factor-profile-01", "r1"),
            crane_id: "gantry-01".to_owned(),
            crane_configuration_revision: "gantry-config-r4".to_owned(),
            crane_chart: CraneChartCell {
                crane_id: "gantry-01".to_owned(),
                configuration_revision: "gantry-config-r4".to_owned(),
                radius_band_id: "radius-band-10m".to_owned(),
                working_quadrant_id: "quadrant-A".to_owned(),
                net_capacity_mn: 500_000_000,
                record_id: "gantry-chart-cell-001".to_owned(),
                evidence: evidence_for("gantry-chart-cell-001", "gantry-01", "gantry-config-r4"),
            },
            crane_radius_band_id: "radius-band-10m".to_owned(),
            crane_working_quadrant_id: "quadrant-A".to_owned(),
            rigging_legs: vec![
                RiggingLeg {
                    leg_id: "leg-A".to_owned(),
                    component_revision: "leg-A-r1".to_owned(),
                    maximum_vertical_share_ppm: 500_000,
                    minimum_sine_angle_ppm: 707_107,
                    working_load_limit_mn: 350_000_000,
                    wll_evidence: evidence_for("wll-leg-A", "leg-A", "leg-A-r1"),
                },
                RiggingLeg {
                    leg_id: "leg-B".to_owned(),
                    component_revision: "leg-B-r1".to_owned(),
                    maximum_vertical_share_ppm: 500_000,
                    minimum_sine_angle_ppm: 707_107,
                    working_load_limit_mn: 350_000_000,
                    wll_evidence: evidence_for("wll-leg-B", "leg-B", "leg-B-r1"),
                },
            ],
            weather: WeatherEnvelope {
                gust_upper_mm_per_s: 8_000,
                maximum_gust_mm_per_s: 10_000,
                observation_subject_id: "weather-station-01".to_owned(),
                observation_subject_revision: "station-config-r3".to_owned(),
                limit_profile_id: "gantry-wind-profile-01".to_owned(),
                limit_profile_revision: "r1".to_owned(),
                observation_evidence: evidence_for(
                    "wind-observation-001",
                    "weather-station-01",
                    "station-config-r3",
                ),
                operating_limit_evidence: evidence_for(
                    "gantry-wind-limit-001",
                    "gantry-wind-profile-01",
                    "r1",
                ),
            },
            motion: MotionOccupancyBinding {
                motion_report_id: "motion-report-001".to_owned(),
                motion_report_evidence: evidence_for("motion-report-001", "block-001", "block-r7"),
                occupancy_projection_id: "occupancy-projection-001".to_owned(),
                occupancy_evidence: evidence_for("occupancy-projection-001", "block-001", "block-r7"),
                block_id: "block-001".to_owned(),
                block_revision: "block-r7".to_owned(),
                payload_id: "block-001".to_owned(),
                site_epoch_id: "yard-epoch-3".to_owned(),
                frame_id: "yard-frame-mm".to_owned(),
                clock_domain_id: "yard-monotonic-ms".to_owned(),
                occupancy_start_tick: 90,
                occupancy_end_tick: 210,
            },
        }
    }

    fn has_code(report: &PreflightReport, code: &str) -> bool {
        report.diagnostics.iter().any(|item| item.code == code)
    }


    #[test]
    fn duplicate_leg_id_permutations_preserve_canonical_input_and_result_bytes() {
        let mut first = baseline();
        first.rigging_legs[1].leg_id = first.rigging_legs[0].leg_id.clone();
        first.rigging_legs[1].component_revision = "leg-Z-r1".to_owned();
        let mut second = first.clone();
        second.rigging_legs.reverse();

        assert_eq!(first.canonical_json(), second.canonical_json());
        assert_eq!(
            evaluate(&first).canonical_json(),
            evaluate(&second).canonical_json()
        );
        assert_eq!(evaluate(&first).disposition, Disposition::Indeterminate);
    }

    #[test]
    fn frozen_conformance_fixture_maps_all_vectors_to_rust_tests() {
        const FIXTURE: &str = include_str!("../fixtures/conformance-vectors-v0.1.json");
        let test_functions: [fn(); 33] = [
            bounded_valid_input_is_ready_for_human_review_only,
            exact_chart_cell_capacity_is_a_hard_gate,
            no_chart_interpolation_or_configuration_substitution_is_permitted,
            rigging_leg_tension_is_checked_with_conservative_rounding,
            missing_load_share_is_not_replaced_with_equal_sharing,
            conservative_maximum_share_bounds_may_sum_above_one,
            share_upper_bounds_below_total_load_are_indeterminate,
            stale_evidence_abstains_instead_of_becoming_a_pass,
            revoked_source_record_is_not_admissible,
            weather_limit_exceeded_and_stale_envelope_are_distinct_failures,
            weather_observation_cannot_substitute_for_limit_profile,
            wrong_block_revision_in_motion_binding_abstains,
            occupancy_projection_must_cover_the_entire_lift,
            identical_source_evidence_may_support_multiple_claims,
            conflicting_metadata_for_one_source_record_id_is_indeterminate,
            evidence_validity_in_a_different_clock_domain_is_indeterminate,
            evidence_for_another_block_revision_cannot_be_reused,
            wll_must_bind_to_the_exact_rigging_component_revision,
            malformed_digest_cannot_support_human_review,
            input_permutation_preserves_canonical_report,
            checked_ceil_mul_div_matches_independent_bounded_integer_oracle,
            canonical_input_json_is_identical_under_leg_input_permutation,
            canonical_input_json_binds_source_digests_and_crane_configuration,
            canonical_json_is_identical_under_input_permutation,
            canonical_json_escapes_diagnostic_text,
            fixed_point_ceiling_rounds_demand_up,
            checked_load_arithmetic_abstains_on_overflow,
            invalid_empty_lift_window_is_indeterminate,
            duplicate_rigging_leg_identity_is_indeterminate,
            crane_capacity_equal_to_conservative_demand_is_accepted_for_human_review,
            crane_capacity_one_millinewton_below_demand_is_rejected,
            rigging_wll_equal_to_conservative_tension_is_accepted_for_human_review,
            rigging_wll_one_millinewton_below_conservative_tension_is_rejected
        ];
        let test_names = [
            stringify!(bounded_valid_input_is_ready_for_human_review_only),
            stringify!(exact_chart_cell_capacity_is_a_hard_gate),
            stringify!(no_chart_interpolation_or_configuration_substitution_is_permitted),
            stringify!(rigging_leg_tension_is_checked_with_conservative_rounding),
            stringify!(missing_load_share_is_not_replaced_with_equal_sharing),
            stringify!(conservative_maximum_share_bounds_may_sum_above_one),
            stringify!(share_upper_bounds_below_total_load_are_indeterminate),
            stringify!(stale_evidence_abstains_instead_of_becoming_a_pass),
            stringify!(revoked_source_record_is_not_admissible),
            stringify!(weather_limit_exceeded_and_stale_envelope_are_distinct_failures),
            stringify!(weather_observation_cannot_substitute_for_limit_profile),
            stringify!(wrong_block_revision_in_motion_binding_abstains),
            stringify!(occupancy_projection_must_cover_the_entire_lift),
            stringify!(identical_source_evidence_may_support_multiple_claims),
            stringify!(conflicting_metadata_for_one_source_record_id_is_indeterminate),
            stringify!(evidence_validity_in_a_different_clock_domain_is_indeterminate),
            stringify!(evidence_for_another_block_revision_cannot_be_reused),
            stringify!(wll_must_bind_to_the_exact_rigging_component_revision),
            stringify!(malformed_digest_cannot_support_human_review),
            stringify!(input_permutation_preserves_canonical_report),
            stringify!(checked_ceil_mul_div_matches_independent_bounded_integer_oracle),
            stringify!(canonical_input_json_is_identical_under_leg_input_permutation),
            stringify!(canonical_input_json_binds_source_digests_and_crane_configuration),
            stringify!(canonical_json_is_identical_under_input_permutation),
            stringify!(canonical_json_escapes_diagnostic_text),
            stringify!(fixed_point_ceiling_rounds_demand_up),
            stringify!(checked_load_arithmetic_abstains_on_overflow),
            stringify!(invalid_empty_lift_window_is_indeterminate),
            stringify!(duplicate_rigging_leg_identity_is_indeterminate),
            stringify!(crane_capacity_equal_to_conservative_demand_is_accepted_for_human_review),
            stringify!(crane_capacity_one_millinewton_below_demand_is_rejected),
            stringify!(rigging_wll_equal_to_conservative_tension_is_accepted_for_human_review),
            stringify!(rigging_wll_one_millinewton_below_conservative_tension_is_rejected)
        ];
        let vector_ids = [
            "LEV-BASE-001",
            "LEV-CRANE-001",
            "LEV-CRANE-002",
            "LEV-RIG-001",
            "LEV-RIG-002",
            "LEV-RIG-003",
            "LEV-RIG-004",
            "LEV-EVID-001",
            "LEV-EVID-002",
            "LEV-WEATHER-001",
            "LEV-WEATHER-002",
            "LEV-MOTION-001",
            "LEV-MOTION-002",
            "LEV-EVID-003",
            "LEV-EVID-004",
            "LEV-EVID-005",
            "LEV-EVID-006",
            "LEV-RIG-005",
            "LEV-EVID-007",
            "LEV-META-001",
            "LEV-ORACLE-001",
            "LEV-META-002",
            "LEV-META-003",
            "LEV-META-004",
            "LEV-SERIAL-001",
            "LEV-ORACLE-002",
            "LEV-OVERFLOW-001",
            "LEV-TIME-001",
            "LEV-RIG-006",
            "LEV-CRANE-003",
            "LEV-CRANE-004",
            "LEV-RIG-007",
            "LEV-RIG-008"
        ];

        assert_eq!(test_functions.len(), 33);
        assert_eq!(test_names.len(), vector_ids.len());
        assert_eq!(FIXTURE.matches(r#""id": "LEV-"#).count(), 33);
        for (test_name, vector_id) in test_names.iter().zip(vector_ids.iter()) {
            assert!(
                FIXTURE.contains(&format!(r#""id": "{vector_id}""#)),
                "missing conformance vector {vector_id}"
            );
            assert!(
                FIXTURE.contains(&format!(r#""test_name": "{test_name}""#)),
                "missing unit-test mapping {test_name}"
            );
        }
    }

    #[test]
    fn bounded_valid_input_is_ready_for_human_review_only() {
        let report = evaluate(&baseline());
        assert_eq!(report.disposition, Disposition::ReadyForHumanReview);
        assert_eq!(report.design_load_upper_mn, Some(431_508_000));
        assert!(report
            .leg_assessments
            .iter()
            .all(|item| item.within_working_load_limit == Some(true)));
    }

    #[test]
    fn exact_chart_cell_capacity_is_a_hard_gate() {
        let mut plan = baseline();
        plan.crane_chart.net_capacity_mn = 400_000_000;
        let report = evaluate(&plan);
        assert_eq!(report.disposition, Disposition::Rejected);
        assert!(has_code(&report, "CRANE_CHART_CAPACITY_EXCEEDED"));
    }

    #[test]
    fn no_chart_interpolation_or_configuration_substitution_is_permitted() {
        let mut plan = baseline();
        plan.crane_radius_band_id = "radius-band-11m".to_owned();
        let report = evaluate(&plan);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "CRANE_CHART_CELL_MISMATCH"));
    }

    #[test]
    fn rigging_leg_tension_is_checked_with_conservative_rounding() {
        let mut plan = baseline();
        plan.rigging_legs[0].working_load_limit_mn = 250_000_000;
        let report = evaluate(&plan);
        assert_eq!(report.disposition, Disposition::Rejected);
        assert!(has_code(&report, "RIGGING_WLL_EXCEEDED"));
    }

    #[test]
    fn duplicate_rigging_leg_identity_is_indeterminate() {
        let mut plan = baseline();
        plan.rigging_legs[1].leg_id = plan.rigging_legs[0].leg_id.clone();
        plan.rigging_legs[1].component_revision =
            plan.rigging_legs[0].component_revision.clone();
        plan.rigging_legs[1].wll_evidence = plan.rigging_legs[0].wll_evidence.clone();
        let report = evaluate(&plan);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "RIGGING_LEG_ID_DUPLICATE"));
    }

    #[test]
    fn crane_capacity_equal_to_conservative_demand_is_accepted_for_human_review() {
        let mut plan = baseline();
        plan.crane_chart.net_capacity_mn = 431_508_000;
        let report = evaluate(&plan);
        assert_eq!(report.disposition, Disposition::ReadyForHumanReview);
        assert!(!has_code(&report, "CRANE_CHART_CAPACITY_EXCEEDED"));
    }

    #[test]
    fn crane_capacity_one_millinewton_below_demand_is_rejected() {
        let mut plan = baseline();
        plan.crane_chart.net_capacity_mn = 431_507_999;
        let report = evaluate(&plan);
        assert_eq!(report.disposition, Disposition::Rejected);
        assert!(has_code(&report, "CRANE_CHART_CAPACITY_EXCEEDED"));
    }

    #[test]
    fn rigging_wll_equal_to_conservative_tension_is_accepted_for_human_review() {
        let mut plan = baseline();
        plan.rigging_legs[0].working_load_limit_mn = 305_122_139;
        let report = evaluate(&plan);
        assert_eq!(report.disposition, Disposition::ReadyForHumanReview);
        assert!(!has_code(&report, "RIGGING_WLL_EXCEEDED"));
    }

    #[test]
    fn rigging_wll_one_millinewton_below_conservative_tension_is_rejected() {
        let mut plan = baseline();
        plan.rigging_legs[0].working_load_limit_mn = 305_122_138;
        let report = evaluate(&plan);
        assert_eq!(report.disposition, Disposition::Rejected);
        assert!(has_code(&report, "RIGGING_WLL_EXCEEDED"));
    }

    #[test]
    fn missing_load_share_is_not_replaced_with_equal_sharing() {
        let mut plan = baseline();
        plan.rigging_legs[0].maximum_vertical_share_ppm = 0;
        let report = evaluate(&plan);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "RIGGING_LEG_BOUNDS_INVALID"));
        assert!(has_code(&report, "RIGGING_SHARE_BOUNDS_DO_NOT_COVER_LOAD"));
    }

    #[test]
    fn conservative_maximum_share_bounds_may_sum_above_one() {
        let mut plan = baseline();
        plan.rigging_legs[0].maximum_vertical_share_ppm = 600_000;
        plan.rigging_legs[1].maximum_vertical_share_ppm = 600_000;
        plan.rigging_legs[0].working_load_limit_mn = 400_000_000;
        plan.rigging_legs[1].working_load_limit_mn = 400_000_000;
        let report = evaluate(&plan);
        assert_eq!(report.disposition, Disposition::ReadyForHumanReview);
    }

    #[test]
    fn share_upper_bounds_below_total_load_are_indeterminate() {
        let mut plan = baseline();
        plan.rigging_legs[0].maximum_vertical_share_ppm = 400_000;
        plan.rigging_legs[1].maximum_vertical_share_ppm = 500_000;
        let report = evaluate(&plan);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(
            &report,
            "RIGGING_SHARE_BOUNDS_DO_NOT_COVER_LOAD"
        ));
    }

    #[test]
    fn stale_evidence_abstains_instead_of_becoming_a_pass() {
        let mut plan = baseline();
        plan.mass_evidence.valid_until_tick = 199;
        let report = evaluate(&plan);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "EVIDENCE_NOT_VALID_FOR_LIFT_INTERVAL"));
    }

    #[test]
    fn revoked_source_record_is_not_admissible() {
        let mut plan = baseline();
        plan.crane_chart.evidence.state = EvidenceState::Revoked;
        let report = evaluate(&plan);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "EVIDENCE_REVOKED"));
    }

    #[test]
    fn weather_limit_exceeded_and_stale_envelope_are_distinct_failures() {
        let mut plan = baseline();
        plan.weather.gust_upper_mm_per_s = 10_001;
        let rejected = evaluate(&plan);
        assert_eq!(rejected.disposition, Disposition::Rejected);
        assert!(has_code(&rejected, "WIND_GUST_LIMIT_EXCEEDED"));

        plan.weather.observation_evidence.valid_until_tick = 199;
        let indeterminate = evaluate(&plan);
        assert_eq!(indeterminate.disposition, Disposition::Indeterminate);
        assert!(has_code(
            &indeterminate,
            "EVIDENCE_NOT_VALID_FOR_LIFT_INTERVAL"
        ));

        let mut profile_stale = baseline();
        profile_stale.weather.operating_limit_evidence.valid_until_tick = 199;
        let profile_report = evaluate(&profile_stale);
        assert_eq!(profile_report.disposition, Disposition::Indeterminate);
        assert!(has_code(
            &profile_report,
            "EVIDENCE_NOT_VALID_FOR_LIFT_INTERVAL"
        ));
    }

    #[test]
    fn weather_observation_cannot_substitute_for_limit_profile() {
        let mut plan = baseline();
        plan.weather.operating_limit_evidence.subject_id =
            "weather-station-01".to_owned();
        plan.weather.operating_limit_evidence.subject_revision =
            "station-config-r3".to_owned();
        let report = evaluate(&plan);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "EVIDENCE_SUBJECT_MISMATCH"));
    }

    #[test]
    fn wrong_block_revision_in_motion_binding_abstains() {
        let mut plan = baseline();
        plan.motion.block_revision = "block-r6".to_owned();
        let report = evaluate(&plan);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "MOTION_CONTEXT_MISMATCH"));
    }

    #[test]
    fn occupancy_projection_must_cover_the_entire_lift() {
        let mut plan = baseline();
        plan.motion.occupancy_end_tick = 199;
        let report = evaluate(&plan);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(
            &report,
            "OCCUPANCY_INTERVAL_DOES_NOT_COVER_LIFT"
        ));
    }

    #[test]
    fn identical_source_evidence_may_support_multiple_claims() {
        let mut plan = baseline();
        plan.pick_point_and_load_share_evidence =
            plan.centre_of_gravity_envelope_evidence.clone();
        let report = evaluate(&plan);
        assert_eq!(report.disposition, Disposition::ReadyForHumanReview);
    }

    #[test]
    fn conflicting_metadata_for_one_source_record_id_is_indeterminate() {
        let mut plan = baseline();
        plan.rigging_legs[1].wll_evidence = plan.rigging_legs[0].wll_evidence.clone();
        plan.rigging_legs[1].wll_evidence.sha256 = "b".repeat(64);
        let report = evaluate(&plan);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "EVIDENCE_ID_CONFLICT"));
    }

    #[test]
    fn evidence_validity_in_a_different_clock_domain_is_indeterminate() {
        let mut plan = baseline();
        plan.mass_evidence.validity_clock_domain_id = "utc-wall-clock".to_owned();
        let report = evaluate(&plan);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "EVIDENCE_CLOCK_DOMAIN_MISMATCH"));
    }

    #[test]
    fn evidence_for_another_block_revision_cannot_be_reused() {
        let mut plan = baseline();
        plan.mass_evidence.subject_id = "another-block".to_owned();
        let report = evaluate(&plan);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "EVIDENCE_SUBJECT_MISMATCH"));
    }

    #[test]
    fn wll_must_bind_to_the_exact_rigging_component_revision() {
        let mut plan = baseline();
        plan.rigging_legs[0].component_revision = "leg-A-r2".to_owned();
        let report = evaluate(&plan);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "EVIDENCE_SUBJECT_MISMATCH"));
    }

    #[test]
    fn malformed_digest_cannot_support_human_review() {
        let mut plan = baseline();
        plan.mass_evidence.sha256 = "not-a-digest".to_owned();
        let report = evaluate(&plan);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "EVIDENCE_DIGEST_INVALID"));
    }

    #[test]
    fn input_permutation_preserves_canonical_report() {
        let first = baseline();
        let mut second = baseline();
        second.rigging_legs.reverse();
        assert_eq!(evaluate(&first), evaluate(&second));
    }

    #[test]
    fn checked_ceil_mul_div_matches_independent_bounded_integer_oracle() {
        for left in 0_u128..40 {
            for right in 0_u128..40 {
                for divisor in 1_u128..40 {
                    let product = left * right;
                    let expected = (product + divisor - 1) / divisor;
                    assert_eq!(
                        ceil_mul_div(left, right, divisor),
                        Some(expected),
                        "left={left}, right={right}, divisor={divisor}"
                    );
                }
            }
        }
    }

    #[test]
    fn canonical_input_json_is_identical_under_leg_input_permutation() {
        let first = baseline();
        let mut second = baseline();
        second.rigging_legs.reverse();
        assert_eq!(first.canonical_json(), second.canonical_json());
    }

    #[test]
    fn canonical_input_json_binds_source_digests_and_crane_configuration() {
        let baseline = baseline();
        let mut changed_mass_source = baseline.clone();
        changed_mass_source.mass_evidence.sha256 = "b".repeat(64);
        assert_ne!(baseline.canonical_json(), changed_mass_source.canonical_json());

        let mut changed_crane = baseline.clone();
        changed_crane.crane_configuration_revision = "gantry-config-r5".to_owned();
        assert_ne!(baseline.canonical_json(), changed_crane.canonical_json());
    }

    #[test]
    fn canonical_json_is_identical_under_input_permutation() {
        let mut first = baseline();
        let mut second = baseline();
        second.rigging_legs.reverse();
        assert_eq!(
            evaluate(&first).canonical_json(),
            evaluate(&second).canonical_json()
        );

        first.rigging_legs.swap(0, 1);
        assert_eq!(
            evaluate(&first).canonical_json(),
            evaluate(&second).canonical_json()
        );
    }

    #[test]
    fn canonical_json_escapes_diagnostic_text() {
        let mut plan = baseline();
        plan.block_id = "block\n001".to_owned();
        let report = evaluate(&plan);
        assert!(report.canonical_json().contains("\\n"));
    }

    #[test]
    fn fixed_point_ceiling_rounds_demand_up() {
        assert_eq!(ceil_mul_div(10, 10, 6), Some(17));
        assert_eq!(ceil_mul_div(12, 10, 6), Some(20));
        assert_eq!(ceil_mul_div(1, 1, 0), None);
    }

    #[test]
    fn checked_load_arithmetic_abstains_on_overflow() {
        let mut plan = baseline();
        plan.mass_upper_kg = u64::MAX;
        plan.dynamic_factor_ppm = u64::MAX;
        let report = evaluate(&plan);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "DESIGN_LOAD_ARITHMETIC_OVERFLOW"));
    }

    #[test]
    fn invalid_empty_lift_window_is_indeterminate() {
        let mut plan = baseline();
        plan.end_tick = plan.start_tick;
        let report = evaluate(&plan);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "LIFT_INTERVAL_INVALID"));
    }
}
