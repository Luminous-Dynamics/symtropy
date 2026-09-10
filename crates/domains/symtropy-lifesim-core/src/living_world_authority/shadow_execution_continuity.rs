// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Machine-checkable continuity transcripts for Q2 shadow execution.
//!
//! #563/#567 authenticate which coarse/reference observation states belong to
//! one Q2 experiment, but a flat collection of state identities is not itself a
//! predecessor/successor history. This layer requires one exact one-tick segment
//! for every canonical tick in the Q2 window and chains those segments from the
//! authenticated T0 state through every observed state.
//!
//! This remains structural evidence. A caller can submit segment manifests, so
//! #568 must still executable-qualify the runner/profile that emits them before
//! this evidence is eligible for closure-anchor renewal.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use crate::information::RepresentationKey;
use crate::population_manifest::PopulationStateManifest;

use super::retained_authority::{
    RetainedAuthorityRecord, RetainedContentManifest, RetainedStoreRevision,
};
use super::shadow_execution_lineage::{ShadowReferenceRunId, ShadowReferenceRunRevision};
use super::shadow_observable_authority::{
    ShadowObservationContentManifest, ShadowObservationSourceIdentity,
};
use super::shadow_paired_execution::{
    PairedShadowExecutionCertificate, ShadowCoarseRunId, ShadowCoarseRunRevision,
};
use super::shadow_validation::{
    ShadowEvidenceKey, ShadowEvidenceRevision, ShadowValidationWindow,
};
use super::spatiotemporal_information::CanonicalTick;
use super::transition_domain::{
    TransitionDomainAuthorityScope, TransitionDomainEvaluationSubject,
    TransitionDomainSnapshotId, TransitionDomainSourceRevision,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShadowExecutionSegmentId(pub u128);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShadowExecutionSegmentManifest(Vec<u8>);

impl ShadowExecutionSegmentManifest {
    pub fn new(bytes: impl Into<Vec<u8>>) -> Result<Self, ShadowExecutionContinuityError> {
        let bytes = bytes.into();
        if bytes.is_empty() {
            return Err(ShadowExecutionContinuityError::EmptySegmentManifest);
        }
        Ok(Self(bytes))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// Revision grammar for exact continuity states.
///
/// Retained-store revisions and canonical representation-state revisions are
/// intentionally distinct. Equal numeric values do not imply equal authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShadowContinuityStateRevision {
    Canonical(TransitionDomainSourceRevision),
    Retained(RetainedStoreRevision),
}

/// Exact content authority already owned by an upstream state identity.
///
/// No new digest is derived here. Keeping the content classes distinct prevents
/// byte equality across unrelated encodings from being mistaken for state
/// identity equality.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShadowContinuityContentIdentity {
    CoarsePopulation(PopulationStateManifest),
    RetainedExact(RetainedContentManifest),
    Observation(ShadowObservationContentManifest),
}

/// Normalized exact state identity consumed by the continuity theorem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowContinuityStateIdentity {
    representation: RepresentationKey,
    scope: TransitionDomainAuthorityScope,
    snapshot: TransitionDomainSnapshotId,
    revision: ShadowContinuityStateRevision,
    content: ShadowContinuityContentIdentity,
}

impl ShadowContinuityStateIdentity {
    pub fn from_coarse_subject(subject: &TransitionDomainEvaluationSubject) -> Self {
        Self {
            representation: subject.source_representation(),
            scope: subject.scope(),
            snapshot: subject.snapshot(),
            revision: ShadowContinuityStateRevision::Canonical(subject.source_revision()),
            content: ShadowContinuityContentIdentity::CoarsePopulation(
                subject.population_manifest().clone(),
            ),
        }
    }

    pub fn from_retained_record(record: &RetainedAuthorityRecord) -> Self {
        Self {
            representation: record.store_representation(),
            scope: record.scope(),
            snapshot: record.snapshot(),
            revision: ShadowContinuityStateRevision::Retained(record.revision()),
            content: ShadowContinuityContentIdentity::RetainedExact(
                record.content_manifest().clone(),
            ),
        }
    }

