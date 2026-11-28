//! Evidence-bound aggregation of ordered magnetic-machine mission profiles.
//!
//! This crate consumes physical measurement intervals. It is not a motor,
//! electromagnetic, thermal, inverter, structural, or life-cycle solver.

use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperatingQuadrant {
    Motoring,
    Regenerative,
    Standstill,
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Interval {
    pub lower: f64,
    pub upper: f64,
}

impl Interval {
    pub const fn new(lower: f64, upper: f64) -> Self {
        Self { lower, upper }
    }

    pub const fn zero() -> Self {
        Self::new(0.0, 0.0)
    }

    pub fn is_valid(self) -> bool {
        self.lower.is_finite() && self.upper.is_finite() && self.lower <= self.upper
    }

    pub fn is_nonnegative(self) -> bool {
        self.is_valid() && self.lower >= 0.0
    }

    pub fn add(self, rhs: Self) -> Self {
        Self::new(self.lower + rhs.lower, self.upper + rhs.upper)
    }

    pub fn subtract(self, rhs: Self) -> Self {
        Self::new(self.lower - rhs.upper, self.upper - rhs.lower)
    }

    /// Conservative interval division for a strictly positive denominator.
    /// This does not assume statistical independence between inputs.
    pub fn divide_positive(self, denominator: Self) -> Option<Self> {
        if !self.is_valid() || !denominator.is_valid() || denominator.lower <= 0.0 {
            return None;
        }
        Some(Self::new(
            self.lower / denominator.upper,
            self.upper / denominator.lower,
        ))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SegmentSpec {
    pub segment_id: String,
    pub sequence_index: u32,
    pub duration_s: f64,
    pub quadrant: OperatingQuadrant,
    pub minimum_torque_tracking: f64,
    pub minimum_speed_tracking: f64,
    pub maximum_winding_temperature_c: f64,
    pub maximum_magnet_temperature_c: f64,
    pub minimum_demagnetization_margin: f64,
    pub minimum_retention_margin: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DutyCycleSpec {
    pub profile_id: String,
    pub component_id: String,
    pub profile_digest: String,
    pub system_boundary_digest: String,
    pub measurement_setup_digest: String,
    pub initial_thermal_state_digest: String,
    pub segments: Vec<SegmentSpec>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnergyFlows {
    /// DC energy drawn by the traction drive, excluding auxiliary loads.
    pub dc_input_j: Option<Interval>,
    /// Mechanical energy delivered at the shaft during motoring.
    pub shaft_output_j: Option<Interval>,
    /// Mechanical energy absorbed at the shaft during regeneration.
    pub shaft_input_j: Option<Interval>,
    /// DC energy returned by the drive during regeneration.
    pub dc_output_j: Option<Interval>,
    /// Cooling, field-excitation, controls, and other declared auxiliaries.
    pub auxiliary_dc_input_j: Interval,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SegmentEvidence {
    pub segment_id: String,
    pub sequence_index: u32,
    pub profile_digest: String,
    pub component_id: String,
    pub system_boundary_digest: String,
    pub measurement_setup_digest: String,
    pub evidence_id: String,
    pub receipt_id: String,
    pub evidence_digest: String,
    pub receipt_digest: String,
    pub calibration_digest: String,
    pub uncertainty_record_digest: String,
    pub method_id: String,
    pub evidence_class: EvidenceClass,
    pub thermal_start_state_digest: String,
    pub thermal_end_state_digest: String,
    pub torque_tracking: Interval,
    pub speed_tracking: Interval,
    pub maximum_winding_temperature_c: Interval,
    pub maximum_magnet_temperature_c: Interval,
    pub minimum_demagnetization_margin: Interval,
    pub minimum_retention_margin: Interval,
    pub energy: EnergyFlows,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Blocker {
    InvalidSpec,
    InvalidDigest,
    DuplicateExpectedSegment,
    InvalidSegmentDefinition,
    MissingSegment,
    ExtraSegment,
    DuplicateSegment,
    DuplicateEvidenceId,
    DuplicateReceiptId,
    SegmentOrderMismatch,
    ProfileMismatch,
    ComponentMismatch,
    SystemBoundaryMismatch,
    MeasurementSetupMismatch,
    MissingEvidenceIdentity,
    NonPhysicalEvidence,
    MissingMeasurementMethod,
    ThermalInitialStateMismatch,
    ThermalStateDiscontinuity,
    InvalidInterval,
    TorqueShortfall,
    TorqueUnresolved,
    SpeedShortfall,
    SpeedUnresolved,
    WindingTemperatureExceeded,
    WindingTemperatureUnresolved,
    MagnetTemperatureExceeded,
    MagnetTemperatureUnresolved,
    DemagnetizationMarginFailed,
    DemagnetizationMarginUnresolved,
    RetentionMarginFailed,
    RetentionMarginUnresolved,
    InvalidEnergyInterval,
    InvalidEnergyQuadrant,
    NonPositiveEnergyDenominator,
    EnergyEfficiencyPhysicallyInconsistent,
    EnergyEfficiencyUnresolved,
    OverflowInEnergyArithmetic,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SegmentEvaluation {
    pub segment_id: String,
    pub evidence_id: Option<String>,
    pub disposition: Disposition,
    pub blockers: Vec<Blocker>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnergySummary {
    pub motoring_dc_input_j: Option<Interval>,
    pub motoring_shaft_output_j: Option<Interval>,
    pub regenerative_shaft_input_j: Option<Interval>,
    pub regenerative_dc_output_j: Option<Interval>,
    pub auxiliary_dc_input_j: Interval,
    pub net_dc_input_j: Interval,
    pub motoring_efficiency: Option<Interval>,
    pub regenerative_efficiency: Option<Interval>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DutyProfileEvaluation {
    pub profile_id: String,
    pub disposition: Disposition,
    pub expected_segment_count: usize,
    pub observed_segment_count: usize,
    pub segment_evaluations: Vec<SegmentEvaluation>,
    pub energy: Option<EnergySummary>,
    pub blockers: Vec<Blocker>,
    pub claim_ceiling: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct EnergyAccumulator {
    motoring_dc_input: Interval,
    motoring_shaft_output: Interval,
    regenerative_shaft_input: Interval,
    regenerative_dc_output: Interval,
    auxiliary_dc_input: Interval,
    has_motoring: bool,
    has_regeneration: bool,
}

impl Default for EnergyAccumulator {
    fn default() -> Self {
        Self {
            motoring_dc_input: Interval::zero(),
            motoring_shaft_output: Interval::zero(),
            regenerative_shaft_input: Interval::zero(),
            regenerative_dc_output: Interval::zero(),
            auxiliary_dc_input: Interval::zero(),
            has_motoring: false,
            has_regeneration: false,
        }
    }
}

impl EnergyAccumulator {
    fn add_segment(&mut self, quadrant: OperatingQuadrant, flows: EnergyFlows) {
        self.auxiliary_dc_input = self
            .auxiliary_dc_input
            .add(flows.auxiliary_dc_input_j);
        match quadrant {
            OperatingQuadrant::Motoring => {
                self.has_motoring = true;
                self.motoring_dc_input = self
                    .motoring_dc_input
                    .add(flows.dc_input_j.expect("validated motoring input"));
                self.motoring_shaft_output = self
                    .motoring_shaft_output
                    .add(flows.shaft_output_j.expect("validated motoring output"));
            }
            OperatingQuadrant::Regenerative => {
                self.has_regeneration = true;
                self.regenerative_shaft_input = self
                    .regenerative_shaft_input
                    .add(flows.shaft_input_j.expect("validated regenerative input"));
                self.regenerative_dc_output = self
                    .regenerative_dc_output
                    .add(flows.dc_output_j.expect("validated regenerative output"));
            }
            OperatingQuadrant::Standstill => {}
        }
    }

    fn summarize(self, blockers: &mut Vec<Blocker>) -> EnergySummary {
        let totals_valid = [
            self.motoring_dc_input,
            self.motoring_shaft_output,
            self.regenerative_shaft_input,
            self.regenerative_dc_output,
            self.auxiliary_dc_input,
        ]
        .into_iter()
        .all(Interval::is_valid);
        if !totals_valid {
            blockers.push(Blocker::OverflowInEnergyArithmetic);
        }

        let motoring_inputs_valid =
            self.motoring_dc_input.is_valid() && self.motoring_shaft_output.is_valid();
        let regenerative_inputs_valid =
            self.regenerative_shaft_input.is_valid() && self.regenerative_dc_output.is_valid();

        let motoring_efficiency = if self.has_motoring && motoring_inputs_valid {
            self.motoring_shaft_output
                .divide_positive(self.motoring_dc_input)
        } else {
            None
        };
        let regenerative_efficiency = if self.has_regeneration && regenerative_inputs_valid {
            self.regenerative_dc_output
                .divide_positive(self.regenerative_shaft_input)
        } else {
            None
        };

        for efficiency in [motoring_efficiency, regenerative_efficiency]
            .into_iter()
            .flatten()
        {
            if !efficiency.is_valid() {
                blockers.push(Blocker::OverflowInEnergyArithmetic);
            } else if efficiency.lower > 1.0 {
                blockers.push(Blocker::EnergyEfficiencyPhysicallyInconsistent);
            } else if efficiency.upper > 1.0 {
                blockers.push(Blocker::EnergyEfficiencyUnresolved);
            }
        }

        if (self.has_motoring
            && self.motoring_dc_input.is_valid()
            && self.motoring_shaft_output.is_valid()
            && motoring_efficiency.is_none())
            || (self.has_regeneration
                && self.regenerative_shaft_input.is_valid()
                && self.regenerative_dc_output.is_valid()
                && regenerative_efficiency.is_none())
        {
            blockers.push(Blocker::NonPositiveEnergyDenominator);
        }

        let net_dc_input_j = self
            .motoring_dc_input
            .add(self.auxiliary_dc_input)
            .subtract(self.regenerative_dc_output);
        if !net_dc_input_j.is_valid() {
            blockers.push(Blocker::OverflowInEnergyArithmetic);
        }

        EnergySummary {
            motoring_dc_input_j: self.has_motoring.then_some(self.motoring_dc_input),
            motoring_shaft_output_j: self
                .has_motoring
                .then_some(self.motoring_shaft_output),
            regenerative_shaft_input_j: self
                .has_regeneration
                .then_some(self.regenerative_shaft_input),
            regenerative_dc_output_j: self
                .has_regeneration
                .then_some(self.regenerative_dc_output),
            auxiliary_dc_input_j: self.auxiliary_dc_input,
            net_dc_input_j,
            motoring_efficiency,
            regenerative_efficiency,
        }
    }
}

pub fn evaluate_duty_cycle(
    spec: &DutyCycleSpec,
    evidence: &[SegmentEvidence],
) -> DutyProfileEvaluation {
    let mut global_blockers = validate_spec(spec);
    let mut seen_segments = BTreeSet::new();
    let mut seen_evidence = BTreeSet::new();
    let mut seen_receipts = BTreeSet::new();

    for item in evidence {
        if !seen_segments.insert(item.segment_id.as_str()) {
            global_blockers.push(Blocker::DuplicateSegment);
        }
        if !seen_evidence.insert(item.evidence_id.as_str()) {
            global_blockers.push(Blocker::DuplicateEvidenceId);
        }
        if !seen_receipts.insert(item.receipt_id.as_str()) {
            global_blockers.push(Blocker::DuplicateReceiptId);
        }
    }

    if evidence.len() < spec.segments.len() {
        global_blockers.push(Blocker::MissingSegment);
    } else if evidence.len() > spec.segments.len() {
        global_blockers.push(Blocker::ExtraSegment);
    }

    let mut segment_evaluations = Vec::with_capacity(spec.segments.len());
    let mut energy_accumulator = EnergyAccumulator::default();
    let mut energy_complete = evidence.len() == spec.segments.len()
        && !global_blockers.iter().any(|item| {
            matches!(
                item,
                Blocker::DuplicateSegment
                    | Blocker::DuplicateEvidenceId
                    | Blocker::DuplicateReceiptId
                    | Blocker::ExtraSegment
            )
        });

    for (index, expected) in spec.segments.iter().enumerate() {
        let Some(observed) = evidence.get(index) else {
            segment_evaluations.push(SegmentEvaluation {
                segment_id: expected.segment_id.clone(),
                evidence_id: None,
                disposition: Disposition::Indeterminate,
                blockers: vec![Blocker::MissingSegment],
            });
            energy_complete = false;
            continue;
        };

        let mut blockers = Vec::new();
        let mut not_comparable = false;
        let mut hard_failure = false;
        let mut unresolved = false;

        if observed.segment_id != expected.segment_id
            || observed.sequence_index != expected.sequence_index
            || expected.sequence_index as usize != index
        {
            blockers.push(Blocker::SegmentOrderMismatch);
            not_comparable = true;
        }
        if observed.profile_digest != spec.profile_digest {
            blockers.push(Blocker::ProfileMismatch);
            not_comparable = true;
        }
        if observed.component_id != spec.component_id {
            blockers.push(Blocker::ComponentMismatch);
            not_comparable = true;
        }
        if observed.system_boundary_digest != spec.system_boundary_digest {
            blockers.push(Blocker::SystemBoundaryMismatch);
            not_comparable = true;
        }
        if observed.measurement_setup_digest != spec.measurement_setup_digest {
            blockers.push(Blocker::MeasurementSetupMismatch);
            not_comparable = true;
        }

        let provenance_valid = validate_provenance(observed, &mut blockers);
        if !provenance_valid {
            unresolved = true;
        }

        let expected_thermal_start = if index == 0 {
            Some(spec.initial_thermal_state_digest.as_str())
        } else {
            evidence
                .get(index - 1)
                .map(|previous| previous.thermal_end_state_digest.as_str())
        };
        if expected_thermal_start
            .is_some_and(|value| value != observed.thermal_start_state_digest)
        {
            blockers.push(if index == 0 {
                Blocker::ThermalInitialStateMismatch
            } else {
                Blocker::ThermalStateDiscontinuity
            });
            unresolved = true;
        }

        let intervals_valid = validate_observation_intervals(observed, &mut blockers);
        if !intervals_valid {
            unresolved = true;
        }

        if provenance_valid && intervals_valid && !not_comparable {
            evaluate_minimum(
                observed.torque_tracking,
                expected.minimum_torque_tracking,
                Blocker::TorqueShortfall,
                Blocker::TorqueUnresolved,
                &mut blockers,
                &mut hard_failure,
                &mut unresolved,
            );
            evaluate_minimum(
                observed.speed_tracking,
                expected.minimum_speed_tracking,
                Blocker::SpeedShortfall,
                Blocker::SpeedUnresolved,
                &mut blockers,
                &mut hard_failure,
                &mut unresolved,
            );
            evaluate_maximum(
                observed.maximum_winding_temperature_c,
                expected.maximum_winding_temperature_c,
                Blocker::WindingTemperatureExceeded,
                Blocker::WindingTemperatureUnresolved,
                &mut blockers,
                &mut hard_failure,
                &mut unresolved,
            );
            evaluate_maximum(
                observed.maximum_magnet_temperature_c,
                expected.maximum_magnet_temperature_c,
                Blocker::MagnetTemperatureExceeded,
                Blocker::MagnetTemperatureUnresolved,
                &mut blockers,
                &mut hard_failure,
                &mut unresolved,
            );
            evaluate_minimum(
                observed.minimum_demagnetization_margin,
                expected.minimum_demagnetization_margin,
                Blocker::DemagnetizationMarginFailed,
                Blocker::DemagnetizationMarginUnresolved,
                &mut blockers,
                &mut hard_failure,
                &mut unresolved,
            );
            evaluate_minimum(
                observed.minimum_retention_margin,
                expected.minimum_retention_margin,
                Blocker::RetentionMarginFailed,
                Blocker::RetentionMarginUnresolved,
                &mut blockers,
                &mut hard_failure,
                &mut unresolved,
            );
        }

        let energy_valid = validate_energy(expected.quadrant, observed.energy, &mut blockers);
        if energy_valid && provenance_valid && !not_comparable {
            energy_accumulator.add_segment(expected.quadrant, observed.energy);
        } else {
            energy_complete = false;
            unresolved = true;
        }

        let disposition = if not_comparable {
            Disposition::NotComparable
        } else if hard_failure {
            Disposition::Fail
        } else if unresolved || !blockers.is_empty() {
            Disposition::Indeterminate
        } else {
            Disposition::Qualified
        };

        segment_evaluations.push(SegmentEvaluation {
            segment_id: expected.segment_id.clone(),
            evidence_id: Some(observed.evidence_id.clone()),
            disposition,
            blockers: dedupe(blockers),
        });
    }

    let energy = if energy_complete {
        Some(energy_accumulator.summarize(&mut global_blockers))
    } else {
        None
    };

    let disposition = if global_blockers.iter().any(is_not_comparable_blocker)
        || segment_evaluations
            .iter()
            .any(|item| item.disposition == Disposition::NotComparable)
    {
        Disposition::NotComparable
    } else if segment_evaluations
        .iter()
        .any(|item| item.disposition == Disposition::Fail)
    {
        Disposition::Fail
    } else if !global_blockers.is_empty()
        || energy.is_none()
        || segment_evaluations
            .iter()
            .any(|item| item.disposition != Disposition::Qualified)
    {
        Disposition::Indeterminate
    } else {
        Disposition::Qualified
    };

    DutyProfileEvaluation {
        profile_id: spec.profile_id.clone(),
        disposition,
        expected_segment_count: spec.segments.len(),
        observed_segment_count: evidence.len(),
        segment_evaluations,
        energy,
        blockers: dedupe(global_blockers),
        claim_ceiling: format!(
            "{disposition:?}: ordered duty-profile evidence aggregation only; no solver validation, standards conformity, certification or product-release authority"
        ),
    }
}

fn validate_spec(spec: &DutyCycleSpec) -> Vec<Blocker> {
    let mut blockers = Vec::new();
    if spec.profile_id.trim().is_empty()
        || spec.component_id.trim().is_empty()
        || spec.segments.is_empty()
    {
        blockers.push(Blocker::InvalidSpec);
    }
    for digest in [
        &spec.profile_digest,
        &spec.system_boundary_digest,
        &spec.measurement_setup_digest,
        &spec.initial_thermal_state_digest,
    ] {
        if !valid_sha256(digest) {
            blockers.push(Blocker::InvalidDigest);
        }
    }

    let mut segment_ids = BTreeSet::new();
    for (index, segment) in spec.segments.iter().enumerate() {
        if segment.segment_id.trim().is_empty()
            || !segment_ids.insert(segment.segment_id.as_str())
        {
            blockers.push(Blocker::DuplicateExpectedSegment);
        }
        if segment.sequence_index as usize != index
            || !segment.duration_s.is_finite()
            || segment.duration_s <= 0.0
            || !segment.minimum_torque_tracking.is_finite()
            || segment.minimum_torque_tracking < 0.0
            || !segment.minimum_speed_tracking.is_finite()
            || segment.minimum_speed_tracking < 0.0
            || !segment.maximum_winding_temperature_c.is_finite()
            || !segment.maximum_magnet_temperature_c.is_finite()
            || !segment.minimum_demagnetization_margin.is_finite()
            || segment.minimum_demagnetization_margin < 0.0
            || !segment.minimum_retention_margin.is_finite()
            || segment.minimum_retention_margin < 0.0
        {
            blockers.push(Blocker::InvalidSegmentDefinition);
        }
    }
    blockers
}

fn validate_provenance(item: &SegmentEvidence, blockers: &mut Vec<Blocker>) -> bool {
    let mut valid = true;
    if item.segment_id.trim().is_empty()
        || item.component_id.trim().is_empty()
        || item.evidence_id.trim().is_empty()
        || item.receipt_id.trim().is_empty()
    {
        blockers.push(Blocker::MissingEvidenceIdentity);
        valid = false;
    }
    for digest in [
        &item.profile_digest,
        &item.system_boundary_digest,
        &item.measurement_setup_digest,
        &item.evidence_digest,
        &item.receipt_digest,
        &item.calibration_digest,
        &item.uncertainty_record_digest,
        &item.thermal_start_state_digest,
        &item.thermal_end_state_digest,
    ] {
        if !valid_sha256(digest) {
            blockers.push(Blocker::InvalidDigest);
            valid = false;
        }
    }
    if item.evidence_class != EvidenceClass::PhysicalMeasurement {
        blockers.push(Blocker::NonPhysicalEvidence);
        valid = false;
    }
    if item.method_id.trim().is_empty() {
        blockers.push(Blocker::MissingMeasurementMethod);
        valid = false;
    }
    valid
}

fn validate_observation_intervals(
    item: &SegmentEvidence,
    blockers: &mut Vec<Blocker>,
) -> bool {
    let valid = [
        item.torque_tracking,
        item.speed_tracking,
        item.maximum_winding_temperature_c,
        item.maximum_magnet_temperature_c,
        item.minimum_demagnetization_margin,
        item.minimum_retention_margin,
    ]
    .into_iter()
    .all(Interval::is_valid);
    if !valid {
        blockers.push(Blocker::InvalidInterval);
    }
    valid
}

fn validate_energy(
    quadrant: OperatingQuadrant,
    energy: EnergyFlows,
    blockers: &mut Vec<Blocker>,
) -> bool {
    let mut valid = energy.auxiliary_dc_input_j.is_nonnegative();
    for interval in [
        energy.dc_input_j,
        energy.shaft_output_j,
        energy.shaft_input_j,
        energy.dc_output_j,
    ]
    .into_iter()
    .flatten()
    {
        valid &= interval.is_nonnegative();
    }

    let quadrant_valid = match quadrant {
        OperatingQuadrant::Motoring => {
            energy.dc_input_j.is_some()
                && energy.shaft_output_j.is_some()
                && energy.shaft_input_j.is_none()
                && energy.dc_output_j.is_none()
        }
        OperatingQuadrant::Regenerative => {
            energy.dc_input_j.is_none()
                && energy.shaft_output_j.is_none()
                && energy.shaft_input_j.is_some()
                && energy.dc_output_j.is_some()
        }
        OperatingQuadrant::Standstill => {
            energy.dc_input_j.is_none()
                && energy.shaft_output_j.is_none()
                && energy.shaft_input_j.is_none()
                && energy.dc_output_j.is_none()
        }
    };

    if !valid {
        blockers.push(Blocker::InvalidEnergyInterval);
    }
    if !quadrant_valid {
        blockers.push(Blocker::InvalidEnergyQuadrant);
    }
    valid && quadrant_valid
}

fn evaluate_minimum(
    interval: Interval,
    threshold: f64,
    fail_blocker: Blocker,
    unresolved_blocker: Blocker,
    blockers: &mut Vec<Blocker>,
    hard_failure: &mut bool,
    unresolved: &mut bool,
) {
    if interval.upper < threshold {
        blockers.push(fail_blocker);
        *hard_failure = true;
    } else if interval.lower < threshold {
        blockers.push(unresolved_blocker);
        *unresolved = true;
    }
}

fn evaluate_maximum(
    interval: Interval,
    threshold: f64,
    fail_blocker: Blocker,
    unresolved_blocker: Blocker,
    blockers: &mut Vec<Blocker>,
    hard_failure: &mut bool,
    unresolved: &mut bool,
) {
    if interval.lower > threshold {
        blockers.push(fail_blocker);
        *hard_failure = true;
    } else if interval.upper > threshold {
        blockers.push(unresolved_blocker);
        *unresolved = true;
    }
}

fn valid_sha256(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_not_comparable_blocker(blocker: &Blocker) -> bool {
    matches!(
        blocker,
        Blocker::InvalidSpec
            | Blocker::DuplicateExpectedSegment
            | Blocker::ExtraSegment
            | Blocker::DuplicateSegment
            | Blocker::DuplicateEvidenceId
            | Blocker::DuplicateReceiptId
            | Blocker::SegmentOrderMismatch
            | Blocker::ProfileMismatch
            | Blocker::ComponentMismatch
            | Blocker::SystemBoundaryMismatch
            | Blocker::MeasurementSetupMismatch
    )
}

fn dedupe(mut blockers: Vec<Blocker>) -> Vec<Blocker> {
    blockers.sort_by_key(|blocker| format!("{blocker:?}"));
    blockers.dedup();
    blockers
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(character: char) -> String {
        format!("sha256:{}", character.to_string().repeat(64))
    }

    fn segment(
        id: &str,
        index: u32,
        quadrant: OperatingQuadrant,
    ) -> SegmentSpec {
        SegmentSpec {
            segment_id: id.to_string(),
            sequence_index: index,
            duration_s: 10.0,
            quadrant,
            minimum_torque_tracking: 0.98,
            minimum_speed_tracking: 0.99,
            maximum_winding_temperature_c: 150.0,
            maximum_magnet_temperature_c: 130.0,
            minimum_demagnetization_margin: 1.2,
            minimum_retention_margin: 2.0,
        }
    }

    fn spec() -> DutyCycleSpec {
        DutyCycleSpec {
            profile_id: "MISSION-50KW-001".to_string(),
            component_id: "MOTOR-001".to_string(),
            profile_digest: digest('a'),
            system_boundary_digest: digest('b'),
            measurement_setup_digest: digest('c'),
            initial_thermal_state_digest: digest('1'),
            segments: vec![
                segment("launch", 0, OperatingQuadrant::Motoring),
                segment("cruise", 1, OperatingQuadrant::Motoring),
                segment("regeneration", 2, OperatingQuadrant::Regenerative),
                segment("standstill", 3, OperatingQuadrant::Standstill),
            ],
        }
    }

    fn energy_for(quadrant: OperatingQuadrant, dc: f64, shaft: f64) -> EnergyFlows {
        match quadrant {
            OperatingQuadrant::Motoring => EnergyFlows {
                dc_input_j: Some(Interval::new(dc - 0.1, dc + 0.1)),
                shaft_output_j: Some(Interval::new(shaft - 0.1, shaft + 0.1)),
                shaft_input_j: None,
                dc_output_j: None,
                auxiliary_dc_input_j: Interval::new(4.9, 5.1),
            },
            OperatingQuadrant::Regenerative => EnergyFlows {
                dc_input_j: None,
                shaft_output_j: None,
                shaft_input_j: Some(Interval::new(dc - 0.1, dc + 0.1)),
                dc_output_j: Some(Interval::new(shaft - 0.1, shaft + 0.1)),
                auxiliary_dc_input_j: Interval::new(0.9, 1.1),
            },
            OperatingQuadrant::Standstill => EnergyFlows {
                dc_input_j: None,
                shaft_output_j: None,
                shaft_input_j: None,
                dc_output_j: None,
                auxiliary_dc_input_j: Interval::new(1.9, 2.1),
            },
        }
    }

    fn evidence_for(
        specification: &DutyCycleSpec,
        index: usize,
        thermal_start: char,
        thermal_end: char,
    ) -> SegmentEvidence {
        let segment = &specification.segments[index];
        let (dc, shaft) = match index {
            0 => (100.0, 90.0),
            1 => (900.0, 450.0),
            2 => (100.0, 80.0),
            _ => (0.0, 0.0),
        };
        SegmentEvidence {
            segment_id: segment.segment_id.clone(),
            sequence_index: segment.sequence_index,
            profile_digest: specification.profile_digest.clone(),
            component_id: specification.component_id.clone(),
            system_boundary_digest: specification.system_boundary_digest.clone(),
            measurement_setup_digest: specification.measurement_setup_digest.clone(),
            evidence_id: format!("OBS-{}", segment.segment_id),
            receipt_id: format!("RECEIPT-{}", segment.segment_id),
            evidence_digest: digest('d'),
            receipt_digest: digest('e'),
            calibration_digest: digest('f'),
            uncertainty_record_digest: digest('2'),
            method_id: format!("METHOD-{}", segment.segment_id),
            evidence_class: EvidenceClass::PhysicalMeasurement,
            thermal_start_state_digest: digest(thermal_start),
            thermal_end_state_digest: digest(thermal_end),
            torque_tracking: Interval::new(0.995, 1.005),
            speed_tracking: Interval::new(0.995, 1.005),
            maximum_winding_temperature_c: Interval::new(90.0, 92.0),
            maximum_magnet_temperature_c: Interval::new(80.0, 82.0),
            minimum_demagnetization_margin: Interval::new(1.5, 1.6),
            minimum_retention_margin: Interval::new(2.5, 2.6),
            energy: energy_for(segment.quadrant, dc, shaft),
        }
    }

    fn evidence() -> Vec<SegmentEvidence> {
        let specification = spec();
        vec![
            evidence_for(&specification, 0, '1', '2'),
            evidence_for(&specification, 1, '2', '3'),
            evidence_for(&specification, 2, '3', '4'),
            evidence_for(&specification, 3, '4', '5'),
        ]
    }

    #[test]
    fn exact_profile_is_qualified_with_complete_coverage() {
        let result = evaluate_duty_cycle(&spec(), &evidence());
        assert_eq!(result.disposition, Disposition::Qualified);
        assert_eq!(result.expected_segment_count, 4);
        assert_eq!(result.observed_segment_count, 4);
        assert!(result.segment_evaluations.iter().all(|item| {
            item.disposition == Disposition::Qualified
        }));
        assert!(result.energy.is_some());
    }

    #[test]
    fn motoring_efficiency_is_ratio_of_summed_energy_not_mean_of_percentages() {
        let result = evaluate_duty_cycle(&spec(), &evidence());
        let energy = result.energy.expect("complete energy summary");
        let efficiency = energy.motoring_efficiency.expect("motoring efficiency");
        assert!((efficiency.lower - 0.539).abs() < 0.002);
        assert!((efficiency.upper - 0.541).abs() < 0.002);
        assert!((efficiency.lower - 0.70).abs() > 0.1);
    }

    #[test]
    fn regenerative_efficiency_is_reported_separately() {
        let result = evaluate_duty_cycle(&spec(), &evidence());
        let energy = result.energy.expect("complete energy summary");
        let efficiency = energy.regenerative_efficiency.expect("regen efficiency");
        assert!(efficiency.lower < 0.80 && efficiency.upper > 0.80);
    }

    #[test]
    fn auxiliary_energy_is_included_in_net_mission_energy() {
        let result = evaluate_duty_cycle(&spec(), &evidence());
        let energy = result.energy.expect("complete energy summary");
        assert!(energy.net_dc_input_j.lower > 930.0);
        assert!(energy.net_dc_input_j.upper < 936.0);
    }

    #[test]
    fn definite_local_temperature_violation_fails_despite_good_cycle_efficiency() {
        let mut records = evidence();
        records[1].maximum_magnet_temperature_c = Interval::new(131.0, 133.0);
        let result = evaluate_duty_cycle(&spec(), &records);
        assert_eq!(result.disposition, Disposition::Fail);
        assert_eq!(result.segment_evaluations[1].disposition, Disposition::Fail);
    }

    #[test]
    fn threshold_overlap_is_indeterminate_not_a_pass() {
        let mut records = evidence();
        records[0].maximum_magnet_temperature_c = Interval::new(129.0, 131.0);
        let result = evaluate_duty_cycle(&spec(), &records);
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert_eq!(
            result.segment_evaluations[0].disposition,
            Disposition::Indeterminate
        );
    }

    #[test]
    fn missing_segment_remains_in_the_expected_denominator() {
        let mut records = evidence();
        records.pop();
        let result = evaluate_duty_cycle(&spec(), &records);
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert_eq!(result.expected_segment_count, 4);
        assert_eq!(result.observed_segment_count, 3);
        assert!(result.energy.is_none());
    }

    #[test]
    fn extra_segment_is_not_silently_ignored() {
        let mut records = evidence();
        let mut extra = records[3].clone();
        extra.segment_id = "unregistered-extra".to_string();
        extra.sequence_index = 4;
        extra.evidence_id = "OBS-extra".to_string();
        extra.receipt_id = "RECEIPT-extra".to_string();
        records.push(extra);
        let result = evaluate_duty_cycle(&spec(), &records);
        assert_eq!(result.disposition, Disposition::NotComparable);
        assert!(result.blockers.contains(&Blocker::ExtraSegment));
    }

    #[test]
    fn reordered_segments_are_not_repaired_by_sorting() {
        let mut records = evidence();
        records.swap(0, 1);
        let result = evaluate_duty_cycle(&spec(), &records);
        assert_eq!(result.disposition, Disposition::NotComparable);
        assert!(result.blockers.contains(&Blocker::SegmentOrderMismatch));
    }

    #[test]
    fn thermal_state_must_chain_between_segments() {
        let mut records = evidence();
        records[2].thermal_start_state_digest = digest('9');
        let result = evaluate_duty_cycle(&spec(), &records);
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(result.segment_evaluations[2]
            .blockers
            .contains(&Blocker::ThermalStateDiscontinuity));
    }

    #[test]
    fn initial_thermal_state_must_match_frozen_profile() {
        let mut records = evidence();
        records[0].thermal_start_state_digest = digest('8');
        let result = evaluate_duty_cycle(&spec(), &records);
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(result.segment_evaluations[0]
            .blockers
            .contains(&Blocker::ThermalInitialStateMismatch));
    }

    #[test]
    fn synthetic_or_simulated_observations_cannot_qualify_physical_duty() {
        let mut records = evidence();
        records[0].evidence_class = EvidenceClass::SyntheticControl;
        let result = evaluate_duty_cycle(&spec(), &records);
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(result.segment_evaluations[0]
            .blockers
            .contains(&Blocker::NonPhysicalEvidence));
    }

    #[test]
    fn wrong_profile_digest_is_not_comparable() {
        let mut records = evidence();
        records[0].profile_digest = digest('9');
        let result = evaluate_duty_cycle(&spec(), &records);
        assert_eq!(result.disposition, Disposition::NotComparable);
        assert!(result.segment_evaluations[0]
            .blockers
            .contains(&Blocker::ProfileMismatch));
    }

    #[test]
    fn quadrant_flow_mismatch_invalidates_energy_summary() {
        let mut records = evidence();
        records[2].energy.dc_input_j = Some(Interval::new(20.0, 21.0));
        let result = evaluate_duty_cycle(&spec(), &records);
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(result.energy.is_none());
        assert!(result.segment_evaluations[2]
            .blockers
            .contains(&Blocker::InvalidEnergyQuadrant));
    }

    #[test]
    fn nonpositive_energy_denominator_does_not_emit_efficiency() {
        let mut records = evidence();
        records[0].energy.dc_input_j = Some(Interval::new(0.0, 0.1));
        records[0].energy.shaft_output_j = Some(Interval::new(0.0, 0.05));
        records[1].energy.dc_input_j = Some(Interval::new(0.0, 0.1));
        records[1].energy.shaft_output_j = Some(Interval::new(0.0, 0.05));
        let result = evaluate_duty_cycle(&spec(), &records);
        let energy = result.energy.expect("energy record is still present");
        assert!(energy.motoring_efficiency.is_none());
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(result.blockers.contains(&Blocker::NonPositiveEnergyDenominator));
    }

    #[test]
    fn duplicate_segment_identity_invalidates_comparison() {
        let mut records = evidence();
        records[3].segment_id = records[2].segment_id.clone();
        let result = evaluate_duty_cycle(&spec(), &records);
        assert_eq!(result.disposition, Disposition::NotComparable);
        assert!(result.blockers.contains(&Blocker::DuplicateSegment));
    }

    #[test]
    fn malformed_interval_is_indeterminate() {
        let mut records = evidence();
        records[0].torque_tracking = Interval::new(1.1, 1.0);
        let result = evaluate_duty_cycle(&spec(), &records);
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(result.segment_evaluations[0]
            .blockers
            .contains(&Blocker::InvalidInterval));
    }

    #[test]
    fn definite_torque_shortfall_is_a_hard_failure() {
        let mut records = evidence();
        records[1].torque_tracking = Interval::new(0.90, 0.95);
        let result = evaluate_duty_cycle(&spec(), &records);
        assert_eq!(result.disposition, Disposition::Fail);
        assert!(result.segment_evaluations[1]
            .blockers
            .contains(&Blocker::TorqueShortfall));
    }

    #[test]
    fn aggregate_energy_overflow_is_indeterminate() {
        let mut records = evidence();
        for record in records.iter_mut().take(2) {
            record.energy.dc_input_j = Some(Interval::new(1.0e308, 1.0e308));
            record.energy.shaft_output_j = Some(Interval::new(5.0e307, 5.0e307));
        }
        let result = evaluate_duty_cycle(&spec(), &records);
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(result
            .blockers
            .contains(&Blocker::OverflowInEnergyArithmetic));
    }

    #[test]
    fn energy_ratio_above_unity_is_not_reported_as_superior_efficiency() {
        let mut records = evidence();
        records[0].energy.shaft_output_j = Some(Interval::new(1_200.0, 1_201.0));
        let result = evaluate_duty_cycle(&spec(), &records);
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(result.blockers.contains(&Blocker::EnergyEfficiencyPhysicallyInconsistent));
    }
}
