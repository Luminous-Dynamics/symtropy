// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Deterministic accounting for collision interactions across a physics pipeline.
//!
//! This validates stage disposition conservation and parent/child lineage. It
//! is not a collision oracle, a cryptographic receipt system, or proof that a
//! pruning decision is physically correct.

use std::collections::{BTreeMap, BTreeSet};
use std::{error::Error, fmt};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct BodyIdentity {
    pub handle: u64,
    pub generation: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ColliderIdentity {
    pub body: BodyIdentity,
    pub collider: u32,
    pub subshape_path: Vec<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PairIdentity {
    pub world_revision: u64,
    pub step_index: u64,
    pub first: ColliderIdentity,
    pub second: ColliderIdentity,
}

impl PairIdentity {
    pub fn new(
        world_revision: u64,
        step_index: u64,
        left: ColliderIdentity,
        right: ColliderIdentity,
    ) -> Result<Self, LedgerError> {
        if left == right {
            return Err(LedgerError::SelfPair);
        }
        let (first, second) = if left < right {
            (left, right)
        } else {
            (right, left)
        };
        Ok(Self {
            world_revision,
            step_index,
            first,
            second,
        })
    }

    #[must_use]
    pub fn is_canonical(&self) -> bool {
        self.first < self.second
    }
}

/// Empty path identifies the pair root; each child appends one stable ordinal.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct LineageId {
    pair: PairIdentity,
    path: Vec<u32>,
}

impl LineageId {
    #[must_use]
    pub fn root(pair: PairIdentity) -> Self {
        Self {
            pair,
            path: Vec::new(),
        }
    }

    #[must_use]
    pub fn child(&self, ordinal: u32) -> Self {
        let mut path = self.path.clone();
        path.push(ordinal);
        Self {
            pair: self.pair.clone(),
            path,
        }
    }

    #[must_use]
    pub fn pair(&self) -> &PairIdentity {
        &self.pair
    }

    #[must_use]
    pub fn path(&self) -> &[u32] {
        &self.path
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(u8)]
pub enum Stage {
    Eligibility = 0,
    TemporalBounds = 1,
    Broadphase = 2,
    Narrowphase = 3,
    CandidateGeneration = 4,
    ManifoldReduction = 5,
    SolverMapping = 6,
    Solve = 7,
}

impl Stage {
    pub const ORDER: [Self; 8] = [
        Self::Eligibility,
        Self::TemporalBounds,
        Self::Broadphase,
        Self::Narrowphase,
        Self::CandidateGeneration,
        Self::ManifoldReduction,
        Self::SolverMapping,
        Self::Solve,
    ];
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(u8)]
pub enum ForwardReason {
    UnsupportedBoundFallback = 0,
    InvalidBoundFallback = 1,
    UnboundedMotionFallback = 2,
    ExhaustiveBroadphaseFallback = 3,
    UnsupportedShapeFallback = 4,
    ManifoldsGenerated = 5,
    CandidateAdmitted = 6,
    ManifoldRetained = 7,
    SolverRowSubmitted = 8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(u8)]
pub enum TerminalReason {
    CollisionFiltered = 0,
    InvalidInput = 1,
    TemporallyProvenDisjoint = 2,
    BroadphaseProvenDisjoint = 3,
    NarrowphaseNoContact = 4,
    UnsupportedGeometry = 5,
    SupportQueryFailed = 6,
    ShallowCandidate = 7,
    DuplicateCandidate = 8,
    CandidateCapacity = 9,
    DeliberatelyReduced = 10,
    ManifoldStorageOverflow = 11,
    SolverRuleRejected = 12,
    SolverUnsupported = 13,
    SolverNotSubmitted = 14,
    Converged = 15,
    BudgetExhausted = 16,
    SolverFailed = 17,
    SleepingSkip = 18,
    Aborted = 19,
    NotRun = 20,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(u8)]
pub enum CandidateAttemptKind {
    PrimaryWitness = 0,
    PerturbationDirection = 1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(u8)]
pub enum DirectionDisposition {
    NotApplicable = 0,
    Generated = 1,
    InvalidOrDegenerate = 2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(u8)]
pub enum SupportQueryDisposition {
    NotAttempted = 0,
    Completed = 1,
    Failed = 2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(u8)]
pub enum CandidateDisposition {
    PrimaryAdmitted = 0,
    Admitted = 1,
    InvalidDirection = 2,
    SupportQueryFailed = 3,
    ShallowRejected = 4,
    DuplicateRejected = 5,
    CapacityRejected = 6,
}

/// One requested candidate-generation attempt. The two support-query outcomes
/// correspond to the two shape support-map calls for a generated direction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateAttempt {
    pub id: LineageId,
    pub kind: CandidateAttemptKind,
    pub direction: DirectionDisposition,
    pub support_a: SupportQueryDisposition,
    pub support_b: SupportQueryDisposition,
    pub disposition: CandidateDisposition,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Outcome {
    Forward,
    ForwardWithReason(ForwardReason),
    Spawn {
        reason: ForwardReason,
        children: Vec<LineageId>,
    },
    Terminate(TerminalReason),
    /// Complete candidate-generation trace. Every attempt is explicitly
    /// admitted to the next stage or terminated with a classified reason.
    CandidateTrace {
        /// Declared number of perturbation directions requested from the generator.
        requested_directions: usize,
        attempts: Vec<CandidateAttempt>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StageEvent {
    pub stage: Stage,
    pub input: LineageId,
    pub outcome: Outcome,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StageStats {
    pub inputs: usize,
    pub forwarded: usize,
    pub spawned: usize,
    pub terminal: usize,
    pub candidate_attempts: usize,
    pub requested_directions: usize,
    pub generated_directions: usize,
    pub invalid_directions: usize,
    pub support_queries_attempted: usize,
    pub support_queries_completed: usize,
    pub support_queries_failed: usize,
    pub admitted_candidates: usize,
    pub shallow_rejections: usize,
    pub duplicate_rejections: usize,
    pub capacity_rejections: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum DispositionCode {
    Forwarded,
    ForwardedWithReason(ForwardReason),
    Spawned(ForwardReason),
    Terminated(TerminalReason),
    CandidateAttempt(CandidateDisposition),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Receipt {
    pub root_pairs: usize,
    pub event_count: usize,
    pub forwarded_count: usize,
    pub spawned_count: usize,
    pub terminal_count: usize,
    pub stages: Vec<(Stage, StageStats)>,
    pub disposition_counts: Vec<(Stage, DispositionCode, usize)>,
    /// Stable FNV-1a 64-bit fingerprint; not a cryptographic digest.
    pub fingerprint: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LedgerError {
    SelfPair,
    NonCanonicalPair,
    DuplicateRoot,
    DuplicateEvent(Stage),
    StageInputMismatch {
        stage: Stage,
        expected: usize,
        actual: usize,
        missing: usize,
        unexpected: usize,
    },
    EmptySpawn,
    EmptyCandidateTrace,
    RequestedDirectionCountMismatch {
        requested: usize,
        recorded: usize,
    },
    MissingPrimaryCandidate,
    MultiplePrimaryCandidates,
    DuplicateCandidateAttempt,
    InvalidCandidateAttempt,
    CandidateTraceWrongStage,
    ChildHasDifferentPair,
    ChildPathNotImmediate,
    DuplicateChild,
    InvalidTerminalReason {
        stage: Stage,
        reason: TerminalReason,
    },
    InvalidForwardReason {
        stage: Stage,
        reason: ForwardReason,
    },
    FinalStageHasForwardedItems(usize),
}

impl fmt::Display for LedgerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SelfPair => write!(f, "a collider cannot be paired with itself"),
            Self::NonCanonicalPair => write!(f, "pair identity is not canonical"),
            Self::DuplicateRoot => write!(f, "duplicate root pair identity"),
            Self::DuplicateEvent(stage) => write!(f, "duplicate disposition at {stage:?}"),
            Self::StageInputMismatch {
                stage,
                expected,
                actual,
                missing,
                unexpected,
            } => write!(
                f,
                "{stage:?} input mismatch: expected {expected}, found {actual} (missing {missing}, unexpected {unexpected})"
            ),
            Self::EmptySpawn => write!(f, "spawn disposition must create at least one child"),
            Self::EmptyCandidateTrace => {
                write!(f, "candidate trace must contain at least one attempt")
            }
            Self::RequestedDirectionCountMismatch {
                requested,
                recorded,
            } => write!(
                f,
                "candidate trace requested {requested} perturbation directions but records {recorded}"
            ),
            Self::MissingPrimaryCandidate => write!(
                f,
                "candidate trace must contain exactly one primary witness"
            ),
            Self::MultiplePrimaryCandidates => {
                write!(f, "candidate trace contains multiple primary witnesses")
            }
            Self::DuplicateCandidateAttempt => {
                write!(f, "candidate trace contains duplicate attempt identities")
            }
            Self::InvalidCandidateAttempt => write!(
                f,
                "candidate attempt has inconsistent direction, query, or disposition fields"
            ),
            Self::CandidateTraceWrongStage => {
                write!(f, "candidate trace is only valid at CandidateGeneration")
            }
            Self::ChildHasDifferentPair => write!(f, "child lineage changed pair identity"),
            Self::ChildPathNotImmediate => {
                write!(f, "child path must extend parent by exactly one ordinal")
            }
            Self::DuplicateChild => write!(f, "duplicate child identity emitted"),
            Self::InvalidTerminalReason { stage, reason } => {
                write!(f, "terminal reason {reason:?} is invalid at {stage:?}")
            }
            Self::InvalidForwardReason { stage, reason } => {
                write!(f, "forward reason {reason:?} is invalid at {stage:?}")
            }
            Self::FinalStageHasForwardedItems(n) => write!(
                f,
                "solve stage left {n} item(s) without terminal disposition"
            ),
        }
    }
}
impl Error for LedgerError {}

#[derive(Clone, Debug)]
pub struct PairLedger {
    roots: BTreeSet<LineageId>,
    events: BTreeMap<Stage, BTreeMap<LineageId, Outcome>>,
}

impl PairLedger {
    pub fn new(pairs: impl IntoIterator<Item = PairIdentity>) -> Result<Self, LedgerError> {
        let mut roots = BTreeSet::new();
        for pair in pairs {
            if !pair.is_canonical() {
                return Err(LedgerError::NonCanonicalPair);
            }
            if !roots.insert(LineageId::root(pair)) {
                return Err(LedgerError::DuplicateRoot);
            }
        }
        Ok(Self {
            roots,
            events: BTreeMap::new(),
        })
    }

    pub fn record(&mut self, mut event: StageEvent) -> Result<(), LedgerError> {
        match &mut event.outcome {
            Outcome::Spawn { children, .. } => children.sort(),
            Outcome::CandidateTrace { attempts, .. } => attempts.sort_by(|a, b| a.id.cmp(&b.id)),
            _ => {}
        }
        let records = self.events.entry(event.stage).or_default();
        if records.contains_key(&event.input) {
            return Err(LedgerError::DuplicateEvent(event.stage));
        }
        records.insert(event.input, event.outcome);
        Ok(())
    }

    /// Requires exact conservation between every adjacent stage.
    pub fn validate(&self) -> Result<(), LedgerError> {
        let mut expected = self.roots.clone();
        for stage in Stage::ORDER {
            let records = self.events.get(&stage).cloned().unwrap_or_default();
            let actual: BTreeSet<_> = records.keys().cloned().collect();
            if actual != expected {
                return Err(LedgerError::StageInputMismatch {
                    stage,
                    expected: expected.len(),
                    actual: actual.len(),
                    missing: expected.difference(&actual).count(),
                    unexpected: actual.difference(&expected).count(),
                });
            }
            let mut next = BTreeSet::new();
            for (input, outcome) in records {
                match outcome {
                    Outcome::Forward => {
                        next.insert(input);
                    }
                    Outcome::ForwardWithReason(reason) => {
                        if !valid_forward(stage, reason) {
                            return Err(LedgerError::InvalidForwardReason { stage, reason });
                        }
                        next.insert(input);
                    }
                    Outcome::Spawn { reason, children } => {
                        if !valid_forward(stage, reason) {
                            return Err(LedgerError::InvalidForwardReason { stage, reason });
                        }
                        if children.is_empty() {
                            return Err(LedgerError::EmptySpawn);
                        }
                        for child in children {
                            if child.pair != input.pair {
                                return Err(LedgerError::ChildHasDifferentPair);
                            }
                            if child.path.len() != input.path.len() + 1
                                || !child.path.starts_with(&input.path)
                            {
                                return Err(LedgerError::ChildPathNotImmediate);
                            }
                            if !next.insert(child) {
                                return Err(LedgerError::DuplicateChild);
                            }
                        }
                    }
                    Outcome::Terminate(reason) => {
                        if !valid_terminal(stage, reason) {
                            return Err(LedgerError::InvalidTerminalReason { stage, reason });
                        }
                    }
                    Outcome::CandidateTrace {
                        requested_directions,
                        attempts,
                    } => {
                        if stage != Stage::CandidateGeneration {
                            return Err(LedgerError::CandidateTraceWrongStage);
                        }
                        validate_candidate_trace(&input, requested_directions, &attempts)?;
                        for attempt in attempts {
                            if is_candidate_admitted(attempt.disposition) {
                                if !next.insert(attempt.id) {
                                    return Err(LedgerError::DuplicateCandidateAttempt);
                                }
                            }
                        }
                    }
                }
            }
            expected = next;
        }
        if !expected.is_empty() {
            return Err(LedgerError::FinalStageHasForwardedItems(expected.len()));
        }
        Ok(())
    }

    pub fn receipt(&self) -> Result<Receipt, LedgerError> {
        self.validate()?;
        let mut stages = Vec::with_capacity(Stage::ORDER.len());
        let mut reasons = BTreeMap::new();
        let (mut event_count, mut forwarded, mut spawned, mut terminal) = (0, 0, 0, 0);
        for stage in Stage::ORDER {
            let records = self.events.get(&stage).cloned().unwrap_or_default();
            let mut stats = StageStats {
                inputs: records.len(),
                ..StageStats::default()
            };
            for outcome in records.values() {
                let code = match outcome {
                    Outcome::Forward => {
                        stats.forwarded += 1;
                        DispositionCode::Forwarded
                    }
                    Outcome::ForwardWithReason(r) => {
                        stats.forwarded += 1;
                        DispositionCode::ForwardedWithReason(*r)
                    }
                    Outcome::Spawn { reason, children } => {
                        stats.spawned += children.len();
                        DispositionCode::Spawned(*reason)
                    }
                    Outcome::Terminate(r) => {
                        stats.terminal += 1;
                        DispositionCode::Terminated(*r)
                    }
                    Outcome::CandidateTrace {
                        requested_directions,
                        attempts,
                    } => {
                        stats.requested_directions += requested_directions;
                        for attempt in attempts {
                            stats.candidate_attempts += 1;
                            stats.spawned += 1;
                            if attempt.kind == CandidateAttemptKind::PerturbationDirection {
                                match attempt.direction {
                                    DirectionDisposition::Generated => {
                                        stats.generated_directions += 1
                                    }
                                    DirectionDisposition::InvalidOrDegenerate => {
                                        stats.invalid_directions += 1
                                    }
                                    DirectionDisposition::NotApplicable => {}
                                }
                            }
                            for query in [attempt.support_a, attempt.support_b] {
                                match query {
                                    SupportQueryDisposition::NotAttempted => {}
                                    SupportQueryDisposition::Completed => {
                                        stats.support_queries_attempted += 1;
                                        stats.support_queries_completed += 1;
                                    }
                                    SupportQueryDisposition::Failed => {
                                        stats.support_queries_attempted += 1;
                                        stats.support_queries_failed += 1;
                                    }
                                }
                            }
                            if is_candidate_admitted(attempt.disposition) {
                                stats.forwarded += 1;
                                stats.admitted_candidates += 1;
                            } else {
                                stats.terminal += 1;
                                match attempt.disposition {
                                    CandidateDisposition::InvalidDirection => {}
                                    CandidateDisposition::SupportQueryFailed => {}
                                    CandidateDisposition::ShallowRejected => {
                                        stats.shallow_rejections += 1
                                    }
                                    CandidateDisposition::DuplicateRejected => {
                                        stats.duplicate_rejections += 1
                                    }
                                    CandidateDisposition::CapacityRejected => {
                                        stats.capacity_rejections += 1
                                    }
                                    CandidateDisposition::PrimaryAdmitted
                                    | CandidateDisposition::Admitted => {}
                                }
                            }
                            let code = DispositionCode::CandidateAttempt(attempt.disposition);
                            *reasons.entry((stage, code)).or_insert(0usize) += 1;
                        }
                        continue;
                    }
                };
                *reasons.entry((stage, code)).or_insert(0usize) += 1;
            }
            event_count += stats.inputs;
            forwarded += stats.forwarded;
            spawned += stats.spawned;
            terminal += stats.terminal;
            stages.push((stage, stats));
        }
        Ok(Receipt {
            root_pairs: self.roots.len(),
            event_count,
            forwarded_count: forwarded,
            spawned_count: spawned,
            terminal_count: terminal,
            stages,
            disposition_counts: reasons.into_iter().map(|((s, c), n)| (s, c, n)).collect(),
            fingerprint: self.fingerprint(),
        })
    }

    /// Deterministic FNV-1a fingerprint. Use a cryptographic digest for signed evidence.
    #[must_use]
    pub fn fingerprint(&self) -> u64 {
        let mut h = Fnv64(0xcbf29ce484222325);
        h.u64(self.roots.len() as u64);
        for root in &self.roots {
            h.lineage(root);
        }
        for stage in Stage::ORDER {
            h.byte(stage as u8);
            if let Some(records) = self.events.get(&stage) {
                h.u64(records.len() as u64);
                for (input, outcome) in records {
                    h.lineage(input);
                    h.outcome(outcome);
                }
            } else {
                h.u64(0);
            }
        }
        h.0
    }
}

fn is_candidate_admitted(disposition: CandidateDisposition) -> bool {
    matches!(
        disposition,
        CandidateDisposition::PrimaryAdmitted | CandidateDisposition::Admitted
    )
}

fn validate_candidate_trace(
    input: &LineageId,
    requested_directions: usize,
    attempts: &[CandidateAttempt],
) -> Result<(), LedgerError> {
    if attempts.is_empty() {
        return Err(LedgerError::EmptyCandidateTrace);
    }
    let mut seen = BTreeSet::new();
    let mut primary_count = 0usize;
    let mut recorded_directions = 0usize;
    for attempt in attempts {
        if attempt.kind == CandidateAttemptKind::PerturbationDirection {
            recorded_directions += 1;
        }
        if attempt.id.pair != input.pair
            || attempt.id.path.len() != input.path.len() + 1
            || !attempt.id.path.starts_with(&input.path)
        {
            return Err(LedgerError::ChildPathNotImmediate);
        }
        if !seen.insert(&attempt.id) {
            return Err(LedgerError::DuplicateCandidateAttempt);
        }
        let valid = match attempt.kind {
            CandidateAttemptKind::PrimaryWitness => {
                primary_count += 1;
                attempt.direction == DirectionDisposition::NotApplicable
                    && attempt.support_a == SupportQueryDisposition::NotAttempted
                    && attempt.support_b == SupportQueryDisposition::NotAttempted
                    && attempt.disposition == CandidateDisposition::PrimaryAdmitted
            }
            CandidateAttemptKind::PerturbationDirection => match attempt.direction {
                DirectionDisposition::NotApplicable => false,
                DirectionDisposition::InvalidOrDegenerate => {
                    attempt.support_a == SupportQueryDisposition::NotAttempted
                        && attempt.support_b == SupportQueryDisposition::NotAttempted
                        && attempt.disposition == CandidateDisposition::InvalidDirection
                }
                DirectionDisposition::Generated => {
                    let both_attempted = attempt.support_a != SupportQueryDisposition::NotAttempted
                        && attempt.support_b != SupportQueryDisposition::NotAttempted;
                    let any_failed = attempt.support_a == SupportQueryDisposition::Failed
                        || attempt.support_b == SupportQueryDisposition::Failed;
                    both_attempted
                        && if any_failed {
                            attempt.disposition == CandidateDisposition::SupportQueryFailed
                        } else {
                            matches!(
                                attempt.disposition,
                                CandidateDisposition::ShallowRejected
                                    | CandidateDisposition::DuplicateRejected
                                    | CandidateDisposition::CapacityRejected
                                    | CandidateDisposition::Admitted
                            )
                        }
                }
            },
        };
        if !valid {
            return Err(LedgerError::InvalidCandidateAttempt);
        }
    }
    if requested_directions != recorded_directions {
        return Err(LedgerError::RequestedDirectionCountMismatch {
            requested: requested_directions,
            recorded: recorded_directions,
        });
    }
    match primary_count {
        0 => Err(LedgerError::MissingPrimaryCandidate),
        1 => Ok(()),
        _ => Err(LedgerError::MultiplePrimaryCandidates),
    }
}

fn valid_forward(stage: Stage, reason: ForwardReason) -> bool {
    matches!(
        (stage, reason),
        (
            Stage::TemporalBounds,
            ForwardReason::UnsupportedBoundFallback
                | ForwardReason::InvalidBoundFallback
                | ForwardReason::UnboundedMotionFallback
        ) | (
            Stage::Broadphase,
            ForwardReason::ExhaustiveBroadphaseFallback
        ) | (
            Stage::Narrowphase,
            ForwardReason::UnsupportedShapeFallback | ForwardReason::ManifoldsGenerated
        ) | (Stage::CandidateGeneration, ForwardReason::CandidateAdmitted)
            | (Stage::ManifoldReduction, ForwardReason::ManifoldRetained)
            | (Stage::SolverMapping, ForwardReason::SolverRowSubmitted)
    )
}

fn valid_terminal(stage: Stage, reason: TerminalReason) -> bool {
    match stage {
        Stage::Eligibility => matches!(
            reason,
            TerminalReason::CollisionFiltered
                | TerminalReason::InvalidInput
                | TerminalReason::Aborted
        ),
        Stage::TemporalBounds => matches!(
            reason,
            TerminalReason::TemporallyProvenDisjoint | TerminalReason::Aborted
        ),
        Stage::Broadphase => matches!(
            reason,
            TerminalReason::BroadphaseProvenDisjoint
                | TerminalReason::InvalidInput
                | TerminalReason::Aborted
        ),
        Stage::Narrowphase => matches!(
            reason,
            TerminalReason::NarrowphaseNoContact
                | TerminalReason::UnsupportedGeometry
                | TerminalReason::InvalidInput
                | TerminalReason::Aborted
        ),
        Stage::CandidateGeneration => matches!(
            reason,
            TerminalReason::SupportQueryFailed
                | TerminalReason::ShallowCandidate
                | TerminalReason::DuplicateCandidate
                | TerminalReason::CandidateCapacity
                | TerminalReason::InvalidInput
                | TerminalReason::Aborted
        ),
        Stage::ManifoldReduction => matches!(
            reason,
            TerminalReason::DeliberatelyReduced
                | TerminalReason::ManifoldStorageOverflow
                | TerminalReason::InvalidInput
                | TerminalReason::Aborted
        ),
        Stage::SolverMapping => matches!(
            reason,
            TerminalReason::SolverRuleRejected
                | TerminalReason::SolverUnsupported
                | TerminalReason::SolverNotSubmitted
                | TerminalReason::InvalidInput
                | TerminalReason::Aborted
        ),
        Stage::Solve => matches!(
            reason,
            TerminalReason::Converged
                | TerminalReason::BudgetExhausted
                | TerminalReason::SolverFailed
                | TerminalReason::SleepingSkip
                | TerminalReason::Aborted
                | TerminalReason::NotRun
        ),
    }
}

struct Fnv64(u64);
impl Fnv64 {
    fn byte(&mut self, b: u8) {
        self.0 = (self.0 ^ u64::from(b)).wrapping_mul(0x100000001b3);
    }
    fn bytes(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.byte(*b);
        }
    }
    fn u32(&mut self, n: u32) {
        self.bytes(&n.to_le_bytes());
    }
    fn u64(&mut self, n: u64) {
        self.bytes(&n.to_le_bytes());
    }
    fn pair(&mut self, p: &PairIdentity) {
        self.u64(p.world_revision);
        self.u64(p.step_index);
        for c in [&p.first, &p.second] {
            self.u64(c.body.handle);
            self.u32(c.body.generation);
            self.u32(c.collider);
            self.u64(c.subshape_path.len() as u64);
            for part in &c.subshape_path {
                self.u32(*part);
            }
        }
    }
    fn lineage(&mut self, id: &LineageId) {
        self.pair(&id.pair);
        self.u64(id.path.len() as u64);
        for part in &id.path {
            self.u32(*part);
        }
    }
    fn outcome(&mut self, o: &Outcome) {
        match o {
            Outcome::Forward => self.byte(0),
            Outcome::ForwardWithReason(r) => {
                self.byte(1);
                self.byte(*r as u8);
            }
            Outcome::Spawn { reason, children } => {
                self.byte(2);
                self.byte(*reason as u8);
                self.u64(children.len() as u64);
                for child in children {
                    self.lineage(child);
                }
            }
            Outcome::Terminate(r) => {
                self.byte(3);
                self.byte(*r as u8);
            }
            Outcome::CandidateTrace {
                requested_directions,
                attempts,
            } => {
                self.byte(4);
                self.u64(*requested_directions as u64);
                self.u64(attempts.len() as u64);
                for attempt in attempts {
                    self.lineage(&attempt.id);
                    self.byte(attempt.kind as u8);
                    self.byte(attempt.direction as u8);
                    self.byte(attempt.support_a as u8);
                    self.byte(attempt.support_b as u8);
                    self.byte(attempt.disposition as u8);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair(a: u64, b: u64) -> PairIdentity {
        let collider = |handle| ColliderIdentity {
            body: BodyIdentity {
                handle,
                generation: 0,
            },
            collider: 0,
            subshape_path: vec![],
        };
        PairIdentity::new(3, 8, collider(a), collider(b)).unwrap()
    }

    #[test]
    fn pair_identity_is_canonical_under_body_order_swap() {
        assert_eq!(pair(2, 9), pair(9, 2));
    }

    #[test]
    fn rejects_self_pair() {
        let c = ColliderIdentity {
            body: BodyIdentity {
                handle: 4,
                generation: 1,
            },
            collider: 0,
            subshape_path: vec![],
        };
        assert_eq!(
            PairIdentity::new(1, 1, c.clone(), c),
            Err(LedgerError::SelfPair)
        );
    }

    #[test]
    fn split_then_reduce_preserves_exact_stage_accounting() {
        let p = pair(1, 2);
        let root = LineageId::root(p.clone());
        let a = root.child(0);
        let b = root.child(1);
        let mut l = PairLedger::new([p]).unwrap();
        for s in [
            Stage::Eligibility,
            Stage::TemporalBounds,
            Stage::Broadphase,
            Stage::Narrowphase,
        ] {
            l.record(StageEvent {
                stage: s,
                input: root.clone(),
                outcome: Outcome::Forward,
            })
            .unwrap();
        }
        l.record(StageEvent {
            stage: Stage::CandidateGeneration,
            input: root.clone(),
            outcome: Outcome::Spawn {
                reason: ForwardReason::CandidateAdmitted,
                children: vec![b.clone(), a.clone()],
            },
        })
        .unwrap();
        l.record(StageEvent {
            stage: Stage::ManifoldReduction,
            input: a.clone(),
            outcome: Outcome::ForwardWithReason(ForwardReason::ManifoldRetained),
        })
        .unwrap();
        l.record(StageEvent {
            stage: Stage::ManifoldReduction,
            input: b,
            outcome: Outcome::Terminate(TerminalReason::DeliberatelyReduced),
        })
        .unwrap();
        l.record(StageEvent {
            stage: Stage::SolverMapping,
            input: a.clone(),
            outcome: Outcome::ForwardWithReason(ForwardReason::SolverRowSubmitted),
        })
        .unwrap();
        l.record(StageEvent {
            stage: Stage::Solve,
            input: a,
            outcome: Outcome::Terminate(TerminalReason::Converged),
        })
        .unwrap();
        let r = l.receipt().unwrap();
        assert_eq!(
            (
                r.root_pairs,
                r.event_count,
                r.forwarded_count,
                r.spawned_count,
                r.terminal_count
            ),
            (1, 9, 6, 2, 2)
        );
        assert_eq!(r.stages[4].1.spawned, 2);
    }

    #[test]
    fn missing_disposition_is_rejected() {
        let p = pair(1, 2);
        let root = LineageId::root(p.clone());
        let mut l = PairLedger::new([p]).unwrap();
        l.record(StageEvent {
            stage: Stage::Eligibility,
            input: root,
            outcome: Outcome::Forward,
        })
        .unwrap();
        assert!(matches!(
            l.validate(),
            Err(LedgerError::StageInputMismatch {
                stage: Stage::TemporalBounds,
                missing: 1,
                ..
            })
        ));
    }

    #[test]
    fn unsupported_temporal_bound_must_fallback_not_prune() {
        let p = pair(1, 2);
        let root = LineageId::root(p.clone());
        let mut l = PairLedger::new([p]).unwrap();
        l.record(StageEvent {
            stage: Stage::Eligibility,
            input: root.clone(),
            outcome: Outcome::Forward,
        })
        .unwrap();
        l.record(StageEvent {
            stage: Stage::TemporalBounds,
            input: root,
            outcome: Outcome::Terminate(TerminalReason::UnsupportedGeometry),
        })
        .unwrap();
        assert!(matches!(
            l.validate(),
            Err(LedgerError::InvalidTerminalReason {
                stage: Stage::TemporalBounds,
                ..
            })
        ));
    }

    #[test]
    fn fingerprint_is_independent_of_event_insertion_order() {
        let p = pair(1, 2);
        let root = LineageId::root(p.clone());
        let events: Vec<_> = Stage::ORDER
            .into_iter()
            .map(|stage| StageEvent {
                stage,
                input: root.clone(),
                outcome: if stage == Stage::Solve {
                    Outcome::Terminate(TerminalReason::Converged)
                } else {
                    Outcome::Forward
                },
            })
            .collect();
        let mut a = PairLedger::new([p.clone()]).unwrap();
        let mut b = PairLedger::new([p]).unwrap();
        for e in &events {
            a.record(e.clone()).unwrap();
        }
        for e in events.into_iter().rev() {
            b.record(e).unwrap();
        }
        assert_eq!(a.receipt().unwrap(), b.receipt().unwrap());
    }

    #[test]
    fn candidate_trace_accounts_attempts_queries_and_each_rejection() {
        let p = pair(1, 2);
        let root = LineageId::root(p.clone());
        let primary = root.child(0);
        let admitted = root.child(1);
        let invalid = root.child(2);
        let failed = root.child(3);
        let shallow = root.child(4);
        let duplicate = root.child(5);
        let capacity = root.child(6);
        let attempt = |id, kind, direction, support_a, support_b, disposition| CandidateAttempt {
            id,
            kind,
            direction,
            support_a,
            support_b,
            disposition,
        };
        let attempts = vec![
            attempt(
                primary.clone(),
                CandidateAttemptKind::PrimaryWitness,
                DirectionDisposition::NotApplicable,
                SupportQueryDisposition::NotAttempted,
                SupportQueryDisposition::NotAttempted,
                CandidateDisposition::PrimaryAdmitted,
            ),
            attempt(
                admitted.clone(),
                CandidateAttemptKind::PerturbationDirection,
                DirectionDisposition::Generated,
                SupportQueryDisposition::Completed,
                SupportQueryDisposition::Completed,
                CandidateDisposition::Admitted,
            ),
            attempt(
                invalid,
                CandidateAttemptKind::PerturbationDirection,
                DirectionDisposition::InvalidOrDegenerate,
                SupportQueryDisposition::NotAttempted,
                SupportQueryDisposition::NotAttempted,
                CandidateDisposition::InvalidDirection,
            ),
            attempt(
                failed,
                CandidateAttemptKind::PerturbationDirection,
                DirectionDisposition::Generated,
                SupportQueryDisposition::Failed,
                SupportQueryDisposition::Completed,
                CandidateDisposition::SupportQueryFailed,
            ),
            attempt(
                shallow,
                CandidateAttemptKind::PerturbationDirection,
                DirectionDisposition::Generated,
                SupportQueryDisposition::Completed,
                SupportQueryDisposition::Completed,
                CandidateDisposition::ShallowRejected,
            ),
            attempt(
                duplicate,
                CandidateAttemptKind::PerturbationDirection,
                DirectionDisposition::Generated,
                SupportQueryDisposition::Completed,
                SupportQueryDisposition::Completed,
                CandidateDisposition::DuplicateRejected,
            ),
            attempt(
                capacity,
                CandidateAttemptKind::PerturbationDirection,
                DirectionDisposition::Generated,
                SupportQueryDisposition::Completed,
                SupportQueryDisposition::Completed,
                CandidateDisposition::CapacityRejected,
            ),
        ];

        let mut ledger = PairLedger::new([p]).unwrap();
        for stage in [
            Stage::Eligibility,
            Stage::TemporalBounds,
            Stage::Broadphase,
            Stage::Narrowphase,
        ] {
            ledger
                .record(StageEvent {
                    stage,
                    input: root.clone(),
                    outcome: Outcome::Forward,
                })
                .unwrap();
        }
        ledger
            .record(StageEvent {
                stage: Stage::CandidateGeneration,
                input: root,
                outcome: Outcome::CandidateTrace {
                    requested_directions: 6,
                    attempts,
                },
            })
            .unwrap();

        // Only the primary witness and explicitly admitted perturbation reach
        // manifold reduction; invalid directions and all rejected queries do not.
        ledger
            .record(StageEvent {
                stage: Stage::ManifoldReduction,
                input: primary,
                outcome: Outcome::Terminate(TerminalReason::DeliberatelyReduced),
            })
            .unwrap();
        ledger
            .record(StageEvent {
                stage: Stage::ManifoldReduction,
                input: admitted.clone(),
                outcome: Outcome::ForwardWithReason(ForwardReason::ManifoldRetained),
            })
            .unwrap();
        ledger
            .record(StageEvent {
                stage: Stage::SolverMapping,
                input: admitted.clone(),
                outcome: Outcome::ForwardWithReason(ForwardReason::SolverRowSubmitted),
            })
            .unwrap();
        ledger
            .record(StageEvent {
                stage: Stage::Solve,
                input: admitted,
                outcome: Outcome::Terminate(TerminalReason::Converged),
            })
            .unwrap();

        let receipt = ledger.receipt().unwrap();
        let candidate_stats = receipt
            .stages
            .iter()
            .find(|(stage, _)| *stage == Stage::CandidateGeneration)
            .unwrap()
            .1;
        assert_eq!(candidate_stats.candidate_attempts, 7);
        assert_eq!(candidate_stats.requested_directions, 6);
        assert_eq!(candidate_stats.generated_directions, 5);
        assert_eq!(candidate_stats.invalid_directions, 1);
        assert_eq!(candidate_stats.support_queries_attempted, 10);
        assert_eq!(candidate_stats.support_queries_completed, 9);
        assert_eq!(candidate_stats.support_queries_failed, 1);
        assert_eq!(candidate_stats.admitted_candidates, 2);
        assert_eq!(candidate_stats.shallow_rejections, 1);
        assert_eq!(candidate_stats.duplicate_rejections, 1);
        assert_eq!(candidate_stats.capacity_rejections, 1);
        assert_eq!(candidate_stats.terminal, 5);
    }

    #[test]
    fn candidate_trace_rejects_inconsistent_support_query_outcome() {
        let p = pair(1, 2);
        let root = LineageId::root(p.clone());
        let mut ledger = PairLedger::new([p]).unwrap();
        for stage in [
            Stage::Eligibility,
            Stage::TemporalBounds,
            Stage::Broadphase,
            Stage::Narrowphase,
        ] {
            ledger
                .record(StageEvent {
                    stage,
                    input: root.clone(),
                    outcome: Outcome::Forward,
                })
                .unwrap();
        }
        ledger
            .record(StageEvent {
                stage: Stage::CandidateGeneration,
                input: root.clone(),
                outcome: Outcome::CandidateTrace {
                    requested_directions: 1,
                    attempts: vec![
                        CandidateAttempt {
                            id: root.child(0),
                            kind: CandidateAttemptKind::PrimaryWitness,
                            direction: DirectionDisposition::NotApplicable,
                            support_a: SupportQueryDisposition::NotAttempted,
                            support_b: SupportQueryDisposition::NotAttempted,
                            disposition: CandidateDisposition::PrimaryAdmitted,
                        },
                        CandidateAttempt {
                            id: root.child(1),
                            kind: CandidateAttemptKind::PerturbationDirection,
                            direction: DirectionDisposition::Generated,
                            support_a: SupportQueryDisposition::Failed,
                            support_b: SupportQueryDisposition::Completed,
                            disposition: CandidateDisposition::Admitted,
                        },
                    ],
                },
            })
            .unwrap();
        assert_eq!(ledger.validate(), Err(LedgerError::InvalidCandidateAttempt));
    }

    #[test]
    fn candidate_trace_requires_exactly_one_primary_witness() {
        let p = pair(1, 2);
        let root = LineageId::root(p.clone());
        let mut ledger = PairLedger::new([p]).unwrap();
        for stage in [
            Stage::Eligibility,
            Stage::TemporalBounds,
            Stage::Broadphase,
            Stage::Narrowphase,
        ] {
            ledger
                .record(StageEvent {
                    stage,
                    input: root.clone(),
                    outcome: Outcome::Forward,
                })
                .unwrap();
        }
        ledger
            .record(StageEvent {
                stage: Stage::CandidateGeneration,
                input: root,
                outcome: Outcome::CandidateTrace {
                    requested_directions: 1,
                    attempts: vec![CandidateAttempt {
                        id: LineageId::root(pair(1, 2)).child(3),
                        kind: CandidateAttemptKind::PerturbationDirection,
                        direction: DirectionDisposition::InvalidOrDegenerate,
                        support_a: SupportQueryDisposition::NotAttempted,
                        support_b: SupportQueryDisposition::NotAttempted,
                        disposition: CandidateDisposition::InvalidDirection,
                    }],
                },
            })
            .unwrap();
        assert_eq!(ledger.validate(), Err(LedgerError::MissingPrimaryCandidate));
    }

    #[test]
    fn candidate_trace_detects_an_omitted_requested_direction() {
        let p = pair(1, 2);
        let root = LineageId::root(p.clone());
        let mut ledger = PairLedger::new([p]).unwrap();
        for stage in [
            Stage::Eligibility,
            Stage::TemporalBounds,
            Stage::Broadphase,
            Stage::Narrowphase,
        ] {
            ledger
                .record(StageEvent {
                    stage,
                    input: root.clone(),
                    outcome: Outcome::Forward,
                })
                .unwrap();
        }
        ledger
            .record(StageEvent {
                stage: Stage::CandidateGeneration,
                input: root.clone(),
                outcome: Outcome::CandidateTrace {
                    requested_directions: 2,
                    attempts: vec![
                        CandidateAttempt {
                            id: root.child(0),
                            kind: CandidateAttemptKind::PrimaryWitness,
                            direction: DirectionDisposition::NotApplicable,
                            support_a: SupportQueryDisposition::NotAttempted,
                            support_b: SupportQueryDisposition::NotAttempted,
                            disposition: CandidateDisposition::PrimaryAdmitted,
                        },
                        CandidateAttempt {
                            id: root.child(1),
                            kind: CandidateAttemptKind::PerturbationDirection,
                            direction: DirectionDisposition::InvalidOrDegenerate,
                            support_a: SupportQueryDisposition::NotAttempted,
                            support_b: SupportQueryDisposition::NotAttempted,
                            disposition: CandidateDisposition::InvalidDirection,
                        },
                    ],
                },
            })
            .unwrap();
        assert_eq!(
            ledger.validate(),
            Err(LedgerError::RequestedDirectionCountMismatch {
                requested: 2,
                recorded: 1,
            })
        );
    }

    #[test]
    fn child_cannot_change_pair_identity() {
        let p = pair(1, 2);
        let other = pair(1, 3);
        let root = LineageId::root(p.clone());
        let mut l = PairLedger::new([p]).unwrap();
        for s in [
            Stage::Eligibility,
            Stage::TemporalBounds,
            Stage::Broadphase,
            Stage::Narrowphase,
        ] {
            l.record(StageEvent {
                stage: s,
                input: root.clone(),
                outcome: Outcome::Forward,
            })
            .unwrap();
        }
        l.record(StageEvent {
            stage: Stage::CandidateGeneration,
            input: root,
            outcome: Outcome::Spawn {
                reason: ForwardReason::CandidateAdmitted,
                children: vec![LineageId::root(other)],
            },
        })
        .unwrap();
        assert_eq!(l.validate(), Err(LedgerError::ChildHasDifferentPair));
    }
}
