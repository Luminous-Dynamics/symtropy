// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Canonical content identity for exact executable fabrication contracts.
//!
//! This module derives a compact identity from authority already established by
//! [`ExecutableFabricationPlan`]. Digest equality is content equality only: it
//! does not authenticate an issuer, prove that a provider is current, establish
//! process execution, engineering safety, commissioning, or civic permission.

use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};
use symtropy_game_state::StableId;

use crate::{ExecutableFabricationPlan, FabricationPlanId, ProcessKind, WorkpieceLifecycle};

/// Canonical executable-plan digest schema emitted by this implementation.
pub const EXECUTABLE_PLAN_DIGEST_SCHEMA_VERSION: u32 = 1;

/// Domain separation is independent of Rust/Serde representation.
const EXECUTABLE_PLAN_DIGEST_DOMAIN: &[u8] = b"symtropy:executable-fabrication-plan";

/// Digest algorithm fixed by schema v1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutablePlanDigestAlgorithm {
    Sha256,
}

/// Exact content identity of one canonical executable fabrication contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct ExecutablePlanDigest {
    schema_version: u32,
    algorithm: ExecutablePlanDigestAlgorithm,
    bytes: [u8; 32],
}

#[derive(Deserialize)]
struct ExecutablePlanDigestWire {
    schema_version: u32,
    algorithm: ExecutablePlanDigestAlgorithm,
    bytes: [u8; 32],
}

impl<'de> Deserialize<'de> for ExecutablePlanDigest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ExecutablePlanDigestWire::deserialize(deserializer)?;
        if wire.schema_version != EXECUTABLE_PLAN_DIGEST_SCHEMA_VERSION {
            return Err(serde::de::Error::custom(format!(
                "unsupported executable-plan digest schema version {}",
                wire.schema_version
            )));
        }
        if wire.algorithm != ExecutablePlanDigestAlgorithm::Sha256 {
            return Err(serde::de::Error::custom(
                "unsupported executable-plan digest algorithm",
            ));
        }
        Ok(Self {
            schema_version: wire.schema_version,
            algorithm: wire.algorithm,
            bytes: wire.bytes,
        })
    }
}

impl ExecutablePlanDigest {
    pub const fn schema_version(self) -> u32 {
        self.schema_version
    }

    pub const fn algorithm(self) -> ExecutablePlanDigestAlgorithm {
        self.algorithm
    }

    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.bytes
    }

    pub fn to_hex(self) -> String {
        hex(&self.bytes)
    }
}

impl fmt::Display for ExecutablePlanDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "sha256:{}", hex(&self.bytes))
    }
}

/// Compact exact reference suitable for downstream persistence/rebind records.
///
/// Deserialization restores only evidence-shaped data. Call
/// [`Self::validate_against`] with the complete strong executable plan before a
/// consequential boundary treats this reference as exact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutableFabricationPlanRef {
    plan_id: FabricationPlanId,
    plan_revision: u64,
    content_digest: ExecutablePlanDigest,
}

impl ExecutableFabricationPlanRef {
    pub fn plan_id(&self) -> &FabricationPlanId {
        &self.plan_id
    }

    pub const fn plan_revision(&self) -> u64 {
        self.plan_revision
    }

    pub const fn content_digest(&self) -> ExecutablePlanDigest {
        self.content_digest
    }

    pub fn validate_against(
        &self,
        plan: &ExecutableFabricationPlan,
    ) -> Result<(), ExecutablePlanIdentityError> {
        if self.plan_id != plan.plan().id || self.plan_revision != plan.plan().revision {
            return Err(ExecutablePlanIdentityError::PlanIdentityMismatch {
                expected_id: self.plan_id.clone(),
                expected_revision: self.plan_revision,
                actual_id: plan.plan().id.clone(),
                actual_revision: plan.plan().revision,
            });
        }
        let actual = plan.content_digest();
        if self.content_digest != actual {
            return Err(ExecutablePlanIdentityError::ContentDigestMismatch {
                expected: self.content_digest,
                actual,
            });
        }
        Ok(())
    }
}

