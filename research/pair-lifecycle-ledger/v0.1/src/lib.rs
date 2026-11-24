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
        let (first, second) = if left < right { (left, right) } else { (right, left) };
        Ok(Self { world_revision, step_index, first, second })
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
        Self { pair, path: Vec::new() }
    }

    #[must_use]
    pub fn child(&self, ordinal: u32) -> Self {
        let mut path = self.path.clone();
        path.push(ordinal);
        Self { pair: self.pair.clone(), path }
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
        Self::Eligibility, Self::TemporalBounds, Self::Broadphase, Self::Narrowphase,
        Self::CandidateGeneration, Self::ManifoldReduction, Self::SolverMapping, Self::Solve,
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Outcome {
    Forward,
    ForwardWithReason(ForwardReason),
    Spawn { reason: ForwardReason, children: Vec<LineageId> },
    Terminate(TerminalReason),
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
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum DispositionCode {
    Forwarded,
    ForwardedWithReason(ForwardReason),
    Spawned(ForwardReason),
    Terminated(TerminalReason),
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
    StageInputMismatch { stage: Stage, expected: usize, actual: usize, missing: usize, unexpected: usize },
    EmptySpawn,
    ChildHasDifferentPair,
    ChildPathNotImmediate,
    DuplicateChild,
    InvalidTerminalReason { stage: Stage, reason: TerminalReason },
    InvalidForwardReason { stage: Stage, reason: ForwardReason },
    FinalStageHasForwardedItems(usize),
}

impl fmt::Display for LedgerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SelfPair => write!(f, "a collider cannot be paired with itself"),
            Self::NonCanonicalPair => write!(f, "pair identity is not canonical"),
            Self::DuplicateRoot => write!(f, "duplicate root pair identity"),
            Self::DuplicateEvent(stage) => write!(f, "duplicate disposition at {stage:?}"),
            Self::StageInputMismatch { stage, expected, actual, missing, unexpected } => write!(
                f, "{stage:?} input mismatch: expected {expected}, found {actual} (missing {missing}, unexpected {unexpected})"
            ),
            Self::EmptySpawn => write!(f, "spawn disposition must create at least one child"),
            Self::ChildHasDifferentPair => write!(f, "child lineage changed pair identity"),
            Self::ChildPathNotImmediate => write!(f, "child path must extend parent by exactly one ordinal"),
            Self::DuplicateChild => write!(f, "duplicate child identity emitted"),
            Self::InvalidTerminalReason { stage, reason } => write!(f, "terminal reason {reason:?} is invalid at {stage:?}"),
            Self::InvalidForwardReason { stage, reason } => write!(f, "forward reason {reason:?} is invalid at {stage:?}"),
            Self::FinalStageHasForwardedItems(n) => write!(f, "solve stage left {n} item(s) without terminal disposition"),
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
        Ok(Self { roots, events: BTreeMap::new() })
    }

    pub fn record(&mut self, mut event: StageEvent) -> Result<(), LedgerError> {
        if let Outcome::Spawn { children, .. } = &mut event.outcome {
            children.sort();
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
                    Outcome::Forward => { next.insert(input); }
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
                        if children.is_empty() { return Err(LedgerError::EmptySpawn); }
                        for child in children {
                            if child.pair != input.pair {
                                return Err(LedgerError::ChildHasDifferentPair);
                            }
                            if child.path.len() != input.path.len() + 1 || !child.path.starts_with(&input.path) {
                                return Err(LedgerError::ChildPathNotImmediate);
                            }
                            if !next.insert(child) { return Err(LedgerError::DuplicateChild); }
                        }
                    }
                    Outcome::Terminate(reason) => {
                        if !valid_terminal(stage, reason) {
                            return Err(LedgerError::InvalidTerminalReason { stage, reason });
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
            let mut stats = StageStats { inputs: records.len(), ..StageStats::default() };
            for outcome in records.values() {
                let code = match outcome {
                    Outcome::Forward => { stats.forwarded += 1; DispositionCode::Forwarded }
                    Outcome::ForwardWithReason(r) => { stats.forwarded += 1; DispositionCode::ForwardedWithReason(*r) }
                    Outcome::Spawn { reason, children } => { stats.spawned += children.len(); DispositionCode::Spawned(*reason) }
                    Outcome::Terminate(r) => { stats.terminal += 1; DispositionCode::Terminated(*r) }
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
        for root in &self.roots { h.lineage(root); }
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

fn valid_forward(stage: Stage, reason: ForwardReason) -> bool {
    matches!(
        (stage, reason),
        (Stage::TemporalBounds, ForwardReason::UnsupportedBoundFallback | ForwardReason::InvalidBoundFallback | ForwardReason::UnboundedMotionFallback)
            | (Stage::Broadphase, ForwardReason::ExhaustiveBroadphaseFallback)
            | (Stage::Narrowphase, ForwardReason::UnsupportedShapeFallback | ForwardReason::ManifoldsGenerated)
            | (Stage::CandidateGeneration, ForwardReason::CandidateAdmitted)
            | (Stage::ManifoldReduction, ForwardReason::ManifoldRetained)
            | (Stage::SolverMapping, ForwardReason::SolverRowSubmitted)
    )
}

fn valid_terminal(stage: Stage, reason: TerminalReason) -> bool {
    match stage {
        Stage::Eligibility => matches!(reason, TerminalReason::CollisionFiltered | TerminalReason::InvalidInput | TerminalReason::Aborted),
        Stage::TemporalBounds => matches!(reason, TerminalReason::TemporallyProvenDisjoint | TerminalReason::Aborted),
        Stage::Broadphase => matches!(reason, TerminalReason::BroadphaseProvenDisjoint | TerminalReason::InvalidInput | TerminalReason::Aborted),
        Stage::Narrowphase => matches!(reason, TerminalReason::NarrowphaseNoContact | TerminalReason::UnsupportedGeometry | TerminalReason::InvalidInput | TerminalReason::Aborted),
        Stage::CandidateGeneration => matches!(reason, TerminalReason::SupportQueryFailed | TerminalReason::ShallowCandidate | TerminalReason::DuplicateCandidate | TerminalReason::CandidateCapacity | TerminalReason::InvalidInput | TerminalReason::Aborted),
        Stage::ManifoldReduction => matches!(reason, TerminalReason::DeliberatelyReduced | TerminalReason::ManifoldStorageOverflow | TerminalReason::InvalidInput | TerminalReason::Aborted),
        Stage::SolverMapping => matches!(reason, TerminalReason::SolverRuleRejected | TerminalReason::SolverUnsupported | TerminalReason::SolverNotSubmitted | TerminalReason::InvalidInput | TerminalReason::Aborted),
        Stage::Solve => matches!(reason, TerminalReason::Converged | TerminalReason::BudgetExhausted | TerminalReason::SolverFailed | TerminalReason::SleepingSkip | TerminalReason::Aborted | TerminalReason::NotRun),
    }
}

struct Fnv64(u64);
impl Fnv64 {
    fn byte(&mut self, b: u8) { self.0 = (self.0 ^ u64::from(b)).wrapping_mul(0x100000001b3); }
    fn bytes(&mut self, bytes: &[u8]) { for b in bytes { self.byte(*b); } }
    fn u32(&mut self, n: u32) { self.bytes(&n.to_le_bytes()); }
    fn u64(&mut self, n: u64) { self.bytes(&n.to_le_bytes()); }
    fn pair(&mut self, p: &PairIdentity) {
        self.u64(p.world_revision);
        self.u64(p.step_index);
        for c in [&p.first, &p.second] {
            self.u64(c.body.handle);
            self.u32(c.body.generation);
            self.u32(c.collider);
            self.u64(c.subshape_path.len() as u64);
            for part in &c.subshape_path { self.u32(*part); }
        }
    }
    fn lineage(&mut self, id: &LineageId) {
        self.pair(&id.pair);
        self.u64(id.path.len() as u64);
        for part in &id.path { self.u32(*part); }
    }
    fn outcome(&mut self, o: &Outcome) {
        match o {
            Outcome::Forward => self.byte(0),
            Outcome::ForwardWithReason(r) => { self.byte(1); self.byte(*r as u8); }
            Outcome::Spawn { reason, children } => {
                self.byte(2);
                self.byte(*reason as u8);
                self.u64(children.len() as u64);
                for child in children { self.lineage(child); }
            }
            Outcome::Terminate(r) => { self.byte(3); self.byte(*r as u8); }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair(a: u64, b: u64) -> PairIdentity {
        let collider = |handle| ColliderIdentity {
            body: BodyIdentity { handle, generation: 0 },
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
        let c = ColliderIdentity { body: BodyIdentity { handle: 4, generation: 1 }, collider: 0, subshape_path: vec![] };
        assert_eq!(PairIdentity::new(1, 1, c.clone(), c), Err(LedgerError::SelfPair));
    }

    #[test]
    fn split_then_reduce_preserves_exact_stage_accounting() {
        let p = pair(1, 2);
        let root = LineageId::root(p.clone());
        let a = root.child(0);
        let b = root.child(1);
        let mut l = PairLedger::new([p]).unwrap();
        for s in [Stage::Eligibility, Stage::TemporalBounds, Stage::Broadphase, Stage::Narrowphase] {
            l.record(StageEvent { stage: s, input: root.clone(), outcome: Outcome::Forward }).unwrap();
        }
        l.record(StageEvent {
            stage: Stage::CandidateGeneration,
            input: root.clone(),
            outcome: Outcome::Spawn { reason: ForwardReason::CandidateAdmitted, children: vec![b.clone(), a.clone()] },
        }).unwrap();
        l.record(StageEvent { stage: Stage::ManifoldReduction, input: a.clone(), outcome: Outcome::ForwardWithReason(ForwardReason::ManifoldRetained) }).unwrap();
        l.record(StageEvent { stage: Stage::ManifoldReduction, input: b, outcome: Outcome::Terminate(TerminalReason::DeliberatelyReduced) }).unwrap();
        l.record(StageEvent { stage: Stage::SolverMapping, input: a.clone(), outcome: Outcome::ForwardWithReason(ForwardReason::SolverRowSubmitted) }).unwrap();
        l.record(StageEvent { stage: Stage::Solve, input: a, outcome: Outcome::Terminate(TerminalReason::Converged) }).unwrap();
        let r = l.receipt().unwrap();
        assert_eq!((r.root_pairs, r.event_count, r.forwarded_count, r.spawned_count, r.terminal_count), (1, 9, 6, 2, 2));
        assert_eq!(r.stages[4].1.spawned, 2);
    }

    #[test]
    fn missing_disposition_is_rejected() {
        let p = pair(1, 2);
        let root = LineageId::root(p.clone());
        let mut l = PairLedger::new([p]).unwrap();
        l.record(StageEvent { stage: Stage::Eligibility, input: root, outcome: Outcome::Forward }).unwrap();
        assert!(matches!(l.validate(), Err(LedgerError::StageInputMismatch { stage: Stage::TemporalBounds, missing: 1, .. })));
    }

    #[test]
    fn unsupported_temporal_bound_must_fallback_not_prune() {
        let p = pair(1, 2);
        let root = LineageId::root(p.clone());
        let mut l = PairLedger::new([p]).unwrap();
        l.record(StageEvent { stage: Stage::Eligibility, input: root.clone(), outcome: Outcome::Forward }).unwrap();
        l.record(StageEvent { stage: Stage::TemporalBounds, input: root, outcome: Outcome::Terminate(TerminalReason::UnsupportedGeometry) }).unwrap();
        assert!(matches!(l.validate(), Err(LedgerError::InvalidTerminalReason { stage: Stage::TemporalBounds, .. })));
    }

    #[test]
    fn fingerprint_is_independent_of_event_insertion_order() {
        let p = pair(1, 2);
        let root = LineageId::root(p.clone());
        let events: Vec<_> = Stage::ORDER.into_iter().map(|stage| StageEvent {
            stage,
            input: root.clone(),
            outcome: if stage == Stage::Solve { Outcome::Terminate(TerminalReason::Converged) } else { Outcome::Forward },
        }).collect();
        let mut a = PairLedger::new([p.clone()]).unwrap();
        let mut b = PairLedger::new([p]).unwrap();
        for e in &events { a.record(e.clone()).unwrap(); }
        for e in events.into_iter().rev() { b.record(e).unwrap(); }
        assert_eq!(a.receipt().unwrap(), b.receipt().unwrap());
    }

    #[test]
    fn child_cannot_change_pair_identity() {
        let p = pair(1, 2);
        let other = pair(1, 3);
        let root = LineageId::root(p.clone());
        let mut l = PairLedger::new([p]).unwrap();
        for s in [Stage::Eligibility, Stage::TemporalBounds, Stage::Broadphase, Stage::Narrowphase] {
            l.record(StageEvent { stage: s, input: root.clone(), outcome: Outcome::Forward }).unwrap();
        }
        l.record(StageEvent { stage: Stage::CandidateGeneration, input: root, outcome: Outcome::Spawn {
            reason: ForwardReason::CandidateAdmitted,
            children: vec![LineageId::root(other)],
        }}).unwrap();
        assert_eq!(l.validate(), Err(LedgerError::ChildHasDifferentPair));
    }
}
