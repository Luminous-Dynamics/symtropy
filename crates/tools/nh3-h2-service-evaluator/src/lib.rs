use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const SCHEMA_VERSION: &str = "luminous.nh3-h2-service.v0";
const BENCHMARK_ID: &str = "NH3-H2-SERVICE-TWIN-001";
const MW_NH3: f64 = 17.03052;
const MW_H2: f64 = 2.01588;
const MW_N2: f64 = 28.0134;
const REL_TOL: f64 = 1.0e-8;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema_version: String,
    benchmark_id: String,
    frozen_at: String,
    evidence_class: String,
    physical_measurements: bool,
    luminous_measurements: bool,
    route_sources: Vec<RouteSource>,
    scenarios: Vec<Scenario>,
    comparisons: Vec<Comparison>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RouteSource {
    source_case_id: String,
    route_id: String,
    evidence_class: String,
    claim_ceiling: String,
    allowed_operating_point_ids: Vec<String>,
    luminous_reproduction: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Scenario {
    id: String,
    evidence_class: String,
    route_id: String,
    operating_point_id: String,
    source_case_id: Option<String>,
    source_operating_point_id: Option<String>,
    feed_contract_id: String,
    product_contract_id: String,
    energy_boundary_id: String,
    duty_cycle_id: String,
    feed: FeedLedger,
    reaction: ReactionLedger,
    separation: SeparationLedger,
    product: ProductContract,
    energy: EnergyBoundary,
    dynamic_service_claimed: bool,
    dynamic_service: Option<DynamicService>,
    expected_disposition: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FeedLedger {
    nh3_input_mass_g: Option<f64>,
    nh3_reacted_mass_g: Option<f64>,
    nh3_recovered_mass_g: Option<f64>,
    nh3_purge_mass_g: Option<f64>,
    nh3_slip_mass_g: Option<f64>,
    reported_conversion_fraction: Option<f64>,
    unknown_streams: Vec<String>,
    tolerance_g: f64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReactionLedger {
    h2_generated_mass_g: Option<f64>,
    n2_generated_mass_g: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SeparationLedger {
    h2_accepted_mass_g: Option<f64>,
    h2_rejected_mass_g: Option<f64>,
    h2_unrecovered_mass_g: Option<f64>,
    n2_recovered_mass_g: Option<f64>,
    n2_purge_mass_g: Option<f64>,
    unknown_streams: Vec<String>,
    tolerance_g: f64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProductContract {
    accepted_h2_mass_g: Option<f64>,
    gross_product_stream_mass_g: Option<f64>,
    reported_h2_purity_fraction: Option<f64>,
    minimum_h2_purity_fraction: f64,
    delivery_pressure_bar_abs: Option<f64>,
    minimum_delivery_pressure_bar_abs: f64,
    delivery_interval_s: Option<f64>,
    contract_id: String,
    quantitative_product_measurement_ref: Option<String>,
    calibration_ref: Option<String>,
    minimum_transient_purity_fraction: f64,
    maximum_flow_overshoot_fraction: f64,
    maximum_offspec_duration_ms: f64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EnergyBoundary {
    boundary_id: String,
    meter_mode: String,
    meters: Vec<EnergyMeter>,
    required_roles: Vec<String>,
    meter_calibration_status: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EnergyMeter {
    meter_id: String,
    role: String,
    energy_kwh: f64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DynamicService {
    claimed_response_measurand: String,
    actual_evidence_layer: String,
    power_command_ms: Option<f64>,
    grid_power_measured_ms: Option<f64>,
    conversion_state_ms: Option<f64>,
    diagnostic_proxy_ms: Option<f64>,
    measured_gas_ms: Option<f64>,
    purified_h2_ms: Option<f64>,
    customer_accepted_h2_ms: Option<f64>,
    customer_delivery_settled_ms: Option<f64>,
    clock_sync_ref: Option<String>,
    product_flow_measurement_ref: Option<String>,
    customer_boundary_ref: Option<String>,
    minimum_transient_purity_observed: Option<f64>,
    product_flow_overshoot_fraction: Option<f64>,
    offspec_duration_ms: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Comparison {
    id: String,
    candidate_scenario_id: String,
    reference_scenario_id: String,
    expected_disposition: String,
}

#[derive(Debug, Serialize)]
pub struct ScenarioResult {
    pub id: String,
    pub disposition: String,
    pub expected_disposition: String,
    pub reasons: Vec<String>,
    pub nh3_balance_residual_g: Option<f64>,
    pub reaction_stoichiometry_residual_g: Option<f64>,
    pub h2_separation_residual_g: Option<f64>,
    pub n2_separation_residual_g: Option<f64>,
    pub accepted_h2_specific_energy_kwh_per_kg: Option<f64>,
    pub accepted_h2_flow_g_per_hour: Option<f64>,
    pub customer_delivery_settling_ms: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct ComparisonResult {
    pub id: String,
    pub disposition: String,
    pub expected_disposition: String,
    pub reasons: Vec<String>,
    pub candidate_specific_energy_kwh_per_kg: Option<f64>,
    pub reference_specific_energy_kwh_per_kg: Option<f64>,
    pub candidate_vs_reference_energy_delta_fraction: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct AuditReport {
    pub benchmark_id: String,
    pub evidence_class: String,
    pub physical_measurements: bool,
    pub luminous_measurements: bool,
    pub scenario_count: usize,
    pub scenario_expectations_matched: usize,
    pub comparison_count: usize,
    pub comparison_expectations_matched: usize,
    pub scenarios: Vec<ScenarioResult>,
    pub comparisons: Vec<ComparisonResult>,
}

#[derive(Debug)]
struct InternalScenario {
    result: ScenarioResult,
    feed_contract_id: String,
    product_contract_id: String,
    energy_boundary_id: String,
    duty_cycle_id: String,
}

fn non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_nonnegative(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

fn close_enough(left: f64, right: f64, tolerance: f64) -> bool {
    (left - right).abs() <= tolerance.max(REL_TOL * left.abs().max(right.abs()).max(1.0))
}

fn complete_energy_kwh(
    energy: &EnergyBoundary,
    blocked: &mut Vec<String>,
    rejected: &mut Vec<String>,
) -> Option<f64> {
    if !non_empty(&energy.boundary_id)
        || energy.meter_mode != "non_overlapping_grid_side_meters"
        || energy.meter_calibration_status != "synthetic_declared"
    {
        blocked.push(
            "grid-side energy boundary, meter mode or calibration status is incomplete".to_owned(),
        );
        return None;
    }
    if energy.meters.is_empty() || energy.required_roles.is_empty() {
        blocked.push("grid energy meters or required subsystem roles are absent".to_owned());
        return None;
    }

    let mut meter_ids = BTreeSet::new();
    let mut roles = BTreeSet::new();
    let mut total = 0.0;
    for meter in &energy.meters {
        if !non_empty(&meter.meter_id)
            || !meter_ids.insert(meter.meter_id.as_str())
            || !non_empty(&meter.role)
            || !roles.insert(meter.role.as_str())
        {
            rejected
                .push("energy meter identity or subsystem role is empty or duplicated".to_owned());
        }
        if !valid_nonnegative(meter.energy_kwh) {
            rejected.push("energy meter reading is non-finite or negative".to_owned());
        } else {
            total += meter.energy_kwh;
        }
    }

    let mut required = BTreeSet::new();
    if energy
        .required_roles
        .iter()
        .any(|role| !non_empty(role) || !required.insert(role.as_str()))
    {
        rejected.push("required energy roles are empty or duplicated".to_owned());
    }
    if required != roles {
        blocked.push(
            "grid energy boundary does not contain exactly one meter for every required role"
                .to_owned(),
        );
    }
    if !total.is_finite() || total <= 0.0 {
        rejected.push("total grid energy must be finite and positive".to_owned());
        None
    } else if !blocked.is_empty() || !rejected.is_empty() {
        None
    } else {
        Some(total)
    }
}

fn validate_dynamic_service(
    dynamic: &DynamicService,
    product: &ProductContract,
    blocked: &mut Vec<String>,
    rejected: &mut Vec<String>,
) -> Option<f64> {
    if dynamic.claimed_response_measurand != "accepted_h2_delivery"
        || dynamic.actual_evidence_layer != "customer_product_metered"
    {
        if dynamic.claimed_response_measurand == "accepted_h2_delivery"
            && dynamic.actual_evidence_layer != "customer_product_metered"
        {
            rejected.push("diagnostic/proxy response cannot be promoted to accepted-H2 customer delivery response".to_owned());
        } else {
            blocked.push(
                "dynamic response is not measured at the accepted-H2 customer boundary".to_owned(),
            );
        }
        return None;
    }

    let refs = [
        dynamic.clock_sync_ref.as_deref(),
        dynamic.product_flow_measurement_ref.as_deref(),
        dynamic.customer_boundary_ref.as_deref(),
    ];
    if refs
        .iter()
        .any(|value| value.is_none_or(|item| !non_empty(item)))
    {
        blocked.push("dynamic response lacks synchronized clock, product-flow measurement or customer-boundary identity".to_owned());
    }

    let raw_times = [
        dynamic.power_command_ms,
        dynamic.grid_power_measured_ms,
        dynamic.conversion_state_ms,
        dynamic.diagnostic_proxy_ms,
        dynamic.measured_gas_ms,
        dynamic.purified_h2_ms,
        dynamic.customer_accepted_h2_ms,
        dynamic.customer_delivery_settled_ms,
    ];
    if raw_times.iter().any(Option::is_none) {
        blocked.push(
            "dynamic response is missing one or more command-to-customer event timestamps"
                .to_owned(),
        );
        return None;
    }
    let times: Vec<f64> = raw_times.into_iter().flatten().collect();
    if times.iter().any(|value| !valid_nonnegative(*value)) {
        rejected.push("dynamic response contains a negative or non-finite timestamp".to_owned());
        return None;
    }
    if times.windows(2).any(|window| window[1] < window[0]) {
        rejected.push("dynamic response timestamps are not monotonically ordered".to_owned());
        return None;
    }

    match (
        dynamic.minimum_transient_purity_observed,
        dynamic.product_flow_overshoot_fraction,
        dynamic.offspec_duration_ms,
    ) {
        (Some(purity), Some(overshoot), Some(offspec))
            if purity.is_finite()
                && purity > 0.0
                && purity <= 1.0
                && overshoot.is_finite()
                && overshoot >= 0.0
                && offspec.is_finite()
                && offspec >= 0.0 =>
        {
            if purity < product.minimum_transient_purity_fraction {
                rejected.push(
                    "transient H2 purity falls below the accepted service contract".to_owned(),
                );
            }
            if overshoot > product.maximum_flow_overshoot_fraction {
                rejected.push(
                    "customer-boundary H2 flow overshoot exceeds product contract".to_owned(),
                );
            }
            if offspec > product.maximum_offspec_duration_ms {
                rejected.push("off-spec H2 duration exceeds product contract".to_owned());
            }
        }
        _ => blocked.push(
            "dynamic response lacks valid transient purity, flow overshoot or off-spec duration"
                .to_owned(),
        ),
    }

    Some(times[7] - times[0])
}

fn evaluate_scenario(
    scenario: &Scenario,
    source_registry: &BTreeMap<&str, &RouteSource>,
) -> InternalScenario {
    let mut blocked = Vec::new();
    let mut rejected = Vec::new();
    let mut nh3_balance_residual_g = None;
    let mut reaction_stoichiometry_residual_g = None;
    let mut h2_separation_residual_g = None;
    let mut n2_separation_residual_g = None;
    let mut specific_energy = None;
    let mut accepted_h2_flow_g_per_hour = None;
    let mut customer_delivery_settling_ms = None;

    if [
        scenario.id.as_str(),
        scenario.route_id.as_str(),
        scenario.operating_point_id.as_str(),
        scenario.feed_contract_id.as_str(),
        scenario.product_contract_id.as_str(),
        scenario.energy_boundary_id.as_str(),
        scenario.duty_cycle_id.as_str(),
        scenario.product.contract_id.as_str(),
    ]
    .iter()
    .any(|value| !non_empty(value))
    {
        rejected.push("scenario, route, operating-point or contract identity is empty".to_owned());
    }
    if scenario.evidence_class != "synthetic" {
        rejected.push("this evaluator accepts synthetic scenarios only".to_owned());
    }
    if scenario.product.contract_id != scenario.product_contract_id {
        rejected
            .push("scenario product-contract identity does not match product record".to_owned());
    }

    match (
        scenario.source_case_id.as_deref(),
        scenario.source_operating_point_id.as_deref(),
    ) {
        (None, None) => {}
        (Some(case_id), Some(point_id)) => {
            if let Some(source) = source_registry.get(case_id) {
                if source.route_id != scenario.route_id {
                    rejected.push(
                        "source case route identity does not match scenario route".to_owned(),
                    );
                }
                if !source
                    .allowed_operating_point_ids
                    .iter()
                    .any(|value| value == point_id)
                {
                    rejected.push(
                        "source operating-point identity is not registered for source case"
                            .to_owned(),
                    );
                }
            } else {
                rejected.push("scenario references an unknown source case".to_owned());
            }
        }
        _ => blocked.push(
            "source case and source operating-point identities must be supplied together"
                .to_owned(),
        ),
    }

    if !scenario.feed.unknown_streams.is_empty() {
        blocked.push("NH3 feed, reacted, recovered, purge or slip stream is unknown".to_owned());
    } else if !scenario.feed.tolerance_g.is_finite() || scenario.feed.tolerance_g < 0.0 {
        rejected.push("NH3 balance tolerance is invalid".to_owned());
    } else {
        match (
            scenario.feed.nh3_input_mass_g,
            scenario.feed.nh3_reacted_mass_g,
            scenario.feed.nh3_recovered_mass_g,
            scenario.feed.nh3_purge_mass_g,
            scenario.feed.nh3_slip_mass_g,
        ) {
            (Some(input), Some(reacted), Some(recovered), Some(purge), Some(slip)) => {
                if [input, reacted, recovered, purge, slip]
                    .iter()
                    .any(|value| !valid_nonnegative(*value))
                    || input <= 0.0
                {
                    rejected.push(
                        "NH3 ledger contains non-finite/negative mass or zero feed".to_owned(),
                    );
                } else {
                    let residual = input - reacted - recovered - purge - slip;
                    nh3_balance_residual_g = Some(residual);
                    if residual.abs() > scenario.feed.tolerance_g {
                        rejected.push(
                            "NH3 feed/reaction/recovery/purge/slip balance does not close"
                                .to_owned(),
                        );
                    }
                    if let Some(reported) = scenario.feed.reported_conversion_fraction {
                        let actual = reacted / input;
                        if !reported.is_finite()
                            || !(0.0..=1.0).contains(&reported)
                            || !close_enough(actual, reported, 1.0e-6)
                        {
                            rejected.push(
                                "reported NH3 conversion does not match feed ledger".to_owned(),
                            );
                        }
                    }
                }
            }
            _ => blocked
                .push("NH3 feed, reacted, recovered, purge or slip stream is unknown".to_owned()),
        }
    }

    match (
        scenario.feed.nh3_reacted_mass_g,
        scenario.reaction.h2_generated_mass_g,
        scenario.reaction.n2_generated_mass_g,
    ) {
        (Some(reacted), Some(h2), Some(n2))
            if valid_nonnegative(reacted) && valid_nonnegative(h2) && valid_nonnegative(n2) =>
        {
            let residual = reacted - h2 - n2;
            reaction_stoichiometry_residual_g = Some(residual);
            if residual.abs() > scenario.feed.tolerance_g {
                rejected.push("NH3 decomposition reaction mass does not close".to_owned());
            }
            let expected_h2 = reacted * (3.0 * MW_H2) / (2.0 * MW_NH3);
            let expected_n2 = reacted * MW_N2 / (2.0 * MW_NH3);
            if !close_enough(h2, expected_h2, scenario.feed.tolerance_g)
                || !close_enough(n2, expected_n2, scenario.feed.tolerance_g)
            {
                rejected.push(
                    "reported H2/N2 generation does not match NH3 decomposition stoichiometry"
                        .to_owned(),
                );
            }
        }
        _ => blocked.push("reaction H2 or N2 generation is unknown".to_owned()),
    }

    match (
        scenario.reaction.h2_generated_mass_g,
        scenario.separation.h2_accepted_mass_g,
        scenario.separation.h2_rejected_mass_g,
        scenario.separation.h2_unrecovered_mass_g,
    ) {
        (Some(generated), Some(accepted), Some(rejected_h2), Some(unrecovered))
            if [generated, accepted, rejected_h2, unrecovered]
                .iter()
                .all(|value| valid_nonnegative(*value)) =>
        {
            let residual = generated - accepted - rejected_h2 - unrecovered;
            h2_separation_residual_g = Some(residual);
            if residual.abs() > scenario.separation.tolerance_g {
                rejected.push(
                    "H2 generation/separation/rejection/loss balance does not close".to_owned(),
                );
            }
        }
        _ => blocked
            .push("H2 generated, accepted, rejected or unrecovered stream is unknown".to_owned()),
    }

    match (
        scenario.reaction.n2_generated_mass_g,
        scenario.separation.n2_recovered_mass_g,
        scenario.separation.n2_purge_mass_g,
    ) {
        (Some(generated), Some(recovered), Some(purge))
            if [generated, recovered, purge]
                .iter()
                .all(|value| valid_nonnegative(*value)) =>
        {
            let residual = generated - recovered - purge;
            n2_separation_residual_g = Some(residual);
            if residual.abs() > scenario.separation.tolerance_g {
                rejected.push("N2 generation/recovery/purge balance does not close".to_owned());
            }
        }
        _ => blocked.push("N2 generation, recovery or purge stream is unknown".to_owned()),
    }
    if !scenario.separation.unknown_streams.is_empty() {
        blocked.push("separation ledger contains unknown streams".to_owned());
    }

    let total_energy_kwh = complete_energy_kwh(&scenario.energy, &mut blocked, &mut rejected);
    if scenario.energy.boundary_id != scenario.energy_boundary_id {
        rejected.push(
            "scenario energy-boundary identity does not match energy meter ledger".to_owned(),
        );
    }

    match (
        scenario.product.accepted_h2_mass_g,
        scenario.product.gross_product_stream_mass_g,
        scenario.product.reported_h2_purity_fraction,
        scenario.product.delivery_pressure_bar_abs,
        scenario.product.delivery_interval_s,
        scenario.product.quantitative_product_measurement_ref.as_deref(),
        scenario.product.calibration_ref.as_deref(),
        scenario.separation.h2_accepted_mass_g,
    ) {
        (Some(h2_mass), Some(gross_mass), Some(reported_purity), Some(pressure), Some(interval), Some(measurement), Some(calibration), Some(separated_h2)) => {
            if ![h2_mass, gross_mass, reported_purity, pressure, interval].iter().all(|value| value.is_finite())
                || h2_mass <= 0.0
                || gross_mass < h2_mass
                || reported_purity <= 0.0
                || reported_purity > 1.0
                || pressure <= 0.0
                || interval <= 0.0
                || scenario.product.minimum_h2_purity_fraction <= 0.0
                || scenario.product.minimum_h2_purity_fraction > 1.0
                || scenario.product.minimum_delivery_pressure_bar_abs <= 0.0
            {
                rejected.push("accepted H2 product mass, purity, pressure or interval is invalid".to_owned());
            } else {
                let actual_purity = h2_mass / gross_mass;
                if !close_enough(actual_purity, reported_purity, 1.0e-6) {
                    rejected.push("reported H2 purity does not match H2/gross product mass".to_owned());
                }
                if !close_enough(h2_mass, separated_h2, scenario.separation.tolerance_g) {
                    rejected.push("accepted H2 product mass does not match separated H2 mass".to_owned());
                }
                if actual_purity < scenario.product.minimum_h2_purity_fraction {
                    rejected.push("accepted H2 product does not meet minimum purity contract".to_owned());
                }
                if pressure < scenario.product.minimum_delivery_pressure_bar_abs {
                    rejected.push("accepted H2 product does not meet minimum delivery pressure contract".to_owned());
                }
                if !non_empty(measurement) || !non_empty(calibration) {
                    blocked.push("quantitative H2 product measurement or calibration reference is empty".to_owned());
                }
                accepted_h2_flow_g_per_hour = Some(h2_mass / interval * 3600.0);
                if let Some(energy) = total_energy_kwh {
                    specific_energy = Some(energy / (h2_mass / 1000.0));
                }
            }
        }
        _ => blocked.push("accepted H2 product lacks mass, purity, pressure, interval, measurement, calibration or separation identity".to_owned()),
    }

    if scenario.dynamic_service_claimed {
        match &scenario.dynamic_service {
            Some(dynamic) => {
                customer_delivery_settling_ms = validate_dynamic_service(
                    dynamic,
                    &scenario.product,
                    &mut blocked,
                    &mut rejected,
                );
            }
            None => blocked.push(
                "dynamic accepted-H2 service is claimed without customer-boundary event trace"
                    .to_owned(),
            ),
        }
    }

    let (disposition, reasons) = if !rejected.is_empty() {
        ("REJECTED".to_owned(), rejected)
    } else if !blocked.is_empty() {
        ("BLOCKED".to_owned(), blocked)
    } else {
        ("SYNTHETIC_ACCOUNTING_PASS".to_owned(), Vec::new())
    };

    InternalScenario {
        result: ScenarioResult {
            id: scenario.id.clone(),
            disposition,
            expected_disposition: scenario.expected_disposition.clone(),
            reasons,
            nh3_balance_residual_g,
            reaction_stoichiometry_residual_g,
            h2_separation_residual_g,
            n2_separation_residual_g,
            accepted_h2_specific_energy_kwh_per_kg: specific_energy,
            accepted_h2_flow_g_per_hour,
            customer_delivery_settling_ms,
        },
        feed_contract_id: scenario.feed_contract_id.clone(),
        product_contract_id: scenario.product_contract_id.clone(),
        energy_boundary_id: scenario.energy_boundary_id.clone(),
        duty_cycle_id: scenario.duty_cycle_id.clone(),
    }
}

fn evaluate_comparison(
    comparison: &Comparison,
    scenarios: &BTreeMap<&str, &InternalScenario>,
) -> ComparisonResult {
    let Some(candidate) = scenarios.get(comparison.candidate_scenario_id.as_str()) else {
        return ComparisonResult {
            id: comparison.id.clone(),
            disposition: "BLOCKED".to_owned(),
            expected_disposition: comparison.expected_disposition.clone(),
            reasons: vec!["candidate scenario identity does not resolve".to_owned()],
            candidate_specific_energy_kwh_per_kg: None,
            reference_specific_energy_kwh_per_kg: None,
            candidate_vs_reference_energy_delta_fraction: None,
        };
    };
    let Some(reference) = scenarios.get(comparison.reference_scenario_id.as_str()) else {
        return ComparisonResult {
            id: comparison.id.clone(),
            disposition: "BLOCKED".to_owned(),
            expected_disposition: comparison.expected_disposition.clone(),
            reasons: vec!["reference scenario identity does not resolve".to_owned()],
            candidate_specific_energy_kwh_per_kg: None,
            reference_specific_energy_kwh_per_kg: None,
            candidate_vs_reference_energy_delta_fraction: None,
        };
    };
    let contracts_match = candidate.feed_contract_id == reference.feed_contract_id
        && candidate.product_contract_id == reference.product_contract_id
        && candidate.energy_boundary_id == reference.energy_boundary_id
        && candidate.duty_cycle_id == reference.duty_cycle_id;
    if !contracts_match {
        return ComparisonResult {
            id: comparison.id.clone(),
            disposition: "NOT_COMPARABLE".to_owned(),
            expected_disposition: comparison.expected_disposition.clone(),
            reasons: vec![
                "feed, accepted-product, grid-energy or duty-cycle contracts differ".to_owned(),
            ],
            candidate_specific_energy_kwh_per_kg: candidate
                .result
                .accepted_h2_specific_energy_kwh_per_kg,
            reference_specific_energy_kwh_per_kg: reference
                .result
                .accepted_h2_specific_energy_kwh_per_kg,
            candidate_vs_reference_energy_delta_fraction: None,
        };
    }
    if candidate.result.disposition != "SYNTHETIC_ACCOUNTING_PASS"
        || reference.result.disposition != "SYNTHETIC_ACCOUNTING_PASS"
    {
        return ComparisonResult {
            id: comparison.id.clone(),
            disposition: "BLOCKED".to_owned(),
            expected_disposition: comparison.expected_disposition.clone(),
            reasons: vec!["both scenarios must independently pass before comparison".to_owned()],
            candidate_specific_energy_kwh_per_kg: candidate
                .result
                .accepted_h2_specific_energy_kwh_per_kg,
            reference_specific_energy_kwh_per_kg: reference
                .result
                .accepted_h2_specific_energy_kwh_per_kg,
            candidate_vs_reference_energy_delta_fraction: None,
        };
    }
    let candidate_energy = candidate.result.accepted_h2_specific_energy_kwh_per_kg;
    let reference_energy = reference.result.accepted_h2_specific_energy_kwh_per_kg;
    let delta = match (candidate_energy, reference_energy) {
        (Some(candidate_value), Some(reference_value)) if reference_value > 0.0 => {
            Some(candidate_value / reference_value - 1.0)
        }
        _ => None,
    };
    ComparisonResult {
        id: comparison.id.clone(),
        disposition: "SYNTHETIC_COMPARABLE_ACCOUNTING_ONLY".to_owned(),
        expected_disposition: comparison.expected_disposition.clone(),
        reasons: Vec::new(),
        candidate_specific_energy_kwh_per_kg: candidate_energy,
        reference_specific_energy_kwh_per_kg: reference_energy,
        candidate_vs_reference_energy_delta_fraction: delta,
    }
}

pub fn audit_manifest_json(source: &str) -> Result<AuditReport, String> {
    let manifest: Manifest =
        serde_json::from_str(source).map_err(|error| format!("invalid manifest: {error}"))?;
    if manifest.schema_version != SCHEMA_VERSION {
        return Err(format!(
            "unsupported schema version: {}",
            manifest.schema_version
        ));
    }
    if manifest.benchmark_id != BENCHMARK_ID {
        return Err(format!(
            "unexpected benchmark ID: {}",
            manifest.benchmark_id
        ));
    }
    if !non_empty(&manifest.frozen_at) {
        return Err("manifest frozen_at is empty".to_owned());
    }
    if manifest.evidence_class != "synthetic"
        || manifest.physical_measurements
        || manifest.luminous_measurements
    {
        return Err(
            "evaluator accepts synthetic-only fixtures and refuses physical/Luminous claims"
                .to_owned(),
        );
    }
    if manifest.scenarios.is_empty() {
        return Err("manifest contains no scenarios".to_owned());
    }

    let mut source_registry = BTreeMap::new();
    for source in &manifest.route_sources {
        if !non_empty(&source.source_case_id)
            || !non_empty(&source.route_id)
            || !non_empty(&source.evidence_class)
            || !non_empty(&source.claim_ceiling)
            || source.allowed_operating_point_ids.is_empty()
            || source.luminous_reproduction
        {
            return Err(
                "route source registry contains incomplete identity or unsupported claim ceiling"
                    .to_owned(),
            );
        }
        let point_ids: BTreeSet<_> = source.allowed_operating_point_ids.iter().collect();
        if point_ids.len() != source.allowed_operating_point_ids.len() {
            return Err(format!(
                "source {} has duplicate operating-point IDs",
                source.source_case_id
            ));
        }
        if source_registry
            .insert(source.source_case_id.as_str(), source)
            .is_some()
        {
            return Err("route source registry contains duplicate source-case IDs".to_owned());
        }
    }

    let mut scenario_ids = BTreeSet::new();
    for scenario in &manifest.scenarios {
        if !non_empty(&scenario.id) || !scenario_ids.insert(scenario.id.as_str()) {
            return Err("scenario IDs must be non-empty and unique".to_owned());
        }
        if !matches!(
            scenario.expected_disposition.as_str(),
            "SYNTHETIC_ACCOUNTING_PASS" | "BLOCKED" | "REJECTED"
        ) {
            return Err(format!(
                "scenario {} has unsupported expected disposition",
                scenario.id
            ));
        }
    }
    let mut comparison_ids = BTreeSet::new();
    for comparison in &manifest.comparisons {
        if !non_empty(&comparison.id) || !comparison_ids.insert(comparison.id.as_str()) {
            return Err("comparison IDs must be non-empty and unique".to_owned());
        }
        if !matches!(
            comparison.expected_disposition.as_str(),
            "SYNTHETIC_COMPARABLE_ACCOUNTING_ONLY" | "NOT_COMPARABLE" | "BLOCKED"
        ) {
            return Err(format!(
                "comparison {} has unsupported expected disposition",
                comparison.id
            ));
        }
    }

    let internal: Vec<InternalScenario> = manifest
        .scenarios
        .iter()
        .map(|scenario| evaluate_scenario(scenario, &source_registry))
        .collect();
    let scenario_expectations_matched = internal
        .iter()
        .filter(|value| value.result.disposition == value.result.expected_disposition)
        .count();
    let scenario_map: BTreeMap<&str, &InternalScenario> = internal
        .iter()
        .map(|value| (value.result.id.as_str(), value))
        .collect();
    let comparisons: Vec<ComparisonResult> = manifest
        .comparisons
        .iter()
        .map(|comparison| evaluate_comparison(comparison, &scenario_map))
        .collect();
    let comparison_expectations_matched = comparisons
        .iter()
        .filter(|value| value.disposition == value.expected_disposition)
        .count();

    let scenario_count = internal.len();
    let comparison_count = comparisons.len();
    Ok(AuditReport {
        benchmark_id: manifest.benchmark_id,
        evidence_class: manifest.evidence_class,
        physical_measurements: manifest.physical_measurements,
        luminous_measurements: manifest.luminous_measurements,
        scenario_count,
        scenario_expectations_matched,
        comparison_count,
        comparison_expectations_matched,
        scenarios: internal.into_iter().map(|value| value.result).collect(),
        comparisons,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    const FIXTURE: &str = include_str!("../../../../benchmarks/ammonia/nh3-h2-service-v0.json");

    fn fixture_value() -> Value {
        serde_json::from_str(FIXTURE).expect("fixture JSON must parse")
    }

    fn scenario_for<'a>(report: &'a AuditReport, id: &str) -> &'a ScenarioResult {
        report
            .scenarios
            .iter()
            .find(|value| value.id == id)
            .expect("scenario exists")
    }

    fn comparison_for<'a>(report: &'a AuditReport, id: &str) -> &'a ComparisonResult {
        report
            .comparisons
            .iter()
            .find(|value| value.id == id)
            .expect("comparison exists")
    }

    #[test]
    fn frozen_service_fixture_matches_expected_dispositions() {
        let report = audit_manifest_json(FIXTURE).expect("manifest should audit");
        assert_eq!(report.scenario_count, 10);
        assert_eq!(report.scenario_expectations_matched, 10);
        assert_eq!(report.comparison_count, 2);
        assert_eq!(report.comparison_expectations_matched, 2);
        assert!(!report.physical_measurements);
        assert!(!report.luminous_measurements);
    }

    #[test]
    fn positive_service_case_closes_material_and_energy_ledgers() {
        let report = audit_manifest_json(FIXTURE).expect("manifest should audit");
        let result = scenario_for(&report, "synthetic-plasma-h2-service-positive");
        assert_eq!(result.disposition, "SYNTHETIC_ACCOUNTING_PASS");
        assert!(result.nh3_balance_residual_g.unwrap().abs() < 1.0e-6);
        assert!(result.reaction_stoichiometry_residual_g.unwrap().abs() < 1.0e-6);
        assert!(result.h2_separation_residual_g.unwrap().abs() < 1.0e-6);
        assert!(result.n2_separation_residual_g.unwrap().abs() < 1.0e-6);
        assert!(result.accepted_h2_specific_energy_kwh_per_kg.unwrap() > 0.0);
        assert_eq!(result.customer_delivery_settling_ms, Some(3200.0));
    }

    #[test]
    fn matched_service_contracts_produce_synthetic_comparable_accounting() {
        let report = audit_manifest_json(FIXTURE).expect("manifest should audit");
        let result = comparison_for(&report, "synthetic-plasma-vs-electrochemical");
        assert_eq!(result.disposition, "SYNTHETIC_COMPARABLE_ACCOUNTING_ONLY");
        assert!(
            result
                .candidate_vs_reference_energy_delta_fraction
                .is_some()
        );
    }

    #[test]
    fn product_contract_mismatch_is_not_comparable() {
        let report = audit_manifest_json(FIXTURE).expect("manifest should audit");
        let result = comparison_for(&report, "not-comparable-different-product-contract");
        assert_eq!(result.disposition, "NOT_COMPARABLE");
        assert_eq!(result.candidate_vs_reference_energy_delta_fraction, None);
    }

    #[test]
    fn incomplete_energy_boundary_blocks_route_accounting() {
        let report = audit_manifest_json(FIXTURE).expect("manifest should audit");
        assert_eq!(
            scenario_for(&report, "blocked-missing-compression-meter").disposition,
            "BLOCKED"
        );
    }

    #[test]
    fn under_spec_h2_product_is_rejected() {
        let report = audit_manifest_json(FIXTURE).expect("manifest should audit");
        assert_eq!(
            scenario_for(&report, "rejected-under-spec-h2-product").disposition,
            "REJECTED"
        );
    }

    #[test]
    fn unknown_recycle_stream_blocks_claim() {
        let report = audit_manifest_json(FIXTURE).expect("manifest should audit");
        assert_eq!(
            scenario_for(&report, "blocked-unknown-nh3-recycle-stream").disposition,
            "BLOCKED"
        );
    }

    #[test]
    fn source_operating_point_cross_pairing_is_rejected() {
        let report = audit_manifest_json(FIXTURE).expect("manifest should audit");
        assert_eq!(
            scenario_for(&report, "rejected-cross-paired-source-operating-point").disposition,
            "REJECTED"
        );
    }

    #[test]
    fn stoichiometric_h2_overclaim_is_rejected() {
        let report = audit_manifest_json(FIXTURE).expect("manifest should audit");
        assert_eq!(
            scenario_for(&report, "rejected-stoichiometric-h2-overclaim").disposition,
            "REJECTED"
        );
    }

    #[test]
    fn diagnostic_proxy_cannot_be_promoted_to_customer_service() {
        let report = audit_manifest_json(FIXTURE).expect("manifest should audit");
        assert_eq!(
            scenario_for(&report, "rejected-diagnostic-proxy-promotion").disposition,
            "REJECTED"
        );
    }

    #[test]
    fn missing_product_calibration_blocks_accounting() {
        let mut value = fixture_value();
        value["scenarios"][0]["product"]["calibration_ref"] = json!(null);
        let report = audit_manifest_json(&value.to_string()).expect("mutated manifest parses");
        assert_eq!(
            scenario_for(&report, "synthetic-plasma-h2-service-positive").disposition,
            "BLOCKED"
        );
    }

    #[test]
    fn source_manifest_cannot_claim_physical_measurements() {
        let mut value = fixture_value();
        value["physical_measurements"] = json!(true);
        assert!(audit_manifest_json(&value.to_string()).is_err());
    }

    #[test]
    fn unknown_manifest_fields_fail_closed() {
        let mut value = fixture_value();
        value["unexpected_claim"] = json!("must fail");
        assert!(audit_manifest_json(&value.to_string()).is_err());
    }

    #[test]
    fn source_registry_rejects_duplicate_operating_points() {
        let mut value = fixture_value();
        value["route_sources"][0]["allowed_operating_point_ids"][0] =
            value["route_sources"][0]["allowed_operating_point_ids"][1].clone();
        assert!(audit_manifest_json(&value.to_string()).is_err());
    }

    #[test]
    fn dynamic_response_requires_customer_acceptance_timestamp() {
        let mut value = fixture_value();
        value["scenarios"][0]["dynamic_service"]["customer_accepted_h2_ms"] = json!(null);
        let report = audit_manifest_json(&value.to_string()).expect("mutated manifest parses");
        assert_eq!(
            scenario_for(&report, "synthetic-plasma-h2-service-positive").disposition,
            "BLOCKED"
        );
    }
}
