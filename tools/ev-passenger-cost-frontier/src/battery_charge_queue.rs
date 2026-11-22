// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Exact finite-horizon charger queue oracle.
//!
//! The evaluator schedules fixed-duration charging blocks only. It is not a
//! charge-curve/electrochemistry model, charger controller, or charge release
//! authority.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::battery_service_network::{
    EvidenceClass, EvidenceEnvelope, PackState, PackStateEvidence,
};

const MAX_JOBS: usize = 4;
const MAX_CHARGERS: usize = 3;
const MAX_HORIZON_SLOTS: u8 = 8;
const CLAIM_CEILING: &str = "Exact synthetic fixed-duration charging-block schedule only; not a measured charge curve, battery physics model, safe charging authorization, return-to-service authority, flight authorization or commercial performance.";

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChargeJob {
    pub job_id: String,
    pub pack_id: String,
    pub pack_configuration_digest: String,
    pub pack_state: PackStateEvidence,
    pub release_slot: u8,
    pub deadline_slot_exclusive: u8,
    pub duration_slots: u8,
    pub requested_power_w: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChargerChannel {
    pub charger_id: String,
    pub configuration_digest: String,
    pub max_power_w: u32,
    pub supported_pack_configuration_digests: Vec<String>,
    pub availability_evidence: EvidenceEnvelope,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GridSlot {
    pub slot: u8,
    pub import_cap_w: u32,
    pub non_charging_site_load_w: u32,
    pub evidence: EvidenceEnvelope,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BatteryChargeQueueInput {
    pub schema_version: String,
    pub benchmark_id: String,
    pub site_id: String,
    pub site_configuration_digest: String,
    pub decision_time_unix_ms: u64,
    pub slot_duration_seconds: u32,
    pub horizon_slots: u8,
    pub site_evidence: EvidenceEnvelope,
    pub jobs: Vec<ChargeJob>,
    pub chargers: Vec<ChargerChannel>,
    pub grid_slots: Vec<GridSlot>,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ChargeAssignment {
    pub job_id: String,
    pub pack_id: String,
    pub charger_id: String,
    pub start_slot: u8,
    pub end_slot_exclusive: u8,
    pub wait_slots: u8,
    pub duration_slots: u8,
    pub requested_power_w: u32,
    pub declared_grid_input_watt_seconds: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct BatteryChargeQueueReport {
    pub schema_version: String,
    pub benchmark_id: String,
    pub site_id: String,
    pub decision_time_unix_ms: u64,
    pub slot_duration_seconds: u32,
    pub horizon_slots: u8,
    pub job_count: usize,
    pub charger_count: usize,
    pub search_nodes: usize,
    pub completed_job_count: usize,
    pub assignments: Vec<ChargeAssignment>,
    pub unserved_job_ids: Vec<String>,
    pub findings: Vec<String>,
    pub total_declared_grid_input_watt_seconds: u64,
    pub all_fixture_evidence_synthetic: bool,
    pub charge_authority_granted: bool,
    pub return_to_service_authority_granted: bool,
    pub claim_ceiling: &'static str,
}

#[derive(Debug, Clone)]
struct CandidatePlacement {
    charger_index: usize,
    start_slot: u8,
    end_slot_exclusive: u8,
    wait_slots: u8,
    requested_power_w: u32,
    declared_grid_input_watt_seconds: u64,
}

#[derive(Debug, Clone)]
struct SearchBest {
    served: usize,
    total_wait_slots: u64,
    makespan_slot: u8,
    selected: Vec<Option<CandidatePlacement>>,
}

struct ExactChargeSearch<'a> {
    candidates: &'a [Vec<CandidatePlacement>],
    jobs: &'a [ChargeJob],
    chargers: &'a [ChargerChannel],
    horizon_slots: u8,
    slot_power_headroom_w: &'a [u32],
    nodes: usize,
    best: Option<SearchBest>,
}

fn add_error(errors: &mut Vec<String>, message: impl Into<String>) {
    errors.push(message.into());
}

fn evidence_covers_interval(
    evidence: &EvidenceEnvelope,
    asset_id: &str,
    configuration_digest: &str,
    decision_time_unix_ms: u64,
    interval_start_unix_ms: u64,
    interval_end_exclusive_unix_ms: u64,
) -> bool {
    if evidence.asset_id != asset_id
        || evidence.configuration_digest != configuration_digest
        || evidence.asset_id.trim().is_empty()
        || evidence.configuration_digest.trim().is_empty()
        || evidence.producer_id.trim().is_empty()
        || evidence.method_id.trim().is_empty()
        || evidence.evidence_reference.trim().is_empty()
        || evidence.evidence_class == EvidenceClass::Unknown
    {
        return false;
    }

    match (
        evidence.observed_at_unix_ms,
        evidence.valid_from_unix_ms,
        evidence.valid_until_unix_ms,
    ) {
        (Some(observed_at), Some(valid_from), Some(valid_until)) => {
            valid_from <= valid_until
                && valid_from <= observed_at
                && observed_at <= valid_until
                && observed_at <= decision_time_unix_ms
                && valid_from <= interval_start_unix_ms
                && interval_end_exclusive_unix_ms.saturating_sub(1) <= valid_until
        }
        _ => false,
    }
}

fn slot_time_ms(
    decision_time_unix_ms: u64,
    slot_duration_seconds: u32,
    slot: u8,
) -> Option<u64> {
    let offset_ms = u64::from(slot)
        .checked_mul(u64::from(slot_duration_seconds))?
        .checked_mul(1_000)?;
    decision_time_unix_ms.checked_add(offset_ms)
}

impl ExactChargeSearch<'_> {
    fn search(
        &mut self,
        job_index: usize,
        selected: &mut Vec<Option<CandidatePlacement>>,
        charger_busy: &mut [Vec<bool>],
        grid_used_w: &mut [u32],
        served: usize,
        total_wait_slots: u64,
        makespan_slot: u8,
    ) {
        self.nodes += 1;
        if job_index == self.candidates.len() {
            let candidate = SearchBest {
                served,
                total_wait_slots,
                makespan_slot,
                selected: selected.clone(),
            };
            if self
                .best
                .as_ref()
                .is_none_or(|current| self.is_better(&candidate, current))
            {
                self.best = Some(candidate);
            }
            return;
        }

        let placements = self.candidates[job_index].clone();
        for placement in placements {
            let charger_index = placement.charger_index;
            let start = usize::from(placement.start_slot);
            let end = usize::from(placement.end_slot_exclusive);
            let power = placement.requested_power_w;

            let charger_free = (start..end).all(|slot| !charger_busy[charger_index][slot]);
            let grid_feasible = (start..end).all(|slot| {
                grid_used_w[slot]
                    .checked_add(power)
                    .is_some_and(|total| total <= self.slot_power_headroom_w[slot])
            });
            if !charger_free || !grid_feasible {
                continue;
            }

            for slot in start..end {
                charger_busy[charger_index][slot] = true;
                grid_used_w[slot] += power;
            }
            selected.push(Some(placement.clone()));
            self.search(
                job_index + 1,
                selected,
                charger_busy,
                grid_used_w,
                served + 1,
                total_wait_slots + u64::from(placement.wait_slots),
                makespan_slot.max(placement.end_slot_exclusive),
            );
            selected.pop();
            for slot in start..end {
                charger_busy[charger_index][slot] = false;
                grid_used_w[slot] -= power;
            }
        }

        selected.push(None);
        self.search(
            job_index + 1,
            selected,
            charger_busy,
            grid_used_w,
            served,
            total_wait_slots,
            makespan_slot,
        );
        selected.pop();
    }

    fn is_better(&self, candidate: &SearchBest, current: &SearchBest) -> bool {
        if candidate.served != current.served {
            return candidate.served > current.served;
        }
        if candidate.total_wait_slots != current.total_wait_slots {
            return candidate.total_wait_slots < current.total_wait_slots;
        }
        if candidate.makespan_slot != current.makespan_slot {
            return candidate.makespan_slot < current.makespan_slot;
        }
        charge_assignment_key(candidate, self.jobs, self.chargers)
            < charge_assignment_key(current, self.jobs, self.chargers)
    }
}

fn charge_assignment_key(
    result: &SearchBest,
    jobs: &[ChargeJob],
    chargers: &[ChargerChannel],
) -> Vec<String> {
    result
        .selected
        .iter()
        .enumerate()
        .map(|(index, placement)| match placement {
            Some(placement) => format!(
                "0:{}:{}:{:08}",
                jobs[index].job_id,
                chargers[placement.charger_index].charger_id,
                placement.start_slot
            ),
            None => format!("1:{}", jobs[index].job_id),
        })
        .collect()
}

pub fn evaluate_battery_charge_queue(
    input: &BatteryChargeQueueInput,
) -> Result<BatteryChargeQueueReport, Vec<String>> {
    let mut errors = Vec::new();
    if input.schema_version != "0.1.0"
        || input.benchmark_id != "CHARGER-QUEUE-ORACLE-001"
        || input.site_id.trim().is_empty()
        || input.site_configuration_digest.trim().is_empty()
        || input.claim_ceiling.trim().is_empty()
        || input.decision_time_unix_ms == 0
        || input.slot_duration_seconds == 0
        || input.horizon_slots == 0
        || input.horizon_slots > MAX_HORIZON_SLOTS
    {
        add_error(&mut errors, "unsupported_identity_or_time_horizon");
    }
    if input.jobs.is_empty()
        || input.jobs.len() > MAX_JOBS
        || input.chargers.is_empty()
        || input.chargers.len() > MAX_CHARGERS
        || input.grid_slots.len() != usize::from(input.horizon_slots)
    {
        add_error(&mut errors, "exact_v0_1_limits_or_grid_slot_count_invalid");
    }

    let horizon_end_unix_ms = slot_time_ms(
        input.decision_time_unix_ms,
        input.slot_duration_seconds,
        input.horizon_slots,
    );
    let Some(horizon_end_unix_ms) = horizon_end_unix_ms else {
        add_error(&mut errors, "horizon_time_overflow");
        return Err(errors);
    };

    if !evidence_covers_interval(
        &input.site_evidence,
        &input.site_id,
        &input.site_configuration_digest,
        input.decision_time_unix_ms,
        input.decision_time_unix_ms,
        horizon_end_unix_ms,
    ) {
        add_error(&mut errors, "site_evidence_not_admissible_for_horizon");
    }

    let mut charger_ids = BTreeSet::new();
    for charger in &input.chargers {
        if charger.charger_id.trim().is_empty()
            || charger.configuration_digest.trim().is_empty()
            || charger.max_power_w == 0
            || charger.supported_pack_configuration_digests.is_empty()
            || !charger_ids.insert(charger.charger_id.as_str())
        {
            add_error(&mut errors, "charger_identity_or_capability_invalid");
        }
        let mut supported = BTreeSet::new();
        for digest in &charger.supported_pack_configuration_digests {
            if digest.trim().is_empty() || !supported.insert(digest.as_str()) {
                add_error(&mut errors, "charger_supported_configuration_empty_or_duplicate");
            }
        }
        if !evidence_covers_interval(
            &charger.availability_evidence,
            &charger.charger_id,
            &charger.configuration_digest,
            input.decision_time_unix_ms,
            input.decision_time_unix_ms,
            horizon_end_unix_ms,
        ) {
            add_error(
                &mut errors,
                format!("charger_availability_evidence_not_admissible:{}", charger.charger_id),
            );
        }
    }

    let mut job_ids = BTreeSet::new();
    let mut pack_ids = BTreeSet::new();
    for job in &input.jobs {
        if job.job_id.trim().is_empty()
            || job.pack_id.trim().is_empty()
            || job.pack_configuration_digest.trim().is_empty()
            || !job_ids.insert(job.job_id.as_str())
            || !pack_ids.insert(job.pack_id.as_str())
            || job.duration_slots == 0
            || job.requested_power_w == 0
            || job.release_slot >= job.deadline_slot_exclusive
            || job.deadline_slot_exclusive > input.horizon_slots
        {
            add_error(&mut errors, format!("charge_job_identity_or_window_invalid:{}", job.job_id));
        }
        if job.pack_state.evidence.asset_id != job.pack_id
            || job.pack_state.evidence.configuration_digest != job.pack_configuration_digest
        {
            add_error(&mut errors, format!("charge_job_pack_state_binding_mismatch:{}", job.job_id));
        }
        if job.pack_state.value != PackState::Cooling {
            add_error(&mut errors, format!("charge_job_pack_not_cooling:{}", job.job_id));
        }
    }

    let mut sorted_grid_slots = input.grid_slots.clone();
    sorted_grid_slots.sort_by_key(|slot| slot.slot);
    for (index, grid_slot) in sorted_grid_slots.iter().enumerate() {
        if usize::from(grid_slot.slot) != index
            || grid_slot.import_cap_w < grid_slot.non_charging_site_load_w
        {
            add_error(&mut errors, format!("grid_slot_identity_or_capacity_invalid:{}", grid_slot.slot));
            continue;
        }
        let Some(slot_start) = slot_time_ms(
            input.decision_time_unix_ms,
            input.slot_duration_seconds,
            grid_slot.slot,
        ) else {
            add_error(&mut errors, "grid_slot_time_overflow");
            continue;
        };
        let Some(slot_end) = slot_time_ms(
            input.decision_time_unix_ms,
            input.slot_duration_seconds,
            grid_slot.slot.saturating_add(1),
        ) else {
            add_error(&mut errors, "grid_slot_time_overflow");
            continue;
        };
        if !evidence_covers_interval(
            &grid_slot.evidence,
            &input.site_id,
            &input.site_configuration_digest,
            input.decision_time_unix_ms,
            slot_start,
            slot_end,
        ) {
            add_error(&mut errors, format!("grid_slot_evidence_not_admissible:{}", grid_slot.slot));
        }
    }

    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        return Err(errors);
    }

    let mut jobs = input.jobs.clone();
    jobs.sort_by(|left, right| {
        left.deadline_slot_exclusive
            .cmp(&right.deadline_slot_exclusive)
            .then_with(|| left.job_id.cmp(&right.job_id))
    });
    let mut chargers = input.chargers.clone();
    chargers.sort_by(|left, right| left.charger_id.cmp(&right.charger_id));
    sorted_grid_slots.sort_by_key(|slot| slot.slot);

    let mut slot_power_headroom_w = Vec::with_capacity(sorted_grid_slots.len());
    for grid_slot in &sorted_grid_slots {
        slot_power_headroom_w.push(grid_slot.import_cap_w - grid_slot.non_charging_site_load_w);
    }

    let mut findings = Vec::new();
    let mut candidates: Vec<Vec<CandidatePlacement>> = Vec::with_capacity(jobs.len());
    for job in &jobs {
        let mut job_placements = Vec::new();
        let mut had_compatible_charger = false;
        for (charger_index, charger) in chargers.iter().enumerate() {
            if charger.max_power_w < job.requested_power_w
                || !charger
                    .supported_pack_configuration_digests
                    .iter()
                    .any(|digest| digest == &job.pack_configuration_digest)
            {
                continue;
            }
            had_compatible_charger = true;
            for start in job.release_slot..job.deadline_slot_exclusive {
                let end = start + job.duration_slots;
                if end > job.deadline_slot_exclusive || end > input.horizon_slots {
                    continue;
                }
                let Some(start_time) = slot_time_ms(
                    input.decision_time_unix_ms,
                    input.slot_duration_seconds,
                    start,
                ) else {
                    continue;
                };
                let Some(end_time) = slot_time_ms(
                    input.decision_time_unix_ms,
                    input.slot_duration_seconds,
                    end,
                ) else {
                    continue;
                };
                if !evidence_covers_interval(
                    &job.pack_state.evidence,
                    &job.pack_id,
                    &job.pack_configuration_digest,
                    input.decision_time_unix_ms,
                    start_time,
                    end_time,
                ) {
                    continue;
                }
                let watt_seconds = u64::from(job.requested_power_w)
                    .checked_mul(u64::from(job.duration_slots))
                    .and_then(|value| value.checked_mul(u64::from(input.slot_duration_seconds)));
                let Some(watt_seconds) = watt_seconds else {
                    continue;
                };
                job_placements.push(CandidatePlacement {
                    charger_index,
                    start_slot: start,
                    end_slot_exclusive: end,
                    wait_slots: start - job.release_slot,
                    requested_power_w: job.requested_power_w,
                    declared_grid_input_watt_seconds: watt_seconds,
                });
            }
        }
        if !had_compatible_charger {
            findings.push(format!("no_compatible_charger:{}", job.job_id));
        } else if job_placements.is_empty() {
            findings.push(format!("no_evidence_valid_deadline_feasible_placement:{}", job.job_id));
        }
        candidates.push(job_placements);
    }

    let mut search = ExactChargeSearch {
        candidates: &candidates,
        jobs: &jobs,
        chargers: &chargers,
        horizon_slots: input.horizon_slots,
        slot_power_headroom_w: &slot_power_headroom_w,
        nodes: 0,
        best: None,
    };
    let mut charger_busy = vec![vec![false; usize::from(input.horizon_slots)]; chargers.len()];
    let mut grid_used_w = vec![0_u32; usize::from(input.horizon_slots)];
    search.search(
        0,
        &mut Vec::new(),
        &mut charger_busy,
        &mut grid_used_w,
        0,
        0,
        0,
    );
    let search_nodes = search.nodes;
    let best = search
        .best
        .expect("validated non-empty job set always has a schedule");

    let mut assignments = Vec::new();
    let mut unserved_job_ids = Vec::new();
    for (job_index, selected) in best.selected.iter().enumerate() {
        match selected {
            Some(placement) => assignments.push(ChargeAssignment {
                job_id: jobs[job_index].job_id.clone(),
                pack_id: jobs[job_index].pack_id.clone(),
                charger_id: chargers[placement.charger_index].charger_id.clone(),
                start_slot: placement.start_slot,
                end_slot_exclusive: placement.end_slot_exclusive,
                wait_slots: placement.wait_slots,
                duration_slots: jobs[job_index].duration_slots,
                requested_power_w: placement.requested_power_w,
                declared_grid_input_watt_seconds: placement.declared_grid_input_watt_seconds,
            }),
            None => unserved_job_ids.push(jobs[job_index].job_id.clone()),
        }
    }
    assignments.sort_by(|left, right| {
        left.start_slot
            .cmp(&right.start_slot)
            .then_with(|| left.charger_id.cmp(&right.charger_id))
            .then_with(|| left.job_id.cmp(&right.job_id))
    });

    let total_declared_grid_input_watt_seconds = assignments
        .iter()
        .try_fold(0_u64, |total, assignment| {
            total.checked_add(assignment.declared_grid_input_watt_seconds)
        })
        .expect("bounded exact instance grid input energy sum must fit u64");
    let all_fixture_evidence_synthetic = input.site_evidence.evidence_class == EvidenceClass::Synthetic
        && input.chargers.iter().all(|charger| charger.availability_evidence.evidence_class == EvidenceClass::Synthetic)
        && input.grid_slots.iter().all(|slot| slot.evidence.evidence_class == EvidenceClass::Synthetic)
        && input.jobs.iter().all(|job| job.pack_state.evidence.evidence_class == EvidenceClass::Synthetic);

    Ok(BatteryChargeQueueReport {
        schema_version: "0.1.0".to_string(),
        benchmark_id: "CHARGER-QUEUE-ORACLE-001".to_string(),
        site_id: input.site_id.clone(),
        decision_time_unix_ms: input.decision_time_unix_ms,
        slot_duration_seconds: input.slot_duration_seconds,
        horizon_slots: input.horizon_slots,
        job_count: jobs.len(),
        charger_count: chargers.len(),
        search_nodes,
        completed_job_count: assignments.len(),
        assignments,
        unserved_job_ids,
        findings,
        total_declared_grid_input_watt_seconds,
        all_fixture_evidence_synthetic,
        charge_authority_granted: false,
        return_to_service_authority_granted: false,
        claim_ceiling: CLAIM_CEILING,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DECISION_TIME_MS: u64 = 1_795_219_200_000;

    fn evidence(
        asset_id: &str,
        configuration_digest: &str,
        suffix: &str,
        observed_at_unix_ms: u64,
        valid_until_unix_ms: u64,
    ) -> EvidenceEnvelope {
        EvidenceEnvelope {
            asset_id: asset_id.to_string(),
            configuration_digest: configuration_digest.to_string(),
            producer_id: "urn:synthetic:producer:charge-queue".to_string(),
            method_id: format!("urn:synthetic:method:{suffix}"),
            evidence_reference: format!("urn:synthetic:evidence:{suffix}"),
            evidence_class: EvidenceClass::Synthetic,
            observed_at_unix_ms: Some(observed_at_unix_ms),
            valid_from_unix_ms: Some(DECISION_TIME_MS - 10_000),
            valid_until_unix_ms: Some(valid_until_unix_ms),
        }
    }

    fn cooling_job(
        job_id: &str,
        pack_id: &str,
        configuration_digest: &str,
        release_slot: u8,
        deadline_slot_exclusive: u8,
        duration_slots: u8,
        requested_power_w: u32,
    ) -> ChargeJob {
        ChargeJob {
            job_id: job_id.to_string(),
            pack_id: pack_id.to_string(),
            pack_configuration_digest: configuration_digest.to_string(),
            pack_state: PackStateEvidence {
                evidence: evidence(
                    pack_id,
                    configuration_digest,
                    &format!("{job_id}-state"),
                    DECISION_TIME_MS - 1_000,
                    DECISION_TIME_MS + 100_000,
                ),
                value: PackState::Cooling,
            },
            release_slot,
            deadline_slot_exclusive,
            duration_slots,
            requested_power_w,
        }
    }

    fn charger(
        id: &str,
        config_digest: &str,
        max_power_w: u32,
        supported: Vec<&str>,
    ) -> ChargerChannel {
        ChargerChannel {
            charger_id: id.to_string(),
            configuration_digest: config_digest.to_string(),
            max_power_w,
            supported_pack_configuration_digests: supported.into_iter().map(str::to_string).collect(),
            availability_evidence: evidence(
                id,
                config_digest,
                &format!("{id}-availability"),
                DECISION_TIME_MS - 1_000,
                DECISION_TIME_MS + 100_000,
            ),
        }
    }

    fn grid_slot(
        slot: u8,
        import_cap_w: u32,
        base_load_w: u32,
        site_id: &str,
        site_digest: &str,
        slot_duration_seconds: u32,
    ) -> GridSlot {
        let start = DECISION_TIME_MS
            + u64::from(slot) * u64::from(slot_duration_seconds) * 1_000;
        let end = start + u64::from(slot_duration_seconds) * 1_000;
        GridSlot {
            slot,
            import_cap_w,
            non_charging_site_load_w: base_load_w,
            evidence: evidence(
                site_id,
                site_digest,
                &format!("grid-slot-{slot}"),
                DECISION_TIME_MS - 1_000,
                end,
            ),
        }
    }

    fn base_input() -> BatteryChargeQueueInput {
        let site_id = "site-1";
        let site_digest = "urn:synthetic:site-config:1";
        let slot_duration_seconds = 900;
        let horizon_slots = 6;
        BatteryChargeQueueInput {
            schema_version: "0.1.0".to_string(),
            benchmark_id: "CHARGER-QUEUE-ORACLE-001".to_string(),
            site_id: site_id.to_string(),
            site_configuration_digest: site_digest.to_string(),
            decision_time_unix_ms: DECISION_TIME_MS,
            slot_duration_seconds,
            horizon_slots,
            site_evidence: evidence(
                site_id,
                site_digest,
                "site-horizon",
                DECISION_TIME_MS - 1_000,
                DECISION_TIME_MS + u64::from(horizon_slots) * 900_000,
            ),
            jobs: vec![
                cooling_job("job-a", "pack-a", "cfg-a", 0, 4, 2, 3_000),
                cooling_job("job-b", "pack-b", "cfg-a", 0, 4, 2, 3_500),
                cooling_job("job-c", "pack-c", "cfg-b", 1, 6, 3, 3_000),
                cooling_job("job-d", "pack-d", "cfg-a", 0, 3, 3, 2_500),
            ],
            chargers: vec![
                charger("charger-1", "charger-config-1", 4_000, vec!["cfg-a", "cfg-b"]),
                charger("charger-2", "charger-config-2", 3_000, vec!["cfg-a"]),
            ],
            grid_slots: (0..horizon_slots)
                .map(|slot| grid_slot(slot, 8_000, 1_000, site_id, site_digest, slot_duration_seconds))
                .collect(),
            claim_ceiling: "synthetic exact fixed-duration charge block scheduling only".to_string(),
        }
    }

    #[test]
    fn exact_queue_schedules_three_jobs_and_leaves_one_unserved() {
        let report = evaluate_battery_charge_queue(&base_input()).unwrap();
        assert_eq!(report.completed_job_count, 3);
        assert_eq!(report.unserved_job_ids, vec!["job-c".to_string()]);
        assert_eq!(report.assignments.len(), 3);
        assert!(!report.charge_authority_granted);
        assert!(!report.return_to_service_authority_granted);
    }

    #[test]
    fn schedule_is_input_order_invariant() {
        let mut input = base_input();
        let first = evaluate_battery_charge_queue(&input).unwrap();
        input.jobs.reverse();
        input.chargers.reverse();
        input.grid_slots.reverse();
        let second = evaluate_battery_charge_queue(&input).unwrap();
        assert_eq!(first.assignments, second.assignments);
        assert_eq!(first.unserved_job_ids, second.unserved_job_ids);
    }

    #[test]
    fn non_cooling_pack_is_not_scheduled() {
        let mut input = base_input();
        input.jobs[0].pack_state.value = PackState::Quarantined;
        let report = evaluate_battery_charge_queue(&input).unwrap();
        assert!(!report.assignments.iter().any(|assignment| assignment.job_id == "job-a"));
        assert!(report.findings.iter().any(|finding| finding == "no_evidence_valid_deadline_feasible_placement:job-a"));
    }

    #[test]
    fn stale_pack_state_evidence_prevents_scheduling() {
        let mut input = base_input();
        input.jobs[0].pack_state.evidence.valid_until_unix_ms = Some(DECISION_TIME_MS - 1);
        let report = evaluate_battery_charge_queue(&input).unwrap();
        assert!(!report.assignments.iter().any(|assignment| assignment.job_id == "job-a"));
    }

    #[test]
    fn grid_cap_is_enforced_in_every_slot() {
        let mut input = base_input();
        for slot in &mut input.grid_slots {
            slot.import_cap_w = 4_000;
            slot.non_charging_site_load_w = 1_000;
        }
        let report = evaluate_battery_charge_queue(&input).unwrap();
        assert!(report.assignments.len() < 3);
    }

    #[test]
    fn unsupported_configuration_is_not_scheduled() {
        let mut input = base_input();
        input.jobs[0].pack_configuration_digest = "unsupported-config".to_string();
        let report = evaluate_battery_charge_queue(&input).unwrap();
        assert!(report.findings.iter().any(|finding| finding == "no_compatible_charger:job-a"));
    }

    #[test]
    fn request_above_charger_power_is_not_scheduled() {
        let mut input = base_input();
        input.jobs[0].requested_power_w = 5_000;
        let report = evaluate_battery_charge_queue(&input).unwrap();
        assert!(report.findings.iter().any(|finding| finding == "no_compatible_charger:job-a"));
    }

    #[test]
    fn duplicate_pack_and_job_ids_are_rejected() {
        let mut input = base_input();
        input.jobs[1].pack_id = input.jobs[0].pack_id.clone();
        assert!(evaluate_battery_charge_queue(&input).is_err());
    }

    #[test]
    fn future_pack_state_observation_is_not_admitted() {
        let mut input = base_input();
        input.jobs[0].pack_state.evidence.observed_at_unix_ms = Some(DECISION_TIME_MS + 1);
        let report = evaluate_battery_charge_queue(&input).unwrap();
        assert!(!report.assignments.iter().any(|assignment| assignment.job_id == "job-a"));
    }
}