impl ExecutableFabricationPlan {
    /// Computes canonical schema-v1 content identity over exact F10 + F4 + F5
    /// semantics. Runtime/provider/execution/commissioning state is excluded.
    pub fn content_digest(&self) -> ExecutablePlanDigest {
        let preimage = canonical_preimage_v1(self);
        let digest = Sha256::digest(preimage);
        let mut bytes = [0_u8; 32];
        bytes.copy_from_slice(&digest);
        ExecutablePlanDigest {
            schema_version: EXECUTABLE_PLAN_DIGEST_SCHEMA_VERSION,
            algorithm: ExecutablePlanDigestAlgorithm::Sha256,
            bytes,
        }
    }

    /// Derives a compact exact reference. The digest is never caller-authored.
    pub fn content_ref(&self) -> ExecutableFabricationPlanRef {
        ExecutableFabricationPlanRef {
            plan_id: self.plan().id.clone(),
            plan_revision: self.plan().revision,
            content_digest: self.content_digest(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutablePlanIdentityError {
    PlanIdentityMismatch {
        expected_id: FabricationPlanId,
        expected_revision: u64,
        actual_id: FabricationPlanId,
        actual_revision: u64,
    },
    ContentDigestMismatch {
        expected: ExecutablePlanDigest,
        actual: ExecutablePlanDigest,
    },
}

impl fmt::Display for ExecutablePlanIdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PlanIdentityMismatch {
                expected_id,
                expected_revision,
                actual_id,
                actual_revision,
            } => write!(
                formatter,
                "executable-plan reference expects {expected_id}@{expected_revision}, got {actual_id}@{actual_revision}"
            ),
            Self::ContentDigestMismatch { expected, actual } => write!(
                formatter,
                "executable-plan content digest mismatch: expected {expected}, got {actual}"
            ),
        }
    }
}

impl Error for ExecutablePlanIdentityError {}

#[derive(Default)]
struct CanonicalWriter {
    bytes: Vec<u8>,
}

impl CanonicalWriter {
    fn finish(self) -> Vec<u8> {
        self.bytes
    }

    fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn i64(&mut self, value: i64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn count(&mut self, len: usize) {
        self.u64(u64::try_from(len).expect("canonical executable-plan collection length fits u64"));
    }

    fn bytes(&mut self, value: &[u8]) {
        self.count(value.len());
        self.bytes.extend_from_slice(value);
    }

    fn text(&mut self, value: &str) {
        self.bytes(value.as_bytes());
    }

    fn stable_id(&mut self, value: &StableId) {
        self.text(value.as_str());
    }

    fn optional_stable_id(&mut self, value: Option<&StableId>) {
        match value {
            None => self.u8(0),
            Some(value) => {
                self.u8(1);
                self.stable_id(value);
            }
        }
    }

    fn optional_u64(&mut self, value: Option<u64>) {
        match value {
            None => self.u8(0),
            Some(value) => {
                self.u8(1);
                self.u64(value);
            }
        }
    }
}

fn canonical_preimage_v1(plan: &ExecutableFabricationPlan) -> Vec<u8> {
    let mut out = CanonicalWriter::default();
    out.bytes(EXECUTABLE_PLAN_DIGEST_DOMAIN);
    out.u32(EXECUTABLE_PLAN_DIGEST_SCHEMA_VERSION);

    let authored = plan.plan();
    out.stable_id(authored.id.stable_id());
    out.u64(authored.revision);

    out.count(authored.steps().len());
    for step in authored.steps() {
        out.stable_id(step.id.stable_id());
        out.stable_id(step.process_spec_id.stable_id());
        out.u64(step.process_spec_revision);

        out.count(step.workpieces().len());
        for workpiece in step.workpieces() {
            out.stable_id(workpiece.stable_id());
        }

        out.count(step.capability_needs().len());
        for need in step.capability_needs() {
            out.stable_id(need.stable_id());
        }

        out.count(step.expected_evidence_kinds().len());
        for evidence_kind in step.expected_evidence_kinds() {
            out.stable_id(evidence_kind);
        }
    }

    out.count(authored.dependencies().len());
    for dependency in authored.dependencies() {
        out.stable_id(dependency.prerequisite.stable_id());
        out.stable_id(dependency.dependent.stable_id());
    }

    out.count(plan.process_bindings().len());
    for binding in plan.process_bindings() {
        out.stable_id(binding.step_id.stable_id());
        let process = &binding.process_spec;
        out.stable_id(process.id.stable_id());
        out.u64(process.revision);
        out.u8(process_kind_tag(process.kind));

        out.count(process.required_capabilities().len());
        for requirement in process.required_capabilities() {
            out.stable_id(&requirement.capability_id);
            out.u64(requirement.minimum_value);
        }

        out.count(process.allowed_workpiece_states().len());
        for lifecycle in process.allowed_workpiece_states() {
            out.u8(workpiece_lifecycle_tag(*lifecycle));
        }
    }

    out.count(plan.capability_needs().len());
    for need in plan.capability_needs() {
        out.u32(need.schema_version());
        out.stable_id(need.id().stable_id());
        out.stable_id(need.capability_id());
        out.optional_stable_id(need.required_mode_id());

        out.count(need.axes().len());
        for axis in need.axes() {
            out.stable_id(&axis.axis_id);
            out.i64(axis.lower);
            out.i64(axis.upper);
            out.optional_u64(axis.max_resolution);
        }

        out.count(need.required_conditions().len());
        for condition in need.required_conditions() {
            out.stable_id(condition);
        }
    }

    out.finish()
}

/// Schema-v1 process tags. These are protocol values, not Rust enum ordinals.
const fn process_kind_tag(kind: ProcessKind) -> u8 {
    match kind {
        ProcessKind::Clean => 0,
        ProcessKind::Cut => 1,
        ProcessKind::Drill => 2,
        ProcessKind::Grind => 3,
        ProcessKind::Bend => 4,
        ProcessKind::Form => 5,
        ProcessKind::Align => 6,
        ProcessKind::Clamp => 7,
        ProcessKind::Fasten => 8,
        ProcessKind::Weld => 9,
        ProcessKind::Seal => 10,
        ProcessKind::Splice => 11,
        ProcessKind::Terminate => 12,
        ProcessKind::Coat => 13,
        ProcessKind::HeatTreat => 14,
        ProcessKind::Configure => 15,
        ProcessKind::Calibrate => 16,
        ProcessKind::Inspect => 17,
        ProcessKind::PressureTest => 18,
        ProcessKind::ContinuityTest => 19,
        ProcessKind::ReleaseClamp => 20,
        ProcessKind::Unfasten => 21,
        ProcessKind::Disconnect => 22,
        ProcessKind::Decouple => 23,
        ProcessKind::Unseal => 24,
        ProcessKind::CutFree => 25,
        ProcessKind::Extract => 26,
        ProcessKind::Couple => 27,
    }
}

/// Schema-v1 lifecycle tags. These are protocol values, not Rust enum ordinals.
const fn workpiece_lifecycle_tag(lifecycle: WorkpieceLifecycle) -> u8 {
    match lifecycle {
        WorkpieceLifecycle::Staged => 0,
        WorkpieceLifecycle::InProcess => 1,
        WorkpieceLifecycle::Available => 2,
        WorkpieceLifecycle::Installed => 3,
        WorkpieceLifecycle::Removed => 4,
        WorkpieceLifecycle::Retired => 5,
    }
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        output.push(char::from(DIGITS[usize::from(byte >> 4)]));
        output.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        CapabilityAdmissionId, CapabilityAxisNeed, CapabilityAxisRange, CapabilityEnvelope,
        CapabilityEvidenceRef, CapabilityNeed, CapabilityNeedId, CapabilityRequirement,
        ExactPlanProcessBinding, FabricationPlan, PlanDependency, PlanStep, PlanStepId,
        ProcessSpec, ProcessSpecId, WorkpieceId,
    };

