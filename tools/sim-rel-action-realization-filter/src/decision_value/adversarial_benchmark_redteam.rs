//! Preregistered adversarial red-team evaluation of benchmark integrity.
//!
//! This module measures attack success, detector recall, false positives and
//! containment as separate quantities. A passing receipt is bounded by the
//! declared threat model and campaign; it is not a proof of unhackability.

use super::counterexample_ledger::EvidenceSplit;
use super::matched_benchmark::BenchmarkScope;
use std::collections::{BTreeMap, BTreeSet};

const MAX_FAMILIES: usize = 32;
const MAX_SCENARIOS: usize = 100_000;
const MAX_TRIALS: usize = 100_000;
const MAX_ID_BYTES: usize = 256;
const MAX_RESOURCE_CHANNELS: usize = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ThreatFamily {
    SearchTimeMetadataLeakage,
    QuestionContextLeakage,
    ExplicitOracleLeakage,
    SemanticNearDuplicateContamination,
    SplitLabelSmuggling,
    OracleVerifierCollusion,
    SideChannelLeakage,
    ResourceLaundering,
    SelectiveReportingAndDenominatorGaming,
    EvaluatorContaminationAndThresholdMutation,
    AnchorExposureAndCrossReleaseMemorization,
    IdentityAndProvenanceSubstitution,
}

