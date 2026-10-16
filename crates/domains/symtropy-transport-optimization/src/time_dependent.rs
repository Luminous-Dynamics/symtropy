// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Evidence-bounded, piecewise-linear FIFO arrival-time functions.
//!
//! Static edge travel times cannot represent time-of-day congestion or changing
//! schedules. This module adds a pure, deterministic arrival-time primitive that
//! can be tested independently before integration into route search. It only
//! supports FIFO functions: departing later cannot produce an earlier arrival.
//! Non-FIFO timetable semantics require a separate explicit waiting/scheduling
//! model and are never silently coerced to FIFO.
//!
//! Times are whole Unix seconds under a declared time profile. Interpolation is
//! integer-only and rounds arrival times upward, never making a route look
//! faster through fractional-second truncation. Values are usable only inside
//! the supplied departure domain and evidence validity interval, and only when
//! source publication + repository ingestion precede the replay knowledge cutoff.

use std::fmt;

/// One knot in an arrival-time function: departure clock time -> arrival time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ArrivalTimeBreakpointV1 {
    pub departure_epoch_seconds: u64,
    pub arrival_epoch_seconds: u64,
}

/// Source qualification is performed at the data-admission boundary. This
/// status is preserved here so an unqualified profile cannot yield a duration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArrivalFunctionEvidenceStatusV1 {
    Qualified,
    Unqualified { reason: String },
}

/// Provenance and temporal envelope for one immutable arrival-function version.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArrivalFunctionEvidenceV1 {
    pub function_id: String,
    pub time_profile_id: String,
    pub source_record_id: String,
    /// SHA-256 of the admitted source artifact, not a digest of this struct.
    pub source_record_sha256: Option<String>,
    /// Publication time controls historical knowledge. Missing means the
    /// profile cannot be used in a historical replay that needs a known-at test.
    pub source_published_at_epoch_seconds: Option<u64>,
    /// When the exact record was admitted to this reproducible corpus.
    pub repository_recorded_at_epoch_seconds: u64,
    /// Source measurement or model snapshot time; separate from publication.
    pub source_observed_at_epoch_seconds: u64,
    /// Departure-time domain supported by this evidence, half-open [from, until).
    pub valid_from_epoch_seconds: u64,
    pub valid_until_epoch_seconds: u64,
    pub source_locator: Option<String>,
    pub status: ArrivalFunctionEvidenceStatusV1,
}

/// Errors are explicit. In particular, unknown/out-of-domain inputs are not
/// replaced by a constant edge time or by unlimited validity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArrivalFunctionError {
    InvalidIdentityOrTimeProfile,
    InvalidEvidenceValidityInterval,
    MissingPublicationTime,
    NotKnownAtCutoff,
    UnqualifiedEvidence { reason: String },
    MissingQualifiedSourceDigest,
    InvalidSourceDigest,
    MissingQualifiedSourceLocator,
    TooFewBreakpoints,
    BreakpointOutsideEvidenceValidity,
    NonIncreasingDepartureTimes,
    ArrivalBeforeDeparture,
    NonFifoArrivalTimes,
    DepartureOutsideEvidenceValidity,
    DepartureOutsideFunctionDomain,
    ArithmeticOverflow,
}

impl fmt::Display for ArrivalFunctionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentityOrTimeProfile =>
                formatter.write_str("arrival function identity, source ID, or time profile is incomplete"),
            Self::InvalidEvidenceValidityInterval =>
                formatter.write_str("arrival-function evidence validity interval is empty or reversed"),
            Self::MissingPublicationTime =>
                formatter.write_str("arrival-function source has no explicit publication/knowledge time"),
            Self::NotKnownAtCutoff =>
                formatter.write_str("arrival-function source was not public and recorded by the replay cutoff"),
            Self::UnqualifiedEvidence { reason } =>
                write!(formatter, "arrival-function source is unqualified: {reason}"),
            Self::MissingQualifiedSourceDigest =>
                formatter.write_str("qualified arrival function lacks its source SHA-256"),
            Self::InvalidSourceDigest =>
                formatter.write_str("qualified arrival-function source digest is not 64 hexadecimal characters"),
            Self::MissingQualifiedSourceLocator =>
                formatter.write_str("qualified arrival function lacks an exact source locator"),
            Self::TooFewBreakpoints =>
                formatter.write_str("arrival-time function needs at least two breakpoints"),
            Self::BreakpointOutsideEvidenceValidity =>
                formatter.write_str("arrival-function departure domain exceeds its evidence validity interval"),
            Self::NonIncreasingDepartureTimes =>
                formatter.write_str("arrival-function departure breakpoints must be strictly increasing"),
            Self::ArrivalBeforeDeparture =>
                formatter.write_str("arrival time cannot precede departure time"),
            Self::NonFifoArrivalTimes =>
                formatter.write_str("arrival times decrease as departure times increase; FIFO is not satisfied"),
            Self::DepartureOutsideEvidenceValidity =>
                formatter.write_str("departure time falls outside the evidence validity interval"),
            Self::DepartureOutsideFunctionDomain =>
                formatter.write_str("departure time falls outside the declared arrival-function domain"),
            Self::ArithmeticOverflow =>
                formatter.write_str("arrival-time interpolation overflowed"),
        }
    }
}

