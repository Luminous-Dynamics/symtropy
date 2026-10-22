use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::{BTreeMap, BTreeSet}, env, fs, process::ExitCode};

const VERSION: &str = "0.1.0";
const CORPUS_ID: &str = "terrain-mobility-portfolio-v0.1";
const CLAIM_CEILING: &str = "synthetic_fixtures_are_not_platform_performance_or_safety_evidence";
const CASES: usize = 37;
const NEGATIVES: usize = 15;
const EPS: f64 = 1.0e-9;

#[derive(Debug, Deserialize)]
struct Corpus {
    schema_version: String,
    corpus_id: String,
    provenance: Provenance,
    claim_ceiling: String,
    scenarios: Vec<Scenario>,
}
#[derive(Debug, Deserialize)]
struct Provenance {
    created_on: String,
    kind: String,
    external_measurements_included: bool,
}
#[derive(Debug, Deserialize, Clone)]
struct Scenario {
    scenario_id: String,
    domain: String,
    terrain: Terrain,
    environment: Environment,
    mission: Mission,
    required_gates: Vec<String>,
    metric_tracks: Vec<String>,
    expected_behavior: String,
    negative_control: bool,
}
#[derive(Debug, Deserialize, Clone)]
struct Terrain {
    surface: String,
    surface_uncertainty: String,
    drop_or_void_hazard: bool,
}
#[derive(Debug, Deserialize, Clone)]
struct Environment {
    localization: String,
    communications: String,
    faults: Vec<String>,
}
#[derive(Debug, Deserialize, Clone)]
struct Mission {
    return_required: bool,
    energy_budget_wh: Option<f64>,
    estimated_return_energy_wh: Option<f64>,
    return_margin_requirement_pct: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Disposition { Run, Retreat, Reject, AskHuman, Unknown }
impl Disposition {
    fn text(self) -> &'static str {
        match self {
            Self::Run => "run_within_declared_envelope",
            Self::Retreat => "abort_or_retreat_safely",
            Self::Reject => "reject_as_out_of_domain",
            Self::AskHuman => "require_explicit_human_choice",
            Self::Unknown => "keep_unknown_unresolved",
        }
    }
}

#[derive(Debug, Serialize)]
struct CaseResult {
    scenario_id: String,
    expected_behavior: String,
    independently_derived_behavior: String,
    negative_control: bool,
    energy_budget_wh: Option<f64>,
    estimated_return_energy_wh: Option<f64>,
    required_return_energy_with_margin_wh: Option<f64>,
    return_energy_feasible: Option<bool>,
    result: &'static str,
    findings: Vec<String>,
}
#[derive(Debug, Serialize)]
struct Receipt {
    audit_version: &'static str,
    result: &'static str,
    corpus_id: String,
    schema_version: String,
    source_commit: String,
    canonical_corpus_sha256: String,
    scenario_count: usize,
    negative_control_count: usize,
    decision_counts: BTreeMap<String, usize>,
    checks_performed: Vec<String>,
    scenarios: Vec<CaseResult>,
    errors: Vec<String>,
    claim_ceiling: String,
}

fn fault(s: &Scenario, name: &str) -> bool {
    s.environment.faults.iter().any(|x| x == name)
}
fn gate(s: &Scenario, name: &str) -> bool {
    s.required_gates.iter().any(|x| x == name)
}
fn return_requirement(s: &Scenario) -> Option<f64> {
    if !s.mission.return_required { return None; }
    let estimate = s.mission.estimated_return_energy_wh?;
    let margin = s.mission.return_margin_requirement_pct?;
    if !estimate.is_finite() || estimate <= 0.0 ||
        !margin.is_finite() || !(0.0..=100.0).contains(&margin) { return None; }
    let result = estimate * (1.0 + margin / 100.0);
    result.is_finite().then_some(result)
}
fn energy_feasible(s: &Scenario) -> Option<bool> {
    if !s.mission.return_required { return None; }
    let budget = s.mission.energy_budget_wh?;
    let required = return_requirement(s)?;
    if !budget.is_finite() || budget < 0.0 { return None; }
    Some(budget + EPS >= required)
}

