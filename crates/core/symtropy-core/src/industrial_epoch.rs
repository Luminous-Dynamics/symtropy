// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Conservation-checked handoff between deterministic industrial-ecology epochs.
//!
//! A design-generation transition is represented as a new validated ecology rather
//! than an in-place rewrite of the old one. Source inventory is exhaustively
//! accounted as transferred or retired; successor bootstrap inventory must either
//! come from that transfer or from an explicit externally admitted quantity.
//! This is simulation/evidence plumbing only and grants no manufacturing,
//! procurement, qualification, operating, or physical-control authority.

use crate::industrial_ecology::{
    IndustrialCapability, IndustrialDependencyState, IndustrialEcology, IndustrialEcologyError,
    IndustrialFlowPrerequisite, IndustrialShock, IndustrialTickReport,
};
use std::collections::{BTreeMap, BTreeSet};

const MAX_ID_LEN: usize = 256;
const MAX_BINDING_LEN: usize = 1024;
const MAX_ITEMS: usize = 4096;

/// Whether an epoch uses legacy ungated local flows or prerequisite-gated flows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndustrialEpochFlowModel {
    /// Preserve the original `IndustrialEcology::new(...)` behavior.
    LegacyUngated,
    /// Require exact prerequisite coverage for every initially positive local flow.
    PrerequisiteGated(Vec<IndustrialFlowPrerequisite>),
}

/// Immutable construction specification for one industrial epoch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndustrialEpochSpec {
    pub epoch_id: String,
    /// Opaque identity/version binding for this exact epoch specification.
    pub evidence_binding: String,
    pub dependencies: Vec<IndustrialDependencyState>,
    pub capabilities: Vec<IndustrialCapability>,
    pub flow_model: IndustrialEpochFlowModel,
}

impl IndustrialEpochSpec {
    fn validate_identity(&self) -> Result<(), IndustrialEpochHandoffError> {
        validate_id(&self.epoch_id)?;
        validate_binding(&self.evidence_binding)?;
        if self.dependencies.len() > MAX_ITEMS || self.capabilities.len() > MAX_ITEMS {
            return Err(IndustrialEpochHandoffError::ModelTooLarge);
        }
        Ok(())
    }

    fn build(self) -> Result<IndustrialEpochState, IndustrialEpochHandoffError> {
        self.validate_identity()?;
        let dependency_ids: Vec<String> = self
            .dependencies
            .iter()
            .map(|dependency| dependency.dependency_id.clone())
            .collect();
        let ecology = match self.flow_model {
            IndustrialEpochFlowModel::LegacyUngated => {
                IndustrialEcology::new(self.dependencies, self.capabilities)
            }
            IndustrialEpochFlowModel::PrerequisiteGated(prerequisites) => {
                IndustrialEcology::new_with_flow_prerequisites(
                    self.dependencies,
                    self.capabilities,
                    prerequisites,
                )
            }
        }
        .map_err(IndustrialEpochHandoffError::Ecology)?;
        Ok(IndustrialEpochState {
            epoch_id: self.epoch_id,
            evidence_binding: self.evidence_binding,
            dependency_ids,
            ecology,
            last_report: None,
        })
    }
}

/// Linear simulation state for one exact industrial epoch.
///
/// Deliberately not `Clone`: `handoff_to(...)` consumes the source epoch so one
/// lineage execution cannot accidentally spend the same inventory twice. A caller
/// may still intentionally construct independent what-if branches from distinct
/// epoch specifications; those branches must carry distinct evidence identities.
#[derive(Debug, PartialEq, Eq)]
pub struct IndustrialEpochState {
    epoch_id: String,
    evidence_binding: String,
    dependency_ids: Vec<String>,
    ecology: IndustrialEcology,
    last_report: Option<IndustrialTickReport>,
}

impl IndustrialEpochState {
    /// Construct a root/current epoch. Initial inventory is allowed here.
    pub fn from_spec(spec: IndustrialEpochSpec) -> Result<Self, IndustrialEpochHandoffError> {
        spec.build()
    }

    pub fn epoch_id(&self) -> &str {
        &self.epoch_id
    }

