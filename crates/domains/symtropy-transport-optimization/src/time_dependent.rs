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
    TimeProfileMismatch,
    ArithmeticOverflow,
}

impl fmt::Display for ArrivalFunctionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentityOrTimeProfile => formatter
                .write_str("arrival function identity, source ID, or time profile is incomplete"),
            Self::InvalidEvidenceValidityInterval => formatter
                .write_str("arrival-function evidence validity interval is empty or reversed"),
            Self::MissingPublicationTime => formatter
                .write_str("arrival-function source has no explicit publication/knowledge time"),
            Self::NotKnownAtCutoff => formatter.write_str(
                "arrival-function source was not public and recorded by the replay cutoff",
            ),
            Self::UnqualifiedEvidence { reason } => write!(
                formatter,
                "arrival-function source is unqualified: {reason}"
            ),
            Self::MissingQualifiedSourceDigest => {
                formatter.write_str("qualified arrival function lacks its source SHA-256")
            }
            Self::InvalidSourceDigest => formatter.write_str(
                "qualified arrival-function source digest is not 64 hexadecimal characters",
            ),
            Self::MissingQualifiedSourceLocator => {
                formatter.write_str("qualified arrival function lacks an exact source locator")
            }
            Self::TooFewBreakpoints => {
                formatter.write_str("arrival-time function needs at least two breakpoints")
            }
            Self::BreakpointOutsideEvidenceValidity => formatter.write_str(
                "arrival-function departure domain exceeds its evidence validity interval",
            ),
            Self::NonIncreasingDepartureTimes => formatter
                .write_str("arrival-function departure breakpoints must be strictly increasing"),
            Self::ArrivalBeforeDeparture => {
                formatter.write_str("arrival time cannot precede departure time")
            }
            Self::NonFifoArrivalTimes => formatter.write_str(
                "arrival times decrease as departure times increase; FIFO is not satisfied",
            ),
            Self::DepartureOutsideEvidenceValidity => {
                formatter.write_str("departure time falls outside the evidence validity interval")
            }
            Self::DepartureOutsideFunctionDomain => formatter
                .write_str("departure time falls outside the declared arrival-function domain"),
            Self::TimeProfileMismatch => {
                formatter.write_str("query and arrival-function time profiles do not match")
            }
            Self::ArithmeticOverflow => {
                formatter.write_str("arrival-time interpolation overflowed")
            }
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

        Ok(Self {
            evidence,
            breakpoints,
        })
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
        query_time_profile_id: &str,
    ) -> Result<ArrivalTimeEstimateV1, ArrivalFunctionError> {
        if query_time_profile_id != self.evidence.time_profile_id {
            return Err(ArrivalFunctionError::TimeProfileMismatch);
        }

        let published_at = self
            .evidence
            .source_published_at_epoch_seconds
            .ok_or(ArrivalFunctionError::MissingPublicationTime)?;

        if published_at > knowledge_cutoff_epoch_seconds
            || self.evidence.repository_recorded_at_epoch_seconds > knowledge_cutoff_epoch_seconds
            || self.evidence.source_observed_at_epoch_seconds > knowledge_cutoff_epoch_seconds
        {
            return Err(ArrivalFunctionError::NotKnownAtCutoff);
        }

        if let ArrivalFunctionEvidenceStatusV1::Unqualified { reason } = &self.evidence.status {
            return Err(ArrivalFunctionError::UnqualifiedEvidence {
                reason: reason.clone(),
            });
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
            .binary_search_by_key(&departure_epoch_seconds, |point| {
                point.departure_epoch_seconds
            }) {
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

/// One ordered route leg with a declared continuous node transition.
///
/// `pre_leg_delay_seconds` is explicit deterministic handling/access/dwell time
/// before this leg; it is not a scheduled-service wait and does not infer a
/// departure from a timetable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FifoPathLegV1 {
    pub edge_id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub pre_leg_delay_seconds: u64,
    pub arrival_function: FifoArrivalTimeFunctionV1,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FifoPathLegEstimateV1 {
    pub edge_id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub pre_leg_delay_seconds: u64,
    pub estimate: ArrivalTimeEstimateV1,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FifoPathEstimateV1 {
    pub origin_node_id: String,
    pub destination_node_id: String,
    pub time_profile_id: String,
    pub knowledge_cutoff_epoch_seconds: u64,
    pub initial_departure_epoch_seconds: u64,
    pub final_arrival_epoch_seconds: u64,
    pub total_elapsed_seconds: u64,
    pub legs: Vec<FifoPathLegEstimateV1>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FifoPathEvaluationError {
    EmptyPath,
    InvalidNodeOrEdgeIdentity,
    RepeatedEdge(String),
    DisconnectedLegs {
        previous_edge_id: String,
        next_edge_id: String,
        expected_node_id: String,
        actual_node_id: String,
    },
    ArithmeticOverflow,
    ArrivalFunction {
        edge_id: String,
        source: ArrivalFunctionError,
    },
}

impl fmt::Display for FifoPathEvaluationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPath => formatter.write_str("FIFO path must contain at least one leg"),
            Self::InvalidNodeOrEdgeIdentity => {
                formatter.write_str("FIFO path contains an empty edge or node identity")
            }
            Self::RepeatedEdge(edge_id) => write!(formatter, "FIFO path repeats edge {edge_id}"),
            Self::DisconnectedLegs {
                previous_edge_id,
                next_edge_id,
                expected_node_id,
                actual_node_id,
            } => write!(
                formatter,
                "FIFO path is disconnected between edge {previous_edge_id} (ends at {expected_node_id}) and edge {next_edge_id} (starts at {actual_node_id})"
            ),
            Self::ArithmeticOverflow => {
                formatter.write_str("FIFO path delay or total elapsed time overflowed")
            }
            Self::ArrivalFunction { edge_id, source } => {
                write!(
                    formatter,
                    "arrival function for edge {edge_id} failed: {source}"
                )
            }
        }
    }
}

impl std::error::Error for FifoPathEvaluationError {}

/// Compose FIFO leg functions in route order without resetting the clock.
///
/// The next leg departs at the previous leg's arrival plus its declared
/// `pre_leg_delay_seconds`. This function does not infer timetable departures,
/// waiting rules, route-edge aliases, physical capacity, or dispatch authority.
pub fn evaluate_fifo_path(
    legs: &[FifoPathLegV1],
    initial_departure_epoch_seconds: u64,
    knowledge_cutoff_epoch_seconds: u64,
    time_profile_id: &str,
) -> Result<FifoPathEstimateV1, FifoPathEvaluationError> {
    if legs.is_empty() {
        return Err(FifoPathEvaluationError::EmptyPath);
    }

    let mut current_time = initial_departure_epoch_seconds;
    let mut previous_node: Option<&str> = None;
    let mut previous_edge_id: Option<&str> = None;
    let mut seen_edges = std::collections::BTreeSet::new();
    let mut estimates = Vec::with_capacity(legs.len());

    for leg in legs {
        if leg.edge_id.trim().is_empty()
            || leg.from_node_id.trim().is_empty()
            || leg.to_node_id.trim().is_empty()
        {
            return Err(FifoPathEvaluationError::InvalidNodeOrEdgeIdentity);
        }
        if !seen_edges.insert(leg.edge_id.as_str()) {
            return Err(FifoPathEvaluationError::RepeatedEdge(leg.edge_id.clone()));
        }
        if let Some(expected_node_id) = previous_node
            && leg.from_node_id.as_str() != expected_node_id
        {
            return Err(FifoPathEvaluationError::DisconnectedLegs {
                previous_edge_id: previous_edge_id.unwrap_or_default().to_owned(),
                next_edge_id: leg.edge_id.clone(),
                expected_node_id: expected_node_id.to_owned(),
                actual_node_id: leg.from_node_id.clone(),
            });
        }

        let leg_departure = current_time
            .checked_add(leg.pre_leg_delay_seconds)
            .ok_or(FifoPathEvaluationError::ArithmeticOverflow)?;
        let estimate = leg
            .arrival_function
            .arrival_at(
                leg_departure,
                knowledge_cutoff_epoch_seconds,
                time_profile_id,
            )
            .map_err(|source| FifoPathEvaluationError::ArrivalFunction {
                edge_id: leg.edge_id.clone(),
                source,
            })?;

        current_time = estimate.arrival_epoch_seconds;
        estimates.push(FifoPathLegEstimateV1 {
            edge_id: leg.edge_id.clone(),
            from_node_id: leg.from_node_id.clone(),
            to_node_id: leg.to_node_id.clone(),
            pre_leg_delay_seconds: leg.pre_leg_delay_seconds,
            estimate,
        });
        previous_node = Some(leg.to_node_id.as_str());
        previous_edge_id = Some(leg.edge_id.as_str());
    }

    let origin_node_id = legs[0].from_node_id.clone();
    let destination_node_id = legs[legs.len() - 1].to_node_id.clone();
    let total_elapsed_seconds = current_time
        .checked_sub(initial_departure_epoch_seconds)
        .ok_or(FifoPathEvaluationError::ArithmeticOverflow)?;

    Ok(FifoPathEstimateV1 {
        origin_node_id,
        destination_node_id,
        time_profile_id: time_profile_id.to_owned(),
        knowledge_cutoff_epoch_seconds,
        initial_departure_epoch_seconds,
        final_arrival_epoch_seconds: current_time,
        total_elapsed_seconds,
        legs: estimates,
    })
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

        let first = function.arrival_at(100, 55, "UTC_UNIX_SECONDS_V1").unwrap();
        assert_eq!(first.arrival_epoch_seconds, 151);
        assert_eq!(first.travel_seconds, 51);

        // Halfway through [100, 200], the linear arrival is 201.5 seconds;
        // integer ceiling yields 202 rather than silently rounding down.
        let middle = function.arrival_at(150, 55, "UTC_UNIX_SECONDS_V1").unwrap();
        assert_eq!(middle.arrival_epoch_seconds, 202);
        assert_eq!(middle.travel_seconds, 52);

        let last = function.arrival_at(300, 55, "UTC_UNIX_SECONDS_V1").unwrap();
        assert_eq!(last.arrival_epoch_seconds, 360);
        assert_eq!(last.travel_seconds, 60);
    }

    #[test]
    fn later_departures_never_arrive_earlier_for_a_valid_fifo_function() {
        let function = function(&[(100, 140), (200, 240), (300, 390)]).unwrap();
        let arrivals = (100..=300)
            .map(|departure| {
                function
                    .arrival_at(departure, 55, "UTC_UNIX_SECONDS_V1")
                    .unwrap()
                    .arrival_epoch_seconds
            })
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
            function.arrival_at(150, 49, "UTC_UNIX_SECONDS_V1"),
            Err(ArrivalFunctionError::NotKnownAtCutoff)
        );
        assert_eq!(
            function.arrival_at(150, 54, "UTC_UNIX_SECONDS_V1"),
            Err(ArrivalFunctionError::NotKnownAtCutoff)
        );
        assert_eq!(
            function
                .arrival_at(150, 55, "UTC_UNIX_SECONDS_V1")
                .unwrap()
                .arrival_epoch_seconds,
            202
        );
    }

    #[test]
    fn future_source_observation_cannot_leak_through_older_publication_metadata() {
        let mut evidence = qualified_evidence();
        evidence.source_observed_at_epoch_seconds = 56;
        let function = FifoArrivalTimeFunctionV1::new(
            evidence,
            vec![
                ArrivalTimeBreakpointV1 {
                    departure_epoch_seconds: 100,
                    arrival_epoch_seconds: 150,
                },
                ArrivalTimeBreakpointV1 {
                    departure_epoch_seconds: 200,
                    arrival_epoch_seconds: 260,
                },
            ],
        )
        .unwrap();
        // Even if metadata claims an earlier publication and ingestion time,
        // a future source observation is impossible at this replay cutoff.
        assert_eq!(
            function.arrival_at(150, 55, "UTC_UNIX_SECONDS_V1"),
            Err(ArrivalFunctionError::NotKnownAtCutoff)
        );
    }

    #[test]
    fn missing_publication_time_and_unqualified_evidence_fail_closed() {
        let mut evidence = qualified_evidence();
        evidence.source_published_at_epoch_seconds = None;
        let missing_publication = FifoArrivalTimeFunctionV1::new(
            evidence,
            vec![
                ArrivalTimeBreakpointV1 {
                    departure_epoch_seconds: 100,
                    arrival_epoch_seconds: 150,
                },
                ArrivalTimeBreakpointV1 {
                    departure_epoch_seconds: 200,
                    arrival_epoch_seconds: 260,
                },
            ],
        )
        .unwrap();
        assert_eq!(
            missing_publication.arrival_at(150, 100, "UTC_UNIX_SECONDS_V1"),
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
                ArrivalTimeBreakpointV1 {
                    departure_epoch_seconds: 100,
                    arrival_epoch_seconds: 150,
                },
                ArrivalTimeBreakpointV1 {
                    departure_epoch_seconds: 200,
                    arrival_epoch_seconds: 260,
                },
            ],
        )
        .unwrap();
        assert_eq!(
            unqualified.arrival_at(150, 100, "UTC_UNIX_SECONDS_V1"),
            Err(ArrivalFunctionError::UnqualifiedEvidence {
                reason: "source-use terms not reviewed".into(),
            })
        );
    }

    #[test]
    fn interpolation_does_not_extrapolate_outside_function_or_evidence_horizon() {
        let function = function(&[(100, 151), (200, 252)]).unwrap();
        assert_eq!(
            function.arrival_at(99, 55, "UTC_UNIX_SECONDS_V1"),
            Err(ArrivalFunctionError::DepartureOutsideEvidenceValidity)
        );
        assert_eq!(
            function.arrival_at(201, 55, "UTC_UNIX_SECONDS_V1"),
            Err(ArrivalFunctionError::DepartureOutsideFunctionDomain)
        );
        assert_eq!(
            function.arrival_at(2_999, 55, "UTC_UNIX_SECONDS_V1"),
            Err(ArrivalFunctionError::DepartureOutsideFunctionDomain)
        );
        assert_eq!(
            function.arrival_at(3_000, 55, "UTC_UNIX_SECONDS_V1"),
            Err(ArrivalFunctionError::DepartureOutsideEvidenceValidity)
        );
    }

    #[test]
    fn qualified_evidence_requires_sha256_and_exact_locator() {
        let mut evidence = qualified_evidence();
        evidence.source_record_sha256 = None;
        assert_eq!(
            FifoArrivalTimeFunctionV1::new(
                evidence,
                vec![
                    ArrivalTimeBreakpointV1 {
                        departure_epoch_seconds: 100,
                        arrival_epoch_seconds: 150
                    },
                    ArrivalTimeBreakpointV1 {
                        departure_epoch_seconds: 200,
                        arrival_epoch_seconds: 260
                    },
                ]
            ),
            Err(ArrivalFunctionError::MissingQualifiedSourceDigest)
        );

        let mut evidence = qualified_evidence();
        evidence.source_record_sha256 = Some("not-a-sha".into());
        assert_eq!(
            FifoArrivalTimeFunctionV1::new(
                evidence,
                vec![
                    ArrivalTimeBreakpointV1 {
                        departure_epoch_seconds: 100,
                        arrival_epoch_seconds: 150
                    },
                    ArrivalTimeBreakpointV1 {
                        departure_epoch_seconds: 200,
                        arrival_epoch_seconds: 260
                    },
                ]
            ),
            Err(ArrivalFunctionError::InvalidSourceDigest)
        );

        let mut evidence = qualified_evidence();
        evidence.source_locator = None;
        assert_eq!(
            FifoArrivalTimeFunctionV1::new(
                evidence,
                vec![
                    ArrivalTimeBreakpointV1 {
                        departure_epoch_seconds: 100,
                        arrival_epoch_seconds: 150
                    },
                    ArrivalTimeBreakpointV1 {
                        departure_epoch_seconds: 200,
                        arrival_epoch_seconds: 260
                    },
                ]
            ),
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
    fn fifo_allows_later_departures_to_have_shorter_trip_durations() {
        // Arrival times remain monotone (200 <= 210), even though duration
        // drops from 100 seconds to 10 seconds between these breakpoints.
        let function = function(&[(100, 200), (200, 210), (300, 330)]).unwrap();
        let early = function.arrival_at(100, 55, "UTC_UNIX_SECONDS_V1").unwrap();
        let later = function.arrival_at(200, 55, "UTC_UNIX_SECONDS_V1").unwrap();

        assert_eq!(early.travel_seconds, 100);
        assert_eq!(later.travel_seconds, 10);
        assert!(early.arrival_epoch_seconds <= later.arrival_epoch_seconds);
    }

    #[test]
    fn every_integer_departure_matches_a_slow_rational_reference() {
        let points = [(100, 103), (125, 148), (150, 160)];
        let function = function(&points).unwrap();

        for departure in 100..=150 {
            let interval = points
                .windows(2)
                .find(|pair| departure >= pair[0].0 && departure <= pair[1].0)
                .expect("departure is bracketed by a breakpoint interval");
            let (left_departure, left_arrival) = interval[0];
            let (right_departure, right_arrival) = interval[1];
            let numerator =
                u128::from(departure - left_departure) * u128::from(right_arrival - left_arrival);
            let denominator = u128::from(right_departure - left_departure);
            let ceiling_delta = numerator.div_ceil(denominator);
            let reference = left_arrival + u64::try_from(ceiling_delta).unwrap();

            assert_eq!(
                function
                    .arrival_at(departure, 55, "UTC_UNIX_SECONDS_V1")
                    .unwrap()
                    .arrival_epoch_seconds,
                reference,
                "departure {departure} disagreed with independent rational reference"
            );
        }
    }

    #[test]
    fn path_composition_carries_arrival_time_and_explicit_delay_forward() {
        let road = function(&[(100, 150), (200, 250)]).unwrap();
        let rail = function(&[(100, 160), (200, 270), (300, 330)]).unwrap();
        let legs = vec![
            FifoPathLegV1 {
                edge_id: "road-1".into(),
                from_node_id: "origin".into(),
                to_node_id: "terminal".into(),
                pre_leg_delay_seconds: 0,
                arrival_function: road,
            },
            FifoPathLegV1 {
                edge_id: "rail-1".into(),
                from_node_id: "terminal".into(),
                to_node_id: "destination".into(),
                pre_leg_delay_seconds: 10,
                arrival_function: rail,
            },
        ];

        let path = evaluate_fifo_path(&legs, 150, 55, "UTC_UNIX_SECONDS_V1").unwrap();
        assert_eq!(path.origin_node_id, "origin");
        assert_eq!(path.destination_node_id, "destination");
        assert_eq!(path.legs[0].estimate.departure_epoch_seconds, 150);
        assert_eq!(path.legs[0].estimate.arrival_epoch_seconds, 200);
        assert_eq!(path.legs[1].estimate.departure_epoch_seconds, 210);
        assert_eq!(path.legs[1].estimate.arrival_epoch_seconds, 276);
        assert_eq!(path.final_arrival_epoch_seconds, 276);
        assert_eq!(path.total_elapsed_seconds, 126);
    }

    #[test]
    fn path_composition_rejects_disconnected_legs_and_empty_paths() {
        let first = FifoPathLegV1 {
            edge_id: "edge-a".into(),
            from_node_id: "A".into(),
            to_node_id: "B".into(),
            pre_leg_delay_seconds: 0,
            arrival_function: function(&[(100, 150), (200, 250)]).unwrap(),
        };
        let disconnected = FifoPathLegV1 {
            edge_id: "edge-c".into(),
            from_node_id: "C".into(),
            to_node_id: "D".into(),
            pre_leg_delay_seconds: 0,
            arrival_function: function(&[(100, 150), (200, 250)]).unwrap(),
        };

        assert_eq!(
            evaluate_fifo_path(&[], 150, 55, "UTC_UNIX_SECONDS_V1"),
            Err(FifoPathEvaluationError::EmptyPath)
        );
        assert!(matches!(
            evaluate_fifo_path(&[first, disconnected], 150, 55, "UTC_UNIX_SECONDS_V1"),
            Err(FifoPathEvaluationError::DisconnectedLegs { .. })
        ));
    }

    #[test]
    fn path_composition_rejects_delay_arithmetic_overflow() {
        let leg = FifoPathLegV1 {
            edge_id: "edge-a".into(),
            from_node_id: "A".into(),
            to_node_id: "B".into(),
            pre_leg_delay_seconds: 1,
            arrival_function: function(&[(100, 150), (200, 250)]).unwrap(),
        };
        assert_eq!(
            evaluate_fifo_path(&[leg], u64::MAX, 55, "UTC_UNIX_SECONDS_V1"),
            Err(FifoPathEvaluationError::ArithmeticOverflow)
        );
    }

    #[test]
    fn mismatched_time_profile_is_rejected_before_interpolation() {
        let function = function(&[(100, 151), (200, 252)]).unwrap();
        assert_eq!(
            function.arrival_at(150, 55, "LOCAL_WALL_CLOCK_UNSPECIFIED"),
            Err(ArrivalFunctionError::TimeProfileMismatch)
        );
    }

    #[test]
    fn repeated_evaluation_is_deterministic_for_immutable_inputs() {
        let forward = function(&[(100, 150), (200, 270), (300, 330)]).unwrap();
        let reversed = function(&[(100, 150), (200, 270), (300, 330)]).unwrap();
        assert_eq!(forward, reversed);
        assert_eq!(
            forward.arrival_at(175, 55, "UTC_UNIX_SECONDS_V1"),
            reversed.arrival_at(175, 55, "UTC_UNIX_SECONDS_V1")
        );
    }
}
