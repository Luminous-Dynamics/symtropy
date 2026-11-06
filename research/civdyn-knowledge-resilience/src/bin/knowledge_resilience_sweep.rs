// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Synthetic knowledge transmission and archive-resilience trace.

use civdyn_knowledge_resilience::{
    ArchiveRecovery, KnowledgeDataset, archive_recovery_after_failures, evaluate,
    reference_dataset,
};
use std::collections::BTreeSet;

const CSV_COLUMNS: usize = 21;

fn emit(fields: Vec<String>) {
    assert_eq!(
        fields.len(),
        CSV_COLUMNS,
        "knowledge-resilience CSV schema must remain fixed"
    );
    println!("{}", fields.join(","));
}

fn empty_row(record_type: &str, item_id: &str) -> Vec<String> {
    vec![
        record_type.into(),
        item_id.into(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
    ]
}

fn item_row(
    dataset: &KnowledgeDataset,
    item: &civdyn_knowledge_resilience::ItemMetrics,
) {
    let mut row = empty_row("item_summary", &item.item_id);
    row[2] = item.epistemic_status.name().into();
    row[3] = item.semantic_family_id.clone();
    row[4] = item.source_evidence_family_count.to_string();
    row[5] = item.transmission_events.to_string();
    row[6] = item.delivered_transmissions.to_string();
    row[7] = item.acquisition_count.to_string();
    row[8] = item.comprehension_count.to_string();
    row[9] = item.demonstration_assessed_count.to_string();
    row[10] = item.demonstration_success_count.to_string();
    row[11] = item.retention_assessed_count.to_string();
    row[12] = item.retained_count.to_string();
    row[13] = item.independent_teacher_count.to_string();
    row[14] = item.refuted_knowledge_acquisition_count.to_string();
    row[15] = item.distinct_demonstrated_generations.to_string();
    row[16] = item.longest_consecutive_demonstrated_generations.to_string();
    row[17] = item.verified_archive_copy_count.to_string();
    row[18] = item.independent_archive_failure_domain_count.to_string();
    let _ = dataset;
    emit(row);
}

fn archive_row(item_id: &str, recovery_label: &str, recovery: &ArchiveRecovery) {
    let mut row = empty_row("archive_recovery", item_id);
    row[2] = recovery_label.into();
    row[19] = recovery.surviving_verified_copy_count.to_string();
    row[20] = recovery.recoverable.to_string();
    emit(row);
}

fn main() {
    println!(
        "record_type,item_id,epistemic_status,semantic_family_id,source_evidence_family_count,transmission_events,delivered_transmissions,acquisition_count,comprehension_count,demonstration_assessed_count,demonstration_success_count,retention_assessed_count,retained_count,independent_teacher_count,refuted_knowledge_acquisition_count,demonstrated_generation_count,longest_consecutive_demonstrated_generations,verified_archive_copy_count,independent_archive_failure_domain_count,recovery_surviving_verified_copy_count,recoverable"
    );

    let dataset = reference_dataset();
    let evaluation = evaluate(&dataset).expect("synthetic knowledge trace must validate");
    for item in &evaluation.items {
        item_row(&dataset, item);
    }

    for event in &dataset.transmissions {
        let mut row = empty_row("transmission_event", &event.item_id);
        row[2] = event.method.clone();
        row[3] = format!("{}->{}", event.sender_id, event.receiver_id);
        row[6] = event.delivered.to_string();
        emit(row);
    }

    let current = archive_recovery_after_failures(&dataset, &BTreeSet::new());
    let failed_primary = archive_recovery_after_failures(
        &dataset,
        &BTreeSet::from(["archive_domain_a".to_owned()]),
    );
    for recovery in &current {
        archive_row(&recovery.item_id, "no_domain_failure", recovery);
    }
    for recovery in &failed_primary {
        archive_row(&recovery.item_id, "archive_domain_a_failed", recovery);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_trace_has_two_items_five_transmissions_and_six_carriers() {
        let dataset = reference_dataset();
        let evaluation = evaluate(&dataset).unwrap();
        assert_eq!(evaluation.items.len(), 2);
        assert_eq!(dataset.transmissions.len(), 5);
        assert_eq!(dataset.carriers.len(), 6);
    }

    #[test]
    fn archive_loss_is_recoverable_for_verified_redundant_knowledge_only() {
        let dataset = reference_dataset();
        let recovery = archive_recovery_after_failures(
            &dataset,
            &BTreeSet::from(["archive_domain_a".to_owned()]),
        );
        let hydro = recovery.iter().find(|item| item.item_id == "hydrology_skill").unwrap();
        let myth = recovery.iter().find(|item| item.item_id == "silt_myth").unwrap();
        assert!(hydro.recoverable);
        assert!(!myth.recoverable);
    }
}
