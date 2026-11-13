//! Robotics joint measurement-uncertainty receipt completeness auditor.
//!
//! Consumes upstream calibration, uncertainty-budget and distribution receipts.
//! It does not calculate uncertainty, covariance, physical residuals, or safety.
//! Missing dependence information is never interpreted as independence.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

/// Exact deployment and experiment identity shared by every receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeBinding {
    pub mission_id: String,
    pub robot_id: String,
    pub actuator_id: String,
    pub actuator_configuration_digest: String,
    pub controller_configuration_digest: String,
    pub scenario_manifest_digest: String,
    pub test_manifest_digest: String,
    pub measurement_window_id: String,
}

/// Physical or derived quantity whose measurement evidence is being qualified.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum MeasurementKind {
    ElectricalVoltage,
    ElectricalCurrent,
    ShaftTorque,
    AngularVelocity,
    AngularPosition,
    ThermalNode(String),
    LoadDisturbance,
    ClockMapping,
    DerivedElectricalEnergy,
    DerivedShaftWork,
    DerivedEnergyResidual,
}

/// Upstream evidence disposition. The adapter may downgrade but never upgrade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceDisposition {
    Qualified,
    Unknown,
    Inconclusive,
    Rejected,
}

/// Method declared by the upstream uncertainty authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoverageMethod {
    ConservativeBound,
    FirstOrderGum,
    HigherOrderGum,
    MonteCarloDistribution,
    OtherApproved,
}

/// Coverage representation; deterministic bounds and probabilistic intervals
/// are distinct types and must never be silently interchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoverageRepresentation {
    ConservativeBound {
        lower_scaled: i64,
        upper_scaled: i64,
        unit_code: String,
        decimal_exponent: i8,
    },
    ProbabilityInterval {
        lower_scaled: i64,
        upper_scaled: i64,
        unit_code: String,
        decimal_exponent: i8,
        coverage_probability_ppm: u32,
        method: CoverageMethod,
        coverage_receipt_digest: String,
        convergence_receipt_digest: Option<String>,
    },
}

/// Known uncertainty budget component classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ComponentKind {
    Repeatability,
    Reproducibility,
    CalibrationReference,
    BiasLinearity,
    TemporalDrift,
    Resolution,
    ClockSynchronization,
    ModelDiscrepancy,
    Other,
}

/// One upstream component receipt. Dependence grouping is mandatory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetComponent {
    pub component_id: String,
    pub kind: ComponentKind,
    pub source_receipt_digest: String,
    pub dependence_group_id: String,
}

/// Dependence may only be supported by a qualified joint model or an
/// evidence-backed independence determination. Unknown remains Unknown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependenceDisposition {
    JointModel {
        joint_model_receipt_digest: String,
    },
    IndependenceSupported {
        independence_receipt_digest: String,
    },
    Unknown,
}

/// One explicit dependence decision for a group of budget components.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependenceDeclaration {
    pub dependence_group_id: String,
    pub disposition: DependenceDisposition,
}

/// Upstream uncertainty budget/distribution receipt for one measured quantity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UncertaintyReceipt {
    pub receipt_id: String,
    pub measurement_id: String,
    pub scope: ScopeBinding,
    pub measurement_artifact_digest: String,
    pub instrument_configuration_digest: String,
    pub calibration_receipt_digest: String,
    pub unit_code: String,
    pub decimal_exponent: i8,
    pub coverage: CoverageRepresentation,
    pub components: Vec<BudgetComponent>,
    pub dependence: Vec<DependenceDeclaration>,
    pub budget_result_receipt_digest: String,
    pub disposition: EvidenceDisposition,
    pub valid_from_tick: u64,
    pub valid_through_tick: u64,
}

/// Instrument-backed measurement identity. Raw artifacts remain external and
/// are content-addressed by measurement_artifact_digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasurementReceipt {
    pub measurement_id: String,
    pub kind: MeasurementKind,
    pub scope: ScopeBinding,
    pub measurement_artifact_digest: String,
    pub instrument_configuration_digest: String,
    pub calibration_receipt_digest: String,
    pub unit_code: String,
    pub decimal_exponent: i8,
    pub start_tick: u64,
    pub end_tick: u64,
    pub disposition: EvidenceDisposition,
    pub uncertainty_receipt_id: Option<String>,
}

/// Derived result receipt must enumerate every exact measurement and
/// uncertainty receipt used by its upstream physical calculation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivedResultReceipt {
    pub result_id: String,
    pub output_measurement_id: String,
    pub input_measurement_ids: Vec<String>,
    pub input_uncertainty_receipt_ids: Vec<String>,
    pub output_result_digest: String,
    pub upstream_disposition: EvidenceDisposition,
}

/// Complete robotics measurement-uncertainty evidence profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UncertaintyProfile {
    pub scope: ScopeBinding,
    pub evaluation_tick: u64,
    pub required_measurement_ids: Vec<String>,
    pub measurements: Vec<MeasurementReceipt>,
    pub uncertainty_receipts: Vec<UncertaintyReceipt>,
    pub derived_results: Vec<DerivedResultReceipt>,
}

