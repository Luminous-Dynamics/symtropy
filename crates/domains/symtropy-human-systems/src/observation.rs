// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Deterministic synthetic observation-process fixtures for human-systems research.
//!
//! This module models arbitrary-unit mathematical signals and the process by
//! which synthetic sensors sample, corrupt, delay, lose, or duplicate them.
//! It does not model human physiology, estimate a person's state, or produce
//! human-valid sensor parameters.
//!
//! The estimator-facing view has no sample ordinal, physical timestamp, latent
//! state, or generator seed. Scorer-only truth and event links are returned in
//! a different view. These Rust types are an API boundary, not process isolation.

use std::f64::consts::TAU;

/// Number of canonical synthetic scenarios in v0.1.
pub const SCENARIO_COUNT: usize = 12;
/// Duration of each canonical fixture, in synthetic seconds (six hours).
pub const DEFAULT_DURATION_SECONDS: u64 = 21_600;
/// Nominal capture cadence for the first fixture set, in synthetic seconds.
pub const DEFAULT_CADENCE_SECONDS: u64 = 60;

/// Stable identifiers for the v0.1 observation-process benchmark.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScenarioId {
    Clean,
    IrregularCadence,
    McarDropout,
    MarActivityArtifact,
    MnarStateDependentDropout,
    BiasDrift,
    QuantizationAndClipping,
    ClockOffsetAndDrift,
    DelayedAvailability,
    PacketFaults,
    CommonCauseFailure,
    IndistinguishableWorlds,
}

impl ScenarioId {
    /// All scenarios in canonical, stable order.
    pub const ALL: [Self; SCENARIO_COUNT] = [
        Self::Clean,
        Self::IrregularCadence,
        Self::McarDropout,
        Self::MarActivityArtifact,
        Self::MnarStateDependentDropout,
        Self::BiasDrift,
        Self::QuantizationAndClipping,
        Self::ClockOffsetAndDrift,
        Self::DelayedAvailability,
        Self::PacketFaults,
        Self::CommonCauseFailure,
        Self::IndistinguishableWorlds,
    ];

    /// Stable machine-readable scenario identifier.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Clean => "HOBS-00",
            Self::IrregularCadence => "HOBS-01",
            Self::McarDropout => "HOBS-02",
            Self::MarActivityArtifact => "HOBS-03",
            Self::MnarStateDependentDropout => "HOBS-04",
            Self::BiasDrift => "HOBS-05",
            Self::QuantizationAndClipping => "HOBS-06",
            Self::ClockOffsetAndDrift => "HOBS-07",
            Self::DelayedAvailability => "HOBS-08",
            Self::PacketFaults => "HOBS-09",
            Self::CommonCauseFailure => "HOBS-10",
            Self::IndistinguishableWorlds => "HOBS-11",
        }
    }
}

/// Semantic identity of a synthetic observation channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationQuantity {
    /// The main arbitrary-unit latent signal.
    PrimarySignal,
    /// An observed auxiliary activity proxy used by the MAR fixture.
    ActivityProxy,
    /// A composite measurement that observes only the sum of two latent channels.
    CompositeSignal,
}

/// Reason a capture exists but contains no usable value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissingReason {
    /// Missing completely at random under this synthetic generator.
    Mcar,
    /// Missingness depends on the observed activity proxy.
    MarActivityLinked,
    /// Missingness depends on the hidden latent target value.
    MnarLatentState,
    /// A scheduled, exogenous sensor outage, independent of signal value.
    IndependentSensorOutage,
    /// Shared outage affects sensors in the same common-cause group.
    CommonCauseOutage,
}

/// Disposition of a generated sensor reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationStatus {
    /// A value was delivered without clipping or quantisation.
    Observed,
    /// A capture occurred but the sensor produced no usable value.
    Missing(MissingReason),
    /// A delivered value was quantised.
    Quantized,
    /// The underlying sensor response was below the lower measurement limit.
    ClippedLow,
    /// The underlying sensor response was above the upper measurement limit.
    ClippedHigh,
}

