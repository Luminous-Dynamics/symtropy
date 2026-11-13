// SPDX-License-Identifier: AGPL-3.0-or-later
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    process::ExitCode,
};

const VERSION: &str = "0.1.0";
const FIXTURE: &str = include_str!(
    "../../../../benchmarks/maritime-control-response/environment-context-robustness-v0.1.json"
);
type Check<T> = Result<T, String>;

fn object<'a>(v: &'a Value, path: &str) -> Check<&'a serde_json::Map<String, Value>> {
    v.as_object()
        .ok_or_else(|| format!("{path}: expected object"))
}
fn exact_keys(v: &Value, keys: &[&str], path: &str) -> Check<()> {
    let map = object(v, path)?;
    let expected: BTreeSet<&str> = keys.iter().copied().collect();
    let actual: BTreeSet<&str> = map.keys().map(String::as_str).collect();
    if expected != actual {
        return Err(format!("{path}: required/actual key sets differ"));
    }
    Ok(())
}
fn field<'a>(v: &'a Value, key: &str, path: &str) -> Check<&'a Value> {
    object(v, path)?
        .get(key)
        .ok_or_else(|| format!("{path}: missing {key}"))
}
fn string<'a>(v: &'a Value, key: &str, path: &str) -> Check<&'a str> {
    field(v, key, path)?
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| format!("{path}.{key}: expected non-empty string"))
}
fn number(v: &Value, key: &str, path: &str) -> Check<f64> {
    let n = field(v, key, path)?
        .as_f64()
        .ok_or_else(|| format!("{path}.{key}: expected number"))?;
    if !n.is_finite() {
        return Err(format!("{path}.{key}: non-finite number"));
    }
    Ok(n)
}
fn unsigned(v: &Value, key: &str, path: &str) -> Check<u64> {
    field(v, key, path)?
        .as_u64()
        .ok_or_else(|| format!("{path}.{key}: expected unsigned integer"))
}
fn boolean(v: &Value, key: &str, path: &str) -> Check<bool> {
    field(v, key, path)?
        .as_bool()
        .ok_or_else(|| format!("{path}.{key}: expected boolean"))
}
fn string_set(v: &Value, key: &str, path: &str) -> Check<BTreeSet<String>> {
    let values = field(v, key, path)?
        .as_array()
        .ok_or_else(|| format!("{path}.{key}: expected array"))?;
    let mut out = BTreeSet::new();
    for item in values {
        let s = item
            .as_str()
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| format!("{path}.{key}: expected non-empty strings"))?;
        if !out.insert(s.to_owned()) {
            return Err(format!("{path}.{key}: duplicate value"));
        }
    }
    Ok(out)
}
fn bounds(t: u64, u: u64) -> Option<(u64, u64)> {
    Some((t.checked_sub(u)?, t.checked_add(u)?))
}
fn interval(v: &Value, path: &str) -> Check<(f64, f64, f64, String)> {
    exact_keys(v, &["lower", "mean", "upper", "unit"], path)?;
    let lower = number(v, "lower", path)?;
    let mean = number(v, "mean", path)?;
    let upper = number(v, "upper", path)?;
    let unit = string(v, "unit", path)?.to_owned();
    if !(lower <= mean && mean <= upper) {
        return Err(format!("{path}: expected lower <= mean <= upper"));
    }
    Ok((lower, mean, upper, unit))
}
fn disposition(name: &str, reason: &str) -> Value {
    json!({
        "disposition": name,
        "reason_code": reason,
        "fault_conclusion": "NOT_ATTEMPTED",
        "causal_attribution": "NOT_ESTABLISHED",
        "operational_authority": "NONE"
    })
}
fn evaluate_case(case: &Value) -> Check<(String, Value)> {
    let path = "$.cases[]";
    exact_keys(
        case,
        &[
            "case_id",
            "asset_configuration_id",
            "regime_id",
            "internal_state_id",
            "fault_truth",
            "timebase",
            "prediction_time_ms",
            "prediction_timestamp_uncertainty_ms",
            "max_response_horizon_ms",
            "internal_dependency_groups",
            "external_context",
            "context_path",
            "state_only_envelope",
            "context_conditioned_envelope",
            "observed_response",
            "expected_disposition",
        ],
        path,
    )?;
    let id = string(case, "case_id", path)?.to_owned();
    let config = string(case, "asset_configuration_id", path)?;
    let regime = string(case, "regime_id", path)?;
    let _internal_state = string(case, "internal_state_id", path)?;
    let truth = string(case, "fault_truth", path)?;
    if !matches!(truth, "HEALTHY" | "FAULT_PRESENT") {
        return Err(format!("{id}: invalid synthetic fault truth"));
    }

    let timebase = field(case, "timebase", path)?;
    exact_keys(
        timebase,
        &[
            "timebase_id",
            "qualification_state",
            "qualification_id",
            "qualification_sha256",
            "valid_from_ms",
            "valid_until_ms",
        ],
        "$.cases[].timebase",
    )?;
    let timebase_id = string(timebase, "timebase_id", path)?;
    let clock_state = string(timebase, "qualification_state", path)?;
    let clock_sha = string(timebase, "qualification_sha256", path)?;
    if clock_sha.len() != 64 || !clock_sha.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Ok((
            id,
            disposition("INVALID_CONTRACT", "CLOCK_QUALIFICATION_DIGEST_INVALID"),
        ));
    }
    if clock_state != "QUALIFIED" {
        return Ok((
            id,
            disposition("NOT_EVALUATED", "CLOCK_QUALIFICATION_UNAVAILABLE"),
        ));
    }
    let clock_from = unsigned(timebase, "valid_from_ms", path)?;
    let clock_until = unsigned(timebase, "valid_until_ms", path)?;
    let prediction_time = unsigned(case, "prediction_time_ms", path)?;
    let prediction_u = unsigned(case, "prediction_timestamp_uncertainty_ms", path)?;
    let prediction = match bounds(prediction_time, prediction_u) {
        Some(v) => v,
        None => {
            return Ok((
                id,
                disposition("INVALID_CONTRACT", "PREDICTION_INTERVAL_OVERFLOW"),
            ));
        }
    };
    if prediction.0 < clock_from || prediction.1 > clock_until {
        return Ok((
            id,
            disposition("NOT_EVALUATED", "TIMEBASE_NOT_VALID_AT_PREDICTION"),
        ));
    }

    let context = field(case, "external_context", path)?;
    exact_keys(
        context,
        &[
            "context_id",
            "context_kind",
            "value",
            "unit",
            "source_role",
            "quality_state",
            "dependency_groups",
            "timebase_id",
            "observed_at_ms",
            "timestamp_uncertainty_ms",
            "in_support",
        ],
        "$.cases[].external_context",
    )?;
    let context_id = string(context, "context_id", path)?.to_owned();
    let context_kind = string(context, "context_kind", path)?;
    if !matches!(
        context_kind,
        "sea_state"
            | "wind_wave_height"
            | "draft_trim"
            | "sea_surface_temperature"
            | "negative_control"
    ) {
        return Ok((id, disposition("INVALID_CONTRACT", "UNKNOWN_CONTEXT_KIND")));
    }
    let _context_value = number(context, "value", path)?;
    let _context_unit = string(context, "unit", path)?;
    let source_role = string(context, "source_role", path)?;
    let quality = string(context, "quality_state", path)?;
    let context_timebase = string(context, "timebase_id", path)?;
    if context_timebase != timebase_id {
        return Ok((
            id,
            disposition("NOT_EVALUATED", "CONTEXT_TIMEBASE_UNMAPPED"),
        ));
    }
    if source_role != "MEASURED" {
        return Ok((
            id,
            disposition(
                "NOT_EVALUATED",
                "EXTERNAL_CONTEXT_NOT_INDEPENDENTLY_MEASURED",
            ),
        ));
    }
    if quality != "CURRENT" {
        return Ok((
            id,
            disposition("NOT_EVALUATED", "EXTERNAL_CONTEXT_NOT_CURRENT"),
        ));
    }
    if !boolean(context, "in_support", path)? {
        return Ok((
            id,
            disposition("NOT_EVALUATED", "EXTERNAL_CONTEXT_OUT_OF_SUPPORT"),
        ));
    }
    let context_deps = string_set(context, "dependency_groups", path)?;
    let internal_deps = string_set(case, "internal_dependency_groups", path)?;
    if context_deps.iter().any(|d| internal_deps.contains(d)) {
        return Ok((
            id,
            disposition(
                "NOT_EVALUATED",
                "EXTERNAL_CONTEXT_SHARES_INTERNAL_DEPENDENCY",
            ),
        ));
    }
    let context_time = unsigned(context, "observed_at_ms", path)?;
    let context_u = unsigned(context, "timestamp_uncertainty_ms", path)?;
    let context_bounds = match bounds(context_time, context_u) {
        Some(v) => v,
        None => {
            return Ok((
                id,
                disposition("INVALID_CONTRACT", "CONTEXT_INTERVAL_OVERFLOW"),
            ));
        }
    };
    if context_bounds.0 > prediction.1 {
        return Ok((
            id,
            disposition("INVALID_CONTRACT", "POST_PREDICTION_CONTEXT_LEAKAGE"),
        ));
    }
    if context_bounds.1 > prediction.0 {
        return Ok((
            id,
            disposition("NOT_EVALUATED", "CONTEXT_PREDICTION_ORDER_UNPROVEN"),
        ));
    }

    let (base_lower, base_mean, base_upper, base_unit) = interval(
        field(case, "state_only_envelope", path)?,
        "$.cases[].state_only_envelope",
    )?;
    let (cond_lower, cond_mean, cond_upper, cond_unit) = interval(
        field(case, "context_conditioned_envelope", path)?,
        "$.cases[].context_conditioned_envelope",
    )?;
    if base_unit != cond_unit {
        return Ok((
            id,
            disposition("INVALID_CONTRACT", "CONTEXT_ENVELOPE_UNIT_MISMATCH"),
        ));
    }

    let cp = field(case, "context_path", path)?;
    exact_keys(
        cp,
        &[
            "path_id",
            "qualification_state",
            "configuration_id",
            "regime_id",
            "timebase_id",
            "qualified_at_ms",
            "valid_until_ms",
            "effect_lower",
            "effect_upper",
            "unit",
        ],
        "$.cases[].context_path",
    )?;
    let cp_state = string(cp, "qualification_state", path)?;
    let cp_config = string(cp, "configuration_id", path)?;
    let cp_regime = string(cp, "regime_id", path)?;
    let cp_timebase = string(cp, "timebase_id", path)?;
    let cp_unit = string(cp, "unit", path)?;
    let effect_lower = number(cp, "effect_lower", path)?;
    let effect_upper = number(cp, "effect_upper", path)?;
    if effect_lower > effect_upper || cp_unit != base_unit {
        return Ok((
            id,
            disposition("INVALID_CONTRACT", "CONTEXT_PATH_EFFECT_INVALID"),
        ));
    }

    if cp_state == "QUALIFIED" {
        if cp_config != config || cp_regime != regime || cp_timebase != timebase_id {
            return Ok((
                id,
                disposition("NOT_EVALUATED", "CONTEXT_PATH_SCOPE_MISMATCH"),
            ));
        }
        let qualified_at = unsigned(cp, "qualified_at_ms", path)?;
        let valid_until = unsigned(cp, "valid_until_ms", path)?;
        if qualified_at > prediction.1 {
            return Ok((
                id,
                disposition(
                    "INVALID_CONTRACT",
                    "CONTEXT_PATH_QUALIFIED_AFTER_PREDICTION",
                ),
            ));
        }
        if qualified_at > prediction.0 {
            return Ok((
                id,
                disposition("NOT_EVALUATED", "CONTEXT_PATH_QUALIFICATION_TIME_UNPROVEN"),
            ));
        }
        if valid_until < prediction.1 {
            return Ok((id, disposition("NOT_EVALUATED", "CONTEXT_PATH_EXPIRED")));
        }
        let lower_shift = cond_lower - base_lower;
        let upper_shift = cond_upper - base_upper;
        if lower_shift < effect_lower - 1e-9
            || lower_shift > effect_upper + 1e-9
            || upper_shift < effect_lower - 1e-9
            || upper_shift > effect_upper + 1e-9
        {
            return Ok((
                id,
                disposition(
                    "INVALID_CONTRACT",
                    "CONDITIONED_ENVELOPE_CONTRADICTS_PREREGISTERED_EFFECT",
                ),
            ));
        }
    } else if cp_state == "NO_QUALIFIED_PATH" {
        if effect_lower != 0.0
            || effect_upper != 0.0
            || cond_lower != base_lower
            || cond_mean != base_mean
            || cond_upper != base_upper
        {
            return Ok((
                id,
                disposition(
                    "INVALID_CONTRACT",
                    "UNQUALIFIED_CONTEXT_PATH_CHANGES_ENVELOPE",
                ),
            ));
        }
    } else {
        return Ok((
            id,
            disposition("NOT_EVALUATED", "CONTEXT_PATH_NOT_QUALIFIED"),
        ));
    }

    let response = field(case, "observed_response", path)?;
    exact_keys(
        response,
        &[
            "value",
            "unit",
            "quality_state",
            "timebase_id",
            "observed_at_ms",
            "timestamp_uncertainty_ms",
        ],
        "$.cases[].observed_response",
    )?;
    let observed = number(response, "value", path)?;
    if string(response, "unit", path)? != base_unit
        || string(response, "timebase_id", path)? != timebase_id
    {
        return Ok((
            id,
            disposition(
                "INVALID_CONTRACT",
                "OBSERVED_RESPONSE_UNIT_OR_TIMEBASE_MISMATCH",
            ),
        ));
    }
    if string(response, "quality_state", path)? != "CURRENT" {
        return Ok((
            id,
            disposition("NOT_EVALUATED", "PHYSICAL_RESPONSE_NOT_CURRENT"),
        ));
    }
    let response_time = unsigned(response, "observed_at_ms", path)?;
    let response_u = unsigned(response, "timestamp_uncertainty_ms", path)?;
    let response_bounds = match bounds(response_time, response_u) {
        Some(v) => v,
        None => {
            return Ok((
                id,
                disposition("INVALID_CONTRACT", "RESPONSE_INTERVAL_OVERFLOW"),
            ));
        }
    };
    if response_bounds.1 <= prediction.0 {
        return Ok((
            id,
            disposition("INVALID_CONTRACT", "RESPONSE_PRECEDES_PREDICTION"),
        ));
    }
    if response_bounds.0 <= prediction.1 {
        return Ok((
            id,
            disposition("NOT_EVALUATED", "RESPONSE_PREDICTION_ORDER_UNPROVEN"),
        ));
    }
    if response_bounds.0 < clock_from || response_bounds.1 > clock_until {
        return Ok((
            id,
            disposition("NOT_EVALUATED", "TIMEBASE_NOT_VALID_AT_RESPONSE"),
        ));
    }
    let horizon = unsigned(case, "max_response_horizon_ms", path)?;
    if response_bounds.1 - prediction.0 > horizon {
        if response_bounds.0 - prediction.1 > horizon {
            return Ok((
                id,
                disposition("INVALID_CONTRACT", "RESPONSE_OUTSIDE_HORIZON"),
            ));
        }
        return Ok((
            id,
            disposition("NOT_EVALUATED", "RESPONSE_HORIZON_UNPROVEN"),
        ));
    }

    let inside_conditioned = observed >= cond_lower && observed <= cond_upper;
    let inside_baseline = observed >= base_lower && observed <= base_upper;
    let mut out = if inside_conditioned && !inside_baseline && cp_state == "QUALIFIED" {
        disposition(
            "CONTEXT_EXPLAINS_RESPONSE",
            "CONDITIONED_RESPONSE_CONSISTENT_STATE_ONLY_WOULD_FLAG",
        )
    } else if inside_conditioned {
        disposition(
            "NO_CONTEXT_CONFOUNDING",
            "OBSERVED_RESPONSE_WITHIN_QUALIFIED_ENVELOPE",
        )
    } else {
        disposition(
            "RESPONSE_MISMATCH_UNISOLATED",
            "OBSERVED_RESPONSE_OUTSIDE_CONDITIONED_ENVELOPE",
        )
    };
    out["context_id"] = json!(context_id);
    out["context_kind"] = json!(context_kind);
    out["context_path_id"] = json!(string(cp, "path_id", path).unwrap_or(""));
    out["observed_response"] = json!(observed);
    out["conditioned_envelope"] = json!([cond_lower, cond_mean, cond_upper]);
    out["state_only_envelope"] = json!([base_lower, base_mean, base_upper]);
    out["fault_truth_fixture_only"] = json!(truth);
    Ok((id, out))
}
fn pair_check(
    pair: &Value,
    outcomes: &BTreeMap<String, Value>,
    cases: &BTreeMap<String, &Value>,
) -> Check<Value> {
    let path = "$.counterfactual_pairs[]";
    let kind = string(pair, "kind", path)?;
    let ids = field(pair, "case_ids", path)?
        .as_array()
        .ok_or("case_ids must be array")?;
    if ids.len() != 2 {
        return Err(format!("{path}: exactly two cases required"));
    }
    let a_id = ids[0].as_str().ok_or("case ID must be string")?;
    let b_id = ids[1].as_str().ok_or("case ID must be string")?;
    let a = *cases.get(a_id).ok_or("unknown case A")?;
    let b = *cases.get(b_id).ok_or("unknown case B")?;
    for id in [a_id, b_id] {
        if !matches!(
            outcomes.get(id).and_then(|v| v["disposition"].as_str()),
            Some(
                "NO_CONTEXT_CONFOUNDING"
                    | "CONTEXT_EXPLAINS_RESPONSE"
                    | "RESPONSE_MISMATCH_UNISOLATED"
            )
        ) {
            return Err(format!("{path}: pair case {id} was not evaluable"));
        }
    }
    match kind {
        "SAME_INTERNAL_STATE_DIFFERENT_CONTEXT" => {
            exact_keys(
                pair,
                &[
                    "pair_id",
                    "kind",
                    "case_ids",
                    "minimum_context_delta",
                    "minimum_conditioned_mean_delta",
                    "minimum_observed_response_delta",
                ],
                path,
            )?;
            if string(a, "internal_state_id", path)? != string(b, "internal_state_id", path)?
                || string(a, "fault_truth", path)? != string(b, "fault_truth", path)?
            {
                return Err(format!(
                    "{path}: internal state/fault truth must be held constant"
                ));
            }
            let ca = field(a, "external_context", path)?;
            let cb = field(b, "external_context", path)?;
            if string(ca, "unit", path)? != string(cb, "unit", path)? {
                return Err("context units differ".into());
            }
            let context_delta = (number(ca, "value", path)? - number(cb, "value", path)?).abs();
            let ea = field(a, "context_conditioned_envelope", path)?;
            let eb = field(b, "context_conditioned_envelope", path)?;
            let mean_delta = (number(ea, "mean", path)? - number(eb, "mean", path)?).abs();
            let ra = field(a, "observed_response", path)?;
            let rb = field(b, "observed_response", path)?;
            let observed_delta = (number(ra, "value", path)? - number(rb, "value", path)?).abs();
            if context_delta < number(pair, "minimum_context_delta", path)?
                || mean_delta < number(pair, "minimum_conditioned_mean_delta", path)?
                || observed_delta < number(pair, "minimum_observed_response_delta", path)?
            {
                return Err(format!(
                    "{path}: same-state/different-context counterfactual failed"
                ));
            }
        }
        "SAME_CONTEXT_DIFFERENT_INTERNAL_FAULT" => {
            exact_keys(pair, &["pair_id", "kind", "case_ids"], path)?;
            let ca = field(a, "external_context", path)?;
            let cb = field(b, "external_context", path)?;
            if string(ca, "context_id", path)? != string(cb, "context_id", path)?
                || number(ca, "value", path)? != number(cb, "value", path)?
                || string(a, "fault_truth", path)? == string(b, "fault_truth", path)?
            {
                return Err(format!(
                    "{path}: context must match and fault truth must differ"
                ));
            }
            let da = outcomes[a_id]["disposition"].as_str().unwrap_or("");
            let db = outcomes[b_id]["disposition"].as_str().unwrap_or("");
            if !matches!(
                (da, db),
                ("CONTEXT_EXPLAINS_RESPONSE", "RESPONSE_MISMATCH_UNISOLATED")
                    | ("RESPONSE_MISMATCH_UNISOLATED", "CONTEXT_EXPLAINS_RESPONSE")
            ) {
                return Err(format!(
                    "{path}: environment must not excuse internal fault"
                ));
            }
        }
        other => return Err(format!("{path}: unknown pair kind {other}")),
    }
    Ok(
        json!({"pair_id":string(pair,"pair_id",path)?,"kind":kind,"status":"PASS","case_ids":[a_id,b_id]}),
    )
}
fn run(path: &str, source: &str, digest: &str) -> Check<Value> {
    if source.len() != 40
        || !source.bytes().all(|b| b.is_ascii_hexdigit())
        || digest.len() != 64
        || !digest.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err("invalid source commit or corpus SHA-256".into());
    }
    let raw = fs::read_to_string(path).map_err(|e| format!("read corpus: {e}"))?;
    let corpus: Value = serde_json::from_str(&raw).map_err(|e| format!("parse corpus: {e}"))?;
    exact_keys(
        &corpus,
        &[
            "schema_version",
            "benchmark_id",
            "evidence_class",
            "cases",
            "counterfactual_pairs",
            "claim_ceiling",
        ],
        "$",
    )?;
    if string(&corpus, "schema_version", "$")? != VERSION
        || string(&corpus, "evidence_class", "$")? != "SYNTHETIC_KNOWN_TRUTH"
    {
        return Err("unexpected corpus schema or evidence class".into());
    }
    let cases = field(&corpus, "cases", "$")?
        .as_array()
        .ok_or("cases must be array")?;
    if cases.is_empty() || cases.len() > 10000 {
        return Err("case count outside limits".into());
    }
    let mut outcomes = BTreeMap::new();
    let mut case_map = BTreeMap::new();
    for case in cases {
        let (id, output) = evaluate_case(case)?;
        if output["disposition"].as_str()
            != field(case, "expected_disposition", "$.cases[]")?.as_str()
        {
            return Err(format!(
                "{id}: expected {}, got {}",
                case["expected_disposition"], output["disposition"]
            ));
        }
        if case_map.insert(id.clone(), case).is_some() {
            return Err(format!("duplicate case ID {id}"));
        }
        outcomes.insert(id, output);
    }
    let pairs = field(&corpus, "counterfactual_pairs", "$")?
        .as_array()
        .ok_or("counterfactual pairs must be array")?;
    let mut pair_ids = BTreeSet::new();
    let mut pair_receipts = Vec::new();
    for pair in pairs {
        let id = string(pair, "pair_id", "$.counterfactual_pairs[]")?.to_owned();
        if !pair_ids.insert(id.clone()) {
            return Err(format!("duplicate pair ID {id}"));
        }
        pair_receipts.push(pair_check(pair, &outcomes, &case_map)?);
    }
    let case_receipts: Vec<Value> = outcomes
        .iter()
        .map(|(id, out)| json!({"case_id":id,"result":out}))
        .collect();
    Ok(json!({
        "result":"pass",
        "benchmark_id":string(&corpus,"benchmark_id","$")?,
        "validator_version":VERSION,
        "source_commit":source,
        "corpus_sha256":digest,
        "case_count":cases.len(),
        "pair_count":pair_receipts.len(),
        "cases":case_receipts,
        "counterfactual_pairs":pair_receipts,
        "fault_conclusion":"NOT_ATTEMPTED",
        "causal_attribution":"NOT_ESTABLISHED",
        "claim_ceiling":string(&corpus,"claim_ceiling","$")?
    }))
}
fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 6 || args[2] != "--source-commit" || args[4] != "--corpus-sha256" {
        eprintln!(
            "usage: external_context_robustness <corpus.json> --source-commit <40-hex> --corpus-sha256 <64-hex>"
        );
        return ExitCode::from(2);
    }
    match run(&args[1], &args[3], &args[5]) {
        Ok(out) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&out)
                    .unwrap_or_else(|_| "{\"result\":\"fail\"}".into())
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("external-context benchmark failed closed: {error}");
            ExitCode::FAILURE
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn corpus() -> Value {
        serde_json::from_str(FIXTURE).expect("fixture parses")
    }
    #[test]
    fn all_ten_synthetic_cases_match_known_truth() {
        let c = corpus();
        let cases = c["cases"].as_array().unwrap();
        assert_eq!(cases.len(), 10);
        for case in cases {
            let (id, out) = evaluate_case(case).unwrap();
            assert_eq!(out["disposition"], case["expected_disposition"], "{id}");
            assert_eq!(out["fault_conclusion"], "NOT_ATTEMPTED");
            assert_eq!(out["causal_attribution"], "NOT_ESTABLISHED");
        }
    }
    #[test]
    fn counterfactual_environment_pairs_pass() {
        let c = corpus();
        let mut out = BTreeMap::new();
        let mut cases = BTreeMap::new();
        for case in c["cases"].as_array().unwrap() {
            let (id, result) = evaluate_case(case).unwrap();
            out.insert(id.clone(), result);
            cases.insert(id, case);
        }
        let pairs = c["counterfactual_pairs"].as_array().unwrap();
        assert_eq!(pairs.len(), 2);
        for p in pairs {
            assert_eq!(pair_check(p, &out, &cases).unwrap()["status"], "PASS");
        }
    }
    #[test]
    fn weather_context_does_not_create_false_internal_fault() {
        let c = corpus();
        let (_, out) = evaluate_case(&c["cases"][1]).unwrap();
        assert_eq!(out["disposition"], "CONTEXT_EXPLAINS_RESPONSE");
        assert_eq!(out["fault_conclusion"], "NOT_ATTEMPTED");
    }
    #[test]
    fn external_context_does_not_excuse_internal_degradation() {
        let c = corpus();
        let (_, out) = evaluate_case(&c["cases"][2]).unwrap();
        assert_eq!(out["disposition"], "RESPONSE_MISMATCH_UNISOLATED");
    }
    #[test]
    fn negative_control_cannot_change_the_response_envelope() {
        let c = corpus();
        let (_, out) = evaluate_case(&c["cases"][3]).unwrap();
        assert_eq!(out["disposition"], "NO_CONTEXT_CONFOUNDING");
    }
    #[test]
    fn post_prediction_context_and_overlapping_clock_are_not_accepted() {
        let c = corpus();
        assert_eq!(
            evaluate_case(&c["cases"][5]).unwrap().1["disposition"],
            "INVALID_CONTRACT"
        );
        assert_eq!(
            evaluate_case(&c["cases"][6]).unwrap().1["disposition"],
            "NOT_EVALUATED"
        );
    }
    #[test]
    fn stale_ood_and_derived_context_are_not_used() {
        let c = corpus();
        for i in [4, 7, 8] {
            assert_eq!(
                evaluate_case(&c["cases"][i]).unwrap().1["disposition"],
                "NOT_EVALUATED"
            );
        }
    }
    #[test]
    fn unqualified_context_path_cannot_change_expected_envelope() {
        let c = corpus();
        let (_, out) = evaluate_case(&c["cases"][9]).unwrap();
        assert_eq!(out["disposition"], "INVALID_CONTRACT");
    }
    #[test]
    fn timebase_validity_covers_response_measurement() {
        let mut c = corpus();
        c["cases"][0]["timebase"]["valid_until_ms"] = json!(1500);
        let (_, out) = evaluate_case(&c["cases"][0]).unwrap();
        assert_eq!(out["disposition"], "NOT_EVALUATED");
    }
}
