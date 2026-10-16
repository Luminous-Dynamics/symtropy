// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Exact finite-state value-of-information comparisons for bounded planning.
//!
//! The caller owns the world model, probability justification, utility meanings,
//! safety/rights gates and external evidence. This crate performs deterministic
//! arithmetic only; it does not infer probabilities or certify real-world claims.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{BTreeMap, BTreeSet};

pub const MEASUREMENT_PLANNER_SCHEMA_VERSION: u16 = 1;
pub const PROBABILITY_SCALE_PPM: u32 = 1_000_000;
pub const EVSI_DENOMINATOR_PPM_SQUARED: u64 = 1_000_000_000_000;

fn serialize_i64_as_string<S>(value: &i64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&value.to_string())
}

fn deserialize_i64_from_string<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: Deserializer<'de>,
{
    String::deserialize(deserializer)?
        .parse::<i64>()
        .map_err(serde::de::Error::custom)
}

fn serialize_u64_as_string<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&value.to_string())
}

fn deserialize_u64_from_string<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    String::deserialize(deserializer)?
        .parse::<u64>()
        .map_err(serde::de::Error::custom)
}

fn serialize_i128_as_string<S>(value: &i128, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&value.to_string())
}

fn deserialize_i128_from_string<'de, D>(deserializer: D) -> Result<i128, D::Error>
where
    D: Deserializer<'de>,
{
    String::deserialize(deserializer)?
        .parse::<i128>()
        .map_err(serde::de::Error::custom)
}

fn serialize_option_i128_as_string<S>(
    value: &Option<i128>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(value) => serializer.serialize_some(&value.to_string()),
        None => serializer.serialize_none(),
    }
}

fn deserialize_option_i128_from_string<'de, D>(
    deserializer: D,
) -> Result<Option<i128>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer)?
        .map(|raw| raw.parse::<i128>().map_err(serde::de::Error::custom))
        .transpose()
}

/// Declared origin of the probability model. This label is caller-supplied and is
/// not a certificate that calibration or expert elicitation is methodologically sound.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbabilityBasisV1 {
    CalibratedEmpirical,
    ReviewedExpertElicitation,
    SyntheticFixture,
    Unjustified,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldProbabilityV1 {
    pub world_id: String,
    pub probability_ppm: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionUtilityV1 {
    pub world_id: String,
    /// Caller-defined utility, represented as an integer at the scenario scale.
    #[serde(
        serialize_with = "serialize_i64_as_string",
        deserialize_with = "deserialize_i64_from_string"
    )]
    pub utility_scaled: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionOptionV1 {
    pub decision_id: String,
    /// Hard feasibility is not a score and cannot be traded for utility.
    pub hard_feasible: bool,
    pub hard_blockers: Vec<String>,
    pub utility_by_world: Vec<DecisionUtilityV1>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationLikelihoodV1 {
    pub world_id: String,
    pub outcome_id: String,
    pub probability_ppm: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeasurementActionV1 {
    pub action_id: String,
    pub hard_feasible: bool,
    pub hard_blockers: Vec<String>,
    /// Measurement cost mapped by the caller to the same scaled utility unit.
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string"
    )]
    pub cost_utility_scaled: u64,
    pub probability_basis: ProbabilityBasisV1,
    /// Likelihood P(outcome | world), for every outcome in every world.
    pub likelihoods: Vec<ObservationLikelihoodV1>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValueOfInformationScenarioV1 {
    pub schema_version: u16,
    pub scenario_id: String,
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string"
    )]
    pub utility_scale: u64,
    pub worlds: Vec<WorldProbabilityV1>,
    pub decisions: Vec<DecisionOptionV1>,
    pub measurements: Vec<MeasurementActionV1>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeasurementPlannerBlockerCodeV1 {
    UnsupportedScenarioSchema,
    EmptyScenarioId,
    ZeroUtilityScale,
    NoWorlds,
    EmptyWorldId,
    DuplicateWorldId,
    ProbabilityAboveScale,
    PriorNotNormalized,
    NoDecisions,
    EmptyDecisionId,
    DuplicateDecisionId,
    DecisionHardMetadataInconsistent,
    DuplicateDecisionUtility,
    UnknownDecisionWorld,
    MissingDecisionWorldUtility,
    NoFeasibleDecision,
    EmptyMeasurementId,
    DuplicateMeasurementId,
    MeasurementHardMetadataInconsistent,
    MeasurementHardBlocked,
    UnjustifiedProbabilityModel,
    EmptyOutcomeId,
    UnknownLikelihoodWorld,
    DuplicateWorldOutcomeLikelihood,
    LikelihoodAboveScale,
    LikelihoodRowNotNormalized,
    IncompleteOutcomeMatrix,
    NoOutcomes,
    ArithmeticOverflow,
    NegativeGrossEvsiInvariant,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MeasurementPlannerBlockerV1 {
    pub code: MeasurementPlannerBlockerCodeV1,
    pub detail: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeasurementDispositionV1 {
    PositiveNetValue,
    NonPositiveNetValue,
    Blocked,
    NotEstimable,
    SyntheticFixtureOnly,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutcomeDecisionV1 {
    pub outcome_id: String,
    pub decision_id: String,
    /// Joint-weighted utility numerator for the selected decision in this outcome.
    #[serde(
        serialize_with = "serialize_i128_as_string",
        deserialize_with = "deserialize_i128_from_string"
    )]
    pub joint_utility_numerator: i128,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeasurementComparisonV1 {
    pub action_id: String,
    pub probability_basis: ProbabilityBasisV1,
    pub disposition: MeasurementDispositionV1,
    pub baseline_decision_id: String,
    /// Denominator is PROBABILITY_SCALE_PPM.
    #[serde(
        serialize_with = "serialize_i128_as_string",
        deserialize_with = "deserialize_i128_from_string"
    )]
    pub baseline_expected_utility_numerator_ppm: i128,
    /// All following VOI numerators use denominator PPM squared.
    #[serde(
        serialize_with = "serialize_option_i128_as_string",
        deserialize_with = "deserialize_option_i128_from_string"
    )]
    pub expected_utility_after_measurement_numerator_ppm_squared: Option<i128>,
    #[serde(
        serialize_with = "serialize_option_i128_as_string",
        deserialize_with = "deserialize_option_i128_from_string"
    )]
    pub gross_evsi_numerator_ppm_squared: Option<i128>,
    #[serde(
        serialize_with = "serialize_option_i128_as_string",
        deserialize_with = "deserialize_option_i128_from_string"
    )]
    pub measurement_cost_numerator_ppm_squared: Option<i128>,
    #[serde(
        serialize_with = "serialize_option_i128_as_string",
        deserialize_with = "deserialize_option_i128_from_string"
    )]
    pub net_evsi_numerator_ppm_squared: Option<i128>,
    pub outcome_decisions: Vec<OutcomeDecisionV1>,
    pub blockers: Vec<MeasurementPlannerBlockerV1>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValueOfInformationReportV1 {
    pub schema_version: u16,
    pub scenario_id: String,
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string"
    )]
    pub utility_scale: u64,
    pub probability_scale_ppm: u32,
    pub evsi_denominator_ppm_squared: u64,
    pub baseline_decision_id: String,
    #[serde(
        serialize_with = "serialize_i128_as_string",
        deserialize_with = "deserialize_i128_from_string"
    )]
    pub baseline_expected_utility_numerator_ppm: i128,
    pub measurement_comparisons: Vec<MeasurementComparisonV1>,
}

