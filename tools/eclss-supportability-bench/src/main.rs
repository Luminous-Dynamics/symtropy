use serde_json::{Value, json};
use std::process::ExitCode;

const SYNTHETIC: &str = include_str!(
    "../../../docs/research/eclss/fixtures/eclss-supportability-synthetic-registry-v0_1.json"
);
const ISS_REFERENCE: &str = include_str!(
    "../../../docs/research/eclss/fixtures/eclss-iss-public-reference-records-v0_1.json"
);
const ISS_OPERATIONAL: &str = include_str!(
    "../../../docs/research/eclss/fixtures/eclss-2026-iss-operational-cases-v0_1.json"
);
const AOGA_SCHEDULE: &str =
    include_str!("../../../docs/research/eclss/fixtures/eclss-aoga-schedule-snapshots-v0_1.json");

type Check<T> = Result<T, String>;

fn object<'a>(value: &'a Value, at: &str) -> Check<&'a serde_json::Map<String, Value>> {
    value
        .as_object()
        .ok_or_else(|| format!("{at}: expected object"))
}

fn exact_keys(value: &Value, expected: &[&str], at: &str) -> Check<()> {
    let actual = object(value, at)?;
    if actual.len() != expected.len() {
        return Err(format!(
            "{at}: expected {} keys, found {}",
            expected.len(),
            actual.len()
        ));
    }
    for key in expected {
        if !actual.contains_key(*key) {
            return Err(format!("{at}: missing required key {key}"));
        }
    }
    if let Some(key) = actual.keys().find(|key| !expected.contains(&key.as_str())) {
        return Err(format!("{at}: unexpected key {key}"));
    }
    Ok(())
}

fn field<'a>(value: &'a Value, key: &str, at: &str) -> Check<&'a Value> {
    object(value, at)?
        .get(key)
        .ok_or_else(|| format!("{at}: missing field {key}"))
}

fn text<'a>(value: &'a Value, at: &str) -> Check<&'a str> {
    value
        .as_str()
        .ok_or_else(|| format!("{at}: expected string"))
}

fn number(value: &Value, at: &str) -> Check<f64> {
    value
        .as_f64()
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("{at}: expected finite number"))
}

fn integer(value: &Value, at: &str) -> Check<u64> {
    value
        .as_u64()
        .ok_or_else(|| format!("{at}: expected non-negative integer"))
}

fn array<'a>(value: &'a Value, at: &str) -> Check<&'a Vec<Value>> {
    value
        .as_array()
        .ok_or_else(|| format!("{at}: expected array"))
}

fn parse_embedded(name: &str, source: &str) -> Check<Value> {
    serde_json::from_str(source).map_err(|error| format!("{name}: invalid JSON: {error}"))
}

fn require_eq<T: PartialEq + std::fmt::Debug>(actual: T, expected: T, at: &str) -> Check<()> {
    if actual != expected {
        return Err(format!("{at}: expected {expected:?}, got {actual:?}"));
    }
    Ok(())
}

