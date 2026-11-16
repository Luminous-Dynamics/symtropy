//! Synthetic-first contingency-option and recoverability oracle for AeroWeft.
//!
//! This crate checks whether explicitly supplied recovery options satisfy a frozen,
//! evidence-bound contingency profile. It does not simulate flight dynamics, compute
//! physical risk, authenticate receipts, authorize an operation, or issue control actions.
//! A positive result means only that a set of supplied option records satisfies the
//! declared synthetic comparison policy.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeSet;

/// Supported high-level contingency trigger classes.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ContingencyKind {
    /// Loss or degradation of command-and-control communications.
    LostCommandAndControl,
    /// Navigation source or PNT integrity degradation.
    NavigationDegraded,
    /// Unexpected traffic conflict or loss of required traffic observability.
    TrafficConflict,
    /// Energy/fuel reserve or propulsion degradation.
    EnergyOrPropulsionDegraded,
    /// New or changed dynamic airspace restriction.
    DynamicRestriction,
    /// Environmental disturbance outside the nominal planning assumption.
    EnvironmentalDisturbance,
    /// Trigger class not defined by this frozen profile.
    Other,
}

/// Exact identity scope shared by a contingency and each candidate option.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubjectScope {
    /// Pseudonymous aircraft identity.
    pub aircraft_id: String,
    /// Individual flight identity.
    pub flight_id: String,
    /// Operation/mission identity.
    pub operation_id: String,
    /// Operational-intent identity.
    pub intent_id: String,
    /// Operational-intent revision.
    pub intent_revision: u64,
    /// Exact operational-intent digest string.
    pub intent_sha256: String,
    /// Authorization identity.
    pub authorization_id: String,
    /// Authorization revision.
    pub authorization_revision: u64,
    /// Exact authorization digest string.
    pub authorization_sha256: String,
    /// Exact aircraft/model artifact digest.
    pub model_artifact_sha256: String,
    /// Exact operating-domain profile digest.
    pub operating_domain_sha256: String,
    /// Exact model-evidence validation package digest.
    pub validation_package_sha256: String,
    /// Coordinate/time profile digest.
    pub reference_basis_sha256: String,
}

impl SubjectScope {
    fn is_complete(&self) -> bool {
        !self.aircraft_id.trim().is_empty()
            && !self.flight_id.trim().is_empty()
            && !self.operation_id.trim().is_empty()
            && !self.intent_id.trim().is_empty()
            && is_sha256(&self.intent_sha256)
            && !self.authorization_id.trim().is_empty()
            && is_sha256(&self.authorization_sha256)
            && is_sha256(&self.model_artifact_sha256)
            && is_sha256(&self.operating_domain_sha256)
            && is_sha256(&self.validation_package_sha256)
            && is_sha256(&self.reference_basis_sha256)
    }
}

/// Applicability of the model and operating domain to this contingency.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Applicability {
    /// Exact model/domain evidence is admitted for this profile.
    InDomain,
    /// The contingency lies outside the evidence-qualified model/domain.
    OutOfDomain,
    /// Applicability cannot be determined from the supplied evidence.
    Unknown,
}

/// Owner-provided evidence state for one recovery-option package.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceState {
    /// Required Track P/Q/T/U evidence has been qualified by its owners.
    Qualified,
    /// Evidence was explicitly rejected or invalidated by its owner.
    Rejected,
    /// Evidence status is missing or unresolved.
    Unknown,
}

/// Authority state supplied by the responsible authorization owner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthorizationState {
    /// Exact authority is declared to include this contingency option.
    AuthorizedForContingency,
    /// The candidate requires a new or changed authorization.
    ReauthorizationRequired,
    /// The candidate is explicitly not authorized.
    NotAuthorized,
    /// Authority scope is unresolved.
    Unknown,
}

/// Completeness state of the candidate-option inventory.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InventoryState {
    /// The source asserts the option inventory is complete for the stated case/cutoff.
    Complete,
    /// The source explicitly reports partial option coverage.
    Partial,
    /// Inventory completeness is unknown.
    Unknown,
}