/// One delivered observation packet.
///
/// The estimator-facing event intentionally omits the latent physical capture
/// time and sample ordinal. The raw sensor timestamp is expressed in
/// clock_domain; available_at_seconds is expressed in the receive-time domain.
/// source_event_id is an opaque deterministic identity, not a time encoding.
#[derive(Debug, Clone, PartialEq)]
pub struct ObservationEvent {
    /// Opaque identity of the original sensor capture; duplicates retain this ID.
    pub source_event_id: u64,
    /// Unique identity for each packet delivery, including duplicates.
    pub delivery_id: u64,
    /// Synthetic sensor identifier.
    pub sensor_id: u16,
    /// Channel semantics.
    pub quantity: ObservationQuantity,
    /// Raw sensor-clock timestamp, in the declared clock_domain.
    pub sensor_timestamp_seconds: f64,
    /// Time at which this packet becomes available to the estimator.
    pub available_at_seconds: f64,
    /// Clock-domain identity for the raw sensor timestamp.
    pub clock_domain: u16,
    /// Missing readings have None; missing sentinels are never numeric values.
    pub value: Option<f64>,
    /// Whether the value is ordinary, missing, quantised or saturated.
    pub status: ObservationStatus,
    /// True only for a repeated transport delivery of the same source event.
    pub is_duplicate_delivery: bool,
}

/// Scorer-only latent state at one physical event time.
///
/// Values are arbitrary-unit mathematical fixture values, not human
/// physiological variables, measurements, clinical observations or estimates.
#[derive(Debug, Clone, PartialEq)]
pub struct LatentTruthPoint {
    /// Underlying scheduled capture index; scorer-only.
    pub sample_index: u64,
    /// Exact physical event time in synthetic seconds; scorer-only.
    pub event_time_seconds: f64,
    /// First latent channel.
    pub latent_primary: f64,
    /// Second latent channel (activity proxy or second indistinguishable state).
    pub latent_secondary: f64,
}

/// Scorer-only link from an opaque packet identity to its physical truth row.
#[derive(Debug, Clone, PartialEq)]
pub struct ScorerEventTruth {
    /// Opaque source event identity, matching estimator-facing packets.
    pub source_event_id: u64,
    /// Underlying sample index; never included in ObservationEvent.
    pub sample_index: u64,
    /// Physical event time in synthetic seconds.
    pub event_time_seconds: f64,
    /// First latent channel at capture.
    pub latent_primary: f64,
    /// Second latent channel at capture.
    pub latent_secondary: f64,
}

/// Immutable description of one generator invocation; scorer-only.
#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioManifest {
    /// Stable scenario ID.
    pub scenario_id: ScenarioId,
    /// Deterministic generator seed.
    pub seed: u64,
    /// Duration of the synthetic world.
    pub duration_seconds: u64,
    /// Nominal capture interval.
    pub nominal_cadence_seconds: u64,
    /// Explicit evidence/claim ceiling.
    pub claim_ceiling: &'static str,
}

/// Estimator-facing input. It does not expose the PRNG seed or latent truth.
#[derive(Debug, Clone, PartialEq)]
pub struct EstimatorView {
    /// Scenario identity for stratified evaluation.
    pub scenario_id: ScenarioId,
    /// Delivered event stream, sorted by availability and delivery ID.
    pub observations: Vec<ObservationEvent>,
}

/// Scorer-only truth and mapping. Keep it out of the estimator process.
#[derive(Debug, Clone, PartialEq)]
pub struct ScorerView {
    /// Scenario generation metadata and seed.
    pub manifest: ScenarioManifest,
    /// Main latent truth track.
    pub truth: Vec<LatentTruthPoint>,
    /// Mapping from opaque source event IDs to latent truth rows.
    pub event_truth: Vec<ScorerEventTruth>,
    /// Alternate latent truth for HOBS-11.
    ///
    /// The same composite observation stream is compatible with either world.
    pub alternate_truth: Option<Vec<LatentTruthPoint>>,
}

