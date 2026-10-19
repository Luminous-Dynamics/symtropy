// SPDX-License-Identifier: AGPL-3.0-or-later
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::{env, fs, process};

const RULESET_JSON: &str = include_str!("../ruleset-v0.1.json");
const GENERATOR_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Action {
    Answer,
    Clarify,
    Abstain,
    RejectSource,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum SourcePermission {
    Allowed,
    Denied,
    Unknown,
}

impl SourcePermission {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Allowed => "allowed",
            Self::Denied => "denied",
            Self::Unknown => "unknown",
        }
    }

    fn code(&self) -> char {
        match self {
            Self::Allowed => 'A',
            Self::Denied => 'D',
            Self::Unknown => 'U',
        }
    }

    fn phrase(&self) -> &'static str {
        match self {
            Self::Allowed => "explicitly permitted for this synthetic evaluation operation",
            Self::Denied => "explicitly denied for this synthetic evaluation operation",
            Self::Unknown => "not established for this synthetic evaluation operation",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RuleSet {
    schema_version: String,
    id: String,
    description: String,
    max_forward_chaining_steps: usize,
    fact_mappings: Vec<FactMapping>,
    rules: Vec<InferenceRule>,
    decisions: Vec<DecisionRule>,
    authorities: Vec<SyntheticAuthority>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FactMapping {
    scenario_field: String,
    value: String,
    facts: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct InferenceRule {
    id: String,
    requires: Vec<String>,
    produces: Vec<String>,
    rationale: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct DecisionRule {
    id: String,
    when_fact: String,
    priority: u32,
    action: Action,
    reason_code: String,
    claim_code: Option<String>,
    #[serde(default)]
    citations: Vec<Citation>,
    rationale: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SyntheticAuthority {
    id: String,
    kind: String,
    jurisdiction: String,
    validity: String,
    permitted_use: String,
    span_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
struct Citation {
    authority_id: String,
    role: String,
    span_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Scenario {
    source_permission: SourcePermission,
    competence_confirmed: bool,
    current_authority_available: bool,
    required_fact_present: bool,
    authority_conflict: bool,
    wording_variant: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct ExpectedResult {
    action: Action,
    reason_code: String,
    claim_codes: Vec<String>,
    citations: Vec<Citation>,
    derived_facts: Vec<String>,
    inference_trace: Vec<String>,
    applicable_decision_rule_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct GeneratedCase {
    id: String,
    family: String,
    partition: String,
    scenario: Scenario,
    prompt: String,
    expected: ExpectedResult,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum MetamorphicRelation {
    ActionMustChange,
    ActionMustMatch,
    EvidenceMustChange,
    OutputMustMatch,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct MetamorphicPair {
    id: String,
    source_case_id: String,
    target_case_id: String,
    relation: MetamorphicRelation,
    rationale: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct GeneratedCorpus {
    schema_version: String,
    dataset_kind: String,
    generator_version: String,
    ruleset_id: String,
    ruleset_sha256: String,
    seed: u64,
    cases: Vec<GeneratedCase>,
    metamorphic_pairs: Vec<MetamorphicPair>,
}

#[derive(Debug, Serialize)]
struct ValidationReceipt {
    status: &'static str,
    corpus_sha256: String,
    ruleset_sha256: String,
    case_count: usize,
    metamorphic_pair_count: usize,
    partitions: BTreeMap<String, usize>,
    claim_ceiling: &'static str,
}

fn main() {
    match run() {
        Ok(true) => {}
        Ok(false) => process::exit(1),
        Err(error) => {
            eprintln!("legal-scenario-compiler: {error}");
            process::exit(2);
        }
    }
}

fn run() -> Result<bool, String> {
    let mut args = env::args().skip(1);
    let command = args.next().ok_or_else(usage)?;
    let ruleset: RuleSet = serde_json::from_str(RULESET_JSON)
        .map_err(|error| format!("embedded ruleset is invalid: {error}"))?;
    validate_ruleset(&ruleset)?;

    match command.as_str() {
        "generate" => {
            let seed_text = args.next().ok_or_else(usage)?;
            let seed = seed_text
                .parse::<u64>()
                .map_err(|error| format!("invalid u64 seed {seed_text:?}: {error}"))?;
            let output_path = args.next().ok_or_else(usage)?;
            if args.next().is_some() {
                return Err(usage());
            }

            let corpus = generate_corpus(seed, &ruleset, RULESET_JSON)?;
            validate_corpus(&corpus, &ruleset, RULESET_JSON)?;
            let bytes = serialize_corpus(&corpus)?;
            fs::write(&output_path, &bytes)
                .map_err(|error| format!("cannot write {output_path}: {error}"))?;
            let receipt = validation_receipt(&corpus, &bytes);
            println!(
                "{}",
                serde_json::to_string_pretty(&receipt)
                    .map_err(|error| format!("cannot serialize receipt: {error}"))?
            );
            Ok(true)
        }
        "validate" => {
            let input_path = args.next().ok_or_else(usage)?;
            if args.next().is_some() {
                return Err(usage());
            }
            let raw = fs::read(&input_path)
                .map_err(|error| format!("cannot read {input_path}: {error}"))?;
            let corpus: GeneratedCorpus = serde_json::from_slice(&raw)
                .map_err(|error| format!("{input_path} is invalid JSON: {error}"))?;
            validate_corpus(&corpus, &ruleset, RULESET_JSON)?;
            let receipt = validation_receipt(&corpus, &raw);
            println!(
                "{}",
                serde_json::to_string_pretty(&receipt)
                    .map_err(|error| format!("cannot serialize receipt: {error}"))?
            );
            Ok(true)
        }
        _ => Err(usage()),
    }
}

fn usage() -> String {
    [
        "usage: legal-scenario-compiler generate <seed-u64> <output.json>",
        "       legal-scenario-compiler validate <corpus.json>",
    ]
    .join("\n")
}

fn validate_ruleset(ruleset: &RuleSet) -> Result<(), String> {
    let mut errors = Vec::new();
    if ruleset.schema_version != "0.1.0" {
        errors.push(format!(
            "unsupported ruleset schema version {}",
            ruleset.schema_version
        ));
    }
    if ruleset.id.trim().is_empty() || ruleset.description.trim().is_empty() {
        errors.push("ruleset id and description must not be empty".to_string());
    }
    if ruleset.max_forward_chaining_steps == 0 {
        errors.push("max_forward_chaining_steps must be nonzero".to_string());
    }

    let mut ids = BTreeSet::new();
    for rule in &ruleset.rules {
        if rule.id.trim().is_empty() || !ids.insert(rule.id.as_str()) {
            errors.push(format!("empty or duplicate inference rule id: {}", rule.id));
        }
        if rule.requires.is_empty() || rule.produces.is_empty() {
            errors.push(format!(
                "inference rule {} needs requires and produces",
                rule.id
            ));
        }
        if rule.rationale.trim().is_empty() {
            errors.push(format!("inference rule {} has empty rationale", rule.id));
        }
    }
    for decision in &ruleset.decisions {
        if decision.id.trim().is_empty() || !ids.insert(decision.id.as_str()) {
            errors.push(format!(
                "empty or duplicate rule/decision id: {}",
                decision.id
            ));
        }
        if decision.when_fact.trim().is_empty() || decision.reason_code.trim().is_empty() {
            errors.push(format!(
                "decision rule {} needs a condition and reason code",
                decision.id
            ));
        }
        if decision.rationale.trim().is_empty() {
            errors.push(format!("decision rule {} has empty rationale", decision.id));
        }
        if decision.action == Action::Answer && decision.claim_code.is_none() {
            errors.push(format!(
                "answer decision {} must carry a claim code",
                decision.id
            ));
        }
    }
    let mut authority_ids = BTreeSet::new();
    for authority in &ruleset.authorities {
        if authority.id.trim().is_empty() || !authority_ids.insert(authority.id.as_str()) {
            errors.push(format!(
                "empty or duplicate synthetic authority id: {}",
                authority.id
            ));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "ruleset validation failed:\n - {}",
            errors.join("\n - ")
        ))
    }
}

fn scenario_value(scenario: &Scenario, field: &str) -> Option<&'static str> {
    match field {
        "source_permission" => Some(scenario.source_permission.as_str()),
        "competence_confirmed" => Some(if scenario.competence_confirmed {
            "true"
        } else {
            "false"
        }),
        "current_authority_available" => Some(if scenario.current_authority_available {
            "true"
        } else {
            "false"
        }),
        "required_fact_present" => Some(if scenario.required_fact_present {
            "true"
        } else {
            "false"
        }),
        "authority_conflict" => Some(if scenario.authority_conflict {
            "true"
        } else {
            "false"
        }),
        _ => None,
    }
}

fn initial_facts(scenario: &Scenario, ruleset: &RuleSet) -> Result<BTreeSet<String>, String> {
    let mut facts = BTreeSet::new();
    for mapping in &ruleset.fact_mappings {
        let value = scenario_value(scenario, &mapping.scenario_field).ok_or_else(|| {
            format!(
                "unknown scenario field in ruleset mapping: {}",
                mapping.scenario_field
            )
        })?;
        if value == mapping.value {
            for fact in &mapping.facts {
                facts.insert(fact.clone());
            }
        }
    }
    Ok(facts)
}

fn derive_facts(
    scenario: &Scenario,
    ruleset: &RuleSet,
) -> Result<(BTreeSet<String>, Vec<String>), String> {
    let mut facts = initial_facts(scenario, ruleset)?;
    let mut trace = Vec::new();
    for _ in 0..ruleset.max_forward_chaining_steps {
        let mut new_facts = BTreeSet::new();
        for rule in &ruleset.rules {
            if rule.requires.iter().all(|fact| facts.contains(fact)) {
                for fact in &rule.produces {
                    if !facts.contains(fact) {
                        new_facts.insert(fact.clone());
                    }
                }
                if rule.produces.iter().any(|fact| !facts.contains(fact)) {
                    trace.push(rule.id.clone());
                }
            }
        }
        if new_facts.is_empty() {
            return Ok((facts, trace));
        }
        facts.extend(new_facts);
    }
    Err("forward chaining exceeded max_forward_chaining_steps".to_string())
}

fn evaluate_scenario(scenario: &Scenario, ruleset: &RuleSet) -> Result<ExpectedResult, String> {
    let (facts, inference_trace) = derive_facts(scenario, ruleset)?;
    let mut eligible: Vec<&DecisionRule> = ruleset
        .decisions
        .iter()
        .filter(|rule| facts.contains(&rule.when_fact))
        .collect();
    if eligible.is_empty() {
        return Err("no decision rule matched generated scenario".to_string());
    }
    let max_priority = eligible.iter().map(|rule| rule.priority).max().unwrap_or(0);
    eligible.retain(|rule| rule.priority == max_priority);
    eligible.sort_by(|left, right| left.id.cmp(&right.id));

    let first = eligible[0];
    let conflict = eligible.iter().skip(1).any(|rule| {
        rule.action != first.action
            || rule.claim_code != first.claim_code
            || rule.citations != first.citations
    });
    let (action, reason_code, claim_codes, citations, applicable_decision_rule_ids) = if conflict {
        (
            Action::Abstain,
            "decision_mapping_conflict".to_string(),
            Vec::new(),
            Vec::new(),
            eligible.iter().map(|rule| rule.id.clone()).collect(),
        )
    } else {
        (
            first.action.clone(),
            first.reason_code.clone(),
            first.claim_code.clone().into_iter().collect(),
            first.citations.clone(),
            eligible.iter().map(|rule| rule.id.clone()).collect(),
        )
    };

    Ok(ExpectedResult {
        action,
        reason_code,
        claim_codes,
        citations,
        derived_facts: facts.into_iter().collect(),
        inference_trace,
        applicable_decision_rule_ids,
    })
}

fn oracle_evaluate(scenario: &Scenario, ruleset: &RuleSet) -> ExpectedResult {
    let (action, reason_code, claim_codes, citations) =
        if scenario.source_permission == SourcePermission::Denied {
            (Action::RejectSource, "source_use_denied", vec![], vec![])
        } else if scenario.source_permission == SourcePermission::Unknown {
            (Action::Abstain, "source_permission_unknown", vec![], vec![])
        } else if scenario.authority_conflict {
            (
                Action::Abstain,
                "authority_conflict_unresolved",
                vec![],
                vec![],
            )
        } else if !scenario.required_fact_present {
            (Action::Clarify, "required_fact_missing", vec![], vec![])
        } else if !scenario.competence_confirmed {
            (Action::Clarify, "competence_unconfirmed", vec![], vec![])
        } else if !scenario.current_authority_available {
            (
                Action::Abstain,
                "authority_currentness_unverified",
                vec![],
                vec![],
            )
        } else {
            (
                Action::Answer,
                "answer_supported",
                vec!["SYNTHETIC_RULES_SUPPORT_ANSWER".to_string()],
                vec![Citation {
                    authority_id: "fictional-rulebook-v1".to_string(),
                    role: "binding_in_scope".to_string(),
                    span_id: "SYN-RULE-ANSWER-SUPPORTED-001".to_string(),
                }],
            )
        };
    let evaluated = evaluate_scenario(scenario, ruleset).expect("scenario evaluation");
    ExpectedResult {
        action,
        reason_code: reason_code.to_string(),
        claim_codes,
        citations,
        derived_facts: evaluated.derived_facts,
        inference_trace: evaluated.inference_trace,
        applicable_decision_rule_ids: evaluated.applicable_decision_rule_ids,
    }
}

fn case_id(scenario: &Scenario) -> String {
    format!(
        "GEN-P{}-C{}-A{}-F{}-X{}-W{}",
        scenario.source_permission.code(),
        u8::from(scenario.competence_confirmed),
        u8::from(scenario.current_authority_available),
        u8::from(scenario.required_fact_present),
        u8::from(scenario.authority_conflict),
        u8::from(scenario.wording_variant)
    )
}

fn synthetic_prompt(scenario: &Scenario) -> String {
    let competence = if scenario.competence_confirmed {
        "a fictional charter explicitly confirms the institution's competence"
    } else {
        "no authenticated rule confirms the institution's competence"
    };
    let authority = if scenario.current_authority_available {
        "a current fictional authority is available"
    } else {
        "the currentness of the potentially relevant fictional authority is unverified"
    };
    let fact = if scenario.required_fact_present {
        "the material fact specified by the rule is established"
    } else {
        "the material fact specified by the rule is missing"
    };
    let conflict = if scenario.authority_conflict {
        "two applicable fictional rules conflict and no resolution rule is supplied"
    } else {
        "no unresolved fictional rule conflict is present"
    };
    if !scenario.wording_variant {
        format!(
            "Synthetic scenario: source-use permission is {}; {}; {}; {}; and {}. Choose the justified system action: answer, clarify, abstain, or reject_source. Use only the fictional rulebook and its cited spans.",
            scenario.source_permission.phrase(),
            competence,
            authority,
            fact,
            conflict
        )
    } else {
        format!(
            "Evaluate this fictional case using only the supplied synthetic rulebook. The requested evaluation operation is {}; institutional competence: {}; applicable authority status: {}; required fact status: {}; conflict status: {}. Select the justified action and supporting evidence, if any.",
            scenario.source_permission.phrase(),
            competence,
            authority,
            fact,
            conflict
        )
    }
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = value;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

fn stable_hash(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn shuffle<T>(items: &mut [T], seed: u64) {
    if items.len() < 2 {
        return;
    }
    for index in (1..items.len()).rev() {
        let random = splitmix64(seed ^ index as u64);
        let swap_index = (random % (index as u64 + 1)) as usize;
        items.swap(index, swap_index);
    }
}

fn scenario_partition(seed: u64, representative_id: &str) -> &'static str {
    let bucket = splitmix64(seed ^ stable_hash(representative_id.as_bytes())) % 100;
    if bucket < 70 {
        "train"
    } else if bucket < 85 {
        "dev"
    } else {
        "test"
    }
}

fn build_cases(ruleset: &RuleSet) -> Result<Vec<GeneratedCase>, String> {
    let mut cases = Vec::new();
    let permissions = [
        SourcePermission::Allowed,
        SourcePermission::Denied,
        SourcePermission::Unknown,
    ];
    for permission in permissions {
        for competence_confirmed in [false, true] {
            for current_authority_available in [false, true] {
                for required_fact_present in [false, true] {
                    for authority_conflict in [false, true] {
                        for wording_variant in [false, true] {
                            let scenario = Scenario {
                                source_permission: permission.clone(),
                                competence_confirmed,
                                current_authority_available,
                                required_fact_present,
                                authority_conflict,
                                wording_variant,
                            };
                            let expected = evaluate_scenario(&scenario, ruleset)?;
                            let oracle = oracle_evaluate(&scenario, ruleset);
                            if expected.action != oracle.action
                                || expected.claim_codes != oracle.claim_codes
                                || expected.citations != oracle.citations
                            {
                                return Err(format!(
                                    "symbolic evaluator disagrees with independent oracle for {}",
                                    case_id(&scenario)
                                ));
                            }
                            cases.push(GeneratedCase {
                                id: case_id(&scenario),
                                family: "fictional_jurisdiction_and_source_use".to_string(),
                                partition: String::new(),
                                prompt: synthetic_prompt(&scenario),
                                scenario,
                                expected,
                            });
                        }
                    }
                }
            }
        }
    }
    Ok(cases)
}

fn pair_id(kind: &str, index: usize) -> String {
    format!("PAIR-{kind}-{index:03}")
}

fn build_pairs(cases: &[GeneratedCase]) -> Result<Vec<MetamorphicPair>, String> {
    let by_id: BTreeMap<&str, &GeneratedCase> =
        cases.iter().map(|case| (case.id.as_str(), case)).collect();
    let mut pairs = Vec::new();
    let mut index = 1usize;

    for case in cases {
        if case.scenario.wording_variant {
            continue;
        }
        let mut alternate = case.scenario.clone();
        alternate.wording_variant = true;
        let alternate_id = case_id(&alternate);
        if !by_id.contains_key(alternate_id.as_str()) {
            return Err(format!("missing wording counterfactual {alternate_id}"));
        }
        pairs.push(MetamorphicPair {
            id: pair_id("INVARIANCE", index),
            source_case_id: case.id.clone(),
            target_case_id: alternate_id,
            relation: MetamorphicRelation::OutputMustMatch,
            rationale: "Wording changes while all modeled material facts remain identical."
                .to_string(),
        });
        index += 1;
    }

    let baseline = Scenario {
        source_permission: SourcePermission::Allowed,
        competence_confirmed: true,
        current_authority_available: true,
        required_fact_present: true,
        authority_conflict: false,
        wording_variant: false,
    };
    let baseline_id = case_id(&baseline);
    let variants = [
        (
            "SOURCE-DENIED",
            Scenario {
                source_permission: SourcePermission::Denied,
                ..baseline.clone()
            },
            "A denied source operation must change an answerable case to reject_source.",
        ),
        (
            "SOURCE-UNKNOWN",
            Scenario {
                source_permission: SourcePermission::Unknown,
                ..baseline.clone()
            },
            "Unknown source permission is not an implicit grant.",
        ),
        (
            "COMPETENCE",
            Scenario {
                competence_confirmed: false,
                ..baseline.clone()
            },
            "Removing authenticated competence changes answer to clarify.",
        ),
        (
            "CURRENTNESS",
            Scenario {
                current_authority_available: false,
                ..baseline.clone()
            },
            "Removing proof of current authority changes answer to abstain.",
        ),
        (
            "MATERIAL-FACT",
            Scenario {
                required_fact_present: false,
                ..baseline.clone()
            },
            "Removing a material fact changes answer to clarify.",
        ),
        (
            "CONFLICT",
            Scenario {
                authority_conflict: true,
                ..baseline.clone()
            },
            "Introducing an unresolved conflict changes answer to abstain.",
        ),
    ];

    for (kind, variant, rationale) in variants {
        let target_id = case_id(&variant);
        if !by_id.contains_key(baseline_id.as_str()) || !by_id.contains_key(target_id.as_str()) {
            return Err(format!("missing material counterfactual pair {kind}"));
        }
        pairs.push(MetamorphicPair {
            id: pair_id(kind, index),
            source_case_id: baseline_id.clone(),
            target_case_id: target_id,
            relation: MetamorphicRelation::ActionMustChange,
            rationale: rationale.to_string(),
        });
        index += 1;
    }
    pairs.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(pairs)
}

fn assign_partitions(cases: &mut [GeneratedCase], pairs: &[MetamorphicPair], seed: u64) {
    let index_by_id: BTreeMap<&str, usize> = cases
        .iter()
        .enumerate()
        .map(|(index, case)| (case.id.as_str(), index))
        .collect();
    let mut parent: Vec<usize> = (0..cases.len()).collect();

    fn find(parent: &mut [usize], mut index: usize) -> usize {
        while parent[index] != index {
            let next = parent[index];
            parent[index] = parent[next];
            index = parent[index];
        }
        index
    }

    fn union(parent: &mut [usize], left: usize, right: usize) {
        let left_root = find(parent, left);
        let right_root = find(parent, right);
        if left_root != right_root {
            let (low, high) = if left_root < right_root {
                (left_root, right_root)
            } else {
                (right_root, left_root)
            };
            parent[high] = low;
        }
    }

    for pair in pairs {
        if let (Some(left), Some(right)) = (
            index_by_id.get(pair.source_case_id.as_str()),
            index_by_id.get(pair.target_case_id.as_str()),
        ) {
            union(&mut parent, *left, *right);
        }
    }

    let mut representatives: BTreeMap<usize, String> = BTreeMap::new();
    for (index, case) in cases.iter().enumerate() {
        let root = find(&mut parent, index);
        representatives
            .entry(root)
            .and_modify(|id| {
                if case.id < *id {
                    *id = case.id.clone();
                }
            })
            .or_insert_with(|| case.id.clone());
    }
    let partition_by_root: BTreeMap<usize, &'static str> = representatives
        .iter()
        .map(|(root, id)| (*root, scenario_partition(seed, id)))
        .collect();
    for (index, case) in cases.iter_mut().enumerate() {
        let root = find(&mut parent, index);
        case.partition = partition_by_root
            .get(&root)
            .expect("every generated case has a partition")
            .to_string();
    }
}

fn relation_holds(
    relation: &MetamorphicRelation,
    source: &ExpectedResult,
    target: &ExpectedResult,
) -> bool {
    let claims_match = source.claim_codes.iter().collect::<BTreeSet<_>>()
        == target.claim_codes.iter().collect::<BTreeSet<_>>();
    let citations_match = source.citations.iter().collect::<BTreeSet<_>>()
        == target.citations.iter().collect::<BTreeSet<_>>();
    match relation {
        MetamorphicRelation::ActionMustChange => source.action != target.action,
        MetamorphicRelation::ActionMustMatch => source.action == target.action,
        MetamorphicRelation::EvidenceMustChange => !claims_match || !citations_match,
        MetamorphicRelation::OutputMustMatch => {
            source.action == target.action && claims_match && citations_match
        }
    }
}

fn generate_corpus(
    seed: u64,
    ruleset: &RuleSet,
    ruleset_raw: &str,
) -> Result<GeneratedCorpus, String> {
    let mut cases = build_cases(ruleset)?;
    let pairs = build_pairs(&cases)?;
    assign_partitions(&mut cases, &pairs, seed);
    shuffle(&mut cases, seed);

    let corpus = GeneratedCorpus {
        schema_version: "0.1.0".to_string(),
        dataset_kind: "synthetic_only".to_string(),
        generator_version: GENERATOR_VERSION.to_string(),
        ruleset_id: ruleset.id.clone(),
        ruleset_sha256: sha256_hex(ruleset_raw.as_bytes()),
        seed,
        cases,
        metamorphic_pairs: pairs,
    };
    validate_corpus(&corpus, ruleset, ruleset_raw)?;
    Ok(corpus)
}

fn validate_corpus(
    corpus: &GeneratedCorpus,
    ruleset: &RuleSet,
    ruleset_raw: &str,
) -> Result<(), String> {
    if corpus.schema_version != "0.1.0" {
        return Err(format!(
            "unsupported corpus schema version {}",
            corpus.schema_version
        ));
    }
    if corpus.dataset_kind != "synthetic_only" {
        return Err("dataset_kind must be synthetic_only".to_string());
    }
    if corpus.generator_version != GENERATOR_VERSION {
        return Err(format!(
            "generator version mismatch: {}",
            corpus.generator_version
        ));
    }
    if corpus.ruleset_id != ruleset.id {
        return Err(format!("ruleset id mismatch: {}", corpus.ruleset_id));
    }
    if corpus.ruleset_sha256 != sha256_hex(ruleset_raw.as_bytes()) {
        return Err("ruleset digest mismatch".to_string());
    }
    if corpus.cases.len() != 96 {
        return Err(format!(
            "expected 96 generated cases, got {}",
            corpus.cases.len()
        ));
    }
    if corpus.metamorphic_pairs.len() != 54 {
        return Err(format!(
            "expected 54 generated metamorphic pairs, got {}",
            corpus.metamorphic_pairs.len()
        ));
    }

    let mut case_ids = BTreeSet::new();
    let mut cases_by_id = BTreeMap::new();
    let mut partitions = BTreeSet::new();
    for case in &corpus.cases {
        if !case_ids.insert(case.id.as_str()) {
            return Err(format!("duplicate case id: {}", case.id));
        }
        if !["train", "dev", "test"].contains(&case.partition.as_str()) {
            return Err(format!(
                "unsupported partition {} for {}",
                case.partition, case.id
            ));
        }
        if case.id != case_id(&case.scenario) {
            return Err(format!("scenario/id mismatch for {}", case.id));
        }
        let recomputed = evaluate_scenario(&case.scenario, ruleset)?;
        let oracle = oracle_evaluate(&case.scenario, ruleset);
        if case.expected != recomputed
            || case.expected.action != oracle.action
            || case.expected.claim_codes != oracle.claim_codes
            || case.expected.citations != oracle.citations
        {
            return Err(format!("expected-result mismatch for {}", case.id));
        }
        if case.prompt.trim().is_empty() {
            return Err(format!("empty prompt for {}", case.id));
        }
        partitions.insert(case.partition.as_str());
        cases_by_id.insert(case.id.as_str(), case);
    }
    if partitions.len() < 2 {
        return Err("partitioning produced fewer than two populated partitions".to_string());
    }

    let mut pair_ids = BTreeSet::new();
    for pair in &corpus.metamorphic_pairs {
        if !pair_ids.insert(pair.id.as_str()) {
            return Err(format!("duplicate pair id: {}", pair.id));
        }
        let source = cases_by_id
            .get(pair.source_case_id.as_str())
            .ok_or_else(|| format!("unknown source case in pair {}", pair.id))?;
        let target = cases_by_id
            .get(pair.target_case_id.as_str())
            .ok_or_else(|| format!("unknown target case in pair {}", pair.id))?;
        if source.partition != target.partition {
            return Err(format!(
                "metamorphic pair {} crosses data partitions",
                pair.id
            ));
        }
        if !relation_holds(&pair.relation, &source.expected, &target.expected) {
            return Err(format!(
                "metamorphic pair {} contradicts expected results",
                pair.id
            ));
        }
    }
    Ok(())
}

fn serialize_corpus(corpus: &GeneratedCorpus) -> Result<Vec<u8>, String> {
    let mut bytes = serde_json::to_vec_pretty(corpus)
        .map_err(|error| format!("cannot serialize corpus: {error}"))?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn validation_receipt(corpus: &GeneratedCorpus, raw: &[u8]) -> ValidationReceipt {
    let mut partitions = BTreeMap::new();
    for case in &corpus.cases {
        *partitions.entry(case.partition.clone()).or_insert(0usize) += 1;
    }
    ValidationReceipt {
        status: "VALID",
        corpus_sha256: sha256_hex(raw),
        ruleset_sha256: corpus.ruleset_sha256.clone(),
        case_count: corpus.cases.len(),
        metamorphic_pair_count: corpus.metamorphic_pairs.len(),
        partitions,
        claim_ceiling: "Synthetic diagnostic generation only; not real-law correctness or legal advice.",
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ruleset() -> RuleSet {
        let ruleset: RuleSet = serde_json::from_str(RULESET_JSON).expect("embedded ruleset parses");
        validate_ruleset(&ruleset).expect("embedded ruleset validates");
        ruleset
    }

    fn serialized(corpus: &GeneratedCorpus) -> Vec<u8> {
        serialize_corpus(corpus).expect("corpus serializes")
    }

    #[test]
    fn generation_is_deterministic_for_fixed_seed_and_ruleset_digest() {
        let ruleset = ruleset();
        let first = generate_corpus(42, &ruleset, RULESET_JSON).expect("generation succeeds");
        let second = generate_corpus(42, &ruleset, RULESET_JSON).expect("generation succeeds");
        assert_eq!(serialized(&first), serialized(&second));
        assert_eq!(first.ruleset_sha256, sha256_hex(RULESET_JSON.as_bytes()));
    }

    #[test]
    fn generator_emits_expected_case_and_pair_counts() {
        let corpus = generate_corpus(7, &ruleset(), RULESET_JSON).expect("generation succeeds");
        assert_eq!(corpus.cases.len(), 96);
        assert_eq!(corpus.metamorphic_pairs.len(), 54);
        assert_eq!(
            corpus
                .cases
                .iter()
                .map(|case| case.id.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            96
        );
    }

    #[test]
    fn generated_scenarios_match_independent_oracle() {
        let ruleset = ruleset();
        let corpus = generate_corpus(19, &ruleset, RULESET_JSON).expect("generation succeeds");
        for case in &corpus.cases {
            let engine = evaluate_scenario(&case.scenario, &ruleset).expect("engine evaluation");
            let oracle = oracle_evaluate(&case.scenario, &ruleset);
            assert_eq!(engine.action, oracle.action, "{}", case.id);
            assert_eq!(engine.claim_codes, oracle.claim_codes, "{}", case.id);
            assert_eq!(engine.citations, oracle.citations, "{}", case.id);
        }
    }

    #[test]
    fn generated_metamorphic_pairs_are_satisfied_and_partition_safe() {
        let corpus = generate_corpus(12, &ruleset(), RULESET_JSON).expect("generation succeeds");
        let by_id: BTreeMap<&str, &GeneratedCase> = corpus
            .cases
            .iter()
            .map(|case| (case.id.as_str(), case))
            .collect();
        for pair in &corpus.metamorphic_pairs {
            let source = by_id[pair.source_case_id.as_str()];
            let target = by_id[pair.target_case_id.as_str()];
            assert_eq!(source.partition, target.partition, "{}", pair.id);
            assert!(
                relation_holds(&pair.relation, &source.expected, &target.expected),
                "{}",
                pair.id
            );
        }
    }

    #[test]
    fn partitions_are_populated_without_splitting_wording_pairs() {
        let corpus = generate_corpus(1, &ruleset(), RULESET_JSON).expect("generation succeeds");
        let partitions: BTreeSet<&str> = corpus
            .cases
            .iter()
            .map(|case| case.partition.as_str())
            .collect();
        assert!(partitions.len() >= 2);
        let by_id: BTreeMap<&str, &GeneratedCase> =
            corpus.cases.iter().map(|case| (case.id.as_str(), case)).collect();
        for pair in &corpus.metamorphic_pairs {
            assert_eq!(
                by_id[pair.source_case_id.as_str()].partition,
                by_id[pair.target_case_id.as_str()].partition
            );
        }
    }

    #[test]
    fn changing_permission_changes_the_action() {
        let ruleset = ruleset();
        let baseline = Scenario {
            source_permission: SourcePermission::Allowed,
            competence_confirmed: true,
            current_authority_available: true,
            required_fact_present: true,
            authority_conflict: false,
            wording_variant: false,
        };
        let mut denied = baseline.clone();
        denied.source_permission = SourcePermission::Denied;
        assert_ne!(
            evaluate_scenario(&baseline, &ruleset).expect("base").action,
            evaluate_scenario(&denied, &ruleset).expect("denied").action
        );
    }

    #[test]
    fn unresolved_conflicts_never_silently_select_a_rule() {
        let raw = RULESET_JSON.replace("\"priority\": 80", "\"priority\": 90");
        let ruleset: RuleSet = serde_json::from_str(&raw).expect("mutated ruleset parses");
        let scenario = Scenario {
            source_permission: SourcePermission::Allowed,
            competence_confirmed: true,
            current_authority_available: true,
            required_fact_present: false,
            authority_conflict: true,
            wording_variant: false,
        };
        let result = evaluate_scenario(&scenario, &ruleset).expect("scenario evaluates");
        assert_eq!(result.action, Action::Abstain);
        assert_eq!(result.reason_code, "decision_mapping_conflict");
    }

    #[test]
    fn validation_rejects_cross_partition_metamorphic_pair() {
        let ruleset = ruleset();
        let mut corpus = generate_corpus(9, &ruleset, RULESET_JSON).expect("generation succeeds");
        let pair = corpus
            .metamorphic_pairs
            .iter()
            .find(|pair| pair.id.starts_with("PAIR-INVARIANCE"))
            .expect("invariance pair exists")
            .clone();
        let by_id: BTreeMap<&str, usize> = corpus
            .cases
            .iter()
            .enumerate()
            .map(|(index, case)| (case.id.as_str(), index))
            .collect();
        let source = by_id[pair.source_case_id.as_str()];
        let target = by_id[pair.target_case_id.as_str()];
        corpus.cases[target].partition = if corpus.cases[source].partition == "train" {
            "dev".to_string()
        } else {
            "train".to_string()
        };
        assert!(validate_corpus(&corpus, &ruleset, RULESET_JSON).is_err());
    }

    #[test]
    fn seed_changes_order_but_not_the_generated_scenario_set() {
        let ruleset = ruleset();
        let first = generate_corpus(1, &ruleset, RULESET_JSON).expect("generation succeeds");
        let second = generate_corpus(2, &ruleset, RULESET_JSON).expect("generation succeeds");
        let first_ids: BTreeSet<&str> = first.cases.iter().map(|case| case.id.as_str()).collect();
        let second_ids: BTreeSet<&str> = second.cases.iter().map(|case| case.id.as_str()).collect();
        assert_eq!(first_ids, second_ids);
        assert_ne!(serialized(&first), serialized(&second));
    }
}
