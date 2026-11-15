//! Tiny finite reference oracle for AeroWeft surveillance observability claims.
//!
//! It checks whether a set of scoped sensor claims supports observability for
//! one traffic class, spatial cell and time tick. It distinguishes coverage
//! claims from observed tracks and ensures that no report never becomes an
//! empty-airspace assertion.
//!
//! This is not an ADS-B, radar, TDoA, EO/IR or acoustic detector, sensor model,
//! DAA system, collision-avoidance algorithm, surveillance service or flight
//! authority. All values and identifiers in the tests are synthetic.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeSet;

/// Traffic class covered by a surveillance claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TrafficClass {
    /// Aircraft that emit a supported cooperative position report.
    CooperativeCrewed,
    /// Crewed aircraft not assumed to emit a cooperative position report.
    NonCooperativeCrewed,
    /// Uncrewed traffic, only where the claim profile explicitly supports it.
    UncrewedProfileQualified,
    /// Target class is unresolved.
    Unknown,
}

/// Sensor modality represented by a synthetic coverage claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensorModality {
    /// Automatic Dependent Surveillance-Broadcast.
    Adsb,
    /// Time Difference of Arrival / multilateration receiver network.
    TdoaMlat,
    /// Electro-optical / infrared sensor.
    Eoir,
    /// Ground-based radar.
    GroundRadar,
}

/// Coverage evidence status for a traffic class/cell/time request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoverageStatus {
    /// Admitted evidence meets the exact declared profile for this class/cell/time.
    QualifiedCoverage,
    /// Some qualified evidence exists, but the declared profile's family floor is not met.
    PartialCoverage,
    /// A relevant evidence dependency is unknown, incomplete, stale or contradictory.
    Indeterminate,
    /// Upstream sensor/service is explicitly degraded under its declared profile.
    Degraded,
    /// No admitted claim covers this class/cell/time or a declared condition is out of scope.
    NotQualified,
}

/// Result of an observation/track lookup, kept separate from coverage status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationStatus {
    /// A current, integrity-qualified track matches an admitted claim.
    QualifiedTrack,
    /// No report/track arrived for the requested class/cell/time.
    NoObservation,
    /// A report exists but is stale, invalid, unsupported or not covered by an admitted claim.
    InvalidOrStaleTrack,
}

/// Evidence class attached to one coverage claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoverageEvidenceClass {
    /// Captured from a declared measurement campaign.
    Measured,
    /// Admitted under an external qualification/certification scope.
    Certified,
    /// Derived from a documented model with declared applicability.
    Modelled,
    /// Fictional data used only for deterministic tests.
    Synthetic,
    /// Evidence method is absent or unresolved.
    Unknown,
}

/// A finite coverage claim for one traffic class, cell and half-open validity interval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoverageClaim {
    /// Stable claim identifier.
    pub claim_id: &'static str,
    /// Sensor/receiver source identity.
    pub source_id: &'static str,
    /// Sensor modality.
    pub modality: SensorModality,
    /// Exact evidence method/class.
    pub evidence_class: CoverageEvidenceClass,
    /// Sensor/coverage profile revision identifier.
    pub profile_id: &'static str,
    /// Environmental operating profile for which the claim applies.
    pub environment_profile: &'static str,
    /// Traffic class the claim explicitly supports.
    pub traffic_class: TrafficClass,
    /// Synthetic spatial cell identity.
    pub cell_id: &'static str,
    /// Inclusive first validity tick.
    pub start_tick: u16,
    /// Exclusive final validity tick.
    pub end_tick: u16,
    /// Whether the source profile supports the claimed target class/range in this cell.
    pub coverage_qualified: bool,
    /// Whether current weather, lighting, clutter and obstruction fit the claim profile.
    pub environment_qualified: bool,
    /// Whether the upstream sensor/service is explicitly in a degraded state.
    pub upstream_degraded: bool,
    /// Whether PNT and time dependencies are qualified for this claim.
    pub pnt_time_qualified: bool,
    /// Whether the relevant source dependency inventory is complete.
    pub dependency_inventory_complete: bool,
    /// Independent failure-domain family identifier, if known.
    pub independent_family_id: Option<&'static str>,
}

/// One observed track/report presented for qualification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Observation {
    /// Stable track/report identity.
    pub observation_id: &'static str,
    /// Source sensor/receiver identity.
    pub source_id: &'static str,
    /// Traffic class attributed by the observation source.
    pub traffic_class: TrafficClass,
    /// Synthetic spatial cell identity.
    pub cell_id: &'static str,
    /// Observation tick.
    pub tick: u16,
    /// Whether the report is current within the operation profile.
    pub current: bool,
    /// Whether integrity, time and source checks have qualified this report.
    pub integrity_qualified: bool,
}

/// Result of the scoped surveillance-observability evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservabilityAssessment {
    /// Coverage status for the requested class/cell/time.
    pub coverage_status: CoverageStatus,
    /// Track/report status, independently of coverage.
    pub observation_status: ObservationStatus,
    /// Number of distinct, fully qualified failure-domain families.
    pub independent_family_count: usize,
    /// Stable claim IDs that contributed to the admitted family set.
    pub qualified_claim_ids: Vec<&'static str>,
    /// Distinct failure-domain family IDs counted for this assessment.
    pub independent_family_ids: Vec<&'static str>,
    /// Whether possible traffic occupancy remains unresolved for downstream planning.
    pub unresolved_possible_occupancy: bool,
}

