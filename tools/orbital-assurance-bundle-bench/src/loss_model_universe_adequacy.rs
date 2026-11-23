//! Independent finite loss-model universe and world/model applicability adequacy.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const SCHEMA: &str = "luminous.orbital-loss-model-universe-adequacy.corpus.v0.1";
const CEILING: &str = "synthetic-loss-model-universe-adequacy-only";
const MAX_CASES: usize = 128;
const MAX_WORLDS: usize = 4096;
const MAX_MODELS: usize = 1024;
const MAX_PAIRS: usize = 16384;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct World {
    pub world_id: String,
    pub profile_tags: Vec<String>,
    pub scope_sha256: String,
}
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Model {
    pub model_id: String,
    pub family: String,
    pub version: String,
    pub scope_sha256: String,
    pub source_receipt_id: Option<String>,
    pub state: String,
}
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Applicability {
    pub world_id: String,
    pub model_id: String,
    pub declared_applicable: bool,
    pub exclusion_reason: Option<String>,
    pub evidence_receipt_id: Option<String>,
}
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct JointPair {
    pub scenario_id: String,
    pub world_id: String,
    pub model_id: String,
}
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Expected {
    pub disposition: String,
    pub missing_model_ids: Vec<String>,
    pub missing_pair_ids: Vec<String>,
    pub unexpected_model_ids: Vec<String>,
    pub unexpected_pair_ids: Vec<String>,
}
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub case_id: String,
    pub mission_class: String,
    pub scope_sha256: String,
    pub worlds: Vec<World>,
    pub producer_models: Vec<Model>,
    pub declared_model_ids: Vec<String>,
    pub applicability: Vec<Applicability>,
    pub joint_pairs: Vec<JointPair>,
    pub expected: Expected,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    pub schema: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub cases: Vec<Case>,
}
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Finding {
    pub code: String,
    pub model_id: Option<String>,
    pub pair_id: Option<String>,
    pub detail: String,
}
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CaseResult {
    pub case_id: String,
    pub disposition: String,
    pub expected_disposition: String,
    pub missing_model_ids: Vec<String>,
    pub missing_pair_ids: Vec<String>,
    pub unexpected_model_ids: Vec<String>,
    pub unexpected_pair_ids: Vec<String>,
    pub unwitnessed_exclusion_count: usize,
    pub findings: Vec<Finding>,
    pub producer_model_count: usize,
    pub declared_model_count: usize,
    pub expected_model_count: usize,
    pub expected_pair_count: usize,
    pub actual_pair_count: usize,
    pub expected_result_matched: bool,
}
#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub case_count: usize,
    pub closed_cases: usize,
    pub common_mode_model_omission_cases: usize,
    pub missing_model_cases: usize,
    pub missing_pair_cases: usize,
    pub unwitnessed_exclusion_cases: usize,
    pub model_family_inadequate_cases: usize,
    pub stale_model_cases: usize,
    pub out_of_profile_model_cases: usize,
    pub unexpected_model_cases: usize,
    pub unexpected_pair_cases: usize,
    pub invalid_cases: usize,
    pub expected_result_mismatches: usize,
}
#[derive(Debug, Clone, Serialize)]
pub struct Evaluation {
    pub result: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub summary: Summary,
    pub cases: Vec<CaseResult>,
}