fn validate_synthetic(value: &Value) -> Check<Value> {
    let at = "synthetic_registry";
    exact_keys(
        value,
        &[
            "benchmark_id",
            "version",
            "claim_status",
            "unit_basis",
            "reference_ledger",
            "cases",
        ],
        at,
    )?;
    require_eq(
        text(field(value, "benchmark_id", at)?, "benchmark_id")?,
        "ECLSS-SUPPORTABILITY-001",
        at,
    )?;
    require_eq(text(field(value, "version", at)?, "version")?, "0.1.0", at)?;
    require_eq(
        text(field(value, "claim_status", at)?, "claim_status")?,
        "SYNTHETIC_ONLY",
        at,
    )?;
    let basis = text(field(value, "unit_basis", at)?, "unit_basis")?;
    if !basis.contains("abstract synthetic") {
        return Err(format!(
            "{at}: unit basis must identify abstract synthetic units"
        ));
    }

    let ledger = field(value, "reference_ledger", at)?;
    exact_keys(
        ledger,
        &[
            "basis",
            "daily_demand_units",
            "recovery_fraction",
            "horizon_days",
            "initial_makeup_inventory_units",
            "expected_daily_makeup_units",
            "expected_total_makeup_units",
            "expected_unserved_units",
            "notes",
        ],
        "reference_ledger",
    )?;
    require_eq(
        text(field(ledger, "basis", at)?, "reference_ledger.basis")?,
        "ABSTRACT_SYNTHETIC_WATER_UNITS",
        "reference_ledger.basis",
    )?;

    let daily_demand = integer(
        field(ledger, "daily_demand_units", at)?,
        "daily_demand_units",
    )?;
    let horizon = integer(field(ledger, "horizon_days", at)?, "horizon_days")?;
    let inventory = integer(
        field(ledger, "initial_makeup_inventory_units", at)?,
        "initial_makeup_inventory_units",
    )?;
    let recovery = number(field(ledger, "recovery_fraction", at)?, "recovery_fraction")?;
    if daily_demand == 0 || horizon == 0 || !(0.0..=1.0).contains(&recovery) {
        return Err(format!(
            "{at}: invalid demand, horizon, or recovery fraction"
        ));
    }

    // Convert the declared decimal recovery ratio to exact integer basis points.
    let recovery_bps = (recovery * 10_000.0).round() as u64;
    if (recovery * 10_000.0 - recovery_bps as f64).abs() > 1e-9 {
        return Err(format!(
            "{at}: recovery ratio is not representable in basis points"
        ));
    }
    let makeup_daily_numerator = daily_demand
        .checked_mul(10_000 - recovery_bps)
        .ok_or_else(|| format!("{at}: overflow computing daily make-up"))?;
    if makeup_daily_numerator % 10_000 != 0 {
        return Err(format!(
            "{at}: reference arithmetic is not integral in v0.1"
        ));
    }
    let makeup_daily = makeup_daily_numerator / 10_000;
    let makeup_total = makeup_daily
        .checked_mul(horizon)
        .ok_or_else(|| format!("{at}: overflow computing mission make-up"))?;
    let unserved = makeup_total.saturating_sub(inventory);

    require_eq(
        makeup_daily,
        integer(
            field(ledger, "expected_daily_makeup_units", at)?,
            "expected_daily_makeup_units",
        )?,
        "expected_daily_makeup_units",
    )?;
    require_eq(
        makeup_total,
        integer(
            field(ledger, "expected_total_makeup_units", at)?,
            "expected_total_makeup_units",
        )?,
        "expected_total_makeup_units",
    )?;
    require_eq(
        unserved,
        integer(
            field(ledger, "expected_unserved_units", at)?,
            "expected_unserved_units",
        )?,
        "expected_unserved_units",
    )?;

    let cases = array(field(value, "cases", at)?, "cases")?;
    require_eq(cases.len(), 30usize, "synthetic case count")?;
    let allowed_families = [
        "WATER_QUALITY",
        "WATER_BALANCE",
        "OXYGEN_RECOVERY",
        "ATMOSPHERE",
        "CONTAMINANT_CONTROL",
        "COMMON_CAUSE",
        "THROUGHPUT",
        "POWER_THERMAL",
        "SPARES",
        "MAINTENANCE",
        "REQUALIFICATION",
        "BIOREGENERATIVE",
        "LOGISTICS",
        "OBSERVABILITY",
        "OUT_OF_DOMAIN",
        "METAMORPHIC_TAMPER",
    ];
    let allowed_dispositions = [
        "ACCEPT_SERVICE",
        "REJECT_OUTPUT",
        "DEGRADED_SERVICE",
        "NO_SERVICE",
        "INDETERMINATE",
        "REJECT_INPUT",
    ];

    for (index, case) in cases.iter().enumerate() {
        let case_at = format!("cases[{index}]");
        exact_keys(
            case,
            &[
                "case_id",
                "family",
                "description",
                "stressors",
                "expected_disposition",
                "independent_assertion",
                "critical_control",
            ],
            &case_at,
        )?;
        let expected_id = format!("ECLSS-{:03}", index + 1);
        require_eq(
            text(field(case, "case_id", &case_at)?, "case_id")?,
            expected_id.as_str(),
            &case_at,
        )?;
        if !allowed_families.contains(&text(field(case, "family", &case_at)?, "family")?) {
            return Err(format!("{case_at}: unknown case family"));
        }
        if !allowed_dispositions.contains(&text(
            field(case, "expected_disposition", &case_at)?,
            "expected_disposition",
        )?) {
            return Err(format!("{case_at}: unknown expected disposition"));
        }
        if text(field(case, "description", &case_at)?, "description")?.len() < 10
            || text(
                field(case, "independent_assertion", &case_at)?,
                "independent_assertion",
            )?
            .len()
                < 10
        {
            return Err(format!("{case_at}: description/assertion too short"));
        }
        let stressors = array(field(case, "stressors", &case_at)?, "stressors")?;
        if stressors.is_empty() {
            return Err(format!("{case_at}: at least one stressor is required"));
        }
        let mut seen = std::collections::BTreeSet::new();
        for stressor in stressors {
            let value = text(stressor, "stressor")?;
            if !seen.insert(value) {
                return Err(format!("{case_at}: duplicate stressor {value}"));
            }
        }
        let _ = field(case, "critical_control", &case_at)?
            .as_bool()
            .ok_or_else(|| format!("{case_at}: critical_control must be boolean"))?;
    }

    let expected_negative_controls = [
        ("ECLSS-002", "REJECT_OUTPUT"),
        ("ECLSS-007", "INDETERMINATE"),
        ("ECLSS-015", "NO_SERVICE"),
        ("ECLSS-016", "NO_SERVICE"),
        ("ECLSS-017", "NO_SERVICE"),
        ("ECLSS-020", "NO_SERVICE"),
        ("ECLSS-021", "NO_SERVICE"),
        ("ECLSS-029", "INDETERMINATE"),
        ("ECLSS-030", "REJECT_INPUT"),
    ];
    for (id, disposition) in expected_negative_controls {
        let case = cases
            .iter()
            .find(|case| case["case_id"] == id)
            .ok_or_else(|| format!("{at}: missing negative control {id}"))?;
        require_eq(
            text(&case["expected_disposition"], "expected_disposition")?,
            disposition,
            id,
        )?;
    }

    Ok(json!({
        "case_count": cases.len(),
        "case_ids_contiguous": true,
        "reference_ledger": {
            "daily_makeup_units": makeup_daily,
            "total_makeup_units": makeup_total,
            "initial_qualified_inventory_units": inventory,
            "unserved_units": unserved,
            "recovery_basis_points": recovery_bps
        },
        "seeded_fail_closed_controls": expected_negative_controls.len()
    }))
}

