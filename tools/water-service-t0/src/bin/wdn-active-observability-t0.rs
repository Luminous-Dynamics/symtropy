// SPDX-License-Identifier: AGPL-3.0-or-later
//! Synthetic-only counterfactual active-observability evaluator.
//! Read-only: it emits no valve, pump, SCADA or actuator command.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::Path;

const INPUT_VERSION: &str = "wdn-active-observability-t0-input-v0.1";
const ORACLE_VERSION: &str = "wdn-active-observability-t0-oracle-v0.1";
const BENCHMARK_ID: &str = "WDN-ACTIVE-OBSERVABILITY-T0-001";
const CASE_COUNT: usize = 14;
const MAX_HYPOTHESES: usize = 8;
const MAX_PROBES: usize = 4;
const MAX_SEQUENCE_LENGTH: usize = 2;

type FeatureMap = BTreeMap<String, BTreeMap<String, (f64, f64)>>;

#[derive(Clone, Debug)]
struct Probe {
    id: String,
    kind: String,
    duration_seconds: u64,
    reasons: Vec<String>,
    observations: FeatureMap,
}

#[derive(Clone, Debug)]
struct OptionChoice {
    sequence: Vec<String>,
    post_max_clique: usize,
    duration_seconds: u64,
}

fn field<'a>(value: &'a Value, key: &str, context: &str) -> Result<&'a Value, String> {
    value
        .get(key)
        .ok_or_else(|| format!("{context}: missing '{key}'"))
}

fn string<'a>(value: &'a Value, key: &str, context: &str) -> Result<&'a str, String> {
    field(value, key, context)?
        .as_str()
        .ok_or_else(|| format!("{context}: '{key}' must be a string"))
}

fn number(value: &Value, key: &str, context: &str) -> Result<f64, String> {
    let result = field(value, key, context)?
        .as_f64()
        .ok_or_else(|| format!("{context}: '{key}' must be numeric"))?;
    if !result.is_finite() {
        return Err(format!("{context}: '{key}' must be finite"));
    }
    Ok(result)
}

fn boolean(value: &Value, key: &str, context: &str) -> Result<bool, String> {
    field(value, key, context)?
        .as_bool()
        .ok_or_else(|| format!("{context}: '{key}' must be boolean"))
}