/// Source-qualified coverage declaration for the candidate-option inventory.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OptionInventory {
    /// Inventory source namespace.
    pub source_id: String,
    /// Exact inventory manifest digest string.
    pub manifest_sha256: String,
    /// Inventory completeness.
    pub state: InventoryState,
    /// Earliest effective time for which inventory history is complete.
    pub effective_from: i64,
    /// Effective-time frontier represented by the inventory.
    pub effective_through: i64,
    /// Earliest knowledge time for which inventory history is complete.
    pub knowledge_from: i64,
    /// Knowledge-time frontier represented by the inventory.
    pub knowledge_through: i64,
}

/// A frozen contingency scenario and conservative acceptance limits.
#[derive(Clone, Debug, PartialEq)]
pub struct ContingencyCase {
    /// Stable contingency-case identity.
    pub contingency_id: String,
    /// Trigger class.
    pub kind: ContingencyKind,
    /// Exact focal aircraft/flight/intent/authorization/model scope.
    pub subject: SubjectScope,
    /// Model/domain applicability state supplied by Track T/U owners.
    pub applicability: Applicability,
    /// Effective-time evaluation point.
    pub effective_at: i64,
    /// Knowledge-time cutoff for the evidence inventory.
    pub known_at: i64,
    /// Failed/degraded dependency domains in this contingency.
    pub failed_domains: Vec<String>,
    /// Maximum response initiation time, milliseconds.
    pub response_deadline_ms: f64,
    /// Required remaining energy/fuel reserve, Wh-equivalent profile units.
    pub required_energy_reserve: f64,
    /// Minimum lower bound on reachable-occupancy containment, [0, 1].
    pub minimum_containment: f64,
    /// Minimum lower bound on separation margin, metres.
    pub minimum_separation_margin_m: f64,
    /// Maximum upper bound on ground risk, [0, 1].
    pub maximum_ground_risk: f64,
    /// Required number of pairwise-independent options.
    pub required_independent_options: usize,
}

/// Conservative resource, reachability and risk bounds for one option.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OptionBounds {
    /// Upper bound on time to initiate the response, milliseconds.
    pub response_time_upper_ms: f64,
    /// Lower bound on energy/fuel available, Wh-equivalent profile units.
    pub energy_available_lower: f64,
    /// Upper bound on energy/fuel required, Wh-equivalent profile units.
    pub energy_required_upper: f64,
    /// Lower bound on reachable-occupancy containment, [0, 1].
    pub containment_lower: f64,
    /// Lower bound on separation margin, metres.
    pub separation_margin_lower_m: f64,
    /// Upper bound on ground risk, [0, 1].
    pub ground_risk_upper: f64,
}

/// One candidate recovery option and its evidence/dependency binding.
#[derive(Clone, Debug, PartialEq)]
pub struct RecoveryOption {
    /// Stable candidate option identity.
    pub option_id: String,
    /// Contingency case this option claims to address.
    pub contingency_id: String,
    /// Exact subject scope; must equal the contingency scope.
    pub subject: SubjectScope,
    /// Owner-qualified model/domain applicability.
    pub applicability: Applicability,
    /// Owner-provided evidence state.
    pub evidence_state: EvidenceState,
    /// Owner-provided authority state.
    pub authorization_state: AuthorizationState,
    /// Evidence validity end time, exclusive.
    pub evidence_valid_until: i64,
    /// Digest of the option's receipt-binding manifest; syntax only.
    pub evidence_binding_sha256: String,
    /// Exact receipt identifiers referenced by the option.
    pub evidence_receipt_ids: Vec<String>,
    /// Conservative option bounds.
    pub bounds: OptionBounds,
    /// Whether the dependency/failure-domain inventory is complete.
    pub dependency_inventory_complete: bool,
    /// Critical dependency/failure-domain identifiers.
    pub dependency_domains: Vec<String>,
}

