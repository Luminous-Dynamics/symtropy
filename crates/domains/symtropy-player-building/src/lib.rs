// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Authority-safe player-building proposal contracts for Symtropy.
//!
//! This crate owns authored construction intent, deterministic proposal-plan
//! identity, non-authoritative constraint reports, and read-only build
//! projections. It does **not** own conserved matter, physical execution,
//! structural truth, fabrication truth, commissioning, ownership, civic
//! permission, place identity, or home association.

use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};
use symtropy_game_state::StableId;

/// Wire/semantic schema for the first player-building proposal IR.
pub const PLAYER_BUILDING_PROPOSAL_SCHEMA_VERSION: u32 = 1;

/// Maximum edge/length admitted by one built-in primitive: 1,000 km.
/// Larger objects remain possible through hierarchy/composition or exact
/// external geometry.
pub const MAX_PRIMITIVE_EXTENT_UM: u64 = 1_000_000_000_000;

/// Bounded proposal sizes. Large settlements are expected to compose many
/// bounded intents rather than becoming one giant atomic proposal.
pub const MAX_INTENT_ELEMENTS: usize = 65_536;
pub const MAX_EXACT_REFS: usize = 4_096;
pub const MAX_PLAN_OPERATIONS: usize = 262_144;
pub const MAX_DEPENDENCIES_PER_OPERATION: usize = 256;
pub const MAX_CONSTRAINT_FINDINGS: usize = 65_536;
pub const MAX_FINDING_SUBJECTS: usize = 1_024;

const INTENT_DIGEST_DOMAIN: &[u8] = b"symtropy.player-building.intent.v1\0";
const PLAN_DIGEST_DOMAIN: &[u8] = b"symtropy.player-building.plan.v1\0";
const SHA256_ALGORITHM_ID: &str = "sha256";

/// Portable digest used by exact proposal references.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ProposalDigest {
    pub algorithm: StableId,
    pub value: String,
}

impl ProposalDigest {
    pub fn new(algorithm: StableId, value: impl Into<String>) -> Result<Self, ProposalError> {
        let digest = Self {
            algorithm,
            value: value.into(),
        };
        digest.validate()?;
        Ok(digest)
    }

    pub fn validate(&self) -> Result<(), ProposalError> {
        validate_stable_id(&self.algorithm)?;
        let valid = !self.value.is_empty()
            && self.value.len() <= 256
            && self.value.bytes().all(|byte| byte.is_ascii_graphic());
        if valid {
            Ok(())
        } else {
            Err(ProposalError::InvalidDigestValue(self.value.clone()))
        }
    }

    fn sha256(bytes: &[u8]) -> Self {
        Self {
            algorithm: StableId::parse(SHA256_ALGORITHM_ID)
                .expect("sha256 is a valid stable identifier literal"),
            value: hex(&Sha256::digest(bytes)),
        }
    }
}

/// Exact content-bound reference to authority owned by another subsystem.
///
/// PB-01 can bind planning to external state without claiming to understand or
/// own that authority. The tuple `(authority_id, subject_id, revision)` is one
/// authority identity; two different digests for that same tuple are treated
/// as conflicting input rather than two independent facts.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ExactAuthorityRef {
    pub authority_id: StableId,
    pub subject_id: StableId,
    pub revision: u64,
    pub content_digest: ProposalDigest,
}

impl ExactAuthorityRef {
    pub fn new(
        authority_id: StableId,
        subject_id: StableId,
        revision: u64,
        content_digest: ProposalDigest,
    ) -> Result<Self, ProposalError> {
        let reference = Self {
            authority_id,
            subject_id,
            revision,
            content_digest,
        };
        reference.validate()?;
        Ok(reference)
    }

    pub fn validate(&self) -> Result<(), ProposalError> {
        validate_stable_id(&self.authority_id)?;
        validate_stable_id(&self.subject_id)?;
        self.content_digest.validate()
    }
}

/// Deterministic authoring pose relative to the intent's exact authoring frame.
///
/// Translation is integer micrometres. Rotation is three intrinsic X -> Y -> Z
/// phase coordinates; the `u32` domain divides one turn into 2^32 discrete
/// phases and wraps modulo one turn. This is an authoring/replay representation,
/// not structural-physics truth and not a replacement for a physics engine's
/// internal transform type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AuthoringPose {
    pub translation_um: [i64; 3],
    pub rotation_turn32: [u32; 3],
}

impl AuthoringPose {
    pub const IDENTITY: Self = Self {
        translation_um: [0, 0, 0],
        rotation_turn32: [0, 0, 0],
    };
}

/// Geometry authored directly in the proposal layer.
///
/// Exact external geometry keeps PB-01 useful for CAD/procedural inputs without
/// turning this crate into a geometry kernel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GeometryIntent {
    Cuboid { size_um: [u64; 3] },
    Cylinder { radius_um: u64, length_um: u64 },
    Panel {
        width_um: u64,
        height_um: u64,
        thickness_um: u64,
    },
    Beam {
        length_um: u64,
        cross_section_um: [u64; 2],
    },
    ExactExternal { geometry_ref: ExactAuthorityRef },
}

impl GeometryIntent {
    pub fn validate(&self) -> Result<(), ProposalError> {
        match self {
            Self::Cuboid { size_um } => {
                validate_extent("cuboid.x", size_um[0])?;
                validate_extent("cuboid.y", size_um[1])?;
                validate_extent("cuboid.z", size_um[2])
            }
            Self::Cylinder {
                radius_um,
                length_um,
            } => {
                validate_extent("cylinder.radius", *radius_um)?;
                validate_extent("cylinder.length", *length_um)
            }
            Self::Panel {
                width_um,
                height_um,
                thickness_um,
            } => {
                validate_extent("panel.width", *width_um)?;
                validate_extent("panel.height", *height_um)?;
                validate_extent("panel.thickness", *thickness_um)
            }
            Self::Beam {
                length_um,
                cross_section_um,
            } => {
                validate_extent("beam.length", *length_um)?;
                validate_extent("beam.cross_section.x", cross_section_um[0])?;
                validate_extent("beam.cross_section.y", cross_section_um[1])
            }
            Self::ExactExternal { geometry_ref } => geometry_ref.validate(),
        }
    }
}

/// Non-authoritative requested material semantics for one authored element.
/// A material class is a selector/requirement, never proof of inventory or a
/// reservation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterialIntent {
    pub material_class: StableId,
    specification_refs: Vec<ExactAuthorityRef>,
}

impl MaterialIntent {
    pub fn new(
        material_class: StableId,
        mut specification_refs: Vec<ExactAuthorityRef>,
    ) -> Result<Self, ProposalError> {
        validate_bounded_len(
            "material.specification_refs",
            specification_refs.len(),
            MAX_EXACT_REFS,
        )?;
        specification_refs.sort_by(compare_exact_ref_identity);
        let material = Self {
            material_class,
            specification_refs,
        };
        material.validate_canonical()?;
        Ok(material)
    }

    pub fn specification_refs(&self) -> &[ExactAuthorityRef] {
        &self.specification_refs
    }

    fn validate_canonical(&self) -> Result<(), ProposalError> {
        validate_stable_id(&self.material_class)?;
        validate_bounded_len(
            "material.specification_refs",
            self.specification_refs.len(),
            MAX_EXACT_REFS,
        )?;
        validate_exact_ref_slice("material.specification_refs", &self.specification_refs)
    }
}

/// One player-authored proposed element.
/// `role_id` is descriptive authoring semantics only; it is not proof of
/// physical existence, room semantics, commissioning, ownership, or use.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntentElement {
    pub element_id: StableId,
    pub role_id: StableId,
    pub pose: AuthoringPose,
    pub geometry: GeometryIntent,
    pub material: MaterialIntent,
}

