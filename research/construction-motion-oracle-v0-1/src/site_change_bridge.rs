//! Bridge immutable IFC/IDS work-package revisions into the construction
//! dependency-aware change-impact compiler.
//!
//! The file-level model digest remains provenance. Per-entity source,
//! construction-revision, geometry, IDS-applicability, IDS-rule and geometry
//! validation receipts drive localized requalification. A change to one entity
//! therefore does not automatically invalidate unrelated entity trajectories.
//!
//! This module only compiles a change set from two caller-verified preflight
//! snapshots. The caller still verifies source bytes, event signatures, receipt
//! digests and completeness of the IFC/IDS reader's entity manifest.

use std::collections::{BTreeMap, BTreeSet};

use crate::change_impact::{
    ArtifactDependencyContract, ArtifactKindPlaceholder, ChangeImpactError, ChangeImpactReport,
    ConstructionArtifactGraph, ConstructionArtifactKind, ConstructionChangeSet,
    DependencyContractSet, DependencyDomain, DependencyKey, DependencyTransition,
    VersionedDependency, compile_change_impact_with_contract,
};
use crate::ifc_ids_preflight::{
    IfcIdsPreflightReport, SUPPORTED_IFC_SCHEMA_RELEASE, SUPPORTED_IDS_VERSION,
    SourceEntityBinding,
};

/// Deterministic tombstone version used when a previously present entity or
/// IDS rule is removed from a newly published work package.
///
/// SHA-256("ABSENT") in lowercase hexadecimal. This is a version marker, not
/// a cryptographic proof that an entity was deleted from the source model.
pub const ABSENT_DEPENDENCY_SHA256: &str =
    "08b6a0bc6906cd596f1458be5ac2911c478709b27e4f3c52e8edd0293e2f4c56";

/// SHA-256("IFC4X3_ADD2") in lowercase hexadecimal. The supported IFC release
/// is fixed by the preflight owner; changing supported release requires
/// updating this registry constant and the schema adapter together.
pub const IFC4X3_ADD2_SCHEMA_RELEASE_SHA256: &str =
    "08f1e940bba1a72329f7cd3c70f5b3c6335d880ea490881ad1384ea00b96b58d";

#[derive(Clone, Copy, Debug)]
pub struct SiteRevisionSnapshot<'a> {
    pub preflight: &'a IfcIdsPreflightReport,
    /// Digest of the exact ledger tip before/after this package publication.
    pub site_ledger_state_sha256: &'a str,
}

#[derive(Clone, Debug)]
pub struct SiteChangeImpactRequest<'a> {
    pub change_id: String,
    /// Digest of the authenticated work-package publication event.
    pub source_event_sha256: String,
    pub site_id: String,
    pub floor_id: String,
    pub previous: SiteRevisionSnapshot<'a>,
    pub current: SiteRevisionSnapshot<'a>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SiteChangeImpactError {
    EmptyChangeIdentity,
    EmptySiteOrFloorScope,
    InvalidDigest(&'static str),
    UnsupportedIfcSchemaRelease(String),
    UnsupportedIdsVersion(String),
    InvalidCoordinateContext,
    CoordinateContextPackageMismatch,
    WorkPackagePreflightMismatch,
    DuplicateDesignEntityId(String),
    DuplicateSourceEntityBinding(String),
    MissingSourceEntityBinding(String),
    UnexpectedSourceEntityBinding(String),
    IfcGlobalIdBindingMismatch(String),
    InvalidEntitySourceDigest(String),
    InvalidEntityRevisionDigest(String),
    InvalidEntityGeometryDigest(String),
    InvalidIdsApplicabilityReceipt(String),
    InvalidIdsRequirementReceipt { entity_id: String, requirement_id: String },
    DuplicateIdsRequirementEvaluation { entity_id: String, requirement_id: String },
    InvalidGeometryPreflightReceipt(String),
    GeometryPreflightBindingMismatch(String),
    MissingSitePackageDependencyContract {
        artifact_kind: ConstructionArtifactKind,
        domain: DependencyDomain,
    },
    NoDependencyChanges,
    ChangeImpact(ChangeImpactError),
}

impl From<ChangeImpactError> for SiteChangeImpactError {
    fn from(value: ChangeImpactError) -> Self {
        Self::ChangeImpact(value)
    }
}

