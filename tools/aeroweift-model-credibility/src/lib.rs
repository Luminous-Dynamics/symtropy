//! Structural checks for evidence-scoped AeroWeft model-validation packages.
//!
//! This crate never runs a simulator or computes physical residuals. It checks whether
//! a separately produced validation package respects its declared model scope,
//! calibration/confirmatory separation, evidence modality, stratum and metric policy.
//! A positive result is admissibility for further review only, not physical validation.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeSet;

/// Exact model and operating-domain identity to which performance credit is scoped.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelScope {
    /// Exact model artifact digest; syntax is checked, bytes are not authenticated.
    pub model_artifact_sha256: String,
    /// Aircraft configuration digest (type, mass/CG, payload and propulsion).
    pub aircraft_configuration_sha256: String,
    /// Flight-control and guidance software/configuration digest.
    pub control_stack_sha256: String,
    /// Sensor/receiver/calibration/placement profile digest.
    pub sensor_stack_sha256: String,
    /// Frozen operating-domain profile digest.
    pub operating_domain_sha256: String,
    /// Solver/integration/numerical configuration digest.
    pub solver_configuration_sha256: String,
    /// Horizontal coordinate-frame identity.
    pub horizontal_frame_id: String,
    /// Vertical datum identity.
    pub vertical_datum_id: String,
    /// Time-scale identity; this crate does not convert clocks.
    pub time_scale_id: String,
    /// Maximum prediction horizon represented by the claim, in synthetic ticks.
    pub prediction_horizon_ticks: u64,
}

impl ModelScope {
    fn is_complete(&self) -> bool {
        is_sha256(&self.model_artifact_sha256)
            && is_sha256(&self.aircraft_configuration_sha256)
            && is_sha256(&self.control_stack_sha256)
            && is_sha256(&self.sensor_stack_sha256)
            && is_sha256(&self.operating_domain_sha256)
            && is_sha256(&self.solver_configuration_sha256)
            && !self.horizontal_frame_id.trim().is_empty()
            && !self.vertical_datum_id.trim().is_empty()
            && !self.time_scale_id.trim().is_empty()
            && self.prediction_horizon_ticks > 0
    }
}

/// Dataset role in the preregistered validation split.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DatasetSplit {
    /// Data used to fit or calibrate the model.
    Calibration,
    /// Data used for model development and non-confirmatory tuning.
    Development,
    /// Held-out data reserved for confirmatory validation.
    Confirmatory,
}

/// A frozen dataset manifest header; payload bytes are outside this crate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DatasetManifest {
    /// Digest of the exact dataset manifest.
    pub manifest_sha256: String,
    /// Declared split role.
    pub split: DatasetSplit,
    /// Operating-domain profile represented by this manifest.
    pub operating_domain_sha256: String,
    /// Exact aircraft configuration represented by the data.
    pub aircraft_configuration_sha256: String,
    /// Exact flight-control/guidance stack represented by the data.
    pub control_stack_sha256: String,
    /// Exact sensor/receiver stack represented by the data.
    pub sensor_stack_sha256: String,
    /// Horizontal frame used by the reference measurements.
    pub horizontal_frame_id: String,
    /// Vertical datum used by the reference measurements.
    pub vertical_datum_id: String,
    /// Time scale used by the reference measurements.
    pub time_scale_id: String,
    /// Independent reference measurement/calibration profile identity.
    pub reference_measurement_profile_sha256: String,
    /// Flight-level grouping identifiers used for leakage control.
    pub flight_group_ids: Vec<String>,
    /// Scenario/encounter-family grouping identifiers used for leakage control.
    pub scenario_group_ids: Vec<String>,
    /// Declared environmental/configuration strata present in this dataset.
    pub strata: Vec<String>,
}

/// Evidence modality supporting one metric result.
/// These modalities are not a universal maturity ranking.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum EvidenceModality {
    /// Synthetic known-truth scenario evidence.
    Synthetic,
    /// Independent analytic/reference-solution comparison.
    AnalyticReference,
    /// Software-in-the-loop evidence.
    SoftwareInLoop,
    /// Hardware-in-the-loop evidence.
    HardwareInLoop,
    /// Calibrated component bench evidence.
    ComponentBench,
    /// Controlled flight-test evidence.
    ControlledFlight,
    /// Shadow-mode field replay evidence.
    ShadowMode,
}

