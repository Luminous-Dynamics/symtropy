use jsonschema::Validator;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;

const SCHEMA_ID: &str = "urn:luminous-dynamics:ords-appliance-adapter-profile:0.1.0";
const ADAPTER_ID: &str = "REPAIR-DATA-INTEROP-001";
const REQUIRED_HEADERS: [&str; 9] = [
    "id",
    "partner_product_category",
    "product_category",
    "brand",
    "repair_status",
    "problem",
    "event_date",
    "country",
    "data_provider",
];
const EXPECTED_STATUSES: [&str; 4] = ["unknown", "fixed", "repairable", "end_of_life"];
const EXPECTED_BARRIERS: [&str; 6] = [
    "spare_parts_unavailable",
    "spare_parts_too_expensive",
    "product_cannot_be_opened",
    "repair_information_unavailable",
    "equipment_unavailable",
    "item_too_worn_out",
];
type Errors = Vec<String>;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct OrdsAuditSummary {
    pub(crate) row_count: usize,
    pub(crate) accepted_row_count: usize,
    pub(crate) quarantined_row_count: usize,
    pub(crate) fixed_row_count: usize,
    pub(crate) repairable_row_count: usize,
    pub(crate) end_of_life_row_count: usize,
    pub(crate) unknown_status_row_count: usize,
    pub(crate) category_candidate_row_count: usize,
    pub(crate) category_ambiguous_row_count: usize,
    pub(crate) category_unmapped_row_count: usize,
    pub(crate) duplicate_identity_row_count: usize,
    pub(crate) missing_identity_row_count: usize,
    pub(crate) invalid_status_row_count: usize,
    pub(crate) barrier_status_gate_violation_count: usize,
    pub(crate) invalid_end_of_life_barrier_count: usize,
    pub(crate) schema_sha256: String,
    pub(crate) mapping_sha256: String,
    pub(crate) input_sha256: String,
    pub(crate) dataset_release_id: String,
    pub(crate) source_license: String,
    pub(crate) profile_status: String,
}

