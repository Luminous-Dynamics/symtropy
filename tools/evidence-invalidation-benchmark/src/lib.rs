use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt::Write as _;

pub const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: String,
    pub claim_ceiling: String,
    pub artifacts: Vec<EvidenceArtifact>,
    pub scenarios: Vec<Scenario>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EvidenceArtifact {
    pub id: String,
    pub kind: EvidenceKind,
    pub revision: String,
    pub validity: EvidenceStatus,
    pub claim_scope: ClaimScope,
    pub coverage: CoverageStatus,
    pub source_content: String,
    pub source_sha256: String,
    pub depends_on: Vec<Dependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ClaimScope {
    pub claim_id: String,
    pub scope_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Dependency {
    pub artifact_id: String,
    pub invalidates_on: Vec<Trigger>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub id: String,
    pub events: Vec<ChangeEvent>,
    pub claim_uses: Vec<ClaimUse>,
    pub expected_stale: Vec<String>,
    pub expected_superseded: Vec<String>,
    pub expected_claims: Vec<ExpectedClaim>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ChangeEvent {
    pub id: String,
    pub trigger: Trigger,
    pub changed_artifact_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ClaimUse {
    pub artifact_id: String,
    pub claim_id: String,
    pub scope_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExpectedClaim {
    pub artifact_id: String,
    pub claim_id: String,
    pub scope_id: String,
    pub disposition: ClaimDisposition,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    Terrain,
    Survey,
    DesignRevision,
    MaterialBatch,
    Commissioning,
    Furnishing,
    Certificate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStatus {
    Valid,
    Unknown,
    Stale,
    Revoked,
    IncompleteCoverage,
    Contradictory,
    OutOfScope,
    Superseded,
    InvalidDigest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum CoverageStatus {
    Complete,
    Partial,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Trigger {
    TerrainChanged,
    SurveyProtocolChanged,
    DesignRevisionChanged,
    SupplierSubstitution,
    MaterialBatchRevoked,
    CommissioningChanged,
    FurnishingSubstituted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ClaimDisposition {
    Accepted,
    RejectedStatus,
    InvalidDigest,
    OutOfScope,
    IncompleteCoverage,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Report {
    pub tool_version: String,
    pub schema_version: String,
    pub source_revision: String,
    pub manifest_sha256: String,
    pub claim_ceiling: String,
    pub artifact_count: usize,
    pub scenario_count: usize,
    pub all_source_digests_valid: bool,
    pub scenarios: Vec<ScenarioResult>,
    pub overall_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScenarioResult {
    pub scenario_id: String,
    pub event_count_received: usize,
    pub unique_event_count: usize,
    pub superseded_artifacts: Vec<String>,
    pub stale_artifacts: Vec<String>,
    pub oracle_agreement: bool,
    pub fixture_expectations_match: bool,
    pub claims: Vec<ClaimResult>,
    pub artifacts: Vec<ArtifactResult>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClaimResult {
    pub artifact_id: String,
    pub claim_id: String,
    pub scope_id: String,
    pub disposition: ClaimDisposition,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactResult {
    pub id: String,
    pub kind: EvidenceKind,
    pub revision: String,
    pub original_validity: EvidenceStatus,
    pub current_status: EvidenceStatus,
    pub coverage: CoverageStatus,
    pub digest_valid: bool,
    pub dependency_count: usize,
}

/// Evaluate a raw manifest. The digest is over the exact input bytes, not a
/// re-serialized JSON object. The caller should pass the exact source commit.
pub fn evaluate(input: &[u8], source_revision: &str) -> Result<Report, String> {
    let manifest: Manifest = serde_json::from_slice(input)
        .map_err(|error| format!("manifest JSON rejected: {error}"))?;
    validate_manifest(&manifest)?;

    let digest_valid: BTreeMap<String, bool> = manifest
        .artifacts
        .iter()
        .map(|artifact| {
            (
                artifact.id.clone(),
                sha256_hex(artifact.source_content.as_bytes()) == artifact.source_sha256,
            )
        })
        .collect();

    let all_source_digests_valid = digest_valid.values().all(|valid| *valid);
    let manifest_sha256 = sha256_hex(input);
    let mut scenarios: Vec<&Scenario> = manifest.scenarios.iter().collect();
    scenarios.sort_by(|left, right| left.id.cmp(&right.id));

    let mut results = Vec::with_capacity(scenarios.len());
    for scenario in scenarios {
        let events = deduplicate_events(&scenario.events)?;
        let (fast_superseded, fast_stale) =
            invalidation_production(&manifest, &events, &digest_valid);
        let (oracle_superseded, oracle_stale) =
            invalidation_oracle(&manifest, &events, &digest_valid);
        let oracle_agreement = fast_superseded == oracle_superseded && fast_stale == oracle_stale;

        let artifacts_by_id: BTreeMap<&str, &EvidenceArtifact> = manifest
            .artifacts
            .iter()
            .map(|artifact| (artifact.id.as_str(), artifact))
            .collect();

        let mut artifact_results: Vec<ArtifactResult> = manifest
            .artifacts
            .iter()
            .map(|artifact| {
                let valid_digest = digest_valid.get(&artifact.id).copied().unwrap_or(false);
                ArtifactResult {
                    id: artifact.id.clone(),
                    kind: artifact.kind.clone(),
                    revision: artifact.revision.clone(),
                    original_validity: artifact.validity.clone(),
                    current_status: final_status(
                        artifact,
                        valid_digest,
                        &fast_superseded,
                        &fast_stale,
                    ),
                    coverage: artifact.coverage.clone(),
                    digest_valid: valid_digest,
                    dependency_count: artifact.depends_on.len(),
                }
            })
            .collect();
        artifact_results.sort_by(|left, right| left.id.cmp(&right.id));

        let mut claims: Vec<ClaimResult> = scenario
            .claim_uses
            .iter()
            .map(|claim_use| {
                let artifact = artifacts_by_id[claim_use.artifact_id.as_str()];
                let valid_digest = digest_valid.get(&artifact.id).copied().unwrap_or(false);
                let status = final_status(artifact, valid_digest, &fast_superseded, &fast_stale);
                let (disposition, reason) =
                    evaluate_claim_use(artifact, claim_use, &status, valid_digest);
                ClaimResult {
                    artifact_id: claim_use.artifact_id.clone(),
                    claim_id: claim_use.claim_id.clone(),
                    scope_id: claim_use.scope_id.clone(),
                    disposition,
                    reason,
                }
            })
            .collect();
        claims.sort_by(|left, right| {
            (&left.artifact_id, &left.claim_id, &left.scope_id).cmp(&(
                &right.artifact_id,
                &right.claim_id,
                &right.scope_id,
            ))
        });

        let fixture_expectations_match =
            expectations_match(scenario, &fast_superseded, &fast_stale, &claims);
        let status = if oracle_agreement && fixture_expectations_match {
            "PASS"
        } else {
            "FAIL"
        };

        results.push(ScenarioResult {
            scenario_id: scenario.id.clone(),
            event_count_received: scenario.events.len(),
            unique_event_count: events.len(),
            superseded_artifacts: fast_superseded.into_iter().collect(),
            stale_artifacts: fast_stale.into_iter().collect(),
            oracle_agreement,
            fixture_expectations_match,
            claims,
            artifacts: artifact_results,
            status: status.to_owned(),
        });
    }

    let scenarios_pass = results.iter().all(|result| result.status == "PASS");
    let overall_status = if all_source_digests_valid && scenarios_pass {
        "PASS_SYNTHETIC_CONTRACT"
    } else {
        "FAIL"
    };

    Ok(Report {
        tool_version: TOOL_VERSION.to_owned(),
        schema_version: manifest.schema_version,
        source_revision: source_revision.to_owned(),
        manifest_sha256,
        claim_ceiling: manifest.claim_ceiling,
        artifact_count: manifest.artifacts.len(),
        scenario_count: results.len(),
        all_source_digests_valid,
        scenarios: results,
        overall_status: overall_status.to_owned(),
    })
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("writing into String cannot fail");
    }
    output
}

fn validate_manifest(manifest: &Manifest) -> Result<(), String> {
    if manifest.schema_version != "0.1" {
        return Err(format!(
            "unsupported schema_version {}; expected 0.1",
            manifest.schema_version
        ));
    }
    if manifest.claim_ceiling != "synthetic_only" {
        return Err("claim_ceiling must remain synthetic_only".to_owned());
    }
    if manifest.artifacts.is_empty() || manifest.scenarios.is_empty() {
        return Err("manifest requires at least one artifact and one scenario".to_owned());
    }

    let mut artifacts_by_id = BTreeMap::new();
    for artifact in &manifest.artifacts {
        if artifact.id.trim().is_empty()
            || artifact.revision.trim().is_empty()
            || artifact.claim_scope.claim_id.trim().is_empty()
            || artifact.claim_scope.scope_id.trim().is_empty()
        {
            return Err(
                "artifact IDs, revisions, claim IDs, and scope IDs must be non-empty".into(),
            );
        }
        if artifact.validity == EvidenceStatus::InvalidDigest {
            return Err(format!(
                "artifact {} cannot assert derived status invalid_digest",
                artifact.id
            ));
        }
        if artifact.source_content.is_empty() || !is_sha256_hex(&artifact.source_sha256) {
            return Err(format!(
                "artifact {} needs non-empty source_content and a 64-character lowercase SHA-256",
                artifact.id
            ));
        }
        if artifacts_by_id
            .insert(artifact.id.as_str(), artifact)
            .is_some()
        {
            return Err(format!("duplicate artifact ID {}", artifact.id));
        }

        let mut dependency_ids = BTreeSet::new();
        for dependency in &artifact.depends_on {
            if dependency.artifact_id == artifact.id {
                return Err(format!("artifact {} depends on itself", artifact.id));
            }
            if !dependency_ids.insert(dependency.artifact_id.as_str()) {
                return Err(format!(
                    "artifact {} repeats dependency {}",
                    artifact.id, dependency.artifact_id
                ));
            }
            if dependency
                .invalidates_on
                .iter()
                .collect::<BTreeSet<_>>()
                .len()
                != dependency.invalidates_on.len()
            {
                return Err(format!(
                    "artifact {} repeats a trigger for dependency {}",
                    artifact.id, dependency.artifact_id
                ));
            }
        }
    }

    for artifact in &manifest.artifacts {
        for dependency in &artifact.depends_on {
            if !artifacts_by_id.contains_key(dependency.artifact_id.as_str()) {
                return Err(format!(
                    "artifact {} references missing dependency {}",
                    artifact.id, dependency.artifact_id
                ));
            }
        }
    }

    let graph: BTreeMap<String, Vec<String>> = manifest
        .artifacts
        .iter()
        .map(|artifact| {
            (
                artifact.id.clone(),
                artifact
                    .depends_on
                    .iter()
                    .map(|dependency| dependency.artifact_id.clone())
                    .collect(),
            )
        })
        .collect();
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    for id in graph.keys() {
        if visit_cycle(id, &graph, &mut visiting, &mut visited) {
            return Err(format!("dependency graph contains a cycle through {id}"));
        }
    }

    let mut scenario_ids = BTreeSet::new();
    for scenario in &manifest.scenarios {
        if scenario.id.trim().is_empty() || !scenario_ids.insert(scenario.id.as_str()) {
            return Err(format!("empty or duplicate scenario ID {}", scenario.id));
        }
        let events = deduplicate_events(&scenario.events)?;
        for event in events {
            if event.id.trim().is_empty() {
                return Err(format!(
                    "scenario {} contains an empty event ID",
                    scenario.id
                ));
            }
            if !artifacts_by_id.contains_key(event.changed_artifact_id.as_str()) {
                return Err(format!(
                    "scenario {} event {} references missing artifact {}",
                    scenario.id, event.id, event.changed_artifact_id
                ));
            }
        }

        let mut use_keys = BTreeSet::new();
        for claim_use in &scenario.claim_uses {
            if !artifacts_by_id.contains_key(claim_use.artifact_id.as_str()) {
                return Err(format!(
                    "scenario {} claim references missing artifact {}",
                    scenario.id, claim_use.artifact_id
                ));
            }
            if claim_use.claim_id.trim().is_empty() || claim_use.scope_id.trim().is_empty() {
                return Err(format!(
                    "scenario {} contains an empty claim or scope ID",
                    scenario.id
                ));
            }
            if !use_keys.insert(claim_key(
                &claim_use.artifact_id,
                &claim_use.claim_id,
                &claim_use.scope_id,
            )) {
                return Err(format!(
                    "scenario {} repeats a claim use for artifact {}",
                    scenario.id, claim_use.artifact_id
                ));
            }
        }

        let mut expected_keys = BTreeSet::new();
        for expected in &scenario.expected_claims {
            let key = claim_key(
                &expected.artifact_id,
                &expected.claim_id,
                &expected.scope_id,
            );
            if !expected_keys.insert(key) {
                return Err(format!(
                    "scenario {} repeats an expected claim",
                    scenario.id
                ));
            }
        }
        if expected_keys != use_keys {
            return Err(format!(
                "scenario {} expected_claims must match claim_uses exactly",
                scenario.id
            ));
        }

        validate_expected_artifact_ids(&scenario.id, &scenario.expected_stale, &artifacts_by_id)?;
        validate_expected_artifact_ids(
            &scenario.id,
            &scenario.expected_superseded,
            &artifacts_by_id,
        )?;
        let expected_stale: BTreeSet<&str> =
            scenario.expected_stale.iter().map(String::as_str).collect();
        let expected_superseded: BTreeSet<&str> = scenario
            .expected_superseded
            .iter()
            .map(String::as_str)
            .collect();
        if expected_stale.len() != scenario.expected_stale.len()
            || expected_superseded.len() != scenario.expected_superseded.len()
            || !expected_stale.is_disjoint(&expected_superseded)
        {
            return Err(format!(
                "scenario {} expected invalidation sets contain duplicates or overlap",
                scenario.id
            ));
        }
    }
    Ok(())
}

fn validate_expected_artifact_ids(
    scenario_id: &str,
    ids: &[String],
    artifacts_by_id: &BTreeMap<&str, &EvidenceArtifact>,
) -> Result<(), String> {
    for id in ids {
        if !artifacts_by_id.contains_key(id.as_str()) {
            return Err(format!(
                "scenario {} expects unknown artifact {}",
                scenario_id, id
            ));
        }
    }
    Ok(())
}

fn visit_cycle(
    id: &str,
    graph: &BTreeMap<String, Vec<String>>,
    visiting: &mut BTreeSet<String>,
    visited: &mut BTreeSet<String>,
) -> bool {
    if visited.contains(id) {
        return false;
    }
    if !visiting.insert(id.to_owned()) {
        return true;
    }
    if let Some(dependencies) = graph.get(id) {
        for dependency in dependencies {
            if visit_cycle(dependency, graph, visiting, visited) {
                return true;
            }
        }
    }
    visiting.remove(id);
    visited.insert(id.to_owned());
    false
}

fn deduplicate_events(events: &[ChangeEvent]) -> Result<Vec<ChangeEvent>, String> {
    let mut by_id: BTreeMap<String, ChangeEvent> = BTreeMap::new();
    for event in events {
        match by_id.get(&event.id) {
            Some(previous) if previous != event => {
                return Err(format!("conflicting duplicate event ID {}", event.id));
            }
            Some(_) => {}
            None => {
                by_id.insert(event.id.clone(), event.clone());
            }
        }
    }
    Ok(by_id.into_values().collect())
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn final_status(
    artifact: &EvidenceArtifact,
    digest_valid: bool,
    superseded: &BTreeSet<String>,
    stale: &BTreeSet<String>,
) -> EvidenceStatus {
    if !digest_valid {
        EvidenceStatus::InvalidDigest
    } else if artifact.validity != EvidenceStatus::Valid {
        artifact.validity.clone()
    } else if superseded.contains(&artifact.id) {
        EvidenceStatus::Superseded
    } else if stale.contains(&artifact.id) {
        EvidenceStatus::Stale
    } else {
        EvidenceStatus::Valid
    }
}

/// Production selector: reverse adjacency and breadth-first propagation.
/// Trigger matching is required at the changed source's direct edge. After an
/// artifact is invalidated, its derived dependents are invalidated transitively.
fn invalidation_production(
    manifest: &Manifest,
    events: &[ChangeEvent],
    digest_valid: &BTreeMap<String, bool>,
) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut superseded = BTreeSet::new();
    for event in events {
        if let Some(artifact) = manifest
            .artifacts
            .iter()
            .find(|artifact| artifact.id == event.changed_artifact_id)
        {
            if artifact.validity == EvidenceStatus::Valid
                && digest_valid.get(&artifact.id).copied().unwrap_or(false)
            {
                superseded.insert(artifact.id.clone());
            }
        }
    }

    let mut reverse: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for artifact in &manifest.artifacts {
        for dependency in &artifact.depends_on {
            reverse
                .entry(dependency.artifact_id.clone())
                .or_default()
                .push(artifact.id.clone());
        }
    }
    for dependents in reverse.values_mut() {
        dependents.sort();
        dependents.dedup();
    }

    let mut reached = BTreeSet::new();
    let mut queue = VecDeque::new();
    for event in events {
        for artifact in &manifest.artifacts {
            let matches_trigger = artifact.depends_on.iter().any(|dependency| {
                dependency.artifact_id == event.changed_artifact_id
                    && dependency.invalidates_on.contains(&event.trigger)
            });
            if matches_trigger && reached.insert(artifact.id.clone()) {
                queue.push_back(artifact.id.clone());
            }
        }
    }

    while let Some(invalidated_id) = queue.pop_front() {
        if let Some(dependents) = reverse.get(&invalidated_id) {
            for dependent in dependents {
                if reached.insert(dependent.clone()) {
                    queue.push_back(dependent.clone());
                }
            }
        }
    }

    let stale = reached
        .into_iter()
        .filter(|id| {
            !superseded.contains(id)
                && digest_valid.get(id).copied().unwrap_or(false)
                && manifest.artifacts.iter().any(|artifact| {
                    artifact.id == *id && artifact.validity == EvidenceStatus::Valid
                })
        })
        .collect();
    (superseded, stale)
}

/// Independent reference oracle: repeated full graph scans to a fixed point.
/// It intentionally does not share the production selector's reverse index or
/// queue traversal.
fn invalidation_oracle(
    manifest: &Manifest,
    events: &[ChangeEvent],
    digest_valid: &BTreeMap<String, bool>,
) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut superseded = BTreeSet::new();
    for artifact in &manifest.artifacts {
        if artifact.validity == EvidenceStatus::Valid
            && digest_valid.get(&artifact.id).copied().unwrap_or(false)
            && events
                .iter()
                .any(|event| event.changed_artifact_id == artifact.id)
        {
            superseded.insert(artifact.id.clone());
        }
    }

    let mut invalidated: BTreeSet<String> = BTreeSet::new();
    for event in events {
        for artifact in &manifest.artifacts {
            for dependency in &artifact.depends_on {
                if dependency.artifact_id == event.changed_artifact_id
                    && dependency
                        .invalidates_on
                        .iter()
                        .any(|trigger| trigger == &event.trigger)
                {
                    invalidated.insert(artifact.id.clone());
                }
            }
        }
    }

    loop {
        let mut newly_invalidated = Vec::new();
        for artifact in &manifest.artifacts {
            if !invalidated.contains(&artifact.id)
                && artifact
                    .depends_on
                    .iter()
                    .any(|dependency| invalidated.contains(&dependency.artifact_id))
            {
                newly_invalidated.push(artifact.id.clone());
            }
        }
        if newly_invalidated.is_empty() {
            break;
        }
        invalidated.extend(newly_invalidated);
    }

    let stale = invalidated
        .into_iter()
        .filter(|id| {
            !superseded.contains(id)
                && digest_valid.get(id).copied().unwrap_or(false)
                && manifest.artifacts.iter().any(|artifact| {
                    artifact.id == *id && artifact.validity == EvidenceStatus::Valid
                })
        })
        .collect();
    (superseded, stale)
}

fn evaluate_claim_use(
    artifact: &EvidenceArtifact,
    claim_use: &ClaimUse,
    status: &EvidenceStatus,
    digest_valid: bool,
) -> (ClaimDisposition, String) {
    if !digest_valid || *status == EvidenceStatus::InvalidDigest {
        return (
            ClaimDisposition::InvalidDigest,
            "source bytes do not match the declared SHA-256".to_owned(),
        );
    }
    match status {
        EvidenceStatus::Unknown => {
            return (
                ClaimDisposition::Unknown,
                "evidence validity is unknown; unknown is not approval".to_owned(),
            );
        }
        EvidenceStatus::IncompleteCoverage => {
            return (
                ClaimDisposition::IncompleteCoverage,
                "evidence is explicitly marked incomplete".to_owned(),
            );
        }
        EvidenceStatus::OutOfScope => {
            return (
                ClaimDisposition::OutOfScope,
                "evidence is explicitly marked out of scope".to_owned(),
            );
        }
        EvidenceStatus::Valid => {}
        EvidenceStatus::Stale
        | EvidenceStatus::Revoked
        | EvidenceStatus::Contradictory
        | EvidenceStatus::Superseded => {
            return (
                ClaimDisposition::RejectedStatus,
                format!("current evidence status is {status:?}"),
            );
        }
        EvidenceStatus::InvalidDigest => unreachable!("handled above"),
    }
    if artifact.claim_scope.claim_id != claim_use.claim_id
        || artifact.claim_scope.scope_id != claim_use.scope_id
    {
        return (
            ClaimDisposition::OutOfScope,
            "claim ID or scope ID does not match the evidence envelope".to_owned(),
        );
    }
    match artifact.coverage {
        CoverageStatus::Complete => (
            ClaimDisposition::Accepted,
            "digest, validity, claim scope, and coverage gates are satisfied".to_owned(),
        ),
        CoverageStatus::Partial => (
            ClaimDisposition::IncompleteCoverage,
            "partial coverage cannot support a complete-coverage claim".to_owned(),
        ),
        CoverageStatus::Unknown => (
            ClaimDisposition::Unknown,
            "coverage is unknown; unknown is not confirmed absent".to_owned(),
        ),
    }
}

fn expectations_match(
    scenario: &Scenario,
    superseded: &BTreeSet<String>,
    stale: &BTreeSet<String>,
    claims: &[ClaimResult],
) -> bool {
    let expected_superseded: BTreeSet<String> =
        scenario.expected_superseded.iter().cloned().collect();
    let expected_stale: BTreeSet<String> = scenario.expected_stale.iter().cloned().collect();

    let actual_claims: BTreeMap<(String, String, String), ClaimDisposition> = claims
        .iter()
        .map(|claim| {
            (
                claim_key(&claim.artifact_id, &claim.claim_id, &claim.scope_id),
                claim.disposition.clone(),
            )
        })
        .collect();
    let expected_claims: BTreeMap<(String, String, String), ClaimDisposition> = scenario
        .expected_claims
        .iter()
        .map(|expected| {
            (
                claim_key(
                    &expected.artifact_id,
                    &expected.claim_id,
                    &expected.scope_id,
                ),
                expected.disposition.clone(),
            )
        })
        .collect();

    *superseded == expected_superseded
        && *stale == expected_stale
        && actual_claims == expected_claims
}

fn claim_key(artifact_id: &str, claim_id: &str, scope_id: &str) -> (String, String, String) {
    (
        artifact_id.to_owned(),
        claim_id.to_owned(),
        scope_id.to_owned(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../fixtures/evidence-validity-v0-1.json");

    fn manifest() -> Manifest {
        serde_json::from_str(FIXTURE).expect("frozen fixture must parse")
    }

    fn run_manifest(manifest: &Manifest) -> Report {
        let bytes = serde_json::to_vec(manifest).expect("manifest serialization must work");
        evaluate(&bytes, "test-revision").expect("manifest must evaluate")
    }

    fn scenario<'a>(report: &'a Report, id: &str) -> &'a ScenarioResult {
        report
            .scenarios
            .iter()
            .find(|scenario| scenario.scenario_id == id)
            .expect("scenario must exist")
    }

    #[test]
    fn frozen_corpus_matches_expected_results_and_independent_oracle() {
        let report = evaluate(FIXTURE.as_bytes(), "test-revision").unwrap();
        assert_eq!(report.overall_status, "PASS_SYNTHETIC_CONTRACT");
        assert!(report.all_source_digests_valid);
        assert_eq!(report.artifact_count, 9);
        assert_eq!(report.scenario_count, 5);
        for result in &report.scenarios {
            assert_eq!(result.status, "PASS", "scenario {}", result.scenario_id);
            assert!(result.oracle_agreement, "scenario {}", result.scenario_id);
            assert!(
                result.fixture_expectations_match,
                "scenario {}",
                result.scenario_id
            );
        }
    }

    #[test]
    fn duplicated_delivery_is_idempotent() {
        let report = evaluate(FIXTURE.as_bytes(), "test-revision").unwrap();
        let once = scenario(&report, "terrain-change");
        let twice = scenario(&report, "terrain-change-duplicate");
        assert_eq!(twice.event_count_received, 2);
        assert_eq!(twice.unique_event_count, 1);
        assert_eq!(once.superseded_artifacts, twice.superseded_artifacts);
        assert_eq!(once.stale_artifacts, twice.stale_artifacts);
        assert_eq!(once.claims, twice.claims);
    }

    #[test]
    fn unrelated_furnishing_change_does_not_invalidate_site_evidence() {
        let report = evaluate(FIXTURE.as_bytes(), "test-revision").unwrap();
        let result = scenario(&report, "furnishing-change");
        assert_eq!(
            result.superseded_artifacts,
            vec!["furnishing-01".to_owned()]
        );
        assert!(result.stale_artifacts.is_empty());
        let concept = result
            .artifacts
            .iter()
            .find(|artifact| artifact.id == "concept-01")
            .unwrap();
        assert_eq!(concept.current_status, EvidenceStatus::Valid);
    }

    #[test]
    fn claims_are_blocked_for_revoked_partial_unknown_and_wrong_scope_evidence() {
        let report = evaluate(FIXTURE.as_bytes(), "test-revision").unwrap();
        let baseline = scenario(&report, "baseline");
        let disposition = |artifact_id: &str, scope_id: &str| {
            baseline
                .claims
                .iter()
                .find(|claim| claim.artifact_id == artifact_id && claim.scope_id == scope_id)
                .unwrap()
                .disposition
                .clone()
        };
        assert_eq!(
            disposition("revoked-certificate-01", "batch-99"),
            ClaimDisposition::RejectedStatus
        );
        assert_eq!(
            disposition("partial-survey-01", "site-B"),
            ClaimDisposition::IncompleteCoverage
        );
        assert_eq!(
            disposition("unknown-groundwater-01", "site-B"),
            ClaimDisposition::Unknown
        );
        assert_eq!(
            disposition("furnishing-01", "site-A"),
            ClaimDisposition::OutOfScope
        );
    }

    #[test]
    fn digest_mismatch_blocks_claim_and_fails_the_corpus() {
        let mut manifest = manifest();
        let batch = manifest
            .artifacts
            .iter_mut()
            .find(|artifact| artifact.id == "material-batch-01")
            .unwrap();
        batch.source_sha256 = "0".repeat(64);
        let report = run_manifest(&manifest);
        assert_eq!(report.overall_status, "FAIL");
        assert!(!report.all_source_digests_valid);
        let baseline = scenario(&report, "baseline");
        let claim = baseline
            .claims
            .iter()
            .find(|claim| claim.artifact_id == "material-batch-01")
            .unwrap();
        assert_eq!(claim.disposition, ClaimDisposition::InvalidDigest);
    }

    #[test]
    fn unknown_artifact_fields_are_rejected_by_deserialization() {
        let bad = FIXTURE.replace(
            "\"schema_version\": \"0.1\"",
            "\"schema_version\": \"0.1\", \"unreviewed_authority\": true",
        );
        assert!(evaluate(bad.as_bytes(), "test-revision").is_err());
    }

    #[test]
    fn dangling_dependencies_are_rejected() {
        let mut manifest = manifest();
        manifest
            .artifacts
            .iter_mut()
            .find(|artifact| artifact.id == "concept-01")
            .unwrap()
            .depends_on[0]
            .artifact_id = "missing-artifact".to_owned();
        let bytes = serde_json::to_vec(&manifest).unwrap();
        assert!(
            evaluate(&bytes, "test-revision")
                .unwrap_err()
                .contains("missing dependency")
        );
    }

    #[test]
    fn dependency_cycles_are_rejected() {
        let mut manifest = manifest();
        manifest
            .artifacts
            .iter_mut()
            .find(|artifact| artifact.id == "terrain-01")
            .unwrap()
            .depends_on
            .push(Dependency {
                artifact_id: "concept-01".to_owned(),
                invalidates_on: vec![Trigger::TerrainChanged],
            });
        let bytes = serde_json::to_vec(&manifest).unwrap();
        assert!(
            evaluate(&bytes, "test-revision")
                .unwrap_err()
                .contains("cycle")
        );
    }

    #[test]
    fn conflicting_duplicate_event_identity_fails_closed() {
        let mut manifest = manifest();
        let events = &mut manifest
            .scenarios
            .iter_mut()
            .find(|scenario| scenario.id == "terrain-change")
            .unwrap()
            .events;
        events.push(ChangeEvent {
            id: "terrain-change-01".to_owned(),
            trigger: Trigger::FurnishingSubstituted,
            changed_artifact_id: "furnishing-01".to_owned(),
        });
        let bytes = serde_json::to_vec(&manifest).unwrap();
        assert!(
            evaluate(&bytes, "test-revision")
                .unwrap_err()
                .contains("conflicting duplicate event ID")
        );
    }

    #[test]
    fn event_order_does_not_change_invalidation_sets() {
        let mut first = manifest();
        let events = &mut first
            .scenarios
            .iter_mut()
            .find(|scenario| scenario.id == "terrain-change")
            .unwrap()
            .events;
        events.push(ChangeEvent {
            id: "supplier-substitution-extra".to_owned(),
            trigger: Trigger::SupplierSubstitution,
            changed_artifact_id: "material-batch-01".to_owned(),
        });
        let mut second = first.clone();
        second
            .scenarios
            .iter_mut()
            .find(|scenario| scenario.id == "terrain-change")
            .unwrap()
            .events
            .reverse();

        let left_report = run_manifest(&first);
        let right_report = run_manifest(&second);
        let left = scenario(&left_report, "terrain-change");
        let right = scenario(&right_report, "terrain-change");
        assert_eq!(left.superseded_artifacts, right.superseded_artifacts);
        assert_eq!(left.stale_artifacts, right.stale_artifacts);
        assert_eq!(left.claims, right.claims);
    }

    #[test]
    fn sha256_known_answer() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
