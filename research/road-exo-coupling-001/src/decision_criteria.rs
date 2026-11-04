//! Decision-criterion sensitivity over a finite, evidence-qualified action table.
//!
//! This module compares decision rules without choosing among them. It keeps
//! hard feasibility separate from utility, reports ties, and never assigns
//! probabilities to epistemic hypotheses.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use crate::StableId;
use crate::decision_closure::{DecisionError, DecisionManifest, ExactRational};

const DOMAIN: &[u8] = b"symtropy.road-decision-criterion-sensitivity.v0.1\0";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeasibilityCell {
    pub action_id: StableId,
    pub hypothesis_id: StableId,
    pub status: FeasibilityStatus,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FeasibilityStatus {
    Feasible,
    Infeasible { reason_id: StableId },
    Unknown { reason_id: StableId },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionCriterionManifest {
    pub analysis_id: StableId,
    pub version: StableId,
    pub decision: DecisionManifest,
    /// Must equal decision.utility_scale_id. Epsilon has the same cardinal units.
    pub epsilon_scale_id: StableId,
    pub epsilon_values: Vec<ExactRational>,
    /// Exactly one explicit feasibility disposition per action/hypothesis pair.
    pub feasibility: Vec<FeasibilityCell>,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CriterionAgreement {
    UniqueAgreement,
    SetAgreementWithTie,
    PartialOverlap,
    Disjoint,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CriterionAnalysisStatus {
    CriterionStable,
    CriterionSensitive,
    NoUniqueRecommendation,
    NoRobustFeasibleAction,
    Incomplete,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionValue {
    pub action_id: StableId,
    pub value: ExactRational,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EpsilonRegretSet {
    pub epsilon: ExactRational,
    pub threshold: ExactRational,
    pub actions: Vec<StableId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DominancePair {
    pub dominator: StableId,
    pub dominated: StableId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionExclusion {
    pub action_id: StableId,
    pub infeasible_under: Vec<FeasibilityGap>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeasibilityGap {
    pub action_id: StableId,
    pub hypothesis_id: StableId,
    pub reason_id: StableId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionCriterionCertificate {
    pub schema_id: &'static str,
    pub manifest_digest: [u8; 32],
    pub status: CriterionAnalysisStatus,
    pub agreement: Option<CriterionAgreement>,
    pub robust_feasible_actions: Vec<StableId>,
    pub excluded_actions: Vec<ActionExclusion>,
    pub unknown_feasibility: Vec<FeasibilityGap>,
    /// Minimum utility over hypotheses, then maximin selection across actions.
    pub maximin_floor_by_action: Vec<ActionValue>,
    pub maximin_actions: Vec<StableId>,
    /// Maximum regret over hypotheses, then minimax-regret selection across actions.
    pub worst_case_regret_by_action: Vec<ActionValue>,
    pub minimax_regret_actions: Vec<StableId>,
    pub epsilon_regret_sets: Vec<EpsilonRegretSet>,
    pub robust_dominance: Vec<DominancePair>,
    pub receipt_digest: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CriterionSensitivityError {
    DecisionInvalid(DecisionError),
    DecisionNotFullySupported,
    UtilityScaleMismatch,
    DuplicateFeasibilityCell,
    MissingFeasibilityCell,
    UnknownAction,
    UnknownHypothesis,
    DuplicateEpsilon,
    NegativeEpsilon,
    ZeroDenominator,
    ArithmeticOverflow,
    ReceiptMismatch,
}

impl DecisionCriterionManifest {
    pub fn validate(&self) -> Result<(), CriterionSensitivityError> {
        self.decision
            .validate()
            .map_err(CriterionSensitivityError::DecisionInvalid)?;

        if self.epsilon_scale_id != self.decision.utility_scale_id {
            return Err(CriterionSensitivityError::UtilityScaleMismatch);
        }

        let mut normalized_epsilons = BTreeSet::new();
        let mut epsilon_values = Vec::with_capacity(self.epsilon_values.len());
        for epsilon in &self.epsilon_values {
            let normalized = normalize(epsilon)?;
            if normalized.numerator < 0 {
                return Err(CriterionSensitivityError::NegativeEpsilon);
            }
            if !normalized_epsilons.insert(normalized.clone()) {
                return Err(CriterionSensitivityError::DuplicateEpsilon);
            }
            epsilon_values.push(normalized);
        }
        // Check every comparison before canonical sorting so a huge rational
        // denominator returns an error instead of panicking in sort_by.
        for left in 0..epsilon_values.len() {
            for right in left + 1..epsilon_values.len() {
                let _ = cmp(&epsilon_values[left], &epsilon_values[right])?;
            }
        }

        let known_actions = self
            .decision
            .actions
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let known_hypotheses = self
            .decision
            .hypotheses
            .iter()
            .map(|hypothesis| hypothesis.hypothesis_id.clone())
            .collect::<BTreeSet<_>>();
        let mut seen = BTreeSet::<(StableId, StableId)>::new();

        for cell in &self.feasibility {
            if !known_actions.contains(&cell.action_id) {
                return Err(CriterionSensitivityError::UnknownAction);
            }
            if !known_hypotheses.contains(&cell.hypothesis_id) {
                return Err(CriterionSensitivityError::UnknownHypothesis);
            }
            if !seen.insert((cell.action_id.clone(), cell.hypothesis_id.clone())) {
                return Err(CriterionSensitivityError::DuplicateFeasibilityCell);
            }
        }

        if seen.len() != known_actions.len() * known_hypotheses.len() {
            return Err(CriterionSensitivityError::MissingFeasibilityCell);
        }

        for hypothesis in &self.decision.hypotheses {
            for outcome in &hypothesis.outcomes {
                let crate::decision_closure::OutcomeSupport::Supported { utility, .. } =
                    &outcome.support
                else {
                    return Err(CriterionSensitivityError::DecisionNotFullySupported);
                };
                if utility.denominator == 0 {
                    return Err(CriterionSensitivityError::ZeroDenominator);
                }
            }
        }
        Ok(())
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, CriterionSensitivityError> {
        self.validate()?;
        let decision_digest = self
            .decision
            .digest()
            .map_err(CriterionSensitivityError::DecisionInvalid)?;

        let mut out = Vec::new();
        out.extend_from_slice(DOMAIN);
        push_id(&mut out, &self.analysis_id);
        push_id(&mut out, &self.version);
        out.extend_from_slice(&decision_digest);
        push_id(&mut out, &self.epsilon_scale_id);

        let mut epsilons = self
            .epsilon_values
            .iter()
            .map(normalize)
            .collect::<Result<Vec<_>, _>>()?;
        epsilons.sort_by(|a, b| cmp(a, b).expect("validated epsilon comparison"));
        push_len(&mut out, epsilons.len());
        for epsilon in &epsilons {
            push_rational(&mut out, epsilon);
        }

        let mut cells = self.feasibility.iter().collect::<Vec<_>>();
        cells.sort_by(|a, b| {
            (&a.action_id, &a.hypothesis_id).cmp(&(&b.action_id, &b.hypothesis_id))
        });
        push_len(&mut out, cells.len());
        for cell in cells {
            push_id(&mut out, &cell.action_id);
            push_id(&mut out, &cell.hypothesis_id);
            match &cell.status {
                FeasibilityStatus::Feasible => out.push(1),
                FeasibilityStatus::Infeasible { reason_id } => {
                    out.push(2);
                    push_id(&mut out, reason_id);
                }
                FeasibilityStatus::Unknown { reason_id } => {
                    out.push(3);
                    push_id(&mut out, reason_id);
                }
            }
        }
        Ok(out)
    }

    pub fn digest(&self) -> Result<[u8; 32], CriterionSensitivityError> {
        Ok(*blake3::hash(&self.canonical_bytes()?).as_bytes())
    }

    pub fn evaluate(&self) -> Result<DecisionCriterionCertificate, CriterionSensitivityError> {
        self.validate()?;
        let manifest_digest = self.digest()?;

        let decision_result = self
            .decision
            .evaluate()
            .map_err(CriterionSensitivityError::DecisionInvalid)?;
        if matches!(
            decision_result.status,
            crate::decision_closure::DecisionStatus::Unsupported
                | crate::decision_closure::DecisionStatus::Incomplete
        ) {
            return Err(CriterionSensitivityError::DecisionNotFullySupported);
        }

        let mut feasibility = BTreeMap::<(StableId, StableId), &FeasibilityStatus>::new();
        for cell in &self.feasibility {
            feasibility.insert(
                (cell.action_id.clone(), cell.hypothesis_id.clone()),
                &cell.status,
            );
        }

        // Unknown feasibility can change the feasible action set. Do not produce
        // a recommendation by assuming the unknown cell is feasible or infeasible.
        let mut unknown_feasibility = self
            .feasibility
            .iter()
            .filter_map(|cell| match &cell.status {
                FeasibilityStatus::Unknown { reason_id } => Some(FeasibilityGap {
                    action_id: cell.action_id.clone(),
                    hypothesis_id: cell.hypothesis_id.clone(),
                    reason_id: reason_id.clone(),
                }),
                FeasibilityStatus::Feasible | FeasibilityStatus::Infeasible { .. } => None,
            })
            .collect::<Vec<_>>();
        unknown_feasibility.sort_by(|a, b| {
            (&a.action_id, &a.hypothesis_id).cmp(&(&b.action_id, &b.hypothesis_id))
        });
        if !unknown_feasibility.is_empty() {
            return Ok(empty_incomplete_certificate(
                manifest_digest,
                unknown_feasibility,
            ));
        }

        let mut robust_feasible_actions = Vec::new();
        let mut excluded_actions = Vec::new();
        for action in &self.decision.actions {
            let mut infeasible_under = Vec::new();
            for hypothesis in &self.decision.hypotheses {
                match feasibility.get(&(action.clone(), hypothesis.hypothesis_id.clone())) {
                    Some(FeasibilityStatus::Feasible) => {}
                    Some(FeasibilityStatus::Infeasible { reason_id }) => {
                        infeasible_under.push(FeasibilityGap {
                            action_id: action.clone(),
                            hypothesis_id: hypothesis.hypothesis_id.clone(),
                            reason_id: reason_id.clone(),
                        });
                    }
                    Some(FeasibilityStatus::Unknown { .. }) | None => {
                        return Ok(empty_incomplete_certificate(manifest_digest));
                    }
                }
            }
            infeasible_under.sort_by(|a, b| a.hypothesis_id.cmp(&b.hypothesis_id));
            if infeasible_under.is_empty() {
                robust_feasible_actions.push(action.clone());
            } else {
                excluded_actions.push(ActionExclusion {
                    action_id: action.clone(),
                    infeasible_under,
                });
            }
        }
        robust_feasible_actions.sort();
        excluded_actions.sort_by(|a, b| a.action_id.cmp(&b.action_id));

        if robust_feasible_actions.is_empty() {
            let mut certificate = DecisionCriterionCertificate {
                schema_id: "symtropy.road-decision-criterion-sensitivity.certificate.v0.1",
                manifest_digest,
                status: CriterionAnalysisStatus::NoRobustFeasibleAction,
                agreement: None,
                robust_feasible_actions,
                excluded_actions,
                unknown_feasibility: Vec::new(),
                maximin_floor_by_action: Vec::new(),
                maximin_actions: Vec::new(),
                worst_case_regret_by_action: Vec::new(),
                minimax_regret_actions: Vec::new(),
                epsilon_regret_sets: Vec::new(),
                robust_dominance: Vec::new(),
                receipt_digest: [0; 32],
            };
            certificate.receipt_digest = *blake3::hash(&certificate_bytes(&certificate)).as_bytes();
            return Ok(certificate);
        }

        let utilities = utility_matrix(&self.decision)?;
        let mut maximin_floor_by_action = Vec::new();
        for action in &robust_feasible_actions {
            let mut floor: Option<ExactRational> = None;
            for hypothesis in &self.decision.hypotheses {
                let value = utility_at(&utilities, action, &hypothesis.hypothesis_id)?;
                floor = Some(match floor {
                    None => value.clone(),
                    Some(current) if cmp(value, &current)? == Ordering::Less => value.clone(),
                    Some(current) => current,
                });
            }
            maximin_floor_by_action.push(ActionValue {
                action_id: action.clone(),
                value: floor.ok_or(CriterionSensitivityError::DecisionNotFullySupported)?,
            });
        }
        let maximin_value = extremum(&maximin_floor_by_action, Ordering::Greater)?;
        let mut maximin_actions = Vec::new();
        for score in &maximin_floor_by_action {
            if cmp(&score.value, &maximin_value)? == Ordering::Equal {
                maximin_actions.push(score.action_id.clone());
            }
        }

        let mut worst_case_regret_by_action = Vec::new();
        for action in &robust_feasible_actions {
            let mut worst = ExactRational::new(0, 1)
                .map_err(|_| CriterionSensitivityError::ArithmeticOverflow)?;
            for hypothesis in &self.decision.hypotheses {
                let best = best_utility_for_hypothesis(
                    &utilities,
                    &robust_feasible_actions,
                    &hypothesis.hypothesis_id,
                )?;
                let chosen = utility_at(&utilities, action, &hypothesis.hypothesis_id)?;
                let regret = best
                    .checked_sub(chosen)
                    .map_err(|_| CriterionSensitivityError::ArithmeticOverflow)?;
                if cmp(&regret, &worst)? == Ordering::Greater {
                    worst = regret;
                }
            }
            worst_case_regret_by_action.push(ActionValue {
                action_id: action.clone(),
                value: worst,
            });
        }
        let minimum_worst_regret = extremum(&worst_case_regret_by_action, Ordering::Less)?;
        let mut minimax_regret_actions = Vec::new();
        for score in &worst_case_regret_by_action {
            if cmp(&score.value, &minimum_worst_regret)? == Ordering::Equal {
                minimax_regret_actions.push(score.action_id.clone());
            }
        }

        let mut ordered_epsilons = self
            .epsilon_values
            .iter()
            .map(normalize)
            .collect::<Result<Vec<_>, _>>()?;
        ordered_epsilons.sort_by(|a, b| cmp(a, b).expect("epsilon comparisons validated"));
        let mut epsilon_regret_sets = Vec::with_capacity(ordered_epsilons.len());
        for epsilon in ordered_epsilons {
            let threshold = add(&minimum_worst_regret, &epsilon)?;
            let mut actions = Vec::new();
            for score in &worst_case_regret_by_action {
                if cmp(&score.value, &threshold)? != Ordering::Greater {
                    actions.push(score.action_id.clone());
                }
            }
            epsilon_regret_sets.push(EpsilonRegretSet {
                epsilon,
                threshold,
                actions,
            });
        }

        let robust_dominance =
            dominance_pairs(&utilities, &robust_feasible_actions, &self.decision)?;
        let agreement = compare_action_sets(&maximin_actions, &minimax_regret_actions);
        let status = match agreement {
            CriterionAgreement::UniqueAgreement => CriterionAnalysisStatus::CriterionStable,
            CriterionAgreement::SetAgreementWithTie => {
                CriterionAnalysisStatus::NoUniqueRecommendation
            }
            CriterionAgreement::PartialOverlap | CriterionAgreement::Disjoint => {
                CriterionAnalysisStatus::CriterionSensitive
            }
        };

        let mut certificate = DecisionCriterionCertificate {
            schema_id: "symtropy.road-decision-criterion-sensitivity.certificate.v0.1",
            manifest_digest,
            status,
            agreement: Some(agreement),
            robust_feasible_actions,
            excluded_actions,
            unknown_feasibility: Vec::new(),
            maximin_floor_by_action,
            maximin_actions,
            worst_case_regret_by_action,
            minimax_regret_actions,
            epsilon_regret_sets,
            robust_dominance,
            receipt_digest: [0; 32],
        };
        certificate.receipt_digest = *blake3::hash(&certificate_bytes(&certificate)).as_bytes();
        Ok(certificate)
    }
}

impl DecisionCriterionCertificate {
    /// Unkeyed checksum only; exact replay against the original manifest is stronger.
    pub fn integrity_check(&self) -> bool {
        *blake3::hash(&certificate_bytes(self)).as_bytes() == self.receipt_digest
    }

    pub fn verify_against(
        &self,
        manifest: &DecisionCriterionManifest,
    ) -> Result<(), CriterionSensitivityError> {
        let recomputed = manifest.evaluate()?;
        if recomputed == *self {
            Ok(())
        } else {
            Err(CriterionSensitivityError::ReceiptMismatch)
        }
    }

    /// Compare the decision outputs while deliberately excluding the input
    /// manifest digest. Useful for detecting duplication sensitivity.
    pub fn decision_projection(&self) -> DecisionProjection {
        DecisionProjection {
            status: self.status,
            agreement: self.agreement,
            robust_feasible_actions: self.robust_feasible_actions.clone(),
            maximin_floor_by_action: self.maximin_floor_by_action.clone(),
            maximin_actions: self.maximin_actions.clone(),
            worst_case_regret_by_action: self.worst_case_regret_by_action.clone(),
            minimax_regret_actions: self.minimax_regret_actions.clone(),
            epsilon_regret_sets: self.epsilon_regret_sets.clone(),
            robust_dominance: self.robust_dominance.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionProjection {
    pub status: CriterionAnalysisStatus,
    pub agreement: Option<CriterionAgreement>,
    pub robust_feasible_actions: Vec<StableId>,
    pub maximin_floor_by_action: Vec<ActionValue>,
    pub maximin_actions: Vec<StableId>,
    pub worst_case_regret_by_action: Vec<ActionValue>,
    pub minimax_regret_actions: Vec<StableId>,
    pub epsilon_regret_sets: Vec<EpsilonRegretSet>,
    pub robust_dominance: Vec<DominancePair>,
}

fn empty_incomplete_certificate(
    manifest_digest: [u8; 32],
    unknown_feasibility: Vec<FeasibilityGap>,
) -> DecisionCriterionCertificate {
    let mut certificate = DecisionCriterionCertificate {
        schema_id: "symtropy.road-decision-criterion-sensitivity.certificate.v0.1",
        manifest_digest,
        status: CriterionAnalysisStatus::Incomplete,
        agreement: None,
        robust_feasible_actions: Vec::new(),
        excluded_actions: Vec::new(),
        unknown_feasibility,
        maximin_floor_by_action: Vec::new(),
        maximin_actions: Vec::new(),
        worst_case_regret_by_action: Vec::new(),
        minimax_regret_actions: Vec::new(),
        epsilon_regret_sets: Vec::new(),
        robust_dominance: Vec::new(),
        receipt_digest: [0; 32],
    };
    certificate.receipt_digest = *blake3::hash(&certificate_bytes(&certificate)).as_bytes();
    certificate
}

fn utility_matrix(
    decision: &DecisionManifest,
) -> Result<BTreeMap<(StableId, StableId), ExactRational>, CriterionSensitivityError> {
    let mut values = BTreeMap::new();
    for hypothesis in &decision.hypotheses {
        for outcome in &hypothesis.outcomes {
            let crate::decision_closure::OutcomeSupport::Supported { utility, .. } =
                &outcome.support
            else {
                return Err(CriterionSensitivityError::DecisionNotFullySupported);
            };
            values.insert(
                (outcome.action_id.clone(), hypothesis.hypothesis_id.clone()),
                normalize(utility)?,
            );
        }
    }
    Ok(values)
}

fn utility_at<'a>(
    utilities: &'a BTreeMap<(StableId, StableId), ExactRational>,
    action: &StableId,
    hypothesis: &StableId,
) -> Result<&'a ExactRational, CriterionSensitivityError> {
    utilities
        .get(&(action.clone(), hypothesis.clone()))
        .ok_or(CriterionSensitivityError::DecisionNotFullySupported)
}

fn best_utility_for_hypothesis(
    utilities: &BTreeMap<(StableId, StableId), ExactRational>,
    actions: &[StableId],
    hypothesis: &StableId,
) -> Result<ExactRational, CriterionSensitivityError> {
    let mut best: Option<ExactRational> = None;
    for action in actions {
        let value = utility_at(utilities, action, hypothesis)?;
        best = Some(match best {
            None => value.clone(),
            Some(current) if cmp(value, &current)? == Ordering::Greater => value.clone(),
            Some(current) => current,
        });
    }
    best.ok_or(CriterionSensitivityError::DecisionNotFullySupported)
}

fn extremum(
    scores: &[ActionValue],
    order: Ordering,
) -> Result<ExactRational, CriterionSensitivityError> {
    let first = scores
        .first()
        .ok_or(CriterionSensitivityError::DecisionNotFullySupported)?;
    let mut best = first.value.clone();
    for score in scores.iter().skip(1) {
        let comparison = cmp(&score.value, &best)?;
        let replace = match order {
            Ordering::Greater => comparison == Ordering::Greater,
            Ordering::Less => comparison == Ordering::Less,
            _ => false,
        };
        if replace {
            best = score.value.clone();
        }
    }
    Ok(best)
}

fn dominance_pairs(
    utilities: &BTreeMap<(StableId, StableId), ExactRational>,
    actions: &[StableId],
    decision: &DecisionManifest,
) -> Result<Vec<DominancePair>, CriterionSensitivityError> {
    let mut result = Vec::new();
    for left in actions {
        for right in actions {
            if left == right {
                continue;
            }
            let mut no_worse_everywhere = true;
            let mut strictly_better_somewhere = false;
            for hypothesis in &decision.hypotheses {
                let left_value = utility_at(utilities, left, &hypothesis.hypothesis_id)?;
                let right_value = utility_at(utilities, right, &hypothesis.hypothesis_id)?;
                match cmp(left_value, right_value)? {
                    Ordering::Less => no_worse_everywhere = false,
                    Ordering::Greater => strictly_better_somewhere = true,
                    Ordering::Equal => {}
                }
            }
            if no_worse_everywhere && strictly_better_somewhere {
                result.push(DominancePair {
                    dominator: left.clone(),
                    dominated: right.clone(),
                });
            }
        }
    }
    result.sort_by(|a, b| (&a.dominator, &a.dominated).cmp(&(&b.dominator, &b.dominated)));
    Ok(result)
}

fn compare_action_sets(left: &[StableId], right: &[StableId]) -> CriterionAgreement {
    let left_set = left.iter().cloned().collect::<BTreeSet<_>>();
    let right_set = right.iter().cloned().collect::<BTreeSet<_>>();
    if left_set == right_set {
        if left_set.len() == 1 {
            CriterionAgreement::UniqueAgreement
        } else {
            CriterionAgreement::SetAgreementWithTie
        }
    } else if left_set.is_disjoint(&right_set) {
        CriterionAgreement::Disjoint
    } else {
        CriterionAgreement::PartialOverlap
    }
}

fn normalize(value: &ExactRational) -> Result<ExactRational, CriterionSensitivityError> {
    ExactRational::new(value.numerator, value.denominator)
        .map_err(|_| CriterionSensitivityError::ZeroDenominator)
}

fn cmp(left: &ExactRational, right: &ExactRational) -> Result<Ordering, CriterionSensitivityError> {
    left.checked_cmp(right)
        .map_err(|_| CriterionSensitivityError::ArithmeticOverflow)
}

fn add(
    left: &ExactRational,
    right: &ExactRational,
) -> Result<ExactRational, CriterionSensitivityError> {
    let left_factor = i128::try_from(right.denominator)
        .map_err(|_| CriterionSensitivityError::ArithmeticOverflow)?;
    let right_factor = i128::try_from(left.denominator)
        .map_err(|_| CriterionSensitivityError::ArithmeticOverflow)?;
    let left_term = left
        .numerator
        .checked_mul(left_factor)
        .ok_or(CriterionSensitivityError::ArithmeticOverflow)?;
    let right_term = right
        .numerator
        .checked_mul(right_factor)
        .ok_or(CriterionSensitivityError::ArithmeticOverflow)?;
    let numerator = left_term
        .checked_add(right_term)
        .ok_or(CriterionSensitivityError::ArithmeticOverflow)?;
    let denominator = left
        .denominator
        .checked_mul(right.denominator)
        .ok_or(CriterionSensitivityError::ArithmeticOverflow)?;
    ExactRational::new(numerator, denominator)
        .map_err(|_| CriterionSensitivityError::ArithmeticOverflow)
}

fn certificate_bytes(certificate: &DecisionCriterionCertificate) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"symtropy.road-decision-criterion-sensitivity.certificate.v0.1\0");
    out.extend_from_slice(&certificate.manifest_digest);
    out.push(certificate.status as u8);
    match certificate.agreement {
        Some(value) => {
            out.push(1);
            out.push(value as u8);
        }
        None => out.push(0),
    }

    push_ids(&mut out, &certificate.robust_feasible_actions);
    push_len(&mut out, certificate.excluded_actions.len());
    for exclusion in &certificate.excluded_actions {
        push_id(&mut out, &exclusion.action_id);
        push_len(&mut out, exclusion.infeasible_under.len());
        for gap in &exclusion.infeasible_under {
            push_id(&mut out, &gap.hypothesis_id);
            push_id(&mut out, &gap.reason_id);
        }
    }
    push_len(&mut out, certificate.unknown_feasibility.len());
    for gap in &certificate.unknown_feasibility {
        push_id(&mut out, &gap.action_id);
        push_id(&mut out, &gap.hypothesis_id);
        push_id(&mut out, &gap.reason_id);
    }
    push_scores(&mut out, &certificate.maximin_floor_by_action);
    push_ids(&mut out, &certificate.maximin_actions);
    push_scores(&mut out, &certificate.worst_case_regret_by_action);
    push_ids(&mut out, &certificate.minimax_regret_actions);
    push_len(&mut out, certificate.epsilon_regret_sets.len());
    for set in &certificate.epsilon_regret_sets {
        push_rational(&mut out, &set.epsilon);
        push_rational(&mut out, &set.threshold);
        push_ids(&mut out, &set.actions);
    }
    push_len(&mut out, certificate.robust_dominance.len());
    for pair in &certificate.robust_dominance {
        push_id(&mut out, &pair.dominator);
        push_id(&mut out, &pair.dominated);
    }
    out
}

fn push_scores(out: &mut Vec<u8>, scores: &[ActionValue]) {
    push_len(out, scores.len());
    for score in scores {
        push_id(out, &score.action_id);
        push_rational(out, &score.value);
    }
}

fn push_rational(out: &mut Vec<u8>, value: &ExactRational) {
    out.extend_from_slice(&value.numerator.to_le_bytes());
    out.extend_from_slice(&value.denominator.to_le_bytes());
}

fn push_ids(out: &mut Vec<u8>, ids: &[StableId]) {
    push_len(out, ids.len());
    for id in ids {
        push_id(out, id);
    }
}

fn push_id(out: &mut Vec<u8>, id: &StableId) {
    let bytes = id.as_str().as_bytes();
    let length = u32::try_from(bytes.len()).expect("StableId length is bounded");
    out.extend_from_slice(&length.to_le_bytes());
    out.extend_from_slice(bytes);
}

fn push_len(out: &mut Vec<u8>, length: usize) {
    let length = u64::try_from(length).expect("collection length must fit u64");
    out.extend_from_slice(&length.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision_closure::{DecisionHypothesis, HypothesisActionOutcome, OutcomeSupport};

    fn id(value: &str) -> StableId {
        StableId::new(value).expect("fixture ID")
    }

    fn supported(action: &str, value: i128, evidence: u8) -> HypothesisActionOutcome {
        HypothesisActionOutcome {
            action_id: id(action),
            support: OutcomeSupport::Supported {
                utility: ExactRational::new(value, 1).unwrap(),
                evidence_digest: [evidence; 32],
            },
        }
    }

    fn hypothesis(name: &str, values: &[(&str, i128)], marker: u8) -> DecisionHypothesis {
        DecisionHypothesis {
            hypothesis_id: id(name),
            reference_manifest_digest: [marker; 32],
            outcomes: values
                .iter()
                .map(|(action, value)| supported(action, *value, marker))
                .collect(),
        }
    }

    fn decision(rows: Vec<DecisionHypothesis>) -> DecisionManifest {
        DecisionManifest {
            decision_id: id("decision-criterion-fixture"),
            version: id("v0.1"),
            utility_scale_id: id("common-utility-scale-v1"),
            utility_semantics_id: id("net-benefit-higher-is-better-v1"),
            actions: vec![id("action-a"), id("action-b")],
            hypotheses: rows,
        }
    }

    fn feasibility(
        hypotheses: &[DecisionHypothesis],
        status: FeasibilityStatus,
    ) -> Vec<FeasibilityCell> {
        hypotheses
            .iter()
            .flat_map(|hypothesis| {
                ["action-a", "action-b"]
                    .into_iter()
                    .map(|action| FeasibilityCell {
                        action_id: id(action),
                        hypothesis_id: hypothesis.hypothesis_id.clone(),
                        status: status.clone(),
                    })
            })
            .collect()
    }

    fn manifest(
        rows: Vec<DecisionHypothesis>,
        epsilon_values: Vec<ExactRational>,
    ) -> DecisionCriterionManifest {
        let decision = decision(rows);
        DecisionCriterionManifest {
            analysis_id: id("criterion-sensitivity-v1"),
            version: id("v0.1"),
            epsilon_scale_id: id("common-utility-scale-v1"),
            feasibility: feasibility(&decision.hypotheses, FeasibilityStatus::Feasible),
            decision,
            epsilon_values,
        }
    }

    #[test]
    fn reports_when_maximin_and_minimax_regret_disagree() {
        let manifest = manifest(
            vec![
                hypothesis("h1", &[("action-a", 100), ("action-b", 90)], 1),
                hypothesis("h2", &[("action-a", 0), ("action-b", 5)], 2),
            ],
            vec![],
        );
        let certificate = manifest.evaluate().unwrap();
        assert_eq!(certificate.maximin_actions, vec![id("action-b")]);
        assert_eq!(certificate.minimax_regret_actions, vec![id("action-a")]);
        assert_eq!(certificate.agreement, Some(CriterionAgreement::Disjoint));
        assert_eq!(
            certificate.status,
            CriterionAnalysisStatus::CriterionSensitive
        );
    }

    #[test]
    fn unique_agreement_is_reported_without_claiming_universal_optimality() {
        let manifest = manifest(
            vec![
                hypothesis("h1", &[("action-a", 10), ("action-b", 2)], 1),
                hypothesis("h2", &[("action-a", 8), ("action-b", 4)], 2),
            ],
            vec![],
        );
        let certificate = manifest.evaluate().unwrap();
        assert_eq!(certificate.maximin_actions, vec![id("action-a")]);
        assert_eq!(certificate.minimax_regret_actions, vec![id("action-a")]);
        assert_eq!(
            certificate.agreement,
            Some(CriterionAgreement::UniqueAgreement)
        );
        assert_eq!(certificate.status, CriterionAnalysisStatus::CriterionStable);
    }

    #[test]
    fn epsilon_regret_preserves_full_near_optimal_set() {
        let manifest = manifest(
            vec![hypothesis("h1", &[("action-a", 10), ("action-b", 9)], 1)],
            vec![
                ExactRational::new(0, 1).unwrap(),
                ExactRational::new(1, 1).unwrap(),
            ],
        );
        let certificate = manifest.evaluate().unwrap();
        assert_eq!(
            certificate.epsilon_regret_sets[0].actions,
            vec![id("action-a")]
        );
        assert_eq!(
            certificate.epsilon_regret_sets[1].actions,
            vec![id("action-a"), id("action-b")]
        );
    }

    #[test]
    fn robust_dominance_is_a_partial_order_not_a_scalar_rank() {
        let manifest = manifest(
            vec![
                hypothesis("h1", &[("action-a", 10), ("action-b", 2)], 1),
                hypothesis("h2", &[("action-a", 8), ("action-b", 4)], 2),
            ],
            vec![],
        );
        let certificate = manifest.evaluate().unwrap();
        assert_eq!(
            certificate.robust_dominance,
            vec![DominancePair {
                dominator: id("action-a"),
                dominated: id("action-b")
            }]
        );
    }

    #[test]
    fn hard_infeasibility_excludes_action_instead_of_penalizing_utility() {
        let mut manifest = manifest(
            vec![
                hypothesis("h1", &[("action-a", 100), ("action-b", 2)], 1),
                hypothesis("h2", &[("action-a", 100), ("action-b", 4)], 2),
            ],
            vec![],
        );
        for cell in &mut manifest.feasibility {
            if cell.action_id == id("action-a") && cell.hypothesis_id == id("h2") {
                cell.status = FeasibilityStatus::Infeasible {
                    reason_id: id("hard-collision-constraint"),
                };
            }
        }
        let certificate = manifest.evaluate().unwrap();
        assert_eq!(certificate.robust_feasible_actions, vec![id("action-b")]);
        assert_eq!(certificate.maximin_actions, vec![id("action-b")]);
        assert_eq!(certificate.excluded_actions[0].action_id, id("action-a"));
        assert_eq!(
            certificate.excluded_actions[0].infeasible_under[0].reason_id,
            id("hard-collision-constraint")
        );
    }

    #[test]
    fn unknown_feasibility_fails_closed() {
        let mut manifest = manifest(
            vec![hypothesis("h1", &[("action-a", 10), ("action-b", 2)], 1)],
            vec![],
        );
        manifest.feasibility[0].status = FeasibilityStatus::Unknown {
            reason_id: id("constraint-unmeasured"),
        };
        let certificate = manifest.evaluate().unwrap();
        assert_eq!(certificate.status, CriterionAnalysisStatus::Incomplete);
        assert_eq!(certificate.unknown_feasibility.len(), 1);
        assert_eq!(
            certificate.unknown_feasibility[0].reason_id,
            id("constraint-unmeasured")
        );
    }

    #[test]
    fn duplicate_hypothesis_rows_do_not_change_decision_projection() {
        let h1 = hypothesis("h1", &[("action-a", 10), ("action-b", 2)], 1);
        let h2 = hypothesis("h2", &[("action-a", 8), ("action-b", 4)], 2);
        let h1_duplicate = hypothesis(
            "h1-common-mode-duplicate",
            &[("action-a", 10), ("action-b", 2)],
            1,
        );
        let original = manifest(
            vec![h1.clone(), h2],
            vec![ExactRational::new(1, 1).unwrap()],
        );
        let duplicated = manifest(
            vec![
                h1,
                h1_duplicate,
                hypothesis("h2", &[("action-a", 8), ("action-b", 4)], 2),
            ],
            vec![ExactRational::new(1, 1).unwrap()],
        );
        assert_eq!(
            original.evaluate().unwrap().decision_projection(),
            duplicated.evaluate().unwrap().decision_projection()
        );
    }

    #[test]
    fn adding_a_new_adverse_hypothesis_is_reported_as_set_sensitivity() {
        let original = manifest(
            vec![
                hypothesis("h1", &[("action-a", 10), ("action-b", 2)], 1),
                hypothesis("h2", &[("action-a", 8), ("action-b", 4)], 2),
            ],
            vec![],
        );
        let expanded = manifest(
            vec![
                hypothesis("h1", &[("action-a", 10), ("action-b", 2)], 1),
                hypothesis("h2", &[("action-a", 8), ("action-b", 4)], 2),
                hypothesis("adverse-h3", &[("action-a", -10), ("action-b", 6)], 3),
            ],
            vec![],
        );
        assert_ne!(
            original.evaluate().unwrap().decision_projection(),
            expanded.evaluate().unwrap().decision_projection()
        );
    }

    #[test]
    fn epsilon_scale_and_nonnegative_values_are_enforced() {
        let mut wrong_scale = manifest(
            vec![hypothesis("h1", &[("action-a", 10), ("action-b", 2)], 1)],
            vec![],
        );
        wrong_scale.epsilon_scale_id = id("vehicle-seconds");
        assert_eq!(
            wrong_scale.validate(),
            Err(CriterionSensitivityError::UtilityScaleMismatch)
        );

        let negative = manifest(
            vec![hypothesis("h1", &[("action-a", 10), ("action-b", 2)], 1)],
            vec![ExactRational::new(-1, 1).unwrap()],
        );
        assert_eq!(
            negative.validate(),
            Err(CriterionSensitivityError::NegativeEpsilon)
        );
    }

    #[test]
    fn duplicate_missing_and_unknown_feasibility_cells_reject() {
        let mut duplicate = manifest(
            vec![hypothesis("h1", &[("action-a", 10), ("action-b", 2)], 1)],
            vec![],
        );
        duplicate.feasibility.push(duplicate.feasibility[0].clone());
        assert_eq!(
            duplicate.validate(),
            Err(CriterionSensitivityError::DuplicateFeasibilityCell)
        );

        let mut missing = manifest(
            vec![hypothesis("h1", &[("action-a", 10), ("action-b", 2)], 1)],
            vec![],
        );
        missing.feasibility.pop();
        assert_eq!(
            missing.validate(),
            Err(CriterionSensitivityError::MissingFeasibilityCell)
        );

        let mut unknown = manifest(
            vec![hypothesis("h1", &[("action-a", 10), ("action-b", 2)], 1)],
            vec![],
        );
        unknown.feasibility[0].hypothesis_id = id("unknown-hypothesis");
        assert_eq!(
            unknown.validate(),
            Err(CriterionSensitivityError::UnknownHypothesis)
        );
    }

    #[test]
    fn equivalent_positive_affine_utility_scaling_preserves_selected_sets() {
        let base = manifest(
            vec![
                hypothesis("h1", &[("action-a", 10), ("action-b", 9)], 1),
                hypothesis("h2", &[("action-a", 8), ("action-b", 7)], 2),
            ],
            vec![ExactRational::new(1, 1).unwrap()],
        );
        let mut scaled = base.clone();
        for hypothesis in &mut scaled.decision.hypotheses {
            for outcome in &mut hypothesis.outcomes {
                if let OutcomeSupport::Supported { utility, .. } = &mut outcome.support {
                    *utility = ExactRational::new(utility.numerator * 10, 1).unwrap();
                }
            }
        }
        scaled.epsilon_values = vec![ExactRational::new(10, 1).unwrap()];
        let left = base.evaluate().unwrap();
        let right = scaled.evaluate().unwrap();
        assert_eq!(left.maximin_actions, right.maximin_actions);
        assert_eq!(left.minimax_regret_actions, right.minimax_regret_actions);
        assert_eq!(
            left.epsilon_regret_sets[0].actions,
            right.epsilon_regret_sets[0].actions
        );
    }

    #[test]
    fn exact_replay_and_manifest_tampering_are_detected() {
        let manifest = manifest(
            vec![hypothesis("h1", &[("action-a", 10), ("action-b", 2)], 1)],
            vec![ExactRational::new(1, 1).unwrap()],
        );
        let certificate = manifest.evaluate().unwrap();
        assert!(certificate.integrity_check());
        assert_eq!(certificate.verify_against(&manifest), Ok(()));
        let mut changed = manifest.clone();
        changed.epsilon_values[0] = ExactRational::new(2, 1).unwrap();
        assert_eq!(
            certificate.verify_against(&changed),
            Err(CriterionSensitivityError::ReceiptMismatch)
        );
    }

    #[test]
    fn declaration_order_does_not_change_certificate() {
        let first = manifest(
            vec![
                hypothesis("h1", &[("action-a", 10), ("action-b", 2)], 1),
                hypothesis("h2", &[("action-a", 8), ("action-b", 4)], 2),
            ],
            vec![
                ExactRational::new(1, 1).unwrap(),
                ExactRational::new(0, 1).unwrap(),
            ],
        );
        let mut second = first.clone();
        second.decision.actions.reverse();
        second.decision.hypotheses.reverse();
        second.feasibility.reverse();
        second.epsilon_values.reverse();
        for hypothesis in &mut second.decision.hypotheses {
            hypothesis.outcomes.reverse();
        }
        assert_eq!(first.digest().unwrap(), second.digest().unwrap());
        assert_eq!(first.evaluate().unwrap(), second.evaluate().unwrap());
    }

    #[test]
    fn no_robustly_feasible_action_is_distinct_from_incomplete() {
        let mut manifest = manifest(
            vec![
                hypothesis("h1", &[("action-a", 10), ("action-b", 2)], 1),
                hypothesis("h2", &[("action-a", 8), ("action-b", 4)], 2),
            ],
            vec![],
        );
        for cell in &mut manifest.feasibility {
            cell.status = FeasibilityStatus::Infeasible {
                reason_id: id("fails-hard-safety-case"),
            };
        }
        assert_eq!(
            manifest.evaluate().unwrap().status,
            CriterionAnalysisStatus::NoRobustFeasibleAction
        );
    }
}
