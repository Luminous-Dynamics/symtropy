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
    /// Caller assertion that every in-scope account uses the same boundary.
    /// This v0.1 evaluator rejects false, but does not audit line-item completeness.
    pub cost_boundary_complete: bool,
    /// Caller-supplied source receipt digests. Syntax is checked here; an upstream
    /// receipt verifier must prove the digests match source bytes. They do not prove truth.
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
    /// Physical consumption quantities in grams, separate from currency accounts.
    pub baseline_fuel_grams: u64,
    pub treatment_fuel_grams: u64,
    /// CO2 mass in grams. Not converted to money by this crate.
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
    EmptySensitivityEnsemble,
    TooFewBaselines,
    DuplicateBaselineReceipt(String),
    InconsistentSensitivityBasis { field: &'static str },
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
    pub baseline_receipt_sha256: String,
    pub treatment_receipt_sha256: String,
    pub comparison_contract_sha256: String,
    pub baseline_fuel_grams: u64,
    pub treatment_fuel_grams: u64,
    /// Positive indicates a reduction; negative indicates an increase.
    pub fuel_reduction_grams: i128,
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
            "{{\"case_id\":\"{}\",\"currency_code\":\"{}\",\"evidence_tier\":\"{}\",\"baseline_receipt_sha256\":\"{}\",\"treatment_receipt_sha256\":\"{}\",\"comparison_contract_sha256\":\"{}\",\"baseline_fuel_grams\":{},\"treatment_fuel_grams\":{},\"fuel_reduction_grams\":{},\"baseline_co2_grams\":{},\"treatment_co2_grams\":{},\"emissions_reduction_grams\":{},\"total_gross_benefit_minor\":{},\"net_transfer_minor\":{},\"all_reservations_met\":{},\"participants\":[",
            self.case_id,
            self.currency_code,
            self.evidence_tier.as_str(),
            self.baseline_receipt_sha256,
            self.treatment_receipt_sha256,
            self.comparison_contract_sha256,
            self.baseline_fuel_grams,
            self.treatment_fuel_grams,
            self.fuel_reduction_grams,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BaselineSensitivityOutcome {
    pub baseline_receipt_sha256: String,
    pub total_gross_benefit_minor: i128,
    pub fuel_reduction_grams: i128,
    pub emissions_reduction_grams: i128,
    pub all_reservations_met: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParticipantSensitivityOutcome {
    pub participant_id: String,
    pub worst_case_net_benefit_minor: i128,
    pub best_case_net_benefit_minor: i128,
    pub minimum_acceptable_benefit_minor: i64,
    pub meets_reservation_in_all_baselines: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CounterfactualSensitivityReport {
    pub case_id: String,
    pub currency_code: String,
    pub evidence_tier: EvidenceTier,
    pub treatment_receipt_sha256: String,
    pub comparison_contract_sha256: String,
    pub baseline_count: usize,
    pub worst_case_total_gross_benefit_minor: i128,
    pub best_case_total_gross_benefit_minor: i128,
    pub worst_case_fuel_reduction_grams: i128,
    pub best_case_fuel_reduction_grams: i128,
    pub worst_case_emissions_reduction_grams: i128,
    pub best_case_emissions_reduction_grams: i128,
    pub all_reservations_met_in_all_baselines: bool,
    pub baselines: Vec<BaselineSensitivityOutcome>,
    pub participants: Vec<ParticipantSensitivityOutcome>,
}

impl CounterfactualSensitivityReport {
    /// Canonical JSON with baseline receipts and participant ranges sorted.
    /// Scenario counts are not probabilities and no expected value is inferred.
    pub fn to_json(&self) -> String {
        let mut output = String::new();
        write!(
            &mut output,
            "{{\"case_id\":\"{}\",\"currency_code\":\"{}\",\"evidence_tier\":\"{}\",\"treatment_receipt_sha256\":\"{}\",\"comparison_contract_sha256\":\"{}\",\"baseline_count\":{},\"worst_case_total_gross_benefit_minor\":{},\"best_case_total_gross_benefit_minor\":{},\"worst_case_fuel_reduction_grams\":{},\"best_case_fuel_reduction_grams\":{},\"worst_case_emissions_reduction_grams\":{},\"best_case_emissions_reduction_grams\":{},\"all_reservations_met_in_all_baselines\":{},\"baselines\":[",
            self.case_id,
            self.currency_code,
            self.evidence_tier.as_str(),
            self.treatment_receipt_sha256,
            self.comparison_contract_sha256,
            self.baseline_count,
            self.worst_case_total_gross_benefit_minor,
            self.best_case_total_gross_benefit_minor,
            self.worst_case_fuel_reduction_grams,
            self.best_case_fuel_reduction_grams,
            self.worst_case_emissions_reduction_grams,
            self.best_case_emissions_reduction_grams,
            self.all_reservations_met_in_all_baselines
        )
        .expect("writing into String cannot fail");

        for (index, baseline) in self.baselines.iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            write!(
                &mut output,
                "{{\"baseline_receipt_sha256\":\"{}\",\"total_gross_benefit_minor\":{},\"fuel_reduction_grams\":{},\"emissions_reduction_grams\":{},\"all_reservations_met\":{}}}",
                baseline.baseline_receipt_sha256,
                baseline.total_gross_benefit_minor,
                baseline.fuel_reduction_grams,
                baseline.emissions_reduction_grams,
                baseline.all_reservations_met
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
                "{{\"id\":\"{}\",\"worst_case_net_benefit_minor\":{},\"best_case_net_benefit_minor\":{},\"minimum_acceptable_benefit_minor\":{},\"meets_reservation_in_all_baselines\":{}}}",
                participant.participant_id,
                participant.worst_case_net_benefit_minor,
                participant.best_case_net_benefit_minor,
                participant.minimum_acceptable_benefit_minor,
                participant.meets_reservation_in_all_baselines
            )
            .expect("writing into String cannot fail");
        }

        output.push_str("]}");
        output
    }
}

fn validate_identifier(kind: &'static str, value: &str) -> Result<(), ValueShareError> {
    let allowed = value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'));
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
        || !basis
            .currency_code
            .bytes()
            .all(|byte| byte.is_ascii_uppercase())
    {
        return Err(ValueShareError::InvalidCurrencyCode(
            basis.currency_code.clone(),
        ));
    }

    validate_sha256("baseline_receipt_sha256", &basis.baseline_receipt_sha256)?;
    validate_sha256("treatment_receipt_sha256", &basis.treatment_receipt_sha256)?;
    validate_sha256(
        "comparison_contract_sha256",
        &basis.comparison_contract_sha256,
    )?;

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
    let fuel_reduction_grams =
        i128::from(input.baseline_fuel_grams) - i128::from(input.treatment_fuel_grams);
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
        baseline_receipt_sha256: input.basis.baseline_receipt_sha256.clone(),
        treatment_receipt_sha256: input.basis.treatment_receipt_sha256.clone(),
        comparison_contract_sha256: input.basis.comparison_contract_sha256.clone(),
        baseline_fuel_grams: input.baseline_fuel_grams,
        treatment_fuel_grams: input.treatment_fuel_grams,
        fuel_reduction_grams,
        baseline_co2_grams: input.baseline_co2_grams,
        treatment_co2_grams: input.treatment_co2_grams,
        emissions_reduction_grams,
        total_gross_benefit_minor,
        net_transfer_minor,
        all_reservations_met,
        participants,
    })
}

