//! Evidence-bounded synthetic preflight checks for a single-crane ship-block hoist.
//!
//! Recomputes only declared fixed-point load arithmetic and checks source
//! identity, validity windows, exact crane-chart-cell matching, and bindings
//! to separately qualified motion/occupancy reports. It does not design rigging.

use std::collections::{BTreeMap, BTreeSet};

pub const PPM_SCALE: u64 = 1_000_000;
pub const GRAVITY_MILLINEWTONS_PER_KG: u128 = 9_807;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceState {
    Current,
    Revoked,
    Superseded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceRef {
    pub source_record_id: String,
    pub owner_id: String,
    pub revision: String,
    pub subject_id: String,
    pub subject_revision: String,
    pub sha256: String,
    /// Inclusive start of the evidence-validity interval.
    pub valid_from_tick: u64,
    /// Exclusive end of the evidence-validity interval.
    pub valid_until_tick: u64,
    pub state: EvidenceState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
    pub evidence: EvidenceRef,
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
    pub weather_profile_revision: String,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
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
    seen_ids: &mut BTreeMap<String, EvidenceRef>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let id = evidence.source_record_id.as_str();
    if !nonblank(id)
        || !nonblank(&evidence.owner_id)
        || !nonblank(&evidence.revision)
        || !nonblank(&evidence.subject_id)
        || !nonblank(&evidence.subject_revision)
    {
        diagnostic(
            diagnostics,
            "EVIDENCE_IDENTITY_INCOMPLETE",
            DiagnosticSeverity::Indeterminate,
            Some(id),
            "source record, owner, document revision and exact subject binding must be nonblank",
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
        ("weather_profile_revision", plan.weather_profile_revision.as_str()),
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
        &plan.weather.evidence,
        &plan.motion.motion_report_evidence,
        &plan.motion.occupancy_evidence,
    ] {
        validate_evidence(
            evidence,
            plan.start_tick,
            plan.end_tick,
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
        &plan.weather.evidence,
        &plan.site_epoch_id,
        &plan.weather_profile_revision,
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
            Some(&plan.weather.evidence.source_record_id),
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

    leg_assessments.sort_by(|left, right| left.leg_id.cmp(&right.leg_id));
    diagnostics.sort_by(|left, right| {
        left.code
            .cmp(right.code)
            .then_with(|| left.subject_id.cmp(&right.subject_id))
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
            weather_profile_revision: "r1".to_owned(),
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
                evidence: evidence_for("weather-envelope-001", "yard-epoch-3", "r1"),
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
    fn missing_load_share_is_not_replaced_with_equal_sharing() {
        let mut plan = baseline();
        plan.rigging_legs[0].maximum_vertical_share_ppm = 0;
        let report = evaluate(&plan);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "RIGGING_LEG_BOUNDS_INVALID"));
        assert!(has_code(&report, "RIGGING_SHARES_DO_NOT_SUM_TO_ONE"));
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

        plan.weather.evidence.valid_until_tick = 199;
        let indeterminate = evaluate(&plan);
        assert_eq!(indeterminate.disposition, Disposition::Indeterminate);
        assert!(has_code(
            &indeterminate,
            "EVIDENCE_NOT_VALID_FOR_LIFT_INTERVAL"
        ));
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