/// Stable reason-coded finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub disposition: EvidenceDisposition,
    pub code: &'static str,
    pub identity: String,
}

/// Scope-completeness result. It is not a conformity or safety certificate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UncertaintyAuditReport {
    pub disposition: EvidenceDisposition,
    pub checked_measurement_count: usize,
    pub checked_uncertainty_receipt_count: usize,
    pub checked_derived_result_count: usize,
    pub findings: Vec<Finding>,
    pub conformity_certified: bool,
    pub actuation_authorized: bool,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn escalate(current: EvidenceDisposition, next: EvidenceDisposition) -> EvidenceDisposition {
    fn rank(value: EvidenceDisposition) -> u8 {
        match value {
            EvidenceDisposition::Qualified => 0,
            EvidenceDisposition::Unknown => 1,
            EvidenceDisposition::Inconclusive => 2,
            EvidenceDisposition::Rejected => 3,
        }
    }
    if rank(next) > rank(current) {
        next
    } else {
        current
    }
}

fn push(
    report: &mut UncertaintyAuditReport,
    disposition: EvidenceDisposition,
    code: &'static str,
    identity: impl Into<String>,
) {
    report.disposition = escalate(report.disposition, disposition);
    report.findings.push(Finding {
        disposition,
        code,
        identity: identity.into(),
    });
}

fn scope_matches(expected: &ScopeBinding, observed: &ScopeBinding) -> bool {
    expected == observed
}

fn validate_scope(scope: &ScopeBinding, report: &mut UncertaintyAuditReport) {
    for (field, value) in [
        ("mission_id", scope.mission_id.as_str()),
        ("robot_id", scope.robot_id.as_str()),
        ("actuator_id", scope.actuator_id.as_str()),
        (
            "actuator_configuration_digest",
            scope.actuator_configuration_digest.as_str(),
        ),
        (
            "controller_configuration_digest",
            scope.controller_configuration_digest.as_str(),
        ),
        (
            "scenario_manifest_digest",
            scope.scenario_manifest_digest.as_str(),
        ),
        ("test_manifest_digest", scope.test_manifest_digest.as_str()),
        (
            "measurement_window_id",
            scope.measurement_window_id.as_str(),
        ),
    ] {
        if !nonempty(value) {
            push(
                report,
                EvidenceDisposition::Rejected,
                "PROFILE_SCOPE_IDENTITY_EMPTY",
                field,
            );
        }
    }
}

fn validate_coverage(
    coverage: &CoverageRepresentation,
    expected_unit: &str,
    expected_exponent: i8,
    identity: &str,
    report: &mut UncertaintyAuditReport,
) {
    let (lower, upper, unit, exponent) = match coverage {
        CoverageRepresentation::ConservativeBound {
            lower_scaled,
            upper_scaled,
            unit_code,
            decimal_exponent,
        } => (
            *lower_scaled,
            *upper_scaled,
            unit_code.as_str(),
            *decimal_exponent,
        ),
        CoverageRepresentation::ProbabilityInterval {
            lower_scaled,
            upper_scaled,
            unit_code,
            decimal_exponent,
            coverage_probability_ppm,
            method,
            coverage_receipt_digest,
            convergence_receipt_digest,
        } => {
            if *coverage_probability_ppm == 0 || *coverage_probability_ppm > 1_000_000 {
                push(
                    report,
                    EvidenceDisposition::Rejected,
                    "COVERAGE_PROBABILITY_OUT_OF_RANGE",
                    identity,
                );
            }
            if !nonempty(coverage_receipt_digest) {
                push(
                    report,
                    EvidenceDisposition::Unknown,
                    "COVERAGE_RECEIPT_MISSING",
                    identity,
                );
            }
            if *method == CoverageMethod::ConservativeBound {
                push(
                    report,
                    EvidenceDisposition::Rejected,
                    "PROBABILISTIC_INTERVAL_HAS_BOUND_ONLY_METHOD",
                    identity,
                );
            }
            if *method == CoverageMethod::MonteCarloDistribution
                && convergence_receipt_digest
                    .as_ref()
                    .is_none_or(|digest| !nonempty(digest))
            {
                push(
                    report,
                    EvidenceDisposition::Inconclusive,
                    "MONTE_CARLO_CONVERGENCE_RECEIPT_MISSING",
                    identity,
                );
            }
            (
                *lower_scaled,
                *upper_scaled,
                unit_code.as_str(),
                *decimal_exponent,
            )
        }
    };

    if lower > upper {
        push(
            report,
            EvidenceDisposition::Rejected,
            "COVERAGE_INTERVAL_INVERTED",
            identity,
        );
    }
    if !nonempty(unit) || unit != expected_unit || exponent != expected_exponent {
        push(
            report,
            EvidenceDisposition::Rejected,
            "UNCERTAINTY_UNIT_OR_SCALE_MISMATCH",
            identity,
        );
    }
    if !(-18..=18).contains(&exponent) {
        push(
            report,
            EvidenceDisposition::Rejected,
            "DECIMAL_EXPONENT_OUT_OF_RANGE",
            identity,
        );
    }
}