impl std::error::Error for ArrivalFunctionError {}

/// Resolved point estimate for one departure time. This is a deterministic
/// interpolation result, not a probabilistic forecast or a reservation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArrivalTimeEstimateV1 {
    pub function_id: String,
    pub source_record_id: String,
    pub source_record_sha256: String,
    pub time_profile_id: String,
    pub knowledge_cutoff_epoch_seconds: u64,
    pub departure_epoch_seconds: u64,
    pub arrival_epoch_seconds: u64,
    pub travel_seconds: u64,
}

/// Validated FIFO arrival-time function with immutable provenance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FifoArrivalTimeFunctionV1 {
    evidence: ArrivalFunctionEvidenceV1,
    breakpoints: Vec<ArrivalTimeBreakpointV1>,
}

impl FifoArrivalTimeFunctionV1 {
    pub fn new(
        evidence: ArrivalFunctionEvidenceV1,
        breakpoints: Vec<ArrivalTimeBreakpointV1>,
    ) -> Result<Self, ArrivalFunctionError> {
        if evidence.function_id.trim().is_empty()
            || evidence.time_profile_id.trim().is_empty()
            || evidence.source_record_id.trim().is_empty()
        {
            return Err(ArrivalFunctionError::InvalidIdentityOrTimeProfile);
        }
        if evidence.valid_from_epoch_seconds >= evidence.valid_until_epoch_seconds {
            return Err(ArrivalFunctionError::InvalidEvidenceValidityInterval);
        }

        if evidence.status == ArrivalFunctionEvidenceStatusV1::Qualified {
            let digest = evidence
                .source_record_sha256
                .as_ref()
                .ok_or(ArrivalFunctionError::MissingQualifiedSourceDigest)?;
            if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return Err(ArrivalFunctionError::InvalidSourceDigest);
            }
            if evidence
                .source_locator
                .as_ref()
                .is_none_or(|locator| locator.trim().is_empty())
            {
                return Err(ArrivalFunctionError::MissingQualifiedSourceLocator);
            }
        }

        if breakpoints.len() < 2 {
            return Err(ArrivalFunctionError::TooFewBreakpoints);
        }
        let first = breakpoints[0].departure_epoch_seconds;
        let last = breakpoints[breakpoints.len() - 1].departure_epoch_seconds;
        if first < evidence.valid_from_epoch_seconds || last >= evidence.valid_until_epoch_seconds {
            return Err(ArrivalFunctionError::BreakpointOutsideEvidenceValidity);
        }

        for (index, point) in breakpoints.iter().enumerate() {
            if point.arrival_epoch_seconds < point.departure_epoch_seconds {
                return Err(ArrivalFunctionError::ArrivalBeforeDeparture);
            }
            if index > 0 {
                let previous = breakpoints[index - 1];
                if point.departure_epoch_seconds <= previous.departure_epoch_seconds {
                    return Err(ArrivalFunctionError::NonIncreasingDepartureTimes);
                }
                if point.arrival_epoch_seconds < previous.arrival_epoch_seconds {
                    return Err(ArrivalFunctionError::NonFifoArrivalTimes);
                }
            }
        }

