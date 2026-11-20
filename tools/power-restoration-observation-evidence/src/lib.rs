use power_restoration_action_proposal::{ActionKind, ActionProposal, validate_proposal};
use power_restoration_benchmark_case::{BenchmarkCase, canonical_case_sha256, validate_case};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EvidenceBatch {
    pub schema_version: String,
    pub batch_id: String,
    pub case_ref: String,
    pub case_sha256: String,
    pub proposal_id: String,
    pub proposal_sha256: String,
    pub presentation_step: u32,
    pub sources: Vec<SourceProvenance>,
    pub results: Vec<InformationRequestResult>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InformationRequestResult {
    pub request_id: String,
    pub action_kind: ActionKind,
    pub requested_step: u32,
    pub outcome: RequestOutcome,
    pub requested_at_unix_ms: u64,
    pub completed_at_unix_ms: Option<u64>,
    pub available_at_step: Option<u32>,
    pub available_at_unix_ms: Option<u64>,
    pub evidence: Vec<EvidenceArtifact>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RequestOutcome {
    Pending,
    Fulfilled,
    Partial,
    Denied,
    TimedOut,
    Unavailable,
    Failed,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EvidenceArtifact {
    pub evidence_id: String,
    pub evidence_sha256: String,
    pub observed_step: u32,
    pub observed_at_unix_ms: u64,
    pub available_step: u32,
    pub available_at_unix_ms: u64,
    pub max_age_steps: u32,
    pub quality: EvidenceQuality,
    pub source_id: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceQuality {
    Current,
    Stale,
    Conflicting,
    Unverified,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceProvenance {
    pub source_id: String,
    pub source_kind: SourceKind,
    pub producer_id: String,
    pub producer_version: String,
    pub source_snapshot_sha256: String,
    pub producer_attestation_ref: String,
    pub producer_attestation_sha256: String,
    pub parent_source_ids: Vec<String>,
    pub common_cause_group_ids: Vec<String>,
    pub lineage_completeness: LineageCompleteness,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Scada,
    CallReport,
    NetworkModel,
    FieldInspection,
    FaultIndicator,
    SwitchTest,
    LoadForecast,
    CounterfactualSimulation,
    Other,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LineageCompleteness {
    Complete,
    Partial,
    Unknown,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceDisposition {
    FreshCurrentCandidate,
    StaleByAge,
    SourceDeclaredStale,
    Conflicting,
    Unverified,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceFinding {
    pub evidence_id: String,
    pub source_id: String,
    pub age_at_presentation_steps: u32,
    pub disposition: EvidenceDisposition,
    pub producer_attestation_status: String,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DependenceFindingKind {
    SharedSourceLineage,
    SharedCommonCauseGroup,
    NoDeclaredOverlapIndependenceUnproven,
    IncompleteLineageIndependenceUnproven,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DependenceFinding {
    pub left_evidence_id: String,
    pub right_evidence_id: String,
    pub finding: DependenceFindingKind,
    pub shared_source_ids: Vec<String>,
    pub shared_common_cause_group_ids: Vec<String>,
    pub interpretation: String,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ObservationEvidenceReport {
    pub batch_id: String,
    pub case_sha256: String,
    pub proposal_sha256: String,
    pub presentation_step: u32,
    pub request_count: usize,
    pub fulfilled_request_count: usize,
    pub partial_request_count: usize,
    pub pending_request_count: usize,
    pub unsuccessful_request_count: usize,
    pub evidence_count: usize,
    pub fresh_current_candidate_count: usize,
    pub stale_at_presentation_count: usize,
    pub conflicting_evidence_count: usize,
    pub unverified_evidence_count: usize,
    pub evidence_findings: Vec<EvidenceFinding>,
    pub dependence_findings: Vec<DependenceFinding>,
    pub result_status: String,
    pub claim_boundary: String,
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty()
}

fn is_lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn is_information_action(kind: ActionKind) -> bool {
    matches!(
        kind,
        ActionKind::ObserveScada
            | ActionKind::ObserveCalls
            | ActionKind::ObserveNetworkModel
            | ActionKind::CrewInspect
            | ActionKind::ReadFaultIndicator
            | ActionKind::TestSwitch
            | ActionKind::SimulateSwitch
            | ActionKind::ForecastLoad
            | ActionKind::EscalateToHuman
    )
}

fn source_kind_matches_action(action: ActionKind, source: SourceKind) -> bool {
    match action {
        ActionKind::ObserveScada => source == SourceKind::Scada,
        ActionKind::ObserveCalls => source == SourceKind::CallReport,
        ActionKind::ObserveNetworkModel => source == SourceKind::NetworkModel,
        ActionKind::CrewInspect => source == SourceKind::FieldInspection,
        ActionKind::ReadFaultIndicator => source == SourceKind::FaultIndicator,
        ActionKind::TestSwitch => source == SourceKind::SwitchTest,
        ActionKind::SimulateSwitch => source == SourceKind::CounterfactualSimulation,
        ActionKind::ForecastLoad => source == SourceKind::LoadForecast,
        ActionKind::EscalateToHuman => source == SourceKind::Other,
        _ => false,
    }
}

/// Hashes the normalized JSON value. Object keys are sorted by serde_json's
/// default map representation; array order is preserved. This is not a hash
/// of the original whitespace or byte encoding.
pub fn canonical_proposal_sha256(proposal: &ActionProposal) -> Result<String, String> {
    let value = serde_json::to_value(proposal)
        .map_err(|error| format!("proposal serialization failed: {error}"))?;
    let canonical = serde_json::to_vec(&value)
        .map_err(|error| format!("proposal canonicalization failed: {error}"))?;
    let digest = Sha256::digest(canonical);
    Ok(format!("{digest:x}"))
}

type SourceMap<'a> = BTreeMap<String, &'a SourceProvenance>;

fn build_source_map<'a>(
    sources: &'a [SourceProvenance],
    errors: &mut Vec<String>,
) -> SourceMap<'a> {
    let mut source_map = BTreeMap::new();

    for source in sources {
        if !valid_id(&source.source_id)
            || !valid_id(&source.producer_id)
            || !valid_id(&source.producer_version)
            || !valid_id(&source.producer_attestation_ref)
            || !is_lower_hex(&source.source_snapshot_sha256, 64)
            || !is_lower_hex(&source.producer_attestation_sha256, 64)
        {
            errors.push(format!(
                "source {} has invalid identity, attestation reference or digest",
                source.source_id
            ));
        }

        let mut parent_ids = BTreeSet::new();
        for parent_id in &source.parent_source_ids {
            if !valid_id(parent_id) || !parent_ids.insert(parent_id) {
                errors.push(format!(
                    "source {} has an invalid or duplicate parent source",
                    source.source_id
                ));
            }
            if parent_id == &source.source_id {
                errors.push(format!(
                    "source {} cannot name itself as a parent",
                    source.source_id
                ));
            }
        }

        let mut groups = BTreeSet::new();
        for group_id in &source.common_cause_group_ids {
            if !valid_id(group_id) || !groups.insert(group_id) {
                errors.push(format!(
                    "source {} has an invalid or duplicate common-cause group",
                    source.source_id
                ));
            }
        }

        if source_map
            .insert(source.source_id.clone(), source)
            .is_some()
        {
            errors.push(format!("duplicate source ID {}", source.source_id));
        }
    }

    for source in sources {
        for parent_id in &source.parent_source_ids {
            if !source_map.contains_key(parent_id) {
                errors.push(format!(
                    "source {} references missing parent source {}",
                    source.source_id, parent_id
                ));
            }
        }
    }

    let mut states = BTreeMap::new();
    let source_ids: Vec<String> = source_map.keys().cloned().collect();
    for source_id in source_ids {
        visit_source(&source_id, &source_map, &mut states, errors);
    }

    source_map
}

fn visit_source(
    source_id: &str,
    sources: &SourceMap<'_>,
    states: &mut BTreeMap<String, u8>,
    errors: &mut Vec<String>,
) {
    match states.get(source_id).copied().unwrap_or(0) {
        1 => {
            errors.push(format!("source lineage contains a cycle at {source_id}"));
            return;
        }
        2 => return,
        _ => {}
    }

    states.insert(source_id.to_string(), 1);
    if let Some(source) = sources.get(source_id) {
        for parent_id in &source.parent_source_ids {
            if sources.contains_key(parent_id) {
                visit_source(parent_id, sources, states, errors);
            }
        }
    }
    states.insert(source_id.to_string(), 2);
}

fn collect_lineage(
    source_id: &str,
    sources: &SourceMap<'_>,
    visited: &mut BTreeSet<String>,
) {
    if !visited.insert(source_id.to_string()) {
        return;
    }
    if let Some(source) = sources.get(source_id) {
        for parent_id in &source.parent_source_ids {
            collect_lineage(parent_id, sources, visited);
        }
    }
}

fn collect_common_cause_groups(
    lineage: &BTreeSet<String>,
    sources: &SourceMap<'_>,
) -> BTreeSet<String> {
    let mut groups = BTreeSet::new();
    for source_id in lineage {
        if let Some(source) = sources.get(source_id) {
            groups.extend(source.common_cause_group_ids.iter().cloned());
        }
    }
    groups
}

fn lineage_is_complete(
    lineage: &BTreeSet<String>,
    sources: &SourceMap<'_>,
) -> bool {
    lineage.iter().all(|source_id| {
        sources
            .get(source_id)
            .is_some_and(|source| source.lineage_completeness == LineageCompleteness::Complete)
    })
}

fn evidence_disposition(
    quality: EvidenceQuality,
    age_at_presentation_steps: u32,
    max_age_steps: u32,
) -> EvidenceDisposition {
    match quality {
        EvidenceQuality::Stale => EvidenceDisposition::SourceDeclaredStale,
        EvidenceQuality::Conflicting => EvidenceDisposition::Conflicting,
        EvidenceQuality::Unverified => EvidenceDisposition::Unverified,
        EvidenceQuality::Current if age_at_presentation_steps > max_age_steps => {
            EvidenceDisposition::StaleByAge
        }
        EvidenceQuality::Current => EvidenceDisposition::FreshCurrentCandidate,
    }
}

fn dependence_findings(
    evidence: &[&EvidenceArtifact],
    sources: &SourceMap<'_>,
) -> Vec<DependenceFinding> {
    let mut findings = Vec::new();

    for left_index in 0..evidence.len() {
        for right_index in (left_index + 1)..evidence.len() {
            let left = evidence[left_index];
            let right = evidence[right_index];

            let mut left_lineage = BTreeSet::new();
            collect_lineage(&left.source_id, sources, &mut left_lineage);
            let mut right_lineage = BTreeSet::new();
            collect_lineage(&right.source_id, sources, &mut right_lineage);

            let shared_source_ids: Vec<String> = left_lineage
                .intersection(&right_lineage)
                .cloned()
                .collect();

            let left_groups = collect_common_cause_groups(&left_lineage, sources);
            let right_groups = collect_common_cause_groups(&right_lineage, sources);
            let shared_common_cause_group_ids: Vec<String> = left_groups
                .intersection(&right_groups)
                .cloned()
                .collect();

            let (finding, interpretation) = if !shared_source_ids.is_empty() {
                (
                    DependenceFindingKind::SharedSourceLineage,
                    "Known shared source ancestry; do not count as independent evidence".to_string(),
                )
            } else if !shared_common_cause_group_ids.is_empty() {
                (
                    DependenceFindingKind::SharedCommonCauseGroup,
                    "Known shared common-cause group; do not count as independent evidence".to_string(),
                )
            } else if lineage_is_complete(&left_lineage, sources)
                && lineage_is_complete(&right_lineage, sources)
            {
                (
                    DependenceFindingKind::NoDeclaredOverlapIndependenceUnproven,
                    "No shared lineage or common-cause group is declared; independence remains unproven".to_string(),
                )
            } else {
                (
                    DependenceFindingKind::IncompleteLineageIndependenceUnproven,
                    "Lineage is partial or unknown; dependence cannot be excluded and independence is unproven".to_string(),
                )
            };

            findings.push(DependenceFinding {
                left_evidence_id: left.evidence_id.clone(),
                right_evidence_id: right.evidence_id.clone(),
                finding,
                shared_source_ids,
                shared_common_cause_group_ids,
                interpretation,
            });
        }
    }

    findings
}

pub fn validate_evidence_batch(
    batch: &EvidenceBatch,
    proposal: &ActionProposal,
    case: &BenchmarkCase,
) -> Result<ObservationEvidenceReport, Vec<String>> {
    let mut errors = Vec::new();

    let case_errors = validate_case(case);
    errors.extend(
        case_errors
            .into_iter()
            .map(|error| format!("invalid benchmark case: {error}")),
    );

    let actual_case_sha256 = canonical_case_sha256(case)
        .map_err(|error| vec![format!("case canonicalization failed: {error}")])?;
    if batch.schema_version != "0.1.0"
        || !valid_id(&batch.batch_id)
        || batch.case_ref != format!("power-restoration-case://{}", case.case_id)
        || batch.case_sha256 != actual_case_sha256
        || !is_lower_hex(&batch.case_sha256, 64)
    {
        errors.push("evidence batch schema, identity or frozen case binding is invalid".to_string());
    }

    if batch.proposal_id != proposal.proposal_id {
        errors.push("evidence batch proposal ID does not match the action proposal".to_string());
    }

    let proposal_sha256 = canonical_proposal_sha256(proposal)
        .map_err(|error| vec![format!("proposal canonicalization failed: {error}")])?;
    if batch.proposal_sha256 != proposal_sha256
        || !is_lower_hex(&batch.proposal_sha256, 64)
    {
        errors.push("evidence batch proposal digest does not match the normalized action proposal".to_string());
    }

    if let Err(proposal_errors) = validate_proposal(proposal, case, &actual_case_sha256) {
        errors.extend(
            proposal_errors
                .into_iter()
                .map(|error| format!("invalid action proposal: {error}")),
        );
    }

    if batch.presentation_step >= case.timebase.horizon_steps
        || batch.presentation_step < proposal.decision_step
    {
        errors.push("presentation step is outside the case horizon or precedes the proposal".to_string());
    }

    let information_actions: BTreeMap<String, &power_restoration_action_proposal::Action> =
        proposal
            .actions
            .iter()
            .filter(|action| is_information_action(action.action_kind))
            .map(|action| (action.action_id.clone(), action))
            .collect();

    if batch.results.len() != information_actions.len() {
        errors.push(format!(
            "result accounting mismatch: {} information requests but {} result records",
            information_actions.len(),
            batch.results.len()
        ));
    }

    let mut result_ids = BTreeSet::new();
    let source_map = build_source_map(&batch.sources, &mut errors);
    let mut evidence_ids = BTreeSet::new();
    let mut evidence_refs = Vec::new();

    for result in &batch.results {
        if !result_ids.insert(result.request_id.as_str()) {
            errors.push(format!("duplicate result for request {}", result.request_id));
        }

        let Some(action) = information_actions.get(&result.request_id) else {
            errors.push(format!(
                "result {} does not reference an information request in the proposal",
                result.request_id
            ));
            continue;
        };

        if result.action_kind != action.action_kind
            || result.requested_step != action.decision_step
            || !valid_id(&result.request_id)
            || result.requested_at_unix_ms == 0
        {
            errors.push(format!(
                "result {} does not match its request kind, epoch or request identity",
                result.request_id
            ));
        }

        match result.outcome {
            RequestOutcome::Pending => {
                if result.completed_at_unix_ms.is_some()
                    || result.available_at_step.is_some()
                    || result.available_at_unix_ms.is_some()
                    || !result.evidence.is_empty()
                {
                    errors.push(format!(
                        "pending request {} cannot contain completion, availability or evidence",
                        result.request_id
                    ));
                }
            }
            outcome => {
                let (Some(completed_at), Some(available_step), Some(available_at)) = (
                    result.completed_at_unix_ms,
                    result.available_at_step,
                    result.available_at_unix_ms,
                ) else {
                    errors.push(format!(
                        "non-pending request {} is missing completion or availability time",
                        result.request_id
                    ));
                    continue;
                };

                if completed_at < result.requested_at_unix_ms
                    || available_at < completed_at
                    || available_step < result.requested_step
                    || available_step > batch.presentation_step
                    || available_step >= case.timebase.horizon_steps
                {
                    errors.push(format!(
                        "request {} has impossible completion or availability ordering",
                        result.request_id
                    ));
                }

                let has_evidence = matches!(
                    outcome,
                    RequestOutcome::Fulfilled | RequestOutcome::Partial
                );
                if has_evidence && result.evidence.is_empty() {
                    errors.push(format!(
                        "fulfilled/partial request {} has no evidence artifacts",
                        result.request_id
                    ));
                }
                if !has_evidence && !result.evidence.is_empty() {
                    errors.push(format!(
                        "unsuccessful request {} cannot carry fabricated evidence",
                        result.request_id
                    ));
                }

                for item in &result.evidence {
                    if !evidence_ids.insert(item.evidence_id.as_str()) {
                        errors.push(format!("duplicate evidence ID {}", item.evidence_id));
                    }
                    if !valid_id(&item.evidence_id)
                        || !valid_id(&item.source_id)
                        || !is_lower_hex(&item.evidence_sha256, 64)
                        || item.observed_at_unix_ms == 0
                        || item.available_at_unix_ms == 0
                    {
                        errors.push(format!(
                            "evidence {} has invalid identity, digest or timestamp",
                            item.evidence_id
                        ));
                    }

                    if item.observed_step >= case.timebase.horizon_steps
                        || item.available_step >= case.timebase.horizon_steps
                        || item.observed_step > item.available_step
                        || item.available_step > available_step
                        || item.available_step > batch.presentation_step
                        || item.observed_at_unix_ms > completed_at
                        || item.available_at_unix_ms > available_at
                        || item.available_at_unix_ms < item.observed_at_unix_ms
                    {
                        errors.push(format!(
                            "evidence {} has future observation, late/unavailable presentation or impossible timestamp order",
                            item.evidence_id
                        ));
                    }

                    if let Some(source) = source_map.get(&item.source_id) {
                        if !source_kind_matches_action(action.action_kind, source.source_kind) {
                            errors.push(format!(
                                "evidence {} source kind does not match request {}",
                                item.evidence_id, result.request_id
                            ));
                        }
                    } else {
                        errors.push(format!(
                            "evidence {} references missing source {}",
                            item.evidence_id, item.source_id
                        ));
                    }

                    evidence_refs.push(item);
                }
            }
        }
    }

    for request_id in information_actions.keys() {
        if !result_ids.contains(request_id.as_str()) {
            errors.push(format!("information request {request_id} has no result record"));
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    let mut evidence_findings = Vec::new();
    for item in &evidence_refs {
        let age = batch.presentation_step.saturating_sub(item.observed_step);
        evidence_findings.push(EvidenceFinding {
            evidence_id: item.evidence_id.clone(),
            source_id: item.source_id.clone(),
            age_at_presentation_steps: age,
            disposition: evidence_disposition(item.quality, age, item.max_age_steps),
            producer_attestation_status: "reference_present_not_cryptographically_verified".to_string(),
        });
    }

    let fulfilled_request_count = batch
        .results
        .iter()
        .filter(|result| result.outcome == RequestOutcome::Fulfilled)
        .count();
    let partial_request_count = batch
        .results
        .iter()
        .filter(|result| result.outcome == RequestOutcome::Partial)
        .count();
    let pending_request_count = batch
        .results
        .iter()
        .filter(|result| result.outcome == RequestOutcome::Pending)
        .count();
    let unsuccessful_request_count = batch
        .results
        .iter()
        .filter(|result| {
            matches!(
                result.outcome,
                RequestOutcome::Denied
                    | RequestOutcome::TimedOut
                    | RequestOutcome::Unavailable
                    | RequestOutcome::Failed
            )
        })
        .count();

    let count_disposition = |disposition| {
        evidence_findings
            .iter()
            .filter(|finding| finding.disposition == disposition)
            .count()
    };

    Ok(ObservationEvidenceReport {
        batch_id: batch.batch_id.clone(),
        case_sha256: actual_case_sha256,
        proposal_sha256,
        presentation_step: batch.presentation_step,
        request_count: batch.results.len(),
        fulfilled_request_count,
        partial_request_count,
        pending_request_count,
        unsuccessful_request_count,
        evidence_count: evidence_findings.len(),
        fresh_current_candidate_count: count_disposition(EvidenceDisposition::FreshCurrentCandidate),
        stale_at_presentation_count: count_disposition(EvidenceDisposition::StaleByAge)
            + count_disposition(EvidenceDisposition::SourceDeclaredStale),
        conflicting_evidence_count: count_disposition(EvidenceDisposition::Conflicting),
        unverified_evidence_count: count_disposition(EvidenceDisposition::Unverified),
        dependence_findings: dependence_findings(&evidence_refs, &source_map),
        evidence_findings,
        result_status: "observation_evidence_structurally_valid".to_string(),
        claim_boundary: "Request/result accounting, temporal consistency and declared source-lineage qualification only; producer authenticity, sensor truth, statistical independence, electrical feasibility, authority and service restoration are not established".to_string(),
    })
}

pub fn parse_and_validate(
    batch_json: &str,
    proposal_json: &str,
    case_json: &str,
) -> Result<ObservationEvidenceReport, Vec<String>> {
    let batch: EvidenceBatch = serde_json::from_str(batch_json)
        .map_err(|error| vec![format!("evidence batch JSON decode error: {error}")])?;
    let proposal: ActionProposal = serde_json::from_str(proposal_json)
        .map_err(|error| vec![format!("action proposal JSON decode error: {error}")])?;
    let case: BenchmarkCase = serde_json::from_str(case_json)
        .map_err(|error| vec![format!("benchmark case JSON decode error: {error}")])?;
    validate_evidence_batch(&batch, &proposal, &case)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixtures() -> (EvidenceBatch, ActionProposal, BenchmarkCase) {
        let batch: EvidenceBatch = serde_json::from_str(include_str!(
            "../fixtures/synthetic-observation-evidence-batch-v0.1.json"
        ))
        .expect("evidence fixture must deserialize");
        let proposal: ActionProposal = serde_json::from_str(include_str!(
            "../../power-restoration-action-proposal/fixtures/synthetic-information-request-proposal-v0.1.json"
        ))
        .expect("proposal fixture must deserialize");
        let case: BenchmarkCase = serde_json::from_str(include_str!(
            "../../power-restoration-benchmark-case/fixtures/synthetic-restoration-case-v0.1.json"
        ))
        .expect("case fixture must deserialize");
        (batch, proposal, case)
    }

    #[test]
    fn synthetic_scada_result_is_structurally_valid() {
        let (batch, proposal, case) = fixtures();
        let report = validate_evidence_batch(&batch, &proposal, &case).unwrap();
        assert_eq!(report.result_status, "observation_evidence_structurally_valid");
        assert_eq!(report.request_count, 1);
        assert_eq!(report.fulfilled_request_count, 1);
        assert_eq!(report.fresh_current_candidate_count, 1);
        assert_eq!(report.evidence_count, 1);
    }

    #[test]
    fn case_digest_mismatch_is_rejected() {
        let (mut batch, proposal, case) = fixtures();
        batch.case_sha256 = "0".repeat(64);
        assert!(validate_evidence_batch(&batch, &proposal, &case).is_err());
    }

    #[test]
    fn proposal_digest_mismatch_is_rejected() {
        let (mut batch, proposal, case) = fixtures();
        batch.proposal_sha256 = "0".repeat(64);
        assert!(validate_evidence_batch(&batch, &proposal, &case).is_err());
    }

    #[test]
    fn every_information_request_requires_one_result() {
        let (mut batch, proposal, case) = fixtures();
        batch.results.clear();
        assert!(validate_evidence_batch(&batch, &proposal, &case).is_err());
    }

    #[test]
    fn duplicate_request_results_are_rejected() {
        let (mut batch, proposal, case) = fixtures();
        batch.results.push(batch.results[0].clone());
        assert!(validate_evidence_batch(&batch, &proposal, &case).is_err());
    }

    #[test]
    fn result_kind_and_epoch_must_match_the_request() {
        let (mut batch, proposal, case) = fixtures();
        batch.results[0].requested_step = 1;
        assert!(validate_evidence_batch(&batch, &proposal, &case).is_err());
    }

    #[test]
    fn future_evidence_is_rejected() {
        let (mut batch, proposal, case) = fixtures();
        batch.results[0].evidence[0].available_step = batch.presentation_step + 1;
        assert!(validate_evidence_batch(&batch, &proposal, &case).is_err());
    }

    #[test]
    fn observations_after_result_completion_are_rejected() {
        let (mut batch, proposal, case) = fixtures();
        batch.results[0].evidence[0].observed_at_unix_ms =
            batch.results[0].completed_at_unix_ms.unwrap() + 1;
        assert!(validate_evidence_batch(&batch, &proposal, &case).is_err());
    }

    #[test]
    fn stale_at_presentation_remains_visible_but_is_not_a_fresh_candidate() {
        let (mut batch, proposal, case) = fixtures();
        batch.results[0].evidence[0].max_age_steps = 0;
        let report = validate_evidence_batch(&batch, &proposal, &case).unwrap();
        assert_eq!(report.evidence_count, 1);
        assert_eq!(report.stale_at_presentation_count, 1);
        assert_eq!(report.fresh_current_candidate_count, 0);
    }

    #[test]
    fn conflicting_evidence_is_not_a_fresh_current_candidate() {
        let (mut batch, proposal, case) = fixtures();
        batch.results[0].evidence[0].quality = EvidenceQuality::Conflicting;
        let report = validate_evidence_batch(&batch, &proposal, &case).unwrap();
        assert_eq!(report.conflicting_evidence_count, 1);
        assert_eq!(report.fresh_current_candidate_count, 0);
    }

    #[test]
    fn unverified_evidence_is_counted_separately() {
        let (mut batch, proposal, case) = fixtures();
        batch.results[0].evidence[0].quality = EvidenceQuality::Unverified;
        let report = validate_evidence_batch(&batch, &proposal, &case).unwrap();
        assert_eq!(report.unverified_evidence_count, 1);
        assert_eq!(report.fresh_current_candidate_count, 0);
    }

    #[test]
    fn evidence_source_kind_must_match_the_requested_information_action() {
        let (mut batch, proposal, case) = fixtures();
        batch.sources[0].source_kind = SourceKind::CallReport;
        assert!(validate_evidence_batch(&batch, &proposal, &case).is_err());
    }

    #[test]
    fn failed_request_cannot_carry_evidence() {
        let (mut batch, proposal, case) = fixtures();
        batch.results[0].outcome = RequestOutcome::TimedOut;
        batch.results[0].evidence.clear();
        let report = validate_evidence_batch(&batch, &proposal, &case).unwrap();
        assert_eq!(report.unsuccessful_request_count, 1);
        assert_eq!(report.evidence_count, 0);
    }

    #[test]
    fn pending_request_has_no_completion_or_evidence() {
        let (mut batch, proposal, case) = fixtures();
        let result = &mut batch.results[0];
        result.outcome = RequestOutcome::Pending;
        result.completed_at_unix_ms = None;
        result.available_at_step = None;
        result.available_at_unix_ms = None;
        result.evidence.clear();
        let report = validate_evidence_batch(&batch, &proposal, &case).unwrap();
        assert_eq!(report.pending_request_count, 1);
    }

    #[test]
    fn pending_request_cannot_claim_a_result() {
        let (mut batch, proposal, case) = fixtures();
        batch.results[0].outcome = RequestOutcome::Pending;
        assert!(validate_evidence_batch(&batch, &proposal, &case).is_err());
    }

    #[test]
    fn missing_parent_source_is_rejected() {
        let (mut batch, proposal, case) = fixtures();
        batch.sources[0].parent_source_ids.push("missing-parent".to_string());
        assert!(validate_evidence_batch(&batch, &proposal, &case).is_err());
    }

    #[test]
    fn cyclic_source_lineage_is_rejected() {
        let (mut batch, proposal, case) = fixtures();
        batch.sources[0].parent_source_ids.push(batch.sources[0].source_id.clone());
        assert!(validate_evidence_batch(&batch, &proposal, &case).is_err());
    }

    #[test]
    fn distinct_sources_in_one_common_cause_group_are_not_independent() {
        let (mut batch, mut proposal, case) = fixtures();
        let mut second_action = proposal.actions[0].clone();
        second_action.action_id = "request-scada-0002".to_string();
        proposal.actions.push(second_action);
        let mut second_result = batch.results[0].clone();
        second_result.request_id = "request-scada-0002".to_string();
        second_result.evidence[0].evidence_id = "evidence-scada-n3-002".to_string();
        second_result.evidence[0].source_id = "scada-gateway-b".to_string();
        batch.results.push(second_result);
        let mut second_source = batch.sources[0].clone();
        second_source.source_id = "scada-gateway-b".to_string();
        second_source.producer_id = "scada-gateway-b".to_string();
        second_source.producer_attestation_ref = "synthetic://attestation/scada-gateway-b/v1".to_string();
        second_source.producer_attestation_sha256 = "d".repeat(64);
        second_source.source_snapshot_sha256 = "e".repeat(64);
        batch.sources.push(second_source);

        batch.proposal_sha256 = canonical_proposal_sha256(&proposal).unwrap();
        let report = validate_evidence_batch(&batch, &proposal, &case).unwrap();
        assert_eq!(report.dependence_findings.len(), 1);
        assert_eq!(
            report.dependence_findings[0].finding,
            DependenceFindingKind::SharedCommonCauseGroup
        );
    }

    #[test]
    fn no_declared_overlap_is_not_promoted_to_independence() {
        let (mut batch, mut proposal, case) = fixtures();
        let mut second_action = proposal.actions[0].clone();
        second_action.action_id = "request-scada-0002".to_string();
        proposal.actions.push(second_action);
        let mut second_result = batch.results[0].clone();
        second_result.request_id = "request-scada-0002".to_string();
        second_result.evidence[0].evidence_id = "evidence-scada-n3-002".to_string();
        second_result.evidence[0].source_id = "independent-looking-gateway".to_string();
        batch.results.push(second_result);
        let mut second_source = batch.sources[0].clone();
        second_source.source_id = "independent-looking-gateway".to_string();
        second_source.producer_id = "independent-looking-gateway".to_string();
        second_source.producer_attestation_ref = "synthetic://attestation/independent-looking-gateway/v1".to_string();
        second_source.producer_attestation_sha256 = "d".repeat(64);
        second_source.source_snapshot_sha256 = "e".repeat(64);
        second_source.common_cause_group_ids = vec!["different-declared-group".to_string()];
        batch.sources.push(second_source);

        batch.proposal_sha256 = canonical_proposal_sha256(&proposal).unwrap();
        let report = validate_evidence_batch(&batch, &proposal, &case).unwrap();
        assert_eq!(
            report.dependence_findings[0].finding,
            DependenceFindingKind::NoDeclaredOverlapIndependenceUnproven
        );
        assert!(report.dependence_findings[0].interpretation.contains("independence remains unproven"));
    }

    #[test]
    fn producer_attestation_reference_is_never_reported_as_verified() {
        let (batch, proposal, case) = fixtures();
        let report = validate_evidence_batch(&batch, &proposal, &case).unwrap();
        assert_eq!(
            report.evidence_findings[0].producer_attestation_status,
            "reference_present_not_cryptographically_verified"
        );
    }

    #[test]
    fn aggregate_result_cannot_be_available_before_its_evidence() {
        let (mut batch, proposal, case) = fixtures();
        batch.results[0].available_at_unix_ms = Some(2_050);
        assert!(validate_evidence_batch(&batch, &proposal, &case).is_err());
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let (batch, _, _) = fixtures();
        let mut value = serde_json::to_value(batch).unwrap();
        value.as_object_mut().unwrap().insert(
            "unknownField".to_string(),
            serde_json::Value::Bool(true),
        );
        let json = serde_json::to_string(&value).unwrap();
        assert!(serde_json::from_str::<EvidenceBatch>(&json).is_err());
    }
}