    pub fn evidence_binding(&self) -> &str {
        &self.evidence_binding
    }

    pub fn tick(&self) -> u64 {
        self.ecology.tick()
    }

    pub fn dependency(&self, dependency_id: &str) -> Option<&IndustrialDependencyState> {
        self.ecology.dependency(dependency_id)
    }

    pub fn last_report(&self) -> Option<&IndustrialTickReport> {
        self.last_report.as_ref()
    }

    pub fn apply_shock(&mut self, shock: IndustrialShock) -> Result<(), IndustrialEcologyError> {
        self.ecology.apply_shock(shock)
    }

    pub fn step(&mut self) -> Result<IndustrialTickReport, IndustrialEcologyError> {
        let report = self.ecology.step()?;
        self.last_report = Some(report.clone());
        Ok(report)
    }

    /// Consume this epoch and create a separately validated successor epoch.
    pub fn handoff_to(
        self,
        successor_spec: IndustrialEpochSpec,
        plan: IndustrialEpochHandoffPlan,
    ) -> Result<(IndustrialEpochState, IndustrialEpochHandoffReceipt), IndustrialEpochHandoffError>
    {
        validate_handoff_identity(&self, &successor_spec, &plan)?;
        validate_successor_zero_inventory(&successor_spec)?;
        validate_plan_shape(&plan)?;

        let source_ids: BTreeSet<&str> = self.dependency_ids.iter().map(String::as_str).collect();
        let disposition_ids: BTreeSet<&str> = plan
            .source_inventory_dispositions
            .iter()
            .map(|disposition| disposition.source_dependency_id.as_str())
            .collect();
        if source_ids != disposition_ids {
            return Err(IndustrialEpochHandoffError::SourceInventoryCoverageMismatch);
        }

        let successor_governance: BTreeMap<String, _> = successor_spec
            .dependencies
            .iter()
            .map(|dependency| (dependency.dependency_id.clone(), dependency.governance))
            .collect();
        let mut transferred_successor_ids = BTreeSet::new();
        let mut transferred_units_by_successor: BTreeMap<String, u64> = BTreeMap::new();

        for disposition in &plan.source_inventory_dispositions {
            let source = self
                .ecology
                .dependency(&disposition.source_dependency_id)
                .ok_or_else(|| IndustrialEpochHandoffError::UnknownSourceDependency {
                    dependency_id: disposition.source_dependency_id.clone(),
                })?;
            let accounted = disposition
                .transferred_units
                .checked_add(disposition.retired_units)
                .ok_or(IndustrialEpochHandoffError::ArithmeticOverflow)?;
            if accounted != source.inventory_units {
                return Err(IndustrialEpochHandoffError::SourceInventoryNotConserved {
                    dependency_id: disposition.source_dependency_id.clone(),
                    available_units: source.inventory_units,
                    accounted_units: accounted,
                });
            }

            match &disposition.successor_dependency_id {
                Some(successor_id) => {
                    if disposition.transferred_units == 0 {
                        return Err(IndustrialEpochHandoffError::ZeroTransferNamesSuccessor {
                            source_dependency_id: disposition.source_dependency_id.clone(),
                        });
                    }
                    let successor_governance = successor_governance
                        .get(successor_id.as_str())
                        .ok_or_else(|| IndustrialEpochHandoffError::UnknownSuccessorDependency {
                            dependency_id: successor_id.clone(),
                        })?;
                    if source.governance != *successor_governance {
                        return Err(IndustrialEpochHandoffError::GovernanceBoundaryChanged {
                            source_dependency_id: disposition.source_dependency_id.clone(),
                            successor_dependency_id: successor_id.clone(),
                        });
                    }
                    if !transferred_successor_ids.insert(successor_id.clone()) {
                        return Err(IndustrialEpochHandoffError::MultipleSourcesForSuccessor {
                            successor_dependency_id: successor_id.clone(),
                        });
                    }
                    transferred_units_by_successor
                        .insert(successor_id.clone(), disposition.transferred_units);
                }
                None => {
                    if disposition.transferred_units != 0 {
                        return Err(IndustrialEpochHandoffError::TransferMissingSuccessor {
                            source_dependency_id: disposition.source_dependency_id.clone(),
                        });
                    }
                }
            }
        }

        let mut external_by_successor: BTreeMap<String, u64> = BTreeMap::new();
        for admission in &plan.external_inventory_admissions {
            if !successor_governance.contains_key(admission.successor_dependency_id.as_str()) {
                return Err(IndustrialEpochHandoffError::UnknownSuccessorDependency {
                    dependency_id: admission.successor_dependency_id.clone(),
                });
            }
            if admission.units == 0 {
                return Err(IndustrialEpochHandoffError::ZeroExternalInventory {
                    successor_dependency_id: admission.successor_dependency_id.clone(),
                });
            }
            validate_binding(&admission.evidence_binding)?;
            if external_by_successor
                .insert(admission.successor_dependency_id.clone(), admission.units)
                .is_some()
            {
                return Err(IndustrialEpochHandoffError::DuplicateExternalInventoryAdmission {
                    successor_dependency_id: admission.successor_dependency_id.clone(),
                });
            }
        }

        let mut successor_spec = successor_spec;
        let mut resulting_inventory = Vec::with_capacity(successor_spec.dependencies.len());
        for dependency in &mut successor_spec.dependencies {
            let transferred = transferred_units_by_successor
                .get(&dependency.dependency_id)
                .copied()
                .unwrap_or(0);
            let external = external_by_successor
                .get(&dependency.dependency_id)
                .copied()
                .unwrap_or(0);
            dependency.inventory_units = transferred
                .checked_add(external)
                .ok_or(IndustrialEpochHandoffError::ArithmeticOverflow)?;
            resulting_inventory.push(IndustrialEpochInventoryResult {
                successor_dependency_id: dependency.dependency_id.clone(),
                transferred_units: transferred,
                external_units: external,
                resulting_units: dependency.inventory_units,
            });
        }
        resulting_inventory.sort_by(|left, right| {
            left.successor_dependency_id
                .cmp(&right.successor_dependency_id)
        });

        let source_final_tick = self.ecology.tick();
        let (source_final_shortage_ids, source_final_unavailable_capability_ids) = self
            .last_report
            .as_ref()
            .map(|report| {
                (
                    report
                        .shortages
                        .iter()
                        .map(|shortage| shortage.dependency_id.clone())
                        .collect(),
                    report.unavailable_capability_ids.clone(),
                )
            })
            .unwrap_or_else(|| (Vec::new(), Vec::new()));

        let source_epoch_id = self.epoch_id;
        let source_epoch_evidence_binding = self.evidence_binding;
        let successor_epoch_id = successor_spec.epoch_id.clone();
        let successor_epoch_evidence_binding = successor_spec.evidence_binding.clone();
        let successor = successor_spec.build()?;

        let receipt = IndustrialEpochHandoffReceipt {
            handoff_id: plan.handoff_id,
            evidence_binding: plan.evidence_binding,
            source_epoch_id,
            source_epoch_evidence_binding,
            source_final_tick,
            successor_epoch_id,
            successor_epoch_evidence_binding,
            source_inventory_dispositions: plan.source_inventory_dispositions,
            external_inventory_admissions: plan.external_inventory_admissions,
            resulting_inventory,
            source_final_shortage_ids,
            source_final_unavailable_capability_ids,
        };
        Ok((successor, receipt))
    }
}