fn validate_public_reference(value: &Value) -> Check<Value> {
    let at = "iss_public_reference";
    exact_keys(
        value,
        &[
            "benchmark_id",
            "version",
            "evidence_class",
            "claim_status",
            "source",
            "records",
        ],
        at,
    )?;
    require_eq(
        text(field(value, "benchmark_id", at)?, "benchmark_id")?,
        "ECLSS-SUPPORTABILITY-001",
        at,
    )?;
    require_eq(
        text(field(value, "evidence_class", at)?, "evidence_class")?,
        "PUBLISHED_REFERENCE",
        at,
    )?;
    require_eq(
        text(field(value, "claim_status", at)?, "claim_status")?,
        "PRELIMINARY_REFERENCE_ONLY",
        at,
    )?;
    let source = field(value, "source", at)?;
    let limitations = array(field(source, "limitations", at)?, "source.limitations")?;
    if limitations.len() < 6 {
        return Err(format!("{at}: source limitations were lost"));
    }
    let records = array(field(value, "records", at)?, "records")?;
    require_eq(records.len(), 11usize, "ISS reference record count")?;
    let mut ids = std::collections::BTreeSet::new();

    for (index, record) in records.iter().enumerate() {
        let record_at = format!("iss_reference.records[{index}]");
        exact_keys(
            record,
            &[
                "record_id",
                "table",
                "printed_page",
                "subsystem",
                "component",
                "mass_kg",
                "mtbf_hours",
                "operational_mttr_hours",
                "operational_k_factor",
                "crew_r_and_r_hours",
                "preventive_r_and_r_interval_days",
                "preventive_r_and_r_duration_hours",
                "throughput_limited_life",
                "calendar_limited_life",
                "notes",
            ],
            &record_at,
        )?;
        let id = text(field(record, "record_id", &record_at)?, "record_id")?;
        if !ids.insert(id) {
            return Err(format!("{record_at}: duplicate record id {id}"));
        }
        for metric in [
            "mass_kg",
            "mtbf_hours",
            "operational_mttr_hours",
            "operational_k_factor",
            "crew_r_and_r_hours",
            "preventive_r_and_r_interval_days",
            "preventive_r_and_r_duration_hours",
        ] {
            let value = field(record, metric, &record_at)?;
            if !value.is_null() {
                let metric_value = number(value, metric)?;
                if metric_value < 0.0
                    || (metric == "mtbf_hours" || metric == "operational_k_factor")
                        && metric_value == 0.0
                {
                    return Err(format!("{record_at}.{metric}: invalid numeric value"));
                }
            }
        }
        for limit in ["throughput_limited_life", "calendar_limited_life"] {
            let value = field(record, limit, &record_at)?;
            if !value.is_null() {
                exact_keys(value, &["value", "unit"], &format!("{record_at}.{limit}"))?;
                if number(field(value, "value", limit)?, "limit.value")? < 0.0
                    || text(field(value, "unit", limit)?, "limit.unit")?.is_empty()
                {
                    return Err(format!("{record_at}.{limit}: invalid limit"));
                }
            }
        }
    }

    let distillation = records
        .iter()
        .find(|record| record["record_id"] == "ICES2025-127-UPA-DISTILLATION-ASSEMBLY")
        .ok_or_else(|| format!("{at}: missing distillation assembly row"))?;
    require_eq(
        number(
            &distillation["operational_mttr_hours"],
            "operational_mttr_hours",
        )?,
        7.0,
        "distillation operational MTTR",
    )?;
    require_eq(
        number(&distillation["crew_r_and_r_hours"], "crew_r_and_r_hours")?,
        3.0,
        "distillation crew R&R",
    )?;

    Ok(json!({
        "component_reference_records": records.len(),
        "unique_record_ids": ids.len(),
        "distinct_mttr_vs_crew_rr_preserved": true,
        "source_limitations_retained": limitations.len()
    }))
}