    fn id(value: &str) -> StableId {
        StableId::parse(value).unwrap()
    }

    fn process_id(value: &str) -> ProcessSpecId {
        ProcessSpecId::new(id(value))
    }

    fn step_id(value: &str) -> PlanStepId {
        PlanStepId::new(id(value))
    }

    fn need_id(value: &str) -> CapabilityNeedId {
        CapabilityNeedId::new(id(value))
    }

    fn golden_executable(reverse_inputs: bool) -> ExecutableFabricationPlan {
        golden_executable_variant(
            reverse_inputs,
            "fabrication-plan:digest-golden",
            7,
            ProcessKind::Cut,
            "mode:cnc",
            "workpiece:b",
            "evidence:dimensional",
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn golden_executable_variant(
        reverse_inputs: bool,
        plan_id_text: &str,
        revision: u64,
        process_kind: ProcessKind,
        mode: &str,
        second_workpiece: &str,
        evidence_kind: &str,
    ) -> ExecutableFabricationPlan {
        let mut workpieces = vec![
            WorkpieceId::new(id("workpiece:a")),
            WorkpieceId::new(id(second_workpiece)),
        ];
        let mut states = vec![WorkpieceLifecycle::Staged, WorkpieceLifecycle::Available];
        let mut axes = vec![
            CapabilityAxisNeed::new(id("axis:x"), -10, 10, Some(1)).unwrap(),
            CapabilityAxisNeed::new(id("axis:z"), 0, 20, None).unwrap(),
        ];
        let mut conditions = vec![id("condition:guard"), id("condition:vacuum")];
        if reverse_inputs {
            workpieces.reverse();
            states.reverse();
            axes.reverse();
            conditions.reverse();
        }

        let need_id = need_id("capability-need:cut");
        let step = PlanStep::new(
            step_id("step:cut"),
            process_id("process-spec:cut"),
            2,
            workpieces,
            vec![need_id.clone()],
            vec![id(evidence_kind)],
        )
        .unwrap();
        let plan = FabricationPlan::new(
            FabricationPlanId::new(id(plan_id_text)),
            revision,
            vec![step],
            Vec::new(),
        )
        .unwrap();

        let process = ProcessSpec::new(
            process_id("process-spec:cut"),
            2,
            process_kind,
            vec![CapabilityRequirement {
                capability_id: need_id.stable_id().clone(),
                minimum_value: 1,
            }],
            states,
        )
        .unwrap()
        .snapshot();
        let need = CapabilityNeed::new(
            need_id,
            id("capability:cut"),
            Some(id(mode)),
            axes,
            conditions,
        )
        .unwrap()
        .snapshot()
        .unwrap();

        ExecutableFabricationPlan::new_with_capability_needs(
            plan,
            vec![ExactPlanProcessBinding::new(step_id("step:cut"), process)],
            vec![need],
        )
        .unwrap()
    }

    fn two_step_with_dependency(reverse_edge: bool) -> ExecutableFabricationPlan {
        let cut = PlanStep::new(
            step_id("step:cut"),
            process_id("process-spec:cut"),
            1,
            vec![WorkpieceId::new(id("workpiece:cut"))],
            Vec::new(),
            vec![id("evidence:cut")],
        )
        .unwrap();
        let inspect = PlanStep::new(
            step_id("step:inspect"),
            process_id("process-spec:inspect"),
            1,
            vec![WorkpieceId::new(id("workpiece:inspect"))],
            Vec::new(),
            vec![id("evidence:inspect")],
        )
        .unwrap();
        let dependency = if reverse_edge {
            PlanDependency::new(step_id("step:inspect"), step_id("step:cut")).unwrap()
        } else {
            PlanDependency::new(step_id("step:cut"), step_id("step:inspect")).unwrap()
        };
        let plan = FabricationPlan::new(
            FabricationPlanId::new(id("fabrication-plan:dependency")),
            1,
            vec![inspect, cut],
            vec![dependency],
        )
        .unwrap();
        let cut_spec = ProcessSpec::new(
            process_id("process-spec:cut"),
            1,
            ProcessKind::Cut,
            Vec::new(),
            vec![WorkpieceLifecycle::Available],
        )
        .unwrap()
        .snapshot();
        let inspect_spec = ProcessSpec::new(
            process_id("process-spec:inspect"),
            1,
            ProcessKind::Inspect,
            Vec::new(),
            vec![WorkpieceLifecycle::Available],
        )
        .unwrap()
        .snapshot();
        ExecutableFabricationPlan::new(
            plan,
            vec![
                ExactPlanProcessBinding::new(step_id("step:inspect"), inspect_spec),
                ExactPlanProcessBinding::new(step_id("step:cut"), cut_spec),
            ],
        )
        .unwrap()
    }

    #[test]
    fn benign_reordering_of_reusable_inputs_has_one_digest() {
        assert_eq!(
            golden_executable(false).content_digest(),
            golden_executable(true).content_digest()
        );
    }

    #[test]
    fn golden_v1_digest_freezes_cross_layer_preimage() {
        assert_eq!(
            golden_executable(false).content_digest().to_hex(),
            "c1a830a601fa472b3c112282454de040bb2c148c8fcf5e90d11de81ba1822413"
        );
    }

    #[test]
    fn authority_relevant_f10_changes_change_digest() {
        let baseline = golden_executable(false).content_digest();
        assert_ne!(
            baseline,
            golden_executable_variant(
                false,
                "fabrication-plan:digest-golden",
                7,
                ProcessKind::Cut,
                "mode:cnc",
                "workpiece:c",
                "evidence:dimensional",
            )
            .content_digest()
        );
        assert_ne!(
            baseline,
            golden_executable_variant(
                false,
                "fabrication-plan:digest-golden",
                7,
                ProcessKind::Cut,
                "mode:cnc",
                "workpiece:b",
                "evidence:surface",
            )
            .content_digest()
        );
        assert_ne!(
            two_step_with_dependency(false).content_digest(),
            two_step_with_dependency(true).content_digest()
        );
    }

    #[test]
    fn plan_lineage_is_part_of_exact_digest() {
        let baseline = golden_executable(false).content_digest();
        assert_ne!(
            baseline,
            golden_executable_variant(
                false,
                "fabrication-plan:digest-other",
                7,
                ProcessKind::Cut,
                "mode:cnc",
                "workpiece:b",
                "evidence:dimensional",
            )
            .content_digest()
        );
        assert_ne!(
            baseline,
            golden_executable_variant(
                false,
                "fabrication-plan:digest-golden",
                8,
                ProcessKind::Cut,
                "mode:cnc",
                "workpiece:b",
                "evidence:dimensional",
            )
            .content_digest()
        );
    }

    #[test]
    fn same_process_identity_with_changed_f4_semantics_changes_digest() {
        assert_ne!(
            golden_executable_variant(
                false,
                "fabrication-plan:digest-golden",
                7,
                ProcessKind::Cut,
                "mode:cnc",
                "workpiece:b",
                "evidence:dimensional",
            )
            .content_digest(),
            golden_executable_variant(
                false,
                "fabrication-plan:digest-golden",
                7,
                ProcessKind::Drill,
                "mode:cnc",
                "workpiece:b",
                "evidence:dimensional",
            )
            .content_digest()
        );
    }

    #[test]
    fn same_need_identity_with_changed_f5_semantics_changes_digest() {
        assert_ne!(
            golden_executable_variant(
                false,
                "fabrication-plan:digest-golden",
                7,
                ProcessKind::Cut,
                "mode:cnc",
                "workpiece:b",
                "evidence:dimensional",
            )
            .content_digest(),
            golden_executable_variant(
                false,
                "fabrication-plan:digest-golden",
                7,
                ProcessKind::Cut,
                "mode:laser",
                "workpiece:b",
                "evidence:dimensional",
            )
            .content_digest()
        );
    }

    #[test]
    fn provider_state_is_outside_plan_digest() {
        let plan = golden_executable(false);
        let digest = plan.content_digest();
        let need = CapabilityNeed::new(
            need_id("capability-need:cut"),
            id("capability:cut"),
            Some(id("mode:cnc")),
            vec![CapabilityAxisNeed::new(id("axis:x"), -10, 10, Some(1)).unwrap()],
            vec![id("condition:guard")],
        )
        .unwrap();
        let evidence = CapabilityEvidenceRef::new(
            id("authority:metrology"),
            id("evidence:provider"),
            1,
            "digest-provider",
        )
        .unwrap();
        let provider_a = CapabilityEnvelope::new(
            id("provider:a"),
            1,
            id("capability:cut"),
            id("mode:cnc"),
            vec![CapabilityAxisRange::new(id("axis:x"), -20, 20, 1).unwrap()],
            vec![id("condition:guard")],
            evidence.clone(),
        )
        .unwrap();
        let provider_b = CapabilityEnvelope::new(
            id("provider:b"),
            99,
            id("capability:cut"),
            id("mode:cnc"),
            vec![CapabilityAxisRange::new(id("axis:x"), -100, 100, 1).unwrap()],
            vec![id("condition:guard")],
            evidence,
        )
        .unwrap();
        let admission_a = need.evaluate(CapabilityAdmissionId::new(id("admission:a")), &provider_a);
        let admission_b = need.evaluate(CapabilityAdmissionId::new(id("admission:b")), &provider_b);
        assert_ne!(admission_a.provider_id, admission_b.provider_id);
        assert_eq!(plan.content_digest(), digest);
    }

    #[test]
    fn digest_wire_rejects_unknown_schema_version() {
        let digest = golden_executable(false).content_digest();
        let mut value = serde_json::to_value(digest).unwrap();
        value["schema_version"] = serde_json::json!(99);
        assert!(serde_json::from_value::<ExecutablePlanDigest>(value).is_err());
    }

    #[test]
    fn round_trip_preserves_digest_and_exact_ref() {
        let plan = golden_executable(false);
        let encoded = serde_json::to_vec(&plan).unwrap();
        let restored: ExecutableFabricationPlan = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(restored.content_digest(), plan.content_digest());

        let reference = plan.content_ref();
        let encoded_ref = serde_json::to_vec(&reference).unwrap();
        let restored_ref: ExecutableFabricationPlanRef =
            serde_json::from_slice(&encoded_ref).unwrap();
        restored_ref.validate_against(&restored).unwrap();
    }

    #[test]
    fn compact_ref_rejects_same_lineage_changed_exact_content() {
        let original = golden_executable(false);
        let changed = golden_executable_variant(
            false,
            "fabrication-plan:digest-golden",
            7,
            ProcessKind::Cut,
            "mode:laser",
            "workpiece:b",
            "evidence:dimensional",
        );
        let reference = original.content_ref();
        assert_eq!(reference.plan_id(), &changed.plan().id);
        assert_eq!(reference.plan_revision(), changed.plan().revision);
        assert!(matches!(
            reference.validate_against(&changed),
            Err(ExecutablePlanIdentityError::ContentDigestMismatch { .. })
        ));
    }
}