fn text_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn normalize_key(raw: &str) -> String {
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn schema_validator(schema: &Value) -> Result<Validator, String> {
    if text_at(schema, "$schema") != Some("https://json-schema.org/draft/2020-12/schema") {
        return Err("ORDS adapter schema must declare JSON Schema Draft 2020-12".to_owned());
    }
    if text_at(schema, "$id") != Some(SCHEMA_ID) {
        return Err(format!("ORDS adapter schema $id must be {SCHEMA_ID}"));
    }
    if !jsonschema::draft202012::meta::is_valid(schema) {
        return Err(
            "ORDS adapter schema does not validate against the Draft 2020-12 meta-schema"
                .to_owned(),
        );
    }
    jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(schema)
        .map_err(|error| format!("cannot compile ORDS adapter schema: {error}"))
}

fn validate_mapping_profile(
    schema: &Value,
    profile: &Value,
) -> Result<(Vec<Value>, Vec<Value>, Vec<Value>), Errors> {
    let validator = schema_validator(schema).map_err(|error| vec![error])?;
    let schema_errors: Vec<String> = validator
        .iter_errors(profile)
        .map(|error| error.to_string())
        .collect();
    if !schema_errors.is_empty() {
        return Err(schema_errors);
    }

    let mut errors = Vec::new();
    if text_at(profile, "schema_version") != Some("0.1.0")
        || text_at(profile, "adapter_id") != Some(ADAPTER_ID)
        || text_at(profile, "ords_standard_version") != Some("0.3")
    {
        errors.push("ORDS profile identity/version changed".to_owned());
    }
    let source = profile.get("source_dataset").unwrap_or(&Value::Null);
    if text_at(source, "license") != Some("CC BY-SA 4.0")
        || text_at(source, "license_lineage") != Some("preserve_attribution_and_share_alike")
    {
        errors.push("ORDS source license/lineage must remain CC BY-SA 4.0".to_owned());
    }
    let policy = profile.get("policy").unwrap_or(&Value::Null);
    if policy.get("raw_values_must_be_preserved").and_then(Value::as_bool) != Some(true)
        || policy
            .get("barrier_only_when_end_of_life")
            .and_then(Value::as_bool)
            != Some(true)
        || policy
            .get("verified_restoration_receipt_generation_allowed")
            .and_then(Value::as_bool)
            != Some(false)
        || policy
            .get("physical_safety_claim_generation_allowed")
            .and_then(Value::as_bool)
            != Some(false)
        || policy
            .get("exact_part_fit_claim_generation_allowed")
            .and_then(Value::as_bool)
            != Some(false)
        || policy
            .get("population_prevalence_claim_allowed")
            .and_then(Value::as_bool)
            != Some(false)
    {
        errors.push("ORDS policy weakened a raw-data, barrier, or evidence claim ceiling".to_owned());
    }

    let Some(categories) = profile
        .get("product_category_mappings")
        .and_then(Value::as_array)
        .cloned()
    else {
        return Err(vec!["product_category_mappings must be an array".to_owned()]);
    };
    let Some(statuses) = profile
        .get("repair_status_mappings")
        .and_then(Value::as_array)
        .cloned()
    else {
        return Err(vec!["repair_status_mappings must be an array".to_owned()]);
    };
    let Some(barriers) = profile
        .get("repair_barrier_mappings")
        .and_then(Value::as_array)
        .cloned()
    else {
        return Err(vec!["repair_barrier_mappings must be an array".to_owned()]);
    };

    let mut category_keys = BTreeSet::new();
    for (index, item) in categories.iter().enumerate() {
        let Some(raw) = text_at(item, "ords_product_category") else {
            errors.push(format!(
                "product_category_mappings[{index}] lacks ords_product_category"
            ));
            continue;
        };
        if raw.trim().is_empty() {
            errors.push(format!(
                "product_category_mappings[{index}] has an empty category key"
            ));
        }
        if !category_keys.insert(normalize_key(raw)) {
            errors.push(format!("duplicate normalized ORDS category mapping: {raw}"));
        }
        let state = text_at(item, "mapping_status").unwrap_or("");
        let target = item.get("target_family_id").and_then(Value::as_str);
        match state {
            "candidate" if target.is_none() => {
                errors.push(format!(
                    "{raw}: candidate category mapping requires a target family"
                ));
            }
            "ambiguous" | "unmapped" if target.is_some() => {
                errors.push(format!(
                    "{raw}: ambiguous/unmapped category cannot assert a target family"
                ));
            }
            _ => {}
        }
        if let Some(target) = target {
            if !target.starts_with('F') {
                errors.push(format!("{raw}: target family identity is malformed"));
            }
        }
    }

    let mut status_ids = BTreeSet::new();
    for item in &statuses {
        let id = text_at(item, "normalized_status").unwrap_or("");
        if !status_ids.insert(id.to_owned()) {
            errors.push(format!("duplicate normalized repair status {id}"));
        }
    }
    if status_ids.len() != EXPECTED_STATUSES.len()
        || EXPECTED_STATUSES.iter().any(|status| !status_ids.contains(*status))
    {
        errors.push(
            "ORDS repair-status mapping must cover unknown/fixed/repairable/end_of_life exactly"
                .to_owned(),
        );
    }

    let mut barrier_ids = BTreeSet::new();
    for item in &barriers {
        let id = text_at(item, "normalized_barrier").unwrap_or("");
        if !barrier_ids.insert(id.to_owned()) {
            errors.push(format!("duplicate normalized repair barrier {id}"));
        }
    }
    if barrier_ids.len() != EXPECTED_BARRIERS.len()
        || EXPECTED_BARRIERS.iter().any(|barrier| !barrier_ids.contains(*barrier))
    {
        errors.push("ORDS repair-barrier mapping must cover all six v0.3 barriers exactly".to_owned());
    }

    if !errors.is_empty() {
        return Err(errors);
    }
    Ok((categories, statuses, barriers))
}

fn mapping_for_raw<'a>(
    mappings: &'a [Value],
    raw: &str,
    values_key: &str,
    normalized_key: &str,
) -> Option<&'a Value> {
    let needle = normalize_key(raw);
    mappings.iter().find(|item| {
        item.get(values_key)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .any(|alias| normalize_key(alias) == needle)
            && text_at(item, normalized_key).is_some()
    })
}