/// Exhaustive disposition for one source dependency's qualified inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndustrialSourceInventoryDisposition {
    pub source_dependency_id: String,
    /// Target dependency in the successor epoch, if any inventory is transferred.
    pub successor_dependency_id: Option<String>,
    pub transferred_units: u64,
    /// Units deliberately retired, scrapped, stranded, or otherwise not admitted
    /// into the successor epoch. The simulator does not prescribe how that occurs.
    pub retired_units: u64,
}

/// Explicit inventory admitted from outside the predecessor epoch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndustrialExternalInventoryAdmission {
    pub successor_dependency_id: String,
    pub units: u64,
    /// Opaque evidence identity for the externally admitted inventory.
    pub evidence_binding: String,
}

/// Exact, evidence-bound transition plan between two epoch identities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndustrialEpochHandoffPlan {
    pub handoff_id: String,
    pub evidence_binding: String,
    pub source_epoch_id: String,
    pub source_epoch_evidence_binding: String,
    pub successor_epoch_id: String,
    pub successor_epoch_evidence_binding: String,
    /// Strictly sorted by source dependency ID, one record per source dependency.
    pub source_inventory_dispositions: Vec<IndustrialSourceInventoryDisposition>,
    /// Strictly sorted by successor dependency ID, duplicate-free.
    pub external_inventory_admissions: Vec<IndustrialExternalInventoryAdmission>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndustrialEpochInventoryResult {
    pub successor_dependency_id: String,
    pub transferred_units: u64,
    pub external_units: u64,
    pub resulting_units: u64,
}

