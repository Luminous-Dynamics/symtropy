//! Clock qualification and causal alignment for multi-rate tactile streams.
//!
//! The module fits an affine mapping from a device clock into a declared
//! reference clock, preserves per-sample timestamp uncertainty, validates
//! sequence/time ordering, and forms causal high-rate windows for each tactile
//! sample. It never interpolates over packet gaps or admits future vibration
//! samples into a tactile decision window.
//!
//! This is a deterministic software oracle. A real system still needs shared
//! synchronization pulses or a characterized clock-transfer path, hardware
//! timestamp semantics, and independent event-time ground truth.

#![forbid(unsafe_code)]

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub struct ClockSyncPair {
    pub local_timestamp_ns: u64,
    pub reference_timestamp_ns: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum ClockSyncEvidenceDisposition {
    IndependentlyCharacterized,
    SharedCommonCause,
    Unresolved,
}

/// External receipt bundle binding synchronization pulses to their reference
/// ancestry and common-cause analysis. Receipt identifiers are references only;
/// consumers must resolve and verify the underlying evidence independently.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct ClockSyncAttestation {
    pub attestation_id: String,
    pub clock_domain: String,
    pub reference_clock_domain: String,
    pub sync_pairs: Vec<ClockSyncPair>,
    pub reference_instrument_receipt_id: String,
    pub reference_ancestry_receipt_id: String,
    pub common_cause_analysis_receipt_id: String,
    pub verifier_receipt_id: String,
    pub disposition: ClockSyncEvidenceDisposition,
}

