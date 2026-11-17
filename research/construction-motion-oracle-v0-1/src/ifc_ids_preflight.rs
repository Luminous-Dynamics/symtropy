//! Typed preflight adapter for IFC 4.3 ADD2 + buildingSMART IDS 1.0.
//!
//! This module does not parse STEP/IFC or execute IDS XML. It validates a
//! typed record supplied by an IFC/IDS reader, preserves the source GlobalIds,
//! verifies required information is present, and emits a frozen SiteWorkPackage.
//! Canonical package/entity-manifest hashing remains the caller's responsibility.

use std::collections::BTreeSet;

use crate::field_layout_acceptance::{
    CrewHours, DesignIntentEntity, DesignIntentManifest, FieldAcceptanceProfile,
    FieldLayoutEntityType, score_field_layout_run,
};
use crate::site_state_ledger::SiteWorkPackage;

pub const SUPPORTED_IFC_SCHEMA_RELEASE: &str = "IFC4X3_ADD2";
pub const SUPPORTED_IDS_VERSION: &str = "1.0";

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
    pub design_revision_sha256: String,
    pub frame_id: String,
    pub frame_sha256: String,
    pub transform_sha256: String,
    /// Digest of the complete sorted source entity binding manifest, including
    /// GlobalIds, geometry digests, semantic types and IDS property bindings.
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
}

#[derive(Clone, Debug, PartialEq)]
pub struct IfcIdsPreflightReport {
    pub work_package: SiteWorkPackage,
    pub source_entity_bindings: Vec<SourceEntityBinding>,
    pub source_entity_count: u64,
    pub in_scope_entity_count: u64,
    pub ifc_schema_release: String,
    pub ids_version: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IfcIdsPreflightError {
    InvalidPackageDigest,
    InvalidSourceModelDigest,
    UnsupportedIfcSchema(String),
    InvalidIdsSpecDigest,
    UnsupportedIdsVersion(String),
    InvalidDesignRevisionDigest,
    InvalidFrameIdentity,
    InvalidFrameDigest,
    InvalidTransformDigest,
    InvalidEntityManifestDigest,
    InvalidAcceptanceProfileDigest,
    InvalidGlobalId(String),
    DuplicateGlobalId(String),
    InvalidGeometryDigest(String),
    DuplicateRequiredIdsProperty { global_id: String, property: String },
    DuplicateDeliveredIdsProperty { global_id: String, property: String },
    EmptyIdsPropertyIdentifier { global_id: String },
    MissingRequiredIdsProperty { global_id: String, property: String },
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

    Ok(IfcIdsPreflightReport {
        work_package: SiteWorkPackage {
            package_sha256: input.package_sha256,
            source_model_sha256: input.source_model_sha256,
            source_schema_release: input.source_schema_release.clone(),
            ids_spec_sha256: input.ids_spec_sha256,
            ids_version: input.ids_version.clone(),
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
        in_scope_entity_count: source_entity_bindings.len() as u64,
        ifc_schema_release: input.source_schema_release,
        ids_version: input.ids_version,
    })
}

fn is_ifc_global_id(value: &str) -> bool {
    value.len() == 22
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'$')
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
                    global_id: global_id('A'),
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
                    in_work_scope: true,
                },
                IfcSourceEntity {
                    global_id: global_id('B'),
                    entity_type: FieldLayoutEntityType::Point,
                    geometry_sha256: digest('a'),
                    required_ids_properties: vec!["Pset_Layout.MarkingClass".into()],
                    delivered_ids_properties: vec!["Pset_Layout.MarkingClass".into()],
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
            format!("ifc4x3:{}", global_id('A'))
        );
        assert_eq!(report.work_package.source_schema_release, SUPPORTED_IFC_SCHEMA_RELEASE);
        assert_eq!(report.work_package.ids_version, SUPPORTED_IDS_VERSION);
    }

    #[test]
    fn missing_ids_information_blocks_package_compilation() {
        let mut input = valid_input();
        input.entities[0].delivered_ids_properties = vec!["Pset_WallCommon.IsExternal".into()];
        assert!(preflight_ifc_ids_work_package(input).unwrap_err().iter().any(
            |error| matches!(
                error,
                IfcIdsPreflightError::MissingRequiredIdsProperty { global_id, property }
                    if global_id == &global_id('A') && property == "Pset_Layout.MarkingClass"
            )
        ));
    }

    #[test]
    fn duplicate_or_malformed_global_ids_are_not_silently_rewritten() {
        let mut duplicate = valid_input();
        duplicate.entities[1].global_id = global_id('A');
        assert!(
            preflight_ifc_ids_work_package(duplicate)
                .unwrap_err()
                .contains(&IfcIdsPreflightError::DuplicateGlobalId(global_id('A')))
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
            global_id: global_id('C'),
            entity_type: FieldLayoutEntityType::Other,
            geometry_sha256: digest('b'),
            required_ids_properties: Vec::new(),
            delivered_ids_properties: Vec::new(),
            in_work_scope: false,
        });
        let report = preflight_ifc_ids_work_package(input).unwrap();
        assert_eq!(report.source_entity_count, 3);
        assert_eq!(report.in_scope_entity_count, 2);
        assert_eq!(report.work_package.entities.len(), 2);
    }
}
