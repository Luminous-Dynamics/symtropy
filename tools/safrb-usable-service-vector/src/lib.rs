#![forbid(unsafe_code)]

//! Non-compensatory service-profile qualification.
//!
//! This crate composes externally qualified evidence intervals. It does not
//! verify artifact bytes/signatures, calibrate instruments, solve physical
//! domains, authorize operation, or certify field service.

use std::collections::{BTreeMap, BTreeSet};

pub const PROFILE_VERSION: &str = "0.1.0";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PredicateKind {
    Energy,
    ActivePower,
    Voltage,
    Frequency,
    ReactivePower,
    WaveformQuality,
    ProcessOutput,
}

impl PredicateKind {
    pub const fn expected_unit(self) -> Option<&'static str> {
        match self {
            Self::Energy => Some("Wh"),
            Self::ActivePower => Some("W"),
            Self::Voltage => Some("mPu"),
            Self::Frequency => Some("mHz"),
            Self::ReactivePower => Some("var"),
            Self::WaveformQuality => Some("ppm"),
            Self::ProcessOutput => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PredicateMode {
    Required,
    Optional,
    NotApplicable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceAdmission {
    AcceptedByUpstreamVerifier,
    Unverified,
    Missing,
    Stale,
    Conflicting,
    NotAssessed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PredicateVerdict {
    Guaranteed,
    Failed,
    Indeterminate,
    NotApplicable,
}

impl PredicateVerdict {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Guaranteed => "guaranteed",
            Self::Failed => "failed",
            Self::Indeterminate => "indeterminate",
            Self::NotApplicable => "not_applicable",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServiceVerdict {
    Guaranteed,
    Failed,
    Indeterminate,
}

impl ServiceVerdict {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Guaranteed => "guaranteed",
            Self::Failed => "failed",
            Self::Indeterminate => "indeterminate",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PredicateSpec {
    pub predicate_id: String,
    pub kind: PredicateKind,
    pub mode: PredicateMode,
    pub unit_code: String,
    pub allowed_min: i64,
    pub allowed_max: i64,
    pub not_applicable_reason: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServiceProfile {
    pub profile_id: String,
    pub profile_version: &'static str,
    pub service_class: String,
    pub service_id: String,
    pub boundary_id: String,
    pub predicates: Vec<PredicateSpec>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EvidenceInterval {
    pub lower: i64,
    pub upper: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PredicateEvidence {
    pub predicate_id: String,
    pub service_id: String,
    pub boundary_id: String,
    pub run_id: String,
    pub case_sha256: String,
    pub window_start_unix_s: u64,
    pub window_end_unix_s: u64,
    pub measurement_start_unix_s: u64,
    pub measurement_end_unix_s: u64,
    pub recorded_at_unix_s: u64,
    pub fresh_until_unix_s: u64,
    pub unit_code: String,
    pub interval: Option<EvidenceInterval>,
    pub admission: EvidenceAdmission,
    pub source_ref: Option<String>,
    pub source_sha256: Option<String>,
    pub provider_id: Option<String>,
    pub qualification_ref: Option<String>,
    pub verifier_receipt_ref: Option<String>,
    pub verifier_receipt_sha256: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QualificationContext {
    pub service_id: String,
    pub boundary_id: String,
    pub run_id: String,
    pub case_sha256: String,
    pub window_start_unix_s: u64,
    pub window_end_unix_s: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PredicateResult {
    pub predicate_id: String,
    pub mode: PredicateMode,
    pub unit_code: String,
    pub verdict: PredicateVerdict,
    pub evidence_interval: Option<EvidenceInterval>,
    pub reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QualificationReport {
    pub profile_id: String,
    pub profile_version: &'static str,
    pub service_id: String,
    pub boundary_id: String,
    pub run_id: String,
    pub case_sha256: String,
    pub window_start_unix_s: u64,
    pub window_end_unix_s: u64,
    pub service_verdict: ServiceVerdict,
    pub required_guaranteed_count: usize,
    pub required_failed_count: usize,
    pub required_indeterminate_count: usize,
    pub optional_results_count: usize,
    pub not_applicable_count: usize,
    pub predicate_results: Vec<PredicateResult>,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn validate_profile(profile: &ServiceProfile) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if !nonempty(&profile.profile_id)
        || !nonempty(&profile.service_class)
        || !nonempty(&profile.service_id)
        || !nonempty(&profile.boundary_id)
    {
        errors.push("profile and service identity fields must not be empty".to_string());
    }
    if profile.profile_version != PROFILE_VERSION {
        errors.push(format!(
            "unsupported service profile version {}",
            profile.profile_version
        ));
    }
    if profile.predicates.is_empty() {
        errors.push("service profile must declare at least one predicate".to_string());
    }

    let mut ids = BTreeSet::new();
    let mut has_required = false;
    for spec in &profile.predicates {
        if !nonempty(&spec.predicate_id) || !ids.insert(spec.predicate_id.as_str()) {
            errors.push(format!(
                "predicate ID {} is empty or duplicated",
                spec.predicate_id
            ));
        }
        if !nonempty(&spec.unit_code) {
            errors.push(format!(
                "predicate {} has an empty unit code",
                spec.predicate_id
            ));
        }
        if spec.allowed_min > spec.allowed_max {
            errors.push(format!(
                "predicate {} has an inverted allowed envelope",
                spec.predicate_id
            ));
        }
        if let Some(expected) = spec.kind.expected_unit() {
            if spec.unit_code != expected {
                errors.push(format!(
                    "predicate {} requires canonical unit {}, received {}",
                    spec.predicate_id, expected, spec.unit_code
                ));
            }
        }
        match spec.mode {
            PredicateMode::Required => {
                has_required = true;
                if spec.not_applicable_reason.is_some() {
                    errors.push(format!(
                        "required predicate {} must not carry a not-applicable reason",
                        spec.predicate_id
                    ));
                }
            }
            PredicateMode::Optional => {
                if spec.not_applicable_reason.is_some() {
                    errors.push(format!(
                        "optional predicate {} must not carry a not-applicable reason",
                        spec.predicate_id
                    ));
                }
            }
            PredicateMode::NotApplicable => {
                if spec
                    .not_applicable_reason
                    .as_deref()
                    .is_none_or(|reason| !nonempty(reason))
                {
                    errors.push(format!(
                        "not-applicable predicate {} requires an explicit profile reason",
                        spec.predicate_id
                    ));
                }
            }
        }
    }
    if !has_required {
        errors.push("service profile must contain at least one required predicate".to_string());
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_context(
    profile: &ServiceProfile,
    context: &QualificationContext,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if context.service_id != profile.service_id || context.boundary_id != profile.boundary_id {
        errors.push("qualification context service/boundary does not match the profile".to_string());
    }
    if !nonempty(&context.run_id) || !lower_hex(&context.case_sha256, 64) {
        errors.push("qualification context run ID or case digest is invalid".to_string());
    }
    if context.window_start_unix_s >= context.window_end_unix_s {
        errors.push("qualification context window must have positive duration".to_string());
    }
    ok_or_errors(errors)
}

fn ok_or_errors(errors: Vec<String>) -> Result<(), Vec<String>> {
    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

fn validate_evidence_identity(
    evidence: &PredicateEvidence,
    context: &QualificationContext,
) -> Result<(), String> {
    if evidence.service_id != context.service_id
        || evidence.boundary_id != context.boundary_id
        || evidence.run_id != context.run_id
        || evidence.case_sha256 != context.case_sha256
        || evidence.window_start_unix_s != context.window_start_unix_s
        || evidence.window_end_unix_s != context.window_end_unix_s
    {
        return Err(format!(
            "predicate {} evidence service/boundary/run/case/window identity mismatch",
            evidence.predicate_id
        ));
    }
    if !nonempty(&evidence.run_id) || !lower_hex(&evidence.case_sha256, 64) {
        return Err(format!(
            "predicate {} evidence run or case identity is invalid",
            evidence.predicate_id
        ));
    }
    if evidence.interval.is_some_and(|interval| interval.lower > interval.upper) {
        return Err(format!(
            "predicate {} evidence lower bound exceeds upper bound",
            evidence.predicate_id
        ));
    }
    Ok(())
}

fn validate_accepted_provenance(evidence: &PredicateEvidence) -> Result<(), String> {
    let fields_valid = evidence.source_ref.as_deref().is_some_and(nonempty)
        && evidence
            .source_sha256
            .as_deref()
            .is_some_and(|digest| lower_hex(digest, 64))
        && evidence.provider_id.as_deref().is_some_and(nonempty)
        && evidence.qualification_ref.as_deref().is_some_and(nonempty)
        && evidence.verifier_receipt_ref.as_deref().is_some_and(nonempty)
        && evidence.verifier_receipt_sha256.as_deref().is_some_and(|digest| lower_hex(digest, 64));
    if fields_valid {
        Ok(())
    } else {
        Err(format!(
            "predicate {} claims upstream admission without complete source/provider/qualification/verifier references",
            evidence.predicate_id
        ))
    }
}

fn interval_verdict(
    spec: &PredicateSpec,
    interval: EvidenceInterval,
) -> (PredicateVerdict, &'static str) {
    if interval.lower >= spec.allowed_min && interval.upper <= spec.allowed_max {
        (PredicateVerdict::Guaranteed, "entire evidence interval is within the allowed envelope")
    } else if interval.upper < spec.allowed_min || interval.lower > spec.allowed_max {
        (PredicateVerdict::Failed, "entire evidence interval is outside a hard operating limit")
    } else {
        (PredicateVerdict::Indeterminate, "evidence interval overlaps a hard operating limit")
    }
}

fn qualify_one(
    spec: &PredicateSpec,
    evidence: Option<&PredicateEvidence>,
    context: &QualificationContext,
) -> Result<PredicateResult, String> {
    if spec.mode == PredicateMode::NotApplicable {
        return Ok(PredicateResult {
            predicate_id: spec.predicate_id.clone(),
            mode: spec.mode,
            unit_code: spec.unit_code.clone(),
            verdict: PredicateVerdict::NotApplicable,
            evidence_interval: None,
            reason: spec.not_applicable_reason.clone().unwrap_or_default(),
        });
    }

    let Some(evidence) = evidence else {
        return Ok(PredicateResult {
            predicate_id: spec.predicate_id.clone(),
            mode: spec.mode,
            unit_code: spec.unit_code.clone(),
            verdict: PredicateVerdict::Indeterminate,
            evidence_interval: None,
            reason: "no evidence supplied for this predicate".to_string(),
        });
    };

    validate_evidence_identity(evidence, context)?;
    if evidence.unit_code != spec.unit_code {
        return Err(format!(
            "predicate {} unit mismatch: expected {}, received {}",
            spec.predicate_id, spec.unit_code, evidence.unit_code
        ));
    }

    let interval = evidence.interval;
    let (verdict, reason) = match evidence.admission {
        EvidenceAdmission::AcceptedByUpstreamVerifier => {
            validate_accepted_provenance(evidence)?;
            let covers_window = evidence.measurement_start_unix_s <= context.window_start_unix_s
                && evidence.measurement_end_unix_s >= context.window_end_unix_s;
            let recorded_after_window = evidence.recorded_at_unix_s >= context.window_end_unix_s;
            let still_fresh = evidence.fresh_until_unix_s >= context.window_end_unix_s;
            if !covers_window || !recorded_after_window || !still_fresh {
                (
                    PredicateVerdict::Indeterminate,
                    "accepted source reference is stale or does not cover the declared window".to_string(),
                )
            } else if let Some(interval) = interval {
                let (verdict, reason) = interval_verdict(spec, interval);
                (verdict, reason.to_string())
            } else {
                (
                    PredicateVerdict::Indeterminate,
                    "admitted evidence does not contain a bounded value interval".to_string(),
                )
            }
        }
        EvidenceAdmission::Unverified => (
            PredicateVerdict::Indeterminate,
            "source has not been accepted by the independent upstream verifier".to_string(),
        ),
        EvidenceAdmission::Missing => (
            PredicateVerdict::Indeterminate,
            "required source observation is missing".to_string(),
        ),
        EvidenceAdmission::Stale => (
            PredicateVerdict::Indeterminate,
            "source observation is stale".to_string(),
        ),
        EvidenceAdmission::Conflicting => (
            PredicateVerdict::Indeterminate,
            "source observations conflict and have not been reconciled".to_string(),
        ),
        EvidenceAdmission::NotAssessed => (
            PredicateVerdict::Indeterminate,
            "predicate was not assessed".to_string(),
        ),
    };

    Ok(PredicateResult {
        predicate_id: spec.predicate_id.clone(),
        mode: spec.mode,
        unit_code: spec.unit_code.clone(),
        verdict,
        evidence_interval: interval,
        reason,
    })
}

pub fn qualify_service(
    profile: &ServiceProfile,
    context: &QualificationContext,
    evidence: &[PredicateEvidence],
) -> Result<QualificationReport, Vec<String>> {
    validate_profile(profile)?;
    validate_context(profile, context)?;

    let mut errors = Vec::new();
    let spec_by_id: BTreeMap<&str, &PredicateSpec> = profile
        .predicates
        .iter()
        .map(|spec| (spec.predicate_id.as_str(), spec))
        .collect();
    let mut evidence_by_id = BTreeMap::<&str, &PredicateEvidence>::new();

    for item in evidence {
        if !spec_by_id.contains_key(item.predicate_id.as_str()) {
            errors.push(format!(
                "evidence references unknown predicate {}",
                item.predicate_id
            ));
        }
        if evidence_by_id.insert(item.predicate_id.as_str(), item).is_some() {
            errors.push(format!(
                "duplicate evidence for predicate {}",
                item.predicate_id
            ));
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    let mut results = Vec::with_capacity(profile.predicates.len());
    for spec in &profile.predicates {
        match qualify_one(
            spec,
            evidence_by_id.get(spec.predicate_id.as_str()).copied(),
            context,
        ) {
            Ok(result) => results.push(result),
            Err(error) => errors.push(error),
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    results.sort_by(|left, right| left.predicate_id.cmp(&right.predicate_id));

    let mut required_guaranteed_count = 0;
    let mut required_failed_count = 0;
    let mut required_indeterminate_count = 0;
    let mut optional_results_count = 0;
    let mut not_applicable_count = 0;

    for result in &results {
        match result.mode {
            PredicateMode::Required => match result.verdict {
                PredicateVerdict::Guaranteed => required_guaranteed_count += 1,
                PredicateVerdict::Failed => required_failed_count += 1,
                PredicateVerdict::Indeterminate => required_indeterminate_count += 1,
                PredicateVerdict::NotApplicable => {
                    errors.push("required predicate cannot be not applicable".to_string())
                }
            },
            PredicateMode::Optional => optional_results_count += 1,
            PredicateMode::NotApplicable => not_applicable_count += 1,
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    let service_verdict = if required_failed_count > 0 {
        ServiceVerdict::Failed
    } else if required_indeterminate_count > 0 {
        ServiceVerdict::Indeterminate
    } else {
        ServiceVerdict::Guaranteed
    };

    Ok(QualificationReport {
        profile_id: profile.profile_id.clone(),
        profile_version: profile.profile_version,
        service_id: context.service_id.clone(),
        boundary_id: context.boundary_id.clone(),
        run_id: context.run_id.clone(),
        case_sha256: context.case_sha256.clone(),
        window_start_unix_s: context.window_start_unix_s,
        window_end_unix_s: context.window_end_unix_s,
        service_verdict,
        required_guaranteed_count,
        required_failed_count,
        required_indeterminate_count,
        optional_results_count,
        not_applicable_count,
        predicate_results: results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> QualificationContext {
        QualificationContext {
            service_id: "critical-electricity".to_string(),
            boundary_id: "hospital-main-bus".to_string(),
            run_id: "run-001".to_string(),
            case_sha256: "a".repeat(64),
            window_start_unix_s: 1_800_000_000,
            window_end_unix_s: 1_800_003_600,
        }
    }

    fn spec(
        id: &str,
        kind: PredicateKind,
        mode: PredicateMode,
        unit: &str,
        min: i64,
        max: i64,
    ) -> PredicateSpec {
        PredicateSpec {
            predicate_id: id.to_string(),
            kind,
            mode,
            unit_code: unit.to_string(),
            allowed_min: min,
            allowed_max: max,
            not_applicable_reason: None,
        }
    }

    fn profile(specs: Vec<PredicateSpec>) -> ServiceProfile {
        ServiceProfile {
            profile_id: "hospital-electricity-v1".to_string(),
            profile_version: PROFILE_VERSION,
            service_class: "healthcare-critical-power".to_string(),
            service_id: "critical-electricity".to_string(),
            boundary_id: "hospital-main-bus".to_string(),
            predicates: specs,
        }
    }

    fn admitted_evidence(
        predicate_id: &str,
        unit: &str,
        lower: i64,
        upper: i64,
    ) -> PredicateEvidence {
        let ctx = context();
        PredicateEvidence {
            predicate_id: predicate_id.to_string(),
            service_id: ctx.service_id,
            boundary_id: ctx.boundary_id,
            run_id: ctx.run_id,
            case_sha256: ctx.case_sha256,
            window_start_unix_s: ctx.window_start_unix_s,
            window_end_unix_s: ctx.window_end_unix_s,
            measurement_start_unix_s: ctx.window_start_unix_s,
            measurement_end_unix_s: ctx.window_end_unix_s,
            recorded_at_unix_s: ctx.window_end_unix_s,
            fresh_until_unix_s: ctx.window_end_unix_s + 3_600,
            unit_code: unit.to_string(),
            interval: Some(EvidenceInterval { lower, upper }),
            admission: EvidenceAdmission::AcceptedByUpstreamVerifier,
            source_ref: Some(format!("measurement://{predicate_id}")),
            source_sha256: Some("b".repeat(64)),
            provider_id: Some("independent-meter-provider".to_string()),
            qualification_ref: Some("calibration-or-model-receipt://1".to_string()),
            verifier_receipt_ref: Some("verified-artifact://1".to_string()),
            verifier_receipt_sha256: Some("c".repeat(64)),
        }
    }

    #[test]
    fn all_required_predicates_inside_envelopes_guarantee_service() {
        let profile = profile(vec![
            spec("energy", PredicateKind::Energy, PredicateMode::Required, "Wh", 1_000, 10_000),
            spec("power", PredicateKind::ActivePower, PredicateMode::Required, "W", 1_000, 20_000),
        ]);
        let evidence = vec![
            admitted_evidence("energy", "Wh", 4_000, 5_000),
            admitted_evidence("power", "W", 8_000, 9_000),
        ];
        let report = qualify_service(&profile, &context(), &evidence).unwrap();
        assert_eq!(report.service_verdict, ServiceVerdict::Guaranteed);
        assert_eq!(report.required_guaranteed_count, 2);
    }

    #[test]
    fn high_energy_cannot_compensate_for_low_required_power() {
        let profile = profile(vec![
            spec("energy", PredicateKind::Energy, PredicateMode::Required, "Wh", 1_000, 10_000),
            spec("power", PredicateKind::ActivePower, PredicateMode::Required, "W", 8_000, 20_000),
        ]);
        let evidence = vec![
            admitted_evidence("energy", "Wh", 9_000, 9_500),
            admitted_evidence("power", "W", 2_000, 3_000),
        ];
        let report = qualify_service(&profile, &context(), &evidence).unwrap();
        assert_eq!(report.service_verdict, ServiceVerdict::Failed);
        assert_eq!(report.required_failed_count, 1);
    }

    #[test]
    fn energy_and_power_pass_cannot_mask_voltage_failure() {
        let profile = profile(vec![
            spec("energy", PredicateKind::Energy, PredicateMode::Required, "Wh", 1, 20_000),
            spec("power", PredicateKind::ActivePower, PredicateMode::Required, "W", 1, 20_000),
            spec("voltage", PredicateKind::Voltage, PredicateMode::Required, "mPu", 950, 1_050),
        ]);
        let evidence = vec![
            admitted_evidence("energy", "Wh", 10_000, 11_000),
            admitted_evidence("power", "W", 10_000, 12_000),
            admitted_evidence("voltage", "mPu", 800, 900),
        ];
        let report = qualify_service(&profile, &context(), &evidence).unwrap();
        assert_eq!(report.service_verdict, ServiceVerdict::Failed);
    }

    #[test]
    fn interval_inside_hard_envelope_is_guaranteed() {
        let profile = profile(vec![spec(
            "frequency",
            PredicateKind::Frequency,
            PredicateMode::Required,
            "mHz",
            49_500,
            50_500,
        )]);
        let evidence = vec![admitted_evidence("frequency", "mHz", 49_900, 50_100)];
        let report = qualify_service(&profile, &context(), &evidence).unwrap();
        assert_eq!(report.service_verdict, ServiceVerdict::Guaranteed);
    }

    #[test]
    fn interval_entirely_outside_hard_envelope_fails() {
        let profile = profile(vec![spec(
            "frequency",
            PredicateKind::Frequency,
            PredicateMode::Required,
            "mHz",
            49_500,
            50_500,
        )]);
        let evidence = vec![admitted_evidence("frequency", "mHz", 48_000, 49_000)];
        let report = qualify_service(&profile, &context(), &evidence).unwrap();
        assert_eq!(report.service_verdict, ServiceVerdict::Failed);
    }

    #[test]
    fn interval_straddling_hard_limit_is_indeterminate() {
        let profile = profile(vec![spec(
            "voltage",
            PredicateKind::Voltage,
            PredicateMode::Required,
            "mPu",
            950,
            1_050,
        )]);
        let evidence = vec![admitted_evidence("voltage", "mPu", 930, 980)];
        let report = qualify_service(&profile, &context(), &evidence).unwrap();
        assert_eq!(report.service_verdict, ServiceVerdict::Indeterminate);
    }

    #[test]
    fn missing_required_evidence_is_indeterminate_not_zero_or_pass() {
        let profile = profile(vec![spec(
            "energy",
            PredicateKind::Energy,
            PredicateMode::Required,
            "Wh",
            1,
            10_000,
        )]);
        let report = qualify_service(&profile, &context(), &[]).unwrap();
        assert_eq!(report.service_verdict, ServiceVerdict::Indeterminate);
    }

    #[test]
    fn stale_or_conflicting_evidence_is_indeterminate() {
        let profile = profile(vec![spec(
            "energy",
            PredicateKind::Energy,
            PredicateMode::Required,
            "Wh",
            1,
            10_000,
        )]);
        let mut evidence = admitted_evidence("energy", "Wh", 9_000, 9_500);
        evidence.admission = EvidenceAdmission::Conflicting;
        let report = qualify_service(&profile, &context(), &[evidence]).unwrap();
        assert_eq!(report.service_verdict, ServiceVerdict::Indeterminate);
    }

    #[test]
    fn accepted_evidence_without_verifier_receipt_is_rejected() {
        let profile = profile(vec![spec(
            "energy",
            PredicateKind::Energy,
            PredicateMode::Required,
            "Wh",
            1,
            10_000,
        )]);
        let mut evidence = admitted_evidence("energy", "Wh", 9_000, 9_500);
        evidence.verifier_receipt_ref = None;
        evidence.verifier_receipt_sha256 = None;
        assert!(qualify_service(&profile, &context(), &[evidence]).is_err());
    }

    #[test]
    fn unit_mismatch_is_rejected_instead_of_converted_implicitly() {
        let profile = profile(vec![spec(
            "energy",
            PredicateKind::Energy,
            PredicateMode::Required,
            "Wh",
            1,
            10_000,
        )]);
        let evidence = vec![admitted_evidence("energy", "kWh", 9, 10)];
        assert!(qualify_service(&profile, &context(), &evidence).is_err());
    }

    #[test]
    fn boundary_or_run_mismatch_is_rejected() {
        let profile = profile(vec![spec(
            "energy",
            PredicateKind::Energy,
            PredicateMode::Required,
            "Wh",
            1,
            10_000,
        )]);
        let mut evidence = admitted_evidence("energy", "Wh", 9_000, 9_500);
        evidence.boundary_id = "other-bus".to_string();
        assert!(qualify_service(&profile, &context(), &[evidence]).is_err());
    }

    #[test]
    fn not_applicable_requires_profile_reason_and_is_not_scored_as_pass() {
        let mut not_applicable = spec(
            "voltage",
            PredicateKind::Voltage,
            PredicateMode::NotApplicable,
            "mPu",
            0,
            0,
        );
        not_applicable.not_applicable_reason = Some("non-electrical service profile".to_string());
        let profile = profile(vec![
            spec("process", PredicateKind::ProcessOutput, PredicateMode::Required, "m3_per_hour_milli", 1, 10_000),
            not_applicable,
        ]);
        let evidence = vec![admitted_evidence("process", "m3_per_hour_milli", 5_000, 6_000)];
        let report = qualify_service(&profile, &context(), &evidence).unwrap();
        assert_eq!(report.service_verdict, ServiceVerdict::Guaranteed);
        assert_eq!(report.not_applicable_count, 1);
        assert!(report.predicate_results.iter().any(|item| item.verdict == PredicateVerdict::NotApplicable));
    }

    #[test]
    fn not_applicable_without_reason_invalidates_profile() {
        let profile = profile(vec![
            spec("energy", PredicateKind::Energy, PredicateMode::Required, "Wh", 1, 10_000),
            spec("voltage", PredicateKind::Voltage, PredicateMode::NotApplicable, "mPu", 0, 0),
        ]);
        assert!(qualify_service(&profile, &context(), &[]).is_err());
    }

    #[test]
    fn optional_unassessed_predicate_does_not_override_required_pass() {
        let profile = profile(vec![
            spec("energy", PredicateKind::Energy, PredicateMode::Required, "Wh", 1, 10_000),
            spec("thd", PredicateKind::WaveformQuality, PredicateMode::Optional, "ppm", 0, 50_000),
        ]);
        let evidence = vec![admitted_evidence("energy", "Wh", 8_000, 9_000)];
        let report = qualify_service(&profile, &context(), &evidence).unwrap();
        assert_eq!(report.service_verdict, ServiceVerdict::Guaranteed);
        assert_eq!(report.optional_results_count, 1);
    }

    #[test]
    fn widening_an_interval_cannot_improve_a_verdict_to_guaranteed() {
        let profile = profile(vec![spec(
            "voltage",
            PredicateKind::Voltage,
            PredicateMode::Required,
            "mPu",
            950,
            1_050,
        )]);
        let narrow = vec![admitted_evidence("voltage", "mPu", 960, 1_020)];
        let mut wide_evidence = admitted_evidence("voltage", "mPu", 960, 1_020);
        wide_evidence.interval = Some(EvidenceInterval { lower: 900, upper: 1_020 });
        let narrow_report = qualify_service(&profile, &context(), &narrow).unwrap();
        let wide_report = qualify_service(&profile, &context(), &[wide_evidence]).unwrap();
        assert_eq!(narrow_report.service_verdict, ServiceVerdict::Guaranteed);
        assert_eq!(wide_report.service_verdict, ServiceVerdict::Indeterminate);
    }

    #[test]
    fn absolute_time_translation_preserves_qualification_verdict() {
        let profile = profile(vec![spec(
            "energy",
            PredicateKind::Energy,
            PredicateMode::Required,
            "Wh",
            1,
            10_000,
        )]);
        let context_a = context();
        let evidence_a = vec![admitted_evidence("energy", "Wh", 8_000, 9_000)];
        let report_a = qualify_service(&profile, &context_a, &evidence_a).unwrap();

        let offset = 86_400;
        let mut context_b = context();
        context_b.window_start_unix_s += offset;
        context_b.window_end_unix_s += offset;
        let mut evidence_b = admitted_evidence("energy", "Wh", 8_000, 9_000);
        evidence_b.window_start_unix_s += offset;
        evidence_b.window_end_unix_s += offset;
        evidence_b.measurement_start_unix_s += offset;
        evidence_b.measurement_end_unix_s += offset;
        evidence_b.recorded_at_unix_s += offset;
        evidence_b.fresh_until_unix_s += offset;
        let report_b = qualify_service(&profile, &context_b, &[evidence_b]).unwrap();
        assert_eq!(report_a.service_verdict, report_b.service_verdict);
    }

    #[test]
    fn duplicate_predicate_evidence_is_rejected() {
        let profile = profile(vec![spec(
            "energy",
            PredicateKind::Energy,
            PredicateMode::Required,
            "Wh",
            1,
            10_000,
        )]);
        let evidence = admitted_evidence("energy", "Wh", 8_000, 9_000);
        assert!(qualify_service(&profile, &context(), &[evidence.clone(), evidence]).is_err());
    }

    #[test]
    fn required_not_assessed_never_becomes_guaranteed() {
        let profile = profile(vec![spec(
            "energy",
            PredicateKind::Energy,
            PredicateMode::Required,
            "Wh",
            1,
            10_000,
        )]);
        let mut evidence = admitted_evidence("energy", "Wh", 8_000, 9_000);
        evidence.admission = EvidenceAdmission::NotAssessed;
        let report = qualify_service(&profile, &context(), &[evidence]).unwrap();
        assert_eq!(report.service_verdict, ServiceVerdict::Indeterminate);
    }
}
