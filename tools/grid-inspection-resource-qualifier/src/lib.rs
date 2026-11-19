use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const TOLERANCE: f64 = 1.0e-9;
const TOTAL_BASIS_POINTS: u32 = 10_000;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Case {
    pub schema_version: String,
    pub case_id: String,
    pub case_maturity: String,
    pub as_of_hour: f64,
    pub resource_manifest_ref: String,
    pub resource_manifest_sha256: String,
    pub policy_certificate_sha256: String,
    pub resource_manifest: ResourceManifest,
    pub inspection_catalog: Vec<Activity>,
    pub recovery_action_catalog: Vec<Activity>,
    pub policy_certificate: PolicyCertificate,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResourceManifest {
    pub schema_version: String,
    pub manifest_id: String,
    pub as_of_hour: f64,
    pub horizon_end_hour: f64,
    pub resources: Vec<Resource>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    pub resource_ref: String,
    pub status: ResourceStatus,
    pub valid_until_hour: f64,
    pub evidence_ref: String,
    pub capacity_windows: Vec<CapacityWindow>,
    pub reservations: Vec<Reservation>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResourceStatus {
    Available,
    Degraded,
    Unavailable,
    Unknown,
    ProposedOnly,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CapacityWindow {
    pub start_hour: f64,
    pub end_hour: f64,
    pub capacity_units: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Reservation {
    pub reservation_ref: String,
    pub start_hour: f64,
    pub end_hour: f64,
    pub units: u32,
    pub state: ReservationState,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReservationState {
    Confirmed,
    Unknown,
    ProposedOnly,
    Revoked,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    pub activity_id: String,
    pub activity_type: ActivityType,
    pub earliest_start_hour: f64,
    pub duration_hours: f64,
    pub result_latency_hours: f64,
    pub evidence_origin_ref: String,
    pub precondition_status: PreconditionStatus,
    pub applicability_ref: String,
    pub authority_requirement_ref: String,
    pub resource_requirements: Vec<ResourceRequirement>,
    pub service_model_ref: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActivityType {
    Inspection,
    RecoveryAction,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PreconditionStatus {
    Current,
    Unknown,
    Stale,
    Conflicted,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResourceRequirement {
    pub resource_ref: String,
    pub units: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PolicyCertificate {
    pub source_policy_ref: String,
    pub source_model_ref: String,
    pub source_assumptions_ref: String,
    pub first_inspection_id: String,
    pub first_outcome_branches: Vec<FirstOutcomeBranch>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FirstOutcomeBranch {
    pub outcome: ObservationOutcome,
    pub probability_basis_points: u32,
    pub second_inspection_id: Option<String>,
    pub recovery_action_id: Option<String>,
    pub service_trajectory_ref: Option<String>,
    pub second_outcome_branches: Option<Vec<SecondOutcomeBranch>>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SecondOutcomeBranch {
    pub outcome: ObservationOutcome,
    pub probability_basis_points: u32,
    pub recovery_action_id: String,
    pub service_trajectory_ref: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ObservationOutcome {
    Negative,
    Positive,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ActivitySchedule {
    pub activity_id: String,
    pub start_hour: f64,
    pub end_hour: f64,
    pub result_available_hour: f64,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BranchSchedule {
    pub first_outcome: ObservationOutcome,
    pub second_outcome: Option<ObservationOutcome>,
    pub probability_basis_points: u32,
    pub second_inspection: Option<ActivitySchedule>,
    pub recovery_action: ActivitySchedule,
    pub branch_resource_hours: Vec<ResourceHours>,
    pub service_model_ref: String,
    pub service_trajectory_ref: String,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResourceHours {
    pub resource_ref: String,
    pub unit_hours: f64,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExpectedResourceHours {
    pub resource_ref: String,
    pub expected_unit_hours: f64,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct QualificationReport {
    pub case_id: String,
    pub case_maturity: String,
    pub result_status: String,
    pub source_policy_ref: String,
    pub resource_manifest_ref: String,
    pub resource_manifest_sha256: String,
    pub policy_certificate_sha256: String,
    pub first_inspection: ActivitySchedule,
    pub branches: Vec<BranchSchedule>,
    pub expected_candidate_action_completion_hour: f64,
    pub worst_case_candidate_action_completion_hour: f64,
    pub expected_resource_hours: Vec<ExpectedResourceHours>,
    pub claim_boundary: String,
}

pub fn canonical_manifest_sha256(manifest: &ResourceManifest) -> Result<String, serde_json::Error> {
    // serde_json::Value uses sorted object keys; array order remains significant.
    let canonical_value = serde_json::to_value(manifest)?;
    let bytes = serde_json::to_vec(&canonical_value)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

pub fn canonical_policy_certificate_sha256(policy: &PolicyCertificate) -> Result<String, serde_json::Error> {
    let canonical_value = serde_json::to_value(policy)?;
    let bytes = serde_json::to_vec(&canonical_value)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn finite_nonnegative(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

fn approximately_equal(left: f64, right: f64) -> bool {
    (left - right).abs() <= TOLERANCE
}

fn add_activity_errors(activity: &Activity, expected_type: ActivityType, errors: &mut Vec<String>) {
    if activity.activity_id.trim().is_empty() || activity.activity_type != expected_type {
        errors.push(format!("activity {} has an invalid ID or activity type", activity.activity_id));
    }
    if !finite_nonnegative(activity.earliest_start_hour)
        || !activity.duration_hours.is_finite()
        || activity.duration_hours <= 0.0
        || !finite_nonnegative(activity.result_latency_hours)
    {
        errors.push(format!("activity {} has invalid timing fields", activity.activity_id));
    }
    if activity.evidence_origin_ref.trim().is_empty()
        || activity.applicability_ref.trim().is_empty()
        || activity.authority_requirement_ref.trim().is_empty()
    {
        errors.push(format!("activity {} is missing evidence, applicability or authority references", activity.activity_id));
    }
    if activity.precondition_status != PreconditionStatus::Current {
        errors.push(format!("activity {} has unknown, stale or conflicted prerequisites", activity.activity_id));
    }
    if expected_type == ActivityType::RecoveryAction
        && activity.result_latency_hours > TOLERANCE
    {
        errors.push(format!("recovery action {} cannot use inspection result latency", activity.activity_id));
    }
    let mut seen = BTreeSet::new();
    for requirement in &activity.resource_requirements {
        if requirement.resource_ref.trim().is_empty()
            || requirement.units == 0
            || !seen.insert(requirement.resource_ref.as_str())
        {
            errors.push(format!("activity {} has an empty, zero or duplicate resource requirement", activity.activity_id));
        }
    }
}

fn validate_manifest(case: &Case, errors: &mut Vec<String>) {
    let manifest = &case.resource_manifest;
    if manifest.schema_version != "0.1.0"
        || manifest.manifest_id.trim().is_empty()
        || !finite_nonnegative(manifest.as_of_hour)
        || !manifest.horizon_end_hour.is_finite()
        || manifest.horizon_end_hour <= manifest.as_of_hour
        || !approximately_equal(manifest.as_of_hour, case.as_of_hour)
    {
        errors.push("resource manifest identity, horizon or as-of time is invalid".to_string());
    }
    if case.resource_manifest_ref.trim().is_empty()
        || case.resource_manifest_sha256.len() != 64
        || !case.resource_manifest_sha256.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        errors.push("resource manifest reference or lowercase SHA-256 digest is invalid".to_string());
    } else {
        match canonical_manifest_sha256(manifest) {
            Ok(actual) if actual == case.resource_manifest_sha256 => {}
            Ok(_) => errors.push("resource manifest SHA-256 does not match canonical manifest content".to_string()),
            Err(error) => errors.push(format!("resource manifest serialization failed: {error}")),
        }
    }
    if case.policy_certificate_sha256.len() != 64
        || !case.policy_certificate_sha256.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        errors.push("policy certificate SHA-256 must be 64 lowercase hexadecimal characters".to_string());
    } else {
        match canonical_policy_certificate_sha256(&case.policy_certificate) {
            Ok(actual) if actual == case.policy_certificate_sha256 => {}
            Ok(_) => errors.push("policy certificate SHA-256 does not match canonical policy content".to_string()),
            Err(error) => errors.push(format!("policy certificate serialization failed: {error}")),
        }
    }

    let mut resource_ids = BTreeSet::new();
    for resource in &manifest.resources {
        if resource.resource_ref.trim().is_empty()
            || !resource_ids.insert(resource.resource_ref.as_str())
            || !finite_nonnegative(resource.valid_until_hour)
            || resource.valid_until_hour < case.as_of_hour
            || resource.evidence_ref.trim().is_empty()
        {
            errors.push(format!("resource {} has an invalid identity, freshness or evidence reference", resource.resource_ref));
        }
        let mut prior_end: Option<f64> = None;
        for window in &resource.capacity_windows {
            if !window.start_hour.is_finite()
                || !window.end_hour.is_finite()
                || window.start_hour < 0.0
                || window.end_hour <= window.start_hour
                || window.capacity_units == 0
                || prior_end.is_some_and(|end| window.start_hour < end - TOLERANCE)
            {
                errors.push(format!("resource {} has invalid or overlapping capacity windows", resource.resource_ref));
            }
            prior_end = Some(window.end_hour);
        }
        for reservation in &resource.reservations {
            if reservation.reservation_ref.trim().is_empty()
                || !reservation.start_hour.is_finite()
                || !reservation.end_hour.is_finite()
                || reservation.start_hour < 0.0
                || reservation.end_hour <= reservation.start_hour
                || reservation.units == 0
            {
                errors.push(format!("resource {} has an invalid reservation", resource.resource_ref));
            }
        }
    }
}

fn validate_case(case: &Case) -> Vec<String> {
    let mut errors = Vec::new();
    if case.schema_version != "0.1.0"
        || case.case_id.trim().is_empty()
        || !matches!(case.case_maturity.as_str(), "synthetic_fixture" | "historical_replay")
        || !finite_nonnegative(case.as_of_hour)
    {
        errors.push("case identity, schema version or as-of hour is invalid".to_string());
    }
    validate_manifest(case, &mut errors);

    let mut inspection_ids = BTreeSet::new();
    for activity in &case.inspection_catalog {
        add_activity_errors(activity, ActivityType::Inspection, &mut errors);
        if !inspection_ids.insert(activity.activity_id.as_str()) {
            errors.push(format!("duplicate inspection ID {}", activity.activity_id));
        }
    }
    let mut action_ids = BTreeSet::new();
    for activity in &case.recovery_action_catalog {
        add_activity_errors(activity, ActivityType::RecoveryAction, &mut errors);
        if !action_ids.insert(activity.activity_id.as_str()) {
            errors.push(format!("duplicate recovery-action ID {}", activity.activity_id));
        }
        if activity.service_model_ref.as_deref().is_none_or(str::is_empty) {
            errors.push(format!("recovery action {} lacks an independent service-model reference", activity.activity_id));
        }
    }

    let policy = &case.policy_certificate;
    if policy.source_policy_ref.trim().is_empty()
        || policy.source_model_ref.trim().is_empty()
        || policy.source_assumptions_ref.trim().is_empty()
    {
        errors.push("upstream policy/model/assumption references are required".to_string());
    }
    let first = case.inspection_catalog.iter().find(|item| item.activity_id.as_str() == policy.first_inspection_id.as_str());
    let Some(first) = first else {
        errors.push("policy references a missing first inspection".to_string());
        return errors;
    };
    if first.activity_type != ActivityType::Inspection {
        errors.push("policy first activity is not an inspection".to_string());
    }

    if policy.first_outcome_branches.len() != 2 {
        errors.push("a depth-two certificate must contain exactly positive and negative first-outcome branches".to_string());
    }
    let mut first_outcomes = BTreeSet::new();
    let first_weight_sum: u64 = policy.first_outcome_branches.iter().map(|branch| branch.probability_basis_points as u64).sum();
    if first_weight_sum != TOTAL_BASIS_POINTS as u64
        || policy.first_outcome_branches.iter().any(|branch| branch.probability_basis_points == 0 || branch.probability_basis_points > TOTAL_BASIS_POINTS)
    {
        errors.push("first-outcome branch weights must be positive and sum to 10,000 basis points".to_string());
    }

    for branch in &policy.first_outcome_branches {
        if !first_outcomes.insert(branch.outcome) {
            errors.push("duplicate first-outcome branch".to_string());
        }
        match (&branch.second_inspection_id, &branch.second_outcome_branches, &branch.recovery_action_id, &branch.service_trajectory_ref) {
            (Some(second_id), Some(second_branches), None, None) => {
                if second_id == &policy.first_inspection_id {
                    errors.push("a policy branch cannot repeat its first inspection".to_string());
                }
                if let Some(second) = case.inspection_catalog.iter().find(|item| item.activity_id.as_str() == second_id.as_str()) {
                    if second.evidence_origin_ref == first.evidence_origin_ref {
                        errors.push("depth-two inspection reuses the first inspection evidence origin".to_string());
                    }
                } else {
                    errors.push(format!("policy references missing second inspection {second_id}"));
                }
                if second_branches.len() != 2 {
                    errors.push("a second inspection must contain exactly positive and negative outcome branches".to_string());
                }
                let mut second_outcomes = BTreeSet::new();
                let second_weight_sum: u64 = second_branches.iter().map(|item| item.probability_basis_points as u64).sum();
                if second_weight_sum != TOTAL_BASIS_POINTS as u64
                    || second_branches.iter().any(|item| item.probability_basis_points == 0 || item.probability_basis_points > TOTAL_BASIS_POINTS)
                {
                    errors.push("conditional second-outcome branch weights must be positive and sum to 10,000".to_string());
                }
                for leaf in second_branches {
                    if !second_outcomes.insert(leaf.outcome) {
                        errors.push("duplicate second-outcome branch".to_string());
                    }
                    if leaf.recovery_action_id.trim().is_empty() || leaf.service_trajectory_ref.trim().is_empty() {
                        errors.push("second-outcome leaf is missing a recovery action or service trajectory reference".to_string());
                    }
                    if case.recovery_action_catalog.iter().all(|action| action.activity_id != leaf.recovery_action_id) {
                        errors.push(format!("policy references missing recovery action {}", leaf.recovery_action_id));
                    }
                    let product = branch.probability_basis_points as u64 * leaf.probability_basis_points as u64;
                    if product % TOTAL_BASIS_POINTS as u64 != 0 {
                        errors.push("nested branch probability cannot be represented exactly in integer basis points".to_string());
                    }
                }
                if second_outcomes != BTreeSet::from([ObservationOutcome::Positive, ObservationOutcome::Negative]) {
                    errors.push("second-outcome branches must contain exactly positive and negative outcomes".to_string());
                }
            }
            (None, None, Some(action_id), Some(service_ref)) => {
                if action_id.trim().is_empty() || service_ref.trim().is_empty() {
                    errors.push("terminal first-outcome branch is missing its recovery action or service trajectory reference".to_string());
                }
                if case.recovery_action_catalog.iter().all(|action| action.activity_id != *action_id) {
                    errors.push(format!("policy references missing recovery action {action_id}"));
                }
            }
            _ => errors.push("first-outcome branch must either stop at one recovery action or include one second inspection and its two leaves".to_string()),
        }
    }
    if first_outcomes != BTreeSet::from([ObservationOutcome::Positive, ObservationOutcome::Negative]) {
        errors.push("first-outcome branches must contain exactly positive and negative outcomes".to_string());
    }
    errors
}

fn resource_by_ref<'a>(manifest: &'a ResourceManifest, reference: &str) -> Option<&'a Resource> {
    manifest.resources.iter().find(|resource| resource.resource_ref.as_str() == reference)
}

fn reservation_load_is_feasible(resource: &Resource, start: f64, end: f64, requested_units: u32, capacity: u32) -> bool {
    let mut boundaries = vec![start, end];
    for reservation in &resource.reservations {
        if reservation.end_hour <= start + TOLERANCE || reservation.start_hour >= end - TOLERANCE {
            continue;
        }
        if reservation.state == ReservationState::Unknown {
            return false;
        }
        boundaries.push(reservation.start_hour.max(start));
        boundaries.push(reservation.end_hour.min(end));
    }
    boundaries.sort_by(f64::total_cmp);
    boundaries.dedup_by(|left, right| approximately_equal(*left, *right));
    for pair in boundaries.windows(2) {
        if pair[1] <= pair[0] + TOLERANCE {
            continue;
        }
        let midpoint = (pair[0] + pair[1]) / 2.0;
        let committed_units: u64 = resource.reservations.iter()
            .filter(|reservation| reservation.state == ReservationState::Confirmed
                && reservation.start_hour <= midpoint
                && reservation.end_hour > midpoint)
            .map(|reservation| reservation.units as u64)
            .sum();
        if committed_units + requested_units as u64 > capacity as u64 {
            return false;
        }
    }
    true
}

fn resource_can_cover(resource: &Resource, start: f64, duration: f64, units: u32) -> bool {
    if !matches!(resource.status, ResourceStatus::Available | ResourceStatus::Degraded)
        || resource.valid_until_hour + TOLERANCE < start + duration
    {
        return false;
    }
    let end = start + duration;
    resource.capacity_windows.iter().any(|window| {
        window.start_hour <= start + TOLERANCE
            && window.end_hour + TOLERANCE >= end
            && window.capacity_units >= units
            && reservation_load_is_feasible(resource, start, end, units, window.capacity_units)
    })
}

fn earliest_common_slot(
    manifest: &ResourceManifest,
    requirements: &[ResourceRequirement],
    earliest_start: f64,
    duration: f64,
) -> Result<f64, String> {
    if requirements.is_empty() {
        let end = earliest_start + duration;
        return (end <= manifest.horizon_end_hour + TOLERANCE)
            .then_some(earliest_start)
            .ok_or_else(|| "activity exceeds the resource manifest horizon".to_string());
    }

    let mut candidates = vec![earliest_start];
    for requirement in requirements {
        let resource = resource_by_ref(manifest, &requirement.resource_ref)
            .ok_or_else(|| format!("missing resource {}", requirement.resource_ref))?;
        if !matches!(resource.status, ResourceStatus::Available | ResourceStatus::Degraded) {
            return Err(format!("resource {} is not allocatable ({:?})", resource.resource_ref, resource.status));
        }
        for window in &resource.capacity_windows {
            if window.start_hour >= earliest_start - TOLERANCE {
                candidates.push(window.start_hour.max(earliest_start));
            }
        }
        for reservation in &resource.reservations {
            if reservation.end_hour >= earliest_start - TOLERANCE {
                candidates.push(reservation.end_hour.max(earliest_start));
            }
        }
    }
    candidates.sort_by(f64::total_cmp);
    candidates.dedup_by(|left, right| approximately_equal(*left, *right));

    for candidate in candidates {
        if candidate + duration > manifest.horizon_end_hour + TOLERANCE {
            continue;
        }
        let all_available = requirements.iter().all(|requirement| {
            resource_by_ref(manifest, &requirement.resource_ref)
                .is_some_and(|resource| resource_can_cover(resource, candidate, duration, requirement.units))
        });
        if all_available {
            return Ok(candidate);
        }
    }
    Err("no common capacity interval satisfies all required resources".to_string())
}

fn schedule_activity(
    activity: &Activity,
    release_hour: f64,
    manifest: &ResourceManifest,
) -> Result<ActivitySchedule, String> {
    let earliest = release_hour.max(activity.earliest_start_hour);
    let start = earliest_common_slot(manifest, &activity.resource_requirements, earliest, activity.duration_hours)
        .map_err(|error| format!("activity {}: {error}", activity.activity_id))?;
    let end = start + activity.duration_hours;
    Ok(ActivitySchedule {
        activity_id: activity.activity_id.clone(),
        start_hour: start,
        end_hour: end,
        result_available_hour: end + activity.result_latency_hours,
    })
}

fn activity_by_id<'a>(activities: &'a [Activity], id: &str) -> Result<&'a Activity, String> {
    activities.iter().find(|activity| activity.activity_id == id)
        .ok_or_else(|| format!("missing activity {id}"))
}

fn add_expected_resource_hours(
    totals: &mut BTreeMap<String, f64>,
    activity: &Activity,
    probability_basis_points: u32,
) {
    let branch_weight = probability_basis_points as f64 / TOTAL_BASIS_POINTS as f64;
    for requirement in &activity.resource_requirements {
        *totals.entry(requirement.resource_ref.clone()).or_insert(0.0) +=
            activity.duration_hours * requirement.units as f64 * branch_weight;
    }
}

fn resource_hours_from_map(totals: BTreeMap<String, f64>) -> Vec<ResourceHours> {
    totals.into_iter()
        .map(|(resource_ref, unit_hours)| ResourceHours { resource_ref, unit_hours })
        .collect()
}

fn expected_resource_hours_from_map(totals: BTreeMap<String, f64>) -> Vec<ExpectedResourceHours> {
    totals.into_iter()
        .map(|(resource_ref, expected_unit_hours)| ExpectedResourceHours { resource_ref, expected_unit_hours })
        .collect()
}

pub fn qualify(case: &Case) -> Result<QualificationReport, Vec<String>> {
    let validation_errors = validate_case(case);
    if !validation_errors.is_empty() {
        return Err(validation_errors);
    }

    let policy = &case.policy_certificate;
    let first_activity = activity_by_id(&case.inspection_catalog, &policy.first_inspection_id)
        .map_err(|error| vec![error])?;
    let first_schedule = schedule_activity(first_activity, case.as_of_hour, &case.resource_manifest)
        .map_err(|error| vec![error])?;
    let mut expected_resource_hours = BTreeMap::new();
    add_expected_resource_hours(&mut expected_resource_hours, first_activity, TOTAL_BASIS_POINTS);
    let mut branches = Vec::new();
    let mut probability_sum = 0_u32;
    let mut expected_completion = 0.0_f64;
    let mut worst_completion = 0.0_f64;

    for first_branch in &policy.first_outcome_branches {
        if let (Some(second_id), Some(second_leaves)) = (&first_branch.second_inspection_id, &first_branch.second_outcome_branches) {
            let second_activity = activity_by_id(&case.inspection_catalog, second_id)
                .map_err(|error| vec![error])?;
            let second_schedule = schedule_activity(second_activity, first_schedule.result_available_hour, &case.resource_manifest)
                .map_err(|error| vec![error])?;
            add_expected_resource_hours(&mut expected_resource_hours, second_activity, first_branch.probability_basis_points);

            for second_leaf in second_leaves {
                let path_product = first_branch.probability_basis_points as u64 * second_leaf.probability_basis_points as u64;
                let path_weight = (path_product / TOTAL_BASIS_POINTS as u64) as u32;
                probability_sum += path_weight;
                let action = activity_by_id(&case.recovery_action_catalog, &second_leaf.recovery_action_id)
                    .map_err(|error| vec![error])?;
                let action_schedule = schedule_activity(action, second_schedule.result_available_hour, &case.resource_manifest)
                    .map_err(|error| vec![error])?;
                add_expected_resource_hours(&mut expected_resource_hours, action, path_weight);
                expected_completion += action_schedule.end_hour * path_weight as f64 / TOTAL_BASIS_POINTS as f64;
                worst_completion = worst_completion.max(action_schedule.end_hour);
                let mut branch_resource_totals = BTreeMap::new();
                add_expected_resource_hours(&mut branch_resource_totals, first_activity, TOTAL_BASIS_POINTS);
                add_expected_resource_hours(&mut branch_resource_totals, second_activity, TOTAL_BASIS_POINTS);
                add_expected_resource_hours(&mut branch_resource_totals, action, TOTAL_BASIS_POINTS);
                branches.push(BranchSchedule {
                    first_outcome: first_branch.outcome,
                    second_outcome: Some(second_leaf.outcome),
                    probability_basis_points: path_weight,
                    second_inspection: Some(second_schedule.clone()),
                    recovery_action: action_schedule,
                    branch_resource_hours: resource_hours_from_map(branch_resource_totals),
                    service_model_ref: action.service_model_ref.clone().unwrap_or_default(),
                    service_trajectory_ref: second_leaf.service_trajectory_ref.clone(),
                });
            }
        } else if let (Some(action_id), Some(service_ref)) = (&first_branch.recovery_action_id, &first_branch.service_trajectory_ref) {
            probability_sum += first_branch.probability_basis_points;
            let action = activity_by_id(&case.recovery_action_catalog, action_id)
                .map_err(|error| vec![error])?;
            let action_schedule = schedule_activity(action, first_schedule.result_available_hour, &case.resource_manifest)
                .map_err(|error| vec![error])?;
            add_expected_resource_hours(&mut expected_resource_hours, action, first_branch.probability_basis_points);
            expected_completion += action_schedule.end_hour * first_branch.probability_basis_points as f64 / TOTAL_BASIS_POINTS as f64;
            worst_completion = worst_completion.max(action_schedule.end_hour);
            let mut branch_resource_totals = BTreeMap::new();
            add_expected_resource_hours(&mut branch_resource_totals, first_activity, TOTAL_BASIS_POINTS);
            add_expected_resource_hours(&mut branch_resource_totals, action, TOTAL_BASIS_POINTS);
            branches.push(BranchSchedule {
                first_outcome: first_branch.outcome,
                second_outcome: None,
                probability_basis_points: first_branch.probability_basis_points,
                second_inspection: None,
                recovery_action: action_schedule,
                branch_resource_hours: resource_hours_from_map(branch_resource_totals),
                service_model_ref: action.service_model_ref.clone().unwrap_or_default(),
                service_trajectory_ref: service_ref.clone(),
            });
        }
    }

    if probability_sum != TOTAL_BASIS_POINTS {
        return Err(vec![format!("leaf path weights sum to {probability_sum}, not 10,000 basis points")]);
    }
    branches.sort_by(|left, right| {
        left.first_outcome.cmp(&right.first_outcome)
            .then_with(|| left.second_outcome.cmp(&right.second_outcome))
    });
    let expected_resource_hours = expected_resource_hours_from_map(expected_resource_hours);

    Ok(QualificationReport {
        case_id: case.case_id.clone(),
        case_maturity: case.case_maturity.clone(),
        result_status: "resource_time_certificate_consistent".to_string(),
        source_policy_ref: policy.source_policy_ref.clone(),
        resource_manifest_ref: case.resource_manifest_ref.clone(),
        resource_manifest_sha256: case.resource_manifest_sha256.clone(),
        policy_certificate_sha256: case.policy_certificate_sha256.clone(),
        first_inspection: first_schedule,
        branches,
        expected_candidate_action_completion_hour: expected_completion,
        worst_case_candidate_action_completion_hour: worst_completion,
        expected_resource_hours,
        claim_boundary: "Candidate action timing/resource feasibility only; not service restoration, dispatch authority or execution".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn refresh_policy_digest(case: &mut Case) {
        case.policy_certificate_sha256 =
            canonical_policy_certificate_sha256(&case.policy_certificate).expect("policy serializes");
    }

    fn fixture() -> Case {
        serde_json::from_str(include_str!("../fixtures/synthetic-grid-inspection-resource-case-v0.1.json"))
            .expect("fixture must deserialize")
    }

    #[test]
    fn baseline_policy_has_three_exclusive_leaf_paths_and_exact_weights() {
        let report = qualify(&fixture()).expect("fixture qualifies");
        assert_eq!(report.branches.len(), 3);
        assert_eq!(report.branches.iter().map(|branch| branch.probability_basis_points).sum::<u32>(), 10_000);
        assert_eq!(report.branches.iter().map(|branch| branch.probability_basis_points).collect::<Vec<_>>(), vec![1_400, 5_600, 3_000]);
    }

    #[test]
    fn inspection_resources_are_released_before_result_latency_ends() {
        let report = qualify(&fixture()).expect("fixture qualifies");
        assert_eq!(report.first_inspection.start_hour, 8.0);
        assert_eq!(report.first_inspection.end_hour, 10.0);
        assert_eq!(report.first_inspection.result_available_hour, 10.5);
        let nested = report.branches.iter().find(|branch| branch.second_outcome.is_some()).unwrap();
        assert_eq!(nested.second_inspection.as_ref().unwrap().start_hour, 10.5);
    }

    #[test]
    fn repair_waits_for_second_result_not_just_inspection_completion() {
        let report = qualify(&fixture()).expect("fixture qualifies");
        let repair = report.branches.iter().find(|branch| branch.recovery_action.activity_id == "repair-bushing").unwrap();
        assert_eq!(repair.second_inspection.as_ref().unwrap().end_hour, 12.0);
        assert_eq!(repair.second_inspection.as_ref().unwrap().result_available_hour, 12.25);
        assert_eq!(repair.recovery_action.start_hour, 12.25);
    }

    #[test]
    fn replacement_waits_for_later_crane_and_heavy_haul_window() {
        let report = qualify(&fixture()).expect("fixture qualifies");
        let replacement = report.branches.iter().find(|branch| branch.recovery_action.activity_id == "replace-transformer").unwrap();
        assert_eq!(replacement.recovery_action.start_hour, 12.0);
        assert_eq!(replacement.recovery_action.end_hour, 18.0);
    }

    #[test]
    fn confirmed_reservation_consumes_shared_crew_capacity() {
        let mut case = fixture();
        let crew = case.resource_manifest.resources.iter_mut()
            .find(|resource| resource.resource_ref == "synthetic://resource/specialist-crew-01").unwrap();
        crew.reservations.push(Reservation {
            reservation_ref: "confirmed-booking".to_string(),
            start_hour: 10.5,
            end_hour: 12.0,
            units: 1,
            state: ReservationState::Confirmed,
        });
        case.resource_manifest_sha256 = canonical_manifest_sha256(&case.resource_manifest).unwrap();
        let report = qualify(&case).expect("policy remains feasible after the reservation");
        let nested = report.branches.iter().find(|branch| branch.second_outcome.is_some()).unwrap();
        assert_eq!(nested.second_inspection.as_ref().unwrap().start_hour, 12.0);
        assert_eq!(nested.recovery_action.start_hour, 13.75);
    }

    #[test]
    fn branch_resource_hours_are_not_aggregated_across_alternative_worlds() {
        let report = qualify(&fixture()).expect("fixture qualifies");
        let repair = report.branches.iter().find(|branch| branch.recovery_action.activity_id == "repair-bushing").unwrap();
        let crew = repair.branch_resource_hours.iter()
            .find(|item| item.resource_ref == "synthetic://resource/specialist-crew-01").unwrap();
        assert!((crew.unit_hours - 9.5).abs() <= TOLERANCE);
        assert_eq!(repair.service_model_ref, "symtropy://6235/service-model/transformer-17/repair");
    }

    #[test]
    fn mutually_exclusive_branches_are_probability_weighted_not_summed() {
        let report = qualify(&fixture()).expect("fixture qualifies");
        let crew = report.expected_resource_hours.iter()
            .find(|item| item.resource_ref == "synthetic://resource/specialist-crew-01").unwrap();
        assert!((crew.expected_unit_hours - 9.05).abs() <= TOLERANCE);
        let test_set = report.expected_resource_hours.iter()
            .find(|item| item.resource_ref == "synthetic://resource/portable-test-set-01").unwrap();
        assert!((test_set.expected_unit_hours - 3.05).abs() <= TOLERANCE);
    }

    #[test]
    fn service_is_not_inferred_from_candidate_action_completion() {
        let report = qualify(&fixture()).expect("fixture qualifies");
        assert!(report.claim_boundary.contains("not service restoration"));
        assert!(report.branches.iter().all(|branch| branch.service_trajectory_ref.starts_with("symtropy://")));
    }

    #[test]
    fn manifest_digest_mismatch_is_rejected() {
        let mut case = fixture();
        case.resource_manifest_sha256.replace_range(0..1, "0");
        assert!(qualify(&case).is_err());
    }

    #[test]
    fn policy_certificate_payload_tampering_is_rejected() {
        let mut case = fixture();
        case.policy_certificate.first_outcome_branches[0].recovery_action_id =
            Some("repair-bushing".to_string());
        assert!(qualify(&case).is_err());
    }

    #[test]
    fn unknown_resource_status_is_not_promoted_to_available() {
        let mut case = fixture();
        case.resource_manifest.resources.iter_mut()
            .find(|resource| resource.resource_ref == "synthetic://resource/crane-01").unwrap().status = ResourceStatus::Unknown;
        case.resource_manifest_sha256 = canonical_manifest_sha256(&case.resource_manifest).unwrap();
        assert!(qualify(&case).is_err());
    }

    #[test]
    fn degraded_resource_is_usable_only_with_explicit_remaining_capacity() {
        let mut case = fixture();
        let crew = case.resource_manifest.resources.iter_mut()
            .find(|resource| resource.resource_ref == "synthetic://resource/specialist-crew-01").unwrap();
        crew.status = ResourceStatus::Degraded;
        crew.capacity_windows[0].capacity_units = 1;
        case.resource_manifest_sha256 = canonical_manifest_sha256(&case.resource_manifest).unwrap();
        assert!(qualify(&case).is_ok());
    }

    #[test]
    fn proposed_only_resource_is_not_treated_as_available() {
        let mut case = fixture();
        case.resource_manifest.resources.iter_mut()
            .find(|resource| resource.resource_ref == "synthetic://resource/crane-01").unwrap().status = ResourceStatus::ProposedOnly;
        case.resource_manifest_sha256 = canonical_manifest_sha256(&case.resource_manifest).unwrap();
        assert!(qualify(&case).is_err());
    }

    #[test]
    fn unknown_reservation_blocks_until_reconciled() {
        let mut case = fixture();
        let crew = case.resource_manifest.resources.iter_mut()
            .find(|resource| resource.resource_ref == "synthetic://resource/specialist-crew-01").unwrap();
        crew.reservations.push(Reservation {
            reservation_ref: "unknown-booking".to_string(),
            start_hour: 10.5,
            end_hour: 11.5,
            units: 1,
            state: ReservationState::Unknown,
        });
        case.resource_manifest_sha256 = canonical_manifest_sha256(&case.resource_manifest).unwrap();
        let report = qualify(&case).expect("policy remains feasible after the unknown hold ends");
        let nested = report.branches.iter().find(|branch| branch.second_outcome.is_some()).unwrap();
        assert_eq!(nested.second_inspection.as_ref().unwrap().start_hour, 11.5);
    }

    #[test]
    fn no_common_capacity_window_fails_closed() {
        let mut case = fixture();
        let test_set = case.resource_manifest.resources.iter_mut()
            .find(|resource| resource.resource_ref == "synthetic://resource/portable-test-set-01").unwrap();
        test_set.capacity_windows = vec![CapacityWindow { start_hour: 20.0, end_hour: 24.0, capacity_units: 1 }];
        case.resource_manifest_sha256 = canonical_manifest_sha256(&case.resource_manifest).unwrap();
        assert!(qualify(&case).is_err());
    }

    #[test]
    fn branch_weight_conservation_is_mandatory() {
        let mut case = fixture();
        case.policy_certificate.first_outcome_branches[0].probability_basis_points = 2_000;
        refresh_policy_digest(&mut case);
        assert!(qualify(&case).is_err());
    }

    #[test]
    fn second_outcome_branches_must_cover_positive_and_negative() {
        let mut case = fixture();
        let nested = case.policy_certificate.first_outcome_branches.iter_mut()
            .find(|branch| branch.second_outcome_branches.is_some()).unwrap();
        nested.second_outcome_branches.as_mut().unwrap()[1].outcome = ObservationOutcome::Positive;
        refresh_policy_digest(&mut case);
        assert!(qualify(&case).is_err());
    }

    #[test]
    fn correlated_second_inspection_origin_is_rejected() {
        let mut case = fixture();
        let first_origin = case.inspection_catalog.iter()
            .find(|activity| activity.activity_id == "inspect-independent-insulation-test").unwrap()
            .evidence_origin_ref.clone();
        case.inspection_catalog.iter_mut()
            .find(|activity| activity.activity_id == "inspect-bushing-resistance").unwrap()
            .evidence_origin_ref = first_origin;
        assert!(qualify(&case).is_err());
    }

    #[test]
    fn non_current_activity_preconditions_fail_closed() {
        let mut case = fixture();
        case.inspection_catalog.iter_mut()
            .find(|activity| activity.activity_id == "inspect-independent-insulation-test").unwrap()
            .precondition_status = PreconditionStatus::Unknown;
        assert!(qualify(&case).is_err());
    }

    #[test]
    fn duplicate_first_outcome_is_rejected() {
        let mut case = fixture();
        case.policy_certificate.first_outcome_branches[1].outcome = ObservationOutcome::Positive;
        refresh_policy_digest(&mut case);
        assert!(qualify(&case).is_err());
    }

    #[test]
    fn report_order_and_digest_are_deterministic() {
        let case = fixture();
        let first = qualify(&case).unwrap();
        let second = qualify(&case).unwrap();
        assert_eq!(serde_json::to_string(&first).unwrap(), serde_json::to_string(&second).unwrap());
    }
}
