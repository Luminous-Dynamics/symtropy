//! Simple, inspectable passive acquisition baseline for ErgoStream v0.1.
//!
//! The baseline uses only public frame sequence, channel, symbol and tick data.
//! It segments at a largest-gap threshold, clusters exact within-segment symbol
//! patterns, and composes adjacent cluster IDs into motifs. It is intentionally
//! weak and interpretable; it is a calibration baseline, not the proposed
//! Symthaea learner or a general-purpose event detector.

use std::collections::BTreeMap;
use std::fmt;

use super::{CandidateSpanV0, PolicyViewV0, SCHEMA_VERSION_V0, SpanV0, StructuralSnapshotV0};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcquisitionErrorV0 {
    SchemaVersionMismatch,
    PrefixLengthMismatch,
    InvalidFrameSequence,
    NonMonotonicTime,
    TooManyClusters,
}

impl fmt::Display for AcquisitionErrorV0 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::SchemaVersionMismatch => "unsupported policy-view schema version",
            Self::PrefixLengthMismatch => "available prefix length does not match frame count",
            Self::InvalidFrameSequence => "frame sequence must be contiguous from zero",
            Self::NonMonotonicTime => "frame ticks must be strictly increasing",
            Self::TooManyClusters => "cluster count exceeds the u32 identifier space",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for AcquisitionErrorV0 {}

/// Passive segmentation and exact-pattern clustering baseline.
///
/// This baseline is deliberately batch-oriented: it uses only the supplied
/// causal prefix, and the end_of_stream argument tells it whether the final
/// segment is closed. Without a terminal marker, an unfinished tail is not
/// promoted to a completed unit. If the observed timing has no separable gap
/// scale, it abstains by returning an empty snapshot rather than inventing
/// boundaries.
#[derive(Debug, Clone, Copy, Default)]
pub struct PassiveGapPatternBaselineV0;

impl PassiveGapPatternBaselineV0 {
    pub const NAME: &'static str = "passive-gap-pattern-v0";

    pub fn acquire(
        &self,
        view: &PolicyViewV0,
        end_of_stream: bool,
    ) -> Result<StructuralSnapshotV0, AcquisitionErrorV0> {
        validate_view(view)?;

        let Some(threshold) = gap_threshold(view) else {
            return Ok(StructuralSnapshotV0::default());
        };

        let mut segments = Vec::new();
        let mut segment_start = 0;

        for index in 1..view.frames.len() {
            let gap = view.frames[index].tick - view.frames[index - 1].tick;
            if gap > threshold {
                segments.push(SpanV0 {
                    start_frame: segment_start,
                    end_frame: index,
                });
                segment_start = index;
            }
        }

        if end_of_stream && segment_start < view.frames.len() {
            segments.push(SpanV0 {
                start_frame: segment_start,
                end_frame: view.frames.len(),
            });
        }

        let mut unit_patterns = BTreeMap::<Vec<(u8, u8)>, u32>::new();
        let mut events = Vec::with_capacity(segments.len());
        let mut units = Vec::with_capacity(segments.len());

        for span in segments {
            let signature = view.frames[span.start_frame..span.end_frame]
                .iter()
                .map(|frame| (frame.channel_id, frame.symbol))
                .collect::<Vec<_>>();

            let cluster_id = match unit_patterns.get(&signature) {
                Some(existing) => *existing,
                None => {
                    let next_id = u32::try_from(unit_patterns.len())
                        .map_err(|_| AcquisitionErrorV0::TooManyClusters)?;
                    unit_patterns.insert(signature, next_id);
                    next_id
                }
            };

            units.push(CandidateSpanV0 { span, cluster_id });
            events.push((span, cluster_id));
        }

        let mut motif_patterns = BTreeMap::<(u32, u32), u32>::new();
        let mut motifs = Vec::with_capacity(events.len().saturating_sub(1));

        for pair in events.windows(2) {
            let signature = (pair[0].1, pair[1].1);
            let cluster_id = match motif_patterns.get(&signature) {
                Some(existing) => *existing,
                None => {
                    let next_id = u32::try_from(motif_patterns.len())
                        .map_err(|_| AcquisitionErrorV0::TooManyClusters)?;
                    motif_patterns.insert(signature, next_id);
                    next_id
                }
            };

            motifs.push(CandidateSpanV0 {
                span: SpanV0 {
                    start_frame: pair[0].0.start_frame,
                    end_frame: pair[1].0.end_frame,
                },
                cluster_id,
            });
        }

        Ok(StructuralSnapshotV0 { units, motifs })
    }
}

fn validate_view(view: &PolicyViewV0) -> Result<(), AcquisitionErrorV0> {
    if view.schema_version != SCHEMA_VERSION_V0 {
        return Err(AcquisitionErrorV0::SchemaVersionMismatch);
    }

    if view.available_prefix_end != view.frames.len() {
        return Err(AcquisitionErrorV0::PrefixLengthMismatch);
    }

    for (index, frame) in view.frames.iter().enumerate() {
        let expected =
            u32::try_from(index).map_err(|_| AcquisitionErrorV0::InvalidFrameSequence)?;
        if frame.sequence != expected {
            return Err(AcquisitionErrorV0::InvalidFrameSequence);
        }
    }

    if view
        .frames
        .windows(2)
        .any(|pair| pair[1].tick <= pair[0].tick)
    {
        return Err(AcquisitionErrorV0::NonMonotonicTime);
    }

    Ok(())
}