fn validate_operational_cases(value: &Value) -> Check<Value> {
    let at = "iss_operational_cases";
    exact_keys(
        value,
        &[
            "benchmark_id",
            "version",
            "evidence_class",
            "claim_status",
            "source",
            "records",
        ],
        at,
    )?;
    require_eq(
        text(field(value, "evidence_class", at)?, "evidence_class")?,
        "PUBLISHED_OPERATIONAL_CASES",
        at,
    )?;
    require_eq(
        text(field(value, "claim_status", at)?, "claim_status")?,
        "SOURCE_BOUNDED_NARRATIVE_REPLAY_ONLY",
        at,
    )?;
    let records = array(field(value, "records", at)?, "records")?;
    require_eq(records.len(), 10usize, "ISS operational record count")?;
    let statuses = [
        "OBSERVED_OPERATIONAL_EVENT_AND_RECOVERY",
        "OBSERVED_OPERATIONAL_FAILURE_SEQUENCE",
        "OBSERVED_OPERATIONAL_DEGRADATION_AND_ADAPTATION",
        "OBSERVED_LIFE_AND_CONSERVATIVE_OPERATING_LIMIT",
        "OBSERVED_SENSOR_FAILURE_AND_CONDITIONAL_LIFE_REVIEW",
        "OBSERVED_MONITORING_BENEFIT_AND_SENSOR_FAILURE_MODE",
        "PLANNED_DESIGN_FEATURE",
        "EXPLICIT_DOMAIN_LIMIT",
        "CERTIFICATION_SCOPE_LIMIT",
    ];
    let mut ids = std::collections::BTreeSet::new();
    for (index, record) in records.iter().enumerate() {
        let record_at = format!("iss_operational.records[{index}]");
        exact_keys(
            record,
            &[
                "record_id",
                "evidence_status",
                "date_or_period",
                "subsystem",
                "event",
                "service_consequence",
                "response",
                "benchmark_challenge",
                "source_locations",
                "claim_ceiling",
            ],
            &record_at,
        )?;
        let id = text(field(record, "record_id", &record_at)?, "record_id")?;
        if !ids.insert(id) {
            return Err(format!("{record_at}: duplicate record id {id}"));
        }
        if !statuses.contains(&text(
            field(record, "evidence_status", &record_at)?,
            "evidence_status",
        )?) {
            return Err(format!("{record_at}: unknown evidence status"));
        }
        for key in [
            "event",
            "service_consequence",
            "response",
            "benchmark_challenge",
            "claim_ceiling",
        ] {
            if text(field(record, key, &record_at)?, key)?.len() < 5 {
                return Err(format!("{record_at}.{key}: too short"));
            }
        }
        let locations = array(
            field(record, "source_locations", &record_at)?,
            "source_locations",
        )?;
        if locations.is_empty() {
            return Err(format!("{record_at}: source location missing"));
        }
        for (location_index, location) in locations.iter().enumerate() {
            let location_at = format!("{record_at}.source_locations[{location_index}]");
            exact_keys(
                location,
                &["pdf_page", "printed_page", "section"],
                &location_at,
            )?;
            integer(field(location, "pdf_page", &location_at)?, "pdf_page")?;
            if integer(
                field(location, "printed_page", &location_at)?,
                "printed_page",
            )? == 0
            {
                return Err(format!("{location_at}: printed page must be positive"));
            }
            text(field(location, "section", &location_at)?, "section")?;
        }
    }
    if !ids.contains("NASA-ICES-2026-TAS-SPARES-DEPENDENT-OPERATIONS") {
        return Err(format!("{at}: finite-spares operational case missing"));
    }
    if !ids.contains("NASA-ICES-2026-FBCO2-SPARE-VALVE-MISMATCH") {
        return Err(format!(
            "{at}: configuration-mismatch operational case missing"
        ));
    }
    Ok(json!({
        "operational_records": records.len(),
        "unique_record_ids": ids.len(),
        "all_records_have_source_locations_and_claim_ceilings": true
    }))
}

