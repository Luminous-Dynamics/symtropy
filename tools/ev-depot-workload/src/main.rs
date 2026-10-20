use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, env, fs, process::ExitCode};

const DAY_SECONDS: u32 = 86_399;
const DAY_MINUTES: f64 = 1_440.0;
const CLAIM_CEILING: &str = "Assumption-driven conversion of audited published duty-cycle summaries into simulated overnight charging jobs. Duty timing and VMT originate in the declared source report; vehicle sampling, repeated daily cycle, battery pack, SOC, consumption, efficiency, acceptance, charger count and site power are model assumptions. Not metered charging data, controller proof, AC grid feasibility, battery validation, or South African fleet evidence.";

#[derive(Debug, Deserialize)]
struct AuditReport {
    schema_version: String,
    tool: String,
    fleet_id: String,
    dataset_id: String,
    source_uri: String,
    source_revision: String,
    evidence_class_declared_by_caller: String,
    kwh_per_mile_assumption: f64,
    operation_days_input_rows: usize,
    operation_days_parsed_valid_rows: usize,
    operation_days_invalid_rows: Vec<serde_json::Value>,
    schedule_input_rows: usize,
    schedule_parsed_valid_rows_with_known_operation_day_ids: usize,
    schedule_invalid_rows: Vec<serde_json::Value>,
    reconciled_operation_day_count: usize,
    operation_day_reconciliation_issues: Vec<serde_json::Value>,
    reconciled_operation_day_profiles: Vec<DutyProfile>,
    operation_day_csv_sha256: String,
    schedule_csv_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct DutyProfile {
    operation_day_id: u64,
    vmt_miles: f64,
    on_shift_seconds: u32,
    off_shift_seconds: u32,
    first_on_shift_start_second: Option<u32>,
    last_on_shift_end_second: Option<u32>,
}

#[derive(Debug, Clone)]
struct Options {
    input_report: String,
    output_corpus: String,
    output_manifest: String,
    fleet_size: usize,
    seed: u64,
    kwh_per_mile: f64,
    nominal_pack_capacity_kwh: f64,
    initial_soc_fraction: f64,
    state_of_health_fraction: f64,
    temperature_capacity_fraction: f64,
    temperature_acceptance_fraction: f64,
    charging_efficiency_fraction: f64,
    maximum_port_input_power_kw: f64,
    maximum_battery_acceptance_kw: f64,
    charger_port_count: usize,
    site_power_budget_kw: f64,
    port_release_policy: String,
}

#[derive(Debug, Clone, Serialize)]
struct CurveBand {
    soc_lower_fraction: f64,
    soc_upper_fraction: f64,
    max_battery_acceptance_kw: f64,
}

#[derive(Debug, Clone, Serialize)]
struct VehicleTemplate {
    template_id: String,
    label: String,
    evidence_class: String,
    evidence_reference: String,
    evidence_note: String,
    nominal_pack_capacity_kwh: f64,
    state_of_health_fraction: f64,
    temperature_capacity_fraction: f64,
    temperature_acceptance_fraction: f64,
    charging_efficiency_fraction: f64,
    maximum_port_input_power_kw: f64,
    acceptance_curve: Vec<CurveBand>,
}

#[derive(Debug, Clone, Serialize)]
struct Job {
    job_id: String,
    template_id: String,
    arrival_minute: f64,
    departure_minute: f64,
    initial_soc_fraction: f64,
    target_soc_fraction: f64,
}

#[derive(Debug, Clone, Serialize)]
struct Scenario {
    scenario_id: String,
    label: String,
    evidence_class: String,
    evidence_reference: String,
    evidence_note: String,
    charger_port_count: usize,
    site_power_budget_kw: f64,
    jobs: Vec<Job>,
    capacity_events: Vec<CapacityEvent>,
    port_release_policy: String,
}

#[derive(Debug, Clone, Serialize)]
struct CapacityEvent {
    start_minute: f64,
    end_minute: f64,
    site_power_budget_kw: f64,
    available_port_count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct Corpus {
    schema_version: String,
    suite_id: String,
    claim_ceiling: String,
    vehicle_templates: Vec<VehicleTemplate>,
    scenarios: Vec<Scenario>,
}

#[derive(Debug, Serialize)]
struct GeneratedJobEvidence {
    job_id: String,
    operation_day_id: u64,
    source_vmt_miles: f64,
    required_battery_energy_kwh: f64,
    charge_window_minutes: f64,
    arrival_minute: f64,
    departure_minute: f64,
    initial_soc_fraction: f64,
    target_soc_fraction: f64,
}

#[derive(Debug, Serialize)]
struct Manifest {
    schema_version: String,
    tool: String,
    source_dataset_id: String,
    source_fleet_id: String,
    source_uri: String,
    source_revision: String,
    source_evidence_class_declared_by_caller: String,
    source_operation_day_csv_sha256: String,
    source_schedule_csv_sha256: String,
    audit_report_sha256: String,
    source_profile_count: usize,
    eligible_profile_count: usize,
    excluded_profile_operation_day_ids: Vec<u64>,
    seed: u64,
    sampling_method: String,
    fleet_size: usize,
    simulated_jobs: Vec<GeneratedJobEvidence>,
    kwh_per_mile_assumption: f64,
    audit_report_kwh_per_mile_assumption: f64,
    nominal_pack_capacity_kwh: f64,
    initial_soc_fraction: f64,
    state_of_health_fraction: f64,
    temperature_capacity_fraction: f64,
    temperature_acceptance_fraction: f64,
    charging_efficiency_fraction: f64,
    maximum_port_input_power_kw: f64,
    maximum_battery_acceptance_kw: f64,
    charger_port_count: usize,
    site_power_budget_kw: f64,
    port_release_policy: String,
    scenario_evidence_class: String,
    assumption_notes: Vec<String>,
    claim_ceiling: String,
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn finite_positive(name: &str, value: f64) -> Result<(), String> {
    if !value.is_finite() || value <= 0.0 {
        Err(format!("{name}_must_be_finite_and_positive"))
    } else {
        Ok(())
    }
}

fn fraction(name: &str, value: f64, allow_zero: bool) -> Result<(), String> {
    let lower_ok = if allow_zero {
        value >= 0.0
    } else {
        value > 0.0
    };
    if !value.is_finite() || !lower_ok || value > 1.0 {
        Err(format!(
            "{name}_must_be_finite_and_in_{}1]",
            if allow_zero { "[0," } else { "(0," }
        ))
    } else {
        Ok(())
    }
}

fn parse_value<T: std::str::FromStr>(args: &[String], name: &str) -> Result<T, String> {
    let index = args
        .iter()
        .position(|arg| arg == name)
        .ok_or_else(|| format!("{name}_required"))?;
    let value = args
        .get(index + 1)
        .ok_or_else(|| format!("{name}_requires_a_value"))?;
    value
        .parse::<T>()
        .map_err(|_| format!("{name}_has_invalid_value"))
}

fn parse_args(args: &[String]) -> Result<Options, String> {
    if args.len() < 2 {
        return Err("usage: ev-depot-workload --input-report audit.json --output-corpus corpus.json --output-manifest manifest.json --fleet-size N --seed N --kwh-per-mile X --pack-capacity-kwh X --initial-soc-fraction X --state-of-health-fraction X --temperature-capacity-fraction X --temperature-acceptance-fraction X --charging-efficiency-fraction X --maximum-port-input-power-kw X --maximum-battery-acceptance-kw X --charger-ports N --site-power-budget-kw X [--port-release-policy release_on_charge_completion|hold_until_departure]".to_string());
    }
    let known = [
        "--input-report",
        "--output-corpus",
        "--output-manifest",
        "--fleet-size",
        "--seed",
        "--kwh-per-mile",
        "--pack-capacity-kwh",
        "--initial-soc-fraction",
        "--state-of-health-fraction",
        "--temperature-capacity-fraction",
        "--temperature-acceptance-fraction",
        "--charging-efficiency-fraction",
        "--maximum-port-input-power-kw",
        "--maximum-battery-acceptance-kw",
        "--charger-ports",
        "--site-power-budget-kw",
        "--port-release-policy",
    ];
    let required = [
        "--input-report",
        "--output-corpus",
        "--output-manifest",
        "--fleet-size",
        "--seed",
        "--kwh-per-mile",
        "--pack-capacity-kwh",
        "--initial-soc-fraction",
        "--state-of-health-fraction",
        "--temperature-capacity-fraction",
        "--temperature-acceptance-fraction",
        "--charging-efficiency-fraction",
        "--maximum-port-input-power-kw",
        "--maximum-battery-acceptance-kw",
        "--charger-ports",
        "--site-power-budget-kw",
    ];
    let mut seen = BTreeSet::new();
    let mut i = 1;
    while i < args.len() {
        let name = args[i].as_str();
        if !known.contains(&name) {
            return Err(format!("unknown_argument:{name}"));
        }
        if !seen.insert(name.to_string()) {
            return Err(format!("duplicate_argument:{name}"));
        }
        if i + 1 >= args.len() {
            return Err(format!("{name}_requires_a_value"));
        }
        i += 2;
    }
    for name in required {
        if !seen.contains(name) {
            return Err(format!("{name}_required"));
        }
    }

    let options = Options {
        input_report: parse_value(args, "--input-report")?,
        output_corpus: parse_value(args, "--output-corpus")?,
        output_manifest: parse_value(args, "--output-manifest")?,
        fleet_size: parse_value(args, "--fleet-size")?,
        seed: parse_value(args, "--seed")?,
        kwh_per_mile: parse_value(args, "--kwh-per-mile")?,
        nominal_pack_capacity_kwh: parse_value(args, "--pack-capacity-kwh")?,
        initial_soc_fraction: parse_value(args, "--initial-soc-fraction")?,
        state_of_health_fraction: parse_value(args, "--state-of-health-fraction")?,
        temperature_capacity_fraction: parse_value(args, "--temperature-capacity-fraction")?,
        temperature_acceptance_fraction: parse_value(args, "--temperature-acceptance-fraction")?,
        charging_efficiency_fraction: parse_value(args, "--charging-efficiency-fraction")?,
        maximum_port_input_power_kw: parse_value(args, "--maximum-port-input-power-kw")?,
        maximum_battery_acceptance_kw: parse_value(args, "--maximum-battery-acceptance-kw")?,
        charger_port_count: parse_value(args, "--charger-ports")?,
        site_power_budget_kw: parse_value(args, "--site-power-budget-kw")?,
        port_release_policy: args
            .iter()
            .position(|arg| arg == "--port-release-policy")
            .and_then(|index| args.get(index + 1).cloned())
            .unwrap_or_else(|| "release_on_charge_completion".to_string()),
    };
    if !["release_on_charge_completion", "hold_until_departure"]
        .contains(&options.port_release_policy.as_str())
    {
        return Err("--port-release-policy_must_be_release_on_charge_completion_or_hold_until_departure".to_string());
    }
    if options.fleet_size == 0 || options.charger_port_count == 0 {
        return Err("fleet_size_and_charger_ports_must_be_positive".to_string());
    }
    finite_positive("--kwh-per-mile", options.kwh_per_mile)?;
    finite_positive("--pack-capacity-kwh", options.nominal_pack_capacity_kwh)?;
    fraction("--initial-soc-fraction", options.initial_soc_fraction, true)?;
    if options.initial_soc_fraction >= 1.0 {
        return Err("--initial-soc-fraction_must_be_less_than_1".to_string());
    }
    fraction(
        "--state-of-health-fraction",
        options.state_of_health_fraction,
        false,
    )?;
    fraction(
        "--temperature-capacity-fraction",
        options.temperature_capacity_fraction,
        false,
    )?;
    fraction(
        "--temperature-acceptance-fraction",
        options.temperature_acceptance_fraction,
        false,
    )?;
    fraction(
        "--charging-efficiency-fraction",
        options.charging_efficiency_fraction,
        false,
    )?;
    finite_positive(
        "--maximum-port-input-power-kw",
        options.maximum_port_input_power_kw,
    )?;
    finite_positive(
        "--maximum-battery-acceptance-kw",
        options.maximum_battery_acceptance_kw,
    )?;
    finite_positive("--site-power-budget-kw", options.site_power_budget_kw)?;
    if options.output_corpus == options.output_manifest {
        return Err("output_corpus_and_output_manifest_must_be_different_paths".to_string());
    }
    Ok(options)
}

fn validate_audit_report(report: &AuditReport) -> Result<Vec<DutyProfile>, String> {
    if report.schema_version != "0.1.0" || report.tool != "ev-depot-duty-audit" {
        return Err("unsupported_audit_report_schema_or_tool".to_string());
    }
    if report.fleet_id.trim().is_empty()
        || report.dataset_id.trim().is_empty()
        || !report.source_uri.starts_with("https://")
        || report.source_revision.trim().is_empty()
        || report.evidence_class_declared_by_caller.trim().is_empty()
    {
        return Err("source_identity_and_evidence_metadata_required".to_string());
    }
    if !is_sha256(&report.operation_day_csv_sha256) || !is_sha256(&report.schedule_csv_sha256) {
        return Err("source_csv_digests_must_be_64_hex_characters".to_string());
    }
    finite_positive(
        "audit_kwh_per_mile_assumption",
        report.kwh_per_mile_assumption,
    )?;
    if report.operation_days_input_rows != report.operation_days_parsed_valid_rows
        || !report.operation_days_invalid_rows.is_empty()
        || report.schedule_input_rows
            != report.schedule_parsed_valid_rows_with_known_operation_day_ids
        || !report.schedule_invalid_rows.is_empty()
        || !report.operation_day_reconciliation_issues.is_empty()
        || report.reconciled_operation_day_count != report.reconciled_operation_day_profiles.len()
    {
        return Err("audit_report_has_invalid_rows_or_unreconciled_operation_days".to_string());
    }
    if report.reconciled_operation_day_profiles.is_empty() {
        return Err("audit_report_has_no_reconciled_operation_day_profiles".to_string());
    }

    let mut ids = BTreeSet::new();
    let mut eligible = Vec::new();
    for profile in &report.reconciled_operation_day_profiles {
        if profile.operation_day_id == 0 || !ids.insert(profile.operation_day_id) {
            return Err(
                "reconciled_profiles_contain_empty_or_duplicate_operation_day_id".to_string(),
            );
        }
        if !profile.vmt_miles.is_finite()
            || profile.vmt_miles < 0.0
            || profile
                .on_shift_seconds
                .saturating_add(profile.off_shift_seconds)
                != DAY_SECONDS
        {
            return Err(format!(
                "invalid_reconciled_profile:{}",
                profile.operation_day_id
            ));
        }
        match (
            profile.first_on_shift_start_second,
            profile.last_on_shift_end_second,
        ) {
            (None, None) if profile.on_shift_seconds == 0 && profile.vmt_miles <= 1.0e-9 => {}
            (Some(first), Some(last))
                if first < DAY_SECONDS
                    && last <= DAY_SECONDS
                    && last > first
                    && profile.on_shift_seconds > 0 =>
            {
                eligible.push(profile.clone());
            }
            _ => {
                return Err(format!(
                    "invalid_or_incomplete_shift_window:{}",
                    profile.operation_day_id
                ));
            }
        }
    }
    if eligible.is_empty() {
        return Err("no_reconciled_profiles_have_a_valid_overnight_shift_window".to_string());
    }
    Ok(eligible)
}

fn next_random(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1);
    *state
}

fn safe_id(value: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            dash = false;
        } else if !dash && !out.is_empty() {
            out.push('-');
            dash = true;
        }
    }
    let id = out.trim_matches('-').to_string();
    if id.is_empty() {
        "fleet".to_string()
    } else {
        id
    }
}