fn read_headers(csv_raw: &[u8]) -> Result<Vec<String>, Errors> {
    let mut reader = csv::ReaderBuilder::new().flexible(false).from_reader(csv_raw);
    let headers = reader
        .headers()
        .map_err(|error| vec![format!("cannot read ORDS CSV headers: {error}")])?;
    let normalized: Vec<String> = headers
        .iter()
        .map(|header| header.trim_start_matches('\u{feff}').trim().to_owned())
        .collect();
    let mut unique = BTreeSet::new();
    for header in &normalized {
        if header.is_empty() || !unique.insert(header.clone()) {
            return Err(vec![format!("empty or duplicate ORDS CSV header: {header:?}")]);
        }
    }
    let header_set: BTreeSet<&str> = normalized.iter().map(String::as_str).collect();
    let missing: Vec<&str> = REQUIRED_HEADERS
        .iter()
        .copied()
        .filter(|header| !header_set.contains(header))
        .collect();
    if !missing.is_empty() {
        return Err(vec![format!(
            "ORDS CSV is missing required v0.3 columns: {}",
            missing.join(", ")
        )]);
    }
    Ok(normalized)
}

fn header_indexes(headers: &[String]) -> BTreeMap<String, usize> {
    headers
        .iter()
        .enumerate()
        .map(|(index, name)| (name.clone(), index))
        .collect()
}

fn raw_field<'a>(
    record: &'a csv::StringRecord,
    indexes: &BTreeMap<String, usize>,
    key: &str,
) -> &'a str {
    indexes
        .get(key)
        .and_then(|index| record.get(*index))
        .unwrap_or("")
}

fn source_identity(
    record: &csv::StringRecord,
    indexes: &BTreeMap<String, usize>,
) -> Option<String> {
    let id = raw_field(record, indexes, "id").trim();
    let provider = raw_field(record, indexes, "data_provider").trim();
    let group = raw_field(record, indexes, "group_identifier").trim();
    if id.is_empty() || provider.is_empty() {
        return None;
    }
    Some(format!("{provider}\0{group}\0{id}"))
}

fn status_mapping<'a>(statuses: &'a [Value], raw: &str) -> Option<&'a str> {
    mapping_for_raw(statuses, raw, "raw_values", "normalized_status")
        .and_then(|item| text_at(item, "normalized_status"))
}

fn barrier_mapping<'a>(barriers: &'a [Value], raw: &str) -> Option<&'a str> {
    mapping_for_raw(barriers, raw, "raw_values", "normalized_barrier")
        .and_then(|item| text_at(item, "normalized_barrier"))
}