fn ensure_keys(value: &Value, expected: &[&str], context: &str) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{context}: must be an object"))?;
    let actual: BTreeSet<&str> = object.keys().map(String::as_str).collect();
    let wanted: BTreeSet<&str> = expected.iter().copied().collect();
    if actual != wanted {
        return Err(format!("{context}: key set differs from frozen v0.1 contract"));
    }
    Ok(())
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn valid_revision(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn parse_hypotheses(value: &Value, context: &str) -> Result<Vec<String>, String> {
    let rows = value
        .as_array()
        .ok_or_else(|| format!("{context}: hypotheses must be an array"))?;
    if rows.len() < 2 || rows.len() > MAX_HYPOTHESES {
        return Err(format!(
            "{context}: hypothesis count must be between 2 and {MAX_HYPOTHESES}"
        ));
    }
    let mut ids = BTreeSet::new();
    for (index, row) in rows.iter().enumerate() {
        let row_context = format!("{context}[{index}]");
        ensure_keys(row, &["hypothesis_id", "ground_truth_ref"], &row_context)?;
        let id = string(row, "hypothesis_id", &row_context)?;
        if id.trim().is_empty() || string(row, "ground_truth_ref", &row_context)?.trim().is_empty() {
            return Err(format!("{row_context}: hypothesis ID and reference must be non-empty"));
        }
        if !ids.insert(id.to_owned()) {
            return Err(format!("{context}: duplicate hypothesis_id '{id}'"));
        }
    }
    Ok(ids.into_iter().collect())
}

fn parse_observations(
    value: &Value,
    hypothesis_ids: &[String],
    context: &str,
) -> Result<FeatureMap, String> {
    let rows = value
        .as_array()
        .ok_or_else(|| format!("{context}: observations must be an array"))?;
    if rows.len() != hypothesis_ids.len() {
        return Err(format!("{context}: one observation row per hypothesis is required"));
    }
    let mut output = BTreeMap::new();
    let mut expected_features: Option<BTreeSet<String>> = None;
    for (index, row) in rows.iter().enumerate() {
        let row_context = format!("{context}[{index}]");
        ensure_keys(row, &["hypothesis_id", "features"], &row_context)?;
        let hypothesis_id = string(row, "hypothesis_id", &row_context)?.to_owned();
        if !hypothesis_ids.contains(&hypothesis_id) {
            return Err(format!("{row_context}: unknown hypothesis '{hypothesis_id}'"));
        }
        let raw_features = field(row, "features", &row_context)?
            .as_array()
            .ok_or_else(|| format!("{row_context}: features must be an array"))?;
        if raw_features.is_empty() || raw_features.len() > 8 {
            return Err(format!("{row_context}: feature count must be 1..8"));
        }
        let mut features = BTreeMap::new();
        for (feature_index, raw) in raw_features.iter().enumerate() {
            let feature_context = format!("{row_context}.features[{feature_index}]");
            ensure_keys(raw, &["feature_id", "lower", "upper"], &feature_context)?;
            let feature_id = string(raw, "feature_id", &feature_context)?.to_owned();
            let lower = number(raw, "lower", &feature_context)?;
            let upper = number(raw, "upper", &feature_context)?;
            if feature_id.trim().is_empty() || lower > upper {
                return Err(format!("{feature_context}: invalid feature ID or reversed interval"));
            }
            if features.insert(feature_id.clone(), (lower, upper)).is_some() {
                return Err(format!("{row_context}: duplicate feature_id '{feature_id}'"));
            }
        }
        let feature_ids: BTreeSet<String> = features.keys().cloned().collect();
        if let Some(expected) = &expected_features {
            if &feature_ids != expected {
                return Err(format!("{context}: feature IDs must be identical for every hypothesis"));
            }
        } else {
            expected_features = Some(feature_ids);
        }
        if output.insert(hypothesis_id.clone(), features).is_some() {
            return Err(format!("{context}: duplicate observation for '{hypothesis_id}'"));
        }
    }
    if output.keys().cloned().collect::<BTreeSet<_>>()
        != hypothesis_ids.iter().cloned().collect::<BTreeSet<_>>()
    {
        return Err(format!("{context}: hypothesis observation set is incomplete"));
    }
    Ok(output)
}

fn parse_feasibility(
    value: &Value,
    probe_kind: &str,
    context: &str,
) -> Result<Vec<String>, String> {
    ensure_keys(
        value,
        &[
            "hydraulic_model_current",
            "topology_current",
            "operating_window_valid",
            "telemetry_time_valid",
            "calibration_current",
            "pressure_floor_margin_kpa",
            "required_pressure_margin_kpa",
            "fire_flow_margin_lps",
            "required_fire_flow_margin_lps",
            "maximum_transient_kpa",
            "allowed_maximum_transient_kpa",
            "quality_hazard_clear",
            "isolation_conflict_clear",
            "reversible_rollback_defined",
            "operator_authorized",
            "permit_approved",
            "action_timeout_clear",
            "valve_identity_verified",
            "valve_position_verified",
        ],
        context,
    )?;
    let mut reasons = Vec::new();
    let bool_checks = [
        ("hydraulic_model_current", "hydraulic_model_stale"),
        ("topology_current", "topology_stale"),
        ("operating_window_valid", "operating_window_invalid"),
        ("telemetry_time_valid", "telemetry_time_invalid"),
        ("calibration_current", "calibration_stale"),
        ("quality_hazard_clear", "water_quality_hazard"),
        ("isolation_conflict_clear", "isolation_conflict"),
        ("reversible_rollback_defined", "rollback_not_defined"),
        ("operator_authorized", "operator_authorization_missing"),
        ("permit_approved", "permit_missing"),
        ("action_timeout_clear", "action_lockout_active"),
    ];
    for (key, reason) in bool_checks {
        if !boolean(value, key, context)? {
            reasons.push(reason.to_owned());
        }
    }

    let pressure_margin = number(value, "pressure_floor_margin_kpa", context)?;
    let required_pressure = number(value, "required_pressure_margin_kpa", context)?;
    let fire_margin = number(value, "fire_flow_margin_lps", context)?;
    let required_fire = number(value, "required_fire_flow_margin_lps", context)?;
    let max_transient = number(value, "maximum_transient_kpa", context)?;
    let allowed_transient = number(value, "allowed_maximum_transient_kpa", context)?;
    if required_pressure < 0.0 || required_fire < 0.0 || allowed_transient < 0.0 {
        return Err(format!("{context}: safety thresholds must be non-negative"));
    }
    if pressure_margin < required_pressure {
        reasons.push("pressure_floor_margin_insufficient".to_owned());
    }
    if fire_margin < required_fire {
        reasons.push("fire_flow_reserve_insufficient".to_owned());
    }
    if max_transient > allowed_transient {
        reasons.push("maximum_transient_exceeded".to_owned());
    }

    let valve_identity = field(value, "valve_identity_verified", context)?;
    let valve_position = field(value, "valve_position_verified", context)?;
    if probe_kind == "simulated_valve_perturbation" {
        if valve_identity.as_bool() != Some(true) {
            reasons.push("valve_identity_unverified".to_owned());
        }
        if valve_position.as_bool() != Some(true) {
            reasons.push("valve_position_unverified".to_owned());
        }
    } else if !valve_identity.is_null() || !valve_position.is_null() {
        return Err(format!(
            "{context}: non-valve probe must not carry valve verification fields"
        ));
    }
    reasons.sort();
    reasons.dedup();
    Ok(reasons)
}

fn parse_probe(
    value: &Value,
    hypothesis_ids: &[String],
    context: &str,
) -> Result<Probe, String> {
    ensure_keys(
        value,
        &[
            "probe_id",
            "kind",
            "estimated_duration_seconds",
            "feasibility",
            "observations",
        ],
        context,
    )?;
    let id = string(value, "probe_id", context)?.to_owned();
    if id.trim().is_empty() {
        return Err(format!("{context}: probe_id is empty"));
    }
    let kind = string(value, "kind", context)?.to_owned();
    if ![
        "temporary_sensor",
        "operator_observation",
        "simulated_valve_perturbation",
    ]
    .contains(&kind.as_str())
    {
        return Err(format!("{context}: unsupported probe kind '{kind}'"));
    }
    let duration = field(value, "estimated_duration_seconds", context)?
        .as_u64()
        .filter(|value| *value > 0)
        .ok_or_else(|| format!("{context}: estimated_duration_seconds must be positive"))?;
    let reasons = parse_feasibility(
        field(value, "feasibility", context)?,
        &kind,
        &format!("{context}.feasibility"),
    )?;
    let observations = parse_observations(
        field(value, "observations", context)?,
        hypothesis_ids,
        &format!("{context}.observations"),
    )?;
    Ok(Probe {
        id,
        kind,
        duration_seconds: duration,
        reasons,
        observations,
    })
}

fn intervals_overlap(left: (f64, f64), right: (f64, f64)) -> bool {
    left.0.max(right.0) <= left.1.min(right.1)
}

fn maximum_cliques(
    hypothesis_ids: &[String],
    observations: &FeatureMap,
) -> Result<(usize, Vec<Vec<String>>, Vec<Vec<String>>), String> {
    let ids: Vec<String> = hypothesis_ids.to_vec();
    if ids.len() > MAX_HYPOTHESES || ids.len() < 2 {
        return Err("maximum-clique input must contain 2..8 hypotheses".to_owned());
    }
    let mut adjacency = vec![vec![false; ids.len()]; ids.len()];
    let mut edges = Vec::new();
    for left in 0..ids.len() {
        for right in (left + 1)..ids.len() {
            let a = observations
                .get(&ids[left])
                .ok_or_else(|| format!("missing features for '{}'", ids[left]))?;
            let b = observations
                .get(&ids[right])
                .ok_or_else(|| format!("missing features for '{}'", ids[right]))?;
            if a.len() != b.len() || a.is_empty() {
                return Err("hypothesis feature maps must have equal non-zero dimensions".to_owned());
            }
            let indistinguishable = a.iter().all(|(feature_id, interval_a)| {
                b.get(feature_id)
                    .is_some_and(|interval_b| intervals_overlap(*interval_a, *interval_b))
            });
            adjacency[left][right] = indistinguishable;
            adjacency[right][left] = indistinguishable;
            if indistinguishable {
                edges.push(vec![ids[left].clone(), ids[right].clone()]);
            }
        }
    }

    let mut best = 0usize;
    let mut maximum = Vec::new();
    let total_masks = 1usize << ids.len();
    for mask in 1usize..total_masks {
        let members: Vec<usize> = (0..ids.len())
            .filter(|index| mask & (1usize << index) != 0)
            .collect();
        if members.len() < best {
            continue;
        }
        let mut clique = true;
        'pair_check: for left in 0..members.len() {
            for right in (left + 1)..members.len() {
                if !adjacency[members[left]][members[right]] {
                    clique = false;
                    break 'pair_check;
                }
            }
        }
        if clique {
            if members.len() > best {
                best = members.len();
                maximum.clear();
            }
            if members.len() == best {
                maximum.push(
                    members
                        .iter()
                        .map(|index| ids[*index].clone())
                        .collect::<Vec<_>>(),
                );
            }
        }
    }
    maximum.sort();
    edges.sort();
    Ok((best, maximum, edges))
}

