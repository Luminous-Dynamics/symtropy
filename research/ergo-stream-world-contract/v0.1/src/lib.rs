//! Tiny deterministic reference world for ErgoStream v0.1.
//!
//! This crate separates the public stream view from evaluator-owned truth.
//! It is a local reference model, not a process/OS security boundary, sensor
//! driver, physical simulator, or complete benchmark runner.

use std::fmt;

pub mod acquisition;
pub mod atlas;
pub mod behaviour;
pub mod bundle;
pub mod calibration;
pub mod decision_value;
pub mod dynamics;
pub mod matching;
pub mod quotient;
pub mod scoped;

pub mod transfer;

pub const SCHEMA_VERSION_V0: u16 = 1;
const MAX_EVENTS_V0: usize = 65_536;
const UNIT_PATTERNS: [[u8; 2]; 3] = [[0, 1], [2, 3], [0, 2]];
const MOTIF_PREFIX: [u8; 18] = [0, 1, 2, 0, 0, 1, 1, 2, 2, 0, 1, 2, 1, 0, 2, 2, 1, 0];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SpanV0 {
    pub start_frame: usize,
    /// Exclusive end frame.
    pub end_frame: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamFrameV0 {
    pub sequence: u32,
    pub tick: u64,
    pub channel_id: u8,
    pub symbol: u8,
}

/// This is the only stream data exposed to a learner in this first reference.
/// It intentionally contains no unit IDs, motif IDs, generator seed or labels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyViewV0 {
    pub schema_version: u16,
    pub frames: Vec<StreamFrameV0>,
    /// Exclusive end of the currently available prefix.
    pub available_prefix_end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CandidateSpanV0 {
    pub span: SpanV0,
    /// Learner-local cluster ID; it has no evaluator meaning.
    pub cluster_id: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StructuralSnapshotV0 {
    pub units: Vec<CandidateSpanV0>,
    pub motifs: Vec<CandidateSpanV0>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CountMetricsV0 {
    pub true_positive: usize,
    pub false_positive: usize,
    pub false_negative: usize,
    pub precision: Option<f64>,
    pub recall: Option<f64>,
    pub f1: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EvaluationReportV0 {
    pub unit_boundaries: CountMetricsV0,
    pub unit_equivalence: CountMetricsV0,
    pub motif_boundaries: CountMetricsV0,
    pub motif_equivalence: CountMetricsV0,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TruthSpan {
    span: SpanV0,
    class_id: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldError {
    TooFewEvents,
    TooManyEvents,
    InvalidTimeScale,
    InvalidSymbolPermutation,
    TimeOverflow,
    SequenceOverflow,
    PrefixOutOfRange,
}

impl fmt::Display for WorldError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooFewEvents => write!(formatter, "event count must be at least two"),
            Self::TooManyEvents => write!(formatter, "event count exceeds the v0 bound"),
            Self::InvalidTimeScale => write!(formatter, "time scale must be nonzero"),
            Self::InvalidSymbolPermutation => {
                write!(formatter, "symbol mapping must be a permutation of 0..4")
            }
            Self::TimeOverflow => write!(formatter, "stream tick overflow"),
            Self::SequenceOverflow => write!(formatter, "stream sequence overflow"),
            Self::PrefixOutOfRange => write!(formatter, "requested prefix exceeds stream length"),
        }
    }
}

impl std::error::Error for WorldError {}

#[derive(Debug, Clone)]
pub struct WorldV0 {
    frames: Vec<StreamFrameV0>,
    truth_units: Vec<TruthSpan>,
    truth_motifs: Vec<TruthSpan>,
}

impl WorldV0 {
    /// Generate a deterministic finite stream.
    ///
    /// The symbol permutation changes the observed alphabet while preserving
    /// latent unit/motif structure. The seed selects the suffix after a fixed
    /// coverage prefix, ensuring all three unit classes are represented.
    pub fn generate(
        seed: u64,
        event_count: usize,
        symbol_permutation: [u8; 4],
        time_scale: u64,
        inter_event_gap: u64,
    ) -> Result<Self, WorldError> {
        if event_count < 2 {
            return Err(WorldError::TooFewEvents);
        }
        if event_count > MAX_EVENTS_V0 {
            return Err(WorldError::TooManyEvents);
        }
        if time_scale == 0 {
            return Err(WorldError::InvalidTimeScale);
        }
        if !is_permutation(symbol_permutation) {
            return Err(WorldError::InvalidSymbolPermutation);
        }

        let mut random_state = seed;
        let mut frames = Vec::with_capacity(event_count.saturating_mul(2));
        let mut truth_units = Vec::with_capacity(event_count);
        let mut classes = Vec::with_capacity(event_count);
        let mut tick = 0_u64;

        let class_stream = MOTIF_PREFIX.into_iter().chain(std::iter::repeat_with(|| {
            (splitmix64_next(&mut random_state) % UNIT_PATTERNS.len() as u64) as u8
        }));
        for (event_index, unit_class) in class_stream.take(event_count).enumerate() {
            let pattern = UNIT_PATTERNS[unit_class as usize];
            let start_frame = frames.len();

            for base_symbol in pattern {
                let sequence =
                    u32::try_from(frames.len()).map_err(|_| WorldError::SequenceOverflow)?;
                frames.push(StreamFrameV0 {
                    sequence,
                    tick,
                    channel_id: 0,
                    symbol: symbol_permutation[base_symbol as usize],
                });
                tick = tick
                    .checked_add(time_scale)
                    .ok_or(WorldError::TimeOverflow)?;
            }

            let end_frame = frames.len();
            truth_units.push(TruthSpan {
                span: SpanV0 {
                    start_frame,
                    end_frame,
                },
                class_id: u16::from(unit_class),
            });
            classes.push(unit_class);
            if event_index + 1 < event_count {
                tick = tick
                    .checked_add(inter_event_gap)
                    .ok_or(WorldError::TimeOverflow)?;
            }
        }

        let mut truth_motifs = Vec::with_capacity(event_count - 1);
        for (class_pair, event_pair) in classes.windows(2).zip(truth_units.windows(2)) {
            let class_id = u16::from(class_pair[0]) * 3 + u16::from(class_pair[1]);
            truth_motifs.push(TruthSpan {
                span: SpanV0 {
                    start_frame: event_pair[0].span.start_frame,
                    end_frame: event_pair[1].span.end_frame,
                },
                class_id,
            });
        }

        Ok(Self {
            frames,
            truth_units,
            truth_motifs,
        })
    }

    /// Return only the requested causally available prefix.
    pub fn policy_view(&self, end_exclusive: usize) -> Result<PolicyViewV0, WorldError> {
        if end_exclusive > self.frames.len() {
            return Err(WorldError::PrefixOutOfRange);
        }
        Ok(PolicyViewV0 {
            schema_version: SCHEMA_VERSION_V0,
            frames: self.frames[..end_exclusive].to_vec(),
            available_prefix_end: end_exclusive,
        })
    }

    /// Evaluate a frozen learner snapshot against evaluator-owned labels.
    ///
    /// This method is owner/evaluator-side API. It must not be exposed to an
    /// untrusted learner process.
    pub fn evaluate(&self, snapshot: &StructuralSnapshotV0) -> EvaluationReportV0 {
        let (unit_boundaries, matched_units) =
            boundary_metrics(&snapshot.units, &self.truth_units, self.frames.len());
        let unit_equivalence = equivalence_metrics(&matched_units, &self.truth_units);

        let (motif_boundaries, matched_motifs) =
            boundary_metrics(&snapshot.motifs, &self.truth_motifs, self.frames.len());
        let motif_equivalence = equivalence_metrics(&matched_motifs, &self.truth_motifs);

        EvaluationReportV0 {
            unit_boundaries,
            unit_equivalence,
            motif_boundaries,
            motif_equivalence,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct MatchedCandidate {
    truth_index: usize,
    predicted_cluster: u32,
}

fn is_permutation(values: [u8; 4]) -> bool {
    let mut seen = [false; 4];
    for value in values {
        let index = usize::from(value);
        if index >= seen.len() || seen[index] {
            return false;
        }
        seen[index] = true;
    }
    seen.into_iter().all(|value| value)
}

fn splitmix64_next(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut value = *state;
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

fn count_metrics(
    true_positive: usize,
    false_positive: usize,
    false_negative: usize,
) -> CountMetricsV0 {
    let predicted = true_positive + false_positive;
    let actual = true_positive + false_negative;
    let precision = if predicted > 0 {
        Some(true_positive as f64 / predicted as f64)
    } else if actual > 0 {
        Some(0.0)
    } else {
        None
    };
    let recall = if actual > 0 {
        Some(true_positive as f64 / actual as f64)
    } else if predicted > 0 {
        Some(0.0)
    } else {
        None
    };
    let f1 = match (precision, recall) {
        (Some(p), Some(r)) if p + r > 0.0 => Some(2.0 * p * r / (p + r)),
        (Some(_), Some(_)) => Some(0.0),
        _ => None,
    };
    CountMetricsV0 {
        true_positive,
        false_positive,
        false_negative,
        precision,
        recall,
        f1,
    }
}

fn boundary_metrics(
    candidates: &[CandidateSpanV0],
    truth: &[TruthSpan],
    frame_count: usize,
) -> (CountMetricsV0, Vec<MatchedCandidate>) {
    let mut used_truth = vec![false; truth.len()];
    let mut matched = Vec::new();
    let mut true_positive = 0;
    let mut false_positive = 0;

    for candidate in candidates {
        if candidate.span.start_frame >= candidate.span.end_frame
            || candidate.span.end_frame > frame_count
        {
            false_positive += 1;
            continue;
        }

        let truth_index = truth.iter().enumerate().find_map(|(index, expected)| {
            (!used_truth[index] && expected.span == candidate.span).then_some(index)
        });

        match truth_index {
            Some(index) => {
                used_truth[index] = true;
                true_positive += 1;
                matched.push(MatchedCandidate {
                    truth_index: index,
                    predicted_cluster: candidate.cluster_id,
                });
            }
            None => false_positive += 1,
        }
    }

    let false_negative = truth.len() - true_positive;
    (
        count_metrics(true_positive, false_positive, false_negative),
        matched,
    )
}

fn equivalence_metrics(matched: &[MatchedCandidate], truth: &[TruthSpan]) -> CountMetricsV0 {
    let mut true_positive = 0;
    let mut false_positive = 0;
    let mut false_negative = 0;

    for (left_index, left) in matched.iter().enumerate() {
        for right in matched.iter().skip(left_index + 1) {
            let predicted_same = left.predicted_cluster == right.predicted_cluster;
            let actual_same = truth[left.truth_index].class_id == truth[right.truth_index].class_id;

            match (predicted_same, actual_same) {
                (true, true) => true_positive += 1,
                (true, false) => false_positive += 1,
                (false, true) => false_negative += 1,
                (false, false) => {}
            }
        }
    }

    count_metrics(true_positive, false_positive, false_negative)
}

#[cfg(test)]
fn exact_snapshot(world: &WorldV0) -> StructuralSnapshotV0 {
    StructuralSnapshotV0 {
        units: world
            .truth_units
            .iter()
            .map(|truth| CandidateSpanV0 {
                span: truth.span,
                cluster_id: u32::from(truth.class_id),
            })
            .collect(),
        motifs: world
            .truth_motifs
            .iter()
            .map(|truth| CandidateSpanV0 {
                span: truth.span,
                cluster_id: u32::from(truth.class_id),
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world() -> WorldV0 {
        WorldV0::generate(0x5EED, 64, [0, 1, 2, 3], 3, 2).expect("known-valid fixture parameters")
    }

    #[test]
    fn same_seed_and_profile_replay_identically() {
        let left = world();
        let right = world();
        assert_eq!(left.frames, right.frames);
        assert_eq!(left.truth_units, right.truth_units);
        assert_eq!(left.truth_motifs, right.truth_motifs);
    }

    #[test]
    fn different_seed_changes_generated_suffix() {
        let left = world();
        let right = WorldV0::generate(0xBEEF, 64, [0, 1, 2, 3], 3, 2).unwrap();
        assert_ne!(left.frames, right.frames);
    }

    #[test]
    fn symbol_permutation_preserves_hidden_structure() {
        let original = world();
        let transformed = WorldV0::generate(0x5EED, 64, [2, 0, 3, 1], 3, 2).unwrap();
        assert_eq!(original.truth_units, transformed.truth_units);
        assert_eq!(original.truth_motifs, transformed.truth_motifs);
        assert_ne!(original.frames, transformed.frames);
    }

    #[test]
    fn rejects_non_permutation_and_invalid_time_scale() {
        assert_eq!(
            WorldV0::generate(1, 8, [0, 1, 1, 3], 1, 0).unwrap_err(),
            WorldError::InvalidSymbolPermutation
        );
        assert_eq!(
            WorldV0::generate(1, 8, [0, 1, 2, 3], 0, 0).unwrap_err(),
            WorldError::InvalidTimeScale
        );
    }

    #[test]
    fn public_view_contains_only_available_prefix() {
        let generated = world();
        let view = generated.policy_view(7).unwrap();
        assert_eq!(view.frames.len(), 7);
        assert_eq!(view.available_prefix_end, 7);
        assert_eq!(view.frames.last().unwrap().sequence, 6);
        assert!(!format!("{view:?}").contains("truth_units"));
        assert!(!format!("{view:?}").contains("class_id"));
    }

    #[test]
    fn exact_oracle_recovers_unit_and_motif_structure() {
        let generated = world();
        let report = generated.evaluate(&exact_snapshot(&generated));
        assert_eq!(
            report.unit_boundaries.true_positive,
            generated.truth_units.len()
        );
        assert_eq!(report.unit_boundaries.false_positive, 0);
        assert_eq!(report.unit_boundaries.false_negative, 0);
        assert_eq!(report.unit_equivalence.f1, Some(1.0));
        assert_eq!(
            report.motif_boundaries.true_positive,
            generated.truth_motifs.len()
        );
        assert_eq!(report.motif_boundaries.false_positive, 0);
        assert_eq!(report.motif_boundaries.false_negative, 0);
        assert_eq!(report.motif_equivalence.f1, Some(1.0));
    }

    #[test]
    fn merging_all_units_preserves_boundaries_but_fails_equivalence() {
        let generated = world();
        let mut snapshot = exact_snapshot(&generated);
        for candidate in &mut snapshot.units {
            candidate.cluster_id = 0;
        }
        let report = generated.evaluate(&snapshot);
        assert_eq!(report.unit_boundaries.f1, Some(1.0));
        assert!(report.unit_equivalence.false_positive > 0);
        assert!(report.unit_equivalence.f1.unwrap() < 1.0);
    }

    #[test]
    fn splitting_every_unit_into_its_own_cluster_fails_reuse_recovery() {
        let generated = world();
        let mut snapshot = exact_snapshot(&generated);
        for (index, candidate) in snapshot.units.iter_mut().enumerate() {
            candidate.cluster_id = u32::try_from(index).expect("fixture index fits u32");
        }
        let report = generated.evaluate(&snapshot);
        assert_eq!(report.unit_boundaries.f1, Some(1.0));
        assert!(report.unit_equivalence.false_negative > 0);
        assert!(report.unit_equivalence.f1.unwrap() < 1.0);
    }

    #[test]
    fn a_merged_span_cannot_score_as_exact_unit_boundaries() {
        let generated = world();
        let snapshot = StructuralSnapshotV0 {
            units: vec![CandidateSpanV0 {
                span: SpanV0 {
                    start_frame: 0,
                    end_frame: generated.frames.len(),
                },
                cluster_id: 0,
            }],
            motifs: Vec::new(),
        };
        let report = generated.evaluate(&snapshot);
        assert_eq!(report.unit_boundaries.true_positive, 0);
        assert_eq!(report.unit_boundaries.false_positive, 1);
        assert_eq!(
            report.unit_boundaries.false_negative,
            generated.truth_units.len()
        );
    }

    #[test]
    fn duplicate_candidates_are_not_double_counted_as_true_positives() {
        let generated = world();
        let mut snapshot = exact_snapshot(&generated);
        snapshot.units.push(snapshot.units[0]);
        let report = generated.evaluate(&snapshot);
        assert_eq!(
            report.unit_boundaries.true_positive,
            generated.truth_units.len()
        );
        assert_eq!(report.unit_boundaries.false_positive, 1);
    }

    #[test]
    fn rejects_event_count_over_bound_before_allocating() {
        assert_eq!(
            WorldV0::generate(1, 65_537, [0, 1, 2, 3], 1, 0).unwrap_err(),
            WorldError::TooManyEvents
        );
    }

    #[test]
    fn invalid_prefix_is_rejected_without_exposing_future_frames() {
        let generated = world();
        assert_eq!(
            generated
                .policy_view(generated.frames.len() + 1)
                .unwrap_err(),
            WorldError::PrefixOutOfRange
        );
    }
}
