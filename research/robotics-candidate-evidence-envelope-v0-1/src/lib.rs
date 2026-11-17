//! Revision-bound, receipt-bearing robotics acquisition candidates.
//!
//! This crate validates the shape and consistency of candidate evidence.
//! It does not authenticate receipts, calculate SHA-256, verify source records,
//! decide which evidence is valuable, authorize motion, or accept mission stages.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

pub const SCHEMA_VERSION: &str = "robotics-candidate-evidence-envelope/v0.1";
const MAX_ABSOLUTE_COORDINATE_M: f64 = 1.0e9;
const MAX_RESOURCE_VALUE: f64 = 1.0e12;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum EvidenceDomain {
    GeometryCoverage,
    SensorQuality,
    Calibration,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceStatus {
    Qualified,
    NotQualified,
    Unknown,
    InvalidEvidence,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateEligibility {
    Qualified,
    NotQualified,
    Unknown,
    InvalidEvidence,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CandidateBinding {
    pub candidate_id: String,
    pub pass_id: String,
    pub mission_id: String,
    pub mission_revision: String,
    /// Exact upstream evidence-request set from Symthaea #12046, including
    /// its behavior-conditioned progress/constraint revision.
    pub request_set_id: String,
    pub request_set_revision: String,
    pub request_set_sha256: String,
    pub scenario_id: String,
    pub scenario_revision: String,
    pub robot_id: String,
    pub robot_configuration_revision: String,
    pub robot_configuration_sha256: String,
    pub module_manifest_sha256: String,
    pub site_id: String,
    pub site_revision: String,
    pub frame_id: String,
    pub frame_revision: String,
    pub map_revision: String,
    /// Clock identity and revision. Timestamps are in this clock domain and
    /// are not assumed to be UTC or directly comparable to another source.
    pub timebase_id: String,
    pub timebase_revision: String,
    pub acquisition_start_ns: u64,
    pub acquisition_end_ns: u64,
    pub sensor_id: String,
    pub sensor_revision: String,
    pub mount_id: String,
    pub mount_revision: String,
    pub calibration_bundle_sha256: String,
    pub observation_bundle_sha256: String,
    /// Sorted during canonical encoding. Duplicate or blank IDs are invalid.
    pub target_ids: Vec<String>,
    /// Coordinates are in the explicitly bound frame, in metres.
    pub position_u_m: f64,
    pub position_v_m: f64,
    pub view_angle_rad: f64,
    pub cost_model_id: String,
    pub cost_model_revision: String,
    pub cost_model_sha256: String,
    pub estimated_acquisition_time_s: f64,
    pub estimated_energy_wh: f64,
    pub risk_penalty_s: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EvidenceReceipt {
    pub domain: EvidenceDomain,
    pub timebase_id: String,
    pub timebase_revision: String,
    pub record_id: String,
    pub issuer_id: String,
    pub issuer_revision: String,
    pub profile_id: String,
    pub profile_revision: String,
    pub status: EvidenceStatus,
    /// SHA-256 of CandidateBinding::canonical_binding_bytes(), supplied by
    /// the external digest/provenance boundary.
    pub binding_sha256: String,
    /// Digest of the evaluator's result record; not verified by this crate.
    pub result_sha256: String,
    /// Geometry/sensor: observation bundle. Calibration: calibration bundle.
    pub source_bundle_sha256: String,
    /// Evaluator-record issuance time in its declared context.
    pub issued_at_ns: u64,
    /// Required as a pair for Calibration; optional as a pair otherwise.
    pub valid_from_ns: Option<u64>,
    pub valid_until_ns: Option<u64>,
    /// Only GeometryCoverage receipts may declare covered targets.
    pub covered_target_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CandidateEvidenceEnvelope {
    pub schema_version: String,
    pub binding: CandidateBinding,
    /// Digest over canonical binding bytes, produced by an external SHA-256
    /// implementation and authenticated by the provenance owner as needed.
    pub binding_sha256: String,
    pub receipts: Vec<EvidenceReceipt>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EnvelopeError {
    UnsupportedSchemaVersion,
    MissingIdentifier(&'static str),
    MissingReceiptMetadata(&'static str),
    MalformedSha256(&'static str),
    ComputedBindingDigestMismatch,
    InvalidAcquisitionInterval,
    InvalidNumericValue(&'static str),
    DuplicateTarget(String),
    MissingReceipt(EvidenceDomain),
    DuplicateReceiptDomain(EvidenceDomain),
    UnexpectedReceiptCount,
    DuplicateReceiptId(String),
    DuplicateResultDigest(String),
    BindingDigestMismatch(EvidenceDomain),
    SourceBundleMismatch(EvidenceDomain),
    GeometryCoverageMismatch,
    UnexpectedReceiptTargetScope(EvidenceDomain),
    ReceiptIssuedBeforeAcquisition(EvidenceDomain),
    MalformedValidityInterval(EvidenceDomain),
    CalibrationNotValidForAcquisition,
    ReceiptValidityDoesNotCoverAcquisition(EvidenceDomain),
    ReceiptTimebaseMismatch(EvidenceDomain),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateEvidenceAssessment {
    pub eligibility: CandidateEligibility,
    pub non_qualified_domains: Vec<EvidenceDomain>,
    pub unknown_domains: Vec<EvidenceDomain>,
    pub invalid_domains: Vec<EvidenceDomain>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QualifiedCandidateProjection {
    pub candidate_id: String,
    pub pass_id: String,
    pub covered_target_ids: Vec<String>,
    pub u_m: f64,
    pub v_m: f64,
    pub view_angle_rad: f64,
    pub acquisition_time_s: f64,
    pub energy_wh: f64,
    pub risk_penalty_s: f64,
}

/// Proof token produced only by successful structural validation. Its fields
/// are private so callers cannot manufacture planner eligibility by setting
/// a boolean or copying a receipt status.
#[derive(Clone, Debug)]
pub struct ValidatedCandidateEnvelope<'a> {
    envelope: &'a CandidateEvidenceEnvelope,
    assessment: CandidateEvidenceAssessment,
}

impl ValidatedCandidateEnvelope<'_> {
    pub fn assessment(&self) -> &CandidateEvidenceAssessment {
        &self.assessment
    }

    pub fn eligibility(&self) -> CandidateEligibility {
        self.assessment.eligibility
    }

    /// Produce planner inputs only when all three evidence domains are
    /// Qualified and all structural bindings have passed validation.
    pub fn qualified_projection(&self) -> Option<QualifiedCandidateProjection> {
        if self.assessment.eligibility != CandidateEligibility::Qualified {
            return None;
        }
        let binding = &self.envelope.binding;
        Some(QualifiedCandidateProjection {
            candidate_id: binding.candidate_id.clone(),
            pass_id: binding.pass_id.clone(),
            covered_target_ids: binding.target_ids.clone(),
            u_m: binding.position_u_m,
            v_m: binding.position_v_m,
            view_angle_rad: binding.view_angle_rad,
            acquisition_time_s: binding.estimated_acquisition_time_s,
            energy_wh: binding.estimated_energy_wh,
            risk_penalty_s: binding.risk_penalty_s,
        })
    }
}

impl CandidateBinding {
    /// Return a deterministic, length-prefixed canonical preimage.
    ///
    /// Callers should SHA-256 these exact bytes and pass the result as
    /// binding_sha256. This crate intentionally contains no cryptography and
    /// does not verify that the supplied digest is genuine.
    pub fn canonical_binding_bytes(&self) -> Vec<u8> {
        let mut output = Vec::new();
        push_str(&mut output, SCHEMA_VERSION);
        push_str(&mut output, &self.candidate_id);
        push_str(&mut output, &self.pass_id);
        push_str(&mut output, &self.mission_id);
        push_str(&mut output, &self.mission_revision);
        push_str(&mut output, &self.request_set_id);
        push_str(&mut output, &self.request_set_revision);
        push_str(&mut output, &self.request_set_sha256);
        push_str(&mut output, &self.scenario_id);
        push_str(&mut output, &self.scenario_revision);
        push_str(&mut output, &self.robot_id);
        push_str(&mut output, &self.robot_configuration_revision);
        push_str(&mut output, &self.robot_configuration_sha256);
        push_str(&mut output, &self.module_manifest_sha256);
        push_str(&mut output, &self.site_id);
        push_str(&mut output, &self.site_revision);
        push_str(&mut output, &self.frame_id);
        push_str(&mut output, &self.frame_revision);
        push_str(&mut output, &self.map_revision);
        push_str(&mut output, &self.timebase_id);
        push_str(&mut output, &self.timebase_revision);
        push_u64(&mut output, self.acquisition_start_ns);
        push_u64(&mut output, self.acquisition_end_ns);
        push_str(&mut output, &self.sensor_id);
        push_str(&mut output, &self.sensor_revision);
        push_str(&mut output, &self.mount_id);
        push_str(&mut output, &self.mount_revision);
        push_str(&mut output, &self.calibration_bundle_sha256);
        push_str(&mut output, &self.observation_bundle_sha256);

        let mut targets = self.target_ids.clone();
        targets.sort();
        push_u64(&mut output, targets.len() as u64);
        for target_id in targets {
            push_str(&mut output, &target_id);
        }

        push_f64(&mut output, self.position_u_m);
        push_f64(&mut output, self.position_v_m);
        push_f64(&mut output, self.view_angle_rad);
        push_str(&mut output, &self.cost_model_id);
        push_str(&mut output, &self.cost_model_revision);
        push_str(&mut output, &self.cost_model_sha256);
        push_f64(&mut output, self.estimated_acquisition_time_s);
        push_f64(&mut output, self.estimated_energy_wh);
        push_f64(&mut output, self.risk_penalty_s);
        output
    }
}

pub fn validate_candidate_envelope<'a>(
    envelope: &'a CandidateEvidenceEnvelope,
    computed_binding_sha256: &str,
) -> Result<ValidatedCandidateEnvelope<'a>, Vec<EnvelopeError>> {
    let mut errors = Vec::new();
    if envelope.schema_version != SCHEMA_VERSION {
        errors.push(EnvelopeError::UnsupportedSchemaVersion);
    }

    validate_binding(&envelope.binding, &mut errors);
    validate_sha256(&envelope.binding_sha256, "binding_sha256", &mut errors);
    validate_sha256(
        computed_binding_sha256,
        "computed_binding_sha256",
        &mut errors,
    );
    if computed_binding_sha256 != envelope.binding_sha256 {
        errors.push(EnvelopeError::ComputedBindingDigestMismatch);
    }

    if envelope.receipts.len() != 3 {
        errors.push(EnvelopeError::UnexpectedReceiptCount);
    }

    let mut seen_domains = BTreeSet::new();
    let mut seen_record_ids = BTreeSet::new();
    let mut seen_result_digests = BTreeSet::new();

    for receipt in &envelope.receipts {
        if !seen_domains.insert(receipt.domain) {
            errors.push(EnvelopeError::DuplicateReceiptDomain(receipt.domain));
        }

        if receipt.record_id.trim().is_empty() {
            errors.push(EnvelopeError::MissingReceiptMetadata("record_id"));
        } else if !seen_record_ids.insert(receipt.record_id.clone()) {
            errors.push(EnvelopeError::DuplicateReceiptId(receipt.record_id.clone()));
        }

        validate_receipt_metadata(receipt, &mut errors);
        validate_sha256(
            &receipt.binding_sha256,
            "receipt.binding_sha256",
            &mut errors,
        );
        validate_sha256(&receipt.result_sha256, "receipt.result_sha256", &mut errors);
        validate_sha256(
            &receipt.source_bundle_sha256,
            "receipt.source_bundle_sha256",
            &mut errors,
        );

        if !receipt.result_sha256.is_empty()
            && !seen_result_digests.insert(receipt.result_sha256.clone())
        {
            errors.push(EnvelopeError::DuplicateResultDigest(
                receipt.result_sha256.clone(),
            ));
        }

        if receipt.binding_sha256 != envelope.binding_sha256 {
            errors.push(EnvelopeError::BindingDigestMismatch(receipt.domain));
        }
        if receipt.timebase_id != envelope.binding.timebase_id
            || receipt.timebase_revision != envelope.binding.timebase_revision
        {
            errors.push(EnvelopeError::ReceiptTimebaseMismatch(receipt.domain));
        }

        let expected_source = match receipt.domain {
            EvidenceDomain::GeometryCoverage | EvidenceDomain::SensorQuality => {
                &envelope.binding.observation_bundle_sha256
            }
            EvidenceDomain::Calibration => &envelope.binding.calibration_bundle_sha256,
        };
        if &receipt.source_bundle_sha256 != expected_source {
            errors.push(EnvelopeError::SourceBundleMismatch(receipt.domain));
        }

        match receipt.domain {
            EvidenceDomain::GeometryCoverage => {
                if !same_string_set(&receipt.covered_target_ids, &envelope.binding.target_ids) {
                    errors.push(EnvelopeError::GeometryCoverageMismatch);
                }
            }
            EvidenceDomain::SensorQuality | EvidenceDomain::Calibration => {
                if !receipt.covered_target_ids.is_empty() {
                    errors.push(EnvelopeError::UnexpectedReceiptTargetScope(receipt.domain));
                }
            }
        }

        match receipt.domain {
            EvidenceDomain::GeometryCoverage | EvidenceDomain::SensorQuality => {
                if receipt.issued_at_ns < envelope.binding.acquisition_end_ns {
                    errors.push(EnvelopeError::ReceiptIssuedBeforeAcquisition(
                        receipt.domain,
                    ));
                }
                validate_optional_validity(receipt, &envelope.binding, &mut errors);
            }
            EvidenceDomain::Calibration => {
                if !validity_covers_acquisition(receipt, &envelope.binding) {
                    errors.push(EnvelopeError::CalibrationNotValidForAcquisition);
                }
            }
        }
    }

    for domain in [
        EvidenceDomain::GeometryCoverage,
        EvidenceDomain::SensorQuality,
        EvidenceDomain::Calibration,
    ] {
        if !seen_domains.contains(&domain) {
            errors.push(EnvelopeError::MissingReceipt(domain));
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    let mut non_qualified_domains = Vec::new();
    let mut unknown_domains = Vec::new();
    let mut invalid_domains = Vec::new();

    for receipt in &envelope.receipts {
        match receipt.status {
            EvidenceStatus::Qualified => {}
            EvidenceStatus::NotQualified => non_qualified_domains.push(receipt.domain),
            EvidenceStatus::Unknown => unknown_domains.push(receipt.domain),
            EvidenceStatus::InvalidEvidence => invalid_domains.push(receipt.domain),
        }
    }

    non_qualified_domains.sort();
    unknown_domains.sort();
    invalid_domains.sort();

    let eligibility = if !invalid_domains.is_empty() {
        CandidateEligibility::InvalidEvidence
    } else if !non_qualified_domains.is_empty() {
        CandidateEligibility::NotQualified
    } else if !unknown_domains.is_empty() {
        CandidateEligibility::Unknown
    } else {
        CandidateEligibility::Qualified
    };

    Ok(ValidatedCandidateEnvelope {
        envelope,
        assessment: CandidateEvidenceAssessment {
            eligibility,
            non_qualified_domains,
            unknown_domains,
            invalid_domains,
        },
    })
}

fn validate_binding(binding: &CandidateBinding, errors: &mut Vec<EnvelopeError>) {
    for (value, label) in [
        (&binding.candidate_id, "candidate_id"),
        (&binding.pass_id, "pass_id"),
        (&binding.mission_id, "mission_id"),
        (&binding.mission_revision, "mission_revision"),
        (&binding.request_set_id, "request_set_id"),
        (&binding.request_set_revision, "request_set_revision"),
        (&binding.scenario_id, "scenario_id"),
        (&binding.scenario_revision, "scenario_revision"),
        (&binding.robot_id, "robot_id"),
        (
            &binding.robot_configuration_revision,
            "robot_configuration_revision",
        ),
        (&binding.site_id, "site_id"),
        (&binding.site_revision, "site_revision"),
        (&binding.frame_id, "frame_id"),
        (&binding.frame_revision, "frame_revision"),
        (&binding.map_revision, "map_revision"),
        (&binding.timebase_id, "timebase_id"),
        (&binding.timebase_revision, "timebase_revision"),
        (&binding.sensor_id, "sensor_id"),
        (&binding.sensor_revision, "sensor_revision"),
        (&binding.mount_id, "mount_id"),
        (&binding.mount_revision, "mount_revision"),
        (&binding.cost_model_id, "cost_model_id"),
        (&binding.cost_model_revision, "cost_model_revision"),
    ] {
        if value.trim().is_empty() {
            errors.push(EnvelopeError::MissingIdentifier(label));
        }
    }

    for (value, label) in [
        (
            &binding.robot_configuration_sha256,
            "robot_configuration_sha256",
        ),
        (&binding.request_set_sha256, "request_set_sha256"),
        (&binding.module_manifest_sha256, "module_manifest_sha256"),
        (
            &binding.calibration_bundle_sha256,
            "calibration_bundle_sha256",
        ),
        (
            &binding.observation_bundle_sha256,
            "observation_bundle_sha256",
        ),
        (&binding.cost_model_sha256, "cost_model_sha256"),
    ] {
        validate_sha256(value, label, errors);
    }

    if binding.acquisition_end_ns <= binding.acquisition_start_ns {
        errors.push(EnvelopeError::InvalidAcquisitionInterval);
    }

    for target_id in &binding.target_ids {
        if target_id.trim().is_empty() {
            errors.push(EnvelopeError::MissingIdentifier("target_id"));
        }
    }

    let mut targets = BTreeSet::new();
    for target_id in &binding.target_ids {
        if !targets.insert(target_id) {
            errors.push(EnvelopeError::DuplicateTarget(target_id.clone()));
        }
    }
    if binding.target_ids.is_empty() {
        errors.push(EnvelopeError::MissingIdentifier("target_ids"));
    }

    for (value, label) in [
        (binding.position_u_m, "position_u_m"),
        (binding.position_v_m, "position_v_m"),
        (binding.view_angle_rad, "view_angle_rad"),
        (
            binding.estimated_acquisition_time_s,
            "estimated_acquisition_time_s",
        ),
        (binding.estimated_energy_wh, "estimated_energy_wh"),
        (binding.risk_penalty_s, "risk_penalty_s"),
    ] {
        if !value.is_finite() {
            errors.push(EnvelopeError::InvalidNumericValue(label));
        }
    }

    if binding.position_u_m.abs() > MAX_ABSOLUTE_COORDINATE_M
        || binding.position_v_m.abs() > MAX_ABSOLUTE_COORDINATE_M
    {
        errors.push(EnvelopeError::InvalidNumericValue("position"));
    }
    if !(-std::f64::consts::PI..=std::f64::consts::PI).contains(&binding.view_angle_rad) {
        errors.push(EnvelopeError::InvalidNumericValue("view_angle_rad"));
    }
    if binding.estimated_acquisition_time_s < 0.0
        || binding.estimated_acquisition_time_s > MAX_RESOURCE_VALUE
    {
        errors.push(EnvelopeError::InvalidNumericValue(
            "estimated_acquisition_time_s",
        ));
    }
    if binding.estimated_energy_wh < 0.0 || binding.estimated_energy_wh > MAX_RESOURCE_VALUE {
        errors.push(EnvelopeError::InvalidNumericValue("estimated_energy_wh"));
    }
    if binding.risk_penalty_s < 0.0 || binding.risk_penalty_s > MAX_RESOURCE_VALUE {
        errors.push(EnvelopeError::InvalidNumericValue("risk_penalty_s"));
    }
}

fn validate_receipt_metadata(receipt: &EvidenceReceipt, errors: &mut Vec<EnvelopeError>) {
    for (value, label) in [
        (&receipt.issuer_id, "issuer_id"),
        (&receipt.issuer_revision, "issuer_revision"),
        (&receipt.profile_id, "profile_id"),
        (&receipt.profile_revision, "profile_revision"),
    ] {
        if value.trim().is_empty() {
            errors.push(EnvelopeError::MissingReceiptMetadata(label));
        }
    }
}

fn validate_optional_validity(
    receipt: &EvidenceReceipt,
    binding: &CandidateBinding,
    errors: &mut Vec<EnvelopeError>,
) {
    match (receipt.valid_from_ns, receipt.valid_until_ns) {
        (Some(start), Some(end)) if end > start => {
            if start > binding.acquisition_start_ns || end <= binding.acquisition_end_ns {
                errors.push(EnvelopeError::ReceiptValidityDoesNotCoverAcquisition(
                    receipt.domain,
                ));
            }
        }
        (None, None) => {}
        _ => errors.push(EnvelopeError::MalformedValidityInterval(receipt.domain)),
    }
}

fn validity_covers_acquisition(receipt: &EvidenceReceipt, binding: &CandidateBinding) -> bool {
    matches!(
        (receipt.valid_from_ns, receipt.valid_until_ns),
        (Some(start), Some(end))
            if start <= binding.acquisition_start_ns && end > binding.acquisition_end_ns
    )
}

fn validate_sha256(value: &str, label: &'static str, errors: &mut Vec<EnvelopeError>) {
    let valid = value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
    if !valid {
        errors.push(EnvelopeError::MalformedSha256(label));
    }
}

fn same_string_set(left: &[String], right: &[String]) -> bool {
    let left: BTreeSet<&str> = left.iter().map(String::as_str).collect();
    let right: BTreeSet<&str> = right.iter().map(String::as_str).collect();
    left == right && left.len() == right.len()
}

fn push_bytes(output: &mut Vec<u8>, value: &[u8]) {
    output.extend_from_slice(&(value.len() as u64).to_le_bytes());
    output.extend_from_slice(value);
}

fn push_str(output: &mut Vec<u8>, value: &str) {
    push_bytes(output, value.as_bytes());
}

fn push_u64(output: &mut Vec<u8>, value: u64) {
    push_bytes(output, &value.to_le_bytes());
}

fn push_f64(output: &mut Vec<u8>, value: f64) {
    let normalized = if value == 0.0 { 0.0 } else { value };
    push_bytes(output, &normalized.to_bits().to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const D: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";

    fn binding() -> CandidateBinding {
        CandidateBinding {
            candidate_id: "candidate-01".to_owned(),
            pass_id: "pass-north-01".to_owned(),
            mission_id: "mission-pv-01".to_owned(),
            mission_revision: "mission-r2".to_owned(),
            request_set_id: "evidence-request-set-01".to_owned(),
            request_set_revision: "request-set-r3".to_owned(),
            request_set_sha256: C.to_owned(),
            scenario_id: "scenario-roof-01".to_owned(),
            scenario_revision: "scenario-r3".to_owned(),
            robot_id: "uav-01".to_owned(),
            robot_configuration_revision: "uav-config-r4".to_owned(),
            robot_configuration_sha256: A.to_owned(),
            module_manifest_sha256: B.to_owned(),
            site_id: "roof-01".to_owned(),
            site_revision: "site-r5".to_owned(),
            frame_id: "array-plane-uv".to_owned(),
            frame_revision: "frame-r3".to_owned(),
            map_revision: "map-r6".to_owned(),
            timebase_id: "flight-clock-01".to_owned(),
            timebase_revision: "clock-map-r1".to_owned(),
            acquisition_start_ns: 1_000,
            acquisition_end_ns: 2_000,
            sensor_id: "rgb-ir-payload-01".to_owned(),
            sensor_revision: "sensor-r7".to_owned(),
            mount_id: "gimbal-mount-01".to_owned(),
            mount_revision: "mount-r2".to_owned(),
            calibration_bundle_sha256: C.to_owned(),
            observation_bundle_sha256: D.to_owned(),
            target_ids: vec!["module-01".to_owned(), "module-02".to_owned()],
            position_u_m: 3.0,
            position_v_m: 4.0,
            view_angle_rad: 0.65,
            cost_model_id: "survey-cost-model".to_owned(),
            cost_model_revision: "cost-r1".to_owned(),
            cost_model_sha256: A.to_owned(),
            estimated_acquisition_time_s: 4.0,
            estimated_energy_wh: 2.5,
            risk_penalty_s: 0.5,
        }
    }

    fn receipt(
        domain: EvidenceDomain,
        record_id: &str,
        result_sha256: &str,
        status: EvidenceStatus,
    ) -> EvidenceReceipt {
        let binding = binding();
        EvidenceReceipt {
            domain,
            timebase_id: binding.timebase_id.clone(),
            timebase_revision: binding.timebase_revision.clone(),
            record_id: record_id.to_owned(),
            issuer_id: format!("evaluator-{domain:?}"),
            issuer_revision: "evaluator-r1".to_owned(),
            profile_id: format!("profile-{domain:?}"),
            profile_revision: "profile-r2".to_owned(),
            status,
            binding_sha256: A.to_owned(),
            result_sha256: result_sha256.to_owned(),
            source_bundle_sha256: match domain {
                EvidenceDomain::GeometryCoverage | EvidenceDomain::SensorQuality => {
                    binding.observation_bundle_sha256
                }
                EvidenceDomain::Calibration => binding.calibration_bundle_sha256,
            },
            issued_at_ns: match domain {
                EvidenceDomain::GeometryCoverage | EvidenceDomain::SensorQuality => 2_500,
                EvidenceDomain::Calibration => 800,
            },
            valid_from_ns: (domain == EvidenceDomain::Calibration).then_some(500),
            valid_until_ns: (domain == EvidenceDomain::Calibration).then_some(3_000),
            covered_target_ids: if domain == EvidenceDomain::GeometryCoverage {
                binding.target_ids
            } else {
                Vec::new()
            },
        }
    }

    fn envelope() -> CandidateEvidenceEnvelope {
        CandidateEvidenceEnvelope {
            schema_version: SCHEMA_VERSION.to_owned(),
            binding: binding(),
            binding_sha256: A.to_owned(),
            receipts: vec![
                receipt(
                    EvidenceDomain::GeometryCoverage,
                    "geometry-record-01",
                    B,
                    EvidenceStatus::Qualified,
                ),
                receipt(
                    EvidenceDomain::SensorQuality,
                    "sensor-record-01",
                    C,
                    EvidenceStatus::Qualified,
                ),
                receipt(
                    EvidenceDomain::Calibration,
                    "calibration-record-01",
                    D,
                    EvidenceStatus::Qualified,
                ),
            ],
        }
    }

    #[test]
    fn all_three_qualified_receipts_make_candidate_eligible() {
        let value = envelope();
        let result = validate_candidate_envelope(&value, A).unwrap();
        assert_eq!(result.eligibility(), CandidateEligibility::Qualified);
        assert!(result.assessment().non_qualified_domains.is_empty());
        assert!(result.assessment().unknown_domains.is_empty());
        assert!(result.assessment().invalid_domains.is_empty());
    }

    #[test]
    fn not_qualified_receipt_is_not_promoted() {
        let mut value = envelope();
        value.receipts[1].status = EvidenceStatus::NotQualified;
        let result = validate_candidate_envelope(&value, A).unwrap();
        assert_eq!(result.eligibility(), CandidateEligibility::NotQualified);
        assert_eq!(
            result.assessment().non_qualified_domains,
            vec![EvidenceDomain::SensorQuality]
        );
    }

    #[test]
    fn unknown_receipt_remains_unknown() {
        let mut value = envelope();
        value.receipts[0].status = EvidenceStatus::Unknown;
        let result = validate_candidate_envelope(&value, A).unwrap();
        assert_eq!(result.eligibility(), CandidateEligibility::Unknown);
    }

    #[test]
    fn invalid_evidence_has_highest_blocking_precedence() {
        let mut value = envelope();
        value.receipts[0].status = EvidenceStatus::Unknown;
        value.receipts[2].status = EvidenceStatus::InvalidEvidence;
        let result = validate_candidate_envelope(&value, A).unwrap();
        assert_eq!(result.eligibility(), CandidateEligibility::InvalidEvidence);
        assert_eq!(
            result.assessment().invalid_domains,
            vec![EvidenceDomain::Calibration]
        );
    }

    #[test]
    fn only_validated_fully_qualified_candidate_has_planner_projection() {
        let value = envelope();
        let qualified = validate_candidate_envelope(&value, A).unwrap();
        let projection = qualified.qualified_projection().unwrap();
        assert_eq!(projection.candidate_id, "candidate-01");
        assert_eq!(projection.covered_target_ids.len(), 2);

        let mut unknown = envelope();
        unknown.receipts[1].status = EvidenceStatus::Unknown;
        let validated_unknown = validate_candidate_envelope(&unknown, A).unwrap();
        assert!(validated_unknown.qualified_projection().is_none());
    }

    #[test]
    fn receipt_timebase_must_match_candidate_binding() {
        let mut value = envelope();
        value.receipts[0].timebase_revision = "other-clock-revision".to_owned();
        assert!(
            validate_candidate_envelope(&value, A)
                .unwrap_err()
                .contains(&EnvelopeError::ReceiptTimebaseMismatch(
                    EvidenceDomain::GeometryCoverage
                ))
        );
    }

    #[test]
    fn unsupported_schema_is_rejected() {
        let mut value = envelope();
        value.schema_version = "robotics-candidate-evidence-envelope/v9".to_owned();
        assert!(
            validate_candidate_envelope(&value, A)
                .unwrap_err()
                .contains(&EnvelopeError::UnsupportedSchemaVersion)
        );
    }

    #[test]
    fn externally_computed_binding_digest_must_match_envelope_commitment() {
        let value = envelope();
        assert!(
            validate_candidate_envelope(&value, B)
                .unwrap_err()
                .contains(&EnvelopeError::ComputedBindingDigestMismatch)
        );
    }

    #[test]
    fn receipt_binding_digest_must_match_exact_candidate_context() {
        let mut value = envelope();
        value.receipts[0].binding_sha256 = B.to_owned();
        assert!(
            validate_candidate_envelope(&value, A)
                .unwrap_err()
                .contains(&EnvelopeError::BindingDigestMismatch(
                    EvidenceDomain::GeometryCoverage
                ))
        );
    }

    #[test]
    fn geometry_receipt_must_cover_exact_declared_targets() {
        let mut value = envelope();
        value.receipts[0].covered_target_ids.pop();
        assert!(
            validate_candidate_envelope(&value, A)
                .unwrap_err()
                .contains(&EnvelopeError::GeometryCoverageMismatch)
        );
    }

    #[test]
    fn geometry_receipt_cannot_claim_an_unlisted_target() {
        let mut value = envelope();
        value.receipts[0]
            .covered_target_ids
            .push("module-unknown".to_owned());
        assert!(
            validate_candidate_envelope(&value, A)
                .unwrap_err()
                .contains(&EnvelopeError::GeometryCoverageMismatch)
        );
    }

    #[test]
    fn sensor_and_calibration_receipts_cannot_claim_geometry_targets() {
        let mut value = envelope();
        value.receipts[1]
            .covered_target_ids
            .push("module-01".to_owned());
        assert!(
            validate_candidate_envelope(&value, A)
                .unwrap_err()
                .contains(&EnvelopeError::UnexpectedReceiptTargetScope(
                    EvidenceDomain::SensorQuality
                ))
        );
    }

    #[test]
    fn calibration_validity_must_cover_the_whole_acquisition_interval() {
        let mut value = envelope();
        value.receipts[2].valid_from_ns = Some(1_500);
        assert!(
            validate_candidate_envelope(&value, A)
                .unwrap_err()
                .contains(&EnvelopeError::CalibrationNotValidForAcquisition)
        );
    }

    #[test]
    fn geometry_result_must_be_issued_after_acquisition() {
        let mut value = envelope();
        value.receipts[0].issued_at_ns = 1_999;
        assert!(
            validate_candidate_envelope(&value, A)
                .unwrap_err()
                .contains(&EnvelopeError::ReceiptIssuedBeforeAcquisition(
                    EvidenceDomain::GeometryCoverage
                ))
        );
    }

    #[test]
    fn all_required_evidence_domains_must_be_present() {
        let mut value = envelope();
        value.receipts.pop();
        assert!(
            validate_candidate_envelope(&value, A)
                .unwrap_err()
                .contains(&EnvelopeError::UnexpectedReceiptCount)
        );
    }

    #[test]
    fn duplicate_receipt_domains_are_rejected() {
        let mut value = envelope();
        value.receipts[1].domain = EvidenceDomain::GeometryCoverage;
        assert!(
            validate_candidate_envelope(&value, A)
                .unwrap_err()
                .contains(&EnvelopeError::DuplicateReceiptDomain(
                    EvidenceDomain::GeometryCoverage
                ))
        );
    }

    #[test]
    fn duplicate_receipt_ids_are_rejected() {
        let mut value = envelope();
        value.receipts[1].record_id = value.receipts[0].record_id.clone();
        assert!(
            validate_candidate_envelope(&value, A)
                .unwrap_err()
                .contains(&EnvelopeError::DuplicateReceiptId(
                    "geometry-record-01".to_owned()
                ))
        );
    }

    #[test]
    fn duplicate_result_digests_are_rejected() {
        let mut value = envelope();
        value.receipts[1].result_sha256 = value.receipts[0].result_sha256.clone();
        assert!(
            validate_candidate_envelope(&value, A)
                .unwrap_err()
                .iter()
                .any(|error| matches!(error, EnvelopeError::DuplicateResultDigest(_)))
        );
    }

    #[test]
    fn source_bundle_must_match_bound_observation_or_calibration() {
        let mut value = envelope();
        value.receipts[1].source_bundle_sha256 = A.to_owned();
        assert!(
            validate_candidate_envelope(&value, A)
                .unwrap_err()
                .contains(&EnvelopeError::SourceBundleMismatch(
                    EvidenceDomain::SensorQuality
                ))
        );
    }

    #[test]
    fn non_finite_coordinates_are_rejected() {
        let mut value = envelope();
        value.binding.position_u_m = f64::NAN;
        assert!(
            validate_candidate_envelope(&value, A)
                .unwrap_err()
                .contains(&EnvelopeError::InvalidNumericValue("position_u_m"))
        );
    }

    #[test]
    fn invalid_acquisition_interval_is_rejected() {
        let mut value = envelope();
        value.binding.acquisition_end_ns = value.binding.acquisition_start_ns;
        assert!(
            validate_candidate_envelope(&value, A)
                .unwrap_err()
                .contains(&EnvelopeError::InvalidAcquisitionInterval)
        );
    }

    #[test]
    fn malformed_digest_is_rejected() {
        let mut value = envelope();
        value.binding.calibration_bundle_sha256 = "not-a-digest".to_owned();
        assert!(
            validate_candidate_envelope(&value, A)
                .unwrap_err()
                .contains(&EnvelopeError::MalformedSha256("calibration_bundle_sha256"))
        );
    }

    #[test]
    fn duplicate_target_ids_are_rejected() {
        let mut value = envelope();
        value.binding.target_ids.push("module-01".to_owned());
        assert!(
            validate_candidate_envelope(&value, A)
                .unwrap_err()
                .contains(&EnvelopeError::DuplicateTarget("module-01".to_owned()))
        );
    }

    #[test]
    fn target_order_does_not_change_canonical_binding_bytes() {
        let mut first = binding();
        let mut second = binding();
        second.target_ids.reverse();
        assert_eq!(
            first.canonical_binding_bytes(),
            second.canonical_binding_bytes()
        );
        first.position_u_m = -0.0;
        second.position_u_m = 0.0;
        assert_eq!(
            first.canonical_binding_bytes(),
            second.canonical_binding_bytes()
        );
    }

    #[test]
    fn critical_revision_changes_invalidate_old_domain_receipts() {
        fn assert_rebinding_required(
            mut value: CandidateEvidenceEnvelope,
            mutate: impl FnOnce(&mut CandidateBinding),
        ) {
            mutate(&mut value.binding);
            value.binding_sha256 = B.to_owned();
            let errors = validate_candidate_envelope(&value, B).unwrap_err();
            assert!(errors.contains(&EnvelopeError::BindingDigestMismatch(
                EvidenceDomain::GeometryCoverage
            )));
            assert!(errors.contains(&EnvelopeError::BindingDigestMismatch(
                EvidenceDomain::SensorQuality
            )));
            assert!(errors.contains(&EnvelopeError::BindingDigestMismatch(
                EvidenceDomain::Calibration
            )));
        }

        assert_rebinding_required(envelope(), |binding| {
            binding.mission_revision = "mission-r-next".to_owned();
        });
        assert_rebinding_required(envelope(), |binding| {
            binding.request_set_revision = "request-set-r-next".to_owned();
        });
        assert_rebinding_required(envelope(), |binding| {
            binding.request_set_sha256 = D.to_owned();
        });
        assert_rebinding_required(envelope(), |binding| {
            binding.scenario_revision = "scenario-r-next".to_owned();
        });
        assert_rebinding_required(envelope(), |binding| {
            binding.robot_configuration_revision = "robot-config-r-next".to_owned();
        });
        assert_rebinding_required(envelope(), |binding| {
            binding.module_manifest_sha256 = B.to_owned();
        });
        assert_rebinding_required(envelope(), |binding| {
            binding.site_revision = "site-r-next".to_owned();
        });
        assert_rebinding_required(envelope(), |binding| {
            binding.frame_revision = "frame-r-next".to_owned();
        });
        assert_rebinding_required(envelope(), |binding| {
            binding.map_revision = "map-r-next".to_owned();
        });
        assert_rebinding_required(envelope(), |binding| {
            binding.timebase_revision = "clock-map-r-next".to_owned();
        });
        assert_rebinding_required(envelope(), |binding| {
            binding.sensor_revision = "sensor-r-next".to_owned();
        });
        assert_rebinding_required(envelope(), |binding| {
            binding.mount_revision = "mount-r-next".to_owned();
        });
        assert_rebinding_required(envelope(), |binding| {
            binding.calibration_bundle_sha256 = C.to_owned();
        });
        assert_rebinding_required(envelope(), |binding| {
            binding.observation_bundle_sha256 = D.to_owned();
        });
    }

    #[test]
    fn calibration_digest_change_changes_canonical_binding_bytes() {
        let first = binding();
        let mut second = binding();
        second.calibration_bundle_sha256 = B.to_owned();
        assert_ne!(
            first.canonical_binding_bytes(),
            second.canonical_binding_bytes()
        );
    }
}