fn validate_schedule(value: &Value) -> Check<Value> {
    let at = "aoga_schedule";
    exact_keys(
        value,
        &[
            "benchmark_id",
            "version",
            "evidence_class",
            "claim_status",
            "research_cutoff",
            "records",
            "interpretation",
        ],
        at,
    )?;
    require_eq(
        text(field(value, "claim_status", at)?, "claim_status")?,
        "SCHEDULE_VARIANCE_UNRESOLVED",
        at,
    )?;
    let records = array(field(value, "records", at)?, "records")?;
    require_eq(records.len(), 5usize, "AOGA schedule snapshot count")?;
    let mut ids = std::collections::BTreeSet::new();
    let mut forecasts = std::collections::BTreeSet::new();

    for (index, record) in records.iter().enumerate() {
        let record_at = format!("aoga_schedule.records[{index}]");
        exact_keys(
            record,
            &[
                "record_id",
                "source",
                "source_url",
                "published",
                "source_type",
                "milestone",
                "forecast",
                "precision",
                "statement",
                "status",
                "notes",
            ],
            &record_at,
        )?;
        let id = text(field(record, "record_id", &record_at)?, "record_id")?;
        if !ids.insert(id) {
            return Err(format!("{record_at}: duplicate record id {id}"));
        }
        require_eq(
            text(field(record, "status", &record_at)?, "status")?,
            "PLANNED",
            &record_at,
        )?;
        forecasts.insert(text(field(record, "forecast", &record_at)?, "forecast")?);
        for key in ["source", "source_url", "milestone", "statement", "notes"] {
            if text(field(record, key, &record_at)?, key)?.is_empty() {
                return Err(format!("{record_at}.{key}: must not be empty"));
            }
        }
    }
    for required in ["early 2026", "late 2026", "mid-2027", "FY2027"] {
        if !forecasts.contains(required) {
            return Err(format!("{at}: missing schedule snapshot {required}"));
        }
    }
    let interpretation = field(value, "interpretation", at)?;
    exact_keys(
        interpretation,
        &["status", "explanation", "required_follow_up"],
        "aoga_schedule.interpretation",
    )?;
    require_eq(
        text(
            field(interpretation, "status", at)?,
            "interpretation.status",
        )?,
        "UNRESOLVED_SCHEDULE_VARIANCE",
        "interpretation.status",
    )?;
    Ok(json!({
        "schedule_snapshots": records.len(),
        "unique_snapshot_ids": ids.len(),
        "forecast_variance_preserved": true
    }))
}

