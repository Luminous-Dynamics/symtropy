from pathlib import Path

CONTINUITY = Path("crates/domains/symtropy-lifesim-core/src/living_world_authority/shadow_execution_continuity.rs")
OBSERVABLE = Path("crates/domains/symtropy-lifesim-core/src/living_world_authority/shadow_observable_authority.rs")
VALIDATION = Path("crates/domains/symtropy-lifesim-core/src/living_world_authority/shadow_validation.rs")


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one predecessor, found {count}")
    return text.replace(old, new, 1)


o = OBSERVABLE.read_text()
o = replace_once(
    o,
    "opaque_bytes!(ShadowObservationContentManifest, EmptyObservationContentManifest);",
    """opaque_bytes!(ShadowSourceStateContentManifest, EmptyObservationContentManifest);

/// Backward-compatible spelling retained for #406 observation APIs. The manifest
/// identifies the exact producer-owned source representation state, not the scalar
/// observation value. New execution-continuity code should prefer the canonical
/// `ShadowSourceStateContentManifest` name.
pub type ShadowObservationContentManifest = ShadowSourceStateContentManifest;""",
    "source-state manifest rename",
)
OBSERVABLE.write_text(o)

v = VALIDATION.read_text()
v = replace_once(
    v,
    "const MAX_SHADOW_SAMPLES: usize = 1_000_000;",
    "pub(super) const MAX_SHADOW_SAMPLES: usize = 1_000_000;",
    "shadow sample bound visibility",
)
VALIDATION.write_text(v)