fn generate(
    report: &AuditReport,
    report_bytes: &[u8],
    eligible: &[DutyProfile],
    options: &Options,
) -> Result<(Corpus, Manifest), String> {
    let effective_capacity = options.nominal_pack_capacity_kwh
        * options.state_of_health_fraction
        * options.temperature_capacity_fraction;
    finite_positive("effective_pack_capacity_kwh", effective_capacity)?;
    let report_digest = sha256(report_bytes);
    let scenario_id = format!(
        "nlr-{}-fleet-{}-seed-{}",
        safe_id(&report.fleet_id),
        options.fleet_size,
        options.seed
    );
    let evidence_reference = format!(
        "{}@{}; audit_report_sha256={}",
        report.source_uri, report.source_revision, report_digest
    );
    let assumption_notes = vec![
        "Each sampled NLR operation-day row represents one hypothetical vehicle's repeating daily duty profile; rows are sampled with replacement using the recorded deterministic seed.".to_string(),
        "Daily battery recharge demand is source VMT multiplied by the explicit CLI kWh-per-mile assumption; the NLR operating-summary table does not measure EV energy consumption.".to_string(),
        "The only charging window is last on-shift segment end until first on-shift segment start on the following cycle. Intermediate off-shift gaps are not treated as depot-connected charge time.".to_string(),
        "Initial charge-window SOC is a declared assumption, not a measured end-of-route state. The target SOC is calculated so its delta represents the daily recharge demand.".to_string(),
        "Pack capacity, SOH, temperature derates, charging efficiency, charge acceptance, port rating, site import capacity and port count are declared model assumptions.".to_string(),
        format!("Post-charge connector occupancy policy is {}; release_on_charge_completion is an optimistic zero-turnover assumption, while hold_until_departure reserves the connector until the declared departure.", options.port_release_policy),
        "The repeating-daily-cycle assumption is a workload abstraction: the source records are individual operating-day samples and do not establish multi-day duty continuity or actual depot connection.".to_string(),
    ];
    let template_id = "nlr-duty-cycle-vehicle".to_string();
    let template = VehicleTemplate {
        template_id: template_id.clone(),
        label: "Assumption-driven heavy-duty EV template from NLR duty-cycle summaries".to_string(),
        evidence_class: "estimate".to_string(),
        evidence_reference: evidence_reference.clone(),
        evidence_note: format!(
            "Generated assumption-only vehicle template. Port input cap {:.3} kW, battery acceptance {:.3} kW, efficiency {:.4}; these are CLI assumptions, not a sourced vehicle charge curve. Source report SHA-256 {}.",
            options.maximum_port_input_power_kw,
            options.maximum_battery_acceptance_kw,
            options.charging_efficiency_fraction,
            report_digest
        ),
        nominal_pack_capacity_kwh: options.nominal_pack_capacity_kwh,
        state_of_health_fraction: options.state_of_health_fraction,
        temperature_capacity_fraction: options.temperature_capacity_fraction,
        temperature_acceptance_fraction: options.temperature_acceptance_fraction,
        charging_efficiency_fraction: options.charging_efficiency_fraction,
        maximum_port_input_power_kw: options.maximum_port_input_power_kw,
        acceptance_curve: vec![CurveBand {
            soc_lower_fraction: 0.0,
            soc_upper_fraction: 1.0,
            max_battery_acceptance_kw: options.maximum_battery_acceptance_kw,
        }],
    };

    let mut rng = options.seed;
    let mut jobs = Vec::with_capacity(options.fleet_size);
    let mut job_evidence = Vec::with_capacity(options.fleet_size);
    let mut sampled_ids = Vec::with_capacity(options.fleet_size);
    for index in 0..options.fleet_size {
        let selection = (next_random(&mut rng) % eligible.len() as u64) as usize;
        let profile = &eligible[selection];
        let daily_energy = profile.vmt_miles * options.kwh_per_mile;
        if !daily_energy.is_finite() || daily_energy < 0.0 {
            return Err(format!(
                "invalid_daily_energy_for_operation_day_{}",
                profile.operation_day_id
            ));
        }
        let energy_headroom = effective_capacity * (1.0 - options.initial_soc_fraction);
        if daily_energy > energy_headroom + 1.0e-9 {
            return Err(format!(
                "daily_energy_exceeds_soc_headroom:operation_day_id={},required_kwh={:.6},headroom_kwh={:.6}; increase pack capacity, lower initial SOC or revise declared consumption assumption",
                profile.operation_day_id, daily_energy, energy_headroom
            ));
        }
        let target_soc =
            (options.initial_soc_fraction + daily_energy / effective_capacity).min(1.0);
        let first_start = f64::from(
            profile
                .first_on_shift_start_second
                .expect("eligible profile"),
        ) / 60.0;
        let last_end =
            f64::from(profile.last_on_shift_end_second.expect("eligible profile")) / 60.0;
        let arrival = last_end;
        let mut departure = first_start + DAY_MINUTES;
        if departure <= arrival + 1.0e-9 {
            departure += DAY_MINUTES;
        }
        if !departure.is_finite() || !arrival.is_finite() || departure <= arrival {
            return Err(format!(
                "invalid_overnight_charge_window_for_operation_day_{}",
                profile.operation_day_id
            ));
        }
        let job_id = format!("vehicle-{:04}", index + 1);
        jobs.push(Job {
            job_id: job_id.clone(),
            template_id: template_id.clone(),
            arrival_minute: arrival,
            departure_minute: departure,
            initial_soc_fraction: options.initial_soc_fraction,
            target_soc_fraction: target_soc,
        });
        sampled_ids.push(profile.operation_day_id);
        job_evidence.push(GeneratedJobEvidence {
            job_id,
            operation_day_id: profile.operation_day_id,
            source_vmt_miles: profile.vmt_miles,
            required_battery_energy_kwh: daily_energy,
            charge_window_minutes: departure - arrival,
            arrival_minute: arrival,
            departure_minute: departure,
            initial_soc_fraction: options.initial_soc_fraction,
            target_soc_fraction: target_soc,
        });
    }
    if options.seed == 0 {
        // Seed zero is valid; the linear-congruential stream still evolves deterministically.
    }
    let scenario = Scenario {
        scenario_id,
        label: format!(
            "NLR-derived duty-cycle workload; fleet size {}; seed {}; assumption-driven overnight charging",
            options.fleet_size, options.seed
        ),
        evidence_class: "estimate".to_string(),
        evidence_reference: evidence_reference.clone(),
        evidence_note: format!(
            "Source duty-window boundaries and VMT are from caller-declared dataset {} fleet {} at revision {}. The scenario is an estimate because fleet resampling, daily repeat assumptions, kWh-per-mile, SOC, battery/thermal properties, efficiency, charging acceptance, site power and charger ports are explicit model assumptions. Input audit report SHA-256 {}.",
            report.dataset_id, report.fleet_id, report.source_revision, report_digest
        ),
        charger_port_count: options.charger_port_count,
        site_power_budget_kw: options.site_power_budget_kw,
        jobs,
        capacity_events: Vec::new(),
        port_release_policy: options.port_release_policy.clone(),
    };
    let corpus = Corpus {
        schema_version: "0.1.0".to_string(),
        suite_id: "EV-CHARGE-RESILIENCE-001".to_string(),
        claim_ceiling: CLAIM_CEILING.to_string(),
        vehicle_templates: vec![template],
        scenarios: vec![scenario],
    };
    let manifest = Manifest {
        schema_version: "0.1.0".to_string(),
        tool: "ev-depot-workload".to_string(),
        source_dataset_id: report.dataset_id.clone(),
        source_fleet_id: report.fleet_id.clone(),
        source_uri: report.source_uri.clone(),
        source_revision: report.source_revision.clone(),
        source_evidence_class_declared_by_caller: report.evidence_class_declared_by_caller.clone(),
        source_operation_day_csv_sha256: report.operation_day_csv_sha256.clone(),
        source_schedule_csv_sha256: report.schedule_csv_sha256.clone(),
        audit_report_sha256: report_digest,
        source_profile_count: report.reconciled_operation_day_profiles.len(),
        eligible_profile_count: eligible.len(),
        excluded_profile_operation_day_ids: report.reconciled_operation_day_profiles.iter()
            .filter(|p| p.first_on_shift_start_second.is_none() && p.last_on_shift_end_second.is_none())
            .map(|p| p.operation_day_id).collect(),
        seed: options.seed,
        sampling_method: "linear-congruential deterministic sampling with replacement; profile index = next_u64 modulo eligible profile count; not a cryptographic random source".to_string(),
        fleet_size: options.fleet_size,
        simulated_jobs: job_evidence,
        kwh_per_mile_assumption: options.kwh_per_mile,
        audit_report_kwh_per_mile_assumption: report.kwh_per_mile_assumption,
        nominal_pack_capacity_kwh: options.nominal_pack_capacity_kwh,
        initial_soc_fraction: options.initial_soc_fraction,
        state_of_health_fraction: options.state_of_health_fraction,
        temperature_capacity_fraction: options.temperature_capacity_fraction,
        temperature_acceptance_fraction: options.temperature_acceptance_fraction,
        charging_efficiency_fraction: options.charging_efficiency_fraction,
        maximum_port_input_power_kw: options.maximum_port_input_power_kw,
        maximum_battery_acceptance_kw: options.maximum_battery_acceptance_kw,
        charger_port_count: options.charger_port_count,
        site_power_budget_kw: options.site_power_budget_kw,
        port_release_policy: options.port_release_policy.clone(),
        scenario_evidence_class: "estimate".to_string(),
        assumption_notes,
        claim_ceiling: CLAIM_CEILING.to_string(),
    };
    Ok((corpus, manifest))
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    let options = parse_args(&args)?;
    let report_bytes = fs::read(&options.input_report)
        .map_err(|error| format!("cannot read audit report {}: {error}", options.input_report))?;
    let report: AuditReport = serde_json::from_slice(&report_bytes)
        .map_err(|error| format!("invalid audit report JSON: {error}"))?;
    let eligible = validate_audit_report(&report)?;
    let (corpus, manifest) = generate(&report, &report_bytes, &eligible, &options)?;
    let corpus_json = serde_json::to_string_pretty(&corpus)
        .map_err(|error| format!("cannot serialize simulator corpus: {error}"))?;
    let manifest_json = serde_json::to_string_pretty(&manifest)
        .map_err(|error| format!("cannot serialize workload manifest: {error}"))?;
    fs::write(&options.output_corpus, corpus_json)
        .map_err(|error| format!("cannot write corpus {}: {error}", options.output_corpus))?;
    fs::write(&options.output_manifest, manifest_json)
        .map_err(|error| format!("cannot write manifest {}: {error}", options.output_manifest))?;
    println!(
        "generated estimate-only corpus: {} jobs; source_report_sha256={}; corpus={}; manifest={}",
        manifest.fleet_size,
        manifest.audit_report_sha256,
        options.output_corpus,
        options.output_manifest
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("ev-depot-workload: {error}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report() -> AuditReport {
        AuditReport {
            schema_version: "0.1.0".into(),
            tool: "ev-depot-duty-audit".into(),
            fleet_id: "fleet1-beverage-delivery".into(),
            dataset_id: "nlr-hdev-depot-charging-2021".into(),
            source_uri: "https://github.com/NatLabRockies/hdev-depot-charging-2021".into(),
            source_revision: "61530df51f6cd7d64f014ebbf8714cef24c8ca70".into(),
            evidence_class_declared_by_caller: "published_operational_summary".into(),
            kwh_per_mile_assumption: 1.8,
            operation_days_input_rows: 2,
            operation_days_parsed_valid_rows: 2,
            operation_days_invalid_rows: vec![],
            schedule_input_rows: 3,
            schedule_parsed_valid_rows_with_known_operation_day_ids: 3,
            schedule_invalid_rows: vec![],
            reconciled_operation_day_count: 2,
            operation_day_reconciliation_issues: vec![],
            reconciled_operation_day_profiles: vec![
                DutyProfile {
                    operation_day_id: 1,
                    vmt_miles: 80.0,
                    on_shift_seconds: 32_400,
                    off_shift_seconds: 53_999,
                    first_on_shift_start_second: Some(8 * 3_600),
                    last_on_shift_end_second: Some(17 * 3_600),
                },
                DutyProfile {
                    operation_day_id: 2,
                    vmt_miles: 20.0,
                    on_shift_seconds: 28_800,
                    off_shift_seconds: 57_599,
                    first_on_shift_start_second: Some(4 * 3_600),
                    last_on_shift_end_second: Some(18 * 3_600),
                },
            ],
            operation_day_csv_sha256: "a".repeat(64),
            schedule_csv_sha256: "b".repeat(64),
        }
    }

    fn options() -> Options {
        Options {
            input_report: "report.json".into(),
            output_corpus: "corpus.json".into(),
            output_manifest: "manifest.json".into(),
            fleet_size: 2,
            seed: 17,
            kwh_per_mile: 1.8,
            nominal_pack_capacity_kwh: 400.0,
            initial_soc_fraction: 0.1,
            state_of_health_fraction: 0.9,
            temperature_capacity_fraction: 0.98,
            temperature_acceptance_fraction: 0.8,
            charging_efficiency_fraction: 0.92,
            maximum_port_input_power_kw: 150.0,
            maximum_battery_acceptance_kw: 140.0,
            charger_port_count: 2,
            site_power_budget_kw: 200.0,
            port_release_policy: "release_on_charge_completion".into(),
        }
    }

    #[test]
    fn generated_job_window_uses_last_shift_end_until_next_first_start() {
        let r = report();
        let eligible = validate_audit_report(&r).unwrap();
        let (corpus, manifest) =
            generate(&r, b"same-audit-report-bytes", &eligible, &options()).unwrap();
        let scenario = &corpus.scenarios[0];
        for job in &manifest.simulated_jobs {
            if job.operation_day_id == 1 {
                assert_eq!(job.arrival_minute, 17.0 * 60.0);
                assert_eq!(job.departure_minute, 8.0 * 60.0 + 1_440.0);
                assert_eq!(job.charge_window_minutes, 900.0);
            } else {
                assert_eq!(job.arrival_minute, 18.0 * 60.0);
                assert_eq!(job.departure_minute, 4.0 * 60.0 + 1_440.0);
                assert_eq!(job.charge_window_minutes, 600.0);
            }
        }
        assert_eq!(scenario.evidence_class, "estimate");
        assert_eq!(scenario.port_release_policy, "release_on_charge_completion");
        assert_eq!(manifest.port_release_policy, "release_on_charge_completion");
        assert_eq!(manifest.scenario_evidence_class, "estimate");
        assert_eq!(scenario.jobs.len(), 2);
    }

    #[test]
    fn same_seed_and_input_produce_identical_corpus_and_manifest() {
        let r = report();
        let eligible = validate_audit_report(&r).unwrap();
        let a = generate(&r, b"same-audit-report-bytes", &eligible, &options()).unwrap();
        let b = generate(&r, b"same-audit-report-bytes", &eligible, &options()).unwrap();
        assert_eq!(
            serde_json::to_string(&a.0).unwrap(),
            serde_json::to_string(&b.0).unwrap()
        );
        assert_eq!(
            serde_json::to_string(&a.1).unwrap(),
            serde_json::to_string(&b.1).unwrap()
        );
    }

    #[test]
    fn impossible_daily_energy_fails_closed() {
        let r = report();
        let eligible = validate_audit_report(&r).unwrap();
        let mut opts = options();
        opts.nominal_pack_capacity_kwh = 100.0;
        assert!(
            generate(&r, b"report", &eligible, &opts)
                .unwrap_err()
                .contains("daily_energy_exceeds_soc_headroom")
        );
    }

    #[test]
    fn unreconciled_audit_reports_are_rejected() {
        let mut r = report();
        r.operation_day_reconciliation_issues
            .push(serde_json::json!({"operation_day_id": 1}));
        assert!(
            validate_audit_report(&r)
                .unwrap_err()
                .contains("unreconciled")
        );
    }

    #[test]
    fn synthetic_or_published_sources_never_become_measured_scenarios() {
        let mut r = report();
        r.evidence_class_declared_by_caller = "synthetic".into();
        let eligible = validate_audit_report(&r).unwrap();
        let (corpus, manifest) = generate(&r, b"report", &eligible, &options()).unwrap();
        assert_eq!(corpus.scenarios[0].evidence_class, "estimate");
        assert_eq!(
            manifest.source_evidence_class_declared_by_caller,
            "synthetic"
        );
        assert!(corpus.scenarios[0].evidence_note.contains("is an estimate"));
    }

    #[test]
    fn accepts_excluded_zero_duty_profile_but_records_it() {
        let mut r = report();
        r.reconciled_operation_day_profiles.push(DutyProfile {
            operation_day_id: 3,
            vmt_miles: 0.0,
            on_shift_seconds: 0,
            off_shift_seconds: DAY_SECONDS,
            first_on_shift_start_second: None,
            last_on_shift_end_second: None,
        });
        r.reconciled_operation_day_count += 1;
        r.operation_days_input_rows += 1;
        r.operation_days_parsed_valid_rows += 1;
        let eligible = validate_audit_report(&r).unwrap();
        let (_, manifest) = generate(&r, b"report", &eligible, &options()).unwrap();
        assert_eq!(manifest.excluded_profile_operation_day_ids, vec![3]);
        assert_eq!(manifest.source_profile_count, 3);
        assert_eq!(manifest.eligible_profile_count, 2);
    }
    #[test]
    fn hold_until_departure_policy_is_recorded_in_corpus_and_manifest() {
        let r = report();
        let eligible = validate_audit_report(&r).unwrap();
        let mut opts = options();
        opts.port_release_policy = "hold_until_departure".into();
        let (corpus, manifest) =
            generate(&r, b"same-audit-report-bytes", &eligible, &opts).unwrap();
        assert_eq!(corpus.scenarios[0].port_release_policy, "hold_until_departure");
        assert_eq!(manifest.port_release_policy, "hold_until_departure");
        assert!(manifest.assumption_notes.iter().any(|note| note.contains("hold_until_departure")));
    }

}
