#![forbid(unsafe_code)]

//! Synthetic coupling: cargo eligibility -> generator fuel issue -> hourly
//! fuel-service model -> non-compensatory service-vector qualification.

use safrb_cargo_receipt_ledger::{
    synthetic_cases, validate_and_project, CargoCase, Projection, HORIZON_HOURS,
};
use safrb_fuel_service_corpus::{
    generate_case, Archetype, FuelServiceCase, ServiceDisposition, TelemetryState,
};
use safrb_usable_service_vector::{
    qualify_service, EvidenceAdmission, EvidenceInterval, PredicateEvidence, PredicateKind,
    PredicateMode, PredicateSpec, QualificationContext, QualificationReport, ServiceProfile,
    ServiceVerdict,
};

pub const SCHEMA_VERSION: &str = "0.1.0";
pub const COUPLING_PROFILE_ID: &str = "safrb-fuel-service-hourly-v0.1";
pub const SERVICE_BOUNDARY_ID: &str = "synthetic-generator-load-boundary";

#[derive(Clone, Debug)]
pub struct CoupledHourlyService {
    pub hour_index: u32,
    pub interval_start_unix_s: u64,
    pub generator_stock_received_grams: u64,
    pub generator_stock_available_grams: u64,
    pub fuel_issue_grams: u64,
    pub compatible_fuel_feed_grams: u64,
    pub unconstrained_fuel_energy_wh: u64,
    pub generator_power_cap_wh: u64,
    pub power_cap_curtailed_wh: u64,
    pub gross_generation_wh: u64,
    pub network_deliverable_wh: u64,
    pub demand_wh: u64,
    pub critical_floor_wh: u64,
    pub delivered_lower_wh: u64,
    pub delivered_upper_wh: u64,
    pub average_active_power_lower_w: u64,
    pub average_active_power_upper_w: u64,
    pub telemetry_state: TelemetryState,
    pub service_disposition: ServiceDisposition,
    pub qualification: QualificationReport,
}

