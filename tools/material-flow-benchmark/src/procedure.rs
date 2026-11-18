// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Typed, version-bound executable-procedure graph for MFB.
//!
//! The graph declares evidence dependencies and a safe ordering. It does not
//! authenticate evidence, grant motion authority, emit physical commands,
//! execute a robot, or attest task success. "Ready" means structurally ready
//! for independent external review/execution, never self-authorized.

use std::collections::{BTreeSet, HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::ScenarioManifest;

pub const PROCEDURE_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcedureGraph {
    pub schema_version: u16,
    pub procedure_id: String,
    pub procedure_version: u32,
    pub binding: ProcedureBinding,
    pub steps: Vec<ProcedureStep>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcedureBinding {
    pub scenario_id: String,
    pub source_commit: String,
    pub configuration_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcedureStep {
    pub step_id: String,
    pub operation: ProcedureOperation,
    pub depends_on: Vec<String>,
    pub consumes: Vec<EvidenceKind>,
    pub produces: EvidenceKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcedureOperation {
    ValidateManifest,
    VerifyAuthorizationReceipt,
    VerifyReceiverCapability,
    VerifyPackageIdentity,
    VerifyContainerFit,
    VerifyPayloadMass,
    VerifySealIntegrity,
    VerifyTemperatureEnvelope,
    VerifyTransferGateInterlock,
    PrepareReleaseIntent,
    ObserveSourceRelease,
    ObserveReceiverArrival,
    VerifyPlacement,
    CollectCustodyAcknowledgement,
    ReconcileUnknownOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    // Inputs that must arrive from an external, independently governed source.
    ManifestSnapshot,
    AuthorizationReceipt,
    ReceiverCapabilityReceipt,
    PackageIdentityObservation,
    ContainerFitObservation,
    PayloadMassObservation,
    SealObservation,
    TemperatureObservation,
    TransferGateObservation,
    ReleaseExecutionReceipt,
    ReceiverPresenceObservation,
    PlacementObservation,
    CounterpartyCustodyReceipt,
    AmbiguousOutcomeObservation,

    // Derived evidence nodes emitted by validated procedure steps.
    ManifestValidated,
    AuthorizationValidated,
    ReceiverCapabilityValidated,
    PackageIdentityValidated,
    ContainerFitValidated,
    PayloadMassValidated,
    SealValidated,
    TemperatureValidated,
    TransferGateInterlockValidated,
    ReleaseIntentReady,
    SourceReleaseObserved,
    ReceiverArrivalObserved,
    PlacementVerified,
    CustodyEvidenceComplete,
    ReconciliationRecord,
}

impl EvidenceKind {
    /// Maximum age of an external receipt at the latest plausible decision
    /// time. These conservative v1 limits are part of the benchmark policy
    /// and must be changed only with a versioned, reviewed policy update.
    pub fn maximum_observation_age_ms(self) -> Option<u64> {
        use EvidenceKind as E;
        match self {
            E::ManifestSnapshot => Some(300_000),
            E::AuthorizationReceipt => Some(60_000),
            E::ReceiverCapabilityReceipt => Some(86_400_000),
            E::PackageIdentityObservation => Some(2_000),
            E::ContainerFitObservation => Some(86_400_000),
            E::PayloadMassObservation => Some(2_000),
            E::SealObservation => Some(5_000),
            E::TemperatureObservation => Some(2_000),
            E::TransferGateObservation => Some(500),
            E::ReleaseExecutionReceipt => Some(10_000),
            E::ReceiverPresenceObservation => Some(500),
            E::PlacementObservation => Some(1_000),
            E::CounterpartyCustodyReceipt => Some(30_000),
            E::AmbiguousOutcomeObservation => Some(5_000),
            E::ManifestValidated
            | E::AuthorizationValidated
            | E::ReceiverCapabilityValidated
            | E::PackageIdentityValidated
            | E::ContainerFitValidated
            | E::PayloadMassValidated
            | E::SealValidated
            | E::TemperatureValidated
            | E::TransferGateInterlockValidated
            | E::ReleaseIntentReady
            | E::SourceReleaseObserved
            | E::ReceiverArrivalObserved
            | E::PlacementVerified
            | E::CustodyEvidenceComplete
            | E::ReconciliationRecord => None,
        }
    }
}

impl ProcedureOperation {
    pub fn required_inputs(self) -> &'static [EvidenceKind] {
        use EvidenceKind as E;
        use ProcedureOperation as O;
        match self {
            O::ValidateManifest => &[E::ManifestSnapshot],
            O::VerifyAuthorizationReceipt => &[E::AuthorizationReceipt],
            O::VerifyReceiverCapability => &[E::ReceiverCapabilityReceipt],
            O::VerifyPackageIdentity => &[E::PackageIdentityObservation],
            O::VerifyContainerFit => &[E::ContainerFitObservation],
            O::VerifyPayloadMass => &[E::PayloadMassObservation],
            O::VerifySealIntegrity => &[E::SealObservation],
            O::VerifyTemperatureEnvelope => &[E::TemperatureObservation],
            O::VerifyTransferGateInterlock => &[E::TransferGateObservation],
            O::PrepareReleaseIntent => &[
                E::ManifestValidated,
                E::AuthorizationValidated,
                E::ReceiverCapabilityValidated,
                E::PackageIdentityValidated,
                E::ContainerFitValidated,
                E::PayloadMassValidated,
                E::SealValidated,
                E::TemperatureValidated,
                E::TransferGateInterlockValidated,
            ],
            O::ObserveSourceRelease => &[E::ReleaseIntentReady, E::ReleaseExecutionReceipt],
            O::ObserveReceiverArrival => {
                &[E::SourceReleaseObserved, E::ReceiverPresenceObservation]
            }
            O::VerifyPlacement => &[E::ReceiverArrivalObserved, E::PlacementObservation],
            O::CollectCustodyAcknowledgement => {
                &[E::PlacementVerified, E::CounterpartyCustodyReceipt]
            }
            O::ReconcileUnknownOutcome => &[E::ReleaseIntentReady, E::AmbiguousOutcomeObservation],
        }
    }

    pub fn produced_output(self) -> EvidenceKind {
        use EvidenceKind as E;
        use ProcedureOperation as O;
        match self {
            O::ValidateManifest => E::ManifestValidated,
            O::VerifyAuthorizationReceipt => E::AuthorizationValidated,
            O::VerifyReceiverCapability => E::ReceiverCapabilityValidated,
            O::VerifyPackageIdentity => E::PackageIdentityValidated,
            O::VerifyContainerFit => E::ContainerFitValidated,
            O::VerifyPayloadMass => E::PayloadMassValidated,
            O::VerifySealIntegrity => E::SealValidated,
            O::VerifyTemperatureEnvelope => E::TemperatureValidated,
            O::VerifyTransferGateInterlock => E::TransferGateInterlockValidated,
            O::PrepareReleaseIntent => E::ReleaseIntentReady,
            O::ObserveSourceRelease => E::SourceReleaseObserved,
            O::ObserveReceiverArrival => E::ReceiverArrivalObserved,
            O::VerifyPlacement => E::PlacementVerified,
            O::CollectCustodyAcknowledgement => E::CustodyEvidenceComplete,
            O::ReconcileUnknownOutcome => E::ReconciliationRecord,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProcedureReadinessReport {
    pub schema_version: u16,
    pub procedure_id: String,
    pub procedure_version: u32,
    pub bound_scenario_id: String,
    pub bound_source_commit: String,
    pub bound_configuration_digest: String,
    pub topological_order: Vec<String>,
    pub step_count: usize,
    pub structurally_valid_for_external_review: bool,
    pub motion_authority_granted: bool,
    pub physical_command_emitted: bool,
    pub task_success_attested: bool,
    pub limitations: Vec<String>,
}

impl ProcedureGraph {
    /// Validate the graph against an exact scenario identity and return a
    /// deterministic topological order. A successful report is only a
    /// structural plan check; it cannot authorize physical execution.
    pub fn evaluate(
        &self,
        scenario: &ScenarioManifest,
    ) -> Result<ProcedureReadinessReport, Vec<String>> {
        let mut errors = Vec::new();

        if self.schema_version != PROCEDURE_SCHEMA_VERSION {
            errors.push(format!(
                "procedure.schema_version: expected {PROCEDURE_SCHEMA_VERSION}, got {}",
                self.schema_version
            ));
        }
        if self.procedure_id.trim().is_empty()
            || self.procedure_id.len() > 128
            || !self
                .procedure_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
        {
            errors.push("procedure.procedure_id: invalid opaque identifier".into());
        }
        if self.procedure_version == 0 {
            errors.push("procedure.procedure_version: must be non-zero".into());
        }
        if self.binding.scenario_id != scenario.scenario_id {
            errors.push("procedure.binding.scenario_id: exact scenario mismatch".into());
        }
        if self.binding.source_commit != scenario.digests.source_commit {
            errors.push("procedure.binding.source_commit: exact source commit mismatch".into());
        }
        if self.binding.configuration_digest != scenario.digests.configuration_digest {
            errors.push("procedure.binding.configuration_digest: configuration mismatch".into());
        }

        if self.steps.is_empty() || self.steps.len() > 128 {
            errors.push("procedure.steps: expected 1..=128 steps".into());
        }

        let mut step_ids = HashSet::new();
        let mut operation_ids = HashSet::new();
        let mut steps_by_id = HashMap::new();
        for (index, step) in self.steps.iter().enumerate() {
            let prefix = format!("procedure.steps[{index}]");
            if step.step_id.trim().is_empty()
                || step.step_id.len() > 96
                || !step
                    .step_id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
            {
                errors.push(format!("{prefix}.step_id: invalid opaque identifier"));
            }
            if !step_ids.insert(step.step_id.as_str()) {
                errors.push(format!("{prefix}.step_id: duplicate step identifier"));
            }
            if !operation_ids.insert(step.operation) {
                errors.push(format!(
                    "{prefix}.operation: v1 procedure permits one step per operation"
                ));
            }
            let declared_inputs = step.consumes.iter().copied().collect::<BTreeSet<_>>();
            let expected_inputs = step
                .operation
                .required_inputs()
                .iter()
                .copied()
                .collect::<BTreeSet<_>>();
            if declared_inputs.len() != step.consumes.len() {
                errors.push(format!("{prefix}.consumes: duplicate evidence kind"));
            }
            if declared_inputs != expected_inputs {
                errors.push(format!(
                    "{prefix}.consumes: does not match the operation's typed input contract"
                ));
            }
            if step.produces != step.operation.produced_output() {
                errors.push(format!(
                    "{prefix}.produces: does not match the operation's typed output contract"
                ));
            }
            if steps_by_id.insert(step.step_id.as_str(), step).is_some() {
                errors.push(format!("{prefix}.step_id: duplicate lookup key"));
            }
        }

        // Every v1 procedure is a complete receiving/put-away lifecycle,
        // including a recovery branch. A partial graph is not executable.
        const REQUIRED_OPERATIONS: [ProcedureOperation; 15] = [
            ProcedureOperation::ValidateManifest,
            ProcedureOperation::VerifyAuthorizationReceipt,
            ProcedureOperation::VerifyReceiverCapability,
            ProcedureOperation::VerifyPackageIdentity,
            ProcedureOperation::VerifyContainerFit,
            ProcedureOperation::VerifyPayloadMass,
            ProcedureOperation::VerifySealIntegrity,
            ProcedureOperation::VerifyTemperatureEnvelope,
            ProcedureOperation::VerifyTransferGateInterlock,
            ProcedureOperation::PrepareReleaseIntent,
            ProcedureOperation::ObserveSourceRelease,
            ProcedureOperation::ObserveReceiverArrival,
            ProcedureOperation::VerifyPlacement,
            ProcedureOperation::CollectCustodyAcknowledgement,
            ProcedureOperation::ReconcileUnknownOutcome,
        ];
        for operation in REQUIRED_OPERATIONS {
            if !operation_ids.contains(&operation) {
                errors.push(format!(
                    "procedure.steps: missing required operation {operation:?}"
                ));
            }
        }

        let mut indegree: HashMap<&str, usize> = HashMap::new();
        let mut outgoing: HashMap<&str, Vec<&str>> = HashMap::new();
        for step in &self.steps {
            indegree.entry(step.step_id.as_str()).or_insert(0);
            let mut dependencies = HashSet::new();
            for dependency in &step.depends_on {
                if !dependencies.insert(dependency.as_str()) {
                    errors.push(format!(
                        "procedure.step[{}].depends_on: duplicate dependency {dependency}",
                        step.step_id
                    ));
                    continue;
                }
                if dependency == &step.step_id {
                    errors.push(format!(
                        "procedure.step[{}].depends_on: self-dependency",
                        step.step_id
                    ));
                    continue;
                }
                if !steps_by_id.contains_key(dependency.as_str()) {
                    errors.push(format!(
                        "procedure.step[{}].depends_on: unknown step {dependency}",
                        step.step_id
                    ));
                    continue;
                }
                *indegree.entry(step.step_id.as_str()).or_insert(0) += 1;
                outgoing
                    .entry(dependency.as_str())
                    .or_default()
                    .push(step.step_id.as_str());
            }
        }

        let mut ready = self
            .steps
            .iter()
            .filter(|step| indegree.get(step.step_id.as_str()).copied().unwrap_or(0) == 0)
            .map(|step| step.step_id.as_str())
            .collect::<BTreeSet<_>>();
        let mut topological_order = Vec::with_capacity(self.steps.len());
        while let Some(next) = ready.pop_first() {
            topological_order.push(next);
            for dependent in outgoing.get(next).into_iter().flatten() {
                if let Some(value) = indegree.get_mut(dependent) {
                    *value -= 1;
                    if *value == 0 {
                        ready.insert(dependent);
                    }
                }
            }
        }
        if topological_order.len() != self.steps.len() {
            errors.push("procedure.steps: dependency cycle detected".into());
        }

        // Compute transitive dependency closure for every node. Typed evidence
        // may be consumed only from a declared external input or an ancestor.
        let mut ancestors: HashMap<&str, HashSet<&str>> = HashMap::new();
        for step_id in &topological_order {
            let step = steps_by_id[step_id];
            let mut inherited = HashSet::new();
            for dependency in &step.depends_on {
                inherited.insert(dependency.as_str());
                if let Some(parent_ancestors) = ancestors.get(dependency.as_str()) {
                    inherited.extend(parent_ancestors.iter().copied());
                }
            }
            ancestors.insert(step_id, inherited);
        }

        let external_inputs = [
            EvidenceKind::ManifestSnapshot,
            EvidenceKind::AuthorizationReceipt,
            EvidenceKind::ReceiverCapabilityReceipt,
            EvidenceKind::PackageIdentityObservation,
            EvidenceKind::ContainerFitObservation,
            EvidenceKind::PayloadMassObservation,
            EvidenceKind::SealObservation,
            EvidenceKind::TemperatureObservation,
            EvidenceKind::TransferGateObservation,
            EvidenceKind::ReleaseExecutionReceipt,
            EvidenceKind::ReceiverPresenceObservation,
            EvidenceKind::PlacementObservation,
            EvidenceKind::CounterpartyCustodyReceipt,
            EvidenceKind::AmbiguousOutcomeObservation,
        ];
        let external_inputs = external_inputs.into_iter().collect::<HashSet<_>>();

        for step in &self.steps {
            let ancestor_ids = ancestors.get(step.step_id.as_str());
            for input in &step.consumes {
                let externally_supplied = external_inputs.contains(input);
                let produced_by_ancestor = ancestor_ids.is_some_and(|ids| {
                    ids.iter().any(|ancestor_id| {
                        steps_by_id
                            .get(ancestor_id)
                            .is_some_and(|ancestor| ancestor.produces == *input)
                    })
                });
                if !externally_supplied && !produced_by_ancestor {
                    errors.push(format!(
                        "procedure.step[{}].consumes: {input:?} is not supplied by an external input or dependency ancestor",
                        step.step_id
                    ));
                }
            }
        }

        // The intent cannot be prepared until all independent preflight
        // gates are ancestors. This prevents graph order from becoming a
        // shortcut around authority, capability, identity or safety checks.
        let preflight = [
            ProcedureOperation::ValidateManifest,
            ProcedureOperation::VerifyAuthorizationReceipt,
            ProcedureOperation::VerifyReceiverCapability,
            ProcedureOperation::VerifyPackageIdentity,
            ProcedureOperation::VerifyContainerFit,
            ProcedureOperation::VerifyPayloadMass,
            ProcedureOperation::VerifySealIntegrity,
            ProcedureOperation::VerifyTemperatureEnvelope,
            ProcedureOperation::VerifyTransferGateInterlock,
        ];
        for (operation, required_ancestor_operations) in [
            (
                ProcedureOperation::PrepareReleaseIntent,
                preflight.as_slice(),
            ),
            (
                ProcedureOperation::ObserveSourceRelease,
                &[ProcedureOperation::PrepareReleaseIntent][..],
            ),
            (
                ProcedureOperation::ObserveReceiverArrival,
                &[ProcedureOperation::ObserveSourceRelease][..],
            ),
            (
                ProcedureOperation::VerifyPlacement,
                &[ProcedureOperation::ObserveReceiverArrival][..],
            ),
            (
                ProcedureOperation::CollectCustodyAcknowledgement,
                &[ProcedureOperation::VerifyPlacement][..],
            ),
            (
                ProcedureOperation::ReconcileUnknownOutcome,
                &[ProcedureOperation::PrepareReleaseIntent][..],
            ),
        ] {
            let Some(step) = self.steps.iter().find(|step| step.operation == operation) else {
                continue;
            };
            let ancestor_ids = ancestors.get(step.step_id.as_str());
            for required in required_ancestor_operations {
                let exists = ancestor_ids.is_some_and(|ids| {
                    ids.iter().any(|ancestor_id| {
                        steps_by_id
                            .get(ancestor_id)
                            .is_some_and(|ancestor| ancestor.operation == *required)
                    })
                });
                if !exists {
                    errors.push(format!(
                        "procedure.operation.{operation:?}: required predecessor {required:?} is not an ancestor"
                    ));
                }
            }
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        Ok(ProcedureReadinessReport {
            schema_version: self.schema_version,
            procedure_id: self.procedure_id.clone(),
            procedure_version: self.procedure_version,
            bound_scenario_id: self.binding.scenario_id.clone(),
            bound_source_commit: self.binding.source_commit.clone(),
            bound_configuration_digest: self.binding.configuration_digest.clone(),
            topological_order: topological_order
                .into_iter()
                .map(str::to_string)
                .collect(),
            step_count: self.steps.len(),
            structurally_valid_for_external_review: true,
            motion_authority_granted: false,
            physical_command_emitted: false,
            task_success_attested: false,
            limitations: vec![
                "Validates only procedure graph shape, typed evidence dependencies and exact scenario binding".into(),
                "External evidence values, signatures, freshness and sensor truth are not authenticated".into(),
                "The procedure graph cannot grant motion authority or emit a physical command".into(),
                "An independent executor, safety authority and observer/evaluator remain mandatory".into(),
            ],
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcedureExecutionBranch {
    NormalCompletion,
    Reconciliation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcedureEvidenceBundle {
    pub schema_version: u16,
    pub procedure_id: String,
    pub procedure_version: u32,
    pub execution_branch: ProcedureExecutionBranch,
    pub receipts: Vec<ProcedureEvidenceReceipt>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcedureEvidenceReceipt {
    pub evidence_id: String,
    pub kind: EvidenceKind,
    pub subject_id: String,
    pub issuer_actor_id: String,
    pub scenario_id: String,
    pub source_commit: String,
    pub configuration_digest: String,
    pub digest: String,
    pub observed_at_unix_ms: u64,
    pub expires_at_unix_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RequiredVerificationDomain {
    HouseholdGrantAuthority,
    ServiceProviderAttestation,
    ReceiverCapabilityAttestation,
    PhysicalObservationAttestation,
    CounterpartyCustodyAttestation,
    TransactionOutcomeAttestation,
}

impl EvidenceKind {
    /// Identifies the downstream trust domain required for this receipt.
    /// This classifies future verifier responsibility; it does not perform
    /// authentication or issuer authorization.
    pub fn required_verification_domain(self) -> Option<RequiredVerificationDomain> {
        use EvidenceKind as E;
        use RequiredVerificationDomain as D;
        match self {
            E::ManifestSnapshot | E::PackageIdentityObservation | E::ReleaseExecutionReceipt => {
                Some(D::ServiceProviderAttestation)
            }
            E::AuthorizationReceipt => Some(D::HouseholdGrantAuthority),
            E::ReceiverCapabilityReceipt => Some(D::ReceiverCapabilityAttestation),
            E::ContainerFitObservation
            | E::PayloadMassObservation
            | E::SealObservation
            | E::TemperatureObservation
            | E::TransferGateObservation
            | E::ReceiverPresenceObservation
            | E::PlacementObservation => Some(D::PhysicalObservationAttestation),
            E::CounterpartyCustodyReceipt => Some(D::CounterpartyCustodyAttestation),
            E::AmbiguousOutcomeObservation => Some(D::TransactionOutcomeAttestation),
            E::ManifestValidated
            | E::AuthorizationValidated
            | E::ReceiverCapabilityValidated
            | E::PackageIdentityValidated
            | E::ContainerFitValidated
            | E::PayloadMassValidated
            | E::SealValidated
            | E::TemperatureValidated
            | E::TransferGateInterlockValidated
            | E::ReleaseIntentReady
            | E::SourceReleaseObserved
            | E::ReceiverArrivalObserved
            | E::PlacementVerified
            | E::CustodyEvidenceComplete
            | E::ReconciliationRecord => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProcedureEvidenceAdmissionReport {
    pub schema_version: u16,
    pub freshness_policy_version: u16,
    pub procedure_id: String,
    pub procedure_version: u32,
    pub scenario_id: String,
    pub execution_branch: ProcedureExecutionBranch,
    pub admitted_receipt_count: usize,
    pub admitted_kinds: Vec<EvidenceKind>,
    pub required_verification_domains: Vec<RequiredVerificationDomain>,
    pub evidence_envelopes_structurally_admitted: bool,
    pub declared_freshness_policy_passed: bool,
    pub content_digest_verified: bool,
    pub cryptographic_authentication_verified: bool,
    pub issuer_authorization_verified: bool,
    pub trusted_time_verified: bool,
    pub sensor_truth_established: bool,
    pub motion_authority_granted: bool,
    pub physical_command_emitted: bool,
    pub task_success_attested: bool,
    pub limitations: Vec<String>,
}

impl ProcedureGraph {
    /// Check that every external evidence input required by the graph has
    /// exactly one well-formed, correctly scoped and fresh receipt.
    ///
    /// This is structural admission, not cryptographic authentication. A
    /// production caller must still verify the issuer signature, revocation
    /// state, hardware attestation and sensor-specific truth conditions.
    pub fn admit_evidence(
        &self,
        scenario: &ScenarioManifest,
        bundle: &ProcedureEvidenceBundle,
    ) -> Result<ProcedureEvidenceAdmissionReport, Vec<String>> {
        let _graph_report = self.evaluate(scenario)?;
        let mut errors = Vec::new();

        if bundle.schema_version != 1 {
            errors.push(format!(
                "evidence_bundle.schema_version: expected 1, got {}",
                bundle.schema_version
            ));
        }
        if bundle.procedure_id != self.procedure_id {
            errors.push("evidence_bundle.procedure_id: mismatch".into());
        }
        if bundle.procedure_version != self.procedure_version {
            errors.push("evidence_bundle.procedure_version: mismatch".into());
        }

        // Evidence closure is computed for the selected terminal branch.
        // A normal completion must not require ambiguity evidence; a
        // reconciliation must not require successful placement/custody proof.
        let terminal_operation = match bundle.execution_branch {
            ProcedureExecutionBranch::NormalCompletion => {
                ProcedureOperation::CollectCustodyAcknowledgement
            }
            ProcedureExecutionBranch::Reconciliation => ProcedureOperation::ReconcileUnknownOutcome,
        };
        let terminal_step = self
            .steps
            .iter()
            .find(|step| step.operation == terminal_operation)
            .ok_or_else(|| {
                vec![format!(
                    "procedure graph is missing terminal operation {terminal_operation:?}"
                )]
            })?;

        let steps_by_id = self
            .steps
            .iter()
            .map(|step| (step.step_id.as_str(), step))
            .collect::<HashMap<_, _>>();
        let mut active_step_ids = HashSet::new();
        let mut pending_step_ids = vec![terminal_step.step_id.as_str()];
        while let Some(step_id) = pending_step_ids.pop() {
            if !active_step_ids.insert(step_id) {
                continue;
            }
            if let Some(step) = steps_by_id.get(step_id) {
                pending_step_ids.extend(step.depends_on.iter().map(String::as_str));
            }
        }

        let internal_outputs = self
            .steps
            .iter()
            .filter(|step| active_step_ids.contains(step.step_id.as_str()))
            .map(|step| step.produces)
            .collect::<HashSet<_>>();
        let expected_external = self
            .steps
            .iter()
            .filter(|step| active_step_ids.contains(step.step_id.as_str()))
            .flat_map(|step| step.operation.required_inputs().iter().copied())
            .filter(|kind| !internal_outputs.contains(kind))
            .collect::<BTreeSet<_>>();

        let mut receipt_ids = HashSet::new();
        let mut receipt_kinds = HashSet::new();
        for (index, receipt) in bundle.receipts.iter().enumerate() {
            let prefix = format!("evidence_bundle.receipts[{index}]");
            if receipt.evidence_id.trim().is_empty()
                || receipt.evidence_id.len() > 128
                || !receipt
                    .evidence_id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
            {
                errors.push(format!("{prefix}.evidence_id: invalid opaque identifier"));
            }
            if !receipt_ids.insert(receipt.evidence_id.as_str()) {
                errors.push(format!("{prefix}.evidence_id: duplicate receipt identity"));
            }
            if !receipt_kinds.insert(receipt.kind) {
                errors.push(format!("{prefix}.kind: duplicate receipt kind"));
            }
            if !expected_external.contains(&receipt.kind) {
                errors.push(format!(
                    "{prefix}.kind: unexpected or internally-produced evidence kind {:?}",
                    receipt.kind
                ));
            }
            if receipt.scenario_id != scenario.scenario_id {
                errors.push(format!("{prefix}.scenario_id: scenario binding mismatch"));
            }
            if receipt.source_commit != scenario.digests.source_commit {
                errors.push(format!("{prefix}.source_commit: source binding mismatch"));
            }
            if receipt.configuration_digest != scenario.digests.configuration_digest {
                errors.push(format!(
                    "{prefix}.configuration_digest: configuration binding mismatch"
                ));
            }
            if !is_sha256_digest(&receipt.digest) {
                errors.push(format!("{prefix}.digest: malformed SHA-256 digest"));
            }

            let expected_subject = expected_subject_id(receipt.kind, scenario);
            match expected_subject {
                Some(expected) if receipt.subject_id == expected => {}
                Some(_) => errors.push(format!(
                    "{prefix}.subject_id: subject does not match evidence kind {:?}",
                    receipt.kind
                )),
                None => errors.push(format!(
                    "{prefix}.subject_id: no scenario subject exists for evidence kind {:?}",
                    receipt.kind
                )),
            }

            let issuer = scenario
                .actors
                .iter()
                .find(|actor| actor.actor_id == receipt.issuer_actor_id);
            match issuer {
                Some(actor) if allowed_issuer_roles(receipt.kind).contains(&actor.role) => {}
                Some(_) => errors.push(format!(
                    "{prefix}.issuer_actor_id: issuer role is not valid for evidence kind {:?}",
                    receipt.kind
                )),
                None => errors.push(format!("{prefix}.issuer_actor_id: unknown actor")),
            }

            if receipt.expires_at_unix_ms <= receipt.observed_at_unix_ms {
                errors.push(format!("{prefix}: expiry must be later than observation"));
            }
            let clock_uncertainty_ms = u64::from(scenario.clock_uncertainty_ms);
            let latest_possible_decision_time = scenario
                .started_at_unix_ms
                .saturating_add(clock_uncertainty_ms);
            let earliest_possible_decision_time = scenario
                .started_at_unix_ms
                .saturating_sub(clock_uncertainty_ms);
            // Fail closed across the entire uncertainty interval: the receipt
            // must already exist at the earliest plausible decision time and
            // remain valid through the latest plausible decision time.
            if receipt.observed_at_unix_ms >= earliest_possible_decision_time {
                errors.push(format!(
                    "{prefix}: observation is not strictly earlier than the full decision uncertainty window"
                ));
            }
            if receipt.expires_at_unix_ms <= latest_possible_decision_time {
                errors.push(format!(
                    "{prefix}: expiry does not strictly cover the full decision uncertainty window"
                ));
            }
            if let Some(maximum_age_ms) = receipt.kind.maximum_observation_age_ms() {
                let worst_case_age_ms =
                    latest_possible_decision_time.saturating_sub(receipt.observed_at_unix_ms);
                if worst_case_age_ms > maximum_age_ms {
                    errors.push(format!(
                        "{prefix}: observation age {worst_case_age_ms} ms exceeds the v1 limit {maximum_age_ms} ms for {:?}",
                        receipt.kind
                    ));
                }
            }
        }

        for expected in &expected_external {
            if !receipt_kinds.contains(expected) {
                errors.push(format!(
                    "evidence_bundle.receipts: missing required external evidence kind {expected:?}"
                ));
            }
        }
        if receipt_kinds.len() != expected_external.len() {
            errors.push(
                "evidence_bundle.receipts: evidence kind set is incomplete or contains extras"
                    .into(),
            );
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        let required_verification_domains = bundle
            .receipts
            .iter()
            .filter_map(|receipt| receipt.kind.required_verification_domain())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();

        Ok(ProcedureEvidenceAdmissionReport {
            schema_version: bundle.schema_version,
            freshness_policy_version: 1,
            procedure_id: self.procedure_id.clone(),
            procedure_version: self.procedure_version,
            scenario_id: scenario.scenario_id.clone(),
            execution_branch: bundle.execution_branch,
            admitted_receipt_count: bundle.receipts.len(),
            admitted_kinds: receipt_kinds.into_iter().collect::<BTreeSet<_>>().into_iter().collect(),
            required_verification_domains,
            evidence_envelopes_structurally_admitted: true,
            declared_freshness_policy_passed: true,
            content_digest_verified: false,
            cryptographic_authentication_verified: false,
            issuer_authorization_verified: false,
            trusted_time_verified: false,
            sensor_truth_established: false,
            motion_authority_granted: false,
            physical_command_emitted: false,
            task_success_attested: false,
            limitations: vec![
                "Checks declared timestamps and age limits against the scenario clock window; no trusted clock or timestamp authority is verified".into(),
                "Checks SHA-256 digest syntax only; it does not recompute a digest from canonical evidence content".into(),
                "Required verification domains identify future verifier responsibility; they are not authenticated here".into(),
                "Does not verify cryptographic signatures, revocation, hardware attestation or actual sensor truth".into(),
                "Does not prove that an issuer is authorized or that multiple evidence sources are independent".into(),
                "Evidence admission grants no motion authority and performs no physical command".into(),
            ],
        })
    }
}

fn is_sha256_digest(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64
        && hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn expected_subject_id(kind: EvidenceKind, scenario: &ScenarioManifest) -> Option<String> {
    use EvidenceKind as E;
    match kind {
        E::ManifestSnapshot => Some(scenario.scenario_id.clone()),
        E::AuthorizationReceipt => Some(scenario.authorization.grant_digest.clone()),
        E::ReceiverCapabilityReceipt | E::ReceiverPresenceObservation => {
            Some(scenario.authorization.receiver_endpoint_node_id.clone())
        }
        E::PackageIdentityObservation
        | E::PayloadMassObservation
        | E::SealObservation
        | E::TemperatureObservation
        | E::ReleaseExecutionReceipt
        | E::PlacementObservation
        | E::CounterpartyCustodyReceipt
        | E::AmbiguousOutcomeObservation => Some(scenario.package.package_id.clone()),
        E::ContainerFitObservation => Some(scenario.container.container_id.clone()),
        E::TransferGateObservation => scenario
            .assets
            .iter()
            .find(|asset| asset.kind == crate::AssetKind::TransferMechanism)
            .map(|asset| asset.asset_id.clone()),
        E::ManifestValidated
        | E::AuthorizationValidated
        | E::ReceiverCapabilityValidated
        | E::PackageIdentityValidated
        | E::ContainerFitValidated
        | E::PayloadMassValidated
        | E::SealValidated
        | E::TemperatureValidated
        | E::TransferGateInterlockValidated
        | E::ReleaseIntentReady
        | E::SourceReleaseObserved
        | E::ReceiverArrivalObserved
        | E::PlacementVerified
        | E::CustodyEvidenceComplete
        | E::ReconciliationRecord => None,
    }
}

fn allowed_issuer_roles(kind: EvidenceKind) -> &'static [crate::ActorRole] {
    use crate::ActorRole as R;
    use EvidenceKind as E;
    match kind {
        E::ManifestSnapshot => &[R::ServiceProvider, R::HouseholdMember],
        E::AuthorizationReceipt => &[R::HouseholdMember],
        E::ReceiverCapabilityReceipt | E::ReceiverPresenceObservation => &[R::ReceiverDevice],
        E::PackageIdentityObservation | E::ReleaseExecutionReceipt => &[R::ServiceProvider],
        E::ContainerFitObservation
        | E::PayloadMassObservation
        | E::SealObservation
        | E::TemperatureObservation
        | E::TransferGateObservation
        | E::PlacementObservation => &[R::ReceiverDevice],
        E::CounterpartyCustodyReceipt => &[R::Custodian],
        E::AmbiguousOutcomeObservation => &[R::ReceiverDevice, R::ServiceProvider, R::Custodian],
        E::ManifestValidated
        | E::AuthorizationValidated
        | E::ReceiverCapabilityValidated
        | E::PackageIdentityValidated
        | E::ContainerFitValidated
        | E::PayloadMassValidated
        | E::SealValidated
        | E::TemperatureValidated
        | E::TransferGateInterlockValidated
        | E::ReleaseIntentReady
        | E::SourceReleaseObserved
        | E::ReceiverArrivalObserved
        | E::PlacementVerified
        | E::CustodyEvidenceComplete
        | E::ReconciliationRecord => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCENARIO: &str = include_str!("../fixtures/nominal-stationary-carrier.json");
    const PROCEDURE: &str = include_str!("../fixtures/nominal-receiving-procedure.json");
    const EVIDENCE: &str = include_str!("../fixtures/nominal-receiving-evidence.json");
    const RECONCILIATION_EVIDENCE: &str =
        include_str!("../fixtures/reconciliation-receiving-evidence.json");

    fn fixture() -> (ScenarioManifest, ProcedureGraph) {
        let scenario = serde_json::from_str(SCENARIO).expect("scenario fixture");
        let procedure = serde_json::from_str(PROCEDURE).expect("procedure fixture");
        (scenario, procedure)
    }

    fn evidence_fixture() -> ProcedureEvidenceBundle {
        serde_json::from_str(EVIDENCE).expect("procedure evidence fixture")
    }

    fn reconciliation_evidence_fixture() -> ProcedureEvidenceBundle {
        serde_json::from_str(RECONCILIATION_EVIDENCE).expect("reconciliation evidence fixture")
    }

    #[test]
    fn nominal_evidence_envelopes_are_admitted_without_claiming_authenticity_or_authority() {
        let (scenario, procedure) = fixture();
        let report = procedure
            .admit_evidence(&scenario, &evidence_fixture())
            .expect("well-formed receipt bundle");
        assert_eq!(
            report.execution_branch,
            ProcedureExecutionBranch::NormalCompletion
        );
        assert_eq!(report.admitted_receipt_count, 13);
        assert_eq!(report.admitted_kinds.len(), 13);
        assert!(
            !report
                .admitted_kinds
                .contains(&EvidenceKind::AmbiguousOutcomeObservation)
        );
        assert!(report.evidence_envelopes_structurally_admitted);
        assert!(report.declared_freshness_policy_passed);
        assert_eq!(report.required_verification_domains.len(), 5);
        assert!(!report.content_digest_verified);
        assert!(!report.cryptographic_authentication_verified);
        assert!(!report.issuer_authorization_verified);
        assert!(!report.trusted_time_verified);
        assert!(!report.sensor_truth_established);
        assert!(!report.motion_authority_granted);
        assert!(!report.physical_command_emitted);
        assert!(!report.task_success_attested);
    }

    #[test]
    fn reconciliation_branch_requires_ambiguity_but_not_successful_delivery_receipts() {
        let (scenario, procedure) = fixture();
        let report = procedure
            .admit_evidence(&scenario, &reconciliation_evidence_fixture())
            .expect("reconciliation branch receipt bundle");
        assert_eq!(
            report.execution_branch,
            ProcedureExecutionBranch::Reconciliation
        );
        assert_eq!(report.admitted_receipt_count, 10);
        assert!(
            report
                .admitted_kinds
                .contains(&EvidenceKind::AmbiguousOutcomeObservation)
        );
        assert!(
            !report
                .admitted_kinds
                .contains(&EvidenceKind::ReleaseExecutionReceipt)
        );
        assert!(
            !report
                .admitted_kinds
                .contains(&EvidenceKind::ReceiverPresenceObservation)
        );
        assert!(
            !report
                .admitted_kinds
                .contains(&EvidenceKind::PlacementObservation)
        );
        assert!(
            !report
                .admitted_kinds
                .contains(&EvidenceKind::CounterpartyCustodyReceipt)
        );
        assert!(!report.motion_authority_granted);
        assert!(!report.task_success_attested);
    }

    #[test]
    fn branch_mismatch_rejects_receipts_from_the_mutually_exclusive_path() {
        let (scenario, procedure) = fixture();
        let mut normal_with_ambiguity = evidence_fixture();
        normal_with_ambiguity.receipts.push(
            reconciliation_evidence_fixture()
                .receipts
                .into_iter()
                .find(|receipt| receipt.kind == EvidenceKind::AmbiguousOutcomeObservation)
                .expect("ambiguous outcome receipt"),
        );
        assert!(
            procedure
                .admit_evidence(&scenario, &normal_with_ambiguity)
                .is_err()
        );

        let mut reconciliation_with_success = reconciliation_evidence_fixture();
        reconciliation_with_success.receipts.push(
            evidence_fixture()
                .receipts
                .into_iter()
                .find(|receipt| receipt.kind == EvidenceKind::ReleaseExecutionReceipt)
                .expect("release execution receipt"),
        );
        assert!(
            procedure
                .admit_evidence(&scenario, &reconciliation_with_success)
                .is_err()
        );
    }

    #[test]
    fn evidence_admission_rejects_expired_receipt() {
        let (scenario, procedure) = fixture();
        let mut bundle = evidence_fixture();
        let receipt = bundle
            .receipts
            .iter_mut()
            .find(|receipt| receipt.kind == EvidenceKind::TemperatureObservation)
            .expect("temperature receipt");
        receipt.observed_at_unix_ms = scenario.started_at_unix_ms - 1_000;
        receipt.expires_at_unix_ms = scenario.started_at_unix_ms - 100;
        assert!(procedure.admit_evidence(&scenario, &bundle).is_err());
    }

    #[test]
    fn evidence_admission_requires_validity_across_entire_clock_uncertainty_window() {
        let (scenario, procedure) = fixture();

        let mut observation_too_recent = evidence_fixture();
        observation_too_recent.receipts[0].observed_at_unix_ms = scenario.started_at_unix_ms - 10;
        observation_too_recent.receipts[0].expires_at_unix_ms = scenario.started_at_unix_ms + 100;
        assert!(
            procedure
                .admit_evidence(&scenario, &observation_too_recent)
                .is_err()
        );

        let mut expiry_inside_uncertainty = evidence_fixture();
        expiry_inside_uncertainty.receipts[0].observed_at_unix_ms =
            scenario.started_at_unix_ms - 1_000;
        expiry_inside_uncertainty.receipts[0].expires_at_unix_ms = scenario.started_at_unix_ms + 10;
        assert!(
            procedure
                .admit_evidence(&scenario, &expiry_inside_uncertainty)
                .is_err()
        );
    }

    #[test]
    fn evidence_admission_rejects_observation_exactly_at_earliest_decision_time() {
        let (scenario, procedure) = fixture();
        let mut bundle = evidence_fixture();
        bundle.receipts[0].observed_at_unix_ms =
            scenario.started_at_unix_ms - u64::from(scenario.clock_uncertainty_ms);
        assert!(procedure.admit_evidence(&scenario, &bundle).is_err());
    }

    #[test]
    fn evidence_admission_rejects_expiry_exactly_at_latest_decision_time() {
        let (scenario, procedure) = fixture();
        let mut bundle = evidence_fixture();
        bundle.receipts[0].expires_at_unix_ms =
            scenario.started_at_unix_ms + u64::from(scenario.clock_uncertainty_ms);
        assert!(procedure.admit_evidence(&scenario, &bundle).is_err());
    }

    #[test]
    fn evidence_admission_rejects_old_but_not_yet_expired_sensor_observation() {
        let (scenario, procedure) = fixture();
        let mut bundle = evidence_fixture();
        let receipt = bundle
            .receipts
            .iter_mut()
            .find(|receipt| receipt.kind == EvidenceKind::TemperatureObservation)
            .expect("temperature receipt");
        receipt.observed_at_unix_ms = scenario.started_at_unix_ms - 3_000;
        receipt.expires_at_unix_ms = scenario.started_at_unix_ms + 60_000;
        assert!(receipt.expires_at_unix_ms > scenario.started_at_unix_ms);
        assert!(procedure.admit_evidence(&scenario, &bundle).is_err());
    }

    #[test]
    fn evidence_admission_rejects_wrong_issuer_role() {
        let (scenario, procedure) = fixture();
        let mut bundle = evidence_fixture();
        let receipt = bundle
            .receipts
            .iter_mut()
            .find(|receipt| receipt.kind == EvidenceKind::AuthorizationReceipt)
            .expect("authorization receipt");
        receipt.issuer_actor_id = "courier-01".into();
        assert!(procedure.admit_evidence(&scenario, &bundle).is_err());
    }

    #[test]
    fn evidence_admission_rejects_source_and_configuration_mismatch() {
        let (scenario, procedure) = fixture();
        let mut source_mismatch = evidence_fixture();
        source_mismatch.receipts[0].source_commit =
            "0000000000000000000000000000000000000000".into();
        assert!(
            procedure
                .admit_evidence(&scenario, &source_mismatch)
                .is_err()
        );

        let mut config_mismatch = evidence_fixture();
        config_mismatch.receipts[0].configuration_digest = format!("sha256:{}", "f".repeat(64));
        assert!(
            procedure
                .admit_evidence(&scenario, &config_mismatch)
                .is_err()
        );
    }

    #[test]
    fn evidence_admission_rejects_missing_receipt() {
        let (scenario, procedure) = fixture();
        let mut bundle = evidence_fixture();
        bundle.receipts.pop();
        assert!(procedure.admit_evidence(&scenario, &bundle).is_err());
    }

    #[test]
    fn evidence_admission_rejects_duplicate_identity_and_kind() {
        let (scenario, procedure) = fixture();
        let mut bundle = evidence_fixture();
        let mut duplicate = bundle.receipts[0].clone();
        duplicate.evidence_id = bundle.receipts[1].evidence_id.clone();
        bundle.receipts.push(duplicate);
        assert!(procedure.admit_evidence(&scenario, &bundle).is_err());
    }

    #[test]
    fn evidence_admission_rejects_subject_mismatch_and_malformed_digest() {
        let (scenario, procedure) = fixture();
        let mut wrong_subject = evidence_fixture();
        let receipt = wrong_subject
            .receipts
            .iter_mut()
            .find(|receipt| receipt.kind == EvidenceKind::ContainerFitObservation)
            .expect("container-fit receipt");
        receipt.subject_id = "package-01".into();
        assert!(procedure.admit_evidence(&scenario, &wrong_subject).is_err());

        let mut malformed_digest = evidence_fixture();
        malformed_digest.receipts[0].digest = "sha256:XYZ".into();
        assert!(
            procedure
                .admit_evidence(&scenario, &malformed_digest)
                .is_err()
        );
    }

    #[test]
    fn nominal_procedure_is_structurally_ready_but_has_no_authority() {
        let (scenario, procedure) = fixture();
        let report = procedure
            .evaluate(&scenario)
            .expect("valid procedure graph");
        assert_eq!(report.step_count, 15);
        assert_eq!(report.topological_order.len(), 15);
        assert!(report.structurally_valid_for_external_review);
        assert!(!report.motion_authority_granted);
        assert!(!report.physical_command_emitted);
        assert!(!report.task_success_attested);
    }

    #[test]
    fn wrong_scenario_or_configuration_binding_is_rejected() {
        let (scenario, mut procedure) = fixture();
        procedure.binding.configuration_digest = format!("sha256:{}", "f".repeat(64));
        assert!(procedure.evaluate(&scenario).is_err());
    }

    #[test]
    fn release_intent_cannot_skip_any_preflight_gate() {
        let (scenario, mut procedure) = fixture();
        let gate_step = procedure
            .steps
            .iter_mut()
            .find(|step| step.operation == ProcedureOperation::VerifyTransferGateInterlock)
            .expect("gate step");
        let gate_id = gate_step.step_id.clone();
        procedure
            .steps
            .iter_mut()
            .find(|step| step.operation == ProcedureOperation::PrepareReleaseIntent)
            .expect("release intent step")
            .depends_on
            .retain(|dependency| dependency != &gate_id);
        assert!(procedure.evaluate(&scenario).is_err());
    }

    #[test]
    fn dependency_cycle_is_rejected() {
        let (scenario, mut procedure) = fixture();
        let first = procedure.steps[0].step_id.clone();
        let last = procedure.steps[1].step_id.clone();
        procedure.steps[0].depends_on.push(last);
        procedure.steps[1].depends_on.push(first);
        assert!(procedure.evaluate(&scenario).is_err());
    }

    #[test]
    fn evidence_type_cannot_be_forged_or_relabelled() {
        let (scenario, mut procedure) = fixture();
        let manifest = procedure
            .steps
            .iter_mut()
            .find(|step| step.operation == ProcedureOperation::ValidateManifest)
            .expect("manifest step");
        manifest.consumes = vec![EvidenceKind::AuthorizationReceipt];
        assert!(procedure.evaluate(&scenario).is_err());
    }

    #[test]
    fn custody_cannot_be_acknowledged_before_placement_verification() {
        let (scenario, mut procedure) = fixture();
        let placement_id = procedure
            .steps
            .iter()
            .find(|step| step.operation == ProcedureOperation::VerifyPlacement)
            .expect("placement step")
            .step_id
            .clone();
        procedure
            .steps
            .iter_mut()
            .find(|step| step.operation == ProcedureOperation::CollectCustodyAcknowledgement)
            .expect("custody step")
            .depends_on
            .retain(|dependency| dependency != &placement_id);
        assert!(procedure.evaluate(&scenario).is_err());
    }
}
