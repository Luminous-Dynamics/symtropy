// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Source-specific intake semantics for South African National Treasury
//! municipal finance facts.
//!
//! This module preserves value ambiguity, amount-type separation, audit state,
//! schema generations and pagination lineage. It deliberately does not fetch,
//! hash or endorse external source data. A caller must supply the exact response
//! digest and source metadata in the intake receipt.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreasurySchemaGeneration {
    PreMscoaV1,
    MscoaV2,
}

impl TreasurySchemaGeneration {
    pub const fn dataset_name(self) -> &'static str {
        match self {
            Self::PreMscoaV1 => "repmaint",
            Self::MscoaV2 => "repmaint_v2",
        }
    }

    pub const fn item_code_is_valid(self, code: &str) -> bool {
        match self {
            Self::PreMscoaV1 => matches!(code, "5001" | "5002" | "5003" | "5004" | "5005"),
            Self::MscoaV2 => matches!(code, "6001" | "6002" | "6003" | "6004"),
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::PreMscoaV1 => "pre_mscoa_v1",
            Self::MscoaV2 => "mscoa_v2",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotVerification {
    Verified,
    CompletenessOrIntegrityPending,
    VerificationUnknown,
}

impl SnapshotVerification {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Verified => "verified",
            Self::CompletenessOrIntegrityPending => "completeness_or_integrity_pending",
            Self::VerificationUnknown => "verification_unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmountTypeClass {
    AuditedActual,
    Actual,
    PreAuditedActual,
    Budget,
    Unknown,
}

impl AmountTypeClass {
    pub const fn name(self) -> &'static str {
        match self {
            Self::AuditedActual => "audited_actual",
            Self::Actual => "actual",
            Self::PreAuditedActual => "pre_audited_actual",
            Self::Budget => "budget",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditOpinionStatus {
    UnqualifiedNoFindings,
    UnqualifiedWithEmphasis,
    Qualified,
    Adverse,
    Disclaimer,
    Unknown,
}

impl AuditOpinionStatus {
    pub const fn name(self) -> &'static str {
        match self {
            Self::UnqualifiedNoFindings => "unqualified_no_findings",
            Self::UnqualifiedWithEmphasis => "unqualified_with_emphasis",
            Self::Qualified => "qualified",
            Self::Adverse => "adverse",
            Self::Disclaimer => "disclaimer",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactValueClass {
    NonZeroReported,
    ZeroOrUnfilledAmbiguous,
    NullOrUnfilledAmbiguous,
}

impl FactValueClass {
    pub const fn name(self) -> &'static str {
        match self {
            Self::NonZeroReported => "nonzero_reported",
            Self::ZeroOrUnfilledAmbiguous => "zero_or_unfilled_ambiguous",
            Self::NullOrUnfilledAmbiguous => "null_or_unfilled_ambiguous",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactEvidenceGrade {
    EligibleForRegisteredAnalysis,
    AmbiguousValue,
    SnapshotNotVerified,
    AmountTypeUnknown,
    AuditedActualWithoutAuditOpinion,
}

impl FactEvidenceGrade {
    pub const fn name(self) -> &'static str {
        match self {
            Self::EligibleForRegisteredAnalysis => "eligible_for_registered_analysis",
            Self::AmbiguousValue => "ambiguous_value",
            Self::SnapshotNotVerified => "snapshot_not_verified",
            Self::AmountTypeUnknown => "amount_type_unknown",
            Self::AuditedActualWithoutAuditOpinion => "audited_actual_without_audit_opinion",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreasurySnapshotReceipt {
    pub dataset_name: String,
    pub schema_generation: TreasurySchemaGeneration,
    pub source_url: String,
    pub request_digest: String,
    pub response_digest: String,
    pub response_bytes: u64,
    pub transformation_digest: String,
    pub retrieved_at: String,
    pub portal_snapshot_id: String,
    pub portal_publication_state: SnapshotVerification,
    pub terms_version: String,
    pub terms_acknowledged: bool,
    pub amount_type_code: String,
    pub amount_type_label: String,
    pub amount_type_class: AmountTypeClass,
    pub audit_opinion_source_digest: Option<String>,
    pub expected_total_fact_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreasuryPageReceipt {
    pub page_number: u32,
    pub page_digest: String,
    pub returned_fact_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TreasuryFactKey {
    pub dataset_name: String,
    pub demarcation_code: String,
    pub financial_year_end: u16,
    /// Kept as a string so source codes such as "01" retain leading zeroes.
    pub financial_period: String,
    pub period_length: String,
    pub amount_type_code: String,
    pub item_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreasuryFact {
    pub key: TreasuryFactKey,
    pub municipality_name: String,
    pub item_label: String,
    /// Integer South African Rand. None is a JSON null, not a numeric zero.
    pub amount_zar: Option<i64>,
    /// Joined from the separate municipality/year audit-opinions source.
    pub audit_opinion: AuditOpinionStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreasurySnapshot {
    pub receipt: TreasurySnapshotReceipt,
    pub pages: Vec<TreasuryPageReceipt>,
    pub facts: Vec<TreasuryFact>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestedFactState {
    Returned(FactValueClass),
    FactNotReturned,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreasuryIntakeSummary {
    pub dataset_name: String,
    pub schema_generation: TreasurySchemaGeneration,
    pub total_facts: usize,
    pub nonzero_reported: usize,
    pub zero_or_unfilled_ambiguous: usize,
    pub null_or_unfilled_ambiguous: usize,
    pub confirmatory_eligible: usize,
    pub not_confirmatory_eligible: usize,
    pub audit_opinion_unknown: usize,
    pub audit_opinion_qualified_or_worse: usize,
    pub snapshot_verification: SnapshotVerification,
    pub amount_type_code: String,
    pub amount_type_class: AmountTypeClass,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreasuryIntakeError {
    MissingReceiptIdentity,
    TermsNotAcknowledged,
    InvalidSchemaDatasetPair,
    InvalidFinancialYearForSchema { year_end: u16, schema: TreasurySchemaGeneration },
    MissingPaginationReceipt,
    InvalidPageSequence,
    DuplicatePageNumber { page_number: u32 },
    DuplicatePageDigest { digest: String },
    PageCountMismatch { expected: u64, received: u64 },
    FactCountMismatch { expected: u64, received: usize },
    DuplicateFactKey { key: TreasuryFactKey },
    WrongDatasetInFact { key: TreasuryFactKey },
    WrongAmountTypeInFact { key: TreasuryFactKey },
    InvalidFactIdentity { key: TreasuryFactKey },
    InvalidItemCode { key: TreasuryFactKey },
    AuditedActualMissingOpinionDigest,
    OpinionWithoutSourceDigest,
}

impl fmt::Display for TreasuryIntakeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for TreasuryIntakeError {}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

pub fn classify_amount_value(amount: Option<i64>) -> FactValueClass {
    match amount {
        Some(value) if value != 0 => FactValueClass::NonZeroReported,
        Some(_) => FactValueClass::ZeroOrUnfilledAmbiguous,
        None => FactValueClass::NullOrUnfilledAmbiguous,
    }
}

pub fn assess_fact_evidence_grade(
    fact: &TreasuryFact,
    receipt: &TreasurySnapshotReceipt,
) -> FactEvidenceGrade {
    if receipt.portal_publication_state != SnapshotVerification::Verified {
        return FactEvidenceGrade::SnapshotNotVerified;
    }
    if classify_amount_value(fact.amount_zar) != FactValueClass::NonZeroReported {
        return FactEvidenceGrade::AmbiguousValue;
    }
    if receipt.amount_type_class == AmountTypeClass::Unknown {
        return FactEvidenceGrade::AmountTypeUnknown;
    }
    if receipt.amount_type_class == AmountTypeClass::AuditedActual
        && fact.audit_opinion == AuditOpinionStatus::Unknown
    {
        return FactEvidenceGrade::AuditedActualWithoutAuditOpinion;
    }
    FactEvidenceGrade::EligibleForRegisteredAnalysis
}

fn validate_receipt(receipt: &TreasurySnapshotReceipt) -> Result<(), TreasuryIntakeError> {
    if !nonempty(&receipt.dataset_name)
        || !nonempty(&receipt.source_url)
        || !nonempty(&receipt.request_digest)
        || !nonempty(&receipt.response_digest)
        || !nonempty(&receipt.transformation_digest)
        || !nonempty(&receipt.retrieved_at)
        || !nonempty(&receipt.portal_snapshot_id)
        || !nonempty(&receipt.terms_version)
        || !nonempty(&receipt.amount_type_code)
        || !nonempty(&receipt.amount_type_label)
        || receipt.response_bytes == 0
        || receipt.expected_total_fact_count == 0
    {
        return Err(TreasuryIntakeError::MissingReceiptIdentity);
    }
    if !receipt.terms_acknowledged {
        return Err(TreasuryIntakeError::TermsNotAcknowledged);
    }
    if receipt.dataset_name != receipt.schema_generation.dataset_name() {
        return Err(TreasuryIntakeError::InvalidSchemaDatasetPair);
    }
    if receipt.amount_type_class == AmountTypeClass::AuditedActual
        && receipt.audit_opinion_source_digest.as_deref().is_none_or(|value| !nonempty(value))
    {
        return Err(TreasuryIntakeError::AuditedActualMissingOpinionDigest);
    }
    if receipt.amount_type_class != AmountTypeClass::AuditedActual
        && receipt.audit_opinion_source_digest.as_deref().is_some_and(|value| !nonempty(value))
    {
        return Err(TreasuryIntakeError::OpinionWithoutSourceDigest);
    }
    Ok(())
}

fn validate_pagination(
    receipt: &TreasurySnapshotReceipt,
    pages: &[TreasuryPageReceipt],
    facts: &[TreasuryFact],
) -> Result<(), TreasuryIntakeError> {
    if pages.is_empty() {
        return Err(TreasuryIntakeError::MissingPaginationReceipt);
    }
    let mut seen_numbers = BTreeSet::new();
    let mut seen_digests = BTreeSet::new();
    let mut total = 0_u64;
    let mut ordered = pages.to_vec();
    ordered.sort_by_key(|page| page.page_number);

    for (index, page) in ordered.iter().enumerate() {
        let expected_number = u32::try_from(index + 1).unwrap_or(u32::MAX);
        if page.page_number != expected_number || page.page_number == 0 {
            return Err(TreasuryIntakeError::InvalidPageSequence);
        }
        if !seen_numbers.insert(page.page_number) {
            return Err(TreasuryIntakeError::DuplicatePageNumber {
                page_number: page.page_number,
            });
        }
        if !nonempty(&page.page_digest) {
            return Err(TreasuryIntakeError::MissingReceiptIdentity);
        }
        if !seen_digests.insert(page.page_digest.as_str()) {
            return Err(TreasuryIntakeError::DuplicatePageDigest {
                digest: page.page_digest.clone(),
            });
        }
        total = total
            .checked_add(page.returned_fact_count)
            .ok_or(TreasuryIntakeError::PageCountMismatch {
                expected: receipt.expected_total_fact_count,
                received: u64::MAX,
            })?;
    }

    if total != receipt.expected_total_fact_count {
        return Err(TreasuryIntakeError::PageCountMismatch {
            expected: receipt.expected_total_fact_count,
            received: total,
        });
    }
    if facts.len() as u64 != receipt.expected_total_fact_count {
        return Err(TreasuryIntakeError::FactCountMismatch {
            expected: receipt.expected_total_fact_count,
            received: facts.len(),
        });
    }
    Ok(())
}

/// Validate a source snapshot without converting ambiguous values to zero.
/// Returns per-fact eligibility counts and keeps audit/release dimensions
/// separate from the reported Rands amount.
pub fn validate_treasury_snapshot(
    snapshot: &TreasurySnapshot,
) -> Result<TreasuryIntakeSummary, TreasuryIntakeError> {
    validate_receipt(&snapshot.receipt)?;
    validate_pagination(&snapshot.receipt, &snapshot.pages, &snapshot.facts)?;

    let mut seen_keys = BTreeSet::new();
    let mut nonzero_reported = 0;
    let mut zero_or_unfilled_ambiguous = 0;
    let mut null_or_unfilled_ambiguous = 0;
    let mut confirmatory_eligible = 0;
    let mut not_confirmatory_eligible = 0;
    let mut audit_opinion_unknown = 0;
    let mut audit_opinion_qualified_or_worse = 0;

    for fact in &snapshot.facts {
        let key = &fact.key;
        if !seen_keys.insert(key.clone()) {
            return Err(TreasuryIntakeError::DuplicateFactKey { key: key.clone() });
        }
        if key.dataset_name != snapshot.receipt.dataset_name {
            return Err(TreasuryIntakeError::WrongDatasetInFact { key: key.clone() });
        }
        if key.amount_type_code != snapshot.receipt.amount_type_code {
            return Err(TreasuryIntakeError::WrongAmountTypeInFact { key: key.clone() });
        }
        if !nonempty(&key.demarcation_code)
            || !nonempty(&fact.municipality_name)
            || !nonempty(&key.financial_period)
            || !nonempty(&key.period_length)
            || !nonempty(&fact.item_label)
            || key.financial_year_end == 0
        {
            return Err(TreasuryIntakeError::InvalidFactIdentity { key: key.clone() });
        }
        let year_valid = match snapshot.receipt.schema_generation {
            TreasurySchemaGeneration::PreMscoaV1 => (2010..=2019).contains(&key.financial_year_end),
            TreasurySchemaGeneration::MscoaV2 => key.financial_year_end >= 2020,
        };
        if !year_valid {
            return Err(TreasuryIntakeError::InvalidFinancialYearForSchema {
                year_end: key.financial_year_end,
                schema: snapshot.receipt.schema_generation,
            });
        }
        if !snapshot.receipt.schema_generation.item_code_is_valid(&key.item_code) {
            return Err(TreasuryIntakeError::InvalidItemCode { key: key.clone() });
        }

        match fact_value_class(fact.amount_zar) {
            FactValueClass::NonZeroReported => nonzero_reported += 1,
            FactValueClass::ZeroOrUnfilledAmbiguous => zero_or_unfilled_ambiguous += 1,
            FactValueClass::NullOrUnfilledAmbiguous => null_or_unfilled_ambiguous += 1,
        }
        match assess_fact_evidence_grade(fact, &snapshot.receipt) {
            FactEvidenceGrade::EligibleForRegisteredAnalysis => confirmatory_eligible += 1,
            _ => not_confirmatory_eligible += 1,
        }
        if fact.audit_opinion == AuditOpinionStatus::Unknown {
            audit_opinion_unknown += 1;
        }
        if matches!(
            fact.audit_opinion,
            AuditOpinionStatus::Qualified
                | AuditOpinionStatus::Adverse
                | AuditOpinionStatus::Disclaimer
        ) {
            audit_opinion_qualified_or_worse += 1;
        }
    }

    Ok(TreasuryIntakeSummary {
        dataset_name: snapshot.receipt.dataset_name.clone(),
        schema_generation: snapshot.receipt.schema_generation,
        total_facts: snapshot.facts.len(),
        nonzero_reported,
        zero_or_unfilled_ambiguous,
        null_or_unfilled_ambiguous,
        confirmatory_eligible,
        not_confirmatory_eligible,
        audit_opinion_unknown,
        audit_opinion_qualified_or_worse,
        snapshot_verification: snapshot.receipt.portal_publication_state,
        amount_type_code: snapshot.receipt.amount_type_code.clone(),
        amount_type_class: snapshot.receipt.amount_type_class,
    })
}

/// Classify a requested key. A row omitted from an acceptable API response is
/// FactNotReturned, never an imputed zero. This helper does not prove that the
/// query itself was complete; validate its pagination receipt first.
pub fn classify_requested_fact(
    snapshot: &TreasurySnapshot,
    requested_key: &TreasuryFactKey,
) -> RequestedFactState {
    match snapshot.facts.iter().find(|fact| &fact.key == requested_key) {
        Some(fact) => RequestedFactState::Returned(fact_value_class(fact.amount_zar)),
        None => RequestedFactState::FactNotReturned,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn receipt(
        generation: TreasurySchemaGeneration,
        verification: SnapshotVerification,
        amount_type_class: AmountTypeClass,
        expected: u64,
    ) -> TreasurySnapshotReceipt {
        TreasurySnapshotReceipt {
            dataset_name: generation.dataset_name().to_string(),
            schema_generation: generation,
            source_url: "https://municipaldata.treasury.gov.za/api/cubes/repmaint_v2/facts".to_string(),
            request_digest: "sha256-request".to_string(),
            response_digest: "sha256-response".to_string(),
            response_bytes: 512,
            transformation_digest: "sha256-transform".to_string(),
            retrieved_at: "2026-11-10T10:00:00+02:00".to_string(),
            portal_snapshot_id: "treasury-2026-q2".to_string(),
            portal_publication_state: verification,
            terms_version: "municipal-money-terms-v1".to_string(),
            terms_acknowledged: true,
            amount_type_code: "AUD".to_string(),
            amount_type_label: "Audited Actual".to_string(),
            amount_type_class,
            audit_opinion_source_digest: if amount_type_class == AmountTypeClass::AuditedActual {
                Some("sha256-audit-opinions".to_string())
            } else {
                None
            },
            expected_total_fact_count: expected,
        }
    }

    fn fact(
        generation: TreasurySchemaGeneration,
        year_end: u16,
        item_code: &str,
        amount: Option<i64>,
        amount_type_code: &str,
        audit: AuditOpinionStatus,
    ) -> TreasuryFact {
        TreasuryFact {
            key: TreasuryFactKey {
                dataset_name: generation.dataset_name().to_string(),
                demarcation_code: "JHB".to_string(),
                financial_year_end: year_end,
                financial_period: "01".to_string(),
                period_length: "month".to_string(),
                amount_type_code: amount_type_code.to_string(),
                item_code: item_code.to_string(),
            },
            municipality_name: "City of Johannesburg".to_string(),
            item_label: "Employee Related Costs".to_string(),
            amount_zar: amount,
            audit_opinion: audit,
        }
    }

    fn snapshot(
        verification: SnapshotVerification,
        amount_type_class: AmountTypeClass,
        facts: Vec<TreasuryFact>,
    ) -> TreasurySnapshot {
        let count = facts.len() as u64;
        TreasurySnapshot {
            receipt: receipt(
                TreasurySchemaGeneration::MscoaV2,
                verification,
                amount_type_class,
                count,
            ),
            pages: vec![TreasuryPageReceipt {
                page_number: 1,
                page_digest: "sha256-page-1".to_string(),
                returned_fact_count: count,
            }],
            facts,
        }
    }

    #[test]
    fn distinguishes_nonzero_zero_null_and_not_returned() {
        let facts = vec![
            fact(TreasurySchemaGeneration::MscoaV2, 2025, "6001", Some(1200), "AUD", AuditOpinionStatus::UnqualifiedNoFindings),
            fact(TreasurySchemaGeneration::MscoaV2, 2025, "6002", Some(0), "AUD", AuditOpinionStatus::UnqualifiedNoFindings),
            fact(TreasurySchemaGeneration::MscoaV2, 2025, "6003", None, "AUD", AuditOpinionStatus::UnqualifiedNoFindings),
        ];
        let candidate = snapshot(SnapshotVerification::Verified, AmountTypeClass::AuditedActual, facts.clone());
        assert_eq!(fact_value_class(facts[0].amount_zar), FactValueClass::NonZeroReported);
        assert_eq!(fact_value_class(facts[1].amount_zar), FactValueClass::ZeroOrUnfilledAmbiguous);
        assert_eq!(fact_value_class(facts[2].amount_zar), FactValueClass::NullOrUnfilledAmbiguous);

        let missing = fact(TreasurySchemaGeneration::MscoaV2, 2025, "6004", Some(500), "AUD", AuditOpinionStatus::UnqualifiedNoFindings);
        assert_eq!(
            classify_requested_fact(&candidate, &missing.key),
            RequestedFactState::FactNotReturned
        );
    }

    #[test]
    fn ambiguous_zero_and_null_never_count_as_confirmatory_observations() {
        let facts = vec![
            fact(TreasurySchemaGeneration::MscoaV2, 2025, "6001", Some(100), "AUD", AuditOpinionStatus::UnqualifiedNoFindings),
            fact(TreasurySchemaGeneration::MscoaV2, 2025, "6002", Some(0), "AUD", AuditOpinionStatus::UnqualifiedNoFindings),
            fact(TreasurySchemaGeneration::MscoaV2, 2025, "6003", None, "AUD", AuditOpinionStatus::UnqualifiedNoFindings),
        ];
        let report = validate_treasury_snapshot(&snapshot(
            SnapshotVerification::Verified,
            AmountTypeClass::AuditedActual,
            facts,
        )).expect("valid source snapshot");
        assert_eq!(report.nonzero_reported, 1);
        assert_eq!(report.zero_or_unfilled_ambiguous, 1);
        assert_eq!(report.null_or_unfilled_ambiguous, 1);
        assert_eq!(report.confirmatory_eligible, 1);
        assert_eq!(report.not_confirmatory_eligible, 2);
    }

    #[test]
    fn provisional_portal_snapshot_blocks_confirmatory_eligibility() {
        let report = validate_treasury_snapshot(&snapshot(
            SnapshotVerification::CompletenessOrIntegrityPending,
            AmountTypeClass::AuditedActual,
            vec![fact(TreasurySchemaGeneration::MscoaV2, 2025, "6001", Some(100), "AUD", AuditOpinionStatus::UnqualifiedNoFindings)],
        )).expect("provisional data can be preserved");
        assert_eq!(report.confirmatory_eligible, 0);
        assert_eq!(report.not_confirmatory_eligible, 1);
        assert_eq!(report.snapshot_verification, SnapshotVerification::CompletenessOrIntegrityPending);
    }

    #[test]
    fn v1_and_v2_schema_generations_are_not_interchangeable() {
        let mut candidate = snapshot(
            SnapshotVerification::Verified,
            AmountTypeClass::AuditedActual,
            vec![fact(TreasurySchemaGeneration::MscoaV2, 2025, "6001", Some(100), "AUD", AuditOpinionStatus::UnqualifiedNoFindings)],
        );
        candidate.receipt.dataset_name = "repmaint".to_string();
        assert_eq!(
            validate_treasury_snapshot(&candidate).unwrap_err(),
            TreasuryIntakeError::InvalidSchemaDatasetPair
        );

        let v1 = TreasurySnapshot {
            receipt: receipt(TreasurySchemaGeneration::PreMscoaV1, SnapshotVerification::Verified, AmountTypeClass::Actual, 1),
            pages: vec![TreasuryPageReceipt { page_number: 1, page_digest: "sha256-v1".to_string(), returned_fact_count: 1 }],
            facts: vec![fact(TreasurySchemaGeneration::PreMscoaV1, 2019, "5005", Some(100), "AUD", AuditOpinionStatus::Unknown)],
        };
        assert!(validate_treasury_snapshot(&v1).is_ok());
    }

    #[test]
    fn schema_boundary_and_item_code_mismatches_fail_closed() {
        let bad_year = TreasurySnapshot {
            receipt: receipt(TreasurySchemaGeneration::PreMscoaV1, SnapshotVerification::Verified, AmountTypeClass::Actual, 1),
            pages: vec![TreasuryPageReceipt { page_number: 1, page_digest: "sha256-page".to_string(), returned_fact_count: 1 }],
            facts: vec![fact(TreasurySchemaGeneration::PreMscoaV1, 2020, "5005", Some(1), "AUD", AuditOpinionStatus::Unknown)],
        };
        assert!(matches!(
            validate_treasury_snapshot(&bad_year),
            Err(TreasuryIntakeError::InvalidFinancialYearForSchema { .. })
        ));

        let bad_code = snapshot(
            SnapshotVerification::Verified,
            AmountTypeClass::AuditedActual,
            vec![fact(TreasurySchemaGeneration::MscoaV2, 2025, "5005", Some(1), "AUD", AuditOpinionStatus::UnqualifiedNoFindings)],
        );
        assert!(matches!(
            validate_treasury_snapshot(&bad_code),
            Err(TreasuryIntakeError::InvalidItemCode { .. })
        ));
    }

    #[test]
    fn audited_actual_requires_separate_audit_opinion_source_identity() {
        let mut candidate = snapshot(
            SnapshotVerification::Verified,
            AmountTypeClass::AuditedActual,
            vec![fact(TreasurySchemaGeneration::MscoaV2, 2025, "6001", Some(100), "AUD", AuditOpinionStatus::Unknown)],
        );
        candidate.receipt.audit_opinion_source_digest = None;
        assert_eq!(
            validate_treasury_snapshot(&candidate).unwrap_err(),
            TreasuryIntakeError::AuditedActualMissingOpinionDigest
        );
    }

    #[test]
    fn audit_unknown_and_qualified_or_worse_are_counted_separately() {
        let report = validate_treasury_snapshot(&snapshot(
            SnapshotVerification::Verified,
            AmountTypeClass::Actual,
            vec![
                fact(TreasurySchemaGeneration::MscoaV2, 2025, "6001", Some(100), "AUD", AuditOpinionStatus::Unknown),
                fact(TreasurySchemaGeneration::MscoaV2, 2025, "6002", Some(200), "AUD", AuditOpinionStatus::Qualified),
            ],
        )).expect("valid source snapshot");
        assert_eq!(report.audit_opinion_unknown, 1);
        assert_eq!(report.audit_opinion_qualified_or_worse, 1);
        assert_eq!(report.confirmatory_eligible, 2);
    }

    #[test]
    fn amount_type_is_a_key_and_budget_cannot_be_substituted() {
        let mut candidate = snapshot(
            SnapshotVerification::Verified,
            AmountTypeClass::Budget,
            vec![fact(TreasurySchemaGeneration::MscoaV2, 2025, "6001", Some(100), "ACT", AuditOpinionStatus::Unknown)],
        );
        candidate.receipt.amount_type_code = "BUD".to_string();
        assert!(matches!(
            validate_treasury_snapshot(&candidate),
            Err(TreasuryIntakeError::WrongAmountTypeInFact { .. })
        ));
    }

    #[test]
    fn pagination_and_duplicate_fact_checks_are_strict() {
        let facts = vec![
            fact(TreasurySchemaGeneration::MscoaV2, 2025, "6001", Some(100), "AUD", AuditOpinionStatus::UnqualifiedNoFindings),
            fact(TreasurySchemaGeneration::MscoaV2, 2025, "6002", Some(200), "AUD", AuditOpinionStatus::UnqualifiedNoFindings),
        ];
        let mut candidate = snapshot(SnapshotVerification::Verified, AmountTypeClass::AuditedActual, facts.clone());
        candidate.pages = vec![
            TreasuryPageReceipt { page_number: 1, page_digest: "sha256-page-1".to_string(), returned_fact_count: 1 },
            TreasuryPageReceipt { page_number: 3, page_digest: "sha256-page-3".to_string(), returned_fact_count: 1 },
        ];
        assert_eq!(
            validate_treasury_snapshot(&candidate).unwrap_err(),
            TreasuryIntakeError::InvalidPageSequence
        );

        let mut duplicate = snapshot(
            SnapshotVerification::Verified,
            AmountTypeClass::AuditedActual,
            vec![facts[0].clone(), facts[0].clone()],
        );
        duplicate.pages[0].returned_fact_count = 2;
        duplicate.receipt.expected_total_fact_count = 2;
        assert!(matches!(
            validate_treasury_snapshot(&duplicate),
            Err(TreasuryIntakeError::DuplicateFactKey { .. })
        ));
    }

    #[test]
    fn terms_and_source_digests_are_required() {
        let mut candidate = snapshot(
            SnapshotVerification::Verified,
            AmountTypeClass::Actual,
            vec![fact(TreasurySchemaGeneration::MscoaV2, 2025, "6001", Some(100), "ACT", AuditOpinionStatus::Unknown)],
        );
        candidate.receipt.terms_acknowledged = false;
        assert_eq!(
            validate_treasury_snapshot(&candidate).unwrap_err(),
            TreasuryIntakeError::TermsNotAcknowledged
        );
        candidate.receipt.terms_acknowledged = true;
        candidate.receipt.response_digest.clear();
        assert_eq!(
            validate_treasury_snapshot(&candidate).unwrap_err(),
            TreasuryIntakeError::MissingReceiptIdentity
        );
    }
}
