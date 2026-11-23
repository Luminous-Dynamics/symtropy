//! Exact confidence-budget composition over independently evidenced statistical claims.
//!
//! This module verifies that required claim classes are present, scope-compatible and
//! within a predeclared alpha budget. It does not validate the underlying statistical
//! theorems, authenticate issuers, or turn structural receipts into probability claims.

use crate::decision_identifiability::Rational;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const CORPUS_SCHEMA: &str = "luminous.orbital-assurance-budget.corpus.v0.1";
const CLAIM_CEILING: &str = "exact-union-bound-assurance-budget-algebra-only";
const REQUIRED_CLASSES: [&str; 3] = [
    "OBSERVATION_WORLD_COVERAGE",
    "LOSS_MODEL_COVERAGE",
    "REGRET_SCORE_CALIBRATION",
];
const MAX_CLAIMS: usize = 128;
const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AssuranceClaim {
    pub claim_id: String,
    pub claim_class: String,
    pub guarantee_kind: String,
    pub claim_scope: String,
    /// Fixed evaluation ID or declared anytime horizon ID.
    pub scope_id: String,
    pub evidence_status: String,
    pub evidence_receipt_id: Option<String>,
    /// Statistical guarantee receipt; structural-only records must not claim one.
    pub claim_receipt_id: Option<String>,
    /// Predeclared allocation receipt for the claim/multiplicity family.
    pub multiplicity_family_id: Option<String>,
    pub multiplicity_allocation_receipt_id: Option<String>,
    /// None for structural-only evidence. Statistical claims require alpha in (0,1).
    pub alpha: Option<Rational>,
    /// Digest of the source contract or proof manifest.
    pub source_contract_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AssuranceBudgetCorpus {
    pub schema: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub assurance_budget_id: String,
    pub requested_scope: String,
    pub requested_scope_id: String,
    pub total_alpha: Rational,
    pub expected_claim_ids: Vec<String>,
    pub expected_claim_ids_sha256: String,
    /// SHA-256 over canonical claim records sorted by claim_id.
    pub claims_sha256: String,
    pub claims: Vec<AssuranceClaim>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClaimReport {
    pub claim_id: String,
    pub claim_class: String,
    pub guarantee_kind: String,
    pub claim_scope: String,
    pub evidence_status: String,
    pub composable: bool,
    pub exclusion_reason: Option<String>,
    pub alpha: Option<Rational>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AssuranceBudgetResult {
    pub result: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub assurance_budget_id: String,
    pub requested_scope: String,
    pub requested_scope_id: String,
    pub status: String,
    pub total_alpha: Rational,
    /// Sum of the declared statistical allocations, even when the claims cannot compose.
    pub alpha_allocated: Rational,
    /// Set only when every statistical claim is admissible and the budget is not exceeded.
    pub alpha_used: Option<Rational>,
    pub alpha_remaining: Option<Rational>,
    /// Union-bound lower confidence for the conjunction of registered events.
    pub joint_lower_confidence: Option<Rational>,
    pub missing_required_classes: Vec<String>,
    pub noncomposable_claim_ids: Vec<String>,
    pub claim_reports: Vec<ClaimReport>,
    pub independence_assumed: bool,
    pub physical_qualification_granted: bool,
    pub authority_granted: bool,
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty() && value.is_ascii()
}

fn digest_ids(ids: &[String]) -> Result<String, String> {
    let mut sorted = ids.to_vec();
    sorted.sort();
    let bytes = serde_json::to_vec(&sorted)
        .map_err(|error| format!("failed to serialize canonical claim IDs: {error}"))?;
    Ok(sha256_hex(&bytes))
}

fn canonical_claims(claims: &[AssuranceClaim]) -> Result<Value, String> {
    let mut sorted = claims.to_vec();
    sorted.sort_by(|left, right| left.claim_id.cmp(&right.claim_id));
    serde_json::to_value(sorted)
        .map_err(|error| format!("failed to encode canonical assurance claims: {error}"))
}

fn digest_claims(claims: &[AssuranceClaim]) -> Result<String, String> {
    let canonical = canonical_claims(claims)?;
    let bytes = serde_json::to_vec(&canonical)
        .map_err(|error| format!("failed to serialize assurance claims: {error}"))?;
    Ok(sha256_hex(&bytes))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn validate(corpus: &AssuranceBudgetCorpus) -> Result<(), String> {
    if corpus.schema != CORPUS_SCHEMA
        || corpus.evidence_class != "synthetic"
        || corpus.claim_ceiling != CLAIM_CEILING
        || !valid_id(&corpus.assurance_budget_id)
        || !valid_id(&corpus.requested_scope_id)
        || !matches!(
            corpus.requested_scope.as_str(),
            "FIXED_EVALUATION" | "ANYTIME"
        )
        || corpus.claims.is_empty()
        || corpus.claims.len() > MAX_CLAIMS
    {
        return Err("unsupported assurance budget corpus or requested scope".to_owned());
    }

    corpus
        .total_alpha
        .validate()
        .map_err(|error| format!("total alpha is invalid: {error}"))?;
    if corpus.total_alpha.numerator == 0
        || corpus.total_alpha.numerator >= corpus.total_alpha.denominator
    {
        return Err("total alpha must be strictly between zero and one".to_owned());
    }

    let mut expected_ids = BTreeSet::new();
    for claim_id in &corpus.expected_claim_ids {
        if !valid_id(claim_id) || !expected_ids.insert(claim_id.as_str()) {
            return Err("planned claim IDs must be unique nonempty ASCII strings".to_owned());
        }
    }
    if expected_ids.len() != corpus.claims.len() || expected_ids.is_empty() {
        return Err("planned claim inventory must match the observed claim count".to_owned());
    }
    if digest_ids(&corpus.expected_claim_ids)? != corpus.expected_claim_ids_sha256 {
        return Err("planned claim inventory digest mismatch".to_owned());
    }

    let mut observed_ids = BTreeSet::new();
    let mut claim_receipts = BTreeSet::new();
    for claim in &corpus.claims {
        if !valid_id(&claim.claim_id)
            || !observed_ids.insert(claim.claim_id.as_str())
            || !valid_id(&claim.scope_id)
            || !valid_sha256(&claim.source_contract_sha256)
            || !matches!(
                claim.claim_class.as_str(),
                "OBSERVATION_WORLD_COVERAGE"
                    | "LOSS_MODEL_COVERAGE"
                    | "REGRET_SCORE_CALIBRATION"
                    | "TRANSPORTABILITY"
                    | "MULTIPLICITY_FAMILY"
            )
            || !matches!(
                claim.guarantee_kind.as_str(),
                "STATISTICAL_GUARANTEE" | "STRUCTURAL_ONLY"
            )
            || !matches!(claim.claim_scope.as_str(), "FIXED_EVALUATION" | "ANYTIME")
            || !matches!(
                claim.evidence_status.as_str(),
                "admissible" | "indeterminate" | "out_of_scope"
            )
        {
            return Err("assurance claim identity, class, scope or digest is invalid".to_owned());
        }

        match claim.guarantee_kind.as_str() {
            "STATISTICAL_GUARANTEE" => {
                let alpha = claim
                    .alpha
                    .as_ref()
                    .ok_or_else(|| format!("statistical claim {} has no alpha", claim.claim_id))?;
                alpha
                    .validate()
                    .map_err(|error| format!("claim alpha is invalid: {error}"))?;
                if alpha.numerator == 0 || alpha.numerator >= alpha.denominator {
                    return Err(format!(
                        "statistical claim {} alpha must be strictly between zero and one",
                        claim.claim_id
                    ));
                }
                for (value, field) in [
                    (claim.evidence_receipt_id.as_deref(), "evidence receipt"),
                    (claim.claim_receipt_id.as_deref(), "claim receipt"),
                    (
                        claim.multiplicity_family_id.as_deref(),
                        "multiplicity family",
                    ),
                    (
                        claim.multiplicity_allocation_receipt_id.as_deref(),
                        "multiplicity allocation receipt",
                    ),
                ] {
                    if value.is_none_or(|item| !valid_id(item)) {
                        return Err(format!(
                            "statistical claim {} lacks a valid {field}",
                            claim.claim_id
                        ));
                    }
                }
                let receipt_id = claim.claim_receipt_id.as_deref().ok_or_else(|| {
                    format!("statistical claim {} has no claim receipt", claim.claim_id)
                })?;
                if !claim_receipts.insert(receipt_id) {
                    return Err(
                        "statistical claim receipt IDs must not be double-counted".to_owned()
                    );
                }
            }
            "STRUCTURAL_ONLY" => {
                if claim.alpha.is_some()
                    || claim.claim_receipt_id.is_some()
                    || claim.multiplicity_allocation_receipt_id.is_some()
                {
                    return Err(format!(
                        "structural-only claim {} must not carry a statistical alpha or claim allocation",
                        claim.claim_id
                    ));
                }
                if claim
                    .evidence_receipt_id
                    .as_deref()
                    .is_none_or(|value| !valid_id(value))
                {
                    return Err(format!(
                        "structural-only claim {} requires a provenance receipt",
                        claim.claim_id
                    ));
                }
            }
            _ => return Err("unsupported guarantee kind".to_owned()),
        }
    }

    if observed_ids != expected_ids {
        return Err("observed claim IDs do not match the upstream planned inventory".to_owned());
    }
    if digest_claims(&corpus.claims)? != corpus.claims_sha256 {
        return Err("assurance claim manifest digest mismatch".to_owned());
    }
    Ok(())
}

fn claim_exclusion_reason(
    claim: &AssuranceClaim,
    requested_scope: &str,
    requested_scope_id: &str,
) -> Option<String> {
    if claim.guarantee_kind == "STRUCTURAL_ONLY" {
        return Some("STRUCTURAL_ONLY_NOT_A_STATISTICAL_GUARANTEE".to_owned());
    }
    if claim.evidence_status != "admissible" {
        return Some(format!(
            "EVIDENCE_{}",
            claim.evidence_status.to_ascii_uppercase()
        ));
    }
    if claim.scope_id != requested_scope_id {
        return Some("SCOPE_ID_MISMATCH".to_owned());
    }
    if requested_scope == "ANYTIME" && claim.claim_scope != "ANYTIME" {
        return Some("FIXED_TIME_CLAIM_CANNOT_SUPPORT_ANYTIME_REQUEST".to_owned());
    }
    None
}

pub fn evaluate(corpus: &AssuranceBudgetCorpus) -> Result<AssuranceBudgetResult, String> {
    validate(corpus)?;

    let mut claims = corpus.claims.iter().collect::<Vec<_>>();
    claims.sort_by(|left, right| left.claim_id.cmp(&right.claim_id));

    let mut alpha_allocated = Rational {
        numerator: 0,
        denominator: 1,
    };
    for claim in &claims {
        if claim.guarantee_kind == "STATISTICAL_GUARANTEE" {
            alpha_allocated =
                alpha_allocated.checked_add(claim.alpha.as_ref().ok_or_else(|| {
                    format!("statistical claim {} has no alpha", claim.claim_id)
                })?)?;
        }
    }
    alpha_allocated = alpha_allocated.canonicalized();

    let mut reports = Vec::with_capacity(claims.len());
    let mut noncomposable_claim_ids = Vec::new();
    let mut composable_classes = BTreeSet::new();
    let mut scope_mismatch = false;
    let mut evidence_indeterminate = false;

    for claim in &claims {
        let reason =
            claim_exclusion_reason(claim, &corpus.requested_scope, &corpus.requested_scope_id);
        let composable = reason.is_none();
        if !composable {
            noncomposable_claim_ids.push(claim.claim_id.clone());
            match reason.as_deref() {
                Some("SCOPE_ID_MISMATCH")
                | Some("FIXED_TIME_CLAIM_CANNOT_SUPPORT_ANYTIME_REQUEST") => {
                    scope_mismatch = true;
                }
                Some(reason) if reason.starts_with("EVIDENCE_") => {
                    evidence_indeterminate = true;
                }
                _ => {}
            }
        } else {
            composable_classes.insert(claim.claim_class.as_str());
        }

        reports.push(ClaimReport {
            claim_id: claim.claim_id.clone(),
            claim_class: claim.claim_class.clone(),
            guarantee_kind: claim.guarantee_kind.clone(),
            claim_scope: claim.claim_scope.clone(),
            evidence_status: claim.evidence_status.clone(),
            composable,
            exclusion_reason: reason,
            alpha: claim.alpha.as_ref().map(Rational::canonicalized),
        });
    }

    let missing_required_classes = REQUIRED_CLASSES
        .iter()
        .filter(|claim_class| !composable_classes.contains(**claim_class))
        .map(|claim_class| (*claim_class).to_owned())
        .collect::<Vec<_>>();

    let (status, alpha_used, alpha_remaining, joint_lower_confidence) = if scope_mismatch {
        ("ASSURANCE_SCOPE_MISMATCH", None, None, None)
    } else if evidence_indeterminate || !missing_required_classes.is_empty() {
        ("ASSURANCE_INDETERMINATE", None, None, None)
    } else if alpha_allocated.compare_exact(&corpus.total_alpha) == std::cmp::Ordering::Greater {
        (
            "ASSURANCE_BUDGET_EXCEEDED",
            Some(alpha_allocated.clone()),
            None,
            None,
        )
    } else {
        let remaining = corpus
            .total_alpha
            .checked_sub(&alpha_allocated)
            .map_err(|error| format!("alpha remaining underflowed: {error}"))?
            .canonicalized();
        let lower_confidence = Rational {
            numerator: 1,
            denominator: 1,
        }
        .checked_sub(&alpha_allocated)
        .map_err(|error| format!("union-bound confidence underflowed: {error}"))?
        .canonicalized();
        let composed_status = if corpus.requested_scope == "ANYTIME" {
            "ASSURANCE_COMPOSED_ANYTIME"
        } else {
            "ASSURANCE_COMPOSED_FIXED_EVALUATION"
        };
        (
            composed_status,
            Some(alpha_allocated.clone()),
            Some(remaining),
            Some(lower_confidence),
        )
    };

    Ok(AssuranceBudgetResult {
        result: "pass".to_owned(),
        evidence_class: corpus.evidence_class.clone(),
        claim_ceiling: corpus.claim_ceiling.clone(),
        assurance_budget_id: corpus.assurance_budget_id.clone(),
        requested_scope: corpus.requested_scope.clone(),
        requested_scope_id: corpus.requested_scope_id.clone(),
        status: status.to_owned(),
        total_alpha: corpus.total_alpha.canonicalized(),
        alpha_allocated,
        alpha_used,
        alpha_remaining,
        joint_lower_confidence,
        missing_required_classes,
        noncomposable_claim_ids,
        claim_reports: reports,
        independence_assumed: false,
        physical_qualification_granted: false,
        authority_granted: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sha(value: &str) -> String {
        hex_sha256(value.as_bytes())
    }

    fn claim(
        id: &str,
        class: &str,
        alpha: Option<(u64, u64)>,
        scope: &str,
        status: &str,
    ) -> AssuranceClaim {
        let statistical = alpha.is_some();
        AssuranceClaim {
            claim_id: id.to_owned(),
            claim_class: class.to_owned(),
            guarantee_kind: if statistical {
                "STATISTICAL_GUARANTEE".to_owned()
            } else {
                "STRUCTURAL_ONLY".to_owned()
            },
            claim_scope: scope.to_owned(),
            scope_id: "scope-v0".to_owned(),
            evidence_status: status.to_owned(),
            evidence_receipt_id: Some(format!("evidence-{id}")),
            claim_receipt_id: statistical.then(|| format!("claim-{id}")),
            multiplicity_family_id: statistical.then(|| format!("family-{id}")),
            multiplicity_allocation_receipt_id: statistical.then(|| format!("allocation-{id}")),
            alpha: alpha.map(|(numerator, denominator)| Rational {
                numerator,
                denominator,
            }),
            source_contract_sha256: sha(&format!("source-{id}")),
        }
    }

    fn corpus(
        requested_scope: &str,
        total_alpha: Rational,
        claims: Vec<AssuranceClaim>,
    ) -> AssuranceBudgetCorpus {
        let mut ids = claims
            .iter()
            .map(|item| item.claim_id.clone())
            .collect::<Vec<_>>();
        ids.sort();
        AssuranceBudgetCorpus {
            schema: CORPUS_SCHEMA.to_owned(),
            evidence_class: "synthetic".to_owned(),
            claim_ceiling: CLAIM_CEILING.to_owned(),
            assurance_budget_id: "budget-v0".to_owned(),
            requested_scope: requested_scope.to_owned(),
            requested_scope_id: "scope-v0".to_owned(),
            total_alpha,
            expected_claim_ids_sha256: digest_ids(&ids).unwrap(),
            expected_claim_ids: ids,
            claims_sha256: digest_claims(&claims).unwrap(),
            claims,
        }
    }

    fn required_claims(scope: &str) -> Vec<AssuranceClaim> {
        vec![
            claim(
                "world",
                "OBSERVATION_WORLD_COVERAGE",
                Some((1, 100)),
                scope,
                "admissible",
            ),
            claim(
                "loss",
                "LOSS_MODEL_COVERAGE",
                Some((2, 100)),
                scope,
                "admissible",
            ),
            claim(
                "regret",
                "REGRET_SCORE_CALIBRATION",
                Some((2, 100)),
                scope,
                "admissible",
            ),
        ]
    }

    #[test]
    fn fixed_evaluation_union_bound_composes_exactly() {
        let input = corpus(
            "FIXED_EVALUATION",
            Rational {
                numerator: 5,
                denominator: 100,
            },
            required_claims("FIXED_EVALUATION"),
        );
        let result = evaluate(&input).unwrap();
        assert_eq!(result.status, "ASSURANCE_COMPOSED_FIXED_EVALUATION");
        assert_eq!(result.alpha_used.unwrap().numerator, 1);
        assert_eq!(result.alpha_used.unwrap().denominator, 20);
        assert_eq!(result.alpha_remaining.unwrap().numerator, 0);
        assert_eq!(result.joint_lower_confidence.unwrap().numerator, 19);
        assert_eq!(result.joint_lower_confidence.unwrap().denominator, 20);
        assert!(!result.independence_assumed);
    }

    #[test]
    fn budget_overrun_is_rejected_without_joint_confidence() {
        let mut claims = required_claims("FIXED_EVALUATION");
        claims[0].alpha = Some(Rational {
            numerator: 2,
            denominator: 100,
        });
        let input = corpus(
            "FIXED_EVALUATION",
            Rational {
                numerator: 5,
                denominator: 100,
            },
            claims,
        );
        let result = evaluate(&input).unwrap();
        assert_eq!(result.status, "ASSURANCE_BUDGET_EXCEEDED");
        assert!(result.joint_lower_confidence.is_none());
        assert!(result.alpha_remaining.is_none());
    }

    #[test]
    fn missing_required_layer_is_indeterminate() {
        let claims = required_claims("FIXED_EVALUATION")
            .into_iter()
            .filter(|item| item.claim_class != "LOSS_MODEL_COVERAGE")
            .collect();
        let input = corpus(
            "FIXED_EVALUATION",
            Rational {
                numerator: 5,
                denominator: 100,
            },
            claims,
        );
        let result = evaluate(&input).unwrap();
        assert_eq!(result.status, "ASSURANCE_INDETERMINATE");
        assert_eq!(result.missing_required_classes, vec!["LOSS_MODEL_COVERAGE"]);
        assert!(result.joint_lower_confidence.is_none());
    }

    #[test]
    fn structural_only_receipt_never_counts_as_a_statistical_claim() {
        let mut claims = required_claims("FIXED_EVALUATION");
        claims[1] = claim(
            "loss",
            "LOSS_MODEL_COVERAGE",
            None,
            "FIXED_EVALUATION",
            "admissible",
        );
        let input = corpus(
            "FIXED_EVALUATION",
            Rational {
                numerator: 5,
                denominator: 100,
            },
            claims,
        );
        let result = evaluate(&input).unwrap();
        assert_eq!(result.status, "ASSURANCE_INDETERMINATE");
        assert_eq!(result.missing_required_classes, vec!["LOSS_MODEL_COVERAGE"]);
    }

    #[test]
    fn fixed_time_claim_cannot_support_anytime_request() {
        let input = corpus(
            "ANYTIME",
            Rational {
                numerator: 5,
                denominator: 100,
            },
            required_claims("FIXED_EVALUATION"),
        );
        let result = evaluate(&input).unwrap();
        assert_eq!(result.status, "ASSURANCE_SCOPE_MISMATCH");
        assert!(result.joint_lower_confidence.is_none());
    }

    #[test]
    fn anytime_claims_can_compose_for_anytime_request() {
        let input = corpus(
            "ANYTIME",
            Rational {
                numerator: 5,
                denominator: 100,
            },
            required_claims("ANYTIME"),
        );
        let result = evaluate(&input).unwrap();
        assert_eq!(result.status, "ASSURANCE_COMPOSED_ANYTIME");
        assert_eq!(result.joint_lower_confidence.unwrap().numerator, 19);
    }

    #[test]
    fn structural_digest_is_not_a_statistical_alpha_allocation() {
        let mut claims = required_claims("FIXED_EVALUATION");
        let structural = claim(
            "transport",
            "TRANSPORTABILITY",
            None,
            "FIXED_EVALUATION",
            "admissible",
        );
        claims.push(structural);
        let input = corpus(
            "FIXED_EVALUATION",
            Rational {
                numerator: 5,
                denominator: 100,
            },
            claims,
        );
        let result = evaluate(&input).unwrap();
        assert_eq!(result.status, "ASSURANCE_COMPOSED_FIXED_EVALUATION");
        assert!(
            result
                .noncomposable_claim_ids
                .contains(&"transport".to_owned())
        );
        assert_eq!(result.alpha_used.unwrap().numerator, 1);
    }

    #[test]
    fn duplicate_statistical_receipt_is_rejected() {
        let mut claims = required_claims("FIXED_EVALUATION");
        claims[1].claim_receipt_id = claims[0].claim_receipt_id.clone();
        let input = corpus(
            "FIXED_EVALUATION",
            Rational {
                numerator: 5,
                denominator: 100,
            },
            claims,
        );
        assert!(evaluate(&input).is_err());
    }

    #[test]
    fn no_physical_qualification_or_authority_is_granted() {
        let input = corpus(
            "FIXED_EVALUATION",
            Rational {
                numerator: 5,
                denominator: 100,
            },
            required_claims("FIXED_EVALUATION"),
        );
        let result = evaluate(&input).unwrap();
        assert!(!result.physical_qualification_granted);
        assert!(!result.authority_granted);
    }
}