fn blocker(
    code: MeasurementPlannerBlockerCodeV1,
    detail: impl Into<String>,
) -> MeasurementPlannerBlockerV1 {
    MeasurementPlannerBlockerV1 {
        code,
        detail: detail.into(),
    }
}

fn sort_blockers(blockers: &mut Vec<MeasurementPlannerBlockerV1>) {
    blockers.sort();
    blockers.dedup();
}

fn checked_add(
    left: i128,
    right: i128,
) -> Result<i128, MeasurementPlannerBlockerCodeV1> {
    left.checked_add(right)
        .ok_or(MeasurementPlannerBlockerCodeV1::ArithmeticOverflow)
}

fn checked_mul(
    left: i128,
    right: i128,
) -> Result<i128, MeasurementPlannerBlockerCodeV1> {
    left.checked_mul(right)
        .ok_or(MeasurementPlannerBlockerCodeV1::ArithmeticOverflow)
}

fn validate_scenario(
    scenario: &ValueOfInformationScenarioV1,
) -> Vec<MeasurementPlannerBlockerV1> {
    use MeasurementPlannerBlockerCodeV1 as Code;
    let mut blockers = Vec::new();

    if scenario.schema_version != MEASUREMENT_PLANNER_SCHEMA_VERSION {
        blockers.push(blocker(
            Code::UnsupportedScenarioSchema,
            "unsupported measurement-planner schema version",
        ));
    }
    if scenario.scenario_id.trim().is_empty() {
        blockers.push(blocker(Code::EmptyScenarioId, "scenario ID is empty"));
    }
    if scenario.utility_scale == 0 {
        blockers.push(blocker(Code::ZeroUtilityScale, "utility scale must be positive"));
    }
    if scenario.worlds.is_empty() {
        blockers.push(blocker(Code::NoWorlds, "scenario must declare at least one world"));
    }

    let mut worlds = BTreeMap::new();
    let mut prior_sum = 0_u64;
    for world in &scenario.worlds {
        if world.world_id.trim().is_empty() {
            blockers.push(blocker(Code::EmptyWorldId, "world ID is empty"));
        }
        if worlds.insert(world.world_id.clone(), world.probability_ppm).is_some() {
            blockers.push(blocker(
                Code::DuplicateWorldId,
                format!("duplicate world ID {:?}", world.world_id),
            ));
        }
        if world.probability_ppm > PROBABILITY_SCALE_PPM {
            blockers.push(blocker(
                Code::ProbabilityAboveScale,
                format!("prior for world {:?} exceeds PPM scale", world.world_id),
            ));
        }
        prior_sum = match prior_sum.checked_add(u64::from(world.probability_ppm)) {
            Some(sum) => sum,
            None => {
                blockers.push(blocker(Code::ArithmeticOverflow, "prior probability sum overflow"));
                prior_sum
            }
        };
    }
    if !scenario.worlds.is_empty() && prior_sum != u64::from(PROBABILITY_SCALE_PPM) {
        blockers.push(blocker(
            Code::PriorNotNormalized,
            format!("prior probabilities sum to {prior_sum}, not {PROBABILITY_SCALE_PPM}"),
        ));
    }

    let world_ids: BTreeSet<_> = worlds.keys().cloned().collect();
    let mut decision_ids = BTreeSet::new();
    let mut has_feasible_decision = false;

    if scenario.decisions.is_empty() {
        blockers.push(blocker(Code::NoDecisions, "scenario must declare a decision"));
    }

    for decision in &scenario.decisions {
        if decision.decision_id.trim().is_empty() {
            blockers.push(blocker(Code::EmptyDecisionId, "decision ID is empty"));
        }
        if !decision_ids.insert(decision.decision_id.clone()) {
            blockers.push(blocker(
                Code::DuplicateDecisionId,
                format!("duplicate decision ID {:?}", decision.decision_id),
            ));
        }
        if decision.hard_feasible != decision.hard_blockers.is_empty() {
            blockers.push(blocker(
                Code::DecisionHardMetadataInconsistent,
                format!(
                    "decision {:?} has conflicting feasibility and blocker declarations",
                    decision.decision_id
                ),
            ));
        }
        has_feasible_decision |= decision.hard_feasible && decision.hard_blockers.is_empty();

        let mut covered_worlds = BTreeSet::new();
        for utility in &decision.utility_by_world {
            if !covered_worlds.insert(utility.world_id.clone()) {
                blockers.push(blocker(
                    Code::DuplicateDecisionUtility,
                    format!(
                        "decision {:?} repeats utility for world {:?}",
                        decision.decision_id, utility.world_id
                    ),
                ));
            }
            if !world_ids.contains(&utility.world_id) {
                blockers.push(blocker(
                    Code::UnknownDecisionWorld,
                    format!(
                        "decision {:?} defines utility for unknown world {:?}",
                        decision.decision_id, utility.world_id
                    ),
                ));
            }
        }
        for missing in world_ids.difference(&covered_worlds) {
            blockers.push(blocker(
                Code::MissingDecisionWorldUtility,
                format!(
                    "decision {:?} omits utility for world {:?}",
                    decision.decision_id, missing
                ),
            ));
        }
    }
    if !scenario.decisions.is_empty() && !has_feasible_decision {
        blockers.push(blocker(
            Code::NoFeasibleDecision,
            "at least one decision must pass every hard gate",
        ));
    }

    let mut measurement_ids = BTreeSet::new();
    for action in &scenario.measurements {
        if action.action_id.trim().is_empty() {
            blockers.push(blocker(
                Code::EmptyMeasurementId,
                "measurement action ID is empty",
            ));
        }
        if !measurement_ids.insert(action.action_id.clone()) {
            blockers.push(blocker(
                Code::DuplicateMeasurementId,
                format!("duplicate measurement ID {:?}", action.action_id),
            ));
        }
    }

    sort_blockers(&mut blockers);
    blockers
}

