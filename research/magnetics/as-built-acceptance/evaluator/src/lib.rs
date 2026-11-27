//! Evidence-gated specimen-level magnetic functional acceptance.
//!
//! This crate validates semantic joins over upstream evidence. It is not a
//! magnetic-field solver, uncertainty engine, measurement system, or release
//! authority. Digests identify content; they do not authenticate producers.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scope {
    pub requirement_id: String,
    pub frame_id: String,
    pub grid_id: String,
    pub operating_condition_id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceRole {
    ProcessRecord,
    AsBuiltInspection,
    MaterialCharacterization,
    PostBuildMagnetization,
    Predicted,
    PhysicalObservation,
    IndependentReference,
    ModelDiscrepancy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceRecord {
    pub id: String,
    pub digest: String,
    pub specimen_id: String,
    pub role: EvidenceRole,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManufacturingEvidence {
    pub record: EvidenceRecord,
    pub material_lot_id: String,
    pub build_run_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeometryEvidence {
    pub inspection_record: EvidenceRecord,
    pub nominal_geometry_digest: String,
    pub as_built_geometry_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SolverEvidence {
    pub record: EvidenceRecord,
    pub scope: Scope,
    pub scope_supported: bool,
    pub geometry_digest: String,
    pub material_state_record_id: String,
    pub magnetization_record_id: String,
    pub manufacturing_record_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalibrationEvidence {
    pub record_id: String,
    pub digest: String,
    pub instrument_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelDiscrepancyEvidence {
    pub record: EvidenceRecord,
    pub scope: Scope,
    pub geometry_digest: String,
    pub material_state_record_id: String,
    pub solver_record_id: String,
    pub observation_record_id: String,
    pub qualified: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservationEvidence {
    pub record: EvidenceRecord,
    pub scope: Scope,
    pub instrument_id: String,
    pub calibration: CalibrationEvidence,
    pub background_field_treatment_id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Safe,
    Fail,
    Unresolved,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComparisonEvidence {
    pub record_id: String,
    pub digest: String,
    pub prediction_record_id: String,
    pub observation_record_id: String,
    pub scope: Scope,
    pub verdict: Verdict,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequirementEvidence {
    pub requirement_id: String,
    pub record: EvidenceRecord,
    pub scope: Scope,
    pub verdict: Verdict,
    /// True only when the result's uncertainty is resolved against the
    /// preregistered requirement limit (not merely a point estimate).
    pub uncertainty_resolved: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpatialCertificate {
    pub certificate_id: String,
    pub digest: String,
    pub specimen_id: String,
    pub scope: Scope,
    pub geometry_digest: String,
    pub material_state_record_id: String,
    pub complete_domain_coverage: bool,
    pub verdict: Verdict,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UncertaintyEvidence {
    pub record_id: String,
    pub digest: String,
    pub scope_complete: bool,
    pub common_cause_source_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcceptanceCase {
    pub specimen_id: String,
    pub material_lot_id: String,
    pub build_run_id: String,
    pub scope: Scope,
    pub acceptance_policy_digest: String,
    pub manufacturing: ManufacturingEvidence,
    pub geometry: GeometryEvidence,
    pub material_state: EvidenceRecord,
    pub post_magnetization: EvidenceRecord,
    pub solver: SolverEvidence,
    pub model_discrepancy: ModelDiscrepancyEvidence,
    pub observation: ObservationEvidence,
    pub comparison: ComparisonEvidence,
    pub spatial_certificate: SpatialCertificate,
    pub hard_requirements: Vec<RequirementEvidence>,
    pub blind_holdout_record_ids: Vec<String>,
    pub uncertainty: UncertaintyEvidence,
    pub measurement_scope_supported: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Disposition {
    Qualified,
    Fail,
    Indeterminate,
    NotEvaluable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Blocker {
    UnsupportedMeasurementScope,
    UnsupportedSolverScope,
    MissingIdentity,
    InvalidSha256Digest,
    WrongSpecimen,
    WrongEvidenceRole,
    WrongScope,
    ManufacturingLotMismatch,
    ManufacturingBuildMismatch,
    AsBuiltGeometryMismatch,
    MaterialStateBindingMismatch,
    MagnetizationBindingMismatch,
    ManufacturingBindingMismatch,
    MissingCalibrationEvidence,
    CalibrationInstrumentMismatch,
    MissingBlindHoldoutEvidence,
    MissingBackgroundFieldTreatment,
    ComparisonPredictionMismatch,
    ComparisonObservationMismatch,
    SpatialSpecimenMismatch,
    SpatialGeometryMismatch,
    SpatialMaterialBindingMismatch,
    NoHardRequirements,
    InvalidRequirementEvidence,
    HoldoutLeakage,
    IncompleteUncertainty,
    MissingCommonCauseEvidence,
    UnqualifiedModelDiscrepancy,
    ModelDiscrepancyBindingMismatch,
    MissingUncertaintyRecord,
    DefiniteHardRequirementFailure,
    UnresolvedHardRequirement,
    ComparisonFailure,
    UnresolvedComparison,
    IncompleteSpatialCoverage,
    SpatialMarginFailure,
    UnresolvedSpatialMargin,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Evaluation {
    pub disposition: Disposition,
    pub blockers: Vec<Blocker>,
}

impl Evaluation {
    fn new(disposition: Disposition, blockers: Vec<Blocker>) -> Self {
        Self { disposition, blockers }
    }
}

/// Evaluate evidence closure and the non-compensatory specimen disposition.
///
/// This function only checks identities, roles, scopes and supplied verdicts.
/// It does not independently establish that a solver, instrument, calibration,
/// physical model or evidence producer is truthful or competent.
pub fn evaluate(case: &AcceptanceCase) -> Evaluation {
    if !case.measurement_scope_supported {
        return Evaluation::new(
            Disposition::NotEvaluable,
            vec![Blocker::UnsupportedMeasurementScope],
        );
    }
    if !case.solver.scope_supported {
        return Evaluation::new(
            Disposition::NotEvaluable,
            vec![Blocker::UnsupportedSolverScope],
        );
    }

    let mut blockers = Vec::new();

    if case.specimen_id.trim().is_empty()
        || case.material_lot_id.trim().is_empty()
        || case.build_run_id.trim().is_empty()
        || case.scope.requirement_id.trim().is_empty()
        || case.scope.frame_id.trim().is_empty()
        || case.scope.grid_id.trim().is_empty()
        || case.scope.operating_condition_id.trim().is_empty()
        || case.acceptance_policy_digest.trim().is_empty()
    {
        blockers.push(Blocker::MissingIdentity);
    }

    if !valid_sha256(&case.acceptance_policy_digest) {
        blockers.push(Blocker::InvalidSha256Digest);
    }

    check_record(
        &case.manufacturing.record,
        &case.specimen_id,
        EvidenceRole::ProcessRecord,
        &mut blockers,
    );
    check_record(
        &case.geometry.inspection_record,
        &case.specimen_id,
        EvidenceRole::AsBuiltInspection,
        &mut blockers,
    );
    check_record(
        &case.material_state,
        &case.specimen_id,
        EvidenceRole::MaterialCharacterization,
        &mut blockers,
    );
    check_record(
        &case.post_magnetization,
        &case.specimen_id,
        EvidenceRole::PostBuildMagnetization,
        &mut blockers,
    );
    check_record(
        &case.solver.record,
        &case.specimen_id,
        EvidenceRole::Predicted,
        &mut blockers,
    );
    check_record(
        &case.observation.record,
        &case.specimen_id,
        EvidenceRole::PhysicalObservation,
        &mut blockers,
    );
    check_record(
        &case.model_discrepancy.record,
        &case.specimen_id,
        EvidenceRole::ModelDiscrepancy,
        &mut blockers,
    );

    if case.manufacturing.material_lot_id != case.material_lot_id {
        blockers.push(Blocker::ManufacturingLotMismatch);
    }
    if case.manufacturing.build_run_id != case.build_run_id {
        blockers.push(Blocker::ManufacturingBuildMismatch);
    }
    if case.solver.geometry_digest != case.geometry.as_built_geometry_digest {
        blockers.push(Blocker::AsBuiltGeometryMismatch);
    }
    if case.solver.material_state_record_id != case.material_state.id {
        blockers.push(Blocker::MaterialStateBindingMismatch);
    }
    if case.solver.magnetization_record_id != case.post_magnetization.id {
        blockers.push(Blocker::MagnetizationBindingMismatch);
    }
    if case.solver.manufacturing_record_id != case.manufacturing.record.id {
        blockers.push(Blocker::ManufacturingBindingMismatch);
    }
    if case.solver.scope != case.scope
        || case.model_discrepancy.scope != case.scope
        || case.observation.scope != case.scope
    {
        blockers.push(Blocker::WrongScope);
    }
    if case.observation.instrument_id.trim().is_empty()
        || case.observation.calibration.record_id.trim().is_empty()
        || !valid_sha256(&case.observation.calibration.digest)
        || case.observation.calibration.instrument_id.trim().is_empty()
    {
        blockers.push(Blocker::MissingCalibrationEvidence);
    }
    if case.observation.calibration.instrument_id != case.observation.instrument_id {
        blockers.push(Blocker::CalibrationInstrumentMismatch);
    }
    if case.observation.background_field_treatment_id.trim().is_empty() {
        blockers.push(Blocker::MissingBackgroundFieldTreatment);
    }
    if case.model_discrepancy.geometry_digest != case.geometry.as_built_geometry_digest
        || case.model_discrepancy.material_state_record_id != case.material_state.id
        || case.model_discrepancy.solver_record_id != case.solver.record.id
        || case.model_discrepancy.observation_record_id != case.observation.record.id
    {
        blockers.push(Blocker::ModelDiscrepancyBindingMismatch);
    }

    if case.comparison.record_id.trim().is_empty() {
        blockers.push(Blocker::MissingIdentity);
    }
    if !valid_sha256(&case.comparison.digest) {
        blockers.push(Blocker::InvalidSha256Digest);
    }
    if case.comparison.prediction_record_id != case.solver.record.id {
        blockers.push(Blocker::ComparisonPredictionMismatch);
    }
    if case.comparison.observation_record_id != case.observation.record.id {
        blockers.push(Blocker::ComparisonObservationMismatch);
    }
    if case.comparison.scope != case.scope {
        blockers.push(Blocker::WrongScope);
    }

    if case.spatial_certificate.certificate_id.trim().is_empty() {
        blockers.push(Blocker::MissingIdentity);
    }
    if !valid_sha256(&case.spatial_certificate.digest) {
        blockers.push(Blocker::InvalidSha256Digest);
    }
    if case.spatial_certificate.specimen_id != case.specimen_id {
        blockers.push(Blocker::SpatialSpecimenMismatch);
    }
    if case.spatial_certificate.scope != case.scope {
        blockers.push(Blocker::WrongScope);
    }
    if case.spatial_certificate.geometry_digest != case.geometry.as_built_geometry_digest {
        blockers.push(Blocker::SpatialGeometryMismatch);
    }
    if case.spatial_certificate.material_state_record_id != case.material_state.id {
        blockers.push(Blocker::SpatialMaterialBindingMismatch);
    }
    if !valid_sha256(&case.geometry.nominal_geometry_digest)
        || !valid_sha256(&case.geometry.as_built_geometry_digest)
    {
        blockers.push(Blocker::InvalidSha256Digest);
    }

    if case.hard_requirements.is_empty() {
        blockers.push(Blocker::NoHardRequirements);
    }
    for requirement in &case.hard_requirements {
        check_record(
            &requirement.record,
            &case.specimen_id,
            requirement.record.role,
            &mut blockers,
        );
        if requirement.requirement_id != case.scope.requirement_id
            || requirement.scope != case.scope
            || !matches!(
                requirement.record.role,
                EvidenceRole::PhysicalObservation | EvidenceRole::IndependentReference
            )
        {
            blockers.push(Blocker::InvalidRequirementEvidence);
        }
    }

    if case.blind_holdout_record_ids.is_empty()
        || case.blind_holdout_record_ids.iter().any(|id| id.trim().is_empty())
        || !case
            .blind_holdout_record_ids
            .iter()
            .any(|id| id == &case.observation.record.id)
    {
        blockers.push(Blocker::MissingBlindHoldoutEvidence);
    }
    if case.blind_holdout_record_ids.iter().any(|holdout_id| {
        holdout_id == &case.observation.calibration.record_id
            || case
                .fit_calibration_record_ids
                .iter()
                .any(|fit_id| fit_id == holdout_id)
    }) {
        blockers.push(Blocker::HoldoutLeakage);
    }
    if !case.uncertainty.scope_complete {
        blockers.push(Blocker::IncompleteUncertainty);
    }
    if case.uncertainty.record_id.trim().is_empty() {
        blockers.push(Blocker::MissingUncertaintyRecord);
    }
    if !valid_sha256(&case.uncertainty.digest) {
        blockers.push(Blocker::InvalidSha256Digest);
    }
    if case.uncertainty.common_cause_source_ids.is_empty()
        || case
            .uncertainty
            .common_cause_source_ids
            .iter()
            .any(|id| id.trim().is_empty())
    {
        blockers.push(Blocker::MissingCommonCauseEvidence);
    }
    if !case.model_discrepancy.qualified {
        blockers.push(Blocker::UnqualifiedModelDiscrepancy);
    }

    if !blockers.is_empty() {
        return Evaluation::new(Disposition::Indeterminate, blockers);
    }

    // From this point, all evidence identities and scopes are closed.
    // A definite failure is therefore admissible and non-compensatory.
    let mut failures = Vec::new();
    let mut unresolved = Vec::new();

    if case.comparison.verdict == Verdict::Fail {
        failures.push(Blocker::ComparisonFailure);
    } else if case.comparison.verdict == Verdict::Unresolved {
        unresolved.push(Blocker::UnresolvedComparison);
    }

    for requirement in &case.hard_requirements {
        match requirement.verdict {
            Verdict::Safe => {}
            Verdict::Fail if requirement.uncertainty_resolved => {
                failures.push(Blocker::DefiniteHardRequirementFailure);
            }
            Verdict::Fail | Verdict::Unresolved => {
                unresolved.push(Blocker::UnresolvedHardRequirement);
            }
        }
    }

    match case.spatial_certificate.verdict {
        Verdict::Fail => failures.push(Blocker::SpatialMarginFailure),
        Verdict::Unresolved => unresolved.push(Blocker::UnresolvedSpatialMargin),
        Verdict::Safe if !case.spatial_certificate.complete_domain_coverage => {
            unresolved.push(Blocker::IncompleteSpatialCoverage);
        }
        Verdict::Safe => {}
    }

    if !failures.is_empty() {
        return Evaluation::new(Disposition::Fail, failures);
    }
    if !unresolved.is_empty() {
        return Evaluation::new(Disposition::Indeterminate, unresolved);
    }

    Evaluation::new(Disposition::Qualified, Vec::new())
}

fn check_record(
    record: &EvidenceRecord,
    specimen_id: &str,
    expected_role: EvidenceRole,
    blockers: &mut Vec<Blocker>,
) {
    if record.id.trim().is_empty()
        || record.specimen_id.trim().is_empty()
        || record.digest.trim().is_empty()
    {
        blockers.push(Blocker::MissingIdentity);
    }
    if !valid_sha256(&record.digest) {
        blockers.push(Blocker::InvalidSha256Digest);
    }
    if record.specimen_id != specimen_id {
        blockers.push(Blocker::WrongSpecimen);
    }
    if record.role != expected_role {
        blockers.push(Blocker::WrongEvidenceRole);
    }
}

fn valid_sha256(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: char) -> String {
        format!("sha256:{}", byte.to_string().repeat(64))
    }

    fn record(id: &str, specimen: &str, role: EvidenceRole, hash: char) -> EvidenceRecord {
        EvidenceRecord {
            id: id.to_string(),
            digest: digest(hash),
            specimen_id: specimen.to_string(),
            role,
        }
    }

    fn observation_record_id_for_holdout() -> String {
        "OBS-001".to_string()
    }

    fn valid_case() -> AcceptanceCase {
        let specimen = "SPEC-001".to_string();
        let scope = Scope {
            requirement_id: "REQ-FIELD-001".to_string(),
            frame_id: "FRAME-001".to_string(),
            grid_id: "GRID-001".to_string(),
            operating_condition_id: "COND-001".to_string(),
        };
        let manufacturing = ManufacturingEvidence {
            record: record("PROC-001", &specimen, EvidenceRole::ProcessRecord, '1'),
            material_lot_id: "LOT-001".to_string(),
            build_run_id: "BUILD-001".to_string(),
        };
        let geometry = GeometryEvidence {
            inspection_record: record("GEOM-001", &specimen, EvidenceRole::AsBuiltInspection, '2'),
            nominal_geometry_digest: digest('a'),
            as_built_geometry_digest: digest('b'),
        };
        let material_state = record(
            "MAT-001",
            &specimen,
            EvidenceRole::MaterialCharacterization,
            '3',
        );
        let post_magnetization = record(
            "MAG-001",
            &specimen,
            EvidenceRole::PostBuildMagnetization,
            '4',
        );
        let solver_record = record("SOLVER-001", &specimen, EvidenceRole::Predicted, '5');
        let observation_record = record(
            "OBS-001",
            &specimen,
            EvidenceRole::PhysicalObservation,
            '6',
        );
        let calibration_id = "CAL-REF-001".to_string();

        AcceptanceCase {
            specimen_id: specimen.clone(),
            material_lot_id: "LOT-001".to_string(),
            build_run_id: "BUILD-001".to_string(),
            scope: scope.clone(),
            acceptance_policy_digest: digest('c'),
            manufacturing: manufacturing.clone(),
            geometry: geometry.clone(),
            material_state: material_state.clone(),
            post_magnetization: post_magnetization.clone(),
            solver: SolverEvidence {
                record: solver_record.clone(),
                scope: scope.clone(),
                scope_supported: true,
                geometry_digest: geometry.as_built_geometry_digest.clone(),
                material_state_record_id: material_state.id.clone(),
                magnetization_record_id: post_magnetization.id.clone(),
                manufacturing_record_id: manufacturing.record.id.clone(),
            },
            model_discrepancy: ModelDiscrepancyEvidence {
                record: record("MODEL-DIFF-001", &specimen, EvidenceRole::ModelDiscrepancy, '9'),
                scope: scope.clone(),
                geometry_digest: geometry.as_built_geometry_digest.clone(),
                material_state_record_id: material_state.id.clone(),
                solver_record_id: solver_record.id.clone(),
                observation_record_id: observation_record.id.clone(),
                qualified: true,
            },
            observation: ObservationEvidence {
                record: observation_record.clone(),
                scope: scope.clone(),
                instrument_id: "PROBE-001".to_string(),
                calibration: CalibrationEvidence {
                    record_id: calibration_id.clone(),
                    digest: digest('d'),
                    instrument_id: "PROBE-001".to_string(),
                },
                background_field_treatment_id: "BG-CORRECTION-001".to_string(),
            },
            comparison: ComparisonEvidence {
                record_id: "CMP-001".to_string(),
                digest: digest('7'),
                prediction_record_id: solver_record.id.clone(),
                observation_record_id: observation_record.id.clone(),
                scope: scope.clone(),
                verdict: Verdict::Safe,
            },
            spatial_certificate: SpatialCertificate {
                certificate_id: "SPACE-001".to_string(),
                digest: digest('8'),
                specimen_id: specimen.clone(),
                scope: scope.clone(),
                geometry_digest: geometry.as_built_geometry_digest.clone(),
                material_state_record_id: material_state.id.clone(),
                complete_domain_coverage: true,
                verdict: Verdict::Safe,
            },
            hard_requirements: vec![RequirementEvidence {
                requirement_id: scope.requirement_id.clone(),
                record: observation_record,
                scope: scope.clone(),
                verdict: Verdict::Safe,
                uncertainty_resolved: true,
            }],
            blind_holdout_record_ids: vec![observation_record_id_for_holdout()],
            uncertainty: UncertaintyEvidence {
                record_id: "UNCERTAINTY-001".to_string(),
                digest: digest('e'),
                scope_complete: true,
                common_cause_source_ids: vec!["COMMON-CAL-001".to_string()],
            },
            measurement_scope_supported: true,
        }
    }

    #[test]
    fn complete_scope_matched_specimen_qualifies() {
        let result = evaluate(&valid_case());
        assert_eq!(result.disposition, Disposition::Qualified);
        assert!(result.blockers.is_empty());
    }

    #[test]
    fn nominal_geometry_cannot_substitute_for_as_built_geometry() {
        let mut case = valid_case();
        case.solver.geometry_digest = case.geometry.nominal_geometry_digest.clone();
        let result = evaluate(&case);
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(result.blockers.contains(&Blocker::AsBuiltGeometryMismatch));
    }

    #[test]
    fn measurement_from_another_specimen_is_rejected() {
        let mut case = valid_case();
        case.observation.record.specimen_id = "SPEC-OTHER".to_string();
        let result = evaluate(&case);
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(result.blockers.contains(&Blocker::WrongSpecimen));
    }

    #[test]
    fn missing_magnetization_binding_is_rejected() {
        let mut case = valid_case();
        case.solver.magnetization_record_id = "MAG-MISSING".to_string();
        let result = evaluate(&case);
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(result.blockers.contains(&Blocker::MagnetizationBindingMismatch));
    }

    #[test]
    fn definite_hard_requirement_failure_is_non_compensatory() {
        let mut case = valid_case();
        case.hard_requirements[0].verdict = Verdict::Fail;
        let result = evaluate(&case);
        assert_eq!(result.disposition, Disposition::Fail);
        assert!(result
            .blockers
            .contains(&Blocker::DefiniteHardRequirementFailure));
    }

    #[test]
    fn unresolved_hard_requirement_does_not_pass() {
        let mut case = valid_case();
        case.hard_requirements[0].verdict = Verdict::Fail;
        case.hard_requirements[0].uncertainty_resolved = false;
        let result = evaluate(&case);
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(result.blockers.contains(&Blocker::UnresolvedHardRequirement));
    }

    #[test]
    fn incomplete_spatial_coverage_does_not_pass() {
        let mut case = valid_case();
        case.spatial_certificate.complete_domain_coverage = false;
        let result = evaluate(&case);
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(result.blockers.contains(&Blocker::IncompleteSpatialCoverage));
    }

    #[test]
    fn calibration_holdout_overlap_is_rejected() {
        let mut case = valid_case();
        case.blind_holdout_record_ids.push("CAL-REF-001".to_string());
        let result = evaluate(&case);
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(result.blockers.contains(&Blocker::HoldoutLeakage));
    }

    #[test]
    fn calibration_must_belong_to_the_observation_instrument() {
        let mut case = valid_case();
        case.observation.calibration.instrument_id = "OTHER-PROBE".to_string();
        let result = evaluate(&case);
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(result
            .blockers
            .contains(&Blocker::CalibrationInstrumentMismatch));
    }

    #[test]
    fn missing_blind_holdout_does_not_pass() {
        let mut case = valid_case();
        case.blind_holdout_record_ids.clear();
        let result = evaluate(&case);
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(result
            .blockers
            .contains(&Blocker::MissingBlindHoldoutEvidence));
    }

    #[test]
    fn model_discrepancy_must_bind_exact_prediction_and_observation() {
        let mut case = valid_case();
        case.model_discrepancy.observation_record_id = "OBS-OTHER".to_string();
        let result = evaluate(&case);
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(result
            .blockers
            .contains(&Blocker::ModelDiscrepancyBindingMismatch));
    }

    #[test]
    fn model_fit_observation_cannot_leak_into_blind_holdout() {
        let mut case = valid_case();
        case.fit_calibration_record_ids.push("OBS-001".to_string());
        let result = evaluate(&case);
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(result.blockers.contains(&Blocker::HoldoutLeakage));
    }

    #[test]
    fn uncertainty_budget_requires_digest_bound_record() {
        let mut case = valid_case();
        case.uncertainty.record_id.clear();
        let result = evaluate(&case);
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(result.blockers.contains(&Blocker::MissingUncertaintyRecord));
    }

    #[test]
    fn missing_common_cause_evidence_does_not_pass() {
        let mut case = valid_case();
        case.uncertainty.common_cause_source_ids.clear();
        let result = evaluate(&case);
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(result
            .blockers
            .contains(&Blocker::MissingCommonCauseEvidence));
    }

    #[test]
    fn unsupported_solver_regime_is_not_evaluable() {
        let mut case = valid_case();
        case.solver.scope_supported = false;
        let result = evaluate(&case);
        assert_eq!(result.disposition, Disposition::NotEvaluable);
        assert_eq!(result.blockers, vec![Blocker::UnsupportedSolverScope]);
    }

    #[test]
    fn comparison_must_bind_both_exact_artifacts() {
        let mut case = valid_case();
        case.comparison.observation_record_id = "OBS-OTHER".to_string();
        let result = evaluate(&case);
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(result
            .blockers
            .contains(&Blocker::ComparisonObservationMismatch));
    }

    #[test]
    fn comparison_receipt_requires_its_own_identity_and_digest() {
        let mut case = valid_case();
        case.comparison.digest = "not-a-sha256".to_string();
        let result = evaluate(&case);
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(result.blockers.contains(&Blocker::InvalidSha256Digest));
    }

    #[test]
    fn spatial_certificate_requires_its_own_identity_and_digest() {
        let mut case = valid_case();
        case.spatial_certificate.certificate_id.clear();
        let result = evaluate(&case);
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(result.blockers.contains(&Blocker::MissingIdentity));
    }

    #[test]
    fn spatial_failure_is_not_hidden_by_other_safe_metrics() {
        let mut case = valid_case();
        case.spatial_certificate.verdict = Verdict::Fail;
        let result = evaluate(&case);
        assert_eq!(result.disposition, Disposition::Fail);
        assert!(result.blockers.contains(&Blocker::SpatialMarginFailure));
    }

    #[test]
    fn widening_uncertainty_cannot_upgrade_disposition() {
        let mut case = valid_case();
        case.spatial_certificate.complete_domain_coverage = false;
        let before = evaluate(&case).disposition;
        case.uncertainty.scope_complete = false;
        let after = evaluate(&case).disposition;
        assert_ne!(before, Disposition::Qualified);
        assert_ne!(after, Disposition::Qualified);
    }
}
