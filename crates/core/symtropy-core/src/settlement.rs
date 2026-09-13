// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Atomic monetary settlement above the ECON-02 financial book.
//!
//! ECON-02 deliberately keeps balanced journal accounting and aggregate monetary
//! supply as independent histories. This module adds the first stronger authority:
//! one explicitly coupled issuance/retirement settlement either commits both
//! histories or commits neither.
//!
//! V0.1 remains intentionally narrow. It supports only settlement against one
//! designated Asset account in the same currency. Issuance requires an exact debit
//! to that account; retirement requires an exact credit and sufficient pre-settlement
//! debit balance. The complete journal transaction must have the same total amount
//! as the monetary event, preventing unrelated value from being hidden inside the
//! settlement transaction.
//!
//! `SettlementAuthorizationRef` binds an external authorization-evidence identity.
//! This crate does not cryptographically verify signatures or capabilities; a future
//! Xenia/Mycelix bridge may satisfy that trust edge without weakening this theorem.

use std::collections::BTreeSet;
use std::fmt;

use crate::economic::{ActorId, CausalId};
use crate::financial::{
    AccountNetBalance, FinancialAccountClass, FinancialAccountId, FinancialBook, FinancialError,
    JournalTransaction, MonetaryAuthorityId, MonetarySupplyEvent, PostingSide,
};

const MAX_ID_LEN: usize = 256;
const MAX_SETTLEMENTS: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SettlementId(String);

