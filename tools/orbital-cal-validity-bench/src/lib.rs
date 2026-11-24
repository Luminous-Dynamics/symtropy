use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const CORPUS_SCHEMA: &str = "luminous.orbital-cal-validity.corpus.v0.1";
pub const RECEIPT_SCHEMA: &str = "luminous.orbital-cal-validity.receipt.v0.1";
pub const BENCHMARK_ID: &str = "ORBITAL-CAL-VALIDITY-001";
pub const CLAIM_CEILING: &str = "synthetic_calibration_scope_and_acquisition_state_validity_only";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CertificateState {
    Valid,
    Revoked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CheckState {
    Pass,
    Hold,
    Unknown,
    NotRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EventImpact {
    Invalidates,
    NoCalibrationImpact,
    UnknownImpact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ValidityStatus {
    ValidWithinScope,
    Expired,
    OutOfScope,
    DriftSuspected,
    IdentityMismatch,
    Indeterminate,
    Invalidated,
    Revoked,
    InvalidInput,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentState {
    pub component_id: String,
    pub configuration: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CalibrationScope {
    pub scope_id: String,
    pub calibration_id: String,
    pub calibration_revision: String,
    pub certificate_sha256: String,
    pub check_receipt_sha256: Option<String>,
    pub component_id: String,
    pub valid_from_ns: i64,
    pub valid_until_ns: i64,
    pub required_configuration: BTreeMap<String, String>,
    pub certificate_state: CertificateState,
    pub check_required: bool,
    pub check_state: CheckState,
    pub last_check_time_ns: Option<i64>,
    pub max_check_age_ns: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeEvent {
    pub event_id: String,
    pub effective_time_ns: i64,
    pub time_uncertainty_ns: u64,
    pub affected_scope_ids: Vec<String>,
    pub impact: EventImpact,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub case_id: String,
    pub acquisition_time_ns: i64,
    pub acquisition_uncertainty_ns: u64,
    pub event_inventory_complete: bool,
    pub required_scope_ids: Vec<String>,
    pub components: Vec<ComponentState>,
    pub scopes: Vec<CalibrationScope>,
    pub events: Vec<ChangeEvent>,
    pub expected_status: ValidityStatus,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    pub schema: String,
    pub benchmark_id: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub cases: Vec<Case>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ScopeResult {
    pub scope_id: String,
    pub calibration_id: String,
    pub calibration_revision: String,
    pub certificate_sha256: String,
    pub check_receipt_sha256: Option<String>,
    pub status: ValidityStatus,
    pub reason_codes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CaseResult {
    pub case_id: String,
    pub acquisition_interval_ns: [i128; 2],
    pub status: ValidityStatus,
    pub reason_codes: Vec<String>,
    pub event_inventory_complete: bool,
    pub scope_results: Vec<ScopeResult>,
    pub may_enter_geometry_precondition: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Receipt {
    pub schema: String,
    pub benchmark_id: String,
    pub source_commit: String,
    pub input_sha256: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub case_count: usize,
    pub expected_outcome_matches: usize,
    pub expected_outcome_mismatches: usize,
    pub authority_granted: bool,
    pub visibility_assessed: bool,
    pub measurement_performance_assessed: bool,
    pub physical_validation: String,
    pub cases: Vec<CaseResult>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn valid_text(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn precedence(status: ValidityStatus) -> u8 {
    match status {
        ValidityStatus::InvalidInput => 0,
        ValidityStatus::IdentityMismatch => 1,
        ValidityStatus::Revoked => 2,
        ValidityStatus::DriftSuspected => 3,
        ValidityStatus::Invalidated => 4,
        ValidityStatus::OutOfScope => 5,
        ValidityStatus::Expired => 6,
        ValidityStatus::Indeterminate => 7,
        ValidityStatus::ValidWithinScope => 8,
    }
}

fn add_reason(
    status: ValidityStatus,
    code: &str,
    statuses: &mut Vec<ValidityStatus>,
    reasons: &mut BTreeSet<String>,
) {
    statuses.push(status);
    reasons.insert(code.to_owned());
}

fn choose_status(statuses: &[ValidityStatus]) -> ValidityStatus {
    statuses
        .iter()
        .copied()
        .min_by_key(|status| precedence(*status))
        .unwrap_or(ValidityStatus::ValidWithinScope)
}

fn checked_interval(center_ns: i64, uncertainty_ns: u64) -> Result<[i128; 2], String> {
    let center = i128::from(center_ns);
    let uncertainty = i128::from(uncertainty_ns);
    let lower = center.checked_sub(uncertainty).ok_or_else(|| "TIME_INTERVAL_OVERFLOW".to_owned())?;
    let upper = center.checked_add(uncertainty).ok_or_else(|| "TIME_INTERVAL_OVERFLOW".to_owned())?;
    Ok([lower, upper])
}

fn validate_case(case: &Case) -> Result<(), String> {
    if !valid_text(&case.case_id) || case.required_scope_ids.is_empty() || case.scopes.is_empty() {
        return Err("CASE_ID_OR_REQUIRED_SCOPE_INVALID".to_owned());
    }

    checked_interval(case.acquisition_time_ns, case.acquisition_uncertainty_ns)?;

    let mut component_ids = BTreeSet::new();
    for component in &case.components {
        if !valid_text(&component.component_id) || !component_ids.insert(component.component_id.clone()) {
            return Err("COMPONENT_ID_DUPLICATE_OR_EMPTY".to_owned());
        }
        if component.configuration.iter().any(|(key, value)| !valid_text(key) || !valid_text(value)) {
            return Err("COMPONENT_CONFIGURATION_INVALID".to_owned());
        }
    }

    let mut scope_ids = BTreeSet::new();
    for scope in &case.scopes {
        if !valid_text(&scope.scope_id)
            || !valid_text(&scope.calibration_id)
            || !valid_text(&scope.calibration_revision)
            || !valid_sha256(&scope.certificate_sha256)
            || !valid_text(&scope.component_id)
            || scope.valid_until_ns <= scope.valid_from_ns
            || !scope_ids.insert(scope.scope_id.clone())
        {
            return Err("CALIBRATION_SCOPE_INVALID".to_owned());
        }
        if scope.required_configuration.iter().any(|(key, value)| !valid_text(key) || !valid_text(value)) {
            return Err("REQUIRED_CONFIGURATION_INVALID".to_owned());
        }
        if scope.check_required {
            if scope.check_state == CheckState::NotRequired {
                return Err("REQUIRED_CHECK_MARKED_NOT_REQUIRED".to_owned());
            }
            if scope.max_check_age_ns.is_none() {
                return Err("REQUIRED_CHECK_MAX_AGE_MISSING".to_owned());
            }
            if !scope.check_receipt_sha256.as_deref().is_some_and(valid_sha256) {
                return Err("REQUIRED_CHECK_RECEIPT_DIGEST_INVALID".to_owned());
            }
        }
    }

    let mut required = BTreeSet::new();
    for scope_id in &case.required_scope_ids {
        if !valid_text(scope_id) || !required.insert(scope_id.clone()) || !scope_ids.contains(scope_id) {
            return Err("REQUIRED_SCOPE_REFERENCE_INVALID".to_owned());
        }
    }

    let mut event_ids = BTreeSet::new();
    for event in &case.events {
        checked_interval(event.effective_time_ns, event.time_uncertainty_ns)?;
        if !valid_text(&event.event_id)
            || !event_ids.insert(event.event_id.clone())
            || event.affected_scope_ids.is_empty()
        {
            return Err("CHANGE_EVENT_INVALID".to_owned());
        }
        let mut affected = BTreeSet::new();
        for scope_id in &event.affected_scope_ids {
            if !affected.insert(scope_id.clone()) || !scope_ids.contains(scope_id) {
                return Err("CHANGE_EVENT_SCOPE_REFERENCE_INVALID".to_owned());
            }
        }
    }
    Ok(())
}

fn evaluate_scope(
    case: &Case,
    scope: &CalibrationScope,
    acquisition: [i128; 2],
) -> ScopeResult {
    let mut statuses = Vec::new();
    let mut reasons = BTreeSet::new();

    if scope.certificate_state == CertificateState::Revoked {
        add_reason(ValidityStatus::Revoked, "CERTIFICATE_REVOKED", &mut statuses, &mut reasons);
    }

    let valid_from = i128::from(scope.valid_from_ns);
    let valid_until = i128::from(scope.valid_until_ns);
    if acquisition[1] < valid_from {
        add_reason(ValidityStatus::Expired, "CALIBRATION_NOT_YET_VALID", &mut statuses, &mut reasons);
    } else if acquisition[0] >= valid_until {
        add_reason(ValidityStatus::Expired, "CALIBRATION_EXPIRED", &mut statuses, &mut reasons);
    } else if acquisition[0] < valid_from || acquisition[1] >= valid_until {
        add_reason(ValidityStatus::Indeterminate, "CALIBRATION_TIME_BOUNDARY_OVERLAP", &mut statuses, &mut reasons);
    }

    let component = case.components.iter().find(|component| component.component_id == scope.component_id);
    match component {
        None => add_reason(ValidityStatus::Indeterminate, "COMPONENT_STATE_MISSING", &mut statuses, &mut reasons),
        Some(component) => {
            for (key, required_value) in &scope.required_configuration {
                match component.configuration.get(key) {
                    None => add_reason(ValidityStatus::Indeterminate, "CONFIGURATION_FIELD_MISSING", &mut statuses, &mut reasons),
                    Some(actual_value) if actual_value != required_value => {
                        add_reason(ValidityStatus::IdentityMismatch, "CONFIGURATION_MISMATCH", &mut statuses, &mut reasons)
                    }
                    Some(_) => {}
                }
            }
        }
    }

    if scope.check_required {
        match scope.check_state {
            CheckState::Hold => add_reason(ValidityStatus::DriftSuspected, "REFERENCE_CHECK_HOLD", &mut statuses, &mut reasons),
            CheckState::Unknown | CheckState::NotRequired => {
                add_reason(ValidityStatus::Indeterminate, "REFERENCE_CHECK_STATE_UNKNOWN", &mut statuses, &mut reasons)
            }
            CheckState::Pass => {
                match (scope.last_check_time_ns, scope.max_check_age_ns) {
                    (Some(check_time), Some(max_age)) => {
                        let check_time = i128::from(check_time);
                        let max_age = i128::from(max_age);
                        if check_time > acquisition[1] {
                            add_reason(ValidityStatus::Indeterminate, "REFERENCE_CHECK_AFTER_ACQUISITION", &mut statuses, &mut reasons);
                        } else if acquisition[0] - check_time > max_age {
                            add_reason(ValidityStatus::Expired, "REFERENCE_CHECK_STALE", &mut statuses, &mut reasons);
                        } else if acquisition[1] - check_time > max_age {
                            add_reason(ValidityStatus::Indeterminate, "REFERENCE_CHECK_AGE_BOUNDARY_OVERLAP", &mut statuses, &mut reasons);
                        }
                    }
                    _ => add_reason(ValidityStatus::Indeterminate, "REFERENCE_CHECK_EVIDENCE_MISSING", &mut statuses, &mut reasons),
                }
            }
        }
    }

    for event in &case.events {
        if !event.affected_scope_ids.iter().any(|scope_id| scope_id == &scope.scope_id) {
            continue;
        }
        if event.impact == EventImpact::NoCalibrationImpact {
            continue;
        }
        let event_interval = match checked_interval(event.effective_time_ns, event.time_uncertainty_ns) {
            Ok(interval) => interval,
            Err(_) => {
                add_reason(ValidityStatus::InvalidInput, "EVENT_TIME_INTERVAL_INVALID", &mut statuses, &mut reasons);
                continue;
            }
        };
        if event_interval[1] <= acquisition[0] {
            match event.impact {
                EventImpact::Invalidates => add_reason(ValidityStatus::Invalidated, "CALIBRATION_INVALIDATED_BY_EVENT", &mut statuses, &mut reasons),
                EventImpact::UnknownImpact => add_reason(ValidityStatus::Indeterminate, "EVENT_IMPACT_UNKNOWN", &mut statuses, &mut reasons),
                EventImpact::NoCalibrationImpact => {}
            }
        } else if event_interval[0] <= acquisition[1] {
            add_reason(ValidityStatus::Indeterminate, "EVENT_TIME_OVERLAPS_ACQUISITION", &mut statuses, &mut reasons);
        }
    }

    let status = choose_status(&statuses);
    ScopeResult {
        scope_id: scope.scope_id.clone(),
        calibration_id: scope.calibration_id.clone(),
        calibration_revision: scope.calibration_revision.clone(),
        certificate_sha256: scope.certificate_sha256.clone(),
        check_receipt_sha256: scope.check_receipt_sha256.clone(),
        status,
        reason_codes: reasons.into_iter().collect(),
    }
}

fn evaluate_case(case: &Case) -> Result<CaseResult, String> {
    validate_case(case)?;
    let acquisition = checked_interval(case.acquisition_time_ns, case.acquisition_uncertainty_ns)?;

    let scope_map: BTreeMap<&str, &CalibrationScope> = case.scopes.iter().map(|scope| (scope.scope_id.as_str(), scope)).collect();
    let mut scope_results = Vec::new();
    let mut statuses = Vec::new();
    let mut reasons = BTreeSet::new();

    for scope_id in &case.required_scope_ids {
        let scope = scope_map.get(scope_id.as_str()).ok_or_else(|| "REQUIRED_SCOPE_REFERENCE_INVALID".to_owned())?;
        let result = evaluate_scope(case, scope, acquisition);
        statuses.push(result.status);
        for reason in &result.reason_codes {
            reasons.insert(reason.clone());
        }
        scope_results.push(result);
    }

    if !case.event_inventory_complete {
        add_reason(ValidityStatus::Indeterminate, "EVENT_INVENTORY_INCOMPLETE", &mut statuses, &mut reasons);
    }
    let status = choose_status(&statuses);
    Ok(CaseResult {
        case_id: case.case_id.clone(),
        acquisition_interval_ns: acquisition,
        status,
        reason_codes: reasons.into_iter().collect(),
        event_inventory_complete: case.event_inventory_complete,
        may_enter_geometry_precondition: status == ValidityStatus::ValidWithinScope,
        scope_results,
    })
}

pub fn audit_corpus(bytes: &[u8], source_commit: &str) -> Result<Receipt, String> {
    if source_commit.len() != 40 || !source_commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("SOURCE_COMMIT_MUST_BE_40_HEX_CHARACTERS".to_owned());
    }
    let corpus: Corpus = serde_json::from_slice(bytes).map_err(|error| format!("CORPUS_JSON_INVALID: {error}"))?;
    if corpus.schema != CORPUS_SCHEMA
        || corpus.benchmark_id != BENCHMARK_ID
        || corpus.evidence_class != "synthetic"
        || corpus.claim_ceiling != CLAIM_CEILING
        || corpus.cases.is_empty()
        || corpus.cases.len() > 256
    {
        return Err("CORPUS_IDENTITY_OR_CLAIM_CEILING_INVALID".to_owned());
    }

    let mut case_ids = BTreeSet::new();
    let mut cases = Vec::new();
    for case in &corpus.cases {
        if !case_ids.insert(&case.case_id) {
            return Err("CASE_ID_DUPLICATE".to_owned());
        }
        cases.push(evaluate_case(case)?);
    }

    let expected_outcome_matches = cases.iter().zip(corpus.cases.iter())
        .filter(|(result, source)| result.status == source.expected_status)
        .count();

    Ok(Receipt {
        schema: RECEIPT_SCHEMA.to_owned(),
        benchmark_id: BENCHMARK_ID.to_owned(),
        source_commit: source_commit.to_ascii_lowercase(),
        input_sha256: sha256_hex(bytes),
        evidence_class: corpus.evidence_class,
        claim_ceiling: corpus.claim_ceiling,
        case_count: cases.len(),
        expected_outcome_matches,
        expected_outcome_mismatches: cases.len() - expected_outcome_matches,
        authority_granted: false,
        visibility_assessed: false,
        measurement_performance_assessed: false,
        physical_validation: "NOT_ASSESSED".to_owned(),
        cases,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope() -> CalibrationScope {
        CalibrationScope {
            scope_id: "camera-intrinsics".to_owned(),
            calibration_id: "CAL-CAM-01".to_owned(),
            calibration_revision: "r7".to_owned(),
            certificate_sha256: "a".repeat(64),
            check_receipt_sha256: Some("b".repeat(64)),
            component_id: "CAM-01".to_owned(),
            valid_from_ns: 100,
            valid_until_ns: 1000,
            required_configuration: BTreeMap::from([
                ("sensor_serial".to_owned(), "SENSOR-01".to_owned()),
                ("lens".to_owned(), "L-1".to_owned()),
                ("resolution".to_owned(), "1920x1080".to_owned()),
            ]),
            certificate_state: CertificateState::Valid,
            check_required: true,
            check_state: CheckState::Pass,
            last_check_time_ns: Some(450),
            max_check_age_ns: Some(200),
        }
    }

    fn case() -> Case {
        Case {
            case_id: "case".to_owned(),
            acquisition_time_ns: 500,
            acquisition_uncertainty_ns: 2,
            event_inventory_complete: true,
            required_scope_ids: vec!["camera-intrinsics".to_owned()],
            components: vec![ComponentState {
                component_id: "CAM-01".to_owned(),
                configuration: BTreeMap::from([
                    ("sensor_serial".to_owned(), "SENSOR-01".to_owned()),
                    ("lens".to_owned(), "L-1".to_owned()),
                    ("resolution".to_owned(), "1920x1080".to_owned()),
                ]),
            }],
            scopes: vec![scope()],
            events: vec![],
            expected_status: ValidityStatus::ValidWithinScope,
        }
    }

    #[test]
    fn matching_fresh_scope_is_valid() {
        let result = evaluate_case(&case()).expect("valid case");
        assert_eq!(result.status, ValidityStatus::ValidWithinScope);
        assert!(result.may_enter_geometry_precondition);
    }

    #[test]
    fn acquisition_after_expiry_is_expired() {
        let mut value = case();
        value.acquisition_time_ns = 1000;
        let result = evaluate_case(&value).expect("well-formed case");
        assert_eq!(result.status, ValidityStatus::Expired);
    }

    #[test]
    fn interval_straddling_expiry_is_indeterminate() {
        let mut value = case();
        value.acquisition_time_ns = 999;
        value.acquisition_uncertainty_ns = 2;
        let result = evaluate_case(&value).expect("well-formed case");
        assert_eq!(result.status, ValidityStatus::Indeterminate);
    }

    #[test]
    fn configuration_mismatch_is_not_qualified() {
        let mut value = case();
        value.components[0].configuration.insert("lens".to_owned(), "L-2".to_owned());
        let result = evaluate_case(&value).expect("well-formed case");
        assert_eq!(result.status, ValidityStatus::IdentityMismatch);
        assert!(!result.may_enter_geometry_precondition);
    }

    #[test]
    fn reference_hold_is_drift_suspected() {
        let mut value = case();
        value.scopes[0].check_state = CheckState::Hold;
        let result = evaluate_case(&value).expect("well-formed case");
        assert_eq!(result.status, ValidityStatus::DriftSuspected);
    }

    #[test]
    fn stale_reference_check_is_expired() {
        let mut value = case();
        value.scopes[0].last_check_time_ns = Some(200);
        let result = evaluate_case(&value).expect("well-formed case");
        assert_eq!(result.status, ValidityStatus::Expired);
        assert!(result.reason_codes.contains(&"REFERENCE_CHECK_STALE".to_owned()));
    }

    #[test]
    fn prior_invalidation_event_invalidates_scope() {
        let mut value = case();
        value.events.push(ChangeEvent {
            event_id: "tool-change-1".to_owned(),
            effective_time_ns: 400,
            time_uncertainty_ns: 1,
            affected_scope_ids: vec!["camera-intrinsics".to_owned()],
            impact: EventImpact::Invalidates,
        });
        let result = evaluate_case(&value).expect("well-formed case");
        assert_eq!(result.status, ValidityStatus::Invalidated);
    }

    #[test]
    fn event_overlapping_acquisition_is_indeterminate() {
        let mut value = case();
        value.events.push(ChangeEvent {
            event_id: "collision-1".to_owned(),
            effective_time_ns: 501,
            time_uncertainty_ns: 4,
            affected_scope_ids: vec!["camera-intrinsics".to_owned()],
            impact: EventImpact::Invalidates,
        });
        let result = evaluate_case(&value).expect("well-formed case");
        assert_eq!(result.status, ValidityStatus::Indeterminate);
    }

    #[test]
    fn incomplete_event_inventory_cannot_pass() {
        let mut value = case();
        value.event_inventory_complete = false;
        let result = evaluate_case(&value).expect("well-formed case");
        assert_eq!(result.status, ValidityStatus::Indeterminate);
        assert!(!result.may_enter_geometry_precondition);
    }

    #[test]
    fn missing_configuration_does_not_become_identity() {
        let mut value = case();
        value.components[0].configuration.remove("lens");
        let result = evaluate_case(&value).expect("well-formed case");
        assert_eq!(result.status, ValidityStatus::Indeterminate);
    }

    #[test]
    fn no_calibration_impact_event_does_not_invalidate() {
        let mut value = case();
        value.events.push(ChangeEvent {
            event_id: "maintenance-log-1".to_owned(),
            effective_time_ns: 400,
            time_uncertainty_ns: 1,
            affected_scope_ids: vec!["camera-intrinsics".to_owned()],
            impact: EventImpact::NoCalibrationImpact,
        });
        let result = evaluate_case(&value).expect("well-formed case");
        assert_eq!(result.status, ValidityStatus::ValidWithinScope);
    }

    #[test]
    fn revoked_certificate_is_revoked() {
        let mut value = case();
        value.scopes[0].certificate_state = CertificateState::Revoked;
        let result = evaluate_case(&value).expect("well-formed case");
        assert_eq!(result.status, ValidityStatus::Revoked);
    }

    #[test]
    fn required_scope_reference_must_exist() {
        let mut value = case();
        value.required_scope_ids.push("missing".to_owned());
        assert!(evaluate_case(&value).is_err());
    }

    #[test]
    fn malformed_certificate_digest_is_rejected() {
        let mut value = case();
        value.scopes[0].certificate_sha256 = "not-a-digest".to_owned();
        assert_eq!(evaluate_case(&value).unwrap_err(), "CALIBRATION_SCOPE_INVALID");
    }

    #[test]
    fn malformed_required_check_receipt_digest_is_rejected() {
        let mut value = case();
        value.scopes[0].check_receipt_sha256 = Some("bad".to_owned());
        assert_eq!(validate_case(&value).unwrap_err(), "REQUIRED_CHECK_RECEIPT_DIGEST_INVALID");
    }

    #[test]
    fn source_commit_must_be_exactly_forty_hex() {
        assert_eq!(audit_corpus(b"{}", "short").unwrap_err(), "SOURCE_COMMIT_MUST_BE_40_HEX_CHARACTERS");
    }
}
