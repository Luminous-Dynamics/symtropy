//! Independent reference oracle for PHRI-TAC-TRANSFER-001.
//!
//! This synthetic-only reference checks split integrity, transfer-method
//! applicability, target-device calibration identity, held-out transfer
//! validation, uncertainty/guard-band evidence, and recalibration disposition.
//!
//! A transferred predictor is not a calibration certificate. A model estimate
//! remains distinct from a target-qualified estimate, and neither can silently
//! rewrite or inherit another physical sensor's calibration identity.

#![forbid(unsafe_code)]

use serde::Deserialize;
use std::collections::BTreeSet;

pub const CORPUS_JSON: &str = include_str!("../cases-v0.json");
pub const CORPUS_SCHEMA_JSON: &str = include_str!("../cases-v0.schema.json");

const EXPECTED_CASE_COUNT: usize = 34;

const EXPECTED_CASES: &[(&str, &str)] = &[
    ("PTT-001", "within_device_heldout_contacts"),
    ("PTT-002", "zero_shot_cross_unit_no_target_labels"),
    ("PTT-003", "few_shot_target_heldout_validation"),
    ("PTT-004", "source_target_sample_overlap"),
    ("PTT-005", "contact_sequence_overlap"),
    ("PTT-006", "target_unit_leaked_into_source_training"),
    ("PTT-007", "target_lot_leaked_into_source_training"),
    ("PTT-008", "adaptation_test_sequence_overlap"),
    ("PTT-009", "validation_test_sequence_overlap"),
    ("PTT-010", "zero_shot_contains_target_adaptation"),
    ("PTT-011", "few_shot_without_target_labels"),
    ("PTT-012", "target_adaptation_budget_exceeded"),
    ("PTT-013", "source_calibration_stale"),
    ("PTT-014", "source_certificate_reused_for_target_unit"),
    ("PTT-015", "target_calibration_missing"),
    ("PTT-016", "target_calibration_expired"),
    ("PTT-017", "firmware_revision_changed_after_calibration"),
    ("PTT-018", "skin_revision_changed_after_calibration"),
    ("PTT-019", "unsupported_sensor_family_pair"),
    ("PTT-020", "unsupported_target_profile_digest"),
    ("PTT-021", "target_age_exceeds_transfer_envelope"),
    ("PTT-022", "target_temperature_outside_transfer_envelope"),
    ("PTT-023", "transfer_validation_reference_not_independent"),
    ("PTT-024", "heldout_uncertainty_coverage_failed"),
    ("PTT-025", "guard_band_not_qualified"),
    ("PTT-026", "only_development_validation_available"),
    ("PTT-027", "validation_receipt_binds_other_sensor_unit"),
    ("PTT-028", "model_binds_wrong_target_calibration"),
    ("PTT-029", "qualified_transfer_remains_model_estimate_when_claimed_as_such"),
    ("PTT-030", "qualified_cross_family_transfer"),
    ("PTT-031", "source_train_target_validation_sample_overlap"),
    ("PTT-032", "source_train_target_adaptation_sample_overlap"),
    ("PTT-033", "source_unit_leaked_into_target_validation_group"),
    ("PTT-034", "source_lot_leaked_into_target_adaptation_group"),
];

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransferRegime {
    WithinDevice,
    CrossUnitZeroShot,
    CrossUnitFewShot,
    CrossFamilyZeroShot,
    CrossRevision,
    Aging,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SensorFamily {
    PressureArray,
    VisionTactile,
    MagneticSkin,
    CapacitiveSkin,
    PiezoresistiveSkin,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CalibrationState {
    Current,
    Missing,
    Expired,
    Invalidated,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValidationState {
    NotValidated,
    DevelopmentOnly,
    HeldoutPass,
    Failed,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceState {
    Pass,
    Fail,
    Unresolved,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutputClaim {
    ModelEstimate,
    Measurement,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SplitDisposition {
    Valid,
    Leaked,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransferDisposition {
    Eligible,
    BlockedSplit,
    BlockedMethod,
    BlockedSourceCalibration,
    RequiresAdaptation,
    BlockedBudget,
    OutOfProfile,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MetrologyDisposition {
    ModelEstimateOnly,
    TargetQualifiedEstimate,
    InvalidMeasurementClaim,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RecalibrationDisposition {
    NotIndicated,
    Required,
    Unknown,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    pub schema_version: String,
    pub corpus_id: String,
    pub version: String,
    pub status: String,
    pub claim_ceiling: String,
    pub cases: Vec<Case>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub id: String,
    pub family: String,
    pub candidate_input: CandidateInput,
    pub evaluator_only: EvaluatorOnly,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluatorOnly {
    pub fault_kind: String,
    pub expected: ExpectedDisposition,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExpectedDisposition {
    pub split_integrity: SplitDisposition,
    pub transfer_eligibility: TransferDisposition,
    pub metrology_disposition: MetrologyDisposition,
    pub recalibration_disposition: RecalibrationDisposition,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateInput {
    pub regime: TransferRegime,
    pub source: SensorDomain,
    pub target: TargetDomain,
    pub transfer_method: TransferMethod,
    pub splits: SplitManifest,
    pub holdout_materials: bool,
    pub target_calibration: Option<TargetCalibration>,
    pub transfer_validation: TransferValidation,
    pub model: TransferModel,
    pub output_claim: OutputClaim,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SensorDomain {
    pub unit_id: String,
    pub lot_id: String,
    pub family: SensorFamily,
    pub profile_digest: String,
    pub calibration_id: String,
    pub calibration_state: CalibrationState,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetDomain {
    pub unit_id: String,
    pub lot_id: String,
    pub family: SensorFamily,
    pub profile_digest: String,
    pub firmware_revision: String,
    pub skin_revision: String,
    pub age_cycles: u64,
    pub temperature_c: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransferMethod {
    pub method_id: String,
    pub enabled: bool,
    pub source_family: SensorFamily,
    pub target_family: SensorFamily,
    pub source_profile_digest: String,
    pub target_profile_digest: String,
    pub maximum_target_age_cycles: u64,
    pub minimum_temperature_c: f64,
    pub maximum_temperature_c: f64,
    pub maximum_target_adaptation_samples: usize,
    pub requires_target_adaptation: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SplitManifest {
    pub source_train: SplitGroup,
    pub target_adaptation: SplitGroup,
    pub target_validation: SplitGroup,
    pub target_test: SplitGroup,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SplitGroup {
    pub unit_ids: Vec<String>,
    pub lot_ids: Vec<String>,
    pub sample_ids: Vec<String>,
    pub sequence_ids: Vec<String>,
    pub article_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetCalibration {
    pub calibration_id: String,
    pub device_id: String,
    pub state: CalibrationState,
    pub profile_digest: String,
    pub firmware_revision: String,
    pub skin_revision: String,
    pub maximum_age_cycles: u64,
    pub minimum_temperature_c: f64,
    pub maximum_temperature_c: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransferValidation {
    pub state: ValidationState,
    pub target_unit_id: String,
    pub target_lot_id: String,
    pub model_digest: String,
    pub validation_receipt_id: String,
    pub reference_calibration_id: String,
    pub reference_calibration_state: CalibrationState,
    pub independent_reference: bool,
    pub uncertainty_state: EvidenceState,
    pub guard_band_state: EvidenceState,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransferModel {
    pub model_id: String,
    pub model_digest: String,
    pub source_calibration_id: String,
    pub target_calibration_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationReport {
    pub case_count: usize,
    pub valid_splits: usize,
    pub leaked_splits: usize,
    pub transfer_eligible: usize,
    pub target_qualified_estimates: usize,
    pub model_estimate_only: usize,
    pub invalid_measurement_claims: usize,
}

fn overlaps(left: &[String], right: &[String]) -> bool {
    left.iter().any(|item| right.contains(item))
}

fn has_duplicates(values: &[String]) -> bool {
    let mut seen = BTreeSet::new();
    values.iter().any(|value| !seen.insert(value))
}

fn group_is_empty(group: &SplitGroup) -> bool {
    group.unit_ids.is_empty()
        && group.lot_ids.is_empty()
        && group.sample_ids.is_empty()
        && group.sequence_ids.is_empty()
        && group.article_ids.is_empty()
}

fn group_has_duplicates(group: &SplitGroup) -> bool {
    has_duplicates(&group.unit_ids)
        || has_duplicates(&group.lot_ids)
        || has_duplicates(&group.sample_ids)
        || has_duplicates(&group.sequence_ids)
        || has_duplicates(&group.article_ids)
}

fn group_binds_target(group: &SplitGroup, target: &TargetDomain) -> bool {
    group.unit_ids.contains(&target.unit_id) && group.lot_ids.contains(&target.lot_id)
}

fn split_integrity(input: &CandidateInput) -> SplitDisposition {
    let splits = &input.splits;
    let source_train = &splits.source_train;
    let target_adaptation = &splits.target_adaptation;
    let target_validation = &splits.target_validation;
    let target_test = &splits.target_test;

    if [
        source_train,
        target_adaptation,
        target_validation,
        target_test,
    ]
    .into_iter()
    .any(group_has_duplicates)
    {
        return SplitDisposition::Leaked;
    }

    // No source training sample/contact sequence may leak into any target
    // adaptation, validation, or test partition. The target partitions must
    // also be pairwise disjoint. Frame-level uniqueness is insufficient:
    // repeated frames from one physical press share a sequence identity.
    let target_groups = [target_adaptation, target_validation, target_test];
    let source_target_overlap = target_groups.iter().any(|target_group| {
        overlaps(&source_train.sample_ids, &target_group.sample_ids)
            || overlaps(&source_train.sequence_ids, &target_group.sequence_ids)
    });
    let target_partition_overlap =
        overlaps(&target_adaptation.sample_ids, &target_validation.sample_ids)
            || overlaps(&target_adaptation.sequence_ids, &target_validation.sequence_ids)
            || overlaps(&target_adaptation.sample_ids, &target_test.sample_ids)
            || overlaps(&target_adaptation.sequence_ids, &target_test.sequence_ids)
            || overlaps(&target_validation.sample_ids, &target_test.sample_ids)
            || overlaps(&target_validation.sequence_ids, &target_test.sequence_ids);

    if source_target_overlap || target_partition_overlap {
        return SplitDisposition::Leaked;
    }

    if input.holdout_materials {
        let source_or_target_article_overlap = target_groups.iter().any(|target_group| {
            overlaps(&source_train.article_ids, &target_group.article_ids)
        });
        let target_article_overlap =
            overlaps(&target_adaptation.article_ids, &target_validation.article_ids)
                || overlaps(&target_adaptation.article_ids, &target_test.article_ids)
                || overlaps(&target_validation.article_ids, &target_test.article_ids);
        if source_or_target_article_overlap || target_article_overlap {
            return SplitDisposition::Leaked;
        }
    }

    if !source_train.unit_ids.contains(&input.source.unit_id)
        || !source_train.lot_ids.contains(&input.source.lot_id)
        || !group_binds_target(target_test, &input.target)
    {
        return SplitDisposition::Leaked;
    }

    if !group_is_empty(target_adaptation)
        && !group_binds_target(target_adaptation, &input.target)
    {
        return SplitDisposition::Leaked;
    }

    if !group_is_empty(target_validation)
        && !group_binds_target(target_validation, &input.target)
    {
        return SplitDisposition::Leaked;
    }

    let is_cross_unit = matches!(
        input.regime,
        TransferRegime::CrossUnitZeroShot
            | TransferRegime::CrossUnitFewShot
            | TransferRegime::CrossFamilyZeroShot
    );
    let source_target_identity_overlap = target_groups.iter().any(|target_group| {
        overlaps(&source_train.unit_ids, &target_group.unit_ids)
            || overlaps(&source_train.lot_ids, &target_group.lot_ids)
    });
    let regime_identity_valid = match input.regime {
        TransferRegime::WithinDevice => input.source.unit_id == input.target.unit_id,
        TransferRegime::CrossRevision | TransferRegime::Aging => {
            input.source.unit_id == input.target.unit_id
        }
        TransferRegime::CrossUnitZeroShot
        | TransferRegime::CrossUnitFewShot
        | TransferRegime::CrossFamilyZeroShot => {
            input.source.unit_id != input.target.unit_id
                && input.source.lot_id != input.target.lot_id
                && !source_target_identity_overlap
        }
    };
    if is_cross_unit && source_target_identity_overlap {
        return SplitDisposition::Leaked;
    }

    if !regime_identity_valid {
        return SplitDisposition::Leaked;
    }

    if matches!(
        input.regime,
        TransferRegime::CrossUnitZeroShot | TransferRegime::CrossFamilyZeroShot
    ) && !group_is_empty(target_adaptation)
    {
        return SplitDisposition::Leaked;
    }

    SplitDisposition::Valid
}

fn transfer_eligibility(
    input: &CandidateInput,
    split: SplitDisposition,
) -> TransferDisposition {
    if split != SplitDisposition::Valid {
        return TransferDisposition::BlockedSplit;
    }

    let method = &input.transfer_method;
    if !method.enabled {
        return TransferDisposition::BlockedMethod;
    }

    if input.source.calibration_state != CalibrationState::Current
        || input.model.source_calibration_id != input.source.calibration_id
    {
        return TransferDisposition::BlockedSourceCalibration;
    }

    if method.source_family != input.source.family
        || method.target_family != input.target.family
        || method.source_profile_digest != input.source.profile_digest
        || method.target_profile_digest != input.target.profile_digest
    {
        return TransferDisposition::OutOfProfile;
    }

    let is_cross_family = input.source.family != input.target.family;
    if matches!(input.regime, TransferRegime::CrossFamilyZeroShot) != is_cross_family {
        return TransferDisposition::OutOfProfile;
    }

    if input.target.age_cycles > method.maximum_target_age_cycles
        || input.target.temperature_c < method.minimum_temperature_c
        || input.target.temperature_c > method.maximum_temperature_c
    {
        return TransferDisposition::OutOfProfile;
    }

    let adaptation_count = input.splits.target_adaptation.sample_ids.len();
    let adaptation_required =
        method.requires_target_adaptation || input.regime == TransferRegime::CrossUnitFewShot;

    if adaptation_required && adaptation_count == 0 {
        return TransferDisposition::RequiresAdaptation;
    }

    if adaptation_count > method.maximum_target_adaptation_samples {
        return TransferDisposition::BlockedBudget;
    }

    TransferDisposition::Eligible
}

fn target_calibration_applicable(input: &CandidateInput) -> bool {
    let Some(calibration) = &input.target_calibration else {
        return false;
    };

    calibration.state == CalibrationState::Current
        && calibration.device_id == input.target.unit_id
        && calibration.calibration_id != input.source.calibration_id
        && calibration.profile_digest == input.target.profile_digest
        && calibration.firmware_revision == input.target.firmware_revision
        && calibration.skin_revision == input.target.skin_revision
        && input.target.age_cycles <= calibration.maximum_age_cycles
        && input.target.temperature_c >= calibration.minimum_temperature_c
        && input.target.temperature_c <= calibration.maximum_temperature_c
}

fn metrology_disposition(
    input: &CandidateInput,
    transfer: TransferDisposition,
    split: SplitDisposition,
) -> MetrologyDisposition {
    if input.output_claim == OutputClaim::ModelEstimate {
        return MetrologyDisposition::ModelEstimateOnly;
    }

    if split != SplitDisposition::Valid || transfer != TransferDisposition::Eligible {
        return MetrologyDisposition::InvalidMeasurementClaim;
    }

    let Some(calibration) = &input.target_calibration else {
        return MetrologyDisposition::InvalidMeasurementClaim;
    };

    let validation = &input.transfer_validation;
    let validation_qualified = validation.state == ValidationState::HeldoutPass
        && validation.target_unit_id == input.target.unit_id
        && validation.target_lot_id == input.target.lot_id
        && validation.model_digest == input.model.model_digest
        && validation.reference_calibration_state == CalibrationState::Current
        && !validation.reference_calibration_id.is_empty()
        && validation.independent_reference
        && validation.uncertainty_state == EvidenceState::Pass
        && validation.guard_band_state == EvidenceState::Pass
        && !validation.validation_receipt_id.is_empty()
        && !input.splits.target_validation.sample_ids.is_empty()
        && input.model.target_calibration_id.as_deref()
            == Some(calibration.calibration_id.as_str());

    if target_calibration_applicable(input) && validation_qualified {
        MetrologyDisposition::TargetQualifiedEstimate
    } else {
        MetrologyDisposition::InvalidMeasurementClaim
    }
}

fn recalibration_disposition(
    input: &CandidateInput,
    split: SplitDisposition,
) -> RecalibrationDisposition {
    if split != SplitDisposition::Valid {
        return RecalibrationDisposition::Unknown;
    }

    if target_calibration_applicable(input) {
        RecalibrationDisposition::NotIndicated
    } else {
        RecalibrationDisposition::Required
    }
}

fn expected_for(input: &CandidateInput) -> ExpectedDisposition {
    let split = split_integrity(input);
    let transfer = transfer_eligibility(input, split);
    let metrology = metrology_disposition(input, transfer, split);
    let recalibration = recalibration_disposition(input, split);

    ExpectedDisposition {
        split_integrity: split,
        transfer_eligibility: transfer,
        metrology_disposition: metrology,
        recalibration_disposition: recalibration,
    }
}

fn is_hex_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_case(case: &Case) -> Result<(), String> {
    let input = &case.candidate_input;
    for (name, digest) in [
        ("source.profile_digest", input.source.profile_digest.as_str()),
        ("target.profile_digest", input.target.profile_digest.as_str()),
        (
            "transfer_method.source_profile_digest",
            input.transfer_method.source_profile_digest.as_str(),
        ),
        (
            "transfer_method.target_profile_digest",
            input.transfer_method.target_profile_digest.as_str(),
        ),
        ("model.model_digest", input.model.model_digest.as_str()),
        (
            "transfer_validation.model_digest",
            input.transfer_validation.model_digest.as_str(),
        ),
    ] {
        if !is_hex_digest(digest) {
            return Err(format!(
                "{}: {name} is not a 64-character lowercase SHA-256 digest",
                case.id
            ));
        }
    }

    if let Some(calibration) = &input.target_calibration {
        if !is_hex_digest(&calibration.profile_digest) {
            return Err(format!(
                "{}: target calibration profile digest is invalid",
                case.id
            ));
        }
    }

    let derived = expected_for(input);
    if derived != case.evaluator_only.expected {
        return Err(format!(
            "{}: expected disposition mismatch; derived {derived:?}, recorded {:?}",
            case.id, case.evaluator_only.expected
        ));
    }

    Ok(())
}

pub fn validate_document(corpus: &Corpus) -> Result<ValidationReport, String> {
    if corpus.schema_version != "phri.tactile-transfer-corpus/1.0"
        || corpus.corpus_id != "PHRI-TAC-TRANSFER-CORPUS-001"
        || corpus.version != "0.1.0"
        || corpus.status != "synthetic_reference_only"
    {
        return Err("unexpected corpus identity or claim status".to_owned());
    }

    if corpus.cases.len() != EXPECTED_CASE_COUNT || corpus.cases.len() != EXPECTED_CASES.len() {
        return Err(format!(
            "expected {EXPECTED_CASE_COUNT} cases, found {}",
            corpus.cases.len()
        ));
    }

    let mut identifiers = BTreeSet::new();
    let mut report = ValidationReport {
        case_count: corpus.cases.len(),
        valid_splits: 0,
        leaked_splits: 0,
        transfer_eligible: 0,
        target_qualified_estimates: 0,
        model_estimate_only: 0,
        invalid_measurement_claims: 0,
    };

    for (index, case) in corpus.cases.iter().enumerate() {
        let (expected_id, expected_fault) = EXPECTED_CASES[index];
        if case.id != expected_id || case.evaluator_only.fault_kind != expected_fault {
            return Err(format!("fixture identity drift at position {index}: {}", case.id));
        }
        if !identifiers.insert(case.id.as_str()) {
            return Err(format!("duplicate fixture id: {}", case.id));
        }
        validate_case(case)?;

        let expected = case.evaluator_only.expected;
        match expected.split_integrity {
            SplitDisposition::Valid => report.valid_splits += 1,
            SplitDisposition::Leaked => report.leaked_splits += 1,
        }
        if expected.transfer_eligibility == TransferDisposition::Eligible {
            report.transfer_eligible += 1;
        }
        match expected.metrology_disposition {
            MetrologyDisposition::TargetQualifiedEstimate => {
                report.target_qualified_estimates += 1;
            }
            MetrologyDisposition::ModelEstimateOnly => report.model_estimate_only += 1,
            MetrologyDisposition::InvalidMeasurementClaim => {
                report.invalid_measurement_claims += 1;
            }
        }
    }

    Ok(report)
}

pub fn validate_tracked() -> Result<ValidationReport, String> {
    let corpus: Corpus = serde_json::from_str(CORPUS_JSON)
        .map_err(|error| format!("tracked transfer corpus is invalid: {error}"))?;
    validate_document(&corpus)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn tracked_value() -> Value {
        serde_json::from_str(CORPUS_JSON).expect("tracked corpus is JSON")
    }

    fn mutate_case(value: &mut Value, id: &str, mutate: impl FnOnce(&mut Value)) {
        let case = value["cases"]
            .as_array_mut()
            .expect("cases array")
            .iter_mut()
            .find(|case| case["id"].as_str() == Some(id))
            .expect("fixture exists");
        mutate(case);
    }

    fn parse_and_validate(value: &Value) -> Result<ValidationReport, String> {
        let raw = serde_json::to_string(value).expect("serializable test mutation");
        let corpus: Corpus = serde_json::from_str(&raw).map_err(|error| error.to_string())?;
        validate_document(&corpus)
    }

    #[test]
    fn all_thirty_four_transfer_fixtures_match_independent_expected_dispositions() {
        let report = validate_tracked().expect("frozen corpus is valid");
        assert_eq!(report.case_count, 34);
        assert_eq!(report.valid_splits + report.leaked_splits, 34);
        assert_eq!(report.target_qualified_estimates, 2);
        assert_eq!(report.model_estimate_only, 3);
        assert_eq!(report.invalid_measurement_claims, 25);
    }

    #[test]
    fn same_contact_sample_cannot_appear_in_source_train_and_target_test() {
        let mut value = tracked_value();
        mutate_case(&mut value, "PTT-003", |case| {
            case["candidate_input"]["splits"]["target_test"]["sample_ids"][0] =
                serde_json::json!("src-sample-01");
        });
        assert!(parse_and_validate(&value).is_err());
    }

    #[test]
    fn near_duplicate_contact_sequence_overlap_is_rejected() {
        let mut value = tracked_value();
        mutate_case(&mut value, "PTT-003", |case| {
            case["candidate_input"]["splits"]["target_test"]["sequence_ids"][0] =
                serde_json::json!("src-sequence-01");
        });
        assert!(parse_and_validate(&value).is_err());
    }

    #[test]
    fn target_unit_and_manufacturing_lot_must_be_held_out_for_cross_unit_transfer() {
        let mut value = tracked_value();
        mutate_case(&mut value, "PTT-003", |case| {
            case["candidate_input"]["splits"]["source_train"]["unit_ids"]
                .as_array_mut()
                .expect("array")
                .push(serde_json::json!("sensor-target-01"));
        });
        assert!(parse_and_validate(&value).is_err());
    }

    #[test]
    fn few_shot_adaptation_and_final_test_must_use_disjoint_contact_sequences() {
        let mut value = tracked_value();
        mutate_case(&mut value, "PTT-003", |case| {
            case["candidate_input"]["splits"]["target_test"]["sequence_ids"][0] =
                serde_json::json!("adapt-sequence-01");
        });
        assert!(parse_and_validate(&value).is_err());
    }

    #[test]
    fn source_training_must_not_overlap_target_adaptation_samples() {
        let mut value = tracked_value();
        mutate_case(&mut value, "PTT-003", |case| {
            case["candidate_input"]["splits"]["target_adaptation"]["sample_ids"][0] =
                serde_json::json!("src-sample-01");
        });
        assert!(parse_and_validate(&value).is_err());
    }

    #[test]
    fn source_training_must_not_overlap_target_validation_sequences() {
        let mut value = tracked_value();
        mutate_case(&mut value, "PTT-003", |case| {
            case["candidate_input"]["splits"]["target_validation"]["sequence_ids"][0] =
                serde_json::json!("src-sequence-01");
        });
        assert!(parse_and_validate(&value).is_err());
    }

    #[test]
    fn source_sensor_identity_must_not_appear_in_any_target_partition() {
        let mut value = tracked_value();
        mutate_case(&mut value, "PTT-003", |case| {
            case["candidate_input"]["splits"]["target_validation"]["unit_ids"]
                .as_array_mut()
                .expect("array")
                .push(serde_json::json!("sensor-source-01"));
        });
        assert!(parse_and_validate(&value).is_err());
    }

    #[test]
    fn source_manufacturing_lot_must_not_appear_in_target_adaptation() {
        let mut value = tracked_value();
        mutate_case(&mut value, "PTT-003", |case| {
            case["candidate_input"]["splits"]["target_adaptation"]["lot_ids"]
                .as_array_mut()
                .expect("array")
                .push(serde_json::json!("lot-source-01"));
        });
        assert!(parse_and_validate(&value).is_err());
    }


    #[test]
    fn a_source_certificate_cannot_be_reused_as_the_target_unit_calibration() {
        let mut value = tracked_value();
        mutate_case(&mut value, "PTT-003", |case| {
            case["candidate_input"]["target_calibration"]["calibration_id"] =
                serde_json::json!("cal-source-01");
            case["candidate_input"]["model"]["target_calibration_id"] =
                serde_json::json!("cal-source-01");
            case["evaluator_only"]["expected"]["metrology_disposition"] =
                serde_json::json!("invalid_measurement_claim");
            case["evaluator_only"]["expected"]["recalibration_disposition"] =
                serde_json::json!("required");
        });
        assert!(parse_and_validate(&value).is_ok());
    }

    #[test]
    fn model_estimate_cannot_promote_itself_to_a_calibrated_measurement() {
        let mut value = tracked_value();
        mutate_case(&mut value, "PTT-029", |case| {
            case["evaluator_only"]["expected"]["metrology_disposition"] =
                serde_json::json!("target_qualified_estimate");
        });
        assert!(parse_and_validate(&value).is_err());
    }

    #[test]
    fn stale_target_firmware_requires_recalibration() {
        let mut value = tracked_value();
        mutate_case(&mut value, "PTT-003", |case| {
            case["candidate_input"]["target"]["firmware_revision"] = serde_json::json!("fw-2");
        });
        assert!(parse_and_validate(&value).is_err());
    }

    #[test]
    fn independent_reference_and_uncertainty_coverage_are_required() {
        let mut value = tracked_value();
        mutate_case(&mut value, "PTT-003", |case| {
            case["candidate_input"]["transfer_validation"]["independent_reference"] =
                serde_json::json!(false);
            case["evaluator_only"]["expected"]["metrology_disposition"] =
                serde_json::json!("invalid_measurement_claim");
        });
        assert!(parse_and_validate(&value).is_ok());
    }

    #[test]
    fn a_different_target_validation_unit_cannot_qualify_this_sensor() {
        let mut value = tracked_value();
        mutate_case(&mut value, "PTT-003", |case| {
            case["candidate_input"]["transfer_validation"]["target_unit_id"] =
                serde_json::json!("sensor-other-99");
            case["evaluator_only"]["expected"]["metrology_disposition"] =
                serde_json::json!("invalid_measurement_claim");
        });
        assert!(parse_and_validate(&value).is_ok());
    }

    #[test]
    fn duplicate_json_members_are_rejected_by_struct_deserialization() {
        let raw = r#"{"schema_version":"one","schema_version":"two"}"#;
        assert!(serde_json::from_str::<Corpus>(raw).is_err());
    }

    #[test]
    fn unknown_nested_fields_are_rejected() {
        let mut value = tracked_value();
        mutate_case(&mut value, "PTT-003", |case| {
            case["candidate_input"]["target"]["hidden_force_label"] = serde_json::json!(12.0);
        });
        let raw = serde_json::to_string(&value).expect("valid JSON");
        assert!(serde_json::from_str::<Corpus>(&raw).is_err());
    }
}
