// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Event-sourced physical identity and installation reconciliation for Power Fabric.
//!
//! This module projects an append-only event history into an as-known-at installation
//! view. It deliberately does not authenticate source evidence, verify signatures,
//! establish product authenticity, solve electrical systems, or authorize field work.
//! Evidence commitments are shape-checked here; their admission and provenance must be
//! supplied by the owning evidence-integrity layer. Independent review is delegated to
//! the separately qualified review-attestation verifier.

use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhysicalIdentityError {
    EmptyField(&'static str),
    InvalidSha256 { field: &'static str, value: String },
    InvalidTimeOrder { event_id: String },
    InvalidEvidenceTimeOrder { evidence_id: String },
    DuplicateProduct { product_id: String },
    DuplicateSlot { slot_id: String },
    DuplicatePhysicalInstance { instance_id: String },
    DuplicateSerialIdentity { namespace: String, serial: String },
    DuplicateEvidence { evidence_id: String },
    DuplicateEvent { event_id: String },
    UnknownProduct { product_id: String },
    UnknownSlot { slot_id: String },
    UnknownPhysicalInstance { instance_id: String },
    UnknownEvidence { evidence_id: String },
    UnknownPredecessor { event_id: String, predecessor_id: String },
    InvalidSlotContract { slot_id: String },
    InvalidEventShape { event_id: String, reason: &'static str },
    PredecessorAfterEvent { event_id: String, predecessor_id: String },
    EventDependencyCycle,
}

impl std::fmt::Display for PhysicalIdentityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for PhysicalIdentityError {}

fn require_non_empty(field: &'static str, value: &str) -> Result<(), PhysicalIdentityError> {
    if value.trim().is_empty() {
        return Err(PhysicalIdentityError::EmptyField(field));
    }
    Ok(())
}

fn validate_sha256(field: &'static str, value: &str) -> Result<(), PhysicalIdentityError> {
    let valid = value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
    if !valid {
        return Err(PhysicalIdentityError::InvalidSha256 {
            field,
            value: value.to_owned(),
        });
    }
    Ok(())
}

/// The immutable design/build snapshot against which installation events are interpreted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InstallationSnapshot {
    installation_id: String,
    installation_revision: String,
    source_ir_sha256: String,
    graph_id: String,
    graph_revision: String,
    graph_payload_sha256: String,
}

impl InstallationSnapshot {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        installation_id: impl Into<String>,
        installation_revision: impl Into<String>,
        source_ir_sha256: impl Into<String>,
        graph_id: impl Into<String>,
        graph_revision: impl Into<String>,
        graph_payload_sha256: impl Into<String>,
    ) -> Result<Self, PhysicalIdentityError> {
        let value = Self {
            installation_id: installation_id.into(),
            installation_revision: installation_revision.into(),
            source_ir_sha256: source_ir_sha256.into(),
            graph_id: graph_id.into(),
            graph_revision: graph_revision.into(),
            graph_payload_sha256: graph_payload_sha256.into(),
        };
        require_non_empty("installation_id", &value.installation_id)?;
        require_non_empty("installation_revision", &value.installation_revision)?;
        require_non_empty("graph_id", &value.graph_id)?;
        require_non_empty("graph_revision", &value.graph_revision)?;
        validate_sha256("source_ir_sha256", &value.source_ir_sha256)?;
        validate_sha256("graph_payload_sha256", &value.graph_payload_sha256)?;
        Ok(value)
    }

    pub fn installation_id(&self) -> &str {
        &self.installation_id
    }

    pub fn graph_payload_sha256(&self) -> &str {
        &self.graph_payload_sha256
    }
}

/// A versioned manufacturer/catalogue product model, not an individual physical unit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProductModel {
    product_id: String,
    manufacturer: String,
    catalogue_identifier: String,
    catalogue_revision: String,
}

impl ProductModel {
    pub fn new(
        product_id: impl Into<String>,
        manufacturer: impl Into<String>,
        catalogue_identifier: impl Into<String>,
        catalogue_revision: impl Into<String>,
    ) -> Result<Self, PhysicalIdentityError> {
        let value = Self {
            product_id: product_id.into(),
            manufacturer: manufacturer.into(),
            catalogue_identifier: catalogue_identifier.into(),
            catalogue_revision: catalogue_revision.into(),
        };
        require_non_empty("product_id", &value.product_id)?;
        require_non_empty("manufacturer", &value.manufacturer)?;
        require_non_empty("catalogue_identifier", &value.catalogue_identifier)?;
        require_non_empty("catalogue_revision", &value.catalogue_revision)?;
        Ok(value)
    }

    pub fn product_id(&self) -> &str {
        &self.product_id
    }
}

/// A stable logical equipment position in the design graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DesignSlot {
    slot_id: String,
    expected_location_id: String,
    accepted_hardware_revisions: BTreeMap<String, BTreeSet<String>>,
    max_active_instances: usize,
}

impl DesignSlot {
    pub fn new(
        slot_id: impl Into<String>,
        expected_location_id: impl Into<String>,
        accepted_hardware_revisions: BTreeMap<String, BTreeSet<String>>,
        max_active_instances: usize,
    ) -> Result<Self, PhysicalIdentityError> {
        let value = Self {
            slot_id: slot_id.into(),
            expected_location_id: expected_location_id.into(),
            accepted_hardware_revisions,
            max_active_instances,
        };
        require_non_empty("slot_id", &value.slot_id)?;
        require_non_empty("expected_location_id", &value.expected_location_id)?;
        if value.accepted_hardware_revisions.is_empty()
            || value.max_active_instances == 0
            || value
                .accepted_hardware_revisions
                .values()
                .any(BTreeSet::is_empty)
        {
            return Err(PhysicalIdentityError::InvalidSlotContract {
                slot_id: value.slot_id.clone(),
            });
        }
        Ok(value)
    }

    fn accepts(&self, instance: &PhysicalInstance) -> bool {
        self.accepted_hardware_revisions
            .get(&instance.product_id)
            .is_some_and(|revisions| revisions.contains(&instance.hardware_revision))
    }
}

/// Namespaced identity of one individual physical unit.
///
/// The namespace is part of identity. A serial number without its issuer/namespace is
/// not globally unique and must never be used as a stand-alone key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PhysicalInstance {
    physical_instance_id: String,
    identity_namespace: String,
    serial_identifier: String,
    product_id: String,
    hardware_revision: String,
    configuration_sha256: String,
}

impl PhysicalInstance {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        physical_instance_id: impl Into<String>,
        identity_namespace: impl Into<String>,
        serial_identifier: impl Into<String>,
        product_id: impl Into<String>,
        hardware_revision: impl Into<String>,
        configuration_sha256: impl Into<String>,
    ) -> Result<Self, PhysicalIdentityError> {
        let value = Self {
            physical_instance_id: physical_instance_id.into(),
            identity_namespace: identity_namespace.into(),
            serial_identifier: serial_identifier.into(),
            product_id: product_id.into(),
            hardware_revision: hardware_revision.into(),
            configuration_sha256: configuration_sha256.into(),
        };
        require_non_empty("physical_instance_id", &value.physical_instance_id)?;
        require_non_empty("identity_namespace", &value.identity_namespace)?;
        require_non_empty("serial_identifier", &value.serial_identifier)?;
        require_non_empty("product_id", &value.product_id)?;
        require_non_empty("hardware_revision", &value.hardware_revision)?;
        validate_sha256("configuration_sha256", &value.configuration_sha256)?;
        Ok(value)
    }

    pub fn physical_instance_id(&self) -> &str {
        &self.physical_instance_id
    }
}