#[derive(Clone, Debug)]
pub struct CoupledCase {
    pub case_id: String,
    pub cargo_case: CargoCase,
    pub cargo_projection: Projection,
    pub generator_template: FuelServiceCase,
    pub hourly_trajectory: Vec<CoupledHourlyService>,
    pub verdict: ServiceVerdict,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CouplingReport {
    pub case_count: usize,
    pub hourly_interval_count: usize,
    pub case_guaranteed_count: usize,
    pub case_failed_count: usize,
    pub case_indeterminate_count: usize,
    pub hourly_guaranteed_count: usize,
    pub hourly_failed_count: usize,
    pub hourly_indeterminate_count: usize,
    pub fuel_issue_grams: u64,
    pub compatible_fuel_feed_grams: u64,
    pub gross_generation_wh: u64,
    pub network_deliverable_wh: u64,
    pub delivered_lower_wh: u64,
    pub power_cap_curtailed_wh: u64,
    pub required_predicate_results: usize,
    pub not_applicable_predicates_per_hour: usize,
}

fn add(total: &mut u64, value: u64, name: &str) -> Result<(), String> {
    *total = total.checked_add(value).ok_or_else(|| format!("{name} overflow"))?;
    Ok(())
}

fn fixture_digest(seed: u64) -> String {
    // Digest-shaped deterministic identity only: not a cryptographic digest.
    format!("{seed:064x}")
}

fn disposition(
    telemetry: TelemetryState,
    lower_wh: u64,
    upper_wh: u64,
    floor_wh: u64,
) -> ServiceDisposition {
    if telemetry == TelemetryState::StaleOrConflicting {
        ServiceDisposition::Indeterminate
    } else if lower_wh >= floor_wh {
        ServiceDisposition::Guaranteed
    } else if upper_wh < floor_wh {
        ServiceDisposition::ConfirmedShortfall
    } else {
        ServiceDisposition::Indeterminate
    }
}

fn to_i64(value: u64, field: &str) -> Result<i64, String> {
    i64::try_from(value).map_err(|_| format!("{field} exceeds signed service-vector range"))
}

fn average_power_w(energy_wh: u64, duration_seconds: u64) -> Result<u64, String> {
    if duration_seconds == 0 {
        return Err("service interval duration must be positive".into());
    }
    energy_wh.checked_mul(3_600)
        .map(|watt_seconds| watt_seconds / duration_seconds)
        .ok_or_else(|| "average active-power conversion overflow".to_string())
}

fn profile(service_id: &str, floor_wh: u64, demand_wh: u64) -> Result<ServiceProfile, String> {
    let min_wh = to_i64(floor_wh, "critical-floor Wh")?;
    let max_wh = to_i64(demand_wh, "demand Wh")?;
    if min_wh > max_wh {
        return Err("critical floor exceeds hourly demand".into());
    }
    let required = |predicate_id: &str, kind: PredicateKind, unit: &str| PredicateSpec {
        predicate_id: predicate_id.into(),
        kind,
        mode: PredicateMode::Required,
        unit_code: unit.into(),
        allowed_min: min_wh,
        allowed_max: max_wh,
        not_applicable_reason: None,
    };
    let not_applicable = |predicate_id: &str, kind: PredicateKind, unit: &str, reason: &str| PredicateSpec {
        predicate_id: predicate_id.into(),
        kind,
        mode: PredicateMode::NotApplicable,
        unit_code: unit.into(),
        allowed_min: 0,
        allowed_max: 0,
        not_applicable_reason: Some(reason.into()),
    };
    Ok(ServiceProfile {
        profile_id: COUPLING_PROFILE_ID.into(),
        profile_version: "0.1.0",
        service_class: "critical-fuel-backed-electricity".into(),
        service_id: service_id.into(),
        boundary_id: SERVICE_BOUNDARY_ID.into(),
        predicates: vec![
            required("hourly-energy-floor", PredicateKind::Energy, "Wh"),
            required("hourly-average-active-power-floor", PredicateKind::ActivePower, "W"),
            not_applicable(
                "voltage-envelope", PredicateKind::Voltage, "mPu",
                "No independent voltage measurement is generated by the fuel-service model",
            ),
            not_applicable(
                "frequency-envelope", PredicateKind::Frequency, "mHz",
                "No independent frequency measurement is generated by the fuel-service model",
            ),
            not_applicable(
                "reactive-power-envelope", PredicateKind::ReactivePower, "var",
                "The fuel-service model does not solve or measure reactive-power flow",
            ),
            not_applicable(
                "waveform-quality", PredicateKind::WaveformQuality, "ppm",
                "The fuel-service model has no waveform or harmonic measurements",
            ),
            not_applicable(
                "beneficiary-process-output", PredicateKind::ProcessOutput, "process_unit",
                "Downstream beneficiary process output is outside this generator-boundary coupling",
            ),
        ],
    })
}

fn evidence(
    predicate_id: &str,
    unit: &str,
    interval: EvidenceInterval,
    context: &QualificationContext,
    ordinal: u64,
    admission: EvidenceAdmission,
) -> PredicateEvidence {
    PredicateEvidence {
        predicate_id: predicate_id.into(),
        service_id: context.service_id.clone(),
        boundary_id: context.boundary_id.clone(),
        run_id: context.run_id.clone(),
        case_sha256: context.case_sha256.clone(),
        window_start_unix_s: context.window_start_unix_s,
        window_end_unix_s: context.window_end_unix_s,
        measurement_start_unix_s: context.window_start_unix_s,
        measurement_end_unix_s: context.window_end_unix_s,
        recorded_at_unix_s: context.window_end_unix_s,
        fresh_until_unix_s: context.window_end_unix_s,
        unit_code: unit.into(),
        interval: Some(interval),
        admission,
        source_ref: Some(format!("synthetic-oracle://fuel-service/{ordinal}")),
        source_sha256: Some(fixture_digest(ordinal + 1)),
        provider_id: Some("safrb-coupling-synthetic-fixture".into()),
        qualification_ref: Some("synthetic-independent-reference-oracle-v0.1".into()),
        verifier_receipt_ref: Some(format!("synthetic-verifier-receipt-{ordinal}")),
        verifier_receipt_sha256: Some(fixture_digest(ordinal + 2)),
    }
}

fn qualify_hour(
    case_id: &str,
    case_index: usize,
    template: &FuelServiceCase,
    row_index: usize,
    lower_wh: u64,
    upper_wh: u64,
    average_power_lower_w: u64,
    average_power_upper_w: u64,
) -> Result<QualificationReport, String> {
    let row = template.hourly_trajectory.get(row_index)
        .ok_or_else(|| format!("missing generator model hour {row_index}"))?;
    let service_id = format!("synthetic-service:{case_id}");
    let window_start = row.interval_start_unix_s;
    let window_end = window_start.checked_add(3_600).ok_or("service window overflow")?;
    let context = QualificationContext {
        service_id: service_id.clone(),
        boundary_id: SERVICE_BOUNDARY_ID.into(),
        run_id: format!("safrb-coupling-run-{case_index:03}"),
        case_sha256: fixture_digest(case_index as u64 + 1),
        window_start_unix_s: window_start,
        window_end_unix_s: window_end,
    };
    let service_profile = profile(&service_id, row.critical_floor_wh, row.demand_wh)?;
    let energy_interval = EvidenceInterval {
        lower: to_i64(lower_wh, "delivered lower Wh")?,
        upper: to_i64(upper_wh, "delivered upper Wh")?,
    };
    let power_interval = EvidenceInterval {
        lower: to_i64(average_power_lower_w, "average active-power lower W")?,
        upper: to_i64(average_power_upper_w, "average active-power upper W")?,
    };
    let admission = if template.service.telemetry_state == TelemetryState::StaleOrConflicting {
        EvidenceAdmission::Stale
    } else {
        // Synthetic fixture oracle only. This does not represent cryptographic
        // evidence admission, physical telemetry or field verification.
        EvidenceAdmission::AcceptedByUpstreamVerifier
    };
    let ordinal = (case_index as u64 * u64::from(HORIZON_HOURS) + row_index as u64) * 2;
    let evidence = vec![
        evidence("hourly-energy-floor", "Wh", energy_interval, &context, ordinal + 1, admission),
        evidence("hourly-average-active-power-floor", "W", power_interval, &context, ordinal + 2, admission),
    ];
    qualify_service(&service_profile, &context, &evidence)
        .map_err(|errors| errors.join("; "))
}

fn issue_by_hour(case: &CargoCase) -> Result<Vec<u64>, String> {
    let mut values = vec![0_u64; HORIZON_HOURS as usize];
    for issue in &case.fuel_issues {
        if issue.issued_at_hour < HORIZON_HOURS {
            add(&mut values[issue.issued_at_hour as usize], issue.issued_quantity_grams, "hourly fuel issue")?;
        }
    }
    Ok(values)
}

pub fn couple_case(
    cargo_case: CargoCase,
    generator_template: FuelServiceCase,
    case_index: usize,
) -> Result<CoupledCase, String> {
    if generator_template.horizon_hours != HORIZON_HOURS {
        return Err("cargo and generator service horizons differ".into());
    }
    let projection = validate_and_project(&cargo_case)?;
    if projection.hourly.len() != HORIZON_HOURS as usize
        || generator_template.hourly_trajectory.len() != HORIZON_HOURS as usize
    {
        return Err("coupled models must provide exactly 72 hourly rows".into());
    }
    let fuel_issues = issue_by_hour(&cargo_case)?;
    let generator = &generator_template.generator;
    let power_cap = generator.rated_power_capacity_kw
        .checked_mul(1_000)
        .and_then(|value| value.checked_mul(generator.derate_basis_points))
        .ok_or("generator power cap overflow")? / 10_000;
    let mut hourly_trajectory = Vec::with_capacity(HORIZON_HOURS as usize);

    for hour in 0..HORIZON_HOURS as usize {
        let cargo_hour = &projection.hourly[hour];
        let service_hour = &generator_template.hourly_trajectory[hour];
        if cargo_hour.hour_index as usize != hour || service_hour.hour_index as usize != hour {
            return Err(format!("hourly alignment mismatch at index {hour}"));
        }
        let issue = fuel_issues[hour];
        let compatible_input = if generator.fuel_grade_compatible { issue } else { 0 };
        let unconstrained = compatible_input
            .checked_mul(generator.synthetic_lhv_milli_wh_per_gram)
            .and_then(|value| value.checked_mul(generator.efficiency_basis_points))
            .ok_or("fuel-to-electricity calculation overflow")? / 10_000 / 1_000;
        let gross = unconstrained.min(power_cap);
        let curtailed = unconstrained - gross;
        let network = gross.checked_mul(generator.network_delivery_basis_points)
            .ok_or("network delivery calculation overflow")? / 10_000;
        let uncertainty = generator_template.service.uncertainty_basis_points;
        if uncertainty > 10_000 {
            return Err("service uncertainty exceeds 100 percent".into());
        }
        let lower_wh = service_hour.demand_wh.min(
            network.checked_mul(10_000 - uncertainty)
                .ok_or("lower delivery interval overflow")? / 10_000
        );
        let upper_wh = service_hour.demand_wh.min(
            network.checked_mul(10_000 + uncertainty)
                .ok_or("upper delivery interval overflow")? / 10_000
        );
        let service_disposition = disposition(
            generator_template.service.telemetry_state,
            lower_wh,
            upper_wh,
            service_hour.critical_floor_wh,
        );
        let interval_start = service_hour.interval_start_unix_s;
        let interval_end = interval_start.checked_add(3_600).ok_or("service window overflow")?;
        let interval_duration = interval_end - interval_start;
        let average_power_lower_w = average_power_w(lower_wh, interval_duration)?;
        let average_power_upper_w = average_power_w(upper_wh, interval_duration)?;
        let qualification = qualify_hour(
            &cargo_case.case_id,
            case_index,
            &generator_template,
            hour,
            lower_wh,
            upper_wh,
            average_power_lower_w,
            average_power_upper_w,
        )?;
        let vector_disposition = qualification.service_verdict;
        if (vector_disposition == ServiceVerdict::Guaranteed && service_disposition != ServiceDisposition::Guaranteed)
            || (vector_disposition == ServiceVerdict::Failed && service_disposition != ServiceDisposition::ConfirmedShortfall)
            || (vector_disposition == ServiceVerdict::Indeterminate && service_disposition != ServiceDisposition::Indeterminate)
        {
            return Err(format!("service-vector and fuel-service dispositions disagree at hour {hour}"));
        }
        hourly_trajectory.push(CoupledHourlyService {
            hour_index: hour as u32,
            interval_start_unix_s: service_hour.interval_start_unix_s,
            generator_stock_received_grams: cargo_hour.delivered_to_generator_grams,
            generator_stock_available_grams: cargo_hour.generator_available_grams,
            fuel_issue_grams: issue,
            compatible_fuel_feed_grams: compatible_input,
            unconstrained_fuel_energy_wh: unconstrained,
            generator_power_cap_wh: power_cap,
            power_cap_curtailed_wh: curtailed,
            gross_generation_wh: gross,
            network_deliverable_wh: network,
            demand_wh: service_hour.demand_wh,
            critical_floor_wh: service_hour.critical_floor_wh,
            delivered_lower_wh: lower_wh,
            delivered_upper_wh: upper_wh,
            average_active_power_lower_w: average_power_lower_w,
            average_active_power_upper_w: average_power_upper_w,
            telemetry_state: generator_template.service.telemetry_state,
            service_disposition,
            qualification,
        });
    }

    let verdict = if hourly_trajectory.iter().any(|row| row.qualification.service_verdict == ServiceVerdict::Failed) {
        ServiceVerdict::Failed
    } else if hourly_trajectory.iter().any(|row| row.qualification.service_verdict == ServiceVerdict::Indeterminate) {
        ServiceVerdict::Indeterminate
    } else {
        ServiceVerdict::Guaranteed
    };
    Ok(CoupledCase {
        case_id: cargo_case.case_id.clone(),
        cargo_case,
        cargo_projection: projection,
        generator_template,
        hourly_trajectory,
        verdict,
    })
}

pub fn build_corpus() -> Result<Vec<CoupledCase>, String> {
    synthetic_cases().into_iter().enumerate().map(|(index, cargo)| {
        let seed = if cargo.case_id == "verified-on-time" { 9 } else { (index % 9) as u64 };
        let template = generate_case(Archetype::BaselineAllPaths, seed);
        couple_case(cargo, template, index)
    }).collect()
}

pub fn evaluate_coupling(cases: &[CoupledCase]) -> Result<CouplingReport, String> {
    let mut report = CouplingReport {
        case_count: cases.len(),
        hourly_interval_count: 0,
        case_guaranteed_count: 0,
        case_failed_count: 0,
        case_indeterminate_count: 0,
        hourly_guaranteed_count: 0,
        hourly_failed_count: 0,
        hourly_indeterminate_count: 0,
        fuel_issue_grams: 0,
        compatible_fuel_feed_grams: 0,
        gross_generation_wh: 0,
        network_deliverable_wh: 0,
        delivered_lower_wh: 0,
        power_cap_curtailed_wh: 0,
        required_predicate_results: 0,
        not_applicable_predicates_per_hour: 0,
    };
    for case in cases {
        if case.hourly_trajectory.len() != HORIZON_HOURS as usize {
            return Err(format!("{}: expected 72 hourly rows", case.case_id));
        }
        report.hourly_interval_count += case.hourly_trajectory.len();
        match case.verdict {
            ServiceVerdict::Guaranteed => report.case_guaranteed_count += 1,
            ServiceVerdict::Failed => report.case_failed_count += 1,
            ServiceVerdict::Indeterminate => report.case_indeterminate_count += 1,
        }
        for row in &case.hourly_trajectory {
            match row.qualification.service_verdict {
                ServiceVerdict::Guaranteed => report.hourly_guaranteed_count += 1,
                ServiceVerdict::Failed => report.hourly_failed_count += 1,
                ServiceVerdict::Indeterminate => report.hourly_indeterminate_count += 1,
            }
            add(&mut report.fuel_issue_grams, row.fuel_issue_grams, "fuel issue total")?;
            add(&mut report.compatible_fuel_feed_grams, row.compatible_fuel_feed_grams, "compatible fuel feed total")?;
            add(&mut report.gross_generation_wh, row.gross_generation_wh, "gross generation total")?;
            add(&mut report.network_deliverable_wh, row.network_deliverable_wh, "network delivery total")?;
            add(&mut report.delivered_lower_wh, row.delivered_lower_wh, "delivered lower energy total")?;
            add(&mut report.power_cap_curtailed_wh, row.power_cap_curtailed_wh, "power-cap curtailed energy total")?;
            report.required_predicate_results += row.qualification.required_guaranteed_count
                + row.qualification.required_failed_count
                + row.qualification.required_indeterminate_count;
            report.not_applicable_predicates_per_hour = row.qualification.not_applicable_count;
        }
    }
    if report.hourly_interval_count != report.case_count * HORIZON_HOURS as usize {
        return Err("coupling corpus row count is inconsistent".into());
    }
    Ok(report)
}

fn q(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '\"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push(' '),
            c => out.push(c),
        }
    }
    out.push('\"');
    out
}