    pub fn from_observation_source(source: &ShadowObservationSourceIdentity) -> Self {
        Self {
            representation: source.representation(),
            scope: source.scope(),
            snapshot: source.snapshot(),
            revision: ShadowContinuityStateRevision::Canonical(source.source_revision()),
            content: ShadowContinuityContentIdentity::Observation(
                source.content_manifest().clone(),
            ),
        }
    }

    pub const fn representation(&self) -> RepresentationKey {
        self.representation
    }

    pub const fn scope(&self) -> TransitionDomainAuthorityScope {
        self.scope
    }

    pub const fn snapshot(&self) -> TransitionDomainSnapshotId {
        self.snapshot
    }

    pub const fn revision(&self) -> ShadowContinuityStateRevision {
        self.revision
    }

    pub const fn content(&self) -> &ShadowContinuityContentIdentity {
        &self.content
    }
}

/// One executor-reported state transition between consecutive canonical ticks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowExecutionSegment {
    id: ShadowExecutionSegmentId,
    from_tick: CanonicalTick,
    to_tick: CanonicalTick,
    predecessor: ShadowContinuityStateIdentity,
    successor: ShadowContinuityStateIdentity,
    execution_manifest: ShadowExecutionSegmentManifest,
}

impl ShadowExecutionSegment {
    pub fn new(
        id: ShadowExecutionSegmentId,
        from_tick: CanonicalTick,
        to_tick: CanonicalTick,
        predecessor: ShadowContinuityStateIdentity,
        successor: ShadowContinuityStateIdentity,
        execution_manifest: ShadowExecutionSegmentManifest,
    ) -> Result<Self, ShadowExecutionContinuityError> {
        let expected_to = from_tick
            .0
            .checked_add(1)
            .ok_or(ShadowExecutionContinuityError::TickOverflow { tick: from_tick })?;
        if to_tick.0 != expected_to {
            return Err(ShadowExecutionContinuityError::SegmentNotOneCanonicalTick {
                from: from_tick,
                to: to_tick,
            });
        }
        Ok(Self {
            id,
            from_tick,
            to_tick,
            predecessor,
            successor,
            execution_manifest,
        })
    }

    pub const fn id(&self) -> ShadowExecutionSegmentId {
        self.id
    }

    pub const fn from_tick(&self) -> CanonicalTick {
        self.from_tick
    }

    pub const fn to_tick(&self) -> CanonicalTick {
        self.to_tick
    }

    pub const fn predecessor(&self) -> &ShadowContinuityStateIdentity {
        &self.predecessor
    }

    pub const fn successor(&self) -> &ShadowContinuityStateIdentity {
        &self.successor
    }

    pub const fn execution_manifest(&self) -> &ShadowExecutionSegmentManifest {
        &self.execution_manifest
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShadowContinuityRunIdentity {
    Coarse {
        id: ShadowCoarseRunId,
        revision: ShadowCoarseRunRevision,
    },
    Reference {
        id: ShadowReferenceRunId,
        revision: ShadowReferenceRunRevision,
    },
}

/// Canonical, gap-free continuity transcript for one Q2 execution lane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowLaneContinuityTranscript {
    run: ShadowContinuityRunIdentity,
    evidence: ShadowEvidenceKey,
    evidence_revision: ShadowEvidenceRevision,
    window: ShadowValidationWindow,
    segments: BTreeMap<CanonicalTick, ShadowExecutionSegment>,
}

impl ShadowLaneContinuityTranscript {
    pub fn new(
        run: ShadowContinuityRunIdentity,
        evidence: ShadowEvidenceKey,
        evidence_revision: ShadowEvidenceRevision,
        window: ShadowValidationWindow,
        segments: impl IntoIterator<Item = ShadowExecutionSegment>,
    ) -> Result<Self, ShadowExecutionContinuityError> {
        let expected_len = usize::try_from(window.duration_ticks()).map_err(|_| {
            ShadowExecutionContinuityError::WindowTooLargeForTranscript {
                duration_ticks: window.duration_ticks(),
            }
        })?;
        let mut canonical = BTreeMap::new();
        let mut segment_ids = BTreeSet::new();

        for segment in segments {
            if !segment_ids.insert(segment.id()) {
                return Err(ShadowExecutionContinuityError::DuplicateSegmentId {
                    id: segment.id(),
                });
            }
            let to_tick = segment.to_tick();
            if canonical.insert(to_tick, segment).is_some() {
                return Err(ShadowExecutionContinuityError::DuplicateSegmentEndTick {
                    tick: to_tick,
                });
            }
        }

        if canonical.len() != expected_len {
            return Err(ShadowExecutionContinuityError::IncompleteTranscriptCoverage {
                expected: expected_len,
                actual: canonical.len(),
            });
        }

        let mut expected_from = window.start_exclusive();
        let mut previous_successor: Option<&ShadowContinuityStateIdentity> = None;
        for segment in canonical.values() {
            if segment.from_tick() != expected_from {
                return Err(ShadowExecutionContinuityError::TranscriptTickGap {
                    expected_from,
                    actual_from: segment.from_tick(),
                });
            }
            if let Some(previous) = previous_successor {
                if segment.predecessor() != previous {
                    return Err(ShadowExecutionContinuityError::StateChainBroken {
                        tick: segment.from_tick(),
                    });
                }
            }
            expected_from = segment.to_tick();
            previous_successor = Some(segment.successor());
        }

        if expected_from != window.end_inclusive() {
            return Err(ShadowExecutionContinuityError::TranscriptDoesNotReachWindowEnd {
                expected: window.end_inclusive(),
                actual: expected_from,
            });
        }

        Ok(Self {
            run,
            evidence,
            evidence_revision,
            window,
            segments: canonical,
        })
    }