impl IntentElement {
    pub fn new(
        element_id: StableId,
        role_id: StableId,
        pose: AuthoringPose,
        geometry: GeometryIntent,
        material: MaterialIntent,
    ) -> Result<Self, ProposalError> {
        let element = Self {
            element_id,
            role_id,
            pose,
            geometry,
            material,
        };
        element.validate()?;
        Ok(element)
    }

    pub fn validate(&self) -> Result<(), ProposalError> {
        validate_stable_id(&self.element_id)?;
        validate_stable_id(&self.role_id)?;
        self.geometry.validate()?;
        self.material.validate_canonical()
    }
}

/// Exact reference to one immutable authored-intent revision.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ConstructionIntentRef {
    pub intent_id: StableId,
    pub revision: u64,
    pub content_digest: ProposalDigest,
}

impl ConstructionIntentRef {
    pub fn validate(&self) -> Result<(), ProposalError> {
        validate_stable_id(&self.intent_id)?;
        self.content_digest.validate()
    }
}

/// Canonical semantic snapshot of one authored construction intent revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConstructionIntentManifest {
    pub schema_version: u32,
    pub intent_id: StableId,
    pub revision: u64,
    pub proposer_id: StableId,
    /// Optional site descriptor. This is not title, permission, or occupancy.
    pub site_id: Option<StableId>,
    /// Exact coordinate-frame identity for every `AuthoringPose` in this intent.
    pub authoring_frame_ref: ExactAuthorityRef,
    parent_refs: Vec<ConstructionIntentRef>,
    elements: Vec<IntentElement>,
    source_refs: Vec<ExactAuthorityRef>,
}

impl ConstructionIntentManifest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        intent_id: StableId,
        revision: u64,
        proposer_id: StableId,
        site_id: Option<StableId>,
        authoring_frame_ref: ExactAuthorityRef,
        mut parent_refs: Vec<ConstructionIntentRef>,
        mut elements: Vec<IntentElement>,
        mut source_refs: Vec<ExactAuthorityRef>,
    ) -> Result<Self, ProposalError> {
        validate_bounded_len("intent.parents", parent_refs.len(), MAX_EXACT_REFS)?;
        validate_bounded_len("intent.elements", elements.len(), MAX_INTENT_ELEMENTS)?;
        validate_bounded_len("intent.source_refs", source_refs.len(), MAX_EXACT_REFS)?;
        parent_refs.sort_by(compare_intent_ref_identity);
        elements.sort_by(|left, right| left.element_id.cmp(&right.element_id));
        source_refs.sort_by(compare_exact_ref_identity);
        let manifest = Self {
            schema_version: PLAYER_BUILDING_PROPOSAL_SCHEMA_VERSION,
            intent_id,
            revision,
            proposer_id,
            site_id,
            authoring_frame_ref,
            parent_refs,
            elements,
            source_refs,
        };
        manifest.validate_canonical()?;
        Ok(manifest)
    }

    pub fn parent_refs(&self) -> &[ConstructionIntentRef] {
        &self.parent_refs
    }

    pub fn elements(&self) -> &[IntentElement] {
        &self.elements
    }

    pub fn source_refs(&self) -> &[ExactAuthorityRef] {
        &self.source_refs
    }

    fn validate_canonical(&self) -> Result<(), ProposalError> {
        if self.schema_version != PLAYER_BUILDING_PROPOSAL_SCHEMA_VERSION {
            return Err(ProposalError::UnsupportedSchema(self.schema_version));
        }
        validate_stable_id(&self.intent_id)?;
        validate_stable_id(&self.proposer_id)?;
        if let Some(site_id) = &self.site_id {
            validate_stable_id(site_id)?;
        }
        self.authoring_frame_ref.validate()?;

        validate_bounded_len("intent.parents", self.parent_refs.len(), MAX_EXACT_REFS)?;
        for parent in &self.parent_refs {
            parent.validate()?;
            if parent.intent_id != self.intent_id {
                return Err(ProposalError::ForeignIntentParent {
                    child_intent_id: self.intent_id.clone(),
                    parent_intent_id: parent.intent_id.clone(),
                });
            }
            if parent.revision >= self.revision {
                return Err(ProposalError::NonPriorIntentParent {
                    intent_id: self.intent_id.clone(),
                    parent_revision: parent.revision,
                    child_revision: self.revision,
                });
            }
        }
        for pair in self.parent_refs.windows(2) {
            match compare_intent_ref_identity(&pair[0], &pair[1]) {
                Ordering::Greater => {
                    return Err(ProposalError::NonCanonicalOrder("intent.parents"));
                }
                Ordering::Equal => {
                    return Err(ProposalError::DuplicateIntentParent {
                        intent_id: pair[0].intent_id.clone(),
                        revision: pair[0].revision,
                    });
                }
                Ordering::Less => {}
            }
        }

        validate_bounded_len("intent.elements", self.elements.len(), MAX_INTENT_ELEMENTS)?;
        if self.elements.is_empty() {
            return Err(ProposalError::IntentElementsRequired);
        }
        for element in &self.elements {
            element.validate()?;
        }
        for pair in self.elements.windows(2) {
            match pair[0].element_id.cmp(&pair[1].element_id) {
                Ordering::Greater => {
                    return Err(ProposalError::NonCanonicalOrder("intent.elements"));
                }
                Ordering::Equal => {
                    return Err(ProposalError::DuplicateElement(pair[0].element_id.clone()));
                }
                Ordering::Less => {}
            }
        }

        validate_bounded_len("intent.source_refs", self.source_refs.len(), MAX_EXACT_REFS)?;
        validate_exact_ref_slice("intent.source_refs", &self.source_refs)
    }

    fn content_digest(&self) -> Result<ProposalDigest, ProposalError> {
        self.validate_canonical()?;
        let canonical = serde_json::to_vec(self).map_err(ProposalError::Serialization)?;
        let mut bytes = Vec::with_capacity(INTENT_DIGEST_DOMAIN.len() + canonical.len());
        bytes.extend_from_slice(INTENT_DIGEST_DOMAIN);
        bytes.extend_from_slice(&canonical);
        Ok(ProposalDigest::sha256(&bytes))
    }
}

/// Sealed authored construction intent.
/// Deserialization revalidates canonical form and recomputes its digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConstructionIntent {
    manifest: ConstructionIntentManifest,
    content_digest: ProposalDigest,
}

impl ConstructionIntent {
    pub fn seal(manifest: ConstructionIntentManifest) -> Result<Self, ProposalError> {
        let content_digest = manifest.content_digest()?;
        Ok(Self {
            manifest,
            content_digest,
        })
    }

    pub fn manifest(&self) -> &ConstructionIntentManifest {
        &self.manifest
    }

    pub fn content_digest(&self) -> &ProposalDigest {
        &self.content_digest
    }

    pub fn exact_ref(&self) -> ConstructionIntentRef {
        ConstructionIntentRef {
            intent_id: self.manifest.intent_id.clone(),
            revision: self.manifest.revision,
            content_digest: self.content_digest.clone(),
        }
    }

    pub fn validate(&self) -> Result<(), ProposalError> {
        let actual = self.manifest.content_digest()?;
        if actual == self.content_digest {
            Ok(())
        } else {
            Err(ProposalError::DigestMismatch {
                subject: "construction intent",
                expected: self.content_digest.clone(),
                actual,
            })
        }
    }
}