fn validate_uncertainty_receipt(
    receipt: &UncertaintyReceipt,
    measurement: Option<&MeasurementReceipt>,
    profile: &UncertaintyProfile,
    report: &mut UncertaintyAuditReport,
) {
    let identity = receipt.receipt_id.as_str();
    for (field, value) in [
        ("receipt_id", receipt.receipt_id.as_str()),
        ("measurement_id", receipt.measurement_id.as_str()),
        (
            "measurement_artifact_digest",
            receipt.measurement_artifact_digest.as_str(),
        ),
        (
            "instrument_configuration_digest",
            receipt.instrument_configuration_digest.as_str(),
        ),
        (
            "calibration_receipt_digest",
            receipt.calibration_receipt_digest.as_str(),
        ),
        ("unit_code", receipt.unit_code.as_str()),
        (
            "budget_result_receipt_digest",
            receipt.budget_result_receipt_digest.as_str(),
        ),
    ] {
        if !nonempty(value) {
            push(
                report,
                EvidenceDisposition::Rejected,
                "UNCERTAINTY_RECEIPT_IDENTITY_EMPTY",
                format!("{identity}:{field}"),
            );
        }
    }

    if !scope_matches(&profile.scope, &receipt.scope) {
        push(
            report,
            EvidenceDisposition::Rejected,
            "UNCERTAINTY_SCOPE_MISMATCH",
            identity,
        );
    }
    if receipt.valid_from_tick > receipt.valid_through_tick {
        push(
            report,
            EvidenceDisposition::Rejected,
            "UNCERTAINTY_VALIDITY_INTERVAL_INVERTED",
            identity,
        );
    } else if profile.evaluation_tick < receipt.valid_from_tick
        || profile.evaluation_tick > receipt.valid_through_tick
    {
        push(
            report,
            EvidenceDisposition::Unknown,
            "UNCERTAINTY_RECEIPT_STALE_OR_NOT_YET_VALID",
            identity,
        );
    }

    match receipt.disposition {
        EvidenceDisposition::Qualified => {}
        EvidenceDisposition::Unknown => push(
            report,
            EvidenceDisposition::Unknown,
            "UPSTREAM_UNCERTAINTY_UNKNOWN",
            identity,
        ),
        EvidenceDisposition::Inconclusive => push(
            report,
            EvidenceDisposition::Inconclusive,
            "UPSTREAM_UNCERTAINTY_INCONCLUSIVE",
            identity,
        ),
        EvidenceDisposition::Rejected => push(
            report,
            EvidenceDisposition::Rejected,
            "UPSTREAM_UNCERTAINTY_REJECTED",
            identity,
        ),
    }

    match measurement {
        Some(measurement) => {
            if receipt.measurement_id != measurement.measurement_id
                || receipt.measurement_artifact_digest != measurement.measurement_artifact_digest
                || receipt.instrument_configuration_digest
                    != measurement.instrument_configuration_digest
                || receipt.calibration_receipt_digest != measurement.calibration_receipt_digest
                || receipt.unit_code != measurement.unit_code
                || receipt.decimal_exponent != measurement.decimal_exponent
            {
                push(
                    report,
                    EvidenceDisposition::Rejected,
                    "UNCERTAINTY_MEASUREMENT_BINDING_MISMATCH",
                    identity,
                );
            }
        }
        None => push(
            report,
            EvidenceDisposition::Rejected,
            "UNCERTAINTY_RECEIPT_FOR_UNKNOWN_MEASUREMENT",
            identity,
        ),
    }

    validate_coverage(
        &receipt.coverage,
        &receipt.unit_code,
        receipt.decimal_exponent,
        identity,
        report,
    );

    if receipt.components.is_empty() {
        push(
            report,
            EvidenceDisposition::Unknown,
            "UNCERTAINTY_COMPONENT_LEDGER_EMPTY",
            identity,
        );
    }

    let mut component_ids = BTreeSet::new();
    let mut required_groups = BTreeSet::new();
    for component in &receipt.components {
        if !nonempty(&component.component_id)
            || !nonempty(&component.source_receipt_digest)
            || !nonempty(&component.dependence_group_id)
        {
            push(
                report,
                EvidenceDisposition::Rejected,
                "UNCERTAINTY_COMPONENT_IDENTITY_EMPTY",
                identity,
            );
        }
        if !component_ids.insert(component.component_id.as_str()) {
            push(
                report,
                EvidenceDisposition::Rejected,
                "DUPLICATE_UNCERTAINTY_COMPONENT",
                format!("{identity}:{}", component.component_id),
            );
        }
        required_groups.insert(component.dependence_group_id.as_str());
    }

    let mut observed_groups = BTreeSet::new();
    for declaration in &receipt.dependence {
        let group = declaration.dependence_group_id.as_str();
        if !nonempty(group) {
            push(
                report,
                EvidenceDisposition::Rejected,
                "DEPENDENCE_GROUP_ID_EMPTY",
                identity,
            );
        }
        if !observed_groups.insert(group) {
            push(
                report,
                EvidenceDisposition::Rejected,
                "DUPLICATE_DEPENDENCE_DECLARATION",
                format!("{identity}:{group}"),
            );
        }
        match &declaration.disposition {
            DependenceDisposition::JointModel {
                joint_model_receipt_digest,
            } if !nonempty(joint_model_receipt_digest) => push(
                report,
                EvidenceDisposition::Rejected,
                "JOINT_DEPENDENCE_MODEL_RECEIPT_EMPTY",
                format!("{identity}:{group}"),
            ),
            DependenceDisposition::IndependenceSupported {
                independence_receipt_digest,
            } if !nonempty(independence_receipt_digest) => push(
                report,
                EvidenceDisposition::Rejected,
                "INDEPENDENCE_EVIDENCE_RECEIPT_EMPTY",
                format!("{identity}:{group}"),
            ),
            DependenceDisposition::Unknown => push(
                report,
                EvidenceDisposition::Unknown,
                "DEPENDENCE_UNRESOLVED",
                format!("{identity}:{group}"),
            ),
            _ => {}
        }
    }

    for group in required_groups.difference(&observed_groups) {
        push(
            report,
            EvidenceDisposition::Unknown,
            "DEPENDENCE_DECLARATION_MISSING",
            format!("{identity}:{group}"),
        );
    }
    for group in observed_groups.difference(&required_groups) {
        push(
            report,
            EvidenceDisposition::Rejected,
            "DEPENDENCE_DECLARATION_UNUSED",
            format!("{identity}:{group}"),
        );
    }
}

