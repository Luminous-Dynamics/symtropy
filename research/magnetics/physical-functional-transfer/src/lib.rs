// SPDX-License-Identifier: MIT
#![allow(clippy::float_cmp)] // Exact spatial identity checks are intentional.

//! Conservative propagation from a physical-field cell enclosure through a
//! bounded two-channel sensor transfer to independent phase and amplitude
//! requirements. This validates supplied intervals; it does not authenticate
//! calibration or prove that an external transfer envelope is physically valid.

use magnetic_interval_ad::Interval;
use magnetic_physical_discrepancy::{EvidenceRole, PhysicalFieldDiscrepancyCertificate};

#[derive(Clone, Debug, PartialEq)]
pub struct BoundedSensorTransfer {
    pub cell_id: String,
    pub cell_bounds_m: [Interval; 3],
    pub coordinate_frame_id: String,
    pub operating_condition_id: String,
    pub geometry_sha256: String,
    pub material_sha256: String,
    pub model_set_id: String,
    pub solver_run_id: String,
    pub upstream_discrepancy_requirement_id: String,
    pub input_field_unit: String,
    pub output_unit: String,
    pub coefficient_unit: String,
    pub offset_unit: String,
    pub residual_unit: String,
    pub sensor_id: String,
    pub frontend_id: String,
    pub calibration_id: String,
    pub calibration_sha256: String,
    pub transfer_model_id: String,
    pub transfer_source_revision: String,
    pub transfer_artifact_sha256: String,
    pub transfer_environment_sha256: String,
    /// Rows are output channels X/Y; columns are physical Bx/By/Bz.
    pub gain: [[Interval; 3]; 2],
    pub offset: [Interval; 2],
    /// Whole-cell residual envelope for effects not represented by the
    /// bounded affine transfer (e.g. declared nonlinearity/hysteresis).
    pub response_residual: [Interval; 2],
    pub calibration_evidence_role: EvidenceRole,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FunctionalRequirements {
    pub rotor_state_id: String,
    pub reference_record_id: String,
    pub reference_sha256: String,
    pub reference_operating_condition_id: String,
    /// Independently traceable mechanical reference phasor, not the candidate
    /// sensor's own output or a calibration fit from the evaluated trace.
    pub reference_phasor: [Interval; 2],
    pub phase_requirement_id: String,
    pub maximum_error_cosine_record_id: String,
    pub maximum_error_cosine_sha256: String,
    /// Supplied enclosure of cos(maximum allowed shortest-arc phase error).
    pub maximum_error_cosine: Interval,
    pub amplitude_requirement_id: String,
    pub minimum_amplitude_record_id: String,
    pub minimum_amplitude_sha256: String,
    /// Squared magnitude threshold in output_unit squared.
    pub minimum_amplitude_squared: Interval,
    pub reference_evidence_role: EvidenceRole,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequirementStatus {
    Safe,
    Failing,
    Unresolved,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombinedStatus {
    Safe,
    Failing,
    Unresolved,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhysicalFunctionalCertificate {
    pub domain_id: String,
    pub cell_id: String,
    pub cell_bounds_m: [Interval; 3],
    pub physical_field_over_cell: [Interval; 3],
    pub measured_phasor_over_cell: [Interval; 2],
    pub reference_phasor: [Interval; 2],
    /// None means the measured phasor may vanish, so phase is undefined.
    pub phase_margin: Option<Interval>,
    pub amplitude_margin: Interval,
    pub phase_status: RequirementStatus,
    pub amplitude_status: RequirementStatus,
    pub combined_status: CombinedStatus,
    pub upstream_discrepancy_requirement_id: String,
    pub phase_requirement_id: String,
    pub amplitude_requirement_id: String,
    pub sensor_id: String,
    pub frontend_id: String,
    pub calibration_id: String,
    pub calibration_sha256: String,
    pub transfer_model_id: String,
    pub transfer_source_revision: String,
    pub transfer_artifact_sha256: String,
    pub transfer_environment_sha256: String,
    pub reference_record_id: String,
    pub reference_sha256: String,
    pub rotor_state_id: String,
    pub physical_evidence_role: EvidenceRole,
    pub calibration_evidence_role: EvidenceRole,
    pub reference_evidence_role: EvidenceRole,
    pub arithmetic_contract: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FunctionalTransferError {
    InvalidIdentity,
    InvalidSha256 { field: &'static str },
    ScopeMismatch { field: &'static str },
    InvalidUnit { field: &'static str },
    InvalidInterval { field: &'static str },
    InvalidCosineThreshold,
    InvalidAmplitudeThreshold,
    ReferencePhasorMayBeZero,
    RequirementIdentityCollision,
    CalibrationMustUseCalibrationEvidence,
    NonFiniteDerivedInterval,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_digest(value: &str, field: &'static str) -> Result<(), FunctionalTransferError> {
    if valid_sha256(value) {
        Ok(())
    } else {
        Err(FunctionalTransferError::InvalidSha256 { field })
    }
}

fn valid_interval(value: Interval) -> bool {
    value.is_finite() && value.lower() <= value.upper()
}

fn same_float(left: f64, right: f64) -> bool {
    left.to_bits() == right.to_bits()
}

fn same_box(left: [Interval; 3], right: [Interval; 3]) -> bool {
    left.iter().zip(right.iter()).all(|(a, b)| {
        same_float(a.lower(), b.lower()) && same_float(a.upper(), b.upper())
    })
}

fn classify(margin: Interval) -> RequirementStatus {
    if margin.lower() > 0.0 {
        RequirementStatus::Safe
    } else if margin.upper() < 0.0 {
        RequirementStatus::Failing
    } else {
        RequirementStatus::Unresolved
    }
}

fn combine(
    phase: RequirementStatus,
    amplitude: RequirementStatus,
) -> CombinedStatus {
    if phase == RequirementStatus::Failing || amplitude == RequirementStatus::Failing {
        CombinedStatus::Failing
    } else if phase == RequirementStatus::Safe && amplitude == RequirementStatus::Safe {
        CombinedStatus::Safe
    } else {
        CombinedStatus::Unresolved
    }
}

fn validate_inputs(
    physical: &PhysicalFieldDiscrepancyCertificate,
    transfer: &BoundedSensorTransfer,
    requirements: &FunctionalRequirements,
) -> Result<(), FunctionalTransferError> {
    let identities = [
        physical.domain_id.as_str(),
        physical.cell_id.as_str(),
        physical.coordinate_frame_id.as_str(),
        physical.operating_condition_id.as_str(),
        physical.model_set_id.as_str(),
        physical.solver_run_id.as_str(),
        physical.requirement_id.as_str(),
        physical.centre_measurement_id.as_str(),
        physical.measurement_uncertainty_record_id.as_str(),
        physical.probe_calibration_id.as_str(),
        physical.registration_record_id.as_str(),
        physical.background_control_record_id.as_str(),
        physical.temperature_record_id.as_str(),
        physical.residual_jacobian_record_id.as_str(),
        physical.residual_jacobian_source_revision.as_str(),
        physical.residual_jacobian_implementation_id.as_str(),
        physical.residual_jacobian_cell_id.as_str(),
        physical.model_run_id.as_str(),
        physical.model_source_revision.as_str(),
        physical.model_implementation_id.as_str(),
        transfer.cell_id.as_str(),
        transfer.coordinate_frame_id.as_str(),
        transfer.operating_condition_id.as_str(),
        transfer.model_set_id.as_str(),
        transfer.solver_run_id.as_str(),
        transfer.upstream_discrepancy_requirement_id.as_str(),
        transfer.sensor_id.as_str(),
        transfer.frontend_id.as_str(),
        transfer.calibration_id.as_str(),
        transfer.transfer_model_id.as_str(),
        transfer.transfer_source_revision.as_str(),
        requirements.rotor_state_id.as_str(),
        requirements.reference_record_id.as_str(),
        requirements.reference_operating_condition_id.as_str(),
        requirements.phase_requirement_id.as_str(),
        requirements.maximum_error_cosine_record_id.as_str(),
        requirements.amplitude_requirement_id.as_str(),
        requirements.minimum_amplitude_record_id.as_str(),
        physical.field_unit.as_str(),
        physical.arithmetic_contract.as_str(),
        transfer.input_field_unit.as_str(),
        transfer.output_unit.as_str(),
        transfer.coefficient_unit.as_str(),
        transfer.offset_unit.as_str(),
        transfer.residual_unit.as_str(),
    ];
    if identities.iter().any(|value| !nonempty(value)) {
        return Err(FunctionalTransferError::InvalidIdentity);
    }

    for (digest, field) in [
        (&physical.centre_measurement_sha256, "centre_measurement_sha256"),
        (&physical.residual_jacobian_artifact_sha256, "residual_jacobian_artifact_sha256"),
        (
            &physical.residual_jacobian_environment_sha256,
            "residual_jacobian_environment_sha256",
        ),
        (&physical.model_artifact_sha256, "model_artifact_sha256"),
        (&physical.model_environment_sha256, "model_environment_sha256"),
        (&physical.model_input_sha256, "model_input_sha256"),
        (&physical.geometry_sha256, "physical_geometry_sha256"),
        (&physical.material_sha256, "physical_material_sha256"),
        (&transfer.geometry_sha256, "transfer_geometry_sha256"),
        (&transfer.material_sha256, "transfer_material_sha256"),
        (&transfer.calibration_sha256, "calibration_sha256"),
        (&transfer.transfer_artifact_sha256, "transfer_artifact_sha256"),
        (&transfer.transfer_environment_sha256, "transfer_environment_sha256"),
        (&requirements.reference_sha256, "reference_sha256"),
        (
            &requirements.maximum_error_cosine_sha256,
            "maximum_error_cosine_sha256",
        ),
        (
            &requirements.minimum_amplitude_sha256,
            "minimum_amplitude_sha256",
        ),
    ] {
        validate_digest(digest, field)?;
    }

    for (matches, field) in [
        (physical.cell_id == transfer.cell_id, "cell_id"),
        (
            same_box(physical.cell_bounds_m, transfer.cell_bounds_m),
            "cell_bounds_m",
        ),
        (
            physical.coordinate_frame_id == transfer.coordinate_frame_id,
            "coordinate_frame_id",
        ),
        (
            physical.operating_condition_id == transfer.operating_condition_id,
            "operating_condition_id",
        ),
        (
            physical.geometry_sha256 == transfer.geometry_sha256,
            "geometry_sha256",
        ),
        (
            physical.material_sha256 == transfer.material_sha256,
            "material_sha256",
        ),
        (physical.model_set_id == transfer.model_set_id, "model_set_id"),
        (physical.solver_run_id == transfer.solver_run_id, "solver_run_id"),
        (
            physical.requirement_id == transfer.upstream_discrepancy_requirement_id,
            "upstream_discrepancy_requirement_id",
        ),
        (
            physical.field_unit == transfer.input_field_unit,
            "input_field_unit",
        ),
        (
            requirements.reference_operating_condition_id
                == physical.operating_condition_id,
            "reference_operating_condition_id",
        ),
    ] {
        if !matches {
            return Err(FunctionalTransferError::ScopeMismatch { field });
        }
    }

    if physical.residual_jacobian_cell_id != physical.cell_id
        || physical.residual_jacobian_unit != format!("{}/metre", physical.field_unit)
    {
        return Err(FunctionalTransferError::ScopeMismatch {
            field: "physical_residual_jacobian_scope",
        });
    }

    for digest in &physical.residual_jacobian_component_input_sha256 {
        validate_digest(digest, "physical_residual_jacobian_component_input_sha256")?;
    }
    for record_id in &physical.residual_jacobian_component_record_ids {
        if !nonempty(record_id) {
            return Err(FunctionalTransferError::InvalidIdentity);
        }
    }
    for expression_id in &physical.residual_jacobian_component_expression_ids {
        if !nonempty(expression_id) {
            return Err(FunctionalTransferError::InvalidIdentity);
        }
    }
    if physical
        .residual_jacobian_abs_supremum_t_per_m
        .iter()
        .flatten()
        .any(|bound| !bound.is_finite() || *bound < 0.0)
    {
        return Err(FunctionalTransferError::InvalidInterval {
            field: "physical_residual_jacobian",
        });
    }
    for intervals in [
        physical.measured_field_at_centre_t,
        physical.model_field_at_centre_t,
        physical.centre_residual_t,
        physical.residual_field_over_cell_t,
        physical.model_field_over_cell_t,
    ] {
        if intervals.iter().any(|interval| !valid_interval(*interval)) {
            return Err(FunctionalTransferError::InvalidInterval {
                field: "physical_field_certificate",
            });
        }
    }

    if transfer.calibration_evidence_role != EvidenceRole::Calibration {
        return Err(FunctionalTransferError::CalibrationMustUseCalibrationEvidence);
    }

    for (axis, bounds) in physical.cell_bounds_m.iter().enumerate() {
        if !valid_interval(*bounds) || bounds.lower() >= bounds.upper() {
            return Err(FunctionalTransferError::InvalidInterval {
                field: "cell_bounds_m",
            });
        }
        if !physical.centre_position_m[axis].is_finite()
            || physical.centre_position_m[axis] < bounds.lower()
            || physical.centre_position_m[axis] > bounds.upper()
        {
            return Err(FunctionalTransferError::InvalidInterval {
                field: "physical_centre_position",
            });
        }
    }
    if transfer.output_unit != "dimensionless"
        || transfer.coefficient_unit != format!("dimensionless/{}", physical.field_unit)
        || transfer.offset_unit != transfer.output_unit
        || transfer.residual_unit != transfer.output_unit
    {
        return Err(FunctionalTransferError::InvalidUnit {
            field: "sensor_transfer_units",
        });
    }

    if requirements.phase_requirement_id == requirements.amplitude_requirement_id
        || requirements.phase_requirement_id == physical.requirement_id
        || requirements.amplitude_requirement_id == physical.requirement_id
    {
        return Err(FunctionalTransferError::RequirementIdentityCollision);
    }

    for interval in physical.physical_field_over_cell_t {
        if !valid_interval(interval) {
            return Err(FunctionalTransferError::InvalidInterval {
                field: "physical_field_over_cell",
            });
        }
    }
    for row in transfer.gain {
        for interval in row {
            if !valid_interval(interval) {
                return Err(FunctionalTransferError::InvalidInterval {
                    field: "gain",
                });
            }
        }
    }
    for interval in transfer.offset.into_iter().chain(transfer.response_residual) {
        if !valid_interval(interval) {
            return Err(FunctionalTransferError::InvalidInterval {
                field: "offset_or_response_residual",
            });
        }
    }
    for interval in requirements.reference_phasor {
        if !valid_interval(interval) {
            return Err(FunctionalTransferError::InvalidInterval {
                field: "reference_phasor",
            });
        }
    }

    if !valid_interval(requirements.maximum_error_cosine)
        || requirements.maximum_error_cosine.lower() <= -1.0
        || requirements.maximum_error_cosine.upper() > 1.0
    {
        return Err(FunctionalTransferError::InvalidCosineThreshold);
    }
    if !valid_interval(requirements.minimum_amplitude_squared)
        || requirements.minimum_amplitude_squared.lower() < 0.0
    {
        return Err(FunctionalTransferError::InvalidAmplitudeThreshold);
    }

    let reference_norm_squared = requirements.reference_phasor[0].square()
        + requirements.reference_phasor[1].square();
    if !reference_norm_squared.is_finite() || reference_norm_squared.lower() <= 0.0 {
        return Err(FunctionalTransferError::ReferencePhasorMayBeZero);
    }

    Ok(())
}

/// Propagate a whole-cell physical magnetic-field enclosure through a bounded
/// two-channel sensor model. Phase and minimum amplitude remain independent
/// non-compensatory requirements. A certificate is conditional on the
/// upstream field, transfer, calibration and reference intervals being valid.
pub fn certify_physical_sensor_function(
    physical: &PhysicalFieldDiscrepancyCertificate,
    transfer: &BoundedSensorTransfer,
    requirements: &FunctionalRequirements,
) -> Result<PhysicalFunctionalCertificate, FunctionalTransferError> {
    validate_inputs(physical, transfer, requirements)?;

    let field = physical.physical_field_over_cell_t;
    let mut measured_phasor = [Interval::ZERO; 2];
    for (channel, measured_component) in measured_phasor.iter_mut().enumerate() {
        let mut value = transfer.offset[channel] + transfer.response_residual[channel];
        for (axis, component) in field.iter().enumerate() {
            value = value + transfer.gain[channel][axis] * *component;
        }
        if !value.is_finite() {
            return Err(FunctionalTransferError::NonFiniteDerivedInterval);
        }
        *measured_component = value;
    }

    let measured_norm_squared =
        measured_phasor[0].square() + measured_phasor[1].square();
    let amplitude_margin =
        measured_norm_squared - requirements.minimum_amplitude_squared;
    if !measured_norm_squared.is_finite() || !amplitude_margin.is_finite() {
        return Err(FunctionalTransferError::NonFiniteDerivedInterval);
    }
    let amplitude_status = classify(amplitude_margin);

    let reference_norm_squared = requirements.reference_phasor[0].square()
        + requirements.reference_phasor[1].square();
    let phase_margin = if measured_norm_squared.lower() <= 0.0 {
        None
    } else {
        let denominator = (measured_norm_squared * reference_norm_squared)
            .sqrt()
            .map_err(|_| FunctionalTransferError::NonFiniteDerivedInterval)?;
        if !denominator.is_finite() || denominator.lower() <= 0.0 {
            return Err(FunctionalTransferError::NonFiniteDerivedInterval);
        }
        let dot = measured_phasor[0] * requirements.reference_phasor[0]
            + measured_phasor[1] * requirements.reference_phasor[1];
        let similarity = dot / denominator;
        let margin = similarity - requirements.maximum_error_cosine;
        if !margin.is_finite() {
            return Err(FunctionalTransferError::NonFiniteDerivedInterval);
        }
        Some(margin)
    };

    let phase_status = phase_margin.map(classify).unwrap_or(RequirementStatus::Unresolved);
    let combined_status = combine(phase_status, amplitude_status);

    Ok(PhysicalFunctionalCertificate {
        domain_id: physical.domain_id.clone(),
        cell_id: physical.cell_id.clone(),
        cell_bounds_m: physical.cell_bounds_m,
        physical_field_over_cell: field,
        measured_phasor_over_cell: measured_phasor,
        reference_phasor: requirements.reference_phasor,
        phase_margin,
        amplitude_margin,
        phase_status,
        amplitude_status,
        combined_status,
        upstream_discrepancy_requirement_id: physical.requirement_id.clone(),
        phase_requirement_id: requirements.phase_requirement_id.clone(),
        amplitude_requirement_id: requirements.amplitude_requirement_id.clone(),
        sensor_id: transfer.sensor_id.clone(),
        frontend_id: transfer.frontend_id.clone(),
        calibration_id: transfer.calibration_id.clone(),
        calibration_sha256: transfer.calibration_sha256.clone(),
        transfer_model_id: transfer.transfer_model_id.clone(),
        transfer_source_revision: transfer.transfer_source_revision.clone(),
        transfer_artifact_sha256: transfer.transfer_artifact_sha256.clone(),
        transfer_environment_sha256: transfer.transfer_environment_sha256.clone(),
        reference_record_id: requirements.reference_record_id.clone(),
        reference_sha256: requirements.reference_sha256.clone(),
        rotor_state_id: requirements.rotor_state_id.clone(),
        physical_evidence_role: physical.evidence_role,
        calibration_evidence_role: transfer.calibration_evidence_role,
        reference_evidence_role: requirements.reference_evidence_role,
        arithmetic_contract: "magnetic-interval-ad outward-expanded binary64 interval operations".to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use magnetic_interval_ad::Interval;

    const DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const DIGEST_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn iv(lower: f64, upper: f64) -> Interval {
        Interval::new(lower, upper).expect("valid interval")
    }

    fn physical(field: [Interval; 3]) -> PhysicalFieldDiscrepancyCertificate {
        PhysicalFieldDiscrepancyCertificate {
            domain_id: "domain-1".to_owned(),
            cell_id: "cell-1".to_owned(),
            cell_bounds_m: [iv(-0.01, 0.01), iv(-0.01, 0.01), iv(-0.01, 0.01)],
            centre_position_m: [0.0; 3],
            measured_field_at_centre_t: field,
            model_field_at_centre_t: field,
            centre_residual_t: [iv(0.0, 0.0); 3],
            residual_variation_radius_t: [0.0; 3],
            residual_field_over_cell_t: [iv(0.0, 0.0); 3],
            model_field_over_cell_t: field,
            physical_field_over_cell_t: field,
            centre_measurement_id: "probe-map-1".to_owned(),
            centre_measurement_sha256: DIGEST.to_owned(),
            measurement_uncertainty_record_id: "measurement-uncertainty-1".to_owned(),
            probe_calibration_id: "probe-cal-1".to_owned(),
            registration_record_id: "registration-1".to_owned(),
            background_control_record_id: "background-1".to_owned(),
            temperature_record_id: "temperature-23c".to_owned(),
            residual_jacobian_record_id: "residual-jacobian-1".to_owned(),
            residual_jacobian_component_record_ids: [
                "rx-1".to_owned(),
                "ry-1".to_owned(),
                "rz-1".to_owned(),
            ],
            residual_jacobian_component_expression_ids: [
                "residual-bx".to_owned(),
                "residual-by".to_owned(),
                "residual-bz".to_owned(),
            ],
            residual_jacobian_component_input_sha256: [
                DIGEST.to_owned(),
                DIGEST_B.to_owned(),
                DIGEST.to_owned(),
            ],
            residual_jacobian_source_revision: "residual-source-1".to_owned(),
            residual_jacobian_implementation_id: "interval-ad-residual".to_owned(),
            residual_jacobian_artifact_sha256: DIGEST.to_owned(),
            residual_jacobian_environment_sha256: DIGEST_B.to_owned(),
            residual_jacobian_cell_id: "cell-1".to_owned(),
            residual_jacobian_abs_supremum_t_per_m: [[0.0; 3]; 3],
            model_run_id: "model-run-1".to_owned(),
            model_source_revision: "model-source-1".to_owned(),
            model_implementation_id: "model-impl-1".to_owned(),
            model_artifact_sha256: DIGEST.to_owned(),
            model_environment_sha256: DIGEST_B.to_owned(),
            model_set_id: "model-set-1".to_owned(),
            model_input_sha256: DIGEST.to_owned(),
            coordinate_frame_id: "sensor-frame-1".to_owned(),
            operating_condition_id: "angle-0-temp-23c".to_owned(),
            geometry_sha256: DIGEST_B.to_owned(),
            material_sha256: DIGEST.to_owned(),
            requirement_id: "physical-field-envelope-1".to_owned(),
            solver_run_id: "solver-run-1".to_owned(),
            field_unit: "tesla".to_owned(),
            residual_jacobian_unit: "tesla/metre".to_owned(),
            arithmetic_contract: "outward-rounded-binary64".to_owned(),
            evidence_role: EvidenceRole::PostSelectionQualification,
        }
    }

    fn transfer() -> BoundedSensorTransfer {
        BoundedSensorTransfer {
            cell_id: "cell-1".to_owned(),
            cell_bounds_m: [iv(-0.01, 0.01), iv(-0.01, 0.01), iv(-0.01, 0.01)],
            coordinate_frame_id: "sensor-frame-1".to_owned(),
            operating_condition_id: "angle-0-temp-23c".to_owned(),
            geometry_sha256: DIGEST_B.to_owned(),
            material_sha256: DIGEST.to_owned(),
            model_set_id: "model-set-1".to_owned(),
            solver_run_id: "solver-run-1".to_owned(),
            upstream_discrepancy_requirement_id: "physical-field-envelope-1".to_owned(),
            input_field_unit: "tesla".to_owned(),
            output_unit: "dimensionless".to_owned(),
            coefficient_unit: "dimensionless/tesla".to_owned(),
            offset_unit: "dimensionless".to_owned(),
            residual_unit: "dimensionless".to_owned(),
            sensor_id: "sensor-2d-001".to_owned(),
            frontend_id: "frontend-001".to_owned(),
            calibration_id: "sensor-cal-001".to_owned(),
            calibration_sha256: DIGEST.to_owned(),
            transfer_model_id: "bounded-linear-transfer-v1".to_owned(),
            transfer_source_revision: "sensor-transfer-source-1".to_owned(),
            transfer_artifact_sha256: DIGEST_B.to_owned(),
            transfer_environment_sha256: DIGEST.to_owned(),
            gain: [
                [iv(1.0, 1.0), iv(0.0, 0.0), iv(0.0, 0.0)],
                [iv(0.0, 0.0), iv(1.0, 1.0), iv(0.0, 0.0)],
            ],
            offset: [iv(0.0, 0.0); 2],
            response_residual: [iv(0.0, 0.0); 2],
            calibration_evidence_role: EvidenceRole::Calibration,
        }
    }

    fn requirements() -> FunctionalRequirements {
        FunctionalRequirements {
            rotor_state_id: "rotor-angle-0".to_owned(),
            reference_record_id: "reference-encoder-sample-001".to_owned(),
            reference_sha256: DIGEST.to_owned(),
            reference_operating_condition_id: "angle-0-temp-23c".to_owned(),
            reference_phasor: [iv(1.0, 1.0), iv(0.0, 0.0)],
            phase_requirement_id: "phase-tolerance-v1".to_owned(),
            maximum_error_cosine_record_id: "phase-limit-record-1".to_owned(),
            maximum_error_cosine_sha256: DIGEST_B.to_owned(),
            maximum_error_cosine: iv(0.99, 0.99),
            amplitude_requirement_id: "minimum-amplitude-v1".to_owned(),
            minimum_amplitude_record_id: "amplitude-limit-record-1".to_owned(),
            minimum_amplitude_sha256: DIGEST.to_owned(),
            minimum_amplitude_squared: iv(0.25, 0.25),
            reference_evidence_role: EvidenceRole::PostSelectionQualification,
        }
    }

    #[test]
    fn identity_transfer_certifies_both_requirements() {
        let result = certify_physical_sensor_function(
            &physical([iv(1.0, 1.0), iv(0.0, 0.0), iv(0.0, 0.0)]),
            &transfer(),
            &requirements(),
        )
        .expect("valid scope");
        assert_eq!(result.phase_status, RequirementStatus::Safe);
        assert_eq!(result.amplitude_status, RequirementStatus::Safe);
        assert_eq!(result.combined_status, CombinedStatus::Safe);
        assert!(result.phase_margin.expect("nonzero phasor").lower() > 0.0);
        assert!(result.amplitude_margin.lower() > 0.0);
    }

    #[test]
    fn wrong_phase_with_healthy_amplitude_is_failing() {
        let result = certify_physical_sensor_function(
            &physical([iv(-1.0, -1.0), iv(0.0, 0.0), iv(0.0, 0.0)]),
            &transfer(),
            &requirements(),
        )
        .expect("valid scope");
        assert_eq!(result.phase_status, RequirementStatus::Failing);
        assert_eq!(result.amplitude_status, RequirementStatus::Safe);
        assert_eq!(result.combined_status, CombinedStatus::Failing);
    }

    #[test]
    fn amplitude_failure_cannot_be_hidden_by_good_phase() {
        let result = certify_physical_sensor_function(
            &physical([iv(0.1, 0.1), iv(0.0, 0.0), iv(0.0, 0.0)]),
            &transfer(),
            &requirements(),
        )
        .expect("valid scope");
        assert_eq!(result.phase_status, RequirementStatus::Safe);
        assert_eq!(result.amplitude_status, RequirementStatus::Failing);
        assert_eq!(result.combined_status, CombinedStatus::Failing);
    }

    #[test]
    fn phasors_across_atan2_cut_remain_close() {
        let mut req = requirements();
        req.reference_phasor = [iv(-1.0, -1.0), iv(0.001, 0.001)];
        let result = certify_physical_sensor_function(
            &physical([iv(-1.0, -1.0), iv(-0.001, -0.001), iv(0.0, 0.0)]),
            &transfer(),
            &req,
        )
        .expect("valid branch-cut challenge");
        assert_eq!(result.phase_status, RequirementStatus::Safe);
    }

    #[test]
    fn zero_possible_measured_phasor_leaves_phase_unresolved() {
        let result = certify_physical_sensor_function(
            &physical([iv(-0.1, 0.1), iv(0.0, 0.0), iv(0.0, 0.0)]),
            &transfer(),
            &requirements(),
        )
        .expect("amplitude can still be classified");
        assert_eq!(result.phase_margin, None);
        assert_eq!(result.phase_status, RequirementStatus::Unresolved);
        assert_eq!(result.amplitude_status, RequirementStatus::Failing);
        assert_eq!(result.combined_status, CombinedStatus::Failing);
    }

    #[test]
    fn reference_phasor_may_not_contain_zero() {
        let mut req = requirements();
        req.reference_phasor = [iv(-0.1, 0.1), iv(0.0, 0.0)];
        assert_eq!(
            certify_physical_sensor_function(
                &physical([iv(1.0, 1.0), iv(0.0, 0.0), iv(0.0, 0.0)]),
                &transfer(),
                &req,
            ),
            Err(FunctionalTransferError::ReferencePhasorMayBeZero)
        );
    }

    #[test]
    fn widening_sensor_residual_cannot_turn_uncertainty_into_safe() {
        let physical = physical([iv(1.0, 1.0), iv(0.0, 0.0), iv(0.0, 0.0)]);
        let mut bounded = transfer();
        bounded.response_residual[1] = iv(-0.01, 0.01);
        let baseline = certify_physical_sensor_function(&physical, &bounded, &requirements())
            .expect("bounded residual");
        bounded.response_residual[1] = iv(-0.5, 0.5);
        let widened = certify_physical_sensor_function(&physical, &bounded, &requirements())
            .expect("wider residual");
        assert_ne!(widened.phase_status, RequirementStatus::Safe);
        let baseline_margin = baseline.phase_margin.expect("baseline nonzero");
        let widened_margin = widened.phase_margin.expect("nonzero lower norm");
        let baseline_width = baseline_margin.upper() - baseline_margin.lower();
        let widened_width = widened_margin.upper() - widened_margin.lower();
        assert!(widened_width >= baseline_width);
    }

    #[test]
    fn rejects_cell_and_calibration_scope_mismatch() {
        let physical = physical([iv(1.0, 1.0), iv(0.0, 0.0), iv(0.0, 0.0)]);
        let mut wrong_cell = transfer();
        wrong_cell.cell_bounds_m[0] = iv(-0.02, 0.02);
        assert_eq!(
            certify_physical_sensor_function(&physical, &wrong_cell, &requirements()),
            Err(FunctionalTransferError::ScopeMismatch { field: "cell_bounds_m" })
        );
        let mut wrong_calibration = transfer();
        wrong_calibration.calibration_sha256 = "not-a-digest".to_owned();
        assert_eq!(
            certify_physical_sensor_function(&physical, &wrong_calibration, &requirements()),
            Err(FunctionalTransferError::InvalidSha256 { field: "calibration_sha256" })
        );
    }

    #[test]
    fn frozen_calibration_can_score_blind_holdout_without_role_collapse() {
        let mut physical = physical([iv(1.0, 1.0), iv(0.0, 0.0), iv(0.0, 0.0)]);
        physical.evidence_role = EvidenceRole::BlindHoldout;
        let mut req = requirements();
        req.reference_evidence_role = EvidenceRole::BlindHoldout;
        let result = certify_physical_sensor_function(&physical, &transfer(), &req)
            .expect("frozen calibration may score a sealed holdout");
        assert_eq!(result.physical_evidence_role, EvidenceRole::BlindHoldout);
        assert_eq!(result.calibration_evidence_role, EvidenceRole::Calibration);
        assert_eq!(result.reference_evidence_role, EvidenceRole::BlindHoldout);
        assert_eq!(result.combined_status, CombinedStatus::Safe);
    }

    #[test]
    fn adaptive_exploration_cannot_be_used_as_sensor_calibration() {
        let physical = physical([iv(1.0, 1.0), iv(0.0, 0.0), iv(0.0, 0.0)]);
        let mut sensor = transfer();
        sensor.calibration_evidence_role = EvidenceRole::AdaptiveExploration;
        assert_eq!(
            certify_physical_sensor_function(&physical, &sensor, &requirements()),
            Err(FunctionalTransferError::CalibrationMustUseCalibrationEvidence)
        );
    }

    #[test]
    fn strict_zero_boundary_is_unresolved() {
        let mut req = requirements();
        req.maximum_error_cosine = iv(1.0, 1.0);
        let result = certify_physical_sensor_function(
            &physical([iv(1.0, 1.0), iv(0.0, 0.0), iv(0.0, 0.0)]),
            &transfer(),
            &req,
        )
        .expect("valid threshold");
        assert_eq!(result.phase_status, RequirementStatus::Unresolved);
    }

    #[test]
    fn wrong_units_fail_closed() {
        let physical = physical([iv(1.0, 1.0), iv(0.0, 0.0), iv(0.0, 0.0)]);
        let mut sensor = transfer();
        sensor.coefficient_unit = "dimensionless".to_owned();
        assert_eq!(
            certify_physical_sensor_function(&physical, &sensor, &requirements()),
            Err(FunctionalTransferError::InvalidUnit {
                field: "sensor_transfer_units"
            })
        );
    }
}
