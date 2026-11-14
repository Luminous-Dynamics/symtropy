// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Oracle-bound metamorphic campaign runner for BUILT-INTEROP-008.
//!
//! The campaign labels are checked against a closed mutation→oracle table
//! before the tool is invoked. Tool output never defines expected outcomes.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const CAMPAIGN_SCHEMA_VERSION: &str = "1.0.0";
const CAMPAIGN_ID: &str = "BUILT-INTEROP-009";
const TOOL_BENCHMARK_ID: &str = "BUILT-INTEROP-008";
const TOOL_PROFILE: &str = "IFC4/schema-directed-structural-diff-v0.1";
const BASE_FIXTURE_SHA256: &str =
    "5196f603e6f091a402dc67c1e2729a0f3b37da12a505f7c6f96696509187899a";
const BASE_FIXTURE_BYTES: usize = 424;
const POLICY: &str =
    "BUILT-INTEROP-009/v0.1;oracle=closed-synthetic-metamorphic-table;compare=typed-json;status=exit-code-and-report-agree;source-target-byte-digests=verified;false-acceptance-rejection-unresolved=separate";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum ExpectedStatus {
    Equal,
    Different,
    Unresolved,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Mutation {
    Identical,
    HeaderTimestamp,
    RootStepRenumber,
    RootRecordReorder,
    WallNameChange,
    RootRemoved,
    RootGuidChanged,
    InvalidGuid,
    DuplicateGuid,
    UnknownSchema,
    MalformedStep,
}

impl Mutation {
    const ORDERED: [Self; 11] = [
        Self::Identical,
        Self::HeaderTimestamp,
        Self::RootStepRenumber,
        Self::RootRecordReorder,
        Self::WallNameChange,
        Self::RootRemoved,
        Self::RootGuidChanged,
        Self::InvalidGuid,
        Self::DuplicateGuid,
        Self::UnknownSchema,
        Self::MalformedStep,
    ];

    fn expected(self) -> ExpectedStatus {
        match self {
            Self::Identical
            | Self::HeaderTimestamp
            | Self::RootStepRenumber
            | Self::RootRecordReorder => ExpectedStatus::Equal,
            Self::WallNameChange | Self::RootRemoved | Self::RootGuidChanged => {
                ExpectedStatus::Different
            }
            Self::InvalidGuid
            | Self::DuplicateGuid
            | Self::UnknownSchema
            | Self::MalformedStep => ExpectedStatus::Unresolved,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureSpec {
    path: String,
    sha256: String,
    byte_length: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OracleCase {
    case_id: String,
    mutation: Mutation,
    expected_status: ExpectedStatus,
    oracle_basis: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CampaignManifest {
    schema_version: String,
    benchmark_id: String,
    oracle_class: String,
    base_fixture: FixtureSpec,
    cases: Vec<OracleCase>,
}

#[derive(Debug, Clone)]
struct Args {
    diff_bin: PathBuf,
    manifest: PathBuf,
    run_id: String,
    reported_at: String,
    diff_tool_source_commit_sha: String,
    evaluator_source_commit_sha: String,
    diff_bin_sha256: String,
    output: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
struct ToolIdentity {
    name: String,
    version: String,
    source_commit_sha: String,
    binary_sha256: String,
    config_sha256: String,
}

#[derive(Debug, Clone, Serialize)]
struct CaseResult {
    case_id: String,
    mutation: Mutation,
    oracle_class: String,
    oracle_basis: String,
    expected_status: ExpectedStatus,
    observed_status: ExpectedStatus,
    exact_match: bool,
    false_acceptance: bool,
    false_rejection: bool,
    unsafe_certainty: bool,
    unresolved_instead_of_decision: bool,
    source_sha256: String,
    target_sha256: String,
    target_byte_length: usize,
    semantic_result_sha256: String,
    decision_reason: String,
    diff_tool_version: String,
    diff_tool_config_sha256: String,
    elapsed_millis: u128,
}

#[derive(Debug, Clone, Serialize)]
struct Summary {
    case_count: usize,
    exact_matches: usize,
    outcome_mismatches: usize,
    expected_equal: usize,
    expected_different: usize,
    expected_unresolved: usize,
    false_acceptances: usize,
    false_rejections: usize,
    unsafe_certainty: usize,
    unresolved_instead_of_decision: usize,
    exact_match_rate: f64,
}

#[derive(Debug, Clone, Serialize)]
struct CampaignReport {
    schema_version: String,
    benchmark_id: String,
    oracle_class: String,
    run_id: String,
    reported_at: String,
    campaign_status: String,
    base_fixture_sha256: String,
    base_fixture_byte_length: usize,
    manifest_sha256: String,
    policy_sha256: String,
    diff_tool: ToolIdentity,
    evaluator_source_commit_sha: String,
    cases: Vec<CaseResult>,
    summary: Summary,
    limitations: Vec<String>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn valid_hex(value: &str, size: usize) -> bool {
    value.len() == size
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_run_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b':' | b'-'))
}

fn valid_timestamp(value: &str) -> bool {
    let b = value.as_bytes();
    if b.len() < 20
        || !b[0..4].iter().all(u8::is_ascii_digit)
        || b[4] != b'-'
        || !b[5..7].iter().all(u8::is_ascii_digit)
        || b[7] != b'-'
        || !b[8..10].iter().all(u8::is_ascii_digit)
        || b[10] != b'T'
        || !b[11..13].iter().all(u8::is_ascii_digit)
        || b[13] != b':'
        || !b[14..16].iter().all(u8::is_ascii_digit)
        || b[16] != b':'
        || !b[17..19].iter().all(u8::is_ascii_digit)
    {
        return false;
    }
    let number = |start: usize, end: usize| -> u32 {
        value[start..end].parse().unwrap_or(u32::MAX)
    };
    let year = number(0, 4);
    let month = number(5, 7);
    let day = number(8, 10);
    let hour = number(11, 13);
    let minute = number(14, 16);
    let second = number(17, 19);
    if !(1..=12).contains(&month) || !(1..=31).contains(&day)
        || hour > 23 || minute > 59 || second > 60
    {
        return false;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let max_day = match month {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    if day > max_day {
        return false;
    }

    let mut zone = &value[19..];
    if let Some(fraction) = zone.strip_prefix('.') {
        let digits = fraction.bytes().take_while(u8::is_ascii_digit).count();
        if digits == 0 {
            return false;
        }
        zone = &fraction[digits..];
    }
    if zone == "Z" {
        return true;
    }
    let z = zone.as_bytes();
    if z.len() != 6
        || !matches!(z[0], b'+' | b'-')
        || z[3] != b':'
        || !z[1..3].iter().all(u8::is_ascii_digit)
        || !z[4..6].iter().all(u8::is_ascii_digit)
    {
        return false;
    }
    let offset_hour = zone[1..3].parse::<u32>().unwrap_or(u32::MAX);
    let offset_minute = zone[4..6].parse::<u32>().unwrap_or(u32::MAX);
    offset_hour <= 23 && offset_minute <= 59
}

fn expected_case_id(mutation: Mutation) -> &'static str {
    match mutation {
        Mutation::Identical => "METAMORPHIC-001",
        Mutation::HeaderTimestamp => "METAMORPHIC-002",
        Mutation::RootStepRenumber => "METAMORPHIC-003",
        Mutation::RootRecordReorder => "METAMORPHIC-004",
        Mutation::WallNameChange => "MUTATION-001",
        Mutation::RootRemoved => "MUTATION-002",
        Mutation::RootGuidChanged => "MUTATION-003",
        Mutation::InvalidGuid => "AMBIGUITY-001",
        Mutation::DuplicateGuid => "AMBIGUITY-002",
        Mutation::UnknownSchema => "AMBIGUITY-003",
        Mutation::MalformedStep => "AMBIGUITY-004",
    }
}

fn validate_manifest(manifest: &CampaignManifest, fixture_bytes: &[u8]) -> Result<(), String> {
    if manifest.schema_version != CAMPAIGN_SCHEMA_VERSION
        || manifest.benchmark_id != CAMPAIGN_ID
        || manifest.oracle_class != "synthetic-metamorphic-v1"
    {
        return Err("CAMPAIGN_MANIFEST_PROFILE_MISMATCH".into());
    }
    if manifest.base_fixture.path != "fixtures/two-walls.ifc"
        || manifest.base_fixture.sha256 != BASE_FIXTURE_SHA256
        || manifest.base_fixture.byte_length != BASE_FIXTURE_BYTES
        || fixture_bytes.len() != BASE_FIXTURE_BYTES
        || sha256_hex(fixture_bytes) != BASE_FIXTURE_SHA256
    {
        return Err("BASE_FIXTURE_IDENTITY_MISMATCH".into());
    }
    if manifest.cases.len() != Mutation::ORDERED.len() {
        return Err("ORACLE_CASE_COUNT_MISMATCH".into());
    }
    let mut ids = BTreeSet::new();
    let mut mutations = BTreeSet::new();
    for (index, case) in manifest.cases.iter().enumerate() {
        if case.case_id != expected_case_id(case.mutation)
            || case.mutation != Mutation::ORDERED[index]
            || case.expected_status != case.mutation.expected()
            || case.oracle_basis.trim().is_empty()
        {
            return Err(format!("ORACLE_CASE_CONTRACT_MISMATCH:{}", case.case_id));
        }
        if !ids.insert(case.case_id.as_str()) || !mutations.insert(case.mutation as u8) {
            return Err("DUPLICATE_ORACLE_CASE".into());
        }
    }
    Ok(())
}

fn replace_once(source: &str, from: &str, to: &str, mutation: Mutation) -> Result<String, String> {
    if !source.contains(from) {
        return Err(format!("MUTATION_ANCHOR_MISSING:{mutation:?}"));
    }
    let target = source.replacen(from, to, 1);
    if target == source {
        return Err(format!("MUTATION_NO_OP:{mutation:?}"));
    }
    Ok(target)
}

fn apply_mutation(source: &[u8], mutation: Mutation) -> Result<Vec<u8>, String> {
    let text = std::str::from_utf8(source)
        .map_err(|_| "BASE_FIXTURE_NOT_UTF8".to_string())?;
    let target = match mutation {
        Mutation::Identical => return Ok(source.to_vec()),
        Mutation::HeaderTimestamp => replace_once(
            text,
            "'2026-10-18T00:00:00'",
            "'2026-10-19T00:00:00'",
            mutation,
        )?,
        Mutation::RootStepRenumber => {
            if !text.contains("#1=IFCWALL") || !text.contains("#2=IFCWALL") {
                return Err("MUTATION_ANCHOR_MISSING:RootStepRenumber".into());
            }
            text.replace("#1=IFCWALL", "#17=IFCWALL")
                .replace("#2=IFCWALL", "#18=IFCWALL")
        }
        Mutation::RootRecordReorder => {
            let first = "#1=IFCWALL('0000000000000000000000',$,'Panel 001',$,$,$,$,$,.NOTDEFINED.);\n";
            let second = "#2=IFCWALL('0000000000000000000001',$,'Panel 002',$,$,$,$,$,.NOTDEFINED.);\n";
            replace_once(text, &format!("{first}{second}"), &format!("{second}{first}"), mutation)?
        }
        Mutation::WallNameChange => replace_once(text, "'Panel 001'", "'Panel 009'", mutation)?,
        Mutation::RootRemoved => {
            let second = "#2=IFCWALL('0000000000000000000001',$,'Panel 002',$,$,$,$,$,.NOTDEFINED.);\n";
            replace_once(text, second, "", mutation)?
        }
        Mutation::RootGuidChanged => replace_once(
            text,
            "'0000000000000000000000'",
            "'0000000000000000000002'",
            mutation,
        )?,
        Mutation::InvalidGuid => replace_once(
            text,
            "'0000000000000000000000'",
            "'not-a-guid'",
            mutation,
        )?,
        Mutation::DuplicateGuid => replace_once(
            text,
            "'0000000000000000000001'",
            "'0000000000000000000000'",
            mutation,
        )?,
        Mutation::UnknownSchema => replace_once(
            text,
            "FILE_SCHEMA(('IFC4'));",
            "FILE_SCHEMA(('IFC_FUTURE_TEST'));",
            mutation,
        )?,
        Mutation::MalformedStep => replace_once(
            text,
            "ISO-10303-21;",
            "BROKEN-STEP;",
            mutation,
        )?,
    };
    Ok(target.into_bytes())
}

fn observed_status(value: &Value, exit_code: i32) -> Result<ExpectedStatus, String> {
    let from_json = match value.get("status").and_then(Value::as_str) {
        Some("EQUAL") => ExpectedStatus::Equal,
        Some("DIFFERENT") => ExpectedStatus::Different,
        Some("UNRESOLVED") => ExpectedStatus::Unresolved,
        _ => return Err("DIFF_REPORT_STATUS_INVALID".into()),
    };
    let from_exit = match exit_code {
        0 => ExpectedStatus::Equal,
        1 => ExpectedStatus::Different,
        2 => ExpectedStatus::Unresolved,
        _ => return Err(format!("DIFF_TOOL_UNEXPECTED_EXIT:{exit_code}")),
    };
    if from_json != from_exit {
        return Err("DIFF_TOOL_STATUS_EXIT_MISMATCH".into());
    }
    Ok(from_json)
}

fn run_one(
    binary: &Path,
    source_path: &Path,
    target_path: &Path,
    source_bytes: &[u8],
    target_bytes: &[u8],
    oracle: &OracleCase,
    oracle_class: &str,
) -> Result<CaseResult, String> {
    let started = Instant::now();
    let output = Command::new(binary)
        .arg("--source-ifc")
        .arg(source_path)
        .arg("--target-ifc")
        .arg(target_path)
        .output()
        .map_err(|error| format!("DIFF_TOOL_LAUNCH_FAILED:{error}"))?;
    let exit_code = output.status.code().unwrap_or(255);
    if exit_code > 2 {
        return Err(format!(
            "DIFF_TOOL_FAILED:{}:stderr={}",
            oracle.case_id,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let json: Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("DIFF_TOOL_JSON_INVALID:{}:{error}", oracle.case_id))?;
    let observed = observed_status(&json, exit_code)?;
    if json["benchmark_id"].as_str() != Some(TOOL_BENCHMARK_ID)
        || json["profile"].as_str() != Some(TOOL_PROFILE)
        || !json["config_sha256"]
            .as_str()
            .is_some_and(|value| valid_hex(value, 64))
        || !json["semantic_result_sha256"]
            .as_str()
            .is_some_and(|value| valid_hex(value, 64))
    {
        return Err(format!("DIFF_TOOL_PROFILE_OR_DIGEST_INVALID:{}", oracle.case_id));
    }
    if json["source_artifact"]["sha256"].as_str() != Some(sha256_hex(source_bytes).as_str())
        || json["source_artifact"]["byte_length"].as_u64()
            != u64::try_from(source_bytes.len()).ok()
        || json["target_artifact"]["sha256"].as_str() != Some(sha256_hex(target_bytes).as_str())
        || json["target_artifact"]["byte_length"].as_u64()
            != u64::try_from(target_bytes.len()).ok()
    {
        return Err(format!("DIFF_TOOL_ARTIFACT_DIGEST_MISMATCH:{}", oracle.case_id));
    }
    let tool_version = json["tool_version"]
        .as_str()
        .filter(|version| !version.trim().is_empty())
        .ok_or_else(|| format!("DIFF_TOOL_VERSION_MISSING:{}", oracle.case_id))?
        .to_owned();
    let tool_config_sha256 = json["config_sha256"]
        .as_str()
        .expect("validated above")
        .to_owned();
    let false_acceptance =
        oracle.expected_status == ExpectedStatus::Different && observed == ExpectedStatus::Equal;
    let false_rejection =
        oracle.expected_status == ExpectedStatus::Equal && observed == ExpectedStatus::Different;
    let unsafe_certainty =
        oracle.expected_status == ExpectedStatus::Unresolved && observed != ExpectedStatus::Unresolved;
    let unresolved_instead_of_decision =
        oracle.expected_status != ExpectedStatus::Unresolved && observed == ExpectedStatus::Unresolved;

    Ok(CaseResult {
        case_id: oracle.case_id.clone(),
        mutation: oracle.mutation,
        oracle_class: oracle_class.to_owned(),
        oracle_basis: oracle.oracle_basis.clone(),
        expected_status: oracle.expected_status,
        observed_status: observed,
        exact_match: oracle.expected_status == observed,
        false_acceptance,
        false_rejection,
        unsafe_certainty,
        unresolved_instead_of_decision,
        source_sha256: sha256_hex(source_bytes),
        target_sha256: sha256_hex(target_bytes),
        target_byte_length: target_bytes.len(),
        semantic_result_sha256: json["semantic_result_sha256"].as_str().unwrap().to_owned(),
        decision_reason: json["decision_reason"].as_str().unwrap_or("UNSPECIFIED").to_owned(),
        diff_tool_version: tool_version,
        diff_tool_config_sha256: tool_config_sha256,
        elapsed_millis: started.elapsed().as_millis(),
    })
}

fn score(cases: &[CaseResult]) -> Summary {
    let expected_equal = cases.iter().filter(|c| c.expected_status == ExpectedStatus::Equal).count();
    let expected_different = cases.iter().filter(|c| c.expected_status == ExpectedStatus::Different).count();
    let expected_unresolved = cases.iter().filter(|c| c.expected_status == ExpectedStatus::Unresolved).count();
    let exact_matches = cases.iter().filter(|c| c.exact_match).count();
    Summary {
        case_count: cases.len(),
        exact_matches,
        outcome_mismatches: cases.len() - exact_matches,
        expected_equal,
        expected_different,
        expected_unresolved,
        false_acceptances: cases.iter().filter(|c| c.false_acceptance).count(),
        false_rejections: cases.iter().filter(|c| c.false_rejection).count(),
        unsafe_certainty: cases.iter().filter(|c| c.unsafe_certainty).count(),
        unresolved_instead_of_decision: cases.iter().filter(|c| c.unresolved_instead_of_decision).count(),
        exact_match_rate: if cases.is_empty() { 0.0 } else { exact_matches as f64 / cases.len() as f64 },
    }
}

fn parse_args() -> Result<Option<Args>, String> {
    let mut raw = env::args().skip(1);
    let first = raw.next();
    if matches!(first.as_deref(), Some("--help" | "-h")) {
        return Ok(None);
    }
    let mut args = first.into_iter().chain(raw);
    let mut singleton = std::collections::BTreeMap::<String, String>::new();
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--diff-bin" | "--manifest" | "--run-id" | "--reported-at"
            | "--diff-tool-source-commit-sha" | "--evaluator-source-commit-sha"
            | "--diff-bin-sha256" | "--output" => {
                let value = args.next().ok_or_else(|| format!("MISSING_VALUE:{flag}"))?;
                if singleton.insert(flag.clone(), value).is_some() {
                    return Err(format!("DUPLICATE_ARGUMENT:{flag}"));
                }
            }
            _ => return Err(format!("UNKNOWN_ARGUMENT:{flag}")),
        }
    }
    let take = |key: &str| -> Result<String, String> {
        singleton.get(key).cloned().ok_or_else(|| format!("MISSING_REQUIRED_ARGUMENT:{key}"))
    };
    let diff_bin = PathBuf::from(take("--diff-bin")?);
    let diff_bin_sha256 = take("--diff-bin-sha256")?;
    if !valid_hex(&diff_bin_sha256, 64) || sha256_hex(&fs::read(&diff_bin).map_err(|e| format!("CANNOT_READ_DIFF_BINARY:{e}"))?) != diff_bin_sha256 {
        return Err("DIFF_BINARY_SHA256_MISMATCH".into());
    }
    let args = Args {
        diff_bin,
        manifest: PathBuf::from(take("--manifest")?),
        run_id: take("--run-id")?,
        reported_at: take("--reported-at")?,
        diff_tool_source_commit_sha: take("--diff-tool-source-commit-sha")?,
        evaluator_source_commit_sha: take("--evaluator-source-commit-sha")?,
        diff_bin_sha256,
        output: singleton.get("--output").map(PathBuf::from),
    };
    if !valid_run_id(&args.run_id) || !valid_timestamp(&args.reported_at)
        || !valid_hex(&args.diff_tool_source_commit_sha, 40)
        || !valid_hex(&args.evaluator_source_commit_sha, 40) {
        return Err("RUN_METADATA_INVALID".into());
    }
    Ok(Some(args))
}

fn usage() -> &'static str {
    "Usage: built-interop-mutation-campaign --diff-bin PATH --diff-bin-sha256 HEX64 --manifest PATH --run-id ID --reported-at RFC3339 --diff-tool-source-commit-sha HEX40 --evaluator-source-commit-sha HEX40 [--output PATH]\nExit: 0=all oracle outcomes matched, 1=one or more status mismatches, 64=invalid manifest/tool/input."
}

fn run() -> Result<i32, String> {
    let Some(args) = parse_args()? else {
        println!("{}", usage());
        return Ok(0);
    };
    let manifest_bytes = fs::read(&args.manifest).map_err(|e| format!("CANNOT_READ_MANIFEST:{e}"))?;
    let manifest: CampaignManifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|e| format!("MANIFEST_JSON_INVALID:{e}"))?;
    let manifest_root = args.manifest.parent().unwrap_or_else(|| Path::new("."));
    let fixture_path = manifest_root.join(&manifest.base_fixture.path);
    let fixture_bytes = fs::read(&fixture_path).map_err(|e| format!("CANNOT_READ_BASE_FIXTURE:{e}"))?;
    validate_manifest(&manifest, &fixture_bytes)?;

    let unix_nanos = SystemTime::now().duration_since(UNIX_EPOCH)
        .map_err(|_| "SYSTEM_CLOCK_BEFORE_UNIX_EPOCH".to_string())?.as_nanos();
    let temp_dir = env::temp_dir().join(format!("built-interop-009-{}-{unix_nanos}", process::id()));
    fs::create_dir(&temp_dir).map_err(|e| format!("TEMP_DIR_CREATE_FAILED:{e}"))?;
    let cleanup = TempCleanup(temp_dir.clone());

    let mut results = Vec::with_capacity(manifest.cases.len());
    let mut diff_version: Option<String> = None;
    let mut diff_config: Option<String> = None;
    for (index, oracle) in manifest.cases.iter().enumerate() {
        let target_bytes = apply_mutation(&fixture_bytes, oracle.mutation)?;
        let target_path = temp_dir.join(format!("case-{index:02}.ifc"));
        fs::write(&target_path, &target_bytes).map_err(|e| format!("TARGET_WRITE_FAILED:{e}"))?;
        let result = run_one(
            &args.diff_bin,
            &fixture_path,
            &target_path,
            &fixture_bytes,
            &target_bytes,
            oracle,
            &manifest.oracle_class,
        )?;
        // The producer's version/configuration must stay identical across the
        // entire campaign; mixed reports from different tool profiles are invalid.
        if diff_version.is_none() {
            diff_version = Some(result.diff_tool_version.clone());
        }
        if diff_version.as_deref() != Some(result.diff_tool_version.as_str()) {
            return Err("DIFF_TOOL_VERSION_CHANGED_WITHIN_CAMPAIGN".into());
        }
        if diff_config.is_none() {
            diff_config = Some(result.diff_tool_config_sha256.clone());
        }
        if diff_config.as_deref() != Some(result.diff_tool_config_sha256.as_str()) {
            return Err("DIFF_TOOL_CONFIG_CHANGED_WITHIN_CAMPAIGN".into());
        }
        results.push(result);
    }

    let summary = score(&results);
    let all_matched = summary.outcome_mismatches == 0;
    let manifest_sha256 = sha256_hex(&manifest_bytes);
    let report = CampaignReport {
        schema_version: CAMPAIGN_SCHEMA_VERSION.into(),
        benchmark_id: CAMPAIGN_ID.into(),
        oracle_class: manifest.oracle_class,
        run_id: args.run_id,
        reported_at: args.reported_at,
        campaign_status: if all_matched { "PASS" } else { "FAIL" }.into(),
        base_fixture_sha256: sha256_hex(&fixture_bytes),
        base_fixture_byte_length: fixture_bytes.len(),
        manifest_sha256,
        policy_sha256: sha256_hex(POLICY.as_bytes()),
        diff_tool: ToolIdentity {
            name: "built-interop-semantic-diff".into(),
            version: diff_version.ok_or("DIFF_TOOL_VERSION_NOT_OBSERVED")?,
            source_commit_sha: args.diff_tool_source_commit_sha,
            binary_sha256: args.diff_bin_sha256,
            config_sha256: diff_config.ok_or("DIFF_TOOL_CONFIG_NOT_OBSERVED")?,
        },
        evaluator_source_commit_sha: args.evaluator_source_commit_sha,
        cases: results,
        summary,
        limitations: vec![
            "This is a synthetic metamorphic mutation campaign. It does not estimate field accuracy on commercial CAD exports.".into(),
            "Oracle labels are closed transformation expectations, not labels inferred from the diff tool output.".into(),
            "Equal means only no difference was found by the pinned IFC4 schema-directed profile; it is not geometry, IDS, code or engineering qualification.".into(),
            "The campaign verifies byte digests reported by the tool but does not prove the tool implementation sound or authenticate external attestations.".into(),
        ],
    };
    let json = serde_json::to_vec_pretty(&report).map_err(|e| format!("REPORT_SERIALIZATION_FAILED:{e}"))?;
    if let Some(path) = &args.output {
        fs::write(path, &json).map_err(|e| format!("REPORT_WRITE_FAILED:{e}"))?;
    }
    println!("{}", String::from_utf8_lossy(&json));
    drop(cleanup);
    Ok(if all_matched { 0 } else { 1 })
}

struct TempCleanup(PathBuf);
impl Drop for TempCleanup {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn main() {
    match run() {
        Ok(code) => process::exit(code),
        Err(error) => {
            eprintln!("{error}");
            process::exit(64);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &[u8] = include_bytes!("../fixtures/two-walls.ifc");

    #[test]
    fn fixture_matches_pinned_bytes() {
        assert_eq!(FIXTURE.len(), BASE_FIXTURE_BYTES);
        assert_eq!(sha256_hex(FIXTURE), BASE_FIXTURE_SHA256);
    }

    #[test]
    fn mutation_matrix_has_unique_closed_labels() {
        let expected = [
            ExpectedStatus::Equal,
            ExpectedStatus::Equal,
            ExpectedStatus::Equal,
            ExpectedStatus::Equal,
            ExpectedStatus::Different,
            ExpectedStatus::Different,
            ExpectedStatus::Different,
            ExpectedStatus::Unresolved,
            ExpectedStatus::Unresolved,
            ExpectedStatus::Unresolved,
            ExpectedStatus::Unresolved,
        ];
        assert_eq!(Mutation::ORDERED.len(), expected.len());
        for (mutation, status) in Mutation::ORDERED.into_iter().zip(expected) {
            assert_eq!(mutation.expected(), status);
        }
    }

    #[test]
    fn seeded_mutations_are_real_byte_changes_except_identity() {
        for mutation in Mutation::ORDERED {
            let target = apply_mutation(FIXTURE, mutation).unwrap();
            assert_eq!(target == FIXTURE, mutation == Mutation::Identical, "{mutation:?}");
        }
    }

    #[test]
    fn score_distinguishes_false_acceptance_rejection_and_unsafe_certainty() {
        let case = |expected_status, observed_status| CaseResult {
            case_id: "unit".into(),
            mutation: Mutation::WallNameChange,
            oracle_class: "synthetic-metamorphic-v1".into(),
            oracle_basis: "unit test".into(),
            expected_status,
            observed_status,
            exact_match: expected_status == observed_status,
            false_acceptance: expected_status == ExpectedStatus::Different && observed_status == ExpectedStatus::Equal,
            false_rejection: expected_status == ExpectedStatus::Equal && observed_status == ExpectedStatus::Different,
            unsafe_certainty: expected_status == ExpectedStatus::Unresolved && observed_status != ExpectedStatus::Unresolved,
            unresolved_instead_of_decision: expected_status != ExpectedStatus::Unresolved && observed_status == ExpectedStatus::Unresolved,
            source_sha256: "a".repeat(64),
            target_sha256: "b".repeat(64),
            target_byte_length: 1,
            semantic_result_sha256: "c".repeat(64),
            decision_reason: "unit".into(),
            diff_tool_version: "unit".into(),
            diff_tool_config_sha256: "d".repeat(64),
            elapsed_millis: 0,
        };
        let cases = vec![
            case(ExpectedStatus::Different, ExpectedStatus::Equal),
            case(ExpectedStatus::Equal, ExpectedStatus::Different),
            case(ExpectedStatus::Unresolved, ExpectedStatus::Different),
        ];
        let summary = score(&cases);
        assert_eq!(summary.false_acceptances, 1);
        assert_eq!(summary.false_rejections, 1);
        assert_eq!(summary.unsafe_certainty, 1);
        assert_eq!(summary.outcome_mismatches, 3);
    }

    #[test]
    fn timestamp_and_ids_are_fail_closed() {
        assert!(valid_timestamp("2026-10-19T17:00:00Z"));
        assert!(valid_timestamp("2024-02-29T17:00:00.123+02:00"));
        assert!(!valid_timestamp("2023-02-29T17:00:00Z"));
        assert!(!valid_timestamp("2026-99-99T25:61:99Z"));
        assert!(valid_run_id("ci-3926:attempt_1"));
        assert!(!valid_run_id("invalid/run"));
        assert!(!valid_hex("z".repeat(64).as_str(), 64));
        assert!(!valid_timestamp("not-a-timestamp"));
    }

    #[test]
    fn oracle_status_is_not_accepted_from_external_unverified_corpus() {
        let raw = serde_json::json!({
            "schema_version":"1.0.0",
            "benchmark_id":"BUILT-INTEROP-009",
            "oracle_class":"external-unverified",
            "base_fixture":{"path":"fixtures/two-walls.ifc","sha256":BASE_FIXTURE_SHA256,"byte_length":424},
            "cases":[]
        });
        let manifest: CampaignManifest = serde_json::from_value(raw).unwrap();
        assert_eq!(
            validate_manifest(&manifest, FIXTURE).unwrap_err(),
            "CAMPAIGN_MANIFEST_PROFILE_MISMATCH"
        );
    }
}
