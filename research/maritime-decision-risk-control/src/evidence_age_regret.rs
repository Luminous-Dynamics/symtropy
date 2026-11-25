// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Evidence-age and conflict-aware decision-regret benchmark.
//!
//! The policy function receives only DecisionInput. GroundTruth is held in a
//! separate evaluator structure and is never passed to a policy. The benchmark
//! compares latest-ingested, freshest-published, fail-closed and bounded
//! contingent policies under identical evidence cutoffs and hidden truth.
//!
//! This is a synthetic decision-quality experiment, not a DGT estimator,
//! physical truth oracle, port-call API adapter or operational scheduler.

use std::collections::BTreeSet;
use std::fmt::Write as FmtWrite;

const FIXED_AS_OF: i64 = 1_000;
const FIXED_MAX_AGE: i64 = 20;
const WRONG_WINDOW_LOSS: u32 = 100;
const ABSTENTION_LOSS: u32 = 40;
const CONTINGENT_PRIMARY_LOSS: u32 = 5;
const CONTINGENT_FALLBACK_LOSS: u32 = 30;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ServiceWindow {
    WindowA,
    WindowB,
    WindowC,
}

impl ServiceWindow {
    pub const fn label(self) -> &'static str {
        match self {
            Self::WindowA => "WINDOW_A",
            Self::WindowB => "WINDOW_B",
            Self::WindowC => "WINDOW_C",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClaimMaturity {
    Requested,
    Estimated,
    Planned,
    SourceReportedActual,
}

impl ClaimMaturity {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Requested => "REQ",
            Self::Estimated => "EST",
            Self::Planned => "PLN",
            Self::SourceReportedActual => "ACT_SOURCE_REPORTED",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObservationStatus {
    Active,
    Withdrawn,
}

impl ObservationStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Active => "ACTIVE",
            Self::Withdrawn => "WITHDRAWN",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationalObservation {
    pub observation_id: String,
    pub source_record_id: String,
    pub source_id: String,
    /// Shared dependency group; multiple records in one group are not
    /// independent witnesses merely because they have different source IDs.
    pub source_cluster_id: String,
    pub subject_id: String,
    pub source_revision: u64,
    pub maturity: ClaimMaturity,
    pub claimed_window: ServiceWindow,
    /// Publisher/source time; None must remain unknown, not inferred from retrieval.
    pub published_at: Option<i64>,
    /// Time this exact record became available to the decision system.
    pub retrieved_at: i64,
    pub status: ObservationStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObservationValidationError {
    EmptyIdentity,
    ZeroSourceRevision,
    NegativeTimestamp,
    PublicationAfterRetrieval,
}

impl OperationalObservation {
    pub fn validate(&self) -> Result<(), ObservationValidationError> {
        if self.observation_id.trim().is_empty()
            || self.source_record_id.trim().is_empty()
            || self.source_id.trim().is_empty()
            || self.source_cluster_id.trim().is_empty()
            || self.subject_id.trim().is_empty()
        {
            return Err(ObservationValidationError::EmptyIdentity);
        }
        if self.source_revision == 0 {
            return Err(ObservationValidationError::ZeroSourceRevision);
        }
        if self.retrieved_at < 0 || self.published_at.is_some_and(|time| time < 0) {
            return Err(ObservationValidationError::NegativeTimestamp);
        }
        if self
            .published_at
            .is_some_and(|published| published > self.retrieved_at)
        {
            return Err(ObservationValidationError::PublicationAfterRetrieval);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionInput {
    pub subject_id: String,
    /// Historical decision-time cutoff. Neither publication nor retrieval after
    /// this point may influence a time-travel replay.
    pub as_of: i64,
    pub maximum_evidence_age: i64,
    pub feasible_windows: Vec<ServiceWindow>,
    pub observations: Vec<OperationalObservation>,
}

impl DecisionInput {
    pub fn validate(&self) -> Result<(), ObservationValidationError> {
        if self.subject_id.trim().is_empty() || self.feasible_windows.is_empty() {
            return Err(ObservationValidationError::EmptyIdentity);
        }
        if self.as_of < 0 || self.maximum_evidence_age < 0 {
            return Err(ObservationValidationError::NegativeTimestamp);
        }
        for observation in &self.observations {
            observation.validate()?;
            if observation.subject_id != self.subject_id {
                return Err(ObservationValidationError::EmptyIdentity);
            }
        }
        Ok(())
    }

    fn available_by_cutoff(&self) -> Vec<&OperationalObservation> {
        self.observations
            .iter()
            .filter(|observation| observation.retrieved_at <= self.as_of)
            .collect()
    }

    fn active_with_known_publication(&self) -> Vec<&OperationalObservation> {
        self.available_by_cutoff()
            .into_iter()
            .filter(|observation| observation.status == ObservationStatus::Active)
            .filter(|observation| {
                observation
                    .published_at
                    .is_some_and(|published| published <= self.as_of)
            })
            .collect()
    }

    fn fresh_active_observations(&self) -> Vec<&OperationalObservation> {
        self.active_with_known_publication()
            .into_iter()
            .filter(|observation| self.is_fresh(observation))
            .collect()
    }

    fn is_fresh(&self, observation: &OperationalObservation) -> bool {
        observation
            .published_at
            .is_some_and(|published| {
                published <= self.as_of
                    && self.as_of.saturating_sub(published) <= self.maximum_evidence_age
            })
            && observation.retrieved_at <= self.as_of
            && observation.status == ObservationStatus::Active
    }

    fn evidence_cluster_count(&self) -> usize {
        self.fresh_active_observations()
            .into_iter()
            .map(|observation| observation.source_cluster_id.as_str())
            .collect::<BTreeSet<_>>()
            .len()
    }

    fn feasible_claimed_windows(
        &self,
        observations: &[&OperationalObservation],
    ) -> BTreeSet<ServiceWindow> {
        observations
            .iter()
            .map(|observation| observation.claimed_window)
            .filter(|window| self.feasible_windows.contains(window))
            .collect()
    }

    fn most_recent_published<'a>(
        &self,
        observations: &'a [&'a OperationalObservation],
    ) -> Option<&'a OperationalObservation> {
        observations.iter().copied().max_by_key(|observation| {
            (
                observation.published_at.unwrap_or(-1),
                observation.retrieved_at,
                observation.source_revision,
                observation.observation_id.as_str(),
            )
        })
    }

    fn most_recent_ingested<'a>(
        &self,
        observations: &'a [&'a OperationalObservation],
    ) -> Option<&'a OperationalObservation> {
        observations.iter().copied().max_by_key(|observation| {
            (
                observation.retrieved_at,
                observation.source_revision,
                observation.observation_id.as_str(),
            )
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecisionStrategy {
    LatestIngested,
    FreshestPublished,
    FailClosed,
    BoundedContingent,
}

impl DecisionStrategy {
    pub const ALL: [Self; 4] = [
        Self::LatestIngested,
        Self::FreshestPublished,
        Self::FailClosed,
        Self::BoundedContingent,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::LatestIngested => "LATEST_INGESTED",
            Self::FreshestPublished => "FRESHEST_PUBLISHED",
            Self::FailClosed => "FAIL_CLOSED",
            Self::BoundedContingent => "BOUNDED_CONTINGENT",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AbstentionReason {
    NoAvailableEvidence,
    PublicationTimeUnknown,
    AllEvidenceStale,
    ConflictingEvidence,
    MoreThanTwoFeasibleAlternatives,
    NoFeasibleClaimedWindow,
}

impl AbstentionReason {
    pub const fn label(self) -> &'static str {
        match self {
            Self::NoAvailableEvidence => "NO_AVAILABLE_EVIDENCE",
            Self::PublicationTimeUnknown => "PUBLICATION_TIME_UNKNOWN",
            Self::AllEvidenceStale => "ALL_EVIDENCE_STALE",
            Self::ConflictingEvidence => "CONFLICTING_EVIDENCE",
            Self::MoreThanTwoFeasibleAlternatives => "MORE_THAN_TWO_ALTERNATIVES",
            Self::NoFeasibleClaimedWindow => "NO_FEASIBLE_CLAIMED_WINDOW",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlannedAction {
    Choose {
        window: ServiceWindow,
        source_observation_id: String,
    },
    Contingent {
        primary_window: ServiceWindow,
        primary_source_observation_id: String,
        fallback_window: ServiceWindow,
        fallback_source_observation_id: String,
    },
    Abstain {
        reason: AbstentionReason,
    },
}

impl PlannedAction {
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Choose { .. } => "CHOOSE",
            Self::Contingent { .. } => "CONTINGENT",
            Self::Abstain { .. } => "ABSTAIN",
        }
    }

    pub fn primary_window(&self) -> Option<ServiceWindow> {
        match self {
            Self::Choose { window, .. } => Some(*window),
            Self::Contingent {
                primary_window, ..
            } => Some(*primary_window),
            Self::Abstain { .. } => None,
        }
    }

    pub fn fallback_window(&self) -> Option<ServiceWindow> {
        match self {
            Self::Contingent {
                fallback_window, ..
            } => Some(*fallback_window),
            _ => None,
        }
    }

    fn source_ids(&self) -> (String, String) {
        match self {
            Self::Choose {
                source_observation_id,
                ..
            } => (source_observation_id.clone(), String::new()),
            Self::Contingent {
                primary_source_observation_id,
                fallback_source_observation_id,
                ..
            } => (
                primary_source_observation_id.clone(),
                fallback_source_observation_id.clone(),
            ),
            Self::Abstain { .. } => (String::new(), String::new()),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyDecision {
    pub strategy: DecisionStrategy,
    pub action: PlannedAction,
    pub available_observation_count: usize,
    pub active_published_observation_count: usize,
    pub fresh_observation_count: usize,
    /// Count of shared source-dependency clusters in the fresh evidence set.
    /// This is not inferred independence and does not imply statistical exchangeability.
    pub source_cluster_count: usize,
    pub stale_evidence_used: bool,
    pub withdrawn_evidence_used: bool,
    pub claim_ceiling: &'static str,
}

impl PolicyDecision {
    fn new(
        strategy: DecisionStrategy,
        action: PlannedAction,
        input: &DecisionInput,
        selected: &[&OperationalObservation],
    ) -> Self {
        let available = input.available_by_cutoff();
        let active = input.active_with_known_publication();
        let fresh_count = input.fresh_active_observations().len();
        let stale_evidence_used = selected.iter().any(|observation| {
            observation.status == ObservationStatus::Withdrawn || !input.is_fresh(observation)
        });
        let withdrawn_evidence_used = selected
            .iter()
            .any(|observation| observation.status == ObservationStatus::Withdrawn);

        Self {
            strategy,
            action,
            available_observation_count: available.len(),
            active_published_observation_count: active.len(),
            fresh_observation_count: fresh_count,
            source_cluster_count: input.evidence_cluster_count(),
            stale_evidence_used,
            withdrawn_evidence_used,
            claim_ceiling: "SOURCE_REPORTED_PLANNING_INPUT_ONLY",
        }
    }
}

/// Policies receive DecisionInput only. Hidden truth is kept in BenchmarkCase
/// and passed to the evaluator after the decision has been produced.
pub fn decide(
    input: &DecisionInput,
    strategy: DecisionStrategy,
) -> Result<PolicyDecision, ObservationValidationError> {
    input.validate()?;

    let action_and_selected = match strategy {
        DecisionStrategy::LatestIngested => {
            let available = input.available_by_cutoff();
            match input.most_recent_ingested(&available) {
                Some(observation) if input.feasible_windows.contains(&observation.claimed_window) => (
                    PlannedAction::Choose {
                        window: observation.claimed_window,
                        source_observation_id: observation.observation_id.clone(),
                    },
                    vec![observation],
                ),
                Some(_) => (
                    PlannedAction::Abstain {
                        reason: AbstentionReason::NoFeasibleClaimedWindow,
                    },
                    Vec::new(),
                ),
                None => (
                    PlannedAction::Abstain {
                        reason: AbstentionReason::NoAvailableEvidence,
                    },
                    Vec::new(),
                ),
            }
        }
        DecisionStrategy::FreshestPublished => {
            let active = input.active_with_known_publication();
            match input.most_recent_published(&active) {
                Some(observation) if input.feasible_windows.contains(&observation.claimed_window) => (
                    PlannedAction::Choose {
                        window: observation.claimed_window,
                        source_observation_id: observation.observation_id.clone(),
                    },
                    vec![observation],
                ),
                Some(_) => (
                    PlannedAction::Abstain {
                        reason: AbstentionReason::NoFeasibleClaimedWindow,
                    },
                    Vec::new(),
                ),
                None if !input.available_by_cutoff().is_empty() => (
                    PlannedAction::Abstain {
                        reason: AbstentionReason::PublicationTimeUnknown,
                    },
                    Vec::new(),
                ),
                None => (
                    PlannedAction::Abstain {
                        reason: AbstentionReason::NoAvailableEvidence,
                    },
                    Vec::new(),
                ),
            }
        }
        DecisionStrategy::FailClosed => {
            let fresh = input.fresh_active_observations();
            let windows = input.feasible_claimed_windows(&fresh);
            if fresh.is_empty() {
                let reason = if input
                    .active_published_observation_count_for_cutoff()
                    .is_empty()
                {
                    AbstentionReason::PublicationTimeUnknown
                } else {
                    AbstentionReason::AllEvidenceStale
                };
                (
                    PlannedAction::Abstain { reason },
                    Vec::new(),
                )
            } else if windows.is_empty() {
                (
                    PlannedAction::Abstain {
                        reason: AbstentionReason::NoFeasibleClaimedWindow,
                    },
                    Vec::new(),
                )
            } else if windows.len() > 1 {
                (
                    PlannedAction::Abstain {
                        reason: AbstentionReason::ConflictingEvidence,
                    },
                    Vec::new(),
                )
            } else {
                let window = *windows.first().expect("one window");
                let observation = input
                    .most_recent_published(&fresh)
                    .expect("fresh source observation");
                (
                    PlannedAction::Choose {
                        window,
                        source_observation_id: observation.observation_id.clone(),
                    },
                    vec![observation],
                )
            }
        }
        DecisionStrategy::BoundedContingent => {
            let fresh = input.fresh_active_observations();
            let windows = input.feasible_claimed_windows(&fresh);
            if fresh.is_empty() {
                let reason = if input
                    .active_published_observation_count_for_cutoff()
                    .is_empty()
                {
                    AbstentionReason::PublicationTimeUnknown
                } else {
                    AbstentionReason::AllEvidenceStale
                };
                (
                    PlannedAction::Abstain { reason },
                    Vec::new(),
                )
            } else if windows.is_empty() {
                (
                    PlannedAction::Abstain {
                        reason: AbstentionReason::NoFeasibleClaimedWindow,
                    },
                    Vec::new(),
                )
            } else if windows.len() > 2 {
                (
                    PlannedAction::Abstain {
                        reason: AbstentionReason::MoreThanTwoFeasibleAlternatives,
                    },
                    Vec::new(),
                )
            } else if windows.len() == 1 {
                let window = *windows.first().expect("one window");
                let observation = input
                    .most_recent_published(&fresh)
                    .expect("fresh source observation");
                (
                    PlannedAction::Choose {
                        window,
                        source_observation_id: observation.observation_id.clone(),
                    },
                    vec![observation],
                )
            } else {
                let sorted = fresh
                    .iter()
                    .copied()
                    .filter(|observation| windows.contains(&observation.claimed_window))
                    .collect::<Vec<_>>();
                let primary = input
                    .most_recent_published(&sorted)
                    .expect("fresh alternatives");
                let fallback = sorted
                    .iter()
                    .copied()
                    .filter(|observation| observation.claimed_window != primary.claimed_window)
                    .max_by_key(|observation| {
                        (
                            observation.published_at.unwrap_or(-1),
                            observation.retrieved_at,
                            observation.source_revision,
                        )
                    })
                    .expect("two distinct windows");
                (
                    PlannedAction::Contingent {
                        primary_window: primary.claimed_window,
                        primary_source_observation_id: primary.observation_id.clone(),
                        fallback_window: fallback.claimed_window,
                        fallback_source_observation_id: fallback.observation_id.clone(),
                    },
                    vec![primary, fallback],
                )
            }
        }
    };

    Ok(PolicyDecision::new(
        strategy,
        action_and_selected.0,
        input,
        &action_and_selected.1,
    ))
}

impl DecisionInput {
    fn active_published_observation_count_for_cutoff(&self) -> Vec<&OperationalObservation> {
        self.active_with_known_publication()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EvaluationTruth {
    /// Hidden outcome for evaluation only; never supplied to a policy.
    pub realized_window: ServiceWindow,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecisionScore {
    pub service_regret_units: u32,
    pub false_confidence: bool,
    pub contingent_fallback_recovered: bool,
    pub unsupported_commitment: bool,
    pub abstained: bool,
    pub unnecessary_abstention: bool,
}

pub fn evaluate(
    input: &DecisionInput,
    decision: &PolicyDecision,
    truth: EvaluationTruth,
) -> DecisionScore {
    let (service_regret_units, false_confidence, contingent_fallback_recovered, unsupported) =
        match &decision.action {
            PlannedAction::Choose { window, .. } if *window == truth.realized_window => {
                (0, false, false, false)
            }
            PlannedAction::Choose { .. } => (WRONG_WINDOW_LOSS, true, false, true),
            PlannedAction::Contingent {
                primary_window,
                fallback_window,
                ..
            } if *primary_window == truth.realized_window => {
                (CONTINGENT_PRIMARY_LOSS, false, false, false)
            }
            PlannedAction::Contingent {
                fallback_window, ..
            } if *fallback_window == truth.realized_window => {
                (CONTINGENT_FALLBACK_LOSS, false, true, false)
            }
            PlannedAction::Contingent { .. } => (WRONG_WINDOW_LOSS, false, false, true),
            PlannedAction::Abstain { .. } => (ABSTENTION_LOSS, false, false, false),
        };
    let abstained = matches!(decision.action, PlannedAction::Abstain { .. });
    let fresh_windows = input.feasible_claimed_windows(&input.fresh_active_observations());
    let unanimous_fresh_feasible = !fresh_windows.is_empty() && fresh_windows.len() == 1;
    let unnecessary_abstention = abstained && unanimous_fresh_feasible;

    DecisionScore {
        service_regret_units,
        false_confidence,
        contingent_fallback_recovered,
        unsupported_commitment: unsupported,
        abstained,
        unnecessary_abstention,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BenchmarkCase {
    pub case_id: &'static str,
    pub input: DecisionInput,
    pub truth: EvaluationTruth,
}

fn observation(
    id: &str,
    source: &str,
    cluster: &str,
    window: ServiceWindow,
    published: Option<i64>,
    retrieved: i64,
    status: ObservationStatus,
    revision: u64,
) -> OperationalObservation {
    OperationalObservation {
        observation_id: id.to_owned(),
        source_record_id: format!("record-{id}"),
        source_id: source.to_owned(),
        source_cluster_id: cluster.to_owned(),
        subject_id: "urn:port-call:synthetic-visit-001".to_owned(),
        source_revision: revision,
        maturity: ClaimMaturity::Planned,
        claimed_window: window,
        published_at: published,
        retrieved_at: retrieved,
        status,
    }
}

fn input(observations: Vec<OperationalObservation>) -> DecisionInput {
    DecisionInput {
        subject_id: "urn:port-call:synthetic-visit-001".to_owned(),
        as_of: FIXED_AS_OF,
        maximum_evidence_age: FIXED_MAX_AGE,
        feasible_windows: vec![
            ServiceWindow::WindowA,
            ServiceWindow::WindowB,
            ServiceWindow::WindowC,
        ],
        observations,
    }
}

pub fn benchmark_cases() -> Vec<BenchmarkCase> {
    vec![
        BenchmarkCase {
            case_id: "fresh_unanimous_consensus",
            input: input(vec![
                observation("fresh-a", "terminal-a", "terminal-owner-a", ServiceWindow::WindowA, Some(995), 996, ObservationStatus::Active, 1),
                observation("fresh-b", "carrier-b", "carrier-owner-b", ServiceWindow::WindowA, Some(994), 997, ObservationStatus::Active, 1),
            ]),
            truth: EvaluationTruth {
                realized_window: ServiceWindow::WindowA,
            },
        },
        BenchmarkCase {
            case_id: "stale_latest_ingested_conflicts_with_fresh",
            input: input(vec![
                observation("current-a", "terminal-a", "terminal-owner-a", ServiceWindow::WindowA, Some(995), 996, ObservationStatus::Active, 1),
                observation("stale-b", "aggregator-b", "aggregator-owner-b", ServiceWindow::WindowB, Some(950), 999, ObservationStatus::Active, 1),
            ]),
            truth: EvaluationTruth {
                realized_window: ServiceWindow::WindowA,
            },
        },
        BenchmarkCase {
            case_id: "fresh_conflict_bounded_contingency_recovers",
            input: input(vec![
                observation("conflict-a", "terminal-a", "terminal-owner-a", ServiceWindow::WindowA, Some(995), 996, ObservationStatus::Active, 1),
                observation("conflict-b", "carrier-b", "carrier-owner-b", ServiceWindow::WindowB, Some(998), 999, ObservationStatus::Active, 1),
            ]),
            truth: EvaluationTruth {
                realized_window: ServiceWindow::WindowA,
            },
        },
        BenchmarkCase {
            case_id: "withdrawn_latest_observation",
            input: input(vec![
                observation("active-a", "terminal-a", "terminal-owner-a", ServiceWindow::WindowA, Some(995), 996, ObservationStatus::Active, 1),
                observation("withdrawn-b", "carrier-b", "carrier-owner-b", ServiceWindow::WindowB, Some(999), 1_000, ObservationStatus::Withdrawn, 2),
            ]),
            truth: EvaluationTruth {
                realized_window: ServiceWindow::WindowA,
            },
        },
        BenchmarkCase {
            case_id: "unknown_publication_time",
            input: input(vec![observation(
                "unknown-published",
                "aggregator",
                "aggregator-owner",
                ServiceWindow::WindowC,
                None,
                999,
                ObservationStatus::Active,
                1,
            )]),
            truth: EvaluationTruth {
                realized_window: ServiceWindow::WindowA,
            },
        },
        BenchmarkCase {
            case_id: "correction_not_retrieved_at_historical_cutoff",
            input: input(vec![
                observation("old-window", "terminal-a", "terminal-owner-a", ServiceWindow::WindowB, Some(990), 991, ObservationStatus::Active, 1),
                observation("late-correction", "terminal-a", "terminal-owner-a", ServiceWindow::WindowA, Some(999), 1_001, ObservationStatus::Active, 2),
            ]),
            truth: EvaluationTruth {
                realized_window: ServiceWindow::WindowA,
            },
        },
        BenchmarkCase {
            case_id: "correction_available_after_cutoff",
            input: {
                let mut case_input = input(vec![
                    observation("old-window", "terminal-a", "terminal-owner-a", ServiceWindow::WindowB, Some(990), 991, ObservationStatus::Active, 1),
                    observation("late-correction", "terminal-a", "terminal-owner-a", ServiceWindow::WindowA, Some(999), 1_001, ObservationStatus::Active, 2),
                ]);
                case_input.as_of = 1_010;
                case_input
            },
            truth: EvaluationTruth {
                realized_window: ServiceWindow::WindowA,
            },
        },
        BenchmarkCase {
            case_id: "conflict_within_one_shared_dependency_cluster",
            input: input(vec![
                observation("same-cluster-a", "terminal-a", "shared-terminal-stack", ServiceWindow::WindowA, Some(997), 998, ObservationStatus::Active, 1),
                observation("same-cluster-b", "terminal-b", "shared-terminal-stack", ServiceWindow::WindowB, Some(999), 1_000, ObservationStatus::Active, 1),
            ]),
            truth: EvaluationTruth {
                realized_window: ServiceWindow::WindowA,
            },
        },
        BenchmarkCase {
            case_id: "too_many_conflicting_alternatives",
            input: input(vec![
                observation("many-a", "terminal-a", "terminal-owner-a", ServiceWindow::WindowA, Some(997), 998, ObservationStatus::Active, 1),
                observation("many-b", "carrier-b", "carrier-owner-b", ServiceWindow::WindowB, Some(998), 999, ObservationStatus::Active, 1),
                observation("many-c", "aggregator-c", "aggregator-owner-c", ServiceWindow::WindowC, Some(999), 1_000, ObservationStatus::Active, 1),
            ]),
            truth: EvaluationTruth {
                realized_window: ServiceWindow::WindowB,
            },
        },
    ]
}

pub fn render_csv() -> String {
    let mut output = String::from(
        "case_id,strategy,as_of,action,primary_window,fallback_window,available_observations,active_published_observations,fresh_observations,source_cluster_count,stale_evidence_used,withdrawn_evidence_used,service_regret_units,false_confidence,contingent_fallback_recovered,unsupported_commitment,abstained,unnecessary_abstention,primary_source_id,fallback_source_id,claim_ceiling,status\n",
    );
    for case in benchmark_cases() {
        for strategy in DecisionStrategy::ALL {
            let decision = decide(&case.input, strategy).expect("valid synthetic decision input");
            let score = evaluate(&case.input, &decision, case.truth);
            let (primary_source, fallback_source) = decision.action.source_ids();
            let _ = writeln!(
                output,
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},SYNTHETIC_ONLY",
                case.case_id,
                strategy.label(),
                case.input.as_of,
                decision.action.label(),
                decision.action.primary_window().map_or("NONE", ServiceWindow::label),
                decision.action.fallback_window().map_or("NONE", ServiceWindow::label),
                decision.available_observation_count,
                decision.active_published_observation_count,
                decision.fresh_observation_count,
                decision.source_cluster_count,
                decision.stale_evidence_used,
                decision.withdrawn_evidence_used,
                score.service_regret_units,
                score.false_confidence,
                score.contingent_fallback_recovered,
                score.unsupported_commitment,
                score.abstained,
                score.unnecessary_abstention,
                primary_source,
                fallback_source,
                decision.claim_ceiling,
            );
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn case(id: &str) -> BenchmarkCase {
        benchmark_cases()
            .into_iter()
            .find(|candidate| candidate.case_id == id)
            .expect("benchmark case exists")
    }

    #[test]
    fn policies_do_not_receive_hidden_evaluation_truth() {
        let scenario = case("fresh_conflict_bounded_contingency_recovers");
        let input_only = &scenario.input;
        let decision = decide(input_only, DecisionStrategy::FailClosed).expect("valid input");
        assert!(matches!(decision.action, PlannedAction::Abstain { .. }));
    }

    #[test]
    fn latest_ingested_can_select_a_stale_wrong_window() {
        let scenario = case("stale_latest_ingested_conflicts_with_fresh");
        let decision = decide(&scenario.input, DecisionStrategy::LatestIngested).expect("valid");
        assert_eq!(
            decision.action.primary_window(),
            Some(ServiceWindow::WindowB)
        );
        let score = evaluate(&scenario.input, &decision, scenario.truth);
        assert_eq!(score.service_regret_units, WRONG_WINDOW_LOSS);
        assert!(score.false_confidence);
        assert!(decision.stale_evidence_used);
    }

    #[test]
    fn freshest_published_chooses_the_current_source_value_over_stale_ingestion() {
        let scenario = case("stale_latest_ingested_conflicts_with_fresh");
        let decision = decide(&scenario.input, DecisionStrategy::FreshestPublished).expect("valid");
        assert_eq!(
            decision.action.primary_window(),
            Some(ServiceWindow::WindowA)
        );
        assert_eq!(
            evaluate(&scenario.input, &decision, scenario.truth).service_regret_units,
            0
        );
    }

    #[test]
    fn fail_closed_abstains_on_fresh_conflict_without_falsely_claiming_truth() {
        let scenario = case("fresh_conflict_bounded_contingency_recovers");
        let decision = decide(&scenario.input, DecisionStrategy::FailClosed).expect("valid");
        assert!(matches!(
            decision.action,
            PlannedAction::Abstain {
                reason: AbstentionReason::ConflictingEvidence
            }
        ));
        let score = evaluate(&scenario.input, &decision, scenario.truth);
        assert!(score.abstained);
        assert!(!score.false_confidence);
        assert_eq!(score.service_regret_units, ABSTENTION_LOSS);
    }

    #[test]
    fn bounded_contingency_can_recover_a_conflicting_but_feasible_alternative() {
        let scenario = case("fresh_conflict_bounded_contingency_recovers");
        let decision =
            decide(&scenario.input, DecisionStrategy::BoundedContingent).expect("valid");
        assert!(matches!(
            decision.action,
            PlannedAction::Contingent {
                primary_window: ServiceWindow::WindowB,
                fallback_window: ServiceWindow::WindowA,
                ..
            }
        ));
        let score = evaluate(&scenario.input, &decision, scenario.truth);
        assert!(score.contingent_fallback_recovered);
        assert!(!score.false_confidence);
        assert_eq!(
            score.service_regret_units,
            CONTINGENT_FALLBACK_LOSS
        );
    }

    #[test]
    fn withdrawn_latest_record_is_excluded_by_non_naive_policies() {
        let scenario = case("withdrawn_latest_observation");
        let naive = decide(&scenario.input, DecisionStrategy::LatestIngested).expect("valid");
        let strict = decide(&scenario.input, DecisionStrategy::FailClosed).expect("valid");
        assert_eq!(
            naive.action.primary_window(),
            Some(ServiceWindow::WindowB)
        );
        assert_eq!(
            strict.action.primary_window(),
            Some(ServiceWindow::WindowA)
        );
        assert!(naive.withdrawn_evidence_used);
        assert!(!strict.withdrawn_evidence_used);
    }

    #[test]
    fn unknown_publication_time_never_becomes_fresh_by_substituting_retrieval_time() {
        let scenario = case("unknown_publication_time");
        let freshest = decide(&scenario.input, DecisionStrategy::FreshestPublished).expect("valid");
        let strict = decide(&scenario.input, DecisionStrategy::FailClosed).expect("valid");
        assert!(matches!(freshest.action, PlannedAction::Abstain { .. }));
        assert!(matches!(strict.action, PlannedAction::Abstain { .. }));
        let naive = decide(&scenario.input, DecisionStrategy::LatestIngested).expect("valid");
        assert_eq!(
            naive.action.primary_window(),
            Some(ServiceWindow::WindowC)
        );
        assert!(naive.stale_evidence_used);
    }

    #[test]
    fn later_correction_cannot_rewrite_what_was_available_at_earlier_cutoff() {
        let historical = case("correction_not_retrieved_at_historical_cutoff");
        let later = case("correction_available_after_cutoff");
        let old_view = decide(&historical.input, DecisionStrategy::FreshestPublished).expect("valid");
        let corrected_view = decide(&later.input, DecisionStrategy::FreshestPublished).expect("valid");
        assert_eq!(
            old_view.action.primary_window(),
            Some(ServiceWindow::WindowB)
        );
        assert_eq!(
            corrected_view.action.primary_window(),
            Some(ServiceWindow::WindowA)
        );
    }

    #[test]
    fn nominal_source_count_does_not_inflate_shared_dependency_clusters() {
        let scenario = case("conflict_within_one_shared_dependency_cluster");
        let decision = decide(&scenario.input, DecisionStrategy::BoundedContingent).expect("valid");
        assert_eq!(decision.fresh_observation_count, 2);
        assert_eq!(decision.source_cluster_count, 1);
        assert!(matches!(decision.action, PlannedAction::Contingent { .. }));
    }

    #[test]
    fn more_than_two_conflicting_alternatives_force_abstention() {
        let scenario = case("too_many_conflicting_alternatives");
        let decision = decide(&scenario.input, DecisionStrategy::BoundedContingent).expect("valid");
        assert!(matches!(
            decision.action,
            PlannedAction::Abstain {
                reason: AbstentionReason::MoreThanTwoFeasibleAlternatives
            }
        ));
    }

    #[test]
    fn all_actions_retain_a_source_reported_claim_ceiling() {
        for scenario in benchmark_cases() {
            for strategy in DecisionStrategy::ALL {
                let decision = decide(&scenario.input, strategy).expect("valid input");
                assert_eq!(decision.claim_ceiling, "SOURCE_REPORTED_PLANNING_INPUT_ONLY");
            }
        }
    }

    #[test]
    fn observation_validation_rejects_future_publication_and_missing_identity() {
        let mut candidate = case("fresh_unanimous_consensus").input.observations[0].clone();
        candidate.published_at = Some(candidate.retrieved_at + 1);
        assert_eq!(
            candidate.validate(),
            Err(ObservationValidationError::PublicationAfterRetrieval)
        );
        candidate.published_at = Some(995);
        candidate.source_cluster_id.clear();
        assert_eq!(
            candidate.validate(),
            Err(ObservationValidationError::EmptyIdentity)
        );
    }

    #[test]
    fn receipt_is_deterministic_and_has_expected_case_strategy_schema() {
        let first = render_csv();
        assert_eq!(first, render_csv());
        let lines = first.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 1 + benchmark_cases().len() * DecisionStrategy::ALL.len());
        let width = lines[0].split(',').count();
        assert_eq!(width, 22);
        for row in lines.iter().skip(1) {
            assert_eq!(row.split(',').count(), width);
        }
        assert!(first.contains("LATEST_INGESTED"));
        assert!(first.contains("FRESHEST_PUBLISHED"));
        assert!(first.contains("FAIL_CLOSED"));
        assert!(first.contains("BOUNDED_CONTINGENT"));
        assert!(first.contains("PUBLICATION_TIME_UNKNOWN"));
    }
}
