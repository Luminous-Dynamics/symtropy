// SPDX-License-Identifier: MIT
#![allow(clippy::float_cmp)] // Exact cell-boundary identity checks are intentional.

//! Measurement-informed cell-wide magnetic model-discrepancy enclosures.
//!
//! This crate validates supplied measurement/model intervals and residual
//! Jacobian supremum bounds. It does not derive or authenticate those bounds.

use magnetic_interval_ad::{
    certify_expression_centered, map_spatial_derivative_suprema, Interval, IntervalAdError,
    IntervalDual, ParameterDomain, SpatialMappingError, ValidatedDerivativeEnclosure,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EvidenceRole {
    Calibration,
    AdaptiveExploration,
    BlindHoldout,
    PostSelectionQualification,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CellDiscrepancyInput {
    pub domain_id: String,
    pub cell_id: String,
    pub cell_bounds_m: [Interval; 3],
    pub centre_position_m: [f64; 3],
    pub centre_measurement_id: String,
    pub centre_measurement_sha256: String,
    /// Probe/calibration/registration/background/temperature uncertainty
    /// must already be enclosed in these component intervals.
    pub measured_field_at_centre_t: [Interval; 3],
    pub measurement_uncertainty_record_id: String,
    pub probe_calibration_id: String,
    pub registration_record_id: String,
    pub background_control_record_id: String,
    pub temperature_record_id: String,
    pub model_run_id: String,
    pub model_source_revision: String,
    pub model_implementation_id: String,
    pub model_artifact_sha256: String,
    pub model_environment_sha256: String,
    pub model_set_id: String,
    pub model_input_sha256: String,
    pub model_field_at_centre_t: [Interval; 3],
    /// Solver prediction enclosure across the complete spatial cell.
    pub model_field_over_cell_t: [Interval; 3],
    pub residual_jacobian_record_id: String,
    pub residual_jacobian_component_record_ids: [String; 3],
    pub residual_jacobian_component_expression_ids: [String; 3],
    pub residual_jacobian_source_revision: String,
    pub residual_jacobian_implementation_id: String,
    pub residual_jacobian_artifact_sha256: String,
    pub residual_jacobian_environment_sha256: String,
    pub residual_jacobian_cell_id: String,
    /// Rows are Bx/By/Bz; columns are x/y/z. Values are absolute
    /// derivative suprema valid over the entire exact cell.
    pub residual_jacobian_abs_supremum_t_per_m: [[f64; 3]; 3],
    pub coordinate_frame_id: String,
    pub operating_condition_id: String,
    pub geometry_sha256: String,
    pub material_sha256: String,
    pub requirement_id: String,
    pub solver_run_id: String,
    pub field_unit: String,
    pub evidence_role: EvidenceRole,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhysicalFieldDiscrepancyCertificate {
    pub domain_id: String,
    pub cell_id: String,
    pub cell_bounds_m: [Interval; 3],
    pub centre_position_m: [f64; 3],
    pub measured_field_at_centre_t: [Interval; 3],
    pub model_field_at_centre_t: [Interval; 3],
    pub centre_residual_t: [Interval; 3],
    pub residual_variation_radius_t: [f64; 3],
    pub residual_field_over_cell_t: [Interval; 3],
    pub model_field_over_cell_t: [Interval; 3],
    pub physical_field_over_cell_t: [Interval; 3],
    pub centre_measurement_id: String,
    pub centre_measurement_sha256: String,
    pub measurement_uncertainty_record_id: String,
    pub probe_calibration_id: String,
    pub registration_record_id: String,
    pub background_control_record_id: String,
    pub temperature_record_id: String,
    pub residual_jacobian_record_id: String,
    pub residual_jacobian_component_record_ids: [String; 3],
    pub residual_jacobian_component_expression_ids: [String; 3],
    pub residual_jacobian_source_revision: String,
    pub residual_jacobian_implementation_id: String,
    pub residual_jacobian_artifact_sha256: String,
    pub residual_jacobian_environment_sha256: String,
    pub residual_jacobian_cell_id: String,
    pub residual_jacobian_abs_supremum_t_per_m: [[f64; 3]; 3],
    pub model_run_id: String,
    pub model_source_revision: String,
    pub model_implementation_id: String,
    pub model_artifact_sha256: String,
    pub model_environment_sha256: String,
    pub model_set_id: String,
    pub model_input_sha256: String,
    pub coordinate_frame_id: String,
    pub operating_condition_id: String,
    pub geometry_sha256: String,
    pub material_sha256: String,
    pub requirement_id: String,
    pub solver_run_id: String,
    pub field_unit: String,
    pub residual_jacobian_unit: String,
    pub arithmetic_contract: String,
    pub evidence_role: EvidenceRole,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiscrepancyError {
    InvalidIdentity,
    InvalidSha256 {
        field: &'static str,
    },
    InvalidCellBounds {
        axis: usize,
    },
    CentreNotAtCellMidpoint {
        axis: usize,
    },
    UnboundedFieldInterval {
        field: &'static str,
        component: usize,
    },
    JacobianCellIdentityMismatch,
    IntervalAdMapping {
        component: usize,
        error: SpatialMappingError,
    },
    IntervalAdScopeMismatch {
        component: usize,
        field: &'static str,
    },
    DuplicateIntervalAdComponentIdentity {
        component: usize,
    },
    InvalidJacobianBound {
        field_component: usize,
        spatial_axis: usize,
    },
    ModelCentreOutsideCellEnclosure {
        component: usize,
    },
    NonFiniteResidualRadius {
        component: usize,
    },
    NonFinitePhysicalFieldEnclosure {
        component: usize,
    },
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_digest(value: &str, field: &'static str) -> Result<(), DiscrepancyError> {
    if valid_sha256(value) {
        Ok(())
    } else {
        Err(DiscrepancyError::InvalidSha256 { field })
    }
}

fn midpoint(lower: f64, upper: f64) -> Option<f64> {
    let centre = if lower.is_sign_negative() == upper.is_sign_negative() {
        lower + (upper - lower) * 0.5
    } else {
        lower * 0.5 + upper * 0.5
    };
    (centre.is_finite() && centre >= lower && centre <= upper).then_some(centre)
}

fn validate_finite_intervals(
    intervals: &[Interval; 3],
    field: &'static str,
) -> Result<(), DiscrepancyError> {
    for (component, interval) in intervals.iter().enumerate() {
        if !interval.is_finite() {
            return Err(DiscrepancyError::UnboundedFieldInterval { field, component });
        }
    }
    Ok(())
}

/// Bind the three residual-field component Jacobians directly from
/// validated interval-AD whole-cell derivative enclosures.
///
/// The records must describe the exact residual components Bx, By and Bz,
/// in that order, over the same physical cell. The adapter verifies parameter
/// identity/order, metre units, field units, cell bounds and model/solver/
—scope metadata, then copies their derivative absolute suprema into the 3x3
/// residual-Jacobian matrix. It does not infer or authenticate the semantics
/// of the supplied expressions; the expected expression IDs are explicit
/// caller inputs and become part of the resulting certificate.
pub fn bind_interval_ad_residual_jacobian(
    input: &mut CellDiscrepancyInput,
    records: [&ValidatedDerivativeEnclosure<3>; 3],
    expected_expression_ids: [&str; 3],
    spatial_axis_parameter_ids: [&str; 3],
) -> Result<(), DiscrepancyError> {
    validate_input(input)?;

    if expected_expression_ids
        .iter()
        .any(|expression_id| !nonempty(expression_id))
    {
        return Err(DiscrepancyError::InvalidIdentity);
    }

    let mut mapped_records: [Option<magnetic_interval_ad::SpatialDerivativeSupremumEvidence>; 3] =
        std::array::from_fn(|_| None);

    for component in 0..3 {
        let record = records[component];
        if record.expression_id != expected_expression_ids[component] {
            return Err(DiscrepancyError::IntervalAdScopeMismatch {
                component,
                field: "expression_id",
            });
        }
        if record.output_unit != input.field_unit {
            return Err(DiscrepancyError::IntervalAdScopeMismatch {
                component,
                field: "field_unit",
            });
        }
        let mapped = map_spatial_derivative_suprema(
            record,
            &input.cell_id,
            &input.requirement_id,
            spatial_axis_parameter_ids,
            "metre",
        )
        .map_err(|error| DiscrepancyError::IntervalAdMapping { component, error })?;

        if mapped.axis_bounds_m != input.cell_bounds_m {
            return Err(DiscrepancyError::IntervalAdScopeMismatch {
                component,
                field: "cell_bounds_m",
            });
        }
        if mapped.coordinate_frame_id != input.coordinate_frame_id {
            return Err(DiscrepancyError::IntervalAdScopeMismatch {
                component,
                field: "coordinate_frame_id",
            });
        }
        if mapped.operating_condition_id != input.operating_condition_id {
            return Err(DiscrepancyError::IntervalAdScopeMismatch {
                component,
                field: "operating_condition_id",
            });
        }
        if mapped.model_set_id != input.model_set_id {
            return Err(DiscrepancyError::IntervalAdScopeMismatch {
                component,
                field: "model_set_id",
            });
        }
        if mapped.solver_run_id != input.solver_run_id {
            return Err(DiscrepancyError::IntervalAdScopeMismatch {
                component,
                field: "solver_run_id",
            });
        }
        if mapped.geometry_sha256 != input.geometry_sha256 {
            return Err(DiscrepancyError::IntervalAdScopeMismatch {
                component,
                field: "geometry_sha256",
            });
        }
        if mapped.material_sha256 != input.material_sha256 {
            return Err(DiscrepancyError::IntervalAdScopeMismatch {
                component,
                field: "material_sha256",
            });
        }
        if mapped.source_revision != input.residual_jacobian_source_revision {
            return Err(DiscrepancyError::IntervalAdScopeMismatch {
                component,
                field: "source_revision",
            });
        }
        if mapped.implementation_id != input.residual_jacobian_implementation_id {
            return Err(DiscrepancyError::IntervalAdScopeMismatch {
                component,
                field: "implementation_id",
            });
        }
        if mapped.artifact_sha256 != input.residual_jacobian_artifact_sha256 {
            return Err(DiscrepancyError::IntervalAdScopeMismatch {
                component,
                field: "artifact_sha256",
            });
        }
        if mapped.environment_sha256 != input.residual_jacobian_environment_sha256 {
            return Err(DiscrepancyError::IntervalAdScopeMismatch {
                component,
                field: "environment_sha256",
            });
        }

        for previous in 0..component {
            if records[previous].record_id == record.record_id
                || records[previous].expression_id == record.expression_id
            {
                return Err(DiscrepancyError::DuplicateIntervalAdComponentIdentity {
                    component,
                });
            }
        }
        mapped_records[component] = Some(mapped);
    }

    let mapped_records = mapped_records.map(|record| record.expect("all components were mapped"));
    input.residual_jacobian_component_record_ids =
        std::array::from_fn(|index| records[index].record_id.clone());
    input.residual_jacobian_component_expression_ids =
        std::array::from_fn(|index| records[index].expression_id.clone());
    input.residual_jacobian_cell_id = input.cell_id.clone();
    input.residual_jacobian_abs_supremum_t_per_m = std::array::from_fn(|component| {
        mapped_records[component].derivative_abs_supremum_by_axis
    });
    input.residual_jacobian_record_id = format!(
        "interval-ad-jacobian-set:{}|{}|{}",
        records[0].record_id, records[1].record_id, records[2].record_id
    );

    Ok(())
}

fn validate_input(input: &CellDiscrepancyInput) -> Result<(), DiscrepancyError> {
    let identities = [
        input.domain_id.as_str(),
        input.cell_id.as_str(),
        input.centre_measurement_id.as_str(),
        input.measurement_uncertainty_record_id.as_str(),
        input.probe_calibration_id.as_str(),
        input.registration_record_id.as_str(),
        input.background_control_record_id.as_str(),
        input.temperature_record_id.as_str(),
        input.model_run_id.as_str(),
        input.model_source_revision.as_str(),
        input.model_implementation_id.as_str(),
        input.model_set_id.as_str(),
        input.residual_jacobian_record_id.as_str(),
        input.residual_jacobian_source_revision.as_str(),
        input.residual_jacobian_implementation_id.as_str(),
        input.residual_jacobian_cell_id.as_str(),
        input.coordinate_frame_id.as_str(),
        input.operating_condition_id.as_str(),
        input.requirement_id.as_str(),
        input.solver_run_id.as_str(),
        input.field_unit.as_str(),
    ];
    if identities.into_iter().any(|value| !nonempty(value)) {
        return Err(DiscrepancyError::InvalidIdentity);
    }
    validate_digest(
        &input.centre_measurement_sha256,
        "centre_measurement_sha256",
    )?;
    validate_digest(&input.model_input_sha256, "model_input_sha256")?;
    validate_digest(&input.model_artifact_sha256, "model_artifact_sha256")?;
    validate_digest(&input.model_environment_sha256, "model_environment_sha256")?;
    validate_digest(
        &input.residual_jacobian_artifact_sha256,
        "residual_jacobian_artifact_sha256",
    )?;
    validate_digest(
        &input.residual_jacobian_environment_sha256,
        "residual_jacobian_environment_sha256",
    )?;
    validate_digest(&input.geometry_sha256, "geometry_sha256")?;
    validate_digest(&input.material_sha256, "material_sha256")?;

    if input.residual_jacobian_cell_id != input.cell_id {
        return Err(DiscrepancyError::JacobianCellIdentityMismatch);
    }

    for axis in 0..3 {
        let bounds = input.cell_bounds_m[axis];
        if !bounds.is_finite() || bounds.lower() > bounds.upper() {
            return Err(DiscrepancyError::InvalidCellBounds { axis });
        }
        let expected_centre = midpoint(bounds.lower(), bounds.upper())
            .ok_or(DiscrepancyError::InvalidCellBounds { axis })?;
        if input.centre_position_m[axis].to_bits() != expected_centre.to_bits() {
            return Err(DiscrepancyError::CentreNotAtCellMidpoint { axis });
        }
    }

    validate_finite_intervals(
        &input.measured_field_at_centre_t,
        "measured_field_at_centre_t",
    )?;
    validate_finite_intervals(&input.model_field_at_centre_t, "model_field_at_centre_t")?;
    validate_finite_intervals(&input.model_field_over_cell_t, "model_field_over_cell_t")?;

    for component in 0..3 {
        let centre_model = input.model_field_at_centre_t[component];
        let cell_model = input.model_field_over_cell_t[component];
        if cell_model.lower() > centre_model.lower() || cell_model.upper() < centre_model.upper() {
            return Err(DiscrepancyError::ModelCentreOutsideCellEnclosure { component });
        }

        for axis in 0..3 {
            let bound = input.residual_jacobian_abs_supremum_t_per_m[component][axis];
            if !bound.is_finite() || bound < 0.0 {
                return Err(DiscrepancyError::InvalidJacobianBound {
                    field_component: component,
                    spatial_axis: axis,
                });
            }
        }
    }
    Ok(())
}

/// Enclose the measured-minus-model residual and physical field over the
/// complete cell. The centre observation and Jacobian supremum are independent
/// inputs with separate record identities.
pub fn certify_physical_field_discrepancy(
    input: &CellDiscrepancyInput,
) -> Result<PhysicalFieldDiscrepancyCertificate, DiscrepancyError> {
    validate_input(input)?;

    let mut centre_residual_t = [Interval::ZERO; 3];
    let mut residual_variation_radius_t = [0.0; 3];
    let mut residual_field_over_cell_t = [Interval::ZERO; 3];
    let mut physical_field_over_cell_t = [Interval::ZERO; 3];

    let half_widths: [f64; 3] = std::array::from_fn(|axis| {
        let centre =
            Interval::point(input.centre_position_m[axis]).expect("validated finite centre");
        (input.cell_bounds_m[axis] - centre)
            .abs_upper_bound()
            .expect("validated finite cell bounds")
    });

    for component in 0..3 {
        centre_residual_t[component] =
            input.measured_field_at_centre_t[component] - input.model_field_at_centre_t[component];

        let mut variation_radius = 0.0;
        for axis in 0..3 {
            let derivative_bound =
                Interval::point(input.residual_jacobian_abs_supremum_t_per_m[component][axis])
                    .expect("validated finite Jacobian bound");
            let cell_radius =
                Interval::point(half_widths[axis]).expect("validated finite half-width");
            let contribution = (derivative_bound * cell_radius)
                .abs_upper_bound()
                .ok_or(DiscrepancyError::NonFiniteResidualRadius { component })?;
            let accumulated = Interval::point(variation_radius).expect("finite accumulated radius")
                + Interval::point(contribution).expect("finite contribution");
            variation_radius = accumulated
                .abs_upper_bound()
                .ok_or(DiscrepancyError::NonFiniteResidualRadius { component })?;
        }

        if !variation_radius.is_finite() {
            return Err(DiscrepancyError::NonFiniteResidualRadius { component });
        }
        residual_variation_radius_t[component] = variation_radius;
        let variation_interval = Interval::new(-variation_radius, variation_radius)
            .expect("symmetric finite residual variation");
        residual_field_over_cell_t[component] = centre_residual_t[component] + variation_interval;
        physical_field_over_cell_t[component] =
            input.model_field_over_cell_t[component] + residual_field_over_cell_t[component];

        if !residual_field_over_cell_t[component].is_finite()
            || !physical_field_over_cell_t[component].is_finite()
        {
            return Err(DiscrepancyError::NonFinitePhysicalFieldEnclosure { component });
        }
    }

    Ok(PhysicalFieldDiscrepancyCertificate {
        domain_id: input.domain_id.clone(),
        cell_id: input.cell_id.clone(),
        cell_bounds_m: input.cell_bounds_m,
        centre_position_m: input.centre_position_m,
        measured_field_at_centre_t: input.measured_field_at_centre_t,
        model_field_at_centre_t: input.model_field_at_centre_t,
        centre_residual_t,
        residual_variation_radius_t,
        residual_field_over_cell_t,
        model_field_over_cell_t: input.model_field_over_cell_t,
        physical_field_over_cell_t,
        centre_measurement_id: input.centre_measurement_id.clone(),
        centre_measurement_sha256: input.centre_measurement_sha256.clone(),
        measurement_uncertainty_record_id: input.measurement_uncertainty_record_id.clone(),
        probe_calibration_id: input.probe_calibration_id.clone(),
        registration_record_id: input.registration_record_id.clone(),
        background_control_record_id: input.background_control_record_id.clone(),
        temperature_record_id: input.temperature_record_id.clone(),
        residual_jacobian_record_id: input.residual_jacobian_record_id.clone(),
        residual_jacobian_component_record_ids: input.residual_jacobian_component_record_ids.clone(),
        residual_jacobian_component_expression_ids: input.residual_jacobian_component_expression_ids.clone(),
        residual_jacobian_source_revision: input.residual_jacobian_source_revision.clone(),
        residual_jacobian_implementation_id: input.residual_jacobian_implementation_id.clone(),
        residual_jacobian_artifact_sha256: input.residual_jacobian_artifact_sha256.clone(),
        residual_jacobian_environment_sha256: input.residual_jacobian_environment_sha256.clone(),
        residual_jacobian_cell_id: input.residual_jacobian_cell_id.clone(),
        residual_jacobian_abs_supremum_t_per_m: input.residual_jacobian_abs_supremum_t_per_m,
        model_run_id: input.model_run_id.clone(),
        model_source_revision: input.model_source_revision.clone(),
        model_implementation_id: input.model_implementation_id.clone(),
        model_artifact_sha256: input.model_artifact_sha256.clone(),
        model_environment_sha256: input.model_environment_sha256.clone(),
        model_set_id: input.model_set_id.clone(),
        model_input_sha256: input.model_input_sha256.clone(),
        coordinate_frame_id: input.coordinate_frame_id.clone(),
        operating_condition_id: input.operating_condition_id.clone(),
        geometry_sha256: input.geometry_sha256.clone(),
        material_sha256: input.material_sha256.clone(),
        requirement_id: input.requirement_id.clone(),
        solver_run_id: input.solver_run_id.clone(),
        field_unit: input.field_unit.clone(),
        residual_jacobian_unit: format!("{}/metre", input.field_unit),
        arithmetic_contract: "binary64-nearest-plus-one-ulp-outward-v0.1".to_owned(),
        evidence_role: input.evidence_role,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIGEST_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const DIGEST_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const DIGEST_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

    fn interval(lower: f64, upper: f64) -> Interval {
        Interval::new(lower, upper).expect("valid interval")
    }

    fn input() -> CellDiscrepancyInput {
        CellDiscrepancyInput {
            domain_id: "magnetic-domain-v1".to_owned(),
            cell_id: "cell-0001".to_owned(),
            cell_bounds_m: [
                interval(-1.0, 1.0),
                interval(-2.0, 2.0),
                interval(-0.5, 0.5),
            ],
            centre_position_m: [0.0, 0.0, 0.0],
            centre_measurement_id: "measurement-centre-001".to_owned(),
            centre_measurement_sha256: DIGEST_A.to_owned(),
            measured_field_at_centre_t: [
                interval(0.09, 0.11),
                interval(0.19, 0.21),
                interval(0.29, 0.31),
            ],
            measurement_uncertainty_record_id: "measurement-uncertainty-001".to_owned(),
            probe_calibration_id: "probe-calibration-001".to_owned(),
            registration_record_id: "registration-001".to_owned(),
            background_control_record_id: "background-001".to_owned(),
            temperature_record_id: "temperature-001".to_owned(),
            model_run_id: "model-run-001".to_owned(),
            model_source_revision: "model-source-v1".to_owned(),
            model_implementation_id: "magnetostatic-model-v1".to_owned(),
            model_artifact_sha256: DIGEST_B.to_owned(),
            model_environment_sha256: DIGEST_C.to_owned(),
            model_set_id: "model-set-001".to_owned(),
            model_input_sha256: DIGEST_B.to_owned(),
            model_field_at_centre_t: [
                interval(-0.001, 0.001),
                interval(-0.001, 0.001),
                interval(-0.001, 0.001),
            ],
            model_field_over_cell_t: [
                interval(-0.2, 0.2),
                interval(-0.3, 0.3),
                interval(-0.4, 0.4),
            ],
            residual_jacobian_record_id: "residual-jacobian-001".to_owned(),
            residual_jacobian_component_record_ids: [
                "residual-jacobian-bx-001".to_owned(),
                "residual-jacobian-by-001".to_owned(),
                "residual-jacobian-bz-001".to_owned(),
            ],
            residual_jacobian_component_expression_ids: [
                "residual-bx-expression".to_owned(),
                "residual-by-expression".to_owned(),
                "residual-bz-expression".to_owned(),
            ],
            residual_jacobian_source_revision: "jacobian-source-v1".to_owned(),
            residual_jacobian_implementation_id: "interval-gradient-v1".to_owned(),
            residual_jacobian_artifact_sha256: DIGEST_A.to_owned(),
            residual_jacobian_environment_sha256: DIGEST_B.to_owned(),
            residual_jacobian_cell_id: "cell-0001".to_owned(),
            residual_jacobian_abs_supremum_t_per_m: [
                [0.1, 0.2, 0.3],
                [0.05, 0.1, 0.0],
                [0.0, 0.2, 0.1],
            ],
            coordinate_frame_id: "magnet-frame-v1".to_owned(),
            operating_condition_id: "room-gap-nominal".to_owned(),
            geometry_sha256: DIGEST_C.to_owned(),
            material_sha256: DIGEST_A.to_owned(),
            requirement_id: "minimum-field-reserve".to_owned(),
            solver_run_id: "solver-run-001".to_owned(),
            field_unit: "tesla".to_owned(),
            evidence_role: EvidenceRole::Calibration,
        }
    }

    #[test]
    fn affine_residual_enclosure_contains_known_cell_wide_residual() {
        let result =
            certify_physical_field_discrepancy(&input()).expect("valid affine residual evidence");

        let expected_radii = [0.65, 0.25, 0.45];
        for component in 0..3 {
            assert!(result.residual_variation_radius_t[component] >= expected_radii[component]);
            assert!(
                result.residual_field_over_cell_t[component].lower()
                    <= result.centre_residual_t[component].lower() - expected_radii[component]
            );
            assert!(
                result.residual_field_over_cell_t[component].upper()
                    >= result.centre_residual_t[component].upper() + expected_radii[component]
            );
            assert!(
                result.physical_field_over_cell_t[component].lower()
                    <= result.model_field_over_cell_t[component].lower()
                        + result.residual_field_over_cell_t[component].lower()
            );
            assert!(
                result.physical_field_over_cell_t[component].upper()
                    >= result.model_field_over_cell_t[component].upper()
                        + result.residual_field_over_cell_t[component].upper()
            );
        }

        let centre_residual = [0.1_f64, 0.2_f64, 0.3_f64];
        for x_sign in [-1.0_f64, 1.0] {
            for y_sign in [-1.0_f64, 1.0] {
                for z_sign in [-1.0_f64, 1.0] {
                    let coordinates = [x_sign, 2.0 * y_sign, 0.5 * z_sign];
                    let gradient = [
                        [0.1_f64, 0.2_f64, 0.3_f64],
                        [0.05_f64, 0.1_f64, 0.0_f64],
                        [0.0_f64, 0.2_f64, 0.1_f64],
                    ];
                    for component in 0..3 {
                        let residual = centre_residual[component]
                            + (0..3)
                                .map(|axis| gradient[component][axis] * coordinates[axis])
                                .sum::<f64>();
                        assert!(
                            residual >= result.residual_field_over_cell_t[component].lower()
                                && residual <= result.residual_field_over_cell_t[component].upper()
                        );
                    }
                }
            }
        }
        assert_eq!(result.residual_jacobian_unit, "tesla/metre");
        assert_eq!(result.centre_measurement_id, "measurement-centre-001");
    }

    #[test]
    fn zero_width_cell_reduces_to_the_centre_residual() {
        let mut evidence = input();
        evidence.cell_bounds_m = [interval(0.0, 0.0), interval(0.0, 0.0), interval(0.0, 0.0)];
        let result = certify_physical_field_discrepancy(&evidence).expect("point cell");
        for component in 0..3 {
            assert_eq!(result.residual_variation_radius_t[component], 0.0);
            assert_eq!(
                result.residual_field_over_cell_t[component],
                result.centre_residual_t[component]
            );
        }
    }

    #[test]
    fn wider_measurement_uncertainty_cannot_tighten_physical_field_bounds() {
        let original = certify_physical_field_discrepancy(&input()).expect("original");
        let mut wider = input();
        wider.measured_field_at_centre_t[0] = interval(0.05, 0.15);
        let result =
            certify_physical_field_discrepancy(&wider).expect("wider measurement interval");
        assert!(
            result.physical_field_over_cell_t[0].lower()
                <= original.physical_field_over_cell_t[0].lower()
        );
        assert!(
            result.physical_field_over_cell_t[0].upper()
                >= original.physical_field_over_cell_t[0].upper()
        );
    }

    #[test]
    fn measurement_must_be_at_exact_cell_centre() {
        let mut evidence = input();
        evidence.centre_position_m[0] = 0.01;
        assert_eq!(
            certify_physical_field_discrepancy(&evidence).expect_err("wrong centre sample"),
            DiscrepancyError::CentreNotAtCellMidpoint { axis: 0 }
        );
    }

    #[test]
    fn residual_jacobian_must_be_bound_to_exact_cell() {
        let mut evidence = input();
        evidence.residual_jacobian_cell_id = "another-cell".to_owned();
        assert_eq!(
            certify_physical_field_discrepancy(&evidence).expect_err("wrong Jacobian cell"),
            DiscrepancyError::JacobianCellIdentityMismatch
        );
    }

    #[test]
    fn negative_or_nonfinite_jacobian_suprema_are_rejected() {
        let mut negative = input();
        negative.residual_jacobian_abs_supremum_t_per_m[1][2] = -0.01;
        assert_eq!(
            certify_physical_field_discrepancy(&negative).expect_err("negative absolute supremum"),
            DiscrepancyError::InvalidJacobianBound {
                field_component: 1,
                spatial_axis: 2
            }
        );

        let mut nonfinite = input();
        nonfinite.residual_jacobian_abs_supremum_t_per_m[2][1] = f64::INFINITY;
        assert_eq!(
            certify_physical_field_discrepancy(&nonfinite).expect_err("unbounded supremum"),
            DiscrepancyError::InvalidJacobianBound {
                field_component: 2,
                spatial_axis: 1
            }
        );
    }

    #[test]
    fn model_centre_must_be_inside_model_cell_enclosure() {
        let mut evidence = input();
        evidence.model_field_over_cell_t[0] = interval(0.1, 0.2);
        assert_eq!(
            certify_physical_field_discrepancy(&evidence)
                .expect_err("inconsistent model enclosures"),
            DiscrepancyError::ModelCentreOutsideCellEnclosure { component: 0 }
        );
    }

    #[test]
    fn unbounded_measurement_interval_fails_closed() {
        let mut evidence = input();
        evidence.measured_field_at_centre_t[1] = Interval::ENTIRE;
        assert_eq!(
            certify_physical_field_discrepancy(&evidence)
                .expect_err("unbounded measurement uncertainty"),
            DiscrepancyError::UnboundedFieldInterval {
                field: "measured_field_at_centre_t",
                component: 1
            }
        );
    }

    #[test]
    fn functional_requirement_identity_is_required() {
        let mut evidence = input();
        evidence.requirement_id.clear();
        assert_eq!(
            certify_physical_field_discrepancy(&evidence)
                .expect_err("functional requirement identity"),
            DiscrepancyError::InvalidIdentity
        );
    }

    #[test]
    fn calibration_and_registration_identities_are_required() {
        let mut evidence = input();
        evidence.probe_calibration_id.clear();
        assert_eq!(
            certify_physical_field_discrepancy(&evidence)
                .expect_err("missing calibration identity"),
            DiscrepancyError::InvalidIdentity
        );

        let mut registration = input();
        registration.registration_record_id.clear();
        assert_eq!(
            certify_physical_field_discrepancy(&registration)
                .expect_err("missing registration identity"),
            DiscrepancyError::InvalidIdentity
        );
    }

    #[test]
    fn certificate_retains_source_artifact_and_requirement_identities() {
        let mut evidence = input();
        evidence.evidence_role = EvidenceRole::BlindHoldout;
        let result = certify_physical_field_discrepancy(&evidence)
            .expect("holdout evidence may be evaluated independently");
        assert_eq!(result.model_source_revision, "model-source-v1");
        assert_eq!(result.model_implementation_id, "magnetostatic-model-v1");
        assert_eq!(result.model_artifact_sha256, DIGEST_B);
        assert_eq!(
            result.residual_jacobian_source_revision,
            "jacobian-source-v1"
        );
        assert_eq!(result.residual_jacobian_artifact_sha256, DIGEST_A);
        assert_eq!(result.requirement_id, "minimum-field-reserve");
        assert_eq!(result.evidence_role, EvidenceRole::BlindHoldout);
    }

    #[test]
    fn invalid_solver_or_jacobian_artifact_digest_fails_closed() {
        let mut model = input();
        model.model_artifact_sha256 = "bad-digest".to_owned();
        assert_eq!(
            certify_physical_field_discrepancy(&model).expect_err("solver artifact identity"),
            DiscrepancyError::InvalidSha256 {
                field: "model_artifact_sha256"
            }
        );

        let mut jacobian = input();
        jacobian.residual_jacobian_artifact_sha256 = "bad-digest".to_owned();
        assert_eq!(
            certify_physical_field_discrepancy(&jacobian).expect_err("Jacobian artifact identity"),
            DiscrepancyError::InvalidSha256 {
                field: "residual_jacobian_artifact_sha256"
            }
        );
    }


    #[test]
    fn increasing_residual_jacobian_suprema_cannot_tighten_physical_field_bounds() {
        let original = certify_physical_field_discrepancy(&input()).expect("original bounds");
        let mut wider = input();
        for component in 0..3 {
            for axis in 0..3 {
                wider.residual_jacobian_abs_supremum_t_per_m[component][axis] *= 2.0;
            }
        }
        let result = certify_physical_field_discrepancy(&wider).expect("wider Jacobian bounds");
        for component in 0..3 {
            assert!(
                result.physical_field_over_cell_t[component].lower()
                    <= original.physical_field_over_cell_t[component].lower()
            );
            assert!(
                result.physical_field_over_cell_t[component].upper()
                    >= original.physical_field_over_cell_t[component].upper()
            );
        }
    }

}
