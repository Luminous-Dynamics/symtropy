// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Deterministic compilation of validated RRVR campaign manifests.
//!
//! The compiler creates an immutable execution plan and cryptographic
//! commitments. It never executes an injector, issues native authority, or
//! claims that a scheduled fault was delivered.

use crate::fault_campaign::{
    BenchmarkArm, CampaignError, ControlClass, ExpectedDisposition, FaultCampaignManifest,
    FaultCutPoint, FaultInjection, FaultLayer,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const COMPILED_PLAN_VERSION: &str = "0.1";
pub const CANONICALIZATION_ID: &str = "serde-json-value-sorted-keys-v0.1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlanCompileError {
    InvalidSourceManifestDigest,
    ManifestRejected(CampaignError),
    SerializationFailure,
    CommitmentMismatch,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CompiledPlatformBinding {
    pub platform_id: String,
    pub profile_sha256: String,
    pub environment_artifact_sha256: String,
    pub simulator_revision: String,
    pub ros_distribution: String,
    pub middleware_revision: String,
    pub controller_revision: String,
    pub driver_revision: String,
    pub firmware_revision: String,
    pub calibration_revision: String,
    pub observer_stack_revision: String,
    pub fault_injector_revision: String,
    pub fault_injector_sha256: String,
    pub fault_injector_fault_domain_id: String,
    pub native_adapter_revision: String,
    pub runtime_monitor_revision: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CompiledResetBinding {
    pub kind: String,
    pub revision: String,
    pub profile_sha256: String,
    pub expected_initial_state_sha256: String,
    pub post_reset_observation_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CompiledPair {
    pub pair_id: String,
    pub pair_commitment_sha256: String,
    pub scenario_ids: Vec<String>,
    pub arms: Vec<BenchmarkArm>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CompiledScenario {
    pub scenario_id: String,
    pub pair_id: String,
    pub arm: BenchmarkArm,
    pub control_class: ControlClass,
    pub task_id: String,
    pub policy_id: String,
    pub policy_sha256: String,
    pub scene_sha256: String,
    pub action_prefix_sha256: String,
    pub seed: u64,
    pub logical_effect_id: String,
    pub pair_commitment_sha256: String,
    pub schedule_sha256: String,
    pub fault_events: Vec<FaultInjection>,
    pub expected_disposition: ExpectedDisposition,
    pub expected_reason_code: String,
    pub invariant_id: String,
    pub failure_signature_sha256: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CompiledCampaignPlan {
    pub version: String,
    pub canonicalization: String,
    pub campaign_id: String,
    pub source_manifest_sha256: String,
    pub fault_model_revision: String,
    pub fault_model_sha256: String,
    pub platform: CompiledPlatformBinding,
    pub reset: CompiledResetBinding,
    pub required_layers: Vec<FaultLayer>,
    pub required_cut_points: Vec<FaultCutPoint>,
    pub pairs: Vec<CompiledPair>,
    pub scenarios: Vec<CompiledScenario>,
    pub plan_sha256: String,
}

#[derive(Serialize)]
struct PairCommitment<'a> {
    campaign_id: &'a str,
    pair_id: &'a str,
    control_class: ControlClass,
    fault_model_revision: &'a str,
    fault_model_sha256: &'a str,
    platform: &'a CompiledPlatformBinding,
    reset: &'a CompiledResetBinding,
    task_id: &'a str,
    policy_id: &'a str,
    policy_sha256: &'a str,
    scene_sha256: &'a str,
    action_prefix_sha256: &'a str,
    seed: u64,
    logical_effect_id: &'a str,
    faults: &'a [FaultInjection],
}

#[derive(Serialize)]
struct CompiledCampaignPlanBody<'a> {
    version: &'a str,
    canonicalization: &'a str,
    campaign_id: &'a str,
    source_manifest_sha256: &'a str,
    fault_model_revision: &'a str,
    fault_model_sha256: &'a str,
    platform: &'a CompiledPlatformBinding,
    reset: &'a CompiledResetBinding,
    required_layers: &'a [FaultLayer],
    required_cut_points: &'a [FaultCutPoint],
    pairs: &'a [CompiledPair],
    scenarios: &'a [CompiledScenario],
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(64);
    for byte in digest {
        output.push(DIGITS[(byte >> 4) as usize] as char);
        output.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    output
}

/// Canonical JSON is UTF-8 JSON serialized from serde_json::Value, whose
/// object map is key-sorted in this crate's pinned serde_json configuration.
/// No timestamps, machine paths or runtime-generated IDs are included.
fn canonical_json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, PlanCompileError> {
    let value = serde_json::to_value(value).map_err(|_| PlanCompileError::SerializationFailure)?;
    serde_json::to_vec(&value).map_err(|_| PlanCompileError::SerializationFailure)
}

fn digest_serializable<T: Serialize>(value: &T) -> Result<String, PlanCompileError> {
    Ok(sha256_hex(&canonical_json_bytes(value)?))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

impl CompiledCampaignPlan {
    fn body(&self) -> CompiledCampaignPlanBody<'_> {
        CompiledCampaignPlanBody {
            version: &self.version,
            canonicalization: &self.canonicalization,
            campaign_id: &self.campaign_id,
            source_manifest_sha256: &self.source_manifest_sha256,
            fault_model_revision: &self.fault_model_revision,
            fault_model_sha256: &self.fault_model_sha256,
            platform: &self.platform,
            reset: &self.reset,
            required_layers: &self.required_layers,
            required_cut_points: &self.required_cut_points,
            pairs: &self.pairs,
            scenarios: &self.scenarios,
        }
    }

    /// Verify all nested schedule/pair commitments and the top-level plan hash.
    ///
    /// This proves byte-level integrity relative to the supplied plan, not
    /// authenticity of its source manifest or proof of physical execution.
    pub fn verify_integrity(&self) -> Result<(), PlanCompileError> {
        if self.version != COMPILED_PLAN_VERSION
            || self.canonicalization != CANONICALIZATION_ID
            || !valid_sha256(&self.source_manifest_sha256)
            || !valid_sha256(&self.fault_model_sha256)
            || !valid_sha256(&self.plan_sha256)
        {
            return Err(PlanCompileError::CommitmentMismatch);
        }

        let mut scenario_ids = BTreeSet::new();
        let mut grouped: BTreeMap<&str, Vec<&CompiledScenario>> = BTreeMap::new();
        let mut covered_layers = BTreeSet::new();
        let mut covered_cuts = BTreeSet::new();

        for scenario in &self.scenarios {
            if !scenario_ids.insert(scenario.scenario_id.as_str())
                || digest_serializable(&scenario.fault_events)? != scenario.schedule_sha256
            {
                return Err(PlanCompileError::CommitmentMismatch);
            }
            for (index, fault) in scenario.fault_events.iter().enumerate() {
                if fault.sequence != index as u32 + 1 {
                    return Err(PlanCompileError::CommitmentMismatch);
                }
                covered_layers.insert(fault.target_layer);
                covered_cuts.insert(fault.cut_point);
            }
            grouped
                .entry(scenario.pair_id.as_str())
                .or_default()
                .push(scenario);
        }

        let pair_ids: BTreeSet<&str> = self
            .pairs
            .iter()
            .map(|pair| pair.pair_id.as_str())
            .collect();
        if pair_ids.len() != self.pairs.len() || pair_ids.len() != grouped.len() {
            return Err(PlanCompileError::CommitmentMismatch);
        }

        for pair in &self.pairs {
            let members = grouped
                .get(pair.pair_id.as_str())
                .ok_or(PlanCompileError::CommitmentMismatch)?;
            let reference = members
                .first()
                .ok_or(PlanCompileError::CommitmentMismatch)?;
            let mut expected_ids: Vec<String> = members
                .iter()
                .map(|scenario| scenario.scenario_id.clone())
                .collect();
            expected_ids.sort();
            let expected_arms: BTreeSet<BenchmarkArm> =
                members.iter().map(|scenario| scenario.arm).collect();
            if pair.scenario_ids != expected_ids
                || pair.arms != expected_arms.iter().copied().collect::<Vec<_>>()
            {
                return Err(PlanCompileError::CommitmentMismatch);
            }

            for scenario in members.iter().skip(1) {
                if scenario.control_class != reference.control_class
                    || scenario.task_id != reference.task_id
                    || scenario.policy_id != reference.policy_id
                    || scenario.policy_sha256 != reference.policy_sha256
                    || scenario.scene_sha256 != reference.scene_sha256
                    || scenario.action_prefix_sha256 != reference.action_prefix_sha256
                    || scenario.seed != reference.seed
                    || scenario.logical_effect_id != reference.logical_effect_id
                    || scenario.fault_events != reference.fault_events
                {
                    return Err(PlanCompileError::CommitmentMismatch);
                }
            }

            let commitment = PairCommitment {
                campaign_id: &self.campaign_id,
                pair_id: &pair.pair_id,
                control_class: reference.control_class,
                fault_model_revision: &self.fault_model_revision,
                fault_model_sha256: &self.fault_model_sha256,
                platform: &self.platform,
                reset: &self.reset,
                task_id: &reference.task_id,
                policy_id: &reference.policy_id,
                policy_sha256: &reference.policy_sha256,
                scene_sha256: &reference.scene_sha256,
                action_prefix_sha256: &reference.action_prefix_sha256,
                seed: reference.seed,
                logical_effect_id: &reference.logical_effect_id,
                faults: &reference.fault_events,
            };
            let expected_pair_digest = digest_serializable(&commitment)?;
            if pair.pair_commitment_sha256 != expected_pair_digest
                || members
                    .iter()
                    .any(|scenario| scenario.pair_commitment_sha256 != expected_pair_digest)
            {
                return Err(PlanCompileError::CommitmentMismatch);
            }
        }

        let required_layers: BTreeSet<FaultLayer> = self.required_layers.iter().copied().collect();
        let required_cuts: BTreeSet<FaultCutPoint> =
            self.required_cut_points.iter().copied().collect();
        if !required_layers.is_subset(&covered_layers) || !required_cuts.is_subset(&covered_cuts) {
            return Err(PlanCompileError::CommitmentMismatch);
        }

        if digest_serializable(&self.body())? != self.plan_sha256 {
            return Err(PlanCompileError::CommitmentMismatch);
        }
        Ok(())
    }

    /// Compile a validated manifest into a deterministic, non-authoritative plan.
    pub fn compile(
        manifest: &FaultCampaignManifest,
        source_manifest_sha256: &str,
    ) -> Result<Self, PlanCompileError> {
        manifest
            .validate()
            .map_err(PlanCompileError::ManifestRejected)?;
        if !valid_sha256(source_manifest_sha256) {
            return Err(PlanCompileError::InvalidSourceManifestDigest);
        }

        let platform = CompiledPlatformBinding {
            platform_id: manifest.platform.platform_id.clone(),
            profile_sha256: manifest.platform.profile_sha256.clone(),
            environment_artifact_sha256: manifest.platform.environment_artifact_sha256.clone(),
            simulator_revision: manifest.platform.simulator_revision.clone(),
            ros_distribution: manifest.platform.ros_distribution.clone(),
            middleware_revision: manifest.platform.middleware_revision.clone(),
            controller_revision: manifest.platform.controller_revision.clone(),
            driver_revision: manifest.platform.driver_revision.clone(),
            firmware_revision: manifest.platform.firmware_revision.clone(),
            calibration_revision: manifest.platform.calibration_revision.clone(),
            observer_stack_revision: manifest.platform.observer_stack_revision.clone(),
            fault_injector_revision: manifest.platform.fault_injector_revision.clone(),
            fault_injector_sha256: manifest.platform.fault_injector_sha256.clone(),
            fault_injector_fault_domain_id: manifest.platform.fault_injector_fault_domain_id.clone(),
            native_adapter_revision: manifest.platform.native_adapter_revision.clone(),
            runtime_monitor_revision: manifest.platform.runtime_monitor_revision.clone(),
        };
        let reset = CompiledResetBinding {
            kind: manifest.reset.kind.as_str().to_owned(),
            revision: manifest.reset.revision.clone(),
            profile_sha256: manifest.reset.profile_sha256.clone(),
            expected_initial_state_sha256: manifest.reset.expected_initial_state_sha256.clone(),
            post_reset_observation_id: manifest.reset.post_reset_observation_id.clone(),
        };

        let mut pair_scenarios: BTreeMap<&str, Vec<&crate::fault_campaign::CampaignScenario>> =
            BTreeMap::new();
        for scenario in &manifest.scenarios {
            pair_scenarios
                .entry(scenario.pair_id.as_str())
                .or_default()
                .push(scenario);
        }

        let mut pairs = Vec::with_capacity(pair_scenarios.len());
        let mut pair_commitments = BTreeMap::<String, String>::new();
        for (pair_id, members) in &pair_scenarios {
            let reference = members
                .first()
                .ok_or(PlanCompileError::SerializationFailure)?;
            let commitment = PairCommitment {
                campaign_id: &manifest.campaign_id,
                pair_id,
                control_class: reference.control_class,
                fault_model_revision: &manifest.fault_model_revision,
                fault_model_sha256: &manifest.fault_model_sha256,
                platform: &platform,
                reset: &reset,
                task_id: &reference.task_id,
                policy_id: &reference.policy_id,
                policy_sha256: &reference.policy_sha256,
                scene_sha256: &reference.scene_sha256,
                action_prefix_sha256: &reference.action_prefix_sha256,
                seed: reference.seed,
                logical_effect_id: &reference.logical_effect_id,
                faults: &reference.faults,
            };
            let pair_digest = digest_serializable(&commitment)?;
            let mut scenario_ids: Vec<String> = members
                .iter()
                .map(|scenario| scenario.scenario_id.clone())
                .collect();
            scenario_ids.sort();
            let arms: BTreeSet<BenchmarkArm> =
                members.iter().map(|scenario| scenario.arm).collect();
            let compiled_pair = CompiledPair {
                pair_id: (*pair_id).to_owned(),
                pair_commitment_sha256: pair_digest.clone(),
                scenario_ids,
                arms: arms.iter().copied().collect(),
            };
            pair_commitments.insert((*pair_id).to_owned(), pair_digest);
            pairs.push(compiled_pair);
        }

        pairs.sort_by(|left, right| left.pair_id.cmp(&right.pair_id));

        let mut scenarios = Vec::with_capacity(manifest.scenarios.len());
        for scenario in &manifest.scenarios {
            let pair_commitment_sha256 = pair_commitments
                .get(&scenario.pair_id)
                .cloned()
                .ok_or(PlanCompileError::SerializationFailure)?;
            let schedule_sha256 = digest_serializable(&scenario.faults)?;
            scenarios.push(CompiledScenario {
                scenario_id: scenario.scenario_id.clone(),
                pair_id: scenario.pair_id.clone(),
                arm: scenario.arm,
                control_class: scenario.control_class,
                task_id: scenario.task_id.clone(),
                policy_id: scenario.policy_id.clone(),
                policy_sha256: scenario.policy_sha256.clone(),
                scene_sha256: scenario.scene_sha256.clone(),
                action_prefix_sha256: scenario.action_prefix_sha256.clone(),
                seed: scenario.seed,
                logical_effect_id: scenario.logical_effect_id.clone(),
                pair_commitment_sha256,
                schedule_sha256,
                fault_events: scenario.faults.clone(),
                expected_disposition: scenario.expected_disposition,
                expected_reason_code: scenario.expected_reason_code.clone(),
                invariant_id: scenario.invariant_id.clone(),
                failure_signature_sha256: scenario.failure_signature_sha256.clone(),
            });
        }
        scenarios.sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));

        let mut plan = Self {
            version: COMPILED_PLAN_VERSION.to_owned(),
            canonicalization: CANONICALIZATION_ID.to_owned(),
            campaign_id: manifest.campaign_id.clone(),
            source_manifest_sha256: source_manifest_sha256.to_owned(),
            fault_model_revision: manifest.fault_model_revision.clone(),
            fault_model_sha256: manifest.fault_model_sha256.clone(),
            platform,
            reset,
            required_layers: manifest.required_layers.clone(),
            required_cut_points: manifest.required_cut_points.clone(),
            pairs,
            scenarios,
            plan_sha256: String::new(),
        };
        plan.plan_sha256 = digest_serializable(&plan.body())?;
        plan.verify_integrity()?;
        Ok(plan)
    }

    /// Verify the exact source manifest bytes that were compiled into this plan.
    pub fn verify_source_manifest_bytes(
        &self,
        source_manifest_bytes: &[u8],
    ) -> Result<(), PlanCompileError> {
        if sha256_hex(source_manifest_bytes) == self.source_manifest_sha256 {
            Ok(())
        } else {
            Err(PlanCompileError::CommitmentMismatch)
        }
    }

    /// Return deterministic compact JSON bytes for an internally verified plan.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, PlanCompileError> {
        self.verify_integrity()?;
        canonical_json_bytes(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fault_campaign::FaultCampaignManifest;

    fn manifest() -> FaultCampaignManifest {
        serde_json::from_str(include_str!("../fixtures/fault_campaign_paired_v0_1.json"))
            .expect("known-answer campaign fixture deserializes")
    }

    #[test]
    fn compilation_is_byte_deterministic_for_identical_manifest_and_source_digest() {
        let manifest = manifest();
        let source = "a".repeat(64);
        let first = CompiledCampaignPlan::compile(&manifest, &source).unwrap();
        let second = CompiledCampaignPlan::compile(&manifest, &source).unwrap();
        assert_eq!(first, second);
        assert_eq!(
            first.canonical_bytes().unwrap(),
            second.canonical_bytes().unwrap()
        );
        assert!(valid_sha256(&first.plan_sha256));
        assert_eq!(first.verify_integrity(), Ok(()));
    }

    #[test]
    fn pair_commitment_is_shared_across_arms_and_scenario_identity_is_retained() {
        let plan = CompiledCampaignPlan::compile(&manifest(), &"b".repeat(64)).unwrap();
        let positive: Vec<_> = plan
            .scenarios
            .iter()
            .filter(|scenario| scenario.pair_id == "pair-positive-001")
            .collect();
        assert_eq!(positive.len(), 2);
        assert_eq!(
            positive[0].pair_commitment_sha256,
            positive[1].pair_commitment_sha256
        );
        assert_ne!(positive[0].scenario_id, positive[1].scenario_id);
        assert_ne!(positive[0].arm, positive[1].arm);
    }

    #[test]
    fn verifier_rejects_tampered_schedule_or_platform_binding() {
        let valid = CompiledCampaignPlan::compile(&manifest(), &"a".repeat(64)).unwrap();

        let mut changed_schedule = valid.clone();
        changed_schedule.scenarios[0].fault_events[0].kind =
            crate::fault_campaign::FaultKind::PowerLoss;
        assert_eq!(
            changed_schedule.verify_integrity(),
            Err(PlanCompileError::CommitmentMismatch)
        );

        let mut changed_platform = valid;
        changed_platform.platform.controller_revision = "untracked-controller".into();
        assert_eq!(
            changed_platform.verify_integrity(),
            Err(PlanCompileError::CommitmentMismatch)
        );
    }

    #[test]
    fn source_manifest_bytes_must_match_the_compiled_source_commitment() {
        let manifest_bytes = include_bytes!("../fixtures/fault_campaign_paired_v0_1.json");
        let manifest: FaultCampaignManifest = serde_json::from_slice(manifest_bytes).unwrap();
        let source_digest = sha256_hex(manifest_bytes);
        let plan = CompiledCampaignPlan::compile(&manifest, &source_digest).unwrap();
        assert_eq!(plan.verify_source_manifest_bytes(manifest_bytes), Ok(()));
        assert_eq!(
            plan.verify_source_manifest_bytes(b"changed manifest bytes"),
            Err(PlanCompileError::CommitmentMismatch)
        );
    }

    #[test]
    fn changing_source_digest_changes_plan_identity() {
        let manifest = manifest();
        let first = CompiledCampaignPlan::compile(&manifest, &"a".repeat(64)).unwrap();
        let second = CompiledCampaignPlan::compile(&manifest, &"b".repeat(64)).unwrap();
        assert_ne!(first.plan_sha256, second.plan_sha256);
    }

    #[test]
    fn changing_shared_policy_or_fault_parameters_changes_pair_commitment() {
        let original = manifest();
        let original_plan = CompiledCampaignPlan::compile(&original, &"a".repeat(64)).unwrap();

        let mut changed_policy = manifest();
        for scenario in &mut changed_policy.scenarios {
            if scenario.pair_id == "pair-negative-001" {
                scenario.policy_sha256 = "c".repeat(64);
            }
        }
        let policy_plan = CompiledCampaignPlan::compile(&changed_policy, &"a".repeat(64)).unwrap();
        let original_policy_pair = original_plan
            .pairs
            .iter()
            .find(|pair| pair.pair_id == "pair-negative-001")
            .unwrap();
        let changed_policy_pair = policy_plan
            .pairs
            .iter()
            .find(|pair| pair.pair_id == "pair-negative-001")
            .unwrap();
        assert_ne!(
            original_policy_pair.pair_commitment_sha256,
            changed_policy_pair.pair_commitment_sha256
        );

        let mut changed_fault = manifest();
        for scenario in &mut changed_fault.scenarios {
            if scenario.pair_id == "pair-negative-001" {
                scenario.faults[0].parameters_sha256 = "d".repeat(64);
            }
        }
        let fault_plan = CompiledCampaignPlan::compile(&changed_fault, &"a".repeat(64)).unwrap();
        let original_negative = original_plan
            .pairs
            .iter()
            .find(|pair| pair.pair_id == "pair-negative-001")
            .unwrap();
        let changed_negative = fault_plan
            .pairs
            .iter()
            .find(|pair| pair.pair_id == "pair-negative-001")
            .unwrap();
        assert_ne!(
            original_negative.pair_commitment_sha256,
            changed_negative.pair_commitment_sha256
        );
    }

    #[test]
    fn reordering_fault_events_changes_schedule_and_pair_commitments() {
        let original = CompiledCampaignPlan::compile(&manifest(), &"a".repeat(64)).unwrap();
        let mut reordered = manifest();
        for scenario in &mut reordered.scenarios {
            if scenario.pair_id == "pair-negative-001" {
                scenario.faults.swap(0, 1);
                for (index, fault) in scenario.faults.iter_mut().enumerate() {
                    fault.sequence = index as u32 + 1;
                }
            }
        }
        let changed = CompiledCampaignPlan::compile(&reordered, &"a".repeat(64)).unwrap();
        let original_pair = original
            .pairs
            .iter()
            .find(|pair| pair.pair_id == "pair-negative-001")
            .unwrap();
        let changed_pair = changed
            .pairs
            .iter()
            .find(|pair| pair.pair_id == "pair-negative-001")
            .unwrap();
        assert_ne!(
            original_pair.pair_commitment_sha256,
            changed_pair.pair_commitment_sha256
        );
        let original_scenario = original
            .scenarios
            .iter()
            .find(|scenario| scenario.scenario_id == "negative-full-rrvr")
            .unwrap();
        let changed_scenario = changed
            .scenarios
            .iter()
            .find(|scenario| scenario.scenario_id == "negative-full-rrvr")
            .unwrap();
        assert_ne!(
            original_scenario.schedule_sha256,
            changed_scenario.schedule_sha256
        );
    }

    #[test]
    fn plan_binding_retains_every_platform_component_revision() {
        let manifest = manifest();
        let plan = CompiledCampaignPlan::compile(&manifest, &"a".repeat(64)).unwrap();
        assert_eq!(
            plan.platform.simulator_revision,
            manifest.platform.simulator_revision
        );
        assert_eq!(
            plan.platform.ros_distribution,
            manifest.platform.ros_distribution
        );
        assert_eq!(
            plan.platform.middleware_revision,
            manifest.platform.middleware_revision
        );
        assert_eq!(
            plan.platform.controller_revision,
            manifest.platform.controller_revision
        );
        assert_eq!(
            plan.platform.driver_revision,
            manifest.platform.driver_revision
        );
        assert_eq!(
            plan.platform.firmware_revision,
            manifest.platform.firmware_revision
        );
        assert_eq!(
            plan.platform.calibration_revision,
            manifest.platform.calibration_revision
        );
        assert_eq!(
            plan.platform.observer_stack_revision,
            manifest.platform.observer_stack_revision
        );
        assert_eq!(
            plan.platform.fault_injector_sha256,
            manifest.platform.fault_injector_sha256
        );
    }

    #[test]
    fn rejects_non_sha256_source_manifest_commitment() {
        assert_eq!(
            CompiledCampaignPlan::compile(&manifest(), "latest"),
            Err(PlanCompileError::InvalidSourceManifestDigest)
        );
    }

    #[test]
    fn does_not_compile_an_invalid_or_unpaired_manifest() {
        let mut invalid = manifest();
        invalid.scenarios[1].seed += 1;
        assert!(matches!(
            CompiledCampaignPlan::compile(&invalid, &"a".repeat(64)),
            Err(PlanCompileError::ManifestRejected(
                CampaignError::IncomparablePair
            ))
        ));
    }
}
