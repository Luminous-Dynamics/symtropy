use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt::Write as _;
use std::collections::HashSet;

const FIXTURE_SCHEMA: &str = "luminous.orbital-interface-conformance.corpus.v0.1";
const SYNTHETIC_CLAIM_CEILING: &str = "synthetic-profile-only";

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ServiceKind {
    CaptureOnly,
    CaptureAndStorableFluid,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DecisionStatus {
    CompatibleSyntheticProfile,
    Rejected,
    Indeterminate,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Fixture {
    pub schema: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub cases: Vec<ConformanceCase>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConformanceCase {
    pub case_id: String,
    pub at_tick: u64,
    pub context_of_use: String,
    pub service: ServiceKind,
    pub provider: InterfaceProfile,
    pub client: InterfaceProfile,
    #[serde(default)]
    pub compatibility_map: Option<CompatibilityMap>,
    pub expected_status: DecisionStatus,
    pub expected_reason: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InterfaceProfile {
    pub profile_id: Option<String>,
    pub role: Option<String>,
    pub organization_id: Option<String>,
    pub config_revision: Option<String>,
    pub icd_id: Option<String>,
    pub icd_revision: Option<String>,
    #[serde(default)]
    pub standard_refs: Vec<String>,
    pub frame_id: Option<String>,
    pub mechanical: Option<MechanicalProfile>,
    pub fluid: Option<FluidProfile>,
    pub evidence: Option<EvidenceEnvelope>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MechanicalProfile {
    pub geometry_profile_id: Option<String>,
    pub max_alignment_error: Option<Scalar>,
    pub max_approach_speed: Option<Scalar>,
    pub max_contact_force: Option<Scalar>,
    pub min_retention_force: Option<Scalar>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FluidProfile {
    pub connector_profile_id: Option<String>,
    pub fluid_class: Option<String>,
    pub fluid_id: Option<String>,
    pub phase: Option<String>,
    pub temperature: Option<Interval>,
    pub pressure: Option<Interval>,
    pub min_receiver_accepted_mass: Option<Scalar>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Scalar {
    pub value: Option<f64>,
    pub unit: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Interval {
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub unit: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EvidenceEnvelope {
    pub evidence_class: Option<String>,
    pub claim_ceiling: Option<String>,
    pub source_commit: Option<String>,
    pub artifact_sha256: Option<String>,
    pub producer_id: Option<String>,
    pub authority_scope: Option<String>,
    pub context_of_use: Option<String>,
    pub valid_from_tick: Option<u64>,
    pub valid_until_tick: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CompatibilityMap {
    pub icd_id: Option<String>,
    pub provider_revision: Option<String>,
    pub client_revision: Option<String>,
    pub verified: Option<bool>,
    pub evidence: Option<EvidenceEnvelope>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaseResult {
    pub case_id: String,
    pub actual_status: DecisionStatus,
    pub actual_reason: String,
    pub expected_status: DecisionStatus,
    pub expected_reason: String,
    pub expected_result_matched: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub case_count: usize,
    pub compatible_synthetic_profile: usize,
    pub rejected: usize,
    pub indeterminate: usize,
    pub expected_result_mismatches: usize,
    pub authority_grants: usize,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Decision {
    status: DecisionStatus,
    reason: &'static str,
}

impl Decision {
    const fn compatible() -> Self {
        Self {
            status: DecisionStatus::CompatibleSyntheticProfile,
            reason: "all_requirements_contained",
        }
    }

    const fn rejected(reason: &'static str) -> Self {
        Self {
            status: DecisionStatus::Rejected,
            reason,
        }
    }

    const fn indeterminate(reason: &'static str) -> Self {
        Self {
            status: DecisionStatus::Indeterminate,
            reason,
        }
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("writing to String is infallible");
    }
    output
}

pub fn is_hex_digest(value: &str, expected_len: usize) -> bool {
    value.len() == expected_len
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub fn is_valid_source_commit(value: &str) -> bool {
    is_hex_digest(value, 40)
}

fn present(value: &Option<String>) -> bool {
    value.as_ref().is_some_and(|text| !text.trim().is_empty())
}

fn validate_evidence(
    evidence: &Option<EvidenceEnvelope>,
    expected_context: &str,
    at_tick: u64,
) -> Result<(), Decision> {
    let Some(evidence) = evidence else {
        return Err(Decision::indeterminate("missing_evidence_envelope"));
    };

    if !present(&evidence.claim_ceiling) {
        return Err(Decision::indeterminate("missing_evidence_claim_ceiling"));
    }
    if evidence.claim_ceiling.as_deref() != Some(SYNTHETIC_CLAIM_CEILING) {
        return Err(Decision::rejected("claim_ceiling_escalation"));
    }

    if !present(&evidence.artifact_sha256)
        || !present(&evidence.source_commit)
        || !present(&evidence.evidence_class)
        || !present(&evidence.context_of_use)
        || evidence.valid_from_tick.is_none()
        || evidence.valid_until_tick.is_none()
    {
        return Err(Decision::indeterminate("incomplete_evidence_envelope"));
    }

    let artifact_digest = evidence.artifact_sha256.as_deref().unwrap_or_default();
    if !is_hex_digest(artifact_digest, 64) {
        return Err(Decision::rejected("malformed_evidence_digest"));
    }
    let source_commit = evidence.source_commit.as_deref().unwrap_or_default();
    if !is_valid_source_commit(source_commit) {
        return Err(Decision::rejected("malformed_source_commit"));
    }

    if evidence.evidence_class.as_deref() != Some("synthetic") {
        return Err(Decision::indeterminate("unsupported_evidence_class"));
    }
    if evidence.context_of_use.as_deref() != Some(expected_context) {
        return Err(Decision::indeterminate("evidence_context_mismatch"));
    }

    let valid_from = evidence.valid_from_tick.unwrap_or_default();
    let valid_until = evidence.valid_until_tick.unwrap_or_default();
    if valid_from > valid_until {
        return Err(Decision::rejected("inverted_evidence_validity"));
    }
    if at_tick < valid_from || at_tick > valid_until {
        return Err(Decision::indeterminate("evidence_not_effective"));
    }

    if !present(&evidence.producer_id) || !present(&evidence.authority_scope) {
        return Err(Decision::indeterminate("missing_evidence_scope"));
    }

    Ok(())
}

fn validate_profile_identity(profile: &InterfaceProfile) -> Result<(), Decision> {
    if !present(&profile.profile_id)
        || !present(&profile.role)
        || !present(&profile.organization_id)
        || !present(&profile.config_revision)
        || !present(&profile.icd_id)
        || !present(&profile.icd_revision)
        || !present(&profile.frame_id)
    {
        return Err(Decision::indeterminate("incomplete_profile_identity"));
    }
    Ok(())
}

fn validate_scalar(value: &Option<Scalar>, expected_unit: &str) -> Result<f64, Decision> {
    let Some(quantity) = value else {
        return Err(Decision::indeterminate("missing_mechanical_requirement"));
    };
    let Some(unit) = quantity.unit.as_deref() else {
        return Err(Decision::indeterminate("missing_quantity_unit"));
    };
    if unit != expected_unit {
        return Err(Decision::rejected("unsupported_unit"));
    }
    let Some(number) = quantity.value else {
        return Err(Decision::indeterminate("missing_quantity_value"));
    };
    if !number.is_finite() || number < 0.0 {
        return Err(Decision::rejected("invalid_numeric_value"));
    }
    Ok(number)
}

fn validate_interval(value: &Option<Interval>, expected_unit: &str) -> Result<(f64, f64), Decision> {
    let Some(interval) = value else {
        return Err(Decision::indeterminate("missing_fluid_requirement"));
    };
    let Some(unit) = interval.unit.as_deref() else {
        return Err(Decision::indeterminate("missing_quantity_unit"));
    };
    if unit != expected_unit {
        return Err(Decision::rejected("unsupported_unit"));
    }
    let (Some(min), Some(max)) = (interval.min, interval.max) else {
        return Err(Decision::indeterminate("incomplete_interval"));
    };
    if !min.is_finite() || !max.is_finite() || min < 0.0 || max < 0.0 {
        return Err(Decision::rejected("invalid_numeric_value"));
    }
    if min > max {
        return Err(Decision::rejected("inverted_range"));
    }
    Ok((min, max))
}

fn compare_mechanical(
    provider: &Option<MechanicalProfile>,
    client: &Option<MechanicalProfile>,
) -> Result<(), Decision> {
    let (Some(provider), Some(client)) = (provider, client) else {
        return Err(Decision::indeterminate("missing_mechanical_profile"));
    };

    if !present(&provider.geometry_profile_id) || !present(&client.geometry_profile_id) {
        return Err(Decision::indeterminate("missing_geometry_profile"));
    }
    if provider.geometry_profile_id != client.geometry_profile_id {
        return Err(Decision::rejected("geometry_profile_mismatch"));
    }

    let provider_alignment = validate_scalar(&provider.max_alignment_error, "m")?;
    let client_alignment = validate_scalar(&client.max_alignment_error, "m")?;
    if provider_alignment < client_alignment {
        return Err(Decision::rejected("alignment_envelope_not_contained"));
    }

    let provider_speed = validate_scalar(&provider.max_approach_speed, "m/s")?;
    let client_speed = validate_scalar(&client.max_approach_speed, "m/s")?;
    if provider_speed < client_speed {
        return Err(Decision::rejected("approach_speed_not_supported"));
    }

    let provider_contact = validate_scalar(&provider.max_contact_force, "N")?;
    let client_contact = validate_scalar(&client.max_contact_force, "N")?;
    if provider_contact < client_contact {
        return Err(Decision::rejected("contact_force_envelope_not_contained"));
    }

    let provider_retention = validate_scalar(&provider.min_retention_force, "N")?;
    let client_retention = validate_scalar(&client.min_retention_force, "N")?;
    if provider_retention < client_retention {
        return Err(Decision::rejected("retention_requirement_not_met"));
    }

    Ok(())
}

fn compare_fluid(
    provider: &Option<FluidProfile>,
    client: &Option<FluidProfile>,
) -> Result<(), Decision> {
    let (Some(provider), Some(client)) = (provider, client) else {
        return Err(Decision::indeterminate("missing_fluid_profile"));
    };

    for (provider_value, client_value, reason) in [
        (
            &provider.connector_profile_id,
            &client.connector_profile_id,
            "fluid_connector_mismatch",
        ),
        (&provider.fluid_class, &client.fluid_class, "fluid_class_mismatch"),
        (&provider.fluid_id, &client.fluid_id, "fluid_identity_mismatch"),
        (&provider.phase, &client.phase, "fluid_phase_mismatch"),
    ] {
        if !present(provider_value) || !present(client_value) {
            return Err(Decision::indeterminate("incomplete_fluid_identity"));
        }
        if provider_value != client_value {
            return Err(Decision::rejected(reason));
        }
    }

    let (provider_temp_min, provider_temp_max) =
        validate_interval(&provider.temperature, "K")?;
    let (client_temp_min, client_temp_max) =
        validate_interval(&client.temperature, "K")?;
    if provider_temp_min > client_temp_min || provider_temp_max < client_temp_max {
        return Err(Decision::rejected("fluid_temperature_envelope_not_contained"));
    }

    let (provider_pressure_min, provider_pressure_max) =
        validate_interval(&provider.pressure, "Pa")?;
    let (client_pressure_min, client_pressure_max) =
        validate_interval(&client.pressure, "Pa")?;
    if provider_pressure_min > client_pressure_min || provider_pressure_max < client_pressure_max {
        return Err(Decision::rejected("fluid_pressure_envelope_not_contained"));
    }

    let provider_mass =
        validate_scalar(&provider.min_receiver_accepted_mass, "kg")?;
    let client_mass = validate_scalar(&client.min_receiver_accepted_mass, "kg")?;
    if provider_mass < client_mass {
        return Err(Decision::rejected("receiver_accepted_mass_not_supported"));
    }

    Ok(())
}

fn check_icd_revision(case: &ConformanceCase) -> Result<(), Decision> {
    if case.provider.icd_id != case.client.icd_id {
        return Err(Decision::rejected("icd_id_mismatch"));
    }
    if case.provider.icd_revision == case.client.icd_revision {
        return Ok(());
    }

    let Some(mapping) = &case.compatibility_map else {
        return Err(Decision::indeterminate("icd_revision_unmapped"));
    };
    if mapping.verified != Some(true)
        || mapping.icd_id != case.provider.icd_id
        || mapping.provider_revision != case.provider.icd_revision
        || mapping.client_revision != case.client.icd_revision
    {
        return Err(Decision::indeterminate("icd_revision_mapping_unverified"));
    }
    validate_evidence(&mapping.evidence, &case.context_of_use, case.at_tick)
}

fn evaluate_case(case: &ConformanceCase) -> Decision {
    if case.case_id.trim().is_empty() {
        return Decision::rejected("empty_case_id");
    }

    if let Err(decision) = validate_profile_identity(&case.provider) {
        return decision;
    }
    if let Err(decision) = validate_profile_identity(&case.client) {
        return decision;
    }

    if case.provider.role.as_deref() != Some("provider")
        || case.client.role.as_deref() != Some("client")
    {
        return Decision::rejected("profile_role_mismatch");
    }
    if case.provider.organization_id == case.client.organization_id
        || case.provider.profile_id == case.client.profile_id
    {
        return Decision::rejected("self_compatibility_not_admissible");
    }

    for profile in [&case.provider, &case.client] {
        if let Err(decision) =
            validate_evidence(&profile.evidence, &case.context_of_use, case.at_tick)
        {
            return decision;
        }
    }

    if let Err(decision) = check_icd_revision(case) {
        return decision;
    }

    if case.provider.frame_id != case.client.frame_id {
        return Decision::indeterminate("frame_mapping_unavailable");
    }

    if let Err(decision) =
        compare_mechanical(&case.provider.mechanical, &case.client.mechanical)
    {
        return decision;
    }

    if case.service == ServiceKind::CaptureAndStorableFluid {
        if let Err(decision) = compare_fluid(&case.provider.fluid, &case.client.fluid) {
            return decision;
        }
    }

    Decision::compatible()
}

pub fn evaluate(fixture: &Fixture) -> Result<Report, String> {
    if fixture.schema != FIXTURE_SCHEMA {
        return Err(format!("unsupported fixture schema: {}", fixture.schema));
    }
    if fixture.evidence_class != "synthetic" {
        return Err("v0.1 accepts synthetic fixture evidence only".to_owned());
    }
    if fixture.claim_ceiling != SYNTHETIC_CLAIM_CEILING {
        return Err("fixture claim ceiling exceeds v0.1 scope".to_owned());
    }
    if fixture.cases.is_empty() {
        return Err("fixture must contain at least one case".to_owned());
    }

    let mut seen = HashSet::new();
    let mut results = Vec::with_capacity(fixture.cases.len());
    let mut compatible = 0;
    let mut rejected = 0;
    let mut indeterminate = 0;
    let mut mismatches = 0;

    for case in &fixture.cases {
        if !seen.insert(case.case_id.as_str()) {
            return Err(format!("duplicate case id: {}", case.case_id));
        }
        let actual = evaluate_case(case);
        match actual.status {
            DecisionStatus::CompatibleSyntheticProfile => compatible += 1,
            DecisionStatus::Rejected => rejected += 1,
            DecisionStatus::Indeterminate => indeterminate += 1,
        }
        let matched = actual.status == case.expected_status
            && actual.reason == case.expected_reason;
        if !matched {
            mismatches += 1;
        }
        results.push(CaseResult {
            case_id: case.case_id.clone(),
            actual_status: actual.status,
            actual_reason: actual.reason.to_owned(),
            expected_status: case.expected_status,
            expected_reason: case.expected_reason.clone(),
            expected_result_matched: matched,
        });
    }

    let summary = Summary {
        case_count: fixture.cases.len(),
        compatible_synthetic_profile: compatible,
        rejected,
        indeterminate,
        expected_result_mismatches: mismatches,
        authority_grants: 0,
    };
    Ok(Report {
        result: if mismatches == 0 { "pass" } else { "fail" }.to_owned(),
        evidence_class: "synthetic".to_owned(),
        claim_ceiling: SYNTHETIC_CLAIM_CEILING.to_owned(),
        authority_granted: false,
        summary,
        cases: results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Fixture {
        serde_json::from_str(include_str!("../fixtures/v0.1/cases.json"))
            .expect("frozen interface corpus must parse")
    }

    #[test]
    fn frozen_corpus_matches_expected_dispositions() {
        let report = evaluate(&fixture()).expect("fixture evaluates");
        assert_eq!(report.result, "pass");
        assert_eq!(report.summary.expected_result_mismatches, 0);
        assert_eq!(report.summary.authority_grants, 0);
        assert!(!report.authority_granted);
        assert!(report.summary.case_count >= 24);
    }

    #[test]
    fn no_compatibility_result_grants_operational_authority() {
        let report = evaluate(&fixture()).expect("fixture evaluates");
        assert!(!report.authority_granted);
        assert_eq!(report.summary.authority_grants, 0);
    }

    #[test]
    fn input_order_does_not_change_case_dispositions() {
        let forward = fixture();
        let mut reverse = fixture();
        reverse.cases.reverse();
        let first = evaluate(&forward).expect("forward fixture evaluates");
        let second = evaluate(&reverse).expect("reverse fixture evaluates");
        let mut first_rows = first.cases;
        let mut second_rows = second.cases;
        first_rows.sort_by(|left, right| left.case_id.cmp(&right.case_id));
        second_rows.sort_by(|left, right| left.case_id.cmp(&right.case_id));
        assert_eq!(
            serde_json::to_value(first_rows).unwrap(),
            serde_json::to_value(second_rows).unwrap()
        );
    }

    #[test]
    fn full_containment_not_partial_overlap_is_required() {
        let report = evaluate(&fixture()).expect("fixture evaluates");
        let temperature = report
            .cases
            .iter()
            .find(|row| row.case_id == "FLUID_TEMPERATURE_PARTIAL_OVERLAP")
            .unwrap();
        let pressure = report
            .cases
            .iter()
            .find(|row| row.case_id == "FLUID_PRESSURE_PARTIAL_OVERLAP")
            .unwrap();
        assert_eq!(temperature.actual_status, DecisionStatus::Rejected);
        assert_eq!(pressure.actual_status, DecisionStatus::Rejected);
    }

    #[test]
    fn verified_revision_mapping_is_explicit_and_scoped() {
        let report = evaluate(&fixture()).expect("fixture evaluates");
        let mapped = report
            .cases
            .iter()
            .find(|row| row.case_id == "ICD_REVISION_VERIFIED_MAPPING")
            .unwrap();
        let unmapped = report
            .cases
            .iter()
            .find(|row| row.case_id == "ICD_REVISION_WITHOUT_MAPPING")
            .unwrap();
        assert_eq!(mapped.actual_status, DecisionStatus::CompatibleSyntheticProfile);
        assert_eq!(unmapped.actual_status, DecisionStatus::Indeterminate);
    }

    #[test]
    fn non_finite_measurement_is_rejected() {
        let mut data = fixture();
        let mechanical = data.cases[0]
            .provider
            .mechanical
            .as_mut()
            .expect("provider mechanical profile");
        mechanical.max_alignment_error = Some(Scalar {
            value: Some(f64::NAN),
            unit: Some("m".to_owned()),
        });
        let result = evaluate_case(&data.cases[0]);
        assert_eq!(result.status, DecisionStatus::Rejected);
        assert_eq!(result.reason, "invalid_numeric_value");
    }

    #[test]
    fn malformed_source_commit_is_rejected() {
        let mut data = fixture();
        let evidence = data.cases[0]
            .provider
            .evidence
            .as_mut()
            .expect("provider evidence");
        evidence.source_commit = Some("main".to_owned());
        assert_eq!(
            evaluate_case(&data.cases[0]).status,
            DecisionStatus::Rejected
        );
    }

    #[test]
    fn sha256_is_lowercase_and_byte_bound() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert!(is_valid_source_commit(
            "0123456789abcdef0123456789abcdef01234567"
        ));
        assert!(!is_valid_source_commit("0123456789abcdef0123456789abcdef0123456g"));
    }
}
