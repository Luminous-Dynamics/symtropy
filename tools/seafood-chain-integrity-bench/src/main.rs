// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

const SCHEMA_ID: &str = "luminous.symtropy.seafood-chain-integrity";
const SCHEMA_VERSION: &str = "0.1.0";
const SUITE_ID: &str = "SEAFOOD-CHAIN-INTEGRITY-SYNTH-001";
const DEFAULT_CORPUS: &str = "docs/tech/SEAFOOD_CHAIN_INTEGRITY_CORPUS_V0_1.json";
const MAX_CORPUS_BYTES: usize = 250_000;

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Corpus {
    schema_version: String,
    suite_id: String,
    scenarios: Vec<Scenario>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Scenario {
    id: String,
    assessment_minute: u64,
    target_lot_id: String,
    lots: Vec<Lot>,
    events: Vec<TrackingEvent>,
    cold_chain: ColdChain,
    raw_ready_to_eat: bool,
    controls: ReleaseControls,
    label: LabelEvidence,
    sustainability_claims: Vec<SustainabilityClaim>,
    expected_disposition: ExpectedDisposition,
    expected_unsubstantiated_claims: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Lot {
    lot_id: String,
    species_latin: String,
    source_kind: SourceKind,
    source_reference: String,
    source_evidence_status: EvidenceStatus,
    mass_g: u64,
    parent_inputs: Vec<ParentInput>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ParentInput {
    lot_id: String,
    mass_g: u64,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum SourceKind {
    WildCapture,
    Aquaculture,
    Derived,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum EvidenceStatus {
    Verified,
    Missing,
    Unverified,
    Expired,
    NotApplicable,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct TrackingEvent {
    event_id: String,
    lot_id: String,
    kind: EventKind,
    at_minute: u64,
    actor_id: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum EventKind {
    Harvest,
    Transform,
    Pack,
    Ship,
    Receive,
    Serve,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ColdChain {
    sensor_calibration_status: EvidenceStatus,
    max_temp_milli_c: i32,
    max_gap_minutes: u32,
    monitoring_start_minute: u64,
    monitoring_end_minute: u64,
    observations: Vec<TemperatureObservation>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct TemperatureObservation {
    at_minute: u64,
    temp_milli_c: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ReleaseControls {
    haccp_review_status: EvidenceStatus,
    parasite_control_status: EvidenceStatus,
    parasite_control_basis: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct LabelEvidence {
    identity_status: EvidenceStatus,
    declared_species_latin: String,
    verified_species_latin: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct SustainabilityClaim {
    claim_id: String,
    claim_text: String,
    evidence_reference: String,
    evidence_status: EvidenceStatus,
    scope_lot_id: String,
    valid_through_minute: u64,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ExpectedDisposition {
    Hold,
    EligibleForHumanReview,
}

#[derive(Debug, Clone, Serialize)]
struct ScenarioReceipt {
    scenario_id: String,
    disposition: ExpectedDisposition,
    hold_reasons: Vec<String>,
    claim_assessments: Vec<ClaimAssessment>,
    expected_results_match: bool,
}

#[derive(Debug, Clone, Serialize)]
struct ClaimAssessment {
    claim_id: String,
    evidence_supported_for_declared_scope: bool,
    reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct Receipt {
    schema_id: String,
    schema_version: String,
    suite_id: String,
    corpus_sha256: String,
    source_commit: String,
    scenario_count: usize,
    hold_count: usize,
    eligible_for_human_review_count: usize,
    unsubstantiated_claim_count: usize,
    all_expected_results_match: bool,
    scenarios: Vec<ScenarioReceipt>,
}

fn main() -> ExitCode {
    match run_cli() {
        Ok(all_expected_match) if all_expected_match => ExitCode::SUCCESS,
        Ok(_) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("seafood-chain-integrity-bench: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run_cli() -> Result<bool, String> {
    let mut corpus_path = PathBuf::from(DEFAULT_CORPUS);
    let mut receipt_path = PathBuf::from("target/seafood-chain-integrity-receipt.json");
    let mut source_commit = String::from("unbound-local-run");
    let mut args = env::args().skip(1);

    while let Some(arg) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {arg}"))?;
        match arg.as_str() {
            "--corpus" => corpus_path = PathBuf::from(value),
            "--receipt" => receipt_path = PathBuf::from(value),
            "--source-commit" => source_commit = value,
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }

    let bytes = fs::read(&corpus_path)
        .map_err(|error| format!("cannot read {}: {error}", corpus_path.display()))?;
    if bytes.len() > MAX_CORPUS_BYTES {
        return Err(format!("corpus exceeds {MAX_CORPUS_BYTES} bytes"));
    }
    let corpus: Corpus = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid JSON corpus: {error}"))?;
    validate_corpus_header(&corpus)?;
    let receipt = evaluate_corpus(&corpus, sha256_hex(&bytes), source_commit);

    if let Some(parent) = receipt_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create receipt directory: {error}"))?;
    }
    let output = serde_json::to_vec_pretty(&receipt)
        .map_err(|error| format!("cannot serialize receipt: {error}"))?;
    fs::write(&receipt_path, output)
        .map_err(|error| format!("cannot write {}: {error}", receipt_path.display()))?;

    println!(
        "suite={} scenarios={} hold={} eligible_for_human_review={} unsupported_claims={} expected_results_match={}",
        receipt.suite_id,
        receipt.scenario_count,
        receipt.hold_count,
        receipt.eligible_for_human_review_count,
        receipt.unsubstantiated_claim_count,
        receipt.all_expected_results_match
    );
    println!("receipt={}", receipt_path.display());
    Ok(receipt.all_expected_results_match)
}

fn validate_corpus_header(corpus: &Corpus) -> Result<(), String> {
    if corpus.schema_version != SCHEMA_VERSION {
        return Err(format!("unsupported schema_version: {}", corpus.schema_version));
    }
    if corpus.suite_id != SUITE_ID {
        return Err(format!("unexpected suite_id: {}", corpus.suite_id));
    }
    if corpus.scenarios.is_empty() {
        return Err("corpus must contain at least one scenario".into());
    }
    let mut ids = BTreeSet::new();
    for scenario in &corpus.scenarios {
        if scenario.id.trim().is_empty() || !ids.insert(scenario.id.clone()) {
            return Err("scenario IDs must be non-empty and unique".into());
        }
    }
    Ok(())
}

fn evaluate_corpus(corpus: &Corpus, corpus_sha256: String, source_commit: String) -> Receipt {
    let scenarios: Vec<ScenarioReceipt> = corpus.scenarios.iter().map(evaluate_scenario).collect();
    let hold_count = scenarios
        .iter()
        .filter(|scenario| scenario.disposition == ExpectedDisposition::Hold)
        .count();
    let eligible_for_human_review_count = scenarios.len() - hold_count;
    let unsubstantiated_claim_count = scenarios
        .iter()
        .flat_map(|scenario| &scenario.claim_assessments)
        .filter(|claim| !claim.evidence_supported_for_declared_scope)
        .count();
    let all_expected_results_match = scenarios.iter().all(|scenario| scenario.expected_results_match);

    Receipt {
        schema_id: SCHEMA_ID.into(),
        schema_version: SCHEMA_VERSION.into(),
        suite_id: corpus.suite_id.clone(),
        corpus_sha256,
        source_commit,
        scenario_count: scenarios.len(),
        hold_count,
        eligible_for_human_review_count,
        unsubstantiated_claim_count,
        all_expected_results_match,
        scenarios,
    }
}

fn evaluate_scenario(scenario: &Scenario) -> ScenarioReceipt {
    let mut reasons = BTreeSet::<String>::new();
    let mut lots = BTreeMap::<String, &Lot>::new();
    for lot in &scenario.lots {
        if lots.insert(lot.lot_id.clone(), lot).is_some() {
            reasons.insert("duplicate_lot_id".into());
        }
        if lot.lot_id.trim().is_empty() || lot.species_latin.trim().is_empty() || lot.mass_g == 0 {
            reasons.insert("invalid_lot_identity_or_mass".into());
        }
    }

    for lot in &scenario.lots {
        match lot.source_kind {
            SourceKind::WildCapture | SourceKind::Aquaculture => {
                if !lot.parent_inputs.is_empty() {
                    reasons.insert("source_lot_has_parents".into());
                }
                if lot.source_reference.trim().is_empty() {
                    reasons.insert("source_reference_missing".into());
                }
                if lot.source_evidence_status != EvidenceStatus::Verified {
                    reasons.insert("legal_source_evidence_not_verified".into());
                }
            }
            SourceKind::Derived => {
                if lot.parent_inputs.is_empty() {
                    reasons.insert("derived_lot_missing_parents".into());
                }
                if lot.source_evidence_status != EvidenceStatus::NotApplicable {
                    reasons.insert("derived_lot_source_status_must_be_not_applicable".into());
                }
                if lot.parent_inputs.iter().any(|parent| parent.mass_g == 0) {
                    reasons.insert("zero_parent_input_mass".into());
                }
                let input_total = lot.parent_inputs.iter().try_fold(0_u64, |total, parent| {
                    total.checked_add(parent.mass_g)
                });
                match input_total {
                    Some(total) if total < lot.mass_g => {
                        reasons.insert("transformation_output_exceeds_declared_inputs".into());
                    }
                    None => {
                        reasons.insert("parent_input_mass_overflow".into());
                    }
                    _ => {}
                }
                for parent in &lot.parent_inputs {
                    if !lots.contains_key(&parent.lot_id) {
                        reasons.insert("unknown_parent_lot".into());
                        continue;
                    }
                    if let Some(parent_lot) = lots.get(&parent.lot_id) {
                        if parent_lot.species_latin != lot.species_latin {
                            reasons.insert("species_changes_without_explicit_identity_event".into());
                        }
                    }
                }
            }
        }
    }

    // Prevent a source lot's mass from being allocated to multiple children more
    // than once. ParentInput records the actual mass taken from each parent.
    let mut parent_usage = BTreeMap::<String, u64>::new();
    for lot in &scenario.lots {
        for parent in &lot.parent_inputs {
            let entry = parent_usage.entry(parent.lot_id.clone()).or_default();
            match entry.checked_add(parent.mass_g) {
                Some(total) => *entry = total,
                None => {
                    reasons.insert("parent_usage_mass_overflow".into());
                }
            }
        }
    }
    for (parent_id, usage) in parent_usage {
        match lots.get(&parent_id) {
            Some(parent) if usage > parent.mass_g => {
                reasons.insert("parent_mass_overallocated_across_children".into());
            }
            None => {
                reasons.insert("unknown_parent_lot".into());
            }
            _ => {}
        }
    }

    let mut visiting = BTreeSet::<String>::new();
    let mut visited = BTreeSet::<String>::new();
    for lot in &scenario.lots {
        detect_cycle(
            &lot.lot_id,
            &lots,
            &mut visiting,
            &mut visited,
            &mut reasons,
        );
    }
    let target_exists = lots.contains_key(&scenario.target_lot_id);
    if !target_exists {
        reasons.insert("target_lot_missing".into());
    }

    let mut events_by_lot = BTreeMap::<String, Vec<&TrackingEvent>>::new();
    let mut event_ids = BTreeSet::<String>::new();
    for event in &scenario.events {
        if event.event_id.trim().is_empty() || !event_ids.insert(event.event_id.clone()) {
            reasons.insert("duplicate_or_empty_event_id".into());
        }
        if event.actor_id.trim().is_empty() {
            reasons.insert("event_actor_missing".into());
        }
        if !lots.contains_key(&event.lot_id) {
            reasons.insert("event_references_unknown_lot".into());
        }
        events_by_lot.entry(event.lot_id.clone()).or_default().push(event);
    }
    for (lot_id, events) in &events_by_lot {
        if events.windows(2).any(|pair| pair[1].at_minute < pair[0].at_minute) {
            reasons.insert("event_time_regression".into());
        }
        if let Some(lot) = lots.get(lot_id) {
            let has_harvest = events.iter().any(|event| event.kind == EventKind::Harvest);
            let has_transform = events.iter().any(|event| event.kind == EventKind::Transform);
            if matches!(lot.source_kind, SourceKind::WildCapture | SourceKind::Aquaculture)
                && !has_harvest
            {
                reasons.insert("source_lot_missing_harvest_event".into());
            }
            if lot.source_kind == SourceKind::Derived && !has_transform {
                reasons.insert("derived_lot_missing_transform_event".into());
            }
        }
    }
    if !events_by_lot
        .get(&scenario.target_lot_id)
        .is_some_and(|events| events.iter().any(|event| event.kind == EventKind::Serve))
    {
        reasons.insert("target_lot_missing_serve_event".into());
    }

    let lineage = ancestor_lots(&scenario.target_lot_id, &lots);
    validate_cold_chain(scenario, &lineage, &events_by_lot, &mut reasons);

    if scenario.controls.haccp_review_status != EvidenceStatus::Verified {
        reasons.insert("hazard_review_evidence_not_verified".into());
    }
    if scenario.raw_ready_to_eat {
        match scenario.controls.parasite_control_status {
            EvidenceStatus::Verified => {}
            EvidenceStatus::NotApplicable
                if !scenario.controls.parasite_control_basis.trim().is_empty() => {}
            EvidenceStatus::NotApplicable => {
                reasons.insert("parasite_control_not_applicable_without_basis".into());
            }
            _ => {
                reasons.insert("raw_rte_parasite_control_not_verified".into());
            }
        }
    }
    if scenario.label.identity_status != EvidenceStatus::Verified {
        reasons.insert("species_identity_evidence_not_verified".into());
    }
    if scenario.label.declared_species_latin.trim().is_empty()
        || scenario.label.verified_species_latin.trim().is_empty()
        || scenario.label.declared_species_latin != scenario.label.verified_species_latin
    {
        reasons.insert("declared_and_verified_species_mismatch".into());
    }
    if let Some(target) = lots.get(&scenario.target_lot_id) {
        if scenario.label.verified_species_latin != target.species_latin {
            reasons.insert("verified_species_does_not_match_lot".into());
        }
    }

    let claim_assessments: Vec<ClaimAssessment> = scenario
        .sustainability_claims
        .iter()
        .map(|claim| assess_claim(claim, scenario.assessment_minute, &lineage))
        .collect();
    let unsubstantiated_claims = claim_assessments
        .iter()
        .filter(|claim| !claim.evidence_supported_for_declared_scope)
        .count();
    let disposition = if reasons.is_empty() {
        ExpectedDisposition::EligibleForHumanReview
    } else {
        ExpectedDisposition::Hold
    };
    let expected_results_match = disposition == scenario.expected_disposition
        && unsubstantiated_claims == scenario.expected_unsubstantiated_claims;

    ScenarioReceipt {
        scenario_id: scenario.id.clone(),
        disposition,
        hold_reasons: reasons.into_iter().collect(),
        claim_assessments,
        expected_results_match,
    }
}

fn detect_cycle(
    lot_id: &str,
    lots: &BTreeMap<String, &Lot>,
    visiting: &mut BTreeSet<String>,
    visited: &mut BTreeSet<String>,
    reasons: &mut BTreeSet<String>,
) {
    if visited.contains(lot_id) {
        return;
    }
    if !visiting.insert(lot_id.to_owned()) {
        reasons.insert("lineage_cycle".into());
        return;
    }
    if let Some(lot) = lots.get(lot_id) {
        for parent in &lot.parent_inputs {
            if lots.contains_key(&parent.lot_id) {
                detect_cycle(&parent.lot_id, lots, visiting, visited, reasons);
            }
        }
    }
    visiting.remove(lot_id);
    visited.insert(lot_id.to_owned());
}

fn ancestor_lots(target_lot_id: &str, lots: &BTreeMap<String, &Lot>) -> BTreeSet<String> {
    let mut ancestors = BTreeSet::<String>::new();
    let mut pending = vec![target_lot_id.to_owned()];
    while let Some(lot_id) = pending.pop() {
        if !ancestors.insert(lot_id.clone()) {
            continue;
        }
        if let Some(lot) = lots.get(&lot_id) {
            pending.extend(lot.parent_inputs.iter().map(|parent| parent.lot_id.clone()));
        }
    }
    ancestors
}

fn validate_cold_chain(
    scenario: &Scenario,
    lineage: &BTreeSet<String>,
    events_by_lot: &BTreeMap<String, Vec<&TrackingEvent>>,
    reasons: &mut BTreeSet<String>,
) {
    let cold = &scenario.cold_chain;
    if cold.sensor_calibration_status != EvidenceStatus::Verified {
        reasons.insert("temperature_sensor_calibration_not_verified".into());
    }
    if cold.max_gap_minutes == 0 || cold.monitoring_end_minute < cold.monitoring_start_minute {
        reasons.insert("invalid_cold_chain_policy_window".into());
    }
    if cold.observations.is_empty() {
        reasons.insert("cold_chain_observations_missing".into());
        return;
    }
    if cold.observations.windows(2).any(|pair| pair[1].at_minute <= pair[0].at_minute) {
        reasons.insert("temperature_observation_times_not_strictly_increasing".into());
    }
    if cold.observations.iter().any(|observation| observation.temp_milli_c > cold.max_temp_milli_c) {
        reasons.insert("temperature_exceeds_declared_control_limit".into());
    }
    if cold
        .observations
        .windows(2)
        .any(|pair| pair[1].at_minute.saturating_sub(pair[0].at_minute) > u64::from(cold.max_gap_minutes))
    {
        reasons.insert("temperature_monitoring_gap_exceeds_declared_limit".into());
    }

    let mut event_times = Vec::<u64>::new();
    for lot_id in lineage {
        if let Some(events) = events_by_lot.get(lot_id) {
            event_times.extend(events.iter().map(|event| event.at_minute));
        }
    }
    if let (Some(first_event), Some(last_event)) = (
        event_times.iter().min(),
        event_times.iter().max(),
    ) {
        if cold.monitoring_start_minute > *first_event
            || cold.observations.first().is_some_and(|obs| obs.at_minute > *first_event)
        {
            reasons.insert("cold_chain_monitoring_starts_after_first_lineage_event".into());
        }
        if cold.monitoring_end_minute < *last_event
            || cold.observations.last().is_some_and(|obs| obs.at_minute < *last_event)
        {
            reasons.insert("cold_chain_monitoring_ends_before_last_lineage_event".into());
        }
    } else {
        reasons.insert("lineage_has_no_tracking_events".into());
    }
    if cold.observations.first().is_some_and(|obs| obs.at_minute > cold.monitoring_start_minute)
        || cold.observations.last().is_some_and(|obs| obs.at_minute < cold.monitoring_end_minute)
    {
        reasons.insert("temperature_samples_do_not_cover_declared_window".into());
    }
}

fn assess_claim(
    claim: &SustainabilityClaim,
    assessment_minute: u64,
    target_lineage: &BTreeSet<String>,
) -> ClaimAssessment {
    let mut reasons = BTreeSet::<String>::new();
    if claim.claim_id.trim().is_empty() || claim.claim_text.trim().is_empty() {
        reasons.insert("claim_identity_or_text_missing".into());
    }
    if claim.evidence_reference.trim().is_empty() {
        reasons.insert("claim_evidence_reference_missing".into());
    }
    if claim.evidence_status != EvidenceStatus::Verified {
        reasons.insert("claim_evidence_not_verified".into());
    }
    if !target_lineage.contains(&claim.scope_lot_id) {
        reasons.insert("claim_scope_does_not_cover_target_lineage".into());
    }
    if claim.valid_through_minute < assessment_minute {
        reasons.insert("claim_evidence_expired".into());
    }
    ClaimAssessment {
        claim_id: claim.claim_id.clone(),
        evidence_supported_for_declared_scope: reasons.is_empty(),
        reasons: reasons.into_iter().collect(),
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus() -> Corpus {
        let parsed: Corpus = serde_json::from_str(include_str!(
            "../../../docs/tech/SEAFOOD_CHAIN_INTEGRITY_CORPUS_V0_1.json"
        ))
        .expect("frozen corpus parses");
        validate_corpus_header(&parsed).expect("frozen corpus header is valid");
        parsed
    }

    fn evaluate_one(scenario: &Scenario) -> ScenarioReceipt {
        let mut scenario = scenario.clone();
        scenario.expected_disposition = ExpectedDisposition::Hold;
        scenario.expected_unsubstantiated_claims = usize::MAX;
        evaluate_scenario(&scenario)
    }

    #[test]
    fn frozen_scenarios_match_expected_decisions_and_claim_counts() {
        let corpus = corpus();
        let receipt = evaluate_corpus(&corpus, "synthetic-test-digest".into(), "test-sha".into());
        assert!(receipt.all_expected_results_match);
        assert_eq!(receipt.scenario_count, 6);
        assert_eq!(receipt.hold_count, 4);
        assert_eq!(receipt.eligible_for_human_review_count, 2);
        assert_eq!(receipt.unsubstantiated_claim_count, 1);
    }

    #[test]
    fn unsupported_sustainability_claim_does_not_become_a_food_safety_hold() {
        let corpus = corpus();
        let scenario = corpus
            .scenarios
            .iter()
            .find(|scenario| scenario.id == "unverified-sustainability-claim")
            .expect("claim scenario exists");
        let result = evaluate_scenario(scenario);
        assert_eq!(result.disposition, ExpectedDisposition::EligibleForHumanReview);
        assert!(!result.claim_assessments[0].evidence_supported_for_declared_scope);
    }

    #[test]
    fn duplicate_lot_ids_fail_closed() {
        let mut scenario = corpus().scenarios.remove(0);
        let duplicate = scenario.lots[0].clone();
        scenario.lots.push(duplicate);
        let result = evaluate_one(&scenario);
        assert_eq!(result.disposition, ExpectedDisposition::Hold);
        assert!(result.hold_reasons.iter().any(|reason| reason == "duplicate_lot_id"));
    }

    #[test]
    fn parent_mass_cannot_be_allocated_twice_across_child_lots() {
        let mut scenario = corpus().scenarios.remove(0);
        let mut extra_child = scenario.lots[1].clone();
        extra_child.lot_id = "extra-child-lot".into();
        extra_child.mass_g = 9_000;
        extra_child.parent_inputs[0].mass_g = 9_000;
        scenario.lots.push(extra_child);
        let result = evaluate_one(&scenario);
        assert!(result
            .hold_reasons
            .iter()
            .any(|reason| reason == "parent_mass_overallocated_across_children"));
    }

    #[test]
    fn unknown_parent_lot_is_not_accepted_as_provenance() {
        let mut scenario = corpus().scenarios.remove(0);
        scenario.lots[1].parent_inputs[0].lot_id = "missing-parent".into();
        let result = evaluate_one(&scenario);
        assert!(result.hold_reasons.iter().any(|reason| reason == "unknown_parent_lot"));
    }

    #[test]
    fn raw_ready_to_eat_non_applicability_needs_a_documented_basis() {
        let mut scenario = corpus().scenarios.remove(0);
        scenario.controls.parasite_control_status = EvidenceStatus::NotApplicable;
        scenario.controls.parasite_control_basis.clear();
        let result = evaluate_one(&scenario);
        assert!(result
            .hold_reasons
            .iter()
            .any(|reason| reason == "parasite_control_not_applicable_without_basis"));
    }

    #[test]
    fn cold_chain_excursion_is_not_averaged_away() {
        let mut scenario = corpus().scenarios.remove(0);
        scenario.cold_chain.observations[1].temp_milli_c = 5_000;
        let result = evaluate_one(&scenario);
        assert!(result
            .hold_reasons
            .iter()
            .any(|reason| reason == "temperature_exceeds_declared_control_limit"));
    }

    #[test]
    fn cyclic_lot_lineage_is_rejected() {
        let mut scenario = corpus().scenarios.remove(0);
        scenario.lots[0].source_kind = SourceKind::Derived;
        scenario.lots[0].source_evidence_status = EvidenceStatus::NotApplicable;
        let child_lot_id = scenario.lots[1].lot_id.clone();
        scenario.lots[0].parent_inputs.push(ParentInput {
            lot_id: child_lot_id,
            mass_g: 1_000,
        });
        let result = evaluate_one(&scenario);
        assert!(result.hold_reasons.iter().any(|reason| reason == "lineage_cycle"));
    }
}