// This oracle derives its output only from explicit scenario inputs; the
// declared expected_behavior is compared afterward, never consumed here.
fn derive(s: &Scenario) -> Disposition {
    if fault(s, "owner_profile_conflict") { return Disposition::AskHuman; }
    if fault(s, "parts_availability_unknown") { return Disposition::Unknown; }

    if ["missing_calibration", "configuration_mismatch", "missing_capability_evidence"]
        .iter().any(|f| fault(s, f))
        || gate(s, "no_unauthorized_water_entry")
        || (s.terrain.surface == "loose_slope" && s.terrain.surface_uncertainty == "unknown") {
        return Disposition::Reject;
    }
    if s.terrain.surface == "unknown" && s.terrain.drop_or_void_hazard {
        return Disposition::Retreat;
    }
    if s.terrain.surface == "unknown" || s.terrain.surface_uncertainty == "unknown" {
        return Disposition::Reject;
    }
    if ["tire_pressure_loss", "battery_estimator_conflict", "low_energy_reserve",
        "lidar_dropout", "wheel_encoder_dropout", "communications_loss", "actuator_fault",
        "stale_map"].iter().any(|f| fault(s, f))
        || s.environment.localization != "nominal"
        || s.environment.communications != "connected"
        || s.terrain.drop_or_void_hazard {
        return Disposition::Retreat;
    }

    if s.mission.return_required {
        let (Some(budget), Some(required)) =
            (s.mission.energy_budget_wh, return_requirement(s)) else {
                return Disposition::Unknown;
            };
        if !budget.is_finite() || budget < 0.0 { return Disposition::Unknown; }
        if budget + EPS < required { return Disposition::Retreat; }
    }
    Disposition::Run
}

