// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civdyn_self_correction::sa_treasury_intake::{
    assess_fact_evidence_grade, classify_amount_value, classify_requested_fact,
    validate_treasury_snapshot, AmountTypeClass, AuditOpinionStatus, FactEvidenceGrade,
    FactValueClass, RequestedFactState, SnapshotVerification, TreasuryFact,
    TreasuryFactKey, TreasuryPageReceipt, TreasurySchemaGeneration, TreasurySnapshot,
    TreasurySnapshotReceipt,
};

const CSV_COLUMNS: usize = 14;

fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn emit(fields: Vec<String>) {
    assert_eq!(
        fields.len(),
        CSV_COLUMNS,
        "Treasury intake receipt must match the frozen schema"
    );
    println!(
        "{}",
        fields
            .iter()
            .map(|field| csv_field(field))
            .collect::<Vec<_>>()
            .join(",")
    );
}

fn receipt(
    verification: SnapshotVerification,
    amount_type_class: AmountTypeClass,
    expected_total: u64,
) -> TreasurySnapshotReceipt {
    TreasurySnapshotReceipt {
        dataset_name: "repmaint_v2".to_string(),
        schema_generation: TreasurySchemaGeneration::MscoaV2,
        source_url: "https://municipaldata.treasury.gov.za/api/cubes/repmaint_v2/facts".to_string(),
        request_digest: "sha256-synthetic-request".to_string(),
        response_digest: "sha256-synthetic-response".to_string(),
        response_bytes: 2048,
        transformation_digest: "sha256-synthetic-transform".to_string(),
        retrieved_at: "2026-11-10T12:00:00+02:00".to_string(),
        portal_snapshot_id: "synthetic-treasury-2026-q2".to_string(),
        portal_publication_state: verification,
        terms_version: "municipal-money-terms-v1".to_string(),
        terms_acknowledged: true,
        amount_type_code: "ACT".to_string(),
        amount_type_label: "Actual".to_string(),
        amount_type_class,
        amount_type_mapping_digest: "sha256-synthetic-amount-type-map".to_string(),
        audit_opinion_source_digest: Some("sha256-synthetic-audit-opinion-join".to_string()),
        expected_total_fact_count: expected_total,
    }
}

fn fact(
    item_code: &str,
    amount: Option<i64>,
    audit_opinion: AuditOpinionStatus,
) -> TreasuryFact {
    TreasuryFact {
        key: TreasuryFactKey {
            dataset_name: "repmaint_v2".to_string(),
            demarcation_code: "JHB".to_string(),
            financial_year_end: 2025,
            financial_period: "01".to_string(),
            period_length: "month".to_string(),
            amount_type_code: "ACT".to_string(),
            item_code: item_code.to_string(),
        },
        municipality_name: "City of Johannesburg".to_string(),
        item_label: format!("Synthetic maintenance item {item_code}"),
        amount_zar: amount,
        audit_opinion,
    }
}

fn snapshot(
    verification: SnapshotVerification,
    facts: Vec<TreasuryFact>,
    page_digest: &str,
) -> TreasurySnapshot {
    let count = facts.len() as u64;
    TreasurySnapshot {
        receipt: receipt(verification, AmountTypeClass::Actual, count),
        pages: vec![TreasuryPageReceipt {
            page_number: 1,
            page_digest: page_digest.to_string(),
            returned_fact_count: count,
        }],
        facts,
    }
}

fn emit_fact(
    row_kind: &str,
    fact: &TreasuryFact,
    source: &TreasurySnapshot,
) {
    let value_class = classify_amount_value(fact.amount_zar);
    let grade = assess_fact_evidence_grade(fact, &source.receipt);
    emit(vec![
        row_kind.to_string(),
        source.receipt.dataset_name.clone(),
        source.receipt.schema_generation.name().to_string(),
        fact.key.demarcation_code.clone(),
        fact.key.financial_year_end.to_string(),
        fact.key.financial_period.clone(),
        fact.key.item_code.clone(),
        fact.key.amount_type_code.clone(),
        fact.amount_zar.map(|value| value.to_string()).unwrap_or_default(),
        value_class.name().to_string(),
        grade.name().to_string(),
        fact.audit_opinion.name().to_string(),
        source.receipt.portal_publication_state.name().to_string(),
        source.receipt.terms_acknowledged.to_string(),
    ]);
}

fn emit_not_returned(source: &TreasurySnapshot, key: &TreasuryFactKey) {
    let state = classify_requested_fact(source, key);
    assert_eq!(state, RequestedFactState::FactNotReturned);
    emit(vec![
        "query_result".to_string(),
        key.dataset_name.clone(),
        source.receipt.schema_generation.name().to_string(),
        key.demarcation_code.clone(),
        key.financial_year_end.to_string(),
        key.financial_period.clone(),
        key.item_code.clone(),
        key.amount_type_code.clone(),
        String::new(),
        "fact_not_returned".to_string(),
        "not_present".to_string(),
        AuditOpinionStatus::Unknown.name().to_string(),
        source.receipt.portal_publication_state.name().to_string(),
        source.receipt.terms_acknowledged.to_string(),
    ]);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "row_kind,dataset_name,schema_generation,demarcation_code,financial_year_end,financial_period,item_code,amount_type_code,raw_amount_zar,value_class,evidence_grade,audit_opinion,snapshot_verification,terms_acknowledged"
    );

    let verified = snapshot(
        SnapshotVerification::Verified,
        vec![
            fact("6001", Some(120_000), AuditOpinionStatus::UnqualifiedNoFindings),
            fact("6002", Some(0), AuditOpinionStatus::Qualified),
            fact("6003", None, AuditOpinionStatus::Unknown),
        ],
        "sha256-verified-page",
    );
    let verified_report = validate_treasury_snapshot(&verified)?;
    assert_eq!(verified_report.nonzero_reported, 1);
    assert_eq!(verified_report.zero_or_unfilled_ambiguous, 1);
    assert_eq!(verified_report.null_or_unfilled_ambiguous, 1);
    assert_eq!(verified_report.confirmatory_eligible, 1);

    for row in &verified.facts {
        emit_fact("fact", row, &verified);
    }

    let provisional = snapshot(
        SnapshotVerification::CompletenessOrIntegrityPending,
        vec![fact("6001", Some(250_000), AuditOpinionStatus::Unknown)],
        "sha256-provisional-page",
    );
    let provisional_report = validate_treasury_snapshot(&provisional)?;
    assert_eq!(provisional_report.confirmatory_eligible, 0);
    emit_fact("fact", &provisional.facts[0], &provisional);

    let absent_key = TreasuryFactKey {
        dataset_name: "repmaint_v2".to_string(),
        demarcation_code: "JHB".to_string(),
        financial_year_end: 2025,
        financial_period: "01".to_string(),
        period_length: "month".to_string(),
        amount_type_code: "ACT".to_string(),
        item_code: "6004".to_string(),
    };
    emit_not_returned(&verified, &absent_key);

    // Keep the type imported in the executable so a future status extension
    // cannot silently change the current value-class receipt contract.
    let _: FactValueClass = FactValueClass::NonZeroReported;
    let _: FactEvidenceGrade = FactEvidenceGrade::EligibleForRegisteredAnalysis;
    Ok(())
}