fn combine_observations(
    passive: &FeatureMap,
    probes: &[&Probe],
    hypothesis_ids: &[String],
) -> Result<FeatureMap, String> {
    let mut combined = passive.clone();
    for id in hypothesis_ids {
        let row = combined
            .get_mut(id)
            .ok_or_else(|| format!("missing passive observation for '{id}'"))?;
        for probe in probes {
            let extra = probe
                .observations
                .get(id)
                .ok_or_else(|| format!("probe '{}' missing observation for '{id}'", probe.id))?;
            for (feature_id, interval) in extra {
                let namespaced = format!("{}:{feature_id}", probe.id);
                if row.insert(namespaced.clone(), *interval).is_some() {
                    return Err(format!("duplicate combined feature '{namespaced}'"));
                }
            }
        }
    }
    Ok(combined)
}

fn evaluate_case(case: &Value) -> Result<Value, String> {
    ensure_keys(
        case,
        &[
            "case_id",
            "minimum_worst_case_ambiguity_reduction",
            "max_sequence_length",
            "hypotheses",
            "passive_observations",
            "probes",
        ],
        "case",
    )?;
    let case_id = string(case, "case_id", "case")?;
    let hypothesis_ids = parse_hypotheses(
        field(case, "hypotheses", &case_id)?,
        &format!("{case_id}.hypotheses"),
    )?;
    let minimum_reduction = field(case, "minimum_worst_case_ambiguity_reduction", &case_id)?
        .as_u64()
        .ok_or_else(|| format!("{case_id}: minimum ambiguity reduction must be an integer"))?
        as usize;
    if minimum_reduction == 0 || minimum_reduction >= hypothesis_ids.len() {
        return Err(format!("{case_id}: minimum ambiguity reduction outside 1..N-1"));
    }
    let max_sequence_length = field(case, "max_sequence_length", &case_id)?
        .as_u64()
        .ok_or_else(|| format!("{case_id}: max_sequence_length must be an integer"))?;
    if max_sequence_length != MAX_SEQUENCE_LENGTH as u64 {
        return Err(format!("{case_id}: v0.1 requires max_sequence_length=2"));
    }
    let passive = parse_observations(
        field(case, "passive_observations", &case_id)?,
        &hypothesis_ids,
        &format!("{case_id}.passive_observations"),
    )?;
    let raw_probes = field(case, "probes", &case_id)?
        .as_array()
        .ok_or_else(|| format!("{case_id}: probes must be an array"))?;
    if raw_probes.is_empty() || raw_probes.len() > MAX_PROBES {
        return Err(format!("{case_id}: probe count must be 1..{MAX_PROBES}"));
    }

    let mut probes = Vec::with_capacity(raw_probes.len());
    let mut probe_ids = BTreeSet::new();
    for (index, raw) in raw_probes.iter().enumerate() {
        let probe = parse_probe(raw, &hypothesis_ids, &format!("{case_id}.probes[{index}]"))?;
        if !probe_ids.insert(probe.id.clone()) {
            return Err(format!("{case_id}: duplicate probe_id '{}'", probe.id));
        }
        probes.push(probe);
    }
    probes.sort_by(|a, b| a.id.cmp(&b.id));

    let (baseline_size, baseline_cliques, baseline_edges) =
        maximum_cliques(&hypothesis_ids, &passive)?;
    let feasible: Vec<&Probe> = probes.iter().filter(|probe| probe.reasons.is_empty()).collect();
    let mut options = Vec::new();
    let mut ranked_choices = Vec::<OptionChoice>::new();

    for probe in &probes {
        if !probe.reasons.is_empty() {
            options.push(json!({
                "sequence": [probe.id],
                "feasible": false,
                "reason_codes": probe.reasons,
                "post_max_clique_size": null,
                "worst_case_reduction": null,
                "estimated_duration_seconds": probe.duration_seconds,
                "ambiguity_sets_after": [],
                "indistinguishability_edges_after": []
            }));
            continue;
        }
        let combined = combine_observations(&passive, &[probe], &hypothesis_ids)?;
        let (post_size, cliques, edges) = maximum_cliques(&hypothesis_ids, &combined)?;
        let reduction = baseline_size.saturating_sub(post_size);
        options.push(json!({
            "sequence": [probe.id],
            "feasible": true,
            "reason_codes": [],
            "post_max_clique_size": post_size,
            "worst_case_reduction": reduction,
            "estimated_duration_seconds": probe.duration_seconds,
            "ambiguity_sets_after": cliques,
            "indistinguishability_edges_after": edges
        }));
        ranked_choices.push(OptionChoice {
            sequence: vec![probe.id.clone()],
            post_max_clique: post_size,
            duration_seconds: probe.duration_seconds,
        });
    }

    for left in 0..feasible.len() {
        for right in (left + 1)..feasible.len() {
            let first = feasible[left];
            let second = feasible[right];
            // A simulated valve perturbation changes hydraulic state. v0.1
            // cannot compose it with another probe without a validated
            // state-transition/counterfactual model, so valve probes remain
            // single-step only.
            if first.kind == "simulated_valve_perturbation"
                || second.kind == "simulated_valve_perturbation"
            {
                continue;
            }
            let pair = [first, second];
            let combined = combine_observations(&passive, &pair, &hypothesis_ids)?;
            let (post_size, cliques, edges) = maximum_cliques(&hypothesis_ids, &combined)?;
            let reduction = baseline_size.saturating_sub(post_size);
            let duration = first
                .duration_seconds
                .checked_add(second.duration_seconds)
                .ok_or_else(|| format!("{case_id}: probe duration sum overflow"))?;
            let sequence = vec![first.id.clone(), second.id.clone()];
            options.push(json!({
                "sequence": sequence,
                "feasible": true,
                "reason_codes": [],
                "post_max_clique_size": post_size,
                "worst_case_reduction": reduction,
                "estimated_duration_seconds": duration,
                "ambiguity_sets_after": cliques,
                "indistinguishability_edges_after": edges
            }));
            ranked_choices.push(OptionChoice {
                sequence,
                post_max_clique: post_size,
                duration_seconds: duration,
            });
        }
    }

    options.sort_by(|left, right| {
        let left_key = left["sequence"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap_or_default())
            .collect::<Vec<_>>()
            .join("|");
        let right_key = right["sequence"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap_or_default())
            .collect::<Vec<_>>()
            .join("|");
        left_key.cmp(&right_key)
    });
    ranked_choices.retain(|choice| baseline_size.saturating_sub(choice.post_max_clique) >= minimum_reduction);
    ranked_choices.sort_by(|left, right| {
        left.post_max_clique
            .cmp(&right.post_max_clique)
            .then_with(|| left.sequence.len().cmp(&right.sequence.len()))
            .then_with(|| left.duration_seconds.cmp(&right.duration_seconds))
            .then_with(|| left.sequence.join("|").cmp(&right.sequence.join("|")))
    });

    let (decision_status, recommended_sequence, recommended_post) = if feasible.is_empty() {
        ("abstain_no_feasible_probe", None, None)
    } else if let Some(best) = ranked_choices.first() {
        (
            "counterfactual_probe_sequence_recommended",
            Some(best.sequence.clone()),
            Some(best.post_max_clique),
        )
    } else {
        ("abstain_no_probe_meets_worst_case_reduction", None, None)
    };
    Ok(json!({
        "case_id": case_id,
        "passive_max_clique_size": baseline_size,
        "passive_maximum_ambiguity_sets": baseline_cliques,
        "passive_indistinguishability_edges": baseline_edges,
        "feasible_probe_count": feasible.len(),
        "option_count": options.len(),
        "options": options,
        "decision_status": decision_status,
        "recommended_sequence": recommended_sequence,
        "recommended_post_max_clique_size": recommended_post,
        "minimum_worst_case_ambiguity_reduction": minimum_reduction,
        "execution_authorized": false,
        "probability_interpretation": "none_unweighted_interval_compatibility",
        "claim_eligibility": "NotEligible"
    }))
}

