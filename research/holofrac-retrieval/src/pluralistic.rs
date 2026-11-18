//! Consent-first, multi-stakeholder decision support for HoloFrac.
//!
//! This synthetic benchmark preserves individual outcomes, vetoes and Pareto
//! alternatives. It does not declare any aggregation rule universally fair.
//! Preference/consent records are explicit participant inputs, not inferred
//! from cameras, biometrics, demographics or behavior.

use std::collections::{BTreeMap, BTreeSet, HashSet};

pub const PREFERENCE_SCHEMA_VERSION: u32 = 1;
pub const CRITERION_COUNT: usize = 8;
pub const PREFERENCE_GENERATOR: &str = "holofrac-pluralistic-decision-fixture-v0";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DecisionCriterion {
    ThermalComfort,
    AirQuality,
    Daylight,
    AcousticComfort,
    Accessibility,
    Privacy,
    EnergyEfficiency,
    Adaptability,
}

impl DecisionCriterion {
    pub const ALL: [Self; CRITERION_COUNT] = [
        Self::ThermalComfort,
        Self::AirQuality,
        Self::Daylight,
        Self::AcousticComfort,
        Self::Accessibility,
        Self::Privacy,
        Self::EnergyEfficiency,
        Self::Adaptability,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ThermalComfort => "thermal_comfort",
            Self::AirQuality => "air_quality",
            Self::Daylight => "daylight",
            Self::AcousticComfort => "acoustic_comfort",
            Self::Accessibility => "accessibility",
            Self::Privacy => "privacy",
            Self::EnergyEfficiency => "energy_efficiency",
            Self::Adaptability => "adaptability",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConsentScope {
    /// Preference can be used only in a participant-local evaluation.
    LocalOnly,
    /// Preference may contribute to an aggregate, but the participant ID
    /// must not appear in shared result rows.
    AnonymousAggregate,
    /// Participant explicitly authorizes named/shared evaluation.
    NamedAggregate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConsentStatus {
    NotGranted,
    Granted,
    Withdrawn,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreferenceEvent {
    pub event_id: u64,
    pub stakeholder_id: String,
    pub revision: u32,
    pub previous_event_id: Option<u64>,
    /// When this preference/consent decision applies in the participant's context.
    pub effective_at_micros: i64,
    /// When this revision became knowable to the decision system.
    pub recorded_at_micros: i64,
    pub consent_scope: ConsentScope,
    pub consent_status: ConsentStatus,
    /// Explicit participant-supplied weights; higher outcome is better.
    /// These weights are used only for this participant's own satisfaction.
    pub weights: [f64; CRITERION_COUNT],
    /// Optional hard minimum for each criterion, set explicitly by the participant.
    pub veto_thresholds: [Option<f64>; CRITERION_COUNT],
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PreferenceLedger {
    /// Append-only per-stakeholder revision events. Events may be interleaved
    /// across stakeholders; each individual history must remain revision ordered.
    pub events: Vec<PreferenceEvent>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreferenceProfile {
    pub stakeholder_id: String,
    pub revision: u32,
    pub consent_scope: ConsentScope,
    pub consent_status: ConsentStatus,
    pub weights: [f64; CRITERION_COUNT],
    pub veto_thresholds: [Option<f64>; CRITERION_COUNT],
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreferenceSnapshot {
    pub decision_cutoff_micros: i64,
    pub visible_event_count: usize,
    /// Latest known profile per participant, sorted by stable stakeholder ID.
    pub profiles: Vec<PreferenceProfile>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DesignAlternative {
    pub alternative_id: String,
    /// Normalized outcome vector in [0, 1]. Every criterion is oriented so
    /// larger is better (e.g. energy efficiency, not raw energy consumption).
    pub outcomes: [f64; CRITERION_COUNT],
    /// Independently evaluated Symtropy hard-constraint result.
    pub hard_constraints_satisfied: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StakeholderSatisfaction {
    /// None for anonymous aggregate consent; never infer a participant identity.
    pub stakeholder_id: Option<String>,
    pub satisfaction: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VetoViolation {
    /// None when the vetoing participant authorized anonymous aggregation.
    pub stakeholder_id: Option<String>,
    pub criterion: DecisionCriterion,
    pub observed_outcome: f64,
    pub minimum_outcome: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AlternativeAssessment {
    pub alternative_id: String,
    pub outcomes: [f64; CRITERION_COUNT],
    pub per_stakeholder_satisfaction: Vec<StakeholderSatisfaction>,
    /// Equal-participant mean is reported as a comparison statistic only.
    /// It is not the default decision rule.
    pub mean_satisfaction: f64,
    pub minimum_satisfaction: f64,
    /// Sorted ascending so maximin/leximin comparisons expose the least-satisfied first.
    pub leximin_vector: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RejectedAlternative {
    pub alternative_id: String,
    pub hard_constraints_satisfied: bool,
    pub veto_violations: Vec<VetoViolation>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GroupDecisionReport {
    pub decision_cutoff_micros: i64,
    pub active_stakeholder_count: usize,
    pub excluded_profile_count: usize,
    pub assessments: Vec<AlternativeAssessment>,
    pub rejected_alternatives: Vec<RejectedAlternative>,
    /// Pareto frontier is computed over the original declared outcome vector,
    /// not a scalarized group score.
    pub pareto_frontier_ids: Vec<String>,
}

impl GroupDecisionReport {
    /// Explicit comparison baseline: equal-participant mean satisfaction.
    pub fn rank_by_mean(&self) -> Vec<String> {
        let mut rows: Vec<_> = self.assessments.iter().collect();
        rows.sort_by(|left, right| {
            right
                .mean_satisfaction
                .total_cmp(&left.mean_satisfaction)
                .then_with(|| left.alternative_id.cmp(&right.alternative_id))
        });
        rows.into_iter()
            .map(|assessment| assessment.alternative_id.clone())
            .collect()
    }

    /// Maximize the least-satisfied participant; ties use group mean then ID.
    pub fn rank_by_maximin(&self) -> Vec<String> {
        let mut rows: Vec<_> = self.assessments.iter().collect();
        rows.sort_by(|left, right| {
            right
                .minimum_satisfaction
                .total_cmp(&left.minimum_satisfaction)
                .then_with(|| {
                    right
                        .mean_satisfaction
                        .total_cmp(&left.mean_satisfaction)
                })
                .then_with(|| left.alternative_id.cmp(&right.alternative_id))
        });
        rows.into_iter()
            .map(|assessment| assessment.alternative_id.clone())
            .collect()
    }

    /// Leximin: compare sorted individual satisfaction from the lowest upwards.
    /// No participant is silently replaced by the average.
    pub fn rank_by_leximin(&self) -> Vec<String> {
        let mut rows: Vec<_> = self.assessments.iter().collect();
        rows.sort_by(|left, right| {
            for (left_value, right_value) in left
                .leximin_vector
                .iter()
                .zip(right.leximin_vector.iter())
            {
                let ordering = right_value.total_cmp(left_value);
                if ordering != std::cmp::Ordering::Equal {
                    return ordering;
                }
            }
            left.alternative_id.cmp(&right.alternative_id)
        });
        rows.into_iter()
            .map(|assessment| assessment.alternative_id.clone())
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PreferenceError {
    EmptyStakeholderId,
    EmptyAlternativeId,
    DuplicateEventId(u64),
    DuplicateAlternative(String),
    DuplicateStakeholder(String),
    RevisionDiscontinuity(String),
    ParentEventMismatch(String),
    InvalidPreferenceChronology(String),
    InvalidPreferenceWeights(String),
    InvalidVetoThreshold(String),
    InvalidOutcome(String),
    EmptyAlternativeSet,
    NoParticipatingStakeholders,
}

impl std::fmt::Display for PreferenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyStakeholderId => f.write_str("stakeholder ID must not be empty"),
            Self::EmptyAlternativeId => f.write_str("alternative ID must not be empty"),
            Self::DuplicateEventId(id) => write!(f, "duplicate preference event ID: {id}"),
            Self::DuplicateAlternative(id) => write!(f, "duplicate alternative ID: {id}"),
            Self::DuplicateStakeholder(id) => write!(f, "duplicate stakeholder ID: {id}"),
            Self::RevisionDiscontinuity(id) => {
                write!(f, "preference revisions are discontinuous for stakeholder {id}")
            }
            Self::ParentEventMismatch(id) => {
                write!(f, "preference event parent is invalid for stakeholder {id}")
            }
            Self::InvalidPreferenceChronology(id) => {
                write!(f, "preference event chronology is invalid for stakeholder {id}")
            }
            Self::InvalidPreferenceWeights(id) => {
                write!(f, "preference weights must be finite, non-negative and sum to one: {id}")
            }
            Self::InvalidVetoThreshold(id) => {
                write!(f, "veto thresholds must be finite and in [0, 1]: {id}")
            }
            Self::InvalidOutcome(id) => {
                write!(f, "alternative has an invalid or non-finite outcome vector: {id}")
            }
            Self::EmptyAlternativeSet => f.write_str("decision requires at least one alternative"),
            Self::NoParticipatingStakeholders => {
                f.write_str("no participants authorized anonymous or named group aggregation")
            }
        }
    }
}

impl std::error::Error for PreferenceError {}

#[derive(Clone, Debug)]
struct StakeholderProgress {
    revision: u32,
    last_event_id: u64,
    last_effective_at_micros: i64,
    last_recorded_at_micros: i64,
}

fn validate_preference_payload(event: &PreferenceEvent) -> Result<(), PreferenceError> {
    match event.consent_status {
        ConsentStatus::Granted => {
            let mut sum = 0.0;
            for weight in event.weights {
                if !weight.is_finite() || weight < 0.0 {
                    return Err(PreferenceError::InvalidPreferenceWeights(
                        event.stakeholder_id.clone(),
                    ));
                }
                sum += weight;
            }
            if (sum - 1.0).abs() > 1.0e-9 {
                return Err(PreferenceError::InvalidPreferenceWeights(
                    event.stakeholder_id.clone(),
                ));
            }

            for threshold in event.veto_thresholds.iter().flatten() {
                if !threshold.is_finite() || !(0.0..=1.0).contains(threshold) {
                    return Err(PreferenceError::InvalidVetoThreshold(
                        event.stakeholder_id.clone(),
                    ));
                }
            }
        }
        ConsentStatus::NotGranted | ConsentStatus::Withdrawn => {
            if event.weights.iter().any(|weight| *weight != 0.0)
                || event.veto_thresholds.iter().any(Option::is_some)
            {
                return Err(PreferenceError::InvalidPreferenceWeights(
                    event.stakeholder_id.clone(),
                ));
            }
        }
    }

    Ok(())
}

impl PreferenceLedger {
    pub fn validate(&self) -> Result<(), PreferenceError> {
        let mut event_ids = HashSet::with_capacity(self.events.len());
        let mut progress: BTreeMap<String, StakeholderProgress> = BTreeMap::new();

        for event in &self.events {
            if event.stakeholder_id.trim().is_empty() {
                return Err(PreferenceError::EmptyStakeholderId);
            }
            if !event_ids.insert(event.event_id) {
                return Err(PreferenceError::DuplicateEventId(event.event_id));
            }
            validate_preference_payload(event)?;

            if let Some(previous) = progress.get(&event.stakeholder_id) {
                if event.revision != previous.revision + 1 {
                    return Err(PreferenceError::RevisionDiscontinuity(
                        event.stakeholder_id.clone(),
                    ));
                }
                if event.previous_event_id != Some(previous.last_event_id) {
                    return Err(PreferenceError::ParentEventMismatch(
                        event.stakeholder_id.clone(),
                    ));
                }
                if event.effective_at_micros < previous.last_effective_at_micros
                    || event.recorded_at_micros < previous.last_recorded_at_micros
                {
                    return Err(PreferenceError::InvalidPreferenceChronology(
                        event.stakeholder_id.clone(),
                    ));
                }
            } else if event.revision != 1 || event.previous_event_id.is_some() {
                return Err(PreferenceError::RevisionDiscontinuity(
                    event.stakeholder_id.clone(),
                ));
            }

            progress.insert(
                event.stakeholder_id.clone(),
                StakeholderProgress {
                    revision: event.revision,
                    last_event_id: event.event_id,
                    last_effective_at_micros: event.effective_at_micros,
                    last_recorded_at_micros: event.recorded_at_micros,
                },
            );
        }

        Ok(())
    }

    /// Reconstruct exactly the preference/consent revisions known at a
    /// historical decision cutoff. Late-recorded revocations do not leak
    /// backward into a decision made before they became known.
    pub fn snapshot_as_of(
        &self,
        decision_cutoff_micros: i64,
    ) -> Result<PreferenceSnapshot, PreferenceError> {
        self.validate()?;

        let mut latest: BTreeMap<String, &PreferenceEvent> = BTreeMap::new();
        let mut visible_event_count = 0;

        for event in self
            .events
            .iter()
            .filter(|event| {
                event.effective_at_micros <= decision_cutoff_micros
                    && event.recorded_at_micros <= decision_cutoff_micros
            })
        {
            visible_event_count += 1;
            let replace = latest
                .get(&event.stakeholder_id)
                .is_none_or(|previous| event.revision > previous.revision);
            if replace {
                latest.insert(event.stakeholder_id.clone(), event);
            }
        }

        let profiles = latest
            .values()
            .map(|event| PreferenceProfile {
                stakeholder_id: event.stakeholder_id.clone(),
                revision: event.revision,
                consent_scope: event.consent_scope,
                consent_status: event.consent_status,
                weights: event.weights,
                veto_thresholds: event.veto_thresholds,
            })
            .collect();

        Ok(PreferenceSnapshot {
            decision_cutoff_micros,
            visible_event_count,
            profiles,
        })
    }
}

fn weighted_satisfaction(
    weights: &[f64; CRITERION_COUNT],
    outcomes: &[f64; CRITERION_COUNT],
) -> f64 {
    weights
        .iter()
        .zip(outcomes.iter())
        .map(|(weight, outcome)| weight * outcome)
        .sum()
}

fn dominates(
    left: &[f64; CRITERION_COUNT],
    right: &[f64; CRITERION_COUNT],
) -> bool {
    let no_worse = left.iter().zip(right.iter()).all(|(a, b)| a >= b);
    let strictly_better = left.iter().zip(right.iter()).any(|(a, b)| a > b);
    no_worse && strictly_better
}

pub fn evaluate_group(
    alternatives: &[DesignAlternative],
    snapshot: &PreferenceSnapshot,
) -> Result<GroupDecisionReport, PreferenceError> {
    if alternatives.is_empty() {
        return Err(PreferenceError::EmptyAlternativeSet);
    }

    let mut alternative_ids = BTreeSet::new();
    for alternative in alternatives {
        if alternative.alternative_id.trim().is_empty() {
            return Err(PreferenceError::EmptyAlternativeId);
        }
        if !alternative_ids.insert(alternative.alternative_id.clone()) {
            return Err(PreferenceError::DuplicateAlternative(
                alternative.alternative_id.clone(),
            ));
        }
        if alternative
            .outcomes
            .iter()
            .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
        {
            return Err(PreferenceError::InvalidOutcome(
                alternative.alternative_id.clone(),
            ));
        }
    }

    let mut stakeholder_ids = BTreeSet::new();
    let mut active_profiles = Vec::new();
    let mut excluded_profile_count = 0;

    for profile in &snapshot.profiles {
        if profile.stakeholder_id.trim().is_empty() {
            return Err(PreferenceError::EmptyStakeholderId);
        }
        if !stakeholder_ids.insert(profile.stakeholder_id.clone()) {
            return Err(PreferenceError::DuplicateStakeholder(
                profile.stakeholder_id.clone(),
            ));
        }

        if profile.consent_status != ConsentStatus::Granted
            || profile.consent_scope == ConsentScope::LocalOnly
        {
            excluded_profile_count += 1;
            continue;
        }

        let mut sum = 0.0;
        for weight in profile.weights {
            if !weight.is_finite() || weight < 0.0 {
                return Err(PreferenceError::InvalidPreferenceWeights(
                    profile.stakeholder_id.clone(),
                ));
            }
            sum += weight;
        }
        if (sum - 1.0).abs() > 1.0e-9 {
            return Err(PreferenceError::InvalidPreferenceWeights(
                profile.stakeholder_id.clone(),
            ));
        }
        for threshold in profile.veto_thresholds.iter().flatten() {
            if !threshold.is_finite() || !(0.0..=1.0).contains(threshold) {
                return Err(PreferenceError::InvalidVetoThreshold(
                    profile.stakeholder_id.clone(),
                ));
            }
        }
        active_profiles.push(profile);
    }

    if active_profiles.is_empty() {
        return Err(PreferenceError::NoParticipatingStakeholders);
    }

    let mut assessments = Vec::new();
    let mut rejected_alternatives = Vec::new();

    for alternative in alternatives {
        let mut veto_violations = Vec::new();
        for profile in &active_profiles {
            for (index, threshold) in profile.veto_thresholds.iter().enumerate() {
                if let Some(minimum_outcome) = threshold {
                    let observed_outcome = alternative.outcomes[index];
                    if observed_outcome < *minimum_outcome {
                        let stakeholder_id = match profile.consent_scope {
                            ConsentScope::AnonymousAggregate => None,
                            ConsentScope::NamedAggregate => {
                                Some(profile.stakeholder_id.clone())
                            }
                            ConsentScope::LocalOnly => None,
                        };
                        veto_violations.push(VetoViolation {
                            stakeholder_id,
                            criterion: DecisionCriterion::ALL[index],
                            observed_outcome,
                            minimum_outcome: *minimum_outcome,
                        });
                    }
                }
            }
        }

        if !alternative.hard_constraints_satisfied || !veto_violations.is_empty() {
            rejected_alternatives.push(RejectedAlternative {
                alternative_id: alternative.alternative_id.clone(),
                hard_constraints_satisfied: alternative.hard_constraints_satisfied,
                veto_violations,
            });
            continue;
        }

        let mut individual = Vec::with_capacity(active_profiles.len());
        let mut leximin_vector = Vec::with_capacity(active_profiles.len());

        for profile in &active_profiles {
            let satisfaction = weighted_satisfaction(&profile.weights, &alternative.outcomes);
            let stakeholder_id = match profile.consent_scope {
                ConsentScope::AnonymousAggregate => None,
                ConsentScope::NamedAggregate => Some(profile.stakeholder_id.clone()),
                ConsentScope::LocalOnly => None,
            };
            individual.push(StakeholderSatisfaction {
                stakeholder_id,
                satisfaction,
            });
            leximin_vector.push(satisfaction);
        }

        leximin_vector.sort_by(f64::total_cmp);
        let minimum_satisfaction = *leximin_vector
            .first()
            .expect("active profiles were checked to be non-empty");
        let mean_satisfaction =
            leximin_vector.iter().sum::<f64>() / leximin_vector.len() as f64;

        assessments.push(AlternativeAssessment {
            alternative_id: alternative.alternative_id.clone(),
            outcomes: alternative.outcomes,
            per_stakeholder_satisfaction: individual,
            mean_satisfaction,
            minimum_satisfaction,
            leximin_vector,
        });
    }

    let mut pareto_frontier_ids = Vec::new();
    for candidate in &assessments {
        let dominated = assessments.iter().any(|other| {
            other.alternative_id != candidate.alternative_id
                && dominates(&other.outcomes, &candidate.outcomes)
        });
        if !dominated {
            pareto_frontier_ids.push(candidate.alternative_id.clone());
        }
    }
    pareto_frontier_ids.sort();

    Ok(GroupDecisionReport {
        decision_cutoff_micros: snapshot.decision_cutoff_micros,
        active_stakeholder_count: active_profiles.len(),
        excluded_profile_count,
        assessments,
        rejected_alternatives,
        pareto_frontier_ids,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn granted_event(
        stakeholder_id: &str,
        event_id: u64,
        revision: u32,
        previous_event_id: Option<u64>,
        recorded_at_micros: i64,
        scope: ConsentScope,
        weights: [f64; CRITERION_COUNT],
        veto_thresholds: [Option<f64>; CRITERION_COUNT],
    ) -> PreferenceEvent {
        PreferenceEvent {
            event_id,
            stakeholder_id: stakeholder_id.to_owned(),
            revision,
            previous_event_id,
            effective_at_micros: recorded_at_micros,
            recorded_at_micros,
            consent_scope: scope,
            consent_status: ConsentStatus::Granted,
            weights,
            veto_thresholds,
        }
    }

    fn balanced_weights(index: usize) -> [f64; CRITERION_COUNT] {
        let mut weights = [0.0; CRITERION_COUNT];
        weights[index] = 1.0;
        weights
    }

    fn alternative(id: &str, first: f64, second: f64, third: f64) -> DesignAlternative {
        let mut outcomes = [0.5; CRITERION_COUNT];
        outcomes[0] = first;
        outcomes[1] = second;
        outcomes[2] = third;
        DesignAlternative {
            alternative_id: id.to_owned(),
            outcomes,
            hard_constraints_satisfied: true,
        }
    }

    #[test]
    fn pareto_frontier_preserves_non_dominated_tradeoffs() {
        let frontier = PreferenceLedger {
            events: vec![granted_event(
                "resident-a",
                1,
                1,
                None,
                10,
                ConsentScope::NamedAggregate,
                balanced_weights(0),
                [None; CRITERION_COUNT],
            )],
        }
        .snapshot_as_of(10)
        .unwrap();

        let report = evaluate_group(
            &[
                alternative("a", 0.9, 0.4, 0.6),
                alternative("b", 0.4, 0.9, 0.6),
                alternative("dominated", 0.3, 0.3, 0.3),
            ],
            &frontier,
        )
        .unwrap();

        assert_eq!(
            report.pareto_frontier_ids,
            vec!["a".to_owned(), "b".to_owned()]
        );
    }

    #[test]
    fn one_participant_veto_rejects_the_candidate_and_explains_why() {
        let mut vetoes = [None; CRITERION_COUNT];
        vetoes[5] = Some(0.8);
        let snapshot = PreferenceLedger {
            events: vec![granted_event(
                "resident-a",
                1,
                1,
                None,
                10,
                ConsentScope::NamedAggregate,
                balanced_weights(5),
                vetoes,
            )],
        }
        .snapshot_as_of(10)
        .unwrap();

        let mut private_enough = alternative("private-enough", 0.7, 0.7, 0.7);
        private_enough.outcomes[5] = 0.9;
        let mut privacy_violating = alternative("privacy-violating", 0.9, 0.9, 0.9);
        privacy_violating.outcomes[5] = 0.5;

        let report = evaluate_group(&[private_enough, privacy_violating], &snapshot).unwrap();

        assert_eq!(report.assessments.len(), 1);
        assert_eq!(
            report.assessments[0].alternative_id,
            "private-enough"
        );
        assert_eq!(report.rejected_alternatives.len(), 1);
        assert_eq!(
            report.rejected_alternatives[0].veto_violations[0].criterion,
            DecisionCriterion::Privacy
        );
        assert_eq!(
            report.rejected_alternatives[0].veto_violations[0].stakeholder_id,
            Some("resident-a".to_owned())
        );
    }

    #[test]
    fn leximin_exposes_minority_loss_that_a_mean_score_can_hide() {
        let ledger = PreferenceLedger {
            events: vec![
                granted_event(
                    "participant-a",
                    1,
                    1,
                    None,
                    10,
                    ConsentScope::AnonymousAggregate,
                    balanced_weights(0),
                    [None; CRITERION_COUNT],
                ),
                granted_event(
                    "participant-b",
                    2,
                    1,
                    None,
                    10,
                    ConsentScope::AnonymousAggregate,
                    balanced_weights(1),
                    [None; CRITERION_COUNT],
                ),
                granted_event(
                    "participant-c",
                    3,
                    1,
                    None,
                    10,
                    ConsentScope::AnonymousAggregate,
                    balanced_weights(2),
                    [None; CRITERION_COUNT],
                ),
            ],
        };
        let snapshot = ledger.snapshot_as_of(10).unwrap();
        let report = evaluate_group(
            &[
                alternative("majority-favoured", 0.4, 1.0, 1.0),
                alternative("balanced", 0.7, 0.7, 0.7),
            ],
            &snapshot,
        )
        .unwrap();

        assert_eq!(
            report.rank_by_mean(),
            vec!["majority-favoured".to_owned(), "balanced".to_owned()]
        );
        assert_eq!(
            report.rank_by_maximin(),
            vec!["balanced".to_owned(), "majority-favoured".to_owned()]
        );
        assert_eq!(
            report.rank_by_leximin(),
            vec!["balanced".to_owned(), "majority-favoured".to_owned()]
        );
        assert!(
            report
                .assessments
                .iter()
                .all(|assessment| assessment
                    .per_stakeholder_satisfaction
                    .iter()
                    .all(|stakeholder| stakeholder.stakeholder_id.is_none()))
        );
    }

    #[test]
    fn local_only_and_withdrawn_profiles_are_excluded_from_shared_aggregation() {
        let withdrawn_weights = [0.0; CRITERION_COUNT];
        let mut events = vec![
            granted_event(
                "local-person",
                1,
                1,
                None,
                10,
                ConsentScope::LocalOnly,
                balanced_weights(0),
                [None; CRITERION_COUNT],
            ),
            granted_event(
                "anonymous-person",
                2,
                1,
                None,
                10,
                ConsentScope::AnonymousAggregate,
                balanced_weights(1),
                [None; CRITERION_COUNT],
            ),
        ];
        events.push(PreferenceEvent {
            event_id: 3,
            stakeholder_id: "withdrawn-person".to_owned(),
            revision: 1,
            previous_event_id: None,
            effective_at_micros: 10,
            recorded_at_micros: 10,
            consent_scope: ConsentScope::NamedAggregate,
            consent_status: ConsentStatus::Withdrawn,
            weights: withdrawn_weights,
            veto_thresholds: [None; CRITERION_COUNT],
        });

        let snapshot = PreferenceLedger { events }.snapshot_as_of(10).unwrap();
        let report = evaluate_group(&[alternative("a", 0.5, 0.5, 0.5)], &snapshot).unwrap();

        assert_eq!(report.active_stakeholder_count, 1);
        assert_eq!(report.excluded_profile_count, 2);
        assert_eq!(
            report.assessments[0].per_stakeholder_satisfaction[0].stakeholder_id,
            None
        );
    }

    #[test]
    fn late_recorded_consent_withdrawal_does_not_leak_into_earlier_cutoff() {
        let original = granted_event(
            "resident-a",
            1,
            1,
            None,
            100,
            ConsentScope::NamedAggregate,
            balanced_weights(0),
            [None; CRITERION_COUNT],
        );
        let withdrawal = PreferenceEvent {
            event_id: 2,
            stakeholder_id: "resident-a".to_owned(),
            revision: 2,
            previous_event_id: Some(1),
            effective_at_micros: 200,
            recorded_at_micros: 400,
            consent_scope: ConsentScope::NamedAggregate,
            consent_status: ConsentStatus::Withdrawn,
            weights: [0.0; CRITERION_COUNT],
            veto_thresholds: [None; CRITERION_COUNT],
        };
        let ledger = PreferenceLedger {
            events: vec![original, withdrawal],
        };

        let before_knowledge = ledger.snapshot_as_of(300).unwrap();
        let after_knowledge = ledger.snapshot_as_of(500).unwrap();

        assert_eq!(before_knowledge.profiles[0].revision, 1);
        assert_eq!(before_knowledge.profiles[0].consent_status, ConsentStatus::Granted);
        assert_eq!(after_knowledge.profiles[0].revision, 2);
        assert_eq!(after_knowledge.profiles[0].consent_status, ConsentStatus::Withdrawn);

        let before = evaluate_group(
            &[alternative("a", 0.6, 0.6, 0.6)],
            &before_knowledge,
        )
        .unwrap();
        assert_eq!(before.active_stakeholder_count, 1);
        assert!(matches!(
            evaluate_group(&[alternative("a", 0.6, 0.6, 0.6)], &after_knowledge),
            Err(PreferenceError::NoParticipatingStakeholders)
        ));
    }

    #[test]
    fn future_effective_preference_is_not_applied_before_its_start_time() {
        let event = PreferenceEvent {
            event_id: 1,
            stakeholder_id: "resident-a".to_owned(),
            revision: 1,
            previous_event_id: None,
            effective_at_micros: 500,
            recorded_at_micros: 100,
            consent_scope: ConsentScope::NamedAggregate,
            consent_status: ConsentStatus::Granted,
            weights: balanced_weights(0),
            veto_thresholds: [None; CRITERION_COUNT],
        };
        let ledger = PreferenceLedger { events: vec![event] };

        let before_effective = ledger.snapshot_as_of(300).unwrap();
        let after_effective = ledger.snapshot_as_of(600).unwrap();

        assert_eq!(before_effective.profiles.len(), 0);
        assert_eq!(after_effective.profiles.len(), 1);
        assert_eq!(after_effective.profiles[0].revision, 1);
    }

    #[test]
    fn hard_infeasible_candidates_never_enter_the_pareto_set() {
        let snapshot = PreferenceLedger {
            events: vec![granted_event(
                "resident-a",
                1,
                1,
                None,
                10,
                ConsentScope::NamedAggregate,
                balanced_weights(0),
                [None; CRITERION_COUNT],
            )],
        }
        .snapshot_as_of(10)
        .unwrap();
        let mut invalid = alternative("invalid", 1.0, 1.0, 1.0);
        invalid.hard_constraints_satisfied = false;

        let report = evaluate_group(
            &[invalid, alternative("feasible", 0.7, 0.7, 0.7)],
            &snapshot,
        )
        .unwrap();

        assert_eq!(report.assessments.len(), 1);
        assert_eq!(report.pareto_frontier_ids, vec!["feasible".to_owned()]);
        assert!(!report.rejected_alternatives[0].hard_constraints_satisfied);
    }

    #[test]
    fn invalid_weights_and_missing_consent_fail_closed() {
        let invalid = PreferenceEvent {
            event_id: 1,
            stakeholder_id: "resident-a".to_owned(),
            revision: 1,
            previous_event_id: None,
            effective_at_micros: 10,
            recorded_at_micros: 10,
            consent_scope: ConsentScope::NamedAggregate,
            consent_status: ConsentStatus::Granted,
            weights: [0.0; CRITERION_COUNT],
            veto_thresholds: [None; CRITERION_COUNT],
        };
        assert!(matches!(
            PreferenceLedger { events: vec![invalid] }.validate(),
            Err(PreferenceError::InvalidPreferenceWeights(_))
        ));

        let no_consent = PreferenceEvent {
            event_id: 2,
            stakeholder_id: "resident-b".to_owned(),
            revision: 1,
            previous_event_id: None,
            effective_at_micros: 10,
            recorded_at_micros: 10,
            consent_scope: ConsentScope::NamedAggregate,
            consent_status: ConsentStatus::NotGranted,
            weights: [0.0; CRITERION_COUNT],
            veto_thresholds: [None; CRITERION_COUNT],
        };
        let snapshot = PreferenceLedger {
            events: vec![no_consent],
        }
        .snapshot_as_of(10)
        .unwrap();
        assert!(matches!(
            evaluate_group(&[alternative("a", 0.5, 0.5, 0.5)], &snapshot),
            Err(PreferenceError::NoParticipatingStakeholders)
        ));
    }
}
