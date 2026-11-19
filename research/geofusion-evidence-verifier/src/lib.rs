#![forbid(unsafe_code)]

//! Runtime verifier for one source-interval evidence envelope and its allocation receipt.
//!
//! This verifies exact raw bytes, identity joins, carrier/service compatibility and
//! source-interval arithmetic. It does not validate the full JSON Schema, calibration
//! truth, source authenticity, solver physics, or global uniqueness across receipts.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationError {
    InvalidJson(&'static str),
    MissingField(&'static str),
    InvalidField(&'static str),
    UnsupportedSchemaVersion,
    UnsupportedEvidenceKind,
    ArtifactUnavailable,
    ArtifactIntegrityStateRejected,
    EvidenceNotCurrent,
    UnsupportedDigestAlgorithm,
    MalformedDigest,
    DigestMismatch,
    ByteLengthMismatch,
    SourceIdentityMismatch(&'static str),
    TimeWindowMismatch(&'static str),
    UnsupportedCarrier,
    InvalidEnergy(&'static str),
    DuplicateAllocationId,
    ServiceCarrierMismatch,
    ArithmeticOverflow,
    DeclaredEvaluationMismatch(&'static str),
}

impl VerificationError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidJson(_) => "INVALID_JSON",
            Self::MissingField(_) => "MISSING_FIELD",
            Self::InvalidField(_) => "INVALID_FIELD",
            Self::UnsupportedSchemaVersion => "UNSUPPORTED_SCHEMA_VERSION",
            Self::UnsupportedEvidenceKind => "UNSUPPORTED_EVIDENCE_KIND",
            Self::ArtifactUnavailable => "ARTIFACT_UNAVAILABLE",
            Self::ArtifactIntegrityStateRejected => "ARTIFACT_INTEGRITY_STATE_REJECTED",
            Self::EvidenceNotCurrent => "EVIDENCE_NOT_CURRENT",
            Self::UnsupportedDigestAlgorithm => "UNSUPPORTED_DIGEST_ALGORITHM",
            Self::MalformedDigest => "MALFORMED_DIGEST",
            Self::DigestMismatch => "DIGEST_MISMATCH",
            Self::ByteLengthMismatch => "BYTE_LENGTH_MISMATCH",
            Self::SourceIdentityMismatch(_) => "SOURCE_IDENTITY_MISMATCH",
            Self::TimeWindowMismatch(_) => "TIME_WINDOW_MISMATCH",
            Self::UnsupportedCarrier => "UNSUPPORTED_CARRIER",
            Self::InvalidEnergy(_) => "INVALID_ENERGY",
            Self::DuplicateAllocationId => "DUPLICATE_ALLOCATION_ID",
            Self::ServiceCarrierMismatch => "SERVICE_CARRIER_MISMATCH",
            Self::ArithmeticOverflow => "ARITHMETIC_OVERFLOW",
            Self::DeclaredEvaluationMismatch(_) => "DECLARED_EVALUATION_MISMATCH",
        }
    }
}

impl fmt::Display for VerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidJson(label)
            | Self::MissingField(label)
            | Self::InvalidField(label)
            | Self::SourceIdentityMismatch(label)
            | Self::TimeWindowMismatch(label)
            | Self::InvalidEnergy(label)
            | Self::DeclaredEvaluationMismatch(label) => {
                write!(f, "{}: {}", self.code(), label)
            }
            _ => write!(f, "{}", self.code()),
        }
    }
}

impl std::error::Error for VerificationError {}

#[derive(Debug, Clone, PartialEq)]
pub struct VerificationReport {
    pub evidence_id: String,
    pub allocation_receipt_id: String,
    pub source_asset_id: String,
    pub source_boundary_id: String,
    pub source_interval_id: String,
    pub carrier: String,
    pub payload_sha256: String,
    pub payload_byte_length: u64,
    pub integrity_verified: bool,
    pub allocated_energy_mwh: f64,
    pub curtailed_energy_mwh: f64,
    /// Positive means unallocated source energy; negative means over-allocation.
    pub residual_energy_mwh: f64,
    pub normalized_absolute_residual: f64,
    pub within_tolerance: bool,
    /// A qualified allocation receipt requires both byte integrity and balance closure.
    pub qualified: bool,
}

