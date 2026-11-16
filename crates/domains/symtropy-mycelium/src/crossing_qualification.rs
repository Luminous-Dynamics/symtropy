// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Conservative overlap qualification for 3D filament reconstructions.
//!
//! An XY projection overlap is not a graph junction. This module uses only
//! depth evidence to rule out a connection when depth intervals are disjoint.
//! Overlapping or missing depth evidence remains unresolved and never creates
//! a branch, fusion or anastomosis event.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CrossingCandidateId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FilamentTrackId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DepthIntervalUm {
    pub lower_um: f64,
    pub upper_um: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrossingQualificationError {
    InvalidDepthInterval,
    InvalidProjectedDistance,
    SameFilamentTrack(FilamentTrackId),
}

impl fmt::Display for CrossingQualificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDepthInterval => {
                write!(f, "depth interval bounds must be finite and ordered")
            }
            Self::InvalidProjectedDistance => {
                write!(
                    f,
                    "projected crossing distance must be finite and non-negative"
                )
            }
            Self::SameFilamentTrack(id) => {
                write!(
                    f,
                    "crossing candidate must reference distinct filament tracks: {:?}",
                    id
                )
            }
        }
    }
}

impl std::error::Error for CrossingQualificationError {}

impl DepthIntervalUm {
    pub fn new(lower_um: f64, upper_um: f64) -> Result<Self, CrossingQualificationError> {
        if !lower_um.is_finite() || !upper_um.is_finite() || lower_um > upper_um {
            return Err(CrossingQualificationError::InvalidDepthInterval);
        }
        Ok(Self { lower_um, upper_um })
    }

