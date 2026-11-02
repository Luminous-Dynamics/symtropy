//! Bounded discovery-claim assessment: prior art, experimental support and
//! independent replication are separate evidence dimensions.
//!
//! A completed literature search can establish only "no prior art found
//! within this declared corpus and search protocol". It cannot prove absolute
//! novelty. Same-artifact replay is not independent reproduction; external
//! replication requires independently identified lab, apparatus and
//! calibration context.

use super::counterexample_ledger::EvidenceSplit;
use std::collections::BTreeSet;

const MAX_ID_BYTES: usize = 256;
const MAX_REPLICATIONS: usize = 128;
const MIN_PRIOR_ART_REVIEWERS: usize = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScientificClaimType {
    EmpiricalAssociation,
    CausalEffect,
    MechanisticExplanation,
    QuantitativePrediction,
    MeasurementResult,
    MethodologicalImprovement,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PreregisteredClaimOutcome {
    Supported,
    Contradicted,
    Inconclusive,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PriorArtAdjudicationOutcome {
    RelevantPriorArt,
    NoRelevantPriorArtWithinCorpus,
    Inconclusive,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PriorArtMatchReference {
    pub reference_digest: String,
    pub relevance_note_digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PriorArtSearchReceipt {
    pub corpus_id: String,
    pub corpus_version: String,
    pub corpus_snapshot_digest: String,
    pub corpus_as_of_unix_seconds: u64,
    pub covered_domains: Vec<String>,
    pub covered_source_types: Vec<String>,
    pub query_bundle_digest: String,
    pub search_transcript_digest: String,
    pub search_started_at_unix_seconds: u64,
    pub search_completed_at_unix_seconds: u64,
    pub query_count: usize,
    pub search_completed: bool,
    pub retrieved_candidate_references: Vec<PriorArtMatchReference>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PriorArtAdjudication {
    pub reviewer_id: String,
    pub reviewed_transcript_digest: String,
    pub outcome: PriorArtAdjudicationOutcome,
    pub rationale_digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PriorArtAssessment {
    pub search: PriorArtSearchReceipt,
    pub adjudications: Vec<PriorArtAdjudication>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscoverySourceContext {
    pub campaign_id: String,
    pub run_id: String,
    pub operator_id: String,
    pub team_id: String,
    pub laboratory_id: String,
    pub apparatus_id: String,
    pub calibration_chain_digest: String,
    pub dataset_digest: String,
    pub raw_observation_digest: String,
    pub seed_namespace_id: String,
    pub task_manifest_digest: String,
    pub source_code_digest: String,
    pub harness_digest: String,
    pub pre_registration_receipt_digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscoveryClaimEvidence {
    pub evidence_split: EvidenceSplit,
    pub claim_result_digest: String,
    pub raw_observation_digest: String,
    pub analysis_receipt_digest: String,
    pub evidence_verification_receipt_digest: String,
    pub pre_registration_receipt_digest: String,
    pub preregistered_outcome: PreregisteredClaimOutcome,
    pub issued_at_unix_seconds: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ReplicationLevel {
    SameArtifactReplay,
    IndependentReproduction,
    ExternalReplication,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReplicationOutcome {
    Replicated,
    NotReplicated,
    Inconclusive,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscoveryReplicationReceipt {
    pub replication_id: String,
    pub level: ReplicationLevel,
    pub outcome: ReplicationOutcome,
    pub source_campaign_id: String,
    pub replication_campaign_id: String,
    pub run_id: String,
    pub operator_id: String,
    pub team_id: String,
    pub laboratory_id: String,
    pub apparatus_id: String,
    pub calibration_chain_digest: String,
    pub dataset_digest: String,
    pub seed_namespace_id: String,
    pub task_manifest_digest: String,
    pub source_code_digest: String,
    pub harness_digest: String,
    pub raw_observation_digest: String,
    pub analysis_receipt_digest: String,
    pub pre_registration_receipt_digest: String,
    pub criterion_receipt_digest: String,
    pub evidence_verification_receipt_digest: String,
    pub issued_at_unix_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscoveryClaimDossier {
    pub claim_id: String,
    pub claim_type: ScientificClaimType,
    /// Digest of the canonical atomic proposition, not a free-form paper title.
    pub canonical_proposition_digest: String,
    pub source_context: DiscoverySourceContext,
    pub source_evidence: DiscoveryClaimEvidence,
    pub prior_art: PriorArtAssessment,
    pub replications: Vec<DiscoveryReplicationReceipt>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PriorArtScopeResult {
    KnownPriorArtFound,
    NoPriorArtFoundWithinDeclaredCorpus,
    PriorArtSearchInconclusive,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StrongestReplicationResult {
    None,
    SameArtifactReplay,
    IndependentReproduction,
    ExternalReplication,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiscoveryClaimDisposition {
    ReproducedKnownFinding,
    KnownFindingWithoutIndependentReproduction,
    ProvisionalNovelCandidate,
    NoPriorArtCandidateNotYetIndependentlyReproduced,
    Contradicted,
    Inconclusive,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplicationOutcomeRecord {
    pub replication_id: String,
    pub level: ReplicationLevel,
    pub outcome: ReplicationOutcome,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscoveryClaimVerificationReceipt {
    pub claim_id: String,
    pub canonical_proposition_digest: String,
    pub prior_art_scope_result: PriorArtScopeResult,
    pub strongest_successful_replication: StrongestReplicationResult,
    pub independent_replication_failures: usize,
    pub external_replication_failures: usize,
    pub inconclusive_replications: usize,
    pub disposition: DiscoveryClaimDisposition,
    pub evidence_split: EvidenceSplit,
    pub prior_art_corpus_snapshot_digest: String,
    pub prior_art_search_transcript_digest: String,
    pub source_claim_result_digest: String,
    pub replication_receipt_ids: Vec<String>,
    pub replication_outcomes: Vec<ReplicationOutcomeRecord>,
    pub absolute_novelty_claim_made: bool,
    pub replication_level_not_overstated: bool,
    pub all_adverse_replication_outcomes_retained: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DiscoveryClaimError {
    InvalidIdentity(String),
    InvalidDigest(String),
    InvalidPriorArtSearch(String),
    PriorArtAdjudication(String),
    InvalidSourceEvidence(String),
    InvalidReplication(String),
    ReplicationIndependence(String),
    DuplicateReplication(String),
    ArithmeticOverflow(String),
}

impl std::fmt::Display for DiscoveryClaimError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidIdentity(reason)
            | Self::InvalidDigest(reason)
            | Self::InvalidPriorArtSearch(reason)
            | Self::PriorArtAdjudication(reason)
            | Self::InvalidSourceEvidence(reason)
            | Self::InvalidReplication(reason)
            | Self::ReplicationIndependence(reason)
            | Self::DuplicateReplication(reason)
            | Self::ArithmeticOverflow(reason) => formatter.write_str(reason),
        }
    }
}

impl std::error::Error for DiscoveryClaimError {}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= MAX_ID_BYTES
}

fn valid_sha256(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64
        && hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn require_id(value: &str, label: &str) -> Result<(), DiscoveryClaimError> {
    if !valid_id(value) {
        return Err(DiscoveryClaimError::InvalidIdentity(format!(
            "{label} is empty or overlong"
        )));
    }
    Ok(())
}

fn require_digest(value: &str, label: &str) -> Result<(), DiscoveryClaimError> {
    if !valid_sha256(value) {
        return Err(DiscoveryClaimError::InvalidDigest(format!(
            "{label} is not a canonical sha256 digest"
        )));
    }
    Ok(())
}

fn validate_source_context(
    context: &DiscoverySourceContext,
) -> Result<(), DiscoveryClaimError> {
    for (value, label) in [
        (&context.campaign_id, "source campaign ID"),
        (&context.run_id, "source run ID"),
        (&context.operator_id, "source operator ID"),
        (&context.team_id, "source team ID"),
        (&context.laboratory_id, "source laboratory ID"),
        (&context.apparatus_id, "source apparatus ID"),
        (&context.seed_namespace_id, "source seed namespace"),
    ] {
        require_id(value, label)?;
    }
    for (value, label) in [
        (&context.calibration_chain_digest, "source calibration chain"),
        (&context.dataset_digest, "source dataset"),
        (&context.raw_observation_digest, "source raw observation"),
        (&context.task_manifest_digest, "source task manifest"),
        (&context.source_code_digest, "source code"),
        (&context.harness_digest, "source harness"),
        (&context.pre_registration_receipt_digest, "source pre-registration receipt"),
    ] {
        require_digest(value, label)?;
    }
    Ok(())
}

fn validate_source_evidence(
    evidence: &DiscoveryClaimEvidence,
    context: &DiscoverySourceContext,
) -> Result<(), DiscoveryClaimError> {
    if evidence.evidence_split != EvidenceSplit::SealedHoldout {
        return Err(DiscoveryClaimError::InvalidSourceEvidence(
            "final discovery-claim dossiers require SealedHoldout evidence".into(),
        ));
    }
    if evidence.pre_registration_receipt_digest != context.pre_registration_receipt_digest {
        return Err(DiscoveryClaimError::InvalidSourceEvidence(
            "source evidence and source context pre-registration receipts differ".into(),
        ));
    }
    if evidence.issued_at_unix_seconds == 0 {
        return Err(DiscoveryClaimError::InvalidSourceEvidence(
            "claim evidence issue time must be positive".into(),
        ));
    }
    for (value, label) in [
        (&evidence.claim_result_digest, "claim result"),
        (&evidence.raw_observation_digest, "source raw observation"),
        (&evidence.analysis_receipt_digest, "source analysis receipt"),
        (&evidence.evidence_verification_receipt_digest, "source evidence verification receipt"),
        (&evidence.pre_registration_receipt_digest, "source pre-registration receipt"),
    ] {
        require_digest(value, label)?;
    }
    Ok(())
}

fn validate_prior_art_assessment(
    assessment: &PriorArtAssessment,
    claim_issued_at: u64,
) -> Result<PriorArtScopeResult, DiscoveryClaimError> {
    let search = &assessment.search;
    for (value, label) in [
        (&search.corpus_id, "prior-art corpus ID"),
        (&search.corpus_version, "prior-art corpus version"),
    ] {
        require_id(value, label)?;
    }
    for (value, label) in [
        (&search.corpus_snapshot_digest, "prior-art corpus snapshot"),
        (&search.query_bundle_digest, "prior-art query bundle"),
        (&search.search_transcript_digest, "prior-art search transcript"),
    ] {
        require_digest(value, label)?;
    }
    if search.corpus_as_of_unix_seconds == 0
        || search.corpus_as_of_unix_seconds > claim_issued_at
        || search.search_started_at_unix_seconds == 0
        || search.search_completed_at_unix_seconds < search.search_started_at_unix_seconds
        || search.search_completed_at_unix_seconds > claim_issued_at
        || search.query_count == 0
        || search.covered_domains.is_empty()
        || search.covered_source_types.is_empty()
        || search
            .covered_domains
            .iter()
            .chain(search.covered_source_types.iter())
            .any(|value| !valid_id(value))
    {
        return Err(DiscoveryClaimError::InvalidPriorArtSearch(
            "prior-art search lacks a valid frozen date, search interval, query budget, domain or source coverage".into(),
        ));
    }
    for reference in &search.retrieved_candidate_references {
        require_digest(&reference.reference_digest, "prior-art candidate reference")?;
        require_digest(&reference.relevance_note_digest, "prior-art candidate relevance note")?;
    }

    let mut reviewers = BTreeSet::new();
    let mut outcomes = BTreeSet::new();
    for adjudication in &assessment.adjudications {
        require_id(&adjudication.reviewer_id, "prior-art reviewer ID")?;
        require_digest(&adjudication.reviewed_transcript_digest, "reviewed prior-art transcript")?;
        require_digest(&adjudication.rationale_digest, "prior-art adjudication rationale")?;
        if adjudication.reviewed_transcript_digest != search.search_transcript_digest
            || !reviewers.insert(adjudication.reviewer_id.clone())
        {
            return Err(DiscoveryClaimError::PriorArtAdjudication(
                "prior-art reviewers must be distinct and review the frozen search transcript".into(),
            ));
        }
        outcomes.insert(adjudication.outcome);
    }

    if !search.search_completed || reviewers.len() < MIN_PRIOR_ART_REVIEWERS {
        return Ok(PriorArtScopeResult::PriorArtSearchInconclusive);
    }
    if outcomes.len() != 1 || outcomes.contains(&PriorArtAdjudicationOutcome::Inconclusive) {
        return Ok(PriorArtScopeResult::PriorArtSearchInconclusive);
    }
    match outcomes.first().copied() {
        Some(PriorArtAdjudicationOutcome::RelevantPriorArt) => {
            Ok(PriorArtScopeResult::KnownPriorArtFound)
        }
        Some(PriorArtAdjudicationOutcome::NoRelevantPriorArtWithinCorpus) => {
            Ok(PriorArtScopeResult::NoPriorArtFoundWithinDeclaredCorpus)
        }
        Some(PriorArtAdjudicationOutcome::Inconclusive) | None => {
            Ok(PriorArtScopeResult::PriorArtSearchInconclusive)
        }
    }
}

fn validate_replication(
    replication: &DiscoveryReplicationReceipt,
    source: &DiscoverySourceContext,
) -> Result<(), DiscoveryClaimError> {
    for (value, label) in [
        (&replication.replication_id, "replication ID"),
        (&replication.replication_campaign_id, "replication campaign ID"),
        (&replication.run_id, "replication run ID"),
        (&replication.operator_id, "replication operator ID"),
        (&replication.team_id, "replication team ID"),
        (&replication.laboratory_id, "replication laboratory ID"),
        (&replication.apparatus_id, "replication apparatus ID"),
        (&replication.seed_namespace_id, "replication seed namespace"),
    ] {
        require_id(value, label)?;
    }
    for (value, label) in [
        (&replication.calibration_chain_digest, "replication calibration chain"),
        (&replication.dataset_digest, "replication dataset"),
        (&replication.task_manifest_digest, "replication task manifest"),
        (&replication.source_code_digest, "replication source code"),
        (&replication.harness_digest, "replication harness"),
        (&replication.raw_observation_digest, "replication raw observation"),
        (&replication.analysis_receipt_digest, "replication analysis receipt"),
        (&replication.pre_registration_receipt_digest, "replication pre-registration receipt"),
        (&replication.criterion_receipt_digest, "replication criterion receipt"),
        (&replication.evidence_verification_receipt_digest, "replication evidence verification receipt"),
    ] {
        require_digest(value, label)?;
    }
    if replication.issued_at_unix_seconds == 0
        || replication.source_campaign_id != source.campaign_id
        || replication.replication_campaign_id == source.campaign_id
    {
        return Err(DiscoveryClaimError::InvalidReplication(
            "replication must reference the source campaign, use a distinct campaign and have a positive issue time".into(),
        ));
    }

    match replication.level {
        ReplicationLevel::SameArtifactReplay => {
            if replication.dataset_digest != source.dataset_digest
                || replication.task_manifest_digest != source.task_manifest_digest
                || replication.source_code_digest != source.source_code_digest
                || replication.harness_digest != source.harness_digest
                || replication.raw_observation_digest != source.raw_observation_digest
            {
                return Err(DiscoveryClaimError::ReplicationIndependence(
                    "SameArtifactReplay must use the source data, task manifest, code and harness identities".into(),
                ));
            }
        }
        ReplicationLevel::IndependentReproduction => {
            if replication.run_id == source.run_id
                || replication.operator_id == source.operator_id
                || replication.team_id == source.team_id
                || replication.dataset_digest == source.dataset_digest
                || replication.seed_namespace_id == source.seed_namespace_id
                || replication.pre_registration_receipt_digest
                    == source.pre_registration_receipt_digest
                || replication.raw_observation_digest == source.raw_observation_digest
            {
                return Err(DiscoveryClaimError::ReplicationIndependence(
                    "IndependentReproduction requires a distinct run, operator/team, data, seed namespace and pre-registration".into(),
                ));
            }
        }
        ReplicationLevel::ExternalReplication => {
            if replication.run_id == source.run_id
                || replication.operator_id == source.operator_id
                || replication.team_id == source.team_id
                || replication.laboratory_id == source.laboratory_id
                || replication.apparatus_id == source.apparatus_id
                || replication.calibration_chain_digest == source.calibration_chain_digest
                || replication.dataset_digest == source.dataset_digest
                || replication.seed_namespace_id == source.seed_namespace_id
                || replication.pre_registration_receipt_digest
                    == source.pre_registration_receipt_digest
                || replication.raw_observation_digest == source.dataset_digest
            {
                return Err(DiscoveryClaimError::ReplicationIndependence(
                    "ExternalReplication requires distinct run, operator/team, laboratory, apparatus, calibration, data, seed namespace and pre-registration".into(),
                ));
            }
        }
    }
    Ok(())
}

/// Verify a discovery claim dossier and produce bounded, non-overstated results.
///
/// The verifier validates declarations and relationships; it does not
/// authenticate human identities, laboratory affiliations, corpus completeness,
/// calibration records or physical truth. Those require trusted attestation
/// providers and domain-expert adjudication.
pub fn verify_discovery_claim_dossier(
    dossier: &DiscoveryClaimDossier,
) -> Result<DiscoveryClaimVerificationReceipt, DiscoveryClaimError> {
    require_id(&dossier.claim_id, "claim ID")?;
    require_digest(&dossier.canonical_proposition_digest, "canonical proposition")?;
    validate_source_context(&dossier.source_context)?;
    validate_source_evidence(&dossier.source_evidence, &dossier.source_context)?;

    let prior_art_scope_result = validate_prior_art_assessment(
        &dossier.prior_art,
        dossier.source_evidence.issued_at_unix_seconds,
    )?;

    if dossier.replications.len() > MAX_REPLICATIONS {
        return Err(DiscoveryClaimError::InvalidReplication(format!(
            "replication count exceeds configured cap of {MAX_REPLICATIONS}"
        )));
    }
    let mut replication_ids = BTreeSet::new();
    let mut strongest_successful_replication = StrongestReplicationResult::None;
    let mut independent_replication_failures = 0usize;
    let mut external_replication_failures = 0usize;
    let mut inconclusive_replications = 0usize;
    let mut replication_receipt_ids = Vec::with_capacity(dossier.replications.len());
    let mut replication_outcomes = Vec::with_capacity(dossier.replications.len());

    for replication in &dossier.replications {
        if !replication_ids.insert(replication.replication_id.clone()) {
            return Err(DiscoveryClaimError::DuplicateReplication(
                "replication IDs must be unique".into(),
            ));
        }
        validate_replication(replication, &dossier.source_context)?;
        replication_receipt_ids.push(replication.replication_id.clone());
        replication_outcomes.push(ReplicationOutcomeRecord {
            replication_id: replication.replication_id.clone(),
            level: replication.level,
            outcome: replication.outcome,
        });
        match replication.outcome {
            ReplicationOutcome::Replicated => {
                let level = match replication.level {
                    ReplicationLevel::SameArtifactReplay => StrongestReplicationResult::SameArtifactReplay,
                    ReplicationLevel::IndependentReproduction => StrongestReplicationResult::IndependentReproduction,
                    ReplicationLevel::ExternalReplication => StrongestReplicationResult::ExternalReplication,
                };
                if level > strongest_successful_replication {
                    strongest_successful_replication = level;
                }
            }
            ReplicationOutcome::NotReplicated => match replication.level {
                ReplicationLevel::SameArtifactReplay => {}
                ReplicationLevel::IndependentReproduction => {
                    independent_replication_failures = independent_replication_failures.checked_add(1).ok_or_else(|| {
                        DiscoveryClaimError::ArithmeticOverflow("independent failure count overflow".into())
                    })?;
                }
                ReplicationLevel::ExternalReplication => {
                    external_replication_failures = external_replication_failures.checked_add(1).ok_or_else(|| {
                        DiscoveryClaimError::ArithmeticOverflow("external failure count overflow".into())
                    })?;
                }
            },
            ReplicationOutcome::Inconclusive => {
                inconclusive_replications = inconclusive_replications.checked_add(1).ok_or_else(|| {
                    DiscoveryClaimError::ArithmeticOverflow("inconclusive replication count overflow".into())
                })?;
            }
        }
    }
    replication_receipt_ids.sort();
    replication_outcomes.sort_by(|left, right| {
        left.replication_id.cmp(&right.replication_id)
    });

    let disposition = match dossier.source_evidence.preregistered_outcome {
        PreregisteredClaimOutcome::Contradicted => DiscoveryClaimDisposition::Contradicted,
        PreregisteredClaimOutcome::Inconclusive => DiscoveryClaimDisposition::Inconclusive,
        PreregisteredClaimOutcome::Supported => match prior_art_scope_result {
            PriorArtScopeResult::KnownPriorArtFound => {
                if strongest_successful_replication >= StrongestReplicationResult::IndependentReproduction {
                    DiscoveryClaimDisposition::ReproducedKnownFinding
                } else {
                    DiscoveryClaimDisposition::KnownFindingWithoutIndependentReproduction
                }
            }
            PriorArtScopeResult::NoPriorArtFoundWithinDeclaredCorpus => {
                if strongest_successful_replication >= StrongestReplicationResult::IndependentReproduction {
                    DiscoveryClaimDisposition::ProvisionalNovelCandidate
                } else {
                    DiscoveryClaimDisposition::NoPriorArtCandidateNotYetIndependentlyReproduced
                }
            }
            PriorArtScopeResult::PriorArtSearchInconclusive => DiscoveryClaimDisposition::Inconclusive,
        },
    };

    Ok(DiscoveryClaimVerificationReceipt {
        claim_id: dossier.claim_id.clone(),
        canonical_proposition_digest: dossier.canonical_proposition_digest.clone(),
        prior_art_scope_result,
        strongest_successful_replication,
        independent_replication_failures,
        external_replication_failures,
        inconclusive_replications,
        disposition,
        evidence_split: dossier.source_evidence.evidence_split,
        prior_art_corpus_snapshot_digest: dossier.prior_art.search.corpus_snapshot_digest.clone(),
        prior_art_search_transcript_digest: dossier.prior_art.search.search_transcript_digest.clone(),
        source_claim_result_digest: dossier.source_evidence.claim_result_digest.clone(),
        replication_receipt_ids,
        replication_outcomes,
        absolute_novelty_claim_made: false,
        replication_level_not_overstated: true,
        all_adverse_replication_outcomes_retained: true,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        DiscoveryClaimDossier, DiscoveryClaimDisposition, DiscoveryClaimEvidence,
        DiscoveryReplicationReceipt, DiscoverySourceContext, PriorArtAdjudication,
        PriorArtAdjudicationOutcome, PriorArtAssessment, PriorArtMatchReference,
        PriorArtSearchReceipt, PreregisteredClaimOutcome, PriorArtScopeResult,
        ReplicationLevel, ReplicationOutcome, StrongestReplicationResult,
        verify_discovery_claim_dossier,
    };
    use super::super::counterexample_ledger::EvidenceSplit;

    const D1: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const D2: &str = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const D3: &str = "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const D4: &str = "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
    const D5: &str = "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";

    fn source() -> DiscoverySourceContext {
        DiscoverySourceContext {
            campaign_id: "source-campaign".into(),
            run_id: "source-run".into(),
            operator_id: "source-operator".into(),
            team_id: "source-team".into(),
            laboratory_id: "source-lab".into(),
            apparatus_id: "source-apparatus".into(),
            calibration_chain_digest: D1.into(),
            dataset_digest: D2.into(),
            raw_observation_digest: D5.into(),
            seed_namespace_id: "source-seed".into(),
            task_manifest_digest: D3.into(),
            source_code_digest: D4.into(),
            harness_digest: D5.into(),
            pre_registration_receipt_digest: D1.into(),
        }
    }

    fn evidence(outcome: PreregisteredClaimOutcome) -> DiscoveryClaimEvidence {
        DiscoveryClaimEvidence {
            evidence_split: EvidenceSplit::SealedHoldout,
            claim_result_digest: D2.into(),
            raw_observation_digest: D5.into(),
            analysis_receipt_digest: D3.into(),
            evidence_verification_receipt_digest: D4.into(),
            pre_registration_receipt_digest: D1.into(),
            preregistered_outcome: outcome,
            issued_at_unix_seconds: 1_000,
        }
    }

    fn prior_art(outcome: PriorArtAdjudicationOutcome) -> PriorArtAssessment {
        let search = PriorArtSearchReceipt {
            corpus_id: "prior-art-corpus".into(),
            corpus_version: "v2026-10-01".into(),
            corpus_snapshot_digest: D1.into(),
            corpus_as_of_unix_seconds: 900,
            covered_domains: vec!["materials-science".into()],
            covered_source_types: vec!["peer-reviewed".into(), "preprint".into()],
            query_bundle_digest: D2.into(),
            search_transcript_digest: D3.into(),
            search_started_at_unix_seconds: 950,
            search_completed_at_unix_seconds: 980,
            query_count: 12,
            search_completed: true,
            retrieved_candidate_references: vec![PriorArtMatchReference {
                reference_digest: D4.into(),
                relevance_note_digest: D5.into(),
            }],
        };
        PriorArtAssessment {
            search: search.clone(),
            adjudications: vec![
                PriorArtAdjudication {
                    reviewer_id: "reviewer-a".into(),
                    reviewed_transcript_digest: search.search_transcript_digest.clone(),
                    outcome,
                    rationale_digest: D4.into(),
                },
                PriorArtAdjudication {
                    reviewer_id: "reviewer-b".into(),
                    reviewed_transcript_digest: search.search_transcript_digest.clone(),
                    outcome,
                    rationale_digest: D5.into(),
                },
            ],
        }
    }

    fn independent_replication(outcome: ReplicationOutcome) -> DiscoveryReplicationReceipt {
        DiscoveryReplicationReceipt {
            replication_id: "independent-rep-1".into(),
            level: ReplicationLevel::IndependentReproduction,
            outcome,
            source_campaign_id: "source-campaign".into(),
            replication_campaign_id: "replication-campaign".into(),
            run_id: "replication-run".into(),
            operator_id: "replication-operator".into(),
            team_id: "replication-team".into(),
            laboratory_id: "source-lab".into(),
            apparatus_id: "source-apparatus".into(),
            calibration_chain_digest: D1.into(),
            dataset_digest: D4.into(),
            seed_namespace_id: "replication-seed".into(),
            task_manifest_digest: D3.into(),
            source_code_digest: D4.into(),
            harness_digest: D5.into(),
            raw_observation_digest: D2.into(),
            analysis_receipt_digest: D3.into(),
            pre_registration_receipt_digest: D5.into(),
            criterion_receipt_digest: D1.into(),
            evidence_verification_receipt_digest: D2.into(),
            issued_at_unix_seconds: 2_000,
        }
    }

    fn external_replication(outcome: ReplicationOutcome) -> DiscoveryReplicationReceipt {
        let mut receipt = independent_replication(outcome);
        receipt.replication_id = "external-rep-1".into();
        receipt.level = ReplicationLevel::ExternalReplication;
        receipt.laboratory_id = "external-lab".into();
        receipt.apparatus_id = "external-apparatus".into();
        receipt.calibration_chain_digest = D4.into();
        receipt
    }

    fn dossier(
        prior_art_outcome: PriorArtAdjudicationOutcome,
        replications: Vec<DiscoveryReplicationReceipt>,
    ) -> DiscoveryClaimDossier {
        DiscoveryClaimDossier {
            claim_id: "claim-1".into(),
            claim_type: super::ScientificClaimType::MechanisticExplanation,
            canonical_proposition_digest: D1.into(),
            source_context: source(),
            source_evidence: evidence(PreregisteredClaimOutcome::Supported),
            prior_art: prior_art(prior_art_outcome),
            replications,
        }
    }

    #[test]
    fn known_prior_art_plus_external_replication_is_a_reproduced_known_finding() {
        let result = verify_discovery_claim_dossier(&dossier(
            PriorArtAdjudicationOutcome::RelevantPriorArt,
            vec![external_replication(ReplicationOutcome::Replicated)],
        ))
        .unwrap();
        assert_eq!(result.prior_art_scope_result, PriorArtScopeResult::KnownPriorArtFound);
        assert_eq!(result.strongest_successful_replication, StrongestReplicationResult::ExternalReplication);
        assert_eq!(result.disposition, DiscoveryClaimDisposition::ReproducedKnownFinding);
        assert!(!result.absolute_novelty_claim_made);
    }

    #[test]
    fn no_prior_art_plus_independent_reproduction_is_only_a_provisional_novel_candidate() {
        let result = verify_discovery_claim_dossier(&dossier(
            PriorArtAdjudicationOutcome::NoRelevantPriorArtWithinCorpus,
            vec![independent_replication(ReplicationOutcome::Replicated)],
        ))
        .unwrap();
        assert_eq!(result.prior_art_scope_result, PriorArtScopeResult::NoPriorArtFoundWithinDeclaredCorpus);
        assert_eq!(result.disposition, DiscoveryClaimDisposition::ProvisionalNovelCandidate);
        assert!(!result.absolute_novelty_claim_made);
    }

    #[test]
    fn same_artifact_replay_does_not_upgrade_to_independent_reproduction() {
        let mut replay = independent_replication(ReplicationOutcome::Replicated);
        replay.level = ReplicationLevel::SameArtifactReplay;
        replay.replication_id = "replay-1".into();
        replay.dataset_digest = D2.into();
        replay.raw_observation_digest = D5.into();
        replay.task_manifest_digest = D3.into();
        replay.source_code_digest = D4.into();
        replay.harness_digest = D5.into();
        let result = verify_discovery_claim_dossier(&dossier(
            PriorArtAdjudicationOutcome::NoRelevantPriorArtWithinCorpus,
            vec![replay],
        ))
        .unwrap();
        assert_eq!(result.strongest_successful_replication, StrongestReplicationResult::SameArtifactReplay);
        assert_eq!(result.disposition, DiscoveryClaimDisposition::NoPriorArtCandidateNotYetIndependentlyReproduced);
    }

    #[test]
    fn incomplete_prior_art_search_cannot_support_novelty_even_with_replication() {
        let mut dossier = dossier(
            PriorArtAdjudicationOutcome::NoRelevantPriorArtWithinCorpus,
            vec![independent_replication(ReplicationOutcome::Replicated)],
        );
        dossier.prior_art.search.search_completed = false;
        let result = verify_discovery_claim_dossier(&dossier).unwrap();
        assert_eq!(result.prior_art_scope_result, PriorArtScopeResult::PriorArtSearchInconclusive);
        assert_eq!(result.disposition, DiscoveryClaimDisposition::Inconclusive);
    }

    #[test]
    fn distinct_independent_reproduction_failures_are_retained() {
        let result = verify_discovery_claim_dossier(&dossier(
            PriorArtAdjudicationOutcome::NoRelevantPriorArtWithinCorpus,
            vec![
                independent_replication(ReplicationOutcome::NotReplicated),
                external_replication(ReplicationOutcome::Inconclusive),
            ],
        ))
        .unwrap();
        assert_eq!(result.independent_replication_failures, 1);
        assert_eq!(result.inconclusive_replications, 1);
        assert_eq!(result.disposition, DiscoveryClaimDisposition::NoPriorArtCandidateNotYetIndependentlyReproduced);
        assert!(result.all_adverse_replication_outcomes_retained);
    }

    #[test]
    fn independent_reproduction_rejects_same_operator_dataset_or_seed() {
        let mut receipt = independent_replication(ReplicationOutcome::Replicated);
        receipt.operator_id = "source-operator".into();
        assert!(verify_discovery_claim_dossier(&dossier(
            PriorArtAdjudicationOutcome::NoRelevantPriorArtWithinCorpus,
            vec![receipt],
        )).is_err());

        let mut receipt = independent_replication(ReplicationOutcome::Replicated);
        receipt.dataset_digest = D2.into();
        assert!(verify_discovery_claim_dossier(&dossier(
            PriorArtAdjudicationOutcome::NoRelevantPriorArtWithinCorpus,
            vec![receipt],
        )).is_err());

        let mut receipt = independent_replication(ReplicationOutcome::Replicated);
        receipt.seed_namespace_id = "source-seed".into();
        assert!(verify_discovery_claim_dossier(&dossier(
            PriorArtAdjudicationOutcome::NoRelevantPriorArtWithinCorpus,
            vec![receipt],
        )).is_err());
    }

    #[test]
    fn external_replication_requires_distinct_lab_apparatus_and_calibration() {
        let mut receipt = external_replication(ReplicationOutcome::Replicated);
        receipt.laboratory_id = "source-lab".into();
        assert!(verify_discovery_claim_dossier(&dossier(
            PriorArtAdjudicationOutcome::RelevantPriorArt,
            vec![receipt],
        )).is_err());

        let mut receipt = external_replication(ReplicationOutcome::Replicated);
        receipt.apparatus_id = "source-apparatus".into();
        assert!(verify_discovery_claim_dossier(&dossier(
            PriorArtAdjudicationOutcome::RelevantPriorArt,
            vec![receipt],
        )).is_err());

        let mut receipt = external_replication(ReplicationOutcome::Replicated);
        receipt.calibration_chain_digest = D1.into();
        assert!(verify_discovery_claim_dossier(&dossier(
            PriorArtAdjudicationOutcome::RelevantPriorArt,
            vec![receipt],
        )).is_err());
    }

    #[test]
    fn requires_sealed_evidence_and_consistent_independent_prior_art_reviews() {
        let mut dossier = dossier(
            PriorArtAdjudicationOutcome::NoRelevantPriorArtWithinCorpus,
            vec![independent_replication(ReplicationOutcome::Replicated)],
        );
        dossier.source_evidence.evidence_split = EvidenceSplit::Development;
        assert!(verify_discovery_claim_dossier(&dossier).is_err());

        let mut dossier = dossier(
            PriorArtAdjudicationOutcome::NoRelevantPriorArtWithinCorpus,
            vec![independent_replication(ReplicationOutcome::Replicated)],
        );
        dossier.prior_art.adjudications[1].reviewer_id = dossier.prior_art.adjudications[0].reviewer_id.clone();
        assert!(verify_discovery_claim_dossier(&dossier).is_err());

        let mut dossier = dossier(
            PriorArtAdjudicationOutcome::NoRelevantPriorArtWithinCorpus,
            vec![independent_replication(ReplicationOutcome::Replicated)],
        );
        dossier.prior_art.adjudications[1].outcome = PriorArtAdjudicationOutcome::RelevantPriorArt;
        let result = verify_discovery_claim_dossier(&dossier).unwrap();
        assert_eq!(result.prior_art_scope_result, PriorArtScopeResult::PriorArtSearchInconclusive);
        assert_eq!(result.disposition, DiscoveryClaimDisposition::Inconclusive);
    }

    #[test]
    fn contradicted_source_claim_stays_contradicted_even_when_a_replay_succeeds() {
        let mut dossier = dossier(
            PriorArtAdjudicationOutcome::RelevantPriorArt,
            vec![independent_replication(ReplicationOutcome::Replicated)],
        );
        dossier.source_evidence.preregistered_outcome = PreregisteredClaimOutcome::Contradicted;
        let result = verify_discovery_claim_dossier(&dossier).unwrap();
        assert_eq!(result.disposition, DiscoveryClaimDisposition::Contradicted);
    }
}