/// Typed rejection reason for a candidate option.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum RejectionReason {
    /// The option references a different contingency.
    WrongContingency,
    /// The option's exact aircraft/flight/intent/authority/model scope differs.
    SubjectScopeMismatch,
    /// Model or operating-domain applicability is out of scope.
    OutOfDomain,
    /// Owner explicitly rejected or invalidated the evidence.
    EvidenceRejected,
    /// Evidence expired before the evaluation time.
    EvidenceExpired,
    /// The option is explicitly not authorized.
    NotAuthorized,
    /// A dependency is known to have failed in this contingency.
    FailedDependency,
    /// Response initiation exceeds the required deadline.
    ResponseDeadlineMissed,
    /// Available energy lower bound does not cover consumption plus reserve.
    EnergyReserveInsufficient,
    /// Reachability/occupancy containment lower bound is too small.
    ContainmentInsufficient,
    /// Separation margin lower bound is too small.
    SeparationMarginInsufficient,
    /// Ground-risk upper bound exceeds the profile maximum.
    GroundRiskExceeded,
}

/// Typed reason a candidate cannot yet be fully assessed.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum UncertaintyReason {
    /// Owner evidence state is unresolved.
    EvidenceUnknown,
    /// Authorization scope is unresolved.
    AuthorizationUnknown,
    /// Model/domain applicability is unresolved.
    ApplicabilityUnknown,
    /// Dependency inventory is incomplete or unknown.
    DependencyInventoryIncomplete,
}

/// Candidate-level classification.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OptionDisposition {
    /// All declared gates are satisfied.
    Admissible,
    /// At least one definite rejection condition applies.
    Rejected,
    /// Required evidence is unresolved.
    Indeterminate,
    /// A new or changed authority is required.
    ReauthorizationRequired,
}

/// Conservative slack values for an individually assessed option.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OptionMargins {
    /// Response deadline minus response-time upper bound.
    pub response_slack_ms: f64,
    /// Available-energy lower bound minus consumption upper bound and reserve.
    pub energy_slack: f64,
    /// Containment lower bound minus minimum.
    pub containment_slack: f64,
    /// Separation lower bound minus minimum.
    pub separation_slack_m: f64,
    /// Maximum ground-risk allowance minus ground-risk upper bound.
    pub ground_risk_slack: f64,
}

/// Full result for one candidate option.
#[derive(Clone, Debug, PartialEq)]
pub struct OptionAssessment {
    /// Candidate identity.
    pub option_id: String,
    /// Candidate disposition.
    pub disposition: OptionDisposition,
    /// Exact option evidence-binding digest string.
    pub evidence_binding_sha256: String,
    /// Exact receipt identifiers consumed by this option.
    pub evidence_receipt_ids: Vec<String>,
    /// Evidence validity end time, exclusive.
    pub evidence_valid_until: i64,
    /// Owner-provided authorization state.
    pub authorization_state: AuthorizationState,
    /// Definite rejection reasons.
    pub rejection_reasons: Vec<RejectionReason>,
    /// Unresolved evidence reasons.
    pub uncertainty_reasons: Vec<UncertaintyReason>,
    /// Whether a new authorization is required.
    pub reauthorization_required: bool,
    /// Conservative slack values, retained even when a gate fails.
    pub margins: OptionMargins,
    /// Critical dependencies after common-mode exemptions are applied.
    pub diversity_dependencies: Vec<String>,
}

/// Aggregate status for one contingency.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoverabilityStatus {
    /// A verified pairwise-independent witness meets the required option count.
    CoveredWithRequiredDiversity,
    /// One admissible option is sufficient under the frozen policy.
    CoveredSingleOption,
    /// Admissible options exist, but no pairwise-independent witness meets the requirement.
    CoveredButCorrelated,
    /// Complete inventory shows no admissible option.
    NoAdmissibleOption,
    /// Missing evidence or a conservative witness limit prevents a definitive conclusion.
    Indeterminate,
    /// No admissible option is currently usable until authority is re-established.
    ReauthorizationRequired,
    /// The contingency is outside the Track T/U evidence envelope.
    OutOfDomain,
    /// The contingency trigger is not supported by the frozen policy.
    UnsupportedContingency,
}

/// Typed structural or policy error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvaluationError {
    /// The frozen policy is incomplete or internally contradictory.
    InvalidPolicy,
    /// Contingency identity, scope or numeric limits are malformed.
    InvalidContingency,
    /// Duplicate or malformed option identity.
    InvalidOption,
    /// Duplicate option identifier in the same candidate set.
    DuplicateOptionId,
    /// Duplicate receipt identifier within one option.
    DuplicateReceiptId,
    /// Duplicate or malformed failed-domain/dependency identifier.
    InvalidDependencySet,
    /// Inventory declaration is malformed.
    InvalidInventory,
}

