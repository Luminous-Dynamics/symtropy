// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Source-bounded kinematic references for diatom gliding and Chlamydomonas phototaxis.
//! These are analysis baselines, not molecular or hydrodynamic simulators.

use std::collections::HashSet;
use std::error::Error;
use std::fmt;

use super::{EventEvidenceStatus, OrientationOrder, orientation_order};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RapheContactMode {
    OneRapheBranch,
    TwoRapheBranches,
    NoContact,
    Unresolved,
}

impl RapheContactMode {
    fn is_calibratable(self) -> bool {
        matches!(self, Self::OneRapheBranch | Self::TwoRapheBranches)
    }
}

/// Signed curvature units are inverse micrometres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RapheCurvatureObservation<'a> {
    pub sample_id: &'a str,
    pub time_s: f64,
    pub contact_mode: RapheContactMode,
    pub active_raphe_curvature_inv_um: Option<f64>,
    pub observed_path_curvature_inv_um: Option<f64>,
    pub evidence_status: EventEvidenceStatus,
}

impl RapheCurvatureObservation<'_> {
    fn validate(&self) -> Result<(), AlgalKinematicsError> {
        if self.sample_id.trim().is_empty() {
            return Err(AlgalKinematicsError::EmptySampleId);
        }
        if !self.time_s.is_finite() || self.time_s < 0.0 {
            return Err(AlgalKinematicsError::InvalidObservationTime);
        }
        for value in [
            self.active_raphe_curvature_inv_um,
            self.observed_path_curvature_inv_um,
        ]
        .into_iter()
        .flatten()
        {
            if !value.is_finite() {
                return Err(AlgalKinematicsError::NonFiniteMeasurement);
            }
        }
        if self.contact_mode.is_calibratable()
            && (self.active_raphe_curvature_inv_um.is_none()
                || self.observed_path_curvature_inv_um.is_none())
        {
            return Err(AlgalKinematicsError::MissingRapheMeasurement);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RapheModeCalibration {
    pub slope: f64,
    pub intercept_inv_um: f64,
    pub sample_count: usize,
    pub training_rmse_inv_um: f64,
}

impl RapheModeCalibration {
    pub fn predict_path_curvature_inv_um(&self, raphe_curvature_inv_um: f64) -> Option<f64> {
        if !raphe_curvature_inv_um.is_finite() {
            return None;
        }
        let predicted = self.slope * raphe_curvature_inv_um + self.intercept_inv_um;
        predicted.is_finite().then_some(predicted)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RapheCurvatureCalibration {
    pub one_branch: Option<RapheModeCalibration>,
    pub two_branches: Option<RapheModeCalibration>,
    pub eligible_sample_count: usize,
    pub excluded_sample_count: usize,
}

/// Fit separate affine maps from local raphe curvature to observed path curvature.
/// Only DIRECTLY_OBSERVED and MODEL_LATENT observations enter the fit.
pub fn fit_raphe_curvature_model(
    observations: &[RapheCurvatureObservation<'_>],
) -> Result<RapheCurvatureCalibration, AlgalKinematicsError> {
    let mut sample_ids = HashSet::new();
    let mut one_branch = Vec::new();
    let mut two_branches = Vec::new();
    let mut eligible_sample_count = 0;
    let mut excluded_sample_count = 0;

    for observation in observations {
        observation.validate()?;
        if !sample_ids.insert(observation.sample_id) {
            return Err(AlgalKinematicsError::DuplicateSampleId);
        }
        let eligible_evidence = matches!(
            observation.evidence_status,
            EventEvidenceStatus::DirectlyObserved | EventEvidenceStatus::ModelLatent
        );
        if !eligible_evidence || !observation.contact_mode.is_calibratable() {
            excluded_sample_count += 1;
            continue;
        }
        let x = observation.active_raphe_curvature_inv_um.unwrap();
        let y = observation.observed_path_curvature_inv_um.unwrap();
        eligible_sample_count += 1;
        match observation.contact_mode {
            RapheContactMode::OneRapheBranch => one_branch.push((x, y)),
            RapheContactMode::TwoRapheBranches => two_branches.push((x, y)),
            RapheContactMode::NoContact | RapheContactMode::Unresolved => unreachable!(),
        }
    }
    if eligible_sample_count == 0 {
        return Err(AlgalKinematicsError::NoEligibleRapheSamples);
    }
    Ok(RapheCurvatureCalibration {
        one_branch: fit_affine_mode(&one_branch),
        two_branches: fit_affine_mode(&two_branches),
        eligible_sample_count,
        excluded_sample_count,
    })
}

fn fit_affine_mode(samples: &[(f64, f64)]) -> Option<RapheModeCalibration> {
    if samples.len() < 2 {
        return None;
    }
    let count = samples.len() as f64;
    let mean_x = samples.iter().map(|(x, _)| x).sum::<f64>() / count;
    let mean_y = samples.iter().map(|(_, y)| y).sum::<f64>() / count;
    let sxx = samples.iter().map(|(x, _)| (x - mean_x).powi(2)).sum::<f64>();
    if !sxx.is_finite() || sxx <= f64::EPSILON {
        return None;
    }
    let sxy = samples
        .iter()
        .map(|(x, y)| (x - mean_x) * (y - mean_y))
        .sum::<f64>();
    let slope = sxy / sxx;
    let intercept_inv_um = mean_y - slope * mean_x;
    let squared_error = samples
        .iter()
        .map(|(x, y)| (y - (slope * x + intercept_inv_um)).powi(2))
        .sum::<f64>();
    let training_rmse_inv_um = (squared_error / count).sqrt();
    if !slope.is_finite() || !intercept_inv_um.is_finite() || !training_rmse_inv_um.is_finite() {
        return None;
    }
    Some(RapheModeCalibration {
        slope,
        intercept_inv_um,
        sample_count: samples.len(),
        training_rmse_inv_um,
    })
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RapheCurvatureMetrics {
    pub sample_count: usize,
    pub mean_absolute_error_inv_um: f64,
    pub root_mean_squared_error_inv_um: f64,
    pub mean_bias_inv_um: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RapheCurvatureScore {
    pub one_branch: Option<RapheCurvatureMetrics>,
    pub two_branches: Option<RapheCurvatureMetrics>,
    pub eligible_sample_count: usize,
    pub excluded_sample_count: usize,
}

/// Score a fitted calibration on separate held-out observations.
pub fn score_raphe_curvature_model(
    observations: &[RapheCurvatureObservation<'_>],
    calibration: RapheCurvatureCalibration,
) -> Result<RapheCurvatureScore, AlgalKinematicsError> {
    let mut sample_ids = HashSet::new();
    let mut one_branch = Vec::new();
    let mut two_branches = Vec::new();
    let mut eligible_sample_count = 0;
    let mut excluded_sample_count = 0;

    for observation in observations {
        observation.validate()?;
        if !sample_ids.insert(observation.sample_id) {
            return Err(AlgalKinematicsError::DuplicateSampleId);
        }
        let eligible_evidence = matches!(
            observation.evidence_status,
            EventEvidenceStatus::DirectlyObserved | EventEvidenceStatus::ModelLatent
        );
        if !eligible_evidence || !observation.contact_mode.is_calibratable() {
            excluded_sample_count += 1;
            continue;
        }
        let x = observation.active_raphe_curvature_inv_um.unwrap();
        let y = observation.observed_path_curvature_inv_um.unwrap();
        eligible_sample_count += 1;
        let (fit, residuals) = match observation.contact_mode {
            RapheContactMode::OneRapheBranch => (calibration.one_branch, &mut one_branch),
            RapheContactMode::TwoRapheBranches => (calibration.two_branches, &mut two_branches),
            RapheContactMode::NoContact | RapheContactMode::Unresolved => unreachable!(),
        };
        let fit = fit.ok_or(AlgalKinematicsError::MissingRapheModeCalibration)?;
        let predicted = fit
            .predict_path_curvature_inv_um(x)
            .ok_or(AlgalKinematicsError::NonFinitePrediction)?;
        residuals.push(y - predicted);
    }
    Ok(RapheCurvatureScore {
        one_branch: residual_metrics(&one_branch),
        two_branches: residual_metrics(&two_branches),
        eligible_sample_count,
        excluded_sample_count,
    })
}

fn residual_metrics(residuals: &[f64]) -> Option<RapheCurvatureMetrics> {
    if residuals.is_empty() {
        return None;
    }
    let count = residuals.len() as f64;
    let mean_bias_inv_um = residuals.iter().sum::<f64>() / count;
    let mean_absolute_error_inv_um = residuals.iter().map(|x| x.abs()).sum::<f64>() / count;
    let root_mean_squared_error_inv_um =
        (residuals.iter().map(|x| x.powi(2)).sum::<f64>() / count).sqrt();
    Some(RapheCurvatureMetrics {
        sample_count: residuals.len(),
        mean_absolute_error_inv_um,
        root_mean_squared_error_inv_um,
        mean_bias_inv_um,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChlamydomonasKinematicClass {
    StationaryOrLocal,
    ConfinedOrCircling,
    Directional,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PhototaxisAlignment {
    Positive,
    Negative,
    WeakOrAmbiguous,
    Unresolved,
    NotAssessed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChlamydomonasTrack {
    pub track_id: String,
    pub duration_s: f64,
    pub path_length_um: f64,
    pub net_displacement_um: f64,
    pub net_displacement_heading_rad: Option<f64>,
    /// Direction of light propagation, not the direction of its gradient.
    pub stimulus_axis_heading_rad: Option<f64>,
}

impl ChlamydomonasTrack {
    fn validate(&self) -> Result<(), AlgalKinematicsError> {
        if self.track_id.trim().is_empty() {
            return Err(AlgalKinematicsError::EmptyTrackId);
        }
        for value in [self.duration_s, self.path_length_um, self.net_displacement_um] {
            if !value.is_finite() || value < 0.0 {
                return Err(AlgalKinematicsError::InvalidTrackMeasurement);
            }
        }
        if self.duration_s <= 0.0 {
            return Err(AlgalKinematicsError::InvalidTrackDuration);
        }
        if self.net_displacement_um > self.path_length_um + 1.0e-9 {
            return Err(AlgalKinematicsError::DisplacementExceedsPath);
        }
        for heading in [self.net_displacement_heading_rad, self.stimulus_axis_heading_rad]
            .into_iter()
            .flatten()
        {
            if !heading.is_finite() {
                return Err(AlgalKinematicsError::NonFiniteHeading);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChlamydomonasClassificationPolicy {
    pub minimum_mean_speed_um_s: f64,
    pub minimum_net_displacement_um: f64,
    pub confined_max_straightness: f64,
    pub directional_min_straightness: f64,
    pub phototaxis_alignment_threshold: f64,
}

impl ChlamydomonasClassificationPolicy {
    pub fn validate(&self) -> Result<(), AlgalKinematicsError> {
        if !self.minimum_mean_speed_um_s.is_finite()
            || self.minimum_mean_speed_um_s <= 0.0
            || !self.minimum_net_displacement_um.is_finite()
            || self.minimum_net_displacement_um < 0.0
            || !self.confined_max_straightness.is_finite()
            || !self.directional_min_straightness.is_finite()
            || !(0.0..=1.0).contains(&self.confined_max_straightness)
            || !(0.0..=1.0).contains(&self.directional_min_straightness)
            || self.confined_max_straightness >= self.directional_min_straightness
            || !self.phototaxis_alignment_threshold.is_finite()
            || !(0.0..1.0).contains(&self.phototaxis_alignment_threshold)
        {
            return Err(AlgalKinematicsError::InvalidClassificationPolicy);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChlamydomonasTrackClassification {
    pub track_id: String,
    pub kinematic_class: ChlamydomonasKinematicClass,
    pub phototaxis_alignment: PhototaxisAlignment,
    pub straightness: f64,
    pub stimulus_alignment_cosine: Option<f64>,
    pub evidence_status: EventEvidenceStatus,
}

pub fn classify_chlamydomonas_track(
    track: &ChlamydomonasTrack,
    policy: ChlamydomonasClassificationPolicy,
) -> Result<ChlamydomonasTrackClassification, AlgalKinematicsError> {
    track.validate()?;
    policy.validate()?;
    let mean_speed_um_s = track.path_length_um / track.duration_s;
    if !mean_speed_um_s.is_finite() {
        return Err(AlgalKinematicsError::NonFiniteDerivedValue);
    }
    let straightness = if track.path_length_um > f64::EPSILON {
        (track.net_displacement_um / track.path_length_um).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let kinematic_class = if mean_speed_um_s < policy.minimum_mean_speed_um_s
        || track.net_displacement_um < policy.minimum_net_displacement_um
    {
        ChlamydomonasKinematicClass::StationaryOrLocal
    } else if straightness <= policy.confined_max_straightness {
        ChlamydomonasKinematicClass::ConfinedOrCircling
    } else if straightness >= policy.directional_min_straightness {
        ChlamydomonasKinematicClass::Directional
    } else {
        ChlamydomonasKinematicClass::Unresolved
    };
    let stimulus_alignment_cosine = match (
        kinematic_class,
        track.net_displacement_heading_rad,
        track.positive_phototaxis_axis_heading_rad,
    ) {
        (
            ChlamydomonasKinematicClass::Directional,
            Some(track_heading),
            Some(stimulus_heading),
        ) => Some((track_heading - stimulus_heading).cos().clamp(-1.0, 1.0)),
        _ => None,
    };
    let phototaxis_alignment = if kinematic_class != ChlamydomonasKinematicClass::Directional {
        PhototaxisAlignment::NotAssessed
    } else if let Some(alignment) = stimulus_alignment_cosine {
        if alignment >= policy.phototaxis_alignment_threshold {
            PhototaxisAlignment::Positive
        } else if alignment <= -policy.phototaxis_alignment_threshold {
            PhototaxisAlignment::Negative
        } else {
            PhototaxisAlignment::WeakOrAmbiguous
        }
    } else {
        PhototaxisAlignment::Unresolved
    };
    Ok(ChlamydomonasTrackClassification {
        track_id: track.track_id.clone(),
        kinematic_class,
        phototaxis_alignment,
        straightness,
        stimulus_alignment_cosine,
        evidence_status: EventEvidenceStatus::AlgorithmInferred,
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChlamydomonasPopulationSummary {
    pub classified_track_count: usize,
    pub source_rejected_track_count: usize,
    pub stationary_or_local_count: usize,
    pub confined_or_circling_count: usize,
    pub directional_count: usize,
    pub unresolved_kinematic_count: usize,
    pub positive_phototaxis_count: usize,
    pub negative_phototaxis_count: usize,
    pub weak_or_ambiguous_alignment_count: usize,
    pub unresolved_alignment_count: usize,
    pub not_assessed_alignment_count: usize,
    pub positive_fraction_among_aligned_directional: Option<f64>,
    pub negative_fraction_among_aligned_directional: Option<f64>,
    pub mean_directional_alignment: Option<f64>,
    pub directional_orientation_order: OrientationOrder,
}

pub fn summarize_chlamydomonas_tracks(
    tracks: &[ChlamydomonasTrack],
    policy: ChlamydomonasClassificationPolicy,
    source_rejected_track_count: usize,
) -> Result<ChlamydomonasPopulationSummary, AlgalKinematicsError> {
    policy.validate()?;
    let mut track_ids = HashSet::new();
    let mut stationary_or_local_count = 0;
    let mut confined_or_circling_count = 0;
    let mut directional_count = 0;
    let mut unresolved_kinematic_count = 0;
    let mut positive_phototaxis_count = 0;
    let mut negative_phototaxis_count = 0;
    let mut weak_or_ambiguous_alignment_count = 0;
    let mut unresolved_alignment_count = 0;
    let mut not_assessed_alignment_count = 0;
    let mut directional_headings = Vec::new();
    let mut directional_alignments = Vec::new();

    for track in tracks {
        if !track_ids.insert(track.track_id.as_str()) {
            return Err(AlgalKinematicsError::DuplicateTrackId);
        }
        let classification = classify_chlamydomonas_track(track, policy)?;
        match classification.kinematic_class {
            ChlamydomonasKinematicClass::StationaryOrLocal => stationary_or_local_count += 1,
            ChlamydomonasKinematicClass::ConfinedOrCircling => confined_or_circling_count += 1,
            ChlamydomonasKinematicClass::Directional => {
                directional_count += 1;
                if let Some(heading) = track.net_displacement_heading_rad {
                    directional_headings.push(heading);
                }
                if let Some(alignment) = classification.stimulus_alignment_cosine {
                    directional_alignments.push(alignment);
                }
            }
            ChlamydomonasKinematicClass::Unresolved => unresolved_kinematic_count += 1,
        }
        match classification.phototaxis_alignment {
            PhototaxisAlignment::Positive => positive_phototaxis_count += 1,
            PhototaxisAlignment::Negative => negative_phototaxis_count += 1,
            PhototaxisAlignment::WeakOrAmbiguous => weak_or_ambiguous_alignment_count += 1,
            PhototaxisAlignment::Unresolved => unresolved_alignment_count += 1,
            PhototaxisAlignment::NotAssessed => not_assessed_alignment_count += 1,
        }
    }

    let aligned_directional_count = positive_phototaxis_count + negative_phototaxis_count;
    let mean_directional_alignment = if directional_alignments.is_empty() {
        None
    } else {
        Some(directional_alignments.iter().sum::<f64>() / directional_alignments.len() as f64)
    };
    let directional_orientation_order = orientation_order(&directional_headings)
        .map_err(|_| AlgalKinematicsError::NonFiniteDerivedValue)?;

    Ok(ChlamydomonasPopulationSummary {
        classified_track_count: tracks.len(),
        source_rejected_track_count,
        stationary_or_local_count,
        confined_or_circling_count,
        directional_count,
        unresolved_kinematic_count,
        positive_phototaxis_count,
        negative_phototaxis_count,
        weak_or_ambiguous_alignment_count,
        unresolved_alignment_count,
        not_assessed_alignment_count,
        positive_fraction_among_aligned_directional: (aligned_directional_count > 0)
            .then(|| positive_phototaxis_count as f64 / aligned_directional_count as f64),
        negative_fraction_among_aligned_directional: (aligned_directional_count > 0)
            .then(|| negative_phototaxis_count as f64 / aligned_directional_count as f64),
        mean_directional_alignment,
        directional_orientation_order,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlgalKinematicsError {
    EmptySampleId,
    EmptyTrackId,
    InvalidObservationTime,
    InvalidTrackDuration,
    InvalidTrackMeasurement,
    InvalidClassificationPolicy,
    NonFiniteMeasurement,
    NonFiniteHeading,
    NonFiniteDerivedValue,
    DisplacementExceedsPath,
    MissingRapheMeasurement,
    DuplicateSampleId,
    DuplicateTrackId,
    NoEligibleRapheSamples,
    MissingRapheModeCalibration,
    NonFinitePrediction,
}

impl fmt::Display for AlgalKinematicsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySampleId => write!(formatter, "raphe sample ID must be non-empty"),
            Self::EmptyTrackId => write!(formatter, "track ID must be non-empty"),
            Self::InvalidObservationTime => {
                write!(formatter, "observation time must be finite and non-negative")
            }
            Self::InvalidTrackDuration => {
                write!(formatter, "track duration must be finite and positive")
            }
            Self::InvalidTrackMeasurement => {
                write!(formatter, "track path/displacement must be finite and non-negative")
            }
            Self::InvalidClassificationPolicy => {
                write!(formatter, "classification thresholds must be finite, ordered and in range")
            }
            Self::NonFiniteMeasurement => write!(formatter, "curvature measurement must be finite"),
            Self::NonFiniteHeading => {
                write!(formatter, "track and stimulus headings must be finite")
            }
            Self::NonFiniteDerivedValue => {
                write!(formatter, "derived kinematic value is non-finite")
            }
            Self::DisplacementExceedsPath => {
                write!(formatter, "net displacement cannot exceed path length")
            }
            Self::MissingRapheMeasurement => {
                write!(formatter, "one/two-raphe contact needs raphe and path curvature measurements")
            }
            Self::DuplicateSampleId => write!(formatter, "duplicate raphe sample ID"),
            Self::DuplicateTrackId => write!(formatter, "duplicate track ID"),
            Self::NoEligibleRapheSamples => {
                write!(formatter, "no evidence-qualified raphe observations for calibration")
            }
            Self::MissingRapheModeCalibration => {
                write!(formatter, "held-out sample uses a raphe mode without fitted calibration")
            }
            Self::NonFinitePrediction => write!(formatter, "curvature prediction is non-finite"),
        }
    }
}

impl Error for AlgalKinematicsError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn raphe_samples() -> Vec<RapheCurvatureObservation<'static>> {
        vec![
            RapheCurvatureObservation {
                sample_id: "one-1",
                time_s: 0.0,
                contact_mode: RapheContactMode::OneRapheBranch,
                active_raphe_curvature_inv_um: Some(0.1),
                observed_path_curvature_inv_um: Some(0.25),
                evidence_status: EventEvidenceStatus::DirectlyObserved,
            },
            RapheCurvatureObservation {
                sample_id: "one-2",
                time_s: 1.0,
                contact_mode: RapheContactMode::OneRapheBranch,
                active_raphe_curvature_inv_um: Some(0.2),
                observed_path_curvature_inv_um: Some(0.45),
                evidence_status: EventEvidenceStatus::DirectlyObserved,
            },
            RapheCurvatureObservation {
                sample_id: "one-3",
                time_s: 2.0,
                contact_mode: RapheContactMode::OneRapheBranch,
                active_raphe_curvature_inv_um: Some(0.3),
                observed_path_curvature_inv_um: Some(0.65),
                evidence_status: EventEvidenceStatus::ModelLatent,
            },
            RapheCurvatureObservation {
                sample_id: "two-1",
                time_s: 0.0,
                contact_mode: RapheContactMode::TwoRapheBranches,
                active_raphe_curvature_inv_um: Some(0.1),
                observed_path_curvature_inv_um: Some(0.12),
                evidence_status: EventEvidenceStatus::DirectlyObserved,
            },
            RapheCurvatureObservation {
                sample_id: "two-2",
                time_s: 1.0,
                contact_mode: RapheContactMode::TwoRapheBranches,
                active_raphe_curvature_inv_um: Some(0.2),
                observed_path_curvature_inv_um: Some(0.22),
                evidence_status: EventEvidenceStatus::DirectlyObserved,
            },
            RapheCurvatureObservation {
                sample_id: "inferred",
                time_s: 1.0,
                contact_mode: RapheContactMode::OneRapheBranch,
                active_raphe_curvature_inv_um: Some(0.4),
                observed_path_curvature_inv_um: Some(0.85),
                evidence_status: EventEvidenceStatus::AlgorithmInferred,
            },
            RapheCurvatureObservation {
                sample_id: "unresolved",
                time_s: 1.0,
                contact_mode: RapheContactMode::Unresolved,
                active_raphe_curvature_inv_um: None,
                observed_path_curvature_inv_um: None,
                evidence_status: EventEvidenceStatus::Unresolved,
            },
        ]
    }

    #[test]
    fn raphe_curvature_fits_distinct_contact_mode_maps() {
        let fit = fit_raphe_curvature_model(&raphe_samples()).unwrap();
        let one = fit.one_branch.unwrap();
        let two = fit.two_branches.unwrap();
        assert!((one.slope - 2.0).abs() < 1.0e-12);
        assert!((one.intercept_inv_um - 0.05).abs() < 1.0e-12);
        assert!((two.slope - 1.0).abs() < 1.0e-12);
        assert!((two.intercept_inv_um - 0.02).abs() < 1.0e-12);
        assert_eq!(one.sample_count, 3);
        assert_eq!(two.sample_count, 2);
        assert_eq!(fit.eligible_sample_count, 5);
        assert_eq!(fit.excluded_sample_count, 2);
    }

    #[test]
    fn raphe_holdout_scoring_keeps_modes_and_exclusions_separate() {
        let calibration = fit_raphe_curvature_model(&raphe_samples()).unwrap();
        let held_out = [
            RapheCurvatureObservation {
                sample_id: "holdout-one",
                time_s: 3.0,
                contact_mode: RapheContactMode::OneRapheBranch,
                active_raphe_curvature_inv_um: Some(0.5),
                observed_path_curvature_inv_um: Some(1.05),
                evidence_status: EventEvidenceStatus::DirectlyObserved,
            },
            RapheCurvatureObservation {
                sample_id: "holdout-two",
                time_s: 3.0,
                contact_mode: RapheContactMode::TwoRapheBranches,
                active_raphe_curvature_inv_um: Some(0.4),
                observed_path_curvature_inv_um: Some(0.42),
                evidence_status: EventEvidenceStatus::ModelLatent,
            },
            RapheCurvatureObservation {
                sample_id: "holdout-inferred",
                time_s: 3.0,
                contact_mode: RapheContactMode::OneRapheBranch,
                active_raphe_curvature_inv_um: Some(0.6),
                observed_path_curvature_inv_um: Some(1.25),
                evidence_status: EventEvidenceStatus::AlgorithmInferred,
            },
        ];
        let score = score_raphe_curvature_model(&held_out, calibration).unwrap();
        assert_eq!(score.eligible_sample_count, 2);
        assert_eq!(score.excluded_sample_count, 1);
        assert_eq!(score.one_branch.unwrap().sample_count, 1);
        assert_eq!(score.two_branches.unwrap().sample_count, 1);
        assert!(score.one_branch.unwrap().root_mean_squared_error_inv_um < 1.0e-12);
    }

    #[test]
    fn raphe_curvature_sign_is_preserved_and_duplicate_ids_rejected() {
        let mut samples = raphe_samples();
        samples[0].active_raphe_curvature_inv_um = Some(-0.1);
        samples[0].observed_path_curvature_inv_um = Some(-0.15);
        assert!(fit_raphe_curvature_model(&samples).is_ok());
        samples[1].sample_id = "one-1";
        assert_eq!(
            fit_raphe_curvature_model(&samples),
            Err(AlgalKinematicsError::DuplicateSampleId)
        );
    }

    #[test]
    fn raphe_mode_with_no_curvature_variation_is_unidentifiable() {
        let samples = [
            RapheCurvatureObservation {
                sample_id: "constant-one",
                time_s: 0.0,
                contact_mode: RapheContactMode::OneRapheBranch,
                active_raphe_curvature_inv_um: Some(0.2),
                observed_path_curvature_inv_um: Some(0.3),
                evidence_status: EventEvidenceStatus::DirectlyObserved,
            },
            RapheCurvatureObservation {
                sample_id: "constant-two",
                time_s: 1.0,
                contact_mode: RapheContactMode::OneRapheBranch,
                active_raphe_curvature_inv_um: Some(0.2),
                observed_path_curvature_inv_um: Some(0.4),
                evidence_status: EventEvidenceStatus::DirectlyObserved,
            },
        ];
        let fit = fit_raphe_curvature_model(&samples).unwrap();
        assert_eq!(fit.one_branch, None);
        assert_eq!(fit.two_branches, None);
    }

    fn policy() -> ChlamydomonasClassificationPolicy {
        ChlamydomonasClassificationPolicy {
            minimum_mean_speed_um_s: 0.5,
            minimum_net_displacement_um: 1.0,
            confined_max_straightness: 0.35,
            directional_min_straightness: 0.70,
            phototaxis_alignment_threshold: 0.5,
        }
    }

    fn track(
        id: &str,
        path_length_um: f64,
        net_displacement_um: f64,
        duration_s: f64,
        net_heading: Option<f64>,
        stimulus_heading: Option<f64>,
    ) -> ChlamydomonasTrack {
        ChlamydomonasTrack {
            track_id: id.to_owned(),
            duration_s,
            path_length_um,
            net_displacement_um,
            net_displacement_heading_rad: net_heading,
            stimulus_axis_heading_rad: stimulus_heading,
        }
    }

    #[test]
    fn chlamydomonas_classes_follow_declared_kinematic_policy() {
        let stationary = track("stationary", 0.5, 0.2, 2.0, None, None);
        let confined = track("confined", 20.0, 4.0, 5.0, Some(0.5), Some(0.0));
        let directional = track("directional", 20.0, 18.0, 5.0, Some(0.0), Some(0.0));
        assert_eq!(
            classify_chlamydomonas_track(&stationary, policy())
                .unwrap()
                .kinematic_class,
            ChlamydomonasKinematicClass::StationaryOrLocal
        );
        assert_eq!(
            classify_chlamydomonas_track(&confined, policy())
                .unwrap()
                .kinematic_class,
            ChlamydomonasKinematicClass::ConfinedOrCircling
        );
        let result = classify_chlamydomonas_track(&directional, policy()).unwrap();
        assert_eq!(result.kinematic_class, ChlamydomonasKinematicClass::Directional);
        assert_eq!(result.phototaxis_alignment, PhototaxisAlignment::Positive);
        assert_eq!(result.evidence_status, EventEvidenceStatus::AlgorithmInferred);
    }

    #[test]
    fn opposing_phototactic_subgroups_are_not_hidden_by_the_mean() {
        let tracks = [
            track("positive-one", 20.0, 19.0, 5.0, Some(0.0), Some(0.0)),
            track("positive-two", 20.0, 19.0, 5.0, Some(0.0), Some(0.0)),
            track("negative-one", 20.0, 19.0, 5.0, Some(std::f64::consts::PI), Some(0.0)),
            track("negative-two", 20.0, 19.0, 5.0, Some(std::f64::consts::PI), Some(0.0)),
        ];
        let summary = summarize_chlamydomonas_tracks(&tracks, policy(), 3).unwrap();
        assert_eq!(summary.classified_track_count, 4);
        assert_eq!(summary.source_rejected_track_count, 3);
        assert_eq!(summary.directional_count, 4);
        assert_eq!(summary.positive_phototaxis_count, 2);
        assert_eq!(summary.negative_phototaxis_count, 2);
        assert!(summary.mean_directional_alignment.unwrap().abs() < 1.0e-12);
        assert!(summary.directional_orientation_order.polar_order.unwrap() < 1.0e-12);
        assert!(
            (summary.directional_orientation_order.nematic_order.unwrap() - 1.0).abs()
                < 1.0e-12
        );
    }

    #[test]
    fn missing_or_ambiguous_stimulus_alignment_remains_unresolved() {
        let missing = track("missing-stimulus", 20.0, 18.0, 5.0, Some(0.0), None);
        let ambiguous = track(
            "ambiguous",
            20.0,
            18.0,
            5.0,
            Some(std::f64::consts::FRAC_PI_2),
            Some(0.0),
        );
        assert_eq!(
            classify_chlamydomonas_track(&missing, policy())
                .unwrap()
                .phototaxis_alignment,
            PhototaxisAlignment::Unresolved
        );
        assert_eq!(
            classify_chlamydomonas_track(&ambiguous, policy())
                .unwrap()
                .phototaxis_alignment,
            PhototaxisAlignment::WeakOrAmbiguous
        );
    }

    #[test]
    fn invalid_tracks_and_duplicate_ids_fail_closed() {
        let invalid = track("invalid", 2.0, 3.0, 1.0, Some(0.0), Some(0.0));
        assert_eq!(
            classify_chlamydomonas_track(&invalid, policy()),
            Err(AlgalKinematicsError::DisplacementExceedsPath)
        );
        let duplicate = track("same", 20.0, 18.0, 5.0, Some(0.0), Some(0.0));
        assert_eq!(
            summarize_chlamydomonas_tracks(&[duplicate.clone(), duplicate], policy(), 0),
            Err(AlgalKinematicsError::DuplicateTrackId)
        );
    }

    #[test]
    fn empty_population_returns_unknown_statistics_not_zero() {
        let summary = summarize_chlamydomonas_tracks(&[], policy(), 4).unwrap();
        assert_eq!(summary.classified_track_count, 0);
        assert_eq!(summary.source_rejected_track_count, 4);
        assert_eq!(summary.mean_directional_alignment, None);
        assert_eq!(summary.directional_orientation_order.polar_order, None);
        assert_eq!(summary.positive_fraction_among_aligned_directional, None);
    }

    #[test]
    fn invalid_classification_policy_fails_closed() {
        let mut invalid = policy();
        invalid.confined_max_straightness = 0.8;
        invalid.directional_min_straightness = 0.7;
        assert_eq!(
            summarize_chlamydomonas_tracks(&[], invalid, 0),
            Err(AlgalKinematicsError::InvalidClassificationPolicy)
        );
    }
}
