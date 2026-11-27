//! Derive MOEM evidence maturity from typed, scope-bound qualification receipts.
//!
//! This evaluator checks evidence graph structure. It does not authenticate
//! issuers, confer certification, verify accreditation registries, or qualify
//! a physical process.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MaturityLevel {
    Unestablished,
    M1,
    M2,
    M3,
    M4,
    M5,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceClass {
    PhysicalMeasurement,
    ProductionRecord,
    PublishedLiterature,
    SyntheticControl,
    ExternalAttestation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceKind {
    DesignModel,
    MaterialPropertyMeasurement,
    ProcessRecord,
    MicrostructureCharacterization,
    PspPLink,
    AsBuiltInspection,
    ComponentFunctionalTest,
    ComponentAcceptance,
    CohortSummary,
    VariabilityAnalysis,
    DurabilityTest,
    SiteQualitySystem,
    EquipmentIq,
    EquipmentOq,
    EquipmentPq,
    LaboratoryMethodScope,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CandidateScope {
    pub application_id: String,
    pub route_family: String,
    pub route_revision: String,
    pub material_family: String,
    pub product_form: String,
    pub target_function_digest: String,
    pub measurement_setup_digest: String,
    pub acceptance_policy_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplicationPolicy {
    pub minimum_independent_builds: u32,
    pub minimum_distinct_material_lots: u32,
    pub policy_id: String,
    pub policy_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub candidate_id: String,
    pub scope: CandidateScope,
    pub component_id: String,
    pub pbf_lb_equipment_qualification_required: bool,
    pub replication_policy: ReplicationPolicy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceReceipt {
    pub evidence_id: String,
    pub digest: String,
    pub kind: EvidenceKind,
    pub class: EvidenceClass,
    pub scope: CandidateScope,
    pub specimen_id: Option<String>,
    pub material_lot_id: Option<String>,
    pub build_id: Option<String>,
    pub component_id: Option<String>,
    pub cohort_id: Option<String>,
    pub method_id: Option<String>,
    pub method_scope_id: Option<String>,
    pub laboratory_id: Option<String>,
    pub calibration_record_id: Option<String>,
    pub calibration_digest: Option<String>,
    pub uncertainty_record_id: Option<String>,
    pub uncertainty_digest: Option<String>,
    pub conditions_digest: Option<String>,
    pub geometry_digest: Option<String>,
    pub standard_reference: Option<String>,
    pub linked_evidence_ids: Vec<String>,
    pub replication: Option<ReplicationEvidence>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplicationEvidence {
    pub independent_builds: u32,
    pub distinct_material_lots: u32,
    pub cohort_policy_digest: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Capability {
    ConceptModel,
    CouponMaterialProperties,
    ProcessStructureProperty,
    AsBuiltGeometry,
    ComponentFunction,
    ComponentAcceptance,
    ReplicatedBuildLotCohort,
    VariabilityControl,
    Durability,
    ProductionSiteQuality,
    EquipmentIqOqPq,
    LaboratoryMethodScope,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Blocker {
    MissingCandidateIdentity,
    InvalidCandidateDigest,
    InvalidReplicationPolicy,
    DuplicateEvidenceId,
    InvalidEvidenceDigest,
    ScopeMismatch,
    MissingEvidenceIdentity,
    InadmissibleEvidenceClass,
    MissingSpecimenBinding,
    MissingLotBinding,
    MissingBuildBinding,
    MissingComponentBinding,
    MissingMethodBinding,
    MissingMethodScope,
    MissingLaboratoryBinding,
    MissingCalibration,
    MissingUncertainty,
    MissingConditions,
    MissingGeometryBinding,
    MissingPspPLink,
    BrokenPspPLink,
    MissingCohortIdentity,
    InsufficientIndependentBuilds,
    InsufficientDistinctMaterialLots,
    WrongCohortPolicy,
    MissingVariabilityAnalysis,
    MissingDurabilityEvidence,
    MissingSiteQualityEvidence,
    WrongSiteStandardReference,
    MissingEquipmentIq,
    MissingEquipmentOq,
    MissingEquipmentPq,
    WrongEquipmentStandardReference,
    MissingLaboratoryScopeEvidence,
    MissingComponentAcceptance,
    LiteratureCannotPromotePhysicalLevel,
    SyntheticCannotPromotePhysicalLevel,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LevelFinding {
    pub level: MaturityLevel,
    pub supported: bool,
    pub required_capabilities: Vec<Capability>,
    pub missing_capabilities: Vec<Capability>,
    pub blockers: Vec<Blocker>,
    pub evidence_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaturityEvaluation {
    pub candidate_id: String,
    pub derived_level: MaturityLevel,
    pub findings: Vec<LevelFinding>,
    pub capability_vector: BTreeMap<Capability, bool>,
    pub invalid_evidence_ids: Vec<String>,
    pub global_blockers: Vec<Blocker>,
    pub claim_ceiling: String,
}

pub fn evaluate(candidate: &Candidate, receipts: &[EvidenceReceipt]) -> MaturityEvaluation {
    let mut global_blockers = validate_candidate(candidate);
    let mut receipt_counts = BTreeMap::<&str, usize>::new();
    for receipt in receipts {
        *receipt_counts.entry(receipt.evidence_id.as_str()).or_default() += 1;
    }

    let mut valid_receipts = Vec::new();
    let mut invalid_evidence_ids = BTreeSet::new();
    let mut evidence_by_id = BTreeMap::new();
    for receipt in receipts {
        let mut blockers = validate_receipt(candidate, receipt);
        if receipt_counts.get(receipt.evidence_id.as_str()).copied().unwrap_or(0) > 1 {
            blockers.push(Blocker::DuplicateEvidenceId);
        }
        if !blockers.is_empty() {
            invalid_evidence_ids.insert(receipt.evidence_id.clone());
            global_blockers.extend(blockers);
            continue;
        }
        evidence_by_id.insert(receipt.evidence_id.as_str(), receipt);
        valid_receipts.push(receipt);
    }

    let has = |kind: EvidenceKind| {
        valid_receipts
            .iter()
            .filter(|receipt| receipt.kind == kind)
            .copied()
            .collect::<Vec<_>>()
    };

    let m1 = has(EvidenceKind::DesignModel);
    let lab_scopes = has(EvidenceKind::LaboratoryMethodScope)
        .into_iter()
        .filter(|receipt| {
            receipt.class == EvidenceClass::ExternalAttestation
                && receipt.laboratory_id.is_some()
                && receipt.method_scope_id.is_some()
                && receipt.standard_reference.as_deref() == Some("ISO/IEC 17025:2017")
        })
        .collect::<Vec<_>>();
    let m2_measurements = has(EvidenceKind::MaterialPropertyMeasurement)
        .into_iter()
        .filter(|receipt| {
            receipt.class == EvidenceClass::PhysicalMeasurement
                && receipt.specimen_id.is_some()
                && receipt.material_lot_id.is_some()
                && method_evidence_complete(receipt)
        })
        .collect::<Vec<_>>();
    let m2_lab_scope_ok = !m2_measurements.is_empty()
        && m2_measurements
            .iter()
            .all(|receipt| lab_scope_covers(receipt, &lab_scopes));
    let m2 = !m2_measurements.is_empty() && m2_lab_scope_ok;
    if !m2_measurements.is_empty() && !m2_lab_scope_ok {
        global_blockers.push(Blocker::MissingLaboratoryScopeEvidence);
    }

    let process_records = has(EvidenceKind::ProcessRecord)
        .into_iter()
        .filter(|receipt| {
            receipt.class == EvidenceClass::ProductionRecord
                && receipt.material_lot_id.is_some()
                && receipt.build_id.is_some()
        })
        .collect::<Vec<_>>();
    let microstructure = has(EvidenceKind::MicrostructureCharacterization)
        .into_iter()
        .filter(|receipt| {
            receipt.class == EvidenceClass::PhysicalMeasurement
                && receipt.specimen_id.is_some()
                && receipt.material_lot_id.is_some()
                && method_evidence_complete(receipt)
        })
        .collect::<Vec<_>>();
    let links = has(EvidenceKind::PspPLink)
        .into_iter()
        .filter(|receipt| {
            receipt.class == EvidenceClass::ProductionRecord
                && receipt.linked_evidence_ids.len() == 3
                && receipt.linked_evidence_ids.iter().all(|id| evidence_by_id.contains_key(id.as_str()))
        })
        .collect::<Vec<_>>();
    let m3_lab_scope_ok = microstructure
        .iter()
        .all(|receipt| lab_scope_covers(receipt, &lab_scopes));
    if !microstructure.is_empty() && !m3_lab_scope_ok {
        global_blockers.push(Blocker::MissingLaboratoryScopeEvidence);
    }
    let m3 = m2
        && !process_records.is_empty()
        && !microstructure.is_empty()
        && !links.is_empty()
        && m3_lab_scope_ok
        && links.iter().any(|link| {
            let linked_process = link.linked_evidence_ids.iter().find_map(|id| {
                process_records.iter().find(|receipt| receipt.evidence_id == *id)
            });
            let linked_microstructure = link.linked_evidence_ids.iter().find_map(|id| {
                microstructure.iter().find(|receipt| receipt.evidence_id == *id)
            });
            let linked_property = link.linked_evidence_ids.iter().find_map(|id| {
                m2_measurements.iter().find(|receipt| receipt.evidence_id == *id)
            });
            matches!(
                (linked_process, linked_microstructure, linked_property),
                (Some(process), Some(structure), Some(property))
                    if process.material_lot_id == structure.material_lot_id
                        && structure.material_lot_id == property.material_lot_id
                        && structure.specimen_id == property.specimen_id
                        && structure.build_id == process.build_id
                        && property.build_id == process.build_id
            )
        });

    let as_built = has(EvidenceKind::AsBuiltInspection)
        .into_iter()
        .filter(|receipt| {
            receipt.class == EvidenceClass::PhysicalMeasurement
                && receipt.component_id.as_deref() == Some(candidate.component_id.as_str())
                && receipt.geometry_digest.as_ref().is_some_and(valid_sha256)
                && method_evidence_complete(receipt)
        })
        .collect::<Vec<_>>();
    let component_function = has(EvidenceKind::ComponentFunctionalTest)
        .into_iter()
        .filter(|receipt| {
            receipt.class == EvidenceClass::PhysicalMeasurement
                && receipt.component_id.as_deref() == Some(candidate.component_id.as_str())
                && receipt.conditions_digest.as_ref().is_some_and(valid_sha256)
                && method_evidence_complete(receipt)
        })
        .collect::<Vec<_>>();
    let component_acceptance = has(EvidenceKind::ComponentAcceptance)
        .into_iter()
        .filter(|receipt| {
            receipt.class == EvidenceClass::ProductionRecord
                && receipt.component_id.as_deref() == Some(candidate.component_id.as_str())
        })
        .collect::<Vec<_>>();
    let m4_lab_scope_ok = as_built
        .iter()
        .chain(component_function.iter())
        .all(|receipt| lab_scope_covers(receipt, &lab_scopes));
    let m4 = m3
        && !as_built.is_empty()
        && !component_function.is_empty()
        && !component_acceptance.is_empty()
        && m4_lab_scope_ok
        && component_function.iter().any(|function| {
            as_built.iter().any(|inspection| {
                inspection.geometry_digest == function.geometry_digest
                    && function.specimen_id == inspection.specimen_id
            }) && component_acceptance.iter().any(|acceptance| {
                acceptance.component_id == function.component_id
                    && acceptance.linked_evidence_ids.contains(&function.evidence_id)
                    && acceptance.linked_evidence_ids.iter().any(|id| {
                        as_built.iter().any(|inspection| inspection.evidence_id == *id)
                    })
            })
        });

    let cohorts = has(EvidenceKind::CohortSummary)
        .into_iter()
        .filter(|receipt| {
            receipt.class == EvidenceClass::ProductionRecord
                && receipt.cohort_id.is_some()
                && receipt.replication.is_some()
        })
        .collect::<Vec<_>>();
    let cohort_ok = cohorts.iter().any(|receipt| {
        let replication = receipt.replication.as_ref().expect("filtered replication");
        replication.independent_builds >= candidate.replication_policy.minimum_independent_builds
            && replication.distinct_material_lots
                >= candidate.replication_policy.minimum_distinct_material_lots
            && replication.cohort_policy_digest == candidate.replication_policy.policy_digest
    });
    let variability = has(EvidenceKind::VariabilityAnalysis)
        .into_iter()
        .any(|receipt| {
            receipt.class == EvidenceClass::PhysicalMeasurement
                || receipt.class == EvidenceClass::ProductionRecord
        });
    let durability = has(EvidenceKind::DurabilityTest)
        .into_iter()
        .any(|receipt| {
            receipt.class == EvidenceClass::PhysicalMeasurement
                && receipt.component_id.as_deref() == Some(candidate.component_id.as_str())
                && method_evidence_complete(receipt)
        });
    let site_quality = has(EvidenceKind::SiteQualitySystem)
        .into_iter()
        .any(|receipt| {
            receipt.class == EvidenceClass::ProductionRecord
                && receipt.standard_reference.as_deref() == Some("ISO/ASTM 52920:2023")
        });
    let equipment_iq = has(EvidenceKind::EquipmentIq);
    let equipment_oq = has(EvidenceKind::EquipmentOq);
    let equipment_pq = has(EvidenceKind::EquipmentPq);
    let equipment_ok = !candidate.pbf_lb_equipment_qualification_required
        || (equipment_iq.iter().any(|r| r.standard_reference.as_deref() == Some("ISO/ASTM TS 52930:2021"))
            && equipment_oq.iter().any(|r| r.standard_reference.as_deref() == Some("ISO/ASTM TS 52930:2021"))
            && equipment_pq.iter().any(|r| r.standard_reference.as_deref() == Some("ISO/ASTM TS 52930:2021")));
    let required_method_receipts = valid_receipts
        .iter()
        .filter(|receipt| is_physical_kind(receipt.kind))
        .copied()
        .collect::<Vec<_>>();
    let labs_cover_methods = !required_method_receipts.is_empty()
        && required_method_receipts
            .iter()
            .all(|receipt| lab_scope_covers(receipt, &lab_scopes));
    if !labs_cover_methods && !required_method_receipts.is_empty() {
        global_blockers.push(Blocker::MissingLaboratoryScopeEvidence);
    }
    let m5 = m4
        && cohort_ok
        && variability
        && durability
        && site_quality
        && equipment_ok
        && labs_cover_methods
        && !lab_scopes.is_empty();

    let findings = build_findings(
        &m1, m2, m3, m4, m5,
        !process_records.is_empty(),
        !microstructure.is_empty(),
        !links.is_empty(),
        !as_built.is_empty(),
        !component_function.is_empty(),
        !component_acceptance.is_empty(),
        !cohort_ok,
        !variability,
        !durability,
        !site_quality,
        !equipment_ok,
        !labs_cover_methods,
        &valid_receipts,
    );

    let derived_level = if m5 {
        MaturityLevel::M5
    } else if m4 {
        MaturityLevel::M4
    } else if m3 {
        MaturityLevel::M3
    } else if m2 {
        MaturityLevel::M2
    } else if !m1.is_empty() {
        MaturityLevel::M1
    } else if !m1.is_empty() {
        MaturityLevel::M1
    } else {
        MaturityLevel::Unestablished
    };

    let mut capability_vector = BTreeMap::new();
    for capability in [
        Capability::ConceptModel,
        Capability::CouponMaterialProperties,
        Capability::ProcessStructureProperty,
        Capability::AsBuiltGeometry,
        Capability::ComponentFunction,
        Capability::ComponentAcceptance,
        Capability::ReplicatedBuildLotCohort,
        Capability::VariabilityControl,
        Capability::Durability,
        Capability::ProductionSiteQuality,
        Capability::EquipmentIqOqPq,
        Capability::LaboratoryMethodScope,
    ] {
        let supported = match capability {
            Capability::ConceptModel => !m1.is_empty(),
            Capability::CouponMaterialProperties => m2,
            Capability::ProcessStructureProperty => m3,
            Capability::AsBuiltGeometry => !as_built.is_empty(),
            Capability::ComponentFunction => !component_function.is_empty(),
            Capability::ComponentAcceptance => !component_acceptance.is_empty(),
            Capability::ReplicatedBuildLotCohort => cohort_ok,
            Capability::VariabilityControl => variability,
            Capability::Durability => durability,
            Capability::ProductionSiteQuality => site_quality,
            Capability::EquipmentIqOqPq => equipment_ok,
            Capability::LaboratoryMethodScope => labs_cover_methods && !lab_scopes.is_empty(),
        };
        capability_vector.insert(capability, supported);
    }

    MaturityEvaluation {
        candidate_id: candidate.candidate_id.clone(),
        derived_level,
        findings,
        capability_vector,
        invalid_evidence_ids: invalid_evidence_ids.into_iter().collect(),
        global_blockers: global_blockers.deduped(),
        claim_ceiling: format!(
            "{:?}: evidence-maturity disposition only; no certification, process release or route superiority claim",
            derived_level
        ),
    }
}

trait Deduped {
    fn deduped(self) -> Self;
}

impl Deduped for Vec<Blocker> {
    fn deduped(mut self) -> Self {
        self.sort_by_key(|blocker| format!("{blocker:?}"));
        self.dedup();
        self
    }
}

fn validate_candidate(candidate: &Candidate) -> Vec<Blocker> {
    let mut blockers = Vec::new();
    if candidate.candidate_id.trim().is_empty()
        || candidate.component_id.trim().is_empty()
        || candidate.scope.application_id.trim().is_empty()
        || candidate.scope.route_family.trim().is_empty()
        || candidate.scope.route_revision.trim().is_empty()
        || candidate.scope.material_family.trim().is_empty()
        || candidate.scope.product_form.trim().is_empty()
    {
        blockers.push(Blocker::MissingCandidateIdentity);
    }
    for digest in [
        &candidate.scope.target_function_digest,
        &candidate.scope.measurement_setup_digest,
        &candidate.scope.acceptance_policy_digest,
        &candidate.replication_policy.policy_digest,
    ] {
        if !valid_sha256(digest) {
            blockers.push(Blocker::InvalidCandidateDigest);
        }
    }
    if candidate.replication_policy.policy_id.trim().is_empty()
        || candidate.replication_policy.minimum_independent_builds == 0
        || candidate.replication_policy.minimum_distinct_material_lots == 0
    {
        blockers.push(Blocker::InvalidReplicationPolicy);
    }
    blockers
}

fn validate_receipt(candidate: &Candidate, receipt: &EvidenceReceipt) -> Vec<Blocker> {
    let mut blockers = Vec::new();
    if receipt.evidence_id.trim().is_empty() {
        blockers.push(Blocker::MissingEvidenceIdentity);
    }
    if !valid_sha256(&receipt.digest) {
        blockers.push(Blocker::InvalidEvidenceDigest);
    }
    if receipt.scope != candidate.scope {
        blockers.push(Blocker::ScopeMismatch);
    }
    if is_physical_kind(receipt.kind) && receipt.class != EvidenceClass::PhysicalMeasurement {
        blockers.push(Blocker::InadmissibleEvidenceClass);
    }
    if matches!(
        receipt.kind,
        EvidenceKind::ProcessRecord
            | EvidenceKind::PspPLink
            | EvidenceKind::ComponentAcceptance
            | EvidenceKind::CohortSummary
            | EvidenceKind::VariabilityAnalysis
            | EvidenceKind::SiteQualitySystem
            | EvidenceKind::EquipmentIq
            | EvidenceKind::EquipmentOq
            | EvidenceKind::EquipmentPq
    ) && receipt.class != EvidenceClass::ProductionRecord
        && receipt.kind != EvidenceKind::VariabilityAnalysis
    {
        blockers.push(Blocker::InadmissibleEvidenceClass);
    }
    if receipt.kind == EvidenceKind::DesignModel
        && !matches!(
            receipt.class,
            EvidenceClass::PublishedLiterature
                | EvidenceClass::SyntheticControl
                | EvidenceClass::ProductionRecord
        )
    {
        blockers.push(Blocker::InadmissibleEvidenceClass);
    }
    if receipt.kind == EvidenceKind::LaboratoryMethodScope
        && receipt.class != EvidenceClass::ExternalAttestation
    {
        blockers.push(Blocker::InadmissibleEvidenceClass);
    }
    if matches!(
        receipt.kind,
        EvidenceKind::MaterialPropertyMeasurement
            | EvidenceKind::MicrostructureCharacterization
    ) {
        require(receipt.specimen_id.as_deref(), Blocker::MissingSpecimenBinding, &mut blockers);
        require(receipt.material_lot_id.as_deref(), Blocker::MissingLotBinding, &mut blockers);
        require(receipt.build_id.as_deref(), Blocker::MissingBuildBinding, &mut blockers);
        validate_method(receipt, &mut blockers);
    }
    if receipt.kind == EvidenceKind::ProcessRecord {
        require(receipt.build_id.as_deref(), Blocker::MissingBuildBinding, &mut blockers);
        require(receipt.material_lot_id.as_deref(), Blocker::MissingLotBinding, &mut blockers);
    }
    if receipt.kind == EvidenceKind::PspPLink && receipt.linked_evidence_ids.len() != 3 {
        blockers.push(Blocker::BrokenPspPLink);
    }
    if matches!(
        receipt.kind,
        EvidenceKind::AsBuiltInspection | EvidenceKind::ComponentFunctionalTest | EvidenceKind::ComponentAcceptance
    ) {
        require(
            receipt.component_id.as_deref(),
            Blocker::MissingComponentBinding,
            &mut blockers,
        );
        if receipt.component_id.as_deref() != Some(candidate.component_id.as_str()) {
            blockers.push(Blocker::ScopeMismatch);
        }
    }
    if matches!(
        receipt.kind,
        EvidenceKind::AsBuiltInspection
            | EvidenceKind::ComponentFunctionalTest
            | EvidenceKind::DurabilityTest
    ) {
        validate_method(receipt, &mut blockers);
    }
    if receipt.kind == EvidenceKind::AsBuiltInspection
        && receipt.geometry_digest.as_ref().is_none_or(|d| !valid_sha256(d))
    {
        blockers.push(Blocker::MissingGeometryBinding);
    }
    if receipt.kind == EvidenceKind::ComponentFunctionalTest
        && receipt.conditions_digest.as_ref().is_none_or(|d| !valid_sha256(d))
    {
        blockers.push(Blocker::MissingConditions);
    }
    if receipt.kind == EvidenceKind::DurabilityTest {
        require(
            receipt.component_id.as_deref(),
            Blocker::MissingComponentBinding,
            &mut blockers,
        );
    }
    if receipt.kind == EvidenceKind::CohortSummary {
        require(receipt.cohort_id.as_deref(), Blocker::MissingCohortIdentity, &mut blockers);
        if receipt.replication.as_ref().is_none_or(|r| !valid_sha256(&r.cohort_policy_digest)) {
            blockers.push(Blocker::WrongCohortPolicy);
        }
    }
    if receipt.kind == EvidenceKind::LaboratoryMethodScope {
        require(receipt.laboratory_id.as_deref(), Blocker::MissingLaboratoryBinding, &mut blockers);
        require(receipt.method_scope_id.as_deref(), Blocker::MissingMethodScope, &mut blockers);
        if receipt.standard_reference.as_deref() != Some("ISO/IEC 17025:2017") {
            blockers.push(Blocker::MissingLaboratoryScopeEvidence);
        }
    }
    blockers
}

fn validate_method(receipt: &EvidenceReceipt, blockers: &mut Vec<Blocker>) {
    require(receipt.method_id.as_deref(), Blocker::MissingMethodBinding, blockers);
    require(receipt.method_scope_id.as_deref(), Blocker::MissingMethodScope, blockers);
    require(receipt.laboratory_id.as_deref(), Blocker::MissingLaboratoryBinding, blockers);
    if receipt.calibration_record_id.as_deref().is_none_or(str::is_empty)
        || receipt.calibration_digest.as_ref().is_none_or(|d| !valid_sha256(d))
    {
        blockers.push(Blocker::MissingCalibration);
    }
    if receipt.uncertainty_record_id.as_deref().is_none_or(str::is_empty)
        || receipt.uncertainty_digest.as_ref().is_none_or(|d| !valid_sha256(d))
    {
        blockers.push(Blocker::MissingUncertainty);
    }
    if receipt.conditions_digest.as_ref().is_none_or(|d| !valid_sha256(d)) {
        blockers.push(Blocker::MissingConditions);
    }
}

fn lab_scope_covers(receipt: &EvidenceReceipt, lab_scopes: &[&EvidenceReceipt]) -> bool {
    receipt.method_scope_id.as_deref().is_some_and(|method_scope| {
        lab_scopes.iter().any(|scope| {
            scope.method_scope_id.as_deref() == Some(method_scope)
                && scope.laboratory_id == receipt.laboratory_id
        })
    })
}

fn method_evidence_complete(receipt: &EvidenceReceipt) -> bool {
    receipt.method_id.as_deref().is_some_and(|v| !v.trim().is_empty())
        && receipt.method_scope_id.as_deref().is_some_and(|v| !v.trim().is_empty())
        && receipt.laboratory_id.as_deref().is_some_and(|v| !v.trim().is_empty())
        && receipt.calibration_record_id.as_deref().is_some_and(|v| !v.trim().is_empty())
        && receipt.calibration_digest.as_ref().is_some_and(valid_sha256)
        && receipt.uncertainty_record_id.as_deref().is_some_and(|v| !v.trim().is_empty())
        && receipt.uncertainty_digest.as_ref().is_some_and(valid_sha256)
        && receipt.conditions_digest.as_ref().is_some_and(valid_sha256)
}

fn is_physical_kind(kind: EvidenceKind) -> bool {
    matches!(
        kind,
        EvidenceKind::MaterialPropertyMeasurement
            | EvidenceKind::MicrostructureCharacterization
            | EvidenceKind::AsBuiltInspection
            | EvidenceKind::ComponentFunctionalTest
            | EvidenceKind::DurabilityTest
    )
}

fn require(value: Option<&str>, blocker: Blocker, blockers: &mut Vec<Blocker>) {
    if value.is_none_or(|v| v.trim().is_empty()) {
        blockers.push(blocker);
    }
}

fn valid_sha256(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn build_findings(
    m1: &[&EvidenceReceipt],
    m2: bool,
    m3: bool,
    m4: bool,
    m5: bool,
    process: bool,
    microstructure: bool,
    link: bool,
    as_built: bool,
    function: bool,
    acceptance: bool,
    cohort_missing: bool,
    variability_missing: bool,
    durability_missing: bool,
    site_missing: bool,
    equipment_missing: bool,
    lab_missing: bool,
    valid_receipts: &[&EvidenceReceipt],
) -> Vec<LevelFinding> {
    let ids = |kinds: &[EvidenceKind]| {
        valid_receipts
            .iter()
            .filter(|receipt| kinds.contains(&receipt.kind))
            .map(|receipt| receipt.evidence_id.clone())
            .collect::<Vec<_>>()
    };
    vec![
        LevelFinding {
            level: MaturityLevel::M1,
            supported: !m1.is_empty(),
            required_capabilities: vec![Capability::ConceptModel],
            missing_capabilities: if m1.is_empty() { vec![Capability::ConceptModel] } else { vec![] },
            blockers: vec![],
            evidence_ids: m1.iter().map(|r| r.evidence_id.clone()).collect(),
        },
        LevelFinding {
            level: MaturityLevel::M2,
            supported: m2,
            required_capabilities: vec![Capability::CouponMaterialProperties, Capability::LaboratoryMethodScope],
            missing_capabilities: if m2 { vec![] } else { vec![Capability::CouponMaterialProperties] },
            blockers: if m2 { vec![] } else { vec![Blocker::LiteratureCannotPromotePhysicalLevel] },
            evidence_ids: ids(&[EvidenceKind::MaterialPropertyMeasurement, EvidenceKind::LaboratoryMethodScope]),
        },
        LevelFinding {
            level: MaturityLevel::M3,
            supported: m3,
            required_capabilities: vec![Capability::ProcessStructureProperty],
            missing_capabilities: if m3 { vec![] } else {
                let mut missing = Vec::new();
                if !process { missing.push(Capability::ProcessStructureProperty); }
                if !microstructure { missing.push(Capability::ProcessStructureProperty); }
                if !link { missing.push(Capability::ProcessStructureProperty); }
                if !m2 { missing.push(Capability::CouponMaterialProperties); }
                missing
            },
            blockers: if m3 { vec![] } else { vec![Blocker::MissingPspPLink] },
            evidence_ids: ids(&[EvidenceKind::ProcessRecord, EvidenceKind::MicrostructureCharacterization, EvidenceKind::PspPLink]),
        },
        LevelFinding {
            level: MaturityLevel::M4,
            supported: m4,
            required_capabilities: vec![Capability::AsBuiltGeometry, Capability::ComponentFunction, Capability::ComponentAcceptance],
            missing_capabilities: [
                (as_built, Capability::AsBuiltGeometry),
                (function, Capability::ComponentFunction),
                (acceptance, Capability::ComponentAcceptance),
            ].into_iter().filter_map(|(present, capability)| if present { None } else { Some(capability) }).collect(),
            blockers: if m4 { vec![] } else { vec![Blocker::MissingComponentAcceptance] },
            evidence_ids: ids(&[EvidenceKind::AsBuiltInspection, EvidenceKind::ComponentFunctionalTest, EvidenceKind::ComponentAcceptance]),
        },
        LevelFinding {
            level: MaturityLevel::M5,
            supported: m5,
            required_capabilities: vec![Capability::ReplicatedBuildLotCohort, Capability::VariabilityControl, Capability::Durability, Capability::ProductionSiteQuality, Capability::EquipmentIqOqPq, Capability::LaboratoryMethodScope],
            missing_capabilities: [
                (cohort_missing, Capability::ReplicatedBuildLotCohort),
                (variability_missing, Capability::VariabilityControl),
                (durability_missing, Capability::Durability),
                (site_missing, Capability::ProductionSiteQuality),
                (equipment_missing, Capability::EquipmentIqOqPq),
                (lab_missing, Capability::LaboratoryMethodScope),
            ].into_iter().filter_map(|(missing, capability)| if missing { Some(capability) } else { None }).collect(),
            blockers: [cohort_missing, variability_missing, durability_missing, site_missing, equipment_missing, lab_missing]
                .into_iter().filter(|missing| *missing).map(|_| Blocker::InsufficientIndependentBuilds).collect(),
            evidence_ids: ids(&[
                EvidenceKind::CohortSummary, EvidenceKind::VariabilityAnalysis,
                EvidenceKind::DurabilityTest, EvidenceKind::SiteQualitySystem,
                EvidenceKind::EquipmentIq, EvidenceKind::EquipmentOq, EvidenceKind::EquipmentPq,
                EvidenceKind::LaboratoryMethodScope,
            ]),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(ch: char) -> String {
        format!("sha256:{}", ch.to_string().repeat(64))
    }

    fn scope() -> CandidateScope {
        CandidateScope {
            application_id: "ENCODER-80MM-001".to_string(),
            route_family: "MEX".to_string(),
            route_revision: "MEX-v1".to_string(),
            material_family: "NdFeB".to_string(),
            product_form: "polymer-bonded".to_string(),
            target_function_digest: digest('a'),
            measurement_setup_digest: digest('b'),
            acceptance_policy_digest: digest('c'),
        }
    }

    fn candidate() -> Candidate {
        Candidate {
            candidate_id: "CANDIDATE-001".to_string(),
            scope: scope(),
            component_id: "COMP-001".to_string(),
            pbf_lb_equipment_qualification_required: false,
            replication_policy: ReplicationPolicy {
                minimum_independent_builds: 3,
                minimum_distinct_material_lots: 2,
                policy_id: "REPLICATION-v1".to_string(),
                policy_digest: digest('d'),
            },
        }
    }

    fn receipt(id: &str, kind: EvidenceKind, class: EvidenceClass) -> EvidenceReceipt {
        let method = is_physical_kind(kind);
        EvidenceReceipt {
            evidence_id: id.to_string(),
            digest: digest('e'),
            kind,
            class,
            scope: scope(),
            specimen_id: method.then(|| "SPEC-001".to_string()),
            material_lot_id: Some("LOT-001".to_string()),
            build_id: Some("BUILD-001".to_string()),
            component_id: matches!(kind, EvidenceKind::AsBuiltInspection | EvidenceKind::ComponentFunctionalTest | EvidenceKind::ComponentAcceptance | EvidenceKind::DurabilityTest)
                .then(|| "COMP-001".to_string()),
            cohort_id: (kind == EvidenceKind::CohortSummary).then(|| "COHORT-001".to_string()),
            method_id: method.then(|| format!("METHOD-{id}")),
            method_scope_id: method.then(|| format!("SCOPE-{id}")),
            laboratory_id: method.then(|| "LAB-001".to_string()),
            calibration_record_id: method.then(|| format!("CAL-{id}")),
            calibration_digest: method.then(|| digest('f')),
            uncertainty_record_id: method.then(|| format!("UNC-{id}")),
            uncertainty_digest: method.then(|| digest('1')),
            conditions_digest: method.then(|| digest('2')),
            geometry_digest: (kind == EvidenceKind::AsBuiltInspection || kind == EvidenceKind::ComponentFunctionalTest).then(|| digest('3')),
            standard_reference: (kind == EvidenceKind::LaboratoryMethodScope)
                .then(|| "ISO/IEC 17025:2017".to_string()),
            linked_evidence_ids: vec![],
            replication: (kind == EvidenceKind::CohortSummary).then(|| ReplicationEvidence {
                independent_builds: 3,
                distinct_material_lots: 2,
                cohort_policy_digest: digest('d'),
            }),
        }
    }

    fn m2_receipts() -> Vec<EvidenceReceipt> {
        let mut receipts = vec![
            receipt("MODEL-001", EvidenceKind::DesignModel, EvidenceClass::PublishedLiterature),
            receipt("PROPERTY-001", EvidenceKind::MaterialPropertyMeasurement, EvidenceClass::PhysicalMeasurement),
            receipt("LAB-SCOPE-001", EvidenceKind::LaboratoryMethodScope, EvidenceClass::ExternalAttestation),
        ];
        receipts.last_mut().unwrap().method_scope_id = Some("SCOPE-PROPERTY-001".to_string());
        receipts[1].method_scope_id = Some("SCOPE-PROPERTY-001".to_string());
        receipts[2].laboratory_id = Some("LAB-001".to_string());
        receipts
    }

    fn m3_receipts() -> Vec<EvidenceReceipt> {
        let mut receipts = m2_receipts();
        receipts.push(receipt("PROCESS-001", EvidenceKind::ProcessRecord, EvidenceClass::ProductionRecord));
        receipts.push(receipt("MICRO-001", EvidenceKind::MicrostructureCharacterization, EvidenceClass::PhysicalMeasurement));
        receipts.push(receipt("LAB-MICRO-001", EvidenceKind::LaboratoryMethodScope, EvidenceClass::ExternalAttestation));
        receipts.last_mut().unwrap().method_scope_id = Some("SCOPE-MICRO-001".to_string());
        receipts[4].method_scope_id = Some("SCOPE-MICRO-001".to_string());
        let mut link = receipt("PSPP-001", EvidenceKind::PspPLink, EvidenceClass::ProductionRecord);
        link.linked_evidence_ids = vec!["PROCESS-001".to_string(),"MICRO-001".to_string(),"PROPERTY-001".to_string()];
        receipts.push(link);
        receipts
    }

    fn m4_receipts() -> Vec<EvidenceReceipt> {
        let mut receipts = m3_receipts();
        receipts.push(receipt("GEOMETRY-001", EvidenceKind::AsBuiltInspection, EvidenceClass::PhysicalMeasurement));
        receipts.push(receipt("FUNCTION-001", EvidenceKind::ComponentFunctionalTest, EvidenceClass::PhysicalMeasurement));
        receipts.push(receipt("ACCEPT-001", EvidenceKind::ComponentAcceptance, EvidenceClass::ProductionRecord));
        receipts.last_mut().unwrap().linked_evidence_ids = vec!["FUNCTION-001".to_string(),"GEOMETRY-001".to_string()];
        receipts
    }

    fn m5_receipts() -> Vec<EvidenceReceipt> {
        let mut receipts = m4_receipts();
        receipts.push(receipt("COHORT-RECEIPT-001", EvidenceKind::CohortSummary, EvidenceClass::ProductionRecord));
        receipts.push(receipt("VARIABILITY-001", EvidenceKind::VariabilityAnalysis, EvidenceClass::ProductionRecord));
        receipts.push(receipt("DURABILITY-001", EvidenceKind::DurabilityTest, EvidenceClass::PhysicalMeasurement));
        receipts.push(receipt("SITE-QMS-001", EvidenceKind::SiteQualitySystem, EvidenceClass::ProductionRecord));
        receipts.last_mut().unwrap().standard_reference = Some("ISO/ASTM 52920:2023".to_string());
        let mut lab_scope = receipt("LAB-DURABILITY-001", EvidenceKind::LaboratoryMethodScope, EvidenceClass::ExternalAttestation);
        lab_scope.method_scope_id = Some("SCOPE-DURABILITY-001".to_string());
        lab_scope.laboratory_id = Some("LAB-001".to_string());
        receipts.push(lab_scope);
        receipts
    }

    #[test]
    fn no_physical_or_conceptual_evidence_is_unestablished() {
        let result = evaluate(&candidate(), &[]);
        assert_eq!(result.derived_level, MaturityLevel::Unestablished);
        assert!(!result.findings[0].supported);
    }

    #[test]
    fn m2_requires_matching_laboratory_method_scope_receipt() {
        let mut receipts = m2_receipts();
        receipts.retain(|receipt| receipt.kind != EvidenceKind::LaboratoryMethodScope);
        let result = evaluate(&candidate(), &receipts);
        assert_eq!(result.derived_level, MaturityLevel::M1);
        assert!(result.global_blockers.contains(&Blocker::MissingLaboratoryScopeEvidence));
    }

    #[test]
    fn laboratory_scope_from_another_lab_cannot_cover_measurement() {
        let mut receipts = m2_receipts();
        receipts[2].laboratory_id = Some("LAB-OTHER".to_string());
        let result = evaluate(&candidate(), &receipts);
        assert_eq!(result.derived_level, MaturityLevel::M1);
    }

    #[test]
    fn pspp_link_requires_same_specimen_lot_and_build_chain() {
        let mut receipts = m3_receipts();
        receipts[4].build_id = Some("BUILD-OTHER".to_string());
        let result = evaluate(&candidate(), &receipts);
        assert_eq!(result.derived_level, M2);
    }

    #[test]
    fn conceptual_design_supports_only_m1() {
        let result = evaluate(&candidate(), &[receipt("MODEL", EvidenceKind::DesignModel, EvidenceClass::PublishedLiterature)]);
        assert_eq!(result.derived_level, MaturityLevel::M1);
        assert!(result.findings[0].supported);
    }

    #[test]
    fn empty_evidence_has_no_maturity_level() {
        let result = evaluate(&candidate(), &[]);
        assert_eq!(result.derived_level, MaturityLevel::Unestablished);
        assert!(!result.findings[0].supported);
    }

    #[test]
    fn valid_coupon_method_and_lab_scope_support_m2() {
        let result = evaluate(&candidate(), &m2_receipts());
        assert_eq!(result.derived_level, MaturityLevel::M2);
    }

    #[test]
    fn literature_cannot_promote_a_material_coupon_to_m2() {
        let mut receipts = m2_receipts();
        receipts[1].class = EvidenceClass::PublishedLiterature;
        let result = evaluate(&candidate(), &receipts);
        assert_eq!(result.derived_level, M1);
        assert!(result.invalid_evidence_ids.contains(&"PROPERTY-001".to_string()));
    }

    #[test]
    fn synthetic_control_cannot_promote_a_material_coupon_to_m2() {
        let mut receipts = m2_receipts();
        receipts[1].class = EvidenceClass::SyntheticControl;
        let result = evaluate(&candidate(), &receipts);
        assert_eq!(result.derived_level, M1);
    }

    #[test]
    fn missing_calibration_or_uncertainty_blocks_m2() {
        let mut receipts = m2_receipts();
        receipts[1].calibration_digest = None;
        let result = evaluate(&candidate(), &receipts);
        assert_eq!(result.derived_level, M1);
    }

    #[test]
    fn wrong_lot_on_property_measurement_blocks_m2() {
        let mut receipts = m2_receipts();
        receipts[1].material_lot_id = Some("LOT-OTHER".to_string());
        let result = evaluate(&candidate(), &receipts);
        assert_eq!(result.derived_level, M1);
        assert!(result.global_blockers.contains(&Blocker::ScopeMismatch));
    }

    #[test]
    fn valid_pspp_links_support_m3() {
        let result = evaluate(&candidate(), &m3_receipts());
        assert_eq!(result.derived_level, M3);
    }

    #[test]
    fn broken_pspp_link_blocks_m3() {
        let mut receipts = m3_receipts();
        receipts.last_mut().unwrap().linked_evidence_ids.pop();
        let result = evaluate(&candidate(), &receipts);
        assert_eq!(result.derived_level, M2);
    }

    #[test]
    fn machine_qualification_does_not_substitute_for_material_properties() {
        let mut receipts = vec![receipt("MODEL", EvidenceKind::DesignModel, EvidenceClass::PublishedLiterature)];
        for (id,kind) in [("IQ",EvidenceKind::EquipmentIq),("OQ",EvidenceKind::EquipmentOq),("PQ",EvidenceKind::EquipmentPq)] {
            let mut r=receipt(id,kind,EvidenceClass::ProductionRecord);
            r.standard_reference=Some("ISO/ASTM TS 52930:2021".to_string());
            receipts.push(r);
        }
        let result=evaluate(&candidate(),&receipts);
        assert_eq!(result.derived_level,M1);
    }

    #[test]
    fn valid_component_test_requires_geometry_function_and_acceptance() {
        let result = evaluate(&candidate(), &m4_receipts());
        assert_eq!(result.derived_level, M4);
    }

    #[test]
    fn component_function_without_as_built_geometry_is_not_m4() {
        let mut receipts = m4_receipts();
        receipts.retain(|receipt| receipt.kind != EvidenceKind::AsBuiltInspection);
        let result = evaluate(&candidate(), &receipts);
        assert_eq!(result.derived_level, M3);
    }

    #[test]
    fn component_function_without_acceptance_is_not_m4() {
        let mut receipts = m4_receipts();
        receipts.retain(|receipt| receipt.kind != EvidenceKind::ComponentAcceptance);
        let result = evaluate(&candidate(), &receipts);
        assert_eq!(result.derived_level, M3);
    }

    #[test]
    fn valid_m5_requires_all_independent_layers() {
        let mut candidate = candidate();
        candidate.pbf_lb_equipment_qualification_required = false;
        let mut receipts = m5_receipts();
        receipts.push(receipt("LAB-FUNCTION-001", EvidenceKind::LaboratoryMethodScope, EvidenceClass::ExternalAttestation));
        receipts.last_mut().unwrap().method_scope_id = Some("SCOPE-FUNCTION-001".to_string());
        receipts.push(receipt("LAB-GEOMETRY-001", EvidenceKind::LaboratoryMethodScope, EvidenceClass::ExternalAttestation));
        receipts.last_mut().unwrap().method_scope_id = Some("SCOPE-GEOMETRY-001".to_string());
        let result = evaluate(&candidate, &receipts);
        assert_eq!(result.derived_level, M5);
    }

    #[test]
    fn m5_without_durability_stays_at_m4() {
        let mut receipts = m5_receipts();
        receipts.retain(|receipt| receipt.kind != EvidenceKind::DurabilityTest);
        let result = evaluate(&candidate(), &receipts);
        assert_eq!(result.derived_level, M4);
    }

    #[test]
    fn pbf_lb_m5_requires_separate_equipment_iq_oq_and_pq() {
        let mut candidate = candidate();
        candidate.pbf_lb_equipment_qualification_required = true;
        let mut receipts = m5_receipts();
        for (id, kind) in [("IQ", EvidenceKind::EquipmentIq),("OQ", EvidenceKind::EquipmentOq),("PQ", EvidenceKind::EquipmentPq)] {
            let mut r = receipt(id, kind, EvidenceClass::ProductionRecord);
            r.standard_reference = Some("ISO/ASTM TS 52930:2021".to_string());
            receipts.push(r);
        }
        receipts.push(receipt("LAB-FUNCTION-001", EvidenceKind::LaboratoryMethodScope, EvidenceClass::ExternalAttestation));
        receipts.last_mut().unwrap().method_scope_id = Some("SCOPE-FUNCTION-001".to_string());
        receipts.push(receipt("LAB-GEOMETRY-001", EvidenceKind::LaboratoryMethodScope, EvidenceClass::ExternalAttestation));
        receipts.last_mut().unwrap().method_scope_id = Some("SCOPE-GEOMETRY-001".to_string());
        let result = evaluate(&candidate, &receipts);
        assert_eq!(result.derived_level, M5);
    }

    #[test]
    fn insufficient_build_or_lot_replication_blocks_m5() {
        let mut receipts = m5_receipts();
        let cohort = receipts.iter_mut().find(|r| r.kind == EvidenceKind::CohortSummary).unwrap();
        cohort.replication.as_mut().unwrap().independent_builds = 1;
        let result = evaluate(&candidate(), &receipts);
        assert_eq!(result.derived_level, M4);
    }

    #[test]
    fn duplicate_evidence_ids_are_invalid_not_extra_support() {
        let mut receipts = m2_receipts();
        receipts.push(receipts[1].clone());
        let result = evaluate(&candidate(), &receipts);
        assert_eq!(result.derived_level, M1);
        assert!(result.global_blockers.contains(&Blocker::DuplicateEvidenceId));
    }

    #[test]
    fn wrong_application_target_scope_is_rejected() {
        let mut receipts = m2_receipts();
        receipts[1].scope.target_function_digest = digest('f');
        let result = evaluate(&candidate(), &receipts);
        assert_eq!(result.derived_level, M1);
    }

    #[test]
    fn missing_variability_and_site_quality_keep_candidate_at_m4() {
        let mut receipts = m5_receipts();
        receipts.retain(|r| !matches!(r.kind, EvidenceKind::VariabilityAnalysis | EvidenceKind::SiteQualitySystem));
        let result = evaluate(&candidate(), &receipts);
        assert_eq!(result.derived_level, M4);
    }

    #[test]
    fn m5_without_lab_method_scope_stays_at_m4() {
        let mut receipts = m5_receipts();
        receipts.retain(|r| r.kind != EvidenceKind::LaboratoryMethodScope);
        let result = evaluate(&candidate(), &receipts);
        assert_eq!(result.derived_level, M4);
    }
}