/// Immutable diagnostic receipt for a completed simulation-epoch transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndustrialEpochHandoffReceipt {
    pub handoff_id: String,
    pub evidence_binding: String,
    pub source_epoch_id: String,
    pub source_epoch_evidence_binding: String,
    pub source_final_tick: u64,
    pub successor_epoch_id: String,
    pub successor_epoch_evidence_binding: String,
    pub source_inventory_dispositions: Vec<IndustrialSourceInventoryDisposition>,
    pub external_inventory_admissions: Vec<IndustrialExternalInventoryAdmission>,
    pub resulting_inventory: Vec<IndustrialEpochInventoryResult>,
    pub source_final_shortage_ids: Vec<String>,
    pub source_final_unavailable_capability_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndustrialEpochHandoffError {
    InvalidIdentifier,
    InvalidEvidenceBinding,
    ModelTooLarge,
    Ecology(IndustrialEcologyError),
    HandoffSourceIdentityMismatch,
    HandoffSuccessorIdentityMismatch,
    EpochIdentityNotDistinct,
    SuccessorTemplateContainsInventory { dependency_id: String },
    NonCanonicalSourceDispositions,
    NonCanonicalExternalAdmissions,
    SourceInventoryCoverageMismatch,
    UnknownSourceDependency { dependency_id: String },
    UnknownSuccessorDependency { dependency_id: String },
    SourceInventoryNotConserved {
        dependency_id: String,
        available_units: u64,
        accounted_units: u64,
    },
    ZeroTransferNamesSuccessor { source_dependency_id: String },
    TransferMissingSuccessor { source_dependency_id: String },
    MultipleSourcesForSuccessor { successor_dependency_id: String },
    GovernanceBoundaryChanged {
        source_dependency_id: String,
        successor_dependency_id: String,
    },
    ZeroExternalInventory { successor_dependency_id: String },
    DuplicateExternalInventoryAdmission { successor_dependency_id: String },
    ArithmeticOverflow,
}

fn validate_handoff_identity(
    source: &IndustrialEpochState,
    successor: &IndustrialEpochSpec,
    plan: &IndustrialEpochHandoffPlan,
) -> Result<(), IndustrialEpochHandoffError> {
    successor.validate_identity()?;
    validate_id(&plan.handoff_id)?;
    validate_binding(&plan.evidence_binding)?;
    validate_id(&plan.source_epoch_id)?;
    validate_binding(&plan.source_epoch_evidence_binding)?;
    validate_id(&plan.successor_epoch_id)?;
    validate_binding(&plan.successor_epoch_evidence_binding)?;
    if plan.source_epoch_id != source.epoch_id
        || plan.source_epoch_evidence_binding != source.evidence_binding
    {
        return Err(IndustrialEpochHandoffError::HandoffSourceIdentityMismatch);
    }
    if plan.successor_epoch_id != successor.epoch_id
        || plan.successor_epoch_evidence_binding != successor.evidence_binding
    {
        return Err(IndustrialEpochHandoffError::HandoffSuccessorIdentityMismatch);
    }
    if source.epoch_id == successor.epoch_id
        || source.evidence_binding == successor.evidence_binding
    {
        return Err(IndustrialEpochHandoffError::EpochIdentityNotDistinct);
    }
    Ok(())
}