/// Typed error for invalid evaluation requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservabilityError {
    /// A zero family floor cannot define a meaningful coverage policy.
    InvalidMinimumIndependentFamilies,
}

fn modality_supports_traffic_class(modality: SensorModality, traffic_class: TrafficClass) -> bool {
    match modality {
        // ADS-B and TDoA/MLAT derive their surveillance evidence from
        // cooperative transponder emissions in this benchmark's claim model.
        SensorModality::Adsb | SensorModality::TdoaMlat => {
            traffic_class == TrafficClass::CooperativeCrewed
        }
        // EO/IR and radar may support non-cooperative targets only where the
        // exact source profile explicitly claims and qualifies that class.
        SensorModality::Eoir | SensorModality::GroundRadar => {
            traffic_class != TrafficClass::Unknown
        }
    }
}

/// Evaluate class-specific coverage and one optional observation.
///
/// A claim only contributes if its traffic class, cell and half-open validity
/// interval match the request. A report is a qualified track only if it matches
/// a currently valid, fully qualified source claim of the same class/cell/time.
///
/// A coverage result never implies that a no-report cell is physically empty.
/// No report always retains unresolved possible occupancy.
pub fn evaluate(
    claims: &[CoverageClaim],
    observation: Option<&Observation>,
    traffic_class: TrafficClass,
    cell_id: &str,
    tick: u16,
    minimum_independent_families: usize,
) -> Result<ObservabilityAssessment, ObservabilityError> {
    if minimum_independent_families == 0 {
        return Err(ObservabilityError::InvalidMinimumIndependentFamilies);
    }
    if traffic_class == TrafficClass::Unknown {
        return Ok(ObservabilityAssessment {
            coverage_status: CoverageStatus::NotQualified,
            observation_status: if observation.is_some() {
                ObservationStatus::InvalidOrStaleTrack
            } else {
                ObservationStatus::NoObservation
            },
            independent_family_count: 0,
            qualified_claim_ids: Vec::new(),
            independent_family_ids: Vec::new(),
            unresolved_possible_occupancy: true,
        });
    }

    let relevant: Vec<_> = claims
        .iter()
        .filter(|claim| {
            claim.traffic_class == traffic_class
                && claim.cell_id == cell_id
                && claim.start_tick <= tick
                && tick < claim.end_tick
        })
        .collect();

    let mut independent_families = BTreeSet::new();
    let mut qualified_claim_ids = Vec::new();
    let mut unknown_dependency = false;
    let mut upstream_degraded = false;

    for claim in &relevant {
        if !modality_supports_traffic_class(claim.modality, traffic_class) {
            continue;
        }
        if claim.upstream_degraded {
            upstream_degraded = true;
            continue;
        }
        if !claim.coverage_qualified || !claim.environment_qualified {
            continue;
        }
        if claim.claim_id.trim().is_empty()
            || claim.source_id.trim().is_empty()
            || claim.evidence_class == CoverageEvidenceClass::Unknown
            || claim.profile_id.trim().is_empty()
            || claim.environment_profile.trim().is_empty()
        {
            unknown_dependency = true;
            continue;
        }
        if !claim.pnt_time_qualified || !claim.dependency_inventory_complete {
            unknown_dependency = true;
            continue;
        }
        let Some(family_id) = claim.independent_family_id else {
            unknown_dependency = true;
            continue;
        };
        if family_id.trim().is_empty() {
            unknown_dependency = true;
            continue;
        }
        independent_families.insert(family_id);
        qualified_claim_ids.push(claim.claim_id);
    }

    qualified_claim_ids.sort_unstable();
    qualified_claim_ids.dedup();
    let independent_family_ids: Vec<_> = independent_families.iter().copied().collect();
    let independent_family_count = independent_family_ids.len();

    let coverage_status = if independent_family_count >= minimum_independent_families {
        CoverageStatus::QualifiedCoverage
    } else if unknown_dependency {
        CoverageStatus::Indeterminate
    } else if upstream_degraded {
        CoverageStatus::Degraded
    } else if independent_family_count > 0 {
        CoverageStatus::PartialCoverage
    } else {
        CoverageStatus::NotQualified
    };

    let observation_status = match observation {
        None => ObservationStatus::NoObservation,
        Some(report) => {
            let matching_claim = relevant.iter().any(|claim| {
                claim.source_id == report.source_id
                    && claim.coverage_qualified
                    && claim.environment_qualified
                    && !claim.upstream_degraded
                    && claim.evidence_class != CoverageEvidenceClass::Unknown
                    && !claim.profile_id.trim().is_empty()
                    && !claim.environment_profile.trim().is_empty()
                    && claim.pnt_time_qualified
                    && claim.dependency_inventory_complete
                    && claim
                        .independent_family_id
                        .is_some_and(|family| !family.trim().is_empty())
            });

            if report.traffic_class == traffic_class
                && report.cell_id == cell_id
                && report.tick == tick
                && report.current
                && report.integrity_qualified
                && matching_claim
            {
                ObservationStatus::QualifiedTrack
            } else {
                ObservationStatus::InvalidOrStaleTrack
            }
        }
    };

    let unresolved_possible_occupancy = observation_status != ObservationStatus::QualifiedTrack
        || coverage_status != CoverageStatus::QualifiedCoverage;

    Ok(ObservabilityAssessment {
        coverage_status,
        observation_status,
        independent_family_count,
        qualified_claim_ids,
        independent_family_ids,
        unresolved_possible_occupancy,
    })
}