/// Build deterministic dependency transitions from two immutable IFC/IDS
/// preflight reports and the corresponding before/after site-ledger tips.
///
/// Global dependencies (source model, schema, IDS specification/engine,
/// floor frame/transform, package manifest and ledger tip) are kept separate
/// from entity-scoped dependencies. Entity-scoped fingerprints are emitted
/// only for entities that existed in the previous revision. Newly added
/// entities are represented by the changed entity-manifest/root preflight
/// artifact and are materialized as new artifacts after compilation. Removed
/// entities receive explicit ABSENT tombstones so old work cannot stay current.
pub fn build_site_package_change_set(
    request: &SiteChangeImpactRequest<'_>,
) -> Result<ConstructionChangeSet, SiteChangeImpactError> {
    if request.change_id.trim().is_empty() {
        return Err(SiteChangeImpactError::EmptyChangeIdentity);
    }
    if request.site_id.trim().is_empty() || request.floor_id.trim().is_empty() {
        return Err(SiteChangeImpactError::EmptySiteOrFloorScope);
    }
    if !is_sha256(&request.source_event_sha256) {
        return Err(SiteChangeImpactError::InvalidDigest("source event"));
    }
    if !is_sha256(request.previous.site_ledger_state_sha256)
        || !is_sha256(request.current.site_ledger_state_sha256)
    {
        return Err(SiteChangeImpactError::InvalidDigest("site ledger state"));
    }

    validate_revision_snapshot(request.previous.preflight)?;
    validate_revision_snapshot(request.current.preflight)?;

    let previous = &request.previous.preflight.work_package;
    let current = &request.current.preflight.work_package;
    let project_scope = format!("site:{}", request.site_id);
    let floor_scope = format!("floor:{}:{}", request.site_id, request.floor_id);
    let mut changes = Vec::new();

    push_if_changed(
        &mut changes,
        DependencyDomain::SourceModel,
        &project_scope,
        &previous.source_model_sha256,
        &current.source_model_sha256,
    );
    push_if_changed(
        &mut changes,
        DependencyDomain::IfcSchema,
        &project_scope,
        ifc_schema_release_digest(&previous.source_schema_release)?,
        ifc_schema_release_digest(&current.source_schema_release)?,
    );
    push_if_changed(
        &mut changes,
        DependencyDomain::IdsSpecification,
        &project_scope,
        &previous.ids_spec_sha256,
        &current.ids_spec_sha256,
    );
    push_if_changed(
        &mut changes,
        DependencyDomain::IdsEngineBuild,
        &project_scope,
        &previous.ids_engine_build_sha256,
        &current.ids_engine_build_sha256,
    );
    push_if_changed(
        &mut changes,
        DependencyDomain::DesignRevision,
        &floor_scope,
        &previous.design_revision_sha256,
        &current.design_revision_sha256,
    );
    push_if_changed(
        &mut changes,
        DependencyDomain::EntityManifest,
        &floor_scope,
        &previous.entity_manifest_sha256,
        &current.entity_manifest_sha256,
    );
    push_if_changed(
        &mut changes,
        DependencyDomain::CoordinateFrame,
        &floor_scope,
        &previous.frame_sha256,
        &current.frame_sha256,
    );
    push_if_changed(
        &mut changes,
        DependencyDomain::CoordinateContext,
        &floor_scope,
        &previous.coordinate_context_sha256,
        &current.coordinate_context_sha256,
    );
    push_if_changed(
        &mut changes,
        DependencyDomain::CoordinateTransform,
        &floor_scope,
        &previous.transform_sha256,
        &current.transform_sha256,
    );
    push_if_changed(
        &mut changes,
        DependencyDomain::AcceptanceProfile,
        &floor_scope,
        &previous.acceptance_profile_sha256,
        &current.acceptance_profile_sha256,
    );
    push_if_changed(
        &mut changes,
        DependencyDomain::SiteLedgerState,
        &project_scope,
        request.previous.site_ledger_state_sha256,
        request.current.site_ledger_state_sha256,
    );

    let previous_entities = entity_bindings_by_id(request.previous.preflight)?;
    let current_entities = entity_bindings_by_id(request.current.preflight)?;

    // Only old entities can have existing work artifacts. Added entities are
    // represented by the entity-manifest transition and new artifact creation.
    for (entity_id, old_binding) in &previous_entities {
        let entity_scope = entity_scope(&request.site_id, &request.floor_id, entity_id);
        let new_binding = current_entities.get(entity_id.as_str()).copied();

        append_entity_binding_transitions(
            &mut changes,
            &entity_scope,
            old_binding,
            new_binding,
        )?;
    }

    changes.sort_by(|left, right| left.key.cmp(&right.key));
    if changes.is_empty() {
        return Err(SiteChangeImpactError::NoDependencyChanges);
    }

    Ok(ConstructionChangeSet {
        change_id: request.change_id.clone(),
        source_event_sha256: request.source_event_sha256.clone(),
        changes,
    })
}

