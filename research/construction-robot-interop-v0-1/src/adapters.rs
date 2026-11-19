//! Two synthetic, differently shaped payload adapters for semantic differential tests.
//!
//! These structs are deliberately not claimed to implement VDA 5050, ISO 21423,
//! MassRobotics, or any vendor API. They exercise the construction-specific canonical
//! contract and make lossy/ambiguous mappings visible before real protocol adapters exist.

use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TaskFamily {
    Layout,
    Piling,
    Masonry,
    MaterialHandling,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapabilityRequirements {
    pub payload_grams: u64,
    pub reach_millimeters: u64,
    pub tool_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalTask {
    pub task_id: String,
    pub work_package_id: String,
    pub site_id: String,
    pub work_zone_id: String,
    pub task_family: TaskFamily,
    pub functional_unit_class: String,
    pub design_revision_digest: String,
    pub target_frame_id: String,
    pub site_epoch: String,
    pub capabilities: CapabilityRequirements,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VendorAWorkOrder {
    pub job_uuid: String,
    pub package_code: String,
    pub site_code: String,
    pub zone_code: String,
    pub operation_code: String,
    pub output_class: String,
    pub source_design_hash: String,
    pub coordinate_frame: String,
    pub worksite_epoch: String,
    pub payload_requirement_grams: u64,
    pub reach_requirement_mm: u64,
    pub tool_attachment: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VendorBTask {
    pub work_order_ref: String,
    pub package_ref: String,
    pub facility: String,
    pub zone: String,
    pub process: String,
    pub deliverable: String,
    pub model_digest: Option<String>,
    pub coordinate_reference: Option<String>,
    pub generation: Option<String>,
    /// Vendor B defines this field in milligrams; no inferred unit is allowed.
    pub payload_required_milligrams: u64,
    /// Vendor B defines this field in micrometres; no inferred unit is allowed.
    pub reach_required_micrometers: u64,
    pub tool_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AdapterError {
    MissingField(&'static str),
    EmptyField(&'static str),
    UnsupportedTaskFamily(String),
    NonIntegralUnitConversion(&'static str),
}

impl fmt::Display for AdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for AdapterError {}

pub fn normalize_vendor_a(input: VendorAWorkOrder) -> Result<CanonicalTask, AdapterError> {
    let task = CanonicalTask {
        task_id: input.job_uuid,
        work_package_id: input.package_code,
        site_id: input.site_code,
        work_zone_id: input.zone_code,
        task_family: parse_task_family(input.operation_code)?,
        functional_unit_class: input.output_class,
        design_revision_digest: input.source_design_hash,
        target_frame_id: input.coordinate_frame,
        site_epoch: input.worksite_epoch,
        capabilities: CapabilityRequirements {
            payload_grams: input.payload_requirement_grams,
            reach_millimeters: input.reach_requirement_mm,
            tool_id: input.tool_attachment,
        },
    };
    validate_canonical(task)
}

pub fn normalize_vendor_b(input: VendorBTask) -> Result<CanonicalTask, AdapterError> {
    let task = CanonicalTask {
        task_id: input.work_order_ref,
        work_package_id: input.package_ref,
        site_id: input.facility,
        work_zone_id: input.zone,
        task_family: parse_task_family(input.process)?,
        functional_unit_class: input.deliverable,
        design_revision_digest: required(input.model_digest, "model_digest")?,
        target_frame_id: required(input.coordinate_reference, "coordinate_reference")?,
        site_epoch: required(input.generation, "generation")?,
        capabilities: CapabilityRequirements {
            payload_grams: exact_division(
                input.payload_required_milligrams,
                1_000,
                "payload_required_milligrams",
            )?,
            reach_millimeters: exact_division(
                input.reach_required_micrometers,
                1_000,
                "reach_required_micrometers",
            )?,
            tool_id: required(input.tool_id, "tool_id")?,
        },
    };
    validate_canonical(task)
}

fn parse_task_family(value: String) -> Result<TaskFamily, AdapterError> {
    match value.as_str() {
        "layout" => Ok(TaskFamily::Layout),
        "piling" => Ok(TaskFamily::Piling),
        "masonry" => Ok(TaskFamily::Masonry),
        "material_handling" => Ok(TaskFamily::MaterialHandling),
        _ => Err(AdapterError::UnsupportedTaskFamily(value)),
    }
}

fn required(value: Option<String>, name: &'static str) -> Result<String, AdapterError> {
    value.ok_or(AdapterError::MissingField(name))
}

fn exact_division(value: u64, divisor: u64, name: &'static str) -> Result<u64, AdapterError> {
    if !value.is_multiple_of(divisor) {
        return Err(AdapterError::NonIntegralUnitConversion(name));
    }
    Ok(value / divisor)
}

fn validate_canonical(task: CanonicalTask) -> Result<CanonicalTask, AdapterError> {
    check_non_empty("task_id", &task.task_id)?;
    check_non_empty("work_package_id", &task.work_package_id)?;
    check_non_empty("site_id", &task.site_id)?;
    check_non_empty("work_zone_id", &task.work_zone_id)?;
    check_non_empty("functional_unit_class", &task.functional_unit_class)?;
    check_non_empty("design_revision_digest", &task.design_revision_digest)?;
    check_non_empty("target_frame_id", &task.target_frame_id)?;
    check_non_empty("site_epoch", &task.site_epoch)?;
    check_non_empty("tool_id", &task.capabilities.tool_id)?;
    Ok(task)
}

fn check_non_empty(name: &'static str, value: &str) -> Result<(), AdapterError> {
    if value.trim().is_empty() {
        return Err(AdapterError::EmptyField(name));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vendor_a() -> VendorAWorkOrder {
        VendorAWorkOrder {
            job_uuid: "task-42".to_owned(),
            package_code: "package-9".to_owned(),
            site_code: "site-JHB-1".to_owned(),
            zone_code: "zone-east".to_owned(),
            operation_code: "layout".to_owned(),
            output_class: "control-point".to_owned(),
            source_design_hash: "sha256:design-abc".to_owned(),
            coordinate_frame: "project-grid-2026".to_owned(),
            worksite_epoch: "epoch-17".to_owned(),
            payload_requirement_grams: 12_500,
            reach_requirement_mm: 8_000,
            tool_attachment: "layout-printer-v2".to_owned(),
        }
    }

    fn vendor_b() -> VendorBTask {
        VendorBTask {
            work_order_ref: "task-42".to_owned(),
            package_ref: "package-9".to_owned(),
            facility: "site-JHB-1".to_owned(),
            zone: "zone-east".to_owned(),
            process: "layout".to_owned(),
            deliverable: "control-point".to_owned(),
            model_digest: Some("sha256:design-abc".to_owned()),
            coordinate_reference: Some("project-grid-2026".to_owned()),
            generation: Some("epoch-17".to_owned()),
            payload_required_milligrams: 12_500_000,
            reach_required_micrometers: 8_000_000,
            tool_id: Some("layout-printer-v2".to_owned()),
        }
    }

    #[test]
    fn two_different_payload_shapes_normalize_to_the_same_task() {
        assert_eq!(
            normalize_vendor_a(vendor_a()),
            normalize_vendor_b(vendor_b())
        );
    }

    #[test]
    fn missing_frame_or_design_revision_is_not_defaulted() {
        let mut task = vendor_b();
        task.coordinate_reference = None;
        assert_eq!(
            normalize_vendor_b(task),
            Err(AdapterError::MissingField("coordinate_reference"))
        );

        let mut task = vendor_b();
        task.model_digest = None;
        assert_eq!(
            normalize_vendor_b(task),
            Err(AdapterError::MissingField("model_digest"))
        );
    }

    #[test]
    fn non_integral_mass_or_length_conversion_is_rejected() {
        let mut task = vendor_b();
        task.payload_required_milligrams += 1;
        assert_eq!(
            normalize_vendor_b(task),
            Err(AdapterError::NonIntegralUnitConversion(
                "payload_required_milligrams"
            ))
        );

        let mut task = vendor_b();
        task.reach_required_micrometers += 1;
        assert_eq!(
            normalize_vendor_b(task),
            Err(AdapterError::NonIntegralUnitConversion(
                "reach_required_micrometers"
            ))
        );
    }

    #[test]
    fn unknown_task_family_is_typed_rejection() {
        let mut task = vendor_a();
        task.operation_code = "unknown_process".to_owned();
        assert_eq!(
            normalize_vendor_a(task),
            Err(AdapterError::UnsupportedTaskFamily(
                "unknown_process".to_owned()
            ))
        );
    }

    #[test]
    fn changed_frame_or_site_epoch_is_not_semantically_equivalent() {
        let canonical_a = normalize_vendor_a(vendor_a()).expect("valid A task");
        let mut changed_b = vendor_b();
        changed_b.coordinate_reference = Some("local-frame".to_owned());
        let canonical_b = normalize_vendor_b(changed_b).expect("valid B task");
        assert_ne!(canonical_a, canonical_b);

        let mut changed_b = vendor_b();
        changed_b.generation = Some("epoch-18".to_owned());
        let canonical_b = normalize_vendor_b(changed_b).expect("valid B task");
        assert_ne!(canonical_a, canonical_b);
    }

    #[test]
    fn empty_required_field_is_rejected() {
        let mut task = vendor_a();
        task.source_design_hash.clear();
        assert_eq!(
            normalize_vendor_a(task),
            Err(AdapterError::EmptyField("design_revision_digest"))
        );
    }

    #[test]
    fn task_id_or_output_class_difference_changes_semantic_identity() {
        let canonical_a = normalize_vendor_a(vendor_a()).expect("valid A task");
        let mut changed_b = vendor_b();
        changed_b.deliverable = "wall-area".to_owned();
        let canonical_b = normalize_vendor_b(changed_b).expect("valid B task");
        assert_ne!(canonical_a, canonical_b);
    }
}