#[derive(Deserialize)]
struct ConstructionIntentWire {
    manifest: ConstructionIntentManifest,
    content_digest: ProposalDigest,
}

impl<'de> Deserialize<'de> for ConstructionIntent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ConstructionIntentWire::deserialize(deserializer)?;
        let intent = Self {
            manifest: wire.manifest,
            content_digest: wire.content_digest,
        };
        intent.validate().map_err(serde::de::Error::custom)?;
        Ok(intent)
    }
}

/// Exact set of state identities used by one deterministic planning pass.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanningContext {
    pub context_id: StableId,
    pub revision: u64,
    exact_inputs: Vec<ExactAuthorityRef>,
}

impl PlanningContext {
    pub fn new(
        context_id: StableId,
        revision: u64,
        mut exact_inputs: Vec<ExactAuthorityRef>,
    ) -> Result<Self, ProposalError> {
        validate_bounded_len("planning_context.exact_inputs", exact_inputs.len(), MAX_EXACT_REFS)?;
        exact_inputs.sort_by(compare_exact_ref_identity);
        let context = Self {
            context_id,
            revision,
            exact_inputs,
        };
        context.validate_canonical()?;
        Ok(context)
    }

    pub fn exact_inputs(&self) -> &[ExactAuthorityRef] {
        &self.exact_inputs
    }

    fn validate_canonical(&self) -> Result<(), ProposalError> {
        validate_stable_id(&self.context_id)?;
        validate_bounded_len("planning_context.exact_inputs", self.exact_inputs.len(), MAX_EXACT_REFS)?;
        validate_exact_ref_slice("planning_context.exact_inputs", &self.exact_inputs)
    }
}

/// Proposal-only operation. None of these variants execute physical work.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConstructionAction {
    RealizeElement { element_id: StableId },
    JoinElements {
        first_element_id: StableId,
        second_element_id: StableId,
        connection_kind: StableId,
    },
    ModifyExactSubject {
        target: ExactAuthorityRef,
        modification_kind: StableId,
        payload_ref: Option<ExactAuthorityRef>,
    },
    RemoveExactSubject {
        target: ExactAuthorityRef,
        removal_kind: StableId,
    },
    /// Unknown profiles remain inert proposals. There is no generic execute API.
    AdapterProposal {
        profile_id: StableId,
        payload_ref: ExactAuthorityRef,
    },
}

impl ConstructionAction {
    fn validate(&self, intent_element_ids: &BTreeSet<StableId>) -> Result<(), ProposalError> {
        match self {
            Self::RealizeElement { element_id } => {
                validate_stable_id(element_id)?;
                ensure_intent_element(intent_element_ids, element_id)
            }
            Self::JoinElements {
                first_element_id,
                second_element_id,
                connection_kind,
            } => {
                validate_stable_id(first_element_id)?;
                validate_stable_id(second_element_id)?;
                validate_stable_id(connection_kind)?;
                if first_element_id == second_element_id {
                    return Err(ProposalError::SelfJoin(first_element_id.clone()));
                }
                ensure_intent_element(intent_element_ids, first_element_id)?;
                ensure_intent_element(intent_element_ids, second_element_id)
            }
            Self::ModifyExactSubject {
                target,
                modification_kind,
                payload_ref,
            } => {
                target.validate()?;
                validate_stable_id(modification_kind)?;
                if let Some(payload_ref) = payload_ref {
                    payload_ref.validate()?;
                }
                Ok(())
            }
            Self::RemoveExactSubject {
                target,
                removal_kind,
            } => {
                target.validate()?;
                validate_stable_id(removal_kind)
            }
            Self::AdapterProposal {
                profile_id,
                payload_ref,
            } => {
                validate_stable_id(profile_id)?;
                payload_ref.validate()
            }
        }
    }
}

/// One node in the deterministic proposal operation DAG.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedOperation {
    pub operation_id: StableId,
    depends_on: Vec<StableId>,
    pub action: ConstructionAction,
}

impl PlannedOperation {
    pub fn new(
        operation_id: StableId,
        mut depends_on: Vec<StableId>,
        action: ConstructionAction,
    ) -> Result<Self, ProposalError> {
        validate_bounded_len(
            "operation.depends_on",
            depends_on.len(),
            MAX_DEPENDENCIES_PER_OPERATION,
        )?;
        depends_on.sort();
        let operation = Self {
            operation_id,
            depends_on,
            action,
        };
        operation.validate_local()?;
        Ok(operation)
    }

    pub fn depends_on(&self) -> &[StableId] {
        &self.depends_on
    }

    fn validate_local(&self) -> Result<(), ProposalError> {
        validate_stable_id(&self.operation_id)?;
        validate_bounded_len(
            "operation.depends_on",
            self.depends_on.len(),
            MAX_DEPENDENCIES_PER_OPERATION,
        )?;
        for dependency in &self.depends_on {
            validate_stable_id(dependency)?;
            if dependency == &self.operation_id {
                return Err(ProposalError::SelfDependency(self.operation_id.clone()));
            }
        }
        for pair in self.depends_on.windows(2) {
            if pair[0] > pair[1] {
                return Err(ProposalError::NonCanonicalOrder("operation.depends_on"));
            }
            if pair[0] == pair[1] {
                return Err(ProposalError::DuplicateDependency {
                    operation_id: self.operation_id.clone(),
                    dependency_id: pair[0].clone(),
                });
            }
        }
        Ok(())
    }
}

/// Canonical deterministic plan proposal for one exact intent and context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConstructionPlanManifest {
    pub schema_version: u32,
    pub plan_id: StableId,
    pub revision: u64,
    pub intent_ref: ConstructionIntentRef,
    intent_element_ids: Vec<StableId>,
    /// Exact compiler/planner implementation/profile identity.
    pub compiler_ref: ExactAuthorityRef,
    pub planning_context: PlanningContext,
    operations: Vec<PlannedOperation>,
}

impl ConstructionPlanManifest {
    pub fn new(
        plan_id: StableId,
        revision: u64,
        intent: &ConstructionIntent,
        compiler_ref: ExactAuthorityRef,
        planning_context: PlanningContext,
        mut operations: Vec<PlannedOperation>,
    ) -> Result<Self, ProposalError> {
        intent.validate()?;
        validate_bounded_len("plan.operations", operations.len(), MAX_PLAN_OPERATIONS)?;
        operations.sort_by(|left, right| left.operation_id.cmp(&right.operation_id));
        let intent_element_ids = intent
            .manifest()
            .elements()
            .iter()
            .map(|element| element.element_id.clone())
            .collect();
        let manifest = Self {
            schema_version: PLAYER_BUILDING_PROPOSAL_SCHEMA_VERSION,
            plan_id,
            revision,
            intent_ref: intent.exact_ref(),
            intent_element_ids,
            compiler_ref,
            planning_context,
            operations,
        };
        manifest.validate_canonical()?;
        Ok(manifest)
    }

    pub fn intent_element_ids(&self) -> &[StableId] {
        &self.intent_element_ids
    }

    pub fn operations(&self) -> &[PlannedOperation] {
        &self.operations
    }

    /// Deterministic topological order with operation identity as the tie-break.
    pub fn topological_order(&self) -> Result<Vec<StableId>, ProposalError> {
        self.validate_canonical()?;
        topological_order(&self.operations)
    }

