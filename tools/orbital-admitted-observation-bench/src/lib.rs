use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const CORPUS_SCHEMA: &str = "luminous.orbital-admitted-observation.corpus.v0.1";
pub const RECEIPT_SCHEMA: &str = "luminous.orbital-admitted-observation.receipt.v0.1";
pub const BENCHMARK_ID: &str = "ORBITAL-ADMITTED-OBSERVATION-001";
pub const CLAIM_CEILING: &str = "synthetic_source_bound_attempt_outcome_composition_only";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ScopeDisposition {
    ScopeCoherent,
    OutOfScope,
    Indeterminate,
    InvalidInput,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OwnerKind {
    Capture,
    Timing,
    Motion,
    Calibration,
    Geometry,
    Feature,
    Metrology,
    Photometry,
    Diversity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GateState {
    Pass,
    Fail,
    Unknown,
    NotRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SourceState {
    Current,
    Stale,
    Revoked,
    Contradicted,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ActionRealization {
    Verified,
    NotRealized,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Disposition {
    MeasurementAdmissible,
    ModelledObservationProcessOutcome,
    AbstainUnmodelledOutcome,
    InadmissibleMeasurement,
    InvalidInput,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Cutoff {
    pub cutoff_id: String,
    pub sequence_watermark: u64,
    pub history_max_sequence: u64,
    pub history_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Action {
    pub action_id: String,
    pub policy_revision: String,
    pub command_receipt_sha256: String,
    pub realization: ActionRealization,
    pub realized_action_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct KernelProfile {
    pub profile_id: String,
    pub profile_revision: String,
    pub receipt_sha256: String,
    pub source_state: SourceState,
    pub allowed_outcomes: Vec<String>,
    pub required_owners: Vec<OwnerKind>,
    pub missingness_semantics_declared: bool,
    pub action_realization_required: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Outcome {
    pub kind: String,
    pub outcome_record_sha256: String,
    pub payload_sha256: Option<String>,
    pub channel_ids: Vec<String>,
    pub missing_channel_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerBinding {
    pub owner: OwnerKind,
    pub required: bool,
    pub state: GateState,
    pub source_state: SourceState,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub case_id: String,
    pub campaign_id: String,
    pub attempt_id: String,
    pub sequence_no: u64,
    pub campaign_attempt_count: u64,
    pub scope_disposition: ScopeDisposition,
    pub scope_receipt_sha256: String,
    pub cutoff: Cutoff,
    pub action: Action,
    pub kernel: KernelProfile,
    pub outcome: Outcome,
    pub owner_bindings: Vec<OwnerBinding>,
    pub expected_disposition: Disposition,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    pub schema: String,
    pub benchmark_id: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub cases: Vec<Case>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OwnerResult {
    pub owner: OwnerKind,
    pub required: bool,
    pub state: GateState,
    pub source_state: SourceState,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CaseResult {
    pub case_id: String,
    pub attempt_id: String,
    pub disposition: Disposition,
    pub sequential_update_permitted: bool,
    pub public_outcome_kind: String,
    pub missing_channel_ids: Vec<String>,
    pub owner_results: Vec<OwnerResult>,
    pub reason_codes: Vec<String>,
    pub cutoff_sha256: String,
    pub scope_receipt_sha256: String,
    pub outcome_record_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Receipt {
    pub schema: String,
    pub benchmark_id: String,
    pub source_commit: String,
    pub input_sha256: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub case_count: usize,
    pub expected_disposition_matches: usize,
    pub expected_disposition_mismatches: usize,
    pub authority_granted: bool,
    pub statistical_update_performed: bool,
    pub likelihood_or_eprocess_generated: bool,
    pub physical_validation: String,
    pub cases: Vec<CaseResult>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn valid_text(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_measurement_outcome(kind: &str) -> bool {
    matches!(
        kind,
        "MEASUREMENT_AVAILABLE" | "NO_INDICATION" | "INDICATION_REPORTED"
    )
}

fn required_failure_owner(kind: &str) -> Option<(OwnerKind, GateState)> {
    match kind {
        "OCCLUDED" => Some((OwnerKind::Geometry, GateState::Fail)),
        "CAPTURE_DROPPED" => Some((OwnerKind::Capture, GateState::Fail)),
        "TRIGGER_OR_FRAME_UNASSOCIATED" => Some((OwnerKind::Timing, GateState::Fail)),
        "SENSOR_SATURATED" | "PHOTOMETRY_OUT_OF_PROFILE" => {
            Some((OwnerKind::Photometry, GateState::Fail))
        }
        "MOTION_BUDGET_EXCEEDED" => Some((OwnerKind::Motion, GateState::Fail)),
        "CALIBRATION_INVALID" => Some((OwnerKind::Calibration, GateState::Fail)),
        "FEATURE_NOT_IDENTIFIABLE" => Some((OwnerKind::Feature, GateState::Fail)),
        "METROLOGY_INDETERMINATE" => Some((OwnerKind::Metrology, GateState::Unknown)),
        _ => None,
    }
}

fn invalid_case(case: &Case, reason: &str) -> CaseResult {
    CaseResult {
        case_id: case.case_id.clone(),
        attempt_id: case.attempt_id.clone(),
        disposition: Disposition::InvalidInput,
        sequential_update_permitted: false,
        public_outcome_kind: case.outcome.kind.clone(),
        missing_channel_ids: case.outcome.missing_channel_ids.clone(),
        owner_results: Vec::new(),
        reason_codes: vec![reason.to_owned()],
        cutoff_sha256: case.cutoff.history_sha256.clone(),
        scope_receipt_sha256: case.scope_receipt_sha256.clone(),
        outcome_record_sha256: case.outcome.outcome_record_sha256.clone(),
    }
}

fn abstain_case(case: &Case, reason: &str, owners: Vec<OwnerResult>) -> CaseResult {
    CaseResult {
        case_id: case.case_id.clone(),
        attempt_id: case.attempt_id.clone(),
        disposition: Disposition::AbstainUnmodelledOutcome,
        sequential_update_permitted: false,
        public_outcome_kind: case.outcome.kind.clone(),
        missing_channel_ids: {
            let mut ids = case.outcome.missing_channel_ids.clone();
            ids.sort();
            ids
        },
        owner_results: owners,
        reason_codes: vec![reason.to_owned()],
        cutoff_sha256: case.cutoff.history_sha256.clone(),
        scope_receipt_sha256: case.scope_receipt_sha256.clone(),
        outcome_record_sha256: case.outcome.outcome_record_sha256.clone(),
    }
}

fn inadmissible_case(case: &Case, reason: &str, owners: Vec<OwnerResult>) -> CaseResult {
    CaseResult {
        case_id: case.case_id.clone(),
        attempt_id: case.attempt_id.clone(),
        disposition: Disposition::InadmissibleMeasurement,
        sequential_update_permitted: false,
        public_outcome_kind: case.outcome.kind.clone(),
        missing_channel_ids: {
            let mut ids = case.outcome.missing_channel_ids.clone();
            ids.sort();
            ids
        },
        owner_results: owners,
        reason_codes: vec![reason.to_owned()],
        cutoff_sha256: case.cutoff.history_sha256.clone(),
        scope_receipt_sha256: case.scope_receipt_sha256.clone(),
        outcome_record_sha256: case.outcome.outcome_record_sha256.clone(),
    }
}

fn validate_case_shape(case: &Case) -> Result<(), String> {
    if !valid_text(&case.case_id)
        || !valid_text(&case.campaign_id)
        || !valid_text(&case.attempt_id)
        || case.sequence_no == 0
        || case.campaign_attempt_count == 0
        || case.sequence_no > case.campaign_attempt_count
    {
        return Err("ATTEMPT_IDENTITY_INVALID".to_owned());
    }
    if !valid_text(&case.cutoff.cutoff_id)
        || !valid_sha256(&case.cutoff.history_sha256)
        || case.cutoff.sequence_watermark != case.sequence_no
        || case.cutoff.history_max_sequence >= case.sequence_no
    {
        return Err("CUTOFF_OR_NO_LOOKAHEAD_INVALID".to_owned());
    }
    if !valid_sha256(&case.scope_receipt_sha256) {
        return Err("SCOPE_RECEIPT_DIGEST_INVALID".to_owned());
    }
    if !valid_text(&case.action.action_id)
        || !valid_text(&case.action.policy_revision)
        || !valid_sha256(&case.action.command_receipt_sha256)
    {
        return Err("ACTION_IDENTITY_INVALID".to_owned());
    }
    if !valid_text(&case.kernel.profile_id)
        || !valid_text(&case.kernel.profile_revision)
        || !valid_sha256(&case.kernel.receipt_sha256)
        || case.kernel.allowed_outcomes.is_empty()
        || case.kernel.required_owners.is_empty()
    {
        return Err("KERNEL_PROFILE_INVALID".to_owned());
    }
    let mut kernel_required_owners = BTreeSet::new();
    for owner in &case.kernel.required_owners {
        if !kernel_required_owners.insert(*owner) {
            return Err("KERNEL_REQUIRED_OWNER_DUPLICATE".to_owned());
        }
    }
    if !valid_text(&case.outcome.kind) || !valid_sha256(&case.outcome.outcome_record_sha256) {
        return Err("OUTCOME_RECORD_INVALID".to_owned());
    }
    if case
        .outcome
        .payload_sha256
        .as_ref()
        .is_some_and(|digest| !valid_sha256(digest))
    {
        return Err("OUTCOME_PAYLOAD_DIGEST_INVALID".to_owned());
    }

    let mut allowed_outcomes = BTreeSet::new();
    for outcome in &case.kernel.allowed_outcomes {
        if !valid_text(outcome) || !allowed_outcomes.insert(outcome) {
            return Err("KERNEL_OUTCOME_DUPLICATE_OR_EMPTY".to_owned());
        }
    }

    let mut channel_ids = BTreeSet::new();
    for channel in &case.outcome.channel_ids {
        if !valid_text(channel) || !channel_ids.insert(channel) {
            return Err("OBSERVED_CHANNEL_DUPLICATE_OR_EMPTY".to_owned());
        }
    }
    let mut missing_ids = BTreeSet::new();
    for channel in &case.outcome.missing_channel_ids {
        if !valid_text(channel) || !missing_ids.insert(channel) {
            return Err("MISSING_CHANNEL_DUPLICATE_OR_EMPTY".to_owned());
        }
        if channel_ids.contains(channel) {
            return Err("CHANNEL_BOTH_OBSERVED_AND_MISSING".to_owned());
        }
    }
    if channel_ids.is_empty() && missing_ids.is_empty() {
        return Err("OUTCOME_HAS_NO_CHANNELS".to_owned());
    }

    if case.action.realization == ActionRealization::Verified {
        if case.action.realized_action_id.as_deref() != Some(case.action.action_id.as_str()) {
            return Err("REALIZED_ACTION_ID_MISMATCH".to_owned());
        }
    } else if case.action.realized_action_id.is_some() {
        return Err("UNVERIFIED_ACTION_HAS_REALIZED_ID".to_owned());
    }

    let mut owners = BTreeSet::new();
    for binding in &case.owner_bindings {
        if !owners.insert(binding.owner) {
            return Err("OWNER_BINDING_DUPLICATE".to_owned());
        }
        if !valid_sha256(&binding.receipt_sha256) {
            return Err("OWNER_RECEIPT_DIGEST_INVALID".to_owned());
        }
        let profile_requires_owner = kernel_required_owners.contains(&binding.owner);
        if binding.required != profile_requires_owner {
            return Err("OWNER_REQUIRED_FLAG_DOES_NOT_MATCH_KERNEL_PROFILE".to_owned());
        }
        if profile_requires_owner && binding.state == GateState::NotRequired {
            return Err("REQUIRED_OWNER_MARKED_NOT_REQUIRED".to_owned());
        }
        if !profile_requires_owner && binding.state != GateState::NotRequired {
            return Err("OPTIONAL_OWNER_HAS_GATE_STATE".to_owned());
        }
    }
    for owner in &kernel_required_owners {
        if !owners.contains(owner) {
            return Err(format!("OWNER_BINDING_MISSING:{owner:?}").to_ascii_uppercase());
        }
    }

    Ok(())
}

fn owner_map(case: &Case) -> BTreeMap<OwnerKind, &OwnerBinding> {
    case.owner_bindings
        .iter()
        .map(|binding| (binding.owner, binding))
        .collect()
}

fn owner_results(case: &Case) -> Vec<OwnerResult> {
    let mut results: Vec<OwnerResult> = case
        .owner_bindings
        .iter()
        .map(|binding| OwnerResult {
            owner: binding.owner,
            required: binding.required,
            state: binding.state,
            source_state: binding.source_state,
            receipt_sha256: binding.receipt_sha256.clone(),
        })
        .collect();
    results.sort_by_key(|binding| binding.owner);
    results
}

fn evaluate_case(case: &Case) -> CaseResult {
    if let Err(reason) = validate_case_shape(case) {
        return invalid_case(case, &reason);
    }

    let owners = owner_results(case);
    if case.kernel.source_state != SourceState::Current {
        return abstain_case(case, "KERNEL_PROFILE_SOURCE_NOT_CURRENT", owners);
    }
    if case.scope_disposition == ScopeDisposition::InvalidInput {
        return invalid_case(case, "SCOPE_RECEIPT_INVALID");
    }
    if case.scope_disposition == ScopeDisposition::OutOfScope {
        return inadmissible_case(case, "ACQUISITION_SCOPE_OUT_OF_SCOPE", owners);
    }
    if case.scope_disposition == ScopeDisposition::Indeterminate {
        return abstain_case(case, "ACQUISITION_SCOPE_INDETERMINATE", owners);
    }

    let owner_map = owner_map(case);
    if let Some(binding) = owner_map
        .values()
        .find(|binding| binding.source_state != SourceState::Current)
    {
        return abstain_case(
            case,
            &format!("OWNER_SOURCE_NOT_CURRENT:{:?}", binding.owner).to_ascii_uppercase(),
            owners,
        );
    }

    let allowed = case
        .kernel
        .allowed_outcomes
        .iter()
        .any(|outcome| outcome == &case.outcome.kind);
    if !allowed {
        return abstain_case(case, "OUTCOME_NOT_IN_FROZEN_KERNEL_SUPPORT", owners);
    }

    if is_measurement_outcome(&case.outcome.kind) {
        if case.outcome.payload_sha256.is_none()
            || case.outcome.channel_ids.is_empty()
            || !case.outcome.missing_channel_ids.is_empty()
        {
            return invalid_case(case, "MEASUREMENT_OUTCOME_PAYLOAD_OR_CHANNELS_INVALID");
        }
        if case.action.realization != ActionRealization::Verified
            && case.kernel.action_realization_required
        {
            return inadmissible_case(case, "ACTION_REALIZATION_NOT_VERIFIED", owners);
        }
        if let Some(binding) = owner_map
            .values()
            .find(|binding| binding.required && binding.state == GateState::Fail)
        {
            return inadmissible_case(
                case,
                &format!("REQUIRED_OWNER_GATE_FAILED:{:?}", binding.owner).to_ascii_uppercase(),
                owners,
            );
        }
        if let Some(binding) = owner_map
            .values()
            .find(|binding| binding.required && binding.state == GateState::Unknown)
        {
            return abstain_case(
                case,
                &format!("REQUIRED_OWNER_GATE_UNKNOWN:{:?}", binding.owner).to_ascii_uppercase(),
                owners,
            );
        }
        return CaseResult {
            case_id: case.case_id.clone(),
            attempt_id: case.attempt_id.clone(),
            disposition: Disposition::MeasurementAdmissible,
            sequential_update_permitted: true,
            public_outcome_kind: case.outcome.kind.clone(),
            missing_channel_ids: Vec::new(),
            owner_results: owners,
            reason_codes: Vec::new(),
            cutoff_sha256: case.cutoff.history_sha256.clone(),
            scope_receipt_sha256: case.scope_receipt_sha256.clone(),
            outcome_record_sha256: case.outcome.outcome_record_sha256.clone(),
        };
    }

    if case.outcome.kind == "SCOPE_MISMATCH" {
        return inadmissible_case(case, "OUTCOME_SCOPE_MISMATCH", owners);
    }
    if !case.kernel.missingness_semantics_declared {
        return abstain_case(case, "KERNEL_MISSINGNESS_SEMANTICS_NOT_DECLARED", owners);
    }

    if case.outcome.kind == "ACTION_NOT_REALIZED" {
        if case.action.realization != ActionRealization::NotRealized {
            return invalid_case(case, "ACTION_NOT_REALIZED_OUTCOME_CONTRADICTS_READBACK");
        }
    } else if case.action.realization != ActionRealization::Verified
        && case.kernel.action_realization_required
    {
        return abstain_case(case, "ACTION_REALIZATION_UNRESOLVED", owners);
    }

    if let Some((owner, expected_state)) = required_failure_owner(&case.outcome.kind) {
        let Some(binding) = owner_map.get(&owner) else {
            return invalid_case(case, "FAILURE_OWNER_BINDING_MISSING");
        };
        if binding.state == GateState::Unknown || binding.source_state != SourceState::Current {
            return abstain_case(case, "FAILURE_OWNER_STATE_UNRESOLVED", owners);
        }
        if binding.state != expected_state {
            return invalid_case(case, "OUTCOME_KIND_CONTRADICTS_OWNER_GATE");
        }
    } else if case.outcome.kind != "ACTION_NOT_REALIZED"
        && case.outcome.kind != "OTHER_DECLARED_FAILURE"
    {
        return abstain_case(case, "OUTCOME_KIND_UNRECOGNIZED", owners);
    } else if case.outcome.kind == "OTHER_DECLARED_FAILURE"
        && !case
            .owner_bindings
            .iter()
            .any(|binding| binding.required && binding.state == GateState::Fail)
    {
        return invalid_case(case, "OTHER_FAILURE_HAS_NO_FAILED_OWNER_GATE");
    }

    CaseResult {
        case_id: case.case_id.clone(),
        attempt_id: case.attempt_id.clone(),
        disposition: Disposition::ModelledObservationProcessOutcome,
        sequential_update_permitted: true,
        public_outcome_kind: case.outcome.kind.clone(),
        missing_channel_ids: {
            let mut ids = case.outcome.missing_channel_ids.clone();
            ids.sort();
            ids
        },
        owner_results: owners,
        reason_codes: Vec::new(),
        cutoff_sha256: case.cutoff.history_sha256.clone(),
        scope_receipt_sha256: case.scope_receipt_sha256.clone(),
        outcome_record_sha256: case.outcome.outcome_record_sha256.clone(),
    }
}

pub fn audit_corpus(bytes: &[u8], source_commit: &str) -> Result<Receipt, String> {
    if source_commit.len() != 40 || !source_commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("SOURCE_COMMIT_MUST_BE_40_HEX_CHARACTERS".to_owned());
    }
    let corpus: Corpus =
        serde_json::from_slice(bytes).map_err(|error| format!("CORPUS_JSON_INVALID: {error}"))?;
    if corpus.schema != CORPUS_SCHEMA
        || corpus.benchmark_id != BENCHMARK_ID
        || corpus.evidence_class != "synthetic"
        || corpus.claim_ceiling != CLAIM_CEILING
        || corpus.cases.is_empty()
        || corpus.cases.len() > 128
    {
        return Err("CORPUS_IDENTITY_OR_CLAIM_CEILING_INVALID".to_owned());
    }

    let mut case_ids = BTreeSet::new();
    let mut attempt_ids = BTreeSet::new();
    let mut campaign_counts = BTreeMap::<String, u64>::new();
    let mut campaign_sequences = BTreeMap::<String, BTreeSet<u64>>::new();
    let mut cases = Vec::new();
    let mut expected_disposition_matches = 0_usize;
    for case in &corpus.cases {
        if !case_ids.insert(&case.case_id) {
            return Err("CASE_ID_DUPLICATE".to_owned());
        }
        if !attempt_ids.insert((&case.campaign_id, &case.attempt_id)) {
            return Err("ATTEMPT_ID_DUPLICATE_WITHIN_CAMPAIGN".to_owned());
        }
        if campaign_counts
            .insert(case.campaign_id.clone(), case.campaign_attempt_count)
            .is_some_and(|count| count != case.campaign_attempt_count)
        {
            return Err("CAMPAIGN_ATTEMPT_COUNT_CONFLICT".to_owned());
        }
        if !campaign_sequences
            .entry(case.campaign_id.clone())
            .or_default()
            .insert(case.sequence_no)
        {
            return Err("CAMPAIGN_SEQUENCE_DUPLICATE".to_owned());
        }
        let result = evaluate_case(case);
        if result.disposition == case.expected_disposition {
            expected_disposition_matches += 1;
        }
        cases.push(result);
    }
    for (campaign_id, expected_count) in &campaign_counts {
        let sequences = campaign_sequences
            .get(campaign_id)
            .ok_or_else(|| "CAMPAIGN_SEQUENCE_INVENTORY_MISSING".to_owned())?;
        if u64::try_from(sequences.len()).ok() != Some(*expected_count)
            || sequences
                .iter()
                .copied()
                .enumerate()
                .any(|(index, sequence)| {
                    u64::try_from(index)
                        .ok()
                        .and_then(|value| value.checked_add(1))
                        != Some(sequence)
                })
        {
            return Err("CAMPAIGN_ATTEMPT_SEQUENCE_GAP".to_owned());
        }
    }
    cases.sort_by(|left, right| left.case_id.cmp(&right.case_id));

    Ok(Receipt {
        schema: RECEIPT_SCHEMA.to_owned(),
        benchmark_id: BENCHMARK_ID.to_owned(),
        source_commit: source_commit.to_ascii_lowercase(),
        input_sha256: sha256_hex(bytes),
        evidence_class: corpus.evidence_class,
        claim_ceiling: corpus.claim_ceiling,
        case_count: cases.len(),
        expected_disposition_matches,
        expected_disposition_mismatches: cases.len() - expected_disposition_matches,
        authority_granted: false,
        statistical_update_performed: false,
        likelihood_or_eprocess_generated: false,
        physical_validation: "NOT_ASSESSED".to_owned(),
        cases,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus_bytes() -> &'static [u8] {
        include_bytes!(
            "../../../docs/research/orbital-truss/fixtures/admitted-observation-v0_1.json"
        )
    }

    fn corpus() -> Corpus {
        serde_json::from_slice(corpus_bytes()).expect("fixture parses")
    }

    #[test]
    fn frozen_corpus_matches_all_expected_dispositions() {
        let receipt = audit_corpus(corpus_bytes(), "0123456789abcdef0123456789abcdef01234567")
            .expect("corpus audits");
        assert_eq!(receipt.case_count, 32);
        assert_eq!(receipt.expected_disposition_matches, 32);
        assert_eq!(receipt.expected_disposition_mismatches, 0);
        assert!(!receipt.authority_granted);
        assert!(!receipt.statistical_update_performed);
        assert!(!receipt.likelihood_or_eprocess_generated);
    }

    #[test]
    fn valid_measurement_and_no_indication_are_admissible() {
        let input = corpus();
        for id in ["AO-001-MEASUREMENT-AVAILABLE", "AO-002-NO-INDICATION"] {
            let case = input.cases.iter().find(|case| case.case_id == id).unwrap();
            let result = evaluate_case(case);
            assert_eq!(result.disposition, Disposition::MeasurementAdmissible);
            assert!(result.sequential_update_permitted);
        }
    }

    #[test]
    fn modelled_failure_is_recorded_as_outcome_not_measurement() {
        let input = corpus();
        let case = input
            .cases
            .iter()
            .find(|case| case.case_id == "AO-007-MOTION-BUDGET-EXCEEDED")
            .unwrap();
        let result = evaluate_case(case);
        assert_eq!(
            result.disposition,
            Disposition::ModelledObservationProcessOutcome
        );
        assert_ne!(result.disposition, Disposition::MeasurementAdmissible);
        assert!(result.sequential_update_permitted);
    }

    #[test]
    fn failure_absent_from_kernel_abstains_instead_of_coercing() {
        let input = corpus();
        let case = input
            .cases
            .iter()
            .find(|case| case.case_id == "AO-009-OCCLUSION-UNMODELLED")
            .unwrap();
        assert_eq!(
            evaluate_case(case).disposition,
            Disposition::AbstainUnmodelledOutcome
        );
    }

    #[test]
    fn unqualified_measurement_is_never_forwarded() {
        let input = corpus();
        for id in [
            "AO-004-CALIBRATION-INVALID-MEASUREMENT",
            "AO-013-METROLOGY-INDETERMINATE-MEASUREMENT",
            "AO-016-WRONG-REALIZED-ACTION",
        ] {
            let case = input.cases.iter().find(|case| case.case_id == id).unwrap();
            assert!(!evaluate_case(case).sequential_update_permitted);
        }
    }

    #[test]
    fn cutoff_lookahead_and_action_identity_contradictions_are_rejected() {
        let input = corpus();
        let cutoff = input
            .cases
            .iter()
            .find(|case| case.case_id == "AO-017-AFTER-CUTOFF")
            .unwrap();
        let action = input
            .cases
            .iter()
            .find(|case| case.case_id == "AO-016-WRONG-REALIZED-ACTION")
            .unwrap();
        assert_eq!(evaluate_case(cutoff).disposition, Disposition::InvalidInput);
        assert_eq!(evaluate_case(action).disposition, Disposition::InvalidInput);
    }

    #[test]
    fn stale_sources_unknown_outcomes_and_bad_digests_fail_closed() {
        let input = corpus();
        for id in ["AO-021-REVOKED-CALIBRATION", "AO-022-UNKNOWN-OUTCOME"] {
            assert_eq!(
                evaluate_case(input.cases.iter().find(|case| case.case_id == id).unwrap())
                    .disposition,
                Disposition::AbstainUnmodelledOutcome
            );
        }
        for id in ["AO-023-BAD-OUTCOME-DIGEST", "AO-024-DUPLICATE-CHANNEL"] {
            assert_eq!(
                evaluate_case(input.cases.iter().find(|case| case.case_id == id).unwrap()).disposition,
                Disposition::InvalidInput
            );
        }
    }

    #[test]
    fn scope_and_channel_common_cause_are_preserved() {
        let input = corpus();
        let mismatch = input
            .cases
            .iter()
            .find(|case| case.case_id == "AO-019-SCOPE-MISMATCH")
            .unwrap();
        let common = input
            .cases
            .iter()
            .find(|case| case.case_id == "AO-020-JOINT-CHANNEL-FAILURE")
            .unwrap();
        assert_eq!(
            evaluate_case(mismatch).disposition,
            Disposition::InadmissibleMeasurement
        );
        let result = evaluate_case(common);
        assert_eq!(
            result.disposition,
            Disposition::ModelledObservationProcessOutcome
        );
        assert_eq!(result.missing_channel_ids.len(), 2);
    }

    #[test]
    fn kernel_profile_controls_required_owners_and_cannot_be_overridden_by_attempt() {
        let input = corpus();
        let case = input
            .cases
            .iter()
            .find(|case| case.case_id == "AO-026-REQUIRED-OWNER-BYPASS")
            .unwrap();
        let result = evaluate_case(case);
        assert_eq!(result.disposition, Disposition::InvalidInput);
        assert!(
            result
                .reason_codes
                .contains(&"OWNER_REQUIRED_FLAG_DOES_NOT_MATCH_KERNEL_PROFILE".to_owned())
        );
    }

    #[test]
    fn stale_kernel_profile_cannot_admit_an_observation() {
        let input = corpus();
        let case = input
            .cases
            .iter()
            .find(|case| case.case_id == "AO-027-STALE-KERNEL-PROFILE")
            .unwrap();
        assert_eq!(
            evaluate_case(case).disposition,
            Disposition::AbstainUnmodelledOutcome
        );
    }

    #[test]
    fn campaign_attempt_sequence_must_be_complete_and_contiguous() {
        let mut input = corpus();
        input.cases[0].campaign_id = "SHARED-CAMPAIGN".to_owned();
        input.cases[0].campaign_attempt_count = 2;
        input.cases[0].sequence_no = 1;
        input.cases[0].cutoff.sequence_watermark = 1;
        input.cases[0].cutoff.history_max_sequence = 0;

        input.cases[1].campaign_id = "SHARED-CAMPAIGN".to_owned();
        input.cases[1].campaign_attempt_count = 2;
        input.cases[1].sequence_no = 3;
        input.cases[1].cutoff.sequence_watermark = 3;
        input.cases[1].cutoff.history_max_sequence = 2;

        let bytes = serde_json::to_vec(&input).unwrap();
        let error = audit_corpus(&bytes, "0123456789abcdef0123456789abcdef01234567").unwrap_err();
        assert_eq!(error, "CAMPAIGN_ATTEMPT_SEQUENCE_GAP");
    }

    #[test]
    fn missingness_semantics_must_be_declared_for_failures() {
        let input = corpus();
        let case = input
            .cases
            .iter()
            .find(|case| case.case_id == "AO-033-MISSINGNESS-NOT-DECLARED")
            .unwrap();
        assert_eq!(
            evaluate_case(case).disposition,
            Disposition::AbstainUnmodelledOutcome
        );
    }

    #[test]
    fn unknown_required_owner_gate_abstains_without_updating() {
        let input = corpus();
        let case = input
            .cases
            .iter()
            .find(|case| case.case_id == "AO-031-UNKNOWN-MOTION-GATE")
            .unwrap();
        let result = evaluate_case(case);
        assert_eq!(result.disposition, Disposition::AbstainUnmodelledOutcome);
        assert!(!result.sequential_update_permitted);
    }

    #[test]
    fn duplicate_attempt_ids_are_rejected_within_a_campaign() {
        let mut input = corpus();
        input.cases[0].campaign_id = "SHARED-CAMPAIGN".to_owned();
        input.cases[1].campaign_id = "SHARED-CAMPAIGN".to_owned();
        input.cases[0].attempt_id = "DUPLICATE-ATTEMPT".to_owned();
        input.cases[1].attempt_id = "DUPLICATE-ATTEMPT".to_owned();
        input.cases[0].campaign_attempt_count = 2;
        input.cases[1].campaign_attempt_count = 2;
        input.cases[0].sequence_no = 1;
        input.cases[1].sequence_no = 2;
        input.cases[0].cutoff.sequence_watermark = 1;
        input.cases[0].cutoff.history_max_sequence = 0;
        input.cases[1].cutoff.sequence_watermark = 2;
        input.cases[1].cutoff.history_max_sequence = 1;

        let bytes = serde_json::to_vec(&input).unwrap();
        let error = audit_corpus(&bytes, "0123456789abcdef0123456789abcdef01234567")
            .unwrap_err();
        assert_eq!(error, "ATTEMPT_ID_DUPLICATE_WITHIN_CAMPAIGN");
    }

    #[test]
    fn owner_binding_order_does_not_change_canonical_result() {
        let mut input = corpus();
        let case = input
            .cases
            .iter_mut()
            .find(|case| case.case_id == "AO-025-OWNER-ORDER-PERMUTATION")
            .unwrap();
        let original = evaluate_case(case);
        case.owner_bindings.reverse();
        let permuted = evaluate_case(case);
        assert_eq!(original, permuted);
    }
}
