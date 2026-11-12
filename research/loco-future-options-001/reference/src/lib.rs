//! Finite, observation-constrained future-option reference evaluator.
//!
//! This crate consumes upstream transition, observation, viability, thermal and
//! recovery receipts. It does not create successor states, estimate thermal
//! dynamics, solve viability, qualify observations, or authorize actuation.
//!
//! The essential distinction is:
//! - clairvoyant per-branch availability: an evaluator may choose differently
//!   for each hidden physical successor;
//! - deployable availability: one continuation must work for every physical
//!   successor that yields the same observation to the deployed controller.
//!
//! Physical possibility, not probability mass, determines membership in the
//! robust intersection. A zero-probability but physically possible branch stays
//! in the calculation.

#![forbid(unsafe_code)]

pub mod policy_tree;

use std::collections::{BTreeMap, BTreeSet};

pub const PROTOCOL_ID: &str = "LOCO-FUTURE-OPTIONS-001";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceManifest {
    pub complete: bool,
    pub digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionAlias {
    /// User-facing / policy action label.
    pub label: String,
    /// Canonical physical action identity. Aliases collapse to this ID.
    pub physical_action_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContinuationDomain {
    pub manifest: EvidenceManifest,
    pub aliases: Vec<ActionAlias>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservationEvidence {
    FreshQualified {
        equivalence_class: String,
        receipt_digest: String,
    },
    Unknown {
        reason: String,
    },
    Invalid {
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HardConstraintEvidence {
    WithinLimits,
    Violated { constraint_id: String },
    Unknown { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuccessorBranch {
    pub branch_id: String,
    pub physically_possible: bool,
    /// Parts per million; diagnostic only. This never excludes a physical branch.
    pub probability_mass_ppm: u32,
    pub successor_state_digest: String,
    pub observation: ObservationEvidence,
    /// The upstream viability owner must cover the complete frozen action domain.
    pub viability_complete: bool,
    pub viability_receipt_digest: String,
    /// Labels that the upstream viability owner admitted for this successor.
    /// Omission means not admitted only when viability_complete is true.
    pub admitted_action_labels: Vec<String>,
    pub hard_constraints: HardConstraintEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThermalSourceClass {
    Measured,
    Estimated,
    EnvironmentalBoundary,
    SyntheticFixture,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThermalNode {
    pub node_id: String,
    /// Conservative upper temperature bound, in milli-degrees Celsius.
    pub upper_temperature_milli_c: i64,
    pub hard_limit_milli_c: i64,
    pub source_class: ThermalSourceClass,
    pub evidence_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThermalEvidence {
    NotApplicable,
    Valid {
        model_digest: String,
        node_manifest_digest: String,
        expected_predecessor_digest: String,
        observed_predecessor_digest: String,
        result_state_digest: String,
        required_node_ids: Vec<String>,
        observed_nodes: Vec<ThermalNode>,
    },
    Unknown {
        reason: String,
    },
    Invalid {
        reason: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpstreamDisposition {
    Qualified,
    Unknown,
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryEvidence {
    pub disposition: UpstreamDisposition,
    pub reserve_state_digest: String,
    pub topology_digest: String,
    pub terminal_safe_stop_reachable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FutureOptionScenario {
    pub candidate_action_id: String,
    pub initial_state_digest: String,
    pub transition_model_digest: String,
    pub action_domain_digest: String,
    pub branch_manifest: EvidenceManifest,
    pub continuation_domain: ContinuationDomain,
    pub branches: Vec<SuccessorBranch>,
    pub thermal: ThermalEvidence,
    pub recovery: RecoveryEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenarioDisposition {
    Qualified,
    Unknown,
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvaluationError {
    EmptyField(&'static str),
    DuplicateBranchId(String),
    DuplicateActionLabel(String),
    DuplicateThermalNode(String),
    UnknownAdmittedActionLabel {
        branch_id: String,
        label: String,
    },
    ProbabilityMassOutOfRange {
        branch_id: String,
        probability_mass_ppm: u32,
    },
    NoPhysicallyPossibleBranches,
    EmptyActionDomain,
}

impl std::fmt::Display for EvaluationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for EvaluationError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationOptionSet {
    pub observation_class: String,
    pub branch_ids: Vec<String>,
    /// Canonical physical actions admitted on every branch in this class.
    pub common_physical_action_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FutureOptionReport {
    pub disposition: ScenarioDisposition,
    pub observation_sets: BTreeMap<String, ObservationOptionSet>,
    /// Optimistic per-hidden-state sets, never usable as deployable options.
    pub clairvoyant_branch_actions: BTreeMap<String, Vec<String>>,
    /// Sum of unique canonical actions available independently in each branch.
    pub clairvoyant_branch_option_count: usize,
    /// Sum of common actions over distinct delivered observation classes.
    pub deployable_common_option_count: usize,
    /// Defined only when the full evidence domain is Qualified.
    pub clairvoyance_gap: Option<usize>,
    pub unknown_reasons: Vec<String>,
    pub invalid_reasons: Vec<String>,
    /// Retrospective benchmark output; never a motor command permit.
    pub actuation_authorized: bool,
}

fn require_nonempty(fields: &[(&'static str, &str)]) -> Result<(), EvaluationError> {
    for (name, value) in fields {
        if value.trim().is_empty() {
            return Err(EvaluationError::EmptyField(name));
        }
    }
    Ok(())
}

fn validate_thermal(
    thermal: &ThermalEvidence,
) -> Result<(Vec<String>, Vec<String>), EvaluationError> {
    let mut unknown = Vec::new();
    let mut invalid = Vec::new();
    match thermal {
        ThermalEvidence::NotApplicable => {}
        ThermalEvidence::Unknown { reason } => unknown.push(format!("thermal:{reason}")),
        ThermalEvidence::Invalid { reason } => invalid.push(format!("thermal:{reason}")),
        ThermalEvidence::Valid {
            model_digest,
            node_manifest_digest,
            expected_predecessor_digest,
            observed_predecessor_digest,
            result_state_digest,
            required_node_ids,
            observed_nodes,
        } => {
            for (field, value) in [
                ("thermal.model_digest", model_digest.as_str()),
                (
                    "thermal.node_manifest_digest",
                    node_manifest_digest.as_str(),
                ),
                (
                    "thermal.expected_predecessor_digest",
                    expected_predecessor_digest.as_str(),
                ),
                (
                    "thermal.observed_predecessor_digest",
                    observed_predecessor_digest.as_str(),
                ),
                ("thermal.result_state_digest", result_state_digest.as_str()),
            ] {
                if value.trim().is_empty() {
                    unknown.push(format!("{field}:missing"));
                }
            }
            if expected_predecessor_digest != observed_predecessor_digest {
                unknown.push("thermal:predecessor-mismatch".to_owned());
            }
            if required_node_ids.is_empty() {
                unknown.push("thermal:empty-required-node-manifest".to_owned());
            }
            let mut required = BTreeSet::new();
            for node in required_node_ids {
                if node.trim().is_empty() || !required.insert(node.as_str()) {
                    unknown.push("thermal:invalid-required-node-manifest".to_owned());
                }
            }
            let mut observed = BTreeSet::new();
            for node in observed_nodes {
                if node.node_id.trim().is_empty() || !observed.insert(node.node_id.as_str()) {
                    return Err(EvaluationError::DuplicateThermalNode(node.node_id.clone()));
                }
                if node.evidence_digest.trim().is_empty() {
                    unknown.push(format!("thermal:{}:missing-evidence-digest", node.node_id));
                }
                if node.upper_temperature_milli_c > node.hard_limit_milli_c {
                    invalid.push(format!("thermal:{}:hard-limit-exceeded", node.node_id));
                }
            }
            if required != observed {
                unknown.push("thermal:node-manifest-incomplete-or-mismatched".to_owned());
            }
        }
    }
    Ok((unknown, invalid))
}

fn validate_recovery(recovery: &RecoveryEvidence) -> (Vec<String>, Vec<String>) {
    let mut unknown = Vec::new();
    let mut invalid = Vec::new();
    match recovery.disposition {
        UpstreamDisposition::Qualified => {
            if recovery.reserve_state_digest.trim().is_empty() {
                unknown.push("recovery:reserve-state-digest-missing".to_owned());
            }
            if recovery.topology_digest.trim().is_empty() {
                unknown.push("recovery:topology-digest-missing".to_owned());
            }
            if !recovery.terminal_safe_stop_reachable {
                invalid.push("recovery:terminal-safe-stop-unreachable".to_owned());
            }
        }
        UpstreamDisposition::Unknown => unknown.push("recovery:upstream-unknown".to_owned()),
        UpstreamDisposition::Invalid => invalid.push("recovery:upstream-invalid".to_owned()),
    }
    (unknown, invalid)
}

fn canonical_domain(
    domain: &ContinuationDomain,
) -> Result<(BTreeMap<String, String>, BTreeSet<String>), EvaluationError> {
    if domain.aliases.is_empty() {
        return Err(EvaluationError::EmptyActionDomain);
    }
    let mut label_to_physical = BTreeMap::new();
    let mut physical_ids = BTreeSet::new();
    for alias in &domain.aliases {
        require_nonempty(&[
            ("action.label", &alias.label),
            ("action.physical_action_id", &alias.physical_action_id),
        ])?;
        if label_to_physical
            .insert(alias.label.clone(), alias.physical_action_id.clone())
            .is_some()
        {
            return Err(EvaluationError::DuplicateActionLabel(alias.label.clone()));
        }
        physical_ids.insert(alias.physical_action_id.clone());
    }
    Ok((label_to_physical, physical_ids))
}

/// Evaluate one finite first-action / successor set.
///
/// An absent action is considered not admitted only if both the complete
/// continuation domain and that branch's viability receipt are complete.
/// Unknown evidence is never interpreted as an empty feasible set.
pub fn evaluate_future_options(
    scenario: &FutureOptionScenario,
) -> Result<FutureOptionReport, EvaluationError> {
    require_nonempty(&[
        ("candidate_action_id", &scenario.candidate_action_id),
        ("initial_state_digest", &scenario.initial_state_digest),
        ("transition_model_digest", &scenario.transition_model_digest),
        ("action_domain_digest", &scenario.action_domain_digest),
        ("branch_manifest.digest", &scenario.branch_manifest.digest),
        (
            "continuation_domain.digest",
            &scenario.continuation_domain.manifest.digest,
        ),
    ])?;

    let (label_to_physical, _physical_domain) = canonical_domain(&scenario.continuation_domain)?;
    let mut branch_ids = BTreeSet::new();
    let mut possible: Vec<&SuccessorBranch> = Vec::new();
    let mut unknown_reasons = Vec::new();
    let mut invalid_reasons = Vec::new();

    if !scenario.branch_manifest.complete {
        unknown_reasons.push("branch-manifest-incomplete".to_owned());
    }
    if !scenario.continuation_domain.manifest.complete {
        unknown_reasons.push("continuation-domain-incomplete".to_owned());
    }

    for branch in &scenario.branches {
        require_nonempty(&[
            ("branch_id", &branch.branch_id),
            ("successor_state_digest", &branch.successor_state_digest),
        ])?;
        if !branch_ids.insert(branch.branch_id.clone()) {
            return Err(EvaluationError::DuplicateBranchId(branch.branch_id.clone()));
        }
        if branch.probability_mass_ppm > 1_000_000 {
            return Err(EvaluationError::ProbabilityMassOutOfRange {
                branch_id: branch.branch_id.clone(),
                probability_mass_ppm: branch.probability_mass_ppm,
            });
        }
        if branch.physically_possible {
            // Probability is deliberately not consulted here.
            possible.push(branch);
        }
    }
    if possible.is_empty() {
        return Err(EvaluationError::NoPhysicallyPossibleBranches);
    }

    let (thermal_unknown, thermal_invalid) = validate_thermal(&scenario.thermal)?;
    unknown_reasons.extend(thermal_unknown);
    invalid_reasons.extend(thermal_invalid);
    let (recovery_unknown, recovery_invalid) = validate_recovery(&scenario.recovery);
    unknown_reasons.extend(recovery_unknown);
    invalid_reasons.extend(recovery_invalid);

    let mut branch_actions: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut observation_branches: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for branch in &possible {
        match &branch.observation {
            ObservationEvidence::FreshQualified {
                equivalence_class,
                receipt_digest,
            } => {
                require_nonempty(&[
                    ("observation.equivalence_class", equivalence_class),
                    ("observation.receipt_digest", receipt_digest),
                ])?;
                observation_branches
                    .entry(equivalence_class.clone())
                    .or_default()
                    .push(branch.branch_id.clone());
            }
            ObservationEvidence::Unknown { reason } => {
                unknown_reasons.push(format!("observation:{}:{reason}", branch.branch_id));
            }
            ObservationEvidence::Invalid { reason } => {
                invalid_reasons.push(format!("observation:{}:{reason}", branch.branch_id));
            }
        }

        match &branch.hard_constraints {
            HardConstraintEvidence::WithinLimits => {}
            HardConstraintEvidence::Violated { constraint_id } => {
                invalid_reasons.push(format!(
                    "branch:{}:constraint:{constraint_id}",
                    branch.branch_id
                ));
            }
            HardConstraintEvidence::Unknown { reason } => unknown_reasons.push(format!(
                "branch:{}:constraint-unknown:{reason}",
                branch.branch_id
            )),
        }

        if !branch.viability_complete || branch.viability_receipt_digest.trim().is_empty() {
            unknown_reasons.push(format!(
                "branch:{}:viability-domain-or-receipt-incomplete",
                branch.branch_id
            ));
            continue;
        }

        let mut admitted = BTreeSet::new();
        let mut labels_seen = BTreeSet::new();
        for label in &branch.admitted_action_labels {
            if !labels_seen.insert(label.as_str()) {
                // Duplicate display labels cannot increase the option count.
                continue;
            }
            let physical = label_to_physical.get(label).ok_or_else(|| {
                EvaluationError::UnknownAdmittedActionLabel {
                    branch_id: branch.branch_id.clone(),
                    label: label.clone(),
                }
            })?;
            admitted.insert(physical.clone());
        }
        branch_actions.insert(branch.branch_id.clone(), admitted);
    }

    let disposition = if !invalid_reasons.is_empty() {
        ScenarioDisposition::Invalid
    } else if !unknown_reasons.is_empty() {
        ScenarioDisposition::Unknown
    } else {
        ScenarioDisposition::Qualified
    };

    // No positive set claim is emitted for incomplete/Unknown observation or
    // viability evidence. An Invalid hard limit produces known empty sets.
    if disposition == ScenarioDisposition::Unknown {
        return Ok(FutureOptionReport {
            disposition,
            observation_sets: BTreeMap::new(),
            clairvoyant_branch_actions: BTreeMap::new(),
            clairvoyant_branch_option_count: 0,
            deployable_common_option_count: 0,
            clairvoyance_gap: None,
            unknown_reasons,
            invalid_reasons,
            actuation_authorized: false,
        });
    }

    let mut clairvoyant_branch_actions = BTreeMap::new();
    let mut clairvoyant_branch_option_count = 0_usize;
    for branch in &possible {
        let actions = branch_actions
            .get(&branch.branch_id)
            .cloned()
            .unwrap_or_default();
        clairvoyant_branch_option_count += actions.len();
        clairvoyant_branch_actions.insert(branch.branch_id.clone(), actions.into_iter().collect());
    }

    let mut observation_sets = BTreeMap::new();
    let mut deployable_common_option_count = 0_usize;
    for (class, ids) in observation_branches {
        let mut sorted_ids = ids;
        sorted_ids.sort();
        let mut iter = sorted_ids.iter();
        let first = iter.next().expect("observation class has a branch");
        let mut common = branch_actions.get(first).cloned().unwrap_or_default();
        for branch_id in iter {
            let current = branch_actions.get(branch_id).cloned().unwrap_or_default();
            common.retain(|action| current.contains(action));
        }
        deployable_common_option_count += common.len();
        observation_sets.insert(
            class.clone(),
            ObservationOptionSet {
                observation_class: class,
                branch_ids: sorted_ids,
                common_physical_action_ids: common.into_iter().collect(),
            },
        );
    }

    if disposition == ScenarioDisposition::Invalid {
        // No branch/action option gets a positive count when any required hard
        // constraint is explicitly invalid, including when observation evidence
        // itself is invalid and therefore produced no observation groups.
        for set in observation_sets.values_mut() {
            set.common_physical_action_ids.clear();
        }
        clairvoyant_branch_actions.clear();
        clairvoyant_branch_option_count = 0;
        deployable_common_option_count = 0;
    }
    let clairvoyance_gap = (disposition == ScenarioDisposition::Qualified)
        .then_some(clairvoyant_branch_option_count.saturating_sub(deployable_common_option_count));

    Ok(FutureOptionReport {
        disposition,
        observation_sets,
        clairvoyant_branch_actions,
        clairvoyant_branch_option_count,
        deployable_common_option_count,
        clairvoyance_gap,
        unknown_reasons,
        invalid_reasons,
        actuation_authorized: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn qualified_recovery() -> RecoveryEvidence {
        RecoveryEvidence {
            disposition: UpstreamDisposition::Qualified,
            reserve_state_digest: "reserve:current".to_owned(),
            topology_digest: "topology:current".to_owned(),
            terminal_safe_stop_reachable: true,
        }
    }

    fn thermal_na() -> ThermalEvidence {
        ThermalEvidence::NotApplicable
    }

    fn scenario(branches: Vec<SuccessorBranch>) -> FutureOptionScenario {
        FutureOptionScenario {
            candidate_action_id: "first-action:step".to_owned(),
            initial_state_digest: "state:initial".to_owned(),
            transition_model_digest: "model:transition".to_owned(),
            action_domain_digest: "domain:first-action".to_owned(),
            branch_manifest: EvidenceManifest {
                complete: true,
                digest: "manifest:branches".to_owned(),
            },
            continuation_domain: ContinuationDomain {
                manifest: EvidenceManifest {
                    complete: true,
                    digest: "manifest:continuations".to_owned(),
                },
                aliases: vec![
                    ActionAlias {
                        label: "left".to_owned(),
                        physical_action_id: "physical:left".to_owned(),
                    },
                    ActionAlias {
                        label: "right".to_owned(),
                        physical_action_id: "physical:right".to_owned(),
                    },
                    ActionAlias {
                        label: "retreat".to_owned(),
                        physical_action_id: "physical:retreat".to_owned(),
                    },
                    ActionAlias {
                        label: "continue".to_owned(),
                        physical_action_id: "physical:continue".to_owned(),
                    },
                ],
            },
            branches,
            thermal: thermal_na(),
            recovery: qualified_recovery(),
        }
    }

    fn branch(
        id: &str,
        observation_class: &str,
        probability_mass_ppm: u32,
        admitted: &[&str],
    ) -> SuccessorBranch {
        SuccessorBranch {
            branch_id: id.to_owned(),
            physically_possible: true,
            probability_mass_ppm,
            successor_state_digest: format!("state:{id}"),
            observation: ObservationEvidence::FreshQualified {
                equivalence_class: observation_class.to_owned(),
                receipt_digest: format!("observation:{id}"),
            },
            viability_complete: true,
            viability_receipt_digest: format!("viability:{id}"),
            admitted_action_labels: admitted.iter().map(|v| (*v).to_owned()).collect(),
            hard_constraints: HardConstraintEvidence::WithinLimits,
        }
    }

    fn oracle(s: &FutureOptionScenario) -> BTreeMap<String, BTreeSet<String>> {
        // Independent, deliberately simple brute-force common-set enumerator.
        let mut result: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for b in s.branches.iter().filter(|b| b.physically_possible) {
            let obs = match &b.observation {
                ObservationEvidence::FreshQualified {
                    equivalence_class, ..
                } => equivalence_class,
                _ => continue,
            };
            let aliases: BTreeMap<&str, &str> = s
                .continuation_domain
                .aliases
                .iter()
                .map(|a| (a.label.as_str(), a.physical_action_id.as_str()))
                .collect();
            let actions: BTreeSet<String> = b
                .admitted_action_labels
                .iter()
                .filter_map(|label| aliases.get(label.as_str()).map(|id| (*id).to_owned()))
                .collect();
            match result.get_mut(obs) {
                Some(current) => current.retain(|action| actions.contains(action)),
                None => {
                    result.insert(obs.clone(), actions);
                }
            }
        }
        result
    }

    #[test]
    fn clairvoyance_trap_distinguishes_hidden_state_union_from_common_action() {
        let s = scenario(vec![
            branch("b1", "ambiguous", 500_000, &["left"]),
            branch("b2", "ambiguous", 500_000, &["right"]),
        ]);
        let report = evaluate_future_options(&s).unwrap();
        assert_eq!(report.disposition, ScenarioDisposition::Qualified);
        assert_eq!(
            report.observation_sets["ambiguous"].common_physical_action_ids,
            Vec::<String>::new()
        );
        assert_eq!(report.clairvoyant_branch_option_count, 2);
        assert_eq!(report.deployable_common_option_count, 0);
        assert_eq!(report.clairvoyance_gap, Some(2));
        assert!(!report.actuation_authorized);
    }

    #[test]
    fn a_fresh_observation_can_legitimately_split_the_continuation_sets() {
        let s = scenario(vec![
            branch("rough", "measured:rough", 500_000, &["retreat"]),
            branch("firm", "measured:firm", 500_000, &["continue"]),
        ]);
        let report = evaluate_future_options(&s).unwrap();
        assert_eq!(report.disposition, ScenarioDisposition::Qualified);
        assert_eq!(
            report.observation_sets["measured:rough"].common_physical_action_ids,
            vec!["physical:retreat".to_owned()]
        );
        assert_eq!(
            report.observation_sets["measured:firm"].common_physical_action_ids,
            vec!["physical:continue".to_owned()]
        );
        assert_eq!(report.clairvoyance_gap, Some(0));
    }

    #[test]
    fn zero_probability_physical_branch_still_removes_a_robust_option() {
        let s = scenario(vec![
            branch("likely", "same-observation", 1_000_000, &["continue"]),
            branch("zero-mass", "same-observation", 0, &[]),
        ]);
        let report = evaluate_future_options(&s).unwrap();
        assert_eq!(report.disposition, ScenarioDisposition::Qualified);
        assert_eq!(
            report.observation_sets["same-observation"].common_physical_action_ids,
            Vec::<String>::new()
        );
        assert_eq!(report.clairvoyant_branch_option_count, 1);
        assert_eq!(report.deployable_common_option_count, 0);
    }

    #[test]
    fn display_aliases_for_one_physical_action_are_counted_once() {
        let mut s = scenario(vec![branch("b1", "o1", 1_000_000, &["short", "recovery"])]);
        s.continuation_domain.aliases.extend([
            ActionAlias {
                label: "short".to_owned(),
                physical_action_id: "physical:step-A".to_owned(),
            },
            ActionAlias {
                label: "recovery".to_owned(),
                physical_action_id: "physical:step-A".to_owned(),
            },
        ]);
        let report = evaluate_future_options(&s).unwrap();
        assert_eq!(
            report.observation_sets["o1"]
                .common_physical_action_ids
                .len(),
            1
        );
        assert_eq!(
            report.observation_sets["o1"].common_physical_action_ids,
            vec!["physical:step-A".to_owned()]
        );
    }

    #[test]
    fn incomplete_branch_manifest_is_unknown_not_a_positive_option_claim() {
        let mut s = scenario(vec![branch("b1", "o1", 1_000_000, &["continue"])]);
        s.branch_manifest.complete = false;
        let report = evaluate_future_options(&s).unwrap();
        assert_eq!(report.disposition, ScenarioDisposition::Unknown);
        assert!(report.observation_sets.is_empty());
        assert_eq!(report.clairvoyance_gap, None);
    }

    #[test]
    fn unknown_observation_freshness_cannot_split_hidden_states() {
        let mut s = scenario(vec![branch("b1", "o1", 500_000, &["continue"])]);
        s.branches[0].observation = ObservationEvidence::Unknown {
            reason: "sensor timestamp relation unavailable".to_owned(),
        };
        let report = evaluate_future_options(&s).unwrap();
        assert_eq!(report.disposition, ScenarioDisposition::Unknown);
        assert!(report.observation_sets.is_empty());
    }

    #[test]
    fn incomplete_per_branch_viability_receipt_is_unknown() {
        let mut s = scenario(vec![branch("b1", "o1", 1_000_000, &["continue"])]);
        s.branches[0].viability_complete = false;
        let report = evaluate_future_options(&s).unwrap();
        assert_eq!(report.disposition, ScenarioDisposition::Unknown);
        assert!(report.observation_sets.is_empty());
    }

    #[test]
    fn hard_thermal_limit_is_per_node_not_hidden_by_the_mean() {
        let mut s = scenario(vec![branch("b1", "hot-state", 1_000_000, &["continue"])]);
        s.thermal = ThermalEvidence::Valid {
            model_digest: "thermal-model".to_owned(),
            node_manifest_digest: "thermal-nodes".to_owned(),
            expected_predecessor_digest: "thermal-prev".to_owned(),
            observed_predecessor_digest: "thermal-prev".to_owned(),
            result_state_digest: "thermal-next".to_owned(),
            required_node_ids: vec![
                "motor-a".to_owned(),
                "motor-b".to_owned(),
                "compute".to_owned(),
            ],
            observed_nodes: vec![
                ThermalNode {
                    node_id: "motor-a".to_owned(),
                    upper_temperature_milli_c: 85_000,
                    hard_limit_milli_c: 110_000,
                    source_class: ThermalSourceClass::SyntheticFixture,
                    evidence_digest: "temp:motor-a".to_owned(),
                },
                ThermalNode {
                    node_id: "motor-b".to_owned(),
                    upper_temperature_milli_c: 101_000,
                    hard_limit_milli_c: 100_000,
                    source_class: ThermalSourceClass::SyntheticFixture,
                    evidence_digest: "temp:motor-b".to_owned(),
                },
                ThermalNode {
                    node_id: "compute".to_owned(),
                    upper_temperature_milli_c: 45_000,
                    hard_limit_milli_c: 105_000,
                    source_class: ThermalSourceClass::SyntheticFixture,
                    evidence_digest: "temp:compute".to_owned(),
                },
            ],
        };
        let report = evaluate_future_options(&s).unwrap();
        assert_eq!(report.disposition, ScenarioDisposition::Invalid);
        assert!(report.invalid_reasons.iter().any(|v| v.contains("motor-b")));
        assert_eq!(report.deployable_common_option_count, 0);
        // The arithmetic mean is 77 C, below all but it cannot mask the 101 C node.
    }

    #[test]
    fn mismatched_thermal_predecessor_is_unknown_not_reused() {
        let mut s = scenario(vec![branch("b1", "o1", 1_000_000, &["continue"])]);
        s.thermal = ThermalEvidence::Valid {
            model_digest: "thermal-model".to_owned(),
            node_manifest_digest: "thermal-nodes".to_owned(),
            expected_predecessor_digest: "current-history".to_owned(),
            observed_predecessor_digest: "other-history".to_owned(),
            result_state_digest: "thermal-next".to_owned(),
            required_node_ids: vec!["motor".to_owned()],
            observed_nodes: vec![ThermalNode {
                node_id: "motor".to_owned(),
                upper_temperature_milli_c: 80_000,
                hard_limit_milli_c: 100_000,
                source_class: ThermalSourceClass::SyntheticFixture,
                evidence_digest: "temp:motor".to_owned(),
            }],
        };
        let report = evaluate_future_options(&s).unwrap();
        assert_eq!(report.disposition, ScenarioDisposition::Unknown);
        assert!(report.observation_sets.is_empty());
    }

    #[test]
    fn terminal_safe_stop_is_a_separate_hard_recovery_requirement() {
        let mut s = scenario(vec![branch("b1", "o1", 1_000_000, &["continue"])]);
        s.recovery.terminal_safe_stop_reachable = false;
        let report = evaluate_future_options(&s).unwrap();
        assert_eq!(report.disposition, ScenarioDisposition::Invalid);
        assert!(
            report
                .invalid_reasons
                .iter()
                .any(|v| v.contains("safe-stop"))
        );
    }

    #[test]
    fn permutation_of_branch_input_does_not_change_the_result() {
        let branches = vec![
            branch("b1", "same", 500_000, &["left", "retreat"]),
            branch("b2", "same", 500_000, &["retreat", "right"]),
            branch("b3", "other", 0, &["continue"]),
        ];
        let forward = evaluate_future_options(&scenario(branches.clone())).unwrap();
        let reverse =
            evaluate_future_options(&scenario(branches.into_iter().rev().collect())).unwrap();
        assert_eq!(forward.observation_sets, reverse.observation_sets);
        assert_eq!(
            forward.clairvoyant_branch_actions,
            reverse.clairvoyant_branch_actions
        );
        assert_eq!(forward.clairvoyance_gap, reverse.clairvoyance_gap);
    }

    #[test]
    fn adding_a_physical_branch_cannot_increase_common_continuations() {
        let base = scenario(vec![
            branch("b1", "same", 500_000, &["left", "retreat"]),
            branch("b2", "same", 500_000, &["retreat", "right"]),
        ]);
        let before = evaluate_future_options(&base).unwrap();
        let mut extended = base;
        extended.branches.push(branch("b3", "same", 0, &["left"]));
        let after = evaluate_future_options(&extended).unwrap();
        let before_set: BTreeSet<_> = before.observation_sets["same"]
            .common_physical_action_ids
            .iter()
            .cloned()
            .collect();
        let after_set: BTreeSet<_> = after.observation_sets["same"]
            .common_physical_action_ids
            .iter()
            .cloned()
            .collect();
        assert!(after_set.is_subset(&before_set));
    }

    #[test]
    fn action_label_alias_cannot_inflate_the_physical_domain() {
        let mut s = scenario(vec![branch("b1", "o1", 1_000_000, &["continue"])]);
        s.continuation_domain.aliases.push(ActionAlias {
            label: "continue-alias".to_owned(),
            physical_action_id: "physical:continue".to_owned(),
        });
        s.branches[0]
            .admitted_action_labels
            .push("continue-alias".to_owned());
        let report = evaluate_future_options(&s).unwrap();
        assert_eq!(
            report.observation_sets["o1"]
                .common_physical_action_ids
                .len(),
            1
        );
    }

    #[test]
    fn recovery_or_topology_unknown_prevents_a_positive_option_claim() {
        let mut s = scenario(vec![branch("b1", "o1", 1_000_000, &["continue"])]);
        s.recovery.disposition = UpstreamDisposition::Unknown;
        let report = evaluate_future_options(&s).unwrap();
        assert_eq!(report.disposition, ScenarioDisposition::Unknown);
        assert!(report.observation_sets.is_empty());
    }

    #[test]
    fn production_intersection_matches_independent_brute_force_oracle() {
        let s = scenario(vec![
            branch("b1", "o1", 500_000, &["left", "retreat"]),
            branch("b2", "o1", 500_000, &["retreat", "right"]),
            branch("b3", "o2", 0, &["continue"]),
        ]);
        let report = evaluate_future_options(&s).unwrap();
        let expected = oracle(&s);
        for (class, actions) in expected {
            assert_eq!(
                report.observation_sets[&class].common_physical_action_ids,
                actions.into_iter().collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn invalid_observation_receipt_cannot_leave_a_clairvoyant_claim() {
        let mut s = scenario(vec![branch("b1", "o1", 1_000_000, &["continue"])]);
        s.branches[0].observation = ObservationEvidence::Invalid {
            reason: "observation receipt contradicts the frozen clock relation".to_owned(),
        };
        let report = evaluate_future_options(&s).unwrap();
        assert_eq!(report.disposition, ScenarioDisposition::Invalid);
        assert!(report.clairvoyant_branch_actions.is_empty());
        assert_eq!(report.clairvoyant_branch_option_count, 0);
        assert_eq!(report.deployable_common_option_count, 0);
        assert_eq!(report.clairvoyance_gap, None);
    }

    #[test]
    fn incomplete_continuation_domain_is_unknown_even_with_branch_receipts() {
        let mut s = scenario(vec![branch("b1", "o1", 1_000_000, &["continue"])]);
        s.continuation_domain.manifest.complete = false;
        let report = evaluate_future_options(&s).unwrap();
        assert_eq!(report.disposition, ScenarioDisposition::Unknown);
        assert!(report.observation_sets.is_empty());
        assert!(report.clairvoyant_branch_actions.is_empty());
    }

    #[test]
    fn explicitly_impossible_branch_does_not_reduce_the_common_set() {
        let mut s = scenario(vec![
            branch("possible", "o1", 1_000_000, &["continue"]),
            branch("impossible", "ignored", 0, &[]),
        ]);
        s.branches[1].physically_possible = false;
        let report = evaluate_future_options(&s).unwrap();
        assert_eq!(report.disposition, ScenarioDisposition::Qualified);
        assert_eq!(
            report.observation_sets["o1"].common_physical_action_ids,
            vec!["physical:continue".to_owned()]
        );
        assert_eq!(report.clairvoyant_branch_option_count, 1);
    }

    #[test]
    fn probability_mass_above_one_is_rejected_as_malformed_input() {
        let s = scenario(vec![branch("b1", "o1", 1_000_001, &["continue"])]);
        assert!(matches!(
            evaluate_future_options(&s),
            Err(EvaluationError::ProbabilityMassOutOfRange { .. })
        ));
    }

    #[test]
    fn malformed_or_unmapped_action_labels_are_not_silently_dropped() {
        let mut s = scenario(vec![branch("b1", "o1", 1_000_000, &["ghost"])]);
        assert!(matches!(
            evaluate_future_options(&s),
            Err(EvaluationError::UnknownAdmittedActionLabel { .. })
        ));
        s.branches[0].admitted_action_labels.clear();
        s.continuation_domain.aliases[0].label.clear();
        assert!(matches!(
            evaluate_future_options(&s),
            Err(EvaluationError::EmptyField("action.label"))
        ));
    }
}