fn validate_derived_result(
    derived: &DerivedResultReceipt,
    measurements: &BTreeMap<&str, &MeasurementReceipt>,
    receipts: &BTreeMap<&str, &UncertaintyReceipt>,
    report: &mut UncertaintyAuditReport,
) {
    let identity = derived.result_id.as_str();
    if !nonempty(identity) || !nonempty(&derived.output_result_digest) {
        push(
            report,
            EvidenceDisposition::Rejected,
            "DERIVED_RESULT_IDENTITY_EMPTY",
            identity,
        );
    }

    match derived.upstream_disposition {
        EvidenceDisposition::Qualified => {}
        EvidenceDisposition::Unknown => push(
            report,
            EvidenceDisposition::Unknown,
            "DERIVED_RESULT_UPSTREAM_UNKNOWN",
            identity,
        ),
        EvidenceDisposition::Inconclusive => push(
            report,
            EvidenceDisposition::Inconclusive,
            "DERIVED_RESULT_UPSTREAM_INCONCLUSIVE",
            identity,
        ),
        EvidenceDisposition::Rejected => push(
            report,
            EvidenceDisposition::Rejected,
            "DERIVED_RESULT_UPSTREAM_REJECTED",
            identity,
        ),
    }

    let mut expected_uncertainty_ids = BTreeSet::new();
    let mut input_measurement_ids = BTreeSet::new();
    for measurement_id in &derived.input_measurement_ids {
        if !input_measurement_ids.insert(measurement_id.as_str()) {
            push(
                report,
                EvidenceDisposition::Rejected,
                "DUPLICATE_DERIVED_INPUT_MEASUREMENT",
                format!("{identity}:{measurement_id}"),
            );
            continue;
        }
        match measurements.get(measurement_id.as_str()) {
            Some(measurement) => match measurement.uncertainty_receipt_id.as_deref() {
                Some(receipt_id) => {
                    expected_uncertainty_ids.insert(receipt_id);
                }
                None => push(
                    report,
                    EvidenceDisposition::Unknown,
                    "DERIVED_INPUT_UNCERTAINTY_MISSING",
                    format!("{identity}:{measurement_id}"),
                ),
            },
            None => push(
                report,
                EvidenceDisposition::Unknown,
                "DERIVED_INPUT_MEASUREMENT_MISSING",
                format!("{identity}:{measurement_id}"),
            ),
        }
    }

    let mut supplied_uncertainty_ids = BTreeSet::new();
    for receipt_id in &derived.input_uncertainty_receipt_ids {
        if !supplied_uncertainty_ids.insert(receipt_id.as_str()) {
            push(
                report,
                EvidenceDisposition::Rejected,
                "DUPLICATE_DERIVED_INPUT_UNCERTAINTY",
                format!("{identity}:{receipt_id}"),
            );
        }
        if !receipts.contains_key(receipt_id.as_str()) {
            push(
                report,
                EvidenceDisposition::Unknown,
                "DERIVED_INPUT_UNCERTAINTY_RECEIPT_UNKNOWN",
                format!("{identity}:{receipt_id}"),
            );
        }
    }

    if expected_uncertainty_ids != supplied_uncertainty_ids {
        push(
            report,
            EvidenceDisposition::Rejected,
            "DERIVED_INPUT_UNCERTAINTY_SET_MISMATCH",
            identity,
        );
    }

    match measurements.get(derived.output_measurement_id.as_str()) {
        Some(output) => {
            if !matches!(
                &output.kind,
                MeasurementKind::DerivedElectricalEnergy
                    | MeasurementKind::DerivedShaftWork
                    | MeasurementKind::DerivedEnergyResidual
            ) {
                push(
                    report,
                    EvidenceDisposition::Rejected,
                    "DERIVED_OUTPUT_KIND_MISMATCH",
                    identity,
                );
            }
            if derived.result_id != derived.output_measurement_id
                || derived.output_result_digest != output.measurement_artifact_digest
            {
                push(
                    report,
                    EvidenceDisposition::Rejected,
                    "DERIVED_OUTPUT_IDENTITY_MISMATCH",
                    identity,
                );
            }
            if output.uncertainty_receipt_id.as_deref()
                != Some(derived.output_uncertainty_receipt_id.as_str())
            {
                push(
                    report,
                    EvidenceDisposition::Rejected,
                    "DERIVED_OUTPUT_UNCERTAINTY_BINDING_MISMATCH",
                    identity,
                );
            }
        }
        None => push(
            report,
            EvidenceDisposition::Unknown,
            "DERIVED_OUTPUT_MEASUREMENT_MISSING",
            identity,
        ),
    }

    if !receipts.contains_key(derived.output_uncertainty_receipt_id.as_str()) {
        push(
            report,
            EvidenceDisposition::Unknown,
            "DERIVED_OUTPUT_UNCERTAINTY_RECEIPT_MISSING",
            identity,
        );
    }
}

