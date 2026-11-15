use serde::{Deserialize, Serialize};
use std::collections::HashSet;

const SCHEMA_VERSION: &str = "luminous.lunar-plasma-isru-process.v0";
const BENCHMARK_ID: &str = "LUNAR-PLASMA-PROCESS-TWIN-001";
const REL_TOLERANCE: f64 = 1.0e-9;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    schema_version: String,
    benchmark_id: String,
    frozen_at: String,
    evidence_class: String,
    physical_measurements: bool,
    plasma_solver_outputs: bool,
    comparison_policy: String,
    scenarios: Vec<Scenario>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    id: String,
    evidence_class: String,
    feedstock: Option<Feedstock>,
    feedstock_mass_balance: Option<MassBalance>,
    reagent_ledger: Option<ReagentLedger>,
    observations: Option<Vec<Observation>>,
    product: Option<Product>,
    coproducts: Option<Vec<Coproduct>>,
    grid_energy: Option<GridEnergy>,
    comparison: Option<Comparison>,
    expected_disposition: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Feedstock {
    batch_id: String,
    material_id: String,
    dry_mass_g: f64,
    mineralogy_status: String,
    source_ref: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MassBalance {
    input_mass_g: f64,
    outputs: Vec<MaterialStream>,
    tolerance_g: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialStream {
    stream_id: String,
    kind: String,
    mass_g: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReagentLedger {
    opening_inventory_g: Option<f64>,
    fresh_makeup_g: Option<f64>,
    ending_inventory_g: Option<f64>,
    bound_in_residue_g: Option<f64>,
    purge_and_waste_g: Option<f64>,
    unknown_streams: Vec<String>,
    tolerance_g: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    observation_id: String,
    method: String,
    species: String,
    evidence_class: String,
    #[serde(default)]
    calibration_ref: Option<String>,
    #[serde(default)]
    calibration_species: Option<String>,
    #[serde(default)]
    calibration_method: Option<String>,
    #[serde(default)]
    calibration_valid_from_utc: Option<String>,
    #[serde(default)]
    calibration_valid_until_utc: Option<String>,
    #[serde(default)]
    observed_at_utc: Option<String>,
    #[serde(default)]
    expanded_uncertainty_fraction: Option<f64>,
    #[serde(default)]
    decision_margin_fraction: Option<f64>,
    #[serde(default)]
    measured_mass_g: Option<f64>,
    #[serde(default)]
    sampling_recovery_fraction: Option<f64>,
    #[serde(default)]
    sampling_recovery_check_ref: Option<String>,
    #[serde(default)]
    cross_sensitivity_status: Option<String>,
    #[serde(default)]
    cross_sensitivity_check_ref: Option<String>,
    #[serde(default)]
    sampling_interval_id: Option<String>,
    #[serde(default)]
    raw_artifact_ref: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Product {
    product_id: String,
    species: String,
    mass_g: Option<f64>,
    purity_fraction: Option<f64>,
    pressure_kpa: Option<f64>,
    delivery_contract_id: Option<String>,
    quantitative_measurement_ref: Option<String>,
    calibration_ref: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Coproduct {
    stream_id: String,
    species_or_material: String,
    mass_g: f64,
    credited_as_product: bool,
    product_grade_ref: Option<String>,
    receiving_process_ref: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GridEnergy {
    boundary_id: String,
    meter_mode: String,
    meters: Vec<Meter>,
    required_roles: Vec<String>,
    meter_calibration_status: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Meter {
    meter_id: String,
    role: String,
    energy_wh: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Comparison {
    candidate_contract_id: String,
    reference_contract_id: String,
    energy_boundary_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScenarioResult {
    pub id: String,
    pub disposition: String,
    pub expected_disposition: String,
    pub reasons: Vec<String>,
    pub feedstock_balance_residual_g: Option<f64>,
    pub reagent_balance_residual_g: Option<f64>,
    pub synthetic_grid_energy_wh_per_kg_o2: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditReport {
    pub schema_version: String,
    pub benchmark_id: String,
    pub evidence_class: String,
    pub physical_measurements: bool,
    pub plasma_solver_outputs: bool,
    pub scenario_count: usize,
    pub matched_expectations: usize,
    pub mismatched_expectations: usize,
    pub results: Vec<ScenarioResult>,
}

fn close_enough(a: f64, b: f64, tolerance: f64) -> bool {
    (a - b).abs() <= tolerance.max(REL_TOLERANCE * a.abs().max(b.abs()).max(1.0))
}

fn non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_nonnegative(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

fn valid_utc_timestamp(value: &str) -> bool {
    let bytes = value.as_bytes();
    let shape_is_valid = bytes.len() == 20
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[10] == b'T'
        && bytes[13] == b':'
        && bytes[16] == b':'
        && bytes[19] == b'Z'
        && bytes.iter().enumerate().all(|(index, byte)| {
            matches!(index, 4 | 7 | 10 | 13 | 16 | 19) || byte.is_ascii_digit()
        });
    if !shape_is_valid {
        return false;
    }

    let Some(year) = value[0..4].parse::<u32>().ok() else {
        return false;
    };
    let Some(month) = value[5..7].parse::<u32>().ok() else {
        return false;
    };
    let Some(day) = value[8..10].parse::<u32>().ok() else {
        return false;
    };
    let Some(hour) = value[11..13].parse::<u32>().ok() else {
        return false;
    };
    let Some(minute) = value[14..16].parse::<u32>().ok() else {
        return false;
    };
    let Some(second) = value[17..19].parse::<u32>().ok() else {
        return false;
    };

    let leap_year = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap_year => 29,
        2 => 28,
        _ => return false,
    };

    year > 0 && day >= 1 && day <= days_in_month && hour <= 23 && minute <= 59 && second <= 59
}

fn observation_is_metrologically_scoreable(
    observation: &Observation,
    product_calibration_ref: &str,
    product_mass_g: f64,
    blocked: &mut Vec<String>,
    rejected: &mut Vec<String>,
) -> bool {
    let mut scoreable = true;

    let required_strings = [
        observation.calibration_ref.as_deref(),
        observation.calibration_species.as_deref(),
        observation.calibration_method.as_deref(),
        observation.calibration_valid_from_utc.as_deref(),
        observation.calibration_valid_until_utc.as_deref(),
        observation.observed_at_utc.as_deref(),
        observation.sampling_recovery_check_ref.as_deref(),
        observation.cross_sensitivity_status.as_deref(),
        observation.cross_sensitivity_check_ref.as_deref(),
        observation.sampling_interval_id.as_deref(),
        observation.raw_artifact_ref.as_deref(),
    ];
    if required_strings
        .iter()
        .any(|value| value.is_none_or(|s| !non_empty(s)))
    {
        blocked.push("quantitative observation is missing calibration, validity, sampling, interference or raw-artifact evidence".to_owned());
        scoreable = false;
    }

    if observation
        .calibration_ref
        .as_deref()
        .is_some_and(|calibration_ref| calibration_ref != product_calibration_ref)
    {
        rejected.push(
            "product calibration identity does not match the quantitative observation".to_owned(),
        );
        scoreable = false;
    }
    if observation.calibration_species.as_deref() != Some("O2") {
        rejected.push("calibration species scope does not match O2".to_owned());
        scoreable = false;
    }
    if observation.calibration_method.as_deref() != Some(observation.method.as_str()) {
        rejected.push("calibration method scope does not match the observation method".to_owned());
        scoreable = false;
    }

    match (
        observation.calibration_valid_from_utc.as_deref(),
        observation.calibration_valid_until_utc.as_deref(),
        observation.observed_at_utc.as_deref(),
    ) {
        (Some(start), Some(end), Some(observed))
            if valid_utc_timestamp(start)
                && valid_utc_timestamp(end)
                && valid_utc_timestamp(observed)
                && start <= observed
                && observed <= end => {}
        (Some(start), Some(end), Some(observed))
            if valid_utc_timestamp(start)
                && valid_utc_timestamp(end)
                && valid_utc_timestamp(observed) =>
        {
            blocked.push("calibration is not valid at the observation timestamp".to_owned());
            scoreable = false;
        }
        _ => {
            blocked.push(
                "calibration validity timestamps are missing or not normalized UTC timestamps"
                    .to_owned(),
            );
            scoreable = false;
        }
    }

    match (
        observation.expanded_uncertainty_fraction,
        observation.decision_margin_fraction,
    ) {
        (Some(uncertainty), Some(margin))
            if uncertainty.is_finite()
                && uncertainty >= 0.0
                && margin.is_finite()
                && margin > 0.0 =>
        {
            if uncertainty > margin / 3.0 {
                blocked.push("expanded measurement uncertainty exceeds one-third of the declared decision margin".to_owned());
                scoreable = false;
            }
        }
        _ => {
            blocked
                .push("expanded uncertainty or decision margin is missing or invalid".to_owned());
            scoreable = false;
        }
    }

    match (
        observation.measured_mass_g,
        observation.sampling_recovery_fraction,
        observation.sampling_recovery_check_ref.as_deref(),
    ) {
        (Some(measured), Some(recovery), Some(check_ref))
            if measured.is_finite()
                && measured > 0.0
                && recovery.is_finite()
                && recovery > 0.0
                && recovery <= 1.0
                && non_empty(check_ref) =>
        {
            let corrected_mass = measured / recovery;
            if !close_enough(corrected_mass, product_mass_g, 1.0e-6) {
                rejected.push(
                    "sampling-recovery-corrected gas mass does not match accepted O2 product mass"
                        .to_owned(),
                );
                scoreable = false;
            }
        }
        (Some(_), Some(recovery), _)
            if !recovery.is_finite() || recovery <= 0.0 || recovery > 1.0 =>
        {
            rejected.push("sampling recovery fraction is outside (0, 1]".to_owned());
            scoreable = false;
        }
        _ => {
            blocked.push("quantitative observation lacks measured mass or independently checked sampling recovery".to_owned());
            scoreable = false;
        }
    }

    match observation.cross_sensitivity_status.as_deref() {
        Some("passed")
            if observation
                .cross_sensitivity_check_ref
                .as_deref()
                .is_some_and(non_empty) => {}
        Some("failed") => {
            rejected.push("cross-sensitivity qualification failed".to_owned());
            scoreable = false;
        }
        _ => {
            blocked
                .push("species cross-sensitivity qualification is absent or incomplete".to_owned());
            scoreable = false;
        }
    }

    if observation
        .sampling_interval_id
        .as_deref()
        .is_none_or(|s| !non_empty(s))
        || observation
            .raw_artifact_ref
            .as_deref()
            .is_none_or(|s| !non_empty(s))
    {
        blocked.push("sampling interval or raw measurement artifact identity is absent".to_owned());
        scoreable = false;
    }

    scoreable
}

fn evaluate_scenario(scenario: &Scenario) -> ScenarioResult {
    let mut rejected = Vec::new();
    let mut blocked = Vec::new();
    let mut not_comparable = Vec::new();
    let mut feedstock_balance_residual_g = None;
    let mut reagent_balance_residual_g = None;
    let mut accepted_o2_mass_g = None;

    if !non_empty(&scenario.id) {
        rejected.push("scenario identity is empty".to_owned());
    }
    if scenario.evidence_class != "synthetic" {
        rejected.push("this evaluator accepts synthetic fixtures only".to_owned());
    }

    match &scenario.feedstock {
        None => blocked.push("feedstock identity and batch are absent".to_owned()),
        Some(feedstock) => {
            if !non_empty(&feedstock.batch_id)
                || !non_empty(&feedstock.material_id)
                || !non_empty(&feedstock.mineralogy_status)
                || !non_empty(&feedstock.source_ref)
            {
                blocked.push("feedstock provenance or mineralogy status is incomplete".to_owned());
            }
            if !feedstock.dry_mass_g.is_finite() || feedstock.dry_mass_g <= 0.0 {
                rejected.push("feedstock dry mass must be finite and positive".to_owned());
            }
        }
    }

    if let Some(balance) = &scenario.feedstock_mass_balance {
        if !balance.input_mass_g.is_finite()
            || balance.input_mass_g < 0.0
            || !balance.tolerance_g.is_finite()
            || balance.tolerance_g < 0.0
            || balance.outputs.is_empty()
        {
            rejected.push(
                "feedstock mass balance has invalid input, tolerance or empty outputs".to_owned(),
            );
        } else {
            if let Some(feedstock) = &scenario.feedstock {
                if !close_enough(balance.input_mass_g, feedstock.dry_mass_g, balance.tolerance_g) {
                    rejected.push("feedstock balance input does not match the declared dry batch mass".to_owned());
                }
            }
            let mut stream_ids = HashSet::new();
            let mut output_sum = 0.0;
            for stream in &balance.outputs {
                if !non_empty(&stream.stream_id)
                    || !non_empty(&stream.kind)
                    || !stream_ids.insert(stream.stream_id.as_str())
                {
                    rejected
                        .push("feedstock output stream identity is empty or duplicated".to_owned());
                }
                if !valid_nonnegative(stream.mass_g) {
                    rejected.push("feedstock output mass is non-finite or negative".to_owned());
                } else {
                    output_sum += stream.mass_g;
                }
            }
            let residual = balance.input_mass_g - output_sum;
            feedstock_balance_residual_g = Some(residual);
            if !residual.is_finite() || residual.abs() > balance.tolerance_g {
                rejected.push(
                    "feedstock mass balance does not close within its declared tolerance"
                        .to_owned(),
                );
            }
        }
    }

    if let Some(ledger) = &scenario.reagent_ledger {
        let inventory = [
            ledger.opening_inventory_g,
            ledger.fresh_makeup_g,
            ledger.ending_inventory_g,
            ledger.bound_in_residue_g,
            ledger.purge_and_waste_g,
        ];
        if !ledger.tolerance_g.is_finite() || ledger.tolerance_g < 0.0 {
            rejected.push("reagent balance tolerance is invalid".to_owned());
        } else if !ledger.unknown_streams.is_empty() || inventory.iter().any(Option::is_none) {
            blocked.push("reagent inventory contains unknown or missing streams".to_owned());
        } else {
            let values: Vec<f64> = inventory.into_iter().flatten().collect();
            if values.iter().any(|value| !valid_nonnegative(*value)) {
                rejected.push("reagent inventory contains non-finite or negative mass".to_owned());
            } else {
                let input = values[0] + values[1];
                let output = values[2] + values[3] + values[4];
                let residual = input - output;
                reagent_balance_residual_g = Some(residual);
                if !residual.is_finite() || residual.abs() > ledger.tolerance_g {
                    rejected.push(
                        "reagent inventory does not close within its declared tolerance".to_owned(),
                    );
                }
            }
        }
    }

    let total_grid_energy_wh = scenario
        .grid_energy
        .as_ref()
        .and_then(|grid| complete_grid_energy(grid, &mut blocked, &mut rejected));

    let observations = scenario.observations.as_deref().unwrap_or(&[]);
    let mut observation_ids = HashSet::new();
    for observation in observations {
        if !non_empty(&observation.observation_id)
            || !non_empty(&observation.method)
            || !non_empty(&observation.species)
            || !non_empty(&observation.evidence_class)
            || !observation_ids.insert(observation.observation_id.as_str())
        {
            rejected
                .push("observation identity or classification is empty or duplicated".to_owned());
        }
    }

    if let Some(product) = &scenario.product {
        let mut product_scoreable = true;
        if !non_empty(&product.product_id) {
            rejected.push("product identity is empty".to_owned());
            product_scoreable = false;
        }
        if product.species != "O2" {
            rejected
                .push("the accepted product contract requires O2 as a distinct species".to_owned());
            product_scoreable = false;
        }

        match (
            product.mass_g,
            product.purity_fraction,
            product.pressure_kpa,
            product.delivery_contract_id.as_deref(),
            product.quantitative_measurement_ref.as_deref(),
            product.calibration_ref.as_deref(),
        ) {
            (
                Some(mass),
                Some(purity),
                Some(pressure),
                Some(contract),
                Some(measurement),
                Some(calibration),
            ) => {
                if !mass.is_finite()
                    || mass <= 0.0
                    || !purity.is_finite()
                    || purity <= 0.0
                    || purity > 1.0
                    || !pressure.is_finite()
                    || pressure <= 0.0
                {
                    rejected.push(
                        "product mass, purity or pressure is outside the valid domain".to_owned(),
                    );
                    product_scoreable = false;
                }
                if !non_empty(contract) || !non_empty(measurement) || !non_empty(calibration) {
                    blocked.push("product contract, quantitative measurement or calibration reference is absent".to_owned());
                    product_scoreable = false;
                } else if let Some(observation) = observations
                    .iter()
                    .find(|obs| obs.observation_id == measurement)
                {
                    if observation.species != "O2" {
                        rejected.push(
                            "product measurement refers to a different gas species".to_owned(),
                        );
                        product_scoreable = false;
                    } else if observation.evidence_class != "synthetic"
                        || !observation
                            .method
                            .to_ascii_lowercase()
                            .contains("quantitative")
                        || !(observation.method.to_ascii_lowercase().contains("gas")
                            || observation
                                .method
                                .to_ascii_lowercase()
                                .contains("mass_flow"))
                    {
                        blocked.push("product measurement is not a synthetic quantitative gas or mass-flow observation".to_owned());
                        product_scoreable = false;
                    } else if !observation_is_metrologically_scoreable(
                        observation,
                        calibration,
                        mass,
                        &mut blocked,
                        &mut rejected,
                    ) {
                        product_scoreable = false;
                    }
                } else {
                    blocked.push("product quantitative measurement reference does not resolve to an observation".to_owned());
                    product_scoreable = false;
                }

                if let Some(balance) = &scenario.feedstock_mass_balance {
                    let product_streams: Vec<_> = balance
                        .outputs
                        .iter()
                        .filter(|stream| stream.kind == "accepted_o2_product")
                        .collect();
                    if product_streams.len() != 1 {
                        blocked.push(
                            "feedstock balance must contain exactly one accepted O2 product stream"
                                .to_owned(),
                        );
                        product_scoreable = false;
                    } else if !close_enough(product_streams[0].mass_g, mass, balance.tolerance_g) {
                        rejected.push(
                            "accepted O2 product mass does not match its feedstock output stream"
                                .to_owned(),
                        );
                        product_scoreable = false;
                    }
                } else {
                    blocked.push(
                        "accepted O2 product has no reconciled feedstock mass-balance stream"
                            .to_owned(),
                    );
                    product_scoreable = false;
                }

                if total_grid_energy_wh.is_none() {
                    blocked.push(
                        "complete grid-to-product energy boundary is required to score accepted O2"
                            .to_owned(),
                    );
                    product_scoreable = false;
                }
                if product_scoreable {
                    accepted_o2_mass_g = Some(mass);
                }
            }
            _ => {
                blocked.push("accepted O2 product is missing mass, purity, pressure, contract, quantitative measurement or calibration".to_owned());
            }
        }
    }

    if let Some(coproducts) = &scenario.coproducts {
        let mut coproduct_ids = HashSet::new();
        for coproduct in coproducts {
            if !non_empty(&coproduct.stream_id)
                || !non_empty(&coproduct.species_or_material)
                || !coproduct_ids.insert(coproduct.stream_id.as_str())
                || !valid_nonnegative(coproduct.mass_g)
            {
                rejected.push("coproduct stream identity or mass is invalid".to_owned());
            }
            if coproduct.credited_as_product
                && (coproduct
                    .product_grade_ref
                    .as_deref()
                    .is_none_or(|value| !non_empty(value))
                    || coproduct
                        .receiving_process_ref
                        .as_deref()
                        .is_none_or(|value| !non_empty(value)))
            {
                blocked.push(
                    "coproduct is credited without a qualified product grade and receiving process"
                        .to_owned(),
                );
            }
        }
    }

    if let Some(comparison) = &scenario.comparison {
        if !non_empty(&comparison.candidate_contract_id)
            || !non_empty(&comparison.reference_contract_id)
            || !non_empty(&comparison.energy_boundary_id)
        {
            blocked.push("comparison product or energy-boundary identity is incomplete".to_owned());
        } else if comparison.candidate_contract_id != comparison.reference_contract_id {
            not_comparable.push("candidate and reference product contracts differ".to_owned());
        }
    }

    if scenario.feedstock_mass_balance.is_some()
        && scenario.reagent_ledger.is_some()
        && scenario.product.is_some()
        && scenario.grid_energy.is_some()
    {
        // The individual validators above decide whether the complete chain is scoreable.
    } else if scenario.comparison.is_none() && scenario.coproducts.is_none() {
        blocked.push(
            "scenario does not contain a complete end-to-end process evidence chain".to_owned(),
        );
    }

    let (disposition, reasons) = if !rejected.is_empty() {
        ("REJECTED".to_owned(), rejected)
    } else if !blocked.is_empty() {
        ("BLOCKED".to_owned(), blocked)
    } else if !not_comparable.is_empty() {
        ("NOT_COMPARABLE".to_owned(), not_comparable)
    } else {
        ("SYNTHETIC_ACCOUNTING_PASS".to_owned(), Vec::new())
    };

    let synthetic_grid_energy_wh_per_kg_o2 = if disposition == "SYNTHETIC_ACCOUNTING_PASS" {
        match (total_grid_energy_wh, accepted_o2_mass_g) {
            (Some(energy_wh), Some(mass_g)) if mass_g > 0.0 => Some(energy_wh / (mass_g / 1000.0)),
            _ => None,
        }
    } else {
        None
    };

    ScenarioResult {
        id: scenario.id.clone(),
        disposition,
        expected_disposition: scenario.expected_disposition.clone(),
        reasons,
        feedstock_balance_residual_g,
        reagent_balance_residual_g,
        synthetic_grid_energy_wh_per_kg_o2,
    }
}

fn complete_grid_energy(
    grid: &GridEnergy,
    blocked: &mut Vec<String>,
    rejected: &mut Vec<String>,
) -> Option<f64> {
    if !non_empty(&grid.boundary_id)
        || grid.meter_mode != "non_overlapping_branch_meters"
        || !non_empty(&grid.meter_calibration_status)
    {
        blocked.push(
            "grid energy boundary identity, meter mode or calibration status is incomplete"
                .to_owned(),
        );
        return None;
    }
    if grid.meters.is_empty() || grid.required_roles.is_empty() {
        blocked.push("grid energy meters or required meter roles are absent".to_owned());
        return None;
    }

    let mut meter_ids = HashSet::new();
    let mut roles = HashSet::new();
    let mut total = 0.0;
    for meter in &grid.meters {
        if !non_empty(&meter.meter_id)
            || !meter_ids.insert(meter.meter_id.as_str())
            || !non_empty(&meter.role)
            || !roles.insert(meter.role.as_str())
        {
            rejected.push(
                "grid energy meter identity or subsystem role is empty or duplicated".to_owned(),
            );
        }
        if !valid_nonnegative(meter.energy_wh) {
            rejected.push("grid energy meter reading is non-finite or negative".to_owned());
        } else {
            total += meter.energy_wh;
        }
    }

    let mut required = HashSet::new();
    if grid
        .required_roles
        .iter()
        .any(|role| !non_empty(role) || !required.insert(role.as_str()))
    {
        rejected.push("required grid energy roles are empty or duplicated".to_owned());
    }
    if required != roles {
        blocked.push(
            "grid energy does not provide exactly one meter for every required subsystem role"
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
    if !non_empty(&manifest.frozen_at) || !non_empty(&manifest.comparison_policy) {
        return Err("manifest freeze date or comparison policy is missing".to_owned());
    }
    if manifest.evidence_class != "synthetic"
        || manifest.physical_measurements
        || manifest.plasma_solver_outputs
    {
        return Err("this evaluator is restricted to synthetic fixtures; physical/solver evidence is not accepted".to_owned());
    }
    if manifest.scenarios.is_empty() {
        return Err("manifest contains no scenarios".to_owned());
    }

    let mut scenario_ids = HashSet::new();
    for scenario in &manifest.scenarios {
        if !non_empty(&scenario.id) || !scenario_ids.insert(scenario.id.as_str()) {
            return Err("scenario IDs must be non-empty and unique".to_owned());
        }
        if !matches!(
            scenario.expected_disposition.as_str(),
            "SYNTHETIC_ACCOUNTING_PASS" | "BLOCKED" | "REJECTED" | "NOT_COMPARABLE"
        ) {
            return Err(format!(
                "scenario {} has unsupported expected disposition",
                scenario.id
            ));
        }
    }

    let results: Vec<ScenarioResult> = manifest.scenarios.iter().map(evaluate_scenario).collect();
    let matched_expectations = results
        .iter()
        .filter(|result| result.disposition == result.expected_disposition)
        .count();
    let scenario_count = results.len();

    Ok(AuditReport {
        schema_version: manifest.schema_version,
        benchmark_id: manifest.benchmark_id,
        evidence_class: manifest.evidence_class,
        physical_measurements: manifest.physical_measurements,
        plasma_solver_outputs: manifest.plasma_solver_outputs,
        scenario_count,
        matched_expectations,
        mismatched_expectations: scenario_count - matched_expectations,
        results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    const FIXTURE: &str =
        include_str!("../../../../benchmarks/plasma/lunar-plasma-isru-process-v0.json");

    fn fixture_value() -> Value {
        serde_json::from_str(FIXTURE).expect("fixture JSON must parse")
    }

    fn result_for<'a>(report: &'a AuditReport, id: &str) -> &'a ScenarioResult {
        report
            .results
            .iter()
            .find(|result| result.id == id)
            .expect("scenario exists")
    }

    #[test]
    fn frozen_fixture_matches_all_fifteen_dispositions() {
        let report = audit_manifest_json(FIXTURE).expect("manifest should audit");
        assert_eq!(report.scenario_count, 15);
        assert_eq!(report.matched_expectations, 15);
        assert_eq!(report.mismatched_expectations, 0);
        assert_eq!(report.evidence_class, "synthetic");
        assert!(!report.physical_measurements);
        assert!(!report.plasma_solver_outputs);
    }

    #[test]
    fn positive_control_closes_feed_reagent_and_energy_boundaries() {
        let report = audit_manifest_json(FIXTURE).expect("manifest should audit");
        let result = result_for(&report, "synthetic-balanced-accounting-positive");
        assert_eq!(result.disposition, "SYNTHETIC_ACCOUNTING_PASS");
        assert_eq!(result.feedstock_balance_residual_g, Some(0.0));
        assert_eq!(result.reagent_balance_residual_g, Some(0.0));
        assert_eq!(result.synthetic_grid_energy_wh_per_kg_o2, Some(350_000.0));
    }

    #[test]
    fn unknown_reagent_inventory_blocks_closed_loop_claim() {
        let report = audit_manifest_json(FIXTURE).expect("manifest should audit");
        assert_eq!(
            result_for(&report, "blocked-unknown-reagent-inventory").disposition,
            "BLOCKED"
        );
    }

    #[test]
    fn qualitative_oxygen_signal_cannot_become_product_mass() {
        let report = audit_manifest_json(FIXTURE).expect("manifest should audit");
        assert_eq!(
            result_for(&report, "blocked-qualitative-oxygen-observation-only").disposition,
            "BLOCKED"
        );
    }

    #[test]
    fn co2_observation_cannot_be_promoted_to_o2() {
        let report = audit_manifest_json(FIXTURE).expect("manifest should audit");
        assert_eq!(
            result_for(&report, "rejected-co2-promoted-to-o2").disposition,
            "REJECTED"
        );
    }

    #[test]
    fn incomplete_grid_boundary_blocks_energy_intensity() {
        let report = audit_manifest_json(FIXTURE).expect("manifest should audit");
        let result = result_for(&report, "blocked-incomplete-grid-boundary");
        assert_eq!(result.disposition, "BLOCKED");
        assert_eq!(result.synthetic_grid_energy_wh_per_kg_o2, None);
        assert!(
            result
                .reasons
                .iter()
                .any(|reason| reason.contains("required subsystem role"))
        );
    }

    #[test]
    fn unqualified_coproduct_cannot_be_credited() {
        let report = audit_manifest_json(FIXTURE).expect("manifest should audit");
        assert_eq!(
            result_for(&report, "blocked-unqualified-coproduct-credit").disposition,
            "BLOCKED"
        );
    }

    #[test]
    fn mismatched_product_contract_is_not_comparable() {
        let report = audit_manifest_json(FIXTURE).expect("manifest should audit");
        assert_eq!(
            result_for(&report, "not-comparable-product-contract").disposition,
            "NOT_COMPARABLE"
        );
    }

    #[test]
    fn balanced_ledger_cannot_override_declared_feedstock_batch_mass() {
        let report = audit_manifest_json(FIXTURE).expect("fixture should audit");
        assert_eq!(
            result_for(&report, "rejected-feedstock-input-mismatch").disposition,
            "REJECTED"
        );
    }

    #[test]
    fn mass_nonclosure_is_rejected() {
        let report = audit_manifest_json(FIXTURE).expect("manifest should audit");
        assert_eq!(
            result_for(&report, "rejected-feedstock-mass-nonclosure").disposition,
            "REJECTED"
        );
    }

    #[test]
    fn oes_reference_cannot_be_used_as_quantitative_product_measurement() {
        let mut value = fixture_value();
        value["scenarios"][0]["product"]["quantitative_measurement_ref"] =
            json!("synthetic-oes-o2");
        let report = audit_manifest_json(&value.to_string()).expect("mutated manifest parses");
        let result = result_for(&report, "synthetic-balanced-accounting-positive");
        assert_eq!(result.disposition, "BLOCKED");
        assert_eq!(result.synthetic_grid_energy_wh_per_kg_o2, None);
    }

    #[test]
    fn duplicate_meter_identity_is_rejected() {
        let mut value = fixture_value();
        let first_meter_id = value["scenarios"][0]["grid_energy"]["meters"][0]["meter_id"].clone();
        value["scenarios"][0]["grid_energy"]["meters"][1]["meter_id"] = first_meter_id;
        let report = audit_manifest_json(&value.to_string()).expect("mutated manifest parses");
        assert_eq!(
            result_for(&report, "synthetic-balanced-accounting-positive").disposition,
            "REJECTED"
        );
    }

    #[test]
    fn missing_required_meter_role_blocks_grid_intensity() {
        let mut value = fixture_value();
        value["scenarios"][0]["grid_energy"]["meters"]
            .as_array_mut()
            .unwrap()
            .pop();
        let report = audit_manifest_json(&value.to_string()).expect("mutated manifest parses");
        let result = result_for(&report, "synthetic-balanced-accounting-positive");
        assert_eq!(result.disposition, "BLOCKED");
        assert_eq!(result.synthetic_grid_energy_wh_per_kg_o2, None);
    }

    #[test]
    fn manifest_cannot_claim_physical_measurements() {
        let mut value = fixture_value();
        value["physical_measurements"] = json!(true);
        assert!(audit_manifest_json(&value.to_string()).is_err());
    }

    #[test]
    fn unknown_manifest_fields_are_rejected() {
        let mut value = fixture_value();
        value["unexpected_claim"] = json!("should fail closed");
        assert!(audit_manifest_json(&value.to_string()).is_err());
    }

    #[test]
    fn expectation_mutation_is_reported_not_silently_accepted() {
        let mut value = fixture_value();
        value["scenarios"][0]["expected_disposition"] = json!("BLOCKED");
        let report = audit_manifest_json(&value.to_string()).expect("manifest structure parses");
        assert_eq!(report.mismatched_expectations, 1);
    }

    #[test]
    fn expired_calibration_blocks_product_claim() {
        let report = audit_manifest_json(FIXTURE).expect("fixture should audit");
        assert_eq!(
            result_for(&report, "blocked-expired-calibration").disposition,
            "BLOCKED"
        );
    }

    #[test]
    fn uncertainty_must_resolve_the_declared_decision_margin() {
        let report = audit_manifest_json(FIXTURE).expect("fixture should audit");
        assert_eq!(
            result_for(&report, "blocked-uncertainty-exceeds-decision-margin").disposition,
            "BLOCKED"
        );
    }

    #[test]
    fn sampling_recovery_requires_an_independent_check() {
        let report = audit_manifest_json(FIXTURE).expect("fixture should audit");
        assert_eq!(
            result_for(&report, "blocked-unverified-sampling-recovery").disposition,
            "BLOCKED"
        );
    }

    #[test]
    fn calibration_species_scope_mismatch_is_rejected() {
        let report = audit_manifest_json(FIXTURE).expect("fixture should audit");
        assert_eq!(
            result_for(&report, "rejected-calibration-species-scope").disposition,
            "REJECTED"
        );
    }

    #[test]
    fn sampling_recovery_corrected_mass_must_match_product() {
        let report = audit_manifest_json(FIXTURE).expect("fixture should audit");
        assert_eq!(
            result_for(&report, "rejected-sampling-recovery-mass-mismatch").disposition,
            "REJECTED"
        );
    }

    #[test]
    fn missing_calibration_identity_blocks_product_claim() {
        let mut value = fixture_value();
        value["scenarios"][0]["observations"][1]["calibration_ref"] = serde_json::Value::Null;
        let report = audit_manifest_json(&value.to_string()).expect("mutated manifest parses");
        assert_eq!(
            result_for(&report, "synthetic-balanced-accounting-positive").disposition,
            "BLOCKED"
        );
    }

    #[test]
    fn raw_measurement_trace_is_required() {
        let mut value = fixture_value();
        value["scenarios"][0]["observations"][1]["raw_artifact_ref"] = serde_json::Value::Null;
        let report = audit_manifest_json(&value.to_string()).expect("mutated manifest parses");
        assert_eq!(
            result_for(&report, "synthetic-balanced-accounting-positive").disposition,
            "BLOCKED"
        );
    }

    #[test]
    fn invalid_calendar_date_in_calibration_receipt_is_blocked() {
        let report = audit_manifest_json(FIXTURE).expect("fixture should audit");
        assert_eq!(
            result_for(&report, "blocked-invalid-calendar-date").disposition,
            "BLOCKED"
        );
    }

    #[test]
    fn failed_cross_sensitivity_qualification_is_rejected() {
        let report = audit_manifest_json(FIXTURE).expect("fixture should audit");
        assert_eq!(
            result_for(&report, "rejected-cross-sensitivity-failure").disposition,
            "REJECTED"
        );
    }

    #[test]
    fn present_but_wrong_calibration_identity_is_rejected() {
        let mut value = fixture_value();
        value["scenarios"][0]["product"]["calibration_ref"] =
            serde_json::json!("different-calibration");
        let report = audit_manifest_json(&value.to_string()).expect("mutated manifest parses");
        assert_eq!(
            result_for(&report, "synthetic-balanced-accounting-positive").disposition,
            "REJECTED"
        );
    }

    #[test]
    fn scenario_ids_must_be_unique() {
        let mut value = fixture_value();
        let first_scenario_id = value["scenarios"][0]["id"].clone();
        value["scenarios"][1]["id"] = first_scenario_id;
        assert!(audit_manifest_json(&value.to_string()).is_err());
    }
}