c = CONTINUITY.read_text()
c = replace_once(
    c,
    """//! Observation cadence and execution continuity are deliberately independent.
//! A terminal-state Q2 metric may observe only T1 while this transcript still
//! carries an executor-state identity for every intervening canonical tick. Any
//! tick that *is* observed must use the exact #406/#563/#567 observation identity.
//!
//! This remains structural evidence. Executor-state and segment manifests are
//! evidence-shaped inputs, so #568 must still executable-qualify the runner/profile
//! that emits them before this evidence is eligible for closure-anchor renewal.""",
    """//! Observation cadence and execution continuity are deliberately independent.
//! A terminal-state Q2 metric may observe only T1 while this transcript still
//! carries the same producer-owned source-state identity for every intervening
//! canonical tick. Observation extraction binds to that same state identity; it
//! never changes the execution chain merely because a metric sampled the tick.
//!
//! This remains structural evidence. Source-state and segment manifests are
//! evidence-shaped inputs, so #568 must still executable-qualify the runner/profile
//! that emits them before this evidence is eligible for closure-anchor renewal.""",
    "module sampling doctrine",
)
c = replace_once(
    c,
    """use super::shadow_observable_authority::{
    ShadowObservationContentManifest, ShadowObservationSourceIdentity,
};""",
    """use super::shadow_observable_authority::{
    ShadowObservationContentManifest, ShadowObservationSourceIdentity,
    ShadowSourceStateContentManifest,
};""",
    "source-state import",
)
c = replace_once(
    c,
    """use super::shadow_validation::{
    ShadowEvidenceKey, ShadowEvidenceRevision, ShadowValidationWindow,
};""",
    """use super::shadow_validation::{
    ShadowEvidenceKey, ShadowEvidenceRevision, ShadowValidationWindow, MAX_SHADOW_SAMPLES,
};""",
    "shared shadow bound import",
)
c = replace_once(
    c,
    """/// Exact executor-owned content identity for a state that was not necessarily
/// observed by #406. The byte grammar remains runner-profile authority and must
/// be executable-qualified by #568 before anchor-grade use.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShadowExecutionStateContentManifest(Vec<u8>);

impl ShadowExecutionStateContentManifest {
    pub fn new(bytes: impl Into<Vec<u8>>) -> Result<Self, ShadowExecutionContinuityError> {
        let bytes = bytes.into();
        if bytes.is_empty() {
            return Err(ShadowExecutionContinuityError::EmptyExecutionStateManifest);
        }
        Ok(Self(bytes))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}
""",
    """/// Semantic spelling for the producer-owned source-state manifest used by runner
/// continuity code. #406 historically names the same exact state identity through
/// `ShadowObservationContentManifest`; the aliases are intentionally one Rust type
/// so observation cadence cannot change execution-state identity.
pub type ShadowExecutionStateContentManifest = ShadowSourceStateContentManifest;
""",
    "execution manifest unification",
)
c = replace_once(
    c,
    """/// Content identity class for one continuity state.
///
/// The variants remain distinct even when their underlying byte encodings happen
/// to match. In particular, an executor-state manifest cannot impersonate an
/// observed #406 state merely by reusing the same bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShadowContinuityContentIdentity {
    CoarsePopulation(PopulationStateManifest),
    RetainedExact(RetainedContentManifest),
    ExecutionState(ShadowExecutionStateContentManifest),
    Observation(ShadowObservationContentManifest),
}
""",
    """/// Content identity class for one continuity state.
///
/// Post-start executor state and #406 observation-source state deliberately share
/// `SourceState`: extracting an observable must not rewrite execution history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShadowContinuityContentIdentity {
    CoarsePopulation(PopulationStateManifest),
    RetainedExact(RetainedContentManifest),
    SourceState(ShadowSourceStateContentManifest),
}
""",
    "sampling-invariant content grammar",
)
c = replace_once(
    c,
    "content: ShadowContinuityContentIdentity::ExecutionState(content_manifest),",
    "content: ShadowContinuityContentIdentity::SourceState(content_manifest),",
    "execution source normalization",
)
c = replace_once(
    c,
    """content: ShadowContinuityContentIdentity::Observation(
                source.content_manifest().clone(),
            ),""",
    """content: ShadowContinuityContentIdentity::SourceState(
                source.content_manifest().clone(),
            ),""",
    "observation source normalization",
)
c = replace_once(
    c,
    """        let expected_len = usize::try_from(window.duration_ticks()).map_err(|_| {
            ShadowExecutionContinuityError::WindowTooLargeForTranscript {
                duration_ticks: window.duration_ticks(),
            }
        })?;
        let mut canonical = BTreeMap::new();
        let mut segment_ids = BTreeSet::new();

        for segment in segments {
            if !segment_ids.insert(segment.id()) {""",
    """        let duration_ticks = window.duration_ticks();
        if duration_ticks > MAX_SHADOW_SAMPLES as u64 {
            return Err(
                ShadowExecutionContinuityError::ValidationWindowExceedsContinuityLimit {
                    maximum: MAX_SHADOW_SAMPLES,
                    actual: duration_ticks,
                },
            );
        }
        let expected_len = usize::try_from(duration_ticks).map_err(|_| {
            ShadowExecutionContinuityError::WindowTooLargeForTranscript { duration_ticks }
        })?;
        let mut canonical = BTreeMap::new();
        let mut segment_ids = BTreeSet::new();

        for segment in segments {
            if canonical.len() >= expected_len {
                return Err(ShadowExecutionContinuityError::TranscriptHasExtraSegments {
                    expected: expected_len,
                });
            }
            if !segment_ids.insert(segment.id()) {""",
    "bounded transcript ingress",
)
c = replace_once(c, "    EmptySegmentManifest,\n    EmptyExecutionStateManifest,\n    TickOverflow {", "    EmptySegmentManifest,\n    TickOverflow {", "remove duplicate manifest error")
c = replace_once(
    c,
    """    WindowTooLargeForTranscript {
        duration_ticks: u64,
    },
    DuplicateSegmentId {""",
    """    WindowTooLargeForTranscript {
        duration_ticks: u64,
    },
    ValidationWindowExceedsContinuityLimit {
        maximum: usize,
        actual: u64,
    },
    TranscriptHasExtraSegments {
        expected: usize,
    },
    DuplicateSegmentId {""",
    "bounded ingress errors",
)
c = replace_once(
    c,
    """            Self::EmptySegmentManifest => write!(f, "shadow execution segment manifest is empty"),
            Self::EmptyExecutionStateManifest => {
                write!(f, "shadow execution state content manifest is empty")
            }
            Self::TickOverflow { tick } => {""",
    """            Self::EmptySegmentManifest => write!(f, "shadow execution segment manifest is empty"),
            Self::TickOverflow { tick } => {""",
    "remove duplicate manifest display",
)
c = replace_once(
    c,
    """            Self::WindowTooLargeForTranscript { duration_ticks } => write!(
                f,
                "shadow continuity window of {duration_ticks} ticks cannot be represented on this platform"
            ),
            Self::DuplicateSegmentId { id } => {""",
    """            Self::WindowTooLargeForTranscript { duration_ticks } => write!(
                f,
                "shadow continuity window of {duration_ticks} ticks cannot be represented on this platform"
            ),
            Self::ValidationWindowExceedsContinuityLimit { maximum, actual } => write!(
                f,
                "shadow continuity window has {actual} ticks; maximum is {maximum}"
            ),
            Self::TranscriptHasExtraSegments { expected } => write!(
                f,
                "shadow continuity transcript contains more than the expected {expected} segments"
            ),
            Self::DuplicateSegmentId { id } => {""",
    "bounded ingress display",
)
c = replace_once(
    c,
    "            ShadowExecutionStateContentManifest::new(vec![content]).unwrap(),",
    "            ShadowSourceStateContentManifest::new(vec![content]).unwrap(),",
    "test execution source manifest",
)
c = replace_once(
    c,
    """            content: ShadowContinuityContentIdentity::ExecutionState(
                ShadowExecutionStateContentManifest::new(vec![content]).unwrap(),
            ),""",
    """            content: ShadowContinuityContentIdentity::SourceState(
                ShadowSourceStateContentManifest::new(vec![content]).unwrap(),
            ),""",
    "test start source state",
)
c = replace_once(
    c,
    """    #[test]
    fn terminal_observation_keeps_unobserved_ticks_as_execution_states() {
        let transcript = transcript_with_states(
            execution_state(11, 2),
            execution_state(12, 3),
            observed_state(13, 4),
        );
        let terminal_only = BTreeMap::from([(CanonicalTick(13), observed_state(13, 4))]);
        assert_eq!(validate(&transcript, &start(1), &terminal_only), Ok(()));
        assert!(matches!(
            transcript.segments().next().unwrap().successor().content(),
            ShadowContinuityContentIdentity::ExecutionState(_)
        ));
    }

    #[test]
    fn execution_state_cannot_impersonate_observation_by_reusing_bytes() {
        assert_ne!(execution_state(11, 2), observed_state(11, 2));
    }
""",
    """    #[test]
    fn observation_cadence_does_not_change_execution_transcript() {
        let every_tick = fully_observed_transcript();
        let terminal_only_transcript = transcript_with_states(
            execution_state(11, 2),
            execution_state(12, 3),
            observed_state(13, 4),
        );
        let terminal_only = BTreeMap::from([(CanonicalTick(13), observed_state(13, 4))]);

        assert_eq!(terminal_only_transcript, every_tick);
        assert_eq!(
            validate(&terminal_only_transcript, &start(1), &terminal_only),
            Ok(())
        );
    }

    #[test]
    fn observation_source_normalizes_to_same_execution_state_identity() {
        assert_eq!(execution_state(11, 2), observed_state(11, 2));
    }

    #[test]
    fn oversized_continuity_window_rejects_before_collection() {
        let oversized = ShadowValidationWindow::new(
            CanonicalTick(0),
            CanonicalTick(MAX_SHADOW_SAMPLES as u64 + 1),
        )
        .unwrap();
        let result = ShadowLaneContinuityTranscript::new(
            ShadowContinuityRunIdentity::Coarse {
                id: ShadowCoarseRunId(1),
                revision: ShadowCoarseRunRevision(1),
            },
            ShadowEvidenceKey::new(2, 1),
            ShadowEvidenceRevision(1),
            oversized,
            std::iter::empty(),
        );
        assert_eq!(
            result,
            Err(
                ShadowExecutionContinuityError::ValidationWindowExceedsContinuityLimit {
                    maximum: MAX_SHADOW_SAMPLES,
                    actual: MAX_SHADOW_SAMPLES as u64 + 1,
                }
            )
        );
    }
""",
    "sampling-invariance regressions",
)
CONTINUITY.write_text(c)

print("LW55_REPAIR_TRANSFORM_APPLIED=1")