#[derive(Clone, Copy)]
struct Requirement {
    family: &'static str,
    id: &'static str,
    tag: Option<&'static str>,
}
fn requirements(mission: &str) -> Option<&'static [Requirement]> {
    const ORBITAL: &[Requirement] = &[
        Requirement {
            family: "propellant_consumption",
            id: "propellant-v1",
            tag: None,
        },
        Requirement {
            family: "thermal_stress",
            id: "thermal-v1",
            tag: Some("thermal"),
        },
        Requirement {
            family: "propulsion_degradation",
            id: "propulsion-v1",
            tag: Some("propulsion"),
        },
        Requirement {
            family: "avionics_reliability",
            id: "avionics-v1",
            tag: None,
        },
    ];
    const FLIGHT: &[Requirement] = &[
        Requirement {
            family: "propellant_consumption",
            id: "propellant-v1",
            tag: None,
        },
        Requirement {
            family: "thermal_stress",
            id: "thermal-v1",
            tag: Some("thermal"),
        },
        Requirement {
            family: "avionics_reliability",
            id: "avionics-v1",
            tag: None,
        },
    ];
    match mission {
        "orbital_vehicle" => Some(ORBITAL),
        "powered_flight" => Some(FLIGHT),
        _ => None,
    }
}
fn valid_id(s: &str) -> bool {
    !s.trim().is_empty() && s.bytes().all(|b| b.is_ascii_graphic() || b == b' ')
}
fn valid_sha(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
fn pair_id(world: &str, model: &str) -> String {
    format!("{world}::{model}")
}
fn finding(code: &str, model: Option<&str>, pair: Option<&str>, detail: &str) -> Finding {
    Finding {
        code: code.to_owned(),
        model_id: model.map(str::to_owned),
        pair_id: pair.map(str::to_owned),
        detail: detail.to_owned(),
    }
}
fn invalid(case: &Case, detail: &str) -> CaseResult {
    CaseResult {
        case_id: case.case_id.clone(),
        disposition: "INVALID_CASE".into(),
        expected_disposition: case.expected.disposition.clone(),
        missing_model_ids: vec![],
        missing_pair_ids: vec![],
        unexpected_model_ids: vec![],
        unexpected_pair_ids: vec![],
        unwitnessed_exclusion_count: 0,
        findings: vec![finding("INVALID_CASE", None, None, detail)],
        producer_model_count: case.producer_models.len(),
        declared_model_count: case.declared_model_ids.len(),
        expected_model_count: 0,
        expected_pair_count: 0,
        actual_pair_count: case.joint_pairs.len(),
        expected_result_matched: case.expected.disposition == "INVALID_CASE",
    }
}
fn evaluate_case(case: &Case) -> CaseResult {
    let Some(reqs) = requirements(&case.mission_class) else {
        return CaseResult {
            case_id: case.case_id.clone(),
            disposition: "MODEL_FAMILY_INADEQUATE".into(),
            expected_disposition: case.expected.disposition.clone(),
            missing_model_ids: vec![],
            missing_pair_ids: vec![],
            unexpected_model_ids: vec![],
            unexpected_pair_ids: vec![],
            unwitnessed_exclusion_count: 0,
            findings: vec![finding(
                "MODEL_FAMILY_INADEQUATE",
                None,
                None,
                "mission class has no independent model-family grammar",
            )],
            producer_model_count: case.producer_models.len(),
            declared_model_count: case.declared_model_ids.len(),
            expected_model_count: 0,
            expected_pair_count: 0,
            actual_pair_count: case.joint_pairs.len(),
            expected_result_matched: case.expected.disposition == "MODEL_FAMILY_INADEQUATE"
                && case.expected.missing_model_ids.is_empty()
                && case.expected.missing_pair_ids.is_empty()
                && case.expected.unexpected_model_ids.is_empty()
                && case.expected.unexpected_pair_ids.is_empty(),
        };
    };
    if !valid_id(&case.case_id)
        || !valid_sha(&case.scope_sha256)
        || case.worlds.is_empty()
        || case.worlds.len() > MAX_WORLDS
        || case.producer_models.len() > MAX_MODELS
        || case.declared_model_ids.len() > MAX_MODELS
        || case.applicability.len() > MAX_PAIRS
        || case.joint_pairs.len() > MAX_PAIRS
    {
        return invalid(case, "invalid scope, identity or finite bounds");
    }

    let mut worlds = BTreeMap::new();
    for w in &case.worlds {
        if !valid_id(&w.world_id)
            || !valid_sha(&w.scope_sha256)
            || w.scope_sha256 != case.scope_sha256
            || w.profile_tags.iter().any(|x| !valid_id(x))
            || worlds.insert(w.world_id.as_str(), w).is_some()
        {
            return invalid(case, "malformed, duplicate or out-of-scope physical world");
        }
    }
    let mut findings = vec![];
    let mut models = BTreeMap::new();
    let mut receipt_owners = BTreeMap::new();
    for m in &case.producer_models {
        if !valid_id(&m.model_id)
            || !valid_id(&m.family)
            || !valid_id(&m.version)
            || !valid_sha(&m.scope_sha256)
            || !matches!(
                m.state.as_str(),
                "CURRENT" | "STALE" | "OUT_OF_PROFILE" | "INDETERMINATE"
            )
            || models.insert(m.model_id.as_str(), m).is_some()
        {
            return invalid(case, "malformed or duplicate producer model");
        }
        if m.scope_sha256 != case.scope_sha256 {
            findings.push(finding(
                "MODEL_SCOPE_MISMATCH",
                Some(&m.model_id),
                None,
                "producer model scope differs from the campaign scope",
            ));
        }
        if let Some(receipt_id) = m.source_receipt_id.as_deref() {
            if let Some(owner) = receipt_owners.insert(receipt_id, m.model_id.as_str()) {
                if owner != m.model_id {
                    findings.push(finding(
                        "MODEL_RECEIPT_ALIAS",
                        Some(&m.model_id),
                        None,
                        "one source receipt ID is reused for distinct consequence models",
                    ));
                }
            }
        }
    }
    let req_by_id = reqs.iter().map(|r| (r.id, r)).collect::<BTreeMap<_, _>>();
    let expected_models = reqs
        .iter()
        .map(|r| r.id.to_owned())
        .collect::<BTreeSet<_>>();
    let producer_ids = models
        .keys()
        .map(|s| s.to_string())
        .collect::<BTreeSet<_>>();
    let declared_ids = case
        .declared_model_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    if declared_ids.len() != case.declared_model_ids.len()
        || declared_ids.iter().any(|x| !valid_id(x))
    {
        return invalid(case, "duplicate or malformed declared model ID");
    }
    let mut missing_models = BTreeSet::new();
    let mut unexpected_models = BTreeSet::new();
    for id in &expected_models {
        let p = producer_ids.contains(id);
        let d = declared_ids.contains(id);
        if !p && !d {
            missing_models.insert(id.clone());
            findings.push(finding(
                "COMMON_MODE_MODEL_OMISSION",
                Some(id),
                None,
                "independent grammar requires a model omitted by both source inventories",
            ));
        } else if !p || !d {
            missing_models.insert(id.clone());
            findings.push(finding(
                "MISSING_LOSS_MODEL",
                Some(id),
                None,
                "required model missing from producer or declaration inventory",
            ));
        }
    }
    for id in producer_ids.union(&declared_ids) {
        if !expected_models.contains(id) {
            unexpected_models.insert(id.clone());
            findings.push(finding(
                "UNEXPECTED_MODEL",
                Some(id),
                None,
                "model is outside the independently registered family grammar",
            ));
        }
    }
    for (id, model) in &models {
        let code = match model.state.as_str() {
            "STALE" => Some("MODEL_STALE"),
            "OUT_OF_PROFILE" => Some("MODEL_OUT_OF_PROFILE"),
            "INDETERMINATE" => Some("MODEL_SOURCE_UNAVAILABLE"),
            _ => None,
        };
        if let Some(code) = code {
            findings.push(finding(
                code,
                Some(id),
                None,
                "model state prevents closed model-family evidence",
            ));
        }
        if model
            .source_receipt_id
            .as_deref()
            .is_none_or(|s| !valid_id(s))
        {
            findings.push(finding(
                "MODEL_SOURCE_UNAVAILABLE",
                Some(id),
                None,
                "model lacks a source receipt identity",
            ));
        }
    }

    let mut expected_pairs = BTreeSet::new();
    for w in &case.worlds {
        for req in reqs {
            if req
                .tag
                .is_none_or(|tag| w.profile_tags.iter().any(|v| v == tag))
            {
                expected_pairs.insert(pair_id(&w.world_id, req.id));
            }
        }
    }
    let mut applicability = BTreeMap::new();
    let mut unwitnessed = 0usize;
    for a in &case.applicability {
        if !worlds.contains_key(a.world_id.as_str())
            || !valid_id(&a.model_id)
            || applicability
                .insert((a.world_id.as_str(), a.model_id.as_str()), a)
                .is_some()
        {
            return invalid(case, "unknown or duplicate applicability identity");
        }
        if !req_by_id.contains_key(a.model_id.as_str()) {
            findings.push(finding(
                "UNEXPECTED_APPLICABILITY",
                Some(&a.model_id),
                Some(&pair_id(&a.world_id, &a.model_id)),
                "applicability references an unknown model",
            ));
        }
    }
    // The applicability manifest is total over every world × required model pair.
    // A non-applicable pair is still a declared exclusion and needs evidence.
    for world in &case.worlds {
        for req in reqs {
            if !producer_ids.contains(req.id) || !declared_ids.contains(req.id) {
                continue;
            }
            let key = (world.world_id.as_str(), req.id);
            let pair = pair_id(world.world_id.as_str(), req.id);
            let applies = req
                .tag
                .is_none_or(|tag| world.profile_tags.iter().any(|v| v == tag));
            let Some(row) = applicability.get(&key) else {
                if applies {
                    findings.push(finding(
                        "MISSING_WORLD_MODEL_PAIR",
                        Some(req.id),
                        Some(&pair),
                        "required applicability row is absent from the manifest",
                    ));
                } else {
                    unwitnessed += 1;
                    findings.push(finding(
                        "UNWITNESSED_MODEL_EXCLUSION",
                        Some(req.id),
                        Some(&pair),
                        "non-applicable model has no explicit witnessed exclusion row",
                    ));
                }
                continue;
            };
            if applies && !row.declared_applicable {
                unwitnessed += 1;
                findings.push(finding(
                    "UNWITNESSED_MODEL_EXCLUSION",
                    Some(req.id),
                    Some(&pair),
                    "grammar requires the model in this world but manifest excludes it",
                ));
            } else if !applies && row.declared_applicable {
                findings.push(finding(
                    "UNEXPECTED_WORLD_MODEL_PAIR",
                    Some(req.id),
                    Some(&pair),
                    "manifest declares a pair excluded by profile grammar",
                ));
            } else if !applies {
                let witnessed = row.exclusion_reason.as_deref() == Some("profile_not_applicable")
                    && row.evidence_receipt_id.as_deref().is_some_and(valid_id);
                if !witnessed {
                    unwitnessed += 1;
                    findings.push(finding(
                        "UNWITNESSED_MODEL_EXCLUSION",
                        Some(req.id),
                        Some(&pair),
                        "profile exclusion lacks reason or evidence receipt",
                    ));
                }
            }
        }
    }
    let declared_pairs = case
        .applicability
        .iter()
        .filter(|a| a.declared_applicable)
        .map(|a| pair_id(&a.world_id, &a.model_id))
        .collect::<BTreeSet<_>>();
    let mut actual_pairs = BTreeSet::new();
    for p in &case.joint_pairs {
        if !valid_id(&p.scenario_id)
            || !worlds.contains_key(p.world_id.as_str())
            || !valid_id(&p.model_id)
            || !actual_pairs.insert(pair_id(&p.world_id, &p.model_id))
        {
            return invalid(case, "malformed or duplicate joint scenario pair");
        }
    }
    let mut missing_pairs = BTreeSet::new();
    for p in &expected_pairs {
        let (_, model_id) = p.split_once("::").expect("pair format");
        if !producer_ids.contains(model_id) || !declared_ids.contains(model_id) {
            continue;
        }
        if !declared_pairs.contains(p) || !actual_pairs.contains(p) {
            missing_pairs.insert(p.clone());
            findings.push(finding("MISSING_WORLD_MODEL_PAIR", Some(model_id), Some(p), "independent grammar requires a world-model pair missing from manifest or joint scenarios"));
        }
    }
    for p in actual_pairs.difference(&expected_pairs) {
        let model_id = p.split_once("::").map(|(_, m)| m);
        findings.push(finding(
            "UNEXPECTED_WORLD_MODEL_PAIR",
            model_id,
            Some(p),
            "joint scenario is outside applicability grammar",
        ));
    }

    findings.sort_by(|a, b| {
        (&a.code, &a.model_id, &a.pair_id, &a.detail).cmp(&(
            &b.code,
            &b.model_id,
            &b.pair_id,
            &b.detail,
        ))
    });
    findings.dedup();
    let codes = findings
        .iter()
        .map(|f| f.code.as_str())
        .collect::<BTreeSet<_>>();
    let disposition = if codes.contains("MODEL_STALE") {
        "MODEL_STALE"
    } else if codes.contains("MODEL_OUT_OF_PROFILE") {
        "MODEL_OUT_OF_PROFILE"
    } else if codes.contains("MODEL_SOURCE_UNAVAILABLE") {
        "MODEL_SOURCE_UNAVAILABLE"
    } else if codes.contains("MODEL_SCOPE_MISMATCH") {
        "MODEL_SCOPE_MISMATCH"
    } else if codes.contains("MODEL_RECEIPT_ALIAS") {
        "MODEL_RECEIPT_ALIAS"
    } else if codes.contains("COMMON_MODE_MODEL_OMISSION") {
        "COMMON_MODE_MODEL_OMISSION"
    } else if codes.contains("MISSING_LOSS_MODEL") {
        "MISSING_LOSS_MODEL"
    } else if codes.contains("UNWITNESSED_MODEL_EXCLUSION") {
        "UNWITNESSED_MODEL_EXCLUSION"
    } else if codes.contains("MISSING_WORLD_MODEL_PAIR") {
        "MISSING_WORLD_MODEL_PAIR"
    } else if codes.contains("UNEXPECTED_MODEL") {
        "UNEXPECTED_MODEL"
    } else if codes.contains("UNEXPECTED_WORLD_MODEL_PAIR")
        || codes.contains("UNEXPECTED_APPLICABILITY")
    {
        "UNEXPECTED_WORLD_MODEL_PAIR"
    } else {
        "LOSS_MODEL_UNIVERSE_CLOSED_WITHIN_DECLARED_GRAMMAR"
    };
    let missing_model_ids = missing_models.into_iter().collect::<Vec<_>>();
    let missing_pair_ids = missing_pairs.into_iter().collect::<Vec<_>>();
    let unexpected_model_ids = unexpected_models.into_iter().collect::<Vec<_>>();
    let unexpected_pair_ids = actual_pairs
        .difference(&expected_pairs)
        .cloned()
        .collect::<Vec<_>>();
    let expected_result_matched = disposition == case.expected.disposition
        && missing_model_ids == case.expected.missing_model_ids
        && missing_pair_ids == case.expected.missing_pair_ids
        && unexpected_model_ids == case.expected.unexpected_model_ids
        && unexpected_pair_ids == case.expected.unexpected_pair_ids;
    CaseResult {
        case_id: case.case_id.clone(),
        disposition: disposition.to_owned(),
        expected_disposition: case.expected.disposition.clone(),
        missing_model_ids,
        missing_pair_ids,
        unexpected_model_ids,
        unexpected_pair_ids,
        unwitnessed_exclusion_count: unwitnessed,
        findings,
        producer_model_count: case.producer_models.len(),
        declared_model_count: case.declared_model_ids.len(),
        expected_model_count: expected_models.len(),
        expected_pair_count: expected_pairs.len(),
        actual_pair_count: actual_pairs.len(),
        expected_result_matched,
    }
}

pub fn evaluate(corpus: &Corpus) -> Result<Evaluation, String> {
    if corpus.schema != SCHEMA
        || corpus.evidence_class != "synthetic"
        || corpus.claim_ceiling != CEILING
        || corpus.cases.is_empty()
        || corpus.cases.len() > MAX_CASES
    {
        return Err("unsupported loss-model universe corpus".to_owned());
    }
    let mut ids = BTreeSet::new();
    let mut cases = vec![];
    for c in &corpus.cases {
        if !ids.insert(c.case_id.as_str()) {
            return Err("duplicate loss-model universe case ID".to_owned());
        }
        cases.push(evaluate_case(c));
    }
    cases.sort_by(|a, b| a.case_id.cmp(&b.case_id));
    let count = |s: &str| cases.iter().filter(|c| c.disposition == s).count();
    let summary = Summary {
        case_count: cases.len(),
        closed_cases: count("LOSS_MODEL_UNIVERSE_CLOSED_WITHIN_DECLARED_GRAMMAR"),
        common_mode_model_omission_cases: count("COMMON_MODE_MODEL_OMISSION"),
        missing_model_cases: count("MISSING_LOSS_MODEL"),
        missing_pair_cases: count("MISSING_WORLD_MODEL_PAIR"),
        unwitnessed_exclusion_cases: count("UNWITNESSED_MODEL_EXCLUSION"),
        model_family_inadequate_cases: count("MODEL_FAMILY_INADEQUATE"),
        stale_model_cases: count("MODEL_STALE"),
        out_of_profile_model_cases: count("MODEL_OUT_OF_PROFILE"),
        unexpected_model_cases: count("UNEXPECTED_MODEL"),
        unexpected_pair_cases: count("UNEXPECTED_WORLD_MODEL_PAIR"),
        invalid_cases: count("INVALID_CASE"),
        expected_result_mismatches: cases.iter().filter(|c| !c.expected_result_matched).count(),
    };
    let result = if summary.expected_result_mismatches == 0
        && summary.closed_cases >= 1
        && summary.common_mode_model_omission_cases >= 1
        && summary.unwitnessed_exclusion_cases >= 1
    {
        "pass"
    } else {
        "fail"
    };
    Ok(Evaluation {
        result: result.into(),
        evidence_class: "synthetic".into(),
        claim_ceiling: CEILING.into(),
        summary,
        cases,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Corpus {
        serde_json::from_str(include_str!(
            "../fixtures/v0.1/loss-model-universe-cases.json"
        ))
        .expect("corpus parses")
    }
    fn case<'a>(e: &'a Evaluation, id: &str) -> &'a CaseResult {
        e.cases
            .iter()
            .find(|c| c.case_id == id)
            .expect("case exists")
    }
    #[test]
    fn model_scope_mismatch_is_explicit() {
        let e = evaluate(&fixture()).expect("evaluate");
        assert_eq!(
            case(&e, "model_scope_mismatch").disposition,
            "MODEL_SCOPE_MISMATCH"
        );
    }

    #[test]
    fn source_receipt_alias_across_models_is_blocked() {
        let e = evaluate(&fixture()).expect("evaluate");
        assert_eq!(
            case(&e, "model_receipt_alias").disposition,
            "MODEL_RECEIPT_ALIAS"
        );
    }

    #[test]
    fn applicability_manifest_is_total_over_required_world_model_pairs() {
        let e = evaluate(&fixture()).expect("evaluate");
        assert_eq!(
            case(&e, "missing_required_applicability_row").disposition,
            "MISSING_WORLD_MODEL_PAIR"
        );
        assert_eq!(
            case(&e, "missing_exclusion_row").disposition,
            "UNWITNESSED_MODEL_EXCLUSION"
        );
    }

    #[test]
    fn unexpected_models_and_pairs_are_visible() {
        let e = evaluate(&fixture()).expect("evaluate");
        assert_eq!(case(&e, "unexpected_model").disposition, "UNEXPECTED_MODEL");
        assert_eq!(
            case(&e, "unexpected_pair").disposition,
            "UNEXPECTED_WORLD_MODEL_PAIR"
        );
    }
    #[test]
    fn permutation_invariant() {
        let expected = evaluate(&fixture()).expect("evaluate");
        let mut c = fixture();
        for x in &mut c.cases {
            x.worlds.reverse();
            x.producer_models.reverse();
            x.declared_model_ids.reverse();
            x.applicability.reverse();
            x.joint_pairs.reverse();
            for w in &mut x.worlds {
                w.profile_tags.reverse();
            }
        }
        let actual = evaluate(&c).expect("permuted");
        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            serde_json::to_value(expected).unwrap()
        );
    }
}