/// An evidence reference admitted by an upstream evidence system.
///
/// This record binds an identifier, digest and source receipt reference. It does not
/// verify the bytes, signer, acquisition device, calibration, or truth of the content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EvidenceRecord {
    evidence_id: String,
    digest_sha256: String,
    source_receipt_id: String,
    captured_at_ms: i64,
    received_at_ms: i64,
}

impl EvidenceRecord {
    pub fn new(
        evidence_id: impl Into<String>,
        digest_sha256: impl Into<String>,
        source_receipt_id: impl Into<String>,
        captured_at_ms: i64,
        received_at_ms: i64,
    ) -> Result<Self, PhysicalIdentityError> {
        let value = Self {
            evidence_id: evidence_id.into(),
            digest_sha256: digest_sha256.into(),
            source_receipt_id: source_receipt_id.into(),
            captured_at_ms,
            received_at_ms,
        };
        require_non_empty("evidence_id", &value.evidence_id)?;
        require_non_empty("source_receipt_id", &value.source_receipt_id)?;
        validate_sha256("evidence_digest_sha256", &value.digest_sha256)?;
        if value.received_at_ms < value.captured_at_ms {
            return Err(PhysicalIdentityError::InvalidEvidenceTimeOrder {
                evidence_id: value.evidence_id.clone(),
            });
        }
        Ok(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PhysicalEventKind {
    DesignSlotDeclared,
    UnitDelivered,
    IdentityObserved,
    UnitInstalled,
    UnitRemoved,
    UnitReplaced,
    UnitRelocated,
    ConfigurationChanged,
    InspectionRecorded,
    IndependentReconciliationRecorded,
    ConflictDeclared,
}

/// Append-only lifecycle event. Optional fields are constrained by the event kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PhysicalEvent {
    pub event_id: String,
    pub event_kind: PhysicalEventKind,
    pub slot_id: String,
    pub actor_ref: String,
    pub occurred_at_ms: i64,
    pub recorded_at_ms: i64,
    pub physical_instance_id: Option<String>,
    pub previous_physical_instance_id: Option<String>,
    pub location_id: Option<String>,
    pub from_location_id: Option<String>,
    pub to_location_id: Option<String>,
    pub new_configuration_sha256: Option<String>,
    pub evidence_refs: Vec<String>,
    pub predecessor_event_ids: Vec<String>,
    pub review_attestation_ref: Option<String>,
}

impl PhysicalEvent {
    pub fn new(
        event_id: impl Into<String>,
        event_kind: PhysicalEventKind,
        slot_id: impl Into<String>,
        actor_ref: impl Into<String>,
        occurred_at_ms: i64,
        recorded_at_ms: i64,
        evidence_refs: Vec<String>,
    ) -> Self {
        Self {
            event_id: event_id.into(),
            event_kind,
            slot_id: slot_id.into(),
            actor_ref: actor_ref.into(),
            occurred_at_ms,
            recorded_at_ms,
            physical_instance_id: None,
            previous_physical_instance_id: None,
            location_id: None,
            from_location_id: None,
            to_location_id: None,
            new_configuration_sha256: None,
            evidence_refs,
            predecessor_event_ids: Vec::new(),
            review_attestation_ref: None,
        }
    }

    pub fn with_instance(mut self, instance_id: impl Into<String>) -> Self {
        self.physical_instance_id = Some(instance_id.into());
        self
    }

    pub fn with_previous_instance(mut self, instance_id: impl Into<String>) -> Self {
        self.previous_physical_instance_id = Some(instance_id.into());
        self
    }

    pub fn with_location(mut self, location_id: impl Into<String>) -> Self {
        self.location_id = Some(location_id.into());
        self
    }

    pub fn with_relocation(
        mut self,
        from_location_id: impl Into<String>,
        to_location_id: impl Into<String>,
    ) -> Self {
        self.from_location_id = Some(from_location_id.into());
        self.to_location_id = Some(to_location_id.into());
        self
    }

    pub fn with_configuration(mut self, digest: impl Into<String>) -> Self {
        self.new_configuration_sha256 = Some(digest.into());
        self
    }

    pub fn with_review_attestation(mut self, attestation_ref: impl Into<String>) -> Self {
        self.review_attestation_ref = Some(attestation_ref.into());
        self
    }

    pub fn with_predecessors(mut self, event_ids: Vec<String>) -> Self {
        self.predecessor_event_ids = event_ids;
        self
    }
}

/// The exact subject presented to the independent attestation verifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewScope {
    pub installation_id: String,
    pub installation_revision: String,
    pub slot_id: String,
    pub physical_instance_id: String,
    pub product_id: String,
    pub hardware_revision: String,
    pub configuration_sha256: String,
    pub location_id: String,
    pub graph_id: String,
    pub graph_revision: String,
    pub graph_payload_sha256: String,
}

/// Result returned by the separately owned review-attestation verifier.
///
/// Implementations must validate signature, signer identity, authority, jurisdiction,
/// scope, independence, expiry, revocation and the exact subject. This enum is an
/// integration boundary, not a cryptographic proof by itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewVerification {
    Valid,
    Invalid,
    Unknown,
}

