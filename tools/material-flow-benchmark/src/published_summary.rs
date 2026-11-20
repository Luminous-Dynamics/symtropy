// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Integrity check for a published truck–drone benchmark summary.
//! This does not re-run the paper's MILP or load its licensed raw dataset.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const SCHEMA_VERSION: u16 = 1;
const MAX_ENERGY_WH: u64 = 10_000_000;
const MAX_DISTANCE_METERS: u64 = 10_000_000;
const MAX_DELIVERIES: u64 = 1_000_000;
const EXPECTED_DOI: &str = "10.3390/drones10070476";
const EXPECTED_LICENSE: &str = "CC-BY-NC-4.0";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PaperCaseMetrics {
    pub makespan_minutes: u64,
    pub truck_energy_wh: u64,
    pub drone_energy_wh: u64,
    pub total_energy_wh: u64,
    pub system_distance_meters: u64,
    pub truck_served_customers: u64,
    pub drone_served_customers: u64,
    /// Hundredths of a watt-hour per delivery (e.g. 11260 = 112.60 Wh).
    pub energy_per_delivery_hundredths_wh: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PublishedSummaryInput {
    pub schema_version: u16,
    pub benchmark_id: String,
    pub citation_doi: String,
    pub title: String,
    pub publication_date: String,
    pub source_url: String,
    pub dataset_name: String,
    pub dataset_year: u16,
    pub dataset_route_count: u64,
    pub dataset_metro_count: u16,
    pub dataset_license: String,
    pub raw_dataset_loaded: bool,
    pub truck_count: u16,
    pub shared_drone_fleet_size: u16,
    pub aircraft_platform: String,
    pub battery_feasibility_enforced: bool,
    pub baseline: PaperCaseMetrics,
    pub collaborative: PaperCaseMetrics,
    /// Narrative value in the paper body. Table 8 gives 30,984 Wh.
    pub narrative_collaborative_truck_energy_wh: u64,
    /// Published Table 8 percentage improvements, in tenths of one percent.
    /// 245 means 24.5%; negative means an increase.
    pub reported_makespan_reduction_tenths_percent: i64,
    pub reported_truck_energy_reduction_tenths_percent: i64,
    pub reported_total_energy_reduction_tenths_percent: i64,
    pub reported_distance_increase_tenths_percent: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PublishedSummaryReport {
    pub schema_version: u16,
    pub benchmark_id: String,
    pub citation_doi: String,
    pub input_digest_sha256: String,
    pub summary_arithmetic_consistent: bool,
    pub computed_makespan_reduction_tenths_percent: i64,
    pub computed_truck_energy_reduction_tenths_percent: i64,
    pub computed_total_energy_reduction_tenths_percent: i64,
    pub computed_distance_increase_tenths_percent: i64,
    pub collaborative_energy_balance_wh: u64,
    pub delivery_count_reconciles: bool,
    pub narrative_table_truck_energy_difference_wh: u64,
    pub published_battery_feasibility_enforced: bool,
    pub battery_feasibility_claim_permitted: bool,
    pub paper_milp_reexecuted: bool,
    pub raw_dataset_loaded: bool,
    pub commercial_use_permitted_by_dataset_license: bool,
    pub required_parallel_reproduction_tracks: Vec<String>,
    pub warnings: Vec<String>,
    pub claim_ceiling: String,
}

fn canonical_digest(input: &PublishedSummaryInput) -> Result<String, String> {
    let bytes = serde_json::to_vec(input)
        .map_err(|error| format!("cannot encode published-summary input: {error}"))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn rounded_tenths_percent(numerator: i128, denominator: u64) -> Result<i64, String> {
    if denominator == 0 {
        return Err("percentage denominator must be positive".into());
    }
    let scaled = numerator
        .checked_mul(1_000)
        .ok_or_else(|| "percentage numerator overflow".to_string())?;
    let denominator = i128::from(denominator);
    let rounded = if scaled >= 0 {
        (scaled + denominator / 2) / denominator
    } else {
        (scaled - denominator / 2) / denominator
    };
    i64::try_from(rounded).map_err(|_| "percentage exceeds report range".into())
}

fn reduction_tenths_percent(baseline: u64, candidate: u64) -> Result<i64, String> {
    let delta = i128::from(baseline) - i128::from(candidate);
    rounded_tenths_percent(delta, baseline)
}

fn increase_tenths_percent(baseline: u64, candidate: u64) -> Result<i64, String> {
    let delta = i128::from(candidate) - i128::from(baseline);
    rounded_tenths_percent(delta, baseline)
}

fn validate_input(input: &PublishedSummaryInput) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if input.schema_version != SCHEMA_VERSION {
        errors.push("unsupported_schema_version".into());
    }
    if input.benchmark_id != "MFB-SOTA-CICEK-2026-SUMMARY-001" {
        errors.push("unexpected_benchmark_id".into());
    }
    if input.citation_doi != EXPECTED_DOI {
        errors.push("citation_doi_does_not_match_registered_paper".into());
    }
    if input.title.trim().is_empty()
        || input.publication_date != "2026-06-23"
        || input.source_url != "https://www.mdpi.com/2504-446X/10/7/476"
    {
        errors.push("publication_metadata_is_incomplete_or_mismatched".into());
    }
    if input.dataset_name != "2021 Amazon Last Mile Routing Research Challenge Dataset"
        || input.dataset_year != 2018
        || input.dataset_route_count != 9_184
        || input.dataset_metro_count != 5
        || input.dataset_license != EXPECTED_LICENSE
    {
        errors.push("dataset_provenance_or_license_mismatch".into());
    }
    if input.truck_count != 3
        || input.shared_drone_fleet_size != 10
        || input.aircraft_platform != "DJI Matrice 100"
    {
        errors.push("published_platform_or_fleet_description_mismatch".into());
    }
    if input.battery_feasibility_enforced {
        errors.push("paper_table_3_battery_feasibility_flag_must_remain_disabled".into());
    }
    if input.raw_dataset_loaded {
        errors.push("summary_integrity_verifier_does_not_load_raw_amazon_dataset".into());
    }
    for (label, metrics) in [
        ("baseline", &input.baseline),
        ("collaborative", &input.collaborative),
    ] {
        if metrics.makespan_minutes == 0
            || metrics.truck_energy_wh > MAX_ENERGY_WH
            || metrics.drone_energy_wh > MAX_ENERGY_WH
            || metrics.total_energy_wh > MAX_ENERGY_WH
            || metrics.system_distance_meters > MAX_DISTANCE_METERS
            || metrics.truck_served_customers > MAX_DELIVERIES
            || metrics.drone_served_customers > MAX_DELIVERIES
            || metrics.energy_per_delivery_hundredths_wh > 10_000_000
        {
            errors.push(format!("{label}_metrics_exceed_v1_bounds"));
        }
    }
    if input.baseline.drone_energy_wh != 0
        || input.baseline.drone_served_customers != 0
        || input.baseline.total_energy_wh != input.baseline.truck_energy_wh
    {
        errors.push("truck_only_baseline_contains_drone_energy_or_deliveries".into());
    }
    if input.narrative_collaborative_truck_energy_wh > MAX_ENERGY_WH {
        errors.push("narrative_truck_energy_exceeds_v1_bounds".into());
    }
    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

/// Verify only published summary arithmetic, source metadata and claim boundaries.
/// This is not an independent reproduction of the paper's model or dataset.
pub fn verify_published_summary(
    input: &PublishedSummaryInput,
) -> Result<PublishedSummaryReport, Vec<String>> {
    validate_input(input)?;
    let digest = canonical_digest(input).map_err(|error| vec![error])?;

    let collaborative_energy_balance = input
        .collaborative
        .truck_energy_wh
        .checked_add(input.collaborative.drone_energy_wh)
        .ok_or_else(|| vec!["collaborative_energy_sum_overflow".into()])?;
    let delivery_count_reconciles = input
        .collaborative
        .truck_served_customers
        .checked_add(input.collaborative.drone_served_customers)
        == input.baseline.truck_served_customers;

    let makespan_reduction = reduction_tenths_percent(
        input.baseline.makespan_minutes,
        input.collaborative.makespan_minutes,
    ).map_err(|error| vec![error])?;
    let truck_energy_reduction = reduction_tenths_percent(
        input.baseline.truck_energy_wh,
        input.collaborative.truck_energy_wh,
    ).map_err(|error| vec![error])?;
    let total_energy_reduction = reduction_tenths_percent(
        input.baseline.total_energy_wh,
        input.collaborative.total_energy_wh,
    ).map_err(|error| vec![error])?;
    let distance_increase = increase_tenths_percent(
        input.baseline.system_distance_meters,
        input.collaborative.system_distance_meters,
    ).map_err(|error| vec![error])?;

    let narrative_difference = input
        .narrative_collaborative_truck_energy_wh
        .abs_diff(input.collaborative.truck_energy_wh);

    let arithmetic_consistent = collaborative_energy_balance
        == input.collaborative.total_energy_wh
        && delivery_count_reconciles
        && makespan_reduction == input.reported_makespan_reduction_tenths_percent
        && truck_energy_reduction == input.reported_truck_energy_reduction_tenths_percent
        && total_energy_reduction == input.reported_total_energy_reduction_tenths_percent
        && distance_increase == input.reported_distance_increase_tenths_percent;

    let mut warnings = Vec::new();
    if narrative_difference != 0 {
        warnings.push(format!(
            "paper_body_vs_table_truck_energy_differs_by_{narrative_difference}_wh;_table_8_value_is_used_for_arithmetic"
        ));
    }
    if !input.battery_feasibility_enforced {
        warnings.push(
            "paper_parameter_table_sets_battery_feasibility_enforced_to_zero;_reported_pareto_point_is_not_battery_feasibility_qualified".into(),
        );
    }
    if !input.raw_dataset_loaded {
        warnings.push("raw_amazon_dataset_not_loaded;_this_is_not_dataset_or_milp_reproduction".into());
    }
    if input.dataset_license == EXPECTED_LICENSE {
        warnings.push("amazon_dataset_is_noncommercial_cc_by_nc_4_0;_commercial_reuse_is_not_permitted_by_this_license".into());
    }

    Ok(PublishedSummaryReport {
        schema_version: SCHEMA_VERSION,
        benchmark_id: input.benchmark_id.clone(),
        citation_doi: input.citation_doi.clone(),
        input_digest_sha256: digest,
        summary_arithmetic_consistent: arithmetic_consistent,
        computed_makespan_reduction_tenths_percent: makespan_reduction,
        computed_truck_energy_reduction_tenths_percent: truck_energy_reduction,
        computed_total_energy_reduction_tenths_percent: total_energy_reduction,
        computed_distance_increase_tenths_percent: distance_increase,
        collaborative_energy_balance_wh: collaborative_energy_balance,
        delivery_count_reconciles,
        narrative_table_truck_energy_difference_wh: narrative_difference,
        published_battery_feasibility_enforced: input.battery_feasibility_enforced,
        // A table summary can never certify an independently reproduced physical mission.
        battery_feasibility_claim_permitted: false,
        paper_milp_reexecuted: false,
        raw_dataset_loaded: input.raw_dataset_loaded,
        commercial_use_permitted_by_dataset_license: false,
        required_parallel_reproduction_tracks: vec![
            "paper_faithful_track_keep_battery_feasibility_flag_off_and_reproduce_published_summary".into(),
            "battery_feasible_extension_enable_mission_reserve_constraints_and_report_changed_feasibility_makespan_energy_and_service".into(),
        ],
        warnings,
        claim_ceiling: "Published-summary arithmetic and provenance consistency only; not an independent MILP reproduction, raw-data rerun, mission/battery feasibility qualification, South African operating case, commercial result or claim of superiority".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> PublishedSummaryInput {
        PublishedSummaryInput {
            schema_version: 1,
            benchmark_id: "MFB-SOTA-CICEK-2026-SUMMARY-001".into(),
            citation_doi: EXPECTED_DOI.into(),
            title: "Data-Driven Optimization of Truck–Drone Collaborative Delivery with Shared Fleet Allocation".into(),
            publication_date: "2026-06-23".into(),
            source_url: "https://www.mdpi.com/2504-446X/10/7/476".into(),
            dataset_name: "2021 Amazon Last Mile Routing Research Challenge Dataset".into(),
            dataset_year: 2018,
            dataset_route_count: 9_184,
            dataset_metro_count: 5,
            dataset_license: EXPECTED_LICENSE.into(),
            raw_dataset_loaded: false,
            truck_count: 3,
            shared_drone_fleet_size: 10,
            aircraft_platform: "DJI Matrice 100".into(),
            battery_feasibility_enforced: false,
            baseline: PaperCaseMetrics {
                makespan_minutes: 474,
                truck_energy_wh: 51_338,
                drone_energy_wh: 0,
                total_energy_wh: 51_338,
                system_distance_meters: 218_230,
                truck_served_customers: 456,
                drone_served_customers: 0,
                energy_per_delivery_hundredths_wh: 11_260,
            },
            collaborative: PaperCaseMetrics {
                makespan_minutes: 358,
                truck_energy_wh: 30_984,
                drone_energy_wh: 12_033,
                total_energy_wh: 43_017,
                system_distance_meters: 578_800,
                truck_served_customers: 12,
                drone_served_customers: 444,
                energy_per_delivery_hundredths_wh: 9_430,
            },
            narrative_collaborative_truck_energy_wh: 30_983,
            reported_makespan_reduction_tenths_percent: 245,
            reported_truck_energy_reduction_tenths_percent: 396,
            reported_total_energy_reduction_tenths_percent: 162,
            reported_distance_increase_tenths_percent: 1_652,
        }
    }

    #[test]
    fn published_table_8_summary_arithmetic_reconciles() {
        let report = verify_published_summary(&input()).expect("published summary is valid");
        assert!(report.summary_arithmetic_consistent);
        assert_eq!(report.computed_makespan_reduction_tenths_percent, 245);
        assert_eq!(report.computed_truck_energy_reduction_tenths_percent, 396);
        assert_eq!(report.computed_total_energy_reduction_tenths_percent, 162);
        assert_eq!(report.computed_distance_increase_tenths_percent, 1_652);
        assert_eq!(report.collaborative_energy_balance_wh, 43_017);
        assert!(report.delivery_count_reconciles);
    }

    #[test]
    fn published_battery_flag_off_never_becomes_a_feasibility_claim() {
        let report = verify_published_summary(&input()).expect("published summary is valid");
        assert!(!report.published_battery_feasibility_enforced);
        assert!(!report.battery_feasible_claim_permitted);
        assert!(!report.paper_milp_reexecuted);
        assert!(report.warnings.iter().any(|warning| warning.contains("battery_feasibility_enforced_to_zero")));
    }

    #[test]
    fn summary_input_rejects_tampering_with_paper_battery_flag_or_raw_data_boundary() {
        let mut battery = input();
        battery.battery_feasibility_enforced = true;
        assert!(validate_input(&battery)
            .expect_err("paper battery flag must stay off")
            .iter()
            .any(|error| error.contains("battery_feasibility_flag_must_remain_disabled")));

        let mut raw_data = input();
        raw_data.raw_dataset_loaded = true;
        assert!(validate_input(&raw_data)
            .expect_err("summary verifier does not load raw data")
            .iter()
            .any(|error| error.contains("does_not_load_raw_amazon_dataset")));
    }

    #[test]
    fn one_watt_hour_body_table_discrepancy_is_preserved_and_reported() {
        let report = verify_published_summary(&input()).expect("published summary is valid");
        assert_eq!(report.narrative_table_truck_energy_difference_wh, 1);
        assert!(report.warnings.iter().any(|warning| warning.contains("differs_by_1_wh")));
    }

    #[test]
    fn invalid_published_total_energy_is_not_silently_reconciled() {
        let mut test = input();
        test.collaborative.total_energy_wh += 1;
        let report = verify_published_summary(&test).expect("structure is valid");
        assert!(!report.summary_arithmetic_consistent);
    }

    #[test]
    fn paper_metadata_and_license_are_pinned() {
        let mut test = input();
        test.dataset_license = "commercial-use-permitted".into();
        assert!(validate_input(&test).is_err());
    }

    #[test]
    fn input_order_is_stable_because_the_published_summary_is_scalar() {
        let first = verify_published_summary(&input()).expect("first report");
        let second = verify_published_summary(&input()).expect("second report");
        assert_eq!(first, second);
    }
}