    fn validate_canonical(&self) -> Result<(), ProposalError> {
        if self.schema_version != PLAYER_BUILDING_PROPOSAL_SCHEMA_VERSION {
            return Err(ProposalError::UnsupportedSchema(self.schema_version));
        }
        validate_stable_id(&self.plan_id)?;
        self.intent_ref.validate()?;
        self.compiler_ref.validate()?;
        self.planning_context.validate_canonical()?;

        if self.intent_element_ids.is_empty() {
            return Err(ProposalError::IntentElementsRequired);
        }
        validate_bounded_len(
            "plan.intent_element_ids",
            self.intent_element_ids.len(),
            MAX_INTENT_ELEMENTS,
        )?;
        for element_id in &self.intent_element_ids {
            validate_stable_id(element_id)?;
        }
        for pair in self.intent_element_ids.windows(2) {
            if pair[0] > pair[1] {
                return Err(ProposalError::NonCanonicalOrder("plan.intent_element_ids"));
            }
            if pair[0] == pair[1] {
                return Err(ProposalError::DuplicateElement(pair[0].clone()));
            }
        }

        validate_bounded_len("plan.operations", self.operations.len(), MAX_PLAN_OPERATIONS)?;
        for pair in self.operations.windows(2) {
            if pair[0].operation_id > pair[1].operation_id {
                return Err(ProposalError::NonCanonicalOrder("plan.operations"));
            }
            if pair[0].operation_id == pair[1].operation_id {
                return Err(ProposalError::DuplicateOperation(
                    pair[0].operation_id.clone(),
                ));
            }
        }

        let element_ids: BTreeSet<_> = self.intent_element_ids.iter().cloned().collect();
        let operation_ids: BTreeSet<_> = self
            .operations
            .iter()
            .map(|operation| operation.operation_id.clone())
            .collect();
        let mut realization_by_element = BTreeMap::<StableId, StableId>::new();

        for operation in &self.operations {
            operation.validate_local()?;
            operation.action.validate(&element_ids)?;
            for dependency in operation.depends_on() {
                if !operation_ids.contains(dependency) {
                    return Err(ProposalError::UnknownDependency {
                        operation_id: operation.operation_id.clone(),
                        dependency_id: dependency.clone(),
                    });
                }
            }
            if let ConstructionAction::RealizeElement { element_id } = &operation.action {
                if realization_by_element
                    .insert(element_id.clone(), operation.operation_id.clone())
                    .is_some()
                {
                    return Err(ProposalError::DuplicateRealization(element_id.clone()));
                }
            }
        }

        for operation in &self.operations {
            if let ConstructionAction::JoinElements {
                first_element_id,
                second_element_id,
                ..
            } = &operation.action
            {
                let first_realization = realization_by_element
                    .get(first_element_id)
                    .ok_or_else(|| ProposalError::JoinElementNotRealized(first_element_id.clone()))?;
                let second_realization = realization_by_element
                    .get(second_element_id)
                    .ok_or_else(|| ProposalError::JoinElementNotRealized(second_element_id.clone()))?;
                if operation.depends_on().binary_search(first_realization).is_err() {
                    return Err(ProposalError::JoinMissingRealizationDependency {
                        join_operation_id: operation.operation_id.clone(),
                        realization_operation_id: first_realization.clone(),
                    });
                }
                if operation.depends_on().binary_search(second_realization).is_err() {
                    return Err(ProposalError::JoinMissingRealizationDependency {
                        join_operation_id: operation.operation_id.clone(),
                        realization_operation_id: second_realization.clone(),
                    });
                }
            }
        }

        topological_order(&self.operations).map(|_| ())
    }

    fn content_digest(&self) -> Result<ProposalDigest, ProposalError> {
        self.validate_canonical()?;
        let canonical = serde_json::to_vec(self).map_err(ProposalError::Serialization)?;
        let mut bytes = Vec::with_capacity(PLAN_DIGEST_DOMAIN.len() + canonical.len());
        bytes.extend_from_slice(PLAN_DIGEST_DOMAIN);
        bytes.extend_from_slice(&canonical);
        Ok(ProposalDigest::sha256(&bytes))
    }
}

/// Exact reference to one deterministic plan proposal.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ConstructionPlanRef {
    pub plan_id: StableId,
    pub revision: u64,
    pub content_digest: ProposalDigest,
}

impl ConstructionPlanRef {
    pub fn validate(&self) -> Result<(), ProposalError> {
        validate_stable_id(&self.plan_id)?;
        self.content_digest.validate()
    }
}

/// Sealed proposal plan. Still not reservation or execution authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConstructionPlan {
    manifest: ConstructionPlanManifest,
    content_digest: ProposalDigest,
}

impl ConstructionPlan {
    pub fn seal(manifest: ConstructionPlanManifest) -> Result<Self, ProposalError> {
        let content_digest = manifest.content_digest()?;
        Ok(Self {
            manifest,
            content_digest,
        })
    }

    pub fn manifest(&self) -> &ConstructionPlanManifest {
        &self.manifest
    }

    pub fn content_digest(&self) -> &ProposalDigest {
        &self.content_digest
    }

    pub fn exact_ref(&self) -> ConstructionPlanRef {
        ConstructionPlanRef {
            plan_id: self.manifest.plan_id.clone(),
            revision: self.manifest.revision,
            content_digest: self.content_digest.clone(),
        }
    }

    pub fn validate(&self) -> Result<(), ProposalError> {
        let actual = self.manifest.content_digest()?;
        if actual == self.content_digest {
            Ok(())
        } else {
            Err(ProposalError::DigestMismatch {
                subject: "construction plan",
                expected: self.content_digest.clone(),
                actual,
            })
        }
    }

    /// Re-resolves the stored plan against the exact authored intent it names.
    /// Future authority-crossing adapters must repeat this check.
    pub fn validate_against_intent(
        &self,
        intent: &ConstructionIntent,
    ) -> Result<(), ProposalError> {
        self.validate()?;
        intent.validate()?;
        if self.manifest.intent_ref != intent.exact_ref() {
            return Err(ProposalError::IntentReferenceMismatch);
        }
        let expected_ids: Vec<_> = intent
            .manifest()
            .elements()
            .iter()
            .map(|element| element.element_id.clone())
            .collect();
        if self.manifest.intent_element_ids != expected_ids {
            return Err(ProposalError::IntentElementSetMismatch);
        }
        Ok(())
    }
}

#[derive(Deserialize)]
struct ConstructionPlanWire {
    manifest: ConstructionPlanManifest,
    content_digest: ProposalDigest,
}

impl<'de> Deserialize<'de> for ConstructionPlan {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ConstructionPlanWire::deserialize(deserializer)?;
        let plan = Self {
            manifest: wire.manifest,
            content_digest: wire.content_digest,
        };
        plan.validate().map_err(serde::de::Error::custom)?;
        Ok(plan)
    }
}

/// Constraint severity without a scalar quality/confidence score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ConstraintSeverity {
    HardFailure,
    Advisory,
}

/// One machine-readable constraint result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConstraintFinding {
    pub finding_id: StableId,
    pub rule_id: StableId,
    pub severity: ConstraintSeverity,
    subjects: Vec<StableId>,
    pub message_key: StableId,
    evidence_refs: Vec<ExactAuthorityRef>,
}

impl ConstraintFinding {
    pub fn new(
        finding_id: StableId,
        rule_id: StableId,
        severity: ConstraintSeverity,
        mut subjects: Vec<StableId>,
        message_key: StableId,
        mut evidence_refs: Vec<ExactAuthorityRef>,
    ) -> Result<Self, ProposalError> {
        validate_bounded_len("constraint.subjects", subjects.len(), MAX_FINDING_SUBJECTS)?;
        validate_bounded_len("constraint.evidence_refs", evidence_refs.len(), MAX_EXACT_REFS)?;
        subjects.sort();
        subjects.dedup();
        evidence_refs.sort_by(compare_exact_ref_identity);
        let finding = Self {
            finding_id,
            rule_id,
            severity,
            subjects,
            message_key,
            evidence_refs,
        };
        finding.validate_canonical()?;
        Ok(finding)
    }

