use super::{
    CounterfactualBasis, EvidenceTier, ParticipantAccount, ScenarioInput, ValueShareError,
    ValueShareReport, evaluate_value_share, validate_identifier, validate_sha256,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

/// One explicitly present cost value for each side of a counterfactual.
/// A zero is an explicit value; omitting the row is not equivalent to zero.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CostComponentRow {
    pub participant_id: String,
    pub category_id: String,
    pub baseline_cost_minor: i64,
    pub treatment_cost_minor: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CostLedgerParticipantTerms {
    pub participant_id: String,
    pub transfer_in_minor: i64,
    pub minimum_acceptable_benefit_minor: i64,
}

/// Component ledger input. The required category manifest is a versioned,
/// externally reviewed contract. This module checks digest syntax and matrix
/// completeness; an upstream verifier must bind the digest to exact manifest
/// bytes and reviewers must establish which real-world categories are in scope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComponentLedgerInput {
    pub case_id: String,
    pub currency_code: String,
    pub evidence_tier: EvidenceTier,
    pub same_delivered_service: bool,
    pub baseline_receipt_sha256: String,
    pub treatment_receipt_sha256: String,
    pub comparison_contract_sha256: String,
    pub category_manifest_sha256: String,
    pub required_category_ids: Vec<String>,
    pub baseline_fuel_grams: u64,
    pub treatment_fuel_grams: u64,
    pub baseline_co2_grams: u64,
    pub treatment_co2_grams: u64,
    pub participants: Vec<CostLedgerParticipantTerms>,
    pub components: Vec<CostComponentRow>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CostLedgerError {
    EmptyRequiredCategories,
    DuplicateCategory(String),
    DuplicateParticipant(String),
    UnknownParticipant { participant_id: String },
    UnknownCategory { category_id: String },
    DuplicateComponent { participant_id: String, category_id: String },
    MissingComponent { participant_id: String, category_id: String },
    AggregateOutsideI64 { participant_id: String, basis: &'static str },
    ArithmeticOverflow,
    ValueShare(ValueShareError),
}

impl std::fmt::Display for CostLedgerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for CostLedgerError {}

impl From<ValueShareError> for CostLedgerError {
    fn from(value: ValueShareError) -> Self {
        Self::ValueShare(value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CategoryTotal {
    pub category_id: String,
    pub baseline_total_minor: i128,
    pub treatment_total_minor: i128,
    /// Positive means the treatment has lower cost in this category.
    pub gross_benefit_minor: i128,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParticipantCategoryImpact {
    pub participant_id: String,
    pub category_id: String,
    pub baseline_cost_minor: i64,
    pub treatment_cost_minor: i64,
    /// Positive means lower treatment cost for this participant in this category.
    pub gross_benefit_minor: i128,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CostLedgerReport {
    pub category_manifest_sha256: String,
    pub account_report: ValueShareReport,
    /// Sorted by category ID.
    pub categories: Vec<CategoryTotal>,
    /// Sorted by participant ID, then category ID.
    pub participant_components: Vec<ParticipantCategoryImpact>,
}

impl CostLedgerReport {
    /// Canonical JSON wraps the established account report and adds the exact
    /// category-level reconciliation. Validated identifiers and hexadecimal
    /// digests avoid unescaped caller-provided free text in this v0.1 format.
    pub fn to_json(&self) -> String {
        let mut output = String::new();
        write!(
            &mut output,
            "{{\"category_manifest_sha256\":\"{}\",\"account_report\":{},\"categories\":[",
            self.category_manifest_sha256,
            self.account_report.to_json()
        )
        .expect("writing into String cannot fail");

        for (index, category) in self.categories.iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            write!(
                &mut output,
                "{{\"category_id\":\"{}\",\"baseline_total_minor\":{},\"treatment_total_minor\":{},\"gross_benefit_minor\":{}}}",
                category.category_id,
                category.baseline_total_minor,
                category.treatment_total_minor,
                category.gross_benefit_minor
            )
            .expect("writing into String cannot fail");
        }

        output.push_str("],\"participant_components\":[");
        for (index, item) in self.participant_components.iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            write!(
                &mut output,
                "{{\"participant_id\":\"{}\",\"category_id\":\"{}\",\"baseline_cost_minor\":{},\"treatment_cost_minor\":{},\"gross_benefit_minor\":{}}}",
                item.participant_id,
                item.category_id,
                item.baseline_cost_minor,
                item.treatment_cost_minor,
                item.gross_benefit_minor
            )
            .expect("writing into String cannot fail");
        }

        output.push_str("]}");
        output
    }
}

/// Rebuild participant totals from a complete participant x category matrix,
/// then pass those derived totals through the v0.1 transfer/evidence checker.
/// The caller cannot authoritatively supply pre-aggregated costs to this path.
pub fn evaluate_component_ledger(
    input: &ComponentLedgerInput,
) -> Result<CostLedgerReport, CostLedgerError> {
    validate_identifier("case", &input.case_id)?;
    validate_sha256("category_manifest_sha256", &input.category_manifest_sha256)?;

    if input.required_category_ids.is_empty() {
        return Err(CostLedgerError::EmptyRequiredCategories);
    }

    let mut required_categories = BTreeSet::new();
    for category_id in &input.required_category_ids {
        validate_identifier("cost category", category_id)?;
        if !required_categories.insert(category_id.clone()) {
            return Err(CostLedgerError::DuplicateCategory(category_id.clone()));
        }
    }

    let mut participant_ids = BTreeSet::new();
    for participant in &input.participants {
        validate_identifier("participant", &participant.participant_id)?;
        if !participant_ids.insert(participant.participant_id.clone()) {
            return Err(CostLedgerError::DuplicateParticipant(
                participant.participant_id.clone(),
            ));
        }
    }

    let mut rows_by_pair: BTreeMap<(String, String), &CostComponentRow> = BTreeMap::new();
    for row in &input.components {
        validate_identifier("participant", &row.participant_id)?;
        validate_identifier("cost category", &row.category_id)?;
        if !participant_ids.contains(&row.participant_id) {
            return Err(CostLedgerError::UnknownParticipant {
                participant_id: row.participant_id.clone(),
            });
        }
        if !required_categories.contains(&row.category_id) {
            return Err(CostLedgerError::UnknownCategory {
                category_id: row.category_id.clone(),
            });
        }
        let key = (row.participant_id.clone(), row.category_id.clone());
        if rows_by_pair.insert(key, row).is_some() {
            return Err(CostLedgerError::DuplicateComponent {
                participant_id: row.participant_id.clone(),
                category_id: row.category_id.clone(),
            });
        }
    }

    // A missing row is not interpreted as a zero-cost component.
    for participant_id in &participant_ids {
        for category_id in &required_categories {
            if !rows_by_pair.contains_key(&(participant_id.clone(), category_id.clone())) {
                return Err(CostLedgerError::MissingComponent {
                    participant_id: participant_id.clone(),
                    category_id: category_id.clone(),
                });
            }
        }
    }

    let mut account_rows = Vec::with_capacity(input.participants.len());
    for terms in &input.participants {
        let mut baseline_total = 0_i128;
        let mut treatment_total = 0_i128;
        for category_id in &required_categories {
            let row = rows_by_pair
                .get(&(terms.participant_id.clone(), category_id.clone()))
                .expect("complete matrix was checked");
            baseline_total = baseline_total
                .checked_add(i128::from(row.baseline_cost_minor))
                .ok_or(CostLedgerError::ArithmeticOverflow)?;
            treatment_total = treatment_total
                .checked_add(i128::from(row.treatment_cost_minor))
                .ok_or(CostLedgerError::ArithmeticOverflow)?;
        }

        let baseline_net_cost_minor = i64::try_from(baseline_total).map_err(|_| {
            CostLedgerError::AggregateOutsideI64 {
                participant_id: terms.participant_id.clone(),
                basis: "baseline",
            }
        })?;
        let treatment_net_cost_before_transfer_minor =
            i64::try_from(treatment_total).map_err(|_| {
                CostLedgerError::AggregateOutsideI64 {
                    participant_id: terms.participant_id.clone(),
                    basis: "treatment",
                }
            })?;

        account_rows.push(ParticipantAccount {
            participant_id: terms.participant_id.clone(),
            baseline_net_cost_minor,
            treatment_net_cost_before_transfer_minor,
            transfer_in_minor: terms.transfer_in_minor,
            minimum_acceptable_benefit_minor: terms.minimum_acceptable_benefit_minor,
        });
    }

    let basis = CounterfactualBasis {
        case_id: input.case_id.clone(),
        currency_code: input.currency_code.clone(),
        evidence_tier: input.evidence_tier,
        same_delivered_service: input.same_delivered_service,
        // Completeness is derived from the exact participant x category matrix
        // above, not accepted from a caller-supplied boolean.
        cost_boundary_complete: true,
        baseline_receipt_sha256: input.baseline_receipt_sha256.clone(),
        treatment_receipt_sha256: input.treatment_receipt_sha256.clone(),
        comparison_contract_sha256: input.comparison_contract_sha256.clone(),
    };
    let account_report = evaluate_value_share(&ScenarioInput {
        basis,
        baseline_fuel_grams: input.baseline_fuel_grams,
        treatment_fuel_grams: input.treatment_fuel_grams,
        baseline_co2_grams: input.baseline_co2_grams,
        treatment_co2_grams: input.treatment_co2_grams,
        participants: account_rows,
    })?;

    let mut category_totals = Vec::with_capacity(required_categories.len());
    for category_id in &required_categories {
        let mut baseline_total = 0_i128;
        let mut treatment_total = 0_i128;
        for participant_id in &participant_ids {
            let row = rows_by_pair
                .get(&(participant_id.clone(), category_id.clone()))
                .expect("complete matrix was checked");
            baseline_total = baseline_total
                .checked_add(i128::from(row.baseline_cost_minor))
                .ok_or(CostLedgerError::ArithmeticOverflow)?;
            treatment_total = treatment_total
                .checked_add(i128::from(row.treatment_cost_minor))
                .ok_or(CostLedgerError::ArithmeticOverflow)?;
        }
        let gross_benefit_minor = baseline_total
            .checked_sub(treatment_total)
            .ok_or(CostLedgerError::ArithmeticOverflow)?;
        category_totals.push(CategoryTotal {
            category_id: category_id.clone(),
            baseline_total_minor: baseline_total,
            treatment_total_minor: treatment_total,
            gross_benefit_minor,
        });
    }

    let mut participant_components = rows_by_pair
        .values()
        .map(|row| {
            let gross_benefit_minor = i128::from(row.baseline_cost_minor)
                - i128::from(row.treatment_cost_minor);
            ParticipantCategoryImpact {
                participant_id: row.participant_id.clone(),
                category_id: row.category_id.clone(),
                baseline_cost_minor: row.baseline_cost_minor,
                treatment_cost_minor: row.treatment_cost_minor,
                gross_benefit_minor,
            }
        })
        .collect::<Vec<_>>();
    participant_components.sort_by_key(|item| {
        (item.participant_id.clone(), item.category_id.clone())
    });

    let component_gross_total = category_totals.iter().try_fold(0_i128, |total, category| {
        total
            .checked_add(category.gross_benefit_minor)
            .ok_or(CostLedgerError::ArithmeticOverflow)
    })?;
    if component_gross_total != account_report.total_gross_benefit_minor {
        return Err(CostLedgerError::ArithmeticOverflow);
    }

    Ok(CostLedgerReport {
        category_manifest_sha256: input.category_manifest_sha256.clone(),
        account_report,
        categories: category_totals,
        participant_components,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> ComponentLedgerInput {
        ComponentLedgerInput {
            case_id: "component-ledger-demo".to_owned(),
            currency_code: "USD".to_owned(),
            evidence_tier: EvidenceTier::Synthetic,
            same_delivered_service: true,
            baseline_receipt_sha256: "a".repeat(64),
            treatment_receipt_sha256: "b".repeat(64),
            comparison_contract_sha256: "c".repeat(64),
            category_manifest_sha256: "d".repeat(64),
            required_category_ids: vec!["fuel".to_owned(), "coordination".to_owned()],
            baseline_fuel_grams: 100_000,
            treatment_fuel_grams: 90_000,
            baseline_co2_grams: 316_000,
            treatment_co2_grams: 284_400,
            participants: vec![
                CostLedgerParticipantTerms {
                    participant_id: "PORT-OPS".to_owned(),
                    transfer_in_minor: -500,
                    minimum_acceptable_benefit_minor: 0,
                },
                CostLedgerParticipantTerms {
                    participant_id: "SHIP-OWNER".to_owned(),
                    transfer_in_minor: 2_000,
                    minimum_acceptable_benefit_minor: 0,
                },
                CostLedgerParticipantTerms {
                    participant_id: "CHARTERER".to_owned(),
                    transfer_in_minor: -1_500,
                    minimum_acceptable_benefit_minor: 0,
                },
            ],
            components: vec![
                row("PORT-OPS", "fuel", 60_000, 58_000),
                row("PORT-OPS", "coordination", 40_000, 37_000),
                row("SHIP-OWNER", "fuel", 70_000, 71_000),
                row("SHIP-OWNER", "coordination", 50_000, 51_000),
                row("CHARTERER", "fuel", 60_000, 57_000),
                row("CHARTERER", "coordination", 20_000, 20_000),
            ],
        }
    }

    fn row(
        participant_id: &str,
        category_id: &str,
        baseline: i64,
        treatment: i64,
    ) -> CostComponentRow {
        CostComponentRow {
            participant_id: participant_id.to_owned(),
            category_id: category_id.to_owned(),
            baseline_cost_minor: baseline,
            treatment_cost_minor: treatment,
        }
    }

    #[test]
    fn category_ledger_reconciles_to_participant_account_totals() {
        let report = evaluate_component_ledger(&input()).unwrap();
        assert_eq!(
            report.to_json(),
            include_str!("../fixtures/component-ledger-v0.1.expected.json").trim_end()
        );
        assert_eq!(report.account_report.total_gross_benefit_minor, 6_000);
        assert_eq!(report.account_report.net_transfer_minor, 0);
        assert!(report.account_report.all_reservations_met);
        assert_eq!(report.categories.len(), 2);
        let coordination = report
            .categories
            .iter()
            .find(|category| category.category_id == "coordination")
            .unwrap();
        assert_eq!(coordination.baseline_total_minor, 110_000);
        assert_eq!(coordination.treatment_total_minor, 108_000);
        assert_eq!(coordination.gross_benefit_minor, 2_000);
        let fuel = report
            .categories
            .iter()
            .find(|category| category.category_id == "fuel")
            .unwrap();
        assert_eq!(fuel.gross_benefit_minor, 4_000);
        assert_eq!(
            report
                .participant_components
                .iter()
                .map(|component| component.gross_benefit_minor)
                .sum::<i128>(),
            6_000
        );
    }

    #[test]
    fn missing_rows_are_not_silently_treated_as_zero() {
        let mut case = input();
        case.components.retain(|item| {
            item.participant_id != "SHIP-OWNER" || item.category_id != "coordination"
        });
        assert_eq!(
            evaluate_component_ledger(&case),
            Err(CostLedgerError::MissingComponent {
                participant_id: "SHIP-OWNER".to_owned(),
                category_id: "coordination".to_owned(),
            })
        );
    }

    #[test]
    fn duplicate_and_unknown_category_rows_reject() {
        let mut duplicate = input();
        duplicate.components.push(duplicate.components[0].clone());
        assert_eq!(
            evaluate_component_ledger(&duplicate),
            Err(CostLedgerError::DuplicateComponent {
                participant_id: "PORT-OPS".to_owned(),
                category_id: "fuel".to_owned(),
            })
        );

        let mut unknown = input();
        unknown.components.push(row("PORT-OPS", "unmanifested", 0, 0));
        assert_eq!(
            evaluate_component_ledger(&unknown),
            Err(CostLedgerError::UnknownCategory {
                category_id: "unmanifested".to_owned(),
            })
        );
    }

    #[test]
    fn duplicate_category_and_participant_contract_entries_reject() {
        let mut duplicate_category = input();
        duplicate_category.required_category_ids.push("fuel".to_owned());
        assert_eq!(
            evaluate_component_ledger(&duplicate_category),
            Err(CostLedgerError::DuplicateCategory("fuel".to_owned()))
        );

        let mut duplicate_participant = input();
        let duplicate_terms = duplicate_participant.participants[0].clone();
        duplicate_participant.participants.push(duplicate_terms);
        assert_eq!(
            evaluate_component_ledger(&duplicate_participant),
            Err(CostLedgerError::DuplicateParticipant("PORT-OPS".to_owned()))
        );
    }

    #[test]
    fn all_zero_category_still_requires_an_explicit_zero_row() {
        let mut case = input();
        for item in &mut case.components {
            if item.category_id == "coordination" {
                item.baseline_cost_minor = 0;
                item.treatment_cost_minor = 0;
            }
        }
        assert!(evaluate_component_ledger(&case).is_ok());

        case.components.retain(|item| item.category_id != "coordination");
        assert!(matches!(
            evaluate_component_ledger(&case),
            Err(CostLedgerError::MissingComponent { .. })
        ));
    }

    #[test]
    fn component_ledger_is_input_order_invariant() {
        let mut first = input();
        let mut second = input();
        first.components.reverse();
        first.participants.reverse();
        second.required_category_ids.reverse();
        second.components.rotate_left(2);
        let a = evaluate_component_ledger(&first).unwrap().to_json();
        let b = evaluate_component_ledger(&second).unwrap().to_json();
        assert_eq!(a, b);
    }

    #[test]
    fn aggregate_outside_supported_account_range_rejects() {
        let mut case = input();
        case.required_category_ids = vec!["one".to_owned(), "two".to_owned()];
        case.components = vec![
            row("PORT-OPS", "one", i64::MAX, i64::MAX),
            row("PORT-OPS", "two", 1, 0),
        ];
        case.participants.truncate(1);
        assert_eq!(
            evaluate_component_ledger(&case),
            Err(CostLedgerError::AggregateOutsideI64 {
                participant_id: "PORT-OPS".to_owned(),
                basis: "baseline",
            })
        );
    }

    #[test]
    fn malformed_manifest_digest_and_unqualified_evidence_fail_closed() {
        let mut bad_digest = input();
        bad_digest.category_manifest_sha256 = "not-a-digest".to_owned();
        assert_eq!(
            evaluate_component_ledger(&bad_digest),
            Err(CostLedgerError::ValueShare(ValueShareError::InvalidSha256 {
                field: "category_manifest_sha256",
            }))
        );

        let mut unqualified = input();
        unqualified.evidence_tier = EvidenceTier::Unqualified;
        assert_eq!(
            evaluate_component_ledger(&unqualified),
            Err(CostLedgerError::ValueShare(ValueShareError::UnqualifiedEvidence))
        );
    }
}