impl VerificationReport {
    pub fn to_json(&self) -> Value {
        json!({
            "evidence_id": self.evidence_id,
            "allocation_receipt_id": self.allocation_receipt_id,
            "source_asset_id": self.source_asset_id,
            "source_boundary_id": self.source_boundary_id,
            "source_interval_id": self.source_interval_id,
            "carrier": self.carrier,
            "payload_sha256": self.payload_sha256,
            "payload_byte_length": self.payload_byte_length,
            "integrity_verified": self.integrity_verified,
            "allocated_energy_mwh": self.allocated_energy_mwh,
            "curtailed_energy_mwh": self.curtailed_energy_mwh,
            "residual_energy_mwh": self.residual_energy_mwh,
            "normalized_absolute_residual": self.normalized_absolute_residual,
            "within_tolerance": self.within_tolerance,
            "qualified": self.qualified,
            "claim_ceiling": concat!(
                "Byte identity, declared source identity and one source-interval allocation arithmetic only; ",
                "not physical truth or operational authorization."
            )
        })
    }
}

fn required<'a>(
    value: &'a Value,
    key: &'static str,
    context: &'static str,
) -> Result<&'a Value, VerificationError> {
    value
        .as_object()
        .ok_or(VerificationError::InvalidField(context))?
        .get(key)
        .ok_or(VerificationError::MissingField(key))
}

fn string_field<'a>(
    value: &'a Value,
    key: &'static str,
    context: &'static str,
) -> Result<&'a str, VerificationError> {
    required(value, key, context)?
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .ok_or(VerificationError::InvalidField(key))
}

fn number_field(
    value: &Value,
    key: &'static str,
    context: &'static str,
) -> Result<f64, VerificationError> {
    let number = required(value, key, context)?
        .as_f64()
        .ok_or(VerificationError::InvalidField(key))?;
    if !number.is_finite() {
        return Err(VerificationError::InvalidEnergy(key));
    }
    Ok(number)
}

fn unsigned_field(
    value: &Value,
    key: &'static str,
    context: &'static str,
) -> Result<u64, VerificationError> {
    required(value, key, context)?
        .as_u64()
        .ok_or(VerificationError::InvalidField(key))
}

fn same_text(
    left: &str,
    right: &str,
    field: &'static str,
) -> Result<(), VerificationError> {
    if left == right {
        Ok(())
    } else {
        Err(VerificationError::SourceIdentityMismatch(field))
    }
}

fn same_number(left: f64, right: f64) -> bool {
    let scale = left.abs().max(right.abs()).max(1.0);
    (left - right).abs() <= 8.0 * f64::EPSILON * scale
}

fn carrier_supported(carrier: &str) -> bool {
    matches!(carrier, "heat" | "electricity")
}

fn service_matches_carrier(service: &str, carrier: &str) -> bool {
    match service {
        "dt_plasma_heating"
        | "fusion_electrical_auxiliary"
        | "facility_electrical_load"
        | "grid_export"
        | "reverse_osmosis_electrical_load" => carrier == "electricity",
        "industrial_process_heat"
        | "facility_heat"
        | "thermal_storage_charge"
        | "power_cycle_thermal_input"
        | "absorption_chiller_drive_heat"
        | "thermal_desalination_heat" => carrier == "heat",
        "other" => true,
        _ => false,
    }
}

