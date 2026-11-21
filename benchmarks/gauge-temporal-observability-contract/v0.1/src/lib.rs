//! Observable-specific temporal-resolution and processing-lineage contract.
//!
//! This crate evaluates explicit sampling requirements against an exported
//! sample rate. It does not recover information lost during downsampling,
//! estimate unknown event bandwidths, or prove measurement-system fidelity.

use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SamplingProfile {
    /// Rate of the trajectory currently being evaluated (for GAUGE JSON this
    /// is the declared 30 FPS export cadence).
    pub exported_sample_rate_hz: f64,
    /// Original source acquisition rate, if documented independently.
    pub source_acquisition_rate_hz: Option<f64>,
    /// The resampling filter is documented, or the identity/no-resampling
    /// path is explicitly declared when source and export rates are equal.
    pub resampling_filter_documented: bool,
    pub sample_phase_documented: bool,
    pub timestamp_mapping_documented: bool,
    pub uncertainty_propagation_documented: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EventRequirements {
    /// Event duration in seconds, independently characterized or preregistered.
    pub event_duration_seconds: Option<f64>,
    /// Minimum number of exported samples required across the event.
    pub minimum_samples_per_event: Option<f64>,
    /// Highest signal frequency that the claim needs to preserve, in Hz.
    pub maximum_signal_bandwidth_hz: Option<f64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoverageDisposition {
    MeetsDeclaredThresholds,
    Undersampled,
    Unassessable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoverageFailure {
    InsufficientSamplesAcrossEvent,
    BandwidthExceedsNyquist,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MissingRequirement {
    EventDuration,
    MinimumSamplesPerEvent,
    MaximumSignalBandwidth,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProvenanceGap {
    SourceAcquisitionRate,
    ResamplingFilter,
    SamplePhase,
    TimestampMapping,
    UncertaintyPropagation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProvenanceDisposition {
    Complete,
    Incomplete,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TemporalError {
    InvalidExportSampleRate,
    InvalidSourceAcquisitionRate,
    SourceRateBelowExportRate,
    InvalidEventDuration,
    InvalidMinimumSamples,
    InvalidSignalBandwidth,
    NonFiniteDerivedValue,
}

impl Display for TemporalError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidExportSampleRate => {
                write!(formatter, "export sample rate must be finite and positive")
            }
            Self::InvalidSourceAcquisitionRate => {
                write!(
                    formatter,
                    "source acquisition rate must be finite and positive"
                )
            }
            Self::SourceRateBelowExportRate => {
                write!(
                    formatter,
                    "source acquisition rate cannot be below export sample rate"
                )
            }
            Self::InvalidEventDuration => {
                write!(formatter, "event duration must be finite and positive")
            }
            Self::InvalidMinimumSamples => {
                write!(
                    formatter,
                    "minimum samples per event must be finite and positive"
                )
            }
            Self::InvalidSignalBandwidth => {
                write!(formatter, "signal bandwidth must be finite and positive")
            }
            Self::NonFiniteDerivedValue => {
                write!(formatter, "derived sampling value is non-finite")
            }
        }
    }
}

impl Error for TemporalError {}

#[derive(Clone, Debug, PartialEq)]
pub struct TemporalAssessment {
    pub exported_sample_rate_hz: f64,
    pub source_acquisition_rate_hz: Option<f64>,
    /// Source acquisition rate / export sample rate, where source rate is known.
    pub acquisition_to_export_ratio: Option<f64>,
    pub sample_interval_seconds: f64,
    pub nyquist_frequency_hz: f64,
    pub samples_per_event: Option<f64>,
    pub coverage_disposition: CoverageDisposition,
    pub coverage_failures: Vec<CoverageFailure>,
    pub missing_requirements: Vec<MissingRequirement>,
    pub provenance_disposition: ProvenanceDisposition,
    pub provenance_gaps: Vec<ProvenanceGap>,
}

impl TemporalAssessment {
    /// Whether the temporal coverage meets the *declared* numeric requirements.
    /// This deliberately says nothing about unknown filter or sensor response.
    pub fn meets_declared_coverage(&self) -> bool {
        self.coverage_disposition == CoverageDisposition::MeetsDeclaredThresholds
    }

    /// A stronger conjunction: declared temporal thresholds pass and every
    /// acquisition/export processing-provenance gate is documented.
    pub fn is_fully_qualified_under_declared_contract(&self) -> bool {
        self.meets_declared_coverage()
            && self.provenance_disposition == ProvenanceDisposition::Complete
    }
}

pub fn assess_temporal_observability(
    profile: SamplingProfile,
    requirements: EventRequirements,
) -> Result<TemporalAssessment, TemporalError> {
    let export_rate = profile.exported_sample_rate_hz;
    if !export_rate.is_finite() || export_rate <= 0.0 {
        return Err(TemporalError::InvalidExportSampleRate);
    }

    let acquisition_to_export_ratio = match profile.source_acquisition_rate_hz {
        Some(source_rate) => {
            if !source_rate.is_finite() || source_rate <= 0.0 {
                return Err(TemporalError::InvalidSourceAcquisitionRate);
            }
            if source_rate + f64::EPSILON < export_rate {
                return Err(TemporalError::SourceRateBelowExportRate);
            }
            let ratio = source_rate / export_rate;
            if !ratio.is_finite() {
                return Err(TemporalError::NonFiniteDerivedValue);
            }
            Some(ratio)
        }
        None => None,
    };

    let sample_interval_seconds = 1.0 / export_rate;
    let nyquist_frequency_hz = export_rate / 2.0;
    if !sample_interval_seconds.is_finite() || !nyquist_frequency_hz.is_finite() {
        return Err(TemporalError::NonFiniteDerivedValue);
    }

    let event_duration = match requirements.event_duration_seconds {
        Some(value) if value.is_finite() && value > 0.0 => Some(value),
        Some(_) => return Err(TemporalError::InvalidEventDuration),
        None => None,
    };
    let minimum_samples = match requirements.minimum_samples_per_event {
        Some(value) if value.is_finite() && value > 0.0 => Some(value),
        Some(_) => return Err(TemporalError::InvalidMinimumSamples),
        None => None,
    };
    let maximum_bandwidth = match requirements.maximum_signal_bandwidth_hz {
        Some(value) if value.is_finite() && value > 0.0 => Some(value),
        Some(_) => return Err(TemporalError::InvalidSignalBandwidth),
        None => None,
    };

    let samples_per_event = event_duration.map(|duration| export_rate * duration);
    if samples_per_event.is_some_and(|value| !value.is_finite()) {
        return Err(TemporalError::NonFiniteDerivedValue);
    }

    let mut missing_requirements = Vec::new();
    if event_duration.is_none() {
        missing_requirements.push(MissingRequirement::EventDuration);
    }
    if minimum_samples.is_none() {
        missing_requirements.push(MissingRequirement::MinimumSamplesPerEvent);
    }
    if maximum_bandwidth.is_none() {
        missing_requirements.push(MissingRequirement::MaximumSignalBandwidth);
    }

    let mut coverage_failures = Vec::new();
    let coverage_disposition = if !missing_requirements.is_empty() {
        CoverageDisposition::Unassessable
    } else {
        let sample_count = samples_per_event.expect("event duration is present");
        if sample_count < minimum_samples.expect("sample requirement is present") {
            coverage_failures.push(CoverageFailure::InsufficientSamplesAcrossEvent);
        }
        if nyquist_frequency_hz < maximum_bandwidth.expect("bandwidth is present") {
            coverage_failures.push(CoverageFailure::BandwidthExceedsNyquist);
        }
        if coverage_failures.is_empty() {
            CoverageDisposition::MeetsDeclaredThresholds
        } else {
            CoverageDisposition::Undersampled
        }
    };

    let mut provenance_gaps = Vec::new();
    if profile.source_acquisition_rate_hz.is_none() {
        provenance_gaps.push(ProvenanceGap::SourceAcquisitionRate);
    }
    if !profile.resampling_filter_documented {
        provenance_gaps.push(ProvenanceGap::ResamplingFilter);
    }
    if !profile.sample_phase_documented {
        provenance_gaps.push(ProvenanceGap::SamplePhase);
    }
    if !profile.timestamp_mapping_documented {
        provenance_gaps.push(ProvenanceGap::TimestampMapping);
    }
    if !profile.uncertainty_propagation_documented {
        provenance_gaps.push(ProvenanceGap::UncertaintyPropagation);
    }

    Ok(TemporalAssessment {
        exported_sample_rate_hz: export_rate,
        source_acquisition_rate_hz: profile.source_acquisition_rate_hz,
        acquisition_to_export_ratio,
        sample_interval_seconds,
        nyquist_frequency_hz,
        samples_per_event,
        coverage_disposition,
        coverage_failures,
        missing_requirements,
        provenance_disposition: if provenance_gaps.is_empty() {
            ProvenanceDisposition::Complete
        } else {
            ProvenanceDisposition::Incomplete
        },
        provenance_gaps,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gauge_export_profile() -> SamplingProfile {
        SamplingProfile {
            exported_sample_rate_hz: 30.0,
            source_acquisition_rate_hz: Some(180.0),
            resampling_filter_documented: false,
            sample_phase_documented: false,
            timestamp_mapping_documented: false,
            uncertainty_propagation_documented: false,
        }
    }

    fn event_requirements() -> EventRequirements {
        EventRequirements {
            // Synthetic example values; not claimed as measured GAUGE event
            // duration or contact bandwidth.
            event_duration_seconds: Some(0.030),
            minimum_samples_per_event: Some(5.0),
            maximum_signal_bandwidth_hz: Some(25.0),
        }
    }

    fn complete_profile(rate: f64) -> SamplingProfile {
        SamplingProfile {
            exported_sample_rate_hz: rate,
            source_acquisition_rate_hz: Some(rate),
            resampling_filter_documented: true,
            sample_phase_documented: true,
            timestamp_mapping_documented: true,
            uncertainty_propagation_documented: true,
        }
    }

    #[test]
    fn six_to_one_capture_to_export_ratio_is_derived() {
        let assessment =
            assess_temporal_observability(gauge_export_profile(), event_requirements())
                .expect("valid sampling profile");
        assert_eq!(assessment.acquisition_to_export_ratio, Some(6.0));
        assert!((assessment.sample_interval_seconds - 1.0 / 30.0).abs() < 1.0e-12);
    }

    #[test]
    fn thirty_hz_export_is_under_sampled_for_declared_short_event_example() {
        let assessment =
            assess_temporal_observability(gauge_export_profile(), event_requirements())
                .expect("valid sampling profile");
        assert_eq!(
            assessment.coverage_disposition,
            CoverageDisposition::Undersampled
        );
        assert!((assessment.samples_per_event.unwrap() - 0.9).abs() < 1.0e-12);
        assert_eq!(assessment.nyquist_frequency_hz, 15.0);
        assert!(
            assessment
                .coverage_failures
                .contains(&CoverageFailure::InsufficientSamplesAcrossEvent)
        );
        assert!(
            assessment
                .coverage_failures
                .contains(&CoverageFailure::BandwidthExceedsNyquist)
        );
        assert_eq!(
            assessment.provenance_disposition,
            ProvenanceDisposition::Incomplete
        );
        assert!(!assessment.is_fully_qualified_under_declared_contract());
    }

    #[test]
    fn one_hundred_eighty_hz_meets_the_same_declared_synthetic_thresholds() {
        let assessment =
            assess_temporal_observability(complete_profile(180.0), event_requirements())
                .expect("valid source-rate profile");
        assert_eq!(
            assessment.coverage_disposition,
            CoverageDisposition::MeetsDeclaredThresholds
        );
        assert!((assessment.samples_per_event.unwrap() - 5.4).abs() < 1.0e-12);
        assert_eq!(assessment.nyquist_frequency_hz, 90.0);
        assert_eq!(
            assessment.provenance_disposition,
            ProvenanceDisposition::Complete
        );
        assert!(assessment.is_fully_qualified_under_declared_contract());
    }

    #[test]
    fn missing_event_characterization_never_becomes_a_pass() {
        let missing = EventRequirements {
            event_duration_seconds: None,
            minimum_samples_per_event: Some(5.0),
            maximum_signal_bandwidth_hz: None,
        };
        let assessment = assess_temporal_observability(complete_profile(180.0), missing)
            .expect("valid profile with incomplete requirements");
        assert_eq!(
            assessment.coverage_disposition,
            CoverageDisposition::Unassessable
        );
        assert!(
            assessment
                .missing_requirements
                .contains(&MissingRequirement::EventDuration)
        );
        assert!(
            assessment
                .missing_requirements
                .contains(&MissingRequirement::MaximumSignalBandwidth)
        );
        assert!(!assessment.meets_declared_coverage());
    }

    #[test]
    fn adequate_sample_arithmetic_does_not_hide_unknown_processing_lineage() {
        let mut profile = complete_profile(180.0);
        profile.resampling_filter_documented = false;
        profile.sample_phase_documented = false;
        profile.uncertainty_propagation_documented = false;

        let assessment =
            assess_temporal_observability(profile, event_requirements()).expect("valid profile");
        assert_eq!(
            assessment.coverage_disposition,
            CoverageDisposition::MeetsDeclaredThresholds
        );
        assert_eq!(
            assessment.provenance_disposition,
            ProvenanceDisposition::Incomplete
        );
        assert!(
            assessment
                .provenance_gaps
                .contains(&ProvenanceGap::ResamplingFilter)
        );
        assert!(!assessment.is_fully_qualified_under_declared_contract());
    }

    #[test]
    fn thirty_and_one_eighty_hz_nyquist_known_answers() {
        let at_30 = assess_temporal_observability(
            complete_profile(30.0),
            EventRequirements {
                event_duration_seconds: Some(0.1),
                minimum_samples_per_event: Some(1.0),
                maximum_signal_bandwidth_hz: Some(10.0),
            },
        )
        .expect("valid 30Hz profile");
        let at_180 = assess_temporal_observability(
            complete_profile(180.0),
            EventRequirements {
                event_duration_seconds: Some(0.1),
                minimum_samples_per_event: Some(1.0),
                maximum_signal_bandwidth_hz: Some(10.0),
            },
        )
        .expect("valid 180Hz profile");
        assert_eq!(at_30.nyquist_frequency_hz, 15.0);
        assert_eq!(at_180.nyquist_frequency_hz, 90.0);
    }

    #[test]
    fn rejects_invalid_rates_and_event_requirements() {
        let mut profile = complete_profile(30.0);
        profile.exported_sample_rate_hz = 0.0;
        assert_eq!(
            assess_temporal_observability(profile, event_requirements()),
            Err(TemporalError::InvalidExportSampleRate)
        );

        profile = complete_profile(30.0);
        profile.source_acquisition_rate_hz = Some(20.0);
        assert_eq!(
            assess_temporal_observability(profile, event_requirements()),
            Err(TemporalError::SourceRateBelowExportRate)
        );

        let invalid_event = EventRequirements {
            event_duration_seconds: Some(f64::NAN),
            minimum_samples_per_event: Some(5.0),
            maximum_signal_bandwidth_hz: Some(25.0),
        };
        assert_eq!(
            assess_temporal_observability(complete_profile(180.0), invalid_event),
            Err(TemporalError::InvalidEventDuration)
        );
    }
}