impl SettlementId {
    pub fn new(value: impl Into<String>) -> Result<Self, SettlementError> {
        let value = value.into();
        validate_id("settlement", &value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SettlementId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Reference to external evidence that the registered monetary authority authorized
/// this exact settlement.
///
/// This is an identity/provenance binding only. V0.1 does not verify a signature,
/// capability token, legal mandate, or remote service response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettlementAuthorizationRef {
    pub monetary_authority_id: MonetaryAuthorityId,
    pub authority_actor_id: ActorId,
    pub evidence_id: CausalId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonetarySettlement {
    pub settlement_id: SettlementId,
    pub transaction: JournalTransaction,
    pub monetary_event: MonetarySupplyEvent,
    /// Exact asset account receiving newly issued claims or surrendering retired
    /// claims. This explicit binding avoids guessing which posting is the settlement
    /// leg when a transaction has several counter-postings.
    pub settlement_account_id: FinancialAccountId,
    pub authorization: SettlementAuthorizationRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettlementLedgerEntry {
    pub sequence: u64,
    pub settlement: MonetarySettlement,
}

/// Stronger financial authority whose mutable state is reachable only through
/// atomic monetary settlements.
///
/// `base_book` freezes the complete pre-settlement ECON-02 state. `entries` are the
/// only transitions after that base inside this authority. `validate()` reconstructs
/// current state by replaying all settlements from the frozen base.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonetarySettlementLedger {
    base_book: FinancialBook,
    current_book: FinancialBook,
    entries: Vec<SettlementLedgerEntry>,
}

impl MonetarySettlementLedger {
    pub fn new(base_book: FinancialBook) -> Result<Self, SettlementError> {
        base_book.validate()?;
        Ok(Self {
            current_book: base_book.clone(),
            base_book,
            entries: Vec::new(),
        })
    }

    pub fn from_history(
        base_book: FinancialBook,
        entries: Vec<SettlementLedgerEntry>,
    ) -> Result<Self, SettlementError> {
        if entries.len() > MAX_SETTLEMENTS {
            return Err(SettlementError::ModelTooLarge);
        }
        base_book.validate()?;
        let current_book = replay_settlements(&base_book, &entries)?;
        Ok(Self {
            base_book,
            current_book,
            entries,
        })
    }

    pub fn validate(&self) -> Result<(), SettlementError> {
        if self.entries.len() > MAX_SETTLEMENTS {
            return Err(SettlementError::ModelTooLarge);
        }
        self.base_book.validate()?;
        self.current_book.validate()?;
        let replayed = replay_settlements(&self.base_book, &self.entries)?;
        if replayed != self.current_book {
            return Err(SettlementError::SettlementHistoryMismatch);
        }
        Ok(())
    }

    pub fn financial_book(&self) -> &FinancialBook {
        &self.current_book
    }

    pub fn base_financial_book(&self) -> &FinancialBook {
        &self.base_book
    }

    pub fn entries(&self) -> &[SettlementLedgerEntry] {
        &self.entries
    }

    /// Atomically apply one settlement.
    ///
    /// Validation and both ECON-02 mutations are executed against a cloned candidate
    /// book. The live current book and settlement history are replaced only after the
    /// balanced journal transaction and the monetary-supply event both succeed.
    pub fn settle(&mut self, settlement: MonetarySettlement) -> Result<(), SettlementError> {
        if self.entries.len() >= MAX_SETTLEMENTS {
            return Err(SettlementError::ModelTooLarge);
        }
        if self
            .entries
            .iter()
            .any(|entry| entry.settlement.settlement_id == settlement.settlement_id)
        {
            return Err(SettlementError::DuplicateSettlement {
                settlement_id: settlement.settlement_id.clone(),
            });
        }

        validate_settlement(&self.current_book, &settlement)?;

        let mut candidate_book = self.current_book.clone();
        apply_settlement_to_book(&mut candidate_book, &settlement)?;
        candidate_book.validate()?;

        let sequence = next_sequence(self.entries.len())?;
        let mut candidate_entries = self.entries.clone();
        candidate_entries.push(SettlementLedgerEntry {
            sequence,
            settlement,
        });

        self.current_book = candidate_book;
        self.entries = candidate_entries;
        Ok(())
    }
}

fn replay_settlements(
    base_book: &FinancialBook,
    entries: &[SettlementLedgerEntry],
) -> Result<FinancialBook, SettlementError> {
    if entries.len() > MAX_SETTLEMENTS {
        return Err(SettlementError::ModelTooLarge);
    }
    let mut book = base_book.clone();
    let mut settlement_ids = BTreeSet::new();

    for (index, entry) in entries.iter().enumerate() {
        let expected = next_sequence(index)?;
        if entry.sequence != expected {
            return Err(SettlementError::InvalidSettlementSequence {
                expected,
                actual: entry.sequence,
            });
        }
        if !settlement_ids.insert(entry.settlement.settlement_id.clone()) {
            return Err(SettlementError::DuplicateSettlement {
                settlement_id: entry.settlement.settlement_id.clone(),
            });
        }
        validate_settlement(&book, &entry.settlement)?;
        apply_settlement_to_book(&mut book, &entry.settlement)?;
    }
    book.validate()?;
    Ok(book)
}

fn validate_settlement(
    book: &FinancialBook,
    settlement: &MonetarySettlement,
) -> Result<(), SettlementError> {
    let event_currency = event_currency_id(&settlement.monetary_event);
    let event_authority = event_authority_id(&settlement.monetary_event);
    let event_cause = event_cause_id(&settlement.monetary_event);
    let event_amount = event_amount(&settlement.monetary_event);

    if &settlement.transaction.currency_id != event_currency {
        return Err(SettlementError::CurrencyMismatch);
    }
    if &settlement.transaction.cause_id != event_cause {
        return Err(SettlementError::CauseMismatch);
    }
    if &settlement.authorization.monetary_authority_id != event_authority {
        return Err(SettlementError::AuthorizationAuthorityMismatch);
    }

    let currency = book
        .currency(event_currency)
        .ok_or_else(|| SettlementError::Financial(FinancialError::UnknownCurrency {
            currency_id: event_currency.clone(),
        }))?;
    if currency.monetary_authority_id != settlement.authorization.monetary_authority_id {
        return Err(SettlementError::AuthorizationAuthorityMismatch);
    }
    if currency.monetary_authority_actor_id != settlement.authorization.authority_actor_id {
        return Err(SettlementError::AuthorizationActorMismatch);
    }

    let account = book
        .account(&settlement.settlement_account_id)
        .ok_or_else(|| SettlementError::Financial(FinancialError::UnknownAccount {
            account_id: settlement.settlement_account_id.clone(),
        }))?;
    if account.currency_id != *event_currency {
        return Err(SettlementError::SettlementAccountCurrencyMismatch);
    }
    if account.class != FinancialAccountClass::Asset {
        return Err(SettlementError::SettlementAccountMustBeAsset {
            account_id: account.account_id.clone(),
        });
    }

    let (required_owner, required_side) = match &settlement.monetary_event {
        MonetarySupplyEvent::Issue {
            beneficiary_actor_id,
            ..
        } => (beneficiary_actor_id, PostingSide::Debit),
        MonetarySupplyEvent::Retire {
            source_actor_id, ..
        } => (source_actor_id, PostingSide::Credit),
    };
    if &account.owner_id != required_owner {
        return Err(SettlementError::SettlementAccountOwnerMismatch);
    }

    let posting = settlement
        .transaction
        .postings
        .iter()
        .find(|posting| posting.account_id == settlement.settlement_account_id)
        .ok_or_else(|| SettlementError::MissingSettlementPosting {
            account_id: settlement.settlement_account_id.clone(),
        })?;
    if posting.side != required_side {
        return Err(SettlementError::WrongSettlementPostingSide {
            expected: required_side,
            actual: posting.side,
        });
    }
    if posting.amount != event_amount {
        return Err(SettlementError::SettlementPostingAmountMismatch {
            event_amount,
            posting_amount: posting.amount,
        });
    }

    let mut debits = 0_u128;
    let mut credits = 0_u128;
    for posting in &settlement.transaction.postings {
        match posting.side {
            PostingSide::Debit => {
                debits = debits
                    .checked_add(u128::from(posting.amount))
                    .ok_or(SettlementError::ArithmeticOverflow)?;
            }
            PostingSide::Credit => {
                credits = credits
                    .checked_add(u128::from(posting.amount))
                    .ok_or(SettlementError::ArithmeticOverflow)?;
            }
        }
    }
    let expected_total = u128::from(event_amount);
    if debits != expected_total || credits != expected_total {
        return Err(SettlementError::TransactionAmountDoesNotEqualMonetaryEvent {
            event_amount,
            debit_total: debits,
            credit_total: credits,
        });
    }

    if matches!(settlement.monetary_event, MonetarySupplyEvent::Retire { .. }) {
        let totals = book
            .account_totals(&settlement.settlement_account_id)
            .ok_or_else(|| SettlementError::Financial(FinancialError::UnknownAccount {
                account_id: settlement.settlement_account_id.clone(),
            }))?;
        let AccountNetBalance { side, amount } = totals.net();
        if side != Some(PostingSide::Debit) || amount < expected_total {
            return Err(SettlementError::InsufficientSettlementAccountClaim {
                available_debit_balance: if side == Some(PostingSide::Debit) {
                    amount
                } else {
                    0
                },
                requested: event_amount,
            });
        }
    }

    Ok(())
}

fn apply_settlement_to_book(
    book: &mut FinancialBook,
    settlement: &MonetarySettlement,
) -> Result<(), SettlementError> {
    // This order is deliberately not itself relied upon for atomicity: callers use
    // a cloned candidate book and commit only the final candidate. The order merely
    // makes duplicate/invalid journal failures surface before monetary mutation.
    book.post_transaction(settlement.transaction.clone())?;
    match &settlement.monetary_event {
        MonetarySupplyEvent::Issue {
            currency_id,
            authority_id,
            amount,
            beneficiary_actor_id,
            cause_id,
        } => book.issue_currency(
            currency_id.clone(),
            authority_id.clone(),
            *amount,
            beneficiary_actor_id.clone(),
            cause_id.clone(),
        )?,
        MonetarySupplyEvent::Retire {
            currency_id,
            authority_id,
            amount,
            source_actor_id,
            cause_id,
        } => book.retire_currency(
            currency_id.clone(),
            authority_id.clone(),
            *amount,
            source_actor_id.clone(),
            cause_id.clone(),
        )?,
    }
    Ok(())
}

fn event_currency_id(event: &MonetarySupplyEvent) -> &crate::financial::CurrencyId {
    match event {
        MonetarySupplyEvent::Issue { currency_id, .. }
        | MonetarySupplyEvent::Retire { currency_id, .. } => currency_id,
    }
}

fn event_authority_id(event: &MonetarySupplyEvent) -> &MonetaryAuthorityId {
    match event {
        MonetarySupplyEvent::Issue { authority_id, .. }
        | MonetarySupplyEvent::Retire { authority_id, .. } => authority_id,
    }
}

fn event_cause_id(event: &MonetarySupplyEvent) -> &CausalId {
    match event {
        MonetarySupplyEvent::Issue { cause_id, .. }
        | MonetarySupplyEvent::Retire { cause_id, .. } => cause_id,
    }
}

fn event_amount(event: &MonetarySupplyEvent) -> u64 {
    match event {
        MonetarySupplyEvent::Issue { amount, .. } | MonetarySupplyEvent::Retire { amount, .. } => {
            *amount
        }
    }
}

fn next_sequence(len: usize) -> Result<u64, SettlementError> {
    u64::try_from(len)
        .map_err(|_| SettlementError::ArithmeticOverflow)?
        .checked_add(1)
        .ok_or(SettlementError::ArithmeticOverflow)
}

fn validate_id(kind: &'static str, value: &str) -> Result<(), SettlementError> {
    if value.is_empty() {
        return Err(SettlementError::EmptyId { kind });
    }
    if value.len() > MAX_ID_LEN {
        return Err(SettlementError::IdTooLong {
            kind,
            max_len: MAX_ID_LEN,
        });
    }
    if value.trim() != value {
        return Err(SettlementError::IdHasSurroundingWhitespace { kind });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettlementError {
    Financial(FinancialError),
    EmptyId {
        kind: &'static str,
    },
    IdTooLong {
        kind: &'static str,
        max_len: usize,
    },
    IdHasSurroundingWhitespace {
        kind: &'static str,
    },
    DuplicateSettlement {
        settlement_id: SettlementId,
    },
    CurrencyMismatch,
    CauseMismatch,
    AuthorizationAuthorityMismatch,
    AuthorizationActorMismatch,
    SettlementAccountCurrencyMismatch,
    SettlementAccountMustBeAsset {
        account_id: FinancialAccountId,
    },
    SettlementAccountOwnerMismatch,
    MissingSettlementPosting {
        account_id: FinancialAccountId,
    },
    WrongSettlementPostingSide {
        expected: PostingSide,
        actual: PostingSide,
    },
    SettlementPostingAmountMismatch {
        event_amount: u64,
        posting_amount: u64,
    },
    TransactionAmountDoesNotEqualMonetaryEvent {
        event_amount: u64,
        debit_total: u128,
        credit_total: u128,
    },
    InsufficientSettlementAccountClaim {
        available_debit_balance: u128,
        requested: u64,
    },
    InvalidSettlementSequence {
        expected: u64,
        actual: u64,
    },
    SettlementHistoryMismatch,
    ArithmeticOverflow,
    ModelTooLarge,
}

impl From<FinancialError> for SettlementError {
    fn from(value: FinancialError) -> Self {
        Self::Financial(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::financial::{
        CurrencyDefinition, CurrencyId, FinancialAccount, FinancialAccountClass,
        JournalTransactionId, Posting,
    };

    fn actor(value: &str) -> ActorId {
        ActorId::new(value).unwrap()
    }

    fn cause(value: &str) -> CausalId {
        CausalId::new(value).unwrap()
    }

    fn currency(value: &str) -> CurrencyId {
        CurrencyId::new(value).unwrap()
    }

    fn authority(value: &str) -> MonetaryAuthorityId {
        MonetaryAuthorityId::new(value).unwrap()
    }

    fn account_id(value: &str) -> FinancialAccountId {
        FinancialAccountId::new(value).unwrap()
    }

    fn tx_id(value: &str) -> JournalTransactionId {
        JournalTransactionId::new(value).unwrap()
    }

    fn settlement_id(value: &str) -> SettlementId {
        SettlementId::new(value).unwrap()
    }

    fn base_book() -> FinancialBook {
        FinancialBook::new(
            vec![CurrencyDefinition {
                currency_id: currency("USD-test"),
                monetary_authority_id: authority("usd-authority"),
                monetary_authority_actor_id: actor("treasury"),
                minor_unit_exponent: 2,
            }],
            vec![
                FinancialAccount {
                    account_id: account_id("alice-cash"),
                    owner_id: actor("alice"),
                    currency_id: currency("USD-test"),
                    class: FinancialAccountClass::Asset,
                },
                FinancialAccount {
                    account_id: account_id("treasury-equity"),
                    owner_id: actor("treasury"),
                    currency_id: currency("USD-test"),
                    class: FinancialAccountClass::Equity,
                },
            ],
        )
        .unwrap()
    }

    fn auth() -> SettlementAuthorizationRef {
        SettlementAuthorizationRef {
            monetary_authority_id: authority("usd-authority"),
            authority_actor_id: actor("treasury"),
            evidence_id: cause("authorization-001"),
        }
    }

    fn issue_settlement(id: &str, amount: u64) -> MonetarySettlement {
        let common_cause = cause(&format!("{id}-cause"));
        MonetarySettlement {
            settlement_id: settlement_id(id),
            transaction: JournalTransaction::new(
                tx_id(&format!("{id}-tx")),
                common_cause.clone(),
                currency("USD-test"),
                vec![
                    Posting {
                        account_id: account_id("alice-cash"),
                        side: PostingSide::Debit,
                        amount,
                    },
                    Posting {
                        account_id: account_id("treasury-equity"),
                        side: PostingSide::Credit,
                        amount,
                    },
                ],
            )
            .unwrap(),
            monetary_event: MonetarySupplyEvent::Issue {
                currency_id: currency("USD-test"),
                authority_id: authority("usd-authority"),
                amount,
                beneficiary_actor_id: actor("alice"),
                cause_id: common_cause,
            },
            settlement_account_id: account_id("alice-cash"),
            authorization: auth(),
        }
    }

    fn retire_settlement(id: &str, amount: u64) -> MonetarySettlement {
        let common_cause = cause(&format!("{id}-cause"));
        MonetarySettlement {
            settlement_id: settlement_id(id),
            transaction: JournalTransaction::new(
                tx_id(&format!("{id}-tx")),
                common_cause.clone(),
                currency("USD-test"),
                vec![
                    Posting {
                        account_id: account_id("alice-cash"),
                        side: PostingSide::Credit,
                        amount,
                    },
                    Posting {
                        account_id: account_id("treasury-equity"),
                        side: PostingSide::Debit,
                        amount,
                    },
                ],
            )
            .unwrap(),
            monetary_event: MonetarySupplyEvent::Retire {
                currency_id: currency("USD-test"),
                authority_id: authority("usd-authority"),
                amount,
                source_actor_id: actor("alice"),
                cause_id: common_cause,
            },
            settlement_account_id: account_id("alice-cash"),
            authorization: auth(),
        }
    }

    #[test]
    fn issuance_commits_journal_and_supply_together() {
        let mut ledger = MonetarySettlementLedger::new(base_book()).unwrap();
        ledger.settle(issue_settlement("issue-1", 100)).unwrap();

        assert_eq!(
            ledger
                .financial_book()
                .monetary_supply(&currency("USD-test")),
            Some(100)
        );
        let totals = ledger
            .financial_book()
            .account_totals(&account_id("alice-cash"))
            .unwrap();
        assert_eq!(totals.net().side, Some(PostingSide::Debit));
        assert_eq!(totals.net().amount, 100);
        assert_eq!(ledger.entries().len(), 1);
        ledger.validate().unwrap();
    }

    #[test]
    fn retirement_requires_existing_debit_claim() {
        let mut ledger = MonetarySettlementLedger::new(base_book()).unwrap();
        let before = ledger.clone();
        let error = ledger
            .settle(retire_settlement("retire-1", 1))
            .unwrap_err();
        assert!(matches!(
            error,
            SettlementError::InsufficientSettlementAccountClaim { .. }
        ));
        assert_eq!(ledger, before);
    }

    #[test]
    fn issue_then_retire_reconciles_both_histories() {
        let mut ledger = MonetarySettlementLedger::new(base_book()).unwrap();
        ledger.settle(issue_settlement("issue-1", 100)).unwrap();
        ledger.settle(retire_settlement("retire-1", 40)).unwrap();

        assert_eq!(
            ledger
                .financial_book()
                .monetary_supply(&currency("USD-test")),
            Some(60)
        );
        let net = ledger
            .financial_book()
            .account_totals(&account_id("alice-cash"))
            .unwrap()
            .net();
        assert_eq!(net.side, Some(PostingSide::Debit));
        assert_eq!(net.amount, 60);
        ledger.validate().unwrap();
    }

    #[test]
    fn mismatched_cause_fails_without_partial_commit() {
        let mut ledger = MonetarySettlementLedger::new(base_book()).unwrap();
        let mut settlement = issue_settlement("issue-1", 100);
        if let MonetarySupplyEvent::Issue { cause_id, .. } = &mut settlement.monetary_event {
            *cause_id = cause("different-cause");
        }
        let before = ledger.clone();
        assert_eq!(ledger.settle(settlement), Err(SettlementError::CauseMismatch));
        assert_eq!(ledger, before);
    }

    #[test]
    fn wrong_authority_actor_fails_closed() {
        let mut ledger = MonetarySettlementLedger::new(base_book()).unwrap();
        let mut settlement = issue_settlement("issue-1", 100);
        settlement.authorization.authority_actor_id = actor("not-treasury");
        let before = ledger.clone();
        assert_eq!(
            ledger.settle(settlement),
            Err(SettlementError::AuthorizationActorMismatch)
        );
        assert_eq!(ledger, before);
    }

    #[test]
    fn settlement_transaction_cannot_hide_extra_value() {
        let mut ledger = MonetarySettlementLedger::new(base_book()).unwrap();
        let common_cause = cause("issue-extra-cause");
        let settlement = MonetarySettlement {
            settlement_id: settlement_id("issue-extra"),
            transaction: JournalTransaction::new(
                tx_id("issue-extra-tx"),
                common_cause.clone(),
                currency("USD-test"),
                vec![
                    Posting {
                        account_id: account_id("alice-cash"),
                        side: PostingSide::Debit,
                        amount: 100,
                    },
                    Posting {
                        account_id: account_id("treasury-equity"),
                        side: PostingSide::Credit,
                        amount: 100,
                    },
                ],
            )
            .unwrap(),
            monetary_event: MonetarySupplyEvent::Issue {
                currency_id: currency("USD-test"),
                authority_id: authority("usd-authority"),
                amount: 90,
                beneficiary_actor_id: actor("alice"),
                cause_id: common_cause,
            },
            settlement_account_id: account_id("alice-cash"),
            authorization: auth(),
        };
        assert!(matches!(
            ledger.settle(settlement),
            Err(SettlementError::SettlementPostingAmountMismatch { .. })
        ));
    }

    #[test]
    fn designated_settlement_account_must_belong_to_event_actor() {
        let mut ledger = MonetarySettlementLedger::new(base_book()).unwrap();
        let mut settlement = issue_settlement("issue-1", 100);
        if let MonetarySupplyEvent::Issue {
            beneficiary_actor_id,
            ..
        } = &mut settlement.monetary_event
        {
            *beneficiary_actor_id = actor("bob");
        }
        assert_eq!(
            ledger.settle(settlement),
            Err(SettlementError::SettlementAccountOwnerMismatch)
        );
    }

    #[test]
    fn duplicate_settlement_id_fails_without_mutation() {
        let mut ledger = MonetarySettlementLedger::new(base_book()).unwrap();
        ledger.settle(issue_settlement("issue-1", 100)).unwrap();
        let before = ledger.clone();
        let error = ledger
            .settle(issue_settlement("issue-1", 10))
            .unwrap_err();
        assert!(matches!(error, SettlementError::DuplicateSettlement { .. }));
        assert_eq!(ledger, before);
    }

    #[test]
    fn complete_history_replays_to_identical_state() {
        let mut ledger = MonetarySettlementLedger::new(base_book()).unwrap();
        ledger.settle(issue_settlement("issue-1", 100)).unwrap();
        ledger.settle(retire_settlement("retire-1", 25)).unwrap();

        let replayed = MonetarySettlementLedger::from_history(
            ledger.base_financial_book().clone(),
            ledger.entries().to_vec(),
        )
        .unwrap();
        assert_eq!(replayed, ledger);
    }

    #[test]
    fn invalid_sequence_is_rejected() {
        let entry = SettlementLedgerEntry {
            sequence: 2,
            settlement: issue_settlement("issue-1", 100),
        };
        assert_eq!(
            MonetarySettlementLedger::from_history(base_book(), vec![entry]),
            Err(SettlementError::InvalidSettlementSequence {
                expected: 1,
                actual: 2,
            })
        );
    }
}