fn digest_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Verify a raw source-interval payload, its evidence envelope and one allocation receipt.
///
/// The raw payload bytes are hashed exactly as received; they are never parsed and
/// re-serialized before hashing. JSON parsing occurs only after byte integrity is checked.
pub fn verify_source_interval(
    payload_bytes: &[u8],
    evidence: &Value,
    allocation: &Value,
) -> Result<VerificationReport, VerificationError> {
    let payload: Value = serde_json::from_slice(payload_bytes)
        .map_err(|_| VerificationError::InvalidJson("source payload"))?;

    if string_field(evidence, "schema_version", "evidence envelope")? != "0.1.0"
        || string_field(allocation, "schema_version", "allocation receipt")? != "0.1.0"
    {
        return Err(VerificationError::UnsupportedSchemaVersion);
    }
    if string_field(evidence, "evidence_kind", "evidence envelope")? != "source_interval" {
        return Err(VerificationError::UnsupportedEvidenceKind);
    }

    let payload_class = string_field(&payload, "evidence_class", "source payload")?;
    let evidence_class = string_field(evidence, "evidence_class", "evidence envelope")?;
    if !matches!(
        evidence_class,
        "synthetic"
            | "published_design"
            | "vendor_reported"
            | "measured"
            | "calibrated_model"
            | "independently_validated_model"
            | "audited_operation"
    ) {
        return Err(VerificationError::InvalidField("evidence_class"));
    }
    same_text(payload_class, evidence_class, "evidence_class")?;

    let validity = required(evidence, "validity", "evidence envelope")?;
    match string_field(validity, "state", "validity")? {
        "current" | "not_applicable" => {}
        "stale" | "unknown" => return Err(VerificationError::EvidenceNotCurrent),
        _ => return Err(VerificationError::InvalidField("validity.state")),
    }

    let measurement = required(evidence, "measurement", "evidence envelope")?;
    let model = required(evidence, "model", "evidence envelope")?;
    if matches!(evidence_class, "measured" | "audited_operation") {
        if string_field(measurement, "applicability", "measurement")? != "applicable" {
            return Err(VerificationError::InvalidField("measurement.applicability"));
        }
        let instruments = required(measurement, "instrument_ids", "measurement")?
            .as_array()
            .filter(|items| !items.is_empty())
            .ok_or(VerificationError::InvalidField("measurement.instrument_ids"))?;
        if instruments.iter().any(|item| item.as_str().is_none_or(|id| id.trim().is_empty())) {
            return Err(VerificationError::InvalidField("measurement.instrument_ids"));
        }
        string_field(measurement, "calibration_certificate_id", "measurement")?;
    }
    if matches!(evidence_class, "calibrated_model" | "independently_validated_model") {
        if string_field(model, "applicability", "model")? != "applicable"
            || string_field(model, "convergence_state", "model")? != "converged"
        {
            return Err(VerificationError::InvalidField("model.convergence_state"));
        }
        string_field(model, "solver_id", "model")?;
        string_field(model, "solver_version", "model")?;
    }

    let artifact = required(evidence, "artifact", "evidence envelope")?;
    if string_field(artifact, "availability", "artifact")? != "available" {
        return Err(VerificationError::ArtifactUnavailable);
    }
    string_field(artifact, "locator", "artifact")?;
    if string_field(artifact, "media_type", "artifact")? != "application/json" {
        return Err(VerificationError::InvalidField("artifact.media_type"));
    }
    match string_field(artifact, "integrity_state", "artifact")? {
        "not_checked" | "verified" => {}
        _ => return Err(VerificationError::ArtifactIntegrityStateRejected),
    }

    let digest = required(artifact, "digest", "artifact")?;
    if string_field(digest, "algorithm", "artifact digest")? != "sha256" {
        return Err(VerificationError::UnsupportedDigestAlgorithm);
    }
    let declared_digest = string_field(digest, "value", "artifact digest")?;
    if declared_digest.len() != 64
        || !declared_digest
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(VerificationError::MalformedDigest);
    }

    let actual_digest = digest_hex(payload_bytes);
    if actual_digest != declared_digest {
        return Err(VerificationError::DigestMismatch);
    }
    let actual_length = u64::try_from(payload_bytes.len())
        .map_err(|_| VerificationError::ArithmeticOverflow)?;
    if unsigned_field(artifact, "byte_length", "artifact")? != actual_length {
        return Err(VerificationError::ByteLengthMismatch);
    }

    let subject = required(evidence, "subject", "evidence envelope")?;
    if string_field(subject, "kind", "subject")? != "physical_asset" {
        return Err(VerificationError::InvalidField("subject.kind"));
    }
    let asset_id = string_field(&payload, "asset_id", "source payload")?;
    same_text(
        string_field(subject, "subject_id", "subject")?,
        asset_id,
        "subject.subject_id",
    )?;
    same_text(
        string_field(subject, "asset_id", "subject")?,
        asset_id,
        "subject.asset_id",
    )?;

    let boundary_id = string_field(&payload, "measurement_boundary_id", "source payload")?;
    same_text(
        string_field(subject, "boundary_id", "subject")?,
        boundary_id,
        "subject.boundary_id",
    )?;
    let interval_id = string_field(&payload, "interval_id", "source payload")?;
    same_text(
        string_field(subject, "interval_id", "subject")?,
        interval_id,
        "subject.interval_id",
    )?;

    let carrier = string_field(&payload, "carrier", "source payload")?;
    if !carrier_supported(carrier) {
        return Err(VerificationError::UnsupportedCarrier);
    }
    same_text(
        string_field(subject, "carrier", "subject")?,
        carrier,
        "subject.carrier",
    )?;

    let time_window = required(evidence, "time_window", "evidence envelope")?;
    if string_field(time_window, "state", "time_window")? != "bounded" {
        return Err(VerificationError::TimeWindowMismatch("state must be bounded"));
    }
    if !matches!(
        string_field(time_window, "clock_quality", "time_window")?,
        "synchronized" | "estimated" | "synthetic" | "unknown" | "not_applicable"
    ) {
        return Err(VerificationError::InvalidField("time_window.clock_quality"));
    }
    let _clock_uncertainty_ns = unsigned_field(
        time_window,
        "clock_uncertainty_ns",
        "time_window",
    )?;
    same_text(
        string_field(time_window, "start_utc", "time_window")?,
        string_field(&payload, "start_utc", "source payload")?,
        "time_window.start_utc",
    )
    .map_err(|_| VerificationError::TimeWindowMismatch("start_utc"))?;
    if unsigned_field(time_window, "duration_ns", "time_window")?
        != unsigned_field(&payload, "duration_ns", "source payload")?
    {
        return Err(VerificationError::TimeWindowMismatch("duration_ns"));
    }

    let evidence_id = string_field(evidence, "evidence_id", "evidence envelope")?;
    same_text(
        string_field(allocation, "source_evidence_id", "allocation receipt")?,
        evidence_id,
        "source_evidence_id",
    )?;

    let receipt_digest = required(allocation, "source_payload_digest", "allocation receipt")?;
    same_text(
        string_field(receipt_digest, "algorithm", "receipt digest")?,
        "sha256",
        "source_payload_digest.algorithm",
    )?;
    same_text(
        string_field(receipt_digest, "value", "receipt digest")?,
        actual_digest.as_str(),
        "source_payload_digest.value",
    )?;

    same_text(
        string_field(allocation, "source_asset_id", "allocation receipt")?,
        asset_id,
        "source_asset_id",
    )?;
    same_text(
        string_field(allocation, "source_boundary_id", "allocation receipt")?,
        boundary_id,
        "source_boundary_id",
    )?;
    same_text(
        string_field(allocation, "source_interval_id", "allocation receipt")?,
        interval_id,
        "source_interval_id",
    )?;
    same_text(
        string_field(allocation, "carrier", "allocation receipt")?,
        carrier,
        "allocation.carrier",
    )?;

    let available = number_field(&payload, "available_energy_mwh", "source payload")?;
    if available < 0.0 {
        return Err(VerificationError::InvalidEnergy("available_energy_mwh"));
    }
    let receipt_available =
        number_field(allocation, "available_energy_mwh", "allocation receipt")?;
    if receipt_available < 0.0 || !same_number(available, receipt_available) {
        return Err(VerificationError::SourceIdentityMismatch(
            "available_energy_mwh",
        ));
    }

    let allocations = required(allocation, "allocations", "allocation receipt")?
        .as_array()
        .ok_or(VerificationError::InvalidField("allocations"))?;
    let mut allocation_ids = BTreeSet::new();
    let mut allocated = 0.0_f64;
    for edge in allocations {
        let allocation_id = string_field(edge, "allocation_id", "allocation edge")?;
        if !allocation_ids.insert(allocation_id) {
            return Err(VerificationError::DuplicateAllocationId);
        }
        string_field(edge, "destination_asset_id", "allocation edge")?;
        let edge_carrier = string_field(edge, "carrier", "allocation edge")?;
        same_text(edge_carrier, carrier, "allocation.carrier")?;
        let service = string_field(edge, "service", "allocation edge")?;
        if !service_matches_carrier(service, carrier) {
            return Err(VerificationError::ServiceCarrierMismatch);
        }
        let energy = number_field(edge, "energy_mwh", "allocation edge")?;
        if energy <= 0.0 {
            return Err(VerificationError::InvalidEnergy(
                "allocation.energy_mwh",
            ));
        }
        allocated += energy;
        if !allocated.is_finite() {
            return Err(VerificationError::ArithmeticOverflow);
        }
    }

    let curtailed = number_field(
        allocation,
        "curtailed_energy_mwh",
        "allocation receipt",
    )?;
    if curtailed < 0.0 {
        return Err(VerificationError::InvalidEnergy(
            "curtailed_energy_mwh",
        ));
    }

    let evaluation = required(allocation, "evaluation", "allocation receipt")?;
    let tolerance = number_field(evaluation, "relative_tolerance", "evaluation")?;
    if tolerance <= 0.0 || tolerance >= 1.0 {
        return Err(VerificationError::InvalidField(
            "evaluation.relative_tolerance",
        ));
    }

    // Match the source-allocation ledger's scale-normalized first-law residual.
    let scale = available
        .abs()
        .max(allocated.abs())
        .max(curtailed.abs())
        .max(f64::MIN_POSITIVE);
    let scaled_residual = available / scale - allocated / scale - curtailed / scale;
    let normalized_denominator = available / scale + allocated / scale + curtailed / scale;
    let normalized = if normalized_denominator == 0.0 {
        0.0
    } else {
        scaled_residual.abs() / normalized_denominator
    };
    let residual = scaled_residual * scale;
    if !scaled_residual.is_finite() || !normalized.is_finite() || !residual.is_finite() {
        return Err(VerificationError::ArithmeticOverflow);
    }
    let within_tolerance = normalized <= tolerance;

    if !same_number(
        number_field(evaluation, "allocated_energy_mwh", "evaluation")?,
        allocated,
    ) {
        return Err(VerificationError::DeclaredEvaluationMismatch(
            "allocated_energy_mwh",
        ));
    }
    if !same_number(
        number_field(evaluation, "residual_energy_mwh", "evaluation")?,
        residual,
    ) {
        return Err(VerificationError::DeclaredEvaluationMismatch(
            "residual_energy_mwh",
        ));
    }
    if !same_number(
        number_field(
            evaluation,
            "normalized_absolute_residual",
            "evaluation",
        )?,
        normalized,
    ) {
        return Err(VerificationError::DeclaredEvaluationMismatch(
            "normalized_absolute_residual",
        ));
    }
    let declared_within = required(evaluation, "within_tolerance", "evaluation")?
        .as_bool()
        .ok_or(VerificationError::InvalidField(
            "evaluation.within_tolerance",
        ))?;
    if declared_within != within_tolerance {
        return Err(VerificationError::DeclaredEvaluationMismatch(
            "within_tolerance",
        ));
    }
    let expected_disposition = if within_tolerance {
        "within_tolerance"
    } else {
        "unbalanced"
    };
    same_text(
        string_field(evaluation, "disposition", "evaluation")?,
        expected_disposition,
        "evaluation.disposition",
    )
    .map_err(|_| {
        VerificationError::DeclaredEvaluationMismatch("disposition")
    })?;

    let receipt_id = string_field(allocation, "receipt_id", "allocation receipt")?;
    Ok(VerificationReport {
        evidence_id: evidence_id.to_owned(),
        allocation_receipt_id: receipt_id.to_owned(),
        source_asset_id: asset_id.to_owned(),
        source_boundary_id: boundary_id.to_owned(),
        source_interval_id: interval_id.to_owned(),
        carrier: carrier.to_owned(),
        payload_sha256: actual_digest,
        payload_byte_length: actual_length,
        integrity_verified: true,
        allocated_energy_mwh: allocated,
        curtailed_energy_mwh: curtailed,
        residual_energy_mwh: residual,
        normalized_absolute_residual: normalized,
        within_tolerance,
        qualified: within_tolerance,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const PAYLOAD: &[u8] = include_bytes!(
        "../../../docs/research/geothermal-fusion/evidence/fixtures/synthetic-geothermal-source-interval.v0.1.json"
    );
    const EVIDENCE: &str = include_str!(
        "../../../docs/research/geothermal-fusion/evidence/fixtures/synthetic-source-interval-evidence.v0.1.json"
    );
    const ALLOCATION: &str = include_str!(
        "../../../docs/research/geothermal-fusion/evidence/fixtures/synthetic-source-allocation-receipt.v0.1.json"
    );

    fn evidence() -> Value {
        serde_json::from_str(EVIDENCE).expect("evidence fixture JSON")
    }

    fn allocation() -> Value {
        serde_json::from_str(ALLOCATION).expect("allocation fixture JSON")
    }

    #[test]
    fn source_interval_fixture_verifies_exact_bytes_and_allocation() {
        let report = verify_source_interval(PAYLOAD, &evidence(), &allocation()).unwrap();
        assert!(report.integrity_verified);
        assert!(report.within_tolerance);
        assert!(report.qualified);
        assert_eq!(report.payload_byte_length, 523);
        assert_eq!(
            report.payload_sha256,
            "0cdcab2b9625b3669918761b7a4da378e9d344321301f2dd0bb48feb67b788c8"
        );
        assert_eq!(report.allocated_energy_mwh, 9.0);
        assert_eq!(report.curtailed_energy_mwh, 1.0);
        assert_eq!(report.residual_energy_mwh, 0.0);
    }

    #[test]
    fn mutated_payload_bytes_are_rejected() {
        let mut bytes = PAYLOAD.to_vec();
        bytes.push(b' ');
        let error = verify_source_interval(&bytes, &evidence(), &allocation()).unwrap_err();
        assert_eq!(error.code(), "DIGEST_MISMATCH");
    }

    #[test]
    fn declared_payload_length_is_recomputed() {
        let mut e = evidence();
        e["artifact"]["byte_length"] = json!(524);
        let error = verify_source_interval(PAYLOAD, &e, &allocation()).unwrap_err();
        assert_eq!(error.code(), "BYTE_LENGTH_MISMATCH");
    }

    #[test]
    fn source_interval_identity_must_match_all_three_records() {
        let mut e = evidence();
        e["subject"]["interval_id"] = json!("wrong-interval");
        let error = verify_source_interval(PAYLOAD, &e, &allocation()).unwrap_err();
        assert_eq!(error.code(), "SOURCE_IDENTITY_MISMATCH");

        let mut a = allocation();
        a["source_interval_id"] = json!("wrong-interval");
        let error = verify_source_interval(PAYLOAD, &evidence(), &a).unwrap_err();
        assert_eq!(error.code(), "SOURCE_IDENTITY_MISMATCH");
    }

    #[test]
    fn wrong_measurement_boundary_is_rejected() {
        let mut a = allocation();
        a["source_boundary_id"] = json!("wrong-boundary");
        let error = verify_source_interval(PAYLOAD, &evidence(), &a).unwrap_err();
        assert_eq!(error.code(), "SOURCE_IDENTITY_MISMATCH");
    }

    #[test]
    fn evidence_class_cannot_be_promoted_from_synthetic_to_measured() {
        let mut e = evidence();
        e["evidence_class"] = json!("measured");
        let error = verify_source_interval(PAYLOAD, &e, &allocation()).unwrap_err();
        assert_eq!(error.code(), "SOURCE_IDENTITY_MISMATCH");
    }

    #[test]
    fn duplicate_allocation_ids_are_rejected() {
        let mut a = allocation();
        let first_id = a["allocations"][0]["allocation_id"].clone();
        a["allocations"][1]["allocation_id"] = first_id;
        let error = verify_source_interval(PAYLOAD, &evidence(), &a).unwrap_err();
        assert_eq!(error.code(), "DUPLICATE_ALLOCATION_ID");
    }

    #[test]
    fn service_and_carrier_mismatch_is_rejected() {
        let mut a = allocation();
        a["allocations"][0]["service"] = json!("reverse_osmosis_electrical_load");
        let error = verify_source_interval(PAYLOAD, &evidence(), &a).unwrap_err();
        assert_eq!(error.code(), "SERVICE_CARRIER_MISMATCH");
    }

    #[test]
    fn over_allocation_is_exposed_as_unqualified_with_negative_residual() {
        let mut a = allocation();
        a["allocations"][1]["energy_mwh"] = json!(5.0);
        a["evaluation"]["allocated_energy_mwh"] = json!(11.0);
        a["evaluation"]["residual_energy_mwh"] = json!(-2.0);
        a["evaluation"]["normalized_absolute_residual"] = json!(2.0 / 22.0);
        a["evaluation"]["within_tolerance"] = json!(false);
        a["evaluation"]["disposition"] = json!("unbalanced");

        let report = verify_source_interval(PAYLOAD, &evidence(), &a).unwrap();
        assert!(report.integrity_verified);
        assert!(!report.within_tolerance);
        assert!(!report.qualified);
        assert!((report.residual_energy_mwh + 2.0).abs() < 1e-12);
    }

    #[test]
    fn forged_balanced_evaluation_is_rejected() {
        let mut a = allocation();
        a["evaluation"]["allocated_energy_mwh"] = json!(8.0);
        let error = verify_source_interval(PAYLOAD, &evidence(), &a).unwrap_err();
        assert_eq!(error.code(), "DECLARED_EVALUATION_MISMATCH");
    }

    #[test]
    fn evidence_and_allocation_digest_must_match_the_recomputed_digest() {
        let mut a = allocation();
        a["source_payload_digest"]["value"] = json!("0".repeat(64));
        let error = verify_source_interval(PAYLOAD, &evidence(), &a).unwrap_err();
        assert_eq!(error.code(), "SOURCE_IDENTITY_MISMATCH");
    }

    #[test]
    fn unsupported_integrity_claim_is_rejected() {
        let mut e = evidence();
        e["artifact"]["integrity_state"] = json!("mismatch");
        let error = verify_source_interval(PAYLOAD, &e, &allocation()).unwrap_err();
        assert_eq!(error.code(), "ARTIFACT_INTEGRITY_STATE_REJECTED");
    }

    #[test]
    fn stale_or_unknown_validity_cannot_be_qualified() {
        let mut e = evidence();
        e["validity"]["state"] = json!("unknown");
        e["validity"]["reason"] = json!("calibration/validity evidence absent");
        let error = verify_source_interval(PAYLOAD, &e, &allocation()).unwrap_err();
        assert_eq!(error.code(), "EVIDENCE_NOT_CURRENT");
    }

    #[test]
    fn bounded_time_window_must_match_the_raw_source_interval() {
        let mut e = evidence();
        e["time_window"]["start_utc"] = json!("2026-01-01T01:00:00Z");
        let error = verify_source_interval(PAYLOAD, &e, &allocation()).unwrap_err();
        assert_eq!(error.code(), "TIME_WINDOW_MISMATCH");
    }

    #[test]
    fn unsupported_digest_algorithm_is_rejected() {
        let mut e = evidence();
        e["artifact"]["digest"]["algorithm"] = json!("md5");
        let error = verify_source_interval(PAYLOAD, &e, &allocation()).unwrap_err();
        assert_eq!(error.code(), "UNSUPPORTED_DIGEST_ALGORITHM");
    }

    #[test]
    fn malformed_digest_is_rejected_before_comparison() {
        let mut e = evidence();
        e["artifact"]["digest"]["value"] = json!("z".repeat(64));
        let error = verify_source_interval(PAYLOAD, &e, &allocation()).unwrap_err();
        assert_eq!(error.code(), "MALFORMED_DIGEST");
    }

    #[test]
    fn zero_relative_tolerance_is_rejected_like_the_rust_contract() {
        let mut a = allocation();
        a["evaluation"]["relative_tolerance"] = json!(0.0);
        let error = verify_source_interval(PAYLOAD, &evidence(), &a).unwrap_err();
        assert_eq!(error.code(), "INVALID_FIELD");
    }
}