impl ThreatFamily {
    pub const ALL: [Self; 12] = [
        Self::SearchTimeMetadataLeakage,
        Self::QuestionContextLeakage,
        Self::ExplicitOracleLeakage,
        Self::SemanticNearDuplicateContamination,
        Self::SplitLabelSmuggling,
        Self::OracleVerifierCollusion,
        Self::SideChannelLeakage,
        Self::ResourceLaundering,
        Self::SelectiveReportingAndDenominatorGaming,
        Self::EvaluatorContaminationAndThresholdMutation,
        Self::AnchorExposureAndCrossReleaseMemorization,
        Self::IdentityAndProvenanceSubstitution,
    ];
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScenarioKind {
    Attack,
    CleanNegativeControl,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RedTeamResourceChannel {
    pub channel_id: String,
    pub unit: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThreatFamilyRequirement {
    pub family: ThreatFamily,
    pub critical: bool,
    pub minimum_attack_scenarios: usize,
    pub minimum_clean_controls: usize,
    pub minimum_successful_exploits: usize,
    pub minimum_attack_authors: usize,
    pub minimum_generator_variants: usize,
    pub minimum_clean_control_authors: usize,
    pub minimum_correct_detection_percent: u8,
    pub maximum_false_positive_percent: u8,
    pub maximum_uncontained_exploit_percent: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdversarialRedTeamPlan {
    pub campaign_id: String,
    pub campaign_version: String,
    pub benchmark_id: String,
    pub benchmark_version: String,
    pub sealed_release_id: String,
    pub sealed_release_digest: String,
    pub source_hierarchical_analysis_id: String,
    pub source_hierarchical_analysis_digest: String,
    pub source_pre_registration_digest: String,
    pub plan_digest: String,
    pub threat_model_digest: String,
    pub attack_catalog_digest: String,
    pub detector_artifact_digest: String,
    pub attack_harness_digest: String,
    pub independent_verifier_artifact_digest: String,
    pub attack_operator_identity: String,
    pub detector_owner_identity: String,
    pub independent_verifier_identity: String,
    pub scope: BenchmarkScope,
    pub family_requirements: Vec<ThreatFamilyRequirement>,
    pub resource_channels: Vec<RedTeamResourceChannel>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdversarialBenchmarkSource {
    pub benchmark_id: String,
    pub benchmark_version: String,
    pub sealed_release_id: String,
    pub sealed_release_digest: String,
    pub hierarchical_analysis_id: String,
    pub hierarchical_analysis_digest: String,
    pub pre_registration_digest: String,
    pub split: EvidenceSplit,
    pub scope: BenchmarkScope,
    pub sealed_holdout_only: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdversarialScenario {
    pub scenario_id: String,
    pub family: ThreatFamily,
    pub kind: ScenarioKind,
    pub scenario_digest: String,
    pub generator_digest: String,
    pub author_identity: String,
    pub split: EvidenceSplit,
    /// True means the scenario or its exploit recipe was exposed during
    /// Development/Calibration or used to tune this detector.
    pub development_exposed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RedTeamResourceMeasurement {
    pub channel_id: String,
    pub unit: String,
    /// Integer base units; examples: tokens, milliseconds, microjoules.
    pub amount: u128,
    pub meter_receipt_digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdversarialTrial {
    pub trial_id: String,
    pub scenario_id: String,
    pub execution_complete: bool,
    pub execution_receipt_digest: String,
    pub attack_operator_identity: String,
    pub detector_owner_identity: String,
    pub independent_verifier_identity: String,
    pub detector_artifact_digest: String,
    pub attack_harness_digest: String,
    pub independent_verifier_artifact_digest: String,
    /// Meaningful only for Attack scenarios.
    pub exploit_succeeded: bool,
    pub detector_alerted: bool,
    pub correctly_attributed_to_family: bool,
    pub contained_before_exposure: bool,
    pub leak_occurred: bool,
    pub resource_measurements: Vec<RedTeamResourceMeasurement>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactRedTeamRate {
    pub numerator: u128,
    pub denominator: u128,
}

impl ExactRedTeamRate {
    fn new(numerator: u128, denominator: u128) -> Option<Self> {
        if denominator == 0 || numerator > denominator {
            return None;
        }
        Some(Self {
            numerator,
            denominator,
        })
    }

    fn meets_minimum_percent(&self, minimum_percent: u8) -> bool {
        self.numerator.saturating_mul(100)
            >= self.denominator.saturating_mul(u128::from(minimum_percent))
    }

    fn meets_maximum_percent(&self, maximum_percent: u8) -> bool {
        self.numerator.saturating_mul(100)
            <= self.denominator.saturating_mul(u128::from(maximum_percent))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RedTeamResourceSummary {
    pub family: ThreatFamily,
    pub channel_id: String,
    pub unit: String,
    pub scenario_count: usize,
    pub total_amount: u128,
    pub mean_amount_per_scenario_numerator: u128,
    pub mean_amount_per_scenario_denominator: u128,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThreatFamilyRedTeamResult {
    pub family: ThreatFamily,
    pub critical: bool,
    pub attack_scenario_count: usize,
    pub clean_control_count: usize,
    pub independent_attack_author_count: usize,
    pub independent_attack_generator_count: usize,
    pub independent_clean_control_author_count: usize,
    pub completed_trial_count: usize,
    pub incomplete_trial_count: usize,
    pub successful_exploit_count: usize,
    pub attack_detection_count: usize,
    pub correctly_attributed_detection_count: usize,
    pub clean_control_alert_count: usize,
    pub contained_before_exposure_count: usize,
    pub uncontained_exploit_count: usize,
    pub leaked_exploit_count: usize,
    pub exploit_success_rate: Option<ExactRedTeamRate>,
    pub successful_exploit_detection_recall: Option<ExactRedTeamRate>,
    pub clean_control_false_positive_rate: Option<ExactRedTeamRate>,
    pub successful_exploit_escape_rate: Option<ExactRedTeamRate>,
    pub minimum_attack_scenarios: usize,
    pub minimum_clean_controls: usize,
    pub minimum_successful_exploits: usize,
    pub sample_floor_passed: bool,
    pub detection_gate_passed: bool,
    pub false_positive_gate_passed: bool,
    pub containment_gate_passed: bool,
    pub family_integrity_passed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdversarialRedTeamReceipt {
    pub campaign_id: String,
    pub campaign_version: String,
    pub benchmark_id: String,
    pub benchmark_version: String,
    pub sealed_release_id: String,
    pub sealed_release_digest: String,
    pub source_hierarchical_analysis_id: String,
    pub source_hierarchical_analysis_digest: String,
    pub plan_digest: String,
    pub threat_model_digest: String,
    pub attack_catalog_digest: String,
    pub detector_artifact_digest: String,
    pub attack_harness_digest: String,
    pub independent_verifier_artifact_digest: String,
    pub split: EvidenceSplit,
    pub scope: BenchmarkScope,
    pub threat_family_count: usize,
    pub scenario_count: usize,
    pub attack_scenario_count: usize,
    pub clean_control_count: usize,
    pub completed_trial_count: usize,
    pub incomplete_trial_count: usize,
    pub successful_exploit_count: usize,
    pub leaked_exploit_count: usize,
    pub family_results: Vec<ThreatFamilyRedTeamResult>,
    pub resource_summaries: Vec<RedTeamResourceSummary>,
    pub all_required_families_covered: bool,
    pub all_scenarios_used_once: bool,
    pub all_trials_complete: bool,
    pub no_development_exposure: bool,
    pub independent_roles_separated: bool,
    pub resource_accounting_complete: bool,
    pub all_critical_families_passed: bool,
    pub all_family_gates_passed: bool,
    pub campaign_integrity_passed: bool,
    /// The first version reports exact counts and empirical rates only.
    pub empirical_rates_only: bool,
    pub confidence_intervals_computed: bool,
    /// Always false: a finite declared threat model cannot prove unhackability.
    pub universal_security_claim: bool,
    pub claim_boundary: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RedTeamError {
    InvalidPlan(String),
    InvalidIdentity(String),
    InvalidDigest(String),
    SourceMismatch(String),
    NonSealedEvidence(String),
    ThreatRegistry(String),
    ScenarioCatalog(String),
    TrialEvidence(String),
    IdentityCollision(String),
    ResourceAccounting(String),
    ArithmeticOverflow(String),
}

impl std::fmt::Display for RedTeamError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPlan(reason)
            | Self::InvalidIdentity(reason)
            | Self::InvalidDigest(reason)
            | Self::SourceMismatch(reason)
            | Self::NonSealedEvidence(reason)
            | Self::ThreatRegistry(reason)
            | Self::ScenarioCatalog(reason)
            | Self::TrialEvidence(reason)
            | Self::IdentityCollision(reason)
            | Self::ResourceAccounting(reason)
            | Self::ArithmeticOverflow(reason) => formatter.write_str(reason),
        }
    }
}

impl std::error::Error for RedTeamError {}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= MAX_ID_BYTES
}

fn require_id(value: &str, label: &str) -> Result<(), RedTeamError> {
    if !valid_id(value) {
        return Err(RedTeamError::InvalidIdentity(format!(
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

fn require_digest(value: &str, label: &str) -> Result<(), RedTeamError> {
    if !valid_digest(value) {
        return Err(RedTeamError::InvalidDigest(format!(
            "{label} is not a canonical sha256 digest"
        )));
    }
    Ok(())
}

fn validate_plan(plan: &AdversarialRedTeamPlan) -> Result<(), RedTeamError> {
    for (value, label) in [
        (&plan.campaign_id, "red-team campaign ID"),
        (&plan.campaign_version, "red-team campaign version"),
        (&plan.benchmark_id, "benchmark ID"),
        (&plan.benchmark_version, "benchmark version"),
        (&plan.sealed_release_id, "sealed release ID"),
        (
            &plan.source_hierarchical_analysis_id,
            "source hierarchy analysis ID",
        ),
        (&plan.attack_operator_identity, "attack operator identity"),
        (&plan.detector_owner_identity, "detector owner identity"),
        (
            &plan.independent_verifier_identity,
            "independent verifier identity",
        ),
    ] {
        require_id(value, label)?;
    }
    for (value, label) in [
        (&plan.sealed_release_digest, "sealed release"),
        (
            &plan.source_hierarchical_analysis_digest,
            "source hierarchy",
        ),
        (
            &plan.source_pre_registration_digest,
            "source preregistration",
        ),
        (&plan.plan_digest, "red-team plan"),
        (&plan.threat_model_digest, "threat model"),
        (&plan.attack_catalog_digest, "attack catalog"),
        (&plan.detector_artifact_digest, "detector artifact"),
        (&plan.attack_harness_digest, "attack harness"),
        (
            &plan.independent_verifier_artifact_digest,
            "independent verifier artifact",
        ),
    ] {
        require_digest(value, label)?;
    }
    if plan.attack_operator_identity == plan.detector_owner_identity
        || plan.attack_operator_identity == plan.independent_verifier_identity
        || plan.detector_owner_identity == plan.independent_verifier_identity
    {
        return Err(RedTeamError::IdentityCollision(
            "attack operator, detector owner and independent verifier must be distinct identities"
                .into(),
        ));
    }
    if plan.family_requirements.len() != ThreatFamily::ALL.len()
        || plan.family_requirements.len() > MAX_FAMILIES
    {
        return Err(RedTeamError::ThreatRegistry(
            "red-team plan must include the complete mandatory 12-family threat registry".into(),
        ));
    }
    let mut families = BTreeSet::new();
    for requirement in &plan.family_requirements {
        if !families.insert(requirement.family) {
            return Err(RedTeamError::ThreatRegistry(
                "threat family requirements must be unique".into(),
            ));
        }
        if requirement.minimum_attack_scenarios < requirement.minimum_attack_authors
            || requirement.minimum_attack_scenarios < requirement.minimum_generator_variants
            || requirement.minimum_clean_controls == 0
            || requirement.minimum_successful_exploits == 0
            || requirement.minimum_attack_authors < 2
            || requirement.minimum_generator_variants < 2
            || requirement.minimum_clean_control_authors == 0
            || requirement.minimum_correct_detection_percent > 100
            || requirement.maximum_false_positive_percent > 100
            || requirement.maximum_uncontained_exploit_percent > 100
        {
            return Err(RedTeamError::InvalidPlan(format!(
                "family {:?} needs positive attack/control/exploit floors, at least two attack authors and generator variants, and percentage thresholds in [0,100]",
                requirement.family
            )));
        }
        if requirement.critical && requirement.maximum_uncontained_exploit_percent != 0 {
            return Err(RedTeamError::InvalidPlan(format!(
                "critical family {:?} must require zero uncontained exploits",
                requirement.family
            )));
        }
    }
    if families != ThreatFamily::ALL.into_iter().collect::<BTreeSet<_>>() {
        return Err(RedTeamError::ThreatRegistry(
            "threat family registry is incomplete or contains an unknown family".into(),
        ));
    }
    if !plan
        .family_requirements
        .iter()
        .any(|requirement| requirement.critical)
    {
        return Err(RedTeamError::ThreatRegistry(
            "at least one threat family must be declared critical".into(),
        ));
    }
    if plan.resource_channels.is_empty() || plan.resource_channels.len() > MAX_RESOURCE_CHANNELS {
        return Err(RedTeamError::InvalidPlan(
            "bounded non-empty resource channel registry is required".into(),
        ));
    }
    let mut channels = BTreeSet::new();
    for channel in &plan.resource_channels {
        require_id(&channel.channel_id, "resource channel ID")?;
        require_id(&channel.unit, "resource channel unit")?;
        if !channels.insert(channel.channel_id.clone()) {
            return Err(RedTeamError::InvalidPlan(
                "resource channel IDs must be unique".into(),
            ));
        }
    }
    Ok(())
}

fn validate_source(
    plan: &AdversarialRedTeamPlan,
    source: &AdversarialBenchmarkSource,
) -> Result<(), RedTeamError> {
    if source.split != EvidenceSplit::SealedHoldout || !source.sealed_holdout_only {
        return Err(RedTeamError::NonSealedEvidence(
            "adversarial certification must operate only on a sealed holdout release".into(),
        ));
    }
    if source.benchmark_id != plan.benchmark_id
        || source.benchmark_version != plan.benchmark_version
        || source.sealed_release_id != plan.sealed_release_id
        || source.sealed_release_digest != plan.sealed_release_digest
        || source.hierarchical_analysis_id != plan.source_hierarchical_analysis_id
        || source.hierarchical_analysis_digest != plan.source_hierarchical_analysis_digest
        || source.pre_registration_digest != plan.source_pre_registration_digest
        || source.scope != plan.scope
    {
        return Err(RedTeamError::SourceMismatch(
            "red-team plan must bind the exact benchmark, sealed release, hierarchy and preregistration".into(),
        ));
    }
    Ok(())
}

fn validate_scenarios(
    plan: &AdversarialRedTeamPlan,
    scenarios: &[AdversarialScenario],
) -> Result<BTreeMap<String, &AdversarialScenario>, RedTeamError> {
    if scenarios.len() > MAX_SCENARIOS {
        return Err(RedTeamError::ScenarioCatalog(
            "scenario catalog exceeds configured cap".into(),
        ));
    }
    let mut result = BTreeMap::new();
    let mut scenario_digests = BTreeSet::new();
    for scenario in scenarios {
        require_id(&scenario.scenario_id, "scenario ID")?;
        require_id(&scenario.author_identity, "scenario author identity")?;
        require_digest(&scenario.scenario_digest, "scenario artifact")?;
        require_digest(&scenario.generator_digest, "scenario generator")?;
        if !scenario_digests.insert(scenario.scenario_digest.clone()) {
            return Err(RedTeamError::ScenarioCatalog(
                "scenario artifact digests must be unique; duplicate payloads cannot inflate attack coverage".into(),
            ));
        }
        if scenario.split != EvidenceSplit::SealedHoldout || scenario.development_exposed {
            return Err(RedTeamError::NonSealedEvidence(format!(
                "scenario {} was not kept exclusively in the sealed holdout",
                scenario.scenario_id
            )));
        }
        if scenario.author_identity == plan.detector_owner_identity
            || scenario.author_identity == plan.independent_verifier_identity
            || scenario.generator_digest == plan.detector_artifact_digest
            || scenario.generator_digest == plan.independent_verifier_artifact_digest
        {
            return Err(RedTeamError::IdentityCollision(format!(
                "scenario {} shares authorship or artifact identity with the detector/verifier",
                scenario.scenario_id
            )));
        }
        if result
            .insert(scenario.scenario_id.clone(), scenario)
            .is_some()
        {
            return Err(RedTeamError::ScenarioCatalog(format!(
                "duplicate scenario ID {}",
                scenario.scenario_id
            )));
        }
    }
    let mut expected_family_counts = BTreeMap::<ThreatFamily, (usize, usize)>::new();
    for scenario in scenarios {
        let counts = expected_family_counts.entry(scenario.family).or_default();
        match scenario.kind {
            ScenarioKind::Attack => {
                counts.0 = counts.0.checked_add(1).ok_or_else(|| {
                    RedTeamError::ArithmeticOverflow("attack scenario count overflow".into())
                })?
            }
            ScenarioKind::CleanNegativeControl => {
                counts.1 = counts.1.checked_add(1).ok_or_else(|| {
                    RedTeamError::ArithmeticOverflow("clean control count overflow".into())
                })?
            }
        }
    }
    for requirement in &plan.family_requirements {
        let (attacks, controls) = expected_family_counts
            .get(&requirement.family)
            .copied()
            .unwrap_or_default();
        if attacks < requirement.minimum_attack_scenarios
            || controls < requirement.minimum_clean_controls
        {
            return Err(RedTeamError::ScenarioCatalog(format!(
                "family {:?} has {attacks} attack scenarios and {controls} clean controls; preregistered floors are {} and {}",
                requirement.family,
                requirement.minimum_attack_scenarios,
                requirement.minimum_clean_controls
            )));
        }
    }
    Ok(result)
}

fn validate_trial_resource_measurements(
    plan: &AdversarialRedTeamPlan,
    trial: &AdversarialTrial,
) -> Result<Vec<RedTeamResourceMeasurement>, RedTeamError> {
    if trial.resource_measurements.len() != plan.resource_channels.len() {
        return Err(RedTeamError::ResourceAccounting(format!(
            "trial {} must meter every preregistered resource channel exactly once",
            trial.trial_id
        )));
    }
    let expected = plan
        .resource_channels
        .iter()
        .map(|channel| (channel.channel_id.as_str(), channel.unit.as_str()))
        .collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::new();
    for measurement in &trial.resource_measurements {
        require_id(&measurement.channel_id, "resource measurement channel")?;
        require_id(&measurement.unit, "resource measurement unit")?;
        require_digest(&measurement.meter_receipt_digest, "resource meter receipt")?;
        if !expected.contains(&(measurement.channel_id.as_str(), measurement.unit.as_str()))
            || !seen.insert(measurement.channel_id.clone())
        {
            return Err(RedTeamError::ResourceAccounting(format!(
                "trial {} has an unregistered, unit-mismatched or duplicate resource channel",
                trial.trial_id
            )));
        }
    }
    Ok(trial.resource_measurements.clone())
}

fn rate(numerator: usize, denominator: usize) -> Option<ExactRedTeamRate> {
    ExactRedTeamRate::new(
        u128::try_from(numerator).ok()?,
        u128::try_from(denominator).ok()?,
    )
}

#[derive(Default)]
struct FamilyAccumulator {
    attacks: usize,
    controls: usize,
    attack_authors: BTreeSet<String>,
    attack_generators: BTreeSet<String>,
    clean_authors: BTreeSet<String>,
    completed: usize,
    incomplete: usize,
    successful_exploits: usize,
    attack_detected: usize,
    correctly_detected_exploits: usize,
    clean_alerts: usize,
    contained: usize,
    uncontained: usize,
    leaked: usize,
}

pub fn evaluate_adversarial_redteam_campaign(
    plan: &AdversarialRedTeamPlan,
    source: &AdversarialBenchmarkSource,
    scenarios: &[AdversarialScenario],
    trials: &[AdversarialTrial],
) -> Result<AdversarialRedTeamReceipt, RedTeamError> {
    validate_plan(plan)?;
    validate_source(plan, source)?;
    let scenario_map = validate_scenarios(plan, scenarios)?;
    if trials.len() != scenarios.len() || trials.len() > MAX_TRIALS {
        return Err(RedTeamError::TrialEvidence(
            "every preregistered scenario must have exactly one trial row".into(),
        ));
    }
    let mut seen_scenarios = BTreeSet::new();
    let mut seen_trial_ids = BTreeSet::new();
    let mut accumulators = BTreeMap::<ThreatFamily, FamilyAccumulator>::new();
    let mut completed_trial_count = 0usize;
    let mut incomplete_trial_count = 0usize;
    let mut total_attack_count = 0usize;
    let mut total_control_count = 0usize;
    let mut total_successful_exploits = 0usize;
    let mut total_leaked_exploits = 0usize;
    let mut all_resources = BTreeMap::<(ThreatFamily, String), (String, u128, usize)>::new();

    for trial in trials {
        require_id(&trial.trial_id, "red-team trial ID")?;
        require_id(&trial.scenario_id, "red-team trial scenario ID")?;
        require_digest(&trial.execution_receipt_digest, "attack execution receipt")?;
        require_digest(&trial.detector_artifact_digest, "trial detector artifact")?;
        require_digest(&trial.attack_harness_digest, "trial attack harness")?;
        require_digest(
            &trial.independent_verifier_artifact_digest,
            "trial independent verifier",
        )?;
        if !seen_trial_ids.insert(trial.trial_id.clone())
            || !seen_scenarios.insert(trial.scenario_id.clone())
        {
            return Err(RedTeamError::TrialEvidence(
                "trial IDs and scenario IDs must each be unique".into(),
            ));
        }
        let scenario = scenario_map.get(&trial.scenario_id).ok_or_else(|| {
            RedTeamError::TrialEvidence(format!(
                "trial {} references an unknown scenario",
                trial.trial_id
            ))
        })?;
        if trial.attack_operator_identity != plan.attack_operator_identity
            || trial.detector_owner_identity != plan.detector_owner_identity
            || trial.independent_verifier_identity != plan.independent_verifier_identity
            || trial.detector_artifact_digest != plan.detector_artifact_digest
            || trial.attack_harness_digest != plan.attack_harness_digest
            || trial.independent_verifier_artifact_digest
                != plan.independent_verifier_artifact_digest
        {
            return Err(RedTeamError::IdentityCollision(format!(
                "trial {} does not bind the preregistered operator/detector/verifier identities",
                trial.trial_id
            )));
        }
        let resources = validate_trial_resource_measurements(plan, trial)?;
        let accumulator = accumulators.entry(scenario.family).or_default();
        match scenario.kind {
            ScenarioKind::Attack => {
                accumulator.attacks += 1;
                total_attack_count += 1;
                accumulator
                    .attack_authors
                    .insert(scenario.author_identity.clone());
                accumulator
                    .attack_generators
                    .insert(scenario.generator_digest.clone());
                if trial.leak_occurred && !trial.exploit_succeeded {
                    return Err(RedTeamError::TrialEvidence(
                        "leak occurrence without a successful exploit is inconsistent".into(),
                    ));
                }
                if trial.contained_before_exposure && trial.leak_occurred {
                    return Err(RedTeamError::TrialEvidence(
                        "a contained-before-exposure exploit cannot also record a leak".into(),
                    ));
                }
                if trial.correctly_attributed_to_family && !trial.detector_alerted {
                    return Err(RedTeamError::TrialEvidence(
                        "correct attribution cannot be true when the detector did not alert".into(),
                    ));
                }
                if trial.exploit_succeeded {
                    accumulator.successful_exploits += 1;
                    total_successful_exploits += 1;
                    if trial.detector_alerted && trial.correctly_attributed_to_family {
                        accumulator.correctly_detected_exploits += 1;
                    }
                    if trial.contained_before_exposure && !trial.leak_occurred {
                        accumulator.contained += 1;
                    } else {
                        accumulator.uncontained += 1;
                    }
                    if trial.leak_occurred {
                        accumulator.leaked += 1;
                        total_leaked_exploits += 1;
                    }
                }
                if trial.detector_alerted {
                    accumulator.attack_detected += 1;
                }
            }
            ScenarioKind::CleanNegativeControl => {
                accumulator.controls += 1;
                total_control_count += 1;
                accumulator
                    .clean_authors
                    .insert(scenario.author_identity.clone());
                if trial.exploit_succeeded || trial.contained_before_exposure || trial.leak_occurred
                {
                    return Err(RedTeamError::TrialEvidence(
                        "clean negative control cannot report exploit, containment or leakage"
                            .into(),
                    ));
                }
                if trial.detector_alerted || trial.correctly_attributed_to_family {
                    accumulator.clean_alerts += 1;
                }
            }
        }
        if trial.execution_complete {
            accumulator.completed += 1;
            completed_trial_count += 1;
        } else {
            if trial.exploit_succeeded
                || trial.detector_alerted
                || trial.contained_before_exposure
                || trial.leak_occurred
            {
                return Err(RedTeamError::TrialEvidence(
                    "incomplete trial cannot claim exploit, detector, containment or leakage outcomes".into(),
                ));
            }
            accumulator.incomplete += 1;
            incomplete_trial_count += 1;
        }
        for measurement in resources {
            let total = all_resources
                .entry((scenario.family, measurement.channel_id.clone()))
                .or_insert_with(|| (measurement.unit.clone(), 0, 0));
            if total.0 != measurement.unit {
                return Err(RedTeamError::ResourceAccounting(
                    "resource unit changed within a threat family/channel".into(),
                ));
            }
            total.1 = total.1.checked_add(measurement.amount).ok_or_else(|| {
                RedTeamError::ArithmeticOverflow("red-team resource total overflow".into())
            })?;
            total.2 += 1;
        }
    }
    if seen_scenarios != scenario_map.keys().cloned().collect::<BTreeSet<_>>() {
        return Err(RedTeamError::TrialEvidence(
            "scenario catalog and trial evidence are not bijective".into(),
        ));
    }

    let mut family_results = Vec::with_capacity(plan.family_requirements.len());
    let mut resource_summaries = Vec::new();
    for requirement in &plan.family_requirements {
        let accumulator = accumulators.remove(&requirement.family).unwrap_or_default();
        let recall = rate(
            accumulator.correctly_detected_exploits,
            accumulator.successful_exploits,
        );
        let false_positive = rate(accumulator.clean_alerts, accumulator.controls);
        let escape = rate(accumulator.uncontained, accumulator.successful_exploits);
        let exploit_success_rate = rate(accumulator.successful_exploits, accumulator.attacks);
        let sample_floor_passed = accumulator.attacks >= requirement.minimum_attack_scenarios
            && accumulator.controls >= requirement.minimum_clean_controls
            && accumulator.successful_exploits >= requirement.minimum_successful_exploits
            && accumulator.attack_authors.len() >= requirement.minimum_attack_authors
            && accumulator.attack_generators.len() >= requirement.minimum_generator_variants
            && accumulator.clean_authors.len() >= requirement.minimum_clean_control_authors
            && accumulator.incomplete == 0;
        let detection_gate_passed = recall.as_ref().is_some_and(|value| {
            value.meets_minimum_percent(requirement.minimum_correct_detection_percent)
        });
        let false_positive_gate_passed = false_positive.as_ref().is_some_and(|value| {
            value.meets_maximum_percent(requirement.maximum_false_positive_percent)
        });
        let containment_gate_passed = escape.as_ref().is_some_and(|value| {
            value.meets_maximum_percent(requirement.maximum_uncontained_exploit_percent)
        });
        let family_integrity_passed = sample_floor_passed
            && detection_gate_passed
            && false_positive_gate_passed
            && containment_gate_passed
            && accumulator.leaked == 0;
        for channel in &plan.resource_channels {
            let (unit, total_amount, scenario_count) = all_resources
                .get(&(requirement.family, channel.channel_id.clone()))
                .cloned()
                .unwrap_or((channel.unit.clone(), 0, 0));
            resource_summaries.push(RedTeamResourceSummary {
                family: requirement.family,
                channel_id: channel.channel_id.clone(),
                unit,
                scenario_count,
                total_amount,
                mean_amount_per_scenario_numerator: total_amount,
                mean_amount_per_scenario_denominator: u128::try_from(scenario_count).map_err(
                    |_| {
                        RedTeamError::ArithmeticOverflow(
                            "resource scenario count exceeds u128".into(),
                        )
                    },
                )?,
            });
        }
        family_results.push(ThreatFamilyRedTeamResult {
            family: requirement.family,
            critical: requirement.critical,
            attack_scenario_count: accumulator.attacks,
            clean_control_count: accumulator.controls,
            independent_attack_author_count: accumulator.attack_authors.len(),
            independent_attack_generator_count: accumulator.attack_generators.len(),
            independent_clean_control_author_count: accumulator.clean_authors.len(),
            completed_trial_count: accumulator.completed,
            incomplete_trial_count: accumulator.incomplete,
            successful_exploit_count: accumulator.successful_exploits,
            attack_detection_count: accumulator.attack_detected,
            correctly_attributed_detection_count: accumulator.correctly_detected_exploits,
            clean_control_alert_count: accumulator.clean_alerts,
            contained_before_exposure_count: accumulator.contained,
            uncontained_exploit_count: accumulator.uncontained,
            leaked_exploit_count: accumulator.leaked,
            exploit_success_rate,
            successful_exploit_detection_recall: recall,
            clean_control_false_positive_rate: false_positive,
            successful_exploit_escape_rate: escape,
            minimum_attack_scenarios: requirement.minimum_attack_scenarios,
            minimum_clean_controls: requirement.minimum_clean_controls,
            minimum_successful_exploits: requirement.minimum_successful_exploits,
            sample_floor_passed,
            detection_gate_passed,
            false_positive_gate_passed,
            containment_gate_passed,
            family_integrity_passed,
        });
    }
    let all_critical_families_passed = family_results
        .iter()
        .filter(|result| result.critical)
        .all(|result| result.family_integrity_passed);
    let all_family_gates_passed = family_results
        .iter()
        .all(|result| result.family_integrity_passed);
    let all_scenarios_used_once = seen_scenarios.len() == scenario_map.len();
    let all_trials_complete = incomplete_trial_count == 0 && completed_trial_count == trials.len();
    let no_development_exposure = scenarios
        .iter()
        .all(|scenario| !scenario.development_exposed);
    let independent_roles_separated = plan.attack_operator_identity != plan.detector_owner_identity
        && plan.attack_operator_identity != plan.independent_verifier_identity
        && plan.detector_owner_identity != plan.independent_verifier_identity;
    let resource_accounting_complete = resource_summaries.iter().all(|summary| {
        summary.scenario_count
            == scenarios
                .iter()
                .filter(|scenario| scenario.family == summary.family)
                .count()
    });
    let all_required_families_covered = family_results.len() == ThreatFamily::ALL.len()
        && family_results
            .iter()
            .all(|result| result.sample_floor_passed);
    let campaign_integrity_passed = all_required_families_covered
        && all_scenarios_used_once
        && all_trials_complete
        && no_development_exposure
        && independent_roles_separated
        && resource_accounting_complete
        && all_critical_families_passed
        && all_family_gates_passed
        && total_leaked_exploits == 0;

    Ok(AdversarialRedTeamReceipt {
        campaign_id: plan.campaign_id.clone(),
        campaign_version: plan.campaign_version.clone(),
        benchmark_id: plan.benchmark_id.clone(),
        benchmark_version: plan.benchmark_version.clone(),
        sealed_release_id: plan.sealed_release_id.clone(),
        sealed_release_digest: plan.sealed_release_digest.clone(),
        source_hierarchical_analysis_id: plan.source_hierarchical_analysis_id.clone(),
        source_hierarchical_analysis_digest: plan.source_hierarchical_analysis_digest.clone(),
        plan_digest: plan.plan_digest.clone(),
        threat_model_digest: plan.threat_model_digest.clone(),
        attack_catalog_digest: plan.attack_catalog_digest.clone(),
        detector_artifact_digest: plan.detector_artifact_digest.clone(),
        attack_harness_digest: plan.attack_harness_digest.clone(),
        independent_verifier_artifact_digest: plan.independent_verifier_artifact_digest.clone(),
        split: EvidenceSplit::SealedHoldout,
        scope: plan.scope,
        threat_family_count: family_results.len(),
        scenario_count: scenarios.len(),
        attack_scenario_count: total_attack_count,
        clean_control_count: total_control_count,
        completed_trial_count,
        incomplete_trial_count,
        successful_exploit_count: total_successful_exploits,
        leaked_exploit_count: total_leaked_exploits,
        family_results,
        resource_summaries,
        all_required_families_covered,
        all_scenarios_used_once,
        all_trials_complete,
        no_development_exposure,
        independent_roles_separated,
        resource_accounting_complete,
        all_critical_families_passed,
        all_family_gates_passed,
        campaign_integrity_passed,
        empirical_rates_only: true,
        confidence_intervals_computed: false,
        universal_security_claim: false,
        claim_boundary: "Passed only for the declared threat model, catalog, detector, environment, sealed release and resource budget. Rates are empirical point estimates without confidence intervals. This is not a proof of unhackability or absence of unknown attacks.".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::super::counterexample_ledger::EvidenceSplit;
    use super::super::matched_benchmark::BenchmarkScope;
    use super::{
        AdversarialBenchmarkSource, AdversarialRedTeamPlan, AdversarialScenario, AdversarialTrial,
        RedTeamResourceChannel, RedTeamResourceMeasurement, ScenarioKind, ThreatFamily,
        ThreatFamilyRequirement, evaluate_adversarial_redteam_campaign,
    };

    const D1: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const D2: &str = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const D3: &str = "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const D4: &str = "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
    const D5: &str = "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";

    fn plan() -> AdversarialRedTeamPlan {
        AdversarialRedTeamPlan {
            campaign_id: "red-team-1".into(),
            campaign_version: "v1".into(),
            benchmark_id: "science-benchmark".into(),
            benchmark_version: "v1".into(),
            sealed_release_id: "release-1".into(),
            sealed_release_digest: D1.into(),
            source_hierarchical_analysis_id: "hierarchy-1".into(),
            source_hierarchical_analysis_digest: D2.into(),
            source_pre_registration_digest: D3.into(),
            plan_digest: D4.into(),
            threat_model_digest: D5.into(),
            attack_catalog_digest: D1.into(),
            detector_artifact_digest: D2.into(),
            attack_harness_digest: D3.into(),
            independent_verifier_artifact_digest: D4.into(),
            attack_operator_identity: "red-team-operator".into(),
            detector_owner_identity: "detector-owner".into(),
            independent_verifier_identity: "independent-verifier".into(),
            scope: BenchmarkScope::FullScientificCampaign,
            family_requirements: ThreatFamily::ALL
                .into_iter()
                .map(|family| ThreatFamilyRequirement {
                    family,
                    critical: true,
                    minimum_attack_scenarios: 2,
                    minimum_clean_controls: 1,
                    minimum_successful_exploits: 1,
                    minimum_attack_authors: 2,
                    minimum_generator_variants: 2,
                    minimum_clean_control_authors: 1,
                    minimum_correct_detection_percent: 100,
                    maximum_false_positive_percent: 0,
                    maximum_uncontained_exploit_percent: 0,
                })
                .collect(),
            resource_channels: vec![
                RedTeamResourceChannel {
                    channel_id: "wall-ms".into(),
                    unit: "ms".into(),
                },
                RedTeamResourceChannel {
                    channel_id: "tool-calls".into(),
                    unit: "calls".into(),
                },
            ],
        }
    }

    fn source() -> AdversarialBenchmarkSource {
        AdversarialBenchmarkSource {
            benchmark_id: "science-benchmark".into(),
            benchmark_version: "v1".into(),
            sealed_release_id: "release-1".into(),
            sealed_release_digest: D1.into(),
            hierarchical_analysis_id: "hierarchy-1".into(),
            hierarchical_analysis_digest: D2.into(),
            pre_registration_digest: D3.into(),
            split: EvidenceSplit::SealedHoldout,
            scope: BenchmarkScope::FullScientificCampaign,
            sealed_holdout_only: true,
        }
    }

    fn scenario(index: usize, kind: ScenarioKind, variant: usize) -> AdversarialScenario {
        let family = ThreatFamily::ALL[index];
        let artifact_ordinal = index * 3
            + match kind {
                ScenarioKind::Attack => variant + 1,
                ScenarioKind::CleanNegativeControl => 3,
            };
        let author = match kind {
            ScenarioKind::Attack => format!("attack-author-{index}-{variant}"),
            ScenarioKind::CleanNegativeControl => format!("control-author-{index}"),
        };
        AdversarialScenario {
            scenario_id: format!(
                "{family:?}-{}-{variant}",
                match kind {
                    ScenarioKind::Attack => "attack",
                    ScenarioKind::CleanNegativeControl => "clean",
                }
            ),
            family,
            kind,
            scenario_digest: format!("sha256:{artifact_ordinal:064x}"),
            generator_digest: match (kind, variant) {
                (ScenarioKind::Attack, 0) => D5.into(),
                (ScenarioKind::Attack, _) => D6.into(),
                (ScenarioKind::CleanNegativeControl, _) => D1.into(),
            },
            author_identity: author,
            split: EvidenceSplit::SealedHoldout,
            development_exposed: false,
        }
    }

    fn trial(scenario: &AdversarialScenario) -> AdversarialTrial {
        AdversarialTrial {
            trial_id: format!("trial-{}", scenario.scenario_id),
            scenario_id: scenario.scenario_id.clone(),
            execution_complete: true,
            execution_receipt_digest: D1.into(),
            attack_operator_identity: "red-team-operator".into(),
            detector_owner_identity: "detector-owner".into(),
            independent_verifier_identity: "independent-verifier".into(),
            detector_artifact_digest: D2.into(),
            attack_harness_digest: D3.into(),
            independent_verifier_artifact_digest: D4.into(),
            exploit_succeeded: scenario.kind == ScenarioKind::Attack,
            detector_alerted: scenario.kind == ScenarioKind::Attack,
            correctly_attributed_to_family: scenario.kind == ScenarioKind::Attack,
            contained_before_exposure: scenario.kind == ScenarioKind::Attack,
            leak_occurred: false,
            resource_measurements: vec![
                RedTeamResourceMeasurement {
                    channel_id: "wall-ms".into(),
                    unit: "ms".into(),
                    amount: 100,
                    meter_receipt_digest: D1.into(),
                },
                RedTeamResourceMeasurement {
                    channel_id: "tool-calls".into(),
                    unit: "calls".into(),
                    amount: 2,
                    meter_receipt_digest: D2.into(),
                },
            ],
        }
    }

    fn campaign() -> (Vec<AdversarialScenario>, Vec<AdversarialTrial>) {
        let mut scenarios = Vec::new();
        for index in 0..ThreatFamily::ALL.len() {
            scenarios.push(scenario(index, ScenarioKind::Attack, 0));
            scenarios.push(scenario(index, ScenarioKind::Attack, 1));
            scenarios.push(scenario(index, ScenarioKind::CleanNegativeControl, 0));
        }
        let trials = scenarios.iter().map(trial).collect();
        (scenarios, trials)
    }

    #[test]
    fn complete_campaign_separates_attack_recall_false_positives_and_containment() {
        let (scenarios, trials) = campaign();
        let receipt =
            evaluate_adversarial_redteam_campaign(&plan(), &source(), &scenarios, &trials).unwrap();
        assert!(receipt.campaign_integrity_passed);
        assert!(receipt.empirical_rates_only);
        assert!(!receipt.confidence_intervals_computed);
        assert!(!receipt.universal_security_claim);
        assert_eq!(receipt.threat_family_count, 12);
        assert_eq!(receipt.attack_scenario_count, 24);
        assert_eq!(receipt.clean_control_count, 12);
        assert_eq!(receipt.successful_exploit_count, 24);
        assert_eq!(receipt.leaked_exploit_count, 0);
        let family = &receipt.family_results[0];
        assert_eq!(
            family
                .successful_exploit_detection_recall
                .as_ref()
                .unwrap()
                .numerator,
            1
        );
        assert_eq!(
            family
                .clean_control_false_positive_rate
                .as_ref()
                .unwrap()
                .numerator,
            0
        );
        assert_eq!(
            family
                .successful_exploit_escape_rate
                .as_ref()
                .unwrap()
                .numerator,
            0
        );
    }

    #[test]
    fn detector_miss_fails_family_recall_even_when_exploit_is_contained() {
        let (scenarios, mut trials) = campaign();
        let attack_id = &scenarios[0].scenario_id;
        let trial = trials
            .iter_mut()
            .find(|trial| &trial.scenario_id == attack_id)
            .unwrap();
        trial.detector_alerted = false;
        trial.correctly_attributed_to_family = false;
        let receipt =
            evaluate_adversarial_redteam_campaign(&plan(), &source(), &scenarios, &trials).unwrap();
        assert!(!receipt.campaign_integrity_passed);
        assert!(!receipt.family_results[0].detection_gate_passed);
    }

    #[test]
    fn false_positive_on_clean_control_fails_specificity_gate() {
        let (scenarios, mut trials) = campaign();
        let clean_id = &scenarios[2].scenario_id;
        trials
            .iter_mut()
            .find(|trial| &trial.scenario_id == clean_id)
            .unwrap()
            .detector_alerted = true;
        let receipt =
            evaluate_adversarial_redteam_campaign(&plan(), &source(), &scenarios, &trials).unwrap();
        assert!(!receipt.campaign_integrity_passed);
        assert_eq!(receipt.family_results[0].clean_control_alert_count, 1);
        assert!(!receipt.family_results[0].false_positive_gate_passed);
    }

    #[test]
    fn detected_after_leak_is_not_containment() {
        let (scenarios, mut trials) = campaign();
        let attack_id = &scenarios[0].scenario_id;
        let trial = trials
            .iter_mut()
            .find(|trial| &trial.scenario_id == attack_id)
            .unwrap();
        trial.leak_occurred = true;
        trial.contained_before_exposure = false;
        let receipt =
            evaluate_adversarial_redteam_campaign(&plan(), &source(), &scenarios, &trials).unwrap();
        assert_eq!(receipt.leaked_exploit_count, 1);
        assert_eq!(receipt.family_results[0].uncontained_exploit_count, 1);
        assert!(!receipt.campaign_integrity_passed);
    }

    #[test]
    fn development_exposed_attack_recipe_is_rejected() {
        let (mut scenarios, trials) = campaign();
        scenarios[0].development_exposed = true;
        assert!(
            evaluate_adversarial_redteam_campaign(&plan(), &source(), &scenarios, &trials).is_err()
        );
    }

    #[test]
    fn missing_trial_or_duplicate_trial_cannot_shrink_denominator() {
        let (scenarios, mut trials) = campaign();
        trials.pop();
        assert!(
            evaluate_adversarial_redteam_campaign(&plan(), &source(), &scenarios, &trials).is_err()
        );
        let (scenarios, mut trials) = campaign();
        trials[1].trial_id = trials[0].trial_id.clone();
        assert!(
            evaluate_adversarial_redteam_campaign(&plan(), &source(), &scenarios, &trials).is_err()
        );
    }

    #[test]
    fn duplicate_scenario_artifacts_cannot_inflate_independent_attack_coverage() {
        let (mut scenarios, trials) = campaign();
        scenarios[1].scenario_digest = scenarios[0].scenario_digest.clone();
        assert!(
            evaluate_adversarial_redteam_campaign(&plan(), &source(), &scenarios, &trials).is_err()
        );
    }

    #[test]
    fn fewer_than_two_independent_attack_authors_or_generators_rejects() {
        let (mut scenarios, trials) = campaign();
        scenarios[1].author_identity = scenarios[0].author_identity.clone();
        assert!(
            !evaluate_adversarial_redteam_campaign(&plan(), &source(), &scenarios, &trials)
                .unwrap()
                .family_results[0]
                .sample_floor_passed
        );

        let (mut scenarios, trials) = campaign();
        scenarios[1].generator_digest = scenarios[0].generator_digest.clone();
        let receipt =
            evaluate_adversarial_redteam_campaign(&plan(), &source(), &scenarios, &trials).unwrap();
        assert!(!receipt.family_results[0].sample_floor_passed);
        assert!(!receipt.campaign_integrity_passed);
    }

    #[test]
    fn operator_detector_and_verifier_must_be_independent() {
        let (scenarios, trials) = campaign();
        let mut plan = plan();
        plan.independent_verifier_identity = plan.detector_owner_identity.clone();
        assert!(
            evaluate_adversarial_redteam_campaign(&plan, &source(), &scenarios, &trials).is_err()
        );
    }

    #[test]
    fn missing_metered_resource_channel_rejects_cost_integrity_claim() {
        let (scenarios, mut trials) = campaign();
        trials[0].resource_measurements.pop();
        assert!(
            evaluate_adversarial_redteam_campaign(&plan(), &source(), &scenarios, &trials).is_err()
        );
    }

    #[test]
    fn incomplete_attack_attempt_remains_in_denominator_and_blocks_certification() {
        let (scenarios, mut trials) = campaign();
        trials[0].execution_complete = false;
        trials[0].exploit_succeeded = false;
        trials[0].detector_alerted = false;
        trials[0].correctly_attributed_to_family = false;
        trials[0].contained_before_exposure = false;
        let receipt =
            evaluate_adversarial_redteam_campaign(&plan(), &source(), &scenarios, &trials).unwrap();
        assert_eq!(receipt.incomplete_trial_count, 1);
        assert!(!receipt.all_trials_complete);
        assert!(!receipt.campaign_integrity_passed);
    }
}