    /// Intervals are closed: touching uncertainty bounds are ambiguous.
    pub fn overlaps(self, other: Self) -> bool {
        self.lower_um <= other.upper_um && other.lower_um <= self.upper_um
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CrossingCandidate {
    pub id: CrossingCandidateId,
    pub first_filament: FilamentTrackId,
    pub second_filament: FilamentTrackId,
    /// XY separation of the projected centerlines, in micrometres.
    pub projected_distance_um: f64,
    /// Depth uncertainty intervals from focal evidence. Missing is unknown.
    pub first_depth: Option<DepthIntervalUm>,
    pub second_depth: Option<DepthIntervalUm>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossingAssessment {
    /// The observed depth intervals do not overlap; do not connect these tracks.
    DepthSeparated,
    /// Depth intervals overlap; projection alone cannot decide topology.
    AmbiguousDepthRequiresReview,
    /// One or both depth intervals are missing; no topology decision is allowed.
    MissingDepthRequiresReview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrossingTopologyHypotheses {
    /// The candidate represents two distinct 3D filaments.
    pub separate_filaments: bool,
    /// The candidate could be a shared 3D junction; this is not fusion approval.
    pub shared_3d_junction: bool,
}

impl CrossingTopologyHypotheses {
    /// True only when the evidence leaves topology unresolved.
    pub const fn requires_review(self) -> bool {
        self.separate_filaments && self.shared_3d_junction
    }

    /// The classifier never authorizes a topology mutation.
    pub const fn permits_automatic_join(self) -> bool {
        false
    }
}

impl CrossingCandidate {
    pub fn validate(&self) -> Result<(), CrossingQualificationError> {
        if !self.projected_distance_um.is_finite() || self.projected_distance_um < 0.0 {
            return Err(CrossingQualificationError::InvalidProjectedDistance);
        }
        if self.first_filament == self.second_filament {
            return Err(CrossingQualificationError::SameFilamentTrack(
                self.first_filament,
            ));
        }
        for interval in [self.first_depth, self.second_depth].into_iter().flatten() {
            if !interval.lower_um.is_finite()
                || !interval.upper_um.is_finite()
                || interval.lower_um > interval.upper_um
            {
                return Err(CrossingQualificationError::InvalidDepthInterval);
            }
        }
        Ok(())
    }

    /// Return the set of topologies still admissible under the depth evidence.
    ///
    /// This is a hypothesis set, not a probability distribution. In ambiguous
    /// cases both topologies remain possible; the caller must not select the
    /// shared-junction hypothesis as an automatic graph mutation.
    pub fn topology_hypotheses(
        &self,
    ) -> Result<CrossingTopologyHypotheses, CrossingQualificationError> {
        match self.assess()? {
            CrossingAssessment::DepthSeparated => Ok(CrossingTopologyHypotheses {
                separate_filaments: true,
                shared_3d_junction: false,
            }),
            CrossingAssessment::AmbiguousDepthRequiresReview
            | CrossingAssessment::MissingDepthRequiresReview => Ok(CrossingTopologyHypotheses {
                separate_filaments: true,
                shared_3d_junction: true,
            }),
        }
    }

    pub fn assess(&self) -> Result<CrossingAssessment, CrossingQualificationError> {
        self.validate()?;
        let (Some(first), Some(second)) = (self.first_depth, self.second_depth) else {
            return Ok(CrossingAssessment::MissingDepthRequiresReview);
        };
        if first.overlaps(second) {
            Ok(CrossingAssessment::AmbiguousDepthRequiresReview)
        } else {
            Ok(CrossingAssessment::DepthSeparated)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(
        first_depth: Option<(f64, f64)>,
        second_depth: Option<(f64, f64)>,
    ) -> CrossingCandidate {
        CrossingCandidate {
            id: CrossingCandidateId(1),
            first_filament: FilamentTrackId(10),
            second_filament: FilamentTrackId(20),
            projected_distance_um: 0.0,
            first_depth: first_depth.map(|(lo, hi)| DepthIntervalUm {
                lower_um: lo,
                upper_um: hi,
            }),
            second_depth: second_depth.map(|(lo, hi)| DepthIntervalUm {
                lower_um: lo,
                upper_um: hi,
            }),
        }
    }

    #[test]
    fn disjoint_depth_intervals_are_not_connected() {
        let item = candidate(Some((1.0, 2.0)), Some((3.0, 4.0)));
        assert_eq!(item.assess(), Ok(CrossingAssessment::DepthSeparated));
    }

    #[test]
    fn overlapping_depth_intervals_require_review() {
        let item = candidate(Some((1.0, 3.0)), Some((2.0, 4.0)));
        assert_eq!(
            item.assess(),
            Ok(CrossingAssessment::AmbiguousDepthRequiresReview)
        );
    }

    #[test]
    fn touching_closed_depth_intervals_are_ambiguous() {
        let item = candidate(Some((1.0, 2.0)), Some((2.0, 3.0)));
        assert_eq!(
            item.assess(),
            Ok(CrossingAssessment::AmbiguousDepthRequiresReview)
        );
    }

    #[test]
    fn missing_depth_never_becomes_a_junction() {
        let item = candidate(Some((1.0, 2.0)), None);
        assert_eq!(
            item.assess(),
            Ok(CrossingAssessment::MissingDepthRequiresReview)
        );
    }

    #[test]
    fn invalid_depth_interval_is_rejected() {
        assert_eq!(
            DepthIntervalUm::new(2.0, 1.0),
            Err(CrossingQualificationError::InvalidDepthInterval)
        );
        assert_eq!(
            DepthIntervalUm::new(f64::NAN, 1.0),
            Err(CrossingQualificationError::InvalidDepthInterval)
        );
    }

    #[test]
    fn overlapping_depth_preserves_both_topology_hypotheses() {
        let item = candidate(Some((1.0, 3.0)), Some((2.0, 4.0)));
        let hypotheses = item.topology_hypotheses().unwrap();
        assert!(hypotheses.separate_filaments);
        assert!(hypotheses.shared_3d_junction);
        assert!(hypotheses.requires_review());
        assert!(!hypotheses.permits_automatic_join());
    }

    #[test]
    fn disjoint_depth_excludes_shared_junction_hypothesis() {
        let item = candidate(Some((1.0, 2.0)), Some((4.0, 5.0)));
        let hypotheses = item.topology_hypotheses().unwrap();
        assert!(hypotheses.separate_filaments);
        assert!(!hypotheses.shared_3d_junction);
        assert!(!hypotheses.requires_review());
        assert!(!hypotheses.permits_automatic_join());
    }

    #[test]
    fn missing_depth_preserves_both_topology_hypotheses() {
        let item = candidate(Some((1.0, 2.0)), None);
        let hypotheses = item.topology_hypotheses().unwrap();
        assert!(hypotheses.separate_filaments);
        assert!(hypotheses.shared_3d_junction);
        assert!(hypotheses.requires_review());
        assert!(!hypotheses.permits_automatic_join());
    }

    #[test]
    fn manually_constructed_invalid_depth_interval_is_rejected() {
        let mut item = candidate(Some((1.0, 2.0)), Some((3.0, 4.0)));
        item.first_depth = Some(DepthIntervalUm {
            lower_um: 5.0,
            upper_um: 4.0,
        });
        assert_eq!(
            item.assess(),
            Err(CrossingQualificationError::InvalidDepthInterval)
        );

        item.first_depth = Some(DepthIntervalUm {
            lower_um: f64::NAN,
            upper_um: 4.0,
        });
        assert_eq!(
            item.assess(),
            Err(CrossingQualificationError::InvalidDepthInterval)
        );
    }

    #[test]
    fn invalid_projection_and_same_track_are_rejected() {
        let mut item = candidate(Some((1.0, 2.0)), Some((3.0, 4.0)));
        item.projected_distance_um = f64::INFINITY;
        assert_eq!(
            item.assess(),
            Err(CrossingQualificationError::InvalidProjectedDistance)
        );

        item = candidate(Some((1.0, 2.0)), Some((3.0, 4.0)));
        item.second_filament = item.first_filament;
        assert_eq!(
            item.assess(),
            Err(CrossingQualificationError::SameFilamentTrack(
                FilamentTrackId(10)
            ))
        );
    }

    #[test]
    fn depth_separation_does_not_assert_biological_fusion_absence_beyond_intervals() {
        let item = candidate(Some((0.0, 1.0)), Some((5.0, 6.0)));
        assert_eq!(item.assess(), Ok(CrossingAssessment::DepthSeparated));
    }
}
