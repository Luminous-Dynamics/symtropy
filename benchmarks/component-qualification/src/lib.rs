//! Synthetic-first component dimensional qualification and assembly consistency.
//!
//! This module consumes declared requirements and external evidence references.
//! It is not a design code, NDT interpreter, product certification or approval authority.

use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentSnapshot {
    pub component_id: String,
    pub serial_id: String,
    pub component_type: String,
    pub design_revision: String,
    pub specification_revision: String,
    pub material_lot_reference: String,
    /// External custody/provenance receipt reference; not authenticated here.
    pub provenance_reference: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DimensionalRequirement {
    pub requirement_id: String,
    pub component_type: String,
    pub design_revision: String,
    pub specification_revision: String,
    pub datum_id: String,
    pub unit_id: String,
    pub nominal_micro_units: i64,
    pub lower_tolerance_micro_units: u64,
    pub upper_tolerance_micro_units: u64,
    pub accepted_method_ids: Vec<String>,
    pub maximum_measurement_age_ticks: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasurementEvidenceState {
    Current,
    Disputed,
    Revoked,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DimensionalMeasurement {
    pub measurement_id: String,
    pub component_id: String,
    pub serial_id: String,
    pub design_revision: String,
    pub specification_revision: String,
    pub method_id: String,
    pub instrument_id: String,
    pub calibration_valid_through_tick: Option<u64>,
    pub observed_at_tick: u64,
    pub unit_id: String,
    pub datum_id: String,
    pub measured_micro_units: i64,
    pub expanded_uncertainty_micro_units: u64,
    pub evidence_state: MeasurementEvidenceState,
    /// External inspection report reference; not authenticated here.
    pub report_reference: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentDisposition {
    AcceptedForSyntheticRequirementSet,
    RejectedByDeclaredRequirement,
    UnknownReviewRequired,
    EvidenceInconsistent,
    UnsupportedRequirementOrMethod,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentReason {
    InvalidComponentIdentity,
    MissingMaterialLotReference,
    MissingProvenanceReference,
    InvalidRequirement,
    UnsupportedComponentType,
    DesignRevisionMismatch,
    SpecificationRevisionMismatch,
    MeasurementIdentityInvalid,
    MeasurementSubjectMismatch,
    MeasurementDesignRevisionMismatch,
    MeasurementSpecificationMismatch,
    UnsupportedMeasurementMethod,
    UnitMismatch,
    DatumMismatch,
    CalibrationCurrentnessUnknown,
    CalibrationExpired,
    MeasurementFromFuture,
    MeasurementStale,
    MeasurementEvidenceDisputed,
    MeasurementEvidenceRevoked,
    MeasurementEvidenceUnknown,
    MissingReportReference,
    EntireIntervalBelowLowerLimit,
    EntireIntervalAboveUpperLimit,
    MeasurementIntervalOverlapsLimit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentQualificationResult {
    pub disposition: ComponentDisposition,
    pub reasons: Vec<ComponentReason>,
    pub permitted_interval_lower: i128,
    pub permitted_interval_upper: i128,
    pub measured_interval_lower: i128,
    pub measured_interval_upper: i128,
}

/// Evaluate one scalar dimensional requirement.
///
/// Measurement interval = [x - u, x + u].
/// Acceptance requires the *complete interval* to lie inside the tolerance band.
/// Rejection requires the complete interval to be outside one side of the band.
/// Overlap is indeterminate, never silently accepted.
#[must_use]
pub fn evaluate_dimension(
    component: &ComponentSnapshot,
    requirement: &DimensionalRequirement,
    measurement: &DimensionalMeasurement,
    evaluation_tick: u64,
) -> ComponentQualificationResult {
    let mut reasons = Vec::new();
    let mut inconsistent = false;
    let mut unsupported = false;
    let mut unknown = false;

    if component.component_id.trim().is_empty()
        || component.serial_id.trim().is_empty()
        || component.component_type.trim().is_empty()
        || component.design_revision.trim().is_empty()
        || component.specification_revision.trim().is_empty()
    {
        reasons.push(ComponentReason::InvalidComponentIdentity);
        inconsistent = true;
    }
    if component.material_lot_reference.trim().is_empty() {
        reasons.push(ComponentReason::MissingMaterialLotReference);
        unknown = true;
    }
    if component.provenance_reference.trim().is_empty() {
        reasons.push(ComponentReason::MissingProvenanceReference);
        unknown = true;
    }
    if requirement.requirement_id.trim().is_empty()
        || requirement.component_type.trim().is_empty()
        || requirement.design_revision.trim().is_empty()
        || requirement.specification_revision.trim().is_empty()
        || requirement.datum_id.trim().is_empty()
        || requirement.unit_id.trim().is_empty()
        || requirement.accepted_method_ids.is_empty()
        || requirement
            .accepted_method_ids
            .iter()
            .any(|id| id.trim().is_empty())
    {
        reasons.push(ComponentReason::InvalidRequirement);
        inconsistent = true;
    }
    if component.component_type != requirement.component_type {
        reasons.push(ComponentReason::UnsupportedComponentType);
        unsupported = true;
    }
    if component.design_revision != requirement.design_revision {
        reasons.push(ComponentReason::DesignRevisionMismatch);
        inconsistent = true;
    }
    if component.specification_revision != requirement.specification_revision {
        reasons.push(ComponentReason::SpecificationRevisionMismatch);
        inconsistent = true;
    }
    if measurement.measurement_id.trim().is_empty()
        || measurement.instrument_id.trim().is_empty()
        || measurement.method_id.trim().is_empty()
    {
        reasons.push(ComponentReason::MeasurementIdentityInvalid);
        inconsistent = true;
    }
    if measurement.component_id != component.component_id
        || measurement.serial_id != component.serial_id
    {
        reasons.push(ComponentReason::MeasurementSubjectMismatch);
        inconsistent = true;
    }
    if measurement.design_revision != requirement.design_revision
        || measurement.design_revision != component.design_revision
    {
        reasons.push(ComponentReason::MeasurementDesignRevisionMismatch);
        inconsistent = true;
    }
    if measurement.specification_revision != requirement.specification_revision
        || measurement.specification_revision != component.specification_revision
    {
        reasons.push(ComponentReason::MeasurementSpecificationMismatch);
        inconsistent = true;
    }
    if !requirement
        .accepted_method_ids
        .contains(&measurement.method_id)
    {
        reasons.push(ComponentReason::UnsupportedMeasurementMethod);
        unsupported = true;
    }
    if measurement.unit_id != requirement.unit_id {
        reasons.push(ComponentReason::UnitMismatch);
        inconsistent = true;
    }
    if measurement.datum_id != requirement.datum_id {
        reasons.push(ComponentReason::DatumMismatch);
        inconsistent = true;
    }
    if measurement.report_reference.trim().is_empty() {
        reasons.push(ComponentReason::MissingReportReference);
        unknown = true;
    }

    match measurement.calibration_valid_through_tick {
        None => {
            reasons.push(ComponentReason::CalibrationCurrentnessUnknown);
            unknown = true;
        }
        Some(expiry) if expiry < evaluation_tick => {
            reasons.push(ComponentReason::CalibrationExpired);
            unknown = true;
        }
        Some(_) => {}
    }
    if measurement.observed_at_tick > evaluation_tick {
        reasons.push(ComponentReason::MeasurementFromFuture);
        inconsistent = true;
    } else if evaluation_tick - measurement.observed_at_tick
        > requirement.maximum_measurement_age_ticks
    {
        reasons.push(ComponentReason::MeasurementStale);
        unknown = true;
    }

    match measurement.evidence_state {
        MeasurementEvidenceState::Current => {}
        MeasurementEvidenceState::Disputed => {
            reasons.push(ComponentReason::MeasurementEvidenceDisputed);
            unknown = true;
        }
        MeasurementEvidenceState::Revoked => {
            reasons.push(ComponentReason::MeasurementEvidenceRevoked);
            inconsistent = true;
        }
        MeasurementEvidenceState::Unknown => {
            reasons.push(ComponentReason::MeasurementEvidenceUnknown);
            unknown = true;
        }
    }

    let nominal = i128::from(requirement.nominal_micro_units);
    let permitted_lower = nominal - i128::from(requirement.lower_tolerance_micro_units);
    let permitted_upper = nominal + i128::from(requirement.upper_tolerance_micro_units);
    let measured = i128::from(measurement.measured_micro_units);
    let uncertainty = i128::from(measurement.expanded_uncertainty_micro_units);
    let measured_lower = measured - uncertainty;
    let measured_upper = measured + uncertainty;

    if !inconsistent && !unsupported && !unknown {
        if measured_upper < permitted_lower {
            reasons.push(ComponentReason::EntireIntervalBelowLowerLimit);
        } else if measured_lower > permitted_upper {
            reasons.push(ComponentReason::EntireIntervalAboveUpperLimit);
        } else if measured_lower >= permitted_lower && measured_upper <= permitted_upper {
            // The whole uncertainty interval is inside the allowed band.
        } else {
            reasons.push(ComponentReason::MeasurementIntervalOverlapsLimit);
            unknown = true;
        }
    }

    let disposition = if inconsistent {
        ComponentDisposition::EvidenceInconsistent
    } else if unsupported {
        ComponentDisposition::UnsupportedRequirementOrMethod
    } else if unknown {
        ComponentDisposition::UnknownReviewRequired
    } else if reasons.contains(&ComponentReason::EntireIntervalBelowLowerLimit)
        || reasons.contains(&ComponentReason::EntireIntervalAboveUpperLimit)
    {
        ComponentDisposition::RejectedByDeclaredRequirement
    } else {
        ComponentDisposition::AcceptedForSyntheticRequirementSet
    };

    ComponentQualificationResult {
        disposition,
        reasons,
        permitted_interval_lower: permitted_lower,
        permitted_interval_upper: permitted_upper,
        measured_interval_lower: measured_lower,
        measured_interval_upper: measured_upper,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequiredAssemblyMember {
    pub component_id: String,
    pub design_revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedAssemblyMember {
    pub component_id: String,
    pub design_revision: String,
    pub qualification: ComponentDisposition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssemblyRequirement {
    pub assembly_id: String,
    pub assembly_revision: String,
    pub required_members: Vec<RequiredAssemblyMember>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssemblyDisposition {
    AcceptedForSyntheticAssembly,
    RejectedByChildComponent,
    UnknownReviewRequired,
    EvidenceInconsistent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssemblyReason {
    InvalidAssemblyIdentity,
    DuplicateRequiredMember,
    DuplicateObservedMember,
    MissingRequiredMember,
    UnexpectedMember,
    MemberRevisionMismatch,
    ChildComponentRejected,
    ChildComponentNotQualified,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssemblyQualificationResult {
    pub disposition: AssemblyDisposition,
    pub reasons: Vec<AssemblyReason>,
}

/// Confirm exact membership/revision and that every required child has a valid
/// synthetic component disposition. This is not a structural assembly analysis.
#[must_use]
pub fn evaluate_assembly(
    requirement: &AssemblyRequirement,
    observed_members: &[ObservedAssemblyMember],
) -> AssemblyQualificationResult {
    let mut reasons = Vec::new();
    let mut inconsistent = false;
    let mut unknown = false;
    let mut rejected = false;

    if requirement.assembly_id.trim().is_empty()
        || requirement.assembly_revision.trim().is_empty()
        || requirement.required_members.is_empty()
    {
        reasons.push(AssemblyReason::InvalidAssemblyIdentity);
        inconsistent = true;
    }

    let mut required_ids = BTreeSet::new();
    for member in &requirement.required_members {
        if member.component_id.trim().is_empty()
            || member.design_revision.trim().is_empty()
            || !required_ids.insert(member.component_id.as_str())
        {
            reasons.push(AssemblyReason::DuplicateRequiredMember);
            inconsistent = true;
        }
    }

    let mut observed_ids = BTreeSet::new();
    for member in observed_members {
        if member.component_id.trim().is_empty()
            || !observed_ids.insert(member.component_id.as_str())
        {
            reasons.push(AssemblyReason::DuplicateObservedMember);
            inconsistent = true;
        }
        if !required_ids.contains(member.component_id.as_str()) {
            reasons.push(AssemblyReason::UnexpectedMember);
            inconsistent = true;
        }
    }

    for required in &requirement.required_members {
        let Some(observed) = observed_members
            .iter()
            .find(|member| member.component_id == required.component_id)
        else {
            reasons.push(AssemblyReason::MissingRequiredMember);
            unknown = true;
            continue;
        };

        if observed.design_revision != required.design_revision {
            reasons.push(AssemblyReason::MemberRevisionMismatch);
            inconsistent = true;
        }
        match observed.qualification {
            ComponentDisposition::AcceptedForSyntheticRequirementSet => {}
            ComponentDisposition::RejectedByDeclaredRequirement => {
                reasons.push(AssemblyReason::ChildComponentRejected);
                rejected = true;
            }
            ComponentDisposition::UnknownReviewRequired
            | ComponentDisposition::EvidenceInconsistent
            | ComponentDisposition::UnsupportedRequirementOrMethod => {
                reasons.push(AssemblyReason::ChildComponentNotQualified);
                unknown = true;
            }
        }
    }

    let disposition = if inconsistent {
        AssemblyDisposition::EvidenceInconsistent
    } else if rejected {
        AssemblyDisposition::RejectedByChildComponent
    } else if unknown {
        AssemblyDisposition::UnknownReviewRequired
    } else {
        AssemblyDisposition::AcceptedForSyntheticAssembly
    };

    AssemblyQualificationResult {
        disposition,
        reasons,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn component() -> ComponentSnapshot {
        ComponentSnapshot {
            component_id: "tbm-segment-001".into(),
            serial_id: "serial-7781".into(),
            component_type: "precast-tbm-lining-segment".into(),
            design_revision: "design-r4".into(),
            specification_revision: "spec-r2".into(),
            material_lot_reference: "concrete-batch-42".into(),
            provenance_reference: "mycelix-receipt-91".into(),
        }
    }

    fn requirement() -> DimensionalRequirement {
        DimensionalRequirement {
            requirement_id: "segment-thickness".into(),
            component_type: "precast-tbm-lining-segment".into(),
            design_revision: "design-r4".into(),
            specification_revision: "spec-r2".into(),
            datum_id: "segment-local-datum-a".into(),
            unit_id: "micrometre".into(),
            nominal_micro_units: 600_000,
            lower_tolerance_micro_units: 2_000,
            upper_tolerance_micro_units: 2_000,
            accepted_method_ids: vec!["calibrated-laser-scan-v1".into()],
            maximum_measurement_age_ticks: 20,
        }
    }

    fn measurement() -> DimensionalMeasurement {
        DimensionalMeasurement {
            measurement_id: "measure-500".into(),
            component_id: "tbm-segment-001".into(),
            serial_id: "serial-7781".into(),
            design_revision: "design-r4".into(),
            specification_revision: "spec-r2".into(),
            method_id: "calibrated-laser-scan-v1".into(),
            instrument_id: "laser-tracker-03".into(),
            calibration_valid_through_tick: Some(500),
            observed_at_tick: 95,
            unit_id: "micrometre".into(),
            datum_id: "segment-local-datum-a".into(),
            measured_micro_units: 600_000,
            expanded_uncertainty_micro_units: 500,
            evidence_state: MeasurementEvidenceState::Current,
            report_reference: "inspection-report-500".into(),
        }
    }

    fn evaluate(
        c: &ComponentSnapshot,
        r: &DimensionalRequirement,
        m: &DimensionalMeasurement,
    ) -> ComponentQualificationResult {
        evaluate_dimension(c, r, m, 100)
    }

    #[test]
    fn complete_measurement_interval_inside_tolerance_is_accepted() {
        assert_eq!(
            evaluate(&component(), &requirement(), &measurement()).disposition,
            ComponentDisposition::AcceptedForSyntheticRequirementSet
        );
    }

    #[test]
    fn exact_lower_tolerance_boundary_is_accepted() {
        let mut m = measurement();
        m.measured_micro_units = 598_500;
        m.expanded_uncertainty_micro_units = 500;
        assert_eq!(
            evaluate(&component(), &requirement(), &m).disposition,
            ComponentDisposition::AcceptedForSyntheticRequirementSet
        );
    }

    #[test]
    fn exact_upper_tolerance_boundary_is_accepted() {
        let mut m = measurement();
        m.measured_micro_units = 601_500;
        m.expanded_uncertainty_micro_units = 500;
        assert_eq!(
            evaluate(&component(), &requirement(), &m).disposition,
            ComponentDisposition::AcceptedForSyntheticRequirementSet
        );
    }

    #[test]
    fn interval_entirely_below_tolerance_is_rejected() {
        let mut m = measurement();
        m.measured_micro_units = 597_000;
        m.expanded_uncertainty_micro_units = 500;
        assert_eq!(
            evaluate(&component(), &requirement(), &m).disposition,
            ComponentDisposition::RejectedByDeclaredRequirement
        );
    }

    #[test]
    fn interval_entirely_above_tolerance_is_rejected() {
        let mut m = measurement();
        m.measured_micro_units = 603_000;
        m.expanded_uncertainty_micro_units = 500;
        assert_eq!(
            evaluate(&component(), &requirement(), &m).disposition,
            ComponentDisposition::RejectedByDeclaredRequirement
        );
    }

    #[test]
    fn central_value_inside_but_uncertainty_crosses_boundary_is_unknown() {
        let mut m = measurement();
        m.measured_micro_units = 601_900;
        m.expanded_uncertainty_micro_units = 500;
        assert_eq!(
            evaluate(&component(), &requirement(), &m).disposition,
            ComponentDisposition::UnknownReviewRequired
        );
    }

    #[test]
    fn interval_straddling_both_limits_is_unknown() {
        let mut m = measurement();
        m.measured_micro_units = 600_000;
        m.expanded_uncertainty_micro_units = 3_000;
        assert_eq!(
            evaluate(&component(), &requirement(), &m).disposition,
            ComponentDisposition::UnknownReviewRequired
        );
    }

    #[test]
    fn zero_uncertainty_uses_exact_measured_value() {
        let mut m = measurement();
        m.expanded_uncertainty_micro_units = 0;
        assert_eq!(
            evaluate(&component(), &requirement(), &m).disposition,
            ComponentDisposition::AcceptedForSyntheticRequirementSet
        );
    }

    #[test]
    fn component_design_revision_mismatch_is_inconsistent() {
        let mut c = component();
        c.design_revision = "design-r3".into();
        assert_eq!(
            evaluate(&c, &requirement(), &measurement()).disposition,
            ComponentDisposition::EvidenceInconsistent
        );
    }

    #[test]
    fn measurement_from_old_design_revision_is_inconsistent() {
        let mut m = measurement();
        m.design_revision = "design-r3".into();
        assert_eq!(
            evaluate(&component(), &requirement(), &m).disposition,
            ComponentDisposition::EvidenceInconsistent
        );
    }

    #[test]
    fn specification_revision_mismatch_is_inconsistent() {
        let mut m = measurement();
        m.specification_revision = "spec-r1".into();
        assert_eq!(
            evaluate(&component(), &requirement(), &m).disposition,
            ComponentDisposition::EvidenceInconsistent
        );
    }

    #[test]
    fn measurement_for_different_serial_is_inconsistent() {
        let mut m = measurement();
        m.serial_id = "serial-replacement".into();
        assert_eq!(
            evaluate(&component(), &requirement(), &m).disposition,
            ComponentDisposition::EvidenceInconsistent
        );
    }

    #[test]
    fn measurement_for_different_component_is_inconsistent() {
        let mut m = measurement();
        m.component_id = "other-component".into();
        assert_eq!(
            evaluate(&component(), &requirement(), &m).disposition,
            ComponentDisposition::EvidenceInconsistent
        );
    }

    #[test]
    fn missing_material_lot_reference_is_unknown() {
        let mut c = component();
        c.material_lot_reference.clear();
        assert_eq!(
            evaluate(&c, &requirement(), &measurement()).disposition,
            ComponentDisposition::UnknownReviewRequired
        );
    }

    #[test]
    fn missing_provenance_receipt_is_unknown() {
        let mut c = component();
        c.provenance_reference.clear();
        assert_eq!(
            evaluate(&c, &requirement(), &measurement()).disposition,
            ComponentDisposition::UnknownReviewRequired
        );
    }

    #[test]
    fn unsupported_component_type_is_unsupported() {
        let mut c = component();
        c.component_type = "steel-caisson-module".into();
        assert_eq!(
            evaluate(&c, &requirement(), &measurement()).disposition,
            ComponentDisposition::UnsupportedRequirementOrMethod
        );
    }

    #[test]
    fn unsupported_measurement_method_is_unsupported() {
        let mut m = measurement();
        m.method_id = "unqualified-tape-measure".into();
        assert_eq!(
            evaluate(&component(), &requirement(), &m).disposition,
            ComponentDisposition::UnsupportedRequirementOrMethod
        );
    }

    #[test]
    fn unit_mismatch_is_inconsistent() {
        let mut m = measurement();
        m.unit_id = "millimetre".into();
        assert_eq!(
            evaluate(&component(), &requirement(), &m).disposition,
            ComponentDisposition::EvidenceInconsistent
        );
    }

    #[test]
    fn datum_mismatch_is_inconsistent() {
        let mut m = measurement();
        m.datum_id = "other-datum".into();
        assert_eq!(
            evaluate(&component(), &requirement(), &m).disposition,
            ComponentDisposition::EvidenceInconsistent
        );
    }

    #[test]
    fn missing_calibration_currentness_is_unknown() {
        let mut m = measurement();
        m.calibration_valid_through_tick = None;
        assert_eq!(
            evaluate(&component(), &requirement(), &m).disposition,
            ComponentDisposition::UnknownReviewRequired
        );
    }

    #[test]
    fn expired_calibration_is_unknown() {
        let mut m = measurement();
        m.calibration_valid_through_tick = Some(99);
        assert_eq!(
            evaluate(&component(), &requirement(), &m).disposition,
            ComponentDisposition::UnknownReviewRequired
        );
    }

    #[test]
    fn stale_measurement_is_unknown() {
        let mut m = measurement();
        m.observed_at_tick = 70;
        assert_eq!(
            evaluate(&component(), &requirement(), &m).disposition,
            ComponentDisposition::UnknownReviewRequired
        );
    }

    #[test]
    fn future_measurement_is_inconsistent() {
        let mut m = measurement();
        m.observed_at_tick = 101;
        assert_eq!(
            evaluate(&component(), &requirement(), &m).disposition,
            ComponentDisposition::EvidenceInconsistent
        );
    }

    #[test]
    fn revoked_measurement_evidence_is_inconsistent() {
        let mut m = measurement();
        m.evidence_state = MeasurementEvidenceState::Revoked;
        assert_eq!(
            evaluate(&component(), &requirement(), &m).disposition,
            ComponentDisposition::EvidenceInconsistent
        );
    }

    #[test]
    fn disputed_measurement_evidence_requires_review() {
        let mut m = measurement();
        m.evidence_state = MeasurementEvidenceState::Disputed;
        assert_eq!(
            evaluate(&component(), &requirement(), &m).disposition,
            ComponentDisposition::UnknownReviewRequired
        );
    }

    #[test]
    fn missing_inspection_report_is_unknown() {
        let mut m = measurement();
        m.report_reference.clear();
        assert_eq!(
            evaluate(&component(), &requirement(), &m).disposition,
            ComponentDisposition::UnknownReviewRequired
        );
    }

    #[test]
    fn blank_requirement_is_inconsistent() {
        let mut r = requirement();
        r.requirement_id.clear();
        assert_eq!(
            evaluate(&component(), &r, &measurement()).disposition,
            ComponentDisposition::EvidenceInconsistent
        );
    }

    #[test]
    fn blank_component_identity_is_inconsistent() {
        let mut c = component();
        c.serial_id.clear();
        assert_eq!(
            evaluate(&c, &requirement(), &measurement()).disposition,
            ComponentDisposition::EvidenceInconsistent
        );
    }

    fn assembly_requirement() -> AssemblyRequirement {
        AssemblyRequirement {
            assembly_id: "tbm-ring-004".into(),
            assembly_revision: "ring-r5".into(),
            required_members: vec![
                RequiredAssemblyMember {
                    component_id: "segment-a".into(),
                    design_revision: "design-r4".into(),
                },
                RequiredAssemblyMember {
                    component_id: "segment-b".into(),
                    design_revision: "design-r4".into(),
                },
            ],
        }
    }

    fn assembly_members() -> Vec<ObservedAssemblyMember> {
        vec![
            ObservedAssemblyMember {
                component_id: "segment-a".into(),
                design_revision: "design-r4".into(),
                qualification: ComponentDisposition::AcceptedForSyntheticRequirementSet,
            },
            ObservedAssemblyMember {
                component_id: "segment-b".into(),
                design_revision: "design-r4".into(),
                qualification: ComponentDisposition::AcceptedForSyntheticRequirementSet,
            },
        ]
    }

    #[test]
    fn assembly_accepts_exact_members_with_accepted_children() {
        assert_eq!(
            evaluate_assembly(&assembly_requirement(), &assembly_members()).disposition,
            AssemblyDisposition::AcceptedForSyntheticAssembly
        );
    }

    #[test]
    fn assembly_with_rejected_child_is_rejected() {
        let mut members = assembly_members();
        members[1].qualification = ComponentDisposition::RejectedByDeclaredRequirement;
        assert_eq!(
            evaluate_assembly(&assembly_requirement(), &members).disposition,
            AssemblyDisposition::RejectedByChildComponent
        );
    }

    #[test]
    fn assembly_with_unknown_child_requires_review() {
        let mut members = assembly_members();
        members[1].qualification = ComponentDisposition::UnknownReviewRequired;
        assert_eq!(
            evaluate_assembly(&assembly_requirement(), &members).disposition,
            AssemblyDisposition::UnknownReviewRequired
        );
    }

    #[test]
    fn assembly_missing_child_requires_review() {
        let mut members = assembly_members();
        members.pop();
        assert_eq!(
            evaluate_assembly(&assembly_requirement(), &members).disposition,
            AssemblyDisposition::UnknownReviewRequired
        );
    }

    #[test]
    fn assembly_with_duplicate_child_is_inconsistent() {
        let mut members = assembly_members();
        members.push(members[0].clone());
        assert_eq!(
            evaluate_assembly(&assembly_requirement(), &members).disposition,
            AssemblyDisposition::EvidenceInconsistent
        );
    }

    #[test]
    fn assembly_with_unexpected_child_is_inconsistent() {
        let mut members = assembly_members();
        members.push(ObservedAssemblyMember {
            component_id: "unexpected-segment".into(),
            design_revision: "design-r4".into(),
            qualification: ComponentDisposition::AcceptedForSyntheticRequirementSet,
        });
        assert_eq!(
            evaluate_assembly(&assembly_requirement(), &members).disposition,
            AssemblyDisposition::EvidenceInconsistent
        );
    }

    #[test]
    fn assembly_child_revision_mismatch_is_inconsistent() {
        let mut members = assembly_members();
        members[0].design_revision = "design-r3".into();
        assert_eq!(
            evaluate_assembly(&assembly_requirement(), &members).disposition,
            AssemblyDisposition::EvidenceInconsistent
        );
    }

    #[test]
    fn duplicate_required_member_is_inconsistent() {
        let mut requirement = assembly_requirement();
        requirement
            .required_members
            .push(requirement.required_members[0].clone());
        assert_eq!(
            evaluate_assembly(&requirement, &assembly_members()).disposition,
            AssemblyDisposition::EvidenceInconsistent
        );
    }
}