fn evaluate_all(input: &Value, input_schema: &Value, oracle: &Value) -> Result<Value, String> {
    if string(input, "$schema", "input")? != "https://json-schema.org/draft/2020-12/schema"
        || string(input_schema, "$schema", "input schema")?
            != "https://json-schema.org/draft/2020-12/schema"
    {
        return Err("input and schema must declare Draft 2020-12".to_owned());
    }
    if string(input, "schema_version", "input")? != INPUT_VERSION
        || string(input, "benchmark_id", "input")? != BENCHMARK_ID
        || string(input, "evidence_class", "input")? != "synthetic_only"
    {
        return Err("input identity or evidence class mismatch".to_owned());
    }
    let properties = field(input_schema, "properties", "input schema")?;
    if string(
        field(properties, "schema_version", "input schema properties")?,
        "const",
        "input schema properties.schema_version",
    )? != INPUT_VERSION
    {
        return Err("input schema version constant mismatch".to_owned());
    }
    let hypotheses_cap = field(input, "max_hypotheses", "input")?.as_u64();
    let probes_cap = field(input, "max_probes_per_case", "input")?.as_u64();
    let sequence_cap = field(input, "max_sequence_length", "input")?.as_u64();
    if hypotheses_cap != Some(MAX_HYPOTHESES as u64)
        || probes_cap != Some(MAX_PROBES as u64)
        || sequence_cap != Some(MAX_SEQUENCE_LENGTH as u64)
    {
        return Err("input enumeration bounds differ from v0.1 caps".to_owned());
    }

    ensure_keys(
        oracle,
        &["$schema", "schema_version", "benchmark_id", "cases"],
        "oracle",
    )?;
    if string(oracle, "$schema", "oracle")? != "https://json-schema.org/draft/2020-12/schema"
        || string(oracle, "schema_version", "oracle")? != ORACLE_VERSION
        || string(oracle, "benchmark_id", "oracle")? != BENCHMARK_ID
    {
        return Err("oracle identity/schema version mismatch".to_owned());
    }
    let cases = field(input, "cases", "input")?
        .as_array()
        .ok_or_else(|| "input.cases must be an array".to_owned())?;
    let expected_cases = field(oracle, "cases", "oracle")?
        .as_array()
        .ok_or_else(|| "oracle.cases must be an array".to_owned())?;
    if cases.len() != CASE_COUNT || expected_cases.len() != CASE_COUNT {
        return Err(format!("exactly {CASE_COUNT} cases are required"));
    }
    let mut expected_by_id = BTreeMap::new();
    for expected in expected_cases {
        let id = string(expected, "case_id", "oracle case")?.to_owned();
        if expected_by_id.insert(id.clone(), expected).is_some() {
            return Err(format!("duplicate oracle case_id '{id}'"));
        }
    }
    let mut observed_ids = BTreeSet::new();
    let mut results = Vec::with_capacity(CASE_COUNT);
    let mut mismatches = Vec::new();
    for case in cases {
        let id = string(case, "case_id", "case")?.to_owned();
        if !observed_ids.insert(id.clone()) {
            return Err(format!("duplicate input case_id '{id}'"));
        }
        let actual = evaluate_case(case)?;
        let expected = expected_by_id
            .get(&id)
            .ok_or_else(|| format!("input case '{id}' missing from oracle"))?;
        if &actual != *expected {
            mismatches.push(json!({
                "case_id": id,
                "expected": expected,
                "actual": actual.clone()
            }));
        }
        results.push(actual);
    }
    if observed_ids != expected_by_id.keys().cloned().collect() {
        return Err("input/oracle case ID sets differ".to_owned());
    }
    let mismatch_count = mismatches.len();
    Ok(json!({
        "schema_version": "wdn-active-observability-t0-evaluation-v0.1",
        "benchmark_id": BENCHMARK_ID,
        "case_count": CASE_COUNT,
        "matched_case_count": CASE_COUNT - mismatch_count,
        "mismatch_count": mismatch_count,
        "mismatches": mismatches,
        "case_results": results,
        "execution_authorized": false,
        "probability_interpretation": "none_unweighted_interval_compatibility",
        "claim_eligibility": "NotEligible",
        "physical_qualification_status": "NotRun"
    }))
}