fn validate_successor_zero_inventory(
    successor: &IndustrialEpochSpec,
) -> Result<(), IndustrialEpochHandoffError> {
    for dependency in &successor.dependencies {
        if dependency.inventory_units != 0 {
            return Err(IndustrialEpochHandoffError::SuccessorTemplateContainsInventory {
                dependency_id: dependency.dependency_id.clone(),
            });
        }
    }
    Ok(())
}

fn validate_plan_shape(plan: &IndustrialEpochHandoffPlan) -> Result<(), IndustrialEpochHandoffError> {
    if plan.source_inventory_dispositions.len() > MAX_ITEMS
        || plan.external_inventory_admissions.len() > MAX_ITEMS
    {
        return Err(IndustrialEpochHandoffError::ModelTooLarge);
    }
    for disposition in &plan.source_inventory_dispositions {
        validate_id(&disposition.source_dependency_id)?;
        if let Some(successor_id) = &disposition.successor_dependency_id {
            validate_id(successor_id)?;
        }
    }
    if plan
        .source_inventory_dispositions
        .windows(2)
        .any(|pair| pair[0].source_dependency_id >= pair[1].source_dependency_id)
    {
        return Err(IndustrialEpochHandoffError::NonCanonicalSourceDispositions);
    }
    for admission in &plan.external_inventory_admissions {
        validate_id(&admission.successor_dependency_id)?;
        validate_binding(&admission.evidence_binding)?;
    }
    if plan
        .external_inventory_admissions
        .windows(2)
        .any(|pair| pair[0].successor_dependency_id >= pair[1].successor_dependency_id)
    {
        return Err(IndustrialEpochHandoffError::NonCanonicalExternalAdmissions);
    }
    Ok(())
}

fn validate_id(value: &str) -> Result<(), IndustrialEpochHandoffError> {
    if value.is_empty()
        || value.len() > MAX_ID_LEN
        || value.trim() != value
        || value.chars().any(char::is_whitespace)
        || value.chars().any(char::is_control)
    {
        Err(IndustrialEpochHandoffError::InvalidIdentifier)
    } else {
        Ok(())
    }
}

