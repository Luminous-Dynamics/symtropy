//! Dependency-closed evidence provenance reference.
//!
//! Consumes a catalogue whose admission statuses are assumed to originate from
//! a separate cryptographic receipt verifier. This tool checks transitive role,
//! digest, scope, time and evidence-tier closure; it does not hash raw files or
//! authenticate signatures.
//!
//! Build and test:
//!   rustc --edition=2021 -D warnings --test tools/manufacturing-assurance/evidence_closure.rs -o /tmp/mfg-evidence-closure-tests
//!   /tmp/mfg-evidence-closure-tests
//!
//! Build and run:
//!   rustc --edition=2021 -D warnings tools/manufacturing-assurance/evidence_closure.rs -o /tmp/mfg-evidence-closure
//!   /tmp/mfg-evidence-closure tools/manufacturing-assurance/fixtures/valid-evidence-closure.tsv

use std::collections::{BTreeMap, BTreeSet, VecDeque};
#[cfg(not(test))]
use std::env;
#[cfg(not(test))]
use std::fs;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Case {
    id: String,
    root_artifact: String,
    scope: String,
    as_of: String,
    minimum_tier: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Artifact {
    id: String,
    role: String,
    digest: String,
    scope: String,
    valid_from: String,
    valid_until: String,
    tier: u8,
    admission_status: String,
    receipt_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Dependency {
    consumer_id: String,
    material_id: String,
    expected_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RolePolicy {
    consumer_role: String,
    material_role: String,
    minimum_tier: u8,
    minimum_count: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Manifest {
    case_record: Option<Case>,
    artifacts: BTreeMap<String, Artifact>,
    dependencies: Vec<Dependency>,
    role_policies: Vec<RolePolicy>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ArtifactResult {
    artifact_id: String,
    status: String,
    reasons: Vec<String>,
    witness_path: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ClosureReport {
    status: String,
    root_artifact: String,
    reachable_count: usize,
    dependency_count: usize,
    results: Vec<ArtifactResult>,
    global_reasons: Vec<String>,
}

fn nonempty(value: &str, context: &str) -> Result<String, String> {
    if value.trim().is_empty() || value == "-" {
        Err(format!("{context}: missing value"))
    } else {
        Ok(value.to_string())
    }
}

fn parse_tier(value: &str, context: &str) -> Result<u8, String> {
    match value {
        "T0" => Ok(0),
        "T1" => Ok(1),
        "T2" => Ok(2),
        "T3" => Ok(3),
        "T4" => Ok(4),
        _ => Err(format!("{context}: expected T0 through T4")),
    }
}

fn tier_name(value: u8) -> String {
    format!("T{value}")
}

fn parse_usize(value: &str, context: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|_| format!("{context}: expected a non-negative integer"))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_utc(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
        || !bytes.iter().enumerate().all(|(index, byte)| {
            matches!(index, 4 | 7 | 10 | 13 | 16 | 19) || byte.is_ascii_digit()
        })
    {
        return false;
    }
    let parse = |range: std::ops::Range<usize>| -> Option<u32> {
        std::str::from_utf8(&bytes[range]).ok()?.parse().ok()
    };
    let year = match parse(0..4) { Some(value) => value, None => return false };
    let month = match parse(5..7) { Some(value) => value, None => return false };
    let day = match parse(8..10) { Some(value) => value, None => return false };
    let hour = match parse(11..13) { Some(value) => value, None => return false };
    let minute = match parse(14..16) { Some(value) => value, None => return false };
    let second = match parse(17..19) { Some(value) => value, None => return false };
    if year == 0 || hour > 23 || minute > 59 || second > 59 {
        return false;
    }
    let leap_year = year % 400 == 0 || (year % 4 == 0 && year % 100 != 0);
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap_year => 29,
        2 => 28,
        _ => return false,
    };
    day >= 1 && day <= days_in_month
}

fn valid_interval(start: &str, end: &str, context: &str) -> Result<(), String> {
    if !valid_utc(start) || !valid_utc(end) || start >= end {
        return Err(format!("{context}: invalid UTC interval"));
    }
    Ok(())
}

fn active_at(start: &str, end: &str, time: &str) -> bool {
    start <= time && time < end
}

fn scope_compatible(source_scope: &str, consumer_scope: &str) -> bool {
    if consumer_scope == "GLOBAL" {
        source_scope == "GLOBAL"
    } else {
        source_scope == "GLOBAL" || source_scope == consumer_scope
    }
}

fn parse_manifest(input: &str) -> Result<Manifest, String> {
    let mut manifest = Manifest::default();

    for (line_index, original) in input.lines().enumerate() {
        let line_number = line_index + 1;
        let line = original.trim_end_matches('\r');
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('|').collect();

        match fields.first().copied().unwrap_or("") {
            "CASE" => {
                if fields.len() != 6 || manifest.case_record.is_some() {
                    return Err(format!("line {line_number}: CASE requires 5 fields and must occur once"));
                }
                let case = Case {
                    id: nonempty(fields[1], "CASE id")?,
                    root_artifact: nonempty(fields[2], "CASE root_artifact")?,
                    scope: nonempty(fields[3], "CASE scope")?,
                    as_of: nonempty(fields[4], "CASE as_of")?,
                    minimum_tier: parse_tier(fields[5], "CASE minimum_tier")?,
                };
                if !valid_utc(&case.as_of) {
                    return Err(format!("line {line_number}: CASE as_of is not canonical UTC"));
                }
                manifest.case_record = Some(case);
            }
            "ARTIFACT" => {
                if fields.len() != 10 {
                    return Err(format!("line {line_number}: ARTIFACT requires 9 fields"));
                }
                let artifact = Artifact {
                    id: nonempty(fields[1], "ARTIFACT id")?,
                    role: nonempty(fields[2], "ARTIFACT role")?,
                    digest: nonempty(fields[3], "ARTIFACT digest")?,
                    scope: nonempty(fields[4], "ARTIFACT scope")?,
                    valid_from: nonempty(fields[5], "ARTIFACT valid_from")?,
                    valid_until: nonempty(fields[6], "ARTIFACT valid_until")?,
                    tier: parse_tier(fields[7], "ARTIFACT tier")?,
                    admission_status: nonempty(fields[8], "ARTIFACT admission_status")?,
                    receipt_digest: nonempty(fields[9], "ARTIFACT receipt_digest")?,
                };
                valid_interval(&artifact.valid_from, &artifact.valid_until, "ARTIFACT validity")?;
                if !is_sha256(&artifact.digest) || !is_sha256(&artifact.receipt_digest) {
                    return Err(format!("line {line_number}: ARTIFACT digest is not lowercase SHA-256"));
                }
                if !matches!(
                    artifact.admission_status.as_str(),
                    "ACCEPTED" | "REJECTED" | "PENDING" | "REVOKED" | "CONTESTED"
                ) {
                    return Err(format!("line {line_number}: unsupported admission status"));
                }
                let id = artifact.id.clone();
                if manifest.artifacts.insert(id.clone(), artifact).is_some() {
                    return Err(format!("line {line_number}: duplicate artifact id {id}"));
                }
            }
            "DEP" => {
                if fields.len() != 4 {
                    return Err(format!("line {line_number}: DEP requires 3 fields"));
                }
                let dependency = Dependency {
                    consumer_id: nonempty(fields[1], "DEP consumer_id")?,
                    material_id: nonempty(fields[2], "DEP material_id")?,
                    expected_digest: nonempty(fields[3], "DEP expected_digest")?,
                };
                if !is_sha256(&dependency.expected_digest) {
                    return Err(format!("line {line_number}: DEP expected digest is not lowercase SHA-256"));
                }
                manifest.dependencies.push(dependency);
            }
            "ROLE_POLICY" => {
                if fields.len() != 5 {
                    return Err(format!("line {line_number}: ROLE_POLICY requires 4 fields"));
                }
                manifest.role_policies.push(RolePolicy {
                    consumer_role: nonempty(fields[1], "ROLE_POLICY consumer_role")?,
                    material_role: nonempty(fields[2], "ROLE_POLICY material_role")?,
                    minimum_tier: parse_tier(fields[3], "ROLE_POLICY minimum_tier")?,
                    minimum_count: parse_usize(fields[4], "ROLE_POLICY minimum_count")?,
                });
            }
            other => return Err(format!("line {line_number}: unknown record type {other:?}")),
        }
    }

    validate_manifest(&manifest)?;
    Ok(manifest)
}

fn validate_manifest(manifest: &Manifest) -> Result<(), String> {
    let case = manifest
        .case_record
        .as_ref()
        .ok_or_else(|| "manifest requires exactly one CASE".to_string())?;
    if manifest.artifacts.is_empty() {
        return Err("manifest requires at least one ARTIFACT".to_string());
    }
    if manifest.role_policies.is_empty() {
        return Err("manifest requires at least one ROLE_POLICY".to_string());
    }
    if !manifest.artifacts.contains_key(&case.root_artifact) {
        return Err(format!("CASE references unknown root artifact {}", case.root_artifact));
    }

    let mut policy_keys = BTreeSet::new();
    for policy in &manifest.role_policies {
        let key = format!("{}|{}", policy.consumer_role, policy.material_role);
        if !policy_keys.insert(key) {
            return Err("duplicate ROLE_POLICY".to_string());
        }
    }

    let mut dependency_keys = BTreeSet::new();
    for dependency in &manifest.dependencies {
        if !manifest.artifacts.contains_key(&dependency.consumer_id) {
            return Err(format!("DEP references unknown consumer {}", dependency.consumer_id));
        }
        if !manifest.artifacts.contains_key(&dependency.material_id) {
            return Err(format!("DEP references unknown material {}", dependency.material_id));
        }
        if dependency.consumer_id == dependency.material_id {
            return Err(format!("self-dependency at {}", dependency.consumer_id));
        }
        let key = format!("{}|{}", dependency.consumer_id, dependency.material_id);
        if !dependency_keys.insert(key) {
            return Err(format!(
                "duplicate dependency {} -> {}",
                dependency.consumer_id, dependency.material_id
            ));
        }
    }

    if manifest.artifacts.values().any(|artifact| artifact.id == case.root_artifact) {
        Ok(())
    } else {
        Err("root artifact identity mismatch".to_string())
    }
}

fn dependency_adjacency(manifest: &Manifest) -> BTreeMap<String, Vec<String>> {
    let mut adjacency: BTreeMap<String, Vec<String>> = manifest
        .artifacts
        .keys()
        .map(|id| (id.clone(), Vec::new()))
        .collect();
    for dependency in &manifest.dependencies {
        adjacency
            .get_mut(&dependency.consumer_id)
            .expect("validated dependency consumer exists")
            .push(dependency.material_id.clone());
    }
    for children in adjacency.values_mut() {
        children.sort();
    }
    adjacency
}

fn has_cycle(adjacency: &BTreeMap<String, Vec<String>>) -> bool {
    let mut indegree: BTreeMap<String, usize> =
        adjacency.keys().map(|id| (id.clone(), 0)).collect();
    for children in adjacency.values() {
        for child in children {
            *indegree.get_mut(child).expect("validated graph child exists") += 1;
        }
    }

    let mut ready: BTreeSet<String> = indegree
        .iter()
        .filter_map(|(id, degree)| if *degree == 0 { Some(id.clone()) } else { None })
        .collect();
    let mut visited = 0usize;
    while let Some(id) = ready.pop_first() {
        visited += 1;
        if let Some(children) = adjacency.get(&id) {
            for child in children {
                let degree = indegree.get_mut(child).expect("child exists in indegree");
                *degree -= 1;
                if *degree == 0 {
                    ready.insert(child.clone());
                }
            }
        }
    }
    visited != adjacency.len()
}

fn reachable_parents(
    root: &str,
    adjacency: &BTreeMap<String, Vec<String>>,
) -> (BTreeSet<String>, BTreeMap<String, String>) {
    let mut reachable = BTreeSet::new();
    let mut parent = BTreeMap::new();
    let mut queue = VecDeque::new();
    reachable.insert(root.to_string());
    queue.push_back(root.to_string());

    while let Some(id) = queue.pop_front() {
        if let Some(children) = adjacency.get(&id) {
            for child in children {
                if reachable.insert(child.clone()) {
                    parent.insert(child.clone(), id.clone());
                    queue.push_back(child.clone());
                }
            }
        }
    }
    (reachable, parent)
}

fn witness_path(id: &str, parent: &BTreeMap<String, String>) -> String {
    let mut path = vec![id.to_string()];
    let mut current = id;
    while let Some(previous) = parent.get(current) {
        path.push(previous.clone());
        current = previous;
    }
    path.reverse();
    path.join("->")
}

fn evaluate(manifest: &Manifest) -> ClosureReport {
    let case = manifest.case_record.as_ref().expect("validated manifest has CASE");
    let adjacency = dependency_adjacency(manifest);
    let cycle_detected = has_cycle(&adjacency);
    let (reachable, parent) = reachable_parents(&case.root_artifact, &adjacency);
    let mut global_reasons = Vec::new();
    let mut results = Vec::new();

    if cycle_detected {
        global_reasons.push("DEPENDENCY_CYCLE".to_string());
    }

    let mut dependencies_by_consumer: BTreeMap<String, Vec<&Dependency>> = BTreeMap::new();
    for dependency in &manifest.dependencies {
        dependencies_by_consumer
            .entry(dependency.consumer_id.clone())
            .or_default()
            .push(dependency);
    }
    for dependencies in dependencies_by_consumer.values_mut() {
        dependencies.sort_by(|left, right| left.material_id.cmp(&right.material_id));
    }

    for artifact_id in &reachable {
        let artifact = manifest.artifacts.get(artifact_id).expect("reachable artifact exists");
        let path = witness_path(artifact_id, &parent);
        let mut reasons = Vec::new();

        if !active_at(&artifact.valid_from, &artifact.valid_until, &case.as_of) {
            reasons.push(format!("ARTIFACT_NOT_CURRENT:{artifact_id}"));
        }
        if !scope_compatible(&artifact.scope, &case.scope) {
            reasons.push(format!("ARTIFACT_SCOPE_MISMATCH:{artifact_id}"));
        }
        if artifact.admission_status != "ACCEPTED" {
            reasons.push(format!(
                "ARTIFACT_NOT_ACCEPTED:{}:{}",
                artifact_id, artifact.admission_status
            ));
        }
        if artifact_id == &case.root_artifact && artifact.tier < case.minimum_tier {
            reasons.push(format!(
                "ROOT_TIER_TOO_LOW:{}:required={}:actual={}",
                artifact_id,
                tier_name(case.minimum_tier),
                tier_name(artifact.tier)
            ));
        }

        let dependencies: Vec<&Dependency> = dependencies_by_consumer
            .get(artifact_id)
            .cloned()
            .unwrap_or_default();

        for policy in manifest
            .role_policies
            .iter()
            .filter(|policy| policy.consumer_role == artifact.role)
        {
            let count = dependencies
                .iter()
                .filter(|dependency| {
                    manifest
                        .artifacts
                        .get(&dependency.material_id)
                        .map(|material| material.role == policy.material_role)
                        .unwrap_or(false)
                })
                .count();
            if count < policy.minimum_count {
                reasons.push(format!(
                    "MISSING_REQUIRED_MATERIAL:{}:{}:required={}:actual={}",
                    artifact_id, policy.material_role, policy.minimum_count, count
                ));
            }
        }

        for dependency in dependencies {
            let material = manifest
                .artifacts
                .get(&dependency.material_id)
                .expect("validated dependency material exists");
            let policy = manifest.role_policies.iter().find(|policy| {
                policy.consumer_role == artifact.role && policy.material_role == material.role
            });

            if material.digest != dependency.expected_digest {
                reasons.push(format!(
                    "MATERIAL_DIGEST_MISMATCH:{}:{}",
                    artifact_id, material.id
                ));
            }
            match policy {
                None => reasons.push(format!(
                    "UNDECLARED_ROLE_DEPENDENCY:{}:{}",
                    artifact.role, material.role
                )),
                Some(policy) => {
                    if material.tier < policy.minimum_tier {
                        reasons.push(format!(
                            "MATERIAL_TIER_TOO_LOW:{}:{}:required={}:actual={}",
                            artifact_id,
                            material.id,
                            tier_name(policy.minimum_tier),
                            tier_name(material.tier)
                        ));
                    }
                }
            }
            if !scope_compatible(&material.scope, &artifact.scope) {
                reasons.push(format!(
                    "DEPENDENCY_SCOPE_MISMATCH:{}:{}",
                    artifact_id, material.id
                ));
            }
        }

        reasons.sort();
        reasons.dedup();
        let status = if reasons.is_empty() {
            "CLOSED"
        } else {
            "BLOCKED"
        };
        results.push(ArtifactResult {
            artifact_id: artifact_id.clone(),
            status: status.to_string(),
            reasons,
            witness_path: path,
        });
    }

    for artifact_id in manifest.artifacts.keys() {
        if !reachable.contains(artifact_id) {
            global_reasons.push(format!("UNREACHABLE_ARTIFACT:{artifact_id}"));
        }
    }

    for result in &results {
        for reason in &result.reasons {
            global_reasons.push(reason.clone());
        }
    }
    global_reasons.sort();
    global_reasons.dedup();
    let status = if global_reasons.is_empty() {
        "CLOSED"
    } else {
        "BLOCKED"
    };

    ClosureReport {
        status: status.to_string(),
        root_artifact: case.root_artifact.clone(),
        reachable_count: reachable.len(),
        dependency_count: manifest.dependencies.len(),
        results,
        global_reasons,
    }
}

fn run_report(input: &str) -> Result<String, String> {
    let manifest = parse_manifest(input)?;
    let report = evaluate(&manifest);
    let case = manifest.case_record.as_ref().expect("validated manifest has CASE");

    let mut output = format!(
        "MFG_EVIDENCE_CLOSURE version=1 case={} root={} scope={} as_of={} minimum_tier={} reachable={} dependencies={}\n",
        case.id,
        case.root_artifact,
        case.scope,
        case.as_of,
        tier_name(case.minimum_tier),
        report.reachable_count,
        report.dependency_count
    );
    output.push_str(&format!(
        "RESULT status={} reasons={}\n",
        report.status,
        if report.global_reasons.is_empty() {
            "-".to_string()
        } else {
            report.global_reasons.join(",")
        }
    ));
    for result in &report.results {
        let artifact = manifest
            .artifacts
            .get(&result.artifact_id)
            .expect("report artifact exists");
        output.push_str(&format!(
            "ARTIFACT id={} role={} status={} tier={} scope={} parent_path={} reasons={}\n",
            artifact.id,
            artifact.role,
            result.status,
            tier_name(artifact.tier),
            artifact.scope,
            result.witness_path,
            if result.reasons.is_empty() {
                "-".to_string()
            } else {
                result.reasons.join(",")
            }
        ));
    }
    output.push_str(
        "CLAIM_CEILING dependency_closure_only raw_artifact_bytes_hashed=false signatures_verified=false receipt_authenticity_assumed_from_upstream_verifier=false physical_truth_proven=false semantic_validity_proven=false\n",
    );
    Ok(output)
}

#[cfg(not(test))]
fn main() {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| "tools/manufacturing-assurance/fixtures/valid-evidence-closure.tsv".to_string());
    match fs::read_to_string(&path)
        .map_err(|error| format!("unable to read {path}: {error}"))
        .and_then(|input| run_report(&input))
    {
        Ok(report) => print!("{report}"),
        Err(error) => {
            eprintln!("mfg-evidence-closure: {error}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HASH_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const HASH_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const HASH_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const HASH_D: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
    const HASH_E: &str = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
    const HASH_F: &str = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

    fn valid_manifest() -> String {
        format!(
            "CASE|case-1|measurement|LINE-1|2026-10-31T12:00:00Z|T2\n\
             ARTIFACT|measurement|PROCESS_MEASUREMENT|{}|LINE-1|2026-10-01T00:00:00Z|2026-11-01T00:00:00Z|T2|ACCEPTED|{}\n\
             ARTIFACT|raw-data|RAW_MEASUREMENT|{}|LINE-1|2026-10-01T00:00:00Z|2026-11-01T00:00:00Z|T2|ACCEPTED|{}\n\
             ARTIFACT|calibration|CALIBRATION_RECORD|{}|LINE-1|2026-10-01T00:00:00Z|2026-11-01T00:00:00Z|T2|ACCEPTED|{}\n\
             ARTIFACT|instrument|INSTRUMENT_IDENTITY|{}|LINE-1|2026-10-01T00:00:00Z|2026-11-01T00:00:00Z|T2|ACCEPTED|{}\n\
             ARTIFACT|cal-cert|CALIBRATION_CERTIFICATE|{}|LINE-1|2026-10-01T00:00:00Z|2026-11-01T00:00:00Z|T2|ACCEPTED|{}\n\
             ARTIFACT|reference-standard|REFERENCE_STANDARD|{}|LINE-1|2026-10-01T00:00:00Z|2026-11-01T00:00:00Z|T2|ACCEPTED|{}\n\
             DEP|measurement|raw-data|{}\n\
             DEP|measurement|calibration|{}\n\
             DEP|measurement|instrument|{}\n\
             DEP|calibration|cal-cert|{}\n\
             DEP|calibration|reference-standard|{}\n\
             ROLE_POLICY|PROCESS_MEASUREMENT|RAW_MEASUREMENT|T2|1\n\
             ROLE_POLICY|PROCESS_MEASUREMENT|CALIBRATION_RECORD|T2|1\n\
             ROLE_POLICY|PROCESS_MEASUREMENT|INSTRUMENT_IDENTITY|T2|1\n\
             ROLE_POLICY|CALIBRATION_RECORD|CALIBRATION_CERTIFICATE|T2|1\n\
             ROLE_POLICY|CALIBRATION_RECORD|REFERENCE_STANDARD|T2|1\n",
            HASH_A, HASH_A, HASH_B, HASH_B, HASH_C, HASH_C, HASH_D, HASH_D, HASH_E, HASH_E, HASH_F, HASH_F,
            HASH_B, HASH_C, HASH_D, HASH_E, HASH_F
        )
    }

    fn mutate(input: String, old: &str, new: &str) -> String {
        assert!(input.contains(old), "missing mutation target: {old}");
        input.replacen(old, new, 1)
    }

    #[test]
    fn valid_measurement_closes_transitively_over_calibration_materials() {
        let report = run_report(&valid_manifest()).unwrap();
        assert!(report.contains("RESULT status=CLOSED"));
        assert!(report.contains("reachable=6"));
        assert!(report.contains("CALIBRATION_CERTIFICATE"));
        assert!(report.contains("REFERENCE_STANDARD"));
    }

    #[test]
    fn missing_calibration_edge_blocks_closure() {
        let input = mutate(valid_manifest(), &format!("DEP|measurement|calibration|{HASH_C}\n"), "");
        let report = run_report(&input).unwrap();
        assert!(report.contains("MISSING_REQUIRED_MATERIAL:measurement:CALIBRATION_RECORD"));
        assert!(report.contains("RESULT status=BLOCKED"));
    }

    #[test]
    fn missing_transitive_calibration_certificate_blocks_closure() {
        let input = mutate(valid_manifest(), &format!("DEP|calibration|cal-cert|{HASH_E}\n"), "");
        let report = run_report(&input).unwrap();
        assert!(report.contains("MISSING_REQUIRED_MATERIAL:calibration:CALIBRATION_CERTIFICATE"));
    }

    #[test]
    fn material_digest_mismatch_blocks_closure() {
        let input = mutate(
            valid_manifest(),
            &format!("DEP|measurement|raw-data|{HASH_B}"),
            &format!("DEP|measurement|raw-data|{HASH_D}"),
        );
        let report = run_report(&input).unwrap();
        assert!(report.contains("MATERIAL_DIGEST_MISMATCH:measurement:raw-data"));
    }

    #[test]
    fn stale_calibration_blocks_closure() {
        let input = mutate(
            valid_manifest(),
            &format!(
                "ARTIFACT|calibration|CALIBRATION_RECORD|{HASH_C}|LINE-1|2026-10-01T00:00:00Z|2026-11-01T00:00:00Z|T2|ACCEPTED|{HASH_C}"
            ),
            &format!(
                "ARTIFACT|calibration|CALIBRATION_RECORD|{HASH_C}|LINE-1|2026-10-01T00:00:00Z|2026-10-30T00:00:00Z|T2|ACCEPTED|{HASH_C}"
            ),
        );
        let report = run_report(&input).unwrap();
        assert!(report.contains("ARTIFACT_NOT_CURRENT:calibration"));
    }

    #[test]
    fn non_accepted_receipt_blocks_closure() {
        let input = mutate(
            valid_manifest(),
            &format!(
                "ARTIFACT|calibration|CALIBRATION_RECORD|{HASH_C}|LINE-1|2026-10-01T00:00:00Z|2026-11-01T00:00:00Z|T2|ACCEPTED|{HASH_C}"
            ),
            &format!(
                "ARTIFACT|calibration|CALIBRATION_RECORD|{HASH_C}|LINE-1|2026-10-01T00:00:00Z|2026-11-01T00:00:00Z|T2|CONTESTED|{HASH_C}"
            ),
        );
        let report = run_report(&input).unwrap();
        assert!(report.contains("ARTIFACT_NOT_ACCEPTED:calibration:CONTESTED"));
    }

    #[test]
    fn cross_scope_material_cannot_support_consumer() {
        let input = mutate(
            valid_manifest(),
            &format!(
                "ARTIFACT|instrument|INSTRUMENT_IDENTITY|{HASH_D}|LINE-1|2026-10-01T00:00:00Z|2026-11-01T00:00:00Z|T2|ACCEPTED|{HASH_D}"
            ),
            &format!(
                "ARTIFACT|instrument|INSTRUMENT_IDENTITY|{HASH_D}|LINE-2|2026-10-01T00:00:00Z|2026-11-01T00:00:00Z|T2|ACCEPTED|{HASH_D}"
            ),
        );
        let report = run_report(&input).unwrap();
        assert!(report.contains("ARTIFACT_SCOPE_MISMATCH:instrument"));
        assert!(report.contains("DEPENDENCY_SCOPE_MISMATCH:measurement:instrument"));
    }

    #[test]
    fn global_material_may_support_local_consumer() {
        let input = mutate(
            valid_manifest(),
            &format!(
                "ARTIFACT|reference-standard|REFERENCE_STANDARD|{HASH_F}|LINE-1|2026-10-01T00:00:00Z|2026-11-01T00:00:00Z|T2|ACCEPTED|{HASH_F}"
            ),
            &format!(
                "ARTIFACT|reference-standard|REFERENCE_STANDARD|{HASH_F}|GLOBAL|2026-10-01T00:00:00Z|2026-11-01T00:00:00Z|T2|ACCEPTED|{HASH_F}"
            ),
        );
        let report = run_report(&input).unwrap();
        assert!(report.contains("RESULT status=CLOSED"));
    }

    #[test]
    fn local_material_cannot_support_global_consumer() {
        let input = mutate(
            valid_manifest(),
            "CASE|case-1|measurement|LINE-1",
            "CASE|case-1|measurement|GLOBAL",
        );
        let report = run_report(&input).unwrap();
        assert!(report.contains("ARTIFACT_SCOPE_MISMATCH"));
    }

    #[test]
    fn root_evidence_tier_is_enforced() {
        let input = mutate(
            valid_manifest(),
            "CASE|case-1|measurement|LINE-1|2026-10-31T12:00:00Z|T2",
            "CASE|case-1|measurement|LINE-1|2026-10-31T12:00:00Z|T3",
        );
        let report = run_report(&input).unwrap();
        assert!(report.contains("ROOT_TIER_TOO_LOW:measurement:required=T3:actual=T2"));
    }

    #[test]
    fn dependency_minimum_tier_is_enforced() {
        let input = mutate(
            valid_manifest(),
            "ROLE_POLICY|CALIBRATION_RECORD|CALIBRATION_CERTIFICATE|T2|1",
            "ROLE_POLICY|CALIBRATION_RECORD|CALIBRATION_CERTIFICATE|T3|1",
        );
        let report = run_report(&input).unwrap();
        assert!(report.contains("MATERIAL_TIER_TOO_LOW:calibration:cal-cert:required=T3:actual=T2"));
    }

    #[test]
    fn artifact_at_valid_until_boundary_is_expired() {
        let input = mutate(
            valid_manifest(),
            "2026-10-31T12:00:00Z|T2",
            "2026-11-01T00:00:00Z|T2",
        );
        let report = run_report(&input).unwrap();
        assert!(report.contains("ARTIFACT_NOT_CURRENT:measurement"));
    }

    #[test]
    fn support_scope_must_be_global_or_exactly_equal() {
        let input = mutate(
            valid_manifest(),
            "ARTIFACT|cal-cert|CALIBRATION_CERTIFICATE|",
            "ARTIFACT|cal-cert|CALIBRATION_CERTIFICATE|",
        );
        let input = mutate(
            input,
            &format!("ARTIFACT|cal-cert|CALIBRATION_CERTIFICATE|{HASH_E}|LINE-1|"),
            &format!("ARTIFACT|cal-cert|CALIBRATION_CERTIFICATE|{HASH_E}|LINE-2|"),
        );
        let report = run_report(&input).unwrap();
        assert!(report.contains("ARTIFACT_SCOPE_MISMATCH:cal-cert"));
        assert!(report.contains("DEPENDENCY_SCOPE_MISMATCH:calibration:cal-cert"));
    }

    #[test]
    fn wrong_role_is_detected_as_missing_required_material() {
        let input = mutate(
            valid_manifest(),
            "CALIBRATION_CERTIFICATE",
            "MATERIAL_LOT",
        );
        let report = run_report(&input).unwrap();
        assert!(report.contains("MISSING_REQUIRED_MATERIAL:calibration:CALIBRATION_CERTIFICATE"));
    }

    #[test]
    fn cycle_blocks_closure() {
        let mut input = valid_manifest();
        input.push_str(&format!("DEP|reference-standard|measurement|{HASH_A}\n"));
        let report = run_report(&input).unwrap();
        assert!(report.contains("DEPENDENCY_CYCLE"));
        assert!(report.contains("RESULT status=BLOCKED"));
    }

    #[test]
    fn unreachable_orphan_artifact_blocks_package() {
        let mut input = valid_manifest();
        input.push_str(&format!(
            "ARTIFACT|orphan|UNUSED|{}|LINE-1|2026-10-01T00:00:00Z|2026-11-01T00:00:00Z|T0|ACCEPTED|{}\n",
            HASH_A, HASH_A
        ));
        let report = run_report(&input).unwrap();
        assert!(report.contains("UNREACHABLE_ARTIFACT:orphan"));
    }

    #[test]
    fn unknown_dependency_reference_is_rejected() {
        let mut input = valid_manifest();
        input.push_str(&format!("DEP|measurement|not-real|{HASH_A}\n"));
        assert!(parse_manifest(&input).unwrap_err().contains("unknown material"));
    }

    #[test]
    fn duplicate_artifact_identity_is_rejected() {
        let mut input = valid_manifest();
        input.push_str(&format!(
            "ARTIFACT|raw-data|RAW_MEASUREMENT|{}|LINE-1|2026-10-01T00:00:00Z|2026-11-01T00:00:00Z|T2|ACCEPTED|{}\n",
            HASH_A, HASH_A
        ));
        assert!(parse_manifest(&input).unwrap_err().contains("duplicate artifact id"));
    }

    #[test]
    fn duplicate_dependency_is_rejected() {
        let mut input = valid_manifest();
        input.push_str(&format!("DEP|measurement|raw-data|{HASH_B}\n"));
        assert!(parse_manifest(&input).unwrap_err().contains("duplicate dependency"));
    }

    #[test]
    fn malformed_artifact_digest_is_rejected() {
        let input = mutate(valid_manifest(), HASH_A, "bad-sha");
        assert!(parse_manifest(&input).unwrap_err().contains("digest is not lowercase SHA-256"));
    }

    #[test]
    fn deterministic_report_replays_identically() {
        let input = valid_manifest();
        assert_eq!(run_report(&input).unwrap(), run_report(&input).unwrap());
    }

    #[test]
    fn multiple_consumers_may_share_one_admitted_material() {
        let mut input = valid_manifest();
        input.push_str(&format!(
            "ARTIFACT|secondary|SECONDARY_MEASUREMENT|{}|LINE-1|2026-10-01T00:00:00Z|2026-11-01T00:00:00Z|T2|ACCEPTED|{}\n\
             DEP|measurement|secondary|{}\n\
             DEP|secondary|raw-data|{}\n\
             DEP|secondary|instrument|{}\n\
             ROLE_POLICY|PROCESS_MEASUREMENT|SECONDARY_MEASUREMENT|T2|1\n\
             ROLE_POLICY|SECONDARY_MEASUREMENT|RAW_MEASUREMENT|T2|1\n\
             ROLE_POLICY|SECONDARY_MEASUREMENT|INSTRUMENT_IDENTITY|T2|1\n",
            HASH_A, HASH_A, HASH_A, HASH_B, HASH_D
        ));
        let report = run_report(&input).unwrap();
        assert!(report.contains("RESULT status=CLOSED"));
    }

    #[test]
    fn optional_role_policy_does_not_require_an_edge() {
        let mut input = valid_manifest();
        input.push_str("ROLE_POLICY|PROCESS_MEASUREMENT|FIRMWARE|T1|0\n");
        let report = run_report(&input).unwrap();
        assert!(report.contains("RESULT status=CLOSED"));
    }

    #[test]
    fn scale_chain_of_ten_thousand_artifacts_closes() {
        let mut input = String::from(
            "CASE|scale|root|LINE-1|2026-10-31T12:00:00Z|T0\n",
        );
        input.push_str(&format!(
            "ARTIFACT|root|ROOT|{}|LINE-1|2026-10-01T00:00:00Z|2026-11-01T00:00:00Z|T0|ACCEPTED|{}\n\
             ROLE_POLICY|ROOT|NODE|T0|1\n\
             ROLE_POLICY|NODE|NODE|T0|0\n",
            HASH_A, HASH_A
        ));
        for index in 0..10_000usize {
            let current = format!("node-{index:05}");
            let digest = if index % 2 == 0 { HASH_B } else { HASH_C };
            input.push_str(&format!(
                "ARTIFACT|{}|NODE|{}|LINE-1|2026-10-01T00:00:00Z|2026-11-01T00:00:00Z|T0|ACCEPTED|{}\n",
                current, digest, HASH_A
            ));
            if index == 0 {
                input.push_str(&format!("DEP|root|{}|{}\n", current, digest));
            } else {
                let previous = format!("node-{:05}", index - 1);
                input.push_str(&format!("DEP|{}|{}|{}\n", previous, current, digest));
            }
        }
        let report = run_report(&input).unwrap();
        assert!(report.contains("RESULT status=CLOSED"));
        assert!(report.contains("reachable=10001"));
    }

    #[test]
    fn claim_ceiling_disclaims_signature_and_raw_byte_verification() {
        let report = run_report(&valid_manifest()).unwrap();
        assert!(report.contains("raw_artifact_bytes_hashed=false"));
        assert!(report.contains("signatures_verified=false"));
    }
}
