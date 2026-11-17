//! Uncertainty-bounded geometric placement reference for a fixed assembly workcell.
//!
//! This crate evaluates measured component poses against a revision-bound
//! geometric fixture. It does not accept production, establish functional
//! quality, authenticate evidence, command a robot, or authorize actuation.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

const QUATERNION_NORM_TOLERANCE: f64 = 1.0e-6;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkcellIdentity {
    pub workpiece_id: String,
    pub workpiece_revision: String,
    pub fixture_id: String,
    pub fixture_revision: String,
    pub frame_id: String,
    pub frame_revision: u64,
    pub tool_id: String,
    pub tool_mount_revision: String,
    pub calibration_id: String,
    pub calibration_revision: String,
    pub calibration_digest: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PartSpecification {
    pub component_id: String,
    pub target_position_m: [f64; 3],
    /// Unit quaternion in x, y, z, w order.
    pub target_orientation_xyzw: [f64; 4],
    pub target_seat_depth_m: f64,
    pub translation_tolerance_m: f64,
    pub orientation_tolerance_rad: f64,
    pub seat_depth_tolerance_m: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WorkcellProfile {
    pub identity: WorkcellIdentity,
    pub parts: Vec<PartSpecification>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlacementObservation {
    pub observation_id: String,
    pub evidence_ref: String,
    pub workpiece_id: String,
    pub workpiece_revision: String,
    pub fixture_id: String,
    pub fixture_revision: String,
    pub component_id: String,
    pub frame_id: String,
    pub frame_revision: u64,
    pub tool_id: String,
    pub tool_mount_revision: String,
    pub calibration_id: String,
    pub calibration_revision: String,
    pub calibration_digest: String,
    pub measured_position_m: [f64; 3],
    /// Unit quaternion in x, y, z, w order.
    pub measured_orientation_xyzw: [f64; 4],
    pub measured_seat_depth_m: f64,
    pub translation_uncertainty_m: f64,
    pub orientation_uncertainty_rad: f64,
    pub seat_depth_uncertainty_m: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PartDisposition {
    WithinTolerance,
    GeometricMismatch,
    Unknown,
    InvalidEvidence,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssemblyDisposition {
    WithinTolerance,
    GeometricMismatch,
    IncompleteAssembly,
    InvalidEvidence,
    Inconsistent,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PartAssessment {
    pub component_id: String,
    pub disposition: PartDisposition,
    pub observation_id: Option<String>,
    pub evidence_ref: Option<String>,
    pub translation_error_m: Option<f64>,
    pub translation_upper_bound_m: Option<f64>,
    pub translation_margin_m: Option<f64>,
    pub orientation_error_rad: Option<f64>,
    pub orientation_upper_bound_rad: Option<f64>,
    pub orientation_margin_rad: Option<f64>,
    pub seat_depth_error_m: Option<f64>,
    pub seat_depth_upper_bound_m: Option<f64>,
    pub seat_depth_margin_m: Option<f64>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AssemblyEvaluation {
    pub disposition: AssemblyDisposition,
    pub expected_parts: usize,
    pub observed_parts: usize,
    /// Always sorted by component ID, independent of input observation order.
    pub parts: Vec<PartAssessment>,
    pub errors: Vec<String>,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn finite_vec3(values: [f64; 3]) -> bool {
    values.iter().all(|value| value.is_finite())
}

fn quaternion_norm(values: [f64; 4]) -> f64 {
    values[0]
        .hypot(values[1])
        .hypot(values[2])
        .hypot(values[3])
}

fn validate_unit_quaternion(values: [f64; 4]) -> Result<(), String> {
    if !values.iter().all(|value| value.is_finite()) {
        return Err("quaternion contains a non-finite value".to_owned());
    }
    let norm = quaternion_norm(values);
    if !norm.is_finite() || norm <= f64::MIN_POSITIVE {
        return Err("quaternion has zero or invalid norm".to_owned());
    }
    if (norm - 1.0).abs() > QUATERNION_NORM_TOLERANCE {
        return Err(format!(
            "quaternion norm {norm} exceeds normalization tolerance"
        ));
    }
    Ok(())
}

fn validate_profile(profile: &WorkcellProfile) -> Result<(), String> {
    let identity = &profile.identity;
    if !nonempty(&identity.workpiece_id)
        || !nonempty(&identity.workpiece_revision)
        || !nonempty(&identity.fixture_id)
        || !nonempty(&identity.fixture_revision)
        || !nonempty(&identity.frame_id)
        || !nonempty(&identity.tool_id)
        || !nonempty(&identity.tool_mount_revision)
        || !nonempty(&identity.calibration_id)
        || !nonempty(&identity.calibration_revision)
    {
        return Err("workcell identity is incomplete".to_owned());
    }
    if !valid_digest(&identity.calibration_digest) {
        return Err("calibration digest must be lowercase SHA-256 hex".to_owned());
    }
    if profile.parts.is_empty() {
        return Err("workcell profile has no expected components".to_owned());
    }

    let mut component_ids = BTreeSet::new();
    for part in &profile.parts {
        if !nonempty(&part.component_id) {
            return Err("expected component identity is missing".to_owned());
        }
        if !component_ids.insert(part.component_id.as_str()) {
            return Err(format!(
                "duplicate expected component identity: {}",
                part.component_id
            ));
        }
        if !finite_vec3(part.target_position_m) || !part.target_seat_depth_m.is_finite() {
            return Err(format!(
                "component {} has a non-finite target pose",
                part.component_id
            ));
        }
        validate_unit_quaternion(part.target_orientation_xyzw).map_err(|error| {
            format!("component {} target orientation: {error}", part.component_id)
        })?;
        if !part.translation_tolerance_m.is_finite()
            || part.translation_tolerance_m <= 0.0
            || !part.orientation_tolerance_rad.is_finite()
            || part.orientation_tolerance_rad <= 0.0
            || !part.seat_depth_tolerance_m.is_finite()
            || part.seat_depth_tolerance_m <= 0.0
        {
            return Err(format!(
                "component {} has a nonpositive or non-finite tolerance",
                part.component_id
            ));
        }
    }
    Ok(())
}

fn validate_observation_identity(
    identity: &WorkcellIdentity,
    observation: &PlacementObservation,
) -> Result<(), String> {
    if !nonempty(&observation.observation_id) || !nonempty(&observation.evidence_ref) {
        return Err("observation identity or evidence reference is missing".to_owned());
    }
    if observation.workpiece_id != identity.workpiece_id
        || observation.workpiece_revision != identity.workpiece_revision
    {
        return Err("observation workpiece identity/revision does not match profile".to_owned());
    }
    if observation.fixture_id != identity.fixture_id
        || observation.fixture_revision != identity.fixture_revision
    {
        return Err("observation fixture identity/revision does not match profile".to_owned());
    }
    if observation.frame_id != identity.frame_id || observation.frame_revision != identity.frame_revision {
        return Err("observation frame identity/revision does not match profile".to_owned());
    }
    if observation.tool_id != identity.tool_id
        || observation.tool_mount_revision != identity.tool_mount_revision
    {
        return Err("observation tool or mount revision does not match profile".to_owned());
    }
    if observation.calibration_id != identity.calibration_id
        || observation.calibration_revision != identity.calibration_revision
        || observation.calibration_digest != identity.calibration_digest
    {
        return Err("observation calibration identity/revision/digest does not match profile".to_owned());
    }
    Ok(())
}

fn quaternion_angle_error(target: [f64; 4], measured: [f64; 4]) -> Result<f64, String> {
    validate_unit_quaternion(target)?;
    validate_unit_quaternion(measured)?;
    let dot = target[0] * measured[0]
        + target[1] * measured[1]
        + target[2] * measured[2]
        + target[3] * measured[3];
    if !dot.is_finite() {
        return Err("quaternion dot product is non-finite".to_owned());
    }
    let shortest_dot = dot.abs().clamp(0.0, 1.0);
    let angle = 2.0 * shortest_dot.acos();
    if !angle.is_finite() {
        return Err("orientation error is non-finite".to_owned());
    }
    Ok(angle)
}

fn translation_error(target: [f64; 3], measured: [f64; 3]) -> Result<f64, String> {
    if !finite_vec3(target) || !finite_vec3(measured) {
        return Err("translation contains a non-finite value".to_owned());
    }
    let dx = measured[0] - target[0];
    let dy = measured[1] - target[1];
    let dz = measured[2] - target[2];
    let error = dx.hypot(dy).hypot(dz);
    if !error.is_finite() {
        return Err("translation error overflowed".to_owned());
    }
    Ok(error)
}

fn invalid_assessment(component_id: &str, error: impl Into<String>) -> PartAssessment {
    PartAssessment {
        component_id: component_id.to_owned(),
        disposition: PartDisposition::InvalidEvidence,
        observation_id: None,
        evidence_ref: None,
        translation_error_m: None,
        translation_upper_bound_m: None,
        translation_margin_m: None,
        orientation_error_rad: None,
        orientation_upper_bound_rad: None,
        orientation_margin_rad: None,
        seat_depth_error_m: None,
        seat_depth_upper_bound_m: None,
        seat_depth_margin_m: None,
        error: Some(error.into()),
    }
}

fn assess_part(
    identity: &WorkcellIdentity,
    part: &PartSpecification,
    observation: &PlacementObservation,
) -> PartAssessment {
    if observation.component_id != part.component_id {
        return invalid_assessment(
            &part.component_id,
            "observation component ID does not match expected part",
        );
    }
    if let Err(error) = validate_observation_identity(identity, observation) {
        return invalid_assessment(&part.component_id, error);
    }
    if !finite_vec3(observation.measured_position_m)
        || !observation.measured_seat_depth_m.is_finite()
        || !observation.translation_uncertainty_m.is_finite()
        || observation.translation_uncertainty_m < 0.0
        || !observation.orientation_uncertainty_rad.is_finite()
        || observation.orientation_uncertainty_rad < 0.0
        || !observation.seat_depth_uncertainty_m.is_finite()
        || observation.seat_depth_uncertainty_m < 0.0
    {
        return invalid_assessment(
            &part.component_id,
            "measurement or uncertainty contains an invalid numeric value",
        );
    }

    let position = match translation_error(part.target_position_m, observation.measured_position_m) {
        Ok(value) => value,
        Err(error) => return invalid_assessment(&part.component_id, error),
    };
    let orientation = match quaternion_angle_error(
        part.target_orientation_xyzw,
        observation.measured_orientation_xyzw,
    ) {
        Ok(value) => value,
        Err(error) => return invalid_assessment(&part.component_id, error),
    };
    let seat = (observation.measured_seat_depth_m - part.target_seat_depth_m).abs();
    if !seat.is_finite() {
        return invalid_assessment(&part.component_id, "seat-depth error overflowed");
    }

    let position_upper = position + observation.translation_uncertainty_m;
    let orientation_upper = orientation + observation.orientation_uncertainty_rad;
    let seat_upper = seat + observation.seat_depth_uncertainty_m;
    if !position_upper.is_finite() || !orientation_upper.is_finite() || !seat_upper.is_finite() {
        return invalid_assessment(&part.component_id, "conservative error bound overflowed");
    }

    let position_margin = part.translation_tolerance_m - position_upper;
    let orientation_margin = part.orientation_tolerance_rad - orientation_upper;
    let seat_margin = part.seat_depth_tolerance_m - seat_upper;
    let disposition = if position_margin >= 0.0
        && orientation_margin >= 0.0
        && seat_margin >= 0.0
    {
        PartDisposition::WithinTolerance
    } else {
        PartDisposition::GeometricMismatch
    };

    PartAssessment {
        component_id: part.component_id.clone(),
        disposition,
        observation_id: Some(observation.observation_id.clone()),
        evidence_ref: Some(observation.evidence_ref.clone()),
        translation_error_m: Some(position),
        translation_upper_bound_m: Some(position_upper),
        translation_margin_m: Some(position_margin),
        orientation_error_rad: Some(orientation),
        orientation_upper_bound_rad: Some(orientation_upper),
        orientation_margin_rad: Some(orientation_margin),
        seat_depth_error_m: Some(seat),
        seat_depth_upper_bound_m: Some(seat_upper),
        seat_depth_margin_m: Some(seat_margin),
        error: None,
    }
}

/// Evaluate measured placements against the frozen workcell geometry.
///
/// This function returns a geometric disposition only. A WithinTolerance
/// result does not mean that the part was functionally inspected or accepted.
pub fn evaluate_placements(
    profile: &WorkcellProfile,
    observations: &[PlacementObservation],
) -> AssemblyEvaluation {
    if let Err(error) = validate_profile(profile) {
        return AssemblyEvaluation {
            disposition: AssemblyDisposition::InvalidEvidence,
            expected_parts: profile.parts.len(),
            observed_parts: observations.len(),
            parts: Vec::new(),
            errors: vec![error],
        };
    }

    let expected: BTreeMap<&str, &PartSpecification> = profile
        .parts
        .iter()
        .map(|part| (part.component_id.as_str(), part))
        .collect();
    let mut observation_ids = BTreeSet::new();
    let mut by_component = BTreeMap::<&str, &PlacementObservation>::new();
    let mut errors = Vec::new();

    for observation in observations {
        if !nonempty(&observation.observation_id) {
            errors.push("observation ID is missing".to_owned());
            continue;
        }
        if !observation_ids.insert(observation.observation_id.as_str()) {
            errors.push(format!(
                "duplicate observation ID: {}",
                observation.observation_id
            ));
            continue;
        }
        if !expected.contains_key(observation.component_id.as_str()) {
            errors.push(format!(
                "unexpected component ID: {}",
                observation.component_id
            ));
            continue;
        }
        if by_component
            .insert(observation.component_id.as_str(), observation)
            .is_some()
        {
            errors.push(format!(
                "multiple observations for component {}",
                observation.component_id
            ));
        }
    }

    let mut parts = Vec::with_capacity(profile.parts.len());
    let mut ordered_specs: Vec<&PartSpecification> = profile.parts.iter().collect();
    ordered_specs.sort_by(|left, right| left.component_id.cmp(&right.component_id));
    for part in ordered_specs {
        let assessment = match by_component.get(part.component_id.as_str()) {
            Some(observation) => assess_part(&profile.identity, part, observation),
            None => PartAssessment {
                component_id: part.component_id.clone(),
                disposition: PartDisposition::Unknown,
                observation_id: None,
                evidence_ref: None,
                translation_error_m: None,
                translation_upper_bound_m: None,
                translation_margin_m: None,
                orientation_error_rad: None,
                orientation_upper_bound_rad: None,
                orientation_margin_rad: None,
                seat_depth_error_m: None,
                seat_depth_upper_bound_m: None,
                seat_depth_margin_m: None,
                error: Some("required placement observation is missing".to_owned()),
            },
        };
        parts.push(assessment);
    }

    let has_invalid = parts
        .iter()
        .any(|part| part.disposition == PartDisposition::InvalidEvidence);
    let has_mismatch = parts
        .iter()
        .any(|part| part.disposition == PartDisposition::GeometricMismatch);
    let has_unknown = parts
        .iter()
        .any(|part| part.disposition == PartDisposition::Unknown);

    let disposition = if !errors.is_empty() {
        AssemblyDisposition::Inconsistent
    } else if has_invalid {
        AssemblyDisposition::InvalidEvidence
    } else if has_mismatch {
        AssemblyDisposition::GeometricMismatch
    } else if has_unknown {
        AssemblyDisposition::IncompleteAssembly
    } else {
        AssemblyDisposition::WithinTolerance
    };

    for part in &parts {
        if let Some(error) = &part.error {
            errors.push(format!("{}: {}", part.component_id, error));
        }
    }

    AssemblyEvaluation {
        disposition,
        expected_parts: profile.parts.len(),
        observed_parts: by_component.len(),
        parts,
        errors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CALIBRATION_DIGEST: &str =
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn identity() -> WorkcellIdentity {
        WorkcellIdentity {
            workpiece_id: "workpiece:article-01".to_owned(),
            workpiece_revision: "workpiece-r1".to_owned(),
            fixture_id: "fixture:tabletop-01".to_owned(),
            fixture_revision: "fixture-r3".to_owned(),
            frame_id: "cell:world".to_owned(),
            frame_revision: 12,
            tool_id: "gripper:parallel-jaw".to_owned(),
            tool_mount_revision: "mount-r2".to_owned(),
            calibration_id: "metrology:laser-tracker-01".to_owned(),
            calibration_revision: "cal-r5".to_owned(),
            calibration_digest: CALIBRATION_DIGEST.to_owned(),
        }
    }

    fn part(component_id: &str, target: [f64; 3]) -> PartSpecification {
        PartSpecification {
            component_id: component_id.to_owned(),
            target_position_m: target,
            target_orientation_xyzw: [0.0, 0.0, 0.0, 1.0],
            target_seat_depth_m: 0.020,
            translation_tolerance_m: 0.010,
            orientation_tolerance_rad: 0.010,
            seat_depth_tolerance_m: 0.002,
        }
    }

    fn profile() -> WorkcellProfile {
        WorkcellProfile {
            identity: identity(),
            parts: vec![
                part("component-A", [0.0, 0.0, 0.0]),
                part("component-B", [0.1, 0.0, 0.0]),
                part("component-C", [0.0, 0.1, 0.0]),
                part("component-D", [0.1, 0.1, 0.0]),
            ],
        }
    }

    fn observation(component_id: &str, position: [f64; 3]) -> PlacementObservation {
        let identity = identity();
        PlacementObservation {
            observation_id: format!("observation:{component_id}"),
            evidence_ref: format!("synthetic-evidence:{component_id}"),
            workpiece_id: identity.workpiece_id,
            workpiece_revision: identity.workpiece_revision,
            fixture_id: identity.fixture_id,
            fixture_revision: identity.fixture_revision,
            component_id: component_id.to_owned(),
            frame_id: identity.frame_id,
            frame_revision: identity.frame_revision,
            tool_id: identity.tool_id,
            tool_mount_revision: identity.tool_mount_revision,
            calibration_id: identity.calibration_id,
            calibration_revision: identity.calibration_revision,
            calibration_digest: identity.calibration_digest,
            measured_position_m: position,
            measured_orientation_xyzw: [0.0, 0.0, 0.0, 1.0],
            measured_seat_depth_m: 0.020,
            translation_uncertainty_m: 0.001,
            orientation_uncertainty_rad: 0.001,
            seat_depth_uncertainty_m: 0.0002,
        }
    }

    fn nominal_observations() -> Vec<PlacementObservation> {
        vec![
            observation("component-A", [0.0, 0.0, 0.0]),
            observation("component-B", [0.1, 0.0, 0.0]),
            observation("component-C", [0.0, 0.1, 0.0]),
            observation("component-D", [0.1, 0.1, 0.0]),
        ]
    }

    #[test]
    fn four_nominal_parts_meet_geometric_tolerances() {
        let result = evaluate_placements(&profile(), &nominal_observations());
        assert_eq!(result.disposition, AssemblyDisposition::WithinTolerance);
        assert_eq!(result.expected_parts, 4);
        assert_eq!(result.observed_parts, 4);
        assert!(result.parts.iter().all(|part| {
            part.disposition == PartDisposition::WithinTolerance
        }));
    }

    #[test]
    fn translation_at_tolerance_boundary_passes_when_uncertainty_fits() {
        let mut observations = nominal_observations();
        observations[0].measured_position_m[0] = 0.009;
        observations[0].translation_uncertainty_m = 0.001;
        let result = evaluate_placements(&profile(), &observations);
        assert_eq!(result.parts[0].disposition, PartDisposition::WithinTolerance);
    }

    #[test]
    fn translation_uncertainty_crossing_tolerance_rejects_geometry() {
        let mut observations = nominal_observations();
        observations[0].measured_position_m[0] = 0.008;
        observations[0].translation_uncertainty_m = 0.003;
        let result = evaluate_placements(&profile(), &observations);
        assert_eq!(result.parts[0].disposition, PartDisposition::GeometricMismatch);
        assert_eq!(result.disposition, AssemblyDisposition::GeometricMismatch);
    }

    #[test]
    fn quaternion_sign_inversion_is_the_same_orientation() {
        let mut observations = nominal_observations();
        observations[0].measured_orientation_xyzw = [0.0, 0.0, 0.0, -1.0];
        let result = evaluate_placements(&profile(), &observations);
        assert_eq!(result.parts[0].orientation_error_rad, Some(0.0));
        assert_eq!(result.parts[0].disposition, PartDisposition::WithinTolerance);
    }

    #[test]
    fn small_rotation_inside_tolerance_passes() {
        let mut observations = nominal_observations();
        let angle = 0.008;
        observations[0].measured_orientation_xyzw = [
            0.0,
            0.0,
            (angle / 2.0).sin(),
            (angle / 2.0).cos(),
        ];
        observations[0].orientation_uncertainty_rad = 0.001;
        let result = evaluate_placements(&profile(), &observations);
        assert_eq!(result.parts[0].disposition, PartDisposition::WithinTolerance);
    }

    #[test]
    fn rotation_outside_tolerance_rejects_geometry() {
        let mut observations = nominal_observations();
        let angle = 0.011;
        observations[0].measured_orientation_xyzw = [
            0.0,
            0.0,
            (angle / 2.0).sin(),
            (angle / 2.0).cos(),
        ];
        observations[0].orientation_uncertainty_rad = 0.0;
        let result = evaluate_placements(&profile(), &observations);
        assert_eq!(result.parts[0].disposition, PartDisposition::GeometricMismatch);
    }

    #[test]
    fn seat_depth_outside_tolerance_rejects_geometry() {
        let mut observations = nominal_observations();
        observations[0].measured_seat_depth_m += 0.003;
        observations[0].seat_depth_uncertainty_m = 0.0;
        let result = evaluate_placements(&profile(), &observations);
        assert_eq!(result.parts[0].disposition, PartDisposition::GeometricMismatch);
    }

    #[test]
    fn missing_component_observation_is_incomplete_not_accepted() {
        let mut observations = nominal_observations();
        observations.pop();
        let result = evaluate_placements(&profile(), &observations);
        assert_eq!(result.disposition, AssemblyDisposition::IncompleteAssembly);
        assert!(result.parts.iter().any(|part| part.disposition == PartDisposition::Unknown));
    }

    #[test]
    fn wrong_unrecognized_component_id_is_inconsistent() {
        let mut observations = nominal_observations();
        observations[0].component_id = "component-lookalike".to_owned();
        let result = evaluate_placements(&profile(), &observations);
        assert_eq!(result.disposition, AssemblyDisposition::Inconsistent);
        assert!(result.errors.iter().any(|error| error.contains("unexpected component ID")));
    }

    #[test]
    fn duplicate_component_observations_are_inconsistent() {
        let mut observations = nominal_observations();
        let mut duplicate = observations[0].clone();
        duplicate.observation_id = "observation:duplicate".to_owned();
        observations.push(duplicate);
        let result = evaluate_placements(&profile(), &observations);
        assert_eq!(result.disposition, AssemblyDisposition::Inconsistent);
    }

    #[test]
    fn duplicate_observation_identity_is_inconsistent() {
        let mut observations = nominal_observations();
        observations[1].observation_id = observations[0].observation_id.clone();
        let result = evaluate_placements(&profile(), &observations);
        assert_eq!(result.disposition, AssemblyDisposition::Inconsistent);
    }

    #[test]
    fn stale_fixture_revision_is_invalid_evidence() {
        let mut observations = nominal_observations();
        observations[0].fixture_revision = "fixture-old".to_owned();
        let result = evaluate_placements(&profile(), &observations);
        assert_eq!(result.parts[0].disposition, PartDisposition::InvalidEvidence);
        assert_eq!(result.disposition, AssemblyDisposition::InvalidEvidence);
    }

    #[test]
    fn stale_workpiece_revision_is_invalid_evidence() {
        let mut observations = nominal_observations();
        observations[0].workpiece_revision = "workpiece-old".to_owned();
        let result = evaluate_placements(&profile(), &observations);
        assert_eq!(result.parts[0].disposition, PartDisposition::InvalidEvidence);
    }

    #[test]
    fn tool_or_mount_mismatch_is_invalid_evidence() {
        let mut observations = nominal_observations();
        observations[0].tool_mount_revision = "mount-old".to_owned();
        let result = evaluate_placements(&profile(), &observations);
        assert_eq!(result.parts[0].disposition, PartDisposition::InvalidEvidence);
    }

    #[test]
    fn frame_mismatch_is_invalid_evidence() {
        let mut observations = nominal_observations();
        observations[0].frame_revision += 1;
        let result = evaluate_placements(&profile(), &observations);
        assert_eq!(result.parts[0].disposition, PartDisposition::InvalidEvidence);
    }

    #[test]
    fn calibration_mismatch_is_invalid_evidence() {
        let mut observations = nominal_observations();
        observations[0].calibration_digest = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_owned();
        let result = evaluate_placements(&profile(), &observations);
        assert_eq!(result.parts[0].disposition, PartDisposition::InvalidEvidence);
    }

    #[test]
    fn zero_norm_quaternion_is_invalid_evidence() {
        let mut observations = nominal_observations();
        observations[0].measured_orientation_xyzw = [0.0, 0.0, 0.0, 0.0];
        let result = evaluate_placements(&profile(), &observations);
        assert_eq!(result.parts[0].disposition, PartDisposition::InvalidEvidence);
    }

    #[test]
    fn non_unit_quaternion_outside_tolerance_is_invalid_evidence() {
        let mut observations = nominal_observations();
        observations[0].measured_orientation_xyzw = [0.0, 0.0, 0.0, 1.01];
        let result = evaluate_placements(&profile(), &observations);
        assert_eq!(result.parts[0].disposition, PartDisposition::InvalidEvidence);
    }

    #[test]
    fn non_finite_pose_and_negative_uncertainty_are_invalid_evidence() {
        let mut observations = nominal_observations();
        observations[0].measured_position_m[0] = f64::NAN;
        let result = evaluate_placements(&profile(), &observations);
        assert_eq!(result.parts[0].disposition, PartDisposition::InvalidEvidence);

        let mut observations = nominal_observations();
        observations[0].translation_uncertainty_m = -0.001;
        let result = evaluate_placements(&profile(), &observations);
        assert_eq!(result.parts[0].disposition, PartDisposition::InvalidEvidence);
    }

    #[test]
    fn observation_order_does_not_change_assessment() {
        let observations = nominal_observations();
        let mut reversed = observations.clone();
        reversed.reverse();
        assert_eq!(
            evaluate_placements(&profile(), &observations),
            evaluate_placements(&profile(), &reversed)
        );
    }

    #[test]
    fn missing_evaluator_observation_cannot_be_replaced_by_robot_self_report() {
        // The API has no robot-reported "placed" input. Missing measured evidence stays Unknown.
        let result = evaluate_placements(&profile(), &[]);
        assert_eq!(result.disposition, AssemblyDisposition::IncompleteAssembly);
        assert_eq!(result.observed_parts, 0);
    }
}