fn baseline_utility(
    scenario: &ValueOfInformationScenarioV1,
) -> Result<(String, i128), MeasurementPlannerBlockerV1> {
    let mut worlds: Vec<_> = scenario.worlds.iter().collect();
    worlds.sort_by(|left, right| left.world_id.cmp(&right.world_id));

    let mut options: Vec<_> = scenario
        .decisions
        .iter()
        .filter(|decision| decision.hard_feasible && decision.hard_blockers.is_empty())
        .collect();
    options.sort_by(|left, right| left.decision_id.cmp(&right.decision_id));

    let mut best: Option<(String, i128)> = None;
    for decision in options {
        let utilities: BTreeMap<_, _> = decision
            .utility_by_world
            .iter()
            .map(|entry| (entry.world_id.as_str(), entry.utility_scaled))
            .collect();
        let mut total = 0_i128;
        for world in &worlds {
            let utility = i128::from(*utilities.get(world.world_id.as_str()).ok_or_else(|| {
                blocker(
                    MeasurementPlannerBlockerCodeV1::MissingDecisionWorldUtility,
                    "validated decision unexpectedly lacks a world utility",
                )
            })?);
            let weighted = checked_mul(
                i128::from(world.probability_ppm),
                utility,
            )
            .map_err(|code| blocker(code, "baseline utility multiplication overflow"))?;
            total = checked_add(total, weighted)
                .map_err(|code| blocker(code, "baseline utility accumulation overflow"))?;
        }

        let replace = match &best {
            None => true,
            Some((best_id, best_total)) => {
                total > *best_total || (total == *best_total && decision.decision_id < *best_id)
            }
        };
        if replace {
            best = Some((decision.decision_id.clone(), total));
        }
    }

    best.ok_or_else(|| {
        blocker(
            MeasurementPlannerBlockerCodeV1::NoFeasibleDecision,
            "no feasible decision remains after validation",
        )
    })
}

type LikelihoodMatrix = BTreeMap<(String, String), u32>;

