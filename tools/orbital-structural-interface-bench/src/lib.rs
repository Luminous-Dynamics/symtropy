use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const CORPUS_SCHEMA: &str = "luminous.orbital-structural-interface.corpus.v0.1";
pub const RECEIPT_SCHEMA: &str = "luminous.orbital-structural-interface.receipt.v0.1";
pub const CLAIM_CEILING: &str = "synthetic_structural_profile_compatibility_only";

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    pub schema: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub base_provider: Value,
    pub base_client: Value,
    pub cases: Vec<CaseSpec>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaseSpec {
    pub case_id: String,
    #[serde(default)]
    pub mutations: Vec<Mutation>,
    pub expected: Expected,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mutation {
    pub target: String,
    pub operation: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub value: Value,
    #[serde(default)]
    pub evidence_id: Option<String>,
    #[serde(default)]
    pub field: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Expected {
    pub disposition: String,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub profile_id: String,
    pub profile_revision: String,
    pub icd_id: String,
    pub icd_revision: String,
    pub configuration_id: String,
    pub article_subject_ref: String,
    pub frame_id: String,
    pub units_profile_id: String,
    pub geometry_profile_id: String,
    pub mating_feature_id: String,
    pub retention_feature_id: String,
    pub max_pose_error_mm: f64,
    pub tooling_profile_id: String,
    pub independent_lock_verification: bool,
    pub verification_method_id: Option<String>,
    pub verification_evidence_ref: Option<String>,
    pub load_cases: Vec<LoadCase>,
    pub max_deflection_mm: f64,
    pub max_slip_mm: f64,
    pub minimum_retention_n: f64,
    pub temperature_min_k: f64,
    pub temperature_max_k: f64,
    pub maximum_service_cycles: u64,
    pub material_spec_id: String,
    pub material_condition_id: String,
    pub material_lot_id: String,
    pub process_route_id: String,
    pub material_lineage_ref: Option<String>,
    pub material_evidence_ref: Option<String>,
    pub inspection_method_id: String,
    pub critical_surface_observable: bool,
    pub inspection_evidence_ref: Option<String>,
    pub replacement_isolated_and_reverified: bool,
    pub requires_replacement: bool,
    pub requires_fatigue: bool,
    pub fatigue_evidence_ref: Option<String>,
    pub requires_assembled_path: bool,
    pub assembled_path_evidence_ref: Option<String>,
    pub requires_independent_evidence: bool,
    pub uncertainty_basis: String,
    #[serde(default)]
    pub evidence: Vec<Evidence>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoadCase {
    pub case_id: String,
    pub interaction_profile_ref: String,
    pub frame_id: String,
    pub limits: LoadVector,
    pub evidence_ref: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoadVector {
    pub axial_n: f64,
    pub shear_y_n: f64,
    pub shear_z_n: f64,
    pub bending_y_nm: f64,
    pub bending_z_nm: f64,
    pub torsion_nm: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub evidence_id: String,
    pub kind: String,
    pub evidence_class: String,
    pub status: String,
    pub sha256: String,
    pub subject_ref: String,
    pub configuration_id: String,
    pub icd_id: String,
    pub icd_revision: String,
    pub dependency_group: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaseResult {
    pub case_id: String,
    pub disposition: String,
    pub reasons: Vec<String>,
    pub expected_matched: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Receipt {
    pub schema: String,
    pub input_sha256: String,
    pub source_commit: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub authority_granted: bool,
    pub case_count: usize,
    pub matched_cases: usize,
    pub mismatched_cases: usize,
    pub results: Vec<CaseResult>,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Severity {
    Indeterminate,
    Incompatible,
    Malformed,
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

fn valid_commit(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

fn set_path(root: &mut Value, path: &str, replacement: Value) -> Result<(), String> {
    let segments: Vec<&str> = path.split('.').collect();
    if segments.is_empty() || segments.iter().any(|s| s.is_empty()) {
        return Err("mutation path must contain non-empty object keys".to_owned());
    }
    let mut cursor = root;
    for segment in &segments[..segments.len() - 1] {
        cursor = cursor
            .as_object_mut()
            .and_then(|object| object.get_mut(*segment))
            .ok_or_else(|| format!("mutation path parent not found: {segment}"))?;
    }
    let last = segments[segments.len() - 1];
    let object = cursor
        .as_object_mut()
        .ok_or_else(|| format!("mutation path parent is not an object: {path}"))?;
    if !object.contains_key(last) {
        return Err(format!("mutation field not found: {path}"));
    }
    object.insert(last.to_owned(), replacement);
    Ok(())
}

fn apply_mutation(
    provider: &mut Value,
    client: &mut Value,
    mutation: &Mutation,
) -> Result<(), String> {
    let target = match mutation.target.as_str() {
        "provider" => provider,
        "client" => client,
        other => return Err(format!("unsupported mutation target: {other}")),
    };
    match mutation.operation.as_str() {
        "set" => {
            let path = mutation
                .path
                .as_deref()
                .ok_or("set mutation requires path")?;
            let value = mutation.value.clone();
            set_path(target, path, value)
        }
        "remove_evidence" => {
            let id = mutation
                .evidence_id
                .as_deref()
                .ok_or("remove_evidence requires evidence_id")?;
            let records = target
                .get_mut("evidence")
                .and_then(Value::as_array_mut)
                .ok_or("evidence collection missing")?;
            let before = records.len();
            records.retain(|record| record.get("evidence_id").and_then(Value::as_str) != Some(id));
            if before == records.len() {
                return Err(format!("evidence record not found: {id}"));
            }
            Ok(())
        }
        "set_evidence" => {
            let id = mutation
                .evidence_id
                .as_deref()
                .ok_or("set_evidence requires evidence_id")?;
            let field = mutation
                .field
                .as_deref()
                .ok_or("set_evidence requires field")?;
            let value = mutation.value.clone();
            let records = target
                .get_mut("evidence")
                .and_then(Value::as_array_mut)
                .ok_or("evidence collection missing")?;
            let record = records
                .iter_mut()
                .find(|record| record.get("evidence_id").and_then(Value::as_str) == Some(id))
                .ok_or_else(|| format!("evidence record not found: {id}"))?;
            let object = record
                .as_object_mut()
                .ok_or("evidence record must be object")?;
            if !object.contains_key(field) {
                return Err(format!("evidence field not found: {field}"));
            }
            object.insert(field.to_owned(), value);
            Ok(())
        }
        "reverse_array" => {
            let path = mutation
                .path
                .as_deref()
                .ok_or("reverse_array requires path")?;
            let array = target
                .get_mut(path)
                .and_then(Value::as_array_mut)
                .ok_or_else(|| format!("array not found: {path}"))?;
            array.reverse();
            Ok(())
        }
        other => Err(format!("unsupported mutation operation: {other}")),
    }
}

fn add_reason(reasons: &mut BTreeMap<String, Severity>, code: &str, severity: Severity) {
    reasons
        .entry(code.to_owned())
        .and_modify(|old| *old = (*old).max(severity))
        .or_insert(severity);
}

fn profile_evidence<'a>(
    profile: &'a Profile,
    reference: Option<&str>,
    required_kind: &str,
    required_class: &str,
    reason_prefix: &str,
    reasons: &mut BTreeMap<String, Severity>,
) -> Option<&'a Evidence> {
    let Some(reference) = reference.filter(|value| !value.trim().is_empty()) else {
        add_reason(
            reasons,
            &format!("{reason_prefix}_EVIDENCE_MISSING"),
            Severity::Indeterminate,
        );
        return None;
    };
    let Some(evidence) = profile
        .evidence
        .iter()
        .find(|item| item.evidence_id == reference)
    else {
        add_reason(
            reasons,
            "EVIDENCE_REFERENCE_MISSING",
            Severity::Indeterminate,
        );
        return None;
    };
    let expected_subject = if required_kind == "material_process" {
        profile.material_lineage_ref.as_deref()
    } else {
        Some(profile.article_subject_ref.as_str())
    };
    if expected_subject.is_some_and(|subject| subject != evidence.subject_ref.as_str())
        || evidence.configuration_id != profile.configuration_id
        || evidence.icd_id != profile.icd_id
        || evidence.icd_revision != profile.icd_revision
    {
        add_reason(reasons, "EVIDENCE_SCOPE_MISMATCH", Severity::Malformed);
    }
    if evidence.status == "stale" {
        add_reason(reasons, "EVIDENCE_STALE", Severity::Malformed);
        return Some(evidence);
    }
    if evidence.status == "disputed" {
        add_reason(reasons, "EVIDENCE_DISPUTED", Severity::Indeterminate);
    }
    if evidence.kind != required_kind || evidence.evidence_class != required_class {
        add_reason(
            reasons,
            "EVIDENCE_CLASS_INSUFFICIENT",
            Severity::Indeterminate,
        );
    }
    Some(evidence)
}

fn vector_finite_nonnegative(v: &LoadVector) -> bool {
    [
        v.axial_n,
        v.shear_y_n,
        v.shear_z_n,
        v.bending_y_nm,
        v.bending_z_nm,
        v.torsion_nm,
    ]
    .iter()
    .all(|value| value.is_finite() && *value >= 0.0)
}

fn valid_profile_numbers(profile: &Profile) -> bool {
    [
        profile.max_pose_error_mm,
        profile.max_deflection_mm,
        profile.max_slip_mm,
        profile.minimum_retention_n,
        profile.temperature_min_k,
        profile.temperature_max_k,
    ]
    .iter()
    .all(|value| value.is_finite() && *value >= 0.0)
        && profile.temperature_min_k <= profile.temperature_max_k
        && profile
            .load_cases
            .iter()
            .all(|case| vector_finite_nonnegative(&case.limits))
}

fn evaluate(provider: &Profile, client: &Profile) -> (String, Vec<String>) {
    let mut reasons = BTreeMap::<String, Severity>::new();

    if !valid_profile_numbers(provider) || !valid_profile_numbers(client) {
        add_reason(
            &mut reasons,
            "NUMERIC_PROFILE_MALFORMED",
            Severity::Malformed,
        );
    }

    let mut evidence_ids = BTreeSet::new();
    for evidence in &provider.evidence {
        if evidence.evidence_id.trim().is_empty()
            || !evidence_ids.insert(evidence.evidence_id.as_str())
        {
            add_reason(
                &mut reasons,
                "EVIDENCE_ID_DUPLICATE_OR_EMPTY",
                Severity::Malformed,
            );
        }
        if !valid_sha256(&evidence.sha256) {
            add_reason(&mut reasons, "EVIDENCE_DIGEST_INVALID", Severity::Malformed);
        }
        match evidence.status.as_str() {
            "current" => {}
            "stale" => add_reason(&mut reasons, "EVIDENCE_STALE", Severity::Malformed),
            "disputed" => add_reason(&mut reasons, "EVIDENCE_DISPUTED", Severity::Indeterminate),
            _ => add_reason(
                &mut reasons,
                "EVIDENCE_STATUS_UNKNOWN",
                Severity::Indeterminate,
            ),
        }
    }

    for profile in [provider, client] {
        let mut load_case_ids = BTreeSet::new();
        for load_case in &profile.load_cases {
            if load_case.case_id.trim().is_empty()
                || !load_case_ids.insert(load_case.case_id.as_str())
            {
                add_reason(
                    &mut reasons,
                    "LOAD_CASE_ID_DUPLICATE_OR_EMPTY",
                    Severity::Malformed,
                );
            }
            if load_case.interaction_profile_ref.trim().is_empty() {
                add_reason(
                    &mut reasons,
                    "LOAD_INTERACTION_PROFILE_ID_EMPTY",
                    Severity::Indeterminate,
                );
            }
        }
    }
    if client.load_cases.is_empty() {
        add_reason(
            &mut reasons,
            "LOAD_REQUIREMENT_SET_EMPTY",
            Severity::Indeterminate,
        );
    }

    for value in [
        &provider.profile_id,
        &provider.profile_revision,
        &provider.icd_id,
        &provider.icd_revision,
        &provider.configuration_id,
        &provider.article_subject_ref,
        &provider.frame_id,
        &provider.units_profile_id,
        &provider.geometry_profile_id,
        &provider.mating_feature_id,
        &provider.retention_feature_id,
        &provider.tooling_profile_id,
        &client.profile_id,
        &client.profile_revision,
        &client.icd_id,
        &client.icd_revision,
        &client.configuration_id,
        &client.article_subject_ref,
        &client.frame_id,
        &client.units_profile_id,
        &client.geometry_profile_id,
        &client.mating_feature_id,
        &client.retention_feature_id,
        &client.tooling_profile_id,
        &provider.material_spec_id,
        &provider.material_condition_id,
        &provider.material_lot_id,
        &provider.process_route_id,
        &provider.inspection_method_id,
        &client.material_spec_id,
        &client.material_condition_id,
        &client.material_lot_id,
        &client.process_route_id,
        &client.inspection_method_id,
    ] {
        if value.trim().is_empty() {
            add_reason(&mut reasons, "IDENTITY_FIELD_EMPTY", Severity::Malformed);
        }
    }

    if provider.icd_id != client.icd_id || provider.icd_revision != client.icd_revision {
        add_reason(&mut reasons, "ICD_IDENTITY_MISMATCH", Severity::Malformed);
    }
    if provider.configuration_id != client.configuration_id {
        add_reason(
            &mut reasons,
            "CONFIGURATION_IDENTITY_MISMATCH",
            Severity::Malformed,
        );
    }
    if provider.frame_id != client.frame_id {
        add_reason(&mut reasons, "FRAME_MISMATCH", Severity::Indeterminate);
    }
    if provider.units_profile_id != client.units_profile_id {
        add_reason(
            &mut reasons,
            "UNITS_PROFILE_MISMATCH",
            Severity::Indeterminate,
        );
    }
    if provider.geometry_profile_id != client.geometry_profile_id {
        add_reason(
            &mut reasons,
            "GEOMETRY_PROFILE_MISMATCH",
            Severity::Incompatible,
        );
    }
    if provider.mating_feature_id != client.mating_feature_id {
        add_reason(
            &mut reasons,
            "MATING_FEATURE_MISMATCH",
            Severity::Incompatible,
        );
    }
    if provider.retention_feature_id != client.retention_feature_id {
        add_reason(
            &mut reasons,
            "RETENTION_FEATURE_MISMATCH",
            Severity::Incompatible,
        );
    }
    if provider.max_pose_error_mm > client.max_pose_error_mm {
        add_reason(
            &mut reasons,
            "POSE_TOLERANCE_EXCEEDED",
            Severity::Incompatible,
        );
    }
    if provider.tooling_profile_id != client.tooling_profile_id {
        add_reason(
            &mut reasons,
            "TOOLING_PROFILE_MISMATCH",
            Severity::Incompatible,
        );
    }
    if !provider.independent_lock_verification || !client.independent_lock_verification {
        add_reason(
            &mut reasons,
            "LOCK_VERIFICATION_UNAVAILABLE",
            Severity::Incompatible,
        );
    }
    if provider
        .verification_method_id
        .as_deref()
        .is_none_or(str::is_empty)
        || client
            .verification_method_id
            .as_deref()
            .is_none_or(str::is_empty)
    {
        add_reason(
            &mut reasons,
            "VERIFICATION_METHOD_MISSING",
            Severity::Indeterminate,
        );
    }
    let verification = profile_evidence(
        provider,
        provider.verification_evidence_ref.as_deref(),
        "verification",
        "independent_engagement_verification",
        "VERIFICATION",
        &mut reasons,
    );

    for required in &client.load_cases {
        let Some(offered) = provider
            .load_cases
            .iter()
            .find(|item| item.case_id == required.case_id)
        else {
            add_reason(&mut reasons, "LOAD_CASE_MISSING", Severity::Indeterminate);
            continue;
        };
        if offered.frame_id != required.frame_id {
            add_reason(&mut reasons, "LOAD_FRAME_MISMATCH", Severity::Indeterminate);
        }
        if offered.interaction_profile_ref != required.interaction_profile_ref {
            add_reason(
                &mut reasons,
                "LOAD_INTERACTION_PROFILE_MISMATCH",
                Severity::Indeterminate,
            );
        }
        if offered.limits.axial_n < required.limits.axial_n
            || offered.limits.shear_y_n < required.limits.shear_y_n
            || offered.limits.shear_z_n < required.limits.shear_z_n
            || offered.limits.bending_y_nm < required.limits.bending_y_nm
            || offered.limits.bending_z_nm < required.limits.bending_z_nm
            || offered.limits.torsion_nm < required.limits.torsion_nm
        {
            add_reason(
                &mut reasons,
                "LOAD_CAPACITY_INSUFFICIENT",
                Severity::Incompatible,
            );
        }
        profile_evidence(
            provider,
            offered.evidence_ref.as_deref(),
            "structural_capability",
            "scoped_test_or_validated_analysis",
            "STRUCTURAL",
            &mut reasons,
        );
    }

    if provider.max_deflection_mm > client.max_deflection_mm {
        add_reason(
            &mut reasons,
            "DEFLECTION_LIMIT_EXCEEDED",
            Severity::Incompatible,
        );
    }
    if provider.max_slip_mm > client.max_slip_mm {
        add_reason(&mut reasons, "SLIP_LIMIT_EXCEEDED", Severity::Incompatible);
    }
    if provider.minimum_retention_n < client.minimum_retention_n {
        add_reason(
            &mut reasons,
            "RETENTION_CAPACITY_INSUFFICIENT",
            Severity::Incompatible,
        );
    }
    if provider.temperature_min_k > client.temperature_min_k
        || provider.temperature_max_k < client.temperature_max_k
    {
        add_reason(
            &mut reasons,
            "TEMPERATURE_RANGE_INSUFFICIENT",
            Severity::Incompatible,
        );
    }
    if provider.maximum_service_cycles < client.maximum_service_cycles {
        add_reason(
            &mut reasons,
            "SERVICE_LIFE_INSUFFICIENT",
            Severity::Incompatible,
        );
    }

    if provider.material_spec_id != client.material_spec_id {
        add_reason(
            &mut reasons,
            "MATERIAL_SPEC_MISMATCH",
            Severity::Incompatible,
        );
    }
    if provider.material_condition_id != client.material_condition_id {
        add_reason(
            &mut reasons,
            "MATERIAL_CONDITION_MISMATCH",
            Severity::Incompatible,
        );
    }
    if provider.material_lot_id != client.material_lot_id {
        add_reason(
            &mut reasons,
            "MATERIAL_LOT_MISMATCH",
            Severity::Incompatible,
        );
    }
    if provider.process_route_id != client.process_route_id {
        add_reason(
            &mut reasons,
            "PROCESS_ROUTE_MISMATCH",
            Severity::Incompatible,
        );
    }
    if provider
        .material_lineage_ref
        .as_deref()
        .is_none_or(str::is_empty)
    {
        add_reason(
            &mut reasons,
            "MATERIAL_LINEAGE_MISSING",
            Severity::Indeterminate,
        );
    }
    let material = profile_evidence(
        provider,
        provider.material_evidence_ref.as_deref(),
        "material_process",
        "process_conditioned_material_characterization",
        "MATERIAL",
        &mut reasons,
    );

    if provider.inspection_method_id != client.inspection_method_id {
        add_reason(
            &mut reasons,
            "INSPECTION_METHOD_UNMAPPED",
            Severity::Indeterminate,
        );
    }
    if client.critical_surface_observable && !provider.critical_surface_observable {
        add_reason(
            &mut reasons,
            "CRITICAL_SURFACE_NOT_OBSERVABLE",
            Severity::Incompatible,
        );
    }
    let inspection = profile_evidence(
        provider,
        provider.inspection_evidence_ref.as_deref(),
        "inspection",
        "method_specific_inspection_report",
        "INSPECTION",
        &mut reasons,
    );

    if client.requires_replacement && !provider.replacement_isolated_and_reverified {
        add_reason(
            &mut reasons,
            "REPLACEMENT_REVERIFICATION_MISSING",
            Severity::Indeterminate,
        );
    }
    if client.requires_fatigue {
        profile_evidence(
            provider,
            provider.fatigue_evidence_ref.as_deref(),
            "fatigue",
            "fatigue_life_evidence",
            "FATIGUE",
            &mut reasons,
        );
    }
    if client.requires_assembled_path {
        profile_evidence(
            provider,
            provider.assembled_path_evidence_ref.as_deref(),
            "assembled_path",
            "as_built_load_path_acceptance",
            "ASSEMBLED_PATH",
            &mut reasons,
        );
    }
    if provider.uncertainty_basis != "hard_bound" || client.uncertainty_basis != "hard_bound" {
        add_reason(
            &mut reasons,
            "UNCERTAINTY_BASIS_UNSUPPORTED",
            Severity::Indeterminate,
        );
    }
    if client.requires_independent_evidence
        && let (Some(structural), Some(verification)) = (
            provider.load_cases.first().and_then(|load| {
                profile_evidence(
                    provider,
                    load.evidence_ref.as_deref(),
                    "structural_capability",
                    "scoped_test_or_validated_analysis",
                    "STRUCTURAL",
                    &mut reasons,
                )
            }),
            verification,
        )
        && structural.dependency_group == verification.dependency_group
    {
        add_reason(
            &mut reasons,
            "INDEPENDENCE_NOT_ESTABLISHED",
            Severity::Indeterminate,
        );
    }

    if let (Some(material), Some(inspection)) = (material, inspection)
        && material.dependency_group == inspection.dependency_group
        && client.requires_independent_evidence
    {
        add_reason(
            &mut reasons,
            "MATERIAL_INSPECTION_COMMON_CAUSE",
            Severity::Indeterminate,
        );
    }

    let disposition = if reasons
        .values()
        .any(|severity| *severity == Severity::Malformed)
    {
        "malformed_or_stale"
    } else if reasons
        .values()
        .any(|severity| *severity == Severity::Incompatible)
    {
        "incompatible"
    } else if !reasons.is_empty() {
        "indeterminate"
    } else {
        "compatible_synthetic_profile"
    };
    (disposition.to_owned(), reasons.into_keys().collect())
}

pub fn run_corpus(input: &[u8], source_commit: &str) -> Result<Receipt, String> {
    if !valid_commit(source_commit) {
        return Err("source commit must be exactly 40 lowercase hexadecimal characters".to_owned());
    }
    let corpus: Corpus =
        serde_json::from_slice(input).map_err(|error| format!("invalid corpus: {error}"))?;
    if corpus.schema != CORPUS_SCHEMA || corpus.claim_ceiling != CLAIM_CEILING {
        return Err("unsupported corpus schema or claim ceiling".to_owned());
    }
    if corpus.evidence_class != "synthetic" {
        return Err("v0.1 accepts synthetic fixtures only".to_owned());
    }
    if corpus.cases.is_empty() {
        return Err("corpus must contain at least one case".to_owned());
    }

    let input_sha256 = format!("{:x}", Sha256::digest(input));
    let mut ids = BTreeSet::new();
    let mut results = Vec::new();
    let mut matched_cases = 0usize;

    for case in &corpus.cases {
        if case.case_id.trim().is_empty() || !ids.insert(case.case_id.as_str()) {
            return Err("case IDs must be non-empty and unique".to_owned());
        }
        let mut provider = corpus.base_provider.clone();
        let mut client = corpus.base_client.clone();
        for mutation in &case.mutations {
            apply_mutation(&mut provider, &mut client, mutation)?;
        }
        let provider_profile: Profile = serde_json::from_value(provider)
            .map_err(|error| format!("{} provider profile invalid: {error}", case.case_id))?;
        let client_profile: Profile = serde_json::from_value(client)
            .map_err(|error| format!("{} client profile invalid: {error}", case.case_id))?;
        let (disposition, reasons) = evaluate(&provider_profile, &client_profile);
        let expected_match =
            disposition == case.expected.disposition && reasons == case.expected.reasons;
        if expected_match {
            matched_cases += 1;
        }
        results.push(CaseResult {
            case_id: case.case_id.clone(),
            disposition,
            reasons,
            expected_matched: expected_match,
        });
    }

    results.sort_by_key(|item| item.case_id.clone());
    let mismatched_cases = results.len() - matched_cases;
    Ok(Receipt {
        schema: RECEIPT_SCHEMA.to_owned(),
        input_sha256,
        source_commit: source_commit.to_owned(),
        evidence_class: "synthetic".to_owned(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
        authority_granted: false,
        case_count: results.len(),
        matched_cases,
        mismatched_cases,
        results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &[u8] = include_bytes!("../fixtures/v0.1/cases.json");
    const SOURCE: &str = "0123456789abcdef0123456789abcdef01234567";

    #[test]
    fn frozen_corpus_matches_every_expected_disposition_and_reason() {
        let receipt = run_corpus(FIXTURE, SOURCE).expect("fixture should parse");
        assert_eq!(receipt.case_count, 44);
        assert_eq!(receipt.matched_cases, 44);
        assert_eq!(receipt.mismatched_cases, 0);
        assert!(!receipt.authority_granted);
    }

    #[test]
    fn source_commit_shape_is_required() {
        assert!(run_corpus(FIXTURE, "main").is_err());
    }

    #[test]
    fn output_is_deterministic() {
        let first = serde_json::to_vec(&run_corpus(FIXTURE, SOURCE).unwrap()).unwrap();
        let second = serde_json::to_vec(&run_corpus(FIXTURE, SOURCE).unwrap()).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn case_order_does_not_change_canonical_results() {
        let mut value: Value = serde_json::from_slice(FIXTURE).unwrap();
        value["cases"].as_array_mut().unwrap().reverse();
        let reversed = serde_json::to_vec(&value).unwrap();
        let first = run_corpus(FIXTURE, SOURCE).unwrap();
        let second = run_corpus(&reversed, SOURCE).unwrap();
        assert_eq!(
            first
                .results
                .iter()
                .map(|r| (&r.case_id, &r.disposition, &r.reasons))
                .collect::<Vec<_>>(),
            second
                .results
                .iter()
                .map(|r| (&r.case_id, &r.disposition, &r.reasons))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn claim_ceiling_cannot_be_promoted_by_fixture() {
        let mut value: Value = serde_json::from_slice(FIXTURE).unwrap();
        value["claim_ceiling"] = Value::String("flight_ready".to_owned());
        let changed = serde_json::to_vec(&value).unwrap();
        assert!(run_corpus(&changed, SOURCE).is_err());
    }
}
