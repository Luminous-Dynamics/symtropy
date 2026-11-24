use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

pub mod dependency_graph_adequacy;
pub mod incumbent_adequacy;
pub mod invalidation;
pub mod policy_adequacy;

const SCHEMA: &str = "luminous.orbital-assurance-bundle.corpus.v0.1";
const CLAIM_CEILING: &str = "synthetic-assurance-graph-structure-only";

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ClaimType {
    OrbitalReviewBundle,
    LifecycleQualification,
    TestMatrixAdequacy,
    EventHistoryIntegrity,
    MissionSuccessProbability,
    ConstantFailureIntensity,
    PowerLawRecurrentIntensity,
    WeibullFirstFailureLife,
    PhysicalTestObservation,
    MeasurementCalibration,
    ModelCredibility,
    EvidenceCustody,
    ExternalAuthorityDecision,
    Defeater,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClaimState {
    Supported,
    Contradicted,
    Indeterminate,
    Stale,
    Withdrawn,
    Unsupported,
    OpenDefeater,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceClass {
    Synthetic,
    Software,
    Bench,
    Physical,
    Authority,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Relation {
    Supports,
    DerivedFrom,
    Contradicts,
    Supersedes,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    ReviewBundleComplete,
    IncompleteArgument,
    BlockedByOpenDefeater,
    BlockedByUnsupportedOrStaleEvidence,
    BlockedByScopeMismatch,
    BlockedByClaimTypeMismatch,
    BlockedByMissingAuthority,
    Contradicted,
    Indeterminate,
    InvalidGraph,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Scope {
    pub subject_id: String,
    pub configuration_id: String,
    pub operating_mode_id: String,
    pub mission_profile_id: String,
    pub environment_profile_id: String,
    pub valid_from: i64,
    pub valid_until: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EvidenceRef {
    pub receipt_id: String,
    pub source_repo: String,
    pub source_path: String,
    pub source_commit: String,
    pub receipt_schema: String,
    pub receipt_sha256: String,
    pub evidence_class: EvidenceClass,
    pub recorded_at: i64,
    pub effective_from: i64,
    pub effective_until: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReceiptPin {
    pub receipt_id: String,
    pub source_repo: String,
    pub source_path: String,
    pub source_commit: String,
    pub receipt_schema: String,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Claim {
    pub claim_id: String,
    pub claim_type: ClaimType,
    pub state: ClaimState,
    pub scope: Scope,
    pub estimand: Option<String>,
    pub unit: Option<String>,
    pub model_profile: Option<String>,
    pub validation_domain: Option<String>,
    pub mandatory: bool,
    pub evidence: Option<EvidenceRef>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Edge {
    pub from_claim_id: String,
    pub to_claim_id: String,
    pub relation: Relation,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Expected {
    pub disposition: Disposition,
    pub blocker_count: usize,
    pub authority_granted: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub case_id: String,
    pub root_claim_id: String,
    pub scope: Scope,
    pub effective_cutoff: i64,
    pub knowledge_cutoff: i64,
    pub required_claim_types: Vec<ClaimType>,
    pub receipt_pins: Vec<ReceiptPin>,
    pub claims: Vec<Claim>,
    pub edges: Vec<Edge>,
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

#[derive(Debug, Clone, Serialize)]
pub struct Blocker {
    pub code: String,
    pub claim_id: Option<String>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaseResult {
    pub case_id: String,
    pub disposition: Disposition,
    pub claim_count: usize,
    pub edge_count: usize,
    pub blocker_count: usize,
    pub blockers: Vec<Blocker>,
    pub authority_granted: bool,
    pub expected_result_matched: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub case_count: usize,
    pub review_bundle_complete: usize,
    pub incomplete_argument: usize,
    pub blocked_by_open_defeater: usize,
    pub blocked_by_unsupported_or_stale_evidence: usize,
    pub blocked_by_scope_mismatch: usize,
    pub blocked_by_claim_type_mismatch: usize,
    pub blocked_by_missing_authority: usize,
    pub contradicted: usize,
    pub indeterminate: usize,
    pub invalid_graph: usize,
    pub blocker_count: usize,
    pub expected_result_mismatches: usize,
    pub authority_grants: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub result: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub authority_granted: bool,
    pub summary: Summary,
    pub cases: Vec<CaseResult>,
}

fn valid_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn evidence_matches_pin(evidence: &EvidenceRef, pin: &ReceiptPin) -> bool {
    evidence.receipt_id == pin.receipt_id
        && evidence.source_repo == pin.source_repo
        && evidence.source_path == pin.source_path
        && evidence.source_commit == pin.source_commit
        && evidence.receipt_schema == pin.receipt_schema
        && evidence.receipt_sha256 == pin.receipt_sha256
}

fn is_statistical_profile_claim(claim_type: ClaimType) -> bool {
    matches!(
        claim_type,
        ClaimType::MissionSuccessProbability
            | ClaimType::ConstantFailureIntensity
            | ClaimType::PowerLawRecurrentIntensity
            | ClaimType::WeibullFirstFailureLife
    )
}

fn claim_model_is_typed(claim: &Claim) -> bool {
    match claim.claim_type {
        ClaimType::MissionSuccessProbability => {
            claim.estimand.as_deref() == Some("mission_success_probability")
                && claim.unit.as_deref() == Some("probability")
                && claim.model_profile.as_deref() == Some("bernoulli_single_mission")
        }
        ClaimType::ConstantFailureIntensity => {
            claim.estimand.as_deref() == Some("constant_failure_intensity")
                && claim.unit.as_deref() == Some("events_per_million_operating_hours")
                && claim.model_profile.as_deref() == Some("homogeneous_poisson_constant_intensity")
        }
        ClaimType::PowerLawRecurrentIntensity => {
            claim.estimand.as_deref() == Some("power_law_recurrent_failure_intensity")
                && claim.unit.as_deref() == Some("events_per_operating_second")
                && claim.model_profile.as_deref() == Some("power_law_nhpp_minimal_repair_v0_1")
        }
        ClaimType::WeibullFirstFailureLife => {
            claim.estimand.as_deref() == Some("first_failure_life_distribution")
                && claim.unit.as_deref() == Some("operating_seconds")
                && claim.model_profile.as_deref() == Some("weibull_first_failure_life_v0_1")
        }
        _ => true,
    }
}

fn is_root_support_edge(edge: &Edge, root_id: &str) -> bool {
    edge.relation == Relation::Supports && edge.to_claim_id == root_id
}

fn add_blocker(
    blockers: &mut Vec<Blocker>,
    code: &str,
    claim_id: Option<&str>,
    detail: impl Into<String>,
) {
    blockers.push(Blocker {
        code: code.to_owned(),
        claim_id: claim_id.map(str::to_owned),
        detail: detail.into(),
    });
}

fn graph_has_cycle(claim_ids: &HashSet<&str>, edges: &[Edge]) -> bool {
    let mut adjacency: HashMap<&str, Vec<&str>> = HashMap::new();
    for edge in edges {
        if edge.relation != Relation::Contradicts {
            adjacency
                .entry(edge.from_claim_id.as_str())
                .or_default()
                .push(edge.to_claim_id.as_str());
        }
    }

    fn visit<'a>(
        node: &'a str,
        adjacency: &HashMap<&'a str, Vec<&'a str>>,
        marks: &mut HashMap<&'a str, u8>,
    ) -> bool {
        match marks.get(node).copied().unwrap_or(0) {
            1 => return true,
            2 => return false,
            _ => {}
        }
        marks.insert(node, 1);
        if let Some(next) = adjacency.get(node) {
            for child in next {
                if visit(child, adjacency, marks) {
                    return true;
                }
            }
        }
        marks.insert(node, 2);
        false
    }

    let mut marks = HashMap::new();
    claim_ids
        .iter()
        .any(|node| visit(node, &adjacency, &mut marks))
}

fn disposition_for_blockers(blockers: &[Blocker]) -> Disposition {
    let codes: BTreeSet<&str> = blockers.iter().map(|item| item.code.as_str()).collect();
    if codes.contains("GRAPH_INVALID") {
        Disposition::InvalidGraph
    } else if codes.contains("CLAIM_TYPE_MISMATCH") {
        Disposition::BlockedByClaimTypeMismatch
    } else if codes.contains("SCOPE_MISMATCH") {
        Disposition::BlockedByScopeMismatch
    } else if codes.contains("MISSING_AUTHORITY") {
        Disposition::BlockedByMissingAuthority
    } else if codes.contains("CONTRADICTED") {
        Disposition::Contradicted
    } else if codes.contains("UNSUPPORTED_OR_STALE") {
        Disposition::BlockedByUnsupportedOrStaleEvidence
    } else if codes.contains("INDETERMINATE") {
        Disposition::Indeterminate
    } else if codes.contains("OPEN_DEFEATER") {
        Disposition::BlockedByOpenDefeater
    } else if codes.contains("INCOMPLETE") {
        Disposition::IncompleteArgument
    } else {
        Disposition::ReviewBundleComplete
    }
}

fn evaluate_case(case: &Case) -> CaseResult {
    let mut blockers = Vec::new();
    if case.case_id.trim().is_empty() || case.root_claim_id.trim().is_empty() {
        add_blocker(
            &mut blockers,
            "GRAPH_INVALID",
            None,
            "case and root IDs must be non-empty",
        );
    }
    if case.scope.subject_id.trim().is_empty()
        || case.scope.configuration_id.trim().is_empty()
        || case.scope.operating_mode_id.trim().is_empty()
        || case.scope.mission_profile_id.trim().is_empty()
        || case.scope.environment_profile_id.trim().is_empty()
        || case.scope.valid_from > case.scope.valid_until
        || case.effective_cutoff < case.scope.valid_from
        || case.effective_cutoff > case.scope.valid_until
    {
        add_blocker(
            &mut blockers,
            "SCOPE_MISMATCH",
            None,
            "invalid or out-of-window case scope",
        );
    }
    if case.knowledge_cutoff < case.effective_cutoff {
        add_blocker(
            &mut blockers,
            "INDETERMINATE",
            None,
            "knowledge cutoff precedes effective cutoff",
        );
    }

    let mut claims_by_id = BTreeMap::new();
    let mut claim_ids = HashSet::new();
    for claim in &case.claims {
        if claim.claim_id.trim().is_empty() || !claim_ids.insert(claim.claim_id.as_str()) {
            add_blocker(
                &mut blockers,
                "GRAPH_INVALID",
                Some(&claim.claim_id),
                "duplicate or empty claim ID",
            );
            continue;
        }
        claims_by_id.insert(claim.claim_id.as_str(), claim);
    }

    let Some(root) = claims_by_id.get(case.root_claim_id.as_str()) else {
        add_blocker(&mut blockers, "INCOMPLETE", None, "root claim is missing");
        let disposition = disposition_for_blockers(&blockers);
        return CaseResult {
            case_id: case.case_id.clone(),
            disposition,
            claim_count: case.claims.len(),
            edge_count: case.edges.len(),
            blocker_count: blockers.len(),
            blockers,
            authority_granted: false,
            expected_result_matched: false,
        };
    };

    if root.claim_type != ClaimType::OrbitalReviewBundle || root.evidence.is_some() {
        add_blocker(
            &mut blockers,
            "CLAIM_TYPE_MISMATCH",
            Some(&root.claim_id),
            "root must be an unevidenced orbital_review_bundle claim",
        );
    }

    let mut pin_by_id: HashMap<&str, &ReceiptPin> = HashMap::new();
    for pin in &case.receipt_pins {
        if pin.receipt_id.trim().is_empty()
            || !valid_hex(&pin.source_commit, 40)
            || !valid_hex(&pin.receipt_sha256, 64)
            || pin.source_repo.trim().is_empty()
            || pin.source_path.trim().is_empty()
            || pin.receipt_schema.trim().is_empty()
            || pin_by_id.insert(&pin.receipt_id, pin).is_some()
        {
            add_blocker(
                &mut blockers,
                "GRAPH_INVALID",
                None,
                "duplicate or malformed receipt pin",
            );
        }
    }

    let mut type_counts: HashMap<ClaimType, usize> = HashMap::new();
    let mut missing_authority = false;
    for claim in &case.claims {
        *type_counts.entry(claim.claim_type).or_default() += 1;
        if claim.claim_id == case.root_claim_id {
            continue;
        }

        if claim.scope != case.scope {
            add_blocker(
                &mut blockers,
                "SCOPE_MISMATCH",
                Some(&claim.claim_id),
                "claim scope differs from the case's exact subject/configuration/profile/time scope",
            );
        }

        if !claim_model_is_typed(claim) {
            add_blocker(
                &mut blockers,
                "CLAIM_TYPE_MISMATCH",
                Some(&claim.claim_id),
                "estimand, unit and model profile do not match the claim type",
            );
        }
        if claim.claim_type == ClaimType::ModelCredibility
            && claim
                .validation_domain
                .as_deref()
                .is_none_or(|domain| domain.trim().is_empty())
        {
            add_blocker(
                &mut blockers,
                "INCOMPLETE",
                Some(&claim.claim_id),
                "model credibility claim lacks a declared validation domain",
            );
        }

        if claim.state != ClaimState::Supported {
            let (code, detail) = match claim.state {
                ClaimState::Contradicted => ("CONTRADICTED", "claim is contradicted"),
                ClaimState::Indeterminate => ("INDETERMINATE", "claim state is indeterminate"),
                ClaimState::OpenDefeater => ("OPEN_DEFEATER", "mandatory defeater remains open"),
                ClaimState::Stale | ClaimState::Withdrawn | ClaimState::Unsupported => (
                    "UNSUPPORTED_OR_STALE",
                    "claim is stale, withdrawn or unsupported",
                ),
                ClaimState::Supported => unreachable!(),
            };
            add_blocker(&mut blockers, code, Some(&claim.claim_id), detail);
        }

        if claim.claim_type == ClaimType::ExternalAuthorityDecision && claim.mandatory {
            missing_authority |= claim.state != ClaimState::Supported;
        }

        match &claim.evidence {
            None => add_blocker(
                &mut blockers,
                "INCOMPLETE",
                Some(&claim.claim_id),
                "claim has no exact receipt reference",
            ),
            Some(evidence) => {
                let class_matches = match claim.claim_type {
                    ClaimType::PhysicalTestObservation | ClaimType::MeasurementCalibration => {
                        matches!(
                            evidence.evidence_class,
                            EvidenceClass::Physical | EvidenceClass::Bench
                        )
                    }
                    ClaimType::EvidenceCustody => {
                        matches!(
                            evidence.evidence_class,
                            EvidenceClass::Software | EvidenceClass::Authority
                        )
                    }
                    ClaimType::ExternalAuthorityDecision => {
                        evidence.evidence_class == EvidenceClass::Authority
                    }
                    _ => true,
                };
                if !class_matches {
                    add_blocker(
                        &mut blockers,
                        "CLAIM_TYPE_MISMATCH",
                        Some(&claim.claim_id),
                        "evidence class is incompatible with this claim type",
                    );
                }
                if evidence.receipt_id.trim().is_empty()
                    || evidence.source_repo.trim().is_empty()
                    || evidence.source_path.trim().is_empty()
                    || evidence.receipt_schema.trim().is_empty()
                    || !valid_hex(&evidence.source_commit, 40)
                    || !valid_hex(&evidence.receipt_sha256, 64)
                {
                    add_blocker(
                        &mut blockers,
                        "INCOMPLETE",
                        Some(&claim.claim_id),
                        "receipt reference is malformed",
                    );
                } else if evidence.effective_from > case.effective_cutoff
                    || evidence.effective_until < case.effective_cutoff
                    || evidence.recorded_at > case.knowledge_cutoff
                {
                    add_blocker(
                        &mut blockers,
                        "UNSUPPORTED_OR_STALE",
                        Some(&claim.claim_id),
                        "receipt is not current at the declared effective/knowledge cutoffs",
                    );
                } else if let Some(pin) = pin_by_id.get(evidence.receipt_id.as_str()) {
                    if !evidence_matches_pin(evidence, pin) {
                        add_blocker(
                            &mut blockers,
                            "UNSUPPORTED_OR_STALE",
                            Some(&claim.claim_id),
                            "claim receipt does not match the exact case-level pin",
                        );
                    }
                } else {
                    add_blocker(
                        &mut blockers,
                        "INCOMPLETE",
                        Some(&claim.claim_id),
                        "receipt is not included in the case-level pin set",
                    );
                }
            }
        }
    }

    let mut unique_required = HashSet::new();
    for required_type in &case.required_claim_types {
        if !unique_required.insert(*required_type) {
            add_blocker(
                &mut blockers,
                "GRAPH_INVALID",
                None,
                "required claim type is listed more than once",
            );
        }
        if *required_type == ClaimType::OrbitalReviewBundle {
            add_blocker(
                &mut blockers,
                "CLAIM_TYPE_MISMATCH",
                Some(&case.root_claim_id),
                "root claim type cannot be declared as a required evidence dependency",
            );
        } else if type_counts.get(required_type).copied().unwrap_or(0) == 0 {
            add_blocker(
                &mut blockers,
                if *required_type == ClaimType::ExternalAuthorityDecision {
                    "MISSING_AUTHORITY"
                } else {
                    "INCOMPLETE"
                },
                None,
                format!("required claim type {required_type:?} is missing"),
            );
            missing_authority |= *required_type == ClaimType::ExternalAuthorityDecision;
        } else if type_counts.get(required_type).copied().unwrap_or(0) > 1 {
            add_blocker(
                &mut blockers,
                "GRAPH_INVALID",
                None,
                format!("required claim type {required_type:?} appears more than once"),
            );
        }
    }

    let mut edge_set = HashSet::new();
    for edge in &case.edges {
        if !claims_by_id.contains_key(edge.from_claim_id.as_str())
            || !claims_by_id.contains_key(edge.to_claim_id.as_str())
            || edge.from_claim_id == edge.to_claim_id
            || !edge_set.insert((
                edge.from_claim_id.as_str(),
                edge.to_claim_id.as_str(),
                edge.relation,
            ))
        {
            add_blocker(
                &mut blockers,
                "GRAPH_INVALID",
                None,
                "edge references missing claim, self-loop or duplicate edge",
            );
            continue;
        }

        let from = claims_by_id[edge.from_claim_id.as_str()];
        let to = claims_by_id[edge.to_claim_id.as_str()];
        match edge.relation {
            Relation::Contradicts => {
                add_blocker(
                    &mut blockers,
                    "CONTRADICTED",
                    Some(&to.claim_id),
                    format!("claim is contradicted by {}", from.claim_id),
                );
            }
            Relation::Supports if from.claim_type == ClaimType::OrbitalReviewBundle => {
                add_blocker(
                    &mut blockers,
                    "GRAPH_INVALID",
                    Some(&from.claim_id),
                    "support edges point from evidence claim to the claim it supports",
                );
            }
            Relation::Supports if to.claim_type != ClaimType::OrbitalReviewBundle => {
                add_blocker(
                    &mut blockers,
                    "GRAPH_INVALID",
                    Some(&from.claim_id),
                    "v0.1 permits direct support edges only into the review root",
                );
            }
            Relation::DerivedFrom => {
                let incompatible = from.claim_type != to.claim_type
                    && is_statistical_profile_claim(from.claim_type)
                    && is_statistical_profile_claim(to.claim_type);
                if incompatible {
                    add_blocker(
                        &mut blockers,
                        "CLAIM_TYPE_MISMATCH",
                        Some(&from.claim_id),
                        "mission-success probability and constant failure intensity are non-interchangeable estimands",
                    );
                }
            }
            _ => {}
        }
    }

    for claim in case
        .claims
        .iter()
        .filter(|claim| claim.claim_id != case.root_claim_id)
    {
        if !case.edges.iter().any(|edge| {
            edge.from_claim_id == claim.claim_id && is_root_support_edge(edge, &case.root_claim_id)
        }) {
            add_blocker(
                &mut blockers,
                "INCOMPLETE",
                Some(&claim.claim_id),
                "claim is not connected to the review root",
            );
        }
    }

    for required_type in &case.required_claim_types {
        for claim in case
            .claims
            .iter()
            .filter(|claim| claim.claim_type == *required_type)
        {
            if !claim.mandatory {
                add_blocker(
                    &mut blockers,
                    "INCOMPLETE",
                    Some(&claim.claim_id),
                    "required claim is not marked mandatory",
                );
            }
        }
    }

    if graph_has_cycle(&claim_ids, &case.edges) {
        add_blocker(
            &mut blockers,
            "GRAPH_INVALID",
            None,
            "claim dependency graph contains a cycle",
        );
    }

    if (case.claims.iter().any(|claim| {
        claim.claim_type == ClaimType::ExternalAuthorityDecision
            && claim.mandatory
            && claim.state != ClaimState::Supported
    }) || missing_authority)
        && !blockers
            .iter()
            .any(|blocker| blocker.code == "MISSING_AUTHORITY")
    {
        add_blocker(
            &mut blockers,
            "MISSING_AUTHORITY",
            None,
            "required external authority decision is absent or not current",
        );
    }

    blockers.sort_by(|left, right| {
        (&left.code, &left.claim_id, &left.detail).cmp(&(
            &right.code,
            &right.claim_id,
            &right.detail,
        ))
    });
    blockers.dedup_by(|left, right| {
        left.code == right.code && left.claim_id == right.claim_id && left.detail == right.detail
    });

    let disposition = disposition_for_blockers(&blockers);
    let authority_granted = false;
    let expected_result_matched = disposition == case.expected.disposition
        && blockers.len() == case.expected.blocker_count
        && !case.expected.authority_granted
        && !authority_granted;

    CaseResult {
        case_id: case.case_id.clone(),
        disposition,
        claim_count: case.claims.len(),
        edge_count: case.edges.len(),
        blocker_count: blockers.len(),
        blockers,
        authority_granted,
        expected_result_matched,
    }
}

pub fn evaluate(corpus: &Corpus) -> Result<Report, String> {
    if corpus.schema != SCHEMA {
        return Err("unsupported assurance-bundle corpus schema".to_owned());
    }
    if corpus.evidence_class != "synthetic" {
        return Err("v0.1 accepts synthetic assurance-bundle fixtures only".to_owned());
    }
    if corpus.claim_ceiling != CLAIM_CEILING {
        return Err("unsupported assurance-bundle claim ceiling".to_owned());
    }
    if corpus.cases.is_empty() {
        return Err("assurance-bundle corpus must contain cases".to_owned());
    }

    let mut case_ids = HashSet::new();
    let mut summary = Summary {
        case_count: corpus.cases.len(),
        review_bundle_complete: 0,
        incomplete_argument: 0,
        blocked_by_open_defeater: 0,
        blocked_by_unsupported_or_stale_evidence: 0,
        blocked_by_scope_mismatch: 0,
        blocked_by_claim_type_mismatch: 0,
        blocked_by_missing_authority: 0,
        contradicted: 0,
        indeterminate: 0,
        invalid_graph: 0,
        blocker_count: 0,
        expected_result_mismatches: 0,
        authority_grants: 0,
    };
    let mut results = Vec::with_capacity(corpus.cases.len());
    for case in &corpus.cases {
        if !case_ids.insert(case.case_id.as_str()) {
            return Err("assurance-bundle case IDs must be unique".to_owned());
        }
        let result = evaluate_case(case);
        match result.disposition {
            Disposition::ReviewBundleComplete => summary.review_bundle_complete += 1,
            Disposition::IncompleteArgument => summary.incomplete_argument += 1,
            Disposition::BlockedByOpenDefeater => summary.blocked_by_open_defeater += 1,
            Disposition::BlockedByUnsupportedOrStaleEvidence => {
                summary.blocked_by_unsupported_or_stale_evidence += 1
            }
            Disposition::BlockedByScopeMismatch => summary.blocked_by_scope_mismatch += 1,
            Disposition::BlockedByClaimTypeMismatch => summary.blocked_by_claim_type_mismatch += 1,
            Disposition::BlockedByMissingAuthority => summary.blocked_by_missing_authority += 1,
            Disposition::Contradicted => summary.contradicted += 1,
            Disposition::Indeterminate => summary.indeterminate += 1,
            Disposition::InvalidGraph => summary.invalid_graph += 1,
        }
        summary.blocker_count += result.blocker_count;
        summary.expected_result_mismatches += usize::from(!result.expected_result_matched);
        summary.authority_grants += usize::from(result.authority_granted);
        results.push(result);
    }

    Ok(Report {
        result: if summary.expected_result_mismatches == 0 && summary.authority_grants == 0 {
            "pass".to_owned()
        } else {
            "fail".to_owned()
        },
        evidence_class: "synthetic".to_owned(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
        authority_granted: false,
        summary,
        cases: results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mission_and_intensity_estimands_are_distinct() {
        let mission = Claim {
            claim_id: "mission".to_owned(),
            claim_type: ClaimType::MissionSuccessProbability,
            state: ClaimState::Supported,
            scope: Scope {
                subject_id: "a".to_owned(),
                configuration_id: "c".to_owned(),
                operating_mode_id: "m".to_owned(),
                mission_profile_id: "mission-v1".to_owned(),
                environment_profile_id: "env-v1".to_owned(),
                valid_from: 0,
                valid_until: 10,
            },
            estimand: Some("mission_success_probability".to_owned()),
            unit: Some("probability".to_owned()),
            model_profile: Some("bernoulli_single_mission".to_owned()),
            validation_domain: None,
            mandatory: true,
            evidence: None,
        };
        assert!(claim_model_is_typed(&mission));
    }

    #[test]
    fn power_law_and_weibull_profiles_are_separate_statistical_claims() {
        assert!(is_statistical_profile_claim(
            ClaimType::PowerLawRecurrentIntensity
        ));
        assert!(is_statistical_profile_claim(
            ClaimType::WeibullFirstFailureLife
        ));
        assert!(!is_statistical_profile_claim(
            ClaimType::LifecycleQualification
        ));

        let power_law = Claim {
            claim_id: "plp".to_owned(),
            claim_type: ClaimType::PowerLawRecurrentIntensity,
            state: ClaimState::Supported,
            scope: Scope {
                subject_id: "a".to_owned(),
                configuration_id: "c".to_owned(),
                operating_mode_id: "m".to_owned(),
                mission_profile_id: "p".to_owned(),
                environment_profile_id: "env".to_owned(),
                valid_from: 0,
                valid_until: 10,
            },
            estimand: Some("power_law_recurrent_failure_intensity".to_owned()),
            unit: Some("events_per_operating_second".to_owned()),
            model_profile: Some("power_law_nhpp_minimal_repair_v0_1".to_owned()),
            validation_domain: Some("repairable-minimal-repair-domain".to_owned()),
            mandatory: true,
            evidence: None,
        };
        assert!(claim_model_is_typed(&power_law));
        let mut mismatched = power_law.clone();
        mismatched.model_profile = Some("homogeneous_poisson_constant_intensity".to_owned());
        assert!(!claim_model_is_typed(&mismatched));
    }

    #[test]
    fn zero_digest_and_commit_are_not_accepted() {
        assert!(!valid_hex(&"a".repeat(63), 64));
        assert!(!valid_hex(&"A".repeat(64), 64));
        assert!(!valid_hex(&"a".repeat(39), 40));
    }

    #[test]
    fn constant_intensity_requires_its_own_unit_and_model() {
        let claim = Claim {
            claim_id: "intensity".to_owned(),
            claim_type: ClaimType::ConstantFailureIntensity,
            state: ClaimState::Supported,
            scope: Scope {
                subject_id: "a".to_owned(),
                configuration_id: "c".to_owned(),
                operating_mode_id: "m".to_owned(),
                mission_profile_id: "p".to_owned(),
                environment_profile_id: "env-v1".to_owned(),
                valid_from: 0,
                valid_until: 10,
            },
            estimand: Some("constant_failure_intensity".to_owned()),
            unit: Some("probability".to_owned()),
            model_profile: Some("bernoulli_single_mission".to_owned()),
            validation_domain: None,
            mandatory: true,
            evidence: None,
        };
        assert!(!claim_model_is_typed(&claim));
    }
}