/// Compile the site revision change through the owner dependency contract.
///
/// The additional site-package contract is deliberately stricter than the
/// generic construction contract. Per-entity artifacts must explicitly bind
/// source entity, entity revision, geometry, IDS applicability/rules and
/// independent geometry-preflight receipts; shared artifacts must bind the
/// full coordinate context. Omitting one of these domains keeps this gate
/// closed rather than silently reducing invalidation coverage.
pub fn compile_site_package_change_impact(
    graph: &ConstructionArtifactGraph,
    contracts: &DependencyContractSet,
    request: &SiteChangeImpactRequest<'_>,
) -> Result<ChangeImpactReport, SiteChangeImpactError> {
    validate_site_package_dependency_contracts(contracts)?;
    let changes = build_site_package_change_set(request)?;
    compile_change_impact_with_contract(graph, contracts, &changes)
        .map_err(SiteChangeImpactError::from)
}

fn validate_site_package_dependency_contracts(
    contracts: &DependencyContractSet,
) -> Result<(), SiteChangeImpactError> {
    let requirements: &[(ConstructionArtifactKind, &[DependencyDomain])] = &[
        (
            ConstructionArtifactKind::IfcIdsPreflight,
            &[
                DependencyDomain::CoordinateContext,
                DependencyDomain::EntityManifest,
                DependencyDomain::EntitySource,
                DependencyDomain::EntityDesignRevision,
                DependencyDomain::EntityGeometry,
                DependencyDomain::IdsEntityApplicability,
                DependencyDomain::IdsRequirementEvaluation,
                DependencyDomain::GeometryPreflight,
                DependencyDomain::GeometryValidatorBuild,
            ],
        ),
        (
            ConstructionArtifactKind::DesignWorkPackage,
            &[
                DependencyDomain::CoordinateContext,
                DependencyDomain::EntitySource,
                DependencyDomain::EntityDesignRevision,
                DependencyDomain::EntityGeometry,
                DependencyDomain::IdsEntityApplicability,
                DependencyDomain::IdsRequirementEvaluation,
                DependencyDomain::GeometryPreflight,
                DependencyDomain::GeometryValidatorBuild,
            ],
        ),
        (
            ConstructionArtifactKind::RobotTrajectory,
            &[
                DependencyDomain::CoordinateContext,
                DependencyDomain::EntitySource,
                DependencyDomain::EntityDesignRevision,
                DependencyDomain::EntityGeometry,
                DependencyDomain::IdsEntityApplicability,
                DependencyDomain::IdsRequirementEvaluation,
                DependencyDomain::GeometryPreflight,
                DependencyDomain::GeometryValidatorBuild,
            ],
        ),
        (
            ConstructionArtifactKind::MotionEnvelope,
            &[
                DependencyDomain::CoordinateContext,
                DependencyDomain::EntitySource,
                DependencyDomain::EntityDesignRevision,
                DependencyDomain::EntityGeometry,
                DependencyDomain::IdsEntityApplicability,
                DependencyDomain::IdsRequirementEvaluation,
                DependencyDomain::GeometryPreflight,
                DependencyDomain::GeometryValidatorBuild,
            ],
        ),
        (
            ConstructionArtifactKind::StationQualification,
            &[DependencyDomain::CoordinateContext],
        ),
        (
            ConstructionArtifactKind::SpaceTimeOccupancyAssessment,
            &[DependencyDomain::CoordinateContext],
        ),
        (
            ConstructionArtifactKind::DispatchReadiness,
            &[DependencyDomain::CoordinateContext],
        ),
        (
            ConstructionArtifactKind::InspectionPlan,
            &[
                DependencyDomain::CoordinateContext,
                DependencyDomain::EntitySource,
                DependencyDomain::EntityDesignRevision,
                DependencyDomain::EntityGeometry,
                DependencyDomain::IdsEntityApplicability,
                DependencyDomain::IdsRequirementEvaluation,
                DependencyDomain::GeometryPreflight,
                DependencyDomain::GeometryValidatorBuild,
            ],
        ),
        (
            ConstructionArtifactKind::AsBuiltMeasurement,
            &[
                DependencyDomain::CoordinateContext,
                DependencyDomain::EntitySource,
                DependencyDomain::EntityDesignRevision,
                DependencyDomain::EntityGeometry,
                DependencyDomain::GeometryPreflight,
                DependencyDomain::GeometryValidatorBuild,
            ],
        ),
        (
            ConstructionArtifactKind::AcceptedConstructionRecord,
            &[
                DependencyDomain::CoordinateContext,
                DependencyDomain::EntitySource,
                DependencyDomain::EntityDesignRevision,
                DependencyDomain::EntityGeometry,
                DependencyDomain::IdsEntityApplicability,
                DependencyDomain::IdsRequirementEvaluation,
                DependencyDomain::GeometryPreflight,
                DependencyDomain::GeometryValidatorBuild,
            ],
        ),
        (
            ConstructionArtifactKind::HandoverRecord,
            &[DependencyDomain::CoordinateContext],
        ),
    ];

    for (kind, required_domains) in requirements {
        let contract = contracts
            .contracts
            .iter()
            .find(|contract| contract.artifact_kind == *kind);
        if let Some(contract) = contract {
            for domain in *required_domains {
                if !contract.required_domains.contains(domain) {
                    return Err(SiteChangeImpactError::MissingSitePackageDependencyContract {
                        artifact_kind: *kind,
                        domain: *domain,
                    });
                }
            }
        }
    }
    Ok(())
}

