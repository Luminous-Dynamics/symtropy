#![forbid(unsafe_code)]

//! Task-constrained active diagnosability from bounded feature signatures.
//!
//! The M1 reference does not synthesize excitation trajectories or generate
//! fault signatures. It consumes externally supplied, evidence-backed
//! signature intervals and prequalified hard-constraint margins. It checks
//! whether fault-hypothesis boxes are distinguishable under every declared
//! sensor-availability scenario.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceClass {
    IndependentPhysical,
    SourceAuthorPhysical,
    VendorPhysical,
    ModelDerived,
    SyntheticFixture,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticConstraintKind {
    TaskPerformance,
    TorqueSpeed,
    PeakDuration,
    ThermalHeadroom,
    EnergyBudget,
    Passivity,
    Workspace,
    ExposureTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairDisposition {
    Separable,
    NotSeparable,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileDisposition {
    Infeasible,
    Unknown,
    FeasibleButNotFullyDiagnosable,
    FeasibleFullyDiagnosable,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NumericInterval {
    pub lower: f64,
    pub upper: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstraintMargin {
    pub kind: DiagnosticConstraintKind,
    /// Conservative lower bound on available margin. Negative is a violation.
    pub minimum_robust_margin: Option<f64>,
    pub unit: String,
    pub evidence_class: EvidenceClass,
    pub source_reference: Option<String>,
    pub method: Option<String>,
    pub configuration_digest: String,
    pub system_boundary_digest: String,
    pub operating_condition_digest: String,
    pub mission_set_digest: String,
    pub profile_digest: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiagnosticFeatureInterval {
    pub feature_id: String,
    pub sensor_channel_id: String,
    pub modality: String,
    pub unit: String,
    pub interval: NumericInterval,
    /// Fixed positive scale for dimensionless interval-gap reporting.
    pub separation_scale: f64,
    pub evidence_class: EvidenceClass,
    pub source_reference: Option<String>,
    pub method: Option<String>,
    pub calibration_digest: Option<String>,
    pub raw_trace_digest: Option<String>,
    pub configuration_digest: String,
    pub system_boundary_digest: String,
    pub operating_condition_digest: String,
    pub mission_set_digest: String,
    pub profile_digest: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FaultSignature {
    pub hypothesis_id: String,
    /// Include a healthy/nominal hypothesis as well as fault hypotheses.
    pub features: Vec<DiagnosticFeatureInterval>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SensorAvailabilityScenario {
    pub scenario_id: String,
    /// Channels available in this required diagnostic scenario. An empty set
    /// is legal input and yields Unknown rather than fabricated separability.
    pub available_channel_ids: Vec<String>,
    pub evidence_class: EvidenceClass,
    pub source_reference: Option<String>,
    pub method: Option<String>,
    pub configuration_digest: String,
    pub system_boundary_digest: String,
    pub operating_condition_digest: String,
    pub mission_set_digest: String,
    pub profile_digest: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiagnosticProfile {
    pub profile_id: String,
    pub profile_digest: String,
    pub configuration_digest: String,
    pub system_boundary_digest: String,
    pub operating_condition_digest: String,
    pub mission_set_digest: String,
    /// Total diagnostic excitation duration (seconds).
    pub diagnostic_duration_s: f64,
    /// Externally estimated/validated electrical energy budget (joules).
    pub electrical_energy_j: f64,
    pub hard_constraint_margins: Vec<ConstraintMargin>,
    pub fault_signatures: Vec<FaultSignature>,
    pub sensor_scenarios: Vec<SensorAvailabilityScenario>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PairAssessment {
    pub scenario_id: String,
    pub left_hypothesis_id: String,
    pub right_hypothesis_id: String,
    pub disposition: PairDisposition,
    /// Features that independently separate the two validated intervals.
    pub separating_feature_ids: Vec<String>,
    /// Maximum dimensionless gap among separating features.
    pub maximum_normalized_gap: Option<f64>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProfileEvaluation {
    pub profile_id: String,
    pub profile_digest: String,
    pub configuration_digest: String,
    pub system_boundary_digest: String,
    pub operating_condition_digest: String,
    pub mission_set_digest: String,
    pub disposition: ProfileDisposition,
    pub known_constraint_violations: Vec<DiagnosticConstraintKind>,
    pub unknown_constraints: Vec<DiagnosticConstraintKind>,
    pub hypothesis_pair_count: usize,
    pub required_scenario_count: usize,
    pub separable_pair_scenario_count: usize,
    pub nonseparable_pair_scenario_count: usize,
    pub unknown_pair_scenario_count: usize,
    pub pair_scenario_separation_coverage: Option<f64>,
    /// Zero when at least one pair/scenario is known not to be separable;
    /// None when separation cannot be evaluated.
    pub worst_case_normalized_separation: Option<f64>,
    pub diagnostic_duration_s: f64,
    pub electrical_energy_j: f64,
    pub pair_assessments: Vec<PairAssessment>,
    pub interpretation: String,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn finite_interval(value: NumericInterval) -> bool {
    value.lower.is_finite() && value.upper.is_finite() && value.lower <= value.upper
}

fn expected_constraints() -> [DiagnosticConstraintKind; 8] {
    [
        DiagnosticConstraintKind::TaskPerformance,
        DiagnosticConstraintKind::TorqueSpeed,
        DiagnosticConstraintKind::PeakDuration,
        DiagnosticConstraintKind::ThermalHeadroom,
        DiagnosticConstraintKind::EnergyBudget,
        DiagnosticConstraintKind::Passivity,
        DiagnosticConstraintKind::Workspace,
        DiagnosticConstraintKind::ExposureTime,
    ]
}

fn validate_profile_structure(profile: &DiagnosticProfile) -> Result<(), String> {
    if !nonempty(&profile.profile_id)
        || !nonempty(&profile.profile_digest)
        || !nonempty(&profile.configuration_digest)
        || !nonempty(&profile.system_boundary_digest)
        || !nonempty(&profile.operating_condition_digest)
        || !nonempty(&profile.mission_set_digest)
    {
        return Err("profile and configuration/boundary/condition/mission digests are required".to_owned());
    }
    if !profile.diagnostic_duration_s.is_finite()
        || profile.diagnostic_duration_s <= 0.0
        || !profile.electrical_energy_j.is_finite()
        || profile.electrical_energy_j < 0.0
    {
        return Err("diagnostic duration must be positive and energy budget must be finite/nonnegative".to_owned());
    }
    if profile.fault_signatures.len() < 2 {
        return Err("at least two fault hypotheses, including healthy, are required".to_owned());
    }
    if !profile.fault_signatures.iter().any(|signature| signature.hypothesis_id == "healthy") {
        return Err("the healthy hypothesis must be included".to_owned());
    }
    if profile.sensor_scenarios.is_empty() {
        return Err("at least one required sensor-availability scenario is required".to_owned());
    }

    let mut hypothesis_ids = BTreeSet::new();
    let mut canonical_features: Option<BTreeMap<String, (String, String, String, u64)>> = None;

    for signature in &profile.fault_signatures {
        if !nonempty(&signature.hypothesis_id)
            || !hypothesis_ids.insert(signature.hypothesis_id.as_str())
            || signature.features.is_empty()
        {
            return Err("fault hypotheses must be unique, named and contain feature intervals".to_owned());
        }
        let mut features = BTreeMap::new();
        for feature in &signature.features {
            if !nonempty(&feature.feature_id)
                || !nonempty(&feature.sensor_channel_id)
                || !nonempty(&feature.modality)
                || !nonempty(&feature.unit)
                || !finite_interval(feature.interval)
                || !feature.separation_scale.is_finite()
                || feature.separation_scale <= 0.0
            {
                return Err(format!(
                    "hypothesis {} contains an invalid feature interval or scale",
                    signature.hypothesis_id
                ));
            }
            if feature.configuration_digest != profile.configuration_digest
                || feature.system_boundary_digest != profile.system_boundary_digest
                || feature.operating_condition_digest != profile.operating_condition_digest
                || feature.mission_set_digest != profile.mission_set_digest
                || feature.profile_digest != profile.profile_digest
            {
                return Err(format!(
                    "feature {} in hypothesis {} has mismatched profile/configuration/boundary/condition/mission identity",
                    feature.feature_id, signature.hypothesis_id
                ));
            }
            if !features.insert(
                feature.feature_id.clone(),
                (
                    feature.sensor_channel_id.clone(),
                    feature.modality.clone(),
                    feature.unit.clone(),
                    feature.separation_scale.to_bits(),
                ),
            ).is_none() {
                return Err(format!(
                    "duplicate feature {} in hypothesis {}",
                    feature.feature_id, signature.hypothesis_id
                ));
            }
        }
        match &canonical_features {
            None => canonical_features = Some(features),
            Some(canonical) if canonical != &features => {
                return Err(format!(
                    "hypothesis {} does not define the same feature dimensions as the other hypotheses",
                    signature.hypothesis_id
                ));
            }
            _ => {}
        }
    }

    let known_channels: BTreeSet<&str> = canonical_features
        .as_ref()
        .unwrap()
        .values()
        .map(|(channel, _, _, _)| channel.as_str())
        .collect();

    let mut scenario_ids = BTreeSet::new();
    for scenario in &profile.sensor_scenarios {
        if !nonempty(&scenario.scenario_id) || !scenario_ids.insert(scenario.scenario_id.as_str()) {
            return Err("sensor-availability scenario IDs must be nonempty and unique".to_owned());
        }
        if scenario.configuration_digest != profile.configuration_digest
            || scenario.system_boundary_digest != profile.system_boundary_digest
            || scenario.operating_condition_digest != profile.operating_condition_digest
            || scenario.mission_set_digest != profile.mission_set_digest
            || scenario.profile_digest != profile.profile_digest
        {
            return Err(format!("scenario {} has mismatched profile/configuration/condition identity", scenario.scenario_id));
        }
        let mut available = BTreeSet::new();
        for channel in &scenario.available_channel_ids {
            if !known_channels.contains(channel.as_str()) {
                return Err(format!(
                    "scenario {} references unknown channel {}",
                    scenario.scenario_id, channel
                ));
            }
            if !available.insert(channel.as_str()) {
                return Err(format!(
                    "scenario {} contains duplicate available channel {}",
                    scenario.scenario_id, channel
                ));
            }
        }
    }

    // Missing or unknown hard margins are represented as Unknown in the result.
    // Duplicate supplied margin kinds are structural errors.
    let mut margin_kinds = BTreeSet::new();
    for margin in &profile.hard_constraint_margins {
        if !margin_kinds.insert(margin.kind) {
            return Err(format!("duplicate hard constraint margin {:?}", margin.kind));
        }
        if margin.unit.trim().is_empty()
            || margin.minimum_robust_margin.is_some_and(|value| !value.is_finite())
        {
            return Err(format!("constraint {:?} has invalid unit or non-finite margin", margin.kind));
        }
        if margin.configuration_digest != profile.configuration_digest
            || margin.system_boundary_digest != profile.system_boundary_digest
            || margin.operating_condition_digest != profile.operating_condition_digest
            || margin.mission_set_digest != profile.mission_set_digest
            || margin.profile_digest != profile.profile_digest
        {
            return Err(format!("constraint {:?} has mismatched profile or operating identity", margin.kind));
        }
    }

    Ok(())
}

fn interval_gap(left: NumericInterval, right: NumericInterval) -> f64 {
    if left.upper < right.lower {
        right.lower - left.upper
    } else if right.upper < left.lower {
        left.lower - right.upper
    } else {
        0.0
    }
}

fn feature_provenance_complete(feature: &DiagnosticFeatureInterval) -> bool {
    feature.evidence_class != EvidenceClass::Unknown
        && feature.source_reference.as_deref().is_some_and(nonempty)
        && feature.method.as_deref().is_some_and(nonempty)
        && feature.calibration_digest.as_deref().is_some_and(nonempty)
        && feature.raw_trace_digest.as_deref().is_some_and(nonempty)
}

fn scenario_provenance_complete(scenario: &SensorAvailabilityScenario) -> bool {
    scenario.evidence_class != EvidenceClass::Unknown
        && scenario.source_reference.as_deref().is_some_and(nonempty)
        && scenario.method.as_deref().is_some_and(nonempty)
}

fn evaluate_pair(
    left: &FaultSignature,
    right: &FaultSignature,
    scenario: &SensorAvailabilityScenario,
) -> PairAssessment {
    let mut separating_features = Vec::new();
    let mut maximum_normalized_gap = 0.0_f64;
    let mut considered_features = 0usize;
    let mut unknown_features = 0usize;

    for left_feature in &left.features {
        if !scenario.available_channel_ids.iter().any(|channel| channel == &left_feature.sensor_channel_id) {
            continue;
        }
        let right_feature = right.features
            .iter()
            .find(|feature| feature.feature_id == left_feature.feature_id)
            .unwrap();

        considered_features += 1;
        if !feature_provenance_complete(left_feature)
            || !feature_provenance_complete(right_feature)
        {
            unknown_features += 1;
            continue;
        }

        let gap = interval_gap(left_feature.interval, right_feature.interval);
        let normalized_gap = gap / left_feature.separation_scale;
        if normalized_gap > 0.0 {
            separating_features.push(left_feature.feature_id.clone());
            maximum_normalized_gap = maximum_normalized_gap.max(normalized_gap);
        }
    }

    let (disposition, reason, normalized_gap) = if !separating_features.is_empty()
        && scenario_provenance_complete(scenario)
    {
        (
            PairDisposition::Separable,
            None,
            Some(maximum_normalized_gap),
        )
    } else if !scenario_provenance_complete(scenario) {
        (
            PairDisposition::Unknown,
            Some("sensor-availability scenario lacks source/method evidence".to_owned()),
            None,
        )
    } else if considered_features == 0 {
        (
            PairDisposition::Unknown,
            Some("no diagnostic feature channel is available in this scenario".to_owned()),
            None,
        )
    } else if unknown_features > 0 {
        (
            PairDisposition::Unknown,
            Some("one or more available diagnostic feature bounds lack qualified provenance".to_owned()),
            None,
        )
    } else {
        (
            PairDisposition::NotSeparable,
            Some("all available validated feature intervals overlap for this hypothesis pair".to_owned()),
            Some(0.0),
        )
    };

    PairAssessment {
        scenario_id: scenario.scenario_id.clone(),
        left_hypothesis_id: left.hypothesis_id.clone(),
        right_hypothesis_id: right.hypothesis_id.clone(),
        disposition,
        separating_feature_ids: separating_features,
        maximum_normalized_gap: normalized_gap,
        reason,
    }
}

/// Evaluates every hypothesis pair under every required sensor-availability
/// scenario. Hard constraints are checked independently of diagnostic score.
pub fn evaluate_diagnostic_profile(
    profile: &DiagnosticProfile,
) -> Result<ProfileEvaluation, String> {
    validate_profile_structure(profile)?;

    let mut known_constraint_violations = Vec::new();
    let mut unknown_constraints = Vec::new();
    let expected = expected_constraints();

    for kind in expected {
        let margin = profile.hard_constraint_margins
            .iter()
            .find(|margin| margin.kind == kind);
        match margin {
            None => unknown_constraints.push(kind),
            Some(margin) if margin.evidence_class == EvidenceClass::Unknown
                || margin.source_reference.as_deref().is_none_or(|value| !nonempty(value))
                || margin.method.as_deref().is_none_or(|value| !nonempty(value))
                || margin.minimum_robust_margin.is_none() =>
            {
                unknown_constraints.push(kind);
            }
            Some(margin) if margin.minimum_robust_margin.unwrap() < 0.0 => {
                known_constraint_violations.push(kind);
            }
            Some(_) => {}
        }
    }

    let mut pair_assessments = Vec::new();
    for left_index in 0..profile.fault_signatures.len() {
        for right_index in (left_index + 1)..profile.fault_signatures.len() {
            let left = &profile.fault_signatures[left_index];
            let right = &profile.fault_signatures[right_index];
            for scenario in &profile.sensor_scenarios {
                pair_assessments.push(evaluate_pair(left, right, scenario));
            }
        }
    }

    let separable_count = pair_assessments
        .iter()
        .filter(|assessment| assessment.disposition == PairDisposition::Separable)
        .count();
    let nonseparable_count = pair_assessments
        .iter()
        .filter(|assessment| assessment.disposition == PairDisposition::NotSeparable)
        .count();
    let unknown_count = pair_assessments
        .iter()
        .filter(|assessment| assessment.disposition == PairDisposition::Unknown)
        .count();
    let total_count = pair_assessments.len();

    let pair_scenario_separation_coverage = if total_count == 0 {
        None
    } else {
        Some(separable_count as f64 / total_count as f64)
    };

    let worst_case_normalized_separation = if unknown_count > 0 {
        None
    } else if nonseparable_count > 0 {
        Some(0.0)
    } else {
        pair_assessments
            .iter()
            .filter_map(|assessment| assessment.maximum_normalized_gap)
            .min_by(f64::total_cmp)
    };

    let disposition = if !known_constraint_violations.is_empty() {
        ProfileDisposition::Infeasible
    } else if !unknown_constraints.is_empty() || unknown_count > 0 {
        ProfileDisposition::Unknown
    } else if nonseparable_count > 0 {
        ProfileDisposition::FeasibleButNotFullyDiagnosable
    } else {
        ProfileDisposition::FeasibleFullyDiagnosable
    };

    Ok(ProfileEvaluation {
        profile_id: profile.profile_id.clone(),
        profile_digest: profile.profile_digest.clone(),
        configuration_digest: profile.configuration_digest.clone(),
        system_boundary_digest: profile.system_boundary_digest.clone(),
        operating_condition_digest: profile.operating_condition_digest.clone(),
        mission_set_digest: profile.mission_set_digest.clone(),
        disposition,
        known_constraint_violations,
        unknown_constraints,
        hypothesis_pair_count: profile.fault_signatures.len() * (profile.fault_signatures.len() - 1) / 2,
        required_scenario_count: profile.sensor_scenarios.len(),
        separable_pair_scenario_count: separable_count,
        nonseparable_pair_scenario_count: nonseparable_count,
        unknown_pair_scenario_count: unknown_count,
        pair_scenario_separation_coverage,
        worst_case_normalized_separation,
        diagnostic_duration_s: profile.diagnostic_duration_s,
        electrical_energy_j: profile.electrical_energy_j,
        pair_assessments,
        interpretation: "Deterministic feature-interval diagnosability under the declared fault signatures and sensor-availability scenarios. It is not a physical safety proof, detector-performance result or authorization to execute an excitation.".to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROFILE: &str = "profile-v1";
    const CONFIG: &str = "joint-config-v1";
    const BOUNDARY: &str = "complete-joint-boundary-v1";
    const CONDITION: &str = "48V-25C-cooling-A";
    const MISSION: &str = "manipulation-mission-v1";

    fn constraint(kind: DiagnosticConstraintKind, margin: f64) -> ConstraintMargin {
        ConstraintMargin {
            kind,
            minimum_robust_margin: Some(margin),
            unit: match kind {
                DiagnosticConstraintKind::TaskPerformance => "fraction",
                DiagnosticConstraintKind::TorqueSpeed => "N m",
                DiagnosticConstraintKind::PeakDuration => "s",
                DiagnosticConstraintKind::ThermalHeadroom => "degC",
                DiagnosticConstraintKind::EnergyBudget => "J",
                DiagnosticConstraintKind::Passivity => "J",
                DiagnosticConstraintKind::Workspace => "rad",
                DiagnosticConstraintKind::ExposureTime => "s",
            }.to_owned(),
            evidence_class: EvidenceClass::SyntheticFixture,
            source_reference: Some(format!("https://example.invalid/margin/{kind:?}")),
            method: Some("Synthetic bounded margin oracle".to_owned()),
            configuration_digest: CONFIG.to_owned(),
            system_boundary_digest: BOUNDARY.to_owned(),
            operating_condition_digest: CONDITION.to_owned(),
            mission_set_digest: MISSION.to_owned(),
            profile_digest: PROFILE.to_owned(),
        }
    }

    fn feature(
        id: &str,
        channel: &str,
        modality: &str,
        lower: f64,
        upper: f64,
    ) -> DiagnosticFeatureInterval {
        DiagnosticFeatureInterval {
            feature_id: id.to_owned(),
            sensor_channel_id: channel.to_owned(),
            modality: modality.to_owned(),
            unit: "normalized_residual".to_owned(),
            interval: NumericInterval { lower, upper },
            separation_scale: 1.0,
            evidence_class: EvidenceClass::SyntheticFixture,
            source_reference: Some(format!("https://example.invalid/features/{id}")),
            method: Some("Synthetic known-answer signature interval".to_owned()),
            calibration_digest: Some(format!("cal-{channel}")),
            raw_trace_digest: Some(format!("raw-{id}")),
            configuration_digest: CONFIG.to_owned(),
            system_boundary_digest: BOUNDARY.to_owned(),
            operating_condition_digest: CONDITION.to_owned(),
            mission_set_digest: MISSION.to_owned(),
            profile_digest: PROFILE.to_owned(),
        }
    }

    fn scenario(id: &str, available: &[&str]) -> SensorAvailabilityScenario {
        SensorAvailabilityScenario {
            scenario_id: id.to_owned(),
            available_channel_ids: available.iter().map(|channel| (*channel).to_owned()).collect(),
            evidence_class: EvidenceClass::SyntheticFixture,
            source_reference: Some(format!("https://example.invalid/scenarios/{id}")),
            method: Some("Synthetic channel availability mask".to_owned()),
            configuration_digest: CONFIG.to_owned(),
            system_boundary_digest: BOUNDARY.to_owned(),
            operating_condition_digest: CONDITION.to_owned(),
            mission_set_digest: MISSION.to_owned(),
            profile_digest: PROFILE.to_owned(),
        }
    }

    fn signature(id: &str, values: [(f64, f64); 3]) -> FaultSignature {
        FaultSignature {
            hypothesis_id: id.to_owned(),
            features: vec![
                feature("vibration-residual", "vibration", "vibration", values[0].0, values[0].1),
                feature("servo-residual", "servo", "motor_servo", values[1].0, values[1].1),
                feature("torque-residual", "metrology", "torque_metrology", values[2].0, values[2].1),
            ],
        }
    }

    fn profile() -> DiagnosticProfile {
        DiagnosticProfile {
            profile_id: "diagnostic-pulse-set-A".to_owned(),
            profile_digest: PROFILE.to_owned(),
            configuration_digest: CONFIG.to_owned(),
            system_boundary_digest: BOUNDARY.to_owned(),
            operating_condition_digest: CONDITION.to_owned(),
            mission_set_digest: MISSION.to_owned(),
            diagnostic_duration_s: 2.0,
            electrical_energy_j: 12.0,
            hard_constraint_margins: expected_constraints()
                .into_iter()
                .map(|kind| constraint(kind, 1.0))
                .collect(),
            fault_signatures: vec![
                signature("healthy", [(-0.2, 0.2), (-0.1, 0.1), (-0.1, 0.1)]),
                signature("gearbox_backlash", [(1.0, 1.3), (-0.1, 0.1), (0.3, 0.5)]),
                signature("bearing_wear", [(-0.1, 0.1), (0.7, 1.0), (-0.1, 0.1)]),
            ],
            sensor_scenarios: vec![
                scenario("nominal", &["vibration", "servo", "metrology"]),
                scenario("vibration_dropout", &["servo", "metrology"]),
                scenario("servo_dropout", &["vibration", "metrology"]),
                scenario("metrology_dropout", &["vibration", "servo"]),
            ],
        }
    }

    fn pair<'a>(report: &'a ProfileEvaluation, scenario_id: &str, a: &str, b: &str) -> &'a PairAssessment {
        report.pair_assessments.iter().find(|assessment| {
            assessment.scenario_id == scenario_id
                && ((assessment.left_hypothesis_id == a && assessment.right_hypothesis_id == b)
                    || (assessment.left_hypothesis_id == b && assessment.right_hypothesis_id == a))
        }).unwrap()
    }

    fn two_signatures_mut<'a>(
        profile: &'a mut DiagnosticProfile,
        left_id: &str,
        right_id: &str,
    ) -> (&'a mut FaultSignature, &'a mut FaultSignature) {
        let left_index = profile.fault_signatures.iter()
            .position(|signature| signature.hypothesis_id == left_id)
            .unwrap();
        let right_index = profile.fault_signatures.iter()
            .position(|signature| signature.hypothesis_id == right_id)
            .unwrap();
        assert_ne!(left_index, right_index);
        if left_index < right_index {
            let (before_right, from_right) = profile.fault_signatures.split_at_mut(right_index);
            (&mut before_right[left_index], &mut from_right[0])
        } else {
            let (before_left, from_left) = profile.fault_signatures.split_at_mut(left_index);
            (&mut from_left[0], &mut before_left[right_index])
        }
    }

    #[test]
    fn diag_001_all_hypothesis_pairs_separate_under_nominal_and_declared_dropout_cases() {
        let mut profile = profile();
        // The first two dropouts retain at least one robust separating feature
        // for each pair; metrology dropout leaves vibration and servo channels.
        profile.sensor_scenarios = vec![
            scenario("nominal", &["vibration", "servo", "metrology"]),
            scenario("vibration_dropout", &["servo", "metrology"]),
            scenario("metrology_dropout", &["vibration", "servo"]),
        ];
        let result = evaluate_diagnostic_profile(&profile).unwrap();
        assert_eq!(result.disposition, ProfileDisposition::FeasibleFullyDiagnosable);
        assert_eq!(result.hypothesis_pair_count, 3);
        assert_eq!(result.required_scenario_count, 3);
        assert_eq!(result.separable_pair_scenario_count, 9);
        assert_eq!(result.nonseparable_pair_scenario_count, 0);
        assert_eq!(result.unknown_pair_scenario_count, 0);
        assert_eq!(result.pair_scenario_separation_coverage, Some(1.0));
        assert!(result.worst_case_normalized_separation.unwrap() > 0.0);
    }

    #[test]
    fn diag_002_channel_dropout_can_make_a_fault_pair_unidentifiable() {
        let mut profile = profile();
        profile.sensor_scenarios.push(scenario("vibration_and_servo_dropout", &["metrology"]));
        let result = evaluate_diagnostic_profile(&profile).unwrap();
        assert_eq!(result.disposition, ProfileDisposition::FeasibleButNotFullyDiagnosable);
        assert!(result.nonseparable_pair_scenario_count > 0);
        let healthy_bearing = pair(
            &result,
            "vibration_and_servo_dropout",
            "healthy",
            "bearing_wear",
        );
        assert_eq!(healthy_bearing.disposition, PairDisposition::NotSeparable);
    }

    #[test]
    fn diag_003_no_available_feature_channels_is_unknown_not_not_separable() {
        let mut profile = profile();
        profile.sensor_scenarios.push(scenario("all_channels_lost", &[]));
        let result = evaluate_diagnostic_profile(&profile).unwrap();
        assert_eq!(result.disposition, ProfileDisposition::Unknown);
        assert!(result.unknown_pair_scenario_count > 0);
    }

    #[test]
    fn diag_004_negative_task_or_thermal_margin_rejects_diagnostic_profile() {
        let mut profile = profile();
        profile.hard_constraint_margins.iter_mut()
            .find(|margin| margin.kind == DiagnosticConstraintKind::ThermalHeadroom)
            .unwrap()
            .minimum_robust_margin = Some(-0.1);
        let result = evaluate_diagnostic_profile(&profile).unwrap();
        assert_eq!(result.disposition, ProfileDisposition::Infeasible);
        assert_eq!(result.known_constraint_violations, vec![DiagnosticConstraintKind::ThermalHeadroom]);
    }

    #[test]
    fn diag_005_missing_or_unknown_hard_margin_is_unknown() {
        let mut missing = profile();
        missing.hard_constraint_margins.retain(|margin| margin.kind != DiagnosticConstraintKind::Passivity);
        let report = evaluate_diagnostic_profile(&missing).unwrap();
        assert_eq!(report.disposition, ProfileDisposition::Unknown);
        assert_eq!(report.unknown_constraints, vec![DiagnosticConstraintKind::Passivity]);

        let mut unknown = profile();
        let margin = unknown.hard_constraint_margins.iter_mut()
            .find(|margin| margin.kind == DiagnosticConstraintKind::EnergyBudget)
            .unwrap();
        margin.minimum_robust_margin = None;
        let report = evaluate_diagnostic_profile(&unknown).unwrap();
        assert_eq!(report.disposition, ProfileDisposition::Unknown);
    }

    #[test]
    fn diag_006_known_negative_margin_takes_precedence_over_unrelated_unknowns() {
        let mut profile = profile();
        profile.hard_constraint_margins.retain(|margin| margin.kind != DiagnosticConstraintKind::Passivity);
        profile.hard_constraint_margins.iter_mut()
            .find(|margin| margin.kind == DiagnosticConstraintKind::TorqueSpeed)
            .unwrap()
            .minimum_robust_margin = Some(-1.0);
        let report = evaluate_diagnostic_profile(&profile).unwrap();
        assert_eq!(report.disposition, ProfileDisposition::Infeasible);
        assert_eq!(report.unknown_constraints, vec![DiagnosticConstraintKind::Passivity]);
    }

    #[test]
    fn diag_007_overlapping_bounded_intervals_are_not_separable() {
        let mut profile = profile();
        let (healthy, bearing) = two_signatures_mut(&mut profile, "healthy", "bearing_wear");
        for (left, right) in healthy.features.iter_mut().zip(&mut bearing.features) {
            left.interval = NumericInterval { lower: -1.0, upper: 1.0 };
            right.interval = NumericInterval { lower: -0.5, upper: 0.5 };
        }
        let report = evaluate_diagnostic_profile(&profile).unwrap();
        assert_eq!(
            pair(&report, "nominal", "healthy", "bearing_wear").disposition,
            PairDisposition::NotSeparable
        );
    }

    #[test]
    fn diag_008_one_robust_separating_feature_is_sufficient_for_box_disjointness() {
        let mut profile = profile();
        let healthy = profile.fault_signatures.iter_mut()
            .find(|signature| signature.hypothesis_id == "healthy")
            .unwrap();
        let bearing = profile.fault_signatures.iter_mut()
            .find(|signature| signature.hypothesis_id == "bearing_wear")
            .unwrap();
        for feature_id in ["vibration-residual", "torque-residual"] {
            let left = healthy.features.iter_mut().find(|feature| feature.feature_id == feature_id).unwrap();
            let right = bearing.features.iter_mut().find(|feature| feature.feature_id == feature_id).unwrap();
            left.interval = NumericInterval { lower: -1.0, upper: 1.0 };
            right.interval = NumericInterval { lower: -0.5, upper: 0.5 };
        }
        let report = evaluate_diagnostic_profile(&profile).unwrap();
        assert_eq!(
            pair(&report, "nominal", "healthy", "bearing_wear").disposition,
            PairDisposition::Separable
        );
        assert_eq!(
            pair(&report, "nominal", "healthy", "bearing_wear").separating_feature_ids,
            vec!["servo-residual"]
        );
    }

    #[test]
    fn diag_009_unknown_feature_evidence_cannot_support_a_separation_claim() {
        let mut profile = profile();
        for signature in &mut profile.fault_signatures {
            for feature in &mut signature.features {
                if feature.sensor_channel_id == "vibration" {
                    feature.evidence_class = EvidenceClass::Unknown;
                }
            }
        }
        let report = evaluate_diagnostic_profile(&profile).unwrap();
        assert_eq!(
            pair(&report, "nominal", "healthy", "gearbox_backlash").disposition,
            PairDisposition::Separable
        );
        // Other valid channels can still provide an independent separating axis.
        assert_eq!(
            pair(&report, "nominal", "healthy", "bearing_wear").disposition,
            PairDisposition::Separable
        );
    }

    #[test]
    fn diag_010_unknown_feature_provenance_without_other_witness_yields_unknown() {
        let mut profile = profile();
        for signature in &mut profile.fault_signatures {
            for feature in &mut signature.features {
                if feature.sensor_channel_id != "vibration" {
                    feature.evidence_class = EvidenceClass::Unknown;
                }
            }
        }
        profile.sensor_scenarios = vec![scenario("non-vibration-only", &["servo", "metrology"])];
        let report = evaluate_diagnostic_profile(&profile).unwrap();
        assert_eq!(
            pair(&report, "non-vibration-only", "healthy", "gearbox_backlash").disposition,
            PairDisposition::Unknown
        );
    }

    #[test]
    fn diag_011_touching_intervals_have_zero_gap_and_are_not_separable() {
        let mut profile = profile();
        let (healthy, gearbox) = two_signatures_mut(&mut profile, "healthy", "gearbox_backlash");
        healthy.features[0].interval = NumericInterval { lower: 0.0, upper: 1.0 };
        gearbox.features[0].interval = NumericInterval { lower: 1.0, upper: 2.0 };
        healthy.features[1].interval = NumericInterval { lower: -1.0, upper: 1.0 };
        gearbox.features[1].interval = NumericInterval { lower: -1.0, upper: 1.0 };
        healthy.features[2].interval = NumericInterval { lower: -1.0, upper: 1.0 };
        gearbox.features[2].interval = NumericInterval { lower: -1.0, upper: 1.0 };
        let report = evaluate_diagnostic_profile(&profile).unwrap();
        assert_eq!(
            pair(&report, "nominal", "healthy", "gearbox_backlash").disposition,
            PairDisposition::NotSeparable
        );
    }

    #[test]
    fn diag_012_dimensionless_gap_uses_preregistered_feature_scale() {
        let mut profile = profile();
        for signature in &mut profile.fault_signatures {
            let feature = signature.features.iter_mut()
                .find(|feature| feature.feature_id == "vibration-residual")
                .unwrap();
            feature.separation_scale = 2.0;
        }
        let report = evaluate_diagnostic_profile(&profile).unwrap();
        let pair = pair(&report, "nominal", "healthy", "gearbox_backlash");
        assert_eq!(pair.maximum_normalized_gap, Some(0.4));
    }

    #[test]
    fn diag_013_missing_fault_hypothesis_or_healthy_reference_rejects() {
        let mut missing_healthy = profile();
        missing_healthy.fault_signatures.retain(|signature| signature.hypothesis_id != "healthy");
        assert!(evaluate_diagnostic_profile(&missing_healthy).is_err());

        let mut one_hypothesis = profile();
        one_hypothesis.fault_signatures.truncate(1);
        assert!(evaluate_diagnostic_profile(&one_hypothesis).is_err());
    }

    #[test]
    fn diag_014_duplicate_profile_feature_or_scenario_ids_reject() {
        let mut duplicate_feature = profile();
        let duplicated_feature = duplicate_feature.fault_signatures[0].features[0].clone();
        duplicate_feature.fault_signatures[0].features.push(duplicated_feature);
        assert!(evaluate_diagnostic_profile(&duplicate_feature).is_err());

        let mut duplicate_scenario = profile();
        let duplicated_scenario = duplicate_scenario.sensor_scenarios[0].clone();
        duplicate_scenario.sensor_scenarios.push(duplicated_scenario);
        assert!(evaluate_diagnostic_profile(&duplicate_scenario).is_err());
    }

    #[test]
    fn diag_015_digest_mismatch_rejects_signature_or_scenario_reuse() {
        let mut stale_feature = profile();
        stale_feature.fault_signatures[0].features[0].mission_set_digest = "other-mission".to_owned();
        assert!(evaluate_diagnostic_profile(&stale_feature).is_err());

        let mut stale_scenario = profile();
        stale_scenario.sensor_scenarios[0].profile_digest = "other-profile".to_owned();
        assert!(evaluate_diagnostic_profile(&stale_scenario).is_err());
    }

    #[test]
    fn diag_016_profiles_require_all_eight_hard_margin_kinds_for_a_feasible_result() {
        let mut profile = profile();
        profile.hard_constraint_margins.retain(|margin| margin.kind != DiagnosticConstraintKind::Workspace);
        let report = evaluate_diagnostic_profile(&profile).unwrap();
        assert_eq!(report.disposition, ProfileDisposition::Unknown);
        assert_eq!(report.unknown_constraints, vec![DiagnosticConstraintKind::Workspace]);
    }

    #[test]
    fn diag_017_feature_intervals_must_be_finite_and_ordered() {
        let mut profile = profile();
        profile.fault_signatures[0].features[0].interval =
            NumericInterval { lower: 2.0, upper: 1.0 };
        assert!(evaluate_diagnostic_profile(&profile).is_err());

        let mut profile = profile();
        profile.fault_signatures[0].features[0].interval.lower = f64::NAN;
        assert!(evaluate_diagnostic_profile(&profile).is_err());
    }

    #[test]
    fn diag_018_sensor_scenario_must_not_invent_a_channel() {
        let mut profile = profile();
        profile.sensor_scenarios[0].available_channel_ids.push("imaginary-sensor".to_owned());
        assert!(evaluate_diagnostic_profile(&profile).is_err());
    }

    #[test]
    fn diag_019_unit_or_modality_mismatch_across_hypotheses_rejects() {
        let mut profile = profile();
        profile.fault_signatures[1].features[0].unit = "g".to_owned();
        assert!(evaluate_diagnostic_profile(&profile).is_err());

        let mut profile = profile();
        profile.fault_signatures[1].features[0].modality = "acoustic".to_owned();
        assert!(evaluate_diagnostic_profile(&profile).is_err());
    }

    #[test]
    fn diag_020_duplicate_constraint_kind_rejects() {
        let mut profile = profile();
        let duplicate_margin = profile.hard_constraint_margins[0].clone();
        profile.hard_constraint_margins.push(duplicate_margin);
        assert!(evaluate_diagnostic_profile(&profile).is_err());
    }
}
