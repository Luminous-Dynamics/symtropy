//! Deterministic, fail-closed HAPS fleet service-availability event replay.
//!
//! V1 evaluates daily platform-count service against a k-out-of-n threshold.
//! It composes scheduled maintenance and regional common-cause stand-downs.
//! It does not model launch/recovery throughput, return-to-ready turnaround,
//! coverage geometry, radio/backhaul, flight dynamics, or stochastic reliability.

use std::collections::HashSet;
use std::fmt;

use serde::{Deserialize, Serialize};

const FRACTION_TOLERANCE: f64 = 1.0e-12;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FleetError {
    Json(String),
    InvalidCase(String),
    Unsupported(String),
    OracleMismatch(String),
}

impl fmt::Display for FleetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(message)
            | Self::InvalidCase(message)
            | Self::Unsupported(message)
            | Self::OracleMismatch(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for FleetError {}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FleetCase {
    pub schema_version: String,
    pub case_id: String,
    pub revision: u64,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub campaign: Campaign,
    pub source: Source,
    pub variants: Vec<Variant>,
    pub rotation_sensitivity: Vec<RotationBoundCase>,
    pub assumptions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Campaign {
    pub duration_days: usize,
    pub step_days: usize,
    pub required_active_platforms: usize,
    pub service_boundary: String,
    pub availability_target: f64,
    pub service_profile_ref: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub source_kind: String,
    pub source_uri: Option<String>,
    pub source_digest: Option<String>,
    pub rights_or_license_ref: Option<String>,
    pub available_at_decision_time: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RotationBoundCase {
    pub case_id: String,
    pub on_station_days: f64,
    pub recovery_to_ready_days: f64,
    pub launch_to_service_days: f64,
    pub required_service_platforms: usize,
    pub expected_minimum_fleet_size: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Variant {
    pub variant_id: String,
    pub description: String,
    pub required_active_platforms: usize,
    pub platforms: Vec<Platform>,
    pub maintenance_windows: Vec<MaintenanceWindow>,
    pub common_cause_events: Vec<CommonCauseEvent>,
    pub logistics: Logistics,
    pub expected_oracle: ExpectedOracle,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Platform {
    pub platform_id: String,
    pub weather_domain_id: String,
    pub initial_state: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaintenanceWindow {
    pub maintenance_id: String,
    pub platform_id: String,
    pub start_day: usize,
    pub duration_days: usize,
    pub maintenance_class: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommonCauseEvent {
    pub event_id: String,
    pub event_class: String,
    pub start_day: usize,
    pub duration_days: usize,
    pub affected_weather_domain_ids: Vec<String>,
    pub impact: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Logistics {
    pub ready_ground_spares: usize,
    pub launch_capacity_per_day: Option<usize>,
    pub launch_to_service_days: Option<usize>,
    pub recovery_capacity_per_day: Option<usize>,
    pub recovery_to_ready_days: Option<usize>,
    pub recovery_turnaround_evidence: String,
    pub turnaround_source_ref: Option<String>,
    pub included_in_availability_oracle: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedOracle {
    pub oracle_status: String,
    pub daily_active_platforms: Vec<usize>,
    pub daily_service_available: Vec<bool>,
    pub unavailable_day_offsets: Vec<usize>,
    pub service_available_days: usize,
    pub service_unavailable_days: usize,
    pub availability_fraction: f64,
    pub longest_continuous_outage_days: usize,
    pub maintenance_unavailable_platform_days: usize,
    pub weather_unavailable_platform_days: usize,
    pub maximum_simultaneous_unavailable_platforms: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct VariantReport {
    pub variant_id: String,
    pub required_active_platforms: usize,
    pub daily_active_platforms: Vec<usize>,
    pub daily_service_available: Vec<bool>,
    pub unavailable_day_offsets: Vec<usize>,
    pub service_available_days: usize,
    pub service_unavailable_days: usize,
    pub availability_fraction: f64,
    pub longest_continuous_outage_days: usize,
    pub maintenance_unavailable_platform_days: usize,
    pub weather_unavailable_platform_days: usize,
    pub maximum_simultaneous_unavailable_platforms: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RotationBoundReport {
    pub case_id: String,
    pub on_station_days: f64,
    pub recovery_to_ready_days: f64,
    pub launch_to_service_days: f64,
    pub required_service_platforms: usize,
    pub duty_fraction: f64,
    pub theoretical_minimum_fleet_size: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct FleetReport {
    pub case_id: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub campaign_duration_days: usize,
    pub service_boundary: String,
    pub variants: Vec<VariantReport>,
    pub rotation_sensitivity: Vec<RotationBoundReport>,
}

pub fn parse_case_json(bytes: &[u8]) -> Result<FleetCase, FleetError> {
    let case: FleetCase =
        serde_json::from_slice(bytes).map_err(|error| FleetError::Json(error.to_string()))?;
    validate_case(&case)?;
    Ok(case)
}

pub fn evaluate_case(case: &FleetCase) -> Result<FleetReport, FleetError> {
    validate_case(case)?;
    let mut reports = Vec::new();
    reports
        .try_reserve_exact(case.variants.len())
        .map_err(|_| FleetError::InvalidCase("could not allocate fleet reports".to_owned()))?;

    for variant in &case.variants {
        let report = evaluate_variant(variant, case.campaign.duration_days)?;
        verify_oracle(variant, &report)?;
        reports.push(report);
    }

    let rotation_sensitivity = case
        .rotation_sensitivity
        .iter()
        .map(evaluate_rotation_bound)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(FleetReport {
        case_id: case.case_id.clone(),
        evidence_class: case.evidence_class.clone(),
        claim_ceiling: case.claim_ceiling.clone(),
        campaign_duration_days: case.campaign.duration_days,
        service_boundary: case.campaign.service_boundary.clone(),
        variants: reports,
        rotation_sensitivity,
    })
}

fn validate_case(case: &FleetCase) -> Result<(), FleetError> {
    if case.schema_version != "haps-fleet-availability-case/v1" {
        return Err(FleetError::Unsupported(format!(
            "unsupported schema_version: {}",
            case.schema_version
        )));
    }
    if case.case_id.trim().is_empty() || case.revision == 0 {
        return Err(FleetError::InvalidCase(
            "case_id must be non-empty and revision must be positive".to_owned(),
        ));
    }
    if case.evidence_class != "synthetic"
        || case.claim_ceiling != "synthetic_event_schedule_oracle"
        || case.source.source_kind != "synthetic_generator"
        || case.source.available_at_decision_time
    {
        return Err(FleetError::Unsupported(
            "v1 executable accepts synthetic offline event cases only".to_owned(),
        ));
    }
    if case.campaign.duration_days == 0
        || case.campaign.step_days != 1
        || case.campaign.required_active_platforms == 0
        || !case.campaign.availability_target.is_finite()
        || !(0.0..=1.0).contains(&case.campaign.availability_target)
    {
        return Err(FleetError::InvalidCase(
            "invalid campaign duration, step, k threshold, or availability target".to_owned(),
        ));
    }
    if case.campaign.service_boundary != "platform_count_only" {
        return Err(FleetError::Unsupported(
            "v1 evaluates platform_count_only; coverage/backhaul-qualified service is not implemented"
                .to_owned(),
        ));
    }
    if case.variants.is_empty() || case.rotation_sensitivity.is_empty() {
        return Err(FleetError::InvalidCase(
            "at least one fleet variant and rotation sensitivity case are required".to_owned(),
        ));
    }
    let mut rotation_case_ids = HashSet::new();
    for rotation in &case.rotation_sensitivity {
        if rotation.case_id.trim().is_empty()
            || !rotation_case_ids.insert(rotation.case_id.as_str())
            || !rotation.on_station_days.is_finite()
            || rotation.on_station_days <= 0.0
            || !rotation.recovery_to_ready_days.is_finite()
            || rotation.recovery_to_ready_days < 0.0
            || !rotation.launch_to_service_days.is_finite()
            || rotation.launch_to_service_days < 0.0
            || rotation.required_service_platforms == 0
            || rotation.expected_minimum_fleet_size == 0
        {
            return Err(FleetError::InvalidCase(format!(
                "invalid or duplicate rotation sensitivity case: {}",
                rotation.case_id
            )));
        }
    }

    let mut variant_ids = HashSet::new();
    for variant in &case.variants {
        if !variant_ids.insert(variant.variant_id.as_str()) {
            return Err(FleetError::InvalidCase(format!(
                "duplicate variant_id: {}",
                variant.variant_id
            )));
        }
        validate_variant(variant, case.campaign.duration_days)?;
    }
    Ok(())
}

fn validate_variant(variant: &Variant, duration_days: usize) -> Result<(), FleetError> {
    if variant.variant_id.trim().is_empty()
        || variant.description.trim().is_empty()
        || variant.platforms.is_empty()
        || variant.required_active_platforms == 0
        || variant.required_active_platforms > variant.platforms.len()
    {
        return Err(FleetError::InvalidCase(format!(
            "invalid platform count or k threshold in {}",
            variant.variant_id
        )));
    }
    if variant.logistics.included_in_availability_oracle {
        return Err(FleetError::Unsupported(
            "v1 does not evaluate launch/recovery/return-to-ready logistics".to_owned(),
        ));
    }

    let mut platform_ids = HashSet::new();
    for platform in &variant.platforms {
        if platform.platform_id.trim().is_empty()
            || platform.weather_domain_id.trim().is_empty()
            || platform.initial_state != "on_station"
            || !platform_ids.insert(platform.platform_id.as_str())
        {
            return Err(FleetError::InvalidCase(format!(
                "invalid or duplicate platform in {}",
                variant.variant_id
            )));
        }
    }

    let mut event_ids = HashSet::new();
    for maintenance in &variant.maintenance_windows {
        if !platform_ids.contains(maintenance.platform_id.as_str()) {
            return Err(FleetError::InvalidCase(format!(
                "maintenance {} refers to unknown platform {}",
                maintenance.maintenance_id, maintenance.platform_id
            )));
        }
        validate_window(
            &maintenance.maintenance_id,
            maintenance.start_day,
            maintenance.duration_days,
            duration_days,
        )?;
        if !event_ids.insert(maintenance.maintenance_id.as_str()) {
            return Err(FleetError::InvalidCase(format!(
                "duplicate event id: {}",
                maintenance.maintenance_id
            )));
        }
    }

    for event in &variant.common_cause_events {
        if event.event_class != "regional_weather_standdown" || event.impact != "platform_standdown"
        {
            return Err(FleetError::Unsupported(format!(
                "unsupported common-cause event {}: only regional_weather_standdown/platform_standdown is implemented",
                event.event_id
            )));
        }
        if event.affected_weather_domain_ids.is_empty() {
            return Err(FleetError::InvalidCase(format!(
                "weather event {} has no affected domains",
                event.event_id
            )));
        }
        validate_window(
            &event.event_id,
            event.start_day,
            event.duration_days,
            duration_days,
        )?;
        if !event_ids.insert(event.event_id.as_str()) {
            return Err(FleetError::InvalidCase(format!(
                "duplicate event id: {}",
                event.event_id
            )));
        }
    }
    Ok(())
}

fn validate_window(
    event_id: &str,
    start_day: usize,
    duration_days: usize,
    campaign_days: usize,
) -> Result<(), FleetError> {
    if duration_days == 0
        || start_day >= campaign_days
        || start_day
            .checked_add(duration_days)
            .is_none_or(|end| end > campaign_days)
    {
        return Err(FleetError::InvalidCase(format!(
            "event {event_id} lies outside campaign or has zero duration"
        )));
    }
    Ok(())
}

fn evaluate_variant(variant: &Variant, duration_days: usize) -> Result<VariantReport, FleetError> {
    let platform_count = variant.platforms.len();
    let mut maintenance = vec![vec![false; duration_days]; platform_count];
    let mut weather = vec![vec![false; duration_days]; platform_count];

    for window in &variant.maintenance_windows {
        let index = variant
            .platforms
            .iter()
            .position(|platform| platform.platform_id == window.platform_id)
            .ok_or_else(|| FleetError::InvalidCase("unknown maintenance platform".to_owned()))?;
        for day in window.start_day..window.start_day + window.duration_days {
            maintenance[index][day] = true;
        }
    }

    for event in &variant.common_cause_events {
        for (index, platform) in variant.platforms.iter().enumerate() {
            if event
                .affected_weather_domain_ids
                .iter()
                .any(|domain| domain == &platform.weather_domain_id)
            {
                for day in event.start_day..event.start_day + event.duration_days {
                    weather[index][day] = true;
                }
            }
        }
    }

    let mut daily_active_platforms = Vec::new();
    let mut daily_service_available = Vec::new();
    let mut unavailable_day_offsets = Vec::new();
    daily_active_platforms
        .try_reserve_exact(duration_days)
        .map_err(|_| {
            FleetError::InvalidCase("could not allocate daily platform counts".to_owned())
        })?;
    daily_service_available
        .try_reserve_exact(duration_days)
        .map_err(|_| {
            FleetError::InvalidCase("could not allocate daily service flags".to_owned())
        })?;

    let mut maintenance_platform_days = 0;
    let mut weather_platform_days = 0;
    let mut maximum_simultaneous_unavailable_platforms = 0;
    let mut longest_continuous_outage_days = 0;
    let mut current_outage_days = 0;

    for day in 0..duration_days {
        let mut unavailable_platforms = 0;
        for index in 0..platform_count {
            if maintenance[index][day] {
                maintenance_platform_days += 1;
            }
            if weather[index][day] {
                weather_platform_days += 1;
            }
            if maintenance[index][day] || weather[index][day] {
                unavailable_platforms += 1;
            }
        }

        maximum_simultaneous_unavailable_platforms =
            maximum_simultaneous_unavailable_platforms.max(unavailable_platforms);
        let active = platform_count - unavailable_platforms;
        let available = active >= variant.required_active_platforms;
        daily_active_platforms.push(active);
        daily_service_available.push(available);

        if available {
            current_outage_days = 0;
        } else {
            unavailable_day_offsets.push(day);
            current_outage_days += 1;
            longest_continuous_outage_days =
                longest_continuous_outage_days.max(current_outage_days);
        }
    }

    let service_available_days = daily_service_available
        .iter()
        .filter(|available| **available)
        .count();
    let service_unavailable_days = duration_days - service_available_days;
    let availability_fraction = service_available_days as f64 / duration_days as f64;

    Ok(VariantReport {
        variant_id: variant.variant_id.clone(),
        required_active_platforms: variant.required_active_platforms,
        daily_active_platforms,
        daily_service_available,
        unavailable_day_offsets,
        service_available_days,
        service_unavailable_days,
        availability_fraction,
        longest_continuous_outage_days,
        maintenance_unavailable_platform_days: maintenance_platform_days,
        weather_unavailable_platform_days: weather_platform_days,
        maximum_simultaneous_unavailable_platforms,
    })
}

fn evaluate_rotation_bound(case: &RotationBoundCase) -> Result<RotationBoundReport, FleetError> {
    let cycle_days =
        case.on_station_days + case.recovery_to_ready_days + case.launch_to_service_days;
    let duty_fraction = case.on_station_days / cycle_days;
    let ideal_platform_count = case.required_service_platforms as f64 / duty_fraction;
    if !ideal_platform_count.is_finite() || ideal_platform_count > usize::MAX as f64 {
        return Err(FleetError::InvalidCase(format!(
            "rotation sensitivity {} exceeds supported fleet-size range",
            case.case_id
        )));
    }
    let theoretical_minimum_fleet_size =
        (ideal_platform_count - FRACTION_TOLERANCE).ceil() as usize;
    if theoretical_minimum_fleet_size != case.expected_minimum_fleet_size {
        return Err(FleetError::OracleMismatch(format!(
            "rotation lower-bound oracle mismatch for {}: expected {}, got {}",
            case.case_id, case.expected_minimum_fleet_size, theoretical_minimum_fleet_size
        )));
    }
    Ok(RotationBoundReport {
        case_id: case.case_id.clone(),
        on_station_days: case.on_station_days,
        recovery_to_ready_days: case.recovery_to_ready_days,
        launch_to_service_days: case.launch_to_service_days,
        required_service_platforms: case.required_service_platforms,
        duty_fraction,
        theoretical_minimum_fleet_size,
    })
}

fn verify_oracle(variant: &Variant, actual: &VariantReport) -> Result<(), FleetError> {
    let expected = &variant.expected_oracle;
    let matches = expected.oracle_status == "independently_calculated"
        && expected.daily_active_platforms == actual.daily_active_platforms
        && expected.daily_service_available == actual.daily_service_available
        && expected.unavailable_day_offsets == actual.unavailable_day_offsets
        && expected.service_available_days == actual.service_available_days
        && expected.service_unavailable_days == actual.service_unavailable_days
        && (expected.availability_fraction - actual.availability_fraction).abs()
            <= FRACTION_TOLERANCE
        && expected.longest_continuous_outage_days == actual.longest_continuous_outage_days
        && expected.maintenance_unavailable_platform_days
            == actual.maintenance_unavailable_platform_days
        && expected.weather_unavailable_platform_days == actual.weather_unavailable_platform_days
        && expected.maximum_simultaneous_unavailable_platforms
            == actual.maximum_simultaneous_unavailable_platforms;

    if !matches {
        return Err(FleetError::OracleMismatch(format!(
            "expected oracle did not match recomputed result for {}",
            variant.variant_id
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!(
        "../../../docs/research/aviation/examples/synthetic_haps_fleet_maintenance_common_cause_30day_v1.json"
    );

    fn fixture() -> FleetCase {
        parse_case_json(FIXTURE.as_bytes()).expect("fixture parses")
    }

    #[test]
    fn three_fleet_variants_match_exact_event_oracles() {
        let report = evaluate_case(&fixture()).expect("all expected oracles match");
        assert_eq!(report.variants.len(), 3);
        assert_eq!(report.variants[0].service_available_days, 19);
        assert_eq!(report.variants[1].service_available_days, 28);
        assert_eq!(report.variants[2].service_available_days, 30);
        assert_eq!(report.variants[0].availability_fraction, 19.0 / 30.0);
        assert_eq!(report.variants[1].availability_fraction, 28.0 / 30.0);
        assert_eq!(report.variants[2].availability_fraction, 1.0);
    }

    #[test]
    fn common_cause_weather_is_applied_to_every_platform_in_the_domain() {
        let case = fixture();
        let report = evaluate_case(&case).expect("oracle matches");
        assert_eq!(
            report.variants[1].maximum_simultaneous_unavailable_platforms,
            2
        );
        assert_eq!(
            report.variants[2].maximum_simultaneous_unavailable_platforms,
            1
        );
        assert_eq!(report.variants[1].unavailable_day_offsets, vec![26, 27]);
        assert!(report.variants[2].unavailable_day_offsets.is_empty());
    }

    #[test]
    fn maintenance_and_weather_on_the_same_node_day_are_unioned_for_service() {
        let mut case = fixture();
        let event = &mut case.variants[2].common_cause_events[0];
        event.start_day = 4;
        event.duration_days = 1;
        let report = evaluate_variant(&case.variants[2], case.campaign.duration_days)
            .expect("event schedule is valid");
        assert_eq!(report.daily_active_platforms[4], 3);
        assert_eq!(report.maximum_simultaneous_unavailable_platforms, 1);
        assert_eq!(report.maintenance_unavailable_platform_days, 12);
        assert_eq!(report.weather_unavailable_platform_days, 1);
    }

    #[test]
    fn out_of_campaign_event_is_rejected() {
        let mut case = fixture();
        let event = &mut case.variants[0].common_cause_events[0];
        event.start_day = 29;
        event.duration_days = 2;
        assert!(matches!(
            validate_case(&case),
            Err(FleetError::InvalidCase(_))
        ));
    }

    #[test]
    fn unsupported_common_cause_kind_fails_closed() {
        let mut case = fixture();
        case.variants[0].common_cause_events[0].event_class = "common_part_shortage".to_owned();
        assert!(matches!(
            validate_case(&case),
            Err(FleetError::Unsupported(_))
        ));
    }

    #[test]
    fn logistics_unknowns_are_not_silently_used_in_availability() {
        let case = fixture();
        assert!(
            case.variants
                .iter()
                .all(|variant| !variant.logistics.included_in_availability_oracle
                    && variant.logistics.recovery_to_ready_days.is_none())
        );
        assert!(evaluate_case(&case).is_ok());
    }

    #[test]
    fn k_greater_than_deployed_fleet_is_rejected() {
        let mut case = fixture();
        case.variants[0].required_active_platforms = 4;
        assert!(matches!(
            validate_case(&case),
            Err(FleetError::InvalidCase(_))
        ));
    }

    #[test]
    fn source_attributed_or_non_synthetic_data_is_not_treated_as_synthetic() {
        let mut case = fixture();
        case.source.available_at_decision_time = true;
        assert!(matches!(
            validate_case(&case),
            Err(FleetError::Unsupported(_))
        ));
    }

    #[test]
    fn rotation_lower_bound_shows_reconditioning_dominates_fleet_size() {
        let report = evaluate_case(&fixture()).expect("rotation cases match");
        let counts = report
            .rotation_sensitivity
            .iter()
            .map(|row| row.theoretical_minimum_fleet_size)
            .collect::<Vec<_>>();
        assert_eq!(counts, vec![5, 6, 9, 10, 4]);
        assert_eq!(report.rotation_sensitivity[2].duty_fraction, 1.0 / 3.0);
        assert_eq!(report.rotation_sensitivity[4].duty_fraction, 0.75);
    }

    #[test]
    fn zero_or_negative_rotation_duration_is_rejected() {
        let mut case = fixture();
        case.rotation_sensitivity[0].on_station_days = 0.0;
        assert!(matches!(
            validate_case(&case),
            Err(FleetError::InvalidCase(_))
        ));
    }

    #[test]
    fn service_boundary_outside_v1_is_not_accepted() {
        let mut case = fixture();
        case.campaign.service_boundary = "end_to_end_customer_sla".to_owned();
        assert!(matches!(
            validate_case(&case),
            Err(FleetError::Unsupported(_))
        ));
    }
}
