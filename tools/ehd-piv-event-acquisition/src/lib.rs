// SPDX-License-Identifier: AGPL-3.0-or-later

//! Robust event-PIV temporal-window and illumination-mode planning.
//!
//! This reference chooses a feasible accumulation interval and a calibrated
//! illumination pattern from local flow, event-rate, particle-density,
//! timestamp, packet-loss and buffer evidence. It does not operate cameras,
//! lasers or illumination hardware. Any selected plan is advisory until the
//! acquisition layer verifies its receipts and observer state.
//!
//! The controller uses conservative interval bounds rather than maximizing
//! event count or update rate in isolation. If the minimum information
//! requirement conflicts with displacement, bandwidth, clock or buffer limits,
//! the controller abstains rather than silently clipping or fabricating data.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

pub const MAX_ILLUMINATION_OPTIONS: usize = 16;
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SpatialIlluminationMode {
    FullField,
    BlockScanned,
    LineScanned,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObserverImpact {
    AssessedNonPerturbing,
    PotentiallyPerturbing,
    Unknown,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IlluminationOption {
    pub option_id: String,
    pub mode: SpatialIlluminationMode,
    /// Maximum event throughput measured under this exact sensor/illumination
    /// configuration; events/second.
    pub maximum_event_rate_per_second: f64,
    /// Maximum particle density validated for this exact mode; particles/pixel.
    pub maximum_particle_density_per_pixel: f64,
    /// Calibrated spatial coverage fraction for a single accumulation window.
    pub spatial_coverage_fraction: f64,
    /// Electrical/optical energy rate for matched exposure duration; mJ/s.
    pub energy_rate_mj_per_second: f64,
    /// Maximum validated accumulation duration for this mode.
    pub maximum_window_seconds: f64,
    pub available: bool,
    pub calibration_receipt_id: String,
    pub observer_impact: ObserverImpact,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EventRegionSnapshot {
    pub stream_id: String,
    pub camera_id: String,
    pub clock_domain_id: String,
    pub protocol_fingerprint: String,
    pub observer_state_id: String,
    pub region_id: String,
    pub acquisition_id: String,
    pub dependence_cluster_id: String,
    /// Mean event rate observed over the previous sentinel interval.
    pub event_rate_per_second: f64,
    /// Relative one-sigma uncertainty of event rate, in [0, 1).
    pub event_rate_relative_standard_uncertainty: f64,
    /// Mean local particle event density in particles per pixel.
    pub particle_density_per_pixel: f64,
    /// Relative one-sigma uncertainty of particle density, in [0, 1].
    pub particle_density_relative_standard_uncertainty: f64,
    /// Local displacement velocity magnitude in pixels per second.
    pub velocity_magnitude_pixels_per_second: f64,
    /// One-sigma uncertainty of local velocity magnitude, pixels per second.
    pub velocity_standard_uncertainty_pixels_per_second: f64,
    /// Hardware event buffer capacity for this region/window.
    pub buffer_capacity_events: u64,
    /// Minimum retained events required by the frozen reconstruction contract.
    pub minimum_events_required: u64,
    /// Upper displacement allowed within a single accumulation window.
    pub maximum_displacement_pixels: f64,
    /// Must be present and measured; None means event loss is unknown.
    pub packet_loss_fraction: Option<f64>,
    pub timestamps_monotonic: bool,
    pub clock_synchronization_qualified: bool,
    /// One-sigma timestamp uncertainty, seconds.
    pub clock_standard_uncertainty_seconds: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EventWindowConfig {
    pub confidence_multiplier: f64,
    pub minimum_window_seconds: f64,
    pub maximum_window_seconds: f64,
    /// Maximum allowed fraction of the event buffer used per window, (0, 1].
    pub maximum_buffer_utilization: f64,
    /// Maximum permitted fraction of the selected interval represented by
    /// the confidence-bounded clock uncertainty.
    pub maximum_clock_uncertainty_fraction: f64,
    pub maximum_packet_loss_fraction: f64,
    pub required_spatial_coverage_fraction: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventPlanAbstention {
    NoAvailableCalibratedIlluminationMode,
    EventRateExceedsAllValidatedModes,
    ParticleDensityExceedsAllValidatedModes,
    InsufficientEventInformation,
    TemporalWindowInfeasible,
    EventBufferCapacityInsufficient,
    DisplacementBoundInfeasible,
    ClockResolutionInsufficient,
    PacketLossUnknown,
    PacketLossExceeded,
    TimestampOrderInvalid,
    ClockSynchronizationUnqualified,
    SpatialCoverageUnavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventPlanDisposition {
    AcquisitionPlanAvailable,
    Abstain,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventPlanError {
    EmptyIdentity {
        field: &'static str,
    },
    InvalidMetric {
        field: &'static str,
    },
    InvalidConfiguration {
        field: &'static str,
    },
    InvalidBufferCapacity,
    InvalidEventRequirement,
    TooManyOptions {
        actual: usize,
        maximum: usize,
    },
    EmptyOptionId {
        index: usize,
    },
    DuplicateOptionId(String),
    InvalidOptionLimit {
        option_id: String,
        field: &'static str,
    },
    MissingCalibrationReceipt {
        option_id: String,
    },
    NonFiniteDerivedValue,
}

impl fmt::Display for EventPlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyIdentity { field } => write!(f, "{field} must be non-empty"),
            Self::InvalidMetric { field } => {
                write!(f, "event acquisition metric {field} is invalid")
            }
            Self::InvalidConfiguration { field } => {
                write!(f, "event acquisition configuration {field} is invalid")
            }
            Self::InvalidBufferCapacity => write!(f, "event buffer capacity must be positive"),
            Self::InvalidEventRequirement => {
                write!(
                    f,
                    "minimum event count must be positive and no greater than the buffer"
                )
            }
            Self::TooManyOptions { actual, maximum } => {
                write!(
                    f,
                    "illumination option count {actual} exceeds limit {maximum}"
                )
            }
            Self::EmptyOptionId { index } => {
                write!(f, "illumination option {index} has an empty ID")
            }
            Self::DuplicateOptionId(id) => write!(f, "duplicate illumination option ID: {id}"),
            Self::InvalidOptionLimit { option_id, field } => {
                write!(f, "illumination option {option_id} has invalid {field}")
            }
            Self::MissingCalibrationReceipt { option_id } => {
                write!(
                    f,
                    "illumination option {option_id} has no calibration receipt"
                )
            }
            Self::NonFiniteDerivedValue => {
                write!(
                    f,
                    "event acquisition calculation produced a non-finite value"
                )
            }
        }
    }
}

impl Error for EventPlanError {}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RejectedIlluminationCounts {
    pub unavailable: usize,
    pub missing_receipt: usize,
    pub event_rate_exceeded: usize,
    pub particle_density_exceeded: usize,
    pub spatial_coverage_insufficient: usize,
    pub event_buffer_infeasible: usize,
    pub displacement_infeasible: usize,
    pub clock_resolution_infeasible: usize,
    pub temporal_window_infeasible: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EventWindowPlan {
    pub stream_id: String,
    pub region_id: String,
    pub acquisition_id: String,
    pub dependence_cluster_id: String,
    pub illumination_option_id: String,
    pub illumination_mode: SpatialIlluminationMode,
    pub calibration_receipt_id: String,
    pub accumulation_window_seconds: f64,
    pub minimum_feasible_window_seconds: f64,
    pub maximum_feasible_window_seconds: f64,
    pub expected_retained_event_count_lower: f64,
    pub expected_retained_event_count_upper: f64,
    pub expected_buffer_utilization_upper: f64,
    pub expected_displacement_upper_pixels: f64,
    pub expected_particle_density_upper_per_pixel: f64,
    pub expected_illumination_energy_mj: f64,
    pub confidence_multiplier: f64,
    pub clock_uncertainty_fraction_upper: f64,
    pub post_measurement_observer_requalification_required: bool,
    pub claim_boundary: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EventAcquisitionReport {
    pub disposition: EventPlanDisposition,
    pub abstention_reason: Option<EventPlanAbstention>,
    pub plan: Option<EventWindowPlan>,
    pub rejected_option_counts: RejectedIlluminationCounts,
    pub claim_boundary: &'static str,
}

#[derive(Clone, Debug)]
struct FeasibleCandidate {
    option_index: usize,
    minimum_window_seconds: f64,
    maximum_window_seconds: f64,
    selected_window_seconds: f64,
    event_rate_lower: f64,
    event_rate_upper: f64,
    particle_density_upper: f64,
    buffer_utilization_upper: f64,
    displacement_upper_pixels: f64,
    clock_uncertainty_fraction_upper: f64,
    energy_mj: f64,
}

pub fn plan_event_acquisition(
    snapshot: &EventRegionSnapshot,
    config: &EventWindowConfig,
    options: &[IlluminationOption],
) -> Result<EventAcquisitionReport, EventPlanError> {
    validate_snapshot(snapshot)?;
    validate_config(config)?;
    validate_options(options)?;

    if !snapshot.timestamps_monotonic {
        return Ok(abstain(
            EventPlanAbstention::TimestampOrderInvalid,
            RejectedIlluminationCounts::default(),
        ));
    }
    if !snapshot.clock_synchronization_qualified {
        return Ok(abstain(
            EventPlanAbstention::ClockSynchronizationUnqualified,
            RejectedIlluminationCounts::default(),
        ));
    }
    let packet_loss = match snapshot.packet_loss_fraction {
        Some(value) => value,
        None => {
            return Ok(abstain(
                EventPlanAbstention::PacketLossUnknown,
                RejectedIlluminationCounts::default(),
            ));
        }
    };
    if packet_loss > config.maximum_packet_loss_fraction {
        return Ok(abstain(
            EventPlanAbstention::PacketLossExceeded,
            RejectedIlluminationCounts::default(),
        ));
    }

    let k = config.confidence_multiplier;
    let event_rate_lower = snapshot.event_rate_per_second
        * (1.0 - k * snapshot.event_rate_relative_standard_uncertainty);
    let event_rate_upper = snapshot.event_rate_per_second
        * (1.0 + k * snapshot.event_rate_relative_standard_uncertainty);
    let particle_density_upper = snapshot.particle_density_per_pixel
        * (1.0 + k * snapshot.particle_density_relative_standard_uncertainty);
    let velocity_upper = snapshot.velocity_magnitude_pixels_per_second
        + k * snapshot.velocity_standard_uncertainty_pixels_per_second;

    if ![
        event_rate_lower,
        event_rate_upper,
        particle_density_upper,
        velocity_upper,
    ]
    .iter()
    .all(|value| value.is_finite())
    {
        return Err(EventPlanError::NonFiniteDerivedValue);
    }
    if event_rate_lower <= 0.0 {
        return Ok(abstain(
            EventPlanAbstention::InsufficientEventInformation,
            RejectedIlluminationCounts::default(),
        ));
    }

    let mut rejected = RejectedIlluminationCounts::default();
    let mut best: Option<FeasibleCandidate> = None;

    for (index, option) in options.iter().enumerate() {
        if !option.available {
            rejected.unavailable += 1;
            continue;
        }
        if option.calibration_receipt_id.trim().is_empty() {
            rejected.missing_receipt += 1;
            continue;
        }
        if event_rate_upper > option.maximum_event_rate_per_second {
            rejected.event_rate_exceeded += 1;
            continue;
        }
        if particle_density_upper > option.maximum_particle_density_per_pixel {
            rejected.particle_density_exceeded += 1;
            continue;
        }
        if option.spatial_coverage_fraction < config.required_spatial_coverage_fraction {
            rejected.spatial_coverage_insufficient += 1;
            continue;
        }

        let retained_event_rate_lower = event_rate_lower * (1.0 - packet_loss);
        if retained_event_rate_lower <= 0.0 || !retained_event_rate_lower.is_finite() {
            return Ok(abstain(
                EventPlanAbstention::InsufficientEventInformation,
                rejected,
            ));
        }
        let event_information_window =
            snapshot.minimum_events_required as f64 / retained_event_rate_lower;
        let clock_limited_window = k * snapshot.clock_standard_uncertainty_seconds
            / config.maximum_clock_uncertainty_fraction;
        let minimum_window = config
            .minimum_window_seconds
            .max(event_information_window)
            .max(clock_limited_window);
        let buffer_limited_window = snapshot.buffer_capacity_events as f64
            * config.maximum_buffer_utilization
            / event_rate_upper;
        let displacement_limited_window = if velocity_upper > 0.0 {
            snapshot.maximum_displacement_pixels / velocity_upper
        } else {
            f64::INFINITY
        };
        let maximum_window = config
            .maximum_window_seconds
            .min(option.maximum_window_seconds)
            .min(buffer_limited_window)
            .min(displacement_limited_window);

        if !minimum_window.is_finite() || !maximum_window.is_finite() || maximum_window <= 0.0 {
            return Err(EventPlanError::NonFiniteDerivedValue);
        }
        if minimum_window > maximum_window {
            let buffer_is_binding = minimum_window > buffer_limited_window;
            let displacement_is_binding = minimum_window > displacement_limited_window;
            let other_time_limit_is_binding = minimum_window
                > config
                    .maximum_window_seconds
                    .min(option.maximum_window_seconds);
            let clock_is_dominant_lower_bound = clock_limited_window
                >= event_information_window.max(config.minimum_window_seconds)
                && clock_limited_window > maximum_window;
            if clock_is_dominant_lower_bound {
                rejected.clock_resolution_infeasible += 1;
            } else if buffer_is_binding && !displacement_is_binding && !other_time_limit_is_binding
            {
                rejected.event_buffer_infeasible += 1;
            } else if displacement_is_binding && !buffer_is_binding && !other_time_limit_is_binding
            {
                rejected.displacement_infeasible += 1;
            } else {
                rejected.temporal_window_infeasible += 1;
            }
            continue;
        }

        // Geometric midpoint balances lower information and upper smearing /
        // buffer constraints in multiplicative scale.
        let selected_window = (minimum_window * maximum_window).sqrt();
        let buffer_utilization =
            event_rate_upper * selected_window / snapshot.buffer_capacity_events as f64;
        let displacement_upper = velocity_upper * selected_window;
        let clock_fraction = k * snapshot.clock_standard_uncertainty_seconds / selected_window;
        let energy = option.energy_rate_mj_per_second * selected_window;
        if ![
            selected_window,
            buffer_utilization,
            displacement_upper,
            clock_fraction,
            energy,
        ]
        .iter()
        .all(|value| value.is_finite())
        {
            return Err(EventPlanError::NonFiniteDerivedValue);
        }

        let candidate = FeasibleCandidate {
            option_index: index,
            minimum_window_seconds: minimum_window,
            maximum_window_seconds: maximum_window,
            selected_window_seconds: selected_window,
            event_rate_lower,
            event_rate_upper,
            particle_density_upper,
            buffer_utilization_upper: buffer_utilization,
            displacement_upper_pixels: displacement_upper,
            clock_uncertainty_fraction_upper: clock_fraction,
            energy_mj: energy,
        };
        let replace = best
            .as_ref()
            .is_none_or(|current| candidate_better(&candidate, current, options));
        if replace {
            best = Some(candidate);
        }
    }

    let Some(best) = best else {
        let reason = determine_abstention(&rejected);
        return Ok(abstain(reason, rejected));
    };

    let option = &options[best.option_index];
    let expected_lower = best.event_rate_lower * best.selected_window_seconds * (1.0 - packet_loss);
    let expected_upper = best.event_rate_upper * best.selected_window_seconds * (1.0 - packet_loss);
    if !expected_lower.is_finite() || !expected_upper.is_finite() {
        return Err(EventPlanError::NonFiniteDerivedValue);
    }

    Ok(EventAcquisitionReport {
        disposition: EventPlanDisposition::AcquisitionPlanAvailable,
        abstention_reason: None,
        plan: Some(EventWindowPlan {
            stream_id: snapshot.stream_id.clone(),
            region_id: snapshot.region_id.clone(),
            acquisition_id: snapshot.acquisition_id.clone(),
            dependence_cluster_id: snapshot.dependence_cluster_id.clone(),
            illumination_option_id: option.option_id.clone(),
            illumination_mode: option.mode,
            calibration_receipt_id: option.calibration_receipt_id.clone(),
            accumulation_window_seconds: best.selected_window_seconds,
            minimum_feasible_window_seconds: best.minimum_window_seconds,
            maximum_feasible_window_seconds: best.maximum_window_seconds,
            expected_retained_event_count_lower: expected_lower,
            expected_retained_event_count_upper: expected_upper,
            expected_buffer_utilization_upper: best.buffer_utilization_upper,
            expected_displacement_upper_pixels: best.displacement_upper_pixels,
            expected_particle_density_upper_per_pixel: best.particle_density_upper,
            expected_illumination_energy_mj: best.energy_mj,
            confidence_multiplier: config.confidence_multiplier,
            clock_uncertainty_fraction_upper: best.clock_uncertainty_fraction_upper,
            post_measurement_observer_requalification_required: option.observer_impact
                != ObserverImpact::AssessedNonPerturbing,
            claim_boundary: "acquisition plan only; event completeness, calibrated velocity, observer passivity and physical validity require independent evidence receipts",
        }),
        rejected_option_counts: rejected,
        claim_boundary: "advisory event acquisition plan; no hardware actuation or physical EHD validation",
    })
}

fn abstain(
    reason: EventPlanAbstention,
    rejected: RejectedIlluminationCounts,
) -> EventAcquisitionReport {
    EventAcquisitionReport {
        disposition: EventPlanDisposition::Abstain,
        abstention_reason: Some(reason),
        plan: None,
        rejected_option_counts: rejected,
        claim_boundary: "no event acquisition plan is authorized; preserve unresolved evidence state",
    }
}

fn determine_abstention(rejected: &RejectedIlluminationCounts) -> EventPlanAbstention {
    if rejected.temporal_window_infeasible > 0 {
        EventPlanAbstention::TemporalWindowInfeasible
    } else if rejected.clock_resolution_infeasible > 0 {
        EventPlanAbstention::ClockResolutionInsufficient
    } else if rejected.event_buffer_infeasible > 0 && rejected.displacement_infeasible == 0 {
        EventPlanAbstention::EventBufferCapacityInsufficient
    } else if rejected.displacement_infeasible > 0 && rejected.event_buffer_infeasible == 0 {
        EventPlanAbstention::DisplacementBoundInfeasible
    } else if rejected.event_buffer_infeasible > 0 || rejected.displacement_infeasible > 0 {
        EventPlanAbstention::TemporalWindowInfeasible
    } else if rejected.event_rate_exceeded > 0 {
        EventPlanAbstention::EventRateExceedsAllValidatedModes
    } else if rejected.particle_density_exceeded > 0 {
        EventPlanAbstention::ParticleDensityExceedsAllValidatedModes
    } else if rejected.spatial_coverage_insufficient > 0 {
        EventPlanAbstention::SpatialCoverageUnavailable
    } else {
        EventPlanAbstention::NoAvailableCalibratedIlluminationMode
    }
}

fn candidate_better(
    candidate: &FeasibleCandidate,
    current: &FeasibleCandidate,
    options: &[IlluminationOption],
) -> bool {
    let left = &options[candidate.option_index];
    let right = &options[current.option_index];
    candidate
        .energy_mj
        .total_cmp(&current.energy_mj)
        .then_with(|| {
            // Prefer larger event-rate headroom when energy is equivalent.
            right
                .maximum_event_rate_per_second
                .total_cmp(&left.maximum_event_rate_per_second)
        })
        .then_with(|| left.option_id.cmp(&right.option_id))
        .is_lt()
}

fn validate_snapshot(snapshot: &EventRegionSnapshot) -> Result<(), EventPlanError> {
    for (field, value) in [
        ("stream_id", snapshot.stream_id.as_str()),
        ("camera_id", snapshot.camera_id.as_str()),
        ("clock_domain_id", snapshot.clock_domain_id.as_str()),
        (
            "protocol_fingerprint",
            snapshot.protocol_fingerprint.as_str(),
        ),
        ("observer_state_id", snapshot.observer_state_id.as_str()),
        ("region_id", snapshot.region_id.as_str()),
        ("acquisition_id", snapshot.acquisition_id.as_str()),
        (
            "dependence_cluster_id",
            snapshot.dependence_cluster_id.as_str(),
        ),
    ] {
        if value.trim().is_empty() || value.chars().any(char::is_control) {
            return Err(EventPlanError::EmptyIdentity { field });
        }
    }
    for (field, value) in [
        ("event_rate_per_second", snapshot.event_rate_per_second),
        (
            "event_rate_relative_standard_uncertainty",
            snapshot.event_rate_relative_standard_uncertainty,
        ),
        (
            "particle_density_per_pixel",
            snapshot.particle_density_per_pixel,
        ),
        (
            "particle_density_relative_standard_uncertainty",
            snapshot.particle_density_relative_standard_uncertainty,
        ),
        (
            "velocity_magnitude_pixels_per_second",
            snapshot.velocity_magnitude_pixels_per_second,
        ),
        (
            "velocity_standard_uncertainty_pixels_per_second",
            snapshot.velocity_standard_uncertainty_pixels_per_second,
        ),
        (
            "clock_standard_uncertainty_seconds",
            snapshot.clock_standard_uncertainty_seconds,
        ),
        (
            "maximum_displacement_pixels",
            snapshot.maximum_displacement_pixels,
        ),
    ] {
        if !value.is_finite() || value < 0.0 {
            return Err(EventPlanError::InvalidMetric { field });
        }
    }
    for (field, value) in [
        (
            "event_rate_relative_standard_uncertainty",
            snapshot.event_rate_relative_standard_uncertainty,
        ),
        (
            "particle_density_relative_standard_uncertainty",
            snapshot.particle_density_relative_standard_uncertainty,
        ),
    ] {
        if value >= 1.0 {
            return Err(EventPlanError::InvalidMetric { field });
        }
    }
    if snapshot.event_rate_per_second <= 0.0 {
        return Err(EventPlanError::InvalidMetric {
            field: "event_rate_per_second",
        });
    }
    if snapshot.buffer_capacity_events == 0 {
        return Err(EventPlanError::InvalidBufferCapacity);
    }
    if snapshot.minimum_events_required == 0
        || snapshot.minimum_events_required > snapshot.buffer_capacity_events
    {
        return Err(EventPlanError::InvalidEventRequirement);
    }
    if snapshot.maximum_displacement_pixels <= 0.0 {
        return Err(EventPlanError::InvalidMetric {
            field: "maximum_displacement_pixels",
        });
    }
    match snapshot.packet_loss_fraction {
        Some(value) if value.is_finite() && (0.0..=1.0).contains(&value) => {}
        Some(_) => {
            return Err(EventPlanError::InvalidMetric {
                field: "packet_loss_fraction",
            });
        }
        None => {}
    }
    Ok(())
}

fn validate_config(config: &EventWindowConfig) -> Result<(), EventPlanError> {
    for (field, value) in [
        ("confidence_multiplier", config.confidence_multiplier),
        ("minimum_window_seconds", config.minimum_window_seconds),
        ("maximum_window_seconds", config.maximum_window_seconds),
        (
            "maximum_buffer_utilization",
            config.maximum_buffer_utilization,
        ),
        (
            "maximum_clock_uncertainty_fraction",
            config.maximum_clock_uncertainty_fraction,
        ),
        (
            "maximum_packet_loss_fraction",
            config.maximum_packet_loss_fraction,
        ),
        (
            "required_spatial_coverage_fraction",
            config.required_spatial_coverage_fraction,
        ),
    ] {
        if !value.is_finite() || value <= 0.0 {
            return Err(EventPlanError::InvalidConfiguration { field });
        }
    }
    if config.minimum_window_seconds > config.maximum_window_seconds
        || config.maximum_buffer_utilization > 1.0
        || config.maximum_clock_uncertainty_fraction > 1.0
        || config.maximum_packet_loss_fraction > 1.0
        || config.required_spatial_coverage_fraction > 1.0
    {
        return Err(EventPlanError::InvalidConfiguration {
            field: "bounded_interval_or_fraction",
        });
    }
    Ok(())
}

fn validate_options(options: &[IlluminationOption]) -> Result<(), EventPlanError> {
    if options.len() > MAX_ILLUMINATION_OPTIONS {
        return Err(EventPlanError::TooManyOptions {
            actual: options.len(),
            maximum: MAX_ILLUMINATION_OPTIONS,
        });
    }
    let mut ids = BTreeSet::new();
    for (index, option) in options.iter().enumerate() {
        if option.option_id.trim().is_empty() || option.option_id.chars().any(char::is_control) {
            return Err(EventPlanError::EmptyOptionId { index });
        }
        if !ids.insert(option.option_id.as_str()) {
            return Err(EventPlanError::DuplicateOptionId(option.option_id.clone()));
        }
        for (field, value) in [
            (
                "maximum_event_rate_per_second",
                option.maximum_event_rate_per_second,
            ),
            (
                "maximum_particle_density_per_pixel",
                option.maximum_particle_density_per_pixel,
            ),
            (
                "spatial_coverage_fraction",
                option.spatial_coverage_fraction,
            ),
            (
                "energy_rate_mj_per_second",
                option.energy_rate_mj_per_second,
            ),
            ("maximum_window_seconds", option.maximum_window_seconds),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(EventPlanError::InvalidOptionLimit {
                    option_id: option.option_id.clone(),
                    field,
                });
            }
        }
        if option.maximum_event_rate_per_second <= 0.0
            || option.maximum_window_seconds <= 0.0
            || option.spatial_coverage_fraction > 1.0
            || option.energy_rate_mj_per_second <= 0.0
        {
            return Err(EventPlanError::InvalidOptionLimit {
                option_id: option.option_id.clone(),
                field: "positive_limits_and_unit_interval",
            });
        }
        if option.available && option.calibration_receipt_id.trim().is_empty() {
            return Err(EventPlanError::MissingCalibrationReceipt {
                option_id: option.option_id.clone(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> EventRegionSnapshot {
        EventRegionSnapshot {
            stream_id: "event-stream-001".to_owned(),
            camera_id: "event-camera-01".to_owned(),
            clock_domain_id: "camera-clock-01".to_owned(),
            protocol_fingerprint: "event-piv-protocol-v1".to_owned(),
            observer_state_id: "observer-state-01".to_owned(),
            region_id: "near-wall".to_owned(),
            acquisition_id: "acq-001".to_owned(),
            dependence_cluster_id: "cluster-001".to_owned(),
            event_rate_per_second: 100_000.0,
            event_rate_relative_standard_uncertainty: 0.05,
            particle_density_per_pixel: 0.02,
            particle_density_relative_standard_uncertainty: 0.05,
            velocity_magnitude_pixels_per_second: 400.0,
            velocity_standard_uncertainty_pixels_per_second: 20.0,
            buffer_capacity_events: 100_000,
            minimum_events_required: 500,
            maximum_displacement_pixels: 8.0,
            packet_loss_fraction: Some(0.0),
            timestamps_monotonic: true,
            clock_synchronization_qualified: true,
            clock_standard_uncertainty_seconds: 1.0e-6,
        }
    }

    fn config() -> EventWindowConfig {
        EventWindowConfig {
            confidence_multiplier: 3.0,
            minimum_window_seconds: 1.0e-4,
            maximum_window_seconds: 0.05,
            maximum_buffer_utilization: 0.8,
            maximum_clock_uncertainty_fraction: 0.1,
            maximum_packet_loss_fraction: 0.001,
            required_spatial_coverage_fraction: 0.95,
        }
    }

    fn option(
        id: &str,
        mode: SpatialIlluminationMode,
        max_rate: f64,
        max_density: f64,
        coverage: f64,
        energy_rate: f64,
        observer_impact: ObserverImpact,
    ) -> IlluminationOption {
        IlluminationOption {
            option_id: id.to_owned(),
            mode,
            maximum_event_rate_per_second: max_rate,
            maximum_particle_density_per_pixel: max_density,
            spatial_coverage_fraction: coverage,
            energy_rate_mj_per_second: energy_rate,
            maximum_window_seconds: 0.05,
            available: true,
            calibration_receipt_id: format!("cal-{id}"),
            observer_impact,
        }
    }

    fn catalog() -> Vec<IlluminationOption> {
        vec![
            option(
                "full-field",
                SpatialIlluminationMode::FullField,
                200_000.0,
                0.03,
                1.0,
                1.0,
                ObserverImpact::AssessedNonPerturbing,
            ),
            option(
                "block-scan",
                SpatialIlluminationMode::BlockScanned,
                400_000.0,
                0.06,
                0.98,
                2.0,
                ObserverImpact::AssessedNonPerturbing,
            ),
            option(
                "line-scan",
                SpatialIlluminationMode::LineScanned,
                800_000.0,
                0.12,
                0.96,
                4.0,
                ObserverImpact::PotentiallyPerturbing,
            ),
        ]
    }

    #[test]
    fn benign_region_selects_low_energy_calibrated_mode() {
        let report = plan_event_acquisition(&snapshot(), &config(), &catalog()).unwrap();
        assert_eq!(
            report.disposition,
            EventPlanDisposition::AcquisitionPlanAvailable
        );
        let plan = report.plan.unwrap();
        assert_eq!(plan.illumination_option_id, "full-field");
        assert!(plan.accumulation_window_seconds >= plan.minimum_feasible_window_seconds);
        assert!(plan.accumulation_window_seconds <= plan.maximum_feasible_window_seconds);
        assert!(plan.expected_buffer_utilization_upper <= config().maximum_buffer_utilization);
    }

    #[test]
    fn density_shift_escalates_to_calibrated_scanned_illumination() {
        let mut sample = snapshot();
        sample.particle_density_per_pixel = 0.08;
        let report = plan_event_acquisition(&sample, &config(), &catalog()).unwrap();
        let plan = report.plan.unwrap();
        assert_eq!(plan.illumination_mode, SpatialIlluminationMode::LineScanned);
        assert!(plan.post_measurement_observer_requalification_required);
    }

    #[test]
    fn higher_local_velocity_shortens_the_accumulation_window() {
        let mut fast = snapshot();
        fast.velocity_magnitude_pixels_per_second = 1_000.0;
        let slow_plan = plan_event_acquisition(&snapshot(), &config(), &catalog())
            .unwrap()
            .plan
            .unwrap();
        let fast_plan = plan_event_acquisition(&fast, &config(), &catalog())
            .unwrap()
            .plan
            .unwrap();
        assert!(fast_plan.accumulation_window_seconds < slow_plan.accumulation_window_seconds);
    }

    #[test]
    fn higher_event_rate_shortens_the_buffer_limited_window() {
        let mut busy = snapshot();
        busy.event_rate_per_second = 350_000.0;
        let baseline = plan_event_acquisition(&snapshot(), &config(), &catalog())
            .unwrap()
            .plan
            .unwrap();
        let busy_plan = plan_event_acquisition(&busy, &config(), &catalog())
            .unwrap()
            .plan
            .unwrap();
        assert!(busy_plan.accumulation_window_seconds < baseline.accumulation_window_seconds);
    }

    #[test]
    fn density_above_all_calibrated_modes_abstains() {
        let mut sample = snapshot();
        sample.particle_density_per_pixel = 0.5;
        let report = plan_event_acquisition(&sample, &config(), &catalog()).unwrap();
        assert_eq!(report.disposition, EventPlanDisposition::Abstain);
        assert_eq!(
            report.abstention_reason,
            Some(EventPlanAbstention::ParticleDensityExceedsAllValidatedModes)
        );
    }

    #[test]
    fn insufficient_event_information_abstains_instead_of_widening_window() {
        let mut sample = snapshot();
        sample.event_rate_relative_standard_uncertainty = 0.99;
        let report = plan_event_acquisition(&sample, &config(), &catalog()).unwrap();
        assert_eq!(report.disposition, EventPlanDisposition::Abstain);
        assert_eq!(
            report.abstention_reason,
            Some(EventPlanAbstention::InsufficientEventInformation)
        );
    }

    #[test]
    fn timestamp_order_failure_abstains() {
        let mut sample = snapshot();
        sample.timestamps_monotonic = false;
        let report = plan_event_acquisition(&sample, &config(), &catalog()).unwrap();
        assert_eq!(
            report.abstention_reason,
            Some(EventPlanAbstention::TimestampOrderInvalid)
        );
    }

    #[test]
    fn unknown_packet_loss_abstains() {
        let mut sample = snapshot();
        sample.packet_loss_fraction = None;
        let report = plan_event_acquisition(&sample, &config(), &catalog()).unwrap();
        assert_eq!(
            report.abstention_reason,
            Some(EventPlanAbstention::PacketLossUnknown)
        );
    }

    #[test]
    fn packet_loss_above_gate_abstains() {
        let mut sample = snapshot();
        sample.packet_loss_fraction = Some(0.02);
        let report = plan_event_acquisition(&sample, &config(), &catalog()).unwrap();
        assert_eq!(
            report.abstention_reason,
            Some(EventPlanAbstention::PacketLossExceeded)
        );
    }

    #[test]
    fn clock_synchronization_failure_abstains() {
        let mut sample = snapshot();
        sample.clock_synchronization_qualified = false;
        let report = plan_event_acquisition(&sample, &config(), &catalog()).unwrap();
        assert_eq!(
            report.abstention_reason,
            Some(EventPlanAbstention::ClockSynchronizationUnqualified)
        );
    }

    #[test]
    fn no_mode_with_sufficient_temporal_bandwidth_abstains() {
        let mut options = catalog();
        for option in &mut options {
            option.maximum_event_rate_per_second = 10.0;
        }
        let report = plan_event_acquisition(&snapshot(), &config(), &options).unwrap();
        assert_eq!(
            report.abstention_reason,
            Some(EventPlanAbstention::EventRateExceedsAllValidatedModes)
        );
    }

    #[test]
    fn illumination_catalog_permutation_preserves_selection() {
        let mut options = catalog();
        let baseline = plan_event_acquisition(&snapshot(), &config(), &options)
            .unwrap()
            .plan
            .unwrap();
        options.reverse();
        let permuted = plan_event_acquisition(&snapshot(), &config(), &options)
            .unwrap()
            .plan
            .unwrap();
        assert_eq!(
            baseline.illumination_option_id,
            permuted.illumination_option_id
        );
        assert_eq!(
            baseline.accumulation_window_seconds,
            permuted.accumulation_window_seconds
        );
        assert_eq!(
            baseline.expected_illumination_energy_mj,
            permuted.expected_illumination_energy_mj
        );
    }

    #[test]
    fn absent_calibration_receipt_is_rejected() {
        let mut options = catalog();
        options[0].calibration_receipt_id.clear();
        assert!(matches!(
            plan_event_acquisition(&snapshot(), &config(), &options),
            Err(EventPlanError::MissingCalibrationReceipt { .. })
        ));
    }

    #[test]
    fn inadequate_spatial_coverage_abstains() {
        let mut options = catalog();
        for option in &mut options {
            option.spatial_coverage_fraction = 0.5;
        }
        let report = plan_event_acquisition(&snapshot(), &config(), &options).unwrap();
        assert_eq!(
            report.abstention_reason,
            Some(EventPlanAbstention::SpatialCoverageUnavailable)
        );
    }

    #[test]
    fn infeasible_minimum_information_vs_buffer_limit_abstains() {
        let mut sample = snapshot();
        sample.minimum_events_required = 50_000;
        sample.buffer_capacity_events = 60_000;
        sample.velocity_magnitude_pixels_per_second = 0.0;
        sample.velocity_standard_uncertainty_pixels_per_second = 0.0;
        let report = plan_event_acquisition(&sample, &config(), &catalog()).unwrap();
        assert_eq!(
            report.abstention_reason,
            Some(EventPlanAbstention::TemporalWindowInfeasible)
        );
    }

    #[test]
    fn displacement_bound_can_make_minimum_information_infeasible() {
        let mut sample = snapshot();
        sample.minimum_events_required = 10_000;
        sample.velocity_magnitude_pixels_per_second = 100_000.0;
        sample.maximum_displacement_pixels = 1.0;
        let report = plan_event_acquisition(&sample, &config(), &catalog()).unwrap();
        assert_eq!(
            report.abstention_reason,
            Some(EventPlanAbstention::TemporalWindowInfeasible)
        );
    }

    #[test]
    fn packet_loss_is_included_in_minimum_retained_event_count() {
        let mut sample = snapshot();
        sample.packet_loss_fraction = Some(0.10);
        let mut config = config();
        config.maximum_packet_loss_fraction = 0.20;
        let report = plan_event_acquisition(&sample, &config, &catalog()).unwrap();
        let plan = report.plan.unwrap();
        assert!(plan.expected_retained_event_count_lower >= sample.minimum_events_required as f64);
    }

    #[test]
    fn event_buffer_constraint_has_a_specific_abstention_reason() {
        let mut sample = snapshot();
        sample.buffer_capacity_events = 1_000;
        sample.minimum_events_required = 800;
        let report = plan_event_acquisition(&sample, &config(), &catalog()).unwrap();
        assert_eq!(
            report.abstention_reason,
            Some(EventPlanAbstention::EventBufferCapacityInsufficient)
        );
    }

    #[test]
    fn displacement_constraint_has_a_specific_abstention_reason() {
        let mut sample = snapshot();
        sample.velocity_magnitude_pixels_per_second = 100_000.0;
        sample.velocity_standard_uncertainty_pixels_per_second = 0.0;
        sample.maximum_displacement_pixels = 1.0;
        let report = plan_event_acquisition(&sample, &config(), &catalog()).unwrap();
        assert_eq!(
            report.abstention_reason,
            Some(EventPlanAbstention::DisplacementBoundInfeasible)
        );
    }

    #[test]
    fn insufficient_clock_precision_abstains_with_specific_reason() {
        let mut sample = snapshot();
        sample.clock_standard_uncertainty_seconds = 1.0e-3;
        let mut config = config();
        config.maximum_clock_uncertainty_fraction = 1.0e-5;
        let report = plan_event_acquisition(&sample, &config, &catalog()).unwrap();
        assert_eq!(
            report.abstention_reason,
            Some(EventPlanAbstention::ClockResolutionInsufficient)
        );
    }

    #[test]
    fn event_window_is_feasible_under_confidence_bounds() {
        let report = plan_event_acquisition(&snapshot(), &config(), &catalog()).unwrap();
        let plan = report.plan.unwrap();
        assert!(
            plan.expected_retained_event_count_lower
                >= snapshot().minimum_events_required as f64
                    * (1.0 - config().maximum_packet_loss_fraction)
        );
        assert!(plan.expected_buffer_utilization_upper <= config().maximum_buffer_utilization);
        assert!(plan.expected_displacement_upper_pixels <= snapshot().maximum_displacement_pixels);
        assert!(
            plan.clock_uncertainty_fraction_upper <= config().maximum_clock_uncertainty_fraction
        );
    }

    #[test]
    fn invalid_relative_rate_uncertainty_is_rejected() {
        let mut sample = snapshot();
        sample.event_rate_relative_standard_uncertainty = 1.0;
        assert!(matches!(
            plan_event_acquisition(&sample, &config(), &catalog()),
            Err(EventPlanError::InvalidMetric { .. })
        ));
    }

    #[test]
    fn unknown_observer_impact_requires_post_measurement_requalification() {
        let mut options = catalog();
        options[0].observer_impact = ObserverImpact::Unknown;
        let report = plan_event_acquisition(&snapshot(), &config(), &options).unwrap();
        assert!(
            report
                .plan
                .unwrap()
                .post_measurement_observer_requalification_required
        );
    }
}