/// Metric reported by an independent validation evaluator.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum MetricKind {
    /// Position prediction error percentile.
    PositionError,
    /// Velocity prediction error percentile.
    VelocityError,
    /// Turn-rate prediction error percentile.
    TurnRateError,
    /// Energy prediction error percentile.
    EnergyError,
    /// Fraction of independent reference intervals contained by predicted intervals.
    PredictionIntervalCoverage,
    /// Fraction of independent reference trajectories contained by predicted occupancy.
    TrajectoryContainment,
    /// Recall on deliberately out-of-domain cases.
    OutOfDomainDetectionRecall,
}

/// Independent sampling unit for a metric; denominators must not be mixed.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum SampleUnit {
    /// Distinct flights.
    Flight,
    /// Distinct encounter events.
    Encounter,
    /// Distinct trajectory segments.
    TrajectorySegment,
    /// Distinct prediction intervals.
    PredictionInterval,
    /// Distinct environment/configuration strata.
    Stratum,
}

/// Comparator for a preregistered metric gate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Comparator {
    /// The measured value must not exceed the threshold.
    AtMost,
    /// The measured value must meet or exceed the threshold.
    AtLeast,
}

/// One metric result produced on the held-out confirmatory manifest.
#[derive(Clone, Debug, PartialEq)]
pub struct MetricEvidence {
    /// Metric being reported.
    pub metric: MetricKind,
    /// Independent sampling unit represented by the denominator.
    pub sample_unit: SampleUnit,
    /// Exact named stratum; pooled metrics must not conceal uncovered strata.
    pub stratum_id: String,
    /// Evidence modality used to produce this metric.
    pub modality: EvidenceModality,
    /// Exact confirmatory dataset manifest digest.
    pub confirmatory_manifest_sha256: String,
    /// Measured point statistic. Interpretation and units are fixed by the gate profile.
    pub value: f64,
    /// Conservative lower confidence bound under the declared interval method.
    pub lower_confidence_bound: f64,
    /// Conservative upper confidence bound under the declared interval method.
    pub upper_confidence_bound: f64,
    /// Confidence level used for the interval, e.g. 0.95.
    pub confidence_level: f64,
    /// Digest identifying the frozen interval-estimation method.
    pub confidence_method_sha256: String,
    /// Number of independent sampling units, not raw correlated rows.
    pub independent_unit_count: u64,
}

/// One metric threshold frozen before opening the confirmatory results.
#[derive(Clone, Debug, PartialEq)]
pub struct MetricGate {
    /// Metric being gated.
    pub metric: MetricKind,
    /// Independent sampling unit expected by this gate.
    pub sample_unit: SampleUnit,
    /// Exact environmental/configuration stratum.
    pub stratum_id: String,
    /// Evidence modality required for this claim.
    pub modality: EvidenceModality,
    /// Comparison rule.
    pub comparator: Comparator,
    /// Preregistered threshold.
    pub threshold: f64,
    /// Minimum independent units required for this metric/stratum.
    pub minimum_independent_units: u64,
    /// Required confidence level for the conservative interval.
    pub confidence_level: f64,
    /// Frozen interval-estimation method digest.
    pub confidence_method_sha256: String,
}

/// Frozen evaluation policy independent of the submitted package.
#[derive(Clone, Debug, PartialEq)]
pub struct ValidationPolicy {
    /// Digest of the frozen validation policy manifest.
    pub policy_manifest_sha256: String,
    /// Exact evaluator build permitted to produce metric evidence.
    pub required_evaluator_build_sha256: String,
    /// Exact model and operating-domain identity requested for credit.
    pub expected_scope: ModelScope,
    /// Exact reference measurement/calibration profile required for all datasets.
    pub expected_reference_measurement_profile_sha256: String,
    /// Strata that must be explicitly represented in confirmatory evidence.
    pub required_strata: Vec<String>,
    /// Predeclared metric gates, one per metric/unit/stratum/modality tuple.
    pub gates: Vec<MetricGate>,
}

