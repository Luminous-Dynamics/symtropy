use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs, process,
};

const CLAIM_CEILING: &str = "Synthetic cost-energy-labor-capacity frontier only. All prices, equipment costs, station power, staffing and grid intensity are sensitivity inputs. This is not a measured factory, ROI calculation, vendor quote, SOTA result or purchase recommendation.";

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct EconomicCatalog {
    schema_version: String,
    benchmark_id: String,
    input_evidence_class: String,
    sites: Vec<SiteProfile>,
    station_profiles: Vec<StationCostProfile>,
    claim_ceiling: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct SiteProfile {
    site_id: String,
    country_or_region: String,
    local_currency: String,
    local_currency_per_usd: f64,
    operating_days_per_year: u32,
    shifts_per_day: u32,
    paid_hours_per_shift: f64,
    fully_loaded_labor_cost_per_hour_local: f64,
    electricity_cost_per_kwh_local: f64,
    building_lease_cost_per_sqm_year_local: f64,
    grid_kgco2e_per_kwh: f64,
    evidence_class: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct StationCostProfile {
    station_id: String,
    capex_usd_per_added_server: f64,
    operators_per_added_server_per_shift: f64,
    standby_kw_per_added_server: f64,
    active_kw_per_added_server: f64,
    annual_maintenance_fraction_of_capex: f64,
    incremental_floor_area_sqm: f64,
    evidence_class: String,
}

#[derive(Debug, Clone, Deserialize)]
struct SimulationReport {
    schema_version: String,
    benchmark_id: String,
    replication_count: usize,
    replication_seeds: Vec<u64>,
    input_evidence_class: String,
    release_ready: bool,
    output_never_exceeds_released_input: bool,
    all_buffers_respected_capacity: bool,
    station_time_accounting_within_horizon: bool,
    scenarios: Vec<SimulationScenario>,
    scenario_statistics: Vec<ScenarioStatistics>,
    replication_outcomes: Vec<ReplicationOutcome>,
    station_operating_statistics: Vec<StationOperatingStatistics>,
}

#[derive(Debug, Clone, Deserialize)]
struct SimulationScenario {
    scenario_id: String,
    intervention_station_id: Option<String>,
    stations: Vec<StationState>,
}

#[derive(Debug, Clone, Deserialize)]
struct StationState {
    station_id: String,
    server_count: usize,
}

#[derive(Debug, Clone, Deserialize)]
struct ScenarioStatistics {
    scenario_id: String,
    intervention_station_id: Option<String>,
    paired_good_units_per_hour_delta_vs_baseline: Option<DistributionSummary>,
}

#[derive(Debug, Clone, Deserialize)]
struct ReplicationOutcome {
    seed: u64,
    scenario_id: String,
    intervention_station_id: Option<String>,
    good_units_per_hour: f64,
    baseline_good_units_per_hour: f64,
    paired_good_units_per_hour_delta_vs_baseline: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
struct StationOperatingStatistics {
    scenario_id: String,
    station_id: String,
    server_count: usize,
    busy_fraction: DistributionSummary,
    blocked_fraction: DistributionSummary,
    starved_fraction: DistributionSummary,
    down_fraction: DistributionSummary,
    accounted_fraction: DistributionSummary,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
struct DistributionSummary {
    sample_count: usize,
    mean: f64,
    sample_standard_deviation: Option<f64>,
    minimum: f64,
    p05: f64,
    median: f64,
    p95: f64,
    maximum: f64,
    mean_confidence_95_lower: Option<f64>,
    mean_confidence_95_upper: Option<f64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct EconomicCandidate {
    site_id: String,
    country_or_region: String,
    local_currency: String,
    station_id: String,
    simulation_scenario_id: String,
    server_count_after_intervention: usize,
    annual_scheduled_operating_hours: f64,
    mean_paired_good_output_gain_units_per_hour: f64,
    paired_good_output_gain_95_ci_lower_units_per_hour: Option<f64>,
    paired_good_output_gain_95_ci_upper_units_per_hour: Option<f64>,
    annual_incremental_good_units_mean: f64,
    annual_incremental_good_units_95_ci_lower: Option<f64>,
    annual_incremental_good_units_95_ci_upper: Option<f64>,
    output_gain_confidently_positive_under_simulation: bool,
    incremental_capex_usd: f64,
    incremental_capex_local: f64,
    incremental_floor_area_sqm: f64,
    annual_incremental_labor_hours: f64,
    annual_incremental_labor_cost_local: f64,
    proxy_added_server_busy_fraction: f64,
    annual_incremental_energy_kwh: f64,
    annual_incremental_energy_cost_local: f64,
    annual_incremental_emissions_kgco2e: f64,
    annual_incremental_maintenance_cost_local: f64,
    annual_incremental_floor_lease_cost_local: f64,
    annual_incremental_opex_local: f64,
    capex_per_incremental_annual_good_unit_local: Option<f64>,
    annual_opex_per_incremental_good_unit_local: Option<f64>,
    cost_inputs_are_synthetic: bool,
    energy_duty_basis: String,
    pareto_eligible_under_declared_model: bool,
    pareto_frontier_member: bool,
    deployment_authorized: bool,
    claim_ceiling: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct SiteFrontier {
    site_id: String,
    local_currency: String,
    eligible_candidate_count: usize,
    pareto_frontier_station_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct Report {
    schema_version: String,
    benchmark_id: String,
    source_simulation_report_sha256: String,
    source_economic_fixture_sha256: String,
    source_simulation_replication_count: usize,
    source_simulation_seed_count: usize,
    site_count: usize,
    station_profile_count: usize,
    candidate_count: usize,
    objective_definition: Vec<Objective>,
    site_frontiers: Vec<SiteFrontier>,
    candidates: Vec<EconomicCandidate>,
    all_cost_and_energy_inputs_are_synthetic: bool,
    production_release_authorized: bool,
    roi_computed: bool,
    claim_ceiling: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct Objective {
    field: String,
    direction: String,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("factory economic frontier rejected input: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        return Err(
            "usage: ev_factory_economic_frontier <simulation-report.json> <economic-fixture.json>"
                .into(),
        );
    }
    let simulation_bytes =
        fs::read(&args[1]).map_err(|error| format!("cannot read simulation report: {error}"))?;
    let fixture_bytes =
        fs::read(&args[2]).map_err(|error| format!("cannot read economic fixture: {error}"))?;
    let simulation: SimulationReport = serde_json::from_slice(&simulation_bytes)
        .map_err(|error| format!("simulation report JSON/schema invalid: {error}"))?;
    let economics: EconomicCatalog = serde_json::from_slice(&fixture_bytes)
        .map_err(|error| format!("economic fixture JSON/schema invalid: {error}"))?;
    validate_inputs(&simulation, &economics)?;

    let mut candidates =
        Vec::with_capacity(economics.sites.len() * economics.station_profiles.len());
    for site in &economics.sites {
        for station_profile in &economics.station_profiles {
            let scenario_id = format!("add-one-server-{}", station_profile.station_id);
            let delta = simulation
                .scenario_statistics
                .iter()
                .find(|row| {
                    row.scenario_id == scenario_id
                        && row.intervention_station_id.as_deref()
                            == Some(station_profile.station_id.as_str())
                })
                .and_then(|row| row.paired_good_units_per_hour_delta_vs_baseline.as_ref())
                .ok_or_else(|| {
                    format!(
                        "paired_output_delta_missing_for_{}",
                        station_profile.station_id
                    )
                })?;
            let operating = simulation
                .station_operating_statistics
                .iter()
                .find(|row| {
                    row.scenario_id == scenario_id && row.station_id == station_profile.station_id
                })
                .ok_or_else(|| {
                    format!(
                        "station_operating_summary_missing_for_{}",
                        station_profile.station_id
                    )
                })?;
            let scenario = simulation
                .scenarios
                .iter()
                .find(|row| {
                    row.scenario_id == scenario_id
                        && row.intervention_station_id.as_deref()
                            == Some(station_profile.station_id.as_str())
                })
                .ok_or_else(|| {
                    format!(
                        "station_intervention_scenario_missing_for_{}",
                        station_profile.station_id
                    )
                })?;
            let station_state = scenario
                .stations
                .iter()
                .find(|row| row.station_id == station_profile.station_id)
                .ok_or_else(|| {
                    format!("station_state_missing_for_{}", station_profile.station_id)
                })?;
            let candidate = calculate_candidate(
                site,
                station_profile,
                delta,
                operating,
                station_state.server_count,
                &scenario_id,
            )?;
            candidates.push(candidate);
        }
    }

    let mut site_frontiers = Vec::with_capacity(economics.sites.len());
    for site in &economics.sites {
        let mut eligible_indices = Vec::new();
        for (index, candidate) in candidates.iter().enumerate() {
            if candidate.site_id == site.site_id && candidate.pareto_eligible_under_declared_model {
                eligible_indices.push(index);
            }
        }
        let mut frontier_ids = Vec::new();
        for &candidate_index in &eligible_indices {
            let candidate = &candidates[candidate_index];
            let dominated = eligible_indices
                .iter()
                .copied()
                .filter(|other_index| *other_index != candidate_index)
                .any(|other_index| dominates(&candidates[other_index], candidate));
            if !dominated {
                frontier_ids.push(candidate.station_id.clone());
                candidates[candidate_index].pareto_frontier_member = true;
            }
        }
        frontier_ids.sort();
        site_frontiers.push(SiteFrontier {
            site_id: site.site_id.clone(),
            local_currency: site.local_currency.clone(),
            eligible_candidate_count: eligible_indices.len(),
            pareto_frontier_station_ids: frontier_ids,
        });
    }

    let objectives = vec![
        Objective {
            field: "annual_incremental_good_units_mean".into(),
            direction: "maximize".into(),
        },
        Objective {
            field: "incremental_capex_local".into(),
            direction: "minimize".into(),
        },
        Objective {
            field: "annual_incremental_opex_local".into(),
            direction: "minimize".into(),
        },
        Objective {
            field: "annual_incremental_energy_kwh".into(),
            direction: "minimize".into(),
        },
        Objective {
            field: "annual_incremental_emissions_kgco2e".into(),
            direction: "minimize".into(),
        },
        Objective {
            field: "incremental_floor_area_sqm".into(),
            direction: "minimize".into(),
        },
    ];
    let report = Report {
        schema_version: "0.1.0".into(),
        benchmark_id: "EV-FACTORY-ECON-001".into(),
        source_simulation_report_sha256: sha256(&simulation_bytes),
        source_economic_fixture_sha256: sha256(&fixture_bytes),
        source_simulation_replication_count: simulation.replication_count,
        source_simulation_seed_count: simulation.replication_seeds.len(),
        site_count: economics.sites.len(),
        station_profile_count: economics.station_profiles.len(),
        candidate_count: candidates.len(),
        objective_definition: objectives,
        site_frontiers,
        candidates,
        all_cost_and_energy_inputs_are_synthetic: true,
        production_release_authorized: false,
        roi_computed: false,
        claim_ceiling: CLAIM_CEILING.into(),
    };
    let output = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("cannot serialize economic report: {error}"))?;
    println!("{output}");
    Ok(())
}

fn validate_inputs(
    simulation: &SimulationReport,
    economics: &EconomicCatalog,
) -> Result<(), String> {
    if simulation.schema_version != "0.1.0"
        || simulation.benchmark_id != "EV-FACTORY-SIM-001"
        || simulation.input_evidence_class != "synthetic"
        || simulation.release_ready
        || !simulation.output_never_exceeds_released_input
        || !simulation.all_buffers_respected_capacity
        || !simulation.station_time_accounting_within_horizon
        || simulation.replication_count < 2
        || simulation.replication_seeds.len() != simulation.replication_count
    {
        return Err("simulation_report_failed_synthetic_credibility_contract".into());
    }
    let mut unique_seeds = BTreeSet::new();
    if simulation
        .replication_seeds
        .iter()
        .any(|seed| *seed == 0 || !unique_seeds.insert(*seed))
    {
        return Err("simulation_seed_list_must_be_positive_and_unique".into());
    }
    let mut seeds = BTreeSet::new();
    if simulation
        .replication_seeds
        .iter()
        .any(|seed| *seed == 0 || !seeds.insert(*seed))
    {
        return Err("simulation_seed_list_must_be_positive_and_unique".into());
    }
    if economics.schema_version != "0.1.0"
        || economics.benchmark_id != "EV-FACTORY-ECON-001"
        || economics.input_evidence_class != "synthetic"
        || economics.sites.is_empty()
        || economics.station_profiles.is_empty()
        || economics.claim_ceiling.trim().is_empty()
    {
        return Err("economic_fixture_failed_synthetic_schema_contract".into());
    }
    let mut site_ids = BTreeSet::new();
    for site in &economics.sites {
        if site.site_id.trim().is_empty()
            || site.local_currency.trim().is_empty()
            || site.country_or_region.trim().is_empty()
            || site.evidence_class != "synthetic"
            || !site_ids.insert(site.site_id.clone())
            || site.local_currency_per_usd <= 0.0
            || site.operating_days_per_year == 0
            || site.shifts_per_day == 0
            || site.paid_hours_per_shift <= 0.0
            || site.fully_loaded_labor_cost_per_hour_local < 0.0
            || site.electricity_cost_per_kwh_local < 0.0
            || site.building_lease_cost_per_sqm_year_local < 0.0
            || site.grid_kgco2e_per_kwh < 0.0
            || !all_finite(&[
                site.local_currency_per_usd,
                site.paid_hours_per_shift,
                site.fully_loaded_labor_cost_per_hour_local,
                site.electricity_cost_per_kwh_local,
                site.building_lease_cost_per_sqm_year_local,
                site.grid_kgco2e_per_kwh,
            ])
        {
            return Err(format!(
                "site_profile_{}_invalid_or_not_synthetic",
                site.site_id
            ));
        }
    }
    let mut station_ids = BTreeSet::new();
    for profile in &economics.station_profiles {
        if profile.station_id.trim().is_empty()
            || profile.evidence_class != "synthetic"
            || !station_ids.insert(profile.station_id.clone())
            || profile.capex_usd_per_added_server <= 0.0
            || profile.operators_per_added_server_per_shift < 0.0
            || profile.standby_kw_per_added_server < 0.0
            || profile.active_kw_per_added_server < profile.standby_kw_per_added_server
            || profile.annual_maintenance_fraction_of_capex < 0.0
            || profile.incremental_floor_area_sqm < 0.0
            || !all_finite(&[
                profile.capex_usd_per_added_server,
                profile.operators_per_added_server_per_shift,
                profile.standby_kw_per_added_server,
                profile.active_kw_per_added_server,
                profile.annual_maintenance_fraction_of_capex,
                profile.incremental_floor_area_sqm,
            ])
        {
            return Err(format!(
                "station_profile_{}_invalid_or_not_synthetic",
                profile.station_id
            ));
        }
    }
    let baseline = simulation
        .scenarios
        .iter()
        .find(|row| row.scenario_id == "baseline")
        .ok_or_else(|| "simulation_baseline_scenario_missing".to_string())?;
    if baseline.intervention_station_id.is_some() {
        return Err("baseline_scenario_must_not_name_intervention".into());
    }
    let baseline_stations: BTreeSet<String> = baseline
        .stations
        .iter()
        .map(|station| station.station_id.clone())
        .collect();
    if baseline_stations != station_ids {
        return Err("station_economic_profiles_must_match_simulator_station_ids_exactly".into());
    }
    for row in simulation
        .scenario_statistics
        .iter()
        .filter(|row| row.scenario_id != "baseline")
    {
        let gain = row
            .paired_good_units_per_hour_delta_vs_baseline
            .as_ref()
            .ok_or_else(|| "intervention_paired_gain_distribution_missing".to_string())?;
        if row.intervention_station_id.is_none()
            || gain.sample_count != simulation.replication_count
            || !gain.mean.is_finite()
            || gain
                .mean_confidence_95_lower
                .is_some_and(|value| !value.is_finite())
            || gain
                .mean_confidence_95_upper
                .is_some_and(|value| !value.is_finite())
        {
            return Err(
                "intervention_gain_distribution_failed_numeric_or_seed_count_checks".into(),
            );
        }
    }
    let expected_outcome_count = simulation.replication_count * (station_ids.len() + 1);
    if simulation.station_operating_statistics.len()
        != simulation.scenario_statistics.len() * station_ids.len()
        || simulation.scenario_statistics.len() != station_ids.len() + 1
        || simulation.station_operating_statistics.iter().any(|row| {
            row.busy_fraction.sample_count != simulation.replication_count
                || row.blocked_fraction.sample_count != simulation.replication_count
                || row.starved_fraction.sample_count != simulation.replication_count
                || row.down_fraction.sample_count != simulation.replication_count
                || row.accounted_fraction.sample_count != simulation.replication_count
                || !all_finite(&[
                    row.busy_fraction.mean,
                    row.blocked_fraction.mean,
                    row.starved_fraction.mean,
                    row.down_fraction.mean,
                    row.accounted_fraction.mean,
                ])
                || !(0.0..=1.0).contains(&row.busy_fraction.mean)
                || !(0.0..=1.0).contains(&row.blocked_fraction.mean)
                || !(0.0..=1.0).contains(&row.starved_fraction.mean)
                || !(0.0..=1.0).contains(&row.down_fraction.mean)
        })
    {
        return Err("station_operating_statistics_are_missing_or_invalid".into());
    }
    if simulation.replication_outcomes.len() != expected_outcome_count {
        return Err("simulation_per_seed_outcome_count_mismatch".into());
    }

    let close_enough =
        |left: f64, right: f64| (left - right).abs() <= 1e-9 * left.abs().max(right.abs()).max(1.0);
    let expected_scenarios: BTreeSet<String> = std::iter::once("baseline".to_string())
        .chain(
            station_ids
                .iter()
                .map(|station_id| format!("add-one-server-{station_id}")),
        )
        .collect();
    let mut outcome_keys = BTreeSet::new();
    let mut baseline_rate_by_seed = BTreeMap::<u64, f64>::new();
    let mut gains_by_scenario = BTreeMap::<String, Vec<f64>>::new();

    for outcome in &simulation.replication_outcomes {
        if !seeds.contains(&outcome.seed)
            || !expected_scenarios.contains(&outcome.scenario_id)
            || !all_finite(&[
                outcome.good_units_per_hour,
                outcome.baseline_good_units_per_hour,
            ])
            || outcome.good_units_per_hour < 0.0
            || outcome.baseline_good_units_per_hour < 0.0
            || !outcome_keys.insert((outcome.seed, outcome.scenario_id.clone()))
        {
            return Err("simulation_per_seed_outcome_identity_or_value_invalid".into());
        }

        if let Some(previous_baseline) =
            baseline_rate_by_seed.insert(outcome.seed, outcome.baseline_good_units_per_hour)
        {
            if !close_enough(previous_baseline, outcome.baseline_good_units_per_hour) {
                return Err("simulation_same_seed_baseline_rate_mismatch".into());
            }
        }

        if outcome.scenario_id == "baseline" {
            if outcome.intervention_station_id.is_some()
                || outcome
                    .paired_good_units_per_hour_delta_vs_baseline
                    .is_some()
                || !close_enough(
                    outcome.good_units_per_hour,
                    outcome.baseline_good_units_per_hour,
                )
            {
                return Err("simulation_baseline_outcome_contract_invalid".into());
            }
        } else {
            let station_id = outcome
                .scenario_id
                .strip_prefix("add-one-server-")
                .unwrap_or_default();
            let delta = outcome
                .paired_good_units_per_hour_delta_vs_baseline
                .ok_or_else(|| "simulation_intervention_paired_delta_missing".to_string())?;
            if !station_ids.contains(station_id)
                || outcome.intervention_station_id.as_deref() != Some(station_id)
                || !delta.is_finite()
                || !close_enough(
                    delta,
                    outcome.good_units_per_hour - outcome.baseline_good_units_per_hour,
                )
            {
                return Err("simulation_intervention_outcome_delta_or_identity_invalid".into());
            }
            gains_by_scenario
                .entry(outcome.scenario_id.clone())
                .or_default()
                .push(delta);
        }
    }

    if outcome_keys.len() != expected_outcome_count || baseline_rate_by_seed.len() != seeds.len() {
        return Err("simulation_per_seed_outcomes_incomplete_or_duplicate".into());
    }

    for row in simulation
        .scenario_statistics
        .iter()
        .filter(|row| row.scenario_id != "baseline")
    {
        let samples = gains_by_scenario
            .get(&row.scenario_id)
            .ok_or_else(|| "simulation_intervention_seed_samples_missing".to_string())?;
        let summary = row
            .paired_good_units_per_hour_delta_vs_baseline
            .as_ref()
            .ok_or_else(|| "simulation_intervention_paired_gain_summary_missing".to_string())?;
        let mean = samples.iter().sum::<f64>() / samples.len() as f64;
        if samples.len() != simulation.replication_count || !close_enough(mean, summary.mean) {
            return Err("simulation_per_seed_gain_mean_disagrees_with_summary".into());
        }
    }
    for profile in &economics.station_profiles {
        let expected_scenario = format!("add-one-server-{}", profile.station_id);
        if !simulation.scenarios.iter().any(|scenario| {
            scenario.scenario_id == expected_scenario
                && scenario.intervention_station_id.as_deref() == Some(profile.station_id.as_str())
        }) {
            return Err(format!(
                "simulation_missing_station_intervention_{}",
                profile.station_id
            ));
        }
    }
    Ok(())
}

fn calculate_candidate(
    site: &SiteProfile,
    profile: &StationCostProfile,
    gain: &DistributionSummary,
    operating: &StationOperatingStatistics,
    servers_after: usize,
    scenario_id: &str,
) -> Result<EconomicCandidate, String> {
    if operating.busy_fraction.mean < 0.0 || operating.busy_fraction.mean > 1.0 {
        return Err(format!("busy_fraction_invalid_for_{}", profile.station_id));
    }
    let annual_hours = site.operating_days_per_year as f64
        * site.shifts_per_day as f64
        * site.paid_hours_per_shift;
    let annual_incremental_good_units = gain.mean * annual_hours;
    let annual_ci_lower = gain
        .mean_confidence_95_lower
        .map(|value| value * annual_hours);
    let annual_ci_upper = gain
        .mean_confidence_95_upper
        .map(|value| value * annual_hours);
    let confidently_positive = gain
        .mean_confidence_95_lower
        .is_some_and(|value| value > 0.0);
    let capex_local = profile.capex_usd_per_added_server * site.local_currency_per_usd;
    let annual_labor_hours = profile.operators_per_added_server_per_shift * annual_hours;
    let annual_labor_cost = annual_labor_hours * site.fully_loaded_labor_cost_per_hour_local;
    let busy = operating.busy_fraction.mean.clamp(0.0, 1.0);
    let annual_energy_kwh = annual_hours
        * (profile.standby_kw_per_added_server
            + busy * (profile.active_kw_per_added_server - profile.standby_kw_per_added_server));
    let annual_energy_cost = annual_energy_kwh * site.electricity_cost_per_kwh_local;
    let annual_emissions = annual_energy_kwh * site.grid_kgco2e_per_kwh;
    let annual_maintenance_cost = capex_local * profile.annual_maintenance_fraction_of_capex;
    let annual_lease_cost =
        profile.incremental_floor_area_sqm * site.building_lease_cost_per_sqm_year_local;
    let annual_opex =
        annual_labor_cost + annual_energy_cost + annual_maintenance_cost + annual_lease_cost;
    let capex_per_capacity_unit = if confidently_positive && annual_incremental_good_units > 0.0 {
        Some(capex_local / annual_incremental_good_units)
    } else {
        None
    };
    let opex_per_incremental_unit = if confidently_positive && annual_incremental_good_units > 0.0 {
        Some(annual_opex / annual_incremental_good_units)
    } else {
        None
    };
    Ok(EconomicCandidate {
        site_id: site.site_id.clone(),
        country_or_region: site.country_or_region.clone(),
        local_currency: site.local_currency.clone(),
        station_id: profile.station_id.clone(),
        simulation_scenario_id: scenario_id.to_string(),
        server_count_after_intervention: servers_after,
        annual_scheduled_operating_hours: annual_hours,
        mean_paired_good_output_gain_units_per_hour: gain.mean,
        paired_good_output_gain_95_ci_lower_units_per_hour: gain.mean_confidence_95_lower,
        paired_good_output_gain_95_ci_upper_units_per_hour: gain.mean_confidence_95_upper,
        annual_incremental_good_units_mean: annual_incremental_good_units,
        annual_incremental_good_units_95_ci_lower: annual_ci_lower,
        annual_incremental_good_units_95_ci_upper: annual_ci_upper,
        output_gain_confidently_positive_under_simulation: confidently_positive,
        incremental_capex_usd: profile.capex_usd_per_added_server,
        incremental_capex_local: capex_local,
        incremental_floor_area_sqm: profile.incremental_floor_area_sqm,
        annual_incremental_labor_hours: annual_labor_hours,
        annual_incremental_labor_cost_local: annual_labor_cost,
        proxy_added_server_busy_fraction: busy,
        annual_incremental_energy_kwh: annual_energy_kwh,
        annual_incremental_energy_cost_local: annual_energy_cost,
        annual_incremental_emissions_kgco2e: annual_emissions,
        annual_incremental_maintenance_cost_local: annual_maintenance_cost,
        annual_incremental_floor_lease_cost_local: annual_lease_cost,
        annual_incremental_opex_local: annual_opex,
        capex_per_incremental_annual_good_unit_local: capex_per_capacity_unit,
        annual_opex_per_incremental_good_unit_local: opex_per_incremental_unit,
        cost_inputs_are_synthetic: true,
        energy_duty_basis:
            "scenario_station_average_busy_fraction_proxied_as_exchangeable_added_server".into(),
        pareto_eligible_under_declared_model: confidently_positive,
        pareto_frontier_member: false,
        deployment_authorized: false,
        claim_ceiling: CLAIM_CEILING.into(),
    })
}

fn dominates(left: &EconomicCandidate, right: &EconomicCandidate) -> bool {
    let no_worse = left.annual_incremental_good_units_mean
        >= right.annual_incremental_good_units_mean
        && left.incremental_capex_local <= right.incremental_capex_local
        && left.annual_incremental_opex_local <= right.annual_incremental_opex_local
        && left.annual_incremental_energy_kwh <= right.annual_incremental_energy_kwh
        && left.annual_incremental_emissions_kgco2e <= right.annual_incremental_emissions_kgco2e
        && left.incremental_floor_area_sqm <= right.incremental_floor_area_sqm;
    let strictly_better = left.annual_incremental_good_units_mean
        > right.annual_incremental_good_units_mean
        || left.incremental_capex_local < right.incremental_capex_local
        || left.annual_incremental_opex_local < right.annual_incremental_opex_local
        || left.annual_incremental_energy_kwh < right.annual_incremental_energy_kwh
        || left.annual_incremental_emissions_kgco2e < right.annual_incremental_emissions_kgco2e
        || left.incremental_floor_area_sqm < right.incremental_floor_area_sqm;
    no_worse && strictly_better
}

fn all_finite(values: &[f64]) -> bool {
    values.iter().all(|value| value.is_finite())
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(
        output: f64,
        capex: f64,
        opex: f64,
        energy: f64,
        emissions: f64,
        area: f64,
    ) -> EconomicCandidate {
        EconomicCandidate {
            site_id: "test".into(),
            country_or_region: "synthetic".into(),
            local_currency: "XYZ".into(),
            station_id: "candidate".into(),
            simulation_scenario_id: "add-one-server-candidate".into(),
            server_count_after_intervention: 2,
            annual_scheduled_operating_hours: 100.0,
            mean_paired_good_output_gain_units_per_hour: output,
            paired_good_output_gain_95_ci_lower_units_per_hour: Some(output),
            paired_good_output_gain_95_ci_upper_units_per_hour: Some(output),
            annual_incremental_good_units_mean: output,
            annual_incremental_good_units_95_ci_lower: Some(output),
            annual_incremental_good_units_95_ci_upper: Some(output),
            output_gain_confidently_positive_under_simulation: output > 0.0,
            incremental_capex_usd: capex,
            incremental_capex_local: capex,
            incremental_floor_area_sqm: area,
            annual_incremental_labor_hours: 1.0,
            annual_incremental_labor_cost_local: 1.0,
            proxy_added_server_busy_fraction: 0.5,
            annual_incremental_energy_kwh: energy,
            annual_incremental_energy_cost_local: energy,
            annual_incremental_emissions_kgco2e: emissions,
            annual_incremental_maintenance_cost_local: 1.0,
            annual_incremental_floor_lease_cost_local: 1.0,
            annual_incremental_opex_local: opex,
            capex_per_incremental_annual_good_unit_local: Some(capex / output),
            annual_opex_per_incremental_good_unit_local: Some(opex / output),
            cost_inputs_are_synthetic: true,
            energy_duty_basis: "test".into(),
            pareto_eligible_under_declared_model: true,
            pareto_frontier_member: false,
            deployment_authorized: false,
            claim_ceiling: CLAIM_CEILING.into(),
        }
    }

    #[test]
    fn annual_hours_labor_energy_and_cost_formula_are_hand_calculable() {
        let site = SiteProfile {
            site_id: "site".into(),
            country_or_region: "test".into(),
            local_currency: "XYZ".into(),
            local_currency_per_usd: 2.0,
            operating_days_per_year: 200,
            shifts_per_day: 2,
            paid_hours_per_shift: 8.0,
            fully_loaded_labor_cost_per_hour_local: 10.0,
            electricity_cost_per_kwh_local: 3.0,
            building_lease_cost_per_sqm_year_local: 4.0,
            grid_kgco2e_per_kwh: 0.5,
            evidence_class: "synthetic".into(),
        };
        let profile = StationCostProfile {
            station_id: "station".into(),
            capex_usd_per_added_server: 100.0,
            operators_per_added_server_per_shift: 1.0,
            standby_kw_per_added_server: 1.0,
            active_kw_per_added_server: 3.0,
            annual_maintenance_fraction_of_capex: 0.1,
            incremental_floor_area_sqm: 2.0,
            evidence_class: "synthetic".into(),
        };
        let gain = DistributionSummary {
            sample_count: 3,
            mean: 1.0,
            sample_standard_deviation: Some(0.0),
            minimum: 1.0,
            p05: 1.0,
            median: 1.0,
            p95: 1.0,
            maximum: 1.0,
            mean_confidence_95_lower: Some(1.0),
            mean_confidence_95_upper: Some(1.0),
        };
        let busy_half = DistributionSummary {
            mean: 0.5,
            ..gain.clone()
        };
        let operating = StationOperatingStatistics {
            scenario_id: "intervention".into(),
            station_id: "station".into(),
            server_count: 2,
            busy_fraction: busy_half,
            blocked_fraction: gain.clone(),
            starved_fraction: gain.clone(),
            down_fraction: gain.clone(),
            accounted_fraction: gain.clone(),
        };
        let row =
            calculate_candidate(&site, &profile, &gain, &operating, 2, "intervention").unwrap();
        assert_eq!(row.annual_scheduled_operating_hours, 3200.0);
        assert_eq!(row.annual_incremental_labor_hours, 3200.0);
        assert_eq!(row.annual_incremental_labor_cost_local, 32000.0);
        assert_eq!(row.annual_incremental_energy_kwh, 6400.0);
        assert_eq!(row.annual_incremental_energy_cost_local, 19200.0);
        assert_eq!(row.annual_incremental_emissions_kgco2e, 3200.0);
        assert_eq!(row.incremental_capex_local, 200.0);
        assert_eq!(row.annual_incremental_maintenance_cost_local, 20.0);
        assert_eq!(row.annual_incremental_floor_lease_cost_local, 8.0);
        assert_eq!(row.annual_incremental_opex_local, 51228.0);
        assert_eq!(row.annual_incremental_good_units_mean, 3200.0);
        assert_eq!(
            row.capex_per_incremental_annual_good_unit_local,
            Some(0.0625)
        );
    }

    #[test]
    fn nonpositive_confidence_lower_bound_blocks_unit_cost_and_pareto_eligibility() {
        let site = SiteProfile {
            site_id: "site".into(),
            country_or_region: "test".into(),
            local_currency: "XYZ".into(),
            local_currency_per_usd: 2.0,
            operating_days_per_year: 200,
            shifts_per_day: 2,
            paid_hours_per_shift: 8.0,
            fully_loaded_labor_cost_per_hour_local: 10.0,
            electricity_cost_per_kwh_local: 3.0,
            building_lease_cost_per_sqm_year_local: 4.0,
            grid_kgco2e_per_kwh: 0.5,
            evidence_class: "synthetic".into(),
        };
        let profile = StationCostProfile {
            station_id: "station".into(),
            capex_usd_per_added_server: 100.0,
            operators_per_added_server_per_shift: 1.0,
            standby_kw_per_added_server: 1.0,
            active_kw_per_added_server: 3.0,
            annual_maintenance_fraction_of_capex: 0.1,
            incremental_floor_area_sqm: 2.0,
            evidence_class: "synthetic".into(),
        };
        let uncertain_gain = DistributionSummary {
            sample_count: 20,
            mean: 0.1,
            sample_standard_deviation: Some(0.2),
            minimum: -0.3,
            p05: -0.2,
            median: 0.1,
            p95: 0.3,
            maximum: 0.4,
            mean_confidence_95_lower: Some(-0.05),
            mean_confidence_95_upper: Some(0.25),
        };
        let duty = DistributionSummary {
            sample_count: 20,
            mean: 0.5,
            sample_standard_deviation: Some(0.1),
            minimum: 0.3,
            p05: 0.35,
            median: 0.5,
            p95: 0.65,
            maximum: 0.7,
            mean_confidence_95_lower: Some(0.45),
            mean_confidence_95_upper: Some(0.55),
        };
        let operating = StationOperatingStatistics {
            scenario_id: "intervention".into(),
            station_id: "station".into(),
            server_count: 2,
            busy_fraction: duty.clone(),
            blocked_fraction: duty.clone(),
            starved_fraction: duty.clone(),
            down_fraction: duty.clone(),
            accounted_fraction: duty,
        };
        let row = calculate_candidate(
            &site,
            &profile,
            &uncertain_gain,
            &operating,
            2,
            "intervention",
        )
        .unwrap();
        assert!(!row.output_gain_confidently_positive_under_simulation);
        assert!(!row.pareto_eligible_under_declared_model);
        assert!(row.capex_per_incremental_annual_good_unit_local.is_none());
        assert!(row.annual_opex_per_incremental_good_unit_local.is_none());
        assert!(!row.deployment_authorized);
    }

    #[test]
    fn pareto_dominance_requires_all_axes_no_worse_and_one_strictly_better() {
        let a = candidate(10.0, 100.0, 20.0, 10.0, 5.0, 10.0);
        let b = candidate(9.0, 110.0, 21.0, 11.0, 5.5, 11.0);
        let c = candidate(11.0, 120.0, 18.0, 12.0, 6.0, 12.0);
        let tied = candidate(10.0, 100.0, 20.0, 10.0, 5.0, 10.0);
        assert!(dominates(&a, &b));
        assert!(!dominates(&b, &a));
        assert!(!dominates(&a, &c));
        assert!(!dominates(&c, &a));
        assert!(!dominates(&a, &tied));
        assert!(!dominates(&tied, &a));
    }
}