    pub const fn run(&self) -> ShadowContinuityRunIdentity {
        self.run
    }

    pub const fn evidence(&self) -> ShadowEvidenceKey {
        self.evidence
    }

    pub const fn evidence_revision(&self) -> ShadowEvidenceRevision {
        self.evidence_revision
    }

    pub const fn window(&self) -> ShadowValidationWindow {
        self.window
    }

    pub fn segments(&self) -> impl Iterator<Item = &ShadowExecutionSegment> {
        self.segments.values()
    }

    pub fn len(&self) -> usize {
        self.segments.len()
    }

    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    fn first_predecessor(&self) -> Option<&ShadowContinuityStateIdentity> {
        self.segments
            .values()
            .next()
            .map(ShadowExecutionSegment::predecessor)
    }
}

/// Paired structural continuity proof over an already-certified #567 experiment.
///
/// This type does not re-run #567's large current-authority validation surface.
/// Consequential callers must first revalidate the `PairedShadowExecutionCertificate`
/// against its current registries, then call `validate_against_pair` here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairedShadowContinuityCertificate {
    paired_execution: PairedShadowExecutionCertificate,
    coarse: ShadowLaneContinuityTranscript,
    reference: ShadowLaneContinuityTranscript,
}

impl PairedShadowContinuityCertificate {
    pub fn certify(
        paired_execution: &PairedShadowExecutionCertificate,
        coarse: &ShadowLaneContinuityTranscript,
        reference: &ShadowLaneContinuityTranscript,
    ) -> Result<Self, ShadowExecutionContinuityError> {
        let coarse_run = paired_execution.coarse_run();
        let expected_coarse_run = ShadowContinuityRunIdentity::Coarse {
            id: coarse_run.id(),
            revision: coarse_run.revision(),
        };
        let coarse_observations = coarse_run
            .observed_states()
            .iter()
            .map(|(tick, state)| {
                (
                    *tick,
                    ShadowContinuityStateIdentity::from_observation_source(state.source()),
                )
            })
            .collect::<BTreeMap<_, _>>();
        validate_transcript_against_expected(
            coarse,
            expected_coarse_run,
            coarse_run.shadow_evidence(),
            coarse_run.shadow_evidence_revision(),
            coarse_run.window(),
            &ShadowContinuityStateIdentity::from_coarse_subject(coarse_run.start()),
            &coarse_observations,
        )?;

        let reference_certificate = paired_execution.reference();
        let reference_run = reference_certificate.run();
        let expected_reference_run = ShadowContinuityRunIdentity::Reference {
            id: reference_run.id(),
            revision: reference_run.revision(),
        };
        let reference_start_record = reference_certificate
            .retained_start()
            .retained_start()
            .record();
        let reference_observations = reference_run
            .observed_states()
            .iter()
            .map(|(tick, state)| {
                (
                    *tick,
                    ShadowContinuityStateIdentity::from_observation_source(state.source()),
                )
            })
            .collect::<BTreeMap<_, _>>();
        validate_transcript_against_expected(
            reference,
            expected_reference_run,
            reference_run.shadow_evidence(),
            reference_run.shadow_evidence_revision(),
            reference_run.window(),
            &ShadowContinuityStateIdentity::from_retained_record(reference_start_record),
            &reference_observations,
        )?;

        if coarse.evidence() != reference.evidence()
            || coarse.evidence_revision() != reference.evidence_revision()
        {
            return Err(ShadowExecutionContinuityError::LaneEvidenceMismatch);
        }
        if coarse.window() != reference.window() {
            return Err(ShadowExecutionContinuityError::LaneWindowMismatch);
        }

        Ok(Self {
            paired_execution: paired_execution.clone(),
            coarse: coarse.clone(),
            reference: reference.clone(),
        })
    }

