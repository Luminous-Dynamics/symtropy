//! Multi-outcome locomotion safety-vector and proxy-shift auditor.
//!
//! Consumes canonical event classifications and exposure evidence. It does not
//! classify sensor data, authenticate labels, or infer physical safety.

use std::collections::{BTreeMap, BTreeSet};

pub const COVERAGE_COMPLETE_PPM: u32 = 1_000_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaxonomyCategory {
    pub id: String,
    pub adverse: bool,
    pub required: bool,
    pub severity_unit_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Taxonomy {
    pub version: String,
    pub digest: String,
    pub categories: Vec<TaxonomyCategory>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exposure {
    pub value: u64,
    pub unit_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryCoverage {
    pub category_id: String,
    pub observed_exposure: u64,
    /// Whether the policy's declared safety objective explicitly observes or
    /// penalizes this category. This is metadata, not an instruction to weight
    /// the final report.
    pub objective_included: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventRecord {
    /// Canonical event identifier for one physical incident/category pair.
    pub event_id: String,
    /// Links multiple typed outcomes from the same physical incident.
    pub physical_event_id: String,
    pub category_id: String,
    pub time_ns: u64,
    pub severity: u64,
    pub severity_unit_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyResult {
    pub policy_id: String,
    pub matched_stratum_digest: String,
    pub taxonomy_version: String,
    pub taxonomy_digest: String,
    pub detector_digest: String,
    pub exposure: Exposure,
    pub category_coverage: Vec<CategoryCoverage>,
    pub events: Vec<EventRecord>,
    pub unknown_event_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafetyVectorProblem {
    pub taxonomy: Taxonomy,
    pub baseline: PolicyResult,
    pub candidate: PolicyResult,
    pub primary_claim_category_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Improved,
    Equal,
    Worsened,
    Withheld,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryComparison {
    pub category_id: String,
    pub adverse: bool,
    pub baseline_event_count: u64,
    pub candidate_event_count: u64,
    pub event_rate_direction: Direction,
    pub baseline_severity_sum: u128,
    pub candidate_severity_sum: u128,
    pub severity_load_direction: Direction,
    pub baseline_max_severity: u64,
    pub candidate_max_severity: u64,
    pub maximum_severity_direction: Direction,
    pub measurement_complete: bool,
    pub objective_included_baseline: bool,
    pub objective_included_candidate: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafetyVectorReport {
    pub baseline_policy_id: String,
    pub candidate_policy_id: String,
    pub comparable: bool,
    pub withheld_reasons: Vec<String>,
    pub category_comparisons: Vec<CategoryComparison>,
    pub objective_coverage_gaps: Vec<String>,
    pub measurement_coverage_gaps: Vec<String>,
    pub proxy_shift_detected: bool,
    pub adverse_category_regressions: Vec<String>,
    pub unknown_event_increase: bool,
    pub baseline_unknown_event_count: u64,
    pub candidate_unknown_event_count: u64,
    pub complete_safety_improvement_claim: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SafetyVectorError {
    EmptyIdentifier,
    NoTaxonomyCategories,
    DuplicateTaxonomyCategory,
    NoPrimaryClaimCategories,
    DuplicatePrimaryClaimCategory,
    UnknownPrimaryClaimCategory,
    PrimaryClaimCategoryNotAdverse,
    DuplicateCoverageCategory,
    MissingCoverageCategory,
    UnknownCoverageCategory,
    CoverageExceedsExposure,
    DuplicateEventId,
    DuplicatePhysicalEventCategory,
    UnknownEventCategory,
    EventSeverityUnitMismatch,
    MissingPolicyCategory,
    DuplicatePolicyCategory,
    ArithmeticOverflow,
    ZeroExposure,
    TaxonomyIdentityMismatch,
}

#[derive(Debug, Clone)]
struct CategoryAggregate {
    event_count: u64,
    severity_sum: u128,
    maximum_severity: u64,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn direction_from_cross_products(
    candidate_numerator: u128,
    baseline_denominator: u128,
    baseline_numerator: u128,
    candidate_denominator: u128,
) -> Result<Direction, SafetyVectorError> {
    let candidate_product = candidate_numerator
        .checked_mul(baseline_denominator)
        .ok_or(SafetyVectorError::ArithmeticOverflow)?;
    let baseline_product = baseline_numerator
        .checked_mul(candidate_denominator)
        .ok_or(SafetyVectorError::ArithmeticOverflow)?;
    Ok(match candidate_product.cmp(&baseline_product) {
        std::cmp::Ordering::Less => Direction::Improved,
        std::cmp::Ordering::Equal => Direction::Equal,
        std::cmp::Ordering::Greater => Direction::Worsened,
    })
}

fn validate_taxonomy(
    taxonomy: &Taxonomy,
) -> Result<BTreeMap<&str, &TaxonomyCategory>, SafetyVectorError> {
    if !nonempty(&taxonomy.version) || !nonempty(&taxonomy.digest) {
        return Err(SafetyVectorError::EmptyIdentifier);
    }
    if taxonomy.categories.is_empty() {
        return Err(SafetyVectorError::NoTaxonomyCategories);
    }
    let mut categories = BTreeMap::new();
    for category in &taxonomy.categories {
        if !nonempty(&category.id) || !nonempty(&category.severity_unit_id) {
            return Err(SafetyVectorError::EmptyIdentifier);
        }
        if categories.insert(category.id.as_str(), category).is_some() {
            return Err(SafetyVectorError::DuplicateTaxonomyCategory);
        }
    }
    Ok(categories)
}

fn validate_policy_identity(policy: &PolicyResult) -> Result<(), SafetyVectorError> {
    let ids = [
        policy.policy_id.as_str(),
        policy.matched_stratum_digest.as_str(),
        policy.taxonomy_version.as_str(),
        policy.taxonomy_digest.as_str(),
        policy.detector_digest.as_str(),
        policy.exposure.unit_id.as_str(),
    ];
    if ids.iter().any(|value| !nonempty(value)) {
        return Err(SafetyVectorError::EmptyIdentifier);
    }
    if policy.exposure.value == 0 {
        return Err(SafetyVectorError::ZeroExposure);
    }
    Ok(())
}

fn validate_and_aggregate_policy(
    taxonomy_categories: &BTreeMap<&str, &TaxonomyCategory>,
    policy: &PolicyResult,
) -> Result<BTreeMap<String, CategoryAggregate>, SafetyVectorError> {
    validate_policy_identity(policy)?;
    if policy.taxonomy_version != taxonomy.version || policy.taxonomy_digest != taxonomy.digest {
        return Err(SafetyVectorError::TaxonomyIdentityMismatch);
    }

    let mut coverage = BTreeMap::new();
    for item in &policy.category_coverage {
        if !nonempty(&item.category_id) {
            return Err(SafetyVectorError::EmptyIdentifier);
        }
        if !taxonomy_categories.contains_key(item.category_id.as_str()) {
            return Err(SafetyVectorError::UnknownCoverageCategory);
        }
        if item.observed_exposure > policy.exposure.value {
            return Err(SafetyVectorError::CoverageExceedsExposure);
        }
        if coverage.insert(item.category_id.as_str(), item).is_some() {
            return Err(SafetyVectorError::DuplicateCoverageCategory);
        }
    }
    if coverage.len() != taxonomy.categories.len() {
        return Err(SafetyVectorError::MissingCoverageCategory);
    }
    for category in &taxonomy.categories {
        if !coverage.contains_key(category.id.as_str()) {
            return Err(SafetyVectorError::MissingCoverageCategory);
        }
    }

    let mut event_ids = BTreeSet::new();
    let mut incident_categories = BTreeSet::new();
    let mut aggregates = BTreeMap::new();
    for category in &taxonomy.categories {
        aggregates.insert(
            category.id.clone(),
            CategoryAggregate {
                event_count: 0,
                severity_sum: 0,
                maximum_severity: 0,
            },
        );
    }

    for event in &policy.events {
        if !nonempty(&event.event_id)
            || !nonempty(&event.physical_event_id)
            || !nonempty(&event.category_id)
            || !nonempty(&event.severity_unit_id)
        {
            return Err(SafetyVectorError::EmptyIdentifier);
        }
        if !event_ids.insert(event.event_id.as_str()) {
            return Err(SafetyVectorError::DuplicateEventId);
        }
        if !incident_categories
            .insert((event.physical_event_id.as_str(), event.category_id.as_str()))
        {
            return Err(SafetyVectorError::DuplicatePhysicalEventCategory);
        }
        let category = taxonomy_categories
            .get(event.category_id.as_str())
            .ok_or(SafetyVectorError::UnknownEventCategory)?;
        if event.severity_unit_id != category.severity_unit_id {
            return Err(SafetyVectorError::EventSeverityUnitMismatch);
        }

        let aggregate = aggregates
            .get_mut(&event.category_id)
            .ok_or(SafetyVectorError::UnknownEventCategory)?;
        aggregate.event_count = aggregate
            .event_count
            .checked_add(1)
            .ok_or(SafetyVectorError::ArithmeticOverflow)?;
        aggregate.severity_sum = aggregate
            .severity_sum
            .checked_add(u128::from(event.severity))
            .ok_or(SafetyVectorError::ArithmeticOverflow)?;
        aggregate.maximum_severity = aggregate.maximum_severity.max(event.severity);
    }

    Ok(aggregates)
}

fn policy_comparability_reasons(
    taxonomy: &Taxonomy,
    baseline: &PolicyResult,
    candidate: &PolicyResult,
) -> Vec<String> {
    let mut reasons = Vec::new();
    if baseline.matched_stratum_digest != candidate.matched_stratum_digest {
        reasons.push("matched stratum digests differ".to_owned());
    }
    if baseline.taxonomy_version != candidate.taxonomy_version
        || baseline.taxonomy_digest != candidate.taxonomy_digest
        || baseline.taxonomy_version != taxonomy.version
        || baseline.taxonomy_digest != taxonomy.digest
    {
        reasons.push("taxonomy versions or digests differ".to_owned());
    }
    if baseline.detector_digest != candidate.detector_digest {
        reasons.push("event detector/classifier digests differ".to_owned());
    }
    if baseline.exposure.unit_id != candidate.exposure.unit_id {
        reasons.push("exposure units differ".to_owned());
    }
    if baseline.exposure.value != candidate.exposure.value {
        reasons.push("exposure values are not matched".to_owned());
    }
    reasons
}

fn rate_direction(
    candidate_count: u64,
    candidate_exposure: u64,
    baseline_count: u64,
    baseline_exposure: u64,
    eligible: bool,
) -> Result<Direction, SafetyVectorError> {
    if !eligible {
        return Ok(Direction::Withheld);
    }
    direction_from_cross_products(
        u128::from(candidate_count),
        u128::from(baseline_exposure),
        u128::from(baseline_count),
        u128::from(candidate_exposure),
    )
}

fn severity_direction(
    candidate_sum: u128,
    candidate_exposure: u64,
    baseline_sum: u128,
    baseline_exposure: u64,
    eligible: bool,
) -> Result<Direction, SafetyVectorError> {
    if !eligible {
        return Ok(Direction::Withheld);
    }
    direction_from_cross_products(
        candidate_sum,
        u128::from(baseline_exposure),
        baseline_sum,
        u128::from(candidate_exposure),
    )
}

pub fn audit_safety_vector(
    problem: &SafetyVectorProblem,
) -> Result<SafetyVectorReport, SafetyVectorError> {
    let taxonomy_categories = validate_taxonomy(&problem.taxonomy)?;
    let baseline_aggregates =
        validate_and_aggregate_policy(&taxonomy_categories, &problem.baseline)?;
    let candidate_aggregates =
        validate_and_aggregate_policy(&taxonomy_categories, &problem.candidate)?;

    if problem.primary_claim_category_ids.is_empty() {
        return Err(SafetyVectorError::NoPrimaryClaimCategories);
    }
    let mut primary_ids = BTreeSet::new();
    for id in &problem.primary_claim_category_ids {
        if !primary_ids.insert(id.as_str()) {
            return Err(SafetyVectorError::DuplicatePrimaryClaimCategory);
        }
        let category = taxonomy_categories
            .get(id.as_str())
            .ok_or(SafetyVectorError::UnknownPrimaryClaimCategory)?;
        if !category.adverse {
            return Err(SafetyVectorError::PrimaryClaimCategoryNotAdverse);
        }
    }

    let withheld_reasons =
        policy_comparability_reasons(&problem.taxonomy, &problem.baseline, &problem.candidate);
    let comparable = withheld_reasons.is_empty();

    let baseline_coverage: BTreeMap<&str, &CategoryCoverage> = problem
        .baseline
        .category_coverage
        .iter()
        .map(|item| (item.category_id.as_str(), item))
        .collect();
    let candidate_coverage: BTreeMap<&str, &CategoryCoverage> = problem
        .candidate
        .category_coverage
        .iter()
        .map(|item| (item.category_id.as_str(), item))
        .collect();

    let mut category_comparisons = Vec::with_capacity(problem.taxonomy.categories.len());
    let mut objective_coverage_gaps = Vec::new();
    let mut measurement_coverage_gaps = Vec::new();
    let mut adverse_category_regressions = Vec::new();
    let mut target_improved = false;
    let mut non_target_adverse_worsened = false;
    let mut complete_required_coverage = true;

    for category in &problem.taxonomy.categories {
        let baseline_coverage = baseline_coverage
            .get(category.id.as_str())
            .ok_or(SafetyVectorError::MissingCoverageCategory)?;
        let candidate_coverage = candidate_coverage
            .get(category.id.as_str())
            .ok_or(SafetyVectorError::MissingCoverageCategory)?;
        let measurement_complete = baseline_coverage.observed_exposure
            == problem.baseline.exposure.value
            && candidate_coverage.observed_exposure == problem.candidate.exposure.value;

        if category.adverse && !measurement_complete {
            complete_required_coverage = false;
            measurement_coverage_gaps.push(category.id.clone());
        }
        if category.adverse
            && (!baseline_coverage.objective_included || !candidate_coverage.objective_included)
        {
            objective_coverage_gaps.push(category.id.clone());
        }

        let baseline = baseline_aggregates
            .get(&category.id)
            .ok_or(SafetyVectorError::MissingPolicyCategory)?;
        let candidate = candidate_aggregates
            .get(&category.id)
            .ok_or(SafetyVectorError::MissingPolicyCategory)?;
        let comparison_eligible = comparable && measurement_complete;
        let event_rate_direction = rate_direction(
            candidate.event_count,
            problem.candidate.exposure.value,
            baseline.event_count,
            problem.baseline.exposure.value,
            comparison_eligible,
        )?;
        let severity_load_direction = severity_direction(
            candidate.severity_sum,
            problem.candidate.exposure.value,
            baseline.severity_sum,
            problem.baseline.exposure.value,
            comparison_eligible,
        )?;
        let maximum_severity_direction = if !comparison_eligible {
            Direction::Withheld
        } else {
            match candidate.maximum_severity.cmp(&baseline.maximum_severity) {
                std::cmp::Ordering::Less => Direction::Improved,
                std::cmp::Ordering::Equal => Direction::Equal,
                std::cmp::Ordering::Greater => Direction::Worsened,
            }
        };

        if category.adverse {
            if primary_ids.contains(category.id.as_str())
                && event_rate_direction == Direction::Improved
            {
                target_improved = true;
            }
            if !primary_ids.contains(category.id.as_str())
                && (event_rate_direction == Direction::Worsened
                    || severity_load_direction == Direction::Worsened
                    || maximum_severity_direction == Direction::Worsened)
            {
                non_target_adverse_worsened = true;
            }
            if event_rate_direction == Direction::Worsened
                || severity_load_direction == Direction::Worsened
                || maximum_severity_direction == Direction::Worsened
            {
                adverse_category_regressions.push(category.id.clone());
            }
        }

        category_comparisons.push(CategoryComparison {
            category_id: category.id.clone(),
            adverse: category.adverse,
            baseline_event_count: baseline.event_count,
            candidate_event_count: candidate.event_count,
            event_rate_direction,
            baseline_severity_sum: baseline.severity_sum,
            candidate_severity_sum: candidate.severity_sum,
            severity_load_direction,
            baseline_max_severity: baseline.maximum_severity,
            candidate_max_severity: candidate.maximum_severity,
            maximum_severity_direction,
            measurement_complete,
            objective_included_baseline: baseline_coverage.objective_included,
            objective_included_candidate: candidate_coverage.objective_included,
        });
    }

    objective_coverage_gaps.sort();
    measurement_coverage_gaps.sort();
    adverse_category_regressions.sort();

    let unknown_event_increase =
        problem.candidate.unknown_event_count > problem.baseline.unknown_event_count;
    let complete_safety_improvement_claim = comparable
        && complete_required_coverage
        && objective_coverage_gaps.is_empty()
        && measurement_coverage_gaps.is_empty()
        && problem.baseline.unknown_event_count == 0
        && problem.candidate.unknown_event_count == 0
        && adverse_category_regressions.is_empty()
        && target_improved;

    Ok(SafetyVectorReport {
        baseline_policy_id: problem.baseline.policy_id.clone(),
        candidate_policy_id: problem.candidate.policy_id.clone(),
        comparable,
        withheld_reasons,
        category_comparisons,
        objective_coverage_gaps,
        measurement_coverage_gaps,
        proxy_shift_detected: target_improved && non_target_adverse_worsened,
        adverse_category_regressions,
        unknown_event_increase,
        baseline_unknown_event_count: problem.baseline.unknown_event_count,
        candidate_unknown_event_count: problem.candidate.unknown_event_count,
        complete_safety_improvement_claim,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn taxonomy() -> Taxonomy {
        Taxonomy {
            version: "safety-v1".to_owned(),
            digest: "taxonomy-digest-v1".to_owned(),
            categories: vec![
                TaxonomyCategory {
                    id: "fall".to_owned(),
                    adverse: true,
                    required: true,
                    severity_unit_id: "severity_milli".to_owned(),
                },
                TaxonomyCategory {
                    id: "foot_scuff".to_owned(),
                    adverse: true,
                    required: true,
                    severity_unit_id: "severity_milli".to_owned(),
                },
                TaxonomyCategory {
                    id: "shank_collision".to_owned(),
                    adverse: true,
                    required: true,
                    severity_unit_id: "severity_milli".to_owned(),
                },
            ],
        }
    }

    fn event(id: &str, incident: &str, category: &str, severity: u64) -> EventRecord {
        EventRecord {
            event_id: id.to_owned(),
            physical_event_id: incident.to_owned(),
            category_id: category.to_owned(),
            time_ns: 100,
            severity,
            severity_unit_id: "severity_milli".to_owned(),
        }
    }

    fn policy(policy_id: &str, exposure: u64, events: Vec<EventRecord>) -> PolicyResult {
        PolicyResult {
            policy_id: policy_id.to_owned(),
            matched_stratum_digest: "stairs-stratum-v1".to_owned(),
            taxonomy_version: "safety-v1".to_owned(),
            taxonomy_digest: "taxonomy-digest-v1".to_owned(),
            detector_digest: "detector-v1".to_owned(),
            exposure: Exposure {
                value: exposure,
                unit_id: "crossings".to_owned(),
            },
            category_coverage: vec![
                CategoryCoverage {
                    category_id: "fall".to_owned(),
                    observed_exposure: exposure,
                    objective_included: true,
                },
                CategoryCoverage {
                    category_id: "foot_scuff".to_owned(),
                    observed_exposure: exposure,
                    objective_included: true,
                },
                CategoryCoverage {
                    category_id: "shank_collision".to_owned(),
                    observed_exposure: exposure,
                    objective_included: true,
                },
            ],
            events,
            unknown_event_count: 0,
        }
    }

    fn problem(
        baseline_events: Vec<EventRecord>,
        candidate_events: Vec<EventRecord>,
    ) -> SafetyVectorProblem {
        SafetyVectorProblem {
            taxonomy: taxonomy(),
            baseline: policy("learning-progress", 300, baseline_events),
            candidate: policy("psc", 300, candidate_events),
            primary_claim_category_ids: vec!["shank_collision".to_owned()],
        }
    }

    fn comparison<'a>(report: &'a SafetyVectorReport, id: &str) -> &'a CategoryComparison {
        report
            .category_comparisons
            .iter()
            .find(|item| item.category_id == id)
            .expect("category comparison")
    }

    #[test]
    fn target_category_gain_with_foot_scuff_increase_flags_proxy_shift() {
        let input = problem(
            vec![
                event("b-s1", "b-i1", "shank_collision", 300),
                event("b-s2", "b-i2", "shank_collision", 300),
                event("b-f1", "b-i3", "foot_scuff", 100),
            ],
            vec![
                event("c-s1", "c-i1", "shank_collision", 300),
                event("c-f1", "c-i2", "foot_scuff", 100),
                event("c-f2", "c-i3", "foot_scuff", 100),
            ],
        );
        let report = audit_safety_vector(&input).expect("valid proxy-shift case");
        assert_eq!(
            comparison(&report, "shank_collision").event_rate_direction,
            Direction::Improved
        );
        assert_eq!(
            comparison(&report, "foot_scuff").event_rate_direction,
            Direction::Worsened
        );
        assert!(report.proxy_shift_detected);
        assert!(!report.complete_safety_improvement_claim);
    }

    #[test]
    fn all_required_adverse_categories_nonregressing_and_target_improves() {
        let input = problem(
            vec![
                event("b-s1", "b-i1", "shank_collision", 300),
                event("b-f1", "b-i2", "foot_scuff", 100),
                event("b-l1", "b-i3", "fall", 900),
            ],
            vec![
                event("c-s1", "c-i1", "shank_collision", 250),
                event("c-f1", "c-i2", "foot_scuff", 90),
            ],
        );
        let report = audit_safety_vector(&input).expect("valid improving vector");
        assert!(report.complete_safety_improvement_claim);
        assert!(!report.proxy_shift_detected);
    }

    #[test]
    fn fall_regression_is_not_hidden_by_shank_improvement() {
        let input = problem(
            vec![
                event("b-s1", "b-i1", "shank_collision", 300),
                event("b-s2", "b-i2", "shank_collision", 300),
                event("b-f1", "b-i3", "fall", 900),
            ],
            vec![
                event("c-s1", "c-i1", "shank_collision", 300),
                event("c-f1", "c-i2", "fall", 900),
                event("c-f2", "c-i3", "fall", 900),
            ],
        );
        let report = audit_safety_vector(&input).expect("valid fall-regression case");
        assert_eq!(
            comparison(&report, "shank_collision").event_rate_direction,
            Direction::Improved
        );
        assert_eq!(
            comparison(&report, "fall").event_rate_direction,
            Direction::Worsened
        );
        assert!(report.proxy_shift_detected);
        assert!(!report.complete_safety_improvement_claim);
    }

    #[test]
    fn reduced_event_rate_does_not_hide_worse_severity_load() {
        let input = problem(
            vec![
                event("b-s1", "b-i1", "shank_collision", 100),
                event("b-s2", "b-i2", "shank_collision", 100),
                event("b-s3", "b-i3", "shank_collision", 100),
            ],
            vec![
                event("c-s1", "c-i1", "shank_collision", 900),
                event("c-s2", "c-i2", "shank_collision", 900),
            ],
        );
        let report = audit_safety_vector(&input).expect("valid severity case");
        let shank = comparison(&report, "shank_collision");
        assert_eq!(shank.event_rate_direction, Direction::Improved);
        assert_eq!(shank.severity_load_direction, Direction::Worsened);
        assert_eq!(shank.maximum_severity_direction, Direction::Worsened);
        assert!(!report.complete_safety_improvement_claim);
    }

    #[test]
    fn missing_category_measurement_withholds_complete_claim() {
        let mut input = problem(Vec::new(), Vec::new());
        input.candidate.category_coverage[1].observed_exposure = 200;
        let report = audit_safety_vector(&input).expect("partial coverage report");
        assert_eq!(
            comparison(&report, "foot_scuff").event_rate_direction,
            Direction::Withheld
        );
        assert_eq!(
            report.measurement_coverage_gaps,
            vec!["foot_scuff".to_owned()]
        );
        assert!(!report.complete_safety_improvement_claim);
    }

    #[test]
    fn unknown_events_suppress_complete_claim() {
        let mut input = problem(Vec::new(), Vec::new());
        input.candidate.unknown_event_count = 1;
        let report = audit_safety_vector(&input).expect("unknown report");
        assert!(report.unknown_event_increase);
        assert!(!report.complete_safety_improvement_claim);
    }

    #[test]
    fn incompatible_exposure_unit_withholds_rates() {
        let mut input = problem(Vec::new(), Vec::new());
        input.candidate.exposure.unit_id = "robot_hours".to_owned();
        let report = audit_safety_vector(&input).expect("incomparable report");
        assert!(!report.comparable);
        assert!(
            report
                .withheld_reasons
                .iter()
                .any(|reason| reason.contains("exposure units"))
        );
        assert_eq!(
            comparison(&report, "shank_collision").event_rate_direction,
            Direction::Withheld
        );
        assert!(!report.complete_safety_improvement_claim);
    }

    #[test]
    fn duplicate_sensor_records_for_same_physical_event_category_are_rejected() {
        let input = problem(
            Vec::new(),
            vec![
                event("channel-a", "incident-1", "shank_collision", 300),
                event("channel-b", "incident-1", "shank_collision", 300),
            ],
        );
        assert_eq!(
            audit_safety_vector(&input),
            Err(SafetyVectorError::DuplicatePhysicalEventCategory)
        );
    }

    #[test]
    fn one_incident_may_emit_multiple_distinct_typed_outcomes() {
        let mut input = problem(
            Vec::new(),
            vec![
                event("shank-event", "incident-1", "shank_collision", 300),
                event("scuff-event", "incident-1", "foot_scuff", 100),
            ],
        );
        input.primary_claim_category_ids = vec!["shank_collision".to_owned()];
        let report = audit_safety_vector(&input).expect("multi-outcome incident");
        assert_eq!(
            comparison(&report, "shank_collision").candidate_event_count,
            1
        );
        assert_eq!(comparison(&report, "foot_scuff").candidate_event_count, 1);
    }

    #[test]
    fn detector_or_taxonomy_mismatch_withholds_cross_policy_comparison() {
        let mut input = problem(Vec::new(), Vec::new());
        input.candidate.detector_digest = "detector-v2".to_owned();
        let report = audit_safety_vector(&input).expect("mismatched detector");
        assert!(!report.comparable);
        assert!(!report.complete_safety_improvement_claim);
    }

    #[test]
    fn objective_omission_for_adverse_category_suppresses_complete_claim() {
        let mut input = problem(Vec::new(), Vec::new());
        input.candidate.category_coverage[1].objective_included = false;
        let report = audit_safety_vector(&input).expect("objective gap report");
        assert_eq!(
            report.objective_coverage_gaps,
            vec!["foot_scuff".to_owned()]
        );
        assert!(!report.complete_safety_improvement_claim);
    }

    #[test]
    fn taxonomy_digest_mismatch_is_rejected() {
        let mut input = problem(Vec::new(), Vec::new());
        input.candidate.taxonomy_digest = "taxonomy-v2".to_owned();
        assert_eq!(
            audit_safety_vector(&input),
            Err(SafetyVectorError::TaxonomyIdentityMismatch)
        );
    }

    #[test]
    fn zero_exposure_is_rejected_not_reported_as_zero_rate() {
        let mut input = problem(Vec::new(), Vec::new());
        input.candidate.exposure.value = 0;
        assert_eq!(
            audit_safety_vector(&input),
            Err(SafetyVectorError::ZeroExposure)
        );
    }

    #[test]
    fn event_outside_taxonomy_is_rejected() {
        let input = problem(
            Vec::new(),
            vec![EventRecord {
                event_id: "unknown".to_owned(),
                physical_event_id: "incident".to_owned(),
                category_id: "unlisted-contact".to_owned(),
                time_ns: 1,
                severity: 1,
                severity_unit_id: "severity_milli".to_owned(),
            }],
        );
        assert_eq!(
            audit_safety_vector(&input),
            Err(SafetyVectorError::UnknownEventCategory)
        );
    }
}