fn validate_revision_snapshot(
    report: &IfcIdsPreflightReport,
) -> Result<(), SiteChangeImpactError> {
    let package = &report.work_package;
    for (label, digest) in [
        ("package", package.package_sha256.as_str()),
        ("source model", package.source_model_sha256.as_str()),
        ("IDS specification", package.ids_spec_sha256.as_str()),
        ("IDS engine build", package.ids_engine_build_sha256.as_str()),
        ("design revision", package.design_revision_sha256.as_str()),
        ("frame", package.frame_sha256.as_str()),
        ("coordinate context", package.coordinate_context_sha256.as_str()),
        ("transform", package.transform_sha256.as_str()),
        ("entity manifest", package.entity_manifest_sha256.as_str()),
        ("acceptance profile", package.acceptance_profile_sha256.as_str()),
    ] {
        if !is_sha256(digest) {
            return Err(SiteChangeImpactError::InvalidDigest(label));
        }
    }
    if package.source_schema_release != report.ifc_schema_release {
        return Err(SiteChangeImpactError::WorkPackagePreflightMismatch);
    }
    if package.ids_version != report.ids_version {
        return Err(SiteChangeImpactError::WorkPackagePreflightMismatch);
    }
    if package.source_schema_release != SUPPORTED_IFC_SCHEMA_RELEASE {
        return Err(SiteChangeImpactError::UnsupportedIfcSchemaRelease(
            package.source_schema_release.clone(),
        ));
    }
    if package.ids_version != SUPPORTED_IDS_VERSION {
        return Err(SiteChangeImpactError::UnsupportedIdsVersion(
            package.ids_version.clone(),
        ));
    }
    if package.frame_id.trim().is_empty()
        || report.coordinate_context.model_frame_id != package.frame_id
        || report.coordinate_context.frame_sha256 != package.frame_sha256
        || report.coordinate_context.transform_sha256 != package.transform_sha256
        || report.coordinate_context.project_crs_identifier.trim().is_empty()
        || report.coordinate_context.vertical_datum_identifier.trim().is_empty()
        || report.length_to_mm_denominator == 0
        || report.length_to_mm_numerator == 0
    {
        return Err(SiteChangeImpactError::CoordinateContextPackageMismatch);
    }
    if package.coordinate_context_sha256.trim().is_empty() {
        return Err(SiteChangeImpactError::InvalidCoordinateContext);
    }

    let package_ids: BTreeSet<&str> = package
        .entities
        .iter()
        .map(|entity| entity.design_entity_id.as_str())
        .collect();
    if package_ids.len() != package.entities.len() {
        return Err(SiteChangeImpactError::DuplicateDesignEntityId(
            "duplicate package entity".into(),
        ));
    }

    let bindings = entity_bindings_by_id(report)?;
    if bindings.len() != package.entities.len() {
        return Err(SiteChangeImpactError::WorkPackagePreflightMismatch);
    }
    for entity_id in package_ids {
        if !bindings.contains_key(entity_id) {
            return Err(SiteChangeImpactError::MissingSourceEntityBinding(
                entity_id.to_owned(),
            ));
        }
    }
    if report.in_scope_entity_count as usize != bindings.len() {
        return Err(SiteChangeImpactError::WorkPackagePreflightMismatch);
    }
    Ok(())
}