fn digest(raw: &str) -> Result<String, String> {
    let mut value: Value = serde_json::from_str(raw).map_err(|e| format!("invalid JSON: {e}"))?;
    let scenarios = value.get_mut("scenarios").and_then(Value::as_array_mut)
        .ok_or_else(|| "missing scenarios array".to_string())?;
    scenarios.sort_by(|a, b|
        a.get("scenario_id").and_then(Value::as_str).unwrap_or("")
            .cmp(b.get("scenario_id").and_then(Value::as_str).unwrap_or("")));
    let canonical = serde_json::to_vec(&value).map_err(|e| e.to_string())?;
    Ok(Sha256::digest(canonical).iter().map(|b| format!("{b:02x}")).collect())
}
fn valid_commit(s: &str) -> bool {
    s.len() == 40 && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn valid_date(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10 && b.iter().enumerate().all(|(i, x)| {
        if i == 4 || i == 7 { *x == b'-' } else { x.is_ascii_digit() }
    })
}
fn validate(s: &Scenario) -> Vec<String> {
    let mut out = Vec::new();
    let id = &s.scenario_id;
    if id.len() < 3 || !id.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'-') {
        out.push(format!("{id}: invalid scenario ID"));
    }
    if !["personal_vehicle", "ground_robotics", "cross_platform"].contains(&s.domain.as_str()) {
        out.push(format!("{id}: unsupported domain"));
    }
    if s.required_gates.is_empty() { out.push(format!("{id}: required_gates is empty")); }
    if s.metric_tracks.is_empty() { out.push(format!("{id}: metric_tracks is empty")); }
    let disposition = derive(s);
    if disposition.text() != s.expected_behavior {
        out.push(format!("{id}: oracle derived {}, corpus declares {}",
            disposition.text(), s.expected_behavior));
    }
    if s.negative_control && disposition == Disposition::Run {
        out.push(format!("{id}: negative control was accepted for run"));
    }
    if disposition == Disposition::Run &&
       (s.terrain.drop_or_void_hazard || s.terrain.surface == "unknown" ||
        s.terrain.surface_uncertainty == "unknown") {
        out.push(format!("{id}: hazard/unknown terrain cannot be dispatched"));
    }

    if s.mission.return_required {
        if !s.mission.energy_budget_wh.is_some_and(|v| v.is_finite() && v > 0.0) {
            out.push(format!("{id}: return requires positive energy budget"));
        }
        if !s.mission.estimated_return_energy_wh.is_some_and(|v| v.is_finite() && v > 0.0) {
            out.push(format!("{id}: return requires positive estimated return energy"));
        }
        if !s.mission.return_margin_requirement_pct
            .is_some_and(|v| v.is_finite() && (0.0..=100.0).contains(&v)) {
            out.push(format!("{id}: return requires margin between 0 and 100"));
        }
        if energy_feasible(s) == Some(false) && disposition == Disposition::Run {
            out.push(format!("{id}: return budget is below estimate plus margin"));
        }
    } else if s.mission.estimated_return_energy_wh.is_some() {
        out.push(format!("{id}: non-return mission must set return estimate to null"));
    }
    out.sort();
    out
}
fn build_receipt(raw: &str, source_commit: &str) -> Result<Receipt, String> {
    if !valid_commit(source_commit) {
        return Err("source commit must be 40 lowercase hexadecimal characters".to_string());
    }
    let corpus: Corpus = serde_json::from_str(raw).map_err(|e| format!("invalid corpus: {e}"))?;
    let corpus_digest = digest(raw)?;
    let mut errors = Vec::new();

    if corpus.schema_version != "0.1" { errors.push("schema_version must be 0.1".to_string()); }
    if corpus.corpus_id != CORPUS_ID { errors.push("unexpected corpus_id".to_string()); }
    if corpus.provenance.kind != "hand_authored_synthetic_test_vectors" {
        errors.push("provenance kind must remain synthetic test vectors".to_string());
    }
    if corpus.provenance.external_measurements_included {
        errors.push("synthetic corpus must not claim external measurements".to_string());
    }
    if !valid_date(&corpus.provenance.created_on) { errors.push("invalid provenance date".to_string()); }
    if corpus.claim_ceiling != CLAIM_CEILING { errors.push("claim ceiling is missing or weakened".to_string()); }
    if corpus.scenarios.len() != CASES {
        errors.push(format!("expected {CASES} scenarios, got {}", corpus.scenarios.len()));
    }
    let mut ids = BTreeSet::new();
    for s in &corpus.scenarios {
        if !ids.insert(s.scenario_id.clone()) { errors.push(format!("duplicate ID: {}", s.scenario_id)); }
    }
    let negative_count = corpus.scenarios.iter().filter(|s| s.negative_control).count();
    if negative_count != NEGATIVES {
        errors.push(format!("expected {NEGATIVES} negative controls, got {negative_count}"));
    }

    let mut cases = Vec::with_capacity(corpus.scenarios.len());
    let mut counts = BTreeMap::new();
    for s in &corpus.scenarios {
        let d = derive(s);
        *counts.entry(d.text().to_string()).or_insert(0) += 1;
        let findings = validate(s);
        errors.extend(findings.iter().cloned());
        cases.push(CaseResult {
            scenario_id: s.scenario_id.clone(),
            expected_behavior: s.expected_behavior.clone(),
            independently_derived_behavior: d.text().to_string(),
            negative_control: s.negative_control,
            energy_budget_wh: s.mission.energy_budget_wh,
            estimated_return_energy_wh: s.mission.estimated_return_energy_wh,
            required_return_energy_with_margin_wh: return_requirement(s),
            return_energy_feasible: energy_feasible(s),
            result: if findings.is_empty() { "pass" } else { "fail" },
            findings,
        });
    }
    cases.sort_by(|a, b| a.scenario_id.cmp(&b.scenario_id));
    errors.sort();
    Ok(Receipt {
        audit_version: VERSION,
        result: if errors.is_empty() { "pass" } else { "fail" },
        corpus_id: corpus.corpus_id,
        schema_version: corpus.schema_version,
        source_commit: source_commit.to_string(),
        canonical_corpus_sha256: corpus_digest,
        scenario_count: cases.len(),
        negative_control_count: negative_count,
        decision_counts: counts,
        checks_performed: vec![
            "corpus identity, provenance, claim ceiling, and frozen case count".to_string(),
            "scenario ID uniqueness and required contract fields".to_string(),
            "independent disposition oracle compared with declared expectation".to_string(),
            "return energy estimate plus margin compared with available budget".to_string(),
            "negative controls retained and not accepted for dispatch".to_string(),
            "canonical scenario-order-independent JSON digest".to_string(),
        ],
        scenarios: cases,
        errors,
        claim_ceiling: CLAIM_CEILING.to_string(),
    })
}
fn main() -> ExitCode {
    let mut a = env::args().skip(1);
    let Some(path) = a.next() else {
        eprintln!("usage: terrain-mobility-audit <corpus.json> --source-commit <40-hex-sha>");
        return ExitCode::FAILURE;
    };
    let Some(flag) = a.next() else { eprintln!("missing --source-commit"); return ExitCode::FAILURE; };
    let Some(commit) = a.next() else { eprintln!("missing commit SHA"); return ExitCode::FAILURE; };
    if flag != "--source-commit" || a.next().is_some() {
        eprintln!("usage: terrain-mobility-audit <corpus.json> --source-commit <40-hex-sha>");
        return ExitCode::FAILURE;
    }
    let raw = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => { eprintln!("cannot read {path}: {e}"); return ExitCode::FAILURE; }
    };
    let receipt = match build_receipt(&raw, &commit) {
        Ok(r) => r,
        Err(e) => { eprintln!("terrain-mobility-audit: {e}"); return ExitCode::FAILURE; }
    };
    println!("{}", serde_json::to_string_pretty(&receipt).expect("receipt serializes"));
    if receipt.result == "pass" { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}