/// A simple midpoint threshold between the smallest and largest observed
/// positive inter-frame gaps. This only models a two-scale timing profile; it
/// is intentionally not robust to arbitrary clocks or missing-frame regimes.
fn gap_threshold(view: &PolicyViewV0) -> Option<u64> {
    let mut gaps = view
        .frames
        .windows(2)
        .map(|pair| pair[1].tick - pair[0].tick);

    let first = gaps.next()?;
    let (minimum, maximum) = gaps.fold((first, first), |(minimum, maximum), gap| {
        (minimum.min(gap), maximum.max(gap))
    });

    (minimum < maximum).then_some(minimum + (maximum - minimum) / 2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WorldV0;

    fn full_view(world: &WorldV0) -> PolicyViewV0 {
        world.policy_view(128).expect("64 two-frame units")
    }

    #[test]
    fn baseline_recovers_reference_unit_and_motif_equivalence() {
        let world = WorldV0::generate(0x5EED, 64, [0, 1, 2, 3], 3, 2).unwrap();
        let view = full_view(&world);
        let snapshot = PassiveGapPatternBaselineV0.acquire(&view, true).unwrap();
        let report = world.evaluate(&snapshot);

        assert_eq!(report.unit_boundaries.f1, Some(1.0));
        assert_eq!(report.unit_equivalence.f1, Some(1.0));
        assert_eq!(report.motif_boundaries.f1, Some(1.0));
        assert_eq!(report.motif_equivalence.f1, Some(1.0));
    }

    #[test]
    fn baseline_uses_only_observed_symbols_under_alphabet_permutation() {
        let world = WorldV0::generate(0x5EED, 64, [2, 0, 3, 1], 3, 2).unwrap();
        let view = full_view(&world);
        let snapshot = PassiveGapPatternBaselineV0.acquire(&view, true).unwrap();
        let report = world.evaluate(&snapshot);

        assert_eq!(report.unit_boundaries.f1, Some(1.0));
        assert_eq!(report.unit_equivalence.f1, Some(1.0));
        assert_eq!(report.motif_equivalence.f1, Some(1.0));
    }

    #[test]
    fn baseline_recovers_structure_under_uniform_time_dilation() {
        let world = WorldV0::generate(0x5EED, 64, [0, 1, 2, 3], 30, 20).unwrap();
        let view = full_view(&world);
        let snapshot = PassiveGapPatternBaselineV0.acquire(&view, true).unwrap();
        let report = world.evaluate(&snapshot);

        assert_eq!(report.unit_boundaries.f1, Some(1.0));
        assert_eq!(report.unit_equivalence.f1, Some(1.0));
        assert_eq!(report.motif_boundaries.f1, Some(1.0));
        assert_eq!(report.motif_equivalence.f1, Some(1.0));
    }

    #[test]
    fn baseline_does_not_promote_an_open_tail_to_a_completed_unit() {
        let world = WorldV0::generate(0x5EED, 64, [0, 1, 2, 3], 3, 2).unwrap();
        let view = world.policy_view(7).unwrap();
        let snapshot = PassiveGapPatternBaselineV0.acquire(&view, false).unwrap();

        assert_eq!(snapshot.units.len(), 3);
        assert_eq!(snapshot.units.last().unwrap().span.end_frame, 6);
    }

    #[test]
    fn baseline_abstains_when_timing_has_no_separable_gap_scale() {
        let world = WorldV0::generate(0x5EED, 64, [0, 1, 2, 3], 3, 0).unwrap();
        let view = full_view(&world);
        let snapshot = PassiveGapPatternBaselineV0.acquire(&view, true).unwrap();

        assert!(snapshot.units.is_empty());
        assert!(snapshot.motifs.is_empty());
    }

    #[test]
    fn baseline_rejects_non_monotonic_time() {
        let world = WorldV0::generate(0x5EED, 64, [0, 1, 2, 3], 3, 2).unwrap();
        let mut view = full_view(&world);
        view.frames[1].tick = view.frames[0].tick;

        assert_eq!(
            PassiveGapPatternBaselineV0.acquire(&view, true),
            Err(AcquisitionErrorV0::NonMonotonicTime)
        );
    }

    #[test]
    fn baseline_rejects_non_contiguous_sequence() {
        let world = WorldV0::generate(0x5EED, 64, [0, 1, 2, 3], 3, 2).unwrap();
        let mut view = full_view(&world);
        view.frames[4].sequence = 99;

        assert_eq!(
            PassiveGapPatternBaselineV0.acquire(&view, true),
            Err(AcquisitionErrorV0::InvalidFrameSequence)
        );
    }
}