fn entity_bindings_by_id<'a>(
    report: &'a IfcIdsPreflightReport,
) -> Result<BTreeMap<&'a str, &'a SourceEntityBinding>, SiteChangeImpactError> {
    let mut result = BTreeMap::new();
    let package_ids: BTreeSet<&str> = report
        .work_package
        .entities
        .iter()
        .map(|entity| entity.design_entity_id.as_str())
        .collect();

    for binding in &report.source_entity_bindings {
        if result
            .insert(binding.design_entity_id.as_str(), binding)
            .is_some()
        {
            return Err(SiteChangeImpactError::DuplicateSourceEntityBinding(
                binding.design_entity_id.clone(),
            ));
        }
        if !package_ids.contains(binding.design_entity_id.as_str()) {
            return Err(SiteChangeImpactError::UnexpectedSourceEntityBinding(
                binding.design_entity_id.clone(),
            ));
        }
        let expected_id = format!("ifc4x3:{}", binding.ifc_global_id);
        if binding.design_entity_id != expected_id {
            return Err(SiteChangeImpactError::IfcGlobalIdBindingMismatch(
                binding.design_entity_id.clone(),
            ));
        }
        if !is_sha256(&binding.source_entity_sha256) {
            return Err(SiteChangeImpactError::InvalidEntitySourceDigest(
                binding.design_entity_id.clone(),
            ));
        }
        if !is_sha256(&binding.entity_revision_sha256) {
            return Err(SiteChangeImpactError::InvalidEntityRevisionDigest(
                binding.design_entity_id.clone(),
            ));
        }
        if !is_sha256(&binding.geometry_sha256) {
            return Err(SiteChangeImpactError::InvalidEntityGeometryDigest(
                binding.design_entity_id.clone(),
            ));
        }
        if !is_sha256(&binding.ids_applicability_receipt_sha256) {
            return Err(SiteChangeImpactError::InvalidIdsApplicabilityReceipt(
                binding.design_entity_id.clone(),
            ));
        }
        if !is_sha256(&binding.geometry_preflight.evaluation_receipt_sha256)
            || !is_sha256(&binding.geometry_preflight.validator_build_sha256)
            || binding.geometry_preflight.geometry_sha256 != binding.geometry_sha256
        {
            return Err(SiteChangeImpactError::InvalidGeometryPreflightReceipt(
                binding.design_entity_id.clone(),
            ));
        }
        if binding.geometry_preflight.frame_sha256 != report.work_package.frame_sha256
            || binding.geometry_preflight.transform_sha256
                != report.work_package.transform_sha256
            || !binding.geometry_preflight.placements_resolved
            || !binding.geometry_preflight.geometry_valid
            || !binding.geometry_preflight.coordinate_transform_valid
            || !binding.geometry_preflight.clash_check_complete
            || binding.geometry_preflight.critical_clash_count > 0
            || binding.geometry_preflight.unresolved_geometry_issue_count > 0
        {
            return Err(SiteChangeImpactError::GeometryPreflightBindingMismatch(
                binding.design_entity_id.clone(),
            ));
        }
        let mut requirement_ids = BTreeSet::new();
        for evaluation in &binding.ids_requirement_evaluations {
            if !requirement_ids.insert(evaluation.requirement_id.as_str()) {
                return Err(SiteChangeImpactError::DuplicateIdsRequirementEvaluation {
                    entity_id: binding.design_entity_id.clone(),
                    requirement_id: evaluation.requirement_id.clone(),
                });
            }
            if !evaluation.passed || !is_sha256(&evaluation.evaluation_receipt_sha256) {
                return Err(SiteChangeImpactError::InvalidIdsRequirementReceipt {
                    entity_id: binding.design_entity_id.clone(),
                    requirement_id: evaluation.requirement_id.clone(),
                });
            }
        }
    }

    Ok(result)
}