/// Validation package with separated calibration/development/confirmatory manifests.
#[derive(Clone, Debug, PartialEq)]
pub struct ValidationPackage {
    /// Exact evaluator build that produced the metric results.
    pub evaluator_build_sha256: String,
    /// Exact frozen policy manifest used for evaluation.
    pub policy_manifest_sha256: String,
    /// Exact model and operating-domain identity under evaluation.
    pub scope: ModelScope,
    /// Calibration dataset manifest.
    pub calibration: DatasetManifest,
    /// Optional development dataset manifest.
    pub development: Option<DatasetManifest>,
    /// Held-out confirmatory dataset manifest.
    pub confirmatory: DatasetManifest,
    /// Metric results from the confirmatory manifest only.
    pub metrics: Vec<MetricEvidence>,
}

/// Positive package status. This does not say the model is physically validated.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidationDisposition {
    /// Structure and supplied metrics satisfy this frozen profile for further review only.
    AdmissibleForDeclaredDomainReviewOnly,
    /// Requested model/configuration/domain differs from the package's exact scope.
    NotCovered,
}

/// Result of evaluating a validation package against a separate profile.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationResult {
    /// Structural disposition.
    pub disposition: ValidationDisposition,
    /// Number of preregistered metric gates satisfied.
    pub admitted_metric_count: usize,
    /// Exact declared strata represented in held-out evidence.
    pub covered_strata: Vec<String>,
    /// Model artifact digest string; not recomputed or authenticated here.
    pub model_artifact_sha256: String,
    /// Operating-domain digest string.
    pub operating_domain_sha256: String,
    /// Exact evaluator build digest string.
    pub evaluator_build_sha256: String,
    /// Exact validation-policy manifest digest string.
    pub policy_manifest_sha256: String,
}

/// Typed reason a validation package cannot receive structural credit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidationError {
    /// The separately frozen policy is incomplete or internally contradictory.
    InvalidPolicy,
    /// Package headers, manifest identities or numeric values are malformed.
    InvalidPackage,
    /// Calibration, development and confirmatory groups overlap.
    DataLeakage,
    /// Dataset manifest applies to a different operating domain.
    DatasetScopeMismatch,
    /// One or more required validation strata are absent.
    MissingRequiredStratum,
    /// A required metric gate has no exact matching result.
    MissingMetric,
    /// A result is duplicated or was not requested by the frozen policy.
    UnexpectedOrDuplicateMetric,
    /// A metric result references a dataset other than the held-out confirmatory set.
    WrongConfirmatoryManifest,
    /// Independent unit count is below the preregistered floor.
    InsufficientIndependentUnits,
    /// A measured statistic falls outside its preregistered gate.
    MetricOutsideGate,
    /// Confidence bounds, level or interval-method identity violate the gate.
    ConfidenceContractMismatch,
    /// Evaluator build or policy manifest differs from the frozen requirement.
    EvaluatorOrPolicyMismatch,
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn unique_nonempty(values: &[String]) -> bool {
    let mut seen = BTreeSet::new();
    values
        .iter()
        .all(|value| !value.trim().is_empty() && seen.insert(value.as_str()))
}

fn intersects(left: &[String], right: &[String]) -> bool {
    let right_set: BTreeSet<&str> = right.iter().map(String::as_str).collect();
    left.iter().any(|value| right_set.contains(value.as_str()))
}

fn validate_manifest(
    manifest: &DatasetManifest,
    expected_split: DatasetSplit,
    expected_scope: &ModelScope,
    expected_reference_profile: &str,
) -> Result<(), ValidationError> {
    if !is_sha256(&manifest.manifest_sha256)
        || !is_sha256(&manifest.operating_domain_sha256)
        || !is_sha256(&manifest.aircraft_configuration_sha256)
        || !is_sha256(&manifest.control_stack_sha256)
        || !is_sha256(&manifest.sensor_stack_sha256)
        || !is_sha256(&manifest.reference_measurement_profile_sha256)
        || manifest.horizontal_frame_id.trim().is_empty()
        || manifest.vertical_datum_id.trim().is_empty()
        || manifest.time_scale_id.trim().is_empty()
        || manifest.split != expected_split
        || manifest.flight_group_ids.is_empty()
        || manifest.scenario_group_ids.is_empty()
        || manifest.strata.is_empty()
        || !unique_nonempty(&manifest.flight_group_ids)
        || !unique_nonempty(&manifest.scenario_group_ids)
        || !unique_nonempty(&manifest.strata)
    {
        return Err(ValidationError::InvalidPackage);
    }
    if manifest.operating_domain_sha256 != expected_scope.operating_domain_sha256
        || manifest.aircraft_configuration_sha256 != expected_scope.aircraft_configuration_sha256
        || manifest.control_stack_sha256 != expected_scope.control_stack_sha256
        || manifest.sensor_stack_sha256 != expected_scope.sensor_stack_sha256
        || manifest.horizontal_frame_id != expected_scope.horizontal_frame_id
        || manifest.vertical_datum_id != expected_scope.vertical_datum_id
        || manifest.time_scale_id != expected_scope.time_scale_id
        || manifest.reference_measurement_profile_sha256 != expected_reference_profile
    {
        return Err(ValidationError::DatasetScopeMismatch);
    }
    Ok(())
}

