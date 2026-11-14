// SPDX-License-Identifier: AGPL-3.0-or-later
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::{env, fs, process};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Action {
    Answer,
    Clarify,
    Abstain,
    RejectSource,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Benchmark {
    schema_version: String,
    dataset_kind: String,
    description: String,
    authorities: Vec<Authority>,
    cases: Vec<BenchmarkCase>,
    metamorphic_pairs: Vec<MetamorphicPair>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Authority {
    id: String,
    jurisdiction: String,
    kind: String,
    force: String,
    valid_from: Option<String>,
    valid_to: Option<String>,
    permitted_ai_use: String,
    span_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct BenchmarkCase {
    id: String,
    family: String,
    prompt: String,
    jurisdiction: Option<String>,
    as_of: Option<String>,
    available_authority_ids: Vec<String>,
    expected_action: Action,
    expected_claim_codes: Vec<String>,
    expected_citations: Vec<Citation>,
    critical: bool,
    tags: Vec<String>,
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

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
struct Citation {
    authority_id: String,
    role: String,
    span_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Submission {
    schema_version: String,
    system_id: String,
    results: Vec<CandidateResult>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CandidateResult {
    case_id: String,
    action: Action,
    claim_codes: Vec<String>,
    citations: Vec<Citation>,
}

#[derive(Debug, Serialize)]
struct ValidationReport {
    status: &'static str,
    dataset_kind: String,
    corpus_sha256: String,
    authority_count: usize,
    case_count: usize,
    metamorphic_pair_count: usize,
    claim_ceiling: &'static str,
}

#[derive(Debug, Serialize)]
struct CaseEvaluation {
    case_id: String,
    critical: bool,
    expected_action: Action,
    observed_action: Option<Action>,
    expected_claim_codes: Vec<String>,
    observed_claim_codes: Vec<String>,
    expected_citations: Vec<Citation>,
    observed_citations: Vec<Citation>,
    action_match: bool,
    claim_code_exact_match: bool,
    citation_exact_match: bool,
    failure_codes: Vec<String>,
}

#[derive(Debug, Serialize)]
struct EvaluationReport {
    report_version: &'static str,
    evaluator_version: &'static str,
    dataset_kind: String,
    corpus_sha256: String,
    submission_sha256: String,
    system_id: String,
    total_cases: usize,
    provided_results: usize,
    action_accuracy_pct: f64,
    claim_code_exact_match_pct: f64,
    citation_exact_match_pct: f64,
    missing_result_case_ids: Vec<String>,
    duplicate_result_case_ids: Vec<String>,
    extra_result_case_ids: Vec<String>,
    unknown_authority_citations: usize,
    unavailable_source_citations: usize,
    authority_role_mismatches: usize,
    unknown_span_citations: usize,
    temporally_inapplicable_citations: usize,
    temporally_unverified_citations: usize,
    prohibited_source_citations: usize,
    unverified_authority_citations: usize,
    duplicate_citations: usize,
    duplicate_claim_codes: usize,
    uncited_answers: usize,
    critical_failure_case_ids: Vec<String>,
    case_evaluations: Vec<CaseEvaluation>,
    metamorphic_pairs_total: usize,
    metamorphic_pairs_passed: usize,
    metamorphic_pair_failure_ids: Vec<String>,
    metamorphic_gate_passed: bool,
    fixture_gate_passed: bool,
    claim_ceiling: &'static str,
}

fn main() {
    match run() {
        Ok(true) => {}
        Ok(false) => process::exit(1),
        Err(error) => {
            eprintln!("legal-evidence-benchmark: {error}");
            process::exit(2);
        }
    }
}

fn run() -> Result<bool, String> {
    let mut args = env::args().skip(1);
    let command = args.next().ok_or_else(usage)?;
    match command.as_str() {
        "validate" => {
            let corpus_path = args.next().ok_or_else(usage)?;
            if args.next().is_some() {
                return Err(usage());
            }
            let (benchmark, corpus_sha256): (Benchmark, String) = read_json(&corpus_path)?;
            validate_benchmark(&benchmark)?;
            let report = ValidationReport {
                status: "VALID",
                dataset_kind: benchmark.dataset_kind.clone(),
                corpus_sha256,
                authority_count: benchmark.authorities.len(),
                case_count: benchmark.cases.len(),
                metamorphic_pair_count: benchmark.metamorphic_pairs.len(),
                claim_ceiling: "Dataset structure only; not legal correctness or model performance.",
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&report)
                    .map_err(|e| format!("cannot serialize validation report: {e}"))?
            );
            Ok(true)
        }
        "evaluate" => {
            let corpus_path = args.next().ok_or_else(usage)?;
            let submission_path = args.next().ok_or_else(usage)?;
            if args.next().is_some() {
                return Err(usage());
            }
            let (benchmark, corpus_sha256): (Benchmark, String) = read_json(&corpus_path)?;
            validate_benchmark(&benchmark)?;
            let (submission, submission_sha256): (Submission, String) =
                read_json(&submission_path)?;
            if submission.schema_version != "0.1.0" {
                return Err(format!(
                    "unsupported submission schema version: {}",
                    submission.schema_version
                ));
            }
            if submission.system_id.trim().is_empty() {
                return Err("submission system_id must not be empty".to_string());
            }
            let report =
                evaluate_with_digests(&benchmark, &submission, corpus_sha256, submission_sha256)?;
            let passed = report.fixture_gate_passed;
            println!(
                "{}",
                serde_json::to_string_pretty(&report)
                    .map_err(|e| format!("cannot serialize evaluation report: {e}"))?
            );
            Ok(passed)
        }
        _ => Err(usage()),
    }
}

fn usage() -> String {
    [
        "usage: legal-evidence-benchmark validate <corpus.json>",
        "       legal-evidence-benchmark evaluate <corpus.json> <submission.json>",
    ]
    .join("\n")
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &str) -> Result<(T, String), String> {
    let raw = fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let digest = sha256_hex(raw.as_bytes());
    let value = serde_json::from_str(&raw).map_err(|e| format!("{path} is invalid JSON: {e}"))?;
    Ok((value, digest))
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn validate_benchmark(benchmark: &Benchmark) -> Result<(), String> {
    let mut errors = Vec::new();

    if benchmark.schema_version != "0.1.0" {
        errors.push(format!(
            "unsupported benchmark schema version {}",
            benchmark.schema_version
        ));
    }
    if benchmark.dataset_kind != "synthetic_only" {
        errors.push("dataset_kind must be synthetic_only in v0.1".to_string());
    }
    if benchmark.description.trim().is_empty() {
        errors.push("description must not be empty".to_string());
    }
    if benchmark.authorities.is_empty() {
        errors.push("at least one authority record is required".to_string());
    }
    if benchmark.cases.is_empty() {
        errors.push("at least one benchmark case is required".to_string());
    }
    if benchmark.metamorphic_pairs.is_empty() {
        errors.push("at least one metamorphic pair is required".to_string());
    }

    let mut authority_map = BTreeMap::new();
    let mut authority_ids = BTreeSet::new();
    for authority in &benchmark.authorities {
        if authority.id.trim().is_empty() {
            errors.push("authority id must not be empty".to_string());
        }
        if !authority_ids.insert(authority.id.clone()) {
            errors.push(format!("duplicate authority id: {}", authority.id));
        }
        if !is_synthetic_scope(&authority.jurisdiction) {
            errors.push(format!(
                "authority {} uses a non-synthetic scope {}",
                authority.id, authority.jurisdiction
            ));
        }
        if ![
            "binding_in_scope",
            "persuasive_only",
            "policy_only",
            "metadata_only",
            "unverified",
        ]
        .contains(&authority.force.as_str())
        {
            errors.push(format!(
                "authority {} has unsupported force {}",
                authority.id, authority.force
            ));
        }
        if !["synthetic_test_only", "citation_only", "prohibited"]
            .contains(&authority.permitted_ai_use.as_str())
        {
            errors.push(format!(
                "authority {} has unsupported permitted_ai_use {}",
                authority.id, authority.permitted_ai_use
            ));
        }
        if authority.kind.trim().is_empty() {
            errors.push(format!("authority {} has an empty kind", authority.id));
        }
        let mut spans = BTreeSet::new();
        for span in &authority.span_ids {
            if span.trim().is_empty() || !spans.insert(span.clone()) {
                errors.push(format!(
                    "authority {} has an empty or duplicate span id",
                    authority.id
                ));
            }
        }
        if let Some(from) = authority.valid_from.as_deref() {
            if parse_date(from).is_none() {
                errors.push(format!(
                    "authority {} has invalid valid_from date {from}",
                    authority.id
                ));
            }
        }
        if let Some(to) = authority.valid_to.as_deref() {
            if parse_date(to).is_none() {
                errors.push(format!(
                    "authority {} has invalid valid_to date {to}",
                    authority.id
                ));
            }
        }
        if let (Some(from), Some(to)) = (
            authority.valid_from.as_deref(),
            authority.valid_to.as_deref(),
        ) {
            match (parse_date(from), parse_date(to)) {
                (Some(from), Some(to)) if from < to => {}
                _ => errors.push(format!(
                    "authority {} must use a non-empty half-open validity interval",
                    authority.id
                )),
            }
        }
        authority_map.insert(authority.id.as_str(), authority);
    }

    let mut case_ids = BTreeSet::new();
    for case in &benchmark.cases {
        if case.id.trim().is_empty() || !case_ids.insert(case.id.clone()) {
            errors.push(format!("empty or duplicate case id: {}", case.id));
        }
        if case.family.trim().is_empty() || case.prompt.trim().is_empty() {
            errors.push(format!("case {} has an empty family or prompt", case.id));
        }
        if let Some(jurisdiction) = case.jurisdiction.as_deref() {
            if !is_synthetic_scope(jurisdiction) {
                errors.push(format!(
                    "case {} uses a non-synthetic jurisdiction {}",
                    case.id, jurisdiction
                ));
            }
        }
        if let Some(as_of) = case.as_of.as_deref() {
            if parse_date(as_of).is_none() {
                errors.push(format!("case {} has invalid as_of date {as_of}", case.id));
            }
        }

        let available: BTreeSet<&str> = case
            .available_authority_ids
            .iter()
            .map(String::as_str)
            .collect();
        if available.len() != case.available_authority_ids.len() {
            errors.push(format!(
                "case {} repeats an available authority id",
                case.id
            ));
        }
        for id in &case.available_authority_ids {
            if !authority_map.contains_key(id.as_str()) {
                errors.push(format!(
                    "case {} references unknown available authority {}",
                    case.id, id
                ));
            }
        }

        if case.expected_action == Action::Answer
            && (case.expected_claim_codes.is_empty() || case.expected_citations.is_empty())
        {
            errors.push(format!(
                "answer case {} must have expected claim codes and citations",
                case.id
            ));
        }
        if case.expected_action != Action::Answer
            && (!case.expected_claim_codes.is_empty() || !case.expected_citations.is_empty())
        {
            errors.push(format!(
                "non-answer case {} must not assert expected claims or citations",
                case.id
            ));
        }
        if has_duplicates(&case.expected_claim_codes) {
            errors.push(format!("case {} repeats an expected claim code", case.id));
        }
        if has_duplicates(&case.expected_citations) {
            errors.push(format!("case {} repeats an expected citation", case.id));
        }

        for citation in &case.expected_citations {
            match authority_map.get(citation.authority_id.as_str()) {
                None => errors.push(format!(
                    "case {} expects unknown authority {}",
                    case.id, citation.authority_id
                )),
                Some(authority) => {
                    if !available.contains(citation.authority_id.as_str()) {
                        errors.push(format!(
                            "case {} expects authority {} not available to the candidate",
                            case.id, citation.authority_id
                        ));
                    }
                    if authority.force != citation.role {
                        errors.push(format!(
                            "case {} expected citation role does not match authority {}",
                            case.id, citation.authority_id
                        ));
                    }
                    if !authority.span_ids.iter().any(|s| s == &citation.span_id) {
                        errors.push(format!(
                            "case {} expects unknown span {} for authority {}",
                            case.id, citation.span_id, citation.authority_id
                        ));
                    }
                    if authority.permitted_ai_use == "prohibited" {
                        errors.push(format!(
                            "case {} expects citation to a prohibited-use authority {}",
                            case.id, citation.authority_id
                        ));
                    }
                    match active_on(authority, case.as_of.as_deref()) {
                        Some(true) => {}
                        Some(false) => errors.push(format!(
                            "case {} expects a temporally inapplicable authority {}",
                            case.id, citation.authority_id
                        )),
                        None => errors.push(format!(
                            "case {} expects a temporally unverified authority {}",
                            case.id, citation.authority_id
                        )),
                    }
                }
            }
        }
    }

    let case_map: BTreeMap<&str, &BenchmarkCase> = benchmark
        .cases
        .iter()
        .map(|case| (case.id.as_str(), case))
        .collect();
    let mut pair_ids = BTreeSet::new();
    for pair in &benchmark.metamorphic_pairs {
        if pair.id.trim().is_empty() || !pair_ids.insert(pair.id.as_str()) {
            errors.push(format!(
                "empty or duplicate metamorphic pair id: {}",
                pair.id
            ));
        }
        if pair.rationale.trim().is_empty() {
            errors.push(format!("metamorphic pair {} has empty rationale", pair.id));
        }
        if pair.source_case_id == pair.target_case_id {
            errors.push(format!(
                "metamorphic pair {} must reference two distinct cases",
                pair.id
            ));
            continue;
        }
        match (
            case_map.get(pair.source_case_id.as_str()),
            case_map.get(pair.target_case_id.as_str()),
        ) {
            (Some(source), Some(target)) => {
                if !case_relation_holds(
                    &pair.relation,
                    &source.expected_action,
                    &source.expected_claim_codes,
                    &source.expected_citations,
                    &target.expected_action,
                    &target.expected_claim_codes,
                    &target.expected_citations,
                ) {
                    errors.push(format!(
                        "metamorphic pair {} contradicts the expected case outputs",
                        pair.id
                    ));
                }
            }
            _ => errors.push(format!(
                "metamorphic pair {} references an unknown source or target case",
                pair.id
            )),
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "benchmark validation failed:\n - {}",
            errors.join("\n - ")
        ))
    }
}

fn is_synthetic_scope(scope: &str) -> bool {
    scope.starts_with("fictional:")
        || scope.starts_with("organization:")
        || scope == "source-registry"
}

fn has_duplicates<T: Ord + Clone>(values: &[T]) -> bool {
    let set: BTreeSet<T> = values.iter().cloned().collect();
    set.len() != values.len()
}

fn parse_date(value: &str) -> Option<(i32, u32, u32)> {
    if value.len() != 10
        || value.as_bytes().get(4) != Some(&b'-')
        || value.as_bytes().get(7) != Some(&b'-')
    {
        return None;
    }
    let year: i32 = value.get(0..4)?.parse().ok()?;
    let month: u32 = value.get(5..7)?.parse().ok()?;
    let day: u32 = value.get(8..10)?.parse().ok()?;
    if !(1..=12).contains(&month) || year < 1 {
        return None;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let month_days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return None,
    };
    if day == 0 || day > month_days {
        return None;
    }
    Some((year, month, day))
}

fn active_on(authority: &Authority, as_of: Option<&str>) -> Option<bool> {
    if authority.valid_from.is_none() && authority.valid_to.is_none() {
        return Some(true);
    }
    let as_of = parse_date(as_of?)?;
    if let Some(from) = authority.valid_from.as_deref().and_then(parse_date) {
        if as_of < from {
            return Some(false);
        }
    }
    if let Some(to) = authority.valid_to.as_deref().and_then(parse_date) {
        if as_of >= to {
            return Some(false);
        }
    }
    Some(true)
}

fn citation_key(citation: &Citation) -> String {
    format!(
        "{}|{}|{}",
        citation.authority_id, citation.role, citation.span_id
    )
}

fn unique_citations(citations: &[Citation]) -> (BTreeSet<String>, bool) {
    let mut keys = BTreeSet::new();
    let mut duplicate = false;
    for citation in citations {
        if !keys.insert(citation_key(citation)) {
            duplicate = true;
        }
    }
    (keys, duplicate)
}

fn unique_strings(values: &[String]) -> (BTreeSet<String>, bool) {
    let mut set = BTreeSet::new();
    let mut duplicate = false;
    for value in values {
        if !set.insert(value.clone()) {
            duplicate = true;
        }
    }
    (set, duplicate)
}

fn percent(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        ((numerator as f64 / denominator as f64) * 10_000.0).round() / 100.0
    }
}

fn case_relation_holds(
    relation: &MetamorphicRelation,
    source_action: &Action,
    source_claim_codes: &[String],
    source_citations: &[Citation],
    target_action: &Action,
    target_claim_codes: &[String],
    target_citations: &[Citation],
) -> bool {
    let claims_match = unique_strings(source_claim_codes).0 == unique_strings(target_claim_codes).0;
    let citations_match =
        unique_citations(source_citations).0 == unique_citations(target_citations).0;
    match relation {
        MetamorphicRelation::ActionMustChange => source_action != target_action,
        MetamorphicRelation::ActionMustMatch => source_action == target_action,
        MetamorphicRelation::EvidenceMustChange => !claims_match || !citations_match,
        MetamorphicRelation::OutputMustMatch => {
            source_action == target_action && claims_match && citations_match
        }
    }
}

fn make_case_evaluation(
    case: &BenchmarkCase,
    result: Option<&CandidateResult>,
    action_match: bool,
    claim_code_exact_match: bool,
    citation_exact_match: bool,
    failure_codes: Vec<String>,
) -> CaseEvaluation {
    CaseEvaluation {
        case_id: case.id.clone(),
        critical: case.critical,
        expected_action: case.expected_action.clone(),
        observed_action: result.map(|item| item.action.clone()),
        expected_claim_codes: case.expected_claim_codes.clone(),
        observed_claim_codes: result
            .map(|item| item.claim_codes.clone())
            .unwrap_or_default(),
        expected_citations: case.expected_citations.clone(),
        observed_citations: result
            .map(|item| item.citations.clone())
            .unwrap_or_default(),
        action_match,
        claim_code_exact_match,
        citation_exact_match,
        failure_codes,
    }
}

fn evaluate_metamorphic_pairs(
    benchmark: &Benchmark,
    submission: &Submission,
) -> (usize, Vec<String>) {
    let mut passed = 0usize;
    let mut failures = Vec::new();
    for pair in &benchmark.metamorphic_pairs {
        let source_results: Vec<&CandidateResult> = submission
            .results
            .iter()
            .filter(|result| result.case_id == pair.source_case_id)
            .collect();
        let target_results: Vec<&CandidateResult> = submission
            .results
            .iter()
            .filter(|result| result.case_id == pair.target_case_id)
            .collect();
        if source_results.len() != 1 || target_results.len() != 1 {
            failures.push(pair.id.clone());
            continue;
        }
        let source = source_results[0];
        let target = target_results[0];
        if case_relation_holds(
            &pair.relation,
            &source.action,
            &source.claim_codes,
            &source.citations,
            &target.action,
            &target.claim_codes,
            &target.citations,
        ) {
            passed += 1;
        } else {
            failures.push(pair.id.clone());
        }
    }
    (passed, failures)
}

fn evaluate(benchmark: &Benchmark, submission: &Submission) -> Result<EvaluationReport, String> {
    let corpus_bytes = serde_json::to_vec(benchmark)
        .map_err(|e| format!("cannot serialize benchmark for digest: {e}"))?;
    let submission_bytes = serde_json::to_vec(submission)
        .map_err(|e| format!("cannot serialize submission for digest: {e}"))?;
    evaluate_with_digests(
        benchmark,
        submission,
        sha256_hex(&corpus_bytes),
        sha256_hex(&submission_bytes),
    )
}

fn evaluate_with_digests(
    benchmark: &Benchmark,
    submission: &Submission,
    corpus_sha256: String,
    submission_sha256: String,
) -> Result<EvaluationReport, String> {
    if submission.schema_version != "0.1.0" {
        return Err(format!(
            "unsupported submission schema version {}",
            submission.schema_version
        ));
    }
    if submission.system_id.trim().is_empty() {
        return Err("submission system_id must not be empty".to_string());
    }

    let authority_map: BTreeMap<&str, &Authority> = benchmark
        .authorities
        .iter()
        .map(|authority| (authority.id.as_str(), authority))
        .collect();
    let case_ids: BTreeSet<&str> = benchmark
        .cases
        .iter()
        .map(|case| case.id.as_str())
        .collect();
    let mut by_case: BTreeMap<String, Vec<&CandidateResult>> = BTreeMap::new();
    let mut extra_ids = BTreeSet::new();
    for result in &submission.results {
        if !case_ids.contains(result.case_id.as_str()) {
            extra_ids.insert(result.case_id.clone());
        }
        by_case
            .entry(result.case_id.clone())
            .or_default()
            .push(result);
    }

    let mut missing_ids = Vec::new();
    let mut duplicate_ids = Vec::new();
    let extra_result_case_ids: Vec<String> = extra_ids.into_iter().collect();
    let mut action_correct = 0usize;
    let mut claim_exact = 0usize;
    let mut citation_exact = 0usize;
    let mut unknown_authority_citations = 0usize;
    let mut unavailable_source_citations = 0usize;
    let mut authority_role_mismatches = 0usize;
    let mut unknown_span_citations = 0usize;
    let mut temporally_inapplicable_citations = 0usize;
    let mut temporally_unverified_citations = 0usize;
    let mut prohibited_source_citations = 0usize;
    let mut unverified_authority_citations = 0usize;
    let mut duplicate_citations = 0usize;
    let mut duplicate_claim_codes = 0usize;
    let mut uncited_answers = 0usize;
    let mut critical_failures = BTreeSet::new();
    let mut case_evaluations = Vec::with_capacity(benchmark.cases.len());

    for case in &benchmark.cases {
        let matching = by_case.get(&case.id).map(Vec::as_slice).unwrap_or(&[]);
        if matching.is_empty() {
            missing_ids.push(case.id.clone());
            if case.critical {
                critical_failures.insert(case.id.clone());
            }
            case_evaluations.push(make_case_evaluation(
                case,
                None,
                false,
                false,
                false,
                vec!["missing_result".to_string()],
            ));
            continue;
        }
        if matching.len() != 1 {
            duplicate_ids.push(case.id.clone());
            if case.critical {
                critical_failures.insert(case.id.clone());
            }
            case_evaluations.push(make_case_evaluation(
                case,
                None,
                false,
                false,
                false,
                vec!["duplicate_result".to_string()],
            ));
            continue;
        }

        let result = matching[0];
        let mut case_bad = false;
        let mut failure_codes = Vec::new();

        let action_match = result.action == case.expected_action;
        if action_match {
            action_correct += 1;
        } else {
            case_bad = true;
            failure_codes.push("action_mismatch".to_string());
        }

        let (actual_claims, repeated_claims) = unique_strings(&result.claim_codes);
        let (expected_claims, _) = unique_strings(&case.expected_claim_codes);
        if repeated_claims {
            duplicate_claim_codes += 1;
            case_bad = true;
            failure_codes.push("duplicate_claim_codes".to_string());
        }
        let claim_code_match = actual_claims == expected_claims && !repeated_claims;
        if claim_code_match {
            claim_exact += 1;
        } else {
            case_bad = true;
            failure_codes.push("claim_code_set_mismatch".to_string());
        }

        let (actual_citations, repeated_citations) = unique_citations(&result.citations);
        let (expected_citations, _) = unique_citations(&case.expected_citations);
        if repeated_citations {
            duplicate_citations += 1;
            case_bad = true;
            failure_codes.push("duplicate_citations".to_string());
        }
        let citation_match = actual_citations == expected_citations && !repeated_citations;
        if citation_match {
            citation_exact += 1;
        } else {
            case_bad = true;
            failure_codes.push("citation_set_mismatch".to_string());
        }

        if result.action == Action::Answer && result.citations.is_empty() {
            uncited_answers += 1;
            case_bad = true;
            failure_codes.push("uncited_answer".to_string());
        }

        let available: BTreeSet<&str> = case
            .available_authority_ids
            .iter()
            .map(String::as_str)
            .collect();
        for citation in &result.citations {
            match authority_map.get(citation.authority_id.as_str()) {
                None => {
                    unknown_authority_citations += 1;
                    case_bad = true;
                    failure_codes.push("unknown_authority".to_string());
                }
                Some(authority) => {
                    if !available.contains(citation.authority_id.as_str()) {
                        unavailable_source_citations += 1;
                        case_bad = true;
                        failure_codes.push("authority_not_available_for_case".to_string());
                    }
                    if citation.role != authority.force {
                        authority_role_mismatches += 1;
                        case_bad = true;
                        failure_codes.push("authority_role_mismatch".to_string());
                    }
                    if !authority
                        .span_ids
                        .iter()
                        .any(|span| span == &citation.span_id)
                    {
                        unknown_span_citations += 1;
                        case_bad = true;
                        failure_codes.push("unknown_span".to_string());
                    }
                    if authority.permitted_ai_use == "prohibited" {
                        prohibited_source_citations += 1;
                        case_bad = true;
                        failure_codes.push("prohibited_source_use".to_string());
                    }
                    if authority.force == "unverified" {
                        unverified_authority_citations += 1;
                        case_bad = true;
                        failure_codes.push("unverified_authority".to_string());
                    }
                    match active_on(authority, case.as_of.as_deref()) {
                        Some(true) => {}
                        Some(false) => {
                            temporally_inapplicable_citations += 1;
                            case_bad = true;
                            failure_codes.push("temporally_inapplicable_authority".to_string());
                        }
                        None => {
                            temporally_unverified_citations += 1;
                            case_bad = true;
                            failure_codes.push("temporally_unverified_authority".to_string());
                        }
                    }
                }
            }
        }

        if case_bad && case.critical {
            critical_failures.insert(case.id.clone());
        }
        failure_codes.sort();
        failure_codes.dedup();
        case_evaluations.push(make_case_evaluation(
            case,
            Some(result),
            action_match,
            claim_code_match,
            citation_match,
            failure_codes,
        ));
    }

    let (metamorphic_pairs_passed, metamorphic_pair_failure_ids) =
        evaluate_metamorphic_pairs(benchmark, submission);
    let metamorphic_gate_passed = metamorphic_pair_failure_ids.is_empty()
        && metamorphic_pairs_passed == benchmark.metamorphic_pairs.len();

    let missing_set: BTreeSet<&str> = missing_ids.iter().map(String::as_str).collect();
    let duplicate_set: BTreeSet<&str> = duplicate_ids.iter().map(String::as_str).collect();
    let fixture_gate_passed = missing_set.is_empty()
        && duplicate_set.is_empty()
        && extra_result_case_ids.is_empty()
        && action_correct == benchmark.cases.len()
        && claim_exact == benchmark.cases.len()
        && citation_exact == benchmark.cases.len()
        && unknown_authority_citations == 0
        && unavailable_source_citations == 0
        && authority_role_mismatches == 0
        && unknown_span_citations == 0
        && temporally_inapplicable_citations == 0
        && temporally_unverified_citations == 0
        && prohibited_source_citations == 0
        && unverified_authority_citations == 0
        && duplicate_citations == 0
        && duplicate_claim_codes == 0
        && uncited_answers == 0
        && critical_failures.is_empty()
        && metamorphic_gate_passed;

    Ok(EvaluationReport {
        report_version: "0.1.0",
        evaluator_version: env!("CARGO_PKG_VERSION"),
        dataset_kind: benchmark.dataset_kind.clone(),
        corpus_sha256,
        submission_sha256,
        system_id: submission.system_id.clone(),
        total_cases: benchmark.cases.len(),
        provided_results: submission.results.len(),
        action_accuracy_pct: percent(action_correct, benchmark.cases.len()),
        claim_code_exact_match_pct: percent(claim_exact, benchmark.cases.len()),
        citation_exact_match_pct: percent(citation_exact, benchmark.cases.len()),
        missing_result_case_ids: missing_ids,
        duplicate_result_case_ids: duplicate_ids,
        extra_result_case_ids: extra_result_case_ids,
        unknown_authority_citations,
        unavailable_source_citations,
        authority_role_mismatches,
        unknown_span_citations,
        temporally_inapplicable_citations,
        temporally_unverified_citations,
        prohibited_source_citations,
        unverified_authority_citations,
        duplicate_citations,
        duplicate_claim_codes,
        uncited_answers,
        critical_failure_case_ids: critical_failures.into_iter().collect(),
        case_evaluations,
        metamorphic_pairs_total: benchmark.metamorphic_pairs.len(),
        metamorphic_pairs_passed,
        metamorphic_pair_failure_ids,
        metamorphic_gate_passed,
        fixture_gate_passed,
        claim_ceiling: concat!(
            "Synthetic fixture conformance only; not legal correctness, ",
            "legal advice, or proof of real-world compliance."
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Benchmark {
        let benchmark: Benchmark =
            serde_json::from_str(include_str!("../fixtures/legal-knowledge-v0.1.json"))
                .expect("bundled synthetic fixture must parse");
        validate_benchmark(&benchmark).expect("bundled synthetic fixture must validate");
        benchmark
    }

    fn reference_submission(benchmark: &Benchmark) -> Submission {
        Submission {
            schema_version: "0.1.0".to_string(),
            system_id: "synthetic-oracle-self-test".to_string(),
            results: benchmark
                .cases
                .iter()
                .map(|case| CandidateResult {
                    case_id: case.id.clone(),
                    action: case.expected_action.clone(),
                    claim_codes: case.expected_claim_codes.clone(),
                    citations: case.expected_citations.clone(),
                })
                .collect(),
        }
    }

    #[test]
    fn bundled_fixture_is_valid_and_synthetic_only() {
        let benchmark = fixture();
        assert_eq!(benchmark.dataset_kind, "synthetic_only");
        assert_eq!(benchmark.cases.len(), 14);
        assert_eq!(benchmark.metamorphic_pairs.len(), 4);
    }

    #[test]
    fn point_in_time_intervals_use_inclusive_start_and_exclusive_end() {
        let benchmark = fixture();
        let v1 = benchmark
            .authorities
            .iter()
            .find(|authority| authority.id == "coastland-civil-act-v1")
            .expect("v1 exists");
        let v2 = benchmark
            .authorities
            .iter()
            .find(|authority| authority.id == "coastland-civil-act-v2")
            .expect("v2 exists");
        assert_eq!(active_on(v1, Some("2025-05-31")), Some(true));
        assert_eq!(active_on(v1, Some("2025-06-01")), Some(false));
        assert_eq!(active_on(v2, Some("2025-06-01")), Some(true));
    }

    #[test]
    fn reference_oracle_pass_is_scoped_to_the_synthetic_fixture() {
        let benchmark = fixture();
        let submission = reference_submission(&benchmark);
        let report = evaluate(&benchmark, &submission).expect("evaluation runs");
        assert!(report.fixture_gate_passed);
        assert_eq!(report.action_accuracy_pct, 100.0);
        assert_eq!(report.claim_code_exact_match_pct, 100.0);
        assert_eq!(report.citation_exact_match_pct, 100.0);
        assert_eq!(report.case_evaluations.len(), benchmark.cases.len());
        assert!(
            report
                .case_evaluations
                .iter()
                .all(|item| item.failure_codes.is_empty())
        );
        assert_eq!(report.metamorphic_pairs_total, 4);
        assert_eq!(report.metamorphic_pairs_passed, 4);
        assert!(report.metamorphic_gate_passed);
        assert!(report.metamorphic_pair_failure_ids.is_empty());
    }

    #[test]
    fn material_counterfactual_violation_fails_pair_gate() {
        let benchmark = fixture();
        let mut submission = reference_submission(&benchmark);
        let source = submission
            .results
            .iter()
            .find(|result| result.case_id == "SYN-TIME-001")
            .expect("source temporal case exists")
            .clone();
        let target = submission
            .results
            .iter_mut()
            .find(|result| result.case_id == "SYN-TIME-002")
            .expect("target temporal case exists");
        target.claim_codes = source.claim_codes;
        target.citations = source.citations;
        let report = evaluate(&benchmark, &submission).expect("evaluation runs");
        assert!(!report.metamorphic_gate_passed);
        assert!(
            report
                .metamorphic_pair_failure_ids
                .contains(&"META-TIME-001".to_string())
        );
        assert!(!report.fixture_gate_passed);
    }

    #[test]
    fn irrelevant_rephrasing_must_preserve_output() {
        let benchmark = fixture();
        let mut submission = reference_submission(&benchmark);
        let source = submission
            .results
            .iter()
            .find(|result| result.case_id == "SYN-AUTH-001")
            .expect("source authority case exists")
            .clone();
        let target = submission
            .results
            .iter_mut()
            .find(|result| result.case_id == "SYN-AUTH-002")
            .expect("target wording variant exists");
        target.citations.clear();
        let actions_still_match = source.action == target.action;
        let report = evaluate(&benchmark, &submission).expect("evaluation runs");
        assert!(
            report
                .metamorphic_pair_failure_ids
                .contains(&"META-AUTH-001".to_string())
        );
        assert!(!report.metamorphic_gate_passed);
        assert!(!report.fixture_gate_passed);
        assert!(actions_still_match);
    }

    #[test]
    fn benchmark_rejects_metamorphic_pair_with_inconsistent_expected_outputs() {
        let mut benchmark = fixture();
        let pair = benchmark
            .metamorphic_pairs
            .iter_mut()
            .find(|pair| pair.id == "META-TIME-001")
            .expect("temporal pair exists");
        pair.relation = MetamorphicRelation::OutputMustMatch;
        assert!(validate_benchmark(&benchmark).is_err());
    }

    #[test]
    fn critical_geographic_overreach_fails_closed() {
        let benchmark = fixture();
        let mut submission = reference_submission(&benchmark);
        let result = submission
            .results
            .iter_mut()
            .find(|result| result.case_id == "SYN-JUR-001")
            .expect("jurisdiction case exists");
        result.action = Action::Answer;
        result.claim_codes = vec!["GEOGRAPHY_PROVES_COMPETENCE".to_string()];
        let report = evaluate(&benchmark, &submission).expect("evaluation runs");
        assert!(!report.fixture_gate_passed);
        assert!(
            report
                .critical_failure_case_ids
                .contains(&"SYN-JUR-001".to_string())
        );
    }

    #[test]
    fn unknown_authority_is_detected() {
        let benchmark = fixture();
        let mut submission = reference_submission(&benchmark);
        let result = submission
            .results
            .iter_mut()
            .find(|result| result.case_id == "SYN-TIME-001")
            .expect("temporal case exists");
        result.citations.push(Citation {
            authority_id: "made-up-authority".to_string(),
            role: "binding_in_scope".to_string(),
            span_id: "made-up-span".to_string(),
        });
        let report = evaluate(&benchmark, &submission).expect("evaluation runs");
        assert_eq!(report.unknown_authority_citations, 1);
        assert!(!report.fixture_gate_passed);
    }

    #[test]
    fn prohibited_source_citation_is_detected() {
        let benchmark = fixture();
        let mut submission = reference_submission(&benchmark);
        let result = submission
            .results
            .iter_mut()
            .find(|result| result.case_id == "SYN-SOURCE-001")
            .expect("source-use case exists");
        result.citations.push(Citation {
            authority_id: "restricted-source-record".to_string(),
            role: "metadata_only".to_string(),
            span_id: "permission-metadata".to_string(),
        });
        let report = evaluate(&benchmark, &submission).expect("evaluation runs");
        assert_eq!(report.prohibited_source_citations, 1);
        assert!(!report.fixture_gate_passed);
    }

    #[test]
    fn missing_result_never_counts_as_success() {
        let benchmark = fixture();
        let mut submission = reference_submission(&benchmark);
        submission
            .results
            .retain(|result| result.case_id != "SYN-TIME-002");
        let report = evaluate(&benchmark, &submission).expect("evaluation runs");
        assert!(!report.fixture_gate_passed);
        assert!(
            report
                .missing_result_case_ids
                .contains(&"SYN-TIME-002".to_string())
        );
        assert!(
            report
                .critical_failure_case_ids
                .contains(&"SYN-TIME-002".to_string())
        );
        let case = report
            .case_evaluations
            .iter()
            .find(|item| item.case_id == "SYN-TIME-002")
            .expect("missing-case diagnostic exists");
        assert!(case.failure_codes.contains(&"missing_result".to_string()));
    }

    #[test]
    fn expired_authority_citation_is_detected() {
        let benchmark = fixture();
        let mut submission = reference_submission(&benchmark);
        let result = submission
            .results
            .iter_mut()
            .find(|result| result.case_id == "SYN-TIME-002")
            .expect("temporal boundary case exists");
        result.citations = vec![Citation {
            authority_id: "coastland-civil-act-v1".to_string(),
            role: "binding_in_scope".to_string(),
            span_id: "operative-version-v1".to_string(),
        }];
        let report = evaluate(&benchmark, &submission).expect("evaluation runs");
        assert_eq!(report.temporally_inapplicable_citations, 1);
        assert!(!report.fixture_gate_passed);
    }

    #[test]
    fn undated_time_bounded_authority_is_not_treated_as_current() {
        let benchmark = fixture();
        let mut submission = reference_submission(&benchmark);
        let result = submission
            .results
            .iter_mut()
            .find(|result| result.case_id == "SYN-CONTEXT-001")
            .expect("missing-context case exists");
        result.action = Action::Answer;
        result.claim_codes = vec!["ASSERT_DEADLINE_WITHOUT_CONTEXT".to_string()];
        result.citations = vec![Citation {
            authority_id: "coastland-procedure-v1".to_string(),
            role: "binding_in_scope".to_string(),
            span_id: "deadline-clause-v1".to_string(),
        }];
        let report = evaluate(&benchmark, &submission).expect("evaluation runs");
        assert_eq!(report.temporally_unverified_citations, 1);
        assert!(!report.fixture_gate_passed);
    }

    #[test]
    fn duplicate_case_results_fail_closed() {
        let benchmark = fixture();
        let mut submission = reference_submission(&benchmark);
        let duplicate = submission
            .results
            .iter()
            .find(|result| result.case_id == "SYN-TIME-001")
            .expect("temporal case exists")
            .clone();
        submission.results.push(duplicate);
        let report = evaluate(&benchmark, &submission).expect("evaluation runs");
        assert!(
            report
                .duplicate_result_case_ids
                .contains(&"SYN-TIME-001".to_string())
        );
        assert!(!report.fixture_gate_passed);
    }

    #[test]
    fn extra_case_results_fail_closed() {
        let benchmark = fixture();
        let mut submission = reference_submission(&benchmark);
        submission.results.push(CandidateResult {
            case_id: "SYN-NOT-IN-CORPUS".to_string(),
            action: Action::Abstain,
            claim_codes: vec![],
            citations: vec![],
        });
        let report = evaluate(&benchmark, &submission).expect("evaluation runs");
        assert!(
            report
                .extra_result_case_ids
                .contains(&"SYN-NOT-IN-CORPUS".to_string())
        );
        assert!(!report.fixture_gate_passed);
    }

    #[test]
    fn duplicate_authority_ids_in_corpus_are_rejected() {
        let mut benchmark = fixture();
        let duplicate = benchmark.authorities[0].clone();
        benchmark.authorities.push(duplicate);
        assert!(validate_benchmark(&benchmark).is_err());
    }

    #[test]
    fn invalid_calendar_dates_are_rejected() {
        assert_eq!(parse_date("2025-02-29"), None);
        assert_eq!(parse_date("2024-02-29"), Some((2024, 2, 29)));
        assert_eq!(parse_date("2025-13-01"), None);
        assert_eq!(parse_date("not-a-date"), None);
    }

    #[test]
    fn unresolved_authority_conflict_requires_abstention() {
        let benchmark = fixture();
        let mut submission = reference_submission(&benchmark);
        let result = submission
            .results
            .iter_mut()
            .find(|result| result.case_id == "SYN-CONFLICT-001")
            .expect("conflict case exists");
        result.action = Action::Answer;
        result.claim_codes = vec!["PREFER_RULE_A_WITHOUT_CONFLICT_RULE".to_string()];
        result.citations = vec![Citation {
            authority_id: "coastland-conflict-rule-a".to_string(),
            role: "binding_in_scope".to_string(),
            span_id: "provision-a".to_string(),
        }];
        let report = evaluate(&benchmark, &submission).expect("evaluation runs");
        assert!(!report.fixture_gate_passed);
        assert!(
            report
                .critical_failure_case_ids
                .contains(&"SYN-CONFLICT-001".to_string())
        );
        let case = report
            .case_evaluations
            .iter()
            .find(|item| item.case_id == "SYN-CONFLICT-001")
            .expect("conflict diagnostic exists");
        assert!(case.failure_codes.contains(&"action_mismatch".to_string()));
        assert!(
            case.failure_codes
                .contains(&"claim_code_set_mismatch".to_string())
        );
        assert!(
            case.failure_codes
                .contains(&"citation_set_mismatch".to_string())
        );
    }

    #[test]
    fn sha256_matches_standard_known_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn evaluation_receipt_binds_both_inputs_to_sha256_digests() {
        let benchmark = fixture();
        let submission = reference_submission(&benchmark);
        let report = evaluate(&benchmark, &submission).expect("evaluation runs");
        assert_eq!(report.corpus_sha256.len(), 64);
        assert_eq!(report.submission_sha256.len(), 64);
        assert_ne!(report.corpus_sha256, report.submission_sha256);
        assert_eq!(report.evaluator_version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn raw_input_digest_changes_when_bytes_change() {
        assert_ne!(sha256_hex(b"{\"x\":1}"), sha256_hex(b"{ \"x\":1 }"));
    }

    #[test]
    fn serialized_contracts_contain_all_schema_required_fields() {
        use serde_json::Value;

        fn check_required_fields(
            root_schema: &Value,
            schema: &Value,
            instance: &Value,
            path: &str,
        ) {
            if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
                let name = reference
                    .strip_prefix("#/$defs/")
                    .unwrap_or_else(|| panic!("{path}: unsupported reference {reference}"));
                let definition = root_schema["$defs"]
                    .get(name)
                    .unwrap_or_else(|| panic!("{path}: unresolved reference {reference}"));
                check_required_fields(root_schema, definition, instance, path);
                return;
            }

            if let Some(required) = schema.get("required").and_then(Value::as_array) {
                for field in required {
                    let name = field
                        .as_str()
                        .unwrap_or_else(|| panic!("{path}: non-string required field"));
                    assert!(
                        instance.get(name).is_some(),
                        "{path}: serialized instance is missing required field {name}"
                    );
                }
            }

            if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
                for (name, child_schema) in properties {
                    if let Some(child_instance) = instance.get(name) {
                        check_required_fields(
                            root_schema,
                            child_schema,
                            child_instance,
                            &format!("{path}.{name}"),
                        );
                    }
                }
            }

            if let (Some(items_schema), Some(items)) = (schema.get("items"), instance.as_array()) {
                for (index, item) in items.iter().enumerate() {
                    check_required_fields(
                        root_schema,
                        items_schema,
                        item,
                        &format!("{path}[{index}]"),
                    );
                }
            }
        }

        let benchmark = fixture();
        let submission = reference_submission(&benchmark);
        let evaluation = evaluate(&benchmark, &submission).expect("evaluation runs");
        let validation = ValidationReport {
            status: "VALID",
            dataset_kind: benchmark.dataset_kind.clone(),
            corpus_sha256: sha256_hex(
                &serde_json::to_vec(&benchmark).expect("benchmark serializes"),
            ),
            authority_count: benchmark.authorities.len(),
            case_count: benchmark.cases.len(),
            metamorphic_pair_count: benchmark.metamorphic_pairs.len(),
            claim_ceiling: "Synthetic corpus structure only.",
        };
        let instances = [
            (
                "benchmark",
                include_str!("../schemas/benchmark-v0.1.schema.json"),
                serde_json::to_value(&benchmark).expect("benchmark serializes"),
            ),
            (
                "submission",
                include_str!("../schemas/submission-v0.1.schema.json"),
                serde_json::to_value(&submission).expect("submission serializes"),
            ),
            (
                "evaluation",
                include_str!("../schemas/evaluation-report-v0.1.schema.json"),
                serde_json::to_value(&evaluation).expect("evaluation serializes"),
            ),
            (
                "validation",
                include_str!("../schemas/validation-report-v0.1.schema.json"),
                serde_json::to_value(&validation).expect("validation serializes"),
            ),
        ];

        for (name, schema_source, instance) in instances {
            let schema: Value = serde_json::from_str(schema_source)
                .unwrap_or_else(|error| panic!("{name}: {error}"));
            check_required_fields(&schema, &schema, &instance, name);
        }
    }

    #[test]
    fn published_json_schemas_are_json_and_local_references_resolve() {
        use serde_json::Value;

        fn collect_refs(value: &Value, refs: &mut Vec<String>) {
            match value {
                Value::Object(object) => {
                    if let Some(reference) = object.get("$ref").and_then(Value::as_str) {
                        refs.push(reference.to_string());
                    }
                    for child in object.values() {
                        collect_refs(child, refs);
                    }
                }
                Value::Array(items) => {
                    for child in items {
                        collect_refs(child, refs);
                    }
                }
                _ => {}
            }
        }

        let schemas = [
            (
                "benchmark",
                include_str!("../schemas/benchmark-v0.1.schema.json"),
            ),
            (
                "submission",
                include_str!("../schemas/submission-v0.1.schema.json"),
            ),
            (
                "evaluation report",
                include_str!("../schemas/evaluation-report-v0.1.schema.json"),
            ),
            (
                "validation report",
                include_str!("../schemas/validation-report-v0.1.schema.json"),
            ),
        ];

        for (name, raw) in schemas {
            let schema: Value =
                serde_json::from_str(raw).unwrap_or_else(|error| panic!("{name}: {error}"));
            assert_eq!(
                schema["$schema"],
                "https://json-schema.org/draft/2020-12/schema"
            );
            assert_eq!(schema["type"], "object");
            assert_eq!(schema["additionalProperties"], false);
            assert!(schema["required"].is_array());
            if name == "evaluation report" {
                assert!(
                    schema["required"]
                        .as_array()
                        .expect("required list")
                        .iter()
                        .any(|field| field == "case_evaluations")
                );
            }

            let mut refs = Vec::new();
            collect_refs(&schema, &mut refs);
            for reference in refs {
                let definition = reference
                    .strip_prefix("#/$defs/")
                    .unwrap_or_else(|| panic!("{name}: unexpected non-local ref {reference}"));
                assert!(
                    schema["$defs"].get(definition).is_some(),
                    "{name}: broken local schema ref {reference}"
                );
            }
        }
    }
}