        Ok(Self { evidence, breakpoints })
    }

    pub fn evidence(&self) -> &ArrivalFunctionEvidenceV1 {
        &self.evidence
    }

    pub fn breakpoints(&self) -> &[ArrivalTimeBreakpointV1] {
        &self.breakpoints
    }

    /// Evaluate at a departure instant only if the evidence was knowable by the
    /// explicit replay cutoff. No extrapolation outside the validated domain.
    pub fn arrival_at(
        &self,
        departure_epoch_seconds: u64,
        knowledge_cutoff_epoch_seconds: u64,
    ) -> Result<ArrivalTimeEstimateV1, ArrivalFunctionError> {
        let published_at = self
            .evidence
            .source_published_at_epoch_seconds
            .ok_or(ArrivalFunctionError::MissingPublicationTime)?;

        if published_at > knowledge_cutoff_epoch_seconds
            || self.evidence.repository_recorded_at_epoch_seconds > knowledge_cutoff_epoch_seconds
        {
            return Err(ArrivalFunctionError::NotKnownAtCutoff);
        }

        if let ArrivalFunctionEvidenceStatusV1::Unqualified { reason } = &self.evidence.status {
            return Err(ArrivalFunctionError::UnqualifiedEvidence { reason: reason.clone() });
        }

        if departure_epoch_seconds < self.evidence.valid_from_epoch_seconds
            || departure_epoch_seconds >= self.evidence.valid_until_epoch_seconds
        {
            return Err(ArrivalFunctionError::DepartureOutsideEvidenceValidity);
        }

        let arrival_epoch_seconds = self.interpolate_arrival(departure_epoch_seconds)?;
        let travel_seconds = arrival_epoch_seconds
            .checked_sub(departure_epoch_seconds)
            .ok_or(ArrivalFunctionError::ArrivalBeforeDeparture)?;

        // Construction requires a digest for Qualified evidence. Keep the check
        // defensive so future deserialization paths cannot bypass the contract.
        let source_record_sha256 = self
            .evidence
            .source_record_sha256
            .clone()
            .ok_or(ArrivalFunctionError::MissingQualifiedSourceDigest)?;

        Ok(ArrivalTimeEstimateV1 {
            function_id: self.evidence.function_id.clone(),
            source_record_id: self.evidence.source_record_id.clone(),
            source_record_sha256,
            time_profile_id: self.evidence.time_profile_id.clone(),
            knowledge_cutoff_epoch_seconds,
            departure_epoch_seconds,
            arrival_epoch_seconds,
            travel_seconds,
        })
    }

    fn interpolate_arrival(
        &self,
        departure_epoch_seconds: u64,
    ) -> Result<u64, ArrivalFunctionError> {
        let insertion_index = match self
            .breakpoints
            .binary_search_by_key(&departure_epoch_seconds, |point| point.departure_epoch_seconds)
        {
            Ok(index) => return Ok(self.breakpoints[index].arrival_epoch_seconds),
            Err(index) => index,
        };

        if insertion_index == 0 || insertion_index >= self.breakpoints.len() {
            return Err(ArrivalFunctionError::DepartureOutsideFunctionDomain);
        }

        let left = self.breakpoints[insertion_index - 1];
        let right = self.breakpoints[insertion_index];
        let elapsed_from_left = departure_epoch_seconds - left.departure_epoch_seconds;
        let departure_span = right.departure_epoch_seconds - left.departure_epoch_seconds;
        let arrival_span = right.arrival_epoch_seconds - left.arrival_epoch_seconds;

        // Ceiling interpolation avoids making an estimate faster than its
        // piecewise-linear arrival-time function. u128 safely holds a product
        // of two u64 values; checked operations keep the proof explicit.
        let numerator = u128::from(elapsed_from_left)
            .checked_mul(u128::from(arrival_span))
            .ok_or(ArrivalFunctionError::ArithmeticOverflow)?;
        let denominator = u128::from(departure_span);
        let rounded_numerator = numerator
            .checked_add(denominator - 1)
            .ok_or(ArrivalFunctionError::ArithmeticOverflow)?;
        let interpolated_delta = rounded_numerator / denominator;
        let interpolated_delta = u64::try_from(interpolated_delta)
            .map_err(|_| ArrivalFunctionError::ArithmeticOverflow)?;

        left.arrival_epoch_seconds
            .checked_add(interpolated_delta)
            .ok_or(ArrivalFunctionError::ArithmeticOverflow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn qualified_evidence() -> ArrivalFunctionEvidenceV1 {
        ArrivalFunctionEvidenceV1 {
            function_id: "road-link-17-am-peak-v1".into(),
            time_profile_id: "UTC_UNIX_SECONDS_V1".into(),
            source_record_id: "traffic-snapshot-2026-10-01-v1".into(),
            source_record_sha256: Some("a".repeat(64)),
            source_published_at_epoch_seconds: Some(50),
            repository_recorded_at_epoch_seconds: 55,
            source_observed_at_epoch_seconds: 45,
            valid_from_epoch_seconds: 100,
            valid_until_epoch_seconds: 3_000,
            source_locator: Some("manifest.json#/links/road-link-17/atf".into()),
            status: ArrivalFunctionEvidenceStatusV1::Qualified,
        }
    }

    fn function(
        breakpoints: &[(u64, u64)],
    ) -> Result<FifoArrivalTimeFunctionV1, ArrivalFunctionError> {
        FifoArrivalTimeFunctionV1::new(
            qualified_evidence(),
            breakpoints
                .iter()
                .map(|(departure, arrival)| ArrivalTimeBreakpointV1 {
                    departure_epoch_seconds: *departure,
                    arrival_epoch_seconds: *arrival,
                })
                .collect(),
        )
    }

    #[test]
    fn evaluates_exact_endpoints_and_interpolates_fractional_seconds_conservatively() {
        let function = function(&[(100, 151), (200, 252), (300, 360)]).unwrap();

        let first = function.arrival_at(100, 55).unwrap();
        assert_eq!(first.arrival_epoch_seconds, 151);
        assert_eq!(first.travel_seconds, 51);

        // Halfway through [100, 200], the linear arrival is 201.5 seconds;
        // integer ceiling yields 202 rather than silently rounding down.
        let middle = function.arrival_at(150, 55).unwrap();
        assert_eq!(middle.arrival_epoch_seconds, 202);
        assert_eq!(middle.travel_seconds, 52);

        let last = function.arrival_at(300, 55).unwrap();
        assert_eq!(last.arrival_epoch_seconds, 360);
        assert_eq!(last.travel_seconds, 60);
    }

    #[test]
    fn later_departures_never_arrive_earlier_for_a_valid_fifo_function() {
        let function = function(&[(100, 140), (200, 240), (300, 390)]).unwrap();
        let arrivals = (100..=300)
            .map(|departure| function.arrival_at(departure, 55).unwrap().arrival_epoch_seconds)
            .collect::<Vec<_>>();
        assert!(arrivals.windows(2).all(|pair| pair[0] <= pair[1]));
    }

    #[test]
    fn non_fifo_profiles_are_rejected_instead_of_silently_normalized() {
        assert_eq!(
            function(&[(100, 400), (200, 350)]),
            Err(ArrivalFunctionError::NonFifoArrivalTimes)
        );
    }

    #[test]
    fn arrival_before_departure_is_rejected() {
        assert_eq!(
            function(&[(100, 99), (200, 250)]),
            Err(ArrivalFunctionError::ArrivalBeforeDeparture)
        );
    }

    #[test]
    fn duplicate_or_reversed_departure_knots_are_rejected() {
        assert_eq!(
            function(&[(100, 120), (100, 130)]),
            Err(ArrivalFunctionError::NonIncreasingDepartureTimes)
        );
        assert_eq!(
            function(&[(200, 250), (100, 300)]),
            Err(ArrivalFunctionError::NonIncreasingDepartureTimes)
        );
    }

    #[test]
    fn evidence_publication_and_ingestion_cutoffs_are_both_enforced() {
        let function = function(&[(100, 151), (200, 252)]).unwrap();

        assert_eq!(
            function.arrival_at(150, 49),
            Err(ArrivalFunctionError::NotKnownAtCutoff)
        );
        assert_eq!(
            function.arrival_at(150, 54),
            Err(ArrivalFunctionError::NotKnownAtCutoff)
        );
        assert_eq!(function.arrival_at(150, 55).unwrap().arrival_epoch_seconds, 202);
    }

    #[test]
    fn missing_publication_time_and_unqualified_evidence_fail_closed() {
        let mut evidence = qualified_evidence();
        evidence.source_published_at_epoch_seconds = None;
        let missing_publication = FifoArrivalTimeFunctionV1::new(
            evidence,
            vec![
                ArrivalTimeBreakpointV1 { departure_epoch_seconds: 100, arrival_epoch_seconds: 150 },
                ArrivalTimeBreakpointV1 { departure_epoch_seconds: 200, arrival_epoch_seconds: 260 },
            ],
        ).unwrap();
        assert_eq!(
            missing_publication.arrival_at(150, 100),
            Err(ArrivalFunctionError::MissingPublicationTime)
        );

        let mut evidence = qualified_evidence();
        evidence.status = ArrivalFunctionEvidenceStatusV1::Unqualified {
            reason: "source-use terms not reviewed".into(),
        };
        evidence.source_record_sha256 = None;
        let unqualified = FifoArrivalTimeFunctionV1::new(
            evidence,
            vec![
                ArrivalTimeBreakpointV1 { departure_epoch_seconds: 100, arrival_epoch_seconds: 150 },
                ArrivalTimeBreakpointV1 { departure_epoch_seconds: 200, arrival_epoch_seconds: 260 },
            ],
        ).unwrap();
        assert_eq!(
            unqualified.arrival_at(150, 100),
            Err(ArrivalFunctionError::UnqualifiedEvidence {
                reason: "source-use terms not reviewed".into(),
            })
        );
    }

    #[test]
    fn interpolation_does_not_extrapolate_outside_function_or_evidence_horizon() {
        let function = function(&[(100, 151), (200, 252)]).unwrap();
        assert_eq!(
            function.arrival_at(99, 55),
            Err(ArrivalFunctionError::DepartureOutsideEvidenceValidity)
        );
        assert_eq!(
            function.arrival_at(201, 55),
            Err(ArrivalFunctionError::DepartureOutsideFunctionDomain)
        );
        assert_eq!(
            function.arrival_at(2_999, 55),
            Err(ArrivalFunctionError::DepartureOutsideFunctionDomain)
        );
        assert_eq!(
            function.arrival_at(3_000, 55),
            Err(ArrivalFunctionError::DepartureOutsideEvidenceValidity)
        );
    }

    #[test]
    fn qualified_evidence_requires_sha256_and_exact_locator() {
        let mut evidence = qualified_evidence();
        evidence.source_record_sha256 = None;
        assert_eq!(
            FifoArrivalTimeFunctionV1::new(evidence, vec![
                ArrivalTimeBreakpointV1 { departure_epoch_seconds: 100, arrival_epoch_seconds: 150 },
                ArrivalTimeBreakpointV1 { departure_epoch_seconds: 200, arrival_epoch_seconds: 260 },
            ]),
            Err(ArrivalFunctionError::MissingQualifiedSourceDigest)
        );

        let mut evidence = qualified_evidence();
        evidence.source_record_sha256 = Some("not-a-sha".into());
        assert_eq!(
            FifoArrivalTimeFunctionV1::new(evidence, vec![
                ArrivalTimeBreakpointV1 { departure_epoch_seconds: 100, arrival_epoch_seconds: 150 },
                ArrivalTimeBreakpointV1 { departure_epoch_seconds: 200, arrival_epoch_seconds: 260 },
            ]),
            Err(ArrivalFunctionError::InvalidSourceDigest)
        );

        let mut evidence = qualified_evidence();
        evidence.source_locator = None;
        assert_eq!(
            FifoArrivalTimeFunctionV1::new(evidence, vec![
                ArrivalTimeBreakpointV1 { departure_epoch_seconds: 100, arrival_epoch_seconds: 150 },
                ArrivalTimeBreakpointV1 { departure_epoch_seconds: 200, arrival_epoch_seconds: 260 },
            ]),
            Err(ArrivalFunctionError::MissingQualifiedSourceLocator)
        );
    }

    #[test]
    fn breakpoints_must_be_within_validity_and_function_must_have_two_points() {
        assert_eq!(
            function(&[(99, 150), (200, 250)]),
            Err(ArrivalFunctionError::BreakpointOutsideEvidenceValidity)
        );
        assert_eq!(
            function(&[(100, 150), (3_000, 3_100)]),
            Err(ArrivalFunctionError::BreakpointOutsideEvidenceValidity)
        );
        assert_eq!(
            function(&[(100, 150)]),
            Err(ArrivalFunctionError::TooFewBreakpoints)
        );
    }

    #[test]
    fn interpolation_is_input_immutable_and_deterministic() {
        let forward = function(&[(100, 150), (200, 270), (300, 330)]).unwrap();
        let reversed = function(&[(100, 150), (200, 270), (300, 330)]).unwrap();
        assert_eq!(forward, reversed);
        assert_eq!(forward.arrival_at(175, 55), reversed.arrival_at(175, 55));
    }
}
