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
const ISS_APPENDIX_SUPPLEMENT: &str =
    include_str!("../../../docs/research/eclss/fixtures/eclss-iss-appendix-supplement-v0_1.json");
const AOGA_SCHEDULE: &str =
    include_str!("../../../docs/research/eclss/fixtures/eclss-aoga-schedule-snapshots-v0_1.json");
const MAINTENANCE_PLAN_ADMISSIBILITY: &str = include_str!(
    "../../../docs/research/eclss/fixtures/eclss-maintenance-plan-admissibility-v0_1.json"
);

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
            "component_records",
            "crew_task_records",
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
    if limitations.len() < 7 {
        return Err(format!("{at}: source limitations were lost"));
    }

    let records = array(field(value, "component_records", at)?, "component_records")?;
    require_eq(records.len(), 9usize, "selected ISS component record count")?;
    let mut component_ids = std::collections::BTreeSet::new();
    for (index, record) in records.iter().enumerate() {
        let record_at = format!("iss_reference.component_records[{index}]");
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
                "component_r_and_r_hours",
                "preventive_r_and_r_interval_days",
                "preventive_r_and_r_duration_hours",
                "throughput_limited_life",
                "calendar_limited_life",
                "notes",
            ],
            &record_at,
        )?;
        let id = text(field(record, "record_id", &record_at)?, "record_id")?;
        if id.starts_with("ICES2025-127-CREW-") || !component_ids.insert(id) {
            return Err(format!(
                "{record_at}: invalid or duplicate component record id {id}"
            ));
        }
        for metric in [
            "mass_kg",
            "mtbf_hours",
            "operational_mttr_hours",
            "operational_k_factor",
            "component_r_and_r_hours",
            "preventive_r_and_r_interval_days",
            "preventive_r_and_r_duration_hours",
        ] {
            let metric_value = field(record, metric, &record_at)?;
            if !metric_value.is_null() {
                let value = number(metric_value, metric)?;
                if value < 0.0
                    || (metric == "mtbf_hours"
                        || metric == "operational_k_factor"
                        || metric == "preventive_r_and_r_interval_days")
                        && value == 0.0
                {
                    return Err(format!("{record_at}.{metric}: invalid numeric value"));
                }
            }
        }
        for limit in ["throughput_limited_life", "calendar_limited_life"] {
            let limit_value = field(record, limit, &record_at)?;
            if !limit_value.is_null() {
                exact_keys(
                    limit_value,
                    &["value", "unit"],
                    &format!("{record_at}.{limit}"),
                )?;
                if number(field(limit_value, "value", limit)?, "limit.value")? < 0.0
                    || text(field(limit_value, "unit", limit)?, "limit.unit")?.is_empty()
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
    if !distillation["component_r_and_r_hours"].is_null() {
        return Err(format!(
            "{at}: Table 1 crew task time was duplicated into the Table 3 component row"
        ));
    }
    let actex = records
        .iter()
        .find(|record| record["record_id"] == "ICES2025-127-WATER-STORAGE-ACTEX-FILTER")
        .ok_or_else(|| format!("{at}: missing water-storage ACTEX row"))?;
    require_eq(
        number(&actex["component_r_and_r_hours"], "component_r_and_r_hours")?,
        0.5,
        "Table 5 component R&R hours",
    )?;
    let brine_bladders = records
        .iter()
        .find(|record| record["record_id"] == "ICES2025-127-BPA-BLADDER")
        .ok_or_else(|| format!("{at}: missing brine processor bladder row"))?;
    if !brine_bladders["throughput_limited_life"].is_null() {
        return Err(format!(
            "{at}: batch cadence must not be encoded as a throughput-limited life"
        ));
    }
    require_eq(
        number(
            &brine_bladders["calendar_limited_life"]["value"],
            "brine bladder calendar life",
        )?,
        35.0,
        "brine bladder 35-week disposal limit",
    )?;
    if !brine_bladders["notes"]
        .as_str()
        .is_some_and(|note| note.contains("24 L") && note.contains("26 days"))
    {
        return Err(format!("{at}: BPA batch volume/cadence reference was lost"));
    }

    let crew_tasks = array(field(value, "crew_task_records", at)?, "crew_task_records")?;
    require_eq(crew_tasks.len(), 2usize, "selected Table 1 crew task count")?;
    let mut crew_ids = std::collections::BTreeSet::new();
    for (index, task) in crew_tasks.iter().enumerate() {
        let task_at = format!("iss_reference.crew_task_records[{index}]");
        exact_keys(
            task,
            &[
                "record_id",
                "table",
                "printed_page",
                "subsystem",
                "oru",
                "crew_time_minutes",
                "notes",
            ],
            &task_at,
        )?;
        let id = text(field(task, "record_id", &task_at)?, "record_id")?;
        if !id.starts_with("ICES2025-127-CREW-") || !crew_ids.insert(id) {
            return Err(format!("{task_at}: invalid or duplicate crew task id {id}"));
        }
        require_eq(
            text(field(task, "table", &task_at)?, "table")?,
            "Table 1",
            &task_at,
        )?;
        require_eq(
            integer(field(task, "printed_page", &task_at)?, "printed_page")?,
            9,
            &task_at,
        )?;
        if integer(
            field(task, "crew_time_minutes", &task_at)?,
            "crew_time_minutes",
        )? == 0
        {
            return Err(format!("{task_at}: crew time must be positive"));
        }
        for key in ["subsystem", "oru", "notes"] {
            if text(field(task, key, &task_at)?, key)?.is_empty() {
                return Err(format!("{task_at}.{key}: must not be empty"));
            }
        }
    }
    let oga_hydrogen = crew_tasks
        .iter()
        .find(|task| task["oru"] == "OGA — Hydrogen")
        .ok_or_else(|| format!("{at}: missing selected OGA hydrogen task"))?;
    require_eq(
        integer(&oga_hydrogen["crew_time_minutes"], "crew_time_minutes")?,
        840,
        "OGA hydrogen crew R&R minutes",
    )?;
    let wpa_combined = crew_tasks
        .iter()
        .find(|task| {
            task["oru"]
                .as_str()
                .is_some_and(|oru| oru.starts_with("WPA — Gas Separator"))
        })
        .ok_or_else(|| format!("{at}: missing selected combined WPA task"))?;
    require_eq(
        integer(&wpa_combined["crew_time_minutes"], "crew_time_minutes")?,
        1155,
        "combined WPA crew R&R minutes",
    )?;

    Ok(json!({
        "selected_register_records": records.len() + crew_tasks.len(),
        "selected_component_records": records.len(),
        "selected_crew_task_records": crew_tasks.len(),
        "unique_component_record_ids": component_ids.len(),
        "unique_crew_task_record_ids": crew_ids.len(),
        "component_rr_distinct_from_table1_crew_time": true,
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

fn validate_appendix_supplement(value: &Value) -> Check<Value> {
    let at = "iss_appendix_supplement";
    exact_keys(
        value,
        &[
            "benchmark_id",
            "version",
            "evidence_class",
            "claim_status",
            "source",
            "records",
            "crew_task_records",
            "reconciliation",
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
        text(field(value, "evidence_class", at)?, "evidence_class")?,
        "PUBLISHED_REFERENCE_SUPPLEMENT",
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
    require_eq(records.len(), 133usize, "supplement component record count")?;
    let allowed_tables = [
        "Table 2", "Table 3", "Table 5", "Table 6", "Table 7", "Table 8", "Table 9", "Table 10",
    ];
    let mut ids = std::collections::BTreeSet::new();
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    for (index, record) in records.iter().enumerate() {
        let record_at = format!("iss_appendix_supplement.records[{index}]");
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
                "component_r_and_r_hours",
                "preventive_r_and_r_interval_days",
                "preventive_r_and_r_duration_hours",
                "throughput_limited_life",
                "calendar_limited_life",
                "notes",
            ],
            &record_at,
        )?;
        let id = text(field(record, "record_id", &record_at)?, "record_id")?;
        if !id.starts_with("ICES2025-127-T") || !ids.insert(id) {
            return Err(format!("{record_at}: invalid or duplicate record id {id}"));
        }
        let table = text(field(record, "table", &record_at)?, "table")?;
        if !allowed_tables.contains(&table) {
            return Err(format!("{record_at}: unexpected source table {table}"));
        }
        *counts.entry(table.to_owned()).or_default() += 1;
        if integer(field(record, "printed_page", &record_at)?, "printed_page")? == 0 {
            return Err(format!("{record_at}: printed page must be positive"));
        }
        for key in ["subsystem", "component", "notes"] {
            if text(field(record, key, &record_at)?, key)?.is_empty() {
                return Err(format!("{record_at}.{key}: must not be empty"));
            }
        }
        for metric in [
            "mass_kg",
            "mtbf_hours",
            "operational_mttr_hours",
            "operational_k_factor",
            "component_r_and_r_hours",
            "preventive_r_and_r_interval_days",
            "preventive_r_and_r_duration_hours",
        ] {
            let metric_value = field(record, metric, &record_at)?;
            if !metric_value.is_null() {
                let value = number(metric_value, metric)?;
                if value < 0.0
                    || (metric == "mtbf_hours"
                        || metric == "operational_k_factor"
                        || metric == "preventive_r_and_r_interval_days")
                        && value == 0.0
                {
                    return Err(format!("{record_at}.{metric}: invalid numeric value"));
                }
            }
        }
        for limit in ["throughput_limited_life", "calendar_limited_life"] {
            let limit_value = field(record, limit, &record_at)?;
            if !limit_value.is_null() {
                exact_keys(
                    limit_value,
                    &["value", "unit"],
                    &format!("{record_at}.{limit}"),
                )?;
                if number(field(limit_value, "value", limit)?, "limit.value")? < 0.0
                    || text(field(limit_value, "unit", limit)?, "limit.unit")?.is_empty()
                {
                    return Err(format!("{record_at}.{limit}: invalid limit"));
                }
            }
        }
    }
    for (table, expected) in [
        ("Table 2", 12usize),
        ("Table 3", 7usize),
        ("Table 5", 12usize),
        ("Table 6", 30usize),
        ("Table 7", 16usize),
        ("Table 8", 24usize),
        ("Table 9", 21usize),
        ("Table 10", 11usize),
    ] {
        require_eq(
            counts.get(table).copied().unwrap_or_default(),
            expected,
            table,
        )?;
    }

    let actex = records
        .iter()
        .find(|record| record["table"] == "Table 6" && record["component"] == "ACTEX")
        .ok_or_else(|| format!("{at}: missing Table 6 ACTEX row"))?;
    require_eq(
        integer(
            &actex["preventive_r_and_r_interval_days"],
            "ACTEX PM interval",
        )?,
        1460,
        "Table 6 ACTEX PM interval",
    )?;
    require_eq(
        number(
            &actex["preventive_r_and_r_duration_hours"],
            "ACTEX PM duration",
        )?,
        0.33,
        "Table 6 ACTEX PM duration",
    )?;
    require_eq(
        number(
            &actex["throughput_limited_life"]["value"],
            "ACTEX throughput",
        )?,
        6750.0,
        "Table 6 ACTEX throughput limit",
    )?;

    let hydrogen_sensor = records
        .iter()
        .find(|record| {
            record["table"] == "Table 10" && record["component"] == "Sensor, Hydrogen ORU"
        })
        .ok_or_else(|| format!("{at}: missing OGA hydrogen sensor row"))?;
    require_eq(
        integer(
            &hydrogen_sensor["preventive_r_and_r_interval_days"],
            "H2 sensor PM interval",
        )?,
        201,
        "H2 sensor PM interval",
    )?;

    let crew_tasks = array(field(value, "crew_task_records", at)?, "crew_task_records")?;
    require_eq(crew_tasks.len(), 39usize, "supplement crew task count")?;
    let mut crew_ids = std::collections::BTreeSet::new();
    for (index, task) in crew_tasks.iter().enumerate() {
        let task_at = format!("iss_appendix_supplement.crew_task_records[{index}]");
        exact_keys(
            task,
            &[
                "record_id",
                "table",
                "printed_page",
                "subsystem",
                "oru",
                "crew_time_minutes",
                "notes",
            ],
            &task_at,
        )?;
        let id = text(field(task, "record_id", &task_at)?, "record_id")?;
        if !id.starts_with("ICES2025-127-CREW-") || !crew_ids.insert(id) {
            return Err(format!("{task_at}: invalid or duplicate crew task id {id}"));
        }
        require_eq(
            text(field(task, "table", &task_at)?, "table")?,
            "Table 1",
            &task_at,
        )?;
        require_eq(
            integer(field(task, "printed_page", &task_at)?, "printed_page")?,
            9,
            &task_at,
        )?;
        if integer(
            field(task, "crew_time_minutes", &task_at)?,
            "crew_time_minutes",
        )? == 0
        {
            return Err(format!("{task_at}: crew time must be positive"));
        }
        for key in ["subsystem", "oru", "notes"] {
            if text(field(task, key, &task_at)?, key)?.is_empty() {
                return Err(format!("{task_at}.{key}: must not be empty"));
            }
        }
    }
    let heat_exchanger_task = crew_tasks
        .iter()
        .find(|task| task["oru"] == "Lab CCAA — Heat Exchanger")
        .ok_or_else(|| format!("{at}: missing CCAA heat exchanger crew task"))?;
    require_eq(
        integer(
            &heat_exchanger_task["crew_time_minutes"],
            "crew task minutes",
        )?,
        330,
        "CCAA heat exchanger crew R&R",
    )?;

    let reconciliation = field(value, "reconciliation", at)?;
    exact_keys(
        reconciliation,
        &[
            "selected_register_component_records",
            "supplementary_component_records",
            "combined_component_records",
            "selected_register_crew_task_records",
            "supplementary_crew_task_records",
            "combined_crew_task_records",
            "note",
        ],
        "reconciliation",
    )?;
    for (key, expected) in [
        ("selected_register_component_records", 9u64),
        ("supplementary_component_records", 133u64),
        ("combined_component_records", 142u64),
        ("selected_register_crew_task_records", 2u64),
        ("supplementary_crew_task_records", 39u64),
        ("combined_crew_task_records", 41u64),
    ] {
        require_eq(
            integer(field(reconciliation, key, at)?, key)?,
            expected,
            key,
        )?;
    }

    Ok(json!({
        "additional_component_records": records.len(),
        "unique_component_record_ids": ids.len(),
        "additional_crew_task_records": crew_tasks.len(),
        "unique_crew_task_record_ids": crew_ids.len(),
        "combined_component_records": 142,
        "combined_crew_task_records": 41,
        "source_limitations_retained": limitations.len(),
        "life_limit_types_kept_separate": true
    }))
}

#[derive(Clone)]
struct MaintenanceTask {
    task_id: String,
    start: u64,
    end: u64,
    worker_id: String,
    tool_ids: Vec<String>,
    dependencies: Vec<String>,
}

fn validate_maintenance_plan_admissibility(value: &Value) -> Check<Value> {
    let at = "maintenance_plan_admissibility";
    exact_keys(
        value,
        &[
            "benchmark_id",
            "contract_id",
            "version",
            "claim_status",
            "unit_basis",
            "cases",
        ],
        at,
    )?;
    require_eq(
        text(field(value, "benchmark_id", at)?, "benchmark_id")?,
        "ECLSS-SUPPORTABILITY-001",
        at,
    )?;
    require_eq(
        text(field(value, "contract_id", at)?, "contract_id")?,
        "ECLSS-MAINTENANCE-PLAN-ADMISSIBILITY-001",
        at,
    )?;
    require_eq(text(field(value, "version", at)?, "version")?, "0.1.0", at)?;
    require_eq(
        text(field(value, "claim_status", at)?, "claim_status")?,
        "SYNTHETIC_ONLY",
        at,
    )?;
    if !text(field(value, "unit_basis", at)?, "unit_basis")?.contains("integer") {
        return Err(format!(
            "{at}: unit basis must declare integer time/resource units"
        ));
    }

    let cases = array(field(value, "cases", at)?, "cases")?;
    require_eq(cases.len(), 13usize, "maintenance plan case count")?;
    let expected_reasons = [
        "NONE",
        "SPARE_CONFIGURATION_MISMATCH",
        "SPARE_EXPIRED",
        "ISOLATION_NOT_VERIFIED",
        "SERVICE_FLOOR_BREACH",
        "WORKER_NOT_QUALIFIED",
        "POST_REPAIR_TEST_MISSING_OR_STALE",
        "WORKER_OR_TOOL_OVERLAP",
        "SPARE_OVERALLOCATED",
        "AUTHORITY_EXPIRED_OR_REVOKED",
        "DEPENDENCY_ORDER_INVALID",
        "TASK_OUTSIDE_MAINTENANCE_WINDOW",
        "PROCEDURE_NOT_APPROVED",
    ];
    let mut accepted = 0usize;
    let mut rejected = 0usize;
    let mut reports = Vec::new();
    let mut ids = std::collections::BTreeSet::new();

    for (case_index, case) in cases.iter().enumerate() {
        let case_at = format!("cases[{case_index}]");
        exact_keys(
            case,
            &["case_id", "plan", "expected_disposition", "expected_reason"],
            &case_at,
        )?;
        let case_id = text(field(case, "case_id", &case_at)?, "case_id")?;
        let expected_id = format!("MPA-{:03}", case_index + 1);
        require_eq(case_id, expected_id.as_str(), &case_at)?;
        if !ids.insert(case_id) {
            return Err(format!("{case_at}: duplicate case id"));
        }
        let expected_disposition = text(
            field(case, "expected_disposition", &case_at)?,
            "expected_disposition",
        )?;
        let expected_reason = text(field(case, "expected_reason", &case_at)?, "expected_reason")?;
        if !["ADMISSIBLE", "REJECTED"].contains(&expected_disposition)
            || !expected_reasons.contains(&expected_reason)
        {
            return Err(format!("{case_at}: unknown expected disposition/reason"));
        }

        let plan = field(case, "plan", &case_at)?;
        exact_keys(
            plan,
            &[
                "plan_id",
                "mission_window_end_minute",
                "assets",
                "workers",
                "tools",
                "procedures",
                "spares",
                "isolation_records",
                "authorizations",
                "tasks",
            ],
            &format!("{case_at}.plan"),
        )?;
        let plan_id = text(field(plan, "plan_id", &case_at)?, "plan_id")?.to_owned();
        let window_end = integer(
            field(plan, "mission_window_end_minute", &case_at)?,
            "mission_window_end_minute",
        )?;
        let assets = array(field(plan, "assets", &case_at)?, "assets")?;
        let workers = array(field(plan, "workers", &case_at)?, "workers")?;
        let tools = array(field(plan, "tools", &case_at)?, "tools")?;
        let procedures = array(field(plan, "procedures", &case_at)?, "procedures")?;
        let spares = array(field(plan, "spares", &case_at)?, "spares")?;
        let isolations = array(
            field(plan, "isolation_records", &case_at)?,
            "isolation_records",
        )?;
        let authorizations = array(field(plan, "authorizations", &case_at)?, "authorizations")?;
        let tasks = array(field(plan, "tasks", &case_at)?, "tasks")?;
        if tasks.is_empty() {
            return Err(format!(
                "{case_at}: at least one maintenance task is required"
            ));
        }

        let mut computed_reason: Option<&'static str> = None;
        macro_rules! reject {
            ($condition:expr, $reason:expr) => {
                if $condition && computed_reason.is_none() {
                    computed_reason = Some($reason);
                }
            };
        }
        let mut task_records = Vec::<MaintenanceTask>::new();
        let mut spare_demand = std::collections::BTreeMap::<String, usize>::new();

        for (task_index, task) in tasks.iter().enumerate() {
            let task_at = format!("{case_at}.plan.tasks[{task_index}]");
            exact_keys(
                task,
                &[
                    "task_id",
                    "asset_id",
                    "asset_revision",
                    "procedure_id",
                    "procedure_revision",
                    "start_minute",
                    "duration_minutes",
                    "worker_id",
                    "required_skill",
                    "tool_ids",
                    "spare_id",
                    "isolation_evidence_id",
                    "required_service_floor_bp",
                    "service_capacity_during_bp",
                    "authority_id",
                    "post_test_id",
                    "post_test_asset_revision",
                    "post_test_status",
                    "dependencies",
                ],
                &task_at,
            )?;

            let task_id = text(field(task, "task_id", &task_at)?, "task_id")?.to_owned();
            let asset_id = text(field(task, "asset_id", &task_at)?, "asset_id")?;
            let asset_revision = text(field(task, "asset_revision", &task_at)?, "asset_revision")?;
            let procedure_id = text(field(task, "procedure_id", &task_at)?, "procedure_id")?;
            let procedure_revision = text(
                field(task, "procedure_revision", &task_at)?,
                "procedure_revision",
            )?;
            let start = integer(field(task, "start_minute", &task_at)?, "start_minute")?;
            let duration = integer(
                field(task, "duration_minutes", &task_at)?,
                "duration_minutes",
            )?;
            if duration == 0 {
                return Err(format!("{task_at}: duration must be positive"));
            }
            let end = start
                .checked_add(duration)
                .ok_or_else(|| format!("{task_at}: task end overflow"))?;
            let worker_id = text(field(task, "worker_id", &task_at)?, "worker_id")?.to_owned();
            let required_skill = text(field(task, "required_skill", &task_at)?, "required_skill")?;
            let tool_ids = array(field(task, "tool_ids", &task_at)?, "tool_ids")?
                .iter()
                .map(|tool| text(tool, "tool_id").map(str::to_owned))
                .collect::<Check<Vec<_>>>()?;
            let spare_value = field(task, "spare_id", &task_at)?;
            let spare_id = if spare_value.is_null() {
                None
            } else {
                Some(text(spare_value, "spare_id")?.to_owned())
            };
            let isolation_id = text(
                field(task, "isolation_evidence_id", &task_at)?,
                "isolation_evidence_id",
            )?;
            let floor = integer(
                field(task, "required_service_floor_bp", &task_at)?,
                "required_service_floor_bp",
            )?;
            let during_capacity = integer(
                field(task, "service_capacity_during_bp", &task_at)?,
                "service_capacity_during_bp",
            )?;
            let authority_id = text(field(task, "authority_id", &task_at)?, "authority_id")?;
            let post_test_status = text(
                field(task, "post_test_status", &task_at)?,
                "post_test_status",
            )?;
            let post_test_revision = text(
                field(task, "post_test_asset_revision", &task_at)?,
                "post_test_asset_revision",
            )?;
            let dependencies = array(field(task, "dependencies", &task_at)?, "dependencies")?
                .iter()
                .map(|dep| text(dep, "dependency").map(str::to_owned))
                .collect::<Check<Vec<_>>>()?;

            let asset = assets
                .iter()
                .find(|asset| asset["asset_id"] == asset_id)
                .ok_or_else(|| format!("{task_at}: referenced asset is missing"))?;
            let installed_revision = text(
                field(asset, "installed_revision", "asset")?,
                "installed_revision",
            )?;
            if installed_revision != asset_revision {
                return Err(format!(
                    "{task_at}: task asset revision does not match installed asset"
                ));
            }

            let procedure = procedures
                .iter()
                .find(|record| record["procedure_id"] == procedure_id)
                .ok_or_else(|| format!("{task_at}: referenced procedure is missing"))?;
            let procedure_is_valid = procedure["revision"] == procedure_revision
                && procedure["approved"].as_bool() == Some(true)
                && integer(
                    &procedure["valid_through_minute"],
                    "procedure.valid_through_minute",
                )? >= end;
            reject!(!procedure_is_valid, "PROCEDURE_NOT_APPROVED");

            let worker = workers
                .iter()
                .find(|record| record["worker_id"] == worker_id)
                .ok_or_else(|| format!("{task_at}: referenced worker is missing"))?;
            let worker_has_skill = array(&worker["skills"], "worker.skills")?
                .iter()
                .any(|skill| skill.as_str() == Some(required_skill));
            let worker_is_valid = worker["status"] == "CURRENT"
                && worker_has_skill
                && integer(
                    &worker["valid_through_minute"],
                    "worker.valid_through_minute",
                )? >= end;
            reject!(!worker_is_valid, "WORKER_NOT_QUALIFIED");

            for tool_id in &tool_ids {
                let tool = tools
                    .iter()
                    .find(|record| record["tool_id"] == tool_id.as_str())
                    .ok_or_else(|| format!("{task_at}: referenced tool {tool_id} is missing"))?;
                reject!(tool["status"] != "AVAILABLE", "WORKER_OR_TOOL_OVERLAP");
            }

            if let Some(spare_id) = &spare_id {
                let spare = spares
                    .iter()
                    .find(|record| record["spare_id"] == spare_id.as_str())
                    .ok_or_else(|| format!("{task_at}: referenced spare {spare_id} is missing"))?;
                let compatible_assets =
                    array(&spare["compatible_asset_ids"], "spare.compatible_asset_ids")?;
                let config_matches = spare["qualified"].as_bool() == Some(true)
                    && spare["compatible_revision"] == asset_revision
                    && compatible_assets
                        .iter()
                        .any(|id| id.as_str() == Some(asset_id));
                reject!(!config_matches, "SPARE_CONFIGURATION_MISMATCH");
                reject!(
                    integer(&spare["expiry_minute"], "spare.expiry_minute")? < end,
                    "SPARE_EXPIRED"
                );
                *spare_demand.entry(spare_id.clone()).or_default() += 1;
            }

            let isolation = isolations
                .iter()
                .find(|record| record["evidence_id"] == isolation_id)
                .ok_or_else(|| format!("{task_at}: isolation evidence is missing"))?;
            let isolation_is_valid = isolation["asset_id"] == asset_id
                && isolation["asset_revision"] == asset_revision
                && isolation["status"] == "VERIFIED"
                && integer(
                    &isolation["valid_through_minute"],
                    "isolation.valid_through_minute",
                )? >= end;
            reject!(!isolation_is_valid, "ISOLATION_NOT_VERIFIED");

            reject!(during_capacity < floor, "SERVICE_FLOOR_BREACH");

            let authority = authorizations
                .iter()
                .find(|record| record["authority_id"] == authority_id)
                .ok_or_else(|| format!("{task_at}: responsible authority record is missing"))?;
            let authority_is_valid = authority["asset_id"] == asset_id
                && authority["asset_revision"] == asset_revision
                && authority["status"] == "APPROVED"
                && integer(
                    &authority["valid_through_minute"],
                    "authority.valid_through_minute",
                )? >= end;
            reject!(!authority_is_valid, "AUTHORITY_EXPIRED_OR_REVOKED");

            reject!(
                post_test_status != "PASS" || post_test_revision != asset_revision,
                "POST_REPAIR_TEST_MISSING_OR_STALE"
            );
            reject!(end > window_end, "TASK_OUTSIDE_MAINTENANCE_WINDOW");

            task_records.push(MaintenanceTask {
                task_id,
                start,
                end,
                worker_id,
                tool_ids,
                dependencies,
            });
        }

        for task in &task_records {
            for dependency_id in &task.dependencies {
                let dependency = task_records
                    .iter()
                    .find(|candidate| candidate.task_id == *dependency_id);
                match dependency {
                    Some(dependency) if dependency.end <= task.start => {}
                    _ => reject!(true, "DEPENDENCY_ORDER_INVALID"),
                }
            }
        }

        for left_index in 0..task_records.len() {
            for right_index in (left_index + 1)..task_records.len() {
                let left = &task_records[left_index];
                let right = &task_records[right_index];
                let overlaps = left.start < right.end && right.start < left.end;
                if !overlaps {
                    continue;
                }
                reject!(left.worker_id == right.worker_id, "WORKER_OR_TOOL_OVERLAP");
                for tool_id in &left.tool_ids {
                    let is_exclusive = tools
                        .iter()
                        .find(|tool| tool["tool_id"] == tool_id.as_str())
                        .map(|tool| tool["exclusive"].as_bool() == Some(true))
                        .unwrap_or(false);
                    if is_exclusive && right.tool_ids.contains(tool_id) {
                        reject!(true, "WORKER_OR_TOOL_OVERLAP");
                    }
                }
            }
        }

        for (spare_id, count) in spare_demand {
            let spare = spares
                .iter()
                .find(|record| record["spare_id"] == spare_id.as_str())
                .ok_or_else(|| format!("{case_at}: internal spare lookup failure"))?;
            let available = integer(&spare["available_quantity"], "spare.available_quantity")?;
            reject!(count as u64 > available, "SPARE_OVERALLOCATED");
        }

        let computed_reason = computed_reason.unwrap_or("NONE");
        let computed_disposition = if computed_reason == "NONE" {
            "ADMISSIBLE"
        } else {
            "REJECTED"
        };
        require_eq(
            computed_disposition,
            expected_disposition,
            &format!("{case_at}.expected_disposition"),
        )?;
        require_eq(
            computed_reason,
            expected_reason,
            &format!("{case_at}.expected_reason"),
        )?;
        if computed_disposition == "ADMISSIBLE" {
            accepted += 1;
        } else {
            rejected += 1;
        }
        reports.push(json!({
            "case_id": case_id,
            "plan_id": plan_id,
            "computed_disposition": computed_disposition,
            "computed_reason": computed_reason,
            "task_count": tasks.len(),
            "note": "ADMISSIBLE is a structural evidence-contract result, not execution authorization."
        }));
    }

    require_eq(accepted, 1usize, "maintenance plan admissible case count")?;
    require_eq(rejected, 12usize, "maintenance plan rejected case count")?;
    Ok(json!({
        "contract_id": "ECLSS-MAINTENANCE-PLAN-ADMISSIBILITY-001",
        "case_count": cases.len(),
        "admissible": accepted,
        "rejected": rejected,
        "execution_authorized": false,
        "claim_status": "SYNTHETIC_ONLY",
        "case_results": reports
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
    let appendix_supplement = parse_embedded("ISS appendix supplement", ISS_APPENDIX_SUPPLEMENT)?;
    let schedule = parse_embedded("AOGA schedule", AOGA_SCHEDULE)?;
    let maintenance_plan_admissibility = parse_embedded(
        "maintenance plan admissibility",
        MAINTENANCE_PLAN_ADMISSIBILITY,
    )?;

    let synthetic_report = validate_synthetic(&synthetic)?;
    let reference_report = validate_public_reference(&public_reference)?;
    let operational_report = validate_operational_cases(&operational)?;
    let appendix_supplement_report = validate_appendix_supplement(&appendix_supplement)?;
    let schedule_report = validate_schedule(&schedule)?;
    let maintenance_plan_report =
        validate_maintenance_plan_admissibility(&maintenance_plan_admissibility)?;

    Ok(json!({
        "benchmark_id": "ECLSS-SUPPORTABILITY-001",
        "validator": "eclss-supportability-bench",
        "validator_version": env!("CARGO_PKG_VERSION"),
        "result": "PASS_FIXTURE_STRUCTURE_AND_KNOWN_ANSWER",
        "claim_boundary": "No physical ECLSS performance, calibrated failure probability, simulator parity, or hardware qualification is evaluated.",
        "synthetic": synthetic_report,
        "published_reference": reference_report,
        "published_operational_cases": operational_report,
        "published_appendix_supplement": appendix_supplement_report,
        "programme_schedule": schedule_report,
        "maintenance_plan_admissibility": maintenance_plan_report
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
    fn appendix_reference_rows_reconcile_to_the_full_published_tables() {
        let report = validate_all().expect("appendix reference rows must validate");
        assert_eq!(
            report["published_appendix_supplement"]["additional_component_records"],
            133
        );
        assert_eq!(
            report["published_appendix_supplement"]["additional_crew_task_records"],
            39
        );
        assert_eq!(
            report["published_appendix_supplement"]["combined_component_records"],
            142
        );
        assert_eq!(
            report["published_appendix_supplement"]["combined_crew_task_records"],
            41
        );
    }

    #[test]
    fn maintenance_plan_admissibility_rejects_all_seeded_negative_controls() {
        let report = validate_all().expect("maintenance plan cases must independently validate");
        assert_eq!(report["maintenance_plan_admissibility"]["case_count"], 13);
        assert_eq!(report["maintenance_plan_admissibility"]["admissible"], 1);
        assert_eq!(report["maintenance_plan_admissibility"]["rejected"], 12);
        assert_eq!(
            report["maintenance_plan_admissibility"]["execution_authorized"],
            false
        );
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