fn append_entity_binding_transitions(
    changes: &mut Vec<DependencyTransition>,
    scope: &str,
    previous: &SourceEntityBinding,
    current: Option<&SourceEntityBinding>,
) -> Result<(), SiteChangeImpactError> {
    let absent = ABSENT_DEPENDENCY_SHA256;
    let current_source = current.map_or(absent, |binding| binding.source_entity_sha256.as_str());
    let current_revision = current.map_or(absent, |binding| binding.entity_revision_sha256.as_str());
    let current_geometry = current.map_or(absent, |binding| binding.geometry_sha256.as_str());
    let current_ids_applicability =
        current.map_or(absent, |binding| binding.ids_applicability_receipt_sha256.as_str());
    let current_geometry_preflight = current.map_or(absent, |binding| {
        binding.geometry_preflight.evaluation_receipt_sha256.as_str()
    });
    let current_geometry_validator = current.map_or(absent, |binding| {
        binding.geometry_preflight.validator_build_sha256.as_str()
    });

    for (domain, old_digest, new_digest) in [
        (
            DependencyDomain::EntitySource,
            previous.source_entity_sha256.as_str(),
            current_source,
        ),
        (
            DependencyDomain::EntityDesignRevision,
            previous.entity_revision_sha256.as_str(),
            current_revision,
        ),
        (
            DependencyDomain::EntityGeometry,
            previous.geometry_sha256.as_str(),
            current_geometry,
        ),
        (
            DependencyDomain::IdsEntityApplicability,
            previous.ids_applicability_receipt_sha256.as_str(),
            current_ids_applicability,
        ),
        (
            DependencyDomain::GeometryPreflight,
            previous.geometry_preflight.evaluation_receipt_sha256.as_str(),
            current_geometry_preflight,
        ),
        (
            DependencyDomain::GeometryValidatorBuild,
            previous.geometry_preflight.validator_build_sha256.as_str(),
            current_geometry_validator,
        ),
    ] {
        push_if_changed(changes, domain, scope, old_digest, new_digest);
    }

    let new_evaluations: BTreeMap<&str, &str> = current
        .map(|binding| {
            binding
                .ids_requirement_evaluations
                .iter()
                .map(|evaluation| {
                    (
                        evaluation.requirement_id.as_str(),
                        evaluation.evaluation_receipt_sha256.as_str(),
                    )
                })
                .collect()
        })
        .unwrap_or_default();

    for previous_evaluation in &previous.ids_requirement_evaluations {
        let new_digest = new_evaluations
            .get(previous_evaluation.requirement_id.as_str())
            .copied()
            .unwrap_or(absent);
        let requirement_scope = format!("{scope}:ids:{}", previous_evaluation.requirement_id);
        push_if_changed(
            changes,
            DependencyDomain::IdsRequirementEvaluation,
            &requirement_scope,
            &previous_evaluation.evaluation_receipt_sha256,
            new_digest,
        );
    }

    Ok(())
}

fn push_if_changed(
    changes: &mut Vec<DependencyTransition>,
    domain: DependencyDomain,
    scope_id: &str,
    previous_sha256: &str,
    current_sha256: &str,
) {
    if previous_sha256 != current_sha256 {
        changes.push(DependencyTransition {
            key: DependencyKey {
                domain,
                scope_id: scope_id.to_owned(),
            },
            previous_sha256: previous_sha256.to_owned(),
            current_sha256: current_sha256.to_owned(),
        });
    }
}

fn entity_scope(site_id: &str, floor_id: &str, entity_id: &str) -> String {
    format!("entity:{site_id}:{floor_id}:{entity_id}")
}

