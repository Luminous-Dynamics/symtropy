//! Time- and scope-aware manufacturing twin change-impact reference.
//!
//! This is a read-only, offline analysis tool. It determines which downstream
//! evidence nodes and engineering claims require requalification after declared
//! version changes. It does not authenticate change records, verify artifact
//! bytes, determine physical drift, or authorize a production change.
//!
//! Build and test:
//!   rustc --edition=2021 -D warnings --test tools/manufacturing-assurance/change_impact.rs -o /tmp/mfg-change-tests
//!   /tmp/mfg-change-tests
//!
//! Build and run:
//!   rustc --edition=2021 -D warnings tools/manufacturing-assurance/change_impact.rs -o /tmp/mfg-change
//!   /tmp/mfg-change <manifest.tsv>

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
struct Item {
    id: String,
    kind: String,
    scope: String,
    revision: String,
    digest: String,
    valid_from: String,
    valid_until: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Claim {
    id: String,
    revision: String,
    scope: String,
    valid_from: String,
    valid_until: String,
    class: String,
    status: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Dependency {
    upstream_id: String,
    upstream_revision: String,
    downstream_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Change {
    id: String,
    target_id: String,
    old_revision: String,
    new_revision: String,
    old_digest: String,
    new_digest: String,
    class: String,
    scope: String,
    effective_from: String,
    effective_until: String,
    semantic_change: bool,
    receipt_id: String,
}

#[derive(Clone, Debug, Default)]
struct Manifest {
    items: BTreeMap<String, Item>,
    claims: BTreeMap<String, Claim>,
    dependencies: Vec<Dependency>,
    changes: Vec<Change>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ClaimImpact {
    claim_id: String,
    current_status: String,
    witness_path: Vec<String>,
    dependency_path_count: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ChangeImpact {
    change_id: String,
    receipt_id: String,
    disposition: String,
    direct_item_id: String,
    affected_items: Vec<String>,
    affected_claims: Vec<ClaimImpact>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ImpactReport {
    change_impacts: Vec<ChangeImpact>,
    union_affected_items: Vec<String>,
    union_affected_claims: Vec<String>,
}

fn nonempty(value: &str, context: &str) -> Result<String, String> {
    if value.is_empty() || value == "-" {
        Err(format!("{context}: missing value"))
    } else {
        Ok(value.to_string())
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
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
        || !bytes.iter().enumerate().all(|(i, b)| {
            matches!(i, 4 | 7 | 10 | 13 | 16 | 19) || b.is_ascii_digit()
        })
    {
        return false;
    }
    let parse = |range: std::ops::Range<usize>| -> Option<u32> {
        std::str::from_utf8(&bytes[range]).ok()?.parse().ok()
    };
    let year = match parse(0..4) { Some(v) => v, None => return false };
    let month = match parse(5..7) { Some(v) => v, None => return false };
    let day = match parse(8..10) { Some(v) => v, None => return false };
    let hour = match parse(11..13) { Some(v) => v, None => return false };
    let minute = match parse(14..16) { Some(v) => v, None => return false };
    let second = match parse(17..19) { Some(v) => v, None => return false };
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

fn valid_interval(start: &str, end: &str, label: &str) -> Result<(), String> {
    if !valid_utc(start) || !valid_utc(end) || start >= end {
        return Err(format!("{label}: invalid UTC validity interval"));
    }
    Ok(())
}

fn parse_bool(value: &str, context: &str) -> Result<bool, String> {
    match value {
        "TRUE" => Ok(true),
        "FALSE" => Ok(false),
        _ => Err(format!("{context}: expected TRUE or FALSE")),
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
            "ITEM" => {
                if fields.len() != 8 {
                    return Err(format!("line {line_number}: ITEM requires 7 fields"));
                }
                let item = Item {
                    id: nonempty(fields[1], "ITEM id")?,
                    kind: nonempty(fields[2], "ITEM kind")?,
                    scope: nonempty(fields[3], "ITEM scope")?,
                    revision: nonempty(fields[4], "ITEM revision")?,
                    digest: nonempty(fields[5], "ITEM digest")?,
                    valid_from: nonempty(fields[6], "ITEM valid_from")?,
                    valid_until: nonempty(fields[7], "ITEM valid_until")?,
                };
                valid_interval(&item.valid_from, &item.valid_until, "ITEM validity")?;
                if !is_sha256(&item.digest) {
                    return Err(format!("line {line_number}: ITEM {} has malformed SHA-256", item.id));
                }
                if manifest.items.insert(item.id.clone(), item).is_some() {
                    return Err(format!("line {line_number}: duplicate item id"));
                }
            }
            "CLAIM" => {
                if fields.len() != 8 {
                    return Err(format!("line {line_number}: CLAIM requires 7 fields"));
                }
                let claim = Claim {
                    id: nonempty(fields[1], "CLAIM id")?,
                    revision: nonempty(fields[2], "CLAIM revision")?,
                    scope: nonempty(fields[3], "CLAIM scope")?,
                    valid_from: nonempty(fields[4], "CLAIM valid_from")?,
                    valid_until: nonempty(fields[5], "CLAIM valid_until")?,
                    class: nonempty(fields[6], "CLAIM class")?,
                    status: nonempty(fields[7], "CLAIM status")?,
                };
                valid_interval(&claim.valid_from, &claim.valid_until, "CLAIM validity")?;
                if !matches!(
                    claim.class.as_str(),
                    "PHYSICS" | "MEASUREMENT" | "CONFORMITY" | "RELEASE" | "INTERVENTION" | "CAUSAL_OUTCOME"
                ) {
                    return Err(format!("line {line_number}: unsupported claim class {}", claim.class));
                }
                if !matches!(claim.status.as_str(), "QUALIFIED" | "SYNTHETIC_ONLY") {
                    return Err(format!("line {line_number}: unsupported claim status {}", claim.status));
                }
                let id = claim.id.clone();
                if manifest.claims.insert(id, claim).is_some() || manifest.items.contains_key(fields[1]) {
                    return Err(format!("line {line_number}: duplicate graph identity"));
                }
            }
            "DEP" => {
                if fields.len() != 4 {
                    return Err(format!("line {line_number}: DEP requires 3 fields"));
                }
                manifest.dependencies.push(Dependency {
                    upstream_id: nonempty(fields[1], "DEP upstream_id")?,
                    upstream_revision: nonempty(fields[2], "DEP upstream_revision")?,
                    downstream_id: nonempty(fields[3], "DEP downstream_id")?,
                });
            }
            "CHANGE" => {
                if fields.len() != 13 {
                    return Err(format!("line {line_number}: CHANGE requires 12 fields"));
                }
                let change = Change {
                    id: nonempty(fields[1], "CHANGE id")?,
                    target_id: nonempty(fields[2], "CHANGE target_id")?,
                    old_revision: nonempty(fields[3], "CHANGE old_revision")?,
                    new_revision: nonempty(fields[4], "CHANGE new_revision")?,
                    old_digest: nonempty(fields[5], "CHANGE old_digest")?,
                    new_digest: nonempty(fields[6], "CHANGE new_digest")?,
                    class: nonempty(fields[7], "CHANGE class")?,
                    scope: nonempty(fields[8], "CHANGE scope")?,
                    effective_from: nonempty(fields[9], "CHANGE effective_from")?,
                    effective_until: nonempty(fields[10], "CHANGE effective_until")?,
                    semantic_change: parse_bool(fields[11], "CHANGE semantic_change")?,
                    receipt_id: nonempty(fields[12], "CHANGE receipt_id")?,
                };
                valid_interval(
                    &change.effective_from,
                    &change.effective_until,
                    "CHANGE effective interval",
                )?;
                if !is_sha256(&change.old_digest) || !is_sha256(&change.new_digest) {
                    return Err(format!("line {line_number}: change {} has malformed SHA-256", change.id));
                }
                if !matches!(
                    change.class.as_str(),
                    "SUBSTANTIVE" | "SCHEMA" | "CALIBRATION" | "MATERIAL" | "ENVIRONMENT" | "PROCESS" | "METADATA_ONLY"
                ) {
                    return Err(format!("line {line_number}: unsupported change class {}", change.class));
                }
                if manifest.changes.iter().any(|existing| existing.id == change.id) {
                    return Err(format!("line {line_number}: duplicate change id"));
                }
                manifest.changes.push(change);
            }
            other => return Err(format!("line {line_number}: unknown record type {other:?}")),
        }
    }

    Ok(manifest)
}

fn graph_identity_exists(manifest: &Manifest, id: &str) -> bool {
    manifest.items.contains_key(id) || manifest.claims.contains_key(id)
}

fn graph_revision(manifest: &Manifest, id: &str) -> Option<&str> {
    manifest
        .items
        .get(id)
        .map(|item| item.revision.as_str())
        .or_else(|| manifest.claims.get(id).map(|claim| claim.revision.as_str()))
}

fn validate_graph(manifest: &Manifest) -> Result<BTreeMap<String, Vec<String>>, String> {
    if manifest.items.is_empty() {
        return Err("manifest must contain at least one ITEM".to_string());
    }
    if manifest.changes.is_empty() {
        return Err("manifest must contain at least one CHANGE".to_string());
    }

    let mut adjacency: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut indegree: BTreeMap<String, usize> = BTreeMap::new();

    for id in manifest.items.keys().chain(manifest.claims.keys()) {
        if manifest.items.contains_key(id) && manifest.claims.contains_key(id) {
            return Err(format!("duplicate graph identity {id}"));
        }
        adjacency.insert(id.clone(), Vec::new());
        indegree.insert(id.clone(), 0);
    }

    let mut seen = BTreeSet::new();
    for dependency in &manifest.dependencies {
        if !graph_identity_exists(manifest, &dependency.upstream_id) {
            return Err(format!("dependency references unknown upstream {}", dependency.upstream_id));
        }
        if !graph_identity_exists(manifest, &dependency.downstream_id) {
            return Err(format!("dependency references unknown downstream {}", dependency.downstream_id));
        }
        if graph_revision(manifest, &dependency.upstream_id) != Some(dependency.upstream_revision.as_str()) {
            return Err(format!(
                "dependency {} -> {} binds the wrong upstream revision",
                dependency.upstream_id, dependency.downstream_id
            ));
        }
        let key = format!(
            "{}@{}->{}",
            dependency.upstream_id, dependency.upstream_revision, dependency.downstream_id
        );
        if !seen.insert(key) {
            return Err(format!(
                "duplicate dependency {} -> {}",
                dependency.upstream_id, dependency.downstream_id
            ));
        }
        adjacency
            .get_mut(&dependency.upstream_id)
            .expect("graph node was inserted")
            .push(dependency.downstream_id.clone());
        *indegree
            .get_mut(&dependency.downstream_id)
            .expect("graph node was inserted") += 1;
    }

    for claim_id in manifest.claims.keys() {
        if indegree.get(claim_id).copied().unwrap_or(0) == 0 {
            return Err(format!("claim {claim_id} has no supporting dependency"));
        }
    }

    for children in adjacency.values_mut() {
        children.sort();
        children.dedup();
    }

    let mut ready: BTreeSet<String> = indegree
        .iter()
        .filter_map(|(id, count)| if *count == 0 { Some(id.clone()) } else { None })
        .collect();
    let mut visited = 0usize;
    while let Some(id) = ready.pop_first() {
        visited += 1;
        if let Some(children) = adjacency.get(&id) {
            for child in children {
                let count = indegree
                    .get_mut(child)
                    .expect("child exists in indegree map");
                *count -= 1;
                if *count == 0 {
                    ready.insert(child.clone());
                }
            }
        }
    }

    if visited != indegree.len() {
        let cyclic: Vec<String> = indegree
            .iter()
            .filter_map(|(id, count)| if *count > 0 { Some(id.clone()) } else { None })
            .collect();
        return Err(format!("dependency graph contains a cycle: {}", cyclic.join(",")));
    }

    Ok(adjacency)
}

fn validate_change(manifest: &Manifest, change: &Change) -> Result<bool, String> {
    let item = manifest
        .items
        .get(&change.target_id)
        .ok_or_else(|| format!("change {} targets unknown item {}", change.id, change.target_id))?;

    if item.revision != change.old_revision || item.digest != change.old_digest {
        return Err(format!(
            "change {} old revision/digest does not match target {}",
            change.id, change.target_id
        ));
    }

    if item.scope == "GLOBAL" && change.scope != "GLOBAL" {
        return Err(format!(
            "change {} scopes a GLOBAL item to a local context; use GLOBAL",
            change.id
        ));
    }
    if item.scope != "GLOBAL" && change.scope != "GLOBAL" && item.scope != change.scope {
        return Err(format!(
            "change {} scope does not match target item {}",
            change.id, change.target_id
        ));
    }
    if !intervals_overlap(
        &item.valid_from,
        &item.valid_until,
        &change.effective_from,
        &change.effective_until,
    ) {
        return Err(format!(
            "change {} effective interval does not overlap target item validity",
            change.id
        ));
    }
    let compatible_kind = match change.class.as_str() {
        "CALIBRATION" => item.kind == "CALIBRATION",
        "MATERIAL" => item.kind == "MATERIAL" || item.kind == "MATERIAL_LOT",
        "ENVIRONMENT" => item.kind == "ENVIRONMENT" || item.kind == "BOUNDARY_CONDITION",
        "PROCESS" => item.kind == "PROCESS" || item.kind == "PROCESS_PARAMETER" || item.kind == "ROUTE",
        "SCHEMA" => item.kind == "SCHEMA" || item.kind == "INTERFACE_MAPPING",
        "SUBSTANTIVE" | "METADATA_ONLY" => true,
        _ => false,
    };
    if !compatible_kind {
        return Err(format!(
            "change {} class {} is incompatible with target kind {}",
            change.id, change.class, item.kind
        ));
    }

    // Explicit no-op: metadata-only revision change, identical content digest,
    // and a declared absence of semantic change. It never invalidates downstream claims.
    if change.class == "METADATA_ONLY" && !change.semantic_change {
        if change.old_digest != change.new_digest || change.old_revision == change.new_revision {
            return Err(format!(
                "change {} is not a valid metadata-only no-op",
                change.id
            ));
        }
        return Ok(true);
    }

    if change.class == "METADATA_ONLY" && change.semantic_change {
        return Err(format!(
            "change {} declares semantic change as METADATA_ONLY",
            change.id
        ));
    }
    if !change.semantic_change {
        return Err(format!(
            "change {} lacks a semantic-change declaration",
            change.id
        ));
    }
    if change.old_revision == change.new_revision || change.old_digest == change.new_digest {
        return Err(format!(
            "change {} declares a substantive change without a new revision and digest",
            change.id
        ));
    }

    Ok(false)
}

fn claim_scope_affected(change_scope: &str, claim_scope: &str) -> bool {
    change_scope == "GLOBAL" || claim_scope == "GLOBAL" || change_scope == claim_scope
}

fn intervals_overlap(
    left_start: &str,
    left_end: &str,
    right_start: &str,
    right_end: &str,
) -> bool {
    left_start <= right_end && right_start <= left_end
}

fn add_saturating(left: u64, right: u64) -> u64 {
    left.saturating_add(right)
}

fn traverse_impact(
    manifest: &Manifest,
    adjacency: &BTreeMap<String, Vec<String>>,
    change: &Change,
) -> ChangeImpact {
    let mut distances: BTreeMap<String, usize> = BTreeMap::new();
    let mut witness_paths: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut path_counts: BTreeMap<String, u64> = BTreeMap::new();

    distances.insert(change.target_id.clone(), 0);
    witness_paths.insert(change.target_id.clone(), vec![change.target_id.clone()]);
    path_counts.insert(change.target_id.clone(), 1);

    // The dependency graph is already a DAG. Topological propagation counts
    // distinct paths while retaining one deterministic shortest witness path.
    let mut indegree: BTreeMap<String, usize> = BTreeMap::new();
    for id in manifest.items.keys().chain(manifest.claims.keys()) {
        indegree.insert(id.clone(), 0);
    }
    for children in adjacency.values() {
        for child in children {
            *indegree.get_mut(child).expect("validated child") += 1;
        }
    }
    let mut ready: BTreeSet<String> = indegree
        .iter()
        .filter_map(|(id, count)| if *count == 0 { Some(id.clone()) } else { None })
        .collect();

    while let Some(id) = ready.pop_first() {
        if let Some(children) = adjacency.get(&id) {
            for child in children {
                let child_is_in_scope = if let Some(item) = manifest.items.get(child) {
                    change.scope == "GLOBAL" || item.scope == "GLOBAL" || item.scope == change.scope
                } else if let Some(claim) = manifest.claims.get(child) {
                    claim_scope_affected(&change.scope, &claim.scope)
                } else {
                    false
                };
                let child_is_in_time = if let Some(item) = manifest.items.get(child) {
                    intervals_overlap(
                        &item.valid_from,
                        &item.valid_until,
                        &change.effective_from,
                        &change.effective_until,
                    )
                } else if let Some(claim) = manifest.claims.get(child) {
                    intervals_overlap(
                        &claim.valid_from,
                        &claim.valid_until,
                        &change.effective_from,
                        &change.effective_until,
                    )
                } else {
                    false
                };

                if child_is_in_scope && child_is_in_time {
                    if let Some(parent_paths) = path_counts.get(&id).copied() {
                        let existing = path_counts.get(child).copied().unwrap_or(0);
                        path_counts.insert(child.clone(), add_saturating(existing, parent_paths));
                        let candidate_distance = distances.get(&id).copied().unwrap_or(0) + 1;
                        let mut candidate_path = witness_paths
                            .get(&id)
                            .cloned()
                            .unwrap_or_else(|| vec![id.clone()]);
                        candidate_path.push(child.clone());

                        let replace = match distances.get(child).copied() {
                            None => true,
                            Some(previous) if candidate_distance < previous => true,
                            Some(previous) if candidate_distance == previous => {
                                witness_paths
                                    .get(child)
                                    .map(|existing_path| candidate_path < *existing_path)
                                    .unwrap_or(true)
                            }
                            _ => false,
                        };
                        if replace {
                            distances.insert(child.clone(), candidate_distance);
                            witness_paths.insert(child.clone(), candidate_path);
                        }
                    }
                }

                let count = indegree.get_mut(child).expect("validated child");
                *count -= 1;
                if *count == 0 {
                    ready.insert(child.clone());
                }
            }
        }
    }

    let mut affected_items = Vec::new();
    for id in distances.keys() {
        if id != &change.target_id && manifest.items.contains_key(id) {
            affected_items.push(id.clone());
        }
    }
    affected_items.sort();

    let mut affected_claims = Vec::new();
    for (claim_id, claim) in &manifest.claims {
        if !path_counts.contains_key(claim_id) {
            continue;
        }
        let scope_matches = claim_scope_affected(&change.scope, &claim.scope);
        let time_matches = intervals_overlap(
            &claim.valid_from,
            &claim.valid_until,
            &change.effective_from,
            &change.effective_until,
        );
        if scope_matches && time_matches {
            affected_claims.push(ClaimImpact {
                claim_id: claim_id.clone(),
                current_status: claim.status.clone(),
                witness_path: witness_paths.get(claim_id).cloned().unwrap_or_default(),
                dependency_path_count: path_counts.get(claim_id).copied().unwrap_or(0),
            });
        }
    }
    affected_claims.sort_by(|left, right| left.claim_id.cmp(&right.claim_id));

    let disposition = if affected_claims.is_empty() && affected_items.is_empty() {
        "NO_DOWNSTREAM_IMPACT"
    } else if affected_claims.is_empty() {
        "AFFECTED_EVIDENCE_NO_CURRENT_CLAIM"
    } else {
        "REQUALIFICATION_REQUIRED"
    };

    ChangeImpact {
        change_id: change.id.clone(),
        receipt_id: change.receipt_id.clone(),
        disposition: disposition.to_string(),
        direct_item_id: change.target_id.clone(),
        affected_items,
        affected_claims,
    }
}

fn analyze(manifest: &Manifest) -> Result<ImpactReport, String> {
    let adjacency = validate_graph(manifest)?;

    let mut impacts = Vec::new();
    let mut union_items = BTreeSet::new();
    let mut union_claims = BTreeSet::new();

    let mut change_ids = BTreeSet::new();
    let mut change_receipts = BTreeSet::new();
    for change in &manifest.changes {
        if !change_ids.insert(change.id.as_str()) {
            return Err(format!("duplicate change id {}", change.id));
        }
        if !change_receipts.insert(change.receipt_id.as_str()) {
            return Err(format!("duplicate or reused change receipt {}", change.receipt_id));
        }
        let no_op = validate_change(manifest, change)?;
        let impact = if no_op {
            ChangeImpact {
                change_id: change.id.clone(),
                receipt_id: change.receipt_id.clone(),
                disposition: "NO_OP_METADATA_ONLY".to_string(),
                direct_item_id: change.target_id.clone(),
                affected_items: Vec::new(),
                affected_claims: Vec::new(),
            }
        } else {
            traverse_impact(manifest, &adjacency, change)
        };

        for item in &impact.affected_items {
            union_items.insert(item.clone());
        }
        for claim in &impact.affected_claims {
            union_claims.insert(claim.claim_id.clone());
        }
        impacts.push(impact);
    }
    impacts.sort_by(|left, right| left.change_id.cmp(&right.change_id));

    Ok(ImpactReport {
        change_impacts: impacts,
        union_affected_items: union_items.into_iter().collect(),
        union_affected_claims: union_claims.into_iter().collect(),
    })
}

fn parse_and_analyze(input: &str) -> Result<ImpactReport, String> {
    let manifest = parse_manifest(input)?;
    analyze(&manifest)
}

fn render_report(report: &ImpactReport) -> String {
    let mut output = String::new();
    for change in &report.change_impacts {
        output.push_str(&format!(
            "CHANGE_IMPACT change={} receipt_ref={} disposition={} direct={} affected_items={} affected_claims={}\n",
            change.change_id,
            change.receipt_id,
            change.disposition,
            change.direct_item_id,
            change.affected_items.len(),
            change.affected_claims.len()
        ));
        for item in &change.affected_items {
            output.push_str(&format!("AFFECTED_ITEM change={} item={}\n", change.change_id, item));
        }
        for claim in &change.affected_claims {
            output.push_str(&format!(
                "AFFECTED_CLAIM change={} claim={} current_status={} dependency_paths={} witness={}\n",
                change.change_id,
                claim.claim_id,
                claim.current_status,
                claim.dependency_path_count,
                claim.witness_path.join("->")
            ));
        }
    }
    output.push_str(&format!(
        "IMPACT_UNION affected_items={} affected_claims={}\n",
        report.union_affected_items.len(),
        report.union_affected_claims.len()
    ));
    output.push_str(&format!(
        "CLAIM_CEILING declared_change_impact_only=true change_receipts_authenticated=false artifact_bytes_hashed=false physical_drift_detected=false requalification_performed=false production_authority_granted=false\n"
    ));
    output
}

#[cfg(not(test))]
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!(
            "usage: {} <change-impact-manifest.tsv>",
            args.first().map(String::as_str).unwrap_or("mfg-change-impact")
        );
        std::process::exit(2);
    }
    let input = match std::fs::read_to_string(&args[1]) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("mfg-change-impact: cannot read {}: {error}", args[1]);
            std::process::exit(2);
        }
    };
    match parse_and_analyze(&input) {
        Ok(report) => print!("{}", render_report(&report)),
        Err(error) => {
            eprintln!("mfg-change-impact: {error}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"# Synthetic-only lifecycle change-impact fixture.
ITEM|physics|PHYSICS_MODEL|GLOBAL|r1|0000000000000000000000000000000000000000000000000000000000000001|2026-01-01T00:00:00Z|2027-12-31T23:59:59Z
ITEM|simulation-a|SIMULATION_RUN|LINE_A|r1|0000000000000000000000000000000000000000000000000000000000000002|2026-01-01T00:00:00Z|2027-12-31T23:59:59Z
ITEM|conformity-a|CONFORMITY_RECEIPT|LINE_A|r1|0000000000000000000000000000000000000000000000000000000000000003|2026-01-01T00:00:00Z|2027-12-31T23:59:59Z
ITEM|simulation-b|SIMULATION_RUN|LINE_B|r1|0000000000000000000000000000000000000000000000000000000000000004|2026-01-01T00:00:00Z|2027-12-31T23:59:59Z
ITEM|conformity-b|CONFORMITY_RECEIPT|LINE_B|r1|0000000000000000000000000000000000000000000000000000000000000005|2026-01-01T00:00:00Z|2027-12-31T23:59:59Z
ITEM|calibration-a|CALIBRATION|LINE_A|r1|0000000000000000000000000000000000000000000000000000000000000006|2026-01-01T00:00:00Z|2027-12-31T23:59:59Z
ITEM|measurement-a|MEASUREMENT|LINE_A|r1|0000000000000000000000000000000000000000000000000000000000000007|2026-01-01T00:00:00Z|2027-12-31T23:59:59Z
ITEM|uncertainty-a|UNCERTAINTY_BUDGET|LINE_A|r1|0000000000000000000000000000000000000000000000000000000000000008|2026-01-01T00:00:00Z|2027-12-31T23:59:59Z
ITEM|conformity-cal-a|CONFORMITY_RECEIPT|LINE_A|r1|0000000000000000000000000000000000000000000000000000000000000009|2026-01-01T00:00:00Z|2027-12-31T23:59:59Z
CLAIM|claim-a|r1|LINE_A|2026-01-01T00:00:00Z|2027-12-31T23:59:59Z|CONFORMITY|SYNTHETIC_ONLY
CLAIM|claim-a-historical|r1|LINE_A|2026-01-01T00:00:00Z|2026-06-30T23:59:59Z|CONFORMITY|SYNTHETIC_ONLY
CLAIM|claim-b|r1|LINE_B|2026-01-01T00:00:00Z|2027-12-31T23:59:59Z|CONFORMITY|SYNTHETIC_ONLY
DEP|physics|r1|simulation-a
DEP|simulation-a|r1|conformity-a
DEP|conformity-a|r1|claim-a
DEP|conformity-a|r1|claim-a-historical
DEP|physics|r1|simulation-b
DEP|simulation-b|r1|conformity-b
DEP|conformity-b|r1|claim-b
DEP|calibration-a|r1|measurement-a
DEP|measurement-a|r1|uncertainty-a
DEP|uncertainty-a|r1|conformity-cal-a
DEP|conformity-cal-a|r1|claim-a
CHANGE|change-physics|physics|r1|r2|0000000000000000000000000000000000000000000000000000000000000001|aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa|SUBSTANTIVE|GLOBAL|2026-10-01T00:00:00Z|9999-12-31T23:59:59Z|TRUE|receipt-physics
"#;

    fn change_for(input: &str, marker: &str, replacement: &str) -> String {
        input.replace(marker, replacement)
    }

    #[test]
    fn global_physics_change_impacts_both_lines_and_preserves_expired_history() {
        let report = parse_and_analyze(FIXTURE).unwrap();
        let impact = report
            .change_impacts
            .iter()
            .find(|entry| entry.change_id == "change-physics")
            .unwrap();
        let affected: Vec<&str> = impact
            .affected_claims
            .iter()
            .map(|entry| entry.claim_id.as_str())
            .collect();
        assert_eq!(affected, vec!["claim-a", "claim-b"]);
        assert!(!affected.contains(&"claim-a-historical"));
        assert!(impact.affected_items.contains(&"simulation-a".to_string()));
        assert!(impact.affected_items.contains(&"conformity-b".to_string()));
    }

    #[test]
    fn scoped_calibration_change_does_not_invalidate_other_line() {
        let input = change_for(
            FIXTURE,
            "CHANGE|change-physics|physics|r1|r2|0000000000000000000000000000000000000000000000000000000000000001|aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa|SUBSTANTIVE|GLOBAL|2026-10-01T00:00:00Z|9999-12-31T23:59:59Z|TRUE|receipt-physics",
            "CHANGE|change-cal|calibration-a|r1|r2|0000000000000000000000000000000000000000000000000000000000000006|bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb|CALIBRATION|LINE_A|2026-10-01T00:00:00Z|9999-12-31T23:59:59Z|TRUE|receipt-cal",
        );
        let report = parse_and_analyze(&input).unwrap();
        let impact = &report.change_impacts[0];
        assert_eq!(impact.affected_claims.len(), 1);
        assert_eq!(impact.affected_claims[0].claim_id, "claim-a");
        assert!(impact.affected_items.contains(&"measurement-a".to_string()));
        assert!(!report.union_affected_claims.contains(&"claim-b".to_string()));
    }

    #[test]
    fn metadata_only_same_digest_and_explicit_no_semantic_change_is_noop() {
        let marker = "CHANGE|change-physics|physics|r1|r2|0000000000000000000000000000000000000000000000000000000000000001|aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa|SUBSTANTIVE|GLOBAL|2026-10-01T00:00:00Z|9999-12-31T23:59:59Z|TRUE|receipt-physics";
        let replacement = "CHANGE|change-meta|physics|r1|r2|0000000000000000000000000000000000000000000000000000000000000001|0000000000000000000000000000000000000000000000000000000000000001|METADATA_ONLY|GLOBAL|2026-10-01T00:00:00Z|9999-12-31T23:59:59Z|FALSE|receipt-meta";
        let report = parse_and_analyze(&change_for(FIXTURE, marker, replacement)).unwrap();
        assert_eq!(report.change_impacts[0].disposition, "NO_OP_METADATA_ONLY");
        assert!(report.union_affected_claims.is_empty());
    }

    #[test]
    fn identical_digest_with_semantic_change_cannot_be_called_metadata_only() {
        let input = FIXTURE.replace(
            "SUBSTANTIVE|GLOBAL|2026-10-01T00:00:00Z|9999-12-31T23:59:59Z|TRUE|receipt-physics",
            "METADATA_ONLY|GLOBAL|2026-10-01T00:00:00Z|9999-12-31T23:59:59Z|TRUE|receipt-physics",
        );
        assert!(parse_and_analyze(&input).unwrap_err().contains("semantic change as METADATA_ONLY"));
    }

    #[test]
    fn wrong_old_revision_or_digest_fails_closed() {
        let input = FIXTURE.replace(
            "CHANGE|change-physics|physics|r1|",
            "CHANGE|change-physics|physics|r0|",
        );
        assert!(parse_and_analyze(&input).unwrap_err().contains("old revision/digest"));
    }

    #[test]
    fn unknown_change_target_fails_closed() {
        let input = FIXTURE.replace(
            "CHANGE|change-physics|physics|",
            "CHANGE|change-physics|not-present|",
        );
        assert!(parse_and_analyze(&input).unwrap_err().contains("unknown item"));
    }

    #[test]
    fn dependency_cycle_fails_closed() {
        let input = FIXTURE.replace(
            "DEP|conformity-b|r1|claim-b",
            "DEP|conformity-b|r1|claim-b\nDEP|claim-b|r1|physics",
        );
        assert!(parse_and_analyze(&input).unwrap_err().contains("cycle"));
    }

    #[test]
    fn dependency_revision_mismatch_fails_closed() {
        let input = FIXTURE.replace("DEP|physics|r1|simulation-a", "DEP|physics|r0|simulation-a");
        assert!(parse_and_analyze(&input).unwrap_err().contains("wrong upstream revision"));
    }

    #[test]
    fn metadata_only_with_changed_digest_is_rejected() {
        let input = FIXTURE.replace(
            "SUBSTANTIVE|GLOBAL|2026-10-01T00:00:00Z|9999-12-31T23:59:59Z|TRUE|receipt-physics",
            "METADATA_ONLY|GLOBAL|2026-10-01T00:00:00Z|9999-12-31T23:59:59Z|FALSE|receipt-physics",
        );
        assert!(parse_and_analyze(&input).unwrap_err().contains("not a valid metadata-only no-op"));
    }

    #[test]
    fn identical_inputs_produce_byte_identical_reports() {
        let first = render_report(&parse_and_analyze(FIXTURE).unwrap());
        let second = render_report(&parse_and_analyze(FIXTURE).unwrap());
        assert_eq!(first, second);
    }

    #[test]
    fn global_change_scope_is_required_for_global_target_item() {
        let input = FIXTURE.replace(
            "|SUBSTANTIVE|GLOBAL|2026-10-01T00:00:00Z|",
            "|SUBSTANTIVE|LINE_A|2026-10-01T00:00:00Z|",
        );
        assert!(parse_and_analyze(&input).unwrap_err().contains("scopes a GLOBAL item"));
    }

    #[test]
    fn combined_changes_deduplicate_impacted_claims() {
        let extra = "CHANGE|change-cal|calibration-a|r1|r2|0000000000000000000000000000000000000000000000000000000000000006|bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb|CALIBRATION|LINE_A|2026-10-01T00:00:00Z|9999-12-31T23:59:59Z|TRUE|receipt-cal";
        let report = parse_and_analyze(&format!("{FIXTURE}\n{extra}\n")).unwrap();
        assert_eq!(
            report.union_affected_claims,
            vec!["claim-a".to_string(), "claim-b".to_string()]
        );
        assert_eq!(report.change_impacts.len(), 2);
    }

    #[test]
    fn change_effective_after_claim_expiry_preserves_claim() {
        let input = FIXTURE.replace(
            "2026-10-01T00:00:00Z|9999-12-31T23:59:59Z|TRUE|receipt-physics",
            "2027-01-01T00:00:00Z|9999-12-31T23:59:59Z|TRUE|receipt-physics",
        );
        let report = parse_and_analyze(&input).unwrap();
        assert!(!report.union_affected_claims.contains(&"claim-a-historical".to_string()));
    }

    #[test]
    fn requires_claim_supporting_dependency() {
        let input = FIXTURE.replace("DEP|conformity-a|r1|claim-a-historical\n", "");
        assert!(parse_and_analyze(&input).unwrap_err().contains("has no supporting dependency"));
    }

    #[test]
    fn multiple_dependency_paths_are_counted_with_a_deterministic_witness() {
        let input = FIXTURE.replace(
            "DEP|conformity-cal-a|r1|claim-a",
            "DEP|conformity-cal-a|r1|claim-a\nDEP|simulation-a|r1|conformity-cal-a",
        );
        let report = parse_and_analyze(&input).unwrap();
        let impact = &report.change_impacts[0];
        let claim = impact.affected_claims.iter().find(|entry| entry.claim_id == "claim-a").unwrap();
        assert_eq!(claim.dependency_path_count, 2);
        assert_eq!(claim.witness_path.first().unwrap(), "physics");
        assert_eq!(claim.witness_path.last().unwrap(), "claim-a");
    }

    #[test]
    fn change_class_must_match_target_item_kind() {
        let input = FIXTURE.replace(
            "|SUBSTANTIVE|GLOBAL|2026-10-01T00:00:00Z|",
            "|CALIBRATION|GLOBAL|2026-10-01T00:00:00Z|",
        );
        assert!(parse_and_analyze(&input).unwrap_err().contains("incompatible with target kind"));
    }

    #[test]
    fn change_effective_interval_must_overlap_target_item_validity() {
        let input = FIXTURE.replace(
            "2027-12-31T23:59:59Z\nITEM|simulation-a",
            "2026-08-31T23:59:59Z\nITEM|simulation-a",
        );
        assert!(parse_and_analyze(&input).unwrap_err().contains("does not overlap target item validity"));
    }

    #[test]
    fn unknown_dependency_target_fails_closed() {
        let input = FIXTURE.replace(
            "DEP|conformity-b|r1|claim-b",
            "DEP|conformity-b|r1|claim-b\nDEP|not-present|r1|claim-b",
        );
        assert!(parse_and_analyze(&input).unwrap_err().contains("unknown upstream"));
    }

    #[test]
    fn reused_change_receipt_fails_closed() {
        let extra = "CHANGE|change-cal|calibration-a|r1|r2|0000000000000000000000000000000000000000000000000000000000000006|bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb|CALIBRATION|LINE_A|2026-10-01T00:00:00Z|9999-12-31T23:59:59Z|TRUE|receipt-physics";
        assert!(parse_and_analyze(&format!("{FIXTURE}\n{extra}\n")).unwrap_err().contains("reused change receipt"));
    }

    #[test]
    fn ten_thousand_dependency_items_complete_with_exact_impact() {
        let mut manifest = Manifest::default();
        let valid_from = "2026-01-01T00:00:00Z".to_string();
        let valid_until = "2027-12-31T23:59:59Z".to_string();
        for index in 0..=10_000usize {
            let id = format!("item-{index:05}");
            let kind = if index == 0 { "PHYSICS_MODEL" } else { "DERIVED_EVIDENCE" };
            let digest = if index == 0 { "1".repeat(64) } else { "a".repeat(64) };
            manifest.items.insert(id.clone(), Item {
                id,
                kind: kind.to_string(),
                scope: "LINE_A".to_string(),
                revision: "r1".to_string(),
                digest,
                valid_from: valid_from.clone(),
                valid_until: valid_until.clone(),
            });
        }
        for index in 1..=10_000usize {
            manifest.dependencies.push(Dependency {
                upstream_id: format!("item-{:05}", index - 1),
                upstream_revision: "r1".to_string(),
                downstream_id: format!("item-{index:05}"),
            });
        }
        manifest.claims.insert("claim-scale".to_string(), Claim {
            id: "claim-scale".to_string(),
            revision: "r1".to_string(),
            scope: "LINE_A".to_string(),
            valid_from: valid_from.clone(),
            valid_until: valid_until.clone(),
            class: "PHYSICS".to_string(),
            status: "SYNTHETIC_ONLY".to_string(),
        });
        manifest.dependencies.push(Dependency {
            upstream_id: "item-10000".to_string(),
            upstream_revision: "r1".to_string(),
            downstream_id: "claim-scale".to_string(),
        });
        manifest.changes.push(Change {
            id: "change-scale".to_string(),
            target_id: "item-00000".to_string(),
            old_revision: "r1".to_string(),
            new_revision: "r2".to_string(),
            old_digest: "1".repeat(64),
            new_digest: "2".repeat(64),
            class: "SUBSTANTIVE".to_string(),
            scope: "LINE_A".to_string(),
            effective_from: "2026-10-01T00:00:00Z".to_string(),
            effective_until: "9999-12-31T23:59:59Z".to_string(),
            semantic_change: true,
            receipt_id: "receipt-scale".to_string(),
        });
        let report = analyze(&manifest).unwrap();
        assert_eq!(report.union_affected_items.len(), 10_000);
        assert_eq!(report.union_affected_claims, vec!["claim-scale".to_string()]);
        assert_eq!(report.change_impacts[0].affected_claims[0].dependency_path_count, 1);
    }

    #[test]
    fn global_change_impacts_global_claim_scope() {
        let input = FIXTURE.replace(
            "CLAIM|claim-b|r1|LINE_B|",
            "CLAIM|claim-b|r1|GLOBAL|",
        );
        let report = parse_and_analyze(&input).unwrap();
        assert!(report.union_affected_claims.contains(&"claim-b".to_string()));
    }
}
