//! Typed preflight adapter for IFC 4.3 ADD2 + buildingSMART IDS 1.0.
//!
//! This module does not parse STEP/IFC or execute IDS XML. It validates a
//! typed record supplied by an IFC/IDS reader, preserves the source GlobalIds,
//! verifies required information and per-rule IDS evaluation receipts, and emits
//! a frozen SiteWorkPackage.
//! Canonical package/entity-manifest hashing remains the caller's responsibility.

use std::collections::BTreeSet;

use crate::field_layout_acceptance::{
    CrewHours, DesignIntentEntity, DesignIntentManifest, FieldAcceptanceProfile,
    FieldLayoutEntityType, score_field_layout_run,
};
use crate::site_state_ledger::SiteWorkPackage;

pub const SUPPORTED_IFC_SCHEMA_RELEASE: &str = "IFC4X3_ADD2";
pub const SUPPORTED_IDS_VERSION: &str = "1.0";
const COORDINATE_ORTHOGONALITY_TOLERANCE: f64 = 1.0e-9;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IfcLengthUnit {
    Millimetre,
    Centimetre,
    Metre,
    Inch,
    Foot,
    Unsupported(String),
}

impl IfcLengthUnit {
    pub fn to_millimetres_ratio(&self) -> Option<(u64, u64)> {
        match self {
            Self::Millimetre => Some((1, 1)),
            Self::Centimetre => Some((10, 1)),
            Self::Metre => Some((1_000, 1)),
            Self::Inch => Some((254, 10)),
            Self::Foot => Some((3_048, 10)),
            Self::Unsupported(_) => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct IfcCoordinateContext {
    pub model_frame_id: String,
    pub project_crs_identifier: String,
    pub vertical_datum_identifier: String,
    pub length_unit: IfcLengthUnit,
    /// Proper rotation from model axes to the declared local site axes.
    pub model_to_site_rotation: [[f64; 3]; 3],
    /// Translation is expressed in millimetres in the target site frame.
    pub model_to_site_translation_mm: [f64; 3],
    /// Uniform map-conversion scale, separate from IFC length-unit conversion.
    pub map_conversion_scale: f64,
    pub frame_sha256: String,
    pub transform_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdsRequirementEvaluation {
    pub requirement_id: String,
    pub passed: bool,
    /// Digest of the canonical IDS engine result, bound to source model,
    /// GlobalId, IDS requirement and evaluated values.
    pub evaluation_receipt_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IfcSourceEntity {
    /// The 22-character compressed IFC GlobalId from the source model.
    pub global_id: String,
    pub entity_type: FieldLayoutEntityType,
    /// Digest of canonical geometry and placement extracted from the IFC object.
    pub geometry_sha256: String,
    /// IDS property requirements selected for this object by the project spec.
    pub required_ids_properties: Vec<String>,
    /// Property/attribute identifiers actually delivered by the IFC reader.
    pub delivered_ids_properties: Vec<String>,
    /// Digest of the IDS engine's complete applicability decision for this
    /// entity, including the matched rule set (even when that set is empty).
    pub ids_applicability_receipt_sha256: String,
    /// IDS specification rule identifiers applicable to this entity.
    pub applicable_ids_requirement_ids: Vec<String>,
    /// Full IDS engine evaluations. A property name being present does not
    /// prove that its type/value/cardinality/constraint facets passed.
    pub ids_requirement_evaluations: Vec<IdsRequirementEvaluation>,
    pub in_work_scope: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IfcIdsWorkPackageInput {
    /// Digest of the canonical compiled package, calculated by the upstream
    /// packager after sorting and serializing the full package contents.
    pub package_sha256: String,
    pub source_model_sha256: String,
    pub source_schema_release: String,
    pub ids_spec_sha256: String,
    pub ids_version: String,
    /// Exact IDS evaluation engine build; receipt digests are not interpretable
    /// or reproducible without identifying the evaluator implementation.
    pub ids_engine_build_sha256: String,
    pub design_revision_sha256: String,
    pub frame_id: String,
    pub frame_sha256: String,
    pub transform_sha256: String,
    pub coordinate_context: IfcCoordinateContext,
    /// Digest of the complete sorted source entity binding manifest, including
    /// GlobalIds, geometry digests, semantic types, IDS property bindings and
    /// per-entity requirement evaluation receipt digests.
    pub entity_manifest_sha256: String,
    pub acceptance_profile_sha256: String,
    pub acceptance_profile: FieldAcceptanceProfile,
    pub entities: Vec<IfcSourceEntity>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceEntityBinding {
    pub design_entity_id: String,
    pub ifc_global_id: String,
    pub geometry_sha256: String,
    pub required_ids_properties: Vec<String>,
    pub ids_applicability_receipt_sha256: String,
    pub applicable_ids_requirement_ids: Vec<String>,
    pub ids_requirement_evaluations: Vec<IdsRequirementEvaluation>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IfcIdsPreflightReport {
    pub work_package: SiteWorkPackage,
    pub source_entity_bindings: Vec<SourceEntityBinding>,
    pub source_entity_count: u64,
    pub in_scope_entity_count: u64,
    pub ifc_schema_release: String,
    pub ids_version: String,
    pub coordinate_context: IfcCoordinateContext,
    pub length_to_mm_numerator: u64,
    pub length_to_mm_denominator: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IfcIdsPreflightError {
    InvalidPackageDigest,
    InvalidSourceModelDigest,
    UnsupportedIfcSchema(String),
    InvalidIdsSpecDigest,
    UnsupportedIdsVersion(String),
    InvalidIdsEngineBuildDigest,
    InvalidDesignRevisionDigest,
    InvalidFrameIdentity,
    InvalidFrameDigest,
    InvalidTransformDigest,
    CoordinateFrameIdentityMismatch,
    CoordinateFrameDigestMismatch,
    MissingProjectCrsIdentifier,
    MissingVerticalDatumIdentifier,
    UnsupportedLengthUnit(String),
    InvalidMapConversionScale,
    NonFiniteCoordinateTranslation,
    InvalidRotationMatrix,
    InvalidEntityManifestDigest,
    InvalidAcceptanceProfileDigest,
    InvalidGlobalId(String),
    DuplicateGlobalId(String),
    InvalidGeometryDigest(String),
    InvalidIdsApplicabilityReceipt {
        global_id: String,
    },
    DuplicateRequiredIdsProperty {
        global_id: String,
        property: String,
    },
    DuplicateDeliveredIdsProperty {
        global_id: String,
        property: String,
    },
    EmptyIdsPropertyIdentifier {
        global_id: String,
    },
    MissingRequiredIdsProperty {
        global_id: String,
        property: String,
    },
    EmptyIdsRequirementId {
        global_id: String,
    },
    DuplicateApplicableIdsRequirement {
        global_id: String,
        requirement_id: String,
    },
    DuplicateIdsRequirementEvaluation {
        global_id: String,
        requirement_id: String,
    },
    MissingIdsRequirementEvaluation {
        global_id: String,
        requirement_id: String,
    },
    FailedIdsRequirement {
        global_id: String,
        requirement_id: String,
    },
    UnexpectedIdsRequirementEvaluation {
        global_id: String,
        requirement_id: String,
    },
    InvalidIdsRequirementEvaluationReceipt {
        global_id: String,
        requirement_id: String,
    },
    NoEntitiesInWorkScope,
    InvalidAcceptanceProfile,
}

/// Validate typed IFC/IDS records and compile their selected entities into a
/// stable robot-neutral work-package manifest.
///
/// The adapter must have already resolved IFC representation maps, units,
/// placements, linked-model transforms and IDS applicability. This function
/// validates the supplied result; it is not an IFC parser or geometric kernel.
pub fn preflight_ifc_ids_work_package(
    input: IfcIdsWorkPackageInput,
) -> Result<IfcIdsPreflightReport, Vec<IfcIdsPreflightError>> {
    let mut errors = Vec::new();

    if !is_sha256(&input.package_sha256) {
        errors.push(IfcIdsPreflightError::InvalidPackageDigest);
    }
    if !is_sha256(&input.source_model_sha256) {
        errors.push(IfcIdsPreflightError::InvalidSourceModelDigest);
    }
    if input.source_schema_release != SUPPORTED_IFC_SCHEMA_RELEASE {
        errors.push(IfcIdsPreflightError::UnsupportedIfcSchema(
            input.source_schema_release.clone(),
        ));
    }
    if !is_sha256(&input.ids_spec_sha256) {
        errors.push(IfcIdsPreflightError::InvalidIdsSpecDigest);
    }
    if input.ids_version != SUPPORTED_IDS_VERSION {
        errors.push(IfcIdsPreflightError::UnsupportedIdsVersion(
            input.ids_version.clone(),
        ));
    }
    if !is_sha256(&input.ids_engine_build_sha256) {
        errors.push(IfcIdsPreflightError::InvalidIdsEngineBuildDigest);
    }
    if !is_sha256(&input.design_revision_sha256) {
        errors.push(IfcIdsPreflightError::InvalidDesignRevisionDigest);
    }
    if input.frame_id.trim().is_empty() {
        errors.push(IfcIdsPreflightError::InvalidFrameIdentity);
    }
    if !is_sha256(&input.frame_sha256) {
        errors.push(IfcIdsPreflightError::InvalidFrameDigest);
    }
    if !is_sha256(&input.transform_sha256) {
        errors.push(IfcIdsPreflightError::InvalidTransformDigest);
    }
    if input.coordinate_context.model_frame_id != input.frame_id {
        errors.push(IfcIdsPreflightError::CoordinateFrameIdentityMismatch);
    }
    if input.coordinate_context.frame_sha256 != input.frame_sha256
        || input.coordinate_context.transform_sha256 != input.transform_sha256
        || !is_sha256(&input.coordinate_context.frame_sha256)
        || !is_sha256(&input.coordinate_context.transform_sha256)
    {
        errors.push(IfcIdsPreflightError::CoordinateFrameDigestMismatch);
    }
    if input
        .coordinate_context
        .project_crs_identifier
        .trim()
        .is_empty()
    {
        errors.push(IfcIdsPreflightError::MissingProjectCrsIdentifier);
    }
    if input
        .coordinate_context
        .vertical_datum_identifier
        .trim()
        .is_empty()
    {
        errors.push(IfcIdsPreflightError::MissingVerticalDatumIdentifier);
    }
    let unit_ratio = input.coordinate_context.length_unit.to_millimetres_ratio();
    if unit_ratio.is_none() {
        let label = match &input.coordinate_context.length_unit {
            IfcLengthUnit::Unsupported(value) => value.clone(),
            _ => "unsupported".into(),
        };
        errors.push(IfcIdsPreflightError::UnsupportedLengthUnit(label));
    }
    if !input.coordinate_context.map_conversion_scale.is_finite()
        || input.coordinate_context.map_conversion_scale <= 0.0
    {
        errors.push(IfcIdsPreflightError::InvalidMapConversionScale);
    }
    if !input
        .coordinate_context
        .model_to_site_translation_mm
        .into_iter()
        .all(f64::is_finite)
    {
        errors.push(IfcIdsPreflightError::NonFiniteCoordinateTranslation);
    }
    if !is_proper_rotation(
        input.coordinate_context.model_to_site_rotation,
        COORDINATE_ORTHOGONALITY_TOLERANCE,
    ) {
        errors.push(IfcIdsPreflightError::InvalidRotationMatrix);
    }
    if !is_sha256(&input.entity_manifest_sha256) {
        errors.push(IfcIdsPreflightError::InvalidEntityManifestDigest);
    }
    if !is_sha256(&input.acceptance_profile_sha256) {
        errors.push(IfcIdsPreflightError::InvalidAcceptanceProfileDigest);
    }

    let mut global_ids = BTreeSet::new();
    let mut source_entity_bindings = Vec::new();
    let mut design_entities = Vec::new();

    for source_entity in &input.entities {
        if !is_ifc_global_id(&source_entity.global_id) {
            errors.push(IfcIdsPreflightError::InvalidGlobalId(
                source_entity.global_id.clone(),
            ));
            continue;
        }
        if !global_ids.insert(source_entity.global_id.as_str()) {
            errors.push(IfcIdsPreflightError::DuplicateGlobalId(
                source_entity.global_id.clone(),
            ));
            continue;
        }

        if !source_entity.in_work_scope {
            continue;
        }
        if !is_sha256(&source_entity.geometry_sha256) {
            errors.push(IfcIdsPreflightError::InvalidGeometryDigest(
                source_entity.global_id.clone(),
            ));
        }
        if !is_sha256(&source_entity.ids_applicability_receipt_sha256) {
            errors.push(IfcIdsPreflightError::InvalidIdsApplicabilityReceipt {
                global_id: source_entity.global_id.clone(),
            });
        }

        let mut required = BTreeSet::new();
        for property in &source_entity.required_ids_properties {
            if property.trim().is_empty() {
                errors.push(IfcIdsPreflightError::EmptyIdsPropertyIdentifier {
                    global_id: source_entity.global_id.clone(),
                });
            } else if !required.insert(property.as_str()) {
                errors.push(IfcIdsPreflightError::DuplicateRequiredIdsProperty {
                    global_id: source_entity.global_id.clone(),
                    property: property.clone(),
                });
            }
        }

        let mut delivered = BTreeSet::new();
        for property in &source_entity.delivered_ids_properties {
            if property.trim().is_empty() {
                errors.push(IfcIdsPreflightError::EmptyIdsPropertyIdentifier {
                    global_id: source_entity.global_id.clone(),
                });
            } else if !delivered.insert(property.as_str()) {
                errors.push(IfcIdsPreflightError::DuplicateDeliveredIdsProperty {
                    global_id: source_entity.global_id.clone(),
                    property: property.clone(),
                });
            }
        }

        for required_property in required.iter() {
            if !delivered.contains(required_property) {
                errors.push(IfcIdsPreflightError::MissingRequiredIdsProperty {
                    global_id: source_entity.global_id.clone(),
                    property: (*required_property).to_owned(),
                });
            }
        }

        let mut applicable_requirements = BTreeSet::new();
        for requirement_id in &source_entity.applicable_ids_requirement_ids {
            if requirement_id.trim().is_empty() {
                errors.push(IfcIdsPreflightError::EmptyIdsRequirementId {
                    global_id: source_entity.global_id.clone(),
                });
            } else if !applicable_requirements.insert(requirement_id.as_str()) {
                errors.push(IfcIdsPreflightError::DuplicateApplicableIdsRequirement {
                    global_id: source_entity.global_id.clone(),
                    requirement_id: requirement_id.clone(),
                });
            }
        }

        let mut evaluated_requirements = BTreeSet::new();
        for evaluation in &source_entity.ids_requirement_evaluations {
            if !evaluated_requirements.insert(evaluation.requirement_id.as_str()) {
                errors.push(IfcIdsPreflightError::DuplicateIdsRequirementEvaluation {
                    global_id: source_entity.global_id.clone(),
                    requirement_id: evaluation.requirement_id.clone(),
                });
                continue;
            }
            if !applicable_requirements.contains(evaluation.requirement_id.as_str()) {
                errors.push(IfcIdsPreflightError::UnexpectedIdsRequirementEvaluation {
                    global_id: source_entity.global_id.clone(),
                    requirement_id: evaluation.requirement_id.clone(),
                });
            }
            if !is_sha256(&evaluation.evaluation_receipt_sha256) {
                errors.push(
                    IfcIdsPreflightError::InvalidIdsRequirementEvaluationReceipt {
                        global_id: source_entity.global_id.clone(),
                        requirement_id: evaluation.requirement_id.clone(),
                    },
                );
            }
            if !evaluation.passed {
                errors.push(IfcIdsPreflightError::FailedIdsRequirement {
                    global_id: source_entity.global_id.clone(),
                    requirement_id: evaluation.requirement_id.clone(),
                });
            }
        }

        for requirement_id in applicable_requirements {
            if !evaluated_requirements.contains(requirement_id) {
                errors.push(IfcIdsPreflightError::MissingIdsRequirementEvaluation {
                    global_id: source_entity.global_id.clone(),
                    requirement_id: requirement_id.to_owned(),
                });
            }
        }

        let design_entity_id = format!("ifc4x3:{}", source_entity.global_id);
        design_entities.push(DesignIntentEntity {
            design_entity_id: design_entity_id.clone(),
            entity_type: source_entity.entity_type,
        });
        source_entity_bindings.push(SourceEntityBinding {
            design_entity_id,
            ifc_global_id: source_entity.global_id.clone(),
            geometry_sha256: source_entity.geometry_sha256.clone(),
            required_ids_properties: source_entity.required_ids_properties.clone(),
            ids_applicability_receipt_sha256: source_entity
                .ids_applicability_receipt_sha256
                .clone(),
            applicable_ids_requirement_ids: source_entity.applicable_ids_requirement_ids.clone(),
            ids_requirement_evaluations: source_entity.ids_requirement_evaluations.clone(),
        });
    }

    if design_entities.is_empty() {
        errors.push(IfcIdsPreflightError::NoEntitiesInWorkScope);
    }

    if errors.is_empty() {
        let manifest = DesignIntentManifest {
            package_sha256: input.package_sha256.clone(),
            design_revision_sha256: input.design_revision_sha256.clone(),
            frame_id: input.frame_id.clone(),
            entity_manifest_sha256: input.entity_manifest_sha256.clone(),
            entities: design_entities.clone(),
        };
        if score_field_layout_run(
            &manifest,
            &[],
            &[],
            input.acceptance_profile,
            CrewHours::default(),
        )
        .is_err()
        {
            errors.push(IfcIdsPreflightError::InvalidAcceptanceProfile);
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    let in_scope_entity_count = source_entity_bindings.len() as u64;
    Ok(IfcIdsPreflightReport {
        work_package: SiteWorkPackage {
            package_sha256: input.package_sha256,
            source_model_sha256: input.source_model_sha256,
            source_schema_release: input.source_schema_release.clone(),
            ids_spec_sha256: input.ids_spec_sha256,
            ids_version: input.ids_version.clone(),
            ids_engine_build_sha256: input.ids_engine_build_sha256,
            design_revision_sha256: input.design_revision_sha256,
            frame_id: input.frame_id,
            frame_sha256: input.frame_sha256,
            transform_sha256: input.transform_sha256,
            entity_manifest_sha256: input.entity_manifest_sha256,
            acceptance_profile_sha256: input.acceptance_profile_sha256,
            entities: design_entities,
            acceptance_profile: input.acceptance_profile,
        },
        source_entity_bindings,
        source_entity_count: input.entities.len() as u64,
        in_scope_entity_count,
        ifc_schema_release: input.source_schema_release,
        ids_version: input.ids_version,
        coordinate_context: input.coordinate_context,
        length_to_mm_numerator: unit_ratio.map_or(0, |ratio| ratio.0),
        length_to_mm_denominator: unit_ratio.map_or(0, |ratio| ratio.1),
    })
}

fn is_proper_rotation(matrix: [[f64; 3]; 3], tolerance: f64) -> bool {
    if !matrix.into_iter().flatten().all(f64::is_finite) {
        return false;
    }
    for row in 0..3 {
        for other_row in 0..3 {
            let dot = (0..3)
                .map(|column| matrix[row][column] * matrix[other_row][column])
                .sum::<f64>();
            let expected = if row == other_row { 1.0 } else { 0.0 };
            if (dot - expected).abs() > tolerance {
                return false;
            }
        }
    }
    let determinant = matrix[0][0] * (matrix[1][1] * matrix[2][2] - matrix[1][2] * matrix[2][1])
        - matrix[0][1] * (matrix[1][0] * matrix[2][2] - matrix[1][2] * matrix[2][0])
        + matrix[0][2] * (matrix[1][0] * matrix[2][1] - matrix[1][1] * matrix[2][0]);
    (determinant - 1.0).abs() <= tolerance
}

fn is_ifc_global_id(value: &str) -> bool {
    value.len() == 22
        && value
            .as_bytes()
            .first()
            .is_some_and(|byte| (b'0'..=b'3').contains(byte))
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == 0x24)
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: char) -> String {
        std::iter::repeat_n(byte, 64).collect()
    }

    fn global_id(byte: char) -> String {
        std::iter::repeat_n(byte, 22).collect()
    }

    fn valid_input() -> IfcIdsWorkPackageInput {
        IfcIdsWorkPackageInput {
            package_sha256: digest('1'),
            source_model_sha256: digest('2'),
            source_schema_release: SUPPORTED_IFC_SCHEMA_RELEASE.into(),
            ids_spec_sha256: digest('3'),
            ids_version: SUPPORTED_IDS_VERSION.into(),
            ids_engine_build_sha256: digest('f'),
            design_revision_sha256: digest('4'),
            frame_id: "site-level-02".into(),
            frame_sha256: digest('5'),
            transform_sha256: digest('6'),
            coordinate_context: IfcCoordinateContext {
                model_frame_id: "site-level-02".into(),
                project_crs_identifier: "EPSG:32735".into(),
                vertical_datum_identifier: "local-project-benchmark-datum".into(),
                length_unit: IfcLengthUnit::Millimetre,
                model_to_site_rotation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                model_to_site_translation_mm: [10_000.0, 20_000.0, 0.0],
                map_conversion_scale: 1.0,
                frame_sha256: digest('5'),
                transform_sha256: digest('6'),
            },
            entity_manifest_sha256: digest('7'),
            acceptance_profile_sha256: digest('8'),
            acceptance_profile: FieldAcceptanceProfile {
                max_abs_xy_residual_mm: 1.0,
                max_reference_u95_xy_mm: 0.25,
                max_abs_z_residual_mm: None,
                max_reference_u95_z_mm: None,
                max_abs_endpoint_residual_mm: Some(1.0),
            },
            entities: vec![
                IfcSourceEntity {
                    global_id: global_id('0'),
                    entity_type: FieldLayoutEntityType::Line,
                    geometry_sha256: digest('9'),
                    required_ids_properties: vec![
                        "Pset_WallCommon.IsExternal".into(),
                        "Pset_Layout.MarkingClass".into(),
                    ],
                    delivered_ids_properties: vec![
                        "Pset_WallCommon.IsExternal".into(),
                        "Pset_Layout.MarkingClass".into(),
                    ],
                    ids_applicability_receipt_sha256: digest('e'),
                    applicable_ids_requirement_ids: vec![
                        "layout-marking-class".into(),
                        "wall-external-classification".into(),
                    ],
                    ids_requirement_evaluations: vec![
                        IdsRequirementEvaluation {
                            requirement_id: "layout-marking-class".into(),
                            passed: true,
                            evaluation_receipt_sha256: digest('c'),
                        },
                        IdsRequirementEvaluation {
                            requirement_id: "wall-external-classification".into(),
                            passed: true,
                            evaluation_receipt_sha256: digest('d'),
                        },
                    ],
                    in_work_scope: true,
                },
                IfcSourceEntity {
                    global_id: global_id('1'),
                    entity_type: FieldLayoutEntityType::Point,
                    geometry_sha256: digest('a'),
                    required_ids_properties: vec!["Pset_Layout.MarkingClass".into()],
                    delivered_ids_properties: vec!["Pset_Layout.MarkingClass".into()],
                    ids_applicability_receipt_sha256: digest('f'),
                    applicable_ids_requirement_ids: vec!["layout-marking-class".into()],
                    ids_requirement_evaluations: vec![IdsRequirementEvaluation {
                        requirement_id: "layout-marking-class".into(),
                        passed: true,
                        evaluation_receipt_sha256: digest('e'),
                    }],
                    in_work_scope: true,
                },
            ],
        }
    }

    #[test]
    fn valid_ifc_ids_input_compiles_stable_global_id_bindings() {
        let report = preflight_ifc_ids_work_package(valid_input()).unwrap();
        assert_eq!(report.source_entity_count, 2);
        assert_eq!(report.in_scope_entity_count, 2);
        assert_eq!(report.work_package.entities.len(), 2);
        assert_eq!(
            report.work_package.entities[0].design_entity_id,
            format!("ifc4x3:{}", global_id('0'))
        );
        assert_eq!(
            report.work_package.source_schema_release,
            SUPPORTED_IFC_SCHEMA_RELEASE
        );
        assert_eq!(report.work_package.ids_version, SUPPORTED_IDS_VERSION);
        assert_eq!(report.length_to_mm_numerator, 1);
        assert_eq!(report.length_to_mm_denominator, 1);
        assert_eq!(
            report.coordinate_context.project_crs_identifier,
            "EPSG:32735"
        );
        assert_eq!(
            report.coordinate_context.vertical_datum_identifier,
            "local-project-benchmark-datum"
        );
    }

    #[test]
    fn missing_ids_information_blocks_package_compilation() {
        let mut input = valid_input();
        input.entities[0].delivered_ids_properties = vec!["Pset_WallCommon.IsExternal".into()];
        assert!(
            preflight_ifc_ids_work_package(input)
                .unwrap_err()
                .iter()
                .any(|error| matches!(
                    error,
                    IfcIdsPreflightError::MissingRequiredIdsProperty { global_id, property }
                        if global_id == &global_id('0') && property == "Pset_Layout.MarkingClass"
                ))
        );
    }

    #[test]
    fn duplicate_or_malformed_global_ids_are_not_silently_rewritten() {
        let mut duplicate = valid_input();
        duplicate.entities[1].global_id = global_id('0');
        assert!(
            preflight_ifc_ids_work_package(duplicate)
                .unwrap_err()
                .contains(&IfcIdsPreflightError::DuplicateGlobalId(global_id('0')))
        );

        let mut malformed = valid_input();
        malformed.entities[0].global_id = "not-an-ifc-global-id".into();
        assert!(
            preflight_ifc_ids_work_package(malformed)
                .unwrap_err()
                .contains(&IfcIdsPreflightError::InvalidGlobalId(
                    "not-an-ifc-global-id".into()
                ))
        );

        let mut invalid_prefix = valid_input();
        invalid_prefix.entities[0].global_id = global_id('A');
        assert!(
            preflight_ifc_ids_work_package(invalid_prefix)
                .unwrap_err()
                .contains(&IfcIdsPreflightError::InvalidGlobalId(global_id('A')))
        );
    }

    #[test]
    fn unsupported_schema_or_ids_version_blocks_dispatchable_package() {
        let mut input = valid_input();
        input.source_schema_release = "IFC4".into();
        input.ids_version = "0.9".into();
        let errors = preflight_ifc_ids_work_package(input).unwrap_err();
        assert!(errors.contains(&IfcIdsPreflightError::UnsupportedIfcSchema("IFC4".into())));
        assert!(errors.contains(&IfcIdsPreflightError::UnsupportedIdsVersion("0.9".into())));
    }

    #[test]
    fn excluded_entities_are_counted_but_not_compiled_into_layout_scope() {
        let mut input = valid_input();
        input.entities.push(IfcSourceEntity {
            global_id: global_id('2'),
            entity_type: FieldLayoutEntityType::Other,
            geometry_sha256: digest('b'),
            required_ids_properties: Vec::new(),
            delivered_ids_properties: Vec::new(),
            ids_applicability_receipt_sha256: digest('1'),
            applicable_ids_requirement_ids: Vec::new(),
            ids_requirement_evaluations: Vec::new(),
            in_work_scope: false,
        });
        let report = preflight_ifc_ids_work_package(input).unwrap();
        assert_eq!(report.source_entity_count, 3);
        assert_eq!(report.in_scope_entity_count, 2);
        assert_eq!(report.work_package.entities.len(), 2);
    }

    #[test]
    fn missing_or_failed_ids_rule_evaluation_holds_package_compilation() {
        let mut missing = valid_input();
        missing.entities[0].ids_requirement_evaluations.pop();
        assert!(
            preflight_ifc_ids_work_package(missing)
                .unwrap_err()
                .contains(&IfcIdsPreflightError::MissingIdsRequirementEvaluation {
                    global_id: global_id('0'),
                    requirement_id: "wall-external-classification".into(),
                })
        );

        let mut failed = valid_input();
        failed.entities[0].ids_requirement_evaluations[0].passed = false;
        assert!(
            preflight_ifc_ids_work_package(failed)
                .unwrap_err()
                .contains(&IfcIdsPreflightError::FailedIdsRequirement {
                    global_id: global_id('0'),
                    requirement_id: "layout-marking-class".into(),
                })
        );
    }

    #[test]
    fn duplicate_and_unexpected_ids_evaluation_receipts_are_rejected() {
        let mut duplicate = valid_input();
        let duplicate_evaluation = duplicate.entities[0].ids_requirement_evaluations[0].clone();
        duplicate.entities[0]
            .ids_requirement_evaluations
            .push(duplicate_evaluation);
        assert!(
            preflight_ifc_ids_work_package(duplicate)
                .unwrap_err()
                .contains(&IfcIdsPreflightError::DuplicateIdsRequirementEvaluation {
                    global_id: global_id('0'),
                    requirement_id: "layout-marking-class".into(),
                })
        );

        let mut unexpected = valid_input();
        unexpected.entities[0].ids_requirement_evaluations[0].requirement_id =
            "not-in-the-applicability-set".into();
        assert!(
            preflight_ifc_ids_work_package(unexpected)
                .unwrap_err()
                .contains(&IfcIdsPreflightError::UnexpectedIdsRequirementEvaluation {
                    global_id: global_id('0'),
                    requirement_id: "not-in-the-applicability-set".into(),
                })
        );
    }

    #[test]
    fn malformed_ids_evaluation_receipt_blocks_package_compilation() {
        let mut invalid = valid_input();
        invalid.entities[0].ids_requirement_evaluations[0].evaluation_receipt_sha256 =
            "unverified".into();
        assert!(
            preflight_ifc_ids_work_package(invalid)
                .unwrap_err()
                .contains(
                    &IfcIdsPreflightError::InvalidIdsRequirementEvaluationReceipt {
                        global_id: global_id('0'),
                        requirement_id: "layout-marking-class".into(),
                    }
                )
        );
    }

    #[test]
    fn supported_length_units_have_exact_millimetre_ratios() {
        assert_eq!(
            IfcLengthUnit::Millimetre.to_millimetres_ratio(),
            Some((1, 1))
        );
        assert_eq!(
            IfcLengthUnit::Centimetre.to_millimetres_ratio(),
            Some((10, 1))
        );
        assert_eq!(
            IfcLengthUnit::Metre.to_millimetres_ratio(),
            Some((1_000, 1))
        );
        assert_eq!(IfcLengthUnit::Inch.to_millimetres_ratio(), Some((254, 10)));
        assert_eq!(
            IfcLengthUnit::Foot.to_millimetres_ratio(),
            Some((3_048, 10))
        );
        assert_eq!(
            IfcLengthUnit::Unsupported("yard".into()).to_millimetres_ratio(),
            None
        );
    }

    #[test]
    fn coordinate_frame_identity_rotation_and_map_scale_fail_closed() {
        let mut bad_frame = valid_input();
        bad_frame.coordinate_context.model_frame_id = "different-frame".into();
        assert!(
            preflight_ifc_ids_work_package(bad_frame)
                .unwrap_err()
                .contains(&IfcIdsPreflightError::CoordinateFrameIdentityMismatch)
        );

        let mut reflection = valid_input();
        reflection.coordinate_context.model_to_site_rotation[0][0] = -1.0;
        assert!(
            preflight_ifc_ids_work_package(reflection)
                .unwrap_err()
                .contains(&IfcIdsPreflightError::InvalidRotationMatrix)
        );

        let mut scaled = valid_input();
        scaled.coordinate_context.map_conversion_scale = 0.0;
        assert!(
            preflight_ifc_ids_work_package(scaled)
                .unwrap_err()
                .contains(&IfcIdsPreflightError::InvalidMapConversionScale)
        );
    }

    #[test]
    fn improper_scaled_and_nonfinite_coordinate_matrices_are_rejected() {
        let mut skew = valid_input();
        skew.coordinate_context.model_to_site_rotation[0][1] = 0.1;
        assert!(
            preflight_ifc_ids_work_package(skew)
                .unwrap_err()
                .contains(&IfcIdsPreflightError::InvalidRotationMatrix)
        );

        let mut nonfinite = valid_input();
        nonfinite.coordinate_context.model_to_site_translation_mm[1] = f64::INFINITY;
        assert!(
            preflight_ifc_ids_work_package(nonfinite)
                .unwrap_err()
                .contains(&IfcIdsPreflightError::NonFiniteCoordinateTranslation)
        );
    }

    #[test]
    fn frame_transform_hashes_must_match_the_site_package_identity() {
        let mut mismatch = valid_input();
        mismatch.coordinate_context.transform_sha256 = digest('0');
        assert!(
            preflight_ifc_ids_work_package(mismatch)
                .unwrap_err()
                .contains(&IfcIdsPreflightError::CoordinateFrameDigestMismatch)
        );
    }

    #[test]
    fn foot_units_are_reported_as_exact_rational_millimetre_conversion() {
        let mut foot = valid_input();
        foot.coordinate_context.length_unit = IfcLengthUnit::Foot;
        let report = preflight_ifc_ids_work_package(foot).unwrap();
        assert_eq!(report.length_to_mm_numerator, 3_048);
        assert_eq!(report.length_to_mm_denominator, 10);
    }

    #[test]
    fn missing_crs_datum_and_unsupported_units_block_preflight() {
        let mut missing_crs = valid_input();
        missing_crs
            .coordinate_context
            .project_crs_identifier
            .clear();
        assert!(
            preflight_ifc_ids_work_package(missing_crs)
                .unwrap_err()
                .contains(&IfcIdsPreflightError::MissingProjectCrsIdentifier)
        );

        let mut missing_datum = valid_input();
        missing_datum
            .coordinate_context
            .vertical_datum_identifier
            .clear();
        assert!(
            preflight_ifc_ids_work_package(missing_datum)
                .unwrap_err()
                .contains(&IfcIdsPreflightError::MissingVerticalDatumIdentifier)
        );

        let mut unsupported = valid_input();
        unsupported.coordinate_context.length_unit = IfcLengthUnit::Unsupported("yard".into());
        assert!(
            preflight_ifc_ids_work_package(unsupported)
                .unwrap_err()
                .contains(&IfcIdsPreflightError::UnsupportedLengthUnit("yard".into()))
        );
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: char) -> String {
        std::iter::repeat_n(byte, 64).collect()
    }

    fn global_id(byte: char) -> String {
        std::iter::repeat_n(byte, 22).collect()
    }

    fn valid_input() -> IfcIdsWorkPackageInput {
        IfcIdsWorkPackageInput {
            package_sha256: digest('1'),
            source_model_sha256: digest('2'),
            source_schema_release: SUPPORTED_IFC_SCHEMA_RELEASE.into(),
            ids_spec_sha256: digest('3'),
            ids_version: SUPPORTED_IDS_VERSION.into(),
            design_revision_sha256: digest('4'),
            frame_id: "site-level-02".into(),
            frame_sha256: digest('5'),
            transform_sha256: digest('6'),
            coordinate_context: IfcCoordinateContext {
                model_frame_id: "site-level-02".into(),
                project_crs_identifier: "EPSG:32735".into(),
                vertical_datum_identifier: "local-project-benchmark-datum".into(),
                length_unit: IfcLengthUnit::Millimetre,
                model_to_site_rotation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                model_to_site_translation_mm: [10_000.0, 20_000.0, 0.0],
                map_conversion_scale: 1.0,
                frame_sha256: digest('5'),
                transform_sha256: digest('6'),
            },
            entity_manifest_sha256: digest('7'),
            acceptance_profile_sha256: digest('8'),
            acceptance_profile: FieldAcceptanceProfile {
                max_abs_xy_residual_mm: 1.0,
                max_reference_u95_xy_mm: 0.25,
                max_abs_z_residual_mm: None,
                max_reference_u95_z_mm: None,
                max_abs_endpoint_residual_mm: Some(1.0),
            },
            entities: vec![
                IfcSourceEntity {
                    global_id: global_id('0'),
                    entity_type: FieldLayoutEntityType::Line,
                    geometry_sha256: digest('9'),
                    required_ids_properties: vec![
                        "Pset_WallCommon.IsExternal".into(),
                        "Pset_Layout.MarkingClass".into(),
                    ],
                    delivered_ids_properties: vec![
                        "Pset_WallCommon.IsExternal".into(),
                        "Pset_Layout.MarkingClass".into(),
                    ],
                    applicable_ids_requirement_ids: vec![
                        "layout-marking-class".into(),
                        "wall-external-classification".into(),
                    ],
                    ids_requirement_evaluations: vec![
                        IdsRequirementEvaluation {
                            requirement_id: "layout-marking-class".into(),
                            passed: true,
                            evaluation_receipt_sha256: digest('c'),
                        },
                        IdsRequirementEvaluation {
                            requirement_id: "wall-external-classification".into(),
                            passed: true,
                            evaluation_receipt_sha256: digest('d'),
                        },
                    ],
                    in_work_scope: true,
                },
                IfcSourceEntity {
                    global_id: global_id('1'),
                    entity_type: FieldLayoutEntityType::Point,
                    geometry_sha256: digest('a'),
                    required_ids_properties: vec!["Pset_Layout.MarkingClass".into()],
                    delivered_ids_properties: vec!["Pset_Layout.MarkingClass".into()],
                    applicable_ids_requirement_ids: vec!["layout-marking-class".into()],
                    ids_requirement_evaluations: vec![IdsRequirementEvaluation {
                        requirement_id: "layout-marking-class".into(),
                        passed: true,
                        evaluation_receipt_sha256: digest('e'),
                    }],
                    in_work_scope: true,
                },
            ],
        }
    }

    #[test]
    fn valid_ifc_ids_input_compiles_stable_global_id_bindings() {
        let report = preflight_ifc_ids_work_package(valid_input()).unwrap();
        assert_eq!(report.source_entity_count, 2);
        assert_eq!(report.in_scope_entity_count, 2);
        assert_eq!(report.work_package.entities.len(), 2);
        assert_eq!(
            report.work_package.entities[0].design_entity_id,
            format!("ifc4x3:{}", global_id('0'))
        );
        assert_eq!(
            report.work_package.source_schema_release,
            SUPPORTED_IFC_SCHEMA_RELEASE
        );
        assert_eq!(report.work_package.ids_version, SUPPORTED_IDS_VERSION);
        assert_eq!(report.length_to_mm_numerator, 1);
        assert_eq!(report.length_to_mm_denominator, 1);
        assert_eq!(
            report.coordinate_context.project_crs_identifier,
            "EPSG:32735"
        );
        assert_eq!(
            report.coordinate_context.vertical_datum_identifier,
            "local-project-benchmark-datum"
        );
    }

    #[test]
    fn missing_ids_information_blocks_package_compilation() {
        let mut input = valid_input();
        input.entities[0].delivered_ids_properties = vec!["Pset_WallCommon.IsExternal".into()];
        assert!(
            preflight_ifc_ids_work_package(input)
                .unwrap_err()
                .iter()
                .any(|error| matches!(
                    error,
                    IfcIdsPreflightError::MissingRequiredIdsProperty { global_id, property }
                        if global_id == &global_id('0') && property == "Pset_Layout.MarkingClass"
                ))
        );
    }

    #[test]
    fn duplicate_or_malformed_global_ids_are_not_silently_rewritten() {
        let mut duplicate = valid_input();
        duplicate.entities[1].global_id = global_id('0');
        assert!(
            preflight_ifc_ids_work_package(duplicate)
                .unwrap_err()
                .contains(&IfcIdsPreflightError::DuplicateGlobalId(global_id('0')))
        );

        let mut malformed = valid_input();
        malformed.entities[0].global_id = "not-an-ifc-global-id".into();
        assert!(
            preflight_ifc_ids_work_package(malformed)
                .unwrap_err()
                .contains(&IfcIdsPreflightError::InvalidGlobalId(
                    "not-an-ifc-global-id".into()
                ))
        );
    }

    #[test]
    fn unsupported_schema_or_ids_version_blocks_dispatchable_package() {
        let mut input = valid_input();
        input.source_schema_release = "IFC4".into();
        input.ids_version = "0.9".into();
        let errors = preflight_ifc_ids_work_package(input).unwrap_err();
        assert!(errors.contains(&IfcIdsPreflightError::UnsupportedIfcSchema("IFC4".into())));
        assert!(errors.contains(&IfcIdsPreflightError::UnsupportedIdsVersion("0.9".into())));
    }

    #[test]
    fn excluded_entities_are_counted_but_not_compiled_into_layout_scope() {
        let mut input = valid_input();
        input.entities.push(IfcSourceEntity {
            global_id: global_id('2'),
            entity_type: FieldLayoutEntityType::Other,
            geometry_sha256: digest('b'),
            required_ids_properties: Vec::new(),
            delivered_ids_properties: Vec::new(),
            applicable_ids_requirement_ids: Vec::new(),
            ids_requirement_evaluations: Vec::new(),
            in_work_scope: false,
        });
        let report = preflight_ifc_ids_work_package(input).unwrap();
        assert_eq!(report.source_entity_count, 3);
        assert_eq!(report.in_scope_entity_count, 2);
        assert_eq!(report.work_package.entities.len(), 2);
    }

    #[test]
    fn missing_or_failed_ids_rule_evaluation_holds_package_compilation() {
        let mut missing = valid_input();
        missing.entities[0].ids_requirement_evaluations.pop();
        assert!(
            preflight_ifc_ids_work_package(missing)
                .unwrap_err()
                .contains(&IfcIdsPreflightError::MissingIdsRequirementEvaluation {
                    global_id: global_id('0'),
                    requirement_id: "wall-external-classification".into(),
                })
        );

        let mut failed = valid_input();
        failed.entities[0].ids_requirement_evaluations[0].passed = false;
        assert!(
            preflight_ifc_ids_work_package(failed)
                .unwrap_err()
                .contains(&IfcIdsPreflightError::FailedIdsRequirement {
                    global_id: global_id('0'),
                    requirement_id: "layout-marking-class".into(),
                })
        );
    }

    #[test]
    fn duplicate_and_unexpected_ids_evaluation_receipts_are_rejected() {
        let mut duplicate = valid_input();
        let duplicate_evaluation = duplicate.entities[0].ids_requirement_evaluations[0].clone();
        duplicate.entities[0]
            .ids_requirement_evaluations
            .push(duplicate_evaluation);
        assert!(
            preflight_ifc_ids_work_package(duplicate)
                .unwrap_err()
                .contains(&IfcIdsPreflightError::DuplicateIdsRequirementEvaluation {
                    global_id: global_id('0'),
                    requirement_id: "layout-marking-class".into(),
                })
        );

        let mut unexpected = valid_input();
        unexpected.entities[0].ids_requirement_evaluations[0].requirement_id =
            "not-in-the-applicability-set".into();
        assert!(
            preflight_ifc_ids_work_package(unexpected)
                .unwrap_err()
                .contains(&IfcIdsPreflightError::UnexpectedIdsRequirementEvaluation {
                    global_id: global_id('0'),
                    requirement_id: "not-in-the-applicability-set".into(),
                })
        );
    }

    #[test]
    fn malformed_ids_evaluation_receipt_blocks_package_compilation() {
        let mut invalid = valid_input();
        invalid.entities[0].ids_requirement_evaluations[0].evaluation_receipt_sha256 =
            "unverified".into();
        assert!(
            preflight_ifc_ids_work_package(invalid)
                .unwrap_err()
                .contains(
                    &IfcIdsPreflightError::InvalidIdsRequirementEvaluationReceipt {
                        global_id: global_id('0'),
                        requirement_id: "layout-marking-class".into(),
                    }
                )
        );
    }

    #[test]
    fn supported_length_units_have_exact_millimetre_ratios() {
        assert_eq!(
            IfcLengthUnit::Millimetre.to_millimetres_ratio(),
            Some((1, 1))
        );
        assert_eq!(
            IfcLengthUnit::Centimetre.to_millimetres_ratio(),
            Some((10, 1))
        );
        assert_eq!(
            IfcLengthUnit::Metre.to_millimetres_ratio(),
            Some((1_000, 1))
        );
        assert_eq!(IfcLengthUnit::Inch.to_millimetres_ratio(), Some((254, 10)));
        assert_eq!(
            IfcLengthUnit::Foot.to_millimetres_ratio(),
            Some((3_048, 10))
        );
        assert_eq!(
            IfcLengthUnit::Unsupported("yard".into()).to_millimetres_ratio(),
            None
        );
    }

    #[test]
    fn coordinate_frame_identity_rotation_and_map_scale_fail_closed() {
        let mut bad_frame = valid_input();
        bad_frame.coordinate_context.model_frame_id = "different-frame".into();
        assert!(
            preflight_ifc_ids_work_package(bad_frame)
                .unwrap_err()
                .contains(&IfcIdsPreflightError::CoordinateFrameIdentityMismatch)
        );

        let mut reflection = valid_input();
        reflection.coordinate_context.model_to_site_rotation[0][0] = -1.0;
        assert!(
            preflight_ifc_ids_work_package(reflection)
                .unwrap_err()
                .contains(&IfcIdsPreflightError::InvalidRotationMatrix)
        );

        let mut scaled = valid_input();
        scaled.coordinate_context.map_conversion_scale = 0.0;
        assert!(
            preflight_ifc_ids_work_package(scaled)
                .unwrap_err()
                .contains(&IfcIdsPreflightError::InvalidMapConversionScale)
        );
    }

    #[test]
    fn improper_scaled_and_nonfinite_coordinate_matrices_are_rejected() {
        let mut skew = valid_input();
        skew.coordinate_context.model_to_site_rotation[0][1] = 0.1;
        assert!(
            preflight_ifc_ids_work_package(skew)
                .unwrap_err()
                .contains(&IfcIdsPreflightError::InvalidRotationMatrix)
        );

        let mut nonfinite = valid_input();
        nonfinite.coordinate_context.model_to_site_translation_mm[1] = f64::INFINITY;
        assert!(
            preflight_ifc_ids_work_package(nonfinite)
                .unwrap_err()
                .contains(&IfcIdsPreflightError::NonFiniteCoordinateTranslation)
        );
    }

    #[test]
    fn frame_transform_hashes_must_match_the_site_package_identity() {
        let mut mismatch = valid_input();
        mismatch.coordinate_context.transform_sha256 = digest('0');
        assert!(
            preflight_ifc_ids_work_package(mismatch)
                .unwrap_err()
                .contains(&IfcIdsPreflightError::CoordinateFrameDigestMismatch)
        );
    }

    #[test]
    fn foot_units_are_reported_as_exact_rational_millimetre_conversion() {
        let mut foot = valid_input();
        foot.coordinate_context.length_unit = IfcLengthUnit::Foot;
        let report = preflight_ifc_ids_work_package(foot).unwrap();
        assert_eq!(report.length_to_mm_numerator, 3_048);
        assert_eq!(report.length_to_mm_denominator, 10);
    }

    #[test]
    fn missing_crs_datum_and_unsupported_units_block_preflight() {
        let mut missing_crs = valid_input();
        missing_crs
            .coordinate_context
            .project_crs_identifier
            .clear();
        assert!(
            preflight_ifc_ids_work_package(missing_crs)
                .unwrap_err()
                .contains(&IfcIdsPreflightError::MissingProjectCrsIdentifier)
        );

        let mut missing_datum = valid_input();
        missing_datum
            .coordinate_context
            .vertical_datum_identifier
            .clear();
        assert!(
            preflight_ifc_ids_work_package(missing_datum)
                .unwrap_err()
                .contains(&IfcIdsPreflightError::MissingVerticalDatumIdentifier)
        );

        let mut unsupported = valid_input();
        unsupported.coordinate_context.length_unit = IfcLengthUnit::Unsupported("yard".into());
        assert!(
            preflight_ifc_ids_work_package(unsupported)
                .unwrap_err()
                .contains(&IfcIdsPreflightError::UnsupportedLengthUnit("yard".into()))
        );
    }

    #[test]
    fn ids_applicability_decision_requires_a_receipt_even_for_an_empty_rule_set() {
        let mut input = valid_input();
        input.entities[0].applicable_ids_requirement_ids.clear();
        input.entities[0].ids_requirement_evaluations.clear();
        input.entities[0].ids_applicability_receipt_sha256 = "missing".into();
        assert!(preflight_ifc_ids_work_package(input).unwrap_err().contains(
            &IfcIdsPreflightError::InvalidIdsApplicabilityReceipt {
                global_id: global_id('A'),
            }
        ));
    }

    #[test]
    fn ids_engine_build_must_be_bound_to_the_work_package() {
        let mut input = valid_input();
        input.ids_engine_build_sha256 = "unknown-engine".into();
        assert!(
            preflight_ifc_ids_work_package(input)
                .unwrap_err()
                .contains(&IfcIdsPreflightError::InvalidIdsEngineBuildDigest)
        );
    }
}