/// Aggregate per-contingency result.
#[derive(Clone, Debug, PartialEq)]
pub struct RecoverabilityAssessment {
    /// Contingency identity.
    pub contingency_id: String,
    /// Aggregate recoverability status.
    pub status: RecoverabilityStatus,
    /// All candidate option assessments in canonical option-ID order.
    pub options: Vec<OptionAssessment>,
    /// IDs of definitely admissible options.
    pub admissible_option_ids: Vec<String>,
    /// Deterministic pairwise-independent witness (a lower bound, not a maximum).
    pub independent_option_witness: Vec<String>,
    /// Whether the candidate-option inventory is complete at both cutoffs.
    pub inventory_complete: bool,
    /// Required number of pairwise-independent options in the frozen profile.
    pub required_independent_options: usize,
    /// Whether the returned witness satisfies the required option count.
    pub required_diversity_met: bool,
    /// Exact failed/degraded domains.
    pub failed_domains: Vec<String>,
    /// Exact profile digest used for the decision.
    pub policy_sha256: String,
}

/// Frozen policy identity and supported contingency classes.
#[derive(Clone, Debug, PartialEq)]
pub struct RecoverabilityPolicy {
    /// Exact policy manifest digest.
    pub policy_sha256: String,
    /// Trigger classes supported by this versioned profile.
    pub supported_contingencies: BTreeSet<ContingencyKind>,
    /// Unavoidable common dependencies excluded from diversity calculations.
    pub diversity_exempt_domains: BTreeSet<String>,
    /// Maximum option count for which V0 gives a determinate diversity result.
    pub maximum_determinate_option_count: usize,
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn unique_nonempty(values: &[String]) -> bool {
    let mut seen = BTreeSet::new();
    values
        .iter()
        .all(|value| !value.trim().is_empty() && seen.insert(value.as_str()))
}

fn finite_nonnegative(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

fn finite_unit_interval(value: f64) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn validate_policy(policy: &RecoverabilityPolicy) -> Result<(), EvaluationError> {
    if !is_sha256(&policy.policy_sha256)
        || policy.supported_contingencies.is_empty()
        || policy.maximum_determinate_option_count == 0
        || policy
            .diversity_exempt_domains
            .iter()
            .any(|value| value.trim().is_empty())
    {
        return Err(EvaluationError::InvalidPolicy);
    }
    Ok(())
}

fn validate_case(case: &ContingencyCase) -> Result<(), EvaluationError> {
    if case.contingency_id.trim().is_empty()
        || !case.subject.is_complete()
        || case.effective_at < 0
        || case.known_at < 0
        || !finite_nonnegative(case.response_deadline_ms)
        || !finite_nonnegative(case.required_energy_reserve)
        || !finite_unit_interval(case.minimum_containment)
        || !finite_nonnegative(case.minimum_separation_margin_m)
        || !finite_unit_interval(case.maximum_ground_risk)
        || case.required_independent_options == 0
        || !unique_nonempty(&case.failed_domains)
    {
        return Err(EvaluationError::InvalidContingency);
    }
    Ok(())
}

fn validate_inventory(inventory: &OptionInventory) -> Result<(), EvaluationError> {
    if inventory.source_id.trim().is_empty()
        || !is_sha256(&inventory.manifest_sha256)
        || inventory.effective_from < 0
        || inventory.effective_from > inventory.effective_through
        || inventory.knowledge_from < 0
        || inventory.knowledge_from > inventory.knowledge_through
    {
        return Err(EvaluationError::InvalidInventory);
    }
    Ok(())
}

fn validate_option(option: &RecoveryOption) -> Result<(), EvaluationError> {
    if !unique_nonempty(&option.dependency_domains) {
        return Err(EvaluationError::InvalidDependencySet);
    }
    if option.dependency_domains.is_empty() {
        return Err(EvaluationError::InvalidDependencySet);
    }
    if option.option_id.trim().is_empty()
        || option.contingency_id.trim().is_empty()
        || !option.subject.is_complete()
        || !is_sha256(&option.evidence_binding_sha256)
        || option.evidence_receipt_ids.is_empty()
        || option
            .evidence_receipt_ids
            .iter()
            .any(|value| value.trim().is_empty())
        || !finite_nonnegative(option.bounds.response_time_upper_ms)
        || !finite_nonnegative(option.bounds.energy_available_lower)
        || !finite_nonnegative(option.bounds.energy_required_upper)
        || !finite_unit_interval(option.bounds.containment_lower)
        || !finite_nonnegative(option.bounds.separation_margin_lower_m)
        || !finite_unit_interval(option.bounds.ground_risk_upper)
    {
        return Err(EvaluationError::InvalidOption);
    }
    Ok(())
}

fn assess_option(
    option: &RecoveryOption,
    case: &ContingencyCase,
    policy: &RecoverabilityPolicy,
) -> OptionAssessment {
    let margins = OptionMargins {
        response_slack_ms: case.response_deadline_ms - option.bounds.response_time_upper_ms,
        energy_slack: option.bounds.energy_available_lower
            - option.bounds.energy_required_upper
            - case.required_energy_reserve,
        containment_slack: option.bounds.containment_lower - case.minimum_containment,
        separation_slack_m: option.bounds.separation_margin_lower_m
            - case.minimum_separation_margin_m,
        ground_risk_slack: case.maximum_ground_risk - option.bounds.ground_risk_upper,
    };
    let mut rejected = BTreeSet::new();
    let mut uncertain = BTreeSet::new();
    let mut reauthorization_required = false;

    if option.contingency_id != case.contingency_id {
        rejected.insert(RejectionReason::WrongContingency);
    }
    if option.subject != case.subject {
        rejected.insert(RejectionReason::SubjectScopeMismatch);
    }
    match option.applicability {
        Applicability::InDomain => {}
        Applicability::OutOfDomain => {
            rejected.insert(RejectionReason::OutOfDomain);
        }
        Applicability::Unknown => {
            uncertain.insert(UncertaintyReason::ApplicabilityUnknown);
        }
    }
    match option.evidence_state {
        EvidenceState::Qualified => {}
        EvidenceState::Rejected => {
            rejected.insert(RejectionReason::EvidenceRejected);
        }
        EvidenceState::Unknown => {
            uncertain.insert(UncertaintyReason::EvidenceUnknown);
        }
    }
    match option.authorization_state {
        AuthorizationState::AuthorizedForContingency => {}
        AuthorizationState::ReauthorizationRequired => {
            reauthorization_required = true;
        }
        AuthorizationState::NotAuthorized => {
            rejected.insert(RejectionReason::NotAuthorized);
        }
        AuthorizationState::Unknown => {
            uncertain.insert(UncertaintyReason::AuthorizationUnknown);
        }
    }
    if option.evidence_valid_until <= case.effective_at {
        rejected.insert(RejectionReason::EvidenceExpired);
    }
    if !option.dependency_inventory_complete {
        uncertain.insert(UncertaintyReason::DependencyInventoryIncomplete);
    }
    let failed: BTreeSet<&str> = case.failed_domains.iter().map(String::as_str).collect();
    if option
        .dependency_domains
        .iter()
        .any(|domain| failed.contains(domain.as_str()))
    {
        rejected.insert(RejectionReason::FailedDependency);
    }
    if margins.response_slack_ms < 0.0 {
        rejected.insert(RejectionReason::ResponseDeadlineMissed);
    }
    if margins.energy_slack < 0.0 {
        rejected.insert(RejectionReason::EnergyReserveInsufficient);
    }
    if margins.containment_slack < 0.0 {
        rejected.insert(RejectionReason::ContainmentInsufficient);
    }
    if margins.separation_slack_m < 0.0 {
        rejected.insert(RejectionReason::SeparationMarginInsufficient);
    }
    if margins.ground_risk_slack < 0.0 {
        rejected.insert(RejectionReason::GroundRiskExceeded);
    }

    let diversity_dependencies: Vec<String> = option
        .dependency_domains
        .iter()
        .filter(|domain| !policy.diversity_exempt_domains.contains(*domain))
        .cloned()
        .collect();

    let disposition = if !rejected.is_empty() {
        OptionDisposition::Rejected
    } else if !uncertain.is_empty() {
        OptionDisposition::Indeterminate
    } else if reauthorization_required {
        OptionDisposition::ReauthorizationRequired
    } else {
        OptionDisposition::Admissible
    };

    OptionAssessment {
        option_id: option.option_id.clone(),
        disposition,
        evidence_binding_sha256: option.evidence_binding_sha256.clone(),
        evidence_receipt_ids: option.evidence_receipt_ids.clone(),
        evidence_valid_until: option.evidence_valid_until,
        authorization_state: option.authorization_state,
        rejection_reasons: rejected.into_iter().collect(),
        uncertainty_reasons: uncertain.into_iter().collect(),
        reauthorization_required,
        margins,
        diversity_dependencies,
    }
}

fn pairwise_independent(left: &OptionAssessment, right: &OptionAssessment) -> bool {
    let right_set: BTreeSet<&str> = right
        .diversity_dependencies
        .iter()
        .map(String::as_str)
        .collect();
    left.diversity_dependencies
        .iter()
        .all(|domain| !right_set.contains(domain.as_str()))
}

/// Build a deterministic greedy witness of pairwise-independent admissible options.
///
/// The witness is a proven lower bound only. A small witness does not establish that a
/// larger independent set does not exist; V0 returns INDETERMINATE when that distinction
/// matters and the exact requested diversity cannot be demonstrated.
fn independent_witness(options: &[OptionAssessment]) -> Vec<String> {
    let mut admissible: Vec<&OptionAssessment> = options
        .iter()
        .filter(|option| option.disposition == OptionDisposition::Admissible)
        .collect();
    admissible.sort_by(|left, right| left.option_id.cmp(&right.option_id));

    let mut selected: Vec<&OptionAssessment> = Vec::new();
    for candidate in admissible {
        if selected
            .iter()
            .all(|chosen| pairwise_independent(candidate, chosen))
        {
            selected.push(candidate);
        }
    }
    selected
        .into_iter()
        .map(|option| option.option_id.clone())
        .collect()
}

/// Evaluate candidate recovery options against a frozen contingency and evidence policy.
///
/// All physical bounds, evidence states, authority states and dependency inventories
/// are supplied by their domain owners. This function checks exact binding and applies
/// conservative comparisons; it does not calculate or authenticate those inputs.
pub fn evaluate_recoverability(
    case: &ContingencyCase,
    candidates: &[RecoveryOption],
    inventory: &OptionInventory,
    policy: &RecoverabilityPolicy,
) -> Result<RecoverabilityAssessment, EvaluationError> {
    validate_policy(policy)?;
    validate_case(case)?;
    validate_inventory(inventory)?;

    if !policy.supported_contingencies.contains(&case.kind) {
        return Ok(RecoverabilityAssessment {
            contingency_id: case.contingency_id.clone(),
            status: RecoverabilityStatus::UnsupportedContingency,
            options: Vec::new(),
            admissible_option_ids: Vec::new(),
            independent_option_witness: Vec::new(),
            inventory_complete: inventory.state == InventoryState::Complete
                && inventory.effective_from <= case.effective_at
                && inventory.effective_through >= case.effective_at
                && inventory.knowledge_from <= case.known_at
                && inventory.knowledge_through >= case.known_at,
            required_independent_options: case.required_independent_options,
            required_diversity_met: false,
            failed_domains: case.failed_domains.clone(),
            policy_sha256: policy.policy_sha256.clone(),
        });
    }

    if case.applicability == Applicability::OutOfDomain {
        return Ok(RecoverabilityAssessment {
            contingency_id: case.contingency_id.clone(),
            status: RecoverabilityStatus::OutOfDomain,
            options: Vec::new(),
            admissible_option_ids: Vec::new(),
            independent_option_witness: Vec::new(),
            inventory_complete: false,
            required_independent_options: case.required_independent_options,
            required_diversity_met: false,
            failed_domains: case.failed_domains.clone(),
            policy_sha256: policy.policy_sha256.clone(),
        });
    }

    if case.applicability == Applicability::Unknown {
        return Ok(RecoverabilityAssessment {
            contingency_id: case.contingency_id.clone(),
            status: RecoverabilityStatus::Indeterminate,
            options: Vec::new(),
            admissible_option_ids: Vec::new(),
            independent_option_witness: Vec::new(),
            inventory_complete: false,
            required_independent_options: case.required_independent_options,
            required_diversity_met: false,
            failed_domains: case.failed_domains.clone(),
            policy_sha256: policy.policy_sha256.clone(),
        });
    }

    let mut seen = BTreeSet::new();
    for option in candidates {
        validate_option(option)?;
        if !seen.insert(option.option_id.as_str()) {
            return Err(EvaluationError::DuplicateOptionId);
        }
        let mut receipt_ids = BTreeSet::new();
        for receipt_id in &option.evidence_receipt_ids {
            if !receipt_ids.insert(receipt_id.as_str()) {
                return Err(EvaluationError::DuplicateReceiptId);
            }
        }
    }

    let inventory_complete = inventory.state == InventoryState::Complete
        && inventory.effective_from <= case.effective_at
        && inventory.effective_through >= case.effective_at
        && inventory.knowledge_from <= case.known_at
        && inventory.knowledge_through >= case.known_at;

    let mut options: Vec<OptionAssessment> = candidates
        .iter()
        .map(|option| assess_option(option, case, policy))
        .collect();
    options.sort_by(|left, right| left.option_id.cmp(&right.option_id));

    let admissible_option_ids: Vec<String> = options
        .iter()
        .filter(|option| option.disposition == OptionDisposition::Admissible)
        .map(|option| option.option_id.clone())
        .collect();
    let witness = independent_witness(&options);
    let has_unknown = options
        .iter()
        .any(|option| option.disposition == OptionDisposition::Indeterminate);
    let has_reauthorization = options
        .iter()
        .any(|option| option.disposition == OptionDisposition::ReauthorizationRequired);
    let exceeds_determinate_limit =
        admissible_option_ids.len() > policy.maximum_determinate_option_count;

    let status = if witness.len() >= case.required_independent_options {
        if case.required_independent_options == 1 {
            RecoverabilityStatus::CoveredSingleOption
        } else {
            RecoverabilityStatus::CoveredWithRequiredDiversity
        }
    } else if !admissible_option_ids.is_empty()
        && witness.len() == 1
        && admissible_option_ids.len() == 1
        && !has_unknown
        && !has_reauthorization
        && inventory_complete
    {
        RecoverabilityStatus::CoveredSingleOption
    } else if admissible_option_ids.is_empty() {
        if has_unknown || !inventory_complete {
            RecoverabilityStatus::Indeterminate
        } else if has_reauthorization {
            RecoverabilityStatus::ReauthorizationRequired
        } else {
            RecoverabilityStatus::NoAdmissibleOption
        }
    } else if exceeds_determinate_limit || has_unknown || !inventory_complete {
        RecoverabilityStatus::Indeterminate
    } else if has_reauthorization {
        RecoverabilityStatus::ReauthorizationRequired
    } else if case.required_independent_options == 2
        && admissible_option_ids.len() > 1
        && options
            .iter()
            .filter(|option| option.disposition == OptionDisposition::Admissible)
            .all(|left| {
                options
                    .iter()
                    .filter(|right| {
                        right.disposition == OptionDisposition::Admissible
                            && right.option_id != left.option_id
                    })
                    .all(|right| !pairwise_independent(left, right))
            })
    {
        RecoverabilityStatus::CoveredButCorrelated
    } else if case.required_independent_options == 1 {
        RecoverabilityStatus::CoveredSingleOption
    } else {
        // The greedy witness is only a lower bound. Do not turn failure to find a
        // sufficiently large witness into a claim that no such set exists.
        RecoverabilityStatus::Indeterminate
    };

    let mut failed_domains = case.failed_domains.clone();
    failed_domains.sort_unstable();
    let required_diversity_met = witness.len() >= case.required_independent_options;

    Ok(RecoverabilityAssessment {
        contingency_id: case.contingency_id.clone(),
        status,
        options,
        admissible_option_ids,
        independent_option_witness: witness,
        inventory_complete,
        required_independent_options: case.required_independent_options,
        required_diversity_met,
        failed_domains,
        policy_sha256: policy.policy_sha256.clone(),
    })
}