impl CoupledCase {
    /// Planner output contains manifest facts, model capability and requirements only.
    pub fn planner_json(&self) -> String {
        let requirements = self.generator_template.hourly_trajectory.iter().map(|row| format!(
            "{{\"hourIndex\":{},\"intervalStartUnixS\":{},\"demandWh\":{},\"criticalFloorWh\":{},\"telemetryState\":{}}}",
            row.hour_index, row.interval_start_unix_s, row.demand_wh, row.critical_floor_wh,
            q(self.generator_template.service.telemetry_state.as_str())
        )).collect::<Vec<_>>().join(",");
        format!(
            "{{\"schemaVersion\":\"0.1.0\",\"recordKind\":\"planner_observation\",\"caseId\":{},\"evidenceMaturity\":\"synthetic_fixture\",\"cargoManifest\":{},\"generatorCapability\":{{\"fuelGradeCompatible\":{},\"syntheticLhvMilliWhPerGram\":{},\"efficiencyBasisPoints\":{},\"ratedPowerCapacityKw\":{},\"derateBasisPoints\":{},\"networkDeliveryBasisPoints\":{},\"uncertaintyBasisPoints\":{}}},\"hourlyRequirements\":[{}]}}",
            q(&self.case_id), self.cargo_case.planner_json(),
            self.generator_template.generator.fuel_grade_compatible,
            self.generator_template.generator.synthetic_lhv_milli_wh_per_gram,
            self.generator_template.generator.efficiency_basis_points,
            self.generator_template.generator.rated_power_capacity_kw,
            self.generator_template.generator.derate_basis_points,
            self.generator_template.generator.network_delivery_basis_points,
            self.generator_template.service.uncertainty_basis_points,
            requirements
        )
    }

