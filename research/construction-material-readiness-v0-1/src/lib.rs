//! Construction material-lot readiness projection.
//!
//! This crate consumes owner-qualified evidence projections from the authoritative
//! provenance, logistics, inventory, component-qualification and design owners.
//! It performs deterministic compatibility and kit-completeness checks only.
//! It never mutates stock, reservations, custody, approvals or acceptance records.
//!
//! It is not an inventory system, warehouse-management system, product-certification
//! authority, procurement optimizer, structural-code evaluator or installation permit.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkPackageBinding {
    pub project_id: String,
    pub floor_id: String,
    pub work_package_id: String,
    pub task_id: String,
    pub design_revision_digest: String,
    pub site_epoch: String,
    pub requirement_revision_digest: String,
    pub receiving_entity_id: String,
    pub target_location_id: String,
    pub required_window_start_ms: u64,
    pub required_window_end_ms: u64,
}

impl WorkPackageBinding {
    fn validate(&self) -> Result<(), ReadinessError> {
        for (field, value) in [
            ("project_id", self.project_id.as_str()),
            ("floor_id", self.floor_id.as_str()),
            ("work_package_id", self.work_package_id.as_str()),
            ("task_id", self.task_id.as_str()),
            (
                "design_revision_digest",
                self.design_revision_digest.as_str(),
            ),
            ("site_epoch", self.site_epoch.as_str()),
            (
                "requirement_revision_digest",
                self.requirement_revision_digest.as_str(),
            ),
            ("receiving_entity_id", self.receiving_entity_id.as_str()),
            ("target_location_id", self.target_location_id.as_str()),
        ] {
            require_non_empty(field, value)?;
        }
        if self.required_window_start_ms >= self.required_window_end_ms {
            return Err(ReadinessError::InvalidWorkWindow);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApprovedSubstitution {
    pub project_id: String,
    pub requirement_revision_digest: String,
    pub material_class_id: String,
    pub grade_id: String,
    pub approval_ref: String,
    pub authority_ref: String,
    pub design_revision_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaterialRequirement {
    pub line_id: String,
    pub material_class_id: String,
    pub grade_id: String,
    pub unit_code: String,
    /// Quantity is fixed-point integer in the declared unit; no implicit conversions.
    pub required_quantity_milli: u64,
    pub allowed_overage_milli: u64,
    pub specification_ref: String,
    pub approved_substitutions: Vec<ApprovedSubstitution>,
}

impl MaterialRequirement {
    fn validate(&self) -> Result<(), ReadinessError> {
        for (field, value) in [
            ("line_id", self.line_id.as_str()),
            ("material_class_id", self.material_class_id.as_str()),
            ("grade_id", self.grade_id.as_str()),
            ("unit_code", self.unit_code.as_str()),
            ("specification_ref", self.specification_ref.as_str()),
        ] {
            require_non_empty(field, value)?;
        }
        if self.required_quantity_milli == 0 {
            return Err(ReadinessError::ZeroRequiredQuantity(self.line_id.clone()));
        }
        let mut substitutions = BTreeSet::new();
        for substitute in &self.approved_substitutions {
            require_non_empty("substitution_project_id", &substitute.project_id)?;
            require_non_empty(
                "substitution_requirement_revision_digest",
                &substitute.requirement_revision_digest,
            )?;
            require_non_empty(
                "substitute_material_class_id",
                &substitute.material_class_id,
            )?;
            require_non_empty("substitute_grade_id", &substitute.grade_id)?;
            require_non_empty("substitution_approval_ref", &substitute.approval_ref)?;
            require_non_empty("substitution_authority_ref", &substitute.authority_ref)?;
            require_non_empty(
                "substitution_design_revision_digest",
                &substitute.design_revision_digest,
            )?;
            if !substitutions.insert((
                substitute.material_class_id.as_str(),
                substitute.grade_id.as_str(),
            )) {
                return Err(ReadinessError::DuplicateApprovedSubstitution(
                    self.line_id.clone(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceDisposition {
    Qualified,
    AssertionOnly,
    Unknown,
    Rejected,
    Revoked,
    Disputed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceReceipt {
    pub receipt_ref: String,
    pub source_profile_id: String,
    pub source_schema_version: String,
    pub project_id: String,
    pub lot_id: String,
    pub design_revision_digest: String,
    pub requirement_revision_digest: String,
    pub authority_ref: String,
    pub disposition: EvidenceDisposition,
    pub observed_at_ms: u64,
    pub valid_until_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogisticsDisposition {
    DispatchOnly,
    InTransit,
    DeliveredAwaitingAcceptance,
    RecipientAccepted,
    Damaged,
    Rejected,
    Quarantined,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogisticsReceipt {
    pub receipt_ref: String,
    pub source_profile_id: String,
    pub source_schema_version: String,
    pub project_id: String,
    pub lot_id: String,
    pub recipient_id: String,
    pub delivery_location_id: String,
    pub disposition: LogisticsDisposition,
    pub observed_at_ms: u64,
    pub valid_until_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InventoryDisposition {
    ReservedForWorkPackage,
    AvailableNotReserved,
    Stale,
    Conflict,
    DamagedOrQuarantined,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InventoryReservationReceipt {
    pub receipt_ref: String,
    pub source_profile_id: String,
    pub source_schema_version: String,
    pub project_id: String,
    pub lot_id: String,
    pub work_package_id: String,
    pub design_revision_digest: String,
    pub site_epoch: String,
    pub location_id: String,
    pub disposition: InventoryDisposition,
    pub reserved_quantity_milli: u64,
    pub window_start_ms: u64,
    pub window_end_ms: u64,
    pub observed_at_ms: u64,
    pub valid_until_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaterialLotProjection {
    pub lot_id: String,
    pub material_class_id: String,
    pub grade_id: String,
    pub unit_code: String,
    /// Provenance receipt from Mycelix #5149; not verified by this crate.
    pub provenance: EvidenceReceipt,
    /// Recipient delivery / custody receipt from Mycelix #4901; not verified here.
    pub logistics: LogisticsReceipt,
    /// Component/material qualification evidence from Symtropy #3331; not verified here.
    pub quality: EvidenceReceipt,
    /// Source-owned reservation from the inventory/logistics authority; not created here.
    pub inventory: InventoryReservationReceipt,
}

impl MaterialLotProjection {
    fn validate_non_empty(&self) -> Result<(), ReadinessError> {
        for (field, value) in [
            ("lot_id", self.lot_id.as_str()),
            ("material_class_id", self.material_class_id.as_str()),
            ("grade_id", self.grade_id.as_str()),
            ("unit_code", self.unit_code.as_str()),
        ] {
            require_non_empty(field, value)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LotAllocation {
    pub line_id: String,
    pub lot_id: String,
    pub quantity_milli: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaterialKit {
    pub binding: WorkPackageBinding,
    pub requirements: Vec<MaterialRequirement>,
    pub allocations: Vec<LotAllocation>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReadinessDisposition {
    ReadyForWorkPackage,
    AwaitingDelivery,
    ReceivingReviewRequired,
    InsufficientQuantity,
    SubstitutionApprovalRequired,
    HeldUnknown,
    Rejected,
    Superseded,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum BlockerCode {
    MissingAllocation,
    MissingLotProjection,
    InvalidAllocationQuantity,
    InsufficientReservedQuantity,
    ExcessAllocation,
    MaterialClassMismatch,
    GradeMismatch,
    UnitMismatch,
    SubstitutionApprovalMissing,
    SubstitutionApprovalStale,
    EvidenceReferenceMissing,
    EvidenceScopeMismatch,
    EvidenceNotCurrent,
    EvidenceAssertionOnly,
    EvidenceUnknown,
    EvidenceDisputed,
    EvidenceRejected,
    EvidenceRevoked,
    DeliveryNotReceived,
    RecipientAcceptanceMissing,
    WrongReceivingEntity,
    WrongDeliveryLocation,
    MaterialDamaged,
    MaterialQuarantined,
    InventoryNotReserved,
    InventoryReservationStale,
    InventoryReservationConflict,
    InventoryReservationScopeMismatch,
    InventoryReservationEpochMismatch,
    InventoryReservationWindowMismatch,
    RequirementRevisionSuperseded,
    DesignRevisionSuperseded,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadinessBlocker {
    pub code: BlockerCode,
    pub line_id: String,
    pub lot_id: Option<String>,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaterialLineAssessment {
    pub line_id: String,
    pub required_quantity_milli: u64,
    pub allocated_quantity_milli: u64,
    pub disposition: ReadinessDisposition,
    pub blockers: Vec<ReadinessBlocker>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaterialKitAssessment {
    pub project_id: String,
    pub floor_id: String,
    pub work_package_id: String,
    pub task_id: String,
    pub design_revision_digest: String,
    pub assessed_at_ms: u64,
    pub disposition: ReadinessDisposition,
    pub lines: Vec<MaterialLineAssessment>,
    pub source_receipt_refs: Vec<String>,
    /// Always false: this adapter is a pure read-only projection.
    pub source_records_mutated: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReadinessError {
    EmptyField(&'static str),
    InvalidWorkWindow,
    EmptyKit,
    ZeroRequiredQuantity(String),
    DuplicateRequirementLine(String),
    DuplicateLotProjection(String),
    DuplicateApprovedSubstitution(String),
    DuplicateAllocation(String),
    UnknownAllocationLine(String),
    QuantityOverflow(String),
}

pub fn assess_material_kit(
    kit: &MaterialKit,
    lot_projections: &[MaterialLotProjection],
    now_ms: u64,
) -> Result<MaterialKitAssessment, ReadinessError> {
    kit.binding.validate()?;
    if kit.requirements.is_empty() {
        return Err(ReadinessError::EmptyKit);
    }

    let mut requirement_by_id = BTreeMap::new();
    for requirement in &kit.requirements {
        requirement.validate()?;
        if requirement_by_id
            .insert(requirement.line_id.as_str(), requirement)
            .is_some()
        {
            return Err(ReadinessError::DuplicateRequirementLine(
                requirement.line_id.clone(),
            ));
        }
    }

    let mut lots_by_id = BTreeMap::new();
    for lot in lot_projections {
        lot.validate_non_empty()?;
        if lots_by_id.insert(lot.lot_id.as_str(), lot).is_some() {
            return Err(ReadinessError::DuplicateLotProjection(lot.lot_id.clone()));
        }
    }

    let mut allocations_by_line: BTreeMap<&str, Vec<&LotAllocation>> = BTreeMap::new();
    let mut unique_allocations = BTreeSet::new();
    let mut used_quantity_by_lot: BTreeMap<&str, u64> = BTreeMap::new();
    for allocation in &kit.allocations {
        require_non_empty("allocation_line_id", &allocation.line_id)?;
        require_non_empty("allocation_lot_id", &allocation.lot_id)?;
        if allocation.quantity_milli == 0 {
            return Err(ReadinessError::EmptyField("allocation_quantity_milli"));
        }
        if !requirement_by_id.contains_key(allocation.line_id.as_str()) {
            return Err(ReadinessError::UnknownAllocationLine(
                allocation.line_id.clone(),
            ));
        }
        if !unique_allocations.insert((allocation.line_id.as_str(), allocation.lot_id.as_str())) {
            return Err(ReadinessError::DuplicateAllocation(format!(
                "{}:{}",
                allocation.line_id, allocation.lot_id
            )));
        }
        let total = used_quantity_by_lot
            .get(allocation.lot_id.as_str())
            .copied()
            .unwrap_or(0)
            .checked_add(allocation.quantity_milli)
            .ok_or_else(|| ReadinessError::QuantityOverflow(allocation.lot_id.clone()))?;
        used_quantity_by_lot.insert(allocation.lot_id.as_str(), total);
        allocations_by_line
            .entry(allocation.line_id.as_str())
            .or_default()
            .push(allocation);
    }

    let mut line_assessments = Vec::with_capacity(kit.requirements.len());
    let mut source_receipt_refs = BTreeSet::new();

    for requirement in &kit.requirements {
        let allocations = allocations_by_line
            .get(requirement.line_id.as_str())
            .cloned()
            .unwrap_or_default();
        let mut blockers = Vec::new();
        let mut allocated_quantity = 0_u64;
        let mut line_dispositions = Vec::new();

        if allocations.is_empty() {
            push_blocker(
                &mut blockers,
                BlockerCode::MissingAllocation,
                &requirement.line_id,
                None,
                "No lot allocation was supplied for this required material line",
                ReadinessDisposition::HeldUnknown,
                &mut line_dispositions,
            );
        }

        for allocation in allocations {
            allocated_quantity = allocated_quantity
                .checked_add(allocation.quantity_milli)
                .ok_or_else(|| ReadinessError::QuantityOverflow(requirement.line_id.clone()))?;

            let Some(lot) = lots_by_id.get(allocation.lot_id.as_str()).copied() else {
                push_blocker(
                    &mut blockers,
                    BlockerCode::MissingLotProjection,
                    &requirement.line_id,
                    Some(&allocation.lot_id),
                    "The source owners supplied no projection for this exact physical lot",
                    ReadinessDisposition::HeldUnknown,
                    &mut line_dispositions,
                );
                continue;
            };

            for receipt_ref in [
                lot.provenance.receipt_ref.as_str(),
                lot.logistics.receipt_ref.as_str(),
                lot.quality.receipt_ref.as_str(),
                lot.inventory.receipt_ref.as_str(),
            ] {
                if !receipt_ref.trim().is_empty() {
                    source_receipt_refs.insert(receipt_ref.to_owned());
                }
            }

            assess_material_compatibility(
                requirement,
                lot,
                &kit.binding,
                now_ms,
                &mut blockers,
                &mut line_dispositions,
            );
        }

        if allocated_quantity < requirement.required_quantity_milli {
            push_blocker(
                &mut blockers,
                BlockerCode::InsufficientReservedQuantity,
                &requirement.line_id,
                None,
                "Allocated quantity is below the required functional quantity",
                ReadinessDisposition::InsufficientQuantity,
                &mut line_dispositions,
            );
        }
        let permitted_total = requirement
            .required_quantity_milli
            .checked_add(requirement.allowed_overage_milli)
            .ok_or_else(|| ReadinessError::QuantityOverflow(requirement.line_id.clone()))?;
        if allocated_quantity > permitted_total {
            push_blocker(
                &mut blockers,
                BlockerCode::ExcessAllocation,
                &requirement.line_id,
                None,
                "Allocated quantity exceeds the requirement and explicitly permitted overage",
                ReadinessDisposition::HeldUnknown,
                &mut line_dispositions,
            );
        }

        line_assessments.push(MaterialLineAssessment {
            line_id: requirement.line_id.clone(),
            required_quantity_milli: requirement.required_quantity_milli,
            allocated_quantity_milli: allocated_quantity,
            disposition: worst_disposition(&line_dispositions),
            blockers,
        });
    }

    for (lot_id, used_quantity) in &used_quantity_by_lot {
        let Some(lot) = lots_by_id.get(lot_id).copied() else {
            continue;
        };
        if *used_quantity > lot.inventory.reserved_quantity_milli {
            let target_line = kit
                .allocations
                .iter()
                .find(|allocation| allocation.lot_id == *lot_id)
                .map(|allocation| allocation.line_id.as_str())
                .unwrap_or("kit");
            if let Some(line) = line_assessments
                .iter_mut()
                .find(|line| line.line_id == target_line)
            {
                let blocker = ReadinessBlocker {
                    code: BlockerCode::InsufficientReservedQuantity,
                    line_id: target_line.to_owned(),
                    lot_id: Some((*lot_id).to_owned()),
                    detail: "The same lot is allocated across kit lines beyond its single source-owned reservation quantity".to_owned(),
                };
                line.blockers.push(blocker);
                line.disposition =
                    worse(line.disposition, ReadinessDisposition::InsufficientQuantity);
            }
        }
    }

    let disposition = worst_disposition(
        &line_assessments
            .iter()
            .map(|line| line.disposition)
            .collect::<Vec<_>>(),
    );
    Ok(MaterialKitAssessment {
        project_id: kit.binding.project_id.clone(),
        floor_id: kit.binding.floor_id.clone(),
        work_package_id: kit.binding.work_package_id.clone(),
        task_id: kit.binding.task_id.clone(),
        design_revision_digest: kit.binding.design_revision_digest.clone(),
        assessed_at_ms: now_ms,
        disposition,
        lines: line_assessments,
        source_receipt_refs: source_receipt_refs.into_iter().collect(),
        source_records_mutated: false,
    })
}

fn assess_material_compatibility(
    requirement: &MaterialRequirement,
    lot: &MaterialLotProjection,
    binding: &WorkPackageBinding,
    now_ms: u64,
    blockers: &mut Vec<ReadinessBlocker>,
    dispositions: &mut Vec<ReadinessDisposition>,
) {
    if lot.unit_code != requirement.unit_code {
        push_blocker(
            blockers,
            BlockerCode::UnitMismatch,
            &requirement.line_id,
            Some(&lot.lot_id),
            "No approved unit conversion profile is bound; implicit conversion is forbidden",
            ReadinessDisposition::HeldUnknown,
            dispositions,
        );
    }

    if lot.material_class_id != requirement.material_class_id
        || lot.grade_id != requirement.grade_id
    {
        let approved = requirement
            .approved_substitutions
            .iter()
            .find(|substitute| {
                substitute.material_class_id == lot.material_class_id
                    && substitute.grade_id == lot.grade_id
            });
        match approved {
            Some(substitute)
                if substitute.project_id == binding.project_id
                    && substitute.requirement_revision_digest
                        == binding.requirement_revision_digest
                    && substitute.design_revision_digest == binding.design_revision_digest =>
            {
                // An explicit, project/spec/design-bound approval exists; source evidence is
                // still required independently.
            }
            Some(_) => push_blocker(
                blockers,
                BlockerCode::SubstitutionApprovalStale,
                &requirement.line_id,
                Some(&lot.lot_id),
                "Substitution approval is bound to a different design revision",
                ReadinessDisposition::SubstitutionApprovalRequired,
                dispositions,
            ),
            None => {
                let code = if lot.material_class_id != requirement.material_class_id {
                    BlockerCode::MaterialClassMismatch
                } else {
                    BlockerCode::GradeMismatch
                };
                push_blocker(
                    blockers,
                    code,
                    &requirement.line_id,
                    Some(&lot.lot_id),
                    "Material class/grade differs from the requirement and no exact approved substitution applies",
                    ReadinessDisposition::SubstitutionApprovalRequired,
                    dispositions,
                );
            }
        }
    }

    assess_evidence(
        &lot.provenance,
        &lot.lot_id,
        requirement,
        binding,
        now_ms,
        blockers,
        dispositions,
    );
    assess_evidence(
        &lot.quality,
        &lot.lot_id,
        requirement,
        binding,
        now_ms,
        blockers,
        dispositions,
    );
    assess_logistics(
        &lot.logistics,
        lot,
        requirement,
        binding,
        now_ms,
        blockers,
        dispositions,
    );
    assess_inventory(
        &lot.inventory,
        lot,
        requirement,
        binding,
        now_ms,
        blockers,
        dispositions,
    );
}

fn assess_evidence(
    receipt: &EvidenceReceipt,
    lot_id: &str,
    requirement: &MaterialRequirement,
    binding: &WorkPackageBinding,
    now_ms: u64,
    blockers: &mut Vec<ReadinessBlocker>,
    dispositions: &mut Vec<ReadinessDisposition>,
) {
    if receipt.receipt_ref.trim().is_empty()
        || receipt.source_profile_id.trim().is_empty()
        || receipt.source_schema_version.trim().is_empty()
        || receipt.authority_ref.trim().is_empty()
    {
        push_blocker(
            blockers,
            BlockerCode::EvidenceReferenceMissing,
            &requirement.line_id,
            Some(lot_id),
            "Evidence source, schema, receipt or accountable authority reference is missing",
            ReadinessDisposition::HeldUnknown,
            dispositions,
        );
        return;
    }
    if receipt.lot_id != lot_id || receipt.project_id != binding.project_id {
        push_blocker(
            blockers,
            BlockerCode::EvidenceScopeMismatch,
            &requirement.line_id,
            Some(lot_id),
            "Evidence does not identify this exact physical lot and project",
            ReadinessDisposition::HeldUnknown,
            dispositions,
        );
        return;
    }
    if receipt.design_revision_digest != binding.design_revision_digest
        || receipt.requirement_revision_digest != binding.requirement_revision_digest
    {
        let code = if receipt.design_revision_digest != binding.design_revision_digest {
            BlockerCode::DesignRevisionSuperseded
        } else {
            BlockerCode::RequirementRevisionSuperseded
        };
        push_blocker(
            blockers,
            code,
            &requirement.line_id,
            Some(lot_id),
            "Evidence is bound to a superseded design or material requirement revision",
            ReadinessDisposition::Superseded,
            dispositions,
        );
        return;
    }
    if !is_current(now_ms, receipt.observed_at_ms, receipt.valid_until_ms) {
        push_blocker(
            blockers,
            BlockerCode::EvidenceNotCurrent,
            &requirement.line_id,
            Some(lot_id),
            "Evidence is not current at the readiness assessment time",
            ReadinessDisposition::HeldUnknown,
            dispositions,
        );
        return;
    }
    match receipt.disposition {
        EvidenceDisposition::Qualified => {}
        EvidenceDisposition::AssertionOnly => push_blocker(
            blockers,
            BlockerCode::EvidenceAssertionOnly,
            &requirement.line_id,
            Some(lot_id),
            "Supplier/source assertion is not independently qualified evidence",
            ReadinessDisposition::HeldUnknown,
            dispositions,
        ),
        EvidenceDisposition::Unknown => push_blocker(
            blockers,
            BlockerCode::EvidenceUnknown,
            &requirement.line_id,
            Some(lot_id),
            "Evidence owner returned Unknown",
            ReadinessDisposition::HeldUnknown,
            dispositions,
        ),
        EvidenceDisposition::Disputed => push_blocker(
            blockers,
            BlockerCode::EvidenceDisputed,
            &requirement.line_id,
            Some(lot_id),
            "Evidence or custody is disputed",
            ReadinessDisposition::HeldUnknown,
            dispositions,
        ),
        EvidenceDisposition::Rejected => push_blocker(
            blockers,
            BlockerCode::EvidenceRejected,
            &requirement.line_id,
            Some(lot_id),
            "Evidence owner rejected the lot or evidence",
            ReadinessDisposition::Rejected,
            dispositions,
        ),
        EvidenceDisposition::Revoked => push_blocker(
            blockers,
            BlockerCode::EvidenceRevoked,
            &requirement.line_id,
            Some(lot_id),
            "Previously qualified evidence has been revoked",
            ReadinessDisposition::Rejected,
            dispositions,
        ),
    }
}

fn assess_logistics(
    receipt: &LogisticsReceipt,
    lot: &MaterialLotProjection,
    requirement: &MaterialRequirement,
    binding: &WorkPackageBinding,
    now_ms: u64,
    blockers: &mut Vec<ReadinessBlocker>,
    dispositions: &mut Vec<ReadinessDisposition>,
) {
    if receipt.receipt_ref.trim().is_empty()
        || receipt.source_profile_id.trim().is_empty()
        || receipt.source_schema_version.trim().is_empty()
    {
        push_blocker(
            blockers,
            BlockerCode::EvidenceReferenceMissing,
            &requirement.line_id,
            Some(&lot.lot_id),
            "Logistics source, schema or receipt reference is missing",
            ReadinessDisposition::HeldUnknown,
            dispositions,
        );
        return;
    }
    if receipt.project_id != binding.project_id || receipt.lot_id != lot.lot_id {
        push_blocker(
            blockers,
            BlockerCode::EvidenceScopeMismatch,
            &requirement.line_id,
            Some(&lot.lot_id),
            "Logistics receipt does not identify this exact lot and project",
            ReadinessDisposition::HeldUnknown,
            dispositions,
        );
        return;
    }
    if !is_current(now_ms, receipt.observed_at_ms, receipt.valid_until_ms) {
        push_blocker(
            blockers,
            BlockerCode::EvidenceNotCurrent,
            &requirement.line_id,
            Some(&lot.lot_id),
            "Logistics receipt is stale or future-dated",
            ReadinessDisposition::HeldUnknown,
            dispositions,
        );
        return;
    }

    match receipt.disposition {
        LogisticsDisposition::DispatchOnly | LogisticsDisposition::InTransit => push_blocker(
            blockers,
            BlockerCode::DeliveryNotReceived,
            &requirement.line_id,
            Some(&lot.lot_id),
            "Dispatch/in-transit is not recipient acceptance",
            ReadinessDisposition::AwaitingDelivery,
            dispositions,
        ),
        LogisticsDisposition::DeliveredAwaitingAcceptance => push_blocker(
            blockers,
            BlockerCode::RecipientAcceptanceMissing,
            &requirement.line_id,
            Some(&lot.lot_id),
            "Delivery is recorded but recipient acceptance/condition review is not complete",
            ReadinessDisposition::ReceivingReviewRequired,
            dispositions,
        ),
        LogisticsDisposition::RecipientAccepted => {
            if receipt.recipient_id != binding.receiving_entity_id {
                push_blocker(
                    blockers,
                    BlockerCode::WrongReceivingEntity,
                    &requirement.line_id,
                    Some(&lot.lot_id),
                    "Receipt is acknowledged by a different receiving entity",
                    ReadinessDisposition::HeldUnknown,
                    dispositions,
                );
            }
            if receipt.delivery_location_id != binding.target_location_id {
                push_blocker(
                    blockers,
                    BlockerCode::WrongDeliveryLocation,
                    &requirement.line_id,
                    Some(&lot.lot_id),
                    "Receipt location differs from the work-package staging location",
                    ReadinessDisposition::HeldUnknown,
                    dispositions,
                );
            }
        }
        LogisticsDisposition::Damaged => push_blocker(
            blockers,
            BlockerCode::MaterialDamaged,
            &requirement.line_id,
            Some(&lot.lot_id),
            "Recipient condition record marks the lot damaged",
            ReadinessDisposition::Rejected,
            dispositions,
        ),
        LogisticsDisposition::Rejected | LogisticsDisposition::Quarantined => push_blocker(
            blockers,
            BlockerCode::MaterialQuarantined,
            &requirement.line_id,
            Some(&lot.lot_id),
            "Lot is rejected or quarantined by the logistics/receiving owner",
            ReadinessDisposition::Rejected,
            dispositions,
        ),
        LogisticsDisposition::Unknown => push_blocker(
            blockers,
            BlockerCode::EvidenceUnknown,
            &requirement.line_id,
            Some(&lot.lot_id),
            "Logistics owner returned Unknown",
            ReadinessDisposition::HeldUnknown,
            dispositions,
        ),
    }
}

fn assess_inventory(
    receipt: &InventoryReservationReceipt,
    lot: &MaterialLotProjection,
    requirement: &MaterialRequirement,
    binding: &WorkPackageBinding,
    now_ms: u64,
    blockers: &mut Vec<ReadinessBlocker>,
    dispositions: &mut Vec<ReadinessDisposition>,
) {
    if receipt.receipt_ref.trim().is_empty()
        || receipt.source_profile_id.trim().is_empty()
        || receipt.source_schema_version.trim().is_empty()
    {
        push_blocker(
            blockers,
            BlockerCode::EvidenceReferenceMissing,
            &requirement.line_id,
            Some(&lot.lot_id),
            "Inventory source, schema or reservation receipt reference is missing",
            ReadinessDisposition::HeldUnknown,
            dispositions,
        );
        return;
    }
    if receipt.project_id != binding.project_id
        || receipt.lot_id != lot.lot_id
        || receipt.work_package_id != binding.work_package_id
        || receipt.design_revision_digest != binding.design_revision_digest
        || receipt.location_id != binding.target_location_id
    {
        push_blocker(
            blockers,
            BlockerCode::InventoryReservationScopeMismatch,
            &requirement.line_id,
            Some(&lot.lot_id),
            "Inventory reservation does not bind this lot, project, work package, design revision and location",
            ReadinessDisposition::HeldUnknown,
            dispositions,
        );
        return;
    }
    if receipt.site_epoch != binding.site_epoch {
        push_blocker(
            blockers,
            BlockerCode::InventoryReservationEpochMismatch,
            &requirement.line_id,
            Some(&lot.lot_id),
            "Inventory reservation belongs to a previous site epoch",
            ReadinessDisposition::Superseded,
            dispositions,
        );
        return;
    }
    if !is_current(now_ms, receipt.observed_at_ms, receipt.valid_until_ms) {
        push_blocker(
            blockers,
            BlockerCode::InventoryReservationStale,
            &requirement.line_id,
            Some(&lot.lot_id),
            "Inventory observation/reservation is not current",
            ReadinessDisposition::HeldUnknown,
            dispositions,
        );
        return;
    }
    if receipt.window_start_ms > binding.required_window_start_ms
        || receipt.window_end_ms < binding.required_window_end_ms
    {
        push_blocker(
            blockers,
            BlockerCode::InventoryReservationWindowMismatch,
            &requirement.line_id,
            Some(&lot.lot_id),
            "Inventory reservation does not cover the required use window",
            ReadinessDisposition::HeldUnknown,
            dispositions,
        );
        return;
    }
    match receipt.disposition {
        InventoryDisposition::ReservedForWorkPackage => {}
        InventoryDisposition::AvailableNotReserved => push_blocker(
            blockers,
            BlockerCode::InventoryNotReserved,
            &requirement.line_id,
            Some(&lot.lot_id),
            "Stock may be available but is not reserved for this work package",
            ReadinessDisposition::HeldUnknown,
            dispositions,
        ),
        InventoryDisposition::Stale => push_blocker(
            blockers,
            BlockerCode::InventoryReservationStale,
            &requirement.line_id,
            Some(&lot.lot_id),
            "Inventory owner marked the reservation stale",
            ReadinessDisposition::HeldUnknown,
            dispositions,
        ),
        InventoryDisposition::Conflict => push_blocker(
            blockers,
            BlockerCode::InventoryReservationConflict,
            &requirement.line_id,
            Some(&lot.lot_id),
            "Inventory owner reports a conflicting reservation",
            ReadinessDisposition::HeldUnknown,
            dispositions,
        ),
        InventoryDisposition::DamagedOrQuarantined => push_blocker(
            blockers,
            BlockerCode::MaterialQuarantined,
            &requirement.line_id,
            Some(&lot.lot_id),
            "Inventory owner quarantined or damaged the lot",
            ReadinessDisposition::Rejected,
            dispositions,
        ),
        InventoryDisposition::Unknown => push_blocker(
            blockers,
            BlockerCode::EvidenceUnknown,
            &requirement.line_id,
            Some(&lot.lot_id),
            "Inventory owner returned Unknown",
            ReadinessDisposition::HeldUnknown,
            dispositions,
        ),
    }
}

fn is_current(now_ms: u64, observed_at_ms: u64, valid_until_ms: u64) -> bool {
    observed_at_ms <= now_ms && now_ms <= valid_until_ms
}

fn push_blocker(
    blockers: &mut Vec<ReadinessBlocker>,
    code: BlockerCode,
    line_id: &str,
    lot_id: Option<&str>,
    detail: &str,
    disposition: ReadinessDisposition,
    dispositions: &mut Vec<ReadinessDisposition>,
) {
    blockers.push(ReadinessBlocker {
        code,
        line_id: line_id.to_owned(),
        lot_id: lot_id.map(str::to_owned),
        detail: detail.to_owned(),
    });
    dispositions.push(disposition);
}

fn worse(left: ReadinessDisposition, right: ReadinessDisposition) -> ReadinessDisposition {
    std::cmp::max(left, right)
}

fn worst_disposition(dispositions: &[ReadinessDisposition]) -> ReadinessDisposition {
    dispositions
        .iter()
        .copied()
        .fold(ReadinessDisposition::ReadyForWorkPackage, worse)
}

fn require_non_empty(field: &'static str, value: &str) -> Result<(), ReadinessError> {
    if value.trim().is_empty() {
        return Err(ReadinessError::EmptyField(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn binding() -> WorkPackageBinding {
        WorkPackageBinding {
            project_id: "project-01".to_owned(),
            floor_id: "floor-02".to_owned(),
            work_package_id: "wp-rebar-02".to_owned(),
            task_id: "task-rebar-02".to_owned(),
            design_revision_digest: "design-r7".to_owned(),
            site_epoch: "site-epoch-3".to_owned(),
            requirement_revision_digest: "material-spec-r4".to_owned(),
            receiving_entity_id: "site-store-01".to_owned(),
            target_location_id: "floor-02-material-zone".to_owned(),
            required_window_start_ms: 10_000,
            required_window_end_ms: 20_000,
        }
    }

    fn requirement() -> MaterialRequirement {
        MaterialRequirement {
            line_id: "rebar-grade-500".to_owned(),
            material_class_id: "rebar".to_owned(),
            grade_id: "500".to_owned(),
            unit_code: "kg".to_owned(),
            required_quantity_milli: 1_000_000,
            allowed_overage_milli: 0,
            specification_ref: "project-spec:rebar-r4".to_owned(),
            approved_substitutions: Vec::new(),
        }
    }

    fn evidence(ref_id: &str) -> EvidenceReceipt {
        EvidenceReceipt {
            receipt_ref: ref_id.to_owned(),
            source_profile_id: "mycelix-construction-provenance".to_owned(),
            source_schema_version: "1.0".to_owned(),
            project_id: "project-01".to_owned(),
            lot_id: "lot-a".to_owned(),
            design_revision_digest: "design-r7".to_owned(),
            requirement_revision_digest: "material-spec-r4".to_owned(),
            authority_ref: format!("authority:{ref_id}"),
            disposition: EvidenceDisposition::Qualified,
            observed_at_ms: 1_000,
            valid_until_ms: 30_000,
        }
    }

    fn lot() -> MaterialLotProjection {
        MaterialLotProjection {
            lot_id: "lot-a".to_owned(),
            material_class_id: "rebar".to_owned(),
            grade_id: "500".to_owned(),
            unit_code: "kg".to_owned(),
            provenance: evidence("provenance:lot-a"),
            logistics: LogisticsReceipt {
                receipt_ref: "logistics:recipient-acceptance:lot-a".to_owned(),
                source_profile_id: "mycelix-logistics".to_owned(),
                source_schema_version: "1.0".to_owned(),
                project_id: "project-01".to_owned(),
                lot_id: "lot-a".to_owned(),
                recipient_id: "site-store-01".to_owned(),
                delivery_location_id: "floor-02-material-zone".to_owned(),
                disposition: LogisticsDisposition::RecipientAccepted,
                observed_at_ms: 1_000,
                valid_until_ms: 30_000,
            },
            quality: evidence("quality:lot-a"),
            inventory: InventoryReservationReceipt {
                receipt_ref: "inventory:reservation:lot-a:wp-rebar-02".to_owned(),
                source_profile_id: "mycelix-logistics".to_owned(),
                source_schema_version: "1.0".to_owned(),
                project_id: "project-01".to_owned(),
                lot_id: "lot-a".to_owned(),
                work_package_id: "wp-rebar-02".to_owned(),
                design_revision_digest: "design-r7".to_owned(),
                site_epoch: "site-epoch-3".to_owned(),
                location_id: "floor-02-material-zone".to_owned(),
                disposition: InventoryDisposition::ReservedForWorkPackage,
                reserved_quantity_milli: 1_000_000,
                window_start_ms: 9_000,
                window_end_ms: 21_000,
                observed_at_ms: 1_000,
                valid_until_ms: 30_000,
            },
        }
    }

    fn kit(requirements: Vec<MaterialRequirement>, allocations: Vec<LotAllocation>) -> MaterialKit {
        MaterialKit {
            binding: binding(),
            requirements,
            allocations,
        }
    }

    fn allocation(line: &str, lot_id: &str, quantity: u64) -> LotAllocation {
        LotAllocation {
            line_id: line.to_owned(),
            lot_id: lot_id.to_owned(),
            quantity_milli: quantity,
        }
    }

    #[test]
    fn exact_qualified_lot_with_current_reservation_is_ready() {
        let result = assess_material_kit(
            &kit(
                vec![requirement()],
                vec![allocation("rebar-grade-500", "lot-a", 1_000_000)],
            ),
            &[lot()],
            5_000,
        )
        .expect("assessment");
        assert_eq!(
            result.disposition,
            ReadinessDisposition::ReadyForWorkPackage
        );
        assert!(!result.source_records_mutated);
        assert_eq!(result.source_receipt_refs.len(), 4);
    }

    #[test]
    fn dispatch_or_in_transit_is_not_recipient_delivery() {
        let mut item = lot();
        item.logistics.disposition = LogisticsDisposition::InTransit;
        let result = assess_material_kit(
            &kit(
                vec![requirement()],
                vec![allocation("rebar-grade-500", "lot-a", 1_000_000)],
            ),
            &[item],
            5_000,
        )
        .expect("assessment");
        assert_eq!(result.disposition, ReadinessDisposition::AwaitingDelivery);
        assert_eq!(
            result.lines[0].blockers[0].code,
            BlockerCode::DeliveryNotReceived
        );
    }

    #[test]
    fn delivered_but_not_recipient_accepted_requires_receiving_review() {
        let mut item = lot();
        item.logistics.disposition = LogisticsDisposition::DeliveredAwaitingAcceptance;
        let result = assess_material_kit(
            &kit(
                vec![requirement()],
                vec![allocation("rebar-grade-500", "lot-a", 1_000_000)],
            ),
            &[item],
            5_000,
        )
        .expect("assessment");
        assert_eq!(
            result.disposition,
            ReadinessDisposition::ReceivingReviewRequired
        );
    }

    #[test]
    fn supplier_assertion_without_qualified_evidence_stays_unknown() {
        let mut item = lot();
        item.provenance.disposition = EvidenceDisposition::AssertionOnly;
        let result = assess_material_kit(
            &kit(
                vec![requirement()],
                vec![allocation("rebar-grade-500", "lot-a", 1_000_000)],
            ),
            &[item],
            5_000,
        )
        .expect("assessment");
        assert_eq!(result.disposition, ReadinessDisposition::HeldUnknown);
        assert!(
            result.lines[0]
                .blockers
                .iter()
                .any(|b| b.code == BlockerCode::EvidenceAssertionOnly)
        );
    }

    #[test]
    fn revoked_quality_evidence_rejects_lot_readiness() {
        let mut item = lot();
        item.quality.disposition = EvidenceDisposition::Revoked;
        let result = assess_material_kit(
            &kit(
                vec![requirement()],
                vec![allocation("rebar-grade-500", "lot-a", 1_000_000)],
            ),
            &[item],
            5_000,
        )
        .expect("assessment");
        assert_eq!(result.disposition, ReadinessDisposition::Rejected);
    }

    #[test]
    fn stale_design_or_requirement_revision_is_superseded() {
        let mut item = lot();
        item.quality.design_revision_digest = "design-r6".to_owned();
        let result = assess_material_kit(
            &kit(
                vec![requirement()],
                vec![allocation("rebar-grade-500", "lot-a", 1_000_000)],
            ),
            &[item],
            5_000,
        )
        .expect("assessment");
        assert_eq!(result.disposition, ReadinessDisposition::Superseded);
    }

    #[test]
    fn inventory_reservation_from_previous_site_epoch_is_superseded() {
        let mut item = lot();
        item.inventory.site_epoch = "site-epoch-2".to_owned();
        let result = assess_material_kit(
            &kit(
                vec![requirement()],
                vec![allocation("rebar-grade-500", "lot-a", 1_000_000)],
            ),
            &[item],
            5_000,
        )
        .expect("assessment");
        assert_eq!(result.disposition, ReadinessDisposition::Superseded);
        assert!(
            result.lines[0]
                .blockers
                .iter()
                .any(|blocker| blocker.code == BlockerCode::InventoryReservationEpochMismatch)
        );
    }

    #[test]
    fn frozen_corpus_has_seventeen_named_cases_and_expected_dispositions() {
        const CORPUS: &str =
            include_str!("../fixtures/material-readiness-cases-v0.1.json");
        assert_eq!(CORPUS.matches("\"id\":").count(), 17);
        assert_eq!(CORPUS.matches("\"expected\":").count(), 17);
        assert_eq!(CORPUS.matches("\"matching_test\":").count(), 17);

        for case_id in [
            "exact-lot-current-reservation",
            "approved-substitution-wrong-project",
            "inventory-reservation-previous-site-epoch",
            "one-missing-kit-line",
        ] {
            assert!(CORPUS.contains(case_id), "missing frozen case {case_id}");
        }
    }

    #[test]
    fn available_but_not_reserved_stock_is_not_ready() {
        let mut item = lot();
        item.inventory.disposition = InventoryDisposition::AvailableNotReserved;
        let result = assess_material_kit(
            &kit(
                vec![requirement()],
                vec![allocation("rebar-grade-500", "lot-a", 1_000_000)],
            ),
            &[item],
            5_000,
        )
        .expect("assessment");
        assert_eq!(result.disposition, ReadinessDisposition::HeldUnknown);
        assert!(
            result.lines[0]
                .blockers
                .iter()
                .any(|b| b.code == BlockerCode::InventoryNotReserved)
        );
    }

    #[test]
    fn insufficient_reserved_quantity_blocks_complete_kit() {
        let mut item = lot();
        item.inventory.reserved_quantity_milli = 800_000;
        let result = assess_material_kit(
            &kit(
                vec![requirement()],
                vec![allocation("rebar-grade-500", "lot-a", 800_000)],
            ),
            &[item],
            5_000,
        )
        .expect("assessment");
        assert_eq!(
            result.disposition,
            ReadinessDisposition::InsufficientQuantity
        );
    }

    #[test]
    fn exact_revision_approved_substitute_can_be_eligible() {
        let mut req = requirement();
        req.approved_substitutions.push(ApprovedSubstitution {
            project_id: "project-01".to_owned(),
            requirement_revision_digest: "material-spec-r4".to_owned(),
            material_class_id: "approved-rebar-alt".to_owned(),
            grade_id: "500A".to_owned(),
            approval_ref: "design-decision:substitution-22".to_owned(),
            authority_ref: "engineer:approval-01".to_owned(),
            design_revision_digest: "design-r7".to_owned(),
        });
        let mut item = lot();
        item.material_class_id = "approved-rebar-alt".to_owned();
        item.grade_id = "500A".to_owned();
        let result = assess_material_kit(
            &kit(
                vec![req],
                vec![allocation("rebar-grade-500", "lot-a", 1_000_000)],
            ),
            &[item],
            5_000,
        )
        .expect("assessment");
        assert_eq!(
            result.disposition,
            ReadinessDisposition::ReadyForWorkPackage
        );
    }

    #[test]
    fn substitution_approval_must_match_project_and_requirement_revision() {
        let mut req = requirement();
        req.approved_substitutions.push(ApprovedSubstitution {
            project_id: "other-project".to_owned(),
            requirement_revision_digest: "material-spec-r4".to_owned(),
            material_class_id: "approved-rebar-alt".to_owned(),
            grade_id: "500A".to_owned(),
            approval_ref: "design-decision:substitution-22".to_owned(),
            authority_ref: "engineer:approval-01".to_owned(),
            design_revision_digest: "design-r7".to_owned(),
        });
        let mut item = lot();
        item.material_class_id = "approved-rebar-alt".to_owned();
        item.grade_id = "500A".to_owned();

        let result = assess_material_kit(
            &kit(
                vec![req],
                vec![allocation("rebar-grade-500", "lot-a", 1_000_000)],
            ),
            &[item],
            5_000,
        )
        .expect("assessment");
        assert_eq!(
            result.disposition,
            ReadinessDisposition::SubstitutionApprovalRequired
        );
    }

    #[test]
    fn duplicate_lot_line_allocation_identity_is_rejected() {
        let result = assess_material_kit(
            &kit(
                vec![requirement()],
                vec![
                    allocation("rebar-grade-500", "lot-a", 500_000),
                    allocation("rebar-grade-500", "lot-a", 500_000),
                ],
            ),
            &[lot()],
            5_000,
        );
        assert_eq!(
            result,
            Err(ReadinessError::DuplicateAllocation(
                "rebar-grade-500:lot-a".to_owned()
            ))
        );
    }

    #[test]
    fn substitute_approval_for_old_design_revision_is_not_current() {
        let mut req = requirement();
        req.approved_substitutions.push(ApprovedSubstitution {
            project_id: "project-01".to_owned(),
            requirement_revision_digest: "material-spec-r4".to_owned(),
            material_class_id: "approved-rebar-alt".to_owned(),
            grade_id: "500A".to_owned(),
            approval_ref: "design-decision:substitution-old".to_owned(),
            authority_ref: "engineer:approval-01".to_owned(),
            design_revision_digest: "design-r6".to_owned(),
        });
        let mut item = lot();
        item.material_class_id = "approved-rebar-alt".to_owned();
        item.grade_id = "500A".to_owned();
        let result = assess_material_kit(
            &kit(
                vec![req],
                vec![allocation("rebar-grade-500", "lot-a", 1_000_000)],
            ),
            &[item],
            5_000,
        )
        .expect("assessment");
        assert_eq!(
            result.disposition,
            ReadinessDisposition::SubstitutionApprovalRequired
        );
    }

    #[test]
    fn same_lot_quantity_cannot_be_double_counted_across_kit_lines() {
        let mut first = requirement();
        first.required_quantity_milli = 600_000;
        first.line_id = "line-a".to_owned();
        let mut second = requirement();
        second.required_quantity_milli = 600_000;
        second.line_id = "line-b".to_owned();
        let result = assess_material_kit(
            &kit(
                vec![first, second],
                vec![
                    allocation("line-a", "lot-a", 600_000),
                    allocation("line-b", "lot-a", 600_000),
                ],
            ),
            &[lot()],
            5_000,
        )
        .expect("assessment");
        assert_eq!(
            result.disposition,
            ReadinessDisposition::InsufficientQuantity
        );
        assert!(result.lines.iter().any(|line| {
            line.blockers
                .iter()
                .any(|b| b.detail.contains("allocated across kit lines"))
        }));
    }

    #[test]
    fn damaged_or_quarantined_material_is_rejected() {
        let mut item = lot();
        item.logistics.disposition = LogisticsDisposition::Quarantined;
        let result = assess_material_kit(
            &kit(
                vec![requirement()],
                vec![allocation("rebar-grade-500", "lot-a", 1_000_000)],
            ),
            &[item],
            5_000,
        )
        .expect("assessment");
        assert_eq!(result.disposition, ReadinessDisposition::Rejected);
    }

    #[test]
    fn unit_mismatch_is_held_without_implicit_conversion() {
        let mut item = lot();
        item.unit_code = "pieces".to_owned();
        let result = assess_material_kit(
            &kit(
                vec![requirement()],
                vec![allocation("rebar-grade-500", "lot-a", 1_000_000)],
            ),
            &[item],
            5_000,
        )
        .expect("assessment");
        assert_eq!(result.disposition, ReadinessDisposition::HeldUnknown);
        assert!(
            result.lines[0]
                .blockers
                .iter()
                .any(|b| b.code == BlockerCode::UnitMismatch)
        );
    }

    #[test]
    fn multi_line_kit_with_one_missing_line_is_not_ready() {
        let mut second = requirement();
        second.line_id = "line-b".to_owned();
        second.material_class_id = "anchor-bolt".to_owned();
        second.grade_id = "A4".to_owned();
        let result = assess_material_kit(
            &kit(
                vec![requirement(), second],
                vec![allocation("rebar-grade-500", "lot-a", 1_000_000)],
            ),
            &[lot()],
            5_000,
        )
        .expect("assessment");
        assert_ne!(
            result.disposition,
            ReadinessDisposition::ReadyForWorkPackage
        );
        assert_eq!(
            result.lines[1].disposition,
            ReadinessDisposition::HeldUnknown
        );
        assert!(
            result.lines[1]
                .blockers
                .iter()
                .any(|b| b.code == BlockerCode::MissingAllocation)
        );
    }
}
