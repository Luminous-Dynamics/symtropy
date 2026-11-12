// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const FIXTURE_SCHEMA_V1: &str = "luminous.civ_infra.corridor_fixture.v0.1";
pub const REPORT_SCHEMA_V1: &str = "luminous.civ_infra.corridor_report.v0.1";
const CLAIM_CEILING: &str = "synthetic_case_only_not_real_world_qualified";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CorridorFixture {
    pub schema_version: String,
    pub fixture_id: String,
    pub coordinate_frame: CoordinateFrame,
    pub requirements: Requirements,
    pub utilities: Vec<UtilityAsset>,
    pub candidates: Vec<CorridorCandidate>,
    pub resources: Vec<ResourceCapacity>,
    pub work_packages: Vec<WorkPackage>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CoordinateFrame {
    pub id: String,
    pub horizontal_unit: String,
    pub vertical_unit: String,
    pub vertical_datum: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Requirements {
    pub max_grade_percent: f64,
    pub max_earthworks_m3: f64,
    pub min_vertical_clearance_m: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AxisEnvelope {
    pub station_start_m: f64,
    pub station_end_m: f64,
    pub lateral_min_m: f64,
    pub lateral_max_m: f64,
    pub elevation_min_m: Option<f64>,
    pub elevation_max_m: Option<f64>,
    pub uncertainty_m: f64,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceState {
    KnownSynthetic,
    BoundedUnknown,
    UnboundedUnknown,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UtilityAsset {
    pub id: String,
    pub coordinate_frame_id: String,
    pub evidence: EvidenceState,
    pub occupancy: Option<AxisEnvelope>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DrainageCase {
    pub scenario_id: String,
    pub required_flow_lps: f64,
    pub capacity_lps: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProfilePoint {
    pub station_m: f64,
    pub elevation_m: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CrossSection {
    pub station_m: f64,
    pub cut_area_m2: f64,
    pub fill_area_m2: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CorridorCandidate {
    pub id: String,
    pub length_m: f64,
    /// Redundant declaration, checked against the longitudinal profile.
    pub declared_max_grade_percent: f64,
    pub profile_points: Vec<ProfilePoint>,
    /// Redundant declaration, checked against trapezoidal integration of cut/fill areas.
    pub declared_earthworks_m3: f64,
    pub cross_sections: Vec<CrossSection>,
    pub road_envelopes: Vec<AxisEnvelope>,
    pub drainage_scenarios: Vec<DrainageCase>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ResourceCapacity {
    pub id: String,
    pub capacity_units: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WorkPackage {
    pub id: String,
    pub resource_uses: Vec<ResourceUse>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ResourceUse {
    pub resource_id: String,
    pub start_tick: u64,
    pub end_tick: u64,
    pub units: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Pass,
    Violation,
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CandidateDisposition {
    FeasibleForSyntheticCase,
    Infeasible,
    EvidenceInconsistent,
    UnknownReviewRequired,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Finding {
    pub code: String,
    pub status: CheckStatus,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct CandidateMetrics {
    pub route_length_m: f64,
    pub derived_max_grade_percent: f64,
    pub derived_cut_volume_m3: f64,
    pub derived_fill_volume_m3: f64,
    pub derived_total_earthworks_m3: f64,
    /// Minimum declared drainage capacity minus demand across every fixture scenario.
    pub minimum_drainage_reserve_lps: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct CandidateResult {
    pub candidate_id: String,
    pub disposition: CandidateDisposition,
    pub metrics: CandidateMetrics,
    pub findings: Vec<Finding>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ScheduleConflict {
    pub resource_id: String,
    pub start_tick: u64,
    pub end_tick: u64,
    pub observed_demand_units: u64,
    pub capacity_units: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct BenchmarkReport {
    pub schema_version: String,
    pub fixture_id: String,
    pub input_sha256: String,
    pub claim_ceiling: String,
    pub candidates: Vec<CandidateResult>,
    pub pareto_frontier_candidate_ids: Vec<String>,
    pub schedule_conflicts: Vec<ScheduleConflict>,
    pub resource_schedule_feasible: bool,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("{digest:x}")
}

fn finite_nonnegative(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

// Report numerical metrics at a stable precision so equivalent but
// independently ordered floating-point calculations compare deterministically.
fn normalize_metric(value: f64) -> f64 {
    if value.is_finite() {
        let scaled = value * 1.0e9;
        if scaled.is_finite() {
            scaled.round() / 1.0e9
        } else {
            value
        }
    } else {
        value
    }
}

fn candidate_metrics(
    route_length_m: f64,
    max_grade_percent: f64,
    cut_volume_m3: f64,
    fill_volume_m3: f64,
    minimum_drainage_reserve_lps: f64,
) -> CandidateMetrics {
    CandidateMetrics {
        route_length_m: normalize_metric(route_length_m),
        derived_max_grade_percent: normalize_metric(max_grade_percent),
        derived_cut_volume_m3: normalize_metric(cut_volume_m3),
        derived_fill_volume_m3: normalize_metric(fill_volume_m3),
        derived_total_earthworks_m3: normalize_metric(cut_volume_m3 + fill_volume_m3),
        minimum_drainage_reserve_lps: normalize_metric(minimum_drainage_reserve_lps),
    }
}

fn minimum_drainage_reserve_lps(scenarios: &[DrainageCase]) -> Option<f64> {
    if scenarios.is_empty() {
        return None;
    }
    let mut minimum = f64::INFINITY;
    for scenario in scenarios {
        let reserve = scenario.capacity_lps - scenario.required_flow_lps;
        if !reserve.is_finite() {
            return None;
        }
        minimum = minimum.min(reserve);
    }
    Some(minimum)
}

fn maximum_grade_percent(points: &[ProfilePoint]) -> Option<f64> {
    if points.len() < 2 {
        return None;
    }
    let mut maximum: f64 = 0.0;
    for pair in points.windows(2) {
        let run = pair[1].station_m - pair[0].station_m;
        if !run.is_finite() || run <= 0.0 {
            return None;
        }
        let rise = pair[1].elevation_m - pair[0].elevation_m;
        let grade = (rise / run * 100.0).abs();
        if !grade.is_finite() {
            return None;
        }
        maximum = maximum.max(grade);
    }
    Some(maximum)
}

fn maximum_earthworks_volume(sections: &[CrossSection]) -> Option<(f64, f64)> {
    if sections.len() < 2 {
        return None;
    }
    let mut cut_volume_m3 = 0.0;
    let mut fill_volume_m3 = 0.0;
    for pair in sections.windows(2) {
        let interval_m = pair[1].station_m - pair[0].station_m;
        if !interval_m.is_finite() || interval_m <= 0.0 {
            return None;
        }
        let cut = interval_m * (pair[0].cut_area_m2 + pair[1].cut_area_m2) / 2.0;
        let fill = interval_m * (pair[0].fill_area_m2 + pair[1].fill_area_m2) / 2.0;
        if !cut.is_finite() || !fill.is_finite() {
            return None;
        }
        cut_volume_m3 += cut;
        fill_volume_m3 += fill;
        if !cut_volume_m3.is_finite() || !fill_volume_m3.is_finite() {
            return None;
        }
    }
    Some((cut_volume_m3, fill_volume_m3))
}

fn validate_envelope(
    envelope: &AxisEnvelope,
    require_elevation: bool,
    context: &str,
    errors: &mut Vec<String>,
) {
    if !envelope.station_start_m.is_finite()
        || !envelope.station_end_m.is_finite()
        || envelope.station_start_m >= envelope.station_end_m
    {
        errors.push(format!("{context}: invalid station interval"));
    }
    if !envelope.lateral_min_m.is_finite()
        || !envelope.lateral_max_m.is_finite()
        || envelope.lateral_min_m >= envelope.lateral_max_m
    {
        errors.push(format!("{context}: invalid lateral interval"));
    }
    if !finite_nonnegative(envelope.uncertainty_m) {
        errors.push(format!(
            "{context}: uncertainty must be finite and non-negative"
        ));
    }

    match (envelope.elevation_min_m, envelope.elevation_max_m) {
        (Some(minimum), Some(maximum))
            if minimum.is_finite() && maximum.is_finite() && minimum < maximum => {}
        (None, None) if !require_elevation => {}
        _ => errors.push(format!(
            "{context}: invalid or unsupported elevation interval"
        )),
    }
}

pub fn validate_fixture(fixture: &CorridorFixture) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    if fixture.schema_version != FIXTURE_SCHEMA_V1 {
        errors.push(format!(
            "unsupported schema_version: {}",
            fixture.schema_version
        ));
    }
    if fixture.fixture_id.trim().is_empty() {
        errors.push("fixture_id must not be empty".to_string());
    }
    if fixture.coordinate_frame.id.trim().is_empty()
        || fixture.coordinate_frame.vertical_datum.trim().is_empty()
    {
        errors.push("coordinate frame id and vertical datum must be explicit".to_string());
    }
    // V0 intentionally supports one declared local metric frame only. It does
    // not silently transform other units, CRSs, or vertical datums.
    if fixture.coordinate_frame.horizontal_unit != "metre"
        || fixture.coordinate_frame.vertical_unit != "metre"
    {
        errors.push(
            "V0 requires explicit metre units for horizontal and vertical coordinates".to_string(),
        );
    }
    if !finite_nonnegative(fixture.requirements.max_grade_percent)
        || !finite_nonnegative(fixture.requirements.max_earthworks_m3)
        || !finite_nonnegative(fixture.requirements.min_vertical_clearance_m)
    {
        errors.push("requirements must be finite and non-negative".to_string());
    }

    let mut candidate_ids = BTreeSet::new();
    if fixture.candidates.is_empty() {
        errors.push("fixture must contain at least one candidate".to_string());
    }
    for candidate in &fixture.candidates {
        if candidate.id.trim().is_empty() || !candidate_ids.insert(candidate.id.clone()) {
            errors.push(format!("empty or duplicate candidate id: {}", candidate.id));
        }
        if !finite_nonnegative(candidate.length_m)
            || candidate.length_m <= 0.0
            || !finite_nonnegative(candidate.declared_max_grade_percent)
            || !finite_nonnegative(candidate.declared_earthworks_m3)
        {
            errors.push(format!(
                "candidate {} contains invalid numeric input",
                candidate.id
            ));
        }
        if candidate.drainage_scenarios.is_empty() {
            errors.push(format!(
                "candidate {} needs at least one drainage scenario",
                candidate.id
            ));
        }
        let mut drainage_scenario_ids = BTreeSet::new();
        for scenario in &candidate.drainage_scenarios {
            if scenario.scenario_id.trim().is_empty()
                || !drainage_scenario_ids.insert(scenario.scenario_id.clone())
            {
                errors.push(format!(
                    "candidate {} has an empty or duplicate drainage scenario id",
                    candidate.id
                ));
            }
            if !finite_nonnegative(scenario.required_flow_lps)
                || !finite_nonnegative(scenario.capacity_lps)
            {
                errors.push(format!(
                    "candidate {} drainage scenario {} has invalid numeric input",
                    candidate.id, scenario.scenario_id
                ));
            }
        }
        if candidate.profile_points.len() < 2 {
            errors.push(format!(
                "candidate {} needs at least two longitudinal profile points",
                candidate.id
            ));
        }
        if candidate
            .profile_points
            .first()
            .is_some_and(|point| point.station_m.abs() > 1.0e-9)
        {
            errors.push(format!(
                "candidate {} profile must start at station zero",
                candidate.id
            ));
        }
        let mut previous_station = None;
        for (index, point) in candidate.profile_points.iter().enumerate() {
            if !point.station_m.is_finite()
                || !point.elevation_m.is_finite()
                || point.station_m < 0.0
                || point.station_m > candidate.length_m
            {
                errors.push(format!(
                    "candidate {} profile point {index} has invalid station/elevation",
                    candidate.id
                ));
            }
            if previous_station.is_some_and(|station| point.station_m <= station) {
                errors.push(format!(
                    "candidate {} profile stations must be strictly increasing",
                    candidate.id
                ));
            }
            previous_station = Some(point.station_m);
        }
        if let Some(last) = candidate.profile_points.last() {
            if (last.station_m - candidate.length_m).abs() > 1.0e-9 {
                errors.push(format!(
                    "candidate {} profile must terminate at its declared length",
                    candidate.id
                ));
            }
        }
        if maximum_grade_percent(&candidate.profile_points).is_none() {
            errors.push(format!(
                "candidate {} longitudinal profile cannot produce a finite grade",
                candidate.id
            ));
        }
        if candidate.cross_sections.len() < 2 {
            errors.push(format!(
                "candidate {} needs at least two cross-sections",
                candidate.id
            ));
        }
        if candidate
            .cross_sections
            .first()
            .is_some_and(|section| section.station_m.abs() > 1.0e-9)
        {
            errors.push(format!(
                "candidate {} cross-sections must start at station zero",
                candidate.id
            ));
        }
        let mut previous_section_station = None;
        for (index, section) in candidate.cross_sections.iter().enumerate() {
            if !section.station_m.is_finite()
                || section.station_m < 0.0
                || section.station_m > candidate.length_m
                || !finite_nonnegative(section.cut_area_m2)
                || !finite_nonnegative(section.fill_area_m2)
            {
                errors.push(format!(
                    "candidate {} cross-section {index} has invalid station/area",
                    candidate.id
                ));
            }
            if previous_section_station.is_some_and(|station| section.station_m <= station) {
                errors.push(format!(
                    "candidate {} cross-section stations must be strictly increasing",
                    candidate.id
                ));
            }
            previous_section_station = Some(section.station_m);
        }
        if let Some(last) = candidate.cross_sections.last() {
            if (last.station_m - candidate.length_m).abs() > 1.0e-9 {
                errors.push(format!(
                    "candidate {} cross-sections must terminate at its declared length",
                    candidate.id
                ));
            }
        }

        if candidate.road_envelopes.is_empty() {
            errors.push(format!("candidate {} has no road envelopes", candidate.id));
        }
        for (index, envelope) in candidate.road_envelopes.iter().enumerate() {
            validate_envelope(
                envelope,
                true,
                &format!("candidate {} road envelope {index}", candidate.id),
                &mut errors,
            );
        }
        let mut ordered_envelopes: Vec<&AxisEnvelope> = candidate.road_envelopes.iter().collect();
        ordered_envelopes
            .sort_by(|left, right| left.station_start_m.total_cmp(&right.station_start_m));
        if ordered_envelopes
            .first()
            .is_some_and(|envelope| envelope.station_start_m.abs() > 1.0e-9)
        {
            errors.push(format!(
                "candidate {} road envelopes must start at station zero",
                candidate.id
            ));
        }
        if ordered_envelopes
            .last()
            .is_some_and(|envelope| (envelope.station_end_m - candidate.length_m).abs() > 1.0e-9)
        {
            errors.push(format!(
                "candidate {} road envelopes must terminate at its declared length",
                candidate.id
            ));
        }
        for pair in ordered_envelopes.windows(2) {
            if pair[1].station_start_m > pair[0].station_end_m + 1.0e-9 {
                errors.push(format!(
                    "candidate {} road envelopes contain an uncovered station interval",
                    candidate.id
                ));
            }
        }
        for envelope in &ordered_envelopes {
            if let (Some(minimum), Some(maximum)) =
                (envelope.elevation_min_m, envelope.elevation_max_m)
            {
                for point in &candidate.profile_points {
                    if point.station_m >= envelope.station_start_m - 1.0e-9
                        && point.station_m <= envelope.station_end_m + 1.0e-9
                        && (point.elevation_m < minimum - 1.0e-9
                            || point.elevation_m > maximum + 1.0e-9)
                    {
                        errors.push(format!(
                            "candidate {} road envelope excludes profile station {}",
                            candidate.id, point.station_m
                        ));
                    }
                }
            }
        }
        if maximum_earthworks_volume(&candidate.cross_sections).is_none() {
            errors.push(format!(
                "candidate {} cross-sections cannot produce finite earthworks volumes",
                candidate.id
            ));
        }
    }

    let mut utility_ids = BTreeSet::new();
    for utility in &fixture.utilities {
        if utility.id.trim().is_empty() || !utility_ids.insert(utility.id.clone()) {
            errors.push(format!("empty or duplicate utility id: {}", utility.id));
        }
        if utility.coordinate_frame_id != fixture.coordinate_frame.id {
            errors.push(format!(
                "utility {} uses undeclared coordinate frame {}",
                utility.id, utility.coordinate_frame_id
            ));
        }
        if let Some(envelope) = &utility.occupancy {
            if utility.evidence == EvidenceState::UnboundedUnknown {
                errors.push(format!(
                    "utility {} has both unbounded-unknown state and a bounding envelope",
                    utility.id
                ));
            }
            validate_envelope(
                envelope,
                utility.evidence == EvidenceState::KnownSynthetic,
                &format!("utility {}", utility.id),
                &mut errors,
            );
        } else if utility.evidence != EvidenceState::UnboundedUnknown {
            errors.push(format!(
                "utility {} needs an envelope unless its spatial extent is unbounded unknown",
                utility.id
            ));
        }
    }

    let mut resource_capacities = BTreeMap::new();
    for resource in &fixture.resources {
        if resource.id.trim().is_empty()
            || resource.capacity_units == 0
            || resource_capacities
                .insert(resource.id.clone(), resource.capacity_units)
                .is_some()
        {
            errors.push(format!(
                "empty, duplicate, or zero-capacity resource: {}",
                resource.id
            ));
        }
    }

    let mut work_package_ids = BTreeSet::new();
    for package in &fixture.work_packages {
        if package.id.trim().is_empty() || !work_package_ids.insert(package.id.clone()) {
            errors.push(format!(
                "empty or duplicate work-package id: {}",
                package.id
            ));
        }
        for usage in &package.resource_uses {
            if !resource_capacities.contains_key(&usage.resource_id) {
                errors.push(format!(
                    "work package {} references unknown resource {}",
                    package.id, usage.resource_id
                ));
            }
            if usage.start_tick >= usage.end_tick || usage.units == 0 {
                errors.push(format!(
                    "work package {} has invalid resource interval or demand",
                    package.id
                ));
            }
        }
    }

    errors.sort();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn planar_ranges_overlap(a_start: f64, a_end: f64, b_start: f64, b_end: f64, padding: f64) -> bool {
    a_start <= b_end + padding && b_start <= a_end + padding
}

fn utility_check(
    candidate: &CorridorCandidate,
    utility: &UtilityAsset,
    required_clearance_m: f64,
) -> CheckStatus {
    let Some(utility_envelope) = utility.occupancy.as_ref() else {
        return CheckStatus::Unknown;
    };
    if utility.evidence == EvidenceState::UnboundedUnknown {
        return CheckStatus::Unknown;
    }

    let mut unknown_seen = false;
    for road in &candidate.road_envelopes {
        let padding = road.uncertainty_m + utility_envelope.uncertainty_m;
        let station_overlap = planar_ranges_overlap(
            road.station_start_m,
            road.station_end_m,
            utility_envelope.station_start_m,
            utility_envelope.station_end_m,
            padding,
        );
        let lateral_overlap = planar_ranges_overlap(
            road.lateral_min_m,
            road.lateral_max_m,
            utility_envelope.lateral_min_m,
            utility_envelope.lateral_max_m,
            padding,
        );
        if !station_overlap || !lateral_overlap {
            continue;
        }

        if utility.evidence == EvidenceState::BoundedUnknown {
            unknown_seen = true;
            continue;
        }

        let (Some(road_min), Some(road_max), Some(utility_min), Some(utility_max)) = (
            road.elevation_min_m,
            road.elevation_max_m,
            utility_envelope.elevation_min_m,
            utility_envelope.elevation_max_m,
        ) else {
            unknown_seen = true;
            continue;
        };

        let vertical_gap = if road_max < utility_min {
            utility_min - road_max
        } else if utility_max < road_min {
            road_min - utility_max
        } else {
            0.0
        };
        let conservative_gap = vertical_gap - padding;
        if conservative_gap < required_clearance_m {
            return CheckStatus::Violation;
        }
    }

    if unknown_seen {
        CheckStatus::Unknown
    } else {
        CheckStatus::Pass
    }
}

fn disposition_from_findings(findings: &[Finding]) -> CandidateDisposition {
    let physical_violation = findings.iter().any(|finding| {
        finding.status == CheckStatus::Violation
            && (finding.code == "grade_limit"
                || finding.code == "earthworks_budget"
                || finding.code.starts_with("drainage_capacity:")
                || finding.code.starts_with("utility_clearance:"))
    });
    if physical_violation {
        CandidateDisposition::Infeasible
    } else if findings.iter().any(|finding| {
        finding.status == CheckStatus::Violation
            && (finding.code == "declared_grade_matches_profile"
                || finding.code == "declared_earthworks_matches_sections")
    }) {
        CandidateDisposition::EvidenceInconsistent
    } else if findings
        .iter()
        .any(|finding| finding.status == CheckStatus::Unknown)
    {
        CandidateDisposition::UnknownReviewRequired
    } else {
        CandidateDisposition::FeasibleForSyntheticCase
    }
}

fn evaluate_candidate(fixture: &CorridorFixture, candidate: &CorridorCandidate) -> CandidateResult {
    let derived_grade = maximum_grade_percent(&candidate.profile_points).unwrap_or(f64::INFINITY);
    let declaration_matches =
        (derived_grade - candidate.declared_max_grade_percent).abs() <= 1.0e-9;
    let (cut_volume_m3, fill_volume_m3) =
        maximum_earthworks_volume(&candidate.cross_sections).unwrap_or((f64::INFINITY, 0.0));
    let derived_earthworks_m3 = cut_volume_m3 + fill_volume_m3;
    let earthworks_declaration_matches = derived_earthworks_m3.is_finite()
        && (derived_earthworks_m3 - candidate.declared_earthworks_m3).abs()
            <= 1.0e-6 * derived_earthworks_m3.abs().max(1.0);
    let mut findings = vec![
        Finding {
            code: "grade_limit".to_string(),
            status: if derived_grade <= fixture.requirements.max_grade_percent {
                CheckStatus::Pass
            } else {
                CheckStatus::Violation
            },
        },
        Finding {
            code: "declared_grade_matches_profile".to_string(),
            status: if declaration_matches {
                CheckStatus::Pass
            } else {
                CheckStatus::Violation
            },
        },
        Finding {
            code: "earthworks_budget".to_string(),
            status: if derived_earthworks_m3 <= fixture.requirements.max_earthworks_m3 {
                CheckStatus::Pass
            } else {
                CheckStatus::Violation
            },
        },
        Finding {
            code: "declared_earthworks_matches_sections".to_string(),
            status: if earthworks_declaration_matches {
                CheckStatus::Pass
            } else {
                CheckStatus::Violation
            },
        },
    ];
    let mut drainage_scenarios: Vec<&DrainageCase> = candidate.drainage_scenarios.iter().collect();
    drainage_scenarios.sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));
    for scenario in drainage_scenarios {
        findings.push(Finding {
            code: format!("drainage_capacity:{}", scenario.scenario_id),
            status: if scenario.required_flow_lps <= scenario.capacity_lps {
                CheckStatus::Pass
            } else {
                CheckStatus::Violation
            },
        });
    }

    let mut utilities: Vec<&UtilityAsset> = fixture.utilities.iter().collect();
    utilities.sort_by(|left, right| left.id.cmp(&right.id));
    for utility in utilities {
        findings.push(Finding {
            code: format!("utility_clearance:{}", utility.id),
            status: utility_check(
                candidate,
                utility,
                fixture.requirements.min_vertical_clearance_m,
            ),
        });
    }
    findings.sort_by(|left, right| left.code.cmp(&right.code));

    CandidateResult {
        candidate_id: candidate.id.clone(),
        disposition: disposition_from_findings(&findings),
        metrics: candidate_metrics(
            candidate.length_m,
            derived_grade,
            cut_volume_m3,
            fill_volume_m3,
            minimum_drainage_reserve_lps(&candidate.drainage_scenarios)
                .unwrap_or(f64::NEG_INFINITY),
        ),
        findings,
    }
}

fn candidate_dominates(left: &CandidateResult, right: &CandidateResult) -> bool {
    let no_worse = left.metrics.route_length_m <= right.metrics.route_length_m
        && left.metrics.derived_max_grade_percent <= right.metrics.derived_max_grade_percent
        && left.metrics.derived_total_earthworks_m3 <= right.metrics.derived_total_earthworks_m3
        && left.metrics.minimum_drainage_reserve_lps >= right.metrics.minimum_drainage_reserve_lps;
    let strictly_better = left.metrics.route_length_m < right.metrics.route_length_m
        || left.metrics.derived_max_grade_percent < right.metrics.derived_max_grade_percent
        || left.metrics.derived_total_earthworks_m3 < right.metrics.derived_total_earthworks_m3
        || left.metrics.minimum_drainage_reserve_lps > right.metrics.minimum_drainage_reserve_lps;
    no_worse && strictly_better
}

fn compute_pareto_frontier(candidates: &[CandidateResult]) -> Vec<String> {
    let eligible: Vec<&CandidateResult> = candidates
        .iter()
        .filter(|candidate| candidate.disposition == CandidateDisposition::FeasibleForSyntheticCase)
        .collect();
    let mut frontier: Vec<String> = eligible
        .iter()
        .filter(|candidate| {
            !eligible.iter().any(|other| {
                other.candidate_id != candidate.candidate_id
                    && candidate_dominates(other, candidate)
            })
        })
        .map(|candidate| candidate.candidate_id.clone())
        .collect();
    frontier.sort();
    frontier
}

fn schedule_conflicts(fixture: &CorridorFixture) -> Vec<ScheduleConflict> {
    let capacities: BTreeMap<&str, u32> = fixture
        .resources
        .iter()
        .map(|resource| (resource.id.as_str(), resource.capacity_units))
        .collect();
    let mut conflicts = Vec::new();

    for resource in &fixture.resources {
        let uses: Vec<&ResourceUse> = fixture
            .work_packages
            .iter()
            .flat_map(|package| package.resource_uses.iter())
            .filter(|usage| usage.resource_id == resource.id)
            .collect();
        let mut boundaries = BTreeSet::new();
        for usage in &uses {
            boundaries.insert(usage.start_tick);
            boundaries.insert(usage.end_tick);
        }
        let ticks: Vec<u64> = boundaries.into_iter().collect();

        for window in ticks.windows(2) {
            let start_tick = window[0];
            let end_tick = window[1];
            if start_tick == end_tick {
                continue;
            }
            // All work windows are half-open [start, end), so adjacent jobs
            // that meet at a tick do not overlap.
            let observed_demand_units: u64 = uses
                .iter()
                .filter(|usage| usage.start_tick <= start_tick && usage.end_tick >= end_tick)
                .map(|usage| u64::from(usage.units))
                .sum();
            let capacity_units = capacities[resource.id.as_str()];
            if observed_demand_units > u64::from(capacity_units) {
                conflicts.push(ScheduleConflict {
                    resource_id: resource.id.clone(),
                    start_tick,
                    end_tick,
                    observed_demand_units,
                    capacity_units,
                });
            }
        }
    }

    conflicts.sort_by(|left, right| {
        (&left.resource_id, left.start_tick, left.end_tick).cmp(&(
            &right.resource_id,
            right.start_tick,
            right.end_tick,
        ))
    });
    conflicts
}

pub fn evaluate_json(bytes: &[u8]) -> Result<BenchmarkReport, Vec<String>> {
    let fixture: CorridorFixture = serde_json::from_slice(bytes)
        .map_err(|error| vec![format!("invalid fixture JSON: {error}")])?;
    validate_fixture(&fixture)?;

    let mut candidates: Vec<CandidateResult> = fixture
        .candidates
        .iter()
        .map(|candidate| evaluate_candidate(&fixture, candidate))
        .collect();
    candidates.sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));

    let pareto_frontier_candidate_ids = compute_pareto_frontier(&candidates);
    let schedule_conflicts = schedule_conflicts(&fixture);
    Ok(BenchmarkReport {
        schema_version: REPORT_SCHEMA_V1.to_string(),
        fixture_id: fixture.fixture_id,
        input_sha256: sha256_hex(bytes),
        claim_ceiling: CLAIM_CEILING.to_string(),
        candidates,
        pareto_frontier_candidate_ids,
        resource_schedule_feasible: schedule_conflicts.is_empty(),
        schedule_conflicts,
    })
}

// Separate reference-check path. This intentionally derives expected results
// from the fixture again instead of trusting the evaluator's findings, report
// digest, candidate order, or schedule summary. It is diverse within the same
// crate, not an independently maintained implementation or third-party audit.
fn reference_ranges_overlap(
    first_min: f64,
    first_max: f64,
    second_min: f64,
    second_max: f64,
    uncertainty: f64,
) -> bool {
    !(first_max + uncertainty < second_min || second_max + uncertainty < first_min)
}

fn reference_utility_status(
    candidate: &CorridorCandidate,
    utility: &UtilityAsset,
    required_clearance_m: f64,
) -> CheckStatus {
    let Some(asset) = utility.occupancy.as_ref() else {
        return CheckStatus::Unknown;
    };
    if utility.evidence == EvidenceState::UnboundedUnknown {
        return CheckStatus::Unknown;
    }

    let mut unresolved = false;
    for corridor in &candidate.road_envelopes {
        let uncertainty = corridor.uncertainty_m + asset.uncertainty_m;
        let same_station_region = reference_ranges_overlap(
            corridor.station_start_m,
            corridor.station_end_m,
            asset.station_start_m,
            asset.station_end_m,
            uncertainty,
        );
        let same_lateral_region = reference_ranges_overlap(
            corridor.lateral_min_m,
            corridor.lateral_max_m,
            asset.lateral_min_m,
            asset.lateral_max_m,
            uncertainty,
        );
        if !(same_station_region && same_lateral_region) {
            continue;
        }

        if utility.evidence == EvidenceState::BoundedUnknown {
            unresolved = true;
            continue;
        }
        let (Some(corridor_low), Some(corridor_high), Some(asset_low), Some(asset_high)) = (
            corridor.elevation_min_m,
            corridor.elevation_max_m,
            asset.elevation_min_m,
            asset.elevation_max_m,
        ) else {
            unresolved = true;
            continue;
        };

        let nominal_separation = if corridor_low > asset_high {
            corridor_low - asset_high
        } else if asset_low > corridor_high {
            asset_low - corridor_high
        } else {
            0.0
        };
        if nominal_separation - uncertainty < required_clearance_m {
            return CheckStatus::Violation;
        }
    }

    if unresolved {
        CheckStatus::Unknown
    } else {
        CheckStatus::Pass
    }
}

fn reference_minimum_drainage_reserve_lps(scenarios: &[DrainageCase]) -> Option<f64> {
    scenarios
        .iter()
        .try_fold(None::<f64>, |minimum, scenario| {
            let reserve = scenario.capacity_lps - scenario.required_flow_lps;
            if !reserve.is_finite() {
                return None;
            }
            Some(Some(match minimum {
                Some(previous) => previous.min(reserve),
                None => reserve,
            }))
        })?
}

fn reference_maximum_grade_percent(points: &[ProfilePoint]) -> Option<f64> {
    if points.len() < 2 {
        return None;
    }
    points.windows(2).try_fold(0.0_f64, |maximum, pair| {
        let run = pair[1].station_m - pair[0].station_m;
        if !run.is_finite() || run <= 0.0 {
            return None;
        }
        let grade = 100.0 * ((pair[1].elevation_m - pair[0].elevation_m).abs() / run);
        if grade.is_finite() {
            Some(maximum.max(grade))
        } else {
            None
        }
    })
}

fn reference_earthworks_volumes(sections: &[CrossSection]) -> Option<(f64, f64)> {
    if sections.len() < 2 {
        return None;
    }
    sections
        .windows(2)
        .try_fold((0.0_f64, 0.0_f64), |volumes, pair| {
            let run = pair[1].station_m - pair[0].station_m;
            if !run.is_finite() || run <= 0.0 {
                return None;
            }
            // Sum each half-interval contribution separately, rather than calling
            // the producer's trapezoid helper.
            let half_run = run / 2.0;
            let cut = volumes.0 + half_run * pair[0].cut_area_m2 + half_run * pair[1].cut_area_m2;
            let fill =
                volumes.1 + half_run * pair[0].fill_area_m2 + half_run * pair[1].fill_area_m2;
            if cut.is_finite() && fill.is_finite() {
                Some((cut, fill))
            } else {
                None
            }
        })
}

fn reference_candidate_result(
    fixture: &CorridorFixture,
    candidate: &CorridorCandidate,
) -> CandidateResult {
    let mut findings = Vec::new();
    let derived_grade =
        reference_maximum_grade_percent(&candidate.profile_points).unwrap_or(f64::INFINITY);
    let (cut_volume_m3, fill_volume_m3) =
        reference_earthworks_volumes(&candidate.cross_sections).unwrap_or((f64::INFINITY, 0.0));
    let derived_earthworks_m3 = cut_volume_m3 + fill_volume_m3;

    findings.push(Finding {
        code: "grade_limit".to_string(),
        status: if derived_grade > fixture.requirements.max_grade_percent {
            CheckStatus::Violation
        } else {
            CheckStatus::Pass
        },
    });
    findings.push(Finding {
        code: "declared_grade_matches_profile".to_string(),
        status: if (derived_grade - candidate.declared_max_grade_percent).abs() <= 1.0e-9 {
            CheckStatus::Pass
        } else {
            CheckStatus::Violation
        },
    });
    findings.push(Finding {
        code: "earthworks_budget".to_string(),
        status: if derived_earthworks_m3 > fixture.requirements.max_earthworks_m3 {
            CheckStatus::Violation
        } else {
            CheckStatus::Pass
        },
    });
    findings.push(Finding {
        code: "declared_earthworks_matches_sections".to_string(),
        status: if derived_earthworks_m3.is_finite()
            && (derived_earthworks_m3 - candidate.declared_earthworks_m3).abs()
                <= 1.0e-6 * derived_earthworks_m3.abs().max(1.0)
        {
            CheckStatus::Pass
        } else {
            CheckStatus::Violation
        },
    });
    let mut drainage_scenarios: Vec<&DrainageCase> = candidate.drainage_scenarios.iter().collect();
    drainage_scenarios.sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));
    for scenario in drainage_scenarios {
        findings.push(Finding {
            code: format!("drainage_capacity:{}", scenario.scenario_id),
            status: if scenario.required_flow_lps > scenario.capacity_lps {
                CheckStatus::Violation
            } else {
                CheckStatus::Pass
            },
        });
    }

    for asset in &fixture.utilities {
        findings.push(Finding {
            code: format!("utility_clearance:{}", asset.id),
            status: reference_utility_status(
                candidate,
                asset,
                fixture.requirements.min_vertical_clearance_m,
            ),
        });
    }
    findings.sort_by(|left, right| left.code.cmp(&right.code));

    let physical_violation = findings.iter().any(|finding| {
        finding.status == CheckStatus::Violation
            && (finding.code == "grade_limit"
                || finding.code == "earthworks_budget"
                || finding.code.starts_with("drainage_capacity:")
                || finding.code.starts_with("utility_clearance:"))
    });
    let evidence_inconsistent = findings.iter().any(|finding| {
        finding.status == CheckStatus::Violation
            && (finding.code == "declared_grade_matches_profile"
                || finding.code == "declared_earthworks_matches_sections")
    });
    let disposition = if physical_violation {
        CandidateDisposition::Infeasible
    } else if evidence_inconsistent {
        CandidateDisposition::EvidenceInconsistent
    } else if findings
        .iter()
        .any(|finding| finding.status == CheckStatus::Unknown)
    {
        CandidateDisposition::UnknownReviewRequired
    } else {
        CandidateDisposition::FeasibleForSyntheticCase
    };

    CandidateResult {
        candidate_id: candidate.id.clone(),
        disposition,
        metrics: candidate_metrics(
            candidate.length_m,
            derived_grade,
            cut_volume_m3,
            fill_volume_m3,
            reference_minimum_drainage_reserve_lps(&candidate.drainage_scenarios)
                .unwrap_or(f64::NEG_INFINITY),
        ),
        findings,
    }
}

fn reference_schedule_conflicts(fixture: &CorridorFixture) -> Vec<ScheduleConflict> {
    let mut expected = Vec::new();

    for resource in &fixture.resources {
        let assigned: Vec<&ResourceUse> = fixture
            .work_packages
            .iter()
            .flat_map(|package| package.resource_uses.iter())
            .filter(|usage| usage.resource_id == resource.id)
            .collect();
        let mut edges = BTreeSet::new();
        for usage in &assigned {
            edges.insert(usage.start_tick);
            edges.insert(usage.end_tick);
        }
        let edges: Vec<u64> = edges.into_iter().collect();
        for index in 0..edges.len().saturating_sub(1) {
            let start = edges[index];
            let end = edges[index + 1];
            let demand = assigned
                .iter()
                .filter(|usage| usage.start_tick < end && usage.end_tick > start)
                .fold(0_u64, |sum, usage| {
                    sum.saturating_add(u64::from(usage.units))
                });
            if demand > u64::from(resource.capacity_units) {
                expected.push(ScheduleConflict {
                    resource_id: resource.id.clone(),
                    start_tick: start,
                    end_tick: end,
                    observed_demand_units: demand,
                    capacity_units: resource.capacity_units,
                });
            }
        }
    }
    expected.sort_by(|left, right| {
        (&left.resource_id, left.start_tick, left.end_tick).cmp(&(
            &right.resource_id,
            right.start_tick,
            right.end_tick,
        ))
    });
    expected
}

fn metrics_match(actual: &CandidateMetrics, expected: &CandidateMetrics) -> bool {
    fn close(left: f64, right: f64) -> bool {
        left.is_finite()
            && right.is_finite()
            && (left - right).abs() <= 1.0e-8 * left.abs().max(right.abs()).max(1.0)
    }

    close(actual.route_length_m, expected.route_length_m)
        && close(
            actual.derived_max_grade_percent,
            expected.derived_max_grade_percent,
        )
        && close(actual.derived_cut_volume_m3, expected.derived_cut_volume_m3)
        && close(
            actual.derived_fill_volume_m3,
            expected.derived_fill_volume_m3,
        )
        && close(
            actual.derived_total_earthworks_m3,
            expected.derived_total_earthworks_m3,
        )
        && close(
            actual.minimum_drainage_reserve_lps,
            expected.minimum_drainage_reserve_lps,
        )
}

fn candidate_results_match(actual: &[CandidateResult], expected: &[CandidateResult]) -> bool {
    actual.len() == expected.len()
        && actual.iter().zip(expected).all(|(actual, expected)| {
            actual.candidate_id == expected.candidate_id
                && actual.disposition == expected.disposition
                && actual.findings == expected.findings
                && metrics_match(&actual.metrics, &expected.metrics)
        })
}

pub fn verify_report(input_bytes: &[u8], report: &BenchmarkReport) -> Result<(), Vec<String>> {
    let fixture: CorridorFixture = serde_json::from_slice(input_bytes).map_err(|error| {
        vec![format!(
            "invalid fixture JSON during reference check: {error}"
        )]
    })?;
    validate_fixture(&fixture)?;
    let mut errors = Vec::new();

    if report.schema_version != REPORT_SCHEMA_V1 {
        errors.push("report schema version mismatch".to_string());
    }
    if report.fixture_id != fixture.fixture_id {
        errors.push("report fixture_id does not match input".to_string());
    }
    if report.input_sha256 != sha256_hex(input_bytes) {
        errors.push("report input SHA-256 does not match exact input bytes".to_string());
    }
    if report.claim_ceiling != CLAIM_CEILING {
        errors.push("report claim ceiling was altered".to_string());
    }

    let mut expected_candidates: Vec<CandidateResult> = fixture
        .candidates
        .iter()
        .map(|candidate| reference_candidate_result(&fixture, candidate))
        .collect();
    expected_candidates.sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));
    if !candidate_results_match(&report.candidates, &expected_candidates) {
        errors.push(
            "candidate metrics/findings/dispositions disagree with reference recomputation"
                .to_string(),
        );
    }
    if report.pareto_frontier_candidate_ids != compute_pareto_frontier(&expected_candidates) {
        errors.push("Pareto frontier disagrees with reference candidate metrics".to_string());
    }

    let expected_schedule = reference_schedule_conflicts(&fixture);
    if report.schedule_conflicts != expected_schedule {
        errors
            .push("resource schedule conflicts disagree with reference recomputation".to_string());
    }
    if report.resource_schedule_feasible != expected_schedule.is_empty() {
        errors
            .push("resource_schedule_feasible disagrees with reference recomputation".to_string());
    }

    errors.sort();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_bytes() -> Vec<u8> {
        include_bytes!("../fixtures/corridor-v0.1.json").to_vec()
    }

    fn parse_fixture(bytes: &[u8]) -> CorridorFixture {
        serde_json::from_slice(bytes).expect("frozen fixture must parse")
    }

    #[test]
    fn frozen_fixture_has_expected_candidate_and_schedule_dispositions() {
        let bytes = fixture_bytes();
        let report = evaluate_json(&bytes).expect("fixture should evaluate");
        let dispositions: BTreeMap<&str, CandidateDisposition> = report
            .candidates
            .iter()
            .map(|result| (result.candidate_id.as_str(), result.disposition))
            .collect();

        assert_eq!(
            dispositions.get("A-shortest"),
            Some(&CandidateDisposition::Infeasible)
        );
        assert_eq!(
            dispositions.get("B-low-earthworks"),
            Some(&CandidateDisposition::Infeasible)
        );
        assert_eq!(
            dispositions.get("C-reference"),
            Some(&CandidateDisposition::FeasibleForSyntheticCase)
        );
        assert_eq!(
            dispositions.get("D-resilient-detour"),
            Some(&CandidateDisposition::FeasibleForSyntheticCase)
        );
        assert_eq!(
            report.pareto_frontier_candidate_ids,
            vec!["C-reference".to_string(), "D-resilient-detour".to_string()]
        );

        let shortest = report
            .candidates
            .iter()
            .find(|candidate| candidate.candidate_id == "A-shortest")
            .expect("shortest candidate exists");
        assert_eq!(shortest.metrics.derived_max_grade_percent, 8.0);
        assert_eq!(shortest.metrics.derived_cut_volume_m3, 3520.0);
        assert_eq!(shortest.metrics.derived_fill_volume_m3, 1760.0);
        assert_eq!(shortest.metrics.derived_total_earthworks_m3, 5280.0);
        assert_eq!(shortest.metrics.minimum_drainage_reserve_lps, -70.0);
        assert!(shortest.findings.iter().any(|finding| {
            finding.code == "grade_limit" && finding.status == CheckStatus::Violation
        }));
        assert!(shortest.findings.iter().any(|finding| {
            finding.code == "earthworks_budget" && finding.status == CheckStatus::Violation
        }));
        assert!(shortest.findings.iter().any(|finding| {
            finding.code == "drainage_capacity:design-storm-synthetic-01"
                && finding.status == CheckStatus::Violation
        }));
        assert!(shortest.findings.iter().any(|finding| {
            finding.code == "drainage_capacity:severe-storm-synthetic-01"
                && finding.status == CheckStatus::Violation
        }));
        assert!(shortest.findings.iter().any(|finding| {
            finding.code == "drainage_capacity:low-flow-synthetic-01"
                && finding.status == CheckStatus::Pass
        }));

        let low_earthworks = report
            .candidates
            .iter()
            .find(|candidate| candidate.candidate_id == "B-low-earthworks")
            .expect("low-earthworks candidate exists");
        assert_eq!(low_earthworks.metrics.minimum_drainage_reserve_lps, 10.0);
        assert!(low_earthworks.findings.iter().any(|finding| {
            finding.code == "utility_clearance:power-duct-known"
                && finding.status == CheckStatus::Violation
        }));

        let reference = report
            .candidates
            .iter()
            .find(|candidate| candidate.candidate_id == "C-reference")
            .expect("reference candidate exists");
        let detour = report
            .candidates
            .iter()
            .find(|candidate| candidate.candidate_id == "D-resilient-detour")
            .expect("detour candidate exists");
        assert_eq!(detour.metrics.route_length_m, 1200.0);
        assert_eq!(detour.metrics.derived_max_grade_percent, 2.5);
        assert_eq!(detour.metrics.derived_cut_volume_m3, 2400.0);
        assert_eq!(detour.metrics.derived_fill_volume_m3, 1800.0);
        assert_eq!(detour.metrics.derived_total_earthworks_m3, 4200.0);
        assert_eq!(detour.metrics.minimum_drainage_reserve_lps, 20.0);
        assert_eq!(reference.metrics.route_length_m, 1080.0);
        assert_eq!(reference.metrics.derived_max_grade_percent, 3.5);
        assert_eq!(reference.metrics.derived_cut_volume_m3, 2160.0);
        assert_eq!(reference.metrics.derived_fill_volume_m3, 1620.0);
        assert_eq!(reference.metrics.derived_total_earthworks_m3, 3780.0);
        assert_eq!(reference.metrics.minimum_drainage_reserve_lps, 10.0);
        assert!(
            reference
                .findings
                .iter()
                .all(|finding| finding.status == CheckStatus::Pass)
        );
        assert!(
            detour
                .findings
                .iter()
                .filter(|finding| finding.code.starts_with("drainage_capacity:"))
                .all(|finding| finding.status == CheckStatus::Pass)
        );
        assert!(!report.resource_schedule_feasible);
        assert_eq!(report.schedule_conflicts.len(), 1);
        assert_eq!(report.schedule_conflicts[0].resource_id, "excavator");
        assert_eq!(report.schedule_conflicts[0].start_tick, 2);
        assert_eq!(report.schedule_conflicts[0].end_tick, 3);
        verify_report(&bytes, &report).expect("reference checker should agree");
    }

    #[test]
    fn overlapping_bounded_unknown_utility_blocks_positive_feasibility_claim() {
        let bytes = fixture_bytes();
        let mut fixture = parse_fixture(&bytes);
        let unknown = fixture
            .utilities
            .iter_mut()
            .find(|utility| utility.id == "legacy-comms-unknown")
            .expect("unknown utility fixture exists");
        let occupancy = unknown
            .occupancy
            .as_mut()
            .expect("horizontal envelope exists");
        occupancy.station_start_m = 300.0;
        occupancy.station_end_m = 450.0;
        occupancy.lateral_min_m = 15.0;
        occupancy.lateral_max_m = 16.0;

        let modified = serde_json::to_vec(&fixture).expect("fixture serializes");
        let report = evaluate_json(&modified).expect("modified synthetic input remains valid");
        let reference = report
            .candidates
            .iter()
            .find(|candidate| candidate.candidate_id == "C-reference")
            .expect("reference candidate exists");
        assert_eq!(
            reference.disposition,
            CandidateDisposition::UnknownReviewRequired
        );
        assert!(reference.findings.iter().any(|finding| {
            finding.code == "utility_clearance:legacy-comms-unknown"
                && finding.status == CheckStatus::Unknown
        }));
        verify_report(&modified, &report).expect("reference checker should recompute unknown");
    }

    #[test]
    fn old_report_cannot_be_reused_after_exact_input_bytes_change() {
        let bytes = fixture_bytes();
        let report = evaluate_json(&bytes).expect("fixture should evaluate");
        let mut changed_bytes = bytes;
        changed_bytes.push(b' ');
        let errors = verify_report(&changed_bytes, &report).expect_err("old digest must fail");
        assert!(errors.iter().any(|error| error.contains("SHA-256")));
    }

    #[test]
    fn altered_candidate_disposition_is_rejected_by_reference_checker() {
        let bytes = fixture_bytes();
        let mut report = evaluate_json(&bytes).expect("fixture should evaluate");
        report
            .candidates
            .iter_mut()
            .find(|candidate| candidate.candidate_id == "C-reference")
            .expect("reference candidate exists")
            .disposition = CandidateDisposition::Infeasible;
        let errors = verify_report(&bytes, &report).expect_err("tampering must fail");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("reference recomputation"))
        );
    }

    #[test]
    fn coordinate_frame_mismatch_fails_closed() {
        let bytes = fixture_bytes();
        let mut fixture = parse_fixture(&bytes);
        fixture.utilities[0].coordinate_frame_id = "UNDECLARED_FRAME".to_string();
        let modified = serde_json::to_vec(&fixture).expect("fixture serializes");
        let errors = evaluate_json(&modified).expect_err("frame mismatch must be rejected");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("coordinate frame"))
        );
    }

    #[test]
    fn adjacent_half_open_work_windows_do_not_overcommit_capacity() {
        let bytes = fixture_bytes();
        let mut fixture = parse_fixture(&bytes);
        let second = fixture
            .work_packages
            .iter_mut()
            .find(|package| package.id == "drainage-crossing")
            .expect("second work package exists");
        second.resource_uses[0].start_tick = 4;
        second.resource_uses[0].end_tick = 6;
        let modified = serde_json::to_vec(&fixture).expect("fixture serializes");
        let report = evaluate_json(&modified).expect("adjacent windows are valid");
        assert!(report.resource_schedule_feasible);
        assert!(report.schedule_conflicts.is_empty());
        verify_report(&modified, &report).expect("reference checker should agree");
    }

    #[test]
    fn duplicate_candidate_ids_are_rejected() {
        let bytes = fixture_bytes();
        let mut fixture = parse_fixture(&bytes);
        fixture.candidates[1].id = fixture.candidates[0].id.clone();
        let modified = serde_json::to_vec(&fixture).expect("fixture serializes");
        let errors = evaluate_json(&modified).expect_err("duplicate IDs must fail");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("duplicate candidate id"))
        );
    }
    #[test]
    fn equivalent_input_order_preserves_semantics_but_changes_exact_byte_digest() {
        let original_bytes = fixture_bytes();
        let original = evaluate_json(&original_bytes).expect("original fixture evaluates");
        let mut reordered_fixture = parse_fixture(&original_bytes);
        reordered_fixture.candidates.reverse();
        reordered_fixture.utilities.reverse();
        reordered_fixture.resources.reverse();
        reordered_fixture.work_packages.reverse();
        for package in &mut reordered_fixture.work_packages {
            package.resource_uses.reverse();
        }
        let reordered_bytes =
            serde_json::to_vec(&reordered_fixture).expect("reordered fixture serializes");
        let reordered = evaluate_json(&reordered_bytes).expect("reordered fixture evaluates");

        assert_eq!(original.candidates, reordered.candidates);
        assert_eq!(original.schedule_conflicts, reordered.schedule_conflicts);
        assert_ne!(original.input_sha256, reordered.input_sha256);
        verify_report(&reordered_bytes, &reordered)
            .expect("reference checker should accept reordered semantic equivalent");
    }

    #[test]
    fn large_aggregate_resource_demand_cannot_wrap_u32_and_hide_overcommitment() {
        let bytes = fixture_bytes();
        let mut fixture = parse_fixture(&bytes);
        fixture
            .resources
            .iter_mut()
            .find(|resource| resource.id == "excavator")
            .expect("excavator resource exists")
            .capacity_units = u32::MAX;
        for package in &mut fixture.work_packages {
            for usage in &mut package.resource_uses {
                if usage.resource_id == "excavator" {
                    usage.units = u32::MAX;
                }
            }
        }

        let modified = serde_json::to_vec(&fixture).expect("fixture serializes");
        let report = evaluate_json(&modified).expect("large but representable demands evaluate");
        assert_eq!(report.schedule_conflicts.len(), 1);
        assert!(report.schedule_conflicts[0].observed_demand_units > u64::from(u32::MAX));
        verify_report(&modified, &report).expect("reference checker should match wide arithmetic");
    }

    #[test]
    fn exact_frozen_input_replays_to_identical_report_bytes() {
        let bytes = fixture_bytes();
        let first_report = evaluate_json(&bytes).expect("first evaluation succeeds");
        let second_report = evaluate_json(&bytes).expect("second evaluation succeeds");
        let first_bytes = serde_json::to_vec(&first_report).expect("first report serializes");
        let second_bytes = serde_json::to_vec(&second_report).expect("second report serializes");
        assert_eq!(first_bytes, second_bytes);
    }

    #[test]
    fn derived_profile_grade_overrules_forged_declared_grade() {
        let bytes = fixture_bytes();
        let mut fixture = parse_fixture(&bytes);
        let candidate = fixture
            .candidates
            .iter_mut()
            .find(|candidate| candidate.id == "C-reference")
            .expect("reference candidate exists");
        candidate.profile_points[1].elevation_m = -60.0;
        // Keep the coarse envelopes consistent with the edited profile while
        // leaving the claimed 3.5% grade untouched.
        candidate.road_envelopes[0].elevation_min_m = Some(-60.0);
        candidate.road_envelopes[1].elevation_min_m = Some(-60.0);
        candidate.road_envelopes[1].elevation_max_m = Some(-3.15);
        let modified = serde_json::to_vec(&fixture).expect("fixture serializes");
        let report = evaluate_json(&modified).expect("finite profile remains structurally valid");
        let result = report
            .candidates
            .iter()
            .find(|candidate| candidate.candidate_id == "C-reference")
            .expect("reference candidate result exists");
        assert_eq!(result.disposition, CandidateDisposition::Infeasible);
        assert!(result.findings.iter().any(|finding| {
            finding.code == "grade_limit" && finding.status == CheckStatus::Violation
        }));
        assert!(result.findings.iter().any(|finding| {
            finding.code == "declared_grade_matches_profile"
                && finding.status == CheckStatus::Violation
        }));
        verify_report(&modified, &report).expect("reference recomputation must derive grade too");
    }

    #[test]
    fn derived_cut_fill_volume_rejects_forged_declared_quantity() {
        let bytes = fixture_bytes();
        let mut fixture = parse_fixture(&bytes);
        let candidate = fixture
            .candidates
            .iter_mut()
            .find(|candidate| candidate.id == "C-reference")
            .expect("reference candidate exists");
        candidate.declared_earthworks_m3 = 0.0;
        let modified = serde_json::to_vec(&fixture).expect("fixture serializes");
        let report = evaluate_json(&modified).expect("fixture inputs remain structurally valid");
        let result = report
            .candidates
            .iter()
            .find(|candidate| candidate.candidate_id == "C-reference")
            .expect("reference candidate result exists");
        assert_eq!(
            result.disposition,
            CandidateDisposition::EvidenceInconsistent
        );
        assert!(result.findings.iter().any(|finding| {
            finding.code == "declared_earthworks_matches_sections"
                && finding.status == CheckStatus::Violation
        }));
        verify_report(&modified, &report)
            .expect("reference recomputation must derive cut/fill volume too");
    }

    #[test]
    fn road_envelope_must_cover_all_profile_stations_and_heights() {
        let bytes = fixture_bytes();
        let mut fixture = parse_fixture(&bytes);
        let candidate = fixture
            .candidates
            .iter_mut()
            .find(|candidate| candidate.id == "C-reference")
            .expect("reference candidate exists");
        candidate.road_envelopes[0].elevation_max_m = Some(-15.5);
        let modified = serde_json::to_vec(&fixture).expect("fixture serializes");
        let errors = evaluate_json(&modified).expect_err("inconsistent envelope must fail");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("road envelope excludes profile"))
        );
    }

    #[test]
    fn drainage_capacity_boundary_is_accepted_and_exceedance_is_rejected() {
        let bytes = fixture_bytes();
        let mut fixture = parse_fixture(&bytes);
        let candidate = fixture
            .candidates
            .iter_mut()
            .find(|candidate| candidate.id == "C-reference")
            .expect("reference candidate exists");
        let drainage_case = candidate
            .drainage_scenarios
            .iter_mut()
            .find(|scenario| scenario.scenario_id == "design-storm-synthetic-01")
            .expect("design storm case exists");
        drainage_case.required_flow_lps = drainage_case.capacity_lps;
        let boundary_bytes = serde_json::to_vec(&fixture).expect("fixture serializes");
        let boundary_report =
            evaluate_json(&boundary_bytes).expect("exact capacity boundary is valid");
        let boundary_result = boundary_report
            .candidates
            .iter()
            .find(|candidate| candidate.candidate_id == "C-reference")
            .expect("reference result exists");
        assert!(boundary_result.findings.iter().any(|finding| {
            finding.code.starts_with("drainage_capacity:") && finding.status == CheckStatus::Pass
        }));
        verify_report(&boundary_bytes, &boundary_report)
            .expect("reference check should accept exact capacity boundary");

        let candidate = fixture
            .candidates
            .iter_mut()
            .find(|candidate| candidate.id == "C-reference")
            .expect("reference candidate exists");
        let drainage_case = candidate
            .drainage_scenarios
            .iter_mut()
            .find(|scenario| scenario.scenario_id == "design-storm-synthetic-01")
            .expect("design storm case exists");
        drainage_case.required_flow_lps += 0.001;
        let exceeded_bytes = serde_json::to_vec(&fixture).expect("fixture serializes");
        let exceeded_report =
            evaluate_json(&exceeded_bytes).expect("finite exceedance input remains valid");
        let exceeded_result = exceeded_report
            .candidates
            .iter()
            .find(|candidate| candidate.candidate_id == "C-reference")
            .expect("reference result exists");
        assert_eq!(
            exceeded_result.disposition,
            CandidateDisposition::Infeasible
        );
        assert!(exceeded_result.findings.iter().any(|finding| {
            finding.code.starts_with("drainage_capacity:")
                && finding.status == CheckStatus::Violation
        }));
        verify_report(&exceeded_bytes, &exceeded_report)
            .expect("reference check should reject capacity exceedance");
    }

    #[test]
    fn altered_derived_metric_is_rejected_by_reference_checker() {
        let bytes = fixture_bytes();
        let mut report = evaluate_json(&bytes).expect("fixture should evaluate");
        let candidate = report
            .candidates
            .iter_mut()
            .find(|candidate| candidate.candidate_id == "C-reference")
            .expect("reference candidate exists");
        candidate.metrics.derived_total_earthworks_m3 = 0.0;
        let errors = verify_report(&bytes, &report).expect_err("metric tampering must fail");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("candidate metrics"))
        );
    }

    #[test]
    fn unbounded_unknown_utility_blocks_clearance_claim_even_without_geometry() {
        let bytes = fixture_bytes();
        let mut fixture = parse_fixture(&bytes);
        let unknown = fixture
            .utilities
            .iter_mut()
            .find(|utility| utility.id == "legacy-comms-unknown")
            .expect("unknown utility fixture exists");
        unknown.evidence = EvidenceState::UnboundedUnknown;
        unknown.occupancy = None;

        let modified = serde_json::to_vec(&fixture).expect("fixture serializes");
        let report = evaluate_json(&modified).expect("unbounded unknown is representable");
        let reference = report
            .candidates
            .iter()
            .find(|candidate| candidate.candidate_id == "C-reference")
            .expect("reference candidate result exists");
        assert_eq!(
            reference.disposition,
            CandidateDisposition::UnknownReviewRequired
        );
        assert!(reference.findings.iter().any(|finding| {
            finding.code == "utility_clearance:legacy-comms-unknown"
                && finding.status == CheckStatus::Unknown
        }));
        verify_report(&modified, &report)
            .expect("reference checker must not convert unbounded unknown to clear");
    }

    #[test]
    fn nonfinite_derived_grade_is_rejected_before_reporting_metrics() {
        let bytes = fixture_bytes();
        let mut fixture = parse_fixture(&bytes);
        let candidate = fixture
            .candidates
            .iter_mut()
            .find(|candidate| candidate.id == "C-reference")
            .expect("reference candidate exists");
        candidate.profile_points[1].station_m = 1.0e-300;
        candidate.profile_points[1].elevation_m = 1.0e308;
        let modified = serde_json::to_vec(&fixture).expect("finite input values serialize");
        let errors = evaluate_json(&modified).expect_err("overflowing grade must fail closed");
        assert!(
            errors.iter().any(|error| {
                error.contains("longitudinal profile cannot produce a finite grade")
            })
        );
    }

    #[test]
    fn altered_pareto_frontier_is_rejected_by_reference_checker() {
        let bytes = fixture_bytes();
        let mut report = evaluate_json(&bytes).expect("fixture should evaluate");
        report.pareto_frontier_candidate_ids = vec!["A-shortest".to_string()];
        let errors = verify_report(&bytes, &report).expect_err("Pareto tampering must fail");
        assert!(errors.iter().any(|error| error.contains("Pareto frontier")));
    }

    #[test]
    fn duplicate_drainage_scenario_ids_fail_closed() {
        let bytes = fixture_bytes();
        let mut fixture = parse_fixture(&bytes);
        let candidate = fixture
            .candidates
            .iter_mut()
            .find(|candidate| candidate.id == "C-reference")
            .expect("reference candidate exists");
        candidate.drainage_scenarios[1].scenario_id =
            candidate.drainage_scenarios[0].scenario_id.clone();
        let modified = serde_json::to_vec(&fixture).expect("fixture serializes");
        let errors = evaluate_json(&modified).expect_err("duplicate scenario IDs must fail");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("duplicate drainage scenario id"))
        );
    }
}