fn validate_likelihoods(
    scenario: &ValueOfInformationScenarioV1,
    action: &MeasurementActionV1,
) -> Result<(LikelihoodMatrix, BTreeSet<String>), Vec<MeasurementPlannerBlockerV1>> {
    use MeasurementPlannerBlockerCodeV1 as Code;
    let mut blockers = Vec::new();
    let world_ids: BTreeSet<_> = scenario.worlds.iter().map(|w| w.world_id.clone()).collect();
    let mut matrix = BTreeMap::new();
    let mut outcomes = BTreeSet::new();
    let mut row_sums: BTreeMap<String, u64> = BTreeMap::new();

    if action.likelihoods.is_empty() {
        blockers.push(blocker(Code::NoOutcomes, "measurement has no outcomes"));
        return Err(blockers);
    }

    for likelihood in &action.likelihoods {
        if likelihood.outcome_id.trim().is_empty() {
            blockers.push(blocker(Code::EmptyOutcomeId, "measurement contains an empty outcome ID"));
        }
        if !world_ids.contains(&likelihood.world_id) {
            blockers.push(blocker(
                Code::UnknownLikelihoodWorld,
                format!("likelihood references unknown world {:?}", likelihood.world_id),
            ));
            continue;
        }
        if likelihood.probability_ppm > PROBABILITY_SCALE_PPM {
            blockers.push(blocker(
                Code::LikelihoodAboveScale,
                format!(
                    "likelihood for ({:?}, {:?}) exceeds PPM scale",
                    likelihood.world_id, likelihood.outcome_id
                ),
            ));
        }
        let key = (likelihood.world_id.clone(), likelihood.outcome_id.clone());
        if matrix.insert(key.clone(), likelihood.probability_ppm).is_some() {
            blockers.push(blocker(
                Code::DuplicateWorldOutcomeLikelihood,
                format!(
                    "duplicate likelihood for world {:?} and outcome {:?}",
                    key.0, key.1
                ),
            ));
            continue;
        }
        outcomes.insert(likelihood.outcome_id.clone());
        let sum = row_sums.entry(likelihood.world_id.clone()).or_default();
        *sum = match sum.checked_add(u64::from(likelihood.probability_ppm)) {
            Some(value) => value,
            None => {
                blockers.push(blocker(Code::ArithmeticOverflow, "likelihood row sum overflow"));
                *sum
            }
        };
    }

    if outcomes.is_empty() {
        blockers.push(blocker(Code::NoOutcomes, "measurement has no valid outcome IDs"));
    }
    for world_id in &world_ids {
        match row_sums.get(world_id) {
            Some(sum) if *sum == u64::from(PROBABILITY_SCALE_PPM) => {}
            Some(sum) => blockers.push(blocker(
                Code::LikelihoodRowNotNormalized,
                format!(
                    "likelihood row for world {:?} sums to {sum}, not {PROBABILITY_SCALE_PPM}",
                    world_id
                ),
            )),
            None => blockers.push(blocker(
                Code::LikelihoodRowNotNormalized,
                format!("likelihood row for world {:?} is absent", world_id),
            )),
        }
        for outcome_id in &outcomes {
            if !matrix.contains_key(&(world_id.clone(), outcome_id.clone())) {
                blockers.push(blocker(
                    Code::IncompleteOutcomeMatrix,
                    format!(
                        "likelihood matrix lacks world {:?}, outcome {:?}",
                        world_id, outcome_id
                    ),
                ));
            }
        }
    }

    sort_blockers(&mut blockers);
    if blockers.is_empty() {
        Ok((matrix, outcomes))
    } else {
        Err(blockers)
    }
}

fn blocked_comparison(
    action: &MeasurementActionV1,
    baseline_id: &str,
    baseline_numerator: i128,
    mut blockers: Vec<MeasurementPlannerBlockerV1>,
) -> MeasurementComparisonV1 {
    sort_blockers(&mut blockers);
    MeasurementComparisonV1 {
        action_id: action.action_id.clone(),
        probability_basis: action.probability_basis,
        disposition: MeasurementDispositionV1::Blocked,
        baseline_decision_id: baseline_id.to_owned(),
        baseline_expected_utility_numerator_ppm: baseline_numerator,
        expected_utility_after_measurement_numerator_ppm_squared: None,
        gross_evsi_numerator_ppm_squared: None,
        measurement_cost_numerator_ppm_squared: None,
        net_evsi_numerator_ppm_squared: None,
        outcome_decisions: Vec::new(),
        blockers,
    }
}