pub trait IndependentReviewVerifier {
    fn verify(
        &self,
        attestation_ref: &str,
        scope: &ReviewScope,
        as_known_at_ms: i64,
    ) -> ReviewVerification;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SlotStatus {
    DeclaredOnly,
    DeliveredUnreconciled,
    InstalledUnreconciled,
    Reconciled,
    Conflicted,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PhysicalIdentityDiagnosticCode {
    EvidenceUnavailableAtCutoff,
    PredecessorUnavailableAtCutoff,
    EventAfterDecisionCutoff,
    InstallationWithoutDelivery,
    ProductNotAcceptedBySlot,
    HardwareRevisionNotAcceptedBySlot,
    PhysicalInstanceAssignedToMultipleSlots,
    ActiveInstanceLimitExceeded,
    RemovalDoesNotMatchActiveAssignment,
    ReplacementDoesNotMatchActiveAssignment,
    ReplacementUnitNotDelivered,
    RelocationSourceDoesNotMatch,
    ConfigurationChangeWithoutActiveAssignment,
    InspectionWithoutActiveAssignment,
    ReviewWithoutInspection,
    ReviewRejected,
    ReviewAuthorityUnknown,
    IdentityObservationWithoutDeliveryOrAssignment,
    ExplicitConflictDeclared,
    ActiveInstanceLimitExceeded,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct PhysicalIdentityDiagnostic {
    pub code: PhysicalIdentityDiagnosticCode,
    pub event_id: String,
    pub slot_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ActivePhysicalAssignment {
    pub slot_id: String,
    pub physical_instance_id: String,
    pub product_id: String,
    pub hardware_revision: String,
    pub configuration_sha256: String,
    pub location_id: String,
    pub reconciled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SlotProjection {
    pub slot_id: String,
    pub status: SlotStatus,
    pub active_assignments: Vec<ActivePhysicalAssignment>,
    pub diagnostics: Vec<PhysicalIdentityDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InstallationProjection {
    pub installation_id: String,
    pub installation_revision: String,
    pub graph_id: String,
    pub graph_revision: String,
    pub graph_payload_sha256: String,
    pub as_known_at_ms: i64,
    pub slots: BTreeMap<String, SlotProjection>,
    pub processed_event_ids: Vec<String>,
    pub diagnostics: Vec<PhysicalIdentityDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeAssignment {
    physical_instance_id: String,
    location_id: String,
    configuration_sha256: String,
    inspected: bool,
    reconciled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct SlotRuntime {
    declared: bool,
    delivered: BTreeSet<String>,
    active: BTreeMap<String, RuntimeAssignment>,
    conflicted: bool,
    unknown: bool,
    diagnostics: BTreeSet<PhysicalIdentityDiagnostic>,
}

pub struct PhysicalIdentityLedger {
    snapshot: InstallationSnapshot,
    products: BTreeMap<String, ProductModel>,
    slots: BTreeMap<String, DesignSlot>,
    instances: BTreeMap<String, PhysicalInstance>,
    evidence: BTreeMap<String, EvidenceRecord>,
    events: BTreeMap<String, PhysicalEvent>,
    ordered_event_ids: Vec<String>,
}

impl PhysicalIdentityLedger {
    pub fn new(
        snapshot: InstallationSnapshot,
        products: Vec<ProductModel>,
        slots: Vec<DesignSlot>,
        instances: Vec<PhysicalInstance>,
        evidence: Vec<EvidenceRecord>,
        events: Vec<PhysicalEvent>,
    ) -> Result<Self, PhysicalIdentityError> {
        let mut product_map = BTreeMap::new();
        for product in products {
            let id = product.product_id.clone();
            if product_map.insert(id.clone(), product).is_some() {
                return Err(PhysicalIdentityError::DuplicateProduct { product_id: id });
            }
        }

        let mut slot_map = BTreeMap::new();
        for slot in slots {
            let id = slot.slot_id.clone();
            if slot_map.insert(id.clone(), slot).is_some() {
                return Err(PhysicalIdentityError::DuplicateSlot { slot_id: id });
            }
        }

        for slot in slot_map.values() {
            if slot
                .accepted_hardware_revisions
                .keys()
                .any(|product_id| !product_map.contains_key(product_id))
            {
                let product_id = slot
                    .accepted_hardware_revisions
                    .keys()
                    .find(|product_id| !product_map.contains_key(*product_id))
                    .expect("missing product was found");
                return Err(PhysicalIdentityError::UnknownProduct {
                    product_id: product_id.clone(),
                });
            }
        }

        let mut instance_map = BTreeMap::new();
        let mut serial_keys = BTreeSet::new();
        for instance in instances {
            if !product_map.contains_key(&instance.product_id) {
                return Err(PhysicalIdentityError::UnknownProduct {
                    product_id: instance.product_id.clone(),
                });
            }
            let serial_key = (
                instance.identity_namespace.clone(),
                instance.serial_identifier.clone(),
            );
            if !serial_keys.insert(serial_key.clone()) {
                return Err(PhysicalIdentityError::DuplicateSerialIdentity {
                    namespace: serial_key.0,
                    serial: serial_key.1,
                });
            }
            let id = instance.physical_instance_id.clone();
            if instance_map.insert(id.clone(), instance).is_some() {
                return Err(PhysicalIdentityError::DuplicatePhysicalInstance {
                    instance_id: id,
                });
            }
        }

        let mut evidence_map = BTreeMap::new();
        for record in evidence {
            let id = record.evidence_id.clone();
            if evidence_map.insert(id.clone(), record).is_some() {
                return Err(PhysicalIdentityError::DuplicateEvidence { evidence_id: id });
            }
        }

        let mut event_map = BTreeMap::new();
        for event in events {
            validate_event_shape(&event)?;
            if !slot_map.contains_key(&event.slot_id) {
                return Err(PhysicalIdentityError::UnknownSlot {
                    slot_id: event.slot_id.clone(),
                });
            }
            if let Some(instance_id) = &event.physical_instance_id {
                if !instance_map.contains_key(instance_id) {
                    return Err(PhysicalIdentityError::UnknownPhysicalInstance {
                        instance_id: instance_id.clone(),
                    });
                }
            }
            if let Some(instance_id) = &event.previous_physical_instance_id {
                if !instance_map.contains_key(instance_id) {
                    return Err(PhysicalIdentityError::UnknownPhysicalInstance {
                        instance_id: instance_id.clone(),
                    });
                }
            }
            for evidence_id in &event.evidence_refs {
                if !evidence_map.contains_key(evidence_id) {
                    return Err(PhysicalIdentityError::UnknownEvidence {
                        evidence_id: evidence_id.clone(),
                    });
                }
            }
            let id = event.event_id.clone();
            if event_map.insert(id.clone(), event).is_some() {
                return Err(PhysicalIdentityError::DuplicateEvent { event_id: id });
            }
        }

        for event in event_map.values() {
            for predecessor_id in &event.predecessor_event_ids {
                let Some(predecessor) = event_map.get(predecessor_id) else {
                    return Err(PhysicalIdentityError::UnknownPredecessor {
                        event_id: event.event_id.clone(),
                        predecessor_id: predecessor_id.clone(),
                    });
                };
                if predecessor.occurred_at_ms > event.occurred_at_ms {
                    return Err(PhysicalIdentityError::PredecessorAfterEvent {
                        event_id: event.event_id.clone(),
                        predecessor_id: predecessor_id.clone(),
                    });
                }
            }
        }

        let ordered_event_ids = topological_event_order(&event_map)?;
        Ok(Self {
            snapshot,
            products: product_map,
            slots: slot_map,
            instances: instance_map,
            evidence: evidence_map,
            events: event_map,
            ordered_event_ids,
        })
    }

    pub fn project(
        &self,
        as_known_at_ms: i64,
        review_verifier: &dyn IndependentReviewVerifier,
    ) -> InstallationProjection {
        let mut runtime: BTreeMap<String, SlotRuntime> = self
            .slots
            .keys()
            .map(|slot_id| (slot_id.clone(), SlotRuntime::default()))
            .collect();
        let mut active_owners: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut processed = BTreeSet::new();
        let mut global_diagnostics = BTreeSet::new();

        for event_id in &self.ordered_event_ids {
            let event = self.events.get(event_id).expect("ordered event exists");
            if event.recorded_at_ms > as_known_at_ms {
                continue;
            }
            if event.occurred_at_ms > as_known_at_ms {
                push_diagnostic(
                    &mut runtime,
                    &mut global_diagnostics,
                    event,
                    PhysicalIdentityDiagnosticCode::EventAfterDecisionCutoff,
                    true,
                );
                continue;
            }
            if event.evidence_refs.iter().any(|evidence_id| {
                self.evidence
                    .get(evidence_id)
                    .is_none_or(|record| record.received_at_ms > as_known_at_ms)
            }) {
                push_diagnostic(
                    &mut runtime,
                    &mut global_diagnostics,
                    event,
                    PhysicalIdentityDiagnosticCode::EvidenceUnavailableAtCutoff,
                    true,
                );
                continue;
            }
            if event
                .predecessor_event_ids
                .iter()
                .any(|predecessor_id| !processed.contains(predecessor_id))
            {
                push_diagnostic(
                    &mut runtime,
                    &mut global_diagnostics,
                    event,
                    PhysicalIdentityDiagnosticCode::PredecessorUnavailableAtCutoff,
                    true,
                );
                continue;
            }

            self.apply_event(
                event,
                as_known_at_ms,
                review_verifier,
                &mut runtime,
                &mut active_owners,
                &mut global_diagnostics,
            );
            processed.insert(event.event_id.clone());
        }

        for (instance_id, owners) in &active_owners {
            if owners.len() > 1 {
                for slot_id in owners {
                    if let Some(slot) = runtime.get_mut(slot_id) {
                        slot.conflicted = true;
                        let diagnostic = PhysicalIdentityDiagnostic {
                            code: PhysicalIdentityDiagnosticCode::PhysicalInstanceAssignedToMultipleSlots,
                            event_id: format!("instance:{instance_id}"),
                            slot_id: slot_id.clone(),
                        };
                        slot.diagnostics.insert(diagnostic.clone());
                        global_diagnostics.insert(diagnostic);
                    }
                }
            }
        }

        let mut slot_projections = BTreeMap::new();
        for (slot_id, mut state) in runtime {
            let slot_contract = self.slots.get(&slot_id).expect("slot exists");
            let mut assignments = state
                .active
                .values()
                .map(|assignment| {
                    let instance = self
                        .instances
                        .get(&assignment.physical_instance_id)
                        .expect("active instance exists");
                    ActivePhysicalAssignment {
                        slot_id: slot_id.clone(),
                        physical_instance_id: assignment.physical_instance_id.clone(),
                        product_id: instance.product_id.clone(),
                        hardware_revision: instance.hardware_revision.clone(),
                        configuration_sha256: assignment.configuration_sha256.clone(),
                        location_id: assignment.location_id.clone(),
                        reconciled: assignment.reconciled,
                    }
                })
                .collect::<Vec<_>>();
            assignments.sort_by(|left, right| {
                left.physical_instance_id.cmp(&right.physical_instance_id)
            });

            if assignments.len() > slot_contract.max_active_instances {
                state.conflicted = true;
                state.diagnostics.insert(PhysicalIdentityDiagnostic {
                    code: PhysicalIdentityDiagnosticCode::ActiveInstanceLimitExceeded,
                    event_id: format!("slot-limit:{}", slot_id),
                    slot_id: slot_id.clone(),
                });
            }

            let mut diagnostics = state.diagnostics.into_iter().collect::<Vec<_>>();
            diagnostics.sort();

            let status = if !state.declared {
                SlotStatus::Unknown
            } else if state.conflicted || assignments.len() > slot_contract.max_active_instances {
                SlotStatus::Conflicted
            } else if state.unknown {
                SlotStatus::Unknown
            } else if !assignments.is_empty() {
                if assignments.iter().all(|assignment| assignment.reconciled) {
                    SlotStatus::Reconciled
                } else {
                    SlotStatus::InstalledUnreconciled
                }
            } else if !state.delivered.is_empty() {
                SlotStatus::DeliveredUnreconciled
            } else {
                SlotStatus::DeclaredOnly
            };

            slot_projections.insert(
                slot_id.clone(),
                SlotProjection {
                    slot_id,
                    status,
                    active_assignments: assignments,
                    diagnostics,
                },
            );
        }

        let mut processed_event_ids = processed.into_iter().collect::<Vec<_>>();
        processed_event_ids.sort();
        InstallationProjection {
            installation_id: self.snapshot.installation_id.clone(),
            installation_revision: self.snapshot.installation_revision.clone(),
            graph_id: self.snapshot.graph_id.clone(),
            graph_revision: self.snapshot.graph_revision.clone(),
            graph_payload_sha256: self.snapshot.graph_payload_sha256.clone(),
            as_known_at_ms,
            slots: slot_projections,
            processed_event_ids,
            diagnostics: global_diagnostics.into_iter().collect(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn apply_event(
        &self,
        event: &PhysicalEvent,
        as_known_at_ms: i64,
        review_verifier: &dyn IndependentReviewVerifier,
        runtime: &mut BTreeMap<String, SlotRuntime>,
        active_owners: &mut BTreeMap<String, BTreeSet<String>>,
        global_diagnostics: &mut BTreeSet<PhysicalIdentityDiagnostic>,
    ) {
        let slot_id = event.slot_id.as_str();
        match event.event_kind {
            PhysicalEventKind::DesignSlotDeclared => {
                runtime.get_mut(slot_id).expect("slot exists").declared = true;
            }
            PhysicalEventKind::UnitDelivered => {
                let instance_id = event.physical_instance_id.as_ref().expect("shape validated");
                let instance = self.instances.get(instance_id).expect("instance validated");
                if !self.slots[slot_id].accepts(instance) {
                    let code = if self.slots[slot_id]
                        .accepted_hardware_revisions
                        .contains_key(&instance.product_id)
                    {
                        PhysicalIdentityDiagnosticCode::HardwareRevisionNotAcceptedBySlot
                    } else {
                        PhysicalIdentityDiagnosticCode::ProductNotAcceptedBySlot
                    };
                    push_diagnostic(runtime, global_diagnostics, event, code, false);
                } else {
                    runtime
                        .get_mut(slot_id)
                        .expect("slot exists")
                        .delivered
                        .insert(instance_id.clone());
                }
            }
            PhysicalEventKind::IdentityObserved => {
                let instance_id = event.physical_instance_id.as_ref().expect("shape validated");
                let known = runtime.get(slot_id).is_some_and(|state| {
                    state.delivered.contains(instance_id) || state.active.contains_key(instance_id)
                });
                if !known {
                    push_diagnostic(
                        runtime,
                        global_diagnostics,
                        event,
                        PhysicalIdentityDiagnosticCode::IdentityObservationWithoutDeliveryOrAssignment,
                        true,
                    );
                }
            }
            PhysicalEventKind::UnitInstalled => {
                let instance_id = event.physical_instance_id.as_ref().expect("shape validated");
                let instance = self.instances.get(instance_id).expect("instance validated");
                let delivered = runtime
                    .get_mut(slot_id)
                    .expect("slot exists")
                    .delivered
                    .remove(instance_id);
                if !delivered {
                    push_diagnostic(
                        runtime,
                        global_diagnostics,
                        event,
                        PhysicalIdentityDiagnosticCode::InstallationWithoutDelivery,
                        true,
                    );
                } else if !self.slots[slot_id].accepts(instance) {
                    let code = if self.slots[slot_id]
                        .accepted_hardware_revisions
                        .contains_key(&instance.product_id)
                    {
                        PhysicalIdentityDiagnosticCode::HardwareRevisionNotAcceptedBySlot
                    } else {
                        PhysicalIdentityDiagnosticCode::ProductNotAcceptedBySlot
                    };
                    push_diagnostic(runtime, global_diagnostics, event, code, false);
                } else {
                    add_assignment(
                        runtime,
                        active_owners,
                        slot_id,
                        instance_id,
                        event.location_id.as_deref().expect("shape validated"),
                        &instance.configuration_sha256,
                    );
                }
            }
            PhysicalEventKind::UnitRemoved => {
                let instance_id = event.physical_instance_id.as_ref().expect("shape validated");
                let location_id = event.location_id.as_ref().expect("shape validated");
                let matches = runtime.get(slot_id).is_some_and(|state| {
                    state.active.get(instance_id).is_some_and(|assignment| {
                        assignment.location_id == *location_id
                    })
                });
                if matches {
                    runtime
                        .get_mut(slot_id)
                        .expect("slot exists")
                        .active
                        .remove(instance_id);
                    if let Some(owners) = active_owners.get_mut(instance_id) {
                        owners.remove(slot_id);
                    }
                } else {
                    push_diagnostic(
                        runtime,
                        global_diagnostics,
                        event,
                        PhysicalIdentityDiagnosticCode::RemovalDoesNotMatchActiveAssignment,
                        false,
                    );
                }
            }
            PhysicalEventKind::UnitReplaced => {
                let old_id = event.previous_physical_instance_id.as_ref().expect("shape validated");
                let new_id = event.physical_instance_id.as_ref().expect("shape validated");
                let location_id = event.location_id.as_ref().expect("shape validated");
                let old_matches = runtime.get(slot_id).is_some_and(|state| {
                    state.active.get(old_id).is_some_and(|assignment| {
                        assignment.location_id == *location_id
                    })
                });
                if !old_matches {
                    push_diagnostic(
                        runtime,
                        global_diagnostics,
                        event,
                        PhysicalIdentityDiagnosticCode::ReplacementDoesNotMatchActiveAssignment,
                        false,
                    );
                } else {
                    let delivered = runtime
                        .get_mut(slot_id)
                        .expect("slot exists")
                        .delivered
                        .remove(new_id);
                    if !delivered {
                        push_diagnostic(
                            runtime,
                            global_diagnostics,
                            event,
                            PhysicalIdentityDiagnosticCode::ReplacementUnitNotDelivered,
                            true,
                        );
                    } else {
                        runtime
                            .get_mut(slot_id)
                            .expect("slot exists")
                            .active
                            .remove(old_id);
                        if let Some(owners) = active_owners.get_mut(old_id) {
                            owners.remove(slot_id);
                        }
                        let new_instance = self.instances.get(new_id).expect("instance validated");
                        if !self.slots[slot_id].accepts(new_instance) {
                            let code = if self.slots[slot_id]
                                .accepted_hardware_revisions
                                .contains_key(&new_instance.product_id)
                            {
                                PhysicalIdentityDiagnosticCode::HardwareRevisionNotAcceptedBySlot
                            } else {
                                PhysicalIdentityDiagnosticCode::ProductNotAcceptedBySlot
                            };
                            push_diagnostic(runtime, global_diagnostics, event, code, false);
                        } else {
                            add_assignment(
                                runtime,
                                active_owners,
                                slot_id,
                                new_id,
                                location_id,
                                &new_instance.configuration_sha256,
                            );
                        }
                    }
                }
            }
            PhysicalEventKind::UnitRelocated => {
                let instance_id = event.physical_instance_id.as_ref().expect("shape validated");
                let from = event.from_location_id.as_ref().expect("shape validated");
                let to = event.to_location_id.as_ref().expect("shape validated");
                let outcome = if let Some(assignment) = runtime
                    .get_mut(slot_id)
                    .and_then(|state| state.active.get_mut(instance_id))
                {
                    if assignment.location_id == *from {
                        assignment.location_id = to.clone();
                        assignment.inspected = false;
                        assignment.reconciled = false;
                        None
                    } else {
                        Some(false)
                    }
                } else {
                    Some(true)
                };
                if let Some(unknown) = outcome {
                    push_diagnostic(
                        runtime,
                        global_diagnostics,
                        event,
                        PhysicalIdentityDiagnosticCode::RelocationSourceDoesNotMatch,
                        unknown,
                    );
                }
            }
            PhysicalEventKind::ConfigurationChanged => {
                let instance_id = event.physical_instance_id.as_ref().expect("shape validated");
                let digest = event.new_configuration_sha256.as_ref().expect("shape validated");
                let found = if let Some(assignment) = runtime
                    .get_mut(slot_id)
                    .and_then(|state| state.active.get_mut(instance_id))
                {
                    assignment.configuration_sha256 = digest.clone();
                    assignment.inspected = false;
                    assignment.reconciled = false;
                    true
                } else {
                    false
                };
                if !found {
                    push_diagnostic(
                        runtime,
                        global_diagnostics,
                        event,
                        PhysicalIdentityDiagnosticCode::ConfigurationChangeWithoutActiveAssignment,
                        true,
                    );
                }
            }
            PhysicalEventKind::InspectionRecorded => {
                let instance_id = event.physical_instance_id.as_ref().expect("shape validated");
                let location_id = event.location_id.as_ref().expect("shape validated");
                let outcome = if let Some(assignment) = runtime
                    .get_mut(slot_id)
                    .and_then(|state| state.active.get_mut(instance_id))
                {
                    if assignment.location_id == *location_id {
                        assignment.inspected = true;
                        assignment.reconciled = false;
                        None
                    } else {
                        Some(false)
                    }
                } else {
                    Some(true)
                };
                if let Some(unknown) = outcome {
                    push_diagnostic(
                        runtime,
                        global_diagnostics,
                        event,
                        PhysicalIdentityDiagnosticCode::InspectionWithoutActiveAssignment,
                        unknown,
                    );
                }
            }
            PhysicalEventKind::IndependentReconciliationRecorded => {
                let instance_id = event.physical_instance_id.as_ref().expect("shape validated");
                let attestation_ref = event.review_attestation_ref.as_ref().expect("shape validated");
                let event_location = event.location_id.as_ref().expect("shape validated");
                let Some(assignment) = runtime
                    .get(slot_id)
                    .and_then(|state| state.active.get(instance_id))
                    .cloned()
                else {
                    push_diagnostic(
                        runtime,
                        global_diagnostics,
                        event,
                        PhysicalIdentityDiagnosticCode::ReviewWithoutInspection,
                        true,
                    );
                    return;
                };
                if !assignment.inspected || assignment.location_id != *event_location {
                    push_note(
                        runtime,
                        global_diagnostics,
                        event,
                        PhysicalIdentityDiagnosticCode::ReviewWithoutInspection,
                    );
                    return;
                }
                let instance = self.instances.get(instance_id).expect("instance validated");
                let scope = ReviewScope {
                    installation_id: self.snapshot.installation_id.clone(),
                    installation_revision: self.snapshot.installation_revision.clone(),
                    slot_id: slot_id.to_owned(),
                    physical_instance_id: instance_id.clone(),
                    product_id: instance.product_id.clone(),
                    hardware_revision: instance.hardware_revision.clone(),
                    configuration_sha256: assignment.configuration_sha256,
                    location_id: assignment.location_id,
                    graph_id: self.snapshot.graph_id.clone(),
                    graph_revision: self.snapshot.graph_revision.clone(),
                    graph_payload_sha256: self.snapshot.graph_payload_sha256.clone(),
                };
                match review_verifier.verify(attestation_ref, &scope, as_known_at_ms) {
                    ReviewVerification::Valid => {
                        if let Some(assignment) = runtime
                            .get_mut(slot_id)
                            .and_then(|state| state.active.get_mut(instance_id))
                        {
                            assignment.reconciled = true;
                        }
                    }
                    ReviewVerification::Invalid => push_note(
                        runtime,
                        global_diagnostics,
                        event,
                        PhysicalIdentityDiagnosticCode::ReviewRejected,
                    ),
                    ReviewVerification::Unknown => push_diagnostic(
                        runtime,
                        global_diagnostics,
                        event,
                        PhysicalIdentityDiagnosticCode::ReviewAuthorityUnknown,
                        true,
                    ),
                }
            }
            PhysicalEventKind::ConflictDeclared => push_diagnostic(
                runtime,
                global_diagnostics,
                event,
                PhysicalIdentityDiagnosticCode::ExplicitConflictDeclared,
                false,
            ),
        }
    }
}

fn add_assignment(
    runtime: &mut BTreeMap<String, SlotRuntime>,
    active_owners: &mut BTreeMap<String, BTreeSet<String>>,
    slot_id: &str,
    instance_id: &str,
    location_id: &str,
    configuration_sha256: &str,
) {
    let state = runtime.get_mut(slot_id).expect("slot exists");
    if state.active.contains_key(instance_id) {
        state.conflicted = true;
        return;
    }
    state.active.insert(
        instance_id.to_owned(),
        RuntimeAssignment {
            physical_instance_id: instance_id.to_owned(),
            location_id: location_id.to_owned(),
            configuration_sha256: configuration_sha256.to_owned(),
            inspected: false,
            reconciled: false,
        },
    );
    active_owners
        .entry(instance_id.to_owned())
        .or_default()
        .insert(slot_id.to_owned());
}

fn push_note(
    runtime: &mut BTreeMap<String, SlotRuntime>,
    global: &mut BTreeSet<PhysicalIdentityDiagnostic>,
    event: &PhysicalEvent,
    code: PhysicalIdentityDiagnosticCode,
) {
    let diagnostic = PhysicalIdentityDiagnostic {
        code,
        event_id: event.event_id.clone(),
        slot_id: event.slot_id.clone(),
    };
    if let Some(state) = runtime.get_mut(&event.slot_id) {
        state.diagnostics.insert(diagnostic.clone());
    }
    global.insert(diagnostic);
}

fn push_diagnostic(
    runtime: &mut BTreeMap<String, SlotRuntime>,
    global: &mut BTreeSet<PhysicalIdentityDiagnostic>,
    event: &PhysicalEvent,
    code: PhysicalIdentityDiagnosticCode,
    unknown: bool,
) {
    let diagnostic = PhysicalIdentityDiagnostic {
        code,
        event_id: event.event_id.clone(),
        slot_id: event.slot_id.clone(),
    };
    if let Some(state) = runtime.get_mut(&event.slot_id) {
        if unknown {
            state.unknown = true;
        } else {
            state.conflicted = true;
        }
        state.diagnostics.insert(diagnostic.clone());
    }
    global.insert(diagnostic);
}

fn validate_event_shape(event: &PhysicalEvent) -> Result<(), PhysicalIdentityError> {
    require_non_empty("event_id", &event.event_id)?;
    require_non_empty("slot_id", &event.slot_id)?;
    require_non_empty("actor_ref", &event.actor_ref)?;
    if event.recorded_at_ms < event.occurred_at_ms {
        return Err(PhysicalIdentityError::InvalidTimeOrder {
            event_id: event.event_id.clone(),
        });
    }
    if event.evidence_refs.is_empty() {
        return Err(PhysicalIdentityError::InvalidEventShape {
            event_id: event.event_id.clone(),
            reason: "event requires at least one evidence reference",
        });
    }
    let needs_instance = matches!(
        event.event_kind,
        PhysicalEventKind::UnitDelivered
            | PhysicalEventKind::IdentityObserved
            | PhysicalEventKind::UnitInstalled
            | PhysicalEventKind::UnitRemoved
            | PhysicalEventKind::UnitReplaced
            | PhysicalEventKind::UnitRelocated
            | PhysicalEventKind::ConfigurationChanged
            | PhysicalEventKind::InspectionRecorded
            | PhysicalEventKind::IndependentReconciliationRecorded
    );
    if needs_instance && event.physical_instance_id.is_none() {
        return Err(PhysicalIdentityError::InvalidEventShape {
            event_id: event.event_id.clone(),
            reason: "event kind requires physical_instance_id",
        });
    }
    match event.event_kind {
        PhysicalEventKind::UnitInstalled
        | PhysicalEventKind::UnitRemoved
        | PhysicalEventKind::InspectionRecorded => {
            if event.location_id.as_deref().is_none_or(str::is_empty) {
                return Err(PhysicalIdentityError::InvalidEventShape {
                    event_id: event.event_id.clone(),
                    reason: "event kind requires location_id",
                });
            }
        }
        PhysicalEventKind::UnitReplaced => {
            if event.previous_physical_instance_id.is_none()
                || event.location_id.as_deref().is_none_or(str::is_empty)
            {
                return Err(PhysicalIdentityError::InvalidEventShape {
                    event_id: event.event_id.clone(),
                    reason: "replacement requires previous instance and location",
                });
            }
        }
        PhysicalEventKind::UnitRelocated => {
            if event.from_location_id.as_deref().is_none_or(str::is_empty)
                || event.to_location_id.as_deref().is_none_or(str::is_empty)
            {
                return Err(PhysicalIdentityError::InvalidEventShape {
                    event_id: event.event_id.clone(),
                    reason: "relocation requires source and destination locations",
                });
            }
        }
        PhysicalEventKind::ConfigurationChanged => {
            let Some(digest) = &event.new_configuration_sha256 else {
                return Err(PhysicalIdentityError::InvalidEventShape {
                    event_id: event.event_id.clone(),
                    reason: "configuration change requires new configuration digest",
                });
            };
            validate_sha256("new_configuration_sha256", digest)?;
        }
        PhysicalEventKind::IndependentReconciliationRecorded => {
            if event
                .review_attestation_ref
                .as_deref()
                .is_none_or(str::is_empty)
                || event.location_id.as_deref().is_none_or(str::is_empty)
            {
                return Err(PhysicalIdentityError::InvalidEventShape {
                    event_id: event.event_id.clone(),
                    reason: "reconciliation requires review reference and location",
                });
            }
        }
        _ => {}
    }
    if let Some(instance_id) = &event.physical_instance_id {
        require_non_empty("physical_instance_id", instance_id)?;
    }
    if let Some(previous_id) = &event.previous_physical_instance_id {
        require_non_empty("previous_physical_instance_id", previous_id)?;
    }
    for evidence_id in &event.evidence_refs {
        require_non_empty("evidence_ref", evidence_id)?;
    }
    let unique_predecessors = event
        .predecessor_event_ids
        .iter()
        .collect::<BTreeSet<_>>();
    if unique_predecessors.len() != event.predecessor_event_ids.len() {
        return Err(PhysicalIdentityError::InvalidEventShape {
            event_id: event.event_id.clone(),
            reason: "duplicate predecessor event ID",
        });
    }
    Ok(())
}

fn topological_event_order(
    events: &BTreeMap<String, PhysicalEvent>,
) -> Result<Vec<String>, PhysicalIdentityError> {
    let mut indegree = events
        .keys()
        .map(|event_id| (event_id.clone(), 0usize))
        .collect::<BTreeMap<_, _>>();
    let mut children: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for event in events.values() {
        for predecessor_id in &event.predecessor_event_ids {
            if !events.contains_key(predecessor_id) {
                return Err(PhysicalIdentityError::UnknownPredecessor {
                    event_id: event.event_id.clone(),
                    predecessor_id: predecessor_id.clone(),
                });
            }
            *indegree
                .get_mut(&event.event_id)
                .expect("event indegree exists") += 1;
            children
                .entry(predecessor_id.clone())
                .or_default()
                .insert(event.event_id.clone());
        }
    }

    let mut ready = BTreeSet::new();
    for (event_id, degree) in &indegree {
        if *degree == 0 {
            let event = events.get(event_id).expect("event exists");
            ready.insert((
                event.occurred_at_ms,
                event.recorded_at_ms,
                event_id.clone(),
            ));
        }
    }

    let mut ordered = Vec::with_capacity(events.len());
    while let Some((_, _, event_id)) = ready.pop_first() {
        ordered.push(event_id.clone());
        if let Some(dependents) = children.get(&event_id) {
            for child_id in dependents {
                let degree = indegree.get_mut(child_id).expect("child exists");
                *degree -= 1;
                if *degree == 0 {
                    let event = events.get(child_id).expect("event exists");
                    ready.insert((
                        event.occurred_at_ms,
                        event.recorded_at_ms,
                        child_id.clone(),
                    ));
                }
            }
        }
    }

    if ordered.len() != events.len() {
        return Err(PhysicalIdentityError::EventDependencyCycle);
    }
    Ok(ordered)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D1: &str = "1111111111111111111111111111111111111111111111111111111111111111";
    const D2: &str = "2222222222222222222222222222222222222222222222222222222222222222";
    const D3: &str = "3333333333333333333333333333333333333333333333333333333333333333";

    struct TestVerifier(ReviewVerification);

    impl IndependentReviewVerifier for TestVerifier {
        fn verify(
            &self,
            _attestation_ref: &str,
            _scope: &ReviewScope,
            _as_known_at_ms: i64,
        ) -> ReviewVerification {
            self.0
        }
    }

    fn event(
        id: &str,
        kind: PhysicalEventKind,
        slot: &str,
        at: i64,
        refs: &[&str],
    ) -> PhysicalEvent {
        PhysicalEvent::new(
            id,
            kind,
            slot,
            "actor-test",
            at,
            at,
            refs.iter().map(|value| (*value).to_owned()).collect(),
        )
    }

    fn ledger(events: Vec<PhysicalEvent>) -> PhysicalIdentityLedger {
        let snapshot = InstallationSnapshot::new(
            "installation-test",
            "revision-1",
            D1,
            "graph-test",
            "graph-revision-1",
            D2,
        )
        .unwrap();
        let product = ProductModel::new("product-a", "manufacturer-a", "model-a", "catalogue-1")
            .unwrap();
        let mut accepted = BTreeMap::new();
        accepted.insert(
            "product-a".to_owned(),
            ["A".to_owned()].into_iter().collect(),
        );
        let slot = DesignSlot::new("slot-1", "cabinet-east", accepted, 1).unwrap();
        let instance = PhysicalInstance::new(
            "instance-1",
            "manufacturer-a",
            "serial-1",
            "product-a",
            "A",
            D3,
        )
        .unwrap();
        let evidence = EvidenceRecord::new("evidence-1", D1, "source-receipt-1", 1, 1)
            .unwrap();
        PhysicalIdentityLedger::new(
            snapshot,
            vec![product],
            vec![slot],
            vec![instance],
            vec![evidence],
            events,
        )
        .unwrap()
    }

    fn declared_and_delivered() -> Vec<PhysicalEvent> {
        vec![
            event("declare", PhysicalEventKind::DesignSlotDeclared, "slot-1", 10, &["evidence-1"]),
            event("deliver", PhysicalEventKind::UnitDelivered, "slot-1", 20, &["evidence-1"])
                .with_instance("instance-1")
                .with_predecessors(vec!["declare".to_owned()]),
        ]
    }

    #[test]
    fn delivery_and_label_observation_do_not_reconcile_installation() {
        let mut events = declared_and_delivered();
        events.push(
            event("label", PhysicalEventKind::IdentityObserved, "slot-1", 30, &["evidence-1"])
                .with_instance("instance-1")
                .with_predecessors(vec!["deliver".to_owned()]),
        );
        let projection = ledger(events).project(100, &TestVerifier(ReviewVerification::Valid));
        assert_eq!(
            projection.slots["slot-1"].status,
            SlotStatus::DeliveredUnreconciled
        );
    }

    #[test]
    fn installation_without_independent_review_remains_unreconciled() {
        let mut events = declared_and_delivered();
        events.push(
            event("install", PhysicalEventKind::UnitInstalled, "slot-1", 30, &["evidence-1"])
                .with_instance("instance-1")
                .with_location("cabinet-east")
                .with_predecessors(vec!["deliver".to_owned()]),
        );
        let projection = ledger(events).project(100, &TestVerifier(ReviewVerification::Valid));
        assert_eq!(
            projection.slots["slot-1"].status,
            SlotStatus::InstalledUnreconciled
        );
    }

    #[test]
    fn verified_review_requires_a_prior_inspection() {
        let mut events = declared_and_delivered();
        events.push(
            event("install", PhysicalEventKind::UnitInstalled, "slot-1", 30, &["evidence-1"])
                .with_instance("instance-1")
                .with_location("cabinet-east")
                .with_predecessors(vec!["deliver".to_owned()]),
        );
        events.push(
            event(
                "review",
                PhysicalEventKind::IndependentReconciliationRecorded,
                "slot-1",
                40,
                &["evidence-1"],
            )
            .with_instance("instance-1")
            .with_location("cabinet-east")
            .with_review_attestation("attestation-1")
            .with_predecessors(vec!["install".to_owned()]),
        );
        let projection = ledger(events).project(100, &TestVerifier(ReviewVerification::Valid));
        assert_eq!(
            projection.slots["slot-1"].status,
            SlotStatus::InstalledUnreconciled
        );
        assert_eq!(
            projection.slots["slot-1"].diagnostics[0].code,
            PhysicalIdentityDiagnosticCode::ReviewWithoutInspection
        );
    }

    #[test]
    fn reconciliation_requires_an_external_valid_review_result() {
        let mut events = declared_and_delivered();
        events.push(
            event("install", PhysicalEventKind::UnitInstalled, "slot-1", 30, &["evidence-1"])
                .with_instance("instance-1")
                .with_location("cabinet-east")
                .with_predecessors(vec!["deliver".to_owned()]),
        );
        events.push(
            event("inspect", PhysicalEventKind::InspectionRecorded, "slot-1", 40, &["evidence-1"])
                .with_instance("instance-1")
                .with_location("cabinet-east")
                .with_predecessors(vec!["install".to_owned()]),
        );
        events.push(
            event(
                "review",
                PhysicalEventKind::IndependentReconciliationRecorded,
                "slot-1",
                50,
                &["evidence-1"],
            )
            .with_instance("instance-1")
            .with_location("cabinet-east")
            .with_review_attestation("attestation-1")
            .with_predecessors(vec!["inspect".to_owned()]),
        );

        let valid = ledger(events.clone())
            .project(100, &TestVerifier(ReviewVerification::Valid));
        assert_eq!(valid.slots["slot-1"].status, SlotStatus::Reconciled);

        let unknown = ledger(events).project(100, &TestVerifier(ReviewVerification::Unknown));
        assert_eq!(unknown.slots["slot-1"].status, SlotStatus::Unknown);
    }

    #[test]
    fn late_evidence_is_not_visible_to_historical_cutoff() {
        let mut events = declared_and_delivered();
        events.push(
            event("install", PhysicalEventKind::UnitInstalled, "slot-1", 30, &["evidence-1"])
                .with_instance("instance-1")
                .with_location("cabinet-east")
                .with_predecessors(vec!["deliver".to_owned()]),
        );
        events.push(
            event("inspect", PhysicalEventKind::InspectionRecorded, "slot-1", 40, &["evidence-1"])
                .with_instance("instance-1")
                .with_location("cabinet-east")
                .with_predecessors(vec!["install".to_owned()]),
        );
        events.push(
            event(
                "review",
                PhysicalEventKind::IndependentReconciliationRecorded,
                "slot-1",
                50,
                &["evidence-1"],
            )
            .with_instance("instance-1")
            .with_location("cabinet-east")
            .with_review_attestation("attestation-1")
            .with_predecessors(vec!["inspect".to_owned()]),
        );

        let mut ledger = ledger(events);
        ledger.evidence.get_mut("evidence-1").unwrap().received_at_ms = 60;
        let verifier = TestVerifier(ReviewVerification::Valid);
        let before = ledger.project(55, &verifier);
        let after = ledger.project(70, &verifier);
        assert_eq!(
            before.slots["slot-1"].status,
            SlotStatus::InstalledUnreconciled
        );
        assert_eq!(after.slots["slot-1"].status, SlotStatus::Reconciled);
    }

    #[test]
    fn replacement_resets_inspection_and_review_state() {
        let mut events = declared_and_delivered();
        events.push(
            event("install", PhysicalEventKind::UnitInstalled, "slot-1", 30, &["evidence-1"])
                .with_instance("instance-1")
                .with_location("cabinet-east")
                .with_predecessors(vec!["deliver".to_owned()]),
        );
        events.push(
            event("inspect", PhysicalEventKind::InspectionRecorded, "slot-1", 40, &["evidence-1"])
                .with_instance("instance-1")
                .with_location("cabinet-east")
                .with_predecessors(vec!["install".to_owned()]),
        );
        events.push(
            event("review", PhysicalEventKind::IndependentReconciliationRecorded, "slot-1", 50, &["evidence-1"])
                .with_instance("instance-1")
                .with_location("cabinet-east")
                .with_review_attestation("attestation-1")
                .with_predecessors(vec!["inspect".to_owned()]),
        );
        // This test intentionally stops before the replacement event because the fixture
        // helper has one physical instance. It still proves that a new install resets state
        // when an assignment is removed and then reintroduced.
        events.push(
            event("remove", PhysicalEventKind::UnitRemoved, "slot-1", 60, &["evidence-1"])
                .with_instance("instance-1")
                .with_location("cabinet-east")
                .with_predecessors(vec!["review".to_owned()]),
        );
        events.push(
            event("redeliver", PhysicalEventKind::UnitDelivered, "slot-1", 70, &["evidence-1"])
                .with_instance("instance-1")
                .with_predecessors(vec!["remove".to_owned()]),
        );
        events.push(
            event("reinstall", PhysicalEventKind::UnitInstalled, "slot-1", 80, &["evidence-1"])
                .with_instance("instance-1")
                .with_location("cabinet-east")
                .with_predecessors(vec!["redeliver".to_owned()]),
        );
        let projection = ledger(events).project(100, &TestVerifier(ReviewVerification::Valid));
        assert_eq!(
            projection.slots["slot-1"].status,
            SlotStatus::InstalledUnreconciled
        );
    }

    #[test]
    fn event_array_permutation_does_not_change_projection() {
        let mut events = declared_and_delivered();
        events.push(
            event("install", PhysicalEventKind::UnitInstalled, "slot-1", 30, &["evidence-1"])
                .with_instance("instance-1")
                .with_location("cabinet-east")
                .with_predecessors(vec!["deliver".to_owned()]),
        );
        let first = ledger(events.clone()).project(100, &TestVerifier(ReviewVerification::Unknown));
        events.reverse();
        let second = ledger(events).project(100, &TestVerifier(ReviewVerification::Unknown));
        assert_eq!(first, second);
    }

    #[test]
    fn duplicate_namespaced_serial_is_rejected() {
        let snapshot = InstallationSnapshot::new(
            "installation-test", "revision-1", D1, "graph-test", "graph-revision-1", D2,
        )
        .unwrap();
        let product = ProductModel::new("product-a", "manufacturer-a", "model-a", "catalogue-1")
            .unwrap();
        let mut accepted = BTreeMap::new();
        accepted.insert("product-a".to_owned(), ["A".to_owned()].into_iter().collect());
        let slot = DesignSlot::new("slot-1", "cabinet-east", accepted, 1).unwrap();
        let one = PhysicalInstance::new("instance-1", "issuer-a", "serial-1", "product-a", "A", D3).unwrap();
        let two = PhysicalInstance::new("instance-2", "issuer-a", "serial-1", "product-a", "A", D3).unwrap();
        let result = PhysicalIdentityLedger::new(
            snapshot, vec![product], vec![slot], vec![one, two],
            vec![EvidenceRecord::new("evidence-1", D1, "receipt-1", 1, 1).unwrap()], vec![],
        );
        assert!(matches!(result, Err(PhysicalIdentityError::DuplicateSerialIdentity { .. })));
    }

    #[test]
    fn unknown_predecessor_and_bad_digest_fail_closed() {
        let bad = PhysicalInstance::new(
            "instance-1", "issuer-a", "serial-1", "product-a", "A", "not-a-digest",
        );
        assert!(matches!(bad, Err(PhysicalIdentityError::InvalidSha256 { .. })));

        let event = event(
            "orphan",
            PhysicalEventKind::DesignSlotDeclared,
            "slot-1",
            10,
            &["evidence-1"],
        )
        .with_predecessors(vec!["missing-event".to_owned()]);
        let result = std::panic::catch_unwind(|| ledger(vec![event]));
        assert!(result.is_err());
    }

    #[test]
    fn relocation_invalidates_reconciliation_until_reinspected() {
        let mut events = declared_and_delivered();
        events.push(
            event("install", PhysicalEventKind::UnitInstalled, "slot-1", 30, &["evidence-1"])
                .with_instance("instance-1")
                .with_location("cabinet-east")
                .with_predecessors(vec!["deliver".to_owned()]),
        );
        events.push(
            event("inspect", PhysicalEventKind::InspectionRecorded, "slot-1", 40, &["evidence-1"])
                .with_instance("instance-1")
                .with_location("cabinet-east")
                .with_predecessors(vec!["install".to_owned()]),
        );
        events.push(
            event("review", PhysicalEventKind::IndependentReconciliationRecorded, "slot-1", 50, &["evidence-1"])
                .with_instance("instance-1")
                .with_location("cabinet-east")
                .with_review_attestation("attestation-1")
                .with_predecessors(vec!["inspect".to_owned()]),
        );
        events.push(
            event("relocate", PhysicalEventKind::UnitRelocated, "slot-1", 60, &["evidence-1"])
                .with_instance("instance-1")
                .with_relocation("cabinet-east", "cabinet-west")
                .with_predecessors(vec!["review".to_owned()]),
        );
        let projection = ledger(events).project(100, &TestVerifier(ReviewVerification::Valid));
        assert_eq!(
            projection.slots["slot-1"].status,
            SlotStatus::InstalledUnreconciled
        );
        assert_eq!(
            projection.slots["slot-1"].active_assignments[0].location_id,
            "cabinet-west"
        );
    }
}
