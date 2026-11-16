//! Coverage-first active selection of candidate tactile calibration contacts.
//!
//! The planner only orders already-defined candidate stimuli. It does not
//! command a physical fixture, generate reference force labels, calculate
//! measurement uncertainty, or qualify a target sensor. The predicted
//! information score is a synthetic/model-supplied ranking input, never a
//! measured uncertainty or calibration result.

#![forbid(unsafe_code)]

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReferenceState {
    Current,
    Missing,
    Expired,
    OutOfRange,
    Invalidated,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MethodState {
    Qualified,
    Missing,
    Unqualified,
    Superseded,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConditionState {
    WithinEnvelope,
    OutOfProfile,
    Unresolved,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CalibrationCandidate {
    pub candidate_id: String,
    pub target_unit_id: String,
    pub target_profile_digest: String,
    pub phantom_id: String,
    pub phantom_revision: String,
    pub reference_calibration_id: String,
    pub reference_state: ReferenceState,
    pub method_state: MethodState,
    pub condition_state: ConditionState,
    pub stratum_id: String,
    pub cost_ms: u64,
    /// Dimensionless ranking input supplied by a candidate model.
    /// It is not a measured uncertainty, information gain or quality result.
    pub predicted_information_gain: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StratumQuota {
    pub stratum_id: String,
    pub minimum_contacts: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CalibrationPlanRequest {
    pub target_unit_id: String,
    pub target_profile_digest: String,
    pub phantom_id: String,
    pub phantom_revision: String,
    pub budget_ms: u64,
    pub maximum_contacts: usize,
    pub required_strata: Vec<StratumQuota>,
    pub candidates: Vec<CalibrationCandidate>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanDisposition {
    Complete,
    IncompleteCoverage,
    BudgetInsufficient,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalibrationPlan {
    pub policy_id: &'static str,
    pub disposition: PlanDisposition,
    pub selected_candidate_ids: Vec<String>,
    pub coverage_counts: BTreeMap<String, usize>,
    pub missing_strata: Vec<String>,
    pub excluded_candidate_reasons: BTreeMap<String, String>,
    pub total_cost_ms: u64,
}

const POLICY_ID: &str = "coverage_then_information_gain_per_cost_v0_1";

fn candidate_exclusion_reason(
    candidate: &CalibrationCandidate,
    request: &CalibrationPlanRequest,
) -> Option<&'static str> {
    if candidate.target_unit_id != request.target_unit_id {
        return Some("TARGET_UNIT_MISMATCH");
    }
    if candidate.target_profile_digest != request.target_profile_digest {
        return Some("TARGET_PROFILE_MISMATCH");
    }
    if candidate.phantom_id != request.phantom_id
        || candidate.phantom_revision != request.phantom_revision
    {
        return Some("PHANTOM_REVISION_MISMATCH");
    }
    if candidate.reference_calibration_id.is_empty()
        || candidate.reference_state != ReferenceState::Current
    {
        return Some("REFERENCE_NOT_CURRENT");
    }
    if candidate.method_state != MethodState::Qualified {
        return Some("METHOD_NOT_QUALIFIED");
    }
    if candidate.condition_state != ConditionState::WithinEnvelope {
        return Some("CONDITIONS_NOT_APPLICABLE");
    }
    if candidate.cost_ms == 0 {
        return Some("NONPOSITIVE_COST");
    }
    if !candidate.predicted_information_gain.is_finite()
        || candidate.predicted_information_gain < 0.0
    {
        return Some("INVALID_PRIORITY_SCORE");
    }
    None
}

fn priority_order(
    left: &CalibrationCandidate,
    right: &CalibrationCandidate,
) -> Ordering {
    let left_rate = left.predicted_information_gain / left.cost_ms as f64;
    let right_rate = right.predicted_information_gain / right.cost_ms as f64;

    right_rate
        .total_cmp(&left_rate)
        .then_with(|| {
            right
                .predicted_information_gain
                .total_cmp(&left.predicted_information_gain)
        })
        .then_with(|| left.cost_ms.cmp(&right.cost_ms))
        .then_with(|| left.candidate_id.cmp(&right.candidate_id))
}

fn can_select(
    candidate: &CalibrationCandidate,
    request: &CalibrationPlanRequest,
    selected_count: usize,
    current_cost_ms: u64,
) -> bool {
    selected_count < request.maximum_contacts
        && current_cost_ms
            .checked_add(candidate.cost_ms)
            .is_some_and(|total| total <= request.budget_ms)
}

pub fn plan_calibration(
    request: &CalibrationPlanRequest,
) -> Result<CalibrationPlan, String> {
    if request.target_unit_id.is_empty()
        || request.target_profile_digest.is_empty()
        || request.phantom_id.is_empty()
        || request.phantom_revision.is_empty()
    {
        return Err("target and phantom identities must be nonempty".to_owned());
    }
    if request.maximum_contacts == 0 {
        return Err("maximum_contacts must be positive".to_owned());
    }
    if request.required_strata.iter().any(|quota| {
        quota.stratum_id.is_empty() || quota.minimum_contacts == 0
    }) {
        return Err("required stratum IDs and quotas must be positive".to_owned());
    }

    let mut quota_ids = BTreeSet::new();
    for quota in &request.required_strata {
        if !quota_ids.insert(quota.stratum_id.as_str()) {
            return Err(format!(
                "duplicate required stratum: {}",
                quota.stratum_id
            ));
        }
    }

    let mut candidate_ids = BTreeSet::new();
    let mut eligible = Vec::new();
    let mut excluded = BTreeMap::new();

    for candidate in &request.candidates {
        if candidate.candidate_id.is_empty() || candidate.stratum_id.is_empty() {
            return Err("candidate ID and stratum ID must be nonempty".to_owned());
        }
        if !candidate_ids.insert(candidate.candidate_id.as_str()) {
            return Err(format!(
                "duplicate candidate ID: {}",
                candidate.candidate_id
            ));
        }

        if let Some(reason) = candidate_exclusion_reason(candidate, request) {
            excluded.insert(candidate.candidate_id.clone(), reason.to_owned());
        } else {
            eligible.push(candidate);
        }
    }

    let mut selected = Vec::new();
    let mut selected_ids = BTreeSet::new();
    let mut coverage = BTreeMap::<String, usize>::new();
    let mut total_cost_ms = 0_u64;
    let mut budget_blocked = false;
    let mut candidate_shortage = false;

    // Stage 1: satisfy mandatory coverage before optimizing optional utility.
    for quota in &request.required_strata {
        let mut stratum_candidates: Vec<_> = eligible
            .iter()
            .copied()
            .filter(|candidate| candidate.stratum_id == quota.stratum_id)
            .collect();
        stratum_candidates.sort_by(|left, right| priority_order(left, right));

        for candidate in stratum_candidates {
            if coverage.get(&quota.stratum_id).copied().unwrap_or(0)
                >= quota.minimum_contacts
            {
                break;
            }

            if !can_select(
                candidate,
                request,
                selected.len(),
                total_cost_ms,
            ) {
                budget_blocked = true;
                break;
            }

            if selected_ids.insert(candidate.candidate_id.as_str()) {
                total_cost_ms = total_cost_ms
                    .checked_add(candidate.cost_ms)
                    .ok_or_else(|| "calibration plan cost overflow".to_owned())?;
                selected.push(candidate.candidate_id.clone());
                *coverage.entry(candidate.stratum_id.clone()).or_default() += 1;
            }
        }

        if coverage.get(&quota.stratum_id).copied().unwrap_or(0)
            < quota.minimum_contacts
        {
            let eligible_count = eligible
                .iter()
                .filter(|candidate| candidate.stratum_id == quota.stratum_id)
                .count();
            if eligible_count == 0 {
                candidate_shortage = true;
            } else {
                budget_blocked = true;
            }
        }
    }

    let missing_strata: Vec<_> = request
        .required_strata
        .iter()
        .filter(|quota| {
            coverage.get(&quota.stratum_id).copied().unwrap_or(0)
                < quota.minimum_contacts
        })
        .map(|quota| quota.stratum_id.clone())
        .collect();

    // Stage 2: after mandatory coverage, use the model-supplied ranking only
    // to spend any remaining budget on additional eligible contacts.
    if missing_strata.is_empty() {
        let mut remaining: Vec<_> = eligible
            .iter()
            .copied()
            .filter(|candidate| !selected_ids.contains(candidate.candidate_id.as_str()))
            .collect();
        remaining.sort_by(|left, right| priority_order(left, right));

        for candidate in remaining {
            if !can_select(
                candidate,
                request,
                selected.len(),
                total_cost_ms,
            ) {
                continue;
            }
            if selected_ids.insert(candidate.candidate_id.as_str()) {
                total_cost_ms = total_cost_ms
                    .checked_add(candidate.cost_ms)
                    .ok_or_else(|| "calibration plan cost overflow".to_owned())?;
                selected.push(candidate.candidate_id.clone());
                *coverage.entry(candidate.stratum_id.clone()).or_default() += 1;
            }
        }
    }

    let disposition = if missing_strata.is_empty() {
        PlanDisposition::Complete
    } else if candidate_shortage && !budget_blocked {
        PlanDisposition::IncompleteCoverage
    } else if candidate_shortage && budget_blocked {
        // Some mandatory strata had no eligible candidate while another
        // mandatory quota was also blocked by budget/contact limits.
        PlanDisposition::IncompleteCoverage
    } else {
        PlanDisposition::BudgetInsufficient
    };

    Ok(CalibrationPlan {
        policy_id: POLICY_ID,
        disposition,
        selected_candidate_ids: selected,
        coverage_counts: coverage,
        missing_strata,
        excluded_candidate_reasons: excluded,
        total_cost_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(
        id: &str,
        stratum: &str,
        score: f64,
        cost_ms: u64,
    ) -> CalibrationCandidate {
        CalibrationCandidate {
            candidate_id: id.to_owned(),
            target_unit_id: "target-01".to_owned(),
            target_profile_digest: "target-profile-v1".to_owned(),
            phantom_id: "phantom-01".to_owned(),
            phantom_revision: "phantom-rev-1".to_owned(),
            reference_calibration_id: "ref-cal-01".to_owned(),
            reference_state: ReferenceState::Current,
            method_state: MethodState::Qualified,
            condition_state: ConditionState::WithinEnvelope,
            stratum_id: stratum.to_owned(),
            cost_ms,
            predicted_information_gain: score,
        }
    }

    fn request(candidates: Vec<CalibrationCandidate>) -> CalibrationPlanRequest {
        CalibrationPlanRequest {
            target_unit_id: "target-01".to_owned(),
            target_profile_digest: "target-profile-v1".to_owned(),
            phantom_id: "phantom-01".to_owned(),
            phantom_revision: "phantom-rev-1".to_owned(),
            budget_ms: 10,
            maximum_contacts: 2,
            required_strata: vec![
                StratumQuota {
                    stratum_id: "low-force".to_owned(),
                    minimum_contacts: 1,
                },
                StratumQuota {
                    stratum_id: "high-force".to_owned(),
                    minimum_contacts: 1,
                },
            ],
            candidates,
        }
    }

    #[test]
    fn mandatory_stratum_coverage_precedes_high_score_optional_contacts() {
        let plan = plan_calibration(&request(vec![
            candidate("low-a", "low-force", 0.2, 1),
            candidate("high-a", "high-force", 0.1, 1),
            candidate("optional-a", "low-force", 100.0, 1),
        ]))
        .expect("valid request");

        assert_eq!(plan.disposition, PlanDisposition::Complete);
        assert_eq!(
            plan.selected_candidate_ids,
            vec!["low-a".to_owned(), "high-a".to_owned()]
        );
        assert_eq!(plan.coverage_counts["low-force"], 1);
        assert_eq!(plan.coverage_counts["high-force"], 1);
    }

    #[test]
    fn stale_reference_method_or_condition_candidates_are_excluded() {
        let mut expired = candidate("expired", "low-force", 100.0, 1);
        expired.reference_state = ReferenceState::Expired;
        let mut unresolved = candidate("unresolved", "high-force", 100.0, 1);
        unresolved.condition_state = ConditionState::Unresolved;

        let plan = plan_calibration(&request(vec![
            candidate("low-a", "low-force", 1.0, 1),
            candidate("high-a", "high-force", 1.0, 1),
            expired,
            unresolved,
        ]))
        .expect("valid request");

        assert_eq!(
            plan.excluded_candidate_reasons["expired"],
            "REFERENCE_NOT_CURRENT"
        );
        assert_eq!(
            plan.excluded_candidate_reasons["unresolved"],
            "CONDITIONS_NOT_APPLICABLE"
        );
        assert!(!plan.selected_candidate_ids.contains(&"expired".to_owned()));
        assert!(!plan.selected_candidate_ids.contains(&"unresolved".to_owned()));
    }

    #[test]
    fn target_unit_profile_and_phantom_revision_are_exactly_bound() {
        let mut wrong_unit = candidate("wrong-unit", "low-force", 10.0, 1);
        wrong_unit.target_unit_id = "other-target".to_owned();
        let mut wrong_profile = candidate("wrong-profile", "high-force", 10.0, 1);
        wrong_profile.target_profile_digest = "other-profile".to_owned();

        let plan = plan_calibration(&request(vec![wrong_unit, wrong_profile]))
            .expect("valid request with exclusions");

        assert_eq!(plan.disposition, PlanDisposition::IncompleteCoverage);
        assert_eq!(
            plan.excluded_candidate_reasons["wrong-unit"],
            "TARGET_UNIT_MISMATCH"
        );
        assert_eq!(
            plan.excluded_candidate_reasons["wrong-profile"],
            "TARGET_PROFILE_MISMATCH"
        );
    }

    #[test]
    fn budget_shortfall_returns_partial_plan_without_claiming_coverage() {
        let mut req = request(vec![
            candidate("low-a", "low-force", 1.0, 8),
            candidate("high-a", "high-force", 1.0, 8),
        ]);
        req.budget_ms = 10;

        let plan = plan_calibration(&req).expect("valid request");

        assert_eq!(plan.disposition, PlanDisposition::BudgetInsufficient);
        assert_eq!(plan.selected_candidate_ids.len(), 1);
        assert_eq!(plan.missing_strata.len(), 1);
        assert!(plan.total_cost_ms <= req.budget_ms);
    }

    #[test]
    fn ranking_is_deterministic_with_candidate_id_tie_break() {
        let mut req = request(vec![
            candidate("z-candidate", "low-force", 1.0, 1),
            candidate("a-candidate", "low-force", 1.0, 1),
            candidate("high-a", "high-force", 1.0, 1),
        ]);
        req.required_strata[0].minimum_contacts = 1;

        let first = plan_calibration(&req).expect("valid request");
        let second = plan_calibration(&req).expect("valid request");

        assert_eq!(first.selected_candidate_ids, second.selected_candidate_ids);
        assert_eq!(first.selected_candidate_ids[0], "a-candidate");
    }

    #[test]
    fn duplicate_candidate_and_stratum_identities_fail_closed() {
        let duplicate_candidate = candidate("same", "low-force", 1.0, 1);
        let mut req = request(vec![
            duplicate_candidate.clone(),
            duplicate_candidate,
            candidate("high-a", "high-force", 1.0, 1),
        ]);
        assert!(plan_calibration(&req).is_err());

        req.candidates = vec![
            candidate("low-a", "low-force", 1.0, 1),
            candidate("high-a", "high-force", 1.0, 1),
        ];
        req.required_strata.push(StratumQuota {
            stratum_id: "low-force".to_owned(),
            minimum_contacts: 1,
        });
        assert!(plan_calibration(&req).is_err());
    }

    #[test]
    fn nonfinite_or_negative_model_priority_is_rejected_not_selected() {
        let mut bad = candidate("bad", "low-force", 1.0, 1);
        bad.predicted_information_gain = f64::NAN;
        let plan = plan_calibration(&request(vec![
            bad,
            candidate("high-a", "high-force", 1.0, 1),
        ]))
        .expect("candidate with invalid ranking is excluded");

        assert_eq!(
            plan.excluded_candidate_reasons["bad"],
            "INVALID_PRIORITY_SCORE"
        );
        assert_eq!(plan.disposition, PlanDisposition::IncompleteCoverage);
    }
}