fn ifc_schema_release_digest(release: &str) -> Result<&'static str, SiteChangeImpactError> {
    match release {
        "IFC4X3_ADD2" => Ok(IFC4X3_ADD2_SCHEMA_RELEASE_SHA256),
        _ => Err(SiteChangeImpactError::UnsupportedIfcSchemaRelease(
            release.to_owned(),
        )),
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
    use crate::field_layout_acceptance::FieldLayoutEntityType;
    use crate::ifc_ids_preflight::{
        GeometryPreflightEvaluation, IfcCoordinateContext, IfcLengthUnit,
        IdsRequirementEvaluation, SourceEntityBinding,
    };

    fn digest(value: char) -> String {
        std::iter::repeat_n(value, 64).collect()
    }

    fn entity_binding(
        id: &str,
        value: char,
        requirement_id: &str,
    ) -> SourceEntityBinding {
        let geometry = digest(value);
        SourceEntityBinding {
            design_entity_id: format!("ifc4x3:{id}"),
            ifc_global_id: id.into(),
            source_entity_sha256: digest((value as u8).wrapping_add(1) as char),
            entity_revision_sha256: digest((value as u8).wrapping_add(2) as char),
            geometry_sha256: geometry.clone(),
            required_ids_properties: vec!["Pset_Layout.MarkingClass".into()],
            ids_applicability_receipt_sha256: digest((value as u8).wrapping_add(3) as char),
            geometry_preflight: GeometryPreflightEvaluation {
                geometry_sha256: geometry,
                validator_build_sha256: digest('d'),
                frame_sha256: digest('e'),
                transform_sha256: digest('f'),
                placements_resolved: true,
                geometry_valid: true,
                coordinate_transform_valid: true,
                clash_check_complete: true,
                critical_clash_count: 0,
                unresolved_geometry_issue_count: 0,
                evaluation_receipt_sha256: digest((value as u8).wrapping_add(4) as char),
            },
            applicable_ids_requirement_ids: vec![requirement_id.into()],
            ids_requirement_evaluations: vec![IdsRequirementEvaluation {
                requirement_id: requirement_id.into(),
                passed: true,
                evaluation_receipt_sha256: digest((value as u8).wrapping_add(5) as char),
            }],
        }
    }

    fn report(
        package_char: char,
        model_char: char,
        revision_char: char,
        entity_bindings: Vec<SourceEntityBinding>,
    ) -> IfcIdsPreflightReport {
        let coordinate_context = IfcCoordinateContext {
            model_frame_id: "site-floor-02".into(),
            project_crs_identifier: "EPSG:32735".into(),
            vertical_datum_identifier: "local-datum".into(),
            length_unit: IfcLengthUnit::Millimetre,
            model_to_site_rotation: [
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
            ],
            model_to_site_translation_mm: [0.0, 0.0, 0.0],
            map_conversion_scale: 1.0,
            frame_sha256: digest('e'),
            transform_sha256: digest('f'),
        };
        let entities = entity_bindings
            .iter()
            .map(|binding| crate::field_layout_acceptance::DesignIntentEntity {
                design_entity_id: binding.design_entity_id.clone(),
                entity_type: FieldLayoutEntityType::Line,
            })
            .collect::<Vec<_>>();
        let in_scope = entities.len() as u64;
        IfcIdsPreflightReport {
            work_package: crate::site_state_ledger::SiteWorkPackage {
                package_sha256: digest(package_char),
                source_model_sha256: digest(model_char),
                source_schema_release: "IFC4X3_ADD2".into(),
                ids_spec_sha256: digest('a'),
                ids_version: "1.0".into(),
                ids_engine_build_sha256: digest('b'),
                coordinate_context_sha256: digest('c'),
                design_revision_sha256: digest(revision_char),
                frame_id: "site-floor-02".into(),
                frame_sha256: digest('e'),
                transform_sha256: digest('f'),
                entity_manifest_sha256: digest((package_char as u8).wrapping_add(1) as char),
                acceptance_profile_sha256: digest('d'),
                entities,
                acceptance_profile: crate::field_layout_acceptance::FieldAcceptanceProfile {
                    max_abs_xy_residual_mm: 1.0,
                    max_reference_u95_xy_mm: 0.25,
                    max_abs_z_residual_mm: None,
                    max_reference_u95_z_mm: None,
                    max_abs_endpoint_residual_mm: Some(1.0),
                },
            },
            source_entity_bindings: entity_bindings,
            source_entity_count: in_scope,
            in_scope_entity_count: in_scope,
            ifc_schema_release: "IFC4X3_ADD2".into(),
            ids_version: "1.0".into(),
            coordinate_context,
            length_to_mm_numerator: 1,
            length_to_mm_denominator: 1,
        }
    }

    fn request<'a>(
        previous: &'a IfcIdsPreflightReport,
        current: &'a IfcIdsPreflightReport,
    ) -> SiteChangeImpactRequest<'a> {
        SiteChangeImpactRequest {
            change_id: "ifc-change-001".into(),
            source_event_sha256: digest('9'),
            site_id: "site-4".into(),
            floor_id: "floor-02".into(),
            previous: SiteRevisionSnapshot {
                preflight: previous,
                site_ledger_state_sha256: &digest('7'),
            },
            current: SiteRevisionSnapshot {
                preflight: current,
                site_ledger_state_sha256: &digest('8'),
            },
        }
    }

    #[test]
    fn localized_entity_change_emits_scoped_dependencies_only_for_changed_entity() {
        let previous = report(
            '1',
            '2',
            '3',
            vec![entity_binding("wall-a", '4', "layout-rule"), entity_binding("wall-b", '5', "layout-rule")],
        );
        let current = report(
            '6',
            '7',
            '8',
            vec![entity_binding("wall-a", '9', "layout-rule"), entity_binding("wall-b", '5', "layout-rule")],
        );
        let request = request(&previous, &current);
        let changes = build_site_package_change_set(&request).unwrap();

        let changed_a = entity_scope("site-4", "floor-02", "ifc4x3:wall-a");
        let unchanged_b = entity_scope("site-4", "floor-02", "ifc4x3:wall-b");
        assert!(changes.changes.iter().any(|transition| {
            transition.key.domain == DependencyDomain::EntityGeometry
                && transition.key.scope_id == changed_a
        }));
        assert!(!changes.changes.iter().any(|transition| {
            matches!(
                transition.key.domain,
                DependencyDomain::EntitySource
                    | DependencyDomain::EntityDesignRevision
                    | DependencyDomain::EntityGeometry
                    | DependencyDomain::IdsEntityApplicability
                    | DependencyDomain::GeometryPreflight
                    | DependencyDomain::GeometryValidatorBuild
            ) && transition.key.scope_id == unchanged_b
        }));
        assert!(changes.changes.iter().any(|transition| {
            transition.key.domain == DependencyDomain::SourceModel
                && transition.key.scope_id == "site:site-4"
        }));
        assert!(changes.changes.iter().any(|transition| {
            transition.key.domain == DependencyDomain::EntityManifest
                && transition.key.scope_id == "floor:site-4:floor-02"
        }));
    }

    #[test]
    fn removed_entity_emits_absent_tombstones_for_old_work_dependencies() {
        let previous = report(
            '1',
            '2',
            '3',
            vec![entity_binding("wall-a", '4', "layout-rule"), entity_binding("wall-b", '5', "layout-rule")],
        );
        let current = report('6', '7', '8', vec![entity_binding("wall-a", '4', "layout-rule")]);
        let changes = build_site_package_change_set(&request(&previous, &current)).unwrap();
        let removed_scope = entity_scope("site-4", "floor-02", "ifc4x3:wall-b");
        assert!(changes.changes.iter().any(|transition| {
            transition.key.domain == DependencyDomain::EntityGeometry
                && transition.key.scope_id == removed_scope
                && transition.current_sha256 == ABSENT_DEPENDENCY_SHA256
        }));
        assert!(changes.changes.iter().any(|transition| {
            transition.key.domain == DependencyDomain::IdsRequirementEvaluation
                && transition.key.scope_id.ends_with(":ids:layout-rule")
                && transition.current_sha256 == ABSENT_DEPENDENCY_SHA256
        }));
    }

    #[test]
    fn shared_coordinate_context_change_emits_floor_scoped_transition() {
        let previous = report('1', '2', '3', vec![entity_binding("wall-a", '4', "layout-rule")]);
        let mut current = report('6', '7', '8', vec![entity_binding("wall-a", '4', "layout-rule")]);
        current.work_package.coordinate_context_sha256 = digest('0');
        let changes = build_site_package_change_set(&request(&previous, &current)).unwrap();
        assert!(changes.changes.iter().any(|transition| {
            transition.key.domain == DependencyDomain::CoordinateContext
                && transition.key.scope_id == "floor:site-4:floor-02"
        }));
    }

    #[test]
    fn duplicate_or_incomplete_source_bindings_fail_closed() {
        let previous = report('1', '2', '3', vec![entity_binding("wall-a", '4', "layout-rule")]);
        let current_duplicate = report(
            '6',
            '7',
            '8',
            vec![
                entity_binding("wall-a", '4', "layout-rule"),
                entity_binding("wall-a", '5', "layout-rule"),
            ],
        );
        assert!(matches!(
            build_site_package_change_set(&request(&previous, &current_duplicate)),
            Err(SiteChangeImpactError::DuplicateSourceEntityBinding(_))
                | Err(SiteChangeImpactError::DuplicateDesignEntityId(_))
        ));

        let mut current_missing = report('6', '7', '8', Vec::new());
        current_missing.in_scope_entity_count = 1;
        assert!(matches!(
            build_site_package_change_set(&request(&previous, &current_missing)),
            Err(SiteChangeImpactError::WorkPackagePreflightMismatch)
        ));
    }
}
