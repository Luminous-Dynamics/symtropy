// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Exact one-epoch battery-pack allocation and capability screen.
//!
//! This v0.1 slice assigns each compatible qualified pack to at most one
//! simultaneous mission. It is not a charging scheduler, battery model,
//! flight authorization service, or return-to-service authority.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

const MAX_EXACT_ITEMS: usize = 6;
const CLAIM_CEILING: &str = "Exact synthetic one-epoch pack allocation only; not battery safety qualification, measured capability, charging-network optimization, dispatch authorization, return-to-service authority, airworthiness or commercial performance.";

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PackState {
    Available,
    Installed,
    Reserved,
    Depleted,
    Charging,
    Cooling,
    Inspection,
    Quarantined,
    InTransit,
    Retired,
    Unknown,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceFreshness {
    Fresh,
    Stale,
    Unknown,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceClass {
    Synthetic,
    Measured,
    IndependentTest,
    Unknown,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MetricEvidence {
    pub freshness: EvidenceFreshness,
    pub evidence_class: EvidenceClass,
    pub evidence_reference: String,
    pub value: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ThermalEvidence {
    pub freshness: EvidenceFreshness,
    pub evidence_class: EvidenceClass,
    pub evidence_reference: String,
    pub pack_temperature_c: Option<i16>,
    pub qualified_max_temperature_c: Option<i16>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BatteryPack {
    pub pack_id: String,
    pub configuration_id: String,
    pub state: PackState,
    pub identity_verified: Option<bool>,
    pub custody_resolved: Option<bool>,
    pub post_test_freshness: EvidenceFreshness,
    pub post_test_passed: Option<bool>,
    pub post_test_evidence_reference: String,
    pub usable_energy: MetricEvidence,
    pub peak_power: MetricEvidence,
    pub thermal: ThermalEvidence,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MissionDemand {
    pub mission_id: String,
    pub configuration_id: String,
    /// Includes the required operational and contingency reserve.
    pub required_energy_with_reserve_wh: u32,
    pub required_peak_power_w: u32,
    pub maximum_pack_temperature_c: i16,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BatteryServiceNetworkInput {
    pub schema_version: String,
    pub benchmark_id: String,
    pub dispatch_epoch_id: String,
    pub packs: Vec<BatteryPack>,
    pub missions: Vec<MissionDemand>,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EligibilityStatus {
    Eligible,
    Blocked,
    Indeterminate,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PairEvaluation {
    pub mission_id: String,
    pub pack_id: String,
    pub status: EligibilityStatus,
    pub energy_margin_wh: Option<u32>,
    pub findings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PackAssignment {
    pub mission_id: String,
    pub pack_id: String,
    pub energy_margin_wh: u32,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct BatteryServiceNetworkReport {
    pub schema_version: String,
    pub benchmark_id: String,
    pub dispatch_epoch_id: String,
    pub pack_count: usize,
    pub mission_count: usize,
    pub search_nodes: usize,
    pub serviceable_mission_count: usize,
    pub assignments: Vec<PackAssignment>,
    pub unserved_missions: Vec<String>,
    pub unused_packs: Vec<String>,
    pub pair_evaluations: Vec<PairEvaluation>,
    pub status: String,
    pub all_pack_evidence_synthetic: bool,
    pub dispatch_authority_granted: bool,
    pub return_to_service_authority_granted: bool,
    pub claim_ceiling: &'static str,
}

#[derive(Debug, Clone)]
struct SearchBest {
    served: usize,
    total_energy_margin_wh: u64,
    selected_pack_indices: Vec<Option<usize>>,
}

fn add_finding(
    findings: &mut Vec<String>,
    status: &mut EligibilityStatus,
    next: EligibilityStatus,
    reason: &str,
) {
    findings.push(reason.to_string());
    if *status == EligibilityStatus::Blocked {
        return;
    }
    if next == EligibilityStatus::Blocked {
        *status = EligibilityStatus::Blocked;
    } else if next == EligibilityStatus::Indeterminate
        && *status == EligibilityStatus::Eligible
    {
        *status = EligibilityStatus::Indeterminate;
    }
}

fn evidence_is_fresh(
    freshness: EvidenceFreshness,
    evidence_class: EvidenceClass,
    reference: &str,
    findings: &mut Vec<String>,
    status: &mut EligibilityStatus,
    name: &str,
) -> bool {
    if freshness != EvidenceFreshness::Fresh {
        add_finding(
            findings,
            status,
            EligibilityStatus::Indeterminate,
            &format!("{name}_evidence_not_fresh"),
        );
        return false;
    }
    if evidence_class == EvidenceClass::Unknown {
        add_finding(
            findings,
            status,
            EligibilityStatus::Indeterminate,
            &format!("{name}_evidence_class_unknown"),
        );
        return false;
    }
    if reference.trim().is_empty() {
        add_finding(
            findings,
            status,
            EligibilityStatus::Indeterminate,
            &format!("{name}_evidence_reference_missing"),
        );
        return false;
    }
    true
}

fn evaluate_pair(mission: &MissionDemand, pack: &BatteryPack) -> PairEvaluation {
    let mut findings = Vec::new();
    let mut status = EligibilityStatus::Eligible;

    match pack.state {
        PackState::Available => {}
        PackState::Unknown => add_finding(
            &mut findings,
            &mut status,
            EligibilityStatus::Indeterminate,
            "pack_state_unknown",
        ),
        _ => add_finding(
            &mut findings,
            &mut status,
            EligibilityStatus::Blocked,
            "pack_not_available_for_dispatch_epoch",
        ),
    }

    match pack.identity_verified {
        Some(true) => {}
        Some(false) => add_finding(
            &mut findings,
            &mut status,
            EligibilityStatus::Blocked,
            "pack_identity_not_verified",
        ),
        None => add_finding(
            &mut findings,
            &mut status,
            EligibilityStatus::Indeterminate,
            "pack_identity_unknown",
        ),
    }
    match pack.custody_resolved {
        Some(true) => {}
        Some(false) => add_finding(
            &mut findings,
            &mut status,
            EligibilityStatus::Blocked,
            "pack_custody_unresolved",
        ),
        None => add_finding(
            &mut findings,
            &mut status,
            EligibilityStatus::Indeterminate,
            "pack_custody_unknown",
        ),
    }

    match pack.post_test_freshness {
        EvidenceFreshness::Fresh if !pack.post_test_evidence_reference.trim().is_empty() => {
            match pack.post_test_passed {
                Some(true) => {}
                Some(false) => add_finding(
                    &mut findings,
                    &mut status,
                    EligibilityStatus::Blocked,
                    "post_test_failed",
                ),
                None => add_finding(
                    &mut findings,
                    &mut status,
                    EligibilityStatus::Indeterminate,
                    "post_test_result_unknown",
                ),
            }
        }
        _ => add_finding(
            &mut findings,
            &mut status,
            EligibilityStatus::Indeterminate,
            "post_test_evidence_not_fresh_or_missing",
        ),
    }

    if pack.configuration_id != mission.configuration_id {
        add_finding(
            &mut findings,
            &mut status,
            EligibilityStatus::Blocked,
            "configuration_mismatch",
        );
    }

    let energy_fresh = evidence_is_fresh(
        pack.usable_energy.freshness,
        pack.usable_energy.evidence_class,
        &pack.usable_energy.evidence_reference,
        &mut findings,
        &mut status,
        "energy",
    );
    let power_fresh = evidence_is_fresh(
        pack.peak_power.freshness,
        pack.peak_power.evidence_class,
        &pack.peak_power.evidence_reference,
        &mut findings,
        &mut status,
        "peak_power",
    );
    let thermal_fresh = evidence_is_fresh(
        pack.thermal.freshness,
        pack.thermal.evidence_class,
        &pack.thermal.evidence_reference,
        &mut findings,
        &mut status,
        "thermal",
    );

    let energy_margin_wh = if energy_fresh {
        match pack.usable_energy.value {
            Some(value) if value >= mission.required_energy_with_reserve_wh => {
                Some(value - mission.required_energy_with_reserve_wh)
            }
            Some(_) => {
                add_finding(
                    &mut findings,
                    &mut status,
                    EligibilityStatus::Blocked,
                    "usable_energy_below_mission_plus_reserve",
                );
                None
            }
            None => {
                add_finding(
                    &mut findings,
                    &mut status,
                    EligibilityStatus::Indeterminate,
                    "usable_energy_value_missing",
                );
                None
            }
        }
    } else {
        None
    };

    if power_fresh {
        match pack.peak_power.value {
            Some(value) if value >= mission.required_peak_power_w => {}
            Some(_) => add_finding(
                &mut findings,
                &mut status,
                EligibilityStatus::Blocked,
                "peak_power_below_mission_requirement",
            ),
            None => add_finding(
                &mut findings,
                &mut status,
                EligibilityStatus::Indeterminate,
                "peak_power_value_missing",
            ),
        }
    }

    if thermal_fresh {
        match (
            pack.thermal.pack_temperature_c,
            pack.thermal.qualified_max_temperature_c,
        ) {
            (Some(current), Some(qualified_max))
                if current <= mission.maximum_pack_temperature_c && current <= qualified_max => {}
            (Some(_), Some(_)) => add_finding(
                &mut findings,
                &mut status,
                EligibilityStatus::Blocked,
                "thermal_envelope_exceeded",
            ),
            _ => add_finding(
                &mut findings,
                &mut status,
                EligibilityStatus::Indeterminate,
                "thermal_values_missing",
            ),
        }
    }

    findings.sort();
    findings.dedup();
    PairEvaluation {
        mission_id: mission.mission_id.clone(),
        pack_id: pack.pack_id.clone(),
        status,
        energy_margin_wh: if status == EligibilityStatus::Eligible {
            energy_margin_wh
        } else {
            None
        },
        findings,
    }
}

fn assignment_key(
    selected: &[Option<usize>],
    packs: &[BatteryPack],
) -> Vec<String> {
    selected
        .iter()
        .map(|index| match index {
            Some(index) => format!("0:{}", packs[*index].pack_id),
            None => "1:".to_string(),
        })
        .collect()
}

fn is_better(candidate: &SearchBest, current: &SearchBest, packs: &[BatteryPack]) -> bool {
    if candidate.served != current.served {
        return candidate.served > current.served;
    }
    if candidate.total_energy_margin_wh != current.total_energy_margin_wh {
        return candidate.total_energy_margin_wh < current.total_energy_margin_wh;
    }
    assignment_key(&candidate.selected_pack_indices, packs)
        < assignment_key(&current.selected_pack_indices, packs)
}

fn search_assignments(
    mission_index: usize,
    candidates: &[Vec<(usize, u32)>],
    packs: &[BatteryPack],
    selected: &mut Vec<Option<usize>>,
    used: &mut BTreeSet<usize>,
    served: usize,
    total_margin: u64,
    nodes: &mut usize,
    best: &mut Option<SearchBest>,
) {
    *nodes += 1;
    if mission_index == candidates.len() {
        let candidate = SearchBest {
            served,
            total_energy_margin_wh: total_margin,
            selected_pack_indices: selected.clone(),
        };
        if best
            .as_ref()
            .is_none_or(|current| is_better(&candidate, current, packs))
        {
            *best = Some(candidate);
        }
        return;
    }

    for (pack_index, energy_margin) in &candidates[mission_index] {
        if used.insert(*pack_index) {
            selected.push(Some(*pack_index));
            search_assignments(
                mission_index + 1,
                candidates,
                packs,
                selected,
                used,
                served + 1,
                total_margin + u64::from(*energy_margin),
                nodes,
                best,
            );
            selected.pop();
            used.remove(pack_index);
        }
    }

    selected.push(None);
    search_assignments(
        mission_index + 1,
        candidates,
        packs,
        selected,
        used,
        served,
        total_margin,
        nodes,
        best,
    );
    selected.pop();
}

pub fn evaluate_battery_service_network(
    input: &BatteryServiceNetworkInput,
) -> Result<BatteryServiceNetworkReport, Vec<String>> {
    let mut errors = Vec::new();
    if input.schema_version != "0.1.0"
        || input.benchmark_id != "BATTERY-SERVICE-NETWORK-001"
        || input.claim_ceiling.trim().is_empty()
        || input.dispatch_epoch_id.trim().is_empty()
    {
        errors.push("unsupported_identity_or_missing_dispatch_epoch".to_string());
    }
    if input.packs.is_empty()
        || input.missions.is_empty()
        || input.packs.len() > MAX_EXACT_ITEMS
        || input.missions.len() > MAX_EXACT_ITEMS
    {
        errors.push(format!("exact_v0_1_limit_is_1_to_{MAX_EXACT_ITEMS}_packs_and_missions"));
    }

    let mut pack_ids = BTreeSet::new();
    for pack in &input.packs {
        if pack.pack_id.trim().is_empty()
            || pack.configuration_id.trim().is_empty()
            || !pack_ids.insert(pack.pack_id.as_str())
        {
            errors.push("pack_identity_empty_or_duplicate".to_string());
        }
    }
    let mut mission_ids = BTreeSet::new();
    for mission in &input.missions {
        if mission.mission_id.trim().is_empty()
            || mission.configuration_id.trim().is_empty()
            || mission.required_energy_with_reserve_wh == 0
            || mission.required_peak_power_w == 0
            || !mission_ids.insert(mission.mission_id.as_str())
        {
            errors.push("mission_identity_or_requirement_invalid".to_string());
        }
    }
    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        return Err(errors);
    }

    let mut packs = input.packs.clone();
    packs.sort_by(|left, right| left.pack_id.cmp(&right.pack_id));
    let mut missions = input.missions.clone();
    missions.sort_by(|left, right| left.mission_id.cmp(&right.mission_id));

    let mut pair_evaluations = Vec::new();
    let mut candidates = Vec::new();
    for mission in &missions {
        let mut mission_candidates = Vec::new();
        for (pack_index, pack) in packs.iter().enumerate() {
            let evaluation = evaluate_pair(mission, pack);
            if evaluation.status == EligibilityStatus::Eligible {
                mission_candidates.push((
                    pack_index,
                    evaluation.energy_margin_wh.unwrap_or_default(),
                ));
            }
            pair_evaluations.push(evaluation);
        }
        mission_candidates.sort_by(|left, right| {
            left.1
                .cmp(&right.1)
                .then_with(|| packs[left.0].pack_id.cmp(&packs[right.0].pack_id))
        });
        candidates.push(mission_candidates);
    }

    let mut search_nodes = 0;
    let mut best = None;
    search_assignments(
        0,
        &candidates,
        &packs,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        0,
        0,
        &mut search_nodes,
        &mut best,
    );
    let best = best.expect("validated non-empty mission set always has an assignment");

    let mut assignments = Vec::new();
    let mut unserved_missions = Vec::new();
    let mut used_pack_indices = BTreeSet::new();
    for (mission_index, selected_pack) in best.selected_pack_indices.iter().enumerate() {
        match selected_pack {
            Some(pack_index) => {
                used_pack_indices.insert(*pack_index);
                let evaluation = pair_evaluations
                    .iter()
                    .find(|item| {
                        item.mission_id == missions[mission_index].mission_id
                            && item.pack_id == packs[*pack_index].pack_id
                    })
                    .expect("selected pair must have an evaluation");
                assignments.push(PackAssignment {
                    mission_id: missions[mission_index].mission_id.clone(),
                    pack_id: packs[*pack_index].pack_id.clone(),
                    energy_margin_wh: evaluation.energy_margin_wh.unwrap_or_default(),
                });
            }
            None => unserved_missions.push(missions[mission_index].mission_id.clone()),
        }
    }

    let unused_packs = packs
        .iter()
        .enumerate()
        .filter(|(index, _)| !used_pack_indices.contains(index))
        .map(|(_, pack)| pack.pack_id.clone())
        .collect();
    pair_evaluations.sort_by(|left, right| {
        left.mission_id
            .cmp(&right.mission_id)
            .then_with(|| left.pack_id.cmp(&right.pack_id))
    });

    let status = if best.served == missions.len() {
        "all_missions_assigned"
    } else if best.served > 0 {
        "partial_service"
    } else {
        "no_serviceable_missions"
    };

    Ok(BatteryServiceNetworkReport {
        schema_version: "0.1.0".to_string(),
        benchmark_id: "BATTERY-SERVICE-NETWORK-001".to_string(),
        dispatch_epoch_id: input.dispatch_epoch_id.clone(),
        pack_count: packs.len(),
        mission_count: missions.len(),
        search_nodes,
        serviceable_mission_count: best.served,
        assignments,
        unserved_missions,
        unused_packs,
        pair_evaluations,
        status: status.to_string(),
        all_pack_evidence_synthetic: packs.iter().all(|pack| {
            pack.usable_energy.evidence_class == EvidenceClass::Synthetic
                && pack.peak_power.evidence_class == EvidenceClass::Synthetic
                && pack.thermal.evidence_class == EvidenceClass::Synthetic
        }),
        dispatch_authority_granted: false,
        return_to_service_authority_granted: false,
        claim_ceiling: CLAIM_CEILING,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metric(value: Option<u32>, freshness: EvidenceFreshness) -> MetricEvidence {
        MetricEvidence {
            freshness,
            evidence_class: EvidenceClass::Synthetic,
            evidence_reference: "urn:synthetic:battery-network:test".to_string(),
            value,
        }
    }

    fn pack(
        id: &str,
        configuration_id: &str,
        energy_wh: Option<u32>,
        peak_power_w: Option<u32>,
    ) -> BatteryPack {
        BatteryPack {
            pack_id: id.to_string(),
            configuration_id: configuration_id.to_string(),
            state: PackState::Available,
            identity_verified: Some(true),
            custody_resolved: Some(true),
            post_test_freshness: EvidenceFreshness::Fresh,
            post_test_passed: Some(true),
            post_test_evidence_reference: "urn:synthetic:post-test".to_string(),
            usable_energy: metric(energy_wh, EvidenceFreshness::Fresh),
            peak_power: metric(peak_power_w, EvidenceFreshness::Fresh),
            thermal: ThermalEvidence {
                freshness: EvidenceFreshness::Fresh,
                evidence_class: EvidenceClass::Synthetic,
                evidence_reference: "urn:synthetic:thermal".to_string(),
                pack_temperature_c: Some(25),
                qualified_max_temperature_c: Some(60),
            },
        }
    }

    fn mission(id: &str, configuration_id: &str, energy_wh: u32, peak_power_w: u32) -> MissionDemand {
        MissionDemand {
            mission_id: id.to_string(),
            configuration_id: configuration_id.to_string(),
            required_energy_with_reserve_wh: energy_wh,
            required_peak_power_w: peak_power_w,
            maximum_pack_temperature_c: 50,
        }
    }

    fn input(packs: Vec<BatteryPack>, missions: Vec<MissionDemand>) -> BatteryServiceNetworkInput {
        BatteryServiceNetworkInput {
            schema_version: "0.1.0".to_string(),
            benchmark_id: "BATTERY-SERVICE-NETWORK-001".to_string(),
            dispatch_epoch_id: "epoch-1".to_string(),
            packs,
            missions,
            claim_ceiling: "synthetic exact allocation only".to_string(),
        }
    }

    fn pair_status(pack: &BatteryPack, mission: &MissionDemand) -> EligibilityStatus {
        evaluate_pair(mission, pack).status
    }

    #[test]
    fn energy_pass_does_not_override_peak_power_failure() {
        let battery = pack("p1", "uav-a", Some(2_000), Some(900));
        let demand = mission("m1", "uav-a", 1_000, 1_000);
        let result = evaluate_pair(&demand, &battery);
        assert_eq!(result.status, EligibilityStatus::Blocked);
        assert!(result.findings.iter().any(|finding| finding == "peak_power_below_mission_requirement"));
    }

    #[test]
    fn unknown_peak_power_is_indeterminate_not_a_pass() {
        let battery = pack("p1", "uav-a", Some(2_000), None);
        let demand = mission("m1", "uav-a", 1_000, 1_000);
        assert_eq!(pair_status(&battery, &demand), EligibilityStatus::Indeterminate);
    }

    #[test]
    fn stale_energy_evidence_is_indeterminate() {
        let mut battery = pack("p1", "uav-a", Some(2_000), Some(2_000));
        battery.usable_energy.freshness = EvidenceFreshness::Stale;
        let demand = mission("m1", "uav-a", 1_000, 1_000);
        assert_eq!(pair_status(&battery, &demand), EligibilityStatus::Indeterminate);
    }

    #[test]
    fn configuration_mismatch_is_blocked() {
        let battery = pack("p1", "uav-a", Some(2_000), Some(2_000));
        let demand = mission("m1", "uav-b", 1_000, 1_000);
        assert_eq!(pair_status(&battery, &demand), EligibilityStatus::Blocked);
    }

    #[test]
    fn quarantined_pack_is_never_allocated() {
        let mut battery = pack("p1", "uav-a", Some(2_000), Some(2_000));
        battery.state = PackState::Quarantined;
        let demand = mission("m1", "uav-a", 1_000, 1_000);
        assert_eq!(pair_status(&battery, &demand), EligibilityStatus::Blocked);
    }

    #[test]
    fn unresolved_custody_is_blocked() {
        let mut battery = pack("p1", "uav-a", Some(2_000), Some(2_000));
        battery.custody_resolved = Some(false);
        let demand = mission("m1", "uav-a", 1_000, 1_000);
        assert_eq!(pair_status(&battery, &demand), EligibilityStatus::Blocked);
    }

    #[test]
    fn exact_assignment_never_double_books_a_pack() {
        let packs = vec![
            pack("pack-a", "uav-a", Some(2_000), Some(2_000)),
            pack("pack-b", "uav-a", Some(1_500), Some(1_500)),
            pack("pack-c", "uav-b", Some(1_200), Some(1_800)),
        ];
        let missions = vec![
            mission("mission-1", "uav-a", 1_700, 1_800),
            mission("mission-2", "uav-a", 1_200, 1_200),
            mission("mission-3", "uav-b", 1_000, 1_500),
            mission("mission-4", "uav-a", 900, 1_000),
        ];
        let report = evaluate_battery_service_network(&input(packs, missions)).unwrap();
        assert_eq!(report.serviceable_mission_count, 3);
        assert_eq!(report.assignments.len(), 3);
        assert_eq!(report.unserved_missions, vec!["mission-4"]);
        let assigned_pack_ids: BTreeSet<_> = report
            .assignments
            .iter()
            .map(|assignment| assignment.pack_id.as_str())
            .collect();
        assert_eq!(assigned_pack_ids.len(), report.assignments.len());
        assert!(!report.dispatch_authority_granted);
        assert!(!report.return_to_service_authority_granted);
    }

    #[test]
    fn result_is_permutation_invariant() {
        let packs = vec![
            pack("pack-b", "uav-a", Some(1_500), Some(1_500)),
            pack("pack-a", "uav-a", Some(2_000), Some(2_000)),
        ];
        let missions = vec![
            mission("mission-b", "uav-a", 1_200, 1_200),
            mission("mission-a", "uav-a", 1_700, 1_800),
        ];
        let first = evaluate_battery_service_network(&input(packs.clone(), missions.clone())).unwrap();
        let mut reversed_packs = packs;
        reversed_packs.reverse();
        let mut reversed_missions = missions;
        reversed_missions.reverse();
        let second = evaluate_battery_service_network(&input(reversed_packs, reversed_missions)).unwrap();
        assert_eq!(first.assignments, second.assignments);
        assert_eq!(first.unserved_missions, second.unserved_missions);
    }

    #[test]
    fn unknown_identity_is_indeterminate_not_eligible() {
        let mut battery = pack("p1", "uav-a", Some(2_000), Some(2_000));
        battery.identity_verified = None;
        let demand = mission("m1", "uav-a", 1_000, 1_000);
        assert_eq!(pair_status(&battery, &demand), EligibilityStatus::Indeterminate);
    }

    #[test]
    fn over_limit_exact_instance_is_rejected() {
        let packs = (0..7)
            .map(|index| pack(&format!("p{index}"), "uav-a", Some(2_000), Some(2_000)))
            .collect();
        let missions = (0..7)
            .map(|index| mission(&format!("m{index}"), "uav-a", 1_000, 1_000))
            .collect();
        assert!(evaluate_battery_service_network(&input(packs, missions)).is_err());
    }
}
