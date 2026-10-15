// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Explicit period-quota accounting, separate from concurrent-use scheduling.
//!
//! A quota is a total quantity available during one half-open period [start, end).
//! Every proposed use must be allocated to exactly one source-backed quota. If an
//! operation crosses periods, callers must provide explicit consumption slices;
//! this module never prorates demand or converts a rate into concurrent slots.
//! It is deterministic planning logic, not a reservation or dispatch authority.

use std::collections::{BTreeMap, BTreeSet};

use super::{
    CargoTransferCapacityEvidenceStatus, CargoTransferCapacityResourceKind, QuantityUnit,
    ScheduledCapacityEvidenceV1,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeriodCapacityQuotaV1 {
    /// Exact source-backed capacity evidence. capacity_units is the total limit
    /// for this quota's period, not a concurrent-use limit or rate by itself.
    pub evidence: ScheduledCapacityEvidenceV1,
    pub period_start_epoch_seconds: u64,
    pub period_end_epoch_seconds: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeriodCapacityUseV1 {
    pub allocation_id: String,
    /// The exact quota/evidence budget ID. There is no best-match lookup.
    pub quota_id: String,
    pub capacity_units_used: u64,
    pub capacity_unit: QuantityUnit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeriodCapacityPolicyV1 {
    pub policy_id: String,
    pub expected_time_profile_id: String,
    pub max_observation_age_seconds: u64,
    pub allowed_future_clock_skew_seconds: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeriodCapacityReceiptV1 {
    pub quota_id: String,
    pub resource_kind: CargoTransferCapacityResourceKind,
    pub resource_id: String,
    pub terminal_id: Option<String>,
    pub capacity_scope_id: String,
    pub period_start_epoch_seconds: u64,
    pub period_end_epoch_seconds: u64,
    pub capacity_unit: QuantityUnit,
    pub capacity_units_used: u64,
    pub capacity_units_limit: u64,
    pub allocation_ids: Vec<String>,
    pub policy_id: String,
    pub time_profile_id: String,
    pub source_record_id: String,
    pub source_schema_profile_id: String,
    pub authority_id: String,
    pub authority_profile_id: String,
    pub source_record_digest: String,
    pub revision: u64,
    pub supersedes_revision: Option<u64>,
    pub source_event_time_epoch_seconds: Option<u64>,
    pub capacity_observed_at_epoch_seconds: u64,
    pub repository_recorded_at_epoch_seconds: u64,
    pub valid_from_epoch_seconds: u64,
    pub valid_until_epoch_seconds: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeriodCapacityReportV1 {
    pub evaluation_time_epoch_seconds: u64,
    pub policy_id: String,
    pub receipts: Vec<PeriodCapacityReceiptV1>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PeriodCapacityError {
    InvalidPolicy,
    InvalidIdentityOrProvenance,
    InvalidRevisionLineage,
    InvalidPeriod,
    PeriodAlreadyStarted,
    TimeProfileMismatch,
    MissingObservationTime,
    MissingRepositoryRecordTime,
    MissingValidityInterval,
    InvalidValidityInterval,
    PeriodNotCoveredByEvidence,
    TimestampInFuture,
    ObservationTooOld,
    ObservationTooOldAtPeriodStart,
    NotCurrent,
    DuplicateQuotaIdentity,
    AmbiguousQuotaEvidence,
    CapacityUnitMismatch,
    DuplicateAllocationIdentity,
    UnknownQuota,
    InvalidCapacityUse,
    CapacityExceeded,
    CapacityArithmeticOverflow,
    UnreferencedQuota,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn validate_period_quota(
    quota: &PeriodCapacityQuotaV1,
    evaluation_time_epoch_seconds: u64,
    policy: &PeriodCapacityPolicyV1,
) -> Result<(), PeriodCapacityError> {
    let evidence = &quota.evidence;
    if !nonempty(&evidence.budget_id)
        || !nonempty(&evidence.resource_id)
        || !nonempty(&evidence.capacity_scope_id)
        || !nonempty(&evidence.authority_id)
        || !nonempty(&evidence.authority_profile_id)
        || !nonempty(&evidence.source_record_id)
        || !nonempty(&evidence.source_schema_profile_id)
        || !nonempty(&evidence.source_record_digest)
        || !nonempty(&evidence.time_profile_id)
        || evidence
            .terminal_id
            .as_deref()
            .is_some_and(|id| !nonempty(id))
        || (evidence.resource_kind == CargoTransferCapacityResourceKind::Equipment
            && evidence.terminal_id.is_none())
        || evidence.capacity_units == 0
        || evidence.revision == 0
    {
        return Err(PeriodCapacityError::InvalidIdentityOrProvenance);
    }
    if evidence
        .supersedes_revision
        .is_some_and(|previous| previous >= evidence.revision)
    {
        return Err(PeriodCapacityError::InvalidRevisionLineage);
    }
    if evidence.status != CargoTransferCapacityEvidenceStatus::ReportedCurrent {
        return Err(PeriodCapacityError::NotCurrent);
    }
    if evidence.time_profile_id != policy.expected_time_profile_id {
        return Err(PeriodCapacityError::TimeProfileMismatch);
    }
    if quota.period_start_epoch_seconds >= quota.period_end_epoch_seconds {
        return Err(PeriodCapacityError::InvalidPeriod);
    }
    if quota.period_start_epoch_seconds < evaluation_time_epoch_seconds {
        return Err(PeriodCapacityError::PeriodAlreadyStarted);
    }

    let observed = evidence
        .capacity_observed_at_epoch_seconds
        .ok_or(PeriodCapacityError::MissingObservationTime)?;
    let recorded = evidence
        .repository_recorded_at_epoch_seconds
        .ok_or(PeriodCapacityError::MissingRepositoryRecordTime)?;
    let valid_from = evidence
        .valid_from_epoch_seconds
        .ok_or(PeriodCapacityError::MissingValidityInterval)?;
    let valid_until = evidence
        .valid_until_epoch_seconds
        .ok_or(PeriodCapacityError::MissingValidityInterval)?;
    if valid_from >= valid_until {
        return Err(PeriodCapacityError::InvalidValidityInterval);
    }
    if valid_from > quota.period_start_epoch_seconds || valid_until < quota.period_end_epoch_seconds
    {
        return Err(PeriodCapacityError::PeriodNotCoveredByEvidence);
    }

    let latest_accepted_time =
        evaluation_time_epoch_seconds.saturating_add(policy.allowed_future_clock_skew_seconds);
    if observed > latest_accepted_time || recorded > latest_accepted_time {
        return Err(PeriodCapacityError::TimestampInFuture);
    }
    if evaluation_time_epoch_seconds.saturating_sub(observed) > policy.max_observation_age_seconds {

        return Err(PeriodCapacityError::ObservationTooOld);
    }
    if observed
        > quota
            .period_start_epoch_seconds
            .saturating_add(policy.allowed_future_clock_skew_seconds)
    {
        return Err(PeriodCapacityError::TimestampInFuture);
    }
    if quota.period_start_epoch_seconds.saturating_sub(observed)
        > policy.max_observation_age_seconds
    {
        return Err(PeriodCapacityError::ObservationTooOldAtPeriodStart);
    }

    Ok(())
}

/// Qualify source-backed period quotas and account for all proposed uses.
///
/// Periods are half-open [start, end). Capacity means a total quantity allowed
/// over that whole period; it does not mean concurrent uses. Evidence must be
/// current, fresh at evaluation and at period start, and valid for the entire
/// period. Overlapping quota evidence for the same physical resource is rejected
/// even when scope labels differ, preventing a duplicate quota from multiplying
/// an apparent limit. Each supplied quota must be referenced by at least one use.
pub fn validate_period_capacity_plan(
    uses: &[PeriodCapacityUseV1],
    quotas: &[PeriodCapacityQuotaV1],
    evaluation_time_epoch_seconds: u64,
    policy: &PeriodCapacityPolicyV1,
) -> Result<PeriodCapacityReportV1, PeriodCapacityError> {
    if !nonempty(&policy.policy_id) || !nonempty(&policy.expected_time_profile_id) {
        return Err(PeriodCapacityError::InvalidPolicy);
    }

    let mut quotas_by_id = BTreeMap::new();
    let mut quota_ids = BTreeSet::new();
    let mut periods_by_resource: BTreeMap<
        (CargoTransferCapacityResourceKind, String),
        Vec<(u64, u64, String)>,
    > = BTreeMap::new();
    let mut units_by_resource: BTreeMap<(CargoTransferCapacityResourceKind, String), QuantityUnit> =
        BTreeMap::new();

    for quota in quotas {
        validate_period_quota(quota, evaluation_time_epoch_seconds, policy)?;
        let evidence = &quota.evidence;
        if !quota_ids.insert(evidence.budget_id.clone()) {
            return Err(PeriodCapacityError::DuplicateQuotaIdentity);
        }

        let resource_key = (evidence.resource_kind, evidence.resource_id.clone());
        if units_by_resource
            .insert(resource_key.clone(), evidence.capacity_unit)
            .is_some_and(|previous| previous != evidence.capacity_unit)
        {
            return Err(PeriodCapacityError::CapacityUnitMismatch);
        }
        periods_by_resource.entry(resource_key).or_default().push((
            quota.period_start_epoch_seconds,
            quota.period_end_epoch_seconds,
            evidence.budget_id.clone(),
        ));
        quotas_by_id.insert(evidence.budget_id.clone(), quota);
    }

    for periods in periods_by_resource.values_mut() {
        periods.sort_by(|left, right| (left.0, left.1, &left.2).cmp(&(right.0, right.1, &right.2)));
        for pair in periods.windows(2) {
            if pair[1].0 < pair[0].1 {
                return Err(PeriodCapacityError::AmbiguousQuotaEvidence);
            }
        }
    }

    let mut allocation_ids = BTreeSet::new();
    let mut referenced_quotas = BTreeSet::new();
    let mut used_by_quota: BTreeMap<String, u64> = BTreeMap::new();
    let mut allocations_by_quota: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for item in uses {
        if !nonempty(&item.allocation_id) || !nonempty(&item.quota_id) {
            return Err(PeriodCapacityError::InvalidCapacityUse);
        }
        if !allocation_ids.insert(item.allocation_id.clone()) {
            return Err(PeriodCapacityError::DuplicateAllocationIdentity);
        }
        if item.capacity_units_used == 0 {
            return Err(PeriodCapacityError::InvalidCapacityUse);
        }
        let quota = quotas_by_id
            .get(&item.quota_id)
            .ok_or(PeriodCapacityError::UnknownQuota)?;
        if item.capacity_unit != quota.evidence.capacity_unit {
            return Err(PeriodCapacityError::CapacityUnitMismatch);
        }

        let used = used_by_quota.entry(item.quota_id.clone()).or_default();
        *used = used
            .checked_add(item.capacity_units_used)
            .ok_or(PeriodCapacityError::CapacityArithmeticOverflow)?;
        if *used > quota.evidence.capacity_units {
            return Err(PeriodCapacityError::CapacityExceeded);
        }
        referenced_quotas.insert(item.quota_id.clone());
        allocations_by_quota
            .entry(item.quota_id.clone())
            .or_default()
            .push(item.allocation_id.clone());
    }

    if quotas_by_id
        .keys()
        .any(|quota_id| !referenced_quotas.contains(quota_id))
    {
        return Err(PeriodCapacityError::UnreferencedQuota);
    }

    let mut receipts = quotas
        .iter()
        .map(|quota| {
            let evidence = &quota.evidence;
            let mut ids = allocations_by_quota
                .get(&evidence.budget_id)
                .cloned()
                .unwrap_or_default();
            ids.sort();
            PeriodCapacityReceiptV1 {
                quota_id: evidence.budget_id.clone(),
                resource_kind: evidence.resource_kind,
                resource_id: evidence.resource_id.clone(),
                terminal_id: evidence.terminal_id.clone(),
                capacity_scope_id: evidence.capacity_scope_id.clone(),
                period_start_epoch_seconds: quota.period_start_epoch_seconds,
                period_end_epoch_seconds: quota.period_end_epoch_seconds,
                capacity_unit: evidence.capacity_unit,
                capacity_units_used: used_by_quota
                    .get(&evidence.budget_id)
                    .copied()
                    .unwrap_or_default(),
                capacity_units_limit: evidence.capacity_units,
                allocation_ids: ids,
                policy_id: policy.policy_id.clone(),
                time_profile_id: evidence.time_profile_id.clone(),
                source_record_id: evidence.source_record_id.clone(),
                source_schema_profile_id: evidence.source_schema_profile_id.clone(),
                authority_id: evidence.authority_id.clone(),
                authority_profile_id: evidence.authority_profile_id.clone(),
                source_record_digest: evidence.source_record_digest.clone(),
                revision: evidence.revision,
                supersedes_revision: evidence.supersedes_revision,
                source_event_time_epoch_seconds: evidence.source_event_time_epoch_seconds,
                capacity_observed_at_epoch_seconds: evidence
                    .capacity_observed_at_epoch_seconds
                    .expect("validated observation time"),
                repository_recorded_at_epoch_seconds: evidence
                    .repository_recorded_at_epoch_seconds
                    .expect("validated repository record time"),
                valid_from_epoch_seconds: evidence
                    .valid_from_epoch_seconds
                    .expect("validated validity interval"),
                valid_until_epoch_seconds: evidence
                    .valid_until_epoch_seconds
                    .expect("validated validity interval"),
            }
        })
        .collect::<Vec<_>>();
    receipts.sort_by(|left, right| {
        (
            left.resource_kind,
            &left.resource_id,
            left.period_start_epoch_seconds,
            left.period_end_epoch_seconds,
            &left.quota_id,
        )
            .cmp(&(
                right.resource_kind,
                &right.resource_id,
                right.period_start_epoch_seconds,
                right.period_end_epoch_seconds,
                &right.quota_id,
            ))
    });

    Ok(PeriodCapacityReportV1 {
        evaluation_time_epoch_seconds,
        policy_id: policy.policy_id.clone(),
        receipts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> PeriodCapacityPolicyV1 {
        PeriodCapacityPolicyV1 {
            policy_id: "daily-tonnage-policy-v1".to_owned(),
            expected_time_profile_id: "utc-unix-v1".to_owned(),
            max_observation_age_seconds: 200,
            allowed_future_clock_skew_seconds: 2,
        }
    }

    fn quota(
        id: &str,
        resource: &str,
        scope: &str,
        start: u64,
        end: u64,
        limit: u64,
    ) -> PeriodCapacityQuotaV1 {
        PeriodCapacityQuotaV1 {
            evidence: ScheduledCapacityEvidenceV1 {
                budget_id: id.to_owned(),
                resource_kind: CargoTransferCapacityResourceKind::TransferEdge,
                resource_id: resource.to_owned(),
                terminal_id: None,
                capacity_scope_id: scope.to_owned(),
                capacity_units: limit,
                capacity_unit: QuantityUnit::Tonnes,
                authority_id: "infra-manager".to_owned(),
                authority_profile_id: "rail-capacity-v1".to_owned(),
                source_record_id: "annexure-33b".to_owned(),
                source_schema_profile_id: "transnet-capacity-v1".to_owned(),
                source_record_digest: "sha256:reviewed-fixture-digest".to_owned(),
                revision: 1,
                supersedes_revision: None,
                source_event_time_epoch_seconds: None,
                capacity_observed_at_epoch_seconds: Some(100),
                repository_recorded_at_epoch_seconds: Some(100),
                valid_from_epoch_seconds: Some(start),
                valid_until_epoch_seconds: Some(end),
                time_profile_id: "utc-unix-v1".to_owned(),
                status: CargoTransferCapacityEvidenceStatus::ReportedCurrent,
            },
            period_start_epoch_seconds: start,
            period_end_epoch_seconds: end,
        }
    }

    fn use_of(id: &str, quota_id: &str, units: u64) -> PeriodCapacityUseV1 {
        PeriodCapacityUseV1 {
            allocation_id: id.to_owned(),
            quota_id: quota_id.to_owned(),
            capacity_units_used: units,
            capacity_unit: QuantityUnit::Tonnes,
        }
    }

    #[test]
    fn sums_period_consumption_and_returns_sorted_provenance_receipts() {
        let quotas = vec![
            quota("next-day", "rail-edge", "daily", 200, 300, 10),
            quota("later-day", "rail-edge", "daily", 300, 400, 20),
        ];
        let uses = vec![
            use_of("shipment-b", "next-day", 4),
            use_of("shipment-a", "next-day", 3),
            use_of("shipment-c", "later-day", 15),
        ];
        let report = validate_period_capacity_plan(&uses, &quotas, 100, &policy()).unwrap();
        assert_eq!(report.receipts.len(), 2);
        assert_eq!(report.receipts[0].quota_id, "next-day");
        assert_eq!(report.receipts[0].capacity_units_used, 7);
        assert_eq!(report.receipts[0].capacity_units_limit, 10);
        assert_eq!(report.receipts[0].authority_id, "infra-manager".to_owned());
        assert_eq!(
            report.receipts[0].authority_profile_id,
            "rail-capacity-v1".to_owned()
        );
        assert_eq!(
            report.receipts[0].allocation_ids,
            vec!["shipment-a".to_owned(), "shipment-b".to_owned()]
        );
        assert_eq!(report.receipts[1].capacity_units_used, 15);
        assert_eq!(report.receipts[1].source_record_id, "annexure-33b");
    }

    #[test]
    fn input_order_does_not_change_the_report() {
        let quota_a = quota("quota-a", "edge-a", "daily", 200, 300, 10);
        let quota_b = quota("quota-b", "edge-b", "daily", 200, 300, 10);
        let uses_a = vec![
            use_of("use-z", "quota-a", 2),
            use_of("use-a", "quota-a", 3),
            use_of("use-b", "quota-b", 1),
        ];
        let uses_b = vec![
            use_of("use-b", "quota-b", 1),
            use_of("use-a", "quota-a", 3),
            use_of("use-z", "quota-a", 2),
        ];
        let quotas_a = vec![quota_b.clone(), quota_a.clone()];
        let quotas_b = vec![quota_a, quota_b];
        assert_eq!(
            validate_period_capacity_plan(&uses_a, &quotas_a, 100, &policy()),
            validate_period_capacity_plan(&uses_b, &quotas_b, 100, &policy())
        );
    }

    #[test]
    fn rejects_period_overcommitment_and_checked_sum_overflow() {
        let q = quota("quota-a", "edge-a", "daily", 200, 300, 10);
        assert_eq!(
            validate_period_capacity_plan(
                &[use_of("a", "quota-a", 6), use_of("b", "quota-a", 5)],
                &[q.clone()],
                100,
                &policy(),
            ),
            Err(PeriodCapacityError::CapacityExceeded)
        );
        let q = quota("quota-a", "edge-a", "daily", 200, 300, u64::MAX);
        assert_eq!(
            validate_period_capacity_plan(
                &[use_of("a", "quota-a", u64::MAX), use_of("b", "quota-a", 1)],
                &[q],
                100,
                &policy(),
            ),
            Err(PeriodCapacityError::CapacityArithmeticOverflow)
        );
    }

    #[test]
    fn rejects_overlapping_physical_resource_quotas_even_when_scopes_differ() {
        let quotas = vec![
            quota("scope-a", "edge-a", "scope-a", 200, 350, 10),
            quota("scope-b", "edge-a", "scope-b", 300, 400, 10),
        ];
        let uses = vec![use_of("use-a", "scope-a", 1), use_of("use-b", "scope-b", 1)];
        assert_eq!(
            validate_period_capacity_plan(&uses, &quotas, 100, &policy()),
            Err(PeriodCapacityError::AmbiguousQuotaEvidence)
        );
    }

    #[test]
    fn accepts_adjacent_periods_without_double_counting() {
        let quotas = vec![
            quota("period-a", "edge-a", "daily", 200, 300, 10),
            quota("period-b", "edge-a", "daily", 300, 400, 12),
        ];
        let uses = vec![
            use_of("use-a", "period-a", 10),
            use_of("use-b", "period-b", 12),
        ];
        assert!(validate_period_capacity_plan(&uses, &quotas, 100, &policy()).is_ok());
    }

    #[test]
    fn rejects_stale_evidence_at_period_start() {
        let mut q = quota("quota-a", "edge-a", "daily", 400, 500, 10);
        q.evidence.capacity_observed_at_epoch_seconds = Some(100);
        q.evidence.repository_recorded_at_epoch_seconds = Some(100);
        assert_eq!(
            validate_period_capacity_plan(&[use_of("use-a", "quota-a", 1)], &[q], 100, &policy()),
            Err(PeriodCapacityError::ObservationTooOldAtPeriodStart)
        );
    }

    #[test]
    fn rejects_evidence_that_does_not_cover_the_whole_period() {
        let mut q = quota("quota-a", "edge-a", "daily", 200, 300, 10);
        q.evidence.valid_until_epoch_seconds = Some(299);
        assert_eq!(
            validate_period_capacity_plan(&[use_of("use-a", "quota-a", 1)], &[q], 100, &policy()),
            Err(PeriodCapacityError::PeriodNotCoveredByEvidence)
        );
    }

    #[test]
    fn rejects_noncurrent_evidence_and_periods_already_started() {
        let mut q = quota("quota-a", "edge-a", "daily", 200, 300, 10);
        q.evidence.status = CargoTransferCapacityEvidenceStatus::Conflicted;
        assert_eq!(
            validate_period_capacity_plan(&[use_of("use-a", "quota-a", 1)], &[q], 100, &policy()),
            Err(PeriodCapacityError::NotCurrent)
        );
        let q = quota("quota-a", "edge-a", "daily", 99, 300, 10);
        assert_eq!(
            validate_period_capacity_plan(&[use_of("use-a", "quota-a", 1)], &[q], 100, &policy()),
            Err(PeriodCapacityError::PeriodAlreadyStarted)
        );
    }

    #[test]
    fn rejects_unknown_unused_duplicate_and_unit_mismatched_uses() {
        let q = quota("quota-a", "edge-a", "daily", 200, 300, 10);
        assert_eq!(
            validate_period_capacity_plan(
                &[use_of("use-a", "missing", 1)],
                &[q.clone()],
                100,
                &policy(),
            ),
            Err(PeriodCapacityError::UnknownQuota)
        );
        assert_eq!(
            validate_period_capacity_plan(&[], &[q.clone()], 100, &policy()),
            Err(PeriodCapacityError::UnreferencedQuota)
        );
        assert_eq!(
            validate_period_capacity_plan(
                &[
                    use_of("same-id", "quota-a", 1),
                    use_of("same-id", "quota-a", 1)
                ],
                &[q.clone()],
                100,
                &policy(),
            ),
            Err(PeriodCapacityError::DuplicateAllocationIdentity)
        );
        let mut wrong_unit = use_of("use-a", "quota-a", 1);
        wrong_unit.capacity_unit = QuantityUnit::Teu;
        assert_eq!(
            validate_period_capacity_plan(&[wrong_unit], &[q], 100, &policy()),
            Err(PeriodCapacityError::CapacityUnitMismatch)
        );
    }
}