fn compare_measurement(
    scenario: &ValueOfInformationScenarioV1,
    action: &MeasurementActionV1,
    baseline_id: &str,
    baseline_numerator_ppm: i128,
) -> MeasurementComparisonV1 {
    use MeasurementPlannerBlockerCodeV1 as Code;

    if !action.hard_feasible || !action.hard_blockers.is_empty() {
        let mut blockers: Vec<_> = action
            .hard_blockers
            .iter()
            .map(|detail| blocker(Code::MeasurementHardBlocked, detail.clone()))
            .collect();
        if action.hard_feasible != action.hard_blockers.is_empty() {
            blockers.push(blocker(
                Code::MeasurementHardMetadataInconsistent,
                "measurement feasibility and blocker declarations conflict",
            ));
        }
        if blockers.is_empty() {
            blockers.push(blocker(
                Code::MeasurementHardBlocked,
                "measurement is hard-infeasible",
            ));
        }
        return blocked_comparison(action, baseline_id, baseline_numerator_ppm, blockers);
    }

    if action.probability_basis == ProbabilityBasisV1::Unjustified {
        return MeasurementComparisonV1 {
            action_id: action.action_id.clone(),
            probability_basis: action.probability_basis,
            disposition: MeasurementDispositionV1::NotEstimable,
            baseline_decision_id: baseline_id.to_owned(),
            baseline_expected_utility_numerator_ppm: baseline_numerator_ppm,
            expected_utility_after_measurement_numerator_ppm_squared: None,
            gross_evsi_numerator_ppm_squared: None,
            measurement_cost_numerator_ppm_squared: None,
            net_evsi_numerator_ppm_squared: None,
            outcome_decisions: Vec::new(),
            blockers: vec![blocker(
                Code::UnjustifiedProbabilityModel,
                "expected value is not estimated without a justified probability model",
            )],
        };
    }

    let (likelihoods, outcomes) = match validate_likelihoods(scenario, action) {
        Ok(valid) => valid,
        Err(blockers) => {
            return blocked_comparison(action, baseline_id, baseline_numerator_ppm, blockers);
        }
    };

    let mut worlds: Vec<_> = scenario.worlds.iter().collect();
    worlds.sort_by(|left, right| left.world_id.cmp(&right.world_id));
    let mut decisions: Vec<_> = scenario
        .decisions
        .iter()
        .filter(|decision| decision.hard_feasible && decision.hard_blockers.is_empty())
        .collect();
    decisions.sort_by(|left, right| left.decision_id.cmp(&right.decision_id));
    let utility_maps: BTreeMap<_, BTreeMap<_, _>> = decisions
        .iter()
        .map(|decision| {
            (
                decision.decision_id.as_str(),
                decision
                    .utility_by_world
                    .iter()
                    .map(|entry| (entry.world_id.as_str(), entry.utility_scaled))
                    .collect(),
            )
        })
        .collect();

    let mut expected_after = 0_i128;
    let mut outcome_decisions = Vec::new();

    for outcome_id in outcomes {
        let mut outcome_mass = 0_i128;
        for world in &worlds {
            let likelihood = *likelihoods
                .get(&(world.world_id.clone(), outcome_id.clone()))
                .unwrap_or(&0);
            let joint = checked_mul(
                i128::from(world.probability_ppm),
                i128::from(likelihood),
            );
            let Ok(joint) = joint else {
                return blocked_comparison(
                    action,
                    baseline_id,
                    baseline_numerator_ppm,
                    vec![blocker(Code::ArithmeticOverflow, "joint probability overflow")],
                );
            };
            let sum = checked_add(outcome_mass, joint);
            let Ok(sum) = sum else {
                return blocked_comparison(
                    action,
                    baseline_id,
                    baseline_numerator_ppm,
                    vec![blocker(Code::ArithmeticOverflow, "outcome mass accumulation overflow")],
                );
            };
            outcome_mass = sum;
        }
        if outcome_mass == 0 {
            // An impossible outcome contributes exactly zero to expected utility.
            continue;
        }

        let mut best: Option<(String, i128)> = None;
        for decision in &decisions {
            let mut joint_utility = 0_i128;
            let utility_map = &utility_maps[decision.decision_id.as_str()];
            for world in &worlds {
                let likelihood = *likelihoods
                    .get(&(world.world_id.clone(), outcome_id.clone()))
                    .unwrap_or(&0);
                let joint_probability = match checked_mul(
                    i128::from(world.probability_ppm),
                    i128::from(likelihood),
                ) {
                    Ok(value) => value,
                    Err(code) => {
                        return blocked_comparison(
                            action,
                            baseline_id,
                            baseline_numerator_ppm,
                            vec![blocker(code, "joint probability overflow")],
                        );
                    }
                };
                let utility = i128::from(utility_map[world.world_id.as_str()]);
                let term = match checked_mul(joint_probability, utility) {
                    Ok(value) => value,
                    Err(code) => {
                        return blocked_comparison(
                            action,
                            baseline_id,
                            baseline_numerator_ppm,
                            vec![blocker(code, "joint utility multiplication overflow")],
                        );
                    }
                };
                joint_utility = match checked_add(joint_utility, term) {
                    Ok(value) => value,
                    Err(code) => {
                        return blocked_comparison(
                            action,
                            baseline_id,
                            baseline_numerator_ppm,
                            vec![blocker(code, "joint utility accumulation overflow")],
                        );
                    }
                };
            }

            let replace = match &best {
                None => true,
                Some((best_id, best_utility)) => {
                    joint_utility > *best_utility
                        || (joint_utility == *best_utility && decision.decision_id < *best_id)
                }
            };
            if replace {
                best = Some((decision.decision_id.clone(), joint_utility));
            }
        }

        if let Some((decision_id, joint_utility_numerator)) = best {
            expected_after = match checked_add(expected_after, joint_utility_numerator) {
                Ok(value) => value,
                Err(code) => {
                    return blocked_comparison(
                        action,
                        baseline_id,
                        baseline_numerator_ppm,
                        vec![blocker(code, "expected utility after measurement overflow")],
                    );
                }
            };
            outcome_decisions.push(OutcomeDecisionV1 {
                outcome_id,
                decision_id,
                joint_utility_numerator,
            });
        }
    }

    let baseline_at_squared_denominator = match checked_mul(
        baseline_numerator_ppm,
        i128::from(PROBABILITY_SCALE_PPM),
    ) {
        Ok(value) => value,
        Err(code) => {
            return blocked_comparison(
                action,
                baseline_id,
                baseline_numerator_ppm,
                vec![blocker(code, "baseline denominator conversion overflow")],
            );
        }
    };
    let gross_evsi = match expected_after.checked_sub(baseline_at_squared_denominator) {
        Some(value) if value >= 0 => value,
        Some(_) => {
            return blocked_comparison(
                action,
                baseline_id,
                baseline_numerator_ppm,
                vec![blocker(
                    Code::NegativeGrossEvsiInvariant,
                    "expected utility after a free observation fell below the baseline",
                )],
            );
        }
        None => {
            return blocked_comparison(
                action,
                baseline_id,
                baseline_numerator_ppm,
                vec![blocker(Code::ArithmeticOverflow, "gross EVSI subtraction overflow")],
            );
        }
    };
    let cost_numerator = match checked_mul(
        i128::from(action.cost_utility_scaled),
        i128::from(EVSI_DENOMINATOR_PPM_SQUARED),
    ) {
        Ok(value) => value,
        Err(code) => {
            return blocked_comparison(
                action,
                baseline_id,
                baseline_numerator_ppm,
                vec![blocker(code, "measurement cost conversion overflow")],
            );
        }
    };
    let net_evsi = match gross_evsi.checked_sub(cost_numerator) {
        Some(value) => value,
        None => {
            return blocked_comparison(
                action,
                baseline_id,
                baseline_numerator_ppm,
                vec![blocker(Code::ArithmeticOverflow, "net EVSI subtraction overflow")],
            );
        }
    };

    let disposition = match action.probability_basis {
        ProbabilityBasisV1::SyntheticFixture => MeasurementDispositionV1::SyntheticFixtureOnly,
        ProbabilityBasisV1::CalibratedEmpirical | ProbabilityBasisV1::ReviewedExpertElicitation
            if net_evsi > 0 => MeasurementDispositionV1::PositiveNetValue,
        ProbabilityBasisV1::CalibratedEmpirical | ProbabilityBasisV1::ReviewedExpertElicitation =>
            MeasurementDispositionV1::NonPositiveNetValue,
        ProbabilityBasisV1::Unjustified => MeasurementDispositionV1::NotEstimable,
    };

    MeasurementComparisonV1 {
        action_id: action.action_id.clone(),
        probability_basis: action.probability_basis,
        disposition,
        baseline_decision_id: baseline_id.to_owned(),
        baseline_expected_utility_numerator_ppm: baseline_numerator_ppm,
        expected_utility_after_measurement_numerator_ppm_squared: Some(expected_after),
        gross_evsi_numerator_ppm_squared: Some(gross_evsi),
        measurement_cost_numerator_ppm_squared: Some(cost_numerator),
        net_evsi_numerator_ppm_squared: Some(net_evsi),
        outcome_decisions,
        blockers: Vec::new(),
    }
}