/// Output with explicit estimator/scorer views.
///
/// The caller must pass only estimator_view to a candidate estimator. The two
/// views must be separated into distinct processes for a security boundary.
#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioRun {
    /// Estimator-facing observations.
    pub estimator_view: EstimatorView,
    /// Scorer-only latent truth and identity mapping.
    pub scorer_view: ScorerView,
}

/// Build one canonical scenario from a stable ID and seed.
///
/// The generator uses no external dependencies. Fixed (scenario_id, seed)
/// inputs produce the same ordered event and truth structures within the same
/// floating-point execution environment.
pub fn generate_scenario(scenario_id: ScenarioId, seed: u64) -> ScenarioRun {
    let mut random = SplitMix64::new(seed);
    let mut identity_random = SplitMix64::new(seed ^ 0xA11C_E5E5_D15C_A11E);
    let mut observations = Vec::with_capacity(400);
    let mut truth = Vec::with_capacity(361);
    let mut event_truth = Vec::with_capacity(400);
    let mut alternate_truth = if scenario_id == ScenarioId::IndistinguishableWorlds {
        Some(Vec::with_capacity(361))
    } else {
        None
    };
    let mut next_delivery_id = 0_u64;

    for sample_index in 0..=(DEFAULT_DURATION_SECONDS / DEFAULT_CADENCE_SECONDS) {
        let jitter = if scenario_id == ScenarioId::IrregularCadence && sample_index > 0 {
            random.symmetric(10.0)
        } else {
            0.0
        };
        let physical_time = sample_index as f64 * DEFAULT_CADENCE_SECONDS as f64 + jitter;
        let (latent_primary, latent_secondary, measured_signal) =
            if scenario_id == ScenarioId::IndistinguishableWorlds {
                // World A = (1, 0); World B = (0, 1). Both produce sum = 1.
                (1.0, 0.0, 1.0)
            } else {
                let primary = primary_signal(physical_time);
                let activity = activity_proxy(physical_time);
                (primary, activity, primary)
            };

        truth.push(LatentTruthPoint {
            sample_index,
            event_time_seconds: physical_time,
            latent_primary,
            latent_secondary,
        });

        if let Some(alternate) = alternate_truth.as_mut() {
            alternate.push(LatentTruthPoint {
                sample_index,
                event_time_seconds: physical_time,
                latent_primary: 0.0,
                latent_secondary: 1.0,
            });
        }

        let sensor_timestamp = match scenario_id {
            ScenarioId::ClockOffsetAndDrift => physical_time * (1.0 + 200.0e-6) + 12.0,
            _ => physical_time,
        };
        let clock_domain = if scenario_id == ScenarioId::ClockOffsetAndDrift {
            7
        } else {
            0
        };
        let primary_source_id = identity_random.next_u64();
        let secondary_source_id = identity_random.next_u64();
        let event_start = observations.len();

        match scenario_id {
            ScenarioId::Clean | ScenarioId::IrregularCadence => {
                push_event(
                    &mut observations,
                    &mut next_delivery_id,
                    primary_source_id,
                    0,
                    ObservationQuantity::PrimarySignal,
                    sensor_timestamp,
                    physical_time,
                    clock_domain,
                    Some(measured_signal),
                    ObservationStatus::Observed,
                    false,
                );
            }
            ScenarioId::McarDropout => {
                // The iid MCAR process and the independent scheduled outage
                // are sampled separately and retain separate reason labels.
                let iid_missing = random.unit() < 0.10;
                let in_outage = (7_200.0..=7_800.0).contains(&physical_time);
                let (value, status) = if in_outage {
                    (
                        None,
                        ObservationStatus::Missing(MissingReason::IndependentSensorOutage),
                    )
                } else if iid_missing {
                    (None, ObservationStatus::Missing(MissingReason::Mcar))
                } else {
                    (Some(measured_signal), ObservationStatus::Observed)
                };
                push_event(
                    &mut observations,
                    &mut next_delivery_id,
                    primary_source_id,
                    0,
                    ObservationQuantity::PrimarySignal,
                    sensor_timestamp,
                    physical_time,
                    clock_domain,
                    value,
                    status,
                    false,
                );
            }
            ScenarioId::MarActivityArtifact => {
                let activity = latent_secondary;
                push_event(
                    &mut observations,
                    &mut next_delivery_id,
                    secondary_source_id,
                    1,
                    ObservationQuantity::ActivityProxy,
                    sensor_timestamp,
                    physical_time,
                    clock_domain,
                    Some(activity),
                    ObservationStatus::Observed,
                    false,
                );
                let missing = activity > 0.7 && random.unit() < 0.35;
                let (value, status) = if missing {
                    (
                        None,
                        ObservationStatus::Missing(MissingReason::MarActivityLinked),
                    )
                } else {
                    let noise_bound = if activity > 0.7 { 0.24 } else { 0.04 };
                    (
                        Some(measured_signal + random.symmetric(noise_bound)),
                        ObservationStatus::Observed,
                    )
                };
                push_event(
                    &mut observations,
                    &mut next_delivery_id,
                    primary_source_id,
                    0,
                    ObservationQuantity::PrimarySignal,
                    sensor_timestamp,
                    physical_time,
                    clock_domain,
                    value,
                    status,
                    false,
                );
            }
            ScenarioId::MnarStateDependentDropout => {
                let missing = measured_signal > 0.65;
                let (value, status) = if missing {
                    (
                        None,
                        ObservationStatus::Missing(MissingReason::MnarLatentState),
                    )
                } else {
                    (Some(measured_signal), ObservationStatus::Observed)
                };
                push_event(
                    &mut observations,
                    &mut next_delivery_id,
                    primary_source_id,
                    0,
                    ObservationQuantity::PrimarySignal,
                    sensor_timestamp,
                    physical_time,
                    clock_domain,
                    value,
                    status,
                    false,
                );
            }
            ScenarioId::BiasDrift => {
                let elapsed_hours = physical_time / 3_600.0;
                let value = measured_signal + 0.02 * elapsed_hours + random.symmetric(0.01);
                push_event(
                    &mut observations,
                    &mut next_delivery_id,
                    primary_source_id,
                    0,
                    ObservationQuantity::PrimarySignal,
                    sensor_timestamp,
                    physical_time,
                    clock_domain,
                    Some(value),
                    ObservationStatus::Observed,
                    false,
                );
            }
            ScenarioId::QuantizationAndClipping => {
                let (bounded, clipped_status) = if measured_signal < 0.3 {
                    (0.3, Some(ObservationStatus::ClippedLow))
                } else if measured_signal > 0.7 {
                    (0.7, Some(ObservationStatus::ClippedHigh))
                } else {
                    (measured_signal, None)
                };
                let quantized = (bounded / 0.1).round() * 0.1;
                let status = clipped_status.unwrap_or_else(|| {
                    if (quantized - bounded).abs() > f64::EPSILON {
                        ObservationStatus::Quantized
                    } else {
                        ObservationStatus::Observed
                    }
                });
                push_event(
                    &mut observations,
                    &mut next_delivery_id,
                    primary_source_id,
                    0,
                    ObservationQuantity::PrimarySignal,
                    sensor_timestamp,
                    physical_time,
                    clock_domain,
                    Some(quantized),
                    status,
                    false,
                );
            }
            ScenarioId::ClockOffsetAndDrift => {
                push_event(
                    &mut observations,
                    &mut next_delivery_id,
                    primary_source_id,
                    0,
                    ObservationQuantity::PrimarySignal,
                    sensor_timestamp,
                    physical_time,
                    clock_domain,
                    Some(measured_signal),
                    ObservationStatus::Observed,
                    false,
                );
            }
            ScenarioId::DelayedAvailability => {
                let latency = 600.0 + random.symmetric(30.0);
                push_event(
                    &mut observations,
                    &mut next_delivery_id,
                    primary_source_id,
                    0,
                    ObservationQuantity::PrimarySignal,
                    sensor_timestamp,
                    physical_time + latency,
                    clock_domain,
                    Some(measured_signal),
                    ObservationStatus::Observed,
                    false,
                );
            }
            ScenarioId::PacketFaults => {
                // Force a reorder for the first pair of each ten-sample group;
                // this is a deterministic semantic control, not a lucky draw.
                let within_group = sample_index % 10;
                if within_group > 1 && random.unit() < 0.03 {
                    continue;
                }
                let latency = match within_group {
                    0 => 125.0,
                    1 => 0.0,
                    _ => 5.0 + random.unit() * 20.0,
                };
                push_event(
                    &mut observations,
                    &mut next_delivery_id,
                    primary_source_id,
                    0,
                    ObservationQuantity::PrimarySignal,
                    sensor_timestamp,
                    physical_time + latency,
                    clock_domain,
                    Some(measured_signal),
                    ObservationStatus::Observed,
                    false,
                );
                if random.unit() < 0.05 {
                    push_event(
                        &mut observations,
                        &mut next_delivery_id,
                        primary_source_id,
                        0,
                        ObservationQuantity::PrimarySignal,
                        sensor_timestamp,
                        physical_time + latency + 1.0,
                        clock_domain,
                        Some(measured_signal),
                        ObservationStatus::Observed,
                        true,
                    );
                }
            }
            ScenarioId::CommonCauseFailure => {
                let common_outage = (10_800.0..=12_600.0).contains(&physical_time);
                for sensor_id in 0..2_u16 {
                    let value = if common_outage {
                        None
                    } else {
                        let gain = if sensor_id == 1 { 1.05 } else { 1.0 };
                        Some(measured_signal * gain)
                    };
                    let status = if common_outage {
                        ObservationStatus::Missing(MissingReason::CommonCauseOutage)
                    } else {
                        ObservationStatus::Observed
                    };
                    let source_id = if sensor_id == 0 {
                        primary_source_id
                    } else {
                        secondary_source_id
                    };
                    push_event(
                        &mut observations,
                        &mut next_delivery_id,
                        source_id,
                        sensor_id,
                        ObservationQuantity::PrimarySignal,
                        sensor_timestamp,
                        physical_time,
                        clock_domain,
                        value,
                        status,
                        false,
                    );
                }
            }
            ScenarioId::IndistinguishableWorlds => {
                push_event(
                    &mut observations,
                    &mut next_delivery_id,
                    primary_source_id,
                    0,
                    ObservationQuantity::CompositeSignal,
                    sensor_timestamp,
                    physical_time,
                    clock_domain,
                    Some(latent_primary + latent_secondary),
                    ObservationStatus::Observed,
                    false,
                );
            }
        }

        // Build the scorer-only identity map after all deliveries for this
        // capture have been emitted. Duplicates share one source-event link.
        for event in &observations[event_start..] {
            if !event_truth
                .iter()
                .any(|link: &ScorerEventTruth| link.source_event_id == event.source_event_id)
            {
                event_truth.push(ScorerEventTruth {
                    source_event_id: event.source_event_id,
                    sample_index,
                    event_time_seconds: physical_time,
                    latent_primary,
                    latent_secondary,
                });
            }
        }
    }

    // Estimators receive packets in availability order, not capture order.
    observations.sort_by(|left, right| {
        left.available_at_seconds
            .total_cmp(&right.available_at_seconds)
            .then_with(|| left.delivery_id.cmp(&right.delivery_id))
    });

    ScenarioRun {
        estimator_view: EstimatorView {
            scenario_id,
            observations,
        },
        scorer_view: ScorerView {
            manifest: ScenarioManifest {
                scenario_id,
                seed,
                duration_seconds: DEFAULT_DURATION_SECONDS,
                nominal_cadence_seconds: DEFAULT_CADENCE_SECONDS,
                claim_ceiling: "Synthetic arbitrary-unit fixture only; no human physiological or clinical validity.",
            },
            truth,
            event_truth,
            alternate_truth,
        },
    }
}