    /// Evaluator-only output nests cargo truth and all downstream service verdicts.
    pub fn oracle_json(&self) -> String {
        let rows = self.hourly_trajectory.iter().map(|row| format!(
            "{{\"hourIndex\":{},\"intervalStartUnixS\":{},\"generatorStockReceivedGrams\":{},\"generatorStockAvailableGrams\":{},\"fuelIssueGrams\":{},\"compatibleFuelFeedGrams\":{},\"unconstrainedFuelEnergyWh\":{},\"generatorPowerCapWh\":{},\"powerCapCurtailedWh\":{},\"grossGenerationWh\":{},\"networkDeliverableWh\":{},\"demandWh\":{},\"criticalFloorWh\":{},\"deliveredLowerWh\":{},\"deliveredUpperWh\":{},\"averageActivePowerLowerW\":{},\"averageActivePowerUpperW\":{},\"telemetryState\":{},\"serviceDisposition\":{},\"serviceVectorVerdict\":{},\"requiredGuaranteed\":{},\"requiredFailed\":{},\"requiredIndeterminate\":{},\"notApplicableCount\":{}}}",
            row.hour_index, row.interval_start_unix_s, row.generator_stock_received_grams,
            row.generator_stock_available_grams, row.fuel_issue_grams, row.compatible_fuel_feed_grams,
            row.unconstrained_fuel_energy_wh, row.generator_power_cap_wh, row.power_cap_curtailed_wh,
            row.gross_generation_wh, row.network_deliverable_wh, row.demand_wh, row.critical_floor_wh,
            row.delivered_lower_wh, row.delivered_upper_wh,
            row.average_active_power_lower_w, row.average_active_power_upper_w,
            q(row.telemetry_state.as_str()),
            q(row.service_disposition.as_str()), q(row.qualification.service_verdict.as_str()),
            row.qualification.required_guaranteed_count, row.qualification.required_failed_count,
            row.qualification.required_indeterminate_count, row.qualification.not_applicable_count
        )).collect::<Vec<_>>().join(",");
        format!(
            "{{\"schemaVersion\":\"0.1.0\",\"recordKind\":\"oracle_truth\",\"caseId\":{},\"evidenceMaturity\":\"synthetic_fixture\",\"generatorCapability\":{{\"fuelGradeCompatible\":{},\"syntheticLhvMilliWhPerGram\":{},\"efficiencyBasisPoints\":{},\"ratedPowerCapacityKw\":{},\"derateBasisPoints\":{},\"networkDeliveryBasisPoints\":{},\"uncertaintyBasisPoints\":{}}},\"cargoLedger\":{},\"caseServiceVerdict\":{},\"hourlyServiceTrajectory\":[{}]}}",
            q(&self.case_id), self.generator_template.generator.fuel_grade_compatible,
            self.generator_template.generator.synthetic_lhv_milli_wh_per_gram,
            self.generator_template.generator.efficiency_basis_points,
            self.generator_template.generator.rated_power_capacity_kw,
            self.generator_template.generator.derate_basis_points,
            self.generator_template.generator.network_delivery_basis_points,
            self.generator_template.service.uncertainty_basis_points,
            self.cargo_case.oracle_json(&self.cargo_projection),
            q(self.verdict.as_str()), rows
        )
    }
}