/// Evaluate a finite-state scenario using exact PPM integer arithmetic.
///
/// Scenario-wide structural errors are returned as blockers. Candidate-specific
/// hard blockers and malformed likelihoods remain attached to the corresponding
/// comparison and prevent that action from receiving a numerical score.
pub fn evaluate_value_of_information(
    scenario: &ValueOfInformationScenarioV1,
) -> Result<ValueOfInformationReportV1, Vec<MeasurementPlannerBlockerV1>> {
    let validation = validate_scenario(scenario);
    if !validation.is_empty() {
        return Err(validation);
    }

    let (baseline_decision_id, baseline_numerator) =
        baseline_utility(scenario).map_err(|issue| vec![issue])?;

    let mut measurement_comparisons: Vec<_> = scenario
        .measurements
        .iter()
        .map(|action| {
            compare_measurement(
                scenario,
                action,
                &baseline_decision_id,
                baseline_numerator,
            )
        })
        .collect();
    measurement_comparisons.sort_by(|left, right| left.action_id.cmp(&right.action_id));

    Ok(ValueOfInformationReportV1 {
        schema_version: MEASUREMENT_PLANNER_SCHEMA_VERSION,
        scenario_id: scenario.scenario_id.clone(),
        utility_scale: scenario.utility_scale,
        probability_scale_ppm: PROBABILITY_SCALE_PPM,
        evsi_denominator_ppm_squared: EVSI_DENOMINATOR_PPM_SQUARED,
        baseline_decision_id,
        baseline_expected_utility_numerator_ppm: baseline_numerator,
        measurement_comparisons,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn perfect_measurement(basis: ProbabilityBasisV1, cost: u64) -> MeasurementActionV1 {
        MeasurementActionV1 {
            action_id: "measure-weather".to_owned(),
            hard_feasible: true,
            hard_blockers: Vec::new(),
            cost_utility_scaled: cost,
            probability_basis: basis,
            likelihoods: vec![
                ObservationLikelihoodV1 {
                    world_id: "dry".to_owned(),
                    outcome_id: "dry-observed".to_owned(),
                    probability_ppm: PROBABILITY_SCALE_PPM,
                },
                ObservationLikelihoodV1 {
                    world_id: "dry".to_owned(),
                    outcome_id: "wet-observed".to_owned(),
                    probability_ppm: 0,
                },
                ObservationLikelihoodV1 {
                    world_id: "wet".to_owned(),
                    outcome_id: "dry-observed".to_owned(),
                    probability_ppm: 0,
                },
                ObservationLikelihoodV1 {
                    world_id: "wet".to_owned(),
                    outcome_id: "wet-observed".to_owned(),
                    probability_ppm: PROBABILITY_SCALE_PPM,
                },
            ],
        }
    }

    fn base_scenario() -> ValueOfInformationScenarioV1 {
        ValueOfInformationScenarioV1 {
            schema_version: MEASUREMENT_PLANNER_SCHEMA_VERSION,
            scenario_id: "fixture:two-weather-worlds".to_owned(),
            utility_scale: 1,
            worlds: vec![
                WorldProbabilityV1 {
                    world_id: "dry".to_owned(),
                    probability_ppm: 500_000,
                },
                WorldProbabilityV1 {
                    world_id: "wet".to_owned(),
                    probability_ppm: 500_000,
                },
            ],
            decisions: vec![
                DecisionOptionV1 {
                    decision_id: "a-storage".to_owned(),
                    hard_feasible: true,
                    hard_blockers: Vec::new(),
                    utility_by_world: vec![
                        DecisionUtilityV1 {
                            world_id: "dry".to_owned(),
                            utility_scaled: 100,
                        },
                        DecisionUtilityV1 {
                            world_id: "wet".to_owned(),
                            utility_scaled: 0,
                        },
                    ],
                },
                DecisionOptionV1 {
                    decision_id: "b-shade".to_owned(),
                    hard_feasible: true,
                    hard_blockers: Vec::new(),
                    utility_by_world: vec![
                        DecisionUtilityV1 {
                            world_id: "dry".to_owned(),
                            utility_scaled: 0,
                        },
                        DecisionUtilityV1 {
                            world_id: "wet".to_owned(),
                            utility_scaled: 100,
                        },
                    ],
                },
            ],
            measurements: vec![perfect_measurement(
                ProbabilityBasisV1::CalibratedEmpirical,
                10,
            )],
        }
    }

    #[test]
    fn perfect_information_has_hand_calculated_positive_net_value() {
        let report = evaluate_value_of_information(&base_scenario()).expect("valid scenario");
        assert_eq!(report.baseline_decision_id, "a-storage");
        assert_eq!(report.baseline_expected_utility_numerator_ppm, 50_000_000);
        let comparison = &report.measurement_comparisons[0];
        assert_eq!(comparison.gross_evsi_numerator_ppm_squared, Some(50_000_000_000_000));
        assert_eq!(comparison.measurement_cost_numerator_ppm_squared, Some(10_000_000_000_000));
        assert_eq!(comparison.net_evsi_numerator_ppm_squared, Some(40_000_000_000_000));
        assert_eq!(
            comparison.expected_utility_after_measurement_numerator_ppm_squared,
            Some(100_000_000_000_000)
        );
        assert_eq!(comparison.disposition, MeasurementDispositionV1::PositiveNetValue);
        assert_eq!(comparison.outcome_decisions.len(), 2);
    }

    #[test]
    fn uninformative_measurement_has_zero_gross_evsi() {
        let mut scenario = base_scenario();
        let action = &mut scenario.measurements[0];
        action.cost_utility_scaled = 0;
        action.likelihoods = vec![
            ObservationLikelihoodV1 { world_id: "dry".into(), outcome_id: "heads".into(), probability_ppm: 500_000 },
            ObservationLikelihoodV1 { world_id: "dry".into(), outcome_id: "tails".into(), probability_ppm: 500_000 },
            ObservationLikelihoodV1 { world_id: "wet".into(), outcome_id: "heads".into(), probability_ppm: 500_000 },
            ObservationLikelihoodV1 { world_id: "wet".into(), outcome_id: "tails".into(), probability_ppm: 500_000 },
        ];
        let report = evaluate_value_of_information(&scenario).expect("valid scenario");
        let comparison = &report.measurement_comparisons[0];
        assert_eq!(comparison.gross_evsi_numerator_ppm_squared, Some(0));
        assert_eq!(comparison.net_evsi_numerator_ppm_squared, Some(0));
        assert_eq!(comparison.disposition, MeasurementDispositionV1::NonPositiveNetValue);
    }

    #[test]
    fn unjustified_probabilities_produce_no_numeric_evsi() {
        let mut scenario = base_scenario();
        scenario.measurements[0].probability_basis = ProbabilityBasisV1::Unjustified;
        let report = evaluate_value_of_information(&scenario).expect("valid scenario");
        let comparison = &report.measurement_comparisons[0];
        assert_eq!(comparison.disposition, MeasurementDispositionV1::NotEstimable);
        assert_eq!(comparison.gross_evsi_numerator_ppm_squared, None);
        assert_eq!(comparison.net_evsi_numerator_ppm_squared, None);
        assert!(comparison.blockers.iter().any(|item| item.code == MeasurementPlannerBlockerCodeV1::UnjustifiedProbabilityModel));
    }

    #[test]
    fn hard_blocked_measurement_never_receives_a_score() {
        let mut scenario = base_scenario();
        scenario.measurements[0].hard_feasible = false;
        scenario.measurements[0].hard_blockers = vec!["consent not granted".to_owned()];
        scenario.measurements[0].cost_utility_scaled = 0;
        let report = evaluate_value_of_information(&scenario).expect("scenario with blocked action is valid");
        let comparison = &report.measurement_comparisons[0];
        assert_eq!(comparison.disposition, MeasurementDispositionV1::Blocked);
        assert_eq!(comparison.gross_evsi_numerator_ppm_squared, None);
        assert!(comparison.blockers.iter().any(|item| item.code == MeasurementPlannerBlockerCodeV1::MeasurementHardBlocked));
    }

    #[test]
    fn invalid_prior_normalization_rejects_the_scenario() {
        let mut scenario = base_scenario();
        scenario.worlds[0].probability_ppm = 499_999;
        let error = evaluate_value_of_information(&scenario).expect_err("priors must normalize");
        assert!(error.iter().any(|item| item.code == MeasurementPlannerBlockerCodeV1::PriorNotNormalized));
    }

    #[test]
    fn malformed_likelihood_matrix_blocks_only_that_action() {
        let mut scenario = base_scenario();
        scenario.measurements[0].likelihoods[0].probability_ppm = 900_000;
        let report = evaluate_value_of_information(&scenario).expect("global scenario remains valid");
        let comparison = &report.measurement_comparisons[0];
        assert_eq!(comparison.disposition, MeasurementDispositionV1::Blocked);
        assert_eq!(comparison.gross_evsi_numerator_ppm_squared, None);
        assert!(comparison.blockers.iter().any(|item| item.code == MeasurementPlannerBlockerCodeV1::LikelihoodRowNotNormalized));
    }

    #[test]
    fn output_is_invariant_to_order_of_semantically_unordered_inputs() {
        let scenario = base_scenario();
        let expected = evaluate_value_of_information(&scenario).expect("valid scenario");
        let mut permuted = scenario.clone();
        permuted.worlds.reverse();
        permuted.decisions.reverse();
        for decision in &mut permuted.decisions {
            decision.utility_by_world.reverse();
        }
        permuted.measurements.reverse();
        for action in &mut permuted.measurements {
            action.likelihoods.reverse();
        }
        let actual = evaluate_value_of_information(&permuted).expect("permuted scenario valid");
        assert_eq!(actual, expected);
    }

    #[test]
    fn duplicate_world_and_decision_ids_are_rejected() {
        let mut scenario = base_scenario();
        scenario.worlds.push(scenario.worlds[0].clone());
        scenario.decisions[1].decision_id = scenario.decisions[0].decision_id.clone();
        let errors = evaluate_value_of_information(&scenario).expect_err("duplicate IDs are invalid");
        assert!(errors.iter().any(|item| item.code == MeasurementPlannerBlockerCodeV1::DuplicateWorldId));
        assert!(errors.iter().any(|item| item.code == MeasurementPlannerBlockerCodeV1::DuplicateDecisionId));
    }

    #[test]
    fn synthetic_probability_model_never_looks_like_a_real_recommendation() {
        let mut scenario = base_scenario();
        scenario.measurements[0].probability_basis = ProbabilityBasisV1::SyntheticFixture;
        let report = evaluate_value_of_information(&scenario).expect("fixture runs");
        assert_eq!(
            report.measurement_comparisons[0].disposition,
            MeasurementDispositionV1::SyntheticFixtureOnly
        );
        assert!(report.measurement_comparisons[0].net_evsi_numerator_ppm_squared.is_some());
    }

    #[test]
    fn wide_utility_and_numerator_values_are_wire_strings() {
        let mut scenario = base_scenario();
        scenario.decisions[0].utility_by_world[0].utility_scaled = i64::MAX;
        let report = evaluate_value_of_information(&scenario).expect("extreme integer remains exact");
        let wire = serde_json::to_string(&report).expect("report serializes");
        assert!(wire.contains(r#""baseline_expected_utility_numerator_ppm":"#));
        assert!(wire.contains(r#""utility_scale":"1""#));
        assert!(wire.contains(r#""baseline_expected_utility_numerator_ppm":"4611686018427387903500000""#));
        let round_trip: ValueOfInformationReportV1 = serde_json::from_str(&wire).expect("report roundtrips");
        assert_eq!(round_trip, report);
    }

    #[test]
    fn one_hundred_seeded_two_world_cases_match_closed_form_oracle() {
        for seed in 0_u32..100 {
            let mut scenario = base_scenario();
            let dry_probability = (seed * 7_919 % 999_998) + 1;
            let wet_probability = PROBABILITY_SCALE_PPM - dry_probability;
            let dry_utility = i64::from((seed * 31 % 500) + 1);
            let wet_utility = i64::from((seed * 47 % 600) + 1);

            scenario.worlds[0].probability_ppm = dry_probability;
            scenario.worlds[1].probability_ppm = wet_probability;
            scenario.decisions[0].utility_by_world[0].utility_scaled = dry_utility;
            scenario.decisions[0].utility_by_world[1].utility_scaled = 0;
            scenario.decisions[1].utility_by_world[0].utility_scaled = 0;
            scenario.decisions[1].utility_by_world[1].utility_scaled = wet_utility;
            scenario.measurements[0] = perfect_measurement(ProbabilityBasisV1::SyntheticFixture, 0);

            let report = evaluate_value_of_information(&scenario).expect("generated valid case");
            let dry_joint = i128::from(dry_probability) * i128::from(dry_utility);
            let wet_joint = i128::from(wet_probability) * i128::from(wet_utility);
            let baseline = dry_joint.max(wet_joint);
            let gross = dry_joint.min(wet_joint) * i128::from(PROBABILITY_SCALE_PPM);
            let comparison = &report.measurement_comparisons[0];

            assert_eq!(
                report.baseline_expected_utility_numerator_ppm,
                baseline,
                "baseline mismatch at synthetic seed {seed}"
            );
            assert_eq!(
                comparison.expected_utility_after_measurement_numerator_ppm_squared,
                Some((dry_joint + wet_joint) * i128::from(PROBABILITY_SCALE_PPM)),
                "expected-after mismatch at synthetic seed {seed}"
            );
            assert_eq!(
                comparison.gross_evsi_numerator_ppm_squared,
                Some(gross),
                "gross EVSI mismatch at synthetic seed {seed}"
            );
            assert_eq!(
                comparison.net_evsi_numerator_ppm_squared,
                Some(gross),
                "zero-cost net EVSI mismatch at synthetic seed {seed}"
            );
            assert_eq!(
                comparison.disposition,
                MeasurementDispositionV1::SyntheticFixtureOnly
            );
        }
    }

    #[test]
    fn infeasible_only_scenario_is_rejected_not_scored() {
        let mut scenario = base_scenario();
        for decision in &mut scenario.decisions {
            decision.hard_feasible = false;
            decision.hard_blockers = vec!["land permission absent".to_owned()];
        }
        let errors = evaluate_value_of_information(&scenario).expect_err("no feasible action");
        assert!(errors.iter().any(|item| item.code == MeasurementPlannerBlockerCodeV1::NoFeasibleDecision));
    }
}