/// Audit uncertainty receipts against exact robot measurement scope.
///
/// The function consumes upstream budget and distribution conclusions. It
/// never computes uncertainty or upgrades a non-qualified upstream result.
pub fn audit_measurement_uncertainty(
    profile: &UncertaintyProfile,
) -> UncertaintyAuditReport {
    let mut report = UncertaintyAuditReport {
        disposition: EvidenceDisposition::Qualified,
        checked_measurement_count: profile.measurements.len(),
        checked_uncertainty_receipt_count: profile.uncertainty_receipts.len(),
        checked_derived_result_count: profile.derived_results.len(),
        findings: Vec::new(),
        conformity_certified: false,
        actuation_authorized: false,
    };

    validate_scope(&profile.scope, &mut report);

    let mut required_ids = BTreeSet::new();
    for id in &profile.required_measurement_ids {
        if !nonempty(id) || !required_ids.insert(id.as_str()) {
            push(
                &mut report,
                EvidenceDisposition::Rejected,
                "REQUIRED_MEASUREMENT_ID_EMPTY_OR_DUPLICATE",
                id,
            );
        }
    }

    let mut measurement_map = BTreeMap::new();
    for measurement in &profile.measurements {
        let id = measurement.measurement_id.as_str();
        if !nonempty(id) {
            push(
                &mut report,
                EvidenceDisposition::Rejected,
                "MEASUREMENT_ID_EMPTY",
                id,
            );
            continue;
        }
        if measurement_map.insert(id, measurement).is_some() {
            push(
                &mut report,
                EvidenceDisposition::Rejected,
                "DUPLICATE_MEASUREMENT_ID",
                id,
            );
        }
        if !scope_matches(&profile.scope, &measurement.scope) {
            push(
                &mut report,
                EvidenceDisposition::Rejected,
                "MEASUREMENT_SCOPE_MISMATCH",
                id,
            );
        }
        if !nonempty(&measurement.measurement_artifact_digest)
            || !nonempty(&measurement.instrument_configuration_digest)
            || !nonempty(&measurement.calibration_receipt_digest)
            || !nonempty(&measurement.unit_code)
        {
            push(
                &mut report,
                EvidenceDisposition::Rejected,
                "MEASUREMENT_IDENTITY_INCOMPLETE",
                id,
            );
        }
        if measurement.start_tick > measurement.end_tick {
            push(
                &mut report,
                EvidenceDisposition::Rejected,
                "MEASUREMENT_WINDOW_INVERTED",
                id,
            );
        }
        match measurement.disposition {
            EvidenceDisposition::Qualified => {}
            EvidenceDisposition::Unknown => push(
                &mut report,
                EvidenceDisposition::Unknown,
                "MEASUREMENT_UPSTREAM_UNKNOWN",
                id,
            ),
            EvidenceDisposition::Inconclusive => push(
                &mut report,
                EvidenceDisposition::Inconclusive,
                "MEASUREMENT_UPSTREAM_INCONCLUSIVE",
                id,
            ),
            EvidenceDisposition::Rejected => push(
                &mut report,
                EvidenceDisposition::Rejected,
                "MEASUREMENT_UPSTREAM_REJECTED",
                id,
            ),
        }
    }

    for required_id in &required_ids {
        if !measurement_map.contains_key(required_id) {
            push(
                &mut report,
                EvidenceDisposition::Unknown,
                "REQUIRED_MEASUREMENT_MISSING",
                *required_id,
            );
        }
    }

    let mut uncertainty_map = BTreeMap::new();
    for receipt in &profile.uncertainty_receipts {
        let id = receipt.receipt_id.as_str();
        if !nonempty(id) {
            push(
                &mut report,
                EvidenceDisposition::Rejected,
                "UNCERTAINTY_RECEIPT_ID_EMPTY",
                id,
            );
            continue;
        }
        if uncertainty_map.insert(id, receipt).is_some() {
            push(
                &mut report,
                EvidenceDisposition::Rejected,
                "DUPLICATE_UNCERTAINTY_RECEIPT_ID",
                id,
            );
        }
        let measurement = measurement_map.get(receipt.measurement_id.as_str()).copied();
        validate_uncertainty_receipt(receipt, measurement, profile, &mut report);
    }

    for measurement in &profile.measurements {
        match measurement.uncertainty_receipt_id.as_deref() {
            Some(receipt_id) => match uncertainty_map.get(receipt_id).copied() {
                Some(receipt) if receipt.measurement_id == measurement.measurement_id => {}
                Some(_) => push(
                    &mut report,
                    EvidenceDisposition::Rejected,
                    "MEASUREMENT_UNCERTAINTY_RECEIPT_POINTS_TO_OTHER_MEASUREMENT",
                    &measurement.measurement_id,
                ),
                None => push(
                    &mut report,
                    EvidenceDisposition::Unknown,
                    "MEASUREMENT_UNCERTAINTY_RECEIPT_MISSING",
                    &measurement.measurement_id,
                ),
            },
            None if required_ids.contains(measurement.measurement_id.as_str()) => push(
                &mut report,
                EvidenceDisposition::Unknown,
                "REQUIRED_MEASUREMENT_HAS_NO_UNCERTAINTY_RECEIPT",
                &measurement.measurement_id,
            ),
            None => {}
        }
    }

    for derived in &profile.derived_results {
        validate_derived_result(derived, &measurement_map, &uncertainty_map, &mut report);
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope() -> ScopeBinding {
        ScopeBinding {
            mission_id: "mission-001".into(),
            robot_id: "robot-001".into(),
            actuator_id: "hip-left".into(),
            actuator_configuration_digest: "actuator-config-digest".into(),
            controller_configuration_digest: "controller-config-digest".into(),
            scenario_manifest_digest: "scenario-digest".into(),
            test_manifest_digest: "test-manifest-digest".into(),
            measurement_window_id: "window-001".into(),
        }
    }

    fn measurement(id: &str, kind: MeasurementKind, receipt_id: Option<&str>) -> MeasurementReceipt {
        MeasurementReceipt {
            measurement_id: id.into(),
            kind,
            scope: scope(),
            measurement_artifact_digest: format!("artifact-{id}"),
            instrument_configuration_digest: format!("instrument-{id}"),
            calibration_receipt_digest: format!("calibration-{id}"),
            unit_code: "SI-unit".into(),
            decimal_exponent: -3,
            start_tick: 100,
            end_tick: 200,
            disposition: EvidenceDisposition::Qualified,
            uncertainty_receipt_id: receipt_id.map(str::to_owned),
        }
    }

    fn uncertainty(id: &str, measurement_id: &str) -> UncertaintyReceipt {
        UncertaintyReceipt {
            receipt_id: id.into(),
            measurement_id: measurement_id.into(),
            scope: scope(),
            measurement_artifact_digest: format!("artifact-{measurement_id}"),
            instrument_configuration_digest: format!("instrument-{measurement_id}"),
            calibration_receipt_digest: format!("calibration-{measurement_id}"),
            unit_code: "SI-unit".into(),
            decimal_exponent: -3,
            coverage: CoverageRepresentation::ProbabilityInterval {
                lower_scaled: 10,
                upper_scaled: 20,
                unit_code: "SI-unit".into(),
                decimal_exponent: -3,
                coverage_probability_ppm: 950_000,
                method: CoverageMethod::FirstOrderGum,
                coverage_receipt_digest: format!("coverage-{id}"),
                convergence_receipt_digest: None,
            },
            components: vec![BudgetComponent {
                component_id: format!("component-{id}"),
                kind: ComponentKind::CalibrationReference,
                source_receipt_digest: format!("component-source-{id}"),
                dependence_group_id: format!("group-{id}"),
            }],
            dependence: vec![DependenceDeclaration {
                dependence_group_id: format!("group-{id}"),
                disposition: DependenceDisposition::IndependenceSupported {
                    independence_receipt_digest: format!("independence-{id}"),
                },
            }],
            budget_result_receipt_digest: format!("budget-{id}"),
            disposition: EvidenceDisposition::Qualified,
            valid_from_tick: 50,
            valid_through_tick: 250,
        }
    }

    fn profile() -> UncertaintyProfile {
        UncertaintyProfile {
            scope: scope(),
            evaluation_tick: 150,
            required_measurement_ids: vec!["torque-001".into()],
            measurements: vec![measurement(
                "torque-001",
                MeasurementKind::ShaftTorque,
                Some("uq-torque-001"),
            )],
            uncertainty_receipts: vec![uncertainty("uq-torque-001", "torque-001")],
            derived_results: Vec::new(),
        }
    }

    #[test]
    fn complete_measurement_uncertainty_profile_is_qualified_without_authority() {
        let report = audit_measurement_uncertainty(&profile());
        assert_eq!(report.disposition, EvidenceDisposition::Qualified);
        assert_eq!(report.checked_measurement_count, 1);
        assert_eq!(report.checked_uncertainty_receipt_count, 1);
        assert!(!report.conformity_certified);
        assert!(!report.actuation_authorized);
    }

    #[test]
    fn missing_required_measurement_is_unknown_not_zero() {
        let mut audit = profile();
        audit.measurements.clear();
        let report = audit_measurement_uncertainty(&audit);
        assert_eq!(report.disposition, EvidenceDisposition::Unknown);
        assert!(report.findings.iter().any(|finding| {
            finding.code == "REQUIRED_MEASUREMENT_MISSING"
        }));
    }

    #[test]
    fn required_measurement_without_uncertainty_receipt_is_unknown() {
        let mut audit = profile();
        audit.measurements[0].uncertainty_receipt_id = None;
        audit.uncertainty_receipts.clear();
        let report = audit_measurement_uncertainty(&audit);
        assert_eq!(report.disposition, EvidenceDisposition::Unknown);
        assert!(report.findings.iter().any(|finding| {
            finding.code == "REQUIRED_MEASUREMENT_HAS_NO_UNCERTAINTY_RECEIPT"
        }));
    }

    #[test]
    fn calibration_digest_mismatch_is_rejected() {
        let mut audit = profile();
        audit.uncertainty_receipts[0].calibration_receipt_digest = "other-calibration".into();
        let report = audit_measurement_uncertainty(&audit);
        assert_eq!(report.disposition, EvidenceDisposition::Rejected);
        assert!(report.findings.iter().any(|finding| {
            finding.code == "UNCERTAINTY_MEASUREMENT_BINDING_MISMATCH"
        }));
    }

    #[test]
    fn stale_uncertainty_receipt_is_unknown() {
        let mut audit = profile();
        audit.uncertainty_receipts[0].valid_through_tick = 149;
        let report = audit_measurement_uncertainty(&audit);
        assert_eq!(report.disposition, EvidenceDisposition::Unknown);
        assert!(report.findings.iter().any(|finding| {
            finding.code == "UNCERTAINTY_RECEIPT_STALE_OR_NOT_YET_VALID"
        }));
    }

    #[test]
    fn shared_source_group_without_dependence_decision_is_unknown() {
        let mut audit = profile();
        audit.uncertainty_receipts[0].dependence[0].disposition =
            DependenceDisposition::Unknown;
        let report = audit_measurement_uncertainty(&audit);
        assert_eq!(report.disposition, EvidenceDisposition::Unknown);
        assert!(report.findings.iter().any(|finding| {
            finding.code == "DEPENDENCE_UNRESOLVED"
        }));
    }

    #[test]
    fn dependence_group_without_declaration_is_unknown() {
        let mut audit = profile();
        audit.uncertainty_receipts[0].dependence.clear();
        let report = audit_measurement_uncertainty(&audit);
        assert_eq!(report.disposition, EvidenceDisposition::Unknown);
        assert!(report.findings.iter().any(|finding| {
            finding.code == "DEPENDENCE_DECLARATION_MISSING"
        }));
    }

    #[test]
    fn supported_joint_model_is_accepted_as_explicit_dependence_evidence() {
        let mut audit = profile();
        audit.uncertainty_receipts[0].dependence[0].disposition =
            DependenceDisposition::JointModel {
                joint_model_receipt_digest: "joint-model-receipt".into(),
            };
        let report = audit_measurement_uncertainty(&audit);
        assert_eq!(report.disposition, EvidenceDisposition::Qualified);
    }

    #[test]
    fn probabilistic_monte_carlo_interval_requires_convergence_receipt() {
        let mut audit = profile();
        if let CoverageRepresentation::ProbabilityInterval {
            method,
            convergence_receipt_digest,
            ..
        } = &mut audit.uncertainty_receipts[0].coverage
        {
            *method = CoverageMethod::MonteCarloDistribution;
            *convergence_receipt_digest = None;
        }
        let report = audit_measurement_uncertainty(&audit);
        assert_eq!(report.disposition, EvidenceDisposition::Inconclusive);
        assert!(report.findings.iter().any(|finding| {
            finding.code == "MONTE_CARLO_CONVERGENCE_RECEIPT_MISSING"
        }));
    }

    #[test]
    fn conservative_bound_remains_distinct_from_probability_interval() {
        let mut audit = profile();
        audit.uncertainty_receipts[0].coverage = CoverageRepresentation::ConservativeBound {
            lower_scaled: -10,
            upper_scaled: 10,
            unit_code: "SI-unit".into(),
            decimal_exponent: -3,
        };
        let report = audit_measurement_uncertainty(&audit);
        assert_eq!(report.disposition, EvidenceDisposition::Qualified);
        assert!(matches!(
            audit.uncertainty_receipts[0].coverage,
            CoverageRepresentation::ConservativeBound { .. }
        ));
    }

    #[test]
    fn derived_receipt_with_exact_input_identity_set_is_qualified() {
        let mut audit = profile();
        audit.required_measurement_ids.push("voltage-001".into());
        audit.measurements.push(measurement(
            "voltage-001",
            MeasurementKind::ElectricalVoltage,
            Some("uq-voltage-001"),
        ));
        audit.uncertainty_receipts
            .push(uncertainty("uq-voltage-001", "voltage-001"));
        audit.measurements.push(measurement(
            "energy-001",
            MeasurementKind::DerivedElectricalEnergy,
            Some("uq-energy-001"),
        ));
        audit.uncertainty_receipts
            .push(uncertainty("uq-energy-001", "energy-001"));
        audit.derived_results.push(DerivedResultReceipt {
            result_id: "energy-001".into(),
            output_measurement_id: "energy-001".into(),
            input_measurement_ids: vec!["torque-001".into(), "voltage-001".into()],
            input_uncertainty_receipt_ids: vec![
                "uq-torque-001".into(),
                "uq-voltage-001".into(),
            ],
            output_result_digest: "artifact-energy-001".into(),
            output_uncertainty_receipt_id: "uq-energy-001".into(),
            upstream_disposition: EvidenceDisposition::Qualified,
        });
        let report = audit_measurement_uncertainty(&audit);
        assert_eq!(report.disposition, EvidenceDisposition::Qualified);
        assert_eq!(report.checked_derived_result_count, 1);
    }

    #[test]
    fn derived_receipt_omitting_an_input_uncertainty_is_rejected() {
        let mut audit = profile();
        audit.required_measurement_ids.push("voltage-001".into());
        audit.measurements.push(measurement(
            "voltage-001",
            MeasurementKind::ElectricalVoltage,
            Some("uq-voltage-001"),
        ));
        audit.uncertainty_receipts
            .push(uncertainty("uq-voltage-001", "voltage-001"));
        audit.measurements.push(measurement(
            "energy-001",
            MeasurementKind::DerivedElectricalEnergy,
            Some("uq-energy-001"),
        ));
        audit.uncertainty_receipts
            .push(uncertainty("uq-energy-001", "energy-001"));
        audit.derived_results.push(DerivedResultReceipt {
            result_id: "energy-result-001".into(),
            output_measurement_id: "energy-001".into(),
            input_measurement_ids: vec!["torque-001".into(), "voltage-001".into()],
            input_uncertainty_receipt_ids: vec!["uq-torque-001".into()],
            output_result_digest: "energy-output-digest".into(),
            output_uncertainty_receipt_id: "uq-energy-001".into(),
            upstream_disposition: EvidenceDisposition::Qualified,
        });
        let report = audit_measurement_uncertainty(&audit);
        assert_eq!(report.disposition, EvidenceDisposition::Rejected);
        assert!(report.findings.iter().any(|finding| {
            finding.code == "DERIVED_INPUT_UNCERTAINTY_SET_MISMATCH"
        }));
    }

    #[test]
    fn upstream_inconclusive_result_cannot_be_upgraded() {
        let mut audit = profile();
        audit.uncertainty_receipts[0].disposition = EvidenceDisposition::Inconclusive;
        let report = audit_measurement_uncertainty(&audit);
        assert_eq!(report.disposition, EvidenceDisposition::Inconclusive);
    }

    #[test]
    fn duplicate_measurement_identity_is_rejected() {
        let mut audit = profile();
        audit.measurements.push(audit.measurements[0].clone());
        let report = audit_measurement_uncertainty(&audit);
        assert_eq!(report.disposition, EvidenceDisposition::Rejected);
        assert!(report.findings.iter().any(|finding| {
            finding.code == "DUPLICATE_MEASUREMENT_ID"
        }));
    }

    #[test]
    fn measurement_scope_drift_is_rejected() {
        let mut audit = profile();
        audit.measurements[0].scope.measurement_window_id = "different-window".into();
        let report = audit_measurement_uncertainty(&audit);
        assert_eq!(report.disposition, EvidenceDisposition::Rejected);
        assert!(report.findings.iter().any(|finding| {
            finding.code == "MEASUREMENT_SCOPE_MISMATCH"
        }));
    }

    #[test]
    fn probability_interval_unit_mismatch_is_rejected() {
        let mut audit = profile();
        if let CoverageRepresentation::ProbabilityInterval { unit_code, .. } =
            &mut audit.uncertainty_receipts[0].coverage
        {
            *unit_code = "other-unit".into();
        }
        let report = audit_measurement_uncertainty(&audit);
        assert_eq!(report.disposition, EvidenceDisposition::Rejected);
        assert!(report.findings.iter().any(|finding| {
            finding.code == "UNCERTAINTY_UNIT_OR_SCALE_MISMATCH"
        }));
    }

    #[test]
    fn upstream_unknown_measurement_is_never_promoted() {
        let mut audit = profile();
        audit.measurements[0].disposition = EvidenceDisposition::Unknown;
        let report = audit_measurement_uncertainty(&audit);
        assert_eq!(report.disposition, EvidenceDisposition::Unknown);
    }
}