/// Compare multiple explicit business-as-usual baselines against one fixed
/// treatment, comparison contract, participant set, and transfer proposal.
/// Each baseline gets a separate deterministic report; no scenario probability
/// or expected saving is invented. Baseline receipts must be distinct.
pub fn evaluate_counterfactual_sensitivity(
    inputs: &[ScenarioInput],
) -> Result<CounterfactualSensitivityReport, ValueShareError> {
    if inputs.is_empty() {
        return Err(ValueShareError::EmptySensitivityEnsemble);
    }
    if inputs.len() < 2 {
        return Err(ValueShareError::TooFewBaselines);
    }

    let mut reports = Vec::with_capacity(inputs.len());
    for input in inputs {
        reports.push(evaluate_value_share(input)?);
    }

    let reference = &inputs[0];
    let reference_participants = reports[0].participants.clone();
    let mut baseline_receipts = BTreeSet::new();
    for input in inputs {
        if !baseline_receipts.insert(input.basis.baseline_receipt_sha256.clone()) {
            return Err(ValueShareError::DuplicateBaselineReceipt(
                input.basis.baseline_receipt_sha256.clone(),
            ));
        }
        if input.basis.case_id != reference.basis.case_id {
            return Err(ValueShareError::InconsistentSensitivityBasis { field: "case_id" });
        }
        if input.basis.currency_code != reference.basis.currency_code {
            return Err(ValueShareError::InconsistentSensitivityBasis {
                field: "currency_code",
            });
        }
        if input.basis.evidence_tier != reference.basis.evidence_tier {
            return Err(ValueShareError::InconsistentSensitivityBasis {
                field: "evidence_tier",
            });
        }
        if input.basis.treatment_receipt_sha256 != reference.basis.treatment_receipt_sha256 {
            return Err(ValueShareError::InconsistentSensitivityBasis {
                field: "treatment_receipt_sha256",
            });
        }
        if input.basis.comparison_contract_sha256 != reference.basis.comparison_contract_sha256 {
            return Err(ValueShareError::InconsistentSensitivityBasis {
                field: "comparison_contract_sha256",
            });
        }
        if input.treatment_fuel_grams != reference.treatment_fuel_grams {
            return Err(ValueShareError::InconsistentSensitivityBasis {
                field: "treatment_fuel_grams",
            });
        }
        if input.treatment_co2_grams != reference.treatment_co2_grams {
            return Err(ValueShareError::InconsistentSensitivityBasis {
                field: "treatment_co2_grams",
            });
        }

        let mut candidate_accounts = input.participants.iter().collect::<Vec<_>>();
        candidate_accounts.sort_by_key(|account| account.participant_id.clone());
        let mut reference_accounts = reference.participants.iter().collect::<Vec<_>>();
        reference_accounts.sort_by_key(|account| account.participant_id.clone());
        if candidate_accounts.len() != reference_accounts.len() {
            return Err(ValueShareError::InconsistentSensitivityBasis {
                field: "participant_count",
            });
        }

        for (candidate, expected) in candidate_accounts.iter().zip(&reference_accounts) {
            if candidate.participant_id != expected.participant_id {
                return Err(ValueShareError::InconsistentSensitivityBasis {
                    field: "participant_ids",
                });
            }
            if candidate.treatment_net_cost_before_transfer_minor
                != expected.treatment_net_cost_before_transfer_minor
            {
                return Err(ValueShareError::InconsistentSensitivityBasis {
                    field: "treatment_net_cost_before_transfer_minor",
                });
            }
            if candidate.transfer_in_minor != expected.transfer_in_minor {
                return Err(ValueShareError::InconsistentSensitivityBasis {
                    field: "transfer_in_minor",
                });
            }
            if candidate.minimum_acceptable_benefit_minor
                != expected.minimum_acceptable_benefit_minor
            {
                return Err(ValueShareError::InconsistentSensitivityBasis {
                    field: "minimum_acceptable_benefit_minor",
                });
            }
        }
    }

    reports.sort_by_key(|report| report.baseline_receipt_sha256.clone());
    let baselines = reports
        .iter()
        .map(|report| BaselineSensitivityOutcome {
            baseline_receipt_sha256: report.baseline_receipt_sha256.clone(),
            total_gross_benefit_minor: report.total_gross_benefit_minor,
            fuel_reduction_grams: report.fuel_reduction_grams,
            emissions_reduction_grams: report.emissions_reduction_grams,
            all_reservations_met: report.all_reservations_met,
        })
        .collect::<Vec<_>>();

    let mut participants = Vec::with_capacity(reference_participants.len());
    for reference_participant in &reference_participants {
        let outcomes = reports
            .iter()
            .map(|report| {
                report
                    .participants
                    .iter()
                    .find(|item| item.participant_id == reference_participant.participant_id)
                    .expect("participant sets were validated")
            })
            .collect::<Vec<_>>();
        let worst_case_net_benefit_minor = outcomes
            .iter()
            .map(|outcome| outcome.net_benefit_minor)
            .min()
            .expect("sensitivity ensemble is non-empty");
        let best_case_net_benefit_minor = outcomes
            .iter()
            .map(|outcome| outcome.net_benefit_minor)
            .max()
            .expect("sensitivity ensemble is non-empty");
        let meets_reservation_in_all_baselines =
            outcomes.iter().all(|outcome| outcome.meets_reservation);
        participants.push(ParticipantSensitivityOutcome {
            participant_id: reference_participant.participant_id.clone(),
            worst_case_net_benefit_minor,
            best_case_net_benefit_minor,
            minimum_acceptable_benefit_minor: reference_participant
                .minimum_acceptable_benefit_minor,
            meets_reservation_in_all_baselines,
        });
    }

    let worst_case_total_gross_benefit_minor = baselines
        .iter()
        .map(|baseline| baseline.total_gross_benefit_minor)
        .min()
        .expect("sensitivity ensemble is non-empty");
    let best_case_total_gross_benefit_minor = baselines
        .iter()
        .map(|baseline| baseline.total_gross_benefit_minor)
        .max()
        .expect("sensitivity ensemble is non-empty");
    let worst_case_fuel_reduction_grams = baselines
        .iter()
        .map(|baseline| baseline.fuel_reduction_grams)
        .min()
        .expect("sensitivity ensemble is non-empty");
    let best_case_fuel_reduction_grams = baselines
        .iter()
        .map(|baseline| baseline.fuel_reduction_grams)
        .max()
        .expect("sensitivity ensemble is non-empty");
    let worst_case_emissions_reduction_grams = baselines
        .iter()
        .map(|baseline| baseline.emissions_reduction_grams)
        .min()
        .expect("sensitivity ensemble is non-empty");
    let best_case_emissions_reduction_grams = baselines
        .iter()
        .map(|baseline| baseline.emissions_reduction_grams)
        .max()
        .expect("sensitivity ensemble is non-empty");

    Ok(CounterfactualSensitivityReport {
        case_id: reference.basis.case_id.clone(),
        currency_code: reference.basis.currency_code.clone(),
        evidence_tier: reference.basis.evidence_tier,
        treatment_receipt_sha256: reference.basis.treatment_receipt_sha256.clone(),
        comparison_contract_sha256: reference.basis.comparison_contract_sha256.clone(),
        baseline_count: baselines.len(),
        worst_case_total_gross_benefit_minor,
        best_case_total_gross_benefit_minor,
        worst_case_fuel_reduction_grams,
        best_case_fuel_reduction_grams,
        worst_case_emissions_reduction_grams,
        best_case_emissions_reduction_grams,
        all_reservations_met_in_all_baselines: participants
            .iter()
            .all(|participant| participant.meets_reservation_in_all_baselines),
        baselines,
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
            baseline_receipt_sha256:
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned(),
            treatment_receipt_sha256:
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_owned(),
            comparison_contract_sha256:
                "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".to_owned(),
        },
        baseline_fuel_grams: 100_000,
        treatment_fuel_grams: 90_000,
        baseline_co2_grams: 316_000,
        treatment_co2_grams: 284_400,
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
            baseline_fuel_grams: 100_000,
            treatment_fuel_grams: 90_000,
            baseline_co2_grams: 316_000,
            treatment_co2_grams: 284_400,
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
        assert_eq!(report.fuel_reduction_grams, 10_000);
        assert_eq!(report.emissions_reduction_grams, 31_600);
        assert_eq!(
            report.baseline_receipt_sha256,
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        );
        assert_eq!(
            report.treatment_receipt_sha256,
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
        );
        assert_eq!(
            report.comparison_contract_sha256,
            "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
        );
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
        let duplicate_id = duplicate.participants[0].participant_id.clone();
        duplicate.participants[1].participant_id = duplicate_id;
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
        first
            .participants
            .sort_by_key(|item| item.participant_id.clone());
        assert_eq!(
            evaluate_value_share(&first).unwrap().to_json(),
            first_report.to_json()
        );
    }

    #[test]
    fn physical_emissions_remain_separate_even_when_the_treatment_increases_them() {
        let mut input = input();
        input.treatment_co2_grams = 330_000;
        let report = evaluate_value_share(&input).unwrap();
        assert_eq!(report.total_gross_benefit_minor, 6_000);
        assert_eq!(report.fuel_reduction_grams, 10_000);
        assert_eq!(report.emissions_reduction_grams, -14_000);
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
    #[test]
    fn bounded_transfer_grid_matches_independent_accounting_oracle() {
        let mut configurations = 0usize;
        for transfer_a in -2_i64..=2 {
            for transfer_b in -2_i64..=2 {
                let transfer_c = -transfer_a - transfer_b;
                let case = ScenarioInput {
                    basis: basis(),
                    baseline_fuel_grams: 100,
                    treatment_fuel_grams: 100,
                    baseline_co2_grams: 300,
                    treatment_co2_grams: 300,
                    participants: vec![
                        account("A", 10, 6, transfer_a, 0),
                        account("B", 7, 10, transfer_b, 0),
                        account("C", 8, 6, transfer_c, 0),
                    ],
                };
                let report = evaluate_value_share(&case).unwrap();
                let expected = [
                    ("A", 4_i128 + i128::from(transfer_a)),
                    ("B", -3_i128 + i128::from(transfer_b)),
                    ("C", 2_i128 + i128::from(transfer_c)),
                ];

                assert_eq!(report.total_gross_benefit_minor, 3);
                assert_eq!(report.net_transfer_minor, 0);
                assert_eq!(
                    report.all_reservations_met,
                    expected.iter().all(|(_, benefit)| *benefit >= 0)
                );
                for outcome in &report.participants {
                    let expected_benefit = expected
                        .iter()
                        .find(|(id, _)| *id == outcome.participant_id)
                        .expect("oracle has every fixed participant")
                        .1;
                    assert_eq!(outcome.net_benefit_minor, expected_benefit);
                    assert_eq!(outcome.meets_reservation, expected_benefit >= 0);
                }
                configurations += 1;
            }
        }
        assert_eq!(configurations, 25);
    }


    #[test]
    fn baseline_sensitivity_reports_worst_case_without_inventing_probabilities() {
        let good_baseline = input();
        let mut pessimistic_baseline = input();
        pessimistic_baseline.basis.baseline_receipt_sha256 = "d".repeat(64);
        pessimistic_baseline.participants[1].baseline_net_cost_minor = 110_000;
        pessimistic_baseline.baseline_fuel_grams = 95_000;
        pessimistic_baseline.baseline_co2_grams = 300_000;

        let report = evaluate_counterfactual_sensitivity(&[
            good_baseline.clone(),
            pessimistic_baseline.clone(),
        ])
        .unwrap();
        assert_eq!(report.baseline_count, 2);
        assert_eq!(report.evidence_tier, EvidenceTier::Synthetic);
        assert_eq!(report.worst_case_total_gross_benefit_minor, -4_000);
        assert_eq!(report.best_case_total_gross_benefit_minor, 6_000);
        assert_eq!(report.worst_case_fuel_reduction_grams, 5_000);
        assert_eq!(report.best_case_fuel_reduction_grams, 10_000);
        assert_eq!(report.worst_case_emissions_reduction_grams, 15_600);
        assert_eq!(report.best_case_emissions_reduction_grams, 31_600);
        assert!(!report.all_reservations_met_in_all_baselines);

        let owner = report
            .participants
            .iter()
            .find(|item| item.participant_id == "SHIP-OWNER")
            .unwrap();
        assert_eq!(owner.worst_case_net_benefit_minor, -10_000);
        assert_eq!(owner.best_case_net_benefit_minor, 0);
        assert!(!owner.meets_reservation_in_all_baselines);
        assert_eq!(report.baselines[0].baseline_receipt_sha256, "a".repeat(64));
        assert_eq!(report.baselines[1].baseline_receipt_sha256, "d".repeat(64));
    }

    #[test]
    fn sensitivity_rejects_changed_treatment_or_duplicate_baseline_receipts() {
        assert_eq!(
            evaluate_counterfactual_sensitivity(&[]),
            Err(ValueShareError::EmptySensitivityEnsemble)
        );
        assert_eq!(
            evaluate_counterfactual_sensitivity(&[input()]),
            Err(ValueShareError::TooFewBaselines)
        );

        let baseline = input();
        let mut changed_treatment = input();
        changed_treatment.basis.baseline_receipt_sha256 = "d".repeat(64);
        changed_treatment.basis.treatment_receipt_sha256 = "e".repeat(64);
        assert_eq!(
            evaluate_counterfactual_sensitivity(&[baseline.clone(), changed_treatment]),
            Err(ValueShareError::InconsistentSensitivityBasis {
                field: "treatment_receipt_sha256",
            })
        );

        let mut duplicate = input();
        duplicate.participants[0].baseline_net_cost_minor += 100;
        assert_eq!(
            evaluate_counterfactual_sensitivity(&[baseline.clone(), duplicate]),
            Err(ValueShareError::DuplicateBaselineReceipt("a".repeat(64)))
        );

        let mut changed_allocation = input();
        changed_allocation.basis.baseline_receipt_sha256 = "d".repeat(64);
        changed_allocation.participants[0].transfer_in_minor = -400;
        changed_allocation.participants[1].transfer_in_minor = 1_900;
        assert_eq!(
            evaluate_counterfactual_sensitivity(&[baseline, changed_allocation]),
            Err(ValueShareError::InconsistentSensitivityBasis {
                field: "transfer_in_minor",
            })
        );
    }

    #[test]
    fn sensitivity_output_is_invariant_to_baseline_and_participant_input_order() {
        let mut first = input();
        let mut second = input();
        second.basis.baseline_receipt_sha256 = "d".repeat(64);
        second.participants[1].baseline_net_cost_minor = 110_000;
        second.baseline_fuel_grams = 95_000;
        second.baseline_co2_grams = 300_000;

        let original =
            evaluate_counterfactual_sensitivity(&[first.clone(), second.clone()]).unwrap();
        first.participants.reverse();
        second.participants.reverse();
        let permuted = evaluate_counterfactual_sensitivity(&[second, first]).unwrap();
        assert_eq!(original.to_json(), permuted.to_json());
    }
}
