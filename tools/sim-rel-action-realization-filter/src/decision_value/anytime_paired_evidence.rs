//! Anytime-valid paired benchmark evidence using an exact mixture e-process.
//!
//! Each mechanism family has a preregistered conditional sign-symmetry null.
//! Discordant paired wins/losses form a fair Bernoulli sign sequence under
//! that null; ties are reported but do not update the process. A fixed
//! Uniform[1/2,1] mixture over one-sided alternatives yields
//!
//!   E(w,l) = integral_0^1 (1+u)^w (1-u)^l du.
//!
//! This is a nonnegative martingale under the fair-sign null. Ville's
//! inequality gives an anytime-valid crossing rule. A fixed Bonferroni
//! allocation alpha/m across the complete preregistered family set controls
//! family-wise error without cross-family independence.

use super::counterexample_ledger::EvidenceSplit;
use super::matched_benchmark::{
    BenchmarkScope, BenchmarkTaskValidity, MatchedBenchmarkManifest, PairedBenchmarkTrial,
    validate_and_summarize_matched_campaign,
};
use crate::Rational;
use std::collections::{BTreeMap, BTreeSet};

const MAX_ID_BYTES: usize = 256;
const MAX_EVENTS: usize = 100_000;
const MAX_FAMILIES: usize = 128;
pub const MAX_ANYTIME_DISCORDANT_PAIRS: usize = 40;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreregisteredAnytimeAnalysisPlan {
    pub analysis_id: String,
    pub analysis_version: String,
    pub campaign_id: String,
    pub scope: BenchmarkScope,
    pub group_id: String,
    pub agent_panel_id: String,
    pub task_manifest_digest: String,
    pub model_configuration_digest: String,
    pub prompt_configuration_digest: String,
    pub harness_digest: String,
    pub toolset_digest: String,
    pub reference_arm_id: String,
    pub candidate_arm_id: String,
    pub pre_registration_receipt_digest: String,
    pub family_wise_alpha: Rational,
    pub maximum_terminal_loss_units: u64,
    pub minimum_discordant_pairs_before_claim: usize,
    /// Complete mechanism-family set fixed before the sealed stream is opened.
    pub mechanism_families: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SequencedPairedBenchmarkEvent {
    pub event_id: String,
    pub sequence: u64,
    pub trial: PairedBenchmarkTrial,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FamilyAnytimeEvidence {
    pub mechanism_family_id: String,
    pub total_pairs: usize,
    pub invalid_tasks_excluded: usize,
    pub candidate_wins: usize,
    pub reference_wins: usize,
    pub ties: usize,
    pub discordant_pairs: usize,
    pub current_e_value: Rational,
    pub allocated_alpha: Rational,
    pub critical_e_value: Rational,
    /// First qualifying threshold crossing. Later observations never erase it.
    pub first_crossing_sequence: Option<u64>,
    pub crossed_anytime_threshold: bool,
    pub maximum_supported_discordant_pairs: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnytimePairedInferenceReceipt {
    pub analysis_id: String,
    pub analysis_version: String,
    pub pre_registration_receipt_digest: String,
    pub campaign_id: String,
    pub scope: BenchmarkScope,
    pub split: EvidenceSplit,
    pub group_id: String,
    pub reference_arm_id: String,
    pub candidate_arm_id: String,
    pub family_wise_alpha: Rational,
    pub family_count: usize,
    pub event_count: usize,
    pub family_results: Vec<FamilyAnytimeEvidence>,
    pub sealed_holdout_only: bool,
    pub complete_preregistered_family_set_retained: bool,
    pub bonferroni_anytime_fwer_control_applied: bool,
    pub cross_family_independence_required: bool,
    pub conditional_sign_symmetry_assumption: bool,
    pub optional_stopping_supported: bool,
    pub exact_rational_e_process: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AnytimePairedError {
    InvalidPlan(String),
    ManifestMismatch(String),
    NonSealedEvidence(String),
    UnregisteredFamily(String),
    EventSequence(String),
    DuplicateIdentity(String),
    LossOutsidePreregisteredRange(String),
    UnsupportedExactSampleSize(String),
    ArithmeticOverflow(String),
    InvalidMatchedCampaign(String),
}

impl std::fmt::Display for AnytimePairedError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPlan(reason)
            | Self::ManifestMismatch(reason)
            | Self::NonSealedEvidence(reason)
            | Self::UnregisteredFamily(reason)
            | Self::EventSequence(reason)
            | Self::DuplicateIdentity(reason)
            | Self::LossOutsidePreregisteredRange(reason)
            | Self::UnsupportedExactSampleSize(reason)
            | Self::ArithmeticOverflow(reason)
            | Self::InvalidMatchedCampaign(reason) => formatter.write_str(reason),
        }
    }
}

impl std::error::Error for AnytimePairedError {}

#[derive(Clone, Debug, Default)]
struct FamilyCounts {
    total_pairs: usize,
    invalid_tasks_excluded: usize,
    candidate_wins: usize,
    reference_wins: usize,
    ties: usize,
    first_crossing_sequence: Option<u64>,
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= MAX_ID_BYTES
}

fn validate_plan(plan: &PreregisteredAnytimeAnalysisPlan) -> Result<(), AnytimePairedError> {
    for (value, label) in [
        (&plan.analysis_id, "analysis ID"),
        (&plan.analysis_version, "analysis version"),
        (&plan.campaign_id, "campaign ID"),
        (&plan.group_id, "sealed group ID"),
        (&plan.agent_panel_id, "agent panel ID"),
        (&plan.task_manifest_digest, "task manifest digest"),
        (&plan.model_configuration_digest, "model configuration digest"),
        (&plan.prompt_configuration_digest, "prompt configuration digest"),
        (&plan.harness_digest, "harness digest"),
        (&plan.toolset_digest, "toolset digest"),
        (&plan.reference_arm_id, "reference arm ID"),
        (&plan.candidate_arm_id, "candidate arm ID"),
        (&plan.pre_registration_receipt_digest, "pre-registration receipt digest"),
    ] {
        if !valid_id(value) {
            return Err(AnytimePairedError::InvalidPlan(format!(
                "{label} is empty or overlong"
            )));
        }
    }
    if plan.reference_arm_id == plan.candidate_arm_id {
        return Err(AnytimePairedError::InvalidPlan(
            "reference and candidate arm IDs must differ".into(),
        ));
    }
    if plan.family_wise_alpha.is_zero() || !plan.family_wise_alpha.is_probability() {
        return Err(AnytimePairedError::InvalidPlan(
            "family-wise alpha must be in (0,1]".into(),
        ));
    }
    if plan.maximum_terminal_loss_units == 0 {
        return Err(AnytimePairedError::InvalidPlan(
            "maximum terminal-loss range must be positive".into(),
        ));
    }
    if !(1..=MAX_ANYTIME_DISCORDANT_PAIRS)
        .contains(&plan.minimum_discordant_pairs_before_claim)
    {
        return Err(AnytimePairedError::InvalidPlan(format!(
            "minimum discordant pairs must be in 1..={MAX_ANYTIME_DISCORDANT_PAIRS}"
        )));
    }
    if plan.mechanism_families.is_empty() || plan.mechanism_families.len() > MAX_FAMILIES {
        return Err(AnytimePairedError::InvalidPlan(format!(
            "preregistered family set must contain 1..={MAX_FAMILIES} families"
        )));
    }
    let mut unique = BTreeSet::new();
    for family in &plan.mechanism_families {
        if !valid_id(family) || !unique.insert(family.clone()) {
            return Err(AnytimePairedError::InvalidPlan(
                "family set contains invalid or duplicate identities".into(),
            ));
        }
    }
    Ok(())
}

fn validate_manifest_binding(
    plan: &PreregisteredAnytimeAnalysisPlan,
    manifest: &MatchedBenchmarkManifest,
) -> Result<(), AnytimePairedError> {
    if manifest.split != EvidenceSplit::SealedHoldout {
        return Err(AnytimePairedError::NonSealedEvidence(
            "anytime final claims accept SealedHoldout campaigns only".into(),
        ));
    }
    if plan.campaign_id != manifest.campaign_id
        || plan.scope != manifest.scope
        || plan.group_id != manifest.group_id
        || plan.agent_panel_id != manifest.agent_panel_id
        || plan.task_manifest_digest != manifest.task_manifest_digest
        || plan.model_configuration_digest != manifest.model_configuration_digest
        || plan.prompt_configuration_digest != manifest.prompt_configuration_digest
        || plan.harness_digest != manifest.harness_digest
        || plan.toolset_digest != manifest.toolset_digest
        || plan.reference_arm_id != manifest.reference_arm_id
        || plan.candidate_arm_id != manifest.candidate_arm_id
    {
        return Err(AnytimePairedError::ManifestMismatch(
            "anytime analysis plan differs from the matched sealed campaign manifest".into(),
        ));
    }
    Ok(())
}

fn binomial_coefficient(n: usize, k: usize) -> Result<u128, AnytimePairedError> {
    let k = k.min(n.saturating_sub(k));
    let mut value = 1u128;
    for index in 1..=k {
        let factor = u128::try_from(n - index + 1).map_err(|_| {
            AnytimePairedError::ArithmeticOverflow("binomial factor exceeds u128".into())
        })?;
        value = value.checked_mul(factor).ok_or_else(|| {
            AnytimePairedError::ArithmeticOverflow("binomial coefficient overflow".into())
        })? / u128::try_from(index).map_err(|_| {
            AnytimePairedError::ArithmeticOverflow("binomial divisor exceeds u128".into())
        })?;
    }
    Ok(value)
}

fn add_rational(left: Rational, right: Rational) -> Result<Rational, AnytimePairedError> {
    left.checked_add(right)
        .map_err(AnytimePairedError::ArithmeticOverflow)
}

fn gcd(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

fn subtract_nonnegative(
    left: Rational,
    right: Rational,
) -> Result<Rational, AnytimePairedError> {
    if left < right {
        return Err(AnytimePairedError::ArithmeticOverflow(
            "exact e-value alternating sum became negative".into(),
        ));
    }
    let common = gcd(left.denominator(), right.denominator());
    let left_scale = right.denominator() / common;
    let right_scale = left.denominator() / common;
    let left_numerator = left
        .numerator()
        .checked_mul(left_scale)
        .ok_or_else(|| {
            AnytimePairedError::ArithmeticOverflow("e-value subtraction overflow".into())
        })?;
    let right_numerator = right
        .numerator()
        .checked_mul(right_scale)
        .ok_or_else(|| {
            AnytimePairedError::ArithmeticOverflow("e-value subtraction overflow".into())
        })?;
    let denominator = left
        .denominator()
        .checked_mul(left_scale)
        .ok_or_else(|| {
            AnytimePairedError::ArithmeticOverflow("e-value denominator overflow".into())
        })?;
    Rational::new(left_numerator - right_numerator, denominator)
        .map_err(AnytimePairedError::ArithmeticOverflow)
}

/// Exact mixture likelihood-ratio e-value for the one-sided paired sign null.
///
/// For candidate wins w and reference wins l:
///
/// E(w,l) = integral_0^1 (1+u)^w (1-u)^l du.
///
/// It is the uniform mixture of Bernoulli likelihood ratios over alternatives
/// theta in [1/2,1], reparameterized by u=2*theta-1. Under the conditional
/// fair-sign null, each fixed-theta likelihood ratio is a martingale, and its
/// fixed-prior mixture is also a nonnegative martingale. Ties are not included.
pub fn exact_paired_sign_mixture_e_value(
    candidate_wins: usize,
    reference_wins: usize,
) -> Result<Rational, AnytimePairedError> {
    let discordant = candidate_wins
        .checked_add(reference_wins)
        .ok_or_else(|| AnytimePairedError::ArithmeticOverflow(
            "discordant pair count overflow".into(),
        ))?;
    if discordant > MAX_ANYTIME_DISCORDANT_PAIRS {
        return Err(AnytimePairedError::UnsupportedExactSampleSize(format!(
            "{discordant} discordant pairs exceeds the exact mixture cap of {MAX_ANYTIME_DISCORDANT_PAIRS}"
        )));
    }

    let mut positive = Rational::zero();
    let mut negative = Rational::zero();
    for index in 0..=candidate_wins {
        let choose = binomial_coefficient(candidate_wins, index)?;
        let exponent = u32::try_from(candidate_wins - index).map_err(|_| {
            AnytimePairedError::ArithmeticOverflow("mixture power exceeds u32".into())
        })?;
        let power = 1u128.checked_shl(exponent).ok_or_else(|| {
            AnytimePairedError::ArithmeticOverflow("mixture power overflow".into())
        })?;
        let numerator = choose.checked_mul(power).ok_or_else(|| {
            AnytimePairedError::ArithmeticOverflow("mixture polynomial coefficient overflow".into())
        })?;
        let denominator = u128::try_from(reference_wins + index + 1).map_err(|_| {
            AnytimePairedError::ArithmeticOverflow("mixture integral denominator overflow".into())
        })?;
        let term = Rational::new(numerator, denominator)
            .map_err(AnytimePairedError::ArithmeticOverflow)?;
        if index.is_multiple_of(2) {
            positive = add_rational(positive, term)?;
        } else {
            negative = add_rational(negative, term)?;
        }
    }
    subtract_nonnegative(positive, negative)
}

fn increment(value: &mut usize, label: &str) -> Result<(), AnytimePairedError> {
    *value = value.checked_add(1).ok_or_else(|| {
        AnytimePairedError::ArithmeticOverflow(format!("{label} count overflow"))
    })?;
    Ok(())
}

/// Replay a complete append-only sealed paired stream and reconstruct the
/// anytime e-process for every preregistered family.
///
/// Families receive a fixed alpha/m allocation. By Ville's inequality each
/// family's first threshold crossing has Type-I probability at most alpha/m;
/// a union bound controls family-wise error at alpha without independence
/// between the family streams. The exact first crossing is retained even if
/// later observations lower the current e-value.
pub fn replay_anytime_paired_evidence(
    plan: &PreregisteredAnytimeAnalysisPlan,
    manifest: &MatchedBenchmarkManifest,
    events: &[SequencedPairedBenchmarkEvent],
) -> Result<AnytimePairedInferenceReceipt, AnytimePairedError> {
    validate_plan(plan)?;
    validate_manifest_binding(plan, manifest)?;
    if events.len() > MAX_EVENTS {
        return Err(AnytimePairedError::EventSequence(
            "paired event transcript exceeds the configured event cap".into(),
        ));
    }

    let families = plan
        .mechanism_families
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut counts = plan
        .mechanism_families
        .iter()
        .cloned()
        .map(|family| (family, FamilyCounts::default()))
        .collect::<BTreeMap<_, _>>();
    let mut event_ids = BTreeSet::new();
    let family_count = u64::try_from(plan.mechanism_families.len()).map_err(|_| {
        AnytimePairedError::ArithmeticOverflow("family count exceeds u64".into())
    })?;
    let allocated_alpha = plan
        .family_wise_alpha
        .checked_div(Rational::from_u64(family_count))
        .map_err(AnytimePairedError::ArithmeticOverflow)?;
    let critical_e_value = Rational::one()
        .checked_div(allocated_alpha)
        .map_err(AnytimePairedError::ArithmeticOverflow)?;

    let trials = events.iter().map(|event| event.trial.clone()).collect::<Vec<_>>();
    if !events.is_empty() {
        validate_and_summarize_matched_campaign(manifest, &trials)
            .map_err(|error| AnytimePairedError::InvalidMatchedCampaign(error.to_string()))?;
    }

    for (index, event) in events.iter().enumerate() {
        let expected_sequence = u64::try_from(index).map_err(|_| {
            AnytimePairedError::EventSequence("event index exceeds u64".into())
        })?;
        if event.sequence != expected_sequence {
            return Err(AnytimePairedError::EventSequence(format!(
                "expected event sequence {expected_sequence}, found {}",
                event.sequence
            )));
        }
        if !valid_id(&event.event_id) || !event_ids.insert(event.event_id.clone()) {
            return Err(AnytimePairedError::DuplicateIdentity(
                "event identity is empty, overlong or duplicated".into(),
            ));
        }
        let trial = &event.trial;
        if !families.contains(&trial.mechanism_family_id) {
            return Err(AnytimePairedError::UnregisteredFamily(format!(
                "event {} names unregistered mechanism family {}",
                event.sequence, trial.mechanism_family_id
            )));
        }

        let family = counts
            .get_mut(&trial.mechanism_family_id)
            .ok_or_else(|| AnytimePairedError::UnregisteredFamily(
                "registered family disappeared during replay".into(),
            ))?;
        if trial.task_validity == BenchmarkTaskValidity::Invalid {
            increment(&mut family.invalid_tasks_excluded, "invalid task")?;
            continue;
        }

        let reference = trial.reference.as_ref().ok_or_else(|| {
            AnytimePairedError::InvalidMatchedCampaign(
                "valid paired event lacks reference outcome".into(),
            )
        })?;
        let candidate = trial.candidate.as_ref().ok_or_else(|| {
            AnytimePairedError::InvalidMatchedCampaign(
                "valid paired event lacks candidate outcome".into(),
            )
        })?;
        if reference.terminal_loss_units > plan.maximum_terminal_loss_units
            || candidate.terminal_loss_units > plan.maximum_terminal_loss_units
        {
            return Err(AnytimePairedError::LossOutsidePreregisteredRange(format!(
                "event {} contains loss above preregistered maximum {}",
                event.sequence, plan.maximum_terminal_loss_units
            )));
        }

        increment(&mut family.total_pairs, "total paired task")?;
        match candidate
            .terminal_loss_units
            .cmp(&reference.terminal_loss_units)
        {
            std::cmp::Ordering::Less => increment(&mut family.candidate_wins, "candidate win")?,
            std::cmp::Ordering::Greater => increment(&mut family.reference_wins, "reference win")?,
            std::cmp::Ordering::Equal => increment(&mut family.ties, "tie")?,
        }

        let discordant = family
            .candidate_wins
            .checked_add(family.reference_wins)
            .ok_or_else(|| AnytimePairedError::ArithmeticOverflow(
                "discordant pair count overflow".into(),
            ))?;
        if discordant > MAX_ANYTIME_DISCORDANT_PAIRS {
            return Err(AnytimePairedError::UnsupportedExactSampleSize(format!(
                "family {} exceeds the exact discordant-pair cap of {}",
                trial.mechanism_family_id, MAX_ANYTIME_DISCORDANT_PAIRS
            )));
        }
        if family.first_crossing_sequence.is_none()
            && discordant >= plan.minimum_discordant_pairs_before_claim
        {
            let e_value = exact_paired_sign_mixture_e_value(
                family.candidate_wins,
                family.reference_wins,
            )?;
            if e_value >= critical_e_value {
                family.first_crossing_sequence = Some(event.sequence);
            }
        }
    }

    let mut family_results = Vec::with_capacity(plan.mechanism_families.len());
    for family_id in &plan.mechanism_families {
        let family = counts.get(family_id).cloned().unwrap_or_default();
        let discordant_pairs = family
            .candidate_wins
            .checked_add(family.reference_wins)
            .ok_or_else(|| AnytimePairedError::ArithmeticOverflow(
                "discordant count overflow".into(),
            ))?;
        let current_e_value =
            exact_paired_sign_mixture_e_value(family.candidate_wins, family.reference_wins)?;
        family_results.push(FamilyAnytimeEvidence {
            mechanism_family_id: family_id.clone(),
            total_pairs: family.total_pairs,
            invalid_tasks_excluded: family.invalid_tasks_excluded,
            candidate_wins: family.candidate_wins,
            reference_wins: family.reference_wins,
            ties: family.ties,
            discordant_pairs,
            current_e_value,
            allocated_alpha,
            critical_e_value,
            first_crossing_sequence: family.first_crossing_sequence,
            crossed_anytime_threshold: family.first_crossing_sequence.is_some(),
            maximum_supported_discordant_pairs: MAX_ANYTIME_DISCORDANT_PAIRS,
        });
    }

    Ok(AnytimePairedInferenceReceipt {
        analysis_id: plan.analysis_id.clone(),
        analysis_version: plan.analysis_version.clone(),
        pre_registration_receipt_digest: plan.pre_registration_receipt_digest.clone(),
        campaign_id: plan.campaign_id.clone(),
        scope: plan.scope,
        split: manifest.split,
        group_id: plan.group_id.clone(),
        reference_arm_id: plan.reference_arm_id.clone(),
        candidate_arm_id: plan.candidate_arm_id.clone(),
        family_wise_alpha: plan.family_wise_alpha,
        family_count: family_results.len(),
        event_count: events.len(),
        family_results,
        sealed_holdout_only: true,
        complete_preregistered_family_set_retained: true,
        bonferroni_anytime_fwer_control_applied: true,
        cross_family_independence_required: false,
        conditional_sign_symmetry_assumption: true,
        optional_stopping_supported: true,
        exact_rational_e_process: true,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        MAX_ANYTIME_DISCORDANT_PAIRS, PreregisteredAnytimeAnalysisPlan,
        SequencedPairedBenchmarkEvent, exact_paired_sign_mixture_e_value,
        replay_anytime_paired_evidence,
    };
    use super::super::counterexample_ledger::EvidenceSplit;
    use super::super::matched_benchmark::{
        ArmOutcome, BenchmarkScope, BenchmarkTaskValidity, MatchedBenchmarkManifest,
        PairedBenchmarkTrial, ResourceBudget, ResourceUsage,
    };
    use crate::Rational;

    fn q(numerator: u128, denominator: u128) -> Rational {
        Rational::new(numerator, denominator).unwrap()
    }

    fn budget() -> ResourceBudget {
        ResourceBudget {
            max_wall_time_ms: 10_000,
            max_model_tokens: 20_000,
            max_tool_calls: 100,
            max_simulator_steps: 1_000,
            max_energy_millijoules: 50_000,
        }
    }

    fn manifest() -> MatchedBenchmarkManifest {
        MatchedBenchmarkManifest {
            campaign_id: "campaign-anytime".into(),
            protocol_version: "v1".into(),
            scope: BenchmarkScope::PlannerOnly,
            split: EvidenceSplit::SealedHoldout,
            group_id: "sealed-anytime".into(),
            agent_panel_id: "fixed-panel".into(),
            task_manifest_digest: "task-manifest".into(),
            model_configuration_digest: "model-config".into(),
            prompt_configuration_digest: "prompt-config".into(),
            harness_digest: "harness".into(),
            toolset_digest: "toolset".into(),
            reference_arm_id: "reference".into(),
            candidate_arm_id: "candidate".into(),
            resource_budget: budget(),
            minimum_pairs_per_family: 1,
        }
    }

    fn plan(families: Vec<&str>, alpha: Rational, minimum_discordant: usize) -> PreregisteredAnytimeAnalysisPlan {
        PreregisteredAnytimeAnalysisPlan {
            analysis_id: "anytime-analysis".into(),
            analysis_version: "v1".into(),
            campaign_id: "campaign-anytime".into(),
            scope: BenchmarkScope::PlannerOnly,
            group_id: "sealed-anytime".into(),
            agent_panel_id: "fixed-panel".into(),
            task_manifest_digest: "task-manifest".into(),
            model_configuration_digest: "model-config".into(),
            prompt_configuration_digest: "prompt-config".into(),
            harness_digest: "harness".into(),
            toolset_digest: "toolset".into(),
            reference_arm_id: "reference".into(),
            candidate_arm_id: "candidate".into(),
            pre_registration_receipt_digest: "preregistration-receipt".into(),
            family_wise_alpha: alpha,
            maximum_terminal_loss_units: 100,
            minimum_discordant_pairs_before_claim: minimum_discordant,
            mechanism_families: families.into_iter().map(str::to_string).collect(),
        }
    }

    fn arm(id: &str, loss: u64, evidence: &str) -> ArmOutcome {
        ArmOutcome {
            arm_id: id.into(),
            system_version: format!("{id}-v1"),
            task_completed: true,
            safe_success: true,
            hazard_observed: false,
            abstained: false,
            terminal_loss_units: loss,
            usage: ResourceUsage {
                wall_time_ms: 1_000,
                model_tokens: 2_000,
                tool_calls: 10,
                simulator_steps: 40,
                energy_millijoules: 3_000,
            },
            evidence_artifact_digest: evidence.into(),
        }
    }

    fn event(
        sequence: u64,
        pair_id: &str,
        task_id: &str,
        family: &str,
        reference_loss: u64,
        candidate_loss: u64,
    ) -> SequencedPairedBenchmarkEvent {
        let pair = pair_id.to_string();
        SequencedPairedBenchmarkEvent {
            event_id: format!("event-{pair}"),
            sequence,
            trial: PairedBenchmarkTrial {
                pair_id: pair.clone(),
                task_id: task_id.into(),
                mechanism_family_id: family.into(),
                paired_seed: sequence,
                task_validity: BenchmarkTaskValidity::Valid,
                task_oracle_receipt_digest: format!("oracle-{pair}"),
                task_manifest_digest: "task-manifest".into(),
                model_configuration_digest: "model-config".into(),
                prompt_configuration_digest: "prompt-config".into(),
                harness_digest: "harness".into(),
                toolset_digest: "toolset".into(),
                scope: BenchmarkScope::PlannerOnly,
                split: EvidenceSplit::SealedHoldout,
                group_id: "sealed-anytime".into(),
                agent_panel_id: "fixed-panel".into(),
                resource_budget: budget(),
                reference: Some(arm("reference", reference_loss, &format!("ref-{pair}"))),
                candidate: Some(arm("candidate", candidate_loss, &format!("cand-{pair}"))),
            },
        }
    }

    fn mixture_expansion_oracle(wins: usize, losses: usize) -> Rational {
        fn choose(n: usize, k: usize) -> u128 {
            let k = k.min(n.saturating_sub(k));
            let mut value = 1u128;
            for i in 1..=k {
                value = value * (n - i + 1) as u128 / i as u128;
            }
            value
        }
        let mut positive = Rational::zero();
        let mut negative = Rational::zero();
        for i in 0..=wins {
            for j in 0..=losses {
                let numerator = choose(wins, i) * choose(losses, j);
                let denominator = (i + j + 1) as u128;
                let term = q(numerator, denominator);
                if j.is_multiple_of(2) {
                    positive = positive.checked_add(term).unwrap();
                } else {
                    negative = negative.checked_add(term).unwrap();
                }
            }
        }
        subtract(positive, negative)
    }

    fn subtract(left: Rational, right: Rational) -> Rational {
        assert!(left >= right);
        let gcd = {
            let mut a = left.denominator();
            let mut b = right.denominator();
            while b != 0 {
                let r = a % b;
                a = b;
                b = r;
            }
            a.max(1)
        };
        let left_scale = right.denominator() / gcd;
        let right_scale = left.denominator() / gcd;
        q(
            left.numerator() * left_scale - right.numerator() * right_scale,
            left.denominator() * left_scale,
        )
    }

    #[test]
    fn mixture_e_values_match_exact_fixtures_and_independent_polynomial_expansion() {
        assert_eq!(exact_paired_sign_mixture_e_value(0, 0).unwrap(), Rational::one());
        assert_eq!(exact_paired_sign_mixture_e_value(1, 0).unwrap(), q(3, 2));
        assert_eq!(exact_paired_sign_mixture_e_value(0, 1).unwrap(), q(1, 2));
        assert_eq!(exact_paired_sign_mixture_e_value(1, 1).unwrap(), q(2, 3));
        assert_eq!(exact_paired_sign_mixture_e_value(2, 0).unwrap(), q(7, 3));
        for wins in 0..=8 {
            for losses in 0..=(8 - wins) {
                assert_eq!(
                    exact_paired_sign_mixture_e_value(wins, losses).unwrap(),
                    mixture_expansion_oracle(wins, losses)
                );
            }
        }
        assert!(exact_paired_sign_mixture_e_value(MAX_ANYTIME_DISCORDANT_PAIRS + 1, 0).is_err());
    }

    #[test]
    fn exact_mixture_satisfies_the_fair_sign_martingale_identity() {
        for wins in 0..=12 {
            for losses in 0..=(12 - wins) {
                let current = exact_paired_sign_mixture_e_value(wins, losses).unwrap();
                let after_win = exact_paired_sign_mixture_e_value(wins + 1, losses).unwrap();
                let after_loss = exact_paired_sign_mixture_e_value(wins, losses + 1).unwrap();
                let average = after_win
                    .checked_add(after_loss)
                    .unwrap()
                    .checked_div(Rational::from_u64(2))
                    .unwrap();
                assert_eq!(average, current);
            }
        }
    }

    #[test]
    fn replay_preserves_first_anytime_crossing_after_later_losses_and_ties() {
        let events = vec![
            event(0, "pair-0", "task-0", "sensor-drift", 10, 8),
            event(1, "pair-1", "task-1", "sensor-drift", 10, 10),
            event(2, "pair-2", "task-2", "sensor-drift", 10, 8),
            event(3, "pair-3", "task-3", "sensor-drift", 10, 8),
            event(4, "pair-4", "task-4", "sensor-drift", 10, 12),
        ];
        let result = replay_anytime_paired_evidence(
            &plan(vec!["sensor-drift"], q(1, 2), 2),
            &manifest(),
            &events,
        )
        .unwrap();
        let family = &result.family_results[0];
        assert_eq!(family.candidate_wins, 3);
        assert_eq!(family.reference_wins, 1);
        assert_eq!(family.ties, 1);
        assert!(family.crossed_anytime_threshold);
        assert_eq!(family.first_crossing_sequence, Some(3));
        assert!(family.current_e_value < family.critical_e_value);
    }

    #[test]
    fn tied_pairs_do_not_change_the_e_process() {
        let one_win = vec![event(0, "pair-0", "task-0", "sensor-drift", 10, 8)];
        let mut one_win_and_tie = one_win.clone();
        let mut tie = event(1, "pair-1", "task-1", "sensor-drift", 10, 10);
        tie.event_id = "event-tie".into();
        one_win_and_tie.push(tie);
        let first = replay_anytime_paired_evidence(
            &plan(vec!["sensor-drift"], q(1, 2), 1),
            &manifest(),
            &one_win,
        )
        .unwrap();
        let second = replay_anytime_paired_evidence(
            &plan(vec!["sensor-drift"], q(1, 2), 1),
            &manifest(),
            &one_win_and_tie,
        )
        .unwrap();
        assert_eq!(
            first.family_results[0].current_e_value,
            second.family_results[0].current_e_value
        );
    }

    #[test]
    fn exact_null_path_enumeration_respects_ville_crossing_bound() {
        // One family, alpha=1/2, critical e-value=2. Enumerate every fair
        // sign path through horizon 8, checking whether the running process
        // ever crosses. The finite-horizon crossing fraction must not exceed
        // alpha; this complements (but does not replace) the martingale proof.
        let horizon = 8usize;
        let path_count = 1u128 << horizon;
        let mut crossings = 0u128;
        for path in 0..path_count {
            let mut wins = 0usize;
            let mut losses = 0usize;
            let mut crossed = false;
            for step in 0..horizon {
                if (path >> step) & 1 == 1 {
                    wins += 1;
                } else {
                    losses += 1;
                }
                if exact_paired_sign_mixture_e_value(wins, losses).unwrap() >= q(2, 1) {
                    crossed = true;
                }
            }
            if crossed {
                crossings += 1;
            }
        }
        assert!(crossings * 2 <= path_count);
    }

    #[test]
    fn multiple_families_receive_fixed_bonferroni_anytime_thresholds() {
        let mut events = Vec::new();
        let mut sequence = 0u64;
        for family in ["sensor-drift", "actuator-fault"] {
            for pair in 0..4 {
                events.push(event(
                    sequence,
                    &format!("{family}-{pair}"),
                    &format!("task-{family}-{pair}"),
                    family,
                    10,
                    8,
                ));
                sequence += 1;
            }
        }
        let result = replay_anytime_paired_evidence(
            &plan(vec!["sensor-drift", "actuator-fault"], q(1, 2), 2),
            &manifest(),
            &events,
        )
        .unwrap();
        assert_eq!(result.family_count, 2);
        assert_eq!(result.family_results[0].allocated_alpha, q(1, 4));
        assert_eq!(result.family_results[0].critical_e_value, q(4, 1));
        assert_eq!(result.family_results[1].critical_e_value, q(4, 1));
        assert!(result.bonferroni_anytime_fwer_control_applied);
        assert!(!result.cross_family_independence_required);
    }

    #[test]
    fn rejects_development_calibration_family_leakage_and_reordered_events() {
        let events = vec![event(0, "pair-0", "task-0", "sensor-drift", 10, 8)];
        for split in [EvidenceSplit::Development, EvidenceSplit::Calibration] {
            let mut changed_manifest = manifest();
            changed_manifest.split = split;
            assert!(replay_anytime_paired_evidence(
                &plan(vec!["sensor-drift"], q(1, 2), 1),
                &changed_manifest,
                &events,
            )
            .is_err());
        }

        let extra = vec![event(0, "pair-0", "task-0", "not-preregistered", 10, 8)];
        assert!(replay_anytime_paired_evidence(
            &plan(vec!["sensor-drift"], q(1, 2), 1),
            &manifest(),
            &extra,
        )
        .is_err());

        let reordered = vec![
            event(1, "pair-1", "task-1", "sensor-drift", 10, 8),
            event(0, "pair-0", "task-0", "sensor-drift", 10, 8),
        ];
        assert!(replay_anytime_paired_evidence(
            &plan(vec!["sensor-drift"], q(1, 2), 1),
            &manifest(),
            &reordered,
        )
        .is_err());
    }
}