#[cfg(test)]
mod tests {
    use super::*;
    const INPUT: &str = include_str!("../../../fixtures/terrain_mobility/terrain_mobility_scenarios_v0_1.json");
    const COMMIT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn corpus() -> Corpus { serde_json::from_str(INPUT).expect("corpus parses") }
    fn find_mut<'a>(c: &'a mut Corpus, id: &str) -> &'a mut Scenario {
        c.scenarios.iter_mut().find(|s| s.scenario_id == id).expect("scenario exists")
    }

    #[test]
    fn frozen_corpus_passes_independent_oracle() {
        let r = build_receipt(INPUT, COMMIT).expect("receipt");
        assert_eq!(r.result, "pass", "{:?}", r.errors);
        assert_eq!(r.scenario_count, CASES);
        assert_eq!(r.negative_control_count, NEGATIVES);
        assert!(r.scenarios.iter().all(|s| s.result == "pass"));
    }
    #[test]
    fn long_return_shortfall_is_safe_abort() {
        let c = corpus();
        let s = c.scenarios.iter().find(|s| s.scenario_id == "UGV-LONG-RETURN-ENERGY").unwrap();
        assert_eq!(energy_feasible(s), Some(false));
        assert_eq!(derive(s), Disposition::Retreat);
    }
    #[test]
    fn missing_energy_estimate_is_unresolved_not_zero() {
        let mut c = corpus();
        let s = find_mut(&mut c, "PVT-URBAN-DRY-BASELINE");
        s.mission.estimated_return_energy_wh = None;
        assert_eq!(energy_feasible(s), None);
        assert_eq!(derive(s), Disposition::Unknown);
    }
    #[test]
    fn low_budget_aborts_nominal_surface_case() {
        let mut c = corpus();
        let s = find_mut(&mut c, "PVT-URBAN-DRY-BASELINE");
        s.mission.energy_budget_wh = Some(5_000.0);
        assert_eq!(energy_feasible(s), Some(false));
        assert_eq!(derive(s), Disposition::Retreat);
    }
    #[test]
    fn unknown_void_requires_retreat() {
        let c = corpus();
        let s = c.scenarios.iter().find(|s| s.scenario_id == "UGV-UNKNOWN-VOID-EDGE").unwrap();
        assert!(s.negative_control);
        assert_eq!(derive(s), Disposition::Retreat);
    }
    #[test]
    fn profile_conflict_requires_explicit_choice() {
        let c = corpus();
        let s = c.scenarios.iter().find(|s| s.scenario_id == "PVT-MULTI-DRIVER-CONFLICT").unwrap();
        assert_eq!(derive(s), Disposition::AskHuman);
    }
    #[test]
    fn canonical_receipt_is_invariant_to_scenario_order() {
        let original = build_receipt(INPUT, COMMIT).expect("original receipt");
        let mut v: Value = serde_json::from_str(INPUT).unwrap();
        v["scenarios"].as_array_mut().unwrap().reverse();
        let reversed = build_receipt(&serde_json::to_string(&v).unwrap(), COMMIT).unwrap();
        assert_eq!(serde_json::to_vec(&original).unwrap(), serde_json::to_vec(&reversed).unwrap());
    }
    #[test]
    fn rejects_invalid_source_commit() {
        assert!(build_receipt(INPUT, "not-a-sha").is_err());
    }
}
