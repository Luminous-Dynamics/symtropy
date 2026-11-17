//! Independent field-layout acceptance scoring.
//!
//! Robot progress and confidence claims are observations only. This module
//! derives each entity's acceptance state from an evaluator-owned design
//! manifest, referee measurements and a frozen acceptance profile. It does not
//! parse JSON, authorize robot motion, or replace site safety assessment.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldLayoutEntityType {
    Line,
    Point,
    Arc,
    Text,
    Opening,
    Elevation,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RobotClaimedEntityState {
    Printed,
    NotPrinted,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeasurementDisposition {
    Measured,
    Unresolved,
    NotMeasured,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefereeEntityState {
    Accepted,
    Rejected,
    Held,
    Waived,
    NotAttempted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntityScoreReason {
    WithinFrozenAcceptanceProfile,
    GeometryOutsideTolerance,
    ReferenceResolutionInsufficient,
    MeasurementUnavailable,
    RequiredEndpointMissing,
    ExplicitHumanWaiver,
    NotAttempted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesignIntentEntity {
    pub design_entity_id: String,
    pub entity_type: FieldLayoutEntityType,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesignIntentManifest {
    pub package_sha256: String,
    pub design_revision_sha256: String,
    pub frame_id: String,
    pub entity_manifest_sha256: String,
    pub entities: Vec<DesignIntentEntity>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RefereeMeasurement {
    /// Signed centerline/anchor residuals in the frozen site frame, in mm.
    pub residual_x_mm: f64,
    pub residual_y_mm: f64,
    pub residual_z_mm: Option<f64>,
    /// Absolute start/end residuals for line-like entities, in mm.
    pub endpoint_start_mm: Option<f64>,
    pub endpoint_end_mm: Option<f64>,
    /// Independent reference method expanded uncertainties (U95), in mm.
    pub reference_u95_x_mm: f64,
    pub reference_u95_y_mm: f64,
    pub reference_u95_z_mm: Option<f64>,
    /// Digest binding the measurement to its raw referee receipt.
    pub measurement_receipt_sha256: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FieldEntityObservation {
    pub design_entity_id: String,
    /// Kept for audit only. Never used as acceptance truth.
    pub robot_claimed_state: RobotClaimedEntityState,
    pub measurement_disposition: MeasurementDisposition,
    pub measurement: Option<RefereeMeasurement>,
    /// A present rework event means this entity was not accepted on first pass.
    pub rework_event_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HumanWaiver {
    pub design_entity_id: String,
    pub authority_id: String,
    pub authority_role: String,
    pub reason: String,
    pub scope: String,
    pub waiver_receipt_sha256: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FieldAcceptanceProfile {
    pub max_abs_xy_residual_mm: f64,
    pub max_reference_u95_xy_mm: f64,
    /// If set, the referee must provide Z residual and U95 within these limits.
    pub max_abs_z_residual_mm: Option<f64>,
    pub max_reference_u95_z_mm: Option<f64>,
    /// If set, line-like entities must include both endpoint residuals.
    pub max_abs_endpoint_residual_mm: Option<f64>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CrewHours {
    pub bim_vdc: f64,
    pub coordination: f64,
    pub survey_control: f64,
    pub setup_calibration: f64,
    pub stationing: f64,
    pub execution_supervision: f64,
    pub inspection: f64,
    pub recovery: f64,
    pub rework: f64,
    pub other: f64,
}

impl CrewHours {
    pub fn total(self) -> f64 {
        self.bim_vdc
            + self.coordination
            + self.survey_control
            + self.setup_calibration
            + self.stationing
            + self.execution_supervision
            + self.inspection
            + self.recovery
            + self.rework
            + self.other
    }

    fn all_finite_nonnegative(self) -> bool {
        [
            self.bim_vdc,
            self.coordination,
            self.survey_control,
            self.setup_calibration,
            self.stationing,
            self.execution_supervision,
            self.inspection,
            self.recovery,
            self.rework,
            self.other,
        ]
        .into_iter()
        .all(|value| value.is_finite() && value >= 0.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldAcceptanceError {
    EmptyManifestIdentity,
    EmptyEntityManifest,
    EmptyEntityId,
    DuplicateManifestEntityId(String),
    DuplicateObservation(String),
    UnknownObservationEntity(String),
    DuplicateWaiver(String),
    UnknownWaiverEntity(String),
    InvalidAcceptanceProfile,
    InvalidCrewHours,
    InvalidMeasurement(String),
    MeasurementDispositionPayloadMismatch(String),
    InvalidWaiver(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntityAcceptanceResult {
    pub design_entity_id: String,
    pub entity_type: FieldLayoutEntityType,
    pub robot_claimed_state: RobotClaimedEntityState,
    pub referee_state: RefereeEntityState,
    pub reason: EntityScoreReason,
    pub rework_event_id: Option<String>,
    pub measurement_receipt_sha256: Option<String>,
    pub waiver_receipt_sha256: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FieldAcceptanceScore {
    pub package_sha256: String,
    pub design_revision_sha256: String,
    pub frame_id: String,
    pub entity_manifest_sha256: String,
    pub design_entity_count: u64,
    pub accepted_count: u64,
    pub rejected_count: u64,
    pub held_count: u64,
    pub waived_count: u64,
    pub not_attempted_count: u64,
    pub attempted_count: u64,
    pub first_pass_accepted_count: u64,
    pub rework_entity_count: u64,
    pub crew_hours: CrewHours,
    pub total_crew_hours: f64,
    pub accepted_entities_per_crew_hour: Option<f64>,
    pub first_pass_yield: Option<f64>,
    pub rework_crew_hour_fraction: Option<f64>,
    pub entity_results: Vec<EntityAcceptanceResult>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MeasurementEvaluation {
    Accepted,
    Rejected,
    Held(EntityScoreReason),
}

/// Independently score every design-intent ID.
///
/// A missing measurement never produces ACCEPTED. A robot PRINTED claim with
/// missing/unresolved referee evidence becomes HELD. A confirmed geometry
/// violation is REJECTED. Inadequate reference uncertainty is HELD rather than
/// misclassified as a geometric failure. Human waivers remain WAIVED and never
/// contribute to the accepted-output numerator.
pub fn score_field_layout_run(
    manifest: &DesignIntentManifest,
    observations: &[FieldEntityObservation],
    waivers: &[HumanWaiver],
    profile: FieldAcceptanceProfile,
    crew_hours: CrewHours,
) -> Result<FieldAcceptanceScore, FieldAcceptanceError> {
    validate_manifest(manifest)?;
    validate_profile(profile)?;
    if !crew_hours.all_finite_nonnegative() || !crew_hours.total().is_finite() {
        return Err(FieldAcceptanceError::InvalidCrewHours);
    }

    let manifest_by_id: BTreeMap<&str, &DesignIntentEntity> = manifest
        .entities
        .iter()
        .map(|entity| (entity.design_entity_id.as_str(), entity))
        .collect();

    let mut observations_by_id = BTreeMap::new();
    for observation in observations {
        if !manifest_by_id.contains_key(observation.design_entity_id.as_str()) {
            return Err(FieldAcceptanceError::UnknownObservationEntity(
                observation.design_entity_id.clone(),
            ));
        }
        if observations_by_id
            .insert(observation.design_entity_id.as_str(), observation)
            .is_some()
        {
            return Err(FieldAcceptanceError::DuplicateObservation(
                observation.design_entity_id.clone(),
            ));
        }
        validate_observation(observation)?;
    }

    let mut waivers_by_id = BTreeMap::new();
    for waiver in waivers {
        if !manifest_by_id.contains_key(waiver.design_entity_id.as_str()) {
            return Err(FieldAcceptanceError::UnknownWaiverEntity(
                waiver.design_entity_id.clone(),
            ));
        }
        if waivers_by_id
            .insert(waiver.design_entity_id.as_str(), waiver)
            .is_some()
        {
            return Err(FieldAcceptanceError::DuplicateWaiver(
                waiver.design_entity_id.clone(),
            ));
        }
        if [
            waiver.authority_id.as_str(),
            waiver.authority_role.as_str(),
            waiver.reason.as_str(),
            waiver.scope.as_str(),
            waiver.waiver_receipt_sha256.as_str(),
        ]
        .into_iter()
        .any(|value| value.trim().is_empty())
        {
            return Err(FieldAcceptanceError::InvalidWaiver(
                waiver.design_entity_id.clone(),
            ));
        }
    }

    let mut results = Vec::with_capacity(manifest.entities.len());
    let mut accepted_count = 0_u64;
    let mut rejected_count = 0_u64;
    let mut held_count = 0_u64;
    let mut waived_count = 0_u64;
    let mut not_attempted_count = 0_u64;
    let mut attempted_count = 0_u64;
    let mut first_pass_accepted_count = 0_u64;
    let mut rework_entity_count = 0_u64;

    for entity in &manifest.entities {
        let observation = observations_by_id
            .get(entity.design_entity_id.as_str())
            .copied();
        let waiver = waivers_by_id.get(entity.design_entity_id.as_str()).copied();

        let robot_claimed_state = observation
            .map(|item| item.robot_claimed_state)
            .unwrap_or(RobotClaimedEntityState::Unknown);
        let rework_event_id = observation.and_then(|item| item.rework_event_id.clone());
        if rework_event_id.is_some() {
            rework_entity_count += 1;
        }

        let measurement_receipt_sha256 = observation
            .and_then(|item| item.measurement)
            .map(|measurement| measurement.measurement_receipt_sha256.clone());

        let (referee_state, reason, waiver_receipt_sha256) = if let Some(waiver) = waiver {
            (
                RefereeEntityState::Waived,
                EntityScoreReason::ExplicitHumanWaiver,
                Some(waiver.waiver_receipt_sha256.clone()),
            )
        } else {
            match observation {
                None => (
                    RefereeEntityState::NotAttempted,
                    EntityScoreReason::NotAttempted,
                    None,
                ),
                Some(item) => match item.measurement_disposition {
                    MeasurementDisposition::NotMeasured => {
                        if item.robot_claimed_state == RobotClaimedEntityState::NotPrinted {
                            (
                                RefereeEntityState::NotAttempted,
                                EntityScoreReason::NotAttempted,
                                None,
                            )
                        } else {
                            (
                                RefereeEntityState::Held,
                                EntityScoreReason::MeasurementUnavailable,
                                None,
                            )
                        }
                    }
                    MeasurementDisposition::Unresolved => (
                        RefereeEntityState::Held,
                        EntityScoreReason::MeasurementUnavailable,
                        None,
                    ),
                    MeasurementDisposition::Measured => {
                        let measurement = item.measurement.ok_or_else(|| {
                            FieldAcceptanceError::MeasurementDispositionPayloadMismatch(
                                item.design_entity_id.clone(),
                            )
                        })?;
                        match evaluate_measurement(entity.entity_type, measurement, profile) {
                            MeasurementEvaluation::Accepted => (
                                RefereeEntityState::Accepted,
                                EntityScoreReason::WithinFrozenAcceptanceProfile,
                                None,
                            ),
                            MeasurementEvaluation::Rejected => (
                                RefereeEntityState::Rejected,
                                EntityScoreReason::GeometryOutsideTolerance,
                                None,
                            ),
                            MeasurementEvaluation::Held(reason) => {
                                (RefereeEntityState::Held, reason, None)
                            }
                        }
                    }
                },
            }
        };

        let attempted = observation.is_some_and(|item| {
            item.robot_claimed_state == RobotClaimedEntityState::Printed
                || item.measurement_disposition != MeasurementDisposition::NotMeasured
        });
        if attempted {
            attempted_count += 1;
        }

        match referee_state {
            RefereeEntityState::Accepted => {
                accepted_count += 1;
                if rework_event_id.is_none() {
                    first_pass_accepted_count += 1;
                }
            }
            RefereeEntityState::Rejected => rejected_count += 1,
            RefereeEntityState::Held => held_count += 1,
            RefereeEntityState::Waived => waived_count += 1,
            RefereeEntityState::NotAttempted => not_attempted_count += 1,
        }

        results.push(EntityAcceptanceResult {
            design_entity_id: entity.design_entity_id.clone(),
            entity_type: entity.entity_type,
            robot_claimed_state,
            referee_state,
            reason,
            rework_event_id,
            measurement_receipt_sha256,
            waiver_receipt_sha256,
        });
    }

    let total_crew_hours = crew_hours.total();
    let accepted_entities_per_crew_hour =
        (total_crew_hours > 0.0).then_some(accepted_count as f64 / total_crew_hours);
    let first_pass_yield =
        (attempted_count > 0).then_some(first_pass_accepted_count as f64 / attempted_count as f64);
    let total_rework_hours = crew_hours.rework;
    let rework_crew_hour_fraction =
        (total_crew_hours > 0.0).then_some(total_rework_hours / total_crew_hours);

    Ok(FieldAcceptanceScore {
        package_sha256: manifest.package_sha256.clone(),
        design_revision_sha256: manifest.design_revision_sha256.clone(),
        frame_id: manifest.frame_id.clone(),
        entity_manifest_sha256: manifest.entity_manifest_sha256.clone(),
        design_entity_count: manifest.entities.len() as u64,
        accepted_count,
        rejected_count,
        held_count,
        waived_count,
        not_attempted_count,
        attempted_count,
        first_pass_accepted_count,
        rework_entity_count,
        crew_hours,
        total_crew_hours,
        accepted_entities_per_crew_hour,
        first_pass_yield,
        rework_crew_hour_fraction,
        entity_results: results,
    })
}

fn validate_manifest(manifest: &DesignIntentManifest) -> Result<(), FieldAcceptanceError> {
    if [
        manifest.package_sha256.as_str(),
        manifest.design_revision_sha256.as_str(),
        manifest.frame_id.as_str(),
        manifest.entity_manifest_sha256.as_str(),
    ]
    .into_iter()
    .any(|value| value.trim().is_empty())
    {
        return Err(FieldAcceptanceError::EmptyManifestIdentity);
    }
    if manifest.entities.is_empty() {
        return Err(FieldAcceptanceError::EmptyEntityManifest);
    }
    let mut ids = BTreeSet::new();
    for entity in &manifest.entities {
        if entity.design_entity_id.trim().is_empty() {
            return Err(FieldAcceptanceError::EmptyEntityId);
        }
        if !ids.insert(entity.design_entity_id.as_str()) {
            return Err(FieldAcceptanceError::DuplicateManifestEntityId(
                entity.design_entity_id.clone(),
            ));
        }
    }
    Ok(())
}

fn validate_profile(profile: FieldAcceptanceProfile) -> Result<(), FieldAcceptanceError> {
    let valid_positive = |value: f64| value.is_finite() && value > 0.0;
    let valid_nonnegative = |value: f64| value.is_finite() && value >= 0.0;
    if !valid_positive(profile.max_abs_xy_residual_mm)
        || !valid_nonnegative(profile.max_reference_u95_xy_mm)
        || profile
            .max_abs_z_residual_mm
            .is_some_and(|value| !valid_positive(value))
        || profile
            .max_reference_u95_z_mm
            .is_some_and(|value| !valid_nonnegative(value))
        || profile
            .max_abs_endpoint_residual_mm
            .is_some_and(|value| !valid_positive(value))
        || profile.max_abs_z_residual_mm.is_some() != profile.max_reference_u95_z_mm.is_some()
    {
        return Err(FieldAcceptanceError::InvalidAcceptanceProfile);
    }
    Ok(())
}

fn validate_observation(observation: &FieldEntityObservation) -> Result<(), FieldAcceptanceError> {
    match (
        observation.measurement_disposition,
        observation.measurement.as_ref(),
    ) {
        (MeasurementDisposition::Measured, None) => {
            return Err(FieldAcceptanceError::MeasurementDispositionPayloadMismatch(
                observation.design_entity_id.clone(),
            ));
        }
        (MeasurementDisposition::NotMeasured, Some(_))
        | (MeasurementDisposition::Unresolved, Some(_)) => {
            return Err(FieldAcceptanceError::MeasurementDispositionPayloadMismatch(
                observation.design_entity_id.clone(),
            ));
        }
        _ => {}
    }
    if observation
        .rework_event_id
        .as_ref()
        .is_some_and(|value| value.trim().is_empty())
    {
        return Err(FieldAcceptanceError::InvalidMeasurement(
            observation.design_entity_id.clone(),
        ));
    }
    if let Some(measurement) = observation.measurement.as_ref() {
        let finite = [
            measurement.residual_x_mm,
            measurement.residual_y_mm,
            measurement.reference_u95_x_mm,
            measurement.reference_u95_y_mm,
        ]
        .into_iter()
        .all(f64::is_finite);
        let optionals_finite = [
            measurement.residual_z_mm,
            measurement.endpoint_start_mm,
            measurement.endpoint_end_mm,
            measurement.reference_u95_z_mm,
        ]
        .into_iter()
        .flatten()
        .all(f64::is_finite);
        let uncertainties_nonnegative = [
            Some(measurement.reference_u95_x_mm),
            Some(measurement.reference_u95_y_mm),
            measurement.reference_u95_z_mm,
        ]
        .into_iter()
        .flatten()
        .all(|value| value >= 0.0);
        if !finite
            || !optionals_finite
            || !uncertainties_nonnegative
            || measurement.measurement_receipt_sha256.trim().is_empty()
        {
            return Err(FieldAcceptanceError::InvalidMeasurement(
                observation.design_entity_id.clone(),
            ));
        }
    }
    Ok(())
}

fn evaluate_measurement(
    entity_type: FieldLayoutEntityType,
    measurement: RefereeMeasurement,
    profile: FieldAcceptanceProfile,
) -> MeasurementEvaluation {
    if measurement.reference_u95_x_mm > profile.max_reference_u95_xy_mm
        || measurement.reference_u95_y_mm > profile.max_reference_u95_xy_mm
    {
        return MeasurementEvaluation::Held(EntityScoreReason::ReferenceResolutionInsufficient);
    }
    if let Some(max_u95_z) = profile.max_reference_u95_z_mm {
        if measurement
            .reference_u95_z_mm
            .is_none_or(|value| value > max_u95_z)
        {
            return MeasurementEvaluation::Held(EntityScoreReason::ReferenceResolutionInsufficient);
        }
    }

    if measurement.residual_x_mm.hypot(measurement.residual_y_mm) > profile.max_abs_xy_residual_mm
    {
        return MeasurementEvaluation::Rejected;
    }

    if let Some(max_z) = profile.max_abs_z_residual_mm {
        if measurement
            .residual_z_mm
            .is_none_or(|value| value.abs() > max_z)
        {
            return if measurement.residual_z_mm.is_none() {
                MeasurementEvaluation::Held(EntityScoreReason::MeasurementUnavailable)
            } else {
                MeasurementEvaluation::Rejected
            };
        }
    }

    if profile.max_abs_endpoint_residual_mm.is_some() && entity_type == FieldLayoutEntityType::Line
    {
        let Some(start) = measurement.endpoint_start_mm else {
            return MeasurementEvaluation::Held(EntityScoreReason::RequiredEndpointMissing);
        };
        let Some(end) = measurement.endpoint_end_mm else {
            return MeasurementEvaluation::Held(EntityScoreReason::RequiredEndpointMissing);
        };
        let threshold = profile
            .max_abs_endpoint_residual_mm
            .unwrap_or(f64::INFINITY);
        if start.abs() > threshold || end.abs() > threshold {
            return MeasurementEvaluation::Rejected;
        }
    }

    MeasurementEvaluation::Accepted
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(ids: &[&str]) -> DesignIntentManifest {
        DesignIntentManifest {
            package_sha256: "package-digest".into(),
            design_revision_sha256: "revision-digest".into(),
            frame_id: "site-world".into(),
            entity_manifest_sha256: "manifest-digest".into(),
            entities: ids
                .iter()
                .enumerate()
                .map(|(index, id)| DesignIntentEntity {
                    design_entity_id: (*id).into(),
                    entity_type: if index == 0 {
                        FieldLayoutEntityType::Line
                    } else {
                        FieldLayoutEntityType::Point
                    },
                })
                .collect(),
        }
    }

    fn profile() -> FieldAcceptanceProfile {
        FieldAcceptanceProfile {
            max_abs_xy_residual_mm: 1.0,
            max_reference_u95_xy_mm: 0.25,
            max_abs_z_residual_mm: None,
            max_reference_u95_z_mm: None,
            max_abs_endpoint_residual_mm: Some(1.0),
        }
    }

    fn measurement(x: f64, y: f64, u95: f64) -> RefereeMeasurement {
        RefereeMeasurement {
            residual_x_mm: x,
            residual_y_mm: y,
            residual_z_mm: None,
            endpoint_start_mm: Some(0.2),
            endpoint_end_mm: Some(0.3),
            reference_u95_x_mm: u95,
            reference_u95_y_mm: u95,
            reference_u95_z_mm: None,
            measurement_receipt_sha256: "measurement-receipt".into(),
        }
    }

    fn observed(
        id: &str,
        robot_claim: RobotClaimedEntityState,
        value: RefereeMeasurement,
    ) -> FieldEntityObservation {
        FieldEntityObservation {
            design_entity_id: id.into(),
            robot_claimed_state: robot_claim,
            measurement_disposition: MeasurementDisposition::Measured,
            measurement: Some(value),
            rework_event_id: None,
        }
    }

    fn crew() -> CrewHours {
        CrewHours {
            bim_vdc: 0.25,
            coordination: 0.1,
            survey_control: 0.2,
            setup_calibration: 0.15,
            stationing: 0.1,
            execution_supervision: 0.1,
            inspection: 0.1,
            recovery: 0.0,
            rework: 0.05,
            other: 0.0,
        }
    }

    #[test]
    fn robot_claim_cannot_override_referee_rejection_and_whole_workflow_is_counted() {
        let manifest = manifest(&["line-1", "point-1", "line-2", "point-2"]);
        let observations = [
            observed(
                "line-1",
                RobotClaimedEntityState::Printed,
                measurement(0.3, 0.2, 0.1),
            ),
            observed(
                "point-1",
                RobotClaimedEntityState::Printed,
                measurement(1.4, -1.1, 0.1),
            ),
            FieldEntityObservation {
                design_entity_id: "line-2".into(),
                robot_claimed_state: RobotClaimedEntityState::Printed,
                measurement_disposition: MeasurementDisposition::NotMeasured,
                measurement: None,
                rework_event_id: None,
            },
            FieldEntityObservation {
                design_entity_id: "point-2".into(),
                robot_claimed_state: RobotClaimedEntityState::NotPrinted,
                measurement_disposition: MeasurementDisposition::NotMeasured,
                measurement: None,
                rework_event_id: None,
            },
        ];
        let result =
            score_field_layout_run(&manifest, &observations, &[], profile(), crew()).unwrap();
        assert_eq!(result.design_entity_count, 4);
        assert_eq!(result.accepted_count, 1);
        assert_eq!(result.rejected_count, 1);
        assert_eq!(result.held_count, 1);
        assert_eq!(result.not_attempted_count, 1);
        assert_eq!(result.attempted_count, 3);
        assert_eq!(result.first_pass_accepted_count, 1);
        assert!((result.total_crew_hours - 1.05).abs() < 1.0e-12);
        assert!((result.accepted_entities_per_crew_hour.unwrap() - (1.0 / 1.05)).abs() < 1.0e-12);
        assert!((result.first_pass_yield.unwrap() - (1.0 / 3.0)).abs() < 1.0e-12);
        assert_eq!(
            result.entity_results[1].referee_state,
            RefereeEntityState::Rejected
        );
    }

    #[test]
    fn insufficient_reference_resolution_is_held_not_accepted_or_rejected() {
        let result = score_field_layout_run(
            &manifest(&["point-1"]),
            &[observed(
                "point-1",
                RobotClaimedEntityState::Printed,
                measurement(0.0, 0.0, 0.5),
            )],
            &[],
            profile(),
            crew(),
        )
        .unwrap();
        assert_eq!(result.held_count, 1);
        assert_eq!(
            result.entity_results[0].reason,
            EntityScoreReason::ReferenceResolutionInsufficient
        );
    }

    #[test]
    fn explicit_human_waiver_is_separate_from_accepted_output() {
        let waiver = HumanWaiver {
            design_entity_id: "point-1".into(),
            authority_id: "authority-17".into(),
            authority_role: "site-quality-manager".into(),
            reason: "approved alternate marking method".into(),
            scope: "point-1 only".into(),
            waiver_receipt_sha256: "waiver-digest".into(),
        };
        let result =
            score_field_layout_run(&manifest(&["point-1"]), &[], &[waiver], profile(), crew())
                .unwrap();
        assert_eq!(result.waived_count, 1);
        assert_eq!(result.accepted_count, 0);
        assert_eq!(
            result.entity_results[0].referee_state,
            RefereeEntityState::Waived
        );
    }

    #[test]
    fn duplicate_or_unknown_design_ids_are_errors_not_silently_deduplicated() {
        let duplicate = manifest(&["line-1", "line-1"]);
        assert_eq!(
            score_field_layout_run(&duplicate, &[], &[], profile(), crew()),
            Err(FieldAcceptanceError::DuplicateManifestEntityId(
                "line-1".into()
            ))
        );

        let result = score_field_layout_run(
            &manifest(&["line-1"]),
            &[observed(
                "unknown",
                RobotClaimedEntityState::Printed,
                measurement(0.0, 0.0, 0.1),
            )],
            &[],
            profile(),
            crew(),
        );
        assert_eq!(
            result,
            Err(FieldAcceptanceError::UnknownObservationEntity(
                "unknown".into()
            ))
        );
    }

    #[test]
    fn zero_crew_hours_does_not_report_infinite_productivity() {
        let result = score_field_layout_run(
            &manifest(&["point-1"]),
            &[observed(
                "point-1",
                RobotClaimedEntityState::Printed,
                measurement(0.0, 0.0, 0.1),
            )],
            &[],
            profile(),
            CrewHours::default(),
        )
        .unwrap();
        assert_eq!(result.accepted_count, 1);
        assert_eq!(result.accepted_entities_per_crew_hour, None);
    }

    #[test]
    fn duplicate_observations_and_invalid_measurement_or_accounting_are_errors() {
        let observation = observed(
            "line-1",
            RobotClaimedEntityState::Printed,
            measurement(0.2, 0.1, 0.1),
        );
        assert_eq!(
            score_field_layout_run(
                &manifest(&["line-1"]),
                &[observation.clone(), observation],
                &[],
                profile(),
                crew(),
            ),
            Err(FieldAcceptanceError::DuplicateObservation("line-1".into()))
        );

        let mut malformed = measurement(0.2, 0.1, 0.1);
        malformed.reference_u95_x_mm = f64::NAN;
        assert!(matches!(
            score_field_layout_run(
                &manifest(&["line-1"]),
                &[observed(
                    "line-1",
                    RobotClaimedEntityState::Printed,
                    malformed
                )],
                &[],
                profile(),
                crew(),
            ),
            Err(FieldAcceptanceError::InvalidMeasurement(id)) if id == "line-1"
        ));

        let invalid_hours = CrewHours {
            inspection: -0.1,
            ..CrewHours::default()
        };
        assert_eq!(
            score_field_layout_run(&manifest(&["line-1"]), &[], &[], profile(), invalid_hours,),
            Err(FieldAcceptanceError::InvalidCrewHours)
        );
    }

    #[test]
    fn a_waiver_does_not_count_as_accepted_or_as_robot_productivity() {
        let waiver = HumanWaiver {
            design_entity_id: "point-1".into(),
            authority_id: "authority-17".into(),
            authority_role: "site-quality-manager".into(),
            reason: "approved alternate marking method".into(),
            scope: "point-1 only".into(),
            waiver_receipt_sha256: "waiver-digest".into(),
        };
        let result = score_field_layout_run(
            &manifest(&["point-1"]),
            &[],
            &[waiver],
            profile(),
            crew(),
        )
        .unwrap();
        assert_eq!(result.waived_count, 1);
        assert_eq!(result.attempted_count, 0);
        assert_eq!(result.first_pass_yield, None);
        assert_eq!(result.accepted_entities_per_crew_hour, Some(0.0));
    }
}
