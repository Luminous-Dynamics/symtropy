//! Evidence and convergence contract for matched magnetic solver comparisons.
//!
//! This crate does not solve magnetostatic problems. It validates the provenance,
//! applicability, convergence record and comparability of external results.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MagneticModelClass {
    UniformPrescribedMagnetization,
    MagnetWithLinearIsotropicSoftMaterial,
    MagnetWithAnisotropicSoftMaterial,
    MagnetWithNonlinearBhMaterial,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SolverMethod {
    AnalyticalClosedForm,
    IndependentSurfaceQuadrature,
    SurfaceFaceCharge,
    CellVolume,
    FiniteElement,
    PhysicalMetrology,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FloatingPointPrecision {
    F32,
    F64,
    Mixed,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SolverIdentity {
    pub software_id: String,
    pub version: String,
    /// Tag, release identifier or exact source revision.
    pub source_revision: String,
    /// Digest of an archived source or build artifact, not an authentication claim.
    pub source_archive_sha256: String,
    /// Declared algorithm lineage, e.g. "magpylib-analytical-cuboid".
    pub implementation_family: String,
    /// Declared direct dependencies relevant to numerical lineage.
    pub direct_dependency_ids: Vec<String>,
    /// Declared mathematical formulation, e.g. "closed-form-surface-charge".
    pub formulation_id: String,
    pub method: SolverMethod,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MagneticProblemIdentity {
    pub model_class: MagneticModelClass,
    /// Digest of the exact geometry and placement.
    pub geometry_sha256: String,
    /// Digest of the exact material grades and constitutive-law parameters.
    pub material_sha256: String,
    /// Digest of all non-geometric physical inputs, including source strengths,
    /// applied fields, temperature and magnetization state.
    pub physics_input_sha256: String,
    /// Opaque, externally managed coordinate-frame identity.
    pub coordinate_frame_id: String,
    /// Digest of the exact ordered observer coordinates, not merely grid dimensions.
    pub observer_grid_sha256: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RefinementObservation {
    /// Monotonically increasing ordinal, not a method-specific mesh-level meaning.
    pub refinement_index: u32,
    /// Characteristic element/panel/cell size in metres; must decrease with refinement.
    pub characteristic_resolution_m: f64,
    /// Maximum field-map change from the previous refinement, in tesla.
    /// The coarsest level has no preceding level and therefore uses None.
    pub maximum_map_change_t: Option<f64>,
    /// Solver-reported absolute field-error estimate at this level, in tesla.
    pub estimated_absolute_error_t: Option<f64>,
    /// Solver-specific residual norm when available; not comparable across solvers.
    pub residual_norm: Option<f64>,
    pub output_map_sha256: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConvergenceRecord {
    pub policy_id: String,
    /// A solver's declaration, checked against the explicit policy below.
    pub declared_converged: bool,
    pub refinements: Vec<RefinementObservation>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConvergencePolicy {
    pub policy_id: String,
    pub minimum_refinement_levels: usize,
    pub maximum_final_map_change_t: f64,
    pub maximum_final_error_estimate_t: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SolverRunRecord {
    pub run_id: String,
    pub solver: SolverIdentity,
    pub problem: MagneticProblemIdentity,
    /// Digest of an archived environment/lockfile and runtime configuration.
    pub environment_sha256: String,
    pub output_map_sha256: String,
    pub precision: FloatingPointPrecision,
    pub runtime_nanoseconds: u64,
    pub peak_memory_bytes: Option<u64>,
    pub unknown_count: Option<u64>,
    /// None is valid for closed-form analytical references and physical observations.
    pub convergence: Option<ConvergenceRecord>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineageRelation {
    SameDeclaredImplementationAndFormulation,
    DifferentFormulationSameSoftware,
    DifferentSoftwareSameDeclaredFormulation,
    DifferentSoftwareAndFormulation,
    PhysicalMeasurementVersusPrediction,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComparablePair {
    pub left_run_id: String,
    pub right_run_id: String,
    pub lineage_relation: LineageRelation,
    /// This means required identity fields match; it does not mean either model is
    /// physically correct, solver implementations are truly independent, or
    /// outputs agree.
    pub physical_inputs_and_observer_grid_match: bool,
    /// Sorted intersection of the declared dependency identifiers.
    pub shared_dependency_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QualificationError {
    EmptyIdentity,
    InvalidSha256 { field: &'static str },
    IncompatibleMethodAndModel,
    MissingConvergenceRecord,
    UnexpectedConvergenceRecord,
    InvalidConvergencePolicy,
    UnknownNumericalPrecision,
    InvalidRefinementCount,
    InvalidRefinementOrder { index: usize },
    InvalidRefinementValue { index: usize },
    MissingFinalConvergenceEvidence,
    DeclaredConvergedWithoutMeetingPolicy,
    OutputDigestDoesNotMatchFinalRefinement,
    ProblemModelMismatch,
    GeometryMismatch,
    MaterialMismatch,
    PhysicsInputMismatch,
    CoordinateFrameMismatch,
    ObserverGridMismatch,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_digest(value: &str, field: &'static str) -> Result<(), QualificationError> {
    if valid_sha256(value) {
        Ok(())
    } else {
        Err(QualificationError::InvalidSha256 { field })
    }
}

fn method_supports_model(method: SolverMethod, model: MagneticModelClass) -> bool {
    match (method, model) {
        (
            SolverMethod::AnalyticalClosedForm | SolverMethod::IndependentSurfaceQuadrature,
            MagneticModelClass::UniformPrescribedMagnetization,
        ) => true,
        (
            SolverMethod::SurfaceFaceCharge,
            MagneticModelClass::MagnetWithLinearIsotropicSoftMaterial,
        ) => true,
        (
            SolverMethod::CellVolume,
            MagneticModelClass::MagnetWithLinearIsotropicSoftMaterial
            | MagneticModelClass::MagnetWithAnisotropicSoftMaterial
            | MagneticModelClass::MagnetWithNonlinearBhMaterial,
        ) => true,
        (
            SolverMethod::FiniteElement,
            MagneticModelClass::UniformPrescribedMagnetization
            | MagneticModelClass::MagnetWithLinearIsotropicSoftMaterial
            | MagneticModelClass::MagnetWithAnisotropicSoftMaterial
            | MagneticModelClass::MagnetWithNonlinearBhMaterial,
        ) => true,
        (SolverMethod::PhysicalMetrology, _) => true,
        _ => false,
    }
}

fn method_requires_convergence(method: SolverMethod) -> bool {
    matches!(
        method,
        SolverMethod::IndependentSurfaceQuadrature
            | SolverMethod::SurfaceFaceCharge
            | SolverMethod::CellVolume
            | SolverMethod::FiniteElement
    )
}

fn validate_convergence(
    convergence: &ConvergenceRecord,
    output_digest: &str,
    policy: ConvergencePolicy,
) -> Result<(), QualificationError> {
    if !nonempty(&convergence.policy_id)
        || !nonempty(&policy.policy_id)
        || convergence.policy_id != policy.policy_id
        || policy.minimum_refinement_levels < 2
        || !policy.maximum_final_map_change_t.is_finite()
        || policy.maximum_final_map_change_t < 0.0
        || !policy.maximum_final_error_estimate_t.is_finite()
        || policy.maximum_final_error_estimate_t < 0.0
    {
        return Err(QualificationError::InvalidConvergencePolicy);
    }

    if convergence.refinements.len() < 2 {
        return Err(QualificationError::InvalidRefinementCount);
    }

    for (index, level) in convergence.refinements.iter().enumerate() {
        if !valid_sha256(&level.output_map_sha256) {
            return Err(QualificationError::InvalidSha256 {
                field: "refinement.output_map_sha256",
            });
        }
        if !level.characteristic_resolution_m.is_finite()
            || level.characteristic_resolution_m <= 0.0
            || level
                .maximum_map_change_t
                .is_some_and(|value| !value.is_finite() || value < 0.0)
            || level
                .estimated_absolute_error_t
                .is_some_and(|value| !value.is_finite() || value < 0.0)
            || level
                .residual_norm
                .is_some_and(|value| !value.is_finite() || value < 0.0)
            || (index == 0 && level.maximum_map_change_t.is_some())
            || (index > 0 && level.maximum_map_change_t.is_none())
        {
            return Err(QualificationError::InvalidRefinementValue { index });
        }

        if index > 0 {
            let previous = &convergence.refinements[index - 1];
            if level.refinement_index <= previous.refinement_index
                || level.characteristic_resolution_m >= previous.characteristic_resolution_m
            {
                return Err(QualificationError::InvalidRefinementOrder { index });
            }
        }
    }

    let final_level = convergence
        .refinements
        .last()
        .ok_or(QualificationError::InvalidRefinementCount)?;

    if final_level.output_map_sha256 != output_digest {
        return Err(QualificationError::OutputDigestDoesNotMatchFinalRefinement);
    }

    if convergence.declared_converged {
        if convergence.refinements.len() < policy.minimum_refinement_levels
            || final_level.maximum_map_change_t.is_none()
            || final_level.estimated_absolute_error_t.is_none()
        {
            return Err(QualificationError::MissingFinalConvergenceEvidence);
        }

        if final_level.maximum_map_change_t.unwrap() > policy.maximum_final_map_change_t
            || final_level.estimated_absolute_error_t.unwrap()
                > policy.maximum_final_error_estimate_t
        {
            return Err(QualificationError::DeclaredConvergedWithoutMeetingPolicy);
        }
    }

    Ok(())
}

/// Validate a solver run's provenance, model applicability and convergence evidence.
///
/// This checks declared metadata and arithmetic constraints only. It does not
/// authenticate the solver, validate the solver's own error estimator, or prove
/// that the chosen constitutive model represents the physical specimen.
pub fn validate_run(
    run: &SolverRunRecord,
    convergence_policy: ConvergencePolicy,
) -> Result<(), QualificationError> {
    let identity_strings = [
        run.run_id.as_str(),
        run.solver.software_id.as_str(),
        run.solver.version.as_str(),
        run.solver.source_revision.as_str(),
        run.solver.implementation_family.as_str(),
        run.solver.formulation_id.as_str(),
        run.problem.coordinate_frame_id.as_str(),
    ];
    if identity_strings.into_iter().any(|value| !nonempty(value))
        || run.solver.direct_dependency_ids.iter().any(|value| !nonempty(value))
    {
        return Err(QualificationError::EmptyIdentity);
    }

    validate_digest(&run.solver.source_archive_sha256, "source_archive_sha256")?;
    validate_digest(&run.environment_sha256, "environment_sha256")?;
    validate_digest(&run.output_map_sha256, "output_map_sha256")?;
    validate_digest(&run.problem.geometry_sha256, "geometry_sha256")?;
    validate_digest(&run.problem.material_sha256, "material_sha256")?;
    validate_digest(&run.problem.physics_input_sha256, "physics_input_sha256")?;
    validate_digest(
        &run.problem.observer_grid_sha256,
        "observer_grid_sha256",
    )?;

    if !method_supports_model(run.solver.method, run.problem.model_class) {
        return Err(QualificationError::IncompatibleMethodAndModel);
    }

    if run.solver.method != SolverMethod::PhysicalMetrology
        && run.precision == FloatingPointPrecision::Unknown
    {
        return Err(QualificationError::UnknownNumericalPrecision);
    }

    if method_requires_convergence(run.solver.method) {
        let convergence = run
            .convergence
            .as_ref()
            .ok_or(QualificationError::MissingConvergenceRecord)?;
        validate_convergence(convergence, &run.output_map_sha256, convergence_policy)?;
    } else if run.convergence.is_some() {
        return Err(QualificationError::UnexpectedConvergenceRecord);
    }

    Ok(())
}

fn lineage_relation(left: &SolverIdentity, right: &SolverIdentity) -> LineageRelation {
    if left.method == SolverMethod::PhysicalMetrology
        || right.method == SolverMethod::PhysicalMetrology
    {
        return LineageRelation::PhysicalMeasurementVersusPrediction;
    }

    if left.software_id == right.software_id
        && left.implementation_family == right.implementation_family
        && left.formulation_id == right.formulation_id
    {
        LineageRelation::SameDeclaredImplementationAndFormulation
    } else if left.software_id == right.software_id {
        LineageRelation::DifferentFormulationSameSoftware
    } else if left.formulation_id == right.formulation_id {
        LineageRelation::DifferentSoftwareSameDeclaredFormulation
    } else {
        LineageRelation::DifferentSoftwareAndFormulation
    }
}

/// Establish whether two runs are directly comparable.
///
/// The physical problem and ordered observer grid must be identical. Solver
/// configuration, method, precision and environment are intentionally allowed
/// to differ and remain visible in each source record.
pub fn compare_runs(
    left: &SolverRunRecord,
    right: &SolverRunRecord,
    policy: ConvergencePolicy,
) -> Result<ComparablePair, QualificationError> {
    validate_run(left, policy)?;
    validate_run(right, policy)?;

    if left.problem.model_class != right.problem.model_class {
        return Err(QualificationError::ProblemModelMismatch);
    }
    if left.problem.geometry_sha256 != right.problem.geometry_sha256 {
        return Err(QualificationError::GeometryMismatch);
    }
    if left.problem.material_sha256 != right.problem.material_sha256 {
        return Err(QualificationError::MaterialMismatch);
    }
    if left.problem.physics_input_sha256 != right.problem.physics_input_sha256 {
        return Err(QualificationError::PhysicsInputMismatch);
    }
    if left.problem.coordinate_frame_id != right.problem.coordinate_frame_id {
        return Err(QualificationError::CoordinateFrameMismatch);
    }
    if left.problem.observer_grid_sha256 != right.problem.observer_grid_sha256 {
        return Err(QualificationError::ObserverGridMismatch);
    }

    let mut shared_dependency_ids = left
        .solver
        .direct_dependency_ids
        .iter()
        .filter(|dependency| {
            right
                .solver
                .direct_dependency_ids
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(dependency))
        })
        .map(|dependency| dependency.to_ascii_lowercase())
        .collect::<Vec<_>>();
    shared_dependency_ids.sort();
    shared_dependency_ids.dedup();

    Ok(ComparablePair {
        left_run_id: left.run_id.clone(),
        right_run_id: right.run_id.clone(),
        lineage_relation: lineage_relation(&left.solver, &right.solver),
        physical_inputs_and_observer_grid_match: true,
        shared_dependency_ids,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIGEST_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const DIGEST_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const DIGEST_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const DIGEST_D: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";

    fn policy() -> ConvergencePolicy {
        ConvergencePolicy {
            policy_id: "encoder-field-map-v0".to_owned(),
            minimum_refinement_levels: 3,
            maximum_final_map_change_t: 1.0e-5,
            maximum_final_error_estimate_t: 2.0e-5,
        }
    }

    fn convergence(output_digest: &str, final_change: f64, final_error: f64) -> ConvergenceRecord {
        ConvergenceRecord {
            policy_id: "encoder-field-map-v0".to_owned(),
            declared_converged: true,
            refinements: vec![
                RefinementObservation {
                    refinement_index: 1,
                    characteristic_resolution_m: 1.0e-3,
                    maximum_map_change_t: None,
                    estimated_absolute_error_t: Some(1.0e-2),
                    residual_norm: Some(1.0e-4),
                    output_map_sha256: DIGEST_A.to_owned(),
                },
                RefinementObservation {
                    refinement_index: 2,
                    characteristic_resolution_m: 5.0e-4,
                    maximum_map_change_t: Some(1.0e-3),
                    estimated_absolute_error_t: Some(5.0e-4),
                    residual_norm: Some(1.0e-6),
                    output_map_sha256: DIGEST_B.to_owned(),
                },
                RefinementObservation {
                    refinement_index: 3,
                    characteristic_resolution_m: 2.5e-4,
                    maximum_map_change_t: Some(final_change),
                    estimated_absolute_error_t: Some(final_error),
                    residual_norm: Some(1.0e-8),
                    output_map_sha256: output_digest.to_owned(),
                },
            ],
        }
    }

    fn face_charge_run() -> SolverRunRecord {
        SolverRunRecord {
            run_id: "face-charge-run-01".to_owned(),
            solver: SolverIdentity {
                software_id: "magpylib-material-response".to_owned(),
                version: "0.14.0".to_owned(),
                source_revision: "v0.14.0".to_owned(),
                source_archive_sha256: DIGEST_A.to_owned(),
                implementation_family: "magpylib-material-response".to_owned(),
                direct_dependency_ids: vec!["magpylib".to_owned(), "numpy".to_owned()],
                formulation_id: "surface-face-charge".to_owned(),
                method: SolverMethod::SurfaceFaceCharge,
            },
            problem: MagneticProblemIdentity {
                model_class: MagneticModelClass::MagnetWithLinearIsotropicSoftMaterial,
                geometry_sha256: DIGEST_A.to_owned(),
                material_sha256: DIGEST_B.to_owned(),
                physics_input_sha256: DIGEST_C.to_owned(),
                coordinate_frame_id: "assembly-frame-v1".to_owned(),
                observer_grid_sha256: DIGEST_A.to_owned(),
            },
            environment_sha256: DIGEST_B.to_owned(),
            output_map_sha256: DIGEST_C.to_owned(),
            precision: FloatingPointPrecision::F64,
            runtime_nanoseconds: 7_000_000,
            peak_memory_bytes: Some(256 * 1024 * 1024),
            unknown_count: Some(2048),
            convergence: Some(convergence(DIGEST_C, 4.0e-6, 8.0e-6)),
        }
    }

    fn fem_run() -> SolverRunRecord {
        let mut run = face_charge_run();
        run.run_id = "fem-run-01".to_owned();
        run.solver.software_id = "magpylib-material-response".to_owned();
        run.solver.formulation_id = "finite-element".to_owned();
        run.solver.method = SolverMethod::FiniteElement;
        run.output_map_sha256 = DIGEST_D.to_owned();
        run.convergence = Some(convergence(DIGEST_D, 3.0e-6, 6.0e-6));
        run
    }

    #[test]
    fn accepts_a_matched_face_charge_run_with_convergence() {
        assert_eq!(validate_run(&face_charge_run(), policy()), Ok(()));
    }

    #[test]
    fn reports_different_formulations_inside_same_software() {
        let pair = compare_runs(&face_charge_run(), &fem_run(), policy()).unwrap();
        assert_eq!(
            pair.lineage_relation,
            LineageRelation::DifferentFormulationSameSoftware
        );
        assert!(pair.physical_inputs_and_observer_grid_match);
        assert_eq!(pair.shared_dependency_ids, vec!["magpylib", "numpy"]);
    }


    #[test]
    fn shared_dependencies_are_reported_across_distinct_software_ids() {
        let left = face_charge_run();
        let mut right = face_charge_run();
        right.run_id = "other-adapter-run".to_owned();
        right.solver.software_id = "independent-adapter".to_owned();
        right.solver.direct_dependency_ids = vec!["NumPy".to_owned(), "magpylib".to_owned()];
        let pair = compare_runs(&left, &right, policy()).unwrap();

        assert_eq!(
            pair.lineage_relation,
            LineageRelation::DifferentSoftwareSameDeclaredFormulation
        );
        assert_eq!(pair.shared_dependency_ids, vec!["magpylib", "numpy"]);
    }

    #[test]
    fn refuses_a_numerical_run_without_convergence_evidence() {
        let mut run = face_charge_run();
        run.convergence = None;
        assert_eq!(
            validate_run(&run, policy()),
            Err(QualificationError::MissingConvergenceRecord)
        );
    }

    #[test]
    fn refuses_false_converged_claim_above_declared_error_policy() {
        let mut run = face_charge_run();
        run.convergence = Some(convergence(DIGEST_C, 5.0e-5, 8.0e-5));
        assert_eq!(
            validate_run(&run, policy()),
            Err(QualificationError::DeclaredConvergedWithoutMeetingPolicy)
        );
    }

    #[test]
    fn refuses_non_monotonic_refinement() {
        let mut run = face_charge_run();
        let mut record = run.convergence.take().unwrap();
        record.refinements[2].characteristic_resolution_m = 6.0e-4;
        run.convergence = Some(record);
        assert_eq!(
            validate_run(&run, policy()),
            Err(QualificationError::InvalidRefinementOrder { index: 2 })
        );
    }

    #[test]
    fn final_run_digest_must_match_final_refinement_digest() {
        let mut run = face_charge_run();
        run.output_map_sha256 = DIGEST_A.to_owned();
        assert_eq!(
            validate_run(&run, policy()),
            Err(QualificationError::OutputDigestDoesNotMatchFinalRefinement)
        );
    }

    #[test]
    fn refuses_model_class_outside_solver_method_scope() {
        let mut run = face_charge_run();
        run.problem.model_class = MagneticModelClass::MagnetWithNonlinearBhMaterial;
        assert_eq!(
            validate_run(&run, policy()),
            Err(QualificationError::IncompatibleMethodAndModel)
        );
    }

    #[test]
    fn rejects_comparison_when_observer_grid_differs() {
        let mut other = fem_run();
        other.problem.observer_grid_sha256 = DIGEST_C.to_owned();
        assert_eq!(
            compare_runs(&face_charge_run(), &other, policy()),
            Err(QualificationError::ObserverGridMismatch)
        );
    }

    #[test]
    fn rejects_comparison_when_physical_geometry_differs() {
        let mut other = fem_run();
        other.problem.geometry_sha256 = DIGEST_C.to_owned();
        assert_eq!(
            compare_runs(&face_charge_run(), &other, policy()),
            Err(QualificationError::GeometryMismatch)
        );
    }


    #[test]
    fn rejects_comparison_when_material_identity_differs() {
        let mut other = fem_run();
        other.problem.material_sha256 = DIGEST_D.to_owned();
        assert_eq!(
            compare_runs(&face_charge_run(), &other, policy()),
            Err(QualificationError::MaterialMismatch)
        );
    }

    #[test]
    fn rejects_comparison_when_physics_inputs_differ() {
        let mut other = fem_run();
        other.problem.physics_input_sha256 = DIGEST_D.to_owned();
        assert_eq!(
            compare_runs(&face_charge_run(), &other, policy()),
            Err(QualificationError::PhysicsInputMismatch)
        );
    }

    #[test]
    fn rejects_comparison_when_coordinate_frame_differs() {
        let mut other = fem_run();
        other.problem.coordinate_frame_id = "other-frame".to_owned();
        assert_eq!(
            compare_runs(&face_charge_run(), &other, policy()),
            Err(QualificationError::CoordinateFrameMismatch)
        );
    }

    #[test]
    fn distinguishes_physical_measurement_from_prediction() {
        let mut measurement = face_charge_run();
        measurement.run_id = "physical-map-01".to_owned();
        measurement.solver.method = SolverMethod::PhysicalMetrology;
        measurement.solver.formulation_id = "calibrated-3d-hall-map".to_owned();
        measurement.problem.model_class = MagneticModelClass::MagnetWithLinearIsotropicSoftMaterial;
        measurement.convergence = None;
        let prediction = face_charge_run();

        let pair = compare_runs(&measurement, &prediction, policy()).unwrap();
        assert_eq!(pair.lineage_relation, LineageRelation::PhysicalMeasurementVersusPrediction);
        assert!(pair.physical_inputs_and_observer_grid_match);
        assert_eq!(
            validate_run(&measurement, policy()),
            Ok(())
        );
    }


    #[test]
    fn refuses_unknown_numeric_precision_for_a_numerical_solver() {
        let mut run = face_charge_run();
        run.precision = FloatingPointPrecision::Unknown;
        assert_eq!(
            validate_run(&run, policy()),
            Err(QualificationError::UnknownNumericalPrecision)
        );
    }

    #[test]
    fn convergence_policy_identity_must_match() {
        let mut policy = policy();
        policy.policy_id = "different-policy".to_owned();
        assert_eq!(
            validate_run(&face_charge_run(), policy),
            Err(QualificationError::InvalidConvergencePolicy)
        );
    }

    #[test]
    fn validates_sha256_shape_without_calling_it_authentication() {
        let mut run = face_charge_run();
        run.environment_sha256 = "not-a-digest".to_owned();
        assert_eq!(
            validate_run(&run, policy()),
            Err(QualificationError::InvalidSha256 {
                field: "environment_sha256"
            })
        );
    }
}