impl ClockSyncAttestation {
    fn validate_binding(
        &self,
        clock_domain: &str,
        reference_clock_domain: &str,
        pairs: &[ClockSyncPair],
    ) -> Result<(), String> {
        for (name, value) in [
            ("attestation_id", self.attestation_id.as_str()),
            ("reference_instrument_receipt_id", self.reference_instrument_receipt_id.as_str()),
            ("reference_ancestry_receipt_id", self.reference_ancestry_receipt_id.as_str()),
            ("common_cause_analysis_receipt_id", self.common_cause_analysis_receipt_id.as_str()),
            ("verifier_receipt_id", self.verifier_receipt_id.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(format!("clock sync attestation field {name} is missing"));
            }
        }
        if self.clock_domain != clock_domain
            || self.reference_clock_domain != reference_clock_domain
        {
            return Err("clock sync attestation domain binding mismatch".to_owned());
        }
        if self.sync_pairs != pairs {
            return Err("clock sync attestation does not bind the supplied synchronization pairs".to_owned());
        }
        if self.disposition != ClockSyncEvidenceDisposition::IndependentlyCharacterized {
            return Err("clock synchronization ancestry/common-cause disposition is not independently characterized".to_owned());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct ClockMapping {
    pub clock_domain: String,
    pub reference_clock_domain: String,
    pub local_epoch_ns: u64,
    pub reference_epoch_ns: u64,
    /// Estimated offset at local_epoch_ns, in reference-clock nanoseconds.
    pub offset_ns: i64,
    /// Relative clock-rate error in parts per billion.
    pub drift_ppb: i64,
    /// Maximum absolute sync-pair residual after affine fitting.
    pub maximum_fit_residual_ns: u64,
    pub sync_pair_count: usize,
    pub sync_attestation_id: String,
    pub reference_instrument_receipt_id: String,
    pub reference_ancestry_receipt_id: String,
    pub common_cause_analysis_receipt_id: String,
    pub verifier_receipt_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TimedSample {
    pub sequence: u64,
    pub local_timestamp_ns: u64,
    pub timestamp_uncertainty_ns: u64,
    pub values: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TimedStream {
    pub device_id: String,
    pub clock_domain: String,
    pub channel_count: usize,
    pub samples: Vec<TimedSample>,
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
pub struct NormalizedSample {
    pub sequence: u64,
    pub reference_timestamp_ns: u64,
    pub timestamp_uncertainty_ns: u64,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct NormalizedStream {
    pub device_id: String,
    pub clock_domain: String,
    pub reference_clock_domain: String,
    pub channel_count: usize,
    pub samples: Vec<NormalizedSample>,
    pub sequence_gap_count: u64,
    pub maximum_inter_sample_gap_ns: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum AlignmentDisposition {
    Valid,
    MissingVibrationSamples,
    ExcessiveVibrationGap,
    ExcessiveClockUncertainty,
    StaleLatestVibrationSample,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct CausalVibrationWindow {
    pub tactile_sequence: u64,
    pub tactile_reference_timestamp_ns: u64,
    pub window_start_reference_timestamp_ns: u64,
    pub window_end_reference_timestamp_ns: u64,
    /// Every included vibration sample is at or before the tactile decision time.
    pub vibration_sequences: Vec<u64>,
    pub vibration_sample_count: usize,
    pub maximum_window_gap_ns: Option<u64>,
    pub latest_vibration_age_ns: Option<u64>,
    pub combined_clock_uncertainty_ns: u64,
    pub disposition: AlignmentDisposition,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct TemporalAlignmentReport {
    pub tactile_device_id: String,
    pub vibration_device_id: String,
    pub reference_clock_domain: String,
    pub tactile_sample_count: usize,
    pub valid_window_count: usize,
    pub missing_window_count: usize,
    pub excessive_gap_window_count: usize,
    pub excessive_uncertainty_window_count: usize,
    pub stale_window_count: usize,
    pub maximum_combined_clock_uncertainty_ns: u64,
    pub windows: Vec<CausalVibrationWindow>,
    pub claim_class: &'static str,
}

fn checked_i64(value: f64, field: &str) -> Result<i64, String> {
    if !value.is_finite() || value < i64::MIN as f64 || value > i64::MAX as f64 {
        return Err(format!("{field} is outside the representable i64 range"));
    }
    Ok(value.round() as i64)
}

/// Fit an affine mapping from device-local time into the reference clock.
///
/// `reference_time = reference_epoch + offset + delta * (1 + drift_ppb / 1e9)`
///
/// Three or more synchronization pairs are required so residuals can be
/// checked rather than simply fitting a line through two points. The exact
/// pulse set must be bound to an independent-reference/common-cause attestation.
pub fn fit_clock_mapping(
    clock_domain: &str,
    reference_clock_domain: &str,
    pairs: &[ClockSyncPair],
    attestation: &ClockSyncAttestation,
) -> Result<ClockMapping, String> {
    if clock_domain.trim().is_empty() || reference_clock_domain.trim().is_empty() {
        return Err("clock domains must be nonempty".to_owned());
    }
    if clock_domain == reference_clock_domain {
        return Err("clock mapping source and reference domains must differ".to_owned());
    }
    attestation.validate_binding(clock_domain, reference_clock_domain, pairs)?;
    if pairs.len() < 3 {
        return Err("at least three synchronization pairs are required".to_owned());
    }
    for pair in pairs.windows(2) {
        if pair[1].local_timestamp_ns <= pair[0].local_timestamp_ns
            || pair[1].reference_timestamp_ns <= pair[0].reference_timestamp_ns
        {
            return Err(
                "clock synchronization pairs must be strictly increasing in both domains"
                    .to_owned(),
            );
        }
    }

    let local_epoch_ns = pairs[0].local_timestamp_ns;
    let reference_epoch_ns = pairs[0].reference_timestamp_ns;
    let x: Vec<f64> = pairs
        .iter()
        .map(|pair| (pair.local_timestamp_ns - local_epoch_ns) as f64)
        .collect();
    let y: Vec<f64> = pairs
        .iter()
        .map(|pair| (pair.reference_timestamp_ns - reference_epoch_ns) as f64)
        .collect();
    let count = pairs.len() as f64;
    let mean_x = x.iter().sum::<f64>() / count;
    let mean_y = y.iter().sum::<f64>() / count;
    let variance_x = x.iter().map(|value| (value - mean_x).powi(2)).sum::<f64>();
    if !variance_x.is_finite() || variance_x <= f64::EPSILON {
        return Err("clock synchronization span is insufficient to estimate drift".to_owned());
    }
    let covariance_xy = x
        .iter()
        .zip(&y)
        .map(|(local, reference)| (local - mean_x) * (reference - mean_y))
        .sum::<f64>();
    let slope = covariance_xy / variance_x;
    if !slope.is_finite() || slope <= 0.0 {
        return Err("estimated clock rate is nonpositive or non-finite".to_owned());
    }

    let offset = mean_y - slope * mean_x;
    let offset_ns = checked_i64(offset, "clock offset")?;
    let drift_ppb = checked_i64((slope - 1.0) * 1.0e9, "clock drift")?;

    let maximum_fit_residual_ns = x
        .iter()
        .zip(&y)
        .map(|(local, reference)| (reference - (offset + slope * local)).abs().ceil())
        .fold(0.0_f64, f64::max);
    if !maximum_fit_residual_ns.is_finite() || maximum_fit_residual_ns > u64::MAX as f64 {
        return Err("clock fit residual is outside the representable u64 range".to_owned());
    }

    Ok(ClockMapping {
        clock_domain: clock_domain.to_owned(),
        reference_clock_domain: reference_clock_domain.to_owned(),
        local_epoch_ns,
        reference_epoch_ns,
        offset_ns,
        drift_ppb,
        maximum_fit_residual_ns: maximum_fit_residual_ns as u64,
        sync_pair_count: pairs.len(),
        sync_attestation_id: attestation.attestation_id.clone(),
        reference_instrument_receipt_id: attestation.reference_instrument_receipt_id.clone(),
        reference_ancestry_receipt_id: attestation.reference_ancestry_receipt_id.clone(),
        common_cause_analysis_receipt_id: attestation.common_cause_analysis_receipt_id.clone(),
        verifier_receipt_id: attestation.verifier_receipt_id.clone(),
    })
}

impl ClockMapping {
    pub fn map_timestamp(&self, local_timestamp_ns: u64) -> Result<u64, String> {
        let delta = i128::from(local_timestamp_ns) - i128::from(self.local_epoch_ns);
        let drift_numerator = delta
            .checked_mul(i128::from(self.drift_ppb))
            .ok_or_else(|| "clock drift adjustment overflows i128".to_owned())?;
        let drift_adjustment = drift_numerator / 1_000_000_000_i128;
        let mapped = i128::from(self.reference_epoch_ns)
            .checked_add(i128::from(self.offset_ns))
            .and_then(|value| value.checked_add(delta))
            .and_then(|value| value.checked_add(drift_adjustment))
            .ok_or_else(|| "mapped timestamp overflows i128".to_owned())?;
        u64::try_from(mapped).map_err(|_| "mapped timestamp is outside u64 range".to_owned())
    }
}

pub fn normalize_stream(
    stream: &TimedStream,
    mapping: &ClockMapping,
) -> Result<NormalizedStream, String> {
    if stream.device_id.trim().is_empty()
        || stream.clock_domain.trim().is_empty()
        || stream.channel_count == 0
    {
        return Err(
            "timed stream requires device, clock domain and nonzero channel count".to_owned(),
        );
    }
    if stream.clock_domain != mapping.clock_domain {
        return Err("timed stream clock domain does not match its mapping".to_owned());
    }
    if stream.samples.is_empty() {
        return Err("timed stream must contain samples".to_owned());
    }

    let mut normalized = Vec::with_capacity(stream.samples.len());
    let mut sequence_gap_count = 0_u64;
    let mut maximum_inter_sample_gap_ns = 0_u64;
    let mut previous_sequence = None;
    let mut previous_timestamp = None;

    for sample in &stream.samples {
        if sample.values.len() != stream.channel_count
            || sample.values.iter().any(|value| !value.is_finite())
        {
            return Err("sample channel count mismatch or non-finite channel value".to_owned());
        }
        if let Some(previous) = previous_sequence {
            if sample.sequence <= previous {
                return Err("sample sequence numbers must be strictly increasing".to_owned());
            }
            sequence_gap_count = sequence_gap_count
                .checked_add(sample.sequence - previous - 1)
                .ok_or_else(|| "sequence gap counter overflow".to_owned())?;
        }
        let reference_timestamp_ns = mapping.map_timestamp(sample.local_timestamp_ns)?;
        if let Some(previous) = previous_timestamp {
            if reference_timestamp_ns <= previous {
                return Err("normalized sample timestamps must be strictly increasing".to_owned());
            }
            maximum_inter_sample_gap_ns =
                maximum_inter_sample_gap_ns.max(reference_timestamp_ns - previous);
        }
        let timestamp_uncertainty_ns = mapping
            .maximum_fit_residual_ns
            .checked_add(sample.timestamp_uncertainty_ns)
            .ok_or_else(|| "sample timestamp uncertainty overflow".to_owned())?;
        normalized.push(NormalizedSample {
            sequence: sample.sequence,
            reference_timestamp_ns,
            timestamp_uncertainty_ns,
        });
        previous_sequence = Some(sample.sequence);
        previous_timestamp = Some(reference_timestamp_ns);
    }

    Ok(NormalizedStream {
        device_id: stream.device_id.clone(),
        clock_domain: stream.clock_domain.clone(),
        reference_clock_domain: mapping.reference_clock_domain.clone(),
        channel_count: stream.channel_count,
        samples: normalized,
        sequence_gap_count,
        maximum_inter_sample_gap_ns,
    })
}

/// Construct a causal vibration window for every tactile sample. The first
/// window uses maximum_window_duration_ns; subsequent windows use the interval
/// since the previous tactile sample. Samples after the tactile decision time
/// are never included, and data gaps are never interpolated.
fn validate_normalized_stream(stream: &NormalizedStream, name: &str) -> Result<(), String> {
    if stream.device_id.trim().is_empty()
        || stream.reference_clock_domain.trim().is_empty()
        || stream.channel_count == 0
        || stream.samples.is_empty()
    {
        return Err(format!(
            "{name} normalized stream metadata or samples are incomplete"
        ));
    }
    for pair in stream.samples.windows(2) {
        if pair[1].sequence <= pair[0].sequence
            || pair[1].reference_timestamp_ns <= pair[0].reference_timestamp_ns
        {
            return Err(format!(
                "{name} normalized samples are not strictly ordered"
            ));
        }
    }
    Ok(())
}

pub fn align_causal_multirate(
    tactile: &NormalizedStream,
    vibration: &NormalizedStream,
    maximum_window_duration_ns: u64,
    maximum_vibration_gap_ns: u64,
    maximum_latest_sample_age_ns: u64,
    maximum_alignment_uncertainty_ns: u64,
) -> Result<TemporalAlignmentReport, String> {
    validate_normalized_stream(tactile, "tactile")?;
    validate_normalized_stream(vibration, "vibration")?;
    if tactile.reference_clock_domain != vibration.reference_clock_domain {
        return Err("streams are not normalized to the same reference clock domain".to_owned());
    }
    if maximum_window_duration_ns == 0 || maximum_vibration_gap_ns == 0 {
        return Err("window duration and maximum vibration gap must be positive".to_owned());
    }

    let mut windows = Vec::with_capacity(tactile.samples.len());
    let mut valid_window_count = 0;
    let mut missing_window_count = 0;
    let mut excessive_gap_window_count = 0;
    let mut excessive_uncertainty_window_count = 0;
    let mut stale_window_count = 0;
    let mut maximum_combined_clock_uncertainty_ns = 0_u64;

    for (index, tactile_sample) in tactile.samples.iter().enumerate() {
        let window_start = if index == 0 {
            tactile_sample
                .reference_timestamp_ns
                .saturating_sub(maximum_window_duration_ns)
        } else {
            tactile.samples[index - 1].reference_timestamp_ns
        };
        let window_end = tactile_sample.reference_timestamp_ns;

        let included: Vec<_> = vibration
            .samples
            .iter()
            .filter(|sample| {
                sample.reference_timestamp_ns > window_start
                    && sample.reference_timestamp_ns <= window_end
            })
            .collect();

        let combined_uncertainty = included
            .iter()
            .map(|sample| sample.timestamp_uncertainty_ns)
            .max()
            .unwrap_or(0)
            .checked_add(tactile_sample.timestamp_uncertainty_ns)
            .ok_or_else(|| "combined timestamp uncertainty overflow".to_owned())?;
        maximum_combined_clock_uncertainty_ns =
            maximum_combined_clock_uncertainty_ns.max(combined_uncertainty);

        let mut maximum_window_gap = None;
        let mut latest_age = None;
        let mut disposition = AlignmentDisposition::Valid;

        if included.is_empty() {
            disposition = AlignmentDisposition::MissingVibrationSamples;
            missing_window_count += 1;
        } else {
            let first = included[0].reference_timestamp_ns;
            let last = included[included.len() - 1].reference_timestamp_ns;
            let mut maximum_gap = first.saturating_sub(window_start);
            for pair in included.windows(2) {
                maximum_gap = maximum_gap
                    .max(pair[1].reference_timestamp_ns - pair[0].reference_timestamp_ns);
            }
            maximum_gap = maximum_gap.max(window_end.saturating_sub(last));
            maximum_window_gap = Some(maximum_gap);
            latest_age = Some(window_end.saturating_sub(last));

            if maximum_gap > maximum_vibration_gap_ns {
                disposition = AlignmentDisposition::ExcessiveVibrationGap;
                excessive_gap_window_count += 1;
            } else if latest_age.unwrap_or(u64::MAX) > maximum_latest_sample_age_ns {
                disposition = AlignmentDisposition::StaleLatestVibrationSample;
                stale_window_count += 1;
            } else if combined_uncertainty > maximum_alignment_uncertainty_ns {
                disposition = AlignmentDisposition::ExcessiveClockUncertainty;
                excessive_uncertainty_window_count += 1;
            }
        }

        if disposition == AlignmentDisposition::Valid {
            valid_window_count += 1;
        }

        windows.push(CausalVibrationWindow {
            tactile_sequence: tactile_sample.sequence,
            tactile_reference_timestamp_ns: tactile_sample.reference_timestamp_ns,
            window_start_reference_timestamp_ns: window_start,
            window_end_reference_timestamp_ns: window_end,
            vibration_sequences: included.iter().map(|sample| sample.sequence).collect(),
            vibration_sample_count: included.len(),
            maximum_window_gap_ns: maximum_window_gap,
            latest_vibration_age_ns: latest_age,
            combined_clock_uncertainty_ns: combined_uncertainty,
            disposition,
        });
    }

    Ok(TemporalAlignmentReport {
        tactile_device_id: tactile.device_id.clone(),
        vibration_device_id: vibration.device_id.clone(),
        reference_clock_domain: tactile.reference_clock_domain.clone(),
        tactile_sample_count: tactile.samples.len(),
        valid_window_count,
        missing_window_count,
        excessive_gap_window_count,
        excessive_uncertainty_window_count,
        stale_window_count,
        maximum_combined_clock_uncertainty_ns,
        windows,
        claim_class: "synthetic_multirate_temporal_integrity_only",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity_mapping(domain: &str) -> ClockMapping {
        ClockMapping {
            clock_domain: domain.to_owned(),
            reference_clock_domain: "robot-clock".to_owned(),
            local_epoch_ns: 0,
            reference_epoch_ns: 0,
            offset_ns: 0,
            drift_ppb: 0,
            maximum_fit_residual_ns: 0,
            sync_pair_count: 3,
            sync_attestation_id: "attestation:identity-sync".to_owned(),
            reference_instrument_receipt_id: "receipt:identity-reference".to_owned(),
            reference_ancestry_receipt_id: "receipt:identity-ancestry".to_owned(),
            common_cause_analysis_receipt_id: "receipt:identity-common-cause".to_owned(),
            verifier_receipt_id: "receipt:identity-verifier".to_owned(),
        }
    }

    fn stream(
        device: &str,
        domain: &str,
        channel_count: usize,
        tuples: &[(u64, u64, f64)],
    ) -> TimedStream {
        TimedStream {
            device_id: device.to_owned(),
            clock_domain: domain.to_owned(),
            channel_count,
            samples: tuples
                .iter()
                .map(|(sequence, timestamp, value)| TimedSample {
                    sequence: *sequence,
                    local_timestamp_ns: *timestamp,
                    timestamp_uncertainty_ns: 1,
                    values: vec![*value; channel_count],
                })
                .collect(),
        }
    }

    #[test]
    fn affine_clock_fit_recovers_offset_and_drift() {
        let pairs = vec![
            ClockSyncPair {
                local_timestamp_ns: 1_000_000_000,
                reference_timestamp_ns: 1_005_000_000,
            },
            ClockSyncPair {
                local_timestamp_ns: 2_000_000_000,
                reference_timestamp_ns: 2_005_000_100,
            },
            ClockSyncPair {
                local_timestamp_ns: 3_000_000_000,
                reference_timestamp_ns: 3_005_000_200,
            },
            ClockSyncPair {
                local_timestamp_ns: 4_000_000_000,
                reference_timestamp_ns: 4_005_000_300,
            },
        ];
        let mapping =
            fit_clock_mapping("accelerometer-clock", "robot-clock", &pairs).expect("clock mapping");
        assert_eq!(mapping.offset_ns, 0);
        assert_eq!(mapping.drift_ppb, 100);
        assert_eq!(mapping.maximum_fit_residual_ns, 0);
        assert_eq!(mapping.map_timestamp(5_000_000_000).unwrap(), 5_005_000_400);
    }

    #[test]
    fn clock_fit_rejects_insufficient_or_nonmonotonic_sync_pairs() {
        assert!(
            fit_clock_mapping(
                "sensor",
                "robot",
                &[ClockSyncPair {
                    local_timestamp_ns: 1,
                    reference_timestamp_ns: 1,
                }]
            )
            .is_err()
        );
        let nonmonotonic = [
            ClockSyncPair {
                local_timestamp_ns: 1,
                reference_timestamp_ns: 1,
            },
            ClockSyncPair {
                local_timestamp_ns: 3,
                reference_timestamp_ns: 3,
            },
            ClockSyncPair {
                local_timestamp_ns: 2,
                reference_timestamp_ns: 4,
            },
        ];
        assert!(fit_clock_mapping("sensor", "robot", &nonmonotonic).is_err());
    }

    #[test]
    fn normalizer_preserves_sequence_gaps_and_rejects_reordering() {
        let mapping = identity_mapping("pressure-clock");
        let input = stream(
            "pressure-array",
            "pressure-clock",
            1,
            &[(1, 100, 0.1), (2, 200, 0.2), (5, 500, 0.5)],
        );
        let normalized = normalize_stream(&input, &mapping).expect("normal stream");
        assert_eq!(normalized.sequence_gap_count, 2);
        assert_eq!(normalized.maximum_inter_sample_gap_ns, 300);

        let reordered = stream(
            "pressure-array",
            "pressure-clock",
            1,
            &[(2, 200, 0.2), (1, 100, 0.1), (3, 300, 0.3)],
        );
        assert!(normalize_stream(&reordered, &mapping).is_err());
    }

    #[test]
    fn causal_alignment_never_includes_future_vibration_samples() {
        let tactile_mapping = identity_mapping("pressure-clock");
        let vibration_mapping = identity_mapping("vibration-clock");
        let tactile = stream(
            "pressure-array",
            "pressure-clock",
            1,
            &[(1, 100, 0.1), (2, 200, 0.2)],
        );
        let vibration = stream(
            "accelerometer",
            "vibration-clock",
            1,
            &[
                (10, 90, 0.1),
                (11, 99, 0.1),
                (12, 101, 0.2),
                (13, 199, 0.3),
                (14, 201, 0.4),
            ],
        );
        let tactile = normalize_stream(&tactile, &tactile_mapping).unwrap();
        let vibration = normalize_stream(&vibration, &vibration_mapping).unwrap();
        let report =
            align_causal_multirate(&tactile, &vibration, 100, 100, 2, 5).expect("causal alignment");
        assert_eq!(report.windows[0].vibration_sequences, vec![10, 11]);
        assert_eq!(report.windows[1].vibration_sequences, vec![12, 13]);
        assert!(
            report
                .windows
                .iter()
                .all(|window| { !window.vibration_sequences.contains(&14) })
        );
    }

    #[test]
    fn vibration_gaps_and_clock_uncertainty_are_explicit_not_interpolated() {
        let tactile_mapping = identity_mapping("pressure-clock");
        let vibration_mapping = ClockMapping {
            maximum_fit_residual_ns: 10,
            ..identity_mapping("vibration-clock")
        };
        let tactile = stream(
            "pressure-array",
            "pressure-clock",
            1,
            &[(1, 100, 0.1), (2, 200, 0.2)],
        );
        let vibration = stream(
            "accelerometer",
            "vibration-clock",
            1,
            &[(10, 90, 0.1), (11, 199, 0.2)],
        );
        let tactile = normalize_stream(&tactile, &tactile_mapping).unwrap();
        let vibration = normalize_stream(&vibration, &vibration_mapping).unwrap();
        let gap_report = align_causal_multirate(&tactile, &vibration, 100, 20, 20, 5)
            .expect("gap alignment report");
        assert_eq!(
            gap_report.windows[0].disposition,
            AlignmentDisposition::ExcessiveVibrationGap
        );
        assert_eq!(
            gap_report.windows[1].disposition,
            AlignmentDisposition::ExcessiveVibrationGap
        );

        let uncertainty_report = align_causal_multirate(&tactile, &vibration, 100, 100, 20, 5)
            .expect("uncertainty alignment report");
        assert_eq!(
            uncertainty_report.windows[1].disposition,
            AlignmentDisposition::ExcessiveClockUncertainty
        );
    }

    #[test]
    fn mismatched_reference_clock_domains_fail_closed() {
        let tactile = NormalizedStream {
            device_id: "pressure".to_owned(),
            clock_domain: "p".to_owned(),
            reference_clock_domain: "robot-a".to_owned(),
            channel_count: 1,
            samples: vec![NormalizedSample {
                sequence: 1,
                reference_timestamp_ns: 10,
                timestamp_uncertainty_ns: 0,
            }],
            sequence_gap_count: 0,
            maximum_inter_sample_gap_ns: 0,
        };
        let vibration = NormalizedStream {
            reference_clock_domain: "robot-b".to_owned(),
            ..tactile.clone()
        };
        assert!(align_causal_multirate(&tactile, &vibration, 10, 10, 10, 10).is_err());
    }
}