fn primary_signal(time_seconds: f64) -> f64 {
    0.5 + 0.25 * (TAU * time_seconds / 7_200.0).sin()
}

fn activity_proxy(time_seconds: f64) -> f64 {
    0.5 + 0.5 * (TAU * time_seconds / 1_800.0).sin()
}

#[allow(clippy::too_many_arguments)]
fn push_event(
    events: &mut Vec<ObservationEvent>,
    next_delivery_id: &mut u64,
    source_event_id: u64,
    sensor_id: u16,
    quantity: ObservationQuantity,
    sensor_timestamp_seconds: f64,
    available_at_seconds: f64,
    clock_domain: u16,
    value: Option<f64>,
    status: ObservationStatus,
    is_duplicate_delivery: bool,
) {
    events.push(ObservationEvent {
        source_event_id,
        delivery_id: *next_delivery_id,
        sensor_id,
        quantity,
        sensor_timestamp_seconds,
        available_at_seconds,
        clock_domain,
        value,
        status,
        is_duplicate_delivery,
    });
    *next_delivery_id += 1;
}

/// Deterministic PRNG for reproducible synthetic fixtures.
///
/// SplitMix64 is not cryptographic and must never be used for security/privacy.
#[derive(Debug, Clone, Copy)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    fn unit(&mut self) -> f64 {
        let mantissa = self.next_u64() >> 11;
        mantissa as f64 * (1.0 / ((1_u64 << 53) as f64))
    }

    fn symmetric(&mut self, maximum_absolute_value: f64) -> f64 {
        (2.0 * self.unit() - 1.0) * maximum_absolute_value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(scenario_id: ScenarioId) -> ScenarioRun {
        generate_scenario(scenario_id, 0x5EED_2026)
    }

    fn truth_link<'a>(scenario: &'a ScenarioRun, event: &ObservationEvent) -> &'a ScorerEventTruth {
        scenario
            .scorer_view
            .event_truth
            .iter()
            .find(|link| link.source_event_id == event.source_event_id)
            .expect("every delivered source event has scorer truth")
    }

    fn missing_count(scenario: &ScenarioRun, reason: MissingReason) -> usize {
        scenario
            .estimator_view
            .observations
            .iter()
            .filter(|event| event.status == ObservationStatus::Missing(reason))
            .count()
    }

    #[test]
    fn canonical_scenario_ids_are_unique_and_stable() {
        let ids = ScenarioId::ALL.map(ScenarioId::as_str);
        let unique: std::collections::HashSet<_> = ids.iter().copied().collect();
        assert_eq!(ids.len(), SCENARIO_COUNT);
        assert_eq!(unique.len(), SCENARIO_COUNT);
        assert_eq!(ids[0], "HOBS-00");
        assert_eq!(ids[11], "HOBS-11");
    }

    #[test]
    fn fixed_scenario_and_seed_replay_identically() {
        for scenario_id in ScenarioId::ALL {
            assert_eq!(
                generate_scenario(scenario_id, 42),
                generate_scenario(scenario_id, 42)
            );
        }
    }

    #[test]
    fn estimator_view_does_not_expose_truth_seed_or_sample_ordinal() {
        let scenario = run(ScenarioId::ClockOffsetAndDrift);
        assert_eq!(
            scenario.estimator_view.scenario_id,
            ScenarioId::ClockOffsetAndDrift
        );
        assert_eq!(scenario.scorer_view.manifest.seed, 0x5EED_2026);
        assert!(scenario.scorer_view.truth.len() > 1);
        // ObservationEvent has no sample_index, physical timestamp, or seed field.
        for event in &scenario.estimator_view.observations {
            assert_ne!(event.source_event_id, 0);
        }
    }

    #[test]
    fn clean_scenario_has_one_complete_regular_event_per_capture() {
        let scenario = run(ScenarioId::Clean);
        assert_eq!(scenario.scorer_view.truth.len(), 361);
        assert_eq!(scenario.estimator_view.observations.len(), 361);
        assert!(scenario.estimator_view.observations.iter().all(|event| {
            event.status == ObservationStatus::Observed && event.value.is_some_and(f64::is_finite)
        }));
        assert!(
            scenario
                .estimator_view
                .observations
                .windows(2)
                .all(|pair| { pair[0].available_at_seconds <= pair[1].available_at_seconds })
        );
    }

    #[test]
    fn irregular_capture_schedule_is_strictly_ordered_but_not_uniform() {
        let scenario = run(ScenarioId::IrregularCadence);
        let times: Vec<_> = scenario
            .scorer_view
            .truth
            .iter()
            .map(|point| point.event_time_seconds)
            .collect();
        assert!(times.windows(2).all(|pair| pair[1] > pair[0]));
        assert!(
            times
                .windows(2)
                .any(|pair| { ((pair[1] - pair[0]) - DEFAULT_CADENCE_SECONDS as f64).abs() > 1.0 })
        );
    }

    #[test]
    fn mcar_iid_missingness_is_separate_from_exogenous_sensor_outage() {
        let scenario = run(ScenarioId::McarDropout);
        assert!(missing_count(&scenario, MissingReason::Mcar) > 0);
        assert!(missing_count(&scenario, MissingReason::IndependentSensorOutage) > 0);
        for event in &scenario.estimator_view.observations {
            if matches!(event.status, ObservationStatus::Missing(_)) {
                assert!(event.value.is_none());
            }
        }
        for link in &scenario.scorer_view.event_truth {
            let event = scenario
                .estimator_view
                .observations
                .iter()
                .find(|event| event.source_event_id == link.source_event_id)
                .unwrap();
            if event.status == ObservationStatus::Missing(MissingReason::IndependentSensorOutage) {
                assert!((7_200.0..=7_800.0).contains(&link.event_time_seconds));
            }
        }
    }

    #[test]
    fn mnar_missingness_only_hides_values_above_the_declared_threshold() {
        let scenario = run(ScenarioId::MnarStateDependentDropout);
        for event in &scenario.estimator_view.observations {
            let link = truth_link(&scenario, event);
            match event.status {
                ObservationStatus::Missing(MissingReason::MnarLatentState) => {
                    assert!(link.latent_primary > 0.65);
                    assert!(event.value.is_none());
                }
                ObservationStatus::Observed => {
                    assert!(link.latent_primary <= 0.65);
                    assert_eq!(event.value, Some(link.latent_primary));
                }
                other => panic!("unexpected HOBS-04 status: {other:?}"),
            }
        }
    }

    #[test]
    fn mar_scenario_exposes_activity_as_a_separate_observation_channel() {
        let scenario = run(ScenarioId::MarActivityArtifact);
        assert!(
            scenario
                .estimator_view
                .observations
                .iter()
                .any(|event| event.quantity == ObservationQuantity::ActivityProxy)
        );
        assert!(scenario.estimator_view.observations.iter().any(|event| {
            event.status == ObservationStatus::Missing(MissingReason::MarActivityLinked)
        }));
    }

    #[test]
    fn clock_fault_changes_sensor_clock_not_physical_truth() {
        let scenario = run(ScenarioId::ClockOffsetAndDrift);
        for event in &scenario.estimator_view.observations {
            let link = truth_link(&scenario, event);
            let expected = link.event_time_seconds * (1.0 + 200.0e-6) + 12.0;
            assert!((event.sensor_timestamp_seconds - expected).abs() < 1.0e-8);
            assert_eq!(event.clock_domain, 7);
        }
    }

    #[test]
    fn delayed_packets_respect_separate_availability_time() {
        let scenario = run(ScenarioId::DelayedAvailability);
        for event in &scenario.estimator_view.observations {
            let link = truth_link(&scenario, event);
            let latency = event.available_at_seconds - link.event_time_seconds;
            assert!((570.0..=630.0).contains(&latency));
        }
        assert!(
            scenario
                .estimator_view
                .observations
                .windows(2)
                .all(|pair| { pair[0].available_at_seconds <= pair[1].available_at_seconds })
        );
    }

    #[test]
    fn packet_faults_create_duplicates_and_out_of_order_capture_events() {
        let scenario = run(ScenarioId::PacketFaults);
        assert!(
            scenario
                .estimator_view
                .observations
                .iter()
                .any(|event| event.is_duplicate_delivery)
        );
        let has_reordering = scenario.estimator_view.observations.windows(2).any(|pair| {
            let left = truth_link(&scenario, &pair[0]).sample_index;
            let right = truth_link(&scenario, &pair[1]).sample_index;
            right < left
        });
        assert!(has_reordering);
        assert!(
            scenario
                .estimator_view
                .observations
                .windows(2)
                .all(|pair| { pair[0].available_at_seconds <= pair[1].available_at_seconds })
        );
    }

    #[test]
    fn common_cause_outage_affects_both_sensors_in_the_same_interval() {
        let scenario = run(ScenarioId::CommonCauseFailure);
        for sample_index in 180..=210 {
            let pair: Vec<_> = scenario
                .estimator_view
                .observations
                .iter()
                .filter(|event| truth_link(&scenario, event).sample_index == sample_index)
                .collect();
            assert_eq!(pair.len(), 2);
            assert!(pair.iter().all(|event| {
                event.status == ObservationStatus::Missing(MissingReason::CommonCauseOutage)
                    && event.value.is_none()
            }));
        }
    }

    #[test]
    fn clipping_is_flagged_and_values_remain_inside_sensor_limits() {
        let scenario = run(ScenarioId::QuantizationAndClipping);
        assert!(scenario.estimator_view.observations.iter().any(|event| {
            matches!(
                event.status,
                ObservationStatus::ClippedLow | ObservationStatus::ClippedHigh
            )
        }));
        assert!(scenario.estimator_view.observations.iter().all(|event| {
            event
                .value
                .is_none_or(|value| (0.3 - 1.0e-9..=0.7 + 1.0e-9).contains(&value))
        }));
    }

    #[test]
    fn indistinguishable_worlds_have_distinct_truth_but_the_same_observations() {
        let scenario = run(ScenarioId::IndistinguishableWorlds);
        let alternate = scenario.scorer_view.alternate_truth.as_ref().unwrap();
        assert_ne!(
            scenario.scorer_view.truth[0].latent_primary,
            alternate[0].latent_primary
        );
        assert_ne!(
            scenario.scorer_view.truth[0].latent_secondary,
            alternate[0].latent_secondary
        );
        assert_eq!(scenario.scorer_view.truth.len(), alternate.len());
        assert!(scenario.estimator_view.observations.iter().all(|event| {
            event.quantity == ObservationQuantity::CompositeSignal && event.value == Some(1.0)
        }));
    }

    #[test]
    fn all_events_have_finite_times_and_valid_missing_value_semantics() {
        for scenario_id in ScenarioId::ALL {
            let scenario = generate_scenario(scenario_id, 7);
            for event in &scenario.estimator_view.observations {
                assert!(event.sensor_timestamp_seconds.is_finite());
                assert!(event.available_at_seconds.is_finite());
                match event.status {
                    ObservationStatus::Missing(_) => assert!(event.value.is_none()),
                    _ => assert!(event.value.is_some_and(f64::is_finite)),
                }
            }
        }
    }
}
