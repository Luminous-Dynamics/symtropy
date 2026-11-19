// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Source-bound industry evidence ledger.
//!
//! This crate preserves the scope and evidence class of public claims. It does
//! not rank operators, infer unreported values, or treat company claims as
//! independent measurements.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u16 = 1;
pub const MAX_RECORDS: usize = 256;
pub const MAX_CLAIMS: usize = 2_048;
pub const MAX_COMPARISON_GROUPS: usize = 256;

const CLAIM_CEILING: &str = "Source-qualified public evidence ledger only. This is not an independent audit, operational dataset, manufacturer specification verification, legal approval, cost estimate, safety rate, fleet recommendation or claim of superiority.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceClass {
    OfficialPublicAgency,
    OfficialOperatorDescription,
    CompanyReportedOperationalClaim,
    ManufacturerSpecification,
    PeerReviewedStudy,
    IndependentMeasurement,
    Synthetic,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Claim {
    pub claim_id: String,
    /// Stable metric identity. Present only for numeric claims.
    pub metric_id: Option<String>,
    /// Statistic definition: cumulative_to_date, average, minimum, annual_output,
    /// share, relative_improvement, or another explicitly documented definition.
    pub statistic: Option<String>,
    pub numeric_value: Option<f64>,
    pub text_value: Option<String>,
    pub unit: Option<String>,
    pub geography: String,
    /// Exact observation or study period; not inferred from the publication date.
    pub period: String,
    /// Product/network/configuration to which the claim applies.
    pub configuration: String,
    /// Population, denominator or scope boundary where known.
    pub population: String,
    /// Limits, exclusions, self-reporting status or comparability warning.
    pub limitation: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceRecord {
    pub record_id: String,
    pub title: String,
    pub publisher: String,
    pub source_url: String,
    pub published_on: String,
    /// Date on which the source or operator says the fact was current.
    pub observed_as_of: String,
    pub evidence_class: EvidenceClass,
    pub scope: String,
    pub claims: Vec<Claim>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComparisonGroup {
    pub group_id: String,
    pub metric_id: String,
    pub statistic: String,
    pub unit: String,
    pub geography: String,
    pub period: String,
    pub claim_ids: Vec<String>,
    /// Explicit statement of why these values are comparable.
    pub comparability_basis: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceLedger {
    pub schema_version: u16,
    pub benchmark_id: String,
    pub snapshot_date: String,
    pub records: Vec<EvidenceRecord>,
    pub comparison_groups: Vec<ComparisonGroup>,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ValidationSummary {
    pub schema_version: u16,
    pub benchmark_id: String,
    pub snapshot_date: String,
    pub evidence_record_count: usize,
    pub numeric_claim_count: usize,
    pub qualitative_claim_count: usize,
    pub comparison_group_count: usize,
    pub evidence_class_counts: Vec<EvidenceClassCount>,
    pub comparisons_are_scope_matched: bool,
    pub operator_claims_are_independently_verified: bool,
    pub claim_ceiling: &'static str,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct EvidenceClassCount {
    pub evidence_class: EvidenceClass,
    pub record_count: usize,
}

impl EvidenceLedger {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.schema_version != SCHEMA_VERSION {
            errors.push(format!(
                "schema_version: expected {SCHEMA_VERSION}, got {}",
                self.schema_version
            ));
        }
        if !valid_identifier(&self.benchmark_id) {
            errors.push("benchmark_id: invalid identifier".into());
        }
        if !valid_date(&self.snapshot_date) {
            errors.push("snapshot_date: expected YYYY-MM-DD".into());
        }
        if self.claim_ceiling.trim().is_empty() {
            errors.push("claim_ceiling: must not be empty".into());
        }
        if self.records.is_empty() || self.records.len() > MAX_RECORDS {
            errors.push(format!("records: expected 1..={MAX_RECORDS}"));
        }
        if self.comparison_groups.len() > MAX_COMPARISON_GROUPS {
            errors.push(format!(
                "comparison_groups: maximum is {MAX_COMPARISON_GROUPS}"
            ));
        }

        let mut record_ids = BTreeSet::new();
        let mut claim_ids = BTreeSet::new();
        let mut claim_index = std::collections::BTreeMap::new();
        let mut total_claims = 0usize;

        for (record_index, record) in self.records.iter().enumerate() {
            let prefix = format!("records[{record_index}]");
            if !valid_identifier(&record.record_id)
                || !record_ids.insert(record.record_id.as_str())
            {
                errors.push(format!("{prefix}.record_id: must be valid and unique"));
            }
            if record.title.trim().is_empty()
                || record.publisher.trim().is_empty()
                || record.scope.trim().is_empty()
            {
                errors.push(format!("{prefix}: title, publisher and scope are required"));
            }
            if !record.source_url.starts_with("https://") {
                errors.push(format!("{prefix}.source_url: https URL required"));
            }
            if !valid_date(&record.published_on) {
                errors.push(format!("{prefix}.published_on: expected YYYY-MM-DD"));
            }
            if !valid_date(&record.observed_as_of) {
                errors.push(format!("{prefix}.observed_as_of: expected YYYY-MM-DD"));
            }
            if record.claims.is_empty() {
                errors.push(format!("{prefix}.claims: must not be empty"));
            }
            total_claims = total_claims.saturating_add(record.claims.len());

            for (claim_index_in_record, claim) in record.claims.iter().enumerate() {
                let claim_prefix = format!("{prefix}.claims[{claim_index_in_record}]");
                if !valid_identifier(&claim.claim_id)
                    || !claim_ids.insert(claim.claim_id.as_str())
                {
                    errors.push(format!("{claim_prefix}.claim_id: must be valid and globally unique"));
                }
                if claim.geography.trim().is_empty()
                    || claim.period.trim().is_empty()
                    || claim.configuration.trim().is_empty()
                    || claim.population.trim().is_empty()
                    || claim.limitation.trim().is_empty()
                {
                    errors.push(format!(
                        "{claim_prefix}: geography, period, configuration, population and limitation are required"
                    ));
                }

                match (claim.numeric_value, claim.text_value.as_deref()) {
                    (Some(value), None) => {
                        if !value.is_finite() {
                            errors.push(format!("{claim_prefix}.numeric_value: must be finite"));
                        }
                        if claim.metric_id.as_deref().is_none_or(str::is_empty)
                            || claim.statistic.as_deref().is_none_or(str::is_empty)
                            || claim.unit.as_deref().is_none_or(str::is_empty)
                        {
                            errors.push(format!(
                                "{claim_prefix}: numeric claims require metric_id, statistic and unit"
                            ));
                        }
                    }
                    (None, Some(text)) if !text.trim().is_empty() => {
                        if claim.metric_id.is_some()
                            || claim.statistic.is_some()
                            || claim.unit.is_some()
                        {
                            errors.push(format!(
                                "{claim_prefix}: qualitative claims must not declare numeric metric fields"
                            ));
                        }
                    }
                    _ => errors.push(format!(
                        "{claim_prefix}: exactly one of numeric_value or nonempty text_value is required"
                    )),
                }

                claim_index.insert(
                    claim.claim_id.as_str(),
                    (record.evidence_class, claim),
                );
            }
        }

        if total_claims > MAX_CLAIMS {
            errors.push(format!("claims: maximum is {MAX_CLAIMS}"));
        }

        let mut group_ids = BTreeSet::new();
        for (group_index, group) in self.comparison_groups.iter().enumerate() {
            let prefix = format!("comparison_groups[{group_index}]");
            if !valid_identifier(&group.group_id)
                || !group_ids.insert(group.group_id.as_str())
            {
                errors.push(format!("{prefix}.group_id: must be valid and unique"));
            }
            if group.metric_id.trim().is_empty()
                || group.statistic.trim().is_empty()
                || group.unit.trim().is_empty()
                || group.geography.trim().is_empty()
                || group.period.trim().is_empty()
                || group.comparability_basis.trim().is_empty()
            {
                errors.push(format!(
                    "{prefix}: metric, statistic, unit, geography, period and comparability basis are required"
                ));
            }
            if group.claim_ids.len() < 2 {
                errors.push(format!("{prefix}.claim_ids: at least two claims required"));
            }
            let mut seen = BTreeSet::new();
            let mut evidence_class: Option<EvidenceClass> = None;
            for claim_id in &group.claim_ids {
                if !seen.insert(claim_id.as_str()) {
                    errors.push(format!("{prefix}.claim_ids: duplicate claim reference"));
                    continue;
                }
                let Some((claim_evidence, claim)) = claim_index.get(claim_id.as_str()) else {
                    errors.push(format!("{prefix}.claim_ids: unknown claim {claim_id}"));
                    continue;
                };
                if evidence_class.is_some_and(|existing| existing != *claim_evidence) {
                    errors.push(format!(
                        "{prefix}: comparisons cannot mix evidence classes"
                    ));
                }
                evidence_class = Some(*claim_evidence);
                if claim.metric_id.as_deref() != Some(group.metric_id.as_str())
                    || claim.statistic.as_deref() != Some(group.statistic.as_str())
                    || claim.unit.as_deref() != Some(group.unit.as_str())
                    || claim.geography != group.geography
                    || claim.period != group.period
                {
                    errors.push(format!(
                        "{prefix}: all claims must match metric, statistic, unit, geography and period exactly"
                    ));
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            errors.sort();
            Err(errors)
        }
    }

    pub fn summary(&self) -> Result<ValidationSummary, Vec<String>> {
        self.validate()?;
        let mut counts = std::collections::BTreeMap::<EvidenceClass, usize>::new();
        let mut numeric_claim_count = 0usize;
        let mut qualitative_claim_count = 0usize;
        for record in &self.records {
            *counts.entry(record.evidence_class).or_default() += 1;
            for claim in &record.claims {
                if claim.numeric_value.is_some() {
                    numeric_claim_count += 1;
                } else {
                    qualitative_claim_count += 1;
                }
            }
        }
        let evidence_class_counts = counts
            .into_iter()
            .map(|(evidence_class, record_count)| EvidenceClassCount {
                evidence_class,
                record_count,
            })
            .collect();
        Ok(ValidationSummary {
            schema_version: self.schema_version,
            benchmark_id: self.benchmark_id.clone(),
            snapshot_date: self.snapshot_date.clone(),
            evidence_record_count: self.records.len(),
            numeric_claim_count,
            qualitative_claim_count,
            comparison_group_count: self.comparison_groups.len(),
            evidence_class_counts,
            comparisons_are_scope_matched: true,
            operator_claims_are_independently_verified: false,
            claim_ceiling: CLAIM_CEILING,
        })
    }
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 96
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':')
        })
}

fn valid_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str =
        include_str!("../fixtures/ev_drone_operations_evidence_v0_1.json");

    fn valid_ledger() -> EvidenceLedger {
        serde_json::from_str(FIXTURE).expect("operations evidence fixture must deserialize")
    }

    #[test]
    fn committed_source_ledger_validates() {
        let ledger = valid_ledger();
        let summary = ledger.summary().expect("valid source ledger");
        assert_eq!(summary.benchmark_id, "EV-DRONE-OPERATIONS-BASELINE-001");
        assert_eq!(summary.evidence_record_count, 8);
        assert!(summary.numeric_claim_count >= 8);
        assert!(summary.qualitative_claim_count >= 3);
        assert!(!summary.operator_claims_are_independently_verified);
    }

    #[test]
    fn numeric_claim_cannot_also_contain_qualitative_text() {
        let mut ledger = valid_ledger();
        ledger.records[0].claims[0].text_value = Some("one million".into());
        assert!(ledger.validate().unwrap_err().iter().any(|error| {
            error.contains("exactly one")
        }));
    }

    #[test]
    fn numeric_claim_requires_metric_statistic_and_unit() {
        let mut ledger = valid_ledger();
        ledger.records[0].claims[0].unit = None;
        assert!(ledger.validate().unwrap_err().iter().any(|error| {
            error.contains("numeric claims require")
        }));
    }

    #[test]
    fn company_reported_claim_is_not_promoted_to_independent_measurement() {
        let ledger = valid_ledger();
        let record = ledger.records.iter().find(|record| {
            record.record_id == "zipline-150m-autonomous-miles-claim"
        }).expect("Zipline record");
        assert_eq!(record.evidence_class, EvidenceClass::CompanyReportedOperationalClaim);
    }

    #[test]
    fn comparison_group_rejects_mismatched_statistic_or_unit() {
        let mut ledger = valid_ledger();
        let first = ledger.records[0].claims[0].clone();
        let mut second = first.clone();
        second.claim_id = "comparable-second".into();
        second.unit = Some("deliveries/day".into());
        ledger.records[1].claims.push(second);
        ledger.comparison_groups.push(ComparisonGroup {
            group_id: "bad-delivery-count-comparison".into(),
            metric_id: first.metric_id.unwrap(),
            statistic: first.statistic.unwrap(),
            unit: "deliveries".into(),
            geography: first.geography,
            period: first.period,
            claim_ids: vec![first.claim_id, "comparable-second".into()],
            comparability_basis: "intentionally mismatched unit negative control".into(),
        });
        assert!(ledger.validate().unwrap_err().iter().any(|error| {
            error.contains("all claims must match")
        }));
    }

    #[test]
    fn comparison_group_rejects_mixed_evidence_classes() {
        let mut ledger = valid_ledger();
        let numeric = ledger.records[0].claims[0].clone();
        let mut second = numeric.clone();
        second.claim_id = "mixed-class-second".into();
        ledger.records[1].claims.push(second);
        ledger.records[1].evidence_class = EvidenceClass::IndependentMeasurement;
        ledger.comparison_groups.push(ComparisonGroup {
            group_id: "mixed-evidence-negative-control".into(),
            metric_id: numeric.metric_id.unwrap(),
            statistic: numeric.statistic.unwrap(),
            unit: numeric.unit.unwrap(),
            geography: numeric.geography,
            period: numeric.period,
            claim_ids: vec![numeric.claim_id, "mixed-class-second".into()],
            comparability_basis: "intentionally mixed evidence classes negative control".into(),
        });
        assert!(ledger.validate().unwrap_err().iter().any(|error| {
            error.contains("cannot mix evidence classes")
        }));
    }

    #[test]
    fn rejects_non_https_source() {
        let mut ledger = valid_ledger();
        ledger.records[0].source_url = "http://example.org/source".into();
        assert!(ledger.validate().unwrap_err().iter().any(|error| {
            error.contains("https URL required")
        }));
    }

    #[test]
    fn rejects_duplicate_claim_identity() {
        let mut ledger = valid_ledger();
        let duplicate = ledger.records[0].claims[0].clone();
        ledger.records[1].claims.push(duplicate);
        assert!(ledger.validate().unwrap_err().iter().any(|error| {
            error.contains("globally unique")
        }));
    }

    #[test]
    fn rejects_bad_publication_date() {
        let mut ledger = valid_ledger();
        ledger.records[0].published_on = "2026-2-5".into();
        assert!(ledger.validate().unwrap_err().iter().any(|error| {
            error.contains("YYYY-MM-DD")
        }));
    }
}
