//! Executable, fail-closed manufacturing assurance-case graph reference.
//!
//! The checker validates structure and declared policy consistency for a
//! versioned assurance case: claims, supporting claims/evidence, assumptions,
//! defeaters, evidence maturity, temporal validity, scope, and intervention
//! authority. It does NOT prove that evidence is true or semantically relevant.
//!
//! Build and test:
//!   rustc --edition=2021 -D warnings --test tools/manufacturing-assurance/assurance_case.rs -o /tmp/mfg-assurance-tests
//!   /tmp/mfg-assurance-tests
//!
//! Build and run:
//!   rustc --edition=2021 -D warnings tools/manufacturing-assurance/assurance_case.rs -o /tmp/mfg-assurance
//!   /tmp/mfg-assurance tools/manufacturing-assurance/fixtures/valid-assurance-case.tsv

use std::collections::{BTreeMap, BTreeSet};
#[cfg(not(test))]
use std::env;
#[cfg(not(test))]
use std::fs;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Case {
    id: String,
    scope: String,
    as_of: String,
    required_tier: u8,
    top_claim: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Claim {
    id: String,
    scope: String,
    valid_from: String,
    valid_until: String,
    minimum_tier: u8,
    strategy: String,
    kind: String,
    action: String,
    statement: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Evidence {
    id: String,
    scope: String,
    valid_from: String,
    valid_until: String,
    tier: u8,
    status: String,
    digest: String,
    statement: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Assumption {
    id: String,
    scope: String,
    valid_from: String,
    valid_until: String,
    status: String,
    statement: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Defeater {
    id: String,
    scope: String,
    valid_from: String,
    valid_until: String,
    statement: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Authority {
    id: String,
    scope: String,
    valid_from: String,
    valid_until: String,
    status: String,
    principal: String,
    action_scope: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Edge {
    source: String,
    target: String,
    relation: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Manifest {
    case_record: Option<Case>,
    claims: BTreeMap<String, Claim>,
    evidence: BTreeMap<String, Evidence>,
    assumptions: BTreeMap<String, Assumption>,
    defeaters: BTreeMap<String, Defeater>,
    authorities: BTreeMap<String, Authority>,
    edges: Vec<Edge>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Evaluation {
    status: String,
    achieved_tier: Option<u8>,
    reasons: Vec<String>,
    assumptions: Vec<String>,
    witness: Vec<String>,
}

impl Evaluation {
    fn blocked(reason: &str) -> Self {
        Self {
            status: "BLOCKED".to_string(),
            achieved_tier: None,
            reasons: vec![reason.to_string()],
            assumptions: Vec::new(),
            witness: Vec::new(),
        }
    }

    fn defeated(reason: &str) -> Self {
        Self {
            status: "DEFEATED".to_string(),
            achieved_tier: None,
            reasons: vec![reason.to_string()],
            assumptions: Vec::new(),
            witness: Vec::new(),
        }
    }

    fn is_supported(&self) -> bool {
        self.status == "QUALIFIED" || self.status == "CONDITIONAL"
    }
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

fn tier_name(value: Option<u8>) -> String {
    match value {
        Some(0) => "T0".to_string(),
        Some(1) => "T1".to_string(),
        Some(2) => "T2".to_string(),
        Some(3) => "T3".to_string(),
        Some(4) => "T4".to_string(),
        _ => "NONE".to_string(),
    }
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

fn valid_interval(start: &str, end: &str, label: &str) -> Result<(), String> {
    if !valid_utc(start) || !valid_utc(end) || start >= end {
        return Err(format!("{label}: invalid UTC validity interval"));
    }
    Ok(())
}

fn active_at(start: &str, end: &str, as_of: &str) -> bool {
    start <= as_of && as_of < end
}

fn scope_compatible(source_scope: &str, target_scope: &str) -> bool {
    if target_scope == "GLOBAL" {
        source_scope == "GLOBAL"
    } else {
        source_scope == "GLOBAL" || source_scope == target_scope
    }
}

impl Manifest {
    fn has_id(&self, id: &str) -> bool {
        self.claims.contains_key(id)
            || self.evidence.contains_key(id)
            || self.assumptions.contains_key(id)
            || self.defeaters.contains_key(id)
            || self.authorities.contains_key(id)
    }

    fn scope_of(&self, id: &str) -> Option<&str> {
        self.claims
            .get(id)
            .map(|node| node.scope.as_str())
            .or_else(|| self.evidence.get(id).map(|node| node.scope.as_str()))
            .or_else(|| self.assumptions.get(id).map(|node| node.scope.as_str()))
            .or_else(|| self.defeaters.get(id).map(|node| node.scope.as_str()))
            .or_else(|| self.authorities.get(id).map(|node| node.scope.as_str()))
    }

    fn insert_id(&self, id: &str) -> Result<(), String> {
        if self.has_id(id) {
            Err(format!("duplicate graph identity {id}"))
        } else {
            Ok(())
        }
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
                    scope: nonempty(fields[2], "CASE scope")?,
                    as_of: nonempty(fields[3], "CASE as_of")?,
                    required_tier: parse_tier(fields[4], "CASE required_tier")?,
                    top_claim: nonempty(fields[5], "CASE top_claim")?,
                };
                if !valid_utc(&case.as_of) {
                    return Err(format!("line {line_number}: CASE as_of is not canonical UTC"));
                }
                manifest.case_record = Some(case);
            }
            "CLAIM" => {
                if fields.len() != 10 {
                    return Err(format!("line {line_number}: CLAIM requires 9 fields"));
                }
                let id = nonempty(fields[1], "CLAIM id")?;
                manifest.insert_id(&id)?;
                let claim = Claim {
                    id: id.clone(),
                    scope: nonempty(fields[2], "CLAIM scope")?,
                    valid_from: nonempty(fields[3], "CLAIM valid_from")?,
                    valid_until: nonempty(fields[4], "CLAIM valid_until")?,
                    minimum_tier: parse_tier(fields[5], "CLAIM minimum_tier")?,
                    strategy: nonempty(fields[6], "CLAIM strategy")?,
                    kind: nonempty(fields[7], "CLAIM kind")?,
                    action: nonempty(fields[8], "CLAIM action")?,
                    statement: nonempty(fields[9], "CLAIM statement")?,
                };
                valid_interval(&claim.valid_from, &claim.valid_until, "CLAIM validity")?;
                if !matches!(claim.strategy.as_str(), "ALL" | "ANY") {
                    return Err(format!("line {line_number}: unsupported claim strategy"));
                }
                if !matches!(
                    claim.kind.as_str(),
                    "GENERAL"
                        | "MODEL_VALIDITY"
                        | "COMPOSITION_VALIDITY"
                        | "INTERVENTION_AUTHORIZATION"
                        | "INTERVENTION_OUTCOME"
                        | "CAUSAL_BENEFIT"
                ) {
                    return Err(format!("line {line_number}: unsupported claim kind"));
                }
                if claim.kind == "INTERVENTION_AUTHORIZATION" {
                    if claim.action == "-" {
                        return Err(format!("line {line_number}: intervention claim requires an action key"));
                    }
                } else if claim.action != "-" {
                    return Err(format!("line {line_number}: action key only applies to intervention claims"));
                }
                manifest.claims.insert(id, claim);
            }
            "EVIDENCE" => {
                if fields.len() != 9 {
                    return Err(format!("line {line_number}: EVIDENCE requires 8 fields"));
                }
                let id = nonempty(fields[1], "EVIDENCE id")?;
                manifest.insert_id(&id)?;
                let evidence = Evidence {
                    id: id.clone(),
                    scope: nonempty(fields[2], "EVIDENCE scope")?,
                    valid_from: nonempty(fields[3], "EVIDENCE valid_from")?,
                    valid_until: nonempty(fields[4], "EVIDENCE valid_until")?,
                    tier: parse_tier(fields[5], "EVIDENCE tier")?,
                    status: nonempty(fields[6], "EVIDENCE status")?,
                    digest: nonempty(fields[7], "EVIDENCE digest")?,
                    statement: nonempty(fields[8], "EVIDENCE statement")?,
                };
                valid_interval(&evidence.valid_from, &evidence.valid_until, "EVIDENCE validity")?;
                if !matches!(evidence.status.as_str(), "VERIFIED" | "UNVERIFIED" | "REVOKED" | "CONTESTED") {
                    return Err(format!("line {line_number}: unsupported evidence status"));
                }
                if !is_sha256(&evidence.digest) {
                    return Err(format!("line {line_number}: evidence {} has malformed SHA-256", id));
                }
                manifest.evidence.insert(id, evidence);
            }
            "ASSUMPTION" => {
                if fields.len() != 7 {
                    return Err(format!("line {line_number}: ASSUMPTION requires 6 fields"));
                }
                let id = nonempty(fields[1], "ASSUMPTION id")?;
                manifest.insert_id(&id)?;
                let assumption = Assumption {
                    id: id.clone(),
                    scope: nonempty(fields[2], "ASSUMPTION scope")?,
                    valid_from: nonempty(fields[3], "ASSUMPTION valid_from")?,
                    valid_until: nonempty(fields[4], "ASSUMPTION valid_until")?,
                    status: nonempty(fields[5], "ASSUMPTION status")?,
                    statement: nonempty(fields[6], "ASSUMPTION statement")?,
                };
                valid_interval(&assumption.valid_from, &assumption.valid_until, "ASSUMPTION validity")?;
                if !matches!(assumption.status.as_str(), "ACCEPTED" | "OPEN" | "REJECTED") {
                    return Err(format!("line {line_number}: unsupported assumption status"));
                }
                manifest.assumptions.insert(id, assumption);
            }
            "DEFEATER" => {
                if fields.len() != 6 {
                    return Err(format!("line {line_number}: DEFEATER requires 5 fields"));
                }
                let id = nonempty(fields[1], "DEFEATER id")?;
                manifest.insert_id(&id)?;
                let defeater = Defeater {
                    id: id.clone(),
                    scope: nonempty(fields[2], "DEFEATER scope")?,
                    valid_from: nonempty(fields[3], "DEFEATER valid_from")?,
                    valid_until: nonempty(fields[4], "DEFEATER valid_until")?,
                    statement: nonempty(fields[5], "DEFEATER statement")?,
                };
                valid_interval(&defeater.valid_from, &defeater.valid_until, "DEFEATER validity")?;
                manifest.defeaters.insert(id, defeater);
            }
            "AUTHORITY" => {
                if fields.len() != 8 {
                    return Err(format!("line {line_number}: AUTHORITY requires 7 fields"));
                }
                let id = nonempty(fields[1], "AUTHORITY id")?;
                manifest.insert_id(&id)?;
                let authority = Authority {
                    id: id.clone(),
                    scope: nonempty(fields[2], "AUTHORITY scope")?,
                    valid_from: nonempty(fields[3], "AUTHORITY valid_from")?,
                    valid_until: nonempty(fields[4], "AUTHORITY valid_until")?,
                    status: nonempty(fields[5], "AUTHORITY status")?,
                    principal: nonempty(fields[6], "AUTHORITY principal")?,
                    action_scope: nonempty(fields[7], "AUTHORITY action_scope")?,
                };
                valid_interval(&authority.valid_from, &authority.valid_until, "AUTHORITY validity")?;
                if !matches!(authority.status.as_str(), "GRANTED" | "REVOKED" | "SUSPENDED") {
                    return Err(format!("line {line_number}: unsupported authority status"));
                }
                manifest.authorities.insert(id, authority);
            }
            "EDGE" => {
                if fields.len() != 4 {
                    return Err(format!("line {line_number}: EDGE requires 3 fields"));
                }
                manifest.edges.push(Edge {
                    source: nonempty(fields[1], "EDGE source")?,
                    target: nonempty(fields[2], "EDGE target")?,
                    relation: nonempty(fields[3], "EDGE relation")?,
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
    if manifest.claims.is_empty() {
        return Err("manifest requires at least one CLAIM".to_string());
    }
    let top = manifest
        .claims
        .get(&case.top_claim)
        .ok_or_else(|| format!("CASE references unknown top claim {}", case.top_claim))?;
    if !scope_compatible(&top.scope, &case.scope) {
        return Err("top claim scope does not cover CASE scope".to_string());
    }

    let mut seen_edges = BTreeSet::new();
    let mut support_count: BTreeMap<String, usize> =
        manifest.claims.keys().map(|id| (id.clone(), 0)).collect();
    let mut support_adjacency: BTreeMap<String, Vec<String>> =
        manifest.claims.keys().map(|id| (id.clone(), Vec::new())).collect();

    for edge in &manifest.edges {
        if !manifest.has_id(&edge.source) || !manifest.has_id(&edge.target) {
            return Err(format!(
                "edge {} -> {} references an unknown node",
                edge.source, edge.target
            ));
        }
        if edge.source == edge.target {
            return Err(format!("self-referential edge at {}", edge.source));
        }
        let key = format!("{}|{}|{}", edge.source, edge.target, edge.relation);
        if !seen_edges.insert(key) {
            return Err(format!("duplicate edge {} -> {}", edge.source, edge.target));
        }

        let source_is_claim = manifest.claims.contains_key(&edge.source);
        let source_is_evidence = manifest.evidence.contains_key(&edge.source);
        let source_is_assumption = manifest.assumptions.contains_key(&edge.source);
        let source_is_defeater = manifest.defeaters.contains_key(&edge.source);
        let source_is_authority = manifest.authorities.contains_key(&edge.source);
        let target_is_claim = manifest.claims.contains_key(&edge.target);
        let target_is_defeater = manifest.defeaters.contains_key(&edge.target);

        let compatible = match edge.relation.as_str() {
            "SUPPORTS" => {
                target_is_claim && (source_is_claim || source_is_evidence)
            }
            "COUNTERS" => target_is_claim && source_is_evidence,
            "ASSUMES" => target_is_claim && source_is_assumption,
            "DEFEATS" => target_is_claim && source_is_defeater,
            "DISCHARGES" => target_is_defeater && source_is_evidence,
            "AUTHORIZES" => {
                target_is_claim
                    && source_is_authority
                    && manifest.claims.get(&edge.target).map(|claim| {
                        claim.kind == "INTERVENTION_AUTHORIZATION"
                    }).unwrap_or(false)
            }
            _ => return Err(format!("unknown edge relation {}", edge.relation)),
        };
        if !compatible {
            return Err(format!(
                "edge relation {} has incompatible endpoints {} -> {}",
                edge.relation, edge.source, edge.target
            ));
        }

        let source_scope = manifest.scope_of(&edge.source).expect("known source has scope");
        let target_scope = manifest.scope_of(&edge.target).expect("known target has scope");
        if !scope_compatible(source_scope, target_scope) {
            return Err(format!(
                "scope mismatch on {} edge {} -> {}",
                edge.relation, edge.source, edge.target
            ));
        }

        if edge.relation == "SUPPORTS" {
            *support_count.get_mut(&edge.target).expect("claim target exists") += 1;
            if source_is_claim {
                support_adjacency
                    .get_mut(&edge.source)
                    .expect("claim source exists")
                    .push(edge.target.clone());
            }
        }
    }

    for claim in manifest.claims.values() {
        if support_count.get(&claim.id).copied().unwrap_or(0) == 0 {
            return Err(format!("claim {} has no supporting claim or evidence", claim.id));
        }
    }

    // A support edge from child claim to parent claim creates a directed
    // dependency child -> parent. Kahn's algorithm rejects support cycles.
    let mut indegree: BTreeMap<String, usize> =
        manifest.claims.keys().map(|id| (id.clone(), 0)).collect();
    for children in support_adjacency.values() {
        for parent in children {
            *indegree.get_mut(parent).expect("parent claim exists") += 1;
        }
    }
    let mut ready: BTreeSet<String> = indegree
        .iter()
        .filter_map(|(id, degree)| if *degree == 0 { Some(id.clone()) } else { None })
        .collect();
    let mut visited = 0_usize;
    while let Some(id) = ready.pop_first() {
        visited += 1;
        if let Some(parents) = support_adjacency.get(&id) {
            for parent in parents {
                let degree = indegree.get_mut(parent).expect("parent claim exists");
                *degree -= 1;
                if *degree == 0 {
                    ready.insert(parent.clone());
                }
            }
        }
    }
    if visited != manifest.claims.len() {
        return Err("claim support graph contains a cycle".to_string());
    }

    Ok(())
}

fn evidence_is_current_and_usable(
    evidence: &Evidence,
    as_of: &str,
    scope: &str,
    minimum_tier: u8,
) -> Result<u8, String> {
    if !active_at(&evidence.valid_from, &evidence.valid_until, as_of) {
        return Err(format!("EVIDENCE_NOT_CURRENT:{}", evidence.id));
    }
    if !scope_compatible(&evidence.scope, scope) {
        return Err(format!("EVIDENCE_SCOPE_MISMATCH:{}", evidence.id));
    }
    if evidence.status != "VERIFIED" {
        return Err(format!("EVIDENCE_NOT_VERIFIED:{}:{}", evidence.id, evidence.status));
    }
    if evidence.tier < minimum_tier {
        return Err(format!("EVIDENCE_TIER_TOO_LOW:{}:{}", evidence.id, tier_name(Some(evidence.tier))));
    }
    Ok(evidence.tier)
}

fn evaluate_claim(
    id: &str,
    manifest: &Manifest,
    memo: &mut BTreeMap<String, Evaluation>,
    stack: &mut BTreeSet<String>,
) -> Evaluation {
    if let Some(existing) = memo.get(id) {
        return existing.clone();
    }
    if !stack.insert(id.to_string()) {
        return Evaluation::blocked("SUPPORT_CYCLE");
    }

    let claim = match manifest.claims.get(id) {
        Some(value) => value.clone(),
        None => {
            stack.remove(id);
            return Evaluation::blocked("UNKNOWN_CLAIM");
        }
    };
    let case = manifest.case_record.as_ref().expect("validated manifest has CASE");

    if !active_at(&claim.valid_from, &claim.valid_until, &case.as_of) {
        let result = Evaluation::blocked("CLAIM_NOT_CURRENT");
        stack.remove(id);
        memo.insert(id.to_string(), result.clone());
        return result;
    }

    let mut reasons = Vec::new();
    let mut assumptions = Vec::new();
    let mut witness = Vec::new();
    let mut support_tiers = Vec::new();
    let support_edges: Vec<&Edge> = manifest
        .edges
        .iter()
        .filter(|edge| edge.target == id && edge.relation == "SUPPORTS")
        .collect();

    for edge in &support_edges {
        if let Some(evidence) = manifest.evidence.get(&edge.source) {
            match evidence_is_current_and_usable(
                evidence,
                &case.as_of,
                &claim.scope,
                claim.minimum_tier,
            ) {
                Ok(tier) => {
                    support_tiers.push(tier);
                    witness.push(evidence.id.clone());
                }
                Err(reason) => reasons.push(reason),
            }
        } else if manifest.claims.contains_key(&edge.source) {
            let child = evaluate_claim(&edge.source, manifest, memo, stack);
            if child.is_supported() {
                if let Some(tier) = child.achieved_tier {
                    if tier >= claim.minimum_tier {
                        support_tiers.push(tier);
                        witness.push(edge.source.clone());
                        witness.extend(child.witness);
                    } else {
                        reasons.push(format!(
                            "SUBCLAIM_TIER_TOO_LOW:{}:{}",
                            edge.source,
                            tier_name(Some(tier))
                        ));
                    }
                } else {
                    reasons.push(format!("SUBCLAIM_HAS_NO_TIER:{}", edge.source));
                }
                if child.status == "CONDITIONAL" {
                    assumptions.extend(child.assumptions);
                }
            } else {
                reasons.push(format!("SUBCLAIM_{}:{}", child.status, edge.source));
                reasons.extend(child.reasons);
            }
        }
    }

    let support_satisfied = match claim.strategy.as_str() {
        "ALL" => support_tiers.len() == support_edges.len() && !support_edges.is_empty(),
        "ANY" => !support_tiers.is_empty(),
        _ => false,
    };
    let achieved_tier = match claim.strategy.as_str() {
        "ALL" => support_tiers.iter().copied().min(),
        "ANY" => support_tiers.iter().copied().max(),
        _ => None,
    };

    let mut defeated = false;
    let mut blocked = !support_satisfied;
    if !support_satisfied && reasons.is_empty() {
        reasons.push("SUPPORT_REQUIREMENT_NOT_MET".to_string());
    }

    // Counter-evidence is distinct from absence of support. Current, verified
    // counter-evidence defeats the claim; unresolved counter-evidence blocks it.
    for edge in manifest.edges.iter().filter(|edge| edge.target == id && edge.relation == "COUNTERS") {
        if let Some(evidence) = manifest.evidence.get(&edge.source) {
            if active_at(&evidence.valid_from, &evidence.valid_until, &case.as_of) {
                if evidence.status == "VERIFIED" && evidence.tier >= claim.minimum_tier {
                    defeated = true;
                    reasons.push(format!("COUNTER_EVIDENCE:{}", evidence.id));
                } else if evidence.status == "VERIFIED" {
                    blocked = true;
                    reasons.push(format!("COUNTER_EVIDENCE_TIER_TOO_LOW:{}", evidence.id));
                } else {
                    blocked = true;
                    reasons.push(format!("COUNTER_EVIDENCE_UNRESOLVED:{}", evidence.id));
                }
            }
        }
    }

    for edge in manifest.edges.iter().filter(|edge| edge.target == id && edge.relation == "ASSUMES") {
        if let Some(assumption) = manifest.assumptions.get(&edge.source) {
            if !active_at(&assumption.valid_from, &assumption.valid_until, &case.as_of) {
                blocked = true;
                reasons.push(format!("ASSUMPTION_NOT_CURRENT:{}", assumption.id));
            } else {
                match assumption.status.as_str() {
                    "ACCEPTED" => assumptions.push(assumption.id.clone()),
                    "OPEN" => {
                        blocked = true;
                        reasons.push(format!("ASSUMPTION_OPEN:{}", assumption.id));
                    }
                    "REJECTED" => {
                        defeated = true;
                        reasons.push(format!("ASSUMPTION_REJECTED:{}", assumption.id));
                    }
                    _ => {
                        blocked = true;
                        reasons.push(format!("ASSUMPTION_INVALID:{}", assumption.id));
                    }
                }
            }
        }
    }

    // Each active defeater blocks until a current, verified, sufficiently
    // mature item explicitly discharges it. The checker does not infer closure.
    for edge in manifest.edges.iter().filter(|edge| edge.target == id && edge.relation == "DEFEATS") {
        if let Some(defeater) = manifest.defeaters.get(&edge.source) {
            if active_at(&defeater.valid_from, &defeater.valid_until, &case.as_of) {
                let discharged = manifest
                    .edges
                    .iter()
                    .filter(|discharge| {
                        discharge.target == defeater.id && discharge.relation == "DISCHARGES"
                    })
                    .filter_map(|discharge| manifest.evidence.get(&discharge.source))
                    .any(|evidence| {
                        evidence_is_current_and_usable(
                            evidence,
                            &case.as_of,
                            &claim.scope,
                            claim.minimum_tier,
                        )
                        .is_ok()
                    });
                if !discharged {
                    blocked = true;
                    reasons.push(format!("UNRESOLVED_DEFEATER:{}", defeater.id));
                }
            }
        }
    }

    // Intervention capability is not authority. An intervention claim needs
    // a current grant for the exact action (or a deliberately broad wildcard).
    if claim.kind == "INTERVENTION_AUTHORIZATION" {
        let mut authorized = false;
        for edge in manifest.edges.iter().filter(|edge| edge.target == id && edge.relation == "AUTHORIZES") {
            if let Some(authority) = manifest.authorities.get(&edge.source) {
                if active_at(&authority.valid_from, &authority.valid_until, &case.as_of)
                    && authority.status == "GRANTED"
                    && (authority.action_scope == "*" || authority.action_scope == claim.action)
                    && scope_compatible(&authority.scope, &claim.scope)
                {
                    authorized = true;
                    witness.push(format!("AUTHORITY:{}", authority.id));
                }
            }
        }
        if !authorized {
            blocked = true;
            reasons.push(format!("NO_CURRENT_AUTHORITY_FOR:{}", claim.action));
        }
    }

    let status = if defeated {
        "DEFEATED"
    } else if blocked {
        "BLOCKED"
    } else if !assumptions.is_empty() {
        "CONDITIONAL"
    } else {
        "QUALIFIED"
    };

    reasons.sort();
    reasons.dedup();
    assumptions.sort();
    assumptions.dedup();
    witness.sort();
    witness.dedup();

    let result = Evaluation {
        status: status.to_string(),
        achieved_tier,
        reasons,
        assumptions,
        witness,
    };
    stack.remove(id);
    memo.insert(id.to_string(), result.clone());
    result
}

fn run_report(input: &str) -> Result<String, String> {
    let manifest = parse_manifest(input)?;
    let case = manifest.case_record.as_ref().expect("validated manifest has CASE");
    let mut memo = BTreeMap::new();
    let mut stack = BTreeSet::new();

    for claim_id in manifest.claims.keys() {
        let _ = evaluate_claim(claim_id, &manifest, &mut memo, &mut stack);
    }

    let mut top = memo
        .get(&case.top_claim)
        .cloned()
        .unwrap_or_else(|| Evaluation::blocked("TOP_CLAIM_NOT_EVALUATED"));
    if top.is_supported()
        && top.achieved_tier.map(|tier| tier < case.required_tier).unwrap_or(true)
    {
        top.status = "BLOCKED".to_string();
        top.reasons.push(format!(
            "CASE_TIER_BELOW_REQUIRED:required={}:achieved={}",
            tier_name(Some(case.required_tier)),
            tier_name(top.achieved_tier)
        ));
        top.reasons.sort();
        top.reasons.dedup();
        memo.insert(case.top_claim.clone(), top.clone());
    }

    let mut output = format!(
        "MFG_ASSURANCE_CASE version=1 case={} scope={} as_of={} required_tier={} top_claim={}\n",
        case.id,
        case.scope,
        case.as_of,
        tier_name(Some(case.required_tier)),
        case.top_claim
    );

    output.push_str(&format!(
        "CASE_RESULT status={} achieved_tier={} assumptions={} reasons={}\n",
        top.status,
        tier_name(top.achieved_tier),
        if top.assumptions.is_empty() {
            "-".to_string()
        } else {
            top.assumptions.join(",")
        },
        if top.reasons.is_empty() {
            "-".to_string()
        } else {
            top.reasons.join(",")
        }
    ));

    for (claim_id, claim) in &manifest.claims {
        let result = memo.get(claim_id).expect("all claims evaluated");
        output.push_str(&format!(
            "CLAIM id={} kind={} scope={} strategy={} status={} achieved_tier={} witness={} reasons={}\n",
            claim.id,
            claim.kind,
            claim.scope,
            claim.strategy,
            result.status,
            tier_name(result.achieved_tier),
            if result.witness.is_empty() {
                "-".to_string()
            } else {
                result.witness.join(",")
            },
            if result.reasons.is_empty() {
                "-".to_string()
            } else {
                result.reasons.join(",")
            }
        ));
        output.push_str(&format!("NODE kind=CLAIM id={} statement={}\n", claim.id, claim.statement));
    }
    for evidence in manifest.evidence.values() {
        output.push_str(&format!(
            "NODE kind=EVIDENCE id={} status={} tier={} digest={} statement={}\n",
            evidence.id, evidence.status, tier_name(Some(evidence.tier)), evidence.digest, evidence.statement
        ));
    }
    for assumption in manifest.assumptions.values() {
        output.push_str(&format!(
            "NODE kind=ASSUMPTION id={} status={} statement={}\n",
            assumption.id, assumption.status, assumption.statement
        ));
    }
    for defeater in manifest.defeaters.values() {
        output.push_str(&format!(
            "NODE kind=DEFEATER id={} statement={}\n",
            defeater.id, defeater.statement
        ));
    }
    for authority in manifest.authorities.values() {
        output.push_str(&format!(
            "NODE kind=AUTHORITY id={} status={} principal={} action_scope={}\n",
            authority.id, authority.status, authority.principal, authority.action_scope
        ));
    }

    output.push_str(
        "CLAIM_CEILING graph_structure_and_declared_policy_only premise_truth_proven=false evidence_bytes_authenticated=false semantic_relevance_proven=false intervention_safety_proven=false standards_conformance_claimed=false\n",
    );
    Ok(output)
}

#[cfg(not(test))]
fn main() {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| "tools/manufacturing-assurance/fixtures/valid-assurance-case.tsv".to_string());
    match fs::read_to_string(&path)
        .map_err(|error| format!("unable to read {path}: {error}"))
        .and_then(|input| run_report(&input))
    {
        Ok(report) => print!("{report}"),
        Err(error) => {
            eprintln!("mfg-assurance-case: {error}");
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

    fn valid_manifest() -> String {
        format!(
            "CASE|case-1|LINE-1|2026-10-31T12:00:00Z|T0|top\n\
             CLAIM|top|LINE-1|2026-10-01T00:00:00Z|2026-11-01T00:00:00Z|T0|ALL|COMPOSITION_VALIDITY|-|Composed twin evidence is admissible\n\
             CLAIM|physics|LINE-1|2026-10-01T00:00:00Z|2026-11-01T00:00:00Z|T0|ALL|MODEL_VALIDITY|-|Physics model is in its valid domain\n\
             CLAIM|intervention|LINE-1|2026-10-01T00:00:00Z|2026-11-01T00:00:00Z|T0|ALL|INTERVENTION_AUTHORIZATION|adjust_feed_rate|Feed-rate action is authorized\n\
             EVIDENCE|e-physics|LINE-1|2026-10-01T00:00:00Z|2026-11-01T00:00:00Z|T0|VERIFIED|{}|Synthetic physics reference replay\n\
             EVIDENCE|e-plan|LINE-1|2026-10-01T00:00:00Z|2026-11-01T00:00:00Z|T0|VERIFIED|{}|Bounded action envelope evidence\n\
             EVIDENCE|e-discharge|LINE-1|2026-10-01T00:00:00Z|2026-11-01T00:00:00Z|T0|VERIFIED|{}|Independent thermal interlock evidence\n\
             ASSUMPTION|a-boundary|LINE-1|2026-10-01T00:00:00Z|2026-11-01T00:00:00Z|ACCEPTED|Ambient boundary remains within declared envelope\n\
             DEFEATER|d-overheat|LINE-1|2026-10-01T00:00:00Z|2026-11-01T00:00:00Z|Possible overheating under loss of cooling\n\
             AUTHORITY|auth-1|LINE-1|2026-10-01T00:00:00Z|2026-11-01T00:00:00Z|GRANTED|operator-1|adjust_feed_rate\n\
             EDGE|e-physics|physics|SUPPORTS\n\
             EDGE|physics|top|SUPPORTS\n\
             EDGE|e-plan|intervention|SUPPORTS\n\
             EDGE|auth-1|intervention|AUTHORIZES\n\
             EDGE|e-discharge|d-overheat|DISCHARGES\n\
             EDGE|d-overheat|top|DEFEATS\n\
             EDGE|a-boundary|top|ASSUMES\n",
            HASH_A, HASH_B, HASH_C
        )
    }

    fn mutate(input: String, old: &str, new: &str) -> String {
        assert!(input.contains(old), "missing mutation target: {old}");
        input.replacen(old, new, 1)
    }

    #[test]
    fn valid_case_is_conditional_when_assumption_is_accepted() {
        let report = run_report(&valid_manifest()).unwrap();
        assert!(report.contains("CASE_RESULT status=CONDITIONAL achieved_tier=T0"));
        assert!(report.contains("assumptions=a-boundary"));
        assert!(report.contains("CLAIM_CEILING"));
    }

    #[test]
    fn verified_defeater_discharge_allows_claim_to_remain_conditional() {
        let report = run_report(&valid_manifest()).unwrap();
        assert!(!report.contains("UNRESOLVED_DEFEATER"));
    }

    #[test]
    fn unresolved_defeater_blocks_claim() {
        let input = mutate(valid_manifest(), "EDGE|e-discharge|d-overheat|DISCHARGES\n", "");
        let report = run_report(&input).unwrap();
        assert!(report.contains("CASE_RESULT status=BLOCKED"));
        assert!(report.contains("UNRESOLVED_DEFEATER:d-overheat"));
    }

    #[test]
    fn verified_counter_evidence_defeats_claim() {
        let mut input = valid_manifest();
        input.push_str(&format!(
            "EVIDENCE|e-counter|LINE-1|2026-10-01T00:00:00Z|2026-11-01T00:00:00Z|T0|VERIFIED|{}|Counterexample invalidates the composition claim\nEDGE|e-counter|top|COUNTERS\n",
            HASH_C
        ));
        let report = run_report(&input).unwrap();
        assert!(report.contains("CASE_RESULT status=DEFEATED"));
        assert!(report.contains("COUNTER_EVIDENCE:e-counter"));
    }

    #[test]
    fn under_tier_counter_evidence_blocks_but_does_not_defeat() {
        let mut input = valid_manifest();
        input = mutate(
            input,
            "CLAIM|top|LINE-1|2026-10-01T00:00:00Z|2026-11-01T00:00:00Z|T0",
            "CLAIM|top|LINE-1|2026-10-01T00:00:00Z|2026-11-01T00:00:00Z|T2",
        );
        input.push_str(&format!(
            "EVIDENCE|e-counter|LINE-1|2026-10-01T00:00:00Z|2026-11-01T00:00:00Z|T0|VERIFIED|{}|Under-tier counterexample\\nEDGE|e-counter|top|COUNTERS\\n",
            HASH_C
        ));
        let report = run_report(&input).unwrap();
        assert!(report.contains("CASE_RESULT status=BLOCKED"));
        assert!(report.contains("COUNTER_EVIDENCE_TIER_TOO_LOW:e-counter"));
    }

    #[test]
    fn unresolved_counter_evidence_blocks_instead_of_being_ignored() {
        let mut input = valid_manifest();
        input.push_str(&format!(
            "EVIDENCE|e-counter|LINE-1|2026-10-01T00:00:00Z|2026-11-01T00:00:00Z|T0|UNVERIFIED|{}|Unreviewed counterexample\nEDGE|e-counter|top|COUNTERS\n",
            HASH_C
        ));
        let report = run_report(&input).unwrap();
        assert!(report.contains("CASE_RESULT status=BLOCKED"));
        assert!(report.contains("COUNTER_EVIDENCE_UNRESOLVED:e-counter"));
    }

    #[test]
    fn stale_supporting_evidence_blocks_claim() {
        let input = mutate(
            valid_manifest(),
            "EVIDENCE|e-physics|LINE-1|2026-10-01T00:00:00Z|2026-11-01T00:00:00Z",
            "EVIDENCE|e-physics|LINE-1|2026-10-01T00:00:00Z|2026-10-30T00:00:00Z",
        );
        let report = run_report(&input).unwrap();
        assert!(report.contains("CASE_RESULT status=BLOCKED"));
        assert!(report.contains("EVIDENCE_NOT_CURRENT:e-physics"));
    }

    #[test]
    fn local_evidence_cannot_support_global_claim() {
        let input = mutate(
            valid_manifest(),
            "CASE|case-1|LINE-1",
            "CASE|case-1|GLOBAL",
        );
        let input = mutate(input, "CLAIM|top|LINE-1", "CLAIM|top|GLOBAL");
        assert!(parse_manifest(&input)
            .unwrap_err()
            .contains("scope mismatch"));
    }

    #[test]
    fn open_assumption_blocks_claim() {
        let input = mutate(valid_manifest(), "|ACCEPTED|Ambient boundary", "|OPEN|Ambient boundary");
        let report = run_report(&input).unwrap();
        assert!(report.contains("CASE_RESULT status=BLOCKED"));
        assert!(report.contains("ASSUMPTION_OPEN:a-boundary"));
    }

    #[test]
    fn rejected_assumption_defeats_claim() {
        let input = mutate(valid_manifest(), "|ACCEPTED|Ambient boundary", "|REJECTED|Ambient boundary");
        let report = run_report(&input).unwrap();
        assert!(report.contains("CASE_RESULT status=DEFEATED"));
        assert!(report.contains("ASSUMPTION_REJECTED:a-boundary"));
    }

    #[test]
    fn intervention_requires_explicit_matching_authority() {
        let input = mutate(valid_manifest(), "EDGE|auth-1|intervention|AUTHORIZES\n", "");
        let report = run_report(&input).unwrap();
        assert!(report.contains("NO_CURRENT_AUTHORITY_FOR:adjust_feed_rate"));
        assert!(report.contains("CLAIM id=intervention"));
        assert!(report.contains("status=BLOCKED"));
    }

    #[test]
    fn revoked_authority_does_not_authorize_intervention() {
        let input = mutate(valid_manifest(), "|GRANTED|operator-1|", "|REVOKED|operator-1|");
        let report = run_report(&input).unwrap();
        assert!(report.contains("NO_CURRENT_AUTHORITY_FOR:adjust_feed_rate"));
    }

    #[test]
    fn wrong_action_authority_does_not_authorize_intervention() {
        let input = mutate(valid_manifest(), "|GRANTED|operator-1|adjust_feed_rate", "|GRANTED|operator-1|change_tool");
        let report = run_report(&input).unwrap();
        assert!(report.contains("NO_CURRENT_AUTHORITY_FOR:adjust_feed_rate"));
    }

    #[test]
    fn authority_scope_must_cover_the_claim_scope() {
        let input = mutate(valid_manifest(), "AUTHORITY|auth-1|LINE-1", "AUTHORITY|auth-1|LINE-2");
        assert!(parse_manifest(&input).unwrap_err().contains("scope mismatch"));
    }

    #[test]
    fn under_tier_evidence_blocks_claim() {
        let input = mutate(
            valid_manifest(),
            "CLAIM|physics|LINE-1|2026-10-01T00:00:00Z|2026-11-01T00:00:00Z|T0",
            "CLAIM|physics|LINE-1|2026-10-01T00:00:00Z|2026-11-01T00:00:00Z|T2",
        );
        let report = run_report(&input).unwrap();
        assert!(report.contains("EVIDENCE_TIER_TOO_LOW:e-physics:T0"));
    }

    #[test]
    fn case_required_tier_is_enforced() {
        let input = mutate(valid_manifest(), "|T0|top\n", "|T2|top\n");
        let report = run_report(&input).unwrap();
        assert!(report.contains("CASE_TIER_BELOW_REQUIRED:required=T2:achieved=T0"));
        assert!(report.contains("CASE_RESULT status=BLOCKED"));
    }

    #[test]
    fn any_strategy_can_use_a_surviving_valid_support_path() {
        let input = mutate(
            valid_manifest(),
            "CLAIM|top|LINE-1|2026-10-01T00:00:00Z|2026-11-01T00:00:00Z|T0|ALL",
            "CLAIM|top|LINE-1|2026-10-01T00:00:00Z|2026-11-01T00:00:00Z|T0|ANY",
        );
        let input = mutate(
            input,
            "EVIDENCE|e-physics|LINE-1|2026-10-01T00:00:00Z|2026-11-01T00:00:00Z",
            "EVIDENCE|e-physics|LINE-1|2026-10-01T00:00:00Z|2026-10-30T00:00:00Z",
        );
        let report = run_report(&input).unwrap();
        assert!(report.contains("CASE_RESULT status=CONDITIONAL"));
    }

    #[test]
    fn support_cycles_are_rejected() {
        let mut input = valid_manifest();
        input.push_str("EDGE|top|physics|SUPPORTS\n");
        assert!(parse_manifest(&input).unwrap_err().contains("cycle"));
    }

    #[test]
    fn unknown_edge_references_are_rejected() {
        let mut input = valid_manifest();
        input.push_str("EDGE|not-real|top|SUPPORTS\n");
        assert!(parse_manifest(&input).unwrap_err().contains("unknown node"));
    }

    #[test]
    fn duplicate_node_ids_are_rejected() {
        let mut input = valid_manifest();
        input.push_str(&format!(
            "EVIDENCE|e-physics|LINE-1|2026-10-01T00:00:00Z|2026-11-01T00:00:00Z|T0|VERIFIED|{}|Duplicate identity\n",
            HASH_A
        ));
        assert!(parse_manifest(&input).unwrap_err().contains("duplicate graph identity"));
    }

    #[test]
    fn duplicate_edges_are_rejected() {
        let mut input = valid_manifest();
        input.push_str("EDGE|e-physics|physics|SUPPORTS\n");
        assert!(parse_manifest(&input).unwrap_err().contains("duplicate edge"));
    }

    #[test]
    fn malformed_evidence_digest_is_rejected() {
        let input = mutate(valid_manifest(), HASH_A, "not-a-sha");
        assert!(parse_manifest(&input).unwrap_err().contains("malformed SHA-256"));
    }

    #[test]
    fn malformed_calendar_date_is_rejected() {
        let input = mutate(valid_manifest(), "2026-10-31T12:00:00Z", "2026-02-30T12:00:00Z");
        assert!(parse_manifest(&input).unwrap_err().contains("as_of"));
    }

    #[test]
    fn non_verified_supporting_evidence_blocks_claim() {
        let input = mutate(valid_manifest(), "|T0|VERIFIED|", "|T0|CONTESTED|");
        let report = run_report(&input).unwrap();
        assert!(report.contains("EVIDENCE_NOT_VERIFIED:e-physics:CONTESTED"));
        assert!(report.contains("CASE_RESULT status=BLOCKED"));
    }

    #[test]
    fn expired_claim_is_blocked() {
        let input = mutate(
            valid_manifest(),
            "CLAIM|top|LINE-1|2026-10-01T00:00:00Z|2026-11-01T00:00:00Z",
            "CLAIM|top|LINE-1|2026-10-01T00:00:00Z|2026-10-30T00:00:00Z",
        );
        let report = run_report(&input).unwrap();
        assert!(report.contains("CLAIM_NOT_CURRENT"));
    }

    #[test]
    fn all_strategy_requires_every_support_path() {
        let input = mutate(
            valid_manifest(),
            "EVIDENCE|e-plan|LINE-1|2026-10-01T00:00:00Z|2026-11-01T00:00:00Z|T0|VERIFIED|",
            "EVIDENCE|e-plan|LINE-1|2026-10-01T00:00:00Z|2026-11-01T00:00:00Z|T0|UNVERIFIED|",
        );
        let report = run_report(&input).unwrap();
        assert!(report.contains("CASE_RESULT status=BLOCKED"));
        assert!(report.contains("EVIDENCE_NOT_VERIFIED:e-plan:UNVERIFIED"));
    }

    #[test]
    fn deterministic_report_replays_identically() {
        let input = valid_manifest();
        assert_eq!(run_report(&input).unwrap(), run_report(&input).unwrap());
    }

    #[test]
    fn graph_report_does_not_claim_evidence_authentication() {
        let report = run_report(&valid_manifest()).unwrap();
        assert!(report.contains("evidence_bytes_authenticated=false"));
        assert!(report.contains("premise_truth_proven=false"));
    }

    #[test]
    fn unsupported_relation_is_rejected() {
        let mut input = valid_manifest();
        input.push_str("EDGE|e-physics|top|MAGICALLY_PROVES\n");
        assert!(parse_manifest(&input).unwrap_err().contains("unknown edge relation"));
    }
}
