//! Evidence qualification for fault-survivable magnetic machines.
//!
//! The evaluator keeps three different questions separate:
//! 1. What irreversible material flux loss occurred (bulk and by mapped zone)?
//! 2. Did the rotor/component remain mechanically and thermally safe?
//! 3. Did the component deliver its preregistered degraded post-fault mission?
//!
//! Flux loss is reported, not compared with a universal percentage threshold.
//! The declared application-specific safety and degraded-mission envelope is
//! the acceptance authority. This crate does not run a motor, solve Maxwell's
//! equations, authenticate issuers, or qualify a physical component.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaultKind {
    PhaseToPhaseShortCircuit,
    ThreePhaseShortCircuit,
    InverterShootThrough,
    OverCurrent,
    CoolingLoss,
    LockedRotor,
    ThermalOverload,
    ExternalDemagnetizingField,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceClass {
    PhysicalMeasurement,
    ProductionRecord,
    Simulation,
    PublishedLiterature,
    SyntheticControl,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Disposition {
    Qualified,
    Fail,
    Indeterminate,
    NotComparable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    NotComparable,
    Fail,
    Indeterminate,
    FaultSurvivable,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Interval {
    pub lower: f64,
    pub upper: f64,
}

impl Interval {
    pub const fn new(lower: f64, upper: f64) -> Self {
        Self { lower, upper }
    }

    pub fn is_valid(self) -> bool {
        self.lower.is_finite() && self.upper.is_finite() && self.lower <= self.upper
    }

    pub fn is_nonnegative(self) -> bool {
        self.is_valid() && self.lower >= 0.0
    }

    pub fn is_positive(self) -> bool {
        self.is_valid() && self.lower > 0.0
    }

    pub fn ratio_positive(self, denominator: Self) -> Option<Self> {
        if !self.is_nonnegative() || !denominator.is_valid() || denominator.lower <= 0.0 {
            return None;
        }
        let result = Self::new(
            self.lower / denominator.upper,
            self.upper / denominator.lower,
        );
        result.is_nonnegative().then_some(result)
    }

    pub fn subtract(self, rhs: Self) -> Option<Self> {
        if !self.is_valid() || !rhs.is_valid() {
            return None;
        }
        let result = Self::new(self.lower - rhs.upper, self.upper - rhs.lower);
        result.is_valid().then_some(result)
    }

    pub fn scale(self, factor: f64) -> Option<Self> {
        if !self.is_valid() || !factor.is_finite() {
            return None;
        }
        let a = self.lower * factor;
        let b = self.upper * factor;
        let result = Self::new(a.min(b), a.max(b));
        result.is_valid().then_some(result)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComponentScope {
    pub application_id: String,
    pub component_id: String,
    pub assembly_revision_digest: String,
    pub magnet_material_lot_digest: String,
    pub rotor_geometry_digest: String,
    pub magnetization_digest: String,
    pub duty_profile_digest: String,
    pub degraded_mission_digest: String,
    pub fault_protocol_digest: String,
    pub measurement_setup_digest: String,
    pub safety_policy_digest: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FaultCaseSpec {
    pub case_id: String,
    pub fault_kind: FaultKind,
    pub minimum_peak_current_a: f64,
    pub maximum_peak_current_a: f64,
    pub minimum_rotor_speed_rpm: f64,
    pub maximum_rotor_speed_rpm: f64,
    pub minimum_magnet_temperature_c: f64,
    pub maximum_magnet_temperature_c: f64,
    pub minimum_exposure_duration_ms: f64,
    pub maximum_exposure_duration_ms: f64,
    pub minimum_degraded_mission_energy_kwh: f64,
    pub maximum_allowed_mechanical_stress_mpa: f64,
    pub minimum_mechanical_safety_factor: f64,
    pub required_magnet_zones: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QualificationSpec {
    pub campaign_id: String,
    pub scope: ComponentScope,
    pub required_fault_cases: Vec<FaultCaseSpec>,
    pub minimum_post_fault_mission_completion: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExposureStep {
    pub sequence: u32,
    pub timestamp_ms: f64,
    pub duration_ms: f64,
    pub phase_current_peak_a: Interval,
    pub rotor_speed_rpm: Interval,
    pub magnet_temperature_c: Interval,
    pub dc_bus_voltage_v: Interval,
    pub cooling_flow_l_per_min: Interval,
    pub fault_active: bool,
    pub event_receipt_id: String,
    pub event_digest: String,
    pub sensor_method_id: String,
    pub calibration_digest: String,
    pub uncertainty_record_digest: String,
    pub evidence_class: EvidenceClass,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FluxMeasurement {
    pub measurement_id: String,
    pub zone_id: String,
    pub pre_fault_flux_density_t: Interval,
    pub post_fault_flux_density_t: Interval,
    pub pre_fault_temperature_c: Interval,
    pub post_fault_temperature_c: Interval,
    pub method_id: String,
    pub instrument_id: String,
    pub calibration_digest: String,
    pub uncertainty_record_digest: String,
    pub evidence_class: EvidenceClass,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StructuralEvidence {
    pub inspection_receipt_id: String,
    pub inspection_digest: String,
    pub evidence_class: EvidenceClass,
    pub maximum_measured_stress_mpa: Interval,
    pub minimum_measured_safety_factor: Interval,
    pub retention_intact: bool,
    pub containment_intact: bool,
    pub crack_or_deformation_detected: bool,
    pub vibration_rms_mm_s: Interval,
    pub method_id: String,
    pub calibration_digest: String,
    pub uncertainty_record_digest: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DegradedMissionEvidence {
    pub receipt_id: String,
    pub receipt_digest: String,
    pub evidence_class: EvidenceClass,
    pub mission_profile_digest: String,
    pub completed_mission_energy_kwh: Interval,
    pub required_mission_energy_kwh: Interval,
    pub mission_completion_fraction: Interval,
    pub minimum_torque_margin_percent: Interval,
    pub maximum_temperature_c: Interval,
    pub method_id: String,
    pub instrument_id: String,
    pub calibration_digest: String,
    pub uncertainty_record_digest: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FaultTest {
    pub case_id: String,
    pub scope: ComponentScope,
    pub build_id: String,
    pub specimen_id: String,
    pub source_material_lot_id: String,
    pub exposure_steps: Vec<ExposureStep>,
    pub flux_measurements: Vec<FluxMeasurement>,
    pub structural: StructuralEvidence,
    pub degraded_mission: DegradedMissionEvidence,
    pub test_receipt_id: String,
    pub test_receipt_digest: String,
    pub evidence_class: EvidenceClass,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Blocker {
    InvalidSpec,
    InvalidDigest,
    MissingIdentity,
    DuplicateFaultCase,
    MissingFaultCase,
    ExtraFaultCase,
    ScopeMismatch,
    NonPhysicalFaultEvidence,
    MissingExposureTrace,
    DuplicateExposureSequence,
    NonMonotonicExposureTime,
    ExposureScopeMismatch,
    FaultEventNotRecorded,
    ExposureOutsideDeclaredEnvelope,
    MissingExposureMethod,
    InvalidSensorLineage,
    MissingFluxMeasurement,
    DuplicateFluxMeasurement,
    MissingMagnetZone,
    UnexpectedMagnetZone,
    InvalidFluxInterval,
    InvalidFluxLineage,
    NonPhysicalFluxEvidence,
    MissingStructuralEvidence,
    InvalidStructuralEvidence,
    StructuralHardFailure,
    StructuralLimitUnresolved,
    MissingDegradedMission,
    DegradedMissionScopeMismatch,
    InvalidDegradedMission,
    DegradedMissionHardFailure,
    DegradedMissionUnresolved,
    MissingMissionMethod,
    InvalidMissionLineage,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ZoneFluxLoss {
    pub zone_id: String,
    /// Irreversible flux loss percentage, interval-bounded.
    /// Negative values are retained: measurement may show flux gain within uncertainty.
    pub loss_percent: Interval,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FaultCaseEvaluation {
    pub case_id: String,
    pub decision: Decision,
    pub exposure_step_count: usize,
    pub bulk_flux_loss_percent: Option<Interval>,
    pub zone_flux_loss: Vec<ZoneFluxLoss>,
    pub degraded_mission_completion: Option<Interval>,
    pub minimum_torque_margin_percent: Option<Interval>,
    pub maximum_temperature_c: Option<Interval>,
    pub blockers: Vec<Blocker>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QualificationEvaluation {
    pub campaign_id: String,
    pub decision: Decision,
    pub fault_cases: Vec<FaultCaseEvaluation>,
    pub blockers: Vec<Blocker>,
    pub claim_ceiling: String,
}

pub fn evaluate_fault_survivability(
    spec: &QualificationSpec,
    tests: &[FaultTest],
) -> QualificationEvaluation {
    let mut blockers = validate_spec(spec);
    let mut evaluations = Vec::new();
    let mut case_ids = BTreeSet::new();
    let mut any_not_comparable = blockers.contains(&Blocker::InvalidSpec);
    let mut any_fail = false;
    let mut any_indeterminate = false;

    for test in tests {
        if !case_ids.insert(test.case_id.as_str()) {
            blockers.push(Blocker::DuplicateFaultCase);
            any_not_comparable = true;
        }
        let Some(case_spec) = spec
            .required_fault_cases
            .iter()
            .find(|item| item.case_id == test.case_id)
        else {
            blockers.push(Blocker::ExtraFaultCase);
            any_not_comparable = true;
            continue;
        };

        let result = evaluate_fault_case(spec, case_spec, test);
        match result.decision {
            Decision::NotComparable => any_not_comparable = true,
            Decision::Fail => any_fail = true,
            Decision::Indeterminate => any_indeterminate = true,
            Decision::FaultSurvivable => {}
        }
        blockers.extend(result.blockers.iter().cloned());
        evaluations.push(result);
    }

    for required in &spec.required_fault_cases {
        if !case_ids.contains(required.case_id.as_str()) {
            blockers.push(Blocker::MissingFaultCase);
            any_indeterminate = true;
        }
    }

    let decision = if any_not_comparable {
        Decision::NotComparable
    } else if any_fail {
        Decision::Fail
    } else if any_indeterminate || evaluations.len() != spec.required_fault_cases.len() {
        Decision::Indeterminate
    } else {
        Decision::FaultSurvivable
    };

    blockers.sort_by_key(|item| format!("{item:?}"));
    blockers.dedup();

    QualificationEvaluation {
        campaign_id: spec.campaign_id.clone(),
        decision,
        fault_cases: evaluations,
        blockers,
        claim_ceiling: format!(
            "{decision:?}: declared fault cases and degraded mission only; no fleet reliability, certification, life prediction or production capability claim"
        ),
    }
}

fn validate_spec(spec: &QualificationSpec) -> Vec<Blocker> {
    let mut blockers = Vec::new();
    if spec.campaign_id.trim().is_empty()
        || spec.scope.application_id.trim().is_empty()
        || spec.scope.component_id.trim().is_empty()
        || spec.required_fault_cases.is_empty()
        || !spec.minimum_post_fault_mission_completion.is_finite()
        || !(0.0..=1.0).contains(&spec.minimum_post_fault_mission_completion)
    {
        blockers.push(Blocker::InvalidSpec);
    }
    for digest in [
        &spec.scope.assembly_revision_digest,
        &spec.scope.magnet_material_lot_digest,
        &spec.scope.rotor_geometry_digest,
        &spec.scope.magnetization_digest,
        &spec.scope.duty_profile_digest,
        &spec.scope.degraded_mission_digest,
        &spec.scope.fault_protocol_digest,
        &spec.scope.measurement_setup_digest,
        &spec.scope.safety_policy_digest,
    ] {
        if !valid_digest(digest) {
            blockers.push(Blocker::InvalidDigest);
        }
    }
    let mut case_ids = BTreeSet::new();
    for case in &spec.required_fault_cases {
        if case.case_id.trim().is_empty() || !case_ids.insert(case.case_id.as_str()) {
            blockers.push(Blocker::DuplicateFaultCase);
        }
        if !case.minimum_peak_current_a.is_finite()
            || !case.maximum_peak_current_a.is_finite()
            || case.minimum_peak_current_a < 0.0
            || case.minimum_peak_current_a > case.maximum_peak_current_a
            || !case.minimum_rotor_speed_rpm.is_finite()
            || !case.maximum_rotor_speed_rpm.is_finite()
            || case.minimum_rotor_speed_rpm < 0.0
            || case.minimum_rotor_speed_rpm > case.maximum_rotor_speed_rpm
            || !case.minimum_magnet_temperature_c.is_finite()
            || !case.maximum_magnet_temperature_c.is_finite()
            || case.minimum_magnet_temperature_c > case.maximum_magnet_temperature_c
            || !case.minimum_exposure_duration_ms.is_finite()
            || !case.maximum_exposure_duration_ms.is_finite()
            || case.minimum_exposure_duration_ms < 0.0
            || case.minimum_exposure_duration_ms > case.maximum_exposure_duration_ms
            || !case.minimum_degraded_mission_energy_kwh.is_finite()
            || case.minimum_degraded_mission_energy_kwh < 0.0
            || !case.maximum_allowed_mechanical_stress_mpa.is_finite()
            || case.maximum_allowed_mechanical_stress_mpa <= 0.0
            || !case.minimum_mechanical_safety_factor.is_finite()
            || case.minimum_mechanical_safety_factor <= 0.0
            || case.required_magnet_zones.is_empty()
        {
            blockers.push(Blocker::InvalidSpec);
        }
        let mut zones = BTreeSet::new();
        for zone in &case.required_magnet_zones {
            if zone.trim().is_empty() || !zones.insert(zone.as_str()) {
                blockers.push(Blocker::InvalidSpec);
            }
        }
    }
    blockers
}

fn evaluate_fault_case(
    spec: &QualificationSpec,
    case_spec: &FaultCaseSpec,
    test: &FaultTest,
) -> FaultCaseEvaluation {
    let mut blockers = Vec::new();
    let mut not_comparable = false;
    let mut hard_failure = false;
    let mut unresolved = false;

    if test.case_id != case_spec.case_id || test.scope != spec.scope {
        blockers.push(Blocker::ScopeMismatch);
        not_comparable = true;
    }
    if test.build_id.trim().is_empty()
        || test.specimen_id.trim().is_empty()
        || test.source_material_lot_id.trim().is_empty()
        || test.test_receipt_id.trim().is_empty()
    {
        blockers.push(Blocker::MissingIdentity);
        unresolved = true;
    }
    if test.evidence_class != EvidenceClass::PhysicalMeasurement {
        blockers.push(Blocker::NonPhysicalFaultEvidence);
        unresolved = true;
    }
    if !valid_digest(&test.test_receipt_digest) {
        blockers.push(Blocker::InvalidDigest);
        unresolved = true;
    }

    let mut exposure_steps = test.exposure_steps.iter().collect::<Vec<_>>();
    exposure_steps.sort_by_key(|step| step.sequence);
    if exposure_steps.is_empty() {
        blockers.push(Blocker::MissingExposureTrace);
        unresolved = true;
    }
    let mut sequence_ids = BTreeSet::new();
    let mut previous_time = None;
    let mut saw_fault = false;
    let mut total_duration_ms = 0.0;
    for step in exposure_steps {
        if !sequence_ids.insert(step.sequence) {
            blockers.push(Blocker::DuplicateExposureSequence);
            unresolved = true;
        }
        if previous_time.is_some_and(|time| step.timestamp_ms < time) {
            blockers.push(Blocker::NonMonotonicExposureTime);
            unresolved = true;
        }
        previous_time = Some(step.timestamp_ms);
        if !step.timestamp_ms.is_finite()
            || !step.duration_ms.is_finite()
            || step.duration_ms < 0.0
            || !step.phase_current_peak_a.is_nonnegative()
            || !step.rotor_speed_rpm.is_nonnegative()
            || !step.magnet_temperature_c.is_valid()
            || !step.dc_bus_voltage_v.is_nonnegative()
            || !step.cooling_flow_l_per_min.is_nonnegative()
        {
            blockers.push(Blocker::ExposureScopeMismatch);
            unresolved = true;
        }
        if step.fault_active {
            saw_fault = true;
            if step.phase_current_peak_a.upper < case_spec.minimum_peak_current_a
                || step.phase_current_peak_a.lower > case_spec.maximum_peak_current_a
                || step.rotor_speed_rpm.upper < case_spec.minimum_rotor_speed_rpm
                || step.rotor_speed_rpm.lower > case_spec.maximum_rotor_speed_rpm
                || step.magnet_temperature_c.upper < case_spec.minimum_magnet_temperature_c
                || step.magnet_temperature_c.lower > case_spec.maximum_magnet_temperature_c
            {
                blockers.push(Blocker::ExposureOutsideDeclaredEnvelope);
                not_comparable = true;
            }
            total_duration_ms += step.duration_ms;
        }
        if step.event_receipt_id.trim().is_empty() {
            blockers.push(Blocker::FaultEventNotRecorded);
            unresolved = true;
        }
        if step.sensor_method_id.trim().is_empty() {
            blockers.push(Blocker::MissingExposureMethod);
            unresolved = true;
        }
        if !valid_digest(&step.event_digest)
            || !valid_digest(&step.calibration_digest)
            || !valid_digest(&step.uncertainty_record_digest)
        {
            blockers.push(Blocker::InvalidSensorLineage);
            unresolved = true;
        }
        if step.evidence_class != EvidenceClass::PhysicalMeasurement {
            blockers.push(Blocker::NonPhysicalFaultEvidence);
            unresolved = true;
        }
    }
    if !saw_fault
        || total_duration_ms < case_spec.minimum_exposure_duration_ms
        || total_duration_ms > case_spec.maximum_exposure_duration_ms
    {
        blockers.push(Blocker::ExposureScopeMismatch);
        unresolved = true;
    }

    let required_zones = case_spec
        .required_magnet_zones
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut flux_by_zone = BTreeMap::new();
    for measurement in &test.flux_measurements {
        if flux_by_zone
            .insert(measurement.zone_id.as_str(), measurement)
            .is_some()
        {
            blockers.push(Blocker::DuplicateFluxMeasurement);
            unresolved = true;
            continue;
        }
        if !required_zones.contains(measurement.zone_id.as_str()) {
            blockers.push(Blocker::UnexpectedMagnetZone);
            not_comparable = true;
            continue;
        }
        if !measurement.pre_fault_flux_density_t.is_positive()
            || !measurement.post_fault_flux_density_t.is_nonnegative()
            || !measurement.pre_fault_temperature_c.is_valid()
            || !measurement.post_fault_temperature_c.is_valid()
        {
            blockers.push(Blocker::InvalidFluxInterval);
            unresolved = true;
        }
        if measurement.method_id.trim().is_empty() || measurement.instrument_id.trim().is_empty() {
            blockers.push(Blocker::InvalidFluxLineage);
            unresolved = true;
        }
        if !valid_digest(&measurement.calibration_digest)
            || !valid_digest(&measurement.uncertainty_record_digest)
        {
            blockers.push(Blocker::InvalidFluxLineage);
            unresolved = true;
        }
        if measurement.evidence_class != EvidenceClass::PhysicalMeasurement {
            blockers.push(Blocker::NonPhysicalFluxEvidence);
            unresolved = true;
        }
    }
    let missing_zones = required_zones
        .iter()
        .filter(|zone| !flux_by_zone.contains_key(**zone))
        .count();
    if missing_zones > 0 {
        blockers.push(Blocker::MissingMagnetZone);
        unresolved = true;
    }

    let zone_flux_loss = test
        .flux_measurements
        .iter()
        .filter_map(|measurement| calculate_flux_loss(measurement).map(|loss| ZoneFluxLoss {
            zone_id: measurement.zone_id.clone(),
            loss_percent: loss,
        }))
        .collect::<Vec<_>>();
    let bulk_flux_loss_percent = if zone_flux_loss.is_empty() {
        None
    } else {
        let total = zone_flux_loss.iter().fold(Interval::new(0.0, 0.0), |sum, zone| {
            Interval::new(sum.lower + zone.loss_percent.lower, sum.upper + zone.loss_percent.upper)
        });
        total.scale(1.0 / zone_flux_loss.len() as f64)
    };

    let structural = &test.structural;
    if structural.inspection_receipt_id.trim().is_empty() {
        blockers.push(Blocker::MissingStructuralEvidence);
        unresolved = true;
    }
    if !valid_digest(&structural.inspection_digest)
        || !valid_digest(&structural.calibration_digest)
        || !valid_digest(&structural.uncertainty_record_digest)
        || structural.method_id.trim().is_empty()
        || !structural.maximum_measured_stress_mpa.is_nonnegative()
        || !structural.minimum_measured_safety_factor.is_nonnegative()
        || !structural.vibration_rms_mm_s.is_nonnegative()
        || structural.evidence_class != EvidenceClass::PhysicalMeasurement
    {
        blockers.push(Blocker::InvalidStructuralEvidence);
        unresolved = true;
    }
    if !structural.retention_intact
        || !structural.containment_intact
        || structural.crack_or_deformation_detected
        || structural.maximum_measured_stress_mpa.lower
            > case_spec.maximum_allowed_mechanical_stress_mpa
        || structural.minimum_measured_safety_factor.upper
            < case_spec.minimum_mechanical_safety_factor
    {
        blockers.push(Blocker::StructuralHardFailure);
        hard_failure = true;
    } else if structural.maximum_measured_stress_mpa.upper
        > case_spec.maximum_allowed_mechanical_stress_mpa
        || structural.minimum_measured_safety_factor.lower
            < case_spec.minimum_mechanical_safety_factor
    {
        blockers.push(Blocker::StructuralLimitUnresolved);
        unresolved = true;
    }

    let mission = &test.degraded_mission;
    if mission.receipt_id.trim().is_empty() || mission.method_id.trim().is_empty() {
        blockers.push(Blocker::MissingDegradedMission);
        unresolved = true;
    }
    if mission.mission_profile_digest != spec.scope.degraded_mission_digest {
        blockers.push(Blocker::DegradedMissionScopeMismatch);
        not_comparable = true;
    }
    if !valid_digest(&mission.receipt_digest)
        || !valid_digest(&mission.mission_profile_digest)
        || !valid_digest(&mission.calibration_digest)
        || !valid_digest(&mission.uncertainty_record_digest)
        || mission.method_id.trim().is_empty()
        || mission.instrument_id.trim().is_empty()
        || mission.evidence_class != EvidenceClass::PhysicalMeasurement
        || !mission.completed_mission_energy_kwh.is_nonnegative()
        || !mission.required_mission_energy_kwh.is_positive()
        || !mission.mission_completion_fraction.is_nonnegative()
        || !mission.minimum_torque_margin_percent.is_valid()
        || !mission.maximum_temperature_c.is_valid()
    {
        blockers.push(Blocker::InvalidDegradedMission);
        unresolved = true;
    }
    if mission.completed_mission_energy_kwh.upper
        < case_spec.minimum_degraded_mission_energy_kwh
        || mission.mission_completion_fraction.upper
            < spec.minimum_post_fault_mission_completion
    {
        blockers.push(Blocker::DegradedMissionHardFailure);
        hard_failure = true;
    } else if mission.completed_mission_energy_kwh.lower
        < case_spec.minimum_degraded_mission_energy_kwh
        || mission.mission_completion_fraction.lower
            < spec.minimum_post_fault_mission_completion
    {
        blockers.push(Blocker::DegradedMissionUnresolved);
        unresolved = true;
    }
    if !valid_digest(&mission.calibration_digest)
        || !valid_digest(&mission.uncertainty_record_digest)
    {
        blockers.push(Blocker::InvalidMissionLineage);
        unresolved = true;
    }

    let decision = if not_comparable {
        Decision::NotComparable
    } else if hard_failure {
        Decision::Fail
    } else if unresolved || !blockers.is_empty() {
        Decision::Indeterminate
    } else {
        Decision::FaultSurvivable
    };

    let total_steps = test.exposure_steps.len();
    FaultCaseEvaluation {
        case_id: case_spec.case_id.clone(),
        decision,
        exposure_step_count: total_steps,
        bulk_flux_loss_percent,
        zone_flux_loss,
        degraded_mission_completion: Some(mission.mission_completion_fraction),
        minimum_torque_margin_percent: Some(mission.minimum_torque_margin_percent),
        maximum_temperature_c: Some(mission.maximum_temperature_c),
        blockers: dedupe(blockers),
    }
}

fn calculate_flux_loss(measurement: &FluxMeasurement) -> Option<Interval> {
    let retained = measurement
        .post_fault_flux_density_t
        .ratio_positive(measurement.pre_fault_flux_density_t)?;
    Interval::new(1.0, 1.0)
        .subtract(retained)?
        .scale(100.0)
}

fn valid_digest(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn dedupe(mut blockers: Vec<Blocker>) -> Vec<Blocker> {
    blockers.sort_by_key(|item| format!("{item:?}"));
    blockers.dedup();
    blockers
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(ch: char) -> String {
        format!("sha256:{}", ch.to_string().repeat(64))
    }

    fn scope() -> ComponentScope {
        ComponentScope {
            application_id: "HEAVY-TRACTION-260KW".to_string(),
            component_id: "ROTOR-ASSEMBLY-001".to_string(),
            assembly_revision_digest: digest('a'),
            magnet_material_lot_digest: digest('b'),
            rotor_geometry_digest: digest('c'),
            magnetization_digest: digest('d'),
            duty_profile_digest: digest('e'),
            degraded_mission_digest: digest('f'),
            fault_protocol_digest: digest('1'),
            measurement_setup_digest: digest('2'),
            safety_policy_digest: digest('3'),
        }
    }

    fn spec() -> QualificationSpec {
        QualificationSpec {
            campaign_id: "FAULT-SURVIVABILITY-001".to_string(),
            scope: scope(),
            required_fault_cases: vec![
                FaultCaseSpec {
                    case_id: "HOT-THREE-PHASE-SHORT".to_string(),
                    fault_kind: FaultKind::ThreePhaseShortCircuit,
                    minimum_peak_current_a: 900.0,
                    maximum_peak_current_a: 1400.0,
                    minimum_rotor_speed_rpm: 1000.0,
                    maximum_rotor_speed_rpm: 6000.0,
                    minimum_magnet_temperature_c: 100.0,
                    maximum_magnet_temperature_c: 180.0,
                    minimum_exposure_duration_ms: 100.0,
                    maximum_exposure_duration_ms: 500.0,
                    minimum_degraded_mission_energy_kwh: 10.0,
                    maximum_allowed_mechanical_stress_mpa: 500.0,
                    minimum_mechanical_safety_factor: 1.5,
                    required_magnet_zones: vec!["north-pole".to_string(), "south-pole".to_string()],
                },
            ],
            minimum_post_fault_mission_completion: 0.80,
        }
    }

    fn exposure(sequence: u32, timestamp: f64, fault: bool) -> ExposureStep {
        ExposureStep {
            sequence,
            timestamp_ms: timestamp,
            duration_ms: if fault { 200.0 } else { 50.0 },
            phase_current_peak_a: Interval::new(1000.0, 1100.0),
            rotor_speed_rpm: Interval::new(2500.0, 2600.0),
            magnet_temperature_c: Interval::new(125.0, 135.0),
            dc_bus_voltage_v: Interval::new(600.0, 620.0),
            cooling_flow_l_per_min: Interval::new(5.0, 6.0),
            fault_active: fault,
            event_receipt_id: format!("EVENT-{sequence}"),
            event_digest: digest('4'),
            sensor_method_id: "CURRENT-TEMP-SPEED-METHOD".to_string(),
            calibration_digest: digest('5'),
            uncertainty_record_digest: digest('6'),
            evidence_class: EvidenceClass::PhysicalMeasurement,
        }
    }

    fn flux(zone: &str, pre: f64, post: f64, suffix: char) -> FluxMeasurement {
        FluxMeasurement {
            measurement_id: format!("FLUX-{zone}"),
            zone_id: zone.to_string(),
            pre_fault_flux_density_t: Interval::new(pre - 0.01, pre + 0.01),
            post_fault_flux_density_t: Interval::new(post - 0.01, post + 0.01),
            pre_fault_temperature_c: Interval::new(25.0, 27.0),
            post_fault_temperature_c: Interval::new(25.0, 27.0),
            method_id: "HALL-MAP-METHOD".to_string(),
            instrument_id: "HALL-PROBE-001".to_string(),
            calibration_digest: digest(suffix),
            uncertainty_record_digest: digest('7'),
            evidence_class: EvidenceClass::PhysicalMeasurement,
        }
    }

    fn structural() -> StructuralEvidence {
        StructuralEvidence {
            inspection_receipt_id: "INSPECTION-001".to_string(),
            inspection_digest: digest('8'),
            evidence_class: EvidenceClass::PhysicalMeasurement,
            maximum_measured_stress_mpa: Interval::new(300.0, 350.0),
            minimum_measured_safety_factor: Interval::new(1.8, 2.0),
            retention_intact: true,
            containment_intact: true,
            crack_or_deformation_detected: false,
            vibration_rms_mm_s: Interval::new(1.0, 1.5),
            method_id: "STRUCTURAL-INSPECTION-METHOD".to_string(),
            calibration_digest: digest('9'),
            uncertainty_record_digest: digest('a'),
        }
    }

    fn mission() -> DegradedMissionEvidence {
        DegradedMissionEvidence {
            receipt_id: "DEGRADED-MISSION-001".to_string(),
            receipt_digest: digest('b'),
            evidence_class: EvidenceClass::PhysicalMeasurement,
            mission_profile_digest: digest('f'),
            completed_mission_energy_kwh: Interval::new(11.0, 12.0),
            required_mission_energy_kwh: Interval::new(10.0, 10.0),
            mission_completion_fraction: Interval::new(0.85, 0.90),
            minimum_torque_margin_percent: Interval::new(8.0, 10.0),
            maximum_temperature_c: Interval::new(150.0, 160.0),
            method_id: "SHAFT-ENERGY-METHOD".to_string(),
            instrument_id: "DYNAMOMETER-001".to_string(),
            calibration_digest: digest('c'),
            uncertainty_record_digest: digest('d'),
        }
    }

    fn test_case() -> FaultTest {
        FaultTest {
            case_id: "HOT-THREE-PHASE-SHORT".to_string(),
            scope: scope(),
            build_id: "BUILD-001".to_string(),
            specimen_id: "SPECIMEN-001".to_string(),
            source_material_lot_id: "MAGNET-LOT-001".to_string(),
            exposure_steps: vec![exposure(0, 0.0, false), exposure(1, 50.0, true), exposure(2, 250.0, false)],
            flux_measurements: vec![
                flux("north-pole", 1.2, 1.1, 'e'),
                flux("south-pole", 1.2, 1.15, 'f'),
            ],
            structural: structural(),
            degraded_mission: mission(),
            test_receipt_id: "FAULT-TEST-001".to_string(),
            test_receipt_digest: digest('1'),
            evidence_class: EvidenceClass::PhysicalMeasurement,
        }
    }

    fn run(test: FaultTest) -> QualificationEvaluation {
        evaluate_fault_survivability(&spec(), &[test])
    }

    #[test]
    fn complete_physical_fault_trace_and_degraded_mission_are_survivable() {
        let result = run(test_case());
        assert_eq!(result.decision, Decision::FaultSurvivable);
        assert_eq!(result.fault_cases.len(), 1);
        assert!(result.fault_cases[0].bulk_flux_loss_percent.is_some());
    }

    #[test]
    fn flux_loss_is_reported_without_a_universal_percentage_fail_limit() {
        let mut test = test_case();
        test.flux_measurements = vec![
            flux("north-pole", 1.2, 0.85, 'e'),
            flux("south-pole", 1.2, 0.85, 'f'),
        ];
        let result = run(test);
        assert_eq!(result.decision, Decision::FaultSurvivable);
        assert!(result.fault_cases[0].bulk_flux_loss_percent.unwrap().lower > 20.0);
    }

    #[test]
    fn low_bulk_loss_cannot_hide_missing_local_magnet_zone() {
        let mut test = test_case();
        test.flux_measurements.pop();
        let result = run(test);
        assert_eq!(result.decision, Decision::Indeterminate);
        assert!(result.blockers.contains(&Blocker::MissingMagnetZone));
    }

    #[test]
    fn local_flux_map_cannot_add_an_unregistered_zone() {
        let mut test = test_case();
        test.flux_measurements.push(flux("unregistered-region", 1.2, 1.1, 'e'));
        let result = run(test);
        assert_eq!(result.decision, Decision::NotComparable);
        assert!(result.blockers.contains(&Blocker::UnexpectedMagnetZone));
    }

    #[test]
    fn physical_post_fault_containment_failure_is_a_hard_fail() {
        let mut test = test_case();
        test.structural.containment_intact = false;
        let result = run(test);
        assert_eq!(result.decision, Decision::Fail);
        assert!(result.blockers.contains(&Blocker::StructuralHardFailure));
    }

    #[test]
    fn structural_uncertainty_crossing_limit_is_indeterminate() {
        let mut test = test_case();
        test.structural.maximum_measured_stress_mpa = Interval::new(490.0, 510.0);
        let result = run(test);
        assert_eq!(result.decision, Decision::Indeterminate);
        assert!(result.blockers.contains(&Blocker::StructuralLimitUnresolved));
    }

    #[test]
    fn degraded_mission_failure_is_a_hard_fail() {
        let mut test = test_case();
        test.degraded_mission.mission_completion_fraction = Interval::new(0.60, 0.70);
        let result = run(test);
        assert_eq!(result.decision, Decision::Fail);
        assert!(result.blockers.contains(&Blocker::DegradedMissionHardFailure));
    }

    #[test]
    fn degraded_mission_interval_crossing_minimum_is_indeterminate() {
        let mut test = test_case();
        test.degraded_mission.mission_completion_fraction = Interval::new(0.75, 0.85);
        let result = run(test);
        assert_eq!(result.decision, Decision::Indeterminate);
        assert!(result.blockers.contains(&Blocker::DegradedMissionUnresolved));
    }

    #[test]
    fn missing_fault_case_cannot_be_promoted() {
        let mut specification = spec();
        specification.required_fault_cases.push(FaultCaseSpec {
            case_id: "COOLING-LOSS".to_string(),
            fault_kind: FaultKind::CoolingLoss,
            minimum_peak_current_a: 0.0,
            maximum_peak_current_a: 500.0,
            minimum_rotor_speed_rpm: 0.0,
            maximum_rotor_speed_rpm: 1000.0,
            minimum_magnet_temperature_c: 20.0,
            maximum_magnet_temperature_c: 180.0,
            minimum_exposure_duration_ms: 50.0,
            maximum_exposure_duration_ms: 100.0,
            minimum_degraded_mission_energy_kwh: 1.0,
            maximum_allowed_mechanical_stress_mpa: 500.0,
            minimum_mechanical_safety_factor: 1.5,
            required_magnet_zones: vec!["north-pole".to_string()],
        });
        let result = evaluate_fault_survivability(&specification, &[test_case()]);
        assert_eq!(result.decision, Decision::Indeterminate);
        assert!(result.blockers.contains(&Blocker::MissingFaultCase));
    }

    #[test]
    fn undeclared_fault_case_is_not_comparable() {
        let mut test = test_case();
        test.case_id = "UNREGISTERED-FAULT".to_string();
        let result = run(test);
        assert_eq!(result.decision, Decision::NotComparable);
        assert!(result.blockers.contains(&Blocker::ExtraFaultCase));
    }

    #[test]
    fn duplicate_fault_case_is_not_silently_overwritten() {
        let test = test_case();
        let result = evaluate_fault_survivability(&spec(), &[test.clone(), test]);
        assert_eq!(result.decision, Decision::NotComparable);
        assert!(result.blockers.contains(&Blocker::DuplicateFaultCase));
    }

    #[test]
    fn missing_fault_current_event_receipt_is_indeterminate() {
        let mut test = test_case();
        test.exposure_steps[1].event_receipt_id.clear();
        let result = run(test);
        assert_eq!(result.decision, Decision::Indeterminate);
        assert!(result.blockers.contains(&Blocker::FaultEventNotRecorded));
    }

    #[test]
    fn nonmonotonic_fault_trace_is_indeterminate() {
        let mut test = test_case();
        test.exposure_steps[2].timestamp_ms = 10.0;
        let result = run(test);
        assert_eq!(result.decision, Decision::Indeterminate);
        assert!(result.blockers.contains(&Blocker::NonMonotonicExposureTime));
    }

    #[test]
    fn duplicated_exposure_sequence_is_indeterminate() {
        let mut test = test_case();
        test.exposure_steps[2].sequence = 1;
        let result = run(test);
        assert_eq!(result.decision, Decision::Indeterminate);
        assert!(result.blockers.contains(&Blocker::DuplicateExposureSequence));
    }

    #[test]
    fn fault_waveform_outside_declared_application_envelope_is_not_comparable() {
        let mut test = test_case();
        test.exposure_steps[1].phase_current_peak_a = Interval::new(2000.0, 2100.0);
        let result = run(test);
        assert_eq!(result.decision, Decision::NotComparable);
        assert!(result.blockers.contains(&Blocker::ExposureOutsideDeclaredEnvelope));
    }

    #[test]
    fn simulation_only_fault_test_cannot_claim_component_survivability() {
        let mut test = test_case();
        test.evidence_class = EvidenceClass::Simulation;
        let result = run(test);
        assert_eq!(result.decision, Decision::Indeterminate);
        assert!(result.blockers.contains(&Blocker::NonPhysicalFaultEvidence));
    }

    #[test]
    fn synthetic_flux_map_cannot_be_used_as_physical_evidence() {
        let mut test = test_case();
        test.flux_measurements[0].evidence_class = EvidenceClass::SyntheticControl;
        let result = run(test);
        assert_eq!(result.decision, Decision::Indeterminate);
        assert!(result.blockers.contains(&Blocker::NonPhysicalFluxEvidence));
    }

    #[test]
    fn scope_mismatch_is_not_comparable() {
        let mut test = test_case();
        test.scope.rotor_geometry_digest = digest('0');
        let result = run(test);
        assert_eq!(result.decision, Decision::NotComparable);
        assert!(result.blockers.contains(&Blocker::ScopeMismatch));
    }

    #[test]
    fn invalid_flux_measurement_cannot_create_negative_pre_fault_flux() {
        let mut test = test_case();
        test.flux_measurements[0].pre_fault_flux_density_t = Interval::new(-0.1, 0.0);
        let result = run(test);
        assert_eq!(result.decision, Decision::Indeterminate);
        assert!(result.blockers.contains(&Blocker::InvalidFluxInterval));
    }

    #[test]
    fn flux_loss_interval_preserves_uncertainty() {
        let measurement = flux("north-pole", 1.2, 1.1, 'e');
        let loss = calculate_flux_loss(&measurement).unwrap();
        assert!(loss.lower < loss.upper);
    }

    #[test]
    fn cooling_state_is_recorded_and_part_of_fault_exposure() {
        let mut test = test_case();
        test.exposure_steps[1].cooling_flow_l_per_min = Interval::new(0.0, 0.1);
        let result = run(test);
        assert_eq!(result.decision, Decision::FaultSurvivable);
    }

    #[test]
    fn physical_cracking_is_a_hard_failure() {
        let mut test = test_case();
        test.structural.crack_or_deformation_detected = true;
        let result = run(test);
        assert_eq!(result.decision, Decision::Fail);
        assert!(result.blockers.contains(&Blocker::StructuralHardFailure));
    }

    #[test]
    fn a_fault_test_must_use_physical_post_fault_mission_measurements() {
        let mut test = test_case();
        test.degraded_mission.evidence_class = EvidenceClass::Simulation;
        let result = run(test);
        assert_eq!(result.decision, Decision::Indeterminate);
        assert!(result.blockers.contains(&Blocker::InvalidDegradedMission));
    }
}