    pub fn subjects(&self) -> &[StableId] {
        &self.subjects
    }

    pub fn evidence_refs(&self) -> &[ExactAuthorityRef] {
        &self.evidence_refs
    }

    fn validate_canonical(&self) -> Result<(), ProposalError> {
        validate_stable_id(&self.finding_id)?;
        validate_stable_id(&self.rule_id)?;
        validate_stable_id(&self.message_key)?;
        validate_bounded_len("constraint.subjects", self.subjects.len(), MAX_FINDING_SUBJECTS)?;
        for subject in &self.subjects {
            validate_stable_id(subject)?;
        }
        for pair in self.subjects.windows(2) {
            if pair[0] >= pair[1] {
                return Err(ProposalError::NonCanonicalOrder("constraint.subjects"));
            }
        }
        validate_bounded_len("constraint.evidence_refs", self.evidence_refs.len(), MAX_EXACT_REFS)?;
        validate_exact_ref_slice("constraint.evidence_refs", &self.evidence_refs)
    }
}

/// Non-authoritative constraint report bound to one exact plan proposal.
/// A future execution adapter must independently decide which evaluator/rule
/// authorities it accepts; `HardFailure` is not self-authenticating authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConstraintReport {
    pub plan_ref: ConstructionPlanRef,
    pub report_profile_id: StableId,
    pub evaluator_ref: Option<ExactAuthorityRef>,
    findings: Vec<ConstraintFinding>,
}

impl ConstraintReport {
    pub fn new(
        plan_ref: ConstructionPlanRef,
        report_profile_id: StableId,
        evaluator_ref: Option<ExactAuthorityRef>,
        mut findings: Vec<ConstraintFinding>,
    ) -> Result<Self, ProposalError> {
        validate_bounded_len("constraint.findings", findings.len(), MAX_CONSTRAINT_FINDINGS)?;
        findings.sort_by(|left, right| left.finding_id.cmp(&right.finding_id));
        let report = Self {
            plan_ref,
            report_profile_id,
            evaluator_ref,
            findings,
        };
        report.validate_canonical()?;
        Ok(report)
    }

    pub fn findings(&self) -> &[ConstraintFinding] {
        &self.findings
    }

    /// This reports only report content; it is not an execution/safety predicate.
    pub fn has_hard_failures(&self) -> bool {
        self.findings
            .iter()
            .any(|finding| finding.severity == ConstraintSeverity::HardFailure)
    }