fn validate_all() -> Check<Value> {
    let synthetic = parse_embedded("synthetic registry", SYNTHETIC)?;
    let public_reference = parse_embedded("ISS public reference", ISS_REFERENCE)?;
    let operational = parse_embedded("ISS operational cases", ISS_OPERATIONAL)?;
    let schedule = parse_embedded("AOGA schedule", AOGA_SCHEDULE)?;

    let synthetic_report = validate_synthetic(&synthetic)?;
    let reference_report = validate_public_reference(&public_reference)?;
    let operational_report = validate_operational_cases(&operational)?;
    let schedule_report = validate_schedule(&schedule)?;

    Ok(json!({
        "benchmark_id": "ECLSS-SUPPORTABILITY-001",
        "validator": "eclss-supportability-bench",
        "validator_version": env!("CARGO_PKG_VERSION"),
        "result": "PASS_FIXTURE_STRUCTURE_AND_KNOWN_ANSWER",
        "claim_boundary": "No physical ECLSS performance, calibrated failure probability, simulator parity, or hardware qualification is evaluated.",
        "synthetic": synthetic_report,
        "published_reference": reference_report,
        "published_operational_cases": operational_report,
        "programme_schedule": schedule_report
    }))
}

fn main() -> ExitCode {
    match validate_all() {
        Ok(report) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&report).expect("report serializes")
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("ECLSS benchmark validation failed: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_registries_and_known_answers_validate() {
        let report = validate_all().expect("all frozen benchmark registries must validate");
        assert_eq!(report["result"], "PASS_FIXTURE_STRUCTURE_AND_KNOWN_ANSWER");
        assert_eq!(
            report["synthetic"]["reference_ledger"]["total_makeup_units"],
            60
        );
        assert_eq!(
            report["synthetic"]["reference_ledger"]["unserved_units"],
            20
        );
        assert_eq!(
            report["published_operational_cases"]["operational_records"],
            10
        );
        assert_eq!(report["programme_schedule"]["schedule_snapshots"], 5);
    }

    #[test]
    fn high_recovery_does_not_hide_finite_inventory_deficit() {
        let report = validate_all().expect("fixture validation");
        let ledger = &report["synthetic"]["reference_ledger"];
        assert_eq!(ledger["daily_makeup_units"], 2);
        assert_eq!(ledger["initial_qualified_inventory_units"], 40);
        assert_eq!(ledger["unserved_units"], 20);
    }

    #[test]
    fn operational_and_planned_claims_keep_distinct_status() {
        let report = validate_all().expect("fixture validation");
        assert_eq!(
            report["published_operational_cases"]["operational_records"],
            10
        );
        assert_eq!(
            report["programme_schedule"]["forecast_variance_preserved"],
            true
        );
    }
}