pub fn report_json(cases: &[CoupledCase]) -> Result<String, String> {
    let report = evaluate_coupling(cases)?;
    Ok(format!(
        "{{\"reportVersion\":\"0.1.0\",\"evidenceMaturity\":\"synthetic_fixture\",\"caseCount\":{},\"hourlyIntervalCount\":{},\"caseGuaranteedCount\":{},\"caseFailedCount\":{},\"caseIndeterminateCount\":{},\"hourlyGuaranteedCount\":{},\"hourlyFailedCount\":{},\"hourlyIndeterminateCount\":{},\"fuelIssueGrams\":{},\"compatibleFuelFeedGrams\":{},\"grossGenerationWh\":{},\"networkDeliverableWh\":{},\"deliveredLowerWh\":{},\"powerCapCurtailedWh\":{},\"requiredPredicateResults\":{},\"notApplicablePredicatesPerHour\":{},\"cargoLedgerAndServiceOutputsSeparate\":true}}",
        report.case_count, report.hourly_interval_count, report.case_guaranteed_count,
        report.case_failed_count, report.case_indeterminate_count, report.hourly_guaranteed_count,
        report.hourly_failed_count, report.hourly_indeterminate_count, report.fuel_issue_grams,
        report.compatible_fuel_feed_grams, report.gross_generation_wh, report.network_deliverable_wh,
        report.delivered_lower_wh, report.power_cap_curtailed_wh, report.required_predicate_results,
        report.not_applicable_predicates_per_hour
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn case(id: &str) -> CargoCase {
        synthetic_cases().into_iter().find(|row| row.case_id == id).expect("fixture")
    }

    fn template(seed: u64) -> FuelServiceCase {
        generate_case(Archetype::BaselineAllPaths, seed)
    }

    #[test]
    fn all_cargo_cases_couple_to_72_hour_service_and_qualifier_outputs() {
        let cases = build_corpus().expect("coupling corpus");
        assert_eq!(cases.len(), 12);
        assert!(cases.iter().all(|case| case.hourly_trajectory.len() == 72));
        let report = evaluate_coupling(&cases).expect("report");
        assert_eq!(report.hourly_interval_count, 864);
        assert_eq!(report.required_predicate_results, 864 * 2);
        assert_eq!(report.not_applicable_predicates_per_hour, 5);
    }

    #[test]
    fn generator_service_cannot_precede_the_cargo_fuel_issue() {
        let coupled = couple_case(case("verified-on-time"), template(0), 0).expect("couple");
        assert_eq!(coupled.hourly_trajectory[11].fuel_issue_grams, 0);
        assert_eq!(coupled.hourly_trajectory[11].gross_generation_wh, 0);
        assert!(coupled.hourly_trajectory[12].fuel_issue_grams > 0);
        assert!(coupled.hourly_trajectory[12].gross_generation_wh > 0);
        assert_eq!(coupled.hourly_trajectory[13].fuel_issue_grams, 0);
        assert_eq!(coupled.hourly_trajectory[13].gross_generation_wh, 0);
    }

    #[test]
    fn late_receipt_and_transfer_do_not_backdate_generator_service() {
        let coupled = couple_case(case("partial-late-receipt"), template(1), 1).expect("couple");
        assert_eq!(coupled.hourly_trajectory[21].generator_stock_received_grams, 0);
        assert_eq!(coupled.hourly_trajectory[21].gross_generation_wh, 0);
        assert!(coupled.hourly_trajectory[22].generator_stock_received_grams > 0);
        assert_eq!(coupled.hourly_trajectory[23].gross_generation_wh, 0);
        assert!(coupled.hourly_trajectory[24].gross_generation_wh > 0);
    }

    #[test]
    fn missing_custody_cannot_produce_generator_service() {
        let coupled = couple_case(case("custody-missing"), template(2), 2).expect("couple");
        assert!(coupled.hourly_trajectory.iter().all(|row| row.gross_generation_wh == 0));
        assert_eq!(coupled.verdict, ServiceVerdict::Failed);
    }

    #[test]
    fn stale_telemetry_remains_indeterminate_even_when_generation_occurs() {
        let coupled = build_corpus().expect("coupling corpus").into_iter()
            .find(|item| item.case_id == "verified-on-time").expect("on-time case");
        assert!(coupled.hourly_trajectory[12].gross_generation_wh > 0);
        assert_eq!(coupled.hourly_trajectory[12].telemetry_state, TelemetryState::StaleOrConflicting);
        assert_eq!(coupled.hourly_trajectory[12].qualification.service_verdict, ServiceVerdict::Indeterminate);
    }

    #[test]
    fn rejected_fuel_grade_never_generates_power() {
        let mut model = template(0);
        model.generator.fuel_grade_compatible = false;
        let coupled = couple_case(case("verified-on-time"), model, 0).expect("couple");
        assert!(coupled.hourly_trajectory.iter().all(|row| row.compatible_fuel_feed_grams == 0));
        assert!(coupled.hourly_trajectory.iter().all(|row| row.gross_generation_wh == 0));
    }

    #[test]
    fn generator_power_cap_is_non_compensatory_with_fuel_quantity() {
        let coupled = couple_case(case("verified-on-time"), template(0), 0).expect("couple");
        let hour = &coupled.hourly_trajectory[12];
        assert!(hour.power_cap_curtailed_wh > 0);
        assert_eq!(hour.gross_generation_wh, hour.generator_power_cap_wh);
    }

    #[test]
    fn hourly_vector_contains_two_required_and_five_scoped_not_applicable_predicates() {
        let coupled = couple_case(case("verified-on-time"), template(0), 0).expect("couple");
        let row = &coupled.hourly_trajectory[12];
        assert_eq!(
            row.qualification.required_guaranteed_count
                + row.qualification.required_failed_count
                + row.qualification.required_indeterminate_count,
            2
        );
        assert_eq!(row.qualification.not_applicable_count, 5);
        assert_eq!(row.average_active_power_lower_w, row.delivered_lower_wh);
        assert_eq!(row.average_active_power_upper_w, row.delivered_upper_wh);
    }

    #[test]
    fn planner_json_does_not_leak_receipt_assay_or_generator_truth() {
        let coupled = couple_case(case("verified-on-time"), template(0), 0).expect("couple");
        let planner = coupled.planner_json();
        assert!(planner.contains("\"planner_observation\""));
        assert!(!planner.contains("\"receiptEventId\""));
        assert!(!planner.contains("\"custodyRecords\""));
        assert!(!planner.contains("\"fuelIssueGrams\""));
        assert!(!planner.contains("\"grossGenerationWh\""));
        assert!(!planner.contains("\"serviceVectorVerdict\""));
    }
}