    fn validate_canonical(&self) -> Result<(), ProposalError> {
        self.plan_ref.validate()?;
        validate_stable_id(&self.report_profile_id)?;
        if let Some(evaluator_ref) = &self.evaluator_ref {
            evaluator_ref.validate()?;
        }
        validate_bounded_len("constraint.findings", self.findings.len(), MAX_CONSTRAINT_FINDINGS)?;
        for finding in &self.findings {
            finding.validate_canonical()?;
        }
        for pair in self.findings.windows(2) {
            if pair[0].finding_id > pair[1].finding_id {
                return Err(ProposalError::NonCanonicalOrder("constraint.findings"));
            }
            if pair[0].finding_id == pair[1].finding_id {
                return Err(ProposalError::DuplicateFinding(
                    pair[0].finding_id.clone(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Deserialize)]
struct ConstraintReportWire {
    plan_ref: ConstructionPlanRef,
    report_profile_id: StableId,
    evaluator_ref: Option<ExactAuthorityRef>,
    findings: Vec<ConstraintFinding>,
}

impl<'de> Deserialize<'de> for ConstraintReport {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ConstraintReportWire::deserialize(deserializer)?;
        let report = Self {
            plan_ref: wire.plan_ref,
            report_profile_id: wire.report_profile_id,
            evaluator_ref: wire.evaluator_ref,
            findings: wire.findings,
        };
        report
            .validate_canonical()
            .map_err(serde::de::Error::custom)?;
        Ok(report)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectionDisposition {
    Planned,
    Unscheduled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectedElement {
    pub element_id: StableId,
    pub role_id: StableId,
    pub pose: AuthoringPose,
    pub geometry: GeometryIntent,
    pub disposition: ProjectionDisposition,
}

/// Read-only visualization/interaction projection.
///
/// It is intentionally Serialize-only: persisted/networked projection bytes do
/// not re-enter as canonical state. Consumers regenerate it from exact intent +
/// plan instead.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BuildProjection {
    pub intent_ref: ConstructionIntentRef,
    pub plan_ref: ConstructionPlanRef,
    elements: Vec<ProjectedElement>,
}

impl BuildProjection {
    pub fn from_intent_and_plan(
        intent: &ConstructionIntent,
        plan: &ConstructionPlan,
    ) -> Result<Self, ProposalError> {
        plan.validate_against_intent(intent)?;
        let realized: BTreeSet<_> = plan
            .manifest()
            .operations()
            .iter()
            .filter_map(|operation| match &operation.action {
                ConstructionAction::RealizeElement { element_id } => Some(element_id.clone()),
                _ => None,
            })
            .collect();
        let elements = intent
            .manifest()
            .elements()
            .iter()
            .map(|element| ProjectedElement {
                element_id: element.element_id.clone(),
                role_id: element.role_id.clone(),
                pose: element.pose,
                geometry: element.geometry.clone(),
                disposition: if realized.contains(&element.element_id) {
                    ProjectionDisposition::Planned
                } else {
                    ProjectionDisposition::Unscheduled
                },
            })
            .collect();
        Ok(Self {
            intent_ref: intent.exact_ref(),
            plan_ref: plan.exact_ref(),
            elements,
        })
    }

    pub fn elements(&self) -> &[ProjectedElement] {
        &self.elements
    }
}

fn ensure_intent_element(
    intent_element_ids: &BTreeSet<StableId>,
    element_id: &StableId,
) -> Result<(), ProposalError> {
    if intent_element_ids.contains(element_id) {
        Ok(())
    } else {
        Err(ProposalError::UnknownIntentElement(element_id.clone()))
    }
}

fn topological_order(operations: &[PlannedOperation]) -> Result<Vec<StableId>, ProposalError> {
    let mut indegree = BTreeMap::<StableId, usize>::new();
    let mut dependents = BTreeMap::<StableId, Vec<StableId>>::new();
    for operation in operations {
        indegree.insert(operation.operation_id.clone(), operation.depends_on().len());
        dependents.entry(operation.operation_id.clone()).or_default();
    }
    for operation in operations {
        for dependency in operation.depends_on() {
            dependents
                .entry(dependency.clone())
                .or_default()
                .push(operation.operation_id.clone());
        }
    }
    for children in dependents.values_mut() {
        children.sort();
    }

    let mut ready: BTreeSet<_> = indegree
        .iter()
        .filter_map(|(operation_id, degree)| {
            if *degree == 0 {
                Some(operation_id.clone())
            } else {
                None
            }
        })
        .collect();
    let mut ordered = Vec::with_capacity(operations.len());

    while let Some(operation_id) = ready.pop_first() {
        ordered.push(operation_id.clone());
        if let Some(children) = dependents.get(&operation_id) {
            for child in children {
                let degree = indegree
                    .get_mut(child)
                    .expect("validated operation graph contains every child");
                *degree -= 1;
                if *degree == 0 {
                    ready.insert(child.clone());
                }
            }
        }
    }

    if ordered.len() == operations.len() {
        Ok(ordered)
    } else {
        Err(ProposalError::DependencyCycle)
    }
}

fn validate_extent(field: &'static str, value: u64) -> Result<(), ProposalError> {
    if value == 0 || value > MAX_PRIMITIVE_EXTENT_UM {
        Err(ProposalError::InvalidPrimitiveExtent { field, value })
    } else {
        Ok(())
    }
}

fn validate_bounded_len(
    field: &'static str,
    actual: usize,
    max: usize,
) -> Result<(), ProposalError> {
    if actual <= max {
        Ok(())
    } else {
        Err(ProposalError::TooManyItems { field, max, actual })
    }
}

fn validate_exact_ref_slice(
    field: &'static str,
    refs: &[ExactAuthorityRef],
) -> Result<(), ProposalError> {
    for reference in refs {
        reference.validate()?;
    }
    for pair in refs.windows(2) {
        match compare_exact_ref_identity(&pair[0], &pair[1]) {
            Ordering::Greater => return Err(ProposalError::NonCanonicalOrder(field)),
            Ordering::Equal => {
                return Err(ProposalError::DuplicateExactAuthorityIdentity {
                    field,
                    authority_id: pair[0].authority_id.clone(),
                    subject_id: pair[0].subject_id.clone(),
                    revision: pair[0].revision,
                });
            }
            Ordering::Less => {}
        }
    }
    Ok(())
}

fn compare_exact_ref_identity(left: &ExactAuthorityRef, right: &ExactAuthorityRef) -> Ordering {
    (&left.authority_id, &left.subject_id, left.revision).cmp(&(
        &right.authority_id,
        &right.subject_id,
        right.revision,
    ))
}

fn compare_intent_ref_identity(
    left: &ConstructionIntentRef,
    right: &ConstructionIntentRef,
) -> Ordering {
    (&left.intent_id, left.revision).cmp(&(&right.intent_id, right.revision))
}

fn validate_stable_id(id: &StableId) -> Result<(), ProposalError> {
    StableId::parse(id.as_str())
        .map(|_| ())
        .map_err(|_| ProposalError::InvalidStableId(id.as_str().to_string()))
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

#[derive(Debug)]
pub enum ProposalError {
    UnsupportedSchema(u32),
    InvalidStableId(String),
    InvalidDigestValue(String),
    InvalidPrimitiveExtent {
        field: &'static str,
        value: u64,
    },
    TooManyItems {
        field: &'static str,
        max: usize,
        actual: usize,
    },
    IntentElementsRequired,
    DuplicateElement(StableId),
    DuplicateIntentParent {
        intent_id: StableId,
        revision: u64,
    },
    ForeignIntentParent {
        child_intent_id: StableId,
        parent_intent_id: StableId,
    },
    NonPriorIntentParent {
        intent_id: StableId,
        parent_revision: u64,
        child_revision: u64,
    },
    DuplicateExactAuthorityIdentity {
        field: &'static str,
        authority_id: StableId,
        subject_id: StableId,
        revision: u64,
    },
    NonCanonicalOrder(&'static str),
    DuplicateOperation(StableId),
    SelfDependency(StableId),
    DuplicateDependency {
        operation_id: StableId,
        dependency_id: StableId,
    },
    UnknownDependency {
        operation_id: StableId,
        dependency_id: StableId,
    },
    DependencyCycle,
    UnknownIntentElement(StableId),
    DuplicateRealization(StableId),
    SelfJoin(StableId),
    JoinElementNotRealized(StableId),
    JoinMissingRealizationDependency {
        join_operation_id: StableId,
        realization_operation_id: StableId,
    },
    IntentReferenceMismatch,
    IntentElementSetMismatch,
    DuplicateFinding(StableId),
    Serialization(serde_json::Error),
    DigestMismatch {
        subject: &'static str,
        expected: ProposalDigest,
        actual: ProposalDigest,
    },
}

impl fmt::Display for ProposalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedSchema(version) => {
                write!(formatter, "unsupported player-building proposal schema {version}")
            }
            Self::InvalidStableId(id) => write!(formatter, "invalid stable identifier: {id}"),
            Self::InvalidDigestValue(value) => write!(
                formatter,
                "proposal digest value must contain 1..=256 printable non-whitespace ASCII bytes, got {} bytes",
                value.len()
            ),
            Self::InvalidPrimitiveExtent { field, value } => write!(
                formatter,
                "invalid primitive extent {field}={value} um; expected 1..={MAX_PRIMITIVE_EXTENT_UM}"
            ),
            Self::TooManyItems { field, max, actual } => {
                write!(formatter, "{field} contains {actual} items; maximum is {max}")
            }
            Self::IntentElementsRequired => {
                write!(formatter, "construction intent requires at least one authored element")
            }
            Self::DuplicateElement(id) => write!(formatter, "duplicate intent element {id}"),
            Self::DuplicateIntentParent { intent_id, revision } => write!(
                formatter,
                "duplicate construction-intent parent {intent_id} revision {revision}"
            ),
            Self::ForeignIntentParent {
                child_intent_id,
                parent_intent_id,
            } => write!(
                formatter,
                "construction intent {child_intent_id} cannot use foreign intent {parent_intent_id} as revision ancestry"
            ),
            Self::NonPriorIntentParent {
                intent_id,
                parent_revision,
                child_revision,
            } => write!(
                formatter,
                "construction intent {intent_id} parent revision {parent_revision} is not prior to child revision {child_revision}"
            ),
            Self::DuplicateExactAuthorityIdentity {
                field,
                authority_id,
                subject_id,
                revision,
            } => write!(
                formatter,
                "duplicate/conflicting exact authority identity in {field}: {authority_id}/{subject_id}@{revision}"
            ),
            Self::NonCanonicalOrder(field) => write!(formatter, "{field} is not canonically ordered"),
            Self::DuplicateOperation(id) => write!(formatter, "duplicate planned operation {id}"),
            Self::SelfDependency(id) => write!(formatter, "operation {id} depends on itself"),
            Self::DuplicateDependency {
                operation_id,
                dependency_id,
            } => write!(
                formatter,
                "operation {operation_id} repeats dependency {dependency_id}"
            ),
            Self::UnknownDependency {
                operation_id,
                dependency_id,
            } => write!(
                formatter,
                "operation {operation_id} depends on unknown operation {dependency_id}"
            ),
            Self::DependencyCycle => write!(formatter, "planned operation graph contains a cycle"),
            Self::UnknownIntentElement(id) => {
                write!(formatter, "planned action references unknown intent element {id}")
            }
            Self::DuplicateRealization(id) => {
                write!(formatter, "intent element {id} is realized more than once")
            }
            Self::SelfJoin(id) => write!(formatter, "intent element {id} cannot be joined to itself"),
            Self::JoinElementNotRealized(id) => write!(
                formatter,
                "join references intent element {id} without a realization operation"
            ),
            Self::JoinMissingRealizationDependency {
                join_operation_id,
                realization_operation_id,
            } => write!(
                formatter,
                "join operation {join_operation_id} does not depend on realization {realization_operation_id}"
            ),
            Self::IntentReferenceMismatch => {
                write!(formatter, "plan does not bind the supplied exact construction intent")
            }
            Self::IntentElementSetMismatch => {
                write!(formatter, "plan intent-element snapshot differs from supplied intent")
            }
            Self::DuplicateFinding(id) => write!(formatter, "duplicate constraint finding {id}"),
            Self::Serialization(error) => write!(formatter, "proposal serialization failed: {error}"),
            Self::DigestMismatch {
                subject,
                expected,
                actual,
            } => write!(
                formatter,
                "{subject} digest mismatch: expected {}:{}, actual {}:{}",
                expected.algorithm, expected.value, actual.algorithm, actual.value
            ),
        }
    }
}

impl Error for ProposalError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Serialization(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> StableId {
        StableId::parse(value).unwrap()
    }

    fn digest(value: &str) -> ProposalDigest {
        ProposalDigest::new(id("sha256"), value).unwrap()
    }

    fn exact(authority: &str, subject: &str, revision: u64, value: &str) -> ExactAuthorityRef {
        ExactAuthorityRef::new(id(authority), id(subject), revision, digest(value)).unwrap()
    }

    fn frame() -> ExactAuthorityRef {
        exact("authority:frame", "frame:firstlight-local", 1, "frame-a")
    }

    fn compiler() -> ExactAuthorityRef {
        exact("authority:planner", "planner:pb01.v1", 1, "compiler-a")
    }

    fn material() -> MaterialIntent {
        MaterialIntent::new(id("material:timber.structural"), Vec::new()).unwrap()
    }

    fn element(name: &str, x_um: i64) -> IntentElement {
        IntentElement::new(
            id(name),
            id("role:frame"),
            AuthoringPose {
                translation_um: [x_um, 0, 0],
                rotation_turn32: [0, 0, 0],
            },
            GeometryIntent::Beam {
                length_um: 2_400_000,
                cross_section_um: [100_000, 100_000],
            },
            material(),
        )
        .unwrap()
    }

    fn intent_with(elements: Vec<IntentElement>) -> ConstructionIntent {
        ConstructionIntent::seal(
            ConstructionIntentManifest::new(
                id("build-intent:shelter"),
                1,
                id("actor:player"),
                Some(id("site:firstlight")),
                frame(),
                Vec::new(),
                elements,
                vec![exact("authority:terrain", "cell:42", 7, "terrain-a")],
            )
            .unwrap(),
        )
        .unwrap()
    }

    fn realize(operation: &str, element: &str) -> PlannedOperation {
        PlannedOperation::new(
            id(operation),
            Vec::new(),
            ConstructionAction::RealizeElement {
                element_id: id(element),
            },
        )
        .unwrap()
    }

    fn plan_with(
        intent: &ConstructionIntent,
        context: PlanningContext,
        operations: Vec<PlannedOperation>,
    ) -> ConstructionPlan {
        ConstructionPlan::seal(
            ConstructionPlanManifest::new(
                id("build-plan:shelter"),
                1,
                intent,
                compiler(),
                context,
                operations,
            )
            .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn caller_element_order_does_not_change_exact_intent() {
        let a = element("element:a", 0);
        let b = element("element:b", 2_000_000);
        let left = intent_with(vec![a.clone(), b.clone()]);
        let right = intent_with(vec![b, a]);
        assert_eq!(left, right);
        assert_eq!(left.exact_ref(), right.exact_ref());
    }

    #[test]
    fn free_placement_change_changes_exact_intent_identity() {
        let left = intent_with(vec![element("element:a", 0)]);
        let right = intent_with(vec![element("element:a", 1)]);
        assert_ne!(left.exact_ref(), right.exact_ref());
    }

    #[test]
    fn coordinate_frame_change_changes_exact_intent_identity() {
        let left = intent_with(vec![element("element:a", 0)]);
        let right = ConstructionIntent::seal(
            ConstructionIntentManifest::new(
                id("build-intent:shelter"),
                1,
                id("actor:player"),
                Some(id("site:firstlight")),
                exact("authority:frame", "frame:firstlight-local", 1, "frame-b"),
                Vec::new(),
                vec![element("element:a", 0)],
                vec![exact("authority:terrain", "cell:42", 7, "terrain-a")],
            )
            .unwrap(),
        )
        .unwrap();
        assert_ne!(left.exact_ref(), right.exact_ref());
    }

    #[test]
    fn intent_parent_must_be_same_identity_and_prior_revision() {
        let parent = ConstructionIntentRef {
            intent_id: id("build-intent:shelter"),
            revision: 1,
            content_digest: digest("parent-a"),
        };
        let child = ConstructionIntentManifest::new(
            id("build-intent:shelter"),
            2,
            id("actor:player"),
            Some(id("site:firstlight")),
            frame(),
            vec![parent],
            vec![element("element:a", 0)],
            Vec::new(),
        );
        assert!(child.is_ok());

        let foreign = ConstructionIntentManifest::new(
            id("build-intent:shelter"),
            2,
            id("actor:player"),
            None,
            frame(),
            vec![ConstructionIntentRef {
                intent_id: id("build-intent:other"),
                revision: 1,
                content_digest: digest("parent-b"),
            }],
            vec![element("element:a", 0)],
            Vec::new(),
        );
        assert!(matches!(foreign, Err(ProposalError::ForeignIntentParent { .. })));
    }

    #[test]
    fn conflicting_same_revision_authority_refs_fail_closed() {
        let result = PlanningContext::new(
            id("context:one"),
            1,
            vec![
                exact("authority:terrain", "cell:42", 7, "terrain-a"),
                exact("authority:terrain", "cell:42", 7, "terrain-b"),
            ],
        );
        assert!(matches!(
            result,
            Err(ProposalError::DuplicateExactAuthorityIdentity { .. })
        ));
    }

    #[test]
    fn zero_or_unbounded_primitive_extent_fails_closed() {
        let zero = GeometryIntent::Panel {
            width_um: 0,
            height_um: 2_000_000,
            thickness_um: 20_000,
        };
        assert!(matches!(
            zero.validate(),
            Err(ProposalError::InvalidPrimitiveExtent { .. })
        ));

        let huge = GeometryIntent::Cuboid {
            size_um: [MAX_PRIMITIVE_EXTENT_UM + 1, 1, 1],
        };
        assert!(matches!(
            huge.validate(),
            Err(ProposalError::InvalidPrimitiveExtent { .. })
        ));
    }

    #[test]
    fn plan_operation_order_is_canonical_and_topological() {
        let intent = intent_with(vec![element("element:a", 0), element("element:b", 1)]);
        let op_a = realize("op:a", "element:a");
        let op_b = realize("op:b", "element:b");
        let join = PlannedOperation::new(
            id("op:join"),
            vec![id("op:b"), id("op:a")],
            ConstructionAction::JoinElements {
                first_element_id: id("element:a"),
                second_element_id: id("element:b"),
                connection_kind: id("join:fastener"),
            },
        )
        .unwrap();
        let context = PlanningContext::new(id("context:one"), 1, Vec::new()).unwrap();
        let left = plan_with(
            &intent,
            context.clone(),
            vec![join.clone(), op_b.clone(), op_a.clone()],
        );
        let right = plan_with(&intent, context, vec![op_a, join, op_b]);
        assert_eq!(left.exact_ref(), right.exact_ref());
        assert_eq!(
            left.manifest().topological_order().unwrap(),
            vec![id("op:a"), id("op:b"), id("op:join")]
        );
    }

    #[test]
    fn unknown_dependency_fails_closed() {
        let intent = intent_with(vec![element("element:a", 0)]);
        let operation = PlannedOperation::new(
            id("op:a"),
            vec![id("op:missing")],
            ConstructionAction::RealizeElement {
                element_id: id("element:a"),
            },
        )
        .unwrap();
        let result = ConstructionPlanManifest::new(
            id("build-plan:shelter"),
            1,
            &intent,
            compiler(),
            PlanningContext::new(id("context:one"), 1, Vec::new()).unwrap(),
            vec![operation],
        );
        assert!(matches!(result, Err(ProposalError::UnknownDependency { .. })));
    }

    #[test]
    fn dependency_cycle_fails_closed() {
        let intent = intent_with(vec![element("element:a", 0), element("element:b", 1)]);
        let a = PlannedOperation::new(
            id("op:a"),
            vec![id("op:b")],
            ConstructionAction::RealizeElement {
                element_id: id("element:a"),
            },
        )
        .unwrap();
        let b = PlannedOperation::new(
            id("op:b"),
            vec![id("op:a")],
            ConstructionAction::RealizeElement {
                element_id: id("element:b"),
            },
        )
        .unwrap();
        let result = ConstructionPlanManifest::new(
            id("build-plan:shelter"),
            1,
            &intent,
            compiler(),
            PlanningContext::new(id("context:one"), 1, Vec::new()).unwrap(),
            vec![a, b],
        );
        assert!(matches!(result, Err(ProposalError::DependencyCycle)));
    }

    #[test]
    fn join_requires_realization_dependencies() {
        let intent = intent_with(vec![element("element:a", 0), element("element:b", 1)]);
        let a = realize("op:a", "element:a");
        let b = realize("op:b", "element:b");
        let join = PlannedOperation::new(
            id("op:join"),
            vec![id("op:a")],
            ConstructionAction::JoinElements {
                first_element_id: id("element:a"),
                second_element_id: id("element:b"),
                connection_kind: id("join:fastener"),
            },
        )
        .unwrap();
        let result = ConstructionPlanManifest::new(
            id("build-plan:shelter"),
            1,
            &intent,
            compiler(),
            PlanningContext::new(id("context:one"), 1, Vec::new()).unwrap(),
            vec![a, b, join],
        );
        assert!(matches!(
            result,
            Err(ProposalError::JoinMissingRealizationDependency { .. })
        ));
    }

    #[test]
    fn changed_exact_planning_input_changes_plan_identity() {
        let intent = intent_with(vec![element("element:a", 0)]);
        let operation = realize("op:a", "element:a");
        let left = plan_with(
            &intent,
            PlanningContext::new(
                id("context:one"),
                1,
                vec![exact("authority:terrain", "cell:42", 7, "terrain-a")],
            )
            .unwrap(),
            vec![operation.clone()],
        );
        let right = plan_with(
            &intent,
            PlanningContext::new(
                id("context:one"),
                1,
                vec![exact("authority:terrain", "cell:42", 8, "terrain-b")],
            )
            .unwrap(),
            vec![operation],
        );
        assert_ne!(left.exact_ref(), right.exact_ref());
    }

    #[test]
    fn compiler_identity_changes_plan_identity_even_if_output_matches() {
        let intent = intent_with(vec![element("element:a", 0)]);
        let context = PlanningContext::new(id("context:one"), 1, Vec::new()).unwrap();
        let manifest_a = ConstructionPlanManifest::new(
            id("build-plan:shelter"),
            1,
            &intent,
            compiler(),
            context.clone(),
            vec![realize("op:a", "element:a")],
        )
        .unwrap();
        let manifest_b = ConstructionPlanManifest::new(
            id("build-plan:shelter"),
            1,
            &intent,
            exact("authority:planner", "planner:pb01.v1", 2, "compiler-b"),
            context,
            vec![realize("op:a", "element:a")],
        )
        .unwrap();
        assert_ne!(
            ConstructionPlan::seal(manifest_a).unwrap().exact_ref(),
            ConstructionPlan::seal(manifest_b).unwrap().exact_ref()
        );
    }

    #[test]
    fn stored_plan_must_resolve_against_exact_intent() {
        let intent = intent_with(vec![element("element:a", 0)]);
        let plan = plan_with(
            &intent,
            PlanningContext::new(id("context:one"), 1, Vec::new()).unwrap(),
            vec![realize("op:a", "element:a")],
        );
        let changed_intent = intent_with(vec![element("element:a", 10)]);
        assert!(matches!(
            plan.validate_against_intent(&changed_intent),
            Err(ProposalError::IntentReferenceMismatch)
        ));
    }

    #[test]
    fn hard_failures_and_advisories_remain_faceted() {
        let intent = intent_with(vec![element("element:a", 0)]);
        let plan = plan_with(
            &intent,
            PlanningContext::new(id("context:one"), 1, Vec::new()).unwrap(),
            vec![realize("op:a", "element:a")],
        );
        let advisory = ConstraintFinding::new(
            id("finding:wind"),
            id("rule:wind-exposure"),
            ConstraintSeverity::Advisory,
            vec![id("element:a")],
            id("message:wind-exposed"),
            Vec::new(),
        )
        .unwrap();
        let clean_report = ConstraintReport::new(
            plan.exact_ref(),
            id("constraints:pb01.v1"),
            None,
            vec![advisory],
        )
        .unwrap();
        assert!(!clean_report.has_hard_failures());

        let hard = ConstraintFinding::new(
            id("finding:permission"),
            id("rule:authority-denial"),
            ConstraintSeverity::HardFailure,
            vec![id("site:firstlight")],
            id("message:authority-denied"),
            Vec::new(),
        )
        .unwrap();
        let blocked_report = ConstraintReport::new(
            plan.exact_ref(),
            id("constraints:pb01.v1"),
            None,
            vec![hard],
        )
        .unwrap();
        assert!(blocked_report.has_hard_failures());
    }

    #[test]
    fn projection_is_bound_to_exact_plan_and_marks_unscheduled_elements() {
        let intent = intent_with(vec![element("element:a", 0), element("element:b", 1)]);
        let plan = plan_with(
            &intent,
            PlanningContext::new(id("context:one"), 1, Vec::new()).unwrap(),
            vec![realize("op:a", "element:a")],
        );
        let projection = BuildProjection::from_intent_and_plan(&intent, &plan).unwrap();
        assert_eq!(projection.elements().len(), 2);
        assert_eq!(
            projection.elements()[0].disposition,
            ProjectionDisposition::Planned
        );
        assert_eq!(
            projection.elements()[1].disposition,
            ProjectionDisposition::Unscheduled
        );
    }

    #[test]
    fn serialized_intent_content_tampering_fails_closed() {
        let intent = intent_with(vec![element("element:a", 0)]);
        let mut wire = serde_json::to_value(&intent).unwrap();
        wire["manifest"]["proposer_id"] = serde_json::Value::String("actor:other".into());
        let result = serde_json::from_value::<ConstructionIntent>(wire);
        assert!(result.is_err());
    }

    #[test]
    fn serialized_plan_content_tampering_fails_closed() {
        let intent = intent_with(vec![element("element:a", 0)]);
        let plan = plan_with(
            &intent,
            PlanningContext::new(id("context:one"), 1, Vec::new()).unwrap(),
            vec![realize("op:a", "element:a")],
        );
        let mut wire = serde_json::to_value(&plan).unwrap();
        wire["manifest"]["compiler_ref"]["content_digest"]["value"] =
            serde_json::Value::String("compiler-tampered".into());
        let result = serde_json::from_value::<ConstructionPlan>(wire);
        assert!(result.is_err());
    }

    #[test]
    fn build_projection_cannot_be_deserialized_as_canonical_state() {
        let intent = intent_with(vec![element("element:a", 0)]);
        let plan = plan_with(
            &intent,
            PlanningContext::new(id("context:one"), 1, Vec::new()).unwrap(),
            vec![realize("op:a", "element:a")],
        );
        let projection = BuildProjection::from_intent_and_plan(&intent, &plan).unwrap();
        let encoded = serde_json::to_string(&projection).unwrap();
        assert!(encoded.contains("element:a"));
    }
}