    pub const fn paired_execution(&self) -> &PairedShadowExecutionCertificate {
        &self.paired_execution
    }

    pub const fn coarse(&self) -> &ShadowLaneContinuityTranscript {
        &self.coarse
    }

    pub const fn reference(&self) -> &ShadowLaneContinuityTranscript {
        &self.reference
    }

    /// Recompute this structural certificate against an already-current #567
    /// certificate. Revalidating #567 itself remains the caller's responsibility.
    pub fn validate_against_pair(
        &self,
        current_pair: &PairedShadowExecutionCertificate,
    ) -> Result<(), ShadowExecutionContinuityError> {
        let current = Self::certify(current_pair, &self.coarse, &self.reference)?;
        if current != *self {
            return Err(ShadowExecutionContinuityError::CertificateStale);
        }
        Ok(())
    }
}

fn validate_transcript_against_expected(
    transcript: &ShadowLaneContinuityTranscript,
    expected_run: ShadowContinuityRunIdentity,
    expected_evidence: ShadowEvidenceKey,
    expected_evidence_revision: ShadowEvidenceRevision,
    expected_window: ShadowValidationWindow,
    expected_start: &ShadowContinuityStateIdentity,
    expected_observations: &BTreeMap<CanonicalTick, ShadowContinuityStateIdentity>,
) -> Result<(), ShadowExecutionContinuityError> {
    if transcript.run() != expected_run {
        return Err(ShadowExecutionContinuityError::RunIdentityMismatch);
    }
    if transcript.evidence() != expected_evidence
        || transcript.evidence_revision() != expected_evidence_revision
    {
        return Err(ShadowExecutionContinuityError::EvidenceIdentityMismatch);
    }
    if transcript.window() != expected_window {
        return Err(ShadowExecutionContinuityError::WindowMismatch {
            expected: expected_window,
            actual: transcript.window(),
        });
    }
    if transcript.first_predecessor() != Some(expected_start) {
        return Err(ShadowExecutionContinuityError::StartStateMismatch);
    }
    if transcript.len() != expected_observations.len() {
        return Err(ShadowExecutionContinuityError::ObservationCoverageMismatch {
            transcript: transcript.len(),
            observations: expected_observations.len(),
        });
    }

    for segment in transcript.segments() {
        let expected = expected_observations.get(&segment.to_tick()).ok_or(
            ShadowExecutionContinuityError::MissingExpectedObservation {
                tick: segment.to_tick(),
            },
        )?;
        if segment.successor() != expected {
            return Err(ShadowExecutionContinuityError::ObservationStateMismatch {
                tick: segment.to_tick(),
            });
        }
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShadowExecutionContinuityError {
    EmptySegmentManifest,
    TickOverflow {
        tick: CanonicalTick,
    },
    SegmentNotOneCanonicalTick {
        from: CanonicalTick,
        to: CanonicalTick,
    },
    WindowTooLargeForTranscript {
        duration_ticks: u64,
    },
    DuplicateSegmentId {
        id: ShadowExecutionSegmentId,
    },
    DuplicateSegmentEndTick {
        tick: CanonicalTick,
    },
    IncompleteTranscriptCoverage {
        expected: usize,
        actual: usize,
    },
    TranscriptTickGap {
        expected_from: CanonicalTick,
        actual_from: CanonicalTick,
    },
    StateChainBroken {
        tick: CanonicalTick,
    },
    TranscriptDoesNotReachWindowEnd {
        expected: CanonicalTick,
        actual: CanonicalTick,
    },
    RunIdentityMismatch,
    EvidenceIdentityMismatch,
    WindowMismatch {
        expected: ShadowValidationWindow,
        actual: ShadowValidationWindow,
    },
    StartStateMismatch,
    ObservationCoverageMismatch {
        transcript: usize,
        observations: usize,
    },
    MissingExpectedObservation {
        tick: CanonicalTick,
    },
    ObservationStateMismatch {
        tick: CanonicalTick,
    },
    LaneEvidenceMismatch,
    LaneWindowMismatch,
    CertificateStale,
}

impl fmt::Display for ShadowExecutionContinuityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySegmentManifest => write!(f, "shadow execution segment manifest is empty"),
            Self::TickOverflow { tick } => {
                write!(f, "shadow continuity tick overflow after {}", tick.0)
            }
            Self::SegmentNotOneCanonicalTick { from, to } => write!(
                f,
                "shadow continuity segment must span exactly one canonical tick: {} -> {}",
                from.0, to.0
            ),
            Self::WindowTooLargeForTranscript { duration_ticks } => write!(
                f,
                "shadow continuity window of {duration_ticks} ticks cannot be represented on this platform"
            ),
            Self::DuplicateSegmentId { id } => {
                write!(f, "shadow continuity segment id {} is duplicated", id.0)
            }
            Self::DuplicateSegmentEndTick { tick } => write!(
                f,
                "multiple shadow continuity segments end at tick {}",
                tick.0
            ),
            Self::IncompleteTranscriptCoverage { expected, actual } => write!(
                f,
                "shadow continuity transcript has {actual} segments; expected {expected}"
            ),
            Self::TranscriptTickGap {
                expected_from,
                actual_from,
            } => write!(
                f,
                "shadow continuity expected next segment from tick {}, got {}",
                expected_from.0, actual_from.0
            ),
            Self::StateChainBroken { tick } => write!(
                f,
                "shadow continuity predecessor does not equal prior successor at tick {}",
                tick.0
            ),
            Self::TranscriptDoesNotReachWindowEnd { expected, actual } => write!(
                f,
                "shadow continuity transcript ended at tick {}, expected {}",
                actual.0, expected.0
            ),
            Self::RunIdentityMismatch => write!(f, "shadow continuity run identity changed"),
            Self::EvidenceIdentityMismatch => {
                write!(f, "shadow continuity Q2 evidence identity changed")
            }
            Self::WindowMismatch { expected, actual } => write!(
                f,
                "shadow continuity window changed from ({}, {}] to ({}, {}]",
                expected.start_exclusive().0,
                expected.end_inclusive().0,
                actual.start_exclusive().0,
                actual.end_inclusive().0
            ),
            Self::StartStateMismatch => write!(
                f,
                "shadow continuity transcript does not begin from authenticated T0 state"
            ),
            Self::ObservationCoverageMismatch {
                transcript,
                observations,
            } => write!(
                f,
                "shadow continuity transcript has {transcript} checkpoints but run has {observations} observations"
            ),
            Self::MissingExpectedObservation { tick } => write!(
                f,
                "shadow continuity segment at tick {} has no run observation",
                tick.0
            ),
            Self::ObservationStateMismatch { tick } => write!(
                f,
                "shadow continuity successor differs from exact observed state at tick {}",
                tick.0
            ),
            Self::LaneEvidenceMismatch => write!(
                f,
                "coarse/reference continuity transcripts bind different Q2 evidence"
            ),
            Self::LaneWindowMismatch => write!(
                f,
                "coarse/reference continuity transcripts bind different windows"
            ),
            Self::CertificateStale => {
                write!(f, "paired shadow continuity certificate is stale")
            }
        }
    }
}

