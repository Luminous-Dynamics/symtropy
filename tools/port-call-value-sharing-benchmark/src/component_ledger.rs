#![forbid(unsafe_code)]

//! Auditable component ledger for port-call counterfactual costs.
//!
//! Each participant must provide exactly one baseline/treatment row for each
//! category required by a frozen contract, including explicit zero values.
//! This qualifies completeness relative to that category contract, not relative
//! to every real-world impact. Receipt digests are syntax-checked here; upstream
//! verification must match them to source bytes.

use super::EvidenceTier;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CostLedgerContract {
    pub case_id: String,
    pub currency_code: String,
    pub evidence_tier: EvidenceTier,
    pub baseline_receipt_sha256: String,
    pub treatment_receipt_sha256: String,
    pub comparison_contract_sha256: String,
    /// These identifiers must match: otherwise the service is not comparable.
    pub baseline_service_contract_id: String,
    pub treatment_service_contract_id: String,
    /// These identifiers must match: otherwise the horizon is not comparable.
    pub baseline_horizon_id: String,
    pub treatment_horizon_id: String,
    /// Frozen cost categories for which every participant must supply one row.
    pub required_categories: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CostComponent {
    pub category_id: String,
    /// Amount in the contract currency's smallest unit. Credits may be negative.
    pub baseline_cost_minor: i64,
    /// Amount in the contract currency's smallest unit. Credits may be negative.
    pub treatment_cost_minor: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParticipantCostLedger {
    pub participant_id: String,
    pub components: Vec<CostComponent>,
    /// Positive means receives value; negative means contributes value.
    pub transfer_in_minor: i64,
    /// Participant-provided acceptance threshold, not inferred from behavior.
    pub minimum_acceptable_benefit_minor: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CostLedgerError {
    InvalidIdentifier { kind: &'static str, value: String },
    InvalidCurrencyCode(String),
    InvalidSha256 { field: &'static str },
    IdenticalCounterfactualReceipts,
    UnqualifiedEvidence,
    DifferentServiceContracts,
    DifferentHorizons,
    EmptyRequiredCategories,
    DuplicateRequiredCategory(String),
    UnknownCostCategory { participant_id: String, category_id: String },
    DuplicateCostComponent { participant_id: String, category_id: String },
    MissingCostComponent { participant_id: String, category_id: String },
    TooFewParticipants,
    DuplicateParticipant(String),
    TransfersDoNotConserve { net_transfer_minor: i128 },
    ArithmeticOverflow,
    AggregateInvariantViolation,
}

impl std::fmt::Display for CostLedgerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for CostLedgerError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CategoryCostOutcome {
    pub category_id: String,
    pub baseline_total_minor: i128,
    pub treatment_total_minor: i128,
    pub gross_benefit_minor: i128,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParticipantCategoryOutcome {
    pub category_id: String,
    pub baseline_cost_minor: i128,
    pub treatment_cost_minor: i128,
    pub gross_benefit_minor: i128,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParticipantCostOutcome {
    pub participant_id: String,
    pub baseline_net_cost_minor: i128,
    pub treatment_net_cost_minor: i128,
    pub gross_benefit_minor: i128,
    pub transfer_in_minor: i64,
    pub net_benefit_minor: i128,
    pub minimum_acceptable_benefit_minor: i64,
    pub meets_reservation: bool,
    /// Sorted by category ID and explicitly includes rows whose cost is zero.
    pub components: Vec<ParticipantCategoryOutcome>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CostLedgerReport {
    pub case_id: String,
    pub currency_code: String,
    pub evidence_tier: EvidenceTier,
    pub baseline_receipt_sha256: String,
    pub treatment_receipt_sha256: String,
    pub comparison_contract_sha256: String,
    pub baseline_service_contract_id: String,
    pub horizon_id: String,
    pub total_baseline_cost_minor: i128,
    pub total_treatment_cost_minor: i128,
    pub total_gross_benefit_minor: i128,
    pub net_transfer_minor: i128,
    pub all_reservations_met: bool,
    /// Sorted by category ID.
    pub categories: Vec<CategoryCostOutcome>,
    /// Sorted by participant ID.
    pub participants: Vec<ParticipantCostOutcome>,
}

impl CostLedgerReport {
    /// Canonical JSON: stable field order, integer-only amounts, and text
    /// constrained to a small identifier alphabet, hex digests, currency, enums.
    pub fn to_json(&self) -> String {
        let mut output = String::new();
        write!(
            &mut output,
            "{{\"case_id\":\"{}\",\"currency_code\":\"{}\",\"evidence_tier\":\"{}\",\"baseline_receipt_sha256\":\"{}\",\"treatment_receipt_sha256\":\"{}\",\"comparison_contract_sha256\":\"{}\",\"service_contract_id\":\"{}\",\"horizon_id\":\"{}\",\"total_baseline_cost_minor\":{},\"total_treatment_cost_minor\":{},\"total_gross_benefit_minor\":{},\"net_transfer_minor\":{},\"all_reservations_met\":{},\"categories\":[",
            self.case_id,
            self.currency_code,
            evidence_tier_name(self.evidence_tier),
            self.baseline_receipt_sha256,
            self.treatment_receipt_sha256,
            self.comparison_contract_sha256,
            self.baseline_service_contract_id,
            self.horizon_id,
            self.total_baseline_cost_minor,
            self.total_treatment_cost_minor,
            self.total_gross_benefit_minor,
            self.net_transfer_minor,
            self.all_reservations_met
        )
        .expect("writing into String cannot fail");

        for (index, category) in self.categories.iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            write!(
                &mut output,
                "{{\"id\":\"{}\",\"baseline_total_minor\":{},\"treatment_total_minor\":{},\"gross_benefit_minor\":{}}}",
                category.category_id,
                category.baseline_total_minor,
                category.treatment_total_minor,
                category.gross_benefit_minor
            )
            .expect("writing into String cannot fail");
        }

        output.push_str("],\"participants\":[");
        for (index, participant) in self.participants.iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            write!(
                &mut output,
                "{{\"id\":\"{}\",\"baseline_net_cost_minor\":{},\"treatment_net_cost_minor\":{},\"gross_benefit_minor\":{},\"transfer_in_minor\":{},\"net_benefit_minor\":{},\"minimum_acceptable_benefit_minor\":{},\"meets_reservation\":{},\"components\":[",
                participant.participant_id,
                participant.baseline_net_cost_minor,
                participant.treatment_net_cost_minor,
                participant.gross_benefit_minor,
                participant.transfer_in_minor,
                participant.net_benefit_minor,
                participant.minimum_acceptable_benefit_minor,
                participant.meets_reservation
            )
            .expect("writing into String cannot fail");

            for (component_index, component) in participant.components.iter().enumerate() {
                if component_index > 0 {
                    output.push(',');
                }
                write!(
                    &mut output,
                    "{{\"id\":\"{}\",\"baseline_cost_minor\":{},\"treatment_cost_minor\":{},\"gross_benefit_minor\":{}}}",
                    component.category_id,
                    component.baseline_cost_minor,
                    component.treatment_cost_minor,
                    component.gross_benefit_minor
                )
                .expect("writing into String cannot fail");
            }
            output.push_str("]}");
        }

        output.push_str("]}");
        output
    }
}

fn evidence_tier_name(value: EvidenceTier) -> &'static str {
    match value {
        EvidenceTier::Synthetic => "synthetic",
        EvidenceTier::QualifiedHistoricalReplay => "qualified_historical_replay",
        EvidenceTier::Unqualified => "unqualified",
    }
}

fn validate_identifier(kind: &'static str, value: &str) -> Result<(), CostLedgerError> {
    let allowed = value.bytes().all(|byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-')
    });
    if value.is_empty() || !allowed {
        return Err(CostLedgerError::InvalidIdentifier {
            kind,
            value: value.to_owned(),
        });
    }
    Ok(())
}

fn validate_sha256(field: &'static str, digest: &str) -> Result<(), CostLedgerError> {
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(CostLedgerError::InvalidSha256 { field });
    }
    Ok(())
}

fn validate_contract(contract: &CostLedgerContract) -> Result<BTreeSet<String>, CostLedgerError> {
    validate_identifier("case", &contract.case_id)?;
    validate_identifier("baseline service contract", &contract.baseline_service_contract_id)?;
    validate_identifier("treatment service contract", &contract.treatment_service_contract_id)?;
    validate_identifier("baseline horizon", &contract.baseline_horizon_id)?;
    validate_identifier("treatment horizon", &contract.treatment_horizon_id)?;

    if contract.currency_code.len() != 3
        || !contract.currency_code.bytes().all(|byte| byte.is_ascii_uppercase())
    {
        return Err(CostLedgerError::InvalidCurrencyCode(
            contract.currency_code.clone(),
        ));
    }

    validate_sha256("baseline_receipt_sha256", &contract.baseline_receipt_sha256)?;
    validate_sha256("treatment_receipt_sha256", &contract.treatment_receipt_sha256)?;
    validate_sha256(
        "comparison_contract_sha256",
        &contract.comparison_contract_sha256,
    )?;

    if contract.baseline_receipt_sha256 == contract.treatment_receipt_sha256 {
        return Err(CostLedgerError::IdenticalCounterfactualReceipts);
    }
    if contract.evidence_tier == EvidenceTier::Unqualified {
        return Err(CostLedgerError::UnqualifiedEvidence);
    }
    if contract.baseline_service_contract_id != contract.treatment_service_contract_id {
        return Err(CostLedgerError::DifferentServiceContracts);
    }
    if contract.baseline_horizon_id != contract.treatment_horizon_id {
        return Err(CostLedgerError::DifferentHorizons);
    }
    if contract.required_categories.is_empty() {
        return Err(CostLedgerError::EmptyRequiredCategories);
    }

    let mut categories = BTreeSet::new();
    for category_id in &contract.required_categories {
        validate_identifier("cost category", category_id)?;
        if !categories.insert(category_id.clone()) {
            return Err(CostLedgerError::DuplicateRequiredCategory(
                category_id.clone(),
            ));
        }
    }
    Ok(categories)
}

/// Evaluate one full baseline/treatment component ledger.
///
/// Every participant must have exactly one row for every required category.
/// Absent zero-cost rows are intentionally rejected: zero must be explicit.
pub fn evaluate_cost_ledger(
    contract: &CostLedgerContract,
    ledgers: &[ParticipantCostLedger],
) -> Result<CostLedgerReport, CostLedgerError> {
    let required_categories = validate_contract(contract)?;
    if ledgers.len() < 2 {
        return Err(CostLedgerError::TooFewParticipants);
    }

    let mut participant_ids = BTreeSet::new();
    let mut participants = Vec::with_capacity(ledgers.len());
    let mut category_totals: BTreeMap<String, (i128, i128)> = required_categories
        .iter()
        .cloned()
        .map(|category| (category, (0, 0)))
        .collect();
    let mut total_baseline_cost_minor = 0_i128;
    let mut total_treatment_cost_minor = 0_i128;
    let mut total_gross_benefit_minor = 0_i128;
    let mut net_transfer_minor = 0_i128;

    for ledger in ledgers {
        validate_identifier("participant", &ledger.participant_id)?;
        if !participant_ids.insert(ledger.participant_id.clone()) {
            return Err(CostLedgerError::DuplicateParticipant(
                ledger.participant_id.clone(),
            ));
        }

        let mut rows = BTreeMap::new();
        for component in &ledger.components {
            validate_identifier("cost category", &component.category_id)?;
            if !required_categories.contains(&component.category_id) {
                return Err(CostLedgerError::UnknownCostCategory {
                    participant_id: ledger.participant_id.clone(),
                    category_id: component.category_id.clone(),
                });
            }
            if rows.insert(component.category_id.clone(), component).is_some() {
                return Err(CostLedgerError::DuplicateCostComponent {
                    participant_id: ledger.participant_id.clone(),
                    category_id: component.category_id.clone(),
                });
            }
        }

        for category_id in &required_categories {
            if !rows.contains_key(category_id) {
                return Err(CostLedgerError::MissingCostComponent {
                    participant_id: ledger.participant_id.clone(),
                    category_id: category_id.clone(),
                });
            }
        }

        let mut participant_components = Vec::with_capacity(required_categories.len());
        let mut baseline_net_cost_minor = 0_i128;
        let mut treatment_net_cost_minor = 0_i128;

        for category_id in &required_categories {
            let component = rows.get(category_id).expect("all required rows validated");
            let baseline = i128::from(component.baseline_cost_minor);
            let treatment = i128::from(component.treatment_cost_minor);
            let gross_benefit = baseline
                .checked_sub(treatment)
                .ok_or(CostLedgerError::ArithmeticOverflow)?;

            baseline_net_cost_minor = baseline_net_cost_minor
                .checked_add(baseline)
                .ok_or(CostLedgerError::ArithmeticOverflow)?;
            treatment_net_cost_minor = treatment_net_cost_minor
                .checked_add(treatment)
                .ok_or(CostLedgerError::ArithmeticOverflow)?;

            let totals = category_totals
                .get_mut(category_id)
                .expect("required category totals preallocated");
            totals.0 = totals
                .0
                .checked_add(baseline)
                .ok_or(CostLedgerError::ArithmeticOverflow)?;
            totals.1 = totals
                .1
                .checked_add(treatment)
                .ok_or(CostLedgerError::ArithmeticOverflow)?;

            participant_components.push(ParticipantCategoryOutcome {
                category_id: category_id.clone(),
                baseline_cost_minor: baseline,
                treatment_cost_minor: treatment,
                gross_benefit_minor: gross_benefit,
            });
        }

        let gross_benefit_minor = baseline_net_cost_minor
            .checked_sub(treatment_net_cost_minor)
            .ok_or(CostLedgerError::ArithmeticOverflow)?;
        let net_benefit_minor = gross_benefit_minor
            .checked_add(i128::from(ledger.transfer_in_minor))
            .ok_or(CostLedgerError::ArithmeticOverflow)?;

        total_baseline_cost_minor = total_baseline_cost_minor
            .checked_add(baseline_net_cost_minor)
            .ok_or(CostLedgerError::ArithmeticOverflow)?;
        total_treatment_cost_minor = total_treatment_cost_minor
            .checked_add(treatment_net_cost_minor)
            .ok_or(CostLedgerError::ArithmeticOverflow)?;
        total_gross_benefit_minor = total_gross_benefit_minor
            .checked_add(gross_benefit_minor)
            .ok_or(CostLedgerError::ArithmeticOverflow)?;
        net_transfer_minor = net_transfer_minor
            .checked_add(i128::from(ledger.transfer_in_minor))
            .ok_or(CostLedgerError::ArithmeticOverflow)?;

        participants.push(ParticipantCostOutcome {
            participant_id: ledger.participant_id.clone(),
            baseline_net_cost_minor,
            treatment_net_cost_minor,
            gross_benefit_minor,
            transfer_in_minor: ledger.transfer_in_minor,
            net_benefit_minor,
            minimum_acceptable_benefit_minor: ledger.minimum_acceptable_benefit_minor,
            meets_reservation: net_benefit_minor
                >= i128::from(ledger.minimum_acceptable_benefit_minor),
            components: participant_components,
        });
    }

    if net_transfer_minor != 0 {
        return Err(CostLedgerError::TransfersDoNotConserve { net_transfer_minor });
    }

    let mut categories = Vec::with_capacity(category_totals.len());
    for (category_id, (baseline_total_minor, treatment_total_minor)) in category_totals {
        let gross_benefit_minor = baseline_total_minor
            .checked_sub(treatment_total_minor)
            .ok_or(CostLedgerError::ArithmeticOverflow)?;
        categories.push(CategoryCostOutcome {
            category_id,
            baseline_total_minor,
            treatment_total_minor,
            gross_benefit_minor,
        });
    }

    participants.sort_by_key(|item| item.participant_id.clone());
    let all_reservations_met = participants.iter().all(|item| item.meets_reservation);
    let category_baseline_total = categories.iter().try_fold(0_i128, |total, category| {
        total
            .checked_add(category.baseline_total_minor)
            .ok_or(CostLedgerError::ArithmeticOverflow)
    })?;
    let category_treatment_total = categories.iter().try_fold(0_i128, |total, category| {
        total
            .checked_add(category.treatment_total_minor)
            .ok_or(CostLedgerError::ArithmeticOverflow)
    })?;
    let category_benefit_total = categories.iter().try_fold(0_i128, |total, category| {
        total
            .checked_add(category.gross_benefit_minor)
            .ok_or(CostLedgerError::ArithmeticOverflow)
    })?;
    let net_benefit_total = participants.iter().try_fold(0_i128, |total, participant| {
        total
            .checked_add(participant.net_benefit_minor)
            .ok_or(CostLedgerError::ArithmeticOverflow)
    })?;

    if category_baseline_total != total_baseline_cost_minor
        || category_treatment_total != total_treatment_cost_minor
        || category_benefit_total != total_gross_benefit_minor
        || net_benefit_total != total_gross_benefit_minor
    {
        return Err(CostLedgerError::AggregateInvariantViolation);
    }

    Ok(CostLedgerReport {
        case_id: contract.case_id.clone(),
        currency_code: contract.currency_code.clone(),
        evidence_tier: contract.evidence_tier,
        baseline_receipt_sha256: contract.baseline_receipt_sha256.clone(),
        treatment_receipt_sha256: contract.treatment_receipt_sha256.clone(),
        comparison_contract_sha256: contract.comparison_contract_sha256.clone(),
        baseline_service_contract_id: contract.baseline_service_contract_id.clone(),
        horizon_id: contract.baseline_horizon_id.clone(),
        total_baseline_cost_minor,
        total_treatment_cost_minor,
        total_gross_benefit_minor,
        net_transfer_minor,
        all_reservations_met,
        categories,
        participants,
    })
}

/// Synthetic component-level example. Each of three parties has exactly one
/// explicit baseline/treatment row for every category in the frozen contract.
pub fn run_demo() -> Result<CostLedgerReport, CostLedgerError> {
    let contract = CostLedgerContract {
        case_id: "synthetic-component-cost-ledger-v0.1".to_owned(),
        currency_code: "USD".to_owned(),
        evidence_tier: EvidenceTier::Synthetic,
        baseline_receipt_sha256: "a".repeat(64),
        treatment_receipt_sha256: "b".repeat(64),
        comparison_contract_sha256: "c".repeat(64),
        baseline_service_contract_id: "container-delivery-service-v1".to_owned(),
        treatment_service_contract_id: "container-delivery-service-v1".to_owned(),
        baseline_horizon_id: "2026-Q3".to_owned(),
        treatment_horizon_id: "2026-Q3".to_owned(),
        required_categories: vec![
            "bunkers".to_owned(),
            "coordination".to_owned(),
            "demurrage".to_owned(),
        ],
    };
    let ledgers = vec![
        ParticipantCostLedger {
            participant_id: "PORT-OPS".to_owned(),
            components: vec![
                CostComponent {
                    category_id: "bunkers".to_owned(),
                    baseline_cost_minor: 70_000,
                    treatment_cost_minor: 68_000,
                },
                CostComponent {
                    category_id: "coordination".to_owned(),
                    baseline_cost_minor: 20_000,
                    treatment_cost_minor: 22_000,
                },
                CostComponent {
                    category_id: "demurrage".to_owned(),
                    baseline_cost_minor: 10_000,
                    treatment_cost_minor: 5_000,
                },
            ],
            transfer_in_minor: -500,
            minimum_acceptable_benefit_minor: 0,
        },
        ParticipantCostLedger {
            participant_id: "SHIP-OWNER".to_owned(),
            components: vec![
                CostComponent {
                    category_id: "bunkers".to_owned(),
                    baseline_cost_minor: 100_000,
                    treatment_cost_minor: 101_000,
                },
                CostComponent {
                    category_id: "coordination".to_owned(),
                    baseline_cost_minor: 10_000,
                    treatment_cost_minor: 15_000,
                },
                CostComponent {
                    category_id: "demurrage".to_owned(),
                    baseline_cost_minor: 10_000,
                    treatment_cost_minor: 6_000,
                },
            ],
            transfer_in_minor: 2_000,
            minimum_acceptable_benefit_minor: 0,
        },
        ParticipantCostLedger {
            participant_id: "CHARTERER".to_owned(),
            components: vec![
                CostComponent {
                    category_id: "bunkers".to_owned(),
                    baseline_cost_minor: 70_000,
                    treatment_cost_minor: 68_000,
                },
                CostComponent {
                    category_id: "coordination".to_owned(),
                    baseline_cost_minor: 5_000,
                    treatment_cost_minor: 4_000,
                },
                CostComponent {
                    category_id: "demurrage".to_owned(),
                    baseline_cost_minor: 5_000,
                    treatment_cost_minor: 5_000,
                },
            ],
            transfer_in_minor: -1_500,
            minimum_acceptable_benefit_minor: 0,
        },
    ];
    evaluate_cost_ledger(&contract, &ledgers)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contract() -> CostLedgerContract {
        CostLedgerContract {
            case_id: "ledger-test".to_owned(),
            currency_code: "USD".to_owned(),
            evidence_tier: EvidenceTier::Synthetic,
            baseline_receipt_sha256: "a".repeat(64),
            treatment_receipt_sha256: "b".repeat(64),
            comparison_contract_sha256: "c".repeat(64),
            baseline_service_contract_id: "service-v1".to_owned(),
            treatment_service_contract_id: "service-v1".to_owned(),
            baseline_horizon_id: "horizon-q3".to_owned(),
            treatment_horizon_id: "horizon-q3".to_owned(),
            required_categories: vec!["fuel".to_owned(), "delay".to_owned()],
        }
    }

    fn component(category: &str, baseline: i64, treatment: i64) -> CostComponent {
        CostComponent {
            category_id: category.to_owned(),
            baseline_cost_minor: baseline,
            treatment_cost_minor: treatment,
        }
    }

    fn ledger(
        participant: &str,
        components: Vec<CostComponent>,
        transfer: i64,
        minimum: i64,
    ) -> ParticipantCostLedger {
        ParticipantCostLedger {
            participant_id: participant.to_owned(),
            components,
            transfer_in_minor: transfer,
            minimum_acceptable_benefit_minor: minimum,
        }
    }

    fn input() -> (CostLedgerContract, Vec<ParticipantCostLedger>) {
        (
            contract(),
            vec![
                ledger("A", vec![component("fuel", 7, 4), component("delay", 3, 2)], 0, 0),
                ledger("B", vec![component("fuel", 2, 3), component("delay", 5, 7)], 0, 0),
                ledger("C", vec![component("fuel", 6, 5), component("delay", 2, 1)], 0, 0),
            ],
        )
    }

    #[test]
    fn demo_matches_frozen_receipt_and_category_aggregates_reconcile() {
        let report = run_demo().unwrap();
        assert_eq!(
            report.to_json(),
            include_str!("../fixtures/component-ledger-v0.1.expected.json").trim_end()
        );
        assert_eq!(report.total_baseline_cost_minor, 300_000);
        assert_eq!(report.total_treatment_cost_minor, 294_000);
        assert_eq!(report.total_gross_benefit_minor, 6_000);
        assert_eq!(report.net_transfer_minor, 0);
        assert!(report.all_reservations_met);
        assert_eq!(
            report.categories.iter().map(|c| c.gross_benefit_minor).sum::<i128>(),
            6_000
        );
    }

    #[test]
    fn missing_category_is_rejected_even_when_missing_cost_would_be_zero() {
        let (contract, mut ledgers) = input();
        ledgers[0].components.pop();
        assert_eq!(
            evaluate_cost_ledger(&contract, &ledgers),
            Err(CostLedgerError::MissingCostComponent {
                participant_id: "A".to_owned(),
                category_id: "delay".to_owned(),
            })
        );
    }

    #[test]
    fn duplicate_or_unknown_cost_categories_are_rejected() {
        let (contract, mut duplicate) = input();
        duplicate[0].components.push(component("fuel", 0, 0));
        assert_eq!(
            evaluate_cost_ledger(&contract, &duplicate),
            Err(CostLedgerError::DuplicateCostComponent {
                participant_id: "A".to_owned(),
                category_id: "fuel".to_owned(),
            })
        );

        let (contract, mut unknown) = input();
        unknown[0].components[0].category_id = "unreviewed-other".to_owned();
        assert_eq!(
            evaluate_cost_ledger(&contract, &unknown),
            Err(CostLedgerError::UnknownCostCategory {
                participant_id: "A".to_owned(),
                category_id: "unreviewed-other".to_owned(),
            })
        );
    }

    #[test]
    fn unequal_service_contract_or_horizon_is_rejected() {
        let (mut contract, ledgers) = input();
        contract.treatment_service_contract_id = "different-delivery".to_owned();
        assert_eq!(
            evaluate_cost_ledger(&contract, &ledgers),
            Err(CostLedgerError::DifferentServiceContracts)
        );
        let (mut contract, ledgers) = input();
        contract.treatment_horizon_id = "other-horizon".to_owned();
        assert_eq!(
            evaluate_cost_ledger(&contract, &ledgers),
            Err(CostLedgerError::DifferentHorizons)
        );
    }

    #[test]
    fn transfers_must_conserve_exactly() {
        let (contract, mut ledgers) = input();
        ledgers[0].transfer_in_minor = -1;
        assert_eq!(
            evaluate_cost_ledger(&contract, &ledgers),
            Err(CostLedgerError::TransfersDoNotConserve {
                net_transfer_minor: -1,
            })
        );
    }

    #[test]
    fn permutations_are_canonical_and_explicit_zero_rows_are_preserved() {
        let mut contract = contract();
        contract.required_categories.push("other".to_owned());
        let (_, mut ledgers) = input();
        for ledger in &mut ledgers {
            ledger.components.push(component("other", 0, 0));
        }
        let first = evaluate_cost_ledger(&contract, &ledgers).unwrap();
        let mut permuted = ledgers.clone();
        permuted.reverse();
        for ledger in &mut permuted {
            ledger.components.reverse();
        }
        let second = evaluate_cost_ledger(&contract, &permuted).unwrap();
        assert_eq!(first.to_json(), second.to_json());
        assert!(
            second.participants[0]
                .components
                .iter()
                .any(|component| {
                    component.category_id == "other"
                        && component.baseline_cost_minor == 0
                        && component.treatment_cost_minor == 0
                })
        );
    }

    #[test]
    fn component_totals_match_independent_hand_checked_arithmetic() {
        let (contract, ledgers) = input();
        let report = evaluate_cost_ledger(&contract, &ledgers).unwrap();
        // Independent direct sums: fuel benefit is +3, delay benefit is 0,
        // so aggregate gross benefit is +3 even though participant B loses.
        let baseline_fuel = 7_i128 + 2 + 6;
        let treatment_fuel = 4_i128 + 3 + 5;
        let baseline_delay = 3_i128 + 5 + 2;
        let treatment_delay = 2_i128 + 7 + 1;
        assert_eq!(report.categories[0].category_id, "delay");
        assert_eq!(report.categories[0].gross_benefit_minor, baseline_delay - treatment_delay);
        assert_eq!(report.categories[1].category_id, "fuel");
        assert_eq!(report.categories[1].gross_benefit_minor, baseline_fuel - treatment_fuel);
        assert_eq!(report.total_gross_benefit_minor, 3);
        assert_eq!(report.participants[1].gross_benefit_minor, -3);
        assert!(!report.all_reservations_met);
    }

    #[test]
    fn category_contract_identity_and_digest_inputs_are_validated() {
        let (mut contract, ledgers) = input();
        contract.required_categories.push("fuel".to_owned());
        assert_eq!(
            evaluate_cost_ledger(&contract, &ledgers),
            Err(CostLedgerError::DuplicateRequiredCategory("fuel".to_owned()))
        );

        let (mut contract, ledgers) = input();
        contract.treatment_receipt_sha256 = "invalid".to_owned();
        assert_eq!(
            evaluate_cost_ledger(&contract, &ledgers),
            Err(CostLedgerError::InvalidSha256 {
                field: "treatment_receipt_sha256",
            })
        );
    }

    #[test]
    fn all_participants_and_each_category_must_be_unique() {
        let (contract, mut ledgers) = input();
        ledgers[1].participant_id = "A".to_owned();
        assert_eq!(
            evaluate_cost_ledger(&contract, &ledgers),
            Err(CostLedgerError::DuplicateParticipant("A".to_owned()))
        );

        let (mut contract, ledgers) = input();
        contract.required_categories.clear();
        assert_eq!(
            evaluate_cost_ledger(&contract, &ledgers),
            Err(CostLedgerError::EmptyRequiredCategories)
        );
    }

    #[test]
    fn bounded_component_cost_sweep_matches_independent_oracle() {
        let mut cases = 0usize;
        for offset in -2_i64..=2 {
            for cost_shift in -2_i64..=2 {
                let contract = contract();
                let ledgers = vec![
                    ledger(
                        "A",
                        vec![
                            component("fuel", 7 + offset, 4 + cost_shift),
                            component("delay", 3, 2),
                        ],
                        0,
                        i64::MIN,
                    ),
                    ledger(
                        "B",
                        vec![component("fuel", 2, 3), component("delay", 5, 7)],
                        0,
                        i64::MIN,
                    ),
                    ledger(
                        "C",
                        vec![component("fuel", 6, 5), component("delay", 2, 1)],
                        0,
                        i64::MIN,
                    ),
                ];
                let report = evaluate_cost_ledger(&contract, &ledgers).unwrap();
                let gross_a = i128::from(7 + offset - (4 + cost_shift)) + 1;
                let gross_b = -3_i128;
                let gross_c = 2_i128;
                let expected_total = gross_a + gross_b + gross_c;
                assert_eq!(report.participants[0].gross_benefit_minor, gross_a);
                assert_eq!(report.participants[1].gross_benefit_minor, gross_b);
                assert_eq!(report.participants[2].gross_benefit_minor, gross_c);
                assert_eq!(report.total_gross_benefit_minor, expected_total);
                assert!(report.all_reservations_met);
                cases += 1;
            }
        }
        assert_eq!(cases, 25);
    }
}
