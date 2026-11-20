use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const TOTAL_BASIS_POINTS: u64 = 10_000;
const TOLERANCE: f64 = 1.0e-9;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Case {
    pub schema_version: String,
    pub case_id: String,
    pub case_maturity: EvidenceMaturity,
    pub as_of_hour: f64,
    pub horizon_end_hour: f64,
    pub source_policy_certificate_ref: String,
    pub source_policy_certificate_sha256: String,
    pub probability_envelope_ref: String,
    pub probability_envelope_sha256: String,
    pub probability_envelope: ProbabilityEnvelope,
    pub service_outcome_manifest_ref: String,
    pub service_outcome_manifest_sha256: String,
    pub service_outcome_manifest: ServiceOutcomeManifest,
    pub risk_limit_manifest_ref: String,
    pub risk_limit_manifest_sha256: String,
    pub risk_limit_manifest: RiskLimitManifest,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceMaturity {
    SyntheticFixture,
    HistoricalReplay,
    FieldObserved,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProbabilityEnvelope {
    pub schema_version: String,
    pub ambiguity_set_ref: String,
    pub source_evidence_ref: String,
    pub branches: Vec<BranchProbability>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BranchProbability {
    pub branch_id: String,
    pub point_basis_points: u32,
    pub lower_basis_points: u32,
    pub upper_basis_points: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ServiceOutcomeManifest {
    pub schema_version: String,
    pub manifest_id: String,
    pub time_origin_ref: String,
    pub horizon_ref: String,
    pub beneficiary_profile_ref: String,
    pub service_model_family_ref: String,
    pub branches: Vec<ServiceBranch>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RiskLimitManifest {
    pub schema_version: String,
    pub manifest_id: String,
    pub policy_ref: String,
    pub limits: Vec<MetricRiskLimit>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MetricRiskLimit {
    pub metric_id: String,
    pub unit: String,
    pub maximum_robust_expected_unserved_quantity: f64,
    pub authority_ref: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ServiceBranch {
    pub branch_id: String,
    pub service_assessments: Vec<ServiceAssessment>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ServiceAssessment {
    pub metric_id: String,
    pub service_type: String,
    pub beneficiary_scope: BeneficiaryScope,
    pub cohort_id: String,
    pub unit: String,
    pub denominator_ref: String,
    pub target_horizon_ref: String,
    pub service_fraction_lower_bound: f64,
    pub service_fraction_upper_bound: f64,
    pub minimum_service_floor: f64,
    pub unserved_quantity: f64,
    pub acceptance_status: AcceptanceStatus,
    pub evidence_maturity: EvidenceMaturity,
    pub service_model_ref: String,
    pub service_trajectory_ref: String,
    pub independent_acceptance_ref: String,
    pub evidence_ref: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BeneficiaryScope {
    HouseholdSite,
    Organization,
    Community,
    TownMunicipality,
    CityMetropolitan,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AcceptanceStatus {
    Accepted,
    Pending,
    Rejected,
    Unknown,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MetricRiskResult {
    pub metric_id: String,
    pub service_type: String,
    pub beneficiary_scope: BeneficiaryScope,
    pub cohort_id: String,
    pub unit: String,
    pub point_expected_unserved_quantity: f64,
    pub robust_expected_unserved_quantity: f64,
    pub robust_risk_budget: f64,
    pub ambiguity_premium: f64,
    pub worst_branch_unserved_quantity: f64,
    pub worst_branch_id: String,
    pub floor_breach_count: usize,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FloorViolation {
    pub branch_id: String,
    pub metric_id: String,
    pub cohort_id: String,
    pub observed_lower_bound: f64,
    pub required_floor: f64,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RiskBudgetViolation {
    pub metric_id: String,
    pub robust_expected_unserved_quantity: f64,
    pub risk_budget: f64,
    pub unit: String,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct QualificationReport {
    pub case_id: String,
    pub case_maturity: EvidenceMaturity,
    pub result_status: String,
    pub source_policy_certificate_ref: String,
    pub source_policy_certificate_sha256: String,
    pub probability_envelope_ref: String,
    pub probability_envelope_sha256: String,
    pub service_outcome_manifest_ref: String,
    pub service_outcome_manifest_sha256: String,
    pub risk_limit_manifest_ref: String,
    pub risk_limit_manifest_sha256: String,
    pub point_probability_sum_basis_points: u64,
    pub probability_interval_lower_sum_basis_points: u64,
    pub probability_interval_upper_sum_basis_points: u64,
    pub interval_worst_case_metric_results: Vec<MetricRiskResult>,
    pub floor_violations: Vec<FloorViolation>,
    pub risk_budget_violations: Vec<RiskBudgetViolation>,
    pub inconclusive_reasons: Vec<String>,
    pub claim_boundary: String,
}

pub fn canonical_probability_envelope_sha256(
    envelope: &ProbabilityEnvelope,
) -> Result<String, serde_json::Error> {
    let value = serde_json::to_value(envelope)?;
    let bytes = serde_json::to_vec(&value)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

pub fn canonical_service_manifest_sha256(
    manifest: &ServiceOutcomeManifest,
) -> Result<String, serde_json::Error> {
    let value = serde_json::to_value(manifest)?;
    let bytes = serde_json::to_vec(&value)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

pub fn canonical_risk_limit_manifest_sha256(
    manifest: &RiskLimitManifest,
) -> Result<String, serde_json::Error> {
    let value = serde_json::to_value(manifest)?;
    let bytes = serde_json::to_vec(&value)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn finite_nonnegative(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

fn approximately_equal(left: f64, right: f64) -> bool {
    (left - right).abs() <= TOLERANCE
}

fn validate_case(case: &Case) -> Vec<String> {
    let mut errors = Vec::new();
    if case.schema_version != "0.1.0"
        || case.case_id.trim().is_empty()
        || !finite_nonnegative(case.as_of_hour)
        || !case.horizon_end_hour.is_finite()
        || case.horizon_end_hour <= case.as_of_hour
    {
        errors.push("case identity, version or horizon is invalid".to_string());
    }
    for (reference, digest, label) in [
        (&case.source_policy_certificate_ref, &case.source_policy_certificate_sha256, "source policy certificate"),
        (&case.probability_envelope_ref, &case.probability_envelope_sha256, "probability envelope"),
        (&case.service_outcome_manifest_ref, &case.service_outcome_manifest_sha256, "service outcome manifest"),
        (&case.risk_limit_manifest_ref, &case.risk_limit_manifest_sha256, "risk limit manifest"),
    ] {
        if reference.trim().is_empty()
            || digest.len() != 64
            || !digest.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            errors.push(format!("{label} reference or lowercase SHA-256 digest is invalid"));
        }
    }

    let envelope = &case.probability_envelope;
    if envelope.schema_version != "0.1.0"
        || envelope.ambiguity_set_ref.trim().is_empty()
        || envelope.source_evidence_ref.trim().is_empty()
    {
        errors.push("probability envelope identity or provenance is invalid".to_string());
    }
    if let Ok(actual) = canonical_probability_envelope_sha256(envelope) {
        if actual != case.probability_envelope_sha256 {
            errors.push("probability envelope SHA-256 does not match canonical content".to_string());
        }
    } else {
        errors.push("probability envelope serialization failed".to_string());
    }

    let mut probability_ids = BTreeSet::new();
    let mut point_sum = 0_u64;
    let mut lower_sum = 0_u64;
    let mut upper_sum = 0_u64;
    for branch in &envelope.branches {
        if branch.branch_id.trim().is_empty() || !probability_ids.insert(branch.branch_id.as_str()) {
            errors.push("probability envelope contains an empty or duplicate branch ID".to_string());
        }
        if branch.lower_basis_points > branch.point_basis_points
            || branch.point_basis_points > branch.upper_basis_points
            || branch.upper_basis_points > TOTAL_BASIS_POINTS as u32
        {
            errors.push(format!("branch {} point probability is outside its declared interval", branch.branch_id));
        }
        point_sum += branch.point_basis_points as u64;
        lower_sum += branch.lower_basis_points as u64;
        upper_sum += branch.upper_basis_points as u64;
    }
    if point_sum != TOTAL_BASIS_POINTS {
        errors.push(format!("point branch probabilities sum to {point_sum}, not 10,000"));
    }
    if lower_sum > TOTAL_BASIS_POINTS || upper_sum < TOTAL_BASIS_POINTS {
        errors.push(format!("probability interval simplex is infeasible: lower sum {lower_sum}, upper sum {upper_sum}"));
    }

    let manifest = &case.service_outcome_manifest;
    if manifest.schema_version != "0.1.0"
        || manifest.manifest_id.trim().is_empty()
        || manifest.time_origin_ref.trim().is_empty()
        || manifest.horizon_ref.trim().is_empty()
        || manifest.beneficiary_profile_ref.trim().is_empty()
        || manifest.service_model_family_ref.trim().is_empty()
    {
        errors.push("service outcome manifest identity or scope references are invalid".to_string());
    }
    if let Ok(actual) = canonical_service_manifest_sha256(manifest) {
        if actual != case.service_outcome_manifest_sha256 {
            errors.push("service outcome manifest SHA-256 does not match canonical content".to_string());
        }
    } else {
        errors.push("service outcome manifest serialization failed".to_string());
    }

    let limits = &case.risk_limit_manifest;
    if limits.schema_version != "0.1.0"
        || limits.manifest_id.trim().is_empty()
        || limits.policy_ref.trim().is_empty()
    {
        errors.push("risk limit manifest identity or policy reference is invalid".to_string());
    }
    if let Ok(actual) = canonical_risk_limit_manifest_sha256(limits) {
        if actual != case.risk_limit_manifest_sha256 {
            errors.push("risk limit manifest SHA-256 does not match canonical content".to_string());
        }
    } else {
        errors.push("risk limit manifest serialization failed".to_string());
    }
    let mut risk_limit_ids = BTreeSet::new();
    for limit in &limits.limits {
        if limit.metric_id.trim().is_empty()
            || limit.unit.trim().is_empty()
            || !finite_nonnegative(limit.maximum_robust_expected_unserved_quantity)
            || limit.authority_ref.trim().is_empty()
            || !risk_limit_ids.insert(limit.metric_id.as_str())
        {
            errors.push("risk limit manifest contains an invalid or duplicate metric limit".to_string());
        }
    }

    let manifest_ids: BTreeSet<&str> = manifest.branches.iter().map(|branch| branch.branch_id.as_str()).collect();
    if manifest_ids.len() != manifest.branches.len() || manifest_ids != probability_ids {
        errors.push("service manifest branch IDs must match probability envelope branch IDs exactly".to_string());
    }

    for branch in &manifest.branches {
        let mut metric_ids = BTreeSet::new();
        for assessment in &branch.service_assessments {
            if assessment.metric_id.trim().is_empty()
                || !metric_ids.insert(assessment.metric_id.as_str())
                || assessment.service_type.trim().is_empty()
                || assessment.cohort_id.trim().is_empty()
                || assessment.unit.trim().is_empty()
                || assessment.denominator_ref.trim().is_empty()
                || assessment.target_horizon_ref != manifest.horizon_ref
                || assessment.service_model_ref.trim().is_empty()
                || assessment.service_trajectory_ref.trim().is_empty()
                || assessment.independent_acceptance_ref.trim().is_empty()
                || assessment.evidence_ref.trim().is_empty()
            {
                errors.push(format!("branch {} has incomplete or duplicate service assessment identity", branch.branch_id));
            }
            if !assessment.service_fraction_lower_bound.is_finite()
                || !assessment.service_fraction_upper_bound.is_finite()
                || assessment.service_fraction_lower_bound < 0.0
                || assessment.service_fraction_upper_bound > 1.0
                || assessment.service_fraction_lower_bound > assessment.service_fraction_upper_bound
                || !assessment.minimum_service_floor.is_finite()
                || assessment.minimum_service_floor < 0.0
                || assessment.minimum_service_floor > 1.0
                || !finite_nonnegative(assessment.unserved_quantity)
            {
                errors.push(format!("branch {} metric {} has invalid values", branch.branch_id, assessment.metric_id));
            }
        }
    }

    if manifest.branches.is_empty() {
        errors.push("service outcome manifest must contain at least one branch".to_string());
    }
    let manifest_metric_ids: BTreeSet<&str> = manifest.branches.first()
        .map(|branch| branch.service_assessments.iter().map(|assessment| assessment.metric_id.as_str()).collect())
        .unwrap_or_default();
    if risk_limit_ids != manifest_metric_ids {
        errors.push("risk limit metric IDs must match service outcome metric IDs exactly".to_string());
    }
    errors
}

fn maximum_interval_expectation(
    rows: &[(&BranchProbability, f64)],
) -> Result<f64, String> {
    let lower_sum: u64 = rows.iter().map(|(probability, _)| probability.lower_basis_points as u64).sum();
    let upper_sum: u64 = rows.iter().map(|(probability, _)| probability.upper_basis_points as u64).sum();
    if lower_sum > TOTAL_BASIS_POINTS || upper_sum < TOTAL_BASIS_POINTS {
        return Err("probability interval simplex is infeasible for the metric".to_string());
    }
    let mut probabilities: Vec<u64> = rows.iter()
        .map(|(probability, _)| probability.lower_basis_points as u64)
        .collect();
    let mut remaining = TOTAL_BASIS_POINTS - lower_sum;
    let mut order: Vec<usize> = (0..rows.len()).collect();
    order.sort_by(|left, right| rows[*right].1.total_cmp(&rows[*left].1)
        .then_with(|| rows[*left].0.branch_id.cmp(&rows[*right].0.branch_id)));
    for index in order {
        if remaining == 0 {
            break;
        }
        let probability = rows[index].0;
        let room = probability.upper_basis_points as u64 - probability.lower_basis_points as u64;
        let allocation = room.min(remaining);
        probabilities[index] += allocation;
        remaining -= allocation;
    }
    if remaining != 0 {
        return Err("probability interval allocator failed to conserve 10,000 basis points".to_string());
    }
    Ok(rows.iter().zip(probabilities)
        .map(|((_, loss), probability)| loss * probability as f64 / TOTAL_BASIS_POINTS as f64)
        .sum())
}

fn point_expectation(rows: &[(&BranchProbability, f64)]) -> f64 {
    rows.iter().map(|(probability, loss)| {
        *loss * probability.point_basis_points as f64 / TOTAL_BASIS_POINTS as f64
    }).sum()
}

pub fn qualify(case: &Case) -> Result<QualificationReport, Vec<String>> {
    let errors = validate_case(case);
    if !errors.is_empty() {
        return Err(errors);
    }

    let envelope = &case.probability_envelope;
    let manifest = &case.service_outcome_manifest;
    let probabilities: BTreeMap<&str, &BranchProbability> =
        envelope.branches.iter().map(|branch| (branch.branch_id.as_str(), branch)).collect();

    let first_branch = &manifest.branches[0];
    let mut assessment_metadata: BTreeMap<String, (&ServiceAssessment, String)> = BTreeMap::new();
    for assessment in &first_branch.service_assessments {
        assessment_metadata.insert(assessment.metric_id.clone(), (assessment, first_branch.branch_id.clone()));
    }
    let required_metrics: BTreeSet<String> = assessment_metadata.keys().cloned().collect();

    let mut inconclusive_reasons = Vec::new();
    for branch in &manifest.branches {
        let actual_ids: BTreeSet<String> = branch.service_assessments.iter().map(|item| item.metric_id.clone()).collect();
        if actual_ids != required_metrics {
            return Err(vec![format!("branch {} does not contain exactly the same service metric set", branch.branch_id)]);
        }
        for assessment in &branch.service_assessments {
            if assessment.acceptance_status != AcceptanceStatus::Accepted {
                inconclusive_reasons.push(format!(
                    "branch {} metric {} is not independently accepted ({:?})",
                    branch.branch_id, assessment.metric_id, assessment.acceptance_status
                ));
            }
            if assessment.evidence_maturity != case.case_maturity {
                inconclusive_reasons.push(format!(
                    "branch {} metric {} evidence maturity differs from case maturity",
                    branch.branch_id, assessment.metric_id
                ));
            }
        }
    }

    let mut results = Vec::new();
    let mut floor_violations = Vec::new();
    let mut risk_budget_violations = Vec::new();

    for metric_id in &required_metrics {
        let mut rows: Vec<(&BranchProbability, f64)> = Vec::new();
        let mut metadata: Option<&ServiceAssessment> = None;
        let mut worst: Option<(&str, f64)> = None;
        let mut metric_inconclusive = false;
        let mut floor_breach_count = 0_usize;
        for branch in &manifest.branches {
            let assessment = branch.service_assessments.iter()
                .find(|item| &item.metric_id == metric_id)
                .expect("branch metric set was validated");
            let probability = probabilities.get(branch.branch_id.as_str())
                .expect("probability branch identity was validated");
            if let Some(first) = metadata {
                if assessment.unit != first.unit
                    || assessment.service_type != first.service_type
                    || assessment.beneficiary_scope != first.beneficiary_scope
                    || assessment.cohort_id != first.cohort_id
                    || assessment.denominator_ref != first.denominator_ref
                    || assessment.target_horizon_ref != first.target_horizon_ref
                    || assessment.service_model_ref != first.service_model_ref
                    || !approximately_equal(assessment.minimum_service_floor, first.minimum_service_floor)
                {
                    return Err(vec![format!("metric {} changes unit, cohort, denominator, horizon or floor across branches", metric_id)]);
                }
            } else {
                metadata = Some(assessment);
            }

            if assessment.acceptance_status != AcceptanceStatus::Accepted
                || assessment.evidence_maturity != case.case_maturity
            {
                metric_inconclusive = true;
            }
            if assessment.acceptance_status == AcceptanceStatus::Accepted
                && assessment.evidence_maturity == case.case_maturity
                && assessment.service_fraction_lower_bound + TOLERANCE < assessment.minimum_service_floor
            {
                floor_breach_count += 1;
                floor_violations.push(FloorViolation {
                    branch_id: branch.branch_id.clone(),
                    metric_id: metric_id.clone(),
                    cohort_id: assessment.cohort_id.clone(),
                    observed_lower_bound: assessment.service_fraction_lower_bound,
                    required_floor: assessment.minimum_service_floor,
                });
            }
            if worst.is_none_or(|(_, value)| assessment.unserved_quantity > value) {
                worst = Some((branch.branch_id.as_str(), assessment.unserved_quantity));
            }
            rows.push((probability, assessment.unserved_quantity));
        }

        let Some(meta) = metadata else {
            continue;
        };
        if metric_inconclusive {
            continue;
        }
        let robust = maximum_interval_expectation(&rows).map_err(|error| vec![error])?;
        let point = point_expectation(&rows);
        let worst = worst.expect("non-empty manifest has at least one branch");
        let risk_limit = case.risk_limit_manifest.limits.iter()
            .find(|limit| limit.metric_id.as_str() == metric_id.as_str())
            .expect("risk limit identity was validated");
        if risk_limit.unit != meta.unit {
            return Err(vec![format!("risk limit unit for metric {} does not match the service metric", metric_id)]);
        }
        if robust > risk_limit.maximum_robust_expected_unserved_quantity + TOLERANCE {
            risk_budget_violations.push(RiskBudgetViolation {
                metric_id: metric_id.clone(),
                robust_expected_unserved_quantity: robust,
                risk_budget: risk_limit.maximum_robust_expected_unserved_quantity,
                unit: meta.unit.clone(),
            });
        }
        results.push(MetricRiskResult {
            metric_id: metric_id.clone(),
            service_type: meta.service_type.clone(),
            beneficiary_scope: meta.beneficiary_scope,
            cohort_id: meta.cohort_id.clone(),
            unit: meta.unit.clone(),
            point_expected_unserved_quantity: point,
            robust_expected_unserved_quantity: robust,
            robust_risk_budget: risk_limit.maximum_robust_expected_unserved_quantity,
            ambiguity_premium: robust - point,
            worst_branch_unserved_quantity: worst.1,
            worst_branch_id: worst.0.to_string(),
            floor_breach_count,
        });
    }

    let result_status = if !inconclusive_reasons.is_empty() {
        "inconclusive_service_evidence"
    } else if !floor_violations.is_empty() {
        "inadmissible_service_floor_breach"
    } else if !risk_budget_violations.is_empty() {
        "inadmissible_robust_risk_budget_breach"
    } else {
        "service_risk_certificate_consistent"
    };

    Ok(QualificationReport {
        case_id: case.case_id.clone(),
        case_maturity: case.case_maturity,
        result_status: result_status.to_string(),
        source_policy_certificate_ref: case.source_policy_certificate_ref.clone(),
        source_policy_certificate_sha256: case.source_policy_certificate_sha256.clone(),
        probability_envelope_ref: case.probability_envelope_ref.clone(),
        probability_envelope_sha256: case.probability_envelope_sha256.clone(),
        service_outcome_manifest_ref: case.service_outcome_manifest_ref.clone(),
        service_outcome_manifest_sha256: case.service_outcome_manifest_sha256.clone(),
        risk_limit_manifest_ref: case.risk_limit_manifest_ref.clone(),
        risk_limit_manifest_sha256: case.risk_limit_manifest_sha256.clone(),
        point_probability_sum_basis_points: envelope.branches.iter().map(|branch| branch.point_basis_points as u64).sum(),
        probability_interval_lower_sum_basis_points: envelope.branches.iter().map(|branch| branch.lower_basis_points as u64).sum(),
        probability_interval_upper_sum_basis_points: envelope.branches.iter().map(|branch| branch.upper_basis_points as u64).sum(),
        interval_worst_case_metric_results: results,
        floor_violations,
        risk_budget_violations,
        inconclusive_reasons,
        claim_boundary: "Independently accepted branch-level service summary qualification only; not a service simulator, dispatch authority or operational restoration decision".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn refresh_probability_digest(case: &mut Case) {
        case.probability_envelope_sha256 =
            canonical_probability_envelope_sha256(&case.probability_envelope).expect("probability envelope serializes");
    }

    fn refresh_service_digest(case: &mut Case) {
        case.service_outcome_manifest_sha256 =
            canonical_service_manifest_sha256(&case.service_outcome_manifest).expect("service manifest serializes");
    }

    fn fixture() -> Case {
        serde_json::from_str(include_str!("../fixtures/synthetic-grid-service-risk-case-v0.1.json"))
            .expect("fixture must deserialize")
    }

    #[test]
    fn fixture_reports_exact_point_and_interval_worst_case_expectations() {
        let report = qualify(&fixture()).expect("fixture qualifies");
        assert_eq!(report.result_status, "service_risk_certificate_consistent");
        let hospital = report.interval_worst_case_metric_results.iter()
            .find(|item| item.metric_id == "hospital-electricity-mwh").unwrap();
        assert!((hospital.point_expected_unserved_quantity - 4.99).abs() <= TOLERANCE);
        assert!((hospital.robust_expected_unserved_quantity - 5.60).abs() <= TOLERANCE);
        let water = report.interval_worst_case_metric_results.iter()
            .find(|item| item.metric_id == "community-water-m3").unwrap();
        assert!((water.point_expected_unserved_quantity - 117.8).abs() <= TOLERANCE);
        assert!((water.robust_expected_unserved_quantity - 134.5).abs() <= TOLERANCE);
    }

    #[test]
    fn robust_risk_budget_breach_is_reported_per_metric_and_unit() {
        let mut case = fixture();
        case.risk_limit_manifest.limits.iter_mut()
            .find(|limit| limit.metric_id == "community-water-m3").unwrap()
            .maximum_robust_expected_unserved_quantity = 100.0;
        case.risk_limit_manifest_sha256 = canonical_risk_limit_manifest_sha256(&case.risk_limit_manifest).unwrap();
        let report = qualify(&case).expect("risk budget breach is a reportable inadmissibility");
        assert_eq!(report.result_status, "inadmissible_robust_risk_budget_breach");
        assert_eq!(report.risk_budget_violations.len(), 1);
        assert_eq!(report.risk_budget_violations[0].unit, "m3");
    }

    #[test]
    fn risk_limit_manifest_digest_tampering_is_rejected() {
        let mut case = fixture();
        case.risk_limit_manifest.limits[0].maximum_robust_expected_unserved_quantity += 1.0;
        assert!(qualify(&case).is_err());
    }

    #[test]
    fn robust_expectation_is_not_below_point_expectation() {
        let report = qualify(&fixture()).expect("fixture qualifies");
        assert!(report.interval_worst_case_metric_results.iter()
            .all(|item| item.robust_expected_unserved_quantity + TOLERANCE >= item.point_expected_unserved_quantity));
    }

    #[test]
    fn every_branch_must_preserve_hard_service_floors() {
        let mut case = fixture();
        let least_served = case.service_outcome_manifest.branches.iter_mut()
            .find(|branch| branch.branch_id == "negative-negative-repair").unwrap();
        least_served.service_assessments.iter_mut()
            .find(|item| item.metric_id == "neighbourhood-electricity-mwh").unwrap()
            .service_fraction_lower_bound = 0.79;
        refresh_service_digest(&mut case);
        let report = qualify(&case).expect("floor breach is a reportable inadmissibility");
        assert_eq!(report.result_status, "inadmissible_service_floor_breach");
        assert_eq!(report.floor_violations.len(), 1);
    }

    #[test]
    fn unaccepted_service_evidence_is_inconclusive_not_zero() {
        let mut case = fixture();
        let unknown = &mut case.service_outcome_manifest.branches[0].service_assessments[0];
        unknown.acceptance_status = AcceptanceStatus::Unknown;
        unknown.service_fraction_lower_bound = 0.0;
        refresh_service_digest(&mut case);
        let report = qualify(&case).expect("unknown service evidence yields an inconclusive report");
        assert_eq!(report.result_status, "inconclusive_service_evidence");
        assert!(!report.inconclusive_reasons.is_empty());
        assert_eq!(report.interval_worst_case_metric_results.len(), 4);
        assert!(report.floor_violations.is_empty(), "unaccepted data cannot assert a hard-floor breach");
    }

    #[test]
    fn point_probabilities_must_lie_inside_intervals() {
        let mut case = fixture();
        case.probability_envelope.branches[0].upper_basis_points = 2_900;
        refresh_probability_digest(&mut case);
        assert!(qualify(&case).is_err());
    }

    #[test]
    fn interval_simplex_must_be_feasible() {
        let mut case = fixture();
        for branch in &mut case.probability_envelope.branches {
            branch.lower_basis_points = 4_000;
            branch.upper_basis_points = 4_500;
        }
        refresh_probability_digest(&mut case);
        assert!(qualify(&case).is_err());
    }

    #[test]
    fn every_policy_branch_must_have_the_same_metric_set() {
        let mut case = fixture();
        case.service_outcome_manifest.branches[0].service_assessments.pop();
        refresh_service_digest(&mut case);
        assert!(qualify(&case).is_err());
    }

    #[test]
    fn service_units_cannot_change_across_branches() {
        let mut case = fixture();
        case.service_outcome_manifest.branches[1].service_assessments[0].unit = "customer_hours".to_string();
        refresh_service_digest(&mut case);
        assert!(qualify(&case).is_err());
    }

    #[test]
    fn risk_limit_unit_must_match_service_metric_unit() {
        let mut case = fixture();
        case.risk_limit_manifest.limits.iter_mut()
            .find(|limit| limit.metric_id == "community-water-m3").unwrap()
            .unit = "MWh".to_string();
        case.risk_limit_manifest_sha256 = canonical_risk_limit_manifest_sha256(&case.risk_limit_manifest).unwrap();
        assert!(qualify(&case).is_err());
    }

    #[test]
    fn service_model_revision_cannot_change_between_policy_branches() {
        let mut case = fixture();
        case.service_outcome_manifest.branches[1].service_assessments[0].service_model_ref =
            "symtropy://6235/model/electricity/unreviewed-v2".to_string();
        refresh_service_digest(&mut case);
        assert!(qualify(&case).is_err());
    }

    #[test]
    fn separate_service_metrics_are_not_collapsed() {
        let report = qualify(&fixture()).expect("fixture qualifies");
        let ids: BTreeSet<&str> = report.interval_worst_case_metric_results.iter().map(|item| item.metric_id.as_str()).collect();
        assert_eq!(ids.len(), 5);
        assert!(report.interval_worst_case_metric_results.iter().any(|item| item.unit == "MWh"));
        assert!(report.interval_worst_case_metric_results.iter().any(|item| item.unit == "m3"));
        assert!(report.interval_worst_case_metric_results.iter().any(|item| item.unit == "customer_hours"));
    }

    #[test]
    fn service_manifest_digest_tampering_is_rejected() {
        let mut case = fixture();
        case.service_outcome_manifest.branches[0].service_assessments[0].unserved_quantity += 1.0;
        assert!(qualify(&case).is_err());
    }

    #[test]
    fn probability_envelope_digest_tampering_is_rejected() {
        let mut case = fixture();
        case.probability_envelope.branches[0].lower_basis_points += 1;
        assert!(qualify(&case).is_err());
    }

    #[test]
    fn deterministic_replay_produces_identical_receipts() {
        let case = fixture();
        let first = qualify(&case).unwrap();
        let second = qualify(&case).unwrap();
        assert_eq!(serde_json::to_string(&first).unwrap(), serde_json::to_string(&second).unwrap());
    }
}