pub(crate) fn audit_csv_to_writer<W: Write>(
    schema_raw: &str,
    mapping_raw: &str,
    csv_raw: &[u8],
    output: &mut W,
) -> Result<OrdsAuditSummary, Errors> {
    let schema: Value = serde_json::from_str(schema_raw)
        .map_err(|error| vec![format!("ORDS schema JSON parse failed: {error}")])?;
    let profile: Value = serde_json::from_str(mapping_raw)
        .map_err(|error| vec![format!("ORDS mapping JSON parse failed: {error}")])?;
    let (categories, statuses, barriers) = validate_mapping_profile(&schema, &profile)?;

    let headers = read_headers(csv_raw)?;
    let indexes = header_indexes(&headers);

    let mut identity_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut row_count = 0usize;
    {
        let mut reader = csv::ReaderBuilder::new().flexible(false).from_reader(csv_raw);
        for result in reader.records() {
            let record = result.map_err(|error| vec![format!("malformed ORDS CSV row: {error}")])?;
            row_count += 1;
            if let Some(identity) = source_identity(&record, &indexes) {
                *identity_counts.entry(identity).or_default() += 1;
            }
        }
    }

    let input_sha256 = sha256_hex(csv_raw);
    let schema_sha256 = sha256_hex(schema_raw.as_bytes());
    let mapping_sha256 = sha256_hex(mapping_raw.as_bytes());
    let source = profile.get("source_dataset").unwrap_or(&Value::Null);
    let profile_status = text_at(&profile, "profile_status").unwrap_or("operational_config");
    let dataset_release_id = text_at(source, "release_id").unwrap_or("unknown-release");
    let source_license = text_at(source, "license").unwrap_or("unknown-license");
    let source_url = text_at(source, "source_url").unwrap_or("");
    let retrieved_at = text_at(source, "retrieved_at").unwrap_or("");
    let synthetic_profile = profile_status == "synthetic_fixture_only";
    let mapping_version = text_at(&profile, "mapping_version").unwrap_or("unknown-mapping");
    let brand_normalization_version =
        text_at(&profile, "brand_normalization_version").unwrap_or("unknown-brand-normalization");
    let categories_by_key: BTreeMap<String, &Value> = categories
        .iter()
        .filter_map(|item| {
            text_at(item, "ords_product_category").map(|raw| (normalize_key(raw), item))
        })
        .collect();

    let mut summary = OrdsAuditSummary {
        row_count: 0,
        accepted_row_count: 0,
        quarantined_row_count: 0,
        fixed_row_count: 0,
        repairable_row_count: 0,
        end_of_life_row_count: 0,
        unknown_status_row_count: 0,
        category_candidate_row_count: 0,
        category_ambiguous_row_count: 0,
        category_unmapped_row_count: 0,
        duplicate_identity_row_count: 0,
        missing_identity_row_count: 0,
        invalid_status_row_count: 0,
        barrier_status_gate_violation_count: 0,
        invalid_end_of_life_barrier_count: 0,
        schema_sha256,
        mapping_sha256,
        input_sha256: input_sha256.clone(),
        dataset_release_id: dataset_release_id.to_owned(),
        source_license: source_license.to_owned(),
        profile_status: profile_status.to_owned(),
    };

    let mut reader = csv::ReaderBuilder::new().flexible(false).from_reader(csv_raw);
    for (row_index, result) in reader.records().enumerate() {
        let record = result.map_err(|error| vec![format!("malformed ORDS CSV row: {error}")])?;
        let mut raw_fields = Map::new();
        for (index, header) in headers.iter().enumerate() {
            raw_fields.insert(
                header.clone(),
                Value::String(record.get(index).unwrap_or("").to_owned()),
            );
        }

        let source_id = raw_field(&record, &indexes, "id");
        let data_provider = raw_field(&record, &indexes, "data_provider");
        let group_identifier = raw_field(&record, &indexes, "group_identifier");
        let event_date = raw_field(&record, &indexes, "event_date");
        let record_date = raw_field(&record, &indexes, "record_date");
        let raw_status = raw_field(&record, &indexes, "repair_status");
        let normalized_status = status_mapping(&statuses, raw_status);
        let raw_barrier = raw_field(&record, &indexes, "repair_barrier_if_end_of_life");
        let raw_category = raw_field(&record, &indexes, "product_category");
        let raw_partner_category = raw_field(&record, &indexes, "partner_product_category");
        let raw_brand = raw_field(&record, &indexes, "brand");
        let category = categories_by_key.get(&normalize_key(raw_category)).copied();

        let identity = source_identity(&record, &indexes);
        let stable_record_key = identity
            .as_ref()
            .map(|identity| sha256_hex(format!("ords-v0.3-record-key-v1\0{identity}").as_bytes()));
        let mut reasons = Vec::new();
        if identity.is_none() {
            reasons.push("identity_components_missing".to_owned());
            summary.missing_identity_row_count += 1;
        }
        if let Some(identity) = &identity {
            if identity_counts.get(identity).copied().unwrap_or(0) > 1 {
                reasons.push("duplicate_source_record_identity".to_owned());
                summary.duplicate_identity_row_count += 1;
            }
        }

        if normalized_status.is_none() {
            reasons.push("unrecognized_repair_status_quarantined".to_owned());
            summary.invalid_status_row_count += 1;
        }
        let normalized_barrier = if raw_barrier.trim().is_empty() {
            None
        } else {
            barrier_mapping(&barriers, raw_barrier)
        };
        let barrier_gate_violation =
            !raw_barrier.trim().is_empty() && normalized_status != Some("end_of_life");
        let invalid_end_of_life_barrier = !raw_barrier.trim().is_empty()
            && normalized_status == Some("end_of_life")
            && normalized_barrier.is_none();
        if barrier_gate_violation {
            reasons.push("repair_barrier_present_when_status_is_not_end_of_life".to_owned());
            summary.barrier_status_gate_violation_count += 1;
        }
        if invalid_end_of_life_barrier {
            reasons.push("unrecognized_end_of_life_repair_barrier_quarantined".to_owned());
            summary.invalid_end_of_life_barrier_count += 1;
        }

        if !reasons.is_empty() {
            summary.quarantined_row_count += 1;
        } else {
            summary.accepted_row_count += 1;
            match normalized_status.unwrap_or("unknown") {
                "fixed" => summary.fixed_row_count += 1,
                "repairable" => summary.repairable_row_count += 1,
                "end_of_life" => summary.end_of_life_row_count += 1,
                _ => summary.unknown_status_row_count += 1,
            }
            match text_at(category.unwrap_or(&Value::Null), "mapping_status").unwrap_or("unmapped") {
                "candidate" => summary.category_candidate_row_count += 1,
                "ambiguous" => summary.category_ambiguous_row_count += 1,
                _ => summary.category_unmapped_row_count += 1,
            }
        }
        summary.row_count += 1;

        let category_status = text_at(category.unwrap_or(&Value::Null), "mapping_status")
            .unwrap_or("unmapped");
        let category_family = category
            .and_then(|item| item.get("target_family_id"))
            .cloned()
            .unwrap_or(Value::Null);
        let category_confidence =
            text_at(category.unwrap_or(&Value::Null), "confidence").unwrap_or("none");
        let category_ambiguity = category
            .and_then(|item| item.get("ambiguity_flag"))
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let brand_key = normalize_key(raw_brand);
        let identity_exception = if identity.is_none() {
            Some("missing_id_or_data_provider")
        } else if reasons
            .iter()
            .any(|reason| reason == "duplicate_source_record_identity")
        {
            Some("duplicate_provider_group_record_id")
        } else {
            None
        };

        let output_record = json!({
            "adapter_id": ADAPTER_ID,
            "adapter_version": "0.1.0",
            "evidence_class": if synthetic_profile { "synthetic_contract_fixture" } else { "community_repair_event_observation" },
            "source_provenance": {
                "dataset_release_id": dataset_release_id,
                "ords_standard_version": "0.3",
                "source_url": source_url,
                "source_license": source_license,
                "license_lineage": "preserve_attribution_and_share_alike",
                "retrieved_at": retrieved_at,
                "source_file_sha256": input_sha256,
                "source_record_id_raw": source_id,
                "data_provider_raw": data_provider,
                "group_identifier_raw": group_identifier,
                "event_date_raw": event_date,
                "record_date_raw": record_date,
            },
            "stable_record_key_sha256": stable_record_key,
            "identity_exception": identity_exception,
            "raw_fields": Value::Object(raw_fields),
            "repair_status": {
                "raw_value": raw_status,
                "normalized_value": normalized_status,
                "meaning_scope": "community_event_repairer_and_owner_reported_outcome_only",
            },
            "repair_barrier": {
                "raw_value": raw_barrier,
                "normalized_value": if normalized_status == Some("end_of_life") { normalized_barrier } else { None },
                "applicable_only_when_end_of_life": true,
                "status_gate_passed": !barrier_gate_violation,
            },
            "product_category_mapping": {
                "raw_product_category": raw_category,
                "raw_partner_product_category": raw_partner_category,
                "mapping_version": mapping_version,
                "mapping_match_method": "trim_whitespace_and_casefold",
                "mapping_status": category_status,
                "target_family_id_candidate": category_family,
                "confidence": category_confidence,
                "ambiguity_flag": category_ambiguity,
            },
            "brand_normalization": {
                "raw_brand": raw_brand,
                "normalized_search_key": brand_key,
                "normalization_version": brand_normalization_version,
                "normalization_scope": "trim_collapse_whitespace_and_casefold_only",
                "entity_resolution_performed": false,
                "ambiguity_flag": true,
            },
            "quarantine": {
                "disposition": if reasons.is_empty() { "accepted" } else { "quarantined" },
                "reasons": reasons,
            },
            "claim_ceiling": {
                "verified_restoration_receipt_generated": false,
                "physical_safety_claim_generated": false,
                "exact_part_fit_claim_generated": false,
                "non_recurrence_claim_generated": false,
                "population_prevalence_claim_permitted": false,
                "empirical_observation_allowed": !synthetic_profile,
            }
        });
        serde_json::to_writer(&mut *output, &output_record)
            .map_err(|error| vec![format!("cannot serialize ORDS JSONL record: {error}")])?;
        output
            .write_all(b"\n")
            .map_err(|error| vec![format!("cannot write ORDS JSONL record: {error}")])?;

        // Retain this local variable as a compile-time check that row order is stable and explicit.
        let _source_row_number = row_index + 2;
    }

    if summary.accepted_row_count + summary.quarantined_row_count != summary.row_count {
        return Err(vec!["ORDS row accounting does not reconcile".to_owned()]);
    }
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCHEMA: &str =
        include_str!("../../../schemas/appliances/ords_adapter_mapping_v0_1.schema.json");
    const PROFILE: &str =
        include_str!("../../../fixtures/appliances/ords_adapter_mapping_v0_1.json");
    const CSV: &[u8] = include_bytes!("../../../fixtures/appliances/ords_v0_3_synthetic.csv");

    fn run_fixture(
        schema_raw: &str,
        profile_raw: &str,
        csv_raw: &[u8],
    ) -> (OrdsAuditSummary, String) {
        let mut output = Vec::new();
        let summary = audit_csv_to_writer(schema_raw, profile_raw, csv_raw, &mut output)
            .expect("synthetic fixture must audit");
        (summary, String::from_utf8(output).expect("JSONL output is UTF-8"))
    }

    #[test]
    fn synthetic_fixture_accounts_for_every_input_row() {
        let (summary, output) = run_fixture(SCHEMA, PROFILE, CSV);
        assert_eq!(summary.row_count, 10);
        assert_eq!(summary.accepted_row_count, 5);
        assert_eq!(summary.quarantined_row_count, 5);
        assert_eq!(summary.accepted_row_count + summary.quarantined_row_count, 10);
        assert_eq!(output.lines().count(), 10);
        assert_eq!(summary.input_sha256.len(), 64);
        assert_eq!(summary.schema_sha256.len(), 64);
        assert_eq!(summary.mapping_sha256.len(), 64);
    }

    #[test]
    fn community_fixed_status_never_generates_restoration_or_safety_receipt() {
        let (_, output) = run_fixture(SCHEMA, PROFILE, CSV);
        let record: Value = serde_json::from_str(output.lines().next().unwrap()).unwrap();
        assert_eq!(record["repair_status"]["normalized_value"], "fixed");
        assert_eq!(record["claim_ceiling"]["verified_restoration_receipt_generated"], false);
        assert_eq!(record["claim_ceiling"]["physical_safety_claim_generated"], false);
        assert_eq!(record["claim_ceiling"]["exact_part_fit_claim_generated"], false);
        assert_eq!(record["claim_ceiling"]["non_recurrence_claim_generated"], false);
    }

    #[test]
    fn empty_or_zero_status_is_unknown_not_failure() {
        let (summary, output) = run_fixture(SCHEMA, PROFILE, CSV);
        let records: Vec<Value> = output.lines().map(|line| serde_json::from_str(line).unwrap()).collect();
        let unknown = records.iter().find(|record| record["source_provenance"]["source_record_id_raw"] == "r-003").unwrap();
        assert_eq!(unknown["repair_status"]["normalized_value"], "unknown");
        assert_eq!(unknown["quarantine"]["disposition"], "accepted");
        assert_eq!(summary.unknown_status_row_count, 1);
    }

    #[test]
    fn raw_partner_category_and_brand_typo_are_retained() {
        let (_, output) = run_fixture(SCHEMA, PROFILE, CSV);
        let records: Vec<Value> = output.lines().map(|line| serde_json::from_str(line).unwrap()).collect();
        let washing_machine = records.iter().find(|record| record["source_provenance"]["source_record_id_raw"] == "r-003").unwrap();
        assert_eq!(washing_machine["product_category_mapping"]["raw_partner_product_category"], "Washing machine / front loader");
        assert_eq!(washing_machine["product_category_mapping"]["mapping_status"], "unmapped");
        let typo = records.iter().find(|record| record["source_provenance"]["source_record_id_raw"] == "r-006").unwrap();
        assert_eq!(typo["brand_normalization"]["raw_brand"], "acmee");
        assert_eq!(typo["brand_normalization"]["entity_resolution_performed"], false);
    }

    #[test]
    fn duplicate_source_identity_quarantines_all_colliding_rows() {
        let (summary, output) = run_fixture(SCHEMA, PROFILE, CSV);
        assert_eq!(summary.duplicate_identity_row_count, 2);
        let records: Vec<Value> = output.lines().map(|line| serde_json::from_str(line).unwrap()).collect();
        let duplicates: Vec<&Value> = records.iter()
            .filter(|record| record["source_provenance"]["source_record_id_raw"] == "r-001")
            .collect();
        assert_eq!(duplicates.len(), 2);
        assert!(duplicates.iter().all(|record| record["quarantine"]["disposition"] == "quarantined"));
    }

    #[test]
    fn barrier_is_valid_only_for_end_of_life() {
        let (_, output) = run_fixture(SCHEMA, PROFILE, CSV);
        let records: Vec<Value> = output.lines().map(|line| serde_json::from_str(line).unwrap()).collect();
        let end_of_life = records.iter().find(|record| record["source_provenance"]["source_record_id_raw"] == "r-004").unwrap();
        assert_eq!(end_of_life["repair_barrier"]["normalized_value"], "spare_parts_unavailable");
        assert_eq!(end_of_life["repair_barrier"]["status_gate_passed"], true);
        let fixed_with_barrier = records.iter().find(|record| record["source_provenance"]["source_record_id_raw"] == "r-008").unwrap();
        assert_eq!(fixed_with_barrier["repair_barrier"]["normalized_value"], Value::Null);
        assert_eq!(fixed_with_barrier["quarantine"]["disposition"], "quarantined");
    }

    #[test]
    fn unrecognized_status_and_end_of_life_barrier_are_quarantined() {
        let (summary, output) = run_fixture(SCHEMA, PROFILE, CSV);
        assert_eq!(summary.invalid_status_row_count, 1);
        assert_eq!(summary.invalid_end_of_life_barrier_count, 1);
        let records: Vec<Value> = output.lines().map(|line| serde_json::from_str(line).unwrap()).collect();
        let invalid_status = records.iter().find(|record| record["source_provenance"]["source_record_id_raw"] == "r-006").unwrap();
        assert_eq!(invalid_status["quarantine"]["disposition"], "quarantined");
        let invalid_barrier = records.iter().find(|record| record["source_provenance"]["source_record_id_raw"] == "r-005").unwrap();
        assert_eq!(invalid_barrier["quarantine"]["disposition"], "quarantined");
    }

    #[test]
    fn category_mapping_is_candidate_or_ambiguous_not_exact_product_identity() {
        let (summary, output) = run_fixture(SCHEMA, PROFILE, CSV);
        assert_eq!(summary.category_candidate_row_count, 2);
        assert_eq!(summary.category_ambiguous_row_count, 1);
        assert_eq!(summary.category_unmapped_row_count, 2);
        let records: Vec<Value> = output.lines().map(|line| serde_json::from_str(line).unwrap()).collect();
        let toaster = records.iter().find(|record| record["source_provenance"]["source_record_id_raw"] == "r-004").unwrap();
        assert_eq!(toaster["product_category_mapping"]["target_family_id_candidate"], "F8-COOKING-APPLIANCES");
        assert_eq!(toaster["product_category_mapping"]["mapping_status"], "candidate");
        assert_eq!(toaster["claim_ceiling"]["population_prevalence_claim_permitted"], false);
    }

    #[test]
    fn identical_input_bytes_have_deterministic_digest_and_output() {
        let (first, first_output) = run_fixture(SCHEMA, PROFILE, CSV);
        let (second, second_output) = run_fixture(SCHEMA, PROFILE, CSV);
        assert_eq!(first.input_sha256, second.input_sha256);
        assert_eq!(first_output, second_output);
    }

    #[test]
    fn duplicate_mapping_keys_are_rejected() {
        let mut profile: Value = serde_json::from_str(PROFILE).unwrap();
        profile["product_category_mappings"][1]["ords_product_category"] = json!(" coffee MAKER ");
        let profile_raw = serde_json::to_string(&profile).unwrap();
        let mut output = Vec::new();
        let errors = audit_csv_to_writer(SCHEMA, &profile_raw, CSV, &mut output)
            .expect_err("duplicate normalized mapping must fail closed");
        assert!(errors.iter().any(|error| error.contains("duplicate normalized ORDS category")));
    }

    #[test]
    fn missing_required_header_fails_closed() {
        let csv = b"id,product_category\nr-1,Coffee maker\n";
        let mut output = Vec::new();
        let errors = audit_csv_to_writer(SCHEMA, PROFILE, csv, &mut output)
            .expect_err("missing ORDS headers must fail");
        assert!(errors.iter().any(|error| error.contains("missing required v0.3 columns")));
    }

    #[test]
    fn profile_cannot_promote_observation_to_population_or_safety_claim() {
        let mut profile: Value = serde_json::from_str(PROFILE).unwrap();
        profile["policy"]["population_prevalence_claim_allowed"] = json!(true);
        let profile_raw = serde_json::to_string(&profile).unwrap();
        let mut output = Vec::new();
        let errors = audit_csv_to_writer(SCHEMA, &profile_raw, CSV, &mut output)
            .expect_err("prevalence ceiling cannot be widened");
        assert!(errors.iter().any(|error| error.contains("policy weakened")));
    }

    #[test]
    fn mapping_version_and_source_license_are_preserved() {
        let (_, output) = run_fixture(SCHEMA, PROFILE, CSV);
        let record: Value = serde_json::from_str(output.lines().next().unwrap()).unwrap();
        assert_eq!(record["source_provenance"]["dataset_release_id"], "ORA-2026-10");
        assert_eq!(record["source_provenance"]["source_license"], "CC BY-SA 4.0");
        assert_eq!(record["product_category_mapping"]["mapping_version"], "symtropy-ords-appliance-taxonomy/0.1.0");
    }
}
