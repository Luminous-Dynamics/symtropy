//! Evidence-gated comparison of as-built magnetic manufacturing routes.
//!
//! This crate is a semantic and decision reference. It does not solve fields,
//! propagate uncertainty, authenticate evidence producers, establish
//! calibration competence, or authorize physical manufacturing or release.

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RouteFamily {
    Mex,
    LpbfSlm,
    Sls,
    BinderJet,
    LcmPhotopolymer,
    PemAdjacent,
    ConventionalBonded,
    SinteredOrAssembled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceRole {
    PhysicalObservation,
    MaterialCharacterization,
    AsBuiltInspection,
    ProcessRecord,
    PublishedSummary,
    Synthetic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObjectiveDirection {
    Minimize,
    Maximize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CohortScope {
    pub application_id: String,
    pub target_function_digest: String,
    pub measurement_setup_digest: String,
    pub acceptance_policy_digest: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MetricSpec {
    pub metric_id: String,
    pub unit: String,
    pub required_role: EvidenceRole,
    pub objective: Option<ObjectiveDirection>,
    pub hard_min: Option<f64>,
    pub hard_max: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ComparisonCohort {
    pub scope: CohortScope,
    pub metrics: Vec<MetricSpec>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MetricObservation {
    pub record_id: String,
    pub record_digest: String,
    pub metric_id: String,
    pub specimen_id: String,
    pub material_lot_id: String,
    pub build_run_id: String,
    pub as_built_geometry_digest: String,
    pub geometry_inspection_digest: String,
    pub process_genealogy_digest: String,
    pub material_state_digest: String,
    pub post_magnetization_digest: String,
    pub scope: CohortScope,
    pub value: f64,
    pub uncertainty_radius: f64,
    pub unit: String,
    pub role: EvidenceRole,
    pub method_id: String,
    pub calibration_digest: Option<String>,
    pub reference_chain_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RouteCandidate {
    pub candidate_id: String,
    pub route_family: RouteFamily,
    pub specimen_id: String,
    pub material_lot_id: String,
    pub build_run_id: String,
    pub nominal_geometry_digest: String,
    pub as_built_geometry_digest: String,
    pub geometry_inspection_record_id: String,
    pub geometry_inspection_digest: String,
    pub geometry_inspection_method_id: String,
    pub geometry_inspection_calibration_digest: String,
    pub geometry_inspection_reference_chain_id: String,
    pub inspected_geometry_digest: String,
    pub process_genealogy_digest: String,
    pub material_state_digest: String,
    pub post_magnetization_digest: String,
    pub scope: CohortScope,
    pub method_scope_supported: bool,
    pub observations: Vec<MetricObservation>,
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
    UnsupportedMethodScope,
    MissingIdentity,
    InvalidDigest,
    InvalidCohort,
    DuplicateMetricSpec,
    InvalidHardBounds,
    NoParetoObjectives,
    DuplicateObservation,
    MissingObservation,
    UnknownMetric,
    WrongSpecimen,
    WrongMaterialLot,
    WrongBuildRun,
    WrongAsBuiltGeometry,
    GeometryInspectionBindingMismatch,
    ProcessGenealogyBindingMismatch,
    MaterialStateBindingMismatch,
    PostMagnetizationBindingMismatch,
    InspectionGeometryMismatch,
    MissingGeometryInspectionMethod,
    MissingGeometryInspectionCalibration,
    MissingGeometryInspectionReference,
    DuplicateCandidateId,
    DuplicateSpecimen,
    WrongCohortScope,
    WrongUnit,
    WrongEvidenceRole,
    InadmissibleEvidenceRole,
    MissingMethod,
    MissingCalibration,
    MissingReferenceChain,
    NonFiniteValue,
    NegativeUncertainty,
    DefiniteHardLimitViolation,
    UnresolvedHardLimit,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Interval {
    pub lower: f64,
    pub upper: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MetricInterval {
    pub metric_id: String,
    pub direction: ObjectiveDirection,
    pub interval: Interval,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CandidateEvaluation {
    pub candidate_id: String,
    pub route_family: RouteFamily,
    pub disposition: Disposition,
    pub blockers: Vec<Blocker>,
    pub objective_intervals: Vec<MetricInterval>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IncomparablePair {
    pub left_candidate_id: String,
    pub right_candidate_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TournamentResult {
    pub evaluations: Vec<CandidateEvaluation>,
    pub nondominated_candidate_ids: Vec<String>,
    pub dominated_candidate_ids: Vec<String>,
    pub incomparable_pairs: Vec<IncomparablePair>,
    pub cohort_blockers: Vec<Blocker>,
}

/// Evaluate one route candidate against an exact, frozen comparison cohort.
///
/// Only PhysicalObservation, MaterialCharacterization, AsBuiltInspection and
/// ProcessRecord observations are admissible. PublishedSummary and Synthetic
/// evidence remain useful for source review and testing, but cannot qualify a
/// newly manufactured specimen.
pub fn evaluate_candidate(
    cohort: &ComparisonCohort,
    candidate: &RouteCandidate,
) -> CandidateEvaluation {
    let mut blockers = validate_cohort(cohort);

    if !candidate.method_scope_supported {
        return CandidateEvaluation {
            candidate_id: candidate.candidate_id.clone(),
            route_family: candidate.route_family,
            disposition: Disposition::NotEvaluable,
            blockers: vec![Blocker::UnsupportedMethodScope],
            objective_intervals: Vec::new(),
        };
    }

    if candidate.candidate_id.trim().is_empty()
        || candidate.specimen_id.trim().is_empty()
        || candidate.material_lot_id.trim().is_empty()
        || candidate.build_run_id.trim().is_empty()
    {
        blockers.push(Blocker::MissingIdentity);
    }

    for digest in [
        &candidate.nominal_geometry_digest,
        &candidate.as_built_geometry_digest,
        &candidate.geometry_inspection_digest,
        &candidate.inspected_geometry_digest,
        &candidate.process_genealogy_digest,
        &candidate.material_state_digest,
        &candidate.post_magnetization_digest,
        &candidate.scope.target_function_digest,
        &candidate.scope.measurement_setup_digest,
        &candidate.scope.acceptance_policy_digest,
    ] {
        if !valid_sha256(digest) {
            blockers.push(Blocker::InvalidDigest);
        }
    }

    if candidate.geometry_inspection_record_id.trim().is_empty()
        || candidate.geometry_inspection_method_id.trim().is_empty()
    {
        blockers.push(Blocker::MissingGeometryInspectionMethod);
    }
    if !valid_sha256(&candidate.geometry_inspection_calibration_digest) {
        blockers.push(Blocker::MissingGeometryInspectionCalibration);
    }
    if candidate
        .geometry_inspection_reference_chain_id
        .trim()
        .is_empty()
    {
        blockers.push(Blocker::MissingGeometryInspectionReference);
    }
    if candidate.inspected_geometry_digest != candidate.as_built_geometry_digest {
        blockers.push(Blocker::InspectionGeometryMismatch);
    }
    if candidate.scope != cohort.scope {
        blockers.push(Blocker::WrongCohortScope);
    }

    let mut observation_by_metric = std::collections::BTreeMap::new();
    for observation in &candidate.observations {
        if observation_by_metric
            .insert(observation.metric_id.as_str(), observation)
            .is_some()
        {
            blockers.push(Blocker::DuplicateObservation);
        }
    }

    for observation in &candidate.observations {
        if observation.record_id.trim().is_empty()
            || observation.method_id.trim().is_empty()
            || observation.specimen_id.trim().is_empty()
        {
            blockers.push(Blocker::MissingIdentity);
        }
        if !valid_sha256(&observation.record_digest) {
            blockers.push(Blocker::InvalidDigest);
        }
        if observation.specimen_id != candidate.specimen_id {
            blockers.push(Blocker::WrongSpecimen);
        }
        if observation.material_lot_id != candidate.material_lot_id {
            blockers.push(Blocker::WrongMaterialLot);
        }
        if observation.build_run_id != candidate.build_run_id {
            blockers.push(Blocker::WrongBuildRun);
        }
        if observation.as_built_geometry_digest != candidate.as_built_geometry_digest {
            blockers.push(Blocker::WrongAsBuiltGeometry);
        }
        if observation.geometry_inspection_digest != candidate.geometry_inspection_digest {
            blockers.push(Blocker::GeometryInspectionBindingMismatch);
        }
        if observation.process_genealogy_digest != candidate.process_genealogy_digest {
            blockers.push(Blocker::ProcessGenealogyBindingMismatch);
        }
        if observation.material_state_digest != candidate.material_state_digest {
            blockers.push(Blocker::MaterialStateBindingMismatch);
        }
        if observation.post_magnetization_digest != candidate.post_magnetization_digest {
            blockers.push(Blocker::PostMagnetizationBindingMismatch);
        }
        if observation.scope != cohort.scope {
            blockers.push(Blocker::WrongCohortScope);
        }
        if !admissible_role(observation.role) {
            blockers.push(Blocker::InadmissibleEvidenceRole);
        }
        if observation.method_id.trim().is_empty() {
            blockers.push(Blocker::MissingMethod);
        }
        if requires_calibration(observation.role)
            && observation
                .calibration_digest
                .as_ref()
                .is_none_or(|digest| !valid_sha256(digest))
        {
            blockers.push(Blocker::MissingCalibration);
        }
        if requires_calibration(observation.role)
            && observation
                .reference_chain_id
                .as_ref()
                .is_none_or(|reference| reference.trim().is_empty())
        {
            blockers.push(Blocker::MissingReferenceChain);
        }
        if !observation.value.is_finite() || !observation.uncertainty_radius.is_finite() {
            blockers.push(Blocker::NonFiniteValue);
        }
        if observation.uncertainty_radius < 0.0 {
            blockers.push(Blocker::NegativeUncertainty);
        }
    }

    let known_metric_ids = cohort
        .metrics
        .iter()
        .map(|metric| metric.metric_id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    for observation in &candidate.observations {
        if !known_metric_ids.contains(observation.metric_id.as_str()) {
            blockers.push(Blocker::UnknownMetric);
        }
    }

    let mut objective_intervals = Vec::new();
    let mut definite_failure = false;
    let mut unresolved_hard_limit = false;

    for metric in &cohort.metrics {
        let Some(observation) = observation_by_metric.get(metric.metric_id.as_str()) else {
            blockers.push(Blocker::MissingObservation);
            continue;
        };

        if observation.unit != metric.unit {
            blockers.push(Blocker::WrongUnit);
            continue;
        }
        if observation.role != metric.required_role {
            blockers.push(Blocker::WrongEvidenceRole);
            continue;
        }
        if !observation.value.is_finite()
            || !observation.uncertainty_radius.is_finite()
            || observation.uncertainty_radius < 0.0
        {
            continue;
        }

        let interval = Interval {
            lower: observation.value - observation.uncertainty_radius,
            upper: observation.value + observation.uncertainty_radius,
        };
        if !interval.lower.is_finite() || !interval.upper.is_finite() {
            blockers.push(Blocker::NonFiniteValue);
            continue;
        }

        if let Some(minimum) = metric.hard_min {
            if interval.upper < minimum {
                definite_failure = true;
            } else if interval.lower < minimum {
                unresolved_hard_limit = true;
            }
        }
        if let Some(maximum) = metric.hard_max {
            if interval.lower > maximum {
                definite_failure = true;
            } else if interval.upper > maximum {
                unresolved_hard_limit = true;
            }
        }

        if let Some(direction) = metric.objective {
            objective_intervals.push(MetricInterval {
                metric_id: metric.metric_id.clone(),
                direction,
                interval,
            });
        }
    }

    if !blockers.is_empty() {
        return CandidateEvaluation {
            candidate_id: candidate.candidate_id.clone(),
            route_family: candidate.route_family,
            disposition: Disposition::Indeterminate,
            blockers,
            objective_intervals,
        };
    }

    if definite_failure {
        blockers.push(Blocker::DefiniteHardLimitViolation);
        return CandidateEvaluation {
            candidate_id: candidate.candidate_id.clone(),
            route_family: candidate.route_family,
            disposition: Disposition::Fail,
            blockers,
            objective_intervals,
        };
    }

    if unresolved_hard_limit {
        blockers.push(Blocker::UnresolvedHardLimit);
        return CandidateEvaluation {
            candidate_id: candidate.candidate_id.clone(),
            route_family: candidate.route_family,
            disposition: Disposition::Indeterminate,
            blockers,
            objective_intervals,
        };
    }

    CandidateEvaluation {
        candidate_id: candidate.candidate_id.clone(),
        route_family: candidate.route_family,
        disposition: Disposition::Qualified,
        blockers,
        objective_intervals,
    }
}

/// Evaluate candidates and construct a robust Pareto set.
///
/// Robust dominance requires interval separation in every objective dimension.
/// Overlapping intervals remain incomparable. No scalar normalization,
/// weighting or route-family preference is introduced.
pub fn run_tournament(
    cohort: &ComparisonCohort,
    candidates: &[RouteCandidate],
) -> TournamentResult {
    let cohort_blockers = validate_cohort(cohort);
    let mut evaluations = candidates
        .iter()
        .map(|candidate| evaluate_candidate(cohort, candidate))
        .collect::<Vec<_>>();

    let mut id_counts = std::collections::BTreeMap::new();
    let mut specimen_counts = std::collections::BTreeMap::new();
    for candidate in candidates {
        *id_counts
            .entry(candidate.candidate_id.as_str())
            .or_insert(0usize) += 1;
        *specimen_counts
            .entry(candidate.specimen_id.as_str())
            .or_insert(0usize) += 1;
    }
    for (candidate, evaluation) in candidates.iter().zip(evaluations.iter_mut()) {
        if id_counts
            .get(candidate.candidate_id.as_str())
            .copied()
            .unwrap_or(0)
            > 1
        {
            evaluation.disposition = Disposition::Indeterminate;
            evaluation.blockers.push(Blocker::DuplicateCandidateId);
        }
        if specimen_counts
            .get(candidate.specimen_id.as_str())
            .copied()
            .unwrap_or(0)
            > 1
        {
            evaluation.disposition = Disposition::Indeterminate;
            evaluation.blockers.push(Blocker::DuplicateSpecimen);
        }
    }

    let qualified = evaluations
        .iter()
        .filter(|evaluation| evaluation.disposition == Disposition::Qualified)
        .collect::<Vec<_>>();

    let mut dominated = std::collections::BTreeSet::new();
    let mut incomparable_pairs = Vec::new();

    for (left_index, left) in qualified.iter().enumerate() {
        for right in qualified.iter().skip(left_index + 1) {
            let left_dominates = robustly_dominates(left, right);
            let right_dominates = robustly_dominates(right, left);
            match (left_dominates, right_dominates) {
                (true, false) => {
                    dominated.insert(right.candidate_id.clone());
                }
                (false, true) => {
                    dominated.insert(left.candidate_id.clone());
                }
                (false, false) => {
                    incomparable_pairs.push(IncomparablePair {
                        left_candidate_id: left.candidate_id.clone(),
                        right_candidate_id: right.candidate_id.clone(),
                    });
                }
                (true, true) => {
                    // Strict interval separation in at least one dimension
                    // makes mutual robust dominance impossible. Preserve an
                    // explicit non-ranking result if malformed input reaches it.
                    incomparable_pairs.push(IncomparablePair {
                        left_candidate_id: left.candidate_id.clone(),
                        right_candidate_id: right.candidate_id.clone(),
                    });
                }
            }
        }
    }

    let nondominated_candidate_ids = qualified
        .iter()
        .filter(|evaluation| !dominated.contains(&evaluation.candidate_id))
        .map(|evaluation| evaluation.candidate_id.clone())
        .collect();

    TournamentResult {
        evaluations,
        nondominated_candidate_ids,
        dominated_candidate_ids: dominated.into_iter().collect(),
        incomparable_pairs,
        cohort_blockers,
    }
}

fn validate_cohort(cohort: &ComparisonCohort) -> Vec<Blocker> {
    let mut blockers = Vec::new();
    if cohort.scope.application_id.trim().is_empty() || cohort.metrics.is_empty() {
        blockers.push(Blocker::InvalidCohort);
    }
    if !valid_sha256(&cohort.scope.target_function_digest)
        || !valid_sha256(&cohort.scope.measurement_setup_digest)
        || !valid_sha256(&cohort.scope.acceptance_policy_digest)
    {
        blockers.push(Blocker::InvalidDigest);
    }

    let mut ids = std::collections::BTreeSet::new();
    for metric in &cohort.metrics {
        if metric.metric_id.trim().is_empty() || metric.unit.trim().is_empty() {
            blockers.push(Blocker::InvalidCohort);
        }
        if !ids.insert(metric.metric_id.as_str()) {
            blockers.push(Blocker::DuplicateMetricSpec);
        }
        if metric.hard_min.is_some_and(|value| !value.is_finite())
            || metric.hard_max.is_some_and(|value| !value.is_finite())
        {
            blockers.push(Blocker::InvalidHardBounds);
        }
        if let (Some(minimum), Some(maximum)) = (metric.hard_min, metric.hard_max)
            && minimum > maximum
        {
            blockers.push(Blocker::InvalidHardBounds);
        }
    }
    if !cohort
        .metrics
        .iter()
        .any(|metric| metric.objective.is_some())
    {
        blockers.push(Blocker::NoParetoObjectives);
    }
    blockers
}

fn robustly_dominates(left: &CandidateEvaluation, right: &CandidateEvaluation) -> bool {
    if left.disposition != Disposition::Qualified
        || right.disposition != Disposition::Qualified
        || left.objective_intervals.len() != right.objective_intervals.len()
    {
        return false;
    }

    let right_by_id = right
        .objective_intervals
        .iter()
        .map(|metric| (metric.metric_id.as_str(), metric))
        .collect::<std::collections::BTreeMap<_, _>>();

    let mut strictly_better = false;
    for left_metric in &left.objective_intervals {
        let Some(right_metric) = right_by_id.get(left_metric.metric_id.as_str()) else {
            return false;
        };
        if left_metric.direction != right_metric.direction {
            return false;
        }
        match left_metric.direction {
            ObjectiveDirection::Minimize => {
                if left_metric.interval.upper > right_metric.interval.lower {
                    return false;
                }
                if left_metric.interval.upper < right_metric.interval.lower {
                    strictly_better = true;
                }
            }
            ObjectiveDirection::Maximize => {
                if left_metric.interval.lower < right_metric.interval.upper {
                    return false;
                }
                if left_metric.interval.lower > right_metric.interval.upper {
                    strictly_better = true;
                }
            }
        }
    }
    strictly_better
}

fn admissible_role(role: EvidenceRole) -> bool {
    matches!(
        role,
        EvidenceRole::PhysicalObservation
            | EvidenceRole::MaterialCharacterization
            | EvidenceRole::AsBuiltInspection
            | EvidenceRole::ProcessRecord
    )
}

fn requires_calibration(role: EvidenceRole) -> bool {
    matches!(
        role,
        EvidenceRole::PhysicalObservation
            | EvidenceRole::MaterialCharacterization
            | EvidenceRole::AsBuiltInspection
    )
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

    fn digest(ch: char) -> String {
        format!("sha256:{}", ch.to_string().repeat(64))
    }

    fn scope() -> CohortScope {
        CohortScope {
            application_id: "ENCODER-80MM-001".to_string(),
            target_function_digest: digest('a'),
            measurement_setup_digest: digest('b'),
            acceptance_policy_digest: digest('c'),
        }
    }

    fn cohort() -> ComparisonCohort {
        ComparisonCohort {
            scope: scope(),
            metrics: vec![
                MetricSpec {
                    metric_id: "field_error".to_string(),
                    unit: "mT".to_string(),
                    required_role: EvidenceRole::PhysicalObservation,
                    objective: Some(ObjectiveDirection::Minimize),
                    hard_min: None,
                    hard_max: Some(30.0),
                },
                MetricSpec {
                    metric_id: "remanence".to_string(),
                    unit: "T".to_string(),
                    required_role: EvidenceRole::MaterialCharacterization,
                    objective: Some(ObjectiveDirection::Maximize),
                    hard_min: Some(0.25),
                    hard_max: None,
                },
                MetricSpec {
                    metric_id: "accepted_part_cost".to_string(),
                    unit: "USD/part".to_string(),
                    required_role: EvidenceRole::ProcessRecord,
                    objective: Some(ObjectiveDirection::Minimize),
                    hard_min: None,
                    hard_max: None,
                },
            ],
        }
    }

    struct ObservationSeed<'a> {
        specimen_id: &'a str,
        material_lot_id: &'a str,
        build_run_id: &'a str,
        metric_id: &'a str,
        value: f64,
        uncertainty: f64,
        unit: &'a str,
        role: EvidenceRole,
        suffix: char,
    }

    fn observation(seed: ObservationSeed<'_>) -> MetricObservation {
        let physical = requires_calibration(seed.role);
        MetricObservation {
            record_id: format!("OBS-{}-{}", seed.specimen_id, seed.metric_id),
            record_digest: digest(seed.suffix),
            metric_id: seed.metric_id.to_string(),
            specimen_id: seed.specimen_id.to_string(),
            material_lot_id: seed.material_lot_id.to_string(),
            build_run_id: seed.build_run_id.to_string(),
            as_built_geometry_digest: digest('d'),
            geometry_inspection_digest: digest('2'),
            process_genealogy_digest: digest('3'),
            material_state_digest: digest('4'),
            post_magnetization_digest: digest('5'),
            scope: scope(),
            value: seed.value,
            uncertainty_radius: seed.uncertainty,
            unit: seed.unit.to_string(),
            role: seed.role,
            method_id: format!("METHOD-{}", seed.metric_id),
            calibration_digest: physical.then(|| digest('e')),
            reference_chain_id: physical.then(|| "REF-CHAIN-001".to_string()),
        }
    }

    fn candidate(
        id: &str,
        route_family: RouteFamily,
        field_error: f64,
        remanence: f64,
        cost: f64,
    ) -> RouteCandidate {
        let specimen_id = format!("SPEC-{id}");
        let material_lot_id = format!("LOT-{id}");
        let build_run_id = format!("BUILD-{id}");
        RouteCandidate {
            candidate_id: id.to_string(),
            route_family,
            specimen_id: specimen_id.clone(),
            material_lot_id: material_lot_id.clone(),
            build_run_id: build_run_id.clone(),
            nominal_geometry_digest: digest('1'),
            as_built_geometry_digest: digest('d'),
            geometry_inspection_record_id: format!("GEOM-{id}"),
            geometry_inspection_digest: digest('2'),
            geometry_inspection_method_id: "CMM-INSPECTION-v1".to_string(),
            geometry_inspection_calibration_digest: digest('e'),
            geometry_inspection_reference_chain_id: "CMM-REFERENCE-001".to_string(),
            inspected_geometry_digest: digest('d'),
            process_genealogy_digest: digest('3'),
            material_state_digest: digest('4'),
            post_magnetization_digest: digest('5'),
            scope: scope(),
            method_scope_supported: true,
            observations: vec![
                observation(ObservationSeed {
                    specimen_id: &specimen_id,
                    material_lot_id: &material_lot_id,
                    build_run_id: &build_run_id,
                    metric_id: "field_error",
                    value: field_error,
                    uncertainty: 0.5,
                    unit: "mT",
                    role: EvidenceRole::PhysicalObservation,
                    suffix: '6',
                }),
                observation(ObservationSeed {
                    specimen_id: &specimen_id,
                    material_lot_id: &material_lot_id,
                    build_run_id: &build_run_id,
                    metric_id: "remanence",
                    value: remanence,
                    uncertainty: 0.01,
                    unit: "T",
                    role: EvidenceRole::MaterialCharacterization,
                    suffix: '4',
                }),
                observation(ObservationSeed {
                    specimen_id: &specimen_id,
                    material_lot_id: &material_lot_id,
                    build_run_id: &build_run_id,
                    metric_id: "accepted_part_cost",
                    value: cost,
                    uncertainty: 0.1,
                    unit: "USD/part",
                    role: EvidenceRole::ProcessRecord,
                    suffix: '8',
                }),
            ],
        }
    }

    #[test]
    fn complete_as_built_candidate_qualifies() {
        let result = evaluate_candidate(
            &cohort(),
            &candidate("MEX-001", RouteFamily::Mex, 20.0, 0.34, 5.0),
        );
        assert_eq!(result.disposition, Disposition::Qualified);
        assert!(result.blockers.is_empty());
        assert_eq!(result.objective_intervals.len(), 3);
    }

    #[test]
    fn route_family_does_not_imply_preference() {
        let mex = candidate("MEX-001", RouteFamily::Mex, 20.0, 0.34, 5.0);
        let lpbf = candidate("LPBF-001", RouteFamily::LpbfSlm, 18.0, 0.32, 7.0);
        let result = run_tournament(&cohort(), &[mex, lpbf]);
        assert_eq!(result.nondominated_candidate_ids.len(), 2);
        assert_eq!(result.incomparable_pairs.len(), 1);
    }

    #[test]
    fn wrong_cohort_scope_is_not_comparable() {
        let mut candidate = candidate("MEX-001", RouteFamily::Mex, 20.0, 0.34, 5.0);
        candidate.scope.measurement_setup_digest = digest('f');
        let result = evaluate_candidate(&cohort(), &candidate);
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(result.blockers.contains(&Blocker::WrongCohortScope));
    }

    #[test]
    fn wrong_unit_is_rejected() {
        let mut candidate = candidate("MEX-001", RouteFamily::Mex, 20.0, 0.34, 5.0);
        candidate.observations[0].unit = "T".to_string();
        let result = evaluate_candidate(&cohort(), &candidate);
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(result.blockers.contains(&Blocker::WrongUnit));
    }

    #[test]
    fn published_summary_cannot_qualify_a_physical_observation() {
        let mut candidate = candidate("MEX-001", RouteFamily::Mex, 20.0, 0.34, 5.0);
        candidate.observations[0].role = EvidenceRole::PublishedSummary;
        let result = evaluate_candidate(&cohort(), &candidate);
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(result.blockers.contains(&Blocker::InadmissibleEvidenceRole));
    }

    #[test]
    fn synthetic_observation_cannot_qualify_a_specimen() {
        let mut candidate = candidate("MEX-001", RouteFamily::Mex, 20.0, 0.34, 5.0);
        candidate.observations[0].role = EvidenceRole::Synthetic;
        let result = evaluate_candidate(&cohort(), &candidate);
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(result.blockers.contains(&Blocker::InadmissibleEvidenceRole));
    }

    #[test]
    fn nominal_geometry_cannot_replace_inspected_as_built_geometry() {
        let mut candidate = candidate("MEX-001", RouteFamily::Mex, 20.0, 0.34, 5.0);
        candidate.as_built_geometry_digest = candidate.nominal_geometry_digest.clone();
        let result = evaluate_candidate(&cohort(), &candidate);
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(
            result
                .blockers
                .contains(&Blocker::InspectionGeometryMismatch)
        );
    }

    #[test]
    fn missing_required_metric_is_indeterminate() {
        let mut candidate = candidate("MEX-001", RouteFamily::Mex, 20.0, 0.34, 5.0);
        candidate
            .observations
            .retain(|item| item.metric_id != "remanence");
        let result = evaluate_candidate(&cohort(), &candidate);
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(result.blockers.contains(&Blocker::MissingObservation));
    }

    #[test]
    fn definite_hard_limit_violation_is_fail() {
        let result = evaluate_candidate(
            &cohort(),
            &candidate("MEX-001", RouteFamily::Mex, 40.0, 0.34, 5.0),
        );
        assert_eq!(result.disposition, Disposition::Fail);
        assert!(
            result
                .blockers
                .contains(&Blocker::DefiniteHardLimitViolation)
        );
    }

    #[test]
    fn uncertainty_overlapping_hard_limit_is_indeterminate() {
        let result = evaluate_candidate(
            &cohort(),
            &candidate("MEX-001", RouteFamily::Mex, 29.8, 0.34, 5.0),
        );
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(result.blockers.contains(&Blocker::UnresolvedHardLimit));
    }

    #[test]
    fn robustly_separated_candidate_dominates() {
        let worse = candidate("MEX-001", RouteFamily::Mex, 25.0, 0.30, 8.0);
        let better = candidate("LCM-001", RouteFamily::LcmPhotopolymer, 15.0, 0.40, 4.0);
        let result = run_tournament(&cohort(), &[worse, better]);
        assert_eq!(result.nondominated_candidate_ids, vec!["LCM-001"]);
        assert_eq!(result.dominated_candidate_ids, vec!["MEX-001"]);
    }

    #[test]
    fn overlapping_intervals_remain_incomparable() {
        let first = candidate("MEX-001", RouteFamily::Mex, 20.0, 0.34, 5.0);
        let second = candidate("SLS-001", RouteFamily::Sls, 20.0, 0.34, 5.0);
        let result = run_tournament(&cohort(), &[first, second]);
        assert_eq!(result.nondominated_candidate_ids.len(), 2);
        assert_eq!(result.incomparable_pairs.len(), 1);
    }

    #[test]
    fn widening_uncertainty_cannot_create_robust_dominance() {
        let first = candidate("MEX-001", RouteFamily::Mex, 20.0, 0.34, 5.0);
        let mut second = candidate("LCM-001", RouteFamily::LcmPhotopolymer, 15.0, 0.40, 4.0);
        let narrow = run_tournament(&cohort(), &[first.clone(), second.clone()]);
        second.observations[0].uncertainty_radius = 20.0;
        let wide = run_tournament(&cohort(), &[first, second]);
        assert!(
            narrow
                .dominated_candidate_ids
                .contains(&"MEX-001".to_string())
        );
        assert!(
            !wide
                .dominated_candidate_ids
                .contains(&"MEX-001".to_string())
        );
    }

    #[test]
    fn geometry_inspection_requires_method_and_reference_evidence() {
        let mut candidate = candidate("MEX-001", RouteFamily::Mex, 20.0, 0.34, 5.0);
        candidate.geometry_inspection_calibration_digest.clear();
        candidate.geometry_inspection_reference_chain_id.clear();
        let result = evaluate_candidate(&cohort(), &candidate);
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(
            result
                .blockers
                .contains(&Blocker::MissingGeometryInspectionCalibration)
        );
        assert!(
            result
                .blockers
                .contains(&Blocker::MissingGeometryInspectionReference)
        );
    }

    #[test]
    fn wrong_specimen_join_is_rejected() {
        let mut candidate = candidate("MEX-001", RouteFamily::Mex, 20.0, 0.34, 5.0);
        candidate.observations[0].specimen_id = "SPEC-OTHER".to_string();
        let result = evaluate_candidate(&cohort(), &candidate);
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(result.blockers.contains(&Blocker::WrongSpecimen));
    }

    #[test]
    fn wrong_material_lot_and_build_are_rejected() {
        let mut candidate = candidate("MEX-001", RouteFamily::Mex, 20.0, 0.34, 5.0);
        candidate.observations[1].material_lot_id = "LOT-OTHER".to_string();
        candidate.observations[2].build_run_id = "BUILD-OTHER".to_string();
        let result = evaluate_candidate(&cohort(), &candidate);
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(result.blockers.contains(&Blocker::WrongMaterialLot));
        assert!(result.blockers.contains(&Blocker::WrongBuildRun));
    }

    #[test]
    fn process_genealogy_and_post_magnetization_must_bind_every_observation() {
        let mut candidate = candidate("MEX-001", RouteFamily::Mex, 20.0, 0.34, 5.0);
        candidate.observations[0].process_genealogy_digest = digest('f');
        candidate.observations[1].post_magnetization_digest = digest('f');
        let result = evaluate_candidate(&cohort(), &candidate);
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(
            result
                .blockers
                .contains(&Blocker::ProcessGenealogyBindingMismatch)
        );
        assert!(
            result
                .blockers
                .contains(&Blocker::PostMagnetizationBindingMismatch)
        );
    }

    #[test]
    fn duplicate_specimen_cannot_be_counted_as_two_route_candidates() {
        let first = candidate("MEX-001", RouteFamily::Mex, 20.0, 0.34, 5.0);
        let mut second = candidate("LPBF-001", RouteFamily::LpbfSlm, 18.0, 0.32, 7.0);
        second.specimen_id = first.specimen_id.clone();
        let result = run_tournament(&cohort(), &[first, second]);
        assert!(
            result
                .evaluations
                .iter()
                .all(|evaluation| evaluation.disposition == Disposition::Indeterminate)
        );
        assert!(
            result
                .evaluations
                .iter()
                .any(|evaluation| evaluation.blockers.contains(&Blocker::DuplicateSpecimen))
        );
    }

    #[test]
    fn duplicate_candidate_ids_are_rejected() {
        let first = candidate("SAME-ID", RouteFamily::Mex, 20.0, 0.34, 5.0);
        let second = candidate("SAME-ID", RouteFamily::LpbfSlm, 18.0, 0.32, 7.0);
        let result = run_tournament(&cohort(), &[first, second]);
        assert!(
            result
                .evaluations
                .iter()
                .all(|evaluation| evaluation.blockers.contains(&Blocker::DuplicateCandidateId))
        );
    }

    #[test]
    fn material_metric_records_may_differ_while_binding_the_same_material_state() {
        let mut candidate = candidate("MEX-001", RouteFamily::Mex, 20.0, 0.34, 5.0);
        candidate.observations[1].record_digest = digest('f');
        let result = evaluate_candidate(&cohort(), &candidate);
        assert_eq!(result.disposition, Disposition::Qualified);
        assert!(!result
            .blockers
            .contains(&Blocker::MaterialStateBindingMismatch));
    }

    #[test]
    fn physical_observation_requires_a_reference_chain() {
        let mut candidate = candidate("MEX-001", RouteFamily::Mex, 20.0, 0.34, 5.0);
        candidate.observations[0].reference_chain_id = None;
        let result = evaluate_candidate(&cohort(), &candidate);
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(result.blockers.contains(&Blocker::MissingReferenceChain));
    }

    #[test]
    fn missing_physical_calibration_is_rejected() {
        let mut candidate = candidate("MEX-001", RouteFamily::Mex, 20.0, 0.34, 5.0);
        candidate.observations[0].calibration_digest = None;
        let result = evaluate_candidate(&cohort(), &candidate);
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(result.blockers.contains(&Blocker::MissingCalibration));
    }

    #[test]
    fn unsupported_measurement_scope_is_not_evaluable() {
        let mut candidate = candidate("MEX-001", RouteFamily::Mex, 20.0, 0.34, 5.0);
        candidate.method_scope_supported = false;
        let result = evaluate_candidate(&cohort(), &candidate);
        assert_eq!(result.disposition, Disposition::NotEvaluable);
        assert_eq!(result.blockers, vec![Blocker::UnsupportedMethodScope]);
    }

    #[test]
    fn invalid_hard_bounds_reject_the_cohort() {
        let mut cohort = cohort();
        cohort.metrics[0].hard_min = Some(40.0);
        cohort.metrics[0].hard_max = Some(20.0);
        let result = run_tournament(
            &cohort,
            &[candidate("MEX-001", RouteFamily::Mex, 20.0, 0.34, 5.0)],
        );
        assert!(result.cohort_blockers.contains(&Blocker::InvalidHardBounds));
        assert_eq!(
            result.evaluations[0].disposition,
            Disposition::Indeterminate
        );
    }
}