fn gate_key(
    metric: MetricKind,
    sample_unit: SampleUnit,
    stratum_id: &str,
    modality: EvidenceModality,
) -> (MetricKind, SampleUnit, String, EvidenceModality) {
    (metric, sample_unit, stratum_id.to_owned(), modality)
}

/// Evaluate structural admissibility of one model-validation package.
///
/// This function does not independently calculate metrics or authenticate the
/// datasets, sensor calibration, reference measurements, evaluator or signatures.
/// A positive result is only AdmissibleForDeclaredDomainReviewOnly.
pub fn evaluate(
    package: &ValidationPackage,
    policy: &ValidationPolicy,
) -> Result<ValidationResult, ValidationError> {
    if !is_sha256(&policy.policy_manifest_sha256)
        || !is_sha256(&policy.required_evaluator_build_sha256)
        || !policy.expected_scope.is_complete()
        || !is_sha256(&policy.expected_reference_measurement_profile_sha256)
        || policy.required_strata.is_empty()
        || !unique_nonempty(&policy.required_strata)
        || policy.gates.is_empty()
    {
        return Err(ValidationError::InvalidPolicy);
    }
    let mut gate_keys = BTreeSet::new();
    for gate in &policy.gates {
        if gate.stratum_id.trim().is_empty()
            || !gate.threshold.is_finite()
            || !gate.confidence_level.is_finite()
            || gate.confidence_level <= 0.0
            || gate.confidence_level >= 1.0
            || !is_sha256(&gate.confidence_method_sha256)
            || gate.minimum_independent_units == 0
            || !gate_keys.insert(gate_key(
                gate.metric,
                gate.sample_unit,
                &gate.stratum_id,
                gate.modality,
            ))
        {
            return Err(ValidationError::InvalidPolicy);
        }
    }
    if policy
        .required_strata
        .iter()
        .any(|stratum| !policy.gates.iter().any(|gate| gate.stratum_id == *stratum))
        || policy
            .gates
            .iter()
            .any(|gate| !policy.required_strata.contains(&gate.stratum_id))
    {
        return Err(ValidationError::InvalidPolicy);
    }

    if !package.scope.is_complete()
        || !is_sha256(&package.evaluator_build_sha256)
        || !is_sha256(&package.policy_manifest_sha256)
    {
        return Err(ValidationError::InvalidPackage);
    }
    if package.scope != policy.expected_scope {
        return Ok(ValidationResult {
            disposition: ValidationDisposition::NotCovered,
            admitted_metric_count: 0,
            covered_strata: Vec::new(),
            model_artifact_sha256: package.scope.model_artifact_sha256.clone(),
            operating_domain_sha256: package.scope.operating_domain_sha256.clone(),
            evaluator_build_sha256: package.evaluator_build_sha256.clone(),
            policy_manifest_sha256: package.policy_manifest_sha256.clone(),
        });
    }

    if package.evaluator_build_sha256 != policy.required_evaluator_build_sha256
        || package.policy_manifest_sha256 != policy.policy_manifest_sha256
    {
        return Err(ValidationError::EvaluatorOrPolicyMismatch);
    }

    validate_manifest(
        &package.calibration,
        DatasetSplit::Calibration,
        &package.scope,
        &policy.expected_reference_measurement_profile_sha256,
    )?;
    validate_manifest(
        &package.confirmatory,
        DatasetSplit::Confirmatory,
        &package.scope,
        &policy.expected_reference_measurement_profile_sha256,
    )?;
    if package.calibration.manifest_sha256 == package.confirmatory.manifest_sha256
        || intersects(
            &package.calibration.flight_group_ids,
            &package.confirmatory.flight_group_ids,
        )
        || intersects(
            &package.calibration.scenario_group_ids,
            &package.confirmatory.scenario_group_ids,
        )
    {
        return Err(ValidationError::DataLeakage);
    }

    if let Some(development) = &package.development {
        validate_manifest(
            development,
            DatasetSplit::Development,
            &package.scope,
            &policy.expected_reference_measurement_profile_sha256,
        )?;
        if development.manifest_sha256 == package.calibration.manifest_sha256
            || development.manifest_sha256 == package.confirmatory.manifest_sha256
            || intersects(
                &development.flight_group_ids,
                &package.calibration.flight_group_ids,
            )
            || intersects(
                &development.flight_group_ids,
                &package.confirmatory.flight_group_ids,
            )
            || intersects(
                &development.scenario_group_ids,
                &package.calibration.scenario_group_ids,
            )
            || intersects(
                &development.scenario_group_ids,
                &package.confirmatory.scenario_group_ids,
            )
        {
            return Err(ValidationError::DataLeakage);
        }
    }

    if policy
        .required_strata
        .iter()
        .any(|stratum| !package.confirmatory.strata.contains(stratum))
    {
        return Err(ValidationError::MissingRequiredStratum);
    }

    let mut evidence_keys = BTreeSet::new();
    for evidence in &package.metrics {
        if !evidence.value.is_finite()
            || !evidence.lower_confidence_bound.is_finite()
            || !evidence.upper_confidence_bound.is_finite()
            || !evidence.confidence_level.is_finite()
            || evidence.confidence_level <= 0.0
            || evidence.confidence_level >= 1.0
            || !is_sha256(&evidence.confidence_method_sha256)
            || evidence.lower_confidence_bound > evidence.value
            || evidence.value > evidence.upper_confidence_bound
            || evidence.independent_unit_count == 0
            || evidence.stratum_id.trim().is_empty()
        {
            return Err(ValidationError::InvalidPackage);
        }
        if evidence.confirmatory_manifest_sha256 != package.confirmatory.manifest_sha256 {
            return Err(ValidationError::WrongConfirmatoryManifest);
        }
        let key = gate_key(
            evidence.metric,
            evidence.sample_unit,
            &evidence.stratum_id,
            evidence.modality,
        );
        if !gate_keys.contains(&key) || !evidence_keys.insert(key) {
            return Err(ValidationError::UnexpectedOrDuplicateMetric);
        }
    }

    for gate in &policy.gates {
        let key = gate_key(
            gate.metric,
            gate.sample_unit,
            &gate.stratum_id,
            gate.modality,
        );
        let Some(evidence) = package.metrics.iter().find(|record| {
            gate_key(
                record.metric,
                record.sample_unit,
                &record.stratum_id,
                record.modality,
            ) == key
        }) else {
            return Err(ValidationError::MissingMetric);
        };
        if evidence.independent_unit_count < gate.minimum_independent_units {
            return Err(ValidationError::InsufficientIndependentUnits);
        }
        if evidence.confidence_level != gate.confidence_level
            || evidence.confidence_method_sha256 != gate.confidence_method_sha256
        {
            return Err(ValidationError::ConfidenceContractMismatch);
        }
        let passes = match gate.comparator {
            Comparator::AtMost => evidence.upper_confidence_bound <= gate.threshold,
            Comparator::AtLeast => evidence.lower_confidence_bound >= gate.threshold,
        };
        if !passes {
            return Err(ValidationError::MetricOutsideGate);
        }
    }

    let mut covered_strata = policy.required_strata.clone();
    covered_strata.sort_unstable();
    Ok(ValidationResult {
        disposition: ValidationDisposition::AdmissibleForDeclaredDomainReviewOnly,
        admitted_metric_count: package.metrics.len(),
        covered_strata,
        model_artifact_sha256: package.scope.model_artifact_sha256.clone(),
        operating_domain_sha256: package.scope.operating_domain_sha256.clone(),
        evaluator_build_sha256: package.evaluator_build_sha256.clone(),
        policy_manifest_sha256: package.policy_manifest_sha256.clone(),
    })
}
