#![forbid(unsafe_code)]

//! Pure arithmetic checks for synthetic port-call counterfactual accounts.
//!
//! This crate verifies a proposed allocation; it does not produce a contract,
//! negotiate with participants, authorize operations, or move money. Monetary
//! costs are net account values in a single declared minor currency unit.
//! Fuel and emissions remain separate physical quantities, not hidden money.

use std::collections::BTreeSet;
use std::fmt::Write as _;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceTier {
    Synthetic,
    QualifiedHistoricalReplay,
    Unqualified,
}

impl EvidenceTier {
    fn as_str(self) -> &'static str {
        match self {
            Self::Synthetic => "synthetic",
            Self::QualifiedHistoricalReplay => "qualified_historical_replay",
            Self::Unqualified => "unqualified",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CounterfactualBasis {
    pub case_id: String,
    pub currency_code: String,
    pub evidence_tier: EvidenceTier,
    /// A reviewed, common service/horizon contract must apply to both cases.
    pub same_delivered_service: bool,
    /// Every in-scope cost/benefit account must use the same declared boundary.
    pub cost_boundary_complete: bool,
    /// Digests identify the frozen source receipts; they do not establish truth.
    pub baseline_receipt_sha256: String,
    pub treatment_receipt_sha256: String,
    pub comparison_contract_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParticipantAccount {
    pub participant_id: String,
    /// Net cost in currency minor units before any coordination transfer.
    /// Negative values are allowed when net revenue exceeds the included costs.
    pub baseline_net_cost_minor: i64,
    pub treatment_net_cost_before_transfer_minor: i64,
    /// Positive means this participant receives value; negative means it pays.
    pub transfer_in_minor: i64,
    /// Declared acceptance threshold, not inferred from behavior or identity.
    pub minimum_acceptable_benefit_minor: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScenarioInput {
    pub basis: CounterfactualBasis,
    /// Physical mass accounting in grams. Not converted to money by this crate.
    pub baseline_co2_grams: u64,
    pub treatment_co2_grams: u64,
    pub participants: Vec<ParticipantAccount>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValueShareError {
    InvalidIdentifier { kind: &'static str, value: String },
    InvalidCurrencyCode(String),
    InvalidSha256 { field: &'static str },
    IdenticalCounterfactualReceipts,
    UnqualifiedEvidence,
    DifferentDeliveredService,
    IncompleteCostBoundary,
    TooFewParticipants,
    DuplicateParticipant(String),
    TransfersDoNotConserve { net_transfer_minor: i128 },
    ArithmeticOverflow,
}

impl std::fmt::Display for ValueShareError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ValueShareError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParticipantOutcome {
    pub participant_id: String,
    pub gross_benefit_minor: i128,
    pub transfer_in_minor: i64,
    pub net_benefit_minor: i128,
    pub minimum_acceptable_benefit_minor: i64,
    pub meets_reservation: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValueShareReport {
    pub case_id: String,
    pub currency_code: String,
    pub evidence_tier: EvidenceTier,
    pub baseline_co2_grams: u64,
    pub treatment_co2_grams: u64,
    /// Positive indicates a reduction; negative indicates an increase.
    pub emissions_reduction_grams: i128,
    pub total_gross_benefit_minor: i128,
    pub net_transfer_minor: i128,
    pub all_reservations_met: bool,
    /// Sorted by participant ID for canonical receipt output.
    pub participants: Vec<ParticipantOutcome>,
}

impl ValueShareReport {
    /// Canonical JSON with deterministic field and row order. All textual values
    /// are validated identifiers, a three-letter currency code, or fixed enums.
    pub fn to_json(&self) -> String {
        let mut output = String::new();
        write!(
            &mut output,
            "{{\"case_id\":\"{}\",\"currency_code\":\"{}\",\"evidence_tier\":\"{}\",\"baseline_co2_grams\":{},\"treatment_co2_grams\":{},\"emissions_reduction_grams\":{},\"total_gross_benefit_minor\":{},\"net_transfer_minor\":{},\"all_reservations_met\":{},\"participants\":[",
            self.case_id,
            self.currency_code,
            self.evidence_tier.as_str(),
            self.baseline_co2_grams,
            self.treatment_co2_grams,
            self.emissions_reduction_grams,
            self.total_gross_benefit_minor,
            self.net_transfer_minor,
            self.all_reservations_met
        )
        .expect("writing into String cannot fail");

        for (index, participant) in self.participants.iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            write!(
                &mut output,
                "{{\"id\":\"{}\",\"gross_benefit_minor\":{},\"transfer_in_minor\":{},\"net_benefit_minor\":{},\"minimum_acceptable_benefit_minor\":{},\"meets_reservation\":{}}}",
                participant.participant_id,
                participant.gross_benefit_minor,
                participant.transfer_in_minor,
                participant.net_benefit_minor,
                participant.minimum_acceptable_benefit_minor,
                participant.meets_reservation
            )
            .expect("writing into String cannot fail");
        }

        output.push_str("]}");
        output
    }
}

fn validate_identifier(kind: &'static str, value: &str) -> Result<(), ValueShareError> {
    let allowed = value.bytes().all(|byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-')
    });
    if value.is_empty() || !allowed {
        return Err(ValueShareError::InvalidIdentifier {
            kind,
            value: value.to_owned(),
        });
    }
    Ok(())
}

fn validate_sha256(field: &'static str, digest: &str) -> Result<(), ValueShareError> {
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ValueShareError::InvalidSha256 { field });
    }
    Ok(())
}

fn validate_basis(basis: &CounterfactualBasis) -> Result<(), ValueShareError> {
    validate_identifier("case", &basis.case_id)?;

    if basis.currency_code.len() != 3
        || !basis.currency_code.bytes().all(|byte| byte.is_ascii_uppercase())
    {
        return Err(ValueShareError::InvalidCurrencyCode(
            basis.currency_code.clone(),
        ));
    }

    validate_sha256("baseline_receipt_sha256", &basis.baseline_receipt_sha256)?;
    validate_sha256("treatment_receipt_sha256", &basis.treatment_receipt_sha256)?;
    validate_sha256("comparison_contract_sha256", &basis.comparison_contract_sha256)?;

    if basis.baseline_receipt_sha256 == basis.treatment_receipt_sha256 {
        return Err(ValueShareError::IdenticalCounterfactualReceipts);
    }
    if basis.evidence_tier == EvidenceTier::Unqualified {
        return Err(ValueShareError::UnqualifiedEvidence);
    }
    if !basis.same_delivered_service {
        return Err(ValueShareError::DifferentDeliveredService);
    }
    if !basis.cost_boundary_complete {
        return Err(ValueShareError::IncompleteCostBoundary);
    }

    Ok(())
}

/// Validate one proposed multi-party allocation and return a deterministic
/// report. This function never creates or executes a transfer.
pub fn evaluate_value_share(input: &ScenarioInput) -> Result<ValueShareReport, ValueShareError> {
    validate_basis(&input.basis)?;
    if input.participants.len() < 2 {
        return Err(ValueShareError::TooFewParticipants);
    }

    let mut participant_ids = BTreeSet::new();
    let mut net_transfer_minor = 0_i128;
    let mut total_gross_benefit_minor = 0_i128;
    let mut participants = Vec::with_capacity(input.participants.len());

    for account in &input.participants {
        validate_identifier("participant", &account.participant_id)?;
        if !participant_ids.insert(account.participant_id.clone()) {
            return Err(ValueShareError::DuplicateParticipant(
                account.participant_id.clone(),
            ));
        }

        let gross_benefit_minor = i128::from(account.baseline_net_cost_minor)
            - i128::from(account.treatment_net_cost_before_transfer_minor);
        let net_benefit_minor = gross_benefit_minor
            .checked_add(i128::from(account.transfer_in_minor))
            .ok_or(ValueShareError::ArithmeticOverflow)?;

        total_gross_benefit_minor = total_gross_benefit_minor
            .checked_add(gross_benefit_minor)
            .ok_or(ValueShareError::ArithmeticOverflow)?;
        net_transfer_minor = net_transfer_minor
            .checked_add(i128::from(account.transfer_in_minor))
            .ok_or(ValueShareError::ArithmeticOverflow)?;

        participants.push(ParticipantOutcome {
            participant_id: account.participant_id.clone(),
            gross_benefit_minor,
            transfer_in_minor: account.transfer_in_minor,
            net_benefit_minor,
            minimum_acceptable_benefit_minor: account.minimum_acceptable_benefit_minor,
            meets_reservation: net_benefit_minor
                >= i128::from(account.minimum_acceptable_benefit_minor),
        });
    }

    if net_transfer_minor != 0 {
        return Err(ValueShareError::TransfersDoNotConserve { net_transfer_minor });
    }

    participants.sort_by_key(|item| item.participant_id.clone());
    let all_reservations_met = participants.iter().all(|item| item.meets_reservation);
    let emissions_reduction_grams =
        i128::from(input.baseline_co2_grams) - i128::from(input.treatment_co2_grams);

    // Transfer conservation implies that system-wide net benefit equals gross
    // operating benefit. Check this invariant explicitly rather than trusting the
    // account total or accepting a balancing-plug residual.
    let net_benefit_total = participants.iter().try_fold(0_i128, |total, participant| {
        total
            .checked_add(participant.net_benefit_minor)
            .ok_or(ValueShareError::ArithmeticOverflow)
    })?;
    if net_benefit_total != total_gross_benefit_minor {
        return Err(ValueShareError::ArithmeticOverflow);
    }

    Ok(ValueShareReport {
        case_id: input.basis.case_id.clone(),
        currency_code: input.basis.currency_code.clone(),
        evidence_tier: input.basis.evidence_tier,
        baseline_co2_grams: input.baseline_co2_grams,
        treatment_co2_grams: input.treatment_co2_grams,
        emissions_reduction_grams,
        total_gross_benefit_minor,
        net_transfer_minor,
        all_reservations_met,
        participants,
    })
}

/// Hand-checkable synthetic case with six thousand cents of gross benefit.
/// The allocation compensates a party that otherwise loses while preserving a
/// positive net benefit for the other parties. It is not a field measurement.
pub fn run_demo() -> Result<ValueShareReport, ValueShareError> {
    let input = ScenarioInput {
        basis: CounterfactualBasis {
            case_id: "synthetic-port-call-share-v0.1".to_owned(),
            currency_code: "USD".to_owned(),
            evidence_tier: EvidenceTier::Synthetic,
            same_delivered_service: true,
            cost_boundary_complete: true,
            baseline_receipt_sha256: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .to_owned(),
            treatment_receipt_sha256: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                .to_owned(),
            comparison_contract_sha256:
                "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".to_owned(),
        },
        baseline_co2_grams: 100_000,
        treatment_co2_grams: 90_000,
        participants: vec![
            ParticipantAccount {
                participant_id: "PORT-OPS".to_owned(),
                baseline_net_cost_minor: 100_000,
                treatment_net_cost_before_transfer_minor: 95_000,
                transfer_in_minor: -500,
                minimum_acceptable_benefit_minor: 0,
            },
            ParticipantAccount {
                participant_id: "SHIP-OWNER".to_owned(),
                baseline_net_cost_minor: 120_000,
                treatment_net_cost_before_transfer_minor: 122_000,
                transfer_in_minor: 2_000,
                minimum_acceptable_benefit_minor: 0,
            },
            ParticipantAccount {
                participant_id: "CHARTERER".to_owned(),
                baseline_net_cost_minor: 80_000,
                treatment_net_cost_before_transfer_minor: 77_000,
                transfer_in_minor: -1_500,
                minimum_acceptable_benefit_minor: 0,
            },
        ],
    };

    evaluate_value_share(&input)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn basis() -> CounterfactualBasis {
        CounterfactualBasis {
            case_id: "test-case".to_owned(),
            currency_code: "USD".to_owned(),
            evidence_tier: EvidenceTier::Synthetic,
            same_delivered_service: true,
            cost_boundary_complete: true,
            baseline_receipt_sha256: "a".repeat(64),
            treatment_receipt_sha256: "b".repeat(64),
            comparison_contract_sha256: "c".repeat(64),
        }
    }

    fn account(
        id: &str,
        baseline: i64,
        treatment: i64,
        transfer: i64,
        minimum: i64,
    ) -> ParticipantAccount {
        ParticipantAccount {
            participant_id: id.to_owned(),
            baseline_net_cost_minor: baseline,
            treatment_net_cost_before_transfer_minor: treatment,
            transfer_in_minor: transfer,
            minimum_acceptable_benefit_minor: minimum,
        }
    }

    fn input() -> ScenarioInput {
        ScenarioInput {
            basis: basis(),
            baseline_co2_grams: 100_000,
            treatment_co2_grams: 90_000,
            participants: vec![
                account("PORT-OPS", 100_000, 95_000, -500, 0),
                account("SHIP-OWNER", 120_000, 122_000, 2_000, 0),
                account("CHARTERER", 80_000, 77_000, -1_500, 0),
            ],
        }
    }

    #[test]
    fn demo_matches_frozen_receipt_and_conserves_value() {
        let report = run_demo().unwrap();
        assert_eq!(
            report.to_json(),
            include_str!("../fixtures/value-share-v0.1.expected.json").trim_end()
        );
        assert_eq!(report.total_gross_benefit_minor, 6_000);
        assert_eq!(report.net_transfer_minor, 0);
        assert!(report.all_reservations_met);
        assert_eq!(report.emissions_reduction_grams, 10_000);
    }

    #[test]
    fn one_minor_unit_transfer_mismatch_is_rejected() {
        let mut input = input();
        input.participants[0].transfer_in_minor -= 1;
        assert_eq!(
            evaluate_value_share(&input),
            Err(ValueShareError::TransfersDoNotConserve {
                net_transfer_minor: -1,
            })
        );
    }

    #[test]
    fn aggregate_gain_does_not_hide_a_participant_reservation_shortfall() {
        let mut input = input();
        input.participants[0].transfer_in_minor = -400;
        input.participants[1].transfer_in_minor = 1_900;
        let report = evaluate_value_share(&input).unwrap();
        assert_eq!(report.total_gross_benefit_minor, 6_000);
        assert!(!report.all_reservations_met);
        let owner = report
            .participants
            .iter()
            .find(|item| item.participant_id == "SHIP-OWNER")
            .unwrap();
        assert_eq!(owner.net_benefit_minor, -100);
        assert!(!owner.meets_reservation);
    }

    #[test]
    fn service_and_cost_boundary_are_hard_gates() {
        let mut service_mismatch = input();
        service_mismatch.basis.same_delivered_service = false;
        assert_eq!(
            evaluate_value_share(&service_mismatch),
            Err(ValueShareError::DifferentDeliveredService)
        );

        let mut incomplete_costs = input();
        incomplete_costs.basis.cost_boundary_complete = false;
        assert_eq!(
            evaluate_value_share(&incomplete_costs),
            Err(ValueShareError::IncompleteCostBoundary)
        );
    }

    #[test]
    fn unqualified_evidence_and_invalid_digests_are_rejected() {
        let mut unqualified = input();
        unqualified.basis.evidence_tier = EvidenceTier::Unqualified;
        assert_eq!(
            evaluate_value_share(&unqualified),
            Err(ValueShareError::UnqualifiedEvidence)
        );

        let mut invalid_digest = input();
        invalid_digest.basis.treatment_receipt_sha256 = "not-a-digest".to_owned();
        assert_eq!(
            evaluate_value_share(&invalid_digest),
            Err(ValueShareError::InvalidSha256 {
                field: "treatment_receipt_sha256",
            })
        );
    }

    #[test]
    fn duplicate_participants_and_identical_receipts_are_rejected() {
        let mut duplicate = input();
        duplicate.participants[1].participant_id = duplicate.participants[0].participant_id.clone();
        assert_eq!(
            evaluate_value_share(&duplicate),
            Err(ValueShareError::DuplicateParticipant("PORT-OPS".to_owned()))
        );

        let mut same_receipt = input();
        same_receipt.basis.treatment_receipt_sha256 =
            same_receipt.basis.baseline_receipt_sha256.clone();
        assert_eq!(
            evaluate_value_share(&same_receipt),
            Err(ValueShareError::IdenticalCounterfactualReceipts)
        );
    }

    #[test]
    fn participant_row_permutations_produce_identical_canonical_output() {
        let mut first = input();
        let mut second = input();
        second.participants.reverse();
        let first_report = evaluate_value_share(&first).unwrap();
        let second_report = evaluate_value_share(&second).unwrap();
        assert_eq!(first_report.to_json(), second_report.to_json());
        first.participants.sort_by_key(|item| item.participant_id.clone());
        assert_eq!(
            evaluate_value_share(&first).unwrap().to_json(),
            first_report.to_json()
        );
    }

    #[test]
    fn physical_emissions_remain_separate_even_when_the_treatment_increases_them() {
        let mut input = input();
        input.treatment_co2_grams = 105_000;
        let report = evaluate_value_share(&input).unwrap();
        assert_eq!(report.total_gross_benefit_minor, 6_000);
        assert_eq!(report.emissions_reduction_grams, -5_000);
    }

    #[test]
    fn accepts_signed_net_costs_and_uses_wide_checked_account_arithmetic() {
        let mut input = input();
        input.participants = vec![
            account("A", i64::MAX, i64::MIN, -1, i64::MIN),
            account("B", 0, 0, 1, i64::MIN),
        ];
        let report = evaluate_value_share(&input).unwrap();
        assert_eq!(
            report.total_gross_benefit_minor,
            i128::from(i64::MAX) - i128::from(i64::MIN)
        );
        assert_eq!(report.net_transfer_minor, 0);
    }

    #[test]
    fn too_few_participants_and_bad_currency_codes_are_rejected() {
        let mut too_few = input();
        too_few.participants.truncate(1);
        assert_eq!(
            evaluate_value_share(&too_few),
            Err(ValueShareError::TooFewParticipants)
        );

        let mut bad_currency = input();
        bad_currency.basis.currency_code = "usd".to_owned();
        assert_eq!(
            evaluate_value_share(&bad_currency),
            Err(ValueShareError::InvalidCurrencyCode("usd".to_owned()))
        );
    }
}
