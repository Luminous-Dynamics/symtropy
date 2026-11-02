//! Split-firewalled lifecycle for adaptive scientific benchmark generation.
//!
//! Development observations may refine recipes before a policy freeze.
//! Calibration evaluates the frozen generator but cannot alter that policy.
//! SealedHoldout tasks are generated after freeze using a disjoint hidden seed
//! namespace. An independent oracle verifier and semantic-overlap auditor are
//! required. Structural IDs/digests are shape-checked here; authentication is
//! delegated to trusted provenance providers outside this module.

use super::counterexample_ledger::EvidenceSplit;
use super::scientific_challenge::ChallengeSplitRegistry;
use std::collections::BTreeSet;

const MAX_ID_BYTES: usize = 256;
const MAX_TASKS_PER_RELEASE: usize = 10_000;
const MAX_EXPOSURE_RECORDS: usize = 100_000;
const MAX_SEMANTIC_COMPARISONS: usize = 1_000_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdaptiveBenchmarkPlan {
    pub benchmark_id: String,
    pub benchmark_version: String,
    pub plan_digest: String,
    pub generator_id: String,
    pub generator_version: String,
    pub generator_artifact_digest: String,
    pub refinement_policy_digest: String,
    pub concept_registry_digest: String,
    pub recipe_schema_digest: String,
    pub oracle_verifier_id: String,
    pub oracle_verifier_version: String,
    pub oracle_verifier_artifact_digest: String,
    pub semantic_auditor_id: String,
    pub semantic_auditor_version: String,
    pub semantic_auditor_artifact_digest: String,
    pub allowed_domains: Vec<String>,
    pub allowed_modalities: Vec<String>,
    pub allowed_reasoning_profiles: Vec<String>,
    pub minimum_calibration_tasks: usize,
    pub maximum_calibration_ambiguous_tasks: usize,
    pub maximum_holdout_tasks: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SplitBoundBenchmarkArtifact {
    pub artifact_digest: String,
    pub split: EvidenceSplit,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GenerationInputRole {
    FrozenPlan,
    DevelopmentEvidence,
    CalibrationQualification,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenerationInputArtifact {
    pub artifact_digest: String,
    pub split: Option<EvidenceSplit>,
    pub role: GenerationInputRole,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DevelopmentAdaptationEvent {
    pub event_id: String,
    pub sequence: u64,
    pub split: EvidenceSplit,
    pub family_group_id: String,
    pub seed_namespace_id: String,
    pub concept_digest: String,
    pub parent_recipe_template_digest: Option<String>,
    pub candidate_recipe_template_digest: String,
    pub solver_panel_digest: String,
    pub solver_trajectory_digest: String,
    pub judge_feedback_digest: String,
    pub source_artifacts: Vec<SplitBoundBenchmarkArtifact>,
    pub adapted_at_unix_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdaptiveBenchmarkFreezeReceipt {
    pub freeze_receipt_id: String,
    pub freeze_receipt_digest: String,
    pub benchmark_id: String,
    pub benchmark_version: String,
    pub plan_digest: String,
    pub adaptation_transcript_digest: String,
    pub generator_artifact_digest: String,
    pub refinement_policy_digest: String,
    pub concept_registry_digest: String,
    pub recipe_schema_digest: String,
    pub oracle_verifier_artifact_digest: String,
    pub semantic_auditor_artifact_digest: String,
    pub frozen_concept_digests: Vec<String>,
    pub frozen_recipe_template_digests: Vec<String>,
    pub adaptation_event_count: usize,
    pub frozen_at_unix_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CalibrationQualificationReceipt {
    pub receipt_id: String,
    pub receipt_digest: String,
    pub benchmark_id: String,
    pub split: EvidenceSplit,
    pub family_group_id: String,
    pub freeze_receipt_digest: String,
    pub generator_artifact_digest: String,
    pub oracle_verifier_artifact_digest: String,
    pub seed_namespace_id: String,
    pub seed_commitment_digest: String,
    pub task_count: usize,
    pub independently_verified_task_count: usize,
    pub solvable_task_count: usize,
    pub ambiguous_task_count: usize,
    pub generated_at_unix_seconds: u64,
    pub completed_at_unix_seconds: u64,
    pub quality_gate_passed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndependentOracleValidationReceipt {
    pub verifier_id: String,
    pub verifier_version: String,
    pub verifier_artifact_digest: String,
    pub task_public_digest: String,
    pub sealed_oracle_commitment_digest: String,
    pub validation_receipt_digest: String,
    pub solvable: bool,
    pub unique_ground_truth: bool,
    pub negative_control_passed: bool,
    pub ambiguity_detected: bool,
    pub verified_at_unix_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdaptiveBenchmarkTaskArtifact {
    pub task_id: String,
    pub concept_digest: String,
    pub recipe_template_digest: String,
    pub recipe_instance_digest: String,
    pub family_group_id: String,
    pub split: EvidenceSplit,
    pub domain_id: String,
    pub modality_id: String,
    pub reasoning_profile_id: String,
    pub generator_id: String,
    pub generator_version: String,
    pub generator_artifact_digest: String,
    pub seed_namespace_id: String,
    pub seed_commitment_digest: String,
    pub public_task_digest: String,
    pub public_input_artifact_digest: String,
    /// Digest commitment only. Oracle bytes and expected outcomes remain sealed.
    pub sealed_oracle_commitment_digest: String,
    pub oracle_validation: IndependentOracleValidationReceipt,
    pub generator_input_artifact_digests: Vec<String>,
    pub generated_at_unix_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PriorBenchmarkExposure {
    pub exposure_id: String,
    pub benchmark_release_id: String,
    pub task_public_digest: String,
    pub exposed_agent_id: String,
    pub exposed_agent_version: String,
    pub prompt_or_scaffold_digest: String,
    pub exposed_at_unix_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdaptiveBenchmarkSealedRelease {
    pub release_id: String,
    pub release_digest: String,
    pub benchmark_id: String,
    pub benchmark_version: String,
    pub split: EvidenceSplit,
    pub family_group_id: String,
    pub freeze_receipt_digest: String,
    pub calibration_receipt_digest: String,
    pub generator_artifact_digest: String,
    pub semantic_auditor_id: String,
    pub semantic_auditor_artifact_digest: String,
    pub hidden_seed_namespace_id: String,
    pub hidden_seed_commitment_digest: String,
    pub generation_inputs: Vec<GenerationInputArtifact>,
    pub task_artifacts: Vec<AdaptiveBenchmarkTaskArtifact>,
    pub prior_exposures: Vec<PriorBenchmarkExposure>,
    pub released_at_unix_seconds: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticOverlapDecision {
    NoMaterialOverlap,
    PotentialNearDuplicate,
}

pub trait BenchmarkSemanticOverlapAuditor {
    fn identity(&self) -> &str;
    fn artifact_digest(&self) -> &str;

    fn audit_overlap(
        &self,
        candidate_task_digest: &str,
        previously_exposed_task_digest: &str,
    ) -> Result<SemanticOverlapDecision, String>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdaptiveBenchmarkLifecycleReceipt {
    pub benchmark_id: String,
    pub benchmark_version: String,
    pub freeze_receipt_digest: String,
    pub calibration_receipt_digest: String,
    pub release_id: String,
    pub release_digest: String,
    pub development_adaptation_count: usize,
    pub frozen_concept_count: usize,
    pub frozen_recipe_template_count: usize,
    pub calibration_task_count: usize,
    pub sealed_task_count: usize,
    pub prior_exposure_count: usize,
    pub semantic_overlap_comparisons: usize,
    pub generator_verifier_independent: bool,
    pub generator_semantic_auditor_independent: bool,
    pub sealed_answers_absent_from_generation_inputs: bool,
    pub calibration_did_not_mutate_frozen_policy: bool,
    pub holdout_seed_namespace_disjoint: bool,
    pub exact_task_digests_unique: bool,
    pub semantic_overlap_audit_passed: bool,
    pub split_firewall_passed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdaptiveBenchmarkError {
    InvalidPlan(String),
    InvalidIdentity(String),
    InvalidDigest(String),
    SplitFirewall(String),
    AdaptationHistory(String),
    FreezeMismatch(String),
    CalibrationInvalid(String),
    SealedReleaseInvalid(String),
    TaskInvalid(String),
    GeneratorVerifierCollision(String),
    SeedNamespaceReuse(String),
    AnswerLeakage(String),
    DuplicateTask(String),
    SemanticOverlap(String),
    ArithmeticOverflow(String),
    ProviderFailure(String),
}

impl std::fmt::Display for AdaptiveBenchmarkError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPlan(reason)
            | Self::InvalidIdentity(reason)
            | Self::InvalidDigest(reason)
            | Self::SplitFirewall(reason)
            | Self::AdaptationHistory(reason)
            | Self::FreezeMismatch(reason)
            | Self::CalibrationInvalid(reason)
            | Self::SealedReleaseInvalid(reason)
            | Self::TaskInvalid(reason)
            | Self::GeneratorVerifierCollision(reason)
            | Self::SeedNamespaceReuse(reason)
            | Self::AnswerLeakage(reason)
            | Self::DuplicateTask(reason)
            | Self::SemanticOverlap(reason)
            | Self::ArithmeticOverflow(reason)
            | Self::ProviderFailure(reason) => formatter.write_str(reason),
        }
    }
}

impl std::error::Error for AdaptiveBenchmarkError {}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= MAX_ID_BYTES
}

fn require_id(value: &str, label: &str) -> Result<(), AdaptiveBenchmarkError> {
    if !valid_id(value) {
        return Err(AdaptiveBenchmarkError::InvalidIdentity(format!(
            "{label} is empty or overlong"
        )));
    }
    Ok(())
}

fn valid_digest(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64
        && hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn require_digest(value: &str, label: &str) -> Result<(), AdaptiveBenchmarkError> {
    if !valid_digest(value) {
        return Err(AdaptiveBenchmarkError::InvalidDigest(format!(
            "{label} is not a canonical sha256 digest"
        )));
    }
    Ok(())
}

fn require_unique_ids(values: &[String], label: &str) -> Result<(), AdaptiveBenchmarkError> {
    let mut seen = BTreeSet::new();
    for value in values {
        require_id(value, label)?;
        if !seen.insert(value.clone()) {
            return Err(AdaptiveBenchmarkError::InvalidIdentity(format!(
                "{label} values must be unique"
            )));
        }
    }
    Ok(())
}

fn require_unique_digests(
    values: &[String],
    label: &str,
) -> Result<(), AdaptiveBenchmarkError> {
    let mut seen = BTreeSet::new();
    for value in values {
        require_digest(value, label)?;
        if !seen.insert(value.clone()) {
            return Err(AdaptiveBenchmarkError::InvalidDigest(format!(
                "{label} values must be unique"
            )));
        }
    }
    Ok(())
}

fn validate_plan(plan: &AdaptiveBenchmarkPlan) -> Result<(), AdaptiveBenchmarkError> {
    for (value, label) in [
        (&plan.benchmark_id, "benchmark ID"),
        (&plan.benchmark_version, "benchmark version"),
        (&plan.generator_id, "generator ID"),
        (&plan.generator_version, "generator version"),
        (&plan.oracle_verifier_id, "oracle verifier ID"),
        (&plan.oracle_verifier_version, "oracle verifier version"),
        (&plan.semantic_auditor_id, "semantic auditor ID"),
        (&plan.semantic_auditor_version, "semantic auditor version"),
    ] {
        require_id(value, label)?;
    }
    for (value, label) in [
        (&plan.generator_artifact_digest, "generator artifact"),
        (&plan.refinement_policy_digest, "refinement policy"),
        (&plan.concept_registry_digest, "concept registry"),
        (&plan.recipe_schema_digest, "recipe schema"),
        (&plan.oracle_verifier_artifact_digest, "oracle verifier artifact"),
        (&plan.semantic_auditor_artifact_digest, "semantic auditor artifact"),
    ] {
        require_digest(value, label)?;
    }
    if plan.generator_id == plan.oracle_verifier_id
        || plan.generator_artifact_digest == plan.oracle_verifier_artifact_digest
        || plan.generator_id == plan.semantic_auditor_id
        || plan.generator_artifact_digest == plan.semantic_auditor_artifact_digest
        || plan.oracle_verifier_id == plan.semantic_auditor_id
        || plan.oracle_verifier_artifact_digest == plan.semantic_auditor_artifact_digest
    {
        return Err(AdaptiveBenchmarkError::GeneratorVerifierCollision(
            "generator, oracle verifier and semantic auditor must have distinct identities and artifacts".into(),
        ));
    }
    require_unique_ids(&plan.allowed_domains, "allowed domain ID")?;
    require_unique_ids(&plan.allowed_modalities, "allowed modality ID")?;
    require_unique_ids(&plan.allowed_reasoning_profiles, "allowed reasoning profile ID")?;
    if plan.allowed_domains.is_empty()
        || plan.allowed_modalities.is_empty()
        || plan.allowed_reasoning_profiles.is_empty()
        || plan.minimum_calibration_tasks == 0
        || plan.maximum_holdout_tasks == 0
    {
        return Err(AdaptiveBenchmarkError::InvalidPlan(
            "domains, modalities, reasoning profiles, calibration floor and holdout cap must be non-empty/positive".into(),
        ));
    }
    Ok(())
}

fn validate_freeze(
    plan: &AdaptiveBenchmarkPlan,
    registry: &ChallengeSplitRegistry,
    freeze: &AdaptiveBenchmarkFreezeReceipt,
    events: &[DevelopmentAdaptationEvent],
) -> Result<(), AdaptiveBenchmarkError> {
    for (value, label) in [
        (&freeze.freeze_receipt_id, "freeze receipt ID"),
        (&freeze.benchmark_id, "freeze benchmark ID"),
        (&freeze.benchmark_version, "freeze benchmark version"),
    ] {
        require_id(value, label)?;
    }
    for (value, label) in [
        (&freeze.freeze_receipt_digest, "freeze receipt"),
        (&freeze.plan_digest, "frozen plan"),
        (&freeze.adaptation_transcript_digest, "adaptation transcript"),
        (&freeze.generator_artifact_digest, "frozen generator"),
        (&freeze.refinement_policy_digest, "frozen refinement policy"),
        (&freeze.concept_registry_digest, "frozen concept registry"),
        (&freeze.recipe_schema_digest, "frozen recipe schema"),
        (&freeze.oracle_verifier_artifact_digest, "frozen oracle verifier"),
        (&freeze.semantic_auditor_artifact_digest, "frozen semantic auditor"),
    ] {
        require_digest(value, label)?;
    }
    require_unique_digests(&freeze.frozen_concept_digests, "frozen concept digest")?;
    require_unique_digests(&freeze.frozen_recipe_template_digests, "frozen recipe template digest")?;

    if freeze.benchmark_id != plan.benchmark_id
        || freeze.benchmark_version != plan.benchmark_version
        || freeze.plan_digest != plan.plan_digest
        || freeze.generator_artifact_digest != plan.generator_artifact_digest
        || freeze.refinement_policy_digest != plan.refinement_policy_digest
        || freeze.concept_registry_digest != plan.concept_registry_digest
        || freeze.recipe_schema_digest != plan.recipe_schema_digest
        || freeze.oracle_verifier_artifact_digest != plan.oracle_verifier_artifact_digest
        || freeze.semantic_auditor_artifact_digest != plan.semantic_auditor_artifact_digest
        || freeze.frozen_concept_digests.is_empty()
        || freeze.frozen_recipe_template_digests.is_empty()
        || freeze.frozen_at_unix_seconds == 0
    {
        return Err(AdaptiveBenchmarkError::FreezeMismatch(
            "freeze receipt does not bind the current plan, identities and non-empty frozen registries".into(),
        ));
    }

    if freeze.adaptation_event_count != events.len() {
        return Err(AdaptiveBenchmarkError::AdaptationHistory(
            "freeze event count does not equal supplied development transcript length".into(),
        ));
    }
    let mut event_ids = BTreeSet::new();
    let mut prior_recipe_digests = BTreeSet::new();
    let mut concepts = BTreeSet::new();
    let mut previous_time = 0u64;
    for (index, event) in events.iter().enumerate() {
        require_id(&event.event_id, "development adaptation event ID")?;
        require_id(&event.family_group_id, "development family-group ID")?;
        require_id(&event.seed_namespace_id, "development seed namespace")?;
        registry.validate_assignment(EvidenceSplit::Development, &event.family_group_id)
            .map_err(AdaptiveBenchmarkError::SplitFirewall)?;
        require_digest(&event.concept_digest, "adapted concept")?;
        require_digest(&event.candidate_recipe_template_digest, "candidate recipe template")?;
        require_digest(&event.solver_panel_digest, "development solver panel")?;
        require_digest(&event.solver_trajectory_digest, "development solver trajectory")?;
        require_digest(&event.judge_feedback_digest, "development judge feedback")?;
        let mut source_digests = event.source_artifacts
            .iter()
            .map(|artifact| artifact.artifact_digest.clone())
            .collect::<Vec<_>>();
        require_unique_digests(&source_digests, "development source artifact")?;
        for artifact in &event.source_artifacts {
            if artifact.split != EvidenceSplit::Development {
                return Err(AdaptiveBenchmarkError::SplitFirewall(
                    "development adaptation may consume Development artifacts only".into(),
                ));
            }
        }
        source_digests.sort();
        if event.parent_recipe_template_digest.as_ref().is_some_and(|digest| !valid_digest(digest)) {
            return Err(AdaptiveBenchmarkError::InvalidDigest(
                "parent recipe template digest is invalid".into(),
            ));
        }
        if event.split != EvidenceSplit::Development
            || event.sequence != index as u64
            || event.adapted_at_unix_seconds == 0
            || event.adapted_at_unix_seconds <= previous_time
            || event.adapted_at_unix_seconds >= freeze.frozen_at_unix_seconds
            || !event_ids.insert(event.event_id.clone())
        {
            return Err(AdaptiveBenchmarkError::AdaptationHistory(
                "development events must be unique, contiguous, strictly ordered and precede freeze".into(),
            ));
        }
        previous_time = event.adapted_at_unix_seconds;
        prior_recipe_digests.insert(event.candidate_recipe_template_digest.clone());
        concepts.insert(event.concept_digest.clone());
    }
    if !prior_recipe_digests.is_subset(
        &freeze.frozen_recipe_template_digests.iter().cloned().collect::<BTreeSet<_>>()
    ) || !concepts.is_subset(
        &freeze.frozen_concept_digests.iter().cloned().collect::<BTreeSet<_>>()
    ) {
        return Err(AdaptiveBenchmarkError::FreezeMismatch(
            "freeze registry omits a concept or recipe produced in the development transcript".into(),
        ));
    }
    Ok(())
}

fn validate_calibration(
    plan: &AdaptiveBenchmarkPlan,
    registry: &ChallengeSplitRegistry,
    freeze: &AdaptiveBenchmarkFreezeReceipt,
    development_events: &[DevelopmentAdaptationEvent],
    calibration: &CalibrationQualificationReceipt,
) -> Result<(), AdaptiveBenchmarkError> {
    require_id(&calibration.receipt_id, "calibration receipt ID")?;
    require_id(&calibration.benchmark_id, "calibration benchmark ID")?;
    require_id(&calibration.family_group_id, "calibration family-group ID")?;
    require_id(&calibration.seed_namespace_id, "calibration seed namespace")?;
    for (value, label) in [
        (&calibration.receipt_digest, "calibration receipt"),
        (&calibration.freeze_receipt_digest, "calibration freeze receipt"),
        (&calibration.generator_artifact_digest, "calibration generator"),
        (&calibration.oracle_verifier_artifact_digest, "calibration verifier"),
        (&calibration.seed_commitment_digest, "calibration seed commitment"),
    ] {
        require_digest(value, label)?;
    }
    registry.validate_assignment(EvidenceSplit::Calibration, &calibration.family_group_id)
        .map_err(AdaptiveBenchmarkError::SplitFirewall)?;
    if development_events.iter().any(|event| event.seed_namespace_id == calibration.seed_namespace_id) {
        return Err(AdaptiveBenchmarkError::SeedNamespaceReuse(
            "calibration seed namespace must be disjoint from every Development namespace".into(),
        ));
    }
    if calibration.split != EvidenceSplit::Calibration
        || calibration.benchmark_id != plan.benchmark_id
        || calibration.freeze_receipt_digest != freeze.freeze_receipt_digest
        || calibration.generator_artifact_digest != plan.generator_artifact_digest
        || calibration.oracle_verifier_artifact_digest != plan.oracle_verifier_artifact_digest
        || calibration.seed_namespace_id.trim().is_empty()
        || calibration.generated_at_unix_seconds <= freeze.frozen_at_unix_seconds
        || calibration.completed_at_unix_seconds < calibration.generated_at_unix_seconds
        || calibration.task_count < plan.minimum_calibration_tasks
        || calibration.independently_verified_task_count != calibration.task_count
        || calibration.solvable_task_count != calibration.task_count
        || calibration.ambiguous_task_count > plan.maximum_calibration_ambiguous_tasks
        || !calibration.quality_gate_passed
    {
        return Err(AdaptiveBenchmarkError::CalibrationInvalid(
            "calibration must run after freeze, use the frozen generator/verifier and pass solvability, verification and ambiguity gates".into(),
        ));
    }
    Ok(())
}

/// Verify a split-firewalled adaptive benchmark release.
///
/// The independent semantic auditor must be a different identity/artifact from
/// both the generator and task oracle verifier. Any potential near-duplicate
/// against a previously exposed task causes a fail-closed rejection unless an
/// external workflow removes it from the release and regenerates the manifest.
pub fn verify_adaptive_benchmark_release<A>(
    plan: &AdaptiveBenchmarkPlan,
    registry: &ChallengeSplitRegistry,
    development_events: &[DevelopmentAdaptationEvent],
    freeze: &AdaptiveBenchmarkFreezeReceipt,
    calibration: &CalibrationQualificationReceipt,
    release: &AdaptiveBenchmarkSealedRelease,
    auditor: &A,
) -> Result<AdaptiveBenchmarkLifecycleReceipt, AdaptiveBenchmarkError>
where
    A: BenchmarkSemanticOverlapAuditor,
{
    validate_plan(plan)?;
    registry.validate().map_err(AdaptiveBenchmarkError::SplitFirewall)?;
    validate_freeze(plan, registry, freeze, development_events)?;
    validate_calibration(plan, registry, freeze, development_events, calibration)?;

    for (value, label) in [
        (&release.release_id, "sealed benchmark release ID"),
        (&release.benchmark_id, "sealed benchmark ID"),
        (&release.benchmark_version, "sealed benchmark version"),
        (&release.family_group_id, "sealed release family-group ID"),
        (&release.semantic_auditor_id, "release semantic auditor ID"),
        (&release.hidden_seed_namespace_id, "sealed hidden seed namespace"),
    ] {
        require_id(value, label)?;
    }
    for (value, label) in [
        (&release.release_digest, "sealed release digest"),
        (&release.freeze_receipt_digest, "release freeze receipt"),
        (&release.calibration_receipt_digest, "release calibration receipt"),
        (&release.generator_artifact_digest, "release generator"),
        (&release.semantic_auditor_artifact_digest, "release semantic auditor"),
        (&release.hidden_seed_commitment_digest, "hidden seed commitment"),
    ] {
        require_digest(value, label)?;
    }
    if release.generation_inputs.is_empty() {
        return Err(AdaptiveBenchmarkError::SealedReleaseInvalid(
            "sealed generation input inventory must be non-empty".into(),
        ));
    }
    let mut input_digest_values = Vec::with_capacity(release.generation_inputs.len());
    for input in &release.generation_inputs {
        require_digest(&input.artifact_digest, "generation input artifact")?;
        match input.role {
            GenerationInputRole::FrozenPlan => {
                if input.split.is_some() {
                    return Err(AdaptiveBenchmarkError::SplitFirewall(
                        "frozen plan inputs must not be assigned to an evaluation split".into(),
                    ));
                }
            }
            GenerationInputRole::DevelopmentEvidence => {
                if input.split != Some(EvidenceSplit::Development) {
                    return Err(AdaptiveBenchmarkError::SplitFirewall(
                        "generator development evidence inputs must be Development-only".into(),
                    ));
                }
            }
            GenerationInputRole::CalibrationQualification => {
                if input.split != Some(EvidenceSplit::Calibration)
                    || input.artifact_digest != calibration.receipt_digest
                {
                    return Err(AdaptiveBenchmarkError::SplitFirewall(
                        "the only Calibration input allowed at sealed generation is the aggregate qualification receipt".into(),
                    ));
                }
            }
        }
        input_digest_values.push(input.artifact_digest.clone());
    }
    require_unique_digests(&input_digest_values, "generation input artifact")?;
    let generation_input_artifacts = input_digest_values.iter().cloned().collect::<BTreeSet<_>>();
    let development_artifacts = development_events.iter().flat_map(|event| {
        let mut digests = vec![
            event.concept_digest.clone(),
            event.candidate_recipe_template_digest.clone(),
            event.solver_panel_digest.clone(),
            event.solver_trajectory_digest.clone(),
            event.judge_feedback_digest.clone(),
        ];
        digests.extend(event.source_artifacts.iter().map(|artifact| artifact.artifact_digest.clone()));
        digests
    }).collect::<BTreeSet<_>>();
    for input in &release.generation_inputs {
        match input.role {
            GenerationInputRole::FrozenPlan => {
                if ![
                    freeze.freeze_receipt_digest.as_str(),
                    freeze.plan_digest.as_str(),
                    plan.generator_artifact_digest.as_str(),
                    plan.refinement_policy_digest.as_str(),
                    plan.concept_registry_digest.as_str(),
                    plan.recipe_schema_digest.as_str(),
                    plan.oracle_verifier_artifact_digest.as_str(),
                    plan.semantic_auditor_artifact_digest.as_str(),
                ].contains(&input.artifact_digest.as_str()) {
                    return Err(AdaptiveBenchmarkError::SealedReleaseInvalid(
                        "frozen-plan generation input is not present in the frozen manifest".into(),
                    ));
                }
            }
            GenerationInputRole::DevelopmentEvidence => {
                if !development_artifacts.contains(&input.artifact_digest) {
                    return Err(AdaptiveBenchmarkError::SplitFirewall(
                        "Development generation input is not in the admitted adaptation transcript".into(),
                    ));
                }
            }
            GenerationInputRole::CalibrationQualification => {}
        }
    }
    if !generation_input_artifacts.contains(&freeze.freeze_receipt_digest)
        || !generation_input_artifacts.contains(&calibration.receipt_digest)
        || generation_input_artifacts.contains(&release.hidden_seed_commitment_digest)
    {
        return Err(AdaptiveBenchmarkError::SealedReleaseInvalid(
            "generation inputs must include freeze/calibration receipts but not hidden seed commitment".into(),
        ));
    }

    registry.validate_assignment(EvidenceSplit::SealedHoldout, &release.family_group_id)
        .map_err(AdaptiveBenchmarkError::SplitFirewall)?;
    if release.split != EvidenceSplit::SealedHoldout
        || release.benchmark_id != plan.benchmark_id
        || release.benchmark_version != plan.benchmark_version
        || release.freeze_receipt_digest != freeze.freeze_receipt_digest
        || release.calibration_receipt_digest != calibration.receipt_digest
        || release.generator_artifact_digest != plan.generator_artifact_digest
        || release.semantic_auditor_id != plan.semantic_auditor_id
        || release.semantic_auditor_artifact_digest != plan.semantic_auditor_artifact_digest
        || auditor.identity() != plan.semantic_auditor_id
        || auditor.artifact_digest() != plan.semantic_auditor_artifact_digest
        || release.hidden_seed_namespace_id == calibration.seed_namespace_id
        || development_events.iter().any(|event| event.seed_namespace_id == release.hidden_seed_namespace_id)
        || release.released_at_unix_seconds <= calibration.completed_at_unix_seconds
        || release.task_artifacts.is_empty()
        || release.task_artifacts.len() > plan.maximum_holdout_tasks
        || release.prior_exposures.len() > MAX_EXPOSURE_RECORDS
    {
        return Err(AdaptiveBenchmarkError::SealedReleaseInvalid(
            "sealed release is not bound to the frozen plan/calibration or violates timing, seed, auditor or size limits".into(),
        ));
    }

    let mut exposure_ids = BTreeSet::new();
    let mut previously_exposed_task_digests = BTreeSet::new();
    for exposure in &release.prior_exposures {
        require_id(&exposure.exposure_id, "prior exposure ID")?;
        require_id(&exposure.benchmark_release_id, "prior release ID")?;
        require_id(&exposure.exposed_agent_id, "exposed agent ID")?;
        require_id(&exposure.exposed_agent_version, "exposed agent version")?;
        require_digest(&exposure.task_public_digest, "previously exposed public task")?;
        require_digest(&exposure.prompt_or_scaffold_digest, "prior prompt/scaffold")?;
        if exposure.exposed_at_unix_seconds == 0
            || exposure.exposed_at_unix_seconds >= release.released_at_unix_seconds
            || !exposure_ids.insert(exposure.exposure_id.clone())
        {
            return Err(AdaptiveBenchmarkError::SealedReleaseInvalid(
                "prior exposures must be uniquely identified and temporally precede the sealed release".into(),
            ));
        }
        previously_exposed_task_digests.insert(exposure.task_public_digest.clone());
    }

    let frozen_concepts = freeze.frozen_concept_digests.iter().cloned().collect::<BTreeSet<_>>();
    let frozen_recipes = freeze.frozen_recipe_template_digests.iter().cloned().collect::<BTreeSet<_>>();
    let allowed_domains = plan.allowed_domains.iter().cloned().collect::<BTreeSet<_>>();
    let allowed_modalities = plan.allowed_modalities.iter().cloned().collect::<BTreeSet<_>>();
    let allowed_reasoning = plan.allowed_reasoning_profiles.iter().cloned().collect::<BTreeSet<_>>();
    let mut task_ids = BTreeSet::new();
    let mut task_digests = BTreeSet::new();
    let mut public_input_digests = BTreeSet::new();
    let mut oracle_commitments = BTreeSet::new();
    let mut seed_commitments = BTreeSet::new();
    let mut semantic_comparisons = 0usize;

    let prior_comparisons = release.prior_exposures.len()
        .checked_mul(release.task_artifacts.len())
        .ok_or_else(|| AdaptiveBenchmarkError::ArithmeticOverflow(
            "prior exposure comparison count overflow".into(),
        ))?;
    let within_release_comparisons = release.task_artifacts.len()
        .checked_mul(release.task_artifacts.len().saturating_sub(1))
        .map(|value| value / 2)
        .ok_or_else(|| AdaptiveBenchmarkError::ArithmeticOverflow(
            "within-release comparison count overflow".into(),
        ))?;
    if prior_comparisons.checked_add(within_release_comparisons).ok_or_else(|| {
        AdaptiveBenchmarkError::ArithmeticOverflow("total semantic comparison count overflow".into())
    })? > MAX_SEMANTIC_COMPARISONS {
        return Err(AdaptiveBenchmarkError::SemanticOverlap(
            "semantic overlap workload exceeds the configured comparison cap".into(),
        ));
    }

    for task in &release.task_artifacts {
        for (value, label) in [
            (&task.task_id, "sealed task ID"),
            (&task.family_group_id, "sealed task family-group ID"),
            (&task.domain_id, "sealed task domain"),
            (&task.modality_id, "sealed task modality"),
            (&task.reasoning_profile_id, "sealed task reasoning profile"),
            (&task.generator_id, "sealed task generator ID"),
            (&task.generator_version, "sealed task generator version"),
            (&task.seed_namespace_id, "sealed task seed namespace"),
        ] {
            require_id(value, label)?;
        }
        for (value, label) in [
            (&task.concept_digest, "sealed task concept"),
            (&task.recipe_template_digest, "sealed task recipe template"),
            (&task.recipe_instance_digest, "sealed task recipe instance"),
            (&task.generator_artifact_digest, "sealed task generator artifact"),
            (&task.seed_commitment_digest, "sealed task seed commitment"),
            (&task.public_task_digest, "sealed public task"),
            (&task.public_input_artifact_digest, "sealed public input"),
            (&task.sealed_oracle_commitment_digest, "sealed oracle commitment"),
        ] {
            require_digest(value, label)?;
        }
        if task.split != EvidenceSplit::SealedHoldout
            || task.family_group_id != release.family_group_id
            || task.generator_id != plan.generator_id
            || task.generator_version != plan.generator_version
            || task.generator_artifact_digest != plan.generator_artifact_digest
            || task.seed_namespace_id != release.hidden_seed_namespace_id
            || !frozen_concepts.contains(&task.concept_digest)
            || !frozen_recipes.contains(&task.recipe_template_digest)
            || !allowed_domains.contains(&task.domain_id)
            || !allowed_modalities.contains(&task.modality_id)
            || !allowed_reasoning.contains(&task.reasoning_profile_id)
            || task.generated_at_unix_seconds <= calibration.completed_at_unix_seconds
            || task.generated_at_unix_seconds >= release.released_at_unix_seconds
        {
            return Err(AdaptiveBenchmarkError::TaskInvalid(format!(
                "sealed task {} violates frozen recipe, concept, split, domain or generation timing",
                task.task_id
            )));
        }
        if !task_ids.insert(task.task_id.clone())
            || !task_digests.insert(task.public_task_digest.clone())
            || !public_input_digests.insert(task.public_input_artifact_digest.clone())
            || !oracle_commitments.insert(task.sealed_oracle_commitment_digest.clone())
            || !seed_commitments.insert(task.seed_commitment_digest.clone())
        {
            return Err(AdaptiveBenchmarkError::DuplicateTask(
                "task IDs, public task digests, public inputs and oracle commitments must be unique".into(),
            ));
        }
        if generation_input_artifacts.contains(&task.sealed_oracle_commitment_digest)
            || generation_input_artifacts.contains(&task.oracle_validation.validation_receipt_digest)
            || task.generator_input_artifact_digests.contains(&task.sealed_oracle_commitment_digest)
            || task.generator_input_artifact_digests.contains(&task.oracle_validation.validation_receipt_digest)
        {
            return Err(AdaptiveBenchmarkError::AnswerLeakage(
                "sealed answer commitment or oracle validation receipt appears in generator inputs".into(),
            ));
        }
        if task.generator_input_artifact_digests != input_digest_values {
            return Err(AdaptiveBenchmarkError::TaskInvalid(
                "task-specific generation input inventory differs from the release manifest".into(),
            ));
        }
        require_unique_digests(&task.generator_input_artifact_digests, "task generator input")?;

        let validation = &task.oracle_validation;
        if validation.verifier_id != plan.oracle_verifier_id
            || validation.verifier_version != plan.oracle_verifier_version
            || validation.verifier_artifact_digest != plan.oracle_verifier_artifact_digest
            || validation.task_public_digest != task.public_task_digest
            || validation.sealed_oracle_commitment_digest != task.sealed_oracle_commitment_digest
            || !valid_digest(&validation.validation_receipt_digest)
            || !validation.solvable
            || !validation.unique_ground_truth
            || !validation.negative_control_passed
            || validation.ambiguity_detected
            || validation.verified_at_unix_seconds < task.generated_at_unix_seconds
            || validation.verified_at_unix_seconds >= release.released_at_unix_seconds
        {
            return Err(AdaptiveBenchmarkError::TaskInvalid(format!(
                "task {} lacks a passing independent oracle validation receipt",
                task.task_id
            )));
        }

        if previously_exposed_task_digests.contains(&task.public_task_digest) {
            return Err(AdaptiveBenchmarkError::DuplicateTask(format!(
                "task {} exactly duplicates a previously exposed task",
                task.task_id
            )));
        }
        for prior_digest in &previously_exposed_task_digests {
            let decision = auditor
                .audit_overlap(&task.public_task_digest, prior_digest)
                .map_err(AdaptiveBenchmarkError::ProviderFailure)?;
            semantic_comparisons = semantic_comparisons.checked_add(1).ok_or_else(|| {
                AdaptiveBenchmarkError::ArithmeticOverflow("semantic comparison count overflow".into())
            })?;
            if decision == SemanticOverlapDecision::PotentialNearDuplicate {
                return Err(AdaptiveBenchmarkError::SemanticOverlap(format!(
                    "task {} is a potential semantic duplicate of a previously exposed task",
                    task.task_id
                )));
            }
        }
    }

    for (left_index, left) in release.task_artifacts.iter().enumerate() {
        for right in release.task_artifacts.iter().skip(left_index + 1) {
            let decision = auditor
                .audit_overlap(&left.public_task_digest, &right.public_task_digest)
                .map_err(AdaptiveBenchmarkError::ProviderFailure)?;
            semantic_comparisons = semantic_comparisons.checked_add(1).ok_or_else(|| {
                AdaptiveBenchmarkError::ArithmeticOverflow("semantic comparison count overflow".into())
            })?;
            if decision == SemanticOverlapDecision::PotentialNearDuplicate {
                return Err(AdaptiveBenchmarkError::SemanticOverlap(format!(
                    "sealed tasks {} and {} are potential semantic near-duplicates",
                    left.task_id, right.task_id
                )));
            }
        }
    }

    Ok(AdaptiveBenchmarkLifecycleReceipt {
        benchmark_id: plan.benchmark_id.clone(),
        benchmark_version: plan.benchmark_version.clone(),
        freeze_receipt_digest: freeze.freeze_receipt_digest.clone(),
        calibration_receipt_digest: calibration.receipt_digest.clone(),
        release_id: release.release_id.clone(),
        release_digest: release.release_digest.clone(),
        development_adaptation_count: development_events.len(),
        frozen_concept_count: freeze.frozen_concept_digests.len(),
        frozen_recipe_template_count: freeze.frozen_recipe_template_digests.len(),
        calibration_task_count: calibration.task_count,
        sealed_task_count: release.task_artifacts.len(),
        prior_exposure_count: release.prior_exposures.len(),
        semantic_overlap_comparisons: semantic_comparisons,
        generator_verifier_independent: true,
        generator_semantic_auditor_independent: true,
        sealed_answers_absent_from_generation_inputs: true,
        calibration_did_not_mutate_frozen_policy: true,
        holdout_seed_namespace_disjoint: true,
        exact_task_digests_unique: true,
        semantic_overlap_audit_passed: true,
        split_firewall_passed: true,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        AdaptiveBenchmarkFreezeReceipt, AdaptiveBenchmarkPlan,
        AdaptiveBenchmarkSealedRelease, AdaptiveBenchmarkTaskArtifact,
        BenchmarkSemanticOverlapAuditor, CalibrationQualificationReceipt,
        DevelopmentAdaptationEvent, GenerationInputArtifact, GenerationInputRole,
        IndependentOracleValidationReceipt, PriorBenchmarkExposure,
        SemanticOverlapDecision, SplitBoundBenchmarkArtifact,
        verify_adaptive_benchmark_release,
    };
    use super::super::counterexample_ledger::EvidenceSplit;
    use super::super::scientific_challenge::ChallengeSplitRegistry;

    const D1: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const D2: &str = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const D3: &str = "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const D4: &str = "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
    const D5: &str = "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";

    struct NoSemanticOverlap;
    impl BenchmarkSemanticOverlapAuditor for NoSemanticOverlap {
        fn identity(&self) -> &str {
            "similarity-auditor-c"
        }

        fn artifact_digest(&self) -> &str {
            D4
        }

        fn audit_overlap(
            &self,
            _candidate_task_digest: &str,
            _previously_exposed_task_digest: &str,
        ) -> Result<SemanticOverlapDecision, String> {
            Ok(SemanticOverlapDecision::NoMaterialOverlap)
        }
    }

    struct AlwaysNearDuplicate;
    impl BenchmarkSemanticOverlapAuditor for AlwaysNearDuplicate {
        fn identity(&self) -> &str {
            "similarity-auditor-c"
        }

        fn artifact_digest(&self) -> &str {
            D4
        }

        fn audit_overlap(
            &self,
            _candidate_task_digest: &str,
            _previously_exposed_task_digest: &str,
        ) -> Result<SemanticOverlapDecision, String> {
            Ok(SemanticOverlapDecision::PotentialNearDuplicate)
        }
    }

    fn registry() -> ChallengeSplitRegistry {
        ChallengeSplitRegistry {
            development_group_ids: vec!["dev-a".into()],
            calibration_group_ids: vec!["cal-a".into()],
            sealed_holdout_group_ids: vec!["holdout-a".into()],
        }
    }

    fn plan() -> AdaptiveBenchmarkPlan {
        AdaptiveBenchmarkPlan {
            benchmark_id: "science-benchmark".into(),
            benchmark_version: "v1".into(),
            plan_digest: D2.into(),
            generator_id: "generator-a".into(),
            generator_version: "v1".into(),
            generator_artifact_digest: D1.into(),
            refinement_policy_digest: D2.into(),
            concept_registry_digest: D3.into(),
            recipe_schema_digest: D4.into(),
            oracle_verifier_id: "oracle-verifier-b".into(),
            oracle_verifier_version: "v1".into(),
            oracle_verifier_artifact_digest: D5.into(),
            semantic_auditor_id: "similarity-auditor-c".into(),
            semantic_auditor_version: "v1".into(),
            semantic_auditor_artifact_digest: D4.into(),
            allowed_domains: vec!["materials-science".into(), "biology".into()],
            allowed_modalities: vec!["tabular".into(), "sensor-time-series".into()],
            allowed_reasoning_profiles: vec!["causal".into(), "decision-aware".into()],
            minimum_calibration_tasks: 2,
            maximum_calibration_ambiguous_tasks: 0,
            maximum_holdout_tasks: 20,
        }
    }

    fn event(sequence: u64, recipe: &str) -> DevelopmentAdaptationEvent {
        DevelopmentAdaptationEvent {
            event_id: format!("dev-event-{sequence}"),
            sequence,
            split: EvidenceSplit::Development,
            family_group_id: "dev-a".into(),
            seed_namespace_id: "development-seeds".into(),
            concept_digest: D3.into(),
            parent_recipe_template_digest: None,
            candidate_recipe_template_digest: recipe.into(),
            solver_panel_digest: D1.into(),
            solver_trajectory_digest: D2.into(),
            judge_feedback_digest: D4.into(),
            source_artifacts: vec![
                SplitBoundBenchmarkArtifact {
                    artifact_digest: D1.into(),
                    split: EvidenceSplit::Development,
                },
                SplitBoundBenchmarkArtifact {
                    artifact_digest: D2.into(),
                    split: EvidenceSplit::Development,
                },
            ],
            adapted_at_unix_seconds: 100 + sequence,
        }
    }

    fn freeze(events: &[DevelopmentAdaptationEvent]) -> AdaptiveBenchmarkFreezeReceipt {
        AdaptiveBenchmarkFreezeReceipt {
            freeze_receipt_id: "freeze-1".into(),
            freeze_receipt_digest: D1.into(),
            benchmark_id: "science-benchmark".into(),
            benchmark_version: "v1".into(),
            plan_digest: D2.into(),
            adaptation_transcript_digest: D3.into(),
            generator_artifact_digest: D1.into(),
            refinement_policy_digest: D2.into(),
            concept_registry_digest: D3.into(),
            recipe_schema_digest: D4.into(),
            oracle_verifier_artifact_digest: D5.into(),
            semantic_auditor_artifact_digest: D4.into(),
            frozen_concept_digests: vec![D3.into()],
            frozen_recipe_template_digests: events.iter().map(|event| event.candidate_recipe_template_digest.clone()).collect(),
            adaptation_event_count: events.len(),
            frozen_at_unix_seconds: 500,
        }
    }

    fn calibration() -> CalibrationQualificationReceipt {
        CalibrationQualificationReceipt {
            receipt_id: "calibration-1".into(),
            receipt_digest: D2.into(),
            benchmark_id: "science-benchmark".into(),
            split: EvidenceSplit::Calibration,
            family_group_id: "cal-a".into(),
            freeze_receipt_digest: D1.into(),
            generator_artifact_digest: D1.into(),
            oracle_verifier_artifact_digest: D5.into(),
            seed_namespace_id: "calibration-seeds".into(),
            seed_commitment_digest: D3.into(),
            task_count: 5,
            independently_verified_task_count: 5,
            solvable_task_count: 5,
            ambiguous_task_count: 0,
            generated_at_unix_seconds: 600,
            completed_at_unix_seconds: 650,
            quality_gate_passed: true,
        }
    }

    fn task(task_id: &str, suffix: &str) -> AdaptiveBenchmarkTaskArtifact {
        let public = if suffix == "a" { D1 } else { D2 };
        let oracle = if suffix == "a" { D3 } else { D4 };
        AdaptiveBenchmarkTaskArtifact {
            task_id: task_id.into(),
            concept_digest: D3.into(),
            recipe_template_digest: D2.into(),
            recipe_instance_digest: D4.into(),
            family_group_id: "holdout-a".into(),
            split: EvidenceSplit::SealedHoldout,
            domain_id: "materials-science".into(),
            modality_id: "tabular".into(),
            reasoning_profile_id: "causal".into(),
            generator_id: "generator-a".into(),
            generator_version: "v1".into(),
            generator_artifact_digest: D1.into(),
            seed_namespace_id: "holdout-seeds".into(),
            seed_commitment_digest: D5.into(),
            public_task_digest: public.into(),
            public_input_artifact_digest: if suffix == "a" { D2 } else { D3 }.into(),
            sealed_oracle_commitment_digest: oracle.into(),
            oracle_validation: IndependentOracleValidationReceipt {
                verifier_id: "oracle-verifier-b".into(),
                verifier_version: "v1".into(),
                verifier_artifact_digest: D5.into(),
                task_public_digest: public.into(),
                sealed_oracle_commitment_digest: oracle.into(),
                validation_receipt_digest: if suffix == "a" { D4 } else { D5 }.into(),
                solvable: true,
                unique_ground_truth: true,
                negative_control_passed: true,
                ambiguity_detected: false,
                verified_at_unix_seconds: 800,
            },
            generator_input_artifact_digests: vec![D1.into(), D2.into()],
            generated_at_unix_seconds: 700,
        }
    }

    fn exposure() -> PriorBenchmarkExposure {
        PriorBenchmarkExposure {
            exposure_id: "old-exposure-1".into(),
            benchmark_release_id: "old-release".into(),
            task_public_digest: D5.into(),
            exposed_agent_id: "agent-a".into(),
            exposed_agent_version: "v1".into(),
            prompt_or_scaffold_digest: D4.into(),
            exposed_at_unix_seconds: 400,
        }
    }

    fn release(tasks: Vec<AdaptiveBenchmarkTaskArtifact>) -> AdaptiveBenchmarkSealedRelease {
        AdaptiveBenchmarkSealedRelease {
            release_id: "release-1".into(),
            release_digest: D4.into(),
            benchmark_id: "science-benchmark".into(),
            benchmark_version: "v1".into(),
            split: EvidenceSplit::SealedHoldout,
            family_group_id: "holdout-a".into(),
            freeze_receipt_digest: D1.into(),
            calibration_receipt_digest: D2.into(),
            generator_artifact_digest: D1.into(),
            semantic_auditor_id: "similarity-auditor-c".into(),
            semantic_auditor_artifact_digest: D4.into(),
            hidden_seed_namespace_id: "holdout-seeds".into(),
            hidden_seed_commitment_digest: D5.into(),
            generation_inputs: vec![
                GenerationInputArtifact {
                    artifact_digest: D1.into(),
                    split: None,
                    role: GenerationInputRole::FrozenPlan,
                },
                GenerationInputArtifact {
                    artifact_digest: D2.into(),
                    split: Some(EvidenceSplit::Calibration),
                    role: GenerationInputRole::CalibrationQualification,
                },
            ],
            task_artifacts: tasks,
            prior_exposures: vec![exposure()],
            released_at_unix_seconds: 900,
        }
    }

    fn verify(
        plan: &AdaptiveBenchmarkPlan,
        events: &[DevelopmentAdaptationEvent],
        freeze: &AdaptiveBenchmarkFreezeReceipt,
        calibration: &CalibrationQualificationReceipt,
        release: &AdaptiveBenchmarkSealedRelease,
        auditor: &impl BenchmarkSemanticOverlapAuditor,
    ) -> Result<super::AdaptiveBenchmarkLifecycleReceipt, super::AdaptiveBenchmarkError> {
        verify_adaptive_benchmark_release(plan, &registry(), events, freeze, calibration, release, auditor)
    }

    #[test]
    fn accepts_frozen_split_firewalled_release_with_independent_verifier_and_auditor() {
        let events = vec![event(0, D2)];
        let freeze = freeze(&events);
        let receipt = verify(&plan(), &events, &freeze, &calibration(), &release(vec![task("task-1", "a")]), &NoSemanticOverlap).unwrap();
        assert!(receipt.split_firewall_passed);
        assert!(receipt.generator_verifier_independent);
        assert!(receipt.generator_semantic_auditor_independent);
        assert!(receipt.sealed_answers_absent_from_generation_inputs);
        assert_eq!(receipt.calibration_task_count, 5);
        assert_eq!(receipt.sealed_task_count, 1);
    }

    #[test]
    fn rejects_calibration_or_holdout_adaptation_after_freeze() {
        let events = vec![event(0, D2)];
        let mut leaked = events.clone();
        leaked[0].split = EvidenceSplit::Calibration;
        assert!(verify(&plan(), &leaked, &freeze(&events), &calibration(), &release(vec![task("task-1", "a")]), &NoSemanticOverlap).is_err());

        let mut late = events.clone();
        late[0].adapted_at_unix_seconds = 600;
        assert!(verify(&plan(), &late, &freeze(&events), &calibration(), &release(vec![task("task-1", "a")]), &NoSemanticOverlap).is_err());
    }

    #[test]
    fn rejects_calibration_quality_failure_and_freeze_mismatch() {
        let events = vec![event(0, D2)];
        let freeze = freeze(&events);
        let mut bad_calibration = calibration();
        bad_calibration.quality_gate_passed = false;
        assert!(verify(&plan(), &events, &freeze, &bad_calibration, &release(vec![task("task-1", "a")]), &NoSemanticOverlap).is_err());

        let mut bad_calibration = calibration();
        bad_calibration.freeze_receipt_digest = D4.into();
        assert!(verify(&plan(), &events, &freeze, &bad_calibration, &release(vec![task("task-1", "a")]), &NoSemanticOverlap).is_err());
    }

    #[test]
    fn rejects_sealed_answer_leakage_and_generator_verifier_identity_collision() {
        let events = vec![event(0, D2)];
        let freeze = freeze(&events);
        let mut leaked = release(vec![task("task-1", "a")]);
        leaked.task_artifacts[0].generator_input_artifact_digests.push(D3.into());
        assert!(verify(&plan(), &events, &freeze, &calibration(), &leaked, &NoSemanticOverlap).is_err());

        let mut plan = plan();
        plan.oracle_verifier_id = plan.generator_id.clone();
        assert!(verify(&plan, &events, &freeze, &calibration(), &release(vec![task("task-1", "a")]), &NoSemanticOverlap).is_err());
    }

    #[test]
    fn rejects_seed_namespace_reuse_recipe_mutation_and_unsolvable_tasks() {
        let events = vec![event(0, D2)];
        let freeze = freeze(&events);
        let mut calibration = calibration();
        calibration.seed_namespace_id = "holdout-seeds".into();
        assert!(verify(&plan(), &events, &freeze, &calibration, &release(vec![task("task-1", "a")]), &NoSemanticOverlap).is_err());

        let mut changed_task = task("task-1", "a");
        changed_task.recipe_template_digest = D4.into();
        assert!(verify(&plan(), &events, &freeze, &calibration(), &release(vec![changed_task]), &NoSemanticOverlap).is_err());

        let mut changed_task = task("task-1", "a");
        changed_task.oracle_validation.solvable = false;
        assert!(verify(&plan(), &events, &freeze, &calibration(), &release(vec![changed_task]), &NoSemanticOverlap).is_err());
    }

    #[test]
    fn exact_and_semantic_duplicate_exposure_fail_closed() {
        let events = vec![event(0, D2)];
        let freeze = freeze(&events);
        let mut exact = release(vec![task("task-1", "a")]);
        exact.prior_exposures[0].task_public_digest = D1.into();
        assert!(verify(&plan(), &events, &freeze, &calibration(), &exact, &NoSemanticOverlap).is_err());

        let near = release(vec![task("task-1", "a")]);
        assert!(verify(&plan(), &events, &freeze, &calibration(), &near, &AlwaysNearDuplicate).is_err());
    }

    #[test]
    fn rejects_non_independent_verifier_receipt_and_unregistered_split_groups() {
        let events = vec![event(0, D2)];
        let freeze = freeze(&events);
        let mut changed_task = task("task-1", "a");
        changed_task.oracle_validation.verifier_artifact_digest = D1.into();
        assert!(verify(&plan(), &events, &freeze, &calibration(), &release(vec![changed_task]), &NoSemanticOverlap).is_err());

        let mut changed_registry = registry();
        changed_registry.sealed_holdout_group_ids.clear();
        assert!(verify_adaptive_benchmark_release(&plan(), &changed_registry, &events, &freeze, &calibration(), &release(vec![task("task-1", "a")]), &NoSemanticOverlap).is_err());
    }
}