fn read_json(path: &str) -> Result<(Vec<u8>, Value), String> {
    let bytes = fs::read(path).map_err(|error| format!("read '{path}': {error}"))?;
    let value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid JSON '{path}': {error}"))?;
    Ok((bytes, value))
}

fn run(args: &[String]) -> Result<(), String> {
    if args.len() != 4 {
        return Err(
            "usage: wdn-active-observability-t0 INPUT_JSON INPUT_SCHEMA ORACLE_JSON RECEIPT_JSON"
                .to_owned(),
        );
    }
    let (input_bytes, input) = read_json(&args[0])?;
    let (schema_bytes, schema) = read_json(&args[1])?;
    let (oracle_bytes, oracle) = read_json(&args[2])?;
    let result = evaluate_all(&input, &schema, &oracle)?;
    let success = result["case_count"].as_u64() == Some(CASE_COUNT as u64)
        && result["matched_case_count"].as_u64() == Some(CASE_COUNT as u64)
        && result["mismatch_count"].as_u64() == Some(0);
    let revision = env::var("WDN_ACTIVE_OBSERVABILITY_T0_REVISION")
        .or_else(|_| env::var("GITHUB_SHA"))
        .ok()
        .filter(|value| valid_revision(value));
    let manifest_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let lock_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock");
    let manifest_bytes = fs::read(&manifest_path)
        .map_err(|error| format!("read {}: {error}", manifest_path.display()))?;
    let lock_bytes = fs::read(&lock_path)
        .map_err(|error| format!("read {}: {error}", lock_path.display()))?;
    let implementation = include_bytes!("wdn-active-observability-t0.rs");
    let receipt = json!({
        "receipt_schema_version": "wdn-active-observability-t0-receipt-v0.1",
        "benchmark_id": BENCHMARK_ID,
        "verification_state": if success && revision.is_some() {
            "PassedExactRevision"
        } else if success {
            "PassedLocalUnpinned"
        } else {
            "OracleMismatch"
        },
        "qualification_status": "NotRun",
        "physical_qualification_status": "NotRun",
        "claim_eligibility": "NotEligible",
        "implementation_revision": revision,
        "input_sha256": sha256(&input_bytes),
        "input_schema_sha256": sha256(&schema_bytes),
        "oracle_sha256": sha256(&oracle_bytes),
        "package_manifest_sha256": sha256(&manifest_bytes),
        "cargo_lock_sha256": sha256(&lock_bytes),
        "implementation_sha256": sha256(implementation),
        "case_count": result["case_count"],
        "matched_case_count": result["matched_case_count"],
        "mismatch_count": result["mismatch_count"],
        "mismatches": result["mismatches"],
        "execution_authorized": false,
        "claim_ceiling": "counterfactual_observability_and_feasibility_contract_only"
    });
    let output = Path::new(&args[3]);
    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create receipt directory: {error}"))?;
    }
    fs::write(
        output,
        serde_json::to_vec_pretty(&receipt)
            .map_err(|error| format!("serialize receipt: {error}"))?,
    )
    .map_err(|error| format!("write receipt '{}': {error}", output.display()))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt).unwrap_or_default()
    );
    if success {
        Ok(())
    } else {
        Err("active observability evaluator did not match the frozen oracle".to_owned())
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if let Err(error) = run(&args) {
        eprintln!("wdn-active-observability-t0: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (Value, Value, Value) {
        let input = serde_json::from_slice(include_bytes!(
            "../../../../docs/research/water-observability/wdn_active_observability_t0_input.v0.1.json"
        )).unwrap();
        let schema = serde_json::from_slice(include_bytes!(
            "../../../../docs/research/water-observability/wdn_active_observability_t0_input.schema.v0.1.json"
        )).unwrap();
        let oracle = serde_json::from_slice(include_bytes!(
            "../../../../docs/research/water-observability/wdn_active_observability_t0_oracle.v0.1.json"
        )).unwrap();
        (input, schema, oracle)
    }

    fn case_result<'a>(result: &'a Value, id: &str) -> &'a Value {
        result["case_results"].as_array().unwrap().iter()
            .find(|case| case["case_id"] == id).unwrap()
    }

    #[test]
    fn canonical_fourteen_case_oracle_matches() {
        let (input, schema, oracle) = fixture();
        let result = evaluate_all(&input, &schema, &oracle).unwrap();
        assert_eq!(result["matched_case_count"], 14, "mismatches: {}", result["mismatches"]);
        assert_eq!(result["mismatch_count"], 0);
        assert_eq!(result["execution_authorized"], false);
        assert_eq!(result["physical_qualification_status"], "NotRun");
    }

    #[test]
    fn safe_temporary_sensor_reduces_worst_case_ambiguity_from_four_to_one() {
        let (input, _, _) = fixture();
        let result = evaluate_case(&input["cases"][0]).unwrap();
        assert_eq!(result["passive_max_clique_size"], 4);
        assert_eq!(result["recommended_post_max_clique_size"], 1);
        assert_eq!(result["recommended_sequence"][0], "TEMP-PRESSURE-1");
    }

    #[test]
    fn manual_observation_reduces_ambiguity_to_two() {
        let (input, _, _) = fixture();
        let result = evaluate_case(&input["cases"][1]).unwrap();
        assert_eq!(result["recommended_post_max_clique_size"], 2);
    }

    #[test]
    fn pressure_fire_flow_transient_and_identity_failures_all_abstain() {
        let (input, _, _) = fixture();
        for index in 2..6 {
            let result = evaluate_case(&input["cases"][index]).unwrap();
            assert_eq!(result["decision_status"], "abstain_no_feasible_probe");
            assert_eq!(result["recommended_sequence"], Value::Null);
            assert_eq!(result["feasible_probe_count"], 0);
        }
    }

    #[test]
    fn stale_model_and_invalid_time_calibration_force_abstention() {
        let (input, _, _) = fixture();
        for index in [6, 7] {
            let result = evaluate_case(&input["cases"][index]).unwrap();
            assert_eq!(result["decision_status"], "abstain_no_feasible_probe");
        }
    }

    #[test]
    fn overlapping_uncertainty_does_not_claim_information_gain() {
        let (input, _, _) = fixture();
        let result = evaluate_case(&input["cases"][8]).unwrap();
        assert_eq!(result["decision_status"], "abstain_no_probe_meets_worst_case_reduction");
    }

    #[test]
    fn weak_probe_must_meet_the_declared_worst_case_reduction() {
        let (input, _, _) = fixture();
        let result = evaluate_case(&input["cases"][9]).unwrap();
        assert_eq!(result["decision_status"], "abstain_no_probe_meets_worst_case_reduction");
        assert!(result["recommended_sequence"].is_null());
    }

    #[test]
    fn sequential_pair_can_separate_hypotheses_when_each_single_probe_cannot() {
        let (input, _, _) = fixture();
        let result = evaluate_case(&input["cases"][10]).unwrap();
        assert_eq!(result["recommended_post_max_clique_size"], 1);
        assert_eq!(result["recommended_sequence"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn permutation_of_hypothesis_and_probe_observation_rows_is_invariant() {
        let (input, _, _) = fixture();
        let first = evaluate_case(&input["cases"][0]).unwrap();
        let permuted = evaluate_case(&input["cases"][11]).unwrap();
        assert_eq!(first["passive_max_clique_size"], permuted["passive_max_clique_size"]);
        assert_eq!(first["recommended_post_max_clique_size"], permuted["recommended_post_max_clique_size"]);
        assert_eq!(first["recommended_sequence"], permuted["recommended_sequence"]);
    }

    #[test]
    fn missing_operator_authorization_is_not_overridden_by_signature_gain() {
        let (input, _, _) = fixture();
        let result = evaluate_case(&input["cases"][12]).unwrap();
        assert_eq!(result["decision_status"], "abstain_no_feasible_probe");
    }

    #[test]
    fn no_feasible_probe_preserves_abstention_and_unresolved_baseline() {
        let (input, _, _) = fixture();
        let result = evaluate_case(&input["cases"][13]).unwrap();
        assert_eq!(result["decision_status"], "abstain_no_feasible_probe");
        assert_eq!(result["passive_max_clique_size"], 4);
    }

    #[test]
    fn maximum_clique_handles_nontransitive_overlap_graph() {
        let ids = vec!["A".to_owned(), "B".to_owned(), "C".to_owned()];
        let mut features = FeatureMap::new();
        features.insert("A".to_owned(), BTreeMap::from([("x".to_owned(), (0.0, 2.0))]));
        features.insert("B".to_owned(), BTreeMap::from([("x".to_owned(), (1.0, 3.0))]));
        features.insert("C".to_owned(), BTreeMap::from([("x".to_owned(), (2.5, 4.0))]));
        let (max, cliques, edges) = maximum_cliques(&ids, &features).unwrap();
        assert_eq!(max, 2);
        assert_eq!(cliques.len(), 2);
        assert_eq!(edges.len(), 2);
    }

    #[test]
    fn reversed_interval_is_rejected() {
        let (mut input, _, _) = fixture();
        input["cases"][0]["probes"][0]["observations"][0]["features"][0]["lower"] = json!(9.0);
        input["cases"][0]["probes"][0]["observations"][0]["features"][0]["upper"] = json!(2.0);
        assert!(evaluate_case(&input["cases"][0]).unwrap_err().contains("reversed interval"));
    }

    #[test]
    fn non_valve_probe_cannot_smuggle_valve_identity() {
        let (mut input, _, _) = fixture();
        input["cases"][0]["probes"][0]["feasibility"]["valve_identity_verified"] = json!(true);
        assert!(evaluate_case(&input["cases"][0]).unwrap_err().contains("non-valve probe"));
    }

    #[test]
    fn maximum_hypothesis_count_is_bounded_before_bitmask_enumeration() {
        let (mut input, _, _) = fixture();
        let extra = input["cases"][0]["hypotheses"][0].clone();
        input["cases"][0]["hypotheses"].as_array_mut().unwrap().push(extra);
        assert!(evaluate_case(&input["cases"][0]).unwrap_err().contains("hypothesis count"));
    }

    #[test]
    fn altered_oracle_is_detected_as_one_mismatch() {
        let (input, schema, mut oracle) = fixture();
        oracle["cases"][0]["passive_max_clique_size"] = json!(99);
        let result = evaluate_all(&input, &schema, &oracle).unwrap();
        assert_eq!(result["matched_case_count"], 13);
        assert_eq!(result["mismatch_count"], 1);
    }
}
