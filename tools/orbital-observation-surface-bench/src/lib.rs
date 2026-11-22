pub mod decision_identifiability;\npub mod robust_decision;

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

const CORPUS_SCHEMA: &str = "luminous.orbital-observation-surface.corpus.v0.1";
const CLAIM_CEILING: &str = "stateful-marked-observation-simulation-only";
const MAX_EVENTS: usize = 100_000;
const MAX_CHANNELS: usize = 8;
const MAX_FALSE_POSITIVE_RATE: f64 = 10_000.0;
const MAX_FALSE_POSITIVE_EVENTS: usize = 100_000;
const MAX_SAFE_SEED: u64 = 9_007_199_254_740_991;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    pub schema: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub scenarios: Vec<Scenario>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub scenario_id: String,
    pub configuration_id: String,
    pub events: Vec<PhysicalEvent>,
    pub shared_common_cause_outages: Vec<TimeInterval>,
    pub channels: Vec<ChannelConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PhysicalEvent {
    pub event_id: String,
    pub time_fraction: f64,
    pub amplitude: f64,
    pub duration: f64,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TimeInterval {
    pub start: f64,
    pub end: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelConfig {
    pub channel_id: String,
    pub logit_intercept: f64,
    pub time_slope: f64,
    pub log_amplitude_slope: f64,
    pub log_duration_slope: f64,
    pub fixed_dead_time: f64,
    pub false_positive_rate: f64,
    pub channel_outages: Vec<TimeInterval>,
    pub detection_seed: u64,
    pub false_positive_seed: u64,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MissReason {
    SharedCommonCause,
    ChannelOutage,
    DeadTime,
    StochasticMiss,
}

#[derive(Debug, Clone, Serialize)]
pub struct EventOutcome {
    pub event_id: String,
    pub conditional_classifier_probability: f64,
    pub detected: bool,
    pub miss_reason: Option<MissReason>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct MissCounts {
    pub shared_common_cause: usize,
    pub channel_outage: usize,
    pub dead_time: usize,
    pub stochastic: usize,
}

/// Timestamp-only record that a downstream estimator is allowed to observe.
/// No latent event ID, miss reason, classifier probability or false-positive label is exposed.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ObservedEvent {
    pub time_fraction: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChannelAudit {
    pub latent_event_count: usize,
    pub detected_event_count: usize,
    pub miss_counts: MissCounts,
    pub false_positive_count: usize,
    pub false_positive_times: Vec<f64>,
    pub event_outcomes: Vec<EventOutcome>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChannelReport {
    pub channel_id: String,
    pub observed_event_stream: Vec<ObservedEvent>,
    pub audit: ChannelAudit,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScenarioReport {
    pub scenario_id: String,
    pub configuration_id: String,
    pub channels: Vec<ChannelReport>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ObservationChannel {
    pub channel_id: String,
    pub events: Vec<ObservedEvent>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ObservationScenario {
    pub scenario_id: String,
    pub configuration_id: String,
    pub channels: Vec<ObservationChannel>,
}

/// Declassified estimator input: observed timestamps only, with source and corpus hashes.
#[derive(Debug, Clone, Serialize)]
pub struct ObservationArtifact {
    pub schema: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub source_commit: String,
    pub corpus_sha256: String,
    pub scenarios: Vec<ObservationScenario>,
}

pub fn observations_only(
    report: &Report,
    source_commit: &str,
    corpus_sha256: &str,
) -> ObservationArtifact {
    ObservationArtifact {
        schema: "luminous.orbital-observation-surface.observations.v0.1".to_owned(),
        evidence_class: "synthetic".to_owned(),
        claim_ceiling: "synthetic-observation-stream-only".to_owned(),
        source_commit: source_commit.to_owned(),
        corpus_sha256: corpus_sha256.to_owned(),
        scenarios: report
            .scenarios
            .iter()
            .map(|scenario| ObservationScenario {
                scenario_id: scenario.scenario_id.clone(),
                configuration_id: scenario.configuration_id.clone(),
                channels: scenario
                    .channels
                    .iter()
                    .map(|channel| ObservationChannel {
                        channel_id: channel.channel_id.clone(),
                        events: channel.observed_event_stream.clone(),
                    })
                    .collect(),
            })
            .collect(),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub scenario_count: usize,
    pub channel_count: usize,
    pub channel_event_opportunity_count: usize,
    pub detected_event_count: usize,
    pub shared_common_cause_misses: usize,
    pub channel_outage_misses: usize,
    pub dead_time_misses: usize,
    pub stochastic_misses: usize,
    pub false_positive_count: usize,
    pub physical_reliability_claims: usize,
    pub qualification_claims: usize,
    pub authority_grants: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub result: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub summary: Summary,
    pub scenarios: Vec<ScenarioReport>,
    pub physical_reliability_claim_granted: bool,
    pub qualification_granted: bool,
    pub authority_granted: bool,
}

#[derive(Debug, Clone, Copy)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    fn uniform_open(&mut self) -> f64 {
        let numerator = (self.next_u64() >> 12) + 1;
        numerator as f64 / ((1_u64 << 52) + 1) as f64
    }
}

fn logistic(value: f64) -> f64 {
    if value >= 0.0 {
        1.0 / (1.0 + (-value).exp())
    } else {
        let exponential = value.exp();
        exponential / (1.0 + exponential)
    }
}

/// A marked, time-varying POD surface with a logistic link.
/// Amplitude and duration are expressed in the caller's frozen reference units.
/// Probability that the classifier accepts an event, conditional on the event reaching
/// the classifier. This is not end-to-end POD: outage and dead-time gates are separate.
pub fn conditional_classifier_probability(event: &PhysicalEvent, channel: &ChannelConfig) -> f64 {
    let linear_predictor = channel.logit_intercept
        + channel.time_slope * (event.time_fraction - 0.5)
        + channel.log_amplitude_slope * event.amplitude.ln()
        + channel.log_duration_slope * event.duration.ln();
    logistic(linear_predictor)
}

fn in_intervals(time: f64, intervals: &[TimeInterval]) -> bool {
    intervals
        .iter()
        .any(|interval| time >= interval.start && time < interval.end)
}

fn validate_intervals(intervals: &[TimeInterval]) -> Result<(), String> {
    let mut previous_end = 0.0;
    for (index, interval) in intervals.iter().enumerate() {
        if !interval.start.is_finite()
            || !interval.end.is_finite()
            || interval.start < 0.0
            || interval.end > 1.0
            || interval.start >= interval.end
            || (index > 0 && interval.start < previous_end)
        {
            return Err(
                "outage intervals must be sorted, non-overlapping and inside [0,1]".to_owned(),
            );
        }
        previous_end = interval.end;
    }
    Ok(())
}

fn validate_scenario(scenario: &Scenario, used_seeds: &mut HashSet<u64>) -> Result<(), String> {
    if scenario.scenario_id.trim().is_empty()
        || scenario.configuration_id.trim().is_empty()
        || scenario.events.is_empty()
        || scenario.events.len() > MAX_EVENTS
        || scenario.channels.is_empty()
        || scenario.channels.len() > MAX_CHANNELS
    {
        return Err(
            "scenario identity, exposure, event count or channel count is invalid".to_owned(),
        );
    }

    validate_intervals(&scenario.shared_common_cause_outages)?;

    let mut event_ids = HashSet::new();
    let mut previous_time = -1.0;
    for event in &scenario.events {
        if event.event_id.trim().is_empty()
            || !event_ids.insert(event.event_id.as_str())
            || !event.time_fraction.is_finite()
            || event.time_fraction < 0.0
            || event.time_fraction > 1.0
            || event.time_fraction <= previous_time
            || !event.amplitude.is_finite()
            || event.amplitude <= 0.0
            || !event.duration.is_finite()
            || event.duration <= 0.0
        {
            return Err("physical events must have unique IDs, strictly increasing times and positive finite marks".to_owned());
        }
        previous_time = event.time_fraction;
    }

    let mut channel_ids = HashSet::new();
    for channel in &scenario.channels {
        if channel.channel_id.trim().is_empty()
            || !channel_ids.insert(channel.channel_id.as_str())
            || !channel.logit_intercept.is_finite()
            || channel.logit_intercept.abs() > 10_000.0
            || !channel.time_slope.is_finite()
            || channel.time_slope.abs() > 10_000.0
            || !channel.log_amplitude_slope.is_finite()
            || channel.log_amplitude_slope.abs() > 10_000.0
            || !channel.log_duration_slope.is_finite()
            || channel.log_duration_slope.abs() > 10_000.0
            || !channel.fixed_dead_time.is_finite()
            || channel.fixed_dead_time < 0.0
            || channel.fixed_dead_time > 1.0
            || !channel.false_positive_rate.is_finite()
            || channel.false_positive_rate < 0.0
            || channel.false_positive_rate > MAX_FALSE_POSITIVE_RATE
        {
            return Err(
                "channel identity, POD coefficients, dead time or false-positive rate is invalid"
                    .to_owned(),
            );
        }
        validate_intervals(&channel.channel_outages)?;
        for seed in [channel.detection_seed, channel.false_positive_seed] {
            if seed > MAX_SAFE_SEED || !used_seeds.insert(seed) {
                return Err("random seeds must be unique across streams and exactly representable in JavaScript".to_owned());
            }
        }
    }
    Ok(())
}

fn simulate_false_positives(channel: &ChannelConfig) -> Result<Vec<f64>, String> {
    let mut random = SplitMix64::new(channel.false_positive_seed);
    let mut times = Vec::new();
    if channel.false_positive_rate == 0.0 {
        return Ok(times);
    }

    let mut time = 0.0;
    loop {
        let increment = -random.uniform_open().ln() / channel.false_positive_rate;
        time += increment;
        if time >= 1.0 {
            break;
        }
        if times.len() >= MAX_FALSE_POSITIVE_EVENTS {
            return Err("false-positive simulation exceeded the frozen event cap".to_owned());
        }
        times.push(time);
    }
    Ok(times)
}

fn simulate_channel(scenario: &Scenario, channel: &ChannelConfig) -> Result<ChannelReport, String> {
    let mut detection_random = SplitMix64::new(channel.detection_seed);
    let mut busy_until = f64::NEG_INFINITY;
    let mut miss_counts = MissCounts::default();
    let mut event_outcomes = Vec::with_capacity(scenario.events.len());
    let mut observed_event_stream = Vec::with_capacity(scenario.events.len());
    let mut detected_event_count = 0;

    for event in &scenario.events {
        // Consume one draw for every latent event so deterministic gating does not
        // shift the future Bernoulli stream when an outage interval is edited.
        let draw = detection_random.uniform_open();
        let probability = conditional_classifier_probability(event, channel);

        let miss_reason =
            if in_intervals(event.time_fraction, &scenario.shared_common_cause_outages) {
                miss_counts.shared_common_cause += 1;
                Some(MissReason::SharedCommonCause)
            } else if in_intervals(event.time_fraction, &channel.channel_outages) {
                miss_counts.channel_outage += 1;
                Some(MissReason::ChannelOutage)
            } else if event.time_fraction < busy_until {
                miss_counts.dead_time += 1;
                Some(MissReason::DeadTime)
            } else {
                // Non-paralyzable front-end model: an event admitted to the detector
                // starts recovery even if the downstream classifier returns a miss.
                busy_until = event.time_fraction + channel.fixed_dead_time;
                if draw < probability {
                    detected_event_count += 1;
                    None
                } else {
                    miss_counts.stochastic += 1;
                    Some(MissReason::StochasticMiss)
                }
            };

        let detected = miss_reason.is_none();
        if detected {
            observed_event_stream.push(ObservedEvent {
                time_fraction: event.time_fraction,
            });
        }
        event_outcomes.push(EventOutcome {
            event_id: event.event_id.clone(),
            conditional_classifier_probability: probability,
            detected,
            miss_reason,
        });
    }

    let false_positive_times = simulate_false_positives(channel)?
        .into_iter()
        .filter(|time| {
            !in_intervals(*time, &scenario.shared_common_cause_outages)
                && !in_intervals(*time, &channel.channel_outages)
        })
        .collect::<Vec<_>>();

    observed_event_stream.extend(false_positive_times.iter().map(|time| ObservedEvent {
        time_fraction: *time,
    }));
    observed_event_stream.sort_by(|left, right| left.time_fraction.total_cmp(&right.time_fraction));

    Ok(ChannelReport {
        channel_id: channel.channel_id.clone(),
        observed_event_stream,
        audit: ChannelAudit {
            latent_event_count: scenario.events.len(),
            detected_event_count,
            miss_counts,
            false_positive_count: false_positive_times.len(),
            false_positive_times,
            event_outcomes,
        },
    })
}

pub fn evaluate(corpus: &Corpus) -> Result<Report, String> {
    if corpus.schema != CORPUS_SCHEMA
        || corpus.evidence_class != "synthetic"
        || corpus.claim_ceiling != CLAIM_CEILING
        || corpus.scenarios.is_empty()
    {
        return Err("unsupported observation-surface corpus or claim ceiling".to_owned());
    }

    let mut used_seeds = HashSet::new();
    let mut scenario_ids = HashSet::new();
    let mut scenario_reports = Vec::with_capacity(corpus.scenarios.len());
    let mut summary = Summary {
        scenario_count: 0,
        channel_count: 0,
        channel_event_opportunity_count: 0,
        detected_event_count: 0,
        shared_common_cause_misses: 0,
        channel_outage_misses: 0,
        dead_time_misses: 0,
        stochastic_misses: 0,
        false_positive_count: 0,
        physical_reliability_claims: 0,
        qualification_claims: 0,
        authority_grants: 0,
    };

    for scenario in &corpus.scenarios {
        if !scenario_ids.insert(scenario.scenario_id.as_str()) {
            return Err("scenario IDs must be unique".to_owned());
        }
        validate_scenario(scenario, &mut used_seeds)?;
        let mut channels = Vec::with_capacity(scenario.channels.len());

        for channel in &scenario.channels {
            let result = simulate_channel(scenario, channel)?;
            summary.channel_count += 1;
            summary.channel_event_opportunity_count += result.audit.latent_event_count;
            summary.detected_event_count += result.audit.detected_event_count;
            summary.shared_common_cause_misses += result.audit.miss_counts.shared_common_cause;
            summary.channel_outage_misses += result.audit.miss_counts.channel_outage;
            summary.dead_time_misses += result.audit.miss_counts.dead_time;
            summary.stochastic_misses += result.audit.miss_counts.stochastic;
            summary.false_positive_count += result.audit.false_positive_count;
            channels.push(result);
        }

        summary.scenario_count += 1;
        scenario_reports.push(ScenarioReport {
            scenario_id: scenario.scenario_id.clone(),
            configuration_id: scenario.configuration_id.clone(),
            channels,
        });
    }

    Ok(Report {
        result: "pass".to_owned(),
        evidence_class: "synthetic".to_owned(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
        summary,
        scenarios: scenario_reports,
        physical_reliability_claim_granted: false,
        qualification_granted: false,
        authority_granted: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(id: &str, time: f64, amplitude: f64, duration: f64) -> PhysicalEvent {
        PhysicalEvent {
            event_id: id.to_owned(),
            time_fraction: time,
            amplitude,
            duration,
        }
    }

    fn channel(id: &str, seed: u64, dead_time: f64) -> ChannelConfig {
        ChannelConfig {
            channel_id: id.to_owned(),
            logit_intercept: 30.0,
            time_slope: 0.0,
            log_amplitude_slope: 0.0,
            log_duration_slope: 0.0,
            fixed_dead_time: dead_time,
            false_positive_rate: 0.0,
            channel_outages: Vec::new(),
            detection_seed: seed,
            false_positive_seed: seed + 1,
        }
    }

    fn corpus(scenario: Scenario) -> Corpus {
        Corpus {
            schema: CORPUS_SCHEMA.to_owned(),
            evidence_class: "synthetic".to_owned(),
            claim_ceiling: CLAIM_CEILING.to_owned(),
            scenarios: vec![scenario],
        }
    }

    #[test]
    fn mark_changes_detection_probability() {
        let mut c = channel("sensor", 1, 0.0);
        c.logit_intercept = 0.0;
        c.log_amplitude_slope = 1.5;
        let small = event("small", 0.5, 0.2, 1.0);
        let large = event("large", 0.5, 5.0, 1.0);
        assert!(
            conditional_classifier_probability(&large, &c)
                > conditional_classifier_probability(&small, &c)
        );
    }

    #[test]
    fn fixed_nonparalyzable_dead_time_suppresses_close_events() {
        let scenario = Scenario {
            scenario_id: "dead-time".to_owned(),
            configuration_id: "cfg-a".to_owned(),
            events: vec![
                event("e1", 0.1, 1.0, 0.01),
                event("e2", 0.15, 1.0, 0.01),
                event("e3", 0.25, 1.0, 0.01),
            ],
            shared_common_cause_outages: Vec::new(),
            channels: vec![channel("sensor", 11, 0.1)],
        };
        let report = evaluate(&corpus(scenario)).unwrap();
        let output = &report.scenarios[0].channels[0];
        assert_eq!(output.audit.detected_event_count, 2);
        assert_eq!(output.audit.miss_counts.dead_time, 1);
        assert_eq!(
            output.audit.event_outcomes[1].miss_reason,
            Some(MissReason::DeadTime)
        );
    }

    #[test]
    fn common_cause_outage_suppresses_all_channels() {
        let scenario = Scenario {
            scenario_id: "common-cause".to_owned(),
            configuration_id: "cfg-b".to_owned(),
            events: vec![event("e1", 0.2, 1.0, 0.01), event("e2", 0.3, 1.0, 0.01)],
            shared_common_cause_outages: vec![TimeInterval {
                start: 0.25,
                end: 0.35,
            }],
            channels: vec![channel("sensor-a", 21, 0.0), channel("sensor-b", 31, 0.0)],
        };
        let report = evaluate(&corpus(scenario)).unwrap();
        for channel in &report.scenarios[0].channels {
            assert_eq!(channel.audit.detected_event_count, 1);
            assert_eq!(channel.audit.miss_counts.shared_common_cause, 1);
            assert!(channel.audit.event_outcomes[1].conditional_classifier_probability > 0.0);
            assert!(!channel.audit.event_outcomes[1].detected);
        }
    }

    #[test]
    fn false_positive_stream_is_reproducible_and_separate() {
        let mut c = channel("sensor", 41, 0.0);
        c.false_positive_seed = 42;
        c.false_positive_rate = 20.0;
        let scenario = Scenario {
            scenario_id: "false-positives".to_owned(),
            configuration_id: "cfg-c".to_owned(),
            events: vec![event("e1", 0.2, 1.0, 0.01)],
            shared_common_cause_outages: Vec::new(),
            channels: vec![c],
        };
        let input = corpus(scenario);
        let first = evaluate(&input).unwrap();
        let second = evaluate(&input).unwrap();
        let first_channel = &first.scenarios[0].channels[0];
        let second_channel = &second.scenarios[0].channels[0];
        assert_eq!(
            first_channel.audit.false_positive_times,
            second_channel.audit.false_positive_times
        );
        assert_eq!(
            first.summary.false_positive_count,
            second.summary.false_positive_count
        );
        assert_eq!(
            first_channel.observed_event_stream.len(),
            first_channel.audit.detected_event_count + first_channel.audit.false_positive_count
        );
        assert!(
            first_channel
                .observed_event_stream
                .windows(2)
                .all(|pair| pair[0].time_fraction <= pair[1].time_fraction)
        );
    }

    #[test]
    fn stochastic_miss_still_triggers_declared_front_end_recovery() {
        let mut c = channel("sensor", 51, 0.2);
        c.logit_intercept = -100.0;
        let scenario = Scenario {
            scenario_id: "miss-trigger".to_owned(),
            configuration_id: "cfg-d".to_owned(),
            events: vec![event("e1", 0.1, 1.0, 0.01), event("e2", 0.15, 1.0, 0.01)],
            shared_common_cause_outages: Vec::new(),
            channels: vec![c],
        };
        let report = evaluate(&corpus(scenario)).unwrap();
        let output = &report.scenarios[0].channels[0];
        assert_eq!(output.audit.miss_counts.stochastic, 1);
        assert_eq!(output.audit.miss_counts.dead_time, 1);
    }

    #[test]
    fn event_times_must_be_strictly_increasing() {
        let scenario = Scenario {
            scenario_id: "invalid-order".to_owned(),
            configuration_id: "cfg-e".to_owned(),
            events: vec![event("e1", 0.2, 1.0, 0.01), event("e2", 0.2, 1.0, 0.01)],
            shared_common_cause_outages: Vec::new(),
            channels: vec![channel("sensor", 61, 0.0)],
        };
        assert!(evaluate(&corpus(scenario)).is_err());
    }

    #[test]
    fn splitmix_uniform_draws_are_strictly_inside_unit_interval() {
        let mut random = SplitMix64::new(0xD1CE_BA5E_1234_5678);
        for _ in 0..50_000 {
            let value = random.uniform_open();
            assert!(value > 0.0 && value < 1.0);
        }
    }

    #[test]
    fn inserting_an_outage_does_not_shift_later_event_random_draws() {
        let scenario = Scenario {
            scenario_id: "rng-stability".to_owned(),
            configuration_id: "cfg-rng".to_owned(),
            events: vec![
                event("e1", 0.1, 1.0, 0.01),
                event("e2", 0.2, 1.0, 0.01),
                event("e3", 0.3, 1.0, 0.01),
            ],
            shared_common_cause_outages: Vec::new(),
            channels: vec![{
                let mut c = channel("sensor", 701, 0.0);
                c.logit_intercept = 0.0;
                c
            }],
        };
        let baseline = evaluate(&corpus(scenario.clone())).unwrap();
        let mut gated_scenario = scenario;
        gated_scenario.shared_common_cause_outages = vec![TimeInterval {
            start: 0.15,
            end: 0.25,
        }];
        let gated = evaluate(&corpus(gated_scenario)).unwrap();
        let baseline_outcomes = &baseline.scenarios[0].channels[0].audit.event_outcomes;
        let gated_outcomes = &gated.scenarios[0].channels[0].audit.event_outcomes;
        assert_eq!(
            gated_outcomes[1].miss_reason,
            Some(MissReason::SharedCommonCause)
        );
        assert_eq!(baseline_outcomes[2].detected, gated_outcomes[2].detected);
    }

    #[test]
    fn declassified_observation_export_contains_no_latent_truth_labels() {
        let scenario = Scenario {
            scenario_id: "declassification".to_owned(),
            configuration_id: "cfg-safe".to_owned(),
            events: vec![event("truth-secret-a", 0.1, 1.0, 0.01)],
            shared_common_cause_outages: Vec::new(),
            channels: vec![channel("sensor", 801, 0.0)],
        };
        let report = evaluate(&corpus(scenario)).unwrap();
        let source = "0123456789abcdef0123456789abcdef01234567";
        let corpus_hash = "a".repeat(64);
        let artifact = observations_only(&report, source, &corpus_hash);
        let encoded = serde_json::to_string(&artifact).unwrap();
        assert!(!encoded.contains("truth-secret-a"));
        assert!(!encoded.contains("miss_reason"));
        assert!(!encoded.contains("conditional_classifier_probability"));
        assert_eq!(artifact.scenarios[0].channels[0].events.len(), 1);
        assert_eq!(
            artifact.scenarios[0].channels[0].events[0].time_fraction,
            0.1
        );
    }
}