impl Error for ShadowExecutionContinuityError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn window() -> ShadowValidationWindow {
        ShadowValidationWindow::new(CanonicalTick(10), CanonicalTick(13)).unwrap()
    }

    fn state(revision: u64, content: u8) -> ShadowContinuityStateIdentity {
        let source = ShadowObservationSourceIdentity::new(
            RepresentationKey::new(7, 1),
            TransitionDomainAuthorityScope::new(11, 1),
            TransitionDomainSnapshotId(13),
            TransitionDomainSourceRevision(revision),
            ShadowObservationContentManifest::new(vec![content]).unwrap(),
        );
        ShadowContinuityStateIdentity::from_observation_source(&source)
    }

    /// Structural tests deliberately construct this private identity directly.
    /// Production coarse T0 identities can only enter through `from_coarse_subject`.
    fn coarse_start(content: u8) -> ShadowContinuityStateIdentity {
        ShadowContinuityStateIdentity {
            representation: RepresentationKey::new(7, 1),
            scope: TransitionDomainAuthorityScope::new(11, 1),
            snapshot: TransitionDomainSnapshotId(13),
            revision: ShadowContinuityStateRevision::Canonical(
                TransitionDomainSourceRevision(10),
            ),
            content: ShadowContinuityContentIdentity::Observation(
                ShadowObservationContentManifest::new(vec![content]).unwrap(),
            ),
        }
    }

    fn segment(
        id: u128,
        from: u64,
        predecessor: ShadowContinuityStateIdentity,
        successor: ShadowContinuityStateIdentity,
    ) -> ShadowExecutionSegment {
        ShadowExecutionSegment::new(
            ShadowExecutionSegmentId(id),
            CanonicalTick(from),
            CanonicalTick(from + 1),
            predecessor,
            successor,
            ShadowExecutionSegmentManifest::new(vec![id as u8]).unwrap(),
        )
        .unwrap()
    }

    fn valid_transcript() -> ShadowLaneContinuityTranscript {
        let s0 = coarse_start(1);
        let s1 = state(11, 2);
        let s2 = state(12, 3);
        let s3 = state(13, 4);
        ShadowLaneContinuityTranscript::new(
            ShadowContinuityRunIdentity::Coarse {
                id: ShadowCoarseRunId(1),
                revision: ShadowCoarseRunRevision(1),
            },
            ShadowEvidenceKey::new(2, 1),
            ShadowEvidenceRevision(1),
            window(),
            [
                segment(1, 10, s0, s1.clone()),
                segment(2, 11, s1, s2.clone()),
                segment(3, 12, s2, s3),
            ],
        )
        .unwrap()
    }

    fn expected_observations() -> BTreeMap<CanonicalTick, ShadowContinuityStateIdentity> {
        BTreeMap::from([
            (CanonicalTick(11), state(11, 2)),
            (CanonicalTick(12), state(12, 3)),
            (CanonicalTick(13), state(13, 4)),
        ])
    }

    #[test]
    fn valid_one_tick_chain_covers_complete_window() {
        let transcript = valid_transcript();
        assert_eq!(transcript.len(), 3);
        assert_eq!(
            transcript.segments().last().unwrap().to_tick(),
            window().end_inclusive()
        );
    }

    #[test]
    fn segment_must_span_exactly_one_canonical_tick() {
        let result = ShadowExecutionSegment::new(
            ShadowExecutionSegmentId(1),
            CanonicalTick(10),
            CanonicalTick(12),
            coarse_start(1),
            state(12, 3),
            ShadowExecutionSegmentManifest::new(vec![1]).unwrap(),
        );
        assert!(matches!(
            result,
            Err(ShadowExecutionContinuityError::SegmentNotOneCanonicalTick { .. })
        ));
    }

    #[test]
    fn duplicate_segment_identity_rejects() {
        let s0 = coarse_start(1);
        let s1 = state(11, 2);
        let s2 = state(12, 3);
        let s3 = state(13, 4);
        let result = ShadowLaneContinuityTranscript::new(
            ShadowContinuityRunIdentity::Coarse {
                id: ShadowCoarseRunId(1),
                revision: ShadowCoarseRunRevision(1),
            },
            ShadowEvidenceKey::new(2, 1),
            ShadowEvidenceRevision(1),
            window(),
            [
                segment(1, 10, s0, s1.clone()),
                segment(1, 11, s1, s2.clone()),
                segment(3, 12, s2, s3),
            ],
        );
        assert!(matches!(
            result,
            Err(ShadowExecutionContinuityError::DuplicateSegmentId { .. })
        ));
    }

    #[test]
    fn restart_or_splice_with_different_boundary_state_rejects() {
        let s0 = coarse_start(1);
        let s1 = state(11, 2);
        let s2 = state(12, 3);
        let substituted_s1 = state(11, 99);
        let s3 = state(13, 4);
        let result = ShadowLaneContinuityTranscript::new(
            ShadowContinuityRunIdentity::Coarse {
                id: ShadowCoarseRunId(1),
                revision: ShadowCoarseRunRevision(1),
            },
            ShadowEvidenceKey::new(2, 1),
            ShadowEvidenceRevision(1),
            window(),
            [
                segment(1, 10, s0, s1),
                segment(2, 11, substituted_s1, s2.clone()),
                segment(3, 12, s2, s3),
            ],
        );
        assert!(matches!(
            result,
            Err(ShadowExecutionContinuityError::StateChainBroken {
                tick: CanonicalTick(11)
            })
        ));
    }

    #[test]
    fn same_revision_with_different_content_is_not_same_state() {
        assert_ne!(state(11, 2), state(11, 99));
    }

    #[test]
    fn exact_observation_checkpoint_mismatch_rejects() {
        let transcript = valid_transcript();
        let mut observations = expected_observations();
        observations.insert(CanonicalTick(12), state(12, 99));
        let result = validate_transcript_against_expected(
            &transcript,
            ShadowContinuityRunIdentity::Coarse {
                id: ShadowCoarseRunId(1),
                revision: ShadowCoarseRunRevision(1),
            },
            ShadowEvidenceKey::new(2, 1),
            ShadowEvidenceRevision(1),
            window(),
            &coarse_start(1),
            &observations,
        );
        assert!(matches!(
            result,
            Err(ShadowExecutionContinuityError::ObservationStateMismatch {
                tick: CanonicalTick(12)
            })
        ));
    }

    #[test]
    fn wrong_authenticated_start_rejects() {
        let transcript = valid_transcript();
        let result = validate_transcript_against_expected(
            &transcript,
            ShadowContinuityRunIdentity::Coarse {
                id: ShadowCoarseRunId(1),
                revision: ShadowCoarseRunRevision(1),
            },
            ShadowEvidenceKey::new(2, 1),
            ShadowEvidenceRevision(1),
            window(),
            &coarse_start(99),
            &expected_observations(),
        );
        assert_eq!(
            result,
            Err(ShadowExecutionContinuityError::StartStateMismatch)
        );
    }

    #[test]
    fn missing_checkpoint_rejects_against_run_observations() {
        let transcript = valid_transcript();
        let mut observations = expected_observations();
        observations.remove(&CanonicalTick(12));
        let result = validate_transcript_against_expected(
            &transcript,
            ShadowContinuityRunIdentity::Coarse {
                id: ShadowCoarseRunId(1),
                revision: ShadowCoarseRunRevision(1),
            },
            ShadowEvidenceKey::new(2, 1),
            ShadowEvidenceRevision(1),
            window(),
            &coarse_start(1),
            &observations,
        );
        assert!(matches!(
            result,
            Err(ShadowExecutionContinuityError::ObservationCoverageMismatch { .. })
        ));
    }

    #[test]
    fn wrong_run_identity_rejects() {
        let transcript = valid_transcript();
        let result = validate_transcript_against_expected(
            &transcript,
            ShadowContinuityRunIdentity::Coarse {
                id: ShadowCoarseRunId(2),
                revision: ShadowCoarseRunRevision(1),
            },
            ShadowEvidenceKey::new(2, 1),
            ShadowEvidenceRevision(1),
            window(),
            &coarse_start(1),
            &expected_observations(),
        );
        assert_eq!(
            result,
            Err(ShadowExecutionContinuityError::RunIdentityMismatch)
        );
    }
}