fn validate_binding(value: &str) -> Result<(), IndustrialEpochHandoffError> {
    if value.is_empty()
        || value.len() > MAX_BINDING_LEN
        || value.trim() != value
        || !value.contains(':')
        || value.chars().any(char::is_whitespace)
        || value.chars().any(char::is_control)
    {
        Err(IndustrialEpochHandoffError::InvalidEvidenceBinding)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::industrial_ecology::{IndustrialFlowKind, IndustrialGovernance};

    fn dependency(
        id: &str,
        governance: IndustrialGovernance,
        inventory: u64,
    ) -> IndustrialDependencyState {
        IndustrialDependencyState {
            dependency_id: id.into(),
            governance,
            demand_units_per_tick: 1,
            local_production_units_per_tick: 0,
            recycling_units_per_tick: 0,
            inventory_units: inventory,
        }
    }

    fn capability(id: &str, dependency_id: &str) -> IndustrialCapability {
        IndustrialCapability {
            capability_id: id.into(),
            essential: true,
            dependency_ids: BTreeSet::from([dependency_id.into()]),
        }
    }

    fn source_spec() -> IndustrialEpochSpec {
        IndustrialEpochSpec {
            epoch_id: "manta-v1".into(),
            evidence_binding: "epoch:manta-v1".into(),
            dependencies: vec![
                dependency("metrology-v1", IndustrialGovernance::Ordinary, 30),
                dependency(
                    "reactor-service",
                    IndustrialGovernance::SafeguardedExternal,
                    7,
                ),
                dependency("spares-v1", IndustrialGovernance::Ordinary, 100),
            ],
            capabilities: vec![capability("operation-v1", "spares-v1")],
            flow_model: IndustrialEpochFlowModel::LegacyUngated,
        }
    }

    fn successor_spec() -> IndustrialEpochSpec {
        IndustrialEpochSpec {
            epoch_id: "manta-v2".into(),
            evidence_binding: "epoch:manta-v2".into(),
            dependencies: vec![
                dependency("metrology-v2", IndustrialGovernance::Ordinary, 0),
                dependency(
                    "reactor-service-v2",
                    IndustrialGovernance::SafeguardedExternal,
                    0,
                ),
                dependency("spares-v2", IndustrialGovernance::Ordinary, 0),
            ],
            capabilities: vec![capability("operation-v2", "spares-v2")],
            flow_model: IndustrialEpochFlowModel::LegacyUngated,
        }
    }

    fn plan() -> IndustrialEpochHandoffPlan {
        IndustrialEpochHandoffPlan {
            handoff_id: "handoff-manta-v1-v2".into(),
            evidence_binding: "handoff:manta-v1-v2".into(),
            source_epoch_id: "manta-v1".into(),
            source_epoch_evidence_binding: "epoch:manta-v1".into(),
            successor_epoch_id: "manta-v2".into(),
            successor_epoch_evidence_binding: "epoch:manta-v2".into(),
            source_inventory_dispositions: vec![
                IndustrialSourceInventoryDisposition {
                    source_dependency_id: "metrology-v1".into(),
                    successor_dependency_id: Some("metrology-v2".into()),
                    transferred_units: 20,
                    retired_units: 10,
                },
                IndustrialSourceInventoryDisposition {
                    source_dependency_id: "reactor-service".into(),
                    successor_dependency_id: Some("reactor-service-v2".into()),
                    transferred_units: 7,
                    retired_units: 0,
                },
                IndustrialSourceInventoryDisposition {
                    source_dependency_id: "spares-v1".into(),
                    successor_dependency_id: Some("spares-v2".into()),
                    transferred_units: 80,
                    retired_units: 20,
                },
            ],
            external_inventory_admissions: vec![IndustrialExternalInventoryAdmission {
                successor_dependency_id: "spares-v2".into(),
                units: 5,
                evidence_binding: "external:qualified-spares".into(),
            }],
        }
    }

    #[test]
    fn handoff_conserves_source_inventory_and_builds_fresh_successor() {
        let mut source = IndustrialEpochState::from_spec(source_spec()).unwrap();
        let report = source.step().unwrap();
        assert_eq!(report.tick, 1);

        // One unit of each source dependency was consumed by the completed tick.
        let mut adjusted = plan();
        adjusted.source_inventory_dispositions[0].transferred_units = 19;
        adjusted.source_inventory_dispositions[1].transferred_units = 6;
        adjusted.source_inventory_dispositions[2].transferred_units = 79;

        let (mut successor, receipt) = source.handoff_to(successor_spec(), adjusted).unwrap();
        assert_eq!(successor.epoch_id(), "manta-v2");
        assert_eq!(successor.tick(), 0);
        assert_eq!(
            successor.dependency("metrology-v2").unwrap().inventory_units,
            19
        );
        assert_eq!(
            successor
                .dependency("reactor-service-v2")
                .unwrap()
                .inventory_units,
            6
        );
        assert_eq!(successor.dependency("spares-v2").unwrap().inventory_units, 84);
        assert_eq!(receipt.source_final_tick, 1);
        assert_eq!(
            receipt.resulting_inventory[2].successor_dependency_id,
            "spares-v2"
        );
        assert_eq!(receipt.resulting_inventory[2].transferred_units, 79);
        assert_eq!(receipt.resulting_inventory[2].external_units, 5);
        assert_eq!(receipt.resulting_inventory[2].resulting_units, 84);
        assert!(successor.step().is_ok());
    }

    #[test]
    fn every_source_inventory_unit_must_be_accounted() {
        let source = IndustrialEpochState::from_spec(source_spec()).unwrap();
        let mut invalid = plan();
        invalid.source_inventory_dispositions[2].retired_units = 19;
        assert!(matches!(
            source.handoff_to(successor_spec(), invalid),
            Err(IndustrialEpochHandoffError::SourceInventoryNotConserved {
                dependency_id,
                available_units: 100,
                accounted_units: 99,
            }) if dependency_id == "spares-v1"
        ));
    }

    #[test]
    fn every_source_dependency_requires_exactly_one_disposition() {
        let source = IndustrialEpochState::from_spec(source_spec()).unwrap();
        let mut invalid = plan();
        invalid.source_inventory_dispositions.remove(1);
        assert_eq!(
            source.handoff_to(successor_spec(), invalid),
            Err(IndustrialEpochHandoffError::SourceInventoryCoverageMismatch)
        );
    }

    #[test]
    fn two_sources_cannot_spend_into_the_same_successor_dependency() {
        let source = IndustrialEpochState::from_spec(source_spec()).unwrap();
        let mut invalid = plan();
        invalid.source_inventory_dispositions[2].successor_dependency_id =
            Some("metrology-v2".into());
        assert_eq!(
            source.handoff_to(successor_spec(), invalid),
            Err(IndustrialEpochHandoffError::MultipleSourcesForSuccessor {
                successor_dependency_id: "metrology-v2".into(),
            })
        );
    }

    #[test]
    fn governance_boundary_cannot_change_during_inventory_transfer() {
        let source = IndustrialEpochState::from_spec(source_spec()).unwrap();
        let mut successor = successor_spec();
        successor.dependencies[1].governance = IndustrialGovernance::Ordinary;
        assert!(matches!(
            source.handoff_to(successor, plan()),
            Err(IndustrialEpochHandoffError::GovernanceBoundaryChanged { .. })
        ));
    }

    #[test]
    fn successor_template_cannot_hide_preexisting_inventory() {
        let source = IndustrialEpochState::from_spec(source_spec()).unwrap();
        let mut successor = successor_spec();
        successor.dependencies[2].inventory_units = 1;
        assert_eq!(
            source.handoff_to(successor, plan()),
            Err(IndustrialEpochHandoffError::SuccessorTemplateContainsInventory {
                dependency_id: "spares-v2".into(),
            })
        );
    }

    #[test]
    fn external_inventory_requires_explicit_canonical_evidence() {
        let source = IndustrialEpochState::from_spec(source_spec()).unwrap();
        let mut invalid = plan();
        invalid.external_inventory_admissions[0].evidence_binding = "opaque".into();
        assert_eq!(
            source.handoff_to(successor_spec(), invalid),
            Err(IndustrialEpochHandoffError::InvalidEvidenceBinding)
        );
    }

    #[test]
    fn successor_epoch_can_revalidate_prerequisite_gated_flows() {
        let source = IndustrialEpochState::from_spec(source_spec()).unwrap();
        let mut successor = successor_spec();
        successor.dependencies[2].local_production_units_per_tick = 1;
        successor.flow_model = IndustrialEpochFlowModel::PrerequisiteGated(vec![
            IndustrialFlowPrerequisite {
                dependency_id: "spares-v2".into(),
                flow_kind: IndustrialFlowKind::Production,
                prerequisite_dependency_ids: BTreeSet::from(["metrology-v2".into()]),
            },
        ]);
        let (mut successor, _) = source.handoff_to(successor, plan()).unwrap();
        assert_eq!(successor.epoch_id(), "manta-v2");
        assert!(successor.step().is_ok());
    }

    #[test]
    fn source_and_successor_epoch_identities_must_be_distinct_and_exact() {
        let source = IndustrialEpochState::from_spec(source_spec()).unwrap();
        let mut invalid = plan();
        invalid.successor_epoch_evidence_binding = "epoch:other".into();
        assert_eq!(
            source.handoff_to(successor_spec(), invalid),
            Err(IndustrialEpochHandoffError::HandoffSuccessorIdentityMismatch)
        );

        let source = IndustrialEpochState::from_spec(source_spec()).unwrap();
        let mut successor = successor_spec();
        successor.epoch_id = "manta-v1".into();
        let mut same = plan();
        same.successor_epoch_id = "manta-v1".into();
        assert_eq!(
            source.handoff_to(successor, same),
            Err(IndustrialEpochHandoffError::EpochIdentityNotDistinct)
        );
    }
}
